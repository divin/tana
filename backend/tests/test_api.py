from __future__ import annotations

from datetime import UTC, datetime, time
from pathlib import Path
from zoneinfo import ZoneInfo

import pytest
from httpx import ASGITransport, AsyncClient
from pydantic import SecretStr

from tana.app import create_app
from tana.config import Settings
from tana.database import Database
from tana.models import SyncState, WatchedItem, WatchEvent
from tana.time_utils import format_utc, resolve_local_time


def settings() -> Settings:
    return Settings(
        _env_file=None,
        JELLYFIN_URL="http://jellyfin.example:8096",
        JELLYFIN_API_KEY=SecretStr("test-key"),
        JELLYFIN_USER_ID="test-user",
        TZ="Europe/Berlin",
    )


def media_item(
    jellyfin_id: str,
    *,
    kind: str,
    title: str,
    watched_at: str,
    play_count: int = 1,
    series_title: str | None = None,
    series_id: str | None = None,
    poster_tag: str | None = None,
    series_poster_tag: str | None = None,
    imdb_id: str | None = None,
    is_favorite: bool = False,
) -> WatchedItem:
    return WatchedItem(
        jellyfin_id=jellyfin_id,
        kind=kind,
        title=title,
        watch_status="watched",
        started_at=None,
        watched_at=watched_at,
        manually_watched_at=None,
        play_count=play_count,
        is_favorite=is_favorite,
        series_title=series_title,
        series_id=series_id,
        poster_tag=poster_tag,
        series_poster_tag=series_poster_tag,
        imdb_id=imdb_id,
        updated_at="2025-01-03T00:00:00.000000Z",
    )


@pytest.mark.asyncio
async def test_media_filter_sort_pagination_and_lifetime_rewatches(
    tmp_path: Path,
) -> None:
    app = create_app(
        settings=settings(),
        data_dir=tmp_path / "data",
        static_dir=Path(__file__).resolve().parents[2] / "frontend" / "public",
        start_background_sync=False,
    )
    async with app.router.lifespan_context(app):
        database: Database = app.state.database
        async with database.session() as session:
            session.add_all(
                [
                    media_item(
                        "00000000-0000-0000-0000-000000000001",
                        kind="movie",
                        title="Zulu",
                        watched_at="2025-01-02T22:30:00.000000Z",
                        play_count=4,
                        poster_tag="tag",
                        imdb_id="tt1234567",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000002",
                        kind="movie",
                        title="Alpha",
                        watched_at="2025-01-02T22:30:00.000000Z",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000003",
                        kind="movie",
                        title="Outside",
                        watched_at="2025-01-01T22:59:59.999999Z",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000004",
                        kind="tv",
                        title="Episode",
                        watched_at="2025-01-02T22:00:00.000000Z",
                        series_title="Example series",
                        series_id="00000000-0000-0000-0000-000000000005",
                        series_poster_tag="series-poster",
                    ),
                ]
            )
            state = await session.get(SyncState, 1)
            assert state is not None
            state.status = "success"
            state.last_started_at = "2025-01-03T00:00:00.000000Z"
            state.last_finished_at = "2025-01-03T00:01:00.000000Z"
            state.last_success_at = state.last_finished_at
            state.items_seen = 4
            state.items_imported = 4
            await session.commit()

        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            response = await client.get(
                "/api/media",
                params={
                    "start_date": "2025-01-02",
                    "end_date": "2025-01-02",
                    "limit": 1,
                },
            )
            next_page = await client.get(
                "/api/media",
                params={"start_date": "2025-01-02", "kind": "movie", "offset": 1},
            )
            tv_media = await client.get("/api/media", params={"kind": "tv"})
            stats = await client.get(
                "/api/stats",
                params={"start_date": "2025-01-02", "end_date": "2025-01-02"},
            )
            all_stats = await client.get("/api/stats")
            config = await client.get("/api/config")
            sync_status = await client.get("/api/sync/status")

    assert response.status_code == 200
    body = response.json()
    assert body["total"] == 3
    assert body["items"][0]["title"] == "Alpha"
    assert body["items"][0]["watched_at"] == "2025-01-02T22:30:00.000000Z"
    assert body["items"][0]["play_count"] == 1
    assert body["items"][0]["rewatch_count"] == 0
    assert body["items"][0]["poster_url"] is None
    next_body = next_page.json()
    assert next_body["items"][0]["title"] == "Zulu"
    assert next_body["items"][0]["play_count"] == 4
    assert next_body["items"][0]["rewatch_count"] == 3
    assert (
        next_body["items"][0]["poster_url"]
        == "/api/posters/00000000-0000-0000-0000-000000000001.jpg"
    )
    assert next_body["items"][0]["imdb_url"] == "https://www.imdb.com/title/tt1234567/"
    assert (
        tv_media.json()["items"][0]["poster_url"]
        == "/api/posters/00000000-0000-0000-0000-000000000005.jpg"
    )
    assert stats.json() == {
        "period": {"start_date": "2025-01-02", "end_date": "2025-01-02"},
        "movies": 2,
        "started_movies": 0,
        "tv_episodes": 1,
        "tv_series": 1,
        "total_items": 3,
    }
    all_stats_body = all_stats.json()
    assert all_stats_body["movies"] == 3
    assert all_stats_body["tv_episodes"] == 1
    assert "recent" in all_stats_body
    assert config.json() == {"timezone": "Europe/Berlin"}
    assert "jellyfin.example" not in config.text
    assert "test-key" not in config.text
    assert sync_status.json()["status"] == "success"


