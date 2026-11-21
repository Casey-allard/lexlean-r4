#!/usr/bin/env node
/**
 * Formal R4.ResidentGpuHybridDrafter E2E Suite (R4-75):
 * 1. Verifies formal admission & negative refusals for zero-download resident turns and hybrid V14 WebGPU drafting.
 * 2. Verifies `mountResidentModelSlabsFromExporter` rejects 64-byte placeholder slabs and binds distinct per-layer GPUBuffers across all 28 layers.
 * 3. Verifies `execute_inprism_foundry_turn` performs 0 downloads on resident models, skips `mapAsync` stalls during prefill slices, and runs bound GPU passes on decode slices.
 * 4. Verifies content-addressed `?v=v12r4-<js_sha12>` query stamping and honest `Resident · Generating (0 B dl)` UI headers.
 */

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '../..');

const bridge = await import(path.join(rootDir, 'crates/wasm/js/worker_host_bridge.js'));
const {
  WebGpuHoloEngine,
  admitResidentTurnZeroDownload,
  admitHybridV14WebGpuDraft,
  execute_inprism_foundry_turn,
} = bridge;

function createMockGpu() {
  const stats = {
    buffersCreated: 0,
    bindGroupsCreated: 0,
    boundLayerBuffers: new Set(),
    setBindGroupCalls: 0,
    dispatchCalls: 0,
    mapAsyncCalls: 0,
  };
  const device = {
    createBuffer({ size, usage }) {
      const id = ++stats.buffersCreated;
      const backing = new Uint8Array(Math.max(16, size));
      return {
        id,
        size,
        usage,
        destroy() {},
        async mapAsync() {
          stats.mapAsyncCalls += 1;
        },
        getMappedRange() {
          return backing.buffer;
        },
        unmap() {},
      };
    },
    createComputePipeline() {
      return {
        getBindGroupLayout() {
          return { __layout: true };
        },
      };
    },
    createBindGroup({ entries }) {
      stats.bindGroupsCreated += 1;
      for (const e of entries) {
        if (e && e.resource && e.resource.buffer && e.resource.buffer.size >= 4096) {
          stats.boundLayerBuffers.add(e.resource.buffer.id);
        }
      }
      return { __validGroup: true, entries };
    },
    createCommandEncoder() {
      return {
        beginComputePass() {
          let bound = false;
          return {
            setPipeline() {},
            setBindGroup(_idx, bg) {
              assert.ok(bg && bg.__validGroup, 'valid GPUBindGroup required');
              bound = true;
              stats.setBindGroupCalls += 1;
            },
            dispatchWorkgroups() {
              assert.ok(bound, 'must bind before dispatch');
              stats.dispatchCalls += 1;
            },
            end() {},
          };
        },
        copyBufferToBuffer() {},
        finish() {
          return {};
        },
      };
    },
    queue: {
      writeBuffer() {},
      submit() {},
    },
  };
  return { device, stats };
}

// 1. Formal R4-75 admission & negative gates
assert.equal(admitResidentTurnZeroDownload(true, false, 0, 0, true, true, false), 'ResidentTurnZeroDownloadAdmitted');
assert.equal(admitResidentTurnZeroDownload(true, true, 512, 0, true, true, false), 'RefusedHydrationOnResidentModel');
assert.equal(admitResidentTurnZeroDownload(true, false, 0, 0, false, true, false), 'RefusedDesyncedModelSelectors');
assert.equal(admitResidentTurnZeroDownload(true, false, 0, 0, true, false, false), 'RefusedStaleStaticAssetQuery');
assert.equal(admitResidentTurnZeroDownload(true, false, 0, 0, true, true, true), 'RefusedMisleadingStreamingLabel');

assert.equal(admitHybridV14WebGpuDraft(true, 16384, 28, 4, 0, 0, false), 'HybridV14WebGpuDraftAdmitted');
assert.equal(admitHybridV14WebGpuDraft(false, 0, 28, 4, 0, 0, false), 'HybridV14WasmSimdFallbackAdmitted');
assert.equal(admitHybridV14WebGpuDraft(true, 64, 28, 4, 0, 0, false), 'RefusedPlaceholderZeroLayerSlab');
assert.equal(admitHybridV14WebGpuDraft(true, 16384, 28, 4, 0, 0, true), 'RefusedPrefillMapAsyncStall');
assert.equal(admitHybridV14WebGpuDraft(true, 16384, 28, 4, 1, 0, false), 'RefusedFloatOrNonZeroVerifierDrift');

// 2. Real 28-layer slab streaming & 64-byte placeholder refusal
const eng = new WebGpuHoloEngine();
const { device, stats } = createMockGpu();
eng.device = device;
eng.deviceMaxStorageBinding = 134_217_728;
for (const name of ['holo_rmsnorm_rht32_pack', 'holo_int4_gemv_gemm', 'holo_rope_flock_attn', 'holo_hamming512_lmhead']) {
  eng.pipelines.set(name, device.createComputePipeline());
}
eng.residentPipelines.set('holo_swiglu_geodesic_topk', device.createComputePipeline());

