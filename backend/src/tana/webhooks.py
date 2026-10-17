from __future__ import annotations

import asyncio
import hashlib
import json
import logging
from dataclasses import dataclass
from datetime import UTC, datetime
from typing import Any
from uuid import UUID

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


def _get(payload: dict[str, Any], field: str) -> Any:
    field_folded = field.casefold()
    for key, value in payload.items():
        if isinstance(key, str) and key.casefold() == field_folded:
            return value
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
    payload: Any, configured_user_id: str, completion_threshold: float = 1.0
) -> CompletionNotice | None:
    if not isinstance(payload, dict):
        raise WebhookPayloadError("Webhook JSON must be an object")

    notification_type = _get(payload, "NotificationType")
    if not isinstance(notification_type, str) or not notification_type:
        raise WebhookPayloadError("Missing or invalid NotificationType")
    if notification_type != "PlaybackStop":
        return None

    played_to_completion = _get(payload, "PlayedToCompletion")
    if not isinstance(played_to_completion, bool):
        raise WebhookPayloadError("Missing or invalid PlayedToCompletion")

    user_id = _canonical_uuid(_get(payload, "UserId"), "UserId")
    if user_id != _expected_user_id(configured_user_id):
        return None

    item_type = _get(payload, "ItemType")
    if not isinstance(item_type, str) or not item_type:
        raise WebhookPayloadError("Missing or invalid ItemType")
    kind = SUPPORTED_TYPES.get(item_type)
    if kind is None:
        return None

    jellyfin_id = _canonical_uuid(_get(payload, "ItemId"), "ItemId")
    title = _get(payload, "Name")
    if not isinstance(title, str) or not title.strip():
        raise WebhookPayloadError("Missing or invalid Name")
    completed_at = _canonical_timestamp(_get(payload, "UtcTimestamp"))

    session_id_value = _get(payload, "SessionId")
    session_id = session_id_value.strip() if isinstance(session_id_value, str) else None
    if not session_id:
        session_id = None
    position = _optional_int(
        _get(payload, "PlaybackPositionTicks"), "PlaybackPositionTicks"
    )
    run_time = _optional_int(_get(payload, "RunTimeTicks"), "RunTimeTicks")
    completed = played_to_completion or (
        position is not None
        and run_time is not None
        and run_time > 0
        and position / run_time >= completion_threshold
    )
    if not completed:
        return None
    series_title = _get(payload, "SeriesName")
    if series_title is not None and not isinstance(series_title, str):
        raise WebhookPayloadError("Invalid SeriesName")
    series_id = _optional_uuid(_get(payload, "SeriesId")) if kind == "tv" else None
    season_number = (
        _optional_int(_get(payload, "SeasonNumber"), "SeasonNumber")
        if kind == "tv"
        else None
    )
    episode_number = (
        _optional_int(_get(payload, "EpisodeNumber"), "EpisodeNumber")
        if kind == "tv"
        else None
    )
    provider_imdb = _get(payload, "Provider_imdb") if kind == "movie" else None
    imdb_id = (
        provider_imdb.strip()
        if isinstance(provider_imdb, str)
        and IMDB_ID_PATTERN.fullmatch(provider_imdb.strip())
        else None
    )

    canonical_fields = (
        user_id,
        jellyfin_id,
        completed_at,
        session_id or "",
        str(position) if position is not None else "",
        item_type,
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

    notification_type = _get(payload, "NotificationType")
    if not isinstance(notification_type, str) or not notification_type:
        raise WebhookPayloadError("Missing or invalid NotificationType")
    if notification_type != "PlaybackStart":
        return None

    user_id = _canonical_uuid(_get(payload, "UserId"), "UserId")
    if user_id != _expected_user_id(configured_user_id):
        return None

    item_type = _get(payload, "ItemType")
    if not isinstance(item_type, str) or not item_type:
        raise WebhookPayloadError("Missing or invalid ItemType")
    if item_type != "Movie":
        return None

    jellyfin_id = _canonical_uuid(_get(payload, "ItemId"), "ItemId")
    title = _get(payload, "Name")
    if not isinstance(title, str) or not title.strip():
        raise WebhookPayloadError("Missing or invalid Name")
    started_at = _canonical_timestamp(_get(payload, "UtcTimestamp"))
    return StartNotice(
        jellyfin_id=jellyfin_id,
        title=title.strip(),
        started_at=started_at,
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

    async def handle_json(self, body: bytes) -> bool:
        try:
            payload = json.loads(body)
        except (UnicodeDecodeError, json.JSONDecodeError):
            raise WebhookPayloadError("Malformed webhook JSON") from None
        notice = parse_completion_notice(
            payload, self.configured_user_id, self.completion_threshold
        )
        if notice is not None:
            async with self._lock:
                return await self._record_completion(notice)
        start = parse_start_notice(payload, self.configured_user_id)
        if start is not None:
            async with self._lock:
                return await self._record_start(start)
        return False

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

    async def _record_start(self, notice: StartNotice) -> bool:
        async with self.database.session() as session:
            existing_item = await session.get(WatchedItem, notice.jellyfin_id)
        if existing_item is not None and (
            existing_item.kind != "movie"
            or not self._start_accepted(existing_item, notice.started_at)
        ):
            return False

        enrichment = None
        if existing_item is None and self.jellyfin is not None:
            enrichment = await self._best_effort_enrich(notice.jellyfin_id)

        async with self.database.session() as session, session.begin():
            item = await session.get(WatchedItem, notice.jellyfin_id)
            if item is not None and (
                item.kind != "movie"
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
