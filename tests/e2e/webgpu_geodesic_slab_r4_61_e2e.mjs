import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  HOLOSLAB2_MAGIC,
  HOLOSLAB2_SCHEMA,
  HOLOSLAB2_RING_BYTES,
  WEBGPU_MAX_STAGING_CHUNK_BYTES,
  OpfsKappaStore,
  WGSL_HOLO_SHADERS,
  WebGpuHoloEngine,
  computeHoloSlab2MerkleRootHex,
  verifyWgslShaderSourcesPureInteger,
  runWgslSplitByteParityCheck,
} from '../../crates/wasm/js/worker_host_bridge.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..', '..');

function sha256HexSync(buf) {
  return crypto.createHash('sha256').update(buf).digest('hex');
}

function createMockOpfsRoot() {
  const dirs = new Map();
  const getDirNode = (parts) => {
    const key = parts.join('/');
    if (!dirs.has(key)) {
      dirs.set(key, { files: new Map() });
    }
    return dirs.get(key);
  };
  const makeDirHandle = (parts) => ({
    async getDirectoryHandle(name, opts = {}) {
      const next = [...parts, name];
      const key = next.join('/');
      if (!dirs.has(key)) {
        if (!opts.create) throw new Error(`NotFoundError: dir ${key}`);
        getDirNode(next);
      }
      return makeDirHandle(next);
    },
    async getFileHandle(name, opts = {}) {
      const node = getDirNode(parts);
      if (!node.files.has(name)) {
        if (!opts.create) throw new Error(`NotFoundError: file ${parts.join('/')}/${name}`);
        node.files.set(name, new Uint8Array(0));
      }
      return {
        async createSyncAccessHandle() {
          return {
            truncate(sz) {
              const cur = node.files.get(name) || new Uint8Array(0);
              const next = new Uint8Array(sz);
              next.set(cur.subarray(0, sz), 0);
              node.files.set(name, next);
            },
            write(u8, { at = 0 } = {}) {
              const src = u8 instanceof Uint8Array ? u8 : new Uint8Array(u8);
              const cur = node.files.get(name) || new Uint8Array(0);
              const need = Math.max(cur.byteLength, at + src.byteLength);
              let dst = cur;
              if (need > cur.byteLength) {
                dst = new Uint8Array(need);
                dst.set(cur, 0);
              }
              dst.set(src, at);
              node.files.set(name, dst);
              return src.byteLength;
            },
            read(dst, { at = 0 } = {}) {
              const cur = node.files.get(name) || new Uint8Array(0);
              const take = Math.min(dst.byteLength, Math.max(0, cur.byteLength - at));
              if (take > 0) dst.set(cur.subarray(at, at + take), 0);
              return take;
            },
            getSize() {
              return (node.files.get(name) || new Uint8Array(0)).byteLength;
            },
            flush() {},
            close() {},
          };
        },
        async move(destDir, newName) {
          const data = node.files.get(name) || new Uint8Array(0);
          const fh = await destDir.getFileHandle(newName, { create: true });
          const acc = await fh.createSyncAccessHandle();
          acc.truncate(0);
          acc.write(data, { at: 0 });
          acc.close();
          node.files.delete(name);
        },
        async getFile() {
          const cur = node.files.get(name) || new Uint8Array(0);
          return {
            size: cur.byteLength,
            slice: (a, b) => ({
              async arrayBuffer() {
                return cur.slice(a, b).buffer;
              },
            }),
            async arrayBuffer() {
              return cur.slice().buffer;
            },
          };
        },
      };
    },
    async removeEntry(name) {
      getDirNode(parts).files.delete(name);
    },
    _dumpFile(name) {
      return getDirNode(parts).files.get(name);
    },
    _overwriteFile(name, bytes) {
      getDirNode(parts).files.set(name, bytes);
    },
  });
  getDirNode([]);
  return makeDirHandle([]);
}

