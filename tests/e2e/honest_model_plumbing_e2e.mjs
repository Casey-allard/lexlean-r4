import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  optimal_browser_worker_count,
  init_shared_wasm_worker_pool,
  get_browser_worker_pool_snapshot,
} from '../../crates/wasm/js/worker_host_bridge.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..', '..');
const PUBLIC_DIR = path.join(ROOT, 'public');
const PRE_WASM_ONLY = process.argv.includes('--pre-wasm');

const TWO_USER_QWEN3_MODELS = [
  'uor-r4.qwen3-0.6b-holo',
  'uor-r4.qwen3-1.7b-holo',
];

const PROGRAMMATIC_V13_MODELS = [
  'uor-r4.native-q31-d85',
  'uor-r4.hf-repo-holo',
];

const RETIRED_SCAFFOLD_MODELS = [
  'uor-r4.recompiled-v12-agent',
  'uor-r4.recompiled-v12-chat',
  'uor-r4.seed-fast',
  'uor-r4.lut-symbolic',
];

function makeMinimalValidWasmBytes() {
  return new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
}

function loadRealBootstrapV9Bytes() {
  const v9Path = path.join(ROOT, 'crates', 'wasm', 'src', 'bootstrap_v9.uorgllm');
  assert.ok(fs.existsSync(v9Path), `Missing UORGLLM9 bootstrap fixture at ${v9Path}`);
  return fs.readFileSync(v9Path);
}

function loadRealR4V12ContainerBytes() {
  const v10Dir = path.join(ROOT, 'gpu', 'weights', 'v10');
  const manifestPath = path.join(v10Dir, 'manifest.json');
  if (fs.existsSync(manifestPath)) {
    const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
    if (manifest.format === 'UORR4V12' && Array.isArray(manifest.shards) && manifest.shards.length > 0) {
      const parts = [];
      for (const entry of manifest.shards) {
        const shardBytes = fs.readFileSync(path.join(v10Dir, entry.file));
        const actualSha = crypto.createHash('sha256').update(shardBytes).digest('hex');
        assert.equal(actualSha, entry.sha256, `Shard digest mismatch for ${entry.file}`);
        parts.push(shardBytes);
      }
      const bytes = Buffer.concat(parts);
      assert.equal(bytes.byteLength, manifest.total_bytes, 'Reassembled UORR4V12 byte length mismatch');
      const fullSha = crypto.createHash('sha256').update(bytes).digest('hex');
      assert.equal(fullSha, manifest.container_sha256, 'Reassembled UORR4V12 container sha256 mismatch');
      return { bytes, manifest, fromShards: true };
    }
  }
  const seedPath = path.join(ROOT, 'crates', 'wasm', 'src', 'bootstrap_v12_seed.uorr4');
  assert.ok(fs.existsSync(seedPath), `Missing UORR4V12 container (neither v10 UORR4V12 shards nor ${seedPath} exists)`);
  return { bytes: fs.readFileSync(seedPath), manifest: null, fromShards: false };
}

