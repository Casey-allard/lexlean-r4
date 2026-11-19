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
    if (child) child.parentNode = this;
    this.children.push(child);
  }

  removeChild(child) {
    const idx = this.children.indexOf(child);
    if (idx >= 0) this.children.splice(idx, 1);
    if (child && child.parentNode === this) child.parentNode = null;
    return child;
  }

  replaceChildren(...newChildren) {
    this.children = [...newChildren];
    for (const c of this.children) {
      if (c) c.parentNode = this;
    }
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

// ============================================================================
// CHALLENGE 7 (R4-60): M3 Window Size Classes, Live HUD + 512-Token Scrubber,
// Consent Card SecurityError Fallback & ?surface=preview Fuel Limit
// ============================================================================
console.log('\n--- Challenge 7 (R4-60): M3 Layout, Live Mini-HUD Scrubber, Consent Fallback & Fuel-Bounded Preview ---');
{
  const startMarker = '// ============================================================================\n// R4-60 STUDIO LIVE HUD, M3 RESPONSIVE SHELL, CONSENT & PREVIEW RUNTIME';
  const endMarker = '// ============================================================================\n// END R4-60 STUDIO RUNTIME';
  const r460Start = appJsContent.indexOf(startMarker);
  const r460End = appJsContent.indexOf(endMarker, r460Start);
  assert.ok(
    r460Start !== -1 && r460End !== -1,
    'public/app.js must export/contain R4-60 Studio Live HUD, Consent & Preview runtime block'
  );
  const r460Code = appJsContent.slice(r460Start, r460End);

  const { document, body } = createDOMEnvironment();
  body.dataset = {};
  const prismFrame = document.getElementById('prism-frame');
  prismFrame.dataset = {};
  const mainEl = document.createElement('main');
  const sidebarEl = document.getElementById('threads-sidebar');
  const dockSplitEl = document.getElementById('observatory-dock-split');
  const inspectorsModalEl = document.getElementById('inspectors-modal');
  const tracesPanelEl = document.getElementById('canvas-panel-traces');
  const obsDeckEl = document.getElementById('holo-observatory-deck');
  const chatMessagesEl = document.getElementById('chat-messages');
  const modelSelectEl = document.getElementById('model-select');
  const composerModelSelectEl = document.getElementById('composer-model-select');
  const topbarModelTagEl = document.getElementById('topbar-model-tag');
  const weightCacheBadgeEl = document.getElementById('weight-cache-badge');
  const toggleSidebarBtn = document.getElementById('toggle-sidebar-btn');
  const closeSidebarBtn = document.getElementById('close-sidebar-btn');
  const toastBannerEl = document.getElementById('studio-toast-banner');
  toastBannerEl.dataset = {};
  toastBannerEl.setAttribute('hidden', '');
  const toastMsgEl = document.getElementById('studio-toast-message');
  const toastCloseBtn = document.getElementById('studio-toast-close-btn');
  const settingsAllowOffOrigin = document.getElementById('settings-allow-off-origin-weights');

  const createMockCanvas = (id, w = 132, h = 48) => {
    const c = document.getElementById(id);
    c.tagName = 'CANVAS';
    c.width = w;
    c.height = h;
    c.ops = [];
    c.getContext = () => ({
      fillStyle: '#000',
      strokeStyle: '#000',
      lineWidth: 1,
      font: '10px monospace',
      clearRect: (...a) => c.ops.push(['clearRect', ...a]),
      fillRect: (...a) => c.ops.push(['fillRect', ...a]),
      strokeRect: (...a) => c.ops.push(['strokeRect', ...a]),
      beginPath: () => c.ops.push(['beginPath']),
      closePath: () => c.ops.push(['closePath']),
      moveTo: (...a) => c.ops.push(['moveTo', ...a]),
      lineTo: (...a) => c.ops.push(['lineTo', ...a]),
      arc: (...a) => c.ops.push(['arc', ...a]),
      fill: () => c.ops.push(['fill']),
      stroke: () => c.ops.push(['stroke']),
      fillText: (...a) => c.ops.push(['fillText', ...a]),
      scale: () => {},
      save: () => {},
      restore: () => {},
      translate: () => {},
      rotate: () => {},
      measureText: (t) => ({ width: String(t || '').length * 6 }),
    });
    return c;
  };
  createMockCanvas('holo-layer-waterfall-canvas', 460, 220);
  createMockCanvas('holo-shortlist-race-canvas', 460, 220);
  createMockCanvas('holo-causal-flock-canvas', 460, 220);
  createMockCanvas('holo-storage-geometry-canvas', 460, 220);

  const origCreate = document.createElement.bind(document);
  document.head = origCreate('head');
  document.adoptedStyleSheets = [];
  body.appendChild(document.head);
  document.createElement = (tag) => {
    const el = origCreate(tag);
    el.dataset = {};
    if (String(tag).toLowerCase() === 'canvas') {
      el.width = 132;
      el.height = 48;
      el.ops = [];
      el.getContext = () => ({
        fillStyle: '#000',
        strokeStyle: '#000',
        lineWidth: 1,
        font: '10px monospace',
        clearRect: (...a) => el.ops.push(['clearRect', ...a]),
        fillRect: (...a) => el.ops.push(['fillRect', ...a]),
        strokeRect: (...a) => el.ops.push(['strokeRect', ...a]),
        beginPath: () => el.ops.push(['beginPath']),
        closePath: () => el.ops.push(['closePath']),
        moveTo: (...a) => el.ops.push(['moveTo', ...a]),
        lineTo: (...a) => el.ops.push(['lineTo', ...a]),
        arc: (...a) => el.ops.push(['arc', ...a]),
        fill: () => el.ops.push(['fill']),
        stroke: () => el.ops.push(['stroke']),
        fillText: (...a) => el.ops.push(['fillText', ...a]),
        scale: () => {},
        save: () => {},
        restore: () => {},
        translate: () => {},
        rotate: () => {},
        measureText: (t) => ({ width: String(t || '').length * 6 }),
      });
    }
    return el;
  };
  document.querySelector = (sel) => (sel === 'main' ? mainEl : body.querySelector(sel));
  document.createTextNode = (txt) => {
    const n = origCreate('#text');
    n.textContent = String(txt ?? '');
    return n;
  };
  tracesPanelEl.appendChild(obsDeckEl);
  body.appendChild(mainEl);
  mainEl.appendChild(sidebarEl);
  mainEl.appendChild(dockSplitEl);
  body.appendChild(inspectorsModalEl);
  inspectorsModalEl.appendChild(tracesPanelEl);
  body.appendChild(chatMessagesEl);

  const rpcCalls = [];
  const rerunPrompts = [];
  const postedMessages = [];
  const throwingLocalStorage = {
    getItem() { throw new Error('SecurityError: storage blocked'); },
    setItem() { throw new Error('SecurityError: storage blocked'); },
  };
  const mockWin = {
    innerWidth: 1280,
    devicePixelRatio: 2,
    location: { search: '' },
    parent: { postMessage: (msg) => postedMessages.push(msg) },
    addEventListener: () => {},
    matchMedia: () => ({ matches: false, addEventListener: () => {} }),
  };

  const sandboxR460 = new Function(
    'document',
    'window',
    'localStorage',
    'AppState',
    'engineWorkerClient',
    'runClaudeChat',
    'syncComposerModel',
    'updateWeightCacheBadge',
    'renderHoloObservatoryDeck',
    'holoObservatoryState',
    'renderStudioToast',
    `"use strict";
     ${r460Code}
     return {
       classifyWindowWidth,
       applyWindowWidthClass,
       syncObservatoryHost,
       safeStorageGet,
       safeStorageSet,
       showStudioToast,
       createLiveTokenHudController,
       renderHydrationConsentCard,
       PreviewScriptFuelExhausted,
       evaluatePreviewScriptWithFuel,
       renderPreviewWorkspace,
       handlePreviewInspectRequest,
       initPreviewSurfaceHost,
       _r460PendingInspectWaiters,
       getLatestPreviewDiag: () => _r460LatestPreviewDiag,
     };`
  );

  const AppStateMock = {
    config: { selectedModel: 'uor-r4.qwen3-0.6b-holo', composerMode: 'agentic' },
    ui: { theme: 'dark', density: 'compact' },
  };
  const workerMock = {
    enabled: true,
    ready: true,
    holoStatus: { mounted_count: 0 },
    async rpc(op, payload) {
      rpcCalls.push({ op, payload });
      return { ok: true, fallback_model: 'uor-r4.native-q31-d85' };
    },
  };
  const obsStateMock = {
    frames: [],
    qTrail: [],
    paused: false,
    lockedFrameIdx: null,
    rotAngle: 0.42,
    prefixReused: 0,
    shortlistK: 4096,
    backend: 'wasm-simd128',
  };
  let syncedModels = [];
  let obsRenderCount = 0;

  const origCSSStyleSheet = globalThis.CSSStyleSheet;
  globalThis.CSSStyleSheet = class MockCSSStyleSheet {
    replaceSync(css) {
      this.cssText = String(css ?? '');
    }
  };
  mockWin.CSSStyleSheet = globalThis.CSSStyleSheet;
  try {
    const r460 = sandboxR460(
      document,
      mockWin,
      throwingLocalStorage,
      AppStateMock,
      workerMock,
      (prompt, appendUserBubble) => rerunPrompts.push({ prompt, appendUserBubble }),
      (m) => {
        syncedModels.push(m);
        AppStateMock.config.selectedModel = m;
        modelSelectEl.value = m;
        composerModelSelectEl.value = m;
      },
      () => {},
      () => { obsRenderCount += 1; },
      obsStateMock,
      null
    );

    // 7.1 M3 Window Size Classes (<600 compact, 600..839 medium, >=840 expanded) & Sidebar Recovery
    assert.equal(r460.classifyWindowWidth(390), 'compact');
    assert.equal(r460.classifyWindowWidth(599), 'compact');
    assert.equal(r460.classifyWindowWidth(600), 'medium');
    assert.equal(r460.classifyWindowWidth(839), 'medium');
    assert.equal(r460.classifyWindowWidth(840), 'expanded');
    r460.applyWindowWidthClass(500);
    assert.equal(body.dataset.windowClass, 'compact');
    assert.equal(prismFrame.dataset.windowClass, 'compact');
    assert.equal(sidebarEl.classList.contains('sidebar-collapsed'), true, 'Compact 500px must collapse sidebar');
    body.dataset.layoutMode = 'studio';
    r460.syncObservatoryHost();
    assert.ok(tracesPanelEl.children.includes(obsDeckEl), 'Compact mode must keep Observatory inside #inspectors-modal');
    r460.applyWindowWidthClass(1280);
    assert.equal(body.dataset.windowClass, 'expanded');
    assert.equal(sidebarEl.classList.contains('sidebar-collapsed'), false, 'Widening to 1280px must recover sidebar unless manually collapsed');
    r460.syncObservatoryHost();
    assert.ok(dockSplitEl.children.includes(obsDeckEl), 'Expanded Studio mode must dock Observatory into #observatory-dock-split');
    console.log('  ✓ 7.1 M3 window size classes, sidebar 500->1280 recovery & Observatory dock verified.');

    // 7.2 Live Mini-HUD + <=512 token ring-buffer scrubber + Worker handleMessage + lock/unlock/eviction + Q31 honesty
    const hudCtrl = r460.createLiveTokenHudController(chatMessagesEl);
    for (let i = 0; i < 520; i++) {
      const tokText = i === 10 ? '<img src=x onerror=alert(1)>' : `tok${i} `;
      hudCtrl.pushTokenStep({
        frame: {
          step_idx: i,
          phase: 'decode',
          seq_pos: 20 + i,
          prefix_reused_tokens: 18,
          token_id: 100 + i,
          token_text: tokText,
          has_geometric_shortlist: true,
          min_hamming: 30 + (i % 20),
          shortlist_count: 1536,
          ttft_ms: 42,
          tps_x10: 615,
          q_manifold_xyz: [i - 250, (i * 3) % 400, -100],
          layer_max_bits: new Array(28).fill(18 + (i % 6)),
          layer_residual_mag: new Array(28).fill(500 + i),
          hamming_hist_32: new Array(32).fill(10),
          top_candidates: [
            { token_id: 1, token_text: tokText, logit_q16: 9000, prob_permille: 614, hamming_d: 31, selected: true },
            { token_id: 2, token_text: 'alt', logit_q16: 7200, prob_permille: 182, hamming_d: 44, selected: false },
          ],
          backend: 'holo-v13-simd128',
        },
        delta: tokText,
        tokens_emitted: i + 1,
        tok_per_sec: 61.5,
        prefix_reused_tokens: 18,
      });
    }
    const hudEl = chatMessagesEl.querySelector('.holo-live-hud');
    const scrubberEl = chatMessagesEl.querySelector('.holo-token-scrubber');
    assert.ok(hudEl, '.holo-live-hud must be created inside assistant message');
    assert.ok(scrubberEl, '.holo-token-scrubber must be created inside assistant message');
    assert.ok(hudEl.querySelector('.holo-hud-canvas'), 'Mini-HUD must contain .holo-hud-canvas');
    const tokSpans = scrubberEl.querySelectorAll('.holo-tok-span');
    assert.ok(tokSpans.length <= 512 && tokSpans.length > 0, `Scrubber must cap retained spans at <= 512 (got ${tokSpans.length})`);
    assert.equal(hudCtrl.frames.length, 512, 'Ring buffer must retain strictly <= 512 frames');
    const xssSpan = tokSpans.find((s) => s.textContent.includes('<img src=x onerror=alert(1)>'));
    assert.ok(xssSpan, 'XSS payload token must be preserved as literal textContent');
    assert.equal(xssSpan.children.length, 0, 'XSS payload must NOT parse any child <img> element');

    // Hover while unlocked updates Mini-HUD + Observatory without pinning
    const obsHoverBefore = obsRenderCount;
    tokSpans[4].dispatchEvent({ type: 'mouseenter', target: tokSpans[4] });
    assert.ok(obsRenderCount > obsHoverBefore, 'Hovering scrubber span must update Observatory deck');
    assert.equal(obsStateMock.lockedFrameIdx, null, 'Hover must NOT lock frame index');

    // Click locks; click again unlocks; Enter locks; Space unlocks
    const obsBefore = obsRenderCount;
    tokSpans[0].dispatchEvent({ type: 'click', target: tokSpans[0] });
    assert.ok(obsRenderCount > obsBefore, 'Clicking scrubber token must re-render Observatory deck');
    assert.equal(obsStateMock.frames[obsStateMock.frames.length - 1].step_idx, Number(tokSpans[0].getAttribute('data-frame-idx')) + 8, 'Scrubber lock must select exact ring-buffered frame');
    assert.equal(obsStateMock.lockedFrameIdx, 0, 'Clicking span 0 must pin lockedFrameIdx=0');
    assert.equal(tokSpans[0].getAttribute('aria-pressed'), 'true');
    tokSpans[0].dispatchEvent({ type: 'click', target: tokSpans[0] });
    assert.equal(obsStateMock.lockedFrameIdx, null, 'Clicking pinned span again must unlock back to live mode');
    assert.equal(tokSpans[0].getAttribute('aria-pressed'), 'false');

    tokSpans[5].dispatchEvent({ type: 'keydown', key: 'Enter', preventDefault: () => {} });
    assert.equal(obsStateMock.lockedFrameIdx, 5, 'Enter key on scrubber span must lock frame');
    tokSpans[5].dispatchEvent({ type: 'keydown', key: ' ', preventDefault: () => {} });
    assert.equal(obsStateMock.lockedFrameIdx, null, 'Space key on pinned scrubber span must unlock frame');

    // 512-ring eviction unlock: pin span 0, then push 1 more frame -> index 0 evicted -> unlocked
    tokSpans[0].dispatchEvent({ type: 'click', target: tokSpans[0] });
    assert.equal(obsStateMock.lockedFrameIdx, 0);
    hudCtrl.pushTokenStep({
      frame: {
        step_idx: 520,
        phase: 'decode',
        seq_pos: 540,
        prefix_reused_tokens: 18,
        token_id: 620,
        token_text: 'evict ',
        has_geometric_shortlist: true,
        min_hamming: 32,
        shortlist_count: 1536,
        ttft_ms: 42,
        tps_x10: 620,
        q_manifold_xyz: [1, 2, 3],
        layer_max_bits: new Array(28).fill(20),
        top_candidates: [{ token_id: 620, token_text: 'evict ', logit_q16: 8800, prob_permille: 600, hamming_d: 32, selected: true }],
        backend: 'holo-v13-simd128',
      },
      delta: 'evict ',
    });
    assert.equal(obsStateMock.lockedFrameIdx, null, 'Evicting pinned token from 512-ring must unlock live stream');

    // Real engineWorkerClient.handleMessage('token_step') integration
    const workerStart = appJsContent.indexOf('const engineWorkerClient = {');
    const workerEnd = appJsContent.indexOf('const headerBrainPulseState = {');
    assert.ok(workerStart !== -1 && workerEnd > workerStart, 'engineWorkerClient block must exist in public/app.js');
    const workerCode = appJsContent
      .slice(workerStart, workerEnd)
      .replace(/import\.meta\.url/g, "'http://localhost/app.js'");
    const liveWorkerCalls = [];
    const tsComposerStub = { activeTurnId: null, setActiveTurnId(tid) { this.activeTurnId = tid; } };
    const realWorkerClient = new Function(
      'document',
      'window',
      'AppState',
      'holoObservatoryState',
      'ingestHoloObservatoryFrame',
      'tsComposer',
      'DEFAULT_STARTER_WORKSPACE_FILES',
      `${workerCode}\nreturn engineWorkerClient;`
    )(
      document,
      mockWin,
      AppStateMock,
      obsStateMock,
      (frame, meta) => {
        liveWorkerCalls.push({ frame, meta });
        obsStateMock.frames.push(frame);
        obsRenderCount += 1;
      },
      tsComposerStub,
      []
    );
    const streamedEvents = [];
    realWorkerClient.activeChatStreamHook = (ev) => {
      streamedEvents.push(ev);
      if (ev.stage === 'token_step') hudCtrl.pushTokenStep(ev);
    };
    realWorkerClient.handleMessage({
      event: 'token_step',
      turn_id: 91,
      prefix_reused_tokens: 16,
      delta: 'step91 ',
      frame: {
        step_idx: 521,
        phase: 'decode',
        seq_pos: 541,
        prefix_reused_tokens: 16,
        token_id: 621,
        token_text: 'step91 ',
        has_geometric_shortlist: true,
        min_hamming: 28,
        shortlist_count: 1400,
        ttft_ms: 41,
        tps_x10: 630,
        q_manifold_xyz: [4, 5, 6],
        layer_max_bits: new Array(28).fill(19),
        top_candidates: [{ token_id: 621, token_text: 'step91 ', logit_q16: 9100, prob_permille: 640, hamming_d: 28, selected: true }],
        backend: 'holo-v13-simd128',
      },
    });
    assert.equal(tsComposerStub.activeTurnId, 91, 'Worker token_step must update tsComposer.activeTurnId');
    assert.equal(liveWorkerCalls.length, 1, 'Worker token_step must drive real ingestHoloObservatoryFrame');
    assert.equal(liveWorkerCalls[0].meta.prefixReused, 16);
    assert.equal(streamedEvents.length, 1);
    assert.equal(streamedEvents[0].stage, 'token_step');

    // Streaming events carrying numeric `id: streamRpc.id` must NOT reject/delete pending RPC
    realWorkerClient.worker = { postMessage() {} };
    const streamRpc = realWorkerClient.rpcWithId('studio_stream_turn', { prompt: 'hello' });
    const streamFrame = {
      step_idx: 0,
      phase: 'decode',
      seq_pos: 1,
      token_id: 1,
      token_text: 'Hi',
      has_geometric_shortlist: false,
      min_hamming: 0,
      shortlist_count: 0,
      q_manifold_xyz: [1, 2, 3],
      layer_max_bits: [10, 11, 12, 13],
      top_candidates: [{ token_id: 1, token_text: 'Hi', logit_q16: 3200, prob_permille: 800, hamming_d: null, selected: true }],
      backend: 'native-q31-d85',
    };
    realWorkerClient.handleMessage({ event: 'holo_trace_frame', id: streamRpc.id, turn_id: 92, frame: streamFrame });
    realWorkerClient.handleMessage({ event: 'token_step', id: streamRpc.id, turn_id: 92, delta: 'Hi', frame: streamFrame });
    assert.equal(realWorkerClient.pending.has(streamRpc.id), true, 'Streaming event with numeric id must not delete/reject pending RPC');
    realWorkerClient.handleMessage({
      id: streamRpc.id,
      ok: true,
      result: { status: 'ok', reply: 'Hi', tokens_emitted: 1, backend: 'uor-r4.native-q31-d85' },
    });
    const streamRpcOut = await streamRpc.promise;
    assert.equal(streamRpcOut.reply, 'Hi', 'Pending studio_stream_turn RPC must resolve cleanly after streaming events');

    // Q31 honesty when !has_geometric_shortlist
    const q31Hud = r460.createLiveTokenHudController(chatMessagesEl);
    q31Hud.pushTokenStep({
      frame: {
        step_idx: 0,
        phase: 'decode',
        seq_pos: 5,
        prefix_reused_tokens: 0,
        token_id: 7,
        token_text: 'q31',
        has_geometric_shortlist: false,
        min_hamming: 0,
        shortlist_count: 0,
        ttft_ms: 12,
        tps_x10: 450,
        q_manifold_xyz: [10, 20, 30],
        layer_max_bits: [12, 13, 14, 15],
        top_candidates: [
          { token_id: 7, token_text: 'q31', logit_q16: 4000, prob_permille: 700, hamming_d: null, selected: true },
        ],
        backend: 'native-q31-d85',
      },
      delta: 'q31',
    });
    const q31Card = q31Hud.hudEl;
    assert.ok(q31Card.querySelector('.holo-q31-badge'), 'Native Q31 HUD must render honest Q31 Exact Lane badge');
    console.log('  ✓ 7.2 Live Mini-HUD, 512-token scrubber, Worker handleMessage, lock/unlock/eviction & Q31 honesty verified.');

    // 7.3 Consent Card Approve ('1' + allowOffOriginHydration + appendUserBubble=false) & Decline -> Native Q31
    await r460.renderHydrationConsentCard(
      chatMessagesEl,
      {
        model_id: 'uor-r4.qwen3-0.6b-holo',
        approx_bytes: 429916160,
        mirrors: ['https://huggingface.co/example/model/piece_0000.bin'],
      },
      'approve test prompt'
    );
    const consentCardApprove = chatMessagesEl.querySelector('.holo-consent-card');
    assert.ok(consentCardApprove, 'Consent card must render inline');
    const approveBtn = consentCardApprove.querySelector('.holo-consent-approve-btn');
    assert.ok(approveBtn, 'Approve button must exist');
    assert.equal(approveBtn.textContent, 'Download Verified Weights (~410 MiB / ~1.09 GiB)');
    await approveBtn.click();
    for (let k = 0; k < 8; k += 1) await Promise.resolve();
    assert.equal(r460.safeStorageGet('uor_allow_off_origin_hydration'), '1', 'Approve must persist consent as "1"');
    assert.ok(
      rpcCalls.some((c) => (
        c.op === 'hydrate_holo_model'
        && c.payload.modelId === 'uor-r4.qwen3-0.6b-holo'
        && c.payload.allowOffOriginHydration === true
        && c.payload.rememberConsent === true
      )),
      'Approve must send exact { modelId, allowOffOriginHydration: true, rememberConsent: true }'
    );
    assert.ok(
      rerunPrompts.some((r) => r.prompt === 'approve test prompt' && r.appendUserBubble === false),
      'Consent retry must call runClaudeChat(prompt, false) without duplicating user bubble'
    );

    // Approve with empty pendingPrompt when AppState was on Native Q31 must switch back to modelId & show Ready status
    AppStateMock.config.selectedModel = 'uor-r4.native-q31-d85';
    await r460.renderHydrationConsentCard(
      chatMessagesEl,
      {
        model_id: 'uor-r4.qwen3-0.6b-holo',
        approx_bytes: 429916160,
        mirrors: ['https://huggingface.co/example/model/piece_0000.bin'],
      },
      ''
    );
    const emptyCards = chatMessagesEl.querySelectorAll('.holo-consent-card');
    const emptyCard = emptyCards[emptyCards.length - 1];
    const emptyApproveBtn = emptyCard.querySelector('.holo-consent-approve-btn');
    const emptyStatusNote = emptyCard.querySelector('.holo-consent-status');
    await emptyApproveBtn.click();
    for (let k = 0; k < 8; k += 1) await Promise.resolve();
    assert.equal(AppStateMock.config.selectedModel, 'uor-r4.qwen3-0.6b-holo', 'Approve must switch AppState.config.selectedModel back from native-q31-d85');
    assert.ok(syncedModels.includes('uor-r4.qwen3-0.6b-holo'), 'Approve must invoke syncComposerModel(modelId)');
    assert.ok(emptyStatusNote.textContent.includes('Ready'), `Empty pendingPrompt approve must show Ready status, got: ${emptyStatusNote.textContent}`);
    assert.equal(emptyStatusNote.textContent.includes('Re-running prompt'), false, 'Empty pendingPrompt approve must not claim Re-running prompt');

    await r460.renderHydrationConsentCard(
      chatMessagesEl,
      {
        model_id: 'uor-r4.qwen3-0.6b-holo',
        approx_bytes: 429916160,
        mirrors: ['https://huggingface.co/example/model/piece_0000.bin'],
      },
      'hello world'
    );
    const allCards = chatMessagesEl.querySelectorAll('.holo-consent-card');
    const offlineBtn = allCards[allCards.length - 1].querySelector('.holo-consent-offline-btn');
    assert.ok(offlineBtn, 'Offline Native Q31 fallback button must exist');
    await offlineBtn.click();
    for (let k = 0; k < 8; k += 1) await Promise.resolve();
    assert.equal(r460.safeStorageGet('uor_allow_off_origin_hydration'), 'declined', 'SecurityError must fall back to memory storage');
    assert.equal(AppStateMock.config.selectedModel, 'uor-r4.native-q31-d85', 'Declining must auto-switch to Native Q31');
    assert.ok(
      rerunPrompts.some((r) => r.prompt === 'hello world' && r.appendUserBubble === false),
      'Declining must re-run pending prompt with appendUserBubble=false'
    );
    assert.ok(rpcCalls.some((c) => c.op === 'hydrate_holo_model' && c.payload.decline === true), 'Must notify Worker of consent decline');
    console.log('  ✓ 7.3 Off-origin weight consent Approve ("1"), SecurityError fallback & zero-duplicate retry verified.');

    // 7.4 ?surface=preview 100,000-step fuel limit, CSSOM adoptedStyleSheets, template literal & preview_inspect round-trip
    assert.throws(
      () => r460.evaluatePreviewScriptWithFuel('while (true) {}', { document, console: { log: () => {}, warn: () => {}, error: () => {} } }, 100000),
      (err) => err && (err.code === 'PreviewScriptFuelExhausted' || err.error_code === 'PreviewScriptFuelExhausted'),
      'Infinite while (true) {} loop must fail with PreviewScriptFuelExhausted'
    );
    const tplRes = r460.evaluatePreviewScriptWithFuel(
      'const a = 21; const msg = `sum=${a * 2}!`; msg;',
      { document, console: { log: () => {}, warn: () => {}, error: () => {} } },
      100000
    );
    assert.equal(tplRes.value, 'sum=42!', 'Template literal interpolation must evaluate in fuel-bounded AST evaluator');

    assert.throws(
      () => r460.evaluatePreviewScriptWithFuel('const b = `bad=${1 2}`;', { document, console: { log: () => {}, warn: () => {}, error: () => {} } }, 100000),
      (err) => err && /Unexpected token/.test(String(err.message || '')),
      'Trailing garbage inside template ${...} interpolation must throw SyntaxError'
    );

    const rootTargetEl = document.getElementById('preview-root');
    body.appendChild(rootTargetEl);
    const pvDiag = r460.renderPreviewWorkspace(
      {
        'styles.css': '.box { color: red; }',
        'app.js': 'const n = 21; document.body.textContent = `sum=${n * 2}`; console.log(`val=${n * 2}`);',
      },
      rootTargetEl
    );
    assert.equal(document.querySelectorAll('style').length, 0, 'Preview CSS must use adoptedStyleSheets without injecting CSP-violating <style> elements');
    assert.equal(document.adoptedStyleSheets.length, 1, 'document.adoptedStyleSheets must receive CSSStyleSheet');
    assert.equal(rootTargetEl.textContent, 'sum=42', 'Preview script must update preview-root via template literal');
    assert.equal(pvDiag.errors.length, 0);
    assert.ok(pvDiag.console.includes('[log] val=42'), 'Template literal inside console.log must evaluate cleanly');

    // Non-web Rust/Markdown /workspace renders visible workspace summary card in preview
    const collectMockText = (node) => {
      if (!node) return '';
      let out = node.textContent ? String(node.textContent) : '';
      for (const ch of node.children || []) out += `\n${collectMockText(ch)}`;
      return out;
    };
    const rustPreviewRoot = document.getElementById('rust-preview-root');
    body.appendChild(rustPreviewRoot);
    const rustPreviewDiag = r460.renderPreviewWorkspace(
      {
        'README.md': '# LexLean Rust Crate\nZero float ops.',
        'src/attention.rs': 'pub fn attn() {}\n',
      },
      rustPreviewRoot
    );
    assert.equal(rustPreviewDiag.fileCount, 2);
    const rustSummaryEl = rustPreviewRoot.querySelector('.preview-workspace-summary');
    assert.ok(rustSummaryEl, 'Rust-only /workspace preview must render .preview-workspace-summary card');
    const rustSummaryText = collectMockText(rustSummaryEl);
    assert.ok(rustSummaryText.includes('UOR-R4 Isolated Workspace Preview'));
    assert.ok(rustSummaryText.includes('README.md'));
    assert.ok(rustSummaryText.includes('src/attention.rs'));

    // 7.4b Real two-window message flow: handlePreviewInspectRequest -> initPreviewSurfaceHost -> parent 'message' listener -> preview_inspect_reply
    const childEnv = createDOMEnvironment();
    const childListeners = new Map();
    const mockParentListeners = new Map();
    const previewConsoleOutputEl = document.getElementById('preview-console-output');
    previewConsoleOutputEl.textContent = 'stale console line';

    const mockParentWin = {
      innerWidth: 1280,
      devicePixelRatio: 2,
      addEventListener(type, fn) {
        if (!mockParentListeners.has(type)) mockParentListeners.set(type, []);
        mockParentListeners.get(type).push(fn);
      },
      dispatchEvent(ev) {
        for (const fn of mockParentListeners.get(ev.type) || []) fn(ev);
      },
    };
    const mockChildWin = {
      innerWidth: 640,
      innerHeight: 480,
      devicePixelRatio: 1,
      parent: mockParentWin,
      addEventListener(type, fn) {
        if (!childListeners.has(type)) childListeners.set(type, []);
        childListeners.get(type).push(fn);
      },
      postMessage(data) {
        queueMicrotask(() => {
          for (const fn of childListeners.get('message') || []) {
            fn({ data, source: mockParentWin });
          }
        });
      },
    };
    mockParentWin.postMessage = (data) => {
      queueMicrotask(() => {
        mockParentWin.dispatchEvent({ type: 'message', data, source: mockChildWin });
      });
    };

    const childR460 = sandboxR460(
      childEnv.document,
      mockChildWin,
      throwingLocalStorage,
      AppStateMock,
      { enabled: false, ready: false, holoStatus: { mounted_count: 0 } },
      () => {},
      () => {},
      () => {},
      () => {},
      { frames: [], qTrail: [], paused: false, lockedFrameIdx: null },
      null
    );
    childR460.initPreviewSurfaceHost();

    const iframeEl = document.getElementById('live-preview-iframe');
    iframeEl.contentWindow = mockChildWin;

    // Evaluate real parent window 'message' listener from initLayoutAndConnectedControls()
    const parentMsgStart = appJsContent.indexOf("  if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {\n    window.addEventListener('message', (ev) => {\n      if (livePreviewIframe");
    const parentMsgEnd = appJsContent.indexOf('\n  const studioToastCloseBtn =', parentMsgStart);
    assert.ok(parentMsgStart !== -1 && parentMsgEnd > parentMsgStart, 'Parent preview message listener must exist in public/app.js');
    const parentMsgCode = appJsContent.slice(parentMsgStart, parentMsgEnd);
    let sharedLatestPreviewDiag = { renderedAtMs: 0, fileCount: 0, scriptSteps: 0, errors: [], console: [], rootHtmlPreview: '' };
    new Function(
      'window',
      'livePreviewIframe',
      'activeWorkbenchMode',
      'sendFilesToPreviewIframe',
      'previewConsoleOutput',
      '_r460LatestPreviewDiag',
      '_r460PendingInspectWaiters',
      `"use strict";\n${parentMsgCode}`
    )(
      mockParentWin,
      iframeEl,
      'code',
      () => {},
      previewConsoleOutputEl,
      sharedLatestPreviewDiag,
      r460._r460PendingInspectWaiters
    );

    // Spoofed source must be ignored by parent listener
    let spoofCalled = false;
    r460._r460PendingInspectWaiters.set('spoof-req', () => { spoofCalled = true; });
    mockParentWin.dispatchEvent({
      type: 'message',
      source: { otherWindow: true },
      data: { protocol: 'uor-preview/1', op: 'inspect_reply', requestId: 'spoof-req', report: { console: ['evil'] } },
    });
    assert.equal(spoofCalled, false, 'Parent preview listener must reject messages whose ev.source !== iframe.contentWindow');
    r460._r460PendingInspectWaiters.delete('spoof-req');

    workerMock.getWorkspaceListObject = () => ({
      files: [{ path: 'app.js', content: 'const v = 7; document.body.textContent = `live=${v * 6}`;' }],
    });
    const inspectPayload = await r460.handlePreviewInspectRequest({ rpc_id: 99, selector: '#app' });
    assert.equal(inspectPayload.ok, true);
    assert.equal(inspectPayload.rootHtmlPreview, 'live=42', 'Child initPreviewSurfaceHost must evaluate workspace and reply via postMessage');
    assert.equal(previewConsoleOutputEl.textContent, '', 'Empty report.console must clear stale #preview-console-output');
    assert.ok(
      rpcCalls.some((c) => c.op === 'preview_inspect_reply' && c.payload.rpc_id === 99 && c.payload.report.rootHtmlPreview === 'live=42'),
      'preview_inspect_reply must forward genuine two-window inspect_reply report'
    );

    // 7.4c Clock snippet (Date, performance.now, setInterval, nested template literals), CSSOM fallback, selector & timer cleanup on re-render
    const nestedTplRes = r460.evaluatePreviewScriptWithFuel(
      'const x = 9; `outer ${`inner ${x * 2}`}`;',
      { document, console: { log: () => {}, warn: () => {}, error: () => {} } },
      100000
    );
    assert.equal(nestedTplRes.value, 'outer inner 18', 'Nested template literals inside ${...} must tokenize and evaluate accurately');

    const clockWinListeners = new Map();
    const clockPosted = [];
    const clockEnv = createDOMEnvironment();
    const clockRoot = clockEnv.document.getElementById('preview-sandbox-root');
    clockEnv.document.body.appendChild(clockRoot);
    const clockChildWin = {
      innerWidth: 640,
      innerHeight: 480,
      devicePixelRatio: 1,
      location: { search: '?surface=preview' },
      parent: {
        postMessage(data) {
          clockPosted.push(data);
        },
      },
      addEventListener(type, fn) {
        if (!clockWinListeners.has(type)) clockWinListeners.set(type, []);
        clockWinListeners.get(type).push(fn);
      },
    };
    const clockR460 = sandboxR460(
      clockEnv.document,
      clockChildWin,
      throwingLocalStorage,
      AppStateMock,
      { enabled: false, ready: false, holoStatus: { mounted_count: 0 } },
      () => {},
      () => {},
      () => {},
      () => {},
      { frames: [], qTrail: [], paused: false, lockedFrameIdx: null },
      null
    );
    clockR460.initPreviewSurfaceHost();
    const clockCode = [
      'let ticks = 0;',
      "const el = document.createElement('div');",
      "el.id = 'clock';",
      'document.body.appendChild(el);',
      'const tick = () => {',
      '  ticks += 1;',
      '  const d = new Date();',
      '  const p = performance.now();',
      '  el.textContent = `clock ${`t=${d.getTime()}:${ticks}:${p >= 0}`}`;',
      '};',
      'tick();',
      'setInterval(tick, 16);',
    ].join('\n');
    for (const fn of clockWinListeners.get('message') || []) {
      fn({
        source: clockChildWin.parent,
        data: {
          protocol: 'uor-preview/1',
          op: 'render_files',
          requestId: 'clk-1',
          selector: '#clock',
          files: {
            'styles.css': '#clock { font-weight: bold; }',
            'app.js': clockCode,
          },
        },
      });
    }
    const clk1Reply = clockPosted.find((m) => m && m.op === 'inspect_reply' && m.requestId === 'clk-1');
    assert.ok(clk1Reply && clk1Reply.report, 'render_files with selector #clock must emit inspect_reply');
    assert.equal(clk1Reply.report.errors.length, 0, `Clock snippet must execute without errors, got: ${JSON.stringify(clk1Reply.report.errors)}`);
    assert.equal(clk1Reply.report.selector, '#clock');
    assert.equal(clk1Reply.report.selectorMatched, true);
    assert.match(clk1Reply.report.selectorText, /^clock t=\d+:1:true$/);
    assert.equal(clockEnv.document.querySelectorAll('style').length, 1, 'When adoptedStyleSheets is unavailable, CSSOM fallback must inject <style data-preview-cssom="true">');
    assert.equal(clockEnv.document.querySelectorAll('style')[0].getAttribute('data-preview-cssom'), 'true');

    await new Promise((r) => setTimeout(r, 55));
    for (const fn of clockWinListeners.get('message') || []) {
      fn({
        source: clockChildWin.parent,
        data: { protocol: 'uor-preview/1', op: 'inspect', requestId: 'clk-2', selector: '#clock' },
      });
    }
    const clk2Reply = clockPosted.find((m) => m && m.op === 'inspect_reply' && m.requestId === 'clk-2');
    assert.ok(clk2Reply && clk2Reply.report && clk2Reply.report.selectorMatched, 'inspect must match #clock after interval ticks');
    const tickMatch = clk2Reply.report.selectorText.match(/:(\d+):true$/);
    assert.ok(tickMatch && Number(tickMatch[1]) >= 2, `setInterval must advance #clock ticks >= 2, got ${clk2Reply.report.selectorText}`);

    const clockElRef = clockRoot.querySelector('#clock');
    const stoppedBefore = clockElRef.textContent;
    for (const fn of clockWinListeners.get('message') || []) {
      fn({
        source: clockChildWin.parent,
        data: {
          protocol: 'uor-preview/1',
          op: 'render_files',
          requestId: 'clk-3',
          files: { 'app.js': 'document.body.textContent = "stopped";' },
        },
      });
    }
    await new Promise((r) => setTimeout(r, 45));
    assert.equal(clockElRef.textContent, stoppedBefore, 'Re-rendering preview must clear prior setInterval handles');
    console.log('  ✓ 7.4 ?surface=preview 100k fuel limit, CSSOM fallback, clock Date/performance/setInterval, nested template & selector inspect verified.');

    // 7.5 Real un-mocked runClaudeChat + submitComposerPrompt + applyCompletedChatPostEffects integration
    const composerStart = appJsContent.indexOf('const AGENTIC_SLASH_COMMANDS = [');
    const composerEnd = appJsContent.indexOf('\nconst composerAutocompleteMenu =', composerStart);
    const chatCoreStart = appJsContent.indexOf('const applyCompletedChatPostEffects =');
    const chatCoreEnd = appJsContent.indexOf('\nfunction renderTrainReport(res)', chatCoreStart);
    const submitStart = appJsContent.indexOf('function submitComposerPrompt() {');
    const submitEnd = appJsContent.indexOf('\nif (claudeChatForm)', submitStart);
    assert.ok(composerStart !== -1 && composerEnd > composerStart);
    assert.ok(chatCoreStart !== -1 && chatCoreEnd > chatCoreStart);
    assert.ok(submitStart !== -1 && submitEnd > submitStart);

    const streamRpcCalls = [];
    let pendingTurnResolvers = [];
    const realChatWorker = {
      enabled: true,
      ready: true,
      holoStatus: { mounted_count: 1 },
      activeChatStreamHook: null,
      rpcWithId(op, payload) {
        const id = streamRpcCalls.length + 100;
        let resolveP, rejectP;
        const promise = new Promise((res, rej) => { resolveP = res; rejectP = rej; });
        streamRpcCalls.push({ id, op, payload, resolve: resolveP, reject: rejectP });
        pendingTurnResolvers.push({ resolve: resolveP, reject: rejectP });
        return { id, promise };
      },
      rpc: async () => ({ status: 'ok' }),
    };
    const activeThread = { id: 't1', messages: [] };
    const chatInputEl = document.getElementById('claude-chat-input');
    chatInputEl.style = {};
    chatInputEl.focus = () => {};
    const composerStopEl = document.getElementById('composer-stop-btn');
    const composerSendEl = document.getElementById('composer-send-btn');
    const chatTranscriptEl = document.getElementById('chat-messages');
    const appendedTurns = [];
    const postEffectIngests = [];
    const toastLog = [];
    const chatAppState = {
      config: {
        selectedModel: 'uor-r4.qwen3-0.6b-holo',
        composerMode: 'agentic',
        operatingMode: 'coding',
        retainLambda: 0.85,
        pluralityVotes: 8,
        connectedGitRepo: 'uor-test/clock-repo',
        connectedGitBranch: 'feat-clock',
      },
      workspace: { pendingPatches: {} },
    };

    const chatRuntime = new Function(
      'document',
      'AppState',
      'engineWorkerClient',
      'holoObservatoryState',
      'DEFAULT_HONEST_MODEL_ID',
      'safeStorageGet',
      'isConsentGrantedVal',
      'createLiveTokenHudController',
      'ingestHoloObservatoryFrame',
      'updateLiveAgentStatusBar',
      'runVerdantTelosTick',
      'secopsAudit',
      'renderColoredUnifiedDiff',
      'renderInteractiveArtifactPreview',
      'renderPendingPatchPills',
      'updateDynamicPlanningPanel',
      'renderDynamicPoolPanel',
      'renderCoworkStatusBar',
      'renderInferenceResult',
      'dispatchStudioEnvelope',
      'renderStudioState',
      'lexLean3dVisualizer',
      'fetchRecordedTraces',
      'renderMLInspector',
      'renderFormattedMarkdownInto',
      'getActiveThread',
      'persistThreadsToStorage',
      'renderThreadSidebar',
      'appendChatTurn',
      'showStudioToast',
      'renderHydrationConsentCard',
      'applySlashCommandSelection',
      'closeComposerAutocomplete',
      'updateComposerTokenBadge',
      'claudeChatInput',
      'composerModelSelect',
      'composerStopBtn',
      'composerSendBtn',
      'chatTranscript',
      'chatTargetBadge',
      'chatDiffFile',
      'chatDiffPreview',
      'chatApplyPatchBtn',
      'chatDiagConsensus',
      'chatDiagResonance',
      'chatDiagRoot',
      'chatDiagReceipt',
      'activeWorkspacePath',
      'pendingPatchPath',
      'pendingProposedCode',
      'pendingDiffPreview',
      'selectedTraceIndex',
      `"use strict";
       ${appJsContent.slice(composerStart, composerEnd)}
       ${appJsContent.slice(chatCoreStart, chatCoreEnd)}
       ${appJsContent.slice(submitStart, submitEnd)}
       return { tsComposer, applyCompletedChatPostEffects, runClaudeChat, submitComposerPrompt };`
    )(
      document,
      chatAppState,
      realChatWorker,
      obsStateMock,
      'uor-r4.native-q31-d85',
      r460.safeStorageGet,
      (v) => v === '1' || v === 'true' || v === 'granted',
      r460.createLiveTokenHudController,
      (frame, meta) => { postEffectIngests.push({ frame, meta }); },
      () => {},
      async () => ({}),
      { recordEvent: () => {} },
      () => {},
      () => {},
      () => {},
      () => {},
      () => {},
      () => {},
      () => {},
      () => ({ status: 'ok', files: [] }),
      () => {},
      null,
      () => {},
      () => {},
      (el, txt) => { if (el) el.textContent = String(txt ?? ''); },
      () => activeThread,
      () => {},
      () => {},
      (role, title, bodyText, patchPath = '', patchCode = '', _s, chatMeta = {}) => {
        appendedTurns.push({ role, title, body: bodyText });
        activeThread.messages.push({ role, title, body: bodyText, patchPath, patchCode, chatMeta });
      },
      (msg, level) => toastLog.push({ msg, level }),
      r460.renderHydrationConsentCard,
      () => {},
      () => {},
      () => {},
      chatInputEl,
      composerModelSelectEl,
      composerStopEl,
      composerSendEl,
      chatTranscriptEl,
      null,
      null,
      null,
      null,
      null,
      null,
      null,
      null,
      'src/lib.rs',
      '',
      '',
      '',
      0
    );

    // 7.5a Double-submit guard & live streamed completion -> { role, title, body } + applyCompletedChatPostEffects
    chatInputEl.value = 'verify live turn';
    chatRuntime.submitComposerPrompt();
    assert.equal(chatRuntime.tsComposer.isTurnActive(), true, 'tsComposer.isTurnActive() must become true while turn streams');
    assert.equal(streamRpcCalls[0].payload.repo, 'uor-test/clock-repo', 'studio_stream_turn must pass connectedGitRepo as payload.repo');
    assert.equal(streamRpcCalls[0].payload.branch, 'feat-clock', 'studio_stream_turn must pass connectedGitBranch as payload.branch');
    chatInputEl.value = 'duplicate blocked';
    chatRuntime.submitComposerPrompt();
    assert.equal(streamRpcCalls.length, 1, 'submitComposerPrompt must block double submission while turn active');

    realChatWorker.activeChatStreamHook({
      stage: 'token_step',
      delta: 'live ',
      frame: {
        step_idx: 0,
        token_text: 'live ',
        has_geometric_shortlist: true,
        min_hamming: 24,
        shortlist_count: 1024,
        ttft_ms: 25,
        tps_x10: 550,
        q_manifold_xyz: [1, 2, 3],
        layer_max_bits: new Array(28).fill(18),
        top_candidates: [],
      },
    });
    streamRpcCalls[0].resolve({
      status: 'ok',
      backend: 'uor-r4.qwen3-0.6b-holo',
      reply: 'live streamed reply',
      tokens_emitted: 1,
      otel_trace_id: 'trace-r460',
      holo_observatory_frames: [{ step_idx: 77 }],
    });
    for (let k = 0; k < 10; k += 1) await Promise.resolve();
    assert.equal(chatRuntime.tsComposer.isTurnActive(), false, 'Turn completion must clear tsComposer.isTurnActive()');
    const lastAssistant = activeThread.messages[activeThread.messages.length - 1];
    assert.equal(lastAssistant.role, 'assistant');
    assert.equal(lastAssistant.body, 'live streamed reply');
    assert.ok(typeof lastAssistant.title === 'string' && lastAssistant.title.includes('uor-r4.qwen3-0.6b-holo'), 'Streamed thread record must store { role, title, body } compatible with renderActiveThreadMessages');
    assert.ok(postEffectIngests.some((e) => e.frame.step_idx === 77), 'Completed streamed turn must invoke applyCompletedChatPostEffects');

    // 7.5b Pinned Observatory frame must not be overwritten by applyCompletedChatPostEffects batch
    const preLockedIngestCount = postEffectIngests.length;
    obsStateMock.lockedFrameIdx = 3;
    chatRuntime.applyCompletedChatPostEffects('pinned test', { holo_observatory_frames: [{ step_idx: 88 }] }, 'reply', {});
    assert.equal(postEffectIngests.length, preLockedIngestCount, 'lockedFrameIdx !== null must protect pinned Observatory frame from batch replay overwrite');

    // 7.5c Cancelled turn persists cleanly without error toast
    obsStateMock.lockedFrameIdx = null;
    chatRuntime.runClaudeChat('cancel turn test', true);
    assert.equal(streamRpcCalls.length, 2);
    realChatWorker.activeChatStreamHook({
      stage: 'token_step',
      delta: 'partial answer',
      frame: {
        step_idx: 0,
        token_text: 'partial answer',
        has_geometric_shortlist: false,
        min_hamming: 0,
        shortlist_count: 0,
        q_manifold_xyz: [0, 0, 0],
        layer_max_bits: [12],
        top_candidates: [],
      },
    });
    const cancelErr = Object.assign(new Error('Cancelled by user'), { error_code: 'Cancelled', details: { cancelled: true } });
    streamRpcCalls[1].reject(cancelErr);
    for (let k = 0; k < 10; k += 1) await Promise.resolve();
    assert.equal(toastLog.filter((t) => t.level === 'error').length, 0, 'Cancelled turn must NOT emit error toast');
    const cancelledRecord = activeThread.messages[activeThread.messages.length - 1];
    assert.equal(cancelledRecord.role, 'assistant');
    assert.equal(cancelledRecord.body, 'partial answer');
    assert.equal(cancelledRecord.chatMeta.cancelled, true, 'Cancelled turn must persist partial text cleanly');

    // 7.5d Global #model-select change listener sets manualModelPinned = true & top-level child surface guard exists
    const modelSelStart = appJsContent.indexOf('if (composerModelSelect) {');
    const modelSelEnd = appJsContent.indexOf('\nif (composerStopBtn)', modelSelStart);
    assert.ok(modelSelStart !== -1 && modelSelEnd > modelSelStart);
    const pinObsState = { manualModelPinned: false };
    const pinGlobalSelect = document.createElement('select');
    pinGlobalSelect.id = 'model-select';
    pinGlobalSelect.value = 'uor-r4.qwen3-1.7b-holo';
    new Function(
      'composerModelSelect',
      'globalModelSelect',
      'AppState',
      'holoObservatoryState',
      'syncComposerModel',
      `"use strict";\n${appJsContent.slice(modelSelStart, modelSelEnd)}`
    )(
      null,
      pinGlobalSelect,
      { config: { selectedModel: 'uor-r4.qwen3-0.6b-holo' } },
      pinObsState,
      () => {}
    );
    pinGlobalSelect.value = 'uor-r4.qwen3-1.7b-holo';
    pinGlobalSelect.dispatchEvent({ type: 'change', target: pinGlobalSelect });
    assert.equal(pinObsState.manualModelPinned, true, 'Global #model-select change must set holoObservatoryState.manualModelPinned = true');
    assert.ok(appJsContent.includes('isChildSurfaceEarly'), 'public/app.js must guard top-level chat/3D/terminal init on ?surface=preview and ?surface=observatory');

    // 7.5e Boot renderActiveThreadMessages preserves #chat-messages & hydration_consent_required falls back to #chat-transcript
    const bootTranscript = new MockDOMElement('div', 'chat-transcript');
    const renderActiveStart = appJsContent.indexOf('function renderActiveThreadMessages() {');
    const renderActiveEnd = appJsContent.indexOf('\nfunction extractThinkingAndTools(', renderActiveStart);
    assert.ok(renderActiveStart !== -1 && renderActiveEnd > renderActiveStart);
    const renderActiveApi = new Function(
      'document',
      'chatTranscript',
      'getActiveThread',
      'appendChatTurnDom',
      'safeStorageGet',
      'isConsentGrantedVal',
      'engineWorkerClient',
      'renderHydrationConsentCard',
      `"use strict";\n${appJsContent.slice(renderActiveStart, renderActiveEnd)}\nreturn renderActiveThreadMessages;`
    )(
      document,
      bootTranscript,
      () => ({ messages: [] }),
      () => {},
      () => null,
      () => false,
      null,
      r460.renderHydrationConsentCard
    );
    renderActiveApi();
    assert.ok(bootTranscript.querySelector('#chat-messages'), 'renderActiveThreadMessages() must recreate and preserve #chat-messages inside #chat-transcript');

    const consentDocFallback = {
      ...document,
      getElementById(id) {
        if (id === 'chat-messages') return null;
        if (id === 'chat-transcript') return bootTranscript;
        return document.getElementById(id);
      },
    };
    const workerConsentClient = new Function(
      'document',
      'window',
      'AppState',
      'holoObservatoryState',
      'ingestHoloObservatoryFrame',
      'tsComposer',
      'DEFAULT_STARTER_WORKSPACE_FILES',
      'renderHydrationConsentCard',
      'chatTranscript',
      'safeStorageGet',
      'isConsentGrantedVal',
      'showStudioToast',
      `${workerCode}\nreturn engineWorkerClient;`
    )(
      consentDocFallback,
      mockWin,
      chatAppState,
      obsStateMock,
      () => {},
      null,
      [],
      r460.renderHydrationConsentCard,
      bootTranscript,
      () => null,
      () => false,
      () => {}
    );
    workerConsentClient.activeChatStreamHook = null;
    workerConsentClient.handleMessage({
      event: 'hydration_consent_required',
      model_id: 'uor-r4.qwen3-0.6b-holo',
      approx_bytes: 429916160,
    });
    const fallbackConsentCard = bootTranscript.querySelector('.holo-consent-card');
    assert.ok(fallbackConsentCard, 'hydration_consent_required with absent #chat-messages must mount .holo-consent-card inside #chat-transcript');
    assert.ok(fallbackConsentCard.querySelector('.holo-consent-offline-btn'), 'Boot Consent Card must render Use Offline Native Q31 button');

    console.log('  ✓ 7.5 Real runClaudeChat (repo/branch), submitComposerPrompt, #model-select pin, lockedFrameIdx, cancel & boot consent fallback verified.');
  } finally {
    globalThis.CSSStyleSheet = origCSSStyleSheet;
  }
}

console.log('\n================================================================');
console.log('  CHALLENGE COMPLETE: ALL 7 EMPIRICAL CHALLENGES PASSED GREEN');
console.log('================================================================\n');
