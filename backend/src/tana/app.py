from __future__ import annotations

import asyncio
import logging
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Annotated

from fastapi import Depends, FastAPI
from fastapi.responses import JSONResponse
from fastapi.staticfiles import StaticFiles
from sqlalchemy import text
from sqlalchemy.exc import SQLAlchemyError
from sqlmodel.ext.asyncio.session import AsyncSession

from tana.api import router as api_router
from tana.app_types import get_session
from tana.config import Settings, load_settings
from tana.database import DEFAULT_DATA_DIR, Database
from tana.jellyfin import JellyfinClient
from tana.posters import PosterService
from tana.scheduler import DailySyncScheduler
from tana.sync import SyncService
from tana.webhook_api import router as webhook_router
from tana.webhooks import WebhookService


def find_static_directory() -> Path:
    for directory in (Path.cwd(), *Path.cwd().parents):
        candidate = directory / "frontend" / "public"
        if candidate.is_dir():
            return candidate
    return Path.cwd() / "frontend" / "public"


def create_app(
    *,
    settings: Settings | None = None,
    data_dir: Path = DEFAULT_DATA_DIR,
    static_dir: Path | None = None,
    start_background_sync: bool = True,
) -> FastAPI:
    site_directory = static_dir or find_static_directory()

    @asynccontextmanager
    async def lifespan(app: FastAPI) -> AsyncIterator[None]:
        app_settings = settings or load_settings()
        logging.basicConfig(level=getattr(logging, app_settings.LOG_LEVEL))
        # HTTP request logs contain Jellyfin user/item IDs and image tags in URLs.
        logging.getLogger("httpx").setLevel(logging.WARNING)
        logging.getLogger("httpcore").setLevel(logging.WARNING)
        database = Database(data_dir)
        try:
            await database.initialize()
        except Exception:
            await database.dispose()
            raise
        jellyfin = JellyfinClient(app_settings)
        sync_service = SyncService(database, jellyfin)
        scheduler = DailySyncScheduler(
            sync_service, app_settings.SYNC_TIME, app_settings.TZ
        )
        app.state.settings = app_settings
        app.state.database = database
        app.state.jellyfin = jellyfin
        app.state.sync_service = sync_service
        app.state.webhook_service = WebhookService(
            database,
            app_settings.JELLYFIN_USER_ID,
            app_settings.PLAYBACK_COMPLETION_THRESHOLD,
            jellyfin,
        )
        app.state.scheduler = scheduler
        app.state.poster_service = PosterService(jellyfin, database.poster_path)
        initial_sync_task: asyncio.Task[None] | None = None
        if start_background_sync:
            scheduler.start()
            initial_sync_task = asyncio.create_task(
                sync_service.run(), name="startup-sync"
            )
        app.state.initial_sync_task = initial_sync_task
        try:
            yield
        finally:
            scheduler.shutdown()
            shutdown_deadline = asyncio.get_running_loop().time() + 30
            if initial_sync_task is not None:
                try:
                    await asyncio.wait_for(
                        asyncio.shield(initial_sync_task),
                        timeout=max(
                            0, shutdown_deadline - asyncio.get_running_loop().time()
                        ),
                    )
                except TimeoutError:
                    initial_sync_task.cancel()
            try:
                await asyncio.wait_for(
                    sync_service.wait_until_idle(),
                    timeout=max(
                        0, shutdown_deadline - asyncio.get_running_loop().time()
                    ),
                )
            except TimeoutError:
                pass
            await jellyfin.close()
            await database.dispose()

    app = FastAPI(title="Tana", lifespan=lifespan)

    @app.get("/healthz")
    async def healthz(
        session: Annotated[AsyncSession, Depends(get_session)],
    ) -> dict[str, str]:
        try:
            await session.exec(text("SELECT 1"))
        except SQLAlchemyError:
            return JSONResponse(status_code=503, content={"status": "error"})
        return {"status": "ok"}

    app.include_router(api_router)
    app.include_router(webhook_router)
    app.mount("/", StaticFiles(directory=site_directory, html=True), name="frontend")
    return app


app = create_app()
