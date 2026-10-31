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
const admitStatus = document.getElementById('admit-status');

const statSchema = document.getElementById('stat-schema');
const statVocab = document.getElementById('stat-vocab');
const statTurnRows = document.getElementById('stat-turn-rows');
const statPointRows = document.getElementById('stat-point-rows');
const statCells = document.getElementById('stat-cells');
const statStateReceipt = document.getElementById('stat-state-receipt');

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

for (const btn of document.querySelectorAll('.preset-btn')) {
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
    if (admitStatus) {
      admitStatus.textContent = 'Reset geometric memory to the 12 default UOR-R4 turns.';
    }
    if (studioPromptInput) {
      runStudioGenerate(studioPromptInput.value);
    }
  });
}

if (studioSubsystemsForm) {
  studioSubsystemsForm.addEventListener('submit', (event) => {
    event.preventDefault();
    runStudioSubsystems(subsystemsPayloadInput ? subsystemsPayloadInput.value : '');
  });
}

const initialState = dispatchStudioEnvelope('state', '');
renderStudioState(initialState);
if (studioPromptInput && studioPromptInput.value) {
  runStudioGenerate(studioPromptInput.value);
}
runStudioSubsystems(subsystemsPayloadInput ? subsystemsPayloadInput.value : '7,6,1,10,2180');
executeCoordinateEnvelope();
