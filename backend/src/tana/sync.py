from __future__ import annotations

import asyncio
import logging
from collections.abc import Sequence
from datetime import UTC, datetime

from tana.database import Database
from tana.jellyfin import (
    JellyfinClient,
    JellyfinError,
    JellyfinResponseError,
    LibraryItemData,
)
from tana.models import SyncState, WatchedItem

logger = logging.getLogger(__name__)


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="microseconds").replace("+00:00", "Z")


class SyncService:
    def __init__(self, database: Database, jellyfin: JellyfinClient) -> None:
        self.database = database
        self.jellyfin = jellyfin
        self._lock = asyncio.Lock()

    async def run(self) -> bool:
        async with self._lock:
            started_at = utc_now()
            await self._mark_started(started_at)
            try:
                items, items_seen = await self.jellyfin.fetch_library_items()
            except asyncio.CancelledError:
                await asyncio.shield(self._mark_failed("Sync interrupted"))
                raise
            except Exception as error:  # noqa: BLE001 - keep scheduled sync failures contained
                await self._mark_failed(self._safe_error(error))
                logger.error("Jellyfin sync failed: %s", self._safe_error(error))
                return False

            imported = self._deduplicate(items)
            finished_at = utc_now()
            imported_count = 0
            try:
                async with self.database.session() as session, session.begin():
                    for item in imported:
                        tracked = await session.get(WatchedItem, item.jellyfin_id)
                        if tracked is None or tracked.kind != item.kind:
                            continue
                        tracked.title = item.title
                        tracked.series_title = item.series_title
                        tracked.series_id = item.series_id
                        tracked.season_number = item.season_number
                        tracked.episode_number = item.episode_number
                        tracked.poster_tag = item.poster_tag
                        tracked.series_poster_tag = item.series_poster_tag
                        tracked.imdb_id = item.imdb_id
                        tracked.updated_at = finished_at
                        imported_count += 1

                    state = await session.get(SyncState, 1)
                    if state is None:
                        state = SyncState(id=1)
                        session.add(state)
                    state.status = "success"
                    state.last_finished_at = finished_at
                    state.last_success_at = finished_at
                    state.last_error = None
                    state.items_seen = items_seen
                    state.items_imported = imported_count
            except asyncio.CancelledError:
                await asyncio.shield(self._mark_failed("Sync interrupted"))
                raise
            except Exception as error:  # noqa: BLE001 - record unexpected DB unit-of-work failures
                await self._mark_failed("Database write failed")
                logger.error(
                    "Jellyfin sync failed: database write failed (%s)",
                    type(error).__name__,
                )
                return False

            logger.info(
                "Jellyfin sync completed: %d DTOs seen, %d tracked items refreshed",
                items_seen,
                imported_count,
            )
            return True

    async def _mark_started(self, started_at: str) -> None:
        async with self.database.session() as session, session.begin():
            state = await session.get(SyncState, 1)
            if state is None:
                state = SyncState(id=1)
                session.add(state)
            state.status = "running"
            state.last_started_at = started_at

    async def _mark_failed(self, error_message: str) -> None:
        failed_at = utc_now()
        try:
            async with self.database.session() as session, session.begin():
                state = await session.get(SyncState, 1)
                if state is None:
                    state = SyncState(id=1)
                    session.add(state)
                state.status = "failed"
                state.last_finished_at = failed_at
                state.last_error = error_message[:500]
        except Exception:  # noqa: BLE001 - status persistence must not mask the original failure
            logger.error("Unable to persist failed Jellyfin sync status")

    @staticmethod
    def _deduplicate(items: Sequence[LibraryItemData]) -> list[LibraryItemData]:
        by_id = {item.jellyfin_id: item for item in items}
        return list(by_id.values())

    @staticmethod
    def _safe_error(error: Exception) -> str:
        if isinstance(error, JellyfinResponseError):
            return "Jellyfin returned an invalid response"
        if isinstance(error, JellyfinError):
            return "Jellyfin request failed"
        return "Jellyfin sync failed"

    async def wait_until_idle(self) -> None:
        async with self._lock:
            return