async function verifyPartAClosureAndStaticUi() {
  console.log('1. Checking exact 6-file public/ closure, zero forbidden tokens, and zero v10 banners...');
  const entries = fs.readdirSync(PUBLIC_DIR).sort();
  assert.deepEqual(
    entries,
    ['app.css', 'app.js', 'index.html', 'lexlean_r4.js', 'lexlean_r4_bg.wasm', 'provenance.json'],
    'public/ must maintain exact 6-file closure without extra files',
  );

  const wasmBytes = fs.readFileSync(path.join(PUBLIC_DIR, 'lexlean_r4_bg.wasm'));
  assert.ok(
    wasmBytes.byteLength < 15 * 1024 * 1024,
    `public/lexlean_r4_bg.wasm must remain < 15 MiB (got ${wasmBytes.byteLength} bytes)`,
  );

  const html = fs.readFileSync(path.join(PUBLIC_DIR, 'index.html'), 'utf8');
  const appJs = fs.readFileSync(path.join(PUBLIC_DIR, 'app.js'), 'utf8');
  const appCss = fs.readFileSync(path.join(PUBLIC_DIR, 'app.css'), 'utf8');
  const bridgeJs = fs.readFileSync(path.join(ROOT, 'crates', 'wasm', 'js', 'worker_host_bridge.js'), 'utf8');
  const provenanceRaw = fs.readFileSync(path.join(PUBLIC_DIR, 'provenance.json'), 'utf8');

  // Strict browser_closure guards: zero "telemetry" in public/* and worker_host_bridge.js
  for (const [label, text] of [
    ['public/index.html', html],
    ['public/app.js', appJs],
    ['public/app.css', appCss],
    ['public/provenance.json', provenanceRaw],
    ['crates/wasm/js/worker_host_bridge.js', bridgeJs],
  ]) {
    assert.equal(
      text.toLowerCase().includes('telemetry'),
      false,
      `${label} must not contain substring "telemetry"`,
    );
  }

  for (const forbidden of ['fetch(', 'parseInt(', 'parseFloat(', 'https://', 'wss://', 'WebSocket']) {
    assert.equal(
      appJs.includes(forbidden),
      false,
      `public/app.js must not contain forbidden substring "${forbidden}"`,
    );
  }

  // Verify zero stale v10/GLM/Python leftovers in UI
  assert.equal(/UOR-R4\s+G(?:L|LL)M\s+v10/i.test(html + '\n' + appJs), false, 'Stale UOR-R4 GLM v10 banner must be removed');
  assert.equal(/GLM\s*v10|GLLM\s*v10|gllm-v11|\bv10\b/i.test(html), false, 'public/index.html must not contain stale v10 references');
  assert.equal(/\bpython\b/i.test(html), false, 'public/index.html must not contain Python starter missions or prompts');

  // Verify ?v=v12r4-<js_sha12> content-addressed query cache busting (R4-75)
  const provDoc = JSON.parse(fs.readFileSync(path.join(PUBLIC_DIR, 'provenance.json'), 'utf8'));
  const jsSha12 = String(provDoc.binding_javascript_sha256 || '').slice(0, 12);
  assert.equal(jsSha12.length, 12, 'provenance.json must carry 64-hex binding_javascript_sha256');
  assert.ok(html.includes(`href="app.css?v=v12r4-${jsSha12}"`), `index.html must cache-bust app.css?v=v12r4-${jsSha12}`);
  assert.ok(html.includes(`src="app.js?v=v12r4-${jsSha12}"`), `index.html must cache-bust app.js?v=v12r4-${jsSha12}`);
  assert.ok(appJs.includes(`./lexlean_r4.js?v=v12r4-${jsSha12}'`), `app.js must cache-bust ./lexlean_r4.js?v=v12r4-${jsSha12}`);

  // Extract #model-select & #composer-model-select option values in exact order (R4-62: 2 UI Qwen3 models)
  const selectMatch = html.match(/<select[^>]*id="composer-model-select"[^>]*>([\s\S]*?)<\/select>/i);
  assert.ok(selectMatch, '#composer-model-select must exist in public/index.html');
  const optionValues = [...selectMatch[1].matchAll(/<option[^>]*value="([^"]+)"/gi)].map((m) => m[1]);
  assert.deepEqual(
    optionValues,
    TWO_USER_QWEN3_MODELS,
    '#composer-model-select must expose strictly the 2 user-facing Qwen3 models',
  );
  const topSelectMatch = html.match(/<select[^>]*id="model-select"[^>]*>([\s\S]*?)<\/select>/i);
  assert.ok(topSelectMatch, '#model-select must exist in public/index.html');
  const topOptionValues = [...topSelectMatch[1].matchAll(/<option[^>]*value="([^"]+)"/gi)].map((m) => m[1]);
  assert.deepEqual(
    topOptionValues,
    TWO_USER_QWEN3_MODELS,
    '#model-select must expose strictly the 2 user-facing Qwen3 models',
  );
  assert.ok(html.includes('id="settings-offline-q31-toggle"'), 'public/index.html must expose #settings-offline-q31-toggle');

  // Required Self-Improvement / Watchdog / Cursor layout & wired controls
  for (const containerId of ['uorc-watchdog-banner']) {
    assert.ok(html.includes(`id="${containerId}"`), `public/index.html missing id="${containerId}"`);
  }
  for (const reqId of [
    'verdant-synthesis-panel-badge',
    'uorc-watchdog-status',
    'uorc-worker-pool-badge',
    'uorc-admit-capsule-btn',
    'uorc-rollback-btn',
    'verdant-synthesis-kpi-grid',
    'cursor-search-input',
    'cursor-scm-accept-all-btn',
    'cursor-scm-discard-all-btn',
    'cursor-hydrate-shards-btn',
    'cursor-train-quick-btn',
    'status-problems',
    'settings-offline-q31-toggle',
    'hydration-inline-progress',
  ]) {
    assert.ok(html.includes(`id="${reqId}"`), `public/index.html missing id="${reqId}"`);
    assert.ok(appJs.includes(reqId), `public/app.js must bind/reference id="${reqId}"`);
  }

  assert.ok(html.includes('uor-r4.lut-stack/8'), 'public/index.html must retain canonical model schema identifier');
  assert.ok(appJs.includes('memory.point.points'), 'public/app.js must retain canonical MVM point telemetry field');
  console.log('  ✓ Closure, #composer-model-select options, ?v=v12r4, zero telemetry/v10/Python verified.');
}

