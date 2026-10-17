from __future__ import annotations

import asyncio
from hashlib import sha256
from io import BytesIO
from pathlib import Path

import pytest
from httpx import ASGITransport, AsyncClient
from PIL import Image
from pydantic import SecretStr

from tana import app as app_module
from tana.app import create_app
from tana.config import Settings
from tana.jellyfin import JellyfinError
from tana.models import PosterCache, WatchedItem
from tana.posters import normalize_jpeg, poster_filename

MOVIE_ID = "6f9619ff-8b86-d011-b42d-00cf4fc964ff"
SERIES_ID = "6f9619ff-8b86-d011-b42d-00cf4fc96400"


def settings() -> Settings:
    return Settings(
        _env_file=None,
        JELLYFIN_URL="http://jellyfin.example:8096",
        JELLYFIN_API_KEY=SecretStr("poster-test-key"),
        JELLYFIN_USER_ID="test-user",
        TZ="UTC",
    )


def jpeg_bytes(color: tuple[int, int, int]) -> bytes:
    image = Image.new("RGB", (800, 1200), color)
    output = BytesIO()
    image.save(output, format="JPEG")
    return output.getvalue()


class FakePosterClient:
    def __init__(self, settings, responses: list[bytes | Exception]) -> None:
        self.settings = settings
        self.responses = responses
        self.calls: list[tuple[str, str]] = []

    async def fetch_primary_poster(self, jellyfin_id: str, image_tag: str) -> bytes:
        self.calls.append((jellyfin_id, image_tag))
        response = self.responses.pop(0)
        if isinstance(response, Exception):
            raise response
        return response

    async def close(self) -> None:
        return


async def seed_movie(app, tag: str = "primary-v1") -> None:
    async with app.state.database.session() as session:
        session.add(
            WatchedItem(
                jellyfin_id=MOVIE_ID,
                kind="movie",
                title="Poster film",
                watch_status="watched",
                watched_at="2025-01-02T03:04:05.000000Z",
                play_count=2,
                poster_tag=tag,
                imdb_id="tt1234567",
                updated_at="2025-01-02T03:04:05.000000Z",
            )
        )
        await session.commit()


async def seed_series(app, series_tag: str = "series-primary-v1") -> None:
    async with app.state.database.session() as session:
        session.add_all(
            [
                WatchedItem(
                    jellyfin_id="00000000-0000-0000-0000-000000000031",
                    kind="tv",
                    title="Older watched episode",
                    watch_status="watched",
                    series_title="Example series",
                    series_id=SERIES_ID,
                    watched_at="2025-01-01T00:00:00.000000Z",
                    play_count=1,
                    series_poster_tag="old-tag",
                    updated_at="2025-01-02T00:00:00.000000Z",
                ),
                WatchedItem(
                    jellyfin_id="00000000-0000-0000-0000-000000000032",
                    kind="tv",
                    title="Recently synced episode",
                    watch_status="watched",
                    series_title="Example series",
                    series_id=SERIES_ID,
                    watched_at="2025-01-02T00:00:00.000000Z",
                    play_count=2,
                    series_poster_tag=series_tag,
                    updated_at="2025-01-03T00:00:00.000000Z",
                ),
            ]
        )
        await session.commit()


def make_app(tmp_path: Path, monkeypatch, fake_client: FakePosterClient):
    monkeypatch.setattr(app_module, "JellyfinClient", lambda _settings: fake_client)
    return create_app(
        settings=settings(),
        data_dir=tmp_path / "data",
        static_dir=Path(__file__).resolve().parents[2] / "frontend" / "public",
        start_background_sync=False,
    )


