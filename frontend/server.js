/**
 * OllamaStudio — Serveur frontend
 * SvelteKit SSR + Proxy HTTP/WebSocket vers le backend Rust.
 *
 * Tout passe par le port 3000 :
 *   /api/*  → proxy HTTP vers le backend (port 8000)
 *   /health → proxy HTTP vers le backend
 *   WS /api/* → proxy WebSocket vers le backend
 *   Reste   → SvelteKit (pages, assets)
 */
import http from 'node:http';
import { handler } from './build/handler.js';

const PORT = parseInt(process.env.PORT || '3000', 10);
const HOST = process.env.HOST || '0.0.0.0';
const BACKEND = process.env.BACKEND_URL || 'http://127.0.0.1:8000';

const backendUrl = new URL(BACKEND);
const backendHost = backendUrl.hostname;
const backendPort = parseInt(backendUrl.port || '8000', 10);

// ── Proxy HTTP pour /api/* et /health ────────────────────────────
function proxyRequest(req, res) {
  const proxyReq = http.request(
    {
      hostname: backendHost,
      port: backendPort,
      path: req.url,
      method: req.method,
      headers: { ...req.headers, host: `${backendHost}:${backendPort}` },
    },
    (proxyRes) => {
      res.writeHead(proxyRes.statusCode, proxyRes.headers);
      proxyRes.pipe(res);
    },
  );

  proxyReq.on('error', (err) => {
    console.error(`[Proxy] ${req.method} ${req.url} → FAILED:`, err.message);
    if (!res.headersSent) {
      res.writeHead(502, { 'Content-Type': 'application/json' });
    }
    res.end(JSON.stringify({ detail: `Backend inaccessible: ${err.message}` }));
  });

  req.pipe(proxyReq);
}

// ── Serveur HTTP ─────────────────────────────────────────────────
const server = http.createServer((req, res) => {
  // WebSocket upgrade — ne pas répondre (géré par l'event 'upgrade')
  if (req.headers.upgrade && req.headers.upgrade.toLowerCase() === 'websocket') {
    return;
  }

  // Proxy /api/* et /health vers le backend
  if (req.url.startsWith('/api/') || req.url === '/health') {
    return proxyRequest(req, res);
  }

  // Tout le reste → SvelteKit
  handler(req, res);
});

// ── Proxy WebSocket pour /api/* ──────────────────────────────────
server.on('upgrade', (req, clientSocket, clientHead) => {
  if (!req.url?.startsWith('/api/')) {
    clientSocket.destroy();
    return;
  }

  console.log(`[WS Proxy] ${req.url} → ${BACKEND}${req.url}`);

  const headers = { ...req.headers };
  delete headers['sec-websocket-extensions']; // Évite permessage-deflate
  headers.host = `${backendHost}:${backendPort}`;

  const proxyReq = http.request({
    hostname: backendHost,
    port: backendPort,
    path: req.url,
    method: 'GET',
    headers,
  });

  proxyReq.on('upgrade', (proxyRes, backendSocket, backendHead) => {
    let responseHeader = 'HTTP/1.1 101 Switching Protocols\r\n';
    for (const [key, value] of Object.entries(proxyRes.headers)) {
      if (key.toLowerCase() === 'sec-websocket-extensions') continue;
      if (value) responseHeader += `${key}: ${value}\r\n`;
    }
    responseHeader += '\r\n';

    clientSocket.write(responseHeader);
    if (backendHead?.length) clientSocket.write(backendHead);
    if (clientHead?.length) backendSocket.write(clientHead);

    clientSocket.setNoDelay(true);
    backendSocket.setNoDelay(true);

    clientSocket.on('data', (c) => backendSocket.write(c));
    backendSocket.on('data', (c) => clientSocket.write(c));
    clientSocket.on('end', () => backendSocket.end());
    backendSocket.on('end', () => clientSocket.end());
    clientSocket.on('error', () => backendSocket.destroy());
    backendSocket.on('error', () => clientSocket.destroy());
  });

  proxyReq.on('response', (res) => {
    console.warn(`[WS Proxy] Upgrade refused: ${res.statusCode}`);
    clientSocket.end();
  });

  proxyReq.on('error', (err) => {
    console.error('[WS Proxy] Backend unreachable:', err.message);
    clientSocket.write('HTTP/1.1 502 Bad Gateway\r\n\r\n');
    clientSocket.destroy();
  });

  proxyReq.end();
});

server.listen(PORT, HOST, () => {
  console.log(`OllamaStudio frontend — http://${HOST}:${PORT}`);
  console.log(`  Proxy: /api/* + /health → ${BACKEND}`);
});
