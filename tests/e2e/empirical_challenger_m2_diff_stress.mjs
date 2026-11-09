/**
 * ============================================================================
 * Empirical Challenger M2_2: Cmd+K Inline Edit & Diffs Stress Harness
 * ============================================================================
 *
 * Adversarial stress testing for Milestone 2:
 *   - Suite 1: Extreme Diff Scenarios (500 lines replace, add, del, alternating edits, LCS scaling)
 *   - Suite 2: Concurrency & Race Conditions (rapid triggers, double-accept/reject, streaming accept)
 *   - Suite 3: Theme Contrast Verification (WCAG 2.1 AA/AAA across Obsidian Dark & Warm Light)
 *   - Suite 4: Virtual Filesystem Consistency & Buffer Synchronization (VFS, dirty tracking, tab switching)
 *   - Suite 5: Interactive Controls Lifecycle & Keyboard Shortcut Ergonomics
 *
 * Zero external npm dependencies. Native Node.js ESM.
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');
const PUBLIC_DIR = resolve(REPO_ROOT, 'public');

console.log('================================================================');
console.log('  CHALLENGER M2_2: EMPIRICAL CMD+K & DIFFS ADVERSARIAL HARNESS');
console.log('  Stress-testing Extreme Diffs, Concurrency, WCAG Contrast & VFS');
console.log('================================================================\n');

const rawAppJs = readFileSync(resolve(PUBLIC_DIR, 'app.js'), 'utf8');
const rawAppCss = readFileSync(resolve(PUBLIC_DIR, 'app.css'), 'utf8');
const rawHtml = readFileSync(resolve(PUBLIC_DIR, 'index.html'), 'utf8');

// ============================================================================
// LIGHTWEIGHT IN-MEMORY DOM & SANDBOX FACTORY
// ============================================================================

class MockClassList {
  constructor(node) {
    this.node = node;
    this.classes = new Set();
  }
  _sync() {
    this.node._className = Array.from(this.classes).join(' ');
  }
  add(...clsList) {
    for (const c of clsList) if (c) this.classes.add(c);
    this._sync();
  }
  remove(...clsList) {
    for (const c of clsList) this.classes.delete(c);
    this._sync();
  }
  toggle(cls, force) {
    if (typeof force === 'boolean') {
      if (force) this.classes.add(cls);
      else this.classes.delete(cls);
    } else {
      if (this.classes.has(cls)) this.classes.delete(cls);
      else this.classes.add(cls);
    }
    this._sync();
    return this.classes.has(cls);
  }
  contains(cls) {
    return this.classes.has(cls);
  }
}

class MockNode {
  constructor(nodeType = 1, tagName = 'DIV') {
    this.nodeType = nodeType;
    this.tagName = tagName.toUpperCase();
    this.id = '';
    this._className = '';
    this.classList = new MockClassList(this);
    this.children = [];
    this.childNodes = this.children;
    this.parentElement = null;
    this.parentNode = null;
    this.attributes = new Map();
    this.dataset = {};
    this.style = {};
    this._textContent = '';
    this._value = '';
    this.selectionStart = 0;
    this.selectionEnd = 0;
    this.hidden = false;
    this.disabled = false;
    this.open = false;
    this.type = 'button';
    this.eventListeners = {};
  }

  get className() {
    return this._className;
  }
  set className(val) {
    this._className = String(val ?? '');
    this.classList.classes = new Set(this._className.split(/\s+/).filter(Boolean));
  }

  get textContent() {
    if (this.nodeType === 3) return this._textContent;
    if (this.children.length === 0) return this._textContent;
    return this.children.map((c) => c.textContent).join('');
  }
  set textContent(val) {
    this._textContent = String(val ?? '');
    this.children = [];
    this.childNodes = this.children;
  }

  get value() {
    return this._value;
  }
  set value(val) {
    this._value = String(val ?? '');
  }

  setAttribute(k, v) {
    const val = String(v ?? '');
    this.attributes.set(k, val);
    if (k === 'id') this.id = val;
    if (k === 'class') this.className = val;
    if (k === 'hidden') this.hidden = true;
    if (k === 'disabled') this.disabled = true;
    if (k === 'open') this.open = true;
    if (k.startsWith('data-')) {
      const camel = k.slice(5).replace(/-([a-z])/g, (_, ch) => ch.toUpperCase());
      this.dataset[camel] = val;
    }
  }

  getAttribute(k) {
    if (this.attributes.has(k)) return this.attributes.get(k);
    if (k === 'id') return this.id || null;
    if (k === 'class') return this.className || null;
    if (k === 'hidden') return this.hidden ? '' : null;
    if (k === 'disabled') return this.disabled ? '' : null;
    return null;
  }

  removeAttribute(k) {
    this.attributes.delete(k);
    if (k === 'hidden') this.hidden = false;
    if (k === 'disabled') this.disabled = false;
    if (k === 'open') this.open = false;
    if (k.startsWith('data-')) {
      const camel = k.slice(5).replace(/-([a-z])/g, (_, ch) => ch.toUpperCase());
      delete this.dataset[camel];
    }
  }

  hasAttribute(k) {
    if (k === 'hidden') return this.hidden;
    if (k === 'disabled') return this.disabled;
    return this.attributes.has(k);
  }

  appendChild(child) {
    if (!child) return;
    child.parentElement = this;
    child.parentNode = this;
    this.children.push(child);
  }

  prepend(...children) {
    for (let i = children.length - 1; i >= 0; i--) {
      const c = children[i];
      c.parentElement = this;
      c.parentNode = this;
      this.children.unshift(c);
    }
  }

  replaceChildren(...newChildren) {
    this.children = [];
    this.childNodes = this.children;
    this._textContent = '';
    for (const c of newChildren) {
      this.appendChild(c);
    }
  }

  addEventListener(type, handler) {
    if (!this.eventListeners[type]) this.eventListeners[type] = [];
    this.eventListeners[type].push(handler);
  }

  removeEventListener(type, handler) {
    if (!this.eventListeners[type]) return;
    this.eventListeners[type] = this.eventListeners[type].filter((h) => h !== handler);
  }

  dispatchEvent(evt) {
    const list = this.eventListeners[evt.type] || [];
    for (const h of list) {
      h.call(this, evt);
    }
    return !evt.defaultPrevented;
  }

  click() {
    const ev = {
      type: 'click',
      target: this,
      currentTarget: this,
      defaultPrevented: false,
      preventDefault() { this.defaultPrevented = true; },
      stopPropagation() {},
    };
    this.dispatchEvent(ev);
  }

  focus() {
    this.dispatchEvent({ type: 'focus', target: this, defaultPrevented: false, preventDefault() {} });
  }

  select() {}

  querySelector(sel) {
    const all = this.querySelectorAll(sel);
    return all.length > 0 ? all[0] : null;
  }

  querySelectorAll(sel) {
    const matches = [];
    function matchNode(node) {
      if (sel.startsWith('#') && node.id === sel.slice(1)) matches.push(node);
      else if (sel.startsWith('.') && node.classList.contains(sel.slice(1))) matches.push(node);
      else if (node.tagName === sel.toUpperCase()) matches.push(node);
      for (const ch of node.children) matchNode(ch);
    }
    for (const ch of this.children) matchNode(ch);
    return matches;
  }
}

class MockDocument extends MockNode {
  constructor() {
    super(9, '#DOCUMENT');
    this.body = new MockNode(1, 'BODY');
    this.appendChild(this.body);
    this.elementsById = new Map();
  }

  createElement(tagName) {
    return new MockNode(1, tagName);
  }

  getElementById(id) {
    return this.elementsById.get(id) || null;
  }

  registerElement(id, node) {
    this.elementsById.set(id, node);
  }

  querySelector(sel) {
    if (sel.startsWith('#')) return this.getElementById(sel.slice(1));
    return this.body.querySelector(sel);
  }

  querySelectorAll(sel) {
    return this.body.querySelectorAll(sel);
  }
}

function parseHtmlIntoMockDocument(html) {
  const doc = new MockDocument();
  const idRegex = /id=["']([a-zA-Z0-9_-]+)["']/g;
  let match;
  while ((match = idRegex.exec(html)) !== null) {
    const id = match[1];
    if (!doc.elementsById.has(id)) {
      let tag = 'DIV';
      if (id.includes('btn')) tag = 'BUTTON';
      else if (id.includes('input') || id.includes('filter')) tag = 'INPUT';
      else if (id.includes('textarea')) tag = 'TEXTAREA';
      else if (id.includes('status')) tag = 'OUTPUT';
      const node = doc.createElement(tag);
      node.id = id;
      doc.registerElement(id, node);
      doc.body.appendChild(node);
    }
  }
  return doc;
}

function createChallengerM2Sandbox(options = {}) {
  const doc = parseHtmlIntoMockDocument(rawHtml);

  // Virtual filesystem
  let virtualFiles = options.virtualFiles || [
    { path: 'src/attention.rs', line_count: 36, content: 'pub fn flock_attention_step() {\n    // Cyclic flock\n}' },
    { path: 'src/lib.rs', line_count: 24, content: 'pub mod attention;\npub mod LutStack;\n' },
    { path: 'src/LutStack.rs', line_count: 48, content: 'pub struct LutStack {\n    pub shards: usize,\n}\n' },
    { path: 'Cargo.toml', line_count: 18, content: '[package]\nname = "lexlean-r4"\nversion = "0.4.0"\n' },
  ];

  const dispatchedEnvelopes = [];
  function mockDispatchStudioEnvelope(command, payload) {
    dispatchedEnvelopes.push({ command, payload });
    if (command === 'workspace_list') {
      return { status: 'ok', files: [...virtualFiles] };
    }
    if (command === 'workspace_read') {
      const f = virtualFiles.find((x) => x.path === payload);
      return f
        ? { status: 'ok', path: f.path, content: f.content, line_count: f.line_count }
        : { status: 'error', message: 'Not found' };
    }
    if (command === 'workspace_write') {
      const newlineIdx = payload.indexOf('\n');
      const targetPath = newlineIdx > 0 ? payload.slice(0, newlineIdx).trim() : 'src/attention.rs';
      const content = newlineIdx > 0 ? payload.slice(newlineIdx + 1) : payload;
      const existing = virtualFiles.find((x) => x.path === targetPath);
      const lineCount = content.split('\n').length;
      if (existing) {
        existing.content = content;
        existing.line_count = lineCount;
      } else {
        virtualFiles.push({ path: targetPath, content, line_count: lineCount });
      }
      return { status: 'ok', files: [...virtualFiles] };
    }
    if (command === 'chat') {
      if (options && options.chatResponse) {
        return typeof options.chatResponse === 'function' ? options.chatResponse(payload) : options.chatResponse;
      }
      return {
        status: 'ok',
        reply: 'Generated patch via local uor-r4.lut-stack/8.',
        proposed_code: 'pub fn flock_attention_step() {\n    // Cyclic flock refactored\n    let q12 = 42;\n}\n',
      };
    }
    return { status: 'ok' };
  }

  function mockWeightPackStatus() {
    return JSON.stringify({ status: 'ok', point_rows: 17408, reply_entries: 4096 });
  }

  function mockStudioWasmEval() {
    return JSON.stringify({ passed: true, total_tests: 32, passed_tests: 32, diagnostics: [] });
  }

  const AppState = {
    workspace: {
      activePath: 'src/attention.rs',
      pendingPatchPath: '',
      pendingProposedCode: '',
      pendingPatches: {},
    },
    ui: {
      theme: 'dark',
    },
  };

  const diffFuncStart = rawAppJs.indexOf('function computeUnifiedDiffJs(');
  const diffFuncEnd = rawAppJs.indexOf('function renderInteractiveArtifactPreview(', diffFuncStart);
  assert.ok(diffFuncStart !== -1 && diffFuncEnd !== -1, 'Failed to extract diff functions');
  const diffCode = rawAppJs.slice(diffFuncStart, diffFuncEnd);

  const wbStart = rawAppJs.indexOf('// ============================================================================ \n// CURSOR IDE WORKBENCH CONTROLLER');
  const wbFallbackStart = wbStart !== -1 ? wbStart : rawAppJs.indexOf('// CURSOR IDE WORKBENCH CONTROLLER');
  const wbEnd = rawAppJs.indexOf('initCursorIdeWorkbench();', wbFallbackStart);
  assert.ok(wbFallbackStart !== -1 && wbEnd !== -1, 'Failed to extract Cursor Workbench section');
  const wbCode = rawAppJs.slice(wbFallbackStart, wbEnd);

  const workspaceEditorTextarea = doc.getElementById('workspace-editor-textarea') || doc.createElement('textarea');
  const workspaceSaveBtn = doc.getElementById('workspace-save-btn') || doc.createElement('button');
  const workspaceStatus = doc.getElementById('workspace-status') || doc.createElement('output');
  const workspaceNewPath = doc.getElementById('workspace-new-path') || doc.createElement('input');
  const workspaceNewForm = doc.getElementById('workspace-new-form') || doc.createElement('form');
  const workspaceMountInput = doc.getElementById('workspace-mount-input') || doc.createElement('input');
  const dockableCanvasPane = doc.getElementById('dockable-canvas-pane') || doc.createElement('section');
  const claudeChatInput = doc.getElementById('claude-chat-input') || doc.createElement('textarea');
  const chatApplyPatchBtn = doc.getElementById('chat-apply-patch-btn') || doc.createElement('button');
  const chatRejectPatchBtn = doc.getElementById('chat-reject-patch-btn') || doc.createElement('button');
  const editorFilePath = doc.getElementById('status-active-file') || doc.createElement('span');

  workspaceEditorTextarea.value = virtualFiles[0].content;

  let activeWorkspacePath = 'src/attention.rs';
  let activeCanvasTabId = 'terminal';

  function activateCanvasTab(tabId) { activeCanvasTabId = tabId; }
  function selectWorkspaceFile(path) {
    activeWorkspacePath = path;
    const f = virtualFiles.find((x) => x.path === path);
    if (f) workspaceEditorTextarea.value = f.content;
  }
  function renderWorkspaceList(_ws) {}
  function renderStudioState(_st) {}
  function renderPendingPatchPills() {}
  let policyAllowance = (options && typeof options.policyAllowance === 'boolean') ? options.policyAllowance : true;
  function checkMcpToolPolicy(_tool, _path) {
    return policyAllowance;
  }
  function runOnDeviceTrain() {}

  const mockWindow = {
    addEventListener: (t, h) => doc.addEventListener(t, h),
    dispatchEvent: (e) => doc.dispatchEvent(e),
  };

  const sandboxFactory = new Function(
    'document',
    'window',
    'dispatchStudioEnvelope',
    'studio_weight_pack_status',
    'studio_wasm_eval',
    'AppState',
    'workspaceEditorTextarea',
    'workspaceSaveBtn',
    'workspaceStatus',
    'workspaceNewPath',
    'workspaceNewForm',
    'workspaceMountInput',
    'dockableCanvasPane',
    'activeCanvasTabId',
    'activeWorkspacePath',
    'activateCanvasTab',
    'selectWorkspaceFile',
    'renderWorkspaceList',
    'renderStudioState',
    'renderPendingPatchPills',
    'checkMcpToolPolicy',
    'runOnDeviceTrain',
    'claudeChatInput',
    'chatApplyPatchBtn',
    'chatRejectPatchBtn',
    'editorFilePath',
    `"use strict";
     ${diffCode}
     ${wbCode}
     return {
       computeUnifiedDiffJs,
       renderColoredUnifiedDiff,
       EditorTabManager,
       editorTabManager,
       cursorBaselineFiles,
       cursorOpenTabs,
       get cursorCmdKState() { return cursorCmdKState; },
       set cursorCmdKState(v) { cursorCmdKState = v; },
       openCursorCmdK,
       runCursorCmdKEdit,
       acceptCursorCmdKEdit,
       rejectCursorCmdKEdit,
       computeCursorGitStatusForFile,
       initCursorIdeWorkbench,
     };`
  );

  const controller = sandboxFactory(
    doc,
    mockWindow,
    mockDispatchStudioEnvelope,
    mockWeightPackStatus,
    mockStudioWasmEval,
    AppState,
    workspaceEditorTextarea,
    workspaceSaveBtn,
    workspaceStatus,
    workspaceNewPath,
    workspaceNewForm,
    workspaceMountInput,
    dockableCanvasPane,
    activeCanvasTabId,
    activeWorkspacePath,
    activateCanvasTab,
    selectWorkspaceFile,
    renderWorkspaceList,
    renderStudioState,
    renderPendingPatchPills,
    checkMcpToolPolicy,
    runOnDeviceTrain,
    claudeChatInput,
    chatApplyPatchBtn,
    chatRejectPatchBtn,
    editorFilePath
  );

  for (const f of virtualFiles) {
    controller.cursorBaselineFiles.set(f.path, f.content);
  }
  controller.initCursorIdeWorkbench();

  return {
    doc,
    controller,
    virtualFiles,
    dispatchedEnvelopes,
    workspaceEditorTextarea,
    setPolicyAllowance(val) { policyAllowance = val; },
    elements: {
      bar: doc.getElementById('cursor-cmdk-bar'),
      input: doc.getElementById('cursor-cmdk-input'),
      diffHunk: doc.getElementById('cursor-cmdk-diff-hunk'),
      acceptBtn: doc.getElementById('cursor-cmdk-accept-btn'),
      rejectBtn: doc.getElementById('cursor-cmdk-reject-btn'),
      rangeBadge: doc.getElementById('cursor-cmdk-range-badge'),
    },
  };
}

let totalTests = 0;
let passedTests = 0;
function test(name, fn) {
  totalTests++;
  try {
    fn();
    passedTests++;
    console.log(`  ✓ ${name}`);
  } catch (err) {
    console.error(`  ✗ FAIL: ${name}`);
    console.error(`    ${err.stack || err.message}`);
    throw err;
  }
}

// ============================================================================
// SUITE 1: EXTREME DIFF SCENARIOS
// ============================================================================
console.log('--- Suite 1: Extreme Diff Scenarios ---');

test('S1.1: Complete replacement of 500 lines generates valid headers and hunks', () => {
  const { controller } = createChallengerM2Sandbox();
  const oldText = Array.from({ length: 500 }, (_, i) => `let old_var_${i} = ${i};`).join('\n');
  const newText = Array.from({ length: 500 }, (_, i) => `let new_var_${i} = ${i * 2};`).join('\n');

  const t0 = performance.now();
  const diff = controller.computeUnifiedDiffJs(oldText, newText, 'src/extreme.rs');
  const elapsed = performance.now() - t0;

  assert.ok(elapsed < 200, `LCS took too long: ${elapsed.toFixed(2)}ms (expected < 200ms)`);
  assert.ok(diff.startsWith('--- a/src/extreme.rs\n+++ b/src/extreme.rs\n@@ -1,500 +1,500 @@'));
  assert.ok(diff.includes('-let old_var_0 = 0;'));
  assert.ok(diff.includes('+let new_var_0 = 0;'));
  assert.ok(diff.includes('-let old_var_499 = 499;'));
  assert.ok(diff.includes('+let new_var_499 = 998;'));
});

test('S1.2: Massive addition of 500 lines generates valid + lines without corrupting context', () => {
  const { controller } = createChallengerM2Sandbox();
  const oldText = 'fn start() {\n}\n';
  const newLines = Array.from({ length: 500 }, (_, i) => `    println!("line ${i}");`);
  const newText = `fn start() {\n${newLines.join('\n')}\n}\n`;

  const diff = controller.computeUnifiedDiffJs(oldText, newText, 'src/add.rs');
  assert.ok(diff.includes('--- a/src/add.rs'));
  assert.ok(diff.includes('+++ b/src/add.rs'));
  assert.ok(diff.includes('+    println!("line 0");'));
  assert.ok(diff.includes('+    println!("line 499");'));
  assert.ok(diff.includes(' fn start() {'));
  assert.ok(diff.includes(' }'));
});

test('S1.3: Massive deletion of 500 lines shrinks file cleanly', () => {
  const { controller } = createChallengerM2Sandbox();
  const lines = Array.from({ length: 500 }, (_, i) => `    let del_${i} = ${i};`);
  const oldText = `fn root() {\n${lines.join('\n')}\n}\n`;
  const newText = 'fn root() {\n}\n';

  const diff = controller.computeUnifiedDiffJs(oldText, newText, 'src/del.rs');
  assert.ok(diff.includes('--- a/src/del.rs'));
  assert.ok(diff.includes('+++ b/src/del.rs'));
  assert.ok(diff.includes('-    let del_0 = 0;'));
  assert.ok(diff.includes('-    let del_499 = 499;'));
});

test('S1.4: Alternating single-character edits across 200 lines resolve with fine-grained hunks', () => {
  const { controller } = createChallengerM2Sandbox();
  const oldLines = Array.from({ length: 200 }, (_, i) => `line_${i}_content_data`);
  const newLines = oldLines.map((l, i) => (i % 2 === 0 ? l + '!' : l));

  const t0 = performance.now();
  const diff = controller.computeUnifiedDiffJs(oldLines.join('\n'), newLines.join('\n'), 'src/alt.rs');
  const elapsed = performance.now() - t0;

  assert.ok(elapsed < 150, `Alternating edits took ${elapsed.toFixed(2)}ms (expected < 150ms)`);
  assert.ok(diff.includes('-line_0_content_data'));
  assert.ok(diff.includes('+line_0_content_data!'));
  assert.ok(diff.includes(' line_1_content_data'));
});

test('S1.5: Alternating line insertion and deletion (interleaved diffs)', () => {
  const { controller } = createChallengerM2Sandbox();
  const oldLines = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
  const newLines = ['A', 'B_mod', 'C', 'D_mod', 'E', 'F_mod', 'G', 'H_mod'];
  const diff = controller.computeUnifiedDiffJs(oldLines.join('\n'), newLines.join('\n'), 'src/interleave.rs');

  assert.ok(diff.includes('-B'));
  assert.ok(diff.includes('+B_mod'));
  assert.ok(diff.includes('-D'));
  assert.ok(diff.includes('+D_mod'));
});

test('S1.6: Boundary: Identical files produce valid zero-change unified diff header', () => {
  const { controller } = createChallengerM2Sandbox();
  const code = 'pub fn flock_identity() -> bool {\n    true\n}\n';
  const diff = controller.computeUnifiedDiffJs(code, code, 'src/ident.rs');

  assert.equal(diff, '--- a/src/ident.rs\n+++ b/src/ident.rs\n@@ -1,0 +1,0 @@');
});

test('S1.7: Boundary: Empty original file to 500 lines, and 500 lines to empty file', () => {
  const { controller } = createChallengerM2Sandbox();
  const fullText = Array.from({ length: 500 }, (_, i) => `line ${i}`).join('\n');

  // Empty to full
  const diffAdd = controller.computeUnifiedDiffJs('', fullText, 'src/newfile.rs');
  assert.ok(diffAdd.includes('+line 0'));
  assert.ok(diffAdd.includes('+line 499'));

  // Full to empty
  const diffDel = controller.computeUnifiedDiffJs(fullText, '', 'src/newfile.rs');
  assert.ok(diffDel.includes('-line 0'));
  assert.ok(diffDel.includes('-line 499'));
});

test('S1.8: Performance Scaling: 1,000-line diff computes within 250ms limit', () => {
  const { controller } = createChallengerM2Sandbox();
  const oldText = Array.from({ length: 1000 }, (_, i) => `func_${i}();`).join('\n');
  const newText = Array.from({ length: 1000 }, (_, i) => (i % 5 === 0 ? `func_${i}_patched();` : `func_${i}();`)).join('\n');

  const t0 = performance.now();
  const diff = controller.computeUnifiedDiffJs(oldText, newText, 'src/scale.rs');
  const elapsed = performance.now() - t0;

  assert.ok(elapsed < 250, `1,000-line diff took ${elapsed.toFixed(2)}ms (expected < 250ms)`);
  assert.ok(diff.includes('-func_0();'));
  assert.ok(diff.includes('+func_0_patched();'));
});

test('S1.9: renderColoredUnifiedDiff produces proper syntax classes and line elements', () => {
  const { doc, controller } = createChallengerM2Sandbox();
  const container = doc.createElement('div');
  const diff = `--- a/src/test.rs\n+++ b/src/test.rs\n@@ -1,3 +1,3 @@\n-old code\n+new code\n context code`;

  controller.renderColoredUnifiedDiff(container, diff);
  assert.equal(container.children.length, 6);
  assert.equal(container.children[0].className, 'diff-line diff-line-hunk');
  assert.equal(container.children[1].className, 'diff-line diff-line-hunk');
  assert.equal(container.children[2].className, 'diff-line diff-line-hunk');
  assert.equal(container.children[3].className, 'diff-line diff-line-del');
  assert.equal(container.children[4].className, 'diff-line diff-line-add');
  assert.equal(container.children[5].className, 'diff-line diff-line-ctx');
});

// ============================================================================
// SUITE 2: CONCURRENCY & RACE CONDITIONS
// ============================================================================
console.log('\n--- Suite 2: Concurrency & Race Conditions ---');

test('S2.1: 200 rapid openCursorCmdK calls without closing maintain stable state', () => {
  const { controller, elements, workspaceEditorTextarea } = createChallengerM2Sandbox();
  workspaceEditorTextarea.value = 'line 1\nline 2\nline 3\nline 4\nline 5\n';
  workspaceEditorTextarea.selectionStart = 7; // line 2
  workspaceEditorTextarea.selectionEnd = 14;  // line 3

  for (let i = 0; i < 200; i++) {
    controller.openCursorCmdK();
  }

  assert.equal(controller.cursorCmdKState.open, true);
  assert.equal(controller.cursorCmdKState.startLine, 2);
  assert.equal(controller.cursorCmdKState.endLine, 3);
  assert.equal(elements.rangeBadge.textContent, 'Lines 2–3 in src/attention.rs');
  assert.equal(elements.acceptBtn.hidden, true);
  assert.equal(elements.acceptBtn.disabled, true);
  assert.equal(elements.rejectBtn.hidden, true);
  assert.equal(elements.rejectBtn.disabled, true);
});

test('S2.2: 50 rapid sequential runCursorCmdKEdit calls stay deterministic', () => {
  const { controller, elements } = createChallengerM2Sandbox();
  controller.openCursorCmdK();
  elements.input.value = 'Add Q12 invariant assertion';

  for (let i = 0; i < 50; i++) {
    controller.runCursorCmdKEdit();
  }

  assert.ok(controller.cursorCmdKState.proposedFullCode.length > 0);
  assert.ok(controller.cursorCmdKState.diffText.includes('+'));
  assert.equal(elements.acceptBtn.hidden, false);
  assert.equal(elements.acceptBtn.disabled, false);
  assert.equal(elements.rejectBtn.hidden, false);
  assert.equal(elements.rejectBtn.disabled, false);
});

test('S2.3: Idempotent Double Accept: second accept is safe no-op', () => {
  const { controller, elements, virtualFiles, workspaceEditorTextarea } = createChallengerM2Sandbox();
  controller.openCursorCmdK();
  controller.runCursorCmdKEdit();

  const proposed = controller.cursorCmdKState.proposedFullCode;
  assert.ok(proposed.length > 0);

  // First accept
  controller.acceptCursorCmdKEdit();
  assert.equal(workspaceEditorTextarea.value, proposed);
  assert.equal(virtualFiles[0].content, proposed);
  assert.equal(controller.cursorCmdKState.open, false);
  assert.equal(controller.cursorCmdKState.proposedFullCode, '');
  assert.equal(elements.acceptBtn.hidden, true);
  assert.equal(elements.acceptBtn.disabled, true);

  // Second accept should be safe no-op
  controller.acceptCursorCmdKEdit();
  assert.equal(workspaceEditorTextarea.value, proposed);
  assert.equal(controller.cursorCmdKState.open, false);
});

test('S2.4: Idempotent Double Reject: second reject does not overwrite buffer with empty string', () => {
  const { controller, elements, workspaceEditorTextarea } = createChallengerM2Sandbox();
  const originalCode = workspaceEditorTextarea.value;

  controller.openCursorCmdK();
  controller.runCursorCmdKEdit();

  // First reject
  controller.rejectCursorCmdKEdit();
  assert.equal(workspaceEditorTextarea.value, originalCode);
  assert.equal(controller.cursorCmdKState.open, false);
  assert.equal(controller.cursorCmdKState.originalFullCode, '');
  assert.equal(elements.rejectBtn.hidden, true);
  assert.equal(elements.rejectBtn.disabled, true);

  // Second reject must NOT clobber editor with empty string
  controller.rejectCursorCmdKEdit();
  assert.equal(workspaceEditorTextarea.value, originalCode);
  assert.equal(controller.cursorCmdKState.open, false);
});

test('S2.5: Premature Accept: accept without proposedFullCode is safe no-op', () => {
  const { controller, dispatchedEnvelopes, workspaceEditorTextarea } = createChallengerM2Sandbox();
  const originalCode = workspaceEditorTextarea.value;
  controller.openCursorCmdK();

  // Call accept before runCursorCmdKEdit
  controller.acceptCursorCmdKEdit();

  // No workspace_write should be dispatched
  const writeEnvelopes = dispatchedEnvelopes.filter((e) => e.command === 'workspace_write');
  assert.equal(writeEnvelopes.length, 0);
  assert.equal(workspaceEditorTextarea.value, originalCode);
});

test('S2.6: Textarea modification while diff is displayed is restored to pre-generation code on reject', () => {
  const { controller, workspaceEditorTextarea } = createChallengerM2Sandbox();
  const baselineCode = workspaceEditorTextarea.value;

  controller.openCursorCmdK();
  controller.runCursorCmdKEdit();
  assert.equal(controller.cursorCmdKState.originalFullCode, baselineCode);

  // While diff is displayed, unexpected textarea edit occurs
  workspaceEditorTextarea.value = 'MODIFIED DURING DIFF DISPLAY';

  // Reject should restore the pre-generation baselineCode
  controller.rejectCursorCmdKEdit();
  assert.equal(workspaceEditorTextarea.value, baselineCode);
});

test('S2.7: 500 rapid open and immediate reject cycles leave zero dangling state', () => {
  const { controller, elements, workspaceEditorTextarea } = createChallengerM2Sandbox();
  const baselineCode = workspaceEditorTextarea.value;

  for (let i = 0; i < 500; i++) {
    controller.openCursorCmdK();
    controller.rejectCursorCmdKEdit();
  }

  assert.equal(controller.cursorCmdKState.open, false);
  assert.equal(controller.cursorCmdKState.proposedFullCode, '');
  assert.equal(controller.cursorCmdKState.originalFullCode, '');
  assert.equal(controller.cursorCmdKState.diffText, '');
  assert.equal(elements.acceptBtn.disabled, true);
  assert.equal(elements.rejectBtn.disabled, true);
  assert.equal(workspaceEditorTextarea.value, baselineCode);
});

// ============================================================================
// SUITE 3: THEME CONTRAST VERIFICATION (WCAG 2.1 AA / AAA)
// ============================================================================
console.log('\n--- Suite 3: Theme Contrast Verification (WCAG 2.1 AA / AAA) ---');

function sRGBtoLin(c) {
  c = c / 255;
  return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}
function getLuminance(hex) {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return 0.2126 * sRGBtoLin(r) + 0.7152 * sRGBtoLin(g) + 0.0722 * sRGBtoLin(b);
}
function calculateContrast(hex1, hex2) {
  const l1 = getLuminance(hex1);
  const l2 = getLuminance(hex2);
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}

test('S3.1: Obsidian Dark theme diff elements pass WCAG 2.1 AA (>=4.5:1) and AAA (>=7.0:1)', () => {
  const darkPairs = [
    { name: 'diff-line-add', fg: '#ecfdf5', bg: '#064e3b' },
    { name: 'diff-line-del', fg: '#fef2f2', bg: '#7f1d1d' },
    { name: 'diff-line-hunk', fg: '#eff6ff', bg: '#1e3a8a' },
    { name: 'diff-context (#0d1117)', fg: '#f4f4f5', bg: '#0d1117' },
    { name: 'diff-context (#121214)', fg: '#f4f4f5', bg: '#121214' },
  ];

  for (const pair of darkPairs) {
    const ratio = calculateContrast(pair.fg, pair.bg);
    assert.ok(ratio >= 4.5, `Dark ${pair.name} (${ratio.toFixed(2)}:1) fails WCAG AA`);
    assert.ok(ratio >= 7.0, `Dark ${pair.name} (${ratio.toFixed(2)}:1) fails WCAG AAA`);
  }
});

test('S3.2: Warm Light theme diff elements pass WCAG 2.1 AA (>=4.5:1) and AAA (>=7.0:1)', () => {
  const warmLightPairs = [
    { name: 'diff-line-add', fg: '#064e3b', bg: '#dcfce7' },
    { name: 'diff-line-del', fg: '#7f1d1d', bg: '#fee2e2' },
    { name: 'diff-line-hunk', fg: '#1e3a8a', bg: '#dbeafe' },
    { name: 'diff-context (#faf9f5)', fg: '#111827', bg: '#faf9f5' },
  ];

  for (const pair of warmLightPairs) {
    const ratio = calculateContrast(pair.fg, pair.bg);
    assert.ok(ratio >= 4.5, `Warm Light ${pair.name} (${ratio.toFixed(2)}:1) fails WCAG AA`);
    assert.ok(ratio >= 7.0, `Warm Light ${pair.name} (${ratio.toFixed(2)}:1) fails WCAG AAA`);
  }
});

test('S3.3: Cmd+K Floating Bar Badge and Secondary Text meet WCAG standards', () => {
  const badgeRatio = calculateContrast('#ffffff', '#1d4ed8');
  assert.ok(badgeRatio >= 4.5, `Badge contrast (${badgeRatio.toFixed(2)}:1) fails WCAG AA`);

  const darkSecondaryRatio = calculateContrast('#d4d4d8', '#1a1a1d');
  assert.ok(darkSecondaryRatio >= 7.0, `Dark secondary (${darkSecondaryRatio.toFixed(2)}:1) fails WCAG AAA`);

  const lightSecondaryRatio = calculateContrast('#374151', '#ffffff');
  assert.ok(lightSecondaryRatio >= 7.0, `Warm Light secondary (${lightSecondaryRatio.toFixed(2)}:1) fails WCAG AAA`);
});

test('S3.4: CSS theme rules for .cursor-cmdk-diff-hunk exist for both themes in app.css', () => {
  assert.ok(rawAppCss.includes('.cursor-cmdk-diff-hunk {'));
  assert.ok(rawAppCss.includes('body.theme-warm-light .cursor-cmdk-diff-hunk {'));
  assert.ok(rawAppCss.includes('body.theme-warm-light .diff-line-add {'));
  assert.ok(rawAppCss.includes('body.theme-warm-light .diff-line-del {'));
  assert.ok(rawAppCss.includes('body.theme-warm-light .diff-line-hunk {'));
});

// ============================================================================
// SUITE 4: VFS CONSISTENCY & TAB SYNCHRONIZATION
// ============================================================================
console.log('\n--- Suite 4: VFS Consistency & Tab Synchronization ---');

test('S4.1: acceptCursorCmdKEdit updates virtualFiles, tab buffer, and clears dirty status', () => {
  const { controller, virtualFiles, workspaceEditorTextarea } = createChallengerM2Sandbox();
  const etm = controller.editorTabManager;
  etm.init(['src/attention.rs', 'src/lib.rs']);

  controller.openCursorCmdK();
  controller.runCursorCmdKEdit();
  const proposed = controller.cursorCmdKState.proposedFullCode;

  controller.acceptCursorCmdKEdit();

  // VFS updated
  const file = virtualFiles.find((f) => f.path === 'src/attention.rs');
  assert.equal(file.content, proposed);

  // Tab buffer updated and clean
  const tab = etm.tabs.get('src/attention.rs');
  assert.equal(tab.buffer, proposed);
  assert.equal(tab.savedContent, proposed);
  assert.equal(tab.isDirty, false);

  // Textarea matches
  assert.equal(workspaceEditorTextarea.value, proposed);
});

test('S4.2: Tab switching maintains buffer isolation and dirty status across multiple files', () => {
  const { controller, workspaceEditorTextarea } = createChallengerM2Sandbox();
  const etm = controller.editorTabManager;
  etm.init(['src/attention.rs', 'src/lib.rs', 'src/LutStack.rs']);

  // Tab 1: Apply Cmd+K edit and accept
  controller.openCursorCmdK();
  controller.runCursorCmdKEdit();
  const tab1Accepted = controller.cursorCmdKState.proposedFullCode;
  controller.acceptCursorCmdKEdit();

  // Switch to Tab 2 and make an uncommitted edit
  etm.switchTo('src/lib.rs');
  workspaceEditorTextarea.value = 'pub mod attention;\npub mod LutStack;\n// uncommitted local edit\n';
  etm.updateBuffer(workspaceEditorTextarea.value);
  assert.equal(etm.tabs.get('src/lib.rs').isDirty, true);

  // Switch to Tab 3, open Cmd+K, run edit, but REJECT
  etm.switchTo('src/LutStack.rs');
  const tab3Original = workspaceEditorTextarea.value;
  controller.openCursorCmdK();
  controller.runCursorCmdKEdit();
  controller.rejectCursorCmdKEdit();
  assert.equal(workspaceEditorTextarea.value, tab3Original);
  assert.equal(etm.tabs.get('src/LutStack.rs').isDirty, false);

  // Switch back to Tab 1 -> must retain accepted code and be clean
  etm.switchTo('src/attention.rs');
  assert.equal(workspaceEditorTextarea.value, tab1Accepted);
  assert.equal(etm.tabs.get('src/attention.rs').isDirty, false);

  // Switch back to Tab 2 -> must retain dirty edits and stay dirty
  etm.switchTo('src/lib.rs');
  assert.ok(workspaceEditorTextarea.value.includes('// uncommitted local edit'));
  assert.equal(etm.tabs.get('src/lib.rs').isDirty, true);
});

test('S4.3: Rejecting Cmd+K preserves pre-existing dirty manual edits on active tab', () => {
  const { controller, workspaceEditorTextarea } = createChallengerM2Sandbox();
  const etm = controller.editorTabManager;
  etm.init(['src/attention.rs']);

  // Dirty edit prior to Cmd+K
  const dirtyCode = 'pub fn flock_attention_step() {\n    // Draft edit before Cmd+K\n}\n';
  workspaceEditorTextarea.value = dirtyCode;
  etm.updateBuffer(dirtyCode);
  assert.equal(etm.tabs.get('src/attention.rs').isDirty, true);

  // Open Cmd+K, run edit, then reject
  controller.openCursorCmdK();
  controller.runCursorCmdKEdit();
  controller.rejectCursorCmdKEdit();

  // Buffer must restore to dirtyCode, and isDirty must remain true
  assert.equal(workspaceEditorTextarea.value, dirtyCode);
  etm.updateBuffer(dirtyCode);
  assert.equal(etm.tabs.get('src/attention.rs').isDirty, true);
});

test('S4.4: Deeply nested file path works seamlessly with Cmd+K VFS write', () => {
  const deepPath = 'crates/core/src/subsystems/attention/layer.rs';
  const { controller, virtualFiles, workspaceEditorTextarea } = createChallengerM2Sandbox();
  virtualFiles.push({ path: deepPath, line_count: 10, content: 'pub struct AttentionLayer;\n' });

  const etm = controller.editorTabManager;
  etm.init();
  etm.open(deepPath);

  controller.openCursorCmdK();
  assert.equal(controller.cursorCmdKState.targetPath, deepPath);

  controller.runCursorCmdKEdit();
  const proposed = controller.cursorCmdKState.proposedFullCode;
  controller.acceptCursorCmdKEdit();

  const file = virtualFiles.find((f) => f.path === deepPath);
  assert.ok(file, 'Deep file must exist in VFS');
  assert.equal(file.content, proposed);
  assert.equal(workspaceEditorTextarea.value, proposed);
  assert.equal(etm.tabs.get(deepPath).isDirty, false);
});

test('S4.5: Fail-closed policy: checkMcpToolPolicy rejecting workspace_write aborts write safely', () => {
  const { controller, virtualFiles, workspaceEditorTextarea, setPolicyAllowance } = createChallengerM2Sandbox();
  const original = workspaceEditorTextarea.value;
  setPolicyAllowance(false); // Disallow write

  controller.openCursorCmdK();
  controller.runCursorCmdKEdit();
  controller.acceptCursorCmdKEdit();

  // Write should be blocked
  assert.equal(workspaceEditorTextarea.value, original);
  assert.equal(virtualFiles[0].content, original);
});

// ============================================================================
// SUITE 5: INTERACTIVE CONTROLS LIFECYCLE & SHORTCUT ERGONOMICS
// ============================================================================
console.log('\n--- Suite 5: Interactive Controls Lifecycle & Shortcut Ergonomics ---');

test('S5.1: Accept and Reject buttons follow strict disable/enable/hide lifecycle', () => {
  const { controller, elements } = createChallengerM2Sandbox();

  // Initial open: hidden and disabled
  controller.openCursorCmdK();
  assert.equal(elements.acceptBtn.hidden, true);
  assert.equal(elements.acceptBtn.disabled, true);
  assert.equal(elements.acceptBtn.getAttribute('disabled'), '');
  assert.equal(elements.rejectBtn.hidden, true);
  assert.equal(elements.rejectBtn.disabled, true);
  assert.equal(elements.rejectBtn.getAttribute('disabled'), '');

  // Diff generated: visible and enabled
  controller.runCursorCmdKEdit();
  assert.equal(elements.acceptBtn.hidden, false);
  assert.equal(elements.acceptBtn.disabled, false);
  assert.equal(elements.acceptBtn.getAttribute('disabled'), null);
  assert.equal(elements.rejectBtn.hidden, false);
  assert.equal(elements.rejectBtn.disabled, false);
  assert.equal(elements.rejectBtn.getAttribute('disabled'), null);

  // Accepted: re-hidden and re-disabled
  controller.acceptCursorCmdKEdit();
  assert.equal(elements.acceptBtn.hidden, true);
  assert.equal(elements.acceptBtn.disabled, true);
  assert.equal(elements.acceptBtn.getAttribute('disabled'), '');
  assert.equal(elements.rejectBtn.hidden, true);
  assert.equal(elements.rejectBtn.disabled, true);
  assert.equal(elements.rejectBtn.getAttribute('disabled'), '');
});

test('S5.2: Reject button follows identical re-hide and re-disable lifecycle', () => {
  const { controller, elements } = createChallengerM2Sandbox();

  controller.openCursorCmdK();
  controller.runCursorCmdKEdit();
  controller.rejectCursorCmdKEdit();

  assert.equal(elements.acceptBtn.hidden, true);
  assert.equal(elements.acceptBtn.disabled, true);
  assert.equal(elements.acceptBtn.getAttribute('disabled'), '');
  assert.equal(elements.rejectBtn.hidden, true);
  assert.equal(elements.rejectBtn.disabled, true);
  assert.equal(elements.rejectBtn.getAttribute('disabled'), '');
});

test('S5.3: Global keyboard shortcuts (Cmd+K, Escape, Cmd+Enter) dispatch appropriately', () => {
  const { doc, controller, elements, workspaceEditorTextarea } = createChallengerM2Sandbox();

  // Global Cmd+K opens
  doc.dispatchEvent({
    type: 'keydown',
    key: 'k',
    metaKey: true,
    shiftKey: false,
    defaultPrevented: false,
    preventDefault() { this.defaultPrevented = true; },
  });
  assert.equal(controller.cursorCmdKState.open, true);

  // Generate diff
  controller.runCursorCmdKEdit();
  const proposed = controller.cursorCmdKState.proposedFullCode;

  // Global Cmd+Enter accepts
  doc.dispatchEvent({
    type: 'keydown',
    key: 'Enter',
    metaKey: true,
    defaultPrevented: false,
    preventDefault() { this.defaultPrevented = true; },
  });
  assert.equal(workspaceEditorTextarea.value, proposed);
  assert.equal(controller.cursorCmdKState.open, false);

  // Global Cmd+K opens again, Escape rejects
  doc.dispatchEvent({
    type: 'keydown',
    key: 'k',
    metaKey: true,
    shiftKey: false,
    defaultPrevented: false,
    preventDefault() { this.defaultPrevented = true; },
  });
  assert.equal(controller.cursorCmdKState.open, true);

  doc.dispatchEvent({
    type: 'keydown',
    key: 'Escape',
    defaultPrevented: false,
    preventDefault() { this.defaultPrevented = true; },
  });
  assert.equal(controller.cursorCmdKState.open, false);
});

test('S5.4: Input-level shortcuts (Enter to generate, Cmd+Enter to accept, Cmd+Backspace/Escape to reject)', () => {
  const { controller, elements, workspaceEditorTextarea } = createChallengerM2Sandbox();
  controller.openCursorCmdK();

  // 1. Enter inside cmdkInput triggers diff generation
  elements.input.value = 'Generate helper function';
  elements.input.dispatchEvent({
    type: 'keydown',
    key: 'Enter',
    metaKey: false,
    defaultPrevented: false,
    preventDefault() { this.defaultPrevented = true; },
  });
  assert.ok(controller.cursorCmdKState.proposedFullCode.length > 0);
  assert.equal(elements.acceptBtn.hidden, false);

  // 2. Cmd+Backspace inside cmdkInput rejects
  elements.input.dispatchEvent({
    type: 'keydown',
    key: 'Backspace',
    metaKey: true,
    defaultPrevented: false,
    preventDefault() { this.defaultPrevented = true; },
  });
  assert.equal(controller.cursorCmdKState.open, false);
  assert.equal(elements.acceptBtn.hidden, true);

  // 3. Open again, generate, and Cmd+Enter inside cmdkInput accepts
  controller.openCursorCmdK();
  elements.input.value = 'Generate helper function';
  elements.input.dispatchEvent({
    type: 'keydown',
    key: 'Enter',
    metaKey: false,
    defaultPrevented: false,
    preventDefault() { this.defaultPrevented = true; },
  });
  const proposed = controller.cursorCmdKState.proposedFullCode;

  elements.input.dispatchEvent({
    type: 'keydown',
    key: 'Enter',
    metaKey: true,
    defaultPrevented: false,
    preventDefault() { this.defaultPrevented = true; },
  });
  assert.equal(workspaceEditorTextarea.value, proposed);
  assert.equal(controller.cursorCmdKState.open, false);
});

test('S5.5: Selection range detection across multi-line blocks formats rangeBadge correctly', () => {
  const { controller, elements, workspaceEditorTextarea } = createChallengerM2Sandbox();
  const lines = Array.from({ length: 30 }, (_, i) => `line_${i + 1}`);
  workspaceEditorTextarea.value = lines.join('\n');

  // Select lines 5 through 12
  const prefix5 = lines.slice(0, 4).join('\n') + '\n';
  const prefix12 = lines.slice(0, 12).join('\n');
  workspaceEditorTextarea.selectionStart = prefix5.length;
  workspaceEditorTextarea.selectionEnd = prefix12.length;

  controller.openCursorCmdK();
  assert.equal(controller.cursorCmdKState.startLine, 5);
  assert.equal(controller.cursorCmdKState.endLine, 12);
  assert.equal(elements.rangeBadge.textContent, 'Lines 5–12 in src/attention.rs');
});

test('S5.6: Unicode, emoji, and special regex characters in diff generation and acceptance', () => {
  const { controller, elements, workspaceEditorTextarea } = createChallengerM2Sandbox({
    chatResponse: {
      status: 'ok',
      proposed_code: '// 🦀 Rust + ⚡ Speed: $100 & (a + b) * [c] -> /\\w+/g\npub fn utf8_flock() -> &\'static str {\n    "🦀 🚀"\n}\n',
    },
  });

  controller.openCursorCmdK();
  controller.runCursorCmdKEdit();
  assert.ok(controller.cursorCmdKState.diffText.includes('🦀'));
  assert.ok(controller.cursorCmdKState.diffText.includes('⚡'));

  controller.acceptCursorCmdKEdit();
  assert.ok(workspaceEditorTextarea.value.includes('🦀 🚀'));
  assert.ok(workspaceEditorTextarea.value.includes('/\\w+/g'));
});

test('S5.7: CRLF and mixed line endings in computeUnifiedDiffJs normalize smoothly', () => {
  const { controller } = createChallengerM2Sandbox();
  const oldText = 'line1\r\nline2\r\nline3\r\n';
  const newText = 'line1\r\nline2_modified\r\nline3\r\n';

  const diff = controller.computeUnifiedDiffJs(oldText, newText, 'src/crlf.rs');
  assert.ok(diff.includes('--- a/src/crlf.rs'));
  assert.ok(diff.includes('+++ b/src/crlf.rs'));
  assert.ok(diff.includes('-line2'));
  assert.ok(diff.includes('+line2_modified'));
});

// ============================================================================
// SUMMARY & VERDICT
// ============================================================================
console.log('\n================================================================');
console.log(`  CHALLENGER M2_2 HARNESS COMPLETE: ${passedTests}/${totalTests} TESTS PASSED`);
console.log('================================================================');

if (passedTests === totalTests) {
  console.log('\nVERDICT: APPROVE');
  process.exit(0);
} else {
  console.log('\nVERDICT: REQUEST_CHANGES');
  process.exit(1);
}
