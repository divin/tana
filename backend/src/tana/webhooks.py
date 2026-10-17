from __future__ import annotations

import asyncio
import hashlib
import json
import logging
import time
from dataclasses import dataclass
from datetime import UTC, datetime
from typing import Any
from uuid import UUID

from sqlalchemy import text
from sqlmodel import select

from tana.database import Database
from tana.jellyfin import IMDB_ID_PATTERN, JellyfinClient, LibraryItemData
from tana.models import CompletedPlayback, WatchedItem
from tana.time_utils import format_utc

logger = logging.getLogger(__name__)
MAX_WEBHOOK_BYTES = 1024 * 1024
SUPPORTED_TYPES = {"Movie": "movie", "Episode": "tv"}


class WebhookPayloadError(ValueError):
    """A malformed webhook; its payload must not be included in errors."""


@dataclass(frozen=True)
class CompletionNotice:
    event_key: str
    jellyfin_id: str
    kind: str
    title: str
    completed_at: str
    user_id: str
    session_id: str | None
    playback_position_ticks: int | None
    series_title: str | None
    series_id: str | None
    season_number: int | None
    episode_number: int | None
    imdb_id: str | None


@dataclass(frozen=True)
class StartNotice:
    jellyfin_id: str
    title: str
    started_at: str
    user_id: str


@dataclass(frozen=True)
class ItemStateNotice:
    jellyfin_id: str
    kind: str
    title: str
    occurred_at: str
    user_id: str


def _get(payload: dict[str, Any], field: str) -> Any:
    field_folded = field.casefold()
    for key, value in payload.items():
        if isinstance(key, str) and key.casefold() == field_folded:
            return value
    return None


def _event_label(payload: dict[str, Any]) -> str:
    for field in ("NotificationType", "Event"):
        event = _get(payload, field)
        if isinstance(event, str) and event.strip():
            normalized = "".join(
                character for character in event.casefold() if character.isalnum()
            )
            return normalized[:48] or "empty"
    return "missing"


def _event_kind(payload: dict[str, Any]) -> str | None:
    for field in ("NotificationType", "Event"):
        event = _get(payload, field)
        if not isinstance(event, str):
            continue
        normalized = "".join(
            character for character in event.casefold() if character.isalnum()
        )
        if normalized in {"playbackstart", "play", "playbackstarted"}:
            return "start"
        if normalized in {"playbackstop", "stop", "playbackstopped"}:
            return "stop"
        if normalized == "playbackprogress":
            return "progress"
        if normalized in {"markplayed", "itemmarkedplayed"}:
            return "mark_played"
        if normalized in {"markunplayed", "itemmarkedunplayed"}:
            return "mark_unplayed"
    return None


def _item(payload: dict[str, Any]) -> dict[str, Any]:
    value = _get(payload, "Item")
    return value if isinstance(value, dict) else {}


def _nested(payload: dict[str, Any], group: str, field: str) -> Any:
    value = _get(payload, group)
    return _get(value, field) if isinstance(value, dict) else None


def _played_signal(payload: dict[str, Any]) -> bool | None:
    """Resolve Jellyfin's played state, preferring any affirmative signal."""
    event_kind = _event_kind(payload)
    if event_kind == "mark_played":
        return True
    if event_kind == "mark_unplayed":
        return False
    signals = (
        _get(payload, "PlayedToCompletion"),
        _nested(_item(payload), "UserData", "Played"),
    )
    if any(signal is not None and not isinstance(signal, bool) for signal in signals):
        raise WebhookPayloadError("Invalid Jellyfin played state")
    if any(signal is True for signal in signals):
        return True
    if any(signal is False for signal in signals):
        return False
    return None


