from __future__ import annotations

import asyncio
import hashlib
import logging
import re
from collections import defaultdict
from collections.abc import Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from uuid import UUID

from sqlmodel import select
from sqlmodel.ext.asyncio.session import AsyncSession

from tana.config import Settings
from tana.database import Database
from tana.jellyfin import (
    ActivityEntry,
    JellyfinClient,
    JellyfinError,
    JellyfinForbiddenError,
    JellyfinResponseError,
    JellyfinTransportError,
    LibraryItemData,
)
from tana.models import SyncState, UserDataSnapshot, WatchedItem, WatchEvent

logger = logging.getLogger(__name__)


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="microseconds").replace("+00:00", "Z")


def _userdata_event_key(
    user_id: str, item_id: str, last_played_date: str, index: int
) -> str:
    payload = "\0".join(
        (user_id, item_id, "userdata", last_played_date, str(index))
    ).encode()
    return hashlib.sha256(payload).hexdigest()


def _activity_event_key(log_entry_id: int) -> str:
    return hashlib.sha256(f"activity\0{log_entry_id}".encode()).hexdigest()


def _epoch_seconds(value: str) -> float:
    normalized = value[:-1] + "+00:00" if value.endswith("Z") else value
    return datetime.fromisoformat(normalized).timestamp()


def _completion_like_stop_ids(
    entries: Sequence[ActivityEntry],
    runtime_ticks: int | None,
    threshold: float,
    *,
    merge_gap_seconds: float = 180.0,
) -> tuple[set[int], set[int]]:
    """Classify an item's activity-log stops into completed vs partial sessions.

    Each stop is paired with the most recent prior start; adjacent pairs whose
    gap is within ``merge_gap_seconds`` are merged into one play span (stream
    restarts split a single session into several log entries). A span whose
    wall-clock duration reaches ``threshold`` of the runtime is classified as
    a completed watch; a stop with a start in the window below that threshold
    is a clearly-partial session (a brief re-open or abandoned play) and is
    reported separately so callers can bound counts by it.

    Sessions with unknown runtime cannot be judged: all stops are reported as
    completion-like (the permissive fallback). Stops without a start in the
    window (the session began before retention) are neither complete nor
    partial — there is no evidence to judge them.

    Returns ``(completion_like_stop_ids, partial_stop_ids)``.
    """
    stops = [entry for entry in entries if entry.kind == "stop"]
    if not stops:
        return set(), set()
    if runtime_ticks is None or runtime_ticks <= 0:
        return {entry.entry_id for entry in stops}, set()
    runtime_seconds = runtime_ticks / 10_000_000

    pairs: list[tuple[float | None, float, int]] = []
    open_start: float | None = None
    for entry in sorted(entries, key=lambda e: e.occurred_at):
        occurred = _epoch_seconds(entry.occurred_at)
        if entry.kind == "start":
            open_start = occurred
        else:
            pairs.append((open_start, occurred, entry.entry_id))
    pairs.sort(key=lambda pair: pair[1])

    span_indexes: list[int] = []
    span_starts: list[float | None] = []
    span_ends: list[float] = []
    current_span = -1
    for start_t, stop_t, _entry_id in pairs:
        if (
            current_span >= 0
            and start_t is not None
            and start_t - span_ends[current_span] <= merge_gap_seconds
        ):
            span_ends[current_span] = stop_t
        else:
            current_span += 1
            span_starts.append(start_t)
            span_ends.append(stop_t)
        span_indexes.append(current_span)

    complete: set[int] = set()
    partial: set[int] = set()
    for (start_t, _stop_t, entry_id), span_index in zip(pairs, span_indexes):
        span_start = span_starts[span_index]
        if span_start is None:
            continue
        duration = span_ends[span_index] - span_start
        if duration >= threshold * runtime_seconds:
            complete.add(entry_id)
        else:
            partial.add(entry_id)
    return complete, partial


@dataclass(frozen=True)
class _Fetched:
    played: dict[str, LibraryItemData]
    resume: dict[str, LibraryItemData]
    entries: list[ActivityEntry]
    max_scanned_id: int | None
    activity_log_enabled: bool