async function testWgslPurityAndSplitByteParity() {
  console.log('1. Verifying 4 Pure-Integer WGSL Shaders & 0-LSB Split-Byte Parity...');
  for (const name of [
    'holo_rmsnorm_rht32_pack',
    'holo_int4_gemv_gemm',
    'holo_rope_flock_attn',
    'holo_hamming512_lmhead',
  ]) {
    assert.ok(typeof WGSL_HOLO_SHADERS[name] === 'string' && WGSL_HOLO_SHADERS[name].length > 100, `missing WGSL shader ${name}`);
  }

  const purity = verifyWgslShaderSourcesPureInteger();
  assert.equal(purity.ok, true, `expected WGSL shader purity ok, got ${JSON.stringify(purity)}`);
  assert.equal(purity.verdict, 'ZeroLsbAdmitted');
  assert.equal(purity.float_ops, 0);

  const zeroLsb = runWgslSplitByteParityCheck(0);
  assert.equal(zeroLsb.ok, true, `expected 0-LSB parity pass, got ${JSON.stringify(zeroLsb)}`);
  assert.equal(zeroLsb.verdict, 'ZeroLsbAdmitted');
  assert.equal(zeroLsb.max_abs_lsb_diff, 0);

  const drift = runWgslSplitByteParityCheck(1);
  assert.equal(drift.ok, false);
  assert.equal(drift.verdict, 'RefusedWgslCpuParityDrift');
  assert.equal(drift.max_abs_lsb_diff, 1);

  const gpuEngine = new WebGpuHoloEngine();
  const badStatus = await gpuEngine.init(1);
  assert.equal(badStatus.verdict, 'RefusedWgslCpuParityDrift');
  assert.equal(badStatus.webgpu_active, false);
  assert.ok(badStatus.backend.startsWith('wasm-simd128'));
}

