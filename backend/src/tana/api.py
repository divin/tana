from __future__ import annotations

from datetime import UTC, date, datetime, time, timedelta
from typing import Annotated, Any
from uuid import UUID
from zoneinfo import ZoneInfo

from fastapi import APIRouter, Depends, Header, HTTPException, Query, Request, Response
from pydantic import BaseModel, ConfigDict, StrictBool, model_validator
from sqlalchemy import case, func
from sqlmodel import select
from sqlmodel.ext.asyncio.session import AsyncSession

from tana.app_types import get_session
from tana.models import SyncState, WatchedItem, WatchEvent
from tana.posters import PosterError
from tana.time_utils import format_utc, resolve_local_time

router = APIRouter()

MOVIE_NOT_FOUND = "Media item not found"
MANUAL_CONFLICT = "Manual watched action conflicts with current state"


def _state_timestamp() -> Any:
    """The row's current-state timestamp: started_at for started rows, otherwise
    the later non-null value of watched_at and manually_watched_at. An ordered
    CASE is used because SQLite's scalar max() returns NULL if any argument is."""
    return case(
        (WatchedItem.watch_status == "started", WatchedItem.started_at),
        (WatchedItem.watched_at.is_(None), WatchedItem.manually_watched_at),
        (WatchedItem.manually_watched_at.is_(None), WatchedItem.watched_at),
        (
            WatchedItem.watched_at >= WatchedItem.manually_watched_at,
            WatchedItem.watched_at,
        ),
        else_=WatchedItem.manually_watched_at,
    )


class MediaItemResponse(BaseModel):
    model_config = ConfigDict(extra="forbid")

    id: str
    kind: str
    title: str
    series_title: str | None
    series_id: str | None
    season_number: int | None
    episode_number: int | None
    watch_status: str
    started_at: str | None
    watched_at: str | None
    manually_watched_at: str | None
    manually_watched: bool
    is_favorite: bool
    play_count: int
    rewatch_count: int
    poster_url: str | None
    imdb_url: str | None


class MediaListResponse(BaseModel):
    items: list[MediaItemResponse]
    total: int
    limit: int
    offset: int


class StatsCounts(BaseModel):
    movies: int
    started_movies: int
    tv_episodes: int
    tv_series: int
    total_items: int


class StatsResponse(StatsCounts):
    period: dict[str, str | None]
    recent: dict[str, StatsCounts] | None = None


class ActivityDayResponse(BaseModel):
    date: str
    count: int


class ActivityResponse(BaseModel):
    start_date: str
    end_date: str
    days: list[ActivityDayResponse]


class SyncStatusResponse(BaseModel):
    status: str
    last_started_at: str | None
    last_finished_at: str | None
    last_success_at: str | None
    items_seen: int
    items_imported: int
    last_error: str | None
    activity_cursor: int | None
    activity_log_enabled: bool


class ConfigResponse(BaseModel):
    timezone: str


class MediaMutation(BaseModel):
    model_config = ConfigDict(extra="forbid")

    is_favorite: StrictBool | None = None
    manually_watched: StrictBool | None = None

    @model_validator(mode="before")
    @classmethod
    def reject_null_fields(cls, data: Any) -> Any:
        if not isinstance(data, dict):
            return data
        for key in ("is_favorite", "manually_watched"):
            if key in data and data[key] is None:
                raise ValueError(f"{key} must be a boolean")
        return data


def _date_boundary(value: date, timezone: ZoneInfo) -> str:
    try:
        local = resolve_local_time(datetime.combine(value, time.min), timezone)
    except (OverflowError, ValueError):
        raise HTTPException(
            status_code=422,
            detail="Date cannot be represented in the configured timezone",
        ) from None
    return format_utc(local)


def _date_filters(
    start_date: date | None,
    end_date: date | None,
    timezone: ZoneInfo,
    timestamp_column: Any,
) -> list[Any]:
    if start_date is not None and end_date is not None and start_date > end_date:
        raise HTTPException(
            status_code=422, detail="start_date must be on or before end_date"
        )
    conditions: list[Any] = []
    if start_date is not None:
        conditions.append(timestamp_column >= _date_boundary(start_date, timezone))
    if end_date is not None:
        try:
            day_after = end_date + timedelta(days=1)
        except OverflowError:
            raise HTTPException(
                status_code=422, detail="end_date is out of range"
            ) from None
        conditions.append(timestamp_column < _date_boundary(day_after, timezone))
    return conditions


def _media_item(item: WatchedItem) -> MediaItemResponse:
    return MediaItemResponse(
        id=item.jellyfin_id,
        kind=item.kind,
        title=item.title,
        series_title=item.series_title,
        series_id=item.series_id,
        season_number=item.season_number,
        episode_number=item.episode_number,
        watch_status=item.watch_status,
        started_at=item.started_at,
        watched_at=item.watched_at,
        manually_watched_at=item.manually_watched_at,
        manually_watched=item.manually_watched_at is not None,
        is_favorite=item.is_favorite,
        play_count=item.play_count,
        rewatch_count=max(item.play_count - 1, 0),
        poster_url=(
            f"/api/posters/{item.jellyfin_id}.jpg"
            if item.kind == "movie" and item.poster_tag
            else f"/api/posters/{item.series_id}.jpg"
            if item.kind == "tv" and item.series_id and item.series_poster_tag
            else None
        ),
        imdb_url=(
            f"https://www.imdb.com/title/{item.imdb_id}/"
            if item.kind == "movie" and item.imdb_id
            else None
        ),
    )