@pytest.mark.asyncio
async def test_date_filters_use_dst_adjusted_local_day_boundaries(
    tmp_path: Path,
) -> None:
    app = create_app(
        settings=settings(),
        data_dir=tmp_path / "data",
        static_dir=Path(__file__).resolve().parents[2] / "frontend" / "public",
        start_background_sync=False,
    )
    async with app.router.lifespan_context(app):
        async with app.state.database.session() as session:
            session.add_all(
                [
                    media_item(
                        "00000000-0000-0000-0000-000000000031",
                        kind="movie",
                        title="First included",
                        watched_at="2025-03-29T23:00:00.000000Z",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000032",
                        kind="movie",
                        title="Last included",
                        watched_at="2025-03-30T21:59:59.999999Z",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000033",
                        kind="movie",
                        title="First excluded",
                        watched_at="2025-03-30T22:00:00.000000Z",
                    ),
                ]
            )
            await session.commit()
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            response = await client.get(
                "/api/media",
                params={"start_date": "2025-03-30", "end_date": "2025-03-30"},
            )

    assert response.status_code == 200
    assert response.json()["total"] == 2
    assert {item["title"] for item in response.json()["items"]} == {
        "First included",
        "Last included",
    }


@pytest.mark.asyncio
async def test_stats_distinct_series_fallback_uses_casefold_and_id_prefixes(
    tmp_path: Path,
) -> None:
    app = create_app(
        settings=settings(),
        data_dir=tmp_path / "data",
        static_dir=Path(__file__).resolve().parents[2] / "frontend" / "public",
        start_background_sync=False,
    )
    async with app.router.lifespan_context(app):
        async with app.state.database.session() as session:
            session.add_all(
                [
                    media_item(
                        "00000000-0000-0000-0000-000000000011",
                        kind="tv",
                        title="Episode A",
                        watched_at="2025-01-01T00:00:00.000000Z",
                        series_title="Show",
                        series_id="series-1",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000012",
                        kind="tv",
                        title="Episode B",
                        watched_at="2025-01-01T00:00:00.000000Z",
                        series_title="Show renamed",
                        series_id="series-1",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000013",
                        kind="tv",
                        title="Episode C",
                        watched_at="2025-01-01T00:00:00.000000Z",
                        series_title="Straße",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000014",
                        kind="tv",
                        title="Episode D",
                        watched_at="2025-01-01T00:00:00.000000Z",
                        series_title="STRASSE",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000015",
                        kind="tv",
                        title="Episode E",
                        watched_at="2025-01-01T00:00:00.000000Z",
                        series_title="series-1",
                    ),
                ]
            )
            await session.commit()

        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            response = await client.get(
                "/api/stats", params={"start_date": "2025-01-01"}
            )

    assert response.status_code == 200
    assert response.json()["tv_series"] == 3
    assert "recent" not in response.json()


@pytest.mark.asyncio
async def test_api_validates_filters_dates_and_pagination(tmp_path: Path) -> None:
    app = create_app(
        settings=settings(),
        data_dir=tmp_path / "data",
        static_dir=Path(__file__).resolve().parents[2] / "frontend" / "public",
        start_background_sync=False,
    )
    async with (
        app.router.lifespan_context(app),
        AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client,
    ):
        invalid_range = await client.get(
            "/api/media",
            params={"start_date": "2025-02-02", "end_date": "2025-02-01"},
        )
        invalid_date = await client.get(
            "/api/stats", params={"start_date": "2025-02-30"}
        )
        invalid_kind = await client.get("/api/media", params={"kind": "audio"})
        invalid_limit = await client.get("/api/media", params={"limit": 201})
        invalid_offset = await client.get("/api/media", params={"offset": -1})

    assert invalid_range.status_code == 422
    assert invalid_date.status_code == 422
    assert invalid_kind.status_code == 422
    assert invalid_limit.status_code == 422
    assert invalid_offset.status_code == 422