@pytest.mark.asyncio
async def test_poster_is_downloaded_resized_cached_and_supports_304(
    tmp_path: Path, monkeypatch
) -> None:
    fake = FakePosterClient(settings(), [jpeg_bytes((200, 10, 20))])
    app = make_app(tmp_path, monkeypatch, fake)

    async with app.router.lifespan_context(app):
        await seed_movie(app)
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            first = await client.get(f"/api/posters/{MOVIE_ID}.jpg")
            etag = first.headers["etag"]
            cached = await client.get(
                f"/api/posters/{MOVIE_ID}.jpg", headers={"If-None-Match": etag}
            )

        poster_path = poster_filename(MOVIE_ID, app.state.database.poster_path)
        async with app.state.database.session() as session:
            metadata = await session.get(PosterCache, ("movie", MOVIE_ID))

    assert first.status_code == 200
    assert first.headers["content-type"] == "image/jpeg"
    assert first.headers["x-content-type-options"] == "nosniff"
    assert first.headers["cache-control"] == "no-cache"
    assert first.content.startswith(b"\xff\xd8")
    assert cached.status_code == 304
    assert cached.content == b""
    assert len(fake.calls) == 1
    assert poster_path.is_file()
    assert metadata is not None
    assert metadata.source_type == "movie"
    assert metadata.image_tag == "primary-v1"
    assert metadata.size_bytes == len(first.content)
    with Image.open(BytesIO(first.content)) as result:
        assert result.format == "JPEG"
        assert result.size == (400, 600)


@pytest.mark.asyncio
async def test_poster_cache_refreshes_when_jellyfin_tag_changes(
    tmp_path: Path, monkeypatch
) -> None:
    fake = FakePosterClient(settings(), [jpeg_bytes((1, 2, 3)), jpeg_bytes((4, 5, 6))])
    app = make_app(tmp_path, monkeypatch, fake)

    async with app.router.lifespan_context(app):
        await seed_movie(app)
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            first = await client.get(f"/api/posters/{MOVIE_ID}.jpg")
            async with app.state.database.session() as session:
                movie = await session.get(WatchedItem, MOVIE_ID)
                assert movie is not None
                movie.poster_tag = "primary-v2"
                await session.commit()
            second = await client.get(f"/api/posters/{MOVIE_ID}.jpg")

        async with app.state.database.session() as session:
            metadata = await session.get(PosterCache, ("movie", MOVIE_ID))

    assert first.status_code == 200
    assert second.status_code == 200
    assert second.content != first.content
    assert second.headers["etag"] == f'"{sha256(b"primary-v2").hexdigest()}"'
    assert fake.calls == [(MOVIE_ID, "primary-v1"), (MOVIE_ID, "primary-v2")]
    assert metadata is not None
    assert metadata.source_type == "movie"
    assert metadata.image_tag == "primary-v2"


@pytest.mark.asyncio
async def test_tv_episodes_share_series_poster_and_refresh_latest_series_tag(
    tmp_path: Path, monkeypatch
) -> None:
    fake = FakePosterClient(
        settings(), [jpeg_bytes((30, 40, 50)), jpeg_bytes((60, 70, 80))]
    )
    app = make_app(tmp_path, monkeypatch, fake)

    async with app.router.lifespan_context(app):
        await seed_series(app)
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            episodes = await client.get("/api/media", params={"kind": "tv"})
            first = await client.get(f"/api/posters/{SERIES_ID}.jpg")
            async with app.state.database.session() as session:
                latest = await session.get(
                    WatchedItem, "00000000-0000-0000-0000-000000000032"
                )
                assert latest is not None
                latest.series_poster_tag = "series-primary-v2"
                await session.commit()
            second = await client.get(f"/api/posters/{SERIES_ID}.jpg")

        async with app.state.database.session() as session:
            metadata = await session.get(PosterCache, ("series", SERIES_ID))

    assert episodes.status_code == 200
    assert {episode["poster_url"] for episode in episodes.json()["items"]} == {
        f"/api/posters/{SERIES_ID}.jpg"
    }
    assert first.status_code == second.status_code == 200
    assert first.content != second.content
    assert fake.calls == [
        (SERIES_ID, "series-primary-v1"),
        (SERIES_ID, "series-primary-v2"),
    ]
    assert poster_filename(
        SERIES_ID, app.state.database.poster_path, "series"
    ).is_file()
    assert metadata is not None
    assert metadata.source_type == "series"
    assert metadata.image_tag == "series-primary-v2"


@pytest.mark.asyncio
async def test_missing_poster_and_invalid_id_return_404(
    tmp_path: Path, monkeypatch
) -> None:
    fake = FakePosterClient(settings(), [])
    app = make_app(tmp_path, monkeypatch, fake)

    async with app.router.lifespan_context(app):
        await seed_movie(app, tag="")
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            missing = await client.get(f"/api/posters/{MOVIE_ID}.jpg")
            invalid = await client.get("/api/posters/not-a-uuid.jpg")

    assert missing.status_code == 404
    assert invalid.status_code == 404
    assert fake.calls == []


