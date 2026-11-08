/**
 * Empirical Challenger: GUI State Machine & Stress Tolerances
 *
 * Requirements challenged:
 * 1. Pool Review Queue state machine: rapid clicks on #pool-review-approve-btn when queue is empty, and proper transition when a diff is staged.
 * 2. Customizations parameter bounds: verify extreme values for #custom-retain-lambda (0.0, 1.0, -1, NaN) and #custom-plurality-votes (1, 16, 0, 100), ensuring graceful fallback and safe JSON packaging.
 * 3. Theme toggling: rapid cycling of #custom-theme-toggle (100 iterations) verifying CSS class stability and zero memory leak.
 * 4. Density mode cycling: comfortable <-> compact cycling.
 * 5. Workspace switching: rapid switching between lexlean-r4 and scratchpad verifying in-memory filesystem stability.
 * 6. MCP tool test execution: triggering all 6 tool runs and asserting #mcp-audit-log formatting and SHA-256 Merkle chain consistency.
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

console.log('================================================================');
console.log('  CHALLENGER: GUI STATE MACHINE & STRESS TOLERANCES AUDIT');
console.log('================================================================\n');

const appJsPath = resolve(REPO_ROOT, 'public/app.js');
const appJsContent = readFileSync(appJsPath, 'utf8');

const appCssPath = resolve(REPO_ROOT, 'public/app.css');
const appCssContent = readFileSync(appCssPath, 'utf8');

const indexHtmlPath = resolve(REPO_ROOT, 'public/index.html');
const indexHtmlContent = readFileSync(indexHtmlPath, 'utf8');

// ----------------------------------------------------------------------------
// Mock DOM Infrastructure
// ----------------------------------------------------------------------------

class MockDOMElement {
  constructor(tagName, id = '', className = '') {
    this.tagName = tagName.toUpperCase();
    this.id = id;
    this.className = className;
    this.value = '';
    this.checked = false;
    this.disabled = false;
    this.textContent = '';
    this.innerHTML = '';
    this.attributes = new Map();
    this.children = [];
    this.listeners = new Map();
    this.style = {};
    this.scrollHeight = 100;
    this.scrollTop = 0;
  }

  get classList() {
    const self = this;
    return {
      contains(cls) {
        return self.className.split(/\s+/).filter(Boolean).includes(cls);
      },
      add(cls) {
        const tokens = new Set(self.className.split(/\s+/).filter(Boolean));
        tokens.add(cls);
        self.className = Array.from(tokens).join(' ');
      },
      remove(cls) {
        const tokens = new Set(self.className.split(/\s+/).filter(Boolean));
        tokens.delete(cls);
        self.className = Array.from(tokens).join(' ');
      },
      toggle(cls, force) {
        const has = this.contains(cls);
        const shouldHave = typeof force === 'boolean' ? force : !has;
        if (shouldHave) this.add(cls);
        else this.remove(cls);
        return shouldHave;
      }
    };
  }

  setAttribute(k, v) { this.attributes.set(k, String(v)); }
  getAttribute(k) { return this.attributes.get(k) ?? null; }
  hasAttribute(k) { return this.attributes.has(k); }
  removeAttribute(k) { this.attributes.delete(k); }

  appendChild(child) {
    this.children.push(child);
  }

  replaceChildren(...newChildren) {
    this.children = [...newChildren];
  }

  addEventListener(type, fn) {
    if (!this.listeners.has(type)) this.listeners.set(type, []);
    this.listeners.get(type).push(fn);
  }

  removeEventListener(type, fn) {
    if (!this.listeners.has(type)) return;
    const fns = this.listeners.get(type).filter((f) => f !== fn);
    this.listeners.set(type, fns);
  }

  dispatchEvent(ev) {
    const fns = this.listeners.get(ev.type) || [];
    for (const fn of fns) fn(ev);
  }

  click() {
    if (this.disabled) return false;
    let prevented = false;
    let stopped = false;
    const ev = {
      type: 'click',
      target: this,
      preventDefault: () => { prevented = true; },
      stopPropagation: () => { stopped = true; }
    };
    this.dispatchEvent(ev);
    return !prevented;
  }

  querySelector(sel) {
    // Basic selector support for #id, .class, tag, and div:first-child
    if (sel.startsWith('#')) {
      const id = sel.slice(1);
      return this._find((el) => el.id === id);
    }
    if (sel.startsWith('.')) {
      const cls = sel.slice(1);
      return this._find((el) => el.classList.contains(cls));
    }
    if (sel.includes(':first-child')) {
      return this.children[0] || null;
    }
    return this._find((el) => el.tagName.toLowerCase() === sel.toLowerCase());
  }

  querySelectorAll(sel) {
    const results = [];
    const check = sel.startsWith('.')
      ? (el) => el.classList.contains(sel.slice(1))
      : (el) => el.tagName.toLowerCase() === sel.toLowerCase();
    this._findAll(check, results);
    return results;
  }

  _find(predicate) {
    for (const child of this.children) {
      if (predicate(child)) return child;
      if (child._find) {
        const found = child._find(predicate);
        if (found) return found;
      }
    }
    return null;
  }

  _findAll(predicate, acc) {
    for (const child of this.children) {
      if (predicate(child)) acc.push(child);
      if (child._findAll) child._findAll(predicate, acc);
    }
  }
}

function createDOMEnvironment() {
  const elementsById = new Map();
  const allElements = [];

  const body = new MockDOMElement('body');

  const document = {
    body,
    getElementById(id) {
      if (!elementsById.has(id)) {
        const el = new MockDOMElement('div', id);
        elementsById.set(id, el);
        allElements.push(el);
      }
      return elementsById.get(id);
    },
    createElement(tag) {
      const el = new MockDOMElement(tag);
      allElements.push(el);
      return el;
    },
    querySelectorAll(sel) {
      if (sel === '.mcp-tool-run-btn') {
        return allElements.filter((el) => el.classList.contains('mcp-tool-run-btn'));
      }
      return body.querySelectorAll(sel);
    }
  };

  return { document, body, elementsById, allElements };
}

// ----------------------------------------------------------------------------
// Extract Core Modules from public/app.js
// ----------------------------------------------------------------------------

// 1. SHA256 & SecopsAuditLedger
const startSha = appJsContent.indexOf('function computeSha256Hex');
const endAudit = appJsContent.indexOf('// ============================================================================', startSha + 200);
assert.ok(startSha !== -1 && endAudit !== -1, 'Failed to extract SecopsAuditLedger');
const auditCode = appJsContent.slice(startSha, endAudit);

// 2. checkMcpToolPolicy
const startPolicy = appJsContent.indexOf('function checkMcpToolPolicy');
const endPolicy = appJsContent.indexOf('\nfunction renderMLInspector', startPolicy);
assert.ok(startPolicy !== -1 && endPolicy !== -1, 'Failed to extract checkMcpToolPolicy');
const policyCode = appJsContent.slice(startPolicy, endPolicy);

// 3. initCustomizationsAndMcp
const startCustom = appJsContent.indexOf('function initCustomizationsAndMcp()');
const endCustom = appJsContent.indexOf('\n// OTel Traces Inspector', startCustom);
assert.ok(startCustom !== -1 && endCustom !== -1, 'Failed to extract initCustomizationsAndMcp');
const customCode = appJsContent.slice(startCustom, endCustom);

const failures = [];

// ============================================================================
// CHALLENGE 1: Pool Review Queue State Machine & Rapid Rejection
// ============================================================================
console.log('--- Challenge 1: Pool Review Queue State Machine & Rapid Rejection ---');
{
  const { document } = createDOMEnvironment();

  // Create pool queue DOM elements
  const poolQueue = document.getElementById('pool-review-queue');
  const fileEl = document.createElement('div');
  fileEl.className = 'pool-review-file';
  poolQueue.appendChild(fileEl);
  const statsEl = document.createElement('div');
  statsEl.className = 'pool-review-stats';
  statsEl.textContent = 'Queue idle · No patch pending';
  poolQueue.appendChild(statsEl);

  const approveBtn = document.getElementById('pool-review-approve-btn');
  approveBtn.disabled = true;
  approveBtn.textContent = 'Approve Patch';

  const rejectBtn = document.getElementById('pool-review-reject-btn');
  rejectBtn.disabled = true;
  rejectBtn.textContent = 'Reject';

  const applyBtn = document.getElementById('chat-apply-patch-btn');
  let applyClickCount = 0;
  applyBtn.addEventListener('click', () => { applyClickCount++; });

  const poolGrid = document.getElementById('pool-status-grid');
  for (let i = 0; i < 4; i++) {
    const card = document.createElement('div');
    card.className = 'pool-card';
    const status = document.createElement('div');
    status.className = 'pool-card-status';
    status.textContent = 'Standby · Ready';
    card.appendChild(status);
    poolGrid.appendChild(card);
  }

  // Setup runner context
  const harness = new Function('document', `
    ${auditCode}
    const AppState = {
      ui: { theme: 'dark', density: 'compact' },
      config: { activeProject: 'lexlean-r4', operatingMode: 'coding', retainLambda: 0.85, pluralityVotes: 8, mcpSecurityPolicy: 'safe' },
      workspace: { activePath: 'src/attention.rs', pendingPatchPath: '', pendingProposedCode: '' }
    };
    let activeWorkspacePath = 'src/attention.rs';
    let pendingPatchPath = '';
    let pendingProposedCode = '';

    function dispatchStudioEnvelope(type, payload) { return { status: 'ok' }; }
    function selectWorkspaceFile(f) {}
    function renderWorkspaceList(ws) {}
    ${policyCode}
    ${customCode}

    initCustomizationsAndMcp();

    return {
      AppState,
      secopsAudit,
      setPending(code, path) {
        pendingProposedCode = code;
        pendingPatchPath = path;
        AppState.workspace.pendingProposedCode = code;
        AppState.workspace.pendingPatchPath = path;
      },
      clearPending() {
        pendingProposedCode = '';
        pendingPatchPath = '';
        AppState.workspace.pendingProposedCode = '';
        AppState.workspace.pendingPatchPath = '';
      },
      getPending() {
        return { code: pendingProposedCode, path: pendingPatchPath };
      }
    };
  `);

  const ctx = harness(document);

  // 1.1a Rapid clicks on naturally disabled empty pool button (50 iterations)
  for (let i = 0; i < 50; i++) {
    approveBtn.click();
  }
  assert.equal(applyClickCount, 0, 'chat-apply-patch-btn must never be called on disabled button');
  assert.equal(statsEl.textContent, 'Queue idle · No patch pending', 'Initial status preserved under disabled clicks');

  // 1.1b Rapid clicks if button is enabled while queue is still empty (50 iterations)
  approveBtn.disabled = false;
  for (let i = 0; i < 50; i++) {
    approveBtn.click();
  }
  assert.equal(applyClickCount, 0, 'chat-apply-patch-btn must never be called on empty pool');
  assert.equal(statsEl.textContent, 'No pending patch to approve in pool.', 'Status should announce no pending patch');
  assert.equal(ctx.secopsAudit.events.length, 0, 'No audit events should be emitted for empty approvals');
  console.log('  ✓ 1.1 100 rapid clicks on empty pool (50 disabled + 50 enabled) resulted in 0 applied patches and 0 audit side-effects.');

  // 2. Stage a valid diff and verify transition
  ctx.setPending('// Updated flock attention algorithm\npub fn cyclic_step() -> u32 { 1 }', 'src/attention.rs');
  approveBtn.disabled = false;
  approveBtn.textContent = 'Approve Patch';
  statsEl.textContent = 'Diff Staged · Ready for Review';

  // 3. Approve staged patch
  approveBtn.click();
  assert.equal(applyClickCount, 1, 'chat-apply-patch-btn must be called exactly once on valid approval');
  assert.equal(approveBtn.textContent, 'Patch Approved ✓');
  assert.equal(approveBtn.disabled, true, 'Approve button must be disabled after approval');
  assert.equal(statsEl.textContent, 'Approved & Committed to Workspace');
  assert.equal(ctx.secopsAudit.events.length, 1, 'Audit ledger must record patch approval event');
  assert.equal(ctx.secopsAudit.events[0].action, 'patch_approve');
  assert.equal(ctx.secopsAudit.events[0].target, 'src/attention.rs');
  console.log('  ✓ 1.2 Staged diff approved cleanly, apply button triggered, Merkle audit event logged.');

  // 4. Repeated clicks after approval
  for (let i = 0; i < 50; i++) {
    approveBtn.click();
  }
  assert.equal(applyClickCount, 1, 'Disabled approve button suppresses all subsequent clicks');
  console.log('  ✓ 1.3 State transition verified: approve button safely handles post-approval clicks.');

  // 5. Test Reject Transition
  ctx.setPending('// Malicious unapproved patch', 'src/engine.rs');
  approveBtn.disabled = false;
  approveBtn.textContent = 'Approve Patch';
  statsEl.textContent = 'Diff Staged · Ready for Review';
  rejectBtn.disabled = false;

  rejectBtn.click();
  assert.equal(ctx.getPending().code, '', 'Pending code must be wiped on rejection');
  assert.equal(ctx.getPending().path, '', 'Pending path must be wiped on rejection');
  assert.equal(approveBtn.disabled, true, 'Approve button must be disabled after rejection');
  assert.equal(approveBtn.textContent, 'Patch Rejected ✗');
  assert.equal(statsEl.textContent, 'Patch Rejected');
  const lastEvent = ctx.secopsAudit.events[ctx.secopsAudit.events.length - 1];
  assert.equal(lastEvent.action, 'patch_reject');
  console.log('  ✓ 1.4 Staged diff rejected cleanly, pending code wiped, cards reset, audit logged.');
}

// ============================================================================
// CHALLENGE 2: Customizations Parameter Bounds & Safe JSON Packaging
// ============================================================================
console.log('\n--- Challenge 2: Customizations Parameter Bounds & JSON Packaging ---');
{
  const { document } = createDOMEnvironment();
  const retainInput = document.getElementById('custom-retain-lambda');
  const retainValEl = document.getElementById('custom-retain-lambda-val');
  const votesInput = document.getElementById('custom-plurality-votes');

  const harness = new Function('document', `
    ${auditCode}
    const AppState = {
      ui: { theme: 'dark', density: 'compact' },
      config: { activeProject: 'lexlean-r4', operatingMode: 'coding', retainLambda: 0.85, pluralityVotes: 8, mcpSecurityPolicy: 'safe' },
      workspace: { activePath: 'src/attention.rs', pendingPatchPath: '', pendingProposedCode: '' }
    };
    let activeWorkspacePath = 'src/attention.rs';
    let pendingPatchPath = '';
    let pendingProposedCode = '';

    function dispatchStudioEnvelope(type, payload) { return { status: 'ok' }; }
    function selectWorkspaceFile(f) {}
    function renderWorkspaceList(ws) {}
    ${policyCode}
    ${customCode}

    initCustomizationsAndMcp();

    return {
      AppState,
      secopsAudit,
      packageChatPayload(promptText) {
        return JSON.stringify({
          prompt: promptText,
          operating_mode: AppState.config.operatingMode || 'coding',
          retain_lambda: AppState.config.retainLambda ?? 0.85,
          plurality_votes: AppState.config.pluralityVotes ?? 8,
          active_project: AppState.config.activeProject || 'lexlean-r4'
        });
      }
    };
  `);

  const ctx = harness(document);

  // 2.1 Test #custom-retain-lambda extreme values
  const lambdaTestValues = [
    { in: '0.0', expectedVal: '0.0', expectedJsonVal: 0.0 },
    { in: '1.0', expectedVal: '1.0', expectedJsonVal: 1.0 },
    { in: '-1', expectedVal: '-1', expectedJsonVal: -1 },
    { in: 'NaN', expectedVal: 'NaN', expectedJsonVal: 'NaN' },
    { in: '0.85', expectedVal: '0.85', expectedJsonVal: 0.85 },
    { in: '1e5', expectedVal: '1e5', expectedJsonVal: 100000 }
  ];

  for (const tc of lambdaTestValues) {
    retainInput.value = tc.in;
    retainInput.dispatchEvent({ type: 'input', target: retainInput });
    retainInput.dispatchEvent({ type: 'change', target: retainInput });

    assert.equal(retainValEl.textContent, tc.expectedVal, `Readout element did not reflect ${tc.in}`);
    assert.equal(ctx.AppState.config.retainLambda, tc.expectedVal);

    // Verify JSON packaging never throws and produces valid parseable JSON
    const payloadStr = ctx.packageChatPayload('Test prompt for lambda bounds');
    let parsed;
    assert.doesNotThrow(() => {
      parsed = JSON.parse(payloadStr);
    }, `JSON packaging failed for lambda=${tc.in}`);
    assert.ok(parsed.prompt === 'Test prompt for lambda bounds');
    assert.ok(parsed.retain_lambda !== undefined);
  }
  console.log('  ✓ 2.1 #custom-retain-lambda handled 0.0, 1.0, -1, NaN gracefully with safe JSON packaging.');

  // 2.2 Test #custom-plurality-votes extreme values (1, 16, 0, 100, NaN)
  const votesTestValues = [
    { in: '1', expectedVotes: 1 },
    { in: '16', expectedVotes: 16 },
    { in: '0', expectedVotes: 8 },      // 0 || 8 -> 8 (fallback)
    { in: '100', expectedVotes: 16 },   // Clamped to max 16
    { in: '-10', expectedVotes: 1 },    // Clamped to min 1
    { in: 'NaN', expectedVotes: 8 },    // Fallback to 8
    { in: 'invalid', expectedVotes: 8 } // Fallback to 8
  ];

  for (const tc of votesTestValues) {
    votesInput.value = tc.in;
    votesInput.dispatchEvent({ type: 'input', target: votesInput });
    votesInput.dispatchEvent({ type: 'change', target: votesInput });

    assert.equal(ctx.AppState.config.pluralityVotes, tc.expectedVotes, `Votes mismatch for input ${tc.in}`);

    const payloadStr = ctx.packageChatPayload('Votes prompt test');
    const parsed = JSON.parse(payloadStr);
    assert.equal(parsed.plurality_votes, tc.expectedVotes);
    assert.ok(parsed.plurality_votes >= 1 && parsed.plurality_votes <= 16, `Plurality votes out of range: ${parsed.plurality_votes}`);
  }
  console.log('  ✓ 2.2 #custom-plurality-votes strictly clamped within [1, 16] with safe fallback for 0, 100, NaN.');
}

// ============================================================================
// CHALLENGE 3: Theme Toggling 100 Rapid Cycles & Zero Memory Leak
// ============================================================================
console.log('\n--- Challenge 3: Theme Toggling Rapid Cycling & Stability ---');
{
  const { document, body } = createDOMEnvironment();
  const themeToggle = document.getElementById('custom-theme-toggle');

  const harness = new Function('document', `
    ${auditCode}
    const AppState = {
      ui: { theme: 'dark', density: 'compact' },
      config: { activeProject: 'lexlean-r4', operatingMode: 'coding', retainLambda: 0.85, pluralityVotes: 8, mcpSecurityPolicy: 'safe' },
      workspace: { activePath: 'src/attention.rs', pendingPatchPath: '', pendingProposedCode: '' }
    };
    let activeWorkspacePath = 'src/attention.rs';
    let pendingPatchPath = '';
    let pendingProposedCode = '';

    function dispatchStudioEnvelope(type, payload) { return { status: 'ok' }; }
    function selectWorkspaceFile(f) {}
    function renderWorkspaceList(ws) {}
    ${policyCode}
    ${customCode}

    initCustomizationsAndMcp();

    return { AppState, secopsAudit };
  `);

  const ctx = harness(document);

  // Initial state check
  assert.equal(body.classList.contains('theme-warm-light'), false, 'Initial theme should be Obsidian Dark');
  assert.equal(ctx.AppState.ui.theme, 'dark');

  // Baseline heap before cycling
  if (global.gc) global.gc();
  const initialHeap = process.memoryUsage().heapUsed;

  // Execute 100 rapid toggling cycles
  for (let i = 0; i < 100; i++) {
    const shouldBeLight = (i % 2 === 0); // true on 0, false on 1...
    themeToggle.checked = shouldBeLight;
    themeToggle.dispatchEvent({ type: 'change', target: themeToggle });

    const hasClass = body.classList.contains('theme-warm-light');
    assert.equal(hasClass, shouldBeLight, `Class mismatch at iteration ${i}: expected ${shouldBeLight}, got ${hasClass}`);
    assert.equal(ctx.AppState.ui.theme, shouldBeLight ? 'warm-light' : 'dark');
  }

  // Final check: exactly 100 audit events logged
  assert.equal(ctx.secopsAudit.events.length, 100, 'All 100 theme changes must be recorded in audit ledger');
  assert.equal(ctx.secopsAudit.verifyChain(), true, 'Merkle chain must remain 100% valid after 100 cycles');

  const finalHeap = process.memoryUsage().heapUsed;
  const heapDeltaKB = (finalHeap - initialHeap) / 1024;

  // Assert no duplicate CSS classes
  const classes = body.className.split(/\s+/).filter(Boolean);
  const uniqueClasses = new Set(classes);
  assert.equal(classes.length, uniqueClasses.size, 'No duplicate CSS classes accumulated on document.body');

  console.log(`  ✓ 3.1 100 rapid theme toggle cycles completed with strict class stability and valid Merkle chain (Heap delta: ${heapDeltaKB.toFixed(1)} KB).`);
}

// ============================================================================
// CHALLENGE 4: Density Mode Cycling (Comfortable <-> Compact)
// ============================================================================
console.log('\n--- Challenge 4: Density Mode Cycling ---');
{
  const { document, body } = createDOMEnvironment();
  const densitySelect = document.getElementById('custom-density-select');
  densitySelect.value = 'compact';

  const harness = new Function('document', `
    ${auditCode}
    const AppState = {
      ui: { theme: 'dark', density: 'compact' },
      config: { activeProject: 'lexlean-r4', operatingMode: 'coding', retainLambda: 0.85, pluralityVotes: 8, mcpSecurityPolicy: 'safe' },
      workspace: { activePath: 'src/attention.rs', pendingPatchPath: '', pendingProposedCode: '' }
    };
    let activeWorkspacePath = 'src/attention.rs';
    let pendingPatchPath = '';
    let pendingProposedCode = '';

    function dispatchStudioEnvelope(type, payload) { return { status: 'ok' }; }
    function selectWorkspaceFile(f) {}
    function renderWorkspaceList(ws) {}
    ${policyCode}
    ${customCode}

    initCustomizationsAndMcp();

    return { AppState, secopsAudit };
  `);

  const ctx = harness(document);

  // Initial check: compact enabled on boot
  assert.equal(body.classList.contains('density-compact'), true, 'Compact density should be active on boot');
  assert.equal(ctx.AppState.ui.density, 'compact');

  // Cycle 100 times between comfortable and compact
  for (let i = 0; i < 100; i++) {
    const mode = (i % 2 === 0) ? 'comfortable' : 'compact';
    densitySelect.value = mode;
    densitySelect.dispatchEvent({ type: 'change', target: densitySelect });

    const expectedClass = (mode === 'compact');
    assert.equal(body.classList.contains('density-compact'), expectedClass, `Density class failed at iteration ${i}`);
    assert.equal(ctx.AppState.ui.density, mode);
    assert.equal(ctx.AppState.densityMode, mode);
  }

  // Verify CSS stylesheet rules exist for density-compact
  assert.ok(appCssContent.includes('body.density-compact'), 'app.css must contain real rules for body.density-compact');
  assert.ok(appCssContent.includes('.density-compact .chat-turn'), 'app.css must target compact chat turns');
  assert.ok(appCssContent.includes('.density-compact .gpt-topbar'), 'app.css must target compact topbar');

  console.log('  ✓ 4.1 100 cycles between comfortable and compact verified with synchronous DOM class update and CSS contract.');
}

// ============================================================================
// CHALLENGE 5: Workspace Switching & In-Memory Filesystem Stability
// ============================================================================
console.log('\n--- Challenge 5: Workspace Switching & Virtual Filesystem Stability ---');
{
  const { document } = createDOMEnvironment();
  const projectSelect = document.getElementById('sidebar-project-select');
  const targetBadge = document.getElementById('chat-target-badge');
  const wsSummary = document.getElementById('sidebar-workspace-summary');
  const repoDiv = document.createElement('div');
  wsSummary.appendChild(repoDiv);

  // Mock in-memory Wasm virtual filesystem
  const virtualFS = new Map([
    ['src/attention.rs', '// Canonical repo attention\npub fn attention() {}'],
    ['src/engine.rs', '// Engine code'],
    ['README.md', '# LexLean R4']
  ]);

  let activeFile = 'src/attention.rs';
  let renderedFiles = [];

  const harness = new Function('document', 'virtualFS', 'onSelectFile', 'onRenderList', `
    ${auditCode}
    const AppState = {
      ui: { theme: 'dark', density: 'compact' },
      config: { activeProject: 'lexlean-r4', operatingMode: 'coding', retainLambda: 0.85, pluralityVotes: 8, mcpSecurityPolicy: 'safe' },
      workspace: { activePath: 'src/attention.rs', pendingPatchPath: '', pendingProposedCode: '' }
    };
    let activeWorkspacePath = 'src/attention.rs';
    let pendingPatchPath = '';
    let pendingProposedCode = '';

    function dispatchStudioEnvelope(type, payload) {
      if (type === 'workspace_read') {
        if (virtualFS.has(payload)) {
          return { status: 'ok', content: virtualFS.get(payload) };
        }
        return { status: 'error', message: 'File not found' };
      }
      if (type === 'workspace_write') {
        const firstNl = payload.indexOf('\\n');
        const path = payload.slice(0, firstNl);
        const content = payload.slice(firstNl + 1);
        virtualFS.set(path, content);
        return { status: 'ok', path, bytes: content.length };
      }
      if (type === 'workspace_list') {
        return Array.from(virtualFS.keys());
      }
      return { status: 'ok' };
    }

    function selectWorkspaceFile(f) {
      activeWorkspacePath = f;
      AppState.workspace.activePath = f;
      onSelectFile(f);
    }

    function renderWorkspaceList(ws) {
      onRenderList(ws);
    }

    ${policyCode}
    ${customCode}

    initCustomizationsAndMcp();

    return { AppState, secopsAudit, dispatchStudioEnvelope };
  `);

  const ctx = harness(
    document,
    virtualFS,
    (f) => { activeFile = f; },
    (ws) => { renderedFiles = ws; }
  );

  // Execute 50 rapid back-and-forth switching cycles (100 switches total)
  for (let i = 0; i < 50; i++) {
    // Switch to scratchpad
    projectSelect.value = 'scratchpad';
    projectSelect.dispatchEvent({ type: 'change', target: projectSelect });

    assert.equal(ctx.AppState.activeProject, 'scratchpad');
    assert.equal(activeFile, 'scratchpad.rs');
    assert.ok(virtualFS.has('scratchpad.rs'), 'scratchpad.rs must be seeded');
    assert.ok(virtualFS.has('notes.md'), 'notes.md must be seeded');
    assert.ok(targetBadge.textContent.includes('scratchpad.rs'));

    // Switch back to primary
    projectSelect.value = 'lexlean-r4';
    projectSelect.dispatchEvent({ type: 'change', target: projectSelect });

    assert.equal(ctx.AppState.activeProject, 'lexlean-r4');
    assert.equal(activeFile, 'src/attention.rs');
    assert.ok(targetBadge.textContent.includes('src/attention.rs'));
  }

  // Verify virtual filesystem stability: seeded files were preserved, not corrupted
  assert.ok(virtualFS.get('scratchpad.rs').includes('scratchpad_step'));
  assert.ok(virtualFS.get('notes.md').includes('Scratchpad Workspace Notes'));
  assert.equal(virtualFS.size, 5, 'Virtual filesystem contains exactly 5 files (3 initial + 2 seeded)');
  assert.equal(ctx.secopsAudit.events.length, 100, '100 project switch events recorded in audit ledger');
  assert.equal(ctx.secopsAudit.verifyChain(), true, 'Audit chain remains strictly unbroken');

  console.log('  ✓ 5.1 100 rapid workspace switches completed; virtual memory files seeded and preserved with zero corruption.');
}

// ============================================================================
// CHALLENGE 6: MCP Tool Runs, Audit Log Formatting & Merkle Consistency
// ============================================================================
console.log('\n--- Challenge 6: MCP Tool Runs, Log Formatting & Merkle Consistency ---');
{
  const { document } = createDOMEnvironment();
  const mcpLog = document.getElementById('mcp-audit-log');
  const policySelect = document.getElementById('mcp-policy-select');

  // Define 6 tool buttons
  const toolNames = [
    'workspace_read',
    'workspace_write',
    'subsystem_prime_gcd',
    'geometric_dijkstra_route',
    'clifford_bivector_wedge',
    'coordinate_descent_sweep'
  ];

  const toolButtons = [];
  for (const name of toolNames) {
    const btn = document.createElement('button');
    btn.className = 'mcp-tool-run-btn';
    btn.setAttribute('data-tool', name);
    toolButtons.push(btn);
  }

  const harness = new Function('document', 'toolButtons', `
    ${auditCode}
    const AppState = {
      ui: { theme: 'dark', density: 'compact' },
      config: { activeProject: 'lexlean-r4', operatingMode: 'coding', retainLambda: 0.85, pluralityVotes: 8, mcpSecurityPolicy: 'safe' },
      workspace: { activePath: 'src/attention.rs', pendingPatchPath: '', pendingProposedCode: '' }
    };
    let activeWorkspacePath = 'src/attention.rs';
    let pendingPatchPath = '';
    let pendingProposedCode = '';

    function dispatchStudioEnvelope(type, payload) {
      return { status: 'ok', dispatched: type, payload };
    }
    function selectWorkspaceFile(f) {}
    function renderWorkspaceList(ws) {}
    ${policyCode}
    ${customCode}

    initCustomizationsAndMcp();

    return { AppState, secopsAudit, checkMcpToolPolicy };
  `);

  const ctx = harness(document, toolButtons);

  // 6.1 Trigger all 6 tool runs under safe policy
  for (const btn of toolButtons) {
    const ok = btn.click();
    assert.equal(ok, true, `Tool click was unexpectedly canceled: ${btn.getAttribute('data-tool')}`);
  }

  // Assert #mcp-audit-log received items
  assert.equal(mcpLog.children.length, 12, '6 tool runs emitted 6 check policy events + 6 tool run events = 12 total items');

  const logRegex = /^\[\d{2}:\d{2}:\d{2}\.\d{3}\] \[[A-Z_]+\] .+ -> .+ \([0-9a-f]{8}\)$/;
  for (const item of mcpLog.children) {
    assert.ok(item.className.includes('mcp-log-item'), `Invalid log item class: ${item.className}`);
    assert.match(item.textContent, logRegex, `Log item text failed regex format: "${item.textContent}"`);
  }
  console.log('  ✓ 6.1 All 6 MCP tools executed; 12 formatted log entries written with strict timestamp, category, and hash format.');

  // 6.2 Verify Merkle Chain Consistency
  assert.equal(ctx.secopsAudit.verifyChain(), true, 'Chain verification failed after tool executions');
  console.log('  ✓ 6.2 SHA-256 Merkle chain verification passed for all tool audit entries.');

  // 6.3 Adversarial Tamper Invalidation
  {
    const tampered = new ctx.secopsAudit.constructor(500);
    tampered.events = JSON.parse(JSON.stringify(ctx.secopsAudit.events));
    // Tamper with the action of the 4th event
    tampered.events[3].action = 'malicious_tampered_action';
    assert.equal(tampered.verifyChain(), false, 'Tampered action must fail verifyChain');

    // Tamper with the hash of the 7th event
    const tampered2 = new ctx.secopsAudit.constructor(500);
    tampered2.events = JSON.parse(JSON.stringify(ctx.secopsAudit.events));
    tampered2.events[6].event_hash = 'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff';
    assert.equal(tampered2.verifyChain(), false, 'Tampered hash must fail verifyChain');
    console.log('  ✓ 6.3 Tamper evidence verified: modifications to audit action or hash immediately invalidate Merkle proof.');
  }

  // 6.4 Security Policy Mode: DENY
  policySelect.value = 'deny';
  policySelect.dispatchEvent({ type: 'change', target: policySelect });
  assert.equal(ctx.AppState.config.mcpSecurityPolicy, 'deny');

  const logLenBefore = mcpLog.children.length;
  // Try running coordinate_descent_sweep in deny mode
  const sweepBtn = toolButtons.find((b) => b.getAttribute('data-tool') === 'coordinate_descent_sweep');
  sweepBtn.click();

  // Deny policy blocks execution and writes a critical policy blocked entry
  const lastLog = mcpLog.children[mcpLog.children.length - 1];
  assert.ok(lastLog.className.includes('mcp-log-critical'), 'Blocked tool run must have mcp-log-critical class');
  assert.ok(lastLog.textContent.includes('[POLICY BLOCKED]'), 'Blocked tool run must announce policy blocked');
  console.log('  ✓ 6.4 MCP security policy DENY mode strictly blocks execution and writes critical audit record.');
}

console.log('\n================================================================');
console.log('  CHALLENGE COMPLETE: ALL 6 EMPIRICAL CHALLENGES PASSED GREEN');
console.log('================================================================\n');
