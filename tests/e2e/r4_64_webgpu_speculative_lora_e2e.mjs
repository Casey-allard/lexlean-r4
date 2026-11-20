import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import {
  WEBGPU_MAX_LAYER_SLAB_BYTES,
  WEBGPU_MAX_STAGING_CHUNK_BYTES,
  WEBGPU_RESULT_SUMMARY_READBACK_BYTES,
  WGSL_HOLO_SHADERS,
  WGSL_RESIDENT_MLP_SHADERS,
  WebGpuHoloEngine,
  admitResidentLayerPartition,
  admitSpeculativeTileStep,
  admitStudioLoraPromptShrink,
  run28LayerResidentParityCheck,
  verifyWgslShaderSourcesPureInteger,
} from '../../crates/wasm/js/worker_host_bridge.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..', '..');

async function testResidentWebGpuSlabsAndSingleEncoderSubmit() {
  console.log('1. Verifying Resident <=128 MiB 28-Layer Slabs & Single-Encoder 256B Readback...');
  assert.equal(WEBGPU_MAX_LAYER_SLAB_BYTES, 134_217_728);
  assert.equal(WEBGPU_RESULT_SUMMARY_READBACK_BYTES, 256);

  const purity = verifyWgslShaderSourcesPureInteger();
  assert.equal(purity.ok, true);
  assert.equal(purity.float_ops, 0);
  assert.equal(purity.shader_count, 4);
  assert.equal(purity.resident_shader_count, 1);
  assert.ok(WGSL_HOLO_SHADERS.holo_int4_gemv_gemm.length > 100);
  assert.ok(WGSL_RESIDENT_MLP_SHADERS.holo_swiglu_geodesic_topk.includes('mul_i32_shift14_round'));

  assert.equal(admitResidentLayerPartition(0, 36_700_160, 134_217_728, 134_217_728), 'ResidentSlabAdmitted');
  assert.equal(admitResidentLayerPartition(1, 36_700_160, 134_217_728, 134_217_728), 'RefusedFloatOpInResidentWgsl');
  assert.equal(admitResidentLayerPartition(0, 150_000_000, 134_217_728, 268_435_456), 'RefusedLayerExceeds128MiB');
  assert.equal(admitResidentLayerPartition(0, 36_700_160, 134_217_728, 16_777_216), 'RefusedInsufficientGpuBufferLimits');

  let createdBuffers = 0;
  let destroyedBuffers = 0;
  let submitCalls = 0;
  let copyBytes = 0;
  const writeChunks = [];
  const origNav = globalThis.navigator;

  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: {
      hardwareConcurrency: 8,
      gpu: {
        async requestAdapter() {
          return {
            info: { description: 'Mock RTX Resident Adapter' },
            features: new Set(['packed_4x8_integer_dot_product', 'subgroups']),
            limits: {
              maxStorageBufferBindingSize: 134_217_728,
              maxBufferSize: 536_870_912,
            },
            async requestDevice(opts) {
              assert.ok(opts.requiredFeatures.includes('packed_4x8_integer_dot_product'));
              assert.equal(opts.requiredLimits.maxStorageBufferBindingSize, 134_217_728);
              return {
                createShaderModule: () => ({}),
                createComputePipeline: () => ({}),
                createBuffer: ({ size }) => {
                  createdBuffers += 1;
                  return {
                    size,
                    destroy() {
                      destroyedBuffers += 1;
                    },
                  };
                },
                createCommandEncoder: () => ({
                  beginComputePass: () => ({
                    setPipeline() {},
                    dispatchWorkgroups() {},
                    end() {},
                  }),
                  copyBufferToBuffer(_src, _so, _dst, _do, byteLen) {
                    copyBytes = byteLen;
                  },
                  finish: () => ({ encoded: true }),
                }),
                queue: {
                  writeBuffer(_buf, _dstOff, _src, _srcOff, take) {
                    writeChunks.push(take);
                  },
                  submit(cmds) {
                    submitCalls += cmds.length;
                  },
                },
              };
            },
          };
        },
      },
    },
  });

  try {
    const engine = new WebGpuHoloEngine();
    const st = await engine.init(0);
    assert.equal(st.verdict, 'ZeroLsbAdmitted');
    assert.equal(st.webgpu_active, true);
    assert.equal(st.pipelines.length, 4);
    assert.deepEqual(st.resident_pipelines, ['holo_swiglu_geodesic_topk']);
    assert.equal(st.has_packed_dot4, true);
    assert.equal(st.has_subgroups, true);

    const oversize = engine.mountResidentModelSlabs(
      'uor-r4.qwen3-1.7b-holo',
      new Uint8Array(1024),
      Array.from({ length: 28 }, () => 150_000_000),
    );
    assert.equal(oversize.ok, false);
    assert.equal(oversize.verdict, 'RefusedLayerExceeds128MiB');

    const mockContainer = new Uint8Array(28 * 4096);
    const layerSizes = Array.from({ length: 28 }, () => 4096);
    const mounted = engine.mountResidentModelSlabs('uor-r4.qwen3-0.6b-holo', mockContainer, layerSizes);
    assert.equal(mounted.ok, true);
    assert.equal(mounted.verdict, 'ResidentSlabAdmitted');
    assert.equal(mounted.resident_layers, 28);
    assert.equal(mounted.readback_bytes, 256);
    for (const c of writeChunks) {
      assert.ok(c <= WEBGPU_MAX_STAGING_CHUNK_BYTES);
    }

    const stepOut = engine.encodeResident28LayerTilePass([11, 22, 33, 44, 55, 66], 32, 'GeodesicSparse1536');
    assert.equal(stepOut.ok, true);
    assert.equal(stepOut.verdict, 'SpeculativeExact0LsbAdmitted');
    assert.equal(stepOut.layers_encoded, 28);
    assert.equal(stepOut.tile_tokens, 6);
    assert.equal(stepOut.submit_count, 1);
    assert.equal(stepOut.readback_bytes, 256);
    assert.equal(submitCalls, 1);
    assert.equal(copyBytes, 256);

    const denseOut = engine.encodeResident28LayerTilePass([11, 22, 33], 38, 'DenseExact');
    assert.equal(denseOut.ok, true);
    assert.equal(denseOut.mlp_mode, 'DenseExact');

    engine.dispose();
    assert.equal(destroyedBuffers, createdBuffers, 'dispose() must destroy every resident & scratch GPUBuffer');
  } finally {
    if (origNav === undefined) {
      delete globalThis.navigator;
    } else {
      Object.defineProperty(globalThis, 'navigator', { configurable: true, value: origNav });
    }
  }
}

