/**
 * Serveur custom OllamaStudio — SvelteKit + proxy WebSocket.
 */
import http from 'node:http';
import { handler } from './build/handler.js';

const PORT = parseInt(process.env.PORT || '3000', 10);
const HOST = process.env.HOST || '0.0.0.0';
const BACKEND = process.env.BACKEND_URL || 'http://backend:8000';

const backendUrl = new URL(BACKEND);

// ── Serveur HTTP ────────────────────────────────────────────────
const server = http.createServer((req, res) => {
  if (req.headers.upgrade && req.headers.upgrade.toLowerCase() === 'websocket') {
    return; // Laisse l'événement 'upgrade' gérer
  }
  handler(req, res);
});

// ── Proxy WebSocket pour /api/* ─────────────────────────────────
server.on('upgrade', (req, clientSocket, clientHead) => {
  if (!req.url?.startsWith('/api/')) {
    clientSocket.destroy();
    return;
  }

  console.log(`[WS Proxy] Upgrade ${req.url} → ${BACKEND}${req.url}`);

  // Copie les headers mais SUPPRIME les extensions WebSocket
  // pour éviter permessage-deflate qui casse le proxy TCP brut
  const headers = { ...req.headers };
  delete headers['sec-websocket-extensions'];
  headers.host = `${backendUrl.hostname}:${backendUrl.port || 8000}`;

  const proxyReq = http.request({
    hostname: backendUrl.hostname,
    port: backendUrl.port || 8000,
    path: req.url,
    method: 'GET',
    headers,
  });

  proxyReq.on('upgrade', (proxyRes, backendSocket, backendHead) => {
    // Reconstruit la réponse 101 SANS les extensions
    let responseHeader = 'HTTP/1.1 101 Switching Protocols\r\n';
    for (const [key, value] of Object.entries(proxyRes.headers)) {
      // Supprime les extensions pour que le navigateur n'utilise pas de compression
      if (key.toLowerCase() === 'sec-websocket-extensions') continue;
      if (value) responseHeader += `${key}: ${value}\r\n`;
    }
    responseHeader += '\r\n';

    clientSocket.write(responseHeader);

    // Envoie les données résiduelles des deux côtés
    if (backendHead && backendHead.length) {
      clientSocket.write(backendHead);
    }
    if (clientHead && clientHead.length) {
      backendSocket.write(clientHead);
    }

    // Désactive le buffering Nagle pour un envoi immédiat
    clientSocket.setNoDelay(true);
    backendSocket.setNoDelay(true);

    // Bidirectionnel avec logging
    clientSocket.on('data', (chunk) => {
      backendSocket.write(chunk);
    });

    backendSocket.on('data', (chunk) => {
      clientSocket.write(chunk);
    });

    clientSocket.on('end', () => backendSocket.end());
    backendSocket.on('end', () => clientSocket.end());

    clientSocket.on('error', (err) => {
      console.error('[WS Proxy] Client error:', err.message);
      backendSocket.destroy();
    });

    backendSocket.on('error', (err) => {
      console.error('[WS Proxy] Backend error:', err.message);
      clientSocket.destroy();
    });

    console.log(`[WS Proxy] Connected: ${req.url}`);
  });

  proxyReq.on('response', (res) => {
    console.warn(`[WS Proxy] Upgrade refused: ${res.statusCode}`);
    let header = `HTTP/1.1 ${res.statusCode} ${res.statusMessage}\r\n`;
    for (const [key, value] of Object.entries(res.headers)) {
      if (value) header += `${key}: ${value}\r\n`;
    }
    header += '\r\n';
    clientSocket.write(header);
    res.pipe(clientSocket);
  });

  proxyReq.on('error', (err) => {
    console.error('[WS Proxy] Backend unreachable:', err.message);
    clientSocket.write('HTTP/1.1 502 Bad Gateway\r\n\r\n');
    clientSocket.destroy();
  });

  proxyReq.end();
});

server.listen(PORT, HOST, () => {
  console.log(`OllamaStudio frontend listening on http://${HOST}:${PORT}`);
  console.log(`WebSocket proxy: /api/* → ${BACKEND}`);
});