@router.get("/api/media", response_model=MediaListResponse)
async def get_media(
    request: Request,
    session: Annotated[AsyncSession, Depends(get_session)],
    kind: Annotated[str, Query(pattern="^(movie|tv|all)$")] = "all",
    status: Annotated[str, Query(pattern="^(all|watched|started)$")] = "all",
    favorite: bool | None = None,
    start_date: date | None = None,
    end_date: date | None = None,
    limit: Annotated[int, Query(ge=1, le=200)] = 50,
    offset: Annotated[int, Query(ge=0)] = 0,
) -> MediaListResponse:
    timezone = ZoneInfo(request.app.state.settings.TZ)
    state_timestamp = _state_timestamp()
    conditions = _date_filters(start_date, end_date, timezone, state_timestamp)
    if kind != "all":
        conditions.append(WatchedItem.kind == kind)
    if status in {"watched", "started"}:
        conditions.append(WatchedItem.watch_status == status)
    if favorite is not None:
        conditions.append(WatchedItem.is_favorite == favorite)

    count_query = select(func.count()).select_from(WatchedItem).where(*conditions)
    total = (await session.exec(count_query)).one()
    query = (
        select(WatchedItem)
        .where(*conditions)
        .order_by(
            state_timestamp.desc(),
            WatchedItem.title.collate("NOCASE").asc(),
            WatchedItem.jellyfin_id.asc(),
        )
        .offset(offset)
        .limit(limit)
    )
    items = (await session.exec(query)).all()
    return MediaListResponse(
        items=[_media_item(item) for item in items],
        total=total,
        limit=limit,
        offset=offset,
    )


async def _stats_for_period(
    session: AsyncSession,
    start_date: date | None,
    end_date: date | None,
    timezone: ZoneInfo,
) -> StatsCounts:
    state_timestamp = _state_timestamp()
    watched_conditions = _date_filters(start_date, end_date, timezone, state_timestamp)
    started_conditions = _date_filters(
        start_date, end_date, timezone, WatchedItem.started_at
    )
    tv_conditions = _date_filters(
        start_date, end_date, timezone, WatchedItem.watched_at
    )

    movies = (
        await session.exec(
            select(func.count())
            .select_from(WatchedItem)
            .where(
                WatchedItem.kind == "movie",
                WatchedItem.watch_status == "watched",
                *watched_conditions,
            )
        )
    ).one()
    started_movies = (
        await session.exec(
            select(func.count())
            .select_from(WatchedItem)
            .where(
                WatchedItem.kind == "movie",
                WatchedItem.watch_status == "started",
                *started_conditions,
            )
        )
    ).one()
    tv_episodes = (
        await session.exec(
            select(func.count())
            .select_from(WatchedItem)
            .where(WatchedItem.kind == "tv", *tv_conditions)
        )
    ).one()
    series_rows = (
        await session.exec(
            select(WatchedItem.series_id, WatchedItem.series_title).where(
                WatchedItem.kind == "tv", *tv_conditions
            )
        )
    ).all()
    series_keys = {
        f"id:{series_id}" if series_id else f"title:{(series_title or '').casefold()}"
        for series_id, series_title in series_rows
    }
    return StatsCounts(
        movies=movies,
        started_movies=started_movies,
        tv_episodes=tv_episodes,
        tv_series=len(series_keys),
        total_items=movies + started_movies + tv_episodes,
    )


@router.get(
    "/api/stats", response_model=StatsResponse, response_model_exclude_unset=True
)
async def get_stats(
    request: Request,
    session: Annotated[AsyncSession, Depends(get_session)],
    start_date: date | None = None,
    end_date: date | None = None,
) -> dict[str, Any]:
    timezone = ZoneInfo(request.app.state.settings.TZ)
    counts = await _stats_for_period(session, start_date, end_date, timezone)
    payload: dict[str, Any] = counts.model_dump()
    payload["period"] = {
        "start_date": start_date.isoformat() if start_date else None,
        "end_date": end_date.isoformat() if end_date else None,
    }
    if start_date is None and end_date is None:
        today = datetime.now(timezone).date()
        recent_7 = await _stats_for_period(
            session, today - timedelta(days=6), today, timezone
        )
        recent_30 = await _stats_for_period(
            session, today - timedelta(days=29), today, timezone
        )
        payload["recent"] = {
            "last_7_days": recent_7.model_dump(),
            "last_30_days": recent_30.model_dump(),
        }
    return payload


