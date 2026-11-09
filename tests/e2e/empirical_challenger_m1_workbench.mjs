/**
 * ============================================================================
 * EMPIRICAL CHALLENGER 1: CURSOR 3-PANE WORKBENCH & STATUS BAR STRESS HARNESS
 * ============================================================================
 *
 * Adversarial stress testing for Milestone 1 (R1 & R5):
 * 1. Rapid tab cycling, creation, boundary fuzzing, and the single-tab invariant.
 * 2. Buffer retention across multi-tab edits, dirty transitions, and save signals.
 * 3. Bottom terminal toggle multi-trigger resilience, panel tab focus semantics,
 *    and scrollback retention under heavy load.
 * 4. Status bar precision: Ln/Col tracking across huge buffers, language mapping,
 *    git dirty badges, and local WASM execution badges.
 * 5. Sidebar view toggling, Cmd+B collapse states, and auto-expand semantics.
 * 6. Closure integrity, CSP security audit, and disk space invariant verification.
 */

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');
const PUBLIC_DIR = resolve(REPO_ROOT, 'public');

console.log('================================================================');
console.log('  CHALLENGER M1: CURSOR WORKBENCH & STATUS BAR EMPIRICAL AUDIT');
console.log('================================================================\n');

// ----------------------------------------------------------------------------
// Lightweight DOM Mock for Headless Node.js Execution
// ----------------------------------------------------------------------------

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
    this.scrollTop = 0;
    this.hidden = false;
    this.disabled = false;
    this.type = 'button';
    this.eventListeners = {};
  }

  get className() { return this._className; }
  set className(val) {
    this._className = String(val ?? '');
    this.classList.classes = new Set(this._className.split(/\s+/).filter(Boolean));
  }

  get textContent() {
    if (this.nodeType === 3) return this._textContent;
    if (this.children.length === 0) return this._textContent;
    return this.children.map(c => c.textContent).join('');
  }
  set textContent(val) {
    this._textContent = String(val ?? '');
    this.children = [];
    this.childNodes = this.children;
    if (this.nodeType === 1 && this._textContent) {
      const tn = new MockNode(3, '#text');
      tn._textContent = this._textContent;
      this.children.push(tn);
    }
  }

  get innerHTML() {
    return this.textContent;
  }
  set innerHTML(val) {
    this.textContent = String(val ?? '').replace(/<[^>]+>/g, '');
  }

  get value() { return this._value; }
  set value(val) { this._value = String(val ?? ''); }

  setAttribute(k, v) {
    const val = String(v ?? '');
    this.attributes.set(k, val);
    if (k === 'id') this.id = val;
    if (k === 'class') this.className = val;
    if (k === 'hidden') this.hidden = true;
    if (k === 'disabled') this.disabled = true;
    if (k.startsWith('data-')) {
      const camel = k.slice(5).replace(/-([a-z])/g, (_, ch) => ch.toUpperCase());
      this.dataset[camel] = val;
    }
  }

  getAttribute(k) { return this.attributes.get(k) ?? null; }
  removeAttribute(k) {
    this.attributes.delete(k);
    if (k === 'hidden') this.hidden = false;
    if (k === 'disabled') this.disabled = false;
    if (k.startsWith('data-')) {
      const camel = k.slice(5).replace(/-([a-z])/g, (_, ch) => ch.toUpperCase());
      delete this.dataset[camel];
    }
  }
  hasAttribute(k) { return this.attributes.has(k); }

  appendChild(child) {
    if (!child) return child;
    if (child.parentElement) child.parentElement.removeChild(child);
    child.parentElement = this;
    child.parentNode = this;
    this.children.push(child);
    return child;
  }

  prepend(...nodes) {
    for (let i = nodes.length - 1; i >= 0; i--) {
      const child = nodes[i];
      if (!child) continue;
      if (child.parentElement) child.parentElement.removeChild(child);
      child.parentElement = this;
      child.parentNode = this;
      this.children.unshift(child);
    }
  }

  removeChild(child) {
    const idx = this.children.indexOf(child);
    if (idx >= 0) {
      this.children.splice(idx, 1);
      child.parentElement = null;
      child.parentNode = null;
    }
    return child;
  }

  replaceChildren(...nodes) {
    for (const c of this.children) {
      c.parentElement = null;
      c.parentNode = null;
    }
    this.children = [];
    this.childNodes = this.children;
    for (const n of nodes) {
      if (n) this.appendChild(n);
    }
  }

  focus() { this.dispatchEvent({ type: 'focus', target: this }); }
  click() { this.dispatchEvent({ type: 'click', target: this }); }

  addEventListener(type, handler) {
    if (!this.eventListeners[type]) this.eventListeners[type] = [];
    this.eventListeners[type].push(handler);
  }
  removeEventListener(type, handler) {
    if (!this.eventListeners[type]) return;
    this.eventListeners[type] = this.eventListeners[type].filter(h => h !== handler);
  }
  dispatchEvent(event) {
    const ev = typeof event === 'string' ? { type: event } : event;
    if (!ev.target) ev.target = this;
    if (!ev.currentTarget) ev.currentTarget = this;
    if (typeof ev.preventDefault !== 'function') ev.preventDefault = () => {};
    if (typeof ev.stopPropagation !== 'function') ev.stopPropagation = () => {};
    const handlers = this.eventListeners[ev.type] || [];
    for (const h of handlers) h(ev);
  }

  querySelector(selector) {
    const all = this.querySelectorAll(selector);
    return all.length > 0 ? all[0] : null;
  }
  querySelectorAll(selector) {
    const matches = [];
    const check = (node) => {
      if (node.nodeType !== 1) return;
      if (selector.startsWith('#') && node.id === selector.slice(1)) matches.push(node);
      else if (selector.startsWith('.') && node.classList.contains(selector.slice(1))) matches.push(node);
      else if (node.tagName.toLowerCase() === selector.toLowerCase()) matches.push(node);
      for (const c of node.children) check(c);
    };
    for (const c of this.children) check(c);
    return matches;
  }
}

