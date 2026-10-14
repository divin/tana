from __future__ import annotations

from datetime import UTC, date, datetime, time, timedelta
from zoneinfo import ZoneInfo

from apscheduler.schedulers.asyncio import AsyncIOScheduler
from apscheduler.triggers.date import DateTrigger

from tana.sync import SyncService
from tana.time_utils import resolve_local_time


def next_daily_run(now: datetime, sync_time: str, timezone: ZoneInfo) -> datetime:
    if now.tzinfo is None:
        raise ValueError("now must be timezone-aware")
    hour, minute = (int(part) for part in sync_time.split(":"))
    local_today = now.astimezone(timezone).date()
    for days_ahead in range(370):
        target_date: date = local_today + timedelta(days=days_ahead)
        target = resolve_local_time(
            datetime.combine(target_date, time(hour=hour, minute=minute)), timezone
        )
        if target.astimezone(UTC) > now.astimezone(UTC):
            return target
    raise RuntimeError("Could not determine next daily sync")


class DailySyncScheduler:
    """Schedule a single APScheduler date job, recalculated for DST-safe local daily time."""

    def __init__(
        self, service: SyncService, sync_time: str, timezone_name: str
    ) -> None:
        self.service = service
        self.sync_time = sync_time
        self.timezone = ZoneInfo(timezone_name)
        self.scheduler = AsyncIOScheduler(timezone=self.timezone)
        self._shutdown = False

    def start(self) -> None:
        self.scheduler.start()
        self._schedule_next()

    def _schedule_next(self, now: datetime | None = None) -> datetime:
        run_at = next_daily_run(now or datetime.now(UTC), self.sync_time, self.timezone)
        self.scheduler.add_job(
            self._run_and_reschedule,
            trigger=DateTrigger(run_date=run_at, timezone=self.timezone),
            id="daily-sync",
            replace_existing=True,
            coalesce=True,
            misfire_grace_time=24 * 60 * 60,
            max_instances=1,
        )
        return run_at

    async def _run_and_reschedule(self) -> None:
        try:
            await self.service.run()
        finally:
            if not self._shutdown:
                self._schedule_next()

    def shutdown(self) -> None:
        self._shutdown = True
        if self.scheduler.running:
            self.scheduler.shutdown(wait=False)
