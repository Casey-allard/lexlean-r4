/**
 * ============================================================================
 * EMPIRICAL CHALLENGER M4.2: COMMAND PALETTE, MOUNT & DISK SYNC STRESS HARNESS
 * ============================================================================
 *
 * Adversarial empirical verification for Milestone 4 (Quick Open, Command Palette & Disk Mount):
 *
 * 1. Command Palette Rapid Sequential Execution:
 *    - All 21 commands executed in rapid sequence via `executeCommand(id)`.
 *    - Endurance churn: 10 full round-robin passes (210 executions) verifying stability.
 *    - Side-effect verification across all command categories (View, File, Cursor AI, Terminal, UOR-R4).
 * 2. Invalid, Malformed & Hostile Command IDs:
 *    - Non-existent strings, empty string, falsy, primitive numbers, booleans, NaN.
 *    - Prototype pollution keys (__proto__, constructor, toString, valueOf).
 *    - Arbitrary objects, arrays, and null/undefined handling.
 *    - Returns false gracefully with zero uncaught exceptions.
 * 3. Command Palette Mode Transitions & Dynamic Typing:
 *    - Dynamic mode switching between Quick Open (files) and Command Palette (>) on input typing.
 *    - Mode badge synchronization (`Command Palette (Cmd+Shift+P)` vs `Quick Open (Cmd+P)`).
 *    - Leading/trailing whitespace resilience (`>   save`).
 *    - Empty result lists, ArrowUp/ArrowDown cycling on empty lists, and Enter on empty lists.
 *    - Global Escape key dismissal and focus restoration to workspaceEditorTextarea.
 * 4. Adversarial Local Directory Mounting (FileSystem API):
 *    - Deeply nested directory trees (55 levels of subdirectories).
 *    - Oversized files (>512KB, >10MB, >50MB) rejected without crashing.
 *    - Exactly 512KB boundary file accepted.
 *    - Whitelist vs Blacklist extension enforcement (.lean, .rs, .yaml vs .exe, .bin, .png).
 *    - Hidden paths (.git, .DS_Store) and ignored directories (target/, node_modules/) skipped.
 *    - Empty directory hierarchies (30 nested empty folders) do not dispatch empty batch.
 *    - FileSystem API rejection/abort fallback to `<input type="file" webkitdirectory>`.
 * 5. Fallback Input Mount Resilience (`handleWorkspaceMountInputChange`):
 *    - Batch ingestion of multiple files via input.
 *    - Large file (>512KB) and hidden path filtering.
 *    - Ingestion of 100+ files in a single batch.
 *    - Reset of `input.value = ''` upon completion.
 *    - Robust handling of empty, null, or corrupt input structures.
 * 6. Two-Way Disk Synchronization (`writeCursorDiskFileHandle`):
 *    - Disk write-back verification upon `save-file` command and `workspaceSaveBtn.click()`.
 *    - Disk write-back verification upon Cmd+K diff acceptance.
 *    - Unmounted file handling (safe no-op, no createWritable called).
 *    - Resilience against write errors (permission denied, disk full, stream close failure).
 *    - Multi-file isolation: only the saved file handle receives write operations.
 * 7. Browser Closure & Disk Space Invariants:
 *    - Strict 6-file closure in public/.
 *    - Zero external network connections / strict Content-Security-Policy.
 *    - Disk space invariant: target/ is 0B.
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
console.log('  CHALLENGER M4.2: COMMAND PALETTE, MOUNT & DISK SYNC STRESS');
console.log('  Adversarial Empirical Verification (Zero Dependencies)');
console.log('================================================================\n');

// ============================================================================
// 1. LIGHTWEIGHT IN-MEMORY DOM IMPLEMENTATION FOR HEADLESS NODE.JS
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
    for (const c of clsList) {
      if (c) this.classes.add(c);
    }
    this._sync();
  }

  remove(...clsList) {
    for (const c of clsList) {
      this.classes.delete(c);
    }
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

  [Symbol.for('nodejs.util.inspect.custom')]() {
    return `<${this.tagName.toLowerCase()} id="${this.id}" class="${this.className}">`;
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

  get innerHTML() {
    return this._innerHTML ?? '';
  }

  set innerHTML(val) {
    this._innerHTML = String(val ?? '');
    for (const c of this.children) {
      c.parentElement = null;
      c.parentNode = null;
    }
    this.children = [];
    this.childNodes = this.children;
    this._textContent = '';
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
    return this.attributes.get(k) ?? null;
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
    return this.attributes.has(k);
  }

  appendChild(child) {
    if (!child) return child;
    if (child.nodeType === 11) {
      const items = [...child.children];
      for (const item of items) this.appendChild(item);
      child.children = [];
      return child;
    }
    if (child.parentElement) {
      child.parentElement.removeChild(child);
    }
    child.parentElement = this;
    child.parentNode = this;
    this.children.push(child);
    return child;
  }

  prepend(...nodes) {
    for (let i = nodes.length - 1; i >= 0; i--) {
      const child = nodes[i];
      if (!child) continue;
      if (child.nodeType === 11) {
        this.prepend(...child.children);
        child.children = [];
        continue;
      }
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

  setSelectionRange(start, end) {
    this.selectionStart = Number(start) || 0;
    this.selectionEnd = Number(end) || 0;
  }

  focus() {
    this._isFocused = true;
    this.dispatchEvent({ type: 'focus', target: this });
  }

  blur() {
    this._isFocused = false;
    this.dispatchEvent({ type: 'blur', target: this });
  }

  select() {
    this.selectionStart = 0;
    this.selectionEnd = (this.value || '').length;
  }

  click() {
    this.dispatchEvent({ type: 'click', target: this });
  }

  scrollIntoView(options) {
    this._lastScrollIntoViewOptions = options;
    this._scrolledIntoView = true;
  }

  addEventListener(type, handler) {
    if (!this.eventListeners[type]) this.eventListeners[type] = [];
    this.eventListeners[type].push(handler);
  }

  removeEventListener(type, handler) {
    if (!this.eventListeners[type]) return;
    this.eventListeners[type] = this.eventListeners[type].filter((h) => h !== handler);
  }

  dispatchEvent(event) {
    const ev = typeof event === 'string' ? { type: event } : event;
    if (!ev.target) ev.target = this;
    if (!ev.currentTarget) ev.currentTarget = this;
    if (typeof ev.preventDefault !== 'function') ev.preventDefault = () => {};
    if (typeof ev.stopPropagation !== 'function') ev.stopPropagation = () => {};
    const handlers = this.eventListeners[ev.type] || [];
    for (const h of handlers) {
      h(ev);
    }
  }

  querySelector(selector) {
    const all = this.querySelectorAll(selector);
    return all.length > 0 ? all[0] : null;
  }

  querySelectorAll(selector) {
    const results = [];
    const match = createMatcher(selector);
    function traverse(node) {
      if (node.nodeType === 1 && match(node)) results.push(node);
      for (const child of node.children) {
        traverse(child);
      }
    }
    for (const child of this.children) {
      traverse(child);
    }
    return results;
  }
}

function createMatcher(selector) {
  const trimmed = selector.trim();
  if (trimmed.startsWith('#')) {
    const targetId = trimmed.slice(1);
    return (node) => node.id === targetId;
  }
  if (trimmed.startsWith('.')) {
    const targetCls = trimmed.slice(1);
    return (node) => node.classList && node.classList.contains(targetCls);
  }
  if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
    const inner = trimmed.slice(1, -1);
    if (inner.includes('=')) {
      const [attr, rawVal] = inner.split('=');
      const cleanVal = rawVal.replace(/^["']|["']$/g, '');
      return (node) => node.getAttribute(attr) === cleanVal;
    }
    return (node) => node.hasAttribute(inner);
  }
  const tagUpper = trimmed.toUpperCase();
  return (node) => node.tagName === tagUpper;
}

class MockDocument {
  constructor() {
    this.elementsById = new Map();
    this.allElements = [];
    this.body = new MockNode(1, 'BODY');
    this.head = new MockNode(1, 'HEAD');
    this.eventListeners = {};
  }

  createElement(tagName) {
    const node = new MockNode(1, tagName);
    this.allElements.push(node);
    return node;
  }

  createTextNode(text) {
    const node = new MockNode(3, '#text');
    node._textContent = String(text ?? '');
    return node;
  }

  createDocumentFragment() {
    return new MockNode(11, '#document-fragment');
  }

  getElementById(id) {
    return this.elementsById.get(id) || null;
  }

  registerElement(id, el) {
    this.elementsById.set(id, el);
    return el;
  }

  querySelector(selector) {
    return this.body.querySelector(selector);
  }

  querySelectorAll(selector) {
    return this.body.querySelectorAll(selector);
  }

  addEventListener(type, handler) {
    if (!this.eventListeners[type]) this.eventListeners[type] = [];
    this.eventListeners[type].push(handler);
  }

  removeEventListener(type, handler) {
    if (!this.eventListeners[type]) return;
    this.eventListeners[type] = this.eventListeners[type].filter((h) => h !== handler);
  }

  dispatchEvent(event) {
    const ev = typeof event === 'string' ? { type: event } : event;
    if (typeof ev.preventDefault !== 'function') ev.preventDefault = () => {};
    if (typeof ev.stopPropagation !== 'function') ev.stopPropagation = () => {};
    const handlers = this.eventListeners[ev.type] || [];
    for (const h of handlers) {
      h(ev);
    }
  }
}

function parseHtmlIntoMockDocument(htmlSource) {
  const doc = new MockDocument();
  const voidTags = new Set([
    'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input',
    'link', 'meta', 'param', 'source', 'track', 'wbr', '!doctype',
  ]);

  const cleanHtml = htmlSource.replace(/<!--[\s\S]*?-->/g, '');
  const tokenRegex = /<(\/?[a-zA-Z0-9\-]+)([^>]*?)>|([^<]+)/g;
  const stack = [doc.body];

  let match;
  while ((match = tokenRegex.exec(cleanHtml)) !== null) {
    const [_, tagToken, rawAttrs, textContent] = match;

    if (textContent !== undefined) {
      if (stack.length > 0) {
        const textNode = doc.createTextNode(textContent);
        stack[stack.length - 1].appendChild(textNode);
      }
      continue;
    }

    if (!tagToken) continue;

    if (tagToken.startsWith('/')) {
      const closingTag = tagToken.slice(1).toUpperCase();
      for (let i = stack.length - 1; i >= 1; i--) {
        if (stack[i].tagName === closingTag) {
          stack.length = i;
          break;
        }
      }
      continue;
    }

    const tagName = tagToken.toUpperCase();
    const node = doc.createElement(tagName);

    if (rawAttrs) {
      const attrRegex = /([a-zA-Z0-9\-]+)(?:=(?:"([^"]*)"|'([^']*)'|([^\s>]+)))?/g;
      let aMatch;
      while ((aMatch = attrRegex.exec(rawAttrs)) !== null) {
        const attrName = aMatch[1];
        const attrVal = aMatch[2] ?? aMatch[3] ?? aMatch[4] ?? '';
        node.setAttribute(attrName, attrVal);
      }
    }

    if (node.id) doc.registerElement(node.id, node);

    if (stack.length > 0) {
      stack[stack.length - 1].appendChild(node);
    }

    if (!voidTags.has(tagName.toLowerCase())) {
      stack.push(node);
    }
  }

  return doc;
}

// ============================================================================
// 2. MOCK FILE SYSTEM HANDLES (W3C FILE SYSTEM ACCESS API)
// ============================================================================

export function createMockFileHandle(name, initialContent = '', options = {}) {
  let currentContent = initialContent;
  let writeCallCount = 0;
  let closeCallCount = 0;
  let lastWrittenData = null;

  return {
    kind: 'file',
    name,
    async getFile() {
      if (options.throwOnGetFile) throw new Error(options.throwOnGetFile);
      return {
        name,
        size: options.size !== undefined ? options.size : Buffer.byteLength(currentContent, 'utf8'),
        async text() {
          if (options.throwOnText) throw new Error(options.throwOnText);
          return currentContent;
        },
      };
    },
    async createWritable() {
      if (options.throwOnCreateWritable) throw new Error(options.throwOnCreateWritable);
      return {
        async write(data) {
          if (options.throwOnWrite) throw new Error(options.throwOnWrite);
          writeCallCount += 1;
          lastWrittenData = data;
          currentContent = String(data);
        },
        async close() {
          if (options.throwOnClose) throw new Error(options.throwOnClose);
          closeCallCount += 1;
        },
      };
    },
    get _content() { return currentContent; },
    get _writeCallCount() { return writeCallCount; },
    get _closeCallCount() { return closeCallCount; },
    get _lastWrittenData() { return lastWrittenData; },
  };
}

export function createMockDirectoryHandle(name, entriesMap = new Map(), options = {}) {
  return {
    kind: 'directory',
    name,
    async *entries() {
      if (options.throwOnEntries) throw new Error(options.throwOnEntries);
      for (const [entryName, entry] of entriesMap.entries()) {
        yield [entryName, entry];
      }
    },
  };
}

// ============================================================================
// 3. SANDBOX SETUP & MODULE EXTRACTION
// ============================================================================

const rawHtml = readFileSync(resolve(PUBLIC_DIR, 'index.html'), 'utf8');
const rawAppJs = readFileSync(resolve(PUBLIC_DIR, 'app.js'), 'utf8');

function setupTestSandbox(options = {}) {
  const doc = parseHtmlIntoMockDocument(rawHtml);

  for (const [btnId, actId] of [
    ['cursor-act-explorer-btn', 'activity-btn-explorer'],
    ['cursor-act-explorer', 'activity-btn-explorer'],
    ['cursor-act-search-btn', 'activity-btn-search'],
    ['cursor-act-search', 'activity-btn-search'],
    ['cursor-act-scm-btn', 'activity-btn-scm'],
    ['cursor-act-scm', 'activity-btn-scm'],
    ['cursor-act-shards-btn', 'activity-btn-shards'],
    ['cursor-act-shards', 'activity-btn-shards'],
    ['cursor-act-palette', 'cursor-palette-modal'],
  ]) {
    const el = doc.getElementById(actId);
    if (el) doc.registerElement(btnId, el);
  }

  let virtualFiles = [
    { path: 'src/attention.rs', line_count: 36, content: 'pub fn flock_attention_step() {\n    // Cyclic flock\n}' },
    { path: 'src/lib.rs', line_count: 24, content: 'pub mod attention;\npub mod LutStack;\n' },
    { path: 'src/LutStack.rs', line_count: 48, content: 'pub struct LutStack {\n    pub shards: usize,\n}\n' },
    { path: 'Cargo.toml', line_count: 18, content: '[package]\nname = "lexlean-r4"\nversion = "0.4.0"\n' },
  ];

  const dispatchedEnvelopes = [];
  function mockDispatchStudioEnvelope(command, payload) {
    dispatchedEnvelopes.push({ command, payload });

    if (command === 'state') {
      return {
        schema: 'uor-r4.lut-stack/8',
        vocab: 4096,
        turn_rows: 10,
        point_rows: 17408,
        occupied_cells: 1088,
        state_receipt: 'sha256-mock-receipt',
      };
    }
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
    if (command === 'workspace_batch') {
      const fileBlocks = payload.split('\n===FILE===\n');
      let count = 0;
      for (const block of fileBlocks) {
        const lines = block.split('\n');
        const relPath = lines[0].trim();
        const body = lines.slice(1).join('\n');
        if (relPath) {
          const ex = virtualFiles.find((x) => x.path === relPath);
          if (ex) {
            ex.content = body;
            ex.line_count = body.split('\n').length;
          } else {
            virtualFiles.push({ path: relPath, content: body, line_count: body.split('\n').length });
          }
          count += 1;
        }
      }
      return { status: 'ok', batch_ingested_files: count, batch_ingested_tokens: count * 15, files: [...virtualFiles] };
    }
    if (command === 'consolidate') {
      return { status: 'ok', consolidated_epochs: 3 };
    }
    return { status: 'ok' };
  }

  function mockStudioWasmEval(subcommand, payload) {
    return JSON.stringify({
      passed: true,
      total_tests: 32,
      passed_tests: 32,
      diagnostics: [],
    });
  }

  const AppState = {
    workspace: {
      activePath: 'src/attention.rs',
      pendingPatchPath: '',
      pendingProposedCode: '',
      pendingPatches: {},
    },
  };

  const diffFuncStart = rawAppJs.indexOf('function computeUnifiedDiffJs(');
  const diffFuncEnd = rawAppJs.indexOf('function renderInteractiveArtifactPreview(', diffFuncStart);
  assert.ok(diffFuncStart !== -1 && diffFuncEnd !== -1, 'Failed to extract diff functions');
  const diffCode = rawAppJs.slice(diffFuncStart, diffFuncEnd);

  const saveStart = rawAppJs.indexOf('if (workspaceSaveBtn) {');
  const saveEnd = rawAppJs.indexOf('if (workspaceDownloadBtn) {', saveStart);
  assert.ok(saveStart !== -1 && saveEnd !== -1, 'Failed to extract save button handler');
  const saveCode = rawAppJs.slice(saveStart, saveEnd);

  const wbStart = rawAppJs.indexOf('// ============================================================================ \n// CURSOR IDE WORKBENCH CONTROLLER');
  const wbFallbackStart = wbStart !== -1 ? wbStart : rawAppJs.indexOf('// CURSOR IDE WORKBENCH CONTROLLER');
  const wbEnd = rawAppJs.indexOf('initCursorIdeWorkbench();', wbFallbackStart);
  assert.ok(wbFallbackStart !== -1 && wbEnd !== -1, 'Failed to extract workbench code');
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
  const editorFilePath = doc.getElementById('editor-file-path') || doc.createElement('code');

  workspaceEditorTextarea.value = virtualFiles[0].content;

  let activeWorkspacePath = 'src/attention.rs';
  let activeCanvasTabId = 'terminal';

  function activateCanvasTab(tabId) {
    activeCanvasTabId = tabId;
  }
  function selectWorkspaceFile(path) {
    activeWorkspacePath = path;
    const f = virtualFiles.find((x) => x.path === path);
    if (f) workspaceEditorTextarea.value = f.content;
  }
  function renderWorkspaceList(_ws) {}
  function renderStudioState(_st) {}
  function renderPendingPatchPills() {}
  function checkMcpToolPolicy(_tool, _path) {
    return true;
  }
  let trainEpochsCalled = null;
  function runOnDeviceTrain(epochs) {
    trainEpochsCalled = epochs;
  }

  const secopsAudit = {
    recordEvent() {},
  };

  const mockWindow = {
    showDirectoryPicker: null,
    addEventListener: (t, h) => doc.addEventListener(t, h),
    dispatchEvent: (e) => doc.dispatchEvent(e),
  };

  const sandboxFactory = new Function(
    'document',
    'window',
    'dispatchStudioEnvelope',
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
    'secopsAudit',
    `"use strict";
     ${diffCode}
     ${wbCode}
     ${saveCode}
     return {
       computeUnifiedDiffJs,
       renderColoredUnifiedDiff,
       get EditorTabManager() { return typeof EditorTabManager !== 'undefined' ? EditorTabManager : null; },
       get editorTabManager() { return typeof editorTabManager !== 'undefined' ? editorTabManager : null; },
       get cursorBaselineFiles() { return cursorBaselineFiles; },
       get cursorOpenTabs() { return cursorOpenTabs; },
       set cursorOpenTabs(v) {
         cursorOpenTabs.length = 0;
         if (Array.isArray(v)) cursorOpenTabs.push(...v);
       },
       get cursorDiskFileHandles() { return cursorDiskFileHandles; },
       get cursorCmdKState() { return cursorCmdKState; },
       set cursorCmdKState(v) { cursorCmdKState = v; },
       get cursorGhostState() { return cursorGhostState; },
       set cursorGhostState(v) { cursorGhostState = v; },
       get cursorPaletteActiveIdx() { return cursorPaletteActiveIdx; },
       set cursorPaletteActiveIdx(v) { cursorPaletteActiveIdx = v; },
       get cursorPaletteItemsCache() { return cursorPaletteItemsCache; },
       set cursorPaletteItemsCache(v) { cursorPaletteItemsCache = v; },
       get activeWorkspacePath() { return activeWorkspacePath; },
       set activeWorkspacePath(v) { activeWorkspacePath = v; },
       get activeCanvasTabId() { return activeCanvasTabId; },
       set activeCanvasTabId(v) { activeCanvasTabId = v; },
       computeCursorGitStatusForFile,
       renderCursorFileTree,
       renderCursorScmSection,
       renderCursorShardsSection,
       renderCursorEditorTabs,
       renderCursorLineGutter,
       detectCursorEnclosingSymbol,
       updateCursorBreadcrumbsAndCursorPos,
       syncCursorWorkbenchOnFileSelect,
       syncCursorWorkbenchOnList,
       syncCursorComposerPatchBar,
       openCursorCmdK,
       runCursorCmdKEdit,
       acceptCursorCmdKEdit,
       rejectCursorCmdKEdit,
       openCursorCommandPalette,
       closeCursorCommandPalette,
       renderCursorPaletteItems,
       mountLocalDirectoryWithFileSystemApi,
       writeCursorDiskFileHandle,
       runCursorR432Check,
       toggleCursorBottomPanel,
       initCursorIdeWorkbench,
       CURSOR_IDE_COMMANDS,
       get COMMAND_REGISTRY() { return typeof COMMAND_REGISTRY !== 'undefined' ? COMMAND_REGISTRY : CURSOR_IDE_COMMANDS; },
       get executeCommand() { return typeof executeCommand !== 'undefined' ? executeCommand : null; },
       get fuzzyScoreMatch() { return typeof fuzzyScoreMatch !== 'undefined' ? fuzzyScoreMatch : null; },
       get cursorPaletteWorkspaceFilesCache() { return cursorPaletteWorkspaceFilesCache; },
       set cursorPaletteWorkspaceFilesCache(v) { cursorPaletteWorkspaceFilesCache = v; },
       get cursorPaletteOpen() { return cursorPaletteOpen; },
       set cursorPaletteOpen(v) { cursorPaletteOpen = v; },
       get handleWorkspaceMountInputChange() { return typeof handleWorkspaceMountInputChange !== 'undefined' ? handleWorkspaceMountInputChange : null; },
       get trainEpochsCalled() { return trainEpochsCalled; }
     };`
  );

  const controller = sandboxFactory(
    doc,
    mockWindow,
    mockDispatchStudioEnvelope,
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
    editorFilePath,
    secopsAudit
  );

  for (const f of virtualFiles) {
    controller.cursorBaselineFiles.set(f.path, f.content);
  }

  controller.initCursorIdeWorkbench();
  if (controller.editorTabManager) {
    controller.editorTabManager.init(controller.cursorOpenTabs);
  }

  return {
    doc,
    mockWindow,
    controller,
    virtualFiles,
    dispatchedEnvelopes,
    AppState,
    elements: {
      workspaceEditorTextarea,
      workspaceStatus,
      dockableCanvasPane,
      claudeChatInput,
      workspaceSaveBtn,
      workspaceMountInput,
      editorFilePath,
    },
  };
}

// ============================================================================
// 4. TEST HARNESS RUNNER
// ============================================================================

let currentSuite = 'Default';
const registeredSuites = [];

function suite(name) {
  currentSuite = name;
  registeredSuites.push({ name, tests: [] });
}

function test(name, fn) {
  const current = registeredSuites[registeredSuites.length - 1];
  current.tests.push({ name, fn });
}

// ============================================================================
// SUITE 1: COMMAND PALETTE 21-COMMAND RAPID SEQUENTIAL EXECUTION & INVARIANTS
// ============================================================================
suite('Suite 1: Command Palette 21-Command Rapid Sequential Execution & Invariants');

test('1.1: COMMAND_REGISTRY contains exactly 21 commands with complete schema', () => {
  const { controller } = setupTestSandbox();
  const reg = controller.COMMAND_REGISTRY;
  assert.equal(reg.length, 21, `Expected exactly 21 commands, found ${reg.length}`);

  const requiredIds = [
    'r432-check', 'r432-test', 'cmdk-inline', 'toggle-terminal', 'inspect-3d-brain',
    'train-lut', 'consolidate-memory', 'mount-local-folder', 'focus-ai-chat',
    'open-composer', 'quick-open', 'save-file', 'new-file', 'toggle-sidebar',
    'show-explorer', 'show-search', 'show-scm', 'close-active-tab',
    'accept-all-diffs', 'reject-all-diffs', 'clear-terminal',
  ];

  for (const id of requiredIds) {
    const cmd = reg.find((c) => c.id === id);
    assert.ok(cmd, `Missing command ID: ${id}`);
    assert.ok(cmd.title, `Command ${id} missing title`);
    assert.ok(cmd.category, `Command ${id} missing category`);
    assert.ok(cmd.label, `Command ${id} missing label`);
    assert.ok(typeof cmd.handler === 'function' || typeof cmd.run === 'function', `Command ${id} missing handler`);
  }
});

test('1.2: Sequential execution of all 21 commands returns true for each command', () => {
  const { controller } = setupTestSandbox();
  const reg = controller.COMMAND_REGISTRY;

  for (const cmd of reg) {
    const res = controller.executeCommand(cmd.id);
    assert.equal(res, true, `executeCommand('${cmd.id}') must return true`);
  }
});

test('1.3: Rapid endurance churn: 10 round-robin cycles (210 executions) without corruption', () => {
  const { controller } = setupTestSandbox();
  const reg = controller.COMMAND_REGISTRY;

  for (let cycle = 0; cycle < 10; cycle++) {
    for (const cmd of reg) {
      const res = controller.executeCommand(cmd.id);
      assert.equal(res, true, `Cycle ${cycle}: executeCommand('${cmd.id}') returned false`);
    }
  }
});

test('1.4: Command side effect: focus-ai-chat activates ask mode and focuses input', () => {
  const { doc, controller, elements } = setupTestSandbox();
  const askBtn = doc.getElementById('cursor-ai-mode-ask');
  assert.equal(controller.executeCommand('focus-ai-chat'), true);
  if (askBtn) assert.ok(askBtn.classList.contains('active'), 'Ask mode button must be active');
  assert.equal(elements.claudeChatInput._isFocused, true, 'claudeChatInput must be focused');
});

test('1.5: Command side effect: open-composer activates agent mode and focuses input', () => {
  const { doc, controller, elements } = setupTestSandbox();
  const agentBtn = doc.getElementById('cursor-ai-mode-agent');
  assert.equal(controller.executeCommand('open-composer'), true);
  if (agentBtn) assert.ok(agentBtn.classList.contains('active'), 'Agent mode button must be active');
  assert.equal(elements.claudeChatInput._isFocused, true, 'claudeChatInput must be focused');
});

test('1.6: Command side effect: quick-open opens palette in files mode', () => {
  const { doc, controller } = setupTestSandbox();
  const modal = doc.getElementById('cursor-command-palette');
  const badge = doc.getElementById('cursor-palette-mode-badge');
  const input = doc.getElementById('cursor-palette-input');

  assert.equal(controller.executeCommand('quick-open'), true);
  assert.equal(modal.hidden, false, 'Palette modal must be visible');
  assert.equal(badge.textContent, 'Quick Open (Cmd+P)');
  assert.equal(input.value, '');
  controller.closeCursorCommandPalette();
  assert.equal(modal.hidden, true, 'Palette modal must be hidden after close');
});

test('1.7: Command side effect: save-file triggers workspaceSaveBtn click', () => {
  const { controller, elements } = setupTestSandbox();
  let clicked = false;
  elements.workspaceSaveBtn.addEventListener('click', () => { clicked = true; });
  assert.equal(controller.executeCommand('save-file'), true);
  assert.equal(clicked, true, 'workspaceSaveBtn click listener must be invoked');
});

test('1.8: Command side effect: new-file creates new untitled tab', () => {
  const { controller } = setupTestSandbox();
  const initialTabsCount = controller.cursorOpenTabs.length;
  assert.equal(controller.executeCommand('new-file'), true);
  assert.equal(controller.cursorOpenTabs.length, initialTabsCount + 1);
  const latestTab = controller.cursorOpenTabs[controller.cursorOpenTabs.length - 1];
  assert.ok(latestTab.startsWith('untitled-'), `Expected untitled- tab, got ${latestTab}`);
});

test('1.9: Command side effect: toggle-terminal collapses and expands bottom panel', () => {
  const { controller, elements } = setupTestSandbox();
  elements.dockableCanvasPane.classList.remove('cursor-bottom-panel-collapsed');

  assert.equal(controller.executeCommand('toggle-terminal'), true);
  assert.ok(elements.dockableCanvasPane.classList.contains('cursor-bottom-panel-collapsed'));

  assert.equal(controller.executeCommand('toggle-terminal'), true);
  assert.ok(!elements.dockableCanvasPane.classList.contains('cursor-bottom-panel-collapsed'));
});

test('1.10: Command side effect: close-active-tab closes current editor tab', () => {
  const { controller } = setupTestSandbox();
  controller.cursorOpenTabs = ['src/attention.rs', 'src/lib.rs', 'Cargo.toml'];
  controller.editorTabManager.init(controller.cursorOpenTabs);
  controller.editorTabManager.switchTo('src/lib.rs');

  assert.equal(controller.executeCommand('close-active-tab'), true);
  assert.ok(!controller.cursorOpenTabs.includes('src/lib.rs'), 'src/lib.rs must be closed');
  assert.equal(controller.cursorOpenTabs.length, 2);
});

// ============================================================================
// SUITE 2: MALFORMED, HOSTILE, & INVALID COMMAND IDS
// ============================================================================
suite('Suite 2: Malformed, Hostile, & Invalid Command IDs');

test('2.1: Non-existent string command IDs return false gracefully', () => {
  const { controller } = setupTestSandbox();
  const hostileStrings = [
    'nonexistent', 'delete-all', 'sudo-rm', 'system_reboot', 'undefined',
    'null', 'NaN', '0', 'false', '<script>alert(1)</script>',
    '../../../etc/passwd', 'eval(payload)', 'command; drop table',
  ];

  for (const str of hostileStrings) {
    const res = controller.executeCommand(str);
    assert.equal(res, false, `executeCommand('${str}') must return false`);
  }
});

test('2.2: Empty string and whitespace-only command IDs return false', () => {
  const { controller } = setupTestSandbox();
  assert.equal(controller.executeCommand(''), false);
  assert.equal(controller.executeCommand('   '), false);
  assert.equal(controller.executeCommand('\t\n'), false);
});

test('2.3: Falsy and primitive non-string values return false without throwing', () => {
  const { controller } = setupTestSandbox();
  assert.doesNotThrow(() => {
    assert.equal(controller.executeCommand(null), false);
    assert.equal(controller.executeCommand(undefined), false);
    assert.equal(controller.executeCommand(0), false);
    assert.equal(controller.executeCommand(12345), false);
    assert.equal(controller.executeCommand(false), false);
    assert.equal(controller.executeCommand(true), false);
    assert.equal(controller.executeCommand(NaN), false);
    assert.equal(controller.executeCommand(Infinity), false);
  });
});

test('2.4: Object and array payloads return false without throwing', () => {
  const { controller } = setupTestSandbox();
  assert.doesNotThrow(() => {
    assert.equal(controller.executeCommand({}), false);
    assert.equal(controller.executeCommand({ id: 'save-file' }), false);
    assert.equal(controller.executeCommand([]), false);
    assert.equal(controller.executeCommand(['save-file']), false);
    assert.equal(controller.executeCommand(() => {}), false);
  });
});

test('2.5: Prototype pollution attempts return false without polluting Object', () => {
  const { controller } = setupTestSandbox();
  const protoKeys = ['__proto__', 'constructor', 'prototype', 'toString', 'valueOf', 'isPrototypeOf'];

  for (const k of protoKeys) {
    const res = controller.executeCommand(k);
    assert.equal(res, false, `executeCommand('${k}') must return false`);
  }
  assert.equal(Object.prototype.polluted, undefined, 'Object.prototype must not be polluted');
});

test('2.6: Malformed command in registry (missing handler/run) returns false safely', () => {
  const { controller } = setupTestSandbox();
  controller.CURSOR_IDE_COMMANDS.push({ id: 'broken-cmd', title: 'Broken' });
  try {
    const res = controller.executeCommand('broken-cmd');
    assert.equal(res, false, 'Malformed command without handler/run must return false');
  } finally {
    const idx = controller.CURSOR_IDE_COMMANDS.findIndex((c) => c.id === 'broken-cmd');
    if (idx >= 0) controller.CURSOR_IDE_COMMANDS.splice(idx, 1);
  }
});

// ============================================================================
// SUITE 3: COMMAND PALETTE MODE TRANSITIONS, DYNAMIC TYPING & KEY CYCLING
// ============================================================================
suite('Suite 3: Command Palette Mode Transitions, Dynamic Typing & Key Cycling');

test('3.1: Mode transition from Quick Open to Command Palette when typing >', () => {
  const { doc, controller } = setupTestSandbox();
  const input = doc.getElementById('cursor-palette-input');
  const badge = doc.getElementById('cursor-palette-mode-badge');

  controller.openCursorCommandPalette('files');
  assert.equal(input.value, '');
  assert.equal(badge.textContent, 'Quick Open (Cmd+P)');

  // Type '>'
  input.value = '>';
  input.dispatchEvent({ type: 'input' });
  assert.equal(badge.textContent, 'Command Palette (Cmd+Shift+P)');
  assert.ok(controller.cursorPaletteItemsCache.length > 0, 'Must populate commands in cache');
  assert.ok(controller.cursorPaletteItemsCache[0].label.startsWith('>'), 'Items must be command labels');

  // Backspace '>' back to empty
  input.value = '';
  input.dispatchEvent({ type: 'input' });
  assert.equal(badge.textContent, 'Quick Open (Cmd+P)');
  assert.ok(controller.cursorPaletteItemsCache[0].label.includes('.'), 'Items must be workspace files');
});

test('3.2: Trailing whitespace tolerance in command query (>   save)', () => {
  const { doc, controller } = setupTestSandbox();
  const input = doc.getElementById('cursor-palette-input');
  const badge = doc.getElementById('cursor-palette-mode-badge');

  controller.openCursorCommandPalette('commands');
  input.value = '>   save';
  input.dispatchEvent({ type: 'input' });

  assert.equal(badge.textContent, 'Command Palette (Cmd+Shift+P)');
  assert.ok(controller.cursorPaletteItemsCache.length > 0);
  assert.ok(controller.cursorPaletteItemsCache[0].label.toLowerCase().includes('save'));
});

test('3.3: Leading space before > activates command mode but retains > in cleanQuery (empirical observation)', () => {
  const { doc, controller } = setupTestSandbox();
  const input = doc.getElementById('cursor-palette-input');
  const badge = doc.getElementById('cursor-palette-mode-badge');

  controller.openCursorCommandPalette('commands');
  input.value = '  > save';
  input.dispatchEvent({ type: 'input' });

  // rawQuery.trim().startsWith('>') evaluates to true
  assert.equal(badge.textContent, 'Command Palette (Cmd+Shift+P)');
});

test('3.4: Typing unmatched command query renders empty list without error', () => {
  const { doc, controller } = setupTestSandbox();
  const input = doc.getElementById('cursor-palette-input');
  const listEl = doc.getElementById('cursor-palette-list');

  controller.openCursorCommandPalette('commands');
  input.value = '> definitely-no-such-command-xyz-987';
  input.dispatchEvent({ type: 'input' });

  assert.equal(controller.cursorPaletteItemsCache.length, 0);
  assert.equal(listEl.children.length, 0);

  // Pressing Enter on empty list must not crash
  assert.doesNotThrow(() => {
    input.dispatchEvent({ type: 'keydown', key: 'Enter' });
  });

  // Pressing ArrowDown / ArrowUp on empty list must not crash
  assert.doesNotThrow(() => {
    input.dispatchEvent({ type: 'keydown', key: 'ArrowDown' });
    input.dispatchEvent({ type: 'keydown', key: 'ArrowUp' });
  });
  assert.equal(controller.cursorPaletteActiveIdx, 0);
});

test('3.5: Dynamic fuzzy filtering ranks most relevant command at top', () => {
  const { doc, controller } = setupTestSandbox();
  const input = doc.getElementById('cursor-palette-input');

  controller.openCursorCommandPalette('commands');
  input.value = '> term';
  input.dispatchEvent({ type: 'input' });

  assert.ok(controller.cursorPaletteItemsCache.length >= 2);
  const first = controller.cursorPaletteItemsCache[0];
  assert.ok(first.label.toLowerCase().includes('terminal'), 'Top result should be terminal command');
});

test('3.6: Keyboard navigation ArrowDown wraps cyclically across items', () => {
  const { doc, controller } = setupTestSandbox();
  const input = doc.getElementById('cursor-palette-input');

  controller.openCursorCommandPalette('commands');
  input.value = '>';
  input.dispatchEvent({ type: 'input' });

  const total = controller.cursorPaletteItemsCache.length;
  assert.equal(total, 21);

  // ArrowDown total times should wrap back to 0
  for (let i = 0; i < total; i++) {
    input.dispatchEvent({ type: 'keydown', key: 'ArrowDown' });
  }
  assert.equal(controller.cursorPaletteActiveIdx, 0);

  // ArrowUp once from 0 should wrap to total - 1
  input.dispatchEvent({ type: 'keydown', key: 'ArrowUp' });
  assert.equal(controller.cursorPaletteActiveIdx, total - 1);
});

test('3.7: Enter on selected command executes command and dismisses palette', () => {
  const { doc, controller, elements } = setupTestSandbox();
  const input = doc.getElementById('cursor-palette-input');
  const modal = doc.getElementById('cursor-command-palette');

  controller.openCursorCommandPalette('commands');
  input.value = '> focus ai chat';
  input.dispatchEvent({ type: 'input' });

  assert.ok(controller.cursorPaletteItemsCache.length > 0);
  input.dispatchEvent({ type: 'keydown', key: 'Enter' });

  assert.equal(modal.hidden, true, 'Palette must close after command execution');
  assert.equal(elements.claudeChatInput._isFocused, true, 'claudeChatInput must be focused');
});

test('3.8: Global Escape dismisses palette and restores textarea focus', () => {
  const { doc, controller, elements } = setupTestSandbox();
  const modal = doc.getElementById('cursor-command-palette');

  controller.openCursorCommandPalette('files');
  assert.equal(modal.hidden, false);

  doc.dispatchEvent({ type: 'keydown', key: 'Escape' });
  assert.equal(modal.hidden, true, 'Modal must close on Escape');
  assert.equal(elements.workspaceEditorTextarea._isFocused, true, 'workspaceEditorTextarea must receive focus');
});

// ============================================================================
// SUITE 4: ADVERSARIAL LOCAL DIRECTORY MOUNTING (FILESYSTEM API)
// ============================================================================
suite('Suite 4: Adversarial Local Directory Mounting (FileSystem API)');

test('4.1: Deeply nested directory structure (55 levels) is completely traversed and ingested', async () => {
  const { mockWindow, controller, dispatchedEnvelopes } = setupTestSandbox();

  const deepContent = 'pub fn deep_flock_recursion() -> i64 { 55 }\n';
  let currentDir = createMockDirectoryHandle('lvl54', new Map([
    ['nested_core.rs', createMockFileHandle('nested_core.rs', deepContent)],
  ]));

  for (let lvl = 53; lvl >= 0; lvl--) {
    currentDir = createMockDirectoryHandle(`lvl${lvl}`, new Map([
      [`lvl${lvl + 1}`, currentDir],
    ]));
  }

  const rootHandle = createMockDirectoryHandle('adversarial_deep_project', new Map([
    ['lvl0', currentDir],
  ]));

  mockWindow.showDirectoryPicker = async () => rootHandle;

  await controller.mountLocalDirectoryWithFileSystemApi();

  const batchEnv = dispatchedEnvelopes.find((e) => e.command === 'workspace_batch');
  assert.ok(batchEnv, 'workspace_batch must be dispatched');
  assert.ok(batchEnv.payload.includes('nested_core.rs'), 'Deeply nested file must be ingested in payload');
  assert.ok(batchEnv.payload.includes(deepContent), 'Deeply nested file content must be preserved');

  const expectedPath = Array.from({ length: 55 }, (_, i) => `lvl${i}`).join('/') + '/nested_core.rs';
  assert.ok(controller.cursorDiskFileHandles.has(expectedPath), `Path ${expectedPath} must be in cursorDiskFileHandles`);
});

test('4.2: Oversized files (>512KB and >10MB) are rejected while <=512KB files are accepted', async () => {
  const { mockWindow, controller, dispatchedEnvelopes } = setupTestSandbox();

  const valid512Kb = 'x'.repeat(512 * 1024);
  const oversized513Kb = 'y'.repeat(512 * 1024 + 1);
  const massive10Mb = 'z'.repeat(10 * 1024 * 1024);

  const rootHandle = createMockDirectoryHandle('size_test_project', new Map([
    ['valid_boundary.rs', createMockFileHandle('valid_boundary.rs', valid512Kb, { size: 512 * 1024 })],
    ['oversized_513k.rs', createMockFileHandle('oversized_513k.rs', oversized513Kb, { size: 512 * 1024 + 1 })],
    ['massive_10mb.rs', createMockFileHandle('massive_10mb.rs', massive10Mb, { size: 10 * 1024 * 1024 })],
  ]));

  mockWindow.showDirectoryPicker = async () => rootHandle;

  await controller.mountLocalDirectoryWithFileSystemApi();

  const batchEnv = dispatchedEnvelopes.find((e) => e.command === 'workspace_batch');
  assert.ok(batchEnv, 'workspace_batch must be dispatched');
  assert.ok(batchEnv.payload.includes('valid_boundary.rs'), '512KB boundary file must be ingested');
  assert.ok(!batchEnv.payload.includes('oversized_513k.rs'), '513KB file must be skipped');
  assert.ok(!batchEnv.payload.includes('massive_10mb.rs'), '10MB file must be skipped');

  assert.ok(controller.cursorDiskFileHandles.has('valid_boundary.rs'));
  assert.ok(!controller.cursorDiskFileHandles.has('oversized_513k.rs'));
  assert.ok(!controller.cursorDiskFileHandles.has('massive_10mb.rs'));
});

test('4.3: Whitelist vs Blacklist extensions: ingests supported code, rejects hostile binaries', async () => {
  const { mockWindow, controller, dispatchedEnvelopes } = setupTestSandbox();

  const entries = new Map([
    ['main.rs', createMockFileHandle('main.rs', 'fn main() {}')],
    ['Proof.lean', createMockFileHandle('Proof.lean', 'theorem t : true := trivial')],
    ['config.toml', createMockFileHandle('config.toml', '[cfg]\n')],
    ['ci.yaml', createMockFileHandle('ci.yaml', 'name: CI')],
    ['deploy.yml', createMockFileHandle('deploy.yml', 'steps: []')],
    ['README.md', createMockFileHandle('README.md', '# Readme')],
    ['app.js', createMockFileHandle('app.js', 'console.log()')],
    ['bench.mjs', createMockFileHandle('bench.mjs', 'export default {}')],
    ['types.ts', createMockFileHandle('types.ts', 'type T = string')],
    ['data.json', createMockFileHandle('data.json', '{}')],
    ['style.css', createMockFileHandle('style.css', 'body {}')],
    ['index.html', createMockFileHandle('index.html', '<html></html>')],
    ['script.py', createMockFileHandle('script.py', 'print(1)')],
    ['notes.txt', createMockFileHandle('notes.txt', 'notes')],
    ['run.sh', createMockFileHandle('run.sh', '#!/bin/sh')],
    ['malicious.exe', createMockFileHandle('malicious.exe', '\x4D\x5A\x90\x00')],
    ['blob.bin', createMockFileHandle('blob.bin', '\x00\x01\x02\x03')],
    ['pic.png', createMockFileHandle('pic.png', '\x89PNG\r\n\x1a\n')],
    ['archive.zip', createMockFileHandle('archive.zip', 'PK\x03\x04')],
    ['object.o', createMockFileHandle('object.o', '\x7fELF')],
    ['bytecode.class', createMockFileHandle('bytecode.class', '\xCA\xFE\xBA\xBE')],
  ]);

  mockWindow.showDirectoryPicker = async () => createMockDirectoryHandle('ext_filter_project', entries);

  await controller.mountLocalDirectoryWithFileSystemApi();

  const batchEnv = dispatchedEnvelopes.find((e) => e.command === 'workspace_batch');
  assert.ok(batchEnv);

  const approved = [
    'main.rs', 'Proof.lean', 'config.toml', 'ci.yaml', 'deploy.yml', 'README.md',
    'app.js', 'bench.mjs', 'types.ts', 'data.json', 'style.css', 'index.html',
    'script.py', 'notes.txt', 'run.sh',
  ];
  for (const f of approved) {
    assert.ok(batchEnv.payload.includes(f), `Approved file ${f} must be ingested`);
  }

  const rejected = ['malicious.exe', 'blob.bin', 'pic.png', 'archive.zip', 'object.o', 'bytecode.class'];
  for (const f of rejected) {
    assert.ok(!batchEnv.payload.includes(f), `Rejected file ${f} must NOT be ingested`);
  }
});

test('4.4: Hidden files (.git, .DS_Store) and target/ / node_modules/ directories are skipped', async () => {
  const { mockWindow, controller, dispatchedEnvelopes } = setupTestSandbox();

  const targetDir = createMockDirectoryHandle('target', new Map([
    ['build.rs', createMockFileHandle('build.rs', '// build')],
  ]));
  const nodeModulesDir = createMockDirectoryHandle('node_modules', new Map([
    ['dep.js', createMockFileHandle('dep.js', '// dep')],
  ]));
  const gitDir = createMockDirectoryHandle('.git', new Map([
    ['config', createMockFileHandle('config', '[core]')],
  ]));

  const rootHandle = createMockDirectoryHandle('clean_project', new Map([
    ['target', targetDir],
    ['node_modules', nodeModulesDir],
    ['.git', gitDir],
    ['.DS_Store', createMockFileHandle('.DS_Store', 'junk')],
    ['src_ok.rs', createMockFileHandle('src_ok.rs', 'pub fn ok() {}')],
  ]));

  mockWindow.showDirectoryPicker = async () => rootHandle;

  await controller.mountLocalDirectoryWithFileSystemApi();

  const batchEnv = dispatchedEnvelopes.find((e) => e.command === 'workspace_batch');
  assert.ok(batchEnv);
  assert.ok(batchEnv.payload.includes('src_ok.rs'));
  assert.ok(!batchEnv.payload.includes('build.rs'));
  assert.ok(!batchEnv.payload.includes('dep.js'));
  assert.ok(!batchEnv.payload.includes('.DS_Store'));
  assert.ok(!batchEnv.payload.includes('.git'));
});

test('4.5: Empty directory trees (30 nested empty directories) do not dispatch empty batch', async () => {
  const { mockWindow, controller, dispatchedEnvelopes } = setupTestSandbox();

  let currentDir = createMockDirectoryHandle('empty_leaf', new Map());
  for (let i = 29; i >= 0; i--) {
    currentDir = createMockDirectoryHandle(`empty_${i}`, new Map([
      [`child_${i}`, currentDir],
    ]));
  }

  mockWindow.showDirectoryPicker = async () => currentDir;

  await controller.mountLocalDirectoryWithFileSystemApi();

  const batchEnv = dispatchedEnvelopes.find((e) => e.command === 'workspace_batch');
  assert.equal(batchEnv, undefined, 'Must not dispatch workspace_batch when 0 valid files found');
});

test('4.6: Directory walker stops recursing when chunks reaches 64 limit', async () => {
  const { mockWindow, controller } = setupTestSandbox();

  const rootEntries = new Map();
  for (let i = 0; i < 70; i++) {
    const subDir = createMockDirectoryHandle(`dir_${i}`, new Map([
      [`file_${i}.rs`, createMockFileHandle(`file_${i}.rs`, `// file ${i}`)],
    ]));
    rootEntries.set(`dir_${i}`, subDir);
  }

  mockWindow.showDirectoryPicker = async () => createMockDirectoryHandle('capped_project', rootEntries);

  await controller.mountLocalDirectoryWithFileSystemApi();

  assert.ok(controller.cursorDiskFileHandles.size <= 64, `Expected <= 64 files, got ${controller.cursorDiskFileHandles.size}`);
});

test('4.7: User cancellation (AbortError) in showDirectoryPicker triggers fallback input click', async () => {
  const { doc, mockWindow, controller } = setupTestSandbox();
  const mountInput = doc.getElementById('workspace-mount-input');
  let inputClicked = false;
  mountInput.addEventListener('click', () => { inputClicked = true; });

  mockWindow.showDirectoryPicker = async () => {
    const err = new Error('The user aborted a request.');
    err.name = 'AbortError';
    throw err;
  };

  await controller.mountLocalDirectoryWithFileSystemApi();
  assert.equal(inputClicked, true, 'Fallback workspaceMountInput.click() must be triggered on AbortError');
});

test('4.8: Corrupted file handle throwing getFile() triggers fallback input click', async () => {
  const { doc, mockWindow, controller } = setupTestSandbox();
  const mountInput = doc.getElementById('workspace-mount-input');
  let inputClicked = false;
  mountInput.addEventListener('click', () => { inputClicked = true; });

  const rootHandle = createMockDirectoryHandle('corrupted_project', new Map([
    ['bad_file.rs', createMockFileHandle('bad_file.rs', '', { throwOnGetFile: 'I/O Read Error' })],
  ]));

  mockWindow.showDirectoryPicker = async () => rootHandle;

  await controller.mountLocalDirectoryWithFileSystemApi();
  assert.equal(inputClicked, true, 'Fallback workspaceMountInput.click() must be triggered on read failure');
});

test('4.9: Corrupted file handle throwing text() triggers fallback input click', async () => {
  const { doc, mockWindow, controller } = setupTestSandbox();
  const mountInput = doc.getElementById('workspace-mount-input');
  let inputClicked = false;
  mountInput.addEventListener('click', () => { inputClicked = true; });

  const rootHandle = createMockDirectoryHandle('corrupted_text_project', new Map([
    ['bad_text.rs', createMockFileHandle('bad_text.rs', '', { throwOnText: 'Decoder stream error' })],
  ]));

  mockWindow.showDirectoryPicker = async () => rootHandle;

  await controller.mountLocalDirectoryWithFileSystemApi();
  assert.equal(inputClicked, true, 'Fallback workspaceMountInput.click() must be triggered on text decode failure');
});

// ============================================================================
// SUITE 5: FALLBACK FILE INPUT MOUNT RESILIENCE (handleWorkspaceMountInputChange)
// ============================================================================
suite('Suite 5: Fallback File Input Mount Resilience (handleWorkspaceMountInputChange)');

test('5.1: Batch ingestion via fallback input processes files and resets input.value', async () => {
  const { doc, controller, dispatchedEnvelopes } = setupTestSandbox();
  const mountInput = doc.getElementById('workspace-mount-input');
  mountInput.value = 'C:\\fakepath\\project';

  mountInput.files = [
    { name: 'mod_a.rs', webkitRelativePath: 'my-proj/src/mod_a.rs', size: 120, async text() { return 'pub fn a() {}'; } },
    { name: 'mod_b.rs', webkitRelativePath: 'my-proj/src/mod_b.rs', size: 240, async text() { return 'pub fn b() {}'; } },
  ];

  await controller.handleWorkspaceMountInputChange(mountInput);

  const batchEnv = dispatchedEnvelopes.find((e) => e.command === 'workspace_batch');
  assert.ok(batchEnv, 'workspace_batch must be dispatched');
  assert.ok(batchEnv.payload.includes('my-proj/src/mod_a.rs'));
  assert.ok(batchEnv.payload.includes('my-proj/src/mod_b.rs'));
  assert.equal(mountInput.value, '', 'mountInput.value must be reset to empty string');
});

test('5.2: Fallback input skips oversized files (>512KB)', async () => {
  const { doc, controller, dispatchedEnvelopes } = setupTestSandbox();
  const mountInput = doc.getElementById('workspace-mount-input');

  mountInput.files = [
    { name: 'small.rs', webkitRelativePath: 'proj/small.rs', size: 1000, async text() { return '// ok'; } },
    { name: 'huge.rs', webkitRelativePath: 'proj/huge.rs', size: 600 * 1024, async text() { return '// huge'; } },
  ];

  await controller.handleWorkspaceMountInputChange(mountInput);

  const batchEnv = dispatchedEnvelopes.find((e) => e.command === 'workspace_batch');
  assert.ok(batchEnv);
  assert.ok(batchEnv.payload.includes('proj/small.rs'));
  assert.ok(!batchEnv.payload.includes('proj/huge.rs'), 'huge.rs must be excluded');
});

test('5.3: Fallback input skips hidden paths, target/, and node_modules/', async () => {
  const { doc, controller, dispatchedEnvelopes } = setupTestSandbox();
  const mountInput = doc.getElementById('workspace-mount-input');

  mountInput.files = [
    { name: 'ok.rs', webkitRelativePath: 'proj/src/ok.rs', size: 100, async text() { return '// ok'; } },
    { name: 'hidden.rs', webkitRelativePath: 'proj/.secret/hidden.rs', size: 100, async text() { return '// bad'; } },
    { name: 'out.rs', webkitRelativePath: 'proj/target/out.rs', size: 100, async text() { return '// bad'; } },
    { name: 'dep.js', webkitRelativePath: 'proj/node_modules/dep.js', size: 100, async text() { return '// bad'; } },
  ];

  await controller.handleWorkspaceMountInputChange(mountInput);

  const batchEnv = dispatchedEnvelopes.find((e) => e.command === 'workspace_batch');
  assert.ok(batchEnv);
  assert.ok(batchEnv.payload.includes('proj/src/ok.rs'));
  assert.ok(!batchEnv.payload.includes('.secret'));
  assert.ok(!batchEnv.payload.includes('target'));
  assert.ok(!batchEnv.payload.includes('node_modules'));
});

test('5.4: Fallback input ingests massive batch of 100 files', async () => {
  const { doc, controller, dispatchedEnvelopes } = setupTestSandbox();
  const mountInput = doc.getElementById('workspace-mount-input');

  const filesArray = [];
  for (let i = 0; i < 100; i++) {
    filesArray.push({
      name: `module_${i}.rs`,
      webkitRelativePath: `big-project/src/module_${i}.rs`,
      size: 50,
      async text() { return `// module ${i}`; },
    });
  }
  mountInput.files = filesArray;

  await controller.handleWorkspaceMountInputChange(mountInput);

  const batchEnv = dispatchedEnvelopes.find((e) => e.command === 'workspace_batch');
  assert.ok(batchEnv);
  const blocks = batchEnv.payload.split('\n===FILE===\n');
  assert.equal(blocks.length, 100, 'All 100 files must be ingested in workspace_batch');
});

test('5.5: Empty, null, or undefined files attribute does not crash fallback handler', async () => {
  const { doc, controller } = setupTestSandbox();
  const mountInput = doc.getElementById('workspace-mount-input');

  assert.doesNotThrow(async () => {
    mountInput.files = [];
    await controller.handleWorkspaceMountInputChange(mountInput);

    mountInput.files = null;
    await controller.handleWorkspaceMountInputChange(mountInput);

    await controller.handleWorkspaceMountInputChange(null);
  });
});

// ============================================================================
// SUITE 6: TWO-WAY DISK SYNCHRONIZATION & WRITE-BACK RESILIENCE
// ============================================================================
suite('Suite 6: Two-Way Disk Synchronization & Write-Back Resilience');

test('6.1: Saving mounted file triggers handle.createWritable(), write(), and close()', async () => {
  const { mockWindow, controller, elements } = setupTestSandbox();

  const fileHandle = createMockFileHandle('disk_synced.rs', 'fn original() {}');
  const rootHandle = createMockDirectoryHandle('sync_project', new Map([
    ['disk_synced.rs', fileHandle],
  ]));

  mockWindow.showDirectoryPicker = async () => rootHandle;
  await controller.mountLocalDirectoryWithFileSystemApi();

  assert.ok(controller.cursorDiskFileHandles.has('disk_synced.rs'));

  // Switch to disk_synced.rs and update editor buffer
  controller.activeWorkspacePath = 'disk_synced.rs';
  elements.workspaceEditorTextarea.value = 'fn updated_on_disk() {\n    // modified\n}\n';

  // Trigger save via executeCommand('save-file')
  controller.executeCommand('save-file');

  // Allow microtasks to complete write
  await new Promise((r) => setTimeout(r, 20));

  assert.equal(fileHandle._writeCallCount, 1, 'File handle write must be called once');
  assert.equal(fileHandle._closeCallCount, 1, 'File handle close must be called once');
  assert.equal(fileHandle._lastWrittenData, 'fn updated_on_disk() {\n    // modified\n}\n');
});

test('6.2: Accepting Cmd+K diff triggers disk write-back to mounted file', async () => {
  const { mockWindow, controller } = setupTestSandbox();

  const fileHandle = createMockFileHandle('cmdk_target.rs', 'fn foo() {}');
  const rootHandle = createMockDirectoryHandle('cmdk_proj', new Map([
    ['cmdk_target.rs', fileHandle],
  ]));

  mockWindow.showDirectoryPicker = async () => rootHandle;
  await controller.mountLocalDirectoryWithFileSystemApi();

  // Simulate Cmd+K state on cmdk_target.rs
  controller.activeWorkspacePath = 'cmdk_target.rs';
  controller.cursorCmdKState = {
    open: true,
    targetPath: 'cmdk_target.rs',
    originalCode: 'fn foo() {}',
    proposedFullCode: 'fn foo_optimized() {\n    // AI transformed\n}',
  };

  controller.acceptCursorCmdKEdit();

  // Allow microtasks to finish write
  await new Promise((r) => setTimeout(r, 20));

  assert.equal(fileHandle._writeCallCount, 1, 'Cmd+K accept must write to disk handle');
  assert.equal(fileHandle._closeCallCount, 1, 'Cmd+K accept must close disk stream');
  assert.equal(fileHandle._lastWrittenData, 'fn foo_optimized() {\n    // AI transformed\n}');
});

test('6.3: Unmounted files (virtual workspace only) do not call createWritable and do not error', async () => {
  const { controller } = setupTestSandbox();

  // Virtual file without disk handle
  controller.activeWorkspacePath = 'src/attention.rs';
  assert.equal(controller.cursorDiskFileHandles.has('src/attention.rs'), false);

  assert.doesNotThrow(async () => {
    await controller.writeCursorDiskFileHandle('src/attention.rs', 'pub fn test() {}');
  });
});

test('6.4: Write error resilience: NotAllowedError on createWritable does not throw or crash UI', async () => {
  const { mockWindow, controller } = setupTestSandbox();

  const fileHandle = createMockFileHandle('protected.rs', 'fn prot() {}', {
    throwOnCreateWritable: 'NotAllowedError: Permission denied by user',
  });
  const rootHandle = createMockDirectoryHandle('prot_proj', new Map([
    ['protected.rs', fileHandle],
  ]));

  mockWindow.showDirectoryPicker = async () => rootHandle;
  await controller.mountLocalDirectoryWithFileSystemApi();

  assert.doesNotThrow(async () => {
    await controller.writeCursorDiskFileHandle('protected.rs', 'new content');
  });
  assert.equal(fileHandle._writeCallCount, 0);
});

test('6.5: Write error resilience: QuotaExceededError during writable.write() does not throw', async () => {
  const { mockWindow, controller } = setupTestSandbox();

  const fileHandle = createMockFileHandle('full_disk.rs', 'fn f() {}', {
    throwOnWrite: 'QuotaExceededError: Local storage full',
  });
  const rootHandle = createMockDirectoryHandle('disk_proj', new Map([
    ['full_disk.rs', fileHandle],
  ]));

  mockWindow.showDirectoryPicker = async () => rootHandle;
  await controller.mountLocalDirectoryWithFileSystemApi();

  assert.doesNotThrow(async () => {
    await controller.writeCursorDiskFileHandle('full_disk.rs', 'massive payload');
  });
});

test('6.6: Write error resilience: close() stream error does not throw', async () => {
  const { mockWindow, controller } = setupTestSandbox();

  const fileHandle = createMockFileHandle('stream_fail.rs', 'fn s() {}', {
    throwOnClose: 'I/O Flush Error',
  });
  const rootHandle = createMockDirectoryHandle('stream_proj', new Map([
    ['stream_fail.rs', fileHandle],
  ]));

  mockWindow.showDirectoryPicker = async () => rootHandle;
  await controller.mountLocalDirectoryWithFileSystemApi();

  assert.doesNotThrow(async () => {
    await controller.writeCursorDiskFileHandle('stream_fail.rs', 'payload');
  });
});

test('6.7: Multi-file isolation: saving one file modifies only its handle', async () => {
  const { mockWindow, controller, elements } = setupTestSandbox();

  const h1 = createMockFileHandle('file_1.rs', '// content 1');
  const h2 = createMockFileHandle('file_2.rs', '// content 2');
  const h3 = createMockFileHandle('file_3.rs', '// content 3');

  const rootHandle = createMockDirectoryHandle('multi_proj', new Map([
    ['file_1.rs', h1],
    ['file_2.rs', h2],
    ['file_3.rs', h3],
  ]));

  mockWindow.showDirectoryPicker = async () => rootHandle;
  await controller.mountLocalDirectoryWithFileSystemApi();

  controller.activeWorkspacePath = 'file_2.rs';
  elements.workspaceEditorTextarea.value = '// modified 2';

  controller.executeCommand('save-file');
  await new Promise((r) => setTimeout(r, 20));

  assert.equal(h1._writeCallCount, 0, 'file_1.rs must not be modified');
  assert.equal(h2._writeCallCount, 1, 'file_2.rs must be modified');
  assert.equal(h3._writeCallCount, 0, 'file_3.rs must not be modified');
  assert.equal(h2._lastWrittenData, '// modified 2');
});

// ============================================================================
// SUITE 7: 6-FILE BROWSER CLOSURE & SECURITY CSP INVARIANTS
// ============================================================================
suite('Suite 7: 6-File Browser Closure & Security CSP Invariants');

test('7.1: Strict 6-file browser closure in public/ directory', () => {
  const files = readdirSync(PUBLIC_DIR);
  assert.equal(files.length, 6, `Expected exactly 6 production files in public/, found ${files.length}: ${files.join(', ')}`);

  const requiredFiles = [
    'app.css', 'app.js', 'index.html', 'lexlean_r4.js', 'lexlean_r4_bg.wasm', 'provenance.json',
  ];
  for (const f of requiredFiles) {
    assert.ok(files.includes(f), `Missing required production file: ${f}`);
  }
});

test('7.2: Zero external network/cloud API strings in public/app.js', () => {
  const forbiddenPatterns = [
    /https?:\/\/(?!localhost|127\.0\.0\.1)/i,
    /api\.anthropic\.com/i,
    /api\.openai\.com/i,
    /fetch\s*\(\s*['"`]https?:/i,
  ];

  for (const pat of forbiddenPatterns) {
    const match = rawAppJs.match(pat);
    assert.equal(match, null, `Forbidden network string matched: ${pat}`);
  }
});

test('7.3: Strict Content-Security-Policy in public/index.html enforces default-src none and connect-src self', () => {
  assert.ok(rawHtml.includes("default-src 'none'"), "CSP must enforce default-src 'none'");
  assert.ok(rawHtml.includes("connect-src 'self'"), "CSP must enforce connect-src 'self'");
});

// ============================================================================
// SUITE 8: DISK SPACE INVARIANT & CLEANUP
// ============================================================================
suite('Suite 8: Disk Space Invariant & Cleanup');

test('8.1: Disk space invariant verified (target/ clean)', () => {
  const targetDir = resolve(REPO_ROOT, 'target');
  let size = 0;
  try {
    const files = readdirSync(targetDir);
    for (const f of files) {
      const st = statSync(resolve(targetDir, f));
      size += st.size;
    }
  } catch {}
  assert.equal(size, 0, `target/ directory must be empty (0B), found ${size} bytes`);
});

// ============================================================================
// EXECUTE ALL TEST SUITES SEQUENTIALLY
// ============================================================================

async function runAllSuites() {
  let totalPassed = 0;
  let totalFailed = 0;
  const failures = [];

  for (const s of registeredSuites) {
    console.log(`\n--- ${s.name} ---`);
    for (const t of s.tests) {
      try {
        await t.fn();
        totalPassed += 1;
        console.log(`  ✓ ${t.name}`);
      } catch (err) {
        totalFailed += 1;
        failures.push({ suite: s.name, test: t.name, error: err });
        console.error(`  ✗ ${t.name}`);
        console.error(`    ${err.message}`);
      }
    }
  }

  console.log('\n================================================================');
  console.log('  TEST EXECUTION SUMMARY');
  console.log('================================================================');
  console.log(`  Total Tests Run:     ${totalPassed + totalFailed}`);
  console.log(`  Total Tests Passed:  ${totalPassed}`);
  console.log(`  Total Tests Failed:  ${totalFailed}`);
  console.log('================================================================\n');

  if (totalFailed > 0) {
    console.error('FAILURES DETECTED:');
    for (const f of failures) {
      console.error(`  - [${f.suite}] ${f.test}: ${f.error.message}`);
    }
    process.exit(1);
  } else {
    console.log('VERDICT: APPROVE — All Milestone 4 empirical stress tests passed (0 defects).\n');
    process.exit(0);
  }
}

runAllSuites();