def _payload_value(payload: dict[str, Any], field: str) -> Any:
    value = _get(payload, field)
    if value is not None:
        return value
    item = _item(payload)
    value = _get(item, field)
    if value is not None:
        return value
    if field == "UserId":
        return _nested(payload, "User", "Id")
    if field == "ItemId":
        return _get(item, "Id")
    if field == "ItemType":
        return _get(item, "Type")
    if field == "Name":
        return _get(item, "Name")
    if field == "SeasonNumber":
        return _get(item, "ParentIndexNumber")
    if field == "EpisodeNumber":
        return _get(item, "IndexNumber")
    if field == "UtcTimestamp":
        return _get(payload, "Timestamp") or _get(payload, "Date")
    if field == "PlayedToCompletion":
        return _nested(item, "UserData", "Played")
    if field == "PlaybackPositionTicks":
        return _nested(item, "UserData", "PlaybackPositionTicks")
    if field == "RunTimeTicks":
        return _get(item, "RunTimeTicks")
    if field == "Provider_imdb":
        return _nested(item, "ProviderIds", "Imdb")
    return None


def _canonical_uuid(value: Any, field_name: str) -> str:
    if not isinstance(value, str):
        raise WebhookPayloadError(f"Missing or invalid {field_name}")
    try:
        return str(UUID(value.strip()))
    except (ValueError, AttributeError):
        raise WebhookPayloadError(f"Missing or invalid {field_name}") from None


def _optional_uuid(value: Any) -> str | None:
    if value is None or value == "":
        return None
    try:
        return str(UUID(str(value).strip()))
    except (ValueError, AttributeError):
        return None


def _canonical_timestamp(value: Any) -> str:
    if not isinstance(value, str) or not value.strip():
        raise WebhookPayloadError("Missing or invalid UtcTimestamp")
    raw = value.strip()
    if raw.endswith("Z"):
        raw = raw[:-1] + "+00:00"
    try:
        parsed = datetime.fromisoformat(raw)
    except ValueError:
        raise WebhookPayloadError("Missing or invalid UtcTimestamp") from None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=UTC)
    return format_utc(parsed)


def _event_timestamp(payload: dict[str, Any], timestamp_value: Any = None) -> str:
    value = timestamp_value or _payload_value(payload, "UtcTimestamp")
    if not value and isinstance(_get(payload, "Event"), str):
        value = format_utc(datetime.now(UTC))
    return _canonical_timestamp(value)


def _optional_int(value: Any, field_name: str) -> int | None:
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise WebhookPayloadError(f"Invalid {field_name}")
    return value


def _expected_user_id(configured_user_id: str) -> str:
    try:
        return str(UUID(configured_user_id))
    except (ValueError, AttributeError):
        raise WebhookPayloadError("Configured Jellyfin user ID is invalid") from None


