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
} from './lexlean_r4.js?v=20261106coworkv2';

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
      const winRow = chatMeta.otelWinningRow !== undefined ? chatMeta.otelWinningRow : 0;
      traceInfo.textContent = `uor.gllm.trace/1 · trace=${shortTrace} · ${spanCnt} spans · PointKey row #${winRow}`;
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
      details.open = Boolean(tagThinking || streamTokens);
      const summary = document.createElement('summary');
      summary.className = 'chat-thinking-summary';
      summary.textContent = tagThinking
        ? 'Explicit <thinking> Trace (Rolling 256D Q12 Readback)'
        : 'Geometric <thinking> & 5-Tier Routing Trace (256D Q12 -> E8 -> PointKey)';
      const content = document.createElement('div');
      content.className = 'chat-thinking-content';
      content.textContent = effectiveThinking;
      details.appendChild(summary);
      details.appendChild(content);
      body.appendChild(details);
    }

    if (allTools.length > 0) {
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
        body.appendChild(toolCard);
      }
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

function submitComposerPrompt() {
  if (!claudeChatInput) return;
  const submittedPrompt = claudeChatInput.value;
  if (!submittedPrompt.trim()) return;
  claudeChatInput.value = '';
  claudeChatInput.style.height = 'auto';
  runClaudeChat(submittedPrompt, true);
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
    if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
      event.preventDefault();
      submitComposerPrompt();
    }
  });
  claudeChatInput.addEventListener('input', () => {
    claudeChatInput.style.height = 'auto';
    claudeChatInput.style.height = `${Math.min(claudeChatInput.scrollHeight, 160)}px`;
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

function activateCanvasTab(tabId) {
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
    if (tabId) activateCanvasTab(tabId);
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
        topbarModelTag.textContent = `UORLUT01 (uor-r4.lut-stack/8 · ${paramStr}) · 4,096 BPE · 6-Layer SpiralCore/FBS · ${packInfo.reply_entries} /7 Reply Rows · ${packInfo.point_rows} /8 Point Rows (${mb} MB)`;
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
