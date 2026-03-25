/**
 * Serveur custom OllamaStudio — SvelteKit + proxy WebSocket.
 *
 * adapter-node génère build/handler.js qui exporte { handler }.
 * On crée notre propre serveur HTTP pour :
 * 1. Servir les pages SvelteKit (requêtes HTTP normales)
 * 2. Proxifier les WebSocket /api/* vers le backend Docker
 *
 * IMPORTANT : les requêtes HTTP Upgrade (WebSocket) ne doivent PAS
 * passer par le handler SvelteKit, sinon il répond avant que
 * l'événement 'upgrade' ne puisse les traiter.
 */
import http from 'node:http';
import { handler } from './build/handler.js';

const PORT = parseInt(process.env.PORT || '3000', 10);
const HOST = process.env.HOST || '0.0.0.0';
const BACKEND = process.env.BACKEND_URL || 'http://backend:8000';

const backendUrl = new URL(BACKEND);

// ── Serveur HTTP ────────────────────────────────────────────────
// Wrapper du handler SvelteKit : ignore les requêtes WebSocket Upgrade
// pour laisser l'événement 'upgrade' les gérer.
const server = http.createServer((req, res) => {
  if (req.headers.upgrade && req.headers.upgrade.toLowerCase() === 'websocket') {
    // Ne rien faire — l'événement 'upgrade' va s'en occuper.
    // On ne répond PAS pour que la connexion reste ouverte.
    return;
  }
  handler(req, res);
});

// ── Proxy WebSocket pour /api/* ─────────────────────────────────
server.on('upgrade', (req, socket, head) => {
  if (!req.url?.startsWith('/api/')) {
    socket.destroy();
    return;
  }

  console.log(`[WS Proxy] Upgrade ${req.url} → ${BACKEND}${req.url}`);

  const proxyReq = http.request({
    hostname: backendUrl.hostname,
    port: backendUrl.port || 8000,
    path: req.url,
    method: 'GET',
    headers: {
      ...req.headers,
      host: `${backendUrl.hostname}:${backendUrl.port || 8000}`,
    },
  });

  proxyReq.on('upgrade', (proxyRes, proxySocket, proxyHead) => {
    let responseHeader = 'HTTP/1.1 101 Switching Protocols\r\n';
    for (const [key, value] of Object.entries(proxyRes.headers)) {
      if (value) responseHeader += `${key}: ${value}\r\n`;
    }
    responseHeader += '\r\n';

    socket.write(responseHeader);
    if (proxyHead && proxyHead.length) {
      socket.write(proxyHead);
    }

    // Bidirectionnel
    proxySocket.pipe(socket);
    socket.pipe(proxySocket);

    proxySocket.on('error', (err) => {
      console.error('[WS Proxy] Backend socket error:', err.message);
      socket.destroy();
    });

    socket.on('error', (err) => {
      console.error('[WS Proxy] Client socket error:', err.message);
      proxySocket.destroy();
    });

    console.log(`[WS Proxy] Connection established: ${req.url}`);
  });

  proxyReq.on('response', (res) => {
    console.warn(`[WS Proxy] Upgrade refused by backend: ${res.statusCode}`);
    let header = `HTTP/1.1 ${res.statusCode} ${res.statusMessage}\r\n`;
    for (const [key, value] of Object.entries(res.headers)) {
      if (value) header += `${key}: ${value}\r\n`;
    }
    header += '\r\n';
    socket.write(header);
    res.pipe(socket);
  });

  proxyReq.on('error', (err) => {
    console.error('[WS Proxy] Connection to backend failed:', err.message);
    socket.write('HTTP/1.1 502 Bad Gateway\r\n\r\n');
    socket.destroy();
  });

  proxyReq.end();
});

server.listen(PORT, HOST, () => {
  console.log(`OllamaStudio frontend listening on http://${HOST}:${PORT}`);
  console.log(`WebSocket proxy: /api/* → ${BACKEND}`);
});
