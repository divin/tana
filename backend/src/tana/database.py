from __future__ import annotations

import asyncio
import os
import shutil
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Any

import aiosqlite
from sqlalchemy import MetaData, event, text
from sqlalchemy.dialects import sqlite as sqlite_dialect
from sqlalchemy.engine import URL
from sqlalchemy.ext.asyncio import AsyncEngine, async_sessionmaker, create_async_engine
from sqlalchemy.schema import CreateIndex, CreateTable
from sqlmodel import SQLModel
from sqlmodel.ext.asyncio.session import AsyncSession

from tana.models import SyncState, WatchedItem

SCHEMA_VERSION = 4
DEFAULT_DATA_DIR = Path("/data")
DATABASE_FILENAME = "tana.sqlite3"
POSTER_DIRECTORY_NAME = "posters"

# Old v3 rows always represent a confirmed completion: migrated rows are watched,
# carry no start/manual timestamps, and are not locally favorited.
_MIGRATION_INSERT_SQL = """
INSERT INTO watched_items_v4 (
    jellyfin_id, kind, title, watch_status, started_at, watched_at,
    manually_watched_at, play_count, is_favorite, series_title, series_id,
    season_number, episode_number, poster_tag, series_poster_tag, imdb_id,
    updated_at
)
SELECT
    jellyfin_id, kind, title, 'watched', NULL, watched_at, NULL, play_count, 0,
    series_title, series_id, season_number, episode_number, poster_tag,
    series_poster_tag, imdb_id, updated_at
FROM watched_items
"""


def _v3_to_v4_ddl() -> tuple[str, list[str]]:
    """Compile the v4 ``watched_items`` DDL for the migration connection."""
    migration_metadata = MetaData()
    v4_clone = WatchedItem.__table__.to_metadata(
        migration_metadata, name="watched_items_v4"
    )
    dialect = sqlite_dialect.dialect()
    create_clause = str(CreateTable(v4_clone).compile(dialect=dialect))
    index_statements = [
        str(CreateIndex(index).compile(dialect=dialect))
        for index in WatchedItem.__table__.indexes
    ]
    return create_clause, index_statements


class Database:
    def __init__(self, data_dir: Path = DEFAULT_DATA_DIR) -> None:
        self.data_dir = data_dir
        self.database_path = data_dir / DATABASE_FILENAME
        self.poster_path = data_dir / POSTER_DIRECTORY_NAME
        self.engine: AsyncEngine | None = None
        self.session_factory: async_sessionmaker[AsyncSession] | None = None

    async def initialize(self) -> None:
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.poster_path.mkdir(parents=True, exist_ok=True)
        if not self.data_dir.is_dir() or not self.poster_path.is_dir():
            raise RuntimeError("Data and poster paths must be directories")
        if not all(
            os.access(path, os.W_OK | os.X_OK)
            for path in (self.data_dir, self.poster_path)
        ):
            raise RuntimeError("Data and poster paths must be writable")

        database_url = URL.create(
            "sqlite+aiosqlite",
            database=str(self.database_path.resolve()),
        )
        self.engine = create_async_engine(database_url, echo=False)
        self._configure_sqlite(self.engine)
        self.session_factory = async_sessionmaker(
            self.engine,
            class_=AsyncSession,
            expire_on_commit=False,
        )

        async with self.engine.connect() as connection:
            schema_version = int(
                (await connection.execute(text("PRAGMA user_version"))).scalar_one()
            )
        if schema_version > SCHEMA_VERSION:
            await self.dispose()
            raise RuntimeError(
                f"Database schema version {schema_version} is newer than supported "
                f"version {SCHEMA_VERSION}"
            )

        if schema_version in {1, 2}:
            # Pre-webhook play counts include starts and cannot be migrated as
            # completions; recreate directly at the fresh v4 schema.
            async with self.engine.begin() as connection:
                await connection.run_sync(SQLModel.metadata.drop_all)
                await connection.run_sync(SQLModel.metadata.create_all)
                await connection.execute(
                    text(f"PRAGMA user_version = {SCHEMA_VERSION}")
                )
            await asyncio.to_thread(shutil.rmtree, self.poster_path, True)
            self.poster_path.mkdir(parents=True, exist_ok=True)
        elif schema_version == 3:
            # Preserving v3-to-v4 migration on its own connection so foreign-key
            # enforcement can be turned off before the migration transaction.
            await self._migrate_v3_to_v4()
        else:
            # Fresh databases (version 0) create the full v4 schema; version 4
            # opens normally because create_all is a no-op on existing tables.
            async with self.engine.begin() as connection:
                await connection.run_sync(SQLModel.metadata.create_all)
                await connection.execute(
                    text(f"PRAGMA user_version = {SCHEMA_VERSION}")
                )

        async with self.session() as session:
            state = await session.get(SyncState, 1)
            if state is None:
                session.add(SyncState(id=1))
                await session.commit()

    async def _migrate_v3_to_v4(self) -> None:
        create_clause, index_statements = _v3_to_v4_ddl()
        # Autocommit mode lets the migration own BEGIN/COMMIT so DDL, the copy,
        # the rename, and the version bump are one atomic unit.
        connection = await aiosqlite.connect(self.database_path, isolation_level=None)
        try:
            # PRAGMA foreign_keys only takes effect outside a transaction, so it
            # must be disabled before BEGIN and restored after COMMIT/ROLLBACK.
            await connection.execute("PRAGMA foreign_keys=OFF")
            await connection.execute("BEGIN")
            try:
                await connection.execute(create_clause)
                await connection.execute(_MIGRATION_INSERT_SQL)
                await connection.execute("DROP TABLE watched_items")
                await connection.execute(
                    "ALTER TABLE watched_items_v4 RENAME TO watched_items"
                )
                for index_ddl in index_statements:
                    await connection.execute(index_ddl)
                violations = await (
                    await connection.execute("PRAGMA foreign_key_check")
                ).fetchall()
                if violations:
                    raise RuntimeError(
                        "v3-to-v4 migration left unresolved foreign keys"
                    )
                await connection.execute(f"PRAGMA user_version = {SCHEMA_VERSION}")
            except Exception:
                await connection.rollback()
                raise
            await connection.commit()
        finally:
            await connection.execute("PRAGMA foreign_keys=ON")
            await connection.close()

    @staticmethod
    def _configure_sqlite(engine: AsyncEngine) -> None:
        @event.listens_for(engine.sync_engine, "connect")
        def set_sqlite_pragmas(dbapi_connection: Any, _connection_record: Any) -> None:
            cursor = dbapi_connection.cursor()
            try:
                cursor.execute("PRAGMA foreign_keys=ON")
                cursor.execute("PRAGMA journal_mode=WAL")
                cursor.fetchone()
                cursor.execute("PRAGMA busy_timeout=5000")
            finally:
                cursor.close()

    @asynccontextmanager
    async def session(self) -> AsyncIterator[AsyncSession]:
        if self.session_factory is None:
            raise RuntimeError("Database has not been initialized")
        async with self.session_factory() as session:
            yield session

    async def dispose(self) -> None:
        if self.engine is not None:
            await self.engine.dispose()
            self.engine = None
            self.session_factory = None
