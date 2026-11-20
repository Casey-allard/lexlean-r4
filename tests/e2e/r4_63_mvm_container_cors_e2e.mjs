import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  SpiralCoreHostBridge,
  admit_public_cors_preemption,
  classifyShellCommandKind,
  admitInteractiveCommand,
  execute_inprism_foundry_turn,
  resolve_staged_mvm_command,
} from '../../crates/wasm/js/worker_host_bridge.js';
import {
  handleSpvsEnvelope,
  handleWorkerFetch,
} from '../../relay/worker.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../..');

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

async function testInteractiveCommandGatesAndStagedFlow() {
  console.log('=== Wave 2 (R4-63) Interactive Command Admission & Staged MVM Flow E2E ===');

  assert.equal(classifyShellCommandKind('ls -la /workspace'), 'ReadOnlyInspection');
  assert.equal(classifyShellCommandKind('git status'), 'ReadOnlyInspection');
  assert.equal(classifyShellCommandKind('rm -rf /workspace/tmp'), 'MutatingOrNetwork');
  assert.equal(classifyShellCommandKind('echo hi > /workspace/out.txt'), 'MutatingOrNetwork');

  assert.equal(admitInteractiveCommand('safe', 'agent', true, 'MutatingOrNetwork'), 'AutoRunAdmitted');
  assert.equal(admitInteractiveCommand('safe', 'agent', false, 'MutatingOrNetwork'), 'StagedPendingUserGo');
  assert.equal(admitInteractiveCommand('confirm', 'agent', true, 'MutatingOrNetwork'), 'StagedPendingUserGo');
  assert.equal(admitInteractiveCommand('safe', 'plan', true, 'MutatingOrNetwork'), 'RefusedInPlanMode');
  assert.equal(admitInteractiveCommand('deny', 'agent', true, 'ReadOnlyInspection'), 'RefusedByDenyPolicy');

  // Deny policy refuses even read-only tool calls
  const denyOut = await execute_inprism_foundry_turn(
    {
      id: 'req-deny-1',
      payload: {
        prompt: 'list files',
        selected_model: 'uor-r4.native-q31-d85',
        composer_mode: 'agent',
        mcp_policy: 'deny',
        auto_run: true,
        offlineEmergencyQ31: true,
      },
    },
    {
      holoRuntimeStatus: () => JSON.stringify({ mounted_count: 1, models: [{ model_id: 'uor-r4.native-q31-d85' }] }),
      holoBeginTurn: () => JSON.stringify({ status: 'ready', turn_id: 401, model_id: 'uor-r4.native-q31-d85' }),
      holoStepTurn: () =>
        JSON.stringify({
          status: 'needs_tool',
          tokens_emitted: 2,
          tool_call: { name: 'mvm_exec', cmd: 'ls /workspace', arguments: { cmd: 'ls /workspace' } },
        }),
      holoCancelTurn: () => JSON.stringify({ status: 'cancelled' }),
      collectWorkspaceSnapshot: () => ({ files: [] }),
    },
  );
  assert.equal(denyOut.ok, false);
  assert.equal(denyOut.error_code, 'RefusedByDenyPolicy');

  // Staged command ('edit' -> runs edited command)
  const executedEnvelopes = [];
  let hop = 0;
  const stagedTurnPromise = execute_inprism_foundry_turn(
    {
      id: 'req-stage-1',
      payload: {
        prompt: 'build demo',
        selected_model: 'uor-r4.native-q31-d85',
        composer_mode: 'agent',
        mcp_policy: 'confirm',
        auto_run: false,
        offlineEmergencyQ31: true,
      },
    },
    {
      postEvent: (stage, data) => {
        if (stage === 'staged_mvm_command') {
          setTimeout(() => {
            resolve_staged_mvm_command({
              stage_id: data.stage_id,
              decision: 'edit',
              cmd: 'make -C /usr/src/mvm && /usr/src/mvm/mvm_hello',
            });
          }, 5);
        }
      },
      holoRuntimeStatus: () => JSON.stringify({ mounted_count: 1, models: [{ model_id: 'uor-r4.native-q31-d85' }] }),
      holoBeginTurn: () => JSON.stringify({ status: 'ready', turn_id: 402 + hop, model_id: 'uor-r4.native-q31-d85' }),
      holoStepTurn: () => {
        hop += 1;
        if (hop === 1) {
          return JSON.stringify({
            status: 'needs_tool',
            tokens_emitted: 3,
            tool_call: { name: 'mvm_exec', cmd: 'make -C /usr/src/mvm', arguments: { cmd: 'make -C /usr/src/mvm' } },
          });
        }
        return JSON.stringify({ status: 'done', tokens_emitted: 4, reply: 'Built self-host binary.' });
      },
      dispatchBridge: async (env) => {
        executedEnvelopes.push(env);
        return { status: 'ok', exit_code: 0, mvm_stdout: 'MVM_SELF_HOST_OK dot=26\n', mvm_stderr: '' };
      },
      collectWorkspaceSnapshot: () => ({ files: [] }),
      flushMvmCowToOpfs: async () => true,
    },
  );
  const stagedOut = await stagedTurnPromise;
  assert.equal(stagedOut.ok, true, `staged turn failed: ${JSON.stringify(stagedOut)}`);
  assert.ok(
    executedEnvelopes.some((e) => e.includes('make -C /usr/src/mvm && /usr/src/mvm/mvm_hello')),
    `expected edited command in dispatchBridge, got: ${JSON.stringify(executedEnvelopes)}`,
  );

  console.log('[PASS] R4-63 interactive command gates & staged edit/run verified');
}

