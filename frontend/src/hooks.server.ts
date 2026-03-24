/**
 * SvelteKit server hook — proxy /api/* vers le backend FastAPI.
 *
 * En production (Docker), le frontend Node contacte http://backend:8000
 * via le réseau Docker interne. Le navigateur ne voit que /api/* sur le
 * même origin, ce qui élimine les problèmes CORS et "Failed to fetch".
 */
import type { Handle } from '@sveltejs/kit';

const BACKEND_URL = process.env.BACKEND_URL || process.env.PUBLIC_API_URL || 'http://localhost:8000';

export const handle: Handle = async ({ event, resolve }) => {
  const { pathname } = event.url;

  // Proxy toutes les requêtes /api/* et /health vers le backend
  if (pathname.startsWith('/api/') || pathname === '/health') {
    const targetUrl = `${BACKEND_URL}${pathname}${event.url.search}`;

    // WebSocket upgrade — laisse passer (géré par adapter-node)
    if (event.request.headers.get('upgrade') === 'websocket') {
      return resolve(event);
    }

    try {
      const headers = new Headers(event.request.headers);
      // Supprime le host pour éviter les conflits
      headers.delete('host');

      const resp = await fetch(targetUrl, {
        method: event.request.method,
        headers,
        body: event.request.method !== 'GET' && event.request.method !== 'HEAD'
          ? event.request.body
          : undefined,
        // @ts-expect-error duplex needed for streaming body
        duplex: 'half',
      });

      // Reconstruit la réponse pour SvelteKit
      return new Response(resp.body, {
        status: resp.status,
        statusText: resp.statusText,
        headers: resp.headers,
      });
    } catch (err) {
      console.error(`[Proxy] ${event.request.method} ${pathname} → ${BACKEND_URL} FAILED:`, err);
      return new Response(
        JSON.stringify({ detail: `Backend inaccessible: ${err}` }),
        { status: 502, headers: { 'Content-Type': 'application/json' } },
      );
    }
  }

  return resolve(event);
};