def parse_completion_notice(
    payload: Any,
    configured_user_id: str,
    completion_threshold: float = 1.0,
    prior_progress: tuple[int, int] | None = None,
) -> CompletionNotice | None:
    if not isinstance(payload, dict):
        raise WebhookPayloadError("Webhook JSON must be an object")

    event_kind = _event_kind(payload)
    if event_kind not in {"stop", "mark_played"}:
        return None

    played_to_completion = _played_signal(payload) is True

    user_id = _canonical_uuid(_payload_value(payload, "UserId"), "UserId")
    if user_id != _expected_user_id(configured_user_id):
        return None

    item_type = _payload_value(payload, "ItemType")
    if not isinstance(item_type, str) or not item_type:
        raise WebhookPayloadError("Missing or invalid ItemType")
    kind = SUPPORTED_TYPES.get(item_type.strip().title())
    if kind is None:
        return None

    jellyfin_id = _canonical_uuid(_payload_value(payload, "ItemId"), "ItemId")
    title = _payload_value(payload, "Name")
    if not isinstance(title, str) or not title.strip():
        raise WebhookPayloadError("Missing or invalid Name")
    timestamp_value = _payload_value(payload, "UtcTimestamp")
    if not timestamp_value and event_kind in {"stop", "mark_played"}:
        timestamp_value = _nested(_item(payload), "UserData", "LastPlayedDate")
    if not timestamp_value and event_kind == "mark_played":
        timestamp_value = format_utc(datetime.now(UTC))
    completed_at = _event_timestamp(payload, timestamp_value)

    session_id_value = _payload_value(payload, "SessionId")
    session_id = session_id_value.strip() if isinstance(session_id_value, str) else None
    if not session_id:
        session_id = None
    position = _optional_int(
        _payload_value(payload, "PlaybackPositionTicks"), "PlaybackPositionTicks"
    )
    run_time = _optional_int(_payload_value(payload, "RunTimeTicks"), "RunTimeTicks")
    if prior_progress is not None:
        prior_position, prior_runtime = prior_progress
        current_ratio = (
            position / run_time
            if position is not None and run_time is not None and run_time > 0
            else -1.0
        )
        prior_ratio = prior_position / prior_runtime if prior_runtime > 0 else -1.0
        if prior_ratio > current_ratio:
            position = prior_position
        if run_time is None or run_time <= 0:
            run_time = prior_runtime
    completed = played_to_completion or (
        position is not None
        and run_time is not None
        and run_time > 0
        and position / run_time >= completion_threshold
    )
    if not completed:
        return None
    series_title = _payload_value(payload, "SeriesName")
    if series_title is not None and not isinstance(series_title, str):
        raise WebhookPayloadError("Invalid SeriesName")
    series_id = (
        _optional_uuid(_payload_value(payload, "SeriesId")) if kind == "tv" else None
    )
    season_number = (
        _optional_int(_payload_value(payload, "SeasonNumber"), "SeasonNumber")
        if kind == "tv"
        else None
    )
    episode_number = (
        _optional_int(_payload_value(payload, "EpisodeNumber"), "EpisodeNumber")
        if kind == "tv"
        else None
    )
    provider_imdb = (
        _payload_value(payload, "Provider_imdb") if kind == "movie" else None
    )
    imdb_id = (
        provider_imdb.strip()
        if isinstance(provider_imdb, str)
        and IMDB_ID_PATTERN.fullmatch(provider_imdb.strip())
        else None
    )

    canonical_fields = (
        (
            user_id,
            jellyfin_id,
            "mark_played",
            item_type,
            completed_at if timestamp_value else "",
        )
        if event_kind == "mark_played"
        else (
            user_id,
            jellyfin_id,
            completed_at if timestamp_value or not session_id else "",
            session_id or "",
            str(position) if position is not None else "",
            item_type,
        )
    )
    event_key = hashlib.sha256("\0".join(canonical_fields).encode()).hexdigest()
    return CompletionNotice(
        event_key=event_key,
        jellyfin_id=jellyfin_id,
        kind=kind,
        title=title.strip(),
        completed_at=completed_at,
        user_id=user_id,
        session_id=session_id,
        playback_position_ticks=position,
        series_title=(
            (
                series_title.strip()
                if series_title and series_title.strip()
                else title.strip()
            )
            if kind == "tv"
            else None
        ),
        series_id=series_id,
        season_number=season_number,
        episode_number=episode_number,
        imdb_id=imdb_id,
    )


def parse_start_notice(payload: Any, configured_user_id: str) -> StartNotice | None:
    if not isinstance(payload, dict):
        raise WebhookPayloadError("Webhook JSON must be an object")
    event_kind = _event_kind(payload)
    if event_kind not in {"start", "stop"}:
        return None

    user_id = _canonical_uuid(_payload_value(payload, "UserId"), "UserId")
    if user_id != _expected_user_id(configured_user_id):
        return None

    item_type = _payload_value(payload, "ItemType")
    if not isinstance(item_type, str) or not item_type:
        raise WebhookPayloadError("Missing or invalid ItemType")
    if item_type.strip().title() != "Movie":
        return None

    jellyfin_id = _canonical_uuid(_payload_value(payload, "ItemId"), "ItemId")
    title = _payload_value(payload, "Name")
    if not isinstance(title, str) or not title.strip():
        raise WebhookPayloadError("Missing or invalid Name")
    started_at = _event_timestamp(payload, _payload_value(payload, "UtcTimestamp"))
    return StartNotice(
        jellyfin_id=jellyfin_id,
        title=title.strip(),
        started_at=started_at,
        user_id=user_id,
    )


