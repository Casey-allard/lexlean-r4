/* Formal R4.LiveWebGpuTurnBridge (R4-65): COOP same-origin + COEP credentialless Service Worker
 * Enables window.crossOriginIsolated (SharedArrayBuffer multi-worker Wasm pool) on static GitHub Pages
 * without blocking cross-origin weight pack mirrors or GitHub API fetches (never uses require-corp).
 */
self.addEventListener('install', () => {
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (!req || req.mode !== 'navigate') {
    return;
  }
  event.respondWith(
    fetch(req).then((resp) => {
      if (!resp || resp.status === 0 || resp.type === 'opaque') {
        return resp;
      }
      const headers = new Headers(resp.headers);
      headers.set('Cross-Origin-Opener-Policy', 'same-origin');
      headers.set('Cross-Origin-Embedder-Policy', 'credentialless');
      return new Response(resp.body, {
        status: resp.status,
        statusText: resp.statusText,
        headers,
      });
    }),
  );
});