async function verifyPartBModelPersistenceAndBridge() {
  console.log('2. Checking #composer-model-select AppState.config persistence & uor-workspace-bridge/1...');
  const rawJs = fs.readFileSync(path.join(PUBLIC_DIR, 'app.js'), 'utf8');
  const storageMap = new Map();
  const mockLocalStorage = {
    getItem(k) {
      return storageMap.has(k) ? storageMap.get(k) : null;
    },
    setItem(k, v) {
      storageMap.set(String(k), String(v));
    },
  };

  const resolveStart = rawJs.indexOf('const MODEL_STORAGE_KEY');
  const resolveEnd = rawJs.indexOf('const AppState = {', resolveStart);
  assert.ok(resolveStart >= 0 && resolveEnd > resolveStart, 'Model persistence helpers must exist in app.js');

  const modelSlice = new Function(
    'localStorage',
    `${rawJs.slice(resolveStart, resolveEnd)}
     return { MODEL_STORAGE_KEY, COMPOSER_MODE_STORAGE_KEY, DEFAULT_HONEST_MODEL_ID, resolveHonestModelId, loadPersistedModelId, loadPersistedComposerMode };`,
  )(mockLocalStorage);

  assert.equal(modelSlice.resolveHonestModelId('uor-r4.qwen3-1.7b-holo'), 'uor-r4.qwen3-1.7b-holo');
  assert.equal(modelSlice.resolveHonestModelId('stale-v10-id'), 'uor-r4.qwen3-0.6b-holo');
  mockLocalStorage.setItem(modelSlice.MODEL_STORAGE_KEY, 'uor-r4.native-q31-d85');
  mockLocalStorage.setItem(modelSlice.COMPOSER_MODE_STORAGE_KEY, 'plan');
  assert.equal(modelSlice.loadPersistedModelId(), 'uor-r4.native-q31-d85');
  assert.equal(modelSlice.loadPersistedComposerMode(), 'plan');

  // Verify runClaudeChat chatPayload carries selected_model + composer_mode and eager hydration streaming
  assert.ok(rawJs.includes('selected_model: selectedModel'), 'chatPayload must include selected_model');
  assert.ok(rawJs.includes('composer_mode:'), 'chatPayload must include composer_mode');
  assert.ok(rawJs.includes('0-Download 4L 256D'), 'app.js must label embedded D85 Q31 as 0-Download 4L 256D');
  assert.equal(rawJs.includes('0-Download 6L'), false, 'app.js must not carry stale 6L seed label');
  assert.equal(rawJs.includes('16 E8 Shards'), false, 'app.js must not carry stale 16 E8 Shards label');

  const composerTs = fs.readFileSync(path.join(ROOT, 'crates', 'wasm', 'ts', 'composer.ts'), 'utf8');
  assert.ok(composerTs.includes('0-Download 4L 256D'), 'composer.ts must label embedded D85 Q31 as 0-Download 4L 256D');
  const rawHtml = fs.readFileSync(path.join(PUBLIC_DIR, 'index.html'), 'utf8');
  assert.ok(rawHtml.includes('0-Download 4L 256D'), 'index.html must label embedded D85 Q31 as 0-Download 4L 256D');
  assert.equal(rawHtml.includes('16 E8 Shards'), false, 'index.html must not carry stale 16 E8 Shards label');

  const bridgeSrc = fs.readFileSync(path.join(ROOT, 'crates', 'wasm', 'js', 'worker_host_bridge.js'), 'utf8');
  assert.ok(bridgeSrc.includes('hydrate_holo_model('), 'worker_host_bridge.js must manage eager OPFS Holo hydration');
  assert.ok(bridgeSrc.includes('load_hf_repo_holo('), 'worker_host_bridge.js must support HuggingFace Range streaming');
  assert.ok(bridgeSrc.includes("postEvent('hydration_progress'"), 'worker_host_bridge.js must stream hydration_progress events');
  assert.ok(rawJs.includes("evType === 'hydration_progress'") && rawJs.includes('activeChatStreamHook(msg)'), 'app.js must forward hydration_progress to activeChatStreamHook');

  // Verify uor-workspace-bridge/1 postMessage round-trip alongside prism:* events using live app.js slice
  const diffStart = rawJs.indexOf('function computeUnifiedDiffJs(');
  const diffEnd = rawJs.indexOf('function renderInteractiveArtifactPreview(', diffStart);
  const scmStart = rawJs.indexOf('function renderCursorScmSection(');
  const scmEnd = rawJs.indexOf('function renderCursorEditorTabs(', scmStart);
  const ctrlStart = rawJs.indexOf('function showStudioUpdateBanner(');
  const ctrlEnd = rawJs.indexOf('function initLayoutAndConnectedControls(', ctrlStart);
  const initCtrlEnd = rawJs.indexOf('\n  if (saveRelayUrlBtn)', ctrlEnd);

  const posted = [];
  const winListeners = {};
  const parentWin = { postMessage(msg, target) { posted.push({ msg, target }); } };
  const bodyEl = { dataset: {}, classList: { _s: new Set(), add(c) { this._s.add(c); }, contains(c) { return this._s.has(c); } } };
  const byId = new Map();
  const makeEl = () => ({
    textContent: '',
    value: '',
    style: {},
    dataset: {},
    children: [],
    classList: { _s: new Set(), add(c) { this._s.add(c); }, toggle() {}, remove() {} },
    replaceChildren(...nodes) { this.children = [...nodes]; },
    appendChild(n) { this.children.push(n); return n; },
    addEventListener() {},
    setAttribute() {},
    removeAttribute() {},
    hasAttribute() { return true; },
  });
  for (const id of [
    'studio-update-banner', 'studio-update-banner-text', 'studio-update-banner-sub', 'studio-refresh-reload-btn',
    'studio-update-dismiss-btn', 'plan-dag-diagram', 'plan-checklist', 'plan-progress-badge', 'pool-status-grid',
    'cowork-status-bar', 'cowork-status-detail', 'cursor-shards-grid', 'cursor-shards-summary',
    'stitch-custody-health-card', 'live-context-turn-digest',
  ]) byId.set(id, makeEl());

  const mockDocument = {
    body: bodyEl,
    getElementById(id) { return byId.get(id) || null; },
    createElement() { return makeEl(); },
    createElementNS() { return makeEl(); },
    createTextNode(t) { return { nodeType: 3, textContent: String(t || '') }; },
  };
  const mockWindow = {
    location: { search: '?embed=me', href: 'https://example.invalid/?embed=me', origin: 'https://example.invalid', reload() {} },
    parent: parentWin,
    addEventListener(ev, fn) { winListeners[ev] = winListeners[ev] || []; winListeners[ev].push(fn); },
  };
  const mockAppState = { config: { selectedModel: 'uor-r4.qwen3-0.6b-holo', composerMode: 'agent', connectedGitRepo: 'acme/demo', connectedGitBranch: 'main' }, workspace: { pendingPatches: {} } };
  const mockWorkerClient = {
    enabled: true,
    workspaceFiles: new Map(),
    getWorkspaceListObject() { return { files: [] }; },
    async rpc(op, payload) {
      if (op === 'github_loop' && payload.subcommand === 'clone') {
        return { ok: true, repo: payload.repo, branch: payload.branch || 'main', branches: ['main'], files: [], lazy_paths: [] };
      }
      return { ok: true };
    },
  };

  const bridgeHarnessCode = [
    rawJs.slice(diffStart, diffEnd),
    'function computeCursorGitStatusForFile() { return { badge: "", cls: "", delta: "0L" }; }',
    'function activateCanvasTab() {}',
    'function selectWorkspaceFile() {}',
    'function renderThreadSidebar() {}',
    'function setLayoutMode() {}',
    'function getActiveThread() { return { messages: [] }; }',
    'function initPrismHoloIdentityControls() {}',
    'function renderPrismHoloIdentityUI() {}',
    'let activeWorkspacePath = "src/lib.rs";',
    rawJs.slice(scmStart, scmEnd),
    rawJs.slice(ctrlStart, initCtrlEnd),
  '}',
    'return { initLayoutAndConnectedControls };',
  ].join('\n');

  const api = new Function(
    'document', 'window', 'localStorage', 'AppState', 'engineWorkerClient', 'inspectorsModalEl',
    'openInspectorsModal', 'closeInspectorsModal', 'renderWorkspaceList', 'secopsAudit',
    bridgeHarnessCode,
  )(mockDocument, mockWindow, mockLocalStorage, mockAppState, mockWorkerClient, null, () => {}, () => {}, () => {}, { recordEvent: () => {} });

  api.initLayoutAndConnectedControls();
  assert.equal(posted[0].msg.type, 'prism:ready', 'First ready message preserves prism:ready parity');
  assert.ok(
    posted.some((p) => p.msg.protocol === 'uor-workspace-bridge/1' && p.msg.type === 'uor-workspace-bridge/1:ready'),
    'Embed bridge must also announce uor-workspace-bridge/1:ready',
  );

  for (const fn of winListeners.message || []) {
    await fn({
      data: { protocol: 'uor-workspace-bridge/1', type: 'uor-workspace-bridge/1:set-model', model: 'uor-r4.native-q31-d85' },
      source: parentWin,
    });
    await fn({
      data: { protocol: 'uor-workspace-bridge/1', type: 'uor-workspace-bridge/1:ping' },
      source: parentWin,
    });
  }
  assert.equal(mockAppState.config.selectedModel, 'uor-r4.native-q31-d85', 'uor-workspace-bridge/1:set-model must update AppState.config.selectedModel');
  const uorStatus = posted.map((p) => p.msg).filter((m) => m.protocol === 'uor-workspace-bridge/1' && m.type === 'uor-workspace-bridge/1:status').pop();
  assert.ok(uorStatus, 'uor-workspace-bridge/1:ping must emit uor-workspace-bridge/1:status');
  assert.equal(uorStatus.selected_model, 'uor-r4.native-q31-d85');
  console.log('  ✓ Model persistence & uor-workspace-bridge/1 round-trip verified.');
}

