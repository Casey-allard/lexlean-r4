/* Formal R4.LiveWebGpuTurnBridge (R4-65): COOP same-origin + COEP credentialless Service Worker (v2)
 * Enables window.crossOriginIsolated (SharedArrayBuffer multi-worker Wasm pool) on static GitHub Pages.
 * Injects COOP + COEP: credentialless + CORP: same-origin on ALL same-origin GET responses (including
 * Dedicated Workers `lexlean_r4.js` and child iframes `index.html?surface=preview`), while letting
 * cross-origin weight pack mirrors and GitHub API fetches pass straight through untouched.
 */
self.addEventListener('install', () => {
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (!req || req.method !== 'GET') {
    return;
  }
  if (req.cache === 'only-if-cached' && req.mode !== 'same-origin') {
    return;
  }
  let sameOrigin = false;
  try {
    sameOrigin = new URL(req.url).origin === self.location.origin;
  } catch {
    sameOrigin = false;
  }
  if (!sameOrigin) {
    return;
  }
  event.respondWith(
    fetch(req).then((resp) => {
      if (!resp || resp.status === 0 || resp.type === 'opaque' || resp.type === 'opaqueredirect') {
        return resp;
      }
      const headers = new Headers(resp.headers);
      headers.set('Cross-Origin-Opener-Policy', 'same-origin');
      headers.set('Cross-Origin-Embedder-Policy', 'credentialless');
      headers.set('Cross-Origin-Resource-Policy', 'same-origin');
      return new Response(resp.body, {
        status: resp.status,
        statusText: resp.statusText,
        headers,
      });
    }),
  );
});
