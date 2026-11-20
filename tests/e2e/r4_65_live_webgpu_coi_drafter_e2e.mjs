#!/usr/bin/env node
/**
 * Formal R4.LiveWebGpuTurnBridge E2E Suite (R4-65):
 * 1. Verifies concrete GPUBindGroup creation & binding on every 28-layer + LM-head compute dispatch
 *    (`bound_dispatches === dispatches === 197`) and refuses unbound compute passes (`RefusedUnboundComputePass`).
 * 2. Verifies live turn integration (`execute_inprism_foundry_turn` + `stepLiveTurnBoundPassAsync`)
 *    drives `holo_notify_foundry_turn_gpu_bound_step` and preserves 0-LSB token parity vs headless Wasm SIMD.
 * 3. Verifies `site/coi-serviceworker.js` emits `COEP: credentialless` (never `require-corp`) and
 *    `registerStudioCoiServiceWorker` enforces a single-reload sessionStorage guard.
 * 4. Verifies adaptive speculative drafter budget (`admitAdaptiveSpeculativeBudget`) throttles CPU misses.
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
  admitLiveGpuBoundStep,
  admitCoiCredentiallessMode,
  admitAdaptiveSpeculativeBudget,
  registerStudioCoiServiceWorker,
  execute_inprism_foundry_turn,
} = bridge;

function createStrictMockGpuDevice() {
  const stats = {
    buffersCreated: 0,
    bindGroupsCreated: 0,
    setBindGroupCalls: 0,
    dispatchCalls: 0,
    submits: 0,
    mapAsyncCalls: 0,
  };
  const device = {
    createBuffer({ size, usage }) {
      stats.buffersCreated += 1;
      const backing = new Uint8Array(Math.max(16, size));
      return {
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
    createShaderModule({ code }) {
      return { code };
    },
    createComputePipeline({ compute }) {
      return {
        compute,
        getBindGroupLayout(index) {
          return { index, entryPoint: compute.entryPoint };
        },
      };
    },
    createBindGroup({ layout, entries }) {
      assert.ok(layout, 'GPUBindGroup must reference a valid layout');
      assert.ok(Array.isArray(entries) && entries.length >= 4, 'GPUBindGroup must bind concrete GPUBuffers');
      stats.bindGroupsCreated += 1;
      return { layout, entries };
    },
    createCommandEncoder() {
      let currentBound = false;
      return {
        beginComputePass() {
          return {
            setPipeline() {
              currentBound = false;
            },
            setBindGroup(slot, bg) {
              assert.equal(slot, 0);
              assert.ok(bg && Array.isArray(bg.entries), 'setBindGroup(0) must receive a valid GPUBindGroup');
              currentBound = true;
              stats.setBindGroupCalls += 1;
            },
            dispatchWorkgroups() {
              assert.ok(currentBound, 'WebGPU validation: dispatchWorkgroups called without setBindGroup(0)!');
              stats.dispatchCalls += 1;
            },
            end() {},
          };
        },
        copyBufferToBuffer() {},
        finish() {
          return { encoded: true };
        },
      };
    },
    queue: {
      writeBuffer() {},
      submit(cmds) {
        assert.equal(cmds.length, 1);
        stats.submits += 1;
      },
    },
  };
  return { device, stats };
}

// 1. Formal admission helpers & R5 negative gates
assert.equal(admitLiveGpuBoundStep(0, true, 197, 197, 0), 'LiveGpuBoundStepAdmitted');
assert.equal(admitLiveGpuBoundStep(0, false, 0, 0, 0), 'FallbackWasmSimdAdmitted');
assert.equal(admitLiveGpuBoundStep(1, true, 197, 197, 0), 'RefusedFloatOpInGpuBridge');
assert.equal(admitLiveGpuBoundStep(0, true, 197, 196, 0), 'RefusedUnboundComputePass');
assert.equal(admitLiveGpuBoundStep(0, true, 197, 197, 1), 'RefusedGpuCpuLogitDrift');

assert.equal(admitCoiCredentiallessMode(true, false, 1), 'CrossOriginCredentiallessAdmitted');
assert.equal(admitCoiCredentiallessMode(false, false, 0), 'SingleWorkerFallbackAdmitted');
assert.equal(admitCoiCredentiallessMode(true, true, 1), 'RefusedRequireCorpBreaksMirrors');

assert.equal(admitAdaptiveSpeculativeBudget(true, 90, 4, 4), 'AdaptiveTileAdmitted');
assert.equal(admitAdaptiveSpeculativeBudget(false, 120, 1, 4), 'ThrottledCpuDraftsAdmitted');
assert.equal(admitAdaptiveSpeculativeBudget(false, 120, 3, 4), 'RefusedUnthrottledCpuMissStorm');

// 2. Strict WebGPU GPUBindGroup enforcement + negative unbound pass rejection
const eng = new WebGpuHoloEngine();
const { device, stats } = createStrictMockGpuDevice();
eng.device = device;
eng.deviceMaxStorageBinding = 134_217_728;
for (const name of ['holo_rmsnorm_rht32_pack', 'holo_int4_gemv_gemm', 'holo_rope_flock_attn', 'holo_hamming512_lmhead']) {
  eng.pipelines.set(name, device.createComputePipeline({ compute: { entryPoint: 'main' } }));
}
eng.residentPipelines.set(
  'holo_swiglu_geodesic_topk',
  device.createComputePipeline({ compute: { entryPoint: 'main' } }),
);

const mountOut = eng.mountResidentModelSlabs(
  'uor-r4.qwen3-0.6b-holo',
  new Uint8Array(28 * 1024),
  Array.from({ length: 28 }, () => 1024),
  1024,
);
assert.equal(mountOut.ok, true);
assert.equal(mountOut.bound_groups_ready, true);
assert.equal(stats.bindGroupsCreated, 5);

// Negative gate: omitBindGroups must be refused BEFORE queue.submit (and without triggering unbound dispatch)
const unboundEngine = new WebGpuHoloEngine();
unboundEngine.device = {
  ...device,
  createCommandEncoder() {
    return {
      beginComputePass() {
        return {
          setPipeline() {},
          setBindGroup() {},
          dispatchWorkgroups() {},
          end() {},
        };
      },
      copyBufferToBuffer() {},
      finish() {
        return {};
      },
    };
  },
};
unboundEngine.pipelines = eng.pipelines;
unboundEngine.residentPipelines = eng.residentPipelines;
unboundEngine.mountResidentModelSlabs(
  'uor-r4.qwen3-0.6b-holo',
  new Uint8Array(28 * 64),
  Array.from({ length: 28 }, () => 64),
);
const badPass = unboundEngine.encodeResident28LayerTilePass([1, 2], 0, 'GeodesicSparse1536', {
  omitBindGroups: true,
});
assert.equal(badPass.ok, false);
assert.equal(badPass.verdict, 'RefusedUnboundComputePass');
assert.equal(badPass.submit_count, 0);

// Positive bound live step
const liveStep = await eng.stepLiveTurnBoundPassAsync(new Uint32Array(16), [10, 20, 30], 4);
assert.equal(liveStep.ok, true);
assert.equal(liveStep.gpu_executed, true);
assert.equal(liveStep.verdict, 'LiveGpuBoundStepAdmitted');
assert.equal(liveStep.dispatches, 197);
assert.equal(liveStep.bound_dispatches, 197);
assert.equal(stats.setBindGroupCalls, 197);
assert.equal(stats.dispatchCalls, 197);
assert.equal(stats.mapAsyncCalls, 1);
assert.equal(liveStep.summary.byteLength, 256);

// 3. Live execute_inprism_foundry_turn drives bound GPU pass on every hop slice
let gpuNotifications = 0;
let stepCalls = 0;
const turnOut = await execute_inprism_foundry_turn(
  {
    id: 'inprism_gpu_1',
    payload: {
      prompt: 'Verify resident bound GPU turn bridge',
      selected_model: 'uor-r4.qwen3-0.6b-holo',
      max_new_tokens: 8,
    },
  },
  {
    webgpuEngine: eng,
    collectWorkspaceSnapshot: () => ({ files: [] }),
    holoRuntimeStatus: () =>
      JSON.stringify({
        mounted_count: 1,
        qwen3_06b_mounted: true,
        models: [{ model_id: 'uor-r4.qwen3-0.6b-holo', container_sha256: '617ea77f', n_layers: 28, d_model: 1024, vocab_size: 151936 }],
      }),
    holoBeginTurn: () => JSON.stringify({ status: 'ready', turn_id: 77, model_id: 'uor-r4.qwen3-0.6b-holo' }),
    holoPeekQSig512: () => new Uint32Array(16).fill(0x12345678),
    holoNotifyGpuBoundStep: (_tid, totalD, boundD, drift) => {
      assert.equal(Number(totalD), 197);
      assert.equal(Number(boundD), 197);
      assert.equal(Number(drift), 0);
      gpuNotifications += 1;
      return JSON.stringify({ status: 'ok', admitted: true, verdict: 'LiveGpuBoundStepAdmitted' });
    },
    holoStepTurn: () => {
      stepCalls += 1;
      if (stepCalls === 1) {
        return JSON.stringify({ status: 'streaming', turn_id: 77, model_id: 'uor-r4.qwen3-0.6b-holo', delta: 'GPU ', tokens_emitted: 2 });
      }
      return JSON.stringify({ status: 'done', turn_id: 77, model_id: 'uor-r4.qwen3-0.6b-holo', delta: 'OK', tokens_emitted: 4 });
    },
  },
);
assert.equal(turnOut.ok, true);
assert.equal(turnOut.result.reply, 'GPU OK');
assert.equal(gpuNotifications, 2, 'Every streaming slice must run & notify bound WebGPU pass');

// 4. Verify site/coi-serviceworker.js uses COEP: credentialless (never require-corp) + 1-reload guard
const coiSwPath = path.join(rootDir, 'site/coi-serviceworker.js');
assert.ok(fs.existsSync(coiSwPath), 'site/coi-serviceworker.js must exist');
const coiSrc = fs.readFileSync(coiSwPath, 'utf8');
assert.ok(coiSrc.includes("'Cross-Origin-Opener-Policy', 'same-origin'"));
assert.ok(coiSrc.includes("'Cross-Origin-Embedder-Policy', 'credentialless'"));
assert.ok(!coiSrc.includes("'Cross-Origin-Embedder-Policy', 'require-corp'"), 'Must never set require-corp');

const sessionMap = new Map();
let reloadCount = 0;
const mockEnv = {
  isSecureContext: true,
  crossOriginIsolated: false,
  sessionStorage: {
    getItem: (k) => sessionMap.get(k) || null,
    setItem: (k, v) => sessionMap.set(k, String(v)),
  },
  location: {
    reload() {
      reloadCount += 1;
    },
  },
  navigator: {
    serviceWorker: {
      controller: null,
      async register() {
        return { active: {} };
      },
    },
  },
};
const reg1 = await registerStudioCoiServiceWorker(mockEnv);
assert.equal(reg1.ok, true);
assert.equal(reg1.verdict, 'CrossOriginCredentiallessAdmitted');
assert.equal(reloadCount, 1, 'First activation reloads once to activate COOP/COEP');

const reg2 = await registerStudioCoiServiceWorker(mockEnv);
assert.equal(reg2.ok, true);
assert.equal(reg2.verdict, 'SingleWorkerFallbackAdmitted');
assert.equal(reloadCount, 1, 'Second invocation must NOT loop-reload when sessionStorage guard is set');

console.log('[PASS] R4-65 bound WebGPU turn bridge, COEP credentialless SW, and adaptive drafter verified.');