class SyncService:
    def __init__(
        self, database: Database, jellyfin: JellyfinClient, settings: Settings
    ):
        self.database = database
        self.jellyfin = jellyfin
        self.settings = settings
        self._lock = asyncio.Lock()

    async def run(self) -> bool:
        async with self._lock:
            started_at = utc_now()
            await self._mark_started(started_at)
            try:
                fetched = await self._fetch()
            except asyncio.CancelledError:
                await asyncio.shield(self._mark_failed("Sync interrupted"))
                raise
            except Exception as error:  # noqa: BLE001 - keep scheduled sync failures contained
                await self._mark_failed(self._safe_error(error))
                logger.error("Jellyfin sync failed: %s", self._safe_error(error))
                return False

            imported = 0
            try:
                async with self.database.session() as session, session.begin():
                    imported = await self._apply(session, fetched, started_at)
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

            finished_at = utc_now()
            try:
                async with self.database.session() as session, session.begin():
                    state = await session.get(SyncState, 1)
                    if state is None:
                        state = SyncState(id=1)
                        session.add(state)
                    state.status = "success"
                    state.last_finished_at = finished_at
                    state.last_success_at = finished_at
                    state.last_error = None
                    state.items_seen = len(fetched.played) + len(fetched.resume)
                    state.items_imported = imported
                    state.activity_cursor = fetched.max_scanned_id
                    state.activity_log_enabled = fetched.activity_log_enabled
            except Exception:  # noqa: BLE001 - status persistence must not mask the sync
                logger.error("Unable to persist successful Jellyfin sync status")

            logger.info(
                "Jellyfin sync completed: %d played items, %d resume items, "
                "%d new events, activity log enabled=%s",
                len(fetched.played),
                len(fetched.resume),
                imported,
                fetched.activity_log_enabled,
            )
            return True

    async def _fetch(self) -> _Fetched:
        played, _seen = await self.jellyfin.fetch_played_items()
        resume = await self.jellyfin.fetch_resume_items()
        activity_log_enabled = self.settings.JELLYFIN_ACTIVITY_LOG_ENABLED
        entries: list[ActivityEntry] = []
        max_scanned_id: int | None = None
        if activity_log_enabled:
            try:
                # Jellyfin prunes the log at ActivityLogRetentionDays, so the
                # whole log is effectively the session window; the sync
                # classifies sessions from it on every run.
                entries, max_scanned_id = await self.jellyfin.fetch_activity_window()
            except JellyfinForbiddenError:
                activity_log_enabled = False
                logger.info(
                    "Jellyfin activity log unavailable (non-admin API key); "
                    "continuing with per-item play state only"
                )
        return _Fetched(
            played={item.jellyfin_id: item for item in played},
            resume={item.jellyfin_id: item for item in resume},
            entries=entries,
            max_scanned_id=max_scanned_id,
            activity_log_enabled=activity_log_enabled,
        )

    async def _apply(
        self, session: AsyncSession, fetched: _Fetched, started_at: str
    ) -> int:
        # The configured user id may be compact hex or hyphenated; the activity
        # log always reports canonical hyphenated form.
        user_id = str(UUID(self.settings.JELLYFIN_USER_ID))
        threshold = self.settings.PLAYBACK_COMPLETION_THRESHOLD

        rows = {
            row.jellyfin_id: row
            for row in (await session.exec(select(WatchedItem))).all()
        }
        snapshots = {
            row.jellyfin_id: row
            for row in (await session.exec(select(UserDataSnapshot))).all()
        }
        existing_keys: set[str] = set()
        userdata_by_item: dict[str, list[WatchEvent]] = defaultdict(list)
        activity_by_item: dict[str, list[WatchEvent]] = defaultdict(list)
        for event in (await session.exec(select(WatchEvent))).all():
            existing_keys.add(event.event_key)
            if event.source == "activity_log":
                activity_by_item[event.jellyfin_id].append(event)
            else:
                userdata_by_item[event.jellyfin_id].append(event)

        own_entries = [entry for entry in fetched.entries if entry.user_id == user_id]
        stops_by_item: dict[str, list[ActivityEntry]] = defaultdict(list)
        for entry in own_entries:
            if entry.kind == "stop":
                stops_by_item[entry.item_id].append(entry)
        latest_start_by_item: dict[str, str] = {}
        for entry in own_entries:
            if entry.kind != "start":
                continue
            current = latest_start_by_item.get(entry.item_id)
            if current is None or entry.occurred_at > current:
                latest_start_by_item[entry.item_id] = entry.occurred_at

        # Resolve log items absent from the library lists so their stops become
        # exact history (or are dropped when the item is gone entirely). Items
        # whose stops were already accepted need no re-resolution.
        await self._resolve_log_items(
            session, rows, stops_by_item, fetched, existing_keys
        )

        imported = 0
        source_ids = (
            set(fetched.played)
            | set(fetched.resume)
            | set(stops_by_item)
            | set(latest_start_by_item)
        )
        for item_id in source_ids:
            played_item = fetched.played.get(item_id)
            resume_item = fetched.resume.get(item_id)
            effective = played_item or resume_item

            in_progress_movie = self._in_progress_movie(played_item, resume_item)
            row = rows.get(item_id)

            # 1. Classify this item's sessions: which stops plausibly completed
            #    and which are clearly-partial (brief re-opens, abandoned plays).
            completion_ids, partial_ids = _completion_like_stop_ids(
                [entry for entry in own_entries if entry.item_id == item_id],
                effective.runtime_ticks if effective is not None else None,
                threshold,
            )

            # 2. Completion gate. ``Played==true`` (or a played percentage past
            #    the threshold) is Jellyfin's definitive completion signal.
            #    ``PlayCount`` only counts playback *starts*, so it is never a
            #    completion fact. An item the server no longer flags played and
            #    that is not in progress was either never completed or was just
            #    marked unplayed: drop ALL of its events (approximate aggregates
            #    and previously recorded exact stops) so the row disappears.
            #    Rewatches in progress keep their prior history.
            completed = effective is not None and self._completed_gate(
                effective, threshold
            )
            in_progress = resume_item is not None or (
                played_item is not None and (played_item.position_ticks or 0) > 0
            )
            if effective is not None and not effective.played and not in_progress:
                for event in userdata_by_item.get(item_id, []) + activity_by_item.get(
                    item_id, []
                ):
                    await session.delete(event)
                userdata_by_item[item_id] = []
                activity_by_item[item_id] = []

            # 3. Estimated completed-watch count. PlayCount counts starts; the
            #    sessions we can see in the window that are clearly partial are
            #    therefore not completions and are subtracted. The Played flag
            #    guarantees at least one completion; with unknown runtime no
            #    session can be judged, so the server count is kept as-is.
            play_count = effective.play_count if effective is not None else 0
            if effective is not None and completed:
                if effective.runtime_ticks is not None and effective.runtime_ticks > 0:
                    completions = max(1, play_count - len(partial_ids))
                else:
                    completions = play_count
            else:
                completions = 0

            # Prune previously recorded exact events that are not completion-like
            # (recorded by earlier logic that trusted PlayCount).
            kept_activity: list[WatchEvent] = []
            if effective is not None and completed:
                for event in activity_by_item.get(item_id, []):
                    if event.log_entry_id in completion_ids:
                        kept_activity.append(event)
                    else:
                        await session.delete(event)
            else:
                kept_activity = list(activity_by_item.get(item_id, []))
            activity_by_item[item_id] = kept_activity

            # 4. Plan this run's new exact events from completion-like stops,
            #    capped so the total never exceeds the completions estimate.
            pending_events: list[WatchEvent] = []
            planned_stops = stops_by_item.get(item_id, [])
            if (
                planned_stops
                and not in_progress_movie
                and self._completed_gate(effective, threshold)
            ):
                planned_stops = [
                    entry for entry in planned_stops if entry.entry_id in completion_ids
                ]
                planned_stops.sort(key=lambda e: e.occurred_at, reverse=True)
                if effective is None:
                    # Deleted items cannot be bounded by session classification;
                    # their stop history stays as it was.
                    allowed_new = len(planned_stops)
                else:
                    allowed_new = max(completions - len(kept_activity), 0)
                for entry in planned_stops[:allowed_new]:
                    key = _activity_event_key(entry.entry_id)
                    if key in existing_keys:
                        continue
                    pending_events.append(
                        WatchEvent(
                            event_key=key,
                            jellyfin_id=item_id,
                            source="activity_log",
                            event_date=entry.occurred_at,
                            log_entry_id=entry.entry_id,
                        )
                    )

            # 5. Aggregate residual: fill the remaining completions estimate
            #    with approximate plays dated at the newest playback date.
            total_exact = len(kept_activity) + len(pending_events)
            desired_userdata = max(completions - total_exact, 0) if completed else 0
            existing_userdata = userdata_by_item.get(item_id, [])
            regenerate_userdata = effective is not None and desired_userdata != len(
                existing_userdata
            )
            if effective is not None and completed and regenerate_userdata:
                for event in existing_userdata:
                    await session.delete(event)
                existing_userdata = []
                userdata_by_item[item_id] = []
                if regenerate_userdata and desired_userdata:
                    if effective.last_played_date is None:
                        logger.warning(
                            "Played item without a last-played date cannot be "
                            "aggregated: %s",
                            item_id,
                        )
                    for index in range(desired_userdata):
                        if effective.last_played_date is None:
                            break
                        pending_events.append(
                            WatchEvent(
                                event_key=_userdata_event_key(
                                    user_id,
                                    item_id,
                                    effective.last_played_date,
                                    index,
                                ),
                                jellyfin_id=item_id,
                                source="userdata",
                                event_date=effective.last_played_date,
                                log_entry_id=None,
                            )
                        )

            # 4. Derive the final current state and ensure the watched row exists.
            started_movie = (
                effective is not None
                and effective.kind == "movie"
                and in_progress_movie
                and not (
                    effective.played
                    and effective.played_percentage is not None
                    and effective.played_percentage >= threshold
                )
            )
            manual_marker = row.manually_watched_at if row is not None else None
            all_events = (
                list(activity_by_item.get(item_id, []))
                + list(userdata_by_item.get(item_id, []))
                + pending_events
            )
            event_dates = [e.event_date for e in all_events]
            event_dates.sort()
            event_count = len(event_dates)

            if row is None:
                if effective is None or (event_count == 0 and not started_movie):
                    continue
                if started_movie:
                    new_started_at: str | None = (
                        latest_start_by_item.get(item_id) or started_at
                    )
                else:
                    new_started_at = None
                row = WatchedItem(
                    jellyfin_id=item_id,
                    kind=effective.kind,
                    title=effective.title,
                    series_title=effective.series_title,
                    series_id=effective.series_id,
                    season_number=effective.season_number,
                    episode_number=effective.episode_number,
                    poster_tag=effective.poster_tag,
                    series_poster_tag=effective.series_poster_tag,
                    imdb_id=effective.imdb_id,
                    watch_status="started" if started_movie else "watched",
                    started_at=new_started_at,
                    watched_at=event_dates[-1] if event_dates else None,
                    manually_watched_at=None,
                    play_count=event_count,
                    is_favorite=False,
                    updated_at=started_at,
                )
                session.add(row)
                await session.flush()
                imported += 1
            else:
                if effective is not None:
                    row.title = effective.title
                    row.series_title = effective.series_title
                    row.series_id = effective.series_id
                    row.season_number = effective.season_number
                    row.episode_number = effective.episode_number
                    row.poster_tag = effective.poster_tag
                    row.series_poster_tag = effective.series_poster_tag
                    row.imdb_id = effective.imdb_id
                row.watched_at = event_dates[-1] if event_dates else None
                row.play_count = event_count
                if started_movie:
                    start_entry = latest_start_by_item.get(item_id)
                    if start_entry is not None:
                        row.started_at = start_entry
                    elif row.started_at is None:
                        row.started_at = started_at
                    row.watch_status = "started"
                else:
                    row.started_at = None
                    row.watch_status = "watched"
                if row.manually_watched_at is not None:
                    row.watch_status = "watched"
                    row.started_at = None
                row.updated_at = started_at

            # A row without history, start, or manual marker is dropped.
            if (
                event_count == 0
                and not started_movie
                and manual_marker is None
                and effective is not None
            ):
                await session.delete(row)
                rows.pop(item_id, None)
                continue

            # Persist the planned events now that the row is guaranteed.
            for event in pending_events:
                session.add(event)
                imported += 1

            # 5. Snapshot the Tier 1 state for reconciliation/observability.
            if effective is not None:
                snapshot = snapshots.get(item_id)
                if snapshot is None:
                    snapshot = UserDataSnapshot(
                        jellyfin_id=item_id, synced_at=started_at
                    )
                    snapshots[item_id] = snapshot
                    session.add(snapshot)
                snapshot.play_count = effective.play_count
                snapshot.last_played_date = effective.last_played_date
                snapshot.played = effective.played
                snapshot.played_percentage = effective.played_percentage
                snapshot.position_ticks = effective.position_ticks
                snapshot.synced_at = started_at

        # Unplayed reconciliation for rows with no session evidence in this run
        # (their last playback predates the activity-log window): the previous
        # sync only knows the item is no longer in the current library state
        # once it is resolved.
        for item_id, row in list(rows.items()):
            if item_id in source_ids:
                continue
            if row.manually_watched_at is not None or row.watch_status != "watched":
                continue
            if activity_by_item.get(item_id):
                continue  # exact activity facts keep the row
            if row.play_count == 0:
                continue
            try:
                resolved = await self.jellyfin.fetch_item_details(item_id)
            except Exception:  # noqa: BLE001 - resolution must never fail the sync
                resolved = None
            if resolved is None:
                continue  # deleted; approximative history is kept
            if resolved.played or (
                resolved.played_percentage is not None
                and resolved.played_percentage >= threshold
            ):
                continue  # still completed: no change
            # Marked unplayed in Jellyfin: drop the approximate history and row.
            for event in userdata_by_item.get(item_id, []):
                await session.delete(event)
            userdata_by_item[item_id] = []
            await session.delete(row)
            rows.pop(item_id, None)
            logger.info(
                "Removed previously tracked item that Jellyfin no longer marks as played: %s",
                item_id,
            )

        return imported

    async def _resolve_log_items(
        self,
        session: AsyncSession,
        rows: dict[str, WatchedItem],
        stops_by_item: dict[str, list[ActivityEntry]],
        fetched: _Fetched,
        existing_keys: set[str],
    ) -> None:
        """Resolve activity-log items absent from the current library lists.

        An item that resolves (including one that was just marked unplayed in
        Jellyfin) gets merged into the played set with its current UserData so
        the completion gate and unplayed reconciliation apply. A deleted item
        with an existing tracked row keeps its exact history (effective stays
        None, gate passes). A deleted item with no tracked row cannot be
        represented (its kind is unknown) and its stops are dropped. Items
        whose stops were already accepted on an earlier run are not re-resolved.
        """
        missing = [
            item_id
            for item_id, entries in stops_by_item.items()
            if entries
            and item_id not in fetched.played
            and item_id not in fetched.resume
        ]
        for item_id in missing:
            try:
                resolved = await self.jellyfin.fetch_item_details(item_id)
            except Exception:  # noqa: BLE001 - resolution must never fail the sync
                resolved = None
            if resolved is None:
                if item_id in rows:
                    # Deleted but tracked: the stop history stays as facts.
                    continue
                stops_by_item.pop(item_id, None)
                logger.info(
                    "Skipping history for unresolvable activity-log item %s",
                    item_id,
                )
            else:
                fetched.played[item_id] = resolved

    @staticmethod
    def _in_progress_movie(
        played_item: LibraryItemData | None,
        resume_item: LibraryItemData | None,
    ) -> bool:
        effective = played_item or resume_item
        if effective is None or effective.kind != "movie":
            return False
        if resume_item is not None:
            return True
        return played_item is not None and (played_item.position_ticks or 0) > 0

    @staticmethod
    def _completed_gate(effective: LibraryItemData | None, threshold: float) -> bool:
        if effective is None:
            # Deleted items cannot be verified; their recent stops count as
            # exact history.
            return True
        if effective.played:
            return True
        return (
            effective.played_percentage is not None
            and effective.played_percentage >= threshold
        )

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
    def _safe_error(error: Exception) -> str:
        if isinstance(error, JellyfinError):
            message = str(error)
        else:
            return "Jellyfin sync failed"
        if isinstance(error, JellyfinResponseError):
            return "Jellyfin returned an invalid response"
        if isinstance(error, JellyfinTransportError):
            status_match = re.fullmatch(
                r"Jellyfin returned HTTP ([1-5][0-9]{2})(?: after [0-9]+ attempts)?",
                message,
            )
            if status_match:
                status = int(status_match.group(1))
                if status == 401:
                    return (
                        "Jellyfin returned HTTP 401; check the API key and user access"
                    )
                if status == 403:
                    return "Jellyfin returned HTTP 403; check API key permissions"
                if status == 404:
                    return "Jellyfin returned HTTP 404; check the configured server URL"
                if status == 429:
                    return "Jellyfin rate limited the sync (HTTP 429)"
                if status >= 500:
                    return f"Jellyfin server error (HTTP {status}) after retries"
                return f"Jellyfin returned HTTP {status}"
            if re.fullmatch(
                r"Jellyfin could not be reached after [0-9]+ attempts", message
            ):
                return (
                    "Jellyfin could not be reached after retries; check the server URL "
                    "and network availability"
                )
            return "Jellyfin request failed"
        return "Jellyfin request failed"

    async def wait_until_idle(self) -> None:
        async with self._lock:
            return
