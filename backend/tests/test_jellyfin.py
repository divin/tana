from __future__ import annotations

from collections.abc import Callable
from typing import Any

import httpx
import pytest
from pydantic import SecretStr

from tana.config import Settings
from tana.jellyfin import (
    MAX_POSTER_BYTES,
    JellyfinClient,
    JellyfinResponseError,
    JellyfinTransportError,
)

USER_ID = "00000000-0000-0000-0000-000000000111"
MOVIE_ID = "6f9619ff-8b86-d011-b42d-00cf4fc964ff"
EPISODE_ID = "6f9619ff-8b86-d011-b42d-00cf4fc96400"


def settings() -> Settings:
    return Settings(
        _env_file=None,
        JELLYFIN_URL="http://jellyfin.example:8096/base/",
        JELLYFIN_API_KEY=SecretStr("unit-test-key"),
        JELLYFIN_USER_ID=USER_ID,
        TZ="Europe/Berlin",
    )


def movie_dto(
    *, item_id: str = MOVIE_ID, name: str = "Example movie"
) -> dict[str, Any]:
    return {
        "Id": item_id,
        "Name": name,
        "Type": "Movie",
        "ImageTags": {"Primary": "primary-tag"},
        "ProviderIds": {"Imdb": "tt1234567"},
    }


def episode_dto() -> dict[str, Any]:
    return {
        "Id": EPISODE_ID,
        "Name": "Episode title",
        "Type": "Episode",
        "SeriesName": "Series name",
        "SeriesId": "00000000000000000000000000000222",
        "SeriesPrimaryImageTag": "series-primary-tag",
        "ParentIndexNumber": 2,
        "IndexNumber": 5,
    }


def page(items: list[dict[str, Any]], total: int) -> dict[str, Any]:
    return {"Items": items, "TotalRecordCount": total}


def client_for(handler: Callable[[httpx.Request], httpx.Response], *, sleep=None):
    transport = httpx.MockTransport(handler)
    kwargs: dict[str, Any] = {"transport": transport}
    if sleep is None:
        client = JellyfinClient(settings(), client=httpx.AsyncClient(**kwargs))
    else:
        client = JellyfinClient(
            settings(), client=httpx.AsyncClient(**kwargs), sleep=sleep
        )
    client._owns_client = True
    return client


@pytest.mark.asyncio
async def test_client_requests_both_item_types_with_verified_auth_and_fields() -> None:
    calls: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        calls.append(request)
        item_type = request.url.params["includeItemTypes"]
        items = [movie_dto()] if item_type == "Movie" else [episode_dto()]
        return httpx.Response(200, json=page(items, 1))

    client = client_for(handler)
    try:
        items, seen = await client.fetch_library_items()
    finally:
        await client.close()

    assert seen == 2
    assert [item.kind for item in items] == ["movie", "tv"]
    assert calls[0].url.path == "/base/Items"
    assert calls[0].url.params["userId"] == USER_ID
    assert calls[0].url.params["includeItemTypes"] == "Movie"
    assert calls[0].url.params["recursive"] == "true"
    assert calls[0].url.params["fields"] == "ProviderIds"
    assert calls[0].url.params["startIndex"] == "0"
    assert calls[0].url.params["limit"] == "500"
    assert (
        calls[0]
        .headers["Authorization"]
        .startswith(
            'MediaBrowser Client="Tana", Device="Tana server", DeviceId="tana", Version="'
        )
    )
    assert '", Token="unit-test-key"' in calls[0].headers["Authorization"]
    assert "X-Emby-Token" not in calls[0].headers
    assert items[0].title == "Example movie"
    assert items[0].poster_tag == "primary-tag"
    assert items[0].imdb_id == "tt1234567"
    assert items[1].series_title == "Series name"
    assert items[1].series_id == "00000000-0000-0000-0000-000000000222"
    assert items[1].series_poster_tag == "series-primary-tag"
    assert items[1].season_number == 2
    assert items[1].episode_number == 5


