from __future__ import annotations

import asyncio
import logging
import math
import re
from collections.abc import Mapping
from datetime import UTC, datetime
from email.utils import parsedate_to_datetime
from importlib.metadata import version as _package_version
from typing import Any
from urllib.parse import quote
from uuid import UUID

import httpx
from pydantic import BaseModel, ConfigDict, ValidationError

from tana.config import Settings

logger = logging.getLogger(__name__)
PAGE_SIZE = 500

# The Jellyfin client "Version" advertises the installed package version so the
# header never drifts from pyproject.toml.
_TANA_VERSION = _package_version("tana")
MAX_ATTEMPTS = 3
MAX_POSTER_BYTES = 10 * 1024 * 1024
IMDB_ID_PATTERN = re.compile(r"tt[0-9]+\Z")


class JellyfinError(RuntimeError):
    """Sanitized error communicating with or decoding a Jellyfin response."""


class JellyfinTransportError(JellyfinError):
    """Jellyfin could not be reached or returned an unsuccessful status."""


class JellyfinResponseError(JellyfinError):
    """Jellyfin returned data that violates the documented response contract."""


class LibraryItemData(BaseModel):
    model_config = ConfigDict(frozen=True)

    jellyfin_id: str
    kind: str
    title: str
    series_title: str | None = None
    series_id: str | None = None
    season_number: int | None = None
    episode_number: int | None = None
    poster_tag: str | None = None
    series_poster_tag: str | None = None
    imdb_id: str | None = None


class _RetryableResponse(Exception):
    def __init__(self, response: httpx.Response) -> None:
        self.response = response