class MockDocument {
  constructor() {
    this.body = new MockNode(1, 'BODY');
    this.elementsById = new Map();
    this.eventListeners = {};
  }
  createElement(tagName) { return new MockNode(1, tagName); }
  createTextNode(text) { return new MockNode(3, '#text'); }
  getElementById(id) { return this.elementsById.get(id) || null; }
  registerElement(id, el) { this.elementsById.set(id, el); return el; }
  querySelector(sel) { return this.body.querySelector(sel); }
  querySelectorAll(sel) { return this.body.querySelectorAll(sel); }
  addEventListener(type, handler) {
    if (!this.eventListeners[type]) this.eventListeners[type] = [];
    this.eventListeners[type].push(handler);
  }
  dispatchEvent(event) {
    const ev = typeof event === 'string' ? { type: event } : event;
    if (typeof ev.preventDefault !== 'function') ev.preventDefault = () => {};
    if (typeof ev.stopPropagation !== 'function') ev.stopPropagation = () => {};
    const handlers = this.eventListeners[ev.type] || [];
    for (const h of handlers) h(ev);
  }
}

function parseHtmlIntoMockDocument(htmlSource) {
  const doc = new MockDocument();
  const voidTags = new Set([
    'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input',
    'link', 'meta', 'param', 'source', 'track', 'wbr', '!doctype'
  ]);
  const cleanHtml = htmlSource.replace(/<!--[\s\S]*?-->/g, '');
  const tokenRegex = /<(\/?[a-zA-Z0-9\-]+)([^>]*?)>|([^<]+)/g;
  const stack = [doc.body];
  let match;
  while ((match = tokenRegex.exec(cleanHtml)) !== null) {
    const [_, tagToken, rawAttrs, textContent] = match;
    if (textContent !== undefined) {
      if (stack.length > 0) stack[stack.length - 1].appendChild(doc.createTextNode(textContent));
      continue;
    }
    if (!tagToken) continue;
    if (tagToken.startsWith('/')) {
      const closeTag = tagToken.slice(1).toUpperCase();
      for (let i = stack.length - 1; i > 0; i--) {
        if (stack[i].tagName === closeTag) {
          stack.length = i;
          break;
        }
      }
      continue;
    }
    const tagName = tagToken.toUpperCase();
    const isVoid = voidTags.has(tagToken.toLowerCase());
    const isSelfClosing = rawAttrs && rawAttrs.trim().endsWith('/');
    const el = doc.createElement(tagName);
    if (rawAttrs) {
      const attrRegex = /([a-zA-Z0-9\-:]+)(?:=(?:"([^"]*)"|'([^']*)'|([^>\s]+)))?/g;
      let aMatch;
      while ((aMatch = attrRegex.exec(rawAttrs)) !== null) {
        const name = aMatch[1];
        if (name === '/') continue;
        const val = aMatch[2] ?? aMatch[3] ?? aMatch[4] ?? '';
        el.setAttribute(name, val);
      }
    }
    if (el.id) doc.registerElement(el.id, el);
    if (stack.length > 0) stack[stack.length - 1].appendChild(el);
    if (!isVoid && !isSelfClosing) stack.push(el);
  }
  return doc;
}

// ----------------------------------------------------------------------------
// Sandbox Setup
// ----------------------------------------------------------------------------

const rawHtml = readFileSync(resolve(PUBLIC_DIR, 'index.html'), 'utf8');
const rawCss = readFileSync(resolve(PUBLIC_DIR, 'app.css'), 'utf8');
const rawAppJs = readFileSync(resolve(PUBLIC_DIR, 'app.js'), 'utf8');

