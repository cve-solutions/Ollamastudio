"""Boucle agentique — gère le cycle tool_call → execution → réponse de l'agent."""
from __future__ import annotations

import json
import logging
from typing import AsyncGenerator

from app.agents.tools import TOOL_SCHEMAS, execute_tool
from app.config import settings
from app.services.ollama import OllamaClient

logger = logging.getLogger(__name__)

MAX_ITERATIONS = 20  # Sécurité : évite les boucles infinies


async def agentic_loop(
    model: str,
    messages: list[dict],
    system: str | None,
    enabled_tools: list[str] | None,
    ollama_base_url: str | None = None,
    ollama_api_mode: str | None = None,
    temperature: float = 0.7,
    max_tokens: int = 4096,
) -> AsyncGenerator[dict, None]:
    """Boucle agentique complète avec streaming des événements.

    Yields des dicts typés :
      - {"type": "text", "content": str}            → token de texte
      - {"type": "tool_start", "name": str, "args": dict}
      - {"type": "tool_result", "name": str, "result": dict}
      - {"type": "error", "message": str}
      - {"type": "done", "iterations": int}
    """
    client = OllamaClient(base_url=ollama_base_url, api_mode=ollama_api_mode)

    # Filtre les schémas selon les outils activés
    tools = (
        [t for t in TOOL_SCHEMAS if t["function"]["name"] in enabled_tools]
        if enabled_tools is not None
        else TOOL_SCHEMAS
    )

    history = list(messages)
    iterations = 0

    while iterations < MAX_ITERATIONS:
        iterations += 1
        logger.debug("Itération agent #%d", iterations)

        # Accumule le contenu texte et les tool calls pendant le stream
        text_buffer = ""
        tool_calls_buffer: list[dict] = []
        current_tc_idx = -1

        async for raw_chunk in client.stream_chat(
            model=model,
            messages=history,
            tools=tools if tools else None,
            system=system,
            temperature=temperature,
            max_tokens=max_tokens,
        ):
            try:
                chunk = json.loads(raw_chunk)
            except (json.JSONDecodeError, ValueError):
                continue

            for choice in chunk.get("choices", []):
                delta = choice.get("delta", {})
                finish = choice.get("finish_reason")

                # Texte courant
                if text := delta.get("content"):
                    text_buffer += text
                    yield {"type": "text", "content": text}

                # Tool call en cours de streaming
                if tcs := delta.get("tool_calls"):
                    for tc in tcs:
                        idx = tc.get("index", current_tc_idx + 1)
                        current_tc_idx = idx
                        while len(tool_calls_buffer) <= idx:
                            tool_calls_buffer.append({
                                "id": "", "type": "function",
                                "function": {"name": "", "arguments": ""}
                            })
                        entry = tool_calls_buffer[idx]
                        fn = tc.get("function", {})
                        entry["id"] = entry["id"] or tc.get("id", "")
                        entry["function"]["name"] = entry["function"]["name"] or fn.get("name", "")
                        entry["function"]["arguments"] += fn.get("arguments", "")

        # ── L'assistant a fini de streamer ──────────────────────────
        if not tool_calls_buffer:
            # Réponse texte pure → fin de la boucle
            if text_buffer:
                history.append({"role": "assistant", "content": text_buffer})
            break

        # ── Il y a des tool calls à exécuter ────────────────────────
        assistant_msg: dict = {
            "role": "assistant",
            "content": text_buffer or "",
            "tool_calls": [
                {
                    "id": tc["id"] or f"call_{i}",
                    "type": "function",
                    "function": {
                        "name": tc["function"]["name"],
                        "arguments": tc["function"]["arguments"],
                    },
                }
                for i, tc in enumerate(tool_calls_buffer)
                if tc["function"]["name"]
            ],
        }
        history.append(assistant_msg)

        # Exécute chaque outil
        tool_results_msgs: list[dict] = []
        for tc in assistant_msg["tool_calls"]:
            name = tc["function"]["name"]
            try:
                args = json.loads(tc["function"]["arguments"] or "{}")
            except (json.JSONDecodeError, ValueError):
                args = {}

            yield {"type": "tool_start", "name": name, "args": args, "call_id": tc["id"]}

            result = await execute_tool(name, args)

            yield {"type": "tool_result", "name": name, "result": result, "call_id": tc["id"]}

            tool_results_msgs.append({
                "role": "tool",
                "tool_call_id": tc["id"],
                "name": name,
                "content": json.dumps(result, ensure_ascii=False),
            })

        history.extend(tool_results_msgs)

    else:
        yield {"type": "error", "message": f"Limite d'itérations atteinte ({MAX_ITERATIONS})"}

    yield {"type": "done", "iterations": iterations}
