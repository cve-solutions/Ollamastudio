"""Router sessions — CRUD sessions de chat et historique messages."""
from __future__ import annotations

from typing import Annotated

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel
from sqlalchemy import delete, desc, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.database import Message, Session, get_db

router = APIRouter()


class SessionCreate(BaseModel):
    title: str = "Nouvelle session"
    model: str = "qwen3-coder"
    skill_id: str | None = None


class SessionUpdate(BaseModel):
    title: str | None = None
    skill_id: str | None = None


class SessionOut(BaseModel):
    id: int
    title: str
    model: str
    skill_id: str | None
    created_at: str
    updated_at: str

    model_config = {"from_attributes": True}


class MessageOut(BaseModel):
    id: int
    session_id: int
    role: str
    content: str
    tool_calls: list | None
    tool_results: list | None
    created_at: str

    model_config = {"from_attributes": True}


def _session_out(s: Session) -> dict:
    return {
        "id": s.id,
        "title": s.title,
        "model": s.model,
        "skill_id": s.skill_id,
        "created_at": s.created_at.isoformat(),
        "updated_at": s.updated_at.isoformat(),
    }


@router.get("/")
async def list_sessions(
    db: Annotated[AsyncSession, Depends(get_db)],
    limit: int = 50,
    offset: int = 0,
) -> list[dict]:
    result = await db.execute(
        select(Session).order_by(desc(Session.updated_at)).limit(limit).offset(offset)
    )
    return [_session_out(s) for s in result.scalars().all()]


@router.post("/", status_code=201)
async def create_session(
    body: SessionCreate,
    db: Annotated[AsyncSession, Depends(get_db)],
) -> dict:
    session = Session(title=body.title, model=body.model, skill_id=body.skill_id)
    db.add(session)
    await db.commit()
    await db.refresh(session)
    return _session_out(session)


@router.get("/{session_id}")
async def get_session(
    session_id: int,
    db: Annotated[AsyncSession, Depends(get_db)],
) -> dict:
    result = await db.execute(select(Session).where(Session.id == session_id))
    session = result.scalar_one_or_none()
    if session is None:
        raise HTTPException(status_code=404, detail="Session introuvable")
    return _session_out(session)


@router.patch("/{session_id}")
async def update_session(
    session_id: int,
    body: SessionUpdate,
    db: Annotated[AsyncSession, Depends(get_db)],
) -> dict:
    result = await db.execute(select(Session).where(Session.id == session_id))
    session = result.scalar_one_or_none()
    if session is None:
        raise HTTPException(status_code=404, detail="Session introuvable")
    if body.title is not None:
        session.title = body.title
    if body.skill_id is not None:
        session.skill_id = body.skill_id
    db.add(session)
    await db.commit()
    await db.refresh(session)
    return _session_out(session)


@router.delete("/{session_id}", status_code=204)
async def delete_session(
    session_id: int,
    db: Annotated[AsyncSession, Depends(get_db)],
) -> None:
    await db.execute(delete(Message).where(Message.session_id == session_id))
    await db.execute(delete(Session).where(Session.id == session_id))
    await db.commit()


@router.get("/{session_id}/messages")
async def get_messages(
    session_id: int,
    db: Annotated[AsyncSession, Depends(get_db)],
) -> list[dict]:
    result = await db.execute(
        select(Message)
        .where(Message.session_id == session_id)
        .order_by(Message.id)
    )
    messages = result.scalars().all()
    return [
        {
            "id": m.id,
            "session_id": m.session_id,
            "role": m.role,
            "content": m.content,
            "tool_calls": m.tool_calls,
            "tool_results": m.tool_results,
            "created_at": m.created_at.isoformat(),
        }
        for m in messages
    ]
