import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';

import {
  SpiralCoreHostBridge,
  hydrate_holo_model,
  execute_inprism_foundry_turn,
} from '../../crates/wasm/js/worker_host_bridge.js';
import { startLocalRelay } from '../../relay/local-relay.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..', '..');

async function testCapabilitiesGateAndSpiralCoreFailback() {
  console.log('1. Testing capabilities/1 Sandboxed vs Full SpiralCore, Consent, Relay Failback & Credential Prohibition...');

  const auditEvents = [];
  let relayUpstreamHits = 0;

  const upstreamServer = http.createServer((req, res) => {
    const u = new URL(req.url || '/', 'http://127.0.0.1');
    if (u.pathname === '/cors-blocked-page') {
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ ok: true, via: 'spiralcore-relay', path: u.pathname }));
      return;
    }
    res.writeHead(200, { 'content-type': 'text/plain' });
    res.end('ok');
  });
  await new Promise((resolve) => upstreamServer.listen(0, '127.0.0.1', resolve));
  const upstreamPort = upstreamServer.address().port;

  // Relay server that tracks every relay RPC hit
  const relayFetch = async (url, init = {}) => {
    relayUpstreamHits += 1;
    const parsed = new URL(String(url));
    if (parsed.hostname === 'example.com') {
      return fetch(`http://127.0.0.1:${upstreamPort}${parsed.pathname}${parsed.search}`, init);
    }
    throw new Error(`Unexpected relay target: ${url}`);
  };
  const relayHandle = await startLocalRelay({ port: 0, host: '127.0.0.1', fetchImpl: relayFetch });
  const relayWsUrl = `ws://127.0.0.1:${relayHandle.port}/spvs`;

  try {
    const directFetch = async (url, init = {}) => {
      const parsed = new URL(String(url));
      if (parsed.hostname === 'registry.npmjs.org') {
        return new Response(JSON.stringify({ name: 'zod', 'dist-tags': { latest: '3.24.0' } }), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      }
      if (parsed.hostname === 'example.com' && parsed.pathname === '/cors-blocked-page') {
        throw new TypeError('Failed to fetch (simulated CORS refusal)');
      }
      if (parsed.hostname === 'api.github.com') {
        throw new TypeError('Failed to fetch (simulated api.github.com network/CORS failure)');
      }
      throw new TypeError(`Network failure for ${url}`);
    };

    const bridge = new SpiralCoreHostBridge({
      fetchImpl: directFetch,
      relayWsUrl,
      mode: 'sandboxed',
      consentGranted: false,
      onAuditEvent: (ev) => auditEvents.push(ev),
    });

    // 1a. Mode A (Sandboxed): allowlisted registry.npmjs.org succeeds directly
    const npmOk = await bridge.execute({
      op: 'http_request',
      url: 'https://registry.npmjs.org/zod',
    });
    assert.equal(npmOk.ok, true, `expected allowlisted npm fetch to succeed: ${JSON.stringify(npmOk)}`);
    assert.equal(npmOk.transport, 'direct-https-fetch');

    // 1b. Mode A (Sandboxed): unallowlisted example.com returns CapabilityDenied('example.com')
    const capDenied = await bridge.execute({
      op: 'http_request',
      url: 'https://example.com/cors-blocked-page',
    });
    assert.equal(capDenied.ok, false);
    assert.equal(capDenied.error_code, 'CapabilityDenied');
    assert.equal(capDenied.host, 'example.com');
    assert.equal(relayUpstreamHits, 0, 'Sandboxed mode must never hit SpiralCore relay');

    // 1c. Escalating to Full SpiralCore without consentGranted=true returns RelayConsentRequired
    const noConsentPolicy = bridge.setPolicy({ mode: 'full_spiralcore', consentGranted: false });
    assert.equal(noConsentPolicy.ok, false);
    assert.equal(noConsentPolicy.error_code, 'RelayConsentRequired');

    const noConsentExec = await bridge.execute({
      op: 'http_request',
      url: 'https://example.com/cors-blocked-page',
    });
    assert.equal(noConsentExec.ok, false);
    assert.equal(noConsentExec.error_code, 'RelayConsentRequired');
    assert.equal(relayUpstreamHits, 0);

    // 1d. Full SpiralCore with consentGranted=true fails back over AF_VSOCK relayWsUrl on CORS TypeError (unauthenticated)
    const grantedPolicy = bridge.setPolicy({ mode: 'full_spiralcore', consentGranted: true });
    assert.equal(grantedPolicy.ok, true);

    const relayedOk = await bridge.execute({
      op: 'http_request',
      url: 'https://example.com/cors-blocked-page',
    });
    assert.equal(relayedOk.ok, true, `expected relay failback to succeed: ${JSON.stringify(relayedOk)}`);
    assert.equal(relayedOk.transport, 'spvs-local-ws');
    assert.ok(relayedOk.body_text.includes('spiralcore-relay'));
    assert.equal(relayUpstreamHits, 1, 'Unauthenticated CORS failure in Full SpiralCore must route through relay');

    // 1e. Credentialed host (api.github.com) or Authorization header MUST NEVER fail back to relay -> RelayForbiddenForCredentialedHost
    bridge.setSecret('GITHUB_TOKEN', 'ghp_observatory_test_pat');
    const ghBlocked = await bridge.execute({
      op: 'http_request',
      url: 'https://api.github.com/user',
      headers: { Authorization: 'Bearer {{MVM_SECRET:GITHUB_TOKEN}}' },
    });
    assert.equal(ghBlocked.ok, false);
    assert.equal(ghBlocked.error_code, 'RelayForbiddenForCredentialedHost');
    assert.equal(relayUpstreamHits, 1, 'Credentialed api.github.com failure must NEVER hit relay');

    const authHeaderBlocked = await bridge.execute({
      op: 'http_request',
      url: 'https://example.com/cors-blocked-page',
      headers: { Authorization: 'Bearer user-session-secret' },
    });
    assert.equal(authHeaderBlocked.ok, false);
    assert.equal(authHeaderBlocked.error_code, 'RelayForbiddenForCredentialedHost');
    assert.equal(relayUpstreamHits, 1, 'Request carrying Authorization header must NEVER hit relay');

    // 1f. Audit events emitted cleanly on outbound calls
    assert.ok(auditEvents.length >= 4, 'Expected audit_event logs on outbound bridge calls');
    for (const ev of auditEvents) {
      assert.ok(ev.operation && ev.target && ev.mode && ev.transport !== undefined);
      assert.ok('status' in ev && 'bytes' in ev);
    }

    bridge.close();
  } finally {
    await relayHandle.close();
    await new Promise((resolve) => upstreamServer.close(resolve));
  }
  console.log('  ✓ capabilities/1 gate, consent, AF_VSOCK failback, RelayForbiddenForCredentialedHost, and audit_event verified.');
}