@pytest.mark.asyncio
async def test_client_paginates_and_deduplicates_last_valid_record() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        offset = int(request.url.params["startIndex"])
        if request.url.params["includeItemTypes"] == "Movie":
            if offset == 0:
                return httpx.Response(
                    200,
                    json=page(
                        [movie_dto(name="Old title"), movie_dto(item_id=EPISODE_ID)], 3
                    ),
                )
            return httpx.Response(200, json=page([movie_dto(name="New title")], 3))
        return httpx.Response(200, json=page([], 0))

    client = client_for(handler)
    try:
        items, seen = await client.fetch_library_items()
    finally:
        await client.close()

    assert seen == 3
    assert len(items) == 2
    assert (
        next(item for item in items if item.jellyfin_id == MOVIE_ID).title
        == "New title"
    )
    movie_calls = [r for r in requests if r.url.params["includeItemTypes"] == "Movie"]
    assert [r.url.params["startIndex"] for r in movie_calls] == ["0", "2"]


@pytest.mark.asyncio
async def test_invalid_items_are_skipped_and_camel_case_is_accepted() -> None:
    camel = {
        "id": MOVIE_ID,
        "name": "Camel Case",
        "type": "Movie",
        "imageTags": {"primary": "tag"},
        "providerIds": {"imdb": "invalid"},
    }
    invalid = movie_dto()
    del invalid["Name"]

    def handler(request: httpx.Request) -> httpx.Response:
        if request.url.params["includeItemTypes"] == "Movie":
            return httpx.Response(200, json=page([invalid, camel], 2))
        return httpx.Response(200, json=page([], 0))

    client = client_for(handler)
    try:
        items, seen = await client.fetch_library_items()
    finally:
        await client.close()

    assert seen == 2
    assert len(items) == 1
    assert items[0].title == "Camel Case"
    assert items[0].imdb_id is None


@pytest.mark.asyncio
async def test_client_retries_transient_response_and_caps_retry_after() -> None:
    calls = 0
    delays: list[float] = []

    async def sleep(seconds: float) -> None:
        delays.append(seconds)

    def handler(request: httpx.Request) -> httpx.Response:
        nonlocal calls
        calls += 1
        if calls == 1:
            return httpx.Response(503, headers={"Retry-After": "100"})
        return httpx.Response(200, json=page([], 0))

    client = client_for(handler, sleep=sleep)
    try:
        data = await client._get_json("/Items", params={})
    finally:
        await client.close()

    assert data["TotalRecordCount"] == 0
    assert calls == 2
    assert delays == [30.0]


@pytest.mark.asyncio
async def test_client_retries_network_failures_then_stops_after_three_attempts() -> (
    None
):
    calls = 0
    delays: list[float] = []

    async def sleep(seconds: float) -> None:
        delays.append(seconds)

    def handler(request: httpx.Request) -> httpx.Response:
        nonlocal calls
        calls += 1
        raise httpx.ConnectError("private endpoint details", request=request)

    client = client_for(handler, sleep=sleep)
    try:
        with pytest.raises(JellyfinTransportError, match="after 3 attempts") as error:
            await client._get_json("/Items", params={})
    finally:
        await client.close()

    assert calls == 3
    assert delays == [1.0, 2.0]
    assert "private endpoint details" not in str(error.value)


@pytest.mark.asyncio
async def test_client_reports_sanitized_status_after_exhausting_server_retries() -> (
    None
):
    calls = 0

    async def no_wait(_seconds: float) -> None:
        return

    def unavailable(request: httpx.Request) -> httpx.Response:
        nonlocal calls
        calls += 1
        return httpx.Response(503, text="private server response")

    client = client_for(unavailable, sleep=no_wait)
    try:
        with pytest.raises(JellyfinTransportError) as error:
            await client._get_json("/Items", params={})
    finally:
        await client.close()

    assert str(error.value) == "Jellyfin returned HTTP 503 after 3 attempts"
    assert "private server response" not in str(error.value)
    assert calls == 3


