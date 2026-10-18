from __future__ import annotations

import asyncio
import os
import shutil
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Any

from sqlalchemy import URL, event, text
from sqlalchemy.ext.asyncio import AsyncEngine, async_sessionmaker, create_async_engine
from sqlmodel import SQLModel
from sqlmodel.ext.asyncio.session import AsyncSession

from tana.models import SyncState

SCHEMA_VERSION = 5
DEFAULT_DATA_DIR = Path("/data")
DATABASE_FILENAME = "tana.sqlite3"
POSTER_DIRECTORY_NAME = "posters"


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

        self._create_engine()
        schema_version = await self._read_schema_version()
        if schema_version != SCHEMA_VERSION:
            # Version 3 is a complete rewrite of both the schema and the data
            # origin; only the current schema is supported and any other file is
            # deleted and recreated fresh.
            await self._recreate_after(schema_version)
        else:
            async with self.engine.begin() as connection:
                await connection.run_sync(SQLModel.metadata.create_all)

        async with self.session() as session:
            state = await session.get(SyncState, 1)
            if state is None:
                session.add(SyncState(id=1))
                await session.commit()

    def _create_engine(self) -> None:
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

    async def _read_schema_version(self) -> int:
        async with self.engine.connect() as connection:
            return int(
                (await connection.execute(text("PRAGMA user_version"))).scalar_one()
            )

    async def _recreate_after(self, schema_version: int) -> None:
        if self.engine is not None:
            await self.engine.dispose()
        self.engine = None
        self.session_factory = None
        # Every non-matching file is discarded, including empty files (version 0)
        # that may have been created by the version probe; only v5 data survives.
        self.database_path.unlink(missing_ok=True)
        await asyncio.to_thread(shutil.rmtree, self.poster_path, True)
        self.poster_path.mkdir(parents=True, exist_ok=True)
        self._create_engine()
        async with self.engine.begin() as connection:
            await connection.run_sync(SQLModel.metadata.create_all)
            await connection.execute(text(f"PRAGMA user_version = {SCHEMA_VERSION}"))

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