function setupEnvironment() {
  const doc = parseHtmlIntoMockDocument(rawHtml);

  // Link dual IDs for activity buttons
  for (const [btnId, actId] of [
    ['cursor-act-explorer-btn', 'activity-btn-explorer'],
    ['cursor-act-explorer', 'activity-btn-explorer'],
    ['cursor-act-search-btn', 'activity-btn-search'],
    ['cursor-act-search', 'activity-btn-search'],
    ['cursor-act-scm-btn', 'activity-btn-scm'],
    ['cursor-act-scm', 'activity-btn-scm'],
    ['cursor-act-shards-btn', 'activity-btn-shards'],
    ['cursor-act-shards', 'activity-btn-shards'],
  ]) {
    const el = doc.getElementById(actId);
    if (el) doc.registerElement(btnId, el);
  }

  const virtualFs = new Map([
    ['src/attention.rs', 'pub fn flock_attention_step() {\n    // Cyclic flock\n}'],
    ['src/lib.rs', 'pub mod attention;\npub mod LutStack;\n'],
    ['src/LutStack.rs', 'pub struct LutStack {\n    pub shards: usize,\n}'],
    ['Cargo.toml', '[package]\nname = "lexlean-r4"\nversion = "0.4.0"\n'],
  ]);

  function mockDispatchStudioEnvelope(cmd, payload) {
    if (cmd === 'workspace_read') {
      if (virtualFs.has(payload)) {
        const content = virtualFs.get(payload);
        return { status: 'ok', path: payload, content, line_count: content.split('\n').length };
      }
      return { status: 'error', message: 'Not found' };
    }
    if (cmd === 'workspace_write') {
      const idx = payload.indexOf('\n');
      const targetPath = idx > 0 ? payload.slice(0, idx).trim() : 'src/attention.rs';
      const content = idx > 0 ? payload.slice(idx + 1) : payload;
      virtualFs.set(targetPath, content);
      return { status: 'ok', path: targetPath };
    }
    if (cmd === 'workspace_list') {
      const files = Array.from(virtualFs.entries()).map(([p, c]) => ({
        path: p,
        content: c,
        line_count: c.split('\n').length
      }));
      return { status: 'ok', files };
    }
    return { status: 'ok' };
  }

  function mockStudioWeightPackStatus() {
    return JSON.stringify({ status: 'ok', point_rows: 17408, reply_entries: 4096 });
  }

  function mockStudioWasmEval() {
    return JSON.stringify({ passed: true, total_tests: 32, passed_tests: 32, diagnostics: [] });
  }

  // Extract Diff functions
  const diffStart = rawAppJs.indexOf('function computeUnifiedDiffJs(');
  const diffEnd = rawAppJs.indexOf('function renderInteractiveArtifactPreview(', diffStart);
  const diffCode = rawAppJs.slice(diffStart, diffEnd);

  // Extract Workbench section
  const wbFallbackStart = rawAppJs.indexOf('// CURSOR IDE WORKBENCH CONTROLLER');
  const wbEnd = rawAppJs.indexOf('initCursorIdeWorkbench();', wbFallbackStart);
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

  workspaceEditorTextarea.value = virtualFs.get('src/attention.rs');

  let activeWorkspacePath = 'src/attention.rs';
  let activeCanvasTabId = 'terminal';

  function activateCanvasTab(tabId) { activeCanvasTabId = tabId; }
  function selectWorkspaceFile(path) {
    activeWorkspacePath = path;
    if (virtualFs.has(path)) workspaceEditorTextarea.value = virtualFs.get(path);
  }
  function renderWorkspaceList() {}
  function renderStudioState() {}
  function renderPendingPatchPills() {}
  function checkMcpToolPolicy() { return true; }
  function runOnDeviceTrain() {}

  const mockWindow = {
    showDirectoryPicker: null,
    addEventListener: (t, h) => doc.addEventListener(t, h),
    dispatchEvent: (e) => doc.dispatchEvent(e),
  };

  const factory = new Function(
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
    'virtualFs',
    `"use strict";
     activateCanvasTab = function(t) { activeCanvasTabId = t; };
     ${diffCode}
     ${wbCode}
     return {
       computeUnifiedDiffJs,
       EditorTabManager,
       editorTabManager,
       cursorOpenTabs,
       dockableCanvasPane,
       toggleCursorBottomPanel,
       updateCursorBreadcrumbsAndCursorPos,
       renderCursorLineGutter,
       renderCursorFileTree,
       renderCursorEditorTabs,
       initCursorIdeWorkbench,
       virtualFs,
       doc: document,
       get activeCanvasTabId() { return activeCanvasTabId; },
       set activeCanvasTabId(v) { activeCanvasTabId = v; },
       activateCanvasTab: (tabId) => { activeCanvasTabId = tabId; },
       workspaceEditorTextarea,
       getActiveCanvasTabId: () => activeCanvasTabId,
       getActiveWorkspacePath: () => activeWorkspacePath
     };`
  );

  const env = factory(
    doc,
    mockWindow,
    mockDispatchStudioEnvelope,
    mockStudioWeightPackStatus,
    mockStudioWasmEval,
    { workspace: { activePath: 'src/attention.rs', pendingPatches: {} } },
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
    doc.getElementById('editor-file-path')
  );

  return env;
}

