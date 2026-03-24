"""Client Ollama — supporte les modes OpenAI (/v1/chat/completions) et Anthropic (/v1/messages)."""
from __future__ import annotations

import json
import logging
from typing import AsyncGenerator

import httpx

from app.services.settings import get_setting_value
from app.services.debug import debug_buffer

logger = logging.getLogger(__name__)


class OllamaClient:
    """Client HTTP async vers Ollama avec support des deux modes API."""

    def __init__(self, base_url: str | None = None, api_mode: str | None = None):
        # Les valeurs seront résolues de façon async dans _resolve()
        self._base_url_override = base_url
        self._api_mode_override = api_mode
        self._resolved = False
        self.base_url = ""
        self.api_mode = ""

    async def _resolve(self) -> None:
        """Résout les valeurs depuis la BDD si non override."""
        if self._resolved:
            return
        if self._base_url_override:
            self.base_url = self._base_url_override.rstrip("/")
        else:
            self.base_url = (await get_setting_value("ollama_base_url", "http://localhost:11434")).rstrip("/")
        if self._api_mode_override:
            self.api_mode = self._api_mode_override
        else:
            self.api_mode = await get_setting_value("ollama_api_mode", "openai")
        self._resolved = True
        debug_buffer.log("DEBUG", "ollama", f"Client résolu: {self.base_url} mode={self.api_mode}")

    # ------------------------------------------------------------------
    # Liste des modèles disponibles
    # ------------------------------------------------------------------

    async def list_models(self) -> list[dict]:
        """Retourne la liste des modèles Ollama disponibles."""
        await self._resolve()
        debug_buffer.log("DEBUG", "ollama", f"GET {self.base_url}/api/tags")
        try:
            async with httpx.AsyncClient(timeout=10) as client:
                resp = await client.get(f"{self.base_url}/api/tags")
                resp.raise_for_status()
                data = resp.json()
                models = data.get("models", [])
                debug_buffer.log("INFO", "ollama", f"Modèles trouvés: {len(models)}")
                return models
        except Exception as e:
            debug_buffer.log("ERROR", "ollama", f"Échec list_models: {e}", url=f"{self.base_url}/api/tags", error=str(e))
            raise

    # ------------------------------------------------------------------
    # Streaming chat — mode OpenAI
    # ------------------------------------------------------------------

    async def _stream_openai(
        self,
        model: str,
        messages: list[dict],
        tools: list[dict] | None,
        system: str | None,
        temperature: float,
        max_tokens: int,
    ) -> AsyncGenerator[str, None]:
        """Stream via /v1/chat/completions (compatible OpenAI)."""
        await self._resolve()
        timeout = await get_setting_value("ollama_timeout", 300)
        payload: dict = {
            "model": model,
            "messages": [{"role": "system", "content": system}, *messages] if system else messages,
            "stream": True,
            "temperature": temperature,
            "max_tokens": max_tokens,
        }
        if tools:
            payload["tools"] = tools

        url = f"{self.base_url}/v1/chat/completions"
        async with httpx.AsyncClient(timeout=timeout) as client:
            async with client.stream("POST", url, json=payload) as resp:
                resp.raise_for_status()
                async for line in resp.aiter_lines():
                    if line.startswith("data: "):
                        data = line[6:]
                        if data == "[DONE]":
                            break
                        yield data

    # ------------------------------------------------------------------
    # Streaming chat — mode Anthropic
    # ------------------------------------------------------------------

    async def _stream_anthropic(
        self,
        model: str,
        messages: list[dict],
        tools: list[dict] | None,
        system: str | None,
        temperature: float,
        max_tokens: int,
    ) -> AsyncGenerator[str, None]:
        """Stream via /v1/messages (compatible Anthropic)."""
        await self._resolve()
        timeout = await get_setting_value("ollama_timeout", 300)
        payload: dict = {
            "model": model,
            "messages": messages,
            "stream": True,
            "max_tokens": max_tokens,
        }
        if system:
            payload["system"] = system
        if tools:
            payload["tools"] = [_openai_tool_to_anthropic(t) for t in tools]

        url = f"{self.base_url}/v1/messages"
        headers = {
            "anthropic-version": "2023-06-01",
            "x-api-key": "ollama",
            "content-type": "application/json",
        }
        async with httpx.AsyncClient(timeout=timeout) as client:
            async with client.stream("POST", url, json=payload, headers=headers) as resp:
                resp.raise_for_status()
                async for line in resp.aiter_lines():
                    if line.startswith("data: "):
                        data = line[6:]
                        # Convertit les events Anthropic → format OpenAI normalisé
                        normalized = _anthropic_event_to_openai(data)
                        if normalized:
                            yield normalized

    # ------------------------------------------------------------------
    # Interface publique unifiée
    # ------------------------------------------------------------------

    async def stream_chat(
        self,
        model: str,
        messages: list[dict],
        tools: list[dict] | None = None,
        system: str | None = None,
        temperature: float = 0.7,
        max_tokens: int = 4096,
    ) -> AsyncGenerator[str, None]:
        """Stream unifié — délègue selon api_mode."""
        await self._resolve()
        debug_buffer.log("INFO", "ollama",
                         f"stream_chat model={model} mode={self.api_mode} msgs={len(messages)} tools={len(tools or [])}")
        if self.api_mode == "anthropic":
            async for chunk in self._stream_anthropic(
                model, messages, tools, system, temperature, max_tokens
            ):
                yield chunk
        else:
            async for chunk in self._stream_openai(
                model, messages, tools, system, temperature, max_tokens
            ):
                yield chunk

    async def complete(
        self,
        model: str,
        messages: list[dict],
        tools: list[dict] | None = None,
        system: str | None = None,
        temperature: float = 0.7,
        max_tokens: int = 4096,
    ) -> dict:
        """Requête non-streaming — retourne la réponse complète."""
        chunks: list[str] = []
        async for chunk in self.stream_chat(model, messages, tools, system, temperature, max_tokens):
            chunks.append(chunk)
        return _merge_chunks(chunks)