@pytest.mark.asyncio
async def test_activity_returns_365_local_days_and_counts_items_not_plays(
    tmp_path: Path,
) -> None:
    timezone = ZoneInfo("Europe/Berlin")
    today = datetime.now(timezone).date()
    yesterday = today.fromordinal(today.toordinal() - 1)
    app = create_app(
        settings=settings(),
        data_dir=tmp_path / "data",
        static_dir=Path(__file__).resolve().parents[2] / "frontend" / "public",
        start_background_sync=False,
    )
    async with app.router.lifespan_context(app):
        async with app.state.database.session() as session:
            movie_id = "00000000-0000-0000-0000-000000000041"
            episode_id = "00000000-0000-0000-0000-000000000042"
            yesterday_id = "00000000-0000-0000-0000-000000000043"
            today_start = format_utc(
                resolve_local_time(datetime.combine(today, time.min), timezone)
            )
            today_noon = format_utc(
                resolve_local_time(datetime.combine(today, time(hour=12)), timezone)
            )
            yesterday_noon = format_utc(
                resolve_local_time(datetime.combine(yesterday, time(hour=12)), timezone)
            )
            session.add_all(
                [
                    media_item(
                        movie_id,
                        kind="movie",
                        title="Rewatched movie",
                        watched_at=today_noon,
                        play_count=12,
                    ),
                    media_item(
                        episode_id,
                        kind="tv",
                        title="Watched episode",
                        watched_at=today_noon,
                        series_title="Series",
                        series_id="series-activity",
                    ),
                    media_item(
                        yesterday_id,
                        kind="movie",
                        title="Yesterday",
                        watched_at=yesterday_noon,
                    ),
                ]
            )
            await session.flush()
            session.add_all(
                [
                    WatchEvent(
                        event_key="event-movie-first",
                        jellyfin_id=movie_id,
                        source="userdata",
                        event_date=today_start,
                    ),
                    WatchEvent(
                        event_key="event-movie-rewatch",
                        jellyfin_id=movie_id,
                        source="activity_log",
                        event_date=today_noon,
                        log_entry_id=1,
                    ),
                    WatchEvent(
                        event_key="event-episode",
                        jellyfin_id=episode_id,
                        source="activity_log",
                        event_date=today_noon,
                        log_entry_id=2,
                    ),
                    WatchEvent(
                        event_key="event-yesterday",
                        jellyfin_id=yesterday_id,
                        source="activity_log",
                        event_date=yesterday_noon,
                        log_entry_id=3,
                    ),
                ]
            )
            await session.commit()
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            response = await client.get("/api/activity")

    assert response.status_code == 200
    body = response.json()
    counts = {day["date"]: day["count"] for day in body["days"]}
    assert body["start_date"] == today.fromordinal(today.toordinal() - 364).isoformat()
    assert body["end_date"] == today.isoformat()
    assert len(body["days"]) == 365
    assert counts[today.isoformat()] == 3
    assert counts[yesterday.isoformat()] == 1
    assert sum(counts.values()) == 4
    assert (
        sum(
            count
            for day, count in counts.items()
            if day not in {today.isoformat(), yesterday.isoformat()}
        )
        == 0
    )


@pytest.mark.asyncio
async def test_stats_recent_windows_use_configured_local_calendar_dates(
    tmp_path: Path,
) -> None:
    app = create_app(
        settings=settings(),
        data_dir=tmp_path / "data",
        static_dir=Path(__file__).resolve().parents[2] / "frontend" / "public",
        start_background_sync=False,
    )
    today = datetime.now(UTC).astimezone(ZoneInfo("Europe/Berlin")).date()
    watched_today = datetime.combine(
        today, time(hour=12), tzinfo=ZoneInfo("Europe/Berlin")
    )
    async with app.router.lifespan_context(app):
        async with app.state.database.session() as session:
            session.add(
                media_item(
                    "00000000-0000-0000-0000-000000000021",
                    kind="movie",
                    title="Today",
                    watched_at=watched_today.astimezone(UTC)
                    .isoformat(timespec="microseconds")
                    .replace("+00:00", "Z"),
                )
            )
            await session.commit()
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            response = await client.get("/api/stats")

    assert response.status_code == 200
    assert response.json()["recent"]["last_7_days"]["movies"] == 1
    assert response.json()["recent"]["last_30_days"]["movies"] == 1


