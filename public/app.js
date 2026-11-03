import init, { process_envelope, studio_dispatch } from './lexlean_r4.js';

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
  const fenceRegex = /```([a-zA-Z0-9_-]*)\n([\s\S]*?)```/g;
  let lastIdx = 0;
  let match = fenceRegex.exec(markdownText);
  while (match !== null) {
    if (match.index > lastIdx) {
      renderProseBlockInto(container, markdownText.slice(lastIdx, match.index));
    }
    const lang = (match[1] || 'code').trim();
    const codeBody = match[2] || '';
    const wrapper = document.createElement('div');
    wrapper.className = 'chat-code-wrapper';
    const langBar = document.createElement('div');
    langBar.className = 'chat-code-lang';
    langBar.textContent = lang.toUpperCase();
    const pre = document.createElement('pre');
    pre.className = 'chat-code-block';
    const codeEl = document.createElement('code');
    codeEl.textContent = codeBody.replace(/\n$/, '');
    pre.appendChild(codeEl);
    wrapper.appendChild(langBar);
    wrapper.appendChild(pre);
    container.appendChild(wrapper);
    lastIdx = fenceRegex.lastIndex;
    match = fenceRegex.exec(markdownText);
  }
  if (lastIdx < markdownText.length) {
    renderProseBlockInto(container, markdownText.slice(lastIdx));
  }
}

function appendChatTurn(role, titleText, bodyText, metaRight = '') {
  if (!chatTranscript) return;
  const card = document.createElement('div');
  card.className = `chat-turn chat-turn-${role}`;

  const meta = document.createElement('div');
  meta.className = 'chat-turn-meta';
  const leftSpan = document.createElement('span');
  leftSpan.textContent = titleText;
  const rightSpan = document.createElement('span');
  rightSpan.textContent = metaRight;
  meta.appendChild(leftSpan);
  meta.appendChild(rightSpan);

  const body = document.createElement('div');
  body.className = 'chat-turn-body';
  if (role === 'assistant') {
    renderFormattedMarkdownInto(body, bodyText);
  } else {
    body.style.whiteSpace = 'pre-wrap';
    body.textContent = bodyText;
  }

  card.appendChild(meta);
  card.appendChild(body);
  chatTranscript.appendChild(card);
  chatTranscript.scrollTop = chatTranscript.scrollHeight;
}

