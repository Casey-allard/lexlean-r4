import init, { process_envelope } from './lexlean_r4.js';

const form = document.getElementById('application-form');
const left = document.getElementById('left');
const right = document.getElementById('right');
const operation = document.getElementById('operation');
const result = document.getElementById('result');

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
  result.textContent = message;
}

await init();

form.addEventListener('submit', (event) => {
  event.preventDefault();
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
});
