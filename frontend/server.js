/**
 * Serveur custom OllamaStudio — SvelteKit + proxy WebSocket.
 *
 * adapter-node génère build/handler.js qui exporte { handler }.
 * On crée notre propre serveur HTTP pour ajouter le proxy WebSocket
 * vers le backend (le navigateur ne peut pas accéder au port 8000
 * directement en Docker).
 */
import http from 'node:http';
import { handler } from './build/handler.js';

const PORT = parseInt(process.env.PORT || '3000', 10);
const HOST = process.env.HOST || '0.0.0.0';
const BACKEND = process.env.BACKEND_URL || 'http://backend:8000';

const backendUrl = new URL(BACKEND);

// ── Serveur HTTP avec le handler SvelteKit ──────────────────────
const server = http.createServer(handler);

// ── Proxy WebSocket pour /api/shell/ws ──────────────────────────
server.on('upgrade', (req, socket, head) => {
  // Ne proxy que les chemins /api/*
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
    // Construit la réponse 101 Switching Protocols
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
  });

  proxyReq.on('response', (res) => {
    // Le backend a refusé l'upgrade — renvoie la réponse HTTP
    console.warn(`[WS Proxy] Upgrade refused: ${res.statusCode} ${res.statusMessage}`);
    let header = `HTTP/1.1 ${res.statusCode} ${res.statusMessage}\r\n`;
    for (const [key, value] of Object.entries(res.headers)) {
      if (value) header += `${key}: ${value}\r\n`;
    }
    header += '\r\n';
    socket.write(header);
    res.pipe(socket);
  });

  proxyReq.on('error', (err) => {
    console.error('[WS Proxy] Connection error:', err.message);
    socket.write('HTTP/1.1 502 Bad Gateway\r\n\r\n');
    socket.destroy();
  });

  proxyReq.end();
});

server.listen(PORT, HOST, () => {
  console.log(`OllamaStudio frontend listening on http://${HOST}:${PORT}`);
  console.log(`WebSocket proxy: /api/* → ${BACKEND}`);
});