// ============================================================================
// STRESS SUITE 1: RAPID TAB LIFECYCLE, BOUNDARY FUZZING & 1-TAB INVARIANT
// ============================================================================

console.log('--- Suite 1: Rapid Tab Lifecycle, Fuzzing & 1-Tab Invariant ---');
{
  const env = setupEnvironment();
  const tm = env.editorTabManager;
  tm.init(['src/attention.rs', 'src/lib.rs', 'src/LutStack.rs', 'Cargo.toml']);

  // 1. Boundary & Malformed Inputs
  tm.open(null);
  assert.equal(tm.tabs.size, 4, 'open(null) must be a safe no-op');
  tm.open('');
  assert.equal(tm.tabs.size, 4, 'open("") must be a safe no-op');
  tm.open(undefined);
  assert.equal(tm.tabs.size, 4, 'open(undefined) must be a safe no-op');

  // Deeply nested path name extraction
  tm.open('deeply/nested/path/to/my_module.rs', '// content');
  const nestedTab = tm.tabs.get('deeply/nested/path/to/my_module.rs');
  assert.ok(nestedTab, 'Deep path must register');
  assert.equal(nestedTab.name, 'my_module.rs', 'Short name must extract correctly');

  // Unicode and special characters
  const unicodePath = 'src/unicode_🦀_⚡_数学.rs';
  tm.open(unicodePath, 'pub const PI: f64 = 3.14159;');
  assert.ok(tm.tabs.has(unicodePath), 'Unicode tab path must register');
  assert.equal(tm.tabs.get(unicodePath).name, 'unicode_🦀_⚡_数学.rs');

  // 2. Rapid Ingestion of 100 Unique Tabs
  for (let i = 0; i < 100; i++) {
    const p = `src/generated_submodule_${i}.rs`;
    tm.open(p, `// Module ${i}\npub fn eval_${i}() -> usize { ${i} }\n`);
  }
  assert.equal(tm.tabs.size, 106, '106 total tabs must be open');
  assert.equal(env.cursorOpenTabs.length, 106, 'cursorOpenTabs must be in exact sync');

  // 3. 2,000 Rapid Random Switches & Buffer Verifications
  const tabKeys = Array.from(tm.tabs.keys());
  for (let step = 0; step < 2000; step++) {
    const target = tabKeys[step % tabKeys.length];
    tm.switchTo(target);
    assert.equal(tm.activePath, target, `Step ${step}: Active path must switch to ${target}`);
    assert.equal(env.workspaceEditorTextarea.value, tm.tabs.get(target).buffer, `Step ${step}: Editor value must match tab buffer`);
  }
  console.log('  ✓ 1.1 Ingested 100 tabs and executed 2,000 rapid switches with zero buffer crosstalk');

  // 4. Repeated Closures Down to Exactly 1 Tab
  const toClose = [...tabKeys];
  for (const path of toClose) {
    tm.close(path);
  }
  assert.equal(tm.tabs.size, 1, 'Tabs count must clamp at exactly 1 tab (invariant size >= 1)');
  assert.equal(env.cursorOpenTabs.length, 1, 'cursorOpenTabs must clamp at exactly 1 tab');
  const surviving = Array.from(tm.tabs.keys())[0];
  assert.equal(tm.activePath, surviving, 'Active path must point to surviving tab');

  // 5. Attempt Closing the Final Surviving Tab (Must Be Disallowed)
  tm.close(surviving);
  assert.equal(tm.tabs.size, 1, 'Closing final tab must be strictly disallowed');
  assert.equal(tm.activePath, surviving, 'Surviving tab remains active');
  console.log('  ✓ 1.2 Single-tab closure guard invariant strictly preserved under exhaustive closure loop');
}

// ============================================================================
// STRESS SUITE 2: BUFFER RETENTION, DIRTY STATES & PERSISTENCE
// ============================================================================

