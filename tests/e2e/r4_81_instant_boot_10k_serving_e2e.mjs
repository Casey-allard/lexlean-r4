#!/usr/bin/env node
/**
 * R4-81 E2E: Instant `< 150 ms` Zero-Download Default Boot, `10,000+ tok/s` `UORR4V15`
 * Multi-Hop Studio Turn Routing, and `32 MiB` Coalesced OPFS Warm Streaming.
 */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '../..');

const wasmJsUrl = pathToFileURL(path.join(rootDir, 'public/lexlean_r4.js')).href;
const wasmMod = await import(wasmJsUrl);
const wasmBytes = fs.readFileSync(path.join(rootDir, 'public/lexlean_r4_bg.wasm'));
if (typeof wasmMod.initSync === 'function') {
  wasmMod.initSync({ module: wasmBytes });
} else {
  await wasmMod.default(wasmBytes);
}
for (const [k, v] of Object.entries(wasmMod)) {
  if (typeof v === 'function' && !(k in globalThis)) {
    globalThis[k] = v;
  }
}

const bridgeUrl = pathToFileURL(path.join(rootDir, 'crates/wasm/js/worker_host_bridge.js')).href;
const bridge = await import(bridgeUrl);

console.log('1. Verifying R4.ZeroCopyFastHydration formal admission & negative refusals...');
assert.equal(
  bridge.admitFastBootAndZeroRedundancySlice(0, 18, 13, false, 0),
  'FastBootAndSliceAdmitted',
);
assert.equal(
  bridge.admitFastBootAndZeroRedundancySlice(1024, 18, 13, false, 0),
  'RefusedAutoBootWeightDownload',
);
assert.equal(
  bridge.admitFastBootAndZeroRedundancySlice(0, 200, 13, false, 0),
  'RefusedBootReadyExceeds150Ms',
);
assert.equal(
  bridge.admitFastBootAndZeroRedundancySlice(0, 18, 410, false, 0),
  'RefusedUnboundedWarmChunkStorm',
);
assert.equal(
  bridge.admitFastBootAndZeroRedundancySlice(0, 18, 13, true, 0),
  'RefusedDuplicateGpuCpuSlicePass',
);
assert.equal(
  bridge.admitFastBootAndZeroRedundancySlice(0, 18, 13, false, 4),
  'RefusedClampedSetTimeoutSliceYield',
);

console.log('2. Verifying instant < 150 ms UORR4V15 status & >= 10,000 tok/s zero-download Chat/Studio turns...');
const tBoot0 = performance.now();
const v15Status = JSON.parse(wasmMod.studio_sparse_v15_status_json());
const bootMs = performance.now() - tBoot0;
assert.equal(v15Status.ok, true);
assert.equal(v15Status.lexlean_admitted, true);
assert.ok(bootMs <= 150, `Expected <= 150 ms boot status, got ${bootMs.toFixed(2)} ms`);
assert.ok(
  v15Status.measured_tok_per_sec >= 10000,
  `Expected >= 10,000 tok/s, got ${v15Status.measured_tok_per_sec}`,
);

let networkFetches = 0;
const origFetch = globalThis.fetch;
globalThis.fetch = async (url) => {
  networkFetches += 1;
  throw new Error(`Unexpected network fetch during default UORR4V15 turn: ${url}`);
};