class JellyfinClient:
    def __init__(
        self,
        settings: Settings,
        *,
        client: httpx.AsyncClient | None = None,
        sleep=asyncio.sleep,
    ) -> None:
        self.settings = settings
        self._sleep = sleep
        self._owns_client = client is None
        self.client = client or httpx.AsyncClient(timeout=30.0)

    @property
    def headers(self) -> dict[str, str]:
        token = self.settings.JELLYFIN_API_KEY.get_secret_value()
        return {
            "Authorization": (
                'MediaBrowser Client="Tana", Device="Tana server", '
                f'DeviceId="tana", Version="{_TANA_VERSION}", Token="{token}"'
            ),
            "Accept": "application/json",
        }

    async def close(self) -> None:
        if self._owns_client:
            await self.client.aclose()

    async def fetch_library_items(self) -> tuple[list[LibraryItemData], int]:
        collected: list[LibraryItemData] = []
        seen = 0
        for item_type, kind in (("Movie", "movie"), ("Episode", "tv")):
            items, type_seen = await self._fetch_type(item_type, kind)
            collected.extend(items)
            seen += type_seen
        return collected, seen

    async def _fetch_type(
        self, item_type: str, kind: str
    ) -> tuple[list[LibraryItemData], int]:
        start_index = 0
        expected_total: int | None = None
        seen = 0
        accepted: dict[str, LibraryItemData] = {}

        while expected_total is None or start_index < expected_total:
            params = {
                "userId": self.settings.JELLYFIN_USER_ID,
                "includeItemTypes": item_type,
                "recursive": "true",
                "fields": "ProviderIds",
                "startIndex": start_index,
                "limit": PAGE_SIZE,
            }
            response_data = await self._get_json("/Items", params=params)
            page = self._get(response_data, "Items")
            total = self._get(response_data, "TotalRecordCount")
            if not isinstance(page, list):
                raise JellyfinResponseError("Jellyfin returned an invalid Items page")
            if isinstance(total, bool) or not isinstance(total, int) or total < 0:
                raise JellyfinResponseError(
                    "Jellyfin returned an invalid total record count"
                )
            if expected_total is None:
                expected_total = total
            elif total != expected_total:
                raise JellyfinResponseError(
                    "Jellyfin total record count changed during pagination"
                )
            if len(page) > PAGE_SIZE or start_index + len(page) > total:
                raise JellyfinResponseError(
                    "Jellyfin returned an inconsistent page size"
                )
            if not page:
                if start_index < expected_total:
                    raise JellyfinResponseError(
                        "Jellyfin pagination stopped before all items were read"
                    )
                break

            seen += len(page)
            for raw_item in page:
                parsed = self._parse_item(raw_item, item_type=item_type, kind=kind)
                if parsed is not None:
                    accepted[parsed.jellyfin_id] = parsed
            start_index += len(page)
            if start_index < expected_total and len(page) < 1:
                raise JellyfinResponseError("Jellyfin pagination made no progress")

        return list(accepted.values()), seen

    async def fetch_item_details(self, item_id: str) -> LibraryItemData | None:
        """Fetch a single item for webhook-time enrichment of a new row."""
        raw = await self._get_json(
            f"/Items/{quote(item_id, safe='')}",
            params={
                "userId": self.settings.JELLYFIN_USER_ID,
                "fields": "ProviderIds",
            },
        )
        item_type = self._get(raw, "Type")
        if not isinstance(item_type, str):
            return None
        kind = {"Movie": "movie", "Episode": "tv"}.get(item_type)
        if kind is None:
            return None
        return self._parse_item(raw, item_type=item_type, kind=kind)

    async def fetch_primary_poster(self, jellyfin_id: str, image_tag: str) -> bytes:
        url = (
            f"{self.settings.JELLYFIN_URL}/Items/{quote(jellyfin_id, safe='')}"
            "/Images/Primary"
        )
        params = {
            "maxWidth": 400,
            "maxHeight": 600,
            "quality": 85,
            "format": "Jpg",
            "tag": image_tag,
        }
        for attempt in range(MAX_ATTEMPTS):
            response: httpx.Response | None = None
            retry_after: float | None = None
            try:
                request = self.client.build_request(
                    "GET",
                    url,
                    params=params,
                    headers=self.headers | {"Accept": "image/*"},
                    timeout=30.0,
                )
                response = await self.client.send(request, stream=True)
                if response.status_code == 429 or response.status_code >= 500:
                    raise _RetryableResponse(response)
                response.raise_for_status()
                content_type = (
                    response.headers.get("content-type", "")
                    .split(";", 1)[0]
                    .strip()
                    .lower()
                )
                if content_type != "image/jpeg":
                    raise JellyfinResponseError(
                        "Jellyfin returned an invalid poster content type"
                    )
                chunks: list[bytes] = []
                size = 0
                async for chunk in response.aiter_bytes():
                    size += len(chunk)
                    if size > MAX_POSTER_BYTES:
                        raise JellyfinResponseError(
                            "Jellyfin poster exceeds size limit"
                        )
                    chunks.append(chunk)
                return b"".join(chunks)
            except _RetryableResponse as error:
                retry_after = self._retry_after(error.response)
            except (httpx.TimeoutException, httpx.NetworkError):
                pass
            except httpx.HTTPStatusError as error:
                raise JellyfinTransportError(
                    f"Jellyfin returned HTTP {error.response.status_code}"
                ) from None
            finally:
                if response is not None:
                    await response.aclose()

            if attempt + 1 >= MAX_ATTEMPTS:
                raise JellyfinTransportError(
                    "Jellyfin poster request failed after 3 attempts"
                ) from None
            await self._sleep(
                retry_after if retry_after is not None else float(attempt + 1)
            )

        raise JellyfinTransportError("Jellyfin poster request failed")

    async def _get_json(
        self, path: str, *, params: Mapping[str, Any]
    ) -> dict[str, Any]:
        url = f"{self.settings.JELLYFIN_URL}{path}"
        for attempt in range(MAX_ATTEMPTS):
            retry_after: float | None = None
            try:
                response = await self.client.get(
                    url, params=params, headers=self.headers, timeout=30.0
                )
                if response.status_code == 429 or response.status_code >= 500:
                    raise _RetryableResponse(response)
                response.raise_for_status()
                try:
                    data = response.json()
                except ValueError:
                    raise JellyfinResponseError(
                        "Jellyfin returned invalid JSON"
                    ) from None
                if not isinstance(data, dict):
                    raise JellyfinResponseError(
                        "Jellyfin returned an invalid JSON object"
                    )
                return data
            except _RetryableResponse as error:
                if attempt + 1 >= MAX_ATTEMPTS:
                    raise JellyfinTransportError(
                        f"Jellyfin returned HTTP {error.response.status_code} "
                        f"after {MAX_ATTEMPTS} attempts"
                    ) from None
                retry_after = self._retry_after(error.response)
            except (httpx.TimeoutException, httpx.NetworkError):
                if attempt + 1 >= MAX_ATTEMPTS:
                    raise JellyfinTransportError(
                        f"Jellyfin could not be reached after {MAX_ATTEMPTS} attempts"
                    ) from None
            except httpx.HTTPStatusError as error:
                raise JellyfinTransportError(
                    f"Jellyfin returned HTTP {error.response.status_code}"
                ) from None

            await self._sleep(
                retry_after if retry_after is not None else float(attempt + 1)
            )

        raise JellyfinTransportError("Jellyfin request failed")

    @staticmethod
    def _retry_after(response: httpx.Response) -> float | None:
        if response.status_code not in {429, 503}:
            return None
        value = response.headers.get("Retry-After")
        if value is None:
            return None
        try:
            delay = float(value)
        except ValueError:
            try:
                target = parsedate_to_datetime(value)
                if target.tzinfo is None:
                    target = target.replace(tzinfo=UTC)
                delay = (target - datetime.now(UTC)).total_seconds()
            except (TypeError, ValueError, OverflowError):
                return None
        if not math.isfinite(delay):
            return None
        return min(max(delay, 0.0), 30.0)

    @classmethod
    def _parse_item(
        cls, raw: Any, *, item_type: str, kind: str
    ) -> LibraryItemData | None:
        item_id_for_log: str | None = None
        try:
            if not isinstance(raw, dict):
                raise TypeError("DTO is not an object")
            item_id = cls._get(raw, "Id")
            if not isinstance(item_id, str) or not item_id.strip():
                raise ValueError("item ID is missing")
            item_id_for_log = str(UUID(item_id.strip()))
            name = cls._get(raw, "Name")
            actual_type = cls._get(raw, "Type")
            if not isinstance(name, str) or not name.strip():
                raise ValueError("item name is missing")
            if actual_type != item_type:
                raise ValueError("item type did not match query")
            if kind == "movie":
                image_tags = cls._get(raw, "ImageTags")
                provider_ids = cls._get(raw, "ProviderIds")
                poster_tag = cls._optional_nonempty_string(
                    cls._get(image_tags, "Primary")
                    if isinstance(image_tags, dict)
                    else None
                )
                imdb_value = cls._optional_nonempty_string(
                    cls._get(provider_ids, "Imdb")
                    if isinstance(provider_ids, dict)
                    else None
                )
                imdb_id = (
                    imdb_value
                    if imdb_value and IMDB_ID_PATTERN.fullmatch(imdb_value)
                    else None
                )
                return LibraryItemData(
                    jellyfin_id=item_id_for_log,
                    kind=kind,
                    title=name.strip(),
                    poster_tag=poster_tag,
                    imdb_id=imdb_id,
                )

            series_title = (
                cls._optional_nonempty_string(cls._get(raw, "SeriesName"))
                or name.strip()
            )
            series_id_value = cls._optional_nonempty_string(cls._get(raw, "SeriesId"))
            try:
                series_id = str(UUID(series_id_value)) if series_id_value else None
            except ValueError:
                series_id = None
            series_poster_tag = cls._optional_nonempty_string(
                cls._get(raw, "SeriesPrimaryImageTag")
            )
            season_number = cls._optional_integer(cls._get(raw, "ParentIndexNumber"))
            episode_number = cls._optional_integer(cls._get(raw, "IndexNumber"))
            return LibraryItemData(
                jellyfin_id=item_id_for_log,
                kind=kind,
                title=name.strip(),
                series_title=series_title,
                series_id=series_id,
                series_poster_tag=series_poster_tag,
                season_number=season_number,
                episode_number=episode_number,
            )
        except (ValueError, TypeError, ValidationError):
            logger.warning(
                "Ignoring invalid Jellyfin library item %s",
                item_id_for_log or "unknown",
            )
            return None

    @staticmethod
    def _get(value: Any, pascal_name: str) -> Any:
        if not isinstance(value, dict):
            return None
        if pascal_name in value:
            return value[pascal_name]
        camel_name = pascal_name[:1].lower() + pascal_name[1:]
        return value.get(camel_name)

    @staticmethod
    def _optional_nonempty_string(value: Any) -> str | None:
        if value is None:
            return None
        if not isinstance(value, str):
            raise TypeError("optional string field has an invalid type")
        return value.strip() or None

    @staticmethod
    def _optional_integer(value: Any) -> int | None:
        if value is None:
            return None
        if isinstance(value, bool) or not isinstance(value, int):
            raise TypeError("episode numbering field has an invalid type")
        return value
