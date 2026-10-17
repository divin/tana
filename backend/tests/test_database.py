from __future__ import annotations

from pathlib import Path

import aiosqlite
import pytest
from sqlalchemy import text
from sqlalchemy.exc import IntegrityError
from sqlmodel import select

from tana.database import Database
from tana.models import CompletedPlayback, PosterCache, SyncState, WatchedItem

V3_STATEMENTS = [
    """
CREATE TABLE watched_items (
    jellyfin_id VARCHAR NOT NULL,
    kind VARCHAR NOT NULL,
    title VARCHAR NOT NULL,
    series_title VARCHAR,
    series_id VARCHAR,
    season_number INTEGER,
    episode_number INTEGER,
    watched_at VARCHAR NOT NULL,
    play_count INTEGER NOT NULL DEFAULT 1,
    poster_tag VARCHAR,
    series_poster_tag VARCHAR,
    imdb_id VARCHAR,
    updated_at VARCHAR NOT NULL,
    PRIMARY KEY (jellyfin_id),
    CONSTRAINT ck_watched_items_kind CHECK (kind IN ('movie', 'tv')),
    CONSTRAINT ck_watched_items_play_count CHECK (play_count >= 1),
    CONSTRAINT ck_watched_items_kind_fields CHECK (
        (kind = 'movie' AND series_title IS NULL AND series_id IS NULL
         AND season_number IS NULL AND episode_number IS NULL
         AND series_poster_tag IS NULL) OR
        (kind = 'tv' AND series_title IS NOT NULL AND poster_tag IS NULL
         AND imdb_id IS NULL)
    )
);
CREATE INDEX ix_watched_items_watched_title
    ON watched_items (watched_at DESC, title COLLATE NOCASE);
CREATE INDEX ix_watched_items_kind_watched ON watched_items (kind, watched_at DESC);

CREATE TABLE completed_playbacks (
    event_key VARCHAR NOT NULL,
    jellyfin_id VARCHAR NOT NULL,
    session_id VARCHAR,
    completed_at VARCHAR NOT NULL,
    PRIMARY KEY (event_key),
    FOREIGN KEY(jellyfin_id) REFERENCES watched_items (jellyfin_id)
);

CREATE TABLE poster_cache (
    source_type VARCHAR NOT NULL,
    source_id VARCHAR NOT NULL,
    image_tag VARCHAR NOT NULL,
    cached_at VARCHAR NOT NULL,
    size_bytes INTEGER NOT NULL,
    PRIMARY KEY (source_type, source_id),
    CONSTRAINT ck_poster_cache_source_type CHECK (source_type IN ('movie', 'series')),
    CONSTRAINT ck_poster_cache_size_positive CHECK (size_bytes > 0)
);

CREATE TABLE sync_state (
    id INTEGER NOT NULL,
    last_started_at VARCHAR,
    last_finished_at VARCHAR,
    last_success_at VARCHAR,
    last_error VARCHAR,
    items_seen INTEGER NOT NULL,
    items_imported INTEGER NOT NULL DEFAULT 0,
    status VARCHAR NOT NULL DEFAULT 'never',
    PRIMARY KEY (id),
    CONSTRAINT ck_sync_state_singleton_id CHECK (id = 1),
    CONSTRAINT ck_sync_state_status CHECK (status IN ('never', 'running', 'success', 'failed')),
    CONSTRAINT ck_sync_state_items_seen CHECK (items_seen >= 0),
    CONSTRAINT ck_sync_state_items_imported CHECK (items_imported >= 0)
);
"""
]


def _schema_runs() -> list[str]:
    """Split the combined schema DDL into one statement per element."""
    runs = [statement.strip() for statement in V3_STATEMENTS[0].split(";")]
    return [statement for statement in runs if statement]


