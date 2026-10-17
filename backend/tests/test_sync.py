from __future__ import annotations

import asyncio
from pathlib import Path

import httpx
import pytest
from pydantic import SecretStr
from sqlalchemy import event
from sqlalchemy.exc import SQLAlchemyError
from sqlmodel import select

from tana.config import Settings
from tana.database import Database
from tana.jellyfin import (
    JellyfinClient,
    JellyfinError,
    JellyfinTransportError,
    LibraryItemData,
)
from tana.models import SyncState, WatchedItem
from tana.sync import SyncService

MOVIE_ID = "6f9619ff-8b86-d011-b42d-00cf4fc964ff"
EPISODE_ID = "6f9619ff-8b86-d011-b42d-00cf4fc96400"
COMPLETED_AT = "2026-10-11T12:30:00.000000Z"
OLD_COMPLETED_AT = "2025-01-02T03:04:05.000000Z"


def metadata(
    item_id: str = MOVIE_ID,
    *,
    title: str = "Example movie",
    kind: str = "movie",
    series_title: str | None = None,
    series_id: str | None = None,
    series_poster_tag: str | None = None,
    poster_tag: str | None = "poster-tag",
    imdb_id: str | None = "tt1234567",
) -> LibraryItemData:
    return LibraryItemData(
        jellyfin_id=item_id,
        kind=kind,
        title=title,
        poster_tag=poster_tag if kind == "movie" else None,
        series_poster_tag=series_poster_tag if kind == "tv" else None,
        imdb_id=imdb_id if kind == "movie" else None,
        series_title=series_title,
        series_id=series_id,
        season_number=2 if kind == "tv" else None,
        episode_number=3 if kind == "tv" else None,
    )


class FakeJellyfin:
    def __init__(self, result=None, error: Exception | None = None) -> None:
        self.result = result or ([], 0)
        self.error = error
        self.calls = 0

    async def fetch_library_items(self):
        self.calls += 1
        if self.error:
            raise self.error
        return self.result


async def make_database(tmp_path: Path) -> Database:
    database = Database(tmp_path / "data")
    await database.initialize()
    return database


async def seed_watched_item(
    database: Database,
    *,
    item_id: str = MOVIE_ID,
    kind: str = "movie",
    title: str = "Previously completed",
    watched_at: str = OLD_COMPLETED_AT,
    play_count: int = 3,
) -> None:
    async with database.session() as session:
        session.add(
            WatchedItem(
                jellyfin_id=item_id,
                kind=kind,
                title=title,
                watch_status="watched",
                started_at=None,
                watched_at=watched_at,
                manually_watched_at=None,
                play_count=play_count,
                is_favorite=False,
                series_title="Example series" if kind == "tv" else None,
                series_id="00000000-0000-0000-0000-000000000222"
                if kind == "tv"
                else None,
                updated_at=OLD_COMPLETED_AT,
            )
        )
        await session.commit()


@pytest.mark.asyncio
async def test_sync_refreshes_metadata_without_overwriting_completion_data(
    tmp_path: Path,
) -> None:
    database = await make_database(tmp_path)
    try:
        await seed_watched_item(database)
        await seed_watched_item(
            database,
            item_id=EPISODE_ID,
            kind="tv",
            title="Old episode title",
        )
        jellyfin = FakeJellyfin(
            (
                [
                    metadata(title="Old metadata"),
                    metadata(title="Refreshed movie title"),
                    metadata(
                        EPISODE_ID,
                        title="Refreshed episode title",
                        kind="tv",
                        series_title="Refreshed series",
                        series_id="00000000-0000-0000-0000-000000000222",
                        series_poster_tag="series-tag",
                        poster_tag=None,
                        imdb_id=None,
                    ),
                    metadata("00000000-0000-0000-0000-000000000099"),
                ],
                9,
            )
        )
        service = SyncService(database, jellyfin)  # type: ignore[arg-type]

        assert await service.run() is True

        async with database.session() as session:
            movie = await session.get(WatchedItem, MOVIE_ID)
            episode = await session.get(WatchedItem, EPISODE_ID)
            unknown = await session.get(
                WatchedItem, "00000000-0000-0000-0000-000000000099"
            )
            state = await session.get(SyncState, 1)
            all_items = (await session.exec(select(WatchedItem))).all()

        assert movie is not None
        assert movie.title == "Refreshed movie title"
        assert movie.watched_at == OLD_COMPLETED_AT
        assert movie.play_count == 3
        assert movie.poster_tag == "poster-tag"
        assert movie.imdb_id == "tt1234567"
        assert episode is not None
        assert episode.title == "Refreshed episode title"
        assert episode.watched_at == OLD_COMPLETED_AT
        assert episode.play_count == 3
        assert episode.series_title == "Refreshed series"
        assert episode.series_poster_tag == "series-tag"
        assert episode.season_number == 2
        assert episode.episode_number == 3
        assert unknown is None
        assert len(all_items) == 2
        assert state is not None
        assert state.status == "success"
        assert state.last_success_at == state.last_finished_at
        assert state.items_seen == 9
        assert state.items_imported == 2
    finally:
        await database.dispose()


