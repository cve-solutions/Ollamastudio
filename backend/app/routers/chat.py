"""Router chat — SSE streaming avec boucle agentique et persistance des messages."""
from __future__ import annotations

import json
import logging
from typing import Annotated

from fastapi import APIRouter, Depends, HTTPException
from fastapi.responses import StreamingResponse
from pydantic import BaseModel, Field
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.agents.loop import agentic_loop
from app.config import settings
from app.database import Message, Session, get_db

logger = logging.getLogger(__name__)
router = APIRouter()


class ChatRequest(BaseModel):
    session_id: int
    content: str
    model: str = settings.ollama_default_model
    ollama_base_url: str | None = None
    ollama_api_mode: str | None = None
    enabled_tools: list[str] | None = None   # None = tous, [] = aucun
    temperature: float = Field(default=0.7, ge=0.0, le=2.0)
    max_tokens: int = Field(default=4096, ge=64, le=128000)


async def _get_session_or_404(session_id: int, db: AsyncSession) -> Session:
    result = await db.execute(select(Session).where(Session.id == session_id))
    session = result.scalar_one_or_none()
    if session is None:
        raise HTTPException(status_code=404, detail="Session introuvable")
    return session


async def _get_history(session_id: int, db: AsyncSession) -> tuple[list[dict], str | None]:
    """Récupère l'historique des messages et le system prompt de la skill."""
    result = await db.execute(
        select(Message)
        .where(Message.session_id == session_id)
        .order_by(Message.id)
        .limit(settings.max_messages_per_session)
    )
    msgs = result.scalars().all()

    history: list[dict] = []
    for m in msgs:
        entry: dict = {"role": m.role, "content": m.content}
        if m.tool_calls:
            entry["tool_calls"] = m.tool_calls
        history.append(entry)

    return history, None  # system prompt géré via skills router


@router.post("/stream")
async def chat_stream(
    req: ChatRequest,
    db: Annotated[AsyncSession, Depends(get_db)],
) -> StreamingResponse:
    """Stream SSE d'un échange agentique complet."""
    session = await _get_session_or_404(req.session_id, db)
    history, _ = await _get_history(req.session_id, db)

    # Charge la skill si associée
    system_prompt: str | None = None
    if session.skill_id:
        from app.routers.skills import load_skill
        skill = await load_skill(session.skill_id)
        if skill:
            system_prompt = skill.get("system_prompt")

    # Persiste le message utilisateur
    user_msg = Message(
        session_id=req.session_id,
        role="user",
        content=req.content,
    )
    db.add(user_msg)
    await db.commit()

    history.append({"role": "user", "content": req.content})

    # Mise à jour du titre de session au 1er message
    if len(history) == 1:
        title = req.content[:60].replace("\n", " ")
        session.title = title
        session.model = req.model
        db.add(session)
        await db.commit()

    async def event_stream():
        assistant_text = ""
        tool_calls_for_db: list[dict] = []

        try:
            async for event in agentic_loop(
                model=req.model,
                messages=history,
                system=system_prompt,
                enabled_tools=req.enabled_tools,
                ollama_base_url=req.ollama_base_url,
                ollama_api_mode=req.ollama_api_mode,
                temperature=req.temperature,
                max_tokens=req.max_tokens,
            ):
                yield f"data: {json.dumps(event, ensure_ascii=False)}\n\n"

                if event["type"] == "text":
                    assistant_text += event["content"]
                elif event["type"] == "tool_start":
                    tool_calls_for_db.append({
                        "name": event["name"],
                        "args": event["args"],
                        "call_id": event["call_id"],
                    })

        except Exception as e:
            logger.exception("Erreur stream chat: %s", e)
            yield f"data: {json.dumps({'type': 'error', 'message': str(e)})}\n\n"

        finally:
            # Persiste la réponse assistant
            if assistant_text or tool_calls_for_db:
                async with db.begin_nested():
                    asst_msg = Message(
                        session_id=req.session_id,
                        role="assistant",
                        content=assistant_text,
                        tool_calls=tool_calls_for_db or None,
                    )
                    db.add(asst_msg)
                await db.commit()

    return StreamingResponse(
        event_stream(),
        media_type="text/event-stream",
        headers={
            "Cache-Control": "no-cache",
            "X-Accel-Buffering": "no",
        },
    )