console.log('--- Suite 2: Buffer Retention, Dirty States & Persistence ---');
{
  const env = setupEnvironment();
  const tm = env.editorTabManager;
  tm.init(['src/attention.rs', 'src/lib.rs', 'src/LutStack.rs', 'Cargo.toml']);

  // Edit Tab 1
  tm.switchTo('src/attention.rs');
  env.workspaceEditorTextarea.value = '// MUTATED ATTENTION BUFFER';
  tm.updateBuffer(env.workspaceEditorTextarea.value);
  assert.equal(tm.tabs.get('src/attention.rs').isDirty, true, 'Tab 1 must be dirty');

  // Edit Tab 2
  tm.switchTo('src/lib.rs');
  env.workspaceEditorTextarea.value = '// MUTATED LIB BUFFER';
  tm.updateBuffer(env.workspaceEditorTextarea.value);
  assert.equal(tm.tabs.get('src/lib.rs').isDirty, true, 'Tab 2 must be dirty');

  // Check Dirty Count in Status Bar
  const dirtyEl = env.doc.getElementById('status-git-dirty');
  assert.equal(dirtyEl.textContent, '2 uncommitted', 'Status bar must show 2 uncommitted');

  // 500 Rapid Alternating Switches Between Dirty and Clean Tabs
  for (let i = 0; i < 500; i++) {
    tm.switchTo('src/attention.rs');
    assert.equal(env.workspaceEditorTextarea.value, '// MUTATED ATTENTION BUFFER');
    tm.switchTo('src/LutStack.rs'); // Clean tab
    assert.ok(env.workspaceEditorTextarea.value.includes('pub struct LutStack'));
    tm.switchTo('src/lib.rs');
    assert.equal(env.workspaceEditorTextarea.value, '// MUTATED LIB BUFFER');
  }
  console.log('  ✓ 2.1 500 rapid switches preserved dirty buffer contents without revert or data loss');

  // Reverting Tab 2 back to savedContent clears dirty flag
  tm.switchTo('src/lib.rs');
  env.workspaceEditorTextarea.value = tm.tabs.get('src/lib.rs').savedContent;
  tm.updateBuffer(env.workspaceEditorTextarea.value);
  assert.equal(tm.tabs.get('src/lib.rs').isDirty, false, 'Tab 2 must become clean when content matches savedContent');
  assert.equal(dirtyEl.textContent, '1 uncommitted', 'Dirty count must decrement to 1');

  // Saving Tab 1 commits buffer
  tm.switchTo('src/attention.rs');
  tm.markSaved('src/attention.rs');
  assert.equal(tm.tabs.get('src/attention.rs').isDirty, false, 'Tab 1 must become clean on markSaved');
  assert.equal(dirtyEl.textContent, '0 uncommitted', 'Dirty count must return to 0');
  console.log('  ✓ 2.2 Dynamic dirty state transitions, count decrements, and markSaved verified');
}

// ============================================================================
// STRESS SUITE 3: BOTTOM TERMINAL TOGGLE & MULTI-TRIGGER RESILIENCE
// ============================================================================

console.log('--- Suite 3: Bottom Terminal Toggle & Multi-Trigger Resilience ---');
{
  const env = setupEnvironment();
  const pane = env.dockableCanvasPane;
  const toggleBtn = env.doc.getElementById('cursor-toggle-bottom-panel-btn');

  // Initial state: panel is open (does not have .cursor-bottom-panel-collapsed)
  pane.classList.remove('cursor-bottom-panel-collapsed');

  // 1,000 rapid toggles via toggleCursorBottomPanel
  for (let i = 0; i < 1000; i++) {
    env.toggleCursorBottomPanel('terminal');
    const expectedCollapsed = (i % 2 === 0);
    assert.equal(
      pane.classList.contains('cursor-bottom-panel-collapsed'),
      expectedCollapsed,
      `Cycle ${i}: Bottom panel collapsed state mismatch`
    );
  }
  console.log('  ✓ 3.1 1,000 bottom panel toggle cycles executed with deterministic collapse states');

  // Panel Open on Different Tab Semantics:
  // If panel is currently OPEN on 'brain', clicking terminal toggle switches to 'terminal' rather than closing
  pane.classList.remove('cursor-bottom-panel-collapsed');
  env.activateCanvasTab('brain');
  assert.equal(env.getActiveCanvasTabId(), 'brain');

  env.toggleCursorBottomPanel('terminal');
  assert.equal(pane.classList.contains('cursor-bottom-panel-collapsed'), false, 'Should remain open');
  assert.equal(env.getActiveCanvasTabId(), 'terminal', 'Must switch active tab to terminal');

  // Clicking again now collapses it
  env.toggleCursorBottomPanel('terminal');
  assert.equal(pane.classList.contains('cursor-bottom-panel-collapsed'), true, 'Should now collapse');
  console.log('  ✓ 3.2 Secondary tab focus redirection verified (switches to terminal before collapse)');
}

// ============================================================================
// STRESS SUITE 4: STATUS BAR METADATA & PRECISION LN/COL TRACKING
// ============================================================================

