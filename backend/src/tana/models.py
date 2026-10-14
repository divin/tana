from __future__ import annotations

from sqlalchemy import CheckConstraint, Index, String, text
from sqlmodel import Field, SQLModel


class WatchedItem(SQLModel, table=True):
    __tablename__ = "watched_items"
    __table_args__ = (
        CheckConstraint("kind IN ('movie', 'tv')", name="ck_watched_items_kind"),
        CheckConstraint("play_count >= 1", name="ck_watched_items_play_count"),
        CheckConstraint(
            "(kind = 'movie' AND series_title IS NULL AND series_id IS NULL "
            "AND season_number IS NULL AND episode_number IS NULL "
            "AND series_poster_tag IS NULL) OR "
            "(kind = 'tv' AND series_title IS NOT NULL AND poster_tag IS NULL "
            "AND imdb_id IS NULL)",
            name="ck_watched_items_kind_fields",
        ),
        Index(
            "ix_watched_items_watched_title",
            text("watched_at DESC"),
            text("title COLLATE NOCASE"),
        ),
        Index(
            "ix_watched_items_kind_watched",
            "kind",
            text("watched_at DESC"),
        ),
    )

    jellyfin_id: str = Field(primary_key=True, sa_type=String)
    kind: str = Field(sa_type=String)
    title: str = Field(sa_type=String)
    series_title: str | None = Field(default=None, sa_type=String)
    series_id: str | None = Field(default=None, sa_type=String)
    season_number: int | None = Field(default=None)
    episode_number: int | None = Field(default=None)
    watched_at: str = Field(sa_type=String)
    play_count: int = Field(default=1)
    poster_tag: str | None = Field(default=None, sa_type=String)
    series_poster_tag: str | None = Field(default=None, sa_type=String)
    imdb_id: str | None = Field(default=None, sa_type=String)
    updated_at: str = Field(sa_type=String)


class CompletedPlayback(SQLModel, table=True):
    __tablename__ = "completed_playbacks"

    event_key: str = Field(primary_key=True, sa_type=String)
    jellyfin_id: str = Field(foreign_key="watched_items.jellyfin_id", sa_type=String)
    session_id: str | None = Field(default=None, sa_type=String)
    completed_at: str = Field(sa_type=String)


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
    )

    id: int = Field(default=1, primary_key=True)
    last_started_at: str | None = Field(default=None, sa_type=String)
    last_finished_at: str | None = Field(default=None, sa_type=String)
    last_success_at: str | None = Field(default=None, sa_type=String)
    last_error: str | None = Field(default=None, sa_type=String)
    items_seen: int = Field(default=0)
    items_imported: int = Field(default=0)
    status: str = Field(default="never", sa_type=String)