def parse_item_state_notice(
    payload: Any, configured_user_id: str
) -> ItemStateNotice | None:
    if not isinstance(payload, dict):
        raise WebhookPayloadError("Webhook JSON must be an object")
    event_kind = _event_kind(payload)
    if event_kind not in {"mark_played", "mark_unplayed"}:
        return None

    user_id = _canonical_uuid(_payload_value(payload, "UserId"), "UserId")
    if user_id != _expected_user_id(configured_user_id):
        return None
    item_type = _payload_value(payload, "ItemType")
    if not isinstance(item_type, str) or not item_type:
        raise WebhookPayloadError("Missing or invalid ItemType")
    kind = SUPPORTED_TYPES.get(item_type.strip().title())
    if kind is None:
        return None
    jellyfin_id = _canonical_uuid(_payload_value(payload, "ItemId"), "ItemId")
    title = _payload_value(payload, "Name")
    if event_kind == "mark_played" and (
        not isinstance(title, str) or not title.strip()
    ):
        raise WebhookPayloadError("Missing or invalid Name")
    marked_at = (
        _event_timestamp(
            payload,
            _payload_value(payload, "UtcTimestamp") or format_utc(datetime.now(UTC)),
        )
        if event_kind == "mark_played"
        else format_utc(datetime.now(UTC))
    )
    return ItemStateNotice(
        jellyfin_id=jellyfin_id,
        kind=kind,
        title=title.strip() if isinstance(title, str) else "",
        occurred_at=marked_at,
        user_id=user_id,
    )