async function test28Layer0LsbParityAndSpeculativeGates() {
  console.log('2. Verifying 28-Layer 0-LSB Parity (Sparse & Dense) and Speculative Tile Refusal Gates...');
  const sparseOk = run28LayerResidentParityCheck(0, 'GeodesicSparse1536');
  assert.equal(sparseOk.ok, true);
  assert.equal(sparseOk.verdict, 'SpeculativeExact0LsbAdmitted');
  assert.equal(sparseOk.layers_verified, 28);
  assert.equal(sparseOk.max_abs_lsb_diff, 0);
  assert.equal(sparseOk.top1_match, true);

  const denseOk = run28LayerResidentParityCheck(0, 'DenseExact');
  assert.equal(denseOk.ok, true);
  assert.equal(denseOk.verdict, 'SpeculativeExact0LsbAdmitted');
  assert.equal(denseOk.max_abs_lsb_diff, 0);

  const drift = run28LayerResidentParityCheck(1, 'GeodesicSparse1536');
  assert.equal(drift.ok, false);
  assert.equal(drift.verdict, 'RefusedSpeculativeParityDrift');
  assert.equal(drift.max_abs_lsb_diff, 1);

  assert.equal(admitSpeculativeTileStep(5, 32, 0, true), 'SpeculativeExact0LsbAdmitted');
  assert.equal(admitSpeculativeTileStep(32, 32, 0, true), 'RefusedDraftExceedsTile');
  assert.equal(admitSpeculativeTileStep(5, 32, 1, true), 'RefusedSpeculativeParityDrift');
  assert.equal(admitSpeculativeTileStep(5, 32, 0, false), 'RefusedKvRewindMismatch');
}

