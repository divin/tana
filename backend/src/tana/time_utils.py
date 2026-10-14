from __future__ import annotations

from datetime import UTC, datetime, timedelta
from zoneinfo import ZoneInfo


def _valid_local_candidates(
    local_naive: datetime, timezone: ZoneInfo
) -> list[datetime]:
    candidates: dict[datetime, datetime] = {}
    for fold in (0, 1):
        local = local_naive.replace(tzinfo=timezone, fold=fold)
        utc_value = local.astimezone(UTC)
        round_trip = utc_value.astimezone(timezone).replace(tzinfo=None)
        if round_trip == local_naive:
            candidates[utc_value] = local
    return list(candidates.values())


def resolve_local_time(local_naive: datetime, timezone: ZoneInfo) -> datetime:
    """Resolve gaps forward and ambiguous times to their earlier occurrence."""
    candidates = _valid_local_candidates(local_naive, timezone)
    if candidates:
        return min(candidates, key=lambda value: value.astimezone(UTC))

    probe = local_naive
    # The minute precision covers configured schedule times and the calendar boundaries.
    for _ in range(24 * 60 + 1):
        candidates = _valid_local_candidates(probe, timezone)
        if candidates:
            return min(candidates, key=lambda value: value.astimezone(UTC))
        probe += timedelta(minutes=1)
    raise ValueError("Could not resolve local time within one day")


def format_utc(value: datetime) -> str:
    if value.tzinfo is None:
        value = value.replace(tzinfo=UTC)
    return (
        value.astimezone(UTC).isoformat(timespec="microseconds").replace("+00:00", "Z")
    )
