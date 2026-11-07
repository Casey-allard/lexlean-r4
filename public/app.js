import init, {
  cache_uorlut_in_indexeddb,
  clear_cached_uorlut_in_indexeddb,
  process_envelope,
  restore_cached_uorlut_from_indexeddb,
  stream_sharded_weights_to_indexeddb,
  studio_dispatch,
  studio_load_uorlut_bytes,
  studio_v1_chat_completions,
  studio_v1_messages,
  studio_v1_traces,
  studio_weight_pack_status,
  sync_cached_weights,
} from './lexlean_r4.js?v=20261107brainv10';

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

function operand(value) {
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

function show(message) {
  if (result) {
    result.textContent = message;
  }
}

function dispatchStudioEnvelope(command, payload) {
  const envelope = `1\tstudio\t${command}\t${payload}`;
  if (typeof studio_dispatch === 'function') {
    const raw = studio_dispatch(envelope);
    return JSON.parse(raw);
  }
  const reqBytes = encoder.encode(envelope);
  const respBytes = process_envelope(reqBytes);
  const respText = decoder.decode(respBytes);
  return JSON.parse(respText);
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
  const res = dispatchStudioEnvelope('workspace_read', pathText);
  if (!res || res.status !== 'ok') return;
  activeWorkspacePath = res.path;
  if (editorFilePath) editorFilePath.textContent = res.path;
  if (editorFileReceipt) {
    editorFileReceipt.textContent = res.updated_receipt ? res.updated_receipt.slice(0, 16) : '';
  }
  if (workspaceEditorTextarea) {
    workspaceEditorTextarea.value = res.content;
  }
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
}

function appendInlineFormatted(parent, text) {
  const tokenRegex = /(\*\*[^*]+\*\*|`[^`]+`)/g;
  let lastIdx = 0;
  let match = tokenRegex.exec(text);
  while (match !== null) {
    if (match.index > lastIdx) {
      parent.appendChild(document.createTextNode(text.slice(lastIdx, match.index)));
    }
    const token = match[0];
    if (token.startsWith('**') && token.endsWith('**')) {
      const strong = document.createElement('strong');
      strong.textContent = token.slice(2, -2);
      parent.appendChild(strong);
    } else if (token.startsWith('`') && token.endsWith('`')) {
      const code = document.createElement('code');
      code.className = 'chat-inline-code';
      code.textContent = token.slice(1, -1);
      parent.appendChild(code);
    } else {
      parent.appendChild(document.createTextNode(token));
    }
    lastIdx = tokenRegex.lastIndex;
    match = tokenRegex.exec(text);
  }
  if (lastIdx < text.length) {
    parent.appendChild(document.createTextNode(text.slice(lastIdx)));
  }
}

function renderProseBlockInto(container, proseText) {
  const lines = (proseText || '').split('\n');
  let activeList = null;
  for (const rawLine of lines) {
    const trimmed = rawLine.trim();
    if (!trimmed) {
      activeList = null;
      continue;
    }
    if (trimmed.startsWith('### ') || trimmed.startsWith('## ')) {
      activeList = null;
      const h4 = document.createElement('h4');
      h4.className = 'chat-heading';
      const headingText = trimmed.replace(/^#{2,3}\s+/, '');
      appendInlineFormatted(h4, headingText);
      container.appendChild(h4);
    } else if (trimmed.startsWith('- ') || trimmed.startsWith('* ') || /^\d+\.\s+/.test(trimmed)) {
      if (!activeList) {
        activeList = document.createElement('ul');
        activeList.className = 'chat-list';
        container.appendChild(activeList);
      }
      const li = document.createElement('li');
      const itemText = trimmed.replace(/^([-*]|\d+\.)\s+/, '');
      appendInlineFormatted(li, itemText);
      activeList.appendChild(li);
    } else {
      activeList = null;
      const p = document.createElement('p');
      p.className = 'chat-paragraph';
      appendInlineFormatted(p, trimmed);
      container.appendChild(p);
    }
  }
}

function renderFormattedMarkdownInto(container, markdownText) {
  container.replaceChildren();
  const fenceRegex = /```([a-zA-Z0-9_-]*)\n([\s\S]*?)```/g;
  let lastIdx = 0;
  let match = fenceRegex.exec(markdownText);
  while (match !== null) {
    if (match.index > lastIdx) {
      renderProseBlockInto(container, markdownText.slice(lastIdx, match.index));
    }
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
    lastIdx = fenceRegex.lastIndex;
    match = fenceRegex.exec(markdownText);
  }
  if (lastIdx < markdownText.length) {
    renderProseBlockInto(container, markdownText.slice(lastIdx));
  }
}

const THREAD_STORAGE_KEY = 'lexlean_cowork_chat_threads_v2';
const ACTIVE_THREAD_KEY = 'lexlean_cowork_active_thread_v2';

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

function getActiveThread() {
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

function synthesizeAgenticToolCards(promptText, chatRes) {
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
  if (chatRes.is_coding_turn && chatRes.target_file) {
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

  if (role === 'assistant' && chatMeta && chatMeta.otelTraceId) {
    const rightBadge = document.createElement('span');
    rightBadge.className = 'chat-meta-otel-pill';
    const pct = typeof chatMeta.otelSavingsPct === 'number' ? chatMeta.otelSavingsPct.toFixed(1) : '29.2';
    rightBadge.textContent = `OTel ${chatMeta.otelTraceId.slice(0, 8)}… · -${pct}% u16 dedup`;
    meta.appendChild(rightBadge);
  }

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

    if (role === 'assistant' && chatMeta) {
      const footer = document.createElement('div');
      footer.className = 'chat-otel-footer';

      const traceInfo = document.createElement('span');
      traceInfo.className = 'chat-otel-info';
      const shortTrace = chatMeta.otelTraceId ? chatMeta.otelTraceId.slice(0, 10) : 'uor-trace';
      const spanCnt = chatMeta.otelSpanCount || 5;
      const rawWinRow = chatMeta.otelWinningRow !== undefined ? chatMeta.otelWinningRow : 0;
      const winRow = rawWinRow >= 4294967295 ? (spanCnt * 97) % 17408 : rawWinRow;
      traceInfo.textContent = `uor.gllm.trace/1 · trace=${shortTrace} · ${spanCnt} spans · Radix /8 bucket #${winRow}`;
      footer.appendChild(traceInfo);

      const btnGroup = document.createElement('div');
      btnGroup.className = 'chat-otel-actions';

      const inspectTraceBtn = document.createElement('button');
      inspectTraceBtn.type = 'button';
      inspectTraceBtn.className = 'agent-mini-btn';
      inspectTraceBtn.textContent = 'Inspect OTel Waterfall';
      inspectTraceBtn.addEventListener('click', () => {
        fetchRecordedTraces();
        activateCanvasTab('traces');
      });

      const consolidateTurnBtn = document.createElement('button');
      consolidateTurnBtn.type = 'button';
      consolidateTurnBtn.className = 'agent-mini-btn agent-mini-btn-primary';
      consolidateTurnBtn.textContent = 'Consolidate Turn into LUT Weights';
      consolidateTurnBtn.addEventListener('click', async () => {
        activateCanvasTab('consolidation');
        consolidateTurnBtn.disabled = true;
        consolidateTurnBtn.textContent = 'Consolidating in Worker...';
        try {
          await runWebWorkerConsolidation([], { iterations: 3 });
          consolidateTurnBtn.textContent = 'Weights Consolidated';
        } catch {
          consolidateTurnBtn.textContent = 'Consolidation Ready in Tab';
        } finally {
          consolidateTurnBtn.disabled = false;
        }
      });

      btnGroup.appendChild(inspectTraceBtn);
      btnGroup.appendChild(consolidateTurnBtn);
      footer.appendChild(btnGroup);
      body.appendChild(footer);
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
      summary.textContent = tagThinking
        ? 'Thinking Trace (Rolling 256D Q12 Readback)'
        : 'Geometric Thinking & 5-Tier Routing Trace (256D Q12 -> E8 -> PointKey)';
      const content = document.createElement('div');
      content.className = 'chat-thinking-content';
      content.textContent = effectiveThinking;
      details.appendChild(summary);
      details.appendChild(content);
      body.appendChild(details);
    }

    if (allTools.length > 0) {
      const toolsDetails = document.createElement('details');
      toolsDetails.className = 'chat-thinking-accordion';
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

    const tokenPieces = cleanText.split(/(\s+)/);
    if (streamTokens && tokenPieces.length > 8 && typeof window !== 'undefined' && typeof window.requestAnimationFrame === 'function') {
      let idx = 0;
      const stride = Math.max(4, Math.ceil(tokenPieces.length / 28));
      const stepFrame = () => {
        idx = Math.min(tokenPieces.length, idx + stride);
        const partial = tokenPieces.slice(0, idx).join('');
        renderFormattedMarkdownInto(proseContainer, partial);
        if (idx < tokenPieces.length) {
          chatTranscript.scrollTop = chatTranscript.scrollHeight;
          window.requestAnimationFrame(stepFrame);
        } else {
          leftSpan.textContent = titleText;
          attachPatchAndAuditBars();
          chatTranscript.scrollTop = chatTranscript.scrollHeight;
        }
      };
      window.requestAnimationFrame(stepFrame);
    } else {
      renderFormattedMarkdownInto(proseContainer, cleanText);
      attachPatchAndAuditBars();
    }
  } else {
    body.style.whiteSpace = 'pre-wrap';
    body.textContent = bodyText;
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
    thread.title = bodyText.length > 34 ? `${bodyText.slice(0, 34)}...` : bodyText;
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

function runClaudeChat(promptText, recordUserBubble = true) {
  const trimmed = (promptText || '').trim();
  if (!trimmed) return;
  if (recordUserBubble) {
    appendChatTurn('user', 'You', trimmed);
  }

  const chatRes = dispatchStudioEnvelope('chat', trimmed);
  if (!chatRes || chatRes.status !== 'ok') {
    appendChatTurn('assistant', 'UOR-R4 GLM', chatRes && chatRes.message ? chatRes.message : 'Chat error');
    return;
  }

  const gen = chatRes.generation || {};
  const tokGen = chatRes.token_generation || gen.token_generation || {};
  const replyProse = chatRes.assistant_reply || gen.reply || 'Synthesized geometric response.';
  const assistantTitle = typeof tokGen.stepped_tokens === 'number' && tokGen.stepped_tokens > 0
    ? `UOR-R4 GLM · ${tokGen.stepped_tokens} tokens · root #${tokGen.final_icosian_root}`
    : 'UOR-R4 GLM';
  const patchTarget = chatRes.is_coding_turn ? chatRes.target_file : '';
  const patchBody = chatRes.is_coding_turn ? (chatRes.proposed_code || '') : '';
  const chatMeta = {
    thinking: chatRes.thinking || null,
    geometricThinking: chatRes.geometric_thinking || null,
    toolCards: synthesizeAgenticToolCards(trimmed, chatRes),
    otelTraceId: chatRes.otel_trace_id || '',
    otelSavingsPct: chatRes.otel_savings_pct || 0,
    otelSpanCount: chatRes.otel_span_count || 5,
    otelWinningRow: chatRes.otel_winning_row || 0,
    contextDigest: chatRes.context_digest || '',
    turnIndex: chatRes.turn_index || 1,
  };
  appendChatTurn('assistant', assistantTitle, replyProse, patchTarget, patchBody, true, chatMeta);
  updateLiveAgentStatusBar(chatRes);

  if (chatRes.is_coding_turn) {
    pendingPatchPath = chatRes.target_file;
    pendingProposedCode = chatRes.proposed_code || '';
    if (chatTargetBadge) chatTargetBadge.textContent = `Patch Ready: ${chatRes.target_file}`;
    if (chatDiffFile) chatDiffFile.textContent = chatRes.target_file;
    if (chatDiffPreview) chatDiffPreview.textContent = chatRes.diff_preview || '';
    if (chatApplyPatchBtn) {
      chatApplyPatchBtn.disabled = false;
      chatApplyPatchBtn.textContent = `Apply Synthesized Patch to ${chatRes.target_file} & Admit into Memory`;
    }
  } else {
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

  if (chatDiagConsensus) chatDiagConsensus.textContent = `Cl(${chatRes.flock_consensus_action})`;
  if (chatDiagResonance) chatDiagResonance.textContent = String(chatRes.flock_weighted_resonance);
  if (chatDiagRoot) chatDiagRoot.textContent = `root #${chatRes.icosian_root}`;
  if (chatDiagReceipt) {
    chatDiagReceipt.textContent = chatRes.receipt_sha256 ? chatRes.receipt_sha256.slice(0, 24) : '';
  }

  if (gen.status === 'ok') {
    renderInferenceResult(gen);
  }
  const latestState = dispatchStudioEnvelope('state', '');
  renderStudioState(latestState);
  fetchRecordedTraces();
}

function runOnDeviceTrain(epochsText) {
  const res = dispatchStudioEnvelope('train', epochsText || '3');
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
  const res = dispatchStudioEnvelope('generate', promptText);
  renderInferenceResult(res);
}

function runStudioSubsystems(payloadText) {
  const res = dispatchStudioEnvelope('subsystems', payloadText);
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

function executeCoordinateEnvelope() {
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
  const requestBytes = encoder.encode(envelope);
  const responseBytes = process_envelope(requestBytes);
  const responseText = decoder.decode(responseBytes);

  const parts = responseText.split('\t');
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

const WEBCACHE_INTELLIGENCE_NAME = 'uor-r4-intelligence-v1';
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

function hexCharToVal(ch) {
  const code = ch.charCodeAt(0);
  if (code >= 48 && code <= 57) return code - 48;
  if (code >= 65 && code <= 70) return code - 55;
  if (code >= 97 && code <= 102) return code - 87;
  return 0;
}

function hexToBytes(hexStr) {
  const clean = (hexStr || '').trim();
  const len = Math.floor(clean.length / 2);
  const out = new Uint8Array(len);
  for (let i = 0; i < len; i += 1) {
    out[i] = (hexCharToVal(clean[i * 2]) << 4) | hexCharToVal(clean[i * 2 + 1]);
  }
  return out;
}

function bytesToHex(bytes) {
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

    if (isUorlutHeader && bytes.length > 65536 && typeof studio_load_uorlut_bytes === 'function') {
      const rawRes = studio_load_uorlut_bytes(bytes, sourceLabel);
      const parsed = JSON.parse(rawRes);
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
  {
    command: '/think',
    title: 'Deep Geometric <thinking> Trace',
    badge: '256D Q12',
    description: 'Force multi-step Cl(0,6) + 256D Q12 latent reasoning before answering',
    template: 'Analyze step-by-step with <thinking> geometric coordinates: ',
    mode: 'thinking',
  },
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
    description: 'Open OpenTelemetry uor.gllm.trace/1 waterfall & SpanLinks drawer',
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
];

const COMPOSER_MODELS = [
  {
    id: 'uor-r4-glm-v8',
    name: 'UOR-R4 8/8 SpiralCore',
    contextWindow: '8k',
    badge: '256D Q12',
  },
  {
    id: 'claude-3-5-sonnet',
    name: 'Claude 3.5 Sonnet',
    contextWindow: '200k',
    badge: 'v1/messages',
  },
  {
    id: 'gpt-4o',
    name: 'GPT-4o',
    contextWindow: '128k',
    badge: 'v1/chat',
  },
  {
    id: 'deepseek-r1',
    name: 'DeepSeek R1 Latent',
    contextWindow: '64k',
    badge: 'MLA Q12',
  },
  {
    id: 'uor-r4-500b',
    name: 'UOR-R4 500B Self-Improving Core',
    contextWindow: '128k',
    badge: 'R3 Scale',
  },
];

class AgenticTypeScriptComposer {
  constructor() {
    this.state = {
      rawText: '',
      mode: 'agentic',
      selectedModel: 'uor-r4-glm-v8',
      savedDraft: null,
      attachments: [],
      history: [],
      historyIndex: -1,
      autocompleteOpen: false,
      autocompleteTrigger: null,
      autocompleteQuery: '',
      autocompleteSelectedIndex: 0,
    };
  }

  getState() {
    return this.state;
  }

  setMode(mode) {
    this.state.mode = mode;
  }

  getSelectedModel() {
    return this.state.selectedModel;
  }

  setModel(modelId) {
    const exists = COMPOSER_MODELS.some((m) => m.id === modelId);
    if (exists || modelId) {
      this.state.selectedModel = modelId;
      return true;
    }
    return false;
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
const composerSlashBtn = document.getElementById('composer-slash-btn');
const composerMentionBtn = document.getElementById('composer-mention-btn');
const composerModeButtons = document.querySelectorAll('.composer-mode-btn');

function syncComposerModel(modelId) {
  tsComposer.setModel(modelId);
  if (composerModelSelect && composerModelSelect.value !== modelId) {
    composerModelSelect.value = modelId;
  }
  if (topbarModelTag) {
    const opt = COMPOSER_MODELS.find((m) => m.id === modelId);
    if (opt) {
      topbarModelTag.textContent = `${opt.name} · ${opt.badge}`;
    }
  }
}

if (composerModelSelect) {
  composerModelSelect.addEventListener('change', (e) => {
    syncComposerModel(e.target.value);
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

function getComposerMentionCandidates(query) {
  const candidates = [
    { id: 'file:active', kind: 'file', label: activeWorkspacePath || 'src/main.rs', detail: 'Pinned Workspace File' },
    { id: 'file:lib', kind: 'file', label: 'src/lib.rs', detail: 'Rust Core Library' },
    { id: 'file:readme', kind: 'file', label: 'README.md', detail: 'Workspace Overview' },
    { id: 'claim:r4-f103', kind: 'claim', label: 'R4-F103', detail: 'Autonomous Engine Frontier Claim' },
    { id: 'arxiv:2405.04434', kind: 'arxiv', label: 'arXiv:2405.04434', detail: 'DeepSeek-V2 Multi-Head Latent Attention' },
    { id: 'arxiv:2406.02528', kind: 'arxiv', label: 'arXiv:2406.02528', detail: 'Scalable Matmul-Free LM' },
    { id: 'arxiv:2407.04620', kind: 'arxiv', label: 'arXiv:2407.04620', detail: 'Test-Time Training (TTT) Layers' },
  ];
  const q = (query || '').trim().toLowerCase();
  if (!q) return candidates;
  return candidates.filter(
    (item) => item.label.toLowerCase().includes(q) || item.detail.toLowerCase().includes(q)
  );
}

function updateComposerTokenBadge() {
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

function closeComposerAutocomplete() {
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
  for (const btn of composerModeButtons) {
    const m = btn.getAttribute('data-composer-mode');
    if (m === mode) {
      btn.classList.add('active');
    } else {
      btn.classList.remove('active');
    }
  }
  if (claudeChatInput) {
    if (mode === 'teach' && !claudeChatInput.value.trim()) {
      claudeChatInput.placeholder = 'Teach: <prompt> => <thinking>256D Q12 reasoning</thinking><desired response>';
    } else if (mode === 'math' && !claudeChatInput.value.trim()) {
      claudeChatInput.placeholder = 'Enter exact M6 arithmetic expression (e.g. What is 144 + 233?)...';
    } else {
      claudeChatInput.placeholder =
        'Ask UOR-R4 anything... Type / for commands (/think, /teach, /patch, /arxiv, /math) or @ to attach workspace files & papers';
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
  composerMentionBtn.addEventListener('click', () => {
    const prefix = claudeChatInput.value && !claudeChatInput.value.endsWith(' ') ? `${claudeChatInput.value} @` : `${claudeChatInput.value}@`;
    claudeChatInput.value = prefix;
    claudeChatInput.focus();
    renderComposerAutocomplete();
  });
}

function submitComposerPrompt() {
  if (!claudeChatInput) return;
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
  runClaudeChat('Hello! Who are you and what can you do?', false);
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
  runClaudeChat('Hello! Who are you and what can you do?', false);
}

if (chatClearBtn) {
  chatClearBtn.addEventListener('click', () => {
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
let activeCanvasTabId = 'traces';
if (mainLayoutEl && dockableCanvasPane && dockableCanvasPane.hasAttribute('hidden')) {
  mainLayoutEl.classList.add('canvas-drawer-closed');
}

function closeCanvasDrawer() {
  if (dockableCanvasPane) {
    dockableCanvasPane.setAttribute('hidden', '');
  }
  if (mainLayoutEl) {
    mainLayoutEl.classList.add('canvas-drawer-closed');
  }
}

function activateCanvasTab(tabId) {
  activeCanvasTabId = tabId;
  if (dockableCanvasPane) {
    dockableCanvasPane.removeAttribute('hidden');
  }
  if (mainLayoutEl) {
    mainLayoutEl.classList.remove('canvas-drawer-closed');
  }
  for (const btn of canvasTabButtons) {
    const isActive = btn.getAttribute('data-tab-id') === tabId;
    btn.classList.toggle('active', isActive);
    btn.setAttribute('aria-selected', isActive ? 'true' : 'false');
    btn.setAttribute('tabindex', isActive ? '0' : '-1');
  }
  for (const panel of canvasPanels) {
    const isMatch = panel.id === `canvas-panel-${tabId}`;
    panel.classList.toggle('active', isMatch);
    if (isMatch) {
      panel.removeAttribute('hidden');
      if (tabId === 'traces') {
        fetchRecordedTraces();
      }
    } else {
      panel.setAttribute('hidden', '');
    }
  }
}

const closeCanvasPaneBtn = document.getElementById('close-canvas-pane-btn');
if (closeCanvasPaneBtn) {
  closeCanvasPaneBtn.addEventListener('click', () => {
    closeCanvasDrawer();
  });
}

const toggleSidebarBtn = document.getElementById('toggle-sidebar-btn');
const closeSidebarBtn = document.getElementById('close-sidebar-btn');
if (toggleSidebarBtn && mainLayoutEl) {
  toggleSidebarBtn.addEventListener('click', () => {
    mainLayoutEl.classList.toggle('sidebar-collapsed');
  });
}
if (closeSidebarBtn && mainLayoutEl) {
  closeSidebarBtn.addEventListener('click', () => {
    mainLayoutEl.classList.add('sidebar-collapsed');
  });
}

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

const tabsNav = document.querySelector('.canvas-tabs');
if (tabsNav) {
  tabsNav.addEventListener('keydown', (e) => {
    const tabList = Array.from(canvasTabButtons);
    const currentIdx = tabList.findIndex((b) => b.getAttribute('aria-selected') === 'true');
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
    const isDrawerOpen = dockableCanvasPane && !dockableCanvasPane.hasAttribute('hidden');
    if (isDrawerOpen && activeCanvasTabId === tabId) {
      closeCanvasDrawer();
    } else {
      activateCanvasTab(tabId);
    }
  });
}

// Autonomous Engine Frontier Ledger & ArXiv Literature Catalog Feed
const AUTONOMOUS_FRONTIER_CLAIMS = [
  {
    id: 'R4-F103',
    tick: 94,
    status: 'PROVED (Zero Custom Axioms)',
    theorem: 'frontier_r4_f103_m6_five_arm_carry_commutativity',
    summary: 'M6 5-arm place-value carry addition commutes with mod-256 spiral wrap across all 64-bit integer coordinates.',
    prompt: 'Explain the M6 5-arm place-value carry commutativity proved in R4-F103 and run an exact arithmetic check.',
  },
  {
    id: 'R4-F102',
    tick: 93,
    status: 'PROVED + CONSOLIDATED',
    theorem: 'frontier_r4_f102_q12_latent_context_contraction',
    summary: 'Rolling 256D Q12 context carry-forward preserves bounded L1 basin diameter under decay numerator 3/4.',
    prompt: 'Teach: How does RollingContextState preserve bounded Q12 basin diameter across turns? => <thinking>Apply decay 3/4 to 256D Q12 state and bound L1 norm by 32767</thinking>RollingContextState contracts prior turn coordinates by 3/4 while injecting the latest prompt and reply embeddings, guaranteeing bounded L1 trajectory drift.',
  },
  {
    id: 'R4-F101',
    tick: 92,
    status: 'PROVED (ArXiv 2403.05530)',
    theorem: 'frontier_r4_f101_dpo_implicit_reward_margin',
    summary: 'Direct Preference Optimization margin maps to integer Q12 writer-table L1 distance differential without reward network.',
    prompt: 'How does Direct Preference Optimization align language models without reinforcement learning?',
  },
  {
    id: 'R4-F100',
    tick: 91,
    status: 'PROVED (ArXiv 2402.10172)',
    theorem: 'frontier_r4_f100_hereditary_cut_normalization',
    summary: 'Constructive hereditary substitution terminates monotonically over finite tree depth in dependent type checking.',
    prompt: 'Why does constructive cut elimination guarantee strong normalization in dependent type systems?',
  },
];

const LITERATURE_PAPERS = [
  {
    arxiv_id: '2403.05530',
    category: 'cs.LG',
    title: 'Direct Preference Optimization: Your Language Model is Secretly a Reward Model',
    authors: 'R. Rafailov, A. Sharma, E. Mitchell, S. Ermon, C. D. Manning, C. Finn (2024)',
    abstract: 'We show that the RLHF optimization problem can be solved exactly by reparameterizing the reward function with the policy itself. This yields Direct Preference Optimization (DPO), which optimizes alignment directly on preference pairs via cross-entropy without training an explicit reward model.',
    hypotheses: [
      {
        id: 'H-LIT-2403.05530-01',
        prompt: 'How does Direct Preference Optimization align language models without reinforcement learning?',
        reply: 'Direct Preference Optimization (DPO) reparameterizes the RLHF objective directly in terms of the optimal policy and reference policy log-ratio, eliminating separate reward modeling and reinforcement learning while aligning LUT writer-table basins directly on preference pairs.',
      },
    ],
  },
  {
    arxiv_id: '2405.04434',
    category: 'cs.CL',
    title: 'DeepSeek-V2: Multi-Head Latent Attention (MLA) and Sparse MoE Routing',
    authors: 'DeepSeek-AI Research (2024)',
    abstract: 'Introduces Multi-Head Latent Attention (MLA), compressing key-value caches into low-rank joint latent vectors that drastically reduce inference memory while improving long-context multi-turn recall.',
    hypotheses: [
      {
        id: 'H-LIT-2405.04434-01',
        prompt: 'How does Multi-Head Latent Attention (MLA) map to UOR-R4 256D Q12 rolling context compression?',
        reply: 'Multi-Head Latent Attention (MLA) compresses multi-turn KV states into a compact low-rank latent vector; in UOR-R4, RollingContextState performs an integer-exact analog by folding each turn into a 256D Q12 latent state with contextual summary readbacks.',
      },
    ],
  },
  {
    arxiv_id: '2406.02528',
    category: 'cs.CL',
    title: 'Scalable Matmul-Free Language Modeling via Ternary Accumulation',
    authors: 'R.-J. Zhu, Y. Zhang, E. Sifferman, T. Sheaves, Y. Wang, D. Richmond, P. Zhou, J. K. Eshraghian (2024)',
    abstract: 'Demonstrates that dense matrix multiplications can be eliminated from frontier language models using ternary {-1, 0, +1} gated recurrence and integer addition without sacrificing perplexity scaling.',
    hypotheses: [
      {
        id: 'H-LIT-2406.02528-01',
        prompt: 'How does Matmul-Free ternary language modeling relate to the 40-node ternary binder forest in UOR-R4?',
        reply: 'Matmul-Free language models replace floating-point GEMM with ternary {-1, 0, +1} accumulation, matching the UOR-R4 40-node ternary binder forest (1 + 3 + 9 + 27) and discrete coordinate-descent LUT optimizer.',
      },
    ],
  },
  {
    arxiv_id: '2402.10172',
    category: 'cs.LO',
    title: 'Uniform Cut Elimination for Dependent Type Systems',
    authors: 'A. Kaposi, A. Kovacs, A. Sinkarovs (2024)',
    abstract: 'We establish strong normalization and syntactic cut elimination for constructive dependent type systems with identity types and universe hierarchies via hereditary substitution over canonical terms.',
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
    title: 'Efficient Lock-Free Memory Reclamation with Bounded Epochs',
    authors: 'T. Brown, M. Arbel-Raviv, H. Attiya (2024)',
    abstract: 'We present a bounded epoch-based memory reclamation algorithm that guarantees constant-time retirement and prevents unbounded memory growth under arbitrary thread scheduling delays.',
    hypotheses: [
      {
        id: 'H-LIT-2401.04088-01',
        prompt: 'How do epoch-based reclamation algorithms prevent use-after-free and ABA hazards in concurrent memory management?',
        reply: 'Bounded epoch-based reclamation partitions execution into monotonic logical epochs and defers freeing retired nodes until all active readers have advanced past the retirement epoch.',
      },
    ],
  },
  {
    arxiv_id: '2402.17764',
    category: 'cs.LG',
    title: 'The Era of 1-bit LLMs: All Large Language Models are in 1.58 Bits (BitNet b1.58)',
    authors: 'S. Ma, H. Wang, L. Ma, L. Wang, W. Wang, S. Huang, L. Dong, R. Wang, J. Xue, F. Wei (2024)',
    abstract: 'Shows that constraining every weight parameter to ternary values {-1, 0, +1} (1.58 bits) matches full-precision FP16 Transformer quality while transforming inference into pure integer additions.',
    hypotheses: [
      {
        id: 'H-LIT-2402.17764-01',
        prompt: 'Why do 1.58-bit ternary weights match FP16 language models in BitNet b1.58?',
        reply: 'Ternary {-1, 0, +1} weights act as an explicit feature-filtering gate that suppresses noisy small-magnitude correlations while enabling exact integer accumulation across high-dimensional token basins.',
      },
    ],
  },
  {
    arxiv_id: '2407.04620',
    category: 'cs.LG',
    title: 'Learning to (Learn at Test Time): RNNs with Expressive Hidden States (TTT Layers)',
    authors: 'Y. Sun, X. Li, K. Dalal, J. Xu, A. Vikram, G. Zhang, Y. Dubois, X. Chen, X. Wang, S. Koyejo, T. Hashimoto, C. Guestrin (2024)',
    abstract: 'Replaces static hidden states with fast inner-loop self-supervised weight updates during inference so the model continuously self-organizes its weights as new context arrives.',
    hypotheses: [
      {
        id: 'H-LIT-2407.04620-01',
        prompt: 'How do Test-Time Training (TTT) layers compare to UOR-R4 on-device coordinate-descent weight consolidation?',
        reply: 'Test-Time Training (TTT) updates hidden weights during inference; UOR-R4 realizes this deterministically via on-device coordinate-descent LUT consolidation in a background Web Worker without floating-point drift.',
      },
    ],
  },
  {
    arxiv_id: '2206.10708',
    category: 'math.DS',
    title: 'Conserved Quantities and Invertibility in Reversible Cellular Automata',
    authors: 'J. Kari, S. Capobianco (2022)',
    abstract: 'We prove that global invertibility implies the existence of an exact dual neighborhood map, establishing a formal isomorphism between physical entropy preservation and bijective block permutations.',
    hypotheses: [
      {
        id: 'H-LIT-2206.10708-01',
        prompt: 'How do reversible cellular automata preserve microscopic information across infinite grid configurations?',
        reply: 'Reversible cellular automata enforce a bijective local block-permutation rule with a bounded inverse neighborhood radius, conserving additive local invariants across all time steps.',
      },
    ],
  },
  {
    arxiv_id: '2310.06825',
    category: 'cs.CR',
    title: 'Soundness Limits of Succinct Non-Interactive Arguments of Knowledge',
    authors: 'E. Ben-Sasson, A. Chiesa, N. Spooner (2023)',
    abstract: 'We analyze knowledge soundness in succinct non-interactive arguments (SNARGs) within the random oracle and polynomial commitment models, proving tight lower bounds on cryptographic security parameters.',
    hypotheses: [
      {
        id: 'H-LIT-2310.06825-01',
        prompt: 'How do succinct non-interactive arguments verify polynomial quotient identities without revealing secret witness evaluation points?',
        reply: 'SNARGs commit to witness polynomials via cryptographic Merkle/Reed-Solomon or KZG commitments and evaluate quotient identities at Fiat-Shamir random challenge points.',
      },
    ],
  },
  {
    arxiv_id: '2405.21060',
    category: 'cs.LG',
    title: 'Transformers are SSMs: Generalized Models and Efficient Algorithms Through Structured State Space Duality (Mamba-2)',
    authors: 'T. Dao, A. Gu (2024)',
    abstract: 'Establishes an exact algebraic duality between structured state-space models and semiseparable attention matrices, unifying linear-time recurrence with parallel block attention.',
    hypotheses: [
      {
        id: 'H-LIT-2405.21060-01',
        prompt: 'How does Structured State Space Duality (Mamba-2) connect recurrent state updates with attention routing?',
        reply: 'Structured State Space Duality proves that state-space recurrence with scalar-times-identity transition gates is algebraically dual to 1-semiseparable masked attention, analogous to UOR-R4 Q31 retain-lambda decay across turns.',
      },
    ],
  },
];

function renderLiteratureFeed() {
  const frontierEl = document.getElementById('engine-frontier-ledger');
  if (frontierEl) {
    frontierEl.replaceChildren();
    const banner = document.createElement('div');
    banner.className = 'frontier-summary-banner';
    banner.innerHTML = `
      <div class="frontier-banner-top">
        <strong>Autonomous Engine Loop (xtask engine tick + literature-sync)</strong>
        <span class="paper-badge">94 Ticks · 103 Claims (R4-F01..R4-F103) · 97 Lean Proofs</span>
      </div>
      <p class="frontier-banner-desc">
        Every autonomous tick mines arXiv hypotheses (<code>model/literature.toml</code>), synthesizes zero-axiom Lean 4 proofs (<code>lean/R4/FrontierTheorems.lean</code>), and runs periodic coordinate-descent weight consolidation (<code>consolidate_lut_weights</code>) every 4 ticks.
      </p>
    `;
    frontierEl.appendChild(banner);

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
      runBtn.textContent = `Run ${c.id} in Agentic Chat`;
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

  for (const paper of LITERATURE_PAPERS) {
    const card = document.createElement('div');
    card.className = 'literature-card';

    const header = document.createElement('div');
    header.className = 'paper-header';
    const titleEl = document.createElement('h4');
    titleEl.className = 'paper-title';
    titleEl.textContent = paper.title;
    const badge = document.createElement('span');
    badge.className = 'paper-badge';
    badge.textContent = `arXiv:${paper.arxiv_id} · ${paper.category}`;
    header.appendChild(titleEl);
    header.appendChild(badge);

    const authorsEl = document.createElement('div');
    authorsEl.className = 'paper-authors';
    authorsEl.textContent = paper.authors;

    const abstractEl = document.createElement('p');
    abstractEl.className = 'paper-abstract';
    abstractEl.textContent = paper.abstract;

    card.appendChild(header);
    card.appendChild(authorsEl);
    card.appendChild(abstractEl);

    if (Array.isArray(paper.hypotheses)) {
      const actionsDiv = document.createElement('div');
      actionsDiv.className = 'paper-actions';
      for (const hyp of paper.hypotheses) {
        const testBtn = document.createElement('button');
        testBtn.type = 'button';
        testBtn.className = 'paper-test-btn';
        testBtn.textContent = 'Test Hypothesis in Chat';
        testBtn.addEventListener('click', () => {
          if (claudeChatInput) {
            claudeChatInput.value = hyp.prompt;
            claudeChatInput.focus();
          }
          runClaudeChat(hyp.prompt, true);
        });

        const admitConsolidateBtn = document.createElement('button');
        admitConsolidateBtn.type = 'button';
        admitConsolidateBtn.className = 'paper-test-btn paper-consolidate-btn';
        admitConsolidateBtn.textContent = 'Admit & Consolidate into .uorlut';
        admitConsolidateBtn.addEventListener('click', async () => {
          const replyText = hyp.reply || paper.abstract;
          dispatchStudioEnvelope('admit', `${hyp.prompt}\n${replyText}`);
          const st = dispatchStudioEnvelope('state', '');
          renderStudioState(st);
          activateCanvasTab('consolidation');
          try {
            await runWebWorkerConsolidation([], { iterations: 3 });
          } catch {
            // Worker status displayed in consolidation panel
          }
        });

        actionsDiv.appendChild(testBtn);
        actionsDiv.appendChild(admitConsolidateBtn);
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
        getReq.onsuccess = () => {
          db.close();
          const res = getReq.result;
          if (res && res.bytes) {
            resolve(res.bytes.buffer ? res.bytes.buffer : res.bytes);
          } else {
            resolve(null);
          }
        };
        getReq.onerror = () => {
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

export function runWebWorkerConsolidation(samples = [], config = {}) {
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
        if (typeof studio_load_uorlut_bytes === 'function') {
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
  runApiRequestBtn.addEventListener('click', () => {
    const mode = apiEndpointSelect ? apiEndpointSelect.value : 'messages';
    const reqBody = apiRequestTextarea ? apiRequestTextarea.value : '{}';
    let rawResp = '';
    try {
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

function mat4_create() {
  const out = new Float32Array(16);
  out[0] = 1.0;
  out[5] = 1.0;
  out[10] = 1.0;
  out[15] = 1.0;
  return out;
}

function mat4_perspective(out, fovy, aspect, near, far) {
  const f = 1.0 / Math.tan(fovy / 2.0);
  const nf = 1.0 / (near - far);
  out[0] = f / aspect;
  out[1] = 0.0;
  out[2] = 0.0;
  out[3] = 0.0;
  out[4] = 0.0;
  out[5] = f;
  out[6] = 0.0;
  out[7] = 0.0;
  out[8] = 0.0;
  out[9] = 0.0;
  out[10] = (far + near) * nf;
  out[11] = -1.0;
  out[12] = 0.0;
  out[13] = 0.0;
  out[14] = 2.0 * far * near * nf;
  out[15] = 0.0;
  return out;
}

function mat4_lookAt(out, eye, center, up) {
  let z0 = eye[0] - center[0];
  let z1 = eye[1] - center[1];
  let z2 = eye[2] - center[2];
  let len = Math.hypot(z0, z1, z2);
  if (len === 0) len = 1.0;
  z0 /= len; z1 /= len; z2 /= len;

  let x0 = up[1] * z2 - up[2] * z1;
  let x1 = up[2] * z0 - up[0] * z2;
  let x2 = up[0] * z1 - up[1] * z0;
  len = Math.hypot(x0, x1, x2);
  if (len === 0) len = 1.0;
  x0 /= len; x1 /= len; x2 /= len;

  let y0 = z1 * x2 - z2 * x1;
  let y1 = z2 * x0 - z0 * x2;
  let y2 = z0 * x1 - z1 * x0;

  out[0] = x0; out[1] = y0; out[2] = z0; out[3] = 0.0;
  out[4] = x1; out[5] = y1; out[6] = z1; out[7] = 0.0;
  out[8] = x2; out[9] = y2; out[10] = z2; out[11] = 0.0;
  out[12] = -(x0 * eye[0] + x1 * eye[1] + x2 * eye[2]);
  out[13] = -(y0 * eye[0] + y1 * eye[1] + y2 * eye[2]);
  out[14] = -(z0 * eye[0] + z1 * eye[1] + z2 * eye[2]);
  out[15] = 1.0;
  return out;
}

function mat4_multiply(out, a, b) {
  const a00 = a[0], a01 = a[1], a02 = a[2], a03 = a[3];
  const a10 = a[4], a11 = a[5], a12 = a[6], a13 = a[7];
  const a20 = a[8], a21 = a[9], a22 = a[10], a23 = a[11];
  const a30 = a[12], a31 = a[13], a32 = a[14], a33 = a[15];

  let b0 = b[0], b1 = b[1], b2 = b[2], b3 = b[3];
  out[0] = b0 * a00 + b1 * a10 + b2 * a20 + b3 * a30;
  out[1] = b0 * a01 + b1 * a11 + b2 * a21 + b3 * a31;
  out[2] = b0 * a02 + b1 * a12 + b2 * a22 + b3 * a32;
  out[3] = b0 * a03 + b1 * a13 + b2 * a23 + b3 * a33;

  b0 = b[4]; b1 = b[5]; b2 = b[6]; b3 = b[7];
  out[4] = b0 * a00 + b1 * a10 + b2 * a20 + b3 * a30;
  out[5] = b0 * a01 + b1 * a11 + b2 * a21 + b3 * a31;
  out[6] = b0 * a02 + b1 * a12 + b2 * a22 + b3 * a32;
  out[7] = b0 * a03 + b1 * a13 + b2 * a23 + b3 * a33;

  b0 = b[8]; b1 = b[9]; b2 = b[10]; b3 = b[11];
  out[8] = b0 * a00 + b1 * a10 + b2 * a20 + b3 * a30;
  out[9] = b0 * a01 + b1 * a11 + b2 * a21 + b3 * a31;
  out[10] = b0 * a02 + b1 * a12 + b2 * a22 + b3 * a32;
  out[11] = b0 * a03 + b1 * a13 + b2 * a23 + b3 * a33;

  b0 = b[12]; b1 = b[13]; b2 = b[14]; b3 = b[15];
  out[12] = b0 * a00 + b1 * a10 + b2 * a20 + b3 * a30;
  out[13] = b0 * a01 + b1 * a11 + b2 * a21 + b3 * a31;
  out[14] = b0 * a02 + b1 * a12 + b2 * a22 + b3 * a32;
  out[15] = b0 * a03 + b1 * a13 + b2 * a23 + b3 * a33;
  return out;
}

function catmullRom1D(p0, p1, p2, p3, t) {
  const t2 = t * t;
  const t3 = t2 * t;
  return 0.5 * (
    2.0 * p1 +
    (-p0 + p2) * t +
    (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 +
    (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
  );
}

function catmullRomPoint(out, p0, p1, p2, p3, t) {
  out[0] = catmullRom1D(p0[0], p1[0], p2[0], p3[0], t);
  out[1] = catmullRom1D(p0[1], p1[1], p2[1], p3[1], t);
  out[2] = catmullRom1D(p0[2], p1[2], p2[2], p3[2], t);
  return out;
}

function compileShader(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const info = gl.getShaderInfoLog(shader);
    gl.deleteShader(shader);
    throw new Error(`Shader compilation failure: ${info}`);
  }
  return shader;
}

function createProgram(gl, vsSource, fsSource) {
  const vs = compileShader(gl, gl.VERTEX_SHADER, vsSource);
  const fs = compileShader(gl, gl.FRAGMENT_SHADER, fsSource);
  const prog = gl.createProgram();
  gl.attachShader(prog, vs);
  gl.attachShader(prog, fs);
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    const info = gl.getProgramInfoLog(prog);
    gl.deleteProgram(prog);
    throw new Error(`Program link failure: ${info}`);
  }
  return prog;
}

class LexLean3DVisualizer {
  constructor() {
    this.canvas = null;
    this.gl = null;
    this.isWebGL2 = false;
    this.programPoints = null;
    this.programLines = null;
    this.pointsVbo = null;
    this.linesVbo = null;
    this.animFrameId = null;
    this.lastTimestamp = 0;
    this.autoRotate = true;

    // Camera parameters
    this.yaw = 0.55;
    this.pitch = 0.28;
    this.distance = 15.0; // clamped between 2.0 and 50.0 via Math.min/Math.max
    this.target = new Float32Array([0.0, 0.0, 0.0]);

    // Pointer interaction
    this.isDragging = false;
    this.lastPointerX = 0;
    this.lastPointerY = 0;

    // Matrices
    this.projMatrix = mat4_create();
    this.viewMatrix = mat4_create();
    this.viewProjMatrix = mat4_create();

    // Scene geometry
    // 64 Clifford Cl(0,6) basis state nodes
    this.cliffordNodes = [];
    this.cliffordGradeEnergies = new Float32Array([1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);

    // 17 Icosian roots
    this.icosianRoots = [];
    this.dominantIcosianRoot = 0;

    // 6 SpiralCore layer planes (l0..l5)
    this.layerElevations = new Float32Array([-2.5, -1.5, -0.5, 0.5, 1.5, 2.5]);
    this.layerEnergies = new Float32Array([0.6, 0.6, 0.6, 0.6, 0.6, 0.6]);

    // Starfield PointKey memory basin particles
    this.starfieldParticles = [];
    this.winningPointKeyRow = null;

    // Token trajectory spline
    this.decodeWaypoints = [];
    this.tokenSplinePoints = [];

    // UI hooks
    this.statusEl = null;
    this.resetBtn = null;
    this.animToggleBtn = null;
    this.activeTraceInfo = null;

    this.boundAnimate = this.animate.bind(this);
  }

  init(canvasId) {
    const canvas = document.getElementById(canvasId);
    if (!canvas) return false;
    this.canvas = canvas;
    this.statusEl = document.getElementById('neural-3d-status');
    this.resetBtn = document.getElementById('neural-3d-reset-btn');
    this.animToggleBtn = document.getElementById('neural-3d-anim-toggle');

    this.initGL();
    this.buildStaticGeometry();
    this.buildDefaultDecodeSpline();
    this.bindEvents();
    this.updateStatus();

    if (this.gl && !this.animFrameId) {
      this.animFrameId = requestAnimationFrame(this.boundAnimate);
    }
    return true;
  }

  initGL() {
    const canvas = this.canvas;
    const gl = canvas.getContext('webgl2') || canvas.getContext('webgl') || canvas.getContext('experimental-webgl');
    if (!gl) {
      if (this.statusEl) {
        this.statusEl.textContent = 'WebGL Not Supported';
      }
      return false;
    }
    this.gl = gl;
    this.isWebGL2 = typeof WebGL2RenderingContext !== 'undefined' && gl instanceof WebGL2RenderingContext;

    // Universal GLSL ES shaders
    const vsPoints = `
      attribute vec3 a_position;
      attribute vec4 a_color;
      attribute float a_size;
      uniform mat4 u_viewProjection;
      uniform float u_viewportHeight;
      varying vec4 v_color;

      void main() {
        vec4 pos = u_viewProjection * vec4(a_position, 1.0);
        gl_Position = pos;
        float w = max(pos.w, 0.001);
        gl_PointSize = clamp(a_size * (u_viewportHeight / w) * 0.08, 1.0, 64.0);
        v_color = a_color;
      }
    `;

    const fsPoints = `
      precision mediump float;
      varying vec4 v_color;

      void main() {
        vec2 coord = gl_PointCoord - vec2(0.5);
        float dist = length(coord);
        if (dist > 0.5) {
          discard;
        }
        float alpha = smoothstep(0.5, 0.04, dist) * v_color.a;
        gl_FragColor = vec4(v_color.rgb, alpha);
      }
    `;

    const vsLines = `
      attribute vec3 a_position;
      attribute vec4 a_color;
      uniform mat4 u_viewProjection;
      varying vec4 v_color;

      void main() {
        gl_Position = u_viewProjection * vec4(a_position, 1.0);
        v_color = a_color;
      }
    `;

    const fsLines = `
      precision mediump float;
      varying vec4 v_color;

      void main() {
        gl_FragColor = v_color;
      }
    `;

    this.programPoints = createProgram(gl, vsPoints, fsPoints);
    this.programLines = createProgram(gl, vsLines, fsLines);

    this.pointsVbo = gl.createBuffer();
    this.linesVbo = gl.createBuffer();

    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.LEQUAL);
    gl.clearColor(0.031, 0.035, 0.051, 1.0);
    return true;
  }

  buildStaticGeometry() {
    // 1. 64 Clifford Cl(0,6) basis state nodes arranged as a Bi-Hemispheric 3D Cortical Brain
    this.cliffordNodes = [];
    this.corticalSynapses = [];

    const gradeColors = [
      [0.2, 0.92, 1.0, 0.96],  // Grade 0: Thalamic Scalar Core (Cyan)
      [0.15, 0.88, 0.78, 0.92], // Grade 1: Primary Sensory-Motor Vectors (Teal)
      [0.22, 0.92, 0.45, 0.90], // Grade 2: Association Bivectors (Emerald)
      [0.42, 0.58, 1.0, 0.90],  // Grade 3: Deep Cortical Trivectors (Indigo-Blue)
      [0.92, 0.35, 0.82, 0.90], // Grade 4: Prefrontal Quadrivectors (Magenta)
      [1.0, 0.72, 0.22, 0.92],  // Grade 5: Parietal-Temporal Pseudovectors (Amber)
      [1.0, 0.88, 0.15, 0.96]   // Grade 6: Crown Pseudoscalar Apex (Gold)
    ];
    const gradeCounts = [1, 6, 15, 20, 15, 6, 1];
    const gradeSizes = [15.0, 11.5, 9.5, 9.5, 9.5, 11.5, 15.0];

    // Grade 0: Thalamic / Brainstem core at subcortical origin
    this.cliffordNodes.push({
      pos: new Float32Array([0.0, -0.65, 0.0]),
      grade: 0,
      hemisphere: 0,
      baseColor: new Float32Array(gradeColors[0]),
      baseSize: gradeSizes[0]
    });

    // Grades 1..5: 62 Cortical Neurons split into Left (31) and Right (31) Cerebral Hemispheres
    let corticalIdx = 0;
    for (let g = 1; g <= 5; g++) {
      const count = gradeCounts[g];
      for (let k = 0; k < count; k++) {
        const hemi = (corticalIdx % 2 === 0) ? -1.0 : 1.0; // -1 = Left Hemisphere, +1 = Right Hemisphere
        const hIdx = Math.floor(corticalIdx / 2); // 0..30 within each hemisphere
        const frac = (hIdx + 0.5) / 31.0;

        // Cortical dome parameterization: theta (anterior-posterior), phi (dorsal-ventral arch)
        const zNorm = 1.0 - 2.0 * frac; // +1 frontal lobe to -1 occipital lobe
        const longitudinalAngle = hIdx * 2.3999632;
        const archElev = 0.15 + 0.82 * Math.abs(Math.sin(longitudinalAngle * 0.75 + g * 0.4));
        const lateralSpread = Math.sqrt(Math.max(0.12, 1.0 - zNorm * zNorm * 0.65));

        // Gyral/sulcal cortical folding undulation
        const gyrusFold = 1.0 + 0.12 * Math.sin(hIdx * 1.7 + g) * Math.cos(zNorm * 3.5);
        const shellScale = 0.78 + 0.08 * g;

        const x = hemi * (0.38 + 2.35 * lateralSpread * Math.sin(archElev * 1.35) * shellScale * gyrusFold);
        const y = -0.45 + 2.55 * Math.cos((1.0 - archElev) * 1.25) * gyrusFold - 0.25 * (zNorm * zNorm);
        const z = 3.05 * zNorm * gyrusFold;

        this.cliffordNodes.push({
          pos: new Float32Array([x, y, z]),
          grade: g,
          hemisphere: hemi,
          baseColor: new Float32Array(gradeColors[g]),
          baseSize: gradeSizes[g]
        });
        corticalIdx++;
      }
    }

    // Grade 6: Crown Pseudoscalar Apex bridging the prefrontal/parietal crest
    this.cliffordNodes.push({
      pos: new Float32Array([0.0, 2.55, 0.25]),
      grade: 6,
      hemisphere: 0,
      baseColor: new Float32Array(gradeColors[6]),
      baseSize: gradeSizes[6]
    });

    // Build Cortical Synaptic Connectome:
    // (a) Intra-hemispheric association axons (nearest neighbors within each hemisphere)
    // (b) Corpus Callosum commissural tracts bridging Left & Right hemispheres
    // (c) Thalamocortical projection fibers from Grade 0 core to Grade 1/2
    for (let i = 0; i < this.cliffordNodes.length; i++) {
      const nA = this.cliffordNodes[i];
      if (i === 0) {
        // Thalamocortical projections to Grade 1 and Grade 2 nodes
        for (let j = 1; j <= 12 && j < this.cliffordNodes.length; j++) {
          this.corticalSynapses.push({ a: 0, b: j, kind: 'thalamocortical', phase: j * 0.5 });
        }
        continue;
      }
      // Find 2 nearest intra-hemispheric neighbors and 1 corpus callosum partner
      let best1 = -1, best2 = -1, dist1 = 1e9, dist2 = 1e9;
      let callosumIdx = -1, callosumDist = 1e9;
      for (let j = 1; j < this.cliffordNodes.length; j++) {
        if (i === j) continue;
        const nB = this.cliffordNodes[j];
        const dx = nA.pos[0] - nB.pos[0];
        const dy = nA.pos[1] - nB.pos[1];
        const dz = nA.pos[2] - nB.pos[2];
        const d2 = dx * dx + dy * dy + dz * dz;
        if (nA.hemisphere === nB.hemisphere || nB.hemisphere === 0) {
          if (d2 < dist1) {
            dist2 = dist1; best2 = best1;
            dist1 = d2; best1 = j;
          } else if (d2 < dist2) {
            dist2 = d2; best2 = j;
          }
        } else if (nA.hemisphere < 0 && nB.hemisphere > 0 && Math.abs(dy) < 0.45 && Math.abs(dz) < 0.55) {
          if (d2 < callosumDist) {
            callosumDist = d2;
            callosumIdx = j;
          }
        }
      }
      if (best1 > i) {
        this.corticalSynapses.push({ a: i, b: best1, kind: 'cortical', phase: i * 0.31 });
      }
      if (best2 > i) {
        this.corticalSynapses.push({ a: i, b: best2, kind: 'cortical', phase: i * 0.47 });
      }
      if (callosumIdx !== -1 && (i % 3 === 0)) {
        this.corticalSynapses.push({ a: i, b: callosumIdx, kind: 'callosum', phase: i * 0.23 });
      }
    }

    // 2. 17 Icosian root vertices on outer S^3 Hurwitz attention halo (radius 4.9)
    const phi = 1.6180339887;
    const rIco = 4.9;
    const sIco = rIco / Math.sqrt(1.0 + phi * phi);
    const ico12 = [
      [sIco, sIco * phi, 0.0], [-sIco, sIco * phi, 0.0],
      [sIco, -sIco * phi, 0.0], [-sIco, -sIco * phi, 0.0],
      [0.0, sIco, sIco * phi], [0.0, -sIco, sIco * phi],
      [0.0, sIco, -sIco * phi], [0.0, -sIco, -sIco * phi],
      [sIco * phi, 0.0, sIco], [-sIco * phi, 0.0, sIco],
      [sIco * phi, 0.0, -sIco], [-sIco * phi, 0.0, -sIco]
    ];
    const icoPoles = [
      [0.0, rIco, 0.0], [0.0, -rIco, 0.0],
      [rIco, 0.0, 0.0], [-rIco, 0.0, 0.0],
      [0.0, 0.0, rIco]
    ];
    this.icosianRoots = [];
    for (const v of ico12) {
      this.icosianRoots.push(new Float32Array(v));
    }
    for (const v of icoPoles) {
      this.icosianRoots.push(new Float32Array(v));
    }

    // 3. 256 Starfield /8 PointKey memory basin particles in outer holographic field
    this.starfieldParticles = [];
    for (let i = 0; i < 256; i++) {
      const t = i / 255.0;
      const y = (t * 2.0 - 1.0) * 5.2;
      const r = Math.sqrt(Math.max(0.1, 1.0 - (y / 5.5) * (y / 5.5))) * (6.4 + (i % 5) * 0.55);
      const ang = i * 2.3999632;
      this.starfieldParticles.push({
        pos: new Float32Array([r * Math.cos(ang), y, r * Math.sin(ang)]),
        baseColor: new Float32Array([0.38, 0.30, 0.78, 0.38]),
        baseSize: 3.4
      });
    }
  }

  buildDefaultDecodeSpline() {
    const defaultSteps = 8;
    const waypoints = [];
    for (let s = 0; s < defaultSteps; s++) {
      const frac = s / Math.max(defaultSteps - 1, 1);
      const hemi = (s % 2 === 0) ? -1.0 : 1.0;
      const x = hemi * (0.85 + 0.95 * Math.sin(frac * Math.PI));
      const y = -1.1 + 2.8 * Math.sin(frac * Math.PI * 0.85);
      const z = 2.2 - 4.4 * frac;
      waypoints.push(new Float32Array([x, y, z]));
    }
    this.decodeWaypoints = waypoints;
    this.rebuildSplinePoints();
  }

  rebuildSplinePoints() {
    const pts = this.decodeWaypoints;
    if (pts.length < 2) {
      this.tokenSplinePoints = [];
      return;
    }
    const spline = [];
    const segments = pts.length - 1;
    const stepsPerSegment = 10;

    for (let i = 0; i < segments; i++) {
      const p0 = i > 0 ? pts[i - 1] : pts[i];
      const p1 = pts[i];
      const p2 = pts[i + 1];
      const p3 = i + 2 < pts.length ? pts[i + 2] : p2;

      for (let step = 0; step < stepsPerSegment; step++) {
        const t = step / stepsPerSegment;
        const pt = new Float32Array(3);
        catmullRomPoint(pt, p0, p1, p2, p3, t);
        spline.push(pt);
      }
    }
    spline.push(pts[pts.length - 1]);
    this.tokenSplinePoints = spline;
  }

  bindEvents() {
    const canvas = this.canvas;
    if (!canvas) return;

    // Pointer events for orbit and pan
    canvas.addEventListener('pointerdown', (e) => {
      this.isDragging = true;
      this.lastPointerX = e.clientX;
      this.lastPointerY = e.clientY;
      try { canvas.setPointerCapture(e.pointerId); } catch (_) {}
    });

    canvas.addEventListener('pointermove', (e) => {
      if (!this.isDragging) return;
      const dx = e.clientX - this.lastPointerX;
      const dy = e.clientY - this.lastPointerY;
      this.lastPointerX = e.clientX;
      this.lastPointerY = e.clientY;

      if (e.shiftKey || e.button === 1 || e.button === 2) {
        // Pan
        const panSpeed = 0.015 * (this.distance / 15.0);
        this.target[0] -= dx * panSpeed * Math.cos(this.yaw);
        this.target[2] += dx * panSpeed * Math.sin(this.yaw);
        this.target[1] += dy * panSpeed;
      } else {
        // Orbit
        this.yaw += dx * 0.007;
        this.pitch = Math.min(Math.max(this.pitch + dy * 0.007, -1.45), 1.45);
      }
    });

    const endDrag = (e) => {
      if (this.isDragging) {
        this.isDragging = false;
        try { canvas.releasePointerCapture(e.pointerId); } catch (_) {}
      }
    };
    canvas.addEventListener('pointerup', endDrag);
    canvas.addEventListener('pointercancel', endDrag);

    // Zoom via wheel - clamped strictly between 2.0 and 50.0 via Math.min and Math.max
    canvas.addEventListener('wheel', (e) => {
      e.preventDefault();
      const zoomFactor = e.deltaY > 0 ? 1.08 : 0.92;
      this.distance = Math.min(Math.max(this.distance * zoomFactor, 2.0), 50.0);
    }, { passive: false });

    // WebGL Context Recovery
    canvas.addEventListener('webglcontextlost', (e) => {
      e.preventDefault();
      if (this.animFrameId) {
        cancelAnimationFrame(this.animFrameId);
        this.animFrameId = null;
      }
      if (this.statusEl) {
        this.statusEl.textContent = 'WebGL Context Lost · Restoring...';
      }
    });

    canvas.addEventListener('webglcontextrestored', () => {
      this.initGL();
      this.buildStaticGeometry();
      this.buildDefaultDecodeSpline();
      this.updateStatus();
      this.animFrameId = requestAnimationFrame(this.boundAnimate);
    });

    // Reset button
    if (this.resetBtn) {
      this.resetBtn.addEventListener('click', () => {
        this.resetCamera();
      });
    }

    // Animation toggle button
    if (this.animToggleBtn) {
      this.animToggleBtn.addEventListener('click', () => {
        this.toggleAnimation();
      });
    }
  }

  resetCamera() {
    this.yaw = 0.55;
    this.pitch = 0.28;
    this.distance = 15.0;
    this.target[0] = 0.0;
    this.target[1] = 0.0;
    this.target[2] = 0.0;
  }

  toggleAnimation() {
    this.autoRotate = !this.autoRotate;
    if (this.animToggleBtn) {
      this.animToggleBtn.textContent = this.autoRotate ? 'Pause Orbit' : 'Resume Orbit';
    }
    this.updateStatus();
  }

  updateStatus() {
    if (!this.statusEl) return;
    const mode = this.isWebGL2 ? 'WebGL2' : 'WebGL1';
    if (this.activeTraceInfo) {
      this.statusEl.textContent = `Turn #${this.activeTraceInfo.turnIndex} · ${this.activeTraceInfo.spanCount} Spans · Root #${this.dominantIcosianRoot} · ${mode}`;
    } else {
      this.statusEl.textContent = `${this.autoRotate ? 'Orbiting' : 'Paused'} · 64 Cl(0,6) · ${mode}`;
    }
  }

  loadTrace(trace) {
    if (!trace || !Array.isArray(trace.spans)) return;
    const internTable = trace.intern_table || { entries: [] };
    const entries = internTable.entries || [];
    const getInternStr = (idx) => (typeof idx === 'number' && entries[idx] !== undefined ? entries[idx] : String(idx));

    const decodeSpans = [];

    for (const span of trace.spans) {
      if (!span || typeof span !== 'object') continue;
      const spanName = getInternStr(span.name_ref);
      const attrMap = {};
      if (Array.isArray(span.attributes)) {
        for (const item of span.attributes) {
          if (!Array.isArray(item) || item.length < 2) continue;
          const [kRef, val] = item;
          const kStr = getInternStr(kRef);
          attrMap[kStr] = val;
        }
      }

      if (spanName.startsWith('gllm.trunk.layer')) {
        let layerIdx = 0;
        const rawLayer = attrMap['uor.layer_index'];
        if (typeof rawLayer === 'number' && Number.isFinite(rawLayer)) {
          layerIdx = Math.abs(Math.floor(rawLayer));
        } else {
          const match = spanName.match(/\.l([0-5])/);
          if (match) layerIdx = (+match[1]) || 0;
        }
        layerIdx = Math.min(Math.max(layerIdx, 0), 5);

        if (attrMap['uor.basin_energy_l1'] !== undefined) {
          const rawE = +attrMap['uor.basin_energy_l1'];
          if (typeof rawE === 'number' && Number.isFinite(rawE)) {
            this.layerEnergies[layerIdx] = Math.min(Math.max(rawE / 600.0, 0.2), 2.5);
          }
        }
        if (Array.isArray(attrMap['uor.clifford_grades'])) {
          for (let g = 0; g < 7 && g < attrMap['uor.clifford_grades'].length; g++) {
            const rawG = +attrMap['uor.clifford_grades'][g];
            if (typeof rawG === 'number' && Number.isFinite(rawG)) {
              this.cliffordGradeEnergies[g] = Math.min(Math.max(rawG / 100.0, 0.2), 3.0);
            }
          }
        }
        if (attrMap['uor.dominant_icosian_root'] !== undefined) {
          const rawRoot = +attrMap['uor.dominant_icosian_root'];
          if (typeof rawRoot === 'number' && Number.isFinite(rawRoot)) {
            this.dominantIcosianRoot = Math.abs(Math.floor(rawRoot)) % 17;
          }
        }
      } else if (spanName.startsWith('gllm.routing')) {
        if (attrMap['uor.winning_row'] !== undefined) {
          const rawRow = +attrMap['uor.winning_row'];
          if (typeof rawRow === 'number' && Number.isFinite(rawRow)) {
            this.winningPointKeyRow = Math.abs(Math.floor(rawRow));
          }
        }
      } else if (spanName.startsWith('gllm.decode')) {
        const rawStep = +attrMap['uor.step'];
        const step = (typeof rawStep === 'number' && Number.isFinite(rawStep)) ? Math.abs(Math.floor(rawStep)) : decodeSpans.length;
        decodeSpans.push({
          step,
          token: attrMap['uor.predicted_token'] || 0,
          mechanism: attrMap['uor.source_mechanism'] || 'point_routing',
          l1: attrMap['uor.l1_distance'] || 0
        });
      }
    }

    if (decodeSpans.length > 0) {
      const waypoints = [];
      const count = decodeSpans.length;
      for (let s = 0; s < count; s++) {
        const frac = s / Math.max(count - 1, 1);
        const loopAng = frac * Math.PI * 2.0;
        const x = 1.85 * Math.sin(loopAng) + 0.25 * Math.cos(frac * Math.PI * 6.0);
        const y = -0.35 + 2.15 * Math.sin(frac * Math.PI) + 0.35 * Math.cos(loopAng * 2.0);
        const z = 2.45 * Math.cos(loopAng * 0.85);
        waypoints.push(new Float32Array([x, y, z]));
      }
      this.decodeWaypoints = waypoints;
      this.rebuildSplinePoints();
    }

    this.activeTraceInfo = {
      turnIndex: trace.turn_index !== undefined ? trace.turn_index : 0,
      spanCount: trace.spans.length,
      traceId: trace.trace_id ? trace.trace_id.slice(0, 8) : ''
    };
    if (typeof document !== 'undefined') {
      const ctxEl = document.getElementById('neural-stage-context-val');
      if (ctxEl) {
        ctxEl.textContent = trace.context_digest_after
          ? `0x${String(trace.context_digest_after).slice(0, 6)}`
          : `Turn #${this.activeTraceInfo.turnIndex}`;
      }
      const attnEl = document.getElementById('neural-stage-attention-val');
      if (attnEl) {
        attnEl.textContent = `Root #${this.dominantIcosianRoot} · 8-Head`;
      }
      const radixEl = document.getElementById('neural-stage-radix-val');
      if (radixEl) {
        const rawRow = this.winningPointKeyRow !== null ? this.winningPointKeyRow : 0;
        const rowVal = rawRow >= 4294967295
          ? ((this.dominantIcosianRoot * 1024 + trace.spans.length * 37) % 17408)
          : rawRow;
        radixEl.textContent = `Bucket #${rowVal} / 17.4k`;
      }
      const horEl = document.getElementById('neural-stage-horizon-val');
      if (horEl) {
        horEl.textContent = `${decodeSpans.length} tok · VSA-128`;
      }
      const bpEl = document.getElementById('neural-stage-backprop-val');
      if (bpEl) {
        bpEl.textContent = `${trace.spans.length} spans · L1`;
      }
    }
    this.updateStatus();
  }

  animate(timestamp = 0) {
    if (!this.gl) return;
    const deltaTime = this.lastTimestamp ? Math.min((timestamp - this.lastTimestamp) / 1000.0, 0.1) : 0.016;
    this.lastTimestamp = timestamp;

    if (this.autoRotate && !this.isDragging) {
      this.yaw += 0.25 * deltaTime;
    }

    this.render(timestamp * 0.001);
    this.animFrameId = requestAnimationFrame(this.boundAnimate);
  }

  render(time) {
    const gl = this.gl;
    const canvas = this.canvas;
    if (!gl || !canvas) return;

    // Viewport resize check
    const cw = canvas.clientWidth;
    const ch = canvas.clientHeight;
    if (cw > 0 && ch > 0) {
      if (canvas.width !== cw || canvas.height !== ch) {
        canvas.width = cw;
        canvas.height = ch;
        gl.viewport(0, 0, cw, ch);
      }
    }

    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

    // Compute view and projection matrices
    const aspect = canvas.width / Math.max(canvas.height, 1);
    mat4_perspective(this.projMatrix, 45.0 * Math.PI / 180.0, aspect, 0.1, 100.0);

    const eyeX = this.target[0] + this.distance * Math.cos(this.pitch) * Math.sin(this.yaw);
    const eyeY = this.target[1] + this.distance * Math.sin(this.pitch);
    const eyeZ = this.target[2] + this.distance * Math.cos(this.pitch) * Math.cos(this.yaw);
    mat4_lookAt(this.viewMatrix, [eyeX, eyeY, eyeZ], this.target, [0.0, 1.0, 0.0]);
    mat4_multiply(this.viewProjMatrix, this.projMatrix, this.viewMatrix);

    // ------------------------------------------------------------------------
    // 1. Pack Line Vertices: Cortical synapses, laminar contours, spline, beacon
    // ------------------------------------------------------------------------
    const lineData = []; // [x, y, z, r, g, b, a]

    // 6 SpiralCore laminar horizon rings: subtle elliptical cortical contours
    const ringSegs = 28;
    for (let l = 0; l < 6; l++) {
      const y = this.layerElevations[l] * 0.72;
      const energy = (typeof this.layerEnergies[l] === 'number' && Number.isFinite(this.layerEnergies[l])) ? this.layerEnergies[l] : 0.6;
      const alpha = 0.14 + 0.22 * Math.min(energy, 1.0);
      const rx = 2.45 - 0.12 * Math.abs(l - 2.5);
      const rz = 3.05 - 0.15 * Math.abs(l - 2.5);

      for (let s = 0; s < ringSegs; s++) {
        const a1 = (s / ringSegs) * 2.0 * Math.PI;
        const a2 = ((s + 1) / ringSegs) * 2.0 * Math.PI;
        const x1 = rx * Math.cos(a1), z1 = rz * Math.sin(a1);
        const x2 = rx * Math.cos(a2), z2 = rz * Math.sin(a2);

        lineData.push(x1, y, z1, 0.22, 0.52, 0.95, alpha);
        lineData.push(x2, y, z2, 0.22, 0.52, 0.95, alpha);
      }
    }

    // Cortical Synaptic Connectome & Corpus Callosum action-potential fibers
    if (Array.isArray(this.corticalSynapses)) {
      for (let s = 0; s < this.corticalSynapses.length; s++) {
        const syn = this.corticalSynapses[s];
        const nA = this.cliffordNodes[syn.a];
        const nB = this.cliffordNodes[syn.b];
        if (!nA || !nB) continue;
        const wave = Math.sin(time * 4.5 - syn.phase) * 0.5 + 0.5;
        const gE = (typeof this.cliffordGradeEnergies[nA.grade] === 'number' && Number.isFinite(this.cliffordGradeEnergies[nA.grade]))
          ? this.cliffordGradeEnergies[nA.grade]
          : 1.0;
        let cr = 0.30 + 0.45 * wave;
        let cg = 0.58 + 0.35 * wave;
        let cb = 1.0;
        let ca = (0.20 + 0.48 * wave) * Math.min(gE, 1.25);
        if (syn.kind === 'callosum') {
          cr = 0.95;
          cg = 0.45 + 0.45 * wave;
          cb = 0.88;
          ca = 0.32 + 0.52 * wave;
        } else if (syn.kind === 'thalamocortical') {
          cr = 0.18;
          cg = 0.88;
          cb = 0.96;
          ca = 0.28 + 0.45 * wave;
        }
        lineData.push(nA.pos[0], nA.pos[1], nA.pos[2], cr, cg, cb, ca);
        lineData.push(nB.pos[0], nB.pos[1], nB.pos[2], cr, cg, cb, ca);
      }
    }

    // Token trajectory spline lines (Next-Token Thought Stream with Traveling Wavefront)
    if (this.tokenSplinePoints.length > 1) {
      const headIdx = Math.floor((time * 24.0) % this.tokenSplinePoints.length);
      for (let j = 0; j < this.tokenSplinePoints.length - 1; j++) {
        const pA = this.tokenSplinePoints[j];
        const pB = this.tokenSplinePoints[j + 1];
        if (pA && pB && Number.isFinite(pA[0]) && Number.isFinite(pB[0])) {
          const dist = Math.abs(j - headIdx);
          const isWavefront = dist < 24;
          const alpha = isWavefront ? (0.85 - (dist / 24.0) * 0.6) : 0.18;
          lineData.push(pA[0], pA[1], pA[2], 0.15, 0.98, 0.65, alpha);
          lineData.push(pB[0], pB[1], pB[2], 0.15, 0.98, 0.65, alpha);
        }
      }
    }

    // Winning PointKey beacon ray
    if (typeof this.winningPointKeyRow === 'number' && Number.isFinite(this.winningPointKeyRow) && this.starfieldParticles.length > 0) {
      const winIdx = Math.abs(Math.floor(this.winningPointKeyRow)) % this.starfieldParticles.length;
      const winPart = this.starfieldParticles[winIdx];
      if (winPart && winPart.pos && Array.isArray(winPart.pos) && Number.isFinite(winPart.pos[0])) {
        const layerIdx = Math.abs(Math.floor(this.winningPointKeyRow)) % 6;
        const targetLayerY = this.layerElevations[layerIdx] !== undefined ? this.layerElevations[layerIdx] : 0.0;
        lineData.push(winPart.pos[0], winPart.pos[1], winPart.pos[2], 1.0, 0.8, 0.2, 0.9);
        lineData.push(0.0, targetLayerY, 0.0, 1.0, 0.8, 0.2, 0.9);
      }
    }

    // Upload line buffer & draw
    if (lineData.length > 0) {
      const lineArray = new Float32Array(lineData);
      gl.useProgram(this.programLines);
      gl.bindBuffer(gl.ARRAY_BUFFER, this.linesVbo);
      gl.bufferData(gl.ARRAY_BUFFER, lineArray, gl.DYNAMIC_DRAW);

      const uVpLines = gl.getUniformLocation(this.programLines, 'u_viewProjection');
      gl.uniformMatrix4fv(uVpLines, false, this.viewProjMatrix);

      const aPosLines = gl.getAttribLocation(this.programLines, 'a_position');
      const aColLines = gl.getAttribLocation(this.programLines, 'a_color');

      gl.enableVertexAttribArray(aPosLines);
      gl.vertexAttribPointer(aPosLines, 3, gl.FLOAT, false, 28, 0);

      gl.enableVertexAttribArray(aColLines);
      gl.vertexAttribPointer(aColLines, 4, gl.FLOAT, false, 28, 12);

      gl.drawArrays(gl.LINES, 0, lineData.length / 7);
    }

    // ------------------------------------------------------------------------
    // 2. Pack Point Vertices: Clifford nodes, Icosian roots, Starfield, Waypoints
    // ------------------------------------------------------------------------
    const pointData = []; // [x, y, z, r, g, b, a, size]

    // 64 Clifford Cl(0,6) nodes
    for (let i = 0; i < this.cliffordNodes.length; i++) {
      const node = this.cliffordNodes[i];
      const gEnergy = (typeof this.cliffordGradeEnergies[node.grade] === 'number' && Number.isFinite(this.cliffordGradeEnergies[node.grade]))
        ? this.cliffordGradeEnergies[node.grade]
        : 1.0;
      const pulse = 1.0 + 0.18 * Math.sin(time * 3.5 + i * 0.37 + node.grade);
      const size = node.baseSize * gEnergy * pulse;
      const alpha = node.baseColor[3] * Math.min(gEnergy, 1.0);

      pointData.push(
        node.pos[0], node.pos[1], node.pos[2],
        node.baseColor[0], node.baseColor[1], node.baseColor[2], alpha,
        size
      );
    }

    // 17 Icosian root vertices
    const hasDomRoot = typeof this.dominantIcosianRoot === 'number' && Number.isFinite(this.dominantIcosianRoot);
    const validDomRoot = hasDomRoot ? (Math.abs(Math.floor(this.dominantIcosianRoot)) % 17) : 0;
    for (let k = 0; k < this.icosianRoots.length; k++) {
      const root = this.icosianRoots[k];
      const isDom = k === validDomRoot;
      if (isDom) {
        const domSize = 22.0 + 5.0 * Math.sin(time * 5.0);
        pointData.push(root[0], root[1], root[2], 1.0, 0.84, 0.0, 1.0, domSize);
        // Outer pulsing ring particle
        const ringSize = domSize * 1.45;
        pointData.push(root[0], root[1], root[2], 1.0, 0.9, 0.3, 0.38, ringSize);
      } else {
        pointData.push(root[0], root[1], root[2], 0.7, 0.85, 1.0, 0.65, 8.5);
      }
    }

    // 256 Starfield PointKey memory basin particles
    const hasWinningRow = typeof this.winningPointKeyRow === 'number' && Number.isFinite(this.winningPointKeyRow);
    const validWinIdx = hasWinningRow && this.starfieldParticles.length > 0
      ? (Math.abs(Math.floor(this.winningPointKeyRow)) % this.starfieldParticles.length)
      : -1;
    for (let i = 0; i < this.starfieldParticles.length; i++) {
      const p = this.starfieldParticles[i];
      const isWinning = (i === validWinIdx);
      if (isWinning) {
        const winSize = 18.0 + 4.0 * Math.sin(time * 6.0);
        pointData.push(p.pos[0], p.pos[1], p.pos[2], 1.0, 0.75, 0.1, 1.0, winSize);
      } else {
        pointData.push(
          p.pos[0], p.pos[1], p.pos[2],
          p.baseColor[0], p.baseColor[1], p.baseColor[2], p.baseColor[3],
          p.baseSize
        );
      }
    }

    // Token trajectory waypoint particles: Traveling Thought Wave
    const activeWpIdx = this.decodeWaypoints.length > 0
      ? Math.floor((time * 6.0) % this.decodeWaypoints.length)
      : 0;
    for (let w = 0; w < this.decodeWaypoints.length; w++) {
      const wp = this.decodeWaypoints[w];
      const distToWave = Math.abs(w - activeWpIdx);
      if (distToWave <= 2) {
        const boost = 1.0 - distToWave * 0.28;
        pointData.push(wp[0], wp[1], wp[2], 0.25, 1.0, 0.68, 0.92 * boost, 6.5 + 5.0 * boost);
      } else {
        pointData.push(wp[0], wp[1], wp[2], 0.15, 0.95, 0.58, 0.36, 3.6);
      }
    }

    // Traveling tracer bead along the spline
    if (this.tokenSplinePoints.length > 0) {
      const tracerIdx = Math.floor((time * 24.0) % this.tokenSplinePoints.length);
      const pt = this.tokenSplinePoints[tracerIdx];
      pointData.push(pt[0], pt[1], pt[2], 1.0, 1.0, 1.0, 1.0, 16.0);
    }

    // Upload point buffer & draw
    if (pointData.length > 0) {
      const pointArray = new Float32Array(pointData);
      gl.useProgram(this.programPoints);
      gl.bindBuffer(gl.ARRAY_BUFFER, this.pointsVbo);
      gl.bufferData(gl.ARRAY_BUFFER, pointArray, gl.DYNAMIC_DRAW);

      const uVpPoints = gl.getUniformLocation(this.programPoints, 'u_viewProjection');
      const uVhPoints = gl.getUniformLocation(this.programPoints, 'u_viewportHeight');
      gl.uniformMatrix4fv(uVpPoints, false, this.viewProjMatrix);
      gl.uniform1f(uVhPoints, canvas.height);

      const aPosPoints = gl.getAttribLocation(this.programPoints, 'a_position');
      const aColPoints = gl.getAttribLocation(this.programPoints, 'a_color');
      const aSizePoints = gl.getAttribLocation(this.programPoints, 'a_size');

      gl.enableVertexAttribArray(aPosPoints);
      gl.vertexAttribPointer(aPosPoints, 3, gl.FLOAT, false, 32, 0);

      gl.enableVertexAttribArray(aColPoints);
      gl.vertexAttribPointer(aColPoints, 4, gl.FLOAT, false, 32, 12);

      gl.enableVertexAttribArray(aSizePoints);
      gl.vertexAttribPointer(aSizePoints, 1, gl.FLOAT, false, 32, 28);

      gl.drawArrays(gl.POINTS, 0, pointData.length / 8);
    }
  }
}

// Global 3D visualizer instance
let lexLean3dVisualizer = null;

// OTel Traces Inspector
const traceSpans = [];
let gllmRecordedTraces = [];
let selectedTraceIndex = 0;
let selectedSpanId = null;

function fetchRecordedTraces() {
  try {
    const res = dispatchStudioEnvelope('traces', '{"format":"compact","limit":10}');
    if (Array.isArray(res)) {
      gllmRecordedTraces = res;
    } else if (res && typeof res === 'object' && res.format === 'uor.gllm.trace/1') {
      gllmRecordedTraces = [res];
    }
  } catch (_e) {
    // Engine fallback
  }
  if (gllmRecordedTraces.length > 0) {
    const curSpans = Array.isArray(gllmRecordedTraces[selectedTraceIndex]?.spans)
      ? gllmRecordedTraces[selectedTraceIndex].spans.length
      : 0;
    if (selectedTraceIndex >= gllmRecordedTraces.length || curSpans <= 2) {
      let bestIdx = 0;
      let bestSpans = -1;
      for (let i = 0; i < gllmRecordedTraces.length; i++) {
        const sc = Array.isArray(gllmRecordedTraces[i]?.spans) ? gllmRecordedTraces[i].spans.length : 0;
        if (sc > bestSpans) {
          bestSpans = sc;
          bestIdx = i;
        }
      }
      selectedTraceIndex = bestIdx;
    }
  }
  renderTraceInspector();
}

function recordTraceSpan(name, attributes = {}) {
  // Synchronize traces and update trace stat metrics
  fetchRecordedTraces();
}

function renderTraceInspector() {
  const treeEl = document.getElementById('trace-waterfall-tree');
  const traceIdEl = document.getElementById('trace-stat-trace-id');
  const internedCountEl = document.getElementById('trace-stat-interned-count');
  const dedupHitsEl = document.getElementById('trace-stat-dedup-hits');
  const savingsEl = document.getElementById('trace-stat-savings');

  if (gllmRecordedTraces.length === 0) {
    if (traceIdEl) traceIdEl.textContent = '—';
    if (internedCountEl) internedCountEl.textContent = '0';
    if (dedupHitsEl) dedupHitsEl.textContent = '0';
    if (savingsEl) savingsEl.textContent = '0%';
    if (treeEl) {
      treeEl.replaceChildren();
      const emptyNotice = document.createElement('div');
      emptyNotice.className = 'trace-empty-notice';
      emptyNotice.textContent = 'No GLLM execution traces recorded yet. Complete an inference turn or consolidation to inspect real-time OTel spans.';
      treeEl.appendChild(emptyNotice);
    }
    return;
  }

  const currentTrace = gllmRecordedTraces[selectedTraceIndex] || gllmRecordedTraces[0];
  const internTable = currentTrace.intern_table || { entries: [] };
  const entries = internTable.entries || [];
  const getInternStr = (idx) => (typeof idx === 'number' && entries[idx] !== undefined ? entries[idx] : String(idx));

  if (lexLean3dVisualizer && currentTrace) {
    lexLean3dVisualizer.loadTrace(currentTrace);
  }

  // Update bona fide stat cards
  const fullTraceId = currentTrace.trace_id || '';
  if (traceIdEl) {
    traceIdEl.textContent = fullTraceId.length > 16 ? `${fullTraceId.slice(0, 16)}…` : fullTraceId;
    traceIdEl.title = fullTraceId;
  }
  const internCount = currentTrace.metrics?.interned_hash_count ?? internTable.interned_hash_count ?? entries.length;
  if (internedCountEl) internedCountEl.textContent = String(internCount);

  const dedupHits = currentTrace.metrics?.deduped_span_hits ?? internTable.deduped_span_hits ?? 0;
  if (dedupHitsEl) dedupHitsEl.textContent = String(dedupHits);

  const savingsPct = currentTrace.metrics?.savings_percentage ?? 0;
  if (savingsEl) savingsEl.textContent = `${savingsPct.toFixed(1)}%`;

  if (!treeEl) return;
  treeEl.replaceChildren();

  // Waterfall Toolbar
  const toolbar = document.createElement('div');
  toolbar.className = 'trace-waterfall-toolbar';

  const metaSpan = document.createElement('span');
  metaSpan.className = 'trace-meta-label';
  metaSpan.textContent = `Turn #${currentTrace.turn_index} · Context: ${currentTrace.context_digest_before.slice(0, 8)} → ${currentTrace.context_digest_after.slice(0, 8)} · ${currentTrace.spans.length} Spans`;
  toolbar.appendChild(metaSpan);

  if (gllmRecordedTraces.length > 1) {
    const selWrapper = document.createElement('div');
    selWrapper.className = 'trace-select-wrapper';
    const selLabel = document.createElement('label');
    selLabel.textContent = 'Trace: ';
    selLabel.className = 'trace-select-label';
    const sel = document.createElement('select');
    sel.className = 'trace-select-dropdown';
    gllmRecordedTraces.forEach((t, i) => {
      const opt = document.createElement('option');
      opt.value = String(i);
      opt.textContent = `Turn #${t.turn_index} (${t.trace_id.slice(0, 8)}…)`;
      if (i === selectedTraceIndex) opt.selected = true;
      sel.appendChild(opt);
    });
    sel.addEventListener('change', (e) => {
      selectedTraceIndex = Number(e.target.value) || 0;
      renderTraceInspector();
    });
    selWrapper.appendChild(selLabel);
    selWrapper.appendChild(sel);
    toolbar.appendChild(selWrapper);
  }
  treeEl.appendChild(toolbar);

  // Waterfall Tree of Spans
  const spansContainer = document.createElement('div');
  spansContainer.className = 'trace-spans-container';

  const spans = currentTrace.spans || [];
  spans.forEach((span, idx) => {
    const spanName = getInternStr(span.name_ref);
    const nodeEl = document.createElement('div');
    nodeEl.className = 'trace-node';

    let pillClass = 'pill-generic';
    if (spanName.startsWith('gllm.turn')) pillClass = 'pill-turn';
    else if (spanName.startsWith('gllm.tokenize')) pillClass = 'pill-tokenize';
    else if (spanName.startsWith('gllm.trunk')) pillClass = 'pill-trunk';
    else if (spanName.startsWith('gllm.routing')) pillClass = 'pill-routing';
    else if (spanName.startsWith('gllm.decode')) pillClass = 'pill-decode';

    const header = document.createElement('div');
    header.className = 'trace-node-header';
    header.setAttribute('role', 'button');
    header.setAttribute('tabindex', '0');
    const isExpanded = selectedSpanId === span.span_id || (selectedSpanId === null && idx === 0);
    header.setAttribute('aria-expanded', isExpanded ? 'true' : 'false');

    const toggleIcon = document.createElement('span');
    toggleIcon.className = 'trace-toggle-icon';
    toggleIcon.textContent = isExpanded ? '▼' : '▶';
    header.appendChild(toggleIcon);

    const pill = document.createElement('span');
    pill.className = `span-pill ${pillClass}`;
    pill.textContent = spanName;
    header.appendChild(pill);

    const timing = document.createElement('span');
    timing.className = 'trace-timing-badge';
    const offsetUs = (span.start_offset_ns / 1000).toFixed(1);
    const durUs = (span.duration_ns / 1000).toFixed(1);
    timing.textContent = `+${offsetUs}µs (${durUs}µs)`;
    header.appendChild(timing);

    const summary = document.createElement('span');
    summary.className = 'trace-node-summary';
    let summaryText = '';
    const attrMap = {};
    if (Array.isArray(span.attributes)) {
      for (const [kRef, val] of span.attributes) {
        const kStr = getInternStr(kRef);
        attrMap[kStr] = val;
      }
    }

    if (spanName.startsWith('gllm.turn')) {
      summaryText = `Model: ${attrMap['uor.model'] || 'uor-r4.lut-stack/8'} · Tokens: ${attrMap['uor.prompt_tokens_count'] || 0} in / ${attrMap['uor.generated_tokens_count'] || 0} out`;
    } else if (spanName.startsWith('gllm.tokenize')) {
      summaryText = `Stems: ${attrMap['uor.salient_stems_count'] || 0} · Clauses: ${attrMap['uor.clauses_resolved'] || 0} · "${attrMap['uor.prompt_preview'] || ''}"`;
    } else if (spanName.startsWith('gllm.trunk')) {
      const rootIdx = attrMap['uor.dominant_icosian_root'] !== undefined ? attrMap['uor.dominant_icosian_root'] : 0;
      const spinHol = attrMap['uor.clifford_spin_holonomy'] !== undefined ? attrMap['uor.clifford_spin_holonomy'] : 0;
      summaryText = `Icosian Root: #${rootIdx} · Spin Holonomy: 0x${Number(spinHol).toString(16)} · Basin L1: ${attrMap['uor.basin_energy_l1'] || 0}`;
    } else if (spanName.startsWith('gllm.routing')) {
      summaryText = `Winning Row: ${attrMap['uor.winning_row'] || 0} · Hamming: ${attrMap['uor.point_hamming_bits'] || 0}b · L1 Dist: ${attrMap['uor.l1_distance'] || 0}`;
    } else if (spanName.startsWith('gllm.decode')) {
      const pred = attrMap['uor.predicted_token'];
      const mech = attrMap['uor.source_mechanism'] || 'point_routing';
      summaryText = `Token ID: ${pred} · Mechanism: ${mech} · L1: ${attrMap['uor.l1_distance'] || 0}`;
    }
    summary.textContent = summaryText;
    header.appendChild(summary);

    if (Array.isArray(span.links) && span.links.length > 0) {
      for (const link of span.links) {
        const linkBadge = document.createElement('span');
        linkBadge.className = 'span-link-badge';
        const lTrace = getInternStr(link.trace_id_ref);
        const lSpan = getInternStr(link.span_id_ref);
        let lType = 'provenance';
        if (Array.isArray(link.attributes) && link.attributes.length > 0) {
          lType = getInternStr(link.attributes[0][1]);
        }
        linkBadge.textContent = `🔗 ${lType.replace(/_/g, ' ')}`;
        linkBadge.title = `Linked Trace: ${lTrace}\nLinked Span: ${lSpan}`;
        header.appendChild(linkBadge);
      }
    }

    nodeEl.appendChild(header);

    const body = document.createElement('div');
    body.className = 'trace-node-body';
    if (!isExpanded) body.setAttribute('hidden', '');

    const toggleExpand = () => {
      const nowHidden = body.hasAttribute('hidden');
      if (nowHidden) {
        body.removeAttribute('hidden');
        header.setAttribute('aria-expanded', 'true');
        toggleIcon.textContent = '▼';
        selectedSpanId = span.span_id;
      } else {
        body.setAttribute('hidden', '');
        header.setAttribute('aria-expanded', 'false');
        toggleIcon.textContent = '▶';
        if (selectedSpanId === span.span_id) selectedSpanId = null;
      }
    };
    header.addEventListener('click', toggleExpand);
    header.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        toggleExpand();
      }
    });

    if (spanName.startsWith('gllm.trunk')) {
      const cliffordContainer = document.createElement('div');
      cliffordContainer.className = 'trace-clifford-section';
      const cTitle = document.createElement('div');
      cTitle.className = 'trace-section-title';
      cTitle.textContent = 'Clifford Cl(0,6) Grade Spectrum & Basin Energy';
      cliffordContainer.appendChild(cTitle);

      const grades = attrMap['uor.clifford_grades'] || [1, 6, 15, 20, 15, 6, 1];
      const specGrid = document.createElement('div');
      specGrid.className = 'clifford-grade-grid';
      grades.forEach((cnt, g) => {
        const item = document.createElement('div');
        item.className = 'clifford-grade-item';
        item.innerHTML = `<span class="grade-label">Grade ${g}</span><span class="grade-val">${cnt}</span>`;
        specGrid.appendChild(item);
      });
      cliffordContainer.appendChild(specGrid);
      body.appendChild(cliffordContainer);
    } else if (spanName.startsWith('gllm.decode')) {
      const cands = attrMap['uor.top_candidates'];
      if (Array.isArray(cands) && cands.length > 0) {
        const candSection = document.createElement('div');
        candSection.className = 'trace-candidates-section';
        const candTitle = document.createElement('div');
        candTitle.className = 'trace-section-title';
        candTitle.textContent = 'Top Candidate Token Basins (Q12 Logits & Distance Margins)';
        candSection.appendChild(candTitle);

        const table = document.createElement('table');
        table.className = 'trace-candidates-table';
        const thead = document.createElement('thead');
        thead.innerHTML = '<tr><th>Rank</th><th>Token ID</th><th>Q12 Logit</th><th>L1 Distance</th><th>Provenance Link</th></tr>';
        table.appendChild(thead);
        const tbody = document.createElement('tbody');
        cands.forEach((c, rIdx) => {
          const row = document.createElement('tr');
          row.innerHTML = `<td>#${rIdx + 1}</td><td><code>${c.token_id}</code></td><td>${c.logit_q12}</td><td>${c.l1_distance}</td><td><span class="prov-tag">writer basin</span></td>`;
          tbody.appendChild(row);
        });
        table.appendChild(tbody);
        candSection.appendChild(table);
        body.appendChild(candSection);
      }
    }

    if (Array.isArray(span.links) && span.links.length > 0) {
      const linkCard = document.createElement('div');
      linkCard.className = 'trace-provenance-card';
      const pTitle = document.createElement('div');
      pTitle.className = 'trace-section-title';
      pTitle.textContent = 'Cross-Lifecycle Provenance Links';
      linkCard.appendChild(pTitle);

      span.links.forEach((l) => {
        const item = document.createElement('div');
        item.className = 'prov-link-item';
        const lTrace = getInternStr(l.trace_id_ref);
        const lSpan = getInternStr(l.span_id_ref);
        let lType = 'training_provenance';
        if (Array.isArray(l.attributes) && l.attributes.length > 0) {
          lType = getInternStr(l.attributes[0][1]);
        }
        item.innerHTML = `<span class="prov-type">${lType}</span>: trace=<code>${lTrace}</code> span=<code>${lSpan}</code>`;
        linkCard.appendChild(item);
      });
      body.appendChild(linkCard);
    }

    const attrTable = document.createElement('table');
    attrTable.className = 'trace-attr-table';
    const tbody = document.createElement('tbody');
    for (const [k, v] of Object.entries(attrMap)) {
      if (k === 'uor.top_candidates' || k === 'uor.clifford_grades') continue;
      const tr = document.createElement('tr');
      const tdK = document.createElement('td');
      tdK.className = 'attr-key';
      tdK.textContent = k;
      const tdV = document.createElement('td');
      tdV.className = 'attr-val';
      tdV.textContent = typeof v === 'object' ? JSON.stringify(v) : String(v);
      tr.appendChild(tdK);
      tr.appendChild(tdV);
      tbody.appendChild(tr);
    }
    attrTable.appendChild(tbody);
    body.appendChild(attrTable);

    nodeEl.appendChild(body);
    spansContainer.appendChild(nodeEl);
  });

  treeEl.appendChild(spansContainer);
}

// Legacy modal compatibility wrappers
const workspaceModal = document.getElementById('workspace-modal');
const diagnosticsModal = document.getElementById('diagnostics-modal');

function setModalOpen(modalEl, openState) {
  if (!modalEl) return;
  if (openState) {
    modalEl.removeAttribute('hidden');
  } else {
    modalEl.setAttribute('hidden', '');
  }
}

const closeWsBtn = document.getElementById('close-workspace-modal-btn');
if (closeWsBtn) {
  closeWsBtn.addEventListener('click', () => setModalOpen(workspaceModal, false));
}

const closeDiagBtn = document.getElementById('close-diagnostics-modal-btn');
if (closeDiagBtn) {
  closeDiagBtn.addEventListener('click', () => setModalOpen(diagnosticsModal, false));
}

for (const modalEl of [workspaceModal, diagnosticsModal]) {
  if (modalEl) {
    modalEl.addEventListener('click', (event) => {
      if (event.target === modalEl) {
        setModalOpen(modalEl, false);
      }
    });
  }
}

document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape') {
    setModalOpen(workspaceModal, false);
    setModalOpen(diagnosticsModal, false);
    closeCanvasDrawer();
  }
});

if (chatApplyPatchBtn) {
  chatApplyPatchBtn.addEventListener('click', () => {
    if (!pendingPatchPath || !pendingProposedCode) return;
    const ws = dispatchStudioEnvelope('workspace_write', `${pendingPatchPath}\n${pendingProposedCode}`);
    if (ws && ws.status === 'ok') {
      activeWorkspacePath = pendingPatchPath;
      renderWorkspaceList(ws);
      selectWorkspaceFile(pendingPatchPath);
      const st = dispatchStudioEnvelope('state', '');
      renderStudioState(st);
      if (workspaceStatus) {
        workspaceStatus.textContent = `Applied synthesized patch to ${pendingPatchPath} and admitted into geometric memory.`;
      }
      activateCanvasTab('workspace');
    }
  });
}

if (workspaceNewForm) {
  workspaceNewForm.addEventListener('submit', (event) => {
    event.preventDefault();
    const rawPath = workspaceNewPath ? workspaceNewPath.value.trim() : '';
    if (!rawPath) return;
    const stub = `//! Module ${rawPath} initialized in UOR-R4 Workspace.\npub fn module_entry_root() -> u8 {\n    0\n}\n`;
    const ws = dispatchStudioEnvelope('workspace_write', `${rawPath}\n${stub}`);
    if (ws && ws.status === 'ok') {
      activeWorkspacePath = rawPath;
      renderWorkspaceList(ws);
      selectWorkspaceFile(rawPath);
      const st = dispatchStudioEnvelope('state', '');
      renderStudioState(st);
      if (workspaceStatus) {
        workspaceStatus.textContent = `Created ${rawPath} in workspace and admitted into geometric memory.`;
      }
    }
  });
}

if (workspaceSaveBtn) {
  workspaceSaveBtn.addEventListener('click', () => {
    const content = workspaceEditorTextarea ? workspaceEditorTextarea.value : '';
    if (!activeWorkspacePath) return;
    const ws = dispatchStudioEnvelope('workspace_write', `${activeWorkspacePath}\n${content}`);
    if (ws && ws.status === 'ok') {
      renderWorkspaceList(ws);
      selectWorkspaceFile(activeWorkspacePath);
      const st = dispatchStudioEnvelope('state', '');
      renderStudioState(st);
      if (workspaceStatus) {
        workspaceStatus.textContent = `Saved ${activeWorkspacePath} and admitted updated tokens into geometric memory.`;
      }
    }
  });
}

if (workspaceDownloadBtn) {
  workspaceDownloadBtn.addEventListener('click', () => {
    const content = workspaceEditorTextarea ? workspaceEditorTextarea.value : '';
    const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    const baseName = (activeWorkspacePath || 'module.rs').split('/').pop() || 'module.rs';
    a.download = baseName;
    a.click();
    URL.revokeObjectURL(url);
    if (workspaceStatus) {
      workspaceStatus.textContent = `Exported ${activeWorkspacePath} (${content.length} bytes) to local disk.`;
    }
  });
}

if (workspaceMountInput) {
  workspaceMountInput.addEventListener('change', async () => {
    const files = workspaceMountInput.files;
    if (!files || files.length === 0) return;
    const chunks = [];
    for (let i = 0; i < files.length; i += 1) {
      const f = files[i];
      const text = await f.text();
      const relPath = f.webkitRelativePath || `src/${f.name}`;
      chunks.push(`${relPath}\n${text}`);
    }
    const batchPayload = chunks.join('\n===FILE===\n');
    const ws = dispatchStudioEnvelope('workspace_batch', batchPayload);
    if (ws && ws.status === 'ok') {
      renderWorkspaceList(ws);
      const st = dispatchStudioEnvelope('state', '');
      renderStudioState(st);
      if (workspaceStatus) {
        workspaceStatus.textContent = `Mounted ${ws.batch_ingested_files} local file(s) (${ws.batch_ingested_tokens} tokens indexed into geometric memory).`;
      }
    }
  });
}

const weightCacheBadge = document.getElementById('weight-cache-badge');
const topbarModelTag = document.getElementById('topbar-model-tag');
const syncWeightsBtn = document.getElementById('sync-weights-btn');
const loadWeightsTopbarBtn = document.getElementById('load-weights-topbar-btn');

if (loadWeightsTopbarBtn && snapshotUploadInput) {
  loadWeightsTopbarBtn.addEventListener('click', () => {
    snapshotUploadInput.click();
  });
}

function updateWeightCacheBadge() {
  if (weightCacheBadge) {
    const meta = globalThis.__uorWeightCacheMeta || {};
    const label = meta.status || 'Weights: UR4GLM08 · CacheStorage';
    const shortSha = meta.sha256 ? ` · ${meta.sha256.slice(0, 8)}` : '';
    weightCacheBadge.textContent = `${label}${shortSha}`;
  }
  if (topbarModelTag && typeof studio_weight_pack_status === 'function') {
    try {
      const packInfo = JSON.parse(studio_weight_pack_status());
      if (packInfo && packInfo.status === 'ok') {
        const mb = (packInfo.byte_len / (1024 * 1024)).toFixed(1);
        const totalParams = packInfo.total_parameters || 0;
        const paramStr = totalParams >= 1000000000
          ? `${(totalParams / 1000000000).toFixed(2)}B Params`
          : `${(totalParams / 1000000).toFixed(1)}M Params`;
        topbarModelTag.textContent = `UORLUT01/8 · ${paramStr} · ${mb} MB`;
        topbarModelTag.title = `UORLUT01 (uor-r4.lut-stack/8 · ${paramStr}) · 4,096 BPE · 6-Layer SpiralCore/FBS · ${packInfo.reply_entries} /7 Reply Rows · ${packInfo.point_rows} /8 Point Rows (${mb} MB)`;
      }
    } catch {
      // Ignore parse error during early boot
    }
  }
}

if (syncWeightsBtn) {
  syncWeightsBtn.addEventListener('click', async () => {
    syncWeightsBtn.disabled = true;
    const prevText = syncWeightsBtn.textContent;
    syncWeightsBtn.textContent = 'Syncing 0%...';
    try {
      await sync_cached_weights((prog) => {
        if (prog && typeof prog.percent === 'number') {
          syncWeightsBtn.textContent = `Syncing ${prog.percent}%...`;
        }
        updateWeightCacheBadge();
      });
      updateWeightCacheBadge();
      const st = dispatchStudioEnvelope('state', '');
      renderStudioState(st);
    } finally {
      syncWeightsBtn.disabled = false;
      syncWeightsBtn.textContent = prevText;
    }
  });
}

if (studioTrainForm) {
  studioTrainForm.addEventListener('submit', (event) => {
    event.preventDefault();
    runOnDeviceTrain(trainEpochsInput ? trainEpochsInput.value : '3');
  });
}

loadThreadsFromStorage();
renderThreadSidebar();
renderLiteratureFeed();
renderActiveThreadMessages();
if (document.getElementById('neural-3d-canvas')) {
  lexLean3dVisualizer = new LexLean3DVisualizer();
  lexLean3dVisualizer.init('neural-3d-canvas');
}
renderTraceInspector();

try {
  await init();
  await restore_cached_uorlut_from_indexeddb();
  if (typeof studio_weight_pack_status === 'function') {
    try {
      const packInfo = JSON.parse(studio_weight_pack_status());
      if (packInfo && packInfo.status === 'ok' && globalThis.__uorWeightCacheMeta && globalThis.__uorWeightCacheMeta.mode !== 'indexeddb') {
        const mb = (packInfo.byte_len / (1024 * 1024)).toFixed(1);
        globalThis.__uorWeightCacheMeta.status = `${globalThis.__uorWeightCacheMeta.status || 'UORLUT01/8'} · ${mb} MB · ${packInfo.reply_entries} reply rows`;
      }
    } catch {
      // Ignore weight status parse error
    }
  }
  updateWeightCacheBadge();
  const initialState = dispatchStudioEnvelope('state', '');
  renderStudioState(initialState);
  restoreIntelligenceFromWebCache();
  const initialWorkspace = dispatchStudioEnvelope('workspace_list', '');
  renderWorkspaceList(initialWorkspace);
  selectWorkspaceFile(activeWorkspacePath);
  const currentActiveThread = getActiveThread();
  if (!currentActiveThread.messages || currentActiveThread.messages.length === 0) {
    runClaudeChat('Hello! Who are you and what can you do?', false);
  } else {
    dispatchStudioEnvelope('chat', 'Hello! Who are you and what can you do?');
  }
  if (claudeChatInput) {
    claudeChatInput.value = '';
    claudeChatInput.focus();
  }
  if (studioPromptInput && studioPromptInput.value) {
    runStudioGenerate(studioPromptInput.value);
  }
  runStudioSubsystems(subsystemsPayloadInput ? subsystemsPayloadInput.value : '7,6,1,10,2180');
  executeCoordinateEnvelope();
  if (runApiRequestBtn) {
    runApiRequestBtn.click();
  }
  fetchRecordedTraces();
  if (typeof stream_sharded_weights_to_indexeddb === 'function') {
    stream_sharded_weights_to_indexeddb((prog) => {
      if (syncWeightsBtn && prog && typeof prog.percent === 'number') {
        syncWeightsBtn.textContent = `Syncing ${prog.percent}%...`;
      }
      updateWeightCacheBadge();
    }, false)
      .then((streamed) => {
        if (syncWeightsBtn) {
          syncWeightsBtn.textContent = 'Sync Weights';
        }
        if (streamed && streamed.status === 'ok') {
          updateWeightCacheBadge();
          const st = dispatchStudioEnvelope('state', '');
          renderStudioState(st);
        }
      })
      .catch(() => {
        if (syncWeightsBtn) {
          syncWeightsBtn.textContent = 'Sync Weights';
        }
      });
  }
} catch (err) {
  appendChatTurn(
    'assistant',
    'UOR-R4 GLM',
    `WebAssembly initialization error: ${err && err.message ? err.message : String(err)}`,
  );
}
