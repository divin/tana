from __future__ import annotations

import asyncio
import os
import tempfile
from datetime import UTC, datetime
from hashlib import sha256
from io import BytesIO
from pathlib import Path
from uuid import UUID

from PIL import Image, UnidentifiedImageError
from sqlmodel import select
from sqlmodel.ext.asyncio.session import AsyncSession

from tana.jellyfin import JellyfinClient, JellyfinError
from tana.models import PosterCache, WatchedItem

MAX_IMAGE_PIXELS = 20_000_000
MAX_POSTER_DIMENSIONS = (400, 600)


class PosterError(RuntimeError):
    def __init__(self, status_code: int, public_message: str) -> None:
        self.status_code = status_code
        self.public_message = public_message
        super().__init__(public_message)


def poster_etag(image_tag: str) -> str:
    digest = sha256(image_tag.encode("utf-8")).hexdigest()
    return f'"{digest}"'


def poster_filename(
    source_id: str, directory: Path, source_type: str = "movie"
) -> Path:
    cache_key = f"{source_type}\0{source_id}".encode()
    digest = sha256(cache_key).hexdigest()
    return directory / f"{digest}.jpg"


def normalize_jpeg(data: bytes) -> bytes:
    try:
        with Image.open(BytesIO(data)) as source:
            width, height = source.size
            if width <= 0 or height <= 0 or width * height > MAX_IMAGE_PIXELS:
                raise PosterError(502, "Poster image is invalid")
            source.seek(0)
            source.load()
            if "A" in source.getbands() or "transparency" in source.info:
                rgba = source.convert("RGBA")
                background = Image.new("RGBA", rgba.size, (255, 255, 255, 255))
                background.alpha_composite(rgba)
                image = background.convert("RGB")
            else:
                image = source.convert("RGB")
            image.thumbnail(MAX_POSTER_DIMENSIONS, Image.Resampling.LANCZOS)
            output = BytesIO()
            image.save(output, format="JPEG", quality=85)
            return output.getvalue()
    except PosterError:
        raise
    except (OSError, ValueError, UnidentifiedImageError, Image.DecompressionBombError):
        raise PosterError(502, "Poster image is invalid") from None


def _valid_cached_jpeg(path: Path) -> bool:
    try:
        with Image.open(path) as image:
            width, height = image.size
            if (
                image.format != "JPEG"
                or width <= 0
                or height <= 0
                or width * height > MAX_IMAGE_PIXELS
            ):
                return False
            image.verify()
        return True
    except (OSError, ValueError, Image.DecompressionBombError):
        return False


def _cache_entry_is_usable(
    metadata: PosterCache | None,
    source_type: str,
    image_tag: str,
    path: Path,
) -> bool:
    if (
        metadata is None
        or metadata.source_type != source_type
        or metadata.image_tag != image_tag
    ):
        return False
    try:
        return path.is_file() and path.stat().st_size > 0 and _valid_cached_jpeg(path)
    except OSError:
        return False


def _atomic_write(path: Path, data: bytes) -> None:
    temporary_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb", dir=path.parent, prefix=".poster-", suffix=".tmp", delete=False
        ) as temporary_file:
            temporary_path = Path(temporary_file.name)
            temporary_file.write(data)
            temporary_file.flush()
            os.fsync(temporary_file.fileno())
        os.replace(temporary_path, path)
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)


class PosterService:
    def __init__(self, jellyfin: JellyfinClient, poster_directory: Path) -> None:
        self.jellyfin = jellyfin
        self.poster_directory = poster_directory
        self._locks: dict[str, asyncio.Lock] = {}

    async def _resolve_source(
        self, source_id: str, session: AsyncSession
    ) -> tuple[str, str] | None:
        movie = await session.get(WatchedItem, source_id)
        if movie is not None and movie.kind == "movie" and movie.poster_tag:
            return "movie", movie.poster_tag

        series_episode = (
            await session.exec(
                select(WatchedItem)
                .where(
                    WatchedItem.kind == "tv",
                    WatchedItem.series_id == source_id,
                    WatchedItem.series_poster_tag.is_not(None),
                )
                .order_by(WatchedItem.updated_at.desc(), WatchedItem.jellyfin_id.asc())
                .limit(1)
            )
        ).first()
        if series_episode is not None and series_episode.series_poster_tag:
            return "series", series_episode.series_poster_tag
        return None

    async def get_poster(
        self, raw_source_id: str, session: AsyncSession
    ) -> tuple[bytes, str]:
        try:
            source_id = str(UUID(raw_source_id))
        except (ValueError, TypeError, AttributeError):
            raise PosterError(404, "Poster not found") from None

        source = await self._resolve_source(source_id, session)
        if source is None:
            raise PosterError(404, "Poster not found")
        source_type, image_tag = source
        path = poster_filename(source_id, self.poster_directory, source_type)
        etag = poster_etag(image_tag)

        cached = await session.get(PosterCache, (source_type, source_id))
        if _cache_entry_is_usable(cached, source_type, image_tag, path):
            try:
                return await asyncio.to_thread(path.read_bytes), etag
            except OSError:
                pass

        lock_key = f"{source_type}:{source_id}"
        lock = self._locks.setdefault(lock_key, asyncio.Lock())
        async with lock:
            # Re-resolve after taking the lock in case a concurrent sync changed the image tag.
            source = await self._resolve_source(source_id, session)
            if source is None:
                raise PosterError(404, "Poster not found")
            source_type, image_tag = source
            path = poster_filename(source_id, self.poster_directory, source_type)
            etag = poster_etag(image_tag)
            cached = await session.get(PosterCache, (source_type, source_id))
            if _cache_entry_is_usable(cached, source_type, image_tag, path):
                try:
                    return await asyncio.to_thread(path.read_bytes), etag
                except OSError:
                    pass

            try:
                upstream = await self.jellyfin.fetch_primary_poster(
                    source_id, image_tag
                )
            except JellyfinError:
                raise PosterError(502, "Poster could not be retrieved") from None
            normalized = await asyncio.to_thread(normalize_jpeg, upstream)
            await asyncio.to_thread(_atomic_write, path, normalized)

            cached_at = (
                datetime.now(UTC)
                .isoformat(timespec="microseconds")
                .replace("+00:00", "Z")
            )
            if cached is None:
                cached = PosterCache(
                    source_id=source_id,
                    source_type=source_type,
                    image_tag=image_tag,
                    cached_at=cached_at,
                    size_bytes=len(normalized),
                )
                session.add(cached)
            else:
                cached.source_type = source_type
                cached.image_tag = image_tag
                cached.cached_at = cached_at
                cached.size_bytes = len(normalized)
            await session.commit()
            return normalized, etag
