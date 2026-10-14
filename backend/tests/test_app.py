from __future__ import annotations

import asyncio
from pathlib import Path

import pytest
from httpx import ASGITransport, AsyncClient
from pydantic import SecretStr
from sqlalchemy import event
from sqlalchemy.exc import SQLAlchemyError

from tana import app as app_module
from tana.app import create_app
from tana.config import Settings


@pytest.mark.asyncio
async def test_generated_zola_site_and_health_endpoint_share_one_origin(
    tmp_path: Path,
) -> None:
    static_dir = Path(__file__).resolve().parents[2] / "frontend" / "public"
    assert (static_dir / "index.html").is_file()
    settings = Settings(
        _env_file=None,
        JELLYFIN_URL="http://jellyfin.example:8096",
        JELLYFIN_API_KEY=SecretStr("test-key"),
        JELLYFIN_USER_ID="test-user",
        TZ="UTC",
    )
    app = create_app(
        settings=settings,
        data_dir=tmp_path / "data",
        static_dir=static_dir,
        start_background_sync=False,
    )

    async with (
        app.router.lifespan_context(app),
        AsyncClient(
            transport=ASGITransport(app=app),
            base_url="http://test",
        ) as client,
    ):
        health = await client.get("/healthz")
        page = await client.get("/")
        stylesheet = await client.get("/main.css")
        script = await client.get("/scripts/main.mjs")

    assert health.status_code == 200
    assert health.json() == {"status": "ok"}
    assert page.status_code == 200
    assert "<h1>Watched media</h1>" in page.text
    assert stylesheet.status_code == 200
    assert script.status_code == 200


@pytest.mark.asyncio
async def test_startup_sync_is_background_and_scheduler_is_configured(
    tmp_path: Path, monkeypatch
) -> None:
    static_dir = Path(__file__).resolve().parents[2] / "frontend" / "public"
    entered_fetch = asyncio.Event()
    continue_fetch = asyncio.Event()

    class SlowJellyfinClient:
        def __init__(self, settings) -> None:
            self.settings = settings

        async def fetch_library_items(self):
            entered_fetch.set()
            await continue_fetch.wait()
            return [], 0

        async def close(self) -> None:
            return

    monkeypatch.setattr(app_module, "JellyfinClient", SlowJellyfinClient)
    settings = Settings(
        _env_file=None,
        JELLYFIN_URL="http://jellyfin.example:8096",
        JELLYFIN_API_KEY=SecretStr("test-key"),
        JELLYFIN_USER_ID="test-user",
        TZ="Europe/Berlin",
        SYNC_TIME="03:00",
    )
    app = create_app(
        settings=settings, data_dir=tmp_path / "data", static_dir=static_dir
    )

    async with app.router.lifespan_context(app):
        await asyncio.wait_for(entered_fetch.wait(), timeout=1)
        assert len(app.state.scheduler.scheduler.get_jobs()) == 1
        async with AsyncClient(
            transport=ASGITransport(app=app),
            base_url="http://test",
        ) as client:
            response = await client.get("/healthz")
        assert response.status_code == 200
        continue_fetch.set()
        await asyncio.wait_for(app.state.initial_sync_task, timeout=1)


@pytest.mark.asyncio
async def test_health_returns_503_when_database_query_fails(tmp_path: Path) -> None:
    settings = Settings(
        _env_file=None,
        JELLYFIN_URL="http://jellyfin.example:8096",
        JELLYFIN_API_KEY=SecretStr("test-key"),
        JELLYFIN_USER_ID="test-user",
        TZ="UTC",
    )
    app = create_app(
        settings=settings,
        data_dir=tmp_path / "data",
        static_dir=Path(__file__).resolve().parents[2] / "frontend" / "public",
        start_background_sync=False,
    )

    async with app.router.lifespan_context(app):

        def fail_health_query(
            _connection, _cursor, statement, _parameters, _context, _many
        ):
            if statement == "SELECT 1":
                raise SQLAlchemyError("private database failure")

        assert app.state.database.engine is not None
        event.listen(
            app.state.database.engine.sync_engine,
            "before_cursor_execute",
            fail_health_query,
        )
        async with AsyncClient(
            transport=ASGITransport(app=app, raise_app_exceptions=False),
            base_url="http://test",
        ) as client:
            response = await client.get("/healthz")

    assert response.status_code == 503
    assert response.json() == {"status": "error"}
    assert "private database failure" not in response.text