@pytest.mark.asyncio
async def test_changed_tag_download_failure_returns_502_and_preserves_old_cache(
    tmp_path: Path, monkeypatch
) -> None:
    fake = FakePosterClient(
        settings(), [jpeg_bytes((20, 30, 40)), JellyfinError("secret upstream details")]
    )
    app = make_app(tmp_path, monkeypatch, fake)

    async with app.router.lifespan_context(app):
        await seed_movie(app)
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            first = await client.get(f"/api/posters/{MOVIE_ID}.jpg")
            path = poster_filename(MOVIE_ID, app.state.database.poster_path)
            prior_file = path.read_bytes()
            async with app.state.database.session() as session:
                movie = await session.get(WatchedItem, MOVIE_ID)
                assert movie is not None
                movie.poster_tag = "primary-v2"
                await session.commit()
            failed = await client.get(f"/api/posters/{MOVIE_ID}.jpg")

        async with app.state.database.session() as session:
            metadata = await session.get(PosterCache, ("movie", MOVIE_ID))

    assert first.status_code == 200
    assert failed.status_code == 502
    assert "secret upstream details" not in failed.text
    assert path.read_bytes() == prior_file
    assert metadata is not None
    assert metadata.source_type == "movie"
    assert metadata.image_tag == "primary-v1"


@pytest.mark.asyncio
async def test_invalid_image_response_returns_502_without_persisting_cache(
    tmp_path: Path, monkeypatch
) -> None:
    fake = FakePosterClient(settings(), [b"not an image"])
    app = make_app(tmp_path, monkeypatch, fake)

    async with app.router.lifespan_context(app):
        await seed_movie(app)
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            response = await client.get(f"/api/posters/{MOVIE_ID}.jpg")
        async with app.state.database.session() as session:
            metadata = await session.get(PosterCache, ("movie", MOVIE_ID))

    assert response.status_code == 502
    assert metadata is None
    assert not poster_filename(MOVIE_ID, app.state.database.poster_path).exists()


@pytest.mark.asyncio
async def test_concurrent_poster_requests_share_one_refresh(
    tmp_path: Path, monkeypatch
) -> None:
    fake = FakePosterClient(settings(), [jpeg_bytes((50, 60, 70))])
    entered = asyncio.Event()
    release = asyncio.Event()

    async def delayed_fetch(jellyfin_id: str, image_tag: str) -> bytes:
        fake.calls.append((jellyfin_id, image_tag))
        entered.set()
        await release.wait()
        return jpeg_bytes((50, 60, 70))

    fake.fetch_primary_poster = delayed_fetch
    app = make_app(tmp_path, monkeypatch, fake)

    async with app.router.lifespan_context(app):
        await seed_movie(app)
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            first_request = asyncio.create_task(
                client.get(f"/api/posters/{MOVIE_ID}.jpg")
            )
            await asyncio.wait_for(entered.wait(), timeout=1)
            second_request = asyncio.create_task(
                client.get(f"/api/posters/{MOVIE_ID}.jpg")
            )
            await asyncio.sleep(0)
            release.set()
            first, second = await asyncio.gather(first_request, second_request)

    assert first.status_code == second.status_code == 200
    assert first.content == second.content
    assert fake.calls == [(MOVIE_ID, "primary-v1")]


def test_normalize_jpeg_composites_transparency_and_rejects_invalid_dimensions() -> (
    None
):
    rgba = Image.new("RGBA", (20, 30), (255, 0, 0, 0))
    output = BytesIO()
    rgba.save(output, format="PNG")

    normalized = normalize_jpeg(output.getvalue())
    with Image.open(BytesIO(normalized)) as image:
        assert image.format == "JPEG"
        assert image.getpixel((0, 0)) == (255, 255, 255)

    huge_png = BytesIO()
    Image.new("RGB", (4500, 4500)).save(huge_png, format="PNG")
    with pytest.raises(Exception, match="Poster image is invalid"):
        normalize_jpeg(huge_png.getvalue())
