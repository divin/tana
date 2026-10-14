from __future__ import annotations

from pathlib import Path

import pytest
from sqlalchemy import text
from sqlalchemy.exc import IntegrityError
from sqlmodel import select

from tana.database import Database
from tana.models import CompletedPlayback, PosterCache, SyncState, WatchedItem


@pytest.mark.asyncio
async def test_initialize_creates_schema_pragmas_and_singleton(tmp_path: Path) -> None:
    data_dir = tmp_path / "data"
    database = Database(data_dir)
    await database.initialize()
    try:
        assert database.database_path.is_file()
        assert database.poster_path.is_dir()
        assert database.engine is not None

        async with database.engine.connect() as connection:
            version = (
                await connection.execute(text("PRAGMA user_version"))
            ).scalar_one()
            foreign_keys = (
                await connection.execute(text("PRAGMA foreign_keys"))
            ).scalar_one()
            journal_mode = (
                await connection.execute(text("PRAGMA journal_mode"))
            ).scalar_one()
            tables = (
                (
                    await connection.execute(
                        text("SELECT name FROM sqlite_master WHERE type='table'")
                    )
                )
                .scalars()
                .all()
            )
            indexes = (
                (await connection.execute(text("PRAGMA index_list('watched_items')")))
                .mappings()
                .all()
            )

        assert version == 3
        assert foreign_keys == 1
        assert journal_mode.lower() == "wal"
        assert {
            "watched_items",
            "completed_playbacks",
            "poster_cache",
            "sync_state",
        }.issubset(tables)
        assert {
            "ix_watched_items_watched_title",
            "ix_watched_items_kind_watched",
        }.issubset({index["name"] for index in indexes})

        async with database.session() as session:
            state = await session.get(SyncState, 1)
            assert state is not None
            assert state.status == "never"
            assert state.items_seen == 0
    finally:
        await database.dispose()


@pytest.mark.asyncio
async def test_models_persist_rows_and_enforce_constraints(tmp_path: Path) -> None:
    database = Database(tmp_path / "data")
    await database.initialize()
    try:
        async with database.session() as session:
            session.add(
                WatchedItem(
                    jellyfin_id="6f9619ff-8b86-d011-b42d-00cf4fc964ff",
                    kind="movie",
                    title="Example film",
                    watched_at="2025-01-02T03:04:05.000000Z",
                    play_count=2,
                    poster_tag="primary-tag",
                    imdb_id="tt1234567",
                    updated_at="2025-01-02T03:04:05.000000Z",
                )
            )
            await session.commit()

        async with database.session() as session:
            item = await session.get(
                WatchedItem, "6f9619ff-8b86-d011-b42d-00cf4fc964ff"
            )
            assert item is not None
            assert item.play_count == 2
            assert item.imdb_id == "tt1234567"

        invalid_items = [
            WatchedItem(
                jellyfin_id="invalid-kind",
                kind="music",
                title="Invalid",
                watched_at="2025-01-02T03:04:05.000000Z",
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
            WatchedItem(
                jellyfin_id="negative-play-count",
                kind="movie",
                title="Invalid",
                watched_at="2025-01-02T03:04:05.000000Z",
                play_count=-1,
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
            WatchedItem(
                jellyfin_id="invalid-tv-row",
                kind="tv",
                title="Episode",
                watched_at="2025-01-02T03:04:05.000000Z",
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
        ]
        for invalid_item in invalid_items:
            async with database.session() as session:
                session.add(invalid_item)
                with pytest.raises(IntegrityError):
                    await session.commit()
                await session.rollback()

        invalid_records = [
            PosterCache(
                source_id="6f9619ff-8b86-d011-b42d-00cf4fc964ff",
                source_type="movie",
                image_tag="tag",
                cached_at="2025-01-02T03:04:05.000000Z",
                size_bytes=0,
            ),
            PosterCache(
                source_id="missing-parent",
                source_type="episode",
                image_tag="tag",
                cached_at="2025-01-02T03:04:05.000000Z",
                size_bytes=10,
            ),
            CompletedPlayback(
                event_key="orphan-event",
                jellyfin_id="missing-parent",
                completed_at="2025-01-02T03:04:05.000000Z",
            ),
            SyncState(id=2),
        ]
        for invalid_record in invalid_records:
            async with database.session() as session:
                session.add(invalid_record)
                with pytest.raises(IntegrityError):
                    await session.commit()
                await session.rollback()

        async with database.session() as session:
            session.add(
                PosterCache(
                    source_id="series-without-item-row",
                    source_type="series",
                    image_tag="series-tag",
                    cached_at="2025-01-02T03:04:05.000000Z",
                    size_bytes=10,
                )
            )
            await session.commit()

        async with database.session() as session:
            assert len((await session.exec(select(WatchedItem))).all()) == 1
    finally:
        await database.dispose()


@pytest.mark.asyncio
async def test_legacy_schema_is_reset_to_fresh_webhook_tracking(tmp_path: Path) -> None:
    data_dir = tmp_path / "data"
    legacy = Database(data_dir)
    await legacy.initialize()
    async with legacy.session() as session:
        session.add(
            WatchedItem(
                jellyfin_id="6f9619ff-8b86-d011-b42d-00cf4fc964ff",
                kind="movie",
                title="Legacy row",
                watched_at="2025-01-02T03:04:05.000000Z",
                play_count=5,
                updated_at="2025-01-02T03:04:05.000000Z",
            )
        )
        await session.commit()
    assert legacy.engine is not None
    async with legacy.engine.begin() as connection:
        await connection.execute(text("PRAGMA user_version = 2"))
    await legacy.dispose()
    (data_dir / "posters" / "legacy.jpg").write_bytes(b"legacy poster")

    current = Database(data_dir)
    await current.initialize()
    try:
        async with current.session() as session:
            items = (await session.exec(select(WatchedItem))).all()
            events = (await session.exec(select(CompletedPlayback))).all()
            state = await session.get(SyncState, 1)
        async with current.engine.connect() as connection:
            version = (
                await connection.execute(text("PRAGMA user_version"))
            ).scalar_one()

        assert items == []
        assert events == []
        assert state is not None
        assert state.status == "never"
        assert version == 3
        assert list((data_dir / "posters").iterdir()) == []
    finally:
        await current.dispose()


@pytest.mark.asyncio
async def test_newer_schema_version_is_rejected(tmp_path: Path) -> None:
    data_dir = tmp_path / "data"
    first = Database(data_dir)
    await first.initialize()
    assert first.engine is not None
    async with first.engine.begin() as connection:
        await connection.execute(text("PRAGMA user_version = 4"))
    await first.dispose()

    second = Database(data_dir)
    with pytest.raises(RuntimeError, match="newer than supported"):
        await second.initialize()
    await second.dispose()