async function testWebGpuChunkedUploadAndDestroy() {
  console.log('2. Verifying <= 4 MiB Chunked WebGPU Upload & Explicit buffer.destroy()...');
  const writeSizes = [];
  let destroyedCount = 0;
  const origNav = globalThis.navigator;
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: {
      hardwareConcurrency: 4,
      gpu: {
        async requestAdapter() {
          return {
            info: { description: 'Mock Pure-Int Adapter' },
            features: new Set(['packed_4x8_integer_dot_product']),
            async requestDevice() {
              return {
                createShaderModule: () => ({}),
                createComputePipeline: () => ({}),
                createBuffer: ({ size }) => ({
                  size,
                  destroy() {
                    destroyedCount += 1;
                  },
                }),
                queue: {
                  writeBuffer(_buf, _dstOff, _src, _srcOff, take) {
                    writeSizes.push(take);
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
    const status = await engine.init(0);
    assert.equal(status.verdict, 'ZeroLsbAdmitted');
    assert.equal(status.webgpu_active, true);
    assert.equal(status.pipelines.length, 4);
    assert.equal(status.max_staging_chunk_bytes, 4_194_304);

    const payload = new Uint8Array(5 * 1024 * 1024 + 123);
    engine.uploadChunkedBuffer(payload, 0x80);
    assert.ok(writeSizes.length >= 2, '5 MiB upload must be chunked across >= 2 staging transfers');
    for (const sz of writeSizes) {
      assert.ok(sz <= WEBGPU_MAX_STAGING_CHUNK_BYTES, `chunk ${sz} exceeded 4 MiB staging budget`);
    }
    engine.dispose();
    assert.equal(destroyedCount, 1, 'model switch dispose() must call buffer.destroy()');
  } finally {
    if (origNav === undefined) {
      delete globalThis.navigator;
    } else {
      Object.defineProperty(globalThis, 'navigator', { configurable: true, value: origNav });
    }
  }
}

async function testHoloSlab2SingleSlabQuotaSealAndStitchRepair() {
  console.log('3. Verifying HOLOSLAB2 Single-Slab Quota, <5ms Warm Seal & 2-of-3 Stitch Repair...');
  assert.equal(HOLOSLAB2_MAGIC, 'HOLOSLAB2');
  assert.equal(HOLOSLAB2_SCHEMA, 'uor-r4.holoslab/2');
  assert.equal(HOLOSLAB2_RING_BYTES, 2 * 1024 * 1024);

  const p0 = new Uint8Array(65_536);
  const p1 = new Uint8Array(65_536);
  const parity = new Uint8Array(65_536);
  for (let i = 0; i < 65_536; i += 1) {
    p0[i] = (i * 17 + 5) & 0xff;
    p1[i] = (i * 31 + 11) & 0xff;
    parity[i] = p0[i] ^ p1[i];
  }
  const sha0 = sha256HexSync(p0);
  const sha1 = sha256HexSync(p1);
  const containerSha = sha256HexSync(Buffer.concat([Buffer.from(p0), Buffer.from(p1)]));
  const manifest = {
    container_sha256: containerSha,
    total_bytes: 131_072,
    pieces: [
      { index: 0, byte_offset: 0, byte_length: 65_536, sha256: sha0 },
      { index: 1, byte_offset: 65_536, byte_length: 65_536, sha256: sha1 },
    ],
  };

  const mockRoot = createMockOpfsRoot();
  const origNav = globalThis.navigator;
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: {
      storage: {
        async estimate() {
          return { quota: 2_000_000_000, usage: 0 };
        },
        async getDirectory() {
          return mockRoot;
        },
      },
    },
  });

  try {
    const store = new OpfsKappaStore();
    await assert.rejects(
      async () => store.preflightStorageQuota(131_072, 131_071),
      (err) => err && String(err.message).includes('RefusedInsufficientStorageQuota'),
    );

    const writeRes = await store.writeVerifiedSlab('uor-r4.qwen3-0.6b-holo', manifest, [p0, p1], {
      quotaBytes: 2_000_000_000,
    });
    assert.equal(writeRes.ok, true);
    assert.equal(writeRes.verdict, 'SlabSealAdmitted');
    assert.equal(writeRes.seal.magic, 'HOLOSLAB2');
    assert.equal(writeRes.seal.schema, 'uor-r4.holoslab/2');

    // Warm reload must validate O(1) seal without full-file rehash
    const warm = await store.readWarmSlabSeal('uor-r4.qwen3-0.6b-holo', containerSha, 131_072, {
      quotaBytes: 2_000_000_000,
    });
    assert.equal(warm.ok, true);
    assert.equal(warm.verdict, 'SlabSealAdmitted');
    assert.ok(warm.elapsedMs < 20, `expected <20ms warm check in Node (target <5ms in browser), got ${warm.elapsedMs}ms`);

    const streamed = [];
    await store.streamWarmSlabChunks('uor-r4.qwen3-0.6b-holo', async (view) => {
      streamed.push(view.slice());
    });
    assert.deepEqual(Buffer.concat(streamed.map((u) => Buffer.from(u))), Buffer.concat([Buffer.from(p0), Buffer.from(p1)]));

    // Full clean scan passes
    const fullClean = await store.verifyOrRepairSlab('uor-r4.qwen3-0.6b-holo', {
      repair: false,
      quotaBytes: 2_000_000_000,
    });
    assert.equal(fullClean.ok, true);
    assert.equal(fullClean.verdict, 'SlabFullMerkleVerified');

    // Corrupt piece 0 in-place -> fail closed with RefusedSlabPieceHashMismatch -> heal via Stitch XOR parity
    const modelDir = await (await mockRoot.getDirectoryHandle('models', { create: false })).getDirectoryHandle('uor-r4.qwen3-0.6b-holo', { create: false });
    const slabCopy = modelDir._dumpFile('model.holoslab').slice();
    slabCopy[0] ^= 0xff;
    modelDir._overwriteFile('model.holoslab', slabCopy);

    const corruptScan = await store.verifyOrRepairSlab('uor-r4.qwen3-0.6b-holo', {
      repair: false,
      quotaBytes: 2_000_000_000,
    });
    assert.equal(corruptScan.ok, false);
    assert.equal(corruptScan.verdict, 'RefusedSlabPieceHashMismatch');
    assert.deepEqual(corruptScan.corruptPieces, [0]);

    const repaired = await store.verifyOrRepairSlab('uor-r4.qwen3-0.6b-holo', {
      repair: true,
      quotaBytes: 2_000_000_000,
      fetchParityBytes: async () => parity,
    });
    assert.equal(repaired.ok, true);
    assert.equal(repaired.verdict, 'SlabFullMerkleVerified');
    assert.equal(repaired.repairedPieces, 1);

    // Corrupt seal Merkle root -> fail closed with RefusedSlabSealMismatch
    const sealOrig = JSON.parse(Buffer.from(modelDir._dumpFile('manifest.verified.json')).toString('utf8'));
    const badSeal = { ...sealOrig, merkle_root_sha256: '0'.repeat(64) };
    modelDir._overwriteFile('manifest.verified.json', new Uint8Array(Buffer.from(JSON.stringify(badSeal))));
    const sealFail = await store.readWarmSlabSeal('uor-r4.qwen3-0.6b-holo', containerSha, 131_072, {
      quotaBytes: 2_000_000_000,
    });
    assert.equal(sealFail.ok, false);
    assert.equal(sealFail.verdict, 'RefusedSlabSealMismatch');
    const recomputed = await computeHoloSlab2MerkleRootHex(manifest.pieces);
    assert.equal(recomputed, sealOrig.merkle_root_sha256);
  } finally {
    if (origNav === undefined) {
      delete globalThis.navigator;
    } else {
      Object.defineProperty(globalThis, 'navigator', { configurable: true, value: origNav });
    }
  }
}

function testUiControlsAndStaticCleanliness() {
  console.log('4. Verifying Wave 2 UI Controls Wiring & Forbidden-Token Hygiene...');
  const indexHtml = fs.readFileSync(path.join(ROOT, 'public/index.html'), 'utf8');
  const appJs = fs.readFileSync(path.join(ROOT, 'public/app.js'), 'utf8');
  const appCss = fs.readFileSync(path.join(ROOT, 'public/app.css'), 'utf8');

  for (const id of [
    'settings-mlp-shortlist-mode',
    'settings-verify-slab-btn',
    'holo-obs-mlp-mode-btn',
    'holo-obs-verify-slab-btn',
  ]) {
    assert.ok(indexHtml.includes(`id="${id}"`), `public/index.html missing #${id}`);
    assert.ok(appJs.includes(id), `public/app.js must wire #${id}`);
  }
  for (const forbidden of ['telemetry', 'fetch(', 'https://', 'WebSocket', 'parseFloat(', 'parseInt(']) {
    assert.ok(!appJs.includes(forbidden), `public/app.js contains forbidden token: ${forbidden}`);
  }
  for (const badCss of ['@import', 'url(', 'http://', 'https://']) {
    assert.ok(!appCss.includes(badCss), `public/app.css contains forbidden token: ${badCss}`);
  }
}

async function main() {
  await testWgslPurityAndSplitByteParity();
  await testWebGpuChunkedUploadAndDestroy();
  await testHoloSlab2SingleSlabQuotaSealAndStitchRepair();
  testUiControlsAndStaticCleanliness();
  console.log('=== ALL R4-61 WEBGPU WGSL + GEODESIC SWIGLU + HOLOSLAB2 E2E CHECKS PASSED ===');
}

main().catch((err) => {
  console.error('R4-61 E2E FAILED:', err);
  process.exit(1);
});
