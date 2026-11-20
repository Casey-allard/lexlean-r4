import assert from 'node:assert/strict';
import {
  SpiralCoreHostBridge,
  admit_public_cors_preemption,
} from '../../crates/wasm/js/worker_host_bridge.js';
import {
  handleSpvsEnvelope,
  handleWorkerFetch,
} from '../../relay/worker.mjs';

const DEFAULT_RELAY_HTTP_URL =
  'https://lexlean-r4-spiralcore-relay.swarm-relay-cloud-8d31a122.workers.dev/spvs';

async function testSpiralCoreNetCorsPreemptionAndFailClosedGuards() {
  console.log('=== Wave 2 (R4-63) SpiralCoreNet & Unprompted Public CORS Preemption E2E ===');

  // 1. Default new SpiralCoreHostBridge() preserves Sandboxed Zero-NIC isolation
  const defaultBridge = new SpiralCoreHostBridge();
  assert.equal(
    defaultBridge.mode,
    'sandboxed',
    'default SpiralCoreHostBridge() must stay sandboxed',
  );

  // Formal admit_public_cors_preemption checks
  assert.equal(
    admit_public_cors_preemption(true, false, false, false),
    'DirectBrowserFetchAdmitted',
  );
  assert.equal(
    admit_public_cors_preemption(true, false, false, true),
    'StatelessRelayCorsPreempted',
  );
  assert.equal(
    admit_public_cors_preemption(true, false, true, true),
    'RefusedCredentialedCorsProxy',
  );
  assert.equal(
    admit_public_cors_preemption(true, true, false, true),
    'RefusedPrivateAddressSsrf',
  );
  assert.equal(
    admit_public_cors_preemption(false, false, false, true),
    'RefusedSpiralCoreNetDisabled',
  );

  // Mock public internet & CORS browser fetch simulation routed through real relay/worker.mjs
  let relayUpstreamHits = 0;
  const mockUpstreamFetch = async (url, _init) => {
    relayUpstreamHits += 1;
    return new Response(`<html><body><h1>Relayed OK</h1><p>${url}</p></body></html>`, {
      status: 200,
      headers: { 'content-type': 'text/html; charset=utf-8' },
    });
  };

  let corsBlockUrls = new Set(['https://example.com/cors-blocked']);
  const browserFetchImpl = async (url, init) => {
    const u = String(url);
    if (u === DEFAULT_RELAY_HTTP_URL) {
      const req = new Request(u, init);
      return await handleWorkerFetch(req, null, null, { fetchImpl: mockUpstreamFetch });
    }
    if (corsBlockUrls.has(u)) {
      throw new TypeError('Failed to fetch (simulated browser CORS preflight block)');
    }
    if (u === 'https://example.com/direct-ok') {
      return new Response('DIRECT_CORS_ENABLED_OK', {
        status: 200,
        headers: { 'content-type': 'text/plain' },
      });
    }
    throw new Error(`unexpected direct fetch URL in test: ${u}`);
  };

  // 2. SpiralCoreNet admits arbitrary public hosts without Consent popup or R4-59 10-domain allowlist
  const netBridge = new SpiralCoreHostBridge({
    mode: 'spiralcore_net',
    fetchImpl: browserFetchImpl,
  });
  assert.equal(netBridge.mode, 'spiralcore_net');
  assert.equal(netBridge.consentGranted, false);

  const directRes = await netBridge.handleHttpRequest({
    method: 'GET',
    url: 'https://example.com/direct-ok',
  });
  assert.equal(directRes.ok, true, `Tier 1 direct public fetch failed: ${JSON.stringify(directRes)}`);
  assert.equal(directRes.transport, 'direct-https-fetch');
  assert.equal(directRes.body_text, 'DIRECT_CORS_ENABLED_OK');
  assert.equal(relayUpstreamHits, 0, 'Tier 1 direct success must not touch relay');

  // 3. Tier 2 unprompted public CORS preemption on CORS TypeError
  const corsPreempted = await netBridge.handleHttpRequest({
    method: 'GET',
    url: 'https://example.com/cors-blocked',
  });
  assert.equal(
    corsPreempted.ok,
    true,
    `Tier 2 CORS preemption failed: ${JSON.stringify(corsPreempted)}`,
  );
  assert.equal(
    corsPreempted.transport,
    'spiralcore-cors-preempt',
    `expected transport='spiralcore-cors-preempt', got ${corsPreempted.transport}`,
  );
  assert.ok(corsPreempted.body_text.includes('Relayed OK'));
  assert.equal(relayUpstreamHits, 1, 'relay/worker.mjs must handle Tier 2 preemption');

  // 4. SSRF private & link-local CIDRs blocked BEFORE direct fetch or relay
  for (const badUrl of [
    'https://127.0.0.1/secret',
    'https://169.254.169.254/latest/meta-data/',
    'https://192.168.1.1/router',
  ]) {
    const ssrfRes = await netBridge.handleHttpRequest({ method: 'GET', url: badUrl });
    assert.equal(ssrfRes.ok, false, `must block ${badUrl}`);
    assert.equal(ssrfRes.error_code, 'PrivateAddressBlocked');
  }

  // 5. Credentialed requests & GitHub hosts remain STRICTLY forbidden from relay failback
  const credCorsBlockFetch = async () => {
    throw new TypeError('Failed to fetch');
  };
  const credBridge = new SpiralCoreHostBridge({
    mode: 'spiralcore_net',
    fetchImpl: credCorsBlockFetch,
  });

  const authFail = await credBridge.handleHttpRequest({
    method: 'GET',
    url: 'https://example.com/private',
    headers: { Authorization: 'Bearer secret-user-token' },
  });
  assert.equal(authFail.ok, false);
  assert.equal(
    authFail.error_code,
    'RelayForbiddenForCredentialedHost',
    `Authorization header must never reach relay: ${JSON.stringify(authFail)}`,
  );

  const cookieFail = await credBridge.handleHttpRequest({
    method: 'GET',
    url: 'https://example.com/account',
    headers: { Cookie: 'session=topsecret' },
  });
  assert.equal(cookieFail.ok, false);
  assert.equal(
    cookieFail.error_code,
    'RelayForbiddenForCredentialedHost',
    `Cookie header must never reach relay: ${JSON.stringify(cookieFail)}`,
  );

  const githubFail = await credBridge.handleHttpRequest({
    method: 'GET',
    url: 'https://api.github.com/user/repos',
  });
  assert.equal(githubFail.ok, false);
  assert.equal(githubFail.error_code, 'RelayForbiddenForCredentialedHost');

  // 6. Direct relay/worker.mjs enforcement against credentialed CORS preemption attempts
  for (const [label, params] of [
    ['Authorization header', { method: 'GET', url: 'https://example.com/a', headers: { Authorization: 'Bearer tok' } }],
    ['Cookie header', { method: 'GET', url: 'https://example.com/b', headers: { Cookie: 'sid=123' } }],
    ['X-GitHub-Token header', { method: 'GET', url: 'https://example.com/c', headers: { 'X-GitHub-Token': 'ghp_123' } }],
    ['GitHub auth_bearer', { method: 'GET', url: 'https://api.github.com/user', auth_bearer: 'ghp_123' }],
  ]) {
    const rRes = await handleSpvsEnvelope(
      { req_id: `relay-cred-${label}`, spvs_magic: 'SPVS', src_cid: 3, dst_cid: 2, vsock_port: 5257, op: 'http_request', params },
      { fetchImpl: mockUpstreamFetch },
    );
    assert.equal(rRes.ok, false, `relay must reject ${label}`);
    assert.equal(
      rRes.error_code,
      'RelayForbiddenForCredentialedHost',
      `relay expected RelayForbiddenForCredentialedHost for ${label}, got ${JSON.stringify(rRes)}`,
    );
  }
  assert.equal(relayUpstreamHits, 1, 'credentialed relay rejections must happen before upstream fetch');

  console.log('[PASS] R4-63 SpiralCoreNet & unprompted CORS preemption verified');
}

await testSpiralCoreNetCorsPreemptionAndFailClosedGuards();