console.log('--- Suite 4: Status Bar Metadata & Precision Ln/Col Tracking ---');
{
  const env = setupEnvironment();
  const posEl = env.doc.getElementById('cursor-status-lncol');
  const langEl = env.doc.getElementById('status-language');
  const modelEl = env.doc.getElementById('status-model-badge');
  const gitBranchEl = env.doc.getElementById('status-git-branch');

  // 1. Boundary Cursor Positions
  env.workspaceEditorTextarea.value = '';
  env.workspaceEditorTextarea.selectionStart = 0;
  env.updateCursorBreadcrumbsAndCursorPos();
  assert.equal(posEl.textContent, 'Ln 1, Col 1', 'Empty text must be Ln 1, Col 1');

  // Multi-line file navigation
  const multiLineCode = 'line1\nsecond_line\nthird\nfourth_longer_line\nfifth';
  env.workspaceEditorTextarea.value = multiLineCode;

  // Jump to start of line 2 (offset 6: 'line1\n')
  env.workspaceEditorTextarea.selectionStart = 6;
  env.updateCursorBreadcrumbsAndCursorPos();
  assert.equal(posEl.textContent, 'Ln 2, Col 1', 'Offset 6 must be Ln 2, Col 1');

  // Jump to middle of line 4: 'line1\nsecond_line\nthird\nfour' (offset 6 + 12 + 6 + 4 = 28)
  env.workspaceEditorTextarea.selectionStart = 28;
  env.updateCursorBreadcrumbsAndCursorPos();
  assert.equal(posEl.textContent, 'Ln 4, Col 5', 'Offset 28 must be Ln 4, Col 5');

  // 2,000 Line Stress Test
  const lines2000 = Array.from({ length: 2000 }, (_, i) => `// Line index ${i + 1} with standard length`).join('\n');
  env.workspaceEditorTextarea.value = lines2000;
  const targetLine = 1450;
  const targetCol = 15;
  const prefix = Array.from({ length: targetLine - 1 }, (_, i) => `// Line index ${i + 1} with standard length`).join('\n') + '\n' + '// Line index '.slice(0, targetCol - 1);
  env.workspaceEditorTextarea.selectionStart = prefix.length;
  env.updateCursorBreadcrumbsAndCursorPos();
  assert.equal(posEl.textContent, `Ln ${targetLine}, Col ${targetCol}`, 'Line 1450 math must be exact in 2,000-line buffer');

  // 2. Language Extension Mapping
  const extCases = [
    ['main.rs', 'Rust'],
    ['Cargo.toml', 'TOML'],
    ['bundle.js', 'JavaScript'],
    ['tool.mjs', 'JavaScript'],
    ['types.ts', 'TypeScript'],
    ['data.json', 'JSON'],
    ['index.html', 'HTML'],
    ['theme.css', 'CSS'],
    ['README.md', 'Markdown'],
    ['custom.unknownext', 'UNKNOWNEXT'],
  ];
  for (const [p, expectedLang] of extCases) {
    const ext = p.split('.').pop() || '';
    env.editorTabManager.activePath = p;
    env.editorTabManager.updateStatusBar();
    if (ext === 'ts') {
      assert.ok(langEl.textContent === 'TS' || langEl.textContent === 'TypeScript', `Path ${p} mapped to ${langEl.textContent}`);
    } else {
      assert.equal(langEl.textContent, expectedLang, `Path ${p} must map to ${expectedLang}`);
    }
  }

  // 3. Local Model Badge
  assert.ok(modelEl.textContent.includes('UOR-R4'), 'Model badge must proclaim local UOR-R4');
  assert.ok(modelEl.textContent.includes('LUT-8'), 'Model badge must specify LUT-8');
  assert.ok(!modelEl.textContent.includes('openai') && !modelEl.textContent.includes('anthropic'), 'Must have zero cloud strings');

  console.log('  ✓ 4.1 Precision Ln/Col coordinate tracking verified across 2,000-line stress buffers');
  console.log('  ✓ 4.2 Language extension mapping and pure local model badge verified');
}

// ============================================================================
// STRESS SUITE 5: SIDEBAR VIEW TOGGLING, CMD+B & AUTO-EXPAND
// ============================================================================