def started_item(
    jellyfin_id: str,
    *,
    title: str,
    started_at: str,
    watched_at: str | None = None,
    play_count: int = 0,
    is_favorite: bool = False,
) -> WatchedItem:
    return WatchedItem(
        jellyfin_id=jellyfin_id,
        kind="movie",
        title=title,
        watch_status="started",
        started_at=started_at,
        watched_at=watched_at,
        manually_watched_at=None,
        play_count=play_count,
        is_favorite=is_favorite,
        updated_at=started_at,
    )


def make_app(tmp_path: Path):
    return create_app(
        settings=settings(),
        data_dir=tmp_path / "data",
        static_dir=Path(__file__).resolve().parents[2] / "frontend" / "public",
        start_background_sync=False,
    )


@pytest.mark.asyncio
async def test_media_dto_reports_v2_status_favorite_and_manual_fields(
    tmp_path: Path,
) -> None:
    app = make_app(tmp_path)
    async with app.router.lifespan_context(app):
        async with app.state.database.session() as session:
            session.add_all(
                [
                    started_item(
                        "00000000-0000-0000-0000-000000000101",
                        title="Started film",
                        started_at="2025-01-03T10:00:00.000000Z",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000102",
                        kind="movie",
                        title="Watched film",
                        watched_at="2025-01-02T10:00:00.000000Z",
                        play_count=1,
                    ),
                ]
            )
            await session.commit()
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            response = await client.get("/api/media", params={"status": "started"})
            watched = await client.get("/api/media", params={"status": "watched"})
            all_media = await client.get("/api/media")

    assert response.status_code == 200
    started_body = response.json()
    assert started_body["total"] == 1
    row = started_body["items"][0]
    assert row["watch_status"] == "started"
    assert row["started_at"] == "2025-01-03T10:00:00.000000Z"
    assert row["watched_at"] is None
    assert row["manually_watched_at"] is None
    assert row["manually_watched"] is False
    assert row["is_favorite"] is False
    assert row["play_count"] == 0
    assert row["rewatch_count"] == 0
    assert sorted(row.keys()) == sorted(
        [
            "id",
            "kind",
            "title",
            "series_title",
            "series_id",
            "season_number",
            "episode_number",
            "watch_status",
            "started_at",
            "watched_at",
            "manually_watched_at",
            "manually_watched",
            "is_favorite",
            "play_count",
            "rewatch_count",
            "poster_url",
            "imdb_url",
        ]
    )
    assert watched.json()["total"] == 1
    assert all_media.json()["total"] == 2


@pytest.mark.asyncio
async def test_media_status_and_favorite_filters_combine(tmp_path: Path) -> None:
    app = make_app(tmp_path)
    async with app.router.lifespan_context(app):
        async with app.state.database.session() as session:
            session.add_all(
                [
                    started_item(
                        "00000000-0000-0000-0000-000000000201",
                        title="Starred started",
                        started_at="2025-01-03T10:00:00.000000Z",
                        is_favorite=True,
                    ),
                    started_item(
                        "00000000-0000-0000-0000-000000000202",
                        title="Plain started",
                        started_at="2025-01-03T11:00:00.000000Z",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000203",
                        kind="movie",
                        title="Starred watched",
                        watched_at="2025-01-01T10:00:00.000000Z",
                        is_favorite=True,
                    ),
                ]
            )
            await session.commit()
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            favorited = await client.get("/api/media", params={"favorite": "true"})
            requested_false = await client.get(
                "/api/media", params={"favorite": "false"}
            )
            started_favorite = await client.get(
                "/api/media", params={"status": "started", "favorite": "true"}
            )
            invalid_status = await client.get("/api/media", params={"status": "paused"})
            invalid_favorite = await client.get(
                "/api/media", params={"favorite": "maybe"}
            )

    assert [item["title"] for item in favorited.json()["items"]] == [
        "Starred started",
        "Starred watched",
    ]
    assert requested_false.json()["total"] == 1
    assert started_favorite.json()["total"] == 1
    assert invalid_status.status_code == 422
    assert invalid_favorite.status_code == 422


