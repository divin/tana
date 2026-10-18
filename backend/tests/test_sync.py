from __future__ import annotations

import asyncio
from pathlib import Path
from typing import Any

import pytest
from pydantic import SecretStr
from sqlmodel import select

from tana.config import Settings
from tana.database import Database
from tana.jellyfin import (
    ActivityEntry,
    JellyfinForbiddenError,
    JellyfinTransportError,
    LibraryItemData,
)
from tana.models import SyncState, WatchedItem, WatchEvent
from tana.sync import SyncService

USER_ID = "11111111-1111-4111-8111-111111111111"
MOVIE_ID = "6f9619ff-8b86-d011-b42d-00cf4fc964ff"
EPISODE_ID = "6f9619ff-8b86-d011-b42d-00cf4fc96400"
DELETED_ID = "6f9619ff-8b86-d011-b42d-00cf4fc964ee"
LAST_PLAYED = "2026-10-11T12:30:00.000000Z"
D1 = "2026-10-01T20:00:00.000000Z"
D2 = "2026-10-05T21:00:00.000000Z"
D3 = "2026-10-11T12:30:00.000000Z"


def item(
    item_id: str = MOVIE_ID,
    *,
    kind: str = "movie",
    title: str = "Example movie",
    play_count: int = 1,
    last_played: str | None = LAST_PLAYED,
    played: bool = True,
    percentage: float | None = None,
    position: int = 0,
    **metadata: Any,
) -> LibraryItemData:
    defaults: dict[str, Any] = (
        {} if kind == "movie" else {"series_title": "Example series"}
    )
    defaults.update(metadata)
    return LibraryItemData(
        jellyfin_id=item_id,
        kind=kind,
        title=title,
        played=played,
        play_count=play_count,
        last_played_date=last_played,
        played_percentage=percentage,
        position_ticks=position,
        **defaults,
    )


def entry(
    entry_id: int,
    kind: str,
    item_id: str,
    date: str,
    user_id: str = USER_ID,
) -> ActivityEntry:
    return ActivityEntry(
        entry_id=entry_id,
        kind=kind,
        item_id=item_id,
        occurred_at=date,
        user_id=user_id,
        title_hint=None,
    )


def make_settings(*, threshold: float = 0.9, activity_log: bool = True) -> Settings:
    return Settings(
        _env_file=None,
        JELLYFIN_URL="http://jellyfin.example:8096",
        JELLYFIN_API_KEY=SecretStr("test-key"),
        JELLYFIN_USER_ID=USER_ID,
        TZ="UTC",
        PLAYBACK_COMPLETION_THRESHOLD=threshold,
        JELLYFIN_ACTIVITY_LOG_ENABLED=activity_log,
    )


class FakeJellyfin:
    def __init__(
        self,
        *,
        played: list[LibraryItemData] | None = None,
        resume: list[LibraryItemData] | None = None,
        entries: list[ActivityEntry] | None = None,
        details: dict[str, LibraryItemData] | None = None,
        played_error: Exception | None = None,
        entries_error: Exception | None = None,
    ) -> None:
        self.played = list(played or [])
        self.resume = list(resume or [])
        self.entries = list(entries or [])
        self.details = details or {}
        self.played_error = played_error
        self.entries_error = entries_error
        self.played_calls = 0
        self.entry_calls = 0

    async def fetch_played_items(self) -> tuple[list[LibraryItemData], int]:
        self.played_calls += 1
        if self.played_error is not None:
            raise self.played_error
        return list(self.played), len(self.played)

    async def fetch_resume_items(self) -> list[LibraryItemData]:
        return list(self.resume)

    async def fetch_activity_window(
        self,
    ) -> tuple[list[ActivityEntry], int | None]:
        self.entry_calls += 1
        if self.entries_error is not None:
            raise self.entries_error
        max_id = max((e.entry_id for e in self.entries), default=None)
        return list(self.entries), max_id

    async def fetch_item_details(self, item_id: str) -> LibraryItemData | None:
        return self.details.get(item_id)


async def make_database(tmp_path: Path) -> Database:
    database = Database(tmp_path / "data")
    await database.initialize()
    return database


async def run_sync(
    database: Database,
    fake: FakeJellyfin,
    *,
    settings: Settings | None = None,
) -> bool:
    service = SyncService(database, fake, settings or make_settings())  # type: ignore[arg-type]
    return await service.run()


