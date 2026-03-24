"""Router MCP — gestion et proxying des serveurs Model Context Protocol."""
from __future__ import annotations

import json
import logging
from pathlib import Path

import aiofiles
import httpx
import yaml
from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from app.config import settings
from app.services.settings import get_setting_value

logger = logging.getLogger(__name__)
router = APIRouter()

MCP_CONFIG_PATH = settings.data_dir / "mcp_servers.yaml"

DEFAULT_MCP_SERVERS: list[dict] = [
    {
        "id": "filesystem",
        "name": "Filesystem MCP",
        "description": "Accès étendu au système de fichiers via MCP",
        "url": "http://localhost:3001",
        "enabled": False,
        "type": "http",
    },
    {
        "id": "git",
        "name": "Git MCP",
        "description": "Opérations Git avancées via MCP",
        "url": "http://localhost:3002",
        "enabled": False,
        "type": "http",
    },
    {
        "id": "datagouv",
        "name": "DataGouv.FR",
        "description": "APIs data.gouv.fr — portail open data français",
        "url": "https://mcp.data.gouv.fr/mcp",
        "enabled": False,
        "type": "http",
    },
]


async def _load_config() -> list[dict]:
    if MCP_CONFIG_PATH.exists():
        async with aiofiles.open(MCP_CONFIG_PATH, encoding="utf-8") as f:
            content = await f.read()
        return yaml.safe_load(content) or []
    return list(DEFAULT_MCP_SERVERS)


async def _save_config(servers: list[dict]) -> None:
    MCP_CONFIG_PATH.parent.mkdir(parents=True, exist_ok=True)
    async with aiofiles.open(MCP_CONFIG_PATH, "w", encoding="utf-8") as f:
        await f.write(yaml.dump(servers, allow_unicode=True, default_flow_style=False))


class McpServer(BaseModel):
    id: str
    name: str
    description: str = ""
    url: str
    enabled: bool = False
    type: str = "http"
    headers: dict[str, str] = {}


@router.get("/servers")
async def list_servers() -> list[dict]:
    return await _load_config()


@router.post("/servers", status_code=201)
async def add_server(body: McpServer) -> dict:
    servers = await _load_config()
    if any(s["id"] == body.id for s in servers):
        raise HTTPException(status_code=409, detail="Serveur MCP déjà configuré")
    servers.append(body.model_dump())
    await _save_config(servers)
    return body.model_dump()


@router.put("/servers/{server_id}")
async def update_server(server_id: str, body: McpServer) -> dict:
    servers = await _load_config()
    for i, s in enumerate(servers):
        if s["id"] == server_id:
            servers[i] = body.model_dump()
            await _save_config(servers)
            return servers[i]
    raise HTTPException(status_code=404, detail="Serveur MCP introuvable")


@router.delete("/servers/{server_id}", status_code=204)
async def delete_server(server_id: str) -> None:
    servers = await _load_config()
    servers = [s for s in servers if s["id"] != server_id]
    await _save_config(servers)


@router.get("/servers/{server_id}/ping")
async def ping_server(server_id: str) -> dict:
    """Teste la connectivité d'un serveur MCP."""
    servers = await _load_config()
    server = next((s for s in servers if s["id"] == server_id), None)
    if server is None:
        raise HTTPException(status_code=404, detail="Serveur MCP introuvable")

    try:
        async with httpx.AsyncClient(timeout=await get_setting_value("mcp_timeout", 30)) as client:
            resp = await client.get(server["url"])
            return {"reachable": True, "status": resp.status_code, "url": server["url"]}
    except Exception as e:
        return {"reachable": False, "error": str(e), "url": server["url"]}


@router.get("/servers/{server_id}/tools")
async def list_server_tools(server_id: str) -> dict:
    """Liste les outils exposés par un serveur MCP."""
    servers = await _load_config()
    server = next((s for s in servers if s["id"] == server_id), None)
    if server is None:
        raise HTTPException(status_code=404, detail="Serveur MCP introuvable")

    try:
        async with httpx.AsyncClient(timeout=await get_setting_value("mcp_timeout", 30)) as client:
            resp = await client.post(
                server["url"],
                json={"jsonrpc": "2.0", "method": "tools/list", "id": 1},
                headers={"Content-Type": "application/json", **server.get("headers", {})},
            )
            data = resp.json()
            return {"server_id": server_id, "tools": data.get("result", {}).get("tools", [])}
    except Exception as e:
        raise HTTPException(status_code=502, detail=f"Erreur MCP: {e}")


@router.post("/servers/{server_id}/call")
async def call_tool(server_id: str, tool_name: str, arguments: dict) -> dict:
    """Appelle un outil sur un serveur MCP."""
    servers = await _load_config()
    server = next((s for s in servers if s["id"] == server_id), None)
    if server is None:
        raise HTTPException(status_code=404, detail="Serveur MCP introuvable")

    try:
        async with httpx.AsyncClient(timeout=await get_setting_value("mcp_timeout", 30)) as client:
            resp = await client.post(
                server["url"],
                json={
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {"name": tool_name, "arguments": arguments},
                    "id": 1,
                },
                headers={"Content-Type": "application/json", **server.get("headers", {})},
            )
            return resp.json()
    except Exception as e:
        raise HTTPException(status_code=502, detail=f"Erreur appel MCP: {e}")