@pytest.mark.asyncio
async def test_media_date_filters_use_current_state_timestamps(tmp_path: Path) -> None:
    app = make_app(tmp_path)
    async with app.router.lifespan_context(app):
        async with app.state.database.session() as session:
            session.add_all(
                [
                    started_item(
                        "00000000-0000-0000-0000-000000000301",
                        title="Started in window",
                        started_at="2025-01-02T10:00:00.000000Z",
                    ),
                    started_item(
                        "00000000-0000-0000-0000-000000000302",
                        title="Started outside",
                        started_at="2025-01-10T10:00:00.000000Z",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000303",
                        kind="movie",
                        title="Watched in window",
                        watched_at="2025-01-02T09:00:00.000000Z",
                    ),
                ]
            )
            await session.commit()
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            response = await client.get(
                "/api/media",
                params={"start_date": "2025-01-02", "end_date": "2025-01-02"},
            )

    assert response.status_code == 200
    assert {item["title"] for item in response.json()["items"]} == {
        "Started in window",
        "Watched in window",
    }


@pytest.mark.asyncio
async def test_stats_include_started_movies_in_periods_and_recent_windows(
    tmp_path: Path,
) -> None:
    app = make_app(tmp_path)
    today = datetime.now(UTC).astimezone(ZoneInfo("Europe/Berlin")).date()
    started_today = (
        datetime.combine(today, time(hour=10), tzinfo=ZoneInfo("Europe/Berlin"))
        .astimezone(UTC)
        .isoformat(timespec="microseconds")
        .replace("+00:00", "Z")
    )
    watched_today = (
        datetime.combine(today, time(hour=9), tzinfo=ZoneInfo("Europe/Berlin"))
        .astimezone(UTC)
        .isoformat(timespec="microseconds")
        .replace("+00:00", "Z")
    )
    async with app.router.lifespan_context(app):
        async with app.state.database.session() as session:
            session.add_all(
                [
                    started_item(
                        "00000000-0000-0000-0000-000000000401",
                        title="Started",
                        started_at=started_today,
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000402",
                        kind="movie",
                        title="Watched",
                        watched_at=watched_today,
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000403",
                        kind="movie",
                        title="Older watched",
                        watched_at="2024-12-01T09:00:00.000000Z",
                    ),
                ]
            )
            await session.commit()
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            period = await client.get(
                "/api/stats", params={"start_date": today.isoformat()}
            )
            all_stats = await client.get("/api/stats")

    assert period.json()["movies"] == 1
    assert period.json()["started_movies"] == 1
    assert period.json()["total_items"] == 2
    body = all_stats.json()
    assert body["started_movies"] == 1
    assert body["recent"]["last_7_days"]["started_movies"] == 1
    assert body["recent"]["last_30_days"]["started_movies"] == 1


@pytest.mark.asyncio
async def test_patch_latest_favorite_sets_and_unsets_on_movies_and_episodes(
    tmp_path: Path,
) -> None:
    app = make_app(tmp_path)
    async with app.router.lifespan_context(app):
        async with app.state.database.session() as session:
            session.add_all(
                [
                    started_item(
                        "00000000-0000-0000-0000-000000000501",
                        title="Started film",
                        started_at="2025-01-02T10:00:00.000000Z",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000502",
                        kind="tv",
                        title="Episode",
                        watched_at="2025-01-02T09:00:00.000000Z",
                        series_title="Series",
                        series_id="00000000-0000-0000-0000-000000000005",
                    ),
                ]
            )
            await session.commit()
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            set_movie = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000501",
                json={"is_favorite": True},
            )
            fav_twice = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000501",
                json={"is_favorite": True},
            )
            set_episode = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000502",
                json={"is_favorite": True},
            )
            unset_movie = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000501",
                json={"is_favorite": False},
            )
            all_media = await client.get("/api/media")

    assert set_movie.status_code == fav_twice.status_code == 200
    assert set_movie.json()["is_favorite"] is True
    assert set_episode.status_code == 200
    assert unset_movie.status_code == 200
    assert unset_movie.json()["is_favorite"] is False
    favorite_rows = [item for item in all_media.json()["items"] if item["is_favorite"]]
    assert [item["id"] for item in favorite_rows] == [
        "00000000-0000-0000-0000-000000000502"
    ]


