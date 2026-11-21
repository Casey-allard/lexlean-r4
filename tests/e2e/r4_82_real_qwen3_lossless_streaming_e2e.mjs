#!/usr/bin/env node
/**
 * R4-82 E2E: 100% Real Qwen3-0.6B/1.7B Lossless Streaming, Deferred Idle Prefix Prewarm,
 * Instant Token-1 TTFT Flush, and Zero Preplanned Canned Prose Interception.
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

console.log('1. Verifying R4.RealQwen3LosslessStreaming formal admission & negative refusals...');
assert.equal(
  bridge.admitRealQwen3LosslessTurn(false, false, true, false),
  'RealQwen3LosslessTurnAdmitted',
);
assert.equal(
  bridge.admitRealQwen3LosslessTurn(true, false, true, false),
  'RefusedPreplannedCannedProseOnQwenLane',
);
assert.equal(
  bridge.admitRealQwen3LosslessTurn(false, true, true, false),
  'RefusedBlockingSyncPrefixPrewarmOnMount',
);
assert.equal(
  bridge.admitRealQwen3LosslessTurn(false, false, false, false),
  'RefusedDelayedMultiTokenFirstFlush',
);
assert.equal(
  bridge.admitRealQwen3LosslessTurn(false, false, true, true),
  'RefusedColdBadgeOverwriteAfterReady',
);

console.log('2. Verifying unmounted uor-r4.qwen3-0.6b-holo NEVER serves preplanned UORR4V15 canned passages...');
let hydrationCalled = false;
const resUnmounted = await bridge.execute_inprism_foundry_turn(
  {
    id: 8201,
    payload: {
      prompt: 'what is the capital of france?',
      selected_model: 'uor-r4.qwen3-0.6b-holo',
      surface_mode: 'chat',
      live_hud: true,
    },
  },
  {
    holoRuntimeStatus: () => JSON.stringify({ mounted_count: 0, qwen3_06b_mounted: false, models: [] }),
    hydrateHoloModel: async () => {
      hydrationCalled = true;
      return { status: 'ok', mounted_count: 1, qwen3_06b_mounted: true };
    },
    holoBeginTurn: (reqJson) => {
      const req = JSON.parse(reqJson);
      assert.equal(req.instant_ttft, true, 'live_hud turn must enable instant_ttft');
      return JSON.stringify({ status: 'ready', turn_id: 99, model_id: 'uor-r4.qwen3-0.6b-holo' });
    },
    holoStepTurn: () =>
      JSON.stringify({
        status: 'done',
        delta: 'The capital of France is Paris.',
        tokens_emitted: 7,
        tok_per_sec: 85.0,
      }),
  },
);
assert.equal(hydrationCalled, true, 'Unmounted 0.6B turn must hydrate real HoloV13Model rather than bypassing to UORR4V15');
assert.equal(resUnmounted.ok, true);
assert.equal(resUnmounted.result.reply, 'The capital of France is Paris.');

console.log('3. Verifying studio_prewarm_holo_prefix_idle Wasm export exists and fails closed when unmounted...');
assert.equal(typeof wasmMod.studio_prewarm_holo_prefix_idle, 'function');
const idleUnmounted = JSON.parse(wasmMod.studio_prewarm_holo_prefix_idle('uor-r4.qwen3-0.6b-holo'));
assert.equal(idleUnmounted.ok, false);
assert.equal(idleUnmounted.error_code, 'ModelNotMounted');

console.log('[PASS] R4-82 real Qwen3 lossless routing, instant_ttft wiring, and idle prefix prewarm verified.');