function runClaudeChat(promptText) {
  const trimmed = (promptText || '').trim();
  if (!trimmed) return;
  appendChatTurn('user', 'You', trimmed, `Context: ${activeWorkspacePath}`);

  const chatRes = dispatchStudioEnvelope('chat', trimmed);
  if (!chatRes || chatRes.status !== 'ok') {
    appendChatTurn('assistant', 'UOR-R4 Engine', chatRes && chatRes.message ? chatRes.message : 'Chat error');
    return;
  }

  const gen = chatRes.generation || {};
  const replyProse = chatRes.assistant_reply || gen.reply || 'Synthesized geometric response.';
  const headSummary = Array.isArray(chatRes.flock_heads)
    ? chatRes.flock_heads.map((h) => `H${h.head}->F${h.cyclic_frame_block}`).join(' ')
    : '';
  const assistantTitle = chatRes.is_coding_turn
    ? `UOR-R4 Code Synthesizer (${chatRes.target_file})`
    : `UOR-R4 Generalized Assistant`;
  appendChatTurn(
    'assistant',
    assistantTitle,
    `${replyProse}\n\n[8-Head Cyclic Routing: ${headSummary}]`,
    `Cl(${chatRes.flock_consensus_action}) | Root #${chatRes.icosian_root}`
  );

  if (chatRes.is_coding_turn) {
    pendingPatchPath = chatRes.target_file;
    pendingProposedCode = chatRes.proposed_code || '';
    if (chatTargetBadge) chatTargetBadge.textContent = `Patch Target: ${chatRes.target_file}`;
    if (chatDiffFile) chatDiffFile.textContent = chatRes.target_file;
    if (chatDiffPreview) chatDiffPreview.textContent = chatRes.diff_preview || '';
    if (chatApplyPatchBtn) {
      chatApplyPatchBtn.disabled = false;
      chatApplyPatchBtn.textContent = `Apply Synthesized Patch to ${chatRes.target_file} & Admit into Memory`;
    }
  } else {
    if (chatTargetBadge) chatTargetBadge.textContent = `Active File: ${activeWorkspacePath}`;
    if (!pendingProposedCode) {
      if (chatDiffFile) chatDiffFile.textContent = activeWorkspacePath || 'No patch staged';
      if (chatDiffPreview) {
        chatDiffPreview.textContent = 'No code patch staged yet. Ask the assistant to write or refactor code (e.g. "Write a function in Python to reverse a string" or "Upgrade multi-head cyclic flock attention in src/attention.rs") to stage a unified diff here.';
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

await init();

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

if (admitResetBtn) {
  admitResetBtn.addEventListener('click', () => {
    const state = dispatchStudioEnvelope('reset', '');
    renderStudioState(state);
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
      if (admitStatus) {
        admitStatus.textContent = `Exported ${snap.byte_length}-byte UORLUT01/8 + UR4SESS1 binary envelope (${count} turns, SHA-256: ${snap.snapshot_sha256})`;
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
      const ws = dispatchStudioEnvelope('workspace_list', '');
      renderWorkspaceList(ws);
      if (admitStatus) {
        admitStatus.textContent = `Imported UORLUT01/8 + UR4SESS1 + UR4WGHT1 snapshot (${state.turn_count} turns, SHA-256: ${state.state_receipt_sha256})`;
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
    const file = snapshotUploadInput.files && snapshotUploadInput.files[0];
    if (!file) return;
    const arrayBuf = await file.arrayBuffer();
    const bytes = new Uint8Array(arrayBuf);
    let hexPayload = '';
    if (bytes.length >= 8 && bytes[0] === 0x55 && bytes[1] === 0x4f && bytes[2] === 0x52) {
      hexPayload = bytesToHex(bytes);
    } else {
      hexPayload = decoder.decode(bytes).trim();
    }
    if (snapshotHexInput) snapshotHexInput.value = hexPayload;
    const state = dispatchStudioEnvelope('import_snapshot', hexPayload);
    if (state && state.status === 'ok') {
      renderStudioState(state);
      const ws = dispatchStudioEnvelope('workspace_list', '');
      renderWorkspaceList(ws);
      if (admitStatus) {
        admitStatus.textContent = `Loaded pre-trained weight pack ${file.name} (${state.turn_count} turns, ${state.training_epoch} epochs, ${state.active_nonzero_gates} gates)`;
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

if (claudeChatForm) {
  claudeChatForm.addEventListener('submit', (event) => {
    event.preventDefault();
    if (claudeChatInput) {
      const submittedPrompt = claudeChatInput.value;
      claudeChatInput.value = '';
      runClaudeChat(submittedPrompt);
    }
  });
}

if (chatClearBtn) {
  chatClearBtn.addEventListener('click', () => {
    if (chatTranscript) {
      chatTranscript.replaceChildren();
    }
    if (claudeChatInput) {
      claudeChatInput.value = '';
    }
    runClaudeChat('Hello! Who are you and what can you do?');
  });
}

for (const btn of document.querySelectorAll('.chat-preset-btn')) {
  btn.addEventListener('click', () => {
    const promptVal = btn.getAttribute('data-chat-prompt') || '';
    if (claudeChatInput) {
      claudeChatInput.value = '';
    }
    runClaudeChat(promptVal);
  });
}

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

if (studioTrainForm) {
  studioTrainForm.addEventListener('submit', (event) => {
    event.preventDefault();
    runOnDeviceTrain(trainEpochsInput ? trainEpochsInput.value : '3');
  });
}

const initialState = dispatchStudioEnvelope('state', '');
renderStudioState(initialState);
const initialWorkspace = dispatchStudioEnvelope('workspace_list', '');
renderWorkspaceList(initialWorkspace);
selectWorkspaceFile(activeWorkspacePath);
runClaudeChat('Hello! Who are you and what can you do?');
if (claudeChatInput) {
  claudeChatInput.value = '';
}
if (studioPromptInput && studioPromptInput.value) {
  runStudioGenerate(studioPromptInput.value);
}
runStudioSubsystems(subsystemsPayloadInput ? subsystemsPayloadInput.value : '7,6,1,10,2180');
executeCoordinateEnvelope();