# ------------------------------------------------------------------
# Helpers de conversion Anthropic ↔ OpenAI
# ------------------------------------------------------------------

def _openai_tool_to_anthropic(tool: dict) -> dict:
    """Convertit une définition d'outil OpenAI en format Anthropic."""
    fn = tool.get("function", tool)
    return {
        "name": fn["name"],
        "description": fn.get("description", ""),
        "input_schema": fn.get("parameters", {"type": "object", "properties": {}}),
    }


def _anthropic_event_to_openai(data: str) -> str | None:
    """Normalise un event SSE Anthropic vers le format OpenAI delta."""
    try:
        evt = json.loads(data)
    except (json.JSONDecodeError, ValueError) as e:
        debug_buffer.log("WARN", "ollama", f"Event SSE Anthropic non-JSON: {e}", raw=data[:200])
        return None

    event_type = evt.get("type")

    if event_type == "content_block_delta":
        delta = evt.get("delta", {})
        text = delta.get("text", "")
        if text:
            return json.dumps({
                "choices": [{"delta": {"content": text}, "finish_reason": None}]
            })

    if event_type == "message_stop":
        return json.dumps({
            "choices": [{"delta": {}, "finish_reason": "stop"}]
        })

    if event_type == "content_block_start":
        block = evt.get("content_block", {})
        if block.get("type") == "tool_use":
            return json.dumps({
                "choices": [{
                    "delta": {
                        "tool_calls": [{
                            "id": block.get("id", ""),
                            "type": "function",
                            "function": {"name": block.get("name", ""), "arguments": ""},
                        }]
                    },
                    "finish_reason": None,
                }]
            })

    return None


def _merge_chunks(chunks: list[str]) -> dict:
    """Assemble les chunks SSE en une réponse complète."""
    content = ""
    tool_calls: list[dict] = []
    finish_reason = "stop"

    for raw in chunks:
        try:
            data = json.loads(raw)
        except (json.JSONDecodeError, ValueError) as e:
            debug_buffer.log("WARN", "ollama", f"Chunk non-JSON dans merge: {e}", raw=raw[:200])
            continue
        for choice in data.get("choices", []):
            delta = choice.get("delta", {})
            if "content" in delta and delta["content"]:
                content += delta["content"]
            if "tool_calls" in delta:
                for tc in delta["tool_calls"]:
                    _merge_tool_call(tool_calls, tc)
            if choice.get("finish_reason"):
                finish_reason = choice["finish_reason"]

    return {
        "content": content,
        "tool_calls": tool_calls or None,
        "finish_reason": finish_reason,
    }


def _merge_tool_call(tool_calls: list[dict], tc: dict) -> None:
    """Fusionne un fragment de tool call dans la liste existante."""
    idx = tc.get("index", len(tool_calls))
    while len(tool_calls) <= idx:
        tool_calls.append({"id": "", "type": "function", "function": {"name": "", "arguments": ""}})
    entry = tool_calls[idx]
    entry["id"] = entry["id"] or tc.get("id", "")
    fn = tc.get("function", {})
    entry["function"]["name"] = entry["function"]["name"] or fn.get("name", "")
    entry["function"]["arguments"] += fn.get("arguments", "")
