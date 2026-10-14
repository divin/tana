from __future__ import annotations

from datetime import datetime, timedelta
from pathlib import Path
from zoneinfo import ZoneInfo

import pytest
from httpx import ASGITransport, AsyncClient
from pydantic import SecretStr
from sqlmodel import select

from tana.app import create_app
from tana.config import Settings
from tana.jellyfin import JellyfinClient, JellyfinError, LibraryItemData
from tana.models import CompletedPlayback, SyncState, WatchedItem
from tana.webhooks import MAX_WEBHOOK_BYTES

USER_ID = "00000000000000000000000000000111"
OTHER_USER_ID = "00000000000000000000000000000112"
MOVIE_ID = "6f9619ff8b86d011b42d00cf4fc964ff"
CANONICAL_MOVIE_ID = "6f9619ff-8b86-d011-b42d-00cf4fc964ff"
EPISODE_ID = "6f9619ff8b86d011b42d00cf4fc96400"
LOCAL_TIMEZONE = ZoneInfo("Europe/Berlin")


def settings(completion_threshold: float = 1.0) -> Settings:
    return Settings(
        _env_file=None,
        JELLYFIN_URL="http://jellyfin.example:8096",
        JELLYFIN_API_KEY=SecretStr("test-key"),
        JELLYFIN_USER_ID=USER_ID,
        TZ="Europe/Berlin",
        PLAYBACK_COMPLETION_THRESHOLD=completion_threshold,
    )


def completion_time(days: int = 0, minutes: int = 0) -> str:
    now = datetime.now(LOCAL_TIMEZONE)
    return (
        now.replace(hour=12, minute=30, second=0, microsecond=0).add
        if False
        else (
            now.replace(hour=12, minute=30, second=0, microsecond=0)
            + timedelta(days=days, minutes=minutes)
        ).isoformat()
    )


def movie_event(
    *,
    completed_at: str | None = None,
    session_id: str = "session-1",
    user_id: str = USER_ID,
    complete: bool = True,
    position_ticks: int = 9_000_000_000,
    run_time_ticks: int | None = None,
) -> dict[str, object]:
    return {
        "NotificationType": "PlaybackStop",
        "PlayedToCompletion": complete,
        "UserId": user_id,
        "ItemId": MOVIE_ID,
        "ItemType": "Movie",
        "Name": "Completed movie",
        "UtcTimestamp": completed_at or completion_time(),
        "SessionId": session_id,
        "PlaybackPositionTicks": position_ticks,
        **({"RunTimeTicks": run_time_ticks} if run_time_ticks is not None else {}),
        "Provider_imdb": "tt1234567",
    }


@pytest.fixture(autouse=True)
def _stub_jellyfin_enrichment(monkeypatch: pytest.MonkeyPatch) -> None:
    """Enrichment is disabled by default so tests never reach Jellyfin."""

    async def no_enrichment(_self: JellyfinClient, _item_id: str) -> None:
        return None

    monkeypatch.setattr(JellyfinClient, "fetch_item_details", no_enrichment)


async def create_test_app(tmp_path: Path, completion_threshold: float = 1.0):
    return create_app(
        settings=settings(completion_threshold=completion_threshold),
        data_dir=tmp_path / "data",
        static_dir=Path(__file__).resolve().parents[2] / "frontend" / "public",
        start_background_sync=False,
    )


@pytest.mark.asyncio
async def test_completed_webhooks_create_and_increment_only_unique_plays(
    tmp_path: Path,
) -> None:
    app = await create_test_app(tmp_path)
    async with app.router.lifespan_context(app):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            first = await client.post("/api/webhooks/jellyfin", json=movie_event())
            duplicate = await client.post("/api/webhooks/jellyfin", json=movie_event())
            second = await client.post(
                "/api/webhooks/jellyfin",
                json=movie_event(
                    completed_at=completion_time(minutes=1), session_id="session-2"
                ),
            )
            media = await client.get("/api/media")
            stats = await client.get("/api/stats")
            activity = await client.get("/api/activity")

        async with app.state.database.session() as session:
            events = (await session.exec(select(CompletedPlayback))).all()
            state = await session.get(SyncState, 1)

    assert first.status_code == duplicate.status_code == second.status_code == 204
    assert first.content == duplicate.content == second.content == b""
    assert media.status_code == stats.status_code == activity.status_code == 200
    item = next(row for row in media.json()["items"] if row["id"] == CANONICAL_MOVIE_ID)
    assert item["play_count"] == 2
    assert item["rewatch_count"] == 1
    assert item["imdb_url"] == "https://www.imdb.com/title/tt1234567/"
    assert stats.json()["movies"] == 1
    assert sum(day["count"] for day in activity.json()["days"]) == 2
    assert len(events) == 2
    assert state is not None
    assert state.status == "never"