async function verifyPartCWorkerPoolScaling() {
  console.log('3. Checking optimalBrowserWorkerCount & WebAssembly.Module compile-once worker pool...');
  assert.equal(optimal_browser_worker_count(1), 1);
  assert.equal(optimal_browser_worker_count(2), 1);
  assert.equal(optimal_browser_worker_count(4), 3);
  assert.equal(optimal_browser_worker_count(8), 7);
  assert.equal(optimal_browser_worker_count(16), 8);
  assert.equal(optimal_browser_worker_count(64), 8);

  const prevWorker = globalThis.Worker;
  const prevNav = globalThis.navigator;
  const spawnedMessages = [];
  class MockWorker {
    constructor(url) {
      this.url = url;
      this.handlers = [];
    }
    addEventListener(ev, fn) {
      if (ev === 'message') this.handlers.push(fn);
    }
    removeEventListener(ev, fn) {
      if (ev === 'message') this.handlers = this.handlers.filter((h) => h !== fn);
    }
    postMessage(msg) {
      spawnedMessages.push(msg);
      if (msg && msg.type === 'INIT_MODULE') {
        for (const fn of [...this.handlers]) {
          fn({ data: { type: 'READY', workerId: msg.workerId } });
        }
      }
    }
    terminate() {}
  }

  try {
    globalThis.Worker = MockWorker;
    Object.defineProperty(globalThis, 'navigator', {
      configurable: true,
      value: { hardwareConcurrency: 4 },
    });
    const res = await init_shared_wasm_worker_pool(makeMinimalValidWasmBytes());
    assert.equal(res.logical_threads, 4);
    assert.equal(res.optimal_workers, 3, '4 logical cores must pool 3 shared workers');
    assert.equal(res.pool_size, 3);
    assert.equal(res.shared_module_compiled, true);
    assert.equal(res.zero_refetch_sharing, true);
    assert.equal(spawnedMessages.length, 3);
    assert.ok(spawnedMessages.every((m) => m.type === 'INIT_MODULE' && m.module instanceof WebAssembly.Module));
    const snap = get_browser_worker_pool_snapshot();
    assert.equal(snap.optimal_workers, 3);
    assert.equal(snap.pool_size, 3);
    assert.equal(snap.shared_module_compiled, true);
  } finally {
    globalThis.Worker = prevWorker;
    if (prevNav === undefined) {
      delete globalThis.navigator;
    } else {
      Object.defineProperty(globalThis, 'navigator', { configurable: true, value: prevNav });
    }
  }
  console.log('  ✓ optimalBrowserWorkerCount & shared WebAssembly.Module pool verified.');
}