const refusedPlaceholder = eng.mountResidentModelSlabsFromExporter(
  'uor-r4.qwen3-0.6b-holo',
  { admitted: true, n_layers: 28, d_model: 1024, layer_bytes: Array.from({ length: 28 }, () => 64) },
  () => new Uint8Array(64),
);
assert.equal(refusedPlaceholder.ok, false);
assert.equal(refusedPlaceholder.verdict, 'RefusedPlaceholderZeroLayerSlab');

const realMount = eng.mountResidentModelSlabsFromExporter(
  'uor-r4.qwen3-0.6b-holo',
  { admitted: true, n_layers: 28, d_model: 1024, layer_bytes: Array.from({ length: 28 }, () => 8192) },
  (_mId, lIdx) => new Uint8Array(8192).fill((lIdx + 1) & 0xff),
  () => new Uint32Array(2048 * 16).fill(0x55aa55aa),
);
assert.equal(realMount.ok, true);
assert.equal(realMount.verdict, 'HybridV14WebGpuDraftAdmitted');
assert.equal(realMount.resident_layers, 28);
assert.equal(realMount.per_layer_groups_ready, true);
assert.ok(stats.boundLayerBuffers.size >= 28, 'all 28 per-layer GPUBuffers must be bound to distinct layer bind groups');

// 3. Resident turn executes with 0 hydration calls, skips mapAsync during prefill, and runs bound GPU on decode
stats.mapAsyncCalls = 0;
let hydrateInvocations = 0;
let stepIdx = 0;
let mapAsyncAtPrefillEnd = -1;
const turnRes = await execute_inprism_foundry_turn(
  {
    id: 'r4_75_turn_1',
    payload: {
      prompt: 'List /workspace files',
      selected_model: 'uor-r4.qwen3-0.6b-holo',
      surface_mode: 'studio',
      slice_tokens: 4,
    },
  },
  {
    webgpuEngine: eng,
    collectWorkspaceSnapshot: () => ({ files: [{ path: 'README.md', content: '# R4\n', sha256: 'abc' }] }),
    hydrateHoloModel: async () => {
      hydrateInvocations += 1;
      return {};
    },
    holoRuntimeStatus: () =>
      JSON.stringify({
        mounted_count: 1,
        qwen3_06b_mounted: true,
        models: [{ model_id: 'uor-r4.qwen3-0.6b-holo', container_sha256: '617ea77f', n_layers: 28, d_model: 1024, vocab_size: 151936 }],
      }),
    holoBeginTurn: () =>
      JSON.stringify({
        status: 'prefilling',
        turn_id: 99,
        model_id: 'uor-r4.qwen3-0.6b-holo',
        prefill_done: 16,
        prefill_total: 32,
        prefix_reused_tokens: 197,
      }),
    holoPeekQSig512: () => new Uint32Array(16).fill(0xdeadbeef),
    holoNotifyGpuBoundStep: () => JSON.stringify({ status: 'ok', admitted: true, verdict: 'LiveGpuBoundStepAdmitted' }),
    holoStepTurn: () => {
      stepIdx += 1;
      if (stepIdx === 1) {
        mapAsyncAtPrefillEnd = stats.mapAsyncCalls;
        return JSON.stringify({
          status: 'prefilling',
          turn_id: 99,
          model_id: 'uor-r4.qwen3-0.6b-holo',
          delta: '',
          tokens_emitted: 0,
          prefill_done: 32,
          prefill_total: 32,
        });
      }
      return JSON.stringify({
        status: 'done',
        turn_id: 99,
        model_id: 'uor-r4.qwen3-0.6b-holo',
        delta: 'README.md is in /workspace.',
        tokens_emitted: 6,
      });
    },
  },
);
assert.equal(turnRes.ok, true);
assert.equal(hydrateInvocations, 0, 'resident model turn must invoke 0 hydration calls');
assert.equal(mapAsyncAtPrefillEnd, 0, 'prefill slices must not stall on GPU mapAsync');
assert.equal(stats.mapAsyncCalls, 1, 'decode slice must execute bound WebGPU pass');
const holoSummary = turnRes.result && turnRes.result.holo_v13;
assert.equal(holoSummary.resident_turn_verdict, 'ResidentTurnZeroDownloadAdmitted');
assert.equal(holoSummary.network_bytes_downloaded, 0);
assert.equal(holoSummary.opfs_bytes_read, 0);

// 4. Verify stamped ?v=v12r4-<js_sha12> and honest Resident header in public/
const prov = JSON.parse(fs.readFileSync(path.join(rootDir, 'public/provenance.json'), 'utf8'));
const sha12 = String(prov.binding_javascript_sha256).slice(0, 12);
const html = fs.readFileSync(path.join(rootDir, 'public/index.html'), 'utf8');
const appJs = fs.readFileSync(path.join(rootDir, 'public/app.js'), 'utf8');
assert.ok(html.includes(`app.js?v=v12r4-${sha12}`), 'index.html must carry stamped ?v=v12r4-<sha12>');
assert.ok(appJs.includes(`./lexlean_r4.js?v=v12r4-${sha12}`), 'app.js must carry stamped ?v=v12r4-<sha12>');
assert.ok(appJs.includes('Resident · Generating (0 B dl)'), 'app.js must render Resident · Generating (0 B dl)');

console.log('r4_75_resident_gpu_hybrid_drafter_e2e: ALL CHECKS PASSED');