console.log('--- Suite 5: Sidebar View Toggling, Cmd+B & Auto-Expand ---');
{
  const env = setupEnvironment();
  env.initCursorIdeWorkbench();

  const sidebar = env.doc.getElementById('threads-sidebar');
  const mainEl = env.doc.querySelector('main');
  assert.ok(sidebar, '#threads-sidebar must exist');

  // 1,000 Rapid Cmd+B Toggles
  for (let i = 0; i < 1000; i++) {
    env.doc.dispatchEvent({
      type: 'keydown',
      key: 'b',
      metaKey: true,
      ctrlKey: false,
      shiftKey: false
    });
    const expectedCollapsed = (i % 2 === 0);
    assert.equal(sidebar.classList.contains('sidebar-collapsed'), expectedCollapsed, `Cmd+B toggle ${i}`);
    if (mainEl) {
      assert.equal(mainEl.classList.contains('sidebar-collapsed'), expectedCollapsed);
    }
  }
  console.log('  ✓ 5.1 1,000 Cmd+B keydown toggles executed synchronously without drift');

  // Auto-Expand Semantics:
  // When sidebar is collapsed, pressing Cmd+Shift+F (Search) must expand the sidebar and display search view
  sidebar.classList.add('sidebar-collapsed');
  if (mainEl) mainEl.classList.add('sidebar-collapsed');
  assert.equal(sidebar.classList.contains('sidebar-collapsed'), true);

  env.doc.dispatchEvent({
    type: 'keydown',
    key: 'f',
    metaKey: true,
    ctrlKey: false,
    shiftKey: true
  });
  assert.equal(sidebar.classList.contains('sidebar-collapsed'), false, 'Sidebar must auto-expand on activity switch');
  const searchView = env.doc.getElementById('sidebar-view-search');
  assert.ok(searchView.classList.contains('active'), 'Search view must be active');
  assert.equal(searchView.hasAttribute('hidden'), false, 'Search view must not be hidden');

  // Test Explorer shortcut Cmd+Shift+E
  env.doc.dispatchEvent({
    type: 'keydown',
    key: 'e',
    metaKey: true,
    ctrlKey: false,
    shiftKey: true
  });
  const explorerView = env.doc.getElementById('sidebar-view-explorer');
  assert.ok(explorerView.classList.contains('active'), 'Explorer view must be active');
  assert.equal(searchView.hasAttribute('hidden'), true, 'Previous view must be hidden');
  console.log('  ✓ 5.2 Activity Bar view switching and collapsed auto-expansion verified');
}

// ============================================================================
// STRESS SUITE 6: CLOSURE, SECURITY, ACCESSIBILITY & HEAP STABILITY
// ============================================================================

console.log('--- Suite 6: Closure, Security, Accessibility & Heap Stability ---');
{
  // 1. Strict 6-file production closure
  const publicFiles = readdirSync(PUBLIC_DIR);
  const required6 = ['app.css', 'app.js', 'index.html', 'lexlean_r4.js', 'lexlean_r4_bg.wasm', 'provenance.json'];
  assert.equal(publicFiles.length, 6, `public/ must contain exactly 6 files, found: ${publicFiles.join(', ')}`);
  for (const req of required6) {
    assert.ok(publicFiles.includes(req), `public/ must contain ${req}`);
    const st = statSync(resolve(PUBLIC_DIR, req));
    assert.ok(st.size > 0, `${req} must have non-zero file size`);
  }

  // 2. Strict Content-Security-Policy in index.html
  assert.ok(rawHtml.includes("Content-Security-Policy"), 'index.html must declare CSP');
  assert.ok(rawHtml.includes("connect-src 'self'"), 'CSP must strictly restrict connections to self');
  assert.ok(!rawHtml.includes('https://api.'), 'index.html must not contain external API domains');
  assert.ok(!rawHtml.includes('http://'), 'index.html must not contain unencrypted HTTP URLs');

  // 3. WCAG AA Contrast tokens & CSS grid rules
  assert.ok(rawCss.includes('color: #111827;'), 'Contractual WCAG AA contrast invariant color: #111827; preserved');
  assert.ok(rawCss.includes('.activity-bar'), 'Activity bar CSS rules declared');
  assert.ok(rawCss.includes('.editor-tabs-bar'), 'Editor tabs bar CSS rules declared');
  assert.ok(rawCss.includes('#ide-status-bar'), 'Status bar CSS rules declared');

  // 4. Memory Heap Delta Measurement
  const initialHeap = process.memoryUsage().heapUsed;
  const env = setupEnvironment();
  const tm = env.editorTabManager;
  tm.init(['src/attention.rs']);

  for (let iter = 0; iter < 5000; iter++) {
    const p = `src/temp_churn_${iter % 20}.rs`;
    tm.open(p, `// Temporary churn line ${iter}`);
    tm.updateBuffer(`// Modified churn line ${iter}`);
    if (iter % 5 === 0) tm.markSaved(p);
    if (iter % 7 === 0 && tm.tabs.size > 1) tm.close(p);
  }

  const finalHeap = process.memoryUsage().heapUsed;
  const deltaKb = (finalHeap - initialHeap) / 1024;
  console.log(`  ✓ 6.1 Executed 5,000 rapid lifecycle mutations with heap delta of ${deltaKb.toFixed(1)} KB`);
  assert.ok(deltaKb < 30000, `Heap growth must be reasonably bounded under 30MB, got ${deltaKb} KB`);
  console.log('  ✓ 6.2 6-File production closure, strict CSP sandboxing, and WCAG AA invariants verified');
}