@pytest.mark.asyncio
async def test_sync_does_not_create_historical_rows_from_played_library(
    tmp_path: Path,
) -> None:
    database = await make_database(tmp_path)
    try:
        service = SyncService(
            database,
            FakeJellyfin(([metadata(), metadata(EPISODE_ID, kind="tv")], 2)),  # type: ignore[arg-type]
        )
        assert await service.run() is True

        async with database.session() as session:
            items = (await session.exec(select(WatchedItem))).all()
            state = await session.get(SyncState, 1)

        assert items == []
        assert state is not None
        assert state.status == "success"
        assert state.items_seen == 2
        assert state.items_imported == 0
    finally:
        await database.dispose()


@pytest.mark.asyncio
async def test_failed_fetch_preserves_items_and_records_safe_failure(
    tmp_path: Path,
) -> None:
    database = await make_database(tmp_path)
    try:
        await seed_watched_item(database)
        service = SyncService(
            database, FakeJellyfin(error=JellyfinError("private server details"))
        )  # type: ignore[arg-type]
        assert await service.run() is False

        async with database.session() as session:
            movie = await session.get(WatchedItem, MOVIE_ID)
            state = await session.get(SyncState, 1)

        assert movie is not None
        assert movie.title == "Previously completed"
        assert movie.watched_at == OLD_COMPLETED_AT
        assert movie.play_count == 3
        assert state is not None
        assert state.status == "failed"
        assert state.last_error == "Jellyfin request failed"
    finally:
        await database.dispose()


@pytest.mark.asyncio
async def test_failed_sync_reports_safe_actionable_jellyfin_error(
    tmp_path: Path,
) -> None:
    database = await make_database(tmp_path)
    try:
        service = SyncService(
            database,
            FakeJellyfin(error=JellyfinTransportError("Jellyfin returned HTTP 401")),  # type: ignore[arg-type]
        )
        assert await service.run() is False

        async with database.session() as session:
            state = await session.get(SyncState, 1)

        assert state is not None
        assert state.last_error == (
            "Jellyfin returned HTTP 401; check the API key and user access"
        )
    finally:
        await database.dispose()


def test_sync_reports_sanitized_network_failure_category() -> None:
    message = SyncService._safe_error(
        JellyfinTransportError("Jellyfin could not be reached after 3 attempts")
    )

    assert message == (
        "Jellyfin could not be reached after retries; check the server URL "
        "and network availability"
    )