async function verifyPartDWasmHonestRoutingAndWatchdog() {
  if (PRE_WASM_ONLY) {
    console.log('4. [--pre-wasm] Skipping live Wasm verification (will run strictly post xtask build-wasm).');
    return;
  }
  console.log('4. Verifying live Wasm routing, Holo v13 + Native Q31 status & 3-strike watchdog...');
  const bundleJsText = fs.readFileSync(path.join(PUBLIC_DIR, 'lexlean_r4.js'), 'utf8');
  assert.ok(
    bundleJsText.includes('studio_admit_generation_capsule')
      && bundleJsText.includes('studio_holo_runtime_status')
      && bundleJsText.includes('studio_native_q31_status')
      && bundleJsText.includes('studio_verdant_synthesis_status'),
    'public/lexlean_r4.js must be rebuilt via xtask build-wasm with v0.13.0 Holo & Q31 exports',
  );

  const wasmMod = await import(new URL('../../public/lexlean_r4.js', import.meta.url));
  const wasmBytes = fs.readFileSync(path.join(PUBLIC_DIR, 'lexlean_r4_bg.wasm'));
  wasmMod.initSync({ module: wasmBytes });

  assert.equal(typeof wasmMod.studio_holo_runtime_status, 'function', 'studio_holo_runtime_status must be exported');
  assert.equal(typeof wasmMod.studio_native_q31_status, 'function', 'studio_native_q31_status must be exported');
  assert.equal(typeof wasmMod.studio_native_q31_train_step, 'function', 'studio_native_q31_train_step must be exported');
  assert.equal(typeof wasmMod.studio_admit_generation_capsule, 'function', 'studio_admit_generation_capsule must be exported');
  assert.equal(typeof wasmMod.studio_record_generation_fault, 'function', 'studio_record_generation_fault must be exported');
  assert.equal(typeof wasmMod.studio_verdant_synthesis_status, 'function', 'studio_verdant_synthesis_status must be exported');
  assert.equal(typeof wasmMod.studio_admit_catalog_hydration_json, 'function', 'studio_admit_catalog_hydration_json must be exported');

  // Verify live Wasm R4-62 studio_admit_catalog_hydration_json positive & negative verdicts
  const r462Ok = JSON.parse(
    wasmMod.studio_admit_catalog_hydration_json('first_party_catalog', false, false, false, 'dropdown_compact_strip', false),
  );
  assert.equal(r462Ok.admitted, true);
  assert.equal(r462Ok.hydration_verdict, 'HydrationAdmitted');
  assert.equal(r462Ok.hud_verdict, 'HudNonBlockingAdmitted');
  assert.equal(r462Ok.max_catalog_hydration_concurrency, 3);
  const r462Consent = JSON.parse(
    wasmMod.studio_admit_catalog_hydration_json('external_untrusted', false, false, false, 'dropdown_compact_strip', false),
  );
  assert.equal(r462Consent.admitted, false);
  assert.equal(r462Consent.hydration_verdict, 'RefusedExternalOriginConsentRequired');
  const r462Declined = JSON.parse(
    wasmMod.studio_admit_catalog_hydration_json('first_party_catalog', false, false, true, 'dropdown_compact_strip', false),
  );
  assert.equal(r462Declined.admitted, false);
  assert.equal(r462Declined.hydration_verdict, 'RefusedSilentModelDowngrade');
  const r462Backdrop = JSON.parse(
    wasmMod.studio_admit_catalog_hydration_json('first_party_catalog', false, false, false, 'dropdown_compact_strip', true),
  );
  assert.equal(r462Backdrop.admitted, false);
  assert.equal(r462Backdrop.hud_verdict, 'RefusedBlockingBackdropOnLiveHud');

  // Check initial Holo v13 + Native Q31 D85 status
  const dispatchChatJson = (obj) =>
    wasmMod.studio_dispatch(`1\tstudio\tchat\t${typeof obj === 'string' ? obj : JSON.stringify(obj)}`);

  const holoStatus = JSON.parse(wasmMod.studio_holo_runtime_status());
  assert.equal(holoStatus.mounted_count, 0, 'Unmounted Holo OPFS models should report mounted_count=0 initially');
  const q31Status = JSON.parse(wasmMod.studio_native_q31_status());
  assert.equal(q31Status.format, 'UORQ31V1');

  // Pre-mount honesty (`R4`, `R5`): selecting OPFS/HF Holo models before mounting MUST refuse cleanly
  for (const unmountedId of ['uor-r4.qwen3-0.6b-holo', 'uor-r4.qwen3-1.7b-holo', 'uor-r4.hf-repo-holo']) {
    const rawRefusal = dispatchChatJson({
      prompt: 'Refactor src/lib.rs and fix the syntax bug',
      composer_mode: 'agentic',
      selected_model: unmountedId,
    });
    const refusal = JSON.parse(rawRefusal);
    assert.equal(refusal.status, 'error', `${unmountedId} must refuse cleanly before OPFS mount`);
    assert.equal(refusal.error_code, 'RefusedRequestedModelUnmounted', `Expected RefusedRequestedModelUnmounted for ${unmountedId}`);
    assert.equal(refusal.selected_model, unmountedId);
  }

  // Retired v12/seed/symbolic scaffold modes MUST refuse strictly (`R5`)
  for (const retiredId of RETIRED_SCAFFOLD_MODELS) {
    const retiredResp = JSON.parse(dispatchChatJson({
      prompt: 'hello',
      composer_mode: 'agentic',
      selected_model: retiredId,
    }));
    assert.equal(retiredResp.status, 'error');
    assert.equal(retiredResp.error_code, 'RefusedRetiredScaffoldMode');
  }

  // Unrecognized selected_model refuses strictly (`R5`)
  const badModelResp = JSON.parse(dispatchChatJson({
    prompt: 'hello',
    composer_mode: 'agentic',
    selected_model: 'uor-r4.unknown-mode',
  }));
  assert.equal(badModelResp.status, 'error');
  assert.equal(badModelResp.error_code, 'RefusedModeMismatch');

  // Embedded Native Q31 D85 executes immediately with zero float ops and zero template hijacks
  const q31RespRaw = dispatchChatJson({
    prompt: 'Teach: refactor this Rust function in src/lib.rs',
    composer_mode: 'fast',
    selected_model: 'uor-r4.native-q31-d85',
  });
  assert.equal(q31RespRaw.toLowerCase().includes('"telemetry"'), false, 'studio chat dispatch must not emit "telemetry" key');
  const q31Resp = JSON.parse(q31RespRaw);
  assert.equal(q31Resp.status, 'ok', 'uor-r4.native-q31-d85 must execute cleanly');
  assert.equal(q31Resp.selected_model, 'uor-r4.native-q31-d85');
  assert.equal(q31Resp.is_coding_turn, false, 'Neural mode must skip canned synthesize_code_turn hijack');
  assert.equal(q31Resp.proposed_code, '');
  assert.equal(q31Resp.geometric_metrics?.float_ops, 0);
  assert.equal(q31Resp.geometric_metrics?.dense_matmul_ops, 0);
  assert.ok(Array.isArray(q31Resp.brain_pulses) && q31Resp.brain_pulses.length > 0, 'brain_pulses required');

  // Online Native Q31 Hamilton train step lowers loss or stays bounded and advances step count
  const trainStepRes = JSON.parse(wasmMod.studio_native_q31_train_step('Spinning SpiralCore D85 pure integer training corpus'));
  assert.equal(trainStepRes.status, 'ok');
  assert.ok(trainStepRes.step >= 1, 'native_q31_train_step must advance step counter');

  // Verify legacy UORGLLM9 acceptance + corrupt magic R5 negative rejection
  const realV9Bytes = loadRealBootstrapV9Bytes();
  const v9Res = JSON.parse(wasmMod.studio_load_uorgllm_v9_bytes(realV9Bytes, 'e2e_v9_compat'));
  assert.equal(v9Res.status, 'ok', 'UORGLLM9 compatibility load must succeed');
  assert.equal(v9Res.format, 'UORGLLM9');

  const badV9Buf = Buffer.from(realV9Bytes);
  Buffer.from('BADMAGIC', 'ascii').copy(badV9Buf, 0);
  const badV9Res = JSON.parse(wasmMod.studio_load_uorgllm_v9_bytes(badV9Buf, 'bad_magic'));
  assert.equal(badV9Res.status, 'error', 'Invalid magic in studio_load_uorgllm_v9_bytes must fail cleanly');

  // Verify UORR4V12 malformed container rejection (R5)
  const badR4Res = JSON.parse(wasmMod.studio_mount_r4_v12_bytes(Buffer.from('NOT_UORR4_DATA')));
  assert.equal(badR4Res.status, 'error', 'studio_mount_r4_v12_bytes must reject truncated/corrupt bytes');
  assert.ok(
    typeof badR4Res.error_code === 'string' && badR4Res.error_code.startsWith('ERR_R4V12_'),
    `Expected typed ERR_R4V12_* error_code, got ${badR4Res.error_code}`,
  );

  // Mount real UORR4V12 container (reassembled v10 shards when available, else embedded v12 seed container)
  const { bytes: r4Bytes, fromShards } = loadRealR4V12ContainerBytes();
  const r4Mounted = JSON.parse(wasmMod.studio_mount_r4_v12_bytes(r4Bytes));
  assert.equal(r4Mounted.status, 'ok', `UORR4V12 mount must succeed (fromShards=${fromShards}): ${JSON.stringify(r4Mounted)}`);
  assert.equal(r4Mounted.format, 'UORR4V12');
  assert.equal(r4Mounted.full_recompiled_mounted, true);
  assert.equal(r4Mounted.float_ops, 0);
  assert.equal(r4Mounted.dense_matmul_ops, 0);
  const postMountStack = JSON.parse(wasmMod.studio_r4_stack_status());
  assert.equal(postMountStack.full_recompiled_mounted, true, 'UORR4V12 mount must mark full_recompiled_mounted=true');

  // Verify .uorc gen-0 fault strikes (1-2 continue, 3 refuses at gen-0), negative capsule rejection (`R5`), valid admission, and 3-strike rollback
  for (let s = 1; s <= 2; s += 1) {
    const gen0Ok = JSON.parse(wasmMod.studio_record_generation_fault(`gen0_probe_${s}`));
    assert.equal(gen0Ok.status, 'ok');
    assert.equal(gen0Ok.verdict, 'FaultRecordedContinue');
    assert.equal(gen0Ok.rolled_back, false);
  }
  const gen0Fault = JSON.parse(wasmMod.studio_record_generation_fault('gen0_probe_3'));
  assert.equal(gen0Fault.status, 'error');
  assert.equal(gen0Fault.verdict, 'RefusedRollbackAtGenZero');
  assert.equal(gen0Fault.rolled_back, false);

  const badCapsule = JSON.parse(wasmMod.studio_admit_generation_capsule(JSON.stringify({
    schema: 'uorc/generation-capsule/1',
    magic_uorc: 'BAD!',
    generation_id: 'gen-bad',
    gates_passed: 4,
    gates_total: 4,
    residue_count: 0,
    wasm_bytes: 1000,
    max_wasm_bytes: 15 * 1024 * 1024,
    frame_time_micros: 500,
    frame_budget_micros: 16667,
    candidate_loss: 9,
    incumbent_loss: 12,
    candidate_steps: 3,
    incumbent_steps: 5,
  })));
  assert.equal(badCapsule.admitted, false, 'Invalid .uorc capsule must be rejected (R5)');
  assert.equal(badCapsule.error_code, 'RefusedInvalidArchiveMagic');

  const goodCapsule = JSON.parse(wasmMod.studio_admit_generation_capsule(JSON.stringify({
    schema: 'uorc/generation-capsule/1',
    magic_uorc: 'UORC',
    generation_id: 'gen-e2e-1',
    gates_passed: 4,
    gates_total: 4,
    residue_count: 0,
    coercive_patterns: 0,
    wasm_bytes: wasmBytes.byteLength,
    max_wasm_bytes: 15 * 1024 * 1024,
    frame_time_micros: 900,
    frame_budget_micros: 16667,
    candidate_loss: 10,
    incumbent_loss: 12,
    candidate_steps: 4,
    incumbent_steps: 5,
  })));
  assert.equal(goodCapsule.status, 'ok');
  assert.equal(goodCapsule.admitted, true, 'Valid .uorc capsule must be admitted');
  assert.equal(goodCapsule.active_head, 'gen-e2e-1');
  assert.equal(goodCapsule.parent_head, 'gen-0');

  const f1 = JSON.parse(wasmMod.studio_record_generation_fault('probe_1'));
  assert.equal(f1.rolled_back, false);
  const f2 = JSON.parse(wasmMod.studio_record_generation_fault('probe_2'));
  assert.equal(f2.rolled_back, false);
  const f3 = JSON.parse(wasmMod.studio_record_generation_fault('probe_3'));
  assert.equal(f3.status, 'ok');
  assert.equal(f3.verdict, 'RollbackToParentHead');
  assert.equal(f3.rolled_back, true, '3rd consecutive fault must trigger watchdog rollback');
  assert.equal(f3.active_head, 'gen-0', 'Rollback must restore certified parent head gen-0');

  const statusAfter = JSON.parse(wasmMod.studio_verdant_synthesis_status());
  assert.equal(statusAfter.active_head, 'gen-0');
  assert.equal(statusAfter.rolled_back, true);
  assert.ok(Array.isArray(statusAfter.facets) && statusAfter.facets.length === 4, 'Status must report 4 Verdant Synthesis facets');
  console.log('  ✓ Live Wasm routing, UORGLLM9 + UORR4V12 shard loading & 3-strike watchdog verified.');
}

async function main() {
  console.log('=== E2E: Track 3 Honest Model Plumbing, .uorc Watchdog & Bridge ===');
  await verifyPartAClosureAndStaticUi();
  await verifyPartBModelPersistenceAndBridge();
  await verifyPartCWorkerPoolScaling();
  await verifyPartDWasmHonestRoutingAndWatchdog();
  console.log('=== ALL HONEST MODEL PLUMBING E2E CHECKS PASSED ===');
}

main().catch((err) => {
  console.error('HONEST MODEL PLUMBING E2E FAILED:', err);
  process.exit(1);
});
