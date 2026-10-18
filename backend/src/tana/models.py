from __future__ import annotations

from sqlalchemy import CheckConstraint, Index, String, text
from sqlmodel import Field, SQLModel


class WatchedItem(SQLModel, table=True):
    __tablename__ = "watched_items"
    __table_args__ = (
        CheckConstraint("kind IN ('movie', 'tv')", name="ck_watched_items_kind"),
        CheckConstraint(
            "watch_status IN ('started', 'watched')",
            name="ck_watched_items_watch_status",
        ),
        CheckConstraint("is_favorite IN (0, 1)", name="ck_watched_items_is_favorite"),
        CheckConstraint("play_count >= 0", name="ck_watched_items_play_count"),
        CheckConstraint(
            "(play_count = 0 AND watched_at IS NULL) "
            "OR (play_count > 0 AND watched_at IS NOT NULL)",
            name="ck_watched_items_count_date_consistency",
        ),
        CheckConstraint(
            "watch_status != 'started' "
            "OR (started_at IS NOT NULL AND manually_watched_at IS NULL)",
            name="ck_watched_items_started_state",
        ),
        CheckConstraint(
            "kind = 'tv' OR watch_status = 'started' "
            "OR watched_at IS NOT NULL OR manually_watched_at IS NOT NULL",
            name="ck_watched_items_watched_evidence",
        ),
        CheckConstraint(
            "kind != 'tv' OR ("
            "watch_status = 'watched' AND started_at IS NULL "
            "AND manually_watched_at IS NULL AND watched_at IS NOT NULL "
            "AND play_count >= 1)",
            name="ck_watched_items_tv_always_watched",
        ),
        CheckConstraint(
            "manually_watched_at IS NULL OR kind = 'movie'",
            name="ck_watched_items_manual_movie_only",
        ),
        CheckConstraint(
            "(kind = 'movie' AND series_title IS NULL AND series_id IS NULL "
            "AND season_number IS NULL AND episode_number IS NULL "
            "AND series_poster_tag IS NULL) OR "
            "(kind = 'tv' AND series_title IS NOT NULL AND poster_tag IS NULL "
            "AND imdb_id IS NULL)",
            name="ck_watched_items_kind_fields",
        ),
        Index(
            "ix_watched_items_status_started_title",
            "watch_status",
            text("started_at DESC"),
            text("title COLLATE NOCASE"),
        ),
        Index(
            "ix_watched_items_status_watched_title",
            "watch_status",
            text("watched_at DESC"),
            text("title COLLATE NOCASE"),
        ),
    )

    jellyfin_id: str = Field(primary_key=True, sa_type=String)
    kind: str = Field(sa_type=String)
    title: str = Field(sa_type=String)
    watch_status: str = Field(sa_type=String)
    started_at: str | None = Field(default=None, sa_type=String)
    watched_at: str | None = Field(default=None, sa_type=String)
    manually_watched_at: str | None = Field(default=None, sa_type=String)
    play_count: int = Field(default=0)
    is_favorite: bool = Field(default=False)
    series_title: str | None = Field(default=None, sa_type=String)
    series_id: str | None = Field(default=None, sa_type=String)
    season_number: int | None = Field(default=None)
    episode_number: int | None = Field(default=None)
    poster_tag: str | None = Field(default=None, sa_type=String)
    series_poster_tag: str | None = Field(default=None, sa_type=String)
    imdb_id: str | None = Field(default=None, sa_type=String)
    updated_at: str = Field(sa_type=String)


class WatchEvent(SQLModel, table=True):
    """One completed-play event.

    ``source='activity_log'`` rows come from the Jellyfin activity log and carry
    the server's integer log entry id. ``source='userdata'`` rows are the
    aggregate approximation created from the per-item ``PlayCount``/
    ``LastPlayedDate`` diff for plays outside (or without) the activity log.
    ``event_date`` is the UTC completion instant: exact for activity-log rows,
    the item's ``LastPlayedDate`` for aggregate rows.
    """

    __tablename__ = "watch_events"
    __table_args__ = (
        CheckConstraint(
            "source IN ('activity_log', 'userdata')",
            name="ck_watch_events_source",
        ),
        CheckConstraint(
            "(source = 'activity_log' AND log_entry_id IS NOT NULL) OR "
            "(source = 'userdata' AND log_entry_id IS NULL)",
            name="ck_watch_events_log_entry_consistency",
        ),
        CheckConstraint(
            "log_entry_id IS NULL OR log_entry_id >= 0",
            name="ck_watch_events_log_entry_id_nonnegative",
        ),
        Index("ix_watch_events_item", "jellyfin_id"),
        Index("ix_watch_events_date", "event_date"),
    )

    event_key: str = Field(primary_key=True, sa_type=String)
    jellyfin_id: str = Field(foreign_key="watched_items.jellyfin_id", sa_type=String)
    source: str = Field(sa_type=String)
    event_date: str = Field(sa_type=String)
    log_entry_id: int | None = Field(default=None)


class UserDataSnapshot(SQLModel, table=True):
    """Last-seen per-item Jellyfin UserData, backing the Tier 1 diff."""

    __tablename__ = "userdata_snapshots"
    __table_args__ = (
        CheckConstraint("play_count >= 0", name="ck_userdata_snapshots_play_count"),
        CheckConstraint("played IN (0, 1)", name="ck_userdata_snapshots_played"),
        CheckConstraint(
            "position_ticks IS NULL OR position_ticks >= 0",
            name="ck_userdata_snapshots_position_ticks",
        ),
    )

    jellyfin_id: str = Field(primary_key=True, sa_type=String)
    play_count: int = Field(default=0)
    last_played_date: str | None = Field(default=None, sa_type=String)
    played: bool = Field(default=False)
    played_percentage: float | None = Field(default=None)
    position_ticks: int | None = Field(default=None)
    synced_at: str = Field(sa_type=String)


class PosterCache(SQLModel, table=True):
    __tablename__ = "poster_cache"
    __table_args__ = (
        CheckConstraint(
            "source_type IN ('movie', 'series')", name="ck_poster_cache_source_type"
        ),
        CheckConstraint("size_bytes > 0", name="ck_poster_cache_size_positive"),
    )

    source_type: str = Field(primary_key=True, sa_type=String)
    source_id: str = Field(primary_key=True, sa_type=String)
    image_tag: str = Field(sa_type=String)
    cached_at: str = Field(sa_type=String)
    size_bytes: int


class SyncState(SQLModel, table=True):
    __tablename__ = "sync_state"
    __table_args__ = (
        CheckConstraint("id = 1", name="ck_sync_state_singleton_id"),
        CheckConstraint(
            "status IN ('never', 'running', 'success', 'failed')",
            name="ck_sync_state_status",
        ),
        CheckConstraint("items_seen >= 0", name="ck_sync_state_items_seen"),
        CheckConstraint("items_imported >= 0", name="ck_sync_state_items_imported"),
        CheckConstraint(
            "activity_cursor IS NULL OR activity_cursor >= 0",
            name="ck_sync_state_activity_cursor_nonnegative",
        ),
    )

    id: int = Field(default=1, primary_key=True)
    last_started_at: str | None = Field(default=None, sa_type=String)
    last_finished_at: str | None = Field(default=None, sa_type=String)
    last_success_at: str | None = Field(default=None, sa_type=String)
    last_error: str | None = Field(default=None, sa_type=String)
    items_seen: int = Field(default=0)
    items_imported: int = Field(default=0)
    status: str = Field(default="never", sa_type=String)
    activity_cursor: int | None = Field(default=None)
    activity_log_enabled: bool = Field(default=True)