@pytest.mark.asyncio
async def test_second_jellyfin_query_failure_does_not_commit_metadata(
    tmp_path: Path,
) -> None:
    database = await make_database(tmp_path)
    await seed_watched_item(database)
    settings = Settings(
        _env_file=None,
        JELLYFIN_URL="http://jellyfin.example:8096",
        JELLYFIN_API_KEY=SecretStr("test-key"),
        JELLYFIN_USER_ID="00000000-0000-0000-0000-000000000111",
        TZ="UTC",
    )
    request_count = 0

    def handler(request: httpx.Request) -> httpx.Response:
        nonlocal request_count
        request_count += 1
        if request.url.params["includeItemTypes"] == "Movie":
            return httpx.Response(
                200,
                json={
                    "Items": [
                        {
                            "Id": MOVIE_ID,
                            "Name": "Changed from Jellyfin",
                            "Type": "Movie",
                        }
                    ],
                    "TotalRecordCount": 1,
                },
            )
        return httpx.Response(500)

    async def no_wait(_seconds: float) -> None:
        return

    jellyfin = JellyfinClient(
        settings,
        client=httpx.AsyncClient(transport=httpx.MockTransport(handler)),
        sleep=no_wait,
    )
    try:
        assert await SyncService(database, jellyfin).run() is False

        async with database.session() as session:
            movie = await session.get(WatchedItem, MOVIE_ID)
            state = await session.get(SyncState, 1)

        assert request_count == 4
        assert movie is not None
        assert movie.title == "Previously completed"
        assert movie.play_count == 3
        assert state is not None
        assert state.status == "failed"
    finally:
        await jellyfin.client.aclose()
        await database.dispose()