async def build_v3_database(path: Path, *, orphan_event: bool = False) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    connection = await aiosqlite.connect(path, isolation_level=None)
    try:
        await connection.execute("PRAGMA foreign_keys=OFF")
        await connection.execute("BEGIN")
        for statement in _schema_runs():
            await connection.execute(statement)
        await connection.executemany(
            """
            INSERT INTO watched_items (
                jellyfin_id, kind, title, series_title, series_id,
                season_number, episode_number, watched_at, play_count,
                poster_tag, series_poster_tag, imdb_id, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            [
                (
                    "6f9619ff-8b86-d011-b42d-00cf4fc964ff",
                    "movie",
                    "Completed film",
                    None,
                    None,
                    None,
                    None,
                    "2025-01-02T03:04:05.000000Z",
                    4,
                    "poster-tag",
                    None,
                    "tt1234567",
                    "2025-01-03T00:00:00.000000Z",
                ),
                (
                    "6f9619ff-8b86-d011-b42d-00cf4fc96400",
                    "tv",
                    "Old episode title",
                    "Example series",
                    "00000000-0000-0000-0000-000000000222",
                    2,
                    3,
                    "2025-01-01T12:00:00.000000Z",
                    2,
                    None,
                    "series-tag",
                    None,
                    "2025-01-03T00:00:00.000000Z",
                ),
            ],
        )
        await connection.executemany(
            """
            INSERT INTO completed_playbacks (event_key, jellyfin_id, session_id, completed_at)
            VALUES (?, ?, ?, ?)
            """,
            [
                (
                    "event-movie-1",
                    "6f9619ff-8b86-d011-b42d-00cf4fc964ff",
                    "s1",
                    "2025-01-01T18:00:00.000000Z",
                ),
                (
                    "event-movie-2",
                    "6f9619ff-8b86-d011-b42d-00cf4fc964ff",
                    "s2",
                    "2025-01-02T03:04:05.000000Z",
                ),
                (
                    "event-episode",
                    "6f9619ff-8b86-d011-b42d-00cf4fc96400",
                    "s3",
                    "2025-01-01T12:00:00.000000Z",
                ),
            ],
        )
        if orphan_event:
            await connection.execute(
                "INSERT INTO completed_playbacks (event_key, jellyfin_id, session_id, completed_at) "
                "VALUES ('orphan-event', '00000000-0000-0000-0000-000000000999', 's9', "
                "'2025-01-05T00:00:00.000000Z')"
            )
        await connection.executemany(
            """
            INSERT INTO poster_cache (source_type, source_id, image_tag, cached_at, size_bytes)
            VALUES (?, ?, ?, ?, ?)
            """,
            [
                (
                    "movie",
                    "6f9619ff-8b86-d011-b42d-00cf4fc964ff",
                    "poster-tag",
                    "2025-01-03T00:00:00.000000Z",
                    42,
                ),
                (
                    "series",
                    "00000000-0000-0000-0000-000000000222",
                    "series-tag",
                    "2025-01-03T00:00:00.000000Z",
                    17,
                ),
            ],
        )
        await connection.execute(
            """
            INSERT INTO sync_state (
                id, last_started_at, last_finished_at, last_success_at,
                last_error, items_seen, items_imported, status
            ) VALUES (1, '2025-01-03T00:00:00.000000Z', '2025-01-03T00:01:00.000000Z',
                      '2025-01-03T00:01:00.000000Z', 'Jellyfin returned an invalid response',
                      8, 7, 'failed')
            """
        )
        await connection.execute("PRAGMA user_version = 3")
        await connection.commit()
    finally:
        await connection.close()


@pytest.mark.asyncio
async def test_initialize_creates_v4_schema_pragmas_and_singleton(
    tmp_path: Path,
) -> None:
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

        assert version == 4
        assert foreign_keys == 1
        assert journal_mode.lower() == "wal"
        assert {
            "watched_items",
            "completed_playbacks",
            "poster_cache",
            "sync_state",
        }.issubset(tables)
        assert {
            "ix_watched_items_status_started_title",
            "ix_watched_items_status_watched_title",
        }.issubset({index["name"] for index in indexes})

        async with database.session() as session:
            state = await session.get(SyncState, 1)
            assert state is not None
            assert state.status == "never"
            assert state.items_seen == 0
    finally:
        await database.dispose()


@pytest.mark.asyncio
async def test_v4_models_persist_valid_rows_and_reject_invalid_shapes(
    tmp_path: Path,
) -> None:
    database = Database(tmp_path / "data")
    await database.initialize()
    try:
        async with database.session() as session:
            session.add_all(
                [
                    WatchedItem(
                        jellyfin_id="6f9619ff-8b86-d011-b42d-00cf4fc964ff",
                        kind="movie",
                        title="Verified film",
                        watch_status="watched",
                        started_at=None,
                        watched_at="2025-01-02T03:04:05.000000Z",
                        manually_watched_at=None,
                        play_count=2,
                        is_favorite=True,
                        poster_tag="primary-tag",
                        imdb_id="tt1234567",
                        updated_at="2025-01-02T03:04:05.000000Z",
                    ),
                    WatchedItem(
                        jellyfin_id="6f9619ff-8b86-d011-b42d-00cf4fc965ff",
                        kind="movie",
                        title="Started film",
                        watch_status="started",
                        started_at="2025-01-03T03:04:05.000000Z",
                        watched_at="2025-01-02T03:04:05.000000Z",
                        manually_watched_at=None,
                        play_count=1,
                        is_favorite=False,
                        updated_at="2025-01-03T03:04:05.000000Z",
                    ),
                    WatchedItem(
                        jellyfin_id="6f9619ff-8b86-d011-b42d-00cf4fc966ff",
                        kind="movie",
                        title="Manually marked film",
                        watch_status="watched",
                        started_at="2025-01-04T03:04:05.000000Z",
                        watched_at=None,
                        manually_watched_at="2025-01-04T05:00:00.000000Z",
                        play_count=0,
                        is_favorite=False,
                        updated_at="2025-01-04T05:00:00.000000Z",
                    ),
                    WatchedItem(
                        jellyfin_id="6f9619ff-8b86-d011-b42d-00cf4fc96400",
                        kind="tv",
                        title="Episode",
                        watch_status="watched",
                        watched_at="2025-01-02T03:04:05.000000Z",
                        play_count=3,
                        series_title="Example series",
                        series_id="00000000-0000-0000-0000-000000000222",
                        season_number=1,
                        episode_number=2,
                        updated_at="2025-01-02T03:04:05.000000Z",
                    ),
                ]
            )
            await session.commit()

        async with database.session() as session:
            item = await session.get(
                WatchedItem, "6f9619ff-8b86-d011-b42d-00cf4fc964ff"
            )
            assert item is not None
            assert item.play_count == 2
            assert item.is_favorite is True
            assert item.watch_status == "watched"

        invalid_items = [
            WatchedItem(
                jellyfin_id="invalid-status",
                kind="movie",
                title="Invalid",
                watch_status="paused",
                watched_at="2025-01-02T03:04:05.000000Z",
                play_count=1,
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
            WatchedItem(
                jellyfin_id="invalid-kind",
                kind="music",
                title="Invalid",
                watch_status="watched",
                watched_at="2025-01-02T03:04:05.000000Z",
                play_count=1,
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
            WatchedItem(
                jellyfin_id="negative-play-count",
                kind="movie",
                title="Invalid",
                watch_status="watched",
                watched_at="2025-01-02T03:04:05.000000Z",
                play_count=-1,
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
            WatchedItem(
                jellyfin_id="count-without-date",
                kind="movie",
                title="Invalid",
                watch_status="watched",
                watched_at=None,
                play_count=1,
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
            WatchedItem(
                jellyfin_id="date-without-count",
                kind="movie",
                title="Invalid",
                watch_status="watched",
                watched_at="2025-01-02T03:04:05.000000Z",
                play_count=0,
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
            WatchedItem(
                jellyfin_id="started-without-timestamp",
                kind="movie",
                title="Invalid",
                watch_status="started",
                started_at=None,
                watched_at=None,
                play_count=0,
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
            WatchedItem(
                jellyfin_id="started-with-manual",
                kind="movie",
                title="Invalid",
                watch_status="started",
                started_at="2025-01-02T03:04:05.000000Z",
                manually_watched_at="2025-01-02T04:00:00.000000Z",
                watched_at=None,
                play_count=0,
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
            WatchedItem(
                jellyfin_id="watched-without-evidence",
                kind="movie",
                title="Invalid",
                watch_status="watched",
                watched_at=None,
                manually_watched_at=None,
                play_count=0,
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
            WatchedItem(
                jellyfin_id="manual-on-tv",
                kind="tv",
                title="Invalid",
                watch_status="watched",
                watched_at="2025-01-02T03:04:05.000000Z",
                manually_watched_at="2025-01-02T04:00:00.000000Z",
                play_count=1,
                series_title="Series",
                series_id="00000000-0000-0000-0000-000000000222",
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
            WatchedItem(
                jellyfin_id="tv-without-watched-date",
                kind="tv",
                title="Invalid",
                watch_status="watched",
                watched_at=None,
                play_count=1,
                series_title="Series",
                series_id="00000000-0000-0000-0000-000000000222",
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
            WatchedItem(
                jellyfin_id="tv-started",
                kind="tv",
                title="Invalid",
                watch_status="started",
                started_at="2025-01-02T03:04:05.000000Z",
                watched_at=None,
                play_count=0,
                series_title="Series",
                series_id="00000000-0000-0000-0000-000000000222",
                updated_at="2025-01-02T03:04:05.000000Z",
            ),
            WatchedItem(
                jellyfin_id="movie-with-series",
                kind="movie",
                title="Invalid",
                watch_status="watched",
                watched_at="2025-01-02T03:04:05.000000Z",
                play_count=1,
                series_title="Series",
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
            assert len((await session.exec(select(WatchedItem))).all()) == 4
    finally:
        await database.dispose()


@pytest.mark.asyncio
async def test_legacy_schema_is_reset_to_fresh_v4(tmp_path: Path) -> None:
    data_dir = tmp_path / "data"
    legacy = Database(data_dir)
    await legacy.initialize()
    async with legacy.session() as session:
        session.add(
            WatchedItem(
                jellyfin_id="6f9619ff-8b86-d011-b42d-00cf4fc964ff",
                kind="movie",
                title="Legacy row",
                watch_status="watched",
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
        assert version == 4
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
        await connection.execute(text("PRAGMA user_version = 5"))
    await first.dispose()

    second = Database(data_dir)
    with pytest.raises(RuntimeError, match="newer than supported"):
        await second.initialize()
    await second.dispose()


@pytest.mark.asyncio
async def test_v3_database_migrates_to_v4_preserving_everything(
    tmp_path: Path,
) -> None:
    data_dir = tmp_path / "data"
    database_path = data_dir / "tana.sqlite3"
    await build_v3_database(database_path)
    (data_dir / "posters").mkdir(parents=True)
    (data_dir / "posters" / "6f9619ff-8b86-d011-b42d-00cf4fc964ff.jpg").write_bytes(
        b"cached poster bytes"
    )

    database = Database(data_dir)
    await database.initialize()
    try:
        async with database.session() as session:
            items = (await session.exec(select(WatchedItem))).all()
            events = (await session.exec(select(CompletedPlayback))).all()
            posters = (await session.exec(select(PosterCache))).all()
            state = await session.get(SyncState, 1)
        async with database.engine.connect() as connection:
            version = (
                await connection.execute(text("PRAGMA user_version"))
            ).scalar_one()
            violations = (
                await connection.execute(text("PRAGMA foreign_key_check"))
            ).fetchall()

        assert version == 4
        assert violations == []
        assert len(items) == 2
        assert len(events) == 3
        assert len(posters) == 2
        assert state is not None
        assert state.status == "failed"
        assert state.last_error == "Jellyfin returned an invalid response"
        assert state.items_seen == 8
        assert state.items_imported == 7
        assert (
            state.last_finished_at
            == state.last_success_at
            == "2025-01-03T00:01:00.000000Z"
        )

        by_id = {item.jellyfin_id: item for item in items}
        movie = by_id["6f9619ff-8b86-d011-b42d-00cf4fc964ff"]
        assert movie.kind == "movie"
        assert movie.title == "Completed film"
        assert movie.watch_status == "watched"
        assert movie.started_at is None
        assert movie.manually_watched_at is None
        assert movie.is_favorite is False
        assert movie.watched_at == "2025-01-02T03:04:05.000000Z"
        assert movie.play_count == 4
        assert movie.poster_tag == "poster-tag"
        assert movie.imdb_id == "tt1234567"

        episode = by_id["6f9619ff-8b86-d011-b42d-00cf4fc96400"]
        assert episode.kind == "tv"
        assert episode.title == "Old episode title"
        assert episode.watch_status == "watched"
        assert episode.series_title == "Example series"
        assert episode.series_id == "00000000-0000-0000-0000-000000000222"
        assert episode.season_number == 2
        assert episode.episode_number == 3
        assert episode.play_count == 2
        assert episode.series_poster_tag == "series-tag"
        assert episode.poster_tag is None
        assert episode.imdb_id is None

        event_keys = {event.event_key for event in events}
        assert event_keys == {"event-movie-1", "event-movie-2", "event-episode"}
        assert (
            data_dir / "posters" / "6f9619ff-8b86-d011-b42d-00cf4fc964ff.jpg"
        ).read_bytes() == b"cached poster bytes"
    finally:
        await database.dispose()


@pytest.mark.asyncio
async def test_v3_migration_rolls_back_on_foreign_key_violation(
    tmp_path: Path,
) -> None:
    data_dir = tmp_path / "data"
    database_path = data_dir / "tana.sqlite3"
    await build_v3_database(database_path, orphan_event=True)

    database = Database(data_dir)
    with pytest.raises(RuntimeError, match="foreign key"):
        await database.initialize()
    await database.dispose()

    # The failed migration leaves the recoverable v3 database untouched.
    verification = await aiosqlite.connect(database_path)
    try:
        version = (
            await (await verification.execute("PRAGMA user_version")).fetchone()
        )[0]
        items = (
            await (
                await verification.execute("SELECT COUNT(*) FROM watched_items")
            ).fetchone()
        )[0]
        events = (
            await (
                await verification.execute("SELECT COUNT(*) FROM completed_playbacks")
            ).fetchone()
        )[0]
    finally:
        await verification.close()

    assert version == 3
    assert items == 2
    assert events == 4