@pytest.mark.asyncio
async def test_patch_manual_mark_undo_and_state_restoration(tmp_path: Path) -> None:
    app = make_app(tmp_path)
    async with app.router.lifespan_context(app):
        async with app.state.database.session() as session:
            session.add(
                started_item(
                    "00000000-0000-0000-0000-000000000601",
                    title="Started film",
                    started_at="2025-01-02T10:00:00.000000Z",
                )
            )
            await session.commit()
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            marked = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000601",
                json={"manually_watched": True},
            )
            repeated = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000601",
                json={"manually_watched": True},
            )
            stats_while_marked = await client.get("/api/stats")
            activity = await client.get("/api/activity")
            undone = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000601",
                json={"manually_watched": False},
            )
            repeated_false = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000601",
                json={"manually_watched": False},
            )

    assert marked.status_code == 200
    row = marked.json()
    assert row["watch_status"] == "watched"
    assert row["manually_watched"] is True
    assert row["manually_watched_at"] is not None
    assert row["started_at"] == "2025-01-02T10:00:00.000000Z"
    assert row["watched_at"] is None
    assert row["play_count"] == 0
    assert repeated.status_code == 200
    assert repeated.json()["manually_watched_at"] == row["manually_watched_at"]
    assert stats_while_marked.json()["movies"] == 1
    assert stats_while_marked.json()["started_movies"] == 0
    assert sum(day["count"] for day in activity.json()["days"]) == 0
    assert undone.status_code == 200
    assert undone.json()["manually_watched"] is False
    assert undone.json()["watch_status"] == "started"
    assert undone.json()["started_at"] == "2025-01-02T10:00:00.000000Z"
    assert repeated_false.status_code == 200
    assert repeated_false.json()["watch_status"] == "started"


@pytest.mark.asyncio
async def test_patch_manual_conflicts_and_tv_rejections(tmp_path: Path) -> None:
    app = make_app(tmp_path)
    async with app.router.lifespan_context(app):
        async with app.state.database.session() as session:
            session.add_all(
                [
                    media_item(
                        "00000000-0000-0000-0000-000000000701",
                        kind="movie",
                        title="Verified watched",
                        watched_at="2025-01-02T09:00:00.000000Z",
                    ),
                    media_item(
                        "00000000-0000-0000-0000-000000000702",
                        kind="tv",
                        title="Episode",
                        watched_at="2025-01-02T09:00:00.000000Z",
                        series_title="Series",
                        series_id="00000000-0000-0000-0000-000000000005",
                    ),
                ]
            )
            await session.commit()
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            conflict = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000701",
                json={"manually_watched": True},
            )
            tv_mark = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000702",
                json={"manually_watched": True},
            )
            tv_unmark = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000702",
                json={"manually_watched": False},
            )
            combined_invalid = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000701",
                json={"is_favorite": True, "manually_watched": True},
            )
            all_media = await client.get("/api/media")

    assert conflict.status_code == 409
    assert conflict.json() == {
        "detail": "Manual watched action conflicts with current state"
    }
    assert tv_mark.status_code == 422
    assert tv_unmark.status_code == 422
    assert combined_invalid.status_code == 409
    movie = next(
        row
        for row in all_media.json()["items"]
        if row["id"] == "00000000-0000-0000-0000-000000000701"
    )
    assert movie["is_favorite"] is False


@pytest.mark.asyncio
async def test_patch_validates_bodies_ids_and_missing_rows(tmp_path: Path) -> None:
    app = make_app(tmp_path)
    async with app.router.lifespan_context(app):
        async with app.state.database.session() as session:
            session.add(
                started_item(
                    "00000000-0000-0000-0000-000000000801",
                    title="Started film",
                    started_at="2025-01-02T10:00:00.000000Z",
                )
            )
            await session.commit()
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            empty = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000801", json={}
            )
            null_value = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000801",
                json={"is_favorite": None},
            )
            non_boolean = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000801",
                json={"is_favorite": "yes"},
            )
            unknown_key = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000801",
                json={"favorite": True},
            )
            malformed_id = await client.patch(
                "/api/media/not-a-uuid", json={"is_favorite": True}
            )
            missing_row = await client.patch(
                "/api/media/00000000-0000-0000-0000-000000000999",
                json={"is_favorite": True},
            )

    assert empty.status_code == 422
    assert null_value.status_code == 422
    assert non_boolean.status_code == 422
    assert unknown_key.status_code == 422
    assert malformed_id.status_code == 404
    assert malformed_id.json() == {"detail": "Media item not found"}
    assert missing_row.status_code == 404
    assert missing_row.json() == {"detail": "Media item not found"}