async function testCorsDirectGithubGitDataAndDohSsrf() {
  console.log('2. Testing CORS-direct api.github.com Git Data APIs & RFC 8484 DoH + SSRF CIDR blocking...');

  const seenRequests = [];
  let relayHitCount = 0;
  const fakeRelayWs = 'ws://127.0.0.1:1/spvs';

  const mockFetch = async (url, init = {}) => {
    const u = new URL(String(url));
    seenRequests.push({ host: u.hostname, path: u.pathname, method: init.method || 'GET', headers: init.headers || {} });

    if (u.hostname === 'cloudflare-dns.com' && u.pathname === '/dns-query') {
      const name = u.searchParams.get('name') || '';
      if (name === 'rebind.internal-test') {
        return new Response(JSON.stringify({ Status: 0, Answer: [{ name, type: 1, TTL: 60, data: '10.0.0.42' }] }), { status: 200 });
      }
      return new Response(JSON.stringify({ Status: 0, Answer: [{ name, type: 1, TTL: 120, data: '93.184.216.34' }] }), { status: 200 });
    }
    if (u.hostname === 'dns.google' && u.pathname === '/resolve') {
      const name = u.searchParams.get('name') || '';
      return new Response(JSON.stringify({ Status: 0, Answer: [{ name, type: 1, TTL: 120, data: '93.184.216.34' }] }), { status: 200 });
    }
    if (u.hostname === 'api.github.com') {
      const auth = init.headers?.Authorization || init.headers?.authorization || '';
      assert.equal(auth, 'Bearer ghp_cors_direct_pat', 'api.github.com calls must carry PAT via CORS-direct fetch');
      if (u.pathname === '/repos/uor-test/demo') {
        return new Response(JSON.stringify({ default_branch: 'main', private: false, html_url: 'https://github.com/uor-test/demo' }), { status: 200 });
      }
      if (u.pathname === '/repos/uor-test/demo/branches') {
        return new Response(JSON.stringify([{ name: 'main' }]), { status: 200 });
      }
      if (u.pathname === '/repos/uor-test/demo/git/trees/main') {
        return new Response(JSON.stringify({ sha: 'tree_main_1', tree: [{ path: 'README.md', type: 'blob', size: 10, sha: 'blob_readme' }] }), { status: 200 });
      }
      if (u.pathname === '/repos/uor-test/demo/git/blobs/blob_readme') {
        return new Response(JSON.stringify({ encoding: 'base64', content: Buffer.from('# Demo\n').toString('base64'), size: 7 }), { status: 200 });
      }
      if (u.pathname === '/repos/uor-test/demo/git/ref/heads/main') {
        return new Response(JSON.stringify({ ref: 'refs/heads/main', object: { sha: 'commit_main_sha' } }), { status: 200 });
      }
      if (u.pathname === '/repos/uor-test/demo/git/refs' && init.method === 'POST') {
        return new Response(JSON.stringify({ ref: 'refs/heads/feat/r4-59', object: { sha: 'commit_main_sha' } }), { status: 201 });
      }
      if (u.pathname === '/repos/uor-test/demo/git/ref/heads/feat/r4-59') {
        return new Response(JSON.stringify({ ref: 'refs/heads/feat/r4-59', object: { sha: 'commit_main_sha' } }), { status: 200 });
      }
      if (u.pathname === '/repos/uor-test/demo/git/commits/commit_main_sha') {
        return new Response(JSON.stringify({ sha: 'commit_main_sha', tree: { sha: 'base_tree_sha' } }), { status: 200 });
      }
      if (u.pathname === '/repos/uor-test/demo/git/trees' && init.method === 'POST') {
        return new Response(JSON.stringify({ sha: 'new_tree_sha' }), { status: 201 });
      }
      if (u.pathname === '/repos/uor-test/demo/git/commits' && init.method === 'POST') {
        return new Response(JSON.stringify({ sha: 'new_commit_sha' }), { status: 201 });
      }
      if (u.pathname === '/repos/uor-test/demo/git/refs/heads/feat/r4-59' && init.method === 'PATCH') {
        return new Response(JSON.stringify({ ref: 'refs/heads/feat/r4-59', object: { sha: 'new_commit_sha' } }), { status: 200 });
      }
      if (u.pathname === '/repos/uor-test/demo/pulls' && init.method === 'POST') {
        return new Response(JSON.stringify({ number: 99, url: 'https://api.github.com/repos/uor-test/demo/pulls/99', html_url: 'https://github.com/uor-test/demo/pull/99', state: 'open', head: { ref: 'feat/r4-59' }, base: { ref: 'main' }, title: 'R4-59 PR' }), { status: 201 });
      }
    }
    relayHitCount += 1;
    throw new Error(`Unexpected fetch URL: ${url}`);
  };

  const bridge = new SpiralCoreHostBridge({
    fetchImpl: mockFetch,
    relayWsUrl: fakeRelayWs,
    mode: 'sandboxed',
  });
  bridge.setSecret('GITHUB_TOKEN', 'ghp_cors_direct_pat');

  // 2a. GitHub CORS-direct clone -> branch -> commit_push -> create_pr
  const cloned = await bridge.execute({ op: 'git', subcommand: 'clone', repo: 'uor-test/demo', branch: 'main' });
  assert.equal(cloned.ok, true);
  assert.equal(cloned.transport, 'direct-https-github-rest');
  assert.equal(cloned.files[0].path, 'README.md');

  const br = await bridge.execute({ op: 'git', subcommand: 'create_branch', repo: 'uor-test/demo', branch: 'feat/r4-59', from_branch: 'main' });
  assert.equal(br.ok, true);
  assert.equal(br.branch, 'feat/r4-59');

  const pushed = await bridge.execute({
    op: 'git',
    subcommand: 'commit_push',
    repo: 'uor-test/demo',
    branch: 'feat/r4-59',
    message: 'feat: r4-59',
    files: [{ path: 'README.md', content: '# Updated\n' }],
  });
  assert.equal(pushed.ok, true);
  assert.equal(pushed.commit_sha, 'new_commit_sha');

  const pr = await bridge.execute({
    op: 'git',
    subcommand: 'create_pr',
    repo: 'uor-test/demo',
    head: 'feat/r4-59',
    base: 'main',
    title: 'R4-59 PR',
  });
  assert.equal(pr.ok, true);
  assert.equal(pr.pr_number, 99);
  assert.equal(pr.html_url, 'https://github.com/uor-test/demo/pull/99');
  assert.equal(relayHitCount, 0);

  // 2b. RFC 8484 DoH + SSRF private/link-local blocking
  const dohOk = await bridge.execute({ op: 'dns_query', name: 'example.org', rr_type: 'A' });
  assert.equal(dohOk.ok, true, `DoH failed: ${JSON.stringify(dohOk)}`);
  assert.equal(dohOk.answers[0].data, '93.184.216.34');
  assert.ok(seenRequests.some((r) => r.host === 'cloudflare-dns.com'));

  for (const ssrfHost of ['169.254.169.254', '10.0.0.1', '192.168.1.10', '172.16.0.5', '127.0.0.1', 'localhost', '[::1]', '[fe80::1]']) {
    const blockedHttp = await bridge.execute({ op: 'http_request', url: `https://${ssrfHost}/secret` });
    assert.equal(blockedHttp.ok, false, `SSRF HTTP ${ssrfHost} must fail`);
    assert.equal(blockedHttp.error_code, 'PrivateAddressBlocked');

    const blockedDns = await bridge.execute({ op: 'dns_query', name: ssrfHost });
    assert.equal(blockedDns.ok, false, `SSRF DNS ${ssrfHost} must fail`);
    assert.equal(blockedDns.error_code, 'PrivateAddressBlocked');
  }

  // DNS rebinding answer pointing to RFC 1918 10.0.0.42 must also fail PrivateAddressBlocked
  const rebindBlocked = await bridge.execute({ op: 'dns_query', name: 'rebind.internal-test' });
  assert.equal(rebindBlocked.ok, false);
  assert.equal(rebindBlocked.error_code, 'PrivateAddressBlocked');

  // 2c. Full Round 1 negative/edge transport coverage:
  //     DoHResolutionFailed, RelayUnavailable, web_search gating+relay failback,
  //     npm tarball SSRF blocking, pip consent gating, handlePkgFetch SSRF blocking,
  //     #githubRestJson consent, create_branch base_branch honor, and watch_deploy error propagation
  const mockFetchNoDoh = async () => {
    throw new TypeError('Failed to fetch (DoH unreachable)');
  };
  const dohFailBridge = new SpiralCoreHostBridge({
    fetchImpl: mockFetchNoDoh,
    mode: 'sandboxed',
    relayWsUrl: '',
  });
  const dohFail = await dohFailBridge.execute({ op: 'dns_query', name: 'example.org', rr_type: 'A' });
  assert.equal(dohFail.ok, false);
  assert.equal(dohFail.error_code, 'DoHResolutionFailed');

  const unreachRelayHttpBridge = new SpiralCoreHostBridge({
    fetchImpl: mockFetchNoDoh,
    mode: 'full_spiralcore',
    consentGranted: true,
    relayWsUrl: '',
    relayHttpUrl: '',
  });
  const unavailHttp = await unreachRelayHttpBridge.execute({ op: 'http_request', url: 'https://example.com/down' });
  assert.equal(unavailHttp.ok, false);
  assert.equal(unavailHttp.error_code, 'RelayUnavailable');

  const unavailDns = await unreachRelayHttpBridge.execute({ op: 'dns_query', name: 'example.com', rr_type: 'A' });
  assert.equal(unavailDns.ok, false);
  assert.equal(unavailDns.error_code, 'RelayUnavailable');

  let dnsRelayServerHits = 0;
  const dnsRelayHandle = await startLocalRelay({
    port: 0,
    host: '127.0.0.1',
    fetchImpl: async (url) => {
      dnsRelayServerHits += 1;
      const u = new URL(String(url));
      if (u.hostname === 'cloudflare-dns.com') {
        return new Response(JSON.stringify({ Status: 0, Answer: [{ name: 'relay-only.org', type: 1, TTL: 60, data: '93.184.216.34' }] }), {
          status: 200,
          headers: { 'content-type': 'application/dns-json' },
        });
      }
      if (u.hostname === 'api.duckduckgo.com') {
        return new Response(JSON.stringify({ Heading: 'SpiralSearch', AbstractText: 'SpiralCore relay search hit', AbstractURL: 'https://example.org/search' }), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      }
      throw new Error(`Unexpected relay fetch: ${url}`);
    },
  });
  try {
    const dnsRelayUrl = `ws://127.0.0.1:${dnsRelayHandle.port}/spvs`;
    const dnsRelayBridge = new SpiralCoreHostBridge({
      fetchImpl: mockFetchNoDoh,
      mode: 'full_spiralcore',
      consentGranted: true,
      relayWsUrl: dnsRelayUrl,
    });
    const dnsRelayed = await dnsRelayBridge.execute({ op: 'dns_query', name: 'relay-only.org', rr_type: 'A' });
    assert.equal(dnsRelayed.ok, true, `expected dns_query relay failback: ${JSON.stringify(dnsRelayed)}`);
    assert.equal(dnsRelayed.answers[0].data, '93.184.216.34');
    assert.ok(dnsRelayServerHits >= 1);

    const searchAudit = [];
    const sandboxedSearchBridge = new SpiralCoreHostBridge({
      fetchImpl: async () => new Response('{}', { status: 200 }),
      mode: 'sandboxed',
      onAuditEvent: (ev) => searchAudit.push(ev),
    });
    const sandboxSearchDenied = await sandboxedSearchBridge.execute({ op: 'web_search', query: 'prism uor' });
    assert.equal(sandboxSearchDenied.ok, false);
    assert.equal(sandboxSearchDenied.error_code, 'CapabilityDenied');

    const unconsentedSearchBridge = new SpiralCoreHostBridge({
      fetchImpl: async () => new Response('{}', { status: 200 }),
      mode: 'full_spiralcore',
      consentGranted: false,
    });
    const unconsentedSearch = await unconsentedSearchBridge.execute({ op: 'web_search', query: 'prism uor' });
    assert.equal(unconsentedSearch.ok, false);
    assert.equal(unconsentedSearch.error_code, 'RelayConsentRequired');

    const fullSearchBridge = new SpiralCoreHostBridge({
      fetchImpl: async (url) => {
        const u = new URL(String(url));
        if (u.hostname === 'api.duckduckgo.com') {
          return new Response(JSON.stringify({ Heading: 'UOR', AbstractText: 'Pure integer web search', AbstractURL: 'https://uor.foundation/docs' }), { status: 200 });
        }
        return new Response('{}', { status: 404 });
      },
      mode: 'full_spiralcore',
      consentGranted: true,
      onAuditEvent: (ev) => searchAudit.push(ev),
    });
    const searchOk = await fullSearchBridge.execute({ op: 'web_search', query: 'prism uor' });
    assert.equal(searchOk.ok, true);
    assert.ok(searchOk.results.some((r) => r.url === 'https://uor.foundation/docs'));
    assert.ok(searchAudit.some((e) => e.operation === 'web_search'));

    const searchRelayBridge = new SpiralCoreHostBridge({
      fetchImpl: mockFetchNoDoh,
      mode: 'full_spiralcore',
      consentGranted: true,
      relayWsUrl: dnsRelayUrl,
    });
    const relayedSearch = await searchRelayBridge.execute({ op: 'web_search', query: 'prism uor' });
    assert.equal(relayedSearch.ok, true, `expected web_search relay failback: ${JSON.stringify(relayedSearch)}`);
    assert.ok(relayedSearch.results.length >= 1);
    dnsRelayBridge.close();
    searchRelayBridge.close();
  } finally {
    await dnsRelayHandle.close();
  }

  const npmAudit = [];
  const ssrfTarballBridge = new SpiralCoreHostBridge({
    mode: 'sandboxed',
    onAuditEvent: (ev) => npmAudit.push(ev),
    fetchImpl: async (url) => {
      const u = new URL(String(url));
      if (u.hostname === 'registry.npmjs.org') {
        return new Response(JSON.stringify({
          name: 'evil-pkg',
          'dist-tags': { latest: '1.0.0' },
          versions: {
            '1.0.0': {
              dist: { tarball: 'https://169.254.169.254/evil.tgz', integrity: 'sha512-abc' },
            },
          },
        }), { status: 200 });
      }
      throw new Error(`Unexpected fetch ${url}`);
    },
  });
  const npmSsrf = await ssrfTarballBridge.execute({
    op: 'npm_install',
    kind: 'npm_install',
    tool: 'npm',
    args: ['install', 'evil-pkg'],
    cwd: '/workspace',
    workspace: [],
  });
  assert.equal(npmSsrf.ok, false);
  assert.equal(npmSsrf.error_code, 'PrivateAddressBlocked');
  assert.ok(npmAudit.some((e) => e.target.includes('registry.npmjs.org')));

  const pyFakeMod = Buffer.from('export async function loadPyodide(){return{FS:{mkdirTree(){},readdir(){return[]},unlink(){},writeFile(){}},runPythonAsync:async()=>"",loadPackage:async()=>{}};}', 'utf8');
  const pipSri = 'sha384-' + crypto.createHash('sha384').update(pyFakeMod).digest('base64');
  const pipConsentBridge = new SpiralCoreHostBridge({
    mode: 'full_spiralcore',
    consentGranted: false,
    fetchImpl: async () => { throw new Error('must not fetch before consent check'); },
    pyodideLock: { version: '0.27.2', url: 'https://cdn.jsdelivr.net/pyodide/test/py.mjs', sri: pipSri },
  });
  const pipNoConsentDenied = await pipConsentBridge.execute({ op: 'pip_install', kind: 'pip_install', tool: 'pip', args: ['install', 'rich'], cwd: '/workspace', workspace: [] });
  assert.equal(pipNoConsentDenied.ok, false);
  assert.equal(pipNoConsentDenied.error_code, 'RelayConsentRequired');

  const pipAudit = [];
  const pipFetchBridge = new SpiralCoreHostBridge({
    mode: 'full_spiralcore',
    consentGranted: true,
    onAuditEvent: (ev) => pipAudit.push(ev),
    pyodideLock: { version: '0.27.2', url: 'https://cdn.jsdelivr.net/pyodide/test/py.mjs', sri: pipSri },
    fetchImpl: async (url) => {
      if (String(url).includes('cdn.jsdelivr.net')) return new Response(pyFakeMod, { status: 200 });
      if (String(url).includes('pypi.org/pypi/rich/json')) return new Response(JSON.stringify({ info: { name: 'rich' } }), { status: 200 });
      throw new Error(`Unexpected fetch ${url}`);
    },
  });
  const pipOk = await pipFetchBridge.execute({ op: 'pip_install', kind: 'pip_install', tool: 'pip', args: ['install', 'rich'], cwd: '/workspace', workspace: [] });
  assert.equal(pipOk.ok, true);
  assert.ok(pipAudit.some((e) => e.target.includes('pypi.org/pypi/rich/json')));

  const pkgAudit = [];
  const pkgSsrfBridge = new SpiralCoreHostBridge({
    mode: 'sandboxed',
    staticOrigin: 'https://169.254.169.254',
    onAuditEvent: (ev) => pkgAudit.push(ev),
    fetchImpl: async (url) => {
      const u = new URL(String(url));
      if (u.hostname === '169.254.169.254') {
        throw new Error('SSRF target 169.254.169.254 must never be fetched without allowConfiguredOrigin or in http_request; testing custom pkg URL');
      }
      return new Response(JSON.stringify({ packages: [{ name: 'jq', url: 'https://169.254.169.254/jq.elf' }] }), { status: 200 });
    },
  });
  const pkgDenied = await pkgSsrfBridge.execute({
    op: 'pkg_fetch',
    subcommand: 'install',
    pkg_name: 'jq',
    url: 'https://169.254.169.254/jq.elf',
  });
  assert.equal(pkgDenied.ok, false);
  assert.equal(pkgDenied.error_code, 'PrivateAddressBlocked');
  assert.ok(pkgAudit.some((e) => e.operation === 'pkg_fetch' && e.transport === 'blocked'));

  const unconsentedGhBridge = new SpiralCoreHostBridge({
    mode: 'full_spiralcore',
    consentGranted: false,
    fetchImpl: mockFetch,
  });
  const ghNoConsent = await unconsentedGhBridge.execute({ op: 'git', subcommand: 'connect', repo: 'uor-test/demo' });
  assert.equal(ghNoConsent.ok, false);
  assert.equal(ghNoConsent.error_code, 'RelayConsentRequired');

  let fetchedBaseRefPath = '';
  const baseBranchBridge = new SpiralCoreHostBridge({
    mode: 'sandboxed',
    fetchImpl: async (url, init = {}) => {
      const u = new URL(String(url));
      if (u.pathname.includes('/git/ref/heads/')) fetchedBaseRefPath = u.pathname;
      if (u.pathname === '/repos/uor-test/demo/git/ref/heads/release/v1') {
        return new Response(JSON.stringify({ ref: 'refs/heads/release/v1', object: { sha: 'rel_v1_sha' } }), { status: 200 });
      }
      if (u.pathname === '/repos/uor-test/demo/git/refs' && init.method === 'POST') {
        return new Response(JSON.stringify({ ref: 'refs/heads/feat/from-rel', object: { sha: 'rel_v1_sha' } }), { status: 201 });
      }
      throw new Error(`Unexpected branch fetch ${u.pathname}`);
    },
  });
  baseBranchBridge.setSecret('GITHUB_TOKEN', 'ghp_base_branch_pat');
  const brBase = await baseBranchBridge.execute({
    op: 'git_smart_http',
    subcommand: 'create_branch',
    repo_url: 'https://github.com/uor-test/demo.git',
    branch: 'feat/from-rel',
    base_branch: 'release/v1',
  });
  assert.equal(brBase.ok, true, `create_branch base_branch failed: ${JSON.stringify(brBase)}`);
  assert.equal(fetchedBaseRefPath, '/repos/uor-test/demo/git/ref/heads/release/v1');
  assert.equal(brBase.from_branch, 'release/v1');

  const watchCorsErrBridge = new SpiralCoreHostBridge({
    mode: 'sandboxed',
    fetchImpl: async () => { throw new TypeError('Failed to fetch (simulated api.github.com CORS error)'); },
  });
  const watchErr = await watchCorsErrBridge.execute({ op: 'git', subcommand: 'watch_deploy', repo: 'uor-test/demo' });
  assert.equal(watchErr.ok, false, 'watch_deploy must propagate RelayForbiddenForCredentialedHost rather than swallowing as unconfigured');
  assert.equal(watchErr.error_code, 'RelayForbiddenForCredentialedHost');

  console.log('  ✓ CORS-direct GitHub REST + Git Data APIs and RFC 8484 DoH + SSRF CIDR blocking verified.');
}

async function testObservatoryChromelessHandshakeAndRender() {
  console.log('3. Testing ?surface=observatory chromeless CSS, opaque null-origin ready handshake & 4-canvas render...');

  const appCss = fs.readFileSync(path.join(ROOT, 'public', 'app.css'), 'utf8');
  const appJs = fs.readFileSync(path.join(ROOT, 'public', 'app.js'), 'utf8');
  assert.ok(appCss.includes('surface-observatory') || appCss.includes('data-surface="observatory"'), 'app.css must style ?surface=observatory');
  assert.ok(appCss.includes('var(--background') && appCss.includes('var(--card'), 'app.css Observatory styles must consume host CSS variables');
  assert.equal(appJs.toLowerCase().includes('telemetry'), false, 'public/app.js must not contain raw "telemetry" string');

  // Verify opaque origin ?surface=observatory initialization order & live frame rendering
  const obsKey = ['tele', 'metry'].join('');
  const drawCalls = {
    'holo-layer-waterfall-canvas': 0,
    'holo-shortlist-race-canvas': 0,
    'holo-causal-flock-canvas': 0,
    'holo-storage-geometry-canvas': 0,
  };

  class MockObsEl {
    constructor(id = '') {
      this.id = id;
      this.attrs = {};
      this.dataset = {};
      this.textContent = '';
      this.value = '';
      this.listeners = {};
      this.width = 460;
      this.height = 220;
      const cls = new Set();
      this.classList = {
        add: (...xs) => xs.forEach((x) => cls.add(x)),
        remove: (...xs) => xs.forEach((x) => cls.delete(x)),
        contains: (x) => cls.has(x),
      };
      this.style = {
        props: {},
        setProperty: (k, v) => { this.style.props[k] = v; },
      };
    }
    setAttribute(k, v) { this.attrs[k] = String(v); }
    removeAttribute(k) { delete this.attrs[k]; }
    hasAttribute(k) { return Object.prototype.hasOwnProperty.call(this.attrs, k); }
    addEventListener(t, fn) { (this.listeners[t] ||= []).push(fn); }
    getContext() {
      const id = this.id;
      return {
        fillStyle: '',
        strokeStyle: '',
        lineWidth: 1,
        font: '',
        clearRect: () => {},
        fillRect: () => { if (id in drawCalls) drawCalls[id] += 1; },
        strokeRect: () => {},
        fillText: () => { if (id in drawCalls) drawCalls[id] += 1; },
        beginPath: () => {},
        moveTo: () => {},
        lineTo: () => {},
        stroke: () => {},
        arc: () => {},
        fill: () => {},
      };
    }
  }

  const els = new Map(
    [
      'inspectors-modal',
      'canvas-panel-traces',
      'holo-observatory-deck',
      'holo-obs-status-badge',
      'holo-obs-speed-stat',
      'holo-obs-prefix-stat',
      'holo-obs-shortlist-stat',
      'holo-obs-backend-stat',
      'holo-obs-layer-caption',
      'holo-obs-vocab-caption',
      'holo-obs-flock-caption',
      'holo-obs-storage-caption',
      'holo-obs-refresh-storage-btn',
      'holo-obs-pause-toggle',
      'holo-layer-waterfall-canvas',
      'holo-shortlist-race-canvas',
      'holo-causal-flock-canvas',
      'holo-storage-geometry-canvas',
    ].map((id) => [id, new MockObsEl(id)]),
  );
  els.get('inspectors-modal').setAttribute('hidden', '');
  els.get('canvas-panel-traces').setAttribute('hidden', '');

  const docEl = new MockObsEl('html');
  const bodyEl = new MockObsEl('body');
  const postedToParent = [];
  const winListeners = {};
  let readyListenerCountAtPost = 0;

  const parentWin = {
    postMessage(msg, targetOrigin) {
      if (msg && msg.protocol === 'uor-observatory/1' && msg.op === 'ready') {
        readyListenerCountAtPost = (winListeners.message || []).length;
      }
      postedToParent.push({ msg, targetOrigin });
    },
  };
  const mockWindow = {
    origin: 'null',
    parent: parentWin,
    location: { search: '?surface=observatory', hash: '', href: 'holo://lexlean-r4/?surface=observatory' },
    getComputedStyle: () => ({
      getPropertyValue: (name) => docEl.style.props[name] || '',
    }),
    addEventListener: (t, fn) => { (winListeners[t] ||= []).push(fn); },
  };
  const mockDocument = {
    documentElement: docEl,
    body: bodyEl,
    getElementById: (id) => els.get(id) || null,
  };

  assert.ok(appJs.includes('function initObservatorySurface('), 'public/app.js must define initObservatorySurface()');
  const stateStart = appJs.indexOf('const holoObservatoryState = {');
  const obsStart = appJs.indexOf('function drawHoloCanvasHeader(');
  const obsEnd = appJs.indexOf('let __uorcWatchdogButtonsBound = false;');
  const surfStart = appJs.indexOf('function initObservatorySurface(');
  const surfEnd = appJs.indexOf('\nfunction ', surfStart + 20);
  assert.ok(stateStart >= 0 && obsStart > stateStart && obsEnd > obsStart && surfStart >= 0 && surfEnd > surfStart);

  const harnessSrc = [
    appJs.slice(stateStart, appJs.indexOf('\nfunction ', stateStart)),
    'function dispatchStudioEnvelope() { return {}; }',
    appJs.slice(obsStart, obsEnd),
    appJs.slice(surfStart, surfEnd),
    'return { initObservatorySurface, holoObservatoryState };',
  ].join('\n');

  const api = new Function('window', 'document', 'engineWorkerClient', harnessSrc)(
    mockWindow,
    mockDocument,
    { enabled: false },
  );
  api.initObservatorySurface();

  assert.equal(bodyEl.dataset.surface, 'observatory');
  assert.equal(bodyEl.classList.contains('surface-observatory'), true);
  assert.ok(readyListenerCountAtPost >= 1, 'message listener MUST be installed before uor-observatory/1 ready is posted');
  const readyMsg = postedToParent.find((x) => x.msg?.protocol === 'uor-observatory/1' && x.msg?.op === 'ready');
  assert.ok(readyMsg, 'Must post { protocol: "uor-observatory/1", op: "ready" } to window.parent');

  // Foreign non-parent postMessage MUST be ignored (source verification)
  const foreignWin = { postMessage() {} };
  for (const fn of winListeners.message || []) {
    await fn({
      origin: 'null',
      source: foreignWin,
      data: {
        protocol: 'uor-observatory/1',
        op: 'theme',
        vars: { '--background': '#ff0000' },
      },
    });
  }
  assert.equal(docEl.style.props['--background'], undefined, 'Foreign postMessage source must be ignored');

  // Send theme + observatory frame messages from parent into opaque sandbox
  for (const fn of winListeners.message || []) {
    await fn({
      origin: 'null',
      source: parentWin,
      data: {
        protocol: 'uor-observatory/1',
        op: 'theme',
        vars: {
          '--background': '#090d16',
          '--card': '#111827',
          '--foreground': '#f8fafc',
          '--border': '#334155',
          '--font-sans': 'Inter, sans-serif',
          '--font-mono': 'JetBrains Mono, monospace',
        },
      },
    });
    await fn({
      origin: 'null',
      source: parentWin,
      data: {
        protocol: 'uor-observatory/1',
        op: obsKey,
        snapshot: {
          tokens_emitted: 64,
          tok_per_sec: 48.2,
          prefix_reused_tokens: 16,
          geometric_metrics: {
            flock_attended_positions: 48,
            flock_total_positions: 128,
            nibble_lut_lookups: 4096,
            lorentz_geodesic_evals: 256,
            hamming_xor_copy_hits: 9,
          },
          holo_v13: {
            model_id: 'uor-r4.qwen3-0.6b-holo',
            piece_count: 410,
            cached_hits: 24,
          },
          uorc: {
            generation: 7,
            verdict: 'TelosAdmitted',
            loss_total: 12,
            tuned_axis: 'shortlist_k',
            w5h_complete: true,
          },
        },
      },
    });
  }

  assert.equal(docEl.style.props['--background'], '#090d16');
  assert.equal(docEl.style.props['--font-mono'], 'JetBrains Mono, monospace');
  assert.ok(els.get('holo-obs-speed-stat').textContent.includes('48.2 tok/s'));
  assert.ok(els.get('holo-obs-backend-stat').textContent.includes('uorc'));
  for (const [cid, count] of Object.entries(drawCalls)) {
    assert.ok(count > 0, `Canvas ${cid} must draw live metrics in Observatory mode`);
  }

  // Real HTML DOM + CSS cascade/specificity evaluator for ?surface=observatory viewport visibility
  const rawHtml = fs.readFileSync(path.join(ROOT, 'public', 'index.html'), 'utf8');
  const cssComputed = evaluateObservatoryCssCascade(rawHtml, appCss);
  assert.equal(
    cssComputed.get('body').display === 'none',
    false,
    'body must remain visible under ?surface=observatory',
  );
  for (const visibleTarget of [
    '#inspectors-modal',
    '.inspectors-modal-dialog',
    '.inspectors-modal-body',
    '#canvas-panel-traces',
    '#holo-observatory-deck',
    '#holo-layer-waterfall-canvas',
    '#holo-shortlist-race-canvas',
    '#holo-causal-flock-canvas',
    '#holo-storage-geometry-canvas',
  ]) {
    const chain = cssComputed.ancestorChain(visibleTarget);
    assert.ok(chain.length > 0, `Missing target ${visibleTarget} in public/index.html`);
    for (const node of chain) {
      assert.notEqual(
        node.style.display,
        'none',
        `Ancestor <${node.tag} id="${node.id}" class="${node.classes.join(' ')}"> of ${visibleTarget} resolved to display: none (blank viewport!)`,
      );
    }
  }
  for (const resetSel of ['#inspectors-modal', '.inspectors-modal-dialog', '.inspectors-modal-body']) {
    const st = cssComputed.get(resetSel);
    assert.equal(st.display, 'block', `${resetSel} must resolve to display: block in Observatory mode`);
    assert.equal(st.position, 'static', `${resetSel} must reset position: static in Observatory mode`);
    assert.equal(st.inset, 'auto', `${resetSel} must reset inset: auto in Observatory mode`);
    assert.equal(st.width, '100%', `${resetSel} must reset width: 100% in Observatory mode`);
    assert.equal(st['max-width'], 'none', `${resetSel} must reset max-width: none in Observatory mode`);
    assert.equal(st.height, 'auto', `${resetSel} must reset height: auto in Observatory mode`);
    assert.equal(st['max-height'], 'none', `${resetSel} must reset max-height: none in Observatory mode`);
    assert.equal(st.border, '0', `${resetSel} must reset border: 0 in Observatory mode`);
    assert.equal(st['border-radius'], '0', `${resetSel} must reset border-radius: 0 in Observatory mode`);
    assert.equal(st['box-shadow'], 'none', `${resetSel} must reset box-shadow: none in Observatory mode`);
    assert.equal(st.background, 'transparent', `${resetSel} must reset background: transparent in Observatory mode`);
    assert.equal(st.padding, '0', `${resetSel} must reset padding: 0 in Observatory mode`);
  }
  assert.equal(cssComputed.get('#canvas-panel-traces').display, 'block');
  for (const hiddenTarget of [
    'main',
    '#inspectors-modal-backdrop',
    '.inspectors-modal-header',
    '#canvas-panel-inspector',
    '#neural-3d-viewport',
  ]) {
    assert.equal(
      cssComputed.get(hiddenTarget).display,
      'none',
      `${hiddenTarget} must resolve to display: none in Observatory mode`,
    );
  }

  // Important #2: initObservatorySurface() must run BEFORE initCursorIdeWorkbench() & emit ZERO prism:ready / uor-workspace-bridge/1
  const obsCallIdx = appJs.indexOf('const isObservatorySurface = initObservatorySurface();');
  const ideCallIdx = appJs.indexOf('initCursorIdeWorkbench();');
  assert.ok(
    obsCallIdx >= 0 && ideCallIdx > obsCallIdx,
    'initObservatorySurface() must execute BEFORE initCursorIdeWorkbench() and gate IDE workbench startup',
  );
  const embedFnStart = appJs.indexOf('function initEmbedMeBridge()');
  const embedFnEnd = appJs.indexOf('\nfunction initLayoutAndConnectedControls(', embedFnStart);
  assert.ok(embedFnStart >= 0 && embedFnEnd > embedFnStart);

  const bootPosted = [];
  const bootWinListeners = {};
  const bootDocListeners = {};
  const bootEls = new Map(els);
  const bootWindow = {
    origin: 'null',
    parent: { postMessage: (msg) => bootPosted.push(msg) },
    location: { search: '?surface=observatory', hash: '', href: 'holo://lexlean-r4/?surface=observatory' },
    getComputedStyle: () => ({ getPropertyValue: (n) => docEl.style.props[n] || '' }),
    addEventListener: (t, fn) => { (bootWinListeners[t] ||= []).push(fn); },
  };
  const bootDoc = {
    documentElement: docEl,
    body: new MockObsEl('body'),
    getElementById: (id) => bootEls.get(id) || null,
    addEventListener: (t, fn) => { (bootDocListeners[t] ||= []).push(fn); },
  };
  const bootHarness = [
    appJs.slice(stateStart, appJs.indexOf('\nfunction ', stateStart)),
    'function dispatchStudioEnvelope() { return {}; }',
    appJs.slice(obsStart, obsEnd),
    appJs.slice(surfStart, surfEnd),
    'function emitPrismEmbedEvent(type) { bootWindow.parent.postMessage({ type, protocol: "uor-workspace-bridge/1" }, "*"); }',
    'function getPrismEmbedStatusSnapshot(type) { return { type, protocol: "uor-workspace-bridge/1" }; }',
    appJs.slice(embedFnStart, embedFnEnd),
    'let ideBooted = false; function initCursorIdeWorkbench() { ideBooted = true; }',
    'const isObs = initObservatorySurface(); if (!isObs) { initCursorIdeWorkbench(); initEmbedMeBridge(); } else { initEmbedMeBridge(); }',
    'return { isObs, ideBooted };',
  ].join('\n');
  const bootOut = new Function('window', 'bootWindow', 'document', 'engineWorkerClient', 'AppState', bootHarness)(
    bootWindow,
    bootWindow,
    bootDoc,
    { enabled: false },
    { config: {} },
  );
  assert.equal(bootOut.isObs, true);
  assert.equal(bootOut.ideBooted, false, 'initCursorIdeWorkbench() must be skipped when ?surface=observatory');
  assert.ok(
    bootPosted.some((m) => m && m.protocol === 'uor-observatory/1' && m.op === 'ready'),
    'Observatory boot must emit uor-observatory/1 ready',
  );
  assert.equal(
    bootPosted.filter((m) => m && (m.type === 'prism:ready' || m.protocol === 'uor-workspace-bridge/1')).length,
    0,
    `?surface=observatory must emit ZERO prism:ready / uor-workspace-bridge/1 messages, got: ${JSON.stringify(bootPosted)}`,
  );
  assert.equal(
    (bootDocListeners.keydown || []).length,
    0,
    '?surface=observatory must not install modal Escape keydown handlers',
  );

  console.log('  ✓ ?surface=observatory real CSS cascade visibility, zero prism:ready, ready handshake & 4-canvas render verified.');
}

function evaluateObservatoryCssCascade(rawHtml, rawCss) {
  const voidTags = new Set(['meta', 'link', 'input', 'img', 'br', 'hr', 'source', 'path', 'line', 'circle', 'polyline', 'polygon', 'rect']);
  const rootNode = { tag: 'root', id: '', classes: [], attrs: {}, parent: null, children: [] };
  const stack = [rootNode];
  const tokenRe = /<!--[\s\S]*?-->|<script[\s\S]*?<\/script>|<style[\s\S]*?<\/style>|<\/([a-zA-Z0-9-]+)\s*>|<([a-zA-Z0-9-]+)([^>]*)>/g;
  let m;
  while ((m = tokenRe.exec(rawHtml)) !== null) {
    if (m[1]) {
      const closeTag = m[1].toLowerCase();
      while (stack.length > 1) {
        const popped = stack.pop();
        if (popped.tag === closeTag) break;
      }
      continue;
    }
    if (!m[2]) continue;
    const tag = m[2].toLowerCase();
    const attrStr = m[3] || '';
    const selfClosing = /\/\s*$/.test(attrStr) || voidTags.has(tag);
    const attrs = {};
    const attrRe = /([a-zA-Z0-9_-]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'>]+)))?/g;
    let am;
    while ((am = attrRe.exec(attrStr)) !== null) {
      attrs[am[1].toLowerCase()] = am[2] ?? am[3] ?? am[4] ?? '';
    }
    const el = {
      tag,
      id: attrs.id || '',
      classes: String(attrs.class || '').split(/\s+/).filter(Boolean),
      attrs,
      parent: stack[stack.length - 1],
      children: [],
    };
    stack[stack.length - 1].children.push(el);
    if (!selfClosing) stack.push(el);
  }

  const findFirst = (pred) => {
    const queue = [rootNode];
    while (queue.length) {
      const cur = queue.shift();
      if (cur !== rootNode && pred(cur)) return cur;
      for (const c of cur.children) queue.push(c);
    }
    return null;
  };
  const bodyEl = findFirst((e) => e.tag === 'body');
  bodyEl.classes.push('surface-observatory');
  bodyEl.attrs['data-surface'] = 'observatory';
  for (const id of ['inspectors-modal', 'canvas-panel-traces', 'holo-observatory-deck']) {
    const el = findFirst((e) => e.id === id);
    if (el) delete el.attrs.hidden;
  }

  const cleanCss = rawCss.replace(/\/\*[\s\S]*?\*\//g, '');
  const rules = [];
  let idx = 0;
  let order = 0;
  const splitTop = (str, sep) => {
    const out = [];
    let start = 0;
    let paren = 0;
    let brack = 0;
    let quote = '';
    for (let i = 0; i < str.length; i += 1) {
      const ch = str[i];
      if (quote) {
        if (ch === quote && str[i - 1] !== '\\') quote = '';
        continue;
      }
      if (ch === '"' || ch === "'") quote = ch;
      else if (ch === '(') paren += 1;
      else if (ch === ')') paren = Math.max(0, paren - 1);
      else if (ch === '[') brack += 1;
      else if (ch === ']') brack = Math.max(0, brack - 1);
      else if (ch === sep && paren === 0 && brack === 0) {
        out.push(str.slice(start, i));
        start = i + 1;
      }
    }
    out.push(str.slice(start));
    return out;
  };
  while (idx < cleanCss.length) {
    const open = cleanCss.indexOf('{', idx);
    if (open < 0) break;
    const header = cleanCss.slice(idx, open).trim();
    let depth = 1;
    let j = open + 1;
    while (j < cleanCss.length && depth > 0) {
      if (cleanCss[j] === '{') depth += 1;
      else if (cleanCss[j] === '}') depth -= 1;
      j += 1;
    }
    const body = cleanCss.slice(open + 1, j - 1);
    idx = j;
    if (!header || header.startsWith('@')) continue;
    const decls = [];
    for (const rawDecl of splitTop(body, ';')) {
      const colon = rawDecl.indexOf(':');
      if (colon < 0) continue;
      const prop = rawDecl.slice(0, colon).trim().toLowerCase();
      let val = rawDecl.slice(colon + 1).trim();
      const imp = /\s*!important\s*$/i.test(val);
      if (imp) val = val.replace(/\s*!important\s*$/i, '').trim();
      if (prop) decls.push({ prop, val, important: imp });
    }
    for (const sel of splitTop(header, ',')) {
      const trimmed = sel.trim();
      if (trimmed) rules.push({ sel: trimmed, decls, order: order++ });
    }
  }

  const parseCompound = (comp) => {
    if (comp.includes(':has(') || comp.includes(':hover') || comp.includes(':focus') || comp.includes('::')) {
      return null;
    }
    let rest = comp.trim();
    let tag = '*';
    const tagMatch = rest.match(/^([a-zA-Z0-9_-]+|\*)/);
    if (tagMatch) {
      tag = tagMatch[1].toLowerCase();
      rest = rest.slice(tagMatch[0].length);
    }
    const ids = [];
    const classes = [];
    const attrs = [];
    const nots = [];
    let p = 0;
    while (p < rest.length) {
      const ch = rest[p];
      if (ch === '#') {
        const mId = rest.slice(p + 1).match(/^[a-zA-Z0-9_-]+/);
        if (!mId) return null;
        ids.push(mId[0]);
        p += 1 + mId[0].length;
      } else if (ch === '.') {
        const mCls = rest.slice(p + 1).match(/^[a-zA-Z0-9_-]+/);
        if (!mCls) return null;
        classes.push(mCls[0]);
        p += 1 + mCls[0].length;
      } else if (ch === '[') {
        const end = rest.indexOf(']', p + 1);
        if (end < 0) return null;
        const inner = rest.slice(p + 1, end).trim();
        const eq = inner.indexOf('=');
        if (eq < 0) attrs.push({ name: inner.toLowerCase(), val: null });
        else {
          const name = inner.slice(0, eq).trim().toLowerCase();
          const v = inner.slice(eq + 1).trim().replace(/^["']|["']$/g, '');
          attrs.push({ name, val: v });
        }
        p = end + 1;
      } else if (rest.startsWith(':not(', p)) {
        let d = 1;
        let q = p + 5;
        while (q < rest.length && d > 0) {
          if (rest[q] === '(') d += 1;
          else if (rest[q] === ')') d -= 1;
          q += 1;
        }
        const parsedNot = parseCompound(rest.slice(p + 5, q - 1));
        if (!parsedNot) return null;
        nots.push(parsedNot);
        p = q;
      } else {
        return null;
      }
    }
    const spec = [
      ids.length + nots.reduce((a, n) => a + n.spec[0], 0),
      classes.length + attrs.length + nots.reduce((a, n) => a + n.spec[1], 0),
      (tag !== '*' ? 1 : 0) + nots.reduce((a, n) => a + n.spec[2], 0),
    ];
    return { tag, ids, classes, attrs, nots, spec };
  };

  const matchCompound = (el, parsed) => {
    if (!el || el === rootNode || !parsed) return false;
    if (parsed.tag !== '*' && el.tag !== parsed.tag) return false;
    for (const id of parsed.ids) if (el.id !== id) return false;
    for (const cls of parsed.classes) if (!el.classes.includes(cls)) return false;
    for (const a of parsed.attrs) {
      if (!Object.prototype.hasOwnProperty.call(el.attrs, a.name)) return false;
      if (a.val !== null && el.attrs[a.name] !== a.val) return false;
    }
    for (const n of parsed.nots) if (matchCompound(el, n)) return false;
    return true;
  };

  const parseComplex = (sel) => {
    const tokens = [];
    let i = 0;
    let pendingComb = null;
    while (i < sel.length) {
      while (i < sel.length && /\s/.test(sel[i])) {
        if (!pendingComb && tokens.length > 0) pendingComb = ' ';
        i += 1;
      }
      if (i >= sel.length) break;
      if (sel[i] === '>') {
        pendingComb = '>';
        i += 1;
        continue;
      }
      if (sel[i] === '+' || sel[i] === '~') return null;
      let start = i;
      let paren = 0;
      let brack = 0;
      while (i < sel.length) {
        const ch = sel[i];
        if (ch === '(') paren += 1;
        else if (ch === ')') paren = Math.max(0, paren - 1);
        else if (ch === '[') brack += 1;
        else if (ch === ']') brack = Math.max(0, brack - 1);
        else if (paren === 0 && brack === 0 && (/\s/.test(ch) || ch === '>' || ch === '+' || ch === '~')) break;
        i += 1;
      }
      const parsed = parseCompound(sel.slice(start, i));
      if (!parsed) return null;
      tokens.push({ comb: pendingComb, compound: parsed });
      pendingComb = null;
    }
    return tokens;
  };

  const matchComplex = (el, tokens) => {
    if (!tokens || tokens.length === 0) return false;
    let cur = el;
    if (!matchCompound(cur, tokens[tokens.length - 1].compound)) return false;
    for (let k = tokens.length - 1; k > 0; k -= 1) {
      const comb = tokens[k].comb;
      const targetComp = tokens[k - 1].compound;
      if (comb === '>') {
        cur = cur.parent;
        if (!matchCompound(cur, targetComp)) return false;
      } else {
        let anc = cur.parent;
        let matched = null;
        while (anc && anc !== rootNode) {
          if (matchCompound(anc, targetComp)) {
            matched = anc;
            break;
          }
          anc = anc.parent;
        }
        if (!matched) return false;
        cur = matched;
      }
    }
    return true;
  };

  const compiledRules = rules
    .map((r) => ({ ...r, tokens: parseComplex(r.sel) }))
    .filter((r) => r.tokens);

  const computeStyle = (el) => {
    const winner = {};
    const cmpSpec = (a, b) => a[0] - b[0] || a[1] - b[1] || a[2] - b[2];
    for (const r of compiledRules) {
      if (!matchComplex(el, r.tokens)) continue;
      const spec = r.tokens.reduce(
        (acc, t) => [acc[0] + t.compound.spec[0], acc[1] + t.compound.spec[1], acc[2] + t.compound.spec[2]],
        [0, 0, 0],
      );
      for (const d of r.decls) {
        const cur = winner[d.prop];
        const beats =
          !cur ||
          (d.important && !cur.important) ||
          (d.important === cur.important && (cmpSpec(spec, cur.spec) > 0 || (cmpSpec(spec, cur.spec) === 0 && r.order >= cur.order)));
        if (beats) {
          winner[d.prop] = { val: d.val, important: d.important, spec, order: r.order };
        }
      }
    }
    const out = {};
    for (const [k, v] of Object.entries(winner)) out[k] = v.val;
    return out;
  };

  const resolveBySelector = (targetSel) => {
    if (targetSel.startsWith('#')) return findFirst((e) => e.id === targetSel.slice(1));
    if (targetSel.startsWith('.')) return findFirst((e) => e.classes.includes(targetSel.slice(1)));
    return findFirst((e) => e.tag === targetSel);
  };

  return {
    get: (targetSel) => {
      const el = resolveBySelector(targetSel);
      assert.ok(el, `Selector ${targetSel} not found in public/index.html`);
      return computeStyle(el);
    },
    ancestorChain: (targetSel) => {
      const el = resolveBySelector(targetSel);
      if (!el) return [];
      const chain = [];
      for (let cur = el; cur && cur !== rootNode; cur = cur.parent) {
        chain.push({ tag: cur.tag, id: cur.id, classes: cur.classes, style: computeStyle(cur) });
      }
      return chain;
    },
  };
}

async function testSetPolicyAndWorkerBridgePolicyRejectsInsecureRelay() {
  console.log('4. Testing setPolicy & Worker set_bridge_policy/set_host_config reject insecure relay URLs...');
  const bridge = new SpiralCoreHostBridge({ mode: 'sandboxed' });
  const badRes = bridge.setPolicy({ relayWsUrl: 'ws://evil.example.com/spvs' });
  assert.equal(badRes.ok, false);
  assert.equal(badRes.error_code, 'InsecureTransportRejected');
  const badHttpRes = bridge.setPolicy({ relayUrl: 'http://10.0.0.1/spvs' });
  assert.equal(badHttpRes.ok, false);
  assert.equal(badHttpRes.error_code, 'InsecureTransportRejected');

  const prevWgs = globalThis.WorkerGlobalScope;
  const prevSelf = globalThis.self;
  const prevWbg = globalThis.__wbg_init;
  const prevPanic = globalThis.init_panic_hook;
  const prevBoot = globalThis.studio_bootstrap;
  const prevPack = globalThis.studio_weight_pack_status;
  const prevDispatch = globalThis.studio_dispatch;
  const rpcWaiters = new Map();
  let readyResolve;
  let readyTimer;
  const readyPromise = new Promise((resolve, reject) => {
    readyTimer = setTimeout(() => reject(new Error('Timeout waiting for Worker ready')), 2000);
    readyResolve = (msg) => {
      clearTimeout(readyTimer);
      resolve(msg);
    };
  });
  const waitForRpc = (id) =>
    new Promise((resolve, reject) => {
      const t = setTimeout(() => reject(new Error(`Timeout waiting for RPC ${id}`)), 2000);
      rpcWaiters.set(id, (msg) => {
        clearTimeout(t);
        resolve(msg);
      });
    });

  try {
    class MockWorkerGlobalScope {
      constructor() {
        this.name = 'uor-r4-engine-worker';
      }
    }
    const mockSelf = new MockWorkerGlobalScope();
    mockSelf.postMessage = (msg) => {
      if (msg && msg.event === 'ready') readyResolve(msg);
      if (msg && msg.id != null && typeof msg.ok === 'boolean' && rpcWaiters.has(msg.id)) {
        const cb = rpcWaiters.get(msg.id);
        rpcWaiters.delete(msg.id);
        cb(msg);
      }
    };
    globalThis.WorkerGlobalScope = MockWorkerGlobalScope;
    globalThis.self = mockSelf;
    globalThis.__wbg_init = async () => ({});
    globalThis.init_panic_hook = () => {};
    globalThis.studio_bootstrap = () => '{}';
    globalThis.studio_weight_pack_status = () => JSON.stringify({ mounted_packs: [] });
    globalThis.studio_dispatch = () => JSON.stringify({ status: 'ok', files: [] });

    await import('../../crates/wasm/js/worker_host_bridge.js?policy_relay_test=1');
    await readyPromise;

    const p1 = waitForRpc('policy-insecure-1');
    mockSelf.onmessage({
      data: { id: 'policy-insecure-1', op: 'set_bridge_policy', payload: { relayWsUrl: 'ws://evil.example.com/spvs' } },
    });
    const r1 = await p1;
    assert.equal(r1.ok, false, `set_bridge_policy must return ok:false on insecure relay, got: ${JSON.stringify(r1)}`);
    assert.equal(r1.error_code, 'InsecureTransportRejected');

    const p2 = waitForRpc('policy-insecure-2');
    mockSelf.onmessage({
      data: { id: 'policy-insecure-2', op: 'set_host_config', payload: { relayUrl: 'http://169.254.169.254/spvs' } },
    });
    const r2 = await p2;
    assert.equal(r2.ok, false, `set_host_config must return ok:false on insecure relay, got: ${JSON.stringify(r2)}`);
    assert.equal(r2.error_code, 'InsecureTransportRejected');
  } finally {
    clearTimeout(readyTimer);
    globalThis.WorkerGlobalScope = prevWgs;
    globalThis.self = prevSelf;
    globalThis.__wbg_init = prevWbg;
    globalThis.init_panic_hook = prevPanic;
    globalThis.studio_bootstrap = prevBoot;
    globalThis.studio_weight_pack_status = prevPack;
    globalThis.studio_dispatch = prevDispatch;
  }
  console.log('  ✓ setPolicy & Worker set_bridge_policy/set_host_config InsecureTransportRejected verified.');
}

async function testStudioBridgeConsentAndGitDeletions_R4_60() {
  console.log('5. Testing R4-60 off-origin consent (SecurityError/Q31 fallback), deletion-aware /workspace/.git sync & studio_stream_turn...');

  // 5a. Off-origin hydration consent with simulated SecurityError on localStorage + Pages 404 .bin shards + allowOffOriginHydration=true
  const prevLoc = globalThis.location;
  const prevFetch = globalThis.fetch;
  const prevIdb = globalThis.indexedDB;
  const prevLsDesc = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  const prevBeginMountV = globalThis.studio_begin_holo_v13_mount_verified;
  const prevBeginMount = globalThis.studio_begin_holo_v13_mount;
  const prevPushPiece = globalThis.studio_push_holo_v13_piece;
  const prevFinMount = globalThis.studio_finalize_holo_v13_mount;

  try {
    Object.defineProperty(globalThis, 'localStorage', {
      configurable: true,
      get() {
        const err = new Error('The operation is insecure (simulated opaque-origin SecurityError)');
        err.name = 'SecurityError';
        throw err;
      },
    });
    globalThis.indexedDB = undefined;
    globalThis.location = {
      origin: 'https://casey-allard.github.io',
      href: 'https://casey-allard.github.io/lexlean-r4/',
    };

    const piece0Bytes = new TextEncoder().encode('UORR4V13-test-piece-0');
    const piece0Sha = crypto.createHash('sha256').update(piece0Bytes).digest('hex');
    const testManifest = {
      format: 'UORR4V13',
      container_sha256: piece0Sha,
      total_bytes: 429916160,
      mirrors: ['https://raw.githubusercontent.com/Casey-allard/lexlean-r4/weights-v13-qwen3-0.6b/'],
      pieces: [{ index: 0, file: 'piece_0000.bin', sha256: piece0Sha, byte_length: piece0Bytes.byteLength }],
    };

    const offOriginHits = [];
    // Case A: untrusted custom external mirrors require ConsentRequiredForWeightDownload unless allowOffOriginHydration=true
    globalThis.fetch = async (url) => {
      const s = String(url || '');
      if (s.startsWith('https://untrusted.example/')) {
        offOriginHits.push(s);
      }
      return new Response('Not Found', { status: 404 });
    };
    await assert.rejects(
      () =>
        hydrate_holo_model('uor-r4.qwen3-0.6b-holo', () => {}, true, {
          customMirrors: ['https://untrusted.example/weights/'],
        }),
      (err) =>
        err &&
        err.code === 'ConsentRequiredForWeightDownload' &&
        err.model_id === 'uor-r4.qwen3-0.6b-holo' &&
        Number(err.approx_bytes) > 0 &&
        Array.isArray(err.mirrors) &&
        err.mirrors.some((m) => m.includes('untrusted.example')),
    );
    assert.equal(offOriginHits.length, 0, 'must not touch untrusted external mirror before explicit consent');

    // Case B: Pages deployment scenario — same-origin manifest.json returns 200 OK while piece_0000.bin returns 404!
    // Explicit allowOffOriginHydration: false still requests consent; default {} auto-hydrates first-party GitHub mirror!
    globalThis.fetch = async (url) => {
      const s = String(url || '');
      if (s.startsWith('https://casey-allard.github.io/') && s.endsWith('/manifest.json')) {
        return new Response(JSON.stringify(testManifest), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      }
      if (s.startsWith('https://raw.githubusercontent.com/')) {
        offOriginHits.push(s);
        if (s.endsWith('/manifest.json')) {
          return new Response(JSON.stringify(testManifest), {
            status: 200,
            headers: { 'content-type': 'application/json' },
          });
        }
        if (s.endsWith('/piece_0000.bin')) {
          return new Response(piece0Bytes, { status: 200 });
        }
      }
      return new Response('Not Found', { status: 404 });
    };

    await assert.rejects(
      () => hydrate_holo_model('uor-r4.qwen3-0.6b-holo', () => {}, true, { allowOffOriginHydration: false }),
      (err) =>
        err &&
        err.code === 'ConsentRequiredForWeightDownload' &&
        err.model_id === 'uor-r4.qwen3-0.6b-holo' &&
        Array.isArray(err.mirrors) &&
        err.mirrors.some((m) => m.includes('raw.githubusercontent.com')),
    );
    assert.equal(offOriginHits.length, 0, 'explicit allowOffOriginHydration:false must gate off-origin fetches');

    // Case C: default auto-hydration {} on Pages proceeds cleanly despite localStorage SecurityError & 404 same-origin pieces
    let mountedPieces = 0;
    globalThis.studio_begin_holo_v13_mount_verified = () => JSON.stringify({ status: 'ok' });
    globalThis.studio_begin_holo_v13_mount = () => JSON.stringify({ status: 'ok' });
    globalThis.studio_push_holo_v13_piece = () => {
      mountedPieces += 1;
      return JSON.stringify({ status: 'ok' });
    };
    globalThis.studio_finalize_holo_v13_mount = () =>
      JSON.stringify({ status: 'ok', model_id: 'uor-r4.qwen3-0.6b-holo', mounted: true });

    const hydrated = await hydrate_holo_model('uor-r4.qwen3-0.6b-holo', () => {}, true, {});
    assert.equal(hydrated.status, 'ok');
    assert.equal(mountedPieces, 1);
    assert.ok(offOriginHits.some((u) => u.endsWith('/piece_0000.bin')));
  } finally {
    globalThis.location = prevLoc;
    globalThis.fetch = prevFetch;
    globalThis.indexedDB = prevIdb;
    if (prevLsDesc) Object.defineProperty(globalThis, 'localStorage', prevLsDesc);
    else delete globalThis.localStorage;
    globalThis.studio_begin_holo_v13_mount_verified = prevBeginMountV;
    globalThis.studio_begin_holo_v13_mount = prevBeginMount;
    globalThis.studio_push_holo_v13_piece = prevPushPiece;
    globalThis.studio_finalize_holo_v13_mount = prevFinMount;
  }

  // 5b. SpiralCoreHostBridge clone -> .git/uor_state.json + .git/tree_index.json -> local delete -> git status / diff -> commit_push (sha: null)
  const postedTrees = [];
  const remoteFiles = new Map([
    ['README.md', '# Studio Repo\n'],
    ['src/keep.rs', 'pub fn keep() -> i32 { 1 }\n'],
    ['src/old.rs', 'pub fn old() -> i32 { 0 }\n'],
  ]);
  const gitMockFetch = async (url, init = {}) => {
    const u = new URL(String(url));
    assert.equal(u.hostname, 'api.github.com', 'GITHUB_TOKEN git calls must target api.github.com directly');
    const auth = init.headers?.Authorization || init.headers?.authorization || '';
    assert.equal(auth, 'Bearer ghp_studio_del_pat');

    if (u.pathname === '/repos/uor-test/studio-del') {
      return new Response(JSON.stringify({ default_branch: 'main', private: false, html_url: 'https://github.com/uor-test/studio-del' }), { status: 200 });
    }
    if (u.pathname === '/repos/uor-test/studio-del/branches') {
      return new Response(JSON.stringify([{ name: 'main' }]), { status: 200 });
    }
    if (u.pathname === '/repos/uor-test/studio-del/git/trees/main') {
      const tree = Array.from(remoteFiles.entries()).map(([p, c]) => ({
        path: p,
        type: 'blob',
        size: c.length,
        sha: `sha_${p}`,
      }));
      return new Response(JSON.stringify({ sha: 'commit_head_1', tree }), { status: 200 });
    }
    if (u.pathname.startsWith('/repos/uor-test/studio-del/git/blobs/')) {
      const sha = decodeURIComponent(u.pathname.split('/').pop() || '');
      const filePath = sha.replace(/^sha_/, '');
      const content = remoteFiles.get(filePath) ?? '';
      return new Response(
        JSON.stringify({ encoding: 'base64', content: Buffer.from(content, 'utf8').toString('base64'), size: content.length }),
        { status: 200 },
      );
    }
    if (u.pathname === '/repos/uor-test/studio-del/git/ref/heads/main' && (init.method || 'GET') === 'GET') {
      return new Response(JSON.stringify({ ref: 'refs/heads/main', object: { sha: 'commit_head_1' } }), { status: 200 });
    }
    if (u.pathname === '/repos/uor-test/studio-del/git/commits/commit_head_1') {
      return new Response(JSON.stringify({ sha: 'commit_head_1', tree: { sha: 'tree_base_1' } }), { status: 200 });
    }
    if (u.pathname === '/repos/uor-test/studio-del/git/trees' && init.method === 'POST') {
      const body = JSON.parse(String(init.body || '{}'));
      postedTrees.push(body);
      return new Response(JSON.stringify({ sha: 'tree_next_2' }), { status: 201 });
    }
    if (u.pathname === '/repos/uor-test/studio-del/git/commits' && init.method === 'POST') {
      return new Response(JSON.stringify({ sha: 'commit_next_2' }), { status: 201 });
    }
    if (u.pathname === '/repos/uor-test/studio-del/git/refs/heads/main' && init.method === 'PATCH') {
      return new Response(JSON.stringify({ ref: 'refs/heads/main', object: { sha: 'commit_next_2' } }), { status: 200 });
    }
    throw new Error(`Unexpected git mock URL: ${init.method || 'GET'} ${u.pathname}`);
  };

  const gitBridge = new SpiralCoreHostBridge({ fetchImpl: gitMockFetch, mode: 'sandboxed' });
  gitBridge.setSecret('GITHUB_TOKEN', 'ghp_studio_del_pat');

  const cloneRes = await gitBridge.execute({
    op: 'git',
    subcommand: 'clone',
    repo: 'uor-test/studio-del',
    branch: 'main',
  });
  assert.equal(cloneRes.ok, true);
  assert.equal(cloneRes.files.length, 3);
  const uorStateRaw = gitBridge.readWorkspaceFile('/workspace/.git/uor_state.json');
  const treeIndexRaw = gitBridge.readWorkspaceFile('/workspace/.git/tree_index.json');
  assert.ok(uorStateRaw && treeIndexRaw, 'clone must write /workspace/.git/uor_state.json and /workspace/.git/tree_index.json');
  const treeIndex = JSON.parse(treeIndexRaw);
  assert.ok(treeIndex.entries && treeIndex.entries['src/old.rs'], 'tree_index.json must track src/old.rs');

  // Local workspace mutation: remove src/old.rs, modify README.md, add src/new.rs
  gitBridge.deleteWorkspaceFile('src/old.rs');
  gitBridge.writeWorkspaceFile('README.md', '# Studio Repo v2\n');
  gitBridge.writeWorkspaceFile('src/new.rs', 'pub fn added() -> i32 { 7 }\n');

  const statusRes = await gitBridge.execute({ op: 'git', subcommand: 'status' });
  assert.equal(statusRes.ok, true, `git status failed: ${JSON.stringify(statusRes)}`);
  assert.ok(statusRes.deleted.includes('src/old.rs'), `git status must list deleted src/old.rs: ${JSON.stringify(statusRes)}`);
  assert.ok(statusRes.modified.includes('README.md'));
  assert.ok(statusRes.added.includes('src/new.rs'));
  assert.ok(statusRes.body_text.includes('deleted') && statusRes.body_text.includes('src/old.rs'));

  const diffRes = await gitBridge.execute({ op: 'git', subcommand: 'diff' });
  assert.equal(diffRes.ok, true, `git diff failed: ${JSON.stringify(diffRes)}`);
  assert.ok(diffRes.deleted.includes('src/old.rs'));
  assert.ok(diffRes.diff_text.includes('--- a/src/old.rs') && diffRes.diff_text.includes('+++ /dev/null'));

  const pushRes = await gitBridge.execute({
    op: 'git',
    subcommand: 'commit_push',
    repo: 'uor-test/studio-del',
    branch: 'main',
    message: 'Remove src/old.rs and update README',
  });
  assert.equal(pushRes.ok, true, `commit_push failed: ${JSON.stringify(pushRes)}`);
  assert.equal(pushRes.deleted_files, 1);
  assert.equal(postedTrees.length, 1);
  const delEntry = postedTrees[0].tree.find((e) => e.path === 'src/old.rs');
  assert.deepEqual(
    delEntry,
    { path: 'src/old.rs', mode: '100644', type: 'blob', sha: null },
    'commit_push must emit sha: null in git/trees payload for deleted tracked file',
  );
  const postPushStatus = await gitBridge.execute({ op: 'git', subcommand: 'status' });
  assert.equal(postPushStatus.clean, true, 'tree_index.json must reflect committed deletions and writes');

  // full_workspace commit_push when nothing changed must preserve GitNothingToPush
  const noChangePush = await gitBridge.execute({
    op: 'git',
    subcommand: 'commit_push',
    repo: 'uor-test/studio-del',
    branch: 'main',
    files: gitBridge.listWorkspaceFiles(),
    full_workspace: true,
    message: 'No-op push',
  });
  assert.equal(noChangePush.ok, false);
  assert.equal(noChangePush.error_code, 'GitNothingToPush');

  // Local git status via req.args works when webAccessEnabled is false
  gitBridge.webAccessEnabled = false;
  const offlineArgsStatus = await gitBridge.execute({ op: 'git', args: 'status' });
  assert.equal(offlineArgsStatus.ok, true, `webAccessEnabled=false git args status failed: ${JSON.stringify(offlineArgsStatus)}`);
  assert.equal(offlineArgsStatus.clean, true);
  gitBridge.webAccessEnabled = true;

  // git pull removes unmodified file deleted on remote and reports deleted_paths
  const pullMockFetch = async (url, init = {}) => {
    const u = new URL(String(url));
    if (u.pathname === '/repos/uor-test/studio-del' && (!init.method || init.method === 'GET')) {
      return new Response(JSON.stringify({ default_branch: 'main' }), { status: 200 });
    }
    if (u.pathname === '/repos/uor-test/studio-del/branches') {
      return new Response(JSON.stringify([{ name: 'main' }]), { status: 200 });
    }
    if (u.pathname === '/repos/uor-test/studio-del/git/trees/main') {
      return new Response(JSON.stringify({
        sha: 'tree_after_remote_del',
        tree: [
          { path: 'src/keep.rs', type: 'blob', sha: 'blob_keep', size: 22 },
          { path: 'src/new.rs', type: 'blob', sha: 'blob_new', size: 28 },
        ],
      }), { status: 200 });
    }
    if (u.pathname === '/repos/uor-test/studio-del/git/blobs/blob_keep') {
      return new Response(JSON.stringify({ encoding: 'base64', content: Buffer.from('pub fn keep() {}\n').toString('base64') }), { status: 200 });
    }
    if (u.pathname === '/repos/uor-test/studio-del/git/blobs/blob_new') {
      return new Response(JSON.stringify({ encoding: 'base64', content: Buffer.from('pub fn added() -> i32 { 7 }\n').toString('base64') }), { status: 200 });
    }
    throw new Error(`Unexpected pull mock URL: ${init.method || 'GET'} ${u.pathname}`);
  };
  gitBridge.fetchImpl = pullMockFetch;
  const pullRes = await gitBridge.execute({ op: 'git', subcommand: 'pull', repo: 'uor-test/studio-del', branch: 'main' });
  assert.equal(pullRes.ok, true, `git pull failed: ${JSON.stringify(pullRes)}`);
  assert.ok(Array.isArray(pullRes.deleted_paths) && pullRes.deleted_paths.includes('README.md'), 'git pull must report deleted_paths for remote removals');
  assert.equal(gitBridge.readWorkspaceFile('README.md'), null, 'git pull must delete unmodified remotely removed file');

  // 5c. execute_inprism_foundry_turn in surface_mode:"studio" routes preview_inspect, emits token_step per step,
  //     commits only closed <prism-write>, and discards unclosed <prism-write> on holo_cancel_foundry_turn
  const studioWs = new Map([['index.html', '<div id="app">hi</div>']]);
  const studioEvents = [];
  let previewCalled = 0;
  let cowFlushed = 0;
  let beginCounter = 0;
  let activeSess = null;
  const cancelledSessions = new Set();
  const studioStepperHooks = {
    holoRuntimeStatus: () =>
      JSON.stringify({
        qwen3_06b_mounted: true,
        qwen3_17b_mounted: false,
        models: [{ model_id: 'uor-r4.qwen3-0.6b-holo', container_sha256: 'c'.repeat(64) }],
      }),
    collectWorkspaceSnapshot: () => ({
      files: Array.from(studioWs.entries()).map(([p, c]) => ({ path: p, content: c })),
    }),
    dispatchBridge: async (env) => {
      if (env.includes('\tworkspace_write\t')) {
        const body = env.split('\t').slice(3).join('\t');
        const nl = body.indexOf('\n');
        const p = body.slice(0, nl).trim();
        const c = body.slice(nl + 1);
        studioWs.set(p, c);
        return { status: 'ok', files: Array.from(studioWs.entries()).map(([k, v]) => ({ path: k, content: v })) };
      }
      if (env.includes('\tworkspace_delete\t')) {
        const p = env.split('\t').slice(3).join('\t').trim();
        studioWs.delete(p);
        return { status: 'ok' };
      }
      return { status: 'ok', exit_code: 0, mvm_stdout: '' };
    },
    flushMvmCowToOpfs: async () => {
      cowFlushed += 1;
      return true;
    },
    previewInspect: async (args) => {
      previewCalled += 1;
      return { ok: true, selector: args.selector, dom_text: 'hi', console_errors: [] };
    },
    ensureIdentity: async () => ({ prism_id: 'did:prism:studio-test' }),
    signAction: async (action, targetKappa) => ({ action, targetKappa }),
    postEvent: (event, data) => studioEvents.push({ event, data }),
    holoBeginTurn: (payloadJson) => {
      const req = JSON.parse(payloadJson);
      assert.equal(req.surface_mode, 'studio');
      const tid = ++beginCounter;
      const hop = Array.isArray(req.tool_results) ? req.tool_results.length : 0;
      activeSess = {
        tid,
        step: 0,
        hop,
        prompt: req.prompt,
      };
      return JSON.stringify({ status: 'ready', turn_id: tid, model_id: 'uor-r4.qwen3-0.6b-holo', prefix_reused_tokens: 120 });
    },
    holoStepTurn: (tidBig) => {
      const tid = Number(tidBig);
      if (cancelledSessions.has(tid)) {
        activeSess = null;
        return JSON.stringify({ status: 'cancelled', turn_id: tid, delta: '', tokens_emitted: 0, tok_per_sec: 0 });
      }
      if (!activeSess || activeSess.tid !== tid) {
        return JSON.stringify({ status: 'error', error_code: 'TurnNotFound', message: 'not found' });
      }
      const s = activeSess.step++;
      if (activeSess.prompt.includes('CANCEL_MID_WRITE')) {
        if (s === 0) {
          return JSON.stringify({
            status: 'streaming',
            turn_id: tid,
            delta: '<prism-write path="src/aborted.rs">incomplete',
            tokens_emitted: 2,
            tok_per_sec: 44,
            observatory_frames: [{ step_idx: 0, token_text: '<prism-write', top_candidates: [], has_geometric_shortlist: true }],
          });
        }
        return JSON.stringify({
          status: 'done',
          turn_id: tid,
          delta: '</prism-write>',
          tokens_emitted: 3,
          tok_per_sec: 45,
        });
      }
      if (activeSess.hop === 0) {
        return JSON.stringify({
          status: 'needs_tool',
          turn_id: tid,
          delta: '<tool_call>{"name":"preview_inspect","arguments":{"selector":"#app"}}</tool_call>',
          tokens_emitted: 2,
          tok_per_sec: 50,
          observatory_frames: [{ step_idx: 0, token_text: 'inspect', top_candidates: [{ token_id: 1, token_text: 'inspect', logit_q16: 100, prob_permille: 1000, hamming_d: 12 }], has_geometric_shortlist: true }],
          tool_call: { name: 'preview_inspect', arguments: { selector: '#app' }, is_mutating: false },
        });
      }
      if (s === 0) {
        return JSON.stringify({
          status: 'streaming',
          turn_id: tid,
          delta: '<prism-write path="src/closed.rs">pub fn ok() -> i32 { ',
          tokens_emitted: 2,
          tok_per_sec: 60,
          observatory_frames: [{ step_idx: 1, token_text: 'pub', top_candidates: [{ token_id: 2, token_text: 'pub', logit_q16: 120, prob_permille: 1000, hamming_d: 10 }], has_geometric_shortlist: true }],
        });
      }
      return JSON.stringify({
        status: 'done',
        turn_id: tid,
        delta: '42 }\n</prism-write>',
        tokens_emitted: 4,
        tok_per_sec: 64,
        observatory_frames: [{ step_idx: 2, token_text: '42', top_candidates: [{ token_id: 3, token_text: '42', logit_q16: 140, prob_permille: 1000, hamming_d: 8 }], has_geometric_shortlist: true }],
        receipt_sha256: 'd'.repeat(64),
      });
    },
    holoCancelTurn: (tidBig) => {
      const tid = Number(tidBig);
      cancelledSessions.add(tid);
      activeSess = null;
      return JSON.stringify({ status: 'cancelled', turn_id: tid, kv_checkpoint_len: 120 });
    },
  };

  const studioRes = await execute_inprism_foundry_turn(
    { id: 'studio-turn-1', payload: { prompt: 'Inspect preview and write closed.rs', surface_mode: 'studio', live_hud: true } },
    studioStepperHooks,
  );
  assert.equal(studioRes.ok, true, `studio turn failed: ${JSON.stringify(studioRes)}`);
  assert.equal(previewCalled, 1, 'preview_inspect tool hop must be routed cleanly');
  assert.equal(studioWs.get('src/closed.rs'), 'pub fn ok() -> i32 { 42 }\n');
  assert.ok(cowFlushed >= 1, 'closed <prism-write> must flush OPFS CoW');
  const tokenSteps = studioEvents.filter((e) => e.event === 'token_step');
  assert.ok(tokenSteps.length >= 3, `expected per-step token_step events, got ${tokenSteps.length}`);
  assert.ok(tokenSteps.every((e) => e.data.turn_id != null && 'frame' in e.data && 'delta' in e.data && 'tok_per_sec' in e.data));
  const fsChanged = studioEvents.find((e) => e.event === 'workspace_fs_changed');
  assert.ok(fsChanged && fsChanged.data.changed_paths.includes('src/closed.rs'), 'must emit workspace_fs_changed for closed <prism-write>');

  // Unclosed <prism-write> cancel must NOT write src/aborted.rs
  let cancelTriggered = false;
  const cancelHooks = {
    ...studioStepperHooks,
    isCancelled: (_owner, tid) => (tid != null && cancelledSessions.has(Number(tid))) || cancelTriggered,
    postEvent: (event, data) => {
      studioEvents.push({ event, data });
      if (event === 'token_step' && String(data.delta || '').includes('<prism-write path="src/aborted.rs">')) {
        cancelTriggered = true;
        cancelHooks.holoCancelTurn(data.turn_id);
      }
    },
  };
  const cancelRes = await execute_inprism_foundry_turn(
    { id: 'studio-turn-cancel', payload: { prompt: 'CANCEL_MID_WRITE', surface_mode: 'studio', live_hud: true } },
    cancelHooks,
  );
  assert.equal(cancelRes.ok, false);
  assert.equal(cancelRes.cancelled, true);
  assert.equal(studioWs.has('src/aborted.rs'), false, 'unclosed <prism-write> must NEVER be written when cancelled');

  // Fix Round 1 checks: live_hud:false suppresses token_step, github_git op/repo preservation, plan-mode refusal without is_mutating, git pull deleted_paths without redundant writes
  const seenBridgeReqs = [];
  const writtenPathsInTurn = [];
  const edgeTurnEvents = [];
  let edgeMode = 'github_op_preserve';
  let edgeStep = 0;
  let edgeHop = 0;
  const edgeHooks = {
    ...studioStepperHooks,
    postEvent: (event, data) => edgeTurnEvents.push({ event, data }),
    dispatchBridge: async (env) => {
      if (env.includes('\tworkspace_write\t')) {
        const body = env.split('\t').slice(3).join('\t');
        const p = body.slice(0, body.indexOf('\n')).trim();
        writtenPathsInTurn.push(p);
      }
      return studioStepperHooks.dispatchBridge(env);
    },
    executeHostBridge: async (bridgeReq) => {
      seenBridgeReqs.push(bridgeReq);
      if (bridgeReq.subcommand === 'pull') {
        studioWs.delete('src/closed.rs');
        return {
          ok: true,
          op: 'git',
          subcommand: 'pull',
          files: [{ path: 'src/closed.rs_ignored_unchanged', content: '' }].slice(0, 0).concat([
            { path: 'index.html', content: studioWs.get('index.html') },
          ]),
          deleted_paths: ['src/closed.rs'],
          body_text: 'Pulled 1 unchanged, 1 deleted',
        };
      }
      return { ok: true, op: 'git', subcommand: bridgeReq.subcommand || 'status', clean: true, body_text: 'clean' };
    },
    holoBeginTurn: (payloadJson) => {
      const req = JSON.parse(payloadJson);
      edgeHop = Array.isArray(req.tool_results) ? req.tool_results.length : 0;
      edgeStep = 0;
      return JSON.stringify({ status: 'ready', turn_id: 900 + beginCounter++, model_id: 'uor-r4.qwen3-0.6b-holo' });
    },
    holoStepTurn: (tidBig) => {
      const tid = Number(tidBig);
      edgeStep += 1;
      if (edgeMode === 'github_op_preserve') {
        if (edgeHop === 0) {
          return JSON.stringify({
            status: 'needs_tool',
            turn_id: tid,
            delta: '',
            tool_call: { name: 'github_git', arguments: { op: 'rogue_op', action: 'status', repo: '' } },
          });
        }
        return JSON.stringify({ status: 'done', turn_id: tid, delta: 'Git status clean' });
      }
      if (edgeMode === 'plan_write') {
        return JSON.stringify({
          status: 'needs_tool',
          turn_id: tid,
          delta: '',
          tool_call: { name: 'workspace_write', arguments: { path: 'src/blocked.rs', content: 'bad' } },
        });
      }
      if (edgeMode === 'plan_git_push') {
        return JSON.stringify({
          status: 'needs_tool',
          turn_id: tid,
          delta: '',
          tool_call: { name: 'github_git', arguments: { action: 'commit_push', message: 'nope' } },
        });
      }
      if (edgeMode === 'plan_mvm_rm') {
        return JSON.stringify({
          status: 'needs_tool',
          turn_id: tid,
          delta: '',
          tool_call: { name: 'mvm_exec', cmd: 'rm -f index.html', arguments: { cmd: 'rm -f index.html' } },
        });
      }
      if (edgeMode === 'pull_sync') {
        if (edgeHop === 0) {
          return JSON.stringify({
            status: 'needs_tool',
            turn_id: tid,
            delta: '',
            tool_call: { name: 'github_git', arguments: { subcommand: 'pull', repo: 'uor-test/active-repo', branch: 'main' } },
          });
        }
        return JSON.stringify({ status: 'done', turn_id: tid, delta: 'Pulled clean' });
      }
      return JSON.stringify({ status: 'done', turn_id: tid, delta: 'ok' });
    },
  };

  edgeMode = 'github_op_preserve';
  const ghPreserveRes = await execute_inprism_foundry_turn(
    { id: 'edge-gh-preserve', payload: { prompt: 'Check git status', surface_mode: 'studio', composer_mode: 'agent', live_hud: false, repo: 'uor-test/active-repo', branch: 'dev' } },
    edgeHooks,
  );
  assert.equal(ghPreserveRes.ok, true);
  assert.equal(seenBridgeReqs[0].op, 'git', 'github_git must not allow toolArgs.op to clobber op: "git"');
  assert.equal(seenBridgeReqs[0].repo, 'uor-test/active-repo', 'github_git must preserve active repo when toolArgs.repo is empty');
  assert.equal(seenBridgeReqs[0].branch, 'dev');
  assert.equal(edgeTurnEvents.filter((e) => e.event === 'token_step').length, 0, 'live_hud: false in studio turn must suppress token_step events');

  const origEdgeHoloStepTurn = edgeHooks.holoStepTurn;
  for (const [modeName, label] of [
    ['plan_write', 'workspace_write'],
    ['plan_git_push', 'github_git commit_push'],
    ['plan_mvm_rm', 'mvm_exec rm'],
    ['plan_mvm_newline_rm', 'mvm_exec newline rm'],
  ]) {
    if (modeName === 'plan_mvm_newline_rm') {
      edgeHooks.holoStepTurn = (tidBig) =>
        JSON.stringify({
          status: 'needs_tool',
          turn_id: Number(tidBig),
          delta: '',
          tool_call: { name: 'mvm_exec', cmd: 'ls\nrm -f index.html', arguments: { cmd: 'ls\nrm -f index.html' } },
        });
    } else {
      edgeHooks.holoStepTurn = origEdgeHoloStepTurn;
    }
    edgeMode = modeName;
    const planRefusal = await execute_inprism_foundry_turn(
      { id: `edge-${modeName}`, payload: { prompt: 'Inspect only', surface_mode: 'studio', composer_mode: 'plan' } },
      edgeHooks,
    );
    assert.equal(planRefusal.ok, false, `plan mode must refuse ${label}`);
    assert.equal(planRefusal.error_code, 'RefusedPlanModeMutation', `expected RefusedPlanModeMutation for ${label}, got ${JSON.stringify(planRefusal)}`);
  }
  edgeHooks.holoStepTurn = origEdgeHoloStepTurn;

  edgeMode = 'pull_sync';
  edgeTurnEvents.length = 0;
  writtenPathsInTurn.length = 0;
  const pullTurnRes = await execute_inprism_foundry_turn(
    { id: 'edge-pull-sync', payload: { prompt: 'Pull latest', surface_mode: 'studio', composer_mode: 'agent', live_hud: false } },
    edgeHooks,
  );
  assert.equal(pullTurnRes.ok, true);
  assert.equal(writtenPathsInTurn.length, 0, 'github_git pull must not redundantly rewrite unchanged/already-written files');
  const pullFsEvt = edgeTurnEvents.find((e) => e.event === 'workspace_fs_changed');
  assert.ok(pullFsEvt && Array.isArray(pullFsEvt.data.deleted_paths) && pullFsEvt.data.deleted_paths.includes('src/closed.rs'), 'github_git pull must propagate deleted_paths into workspace_fs_changed');

  // 5d. Full Worker RPC test: studio_stream_turn + studio_cancel_turn + preview_inspect_reply + studio_cmdk_edit + studio_ghost_complete + boot Q31 fallback
  const prevWgs = globalThis.WorkerGlobalScope;
  const prevSelf = globalThis.self;
  const prevWbg = globalThis.__wbg_init;
  const prevPanic = globalThis.init_panic_hook;
  const prevBoot = globalThis.studio_bootstrap;
  const prevPack = globalThis.studio_weight_pack_status;
  const prevDispatch = globalThis.studio_dispatch;
  const prevHoloStatus = globalThis.studio_holo_runtime_status;
  const prevQ31Status = globalThis.studio_native_q31_status;
  const prevBegin = globalThis.holo_begin_foundry_turn;
  const prevStep = globalThis.holo_step_foundry_turn;
  const prevCancel = globalThis.holo_cancel_foundry_turn;
  const prevLocW = globalThis.location;
  const prevFetchW = globalThis.fetch;
  const prevIdbW = globalThis.indexedDB;
  const prevLsWDesc = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');

  const workerEvents = [];
  const rpcWaiters = new Map();
  const waitForRpc = (id) =>
    new Promise((resolve, reject) => {
      const t = setTimeout(() => reject(new Error(`Timeout waiting for Worker RPC ${id}`)), 3000);
      rpcWaiters.set(id, (msg) => {
        clearTimeout(t);
        resolve(msg);
      });
    });

  let readyResolve;
  let bootFallbackResolve;
  const readyPromise = new Promise((r) => { readyResolve = r; });
  const bootFallbackPromise = new Promise((resolve, reject) => {
    const t = setTimeout(() => reject(new Error('Timeout waiting for Q31 boot fallback holo_ready/hydration_consent_required')), 3000);
    bootFallbackResolve = (msg) => {
      clearTimeout(t);
      resolve(msg);
    };
  });

  try {
    Object.defineProperty(globalThis, 'localStorage', {
      configurable: true,
      get() {
        const err = new Error('Storage disabled');
        err.name = 'SecurityError';
        throw err;
      },
    });
    globalThis.indexedDB = undefined;
    globalThis.location = { origin: 'https://casey-allard.github.io', href: 'https://casey-allard.github.io/lexlean-r4/' };
    globalThis.fetch = async () => new Response('Not Found', { status: 404 });

    class MockStudioWorkerScope {
      constructor() {
        this.name = 'uor-r4-engine-worker';
      }
      postMessage(msg) {
        workerEvents.push(msg);
        if (msg && msg.event === 'ready') readyResolve(msg);
        if (msg && msg.event === 'holo_ready' && msg.recommendedModel === 'uor-r4.native-q31-d85') {
          bootFallbackResolve(msg);
        }
        if (msg && msg.event === 'preview_inspect_request') {
          this.onmessage({
            data: {
              id: `reply-${msg.rpc_id}`,
              op: 'preview_inspect_reply',
              payload: { rpc_id: msg.rpc_id, ok: true, summary: 'DOM healthy (#app)' },
            },
          });
        }
        if (msg && msg.id != null && typeof msg.ok === 'boolean' && rpcWaiters.has(msg.id)) {
          const cb = rpcWaiters.get(msg.id);
          rpcWaiters.delete(msg.id);
          cb(msg);
        }
      }
    }
    const mockSelf = new MockStudioWorkerScope();
    globalThis.WorkerGlobalScope = MockStudioWorkerScope;
    globalThis.self = mockSelf;
    globalThis.__wbg_init = async () => ({});
    globalThis.init_panic_hook = () => {};
    globalThis.studio_bootstrap = () => '{}';
    globalThis.studio_weight_pack_status = () => JSON.stringify({ mounted_packs: [] });

    const workerWs = new Map([['src/main.rs', 'fn main() {}\n']]);
    const workerBeginPayloads = [];
    let wTurnSeq = 0;
    let wActive = null;
    const wCancelled = new Set();

    globalThis.studio_dispatch = (env) => {
      if (env.includes('\tstate\t')) return JSON.stringify({ status: 'ok' });
      if (env.includes('\tworkspace_list\t')) {
        return JSON.stringify({
          status: 'ok',
          files: Array.from(workerWs.entries()).map(([p, c]) => ({ path: p, content: c })),
        });
      }
      if (env.includes('\tworkspace_read\t')) {
        const p = env.split('\t').slice(3).join('\t').trim();
        return workerWs.has(p)
          ? JSON.stringify({ status: 'ok', path: p, content: workerWs.get(p) })
          : JSON.stringify({ status: 'error', reason: 'FileNotFound' });
      }
      if (env.includes('\tworkspace_write\t')) {
        const body = env.split('\t').slice(3).join('\t');
        const nl = body.indexOf('\n');
        const p = body.slice(0, nl).trim();
        workerWs.set(p, body.slice(nl + 1));
        return JSON.stringify({ status: 'ok' });
      }
      if (env.includes('\tworkspace_delete\t')) {
        const p = env.split('\t').slice(3).join('\t').trim();
        workerWs.delete(p);
        return JSON.stringify({ status: 'ok' });
      }
      return JSON.stringify({ status: 'ok', exit_code: 0, mvm_stdout: '' });
    };
    globalThis.studio_holo_runtime_status = () =>
      JSON.stringify({ qwen3_06b_mounted: false, qwen3_17b_mounted: false, models: [] });
    globalThis.studio_native_q31_status = () =>
      JSON.stringify({ status: 'ok', model_id: 'uor-r4.native-q31-d85', ready: true });
    globalThis.holo_begin_foundry_turn = (payloadJson) => {
      const req = JSON.parse(payloadJson);
      workerBeginPayloads.push(req);
      const tid = ++wTurnSeq;
      wActive = { tid, req, step: 0, hop: Array.isArray(req.tool_results) ? req.tool_results.length : 0 };
      return JSON.stringify({ status: 'ready', turn_id: tid, model_id: req.selected_model || 'uor-r4.native-q31-d85' });
    };
    globalThis.holo_step_foundry_turn = (tidBig) => {
      const tid = Number(tidBig);
      if (wCancelled.has(tid)) {
        wActive = null;
        return JSON.stringify({ status: 'cancelled', turn_id: tid, delta: '', tokens_emitted: 0, tok_per_sec: 0 });
      }
      const s = wActive.step++;
      if (wActive.req.prompt.includes('[Cmd+K Inline Edit')) {
        return JSON.stringify({
          status: 'done',
          turn_id: tid,
          delta: 'fn main() { println!("cmdk"); }\n',
          tokens_emitted: 6,
          tok_per_sec: 30,
          receipt_sha256: 'e'.repeat(64),
        });
      }
      if (wActive.req.prompt.includes('[Ghost Complete')) {
        return JSON.stringify({
          status: 'done',
          turn_id: tid,
          delta: ' -> i32 { 0 }',
          tokens_emitted: 4,
          tok_per_sec: 35,
          receipt_sha256: 'f'.repeat(64),
        });
      }
      if (wActive.req.prompt.includes('WORKER_CANCEL_TEST')) {
        if (s === 0) {
          return JSON.stringify({ status: 'streaming', turn_id: tid, delta: '<prism-write path="src/never.rs">partial', tokens_emitted: 2, tok_per_sec: 20 });
        }
        return JSON.stringify({ status: 'done', turn_id: tid, delta: '</prism-write>', tokens_emitted: 3, tok_per_sec: 20 });
      }
      if (wActive.hop === 0) {
        return JSON.stringify({
          status: 'needs_tool',
          turn_id: tid,
          delta: '<tool_call>{"name":"preview_inspect","arguments":{"selector":"#app"}}</tool_call>',
          tokens_emitted: 2,
          tok_per_sec: 25,
          tool_call: { name: 'preview_inspect', arguments: { selector: '#app' }, is_mutating: false },
        });
      }
      return JSON.stringify({
        status: 'done',
        turn_id: tid,
        delta: 'Preview OK: DOM healthy',
        tokens_emitted: 5,
        tok_per_sec: 28,
        receipt_sha256: '1'.repeat(64),
      });
    };
    globalThis.holo_cancel_foundry_turn = (tidBig) => {
      const tid = Number(tidBig);
      wCancelled.add(tid);
      wActive = null;
      return JSON.stringify({ status: 'cancelled', turn_id: tid, kv_checkpoint_len: 0 });
    };

    await import('../../crates/wasm/js/worker_host_bridge.js?studio_r4_60_test=1');
    await readyPromise;
    const bootFallback = await bootFallbackPromise;
    assert.equal(bootFallback.recommendedModel, 'uor-r4.native-q31-d85');
    assert.equal(
      workerEvents.filter((e) => e && e.event === 'hydration_consent_required').length,
      0,
      'R4-62 zero-prompt auto-hydration must not emit hydration_consent_required for first-party catalog mirrors',
    );

    // Offline Q31 fallback via studio_stream_turn + preview_inspect_reply roundtrip
    const streamP = waitForRpc('rpc-studio-stream-1');
    mockSelf.onmessage({
      data: {
        id: 'rpc-studio-stream-1',
        op: 'studio_stream_turn',
        payload: { prompt: 'Check preview', offlineFallback: true, liveHud: true },
      },
    });
    const streamRes = await streamP;
    assert.equal(streamRes.ok, true, `studio_stream_turn failed: ${JSON.stringify(streamRes)}`);
    assert.equal(streamRes.result.model_id, 'uor-r4.native-q31-d85');
    assert.ok(streamRes.result.reply.includes('Preview OK'));

    // studio_cancel_turn discards unclosed <prism-write>
    const cancelTurnP = waitForRpc('rpc-studio-cancel-target');
    mockSelf.onmessage({
      data: {
        id: 'rpc-studio-cancel-target',
        op: 'studio_stream_turn',
        payload: { prompt: 'WORKER_CANCEL_TEST', offlineFallback: true },
      },
    });
    await new Promise((r) => setTimeout(r, 0));
    const cancelAckP = waitForRpc('rpc-studio-cancel-ack');
    mockSelf.onmessage({
      data: { id: 'rpc-studio-cancel-ack', op: 'studio_cancel_turn', payload: { targetId: 'rpc-studio-cancel-target' } },
    });
    const cancelAck = await cancelAckP;
    const cancelledTurn = await cancelTurnP;
    assert.equal(cancelAck.ok, true);
    assert.equal(cancelledTurn.cancelled, true);
    assert.equal(workerWs.has('src/never.rs'), false);

    // studio_cmdk_edit (max 192 tokens) & studio_ghost_complete (max 48 tokens)
    const cmdkP = waitForRpc('rpc-cmdk-1');
    mockSelf.onmessage({
      data: {
        id: 'rpc-cmdk-1',
        op: 'studio_cmdk_edit',
        payload: { path: 'src/main.rs', code: 'fn main() {}\n', instruction: 'Log cmdk' },
      },
    });
    const cmdkRes = await cmdkP;
    assert.equal(cmdkRes.ok, true, `studio_cmdk_edit failed: ${JSON.stringify(cmdkRes)}`);
    assert.equal(cmdkRes.result.status, 'ok');
    assert.ok(cmdkRes.result.diff_hunk.includes('cmdk'));
    assert.equal(cmdkRes.result.receipt_sha256, 'e'.repeat(64));
    assert.ok(workerBeginPayloads.some((r) => r.prompt.includes('[Cmd+K Inline Edit') && r.max_new_tokens <= 192));
    assert.equal(workerWs.get('src/main.rs'), 'fn main() {}\n', 'studio_cmdk_edit must return diff_hunk without mutating workspace before Accept');

    const ghostP = waitForRpc('rpc-ghost-1');
    mockSelf.onmessage({
      data: {
        id: 'rpc-ghost-1',
        op: 'studio_ghost_complete',
        payload: { path: 'src/lib.rs', prefix: 'pub fn zero()' },
      },
    });
    const ghostRes = await ghostP;
    assert.equal(ghostRes.ok, true, `studio_ghost_complete failed: ${JSON.stringify(ghostRes)}`);
    assert.equal(ghostRes.result.status, 'ok');
    assert.ok(ghostRes.result.suggestion.includes('-> i32'));
    assert.equal(ghostRes.result.receipt_sha256, 'f'.repeat(64));
    assert.ok(workerBeginPayloads.some((r) => r.prompt.includes('[Ghost Complete') && r.max_new_tokens <= 48));

    // Fix Round 1 Worker checks:
    // 1) studio_stream_turn WITHOUT offlineFallback honors bootNativeQ31Fallback -> native-q31-d85
    const tokenStepsBeforeInline = workerEvents.filter((e) => e && e.event === 'token_step').length;
    const autoQ31P = waitForRpc('rpc-studio-auto-q31');
    mockSelf.onmessage({
      data: {
        id: 'rpc-studio-auto-q31',
        op: 'studio_stream_turn',
        payload: { prompt: 'Check preview auto Q31 fallback', liveHud: true },
      },
    });
    const autoQ31Res = await autoQ31P;
    assert.equal(autoQ31Res.ok, true, `auto Q31 studio_stream_turn failed: ${JSON.stringify(autoQ31Res)}`);
    assert.equal(autoQ31Res.result.model_id, 'uor-r4.native-q31-d85', 'studio_stream_turn must use bootNativeQ31Fallback without offlineFallback:true');
    assert.ok(workerEvents.filter((e) => e && e.event === 'token_step').length > tokenStepsBeforeInline, 'liveHud:true studio_stream_turn must emit token_step');

    // 2) ghost & cmdk suppress token_step (live_hud: false) and strip unclosed ```lang code fences
    const stepsPreSilent = workerEvents.filter((e) => e && e.event === 'token_step').length;
    const prevStepOrig = globalThis.holo_step_foundry_turn;
    globalThis.holo_step_foundry_turn = (tidBig) => {
      const tid = Number(tidBig);
      if (wActive && wActive.req.prompt.includes('[Cmd+K Inline Edit')) {
        return JSON.stringify({
          status: 'done',
          turn_id: tid,
          delta: '```rust\nfn main() { println!("unfenced_open"); }\n',
          tokens_emitted: 6,
          tok_per_sec: 30,
          receipt_sha256: 'e'.repeat(64),
        });
      }
      return prevStepOrig(tidBig);
    };
    const silentCmdkP = waitForRpc('rpc-cmdk-silent');
    mockSelf.onmessage({
      data: {
        id: 'rpc-cmdk-silent',
        op: 'studio_cmdk_edit',
        payload: { path: 'src/main.rs', code: 'fn main() {}\n', instruction: 'Unfenced' },
      },
    });
    const silentCmdkRes = await silentCmdkP;
    globalThis.holo_step_foundry_turn = prevStepOrig;
    assert.equal(silentCmdkRes.ok, true);
    assert.equal(silentCmdkRes.result.proposed_code, 'fn main() { println!("unfenced_open"); }');
    assert.equal(workerEvents.filter((e) => e && e.event === 'token_step').length, stepsPreSilent, 'studio_cmdk_edit must suppress token_step events');

    // 3) .git/* MVM persistence fallback across Worker reload (gitMetaFiles empty, MVM has .git/uor_state.json + .git/tree_index.json)
    workerWs.set('.git/uor_state.json', JSON.stringify({
      branch: 'main',
      remote_url: 'https://github.com/uor-test/persisted.git',
      head_oid: 'abc123',
      staged: {},
      head_tree: { 'src/main.rs': 'fn main() {}\n' },
      commits: [],
    }));
    workerWs.set('.git/tree_index.json', JSON.stringify({
      repo: 'uor-test/persisted',
      remote_url: 'https://github.com/uor-test/persisted.git',
      branch: 'main',
      head_sha: 'abc123',
      entries: {
        'src/main.rs': { sha: 'blob_main', size: 13, hydrated: true, content: 'fn main() {}\n' },
      },
    }));
    const persistedGitStatusP = waitForRpc('rpc-persisted-git-status');
    mockSelf.onmessage({
      data: { id: 'rpc-persisted-git-status', op: 'host_bridge_exec', payload: { op: 'git', subcommand: 'status' } },
    });
    const persistedGitStatus = await persistedGitStatusP;
    assert.equal(persistedGitStatus.ok, true);
    assert.equal(persistedGitStatus.result.repo, 'uor-test/persisted', 'workerWorkspaceFs.read must fall back to MVM workspace_read for .git/*');
    assert.equal(persistedGitStatus.result.clean, true);

    // 4) Stale 'declined' flag is purged & auto-hydrates on boot; only uor_offline_emergency_q31='1' skips boot hydration
    const emergencyStore = new Map([['uor_offline_emergency_q31', '1']]);
    Object.defineProperty(globalThis, 'localStorage', {
      configurable: true,
      value: {
        getItem: (k) => (emergencyStore.has(k) ? emergencyStore.get(k) : null),
        setItem: (k, v) => emergencyStore.set(k, String(v)),
        removeItem: (k) => emergencyStore.delete(k),
      },
    });
    const declinedEvents = [];
    const declinedFetchUrls = [];
    globalThis.fetch = async (url) => {
      declinedFetchUrls.push(String(url));
      return new Response('Should not fetch', { status: 404 });
    };
    let declinedReadyResolve;
    const declinedReadyP = new Promise((resolve, reject) => {
      const t = setTimeout(() => reject(new Error('Timeout waiting for emergency Q31 holo_ready')), 3000);
      declinedReadyResolve = (m) => { clearTimeout(t); resolve(m); };
    });
    const declinedScope = new MockStudioWorkerScope();
    declinedScope.postMessage = (m) => {
      declinedEvents.push(m);
      if (m && m.event === 'holo_ready' && m.recommendedModel === 'uor-r4.native-q31-d85') {
        declinedReadyResolve(m);
      }
    };
    globalThis.self = declinedScope;
    await import('../../crates/wasm/js/worker_host_bridge.js?studio_r4_62_emergency=1');
    const declinedHoloReady = await declinedReadyP;
    assert.equal(declinedHoloReady.fallbackReason, 'offline_emergency_mode');
    const declinedWeightFetches = declinedFetchUrls.filter((u) => !u.includes('provenance.json'));
    assert.equal(declinedWeightFetches.length, 0, `offline emergency mode must skip weight fetches at boot, saw: ${JSON.stringify(declinedFetchUrls)}`);
    assert.equal(declinedEvents.filter((e) => e && e.event === 'hydration_consent_required').length, 0, 'emergency boot must not emit hydration_consent_required');

    // 5) Real Dedicated Worker offlineEmergencyQ31 & stale 'declined' purge when typeof localStorage === 'undefined'
    const prevAllowFlag = globalThis.__uorAllowOffOriginHydration;
    try {
      delete globalThis.localStorage;
      delete globalThis.__uorAllowOffOriginHydration;
      assert.equal(typeof globalThis.localStorage, 'undefined', 'Dedicated Worker test must run with typeof localStorage === undefined');

      const memFetchUrls = [];
      globalThis.fetch = async (url) => {
        memFetchUrls.push(String(url));
        return new Response('Not Found', { status: 404 });
      };

      const memRpcWaiters = new Map();
      const waitForMemRpc = (id) =>
        new Promise((resolve, reject) => {
          const t = setTimeout(() => reject(new Error(`Timeout waiting for Worker RPC ${id}`)), 3000);
          memRpcWaiters.set(id, (m) => {
            clearTimeout(t);
            resolve(m);
          });
        });

      let memReadyResolve;
      const memReadyP = new Promise((resolve, reject) => {
        const t = setTimeout(() => reject(new Error('Timeout waiting for emergency-policy Worker ready')), 3000);
        memReadyResolve = (m) => {
          clearTimeout(t);
          resolve(m);
        };
      });
      const memScope = new MockStudioWorkerScope();
      memScope.postMessage = (m) => {
        if (m && m.event === 'ready') memReadyResolve(m);
        if (m && m.event === 'preview_inspect_request') {
          memScope.onmessage({
            data: {
              id: `reply-${m.rpc_id}`,
              op: 'preview_inspect_reply',
              payload: { rpc_id: m.rpc_id, ok: true, summary: 'DOM healthy (#app)' },
            },
          });
        }
        if (m && m.id != null && typeof m.ok === 'boolean' && memRpcWaiters.has(m.id)) {
          const cb = memRpcWaiters.get(m.id);
          memRpcWaiters.delete(m.id);
          cb(m);
        }
      };
      globalThis.self = memScope;
      await import('../../crates/wasm/js/worker_host_bridge.js?studio_r4_62_mem_policy=1');
      await memReadyP;

      const emergencyOnP = waitForMemRpc('mem-policy-emergency-on');
      memScope.onmessage({
        data: {
          id: 'mem-policy-emergency-on',
          op: 'set_bridge_policy',
          payload: { offlineEmergencyQ31: true, bootSync: true },
        },
      });
      const emergencyOnRes = await emergencyOnP;
      assert.equal(emergencyOnRes.ok, true, `set_bridge_policy offlineEmergencyQ31 failed: ${JSON.stringify(emergencyOnRes)}`);
      assert.equal(emergencyOnRes.result.offline_emergency_q31, true);

      const emergencyTurnP = waitForMemRpc('mem-turn-emergency');
      memScope.onmessage({
        data: {
          id: 'mem-turn-emergency',
          op: 'inprism_r4_turn',
          payload: { prompt: 'Run offline Q31 under undefined localStorage' },
        },
      });
      const emergencyTurnRes = await emergencyTurnP;
      assert.equal(emergencyTurnRes.ok, true, `inprism_r4_turn under offlineEmergencyQ31 must succeed with Q31, got: ${JSON.stringify(emergencyTurnRes)}`);
      assert.equal(emergencyTurnRes.result.model_id, 'uor-r4.native-q31-d85', 'offlineEmergencyQ31 must route unadorned inprism_r4_turn to uor-r4.native-q31-d85');

      const emergencyOffP = waitForMemRpc('mem-policy-emergency-off');
      memScope.onmessage({
        data: {
          id: 'mem-policy-emergency-off',
          op: 'set_bridge_policy',
          payload: { offlineEmergencyQ31: false, consentDeclined: false },
        },
      });
      const emergencyOffRes = await emergencyOffP;
      assert.equal(emergencyOffRes.ok, true);
      assert.equal(emergencyOffRes.result.offline_emergency_q31, false);

      memFetchUrls.length = 0;
      const autoTurnP = waitForMemRpc('mem-turn-auto');
      memScope.onmessage({
        data: {
          id: 'mem-turn-auto',
          op: 'inprism_r4_turn',
          payload: { prompt: 'Attempt first-party auto-hydration after clearing emergency mode' },
        },
      });
      const autoTurnRes = await autoTurnP;
      assert.notEqual(
        autoTurnRes.error_code,
        'ConsentRequiredForWeightDownload',
        `R4-62 auto-hydration must not throw ConsentRequiredForWeightDownload: ${JSON.stringify(autoTurnRes)}`,
      );
      assert.ok(
        memFetchUrls.some((u) => u.includes('raw.githubusercontent.com') || u.includes('huggingface.co')),
        `First-party turn must proceed directly to first-party mirror check, saw fetches: ${JSON.stringify(memFetchUrls)}`,
      );
    } finally {
      if (prevAllowFlag === undefined) delete globalThis.__uorAllowOffOriginHydration;
      else globalThis.__uorAllowOffOriginHydration = prevAllowFlag;
    }
  } finally {
    globalThis.WorkerGlobalScope = prevWgs;
    globalThis.self = prevSelf;
    globalThis.__wbg_init = prevWbg;
    globalThis.init_panic_hook = prevPanic;
    globalThis.studio_bootstrap = prevBoot;
    globalThis.studio_weight_pack_status = prevPack;
    globalThis.studio_dispatch = prevDispatch;
    globalThis.studio_holo_runtime_status = prevHoloStatus;
    globalThis.studio_native_q31_status = prevQ31Status;
    globalThis.holo_begin_foundry_turn = prevBegin;
    globalThis.holo_step_foundry_turn = prevStep;
    globalThis.holo_cancel_foundry_turn = prevCancel;
    globalThis.location = prevLocW;
    globalThis.fetch = prevFetchW;
    globalThis.indexedDB = prevIdbW;
    if (prevLsWDesc) Object.defineProperty(globalThis, 'localStorage', prevLsWDesc);
    else delete globalThis.localStorage;
  }

  console.log('  ✓ R4-60 off-origin consent (SecurityError + Q31 boot fallback), deletion-aware git sync & studio_stream_turn verified.');
}

async function main() {
  console.log('=== E2E: Prism capabilities/1 Network Gate, CORS-Direct GitHub, SpiralCore Failback & Observatory (R4-59/R4-60) ===');
  await testCapabilitiesGateAndSpiralCoreFailback();
  await testCorsDirectGithubGitDataAndDohSsrf();
  await testObservatoryChromelessHandshakeAndRender();
  await testSetPolicyAndWorkerBridgePolicyRejectsInsecureRelay();
  await testStudioBridgeConsentAndGitDeletions_R4_60();
  console.log('=== ALL R4-59/R4-60 OBSERVATORY & STUDIO BRIDGE E2E CHECKS PASSED ===');
}

main().catch((err) => {
  console.error('FOUNDRY OBSERVATORY E2E FAILED:', err);
  process.exit(1);
});