@pytest.mark.asyncio
async def test_metadata_database_failure_rolls_back_refresh(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    try:
        await seed_watched_item(database)
        assert database.engine is not None

        def fail_watched_item_update(
            _connection, _cursor, statement, _parameters, _context, _executemany
        ):
            if statement.startswith("UPDATE watched_items"):
                raise SQLAlchemyError("synthetic write failure")

        event.listen(
            database.engine.sync_engine,
            "before_cursor_execute",
            fail_watched_item_update,
        )
        service = SyncService(database, FakeJellyfin(([metadata(title="Changed")], 1)))  # type: ignore[arg-type]
        assert await service.run() is False

        async with database.session() as session:
            movie = await session.get(WatchedItem, MOVIE_ID)
            state = await session.get(SyncState, 1)

        assert movie is not None
        assert movie.title == "Previously completed"
        assert movie.play_count == 3
        assert state is not None
        assert state.status == "failed"
        assert state.last_error == "Database write failed"
    finally:
        await database.dispose()


@pytest.mark.asyncio
async def test_sync_serializes_concurrent_runs(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    try:
        await seed_watched_item(database)
        active = 0
        maximum_active = 0

        class SlowJellyfin:
            async def fetch_library_items(self):
                nonlocal active, maximum_active
                active += 1
                maximum_active = max(maximum_active, active)
                await asyncio.sleep(0.02)
                active -= 1
                return [metadata()], 1

        service = SyncService(database, SlowJellyfin())  # type: ignore[arg-type]
        results = await asyncio.gather(service.run(), service.run())

        assert results == [True, True]
        assert maximum_active == 1
    finally:
        await database.dispose()


@pytest.mark.asyncio
async def test_metadata_sync_preserves_started_manual_and_favorite_state(
    tmp_path: Path,
) -> None:
    database = await make_database(tmp_path)
    try:
        async with database.session() as session:
            session.add_all(
                [
                    WatchedItem(
                        jellyfin_id="6f9619ff-8b86-d011-b42d-00cf4fc965ff",
                        kind="movie",
                        title="Started film",
                        watch_status="started",
                        started_at="2026-10-15T10:00:00.000000Z",
                        watched_at=OLD_COMPLETED_AT,
                        play_count=1,
                        is_favorite=False,
                        poster_tag="old-ag-tag",
                        updated_at=OLD_COMPLETED_AT,
                    ),
                    WatchedItem(
                        jellyfin_id="6f9619ff-8b86-d011-b42d-00cf4fc966ff",
                        kind="movie",
                        title="Manual film",
                        watch_status="watched",
                        started_at="2026-10-15T11:00:00.000000Z",
                        watched_at=None,
                        manually_watched_at="2026-10-15T11:30:00.000000Z",
                        play_count=0,
                        is_favorite=True,
                        updated_at="2026-10-15T11:30:00.000000Z",
                    ),
                    WatchedItem(
                        jellyfin_id=EPISODE_ID,
                        kind="tv",
                        title="Episode",
                        watch_status="watched",
                        watched_at=OLD_COMPLETED_AT,
                        play_count=2,
                        is_favorite=True,
                        series_title="Series",
                        series_id="00000000-0000-0000-0000-000000000222",
                        season_number=1,
                        episode_number=2,
                        series_poster_tag="old-series-tag",
                        updated_at=OLD_COMPLETED_AT,
                    ),
                ]
            )
            await session.commit()

        jellyfin = FakeJellyfin(
            (
                [
                    metadata(
                        "6f9619ff-8b86-d011-b42d-00cf4fc965ff",
                        title="Refreshed started",
                        poster_tag="started-tag",
                    ),
                    metadata(
                        "6f9619ff-8b86-d011-b42d-00cf4fc966ff",
                        title="Refreshed manual",
                        poster_tag="manual-tag",
                        imdb_id="tt9999999",
                    ),
                    metadata(
                        EPISODE_ID,
                        title="Refreshed episode",
                        kind="tv",
                        series_title="Refreshed series",
                        series_id="00000000-0000-0000-0000-000000000222",
                        series_poster_tag="new-series-tag",
                    ),
                ],
                3,
            )
        )
        service = SyncService(database, jellyfin)  # type: ignore[arg-type]

        assert await service.run() is True

        async with database.session() as session:
            started = await session.get(
                WatchedItem, "6f9619ff-8b86-d011-b42d-00cf4fc965ff"
            )
            manual = await session.get(
                WatchedItem, "6f9619ff-8b86-d011-b42d-00cf4fc966ff"
            )
            episode = await session.get(WatchedItem, EPISODE_ID)

        assert started is not None
        assert started.title == "Refreshed started"
        assert started.poster_tag == "started-tag"
        assert started.watch_status == "started"
        assert started.started_at == "2026-10-15T10:00:00.000000Z"
        assert started.watched_at == OLD_COMPLETED_AT
        assert started.play_count == 1
        assert started.manually_watched_at is None
        assert started.is_favorite is False

        assert manual is not None
        assert manual.title == "Refreshed manual"
        assert manual.poster_tag == "manual-tag"
        assert manual.imdb_id == "tt9999999"
        assert manual.watch_status == "watched"
        assert manual.watched_at is None
        assert manual.manually_watched_at == "2026-10-15T11:30:00.000000Z"
        assert manual.play_count == 0
        assert manual.is_favorite is True

        assert episode is not None
        assert episode.title == "Refreshed episode"
        assert episode.watch_status == "watched"
        assert episode.watched_at == OLD_COMPLETED_AT
        assert episode.play_count == 2
        assert episode.is_favorite is True
        assert episode.series_poster_tag == "new-series-tag"
    finally:
        await database.dispose()


@pytest.mark.asyncio
async def test_failed_sync_preserves_started_and_favorite_state(
    tmp_path: Path,
) -> None:
    database = await make_database(tmp_path)
    try:
        async with database.session() as session:
            session.add(
                WatchedItem(
                    jellyfin_id="6f9619ff-8b86-d011-b42d-00cf4fc967ff",
                    kind="movie",
                    title="Started film",
                    watch_status="started",
                    started_at="2026-10-15T10:00:00.000000Z",
                    watched_at=None,
                    play_count=0,
                    is_favorite=True,
                    poster_tag="tag",
                    updated_at="2026-10-15T10:00:00.000000Z",
                )
            )
            await session.commit()

        service = SyncService(
            database,
            FakeJellyfin(error=JellyfinTransportError("Jellyfin returned HTTP 503")),  # type: ignore[arg-type]
        )
        assert await service.run() is False

        async with database.session() as session:
            item = await session.get(
                WatchedItem, "6f9619ff-8b86-d011-b42d-00cf4fc967ff"
            )
            state = await session.get(SyncState, 1)

        assert item is not None
        assert item.watch_status == "started"
        assert item.started_at == "2026-10-15T10:00:00.000000Z"
        assert item.watched_at is None
        assert item.play_count == 0
        assert item.is_favorite is True
        assert item.poster_tag == "tag"
        assert state is not None
        assert state.last_error == "Jellyfin server error (HTTP 503) after retries"
    finally:
        await database.dispose()