async function testWasmMvmSingleSourceAndUiCleanliness() {
  console.log('=== Wave 2 (R4-63) Wasm Single-Source /workspace, Self-Host C, & Chat UI Cleanliness ===');

  const appJs = readFileSync(resolve(REPO_ROOT, 'public/app.js'), 'utf8');
  const appCss = readFileSync(resolve(REPO_ROOT, 'public/app.css'), 'utf8');
  const indexHtml = readFileSync(resolve(REPO_ROOT, 'public/index.html'), 'utf8');

  // Verify fake JS POSIX shell is deleted & chat HUD graph block is eliminated from runClaudeChat
  assert.ok(!appJs.includes('this.gitHeadFiles'), 'fake gitHeadFiles must be removed from LexLeanTerminal');
  assert.ok(!appJs.includes('applyStreamFilter('), 'fake JS pipe filter must be removed from LexLeanTerminal');
  assert.ok(!appJs.includes('Linux wasm-container'), 'fake uname wasm-container must be removed');
  assert.ok(!appJs.includes('createLiveTokenHudController(chatTranscript'), 'runClaudeChat must NEVER inject per-response graph block');
  assert.ok(appCss.includes('#chat-transcript .holo-live-hud'), 'app.css must hard-hide .holo-live-hud inside #chat-transcript');
  assert.ok(indexHtml.includes('id="composer-autorun-toggle"'), 'index.html must include #composer-autorun-toggle');

  // Verify real compiled Wasm binary single-source /workspace + self-hosted multi-TU C build
  const wasmJsUrl = new URL('../../public/lexlean_r4.js', import.meta.url);
  const wasmBytes = readFileSync(resolve(REPO_ROOT, 'public/lexlean_r4_bg.wasm'));
  const wasmMod = await import(wasmJsUrl.href);
  wasmMod.initSync({ module: wasmBytes });

  const wRes = JSON.parse(wasmMod.studio_dispatch('1\tstudio\tworkspace_write\thello_r463.txt\nfrom_editor_v1\n'));
  assert.equal(wRes.status, 'ok');

  const catRes = JSON.parse(
    wasmMod.studio_dispatch(`1\tstudio\tmvm_exec\t${JSON.stringify({ cmd: 'cat /workspace/hello_r463.txt', cwd: '/workspace' })}`),
  );
  assert.equal(catRes.exit_code, 0);
  assert.equal(catRes.mvm_stdout.trim(), 'from_editor_v1');

  const shWrite = JSON.parse(
    wasmMod.studio_dispatch(`1\tstudio\tmvm_exec\t${JSON.stringify({ cmd: 'echo from_mvm_shell_v2 > /workspace/hello_r463.txt', cwd: '/workspace' })}`),
  );
  assert.equal(shWrite.exit_code, 0);

  const editorRead = JSON.parse(wasmMod.studio_dispatch('1\tstudio\tworkspace_read\thello_r463.txt'));
  assert.equal(editorRead.status, 'ok');
  assert.equal(editorRead.content.trim(), 'from_mvm_shell_v2', 'Editor read must see MVM shell write immediately (single-source CoW)');

  const selfHost = JSON.parse(
    wasmMod.studio_dispatch(`1\tstudio\tmvm_exec\t${JSON.stringify({ cmd: 'make -C /usr/src/mvm && /usr/src/mvm/mvm_hello', cwd: '/workspace' })}`),
  );
  assert.equal(selfHost.exit_code, 0, `self-host make failed: ${JSON.stringify(selfHost)}`);
  assert.ok(selfHost.mvm_stdout.includes('MVM_SELF_HOST_OK dot=26'), `unexpected selfHost stdout: ${selfHost.mvm_stdout}`);

  console.log('[PASS] R4-63 single-source /workspace, self-hosted make+cc, and chat UI cleanliness verified');
}

await testSpiralCoreNetCorsPreemptionAndFailClosedGuards();
await testInteractiveCommandGatesAndStagedFlow();
await testWasmMvmSingleSourceAndUiCleanliness();