async def all_events(database: Database) -> list[WatchEvent]:
    async with database.session() as session:
        return list((await session.exec(select(WatchEvent))).all())


async def events_for(database: Database, item_id: str) -> list[WatchEvent]:
    events = await all_events(database)
    return [e for e in events if e.jellyfin_id == item_id]


async def get_item(database: Database, item_id: str) -> WatchedItem | None:
    async with database.session() as session:
        return await session.get(WatchedItem, item_id)


@pytest.mark.asyncio
async def test_first_run_backfills_aggregate_history(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    fake = FakeJellyfin(
        played=[
            item(MOVIE_ID, play_count=2, last_played=D3),
            item(EPISODE_ID, kind="tv", title="Episode", play_count=1, last_played=D1),
        ]
    )
    assert await run_sync(database, fake) is True

    movie = await get_item(database, MOVIE_ID)
    assert movie is not None
    assert movie.watch_status == "watched"
    assert movie.play_count == 2
    assert movie.watched_at == D3
    episode = await get_item(database, EPISODE_ID)
    assert episode is not None and episode.play_count == 1 and episode.watched_at == D1

    movie_events = await events_for(database, MOVIE_ID)
    assert len(movie_events) == 2
    assert all(e.source == "userdata" for e in movie_events)
    assert all(e.event_date == D3 for e in movie_events)

    # Re-run is idempotent: no additional events or count changes.
    assert await run_sync(database, fake) is True
    assert len(await events_for(database, MOVIE_ID)) == 2
    assert (await get_item(database, MOVIE_ID)).play_count == 2
    assert (await get_item(database, MOVIE_ID)).watched_at == D3
    await database.dispose()


@pytest.mark.asyncio
async def test_activity_log_stops_become_exact_events(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    fake = FakeJellyfin(
        played=[item(MOVIE_ID, play_count=3, last_played=D3)],
        entries=[
            entry(100, "start", MOVIE_ID, "2026-10-01T19:30:00.000000Z"),
            entry(101, "stop", MOVIE_ID, D1),
            entry(102, "start", MOVIE_ID, "2026-10-05T20:30:00.000000Z"),
            entry(103, "stop", MOVIE_ID, D2),
            entry(104, "start", MOVIE_ID, "2026-10-11T12:00:00.000000Z"),
            entry(105, "stop", MOVIE_ID, D3),
        ],
    )
    assert await run_sync(database, fake) is True

    events = await events_for(database, MOVIE_ID)
    exact = [e for e in events if e.source == "activity_log"]
    aggregate = [e for e in events if e.source == "userdata"]
    assert len(exact) == 3
    assert len(aggregate) == 0
    assert sorted(e.event_date for e in exact) == [D1, D2, D3]

    movie = await get_item(database, MOVIE_ID)
    assert movie.play_count == 3
    assert movie.watched_at == D3

    # Cursor records the highest scanned id.
    async with database.session() as session:
        state = await session.get(SyncState, 1)
        assert state.activity_cursor == 105

    # A second run observes nothing new and changes nothing.
    assert await run_sync(database, fake) is True
    assert len(await events_for(database, MOVIE_ID)) == 3
    await database.dispose()


@pytest.mark.asyncio
async def test_exact_events_fill_residual_aggregates(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    fake = FakeJellyfin(
        played=[item(MOVIE_ID, play_count=3, last_played=D3)],
        entries=[entry(200, "stop", MOVIE_ID, D2)],
    )
    assert await run_sync(database, fake) is True

    events = await events_for(database, MOVIE_ID)
    exact = [e for e in events if e.source == "activity_log"]
    aggregate = [e for e in events if e.source == "userdata"]
    assert len(exact) == 1 and exact[0].event_date == D2
    # The remaining two plays are the aggregate approximation dated at the
    # newest known completion; total events equal the server play count.
    assert len(aggregate) == 2
    assert sum(1 for _ in events) == 3
    assert (await get_item(database, MOVIE_ID)).play_count == 3
    await database.dispose()


@pytest.mark.asyncio
async def test_more_stops_than_play_count_are_capped(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    fake = FakeJellyfin(
        played=[item(MOVIE_ID, play_count=1, last_played=D3)],
        entries=[
            entry(300, "stop", MOVIE_ID, D1),
            entry(301, "stop", MOVIE_ID, D2),
        ],
    )
    assert await run_sync(database, fake) is True

    events = await events_for(database, MOVIE_ID)
    assert len(events) == 1
    assert events[0].source == "activity_log"
    # The most recent stop is kept.
    assert events[0].event_date == D2
    movie = await get_item(database, MOVIE_ID)
    assert movie.play_count == 1
    await database.dispose()


@pytest.mark.asyncio
async def test_started_movie_from_resume_and_start_entry(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    fake = FakeJellyfin(
        resume=[
            item(
                MOVIE_ID,
                play_count=1,
                last_played=D1,
                played=False,
                percentage=0.3,
                position=12345,
            )
        ],
        entries=[entry(400, "start", MOVIE_ID, "2026-10-11T10:00:00.000000Z")],
    )
    assert await run_sync(database, fake) is True

    movie = await get_item(database, MOVIE_ID)
    assert movie is not None
    assert movie.watch_status == "started"
    assert movie.started_at == "2026-10-11T10:00:00.000000Z"
    # Played=false means no verified completion: the partial watch counts zero
    # and creates no event.
    assert movie.play_count == 0
    assert movie.watched_at is None
    assert not await events_for(database, MOVIE_ID)
    await database.dispose()


@pytest.mark.asyncio
async def test_rewatch_in_progress_keeps_prior_completions(tmp_path: Path) -> None:
    """Jellyfin keeps Played=true while re-watching a completed item, so a
    rewatch in progress retains its prior verified history."""
    database = await make_database(tmp_path)
    fake = FakeJellyfin(
        resume=[
            item(
                MOVIE_ID,
                play_count=3,
                last_played=D3,
                played=True,
                percentage=0.35,
                position=9999,
            )
        ],
        entries=[entry(410, "start", MOVIE_ID, "2026-10-11T10:00:00.000000Z")],
    )
    assert await run_sync(database, fake) is True

    movie = await get_item(database, MOVIE_ID)
    assert movie is not None
    assert movie.watch_status == "started"
    assert movie.play_count == 3
    assert movie.watched_at == D3
    events = await events_for(database, MOVIE_ID)
    assert len(events) == 3
    assert all(e.source == "userdata" for e in events)
    await database.dispose()


@pytest.mark.asyncio
async def test_unplayed_item_history_is_removed(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    played = item(MOVIE_ID, play_count=1, last_played=D1)
    assert await run_sync(database, FakeJellyfin(played=[played])) is True
    assert (await get_item(database, MOVIE_ID)) is not None
    assert len(await events_for(database, MOVIE_ID)) == 1

    # Server side marks the movie unplayed: Played=false, PlayCount=0.
    unplayed = item(MOVIE_ID, play_count=0, last_played=None, played=False)
    assert await run_sync(database, FakeJellyfin(played=[unplayed])) is True

    assert (await get_item(database, MOVIE_ID)) is None
    assert len(await events_for(database, MOVIE_ID)) == 0
    await database.dispose()


@pytest.mark.asyncio
async def test_started_but_never_completed_item_counts_zero(tmp_path: Path) -> None:
    """Jellyfin increments PlayCount on playback start; without the Played flag
    (or a PlayedPercentage past the threshold) that is not a completion."""
    database = await make_database(tmp_path)
    fake = FakeJellyfin(
        played=[
            item(
                MOVIE_ID,
                title="Alone at last",
                played=False,
                play_count=1,
                last_played="2026-10-04T11:01:37.000000Z",
            )
        ],
        entries=[
            entry(1480, "start", MOVIE_ID, "2026-10-04T11:01:37.000000Z"),
            entry(1481, "stop", MOVIE_ID, "2026-10-04T11:01:43.000000Z"),
        ],
    )
    assert await run_sync(database, fake) is True

    assert (await get_item(database, MOVIE_ID)) is None
    assert not await events_for(database, MOVIE_ID)
    await database.dispose()


@pytest.mark.asyncio
async def test_false_watch_row_is_self_healed(tmp_path: Path) -> None:
    """A row created under the old PlayCount-is-a-completion bug is removed
    once the server reports no completion evidence."""
    database = await make_database(tmp_path)
    # First pass previously produced a false positive from PlayCount alone…
    async with database.session() as session, session.begin():
        session.add(
            WatchedItem(
                jellyfin_id=MOVIE_ID,
                kind="tv",
                title="Alone at last",
                series_title="Example series",
                watch_status="watched",
                watched_at="2026-10-04T11:01:37.000000Z",
                play_count=1,
                is_favorite=False,
                series_poster_tag="poster",
                updated_at="2026-10-04T11:01:37.000000Z",
            )
        )
        await session.flush()
        session.add(
            WatchEvent(
                event_key="stale-aggregate",
                jellyfin_id=MOVIE_ID,
                source="userdata",
                event_date="2026-10-04T11:01:37.000000Z",
                log_entry_id=None,
            )
        )
    assert (await get_item(database, MOVIE_ID)) is not None

    # …and the server now reports Played=false with only PlayCount raised by starts.
    fake = FakeJellyfin(
        played=[
            item(MOVIE_ID, kind="tv", title="Alone at last", played=False, play_count=1)
        ]
    )
    assert await run_sync(database, fake) is True

    assert (await get_item(database, MOVIE_ID)) is None
    assert not await events_for(database, MOVIE_ID)
    await database.dispose()


@pytest.mark.asyncio
async def test_metadata_refresh_preserves_counts(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    fake = FakeJellyfin(played=[item(MOVIE_ID, title="Old title")])
    await run_sync(database, fake)
    fake.played[0] = item(MOVIE_ID, title="New title")
    await run_sync(database, fake)

    movie = await get_item(database, MOVIE_ID)
    assert movie.title == "New title"
    assert movie.play_count == 1
    assert movie.watched_at == LAST_PLAYED
    await database.dispose()


@pytest.mark.asyncio
async def test_deleted_item_without_row_is_skipped(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    fake = FakeJellyfin(
        entries=[entry(500, "stop", DELETED_ID, D1)],
        details={},
    )
    assert await run_sync(database, fake) is True
    assert (await get_item(database, DELETED_ID)) is None
    assert not await events_for(database, DELETED_ID)
    await database.dispose()


@pytest.mark.asyncio
async def test_deleted_item_with_existing_row_keeps_exact_history(
    tmp_path: Path,
) -> None:
    database = await make_database(tmp_path)
    await run_sync(database, FakeJellyfin(played=[item(MOVIE_ID)]))
    assert (await get_item(database, MOVIE_ID)) is not None

    # The item disappears from the library but a stop is still in the log; the
    # movie resolves to nothing (deleted), so its stop becomes exact history.
    fake = FakeJellyfin(
        entries=[entry(600, "stop", MOVIE_ID, D2)],
        details={},
    )
    assert await run_sync(database, fake) is True

    movie = await get_item(database, MOVIE_ID)
    assert movie is not None
    events = await events_for(database, MOVIE_ID)
    assert len(events) == 2
    assert any(e.source == "activity_log" and e.event_date == D2 for e in events)
    # The retained aggregate (newer) and the exact stop both remain; watched_at
    # is the newest event.
    assert movie.watched_at == LAST_PLAYED
    await database.dispose()


@pytest.mark.asyncio
async def test_deleted_item_resolved_via_details_uses_current_state(
    tmp_path: Path,
) -> None:
    database = await make_database(tmp_path)
    # Item is gone from the played list but resolves (played in Jellyfin), so
    # its stop is gated by the current UserData and becomes exact history.
    resolved = item(MOVIE_ID, play_count=1, last_played=D2)
    fake = FakeJellyfin(
        entries=[entry(700, "stop", MOVIE_ID, D2)],
        details={MOVIE_ID: resolved},
    )
    assert await run_sync(database, fake) is True
    movie = await get_item(database, MOVIE_ID)
    assert movie is not None
    events = await events_for(database, MOVIE_ID)
    assert len(events) == 1
    assert events[0].source == "activity_log"
    await database.dispose()


@pytest.mark.asyncio
async def test_forbidden_activity_log_falls_back_to_tier_one(
    tmp_path: Path,
) -> None:
    database = await make_database(tmp_path)
    fake = FakeJellyfin(
        played=[item(MOVIE_ID)],
        entries_error=JellyfinForbiddenError(
            "Jellyfin returned HTTP 403; check API key permissions"
        ),
    )
    assert await run_sync(database, fake) is True

    movie = await get_item(database, MOVIE_ID)
    assert movie is not None and movie.play_count == 1
    async with database.session() as session:
        state = await session.get(SyncState, 1)
        assert state.status == "success"
        assert state.activity_log_enabled is False
        assert state.activity_cursor is None
    await database.dispose()


@pytest.mark.asyncio
async def test_disabled_activity_log_skips_tier_two(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    fake = FakeJellyfin(played=[item(MOVIE_ID)])
    settings = make_settings(activity_log=False)
    assert await run_sync(database, fake, settings=settings) is True
    assert fake.entry_calls == 0
    async with database.session() as session:
        state = await session.get(SyncState, 1)
        assert state.activity_log_enabled is False
    await database.dispose()


@pytest.mark.asyncio
async def test_transport_failure_marks_sync_failed_sanitized(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    fake = FakeJellyfin(
        played_error=JellyfinTransportError("Jellyfin returned HTTP 500")
    )
    service = SyncService(database, fake, make_settings())  # type: ignore[arg-type]
    assert await service.run() is False

    async with database.session() as session:
        state = await session.get(SyncState, 1)
        assert state.status == "failed"
        assert state.last_error == "Jellyfin server error (HTTP 500) after retries"
    assert (await get_item(database, MOVIE_ID)) is None
    await database.dispose()


@pytest.mark.asyncio
async def test_favorite_and_manual_state_survive_sync(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    await run_sync(database, FakeJellyfin(played=[item(MOVIE_ID)]))

    async with database.session() as session, session.begin():
        movie = await session.get(WatchedItem, MOVIE_ID)
        assert movie is not None
        movie.is_favorite = True
        movie.manually_watched_at = "2026-10-12T00:00:00.000000Z"

    # The movie is manually marked; a following sync must not clear it.
    await run_sync(
        database,
        FakeJellyfin(played=[item(MOVIE_ID, play_count=1, last_played=LAST_PLAYED)]),
    )
    movie = await get_item(database, MOVIE_ID)
    assert movie.is_favorite is True
    assert movie.manually_watched_at == "2026-10-12T00:00:00.000000Z"
    assert movie.watch_status == "watched"
    await database.dispose()


@pytest.mark.asyncio
async def test_episode_tracking(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    fake = FakeJellyfin(
        played=[
            item(
                EPISODE_ID,
                kind="tv",
                title="Episode 3",
                series_title="Series",
                series_id="22222222-2222-4222-8222-222222222222",
                season_number=2,
                episode_number=3,
                play_count=1,
                last_played=D1,
            )
        ]
    )
    assert await run_sync(database, fake) is True

    episode = await get_item(database, EPISODE_ID)
    assert episode is not None
    assert episode.kind == "tv"
    assert episode.watch_status == "watched"
    assert episode.play_count == 1
    assert episode.series_title == "Series"
    await database.dispose()


@pytest.mark.asyncio
async def test_safe_error_categories() -> None:
    cases = {
        JellyfinTransportError("Jellyfin returned HTTP 401"): (
            "Jellyfin returned HTTP 401; check the API key and user access"
        ),
        JellyfinTransportError("Jellyfin returned HTTP 404"): (
            "Jellyfin returned HTTP 404; check the configured server URL"
        ),
        JellyfinTransportError("Jellyfin returned HTTP 429"): (
            "Jellyfin rate limited the sync (HTTP 429)"
        ),
        JellyfinTransportError("Jellyfin could not be reached after 3 attempts"): (
            "Jellyfin could not be reached after retries; check the server URL "
            "and network availability"
        ),
    }
    for error, expected in cases.items():
        assert SyncService._safe_error(error) == expected


@pytest.mark.asyncio
async def test_cancelled_sync_fails_cleanly(tmp_path: Path) -> None:
    database = await make_database(tmp_path)

    class BlockingFake(FakeJellyfin):
        async def fetch_played_items(self) -> tuple[list[LibraryItemData], int]:
            await asyncio.sleep(60)
            return [], 0

    service = SyncService(database, BlockingFake(), make_settings())  # type: ignore[arg-type]
    task = asyncio.create_task(service.run())
    await asyncio.sleep(0.05)
    task.cancel()
    with pytest.raises(asyncio.CancelledError):
        await task

    async with database.session() as session:
        state = await session.get(SyncState, 1)
        assert state.status == "failed"
        assert state.last_error == "Sync interrupted"
    await database.dispose()


@pytest.mark.asyncio
async def test_short_reopen_is_not_used_as_exact_watch_date(tmp_path: Path) -> None:
    """A full session's stop is the exact event; a brief re-open is not."""
    database = await make_database(tmp_path)
    runtime_ticks = 60 * 60 * 10_000_000  # 60 minutes
    completion_stop = "2026-10-10T11:00:00.000000Z"
    fake = FakeJellyfin(
        played=[
            item(
                MOVIE_ID,
                play_count=2,
                last_played=completion_stop,
                runtime_ticks=runtime_ticks,
            )
        ],
        entries=[
            entry(900, "start", MOVIE_ID, "2026-10-10T10:00:00.000000Z"),
            entry(901, "stop", MOVIE_ID, completion_stop),  # 60 min -> completed
            entry(902, "start", MOVIE_ID, "2026-10-12T09:00:00.000000Z"),
            entry(903, "stop", MOVIE_ID, "2026-10-12T09:00:10.000000Z"),  # 10s re-open
        ],
    )
    assert await run_sync(database, fake) is True

    events = await events_for(database, MOVIE_ID)
    exact = [e for e in events if e.source == "activity_log"]
    assert len(exact) == 1
    assert exact[0].log_entry_id == 901
    assert exact[0].event_date == completion_stop
    movie = await get_item(database, MOVIE_ID)
    # PlayCount=2 counted the re-open as a start; the observable partial session
    # is subtracted so only the real completion remains.
    assert movie.play_count == 1
    assert movie.watched_at == completion_stop
    await database.dispose()


@pytest.mark.asyncio
async def test_restart_fragments_merge_into_one_completion_span(
    tmp_path: Path,
) -> None:
    """Stream restarts split one watch into several log entries; the merged
    span is still recognized as a completed session."""
    database = await make_database(tmp_path)
    runtime_ticks = 60 * 60 * 10_000_000
    fake = FakeJellyfin(
        played=[
            item(
                MOVIE_ID,
                play_count=1,
                last_played="2026-10-11T11:00:00.000000Z",
                runtime_ticks=runtime_ticks,
            )
        ],
        entries=[
            entry(910, "start", MOVIE_ID, "2026-10-11T10:00:00.000000Z"),
            entry(911, "stop", MOVIE_ID, "2026-10-11T10:05:00.000000Z"),  # restart
            entry(912, "start", MOVIE_ID, "2026-10-11T10:06:00.000000Z"),
            entry(913, "stop", MOVIE_ID, "2026-10-11T11:00:00.000000Z"),  # 55min more
        ],
    )
    assert await run_sync(database, fake) is True

    exact = [
        e for e in await events_for(database, MOVIE_ID) if e.source == "activity_log"
    ]
    # Merged span spans 60 minutes, so both fragment stops classify as complete.
    assert len(exact) == 1
    assert exact[0].log_entry_id == 913
    await database.dispose()


def test_completion_like_stop_ids_classification() -> None:
    from tana.sync import _completion_like_stop_ids

    runtime_ticks = 60 * 60 * 10_000_000  # 60 minutes

    def complete(entries: list[ActivityEntry]) -> set[int]:
        return _completion_like_stop_ids(entries, runtime_ticks, 0.9)[0]

    def partial(entries: list[ActivityEntry]) -> set[int]:
        return _completion_like_stop_ids(entries, runtime_ticks, 0.9)[1]

    # A full session completes; a 10-second re-open is clearly partial.
    full = [
        entry(1, "start", MOVIE_ID, "2026-10-01T10:00:00.000000Z"),
        entry(2, "stop", MOVIE_ID, "2026-10-01T11:00:00.000000Z"),
    ]
    assert complete(full) == {2}
    assert partial(full) == set()
    reopen = [
        entry(3, "start", MOVIE_ID, "2026-10-02T10:00:00.000000Z"),
        entry(4, "stop", MOVIE_ID, "2026-10-02T10:00:10.000000Z"),
    ]
    assert complete(reopen) == set()
    assert partial(reopen) == {4}

    # Two adjacent fragments (stream restart) merge into one completing span.
    fragments = [
        entry(5, "start", MOVIE_ID, "2026-10-03T10:00:00.000000Z"),
        entry(6, "stop", MOVIE_ID, "2026-10-03T10:05:00.000000Z"),
        entry(7, "start", MOVIE_ID, "2026-10-03T10:06:00.000000Z"),
        entry(8, "stop", MOVIE_ID, "2026-10-03T11:00:00.000000Z"),
    ]
    assert complete(fragments) == {6, 8}
    assert partial(fragments) == set()

    # A gap larger than the merge distance splits two partial sessions.
    split = [
        entry(9, "start", MOVIE_ID, "2026-10-04T10:00:00.000000Z"),
        entry(10, "stop", MOVIE_ID, "2026-10-04T10:40:00.000000Z"),
        entry(11, "start", MOVIE_ID, "2026-10-05T10:00:00.000000Z"),
        entry(12, "stop", MOVIE_ID, "2026-10-05T10:40:00.000000Z"),
    ]
    assert complete(split) == set()
    assert partial(split) == {10, 12}

    # Unknown runtime keeps the permissive behavior (no partials reported).
    allowed, reported = _completion_like_stop_ids(split, None, 0.9)
    assert allowed == {10, 12}
    assert reported == set()

    # A stop without a start in the window is neither complete nor partial.
    orphan = [entry(13, "stop", MOVIE_ID, "2026-10-06T10:00:00.000000Z")]
    assert complete(orphan) == set()
    assert partial(orphan) == set()


@pytest.mark.asyncio
async def test_unplayed_item_with_recent_stop_is_removed(tmp_path: Path) -> None:
    """Marking an item unplayed in Jellyfin removes its row even when an older
    partial session still exists in the activity window."""
    database = await make_database(tmp_path)
    # First run: the movie is completed (Played=true), one aggregate event.
    watched = item(MOVIE_ID, play_count=1, last_played=D1)
    assert await run_sync(database, FakeJellyfin(played=[watched])) is True
    assert (await get_item(database, MOVIE_ID)) is not None

    # The user marks it unplayed: dropped from IsPlayed, Played=false. The old
    # 6-second session is still in the log and the item still resolves.
    unplayed = item(MOVIE_ID, play_count=0, last_played=None, played=False)
    fake = FakeJellyfin(
        played=[],
        entries=[
            entry(800, "start", MOVIE_ID, "2026-10-04T11:01:37.000000Z"),
            entry(801, "stop", MOVIE_ID, "2026-10-04T11:01:43.000000Z"),
        ],
        details={MOVIE_ID: unplayed},
    )
    assert await run_sync(database, fake) is True

    assert (await get_item(database, MOVIE_ID)) is None
    assert not await events_for(database, MOVIE_ID)
    await database.dispose()


@pytest.mark.asyncio
async def test_idle_unplayed_item_without_recent_session_is_removed(
    tmp_path: Path,
) -> None:
    """A tracked item whose last playback predates the activity window still
    heals when Jellyfin no longer marks it played."""
    database = await make_database(tmp_path)
    watched = item(MOVIE_ID, play_count=1, last_played=D1)
    assert await run_sync(database, FakeJellyfin(played=[watched])) is True
    assert (await get_item(database, MOVIE_ID)) is not None

    unplayed = item(MOVIE_ID, play_count=0, last_played=None, played=False)
    fake = FakeJellyfin(played=[], entries=[], details={MOVIE_ID: unplayed})
    assert await run_sync(database, fake) is True

    assert (await get_item(database, MOVIE_ID)) is None
    assert not await events_for(database, MOVIE_ID)
    await database.dispose()


@pytest.mark.asyncio
async def test_deleted_item_without_recent_session_is_kept(tmp_path: Path) -> None:
    """A deleted-but-tracked item keeps its approximate history (no way to tell
    'deleted' from 'unplayed' when the item cannot be resolved)."""
    database = await make_database(tmp_path)
    watched = item(MOVIE_ID, play_count=1, last_played=D1)
    assert await run_sync(database, FakeJellyfin(played=[watched])) is True

    fake = FakeJellyfin(played=[], entries=[], details={})
    assert await run_sync(database, fake) is True

    assert (await get_item(database, MOVIE_ID)) is not None
    assert len(await events_for(database, MOVIE_ID)) == 1
    await database.dispose()


@pytest.mark.asyncio
async def test_small_played_percentage_is_not_a_completion(tmp_path: Path) -> None:
    """PlayedPercentage is a 0-100 Jellyfin percentage; a 1% partial watch must
    never satisfy the 0.9 fraction threshold."""
    database = await make_database(tmp_path)
    fake = FakeJellyfin(
        resume=[
            item(
                MOVIE_ID,
                play_count=1,
                last_played=D1,
                played=False,
                percentage=0.01,
                position=123,
            )
        ],
        entries=[entry(500, "start", MOVIE_ID, "2026-10-11T10:00:00.000000Z")],
    )
    assert await run_sync(database, fake) is True

    movie = await get_item(database, MOVIE_ID)
    assert movie is not None
    assert movie.watch_status == "started"
    assert movie.play_count == 0
    assert movie.watched_at is None
    # No completion event may be derived from a 1% start.
    assert not await events_for(database, MOVIE_ID)
    await database.dispose()


@pytest.mark.asyncio
async def test_four_brief_starts_but_one_completion_counts_once(
    tmp_path: Path,
) -> None:
    """The Island case: Played=true with PlayCount=4 from four 20-second device
    starts; only the real completion may count."""
    database = await make_database(tmp_path)
    runtime_ticks = 136 * 60 * 10_000_000  # 136 minutes
    played = item(
        MOVIE_ID,
        title="The Island",
        play_count=4,
        last_played="2026-10-17T13:23:17.000000Z",
        runtime_ticks=runtime_ticks,
    )
    entries = []
    for i, hour in enumerate(("13:20:42", "13:21:13", "13:22:13", "13:23:02")):
        entries.append(entry(900 + i * 2, "start", MOVIE_ID, f"2026-10-17T{hour}Z"))
        entries.append(
            entry(901 + i * 2, "stop", MOVIE_ID, f"2026-10-17T{hour[:-2]}5{i + 1}Z")
        )
    assert (
        await run_sync(database, FakeJellyfin(played=[played], entries=entries)) is True
    )

    movie = await get_item(database, MOVIE_ID)
    assert movie is not None
    assert movie.play_count == 1
    events = await events_for(database, MOVIE_ID)
    assert len(events) == 1
    # No exact event may be derived from any of the four brief sessions.
    assert events[0].source == "userdata"
    assert events[0].event_date == "2026-10-17T13:23:17.000000Z"
    await database.dispose()


@pytest.mark.asyncio
async def test_stale_exact_events_from_brief_starts_are_pruned(
    tmp_path: Path,
) -> None:
    """Previously recorded exact events from brief re-opens are removed on the
    next sync so the corrected count and dates apply."""
    database = await make_database(tmp_path)
    runtime_ticks = 136 * 60 * 10_000_000
    # Simulate the old, PlayCount-trusting behavior: one bogus exact event.
    async with database.session() as session, session.begin():
        session.add(
            WatchedItem(
                jellyfin_id=MOVIE_ID,
                kind="movie",
                title="The Island",
                watch_status="watched",
                watched_at="2026-10-17T13:23:17.000000Z",
                play_count=1,
                is_favorite=False,
                poster_tag="poster",
                updated_at="2026-10-17T13:23:17.000000Z",
            )
        )
        await session.flush()
        session.add(
            WatchEvent(
                event_key="stale-exact",
                jellyfin_id=MOVIE_ID,
                source="activity_log",
                event_date="2026-10-17T13:22:29.000000Z",
                log_entry_id=932,
            )
        )

    played = item(
        MOVIE_ID,
        title="The Island",
        play_count=2,
        last_played="2026-10-17T13:23:17.000000Z",
        runtime_ticks=runtime_ticks,
    )
    fake = FakeJellyfin(
        played=[played],
        entries=[
            entry(930, "start", MOVIE_ID, "2026-10-17T13:22:13.000000Z"),
            entry(931, "stop", MOVIE_ID, "2026-10-17T13:22:29.000000Z"),
            entry(932, "start", MOVIE_ID, "2026-10-17T13:23:02.000000Z"),
            entry(933, "stop", MOVIE_ID, "2026-10-17T13:23:17.000000Z"),
        ],
    )
    assert await run_sync(database, fake) is True

    movie = await get_item(database, MOVIE_ID)
    assert movie is not None
    # Two brief starts -> one real completion: the stale exact event is dropped.
    assert movie.play_count == 1
    events = await events_for(database, MOVIE_ID)
    assert len(events) == 1
    assert events[0].source == "userdata"
    await database.dispose()
