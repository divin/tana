from __future__ import annotations

from collections.abc import AsyncIterator
from typing import TYPE_CHECKING

from fastapi import Request
from sqlmodel.ext.asyncio.session import AsyncSession

if TYPE_CHECKING:
    from tana.database import Database


async def get_session(request: Request) -> AsyncIterator[AsyncSession]:
    database: Database = request.app.state.database
    async with database.session() as session:
        yield session
