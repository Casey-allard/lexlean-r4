/**
 * Empirical Stress-Test Harness for LexLean Chatbox Interface (Requirement R2 / Milestone M1)
 *
 * Exercises:
 * 1. History stepping (0, 1, 5, 20 items, forward & backward recall, boundary clamping).
 * 2. Uncommitted draft restoration (typing draft, Up 3x, Down 3x -> byte-for-byte draft restoration).
 * 3. Rapid model switching across all 5 models (1,000 rapid cycles, state synchronization).
 * 4. File attachment chip addition, duplicate rejection, removal, and re-addition.
 * 5. Parity between crates/wasm/ts/composer.ts and public/app.js.
 * 6. DOM-level event simulation (typing, auto-expand, model change, file attachment chips).
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

console.log('=== LexLean Chatbox Interface Empirical Stress Harness ===\n');

// 1. Load TypeScript Source directly via deterministic type stripping (compatible with Node 18/20/22+)
const tsPath = resolve(REPO_ROOT, 'crates/wasm/ts/composer.ts');
const tsRawSource = readFileSync(tsPath, 'utf8');

// Verify TypeScript type definitions are intact in composer.ts before stripping
assert.ok(tsRawSource.includes('export type ComposerMode ='), 'composer.ts must define ComposerMode');
assert.ok(tsRawSource.includes('export interface ComposerAttachmentChip'), 'composer.ts must define ComposerAttachmentChip');
assert.ok(tsRawSource.includes('export interface ComposerModelOption'), 'composer.ts must define ComposerModelOption');
assert.ok(tsRawSource.includes('export interface ComposerState'), 'composer.ts must define ComposerState');

function stripComposerTypeScript(src) {
  return src
    .replace(/export\s+type\s+\w+\s*=[^;]+;/g, '')
    .replace(/export\s+interface\s+\w+\s*\{[\s\S]*?\n\}/g, '')
    .replace(/export\s+const\s+(\w+)\s*:\s*[^=]+=/g, 'const $1 =')
    .replace(/export\s+class\s+/g, 'class ')
    .replace(/^\s*private\s+state\s*:\s*ComposerState\s*;\s*$/gm, '')
    .replace(/const\s+chip\s*:\s*ComposerAttachmentChip\s*=/g, 'const chip =')
    .replace(/public\s+detectAutocompleteTrigger\s*\(\s*text\s*:\s*string\s*,\s*cursorOffset\s*:\s*number\s*\)\s*:\s*\{[\s\S]*?\}\s*\{/g, 'detectAutocompleteTrigger(text, cursorOffset) {')
    .replace(/public\s+attachFile\s*\(\s*file\s*:\s*\{[^}]+\}\s*\)\s*:\s*boolean\s*\{/g, 'attachFile(file) {')
    .replace(/public\s+(\w+)\s*\(([^)]*)\)\s*:\s*[^{]+\{/g, (_m, name, params) => {
      const cleanParams = params
        .split(',')
        .map((p) => p.trim().replace(/\??\s*:.*$/, ''))
        .filter(Boolean)
        .join(', ');
      return `${name}(${cleanParams}) {`;
    });
}

const transpiledTs = stripComposerTypeScript(tsRawSource);
const { AgenticTypeScriptComposer, COMPOSER_MODELS, AGENTIC_SLASH_COMMANDS } = new Function(
  `"use strict";
   ${transpiledTs}
   return { AgenticTypeScriptComposer, COMPOSER_MODELS, AGENTIC_SLASH_COMMANDS };`
)();

// 2. Extract and evaluate AgenticTypeScriptComposer directly from public/app.js
const appJsPath = resolve(REPO_ROOT, 'public/app.js');
const appJsContent = readFileSync(appJsPath, 'utf8');

// Isolate the AgenticTypeScriptComposer definition from app.js
const composerClassMatch = appJsContent.match(
  /class AgenticTypeScriptComposer \{[\s\S]*?\n\}/
);
assert.ok(composerClassMatch, 'Failed to extract AgenticTypeScriptComposer from public/app.js');

const composerModelsMatch = appJsContent.match(
  /const COMPOSER_MODELS = (\[[\s\S]*?\]);/
);
assert.ok(composerModelsMatch, 'Failed to extract COMPOSER_MODELS from public/app.js');

const EmbeddedComposerClass = new Function(
  `"use strict";
   const COMPOSER_MODELS = ${composerModelsMatch[1]};
   const AGENTIC_SLASH_COMMANDS = [];
   ${composerClassMatch[0]};
   return { AgenticTypeScriptComposer, COMPOSER_MODELS };`
)();

console.log('✓ Successfully loaded both TypeScript source and embedded app.js classes.\n');

// Run test against both implementations to guarantee 100% parity
const implementations = [
  { name: 'TypeScript Source (composer.ts)', ComposerClass: AgenticTypeScriptComposer, models: COMPOSER_MODELS },
  { name: 'Embedded app.js (public/app.js)', ComposerClass: EmbeddedComposerClass.AgenticTypeScriptComposer, models: EmbeddedComposerClass.COMPOSER_MODELS },
];

for (const { name, ComposerClass, models } of implementations) {
  console.log(`--- Testing Implementation: ${name} ---`);

  // =========================================================================
  // SUITE 1: HISTORY STEPPING (0, 1, 5, 20 items, forward & backward recall)
  // =========================================================================
  console.log('  Suite 1: History Stepping');

  // Case 1.1: 0 items
  {
    const composer = new ComposerClass();
    assert.equal(composer.stepHistory('up'), null, 'Up on 0 items must return null');
    assert.equal(composer.stepHistory('down'), null, 'Down on 0 items must return null');
    assert.equal(composer.stepHistory('up', 'wip draft'), null, 'Up with draft on 0 items must return null');
    assert.equal(composer.stepHistory('down'), null, 'Down after up on 0 items must return null');
    assert.equal(composer.getState().historyIndex, -1, 'historyIndex must remain -1 on empty history');
    assert.equal(composer.getState().savedDraft, null, 'savedDraft must remain null');
  }

  // Case 1.2: 1 item
  {
    const composer = new ComposerClass();
    composer.pushHistory('only item');
    assert.equal(composer.getState().history.length, 1);

    // Initial state: down returns null
    assert.equal(composer.stepHistory('down'), null, 'Down when historyIndex === -1 must return null');

    // Step up with draft
    const recalled1 = composer.stepHistory('up', 'uncommitted draft');
    assert.equal(recalled1, 'only item', 'Step up must recall only item');
    assert.equal(composer.getState().historyIndex, 0);
    assert.equal(composer.getState().savedDraft, 'uncommitted draft');

    // Step up again (boundary: clamping at index 0)
    const recalled2 = composer.stepHistory('up');
    assert.equal(recalled2, 'only item', 'Step up at top must remain clamped to only item');
    assert.equal(composer.getState().historyIndex, 0);
    assert.equal(composer.getState().savedDraft, 'uncommitted draft');

    // Step down: restores saved draft
    const restoredDraft = composer.stepHistory('down');
    assert.equal(restoredDraft, 'uncommitted draft', 'Step down from 0 must restore exact saved draft');
    assert.equal(composer.getState().historyIndex, -1, 'historyIndex must return to -1');
    assert.equal(composer.getState().savedDraft, null, 'savedDraft must be cleared to null');

    // Step down again: returns null
    assert.equal(composer.stepHistory('down'), null, 'Step down from -1 must return null');
  }

  // Case 1.3: 5 items
  {
    const composer = new ComposerClass();
    const prompts = ['prompt 1', 'prompt 2', 'prompt 3', 'prompt 4', 'prompt 5'];
    for (const p of prompts) {
      composer.pushHistory(p);
    }
    // Note: pushHistory unshifts, so history is: ['prompt 5', 'prompt 4', 'prompt 3', 'prompt 2', 'prompt 1']
    assert.equal(composer.getState().history.length, 5);

    const draft = 'my 5-item draft test';

    // Step Up 5 times
    assert.equal(composer.stepHistory('up', draft), 'prompt 5');
    assert.equal(composer.stepHistory('up'), 'prompt 4');
    assert.equal(composer.stepHistory('up'), 'prompt 3');
    assert.equal(composer.stepHistory('up'), 'prompt 2');
    assert.equal(composer.stepHistory('up'), 'prompt 1');

    // Boundary: step up beyond oldest item (clamp at index 4)
    assert.equal(composer.stepHistory('up'), 'prompt 1');
    assert.equal(composer.stepHistory('up'), 'prompt 1');
    assert.equal(composer.getState().historyIndex, 4);

    // Step Down 5 times
    assert.equal(composer.stepHistory('down'), 'prompt 2');
    assert.equal(composer.stepHistory('down'), 'prompt 3');
    assert.equal(composer.stepHistory('down'), 'prompt 4');
    assert.equal(composer.stepHistory('down'), 'prompt 5');
    assert.equal(composer.stepHistory('down'), draft, 'Step down back to -1 must restore original draft');
    assert.equal(composer.getState().historyIndex, -1);
    assert.equal(composer.getState().savedDraft, null);

    // Further down returns null
    assert.equal(composer.stepHistory('down'), null);
  }

  // Case 1.4: 20 items
  {
    const composer = new ComposerClass();
    for (let i = 0; i < 20; i++) {
      composer.pushHistory(`entry_${i}`);
    }
    assert.equal(composer.getState().history.length, 20);

    const draft = 'draft_at_20';

    // Forward recall: step up 20 times
    for (let i = 0; i < 20; i++) {
      const expected = `entry_${19 - i}`;
      const actual = i === 0 ? composer.stepHistory('up', draft) : composer.stepHistory('up');
      assert.equal(actual, expected, `Up step ${i} expected ${expected}, got ${actual}`);
      assert.equal(composer.getState().historyIndex, i);
    }

    // Boundary clamp: 10 extra up steps
    for (let j = 0; j < 10; j++) {
      assert.equal(composer.stepHistory('up'), 'entry_0');
      assert.equal(composer.getState().historyIndex, 19);
    }

    // Backward recall: step down 19 times to reach index 0
    for (let k = 18; k >= 0; k--) {
      const expected = `entry_${19 - k}`;
      const actual = composer.stepHistory('down');
      assert.equal(actual, expected, `Down step to index ${k} expected ${expected}, got ${actual}`);
      assert.equal(composer.getState().historyIndex, k);
    }

    // Final step down: restores draft
    const restored = composer.stepHistory('down');
    assert.equal(restored, draft);
    assert.equal(composer.getState().historyIndex, -1);
    assert.equal(composer.getState().savedDraft, null);
  }

  // Case 1.5: Push history deduplication and 50-item cap
  {
    const composer = new ComposerClass();
    composer.pushHistory('same prompt');
    composer.pushHistory('same prompt'); // Duplicate of top
    assert.equal(composer.getState().history.length, 1, 'Consecutive duplicate prompt must be deduplicated');

    // Push 70 distinct items
    for (let i = 0; i < 70; i++) {
      composer.pushHistory(`unique_${i}`);
    }
    assert.equal(composer.getState().history.length, 50, 'History stack must be capped at 50 items');
    assert.equal(composer.getState().history[0], 'unique_69', 'Most recent item must be at index 0');
    assert.equal(composer.getState().history[49], 'unique_20', 'Oldest retained item must be at index 49');
  }

  console.log('    ✓ History stepping passed all 0, 1, 5, 20 items & boundary tests.');

  // =========================================================================
  // SUITE 2: UNCOMMITTED DRAFT RESTORATION (Up 3x, Down 3x)
  // =========================================================================
  console.log('  Suite 2: Uncommitted Draft Restoration');

  {
    const composer = new ComposerClass();
    // Seed 5 past prompts
    composer.pushHistory('fix(wasm): align coordinate descent clamp with v8');
    composer.pushHistory('test(browser): verify 6-file offline closure');
    composer.pushHistory('feat(composer): add compact send bar and model selector');
    composer.pushHistory('docs(500b): calculate Chinchilla token budget');
    composer.pushHistory('refactor(otel): zero float narrowing in trace parse');

    const testDrafts = [
      'Draft with simple text',
      'Draft with <thinking> 256D Q12 coordinates </thinking> & special chars: !@#$%^&*()_+~`|}{[]\\:;?><,./',
      'Multi-line draft:\nLine 2\nLine 3 with tabs\t\tand emoji 🚀 🔥',
      '', // Empty string draft
      '   \n\t   ', // Whitespace-only draft
    ];

    for (const originalDraft of testDrafts) {
      assert.equal(composer.getState().historyIndex, -1);

      // 1. Press Up 3 times
      const recall1 = composer.stepHistory('up', originalDraft);
      assert.equal(recall1, 'refactor(otel): zero float narrowing in trace parse');
      assert.equal(composer.getState().historyIndex, 0);
      assert.equal(composer.getState().savedDraft, originalDraft);

      const recall2 = composer.stepHistory('up');
      assert.equal(recall2, 'docs(500b): calculate Chinchilla token budget');
      assert.equal(composer.getState().historyIndex, 1);
      assert.equal(composer.getState().savedDraft, originalDraft); // Must not overwrite!

      const recall3 = composer.stepHistory('up');
      assert.equal(recall3, 'feat(composer): add compact send bar and model selector');
      assert.equal(composer.getState().historyIndex, 2);
      assert.equal(composer.getState().savedDraft, originalDraft);

      // 2. Press Down 3 times
      const back2 = composer.stepHistory('down');
      assert.equal(back2, 'docs(500b): calculate Chinchilla token budget');
      assert.equal(composer.getState().historyIndex, 1);

      const back1 = composer.stepHistory('down');
      assert.equal(back1, 'refactor(otel): zero float narrowing in trace parse');
      assert.equal(composer.getState().historyIndex, 0);

      const restored = composer.stepHistory('down');
      assert.equal(restored, originalDraft, 'Exact original draft must be restored');
      assert.equal(composer.getState().historyIndex, -1);
      assert.equal(composer.getState().savedDraft, null);
    }
  }

  console.log('    ✓ Uncommitted draft restoration passed across all test payloads.');

  // =========================================================================
  // SUITE 3: RAPID MODEL SWITCHING (All 5 models)
  // =========================================================================
  console.log('  Suite 3: Rapid Model Switching Across All 5 Models');

  {
    const composer = new ComposerClass();
    const expectedModelIds = [
      'uor-r4-glm-v8',
      'claude-3-5-sonnet',
      'gpt-4o',
      'deepseek-r1',
      'uor-r4-500b',
    ];

    // Verify model inventory
    const registered = composer.getModelOptions();
    assert.equal(registered.length, 5, 'Must have exactly 5 models registered');
    for (let i = 0; i < 5; i++) {
      assert.equal(registered[i].id, expectedModelIds[i]);
      assert.ok(registered[i].name.length > 0, `Model ${expectedModelIds[i]} must have non-empty name`);
      assert.ok(registered[i].contextWindow.length > 0, `Model ${expectedModelIds[i]} must have contextWindow`);
      assert.ok(registered[i].badge.length > 0, `Model ${expectedModelIds[i]} must have badge`);
    }

    // Default model
    assert.equal(composer.getSelectedModel(), 'uor-r4-glm-v8');

    // 1,000 rapid switches in round-robin and random sequence
    for (let iter = 0; iter < 1000; iter++) {
      const targetModel = expectedModelIds[iter % 5];
      const ok = composer.setModel(targetModel);
      assert.equal(ok, true, `setModel(${targetModel}) must return true`);
      assert.equal(composer.getSelectedModel(), targetModel, `getSelectedModel() must match ${targetModel}`);
    }

    // Random shuffle 1,000 switches
    for (let iter = 0; iter < 1000; iter++) {
      const idx = Math.floor(Math.random() * 5);
      const targetModel = expectedModelIds[idx];
      const ok = composer.setModel(targetModel);
      assert.equal(ok, true);
      assert.equal(composer.getSelectedModel(), targetModel);
    }
  }

  console.log('    ✓ Rapid model switching passed 2,000 rapid cycles.');

  // =========================================================================
  // SUITE 4: FILE ATTACHMENT LIFECYCLE & DUPLICATE HANDLING
  // =========================================================================
  console.log('  Suite 4: File Attachment Lifecycle & Duplicate Handling');

  {
    const composer = new ComposerClass();
    assert.equal(composer.getState().attachments.length, 0);

    // 1. Attach file 1
    const ok1 = composer.attachFile({
      name: 'crates/wasm/src/otel.rs',
      size: 4096,
      type: 'text/rust',
      text: 'pub fn trace() {}',
    });
    assert.equal(ok1, true, 'Attaching first file must return true');
    assert.equal(composer.getState().attachments.length, 1);
    assert.equal(composer.getState().attachments[0].id, 'file:crates/wasm/src/otel.rs');
    assert.equal(composer.getState().attachments[0].label, 'crates/wasm/src/otel.rs');

    // 2. Attach duplicate of file 1
    const dup1 = composer.attachFile({
      name: 'crates/wasm/src/otel.rs',
      size: 4096,
      type: 'text/rust',
      text: 'pub fn trace() {}',
    });
    assert.equal(dup1, false, 'Attaching duplicate file must return false');
    assert.equal(composer.getState().attachments.length, 1, 'Attachments count must stay 1');

    // 3. Attach file 2
    const ok2 = composer.attachFile({
      name: 'crates/wasm/ts/composer.ts',
      size: 8192,
      type: 'text/typescript',
      text: 'export class AgenticTypeScriptComposer {}',
    });
    assert.equal(ok2, true, 'Attaching second file must return true');
    assert.equal(composer.getState().attachments.length, 2);

    // Badges
    const badges = composer.getAttachmentBadges();
    assert.deepEqual(badges, [
      '@crates/wasm/src/otel.rs',
      '@crates/wasm/ts/composer.ts',
    ]);

    // Token estimation: 2 attachments add 2 * 6 = 12 tokens
    const tokCount = composer.estimateTokenCount('Analyze these files');
    // "Analyze these files" = 3 words + 0 punctuation + 12 = 15
    assert.equal(tokCount, 15);

    // 4. Remove file 1
    composer.removeAttachment('file:crates/wasm/src/otel.rs');
    assert.equal(composer.getState().attachments.length, 1);
    assert.equal(composer.getState().attachments[0].id, 'file:crates/wasm/ts/composer.ts');

    // Remove non-existent attachment (no-op)
    composer.removeAttachment('file:does_not_exist.rs');
    assert.equal(composer.getState().attachments.length, 1);

    // 5. Re-add file 1
    const reAdd = composer.attachFile({
      name: 'crates/wasm/src/otel.rs',
      size: 4096,
      type: 'text/rust',
      text: 'pub fn trace() {}',
    });
    assert.equal(reAdd, true, 'Re-attaching previously removed file must return true');
    assert.equal(composer.getState().attachments.length, 2);

    // 6. Clear all attachments
    composer.clearAttachments();
    assert.equal(composer.getState().attachments.length, 0);
  }

  console.log('    ✓ File attachment lifecycle passed addition, duplicate rejection, and removal.\n');
}

// =========================================================================
// SUITE 5: DOM-LEVEL EVENT SIMULATION (Simulating public/app.js DOM handlers)
// =========================================================================
console.log('--- Testing Suite 5: DOM-Level Event Simulation ---');

class MockElement {
  constructor(tagName, id = '', className = '') {
    this.tagName = tagName.toUpperCase();
    this.id = id;
    this.className = className;
    this.value = '';
    this.textContent = '';
    this.attributes = new Map();
    this.children = [];
    this.listeners = new Map();
    this.style = {};
    this.selectionStart = 0;
    this.selectionEnd = 0;
    this.scrollHeight = 24;
  }

  setAttribute(k, v) { this.attributes.set(k, v); }
  removeAttribute(k) { this.attributes.delete(k); }
  hasAttribute(k) { return this.attributes.has(k); }
  getAttribute(k) { return this.attributes.get(k) ?? null; }

  appendChild(child) { this.children.push(child); }
  replaceChildren(...newChildren) { this.children = [...newChildren]; }

  addEventListener(type, cb) {
    if (!this.listeners.has(type)) this.listeners.set(type, []);
    this.listeners.get(type).push(cb);
  }

  dispatchEvent(event) {
    const list = this.listeners.get(event.type) || [];
    for (const cb of list) cb(event);
  }

  setSelectionRange(start, end) {
    this.selectionStart = start;
    this.selectionEnd = end;
  }

  focus() {}
  click() {
    this.dispatchEvent({ type: 'click', preventDefault: () => {} });
  }
}

// Setup simulated DOM environment
const inputEl = new MockElement('textarea', 'claude-chat-input');
const formEl = new MockElement('form', 'claude-chat-form');
const selectEl = new MockElement('select', 'composer-model-select');
const topbarTag = new MockElement('span', 'topbar-model-tag');
const tokenBadge = new MockElement('span', 'composer-token-badge');
const attachmentsBar = new MockElement('div', 'composer-attachments-bar');
attachmentsBar.setAttribute('hidden', '');

const domComposer = new AgenticTypeScriptComposer();

// Wire syncComposerModel
function syncComposerModel(modelId) {
  domComposer.setModel(modelId);
  if (selectEl.value !== modelId) {
    selectEl.value = modelId;
  }
  const opt = COMPOSER_MODELS.find((m) => m.id === modelId);
  if (opt) {
    topbarTag.textContent = `${opt.name} · ${opt.badge}`;
  }
}

selectEl.addEventListener('change', (e) => {
  syncComposerModel(e.target.value);
});

function updateComposerTokenBadge() {
  const tokCount = domComposer.estimateTokenCount(inputEl.value);
  tokenBadge.textContent = `${tokCount}t`;
}

function renderComposerAttachments() {
  attachmentsBar.replaceChildren();
  const chips = domComposer.getState().attachments;
  if (chips.length === 0) {
    attachmentsBar.setAttribute('hidden', '');
    updateComposerTokenBadge();
    return;
  }
  attachmentsBar.removeAttribute('hidden');
  for (const chip of chips) {
    const pill = new MockElement('span', '', `composer-chip composer-chip-${chip.kind}`);
    const labelSpan = new MockElement('span');
    labelSpan.textContent = `@${chip.label}`;
    const removeBtn = new MockElement('button', '', 'composer-chip-remove');
    removeBtn.textContent = '×';
    removeBtn.addEventListener('click', () => {
      domComposer.removeAttachment(chip.id);
      renderComposerAttachments();
    });
    pill.appendChild(labelSpan);
    pill.appendChild(removeBtn);
    attachmentsBar.appendChild(pill);
  }
  updateComposerTokenBadge();
}

// Wire Keydown handler exactly as in public/app.js lines 1985-2014
inputEl.addEventListener('keydown', (event) => {
  const st = domComposer.getState();
  if (event.key === 'ArrowUp') {
    const canStepUp =
      st.historyIndex >= 0 ||
      inputEl.selectionStart === 0 ||
      !inputEl.value.includes('\n');
    if (canStepUp) {
      const recalled = domComposer.stepHistory('up', inputEl.value);
      if (recalled !== null) {
        event.preventDefault();
        inputEl.value = recalled;
        inputEl.setSelectionRange(recalled.length, recalled.length);
        inputEl.style.height = '1.5rem';
        const newHeight = Math.min(Math.max(inputEl.scrollHeight, 24), 128);
        inputEl.style.height = `${newHeight}px`;
        updateComposerTokenBadge();
      }
    }
  } else if (event.key === 'ArrowDown' && st.historyIndex >= 0) {
    const recalled = domComposer.stepHistory('down');
    if (recalled !== null) {
      event.preventDefault();
      inputEl.value = recalled;
      inputEl.setSelectionRange(recalled.length, recalled.length);
      inputEl.style.height = '1.5rem';
      const newHeight = Math.min(Math.max(inputEl.scrollHeight, 24), 128);
      inputEl.style.height = `${newHeight}px`;
      updateComposerTokenBadge();
    }
  }
});

// Wire input listener
inputEl.addEventListener('input', () => {
  inputEl.style.height = '1.5rem';
  const newHeight = Math.min(Math.max(inputEl.scrollHeight, 24), 128);
  inputEl.style.height = `${newHeight}px`;
  updateComposerTokenBadge();
});

// TEST 5.1: Model selection change event
syncComposerModel('claude-3-5-sonnet');
assert.equal(domComposer.getSelectedModel(), 'claude-3-5-sonnet');
assert.equal(selectEl.value, 'claude-3-5-sonnet');
assert.equal(topbarTag.textContent, 'Claude 3.5 Sonnet · v1/messages');

syncComposerModel('uor-r4-500b');
assert.equal(domComposer.getSelectedModel(), 'uor-r4-500b');
assert.equal(selectEl.value, 'uor-r4-500b');
assert.equal(topbarTag.textContent, 'UOR-R4 500B Self-Improving Core · R3 Scale');
console.log('  ✓ DOM model select synchronization works as expected.');

// TEST 5.2: Textarea input and auto-expansion
inputEl.value = 'First line';
inputEl.scrollHeight = 24;
inputEl.dispatchEvent({ type: 'input' });
assert.equal(inputEl.style.height, '24px');
assert.equal(tokenBadge.textContent, '2t');

inputEl.value = 'Multi\nLine\nText\nExpanding';
inputEl.scrollHeight = 96;
inputEl.dispatchEvent({ type: 'input' });
assert.equal(inputEl.style.height, '96px');

inputEl.value = 'Massive\nText\nExceeding\nMax\nHeight\nBounds\nBeyond\n128px';
inputEl.scrollHeight = 250;
inputEl.dispatchEvent({ type: 'input' });
assert.equal(inputEl.style.height, '128px', 'Auto-expand must clamp at 128px (8rem)');
console.log('  ✓ DOM textarea input and auto-expansion bounds (24px - 128px) verified.');

// TEST 5.3: ArrowUp and ArrowDown event dispatch with draft preservation
domComposer.pushHistory('prev command 1');
domComposer.pushHistory('prev command 2');
domComposer.pushHistory('prev command 3');

inputEl.value = 'my active unsaved draft';
inputEl.selectionStart = inputEl.value.length;

// Press ArrowUp 1
let prevented = false;
inputEl.dispatchEvent({ key: 'ArrowUp', type: 'keydown', preventDefault: () => { prevented = true; } });
assert.equal(prevented, true);
assert.equal(inputEl.value, 'prev command 3');
assert.equal(inputEl.selectionStart, 'prev command 3'.length);

// Press ArrowUp 2
inputEl.dispatchEvent({ key: 'ArrowUp', type: 'keydown', preventDefault: () => {} });
assert.equal(inputEl.value, 'prev command 2');

// Press ArrowUp 3
inputEl.dispatchEvent({ key: 'ArrowUp', type: 'keydown', preventDefault: () => {} });
assert.equal(inputEl.value, 'prev command 1');

// Press ArrowDown 1
inputEl.dispatchEvent({ key: 'ArrowDown', type: 'keydown', preventDefault: () => {} });
assert.equal(inputEl.value, 'prev command 2');

// Press ArrowDown 2
inputEl.dispatchEvent({ key: 'ArrowDown', type: 'keydown', preventDefault: () => {} });
assert.equal(inputEl.value, 'prev command 3');

// Press ArrowDown 3 (Back to original draft!)
inputEl.dispatchEvent({ key: 'ArrowDown', type: 'keydown', preventDefault: () => {} });
assert.equal(inputEl.value, 'my active unsaved draft', 'Original draft must be restored on final ArrowDown');
assert.equal(domComposer.getState().historyIndex, -1);
console.log('  ✓ DOM keydown ArrowUp / ArrowDown navigation and draft restoration verified.');

// TEST 5.4: Attachment Chip Rendering & Removal Button Click
domComposer.attachFile({ name: 'test.rs', size: 100, type: 'text/rust', text: 'code' });
renderComposerAttachments();
assert.equal(attachmentsBar.hasAttribute('hidden'), false);
assert.equal(attachmentsBar.children.length, 1);
assert.equal(attachmentsBar.children[0].children[0].textContent, '@test.rs');

// Click the remove button
const removeBtn = attachmentsBar.children[0].children[1];
assert.equal(removeBtn.textContent, '×');
removeBtn.click();
assert.equal(domComposer.getState().attachments.length, 0);
assert.equal(attachmentsBar.hasAttribute('hidden'), true);
console.log('  ✓ DOM attachment chip rendering and click-to-remove verified.');

console.log('\n=== ALL EMPIRICAL STRESS TESTS PASSED WITH ZERO FAILURES ===');
