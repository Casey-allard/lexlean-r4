// Node E2E verification for R4-72 & R4-73: UORR4V14 4D Signed Hopf, Golden-Ratio Poincare,
// 64-Ordinate C2 Riemann Zeta-Zero Ring, 120-Vertex H4 Icosian Snap, and 10,000+ tok/s Engine.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '..', '..');

const wasmBytes = fs.readFileSync(path.join(rootDir, 'public', 'lexlean_r4_bg.wasm'));
const wasmJsUrl = pathToFileURL(path.join(rootDir, 'public', 'lexlean_r4.js')).href;
const wasmMod = await import(wasmJsUrl);
await wasmMod.default(wasmBytes);

const selfCheck = JSON.parse(wasmMod.studio_hopf_zeta_v14_self_check_json());
assert.equal(selfCheck.spec, 'uor-hopf-zeta-v14/1');
assert.equal(selfCheck.r4_72_lexlean_admitted, true);
assert.equal(selfCheck.r4_73_lexlean_admitted, true);
assert.equal(selfCheck.magic, 'UORR4V14');
assert.equal(selfCheck.icosian_vertices, 120);
assert.equal(selfCheck.zeta_ordinates, 64);
assert.equal(selfCheck.signed_orthants, 16);
assert.equal(selfCheck.storage_angle_preserved, true);
assert.equal(selfCheck.float_ops, 0);
assert.equal(selfCheck.dense_matmul_ops, 0);
assert.equal(selfCheck.softmax_ops, 0);
assert.equal(selfCheck.icosian_drift_lsb, 0);
assert.ok(selfCheck.active_basin_kb <= 1024);
assert.ok(selfCheck.bandwidth_ceiling_tps >= 320000);

// Warm up Wasm JIT once, then measure sustained pure-integer Wasm simd128 wave throughput over 8,192 tokens!
wasmMod.studio_hopf_zeta_v14_generate_json('Canonical Poincare ball Hopf fibration', 64);
const t0 = performance.now();
const benchOut = JSON.parse(
  wasmMod.studio_hopf_zeta_v14_generate_json(
    'Golden ratio Poincare ball Hopf fibration Zeta zero token ring Icosian snap',
    8192,
  ),
);
const elapsedMs = Math.max(0.05, performance.now() - t0);
const wasmMeasuredTokPerSec = Math.round((benchOut.receipt.tokens_generated * 1000) / elapsedMs);

assert.equal(benchOut.ok, true);
assert.equal(benchOut.receipt.tokens_generated, 8192);
assert.equal(benchOut.receipt.icosian_drift_lsb, 0);
assert.ok(
  wasmMeasuredTokPerSec >= 10000,
  `expected >= 10,000 tok/s in Wasm simd128, measured ${wasmMeasuredTokPerSec} tok/s (${elapsedMs.toFixed(2)} ms for 8,192 tok)`,
);

// Verify live 3D PBR Observatory renders the UORR4V14 S2 Hopf trajectory trail seamlessly
const trailCsv = benchOut.receipt.hopf_s2_trail_milli
  .slice(0, 16)
  .map((pt) => `${pt[0]},${pt[1]},${pt[2]}`)
  .join(';');
const obsRgba = wasmMod.studio_render_hopf_zeta_3d_rgba(128, 96, 200, 2, trailCsv);
assert.equal(obsRgba.length, 128 * 96 * 4);

// Verify pure-integer WGSL shader source for WebGPU parallel wave casting is exposed
assert.equal(typeof globalThis.__uorSelfHostRuntime.hopfZetaV14WgslShader, 'string');
assert.ok(!globalThis.__uorSelfHostRuntime.hopfZetaV14WgslShader.includes('f32'));

// Verify Worker turn envelope for `/v14` and `/qpc` returns `result.status === 'ok'` (prevents Chat error regression!)
const dummyHooks = {
  postEvent: () => {},
  beginTurn: () => '{}',
  stepTurn: () => '{}',
};
const v14Turn = await globalThis.__uorSelfHostRuntime.executeInprismFoundryTurn(
  { id: 't-v14', payload: { prompt: '/v14 Explain Hopf Poincare Zeta zero attention' } },
  dummyHooks,
);
assert.equal(v14Turn.ok, true);
assert.equal(v14Turn.result.status, 'ok');
assert.equal(v14Turn.result.tokens_emitted, 4096);
assert.ok(v14Turn.result.reply.includes('UORR4V14 Hopf–Poincaré–Zeta–Icosian Wave Receipt'));

const qpcTurn = await globalThis.__uorSelfHostRuntime.executeInprismFoundryTurn(
  { id: 't-qpc', payload: { prompt: '/qpc' } },
  dummyHooks,
);
assert.equal(qpcTurn.ok, true);
assert.equal(qpcTurn.result.status, 'ok');
assert.ok(qpcTurn.result.reply.includes('Foundry Quantum PC'));

console.log('R4-72 & R4-73 UORR4V14 10,000+ tok/s Hopf-Poincare-Zeta-Icosian E2E: PASS', {
  wasmMeasuredTokPerSec,
  elapsedMs: Number(elapsedMs.toFixed(2)),
  tokensGenerated: benchOut.receipt.tokens_generated,
  activeBasinKB: benchOut.receipt.active_basin_kb,
  icosianDriftLsb: benchOut.receipt.icosian_drift_lsb,
});