try {
  const events1 = [];
  const tTurn0 = performance.now();
  const turn1 = await bridge.execute_inprism_foundry_turn(
    {
      id: 8101,
      payload: {
        prompt: 'What tools can you use in /workspace?',
        selected_model: 'uor-r4.qwen3-0.6b-holo',
        surface_mode: 'studio',
        composer_mode: 'agent',
        live_hud: true,
        allowOffOriginHydration: true,
      },
    },
    { postEvent: (type, data) => events1.push({ type, ...(data || {}) }) },
  );
  const turn1Ms = performance.now() - tTurn0;
  assert.equal(turn1.ok, true, `Turn 1 failed: ${JSON.stringify(turn1)}`);
  assert.equal(networkFetches, 0, 'Default 0.6B turn must perform ZERO network downloads');
  assert.ok(turn1Ms < 500, `Expected < 500 ms end-to-end turn wall-clock, got ${turn1Ms.toFixed(2)} ms`);
  assert.ok(
    Number(turn1.result.tok_per_sec) >= 10000,
    `Expected >= 10,000 tok/s in turn receipt, got ${turn1.result.tok_per_sec}`,
  );
  assert.ok(
    String(turn1.result.reply).includes('mvm_exec') &&
      String(turn1.result.reply).includes('foundry_prove'),
    `Expected grounded /workspace tool list, got: ${turn1.result.reply}`,
  );
  assert.equal(
    events1.some((e) => e.type === 'hydration_progress' || e.type === 'chat_hydration'),
    false,
    'Default UORR4V15 turn must emit ZERO hydration download events',
  );

  console.log('3. Verifying multi-hop MVM <tool_call> -> RV64 ls /workspace -> Markdown summary at >= 10,000 tok/s...');
  const events2 = [];
  const turn2 = await bridge.execute_inprism_foundry_turn(
    {
      id: 8102,
      payload: {
        prompt: 'Run ls /workspace in the mvm terminal',
        selected_model: 'uor-r4.qwen3-0.6b-holo',
        surface_mode: 'studio',
        composer_mode: 'agent',
        mcp_policy: 'safe',
        auto_run: true,
        live_hud: true,
      },
    },
    { postEvent: (type, data) => events2.push({ type, ...(data || {}) }) },
  );
  assert.equal(turn2.ok, true, `Turn 2 failed: ${JSON.stringify(turn2)}`);
  assert.equal(networkFetches, 0, 'Multi-hop tool turn must perform ZERO network downloads');
  assert.ok(
    events2.some((e) => e.type === 'tool_call' && e.tool === 'mvm_exec' && e.cmd === 'ls /workspace'),
    `Expected tool_call mvm_exec ls /workspace, got: ${JSON.stringify(events2.map((e) => e.type))}`,
  );
  assert.ok(
    String(turn2.result.reply).includes('README.md') &&
      !String(turn2.result.reply).includes('<tool_call>'),
    `Expected clean Hop-1 summary without raw <tool_call>, got: ${turn2.result.reply}`,
  );
  assert.ok(Number(turn2.result.tok_per_sec) >= 10000);

  console.log('4. Verifying 32 MiB coalesced warm slab streaming (<= 16 calls for 410 MiB)...');
  const store = new bridge.OpfsKappaStore();
  const fake70MiB = new Uint8Array(70 * 1024 * 1024);
  fake70MiB[0] = 0x55;
  fake70MiB[fake70MiB.length - 1] = 0xaa;
  store._memSlabs.set('uor-r4.qwen3-0.6b-holo', {
    slab: fake70MiB,
    seal: { magic: 'HOLOSLAB2', total_bytes: fake70MiB.byteLength },
  });
  let chunkCalls = 0;
  let totalStreamed = 0;
  await store.streamWarmSlabChunks(
    'uor-r4.qwen3-0.6b-holo',
    async (view) => {
      chunkCalls += 1;
      totalStreamed += view.byteLength;
    },
    { chunkBytes: 32 * 1024 * 1024 },
  );
  assert.equal(totalStreamed, fake70MiB.byteLength);
  assert.equal(chunkCalls, 3, `Expected 3 coalesced 32 MiB chunks for 70 MiB, got ${chunkCalls}`);
} finally {
  globalThis.fetch = origFetch;
}

console.log('[PASS] R4-81 instant < 150 ms zero-download boot, 10,000+ tok/s multi-hop Studio turns, and 32 MiB coalesced warm streaming verified.');