@pytest.mark.asyncio
async def test_incomplete_wrong_user_and_other_notification_events_are_ignored(
    tmp_path: Path,
) -> None:
    app = await create_test_app(tmp_path)
    async with app.router.lifespan_context(app):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            partial = await client.post(
                "/api/webhooks/jellyfin", json=movie_event(complete=False)
            )
            other_user = await client.post(
                "/api/webhooks/jellyfin", json=movie_event(user_id=OTHER_USER_ID)
            )
            other_event = await client.post(
                "/api/webhooks/jellyfin",
                json={
                    "NotificationType": "PlaybackProgress",
                    "PlayedToCompletion": True,
                },
            )
            media = await client.get("/api/media")

        async with app.state.database.session() as session:
            events = (await session.exec(select(CompletedPlayback))).all()
            items = (await session.exec(select(WatchedItem))).all()

    assert (
        partial.status_code == other_user.status_code == other_event.status_code == 204
    )
    assert media.json()["total"] == 0
    assert events == []
    assert items == []


@pytest.mark.asyncio
async def test_episode_completion_maps_series_fields_and_canonical_uuids(
    tmp_path: Path,
) -> None:
    app = await create_test_app(tmp_path)
    payload = {
        "NotificationType": "PlaybackStop",
        "PlayedToCompletion": True,
        "UserId": USER_ID,
        "ItemId": EPISODE_ID,
        "ItemType": "Episode",
        "Name": "Pilot",
        "SeriesName": "Example series",
        "SeriesId": "00000000000000000000000000000222",
        "SeasonNumber": 1,
        "EpisodeNumber": 1,
        "UtcTimestamp": completion_time(),
        "SessionId": "episode-session",
        "PlaybackPositionTicks": 100,
    }
    async with (
        app.router.lifespan_context(app),
        AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client,
    ):
        response = await client.post("/api/webhooks/jellyfin", json=payload)
        media = await client.get("/api/media", params={"kind": "tv"})

    assert response.status_code == 204
    episode = media.json()["items"][0]
    assert episode["kind"] == "tv"
    assert episode["series_title"] == "Example series"
    assert episode["series_id"] == "00000000-0000-0000-0000-000000000222"
    assert episode["watched_at"].endswith("Z")
    assert episode["play_count"] == 1
    assert episode["rewatch_count"] == 0
    assert episode["poster_url"] is None


@pytest.mark.asyncio
async def test_webhook_validates_content_type_payload_and_size_without_auth(
    tmp_path: Path,
) -> None:
    app = await create_test_app(tmp_path)
    async with (
        app.router.lifespan_context(app),
        AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client,
    ):
        missing_content_type = await client.post(
            "/api/webhooks/jellyfin",
            content=b"{}",
            headers={"Content-Type": "text/plain"},
        )
        malformed = await client.post(
            "/api/webhooks/jellyfin",
            content=b"not json",
            headers={"Content-Type": "application/json"},
        )
        oversized = await client.post(
            "/api/webhooks/jellyfin",
            content=b" " * (MAX_WEBHOOK_BYTES + 1),
            headers={"Content-Type": "application/json"},
        )
        missing_timestamp = await client.post(
            "/api/webhooks/jellyfin",
            json={
                "NotificationType": "PlaybackStop",
                "PlayedToCompletion": True,
                "UserId": USER_ID,
                "ItemId": MOVIE_ID,
                "ItemType": "Movie",
                "Name": "Malformed event",
            },
        )
        valid = await client.post("/api/webhooks/jellyfin", json=movie_event())

    assert missing_content_type.status_code == 415
    assert malformed.status_code == 400
    assert oversized.status_code == 413
    assert missing_timestamp.status_code == 400
    assert valid.status_code == 204


