from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest
from sqlalchemy import text
from sqlalchemy.exc import IntegrityError
from sqlmodel import select

from tana.database import SCHEMA_VERSION, Database
from tana.models import SyncState, UserDataSnapshot, WatchedItem, WatchEvent


async def make_database(tmp_path: Path) -> Database:
    database = Database(tmp_path / "data")
    await database.initialize()
    return database


def tables_on_disk(db_path: Path) -> set[str]:
    with sqlite3.connect(db_path) as connection:
        rows = connection.execute(
            "SELECT name FROM sqlite_master WHERE type='table'"
        ).fetchall()
    return {row[0] for row in rows}


def user_version(db_path: Path) -> int:
    with sqlite3.connect(db_path) as connection:
        return int(connection.execute("PRAGMA user_version").fetchone()[0])


async def assert_v5_database(database: Database) -> None:
    async with database.session() as session:
        tables = {
            row[0]
            for row in (
                await session.exec(
                    text("SELECT name FROM sqlite_master WHERE type='table'")
                )
            ).all()
        }
        assert {
            "watched_items",
            "watch_events",
            "userdata_snapshots",
            "poster_cache",
            "sync_state",
        } <= tables
        assert "completed_playbacks" not in tables
        state = await session.get(SyncState, 1)
        assert state is not None
        assert state.status == "never"
        assert state.activity_cursor is None
        assert state.activity_log_enabled is True


def write_legacy_database(path: Path, version: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(path) as connection:
        connection.execute(
            "CREATE TABLE watched_items (jellyfin_id TEXT PRIMARY KEY, title TEXT)"
        )
        connection.execute(
            "INSERT INTO watched_items (jellyfin_id, title) VALUES ('legacy', 'old')"
        )
        connection.execute(f"PRAGMA user_version = {version}")


@pytest.mark.asyncio
async def test_fresh_database_creates_v5_schema(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    await assert_v5_database(database)
    assert user_version(database.database_path) == SCHEMA_VERSION
    await database.dispose()


@pytest.mark.asyncio
async def test_every_legacy_and_unknown_version_is_recreated(tmp_path: Path) -> None:
    for version in (1, 2, 3, 4, 99):
        data_dir = tmp_path / f"legacy-{version}"
        write_legacy_database(data_dir / "tana.sqlite3", version)
        (data_dir / "posters").mkdir(parents=True, exist_ok=True)
        (data_dir / "posters" / "stale.jpg").write_bytes(b"legacy poster bytes")

        database = Database(data_dir)
        await database.initialize()

        assert tables_on_disk(database.database_path) != {"watched_items"}
        assert not (data_dir / "posters" / "stale.jpg").exists()
        await assert_v5_database(database)
        await database.dispose()


@pytest.mark.asyncio
async def test_recreation_discards_legacy_rows_and_poster_files(
    tmp_path: Path,
) -> None:
    data_dir = tmp_path / "legacy"
    write_legacy_database(data_dir / "tana.sqlite3", 4)
    (data_dir / "posters").mkdir(parents=True)
    (data_dir / "posters" / "stale.jpg").write_bytes(b"stale")

    database = Database(data_dir)
    await database.initialize()
    async with database.session() as session:
        rows = (await session.exec(select(WatchedItem))).all()
        assert rows == []
    assert not (data_dir / "posters" / "stale.jpg").exists()
    await database.dispose()


@pytest.mark.asyncio
async def test_v5_database_opens_and_preserves_existing_data(tmp_path: Path) -> None:
    data_dir = tmp_path / "stable"
    database = Database(data_dir)
    await database.initialize()
    # Seed a v5 database with a row and poster file, then reopen it.
    poster_path = database.poster_path
    poster_path.mkdir(parents=True, exist_ok=True)
    (poster_path / "keep.jpg").write_bytes(b"keep me")
    async with database.session() as session, session.begin():
        session.add(
            WatchedItem(
                jellyfin_id="11111111-1111-4111-8111-111111111111",
                kind="movie",
                title="Kept movie",
                watch_status="watched",
                watched_at="2026-10-01T00:00:00.000000Z",
                play_count=1,
                is_favorite=False,
                updated_at="2026-10-01T00:00:00.000000Z",
            )
        )
        await session.flush()
        session.add(
            WatchEvent(
                event_key="event-1",
                jellyfin_id="11111111-1111-4111-8111-111111111111",
                source="activity_log",
                event_date="2026-10-01T00:00:00.000000Z",
                log_entry_id=42,
            )
        )
    await database.dispose()

    reopened = Database(data_dir)
    await reopened.initialize()
    async with reopened.session() as session:
        item = await session.get(WatchedItem, "11111111-1111-4111-8111-111111111111")
        assert item is not None
        assert item.title == "Kept movie"
        events = (await session.exec(select(WatchEvent))).all()
        assert len(events) == 1
        assert events[0].log_entry_id == 42
    assert (poster_path / "keep.jpg").exists()
    assert user_version(reopened.database_path) == SCHEMA_VERSION
    await reopened.dispose()


@pytest.mark.asyncio
async def test_watch_event_references_existing_watched_row(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    async with database.session() as session, session.begin():
        session.add(
            WatchEvent(
                event_key="orphan",
                jellyfin_id="99999999-9999-4999-8999-999999999999",
                source="activity_log",
                event_date="2026-10-01T00:00:00.000000Z",
                log_entry_id=1,
            )
        )
        with pytest.raises(IntegrityError):
            await session.commit()
    await database.dispose()


@pytest.mark.asyncio
async def test_watch_event_source_constraints(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    async with database.session() as session, session.begin():
        session.add(
            WatchedItem(
                jellyfin_id="11111111-1111-4111-8111-111111111111",
                kind="movie",
                title="Movie",
                watch_status="watched",
                watched_at="2026-10-01T00:00:00.000000Z",
                play_count=1,
                is_favorite=False,
                updated_at="2026-10-01T00:00:00.000000Z",
            )
        )
    item_id = "11111111-1111-4111-8111-111111111111"

    # A userdata event must not carry a log entry id.
    async with database.session() as session, session.begin():
        session.add(
            WatchEvent(
                event_key="bad-userdata",
                jellyfin_id=item_id,
                source="userdata",
                event_date="2026-10-01T00:00:00.000000Z",
                log_entry_id=7,
            )
        )
        with pytest.raises(IntegrityError):
            await session.commit()

    # An activity event must carry a log entry id.
    async with database.session() as session, session.begin():
        session.add(
            WatchEvent(
                event_key="bad-activity",
                jellyfin_id=item_id,
                source="activity_log",
                event_date="2026-10-01T00:00:00.000000Z",
                log_entry_id=None,
            )
        )
        with pytest.raises(IntegrityError):
            await session.commit()

    # Unknown sources are rejected.
    async with database.session() as session, session.begin():
        session.add(
            WatchEvent(
                event_key="bad-source",
                jellyfin_id=item_id,
                source="webhook",
                event_date="2026-10-01T00:00:00.000000Z",
                log_entry_id=None,
            )
        )
        with pytest.raises(IntegrityError):
            await session.commit()
    await database.dispose()


@pytest.mark.asyncio
async def test_snapshot_constraints(tmp_path: Path) -> None:
    database = await make_database(tmp_path)
    async with database.session() as session, session.begin():
        session.add(
            UserDataSnapshot(
                jellyfin_id="11111111-1111-4111-8111-111111111111",
                play_count=-1,
                played=False,
                synced_at="2026-10-01T00:00:00.000000Z",
            )
        )
        with pytest.raises(IntegrityError):
            await session.commit()
    await database.dispose()