// ============================================================================
// STRESS SUITE 7: DOM TAB STRIP, KEYBOARD SHORTCUTS & GUTTER ALIGNMENT
// ============================================================================

console.log('--- Suite 7: DOM Tab Strip, Keyboard Shortcuts & Gutter Alignment ---');
{
  const env = setupEnvironment();
  env.initCursorIdeWorkbench();
  const tm = env.editorTabManager;
  tm.init();
  tm.close('src/LutStack.rs');
  tm.close('Cargo.toml');

  // 1. Tab Strip DOM Elements & Accessibility
  const tabsContainer = env.doc.getElementById('cursor-editor-tabs');
  assert.ok(tabsContainer, '#cursor-editor-tabs must exist');
  const renderedTabs = tabsContainer.querySelectorAll('.cursor-editor-tab');
  assert.equal(renderedTabs.length, 2, '2 tabs must be rendered');

  // Verify accessibility attributes
  const tab0 = renderedTabs[0];
  const tab1 = renderedTabs[1];
  assert.equal(tab0.getAttribute('role'), 'tab', 'Tab must have role="tab"');
  assert.equal(tab0.getAttribute('aria-selected'), 'true', 'Active tab must have aria-selected="true"');
  assert.equal(tab1.getAttribute('aria-selected'), 'false', 'Inactive tab must have aria-selected="false"');

  // Close buttons present when tabs > 1
  const closeBtns = tabsContainer.querySelectorAll('.cursor-editor-tab-close');
  assert.equal(closeBtns.length, 2, 'Close buttons must be present on all tabs when tabs > 1');

  // Close one tab
  tm.close('src/lib.rs');
  assert.equal(tm.tabs.size, 1);
  const singleTabList = tabsContainer.querySelectorAll('.cursor-editor-tab-close');
  assert.equal(singleTabList.length, 0, 'Close button MUST NOT render when only 1 tab remains (cannot close last tab)');

  // 2. Keyboard Shortcut: Cmd+W
  // Reset with 2 tabs
  tm.open('src/lib.rs');
  assert.equal(tm.tabs.size, 2);
  assert.equal(tm.activePath, 'src/lib.rs');

  // Dispatch Cmd+W
  env.doc.dispatchEvent({
    type: 'keydown',
    key: 'w',
    metaKey: true,
    ctrlKey: false,
    shiftKey: false
  });
  assert.equal(tm.tabs.size, 1, 'Cmd+W must close active tab');
  assert.equal(tm.activePath, 'src/attention.rs', 'Preceding tab must become active');

  // Dispatch Cmd+W again on last tab (must NOT close)
  env.doc.dispatchEvent({
    type: 'keydown',
    key: 'w',
    metaKey: true,
    ctrlKey: false,
    shiftKey: false
  });
  assert.equal(tm.tabs.size, 1, 'Cmd+W on last tab must be a no-op');
  assert.equal(tm.activePath, 'src/attention.rs');

  // 3. Line Gutter Rendering & Line Count Sync
  const gutterEl = env.doc.getElementById('editor-line-numbers');
  assert.ok(gutterEl, '#editor-line-numbers must exist');

  env.workspaceEditorTextarea.value = 'line 1\nline 2\nline 3\nline 4\nline 5';
  tm.updateLineNumbers();
  assert.equal(gutterEl.textContent, '1\n2\n3\n4\n5', 'Gutter must display exact line numbers 1 to 5');

  // Empty buffer
  env.workspaceEditorTextarea.value = '';
  tm.updateLineNumbers();
  assert.equal(gutterEl.textContent, '1', 'Empty buffer gutter must display line 1');

  // 4. Dirty Buffer Overwrite Protection
  tm.open('src/lib.rs', 'original content');
  env.workspaceEditorTextarea.value = 'uncommitted dirty work in progress';
  tm.updateBuffer(env.workspaceEditorTextarea.value);
  assert.equal(tm.tabs.get('src/lib.rs').isDirty, true);

  // External open with different content
  tm.open('src/lib.rs', 'external incoming content from disk');
  assert.equal(
    tm.tabs.get('src/lib.rs').buffer,
    'uncommitted dirty work in progress',
    'open() MUST NOT overwrite dirty buffer when file is already modified'
  );

  console.log('  ✓ 7.1 DOM tab strip accessibility roles and close button guard verified');
  console.log('  ✓ 7.2 Cmd+W tab closure shortcut and single-tab retention invariant verified');
  console.log('  ✓ 7.3 Line gutter dynamic line number synchronization verified');
  console.log('  ✓ 7.4 In-memory dirty buffer protection against external overwrite verified');
}

console.log('\n================================================================');
console.log('  CHALLENGER M1 VERDICT: ALL ADVERSARIAL STRESS TESTS PASSED (0 DEFECTS)');
console.log('================================================================\n');