@pytest.mark.asyncio
async def test_out_of_order_completion_counts_but_does_not_move_last_watched_back(
    tmp_path: Path,
) -> None:
    now = datetime.now(LOCAL_TIMEZONE).replace(
        hour=12, minute=30, second=0, microsecond=0
    )
    later = (now + timedelta(minutes=1)).isoformat()
    earlier = now.isoformat()
    app = await create_test_app(tmp_path)
    async with (
        app.router.lifespan_context(app),
        AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client,
    ):
        later_response = await client.post(
            "/api/webhooks/jellyfin",
            json=movie_event(completed_at=later, session_id="later"),
        )
        earlier_response = await client.post(
            "/api/webhooks/jellyfin",
            json=movie_event(completed_at=earlier, session_id="earlier"),
        )
        media = await client.get("/api/media")

    assert later_response.status_code == earlier_response.status_code == 204
    row = media.json()["items"][0]
    assert row["play_count"] == 2
    assert row["rewatch_count"] == 1
    assert row["watched_at"].endswith("Z")


@pytest.mark.asyncio
async def test_unflagged_stop_at_full_runtime_counts_with_default_threshold(
    tmp_path: Path,
) -> None:
    app = await create_test_app(tmp_path)
    async with (
        app.router.lifespan_context(app),
        AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client,
    ):
        full = await client.post(
            "/api/webhooks/jellyfin",
            json=movie_event(
                complete=False,
                position_ticks=10_000_000_000,
                run_time_ticks=10_000_000_000,
                session_id="full",
            ),
        )
        short = await client.post(
            "/api/webhooks/jellyfin",
            json=movie_event(
                complete=False,
                position_ticks=9_000_000_000,
                run_time_ticks=10_000_000_000,
                session_id="short",
            ),
        )
        media = await client.get("/api/media")

    assert full.status_code == short.status_code == 204
    assert media.json()["total"] == 1
    row = media.json()["items"][0]
    assert row["id"] == CANONICAL_MOVIE_ID
    assert row["play_count"] == 1
    assert row["rewatch_count"] == 0


@pytest.mark.asyncio
async def test_unflagged_stop_respects_configured_threshold(tmp_path: Path) -> None:
    app = await create_test_app(tmp_path, completion_threshold=0.9)
    async with (
        app.router.lifespan_context(app),
        AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client,
    ):
        reaches = await client.post(
            "/api/webhooks/jellyfin",
            json=movie_event(
                complete=False,
                position_ticks=9_000_000_000,
                run_time_ticks=10_000_000_000,
                session_id="reaches",
            ),
        )
        below = await client.post(
            "/api/webhooks/jellyfin",
            json=movie_event(
                complete=False,
                position_ticks=8_000_000_000,
                run_time_ticks=10_000_000_000,
                session_id="below",
            ),
        )
        media = await client.get("/api/media")

    assert reaches.status_code == below.status_code == 204
    assert media.json()["total"] == 1
    row = media.json()["items"][0]
    assert row["id"] == CANONICAL_MOVIE_ID
    assert row["play_count"] == 1
    assert row["rewatch_count"] == 0


@pytest.mark.asyncio
async def test_new_movie_row_is_enriched_with_poster_and_imdb(
    tmp_path: Path,
) -> None:
    app = await create_test_app(tmp_path)
    calls = 0

    async def enrich(item_id: str) -> LibraryItemData:
        nonlocal calls
        calls += 1
        return LibraryItemData(
            jellyfin_id=item_id,
            kind="movie",
            title="Prometheus",
            poster_tag="enriched-tag",
            imdb_id="tt1446714",
        )

    async with (
        app.router.lifespan_context(app),
        AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client,
    ):
        app.state.jellyfin.fetch_item_details = enrich
        response = await client.post("/api/webhooks/jellyfin", json=movie_event())
        media = await client.get("/api/media")

    assert response.status_code == 204
    assert calls == 1
    row = media.json()["items"][0]
    assert row["poster_url"] == f"/api/posters/{CANONICAL_MOVIE_ID}.jpg"
    assert row["imdb_url"] == "https://www.imdb.com/title/tt1446714/"