@pytest.mark.asyncio
async def test_poster_request_uses_verified_auth_and_image_parameters() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(
            200, headers={"Content-Type": "image/jpeg"}, content=b"jpeg-data"
        )

    client = client_for(handler)
    try:
        image = await client.fetch_primary_poster(MOVIE_ID, "primary tag")
    finally:
        await client.close()

    assert image == b"jpeg-data"
    request = requests[0]
    assert request.url.path == f"/base/Items/{MOVIE_ID}/Images/Primary"
    assert request.url.params["maxWidth"] == "400"
    assert request.url.params["maxHeight"] == "600"
    assert request.url.params["quality"] == "85"
    assert request.url.params["format"] == "Jpg"
    assert request.url.params["tag"] == "primary tag"
    assert request.headers["Accept"] == "image/*"
    assert 'Token="unit-test-key"' in request.headers["Authorization"]


@pytest.mark.asyncio
async def test_poster_client_rejects_non_jpeg_and_oversized_responses() -> None:
    wrong_type = client_for(
        lambda request: httpx.Response(
            200, headers={"Content-Type": "application/json"}
        )
    )
    try:
        with pytest.raises(JellyfinResponseError, match="content type"):
            await wrong_type.fetch_primary_poster(MOVIE_ID, "tag")
    finally:
        await wrong_type.close()

    too_large = client_for(
        lambda request: httpx.Response(
            200,
            headers={"Content-Type": "image/jpeg"},
            content=b"x" * (MAX_POSTER_BYTES + 1),
        )
    )
    try:
        with pytest.raises(JellyfinResponseError, match="size limit"):
            await too_large.fetch_primary_poster(MOVIE_ID, "tag")
    finally:
        await too_large.close()


@pytest.mark.asyncio
async def test_client_does_not_retry_auth_errors_and_rejects_bad_response() -> None:
    calls = 0

    def unauthorized(request: httpx.Request) -> httpx.Response:
        nonlocal calls
        calls += 1
        return httpx.Response(401)

    client = client_for(unauthorized)
    try:
        with pytest.raises(JellyfinTransportError, match="HTTP 401"):
            await client._get_json("/Items", params={})
    finally:
        await client.close()
    assert calls == 1

    bad_client = client_for(lambda request: httpx.Response(200, content=b"not json"))
    try:
        with pytest.raises(JellyfinResponseError, match="invalid JSON"):
            await bad_client._get_json("/Items", params={})
    finally:
        await bad_client.close()


@pytest.mark.asyncio
async def test_fetch_item_details_returns_single_item_metadata() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        if request.url.path == f"/base/Items/{EPISODE_ID}":
            return httpx.Response(
                200,
                json={
                    "Id": EPISODE_ID,
                    "Name": "Pilot",
                    "Type": "Episode",
                    "SeriesName": "Series name",
                    "SeriesId": "00000000000000000000000000000222",
                    "SeriesPrimaryImageTag": "series-tag",
                    "ParentIndexNumber": 1,
                    "IndexNumber": 1,
                },
            )
        return httpx.Response(
            200,
            json={
                "Id": MOVIE_ID,
                "Name": "Single movie",
                "Type": "Movie",
                "ImageTags": {"Primary": "single-tag"},
                "ProviderIds": {"Imdb": "tt1234567"},
            },
        )

    client = client_for(handler)
    try:
        movie = await client.fetch_item_details(MOVIE_ID)
        episode = await client.fetch_item_details(EPISODE_ID)
    finally:
        await client.close()

    assert movie is not None
    assert movie.kind == "movie"
    assert movie.poster_tag == "single-tag"
    assert movie.imdb_id == "tt1234567"
    assert [r.url.path for r in requests] == [
        f"/base/Items/{MOVIE_ID}",
        f"/base/Items/{EPISODE_ID}",
    ]
    assert all(r.url.params["userId"] == USER_ID for r in requests)
    assert all(r.url.params["fields"] == "ProviderIds" for r in requests)
    assert episode is not None
    assert episode.kind == "tv"
    assert episode.series_poster_tag == "series-tag"
    assert episode.title == "Pilot"


@pytest.mark.asyncio
async def test_fetch_item_details_ignores_unsupported_item_types() -> None:
    client = client_for(
        lambda request: httpx.Response(
            200,
            json={"Id": MOVIE_ID, "Name": "Folder", "Type": "Season"},
        )
    )
    try:
        result = await client.fetch_item_details("00000000000000000000000000000222")
    finally:
        await client.close()

    assert result is None