async function testWasmResidentBridgeAndStudioLoraLockstep() {
  console.log('3. Verifying Wasm R4-64 Bindings & Studio LoRA <=40-Token Lockstep...');
  assert.equal(admitStudioLoraPromptShrink(0, true, true, 34), 'StudioLoraUorV13Admitted');
  assert.equal(admitStudioLoraPromptShrink(0, true, false, 34), 'RefusedPromptShrinkWithoutFineTunedManifest');
  assert.equal(admitStudioLoraPromptShrink(0, false, true, 34), 'RefusedStitchParityMismatch');
  assert.equal(admitStudioLoraPromptShrink(1, true, true, 34), 'RefusedFloatOpInLoraExport');
  assert.equal(admitStudioLoraPromptShrink(0, true, true, 41), 'RefusedPromptExceedsCompactBudget');

  const wasmJsPath = path.join(ROOT, 'public', 'lexlean_r4.js');
  const wasmBinPath = path.join(ROOT, 'public', 'lexlean_r4_bg.wasm');
  const wasmMod = await import(pathToFileURL(wasmJsPath).href);
  wasmMod.initSync({ module: fs.readFileSync(wasmBinPath) });

  if (typeof wasmMod.studio_verify_resident_webgpu_step === 'function') {
    const okStep = JSON.parse(
      wasmMod.studio_verify_resident_webgpu_step(0n, 36_700_160n, 134_217_728n, 5n, 0n, true, true, true, 34n),
    );
    assert.equal(okStep.admitted, true);
    assert.equal(okStep.slab_verdict, 'ResidentSlabAdmitted');
    assert.equal(okStep.speculative_verdict, 'SpeculativeExact0LsbAdmitted');
    assert.equal(okStep.lora_verdict, 'StudioLoraUorV13Admitted');
    assert.equal(okStep.max_compact_studio_prompt_tokens, 40);

    const unpinned = JSON.parse(
      wasmMod.studio_verify_resident_webgpu_step(0n, 36_700_160n, 134_217_728n, 5n, 0n, true, true, false, 34n),
    );
    assert.equal(unpinned.admitted, false);
    assert.equal(unpinned.lora_verdict, 'RefusedPromptShrinkWithoutFineTunedManifest');
  }

  for (const slug of ['v13_qwen3_0_6b', 'v13_qwen3_1_7b']) {
    const m = JSON.parse(fs.readFileSync(path.join(ROOT, 'gpu', 'weights', slug, 'manifest.json'), 'utf8'));
    const s = JSON.parse(fs.readFileSync(path.join(ROOT, 'gpu', 'weights', slug, 'manifest.stitch.json'), 'utf8'));
    assert.equal(m.format, 'UORR4V13');
    assert.equal(s.schema, 'stitch/xor-2-of-3/1');
    assert.equal(m.container_sha256, s.container_sha256);
    assert.equal(m.piece_count, m.pieces.length);
  }

  const m06 = JSON.parse(fs.readFileSync(path.join(ROOT, 'gpu', 'weights', 'v13_qwen3_0_6b', 'manifest.json'), 'utf8'));
  const siteApp = JSON.parse(fs.readFileSync(path.join(ROOT, 'site', 'app.json'), 'utf8'));
  assert.equal(siteApp.qwen3WeightsKappa, `sha256:${m06.container_sha256}`);
}

await testResidentWebGpuSlabsAndSingleEncoderSubmit();
await test28Layer0LsbParityAndSpeculativeGates();
await testWasmResidentBridgeAndStudioLoraLockstep();
console.log('[PASS] R4-64 resident WebGPU <=128 MiB slabs, 28-layer 0-LSB speculative tiles, and Studio LoRA lockstep verified.');
