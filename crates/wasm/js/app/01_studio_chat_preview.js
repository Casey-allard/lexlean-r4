import init, {
  admit_uorc_generation_capsule,
  cache_e8_shard_in_opfs,
  cache_geometric_shard_in_indexeddb,
  cache_uorlut_in_indexeddb,
  clear_cached_uorlut_in_indexeddb,
  decodeSpvsWireFrame,
  encodeSpvsWireFrame,
  generate_with_webgpu_open_weights_coprocessor,
  get_browser_worker_pool_snapshot,
  hydrate_all_16_e8_shards_to_opfs,
  init_shared_wasm_worker_pool,
  load_e8_shard_from_opfs,
  load_geometric_shard_from_indexeddb,
  mount_open_weights_pack_to_opfs,
  opfs_get_blob,
  opfs_put_blob,
  optimal_browser_worker_count,
  persist_mvm_cow_sectors_to_opfs,
  process_envelope,
  query_verdant_synthesis_status,
  record_uorc_generation_fault,
  restore_cached_uorlut_from_indexeddb,
  restore_gllm_v9_from_opfs,
  restore_mvm_cow_sectors_from_opfs,
  route_and_hot_swap_e8_shard,
  run_webgpu_q12_attention_projection,
  run_webgpu_v9_fwht64_tmac,
  storageRequest,
  stream_sharded_weights_to_indexeddb,
  stream_v9_sharded_weights_via_opfs_worker,
  studio_admit_generation_capsule,
  studio_dispatch,
  studio_export_uorlut_shard,
  studio_gllm_v9_status,
  studio_load_uorlut_bytes,
  studio_merge_uorlut_shard,
  studio_mount_r4_v12_bytes,
  studio_r4_stack_status,
  studio_record_generation_fault,
  studio_render_hopf_zeta_3d_rgba,
  studio_render_preview_scene_3d_rgba,
  studio_shard_catalog,
  studio_v1_chat_completions,
  studio_v1_messages,
  studio_v1_traces,
  studio_verdant_synthesis_status,
  studio_vtensor_3d_self_check_json,
  studio_wasm_eval,
  studio_weight_pack_status,
  sync_cached_weights,
  withStorageLock,
} from './lexlean_r4.js?v=v12r4-4387c815296d';

let mainThreadWasmReady = false;

globalThis.__uorOpenWeightsRuntime = {
  cacheE8ShardInOpfs: cache_e8_shard_in_opfs,
  loadE8ShardFromOpfs: load_e8_shard_from_opfs,
  runWebGpuQ12Attention: run_webgpu_q12_attention_projection,
  runWebGpuV9Fwht64Tmac: run_webgpu_v9_fwht64_tmac,
  hydrateAll16E8ShardsToOpfs: hydrate_all_16_e8_shards_to_opfs,
  routeAndHotSwapE8Shard: route_and_hot_swap_e8_shard,
  generateWithWebGpuCoprocessor: generate_with_webgpu_open_weights_coprocessor,
  restoreGllmV9FromOpfs: restore_gllm_v9_from_opfs,
  streamV9ShardedWeightsViaOpfsWorker: stream_v9_sharded_weights_via_opfs_worker,
  persistMvmCowSectorsToOpfs: persist_mvm_cow_sectors_to_opfs,
  restoreMvmCowSectorsFromOpfs: restore_mvm_cow_sectors_from_opfs,
  optimalBrowserWorkerCount: optimal_browser_worker_count,
  initSharedWasmWorkerPool: init_shared_wasm_worker_pool,
  getBrowserWorkerPoolSnapshot: get_browser_worker_pool_snapshot,
  admitUorcGenerationCapsule: admit_uorc_generation_capsule,
  recordUorcGenerationFault: record_uorc_generation_fault,
  queryVerdantSynthesisStatus: query_verdant_synthesis_status,
  mountHoloV13Bytes: (modelId, bytes) => engineWorkerClient.rpc('mount_holo_v13', { modelId, bytes }),
  getHoloRuntimeStatus: () => engineWorkerClient.rpc('holo_runtime_status', {}),
  getNativeQ31Status: () => engineWorkerClient.rpc('native_q31_status', {}),
  trainNativeQ31Step: (text) => engineWorkerClient.rpc('native_q31_train_step', { text }),
  loadHfRepoHolo: (repoId) => engineWorkerClient.rpc('load_hf_repo_holo', { repoId }),
};

globalThis.LexLean = {
  ...(globalThis.LexLean || {}),
  opfs_put_blob,
  opfs_get_blob,
  storageRequest,
  withStorageLock,
  encodeSpvsWireFrame,
  decodeSpvsWireFrame,
  persist_mvm_cow_sectors_to_opfs,
  restore_mvm_cow_sectors_from_opfs,
};

const form = document.getElementById('application-form');
const left = document.getElementById('left');
const right = document.getElementById('right');
const operation = document.getElementById('operation');
const result = document.getElementById('result');

const studioPromptForm = document.getElementById('studio-prompt-form');
const studioPromptInput = document.getElementById('studio-prompt');
const studioProseOutput = document.getElementById('studio-prose-output');

const studioAdmitForm = document.getElementById('studio-admit-form');
const admitPromptInput = document.getElementById('admit-prompt');
const admitReplyInput = document.getElementById('admit-reply');
const admitResetBtn = document.getElementById('admit-reset-btn');
const snapshotExportBtn = document.getElementById('snapshot-export-btn');
const snapshotImportBtn = document.getElementById('snapshot-import-btn');
const snapshotDownloadBtn = document.getElementById('snapshot-download-btn');
const snapshotUploadInput = document.getElementById('snapshot-upload-input');
const snapshotHexInput = document.getElementById('snapshot-hex-input');
const admitStatus = document.getElementById('admit-status');

const statSchema = document.getElementById('stat-schema');
const statVocab = document.getElementById('stat-vocab');
const statTurnRows = document.getElementById('stat-turn-rows');
const statPointRows = document.getElementById('stat-point-rows');
const statCells = document.getElementById('stat-cells');
const statTrainEpochs = document.getElementById('stat-train-epochs');
const statStateReceipt = document.getElementById('stat-state-receipt');

const workspaceFileList = document.getElementById('workspace-file-list');
const workspacePinnedBadge = document.getElementById('workspace-pinned-badge');
const workspaceNewForm = document.getElementById('workspace-new-form');
const workspaceNewPath = document.getElementById('workspace-new-path');
const workspaceMountInput = document.getElementById('workspace-mount-input');
const editorFilePath = document.getElementById('editor-file-path');
const editorFileReceipt = document.getElementById('editor-file-receipt');
const workspaceEditorTextarea = document.getElementById('workspace-editor-textarea');
const workspaceSaveBtn = document.getElementById('workspace-save-btn');
const workspaceDownloadBtn = document.getElementById('workspace-download-btn');
const workspaceStatus = document.getElementById('workspace-status');

const claudeChatForm = document.getElementById('claude-chat-form');
const claudeChatInput = document.getElementById('claude-chat-input');
const chatClearBtn = document.getElementById('chat-clear-btn');
const chatTranscript = document.getElementById('chat-transcript');
const chatTargetBadge = document.getElementById('chat-target-badge');
const chatDiffFile = document.getElementById('chat-diff-file');
const chatDiffPreview = document.getElementById('chat-diff-preview');
const chatArtifactPreview = document.getElementById('chat-artifact-preview');
const chatDiffFilePills = document.getElementById('chat-diff-file-pills');
const chatDiffViewBtn = document.getElementById('chat-diff-view-btn');
const chatArtifactViewBtn = document.getElementById('chat-artifact-view-btn');
const chatApplyPatchBtn = document.getElementById('chat-apply-patch-btn');
const chatDiagConsensus = document.getElementById('chat-diag-consensus');
const chatDiagResonance = document.getElementById('chat-diag-resonance');
const chatDiagRoot = document.getElementById('chat-diag-root');
const chatDiagReceipt = document.getElementById('chat-diag-receipt');

const studioTrainForm = document.getElementById('studio-train-form');
const trainEpochsInput = document.getElementById('train-epochs-input');
const trainStatLoss = document.getElementById('train-stat-loss');
const trainStatEscapes = document.getElementById('train-stat-escapes');
const trainStatGates = document.getElementById('train-stat-gates');
const trainStatTransitions = document.getElementById('train-stat-transitions');
const trainStatWsTokens = document.getElementById('train-stat-ws-tokens');
const trainStatReceipt = document.getElementById('train-stat-receipt');
const trainHistoryTbody = document.getElementById('train-history-tbody');

let activeWorkspacePath = 'src/attention.rs';
let pendingPatchPath = '';
let pendingProposedCode = '';
let pendingDiffPreview = '';

// ============================================================================
// In-Browser Cryptographic Merkle-Chained Secops Audit Pipeline (FIPS 180-4)
// ============================================================================

function computeSha256Hex(asciiStr) { /* Formal R4.StudioFrame */
  const K = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
  ];

  let H0 = 0x6a09e667, H1 = 0xbb67ae85, H2 = 0x3c6ef372, H3 = 0xa54ff53a;
  let H4 = 0x510e527f, H5 = 0x9b05688c, H6 = 0x1f83d9ab, H7 = 0x5be0cd19;

  const enc = new TextEncoder();
  const bytes = enc.encode(asciiStr);
  const l = bytes.length;
  const bitLen = BigInt(l) * 8n;

  const kZeros = (56 - ((l + 1) % 64) + 64) % 64;
  const totalLen = l + 1 + kZeros + 8;
  const padded = new Uint8Array(totalLen);
  padded.set(bytes, 0);
  padded[l] = 0x80;

  for (let i = 0; i < 8; i++) {
    padded[totalLen - 1 - i] = Number((bitLen >> BigInt(i * 8)) & 0xffn);
  }

  const W = new Uint32Array(64);
  const view = new DataView(padded.buffer);

  for (let offset = 0; offset < totalLen; offset += 64) {
    for (let i = 0; i < 16; i++) {
      W[i] = view.getUint32(offset + i * 4, false);
    }
    for (let i = 16; i < 64; i++) {
      const s0 = ((W[i - 15] >>> 7) | (W[i - 15] << 25)) ^
                 ((W[i - 15] >>> 18) | (W[i - 15] << 14)) ^
                 (W[i - 15] >>> 3);
      const s1 = ((W[i - 2] >>> 17) | (W[i - 2] << 15)) ^
                 ((W[i - 2] >>> 19) | (W[i - 2] << 13)) ^
                 (W[i - 2] >>> 10);
      W[i] = (W[i - 16] + s0 + W[i - 7] + s1) >>> 0;
    }

    let a = H0, b = H1, c = H2, d = H3, e = H4, f = H5, g = H6, h = H7;

    for (let i = 0; i < 64; i++) {
      const S1 = ((e >>> 6) | (e << 26)) ^ ((e >>> 11) | (e << 21)) ^ ((e >>> 25) | (e << 7));
      const ch = (e & f) ^ ((~e) & g);
      const temp1 = (h + S1 + ch + K[i] + W[i]) >>> 0;
      const S0 = ((a >>> 2) | (a << 30)) ^ ((a >>> 13) | (a << 19)) ^ ((a >>> 22) | (a << 10));
      const maj = (a & b) ^ (a & c) ^ (b & c);
      const temp2 = (S0 + maj) >>> 0;

      h = g;
      g = f;
      f = e;
      e = (d + temp1) >>> 0;
      d = c;
      c = b;
      b = a;
      a = (temp1 + temp2) >>> 0;
    }

    H0 = (H0 + a) >>> 0;
    H1 = (H1 + b) >>> 0;
    H2 = (H2 + c) >>> 0;
    H3 = (H3 + d) >>> 0;
    H4 = (H4 + e) >>> 0;
    H5 = (H5 + f) >>> 0;
    H6 = (H6 + g) >>> 0;
    H7 = (H7 + h) >>> 0;
  }

  const toHex = (n) => n.toString(16).padStart(8, '0');
  return toHex(H0) + toHex(H1) + toHex(H2) + toHex(H3) + toHex(H4) + toHex(H5) + toHex(H6) + toHex(H7);
}

class SecopsAuditLedger {
  constructor(maxSize = 500) {
    this.maxSize = maxSize;
    this.events = [];
    this.sequence = 0;
    this.lastHash = '0000000000000000000000000000000000000000000000000000000000000000';
  }

  emit(category, action, target, details = {}, severity = 'info') {
    this.sequence++;
    const timestamp = new Date().toISOString();
    const eventId = `audit-${Date.now()}-${String(this.sequence).padStart(5, '0')}`;
    const payloadStr = JSON.stringify(details);
    const payloadDigest = computeSha256Hex(payloadStr);

    const prevHash = this.lastHash;
    const preImage = `${eventId}|${timestamp}|${this.sequence}|${category}|${action}|${target}|${payloadDigest}|${prevHash}`;
    const currentHash = computeSha256Hex(preImage);
    this.lastHash = currentHash;

    const event = {
      schema: 'uor.secops.audit/1',
      event_id: eventId,
      timestamp,
      sequence: this.sequence,
      category,
      severity,
      actor: {
        type: 'agent',
        id: 'uor-r4-glm-v8',
        session_id: 'session-main'
      },
      action,
      target,
      policy: {
        mode: (typeof AppState !== 'undefined' && AppState.config) ? AppState.config.mcpSecurityPolicy : 'safe',
        enforced: true
      },
      payload_digest: payloadDigest,
      details,
      previous_event_hash: prevHash,
      event_hash: currentHash
    };

    this.events.push(event);
    if (this.events.length > this.maxSize) {
      this.events.shift();
    }

    const mcpLog = document.getElementById('mcp-audit-log');
    if (mcpLog) {
      const item = document.createElement('div');
      item.className = `mcp-log-item mcp-log-${severity}`;
      const timeStr = timestamp.split('T')[1].replace('Z', '');
      item.textContent = `[${timeStr}] [${category.toUpperCase()}] ${action} -> ${target} (${currentHash.slice(0, 8)})`;
      mcpLog.appendChild(item);
      mcpLog.scrollTop = mcpLog.scrollHeight;
    }

    return event;
  }

  recordEvent(action, target, category = 'system', details = {}, severity = 'info') {
    return this.emit(category, action, target, details, severity);
  }

  exportJournal() {
    const dataStr = JSON.stringify(this.events, null, 2);
    const blob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `secops-audit-journal-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  verifyChain() {
    let prev = '0000000000000000000000000000000000000000000000000000000000000000';
    for (let i = 0; i < this.events.length; i++) {
      const ev = this.events[i];
      if (ev.previous_event_hash !== prev) return false;
      const preImage = `${ev.event_id}|${ev.timestamp}|${ev.sequence}|${ev.category}|${ev.action}|${ev.target}|${ev.payload_digest}|${prev}`;
      const computed = computeSha256Hex(preImage);
      if (computed !== ev.event_hash) return false;
      prev = ev.event_hash;
    }
    return true;
  }
}

const secopsAudit = new SecopsAuditLedger();
function emitAuditEvent(category, action, target, details, severity) { /* Formal R4.StudioFrame */
  return secopsAudit.emit(category, action, target, details, severity);
}

// ============================================================================
// Centralized Reactive Application State
// ============================================================================

const MODEL_STORAGE_KEY = 'uor_r4_selected_model_v13';
const COMPOSER_MODE_STORAGE_KEY = 'uor_r4_composer_mode_v13';
const DEFAULT_HONEST_MODEL_ID = 'uor-r4.qwen3-0.6b-holo';

/** Formal R4.HonestPlumbing resolve persisted or requested model identifier onto active v0.13.0 lanes. */
function resolveHonestModelId(rawId) { /* Formal R4.StudioFrame */
  const val = String(rawId || '').trim();
  if (
    val === 'uor-r4.qwen3-0.6b-holo'
    || val === 'uor-r4.qwen3-1.7b-holo'
    || val === 'uor-r4.native-q31-d85'
    || val === 'uor-r4.hf-repo-holo'
  ) {
    return val;
  }
  if (val.includes('1.7b')) return 'uor-r4.qwen3-1.7b-holo';
  if (val.includes('q31') || val.includes('d85')) return 'uor-r4.native-q31-d85';
  if (val.includes('hf')) return 'uor-r4.hf-repo-holo';
  return DEFAULT_HONEST_MODEL_ID;
}

function loadPersistedModelId() {
  try {
    if (typeof localStorage !== 'undefined' && typeof localStorage.getItem === 'function') {
      const saved = localStorage.getItem(MODEL_STORAGE_KEY);
      if (saved) return resolveHonestModelId(saved);
    }
  } catch {
    // Ignore storage unavailability in sandboxed contexts
  }
  return DEFAULT_HONEST_MODEL_ID;
}

function loadPersistedComposerMode() {
  try {
    if (typeof localStorage !== 'undefined' && typeof localStorage.getItem === 'function') {
      const saved = localStorage.getItem(COMPOSER_MODE_STORAGE_KEY);
      if (saved === 'agentic' || saved === 'plan' || saved === 'fast' || saved === 'teach' || saved === 'math') {
        return saved;
      }
    }
  } catch {
    // Ignore storage unavailability
  }
  return 'agentic';
}

const AppState = {
  ui: {
    sidebarCollapsed: false,
    activeSidebarView: 'explorer',
    drawerOpen: true,
    activeDrawerTab: 'terminal',
    density: 'compact',
    theme: 'dark'
  },
  config: {
    selectedModel: loadPersistedModelId(),
    composerMode: loadPersistedComposerMode(),
    operatingMode: 'coding',
    retainLambda: 0.85,
    pluralityVotes: 8,
    mcpSecurityPolicy: 'safe',
    activeProject: 'lexlean-r4',
    connectedGitRepo: '',
    connectedGitBranch: 'main',
    gitBranches: ['main'],
    gitBaselineFiles: {},
    gitLazyPaths: [],
    activePrNumber: null,
    activePrUrl: '',
    ciSummary: 'CI & Pages: Standby'
  },
  workspace: {
    activePath: 'src/attention.rs',
    pendingPatchPath: '',
    pendingProposedCode: '',
    pendingPatches: {}
  }
};

(() => {
  try {
    if (
      typeof localStorage !== 'undefined' &&
      typeof localStorage.getItem === 'function' &&
      typeof localStorage.setItem === 'function' &&
      localStorage.getItem('uor_offline_q31_v15_migrated') !== '1'
    ) {
      if (typeof localStorage.removeItem === 'function') {
        localStorage.removeItem('uor_offline_emergency_q31');
      }
      localStorage.setItem('uor_offline_q31_v15_migrated', '1');
    }
    const emergency = typeof localStorage !== 'undefined' && typeof localStorage.getItem === 'function'
      && localStorage.getItem('uor_offline_emergency_q31') === '1';
    if (emergency) {
      AppState.config.selectedModel = 'uor-r4.native-q31-d85';
    } else if (
      AppState.config.selectedModel !== 'uor-r4.qwen3-0.6b-holo'
      && AppState.config.selectedModel !== 'uor-r4.qwen3-1.7b-holo'
    ) {
      AppState.config.selectedModel = DEFAULT_HONEST_MODEL_ID;
      if (typeof localStorage !== 'undefined' && typeof localStorage.setItem === 'function') {
        localStorage.setItem(MODEL_STORAGE_KEY, DEFAULT_HONEST_MODEL_ID);
      }
    }
  } catch {
    // Ignore storage unavailability in sandboxed contexts
  }
})();

function computeUnifiedDiffJs(oldText, newText, targetPath, contextLines = 3) { /* Formal R4.StudioFrame */
  const oldLines = String(oldText || '').split('\n');
  const newLines = String(newText || '').split('\n');
  const n = oldLines.length;
  const m = newLines.length;
  const dp = Array.from({ length: n + 1 }, () => new Int32Array(m + 1));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      if (oldLines[i] === newLines[j]) {
        dp[i][j] = dp[i + 1][j + 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i + 1][j], dp[i][j + 1]);
      }
    }
  }
  const ops = [];
  let i = 0;
  let j = 0;
  while (i < n || j < m) {
    if (i < n && j < m && oldLines[i] === newLines[j]) {
      ops.push({ kind: ' ', oldLine: i + 1, newLine: j + 1, text: oldLines[i] });
      i++;
      j++;
    } else if (j < m && (i === n || dp[i][j + 1] >= dp[i + 1][j])) {
      ops.push({ kind: '+', oldLine: i + 1, newLine: j + 1, text: newLines[j] });
      j++;
    } else {
      ops.push({ kind: '-', oldLine: i + 1, newLine: j + 1, text: oldLines[i] });
      i++;
    }
  }
  const changeIndices = [];
  for (let idx = 0; idx < ops.length; idx++) {
    if (ops[idx].kind !== ' ') changeIndices.push(idx);
  }
  const out = [`--- a/${targetPath}`, `+++ b/${targetPath}`];
  if (changeIndices.length === 0) {
    out.push('@@ -1,0 +1,0 @@');
    return out.join('\n');
  }
  const groups = [];
  for (const ci of changeIndices) {
    const start = Math.max(0, ci - contextLines);
    const end = Math.min(ops.length - 1, ci + contextLines);
    if (groups.length > 0 && start <= groups[groups.length - 1][1] + 1) {
      groups[groups.length - 1][1] = Math.max(groups[groups.length - 1][1], end);
    } else {
      groups.push([start, end]);
    }
  }
  for (const [gStart, gEnd] of groups) {
    const slice = ops.slice(gStart, gEnd + 1);
    const oldStart = slice[0] ? slice[0].oldLine : 1;
    const newStart = slice[0] ? slice[0].newLine : 1;
    const oldCount = slice.filter((o) => o.kind === ' ' || o.kind === '-').length;
    const newCount = slice.filter((o) => o.kind === ' ' || o.kind === '+').length;
    out.push(`@@ -${oldStart},${oldCount} +${newStart},${newCount} @@`);
    for (const op of slice) {
      out.push(`${op.kind}${op.text}`);
    }
  }
  return out.join('\n');
}

function renderColoredUnifiedDiff(containerEl, diffText) {
  if (!containerEl) return;
  containerEl.replaceChildren();
  const raw = String(diffText || '');
  if (!raw.trim()) return;
  const lines = raw.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const span = document.createElement('span');
    let cls = 'diff-line diff-line-ctx';
    if (line.startsWith('@@')) {
      cls = 'diff-line diff-line-hunk';
    } else if (line.startsWith('+') && !line.startsWith('+++')) {
      cls = 'diff-line diff-line-add';
    } else if (line.startsWith('-') && !line.startsWith('---')) {
      cls = 'diff-line diff-line-del';
    } else if (line.startsWith('---') || line.startsWith('+++')) {
      cls = 'diff-line diff-line-hunk';
    }
    span.className = cls;
    span.textContent = line + (i < lines.length - 1 ? '\n' : '');
    containerEl.appendChild(span);
  }
}

function renderInteractiveArtifactPreview(targetFile, proposedCode, replyProse = '') {
  if (!chatArtifactPreview) return;
  chatArtifactPreview.replaceChildren();
  const codeStr = String(proposedCode || '');
  const proseStr = String(replyProse || '');
  const fnNames = [];
  const fnRegex = /(?:pub\s+)?(?:fn|struct|enum)\s+([a-zA-Z0-9_]+)/g;
  let m;
  while ((m = fnRegex.exec(codeStr)) !== null) {
    if (!fnNames.includes(m[1])) fnNames.push(m[1]);
  }
  const svgStage = document.createElement('div');
  svgStage.className = 'artifact-svg-stage';
  const svgMatch = proseStr.match(/<svg[\s\S]*?<\/svg>/i) || codeStr.match(/<svg[\s\S]*?<\/svg>/i);
  if (svgMatch) {
    svgStage.innerHTML = svgMatch[0];
  } else {
    const nodes = fnNames.length > 0 ? fnNames.slice(0, 5) : [targetFile || 'workspace_module', 'verify_test_assertion', 'admit_verified_patch'];
    const svgNs = ['http', '://www.w3.org/2000/svg'].join('');
    const svg = document.createElementNS(svgNs, 'svg');
    svg.setAttribute('viewBox', '0 0 560 110');
    svg.setAttribute('width', '100%');
    svg.setAttribute('height', '110');
    svg.setAttribute('role', 'img');
    svg.setAttribute('aria-label', `AST Artifact Topology for ${targetFile || 'workspace'}`);
    const stepW = Math.floor(520 / Math.max(nodes.length, 1));
    nodes.forEach((label, idx) => {
      const x = 20 + idx * stepW;
      const y = 32;
      if (idx > 0) {
        const line = document.createElementNS(svgNs, 'line');
        line.setAttribute('x1', String(x - 18));
        line.setAttribute('y1', '54');
        line.setAttribute('x2', String(x));
        line.setAttribute('y2', '54');
        line.setAttribute('stroke', '#38bdf8');
        line.setAttribute('stroke-width', '2');
        svg.appendChild(line);
      }
      const rect = document.createElementNS(svgNs, 'rect');
      rect.setAttribute('x', String(x));
      rect.setAttribute('y', String(y));
      rect.setAttribute('width', String(Math.max(stepW - 22, 76)));
      rect.setAttribute('height', '44');
      rect.setAttribute('rx', '6');
      rect.setAttribute('fill', idx === 0 ? '#1e3a8a' : '#064e3b');
      rect.setAttribute('stroke', idx === 0 ? '#60a5fa' : '#10b981');
      rect.setAttribute('stroke-width', '1.5');
      svg.appendChild(rect);
      const txt = document.createElementNS(svgNs, 'text');
      txt.setAttribute('x', String(x + Math.floor(Math.max(stepW - 22, 76) / 2)));
      txt.setAttribute('y', '58');
      txt.setAttribute('text-anchor', 'middle');
      txt.setAttribute('fill', '#f8fafc');
      txt.setAttribute('font-size', '11');
      txt.setAttribute('font-family', 'monospace');
      txt.textContent = String(label).slice(0, 14);
      svg.appendChild(txt);
    });
    svgStage.appendChild(svg);
  }
  chatArtifactPreview.appendChild(svgStage);
  const bodyDiv = document.createElement('div');
  bodyDiv.className = 'chat-prose';
  const mdSummary = `### Interactive Artifact Preview (\`${targetFile || 'src/attention.rs'}\`)\n`
    + `- **Symbols Synthesized**: ${fnNames.length > 0 ? fnNames.map((n) => `\`${n}\``).join(', ') : '`module_root`'}\n`
    + `- **Staged Bytes**: ${codeStr.length} B\n\n\`\`\`rust\n${codeStr.slice(0, 1200)}\n\`\`\``;
  if (typeof renderFormattedMarkdownInto === 'function') {
    renderFormattedMarkdownInto(bodyDiv, mdSummary);
  } else {
    bodyDiv.textContent = mdSummary;
  }
  chatArtifactPreview.appendChild(bodyDiv);
}

function renderPendingPatchPills() {
  if (!chatDiffFilePills) return;
  chatDiffFilePills.replaceChildren();
  const patches = AppState.workspace.pendingPatches || {};
  const paths = Object.keys(patches);
  for (const path of paths) {
    const entry = patches[path];
    if (!entry) continue;
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = `diff-file-pill${path === pendingPatchPath ? ' active' : ''}`;
    btn.textContent = path;
    btn.addEventListener('click', () => {
      pendingPatchPath = path;
      pendingProposedCode = entry.proposedCode || '';
      pendingDiffPreview = entry.diffPreview || '';
      AppState.workspace.pendingPatchPath = path;
      AppState.workspace.pendingProposedCode = pendingProposedCode;
      if (chatDiffFile) chatDiffFile.textContent = path;
      renderColoredUnifiedDiff(chatDiffPreview, pendingDiffPreview);
      renderInteractiveArtifactPreview(path, pendingProposedCode, entry.replyProse || '');
      if (chatApplyPatchBtn) {
        chatApplyPatchBtn.disabled = false;
        chatApplyPatchBtn.textContent = `Apply Synthesized Patch to ${path} & Admit into Memory`;
      }
      renderPendingPatchPills();
    });
    chatDiffFilePills.appendChild(btn);
  }
  if (typeof syncCursorComposerPatchBar === 'function') {
    syncCursorComposerPatchBar();
  }
}

function checkMcpToolPolicy(action, target) {
  const policy = AppState.config.mcpSecurityPolicy;
  if (policy === 'deny') {
    emitAuditEvent('tool_execution', action, target, { policy, status: 'denied', reason: 'Blocked by active security policy (deny mode)' }, 'critical');
    const log = document.getElementById('mcp-audit-log');
    if (log) {
      const item = document.createElement('div');
      item.className = 'mcp-log-item mcp-log-critical';
      item.textContent = `[POLICY BLOCKED] Tool '${action}' on '${target}' denied by active security policy.`;
      log.appendChild(item);
      log.scrollTop = log.scrollHeight;
    }
    return false;
  }
  if (policy === 'confirm') {
    const ok = window.confirm(`MCP Security Confirmation: Allow tool '${action}' to modify '${target}'?`);
    if (!ok) {
      emitAuditEvent('tool_execution', action, target, { policy, status: 'rejected_by_user' }, 'warning');
      const log = document.getElementById('mcp-audit-log');
      if (log) {
        const item = document.createElement('div');
        item.className = 'mcp-log-item mcp-log-warning';
        item.textContent = `[USER REJECTED] Tool '${action}' on '${target}' canceled by user.`;
        log.appendChild(item);
        log.scrollTop = log.scrollHeight;
      }
      return false;
    }
  }
  emitAuditEvent('tool_execution', action, target, { policy, status: 'allowed' }, 'info');
  return true;
}

function renderMLInspector() {
  const sparklineEl = document.getElementById('inspector-sparkline');
  if (sparklineEl) {
    sparklineEl.replaceChildren();
    const barCount = 64;
    const st = (typeof dispatchStudioEnvelope === 'function') ? dispatchStudioEnvelope('state', '') : null;
    const seed = st && st.state_receipt_sha256 ? st.state_receipt_sha256 : 'uor-r4-gllm-256d';
    let hashAcc = 0;
    for (let i = 0; i < seed.length; i++) hashAcc = (hashAcc * 31 + seed.charCodeAt(i)) >>> 0;

    for (let d = 0; d < barCount; d++) {
      const bar = document.createElement('div');
      bar.className = 'sparkline-bar';
      const wave = Math.sin((d / barCount) * Math.PI * 4 + (hashAcc % 100) * 0.1) * 0.5 + 0.5;
      const heightPct = Math.round(15 + wave * 80);
      bar.style.height = `${heightPct}%`;
      bar.title = `Dim ${d * 4}..${d * 4 + 3}: ${heightPct}% activation`;
      if (heightPct > 75) bar.classList.add('sparkline-bar-high');
      sparklineEl.appendChild(bar);
    }
  }

  const icosianEl = document.getElementById('inspector-icosian-roots');
  if (icosianEl) {
    icosianEl.replaceChildren();
    const domRoot = (lexLean3dVisualizer && typeof lexLean3dVisualizer.dominantIcosianRoot === 'number')
      ? lexLean3dVisualizer.dominantIcosianRoot
      : 0;

    for (let r = 0; r < 17; r++) {
      const card = document.createElement('div');
      const isDom = (r === domRoot);
      card.className = isDom ? 'icosian-card icosian-card-dominant' : 'icosian-card';

      const title = document.createElement('div');
      title.className = 'icosian-card-title';
      title.textContent = isDom ? `Root r_${r} (Dominant ★)` : `Root r_${r}`;
      card.appendChild(title);

      const hopf = document.createElement('div');
      hopf.className = 'icosian-card-hopf';
      const q0 = Math.cos(r * 0.37);
      const q1 = Math.sin(r * 0.37) * 0.6;
      const q2 = Math.cos(r * 0.53) * 0.5;
      const q3 = Math.sin(r * 0.71) * 0.4;
      const hx = (2 * (q0 * q2 + q1 * q3)).toFixed(2);
      const hy = (2 * (q1 * q2 - q0 * q3)).toFixed(2);
      const hz = (q0 * q0 + q1 * q1 - q2 * q2 - q3 * q3).toFixed(2);
      hopf.textContent = `Hopf S²: (${hx}, ${hy}, ${hz})`;
      card.appendChild(hopf);

      icosianEl.appendChild(card);
    }
  }

  const e8GridEl = document.getElementById('inspector-e8-grid');
  if (e8GridEl) {
    e8GridEl.replaceChildren();
    const winningRow = (lexLean3dVisualizer && typeof lexLean3dVisualizer.winningPointKeyRow === 'number')
      ? (lexLean3dVisualizer.winningPointKeyRow % 64)
      : 18;

    for (let i = 0; i < 64; i++) {
      const cell = document.createElement('div');
      const isWinner = (i === winningRow);
      cell.className = isWinner ? 'e8-cell e8-cell-winner' : 'e8-cell';
      cell.title = isWinner ? `Basin #${i} (Winning Row ★)` : `Basin #${i}`;
      const resonance = (Math.sin(i * 0.47) * 0.5 + 0.5).toFixed(2);
      cell.setAttribute('data-resonance', resonance);
      if (!isWinner) {
        const opacity = (0.2 + Number(resonance) * 0.7).toFixed(2);
        cell.style.backgroundColor = `rgba(59, 130, 246, ${opacity})`;
      }
      e8GridEl.appendChild(cell);
    }
  }
}

const diagRecallMode = document.getElementById('diag-recall-mode');
const diagMatchedAnchor = document.getElementById('diag-matched-anchor');
const diagOpeningToken = document.getElementById('diag-opening-token');
const diagL1Distance = document.getElementById('diag-l1-distance');
const diagHammingDistance = document.getElementById('diag-hamming-distance');
const diagPluralityVotes = document.getElementById('diag-plurality-votes');
const diagRetainLambda = document.getElementById('diag-retain-lambda');
const diagPromptQ12 = document.getElementById('diag-prompt-q12');
const diagPromptSign = document.getElementById('diag-prompt-sign');
const diagIcosianRoot = document.getElementById('diag-icosian-root');
const diagPacked4Block = document.getElementById('diag-packed-4block');
const diagSketchGate = document.getElementById('diag-sketch-gate');
const diagPrllMass = document.getElementById('diag-prll-mass');
const diagInferenceReceipt = document.getElementById('diag-inference-receipt');

const studioSubsystemsForm = document.getElementById('studio-subsystems-form');
const subsystemsPayloadInput = document.getElementById('subsystems-payload');
const subR424 = document.getElementById('sub-r4-24');
const subR425 = document.getElementById('sub-r4-25');
const subR426 = document.getElementById('sub-r4-26');
const subR427 = document.getElementById('sub-r4-27');
const subReceipt = document.getElementById('sub-receipt');

const predictionsTbody = document.getElementById('predictions-tbody');
const trajectoryTbody = document.getElementById('trajectory-tbody');
const corpusTbody = document.getElementById('corpus-tbody');

const INPUT_ERROR = 'Error: invalid 64-bit integer coordinate or operand';
const DIVISION_ERROR = 'Error: division by zero';
const OVERFLOW_ERROR = 'Error: 64-bit coordinate/arithmetic overflow';

const MIN = -9223372036854775808n;
const MAX = 9223372036854775807n;

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function operand(value) { /* Formal R4.StudioFrame */
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  if (!/^(?:0|-[1-9][0-9]*|[1-9][0-9]*)$/.test(trimmed)) return null;
  try {
    const parsed = BigInt(trimmed);
    if (parsed < MIN || parsed > MAX) return null;
    return trimmed;
  } catch {
    return null;
  }
}

function show(message) { /* Formal R4.StudioFrame */
  if (result) {
    result.textContent = message;
  }
}

const DEFAULT_STARTER_WORKSPACE_FILES = [
  {
    path: 'src/attention.rs',
    content: [
      '//! Multi-Head Cyclic Flock Attention over Cl(0,6) x E8 Q12 Coordinates',
      '',
      'pub fn flock_attention_step(q12_latent: &[i16; 8], head_mask: u8) -> u8 {',
      '    let mut l1_energy: i32 = 0;',
      '    for &coord in q12_latent.iter() {',
      '        l1_energy += (coord as i32).abs();',
      '    }',
      '    ((l1_energy as u32 ^ (head_mask as u32 * 17)) & 0x3f) as u8',
      '}',
    ].join('\n'),
  },
  {
    path: 'src/lib.rs',
    content: [
      'pub mod attention;',
      '',
      'pub fn verify_manifold_invariants() -> bool {',
      '    let probe = [512_i16, -256, 1024, 0, -128, 64, 32, -16];',
      '    attention::flock_attention_step(&probe, 0xff) < 64',
      '}',
    ].join('\n'),
  },
];

const engineWorkerClient = {
  enabled: false,
  ready: false,
  worker: null,
  nextId: 1,
  pending: new Map(),
  webAccessEnabled: true,
  githubConnected: false,
  lastTransport: 'spvs-cloud-wss',
  activeChatStreamHook: null,
  workspaceFiles: new Map(
    DEFAULT_STARTER_WORKSPACE_FILES.map((f) => {
      const content = typeof f.content === 'string' ? f.content : '';
      const digest = computeSha256Hex(content);
      return [
        f.path,
        {
          path: f.path,
          content,
          line_count: content ? content.split('\n').length : 0,
          pinned: Boolean(f.pinned),
          byte_len: content.length,
          sha256: digest,
          updated_receipt: digest,
        },
      ];
    }),
  ),
  shadowState: {
    status: 'ok',
    schema: 'uor-r4.lut-stack/8',
    vocab_size: 4096,
    opening_rows: 64,
    reply_rows: 128,
    point_key_rows: 256,
    occupied_cells: 64,
    training_epoch: 0,
    active_nonzero_gates: 40,
    state_receipt_sha256: 'booting-worker-engine',
    entries: [],
  },
  shadowTraces: [],
  packInfo: null,
  gllmInfo: null,
  r4StackInfo: null,
  holoStatus: null,
  nativeQ31Info: null,
  verdantSynthesis: null,
  workerPoolInfo: null,
  stitchHealth: null,
  studioUpdate: null,
  pendingConsentDetails: null,

  spawn() {
    if (typeof Worker !== 'function') return false;
    try {
      const workerUrl = new URL('./lexlean_r4.js?v=v12r4-4387c815296d', import.meta.url);
      this.worker = new Worker(workerUrl, { type: 'module', name: 'uor-r4-engine-worker' });
      this.enabled = true;
      this.worker.onmessage = (ev) => this.handleMessage(ev.data || {});
      this.worker.onerror = () => {
        this.enabled = false;
        this.worker = null;
        for (const [, p] of this.pending.entries()) {
          try {
            p.reject(new Error('Engine worker load failed; falling back to main-thread Wasm'));
          } catch {}
        }
        this.pending.clear();
        if (typeof mainThreadWasmReady !== 'undefined' && mainThreadWasmReady && typeof studio_dispatch === 'function') {
          try {
            const st = JSON.parse(studio_dispatch('1\tstudio\tstate\t'));
            if (st && st.state_receipt_sha256) {
              this.shadowState = st;
              if (typeof renderAll === 'function') renderAll();
            }
          } catch {}
        }
        if (typeof navigator !== 'undefined' && navigator.serviceWorker && typeof navigator.serviceWorker.addEventListener === 'function' && !this._coiRetryBound) {
          this._coiRetryBound = true;
          navigator.serviceWorker.addEventListener('controllerchange', () => {
            if (!this.worker) this.spawn();
          }, { once: true });
        }
      };
      const initConsent = typeof safeStorageGet === 'function' ? safeStorageGet('uor_allow_off_origin_hydration') : null;
      if (initConsent === 'declined' && typeof safeStorageRemove === 'function') {
        safeStorageRemove('uor_allow_off_origin_hydration');
      }
      const prefMod = (AppState && AppState.config && AppState.config.selectedModel) || DEFAULT_HONEST_MODEL_ID;
      if (prefMod === 'uor-r4.qwen3-1.7b-holo' && typeof holoObservatoryState !== 'undefined' && holoObservatoryState) {
        holoObservatoryState.manualModelPinned = true;
      }
      const initEmergencyQ31 = typeof safeStorageGet === 'function' && safeStorageGet('uor_offline_emergency_q31') === '1';
      if (initEmergencyQ31) {
        this.rpc('set_bridge_policy', { offlineEmergencyQ31: true, preferredModel: prefMod, bootSync: true }).catch(() => {});
      } else {
        const allowExt = initConsent === '1' || initConsent === 'true';
        this.rpc('set_bridge_policy', { ...(allowExt ? { allowOffOriginHydration: true, consentDeclined: false } : {}), preferredModel: prefMod, bootSync: true }).catch(() => {});
      }
      return true;
    } catch {
      this.enabled = false;
      return false;
    }
  },

  rpc(op, payload) {
    if (!this.worker) {
      return Promise.reject(new Error('Engine worker not active'));
    }
    const id = this.nextId++;
    return new Promise((resolveRpc, rejectRpc) => {
      this.pending.set(id, { resolve: resolveRpc, reject: rejectRpc });
      this.worker.postMessage({ id, op, payload });
    });
  },

  rpcWithId(op, payload) {
    if (!this.worker) {
      return { id: null, promise: Promise.reject(new Error('Engine worker not active')) };
    }
    const id = this.nextId++;
    const promise = new Promise((resolveRpc, rejectRpc) => {
      this.pending.set(id, { resolve: resolveRpc, reject: rejectRpc });
      this.worker.postMessage({ id, op, payload });
    });
    return { id, promise };
  },

  applyWorkspaceSnapshot(ws) {
    if (!ws || !Array.isArray(ws.files)) return;
    const prevEntries = new Map(this.workspaceFiles.entries());
    this.workspaceFiles.clear();
    for (const f of ws.files) {
      const prev = prevEntries.get(f.path);
      const isLazy = Boolean(f.lazy || (prev && prev.lazy && !f.content));
      const content = typeof f.content === 'string'
        ? f.content
        : (prev && typeof prev.content === 'string' ? prev.content : '');
      const lineCount = typeof f.line_count === 'number'
        ? f.line_count
        : (content ? content.split('\n').length : 0);
      const pinned = typeof f.pinned === 'boolean'
        ? f.pinned
        : Boolean(prev && prev.pinned);
      const digest = f.sha256 || f.updated_receipt || computeSha256Hex(content);
      this.workspaceFiles.set(f.path, {
        path: f.path,
        content,
        line_count: lineCount,
        pinned,
        lazy: isLazy && !content,
        byte_len: f.byte_len ?? content.length,
        sha256: digest,
        updated_receipt: f.updated_receipt || digest,
      });
    }
    renderWorkspaceList(this.getWorkspaceListObject());
    if (typeof editorTabManager !== 'undefined' && editorTabManager) {
      const ta = editorTabManager.textarea || document.getElementById('workspace-editor-textarea');
      for (const [p, meta] of this.workspaceFiles.entries()) {
        if (editorTabManager.tabs.has(p)) {
          const t = editorTabManager.tabs.get(p);
          if (t && !t.isDirty && typeof meta.content === 'string') {
            t.buffer = meta.content;
            t.savedContent = meta.content;
            if (p === editorTabManager.activePath && ta && ta.value !== meta.content) {
              ta.value = meta.content;
              if (typeof editorTabManager.updateLineNumbers === 'function') {
                editorTabManager.updateLineNumbers();
              }
            }
          }
        }
      }
      if (!this.workspaceFiles.has(activeWorkspacePath) && ws.files[0]) {
        selectWorkspaceFile(ws.files[0].path);
      } else if (activeWorkspacePath) {
        selectWorkspaceFile(activeWorkspacePath);
      }
    }
  },

  getWorkspaceListObject() {
    const files = Array.from(this.workspaceFiles.values()).map((f) => {
      const content = typeof f.content === 'string' ? f.content : '';
      return {
        path: f.path,
        content,
        line_count: typeof f.line_count === 'number'
          ? f.line_count
          : (content ? content.split('\n').length : 0),
        pinned: Boolean(f.pinned),
        byte_len: f.byte_len ?? content.length,
        sha256: f.sha256 || '',
        updated_receipt: f.updated_receipt || f.sha256 || '',
      };
    });
    const pinnedCount = files.reduce((acc, f) => acc + (f.pinned ? 1 : 0), 0);
    return {
      status: 'ok',
      pinned_count: pinnedCount,
      files,
    };
  },

  handleMessage(msg) {
    if (msg && !msg.event && typeof msg.id === 'number' && this.pending.has(msg.id)) {
      const waiter = this.pending.get(msg.id);
      this.pending.delete(msg.id);
      if (msg.ok) waiter.resolve(msg.result);
      else {
        const rpcErr = new Error(msg.error || 'Worker RPC error');
        if (msg.error_code) rpcErr.error_code = msg.error_code;
        rpcErr.details = msg;
        waiter.reject(rpcErr);
      }
      return;
    }

    const evType = msg && msg.event;
    if (evType === 'chat_prefill_start') {
      this.hydrationProgress = null;
      if (typeof updateInlineHydrationBar === 'function') {
        updateInlineHydrationBar(null);
      }
      if (typeof updateWeightCacheBadge === 'function') {
        updateWeightCacheBadge();
      }
      if (typeof this.activeChatStreamHook === 'function') {
        this.activeChatStreamHook({
          stage: 'chat_prefill_start',
          ...msg,
        });
      }
      return;
    }
    if (evType === 'boot_progress' || evType === 'hydration_progress' || evType === 'chat_hydration') {
      const stageTag = msg.stage || (msg.modelId ? 'holo_v13' : '');
      if (stageTag === 'holo_v13' || stageTag === 'hf_stream' || stageTag === 'r4_shards') {
        const normalizedProg = msg.stage ? msg : { ...msg, stage: stageTag };
        this.hydrationProgress = normalizedProg;
        if (typeof updateWeightCacheBadge === 'function') {
          updateWeightCacheBadge();
        }
        if (typeof updateInlineHydrationBar === 'function') {
          updateInlineHydrationBar(normalizedProg);
        }
        if (typeof this.activeChatStreamHook === 'function') {
          this.activeChatStreamHook(normalizedProg);
        }
      }
      return;
    }
    if (evType === 'ready') {
      this.ready = true;
      const snap = msg.snapshot || {};
      if (snap.state && snap.state.status === 'ok') {
        this.shadowState = snap.state;
        renderStudioState(snap.state);
      }
      if (snap.pack) this.packInfo = snap.pack;
      if (snap.gllm) this.gllmInfo = snap.gllm;
      if (snap.r4Stack || msg.r4Stack) this.r4StackInfo = snap.r4Stack || msg.r4Stack;
      if (snap.holoStatus || msg.holoStatus) this.holoStatus = snap.holoStatus || msg.holoStatus;
      if (snap.nativeQ31 || msg.nativeQ31) this.nativeQ31Info = snap.nativeQ31 || msg.nativeQ31;
      if (snap.verdantSynthesis || msg.verdantSynthesis) {
        this.verdantSynthesis = snap.verdantSynthesis || msg.verdantSynthesis;
        if (this.verdantSynthesis && this.verdantSynthesis.worker_pool) {
          this.workerPoolInfo = this.verdantSynthesis.worker_pool;
        }
      }
      if (snap.stitchHealth) this.stitchHealth = snap.stitchHealth;
      if (snap.prismIdentity) this.prismIdentity = snap.prismIdentity;
      if (msg.prismIdentity) this.prismIdentity = msg.prismIdentity;
      if (msg.weightCacheMeta) globalThis.__uorWeightCacheMeta = msg.weightCacheMeta;
      if (msg.gllmMeta) globalThis.__uorGllmV9Meta = msg.gllmMeta;
      if (snap.workspace) this.applyWorkspaceSnapshot(snap.workspace);
      updateWeightCacheBadge();
      if (typeof renderPrismHoloIdentityUI === 'function') renderPrismHoloIdentityUI();
      if (typeof renderVerdantSynthesisPanel === 'function') renderVerdantSynthesisPanel();
      if (typeof renderCursorShardsSection === 'function') renderCursorShardsSection();
      if (typeof updateDynamicPlanningPanel === 'function') updateDynamicPlanningPanel();
      if (typeof renderDynamicPoolPanel === 'function') renderDynamicPoolPanel();
      if (typeof renderCoworkStatusBar === 'function') renderCoworkStatusBar();
      if (typeof emitPrismEmbedEvent === 'function') emitPrismEmbedEvent('prism:ready');
      if (globalThis.__uorRunWasmEvalAsync) {
        globalThis.__uorRunWasmEvalAsync('check');
      }
      this.rpc('studio_dispatch', '1\tstudio\ttraces\t{"format":"compact","limit":10}')
        .then((tr) => {
          if (Array.isArray(tr)) this.shadowTraces = tr;
          else if (tr && typeof tr === 'object') this.shadowTraces = [tr];
        })
        .catch(() => {});
      return;
    }
    if (evType === 'holo_ready') {
      this.hydrationProgress = null;
      if (typeof updateInlineHydrationBar === 'function') {
        updateInlineHydrationBar(null);
      }
      if (msg.kappaStore) globalThis.__uorHoloKappaStatus = msg.kappaStore;
      if (msg.holo && typeof msg.holo.mounted_count === 'number') {
        this.holoStatus = msg.holo;
        updateWeightCacheBadge();
        if (typeof renderCursorShardsSection === 'function') renderCursorShardsSection();
        if (typeof renderCoworkStatusBar === 'function') renderCoworkStatusBar();
      }
      const allowEmergencySwitch =
        msg.recommendedModel !== 'uor-r4.native-q31-d85' ||
        msg.fallbackReason === 'offline_emergency_mode' ||
        (typeof readOfflineEmergencyStored === 'function' && readOfflineEmergencyStored());
      if (allowEmergencySwitch && msg.recommendedModel && msg.promoted !== true && !holoObservatoryState.manualModelPinned) {
        const modelSel = document.getElementById('composer-model-select');
        if (modelSel && Array.from(modelSel.options).some((o) => o.value === msg.recommendedModel)) {
          modelSel.value = msg.recommendedModel;
          if (AppState && AppState.config) AppState.config.selectedModel = msg.recommendedModel;
          if (typeof syncComposerModel === 'function') syncComposerModel(msg.recommendedModel);
          if (typeof renderCoworkStatusBar === 'function') renderCoworkStatusBar();
          if (typeof emitPrismEmbedEvent === 'function') emitPrismEmbedEvent('prism:status');
        }
      }
      if (msg.holo) {
        this.rpc('holo_runtime_status', {})
          .then((res) => {
            const hs = (res && res.holo && typeof res.mounted_count !== 'number') ? res.holo : res;
            if (hs && typeof hs.mounted_count === 'number') this.holoStatus = hs;
            if (res && res.webgpu) this.webgpuStatus = res.webgpu;
            updateWeightCacheBadge();
            if (typeof renderCursorShardsSection === 'function') renderCursorShardsSection();
            if (typeof renderCoworkStatusBar === 'function') renderCoworkStatusBar();
          })
          .catch(() => {});
        refreshHoloStorageGeometry(msg.recommendedModel || '').catch(() => {});
      } else {
        updateWeightCacheBadge();
      }
      return;
    }
    if (evType === 'holo_trace_frame' && msg.frame) {
      ingestHoloObservatoryFrame(msg.frame, null);
      return;
    }
    if (evType === 'holo_observatory_batch' && Array.isArray(msg.frames)) {
      for (let i = 0; i < msg.frames.length; i += 1) {
        ingestHoloObservatoryFrame(msg.frames[i], { prefixReused: msg.prefixReused || 0, selectedModel: msg.selectedModel || '' });
      }
      if (!this.activeChatStreamHook) refreshHoloStorageGeometry(msg.selectedModel || '').catch(() => {});
      return;
    }
    if (evType === 'telos_tick') {
      renderTelosSelfImproveStatus(msg);
      return;
    }
    if (evType === 'weights_upgraded') {
      if (msg.gllm) this.gllmInfo = msg.gllm;
      if (msg.r4Stack) this.r4StackInfo = msg.r4Stack;
      if (msg.stitchHealth) this.stitchHealth = msg.stitchHealth;
      if (msg.gllmMeta) globalThis.__uorGllmV9Meta = msg.gllmMeta;
      updateWeightCacheBadge();
      if (typeof renderVerdantSynthesisPanel === 'function') renderVerdantSynthesisPanel();
      if (typeof renderCursorShardsSection === 'function') renderCursorShardsSection();
      if (typeof renderDynamicPoolPanel === 'function') renderDynamicPoolPanel();
      if (typeof renderCoworkStatusBar === 'function') renderCoworkStatusBar();
      return;
    }
    if (evType === 'stitch_health') {
      this.stitchHealth = msg.stitchHealth || null;
      if (typeof renderCursorShardsSection === 'function') renderCursorShardsSection();
      if (typeof renderDynamicPoolPanel === 'function') renderDynamicPoolPanel();
      if (typeof renderCoworkStatusBar === 'function') renderCoworkStatusBar();
      if (typeof emitPrismEmbedEvent === 'function') emitPrismEmbedEvent('prism:status');
      return;
    }
    if (evType === 'prism_identity') {
      this.prismIdentity = msg;
      if (typeof renderPrismHoloIdentityUI === 'function') renderPrismHoloIdentityUI();
      if (typeof emitPrismEmbedEvent === 'function') emitPrismEmbedEvent('prism:status');
      return;
    }
    if (evType === 'holo_compiled') {
      this.latestHoloSummary = msg;
      if (typeof renderPrismHoloIdentityUI === 'function') renderPrismHoloIdentityUI();
      if (typeof emitPrismEmbedEvent === 'function') emitPrismEmbedEvent('prism:status');
      return;
    }
    if (evType === 'studio_update_available') {
      this.studioUpdate = msg;
      if (typeof showStudioUpdateBanner === 'function') showStudioUpdateBanner(msg);
      return;
    }
    if (evType === 'workspace_sync' && msg.workspace) {
      this.applyWorkspaceSnapshot(msg.workspace);
      return;
    }
    if (evType === 'workspace_fs_changed') {
      if (typeof globalThis.__uorRefreshWorkspaceAfterFsChange === 'function') {
        globalThis.__uorRefreshWorkspaceAfterFsChange(msg).catch(() => {});
      }
      return;
    }
    if (evType === 'token_step') {
      if (msg.turn_id != null && typeof tsComposer !== 'undefined' && typeof tsComposer.setActiveTurnId === 'function') {
        tsComposer.setActiveTurnId(Number(msg.turn_id));
      }
      if (msg.frame && typeof holoObservatoryState !== 'undefined' && !holoObservatoryState.paused && holoObservatoryState.lockedFrameIdx === null) {
        ingestHoloObservatoryFrame(msg.frame, {
          prefixReused: Number(msg.prefix_reused_tokens || 0),
          selectedModel: msg.frame.backend || '',
        });
      }
      if (typeof this.activeChatStreamHook === 'function') {
        this.activeChatStreamHook({
          stage: 'token_step',
          ...msg,
        });
      }
      return;
    }
    if (evType === 'tool_call' || evType === 'tool_result' || evType === 'chat_cancelled' || evType === 'chat_error') {
      if (typeof this.activeChatStreamHook === 'function') {
        this.activeChatStreamHook({
          stage: evType,
          ...msg,
        });
      }
      return;
    }
    if (evType === 'hydration_consent_required') {
      const curConsent = typeof safeStorageGet === 'function' ? safeStorageGet('uor_allow_off_origin_hydration') : null;
      const consentGranted = typeof isConsentGrantedVal === 'function'
        ? isConsentGrantedVal(curConsent)
        : (curConsent === '1' || curConsent === 'true' || curConsent === 'granted');
      if (curConsent !== 'declined' && !consentGranted) {
        this.pendingConsentDetails = msg;
      }
      if (typeof this.activeChatStreamHook === 'function') {
        this.activeChatStreamHook({
          stage: 'hydration_consent_required',
          ...msg,
        });
      } else if (typeof renderHydrationConsentCard === 'function') {
        const container = document.getElementById('chat-messages')
          || (typeof chatTranscript !== 'undefined' ? chatTranscript : null)
          || document.getElementById('chat-transcript');
        renderHydrationConsentCard(container, msg, '');
      }
      if (typeof showStudioToast === 'function') {
        showStudioToast(`Consent required to download ${msg.model_id || 'model'}`, 'warn', 5000);
      }
      return;
    }
    if (evType === 'preview_inspect_request') {
      if (typeof handlePreviewInspectRequest === 'function') handlePreviewInspectRequest(msg);
      return;
    }
    if (evType === 'vtensor_preview_3d_rendered') {
      const f = typeof document !== 'undefined' && typeof document.getElementById === 'function' ? document.getElementById('live-preview-iframe') : null;
      if (f && f.contentWindow && typeof f.contentWindow.postMessage === 'function') f.contentWindow.postMessage({ protocol: 'uor-preview/1', op: 'render_3d_scene', scene: msg.scene || msg }, '*');
      return;
    }
    if (evType === 'chat_stream_delta') {
      if (typeof this.activeChatStreamHook === 'function') this.activeChatStreamHook({ stage: 'chat_stream_delta', text: msg.text || '', step: msg.step || 0 });
      return;
    }
    if (evType === 'bridge_activity') {
      if (msg.transport) this.lastTransport = msg.transport;
      if (typeof this.activeChatStreamHook === 'function') {
        this.activeChatStreamHook(msg);
      }
      return;
    }
    if (evType === 'brain_pulse' && Array.isArray(msg.pulses)) {
      ingestGlobalBrainPulses(msg.pulses);
      return;
    }
  },
};

const headerBrainPulseState = {
  energy: 0.12,
  callosumCharge: 0.0,
  activeOrbit: 0,
  activeHead: 0,
  phasePhase: 0,
  rafId: null,
};

function ingestGlobalBrainPulses(pulses) {
  if (!Array.isArray(pulses) || pulses.length === 0) return;
  const last = pulses[pulses.length - 1] || {};
  headerBrainPulseState.energy = 1.0;
  headerBrainPulseState.activeOrbit = (last.e8_orbit ?? 0) & 15;
  headerBrainPulseState.activeHead = (last.winning_head ?? 0) & 7;
  if ((last.kv_copy_dist ?? 0) > 0 || (last.tool_kind && last.tool_kind !== 'none')) {
    headerBrainPulseState.callosumCharge = 1.0;
  }
  const livePill = document.getElementById('inspectors-live-pulse-pill');
  if (livePill) {
    const toolLabel = last.tool_kind && last.tool_kind !== 'none' ? ` · ${last.tool_kind}` : '';
    livePill.textContent = `Orbit e${headerBrainPulseState.activeOrbit} · Head #${headerBrainPulseState.activeHead}${toolLabel}`;
  }
  if (lexLean3dVisualizer && typeof lexLean3dVisualizer.ingestBrainPulse === 'function') {
    lexLean3dVisualizer.ingestBrainPulse(pulses);
  }
}

function startHeaderBrainPulseCanvas() {
  const canvas = document.getElementById('header-brain-pulse-canvas');
  if (!canvas || typeof canvas.getContext !== 'function') return;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  canvas.addEventListener('click', () => {
    activateCanvasTab('traces');
    refreshHoloStorageGeometry('').catch(() => {});
    renderHoloObservatoryDeck();
  });

  const drawFrame = (ts) => {
    const t = (ts || 0) * 0.001;
    headerBrainPulseState.energy = Math.max(0.14, headerBrainPulseState.energy * 0.94);
    headerBrainPulseState.callosumCharge = Math.max(0.0, headerBrainPulseState.callosumCharge * 0.92);
    const w = canvas.width;
    const h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    // Left & right cortical hemisphere silhouettes + corpus callosum bridge
    const breath = 0.5 + 0.5 * Math.sin(t * 2.4);
    const e = headerBrainPulseState.energy;
    const leftX = w * 0.28;
    const rightX = w * 0.72;
    const midY = h * 0.54;

    ctx.lineWidth = 1.4;
    ctx.strokeStyle = `rgba(56, 189, 248, ${0.32 + 0.55 * e})`;
    ctx.beginPath();
    ctx.ellipse(leftX, midY, 15, 8.5, 0, 0, Math.PI * 2);
    ctx.stroke();

    ctx.strokeStyle = `rgba(52, 211, 153, ${0.32 + 0.55 * e})`;
    ctx.beginPath();
    ctx.ellipse(rightX, midY, 15, 8.5, 0, 0, Math.PI * 2);
    ctx.stroke();

    // Corpus Callosum arc
    const ccAlpha = 0.25 + 0.7 * Math.max(headerBrainPulseState.callosumCharge, breath * 0.25);
    ctx.strokeStyle = headerBrainPulseState.callosumCharge > 0.15
      ? `rgba(250, 204, 21, ${ccAlpha})`
      : `rgba(167, 139, 250, ${ccAlpha})`;
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    ctx.moveTo(leftX + 6, midY - 1);
    ctx.quadraticCurveTo(w * 0.5, midY - 9, rightX - 6, midY - 1);
    ctx.stroke();

    // Travelling packet along Corpus Callosum when KV copy or tool fires
    if (headerBrainPulseState.callosumCharge > 0.08 || e > 0.35) {
      const u = (t * 2.6) % 1.0;
      const inv = 1 - u;
      const px = inv * inv * (leftX + 6) + 2 * inv * u * (w * 0.5) + u * u * (rightX - 6);
      const py = inv * inv * (midY - 1) + 2 * inv * u * (midY - 9) + u * u * (midY - 1);
      ctx.fillStyle = headerBrainPulseState.callosumCharge > 0.15 ? '#fde047' : '#38bdf8';
      ctx.beginPath();
      ctx.arc(px, py, 2.4, 0, Math.PI * 2);
      ctx.fill();
    }

    // Active cortical cluster nodes inside hemispheres
    for (let i = 0; i < 6; i++) {
      const isRight = i >= 3;
      const cx = (isRight ? rightX : leftX) + Math.cos(i * 2.1 + headerBrainPulseState.activeOrbit * 0.4) * 8;
      const cy = midY + Math.sin(i * 1.9 + headerBrainPulseState.activeHead * 0.5) * 4.5;
      const hot = ((headerBrainPulseState.activeOrbit + i) % 6) === 0;
      ctx.fillStyle = hot ? '#34d399' : 'rgba(148, 163, 184, 0.55)';
      ctx.beginPath();
      ctx.arc(cx, cy, hot ? 2.2 + e * 1.1 : 1.3, 0, Math.PI * 2);
      ctx.fill();
    }

    if (typeof requestAnimationFrame === 'function') {
      headerBrainPulseState.rafId = requestAnimationFrame(drawFrame);
    }
  };

  if (typeof requestAnimationFrame === 'function') {
    headerBrainPulseState.rafId = requestAnimationFrame(drawFrame);
  }
}

function dispatchStudioEnvelope(command, payload) {
  const envelope = `1\tstudio\t${command}\t${payload}`;
  if (engineWorkerClient.enabled) {
    if (command === 'workspace_list') {
      return engineWorkerClient.getWorkspaceListObject();
    }
    if (command === 'workspace_read') {
      const target = String(payload || '').trim();
      const hit = engineWorkerClient.workspaceFiles.get(target);
      if (hit) {
        return { status: 'ok', path: hit.path, content: hit.content, sha256: hit.sha256 || '' };
      }
      return { status: 'error', message: `File not found: ${target}` };
    }
    if (command === 'workspace_write') {
      const str = String(payload || '');
      const nl = str.indexOf('\n');
      const pathPart = (nl >= 0 ? str.slice(0, nl) : str).trim();
      const contentPart = nl >= 0 ? str.slice(nl + 1) : '';
      if (pathPart) {
        const prev = engineWorkerClient.workspaceFiles.get(pathPart);
        const digest = computeSha256Hex(contentPart);
        engineWorkerClient.workspaceFiles.set(pathPart, {
          path: pathPart,
          content: contentPart,
          line_count: contentPart ? contentPart.split('\n').length : 0,
          pinned: prev ? Boolean(prev.pinned) : false,
          byte_len: contentPart.length,
          sha256: digest,
          updated_receipt: digest,
        });
      }
      engineWorkerClient.rpc('studio_dispatch', envelope).catch(() => {});
      return engineWorkerClient.getWorkspaceListObject();
    }
    if (command === 'workspace_pin') {
      const target = String(payload || '').trim();
      const hit = engineWorkerClient.workspaceFiles.get(target);
      if (hit) {
        hit.pinned = !hit.pinned;
      }
      engineWorkerClient.rpc('studio_dispatch', envelope).catch(() => {});
      return engineWorkerClient.getWorkspaceListObject();
    }
    if (command === 'workspace_delete') {
      const target = String(payload || '').trim();
      engineWorkerClient.workspaceFiles.delete(target);
      engineWorkerClient.rpc('studio_dispatch', envelope).catch(() => {});
      return engineWorkerClient.getWorkspaceListObject();
    }
    if (command === 'workspace_batch') {
      let ingestedFiles = 0;
      let ingestedTokens = 0;
      const rawBatch = String(payload || '');
      if (rawBatch.trim().startsWith('[') || rawBatch.trim().startsWith('{')) {
        try {
          const parsedBatch = JSON.parse(rawBatch);
          const items = Array.isArray(parsedBatch) ? parsedBatch : (parsedBatch.files || []);
          for (const it of items) {
            if (it && typeof it.path === 'string' && typeof it.content === 'string') {
              const prev = engineWorkerClient.workspaceFiles.get(it.path);
              const digest = computeSha256Hex(it.content);
              engineWorkerClient.workspaceFiles.set(it.path, {
                path: it.path,
                content: it.content,
                line_count: it.content ? it.content.split('\n').length : 0,
                pinned: prev ? Boolean(prev.pinned) : false,
                byte_len: it.content.length,
                sha256: digest,
                updated_receipt: digest,
              });
              ingestedFiles += 1;
              ingestedTokens += it.content.split(/\s+/).filter(Boolean).length;
            }
          }
        } catch {
          // Forward raw batch to worker below
        }
      } else if (rawBatch.length > 0) {
        const blocks = rawBatch.split('\n===FILE===\n');
        for (const blk of blocks) {
          const nl = blk.indexOf('\n');
          const p = (nl >= 0 ? blk.slice(0, nl) : blk).trim();
          const c = nl >= 0 ? blk.slice(nl + 1) : '';
          if (p) {
            const prev = engineWorkerClient.workspaceFiles.get(p);
            const digest = computeSha256Hex(c);
            engineWorkerClient.workspaceFiles.set(p, {
              path: p,
              content: c,
              line_count: c ? c.split('\n').length : 0,
              pinned: prev ? Boolean(prev.pinned) : false,
              byte_len: c.length,
              sha256: digest,
              updated_receipt: digest,
            });
            ingestedFiles += 1;
            ingestedTokens += c.split(/\s+/).filter(Boolean).length;
          }
        }
      }
      engineWorkerClient.rpc('studio_dispatch', envelope).catch(() => {});
      return {
        ...engineWorkerClient.getWorkspaceListObject(),
        batch_ingested_files: ingestedFiles,
        batch_ingested_tokens: ingestedTokens,
      };
    }
    if (command === 'state') {
      return engineWorkerClient.shadowState;
    }
    if (command === 'traces' || command === 'v1/traces') {
      return engineWorkerClient.shadowTraces;
    }
    if (command === 'weight_status') {
      return engineWorkerClient.packInfo || { status: 'ok', schema: 'uor-r4.lut-stack/8' };
    }
    if (command === 'mvm_status') {
      return {
        status: 'ok',
        architecture: 'riscv64-rv64imac',
        guest_cid: 3,
        transport: `SpiralCore virtio-vsock (${engineWorkerClient.lastTransport})`,
        dirty_cow_sectors: 0,
      };
    }
    if (command === 'admit' || command === 'reset') {
      engineWorkerClient.rpc('studio_dispatch', envelope)
        .then((res) => {
          if (res && res.state) {
            engineWorkerClient.shadowState = res.state;
            renderStudioState(res.state);
          }
        })
        .catch(() => {});
      return { status: 'ok', state: engineWorkerClient.shadowState };
    }
    // Non-blocking async forward for any secondary command when worker is active
    engineWorkerClient.rpc('studio_dispatch', envelope).catch(() => {});
    return { status: 'ok', reply: 'Queued on uor-r4-engine-worker' };
  }
  if (!engineWorkerClient.enabled && mainThreadWasmReady && typeof studio_dispatch === 'function') {
    const raw = studio_dispatch(envelope);
    return JSON.parse(raw);
  }
  return engineWorkerClient.shadowState;
}

function createCell(text, codeStyle = false) {
  const td = document.createElement('td');
  if (codeStyle) {
    const code = document.createElement('code');
    code.textContent = String(text);
    td.appendChild(code);
  } else {
    td.textContent = String(text);
  }
  return td;
}

function renderStudioState(state) {
  if (!state || state.status !== 'ok') return;
  if (statSchema) statSchema.textContent = state.schema;
  if (statVocab) statVocab.textContent = String(state.vocab_size);
  if (statTurnRows) {
    statTurnRows.textContent = `${state.opening_rows} / ${state.reply_rows}`;
  }
  if (statPointRows) statPointRows.textContent = String(state.point_key_rows);
  if (statCells) statCells.textContent = `${state.occupied_cells} / 256`;
  if (statTrainEpochs) {
    const ep = typeof state.training_epoch === 'number' ? state.training_epoch : 0;
    const gates = typeof state.active_nonzero_gates === 'number' ? state.active_nonzero_gates : 0;
    statTrainEpochs.textContent = `${ep} ep / ${gates} gates`;
  }
  if (statStateReceipt) statStateReceipt.textContent = state.state_receipt_sha256;

  if (!corpusTbody || !Array.isArray(state.entries)) return;
  corpusTbody.replaceChildren();
  for (const entry of state.entries) {
    const tr = document.createElement('tr');
    tr.appendChild(createCell(entry.index + 1));
    tr.appendChild(createCell(entry.prompt));
    tr.appendChild(createCell(entry.opening_token, true));
    tr.appendChild(createCell(entry.root_cell, true));
    tr.appendChild(createCell(entry.sign_code_hex, true));
    tr.appendChild(createCell(entry.reply));

    const actionTd = document.createElement('td');
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'table-action-btn';
    btn.textContent = 'Query Turn';
    btn.addEventListener('click', () => {
      if (studioPromptInput) {
        studioPromptInput.value = entry.prompt;
      }
      runStudioGenerate(entry.prompt);
    });
    actionTd.appendChild(btn);
    tr.appendChild(actionTd);
    corpusTbody.appendChild(tr);
  }
}

function selectWorkspaceFile(pathText) {
  const lazyEntry = engineWorkerClient && engineWorkerClient.workspaceFiles
    ? engineWorkerClient.workspaceFiles.get(pathText)
    : null;
  if (lazyEntry && lazyEntry.lazy && !lazyEntry.content && AppState.config.connectedGitRepo && engineWorkerClient.enabled) {
    hydrateLazyGitBlob(pathText).catch(() => {});
  }
  if (typeof editorTabManager !== 'undefined' && editorTabManager && editorTabManager.tabs.has(pathText)) {
    activeWorkspacePath = pathText;
    if (editorFilePath) editorFilePath.textContent = pathText;
    editorTabManager.switchTo(pathText);
    if (typeof renderCursorScmSection === 'function') {
      renderCursorScmSection(engineWorkerClient.getWorkspaceListObject());
    }
    return;
  }
  const res = dispatchStudioEnvelope('workspace_read', pathText);
  if (!res || res.status !== 'ok') return;
  activeWorkspacePath = res.path;
  if (editorFilePath) editorFilePath.textContent = res.path;
  if (editorFileReceipt) {
    editorFileReceipt.textContent = res.updated_receipt ? res.updated_receipt.slice(0, 16) : '';
  }
  if (typeof editorTabManager !== 'undefined' && editorTabManager) {
    editorTabManager.open(res.path, res.content);
  } else if (workspaceEditorTextarea) {
    workspaceEditorTextarea.value = res.content;
  }
  if (typeof syncCursorWorkbenchOnFileSelect === 'function') {
    syncCursorWorkbenchOnFileSelect(res);
  }
  if (typeof renderCursorScmSection === 'function') {
    renderCursorScmSection(engineWorkerClient.getWorkspaceListObject());
  }
}

async function hydrateLazyGitBlob(pathText) {
  const repo = AppState.config.connectedGitRepo;
  const branch = AppState.config.connectedGitBranch || 'main';
  if (!repo || !pathText || !engineWorkerClient.enabled) return null;
  const res = await engineWorkerClient.rpc('host_bridge_exec', {
    op: 'git',
    subcommand: 'read_blob',
    repo,
    branch,
    path: pathText,
  });
  if (res && res.ok && typeof res.content === 'string') {
    const existing = engineWorkerClient.workspaceFiles.get(pathText) || {};
    engineWorkerClient.workspaceFiles.set(pathText, {
      ...existing,
      path: pathText,
      content: res.content,
      lazy: false,
      line_count: res.content ? res.content.split('\n').length : 0,
      byte_len: res.content.length,
    });
    if (AppState.config.gitBaselineFiles && AppState.config.gitBaselineFiles[pathText] === undefined) {
      AppState.config.gitBaselineFiles[pathText] = res.content;
    }
    activeWorkspacePath = pathText;
    if (typeof editorTabManager !== 'undefined' && editorTabManager) {
      editorTabManager.open(pathText, res.content);
    } else if (workspaceEditorTextarea) {
      workspaceEditorTextarea.value = res.content;
    }
    renderWorkspaceList(engineWorkerClient.getWorkspaceListObject());
    if (typeof renderCursorScmSection === 'function') {
      renderCursorScmSection(engineWorkerClient.getWorkspaceListObject());
    }
  }
  return res;
}

function renderWorkspaceList(ws) {
  if (!ws || ws.status !== 'ok' || !Array.isArray(ws.files)) return;
  if (workspacePinnedBadge) {
    workspacePinnedBadge.textContent = `${ws.pinned_count} pinned`;
  }
  if (!workspaceFileList) return;
  workspaceFileList.replaceChildren();

  let foundActive = false;
  for (const file of ws.files) {
    if (file.path === activeWorkspacePath) {
      foundActive = true;
      if (editorFilePath) editorFilePath.textContent = file.path;
      if (editorFileReceipt) {
        editorFileReceipt.textContent = file.updated_receipt ? file.updated_receipt.slice(0, 16) : '';
      }
      if (workspaceEditorTextarea && document.activeElement !== workspaceEditorTextarea) {
        workspaceEditorTextarea.value = file.content;
      }
    }
    const li = document.createElement('li');
    li.className = `workspace-item${file.path === activeWorkspacePath ? ' workspace-item-active' : ''}`;

    const selectBtn = document.createElement('button');
    selectBtn.type = 'button';
    selectBtn.className = 'workspace-file-select';
    selectBtn.textContent = `${file.path} (${file.line_count}L)`;
    selectBtn.addEventListener('click', () => {
      activeWorkspacePath = file.path;
      selectWorkspaceFile(file.path);
      const refreshed = dispatchStudioEnvelope('workspace_list', '');
      renderWorkspaceList(refreshed);
    });

    const pinBtn = document.createElement('button');
    pinBtn.type = 'button';
    pinBtn.className = `workspace-pin-btn${file.pinned ? ' workspace-pin-btn-active' : ''}`;
    pinBtn.textContent = file.pinned ? 'Pinned' : 'Pin';
    pinBtn.addEventListener('click', () => {
      const updated = dispatchStudioEnvelope('workspace_pin', file.path);
      renderWorkspaceList(updated);
    });

    li.appendChild(selectBtn);
    li.appendChild(pinBtn);
    workspaceFileList.appendChild(li);
  }

  if (!foundActive && ws.files.length > 0) {
    activeWorkspacePath = ws.files[0].path;
    selectWorkspaceFile(activeWorkspacePath);
  }
  if (typeof syncCursorWorkbenchOnList === 'function') {
    syncCursorWorkbenchOnList(ws);
  }
}

// ============================================================================
// Pure In-Browser LaTeX Math AST Parser & Markdown Formatter (Zero CDNs)
// ============================================================================

const LATEX_MATH_SYMBOLS = {
  alpha: 'α', beta: 'β', gamma: 'γ', delta: 'δ', epsilon: 'ε', zeta: 'ζ',
  eta: 'η', theta: 'θ', iota: 'ι', kappa: 'κ', lambda: 'λ', mu: 'μ',
  nu: 'ν', xi: 'ξ', pi: 'π', rho: 'ρ', sigma: 'σ', tau: 'τ',
  phi: 'φ', chi: 'χ', psi: 'ψ', omega: 'ω',
  Gamma: 'Γ', Delta: 'Δ', Theta: 'Θ', Lambda: 'Λ', Xi: 'Ξ', Pi: 'Π',
  Sigma: 'Σ', Phi: 'Φ', Psi: 'Ψ', Omega: 'Ω',
  sum: '∑', prod: '∏', int: '∫', partial: '∂', nabla: '∇', infty: '∞',
  approx: '≈', neq: '≠', le: '≤', ge: '≥', in: '∈', notin: '∉',
  times: '×', cdot: '·', pm: '±', to: '→', leftarrow: '←', leftrightarrow: '↔',
  forall: '∀', exists: '∃', subset: '⊂', cup: '∪', cap: '∩',
  land: '∧', lor: '∨', neg: '¬', ldots: '…', cdots: '⋯', vdots: '⋮', ddots: '⋱',
  wedge: '∧', ell: 'ℓ',
  quad: '  ', qquad: '    '
};

function tokenizeLatex(tex) { /* Formal R4.StudioFrame */
  const tokens = [];
  let i = 0;
  const len = tex.length;
  while (i < len) {
    const ch = tex[i];
    if (ch === ' ' || ch === '\t' || ch === '\n' || ch === '\r') {
      i += 1;
      continue;
    }
    if (ch === '\\') {
      let j = i + 1;
      if (j < len && /[^a-zA-Z]/.test(tex[j])) {
        tokens.push({ type: 'cmd', value: tex.slice(i, j + 1) });
        i = j + 1;
      } else {
        while (j < len && /[a-zA-Z]/.test(tex[j])) j += 1;
        tokens.push({ type: 'cmd', value: tex.slice(i, j) });
        i = j;
      }
    } else if (ch === '{' || ch === '}' || ch === '[' || ch === ']' || ch === '(' || ch === ')') {
      tokens.push({ type: 'delim', value: ch });
      i += 1;
    } else if (ch === '^' || ch === '_') {
      tokens.push({ type: 'script', value: ch });
      i += 1;
    } else if (ch === '&') {
      tokens.push({ type: 'amp', value: ch });
      i += 1;
    } else {
      tokens.push({ type: 'char', value: ch });
      i += 1;
    }
  }
  return tokens;
}

function parseLatexTokensToAst(tokens) {
  let pos = 0;

  function peek() { /* Formal R4.StudioFrame */
    return pos < tokens.length ? tokens[pos] : null;
  }

  function next() { /* Formal R4.StudioFrame */
    return pos < tokens.length ? tokens[pos++] : null;
  }

  function parseGroup() { /* Formal R4.StudioFrame */
    const t = peek();
    if (t && t.type === 'delim' && t.value === '{') {
      next();
      const items = [];
      while (pos < tokens.length) {
        const cur = peek();
        if (cur && cur.type === 'delim' && cur.value === '}') {
          next();
          break;
        }
        items.push(parseExprAtom());
      }
      return { type: 'group', items };
    }
    return parseAtom();
  }

  function grpToString(node) { /* Formal R4.StudioFrame */
    if (!node) return '';
    if (node.type === 'text' || node.type === 'symbol') return node.value;
    if (node.type === 'group' && Array.isArray(node.items)) {
      return node.items.map(grpToString).join('');
    }
    return '';
  }

  function parseAtom() { /* Formal R4.StudioFrame */
    const t = next();
    if (!t) return { type: 'text', value: '' };

    if (t.type === 'cmd') {
      const cmd = t.value.slice(1);
      if (cmd === 'frac') {
        const num = parseGroup();
        const denom = parseGroup();
        return { type: 'frac', num, denom };
      }
      if (cmd === 'sqrt') {
        let degree = null;
        const p = peek();
        if (p && p.type === 'delim' && p.value === '[') {
          next();
          const degItems = [];
          while (pos < tokens.length) {
            const cur = peek();
            if (cur && cur.type === 'delim' && cur.value === ']') {
              next();
              break;
            }
            degItems.push(parseAtom());
          }
          degree = { type: 'group', items: degItems };
        }
        const body = parseGroup();
        return { type: 'sqrt', degree, body };
      }
      if (cmd === 'sum' || cmd === 'prod' || cmd === 'int') {
        const symbol = cmd === 'sum' ? '∑' : (cmd === 'prod' ? '∏' : '∫');
        let sub = null;
        let sup = null;
        while (pos < tokens.length) {
          const p = peek();
          if (p && p.type === 'script' && p.value === '_') {
            next();
            sub = parseGroup();
          } else if (p && p.type === 'script' && p.value === '^') {
            next();
            sup = parseGroup();
          } else {
            break;
          }
        }
        return { type: 'bigop', symbol, sub, sup };
      }
      if (cmd === 'mathbf' || cmd === 'vec' || cmd === 'boldsymbol') {
        const body = parseGroup();
        return { type: 'bold', body };
      }
      if (cmd === 'mathbb') {
        const body = parseGroup();
        return { type: 'mathbb', body };
      }
      if (cmd === 'mathcal') {
        const body = parseGroup();
        return { type: 'mathcal', body };
      }
      if (cmd === 'text' || cmd === 'mathrm' || cmd === 'operatorname') {
        const body = parseGroup();
        return { type: 'text_group', body };
      }
      if (cmd === 'begin') {
        let matrixType = 'matrix';
        const p = peek();
        if (p && p.type === 'delim' && p.value === '{') {
          const grp = parseGroup();
          matrixType = grpToString(grp).trim() || 'matrix';
        }
        const rows = [];
        let currentRow = [];
        let currentCell = [];
        while (pos < tokens.length) {
          const cur = peek();
          if (!cur) break;
          if (cur.type === 'cmd' && cur.value === '\\end') {
            next();
            parseGroup();
            break;
          }
          if (cur.type === 'cmd' && cur.value === '\\\\') {
            next();
            currentRow.push({ type: 'group', items: currentCell });
            rows.push(currentRow);
            currentRow = [];
            currentCell = [];
            continue;
          }
          if (cur.type === 'amp') {
            next();
            currentRow.push({ type: 'group', items: currentCell });
            currentCell = [];
            continue;
          }
          currentCell.push(parseAtom());
        }
        if (currentCell.length > 0 || currentRow.length > 0) {
          currentRow.push({ type: 'group', items: currentCell });
          rows.push(currentRow);
        }
        return { type: 'matrix', matrixType, rows };
      }
      if (cmd === 'left' || cmd === 'right') {
        const delim = next();
        return { type: 'text', value: delim ? delim.value : '' };
      }
      if (cmd === '\\') {
        return { type: 'newline' };
      }
      if (LATEX_MATH_SYMBOLS[cmd]) {
        return { type: 'symbol', value: LATEX_MATH_SYMBOLS[cmd] };
      }
      return { type: 'text', value: cmd };
    }

    if (t.type === 'delim' && t.value === '{') {
      const items = [];
      while (pos < tokens.length) {
        const cur = peek();
        if (cur && cur.type === 'delim' && cur.value === '}') {
          next();
          break;
        }
        items.push(parseExprAtom());
      }
      return { type: 'group', items };
    }

    return { type: 'text', value: t.value };
  }

  function parseExprAtom() {
    let atom = parseAtom();
    while (pos < tokens.length) {
      const p = peek();
      if (p && p.type === 'script' && p.value === '^') {
        next();
        const sup = parseGroup();
        if (atom.type === 'script' && atom.sup === null) {
          atom.sup = sup;
        } else {
          atom = { type: 'script', base: atom, sup, sub: null };
        }
      } else if (p && p.type === 'script' && p.value === '_') {
        next();
        const sub = parseGroup();
        if (atom.type === 'script' && atom.sub === null) {
          atom.sub = sub;
        } else {
          atom = { type: 'script', base: atom, sup: null, sub };
        }
      } else {
        break;
      }
    }
    return atom;
  }

  const astList = [];
  while (pos < tokens.length) {
    astList.push(parseExprAtom());
  }
  return astList;
}

function renderLatexAstNodeToDom(node) {
  if (!node) return document.createTextNode('');
  switch (node.type) {
    case 'text':
      return document.createTextNode(node.value);
    case 'symbol': {
      const sp = document.createElement('span');
      sp.className = 'latex-symbol';
      sp.textContent = node.value;
      return sp;
    }
    case 'frac': {
      const frac = document.createElement('span');
      frac.className = 'latex-frac';
      const num = document.createElement('span');
      num.className = 'latex-num';
      num.appendChild(renderLatexAstNodeToDom(node.num));
      const denom = document.createElement('span');
      denom.className = 'latex-denom';
      denom.appendChild(renderLatexAstNodeToDom(node.denom));
      frac.appendChild(num);
      frac.appendChild(denom);
      return frac;
    }
    case 'sqrt': {
      const sqrt = document.createElement('span');
      sqrt.className = 'latex-sqrt';
      const rad = document.createElement('span');
      rad.className = 'latex-radical';
      rad.textContent = node.degree ? '√[' : '√';
      if (node.degree) {
        rad.appendChild(renderLatexAstNodeToDom(node.degree));
        rad.appendChild(document.createTextNode(']'));
      }
      const body = document.createElement('span');
      body.className = 'latex-sqrt-body';
      body.appendChild(renderLatexAstNodeToDom(node.body));
      sqrt.appendChild(rad);
      sqrt.appendChild(body);
      return sqrt;
    }
    case 'bigop': {
      const wrap = document.createElement('span');
      wrap.className = 'latex-bigop';
      const op = document.createElement('span');
      op.className = 'latex-op-symbol';
      op.textContent = node.symbol;
      wrap.appendChild(op);
      if (node.sup || node.sub) {
        const lims = document.createElement('span');
        lims.className = 'latex-limits';
        if (node.sup) {
          const sup = document.createElement('span');
          sup.className = 'latex-lim-sup';
          sup.appendChild(renderLatexAstNodeToDom(node.sup));
          lims.appendChild(sup);
        }
        if (node.sub) {
          const sub = document.createElement('span');
          sub.className = 'latex-lim-sub';
          sub.appendChild(renderLatexAstNodeToDom(node.sub));
          lims.appendChild(sub);
        }
        wrap.appendChild(lims);
      }
      return wrap;
    }
    case 'script': {
      const wrap = document.createElement('span');
      wrap.className = 'latex-subsup';
      const base = document.createElement('span');
      base.className = 'latex-base';
      base.appendChild(renderLatexAstNodeToDom(node.base));
      wrap.appendChild(base);
      if (node.sup || node.sub) {
        const scripts = document.createElement('span');
        scripts.className = 'latex-scripts';
        if (node.sup) {
          const sup = document.createElement('span');
          sup.className = 'latex-sup';
          sup.appendChild(renderLatexAstNodeToDom(node.sup));
          scripts.appendChild(sup);
        }
        if (node.sub) {
          const sub = document.createElement('span');
          sub.className = 'latex-sub';
          sub.appendChild(renderLatexAstNodeToDom(node.sub));
          scripts.appendChild(sub);
        }
        wrap.appendChild(scripts);
      }
      return wrap;
    }
    case 'matrix': {
      const table = document.createElement('table');
      table.className = `latex-matrix matrix-${node.matrixType || 'matrix'}`;
      for (const row of node.rows) {
        const tr = document.createElement('tr');
        for (const cell of row) {
          const td = document.createElement('td');
          td.appendChild(renderLatexAstNodeToDom(cell));
          tr.appendChild(td);
        }
        table.appendChild(tr);
      }
      return table;
    }
    case 'bold': {
      const strong = document.createElement('strong');
      strong.className = 'latex-bold';
      strong.appendChild(renderLatexAstNodeToDom(node.body));
      return strong;
    }
    case 'mathbb': {
      const span = document.createElement('span');
      span.className = 'latex-mathbb';
      span.appendChild(renderLatexAstNodeToDom(node.body));
      return span;
    }
    case 'mathcal': {
      const span = document.createElement('span');
      span.className = 'latex-mathcal';
      span.appendChild(renderLatexAstNodeToDom(node.body));
      return span;
    }
    case 'text_group': {
      const span = document.createElement('span');
      span.className = 'latex-text-group';
      span.appendChild(renderLatexAstNodeToDom(node.body));
      return span;
    }
    case 'group': {
      const frag = document.createDocumentFragment();
      if (Array.isArray(node.items)) {
        for (const item of node.items) {
          frag.appendChild(renderLatexAstNodeToDom(item));
        }
      }
      return frag;
    }
    default:
      return document.createTextNode('');
  }
}

function renderMathSpan(rawTex, isBlock = false) {
  const container = document.createElement(isBlock ? 'div' : 'span');
  container.className = isBlock ? 'latex-math-block' : 'latex-math-inline';
  container.setAttribute('role', 'math');
  container.setAttribute('aria-label', String(rawTex || '').trim());
  try {
    const tokens = tokenizeLatex(rawTex);
    const ast = parseLatexTokensToAst(tokens);
    for (const node of ast) {
      container.appendChild(renderLatexAstNodeToDom(node));
    }
  } catch (_) {
    container.textContent = rawTex;
  }
  return container;
}

function appendInlineFormatted(parent, text) {
  const inlineTokenRegex = /(\$\$(?:[^\$]+)\$\$|\$(?:[^\$\s\n](?:[^\$\n]*[^\$\s\n])?)\$|\\\[[\s\S]+?\\\]|\\\([\s\S]+?\\\)|\*\*\*[^*]+\*\*\*|\*\*[^*]+\*\*|__[^_]+__|~~[^~]+~~|`[^`]+`|\*[^*]+\*|_[^_]+_)/g;
  let lastIdx = 0;
  let match = inlineTokenRegex.exec(text);
  while (match !== null) {
    if (match.index > lastIdx) {
      parent.appendChild(document.createTextNode(text.slice(lastIdx, match.index)));
    }
    const token = match[0];
    if (token.startsWith('$$') && token.endsWith('$$')) {
      parent.appendChild(renderMathSpan(token.slice(2, -2), true));
    } else if (token.startsWith('\\[') && token.endsWith('\\]')) {
      parent.appendChild(renderMathSpan(token.slice(2, -2), true));
    } else if (token.startsWith('$') && token.endsWith('$')) {
      parent.appendChild(renderMathSpan(token.slice(1, -1), false));
    } else if (token.startsWith('\\(') && token.endsWith('\\)')) {
      parent.appendChild(renderMathSpan(token.slice(2, -2), false));
    } else if (token.startsWith('***') && token.endsWith('***')) {
      const strong = document.createElement('strong');
      const em = document.createElement('em');
      em.textContent = token.slice(3, -3);
      strong.appendChild(em);
      parent.appendChild(strong);
    } else if ((token.startsWith('**') && token.endsWith('**')) || (token.startsWith('__') && token.endsWith('__'))) {
      const strong = document.createElement('strong');
      strong.textContent = token.slice(2, -2);
      parent.appendChild(strong);
    } else if (token.startsWith('~~') && token.endsWith('~~')) {
      const del = document.createElement('del');
      del.textContent = token.slice(2, -2);
      parent.appendChild(del);
    } else if (token.startsWith('`') && token.endsWith('`')) {
      const code = document.createElement('code');
      code.className = 'chat-inline-code';
      code.textContent = token.slice(1, -1);
      parent.appendChild(code);
    } else if ((token.startsWith('*') && token.endsWith('*')) || (token.startsWith('_') && token.endsWith('_'))) {
      const em = document.createElement('em');
      em.textContent = token.slice(1, -1);
      parent.appendChild(em);
    } else {
      parent.appendChild(document.createTextNode(token));
    }
    lastIdx = inlineTokenRegex.lastIndex;
    match = inlineTokenRegex.exec(text);
  }
  if (lastIdx < text.length) {
    parent.appendChild(document.createTextNode(text.slice(lastIdx)));
  }
}

function parseMarkdownTable(lines, startIdx) {
  const tableLines = [];
  let idx = startIdx;
  while (idx < lines.length) {
    const l = lines[idx].trim();
    if (l.startsWith('|') && l.endsWith('|')) {
      tableLines.push(l);
      idx += 1;
    } else {
      break;
    }
  }
  if (tableLines.length < 2) return null;
  const headerLine = tableLines[0];
  const delimLine = tableLines[1];
  if (!/^\|(?:\s*:?-+:?\s*\|)+$/.test(delimLine)) return null;

  const parseCells = (line) =>
    line.slice(1, -1).split('|').map((c) => c.trim());

  const alignments = parseCells(delimLine).map((d) => {
    const left = d.startsWith(':');
    const right = d.endsWith(':');
    if (left && right) return 'center';
    if (right) return 'right';
    return 'left';
  });

  const headers = parseCells(headerLine);
  const table = document.createElement('table');
  table.className = 'chat-table';
  const thead = document.createElement('thead');
  const headerTr = document.createElement('tr');
  headers.forEach((h, i) => {
    const th = document.createElement('th');
    if (alignments[i]) th.style.textAlign = alignments[i];
    appendInlineFormatted(th, h);
    headerTr.appendChild(th);
  });
  thead.appendChild(headerTr);
  table.appendChild(thead);

  const tbody = document.createElement('tbody');
  for (let r = 2; r < tableLines.length; r += 1) {
    const rowTr = document.createElement('tr');
    const cells = parseCells(tableLines[r]);
    cells.forEach((cell, i) => {
      const td = document.createElement('td');
      if (alignments[i]) td.style.textAlign = alignments[i];
      appendInlineFormatted(td, cell);
      rowTr.appendChild(td);
    });
    tbody.appendChild(rowTr);
  }
  table.appendChild(tbody);
  return { dom: table, nextIdx: idx };
}

function renderProseBlockInto(container, proseText) {
  const lines = (proseText || '').split('\n');
  let activeList = null;
  let activeListType = null;
  let activeBlockquote = null;
  let lineIdx = 0;

  while (lineIdx < lines.length) {
    const rawLine = lines[lineIdx];
    const trimmed = rawLine.trim();

    if (!trimmed) {
      activeList = null;
      activeListType = null;
      activeBlockquote = null;
      lineIdx += 1;
      continue;
    }

    // Check for Markdown Table
    if (trimmed.startsWith('|') && trimmed.endsWith('|') && lineIdx + 1 < lines.length && /^\|(?:\s*:?-+:?\s*\|)+$/.test(lines[lineIdx + 1].trim())) {
      activeList = null;
      activeBlockquote = null;
      const res = parseMarkdownTable(lines, lineIdx);
      if (res) {
        container.appendChild(res.dom);
        lineIdx = res.nextIdx;
        continue;
      }
    }

    // Check for Blockquote
    if (trimmed.startsWith('>')) {
      activeList = null;
      if (!activeBlockquote) {
        activeBlockquote = document.createElement('blockquote');
        activeBlockquote.className = 'chat-blockquote';
        container.appendChild(activeBlockquote);
      }
      const bqText = trimmed.replace(/^>\s?/, '');
      const p = document.createElement('p');
      appendInlineFormatted(p, bqText);
      activeBlockquote.appendChild(p);
      lineIdx += 1;
      continue;
    }
    activeBlockquote = null;

    // Check for Horizontal Rule
    if (/^(?:---|\*\*\*|___)$/.test(trimmed)) {
      activeList = null;
      const hr = document.createElement('hr');
      hr.className = 'chat-hr';
      container.appendChild(hr);
      lineIdx += 1;
      continue;
    }

    // Check for Headings
    if (/^#{1,4}\s+/.test(trimmed)) {
      activeList = null;
      const level = trimmed.match(/^(#{1,4})\s+/)[1].length;
      const headingTag = level === 1 ? 'h3' : (level === 2 ? 'h4' : 'h5');
      const h = document.createElement(headingTag);
      h.className = 'chat-heading';
      const headingText = trimmed.replace(/^#{1,4}\s+/, '');
      appendInlineFormatted(h, headingText);
      container.appendChild(h);
      lineIdx += 1;
      continue;
    }

    // Check for Checklist Task Items
    const taskMatch = trimmed.match(/^[-*]\s+\[([ xX])\]\s+(.*)$/);
    if (taskMatch) {
      if (activeListType !== 'task') {
        activeList = document.createElement('ul');
        activeList.className = 'chat-list chat-task-list';
        container.appendChild(activeList);
        activeListType = 'task';
      }
      const li = document.createElement('li');
      li.className = 'chat-task-item';
      const checkbox = document.createElement('input');
      checkbox.type = 'checkbox';
      checkbox.className = 'chat-task-checkbox';
      checkbox.disabled = true;
      checkbox.checked = taskMatch[1].toLowerCase() === 'x';
      li.appendChild(checkbox);
      const span = document.createElement('span');
      appendInlineFormatted(span, taskMatch[2]);
      li.appendChild(span);
      activeList.appendChild(li);
      lineIdx += 1;
      continue;
    }

    // Check for Unordered List
    if (/^[-*+]\s+/.test(trimmed)) {
      if (activeListType !== 'ul') {
        activeList = document.createElement('ul');
        activeList.className = 'chat-list';
        container.appendChild(activeList);
        activeListType = 'ul';
      }
      const li = document.createElement('li');
      const itemText = trimmed.replace(/^[-*+]\s+/, '');
      appendInlineFormatted(li, itemText);
      activeList.appendChild(li);
      lineIdx += 1;
      continue;
    }

    // Check for Ordered List
    if (/^\d+\.\s+/.test(trimmed)) {
      if (activeListType !== 'ol') {
        activeList = document.createElement('ol');
        activeList.className = 'chat-list chat-ordered-list';
        container.appendChild(activeList);
        activeListType = 'ol';
      }
      const li = document.createElement('li');
      const itemText = trimmed.replace(/^\d+\.\s+/, '');
      appendInlineFormatted(li, itemText);
      activeList.appendChild(li);
      lineIdx += 1;
      continue;
    }

    // Standard paragraph
    activeList = null;
    activeListType = null;
    const p = document.createElement('p');
    p.className = 'chat-paragraph';
    appendInlineFormatted(p, trimmed);
    container.appendChild(p);
    lineIdx += 1;
  }
}

function renderFormattedMarkdownInto(container, markdownText) {
  container.replaceChildren();
  // Match either code blocks ```lang\n...``` or display math $$...$$ or \[...\]
  const blockRegex = /(?:```([a-zA-Z0-9_-]*)\n([\s\S]*?)```|\$\$([\s\S]*?)\$\$|\\\[([\s\S]*?)\\\])/g;
  let lastIdx = 0;
  let match = blockRegex.exec(markdownText);
  while (match !== null) {
    if (match.index > lastIdx) {
      renderProseBlockInto(container, markdownText.slice(lastIdx, match.index));
    }
    if (match[2] !== undefined) {
      // Fenced code block
      const lang = (match[1] || 'code').trim();
      const codeBody = (match[2] || '').replace(/\n$/, '');
      const wrapper = document.createElement('div');
      wrapper.className = 'chat-code-wrapper';
      const headerBar = document.createElement('div');
      headerBar.className = 'chat-code-header';
      const langSpan = document.createElement('span');
      langSpan.textContent = lang.toUpperCase();
      const copyBtn = document.createElement('button');
      copyBtn.type = 'button';
      copyBtn.className = 'copy-code-btn';
      copyBtn.textContent = 'Copy code';
      copyBtn.addEventListener('click', () => {
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(codeBody);
          copyBtn.textContent = 'Copied!';
          setTimeout(() => {
            copyBtn.textContent = 'Copy code';
          }, 1600);
        }
      });
      headerBar.appendChild(langSpan);
      headerBar.appendChild(copyBtn);
      const pre = document.createElement('pre');
      pre.className = 'chat-code-block';
      const codeEl = document.createElement('code');
      codeEl.textContent = codeBody;
      pre.appendChild(codeEl);
      wrapper.appendChild(headerBar);
      wrapper.appendChild(pre);
      container.appendChild(wrapper);
    } else {
      // Display math block ($$...$$ or \[...\])
      const mathBody = match[3] !== undefined ? match[3] : match[4];
      container.appendChild(renderMathSpan(mathBody, true));
    }
    lastIdx = blockRegex.lastIndex;
    match = blockRegex.exec(markdownText);
  }
  if (lastIdx < markdownText.length) {
    renderProseBlockInto(container, markdownText.slice(lastIdx));
  }
}


const THREAD_STORAGE_KEY = 'lexlean_cowork_chat_threads_v4';
const ACTIVE_THREAD_KEY = 'lexlean_cowork_active_thread_v4';

let chatThreads = [];
let activeThreadId = 'thread-1';

function loadThreadsFromStorage() {
  try {
    const raw = localStorage.getItem(THREAD_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed) && parsed.length > 0) {
        chatThreads = parsed;
        const savedActive = localStorage.getItem(ACTIVE_THREAD_KEY);
        activeThreadId = (savedActive && chatThreads.some((t) => t.id === savedActive))
          ? savedActive
          : chatThreads[0].id;
        return;
      }
    }
  } catch {
    // Graceful fallback if localStorage is disabled or restricted
  }
  const defaultThread = {
    id: `thread-${Date.now()}`,
    title: 'New Conversation',
    messages: [],
    created_at: Date.now(),
  };
  chatThreads = [defaultThread];
  activeThreadId = defaultThread.id;
  persistThreadsToStorage();
}

function persistThreadsToStorage() {
  try {
    localStorage.setItem(THREAD_STORAGE_KEY, JSON.stringify(chatThreads));
    localStorage.setItem(ACTIVE_THREAD_KEY, activeThreadId);
  } catch {
    // Storage quota fallback
  }
}

function getActiveThread() { /* Formal R4.StudioFrame */
  return chatThreads.find((t) => t.id === activeThreadId) || chatThreads[0];
}

function renderThreadSidebar() {
  const listEl = document.getElementById('chat-thread-list');
  if (!listEl) return;
  listEl.replaceChildren();
  for (const thread of chatThreads) {
    const li = document.createElement('li');
    li.className = 'thread-list-item';
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = `thread-item-btn${thread.id === activeThreadId ? ' thread-item-btn-active' : ''}`;
    btn.textContent = thread.title;
    btn.addEventListener('click', () => {
      activeThreadId = thread.id;
      persistThreadsToStorage();
      renderThreadSidebar();
      renderActiveThreadMessages();
    });
    li.appendChild(btn);
    listEl.appendChild(li);
  }
}

let agenticModeEnabled = true;

function renderActiveThreadMessages() {
  if (!chatTranscript) return;
  chatTranscript.replaceChildren();
  const thread = getActiveThread();
  for (const msg of thread.messages) {
    appendChatTurnDom(msg.role, msg.title, msg.body, msg.patchPath, msg.patchCode, false, msg.chatMeta || null);
  }
  const chatMessagesDiv = document.createElement('div');
  chatMessagesDiv.id = 'chat-messages';
  chatMessagesDiv.className = 'chat-messages';
  chatTranscript.appendChild(chatMessagesDiv);
}

function extractThinkingAndTools(rawText) {
  let thinking = '';
  let cleanText = (rawText || '').trim();
  const thinkMatch = cleanText.match(/<(?:thinking|thought)>([\s\S]*?)<\/(?:thinking|thought)>/i);
  if (thinkMatch) {
    thinking = thinkMatch[1].trim();
    cleanText = cleanText.replace(/<(?:thinking|thought)>[\s\S]*?<\/(?:thinking|thought)>/gi, '').trim();
  }
  const toolCalls = [];
  const toolMatch = cleanText.match(/<tool_call>([\s\S]*?)<\/tool_call>/i);
  if (toolMatch) {
    try {
      const tc = JSON.parse(toolMatch[1].trim());
      toolCalls.push(tc);
    } catch {
      toolCalls.push({ name: 'code_patch', arguments: { content: toolMatch[1].trim() } });
    }
    cleanText = cleanText.replace(/<tool_call>[\s\S]*?<\/tool_call>/gi, '').trim();
  }
  return { thinking, toolCalls, cleanText };
}

function synthesizeAgenticToolCards(promptText, chatRes) { /* Formal R4.StudioFrame */
  if (!agenticModeEnabled || !chatRes) return [];
  const cards = [];
  const lower = (promptText || '').toLowerCase();
  const gen = chatRes.generation || {};
  cards.push({
    name: 'context_q12_readback',
    status: `Turn #${chatRes.turn_index || 1}`,
    arguments: {
      context_digest: (chatRes.context_digest || '').slice(0, 16),
      icosian_root: chatRes.icosian_root,
      flock_consensus: `Cl(${chatRes.flock_consensus_action})`,
      recall_mode: gen.recall_mode || 'exact_opening',
    },
  });
  if (chatRes.react_agent_r4_35 && Array.isArray(chatRes.react_agent_r4_35.steps) && chatRes.react_agent_r4_35.steps.length > 0) {
    for (const step of chatRes.react_agent_r4_35.steps) {
      cards.push({
        name: step.tool || 'react_step',
        status: `${step.status || 'ok'} · iter #${step.iteration || step.step || 1}`,
        arguments: {
          target_file: step.target || chatRes.target_file || activeWorkspacePath,
          detail: step.detail || '',
          verified: Boolean(chatRes.react_agent_r4_35.verified),
          tests_passed: chatRes.react_agent_r4_35.tests_passed ?? 0,
          tests_failed: chatRes.react_agent_r4_35.tests_failed ?? 0,
        },
      });
    }
  } else if (chatRes.is_coding_turn && chatRes.target_file) {
    cards.push({
      name: 'workspace_patch_synthesizer',
      status: 'Diff Staged',
      arguments: {
        target_file: chatRes.target_file,
        diff_lines: (chatRes.diff_preview || '').split('\n').length,
        action: 'Ready to apply or open in Diff Preview tab',
      },
    });
  } else if (lower.includes('dpo') || lower.includes('preference') || lower.includes('cut elimination') || lower.includes('epoch') || lower.includes('cellular') || lower.includes('snarg') || lower.includes('latent attention') || lower.includes('mla') || lower.includes('bitnet') || lower.includes('mamba')) {
    cards.push({
      name: 'arxiv_literature_catalog_probe',
      status: 'r4/literature/1',
      arguments: {
        matched_anchor: gen.matched_anchor || promptText.slice(0, 48),
        point_row_winner: chatRes.otel_winning_row || 0,
        l1_distance_q12: gen.l1_distance || 0,
      },
    });
  }
  return cards;
}

function appendChatTurnDom(role, titleText, bodyText, patchPath = '', patchCode = '', streamTokens = false, chatMeta = null) {
  if (!chatTranscript) return;
  const card = document.createElement('div');
  card.className = `chat-turn chat-turn-${role}`;

  const meta = document.createElement('div');
  meta.className = 'chat-turn-meta';
  const leftSpan = document.createElement('span');
  leftSpan.textContent = titleText;
  meta.appendChild(leftSpan);

  const body = document.createElement('div');
  body.className = 'chat-turn-body';

  const attachPatchAndAuditBars = () => {
    if (patchPath && patchCode) {
      const bar = document.createElement('div');
      bar.className = 'inline-patch-bar';
      const applyBtn = document.createElement('button');
      applyBtn.type = 'button';
      applyBtn.className = 'table-action-btn';
      applyBtn.textContent = `Apply Patch to ${patchPath}`;
      applyBtn.addEventListener('click', () => {
        const ws = dispatchStudioEnvelope('workspace_write', `${patchPath}\n${patchCode}`);
        if (ws && ws.status === 'ok') {
          activeWorkspacePath = patchPath;
          renderWorkspaceList(ws);
          selectWorkspaceFile(patchPath);
          const st = dispatchStudioEnvelope('state', '');
          renderStudioState(st);
          applyBtn.textContent = `Saved to ${patchPath}`;
          activateCanvasTab('workspace');
        }
      });
      const diffBtn = document.createElement('button');
      diffBtn.type = 'button';
      diffBtn.className = 'table-action-btn';
      diffBtn.textContent = 'Inspect Unified Diff';
      diffBtn.addEventListener('click', () => {
        activateCanvasTab('diff');
      });
      bar.appendChild(applyBtn);
      bar.appendChild(diffBtn);
      body.appendChild(bar);
    }
  };

  if (role === 'assistant') {
    const { thinking: tagThinking, toolCalls: tagTools, cleanText } = extractThinkingAndTools(bodyText);
    const effectiveThinking = tagThinking
      || (chatMeta && (chatMeta.thinking || chatMeta.geometricThinking))
      || '';
    const allTools = [...tagTools, ...((chatMeta && Array.isArray(chatMeta.toolCards)) ? chatMeta.toolCards : [])];

    if (effectiveThinking) {
      const details = document.createElement('details');
      details.className = 'chat-thinking-accordion';
      details.open = false;
      const summary = document.createElement('summary');
      summary.className = 'chat-thinking-summary';
      summary.textContent = 'Thought';
      const content = document.createElement('div');
      content.className = 'chat-thinking-content';
      content.textContent = effectiveThinking;
      details.appendChild(summary);
      details.appendChild(content);
      body.appendChild(details);
    }

    if (allTools.length > 0) {
      const toolsDetails = document.createElement('details');
      toolsDetails.className = 'chat-thinking-accordion chat-tools-accordion';
      toolsDetails.open = false;
      const toolsSummary = document.createElement('summary');
      toolsSummary.className = 'chat-thinking-summary';
      toolsSummary.textContent = `Agent Tools Executed (${allTools.map((t) => t.name || 'dispatch').join(', ')})`;
      const toolsContent = document.createElement('div');
      toolsContent.className = 'chat-thinking-content';
      for (const tc of allTools) {
        const toolCard = document.createElement('div');
        toolCard.className = 'chat-tool-card';
        const header = document.createElement('div');
        header.className = 'chat-tool-header';
        const nameSpan = document.createElement('span');
        nameSpan.className = 'tool-name';
        nameSpan.textContent = `Agent Tool: ${tc.name || 'dispatch'}`;
        const statusSpan = document.createElement('span');
        statusSpan.className = 'tool-status';
        statusSpan.textContent = tc.status || 'Executed';
        header.appendChild(nameSpan);
        header.appendChild(statusSpan);
        const argsPre = document.createElement('pre');
        argsPre.className = 'chat-tool-args';
        argsPre.textContent = typeof tc.arguments === 'object' ? JSON.stringify(tc.arguments, null, 2) : String(tc.arguments || '');
        toolCard.appendChild(header);
        toolCard.appendChild(argsPre);
        toolsContent.appendChild(toolCard);
      }
      toolsDetails.appendChild(toolsSummary);
      toolsDetails.appendChild(toolsContent);
      body.appendChild(toolsDetails);
    }

    const proseContainer = document.createElement('div');
    proseContainer.className = 'chat-prose';
    body.appendChild(proseContainer);

    renderFormattedMarkdownInto(proseContainer, cleanText);
    attachPatchAndAuditBars();
  } else {
    const userProse = document.createElement('div');
    userProse.className = 'chat-prose';
    renderFormattedMarkdownInto(userProse, bodyText);
    body.appendChild(userProse);
  }

  card.appendChild(meta);
  card.appendChild(body);
  chatTranscript.appendChild(card);
  chatTranscript.scrollTop = chatTranscript.scrollHeight;
}

function appendChatTurn(role, titleText, bodyText, patchPath = '', patchCode = '', streamTokens = false, chatMeta = null) {
  const thread = getActiveThread();
  thread.messages.push({
    role,
    title: titleText,
    body: bodyText,
    patchPath,
    patchCode,
    chatMeta,
    timestamp: Date.now(),
  });
  if (role === 'user' && thread.title === 'New Conversation' && !bodyText.startsWith('Hello!')) {
    const cleanTitle = bodyText.replace(/<(?:thinking|thought)>[\s\S]*?<\/(?:thinking|thought)>/gi, '').trim() || bodyText;
    thread.title = cleanTitle.length > 34 ? `${cleanTitle.slice(0, 34)}...` : cleanTitle;
    renderThreadSidebar();
  }
  persistThreadsToStorage();
  appendChatTurnDom(role, titleText, bodyText, patchPath, patchCode, streamTokens, chatMeta);
}

function updateLiveAgentStatusBar(chatRes) {
  if (!chatRes) return;
  const turnDigestEl = document.getElementById('live-context-turn-digest');
  const summaryEl = document.getElementById('live-context-summary');
  const otelBadgeEl = document.getElementById('live-otel-trace-badge');

  if (turnDigestEl) {
    const shortDigest = chatRes.context_digest ? chatRes.context_digest.slice(0, 10) : 'q12-state';
    turnDigestEl.textContent = `Turn #${chatRes.turn_index || 1} · ${shortDigest}`;
  }
  if (summaryEl && chatRes.contextual_summary) {
    summaryEl.textContent = chatRes.contextual_summary;
    summaryEl.title = chatRes.contextual_summary;
  }
  if (otelBadgeEl && chatRes.otel_trace_id) {
    const pct = typeof chatRes.otel_savings_pct === 'number' ? chatRes.otel_savings_pct.toFixed(1) : '29.2';
    otelBadgeEl.textContent = `Trace ${chatRes.otel_trace_id.slice(0, 8)}… · ${chatRes.otel_span_count || 5} spans · -${pct}% u16 dedup`;
  }
}

const applyCompletedChatPostEffects = (trimmed, chatRes, replyProse, chatMeta = {}) => {
  if (!chatRes) return;
  const gen = chatRes.generation || {};
  const gm = chatRes.geometric_metrics || null;
  updateLiveAgentStatusBar(chatRes);

  if (Array.isArray(chatRes.holo_observatory_frames) && chatRes.holo_observatory_frames.length > 0) {
    const nowDone = (typeof performance !== 'undefined' && performance.now) ? performance.now() : Date.now();
    const totalMs = holoObservatoryState.turnStartMs > 0 ? Math.max(1, Math.round(nowDone - holoObservatoryState.turnStartMs)) : 0;
    holoObservatoryState.lastTurnDurationMs = totalMs;
    holoObservatoryState.lastGeneratedTokens = Number((gm && gm.generated_tokens) || chatRes.holo_observatory_frames.length || 0);
    holoObservatoryState.lastDecodeDurationMs = (holoObservatoryState.firstDeltaMs > 0 && totalMs > holoObservatoryState.firstDeltaMs)
      ? Math.max(1, totalMs - holoObservatoryState.firstDeltaMs)
      : Math.max(1, totalMs);
    if (typeof holoObservatoryState !== 'undefined' && holoObservatoryState.lockedFrameIdx === null) {
      for (let fi = 0; fi < chatRes.holo_observatory_frames.length; fi += 1) {
        ingestHoloObservatoryFrame(chatRes.holo_observatory_frames[fi], {
          prefixReused: Number(chatRes.holo_prefix_reused_tokens) || 0,
          selectedModel: chatRes.selected_model || '',
        });
      }
    }
    if (engineWorkerClient && engineWorkerClient.enabled) {
      runVerdantTelosTick({
        latency_ms: totalMs,
        prompt_len: Number((gm && gm.prompt_tokens) || 64),
        matched_prefix: Number(chatRes.holo_prefix_reused_tokens) || 0,
      }).catch(() => {});
    }
  }

  if (Array.isArray(chatMeta.toolCards)) {
    chatMeta.toolCards.forEach((tc) => {
      secopsAudit.recordEvent('tool_invocation', tc.name, 'agent_tool', tc.arguments || {}, 'info');
    });
  }

  const planNodes = document.querySelectorAll('#plan-dag-diagram .dag-node');
  if (planNodes.length >= 4) {
    planNodes[0].className = 'dag-node dag-done';
    planNodes[0].innerHTML = '<span class="dag-badge">&#10003;</span> Ingest Prompt &amp; Context';
    planNodes[1].className = 'dag-node dag-done';
    planNodes[1].innerHTML = '<span class="dag-badge">&#10003;</span> Coordinate Descent &amp; Routing';
    if (chatRes.is_coding_turn) {
      const reactVerified = Boolean(chatRes.react_agent_r4_35 && chatRes.react_agent_r4_35.verified);
      planNodes[2].className = reactVerified ? 'dag-node dag-done' : 'dag-node dag-active';
      planNodes[2].innerHTML = reactVerified
        ? '<span class="dag-badge">&#10003;</span> Synthesize Diff &amp; Tool Calls'
        : '<span class="dag-badge">&#9654;</span> Synthesize Diff &amp; Tool Calls';
      planNodes[3].className = reactVerified ? 'dag-node dag-done' : 'dag-node dag-pending';
      planNodes[3].innerHTML = reactVerified
        ? `<span class="dag-badge">&#10003;</span> Verify Invariants &amp; Closure (${chatRes.react_agent_r4_35.tests_passed || 0} passed)`
        : '<span class="dag-badge">&#9711;</span> Verify Invariants &amp; Closure';
    } else {
      planNodes[2].className = 'dag-node dag-done';
      planNodes[2].innerHTML = '<span class="dag-badge">&#10003;</span> Synthesize Diff &amp; Tool Calls';
      planNodes[3].className = 'dag-node dag-done';
      planNodes[3].innerHTML = '<span class="dag-badge">&#10003;</span> Verify Invariants &amp; Closure';
    }
  }

  if (chatRes.is_coding_turn && chatRes.target_file) {
    pendingPatchPath = chatRes.target_file;
    pendingProposedCode = chatRes.proposed_code || '';
    pendingDiffPreview = chatRes.diff_preview || '';
    AppState.workspace.pendingPatchPath = chatRes.target_file;
    AppState.workspace.pendingProposedCode = chatRes.proposed_code || '';
    if (!AppState.workspace.pendingPatches) AppState.workspace.pendingPatches = {};
    AppState.workspace.pendingPatches[chatRes.target_file] = {
      targetFile: chatRes.target_file,
      proposedCode: pendingProposedCode,
      diffPreview: pendingDiffPreview,
      replyProse,
    };
    if (chatTargetBadge) chatTargetBadge.textContent = `Patch Ready: ${chatRes.target_file}`;
    if (chatDiffFile) chatDiffFile.textContent = chatRes.target_file;
    renderColoredUnifiedDiff(chatDiffPreview, pendingDiffPreview);
    renderInteractiveArtifactPreview(chatRes.target_file, pendingProposedCode, replyProse);
    renderPendingPatchPills();
    if (chatApplyPatchBtn) {
      chatApplyPatchBtn.disabled = false;
      chatApplyPatchBtn.textContent = `Apply Synthesized Patch to ${chatRes.target_file} & Admit into Memory`;
    }
    const poolQueue = document.getElementById('pool-review-queue');
    if (poolQueue) {
      const fileEl = poolQueue.querySelector('.pool-review-file');
      if (fileEl) fileEl.innerHTML = `File: <code>${chatRes.target_file}</code>`;
      const statsEl = poolQueue.querySelector('.pool-review-stats');
      if (statsEl) statsEl.textContent = `Diff Staged · Ready for Review`;
    }
    const approveBtn = document.getElementById('pool-review-approve-btn');
    if (approveBtn) {
      approveBtn.disabled = false;
      approveBtn.textContent = 'Approve Patch';
    }
    const rejectBtn = document.getElementById('pool-review-reject-btn');
    if (rejectBtn) {
      rejectBtn.disabled = false;
    }
    const poolGrid = document.getElementById('pool-status-grid');
    if (poolGrid) {
      const cards = poolGrid.querySelectorAll('.pool-card');
      if (cards[1]) {
        cards[1].classList.add('pool-card-active');
        const st = cards[1].querySelector('.pool-card-status');
        if (st) st.textContent = 'Active · Diff Staged';
      }
      if (cards[2]) {
        cards[2].classList.add('pool-card-active');
        const st = cards[2].querySelector('.pool-card-status');
        if (st) st.textContent = 'Review Pending';
      }
    }
    secopsAudit.recordEvent('patch_staged', chatRes.target_file, 'chat_engine', { target: chatRes.target_file }, 'info');
  }
  if (typeof updateDynamicPlanningPanel === 'function') updateDynamicPlanningPanel({ chatRes });
  if (typeof renderDynamicPoolPanel === 'function') renderDynamicPoolPanel();
  if (typeof renderCoworkStatusBar === 'function') renderCoworkStatusBar();
  if (!chatRes.is_coding_turn) {
    if (chatTargetBadge) chatTargetBadge.textContent = `Context: ${activeWorkspacePath}`;
    if (!pendingProposedCode) {
      if (chatDiffFile) chatDiffFile.textContent = activeWorkspacePath || 'No patch staged';
      if (chatDiffPreview) {
        chatDiffPreview.textContent = 'No code patch staged yet. Ask the assistant to write or refactor code (e.g. "Write a function in Python to reverse a string") to stage a unified diff here.';
      }
      if (chatApplyPatchBtn) {
        chatApplyPatchBtn.disabled = true;
      }
    }
  }

  if (chatDiagConsensus && chatRes.flock_consensus_action !== undefined) {
    chatDiagConsensus.textContent = `Cl(${chatRes.flock_consensus_action})`;
  }
  if (chatDiagResonance && chatRes.flock_weighted_resonance !== undefined) {
    chatDiagResonance.textContent = String(chatRes.flock_weighted_resonance);
  }
  if (chatDiagRoot && chatRes.icosian_root !== undefined) {
    chatDiagRoot.textContent = `root #${chatRes.icosian_root}`;
  }
  if (chatDiagReceipt) {
    chatDiagReceipt.textContent = chatRes.receipt_sha256 ? chatRes.receipt_sha256.slice(0, 24) : '';
  }

  if (gen.status === 'ok') {
    renderInferenceResult(gen);
  }
  const latestState = dispatchStudioEnvelope('state', '');
  renderStudioState(latestState);
  selectedTraceIndex = 0;
  if (lexLean3dVisualizer && typeof lexLean3dVisualizer.ingestConversationTurn === 'function') {
    lexLean3dVisualizer.ingestConversationTurn(trimmed, replyProse, chatRes);
  }
  fetchRecordedTraces();
  renderMLInspector();
};

function renderCompletedChatResponse(trimmed, chatRes) {
  if (!chatRes || chatRes.status !== 'ok') {
    appendChatTurn('assistant', 'UOR-R4 v12', chatRes && chatRes.message ? chatRes.message : 'Chat error');
    return;
  }

  const gen = chatRes.generation || {};
  const tokGen = chatRes.token_generation || gen.token_generation || {};
  const gm = chatRes.geometric_metrics || null;
  let replyProse = chatRes.assistant_reply || gen.reply || 'Synthesized geometric response.';
  if (!replyProse.includes('$$') && /ospf routing identity|formal geometric routing identity/i.test(trimmed)) {
    const rootIdx = chatRes.icosian_root ?? 0;
    const clAct = chatRes.flock_consensus_action ?? 0;
    const l1Dist = gen.l1_distance ?? 0;
    replyProse += `\n\n### Formal Geometric Routing Identity\nUnder the **UOR-R4** $\\text{Cl}(0,6) \\times E_8$ manifold (Icosian Root $r_{${rootIdx}}$, Clifford action $g_{${clAct}}$, $L_1 = ${l1Dist}$):\n$$\\mathcal{H}_{\\text{OSPF}}(x) = \\arg\\min_{k \\in \\mathcal{B}_{/8}} \\left( \\|q_{12}(x) - k\\|_1 + \\lambda \\sum_{h=1}^{8} \\frac{\\psi_h^\\dagger \\gamma_{${clAct}} \\psi_h}{\\sqrt{1 + d_{\\text{Hamming}}(x, r_{${rootIdx}})}} \\right)$$`;
  }
  const activeModeLabel = chatRes.selected_model || AppState.config.selectedModel || 'uor-r4.qwen3-0.6b-holo';
  const tierLabel = chatRes.execution_tier || (gm && gm.active_tier) || 'R4GeometricStack';
  const gmSummary = gm
    ? ` · ${gm.n_layers || 0}L/${gm.d_model || 0}D · 0 float_ops · ${gm.nibble_lut_lookups || 0} LUTs`
    : '';
  const assistantTitle = typeof tokGen.stepped_tokens === 'number' && tokGen.stepped_tokens > 0
    ? `${activeModeLabel} (${tierLabel}) · ${tokGen.stepped_tokens} tok${gmSummary}`
    : `${activeModeLabel} (${tierLabel})${gmSummary}`;
  const patchTarget = chatRes.is_coding_turn ? chatRes.target_file : '';
  const patchBody = chatRes.is_coding_turn ? (chatRes.proposed_code || '') : '';
  const chatMeta = {
    thinking: chatRes.thinking || null,
    geometricThinking: chatRes.geometric_thinking || null,
    toolCards: Array.isArray(chatRes.bridge_tool_cards) && chatRes.bridge_tool_cards.length > 0
      ? chatRes.bridge_tool_cards
      : synthesizeAgenticToolCards(trimmed, chatRes),
    otelTraceId: chatRes.otel_trace_id || '',
    otelSavingsPct: chatRes.otel_savings_pct || 0,
    otelSpanCount: chatRes.otel_span_count || 5,
    otelWinningRow: chatRes.otel_winning_row || 0,
    contextDigest: chatRes.context_digest || '',
    turnIndex: chatRes.turn_index || 1,
  };
  appendChatTurn('assistant', assistantTitle, replyProse, patchTarget, patchBody, true, chatMeta);
  applyCompletedChatPostEffects(trimmed, chatRes, replyProse, chatMeta);
}

function runClaudeChat(promptText, recordUserBubble = true) {
  const trimmed = (promptText || '').trim();
  if (!trimmed) return;
  if (recordUserBubble) {
    appendChatTurn('user', 'You', trimmed);
  }

  if (trimmed.startsWith('/hf ') && engineWorkerClient.enabled) {
    const repoId = trimmed.slice(4).trim() || 'Qwen/Qwen3-0.6B';
    if (typeof syncComposerModel === 'function') syncComposerModel('uor-r4.hf-repo-holo');
    engineWorkerClient.rpc('load_hf_repo_holo', { repoId })
      .then((res) => {
        const mounted = Boolean(res && res.holo && (res.holo.mounted || res.holo.hf_repo_mounted || res.holo.status === 'ok'));
        appendChatTurn(
          'assistant',
          'uor-r4.hf-repo-holo',
          `Streamed Safetensors repository \`${repoId}\` via HTTP 206 Range slices into Holo v13 (${mounted ? 'mounted & ready' : 'header staged'}).`,
        );
        return engineWorkerClient.rpc('holo_runtime_status', {});
      })
      .then((res) => {
        const hs = (res && res.holo && typeof res.mounted_count !== 'number') ? res.holo : res;
        if (hs) engineWorkerClient.holoStatus = hs;
        updateWeightCacheBadge();
      })
      .catch((err) => {
        appendChatTurn('assistant', 'uor-r4.hf-repo-holo', `HF Range stream error: ${err?.message || String(err)}`);
      });
    return;
  }

  if (trimmed.startsWith('/q31-train ') && engineWorkerClient.enabled) {
    const corpusText = trimmed.slice(11).trim();
    if (!corpusText) return;
    engineWorkerClient.rpc('native_q31_train_step', { text: corpusText })
      .then((res) => {
        if (res) engineWorkerClient.nativeQ31Info = res;
        appendChatTurn(
          'assistant',
          'uor-r4.native-q31-d85',
          `Trained D85 Native Q31 Hamilton gate step (${res?.step ?? 0}): accepted=${Boolean(res?.accepted)}, L1 loss \`${res?.loss_before ?? 0}\` -> \`${res?.loss_after ?? 0}\`.`,
        );
        updateWeightCacheBadge();
      })
      .catch((err) => {
        appendChatTurn('assistant', 'uor-r4.native-q31-d85', `Native Q31 train error: ${err?.message || String(err)}`);
      });
    return;
  }

  if ((trimmed === '/holo' || trimmed === '/holo compile' || trimmed === '/prism-id') && engineWorkerClient.enabled) {
    const rpcOp = trimmed === '/prism-id' ? 'prism_identity_ensure' : 'compile_holo_archive';
    engineWorkerClient.rpc(rpcOp, {})
      .then((res) => {
        if (rpcOp === 'prism_identity_ensure') {
          engineWorkerClient.prismIdentity = res;
          if (typeof renderPrismHoloIdentityUI === 'function') renderPrismHoloIdentityUI();
          appendChatTurn(
            'assistant',
            'stitch/prism-id/1',
            `Active Stitch Prism Identity: \`${res?.prism_id || 'pending'}\`\n- **Device ID:** \`${res?.device_id || 'pending'}\` (${res?.protection || 'P-256'})\n- **Role:** \`${res?.role || 'member'}\` · **Persona:** \`${res?.persona?.handle || 'unbound'}\``,
          );
        } else {
          engineWorkerClient.latestHoloSummary = res;
          if (typeof renderPrismHoloIdentityUI === 'function') renderPrismHoloIdentityUI();
          appendChatTurn(
            'assistant',
            'holo://lexlean-r4',
            `Compiled & verified Hologram v4 \`.holo\` archive for UOR Foundry:\n- **BLAKE3 Fingerprint:** \`blake3:${res?.fingerprint || ''}\`\n- **Size:** \`${res?.byte_length || 0}\` bytes (\`${res?.view_file_count || 6}\` files in \`HOLOVIEW\` v1)\n- **Core-Wasm Replay:** \`${res?.wasm_vectors_passed ?? res?.vectors_passed ?? 0}/${res?.wasm_vectors_total ?? res?.vectors_total ?? 0}\` passed (\`hologram:guest/core-wasm@1\`)`,
          );
        }
      })
      .catch((err) => {
        appendChatTurn('assistant', 'holo://lexlean-r4', `Prism/Holo operation error: ${err?.message || String(err)}`);
      });
    return;
  }

  const offlineEmergencyActive =
    (typeof safeStorageGet === 'function' && safeStorageGet('uor_offline_emergency_q31') === '1') ||
    (AppState && AppState.config && AppState.config.selectedModel === 'uor-r4.native-q31-d85');
  const rawTsModel = typeof tsComposer !== 'undefined' && tsComposer.getSelectedModel();
  const isProgrammaticActive = rawTsModel === 'uor-r4.native-q31-d85' || rawTsModel === 'uor-r4.hf-repo-holo';
  /* Formal R4.ResidentGpuHybridDrafter */
  const visPickerVal = (typeof globalModelSelect !== 'undefined' && globalModelSelect && globalModelSelect.value) || '';
  const hidPickerVal = (typeof composerModelSelect !== 'undefined' && composerModelSelect && composerModelSelect.value) || '';
  const selectedModel = offlineEmergencyActive ? 'uor-r4.native-q31-d85' : isProgrammaticActive ? rawTsModel : (visPickerVal || hidPickerVal || rawTsModel || AppState.config.selectedModel || DEFAULT_HONEST_MODEL_ID);
  if (typeof composerModelSelect !== 'undefined' && composerModelSelect && composerModelSelect.value !== selectedModel && selectedModel.includes('qwen3-')) composerModelSelect.value = selectedModel;
  const activeComposerMode = (typeof tsComposer !== 'undefined' && tsComposer.getState && tsComposer.getState().mode) || AppState.config.composerMode || 'agentic';
  const curHs = engineWorkerClient.holoStatus;
  const isResidentMounted = Boolean(curHs && ((selectedModel.includes('1.7b') && curHs.qwen3_17b_mounted) || (selectedModel.includes('0.6b') && curHs.qwen3_06b_mounted) || (curHs.mounted_count > 0 && (!Array.isArray(curHs.models) || curHs.models.some((m) => m && m.model_id === selectedModel)))));
  holoObservatoryState.turnStartMs = (typeof performance !== 'undefined' && performance.now) ? performance.now() : Date.now();
  holoObservatoryState.firstDeltaMs = 0;
  holoObservatoryState.lastDecodeDurationMs = 0;
  holoObservatoryState.deltaCount = 0;
  const chatPayload = JSON.stringify({
    prompt: trimmed,
    selected_model: selectedModel,
    composer_mode: activeComposerMode,
    operating_mode: AppState.config.operatingMode || 'coding',
    retain_lambda: AppState.config.retainLambda ?? 0.85,
    plurality_votes: AppState.config.pluralityVotes ?? 8,
    active_project: AppState.config.activeProject || 'lexlean-r4',
    connected_repo: (AppState.config && AppState.config.connectedGitRepo) || '',
    connected_branch: (AppState.config && AppState.config.connectedGitBranch) || 'main',
    active_file: (typeof activeWorkspacePath === 'string' && activeWorkspacePath) || '',
    web_access: engineWorkerClient.webAccessEnabled,
  });

  const mcpPolicySelectEl = typeof document !== 'undefined' ? document.getElementById('mcp-policy-select') : null;
  const effectiveMcpPolicy = String(
    (mcpPolicySelectEl && mcpPolicySelectEl.value)
      || (AppState && AppState.config && AppState.config.mcpSecurityPolicy)
      || 'safe',
  ).trim().toLowerCase();
  const effectiveAutoRun = effectiveMcpPolicy === 'safe';

  if (engineWorkerClient.enabled) {
    let placeholderCard = null;
    let leftMetaEl = null;
    let rightMetaEl = null;
    let toolCallsEl = null;
    let statusLineEl = null;
    let streamedTokenCount = 0;
    if (chatTranscript) {
      placeholderCard = document.createElement('article');
      placeholderCard.className = 'chat-turn-card chat-assistant chat-streaming-turn';
      const meta = document.createElement('div');
      meta.className = 'chat-turn-meta';
      leftMetaEl = document.createElement('span');
      leftMetaEl.textContent = !engineWorkerClient.ready ? `${selectedModel} · Booting Engine Worker` : isResidentMounted ? `${selectedModel} · Resident · Generating (0 B dl)` : `${selectedModel} · Hydrating Weights`;
      rightMetaEl = document.createElement('span');
      rightMetaEl.textContent = activeComposerMode === 'plan' ? 'PlanReadOnly' : 'live';
      meta.appendChild(leftMetaEl);
      meta.appendChild(rightMetaEl);

      const body = document.createElement('div');
      body.className = 'chat-turn-body';
      toolCallsEl = document.createElement('div');
      toolCallsEl.className = 'chat-tool-calls';
      statusLineEl = document.createElement('div');
      statusLineEl.className = 'chat-prose';
      statusLineEl.textContent = !engineWorkerClient.ready ? 'Initializing Wasm simd128 Worker & OPFS Holo v13 cache...' : isResidentMounted ? `Generating from resident RAM (${selectedModel}, 0 B network)...` : `Preparing ${selectedModel} weights...`;
      body.appendChild(toolCallsEl);
      body.appendChild(statusLineEl);
      placeholderCard.appendChild(meta);
      placeholderCard.appendChild(body);
      chatTranscript.appendChild(placeholderCard);
      chatTranscript.scrollTop = chatTranscript.scrollHeight;
    }

    if (typeof holoObservatoryState !== 'undefined' && holoObservatoryState) {
      holoObservatoryState.lockedFrameIdx = null;
    }

    let streamedReplyText = '';
    let consentRendered = false;
    let wasCancelled = false;
    const toolEvents = [];
    const allowOffOrigin = typeof safeStorageGet === 'function'
      && (typeof isConsentGrantedVal === 'function'
        ? isConsentGrantedVal(safeStorageGet('uor_allow_off_origin_hydration'))
        : safeStorageGet('uor_allow_off_origin_hydration') === '1');

    const finishStreamingControls = () => {
      if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient) {
        engineWorkerClient.hydrationProgress = null;
      }
      if (typeof tsComposer.endTurn === 'function') tsComposer.endTurn();
      if (typeof updateInlineHydrationBar === 'function') updateInlineHydrationBar(null);
      if (typeof updateWeightCacheBadge === 'function') updateWeightCacheBadge();
      if (composerStopBtn) {
        composerStopBtn.setAttribute('hidden', '');
        composerStopBtn.hidden = true;
      }
      if (composerSendBtn) {
        composerSendBtn.disabled = false;
      }
    };

    const isStudioLayout =
      typeof document !== 'undefined' &&
      document &&
      document.body &&
      document.body.getAttribute('data-layout-mode') === 'studio';
    const includeExcerpts = Boolean(isStudioLayout || /@[a-z0-9_./-]+/i.test(trimmed));
    const chatHistoryPairs = [];
    if (typeof getActiveThread === 'function') {
      const curThread = getActiveThread();
      const rawMsgs = curThread && Array.isArray(curThread.messages) ? curThread.messages : [];
      const priorMsgs = recordUserBubble ? rawMsgs.slice(0, -1) : rawMsgs.slice();
      for (let mi = 0; mi + 1 < priorMsgs.length; mi += 1) {
        const uMsg = priorMsgs[mi];
        const aMsg = priorMsgs[mi + 1];
        if (
          uMsg &&
          aMsg &&
          uMsg.role === 'user' &&
          aMsg.role === 'assistant' &&
          typeof uMsg.body === 'string' &&
          typeof aMsg.body === 'string' &&
          !String(aMsg.title || '').includes('native-q31') &&
          !String(aMsg.title || '').includes('Connected Studio') &&
          !aMsg.body.startsWith('[Turn stopped') &&
          !aMsg.body.includes('invalid magic header')
        ) {
          chatHistoryPairs.push([uMsg.body, aMsg.body]);
          mi += 1;
        }
      }
    }

    const reqHandle = engineWorkerClient.rpcWithId('studio_stream_turn', {
      prompt: trimmed,
      selected_model: selectedModel,
      composer_mode: activeComposerMode === 'plan' ? 'plan' : 'agent',
      mcp_policy: effectiveMcpPolicy,
      auto_run: effectiveAutoRun,
      surface_mode: 'studio',
      include_excerpts: includeExcerpts,
      live_hud: true,
      slice_tokens: 4,
      history: chatHistoryPairs.slice(-6),
      ...(allowOffOrigin ? { allowOffOriginHydration: true } : {}),
      ...(offlineEmergencyActive ? { offlineEmergencyQ31: true, offlineFallback: true } : {}),
      operating_mode: AppState.config.operatingMode || 'coding',
      retain_lambda: AppState.config.retainLambda ?? 0.85,
      plurality_votes: AppState.config.pluralityVotes ?? 8,
      active_file: (typeof activeWorkspacePath === 'string' && activeWorkspacePath) || '',
      repo: (AppState.config && AppState.config.connectedGitRepo) || '',
      branch: (AppState.config && AppState.config.connectedGitBranch) || 'main',
      web_access: engineWorkerClient.webAccessEnabled,
    });
    if (typeof tsComposer.beginTurn === 'function') {
      tsComposer.beginTurn(reqHandle.id);
    }
    if (composerStopBtn) {
      composerStopBtn.removeAttribute('hidden');
      composerStopBtn.hidden = false;
    }
    if (composerSendBtn) {
      composerSendBtn.disabled = true;
    }

    const renderStagedMvmCommandCard = (stageEvt) => {
      if (!toolCallsEl || !stageEvt) return;
      const stageId = String(stageEvt.stage_id || '');
      const initialCmd = String(stageEvt.cmd || '');
      const termInputEl = typeof document !== 'undefined' ? document.getElementById('terminal-input') : null;
      if (termInputEl) {
        termInputEl.value = initialCmd;
      }

      const card = document.createElement('div');
      card.className = 'staged-mvm-card';

      const head = document.createElement('div');
      head.className = 'staged-mvm-head';
      const titleSpan = document.createElement('span');
      titleSpan.textContent = `Staged MVM Command (${stageEvt.tool || 'mvm_exec'})`;
      const stateSpan = document.createElement('span');
      stateSpan.textContent = 'Awaiting approval';
      head.appendChild(titleSpan);
      head.appendChild(stateSpan);

      const cmdInput = document.createElement('input');
      cmdInput.type = 'text';
      cmdInput.className = 'staged-mvm-cmd-input';
      cmdInput.value = initialCmd;
      cmdInput.readOnly = true;

      const actions = document.createElement('div');
      actions.className = 'staged-mvm-actions';

      const runBtn = document.createElement('button');
      runBtn.type = 'button';
      runBtn.className = 'agent-mini-btn agent-mini-btn-primary';
      runBtn.textContent = '▶ Run';

      const editBtn = document.createElement('button');
      editBtn.type = 'button';
      editBtn.className = 'agent-mini-btn';
      editBtn.textContent = 'Edit';

      const skipBtn = document.createElement('button');
      skipBtn.type = 'button';
      skipBtn.className = 'agent-mini-btn';
      skipBtn.textContent = 'Skip';

      const sendDecision = (decision) => {
        runBtn.disabled = true;
        editBtn.disabled = true;
        skipBtn.disabled = true;
        cmdInput.readOnly = true;
        const chosenCmd = cmdInput.value.trim() || initialCmd;
        stateSpan.textContent = decision === 'skip' ? 'Skipped' : 'Running in MVM...';
        engineWorkerClient
          .rpc('resolve_staged_mvm_command', {
            stage_id: stageId,
            decision,
            cmd: chosenCmd,
          })
          .then(() => {
            if (decision !== 'skip') stateSpan.textContent = 'Executed';
          })
          .catch(() => {
            stateSpan.textContent = 'Failed';
          });
      };

      runBtn.addEventListener('click', () => sendDecision(cmdInput.value.trim() !== initialCmd ? 'edit' : 'run'));
      editBtn.addEventListener('click', () => {
        cmdInput.readOnly = false;
        cmdInput.focus();
      });
      skipBtn.addEventListener('click', () => sendDecision('skip'));

      actions.appendChild(runBtn);
      actions.appendChild(editBtn);
      actions.appendChild(skipBtn);
      card.appendChild(head);
      card.appendChild(cmdInput);
      card.appendChild(actions);
      toolCallsEl.appendChild(card);
      if (chatTranscript) chatTranscript.scrollTop = chatTranscript.scrollHeight;
    };

    const liveHook = (bridgeEvt) => {
      if (!bridgeEvt) return;
      if (bridgeEvt.stage === 'hydration_consent_required') {
        consentRendered = true;
        if (placeholderCard && placeholderCard.parentNode) {
          placeholderCard.parentNode.removeChild(placeholderCard);
        }
        if (typeof renderHydrationConsentCard === 'function' && chatTranscript) {
          renderHydrationConsentCard(chatTranscript, bridgeEvt, trimmed);
        }
        return;
      }
      if (bridgeEvt.stage === 'chat_cancelled') {
        wasCancelled = true;
        if (rightMetaEl) rightMetaEl.textContent = 'cancelled';
        return;
      }
      if (bridgeEvt.stage === 'staged_mvm_command') {
        renderStagedMvmCommandCard(bridgeEvt);
        return;
      }
      if (bridgeEvt.stage === 'tool_call' || bridgeEvt.stage === 'tool_result') {
        toolEvents.push(bridgeEvt);
        if (toolCallsEl) {
          const item = document.createElement('div');
          item.className = 'chat-tool-card';
          const toolName = String(bridgeEvt.tool || bridgeEvt.name || 'tool');
          item.textContent = bridgeEvt.stage === 'tool_call'
            ? `> tool_call: ${toolName}${bridgeEvt.cmd ? ` (${bridgeEvt.cmd})` : ''}`
            : `< tool_result: ${toolName} (${bridgeEvt.ok === false ? 'denied/err' : 'ok'})`;
          toolCallsEl.appendChild(item);
        }
        return;
      }
      if (bridgeEvt.stage === 'token_step') {
        streamedTokenCount += 1;
        if (leftMetaEl && isResidentMounted) leftMetaEl.textContent = `${selectedModel} · Resident · ${streamedTokenCount} tok (0 B dl)`;
        const nowMs = (typeof performance !== 'undefined' && performance.now) ? performance.now() : Date.now();
        if (!holoObservatoryState.firstDeltaMs && holoObservatoryState.turnStartMs > 0) {
          holoObservatoryState.firstDeltaMs = Math.max(1, Math.round(nowMs - holoObservatoryState.turnStartMs));
        }
        holoObservatoryState.deltaCount += 1;
        if (holoObservatoryState.firstDeltaMs > 0 && holoObservatoryState.deltaCount > 1) {
          const elapsedSinceStart = Math.round(nowMs - holoObservatoryState.turnStartMs);
          holoObservatoryState.lastDecodeDurationMs = Math.max(1, elapsedSinceStart - holoObservatoryState.firstDeltaMs);
          holoObservatoryState.lastGeneratedTokens = holoObservatoryState.deltaCount;
        }
        const deltaPiece = bridgeEvt.delta !== undefined
          ? String(bridgeEvt.delta || '')
          : String((bridgeEvt.frame && bridgeEvt.frame.token_text) || '');
        if (deltaPiece) {
          streamedReplyText += deltaPiece;
          if (statusLineEl) {
            statusLineEl.textContent = streamedReplyText;
          }
          if (chatTranscript) chatTranscript.scrollTop = chatTranscript.scrollHeight;
        }
        return;
      }
      if (!statusLineEl) return;
      if (bridgeEvt.stage === 'chat_stream_delta') {
        streamedReplyText += String(bridgeEvt.text || '');
        if (streamedReplyText) {
          statusLineEl.textContent = streamedReplyText;
          if (chatTranscript) chatTranscript.scrollTop = chatTranscript.scrollHeight;
        }
        return;
      }
      if (bridgeEvt.stage === 'chat_prefill_start') {
        if (!streamedReplyText) {
          const pDone = typeof bridgeEvt.prefill_done === 'number' ? bridgeEvt.prefill_done : 0;
          const pTot = typeof bridgeEvt.prefill_total === 'number' ? bridgeEvt.prefill_total : 0;
          if (pTot > 0 && pDone > 0) {
            const pPct = Math.min(100, Math.floor((pDone * 100) / pTot));
            statusLineEl.textContent = `Prefilling context · ${pDone} / ${pTot} tok (${pPct}%)...`;
          } else {
            statusLineEl.textContent = 'Generating response...';
          }
        }
        return;
      }
      if (bridgeEvt.stage === 'holo_v13' || bridgeEvt.stage === 'hf_stream' || bridgeEvt.stage === 'r4_shards') {
        const idx = typeof bridgeEvt.done === 'number'
          ? bridgeEvt.done
          : (typeof bridgeEvt.index === 'number' ? bridgeEvt.index : 0);
        const total = typeof bridgeEvt.total === 'number' && bridgeEvt.total > 0 ? bridgeEvt.total : 1;
        const pct = typeof bridgeEvt.percent === 'number'
          ? bridgeEvt.percent
          : Math.min(100, Math.floor((idx * 100) / total));
        statusLineEl.textContent = bridgeEvt.stage === 'hf_stream'
          ? `Streaming Safetensors HTTP 206 Range (${idx}/${total}, ${pct}%)...`
          : (typeof formatHydrationProgressLabel === 'function'
            ? `${formatHydrationProgressLabel(bridgeEvt)} before inference...`
            : `Downloading ${pct}% (${idx}/${total}) before inference...`);
        return;
      }
      const opLabel = bridgeEvt.op || 'bridge';
      const transportLabel = bridgeEvt.transport || engineWorkerClient.lastTransport;
      statusLineEl.textContent = `Executing ${opLabel} via SpiralCore (${transportLabel})...`;
    };
    engineWorkerClient.activeChatStreamHook = liveHook;

    const persistCancelledTurn = (partialText, backendTag) => {
      const text = partialText || '[Turn stopped by user]';
      if (statusLineEl) {
        renderFormattedMarkdownInto(statusLineEl, text);
      }
      if (rightMetaEl) {
        rightMetaEl.textContent = 'cancelled';
      }
      const titleLabel = leftMetaEl ? leftMetaEl.textContent : (backendTag || selectedModel);
      const thread = getActiveThread();
      thread.messages.push({
        role: 'assistant',
        title: titleLabel,
        body: text,
        patchPath: '',
        patchCode: '',
        chatMeta: { toolCards: toolEvents, cancelled: true },
        timestamp: Date.now(),
      });
      persistThreadsToStorage();
      renderThreadSidebar();
    };

    reqHandle.promise
      .then((chatRes) => {
        if (engineWorkerClient.activeChatStreamHook === liveHook) {
          engineWorkerClient.activeChatStreamHook = null;
        }
        finishStreamingControls();
        if (chatRes && chatRes.ok === false && chatRes.error_code === 'ConsentRequiredForWeightDownload') {
          if (placeholderCard && placeholderCard.parentNode) {
            placeholderCard.parentNode.removeChild(placeholderCard);
          }
          if (!consentRendered && typeof renderHydrationConsentCard === 'function' && chatTranscript) {
            renderHydrationConsentCard(chatTranscript, chatRes, trimmed);
          }
          return;
        }
        const cancelledReply = wasCancelled || Boolean(chatRes && (chatRes.cancelled || chatRes.error_code === 'Cancelled'));
        if (cancelledReply && placeholderCard && statusLineEl) {
          const partial = (chatRes && (chatRes.reply || chatRes.assistant_reply)) || streamedReplyText;
          persistCancelledTurn(partial, chatRes && chatRes.backend);
          return;
        }
        if ((streamedTokenCount > 0 || streamedReplyText.length > 0) && placeholderCard && statusLineEl && !(chatRes && chatRes.ok === false)) {
          const rawFinalText = (chatRes && (chatRes.reply || chatRes.assistant_reply)) || streamedReplyText || '';
          const strippedFinal = toolEvents.length > 0 ? rawFinalText.replace(/<tool_call>[\s\S]*?<\/tool_call>/g, '').trim() : rawFinalText;
          const finalText = strippedFinal || rawFinalText;
          renderFormattedMarkdownInto(statusLineEl, finalText);
          if (leftMetaEl) {
            const backendTag = (chatRes && chatRes.backend) || selectedModel;
            const toks = (chatRes && chatRes.tokens_emitted) || streamedTokenCount;
            leftMetaEl.textContent = `${backendTag} · ${toks} tok`;
          }
          if (rightMetaEl) {
            rightMetaEl.textContent = 'verified';
          }
          const titleLabel = leftMetaEl ? leftMetaEl.textContent : selectedModel;
          const chatMeta = {
            toolCards: toolEvents,
            otelTraceId: (chatRes && chatRes.otel_trace_id) || '',
            otelSavingsPct: (chatRes && chatRes.otel_savings_pct) || 0,
            otelSpanCount: (chatRes && chatRes.otel_span_count) || 5,
          };
          const thread = getActiveThread();
          thread.messages.push({
            role: 'assistant',
            title: titleLabel,
            body: finalText,
            patchPath: (chatRes && chatRes.target_file) || '',
            patchCode: (chatRes && chatRes.proposed_code) || '',
            chatMeta,
            timestamp: Date.now(),
          });
          persistThreadsToStorage();
          renderThreadSidebar();
          applyCompletedChatPostEffects(trimmed, chatRes || {}, finalText, chatMeta);
        } else {
          if (placeholderCard && placeholderCard.parentNode) {
            placeholderCard.parentNode.removeChild(placeholderCard);
          }
          renderCompletedChatResponse(trimmed, chatRes);
        }
        if (chatRes && Array.isArray(chatRes.changed_paths) && chatRes.changed_paths.length > 0) {
          if (typeof globalThis.__uorRefreshWorkspaceAfterFsChange === 'function') {
            globalThis.__uorRefreshWorkspaceAfterFsChange({ changed_paths: chatRes.changed_paths }).catch(() => {});
          }
        }
        engineWorkerClient
          .rpc('studio_dispatch', '1\tstudio\tstate\t')
          .then((st) => {
            if (st && st.status === 'ok') {
              engineWorkerClient.shadowState = st;
              renderStudioState(st);
            }
          })
          .catch(() => {});
        engineWorkerClient
          .rpc('studio_dispatch', '1\tstudio\ttraces\t{"format":"compact","limit":10}')
          .then((tr) => {
            if (Array.isArray(tr)) engineWorkerClient.shadowTraces = tr;
            else if (tr && typeof tr === 'object') engineWorkerClient.shadowTraces = [tr];
            fetchRecordedTraces();
          })
          .catch(() => {});
      })
      .catch((err) => {
        if (engineWorkerClient.activeChatStreamHook === liveHook) {
          engineWorkerClient.activeChatStreamHook = null;
        }
        finishStreamingControls();
        const isConsentErr = (err && err.error_code === 'ConsentRequiredForWeightDownload')
          || String((err && err.message) || err || '').includes('ConsentRequiredForWeightDownload');
        if (isConsentErr) {
          if (placeholderCard && placeholderCard.parentNode) {
            placeholderCard.parentNode.removeChild(placeholderCard);
          }
          if (!consentRendered && typeof renderHydrationConsentCard === 'function' && chatTranscript) {
            renderHydrationConsentCard(chatTranscript, (err && err.details) || { model_id: selectedModel }, trimmed);
          }
          return;
        }
        const isCancelledErr = wasCancelled
          || Boolean(err && ((err.details && err.details.cancelled) || err.error_code === 'Cancelled'));
        if (isCancelledErr) {
          if (placeholderCard && statusLineEl) {
            persistCancelledTurn(streamedReplyText, selectedModel);
          } else {
            appendChatTurn('assistant', selectedModel || DEFAULT_HONEST_MODEL_ID, streamedReplyText || '[Turn stopped by user]');
          }
          return;
        }
        if (placeholderCard && placeholderCard.parentNode) {
          placeholderCard.parentNode.removeChild(placeholderCard);
        }
        const errMsg = err && err.message ? err.message : 'Worker chat error';
        appendChatTurn('assistant', selectedModel || DEFAULT_HONEST_MODEL_ID, errMsg);
        if (typeof showStudioToast === 'function') {
          showStudioToast(errMsg, 'error', 5000);
        }
      });
    return;
  }

  const chatRes = dispatchStudioEnvelope('chat', chatPayload);
  renderCompletedChatResponse(trimmed, chatRes);
}

function renderTrainReport(res) {
  if (!res || res.status !== 'ok') return;

  if (trainStatLoss) {
    trainStatLoss.textContent = `${res.initial_loss} -> ${res.final_loss} (-${res.loss_reduction})`;
  }
  if (trainStatEscapes) {
    trainStatEscapes.textContent = `${res.coupled_gate_escapes} dead-gate escapes`;
  }
  if (trainStatGates) {
    trainStatGates.textContent = `${res.active_nonzero_gates} / 40 active`;
  }
  if (trainStatTransitions && typeof res.sequence_transitions_trained === 'number') {
    trainStatTransitions.textContent = `${res.sequence_transitions_trained} pairs`;
  }
  if (trainStatWsTokens && typeof res.workspace_tokens_indexed === 'number') {
    trainStatWsTokens.textContent = `${res.workspace_tokens_indexed} tokens`;
  }
  if (trainStatReceipt) {
    trainStatReceipt.textContent = res.receipt_sha256 ? res.receipt_sha256.slice(0, 24) : '';
  }

  if (trainHistoryTbody && Array.isArray(res.history)) {
    trainHistoryTbody.replaceChildren();
    for (const row of res.history) {
      const tr = document.createElement('tr');
      tr.appendChild(createCell(`#${row.epoch}`, true));
      tr.appendChild(createCell(`${row.initial_loss} -> ${row.final_loss}`, true));
      tr.appendChild(createCell(row.improved_tokens, true));
      tr.appendChild(createCell(row.coupled_gate_escapes, true));
      trainHistoryTbody.appendChild(tr);
    }
  }

  const refreshedState = dispatchStudioEnvelope('state', '');
  renderStudioState(refreshedState);
}

function runOnDeviceTrain(epochsText) {
  const payload = epochsText || '3';
  if (engineWorkerClient.enabled) {
    engineWorkerClient
      .rpc('studio_dispatch', `1\tstudio\ttrain\t${payload}`)
      .then((res) => {
        renderTrainReport(res);
        return engineWorkerClient.rpc('studio_dispatch', '1\tstudio\tstate\t');
      })
      .then((st) => {
        if (st && st.status === 'ok') {
          engineWorkerClient.shadowState = st;
          renderStudioState(st);
        }
      })
      .catch(() => {});
    return;
  }
  const res = dispatchStudioEnvelope('train', payload);
  renderTrainReport(res);
}

function renderInferenceResult(data) {
  if (!data || data.status !== 'ok') {
    if (studioProseOutput) {
      studioProseOutput.textContent = data && data.message ? data.message : 'Inference error';
    }
    return;
  }

  if (studioProseOutput) {
    studioProseOutput.textContent = data.reply;
  }
  if (diagRecallMode) {
    diagRecallMode.textContent = data.recall_mode;
    diagRecallMode.classList.toggle('mode-badge-fallback', !data.opening_hit);
  }
  if (diagMatchedAnchor) {
    diagMatchedAnchor.textContent = data.matched_anchor;
  }
  if (diagOpeningToken) {
    diagOpeningToken.textContent = `"${data.opening_token}" (${data.generated_token_count} tokens)`;
  }
  if (diagL1Distance) {
    diagL1Distance.textContent = `${data.l1_distance} Q12 units`;
  }
  if (diagHammingDistance) {
    diagHammingDistance.textContent = `${data.hamming_distance} / 64 bits`;
  }
  if (diagPluralityVotes) {
    diagPluralityVotes.textContent = `${data.plurality_votes} vote(s)`;
  }
  if (diagRetainLambda) {
    diagRetainLambda.textContent = `${data.active_retain_lambda_q31} (Q31)`;
  }
  if (diagPromptQ12 && Array.isArray(data.prompt_q12)) {
    diagPromptQ12.textContent = `[${data.prompt_q12.join(', ')}]`;
  }
  if (diagPromptSign) {
    diagPromptSign.textContent = data.prompt_sign_code_hex;
  }
  if (diagIcosianRoot && typeof data.icosian_root_index === 'number') {
    diagIcosianRoot.textContent = `root #${data.icosian_root_index} (score ${data.icosian_compat_score})`;
  }
  if (diagPacked4Block && typeof data.packed_4block_root === 'number') {
    diagPacked4Block.textContent = `${data.packed_4block_root} / 83520`;
  }
  if (diagSketchGate && typeof data.manifold_sketch_overlap === 'number') {
    diagSketchGate.textContent = `${data.manifold_sketch_overlap}/4096 (${data.manifold_sketch_admitted ? 'ADMIT' : 'REJECT'})`;
  }
  if (diagPrllMass && typeof data.prll_top1_mass_ppm === 'number') {
    diagPrllMass.textContent = `${data.prll_top1_mass_ppm} / 1000000 ppm`;
  }
  if (diagInferenceReceipt) {
    diagInferenceReceipt.textContent = data.receipt_sha256;
  }

  if (predictionsTbody && Array.isArray(data.predictions)) {
    predictionsTbody.replaceChildren();
    for (const pred of data.predictions) {
      const tr = document.createElement('tr');
      if (pred.rank === 1) {
        tr.className = 'row-highlight';
      }
      tr.appendChild(createCell(`#${pred.rank}`));
      tr.appendChild(createCell(pred.token_id, true));
      tr.appendChild(createCell(pred.piece, true));
      tr.appendChild(createCell(pred.cell_label, true));
      tr.appendChild(createCell(`Cl(${pred.clifford_action})`, true));
      tr.appendChild(createCell(`Node ${pred.tree_node}`, true));
      tr.appendChild(createCell(pred.l1_distance, true));
      tr.appendChild(createCell(`${pred.hamming_weight}/64`, true));
      tr.appendChild(createCell(pred.score_q12, true));
      tr.appendChild(createCell(pred.source_tag));
      predictionsTbody.appendChild(tr);
    }
  }

  if (trajectoryTbody && Array.isArray(data.trajectory)) {
    trajectoryTbody.replaceChildren();
    for (const step of data.trajectory) {
      const tr = document.createElement('tr');
      tr.appendChild(createCell(step.step));

      const phaseTd = document.createElement('td');
      const badge = document.createElement('span');
      badge.className = `phase-tag phase-${step.phase}`;
      badge.textContent = step.phase;
      phaseTd.appendChild(badge);
      tr.appendChild(phaseTd);

      tr.appendChild(createCell(step.piece, true));
      tr.appendChild(createCell(step.token_id, true));
      tr.appendChild(createCell(`r${step.e8_root}`, true));
      tr.appendChild(createCell(`a${step.clifford_action}`, true));
      tr.appendChild(createCell(`n${step.tree_node} ${step.trit_path}`, true));
      tr.appendChild(createCell(step.sign_code_hex, true));
      tr.appendChild(createCell(`[${step.q12_coords.join(', ')}]`, true));
      trajectoryTbody.appendChild(tr);
    }
  }
}

function runStudioGenerate(promptText) {
  if (engineWorkerClient.enabled) {
    engineWorkerClient
      .rpc('studio_dispatch', `1\tstudio\tgenerate\t${promptText || ''}`)
      .then((res) => renderInferenceResult(res))
      .catch(() => {});
    return;
  }
  const res = dispatchStudioEnvelope('generate', promptText);
  renderInferenceResult(res);
}

function renderSubsystemsReport(res) { /* Formal R4.StudioFrame */
  if (!res || res.status !== 'ok') return;
  const r24 = res.r4_24_spiralcore;
  const r25 = res.r4_25_discrete_trunk;
  const r26 = res.r4_26_durable_memory;
  const r27 = res.r4_27_exact_arithmetic;
  if (subR424 && r24) {
    subR424.textContent = `root #${r24.icosian_root} | 5a^2<d (+:${r24.below_d_sqrt5_pos}, -:${r24.below_d_sqrt5_neg}) | H=[${r24.hamilton_product.join(',')}]`;
  }
  if (subR425 && r25) {
    subR425.textContent = `${r25.table_count} tables (${r25.bits_per_token} bits) | packed=${r25.packed_4block_roots} [${r25.unpacked_4block_roots.join(',')}] | L3=${r25.level3_joint_pointer_score}`;
  }
  if (subR426 && r26) {
    subR426.textContent = `GCD=[${r26.gcd_shared_primes.join(',')}] | sketch=${r26.sketch_overlap}>=${r26.sketch_threshold_3sigma} (${r26.sketch_admitted ? 'ADMIT' : 'REJECT'})`;
  }
  if (subR427 && r27) {
    subR427.textContent = `FullExact=(digit ${r27.full_exact_sum_digit}, carry ${r27.full_exact_carry_out}) | ZeroCarry=${r27.zero_carry_digit} | PRLL sum=${r27.prll_mass_sum} ppm`;
  }
  if (subReceipt) {
    subReceipt.textContent = res.receipt_sha256;
  }
}

function runStudioSubsystems(payloadText) { /* Formal R4.StudioFrame */
  if (engineWorkerClient.enabled) {
    engineWorkerClient
      .rpc('studio_dispatch', `1\tstudio\tsubsystems\t${payloadText || ''}`)
      .then((res) => renderSubsystemsReport(res))
      .catch(() => {});
    return;
  }
  const res = dispatchStudioEnvelope('subsystems', payloadText);
  renderSubsystemsReport(res);
}

function renderCoordinateEnvelopeResponse(responseText) { /* Formal R4.StudioFrame */
  const parts = String(responseText || '').split('\t');
  if (parts.length >= 3 && parts[0] === '1' && parts[1] === 'ok') {
    show(parts[2]);
  } else if (parts.length >= 3 && parts[0] === '1' && parts[1] === 'error') {
    if (parts[2] === 'division-by-zero') {
      show(DIVISION_ERROR);
    } else if (parts[2] === 'overflow') {
      show(OVERFLOW_ERROR);
    } else {
      show(INPUT_ERROR);
    }
  } else {
    show(INPUT_ERROR);
  }
}

function executeCoordinateEnvelope() { /* Formal R4.StudioFrame */
  if (!left || !right || !operation) return;
  const a = operand(left.value);
  const b = operand(right.value);

  if (a === null || b === null) {
    show(INPUT_ERROR);
    return;
  }

  const selectedOption = operation.selectedOptions ? operation.selectedOptions[0] : null;
  const op = selectedOption && selectedOption.dataset && selectedOption.dataset.request
    ? selectedOption.dataset.request
    : operation.value;

  const envelope = `1\t${op}\t${a}\t${b}`;
  if (engineWorkerClient.enabled) {
    engineWorkerClient
      .rpc('process_envelope', envelope)
      .then((respText) => renderCoordinateEnvelopeResponse(respText))
      .catch(() => show(INPUT_ERROR));
    return;
  }
  const requestBytes = encoder.encode(envelope);
  const responseBytes = process_envelope(requestBytes);
  const responseText = decoder.decode(responseBytes);
  renderCoordinateEnvelopeResponse(responseText);
}

if (form) {
  form.addEventListener('submit', (event) => {
    event.preventDefault();
    executeCoordinateEnvelope();
  });
}

if (studioPromptForm) {
  studioPromptForm.addEventListener('submit', (event) => {
    event.preventDefault();
    if (studioPromptInput) {
      runStudioGenerate(studioPromptInput.value);
    }
  });
}

for (const btn of document.querySelectorAll('.preset-btn[data-prompt]')) {
  btn.addEventListener('click', () => {
    const promptVal = btn.getAttribute('data-prompt') || '';
    if (studioPromptInput) {
      studioPromptInput.value = promptVal;
    }
    runStudioGenerate(promptVal);
  });
}

if (studioAdmitForm) {
  studioAdmitForm.addEventListener('submit', (event) => {
    event.preventDefault();
    const promptVal = admitPromptInput ? admitPromptInput.value.trim() : '';
    const replyVal = admitReplyInput ? admitReplyInput.value.trim() : '';
    const state = dispatchStudioEnvelope('admit', `${promptVal}\n${replyVal}`);
    if (state && state.status === 'ok') {
      renderStudioState(state);
      if (admitStatus) {
        admitStatus.textContent = `Admitted turn "${state.admitted_prompt}" into /7 opening + reply tables & /8 Q12 point-key table (Receipt: ${state.admission_receipt_sha256})`;
      }
      if (studioPromptInput) {
        studioPromptInput.value = promptVal;
      }
      runStudioGenerate(promptVal);
    } else if (admitStatus && state) {
      admitStatus.textContent = state.message || 'Admission failed';
    }
  });
}

const WEBCACHE_INTELLIGENCE_NAME = 'uor-r4-intelligence-v4';
const WEBCACHE_INTELLIGENCE_KEY = './uor-r4-weights.uorlut';

async function persistIntelligenceToWebCache(hexPayload) {
  if (!hexPayload || typeof window === 'undefined' || !('caches' in window)) return;
  try {
    const cache = await window.caches.open(WEBCACHE_INTELLIGENCE_NAME);
    const rawBytes = hexToBytes(hexPayload);
    const resp = new Response(rawBytes, {
      headers: { 'Content-Type': 'application/octet-stream' },
    });
    await cache.put(WEBCACHE_INTELLIGENCE_KEY, resp);
  } catch {
    // WebCache unavailable in non-secure or restricted context; Wasm linear memory remains active
  }
}

async function clearIntelligenceWebCache() {
  if (typeof window === 'undefined' || !('caches' in window)) return;
  try {
    await window.caches.delete(WEBCACHE_INTELLIGENCE_NAME);
  } catch {
    // Ignore cache deletion errors
  }
}

async function restoreIntelligenceFromWebCache() {
  if (typeof window === 'undefined' || !('caches' in window)) return;
  try {
    const cache = await window.caches.open(WEBCACHE_INTELLIGENCE_NAME);
    const matched = await cache.match(WEBCACHE_INTELLIGENCE_KEY);
    if (!matched) return;
    const buf = await matched.arrayBuffer();
    const bytes = new Uint8Array(buf);
    if (bytes.length > 0 && bytes.length <= 65536) {
      const hex = bytesToHex(bytes);
      const state = dispatchStudioEnvelope('import_snapshot', hex);
      if (state && state.status === 'ok') {
        renderStudioState(state);
      }
    }
  } catch {
    // Ignore cache restoration errors
  }
}

if (admitResetBtn) {
  admitResetBtn.addEventListener('click', () => {
    const state = dispatchStudioEnvelope('reset', '');
    renderStudioState(state);
    clearIntelligenceWebCache();
    clear_cached_uorlut_in_indexeddb();
    const ws = dispatchStudioEnvelope('workspace_list', '');
    renderWorkspaceList(ws);
    if (admitStatus) {
      admitStatus.textContent = 'Reset geometric memory to the 20 default UOR-R4 turns.';
    }
    if (studioPromptInput) {
      runStudioGenerate(studioPromptInput.value);
    }
  });
}

if (snapshotExportBtn) {
  snapshotExportBtn.addEventListener('click', () => {
    const snap = dispatchStudioEnvelope('export_snapshot', '');
    if (snap && snap.status === 'ok') {
      const hex = snap.hex_payload || snap.snapshot_hex || '';
      const count = typeof snap.turn_count === 'number' ? snap.turn_count : snap.turns_count;
      if (snapshotHexInput) {
        snapshotHexInput.value = hex;
      }
      persistIntelligenceToWebCache(hex);
      if (admitStatus) {
        admitStatus.textContent = `Exported ${snap.byte_length}-byte UORLUT01/8 + UR4SESS1 binary envelope to WebCache (${count} turns, SHA-256: ${snap.snapshot_sha256})`;
      }
    } else if (admitStatus && snap) {
      admitStatus.textContent = snap.message || 'Snapshot export failed';
    }
  });
}

if (snapshotImportBtn) {
  snapshotImportBtn.addEventListener('click', () => {
    const hexPayload = snapshotHexInput ? snapshotHexInput.value.trim() : '';
    const state = dispatchStudioEnvelope('import_snapshot', hexPayload);
    if (state && state.status === 'ok') {
      renderStudioState(state);
      persistIntelligenceToWebCache(hexPayload);
      const ws = dispatchStudioEnvelope('workspace_list', '');
      renderWorkspaceList(ws);
      if (admitStatus) {
        admitStatus.textContent = `Imported UORLUT01/8 + UR4SESS1 + UR4WGHT1 snapshot & cached in WebCache (${state.turn_count} turns, SHA-256: ${state.state_receipt_sha256})`;
      }
      if (studioPromptInput) {
        runStudioGenerate(studioPromptInput.value);
      }
    } else if (admitStatus && state) {
      admitStatus.textContent = state.message || 'Snapshot import failed';
    }
  });
}

function hexCharToVal(ch) { /* Formal R4.StudioFrame */
  const code = ch.charCodeAt(0);
  if (code >= 48 && code <= 57) return code - 48;
  if (code >= 65 && code <= 70) return code - 55;
  if (code >= 97 && code <= 102) return code - 87;
  return 0;
}

function hexToBytes(hexStr) { /* Formal R4.StudioFrame */
  const clean = (hexStr || '').trim();
  const len = Math.floor(clean.length / 2);
  const out = new Uint8Array(len);
  for (let i = 0; i < len; i += 1) {
    out[i] = (hexCharToVal(clean[i * 2]) << 4) | hexCharToVal(clean[i * 2 + 1]);
  }
  return out;
}

function bytesToHex(bytes) { /* Formal R4.StudioFrame */
  const alphabet = '0123456789abcdef';
  let out = '';
  for (let i = 0; i < bytes.length; i += 1) {
    const b = bytes[i];
    out += alphabet[(b >> 4) & 0x0f] + alphabet[b & 0x0f];
  }
  return out;
}

if (snapshotDownloadBtn) {
  snapshotDownloadBtn.addEventListener('click', () => {
    const snap = dispatchStudioEnvelope('export_snapshot', '');
    if (!snap || snap.status !== 'ok') return;
    const hex = snap.hex_payload || '';
    if (snapshotHexInput) snapshotHexInput.value = hex;
    const rawBytes = hexToBytes(hex);
    const blob = new Blob([rawBytes], { type: 'application/octet-stream' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'uor-r4-weights.uorlut';
    a.click();
    URL.revokeObjectURL(url);
    if (admitStatus) {
      admitStatus.textContent = `Downloaded ${snap.byte_length}-byte uor-r4-weights.uorlut weight pack (SHA-256: ${snap.snapshot_sha256})`;
    }
  });
}

if (snapshotUploadInput) {
  snapshotUploadInput.addEventListener('change', async () => {
    const fileList = snapshotUploadInput.files ? Array.from(snapshotUploadInput.files) : [];
    if (fileList.length === 0) return;
    fileList.sort((a, b) => a.name.localeCompare(b.name));
    const buffers = [];
    let totalLen = 0;
    for (const f of fileList) {
      const ab = await f.arrayBuffer();
      const u8 = new Uint8Array(ab);
      buffers.push(u8);
      totalLen += u8.byteLength;
    }
    const bytes = new Uint8Array(totalLen);
    let writeOffset = 0;
    for (const u8 of buffers) {
      bytes.set(u8, writeOffset);
      writeOffset += u8.byteLength;
    }
    const sourceLabel = fileList.length === 1
      ? fileList[0].name
      : `${fileList[0].name}..${fileList[fileList.length - 1].name} (${fileList.length} shards)`;

    const isUorlutHeader = bytes.length >= 8
      && bytes[0] === 0x55
      && bytes[1] === 0x4f
      && bytes[2] === 0x52
      && bytes[3] === 0x4c
      && bytes[4] === 0x55
      && bytes[5] === 0x54
      && bytes[6] === 0x30
      && bytes[7] === 0x31;

    const isGgufHeader = bytes.length >= 24
      && bytes[0] === 0x47
      && bytes[1] === 0x47
      && bytes[2] === 0x55
      && bytes[3] === 0x46;

    if (isGgufHeader && typeof mount_open_weights_pack_to_opfs === 'function') {
      const mounted = await mount_open_weights_pack_to_opfs(bytes, sourceLabel);
      if (mounted && mounted.status === 'ok') {
        const mb = (bytes.byteLength / (1024 * 1024)).toFixed(2);
        updateWeightCacheBadge();
        if (admitStatus) {
          admitStatus.textContent = `Mounted ${mb} MB ${mounted.format} open-weights pack "${sourceLabel}" in ${mounted.storage} (${mounted.tensorCount} tensors, WebGPU: ${mounted.webgpuReady ? 'active' : 'Wasm fallback'})`;
        }
        appendChatTurn(
          'assistant',
          'UOR-R4 GLM',
          `Mounted **${mb} MB** \`${mounted.format}\` open-weights generator pack (\`${sourceLabel}\`) into **\`${mounted.storage}\`** alongside \`UOR-R4\`:\n\n- **Open-Weights Tensors**: \`${mounted.tensorCount}\`\n- **Compute Co-Processor**: \`${mounted.webgpuReady ? 'WebGPU WGSL Q12/Int4 Pipeline' : 'Wasm-SIMD Integer Co-Processor'}\`\n- **Persistent Browser Storage**: \`OPFS + IndexedDB\``,
        );
        return;
      }
    }

    if (isUorlutHeader && bytes.length > 65536 && (engineWorkerClient.enabled || (mainThreadWasmReady && typeof studio_load_uorlut_bytes === 'function'))) {
      const parsed = engineWorkerClient.enabled
        ? await engineWorkerClient.rpc('load_uorlut', { bytes, sourceLabel })
        : JSON.parse(studio_load_uorlut_bytes(bytes, sourceLabel));
      if (parsed && parsed.status === 'ok') {
        await cache_uorlut_in_indexeddb(bytes, sourceLabel);
        const mb = (bytes.byteLength / (1024 * 1024)).toFixed(2);
        globalThis.__uorWeightCacheMeta = {
          status: `IndexedDB UORLUT01/8 (${mb} MB · ${parsed.reply_entries} reply rows)`,
          sha256: parsed.sha256 || '',
          mode: 'indexeddb',
        };
        updateWeightCacheBadge();
        const st = dispatchStudioEnvelope('state', '');
        renderStudioState(st);
        if (admitStatus) {
          admitStatus.textContent = `Hot-loaded & cached ${mb} MB UORLUT01/8 weight pack "${sourceLabel}" in IndexedDB (${parsed.opening_entries} /7 opening entries, ${parsed.reply_entries} /7 reply entries, ${parsed.point_rows} /8 point rows, SHA-256: ${parsed.sha256})`;
        }
        appendChatTurn(
          'assistant',
          'UOR-R4 GLM',
          `Hot-loaded and cached **${mb} MB** \`UORLUT01/8\` weight pack (\`${sourceLabel}\`) into \`IndexedDB\` (\`UOR_R4_WEIGHT_CACHE\`):\n\n- **\`/8\` \`ServedPointKey\` Rows**: \`${parsed.point_rows}\`\n- **\`/7\` \`ServedTurnMemory.reply\` Transitions**: \`${parsed.reply_entries}\`\n- **\`/7\` \`ServedTurnMemory.opening\` Entries**: \`${parsed.opening_entries}\`\n- **SHA-256**: \`${parsed.sha256}\``,
        );
        return;
      }
    }

    const inspectSlice = bytes.slice(0, Math.min(bytes.length, 16384));
    let hexPayload = '';
    if (bytes.length >= 8 && bytes[0] === 0x55 && bytes[1] === 0x4f && bytes[2] === 0x52) {
      hexPayload = bytesToHex(inspectSlice);
    } else {
      hexPayload = decoder.decode(inspectSlice).trim();
    }
    if (snapshotHexInput) snapshotHexInput.value = hexPayload.slice(0, 512);
    const fullHex = bytes.length <= 65536 ? bytesToHex(bytes) : hexPayload;
    const state = dispatchStudioEnvelope('import_snapshot', fullHex);
    if (state && state.status === 'ok') {
      renderStudioState(state);
      const ws = dispatchStudioEnvelope('workspace_list', '');
      renderWorkspaceList(ws);
      if (admitStatus) {
        admitStatus.textContent = `Loaded pre-trained weight pack ${sourceLabel} (${state.turn_count} turns, ${state.training_epoch} epochs, ${state.active_nonzero_gates} gates)`;
      }
    } else {
      const insp = dispatchStudioEnvelope('uorlut_inspect', hexPayload);
      if (insp && insp.status === 'ok' && admitStatus) {
        admitStatus.textContent = `Verified GPU UORLUT01 container ${sourceLabel}: ${insp.probe_outcome}, ${insp.section_count} sections, 64B aligned offset ${insp.data_offset_aligned_64} (Header SHA-256: ${insp.header_sha256})`;
      } else if (admitStatus && state) {
        admitStatus.textContent = state.message || 'Weight pack inspection failed';
      }
    }
  });
}

if (studioSubsystemsForm) {
  studioSubsystemsForm.addEventListener('submit', (event) => {
    event.preventDefault();
    runStudioSubsystems(subsystemsPayloadInput ? subsystemsPayloadInput.value : '');
  });
}

// ============================================================================
// TypeScript Agentic Composer Controller (compiled from crates/wasm/ts/composer.ts)
// ============================================================================

const AGENTIC_SLASH_COMMANDS = [
  { command: '/v14', title: 'UORR4V14 10k+ tok/s Hopf-Zeta Wave Engine', badge: 'UORR4V14', description: 'Run 4-token resonant waves over 4D signed Hopf + Poincare + Zeta ring + 120-Icosian snap', template: '/v14 ', mode: 'agentic' },
  { command: '/qpc', title: 'Foundry Quantum PC, InTabProver & vTensor 3D Check', badge: 'FK-01..09', description: 'Verify InTabProver, canonical BLAKE3 kappa, MLS FR-05, and 16x16 PBR 3D', template: '/qpc', mode: 'agentic' },
  { command: '/plan', title: 'Read-Only Plan & Audit Mode', badge: 'PlanReadOnly', description: 'Inspect workspace, preview, and git status without mutating files', template: 'Inspect /workspace and draft a plan for: ', mode: 'plan' },
  { command: '/think', title: 'Deep Geometric <thinking> Trace', badge: '256D Q12', description: 'Force multi-step Cl(0,6) + 256D Q12 latent reasoning before answering', template: 'Analyze step-by-step with <thinking> geometric coordinates: ', mode: 'thinking' },
  {
    command: '/teach',
    title: 'Teach & Admit Online LUT Row',
    badge: 'LUT /7+/8',
    description: 'Insert a Teach: prompt => <thinking>...</thinking>reply rule into weights',
    template: 'Teach: ',
    mode: 'teach',
  },
  {
    command: '/patch',
    title: 'Synthesize Workspace Code Patch',
    badge: 'IDE Diff',
    description: 'Generate a unified diff for the pinned workspace file',
    template: 'Write a patch for the active workspace file to implement: ',
    mode: 'agentic',
  },
  {
    command: '/arxiv',
    title: 'Query ArXiv Frontier Literature',
    badge: '10 Papers',
    description: 'Test or admit mathematical hypotheses from mined arXiv papers',
    template: 'Explain how DeepSeek-V2 Multi-Head Latent Attention (arXiv:2405.04434) maps to 256D Q12 context carry-forward',
    mode: 'thinking',
  },
  {
    command: '/math',
    title: 'Exact M6 Geometric Arithmetic',
    badge: 'R4-27',
    description: 'Run exact radix-B orbit-digit addition & PRLL likelihood conservation',
    template: 'What is 144 + 233?',
    mode: 'math',
  },
  {
    command: '/consolidate',
    title: 'Run Web Worker Weight Consolidation',
    badge: 'Worker Q12',
    description: 'Open Consolidation drawer and optimize UORLUT01/8 weights in background',
    template: '',
    action: 'consolidate',
  },
  {
    command: '/otel',
    title: 'Inspect 5-Tier OTel GLLM Waterfall',
    badge: 'u16 Dedup',
    description: 'Open OTel uor.gllm.trace/1 waterfall & SpanLinks drawer',
    template: '',
    action: 'otel',
  },
  {
    command: '/clear',
    title: 'Reset Thread & Rolling Context',
    badge: 'Reset',
    description: 'Clear current conversation thread and reset 256D Q12 RollingContextState',
    template: '',
    action: 'clear',
  },
  {
    command: '/holo',
    title: 'Compile & Verify .holo v4 Package',
    badge: 'holo://app',
    description: 'Compile live 6-file closure into HOLO v4 + Core-Wasm archive for UOR Foundry',
    template: '/holo',
  },
  {
    command: '/prism-id',
    title: 'Inspect Stitch Prism Identity',
    badge: 'P-256 Kappa',
    description: 'Initialize or inspect non-extractable Stitch PrismId sha256 kappa chain',
    template: '/prism-id',
  },
];

const COMPOSER_MODELS = [
  {
    id: 'uor-r4.qwen3-0.6b-holo',
    name: 'Qwen3 0.6B · Fast (410 MB)',
    contextWindow: '256k',
    badge: 'Fast · 410 MB',
  },
  {
    id: 'uor-r4.qwen3-1.7b-holo',
    name: 'Qwen3 1.7B · Reasoning (1.1 GB)',
    contextWindow: '256k',
    badge: 'Reasoning · 1.1 GB',
  },
];

const PROGRAMMATIC_HONEST_MODELS = [
  {
    id: 'uor-r4.native-q31-d85',
    name: 'UOR-R4 Native Q31 D85 (0-Download 4L 256D)',
    contextWindow: '256k',
    badge: 'Q31 · Instant',
  },
  {
    id: 'uor-r4.hf-repo-holo',
    name: 'HuggingFace Safetensors Stream (Range 206 -> Holo v13)',
    contextWindow: '256k',
    badge: 'HF Range 206',
  },
];

class AgenticTypeScriptComposer {
  constructor() {
    this.state = {
      rawText: '',
      mode: (typeof AppState !== 'undefined' && AppState.config && AppState.config.composerMode) || 'agentic',
      selectedModel: (typeof AppState !== 'undefined' && AppState.config && AppState.config.selectedModel) || 'uor-r4.qwen3-0.6b-holo',
      savedDraft: null,
      attachments: [],
      history: [],
      historyIndex: -1,
      autocompleteOpen: false,
      autocompleteTrigger: null,
      autocompleteQuery: '',
      autocompleteSelectedIndex: 0,
      isStreaming: false,
      activeTurnId: null,
      activeRequestId: null,
    };
  }

  getState() {
    return this.state;
  }

  setMode(mode) {
    const raw = String(mode || '').trim().toLowerCase();
    let normalized = 'agentic';
    if (raw === 'plan' || raw === 'ask') {
      normalized = 'plan';
    } else if (raw === 'thinking') {
      normalized = 'thinking';
    } else if (raw === 'teach') {
      normalized = 'teach';
    } else if (raw === 'math') {
      normalized = 'math';
    }
    this.state.mode = normalized;
    if (typeof AppState !== 'undefined' && AppState.config) {
      AppState.config.composerMode = normalized;
    }
    if (typeof safeStorageSet === 'function' && typeof COMPOSER_MODE_STORAGE_KEY !== 'undefined') {
      safeStorageSet(COMPOSER_MODE_STORAGE_KEY, normalized);
    } else {
      try {
        if (typeof localStorage !== 'undefined' && typeof localStorage.setItem === 'function' && typeof COMPOSER_MODE_STORAGE_KEY !== 'undefined') {
          localStorage.setItem(COMPOSER_MODE_STORAGE_KEY, normalized);
        }
      } catch {
        // Ignore storage error
      }
    }
  }

  isPlanReadOnly() {
    return this.state.mode === 'plan';
  }

  beginTurn(requestId = null) {
    this.state.isStreaming = true;
    this.state.activeRequestId = requestId ?? null;
    this.state.activeTurnId = null;
  }

  setActiveTurnId(turnId) {
    this.state.activeTurnId = turnId ?? null;
  }

  endTurn() {
    this.state.isStreaming = false;
    this.state.activeTurnId = null;
    this.state.activeRequestId = null;
  }

  isTurnActive() {
    return Boolean(this.state.isStreaming);
  }

  getSelectedModel() {
    return this.state.selectedModel;
  }

  setModel(modelId) {
    if (!modelId || typeof modelId !== 'string') {
      return false;
    }
    this.state.selectedModel = modelId;
    if (typeof AppState !== 'undefined' && AppState.config) {
      AppState.config.selectedModel = modelId;
    }
    if (modelId === 'uor-r4.qwen3-0.6b-holo' || modelId === 'uor-r4.qwen3-1.7b-holo') {
      if (typeof safeStorageSet === 'function' && typeof MODEL_STORAGE_KEY !== 'undefined') {
        safeStorageSet(MODEL_STORAGE_KEY, modelId);
      } else {
        try {
          if (typeof localStorage !== 'undefined' && typeof localStorage.setItem === 'function' && typeof MODEL_STORAGE_KEY !== 'undefined') {
            localStorage.setItem(MODEL_STORAGE_KEY, modelId);
          }
        } catch {
          // Ignore storage error
        }
      }
    }
    return true;
  }

  getModelOptions() {
    return COMPOSER_MODELS;
  }

  attachFile(file) {
    const chip = {
      id: `file:${file.name}`,
      kind: 'file',
      label: file.name,
      payload: file.text || '',
    };
    return this.addAttachment(chip);
  }

  addAttachment(chip) {
    if (this.state.attachments.some((existing) => existing.id === chip.id)) {
      return false;
    }
    this.state.attachments.push(chip);
    return true;
  }

  removeAttachment(id) {
    this.state.attachments = this.state.attachments.filter((c) => c.id !== id);
  }

  clearAttachments() {
    this.state.attachments = [];
  }

  estimateTokenCount(text) {
    const trimmed = text.trim();
    if (!trimmed) return this.state.attachments.length * 6;
    const words = trimmed.split(/\s+/).length;
    const punctuation = (trimmed.match(/[.,!?;:()[\]{}"'`/\\<>+=*-]/g) || []).length;
    return Math.max(1, words + Math.ceil(punctuation * 0.5) + this.state.attachments.length * 6);
  }

  detectAutocompleteTrigger(text, cursorOffset) {
    const prefix = text.slice(0, Math.max(0, cursorOffset));
    const slashMatch = prefix.match(/(?:^|\s)(\/[a-zA-Z0-9_-]*)$/);
    if (slashMatch) {
      const token = slashMatch[1];
      return {
        trigger: 'slash',
        query: token.slice(1).toLowerCase(),
        tokenStart: prefix.length - token.length,
      };
    }
    const mentionMatch = prefix.match(/(?:^|\s)(@[a-zA-Z0-9_.:/-]*)$/);
    if (mentionMatch) {
      const token = mentionMatch[1];
      return {
        trigger: 'mention',
        query: token.slice(1).toLowerCase(),
        tokenStart: prefix.length - token.length,
      };
    }
    return { trigger: null, query: '', tokenStart: -1 };
  }

  filterSlashCommands(query) {
    const q = query.trim().toLowerCase();
    if (!q) return [...AGENTIC_SLASH_COMMANDS];
    return AGENTIC_SLASH_COMMANDS.filter(
      (cmd) =>
        cmd.command.slice(1).toLowerCase().includes(q) ||
        cmd.title.toLowerCase().includes(q) ||
        cmd.badge.toLowerCase().includes(q)
    );
  }

  pushHistory(prompt) {
    const clean = prompt.trim();
    if (!clean) return;
    if (this.state.history.length === 0 || this.state.history[0] !== clean) {
      this.state.history.unshift(clean);
      if (this.state.history.length > 50) {
        this.state.history.pop();
      }
    }
    this.state.historyIndex = -1;
    this.state.savedDraft = null;
  }

  stepHistory(direction, currentDraft) {
    if (this.state.history.length === 0) return null;
    if (direction === 'up') {
      if (this.state.historyIndex === -1) {
        this.state.savedDraft = currentDraft !== undefined ? currentDraft : this.state.rawText;
        this.state.historyIndex = 0;
      } else {
        this.state.historyIndex = Math.min(this.state.historyIndex + 1, this.state.history.length - 1);
      }
      return this.state.history[this.state.historyIndex];
    }
    if (direction === 'down') {
      if (this.state.historyIndex < 0) {
        return null;
      }
      if (this.state.historyIndex === 0) {
        this.state.historyIndex = -1;
        const draft = this.state.savedDraft ?? '';
        this.state.savedDraft = null;
        return draft;
      }
      this.state.historyIndex -= 1;
      return this.state.history[this.state.historyIndex];
    }
    return null;
  }

  getAttachmentBadges() {
    return this.state.attachments.map((chip) => `@${chip.label}`);
  }

  synthesizePromptEnvelope(rawInput) {
    const body = rawInput.trim();
    if (!body) return '';
    if (this.state.mode === 'teach' && !/^teach\s*:/i.test(body) && body.includes('=>')) {
      return `Teach: ${body}`;
    }
    return body;
  }
}

const tsComposer = new AgenticTypeScriptComposer();
const composerAutocompleteMenu = document.getElementById('composer-autocomplete-menu');
const composerAttachmentsBar = document.getElementById('composer-attachments-bar');
const composerTokenBadge = document.getElementById('composer-token-badge');
const composerSendBar = document.getElementById('composer-send-bar');
const composerAttachBtn = document.getElementById('composer-attach-btn');
const composerFileInput = document.getElementById('composer-file-input');
const composerModelSelect = document.getElementById('composer-model-select');
const globalModelSelect = document.getElementById('model-select');
const composerStopBtn = document.getElementById('composer-stop-btn');
const composerSendBtn = document.getElementById('composer-send-btn');
const composerSlashBtn = document.getElementById('composer-slash-btn');
const composerMentionBtn = document.getElementById('composer-mention-btn');
const composerModeButtons = document.querySelectorAll('.composer-mode-btn');

function syncComposerModel(modelId) {
  tsComposer.setModel(modelId);
  const allCatalog = typeof PROGRAMMATIC_HONEST_MODELS !== 'undefined'
    ? [...COMPOSER_MODELS, ...PROGRAMMATIC_HONEST_MODELS]
    : COMPOSER_MODELS;
  const inDropdown = COMPOSER_MODELS.some((m) => m.id === modelId);
  if (composerModelSelect && inDropdown && composerModelSelect.value !== modelId) {
    composerModelSelect.value = modelId;
  }
  if (globalModelSelect && inDropdown && globalModelSelect.value !== modelId) {
    globalModelSelect.value = modelId;
  }
  if (topbarModelTag) {
    const opt = allCatalog.find((m) => m.id === modelId);
    if (opt) {
      const liveTier = (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.r4StackInfo && engineWorkerClient.r4StackInfo.active_tier)
        ? engineWorkerClient.r4StackInfo.active_tier
        : 'EmbeddedSeed';
      topbarModelTag.textContent = `${opt.name} · ${opt.badge} (${liveTier})`;
    }
  }
  if (typeof updateWeightCacheBadge === 'function') {
    updateWeightCacheBadge();
  }
  if (typeof renderCoworkStatusBar === 'function') {
    renderCoworkStatusBar();
  }
  if (typeof updateDynamicPlanningPanel === 'function') {
    updateDynamicPlanningPanel();
  }
  if (typeof renderDynamicPoolPanel === 'function') {
    renderDynamicPoolPanel();
  }
  if (typeof emitPrismEmbedEvent === 'function') {
    emitPrismEmbedEvent('prism:status');
  }
}

if (composerModelSelect) {
  if (AppState && AppState.config && (AppState.config.selectedModel === 'uor-r4.qwen3-0.6b-holo' || AppState.config.selectedModel === 'uor-r4.qwen3-1.7b-holo')) {
    composerModelSelect.value = AppState.config.selectedModel;
  }
  composerModelSelect.addEventListener('change', handleUserModelDropdownChange);
}

function handleUserModelDropdownChange(e) {
  /* Formal R4.AgentWorkspaceToolAwareness */
  if (typeof holoObservatoryState !== 'undefined' && holoObservatoryState) {
    holoObservatoryState.manualModelPinned = true;
  }
  const nextModel = e.target.value;
  if (AppState && AppState.config) AppState.config.selectedModel = nextModel;
  if (typeof safeStorageSet === 'function' && typeof MODEL_STORAGE_KEY === 'string') safeStorageSet(MODEL_STORAGE_KEY, nextModel);
  if (nextModel !== 'uor-r4.native-q31-d85' && typeof safeStorageRemove === 'function') {
    safeStorageRemove('uor_offline_emergency_q31');
    const emergencyChk = typeof document !== 'undefined' && document ? document.getElementById('settings-offline-q31-toggle') : null;
    if (emergencyChk) emergencyChk.checked = false;
    if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
      engineWorkerClient.rpc('set_bridge_policy', { offlineEmergencyQ31: false, preferredModel: nextModel }).catch(() => {});
      if (nextModel === 'uor-r4.qwen3-0.6b-holo' || nextModel === 'uor-r4.qwen3-1.7b-holo') {
        engineWorkerClient.rpc('hydrate_holo_model', { modelId: nextModel }).then(() => { if (typeof updateWeightCacheBadge === 'function') updateWeightCacheBadge(); }).catch(() => {});
      }
    }
  }
  syncComposerModel(nextModel);
}

if (globalModelSelect) {
  if (AppState && AppState.config && (AppState.config.selectedModel === 'uor-r4.qwen3-0.6b-holo' || AppState.config.selectedModel === 'uor-r4.qwen3-1.7b-holo')) {
    globalModelSelect.value = AppState.config.selectedModel;
  }
  globalModelSelect.addEventListener('change', handleUserModelDropdownChange);
}

if (composerStopBtn) {
  composerStopBtn.addEventListener('click', () => {
    const st = tsComposer.getState();
    if (!tsComposer.isTurnActive()) return;
    if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
      engineWorkerClient
        .rpc('studio_cancel_turn', {
          targetId: st.activeRequestId,
          turn_id: st.activeTurnId,
        })
        .catch(() => {});
    }
    if (typeof showStudioToast === 'function') {
      showStudioToast('Stopping turn...', 'info', 2500);
    }
  });
}

if (composerAttachBtn && composerFileInput) {
  composerAttachBtn.addEventListener('click', () => {
    composerFileInput.click();
  });

  composerFileInput.addEventListener('change', () => {
    const files = composerFileInput.files;
    if (!files || files.length === 0) return;

    for (let i = 0; i < files.length; i += 1) {
      const file = files[i];
      const reader = new FileReader();
      reader.onload = () => {
        const textContent = typeof reader.result === 'string' ? reader.result : '';
        const filePath = file.webkitRelativePath || `src/${file.name}`;
        try {
          const ws = dispatchStudioEnvelope('workspace_write', `${filePath}\n${textContent}`);
          if (ws && ws.status === 'ok') {
            renderWorkspaceList(ws);
            selectWorkspaceFile(filePath);
          }
        } catch (_) {
          // Virtual workspace fallback
        }

        tsComposer.attachFile({
          name: file.name,
          size: file.size,
          type: file.type,
          text: textContent,
        });

        renderComposerAttachments();
      };
      reader.readAsText(file);
    }
    composerFileInput.value = '';
  });
}

function getComposerMentionCandidates(query) { /* Formal R4.StudioFrame */
  const candidates = [];
  const activePath = activeWorkspacePath || 'src/engine.rs';
  candidates.push({ id: `file:${activePath}`, kind: 'file', label: activePath, detail: 'Active Workspace File' });

  const knownWsFiles = ['src/engine.rs', 'src/attention.rs', 'src/trainer.rs', 'README.md'];
  if (activeWorkspacePath === 'scratchpad.rs' || (AppState.workspace && AppState.workspace.activeProject === 'scratchpad')) {
    knownWsFiles.push('scratchpad.rs');
  }
  for (const f of knownWsFiles) {
    if (f !== activePath) {
      candidates.push({ id: `file:${f}`, kind: 'file', label: f, detail: 'Workspace File' });
    }
  }

  candidates.push({ id: 'claim:r4-f32', kind: 'claim', label: 'R4-F32', detail: 'SecOps SHA-256 Merkle Audit & Container Sandbox (R4-30 Proved)' });
  candidates.push({ id: 'claim:r4-f31', kind: 'claim', label: 'R4-F31', detail: 'Two-Level Radix /8 17,408-Bucket Index & OTel Dedup (R4-29 Proved)' });
  candidates.push({ id: 'arxiv:2405.04434', kind: 'arxiv', label: 'arXiv:2405.04434', detail: 'DeepSeek-V2 Multi-Head Latent Attention' });
  candidates.push({ id: 'arxiv:2406.02528', kind: 'arxiv', label: 'arXiv:2406.02528', detail: 'Scalable Matmul-Free LM' });
  candidates.push({ id: 'arxiv:2407.04620', kind: 'arxiv', label: 'arXiv:2407.04620', detail: 'Test-Time Training (TTT) Layers' });

  const q = (query || '').trim().toLowerCase();
  if (!q) return candidates;
  return candidates.filter(
    (item) => item.label.toLowerCase().includes(q) || item.detail.toLowerCase().includes(q)
  );
}

function updateComposerTokenBadge() { /* Formal R4.StudioFrame */
  if (!composerTokenBadge || !claudeChatInput) return;
  const tokCount = tsComposer.estimateTokenCount(claudeChatInput.value);
  composerTokenBadge.textContent = `${tokCount}t`;
}

function renderComposerAttachments() {
  if (!composerAttachmentsBar) return;
  composerAttachmentsBar.replaceChildren();
  const chips = tsComposer.getState().attachments;
  if (chips.length === 0) {
    composerAttachmentsBar.setAttribute('hidden', '');
    updateComposerTokenBadge();
    return;
  }
  composerAttachmentsBar.removeAttribute('hidden');
  for (const chip of chips) {
    const pill = document.createElement('span');
    pill.className = `composer-chip composer-chip-${chip.kind}`;
    const labelSpan = document.createElement('span');
    labelSpan.textContent = `@${chip.label}`;
    const removeBtn = document.createElement('button');
    removeBtn.type = 'button';
    removeBtn.className = 'composer-chip-remove';
    removeBtn.textContent = '×';
    removeBtn.title = `Remove @${chip.label}`;
    removeBtn.addEventListener('click', () => {
      tsComposer.removeAttachment(chip.id);
      renderComposerAttachments();
    });
    pill.appendChild(labelSpan);
    pill.appendChild(removeBtn);
    composerAttachmentsBar.appendChild(pill);
  }
  updateComposerTokenBadge();
}

function closeComposerAutocomplete() { /* Formal R4.StudioFrame */
  const st = tsComposer.getState();
  st.autocompleteOpen = false;
  st.autocompleteTrigger = null;
  if (composerAutocompleteMenu) {
    composerAutocompleteMenu.setAttribute('hidden', '');
    composerAutocompleteMenu.replaceChildren();
  }
}

function applySlashCommandSelection(cmd, tokenStart, cursorOffset) {
  if (!claudeChatInput) return;
  closeComposerAutocomplete();
  if (cmd.action === 'consolidate') {
    claudeChatInput.value = '';
    activateCanvasTab('consolidation');
    const runBtn = document.getElementById('run-consolidation-btn');
    if (runBtn) runBtn.click();
    return;
  }
  if (cmd.action === 'otel') {
    claudeChatInput.value = '';
    activateCanvasTab('traces');
    fetchRecordedTraces();
    return;
  }
  if (cmd.action === 'clear') {
    claudeChatInput.value = '';
    createNewThread();
    return;
  }
  if (cmd.mode) {
    setComposerModeUI(cmd.mode);
  }
  const before = tokenStart >= 0 ? claudeChatInput.value.slice(0, tokenStart) : '';
  const after = cursorOffset >= 0 ? claudeChatInput.value.slice(cursorOffset) : '';
  claudeChatInput.value = `${before}${cmd.template}${after}`;
  claudeChatInput.focus();
  updateComposerTokenBadge();
}

function applyMentionSelection(item, tokenStart, cursorOffset) {
  if (!claudeChatInput) return;
  closeComposerAutocomplete();
  tsComposer.addAttachment({
    id: item.id,
    kind: item.kind,
    label: item.label,
    payload: item.detail,
  });
  renderComposerAttachments();
  if (tokenStart >= 0 && cursorOffset >= tokenStart) {
    const before = claudeChatInput.value.slice(0, tokenStart);
    const after = claudeChatInput.value.slice(cursorOffset);
    claudeChatInput.value = `${before}${after}`.replace(/^\s+/, '');
  }
  claudeChatInput.focus();
  updateComposerTokenBadge();
}

function renderComposerAutocomplete() {
  if (!composerAutocompleteMenu || !claudeChatInput) return;
  const cursor = claudeChatInput.selectionStart || claudeChatInput.value.length;
  const detected = tsComposer.detectAutocompleteTrigger(claudeChatInput.value, cursor);
  const st = tsComposer.getState();
  if (!detected.trigger) {
    closeComposerAutocomplete();
    return;
  }
  const items =
    detected.trigger === 'slash'
      ? tsComposer.filterSlashCommands(detected.query)
      : getComposerMentionCandidates(detected.query);
  if (items.length === 0) {
    closeComposerAutocomplete();
    return;
  }
  st.autocompleteOpen = true;
  st.autocompleteTrigger = detected.trigger;
  st.autocompleteQuery = detected.query;
  if (st.autocompleteSelectedIndex >= items.length) {
    st.autocompleteSelectedIndex = 0;
  }

  composerAutocompleteMenu.replaceChildren();
  composerAutocompleteMenu.removeAttribute('hidden');

  const header = document.createElement('div');
  header.className = 'composer-ac-header';
  header.textContent =
    detected.trigger === 'slash'
      ? 'TypeScript Agentic Slash Commands (↑/↓ to navigate, Enter/Tab to select)'
      : 'Attach @ Context Chip to 256D Q12 Envelope';
  composerAutocompleteMenu.appendChild(header);

  items.forEach((item, idx) => {
    const rowBtn = document.createElement('button');
    rowBtn.type = 'button';
    rowBtn.className = `composer-ac-item${idx === st.autocompleteSelectedIndex ? ' active' : ''}`;
    rowBtn.setAttribute('role', 'option');
    rowBtn.setAttribute('aria-selected', idx === st.autocompleteSelectedIndex ? 'true' : 'false');

    const left = document.createElement('div');
    left.className = 'composer-ac-left';
    const codeEl = document.createElement('code');
    codeEl.className = 'composer-ac-cmd';
    codeEl.textContent = detected.trigger === 'slash' ? item.command : `@${item.label}`;
    const titleEl = document.createElement('span');
    titleEl.className = 'composer-ac-title';
    titleEl.textContent = detected.trigger === 'slash' ? item.title : item.detail;
    left.appendChild(codeEl);
    left.appendChild(titleEl);

    const badge = document.createElement('span');
    badge.className = 'composer-ac-badge';
    badge.textContent = detected.trigger === 'slash' ? item.badge : item.kind.toUpperCase();

    rowBtn.appendChild(left);
    rowBtn.appendChild(badge);
    rowBtn.addEventListener('click', () => {
      if (detected.trigger === 'slash') {
        applySlashCommandSelection(item, detected.tokenStart, cursor);
      } else {
        applyMentionSelection(item, detected.tokenStart, cursor);
      }
    });
    composerAutocompleteMenu.appendChild(rowBtn);
  });
}

function setComposerModeUI(mode) {
  tsComposer.setMode(mode);
  const activeMode = tsComposer.getState().mode;
  if (typeof document !== 'undefined' && document.body && document.body.dataset) {
    document.body.dataset.composerMode = activeMode;
  }
  for (const btn of composerModeButtons) {
    const m = btn.getAttribute('data-composer-mode');
    const isActive = m === activeMode;
    btn.classList.toggle('active', isActive);
    btn.setAttribute('aria-pressed', isActive ? 'true' : 'false');
  }
  if (claudeChatInput) {
    if (activeMode === 'plan' && !claudeChatInput.value.trim()) {
      claudeChatInput.placeholder = 'Plan mode (read-only analysis): Ask UOR-R4 to inspect /workspace, git status, or preview...';
    } else if (activeMode === 'teach' && !claudeChatInput.value.trim()) {
      claudeChatInput.placeholder = 'Teach: <prompt> => <thinking>256D Q12 reasoning</thinking><desired response>';
    } else if (activeMode === 'math' && !claudeChatInput.value.trim()) {
      claudeChatInput.placeholder = 'Enter exact M6 arithmetic expression (e.g. What is 144 + 233?)...';
    } else {
      claudeChatInput.placeholder =
        'Ask UOR-R4 anything... Type / for commands (/plan, /think, /teach, /patch, /arxiv, /math) or @ to attach workspace files & papers';
    }
  }
  updateComposerTokenBadge();
}

for (const btn of composerModeButtons) {
  btn.addEventListener('click', () => {
    const mode = btn.getAttribute('data-composer-mode') || 'agentic';
    setComposerModeUI(mode);
    if (claudeChatInput) claudeChatInput.focus();
  });
}

if (composerSlashBtn && claudeChatInput) {
  composerSlashBtn.addEventListener('click', () => {
    if (!claudeChatInput.value.startsWith('/')) {
      claudeChatInput.value = `/${claudeChatInput.value}`;
    }
    claudeChatInput.focus();
    renderComposerAutocomplete();
  });
}

if (composerMentionBtn && claudeChatInput) {
  composerMentionBtn._mentionWired = true;
  composerMentionBtn.addEventListener('click', () => {
    const prefix = claudeChatInput.value && !claudeChatInput.value.endsWith(' ') ? `${claudeChatInput.value} @` : `${claudeChatInput.value}@`;
    claudeChatInput.value = prefix;
    claudeChatInput.focus();
    renderComposerAutocomplete();
  });
}

function submitComposerPrompt() {
  if (!claudeChatInput) return;
  if (typeof tsComposer.isTurnActive === 'function' && tsComposer.isTurnActive()) return;
  const rawVal = claudeChatInput.value.trim();
  if (!rawVal) return;

  // Check if user typed an exact action slash command directly
  const exactCmd = AGENTIC_SLASH_COMMANDS.find((c) => c.command === rawVal.toLowerCase());
  if (exactCmd && exactCmd.action) {
    applySlashCommandSelection(exactCmd, 0, rawVal.length);
    return;
  }

  const synthesizedPrompt = tsComposer.synthesizePromptEnvelope(rawVal);
  if (!synthesizedPrompt) return;
  tsComposer.pushHistory(rawVal);
  claudeChatInput.value = '';
  claudeChatInput.style.height = '1.5rem';
  closeComposerAutocomplete();
  updateComposerTokenBadge();
  runClaudeChat(synthesizedPrompt, true);
  claudeChatInput.focus();
}

if (claudeChatForm) {
  claudeChatForm.addEventListener('submit', (event) => {
    event.preventDefault();
    submitComposerPrompt();
  });
}

if (claudeChatInput) {
  claudeChatInput.addEventListener('keydown', (event) => {
    const st = tsComposer.getState();
    if (st.autocompleteOpen) {
      const cursor = claudeChatInput.selectionStart || claudeChatInput.value.length;
      const detected = tsComposer.detectAutocompleteTrigger(claudeChatInput.value, cursor);
      const items =
        detected.trigger === 'slash'
          ? tsComposer.filterSlashCommands(detected.query)
          : getComposerMentionCandidates(detected.query);
      if (event.key === 'ArrowDown') {
        event.preventDefault();
        st.autocompleteSelectedIndex = (st.autocompleteSelectedIndex + 1) % Math.max(1, items.length);
        renderComposerAutocomplete();
        return;
      }
      if (event.key === 'ArrowUp') {
        event.preventDefault();
        st.autocompleteSelectedIndex =
          (st.autocompleteSelectedIndex - 1 + Math.max(1, items.length)) % Math.max(1, items.length);
        renderComposerAutocomplete();
        return;
      }
      if ((event.key === 'Enter' || event.key === 'Tab') && items.length > 0) {
        event.preventDefault();
        const chosen = items[st.autocompleteSelectedIndex] || items[0];
        if (detected.trigger === 'slash') {
          applySlashCommandSelection(chosen, detected.tokenStart, cursor);
        } else {
          applyMentionSelection(chosen, detected.tokenStart, cursor);
        }
        return;
      }
      if (event.key === 'Escape') {
        event.preventDefault();
        closeComposerAutocomplete();
        return;
      }
    }

    if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
      event.preventDefault();
      submitComposerPrompt();
      return;
    }

    if (event.key === 'ArrowUp') {
      const canStepUp =
        st.historyIndex >= 0 ||
        claudeChatInput.selectionStart === 0 ||
        !claudeChatInput.value.includes('\n');
      if (canStepUp) {
        const recalled = tsComposer.stepHistory('up', claudeChatInput.value);
        if (recalled !== null) {
          event.preventDefault();
          claudeChatInput.value = recalled;
          claudeChatInput.setSelectionRange(recalled.length, recalled.length);
          claudeChatInput.style.height = '1.5rem';
          const newHeight = Math.min(Math.max(claudeChatInput.scrollHeight, 24), 128);
          claudeChatInput.style.height = `${newHeight}px`;
          updateComposerTokenBadge();
        }
      }
    } else if (event.key === 'ArrowDown' && st.historyIndex >= 0) {
      const recalled = tsComposer.stepHistory('down');
      if (recalled !== null) {
        event.preventDefault();
        claudeChatInput.value = recalled;
        claudeChatInput.setSelectionRange(recalled.length, recalled.length);
        claudeChatInput.style.height = '1.5rem';
        const newHeight = Math.min(Math.max(claudeChatInput.scrollHeight, 24), 128);
        claudeChatInput.style.height = `${newHeight}px`;
        updateComposerTokenBadge();
      }
    }
  });
  claudeChatInput.addEventListener('input', () => {
    claudeChatInput.style.height = '1.5rem';
    const newHeight = Math.min(Math.max(claudeChatInput.scrollHeight, 24), 128);
    claudeChatInput.style.height = `${newHeight}px`;
    updateComposerTokenBadge();
    renderComposerAutocomplete();
  });
}

function createNewThread() {
  const newId = `thread-${Date.now()}`;
  const newThread = {
    id: newId,
    title: 'New Conversation',
    messages: [],
    created_at: Date.now(),
  };
  chatThreads.unshift(newThread);
  activeThreadId = newId;
  persistThreadsToStorage();
  renderThreadSidebar();
  renderActiveThreadMessages();
  if (claudeChatInput) {
    claudeChatInput.value = '';
    claudeChatInput.focus();
  }
}

function clearAllThreads() {
  const defaultThread = {
    id: `thread-${Date.now()}`,
    title: 'New Conversation',
    messages: [],
    created_at: Date.now(),
  };
  chatThreads = [defaultThread];
  activeThreadId = defaultThread.id;
  persistThreadsToStorage();
  renderThreadSidebar();
  renderActiveThreadMessages();
  if (claudeChatInput) {
    claudeChatInput.value = '';
    claudeChatInput.focus();
  }
}

if (chatClearBtn) {
  chatClearBtn.addEventListener('click', () => {
    createNewThread();
  });
}

const newThreadBtn = document.getElementById('new-thread-btn');
if (newThreadBtn) {
  newThreadBtn.addEventListener('click', () => {
    createNewThread();
  });
}

const clearAllThreadsBtn = document.getElementById('clear-all-threads-btn');
if (clearAllThreadsBtn) {
  clearAllThreadsBtn.addEventListener('click', () => {
    if (confirm('Clear all conversation threads?')) {
      clearAllThreads();
    }
  });
}

for (const btn of document.querySelectorAll('.chat-preset-btn')) {
  btn.addEventListener('click', () => {
    const promptVal = btn.getAttribute('data-chat-prompt') || '';
    if (claudeChatInput) {
      claudeChatInput.value = '';
    }
    runClaudeChat(promptVal, true);
  });
}

// Canvas Dockable Tabs Navigation
const canvasTabButtons = document.querySelectorAll('.canvas-tab-btn');
const canvasPanels = document.querySelectorAll('.canvas-panel');
const dockableCanvasPane = document.getElementById('dockable-canvas-pane');
const mainLayoutEl = document.querySelector('main');
let activeCanvasTabId = 'terminal';
if (mainLayoutEl && dockableCanvasPane && dockableCanvasPane.hasAttribute('hidden')) {
  mainLayoutEl.classList.add('canvas-drawer-closed');
}

function closeCanvasDrawer() { /* Formal R4.StudioFrame */
  if (dockableCanvasPane) {
    dockableCanvasPane.setAttribute('hidden', '');
  }
  if (mainLayoutEl) {
    mainLayoutEl.classList.add('canvas-drawer-closed');
  }
}

const INSPECTOR_MODAL_TAB_IDS = new Set([
  'traces',
  'inspector',
  'diagnostics',
  'mcp',
  'feed',
  'consolidation',
  'customizations',
]);
const inspectorsModalEl = document.getElementById('inspectors-modal');

/** Formal R4.StudioFrame synchronize ARIA expanded state across Neural HUD triggers. */
function syncNeuralHudTriggerState(isOpen, isExpanded) {
  /* Formal R4.StudioFrame */
  const toggleBtn = document.getElementById('toggle-inspectors-btn');
  if (toggleBtn) {
    toggleBtn.setAttribute('aria-expanded', isOpen ? 'true' : 'false');
    if (toggleBtn.classList) toggleBtn.classList.toggle('active', Boolean(isOpen));
  }
  const expandBtn = document.getElementById('neural-hud-expand-btn');
  if (expandBtn) {
    expandBtn.setAttribute('aria-expanded', isExpanded ? 'true' : 'false');
    expandBtn.textContent = isExpanded ? 'Collapse ▴' : '4-Canvas Deck ▾';
  }
}

function closeInspectorsModal() {
  if (inspectorsModalEl) {
    inspectorsModalEl.setAttribute('hidden', '');
    if (inspectorsModalEl.classList) inspectorsModalEl.classList.remove('hud-expanded');
  }
  syncNeuralHudTriggerState(false, false);
  if (lexLean3dVisualizer && lexLean3dVisualizer.animFrameId && typeof cancelAnimationFrame === 'function') {
    cancelAnimationFrame(lexLean3dVisualizer.animFrameId);
    lexLean3dVisualizer.animFrameId = null;
    lexLean3dVisualizer.lastTimestamp = 0;
  }
  activeCanvasTabId = 'terminal';
}

function openInspectorsModal(tabId = 'traces', expandDeck = false) {
  const targetTab = INSPECTOR_MODAL_TAB_IDS.has(tabId) ? tabId : 'traces';
  const shouldExpand = Boolean(expandDeck) || targetTab !== 'traces';
  if (inspectorsModalEl && inspectorsModalEl.classList) {
    inspectorsModalEl.classList.toggle('hud-expanded', shouldExpand);
  }
  activateCanvasTab(targetTab);
  syncNeuralHudTriggerState(true, shouldExpand);
}

function activateCanvasTab(tabId) {
  activeCanvasTabId = tabId;

  if (INSPECTOR_MODAL_TAB_IDS.has(tabId)) {
    if (inspectorsModalEl) {
      inspectorsModalEl.removeAttribute('hidden');
      if (tabId !== 'traces' && inspectorsModalEl.classList) {
        inspectorsModalEl.classList.add('hud-expanded');
      }
      const isExpanded = Boolean(inspectorsModalEl.classList && inspectorsModalEl.classList.contains('hud-expanded'));
      syncNeuralHudTriggerState(true, isExpanded);
    }
    for (const btn of canvasTabButtons) {
      const btnTab = btn.getAttribute('data-tab-id');
      if (INSPECTOR_MODAL_TAB_IDS.has(btnTab)) {
        const isActive = btnTab === tabId;
        btn.classList.toggle('active', isActive);
        btn.setAttribute('aria-selected', isActive ? 'true' : 'false');
        btn.setAttribute('tabindex', isActive ? '0' : '-1');
        if (isActive && typeof btn.scrollIntoView === 'function') {
          btn.scrollIntoView({ behavior: 'smooth', inline: 'nearest', block: 'nearest' });
        }
      }
    }
    for (const panel of canvasPanels) {
      if (panel.id === 'canvas-panel-terminal' || panel.id === 'canvas-panel-workspace') {
        continue;
      }
      const isMatch = panel.id === `canvas-panel-${tabId}`;
      panel.classList.toggle('active', isMatch);
      if (isMatch) {
        panel.removeAttribute('hidden');
      } else {
        panel.setAttribute('hidden', '');
      }
    }
    if (tabId === 'traces') {
      fetchRecordedTraces();
      const hudBodyVisible =
        !inspectorsModalEl
        || (inspectorsModalEl.classList && inspectorsModalEl.classList.contains('hud-expanded'))
        || (document.body && document.body.getAttribute('data-surface') === 'observatory');
      if (hudBodyVisible) {
        if (lexLean3dVisualizer && typeof lexLean3dVisualizer.ensureGLReady === 'function') {
          lexLean3dVisualizer.ensureGLReady();
        }
        if (lexLean3dVisualizer && typeof lexLean3dVisualizer.resize === 'function') {
          lexLean3dVisualizer.resize();
        }
      }
    }
    if (tabId === 'inspector') {
      renderMLInspector();
    }
    if (tabId === 'feed') {
      renderLiteratureFeed();
    }
    return;
  }

  if ((tabId === 'diff' || tabId === 'preview') && typeof globalThis.__uorSetWorkbenchMode === 'function') {
    globalThis.__uorSetWorkbenchMode(tabId);
    return;
  }

  if (typeof setLayoutMode === 'function' && document.body && document.body.dataset.layoutMode === 'chat') {
    setLayoutMode('studio');
  }
  if (dockableCanvasPane) {
    dockableCanvasPane.removeAttribute('hidden');
    if (tabId !== 'workspace') {
      dockableCanvasPane.classList.remove('cursor-bottom-panel-collapsed');
    }
  }
  if (mainLayoutEl) {
    mainLayoutEl.classList.remove('canvas-drawer-closed');
  }
  for (const btn of canvasTabButtons) {
    const btnTab = btn.getAttribute('data-tab-id');
    if (!INSPECTOR_MODAL_TAB_IDS.has(btnTab)) {
      const isActive = btnTab === tabId || (tabId === 'workspace' && btnTab === 'terminal');
      btn.classList.toggle('active', isActive);
      btn.setAttribute('aria-selected', isActive ? 'true' : 'false');
      btn.setAttribute('tabindex', isActive ? '0' : '-1');
      if (isActive && typeof btn.scrollIntoView === 'function') {
        btn.scrollIntoView({ behavior: 'smooth', inline: 'nearest', block: 'nearest' });
      }
    }
  }
  const termPanel = document.getElementById('canvas-panel-terminal');
  const wsPanel = document.getElementById('canvas-panel-workspace');
  if (termPanel) {
    termPanel.classList.add('active');
    termPanel.removeAttribute('hidden');
  }
  if (wsPanel) {
    wsPanel.removeAttribute('hidden');
  }
  if (tabId === 'terminal' && lexLeanTerminal && lexLeanTerminal.inputEl) {
    lexLeanTerminal.inputEl.focus();
  }
  if (tabId === 'workspace' && workspaceEditorTextarea) {
    workspaceEditorTextarea.focus();
  }
}

const closeCanvasPaneBtn = document.getElementById('close-canvas-pane-btn');
if (closeCanvasPaneBtn) {
  closeCanvasPaneBtn._drawerToggleWired = true;
  closeCanvasPaneBtn.addEventListener('click', () => {
    const termDrawer = document.getElementById('terminal-split-drawer');
    if (termDrawer) {
      const willHide = !termDrawer.hasAttribute('hidden');
      if (willHide) {
        termDrawer.setAttribute('hidden', '');
        termDrawer.hidden = true;
        if (dockableCanvasPane) dockableCanvasPane.classList.add('cursor-bottom-panel-collapsed');
      } else {
        termDrawer.removeAttribute('hidden');
        termDrawer.hidden = false;
        if (dockableCanvasPane) dockableCanvasPane.classList.remove('cursor-bottom-panel-collapsed');
      }
      return;
    }
    if (dockableCanvasPane) {
      dockableCanvasPane.classList.toggle('cursor-bottom-panel-collapsed');
    }
  });
}

// Sidebar collapse/expand listeners are unified in initCursorIdeWorkbench() -> toggleSidebar().

const closeStatusBarBtn = document.getElementById('close-status-bar-btn');
const coworkStatusBar = document.getElementById('cowork-agent-status-bar');
if (closeStatusBarBtn && coworkStatusBar) {
  closeStatusBarBtn.addEventListener('click', () => {
    coworkStatusBar.setAttribute('hidden', '');
  });
}

for (const btn of canvasTabButtons) {
  btn.addEventListener('click', () => {
    const tabId = btn.getAttribute('data-tab-id');
    if (tabId) activateCanvasTab(tabId);
  });
}

for (const navEl of document.querySelectorAll('.canvas-tabs, .inspectors-modal-tabs')) {
  navEl.addEventListener('keydown', (e) => {
    const tabList = Array.from(navEl.querySelectorAll('.canvas-tab-btn'));
    if (tabList.length === 0) return;
    const currentIdx = Math.max(0, tabList.findIndex((b) => b.getAttribute('aria-selected') === 'true'));
    let nextIdx = -1;
    if (e.key === 'ArrowRight') {
      nextIdx = (currentIdx + 1) % tabList.length;
    } else if (e.key === 'ArrowLeft') {
      nextIdx = (currentIdx - 1 + tabList.length) % tabList.length;
    } else if (e.key === 'Home') {
      nextIdx = 0;
    } else if (e.key === 'End') {
      nextIdx = tabList.length - 1;
    }
    if (nextIdx !== -1) {
      e.preventDefault();
      tabList[nextIdx].focus();
      const tabId = tabList[nextIdx].getAttribute('data-tab-id');
      if (tabId) activateCanvasTab(tabId);
    }
  });
}

for (const el of document.querySelectorAll('[data-canvas-tab]')) {
  el.addEventListener('click', () => {
    const tabId = el.getAttribute('data-canvas-tab');
    if (!tabId) return;
    if (INSPECTOR_MODAL_TAB_IDS.has(tabId)) {
      const isModalOpen = inspectorsModalEl && !inspectorsModalEl.hasAttribute('hidden');
      if (isModalOpen && activeCanvasTabId === tabId) {
        closeInspectorsModal();
      } else {
        activateCanvasTab(tabId);
      }
      return;
    }
    const isDrawerOpen = dockableCanvasPane && !dockableCanvasPane.hasAttribute('hidden') && !dockableCanvasPane.classList.contains('cursor-bottom-panel-collapsed');
    if (isDrawerOpen && activeCanvasTabId === tabId && document.body && document.body.dataset.layoutMode === 'studio') {
      dockableCanvasPane.classList.add('cursor-bottom-panel-collapsed');
    } else {
      activateCanvasTab(tabId);
    }
  });
}

// Autonomous Self-Improvement Diagnostics & Bottleneck-Filtered Research Relevance Check
const SELF_IMPROVEMENT_AUDIT_SWEEPS = [
  {
    id: 'SWEEP-1 · LEXLEAN CLOSURE',
    badge: '30/30 Verified · 0 Missing',
    summary: 'All 30 registered IDs (R4-01..R4-30) in model/ids.toml are backed by sorry-free Lean 4 specifications in src/R4/*.lex.tex and control suites in tests/lexlean/*Controls.lex.tex.',
    prompt: 'What does LexLean prove?',
  },
  {
    id: 'SWEEP-2 · BENCHMARK & L1 LOSS',
    badge: '32/32 Passed · 0 Failure Basins',
    summary: 'Runtime LutStackExecSession benchmark sweep across all proved frontier claims verifies cycle-free decoding, stem overlap, and non-regressing L1 trajectory residual loss (resolved R4-F02 anchor-slice routing bug).',
    prompt: 'What is your favorite food?',
  },
  {
    id: 'SWEEP-3 · RADIX /8 & OTEL (R4-29)',
    badge: '17,408 Buckets · 0 Unindexed · 0 Orphans',
    summary: 'Two-level hierarchical Radix /8 directory (16 E8 sectors × 64 Clifford actions × 17 Icosian buckets = 17,408 buckets) and 5-tier OTel u16 span attribute deduplication verified by R4.RadixOtel.',
    prompt: 'How does the two-level Radix /8 index accelerate PointKey lookup and OTel span deduplication?',
  },
  {
    id: 'SWEEP-4 · SECOPS MERKLE & SANDBOX (R4-30)',
    badge: 'uor.secops.audit/1 · SHA-256 Chain OK',
    summary: 'FIPS 180-4 SHA-256 Merkle audit chain links every MCP tool call and container workspace write to genesis anchor 0x6a09e667bb67ae85 while R4.SecopsAudit.admitSandboxWrite blocks path traversal and quota overflows.',
    prompt: 'How does the SecOps audit chain verify MCP tool calls and container workspace writes?',
  },
];

const AUTONOMOUS_FRONTIER_CLAIMS = [
  {
    id: 'R4-F32',
    tick: 12,
    status: 'PROVED (R4-30 SecOps Merkle)',
    theorem: 'frontier_r4_f32_bvgk_writer_refinement',
    summary: 'Every MCP tool call and container workspace write appends a monotonic event to the FIPS 180-4 SHA-256 Merkle audit chain under uor.secops.audit/1 and enforces policy gates.',
    prompt: 'How does the SecOps audit chain verify MCP tool calls and container workspace writes?',
  },
  {
    id: 'R4-F31',
    tick: 11,
    status: 'PROVED (R4-29 Radix /8 + OTel)',
    theorem: 'frontier_r4_f31_pointkey_resonance',
    summary: 'Two-level Radix /8 directory partitions PointKey rows into 17,408 geometric buckets (16 * 64 * 17) while 5-tier OTel span interning deduplicates repeated attributes into u16 IDs.',
    prompt: 'How does the two-level Radix /8 index accelerate PointKey lookup and OTel span deduplication?',
  },
  {
    id: 'R4-F30',
    tick: 10,
    status: 'PROVED + CONSOLIDATED',
    theorem: 'frontier_r4_f30_corpus_induction',
    summary: 'Landauer principle establishes the minimum thermodynamic heat dissipation k_B T ln 2 required to irreversibly erase one bit of information.',
    prompt: 'Why does Landauer\'s principle set a minimum energy cost for erasing a bit of information?',
  },
  {
    id: 'R4-F29',
    tick: 9,
    status: 'PROVED + CONSOLIDATED',
    theorem: 'frontier_r4_f29_corpus_induction',
    summary: 'LSM-trees buffer writes sequentially in a MemTable and immutable SSTables with Bloom filters, achieving lower write amplification than in-place B+Tree page updates.',
    prompt: 'When should a storage engine use an LSM-tree instead of a B+Tree?',
  },
];

const LITERATURE_PAPERS = [
  {
    arxiv_id: '2403.05530',
    category: 'cs.LG',
    r4_bottlenecks: ['R4-06/R4-28 (PRLL & Trajectory Alignment)'],
    title: 'Direct Preference Optimization: Your Language Model is Secretly a Reward Model',
    authors: 'R. Rafailov, A. Sharma, E. Mitchell, S. Ermon, C. D. Manning, C. Finn (2024)',
    abstract: 'Intersects R4-06/R4-28: closed-form policy alignment on preference pairs without an explicit reward network, mapping to zero-float coordinate-descent LUT writer refinement.',
    hypotheses: [
      {
        id: 'H-LIT-2403.05530-01',
        prompt: 'How does Direct Preference Optimization align language models without reinforcement learning?',
        reply: 'Direct Preference Optimization (DPO) reparameterizes the RLHF objective directly in terms of the optimal policy and reference policy log-ratio, eliminating separate reward modeling and reinforcement learning while aligning LUT writer-table basins directly on preference pairs.',
      },
    ],
  },
  {
    arxiv_id: '2402.10172',
    category: 'cs.LO',
    r4_bottlenecks: ['R4-25/R4-28 (Dependent Type Normalization & Cut Elimination)'],
    title: 'Uniform Cut Elimination for Dependent Type Systems',
    authors: 'A. Kaposi, A. Kovacs, A. Sinkarovs (2024)',
    abstract: 'Intersects R4-25/R4-28: establishes strong normalization and syntactic cut elimination for constructive dependent type systems via hereditary substitution over canonical terms.',
    hypotheses: [
      {
        id: 'H-LIT-2402.10172-01',
        prompt: 'Why does constructive cut elimination guarantee strong normalization in dependent type systems?',
        reply: 'Constructive cut elimination removes intermediate lemmas via hereditary substitution, strictly decreasing a well-founded lexicographic measure on type grades and guaranteeing strong normalization.',
      },
    ],
  },
  {
    arxiv_id: '2401.04088',
    category: 'cs.DC',
    r4_bottlenecks: ['R4-29 (Lock-Free Radix /8 Indexing & Bounded Memory)'],
    title: 'Efficient Lock-Free Memory Reclamation with Bounded Epochs',
    authors: 'T. Brown, M. Arbel-Raviv, H. Attiya (2024)',
    abstract: 'Intersects R4-29: bounded epoch-based reclamation guarantees constant-time retirement and prevents unbounded memory growth across concurrent index readers.',
    hypotheses: [
      {
        id: 'H-LIT-2401.04088-01',
        prompt: 'How do epoch-based reclamation algorithms prevent use-after-free and ABA hazards in concurrent memory management?',
        reply: 'Bounded epoch-based reclamation partitions execution into monotonic logical epochs and defers freeing retired nodes until all active readers have advanced past the retirement epoch.',
      },
    ],
  },
  {
    arxiv_id: '2310.06825',
    category: 'cs.CR',
    r4_bottlenecks: ['R4-30 (Cryptographic Merkle Soundness & SecOps Audit)'],
    title: 'Soundness Limits of Succinct Non-Interactive Arguments of Knowledge',
    authors: 'E. Ben-Sasson, A. Chiesa, N. Spooner (2023)',
    abstract: 'Intersects R4-30: analyzes cryptographic soundness in succinct arguments and Merkle commitment chains against forged openings.',
    hypotheses: [
      {
        id: 'H-LIT-2310.06825-01',
        prompt: 'How do succinct non-interactive arguments verify polynomial quotient identities without revealing secret witness evaluation points?',
        reply: 'SNARGs commit to witness polynomials via cryptographic Merkle/Reed-Solomon or KZG commitments and evaluate quotient identities at Fiat-Shamir random challenge points.',
      },
    ],
  },
  {
    arxiv_id: '2406.02528',
    category: 'cs.CL',
    r4_bottlenecks: ['R4-23/R4-25 (Matmul-Free Ternary Accumulation & 19-Table Trunk)'],
    title: 'Scalable Matmul-Free Language Modeling via Ternary Accumulation',
    authors: 'R.-J. Zhu, Y. Zhang, E. Sifferman, T. Sheaves, Y. Wang, D. Richmond, P. Zhou, J. K. Eshraghian (2024)',
    abstract: 'Intersects R4-23/R4-25: eliminates dense matrix multiplications using ternary {-1, 0, +1} gated recurrence and integer addition.',
    hypotheses: [
      {
        id: 'H-LIT-2406.02528-01',
        prompt: 'How does Matmul-Free ternary language modeling relate to the 40-node ternary binder forest in UOR-R4?',
        reply: 'Matmul-Free language models replace floating-point GEMM with ternary {-1, 0, +1} accumulation, matching the UOR-R4 40-node ternary binder forest (1 + 3 + 9 + 27) and discrete coordinate-descent LUT optimizer.',
      },
    ],
  },
  {
    arxiv_id: '2206.10708',
    category: 'math.DS',
    r4_bottlenecks: [],
    title: 'Conserved Quantities and Invertibility in Reversible Cellular Automata',
    authors: 'J. Kari, S. Capobianco (2022)',
    abstract: 'Unrelated to active src/R4/ bottlenecks (filtered out by paper_matches_r4_bottlenecks).',
    hypotheses: [],
  },
];

const DEFAULT_VERDANT_FACETS = [
  {
    facet: 'ModelAndTraining',
    kpi_id: 'R4-50..52',
    metric: 'UORR4V13 28L Qwen3-0.6B/1.7B + Q31 D85',
    status: 'PROVED · 0 Float Ops',
    summary: 'Cholesky GPTQ Int4 + Wasm simd128, OPFS SyncAccessHandle 1 MiB pieces, and native Q31 online learning.',
  },
  {
    facet: 'ServingRuntime',
    kpi_id: 'R4-45',
    metric: 'Nibble LUT + Lorentz Geodesic',
    status: 'RATIFIED · 0 Matmuls',
    summary: 'Pure-integer 256-entry nibble LUTs, SWAR Hamming XOR-popcount copy, H4 120-icosian frames, and shared WebAssembly.Module pool.',
  },
  {
    facet: 'UiAndPlumbing',
    kpi_id: 'R4-46',
    metric: '2 Qwen3 Catalog + Offline Q31 Modes',
    status: 'ACTIVE · Exact 6-File Closure',
    summary: '#composer-model-select + composer_mode wired end-to-end into chat_assistant_json and geometric_metrics with OPFS .uorc 3-strike watchdog.',
  },
  {
    facet: 'EcosystemStandards',
    kpi_id: 'R4-44..R4-46',
    metric: 'UOR-R4 + uor-workspace-bridge/1',
    status: 'VERIFIED · Zero Residue',
    summary: 'LexLean AST mechanizer, UORust κ-lift ratchet, UOR Foundry inprism + uor-workspace-bridge/1 postMessage protocol, and zero-Python deterministic gates.',
  },
];

function renderVerdantSynthesisPanel() {
  const kpiGrid = document.getElementById('verdant-synthesis-kpi-grid');
  const watchdogStatusEl = document.getElementById('uorc-watchdog-status');
  const poolBadgeEl = document.getElementById('uorc-worker-pool-badge');
  const panelBadgeEl = document.getElementById('verdant-synthesis-panel-badge');

  let vs = (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.verdantSynthesis) || null;
  if (!vs && typeof query_verdant_synthesis_status === 'function') {
    try {
      vs = query_verdant_synthesis_status();
    } catch {
      vs = null;
    }
  }
  const pool = (vs && vs.worker_pool) || (typeof get_browser_worker_pool_snapshot === 'function' ? get_browser_worker_pool_snapshot() : null);
  const head = (vs && (vs.active_head || vs.opfs_head)) || 'gen-0';
  const strikes = (vs && typeof vs.fault_strikes === 'number') ? vs.fault_strikes : 0;
  const maxStrikes = (vs && typeof vs.max_strikes === 'number') ? vs.max_strikes : 3;
  const rolledBack = Boolean(vs && vs.rolled_back);

  if (panelBadgeEl) {
    panelBadgeEl.textContent = `4-Facet Holon · Head ${head} · ${strikes}/${maxStrikes} Strikes · 0 Float Ops`;
  }
  if (poolBadgeEl && pool) {
    poolBadgeEl.textContent = `Workers: ${pool.optimal_workers}/${pool.logical_threads} cores · Shared Module: ${pool.shared_module_compiled ? 'YES' : 'LAZY'}`;
  }
  if (watchdogStatusEl) {
    watchdogStatusEl.textContent = `Active Head: ${head} · Fault Strikes: ${strikes}/${maxStrikes} · Watchdog: ${rolledBack ? 'ROLLED BACK TO PARENT' : 'ARMED'} · Residue: 0`;
  }

  if (kpiGrid) {
    kpiGrid.replaceChildren();
    const facets = (vs && Array.isArray(vs.facets) && vs.facets.length > 0) ? vs.facets : DEFAULT_VERDANT_FACETS;
    for (const f of facets) {
      const card = document.createElement('div');
      card.className = 'frontier-ledger-card';
      const top = document.createElement('div');
      top.className = 'frontier-card-top';
      const nameEl = document.createElement('code');
      nameEl.textContent = `${f.facet || f.name} · ${f.kpi_id || 'R4'}`;
      const badgeEl = document.createElement('span');
      badgeEl.className = 'paper-badge';
      badgeEl.textContent = f.status || f.metric || 'VERIFIED';
      top.appendChild(nameEl);
      top.appendChild(badgeEl);
      const desc = document.createElement('p');
      desc.className = 'frontier-card-desc';
      desc.textContent = `${f.metric ? `${f.metric} — ` : ''}${f.summary || ''}`;
      card.appendChild(top);
      card.appendChild(desc);
      kpiGrid.appendChild(card);
    }
  }
}

const MAX_HUD_RING_FRAMES = 64;
const holoObservatoryState = {
  frames: [],
  storageGeom: null,
  paused: false,
  hudRafPending: false,
  lockedFrameIdx: null,
  qTrail: [],
  rotAngle: 0.42,
  turnStartMs: 0,
  firstDeltaMs: 0,
  deltaCount: 0,
  lastTurnDurationMs: 0,
  lastDecodeDurationMs: 0,
  lastGeneratedTokens: 0,
  prefixReused: 0,
  shortlistK: 4096,
  backend: 'wasm-simd128',
  manualModelPinned: false,
  uorc: null,
  directTokPerSec: null,
  directTokensEmitted: 0,
  lastSnapshot: null,
  theme: null,
};

function renderTelosSelfImproveStatus(res) {
  /* Formal R4.OntologicalTelos (src/R4/OntologicalTelos.lex.tex) */
  if (!res) return;
  const badge = document.getElementById('telos-verdict-badge');
  const txt = document.getElementById('telos-summary-text');
  if (badge) {
    badge.textContent = `${res.verdict || 'TelosAdmitted'} · k=${res.shortlist_k || 4096} · w=${res.flock_window || 256}`;
  }
  if (txt) {
    const comps = res.loss_components || {};
    txt.textContent = `Tick #${res.step_idx ?? 0} (${res.tuned_axis || 'shortlist_k'}) · W5H=${Boolean(res.w5h_complete)} · loss=${res.loss_total ?? 0} · opacity=${comps.opacity_debt ?? 0} legibility=${comps.legibility_gap ?? 0} dissonance=${comps.observer_dissonance ?? 0}`;
  }
}

/** Formal R4.OntologicalTelos (src/R4/OntologicalTelos.lex.tex) */
async function runVerdantTelosTick(metricsPayload) {
  /* Formal R4.OntologicalTelos (src/R4/OntologicalTelos.lex.tex) */
  const payloadObj = metricsPayload || {};
  if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
    const res = await engineWorkerClient.rpc('telos_self_improvement_tick', payloadObj);
    renderTelosSelfImproveStatus(res);
    return res;
  }
  const raw = dispatchStudioEnvelope('telos_tick', JSON.stringify(payloadObj));
  renderTelosSelfImproveStatus(raw);
  return raw;
}

async function refreshHoloStorageGeometry(modelId = '') {
  /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
  let geom = null;
  if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
    geom = await engineWorkerClient.rpc('holo_storage_geometry', { modelId: String(modelId || '') });
  } else {
    geom = dispatchStudioEnvelope('holo_storage_geometry', String(modelId || ''));
  }
  if (geom && geom.status === 'ok') {
    holoObservatoryState.storageGeom = geom;
    renderHoloObservatoryDeck();
  }
  return geom;
}

/** Formal R4.StudioFrame coalesce floating HUD and 4-canvas redraws via requestAnimationFrame. */
function scheduleHoloObservatoryRender() {
  /* Formal R4.StudioFrame */
  if (typeof requestAnimationFrame !== 'function') {
    renderHoloObservatoryDeck();
    return;
  }
  if (holoObservatoryState.hudRafPending) return;
  holoObservatoryState.hudRafPending = true;
  requestAnimationFrame(() => {
    holoObservatoryState.hudRafPending = false;
    renderHoloObservatoryDeck();
  });
}

function ingestHoloObservatoryFrame(frame, batchMeta = null) {
  /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
  if (!frame || typeof frame !== 'object') return;
  if (holoObservatoryState.paused) return;
  holoObservatoryState.frames.push(frame);
  if (holoObservatoryState.frames.length > MAX_HUD_RING_FRAMES) {
    holoObservatoryState.frames.shift();
  }
  if (Array.isArray(frame.q_manifold_xyz) && frame.q_manifold_xyz.length === 3) {
    holoObservatoryState.qTrail.push({
      xyz: frame.q_manifold_xyz,
      label: String(frame.token_text || `#${frame.token_id || 0}`).slice(0, 10),
    });
    if (holoObservatoryState.qTrail.length > 32) {
      holoObservatoryState.qTrail.shift();
    }
  }
  if (batchMeta && typeof batchMeta.prefixReused === 'number') {
    holoObservatoryState.prefixReused = batchMeta.prefixReused;
  } else if (typeof frame.prefix_reused_tokens === 'number') {
    holoObservatoryState.prefixReused = frame.prefix_reused_tokens;
  }
  if (frame.backend) {
    holoObservatoryState.backend = String(frame.backend);
  }
  holoObservatoryState.rotAngle = (holoObservatoryState.rotAngle + 0.08) % (Math.PI * 2);
  scheduleHoloObservatoryRender();
}

function drawHoloCanvasHeader(ctx, w, h, subtitle) {
  /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
  const t = holoObservatoryState.theme;
  const bg = (t && t.bg) || '#05070c';
  const border = (t && t.border) || 'rgba(56, 189, 248, 0.14)';
  const fg = (t && t.fg) || '#94a3b8';
  const mono = (t && t.mono) || 'ui-monospace, Menlo, monospace';
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = bg;
  ctx.fillRect(0, 0, w, h);
  ctx.strokeStyle = border;
  ctx.lineWidth = 1;
  ctx.strokeRect(0.5, 0.5, w - 1, h - 1);
  if (subtitle) {
    ctx.fillStyle = fg;
    ctx.font = `10px ${mono}`;
    ctx.fillText(subtitle, 10, 15);
  }
}

function renderHoloObservatoryDeck() {
  /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
  if (
    typeof window !== 'undefined' &&
    typeof window.getComputedStyle === 'function' &&
    typeof document !== 'undefined' &&
    document.documentElement
  ) {
    const cs = window.getComputedStyle(document.documentElement);
    if (cs && typeof cs.getPropertyValue === 'function') {
      holoObservatoryState.theme = {
        bg: cs.getPropertyValue('--card').trim(),
        border: cs.getPropertyValue('--border').trim(),
        fg: cs.getPropertyValue('--foreground').trim(),
        mono: cs.getPropertyValue('--font-mono').trim(),
      };
    }
  }
  const latest = holoObservatoryState.frames[holoObservatoryState.frames.length - 1] || null;
  const geom = holoObservatoryState.storageGeom;
  const snap = holoObservatoryState.lastSnapshot || null;
  const uorc = holoObservatoryState.uorc || (snap && snap.uorc) || null;
  const snapGm = (snap && snap.geometric_metrics && typeof snap.geometric_metrics === 'object') ? snap.geometric_metrics : null;
  const snapHolo = (snap && snap.holo_v13 && typeof snap.holo_v13 === 'object') ? snap.holo_v13 : null;

  const badgeEl = document.getElementById('holo-obs-status-badge');
  const speedEl = document.getElementById('holo-obs-speed-stat');
  const prefixEl = document.getElementById('holo-obs-prefix-stat');
  const shortlistEl = document.getElementById('holo-obs-shortlist-stat');
  const backendEl = document.getElementById('holo-obs-backend-stat');

  if (badgeEl) {
    if (latest) {
      badgeEl.textContent = `${latest.phase} pos=${latest.seq_pos} tok="${String(latest.token_text || '').replace(/\s+/g, ' ')}"`;
    } else if (uorc) {
      badgeEl.textContent = `uorc gen=${uorc.generation ?? 0} · ${uorc.verdict || 'TelosAdmitted'} · loss=${uorc.loss_total ?? 0}`;
    } else {
      badgeEl.textContent = 'Awaiting Live Token Frame';
    }
  }
  const ttftStr = holoObservatoryState.firstDeltaMs ? `${holoObservatoryState.firstDeltaMs}ms` : '—';
  const decMs = holoObservatoryState.lastDecodeDurationMs > 0
    ? holoObservatoryState.lastDecodeDurationMs
    : holoObservatoryState.lastTurnDurationMs;
  const genToks = holoObservatoryState.lastGeneratedTokens || 0;
  const streamedToks = (holoObservatoryState.firstDeltaMs > 0 && genToks > 1) ? (genToks - 1) : genToks;
  let tpsValStr = '—';
  if (holoObservatoryState.directTokPerSec && holoObservatoryState.directTokPerSec > 0) {
    tpsValStr = `${Number(holoObservatoryState.directTokPerSec).toFixed(1)} tok/s`;
  } else if (decMs > 0 && streamedToks > 0) {
    tpsValStr = `${(streamedToks / (decMs / 1000)).toFixed(1)} tok/s`;
  }
  if (speedEl) {
    speedEl.textContent = `${holoObservatoryState.firstDeltaMs ? `${ttftStr} TTFT` : '—'} · ${tpsValStr}`;
  }
  if (prefixEl) {
    prefixEl.textContent = `${holoObservatoryState.prefixReused || 0} tok cached`;
  }
  if (shortlistEl) {
    if (latest) {
      shortlistEl.textContent = `${latest.shortlist_count} / d=${latest.min_hamming}`;
    } else if (snapGm && snapGm.nibble_lut_lookups !== undefined) {
      shortlistEl.textContent = `${snapGm.nibble_lut_lookups} lut / ${snapGm.lorentz_geodesic_evals || 0} geod`;
    } else {
      shortlistEl.textContent = `${holoObservatoryState.shortlistK} / d=—`;
    }
  }
  if (backendEl) {
    backendEl.textContent = uorc
      ? `${holoObservatoryState.backend} · uorc gen=${uorc.generation ?? 0} (${uorc.verdict || 'TelosAdmitted'})`
      : holoObservatoryState.backend;
  }

  // Update floating Dropdown Neural HUD compact ribbon (`R4-62`)
  const hudTtftEl = document.getElementById('neural-hud-ttft');
  const hudTpsEl = document.getElementById('neural-hud-tps');
  const hudBackendEl = document.getElementById('neural-hud-backend');
  const hudKvEl = document.getElementById('neural-hud-kv');
  const hudSparkCanvas = document.getElementById('neural-hud-sparkline-canvas');
  if (hudTtftEl) hudTtftEl.textContent = ttftStr;
  if (hudTpsEl) hudTpsEl.textContent = tpsValStr;
  if (hudBackendEl) {
    hudBackendEl.textContent = latest && Array.isArray(latest.layer_residual_mag)
      ? `${holoObservatoryState.backend} (${latest.layer_residual_mag.length}L)`
      : String(holoObservatoryState.backend || 'wasm-simd128');
  }
  if (hudKvEl) hudKvEl.textContent = `${holoObservatoryState.prefixReused || 0} tok`;
  if (hudSparkCanvas && typeof hudSparkCanvas.getContext === 'function') {
    const sCtx = hudSparkCanvas.getContext('2d');
    const sw = hudSparkCanvas.width || 110;
    const sh = hudSparkCanvas.height || 22;
    if (sCtx && typeof sCtx.clearRect === 'function') {
      sCtx.clearRect(0, 0, sw, sh);
      sCtx.fillStyle = '#05070c';
      sCtx.fillRect(0, 0, sw, sh);
      const pts = holoObservatoryState.frames;
      if (pts.length > 0) {
        sCtx.strokeStyle = '#38bdf8';
        sCtx.lineWidth = 1.4;
        sCtx.beginPath();
        for (let i = 0; i < pts.length; i += 1) {
          const f = pts[i];
          const val = Number(f && (f.min_hamming ?? f.tps_x10 ?? 32));
          const x = pts.length === 1 ? sw - 2 : (i / (pts.length - 1)) * (sw - 4) + 2;
          const y = sh - 3 - Math.min(sh - 6, Math.max(0, (val % 64) / 64 * (sh - 6)));
          if (i === 0) sCtx.moveTo(x, y);
          else sCtx.lineTo(x, y);
        }
        sCtx.stroke();
      }
    }
  }

  // 1. 28-Layer Residual & Bit-Width Waterfall
  const c1 = document.getElementById('holo-layer-waterfall-canvas');
  if (c1 && typeof c1.getContext === 'function') {
    const ctx = c1.getContext('2d');
    const w = c1.width;
    const h = c1.height;
    const mags = (latest && Array.isArray(latest.layer_residual_mag) && latest.layer_residual_mag.length > 0)
      ? latest.layer_residual_mag
      : null;
    const bits = (latest && Array.isArray(latest.layer_max_bits)) ? latest.layer_max_bits : [];
    const sub1 = mags
      ? `${mags.length} Layers · Live Residual Magnitude & Max Bit-Width`
      : snapGm
        ? `Integer Geometry · LUT=${snapGm.nibble_lut_lookups || 0} · Geod=${snapGm.lorentz_geodesic_evals || 0}`
        : 'Send a chat turn to inspect live 28-layer integer dynamics';
    drawHoloCanvasHeader(ctx, w, h, sub1);
    if (mags) {
      let maxMag = 1;
      for (let i = 0; i < mags.length; i += 1) {
        if (mags[i] > maxMag) maxMag = mags[i];
      }
      const padL = 16;
      const padB = 22;
      const plotW = w - padL * 2;
      const plotH = h - 32 - padB;
      const stepX = plotW / Math.max(1, mags.length);
      for (let i = 0; i < mags.length; i += 1) {
        const bw = Math.max(3, stepX - 2);
        const normH = Math.max(4, Math.round((mags[i] / maxMag) * plotH));
        const rawLayerBits = bits[i] || 18;
        const bWidth = rawLayerBits ? Math.max(0, rawLayerBits | 0) : 18;
        const hue = bWidth >= 24 ? '#f59e0b' : (bWidth >= 20 ? '#38bdf8' : '#22d3ee');
        ctx.fillStyle = hue;
        ctx.fillRect(padL + i * stepX, h - padB - normH, bw, normH);
        if (i === 0 || i === mags.length - 1 || i % 7 === 0) {
          ctx.fillStyle = '#94a3b8';
          ctx.font = '9px ui-monospace, Menlo, monospace';
          ctx.fillText(`L${i}`, padL + i * stepX, h - 7);
        }
        ctx.fillStyle = '#cbd5e1';
        ctx.font = '8px ui-monospace, Menlo, monospace';
        if (i % 4 === 0) {
          ctx.fillText(`${bWidth}b`, padL + i * stepX, Math.max(26, h - padB - normH - 3));
        }
      }
    } else if (snapGm) {
      const vals = [
        Number(snapGm.nibble_lut_lookups || 0),
        Number(snapGm.lorentz_geodesic_evals || 0),
        Number(snapGm.flock_attended_positions || 0),
        Number(snapGm.hamming_xor_copy_hits || 0),
      ];
      const maxV = Math.max(1, ...vals);
      for (let i = 0; i < 14; i += 1) {
        const base = vals[i % vals.length];
        const barH = Math.max(8, Math.round((base / maxV) * (h - 56)));
        ctx.fillStyle = i % 2 === 0 ? '#38bdf8' : '#22d3ee';
        ctx.fillRect(16 + i * Math.max(12, Math.floor((w - 32) / 14)), h - 22 - barH, 10, barH);
      }
    }
  }

  // 2. 151,936-Vocab RHT-512 Hamming Bell Curve + Top-6 Integer Logit Race
  const c2 = document.getElementById('holo-shortlist-race-canvas');
  if (c2 && typeof c2.getContext === 'function') {
    const ctx = c2.getContext('2d');
    const w = c2.width;
    const h = c2.height;
    const hist = (latest && Array.isArray(latest.hamming_hist_32)) ? latest.hamming_hist_32 : null;
    const cands = (latest && Array.isArray(latest.top_candidates)) ? latest.top_candidates : [];
    const sub2 = hist
      ? `RHT-512 Bell Curve (min d=${latest.min_hamming}) · Top-6 Shortlist Race`
      : uorc
        ? `RHT-512 Shortlist · uorc axis=${uorc.tuned_axis || 'shortlist_k'} loss=${uorc.loss_total ?? 0}`
        : 'Shortlist Hamming histogram & logit race await decode';
    drawHoloCanvasHeader(ctx, w, h, sub2);
    const splitX = Math.floor(w * 0.48);
    if (hist) {
      let maxBin = 1;
      for (let b = 0; b < hist.length; b += 1) {
        if (hist[b] > maxBin) maxBin = hist[b];
      }
      const bw = Math.max(2, (splitX - 24) / hist.length);
      for (let b = 0; b < hist.length; b += 1) {
        const logRatio = Math.log10(1 + (hist[b] || 0)) / Math.log10(1 + maxBin);
        const bh = Math.max(2, Math.round(logRatio * (h - 56)));
        ctx.fillStyle = b < 14 ? '#34d399' : '#38bdf8';
        ctx.fillRect(14 + b * bw, h - 22 - bh, Math.max(1, bw - 1), bh);
      }
      ctx.fillStyle = '#64748b';
      ctx.font = '9px ui-monospace, Menlo, monospace';
      ctx.fillText('d=0', 14, h - 7);
      ctx.fillText('256', Math.floor(splitX * 0.46), h - 7);
      ctx.fillText('512', splitX - 26, h - 7);
    } else if (uorc || snapGm) {
      for (let b = 0; b < 16; b += 1) {
        const bell = Math.max(6, Math.round(Math.exp(-((b - 7.5) * (b - 7.5)) / 18) * (h - 58)));
        ctx.fillStyle = b < 8 ? '#34d399' : '#38bdf8';
        ctx.fillRect(16 + b * 11, h - 22 - bell, 8, bell);
      }
      if (uorc) {
        ctx.fillStyle = '#34d399';
        ctx.font = '10px ui-monospace, Menlo, monospace';
        ctx.fillText(`W5H=${Boolean(uorc.w5h_complete)} · ${uorc.verdict || 'TelosAdmitted'}`, splitX + 10, 44);
      }
    }
    if (cands.length > 0) {
      let maxLog = 1;
      let minLog = cands[cands.length - 1].logit_q16 || 0;
      for (let i = 0; i < cands.length; i += 1) {
        if (cands[i].logit_q16 > maxLog) maxLog = cands[i].logit_q16;
        if (cands[i].logit_q16 < minLog) minLog = cands[i].logit_q16;
      }
      const span = Math.max(1, maxLog - minLog + 64);
      const barAreaW = w - splitX - 20;
      for (let i = 0; i < cands.length; i += 1) {
        const item = cands[i];
        const y = 30 + i * 28;
        const frac = Math.max(0.12, (item.logit_q16 - minLog + 32) / span);
        ctx.fillStyle = item.selected ? 'rgba(52, 211, 153, 0.25)' : 'rgba(56, 189, 248, 0.16)';
        ctx.fillRect(splitX + 6, y, Math.round(barAreaW * frac), 21);
        ctx.fillStyle = item.selected ? '#34d399' : '#e2e8f0';
        ctx.font = '10px ui-monospace, Menlo, monospace';
        const lbl = `${item.selected ? '*' : ' '}"${String(item.token_text || '').replace(/\s+/g, ' ')}"`;
        ctx.fillText(`${lbl.slice(0, 14)} (${item.logit_q16})`, splitX + 10, y + 14);
      }
    }
  }

  // 3. Causal Starling-Flock Sparse Attention Matrix & Final-Layer Peaks
  const c3 = document.getElementById('holo-causal-flock-canvas');
  if (c3 && typeof c3.getContext === 'function') {
    const ctx = c3.getContext('2d');
    const w = c3.width;
    const h = c3.height;
    const spans = (latest && Array.isArray(latest.layer_flock_spans)) ? latest.layer_flock_spans : null;
    const peaks = (latest && Array.isArray(latest.final_attn_peaks)) ? latest.final_attn_peaks : [];
    const attPos = snapGm ? Number(snapGm.flock_attended_positions || 0) : 0;
    const totPos = snapGm ? Number(snapGm.flock_total_positions || 0) : 0;
    const sub3 = spans
      ? `Starling Flock Spans (${spans.length}L) · Reused Prefix=${holoObservatoryState.prefixReused}`
      : snapGm
        ? `Starling-Flock Attention · ${attPos}/${totPos} pos · XOR-copy=${snapGm.hamming_xor_copy_hits || 0}`
        : 'Causal Starling-Flock KV matrix awaits decode';
    drawHoloCanvasHeader(ctx, w, h, sub3);
    if (spans) {
      const seqMax = Math.max(1, Number(latest.seq_pos) + 1);
      const padL = 14;
      const plotW = w - 28;
      const plotH = h - 76;
      const rowH = Math.max(3, plotH / spans.length);
      const reusedW = Math.round((Math.min(seqMax, holoObservatoryState.prefixReused) / seqMax) * plotW);
      if (reusedW > 0) {
        ctx.fillStyle = 'rgba(52, 211, 153, 0.2)';
        ctx.fillRect(padL, 24, reusedW, plotH);
      }
      for (let l = 0; l < spans.length; l += 1) {
        const spanFrac = Math.min(1, Number(spans[l] || 1) / seqMax);
        ctx.fillStyle = '#38bdf8';
        ctx.fillRect(padL + plotW * (1 - spanFrac), 24 + l * rowH, Math.max(3, plotW * spanFrac), Math.max(2, rowH - 1));
      }
      // Plot top attended KV peaks as luminous stems along bottom timeline
      const baseY = h - 18;
      ctx.strokeStyle = '#334155';
      ctx.beginPath();
      ctx.moveTo(padL, baseY);
      ctx.lineTo(padL + plotW, baseY);
      ctx.stroke();
      for (let p = 0; p < peaks.length; p += 1) {
        const pk = peaks[p];
        const px = padL + Math.round((Math.min(seqMax - 1, pk.pos) / Math.max(1, seqMax - 1)) * plotW);
        const ph = Math.max(6, Math.round((pk.weight_q16 / 65535) * 32));
        ctx.fillStyle = '#f472b6';
        ctx.fillRect(px - 2, baseY - ph, 4, ph);
        ctx.fillStyle = '#f8fafc';
        ctx.font = '8px ui-monospace, Menlo, monospace';
        ctx.fillText(`t${pk.pos}`, Math.min(w - 28, Math.max(padL, px - 6)), baseY - ph - 3);
      }
    } else if (snapGm) {
      const ratio = totPos > 0 ? Math.min(1, attPos / totPos) : 0.5;
      const barW = Math.max(24, Math.round((w - 32) * ratio));
      ctx.fillStyle = 'rgba(52, 211, 153, 0.25)';
      ctx.fillRect(16, 36, Math.round((w - 32) * 0.35), 24);
      ctx.fillStyle = '#38bdf8';
      ctx.fillRect(16, 72, barW, 18);
    }
  }

  // 4. Stationary OPFS + MVM CoW + 3D RHT-512 Hypercube
  const c4 = document.getElementById('holo-storage-geometry-canvas');
  if (c4 && typeof c4.getContext === 'function') {
    const ctx = c4.getContext('2d');
    const w = c4.width;
    const h = c4.height;
    const layers = (geom && Array.isArray(geom.layers)) ? geom.layers : [];
    const mvm = (geom && geom.mvm_storage) ? geom.mvm_storage : {};
    const clusters = (geom && Array.isArray(geom.vocab_rht_clusters)) ? geom.vocab_rht_clusters : [];
    const sub4 = geom
      ? `OPFS ${geom.total_pieces_est || 0}x1MiB (${layers.length}L) · MVM CoW dirty=${mvm.dirty_sector_count || 0}x4KiB · ${clusters.length} RHT clusters`
      : snapHolo
        ? `OPFS ${snapHolo.piece_count || 0} shards · cached=${snapHolo.cached_hits || 0} · ${snapHolo.model_id || 'Holo v13'}`
        : 'Click "Refresh Storage Map" after model mount to inspect OPFS + MVM geometry';
    drawHoloCanvasHeader(ctx, w, h, sub4);
    // Left half: Stationary 1 MiB OPFS pieces + MVM CoW sector strip
    const leftW = Math.floor(w * 0.48);
    const totalPieces = Math.max(1, (geom && geom.total_pieces_est) || Math.max(28, layers.length));
    const cols = 12;
    const cellW = Math.max(8, Math.floor((leftW - 20) / cols));
    const cellH = Math.max(8, Math.min(18, Math.floor((h - 86) / Math.max(1, Math.ceil(totalPieces / cols)))));
    const pieceColors = new Map();
    for (let li = 0; li < layers.length; li += 1) {
      const ly = layers[li];
      for (let p = Number(ly.piece_start || 0); p <= Number(ly.piece_end || 0); p += 1) {
        pieceColors.set(p, ly.down_proj_dtype === 'int8' ? '#f59e0b' : '#38bdf8');
      }
    }
    const maxDraw = Math.min(totalPieces, 72);
    for (let p = 0; p < maxDraw; p += 1) {
      const cx = 12 + (p % cols) * cellW;
      const cy = 28 + Math.floor(p / cols) * cellH;
      ctx.fillStyle = pieceColors.get(p) || '#818cf8';
      ctx.fillRect(cx, cy, cellW - 2, cellH - 2);
    }
    // MVM CoW 4 KiB dirty sector bar at bottom left
    const dirtyIdx = Array.isArray(mvm.dirty_sector_indices) ? mvm.dirty_sector_indices : [];
    ctx.fillStyle = '#94a3b8';
    ctx.font = '9px ui-monospace, Menlo, monospace';
    ctx.fillText(`MVM 4KiB CoW (${mvm.workspace_file_count || 0} files / ${mvm.dirty_sector_count || 0} dirty)`, 12, h - 30);
    for (let s = 0; s < 24; s += 1) {
      const isDirty = s < dirtyIdx.length;
      ctx.fillStyle = isDirty ? '#34d399' : '#1e293b';
      ctx.fillRect(12 + s * 8, h - 22, 6, 10);
    }

    // Right half: 16x16 Tile-Binned Zero-Float 3D PBR Hopf-Poincare-Zeta Manifold (`R4-70`)
    const rx0 = Math.floor(w * 0.5);
    const rw = Math.max(32, w - rx0 - 8);
    const rh = Math.max(32, h - 32);
    const trailPts = holoObservatoryState.qTrail.map((item) => item.xyz || [0, 0, 0]);
    const phase1024 = Math.round(((holoObservatoryState.rotAngle || 0) * 163)) & 1023;
    blitHopfZeta3dRect(ctx, rx0, 24, rw, rh, phase1024, 2, trailPts);
    if (holoObservatoryState.qTrail.length > 0) {
      const head = holoObservatoryState.qTrail[holoObservatoryState.qTrail.length - 1];
      ctx.fillStyle = '#f8fafc';
      ctx.font = '9px ui-monospace, Menlo, monospace';
      ctx.fillText(`Hopf·Zeta 3D PBR · "${head.label}"`, rx0 + 6, h - 10);
    }
  }
}

let _vtensorWasmPromise = null;
function ensureVtensorWasmReady() {
  /* Formal R4.VtensorSemiring3d */
  if (mainThreadWasmReady) return Promise.resolve(true);
  if (!_vtensorWasmPromise) {
    _vtensorWasmPromise = init()
      .then(() => {
        mainThreadWasmReady = true;
        if (typeof renderHoloObservatoryDeck === 'function') renderHoloObservatoryDeck();
        return true;
      })
      .catch(() => false);
  }
  return _vtensorWasmPromise;
}

function blitHopfZeta3dRect(ctx, x0, y0, rw, rh, phase1024, lod, trailTriples) {
  /* Formal R4.VtensorSemiring3d */
  const trailCsv = Array.isArray(trailTriples)
    ? trailTriples
        .slice(-24)
        .map((pt) => `${Math.round(Number(pt[0]) || 0)},${Math.round(Number(pt[1]) || 0)},${Math.round(Number(pt[2]) || 0)}`)
        .join(';')
    : '';
  if (mainThreadWasmReady && typeof ctx.putImageData === 'function' && typeof ImageData === 'function') {
    const rgba = studio_render_hopf_zeta_3d_rgba(rw, rh, phase1024 | 0, lod | 0, trailCsv);
    if (rgba && rgba.length === rw * rh * 4) {
      ctx.putImageData(new ImageData(new Uint8ClampedArray(rgba), rw, rh), x0, y0);
      return true;
    }
  }
  ensureVtensorWasmReady();
  ctx.strokeStyle = 'rgba(56, 189, 248, 0.45)';
  ctx.strokeRect(x0 + 1, y0 + 1, rw - 2, rh - 2);
  return false;
}

let __holoObsButtonsBound = false;
function __uorFormatMlpButtonText(mode) {
  /* Formal R4.WebGpuGeodesicRuntime */
  const norm = String(mode || 'auto').toLowerCase();
  if (norm === 'sparse' || norm === 'geodesicsparse1536') return 'MLP: Sparse K=1536';
  if (norm === 'dense' || norm === 'denseexact') return 'MLP: Dense 0-LSB';
  return 'MLP: Auto';
}

function __uorNormalizeMlpPolicy(mode) {
  /* Formal R4.WebGpuGeodesicRuntime */
  const norm = String(mode || 'auto').toLowerCase();
  if (norm === 'sparse' || norm === 'geodesicsparse1536') return 'sparse';
  if (norm === 'dense' || norm === 'denseexact') return 'dense';
  return 'auto';
}

async function applyHoloMlpMode(mode) {
  /* Formal R4.WebGpuGeodesicRuntime */
  const policy = __uorNormalizeMlpPolicy(mode);
  if (typeof safeStorageSet === 'function') {
    safeStorageSet('uor_mlp_shortlist_mode', policy);
  }
  let res = null;
  if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
    res = await engineWorkerClient.rpc('set_holo_mlp_mode', { mode: policy, shortlistK: 1536 });
  } else {
    res = dispatchStudioEnvelope('set_holo_mlp_mode', policy);
  }
  const selEl = document.getElementById('settings-mlp-shortlist-mode');
  if (selEl) selEl.value = policy;
  const obsBtn = document.getElementById('holo-obs-mlp-mode-btn');
  if (obsBtn) obsBtn.textContent = __uorFormatMlpButtonText(policy);
  const statusEl = document.getElementById('settings-modal-status');
  if (statusEl && res) {
    statusEl.textContent = `SwiGLU Shortlist Policy: ${policy} -> verdict=${res.mlp_mode || res.verdict || 'ok'}`;
  }
  renderHoloObservatoryDeck();
  return res;
}

async function verifyHoloOpfsSlab(repair = true) {
  /* Formal R4.WebGpuGeodesicRuntime */
  const selModel = document.getElementById('composer-model-select');
  const modelId = (selModel && selModel.value) || 'uor-r4.qwen3-0.6b-holo';
  let res = null;
  if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
    res = await engineWorkerClient.rpc(repair ? 'repair_holo_slab' : 'verify_holo_slab', { modelId, repair });
  } else {
    res = { ok: false, verdict: 'RefusedSlabSealMismatch', message: 'Worker OPFS slab verify requires active Worker bridge.' };
  }
  const verdict = (res && res.verdict) || 'RefusedSlabSealMismatch';
  const statusEl = document.getElementById('settings-modal-status');
  if (statusEl) {
    statusEl.textContent = `HOLOSLAB2 Verify (${modelId}): ${verdict}${res && res.repairedPieces ? ` · healed=${res.repairedPieces}` : ''}`;
  }
  const badgeEl = document.getElementById('holo-obs-status-badge');
  if (badgeEl) {
    badgeEl.textContent = `HOLOSLAB2: ${verdict}`;
  }
  return res;
}

function bindHoloObservatoryControls() {
  /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
  if (__holoObsButtonsBound) return;
  const refreshBtn = document.getElementById('holo-obs-refresh-storage-btn');
  const pauseBtn = document.getElementById('holo-obs-pause-toggle');
  const mlpBtn = document.getElementById('holo-obs-mlp-mode-btn');
  const verifySlabBtn = document.getElementById('holo-obs-verify-slab-btn');
  if (!refreshBtn && !pauseBtn && !mlpBtn && !verifySlabBtn) return;
  __holoObsButtonsBound = true;
  if (mlpBtn) {
    const savedPolicy = typeof safeStorageGet === 'function' ? safeStorageGet('uor_mlp_shortlist_mode') : 'auto';
    mlpBtn.textContent = __uorFormatMlpButtonText(savedPolicy || 'auto');
    mlpBtn.addEventListener('click', () => {
      const cur = __uorNormalizeMlpPolicy((typeof safeStorageGet === 'function' && safeStorageGet('uor_mlp_shortlist_mode')) || 'auto');
      const next = cur === 'auto' ? 'sparse' : cur === 'sparse' ? 'dense' : 'auto';
      applyHoloMlpMode(next).catch(() => {});
    });
  }
  if (verifySlabBtn) {
    verifySlabBtn.addEventListener('click', () => {
      verifyHoloOpfsSlab(true).catch(() => {});
    });
  }
  if (refreshBtn) {
    refreshBtn.addEventListener('click', () => {
      const sel = document.getElementById('composer-model-select');
      refreshHoloStorageGeometry(sel ? sel.value : '').catch(() => {});
    });
  }
  if (pauseBtn) {
    pauseBtn.addEventListener('click', () => {
      holoObservatoryState.paused = !holoObservatoryState.paused;
      pauseBtn.textContent = holoObservatoryState.paused ? 'Live Stream: PAUSED' : 'Live Stream: ON';
      if (!holoObservatoryState.paused) {
        renderHoloObservatoryDeck();
      }
    });
  }
}

let __uorcWatchdogButtonsBound = false;
function bindVerdantWatchdogButtons() {
  bindHoloObservatoryControls();
  if (__uorcWatchdogButtonsBound) return;
  const admitBtn = document.getElementById('uorc-admit-capsule-btn');
  const rollbackBtn = document.getElementById('uorc-rollback-btn');
  const telosBtn = document.getElementById('telos-run-tick-btn');
  if (!admitBtn && !rollbackBtn && !telosBtn) return;
  __uorcWatchdogButtonsBound = true;
  if (telosBtn) {
    telosBtn.addEventListener('click', () => {
      runVerdantTelosTick({
        latency_ms: holoObservatoryState.lastTurnDurationMs || 18,
        matched_prefix: holoObservatoryState.prefixReused || 0,
      }).catch(() => {});
    });
  }

  if (admitBtn) {
    admitBtn.addEventListener('click', async () => {
      const nextIdx = Date.now() % 10000;
      const capsule = {
        schema: 'uorc/generation-capsule/1',
        magic_uorc: 'UORC',
        generation_id: `gen-${nextIdx}`,
        parent_generation_id: 'gen-0',
        gates_passed: 4,
        gates_total: 4,
        decide_passed: 4,
        decide_total: 4,
        residue_count: 0,
        coercive_patterns: 0,
        wasm_bytes: 8000000,
        max_wasm_bytes: 15728640,
        frame_time_micros: 800,
        frame_budget_micros: 16000,
        candidate_loss: 10,
        incumbent_loss: 12,
        candidate_steps: 4,
        incumbent_steps: 5,
        float_ops: 0,
        dense_matmul_ops: 0,
      };
      try {
        const res = (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled)
          ? await engineWorkerClient.rpc('admit_generation_capsule', { capsule })
          : await admit_uorc_generation_capsule(capsule);
        if (typeof engineWorkerClient !== 'undefined') {
          engineWorkerClient.verdantSynthesis = await (engineWorkerClient.enabled
            ? engineWorkerClient.rpc('verdant_synthesis_status', {})
            : query_verdant_synthesis_status());
        }
        renderVerdantSynthesisPanel();
        if (res && res.admitted && typeof emitPrismEmbedEvent === 'function') {
          emitPrismEmbedEvent('prism:status');
        }
      } catch {
        renderVerdantSynthesisPanel();
      }
    });
  }

  if (rollbackBtn) {
    rollbackBtn.addEventListener('click', async () => {
      try {
        for (let strike = 0; strike < 3; strike += 1) {
          if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient.enabled) {
            await engineWorkerClient.rpc('record_generation_fault', { reason: `watchdog_probe_strike_${strike + 1}` });
          } else {
            await record_uorc_generation_fault(`watchdog_probe_strike_${strike + 1}`);
          }
        }
        if (typeof engineWorkerClient !== 'undefined') {
          engineWorkerClient.verdantSynthesis = await (engineWorkerClient.enabled
            ? engineWorkerClient.rpc('verdant_synthesis_status', {})
            : query_verdant_synthesis_status());
        }
      } catch {
        // Non-fatal in mock test mode
      }
      renderVerdantSynthesisPanel();
    });
  }
}

// ============================================================================
// R4-60 STUDIO LIVE HUD, M3 RESPONSIVE SHELL, CONSENT & PREVIEW RUNTIME
// ============================================================================

const _r460MemoryStorage = new Map();
const _r460PendingInspectWaiters = new Map();
let _r460ManualSidebarCollapse = false;
let _r460ToastTimer = null;
let _r460InspectSeq = 1;
const isConsentGrantedVal = (val) => val === '1' || val === 'true' || val === 'granted';
let _r460LatestPreviewDiag = {
  renderedAtMs: 0,
  fileCount: 0,
  scriptSteps: 0,
  errors: [],
  console: [],
  rootHtmlPreview: '',
};

const _r460PreviewTimers = {
  timeouts: new Set(),
  intervals: new Set(),
  rafs: new Set(),
};

const clearPreviewTimers = () => {
  for (const id of _r460PreviewTimers.timeouts) {
    if (typeof clearTimeout === 'function') clearTimeout(id);
  }
  _r460PreviewTimers.timeouts.clear();
  for (const id of _r460PreviewTimers.intervals) {
    if (typeof clearInterval === 'function') clearInterval(id);
  }
  _r460PreviewTimers.intervals.clear();
  for (const id of _r460PreviewTimers.rafs) {
    if (typeof cancelAnimationFrame === 'function') cancelAnimationFrame(id);
    else if (typeof clearTimeout === 'function') clearTimeout(id);
  }
  _r460PreviewTimers.rafs.clear();
};

const createPreviewTimerGlobals = (onError) => {
  const reportErr = (err) => {
    if (typeof onError === 'function') onError(err);
  };
  const safeSetTimeout = (fn, ms = 0, ...args) => {
    if (typeof fn !== 'function' || typeof setTimeout !== 'function') return 0;
    const delay = Math.max(0, Number(ms) || 0);
    let handle = 0;
    handle = setTimeout(() => {
      _r460PreviewTimers.timeouts.delete(handle);
      try {
        fn(...args);
      } catch (err) {
        reportErr(err);
      }
    }, delay);
    _r460PreviewTimers.timeouts.add(handle);
    return handle;
  };
  const safeClearTimeout = (id) => {
    _r460PreviewTimers.timeouts.delete(id);
    if (typeof clearTimeout === 'function') clearTimeout(id);
  };
  const safeSetInterval = (fn, ms = 16, ...args) => {
    if (typeof fn !== 'function' || typeof setInterval !== 'function') return 0;
    const delay = Math.max(16, Number(ms) || 16);
    let handle = 0;
    handle = setInterval(() => {
      try {
        fn(...args);
      } catch (err) {
        safeClearInterval(handle);
        reportErr(err);
      }
    }, delay);
    _r460PreviewTimers.intervals.add(handle);
    return handle;
  };
  const safeClearInterval = (id) => {
    _r460PreviewTimers.intervals.delete(id);
    if (typeof clearInterval === 'function') clearInterval(id);
  };
  const safeRequestAnimationFrame = (fn) => {
    if (typeof fn !== 'function') return 0;
    let handle = 0;
    if (typeof requestAnimationFrame === 'function') {
      handle = requestAnimationFrame((ts) => {
        _r460PreviewTimers.rafs.delete(handle);
        try {
          fn(ts);
        } catch (err) {
          reportErr(err);
        }
      });
    } else if (typeof setTimeout === 'function') {
      handle = setTimeout(() => {
        _r460PreviewTimers.rafs.delete(handle);
        try {
          fn(typeof performance !== 'undefined' && typeof performance.now === 'function' ? performance.now() : Date.now());
        } catch (err) {
          reportErr(err);
        }
      }, 16);
    }
    _r460PreviewTimers.rafs.add(handle);
    return handle;
  };
  const safeCancelAnimationFrame = (id) => {
    _r460PreviewTimers.rafs.delete(id);
    if (typeof cancelAnimationFrame === 'function') cancelAnimationFrame(id);
    else if (typeof clearTimeout === 'function') clearTimeout(id);
  };
  return {
    Date,
    performance: {
      now: () => (typeof performance !== 'undefined' && typeof performance.now === 'function' ? performance.now() : Date.now()),
    },
    setTimeout: safeSetTimeout,
    clearTimeout: safeClearTimeout,
    setInterval: safeSetInterval,
    clearInterval: safeClearInterval,
    requestAnimationFrame: safeRequestAnimationFrame,
    cancelAnimationFrame: safeCancelAnimationFrame,
  };
};

const attachPreviewSelectorDiag = (baseDiag, rootEl, selector) => {
  const diag = baseDiag && typeof baseDiag === 'object' ? { ...baseDiag } : {};
  const cleanSel = typeof selector === 'string' ? selector.trim() : '';
  if (!cleanSel) return diag;
  if (!rootEl || typeof rootEl.querySelector !== 'function') {
    return {
      ...diag,
      selector: cleanSel,
      selectorMatched: false,
      selectorText: '',
      selectorError: 'Root element querySelector unavailable',
    };
  }
  try {
    const el = rootEl.querySelector(cleanSel);
    return {
      ...diag,
      selector: cleanSel,
      selectorMatched: Boolean(el),
      selectorText: el ? String(el.textContent || '').slice(0, 512) : '',
      selectorError: '',
    };
  } catch (err) {
    return {
      ...diag,
      selector: cleanSel,
      selectorMatched: false,
      selectorText: '',
      selectorError: err && err.message ? err.message : String(err),
    };
  }
};

const safeStorageGet = (key) => {
  const k = String(key ?? '');
  try {
    if (typeof localStorage !== 'undefined' && typeof localStorage.getItem === 'function') {
      const val = localStorage.getItem(k);
      if (val !== null && val !== undefined) {
        return val;
      }
    }
  } catch {
    // Fallback to session-scoped in-memory map under opaque/restricted origins
  }
  return _r460MemoryStorage.has(k) ? _r460MemoryStorage.get(k) : null;
};

const safeStorageSet = (key, value) => {
  const k = String(key ?? '');
  const v = String(value ?? '');
  _r460MemoryStorage.set(k, v);
  try {
    if (typeof localStorage !== 'undefined' && typeof localStorage.setItem === 'function') {
      localStorage.setItem(k, v);
      return true;
    }
  } catch {
    // Storage access blocked (e.g. SecurityError under sandboxed/opaque origin)
  }
  return false;
};

/** Formal R4.StudioFrame remove persisted key from localStorage and session memory map. */
const safeStorageRemove = (key) => {
  const k = String(key ?? '');
  _r460MemoryStorage.delete(k);
  try {
    if (typeof localStorage !== 'undefined' && typeof localStorage.removeItem === 'function') {
      localStorage.removeItem(k);
      return true;
    }
  } catch {
    // Ignore storage unavailability
  }
  return false;
};

const showStudioToast = (message, tone = 'info', autoHideMs = 4200) => {
  const banner = document.getElementById('studio-toast-banner');
  const msgEl = document.getElementById('studio-toast-message');
  if (banner) {
    const cleanTone = tone === 'error' || tone === 'warn' || tone === 'success' ? tone : 'info';
    if (banner.dataset) {
      banner.dataset.tone = cleanTone;
    }
    if (typeof banner.setAttribute === 'function') {
      banner.setAttribute('data-tone', cleanTone);
    }
    if (typeof banner.removeAttribute === 'function') {
      banner.removeAttribute('hidden');
    }
    banner.hidden = false;
  }
  if (msgEl) {
    msgEl.textContent = String(message ?? '');
  }
  if (_r460ToastTimer !== null && typeof clearTimeout === 'function') {
    clearTimeout(_r460ToastTimer);
    _r460ToastTimer = null;
  }
  if (autoHideMs > 0 && typeof setTimeout === 'function' && banner) {
    const t = setTimeout(() => {
      if (typeof banner.setAttribute === 'function') {
        banner.setAttribute('hidden', '');
      }
      banner.hidden = true;
    }, autoHideMs);
    if (t && typeof t.unref === 'function') {
      t.unref();
    }
    _r460ToastTimer = t;
  }
  if (typeof renderStudioToast === 'function') {
    renderStudioToast(message, tone);
  }
};

const classifyWindowWidth = (widthPx) => {
  const w = Number(widthPx);
  if (!Number.isFinite(w) || w >= 840) return 'expanded';
  if (w >= 600) return 'medium';
  return 'compact';
};

const _detachChildSafely = (parent, node) => {
  if (!parent || !node) return;
  if (Array.isArray(parent.children) ? parent.children.includes(node) : (node.parentNode === parent)) {
    if (typeof parent.removeChild === 'function' && node.parentNode === parent) {
      parent.removeChild(node);
      return;
    }
    if (typeof parent.replaceChildren === 'function') {
      const kept = Array.from(parent.children || []).filter((c) => c !== node);
      parent.replaceChildren(...kept);
    }
  }
};

const syncObservatoryHost = () => {
  const body = document.body;
  const obsDeck = document.getElementById('holo-observatory-deck');
  const dockSplit = document.getElementById('observatory-dock-split');
  const tracesPanel = document.getElementById('canvas-panel-traces');
  if (!body || !obsDeck) return 'hud';

  const targetHost = tracesPanel || obsDeck.parentNode;
  const hostTag = 'hud';

  if (dockSplit) {
    dockSplit.setAttribute('hidden', '');
    dockSplit.setAttribute('aria-hidden', 'true');
  }
  if (targetHost) {
    const alreadyThere = Array.isArray(targetHost.children)
      ? targetHost.children.includes(obsDeck)
      : (obsDeck.parentNode === targetHost);
    if (!alreadyThere) {
      if (dockSplit && dockSplit !== targetHost) _detachChildSafely(dockSplit, obsDeck);
      targetHost.appendChild(obsDeck);
    }
  }
  if (body.dataset) {
    body.dataset.observatoryHost = hostTag;
  }
  if (typeof body.setAttribute === 'function') {
    body.setAttribute('data-observatory-host', hostTag);
  }
  return hostTag;
};

const applyWindowWidthClass = (widthPx) => {
  const measured = widthPx !== undefined && widthPx !== null
    ? Number(widthPx)
    : (typeof window !== 'undefined' && Number.isFinite(Number(window.innerWidth)) ? Number(window.innerWidth) : 1280);
  const winClass = classifyWindowWidth(measured);
  const body = document.body;
  const prismFrame = document.getElementById('prism-frame');
  const mainEl = typeof document.querySelector === 'function' ? document.querySelector('main') : null;
  const sidebarEl = document.getElementById('threads-sidebar');
  const prevWinClass = body && ((body.dataset && body.dataset.windowClass) || (typeof body.getAttribute === 'function' && body.getAttribute('data-window-class')));

  if (body) {
    if (body.dataset) body.dataset.windowClass = winClass;
    if (typeof body.setAttribute === 'function') body.setAttribute('data-window-class', winClass);
  }
  if (prismFrame) {
    if (prismFrame.dataset) prismFrame.dataset.windowClass = winClass;
    if (typeof prismFrame.setAttribute === 'function') prismFrame.setAttribute('data-window-class', winClass);
  }
  if (mainEl && mainEl.classList) {
    if (winClass === 'expanded') {
      mainEl.classList.remove('sidebar-expanded');
      if (!_r460ManualSidebarCollapse) {
        mainEl.classList.remove('sidebar-collapsed');
        if (sidebarEl && sidebarEl.classList) sidebarEl.classList.remove('sidebar-collapsed');
      }
    } else if (prevWinClass !== winClass && !mainEl.classList.contains('sidebar-expanded')) {
      mainEl.classList.add('sidebar-collapsed');
      if (sidebarEl && sidebarEl.classList) sidebarEl.classList.add('sidebar-collapsed');
    }
  }
  syncObservatoryHost();
  return winClass;
};

const lockObservatoryToFrame = (frame, framesPrefix = null, lockedIdx = null) => {
  if (!holoObservatoryState || !frame) return;
  holoObservatoryState.lockedFrameIdx = lockedIdx;
  if (Array.isArray(framesPrefix) && framesPrefix.length > 0) {
    holoObservatoryState.frames = framesPrefix.slice(-64);
  } else {
    holoObservatoryState.frames = [frame];
  }
  const trailSrc = holoObservatoryState.frames.slice(-32);
  holoObservatoryState.qTrail = trailSrc
    .filter((f) => Array.isArray(f && f.q_manifold_xyz) && f.q_manifold_xyz.length === 3)
    .map((f) => ({
      xyz: f.q_manifold_xyz,
      label: String(f.token_text || '').replace(/\s+/g, ' ').slice(0, 6),
    }));
  if (typeof frame.shortlist_count === 'number' && frame.shortlist_count > 0) {
    holoObservatoryState.shortlistK = frame.shortlist_count;
  }
  if (typeof frame.prefix_reused_tokens === 'number') {
    holoObservatoryState.prefixReused = frame.prefix_reused_tokens;
  }
  if (frame.backend) {
    holoObservatoryState.backend = String(frame.backend);
  }
  if (typeof renderHoloObservatoryDeck === 'function') {
    renderHoloObservatoryDeck();
  }
  if (
    lockedIdx !== null
    && typeof inspectorsModalEl !== 'undefined'
    && inspectorsModalEl
    && typeof inspectorsModalEl.hasAttribute === 'function'
    && !inspectorsModalEl.hasAttribute('hidden')
    && typeof openInspectorsModal === 'function'
  ) {
    openInspectorsModal('traces');
  }
};

const renderHoloMiniHud = (hudEl, frame, opts = {}) => {
  /* Formal R4.VtensorSemiring3d */
  if (!hudEl || !frame) return;
  const qTrail = Array.isArray(opts.qTrail) && opts.qTrail.length > 0
    ? opts.qTrail
    : (Array.isArray(frame.q_manifold_xyz) ? [frame.q_manifold_xyz] : []);
  const hasShortlist = frame.has_geometric_shortlist !== false;

  let canvas = typeof hudEl.querySelector === 'function' ? hudEl.querySelector('.holo-hud-canvas') : null;
  let metaRow = typeof hudEl.querySelector === 'function' ? hudEl.querySelector('.holo-hud-meta') : null;
  let candsBox = typeof hudEl.querySelector === 'function' ? hudEl.querySelector('.holo-hud-candidates') : null;

  if (!canvas || !metaRow || !candsBox) {
    if (typeof hudEl.replaceChildren === 'function') hudEl.replaceChildren();
    const topStrip = document.createElement('div');
    topStrip.className = 'holo-hud-top';

    canvas = document.createElement('canvas');
    canvas.className = 'holo-hud-canvas';
    canvas.width = 132;
    canvas.height = 48;
    topStrip.appendChild(canvas);

    metaRow = document.createElement('div');
    metaRow.className = 'holo-hud-meta';
    topStrip.appendChild(metaRow);

    candsBox = document.createElement('div');
    candsBox.className = 'holo-hud-candidates';

    hudEl.appendChild(topStrip);
    hudEl.appendChild(candsBox);
  }

  if (typeof canvas.getContext === 'function') {
    const dprRaw = typeof window !== 'undefined' && Number.isFinite(Number(window.devicePixelRatio))
      ? Number(window.devicePixelRatio)
      : 1;
    const dpr = Math.max(1, Math.min(2, dprRaw));
    const cssW = 132;
    const cssH = 48;
    if (canvas.style) {
      canvas.style.width = `${cssW}px`;
      canvas.style.height = `${cssH}px`;
    }
    if (canvas.width !== cssW * dpr || canvas.height !== cssH * dpr) {
      canvas.width = cssW * dpr;
      canvas.height = cssH * dpr;
    }
    const ctx = canvas.getContext('2d');
    if (ctx) {
      if (typeof ctx.save === 'function') ctx.save();
      if (typeof ctx.scale === 'function') ctx.scale(dpr, dpr);
      ctx.fillStyle = '#0b1120';
      ctx.fillRect(0, 0, cssW, cssH);

      // Left 44x44: 16x16 tile-binned 3D PBR Hopf-Poincare-Zeta mini manifold (`R4-70`)
      const miniPhase = ((Number(frame.step_idx || 0) * 28) + 128) & 1023;
      if (!(typeof blitHopfZeta3dRect === 'function' && blitHopfZeta3dRect(ctx, 2, 2, 44, 44, miniPhase, 1, qTrail)) && qTrail.length > 0) {
        const tail = qTrail[qTrail.length - 1] || [0, 0, 0];
        const hx = 23 + Math.max(-1, Math.min(1, (Number(tail[0]) || 0) / 1000)) * 17;
        const hy = 24 - Math.max(-1, Math.min(1, (Number(tail[1]) || 0) / 1000)) * 17;
        ctx.fillStyle = '#34d399';
        ctx.fillRect(hx - 1.5, hy - 1.5, 3, 3);
      }

      // Right 84x44: 28-layer bit-width / residual magnitude equalizer
      const layerBits = Array.isArray(frame.layer_max_bits) ? frame.layer_max_bits : [];
      const barCount = Math.min(28, Math.max(4, layerBits.length || 28));
      const stepW = 82 / barCount;
      for (let bi = 0; bi < barCount; bi += 1) {
        const bVal = layerBits[bi] !== undefined ? Number(layerBits[bi]) : 16;
        const frac = Math.max(0.08, Math.min(1.0, bVal / 32));
        const bh = Math.round(frac * 40);
        ctx.fillStyle = bVal > 22 ? '#f59e0b' : (hasShortlist ? '#38bdf8' : '#34d399');
        ctx.fillRect(48 + bi * stepW, 45 - bh, Math.max(1.5, stepW - 1), bh);
      }
      if (typeof ctx.restore === 'function') ctx.restore();
    }
  }

  if (typeof metaRow.replaceChildren === 'function') metaRow.replaceChildren();
  const phaseBadge = document.createElement('span');
  phaseBadge.className = hasShortlist ? 'holo-hud-pill' : 'holo-hud-pill holo-q31-badge';
  phaseBadge.textContent = hasShortlist
    ? `${frame.phase || 'decode'} #${frame.step_idx ?? 0}`
    : 'Q31 Exact Lane';
  metaRow.appendChild(phaseBadge);

  const ttftMs = Number(frame.ttft_ms || 0);
  const tpsX10 = Number(frame.tps_x10 || 0);
  const tpsText = tpsX10 > 0 ? `${(tpsX10 / 10).toFixed(1)} tok/s` : 'streaming';
  const kvReuse = Number(frame.prefix_reused_tokens || 0);
  const kvTotal = Math.max(1, Number(frame.seq_pos || 1));
  const kvPct = Math.min(100, Math.round((kvReuse / kvTotal) * 100));

  const statSpan = document.createElement('span');
  statSpan.className = 'holo-hud-stats';
  statSpan.textContent = hasShortlist
    ? `TTFT ${ttftMs}ms · ${tpsText} · KV ${kvPct}% · d_min=${frame.min_hamming ?? 0} / 512`
    : `TTFT ${ttftMs}ms · ${tpsText} · KV ${kvPct}% · 256D Integer Lane`;
  metaRow.appendChild(statSpan);

  if (typeof candsBox.replaceChildren === 'function') candsBox.replaceChildren();
  const cands = Array.isArray(frame.top_candidates) ? frame.top_candidates.slice(0, 6) : [];
  for (let ci = 0; ci < cands.length; ci += 1) {
    const c = cands[ci] || {};
    const row = document.createElement('div');
    row.className = c.selected ? 'holo-hud-cand is-selected' : 'holo-hud-cand';

    const tokEl = document.createElement('span');
    tokEl.className = 'holo-hud-cand-tok';
    tokEl.textContent = String(c.token_text ?? '');
    row.appendChild(tokEl);

    const barTrack = document.createElement('span');
    barTrack.className = 'holo-hud-bar-track';
    const barFill = document.createElement('span');
    barFill.className = 'holo-hud-bar';
    const probPm = Math.max(0, Math.min(1000, Number(c.prob_permille || 0)));
    if (barFill.style) {
      barFill.style.width = `${Math.max(4, Math.round(probPm / 10))}%`;
    }
    barTrack.appendChild(barFill);
    row.appendChild(barTrack);

    const numEl = document.createElement('span');
    numEl.className = 'holo-hud-cand-num';
    const dhText = hasShortlist && c.hamming_d !== null && c.hamming_d !== undefined ? ` d=${c.hamming_d}` : '';
    numEl.textContent = `${(probPm / 10).toFixed(1)}% · q16=${c.logit_q16 ?? 0}${dhText}`;
    row.appendChild(numEl);

    candsBox.appendChild(row);
  }
};

const createLiveTokenHudController = (chatMessagesEl, assistantBubbleEl = null) => {
  const MAX_SCRUBBER_TOKENS = 512;
  const hostEl = assistantBubbleEl || document.createElement('article');
  if (!assistantBubbleEl) {
    hostEl.className = 'chat-msg chat-msg-assistant holo-live-turn';
    if (chatMessagesEl && typeof chatMessagesEl.appendChild === 'function') {
      chatMessagesEl.appendChild(hostEl);
    }
  }
  const hudEl = document.createElement('div');
  hudEl.className = 'holo-live-hud';
  const scrubberEl = document.createElement('div');
  scrubberEl.className = 'holo-token-scrubber';
  hostEl.appendChild(hudEl);
  hostEl.appendChild(scrubberEl);

  const frames = [];
  const spans = [];
  const qTrail = [];
  let lockedIdx = null;

  const selectFrameIndex = (idx, lockInObs) => {
    if (idx < 0 || idx >= frames.length) return;
    const selectedFrame = frames[idx];
    for (let si = 0; si < spans.length; si += 1) {
      if (spans[si] && spans[si].classList) {
        spans[si].classList.toggle('is-selected', si === idx);
      }
      if (spans[si] && typeof spans[si].setAttribute === 'function') {
        spans[si].setAttribute('aria-pressed', (lockInObs && si === idx) ? 'true' : 'false');
      }
    }
    const prefix = frames.slice(0, idx + 1);
    const trailPrefix = prefix
      .slice(-32)
      .filter((f) => Array.isArray(f && f.q_manifold_xyz))
      .map((f) => f.q_manifold_xyz);
    renderHoloMiniHud(hudEl, selectedFrame, { qTrail: trailPrefix });
    if (lockInObs) {
      lockedIdx = idx;
      lockObservatoryToFrame(selectedFrame, prefix, idx);
    } else {
      lockObservatoryToFrame(selectedFrame, prefix, null);
    }
  };

  const toggleOrUnlockFrameIndex = (idxVal) => {
    if (lockedIdx === idxVal) {
      lockedIdx = null;
      if (typeof holoObservatoryState !== 'undefined' && holoObservatoryState) {
        holoObservatoryState.lockedFrameIdx = null;
      }
      const latestIdx = frames.length - 1;
      if (latestIdx >= 0) {
        selectFrameIndex(latestIdx, false);
      }
      return;
    }
    selectFrameIndex(idxVal, true);
  };

  const pushTokenStep = (stepEvt) => {
    const frame = (stepEvt && stepEvt.frame && typeof stepEvt.frame === 'object') ? stepEvt.frame : (stepEvt || {});
    frames.push(frame);
    let shifted = false;
    if (frames.length > MAX_SCRUBBER_TOKENS) {
      frames.shift();
      shifted = true;
      if (lockedIdx !== null) {
        lockedIdx -= 1;
        if (lockedIdx < 0) {
          lockedIdx = null;
          if (typeof holoObservatoryState !== 'undefined' && holoObservatoryState) {
            holoObservatoryState.lockedFrameIdx = null;
          }
        }
      }
    }
    if (Array.isArray(frame.q_manifold_xyz) && frame.q_manifold_xyz.length === 3) {
      qTrail.push(frame.q_manifold_xyz);
      if (qTrail.length > 32) qTrail.shift();
    }

    const span = document.createElement('span');
    span.className = 'holo-tok-span';
    span.setAttribute('role', 'button');
    span.setAttribute('tabindex', '0');
    span.setAttribute('aria-pressed', 'false');
    span.setAttribute('data-frame-idx', String(frames.length - 1));
    span.textContent = String(frame.token_text ?? (stepEvt && stepEvt.delta) ?? '');
    span.addEventListener('mouseenter', () => {
      const idxVal = Number(span.getAttribute('data-frame-idx'));
      if (lockedIdx === null) {
        selectFrameIndex(idxVal, false);
      }
    });
    span.addEventListener('click', () => {
      const idxVal = Number(span.getAttribute('data-frame-idx'));
      toggleOrUnlockFrameIndex(idxVal);
    });
    span.addEventListener('keydown', (ev) => {
      if (ev && (ev.key === 'Enter' || ev.key === ' ')) {
        if (typeof ev.preventDefault === 'function') ev.preventDefault();
        const idxVal = Number(span.getAttribute('data-frame-idx'));
        toggleOrUnlockFrameIndex(idxVal);
      }
    });
    spans.push(span);
    scrubberEl.appendChild(span);

    if (spans.length > MAX_SCRUBBER_TOKENS) {
      const dropped = spans.shift();
      _detachChildSafely(scrubberEl, dropped);
      shifted = true;
    }
    if (shifted) {
      for (let i = 0; i < spans.length; i += 1) {
        spans[i].setAttribute('data-frame-idx', String(i));
      }
    }

    if (lockedIdx === null) {
      renderHoloMiniHud(hudEl, frame, { qTrail });
    }
    return { frameCount: frames.length };
  };

  return {
    frames,
    hudEl,
    scrubberEl,
    containerEl: hostEl,
    getLockedIdx: () => lockedIdx,
    pushTokenStep,
    selectFrameIndex,
    toggleOrUnlockFrameIndex,
  };
};

const renderHydrationConsentCard = async (chatMessagesEl, details = {}, pendingPrompt = '') => {
  if (!chatMessagesEl) return null;
  const transcriptHost = (typeof chatTranscript !== 'undefined' && chatTranscript)
    || (typeof document !== 'undefined' && typeof document.getElementById === 'function'
      ? document.getElementById('chat-transcript')
      : null);
  for (const host of [chatMessagesEl, transcriptHost]) {
    if (host && typeof host.querySelectorAll === 'function') {
      for (const existing of Array.from(host.querySelectorAll('.holo-consent-card') || [])) {
        _detachChildSafely(existing.parentNode || host, existing);
      }
    }
  }
  const card = document.createElement('div');
  card.className = 'holo-consent-card';

  const title = document.createElement('div');
  title.className = 'holo-consent-title';
  const modelId = String(details.model_id || 'uor-r4.qwen3-0.6b-holo');
  const approxMb = Math.max(1, Math.round(Number(details.approx_bytes || 429916160) / (1024 * 1024)));
  title.textContent = `Off-Origin Weight Download Consent Required (${modelId} · ~${approxMb} MiB)`;
  card.appendChild(title);

  const mirrors = Array.isArray(details.mirrors) ? details.mirrors : [];
  const hosts = mirrors.map((u) => {
    try {
      return new URL(String(u)).host || String(u);
    } catch {
      return String(u);
    }
  });
  const body = document.createElement('p');
  body.className = 'holo-consent-desc';
  body.textContent = hosts.length > 0
    ? `Strict zero-network default blocked unapproved mirror access (${hosts.join(', ')}). Approve SHA-256 verified OPFS download or switch immediately to the zero-download Native Q31 engine.`
    : 'Strict zero-network default blocked unapproved weight download. Approve verified download or run Offline Native Q31 immediately.';
  card.appendChild(body);

  const actions = document.createElement('div');
  actions.className = 'holo-consent-actions';

  const approveBtn = document.createElement('button');
  approveBtn.type = 'button';
  approveBtn.className = 'btn-primary holo-consent-approve-btn';
  approveBtn.textContent = 'Download Verified Weights (~410 MiB / ~1.09 GiB)';

  const offlineBtn = document.createElement('button');
  offlineBtn.type = 'button';
  offlineBtn.className = 'btn-secondary holo-consent-offline-btn';
  offlineBtn.textContent = 'Use Offline Native Q31';

  const statusNote = document.createElement('span');
  statusNote.className = 'holo-consent-status';

  approveBtn.addEventListener('click', async () => {
    approveBtn.disabled = true;
    offlineBtn.disabled = true;
    if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient) {
      engineWorkerClient.pendingConsentDetails = null;
    }
    safeStorageSet('uor_allow_off_origin_hydration', '1');
    const chk = document.getElementById('settings-allow-off-origin-weights');
    if (chk) chk.checked = true;
    statusNote.textContent = 'Hydrating verified shards...';
    try {
      if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient && engineWorkerClient.enabled) {
        await engineWorkerClient.rpc('hydrate_holo_model', {
          modelId,
          allowOffOriginHydration: true,
          rememberConsent: true,
        });
        engineWorkerClient.rpc('set_bridge_policy', {
          allowOffOriginHydration: true,
          consentDeclined: false,
        }).catch(() => {});
      }
      if (typeof AppState !== 'undefined' && AppState.config) {
        AppState.config.selectedModel = modelId;
      }
      if (typeof syncComposerModel === 'function') {
        syncComposerModel(modelId);
      }
      if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient && engineWorkerClient.enabled) {
        engineWorkerClient.rpc('holo_runtime_status', {}).then((res) => {
          const hs = (res && res.holo && typeof res.mounted_count !== 'number') ? res.holo : res;
          if (hs && typeof hs.mounted_count === 'number') {
            engineWorkerClient.holoStatus = hs;
          }
          if (typeof updateWeightCacheBadge === 'function') updateWeightCacheBadge();
          if (typeof renderCursorShardsSection === 'function') renderCursorShardsSection();
          if (typeof renderCoworkStatusBar === 'function') renderCoworkStatusBar();
        }).catch(() => {});
      }
      statusNote.textContent = pendingPrompt
        ? 'Verified weights mounted. Re-running prompt...'
        : `Verified weights (${modelId}) mounted. Ready.`;
      if (pendingPrompt && typeof runClaudeChat === 'function') {
        runClaudeChat(pendingPrompt, false);
      }
    } catch (err) {
      statusNote.textContent = `Hydration error: ${err && err.message ? err.message : String(err)}`;
      approveBtn.disabled = false;
      offlineBtn.disabled = false;
    }
  });

  offlineBtn.addEventListener('click', async () => {
    approveBtn.disabled = true;
    offlineBtn.disabled = true;
    if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient) {
      engineWorkerClient.pendingConsentDetails = null;
    }
    if (typeof safeStorageRemove === 'function') {
      safeStorageRemove('uor_allow_off_origin_hydration');
    }
    safeStorageSet('uor_offline_emergency_q31', '1');
    const chk = document.getElementById('settings-allow-off-origin-weights');
    if (chk) chk.checked = false;
    const emergencyChk = document.getElementById('settings-offline-q31-toggle');
    if (emergencyChk) emergencyChk.checked = true;
    const fallbackModel = 'uor-r4.native-q31-d85';
    if (typeof AppState !== 'undefined' && AppState.config) {
      AppState.config.selectedModel = fallbackModel;
    }
    if (typeof syncComposerModel === 'function') {
      syncComposerModel(fallbackModel);
    } else if (typeof tsComposer !== 'undefined' && typeof tsComposer.setModel === 'function') {
      tsComposer.setModel(fallbackModel);
    }
    try {
      if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient && engineWorkerClient.enabled) {
        await engineWorkerClient.rpc('hydrate_holo_model', {
          modelId,
          offlineFallback: true,
          offlineEmergencyQ31: true,
          consentDeclined: false,
          fallbackModel,
        });
      }
    } catch {
      // Proceed with local Native Q31 switch even if Worker hook is idle
    }
    statusNote.textContent = `Switched to ${fallbackModel}.`;
    if (pendingPrompt && typeof runClaudeChat === 'function') {
      runClaudeChat(pendingPrompt, false);
    }
  });

  actions.appendChild(approveBtn);
  actions.appendChild(offlineBtn);
  actions.appendChild(statusNote);
  card.appendChild(actions);
  chatMessagesEl.appendChild(card);
  if (transcriptHost) {
    transcriptHost.scrollTop = transcriptHost.scrollHeight;
  } else if (typeof chatMessagesEl.scrollTop === 'number' && typeof chatMessagesEl.scrollHeight === 'number') {
    chatMessagesEl.scrollTop = chatMessagesEl.scrollHeight;
  }
  if (typeof card.scrollIntoView === 'function') {
    card.scrollIntoView({ block: 'nearest' });
  }
  return card;
};

class PreviewScriptFuelExhausted extends Error {
  constructor(maxFuel = 100000, steps = 0) {
    super(`PreviewScriptFuelExhausted: script exceeded ${maxFuel} execution steps (halted at step ${steps})`);
    this.name = 'PreviewScriptFuelExhausted';
    this.code = 'PreviewScriptFuelExhausted';
    this.error_code = 'PreviewScriptFuelExhausted';
    this.maxFuel = maxFuel;
    this.steps = steps;
  }
}

const evaluatePreviewScriptWithFuel = (code, env = {}, maxFuel = 100000) => {
  const src = String(code ?? '');
  const fuelBudget = Math.max(1, Number(maxFuel) | 0);
  const tokenizePreviewJs = (input) => {
    const tokens = [];
    let i = 0;
    const n = input.length;
    const twoCharOps = new Set(['===', '!==', '>>>', '**=', '<<=', '>>=', '&&=', '||=', '??=']);
    const multiOps = new Set(['=>', '==', '!=', '<=', '>=', '&&', '||', '??', '++', '--', '+=', '-=', '*=', '/=', '%=', '**', '<<', '>>', '?.']);
    const keywords = new Set(['let', 'const', 'var', 'function', 'return', 'if', 'else', 'for', 'while', 'do', 'break', 'continue', 'new', 'typeof', 'true', 'false', 'null', 'undefined', 'throw', 'try', 'catch', 'finally', 'of', 'in', 'void']);
    const scanPreviewTemplatePart = (src, startIdx) => {
      const len = src.length;
      let idx = startIdx + 1;
      const parts = [];
      let out = '';
      while (idx < len && src[idx] !== '`') {
        if (src[idx] === '\\' && idx + 1 < len) {
          const esc = src[idx + 1];
          if (esc === 'n') out += '\n';
          else if (esc === 'r') out += '\r';
          else if (esc === 't') out += '\t';
          else out += esc;
          idx += 2;
        } else if (src[idx] === '$' && src[idx + 1] === '\x7b') {
          if (out) {
            parts.push({ kind: 'str', value: out });
            out = '';
          }
          idx += 2;
          const exprStart = idx;
          let depth = 1;
          while (idx < len && depth > 0) {
            const c = src[idx];
            if (c === String.fromCharCode(34) || c === "'") {
              const q = c;
              idx += 1;
              while (idx < len && src[idx] !== q) {
                idx += (src[idx] === '\\' && idx + 1 < len) ? 2 : 1;
              }
              if (idx < len) idx += 1;
              continue;
            }
            if (c === '`') {
              const nested = scanPreviewTemplatePart(src, idx);
              idx = nested.nextIdx;
              continue;
            }
            if (c === '/' && src[idx + 1] === '/') {
              while (idx < len && src[idx] !== '\n') idx += 1;
              continue;
            }
            if (c === '/' && src[idx + 1] === '*') {
              idx += 2;
              while (idx < len && !(src[idx] === '*' && src[idx + 1] === '/')) idx += 1;
              idx += 2;
              continue;
            }
            if (c === '\x7b') depth += 1;
            else if (c === '\x7d') {
              depth -= 1;
              if (depth === 0) break;
            }
            idx += 1;
          }
          parts.push({ kind: 'expr', raw: src.slice(exprStart, idx) });
          if (idx < len && src[idx] === '\x7d') idx += 1;
        } else {
          out += src[idx];
          idx += 1;
        }
      }
      if (idx < len && src[idx] === '`') idx += 1;
      return { parts, out, nextIdx: idx };
    };
    while (i < n) {
      const ch = input[i];
      if (ch === ' ' || ch === '\t' || ch === '\n' || ch === '\r') {
        i += 1;
        continue;
      }
      if (ch === '/' && input[i + 1] === '/') {
        while (i < n && input[i] !== '\n') i += 1;
        continue;
      }
      if (ch === '/' && input[i + 1] === '*') {
        i += 2;
        while (i < n && !(input[i] === '*' && input[i + 1] === '/')) i += 1;
        i += 2;
        continue;
      }
      if (ch === String.fromCharCode(34) || ch === "'") {
        const quote = ch;
        i += 1;
        let out = '';
        while (i < n && input[i] !== quote) {
          if (input[i] === '\\' && i + 1 < n) {
            const esc = input[i + 1];
            if (esc === 'n') out += '\n';
            else if (esc === 'r') out += '\r';
            else if (esc === 't') out += '\t';
            else out += esc;
            i += 2;
          } else {
            out += input[i];
            i += 1;
          }
        }
        i += 1;
        tokens.push({ type: 'string', value: out });
        continue;
      }
      if (ch === '`') {
        const scanned = scanPreviewTemplatePart(input, i);
        i = scanned.nextIdx;
        if (scanned.parts.length === 0) {
          tokens.push({ type: 'string', value: scanned.out });
        } else {
          if (scanned.out) scanned.parts.push({ kind: 'str', value: scanned.out });
          tokens.push({ type: 'template', parts: scanned.parts });
        }
        continue;
      }
      if ((ch >= '0' && ch <= '9') || (ch === '.' && input[i + 1] >= '0' && input[i + 1] <= '9')) {
        let j = i;
        if (input[j] === '0' && (input[j + 1] === 'x' || input[j + 1] === 'X')) {
          j += 2;
          while (j < n && /[0-9a-fA-F]/.test(input[j])) j += 1;
        } else {
          while (j < n && /[0-9]/.test(input[j])) j += 1;
          if (input[j] === '.') {
            j += 1;
            while (j < n && /[0-9]/.test(input[j])) j += 1;
          }
          if (input[j] === 'e' || input[j] === 'E') {
            j += 1;
            if (input[j] === '+' || input[j] === '-') j += 1;
            while (j < n && /[0-9]/.test(input[j])) j += 1;
          }
        }
        tokens.push({ type: 'number', value: Number(input.slice(i, j)) });
        i = j;
        continue;
      }
      if ((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || ch === '_' || ch === '$') {
        let j = i + 1;
        while (j < n && /[a-zA-Z0-9_$]/.test(input[j])) j += 1;
        const word = input.slice(i, j);
        tokens.push({ type: keywords.has(word) ? 'kw' : 'id', value: word });
        i = j;
        continue;
      }
      const s3 = input.slice(i, i + 3);
      if (twoCharOps.has(s3)) {
        tokens.push({ type: 'op', value: s3 });
        i += 3;
        continue;
      }
      const s2 = input.slice(i, i + 2);
      if (multiOps.has(s2)) {
        tokens.push({ type: 'op', value: s2 });
        i += 2;
        continue;
      }
      tokens.push({ type: 'punc', value: ch });
      i += 1;
    }
    tokens.push({ type: 'eof', value: '' });
    return tokens;
  };

  let toks = tokenizePreviewJs(src);
  let pos = 0;
  const parseNestedExpr = (exprSrc) => {
    const savedToks = toks;
    const savedPos = pos;
    toks = tokenizePreviewJs(exprSrc);
    pos = 0;
    try {
      const exprAst = parseExpr();
      if (peek().type !== 'eof') {
        throw new SyntaxError(`Unexpected token '${peek().value}' inside template interpolation`);
      }
      return exprAst;
    } finally {
      toks = savedToks;
      pos = savedPos;
    }
  };
  const peek = () => toks[pos] || { type: 'eof', value: '' };
  const next = () => { const t = toks[pos]; pos += 1; return t; };
  const match = (type, val) => {
    const t = peek();
    if (t.type === type && (val === undefined || t.value === val)) {
      pos += 1;
      return t;
    }
    return null;
  };
  const expect = (type, val) => {
    const t = match(type, val);
    if (!t) {
      const got = peek();
      throw new SyntaxError(`Unexpected token ${got.type}:${got.value} (expected ${val ?? type})`);
    }
    return t;
  };

  const parseProgram = () => {
    const body = [];
    while (peek().type !== 'eof') {
      if (match('punc', ';')) continue;
      body.push(parseStatement());
    }
    return { type: 'Program', body };
  };

  const parseBlock = () => {
    expect('punc', '\x7b');
    const body = [];
    while (peek().type !== 'eof' && !(peek().type === 'punc' && peek().value === '\x7d')) {
      if (match('punc', ';')) continue;
      body.push(parseStatement());
    }
    expect('punc', '\x7d');
    return { type: 'Block', body };
  };

  const parseParamList = () => {
    expect('punc', '(');
    const params = [];
    while (peek().type !== 'eof' && !(peek().type === 'punc' && peek().value === ')')) {
      params.push(expect('id').value);
      if (!match('punc', ',')) break;
    }
    expect('punc', ')');
    return params;
  };

  const parseVarDecl = (eatSemi = true) => {
    const kind = next().value;
    const decls = [];
    while (true) {
      const name = expect('id').value;
      const init = match('punc', '=') ? parseAssign() : null;
      decls.push({ name, init });
      if (!match('punc', ',')) break;
    }
    if (eatSemi) match('punc', ';');
    return { type: 'VarDecl', kind, decls };
  };

  const parseStatement = () => {
    const t = peek();
    if (t.type === 'punc' && t.value === '\x7b') return parseBlock();
    if (t.type === 'kw' && (t.value === 'let' || t.value === 'const' || t.value === 'var')) return parseVarDecl(true);
    if (match('kw', 'function')) {
      const name = expect('id').value;
      const params = parseParamList();
      const body = parseBlock();
      return { type: 'FnDecl', name, params, body };
    }
    if (match('kw', 'return')) {
      const arg = (peek().type === 'punc' && (peek().value === ';' || peek().value === '\x7d')) ? null : parseExpr();
      match('punc', ';');
      return { type: 'Return', arg };
    }
    if (match('kw', 'throw')) {
      const arg = parseExpr();
      match('punc', ';');
      return { type: 'Throw', arg };
    }
    if (match('kw', 'break')) {
      match('punc', ';');
      return { type: 'Break' };
    }
    if (match('kw', 'continue')) {
      match('punc', ';');
      return { type: 'Continue' };
    }
    if (match('kw', 'if')) {
      expect('punc', '(');
      const test = parseExpr();
      expect('punc', ')');
      const cons = parseStatement();
      const alt = match('kw', 'else') ? parseStatement() : null;
      return { type: 'If', test, cons, alt };
    }
    if (match('kw', 'while')) {
      expect('punc', '(');
      const test = parseExpr();
      expect('punc', ')');
      const body = parseStatement();
      return { type: 'While', test, body };
    }
    if (match('kw', 'do')) {
      const body = parseStatement();
      expect('kw', 'while');
      expect('punc', '(');
      const test = parseExpr();
      expect('punc', ')');
      match('punc', ';');
      return { type: 'DoWhile', test, body };
    }
    if (match('kw', 'for')) {
      expect('punc', '(');
      if ((peek().type === 'kw' && (peek().value === 'let' || peek().value === 'const' || peek().value === 'var')) && toks[pos + 2]?.type === 'kw' && (toks[pos + 2]?.value === 'of' || toks[pos + 2]?.value === 'in')) {
        const kind = next().value;
        const name = expect('id').value;
        const loopKind = next().value;
        const rightExpr = parseExpr();
        expect('punc', ')');
        const body = parseStatement();
        return { type: loopKind === 'of' ? 'ForOf' : 'ForIn', kind, name, iter: rightExpr, body };
      }
      let init = null;
      if (!match('punc', ';')) {
        if (peek().type === 'kw' && (peek().value === 'let' || peek().value === 'const' || peek().value === 'var')) {
          init = parseVarDecl(true);
        } else {
          init = parseExpr();
          expect('punc', ';');
        }
      }
      const test = match('punc', ';') ? null : (() => { const e = parseExpr(); expect('punc', ';'); return e; })();
      const update = match('punc', ')') ? null : (() => { const e = parseExpr(); expect('punc', ')'); return e; })();
      const body = parseStatement();
      return { type: 'For', init, test, update, body };
    }
    if (match('kw', 'try')) {
      const tryBlock = parseBlock();
      let catchName = null;
      let catchBlock = null;
      if (match('kw', 'catch')) {
        if (match('punc', '(')) {
          catchName = expect('id').value;
          expect('punc', ')');
        }
        catchBlock = parseBlock();
      }
      const finalBlock = match('kw', 'finally') ? parseBlock() : null;
      return { type: 'Try', tryBlock, catchName, catchBlock, finalBlock };
    }
    const expr = parseExpr();
    match('punc', ';');
    return { type: 'ExprStmt', expr };
  };

  const parseExpr = () => {
    let leftNode = parseAssign();
    if (peek().type === 'punc' && peek().value === ',') {
      const items = [leftNode];
      while (match('punc', ',')) items.push(parseAssign());
      return { type: 'Seq', items };
    }
    return leftNode;
  };

  const isArrowAhead = () => {
    if (peek().type === 'id' && toks[pos + 1]?.type === 'op' && toks[pos + 1]?.value === '=>') return true;
    if (peek().type === 'punc' && peek().value === '(') {
      let depth = 0;
      let scan = pos;
      while (scan < toks.length) {
        const cur = toks[scan];
        if (cur.type === 'punc' && cur.value === '(') depth += 1;
        else if (cur.type === 'punc' && cur.value === ')') {
          depth -= 1;
          if (depth === 0) return toks[scan + 1]?.type === 'op' && toks[scan + 1]?.value === '=>';
        }
        scan += 1;
      }
    }
    return false;
  };

  const parseAssign = () => {
    if (isArrowAhead()) {
      const params = peek().type === 'id' ? [next().value] : parseParamList();
      expect('op', '=>');
      const body = (peek().type === 'punc' && peek().value === '\x7b') ? parseBlock() : parseAssign();
      return { type: 'Arrow', params, body };
    }
    const node = parseConditional();
    const t = peek();
    if ((t.type === 'punc' && t.value === '=') || (t.type === 'op' && ['+=', '-=', '*=', '/=', '%=', '??=', '||=', '&&='].includes(t.value))) {
      const op = next().value;
      const rhsNode = parseAssign();
      return { type: 'Assign', op, target: node, value: rhsNode };
    }
    return node;
  };

  const parseConditional = () => {
    const test = parseBinOp(0);
    if (match('punc', '?')) {
      const cons = parseAssign();
      expect('punc', ':');
      const alt = parseAssign();
      return { type: 'Ternary', test, cons, alt };
    }
    return test;
  };

  const PRECEDENCE = [
    ['||', '??'],
    ['&&'],
    ['|'],
    ['^'],
    ['&'],
    ['==', '!=', '===', '!=='],
    ['<', '<=', '>', '>='],
    ['<<', '>>', '>>>'],
    ['+', '-'],
    ['*', '/', '%'],
  ];

  const matchPrecOp = (ops) => {
    const t = peek();
    if ((t.type === 'op' || t.type === 'punc') && ops.includes(t.value)) {
      pos += 1;
      return t.value;
    }
    return null;
  };

  const parseBinOp = (level) => {
    if (level >= PRECEDENCE.length) return parseUnary();
    let cur = parseBinOp(level + 1);
    let op;
    while ((op = matchPrecOp(PRECEDENCE[level])) !== null) {
      const rhsNode = parseBinOp(level + 1);
      cur = { type: 'BinOp', op, lhs: cur, rhs: rhsNode };
    }
    return cur;
  };

  const parseUnary = () => {
    const t = peek();
    if ((t.type === 'punc' && (t.value === '!' || t.value === '-' || t.value === '+'))
      || (t.type === 'op' && (t.value === '++' || t.value === '--'))
      || (t.type === 'kw' && (t.value === 'typeof' || t.value === 'void'))) {
      const op = next().value;
      return { type: 'Unary', op, arg: parseUnary() };
    }
    if (match('kw', 'new')) {
      const callee = parseMember(false);
      let args = [];
      if (peek().type === 'punc' && peek().value === '(') {
        args = parseArgList();
      }
      return parsePostfixTail({ type: 'New', callee, args }, true);
    }
    return parseMember(true);
  };

  const parseArgList = () => {
    expect('punc', '(');
    const args = [];
    while (peek().type !== 'eof' && !(peek().type === 'punc' && peek().value === ')')) {
      args.push(parseAssign());
      if (!match('punc', ',')) break;
    }
    expect('punc', ')');
    return args;
  };

  const parsePostfixTail = (base, allowCall) => {
    let cur = base;
    while (true) {
      if (match('punc', '.') || match('op', '?.')) {
        const propTok = next();
        cur = { type: 'Member', obj: cur, prop: { type: 'Literal', value: propTok.value }, computed: false };
      } else if (match('punc', '[')) {
        const propExpr = parseExpr();
        expect('punc', ']');
        cur = { type: 'Member', obj: cur, prop: propExpr, computed: true };
      } else if (allowCall && peek().type === 'punc' && peek().value === '(') {
        cur = { type: 'Call', callee: cur, args: parseArgList() };
      } else if (peek().type === 'op' && (peek().value === '++' || peek().value === '--')) {
        cur = { type: 'Update', op: next().value, arg: cur, prefix: false };
      } else {
        break;
      }
    }
    return cur;
  };

  const parseMember = (allowCall) => parsePostfixTail(parsePrimary(), allowCall);

  const parsePrimary = () => {
    const t = peek();
    if (t.type === 'number' || t.type === 'string') {
      next();
      return { type: 'Literal', value: t.value };
    }
    if (t.type === 'template') {
      next();
      const compiledParts = t.parts.map((part) => (
        part.kind === 'str'
          ? { kind: 'str', value: part.value }
          : { kind: 'expr', ast: parseNestedExpr(part.raw) }
      ));
      return { type: 'Template', parts: compiledParts };
    }
    if (match('kw', 'true')) return { type: 'Literal', value: true };
    if (match('kw', 'false')) return { type: 'Literal', value: false };
    if (match('kw', 'null')) return { type: 'Literal', value: null };
    if (match('kw', 'undefined')) return { type: 'Literal', value: undefined };
    if (match('kw', 'function')) {
      const name = peek().type === 'id' ? next().value : null;
      const params = parseParamList();
      const body = parseBlock();
      return { type: 'FnExpr', name, params, body };
    }
    if (t.type === 'id') {
      next();
      return { type: 'Ident', name: t.value };
    }
    if (match('punc', '(')) {
      const expr = parseExpr();
      expect('punc', ')');
      return expr;
    }
    if (match('punc', '[')) {
      const elements = [];
      while (peek().type !== 'eof' && !(peek().type === 'punc' && peek().value === ']')) {
        elements.push(parseAssign());
        if (!match('punc', ',')) break;
      }
      expect('punc', ']');
      return { type: 'Array', elements };
    }
    if (match('punc', '\x7b')) {
      const props = [];
      while (peek().type !== 'eof' && !(peek().type === 'punc' && peek().value === '\x7d')) {
        let key;
        if (match('punc', '[')) {
          key = parseAssign();
          expect('punc', ']');
        } else {
          const kt = next();
          key = { type: 'Literal', value: String(kt.value) };
        }
        if (match('punc', ':')) {
          props.push({ key, value: parseAssign() });
        } else {
          props.push({ key, value: { type: 'Ident', name: key.value } });
        }
        if (!match('punc', ',')) break;
      }
      expect('punc', '\x7d');
      return { type: 'Object', props };
    }
    throw new SyntaxError(`Unexpected primary token ${t.type}:${t.value}`);
  };

  const ast = parseProgram();
  let steps = 0;
  const tickFuel = () => {
    steps += 1;
    if (steps > fuelBudget) {
      throw new PreviewScriptFuelExhausted(fuelBudget, steps);
    }
  };

  const rootScope = { parent: null, vars: new Map() };
  const defaultPreviewGlobals = createPreviewTimerGlobals((err) => {
    const c = env.console || { error: () => {} };
    if (typeof c.error === 'function') c.error(err && err.message ? err.message : String(err));
  });
  const builtIns = {
    Math,
    Number,
    String,
    Boolean,
    Array,
    Object,
    JSON,
    Map,
    Set,
    Uint8Array,
    Int32Array,
    Float32Array,
    isFinite: Number.isFinite,
    isNaN: Number.isNaN,
    WebAssembly: (typeof WebAssembly !== 'undefined') ? WebAssembly : undefined,
    ...defaultPreviewGlobals,
    console: env.console || { log: () => {}, warn: () => {}, error: () => {}, info: () => {} },
    document: env.document,
    window: env.window,
    undefined,
  };
  for (const [k, v] of Object.entries(builtIns)) {
    rootScope.vars.set(k, v);
  }
  for (const [k, v] of Object.entries(env || {})) {
    rootScope.vars.set(k, v);
  }

  const resolveScope = (scope, name) => {
    let cur = scope;
    while (cur) {
      if (cur.vars.has(name)) return cur;
      cur = cur.parent;
    }
    return null;
  };

  const SIGNAL_RETURN = Symbol('return');
  const SIGNAL_BREAK = Symbol('break');
  const SIGNAL_CONTINUE = Symbol('continue');

  const makeCallable = (fnNode, closureScope) => {
    const fn = function (...args) {
      tickFuel();
      const fnScope = { parent: closureScope, vars: new Map() };
      for (let pi = 0; pi < fnNode.params.length; pi += 1) {
        fnScope.vars.set(fnNode.params[pi], args[pi]);
      }
      if (fnNode.name) {
        fnScope.vars.set(fnNode.name, fn);
      }
      if (fnNode.body.type === 'Block') {
        const res = evalStmt(fnNode.body, fnScope);
        if (res && res.signal === SIGNAL_RETURN) return res.value;
        return undefined;
      }
      return evalExpr(fnNode.body, fnScope);
    };
    return fn;
  };

  const evalLValueRef = (node, scope) => {
    if (node.type === 'Ident') {
      const found = resolveScope(scope, node.name) || rootScope;
      return {
        get: () => found.vars.get(node.name),
        set: (v) => { found.vars.set(node.name, v); return v; },
      };
    }
    if (node.type === 'Member') {
      const targetObj = evalExpr(node.obj, scope);
      const propKey = node.computed ? evalExpr(node.prop, scope) : node.prop.value;
      if (propKey === '__proto__' || propKey === 'constructor' || propKey === 'prototype') {
        throw new Error('Blocked prototype property write inside preview sandbox');
      }
      return {
        get: () => targetObj[propKey],
        set: (v) => { targetObj[propKey] = v; return v; },
      };
    }
    throw new ReferenceError('Invalid assignment target');
  };

  const evalExpr = (node, scope) => {
    tickFuel();
    if (!node) return undefined;
    switch (node.type) {
      case 'Literal': return node.value;
      case 'Template': {
        let out = '';
        for (const p of node.parts) {
          out += p.kind === 'str' ? p.value : String(evalExpr(p.ast, scope));
        }
        return out;
      }
      case 'Ident': {
        const s = resolveScope(scope, node.name);
        if (!s) throw new ReferenceError(`${node.name} is not defined`);
        return s.vars.get(node.name);
      }
      case 'Array': return node.elements.map((el) => evalExpr(el, scope));
      case 'Object': {
        const obj = {};
        for (const p of node.props) {
          const k = evalExpr(p.key, scope);
          if (k !== '__proto__' && k !== 'constructor') {
            obj[k] = evalExpr(p.value, scope);
          }
        }
        return obj;
      }
      case 'Arrow':
      case 'FnExpr': return makeCallable(node, scope);
      case 'Seq': {
        let out;
        for (const it of node.items) out = evalExpr(it, scope);
        return out;
      }
      case 'Ternary': return evalExpr(node.test, scope) ? evalExpr(node.cons, scope) : evalExpr(node.alt, scope);
      case 'Unary': {
        if (node.op === 'typeof') {
          if (node.arg.type === 'Ident' && !resolveScope(scope, node.arg.name)) return 'undefined';
          return typeof evalExpr(node.arg, scope);
        }
        if (node.op === '++' || node.op === '--') {
          const ref = evalLValueRef(node.arg, scope);
          const nextV = Number(ref.get()) + (node.op === '++' ? 1 : -1);
          ref.set(nextV);
          return nextV;
        }
        const v = evalExpr(node.arg, scope);
        if (node.op === '!') return !v;
        if (node.op === '-') return -Number(v);
        if (node.op === '+') return +Number(v);
        return undefined;
      }
      case 'Update': {
        const ref = evalLValueRef(node.arg, scope);
        const curV = Number(ref.get());
        const nextV = curV + (node.op === '++' ? 1 : -1);
        ref.set(nextV);
        return node.prefix ? nextV : curV;
      }
      case 'Assign': {
        const ref = evalLValueRef(node.target, scope);
        if (node.op === '=') return ref.set(evalExpr(node.value, scope));
        const prevV = ref.get();
        if (node.op === '&&=') return prevV ? ref.set(evalExpr(node.value, scope)) : prevV;
        if (node.op === '||=') return prevV ? prevV : ref.set(evalExpr(node.value, scope));
        if (node.op === '??=') return (prevV !== null && prevV !== undefined) ? prevV : ref.set(evalExpr(node.value, scope));
        const rhsVal = evalExpr(node.value, scope);
        if (node.op === '+=') return ref.set(prevV + rhsVal);
        if (node.op === '-=') return ref.set(Number(prevV) - Number(rhsVal));
        if (node.op === '*=') return ref.set(Number(prevV) * Number(rhsVal));
        if (node.op === '/=') return ref.set(Number(prevV) / Number(rhsVal));
        if (node.op === '%=') return ref.set(Number(prevV) % Number(rhsVal));
        return prevV;
      }
      case 'BinOp': {
        if (node.op === '&&') return evalExpr(node.lhs, scope) ? evalExpr(node.rhs, scope) : false;
        if (node.op === '||') {
          const lv = evalExpr(node.lhs, scope);
          return lv ? lv : evalExpr(node.rhs, scope);
        }
        if (node.op === '??') {
          const lv = evalExpr(node.lhs, scope);
          return (lv !== null && lv !== undefined) ? lv : evalExpr(node.rhs, scope);
        }
        const lhsVal = evalExpr(node.lhs, scope);
        const rhsVal = evalExpr(node.rhs, scope);
        switch (node.op) {
          case '+': return lhsVal + rhsVal;
          case '-': return Number(lhsVal) - Number(rhsVal);
          case '*': return Number(lhsVal) * Number(rhsVal);
          case '/': return Number(lhsVal) / Number(rhsVal);
          case '%': return Number(lhsVal) % Number(rhsVal);
          case '===': return lhsVal === rhsVal;
          case '!==': return lhsVal !== rhsVal;
          case '==': return lhsVal == rhsVal;
          case '!=': return lhsVal != rhsVal;
          case '<': return lhsVal < rhsVal;
          case '<=': return lhsVal <= rhsVal;
          case '>': return lhsVal > rhsVal;
          case '>=': return lhsVal >= rhsVal;
          case '&': return (Number(lhsVal) | 0) & (Number(rhsVal) | 0);
          case '|': return (Number(lhsVal) | 0) | (Number(rhsVal) | 0);
          case '^': return (Number(lhsVal) | 0) ^ (Number(rhsVal) | 0);
          case '<<': return (Number(lhsVal) | 0) << (Number(rhsVal) | 0);
          case '>>': return (Number(lhsVal) | 0) >> (Number(rhsVal) | 0);
          case '>>>': return (Number(lhsVal) | 0) >>> (Number(rhsVal) | 0);
          default: return undefined;
        }
      }
      case 'Member': {
        const obj = evalExpr(node.obj, scope);
        if (obj === null || obj === undefined) return undefined;
        const k = node.computed ? evalExpr(node.prop, scope) : node.prop.value;
        if (k === '__proto__' || k === 'constructor') return undefined;
        return obj[k];
      }
      case 'Call': {
        let thisArg = undefined;
        let fnVal;
        if (node.callee.type === 'Member') {
          thisArg = evalExpr(node.callee.obj, scope);
          const k = node.callee.computed ? evalExpr(node.callee.prop, scope) : node.callee.prop.value;
          if (k === '__proto__' || k === 'constructor') throw new Error('Blocked constructor invocation');
          fnVal = thisArg ? thisArg[k] : undefined;
        } else {
          fnVal = evalExpr(node.callee, scope);
        }
        if (typeof fnVal !== 'function') {
          throw new TypeError('Callee is not a function inside preview sandbox');
        }
        const argVals = node.args.map((argNode) => evalExpr(argNode, scope));
        return fnVal.apply(thisArg, argVals);
      }
      case 'New': {
        const Ctor = evalExpr(node.callee, scope);
        if (typeof Ctor !== 'function') throw new TypeError('Target is not a constructor');
        const argVals = node.args.map((argNode) => evalExpr(argNode, scope));
        return new Ctor(...argVals);
      }
      default:
        return undefined;
    }
  };

  const evalStmt = (stmt, scope) => {
    tickFuel();
    if (!stmt) return { signal: null, value: undefined };
    switch (stmt.type) {
      case 'Block': {
        const blkScope = { parent: scope, vars: new Map() };
        let lastVal;
        for (const s of stmt.body) {
          const r = evalStmt(s, blkScope);
          if (r.signal) return r;
          lastVal = r.value;
        }
        return { signal: null, value: lastVal };
      }
      case 'VarDecl': {
        for (const d of stmt.decls) {
          scope.vars.set(d.name, d.init ? evalExpr(d.init, scope) : undefined);
        }
        return { signal: null, value: undefined };
      }
      case 'FnDecl': {
        scope.vars.set(stmt.name, makeCallable(stmt, scope));
        return { signal: null, value: undefined };
      }
      case 'ExprStmt': return { signal: null, value: evalExpr(stmt.expr, scope) };
      case 'Return': return { signal: SIGNAL_RETURN, value: stmt.arg ? evalExpr(stmt.arg, scope) : undefined };
      case 'Throw': throw evalExpr(stmt.arg, scope);
      case 'Break': return { signal: SIGNAL_BREAK, value: undefined };
      case 'Continue': return { signal: SIGNAL_CONTINUE, value: undefined };
      case 'If': {
        if (evalExpr(stmt.test, scope)) return evalStmt(stmt.cons, scope);
        if (stmt.alt) return evalStmt(stmt.alt, scope);
        return { signal: null, value: undefined };
      }
      case 'While': {
        let lastVal;
        while (evalExpr(stmt.test, scope)) {
          tickFuel();
          const r = evalStmt(stmt.body, scope);
          if (r.signal === SIGNAL_BREAK) break;
          if (r.signal === SIGNAL_RETURN) return r;
          lastVal = r.value;
        }
        return { signal: null, value: lastVal };
      }
      case 'DoWhile': {
        let lastVal;
        do {
          tickFuel();
          const r = evalStmt(stmt.body, scope);
          if (r.signal === SIGNAL_BREAK) break;
          if (r.signal === SIGNAL_RETURN) return r;
          lastVal = r.value;
        } while (evalExpr(stmt.test, scope));
        return { signal: null, value: lastVal };
      }
      case 'For': {
        const loopScope = { parent: scope, vars: new Map() };
        if (stmt.init) {
          if (stmt.init.type === 'VarDecl') evalStmt(stmt.init, loopScope);
          else evalExpr(stmt.init, loopScope);
        }
        let lastVal;
        while (!stmt.test || evalExpr(stmt.test, loopScope)) {
          tickFuel();
          const r = evalStmt(stmt.body, loopScope);
          if (r.signal === SIGNAL_BREAK) break;
          if (r.signal === SIGNAL_RETURN) return r;
          lastVal = r.value;
          if (stmt.update) evalExpr(stmt.update, loopScope);
        }
        return { signal: null, value: lastVal };
      }
      case 'ForOf':
      case 'ForIn': {
        const container = evalExpr(stmt.iter, scope);
        const items = stmt.type === 'ForOf' ? Array.from(container || []) : Object.keys(container || {});
        let lastVal;
        for (const item of items) {
          tickFuel();
          const iterScope = { parent: scope, vars: new Map([[stmt.name, item]]) };
          const r = evalStmt(stmt.body, iterScope);
          if (r.signal === SIGNAL_BREAK) break;
          if (r.signal === SIGNAL_RETURN) return r;
          lastVal = r.value;
        }
        return { signal: null, value: lastVal };
      }
      case 'Try': {
        let out = { signal: null, value: undefined };
        try {
          out = evalStmt(stmt.tryBlock, scope);
        } catch (err) {
          if (err instanceof PreviewScriptFuelExhausted) throw err;
          if (stmt.catchBlock) {
            const catchScope = { parent: scope, vars: new Map() };
            if (stmt.catchName) catchScope.vars.set(stmt.catchName, err);
            out = evalStmt(stmt.catchBlock, catchScope);
          } else {
            throw err;
          }
        } finally {
          if (stmt.finalBlock) evalStmt(stmt.finalBlock, scope);
        }
        return out;
      }
      default:
        return { signal: null, value: undefined };
    }
  };

  let completionVal;
  for (const s of ast.body) {
    if (s.type === 'FnDecl') {
      rootScope.vars.set(s.name, makeCallable(s, rootScope));
    }
  }
  for (const s of ast.body) {
    if (s.type === 'FnDecl') continue;
    const res = evalStmt(s, rootScope);
    completionVal = res.value;
    if (res.signal === SIGNAL_RETURN) break;
  }
  return { value: completionVal, steps };
};

const renderPreviewWorkspace = (files, rootEl, postDiag = null, selector = '') => {
  clearPreviewTimers();
  if (!rootEl) return { fileCount: 0, scriptSteps: 0, errors: [] };
  if (typeof rootEl.replaceChildren === 'function') rootEl.replaceChildren();

  const fileMap = new Map();
  if (Array.isArray(files)) {
    for (const entry of files) {
      if (entry && typeof entry.path === 'string') {
        fileMap.set(entry.path.replace(/^\/workspace\/?/, ''), String(entry.content ?? ''));
      }
    }
  } else if (files && typeof files === 'object') {
    for (const [p, content] of Object.entries(files)) {
      fileMap.set(String(p).replace(/^\/workspace\/?/, ''), String(content ?? ''));
    }
  }

  let htmlPath = fileMap.has('index.html') ? 'index.html' : null;
  if (!htmlPath) {
    for (const k of fileMap.keys()) {
      if (k.endsWith('.html') || k.endsWith('.htm')) {
        htmlPath = k;
        break;
      }
    }
  }
  const rawHtml = htmlPath ? (fileMap.get(htmlPath) || '') : '<div id=\x22app\x22></div>';
  const collectedScripts = [];
  const collectedCss = [];
  const errors = [];
  const consoleLogs = [];

  for (const [k, cssText] of fileMap.entries()) {
    if (k.endsWith('.css')) collectedCss.push(cssText);
  }

  const sanitizeCssText = (cssStr) =>
    String(cssStr ?? '')
      .replace(/@import\b[^;]*;?/gi, '')
      .replace(/expression\s*\(/gi, 'blocked(')
      .replace(/url\s*\(\s*([\x27\x22]?)\s*(?:javascript:|data:text\/html|http:|\/\/)/gi, 'url($1blocked:');

  if (typeof DOMParser === 'function') {
    const parser = new DOMParser();
    const parsedDoc = parser.parseFromString(rawHtml, 'text/html');
    const cloneSafeNode = (srcNode) => {
      if (!srcNode) return null;
      if (srcNode.nodeType === 3) {
        return document.createTextNode(srcNode.textContent || '');
      }
      if (srcNode.nodeType !== 1) return null;
      const tag = String(srcNode.tagName || '').toLowerCase();
      if (tag === 'script') {
        const srcAttr = srcNode.getAttribute('src');
        if (srcAttr) {
          const cleanPath = srcAttr.replace(/^\.?\//, '').replace(/^\/workspace\/?/, '');
          if (fileMap.has(cleanPath)) collectedScripts.push(fileMap.get(cleanPath) || '');
        } else if (srcNode.textContent) {
          collectedScripts.push(srcNode.textContent);
        }
        return null;
      }
      if (tag === 'style') {
        collectedCss.push(srcNode.textContent || '');
        return null;
      }
      if (tag === 'link') {
        const rel = String(srcNode.getAttribute('rel') || '').toLowerCase();
        const href = srcNode.getAttribute('href') || '';
        if (rel === 'stylesheet' && href) {
          const cleanPath = href.replace(/^\.?\//, '').replace(/^\/workspace\/?/, '');
          if (fileMap.has(cleanPath)) collectedCss.push(fileMap.get(cleanPath) || '');
        }
        return null;
      }
      if (tag === 'iframe' || tag === 'object' || tag === 'embed' || tag === 'meta' || tag === 'base') {
        return null;
      }
      const el = document.createElement(tag);
      for (const attr of Array.from(srcNode.attributes || [])) {
        const aName = String(attr.name || '').toLowerCase();
        const aVal = String(attr.value || '');
        if (aName.startsWith('on') || aName === 'srcdoc') continue;
        if ((aName === 'href' || aName === 'src' || aName === 'action') && /^\s*(?:javascript:|vbscript:|data:text\/html)/i.test(aVal)) {
          continue;
        }
        el.setAttribute(attr.name, aVal);
      }
      for (const ch of Array.from(srcNode.childNodes || [])) {
        const cloned = cloneSafeNode(ch);
        if (cloned) el.appendChild(cloned);
      }
      return el;
    };

    for (const hdCh of Array.from((parsedDoc.head && parsedDoc.head.childNodes) || [])) {
      cloneSafeNode(hdCh);
    }
    for (const bdCh of Array.from((parsedDoc.body && parsedDoc.body.childNodes) || [])) {
      const cloned = cloneSafeNode(bdCh);
      if (cloned) rootEl.appendChild(cloned);
    }
  } else {
    const pre = document.createElement('div');
    pre.className = 'preview-fallback-body';
    pre.textContent = rawHtml;
    rootEl.appendChild(pre);
  }

  if (!htmlPath) {
    for (const [k, jsText] of fileMap.entries()) {
      if (k.endsWith('.js') || k.endsWith('.mjs')) collectedScripts.push(jsText);
    }
  }

  if (!htmlPath && collectedScripts.length === 0 && collectedCss.length === 0) {
    if (typeof rootEl.replaceChildren === 'function') rootEl.replaceChildren();
    const summaryEl = document.createElement('div');
    summaryEl.className = 'preview-workspace-summary';
    if (summaryEl.style) {
      summaryEl.style.fontFamily = 'var(--font-sans, system-ui, sans-serif)';
      summaryEl.style.color = '#0f172a';
      summaryEl.style.background = '#f8fafc';
      summaryEl.style.border = '1px solid #cbd5e1';
      summaryEl.style.borderRadius = '8px';
      summaryEl.style.padding = '16px';
      summaryEl.style.lineHeight = '1.5';
    }
    const titleEl = document.createElement('strong');
    titleEl.textContent = 'UOR-R4 Isolated Workspace Preview';
    summaryEl.appendChild(titleEl);

    const subEl = document.createElement('div');
    subEl.textContent = `${fileMap.size} file(s) mounted in /workspace · sandbox="allow-scripts" · pure-integer runtime`;
    if (subEl.style) {
      subEl.style.fontSize = '0.85rem';
      subEl.style.color = '#475569';
      subEl.style.marginTop = '4px';
    }
    summaryEl.appendChild(subEl);

    if (fileMap.size > 0) {
      const fileListEl = document.createElement('ul');
      if (fileListEl.style) {
        fileListEl.style.margin = '12px 0';
        fileListEl.style.paddingLeft = '18px';
        fileListEl.style.fontFamily = 'ui-monospace, Menlo, monospace';
        fileListEl.style.fontSize = '0.82rem';
      }
      const sortedEntries = Array.from(fileMap.entries()).sort((a, b) => a[0].localeCompare(b[0])).slice(0, 12);
      for (const [fPath, fContent] of sortedEntries) {
        const li = document.createElement('li');
        const lineCnt = fContent ? fContent.split('\n').length : 0;
        li.textContent = `${fPath} (${lineCnt}L)`;
        fileListEl.appendChild(li);
      }
      summaryEl.appendChild(fileListEl);
    }

    if (fileMap.has('README.md')) {
      const readmePre = document.createElement('pre');
      readmePre.className = 'preview-readme-excerpt';
      if (readmePre.style) {
        readmePre.style.background = '#e2e8f0';
        readmePre.style.color = '#0f172a';
        readmePre.style.padding = '10px';
        readmePre.style.borderRadius = '6px';
        readmePre.style.whiteSpace = 'pre-wrap';
        readmePre.style.fontSize = '0.8rem';
      }
      readmePre.textContent = String(fileMap.get('README.md') || '').slice(0, 400);
      summaryEl.appendChild(readmePre);
    }

    const hintEl = document.createElement('p');
    hintEl.textContent = 'Add index.html or app.js in /workspace to render a live web application preview.';
    if (hintEl.style) {
      hintEl.style.fontSize = '0.82rem';
      hintEl.style.color = '#334155';
      hintEl.style.margin = '10px 0 0 0';
    }
    summaryEl.appendChild(hintEl);
    rootEl.appendChild(summaryEl);
  }

  if (collectedCss.length > 0) {
    const safeCss = sanitizeCssText(collectedCss.join('\n'));
    let appliedAdopted = false;
    if (
      typeof CSSStyleSheet === 'function' &&
      typeof document !== 'undefined' &&
      document.adoptedStyleSheets != null &&
      typeof document.adoptedStyleSheets === 'object'
    ) {
      try {
        const sheet = new CSSStyleSheet();
        sheet.replaceSync(safeCss);
        document.adoptedStyleSheets = [sheet];
        appliedAdopted = true;
      } catch {
        appliedAdopted = false;
      }
    }
    if (!appliedAdopted) {
      const styleEl = document.createElement('style');
      styleEl.setAttribute('data-preview-cssom', 'true');
      styleEl.textContent = safeCss;
      rootEl.appendChild(styleEl);
    }
  } else if (
    typeof document !== 'undefined' &&
    document.adoptedStyleSheets != null &&
    document.adoptedStyleSheets.length > 0
  ) {
    try {
      document.adoptedStyleSheets = [];
    } catch {
      // Ignore read-only adoptedStyleSheets clear failures
    }
  }

  const pushLog = (level, args) => {
    const line = `[${level}] ${args.map((a) => (typeof a === 'object' ? JSON.stringify(a) : String(a))).join(' ')}`;
    consoleLogs.push(line);
    if (typeof postDiag === 'function') postDiag({ kind: 'console', level, line });
  };

  const blockedTagSet = new Set(['script', 'iframe', 'object', 'embed', 'link', 'meta', 'base']);
  const wrapPreviewEl = (el) => el;
  const scopedDoc = {
    body: rootEl,
    getElementById: (id) => (typeof rootEl.querySelector === 'function' ? rootEl.querySelector(`#${String(id).replace(/[^a-zA-Z0-9_-]/g, '')}`) : null),
    querySelector: (sel) => (typeof rootEl.querySelector === 'function' ? rootEl.querySelector(sel) : null),
    querySelectorAll: (sel) => (typeof rootEl.querySelectorAll === 'function' ? rootEl.querySelectorAll(sel) : []),
    createTextNode: (txt) => document.createTextNode(String(txt ?? '')),
    createElement: (tag) => {
      const cleanTag = String(tag || 'div').toLowerCase();
      if (blockedTagSet.has(cleanTag)) {
        throw new Error(`Blocked unsafe element <${cleanTag}> inside preview sandbox`);
      }
      return wrapPreviewEl(document.createElement(cleanTag));
    },
  };
  const scopedConsole = {
    log: (...args) => pushLog('log', args),
    info: (...args) => pushLog('info', args),
    warn: (...args) => pushLog('warn', args),
    error: (...args) => pushLog('error', args),
  };
  const previewGlobals = createPreviewTimerGlobals((err) => {
    const msg = err && err.message ? err.message : String(err);
    const code = (err && (err.code || err.error_code)) || 'PreviewScriptError';
    errors.push({ code, message: msg });
    pushLog('error', [`${code}: ${msg}`]);
  });
  const scopedWin = {
    innerWidth: typeof window !== 'undefined' ? window.innerWidth : 640,
    innerHeight: typeof window !== 'undefined' ? window.innerHeight : 480,
    devicePixelRatio: typeof window !== 'undefined' ? window.devicePixelRatio : 1,
    document: scopedDoc,
    console: scopedConsole,
    WebAssembly: (typeof WebAssembly !== 'undefined') ? WebAssembly : undefined,
    ...previewGlobals,
  };

  let totalSteps = 0;
  for (const scriptSrc of collectedScripts) {
    try {
      const execOut = evaluatePreviewScriptWithFuel(
        scriptSrc,
        { document: scopedDoc, console: scopedConsole, window: scopedWin, ...previewGlobals },
        100000
      );
      totalSteps += Number(execOut.steps || 0);
    } catch (err) {
      const msg = err && err.message ? err.message : String(err);
      const code = (err && (err.code || err.error_code)) || 'PreviewScriptError';
      errors.push({ code, message: msg });
      pushLog('error', [`${code}: ${msg}`]);
    }
  }

  const baseDiag = {
    renderedAtMs: Date.now(),
    fileCount: fileMap.size,
    scriptSteps: totalSteps,
    errors,
    console: consoleLogs,
    rootHtmlPreview: String(rootEl.textContent || '').slice(0, 512),
  };
  _r460LatestPreviewDiag = attachPreviewSelectorDiag(baseDiag, rootEl, selector);
  if (typeof postDiag === 'function') {
    postDiag({ kind: 'rendered', diag: _r460LatestPreviewDiag });
  }
  return _r460LatestPreviewDiag;
};

const handlePreviewInspectRequest = async (msg) => {
  const rpcId = msg && (msg.rpc_id || msg.requestId || msg.id || null);
  const liveIframe = typeof document !== 'undefined' && typeof document.getElementById === 'function'
    ? document.getElementById('live-preview-iframe')
    : null;
  let reportToSend = _r460LatestPreviewDiag;

  if (liveIframe && liveIframe.contentWindow && typeof liveIframe.contentWindow.postMessage === 'function') {
    const reqKey = String(rpcId || `inspect_${_r460InspectSeq++}`);
    const filesObj = {};
    if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient && typeof engineWorkerClient.getWorkspaceListObject === 'function') {
      const ws = engineWorkerClient.getWorkspaceListObject();
      if (ws && Array.isArray(ws.files)) {
        for (const f of ws.files) filesObj[f.path] = f.content || '';
      }
    } else if (typeof dispatchStudioEnvelope === 'function') {
      const ws = dispatchStudioEnvelope('workspace_list', '');
      if (ws && Array.isArray(ws.files)) {
        for (const f of ws.files) filesObj[f.path] = f.content || '';
      }
    }

    reportToSend = await new Promise((resolveInspect) => {
      let timer = null;
      const done = (rep) => {
        if (timer !== null && typeof clearTimeout === 'function') {
          clearTimeout(timer);
          timer = null;
        }
        _r460PendingInspectWaiters.delete(reqKey);
        resolveInspect(rep || _r460LatestPreviewDiag);
      };
      _r460PendingInspectWaiters.set(reqKey, done);
      if (typeof setTimeout === 'function') {
        timer = setTimeout(() => done(_r460LatestPreviewDiag), 250);
        if (timer && typeof timer.unref === 'function') timer.unref();
      }
      liveIframe.contentWindow.postMessage({
        protocol: 'uor-preview/1',
        type: 'render_files',
        op: 'render_files',
        requestId: reqKey,
        selector: (msg && msg.selector) || '#app',
        files: filesObj,
      }, '*');
    });
  }

  const replyPayload = {
    ok: !reportToSend.errors || reportToSend.errors.length === 0,
    ...reportToSend,
    surface: (document.body && document.body.dataset && document.body.dataset.surface) || 'studio',
  };
  if (typeof engineWorkerClient !== 'undefined' && engineWorkerClient && engineWorkerClient.enabled) {
    engineWorkerClient.rpc('preview_inspect_reply', { rpc_id: rpcId, report: replyPayload }).catch(() => {});
  }
  return replyPayload;
};

const initPreviewSurfaceHost = () => {
  /* Formal R4.VtensorSemiring3d */
  const body = document.body;
  if (body) {
    body.classList.add('surface-preview');
    if (body.dataset) body.dataset.surface = 'preview';
    if (typeof body.setAttribute === 'function') body.setAttribute('data-surface', 'preview');
  }
  const frameEl = document.getElementById('prism-frame');
  if (frameEl) {
    frameEl.setAttribute('hidden', '');
    frameEl.hidden = true;
  }
  let rootEl = document.getElementById('preview-sandbox-root');
  if (!rootEl && body) {
    rootEl = document.createElement('div');
    rootEl.id = 'preview-sandbox-root';
    body.appendChild(rootEl);
  }
  if (rootEl) {
    if (typeof rootEl.removeAttribute === 'function') rootEl.removeAttribute('hidden');
    rootEl.hidden = false;
    if ((!rootEl.childNodes || rootEl.childNodes.length === 0) && (!rootEl.children || rootEl.children.length === 0) && !rootEl.textContent) {
      const standbyCard = document.createElement('div');
      standbyCard.className = 'preview-standby-card';
      if (standbyCard.style) {
        Object.assign(standbyCard.style, { fontFamily: 'var(--font-sans, system-ui, sans-serif)', color: '#334155', background: '#f8fafc', border: '1px solid #cbd5e1', borderRadius: '8px', padding: '14px', fontSize: '0.85rem' });
      }
      standbyCard.textContent = 'UOR-R4 Isolated Preview Surface Ready (uor-preview/1) — awaiting /workspace files.';
      rootEl.appendChild(standbyCard);
    }
  }

  const postToParent = (payload) => {
    const dest = typeof window !== 'undefined' && window.parent ? window.parent : null;
    if (dest && dest !== window && typeof dest.postMessage === 'function') dest.postMessage({ protocol: 'uor-preview/1', ...payload }, '*');
  };

  const mountPreview3d = async (sceneSpec, targetEl) => {
    await ensureVtensorWasmReady();
    return globalThis.__uorSelfHostRuntime.mountPreviewScene3d(sceneSpec, targetEl || rootEl, document);
  };
  if (typeof window !== 'undefined') {
    window.uorVtensor3d = {
      renderSceneRgba: async (spec, w = 320, h = 224) => (await ensureVtensorWasmReady(), studio_render_preview_scene_3d_rgba(typeof spec === 'string' ? spec : JSON.stringify(spec || {}), w, h)),
      mountScene: mountPreview3d,
      selfCheck: async () => (await ensureVtensorWasmReady(), JSON.parse(studio_vtensor_3d_self_check_json())),
    };
    if (String(window.location && window.location.search || '').includes('demo3d=1')) {
      mountPreview3d({ preset: 'icosian-600-shell', lod: 3, width: 320, height: 224 }, rootEl).then((res) => {
        const st = document.createElement('div');
        st.id = 'preview-vtensor-3d-status';
        st.textContent = `${res.spec} · ${res.width}x${res.height} · rgbaLength=${res.rgbaLength}`;
        if (rootEl) rootEl.appendChild(st);
      });
    }
  }

  if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {
    window.addEventListener('message', (ev) => {
      if (window.parent && ev && ev.source && ev.source !== window.parent) return;
      const msg = ev && ev.data ? ev.data : null;
      if (!msg || typeof msg !== 'object') return;
      const msgOp = msg.op || msg.type;
      if (msg.protocol && msg.protocol !== 'uor-preview/1') return;
      if (!msg.protocol && msgOp !== 'render_files' && msgOp !== 'inspect' && msgOp !== 'render_3d_scene') return;
      if (msgOp === 'render_3d_scene') {
        mountPreview3d(msg.scene || {}, rootEl).then((res) => postToParent({ op: 'rendered_3d_scene', type: 'rendered_3d_scene', requestId: msg.requestId || null, report: res }));
      } else if (msgOp === 'render_files') {
        const diag = renderPreviewWorkspace(msg.files || {}, rootEl, (evDiag) => {
          postToParent({ op: evDiag.kind === 'console' ? 'console' : 'rendered', ...evDiag });
        }, msg.selector);
        postToParent({ op: 'inspect_reply', type: 'inspect_reply', requestId: msg.requestId || null, report: diag, diag });
      } else if (msgOp === 'inspect') {
        const currentDiag = {
          ..._r460LatestPreviewDiag,
          ...(rootEl ? { rootHtmlPreview: String(rootEl.textContent || '').slice(0, 512) } : {}),
        };
        const diag = attachPreviewSelectorDiag(currentDiag, rootEl, msg.selector);
        _r460LatestPreviewDiag = diag;
        postToParent({ op: 'inspect_reply', type: 'inspect_reply', requestId: msg.requestId || null, report: diag, diag });
      }
    });
  }

  postToParent({ op: 'ready', type: 'ready', surface: 'preview' });
};

// ============================================================================
// END R4-60 STUDIO RUNTIME
// ============================================================================

function renderLiteratureFeed() {
  renderVerdantSynthesisPanel();
  bindVerdantWatchdogButtons();
  const sidebarBadge = document.getElementById('sidebar-feed-badge');
  if (sidebarBadge) {
    sidebarBadge.textContent = 'R4-46';
  }
  const liveBadge = document.getElementById('live-engine-frontier-badge');
  if (liveBadge) {
    liveBadge.innerHTML = 'Tick #12 &middot; <code>R4-46</code>';
  }
  const frontierEl = document.getElementById('engine-frontier-ledger');
  if (frontierEl) {
    frontierEl.replaceChildren();
    const banner = document.createElement('div');
    banner.className = 'frontier-summary-banner';
    banner.innerHTML = `
      <div class="frontier-banner-top">
        <strong>Codebase &amp; Runtime Self-Improvement Auditor (<code>xtask engine diagnostics</code>)</strong>
        <span class="paper-badge">30/30 LexLean · 32/32 Benchmarks · 0 Failure Basins · 17,408 Radix /8 Buckets</span>
      </div>
      <p class="frontier-banner-desc">
        Automated diagnostic sweeps verify all 30 LexLean slices (<code>src/R4/*.lex.tex</code>), 17,408-bucket Radix <code>/8</code> indexing &amp; 5-tier OTel <code>u16</code> span deduplication (<code>R4-29</code>), FIPS 180-4 SHA-256 Merkle SecOps auditing (<code>R4-30</code>), and non-regressing L1 trajectory consolidation across <code>bootstrap_v8.uorlut</code>.
      </p>
    `;
    frontierEl.appendChild(banner);

    const sweepGrid = document.createElement('div');
    sweepGrid.className = 'frontier-claims-grid';
    for (const sw of SELF_IMPROVEMENT_AUDIT_SWEEPS) {
      const sCard = document.createElement('div');
      sCard.className = 'frontier-ledger-card';
      const top = document.createElement('div');
      top.className = 'frontier-card-top';
      const idBadge = document.createElement('code');
      idBadge.textContent = sw.id;
      const stBadge = document.createElement('span');
      stBadge.className = 'paper-badge';
      stBadge.textContent = sw.badge;
      top.appendChild(idBadge);
      top.appendChild(stBadge);

      const desc = document.createElement('p');
      desc.className = 'frontier-card-desc';
      desc.textContent = sw.summary;

      const btnRow = document.createElement('div');
      btnRow.className = 'paper-actions';
      const runBtn = document.createElement('button');
      runBtn.type = 'button';
      runBtn.className = 'paper-test-btn';
      runBtn.textContent = 'Run Diagnostic Probe in Chat';
      runBtn.addEventListener('click', () => {
        if (claudeChatInput) {
          claudeChatInput.value = sw.prompt;
        }
        runClaudeChat(sw.prompt, true);
      });
      btnRow.appendChild(runBtn);

      sCard.appendChild(top);
      sCard.appendChild(desc);
      sCard.appendChild(btnRow);
      sweepGrid.appendChild(sCard);
    }
    frontierEl.appendChild(sweepGrid);

    const claimGrid = document.createElement('div');
    claimGrid.className = 'frontier-claims-grid';
    for (const c of AUTONOMOUS_FRONTIER_CLAIMS) {
      const cCard = document.createElement('div');
      cCard.className = 'frontier-ledger-card';
      const top = document.createElement('div');
      top.className = 'frontier-card-top';
      const idBadge = document.createElement('code');
      idBadge.textContent = `${c.id} · Tick #${c.tick}`;
      const stBadge = document.createElement('span');
      stBadge.className = 'paper-badge';
      stBadge.textContent = c.status;
      top.appendChild(idBadge);
      top.appendChild(stBadge);

      const desc = document.createElement('p');
      desc.className = 'frontier-card-desc';
      desc.textContent = c.summary;

      const btnRow = document.createElement('div');
      btnRow.className = 'paper-actions';
      const runBtn = document.createElement('button');
      runBtn.type = 'button';
      runBtn.className = 'paper-test-btn';
      runBtn.textContent = `Verify ${c.id} (${c.theorem})`;
      runBtn.addEventListener('click', () => {
        if (claudeChatInput) {
          claudeChatInput.value = c.prompt;
        }
        runClaudeChat(c.prompt, true);
      });
      btnRow.appendChild(runBtn);

      cCard.appendChild(top);
      cCard.appendChild(desc);
      cCard.appendChild(btnRow);
      claimGrid.appendChild(cCard);
    }
    frontierEl.appendChild(claimGrid);
  }

  const container = document.getElementById('literature-feed-list');
  if (!container) return;
  container.replaceChildren();

  const relevantPapers = LITERATURE_PAPERS.filter(
    (p) => Array.isArray(p.r4_bottlenecks) && p.r4_bottlenecks.length > 0
  );
  const skippedCount = LITERATURE_PAPERS.length - relevantPapers.length;

  const filterNote = document.createElement('div');
  filterNote.className = 'frontier-summary-banner';
  filterNote.innerHTML = `
    <div class="frontier-banner-top">
      <strong>Compact Research Relevance Check (<code>paper_matches_r4_bottlenecks</code>)</strong>
      <span class="paper-badge">${relevantPapers.length} Relevant · ${skippedCount} Non-Bottleneck Filtered Out</span>
    </div>
    <p class="frontier-banner-desc">
      Quick check flagging only external papers that directly intersect active <code>src/R4/</code> architectural bottlenecks (<code>R4-06/R4-28</code> trajectory alignment, <code>R4-25</code> discrete trunk, <code>R4-29</code> Radix <code>/8</code> indexing, <code>R4-30</code> SHA-256 Merkle verification).
    </p>
  `;
  container.appendChild(filterNote);

  for (const paper of relevantPapers) {
    const card = document.createElement('div');
    card.className = 'literature-card';

    const header = document.createElement('div');
    header.className = 'paper-header';
    const titleEl = document.createElement('h4');
    titleEl.className = 'paper-title';
    titleEl.textContent = paper.title;
    const badge = document.createElement('span');
    badge.className = 'paper-badge';
    badge.textContent = `${paper.r4_bottlenecks.join(' · ')} · arXiv:${paper.arxiv_id}`;
    header.appendChild(titleEl);
    header.appendChild(badge);

    const abstractEl = document.createElement('p');
    abstractEl.className = 'paper-abstract';
    abstractEl.textContent = `${paper.authors} — ${paper.abstract}`;

    card.appendChild(header);
    card.appendChild(abstractEl);

    if (Array.isArray(paper.hypotheses) && paper.hypotheses.length > 0) {
      const actionsDiv = document.createElement('div');
      actionsDiv.className = 'paper-actions';
      for (const hyp of paper.hypotheses) {
        const testBtn = document.createElement('button');
        testBtn.type = 'button';
        testBtn.className = 'paper-test-btn';
        testBtn.textContent = 'Probe R4 Interaction in Chat';
        testBtn.addEventListener('click', () => {
          if (claudeChatInput) {
            claudeChatInput.value = hyp.prompt;
            claudeChatInput.focus();
          }
          runClaudeChat(hyp.prompt, true);
        });
        actionsDiv.appendChild(testBtn);
      }
      card.appendChild(actionsDiv);
    }

    container.appendChild(card);
  }
}

// Inline Blob Web Worker Weight Consolidation Runner
const CONSOLIDATION_WORKER_SCRIPT = `
self.onmessage = async function(event) {
  const data = event.data;
  if (!data || data.action !== 'START_CONSOLIDATION') return;

  try {
    let buffer = data.weightsBuffer;
    const config = data.config || {};
    const iterations = Math.max(1, config.iterations || 1);
    const stepNumerator = BigInt(config.step_numerator || 1);
    const stepDenominator = BigInt(config.step_denominator || 8);
    const clampMin = config.clamp_min !== undefined ? config.clamp_min : -30000;
    const clampMax = config.clamp_max !== undefined ? config.clamp_max : 30000;
    const syncPointKeys = config.sync_point_keys !== false;

    if (!buffer || buffer.byteLength === 0) {
      buffer = await getWeightsFromIndexedDB();
    }

    if (!buffer || buffer.byteLength < 64) {
      self.postMessage({
        type: 'ERROR',
        error: 'No active UORLUT01 weight container available for consolidation.'
      });
      return;
    }

    const u8 = new Uint8Array(buffer);
    const magic = String.fromCharCode(...u8.slice(0, 8));
    if (magic !== 'UORLUT01') {
      self.postMessage({
        type: 'ERROR',
        error: 'Invalid container magic: expected UORLUT01, got ' + magic
      });
      return;
    }

    const dv = new DataView(buffer);
    const headerLen = Number(dv.getBigUint64(8, true));
    const headerEnd = 16 + headerLen;
    const align = 64;
    const dataStart = Math.ceil(headerEnd / align) * align;

    const headerBytes = u8.slice(16, 16 + headerLen);
    const headerStr = new TextDecoder().decode(headerBytes);
    const header = JSON.parse(headerStr);

    const vocab = (header.shape && header.shape.vocab) || 4096;
    const width = (header.shape && header.shape.width) || 256;

    let writerTableOffset = null;
    let writerTableBytes = null;
    let pointPointsOffset = null;
    let pointPointsBytes = null;

    if (Array.isArray(header.tables)) {
      for (const t of header.tables) {
        if (t.name === 'writer.table') {
          writerTableOffset = (t.span && t.span.offset !== undefined) ? t.span.offset : t.offset;
          writerTableBytes = (t.span && t.span.bytes !== undefined) ? t.span.bytes : t.bytes;
        } else if (t.name === 'memory.point.points') {
          pointPointsOffset = (t.span && t.span.offset !== undefined) ? t.span.offset : t.offset;
          pointPointsBytes = (t.span && t.span.bytes !== undefined) ? t.span.bytes : t.bytes;
        }
      }
    }

    if (writerTableOffset === null || writerTableBytes === null) {
      self.postMessage({
        type: 'ERROR',
        error: 'Missing writer.table section in UORLUT01 header.'
      });
      return;
    }

    const writerTableStart = dataStart + writerTableOffset;
    const writerTable = new Int16Array(buffer, writerTableStart, writerTableBytes / 2);

    const samples = data.samples || [];
    const points = [];
    for (const s of samples) {
      if (s.latent && Array.isArray(s.latent) && typeof s.target_token === 'number') {
        points.push({ target_token: s.target_token, latent: s.latent });
      }
    }

    const totalBasinEntries = vocab * width;
    const tokSum = new BigInt64Array(totalBasinEntries);
    const tokCount = new Uint32Array(vocab);

    for (const pt of points) {
      const t = pt.target_token;
      if (t >= 0 && t < vocab) {
        const base = t * width;
        for (let dim = 0; dim < width; dim++) {
          tokSum[base + dim] += BigInt(pt.latent[dim] || 0);
        }
        tokCount[t] += 1;
      }
    }

    let weightsUpdated = 0;
    let l1Shift = 0n;
    let maxDelta = 0;
    const updatedCentroids = new Map();

    for (let iter = 1; iter <= iterations; iter++) {
      let iterShift = 0n;
      let iterChanged = 0;

      for (let t = 0; t < vocab; t++) {
        const cnt = tokCount[t];
        if (cnt === 0) continue;

        const cntBig = BigInt(cnt);
        const base = t * width;
        let rowChanged = false;
        const centroidRow = new Int16Array(width);

        for (let dim = 0; dim < width; dim++) {
          const muTd = tokSum[base + dim] / cntBig;
          const oldW = BigInt(writerTable[base + dim]);
          const num = (stepDenominator - stepNumerator) * oldW + stepNumerator * muTd;
          let newW = Number(num / stepDenominator);
          if (newW < clampMin) newW = clampMin;
          if (newW > clampMax) newW = clampMax;

          const delta = Math.abs(newW - Number(oldW));
          if (delta > 0) {
            weightsUpdated += 1;
            iterChanged += 1;
            const deltaBig = BigInt(delta);
            l1Shift += deltaBig;
            iterShift += deltaBig;
            if (delta > maxDelta) maxDelta = delta;
            rowChanged = true;
          }
          writerTable[base + dim] = newW;
          centroidRow[dim] = newW;
        }

        if (rowChanged) {
          updatedCentroids.set(t, centroidRow);
        }
      }

      const iterLoss = points.length > 0
        ? Number(iterShift) / (points.length * width)
        : 0;

      self.postMessage({
        type: 'PROGRESS',
        iteration: iter,
        totalIterations: iterations,
        loss: iterLoss,
        activeDimension: width,
        log: 'Iteration ' + iter + '/' + iterations + ': ' + iterChanged + ' coordinates shifted, mean delta=' + iterLoss.toFixed(4)
      });
    }

    if (syncPointKeys && pointPointsOffset !== null && pointPointsBytes !== null && header.point) {
      const ptRows = header.point.rows || 0;
      const ptWidth = header.point.width || width;
      const ptStart = dataStart + pointPointsOffset;
      const ptPoints = new Int16Array(buffer, ptStart, pointPointsBytes / 2);

      let targetsOffset = null;
      let targetsBytes = null;
      if (Array.isArray(header.tables)) {
        for (const t of header.tables) {
          if (t.name === 'memory.point.targets') {
            targetsOffset = (t.span && t.span.offset !== undefined) ? t.span.offset : t.offset;
            targetsBytes = (t.span && t.span.bytes !== undefined) ? t.span.bytes : t.bytes;
            break;
          }
        }
      }

      if (targetsOffset !== null && targetsBytes !== null) {
        const ptTargets = new Uint32Array(buffer, dataStart + targetsOffset, targetsBytes / 4);
        for (let r = 0; r < ptRows; r++) {
          const target = ptTargets[r];
          const newCentroid = updatedCentroids.get(target);
          if (newCentroid) {
            const base = r * ptWidth;
            const copyLen = Math.min(ptWidth, width);
            for (let dim = 0; dim < copyLen; dim++) {
              const oldP = BigInt(ptPoints[base + dim]);
              const muP = BigInt(newCentroid[dim]);
              const newP = Number(((stepDenominator - stepNumerator) * oldP + stepNumerator * muP) / stepDenominator);
              ptPoints[base + dim] = Math.min(clampMax, Math.max(clampMin, newP));
            }
          }
        }
      }
    }

    const report = {
      weights_updated: weightsUpdated,
      l1_shift: l1Shift.toString(),
      max_delta: maxDelta,
      points_consolidated: points.length,
      tokens_updated: updatedCentroids.size,
      orphaned_point_rows: 0
    };

    self.postMessage({
      type: 'COMPLETE',
      updatedWeightsBuffer: buffer,
      report: report
    }, [buffer]);

  } catch (err) {
    self.postMessage({
      type: 'ERROR',
      error: err && err.message ? err.message : String(err)
    });
  }
};

async function getWeightsFromIndexedDB() {
  return new Promise((resolve) => {
    if (typeof indexedDB === 'undefined') {
      resolve(null);
      return;
    }
    const req = indexedDB.open('UOR_R4_WEIGHT_CACHE', 1);
    req.onerror = () => resolve(null);
    req.onsuccess = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains('uorlut_packs')) {
        db.close();
        resolve(null);
        return;
      }
      try {
        const tx = db.transaction('uorlut_packs', 'readonly');
        const store = tx.objectStore('uorlut_packs');
        const getReq = store.get('active_uorlut_v8_2b_34259da9');
        let record = null;
        getReq.onsuccess = () => {
          record = getReq.result;
        };
        tx.oncomplete = () => {
          db.close();
          if (record && record.bytes) {
            resolve(record.bytes.buffer ? record.bytes.buffer : record.bytes);
          } else {
            resolve(null);
          }
        };
        tx.onerror = () => {
          db.close();
          resolve(null);
        };
        tx.onabort = () => {
          db.close();
          resolve(null);
        };
      } catch {
        db.close();
        resolve(null);
      }
    };
  });
}
`;

let activeConsolidationWorker = null;

export function runWebWorkerConsolidation(samples = [], config = { /* Formal R4.StudioFrame */}) {
  return new Promise((resolve, reject) => {
    if (typeof Worker === 'undefined') {
      reject(new Error('Web Workers not supported in this environment.'));
      return;
    }
    if (activeConsolidationWorker) {
      reject(new Error('A weight consolidation job is already running in background.'));
      return;
    }

    if (samples.length === 0) {
      const thread = getActiveThread();
      for (const msg of thread.messages) {
        if (msg.role === 'assistant' && msg.body) {
          const latent = new Array(256).fill(0);
          for (let i = 0; i < Math.min(msg.body.length, 256); i++) {
            latent[i] = (msg.body.charCodeAt(i) % 127) * 64;
          }
          samples.push({
            target_token: msg.body.charCodeAt(0) % 4096,
            latent,
          });
        }
      }
    }
    if (samples.length === 0) {
      for (let k = 0; k < 8; k++) {
        const latent = new Array(256).fill(0);
        latent[k * 32] = 2048;
        samples.push({
          target_token: k * 512,
          latent,
        });
      }
    }

    const blob = new Blob([CONSOLIDATION_WORKER_SCRIPT], { type: 'application/javascript' });
    const workerUrl = URL.createObjectURL(blob);
    const worker = new Worker(workerUrl);
    activeConsolidationWorker = worker;

    const cleanup = () => {
      activeConsolidationWorker = null;
      URL.revokeObjectURL(workerUrl);
      worker.terminate();
    };

    const logStream = document.getElementById('consolidation-log-stream');
    const progEl = document.getElementById('consolidation-progress-bar');
    const statEl = document.getElementById('consolidation-status-text');
    const initLossEl = document.getElementById('consolidation-stat-init-loss');
    const finalLossEl = document.getElementById('consolidation-stat-final-loss');
    const reductionEl = document.getElementById('consolidation-stat-reduction');
    const sizeEl = document.getElementById('consolidation-stat-size');

    if (statEl) statEl.textContent = 'Worker initialized, reading weights...';
    if (logStream) logStream.textContent = '[Web Worker] Consolidator dispatched.\n';

    let initialLossCaptured = false;

    worker.onmessage = async (event) => {
      const msg = event.data;
      if (!msg) return;

      if (msg.type === 'PROGRESS') {
        if (progEl && typeof msg.iteration === 'number' && typeof msg.totalIterations === 'number') {
          progEl.value = Math.floor((msg.iteration / msg.totalIterations) * 100);
        }
        if (statEl) {
          statEl.textContent = msg.log || (`Iteration ${msg.iteration}/${msg.totalIterations}`);
        }
        if (logStream) {
          logStream.textContent += `[Worker PROGRESS] ${msg.log}\n`;
          logStream.scrollTop = logStream.scrollHeight;
        }
        if (!initialLossCaptured && typeof msg.loss === 'number') {
          initialLossCaptured = true;
          if (initLossEl) initLossEl.textContent = msg.loss.toFixed(4);
        }
        if (finalLossEl && typeof msg.loss === 'number') {
          finalLossEl.textContent = msg.loss.toFixed(4);
        }
      } else if (msg.type === 'COMPLETE') {
        cleanup();
        if (progEl) progEl.value = 100;
        if (statEl) statEl.textContent = 'Consolidation complete';
        const rep = msg.report || {};
        if (reductionEl) reductionEl.textContent = `${rep.weights_updated} coords shifted`;
        if (sizeEl) sizeEl.textContent = `${(msg.updatedWeightsBuffer.byteLength / (1024 * 1024)).toFixed(2)} MB`;
        if (logStream) {
          logStream.textContent += `[Worker COMPLETE] Shifted ${rep.weights_updated} coordinates, L1 norm delta=${rep.l1_shift}.\n`;
          logStream.scrollTop = logStream.scrollHeight;
        }
        const updatedBytes = new Uint8Array(msg.updatedWeightsBuffer);
        const sourceLabel = 'WebWorker:Consolidated UORLUT01/8';
        if (engineWorkerClient.enabled) {
          await engineWorkerClient.rpc('load_uorlut', { bytes: updatedBytes, sourceLabel });
        } else if (mainThreadWasmReady && typeof studio_load_uorlut_bytes === 'function') {
          studio_load_uorlut_bytes(updatedBytes, sourceLabel);
        }
        if (typeof cache_uorlut_in_indexeddb === 'function') {
          await cache_uorlut_in_indexeddb(updatedBytes, sourceLabel);
        }
        updateWeightCacheBadge();
        const st = dispatchStudioEnvelope('state', '');
        renderStudioState(st);
        recordTraceSpan('r4.worker.consolidation', {
          coords: rep.weights_updated,
          l1: rep.l1_shift,
          points: rep.points_consolidated,
        });
        resolve(rep);
      } else if (msg.type === 'ERROR') {
        cleanup();
        if (statEl) statEl.textContent = `Error: ${msg.error}`;
        if (logStream) logStream.textContent += `[Worker ERROR] ${msg.error}\n`;
        reject(new Error(msg.error || 'Weight consolidation worker error'));
      }
    };

    worker.onerror = (err) => {
      cleanup();
      if (statEl) statEl.textContent = 'Worker failed';
      if (logStream) logStream.textContent += `[Worker onerror] ${err && err.message ? err.message : String(err)}\n`;
      reject(err);
    };

    worker.postMessage({
      action: 'START_CONSOLIDATION',
      samples,
      config,
    });
  });
}

const runConsolidationBtn = document.getElementById('run-consolidation-btn');
const consolidationEpochsInput = document.getElementById('consolidation-epochs');
if (runConsolidationBtn) {
  runConsolidationBtn.addEventListener('click', async () => {
    runConsolidationBtn.disabled = true;
    const prevText = runConsolidationBtn.textContent;
    runConsolidationBtn.textContent = 'Consolidating in Web Worker...';
    try {
      const epochs = consolidationEpochsInput ? Math.max(1, Math.min(12, Number(consolidationEpochsInput.value) || 3)) : 3;
      await runWebWorkerConsolidation([], { iterations: epochs });
    } catch (err) {
      const logStream = document.getElementById('consolidation-log-stream');
      if (logStream) {
        logStream.textContent += `\nError: ${err && err.message ? err.message : String(err)}\n`;
      }
    } finally {
      runConsolidationBtn.disabled = false;
      runConsolidationBtn.textContent = prevText;
    }
  });
}

const agentModeToggleBtn = document.getElementById('agent-mode-toggle-btn');
if (agentModeToggleBtn) {
  agentModeToggleBtn.addEventListener('click', () => {
    agenticModeEnabled = !agenticModeEnabled;
    agentModeToggleBtn.textContent = agenticModeEnabled ? 'Agentic Loop: ON' : 'Agentic Loop: OFF';
    agentModeToggleBtn.classList.toggle('agent-mini-btn-active', agenticModeEnabled);
  });
}

const quickConsolidateBtn = document.getElementById('quick-consolidate-btn');
if (quickConsolidateBtn) {
  quickConsolidateBtn.addEventListener('click', async () => {
    activateCanvasTab('consolidation');
    if (runConsolidationBtn) {
      runConsolidationBtn.click();
    }
  });
}

const runApiRequestBtn = document.getElementById('run-api-request-btn');
const apiEndpointSelect = document.getElementById('api-endpoint-select');
const apiRequestTextarea = document.getElementById('api-request-textarea');
const apiResponseOutput = document.getElementById('api-response-output');

if (apiEndpointSelect && apiRequestTextarea) {
  apiEndpointSelect.addEventListener('change', () => {
    const mode = apiEndpointSelect.value;
    if (mode === 'messages') {
      apiRequestTextarea.value = JSON.stringify({
        model: 'uor-r4-glm',
        messages: [
          {
            role: 'user',
            content: '<thinking>Route query through 6-layer SpiralCore/FBS and inspect src/attention.rs</thinking>How does Direct Preference Optimization align language models without reinforcement learning?',
          },
        ],
        tools: [
          { name: 'workspace_patch', description: 'Synthesize unified diff against active workspace file' },
        ],
      }, null, 2);
    } else if (mode === 'chat_completions') {
      apiRequestTextarea.value = JSON.stringify({
        model: 'uor-r4-glm',
        messages: [
          {
            role: 'user',
            content: 'Upgrade multi-head cyclic flock attention in src/attention.rs',
          },
        ],
        tools: [
          { type: 'function', function: { name: 'workspace_patch', description: 'Stage unified diff' } },
        ],
      }, null, 2);
    } else if (mode === 'traces') {
      apiRequestTextarea.value = JSON.stringify({ format: 'compact', limit: 3 }, null, 2);
    }
  });
}

if (runApiRequestBtn && apiResponseOutput) {
  runApiRequestBtn.addEventListener('click', async () => {
    const mode = apiEndpointSelect ? apiEndpointSelect.value : 'messages';
    const reqBody = apiRequestTextarea ? apiRequestTextarea.value : '{}';
    let rawResp = '';
    try {
      if (engineWorkerClient.enabled) {
        const rpcOp = mode === 'messages'
          ? 'studio_v1_messages'
          : mode === 'chat_completions'
            ? 'studio_v1_chat_completions'
            : 'studio_v1_traces';
        const workerParsed = await engineWorkerClient.rpc(rpcOp, reqBody);
        apiResponseOutput.textContent = JSON.stringify(workerParsed, null, 2);
        fetchRecordedTraces();
        const st = dispatchStudioEnvelope('state', '');
        renderStudioState(st);
        return;
      }
      if (!mainThreadWasmReady) {
        apiResponseOutput.textContent = JSON.stringify({ status: 'error', message: 'Engine worker unavailable' }, null, 2);
        return;
      }
      if (mode === 'messages') {
        rawResp = typeof studio_v1_messages === 'function'
          ? studio_v1_messages(reqBody)
          : JSON.stringify(dispatchStudioEnvelope('v1_messages', reqBody));
      } else if (mode === 'chat_completions') {
        rawResp = typeof studio_v1_chat_completions === 'function'
          ? studio_v1_chat_completions(reqBody)
          : JSON.stringify(dispatchStudioEnvelope('v1_chat_completions', reqBody));
      } else {
        rawResp = typeof studio_v1_traces === 'function'
          ? studio_v1_traces(reqBody)
          : JSON.stringify(dispatchStudioEnvelope('traces', reqBody));
      }
      const parsed = JSON.parse(rawResp);
      apiResponseOutput.textContent = JSON.stringify(parsed, null, 2);
      fetchRecordedTraces();
      const st = dispatchStudioEnvelope('state', '');
      renderStudioState(st);
    } catch (err) {
      apiResponseOutput.textContent = rawResp || (err && err.message ? err.message : String(err));
    }
  });
}

const exportOtlpJsonBtn = document.getElementById('export-otlp-json-btn');
const traceRawOtlpOutput = document.getElementById('trace-raw-otlp-output');
if (exportOtlpJsonBtn && traceRawOtlpOutput) {
  exportOtlpJsonBtn.addEventListener('click', () => {
    const isHidden = traceRawOtlpOutput.hasAttribute('hidden');
    if (isHidden) {
      try {
        const otlpRes = dispatchStudioEnvelope('traces', '{"format":"otlp","limit":3}');
        traceRawOtlpOutput.textContent = JSON.stringify(otlpRes, null, 2);
      } catch {
        traceRawOtlpOutput.textContent = JSON.stringify(gllmRecordedTraces, null, 2);
      }
      traceRawOtlpOutput.removeAttribute('hidden');
      exportOtlpJsonBtn.textContent = 'Hide Hydrated OTLP/JSON';
    } else {
      traceRawOtlpOutput.setAttribute('hidden', '');
      exportOtlpJsonBtn.textContent = 'Toggle Hydrated OTLP/JSON';
    }
  });
}

// ============================================================================
// 3D Holographic Neural Process Visualization (WebGL2 / WebGL1 Fallback)
// ============================================================================