class WebhookService:
    def __init__(
        self,
        database: Database,
        configured_user_id: str,
        completion_threshold: float = 1.0,
        jellyfin: JellyfinClient | None = None,
    ) -> None:
        self.database = database
        self.configured_user_id = configured_user_id
        self.completion_threshold = completion_threshold
        self.jellyfin = jellyfin
        self._lock = asyncio.Lock()
        self._session_progress: dict[tuple[str, str, str], tuple[int, int, float]] = {}

    async def handle_json(self, body: bytes) -> bool:
        try:
            payload = json.loads(body)
        except (UnicodeDecodeError, json.JSONDecodeError):
            raise WebhookPayloadError("Malformed webhook JSON") from None
        event_kind = _event_kind(payload) if isinstance(payload, dict) else None
        if event_kind == "progress":
            async with self._lock:
                self._remember_progress(payload)
            return False
        if event_kind in {"mark_played", "mark_unplayed"}:
            state_notice = parse_item_state_notice(payload, self.configured_user_id)
            if state_notice is None:
                self._log_ignored(payload)
                return False
            if event_kind == "mark_unplayed":
                async with self._lock:
                    return await self._record_unplayed(state_notice)
            if state_notice.kind == "movie":
                async with self._lock:
                    return await self._record_manual_watched(state_notice)
            completion = parse_completion_notice(
                payload, self.configured_user_id, self.completion_threshold
            )
            if completion is not None:
                async with self._lock:
                    return await self._record_completion(completion)
            return False

        prior_progress = None
        if event_kind == "stop":
            async with self._lock:
                prior_progress = self._take_progress(payload)
        notice = parse_completion_notice(
            payload,
            self.configured_user_id,
            self.completion_threshold,
            prior_progress,
        )
        if notice is not None:
            async with self._lock:
                recorded = await self._record_completion(notice)
            if not recorded:
                logger.info("Ignored Jellyfin webhook: reason=duplicate_or_conflict")
            return recorded
        start = parse_start_notice(payload, self.configured_user_id)
        if start is not None:
            if event_kind == "start" and _played_signal(payload) is True:
                state_notice = ItemStateNotice(
                    jellyfin_id=start.jellyfin_id,
                    kind="movie",
                    title=start.title,
                    occurred_at=start.started_at,
                    user_id=start.user_id,
                )
                async with self._lock:
                    recorded = await self._record_manual_watched(state_notice)
            else:
                async with self._lock:
                    recorded = await self._record_start(
                        start, only_if_new=event_kind == "stop"
                    )
            if not recorded:
                logger.info("Ignored Jellyfin webhook: reason=duplicate_or_stale_state")
            return recorded
        self._log_ignored(payload, prior_progress)
        return False

    def _progress_key(self, payload: Any) -> tuple[str, str, str] | None:
        if not isinstance(payload, dict):
            return None
        try:
            user_id = _canonical_uuid(_payload_value(payload, "UserId"), "UserId")
            if user_id != _expected_user_id(self.configured_user_id):
                return None
            item_type = _payload_value(payload, "ItemType")
            if (
                not isinstance(item_type, str)
                or item_type.strip().title() not in SUPPORTED_TYPES
            ):
                return None
            jellyfin_id = _canonical_uuid(_payload_value(payload, "ItemId"), "ItemId")
        except WebhookPayloadError:
            return None
        session_id = _payload_value(payload, "SessionId")
        if not isinstance(session_id, str) or not session_id.strip():
            return None
        return (user_id, jellyfin_id, session_id.strip())

    def _remember_progress(self, payload: dict[str, Any]) -> None:
        session_id = _payload_value(payload, "SessionId")
        item_id = _payload_value(payload, "ItemId")
        user_value = _payload_value(payload, "UserId")
        try:
            user_matches = _canonical_uuid(user_value, "UserId") == _expected_user_id(
                self.configured_user_id
            )
        except WebhookPayloadError:
            user_matches = False
        position_value = _payload_value(payload, "PlaybackPositionTicks")
        runtime_value = _payload_value(payload, "RunTimeTicks")
        key = self._progress_key(payload)
        if key is None:
            logger.debug(
                "Jellyfin progress not cached: user_matches=%s item_id_present=%s "
                "session_id_present=%s",
                user_matches,
                isinstance(item_id, str) and bool(item_id.strip()),
                isinstance(session_id, str) and bool(session_id.strip()),
            )
            return
        try:
            position = _optional_int(position_value, "PlaybackPositionTicks")
            runtime = _optional_int(runtime_value, "RunTimeTicks")
        except WebhookPayloadError:
            logger.debug("Jellyfin progress not cached: reason=invalid_progress_fields")
            return
        if position is None or runtime is None or runtime == 0:
            logger.debug(
                "Jellyfin progress not cached: reason=missing_progress_fields "
                "position_present=%s runtime_present=%s",
                position is not None,
                runtime is not None and runtime > 0,
            )
            return

        now = time.monotonic()
        for expired_key, (_, _, updated_at) in tuple(self._session_progress.items()):
            if now - updated_at > 6 * 60 * 60:
                del self._session_progress[expired_key]
        previous = self._session_progress.pop(key, None)
        if previous is not None:
            previous_position, previous_runtime, _ = previous
            if (
                previous_runtime > 0
                and previous_position / previous_runtime > position / runtime
            ):
                position, runtime = previous_position, previous_runtime
        elif len(self._session_progress) >= 1024:
            self._session_progress.pop(next(iter(self._session_progress)))
        self._session_progress[key] = (position, runtime, now)
        logger.debug(
            "Cached Jellyfin session progress: ratio=%.3f",
            position / runtime,
        )

    def _take_progress(self, payload: dict[str, Any]) -> tuple[int, int] | None:
        key = self._progress_key(payload)
        if key is None:
            session_id = _payload_value(payload, "SessionId")
            logger.debug(
                "Jellyfin stop progress lookup unavailable: item_id_present=%s "
                "session_id_present=%s",
                isinstance(_payload_value(payload, "ItemId"), str),
                isinstance(session_id, str) and bool(session_id.strip()),
            )
            return None
        snapshot = self._session_progress.pop(key, None)
        if snapshot is None:
            same_item_sessions = sum(
                cached_key[:2] == key[:2] for cached_key in self._session_progress
            )
            logger.debug(
                "Jellyfin stop progress lookup missed: same_item_sessions=%s",
                same_item_sessions,
            )
            return None
        position, runtime, updated_at = snapshot
        if time.monotonic() - updated_at > 6 * 60 * 60:
            logger.debug("Jellyfin stop progress lookup expired")
            return None
        logger.debug(
            "Matched Jellyfin session progress: ratio=%.3f",
            position / runtime,
        )
        return position, runtime

    def _log_ignored(
        self, payload: Any, prior_progress: tuple[int, int] | None = None
    ) -> None:
        if not isinstance(payload, dict):
            return
        event_kind = _event_kind(payload)
        user_value = _payload_value(payload, "UserId")
        try:
            user_matches = (
                str(UUID(user_value.strip()))
                == _expected_user_id(self.configured_user_id)
                if isinstance(user_value, str)
                else False
            )
        except (ValueError, AttributeError):
            user_matches = False
        item_type = _payload_value(payload, "ItemType")
        raw_item_kind = (
            item_type.strip().casefold() if isinstance(item_type, str) else ""
        )
        item_kind = (
            raw_item_kind
            if raw_item_kind in {"movie", "episode"}
            else "other"
            if raw_item_kind
            else "missing"
        )
        played_status = "n/a"
        progress_ratio = None
        session_progress_ratio = (
            prior_progress[0] / prior_progress[1]
            if prior_progress is not None and prior_progress[1] > 0
            else None
        )
        if event_kind is None:
            reason = "unsupported_event"
        elif not user_matches:
            reason = "user_mismatch"
        elif (
            event_kind == "start"
            and item_kind != "movie"
            or event_kind == "stop"
            and item_kind not in {"movie", "episode"}
        ):
            reason = "unsupported_item_type"
        elif event_kind == "stop":
            played_value = _payload_value(payload, "PlayedToCompletion")
            played = played_value is True
            position = _payload_value(payload, "PlaybackPositionTicks")
            runtime = _payload_value(payload, "RunTimeTicks")
            progress_ratio = (
                position / runtime
                if isinstance(position, int)
                and not isinstance(position, bool)
                and isinstance(runtime, int)
                and not isinstance(runtime, bool)
                and runtime > 0
                else None
            )
            if prior_progress is not None:
                prior_position, prior_runtime = prior_progress
                prior_ratio = (
                    prior_position / prior_runtime if prior_runtime > 0 else None
                )
                if prior_ratio is not None and (
                    progress_ratio is None or prior_ratio > progress_ratio
                ):
                    progress_ratio = prior_ratio
            reached_threshold = (
                progress_ratio is not None
                and progress_ratio >= self.completion_threshold
            )
            played_status = (
                "true"
                if played_value is True
                else "false"
                if played_value is False
                else "missing"
            )
            reason = (
                "incomplete_playback"
                if not (played or reached_threshold)
                else "filtered_event"
            )
        else:
            reason = "filtered_event"
        logger.info(
            "Ignored Jellyfin webhook: event=%s event_name=%s reason=%s "
            "item_type=%s user_matches=%s played_to_completion=%s "
            "playback_ratio=%s session_progress_ratio=%s "
            "completion_threshold=%.2f",
            event_kind or "unsupported",
            _event_label(payload),
            reason,
            item_kind,
            user_matches,
            played_status,
            f"{progress_ratio:.3f}" if progress_ratio is not None else "unavailable",
            f"{session_progress_ratio:.3f}"
            if session_progress_ratio is not None
            else "unavailable",
            self.completion_threshold,
        )

    async def _record_manual_watched(self, notice: ItemStateNotice) -> bool:
        async with self.database.session() as session:
            existing_item = await session.get(WatchedItem, notice.jellyfin_id)
        if existing_item is not None and (
            existing_item.kind != "movie" or existing_item.watch_status == "watched"
        ):
            return False

        enrichment = None
        if existing_item is None and self.jellyfin is not None:
            enrichment = await self._best_effort_enrich(notice.jellyfin_id)

        async with self.database.session() as session, session.begin():
            item = await session.get(WatchedItem, notice.jellyfin_id)
            if item is not None and (
                item.kind != "movie" or item.watch_status == "watched"
            ):
                return False
            now = format_utc(datetime.now(UTC))
            if item is None:
                item = WatchedItem(
                    jellyfin_id=notice.jellyfin_id,
                    kind="movie",
                    title=notice.title,
                    watch_status="watched",
                    started_at=None,
                    watched_at=None,
                    manually_watched_at=notice.occurred_at,
                    play_count=0,
                    is_favorite=False,
                    poster_tag=(
                        enrichment.poster_tag if enrichment is not None else None
                    ),
                    imdb_id=(enrichment.imdb_id if enrichment is not None else None),
                    updated_at=now,
                )
                session.add(item)
            else:
                item.title = notice.title
                item.watch_status = "watched"
                item.started_at = None
                item.manually_watched_at = notice.occurred_at
                item.updated_at = now
            await session.flush()
        logger.info("Recorded a Jellyfin Movie watched-state correction")
        return True

    async def _record_unplayed(self, notice: ItemStateNotice) -> bool:
        async with self.database.session() as session, session.begin():
            item = await session.get(WatchedItem, notice.jellyfin_id)
            if item is None or item.kind != notice.kind:
                return False
            events = (
                await session.exec(
                    select(CompletedPlayback)
                    .where(CompletedPlayback.jellyfin_id == notice.jellyfin_id)
                    .order_by(
                        text("completed_at DESC"),
                        text("event_key DESC"),
                    )
                )
            ).all()
            if notice.kind == "movie":
                for event in events:
                    await session.delete(event)
                await session.flush()
                await session.delete(item)
            elif events:
                await session.delete(events[0])
                remaining = events[1:]
                await session.flush()
                if remaining:
                    item.play_count = len(remaining)
                    item.watched_at = max(event.completed_at for event in remaining)
                    item.updated_at = format_utc(datetime.now(UTC))
                else:
                    await session.delete(item)
            else:
                await session.delete(item)
            await session.flush()

        for key in tuple(self._session_progress):
            if key[:2] == (notice.user_id, notice.jellyfin_id):
                del self._session_progress[key]
        logger.info("Applied a Jellyfin MarkUnplayed event")
        return True

    async def _record_completion(self, notice: CompletionNotice) -> bool:
        # Resolve idempotency and row existence with a short read before the write
        # transaction, so the best-effort Jellyfin enrichment call never holds the
        # SQLite write lock.
        async with self.database.session() as session:
            deduplicated = (
                await session.get(CompletedPlayback, notice.event_key) is not None
            )
            existing_item = await session.get(WatchedItem, notice.jellyfin_id)
        if deduplicated:
            return False
        if existing_item is not None and existing_item.kind != notice.kind:
            return False

        enrichment = None
        if existing_item is None and self.jellyfin is not None:
            enrichment = await self._best_effort_enrich(notice.jellyfin_id)

        async with self.database.session() as session, session.begin():
            if await session.get(CompletedPlayback, notice.event_key) is not None:
                return False

            item = await session.get(WatchedItem, notice.jellyfin_id)
            if item is not None and item.kind != notice.kind:
                return False

            now = format_utc(datetime.now(UTC))
            if item is None:
                item = WatchedItem(
                    jellyfin_id=notice.jellyfin_id,
                    kind=notice.kind,
                    title=notice.title,
                    series_title=notice.series_title,
                    series_id=notice.series_id,
                    season_number=notice.season_number,
                    episode_number=notice.episode_number,
                    watch_status="watched",
                    started_at=None,
                    watched_at=notice.completed_at,
                    manually_watched_at=None,
                    play_count=1,
                    is_favorite=False,
                    poster_tag=(
                        enrichment.poster_tag
                        if enrichment is not None and notice.kind == "movie"
                        else None
                    ),
                    series_poster_tag=(
                        enrichment.series_poster_tag
                        if enrichment is not None and notice.kind == "tv"
                        else None
                    ),
                    imdb_id=(
                        enrichment.imdb_id
                        if enrichment is not None and enrichment.imdb_id is not None
                        else notice.imdb_id
                    ),
                    updated_at=now,
                )
                session.add(item)
                await session.flush()
            else:
                item.title = notice.title
                if notice.kind == "tv":
                    item.series_title = notice.series_title or item.series_title
                    item.series_id = notice.series_id or item.series_id
                    item.season_number = (
                        notice.season_number
                        if notice.season_number is not None
                        else item.season_number
                    )
                    item.episode_number = (
                        notice.episode_number
                        if notice.episode_number is not None
                        else item.episode_number
                    )
                elif notice.imdb_id:
                    item.imdb_id = notice.imdb_id
                item.watched_at = (
                    max(item.watched_at, notice.completed_at)
                    if item.watched_at is not None
                    else notice.completed_at
                )
                item.play_count += 1
                # A late completion still enters the ledger and count, but cannot
                # cancel a newer start or manual action.
                if (
                    item.started_at is not None
                    and item.started_at <= notice.completed_at
                ):
                    item.started_at = None
                if (
                    item.manually_watched_at is not None
                    and item.manually_watched_at <= notice.completed_at
                ):
                    item.manually_watched_at = None
                item.watch_status = (
                    "started" if item.started_at is not None else "watched"
                )
                item.updated_at = now

            session.add(
                CompletedPlayback(
                    event_key=notice.event_key,
                    jellyfin_id=notice.jellyfin_id,
                    session_id=notice.session_id,
                    completed_at=notice.completed_at,
                )
            )
            await session.flush()
        logger.info("Recorded a confirmed Jellyfin playback completion")
        return True

    @staticmethod
    def _start_accepted(item: WatchedItem, started_at: str) -> bool:
        """A start only supersedes strictly older state timestamps."""
        return not (
            (item.started_at is not None and started_at <= item.started_at)
            or (item.watched_at is not None and started_at <= item.watched_at)
            or (
                item.manually_watched_at is not None
                and started_at <= item.manually_watched_at
            )
        )

    async def _record_start(
        self, notice: StartNotice, *, only_if_new: bool = False
    ) -> bool:
        async with self.database.session() as session:
            existing_item = await session.get(WatchedItem, notice.jellyfin_id)
        if existing_item is not None and (
            only_if_new
            or existing_item.kind != "movie"
            or not self._start_accepted(existing_item, notice.started_at)
        ):
            return False

        enrichment = None
        if existing_item is None and self.jellyfin is not None:
            enrichment = await self._best_effort_enrich(notice.jellyfin_id)

        async with self.database.session() as session, session.begin():
            item = await session.get(WatchedItem, notice.jellyfin_id)
            if item is not None and (
                only_if_new
                or item.kind != "movie"
                or not self._start_accepted(item, notice.started_at)
            ):
                return False

            now = format_utc(datetime.now(UTC))
            if item is None:
                item = WatchedItem(
                    jellyfin_id=notice.jellyfin_id,
                    kind="movie",
                    title=notice.title,
                    watch_status="started",
                    started_at=notice.started_at,
                    watched_at=None,
                    manually_watched_at=None,
                    play_count=0,
                    is_favorite=False,
                    poster_tag=(
                        enrichment.poster_tag if enrichment is not None else None
                    ),
                    imdb_id=(
                        enrichment.imdb_id
                        if enrichment is not None and enrichment.imdb_id is not None
                        else None
                    ),
                    updated_at=now,
                )
                session.add(item)
            else:
                item.title = notice.title
                item.watch_status = "started"
                item.started_at = notice.started_at
                item.manually_watched_at = None
                item.updated_at = now
            await session.flush()
        logger.info("Recorded a Jellyfin Movie playback start")
        return True

    async def _best_effort_enrich(self, jellyfin_id: str) -> LibraryItemData | None:
        """Best-effort current item metadata; failures never fail the webhook."""
        if self.jellyfin is None:
            return None
        try:
            return await self.jellyfin.fetch_item_details(jellyfin_id)
        except Exception:  # noqa: BLE001 - enrichment must never fail the webhook
            logger.warning(
                "Webhook metadata enrichment failed; the scheduled sync will repair it"
            )
            return None
