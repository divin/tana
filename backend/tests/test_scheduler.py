from __future__ import annotations

from datetime import UTC, datetime
from zoneinfo import ZoneInfo

from tana.scheduler import next_daily_run
from tana.time_utils import resolve_local_time


def test_daily_run_uses_configured_timezone_and_next_date_if_time_passed() -> None:
    timezone = ZoneInfo("Europe/Berlin")
    now = datetime(2025, 1, 1, 3, 0, tzinfo=timezone)

    upcoming = next_daily_run(now, "03:00", timezone)

    assert upcoming == datetime(2025, 1, 2, 3, 0, tzinfo=timezone)


def test_nonexistent_daylight_saving_wall_time_moves_to_first_valid_minute() -> None:
    timezone = ZoneInfo("Europe/Berlin")
    resolved = resolve_local_time(datetime(2025, 3, 30, 2, 30), timezone)

    assert resolved == datetime(2025, 3, 30, 3, 0, tzinfo=timezone)


def test_ambiguous_daylight_saving_wall_time_uses_earlier_occurrence() -> None:
    timezone = ZoneInfo("Europe/Berlin")
    resolved = resolve_local_time(datetime(2025, 10, 26, 2, 30), timezone)

    assert resolved.fold == 0
    assert resolved.utcoffset().total_seconds() == 2 * 60 * 60
    assert resolved.astimezone(UTC) == datetime(2025, 10, 26, 0, 30, tzinfo=UTC)


def test_sync_scheduled_for_gap_runs_once_at_first_valid_instant() -> None:
    timezone = ZoneInfo("Europe/Berlin")
    now = datetime(2025, 3, 30, 0, 0, tzinfo=UTC)

    upcoming = next_daily_run(now, "02:30", timezone)

    assert upcoming == datetime(2025, 3, 30, 3, 0, tzinfo=timezone)
