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


def user_data(**overrides: Any) -> dict[str, Any]:
    return {
        "Played": True,
        "PlayCount": 1,
        "LastPlayedDate": "2026-10-01T10:00:00.000000Z",
        "PlaybackPositionTicks": 0,
        **overrides,
    }


def movie_dto(
    *, item_id: str = MOVIE_ID, name: str = "Example movie"
) -> dict[str, Any]:
    return {
        "Id": item_id,
        "Name": name,
        "Type": "Movie",
        "ImageTags": {"Primary": "primary-tag"},
        "ProviderIds": {"Imdb": "tt1234567"},
        "UserData": user_data(),
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
        "UserData": user_data(),
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
        items, seen = await client.fetch_played_items()
    finally:
        await client.close()

    assert seen == 2
    assert [item.kind for item in items] == ["movie", "tv"]
    assert calls[0].url.path == f"/base/Users/{USER_ID}/Items"
    assert calls[0].url.params["userId"] == USER_ID
    assert calls[0].url.params["includeItemTypes"] == "Movie"
    assert calls[0].url.params["recursive"] == "true"
    assert calls[0].url.params["filters"] == "IsPlayed"
    assert calls[0].url.params["fields"] == "UserData,ProviderIds,DateCreated"
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
    assert items[0].play_count == 1
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
        items, seen = await client.fetch_played_items()
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
        "userData": user_data(),
    }
    invalid = movie_dto()
    del invalid["Name"]

    def handler(request: httpx.Request) -> httpx.Response:
        if request.url.params["includeItemTypes"] == "Movie":
            return httpx.Response(200, json=page([invalid, camel], 2))
        return httpx.Response(200, json=page([], 0))

    client = client_for(handler)
    try:
        items, seen = await client.fetch_played_items()
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
        user_data = {
            "Played": True,
            "PlayCount": 1,
            "LastPlayedDate": "2026-10-01T10:00:00.000000Z",
            "PlaybackPositionTicks": 0,
        }
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
                    "UserData": user_data,
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
                "UserData": user_data,
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
    assert all(r.url.params["fields"] == "UserData,ProviderIds" for r in requests)
    assert movie.play_count == 1
    assert movie.last_played_date == "2026-10-01T10:00:00.000000Z"
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


def load_fixture(name: str) -> Any:
    import json
    from pathlib import Path

    return json.loads((Path(__file__).parent / "fixtures" / name).read_text())


@pytest.mark.asyncio
async def test_fetch_played_items_parses_live_shaped_responses() -> None:
    movie_dto = load_fixture("items_played_movie.json")
    episode_dto = load_fixture("items_played_episode.json")

    def handler(request: httpx.Request) -> httpx.Response:
        item_type = request.url.params["includeItemTypes"]
        body = page([movie_dto] if item_type == "Movie" else [episode_dto], 1)
        return httpx.Response(200, json=body)

    client = client_for(handler)
    try:
        items, seen = await client.fetch_played_items()
    finally:
        await client.close()

    assert seen == 2
    assert len(items) == 2
    movie = next(item for item in items if item.kind == "movie")
    episode = next(item for item in items if item.kind == "tv")
    assert movie.played is True
    assert movie.play_count == 1
    # 3-fraction-digit server timestamps are normalized to six digits + Z.
    assert movie.last_played_date == "2026-08-28T18:31:36.438000Z"
    assert episode.played is True
    assert episode.series_title == "Series 0"  # sanitized fixture name
    assert episode.season_number == 1
    assert episode.episode_number == 1


@pytest.mark.asyncio
async def test_fetch_played_items_skips_items_without_user_data() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        item_type = request.url.params["includeItemTypes"]
        return httpx.Response(
            200, json=page([{"Id": MOVIE_ID, "Name": "No", "Type": item_type}], 1)
        )

    client = client_for(handler)
    try:
        items, _seen = await client.fetch_played_items()
    finally:
        await client.close()
    assert items == []


@pytest.mark.asyncio
async def test_fetch_resume_items_parses_live_shaped_response() -> None:
    resume_dto = load_fixture("items_resume.json")

    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, json={"Items": resume_dto})

    client = client_for(handler)
    try:
        items = await client.fetch_resume_items()
    finally:
        await client.close()

    assert len(items) == 1
    episode = items[0]
    assert episode.kind == "tv"
    assert episode.played is False
    assert episode.play_count == 3
    # Jellyfin's 0-100 percentage is normalized to a 0-1 fraction.
    assert episode.played_percentage == pytest.approx(0.3846)
    assert episode.position_ticks == 4602687810
    # Seven fractional digits are normalized to six.
    assert episode.last_played_date == "2026-10-01T16:01:35.020818Z"


@pytest.mark.asyncio
async def test_fetch_activity_entries_parses_and_reports_max_raw_id() -> None:
    [start_dto, stop_dto] = load_fixture("activity_log_entries.json")
    noise = {"Id": 2571, "Type": "SessionStarted", "Name": "online"}
    page_body = {"Items": [start_dto, stop_dto, noise], "TotalRecordCount": 3}

    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, json=page_body)

    client = client_for(handler)
    try:
        entries, total, max_raw_id, raw_count = await client.fetch_activity_entries()
    finally:
        await client.close()

    assert raw_count == 3

    assert total == 3
    # The session-noise entry is dropped; its raw id still advances the cursor.
    assert max_raw_id == 2571
    assert len(entries) == 2
    start, stop = entries
    assert start.kind == "start"
    assert stop.kind == "stop"
    # Compact log UUIDs are canonicalized to hyphenated form.
    assert start.item_id == stop.item_id
    assert "-" in start.item_id
    assert start.user_id == "11111111-1111-4111-8111-111111111111"
    # Seven fractional digits truncated to six (microsecond resolution).
    assert stop.occurred_at == "2026-10-17T16:10:14.086700Z"
    assert stop.title_hint == "PlayedItem-0"


@pytest.mark.asyncio
async def test_fetch_activity_window_scans_all_retained_entries() -> None:
    start_dto = load_fixture("activity_log_entries.json")[0]

    def handler(request: httpx.Request) -> httpx.Response:
        offset = int(request.url.params["startIndex"])
        if offset == 0:
            items = [{**start_dto, "Id": id} for id in (103, 102, 101, 100)]
        elif offset == 4:
            # A live log that grows while we scan: the scan continues past the
            # reported total because it terminates only on an empty page.
            items = [{**start_dto, "Id": 99}]
        else:
            items = []
        return httpx.Response(200, json={"Items": items, "TotalRecordCount": 8})

    client = client_for(handler)
    try:
        all_entries, max_id = await client.fetch_activity_window()
    finally:
        await client.close()

    assert [e.entry_id for e in all_entries] == [103, 102, 101, 100, 99]
    assert max_id == 103