@router.get("/api/activity", response_model=ActivityResponse)
async def get_activity(
    request: Request,
    session: Annotated[AsyncSession, Depends(get_session)],
) -> ActivityResponse:
    timezone = ZoneInfo(request.app.state.settings.TZ)
    today = datetime.now(timezone).date()
    start_date = today - timedelta(days=364)
    start_instant = _date_boundary(start_date, timezone)
    end_instant = _date_boundary(today + timedelta(days=1), timezone)
    timestamps = (
        await session.exec(
            select(WatchEvent.event_date).where(
                WatchEvent.event_date >= start_instant,
                WatchEvent.event_date < end_instant,
            )
        )
    ).all()

    counts = {
        (start_date + timedelta(days=day_offset)).isoformat(): 0
        for day_offset in range(365)
    }
    for timestamp in timestamps:
        normalized = timestamp[:-1] + "+00:00" if timestamp.endswith("Z") else timestamp
        watched_at = datetime.fromisoformat(normalized)
        if watched_at.tzinfo is None:
            watched_at = watched_at.replace(tzinfo=ZoneInfo("UTC"))
        local_date = watched_at.astimezone(timezone).date().isoformat()
        if local_date in counts:
            counts[local_date] += 1

    return ActivityResponse(
        start_date=start_date.isoformat(),
        end_date=today.isoformat(),
        days=[
            ActivityDayResponse(date=day, count=count) for day, count in counts.items()
        ],
    )


@router.get("/api/config", response_model=ConfigResponse)
async def get_config(request: Request) -> ConfigResponse:
    return ConfigResponse(timezone=request.app.state.settings.TZ)


@router.get("/api/sync/status", response_model=SyncStatusResponse)
async def get_sync_status(
    session: Annotated[AsyncSession, Depends(get_session)],
) -> SyncStatusResponse:
    state = await session.get(SyncState, 1)
    if state is None:
        return SyncStatusResponse(
            status="never",
            last_started_at=None,
            last_finished_at=None,
            last_success_at=None,
            items_seen=0,
            items_imported=0,
            last_error=None,
            activity_cursor=None,
            activity_log_enabled=True,
        )
    return SyncStatusResponse(
        status=state.status,
        last_started_at=state.last_started_at,
        last_finished_at=state.last_finished_at,
        last_success_at=state.last_success_at,
        items_seen=state.items_seen,
        items_imported=state.items_imported,
        last_error=state.last_error,
        activity_cursor=state.activity_cursor,
        activity_log_enabled=state.activity_log_enabled,
    )


@router.get("/api/posters/{source_id}.jpg")
async def get_poster(
    source_id: str,
    request: Request,
    session: Annotated[AsyncSession, Depends(get_session)],
    if_none_match: Annotated[str | None, Header()] = None,
) -> Response:
    try:
        content, etag = await request.app.state.poster_service.get_poster(
            source_id, session
        )
    except PosterError as error:
        raise HTTPException(
            status_code=error.status_code, detail=error.public_message
        ) from None

    headers = {
        "ETag": etag,
        "Cache-Control": "no-cache",
        "X-Content-Type-Options": "nosniff",
    }
    if if_none_match is not None and any(
        candidate.strip() in {etag, "*"} for candidate in if_none_match.split(",")
    ):
        return Response(status_code=304, headers=headers)
    return Response(content=content, media_type="image/jpeg", headers=headers)


@router.patch("/api/media/{item_id}", response_model=MediaItemResponse)
async def patch_media(
    item_id: str,
    body: MediaMutation,
    request: Request,
    session: Annotated[AsyncSession, Depends(get_session)],
) -> MediaItemResponse:
    if body.is_favorite is None and body.manually_watched is None:
        raise HTTPException(
            status_code=422,
            detail="At least one of is_favorite or manually_watched is required",
        )
    try:
        canonical_id = str(UUID(item_id))
    except (ValueError, AttributeError):
        raise HTTPException(status_code=404, detail=MOVIE_NOT_FOUND) from None

    async with session.begin():
        item = await session.get(WatchedItem, canonical_id)
        if item is None:
            raise HTTPException(status_code=404, detail=MOVIE_NOT_FOUND)

        if body.manually_watched is not None:
            if item.kind != "movie":
                raise HTTPException(
                    status_code=422,
                    detail="Manual watch actions apply only to movies",
                )
            if (
                body.manually_watched
                and item.watch_status == "watched"
                and item.manually_watched_at is None
            ):
                raise HTTPException(status_code=409, detail=MANUAL_CONFLICT)

        now = format_utc(datetime.now(UTC))
        changed = False
        if body.manually_watched is True and item.manually_watched_at is None:
            item.manually_watched_at = now
            item.watch_status = "watched"
            changed = True
        elif body.manually_watched is False and item.manually_watched_at is not None:
            item.manually_watched_at = None
            item.watch_status = "started" if item.started_at is not None else "watched"
            changed = True
        if body.is_favorite is not None and body.is_favorite != item.is_favorite:
            item.is_favorite = body.is_favorite
            changed = True
        if changed:
            item.updated_at = now
        await session.flush()
    return _media_item(item)