@pytest.mark.asyncio
async def test_enrichment_failure_still_records_webhook_row(tmp_path: Path) -> None:
    app = await create_test_app(tmp_path)
    calls = 0

    async def fail(_item_id: str) -> LibraryItemData:
        nonlocal calls
        calls += 1
        raise JellyfinError("jellyfin unavailable")

    async with (
        app.router.lifespan_context(app),
        AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client,
    ):
        app.state.jellyfin.fetch_item_details = fail
        response = await client.post("/api/webhooks/jellyfin", json=movie_event())
        media = await client.get("/api/media")

    assert response.status_code == 204
    assert calls == 1
    assert media.json()["total"] == 1
    row = media.json()["items"][0]
    assert row["poster_url"] is None
    assert row["play_count"] == 1
    assert row["imdb_url"] == "https://www.imdb.com/title/tt1234567/"


@pytest.mark.asyncio
async def test_rewatch_and_duplicate_deliveries_skip_enrichment(tmp_path: Path) -> None:
    app = await create_test_app(tmp_path)
    calls = 0

    async def enrich(item_id: str) -> LibraryItemData:
        nonlocal calls
        calls += 1
        return LibraryItemData(
            jellyfin_id=item_id,
            kind="movie",
            title="Enriched movie",
            poster_tag="enriched-tag",
        )

    async with (
        app.router.lifespan_context(app),
        AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client,
    ):
        app.state.jellyfin.fetch_item_details = enrich
        first = await client.post(
            "/api/webhooks/jellyfin", json=movie_event(session_id="first")
        )
        duplicate = await client.post(
            "/api/webhooks/jellyfin", json=movie_event(session_id="first")
        )
        rewatch = await client.post(
            "/api/webhooks/jellyfin",
            json=movie_event(
                session_id="rewatch",
                completed_at=completion_time(minutes=1),
            ),
        )
        media = await client.get("/api/media")

    assert first.status_code == duplicate.status_code == rewatch.status_code == 204
    assert calls == 1
    row = media.json()["items"][0]
    assert row["play_count"] == 2
    assert row["rewatch_count"] == 1


@pytest.mark.asyncio
async def test_new_tv_row_uses_series_poster_tag_from_enrichment(
    tmp_path: Path,
) -> None:
    app = await create_test_app(tmp_path)

    async def enrich(_item_id: str) -> LibraryItemData:
        return LibraryItemData(
            jellyfin_id=EPISODE_ID,
            kind="tv",
            title="Pilot",
            series_title="Example series",
            series_id="00000000000000000000000000000222",
            series_poster_tag="series-tag",
            season_number=1,
            episode_number=1,
        )

    payload = {
        "NotificationType": "PlaybackStop",
        "PlayedToCompletion": True,
        "UserId": USER_ID,
        "ItemId": EPISODE_ID,
        "ItemType": "Episode",
        "Name": "Pilot",
        "SeriesName": "Example series",
        "SeriesId": "00000000000000000000000000000222",
        "SeasonNumber": 1,
        "EpisodeNumber": 1,
        "UtcTimestamp": completion_time(),
        "SessionId": "episode-session",
        "PlaybackPositionTicks": 10_000_000_000,
        "RunTimeTicks": 10_000_000_000,
    }
    async with (
        app.router.lifespan_context(app),
        AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client,
    ):
        app.state.jellyfin.fetch_item_details = enrich
        response = await client.post("/api/webhooks/jellyfin", json=payload)
        media = await client.get("/api/media", params={"kind": "tv"})

    assert response.status_code == 204
    episode = media.json()["items"][0]
    assert (
        episode["poster_url"] == "/api/posters/00000000-0000-0000-0000-000000000222.jpg"
    )
    assert episode["play_count"] == 1
