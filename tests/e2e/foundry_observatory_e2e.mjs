import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';

import { SpiralCoreHostBridge } from '../../crates/wasm/js/worker_host_bridge.js';
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
    url: 'https://evil.example.com/jq.elf',
  });
  assert.equal(pkgDenied.ok, false);
  assert.equal(pkgDenied.error_code, 'CapabilityDenied');

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
  console.log('  ✓ ?surface=observatory chromeless layout, CSS theme sync, opaque-origin ready handshake & 4-canvas render verified.');
}

async function main() {
  console.log('=== E2E: Prism capabilities/1 Network Gate, CORS-Direct GitHub, SpiralCore Failback & Observatory (R4-59) ===');
  await testCapabilitiesGateAndSpiralCoreFailback();
  await testCorsDirectGithubGitDataAndDohSsrf();
  await testObservatoryChromelessHandshakeAndRender();
  console.log('=== ALL R4-59 OBSERVATORY & SPIRALCORE E2E CHECKS PASSED ===');
}

main().catch((err) => {
  console.error('FOUNDRY OBSERVATORY E2E FAILED:', err);
  process.exit(1);
});
