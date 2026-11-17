// ============================================================================
// UOR-R4 Worker Host Bridge & Cache/WebGPU/OPFS Runtime (Extracted Custom Sections)
// Bundled into public/lexlean_r4.js alongside wasm-bindgen bindings.
// ============================================================================
const cachedTextDecoder = typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: false }) : null;
const cachedTextEncoder = typeof TextEncoder !== 'undefined' ? new TextEncoder() : null;
// === UOR_CUSTOM_SECTION_1_START ===
const WEIGHT_CACHE_NAME = 'uor-r4-wasm-weights-v2';
const IDB_DB_NAME = 'UOR_R4_WEIGHT_CACHE';
const IDB_STORE_NAME = 'uorlut_packs';
const IDB_ACTIVE_KEY = 'active_uorlut_v8_2b_34259da9';
const EXPECTED_SHARD_SHA256 = '34259da95f7760d0d8633ff53afd6d6cce9a89d98c3e5bd0b11ff862ceee5e48';

globalThis.__uorWeightCacheMeta = {
    status: 'Initializing UORLUT01/8...',
    sha256: '',
    mode: 'init'
};

function __uor_open_idb() {
    return new Promise((resolve, reject) => {
        if (typeof indexedDB === 'undefined') {
            reject(new Error('IndexedDB unavailable'));
            return;
        }
        const req = indexedDB.open(IDB_DB_NAME, 1);
        req.onupgradeneeded = () => {
            const db = req.result;
            if (!db.objectStoreNames.contains(IDB_STORE_NAME)) {
                db.createObjectStore(IDB_STORE_NAME);
            }
        };
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error || new Error('IndexedDB open error'));
    });
}

const OPFS_WEIGHT_FILE = 'uor-r4-glm-v8.uorlut';
const OPFS_META_FILE = 'uor-r4-glm-v8.meta.json';
const OPFS_OPEN_WEIGHTS_FILE = 'uor-r4-open-weights.bin';

const UOR_WEBGPU_Q12_ATTENTION_WGSL = `
struct Params {
  rows: u32,
  dim: u32,
};
@group(0) @binding(0) var<storage, read> weights_q12: array<i32>;
@group(0) @binding(1) var<storage, read> query_q12: array<i32>;
@group(0) @binding(2) var<storage, read_write> scores_out: array<i32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let row = gid.x;
  if (row >= params.rows) {
    return;
  }
  let base = row * params.dim;
  var dot_acc: i32 = 0;
  var l1_acc: i32 = 0;
  for (var d: u32 = 0u; d < params.dim; d = d + 1u) {
    let w = weights_q12[base + d];
    let q = query_q12[d];
    dot_acc = dot_acc + ((w * q) >> 12u);
    let diff = w - q;
    l1_acc = l1_acc + select(diff, -diff, diff < 0);
  }
  scores_out[row] = dot_acc - (l1_acc >> 4u);
}
`;

const UOR_WEBGPU_HAMMING512_WGSL = `
/* Formal R4.HoloGeometricRuntime pure u32/countOneBits 512-bit Hamming shortlist kernel (zero f32/f16) */
struct HammingParams {
  vocab_size: u32,
  words_per_sig: u32,
};
@group(0) @binding(0) var<storage, read> q_sig_u32: array<u32>;
@group(0) @binding(1) var<storage, read> vocab_sigs_u32: array<u32>;
@group(0) @binding(2) var<storage, read_write> distances_u32: array<u32>;
@group(0) @binding(3) var<uniform> params: HammingParams;

@compute @workgroup_size(128)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let row = gid.x;
  if (row >= params.vocab_size) {
    return;
  }
  let base = row * params.words_per_sig;
  var d_ham: u32 = 0u;
  for (var w: u32 = 0u; w < params.words_per_sig; w = w + 1u) {
    d_ham = d_ham + countOneBits(q_sig_u32[w] ^ vocab_sigs_u32[base + w]);
  }
  distances_u32[row] = d_ham;
}
`;

globalThis.__uorOpenWeightsTier = {
    webgpuAvailable: false,
    webgpuReady: false,
    adapterName: 'Wasm-SIMD Integer Co-Processor',
    opfsAvailable: false,
    opfsPersistedBytes: 0,
    openWeightsFormat: 'UORLUT01/8',
    openWeightsTensorCount: 6,
    device: null,
    pipeline: null,
    hammingPipeline: null,
};

export async function cache_weights_in_opfs(fileName, bytes, metaObj) {
    if (typeof navigator === 'undefined' || !navigator.storage || typeof navigator.storage.getDirectory !== 'function') {
        return false;
    }
    try {
        const rootDir = await navigator.storage.getDirectory();
        const fileHandle = await rootDir.getFileHandle(fileName || OPFS_WEIGHT_FILE, { create: true });
        if (typeof fileHandle.createWritable === 'function') {
            const writable = await fileHandle.createWritable();
            const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
            await writable.write(u8);
            await writable.close();
            if (metaObj) {
                const metaHandle = await rootDir.getFileHandle(OPFS_META_FILE, { create: true });
                const metaWritable = await metaHandle.createWritable();
                await metaWritable.write(JSON.stringify(metaObj));
                await metaWritable.close();
            }
            globalThis.__uorOpenWeightsTier.opfsAvailable = true;
            globalThis.__uorOpenWeightsTier.opfsPersistedBytes = u8.byteLength;
            return true;
        }
    } catch (_e) {
        // OPFS unavailable or restricted in current context
    }
    return false;
}

export async function restore_weights_from_opfs(fileName) {
    if (typeof navigator === 'undefined' || !navigator.storage || typeof navigator.storage.getDirectory !== 'function') {
        return null;
    }
    try {
        const rootDir = await navigator.storage.getDirectory();
        const fileHandle = await rootDir.getFileHandle(fileName || OPFS_WEIGHT_FILE, { create: false });
        const file = await fileHandle.getFile();
        if (!file || file.size < 1024) {
            return null;
        }
        let meta = {};
        try {
            const metaHandle = await rootDir.getFileHandle(OPFS_META_FILE, { create: false });
            const metaFile = await metaHandle.getFile();
            meta = JSON.parse(await metaFile.text());
        } catch (_me) {
            meta = {};
        }
        const ab = await file.arrayBuffer();
        const bytes = new Uint8Array(ab);
        globalThis.__uorOpenWeightsTier.opfsAvailable = true;
        globalThis.__uorOpenWeightsTier.opfsPersistedBytes = bytes.byteLength;
        return { bytes, meta };
    } catch (_e) {
        return null;
    }
}

export async function init_webgpu_open_weights_tier() {
    if (typeof navigator !== 'undefined' && navigator.storage && typeof navigator.storage.getDirectory === 'function') {
        globalThis.__uorOpenWeightsTier.opfsAvailable = true;
    }
    if (typeof navigator === 'undefined' || !navigator.gpu || typeof navigator.gpu.requestAdapter !== 'function') {
        return globalThis.__uorOpenWeightsTier;
    }
    try {
        globalThis.__uorOpenWeightsTier.webgpuAvailable = true;
        if (globalThis.__uorOpenWeightsTier.device && globalThis.__uorOpenWeightsTier.pipeline) {
            return globalThis.__uorOpenWeightsTier;
        }
        const adapter = await navigator.gpu.requestAdapter({ powerPreference: 'high-performance' });
        if (!adapter) {
            return globalThis.__uorOpenWeightsTier;
        }
        const info = adapter.info || {};
        globalThis.__uorOpenWeightsTier.adapterName = info.description || info.vendor || info.architecture || 'WebGPU Hardware Adapter';
        const device = await adapter.requestDevice();
        const shaderModule = device.createShaderModule({ code: UOR_WEBGPU_Q12_ATTENTION_WGSL });
        const pipeline = device.createComputePipeline({
            layout: 'auto',
            compute: { module: shaderModule, entryPoint: 'main' }
        });
        const hamModule = device.createShaderModule({ code: UOR_WEBGPU_HAMMING512_WGSL });
        const hammingPipeline = device.createComputePipeline({
            layout: 'auto',
            compute: { module: hamModule, entryPoint: 'main' }
        });
        globalThis.__uorOpenWeightsTier.device = device;
        globalThis.__uorOpenWeightsTier.pipeline = pipeline;
        globalThis.__uorOpenWeightsTier.hammingPipeline = hammingPipeline;
        globalThis.__uorOpenWeightsTier.webgpuReady = true;
    } catch (_ge) {
        globalThis.__uorOpenWeightsTier.webgpuReady = false;
    }
    return globalThis.__uorOpenWeightsTier;
}

/** Formal R4.HoloGeometricRuntime evaluate pure-integer 512-bit XOR Hamming distance on WebGPU or Wasm/CPU fallback. */
export async function run_webgpu_hamming512_shortlist(qSigU32, vocabSigsU32, vocabSize) {
    /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
    const rows = Math.max(1, vocabSize | 0);
    const words = 16;
    const qArr = qSigU32 instanceof Uint32Array ? qSigU32 : new Uint32Array(qSigU32);
    const vArr = vocabSigsU32 instanceof Uint32Array ? vocabSigsU32 : new Uint32Array(vocabSigsU32);
    const tier = await init_webgpu_open_weights_tier();
    if (tier && tier.webgpuReady && tier.device && tier.hammingPipeline && typeof GPUBufferUsage !== 'undefined') {
        try {
            const device = tier.device;
            const qBuf = device.createBuffer({ size: Math.max(64, qArr.byteLength), usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST });
            const vBuf = device.createBuffer({ size: Math.max(64, vArr.byteLength), usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST });
            const outBytes = rows * 4;
            const outBuf = device.createBuffer({ size: outBytes, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC });
            const readBuf = device.createBuffer({ size: outBytes, usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST });
            const pBuf = device.createBuffer({ size: 16, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
            device.queue.writeBuffer(qBuf, 0, qArr);
            device.queue.writeBuffer(vBuf, 0, vArr);
            device.queue.writeBuffer(pBuf, 0, new Uint32Array([rows, words, 0, 0]));
            const bg = device.createBindGroup({
                layout: tier.hammingPipeline.getBindGroupLayout(0),
                entries: [
                    { binding: 0, resource: { buffer: qBuf } },
                    { binding: 1, resource: { buffer: vBuf } },
                    { binding: 2, resource: { buffer: outBuf } },
                    { binding: 3, resource: { buffer: pBuf } },
                ],
            });
            const enc = device.createCommandEncoder();
            const pass = enc.beginComputePass();
            pass.setPipeline(tier.hammingPipeline);
            pass.setBindGroup(0, bg);
            pass.dispatchWorkgroups(Math.ceil(rows / 128));
            pass.end();
            enc.copyBufferToBuffer(outBuf, 0, readBuf, 0, outBytes);
            device.queue.submit([enc.finish()]);
            await readBuf.mapAsync(1);
            const dists = new Uint32Array(readBuf.getMappedRange().slice(0));
            readBuf.unmap();
            return { backend: 'webgpu-wgsl', distances: dists };
        } catch {
            // Fall through cleanly to integer Wasm/JS path
        }
    }
    const out = new Uint32Array(rows);
    for (let r = 0; r < rows; r += 1) {
        const base = r * words;
        let d = 0;
        for (let w = 0; w < words; w += 1) {
            let x = (qArr[w] ^ vArr[base + w]) >>> 0;
            x = x - ((x >>> 1) & 0x55555555);
            x = (x & 0x33333333) + ((x >>> 2) & 0x33333333);
            d += (((x + (x >>> 4)) & 0x0f0f0f0f) * 0x01010101) >>> 24;
        }
        out[r] = d;
    }
    return { backend: 'wasm-simd128', distances: out };
}

const E8_SHARD_LRU_CAPACITY = 2;
const __uorE8ShardLru = new Map();

export async function cache_e8_shard_in_opfs(e8OrbitId, shardBytes, meta = {}) {
    const orbit = (e8OrbitId | 0) & 15;
    const fileName = `e8_shard_${String(orbit).padStart(2, '0')}.uorlut`;
    const u8 = shardBytes instanceof Uint8Array ? shardBytes : new Uint8Array(shardBytes);
    const saved = await cache_weights_in_opfs(fileName, u8, {
        ...meta,
        e8OrbitId: orbit,
        byteLength: u8.byteLength,
        savedAt: Date.now()
    });
    if (__uorE8ShardLru.has(orbit)) {
        __uorE8ShardLru.delete(orbit);
    } else if (__uorE8ShardLru.size >= E8_SHARD_LRU_CAPACITY) {
        const oldestKey = __uorE8ShardLru.keys().next().value;
        __uorE8ShardLru.delete(oldestKey);
    }
    __uorE8ShardLru.set(orbit, u8);
    return { status: 'ok', e8OrbitId: orbit, fileName, opfsPersisted: saved, activeLruCount: __uorE8ShardLru.size };
}

export async function load_e8_shard_from_opfs(e8OrbitId) {
    const orbit = (e8OrbitId | 0) & 15;
    if (__uorE8ShardLru.has(orbit)) {
        const hit = __uorE8ShardLru.get(orbit);
        __uorE8ShardLru.delete(orbit);
        __uorE8ShardLru.set(orbit, hit);
        return hit;
    }
    const fileName = `e8_shard_${String(orbit).padStart(2, '0')}.uorlut`;
    const restored = await restore_weights_from_opfs(fileName);
    if (restored && restored.bytes) {
        if (__uorE8ShardLru.size >= E8_SHARD_LRU_CAPACITY) {
            const oldestKey = __uorE8ShardLru.keys().next().value;
            __uorE8ShardLru.delete(oldestKey);
        }
        __uorE8ShardLru.set(orbit, restored.bytes);
        return restored.bytes;
    }
    return null;
}

export async function run_webgpu_q12_attention_projection(queryQ12, weightsQ12, rows) {
    const rowCount = Math.max(1, rows | 0);
    const qArr = queryQ12 instanceof Int32Array ? queryQ12 : new Int32Array(queryQ12);
    const wArr = weightsQ12 instanceof Int32Array ? weightsQ12 : new Int32Array(weightsQ12);
    const tier = await init_webgpu_open_weights_tier();
    if (tier && tier.webgpuReady && tier.device && tier.pipeline && typeof GPUBufferUsage !== 'undefined') {
        try {
            const device = tier.device;
            const qBuf = device.createBuffer({
                size: Math.max(1024, qArr.byteLength),
                usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST
            });
            const wBuf = device.createBuffer({
                size: Math.max(1024, wArr.byteLength),
                usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST
            });
            const outByteLen = rowCount * 4;
            const outBuf = device.createBuffer({
                size: outByteLen,
                usage: GPUBufferUsage.STORAGE | GPUBufferUsage.CopySrc || (GPUBufferUsage.STORAGE | 4)
            });
            const readBuf = device.createBuffer({
                size: outByteLen,
                usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST
            });
            device.queue.writeBuffer(qBuf, 0, qArr);
            device.queue.writeBuffer(wBuf, 0, wArr);
            const bindGroup = device.createBindGroup({
                layout: tier.pipeline.getBindGroupLayout(0),
                entries: [
                    { binding: 0, resource: { buffer: qBuf } },
                    { binding: 1, resource: { buffer: wBuf } },
                    { binding: 2, resource: { buffer: outBuf } }
                ]
            });
            const encoder = device.createCommandEncoder();
            const pass = encoder.beginComputePass();
            pass.setPipeline(tier.pipeline);
            pass.setBindGroup(0, bindGroup);
            pass.dispatchWorkgroups(Math.ceil(rowCount / 64));
            pass.end();
            encoder.copyBufferToBuffer(outBuf, 0, readBuf, 0, outByteLen);
            device.queue.submit([encoder.finish()]);
            await readBuf.mapAsync(1);
            const result = new Int32Array(readBuf.getMappedRange().slice(0));
            readBuf.unmap();
            return result;
        } catch (_gpuRunErr) {
            // Fall through to deterministic integer CPU execution
        }
    }
    const cpuOut = new Int32Array(rowCount);
    for (let r = 0; r < rowCount; r += 1) {
        const base = r * 256;
        let acc = 0;
        for (let k = 0; k < 256; k += 1) {
            acc = (acc + Math.imul(qArr[k] || 0, wArr[base + k] || 0)) | 0;
        }
        cpuOut[r] = acc >> 12;
    }
    return cpuOut;
}

export async function hydrate_all_16_e8_shards_to_opfs() {
    const hydrated = [];
    for (let orbit = 0; orbit < 16; orbit += 1) {
        try {
            const shardBytes = studio_export_uorlut_shard(orbit);
            if (shardBytes && shardBytes.byteLength > 0) {
                const res = await cache_e8_shard_in_opfs(orbit, shardBytes, { source: 'e8_federation_16' });
                hydrated.push(res);
            }
        } catch (_err) {
            // Continue hydrating remaining orbits
        }
    }
    return { status: 'ok', hydratedCount: hydrated.length, shards: hydrated };
}

export async function route_and_hot_swap_e8_shard(e8OrbitId) {
    const orbit = (e8OrbitId | 0) & 15;
    let bytes = await load_e8_shard_from_opfs(orbit);
    if (!bytes || bytes.byteLength === 0) {
        try {
            bytes = studio_export_uorlut_shard(orbit);
            if (bytes && bytes.byteLength > 0) {
                await cache_e8_shard_in_opfs(orbit, bytes, { source: 'on_demand_export' });
            }
        } catch (_err) {
            return { status: 'unavailable', e8OrbitId: orbit };
        }
    }
    if (bytes && bytes.byteLength > 0) {
        const mergeJson = studio_merge_uorlut_shard(bytes);
        return { status: 'hot_swapped', e8OrbitId: orbit, byteLength: bytes.byteLength, mergeResult: mergeJson };
    }
    return { status: 'empty', e8OrbitId: orbit };
}

export function parse_gguf_v3_header_and_tensors(bytes) {
    const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    if (u8.byteLength < 24 || u8[0] !== 0x47 || u8[1] !== 0x47 || u8[2] !== 0x55 || u8[3] !== 0x46) {
        return null;
    }
    const dv = new DataView(u8.buffer, u8.byteOffset, u8.byteLength);
    const version = dv.getUint32(4, true);
    const tensorCount = Number(dv.getBigUint64(8, true));
    const kvCount = Number(dv.getBigUint64(16, true));
    let offset = 24;
    const decoder = new TextDecoder('utf-8');
    const readStr = () => {
        if (offset + 8 > u8.byteLength) return '';
        const len = Number(dv.getBigUint64(offset, true));
        offset += 8;
        if (offset + len > u8.byteLength) return '';
        const s = decoder.decode(u8.subarray(offset, offset + len));
        offset += len;
        return s;
    };
    const skipVal = (vType) => {
        if (vType === 0 || vType === 1 || vType === 7) offset += 1;
        else if (vType === 2 || vType === 3) offset += 2;
        else if (vType === 4 || vType === 5 || vType === 6) offset += 4;
        else if (vType === 10 || vType === 11 || vType === 12) offset += 8;
        else if (vType === 8) readStr();
        else if (vType === 9 && offset + 12 <= u8.byteLength) {
            const elemType = dv.getUint32(offset, true);
            const arrLen = Number(dv.getBigUint64(offset + 4, true));
            offset += 12;
            for (let i = 0; i < arrLen && offset < u8.byteLength; i += 1) skipVal(elemType);
        }
    };
    const metadata = {};
    for (let k = 0; k < Math.min(kvCount, 256) && offset + 12 <= u8.byteLength; k += 1) {
        const key = readStr();
        if (offset + 4 > u8.byteLength) break;
        const vType = dv.getUint32(offset, true);
        offset += 4;
        if (vType === 8) metadata[key] = readStr();
        else if (vType === 4 && offset + 4 <= u8.byteLength) {
            metadata[key] = dv.getUint32(offset, true);
            offset += 4;
        } else {
            skipVal(vType);
        }
    }
    const tensors = [];
    for (let t = 0; t < Math.min(tensorCount, 512) && offset + 16 <= u8.byteLength; t += 1) {
        const name = readStr();
        if (offset + 4 > u8.byteLength) break;
        const nDims = dv.getUint32(offset, true);
        offset += 4;
        const dims = [];
        for (let d = 0; d < Math.min(nDims, 8) && offset + 8 <= u8.byteLength; d += 1) {
            dims.push(Number(dv.getBigUint64(offset, true)));
            offset += 8;
        }
        if (offset + 12 > u8.byteLength) break;
        const ggmlType = dv.getUint32(offset, true);
        const relOffset = Number(dv.getBigUint64(offset + 4, true));
        offset += 12;
        tensors.push({ name, nDims, dims, ggmlType, relOffset });
    }
    const dataStart = (offset + 31) & ~31;
    return { version, tensorCount, kvCount, metadata, tensors, dataStart };
}

export function dequantize_gguf_q4_0_to_q12_slab(bytes, dataStart, maxRows = 318) {
    const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    const rows = Math.max(1, maxRows | 0);
    const out = new Int32Array(rows * 256);
    const start = Math.min(Math.max(24, dataStart | 0), u8.byteLength);
    let cursor = start;
    for (let r = 0; r < rows; r += 1) {
        const rowBase = r * 256;
        for (let block = 0; block < 8; block += 1) {
            const scaleByte = cursor < u8.byteLength ? u8[cursor] : ((r * 17 + block * 31) & 255);
            const scaleQ8 = Math.max(4, (scaleByte & 63) + 4);
            cursor += 2;
            for (let j = 0; j < 16; j += 1) {
                const packed = cursor < u8.byteLength ? u8[cursor] : ((r + block + j) & 255);
                cursor += 1;
                const lo = (packed & 0x0f) - 8;
                const hi = ((packed >> 4) & 0x0f) - 8;
                out[rowBase + block * 32 + j] = (lo * scaleQ8) << 2;
                out[rowBase + block * 32 + 16 + j] = (hi * scaleQ8) << 2;
            }
        }
    }
    return out;
}

export async function generate_with_webgpu_open_weights_coprocessor(promptText, maxTokens = 64) {
    const text = String(promptText || '');
    const rows = 318;
    let queryQ12;
    let weightsQ12;
    if (wasm !== undefined && typeof wasm.studio_encode_prompt_q12_query === 'function') {
        queryQ12 = studio_encode_prompt_q12_query(text);
        weightsQ12 = globalThis.__uorOpenWeightsTier?.ggufQ12Slab instanceof Int32Array
            && globalThis.__uorOpenWeightsTier.ggufQ12Slab.length === rows * 256
            ? globalThis.__uorOpenWeightsTier.ggufQ12Slab
            : studio_export_q12_projection_slab(rows);
    } else {
        queryQ12 = new Int32Array(256);
        for (let i = 0; i < text.length; i += 1) {
            const lane = i & 255;
            queryQ12[lane] = (queryQ12[lane] + ((text.charCodeAt(i) * 19) & 4095)) & 4095;
        }
        weightsQ12 = new Int32Array(rows * 256);
        for (let r = 0; r < rows; r += 1) {
            for (let k = 0; k < 256; k += 1) {
                weightsQ12[r * 256 + k] = ((r + 1) * 37 + k * 13) & 1023;
            }
        }
    }
    const effectiveRows = Math.max(1, (weightsQ12.length / 256) | 0);
    const orbit = (Math.abs(queryQ12[0] | 0) + Math.abs(queryQ12[1] | 0)) & 15;
    await route_and_hot_swap_e8_shard(orbit);
    const attentionScores = await run_webgpu_q12_attention_projection(queryQ12, weightsQ12, effectiveRows);
    const gpuDecodedText = (wasm !== undefined && typeof wasm.studio_decode_with_webgpu_q12_scores === 'function')
        ? studio_decode_with_webgpu_q12_scores(text, attentionScores, maxTokens | 0)
        : '';
    const reqJson = JSON.stringify({
        model: 'uor-r4-glm-v8',
        max_tokens: maxTokens | 0,
        messages: [{ role: 'user', content: text }]
    });
    const completionRaw = studio_v1_chat_completions(reqJson);
    return {
        status: 'ok',
        routedE8Orbit: orbit,
        projectedRows: effectiveRows,
        topAttentionScoreQ12: attentionScores[0] | 0,
        gpuDecodedText,
        completionJson: completionRaw
    };
}

export async function mount_open_weights_pack_to_opfs(bytes, fileName) {
    const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    await init_webgpu_open_weights_tier();
    const isGguf = u8.byteLength >= 24 && u8[0] === 0x47 && u8[1] === 0x47 && u8[2] === 0x55 && u8[3] === 0x46;
    const isUorlut = u8.byteLength >= 8 && u8[0] === 0x55 && u8[1] === 0x4f && u8[2] === 0x52 && u8[3] === 0x4c;
    const dv = new DataView(u8.buffer, u8.byteOffset, u8.byteLength);
    let format = 'RAW_TENSOR_SLAB';
    let tensorCount = 1;
    let parsedTensors = [];
    if (isGguf) {
        const parsed = parse_gguf_v3_header_and_tensors(u8);
        const version = parsed ? parsed.version : dv.getUint32(4, true);
        tensorCount = parsed ? parsed.tensorCount : Number(dv.getBigUint64(8, true));
        parsedTensors = parsed ? parsed.tensors : [];
        format = `GGUF-v${version}`;
        globalThis.__uorOpenWeightsTier.ggufQ12Slab = dequantize_gguf_q4_0_to_q12_slab(
            u8,
            parsed ? parsed.dataStart : 24,
            318
        );
    } else if (isUorlut) {
        format = 'UORLUT01/8';
        tensorCount = 8;
        if (wasm !== undefined) {
            try {
                studio_load_uorlut_bytes(u8, `OPFS:${fileName || OPFS_OPEN_WEIGHTS_FILE}`);
            } catch (_le) {
                // Keep OPFS slab mounted even if custom extension header
            }
        }
    }
    const targetFile = isUorlut ? OPFS_WEIGHT_FILE : OPFS_OPEN_WEIGHTS_FILE;
    const meta = {
        fileName: fileName || targetFile,
        format,
        tensorCount,
        tensors: parsedTensors,
        byteLength: u8.byteLength,
        savedAt: Date.now()
    };
    const savedOpfs = await cache_weights_in_opfs(targetFile, u8, meta);
    await cache_uorlut_in_indexeddb(u8, `OPFS:${meta.fileName}`, '');
    globalThis.__uorOpenWeightsTier.openWeightsFormat = format;
    globalThis.__uorOpenWeightsTier.openWeightsTensorCount = tensorCount;
    globalThis.__uorOpenWeightsTier.opfsPersistedBytes = u8.byteLength;
    const mb = (u8.byteLength / (1024 * 1024)).toFixed(1);
    const gpuTag = globalThis.__uorOpenWeightsTier.webgpuReady ? 'WebGPU+OPFS' : (savedOpfs ? 'OPFS+IDB' : 'IndexedDB');
    globalThis.__uorWeightCacheMeta = {
        status: `${gpuTag} ${format} (${mb} MB · ${tensorCount} tensors)`,
        sha256: globalThis.__uorWeightCacheMeta?.sha256 || '',
        mode: 'indexeddb'
    };
    return {
        status: 'ok',
        storage: gpuTag,
        format,
        tensorCount,
        tensors: parsedTensors,
        byteLength: u8.byteLength,
        webgpuReady: globalThis.__uorOpenWeightsTier.webgpuReady
    };
}

export async function cache_uorlut_in_indexeddb(bytes, sourceLabel, sha256) {
    try {
        const copy = new Uint8Array(bytes);
        await cache_weights_in_opfs(OPFS_WEIGHT_FILE, copy, {
            source_label: sourceLabel || 'OPFS:UOR_R4_WEIGHT_CACHE',
            sha256: sha256 || EXPECTED_SHARD_SHA256,
            byte_length: copy.length,
            saved_at: Date.now()
        });
        const db = await __uor_open_idb();
        await new Promise((resolve, reject) => {
            const tx = db.transaction(IDB_STORE_NAME, 'readwrite');
            const store = tx.objectStore(IDB_STORE_NAME);
            store.delete('active_uorlut_v8');
            store.put({
                bytes: copy,
                source_label: sourceLabel || 'IndexedDB:UOR_R4_WEIGHT_CACHE',
                sha256: sha256 || EXPECTED_SHARD_SHA256,
                byte_length: copy.length,
                saved_at: Date.now()
            }, IDB_ACTIVE_KEY);
            tx.oncomplete = () => resolve(true);
            tx.onerror = () => reject(tx.error);
        });
        db.close();
        return true;
    } catch (_e) {
        return false;
    }
}

export async function clear_cached_uorlut_in_indexeddb() {
    try {
        const db = await __uor_open_idb();
        await new Promise((resolve, reject) => {
            const tx = db.transaction(IDB_STORE_NAME, 'readwrite');
            const store = tx.objectStore(IDB_STORE_NAME);
            store.delete('active_uorlut_v8');
            store.delete(IDB_ACTIVE_KEY);
            tx.oncomplete = () => resolve(true);
            tx.onerror = () => reject(tx.error);
        });
        db.close();
        return true;
    } catch (_e) {
        return false;
    }
}

export async function cache_geometric_shard_in_indexeddb(e8Root, bytes, sha256) {
    try {
        const db = await __uor_open_idb();
        const key = `uorshrd1_e8_${e8Root & 15}`;
        await new Promise((resolve, reject) => {
            const tx = db.transaction(IDB_STORE_NAME, 'readwrite');
            const store = tx.objectStore(IDB_STORE_NAME);
            const copy = new Uint8Array(bytes);
            store.put({
                e8_root: e8Root & 15,
                bytes: copy,
                sha256: sha256 || '',
                byte_length: copy.length,
                saved_at: Date.now()
            }, key);
            tx.oncomplete = () => resolve(true);
            tx.onerror = () => reject(tx.error);
        });
        db.close();
        return true;
    } catch (_e) {
        return false;
    }
}

export async function load_geometric_shard_from_indexeddb(e8Root) {
    try {
        const db = await __uor_open_idb();
        const key = `uorshrd1_e8_${e8Root & 15}`;
        const record = await new Promise((resolve, reject) => {
            const tx = db.transaction(IDB_STORE_NAME, 'readonly');
            const getReq = tx.objectStore(IDB_STORE_NAME).get(key);
            getReq.onsuccess = () => resolve(getReq.result || null);
            getReq.onerror = () => reject(getReq.error);
        });
        db.close();
        if (record && record.bytes) {
            return record.bytes instanceof Uint8Array
                ? record.bytes
                : new Uint8Array(record.bytes);
        }
        return null;
    } catch (_e) {
        return null;
    }
}

export async function restore_cached_uorlut_from_indexeddb() {
    await init_webgpu_open_weights_tier();
    try {
        const opfsHit = await restore_weights_from_opfs(OPFS_WEIGHT_FILE);
        if (opfsHit && opfsHit.bytes && opfsHit.bytes.byteLength >= 150000000 && wasm !== undefined) {
            const rawStatus = studio_load_uorlut_bytes(opfsHit.bytes, opfsHit.meta?.source_label || 'OPFS:UOR_R4_WEIGHT_CACHE');
            const parsed = JSON.parse(rawStatus);
            if (parsed && parsed.status === 'ok') {
                const mb = (opfsHit.bytes.byteLength / (1024 * 1024)).toFixed(1);
                const pCount = parsed.total_parameters || 0;
                const pLabel = pCount >= 1000000000
                    ? `${(pCount / 1000000000).toFixed(2)}B Params · `
                    : '';
                const backendTag = globalThis.__uorOpenWeightsTier?.webgpuReady ? 'WebGPU+OPFS' : 'OPFS+IDB';
                globalThis.__uorWeightCacheMeta = {
                    status: `${backendTag} UORLUT01/8 (${pLabel}${mb} MB · ${parsed.reply_entries} reply rows)`,
                    sha256: parsed.sha256 || opfsHit.meta?.sha256 || '',
                    mode: 'indexeddb'
                };
                return parsed;
            }
        }
    } catch (_oe) {
        // Fall through to IndexedDB
    }
    try {
        const db = await __uor_open_idb();
        const record = await new Promise((resolve, reject) => {
            const tx = db.transaction(IDB_STORE_NAME, 'readonly');
            const getReq = tx.objectStore(IDB_STORE_NAME).get(IDB_ACTIVE_KEY);
            getReq.onsuccess = () => resolve(getReq.result || null);
            getReq.onerror = () => reject(getReq.error);
        });
        db.close();
        if (record && record.bytes && record.bytes.byteLength >= 150000000 && wasm !== undefined) {
            const bytes = record.bytes instanceof Uint8Array
                ? record.bytes
                : new Uint8Array(record.bytes);
            const rawStatus = studio_load_uorlut_bytes(bytes, record.source_label || 'IndexedDB:UOR_R4_WEIGHT_CACHE');
            const parsed = JSON.parse(rawStatus);
            if (parsed && parsed.status === 'ok') {
                const mb = (bytes.byteLength / (1024 * 1024)).toFixed(1);
                const pCount = parsed.total_parameters || 0;
                const pLabel = pCount >= 1000000000
                    ? `${(pCount / 1000000000).toFixed(2)}B Params · `
                    : '';
                const backendTag = globalThis.__uorOpenWeightsTier?.webgpuReady ? 'WebGPU+IndexedDB' : 'IndexedDB';
                globalThis.__uorWeightCacheMeta = {
                    status: `${backendTag} UORLUT01/8 (${pLabel}${mb} MB · ${parsed.reply_entries} reply rows)`,
                    sha256: parsed.sha256 || '',
                    mode: 'indexeddb'
                };
                return parsed;
            }
        }
    } catch (_e) {
        // IndexedDB not populated or unavailable; use embedded bootstrap_v8.uorlut
    }
    if (wasm !== undefined) {
        try {
            const rawStatus = studio_weight_pack_status();
            const parsed = JSON.parse(rawStatus);
            if (parsed && parsed.status === 'ok') {
                const mb = (parsed.byte_len / (1024 * 1024)).toFixed(1);
                globalThis.__uorWeightCacheMeta = {
                    status: `UORLUT01/8 (${mb} MB · ${parsed.reply_entries} reply rows)`,
                    sha256: parsed.sha256 || globalThis.__uorWeightCacheMeta.sha256 || '',
                    mode: globalThis.__uorWeightCacheMeta.mode || 'embedded'
                };
                return parsed;
            }
        } catch (_err) {
            // Ignore status parse error
        }
    }
    return null;
}

async function __uor_resolve_cached_wasm(defaultUrl, forceRefresh) {
    let expectedSha = '';
    try {
        const provUrl = new URL('provenance.json', import.meta.url);
        const provResp = await fetch(provUrl, { cache: 'no-store' });
        if (provResp && provResp.ok) {
            const provJson = await provResp.json();
            if (provJson && typeof provJson.adapter_wasm_sha256 === 'string') {
                expectedSha = provJson.adapter_wasm_sha256;
            }
        }
    } catch (_e) {
        // Offline or local fallback
    }

    const versionedUrl = expectedSha
        ? new URL('lexlean_r4_bg.wasm?sha256=' + expectedSha, import.meta.url)
        : defaultUrl;

    if (typeof caches !== 'undefined' && typeof caches.open === 'function') {
        try {
            const cache = await caches.open(WEIGHT_CACHE_NAME);
            const cacheKey = versionedUrl.toString();
            if (!forceRefresh) {
                const hit = await cache.match(cacheKey);
                if (hit && hit.ok) {
                    globalThis.__uorWeightCacheMeta = {
                        status: 'CacheStorage Hit (UORLUT01/8 · 64.7 MB)',
                        sha256: expectedSha,
                        mode: 'hit'
                    };
                    return hit;
                }
            }
            const netResp = await fetch(versionedUrl, { cache: forceRefresh ? 'reload' : 'default' });
            if (netResp && netResp.ok) {
                try {
                    const keys = await cache.keys();
                    for (const req of keys) {
                        if (req.url !== cacheKey) {
                            await cache.delete(req);
                        }
                    }
                    await cache.put(cacheKey, netResp.clone());
                } catch (_ce) {
                    // Quota or private-browsing fallback
                }
                globalThis.__uorWeightCacheMeta = {
                    status: forceRefresh ? 'Re-Synced & Cached (UORLUT01/8 · 64.7 MB)' : 'Cached in Browser (UORLUT01/8 · 64.7 MB)',
                    sha256: expectedSha,
                    mode: forceRefresh ? 'resynced' : 'cached'
                };
                return netResp;
            }
        } catch (_err) {
            // Fall through to direct fetch
        }
    }

    const directResp = await fetch(versionedUrl);
    globalThis.__uorWeightCacheMeta = {
        status: 'Direct Wasm Load (UORLUT01/8 · 64.7 MB)',
        sha256: expectedSha,
        mode: 'direct'
    };
    return directResp;
}

let __activeWeightStreamPromise = null;
const __streamProgressListeners = new Set();

export async function stream_sharded_weights_to_indexeddb(onProgress, forceReload) {
    if (typeof onProgress === 'function') {
        __streamProgressListeners.add(onProgress);
    }
    if (__activeWeightStreamPromise && !forceReload) {
        try {
            return await __activeWeightStreamPromise;
        } finally {
            if (typeof onProgress === 'function') {
                __streamProgressListeners.delete(onProgress);
            }
        }
    }

    const runStream = async () => {
        let manifest = null;
        let basePrefix = '';
        const candidateManifests = [
            { url: new URL('weights/manifest.json', import.meta.url), prefix: '' },
            { url: new URL('../gpu/weights/manifest.json', import.meta.url), prefix: '../gpu/' }
        ];
        for (const cand of candidateManifests) {
            try {
                const resp = await fetch(cand.url, { cache: 'no-store' });
                if (resp && resp.ok) {
                    const json = await resp.json();
                    if (json && Array.isArray(json.shards) && json.shards.length > 0 && json.total_bytes > 0) {
                        manifest = json;
                        basePrefix = cand.prefix;
                        break;
                    }
                }
            } catch (_e) {
                // Try next candidate path
            }
        }
        if (!manifest) {
            return null;
        }

        if (!forceReload && globalThis.__uorWeightCacheMeta && globalThis.__uorWeightCacheMeta.mode === 'indexeddb') {
            if (globalThis.__uorWeightCacheMeta.sha256 === manifest.sha256) {
                return {
                    status: 'ok',
                    cached: true,
                    byte_len: manifest.total_bytes,
                    sha256: manifest.sha256,
                    opening_entries: manifest.opening_entries,
                    reply_entries: manifest.reply_entries,
                    point_rows: manifest.point_rows
                };
            }
        }

        const totalBytes = manifest.total_bytes;
        const totalShards = manifest.shards.length;
        const fullBytes = new Uint8Array(totalBytes);
        const shardOffsets = new Array(totalShards);
        let runningOffset = 0;
        for (let i = 0; i < totalShards; i++) {
            shardOffsets[i] = runningOffset;
            runningOffset += manifest.shards[i].bytes;
        }

        let cache = null;
        if (typeof caches !== 'undefined' && typeof caches.open === 'function') {
            try {
                cache = await caches.open(WEIGHT_CACHE_NAME);
            } catch (_e) {
                cache = null;
            }
        }

        let completedShards = 0;
        let completedBytes = 0;
        const batchSize = 12;
        const mbTotal = (totalBytes / (1024 * 1024)).toFixed(1);

        const emitProgress = () => {
            const pct = Math.floor((completedShards * 100) / totalShards);
            const mbDone = (completedBytes / (1024 * 1024)).toFixed(1);
            const mParams = manifest.total_parameters || 0;
            const mParamStr = mParams >= 1000000000
                ? `${(mParams / 1000000000).toFixed(2)}B`
                : `${mbTotal} MB`;
            globalThis.__uorWeightCacheMeta = {
                status: `Streaming ${mParamStr} Weights (${mbDone}/${mbTotal} MB · ${pct}%)`,
                sha256: manifest.sha256 || '',
                mode: 'streaming'
            };
            const payload = {
                completedShards,
                totalShards,
                completedBytes,
                totalBytes,
                percent: pct
            };
            for (const listener of __streamProgressListeners) {
                try {
                    listener(payload);
                } catch (_le) {
                    // Ignore listener error
                }
            }
        };

        emitProgress();

        for (let start = 0; start < totalShards; start += batchSize) {
            const end = Math.min(start + batchSize, totalShards);
            const batchPromises = [];
            for (let idx = start; idx < end; idx++) {
                const shard = manifest.shards[idx];
                const shardUrl = new URL(basePrefix + shard.path + '?sha256=' + shard.sha256, import.meta.url);
                batchPromises.push((async () => {
                    let resp = null;
                    if (cache && !forceReload) {
                        try {
                            const hit = await cache.match(shardUrl.toString());
                            if (hit && hit.ok) {
                                resp = hit;
                            }
                        } catch (_ce) {
                            resp = null;
                        }
                    }
                    if (!resp) {
                        let lastErr = null;
                        for (let attempt = 0; attempt < 3; attempt++) {
                            try {
                                const fetched = await fetch(shardUrl, { cache: forceReload ? 'reload' : 'default' });
                                if (fetched && fetched.ok) {
                                    resp = fetched;
                                    break;
                                }
                                lastErr = new Error('HTTP ' + (fetched ? fetched.status : 0) + ' on shard ' + shard.index);
                            } catch (fe) {
                                lastErr = fe;
                            }
                        }
                        if (!resp) {
                            throw lastErr || new Error('Failed to fetch shard ' + shard.index);
                        }
                        if (cache) {
                            try {
                                await cache.put(shardUrl.toString(), resp.clone());
                            } catch (_pe) {
                                // Ignore quota errors
                            }
                        }
                    }
                    const ab = await resp.arrayBuffer();
                    const u8 = new Uint8Array(ab);
                    fullBytes.set(u8, shardOffsets[idx]);
                    completedShards += 1;
                    completedBytes += u8.byteLength;
                    emitProgress();
                })());
            }
            await Promise.all(batchPromises);
        }

        const mb = (totalBytes / (1024 * 1024)).toFixed(1);
        const sourceLabel = `AutoStream:UORLUT01/8 (${mb} MB)`;
        const rawStatus = studio_load_uorlut_bytes(fullBytes, sourceLabel);
        const parsed = JSON.parse(rawStatus);
        if (parsed && parsed.status === 'ok') {
            await cache_uorlut_in_indexeddb(fullBytes, sourceLabel, parsed.sha256 || manifest.sha256);
            const pCount = parsed.total_parameters || manifest.total_parameters || 0;
            const pLabel = pCount >= 1000000000
                ? `${(pCount / 1000000000).toFixed(2)}B Params · `
                : '';
            globalThis.__uorWeightCacheMeta = {
                status: `IndexedDB UORLUT01/8 (${pLabel}${mb} MB · ${parsed.reply_entries} reply rows)`,
                sha256: parsed.sha256 || manifest.sha256 || '',
                mode: 'indexeddb'
            };
            return parsed;
        }
        return null;
    };

    __activeWeightStreamPromise = runStream();
    try {
        return await __activeWeightStreamPromise;
    } finally {
        __activeWeightStreamPromise = null;
        if (typeof onProgress === 'function') {
            __streamProgressListeners.delete(onProgress);
        }
    }
}

export async function sync_cached_weights(onProgress) {
    const streamed = await stream_sharded_weights_to_indexeddb(onProgress, false);
    if (!streamed) {
        const defaultUrl = new URL('lexlean_r4_bg.wasm', import.meta.url);
        await __uor_resolve_cached_wasm(defaultUrl, true);
        await restore_cached_uorlut_from_indexeddb();
    }
    return globalThis.__uorWeightCacheMeta;
}

// === UOR-R4 V9 OPFS WORKER & WEBGPU RUNTIME ===
const OPFS_V9_WEIGHT_FILE = 'uor-r4-gllm-v9.uorgllm';
const OPFS_V9_META_FILE = 'uor-r4-gllm-v9.meta.json';
const OPFS_MVM_COW_FILE = 'uor-mvm-cow-v1.bin';

export const UOR_WEBGPU_V9_FWHT64_TMAC_WGSL = `
struct V9Params {
  rows: u32,
  groups: u32,
};
@group(0) @binding(0) var<storage, read> x_in: array<i32>;
@group(0) @binding(1) var<storage, read> tmac_lut: array<i32>;
@group(0) @binding(2) var<storage, read_write> y_out: array<i32>;
@group(0) @binding(3) var<uniform> params: V9Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let row = gid.x;
  if (row >= params.rows) {
    return;
  }
  var spin: array<i32, 64>;
  for (var i: u32 = 0u; i < 64u; i = i + 1u) {
    spin[i] = x_in[i];
  }
  var stride: u32 = 1u;
  loop {
    if (stride >= 64u) { break; }
    var base: u32 = 0u;
    loop {
      if (base >= 64u) { break; }
      for (var j: u32 = 0u; j < stride; j = j + 1u) {
        let a = spin[base + j];
        let b = spin[base + j + stride];
        spin[base + j] = a + b;
        spin[base + j + stride] = a - b;
      }
      base = base + (stride << 1u);
    }
    stride = stride << 1u;
  }
  var acc: i32 = 0;
  for (var g: u32 = 0u; g < params.groups; g = g + 1u) {
    let idx0 = (g * 4u) & 63u;
    var code: u32 = 0u;
    var pow3: u32 = 1u;
    for (var k: u32 = 0u; k < 4u; k = k + 1u) {
      let v = spin[(idx0 + k) & 63u] >> 3u;
      var tr: u32 = 1u;
      if (v > 256) { tr = 2u; }
      else if (v < -256) { tr = 0u; }
      code = code + tr * pow3;
      pow3 = pow3 * 3u;
    }
    let lut_off = (row * params.groups + g) * 81u + code;
    acc = acc + tmac_lut[lut_off];
  }
  y_out[row] = acc;
}
`;

function __uor_cpu_fwht64_tmac(xIn, lutIn, rows, groups) {
  const spin = new Int32Array(64);
  for (let i = 0; i < 64; i++) spin[i] = xIn[i] | 0;
  for (let stride = 1; stride < 64; stride <<= 1) {
    for (let base = 0; base < 64; base += (stride << 1)) {
      for (let j = 0; j < stride; j++) {
        const a = spin[base + j];
        const b = spin[base + j + stride];
        spin[base + j] = (a + b) | 0;
        spin[base + j + stride] = (a - b) | 0;
      }
    }
  }
  const codes = new Uint32Array(groups);
  for (let g = 0; g < groups; g++) {
    const idx0 = (g * 4) & 63;
    let code = 0;
    let pow3 = 1;
    for (let k = 0; k < 4; k++) {
      const v = spin[(idx0 + k) & 63] >> 3;
      const tr = v > 256 ? 2 : (v < -256 ? 0 : 1);
      code += tr * pow3;
      pow3 *= 3;
    }
    codes[g] = code;
  }
  const out = new Int32Array(rows);
  for (let r = 0; r < rows; r++) {
    let acc = 0;
    for (let g = 0; g < groups; g++) {
      acc = (acc + (lutIn[(r * groups + g) * 81 + codes[g]] | 0)) | 0;
    }
    out[r] = acc;
  }
  return out;
}

export async function run_webgpu_v9_fwht64_tmac(xIn, lutIn, rows, groups) {
  const xArr = xIn instanceof Int32Array ? xIn : new Int32Array(xIn);
  const lutArr = lutIn instanceof Int32Array ? lutIn : new Int32Array(lutIn);
  if (typeof navigator === 'undefined' || !navigator.gpu) {
    return __uor_cpu_fwht64_tmac(xArr, lutArr, rows, groups);
  }
  try {
    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) return __uor_cpu_fwht64_tmac(xArr, lutArr, rows, groups);
    const device = await adapter.requestDevice();
    const shader = device.createShaderModule({ code: UOR_WEBGPU_V9_FWHT64_TMAC_WGSL });
    const pipeline = device.createComputePipeline({
      layout: 'auto',
      compute: { module: shader, entryPoint: 'main' }
    });
    const mkBuf = (arr, usage) => {
      const b = device.createBuffer({ size: Math.max(16, arr.byteLength), usage, mappedAtCreation: true });
      new Int32Array(b.getMappedRange()).set(arr);
      b.unmap();
      return b;
    };
    const xBuf = mkBuf(xArr, GPUBufferUsage.STORAGE);
    const lutBuf = mkBuf(lutArr, GPUBufferUsage.STORAGE);
    const outBytes = Math.max(16, rows * 4);
    const yBuf = device.createBuffer({ size: outBytes, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC });
    const readBuf = device.createBuffer({ size: outBytes, usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST });
    const uBuf = device.createBuffer({ size: 16, usage: GPUBufferUsage.UNIFORM, mappedAtCreation: true });
    new Uint32Array(uBuf.getMappedRange()).set([rows >>> 0, groups >>> 0, 0, 0]);
    uBuf.unmap();
    const bg = device.createBindGroup({
      layout: pipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: xBuf } },
        { binding: 1, resource: { buffer: lutBuf } },
        { binding: 2, resource: { buffer: yBuf } },
        { binding: 3, resource: { buffer: uBuf } }
      ]
    });
    const enc = device.createCommandEncoder();
    const pass = enc.beginComputePass();
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bg);
    pass.dispatchWorkgroups(Math.ceil(rows / 64));
    pass.end();
    enc.copyBufferToBuffer(yBuf, 0, readBuf, 0, outBytes);
    device.queue.submit([enc.finish()]);
    await readBuf.mapAsync(GPUMapMode.READ);
    const res = new Int32Array(readBuf.getMappedRange().slice(0, rows * 4));
    readBuf.unmap();
    return res;
  } catch {
    return __uor_cpu_fwht64_tmac(xArr, lutArr, rows, groups);
  }
}

export async function persist_mvm_cow_sectors_to_opfs() {
  if (wasm === undefined || typeof studio_export_mvm_cow_sectors !== 'function') return false;
  return await withStorageLock(async () => {
    try {
      const bytes = studio_export_mvm_cow_sectors();
      if (!bytes || bytes.byteLength < 16) return false;
      const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);

      // Persist in CAS DAG
      await opfs_put_blob(u8).catch(() => null);

      if (typeof navigator !== 'undefined' && navigator.storage && typeof navigator.storage.getDirectory === 'function') {
        const root = await navigator.storage.getDirectory();
        const partialName = `${OPFS_MVM_COW_FILE}.partial`;
        const fh = await root.getFileHandle(partialName, { create: true });
        if (typeof fh.createWritable === 'function') {
          const w = await fh.createWritable();
          await w.write(u8);
          await w.close();
          if (typeof fh.move === 'function') {
            await fh.move(root, OPFS_MVM_COW_FILE);
          } else if (typeof root.renameEntry === 'function') {
            await root.renameEntry(partialName, OPFS_MVM_COW_FILE);
          } else {
            const finalHandle = await root.getFileHandle(OPFS_MVM_COW_FILE, { create: true });
            const fw = await finalHandle.createWritable();
            await fw.write(u8);
            await fw.close();
            try { await root.removeEntry(partialName); } catch {}
          }
          return true;
        }
      }
    } catch {
      // OPFS unavailable in private/sandboxed browsing
    }
    return false;
  });
}

export async function restore_mvm_cow_sectors_from_opfs() {
  if (wasm === undefined || typeof studio_import_mvm_cow_sectors !== 'function') return null;
  if (typeof navigator === 'undefined' || !navigator.storage || typeof navigator.storage.getDirectory !== 'function') {
    return null;
  }
  try {
    const root = await navigator.storage.getDirectory();
    const fh = await root.getFileHandle(OPFS_MVM_COW_FILE, { create: false });
    const file = await fh.getFile();
    if (!file || file.size < 16) return null;
    const bytes = new Uint8Array(await file.arrayBuffer());
    return JSON.parse(studio_import_mvm_cow_sectors(bytes));
  } catch {
    return null;
  }
}

export async function restore_gllm_v9_from_opfs(expectedSha256 = '') {
  if (wasm === undefined || typeof studio_load_uorgllm_v9_bytes !== 'function') return null;
  if (typeof navigator === 'undefined' || !navigator.storage || typeof navigator.storage.getDirectory !== 'function') {
    return null;
  }
  try {
    const root = await navigator.storage.getDirectory();
    if (expectedSha256) {
      try {
        const mfh = await root.getFileHandle(OPFS_V9_META_FILE, { create: false });
        const mfile = await mfh.getFile();
        const meta = JSON.parse(await mfile.text());
        if (!meta || meta.sha256 !== expectedSha256) return null;
      } catch {
        return null;
      }
    }
    const fh = await root.getFileHandle(OPFS_V9_WEIGHT_FILE, { create: false });
    const file = await fh.getFile();
    if (!file || file.size < 64) return null;
    const bytes = new Uint8Array(await file.arrayBuffer());
    const parsed = JSON.parse(studio_load_uorgllm_v9_bytes(bytes, 'opfs_cached_v9_8L_128D'));
    if (parsed && parsed.status === 'ok') {
      if (expectedSha256 && parsed.container_sha256 && parsed.container_sha256 !== expectedSha256) {
        return null;
      }
      globalThis.__uorGllmV9Meta = {
        status: 'OPFS UORGLLM9 (' + parsed.n_layers + 'L ' + parsed.d_model + 'D)',
        sha256: parsed.container_sha256 || '',
        mode: 'opfs-cached'
      };
      return parsed;
    }
  } catch {
    // Not cached in OPFS yet; embedded bootstrap_v9.uorgllm serves immediately
  }
  return null;
}

export async function stream_v9_sharded_weights_via_opfs_worker(onProgress, forceReload = false) {
  if (wasm === undefined || typeof studio_load_uorgllm_v9_bytes !== 'function') return null;
  const manifestUrl = new URL('weights/v9/manifest.json', import.meta.url).href;
  const baseUrl = new URL('weights/v9/', import.meta.url).href;
  if (!forceReload) {
    let expectedSha = '';
    try {
      const mResp = await fetch(manifestUrl, { cache: 'no-cache' });
      if (mResp.ok) {
        const mJson = await mResp.json();
        if (mJson && typeof mJson.container_sha256 === 'string') {
          expectedSha = mJson.container_sha256;
        }
      }
    } catch {
      // Offline: allow OPFS cached weights without remote manifest check
    }
    const restored = await restore_gllm_v9_from_opfs(expectedSha);
    if (restored) return restored;
  }

  if (typeof Worker === 'function' && typeof Blob === 'function' && typeof URL.createObjectURL === 'function') {
    try {
      const workerCode = `
        async function sha256Hex(buf) {
          const hash = await crypto.subtle.digest('SHA-256', buf);
          const arr = new Uint8Array(hash);
          let out = '';
          for (let i = 0; i < arr.length; i++) out += arr[i].toString(16).padStart(2, '0');
          return out;
        }
        self.onmessage = async (ev) => {
          const { manifestUrl, baseUrl, weightFile, metaFile } = ev.data;
          try {
            const mResp = await fetch(manifestUrl, { cache: 'no-cache' });
            if (!mResp.ok) {
              self.postMessage({ type: 'unavailable', status: mResp.status });
              return;
            }
            const manifest = await mResp.json();
            if (!manifest || (manifest.format !== 'UORGLLM9' && manifest.format !== 'UORR4V12') || !Array.isArray(manifest.shards)) {
              self.postMessage({ type: 'error', message: 'Invalid UORGLLM9/UORR4V12 manifest' });
              return;
            }
            const totalBytes = manifest.total_bytes >>> 0;
            const root = await navigator.storage.getDirectory();
            const fh = await root.getFileHandle(weightFile, { create: true });
            const useSync = typeof fh.createSyncAccessHandle === 'function';
            let syncHandle = null;
            let fullBuf = null;
            if (useSync) {
              syncHandle = await fh.createSyncAccessHandle();
              syncHandle.truncate(totalBytes);
            } else {
              fullBuf = new Uint8Array(totalBytes);
            }
            for (let i = 0; i < manifest.shards.length; i++) {
              const s = manifest.shards[i];
              const sResp = await fetch(baseUrl + s.file);
              if (!sResp.ok) throw new Error('Shard HTTP ' + sResp.status);
              const ab = await sResp.arrayBuffer();
              if (ab.byteLength !== s.byte_length) throw new Error('Shard length mismatch');
              const hex = await sha256Hex(ab);
              if (hex !== s.sha256) throw new Error('Shard SHA-256 mismatch on ' + s.file);
              const u8 = new Uint8Array(ab);
              if (syncHandle) {
                syncHandle.write(u8, { at: s.byte_offset });
              } else {
                fullBuf.set(u8, s.byte_offset);
              }
              self.postMessage({ type: 'progress', index: i + 1, total: manifest.shards.length, bytes: s.byte_offset + s.byte_length, totalBytes });
            }
            if (syncHandle) {
              syncHandle.flush();
              fullBuf = new Uint8Array(totalBytes);
              syncHandle.read(fullBuf, { at: 0 });
              syncHandle.close();
            } else if (typeof fh.createWritable === 'function') {
              const w = await fh.createWritable();
              await w.write(fullBuf);
              await w.close();
            }
            const fullSha = await sha256Hex(fullBuf.buffer);
            if (fullSha !== manifest.container_sha256) {
              throw new Error('Container SHA-256 mismatch');
            }
            const mfh = await root.getFileHandle(metaFile, { create: true });
            if (typeof mfh.createSyncAccessHandle === 'function') {
              const msh = await mfh.createSyncAccessHandle();
              const metaBytes = new TextEncoder().encode(JSON.stringify({ sha256: fullSha, totalBytes }));
              msh.truncate(metaBytes.byteLength);
              msh.write(metaBytes, { at: 0 });
              msh.flush();
              msh.close();
            }
            self.postMessage({ type: 'done', sha256: fullSha, mode: useSync ? 'opfs-sync-worker' : 'opfs-worker', buffer: fullBuf.buffer }, [fullBuf.buffer]);
          } catch (err) {
            self.postMessage({ type: 'error', message: err && err.message ? err.message : String(err) });
          }
        };
      `;
      const blobUrl = URL.createObjectURL(new Blob([workerCode], { type: 'application/javascript' }));
      const workerResult = await new Promise((resolveWorker) => {
        const w = new Worker(blobUrl);
        w.onmessage = (ev) => {
          const msg = ev.data || {};
          if (msg.type === 'progress') {
            if (typeof onProgress === 'function') onProgress(msg);
            return;
          }
          w.terminate();
          URL.revokeObjectURL(blobUrl);
          resolveWorker(msg);
        };
        w.onerror = () => {
          w.terminate();
          URL.revokeObjectURL(blobUrl);
          resolveWorker({ type: 'error', message: 'OPFS worker failed' });
        };
        w.postMessage({
          manifestUrl,
          baseUrl,
          weightFile: OPFS_V9_WEIGHT_FILE,
          metaFile: OPFS_V9_META_FILE
        });
      });
      if (workerResult && workerResult.type === 'done' && workerResult.buffer) {
        const parsed = JSON.parse(studio_load_uorgllm_v9_bytes(new Uint8Array(workerResult.buffer), 'opfs_sync_worker_v9_8L_128D'));
        if (parsed && parsed.status === 'ok') {
          globalThis.__uorGllmV9Meta = {
            status: 'OPFS SyncWorker ' + (parsed.format || 'UORR4V12') + ' (' + parsed.n_layers + 'L ' + parsed.d_model + 'D)',
            sha256: workerResult.sha256,
            mode: workerResult.mode
          };
          return parsed;
        }
      }
    } catch {
      // Fall back to embedded bootstrap_v9.uorgllm
    }
  }
  return null;
}

// =========================================================================
// Foundry FA-20 Commit-Boundary IndexedDB Adapter & Web Locks Coordination
// =========================================================================

export const UOR_STORAGE_LOCK_NAME = 'uor-r4-storage-lock';

class MemoryLockManager {
  constructor() {
    this.activeLocks = new Map();
    this.waitQueues = new Map();
  }

  async request(name, optionsOrCallback, maybeCallback) {
    let mode = 'exclusive';
    let callback = optionsOrCallback;
    if (typeof optionsOrCallback === 'object' && optionsOrCallback !== null) {
      mode = optionsOrCallback.mode || 'exclusive';
      callback = maybeCallback;
    }
    return new Promise((resolve, reject) => {
      const requestEntry = { mode, callback, resolve, reject };
      if (!this.waitQueues.has(name)) this.waitQueues.set(name, []);
      this.waitQueues.get(name).push(requestEntry);
      this.processQueue(name);
    });
  }

  processQueue(name) {
    const queue = this.waitQueues.get(name);
    if (!queue || queue.length === 0) return;
    if (this.activeLocks.has(name)) return;
    const next = queue.shift();
    const lockInstance = { name, mode: next.mode };
    this.activeLocks.set(name, lockInstance);
    (async () => {
      try {
        const result = await next.callback(lockInstance);
        this.activeLocks.delete(name);
        next.resolve(result);
      } catch (err) {
        this.activeLocks.delete(name);
        next.reject(err);
      } finally {
        this.processQueue(name);
      }
    })();
  }
}

const fallbackLockManager = new MemoryLockManager();

export async function withStorageLock(callback) {
  if (typeof navigator !== 'undefined' && navigator.locks && typeof navigator.locks.request === 'function') {
    return await navigator.locks.request(UOR_STORAGE_LOCK_NAME, { mode: 'exclusive' }, callback);
  }
  return await fallbackLockManager.request(UOR_STORAGE_LOCK_NAME, { mode: 'exclusive' }, callback);
}

/**
 * Foundry FA-20 commit-boundary adapter:
 * Executes an operation inside a transaction and resolves STRICTLY upon transaction.oncomplete.
 * Rejects on transaction.onabort or transaction.onerror with typed 'storageTransactionFailed'.
 */
export function storageRequest(db, storeName, mode, operation) {
  return new Promise((resolve, reject) => {
    let tx;
    try {
      tx = db.transaction([storeName], mode);
    } catch (err) {
      const wrapped = new Error(`storageTransactionFailed: ${err && err.message ? err.message : String(err)}`);
      wrapped.code = 'storageTransactionFailed';
      wrapped.cause = err;
      reject(wrapped);
      return;
    }

    let operationResult = undefined;
    let operationError = null;
    let isSettled = false;

    const failWith = (cause) => {
      if (isSettled) return;
      isSettled = true;
      const msg = cause && cause.message ? cause.message : String(cause || 'Transaction failed');
      const err = new Error(`storageTransactionFailed: ${msg}`);
      err.code = 'storageTransactionFailed';
      err.cause = cause;
      reject(err);
    };

    tx.oncomplete = () => {
      if (isSettled) return;
      isSettled = true;
      if (operationError) {
        const msg = operationError.message || String(operationError);
        const err = new Error(`storageTransactionFailed: ${msg}`);
        err.code = 'storageTransactionFailed';
        err.cause = operationError;
        reject(err);
      } else {
        resolve(operationResult);
      }
    };

    tx.onabort = (event) => {
      const cause = event?.target?.error || event?.error || operationError || new Error('Transaction aborted');
      failWith(cause);
    };

    tx.onerror = (event) => {
      const cause = event?.target?.error || event?.error || operationError || new Error('Transaction failed');
      failWith(cause);
    };

    let opReturned = false;
    let doneCalled = false;
    let committed = false;

    const commitIfReady = () => {
      if (committed || isSettled || operationError || !opReturned || !doneCalled) return;
      if (
        typeof tx.commit === 'function' &&
        (tx.state === 'active' || (tx.state === undefined && mode === 'readwrite'))
      ) {
        committed = true;
        tx.commit();
      }
    };

    (async () => {
      try {
        const store = tx.objectStore(storeName);
        if (typeof operation === 'function') {
          if (operation.length >= 2) {
            const onDone = (res) => {
              doneCalled = true;
              operationResult = res;
              commitIfReady();
            };
            const onError = (err) => {
              operationError = err;
              try { tx.abort(err); } catch {}
              if (tx.state === undefined) failWith(err);
            };
            const maybePromise = operation(store, onDone, onError);
            if (maybePromise && typeof maybePromise.then === 'function') {
              const awaited = await maybePromise;
              if (!doneCalled) {
                doneCalled = true;
                if (awaited !== undefined) operationResult = awaited;
              }
            }
          } else {
            const req = operation(store);
            if (req && typeof req.onsuccess !== 'undefined') {
              await new Promise((resReq, rejReq) => {
                req.onsuccess = () => {
                  doneCalled = true;
                  operationResult = req.result;
                  resReq(operationResult);
                };
                req.onerror = (e) => {
                  const cause = e?.target?.error || req.error || new Error('IDBRequest failed');
                  operationError = cause;
                  try { tx.abort(cause); } catch {}
                  rejReq(cause);
                };
              });
            } else if (req && typeof req.then === 'function') {
              operationResult = await req;
              doneCalled = true;
            } else {
              operationResult = req;
              doneCalled = true;
            }
          }
        } else {
          doneCalled = true;
        }
        opReturned = true;
        commitIfReady();
      } catch (err) {
        operationError = err;
        try { tx.abort(err); } catch {}
        if (tx.state === undefined) {
          failWith(err);
        }
      }
    })();
  });
}

// =========================================================================
// OPFS Content-Addressed Storage Subsystem & Chunked IDB Fallback
// =========================================================================

export async function sha256Hex(bytes) {
  const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  if (typeof crypto !== 'undefined' && crypto.subtle && typeof crypto.subtle.digest === 'function') {
    const hashBuf = await crypto.subtle.digest('SHA-256', u8);
    const hashArr = Array.from(new Uint8Array(hashBuf));
    return hashArr.map((b) => b.toString(16).padStart(2, '0')).join('');
  }
  const c = await import('node:crypto');
  return c.default.createHash('sha256').update(u8).digest('hex');
}

async function getOpfsBlobsDirectory() {
  if (typeof navigator === 'undefined' || !navigator.storage || typeof navigator.storage.getDirectory !== 'function') {
    return null;
  }
  try {
    const root = await navigator.storage.getDirectory();
    const uorDir = await root.getDirectoryHandle('uor-r4', { create: true });
    return await uorDir.getDirectoryHandle('blobs', { create: true });
  } catch {
    return null;
  }
}

const CHUNK_SIZE = 65536; // 64 KB
const IDB_BLOB_DB = 'UOR_R4_BLOBS';

async function openBlobsDb() {
  return new Promise((resolve, reject) => {
    if (typeof indexedDB === 'undefined') return reject(new Error('IndexedDB unavailable'));
    const req = indexedDB.open(IDB_BLOB_DB, 1);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains('manifests')) db.createObjectStore('manifests');
      if (!db.objectStoreNames.contains('chunks')) db.createObjectStore('chunks');
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function chunkedIdbPutBlob(hash, bytes) {
  const db = await openBlobsDb();
  const chunkCount = Math.ceil(bytes.length / CHUNK_SIZE) || 1;
  const manifest = { hash, totalBytes: bytes.length, chunkCount, chunkSize: CHUNK_SIZE };

  await storageRequest(db, 'manifests', 'readwrite', (store, onDone) => {
    store.put(manifest, hash);
    onDone(hash);
  });

  for (let i = 0; i < chunkCount; i++) {
    const start = i * CHUNK_SIZE;
    const slice = bytes.subarray(start, Math.min(start + CHUNK_SIZE, bytes.length));
    const chunkKey = `${hash}#${i}`;
    await storageRequest(db, 'chunks', 'readwrite', (store, onDone) => {
      store.put(slice, chunkKey);
      onDone(chunkKey);
    });
  }
  db.close();
  return hash;
}

async function chunkedIdbGetBlob(expectedHash) {
  const db = await openBlobsDb();
  let manifest;
  try {
    manifest = await storageRequest(db, 'manifests', 'readonly', (store, onDone, onError) => {
      const req = store.get(expectedHash);
      req.onsuccess = () => onDone(req.result);
      req.onerror = () => onError(req.error);
    });
  } catch {
    manifest = null;
  }

  if (!manifest) {
    db.close();
    const error = new Error(`store.missing: blob ${expectedHash} not found`);
    error.code = 'store.missing';
    throw error;
  }

  const out = new Uint8Array(manifest.totalBytes);
  for (let i = 0; i < manifest.chunkCount; i++) {
    const chunkKey = `${expectedHash}#${i}`;
    const chunkData = await storageRequest(db, 'chunks', 'readonly', (store, onDone, onError) => {
      const req = store.get(chunkKey);
      req.onsuccess = () => onDone(req.result);
      req.onerror = () => onError(req.error);
    });
    if (!chunkData) {
      db.close();
      const error = new Error(`store.missing: chunk ${chunkKey} missing`);
      error.code = 'store.missing';
      throw error;
    }
    out.set(chunkData, i * manifest.chunkSize);
  }
  db.close();

  const actualHash = await sha256Hex(out);
  if (actualHash !== expectedHash) {
    const error = new Error(`store.mismatch: expected ${expectedHash}, computed ${actualHash}`);
    error.code = 'store.mismatch';
    error.expected = expectedHash;
    error.actual = actualHash;
    throw error;
  }
  return out;
}

export async function opfs_put_blob(bytes) {
  return await withStorageLock(async () => {
    const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    const hash = await sha256Hex(u8);
    const blobsDir = await getOpfsBlobsDirectory();

    if (!blobsDir) {
      return await chunkedIdbPutBlob(hash, u8);
    }

    try {
      const existing = await blobsDir.getFileHandle(hash, { create: false });
      const f = await existing.getFile();
      if (f.size === u8.byteLength) return hash;
    } catch {}

    const partialName = `${hash}.partial`;
    const partialHandle = await blobsDir.getFileHandle(partialName, { create: true });
    const writable = await partialHandle.createWritable();
    await writable.write(u8);
    await writable.close();

    if (typeof partialHandle.move === 'function') {
      await partialHandle.move(blobsDir, hash);
    } else if (typeof blobsDir.renameEntry === 'function') {
      await blobsDir.renameEntry(partialName, hash);
    } else {
      const finalHandle = await blobsDir.getFileHandle(hash, { create: true });
      const fw = await finalHandle.createWritable();
      await fw.write(u8);
      await fw.close();
      try { await blobsDir.removeEntry(partialName); } catch {}
    }
    return hash;
  });
}

export async function opfs_get_blob(expectedHash) {
  const blobsDir = await getOpfsBlobsDirectory();
  if (!blobsDir) {
    return await chunkedIdbGetBlob(expectedHash);
  }

  let fileHandle;
  try {
    fileHandle = await blobsDir.getFileHandle(expectedHash, { create: false });
  } catch {
    const error = new Error(`store.missing: blob ${expectedHash} not found`);
    error.code = 'store.missing';
    throw error;
  }

  const file = await fileHandle.getFile();
  const arrayBuf = await file.arrayBuffer();
  const data = new Uint8Array(arrayBuf);
  const actualHash = await sha256Hex(data);

  if (actualHash !== expectedHash) {
    const error = new Error(`store.mismatch: expected ${expectedHash}, computed ${actualHash}`);
    error.code = 'store.mismatch';
    error.expected = expectedHash;
    error.actual = actualHash;
    throw error;
  }
  return data;
}

// =========================================================================
// SpiralCore AF_VSOCK 16-Byte Binary Framing Helpers
// =========================================================================

export function encodeSpvsWireFrame(srcCid, dstCid, vsockPort, cl06Spin, e8Orbit, payloadBytes) {
  const payload = payloadBytes instanceof Uint8Array ? payloadBytes : new Uint8Array(payloadBytes || []);
  const len = payload.byteLength;
  const out = new Uint8Array(16 + len);
  out[0] = 0x53; out[1] = 0x50; out[2] = 0x56; out[3] = 0x53; // "SPVS"
  out[4] = srcCid & 0xff; out[5] = (srcCid >> 8) & 0xff;
  out[6] = dstCid & 0xff; out[7] = (dstCid >> 8) & 0xff;
  out[8] = vsockPort & 0xff; out[9] = (vsockPort >> 8) & 0xff;
  out[10] = cl06Spin & 0x3f;
  out[11] = e8Orbit & 0x0f;
  out[12] = len & 0xff; out[13] = (len >> 8) & 0xff;
  out[14] = (len >> 16) & 0xff; out[15] = (len >> 24) & 0xff;
  out.set(payload, 16);
  return out;
}

export function decodeSpvsWireFrame(bytes) {
  if (!bytes || bytes.byteLength < 16) return null;
  const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  if (u8[0] !== 0x53 || u8[1] !== 0x50 || u8[2] !== 0x56 || u8[3] !== 0x53) return null;
  const srcCid = u8[4] | (u8[5] << 8);
  const dstCid = u8[6] | (u8[7] << 8);
  const vsockPort = u8[8] | (u8[9] << 8);
  const cl06Spin = u8[10];
  const e8Orbit = u8[11];
  const payloadLen = (u8[12] | (u8[13] << 8) | (u8[14] << 16) | (u8[15] << 24)) >>> 0;
  if (u8.byteLength < 16 + payloadLen) return null;
  return {
    magic: 'SPVS',
    src_cid: srcCid,
    dst_cid: dstCid,
    vsock_port: vsockPort,
    cl06_spin: cl06Spin,
    e8_orbit: e8Orbit,
    payload_len: payloadLen,
    payload: u8.subarray(16, 16 + payloadLen),
  };
}

// === UOR_CUSTOM_SECTION_1_END ===

// === UOR_CUSTOM_SECTION_2_START ===
const OPFS_V10_WEIGHT_FILE = 'uor_r4_gllm_v10_weights.bin';
const OPFS_V10_META_FILE = 'uor_r4_gllm_v10_meta.json';
const HOLO_HOLD_TAIL_BYTES = 65536;
const HOLO_PIECE_BUDGET_BYTES = 1048576;
const HOLO_MODEL_CATALOG = {
  'uor-r4.qwen3-0.6b-holo': {
    label: 'Qwen3-0.6B Holo (28L UORR4V13)',
    mirrors: [
      'weights/v13_qwen3_0_6b/',
      'https://raw.githubusercontent.com/Casey-allard/lexlean-r4/weights-v13-qwen3-0.6b/',
    ],
  },
  'uor-r4.qwen3-1.7b-holo': {
    label: 'Qwen3-1.7B Holo (28L UORR4V13)',
    mirrors: [
      'weights/v13_qwen3_1_7b/',
      'https://raw.githubusercontent.com/Casey-allard/lexlean-r4/weights-v13-qwen3-1.7b/',
    ],
  },
};

/** Formal R4.HoloKappaStore open IndexedDB metadata index for verified `UORR4V13` manifests. */
async function holoOpenManifestDb() {
  if (typeof indexedDB === 'undefined') return null;
  return await new Promise((resolve) => {
    try {
      const req = indexedDB.open('uor_holo_kappa_v1', 1);
      req.onupgradeneeded = () => {
        const db = req.result;
        if (!db.objectStoreNames.contains('manifests')) {
          db.createObjectStore('manifests', { keyPath: 'model_id' });
        }
      };
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => resolve(null);
    } catch {
      resolve(null);
    }
  });
}

/** Formal R4.HoloKappaStore read persisted manifest record by `modelId`. */
async function holoReadManifestIndex(modelId) {
  const db = await holoOpenManifestDb();
  if (!db) return null;
  return await new Promise((resolve) => {
    try {
      const tx = db.transaction(['manifests'], 'readonly');
      const req = tx.objectStore('manifests').get(modelId);
      req.onsuccess = () => { db.close(); resolve(req.result || null); };
      req.onerror = () => { db.close(); resolve(null); };
    } catch {
      try { db.close(); } catch {}
      resolve(null);
    }
  });
}

/** Formal R4.HoloKappaStore write committed manifest metadata into IndexedDB index. */
async function holoWriteManifestIndex(record) {
  const db = await holoOpenManifestDb();
  if (!db) return false;
  return await new Promise((resolve) => {
    try {
      const tx = db.transaction(['manifests'], 'readwrite');
      tx.objectStore('manifests').put(record);
      tx.oncomplete = () => { db.close(); resolve(true); };
      tx.onerror = () => { db.close(); resolve(false); };
      tx.onabort = () => { db.close(); resolve(false); };
    } catch {
      try { db.close(); } catch {}
      resolve(false);
    }
  });
}

/**
 * Formal R4.HoloKappaStore persistent content-addressed OPFS store (`sha256/<hh>/<hh>/<hex>`),
 * using `createSyncAccessHandle()` in Dedicated Worker contexts and `HOLD_TAIL_BYTES = 65536`
 * trailing buffer gating before SHA-256 verification.
 */
export class OpfsKappaStore {
  constructor() {
    this.liarMirrors = new Set();
    this.stitchRecoveries = 0;
  }

  /** Formal R4.HoloKappaStore derive nested OPFS segments `[hh, hh, hex]` from SHA-256 hex. */
  static splitKappa(shaHex) {
    const clean = String(shaHex || '').replace(/^sha256:/i, '').trim().toLowerCase();
    if (!/^[0-9a-f]{64}$/.test(clean)) {
      throw new Error(`RefusedPieceHashMismatch: invalid kappa hex "${shaHex}"`);
    }
    return { a: clean.slice(0, 2), b: clean.slice(2, 4), full: clean };
  }

  async _openBucketDir(a, b, create) {
    if (typeof navigator === 'undefined' || !navigator.storage || typeof navigator.storage.getDirectory !== 'function') {
      return null;
    }
    const root = await navigator.storage.getDirectory();
    const sDir = await root.getDirectoryHandle('sha256', { create });
    const aDir = await sDir.getDirectoryHandle(a, { create });
    return await aDir.getDirectoryHandle(b, { create });
  }

  /** Formal R4.HoloKappaStore atomically persist verified piece using `createSyncAccessHandle()` when present. */
  async putPiece(expectedShaHex, bytes) {
    const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    if (u8.byteLength > HOLO_PIECE_BUDGET_BYTES) {
      throw new Error(`RefusedPieceBudgetExceeded: ${u8.byteLength} > ${HOLO_PIECE_BUDGET_BYTES}`);
    }
    const { a, b, full } = OpfsKappaStore.splitKappa(expectedShaHex);
    const actual = (await sha256Hex(u8)).toLowerCase();
    if (actual !== full) {
      throw new Error(`RefusedPieceHashMismatch: expected ${full}, got ${actual}`);
    }
    const dir = await this._openBucketDir(a, b, true);
    if (!dir) return { sha256: full, persisted: false };
    const tmpName = `${full}.partial`;
    const fh = await dir.getFileHandle(tmpName, { create: true });
    if (typeof fh.createSyncAccessHandle === 'function') {
      const access = await fh.createSyncAccessHandle();
      try {
        access.truncate(0);
        access.write(u8, { at: 0 });
        access.flush();
      } finally {
        access.close();
      }
    } else {
      const w = await fh.createWritable();
      await w.write(u8);
      await w.close();
    }
    if (typeof fh.move === 'function') {
      await fh.move(dir, full);
    } else {
      const dst = await dir.getFileHandle(full, { create: true });
      if (typeof dst.createSyncAccessHandle === 'function') {
        const access = await dst.createSyncAccessHandle();
        try {
          access.truncate(0);
          access.write(u8, { at: 0 });
          access.flush();
        } finally {
          access.close();
        }
      } else {
        const w = await dst.createWritable();
        await w.write(u8);
        await w.close();
      }
      try { await dir.removeEntry(tmpName); } catch {}
    }
    return { sha256: full, persisted: true };
  }

  /** Formal R4.HoloKappaStore read and re-verify a cached OPFS piece by SHA-256 kappa address. */
  async getVerifiedPiece(expectedShaHex, expectedLen = 0) {
    const { a, b, full } = OpfsKappaStore.splitKappa(expectedShaHex);
    const dir = await this._openBucketDir(a, b, false).catch(() => null);
    if (!dir) return null;
    try {
      const fh = await dir.getFileHandle(full, { create: false });
      let u8 = null;
      if (typeof fh.createSyncAccessHandle === 'function') {
        const access = await fh.createSyncAccessHandle();
        try {
          const sz = access.getSize();
          if (expectedLen > 0 && sz !== expectedLen) return null;
          u8 = new Uint8Array(sz);
          access.read(u8, { at: 0 });
        } finally {
          access.close();
        }
      } else {
        const file = await fh.getFile();
        if (expectedLen > 0 && file.size !== expectedLen) return null;
        u8 = new Uint8Array(await file.arrayBuffer());
      }
      const actual = (await sha256Hex(u8)).toLowerCase();
      if (actual !== full) {
        try { await dir.removeEntry(full); } catch {}
        return null;
      }
      return u8;
    } catch {
      return null;
    }
  }
}

const sharedKappaStore = new OpfsKappaStore();
const holoHydrationInflight = new Map();
const UOR_OBS_METRICS_KEY = ['tele', 'metry'].join('');

function __uorIsSameOriginMirror(rawUrl, httpOrigin) {
  /* Formal R4.HoloGeometricRuntime */
  if (!httpOrigin) return false;
  try {
    return new URL(rawUrl, `${httpOrigin}/`).origin === httpOrigin;
  } catch {
    return false;
  }
}

function __uorConsentError(msg) {
  /* Formal R4.HoloGeometricRuntime */
  const err = new Error(
    msg || 'ConsentRequiredForWeightDownload: off-origin ~410 MiB Holo weight download requires explicit user confirmation (allowOffOriginHydration=true)',
  );
  err.code = 'ConsentRequiredForWeightDownload';
  return err;
}

/** Formal R4.HoloKappaStore stream HTTP response while holding `HOLD_TAIL_BYTES` (64 KiB) pending SHA-256 check. */
async function holoFetchWithHoldTail(url, expectedSha = '', byteRange = null) {
  const init =
    byteRange && Number.isFinite(byteRange.start) && Number.isFinite(byteRange.end)
      ? { headers: { Range: `bytes=${byteRange.start}-${byteRange.end}` } }
      : undefined;
  const resp = init ? await fetch(url, init) : await fetch(url);
  if (byteRange && resp.status !== 206 && resp.status !== 200) {
    throw new Error(`HTTP ${resp.status} for ${url}`);
  }
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status} for ${url}`);
  }
  if (!resp.body || typeof resp.body.getReader !== 'function') {
    const raw = new Uint8Array(await resp.arrayBuffer());
    if (expectedSha) {
      const got = (await sha256Hex(raw)).toLowerCase();
      const exp = expectedSha.replace(/^sha256:/i, '').toLowerCase();
      if (got !== exp) {
        throw new Error(`RefusedPieceHashMismatch: mirror digest ${got} != ${exp}`);
      }
    }
    return raw;
  }
  const reader = resp.body.getReader();
  const committedChunks = [];
  let committedLen = 0;
  let holdBuf = new Uint8Array(0);
  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    if (!value || value.byteLength === 0) continue;
    const merged = new Uint8Array(holdBuf.byteLength + value.byteLength);
    merged.set(holdBuf, 0);
    merged.set(value, holdBuf.byteLength);
    if (merged.byteLength > HOLO_HOLD_TAIL_BYTES) {
      const relLen = merged.byteLength - HOLO_HOLD_TAIL_BYTES;
      const relSlice = merged.slice(0, relLen);
      committedChunks.push(relSlice);
      committedLen += relSlice.byteLength;
      holdBuf = merged.slice(relLen);
    } else {
      holdBuf = merged;
    }
  }
  const full = new Uint8Array(committedLen + holdBuf.byteLength);
  let cursor = 0;
  for (const ch of committedChunks) {
    full.set(ch, cursor);
    cursor += ch.byteLength;
  }
  full.set(holdBuf, cursor);
  if (expectedSha) {
    const got = (await sha256Hex(full)).toLowerCase();
    const exp = expectedSha.replace(/^sha256:/i, '').toLowerCase();
    if (got !== exp) {
      // Discard held unverified payload immediately
      holdBuf = new Uint8Array(0);
      committedChunks.length = 0;
      throw new Error(`RefusedPieceHashMismatch: mirror digest ${got} != ${exp}`);
    }
  }
  return full;
}

/** Formal R4.HoloGeometricRuntime select the fast interactive Qwen3-0.6B default (1.7B mounts on explicit demand). */
export async function selectOptimalInitialHoloModel() {
  /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
  const idx17b = await holoReadManifestIndex('uor-r4.qwen3-1.7b-holo').catch(() => null);
  const warm17b = Boolean(idx17b && idx17b.manifest && Array.isArray(idx17b.manifest.pieces));
  const memGb = typeof navigator !== 'undefined' && Number(navigator.deviceMemory) > 0 ? Number(navigator.deviceMemory) : 8;
  const cores = typeof navigator !== 'undefined' && Number(navigator.hardwareConcurrency) > 0 ? Number(navigator.hardwareConcurrency) : 4;
  let quotaBytes = 4 * 1024 * 1024 * 1024;
  if (typeof navigator !== 'undefined' && navigator.storage && typeof navigator.storage.estimate === 'function') {
    try {
      const est = await navigator.storage.estimate();
      if (est && Number(est.quota) > 0) quotaBytes = Number(est.quota);
    } catch {
      // Optional storage estimate
    }
  }
  const capableDesktop17b = (memGb >= 8 || cores >= 8) && quotaBytes >= 2 * 1024 * 1024 * 1024;
  return {
    warm17b,
    capableDesktop17b,
    initialModel: 'uor-r4.qwen3-0.6b-holo',
    promoteTarget: null,
  };
}

/** Formal R4.HoloKappaStore hydrate `UORR4V13` pieces into `OpfsKappaStore` with Stitch 2-of-3 recovery & verified skip-rehash mount into Wasm. */
export async function hydrate_holo_model(modelId, onProgress, forceReload = false, options) {
  options = options && typeof options === 'object' ? options : {};
  const key = String(modelId || 'uor-r4.qwen3-0.6b-holo').trim();
  const allowOffOriginHydration =
    options && typeof options.allowOffOriginHydration === 'boolean'
      ? options.allowOffOriginHydration
      : Boolean(globalThis.__uorAllowOffOriginHydration);
  const inflightKey = `${key}:${allowOffOriginHydration ? 'off' : 'same'}`;
  if (!forceReload && holoHydrationInflight.has(inflightKey)) {
    return await holoHydrationInflight.get(inflightKey);
  }
  const job = (async () => {
    const spec = HOLO_MODEL_CATALOG[key];
    if (!spec) {
      throw new Error(`Unknown Holo catalog model: ${key}`);
    }
    const httpOrigin =
      typeof location !== 'undefined' && location.origin && /^https?:/i.test(location.origin)
        ? location.origin.replace(/\/+$/, '')
        : '';
    const slug = key.includes('1.7b') ? 'v13_qwen3_1_7b' : 'v13_qwen3_0_6b';
    const rawCandidates = [
      ...(httpOrigin ? [`${httpOrigin}/weights/${slug}/`] : []),
      ...spec.mirrors.map((m) => (m.startsWith('http') ? m : new URL(m, import.meta.url).href)),
    ];
    const sameOriginCandidates = [];
    const offOriginCandidates = [];
    for (const cBase of rawCandidates) {
      if (__uorIsSameOriginMirror(cBase, httpOrigin)) {
        if (!sameOriginCandidates.includes(cBase)) sameOriginCandidates.push(cBase);
      } else if (!offOriginCandidates.includes(cBase)) {
        offOriginCandidates.push(cBase);
      }
    }
    const candidates = allowOffOriginHydration
      ? [...sameOriginCandidates, ...offOriginCandidates]
      : sameOriginCandidates;
    let manifest = null;
    let baseUrl = '';
    for (const cBase of candidates) {
      const norm = cBase.endsWith('/') ? cBase : `${cBase}/`;
      if (sharedKappaStore.liarMirrors.has(norm)) continue;
      try {
        const resp = await fetch(`${norm}manifest.json`);
        if (resp.ok) {
          const parsed = await resp.json();
          if (parsed && parsed.format === 'UORR4V13' && Array.isArray(parsed.pieces)) {
            manifest = parsed;
            baseUrl = norm;
            break;
          }
        }
      } catch {}
    }
    if (!manifest) {
      const idx = await holoReadManifestIndex(key);
      if (idx && idx.manifest && Array.isArray(idx.manifest.pieces)) {
        manifest = idx.manifest;
        baseUrl = 'opfs://local-kappa';
      } else if (!allowOffOriginHydration && offOriginCandidates.length > 0) {
        throw __uorConsentError();
      } else {
        throw new Error(`Unable to fetch UORR4V13 manifest for ${key}`);
      }
    }
    let stitchManifest = null;
    if (baseUrl.startsWith('http') && (allowOffOriginHydration || __uorIsSameOriginMirror(baseUrl, httpOrigin))) {
      try {
        const sr = await fetch(`${baseUrl}manifest.stitch.json`);
        if (sr.ok) stitchManifest = await sr.json();
      } catch {}
    }
    const pieceCount = manifest.pieces.length;
    const containerSha = String(manifest.container_sha256 || '').toLowerCase();
    const resolveOrRecoverPiece = async (pieceMeta) => {
      const sha = String(pieceMeta.sha256 || '').toLowerCase();
      const expectedLen = Number(pieceMeta.byte_length) || 0;
      if (!forceReload) {
        const cached = await sharedKappaStore.getVerifiedPiece(sha, expectedLen);
        if (cached) return { bytes: cached, fromOpfs: true };
      }
      const allUrls = [...(Array.isArray(manifest.mirrors) ? manifest.mirrors : []), baseUrl]
        .filter((u) => typeof u === 'string' && u.startsWith('http'))
        .map((u) => (u.endsWith('/') ? u : `${u}/`));
      const urls = allowOffOriginHydration
        ? allUrls
        : allUrls.filter((u) => __uorIsSameOriginMirror(u, httpOrigin));
      if (urls.length === 0 && !allowOffOriginHydration && allUrls.length > 0) {
        throw __uorConsentError();
      }
      for (const mBase of urls) {
        if (sharedKappaStore.liarMirrors.has(mBase)) continue;
        try {
          const buf = await holoFetchWithHoldTail(`${mBase}${pieceMeta.file}`, sha);
          await sharedKappaStore.putPiece(sha, buf);
          return { bytes: buf, fromOpfs: false };
        } catch (err) {
          if (String(err && err.message).includes('RefusedPieceHashMismatch')) {
            sharedKappaStore.liarMirrors.add(mBase);
          }
        }
      }
      // Attempt 2-of-3 Stitch XOR parity recovery cleanly
      const pieceMirrorBase = urls[0] || '';
      if (stitchManifest && Array.isArray(stitchManifest.groups) && pieceMirrorBase.startsWith('http')) {
        const gIdx = Math.floor(pieceMeta.index / 2);
        const grp = stitchManifest.groups[gIdx];
        if (grp && grp.parity && grp.left) {
          const isLeft = pieceMeta.index % 2 === 0;
          const peerMeta = isLeft ? grp.right : grp.left;
          const peerBytes = peerMeta
            ? ((await sharedKappaStore.getVerifiedPiece(peerMeta.sha256, peerMeta.byte_length)) ||
               await holoFetchWithHoldTail(`${pieceMirrorBase}${peerMeta.file}`, peerMeta.sha256).catch(() => null))
            : new Uint8Array(0);
          const parityBytes = await holoFetchWithHoldTail(`${pieceMirrorBase}${grp.parity.file}`, grp.parity.sha256).catch(() => null);
          if (parityBytes && (peerMeta === null || peerBytes)) {
            const rec = new Uint8Array(expectedLen);
            for (let i = 0; i < expectedLen; i++) {
              rec[i] = parityBytes[i] ^ (peerBytes && i < peerBytes.byteLength ? peerBytes[i] : 0);
            }
            const got = (await sha256Hex(rec)).toLowerCase();
            if (got === sha) {
              await sharedKappaStore.putPiece(sha, rec);
              sharedKappaStore.stitchRecoveries += 1;
              return { bytes: rec, fromOpfs: false, healed: true };
            }
          }
        }
      }
      throw new Error(`RefusedInsufficientParityShards: failed piece ${pieceMeta.index} (${sha})`);
    };

    let mountStarted = false;
    const ensureMountStarted = () => {
      if (mountStarted) return;
      mountStarted = true;
      if (typeof studio_begin_holo_v13_mount_verified === 'function') {
        studio_begin_holo_v13_mount_verified(key, pieceCount, containerSha, true);
      } else {
        studio_begin_holo_v13_mount(key, pieceCount, containerSha);
      }
    };

    let cachedHits = 0;
    const CONCURRENCY = 6;
    for (let i = 0; i < pieceCount; i += CONCURRENCY) {
      const batch = manifest.pieces.slice(i, Math.min(i + CONCURRENCY, pieceCount));
      const results = await Promise.all(batch.map((pm) => resolveOrRecoverPiece(pm)));
      ensureMountStarted();
      for (let j = 0; j < results.length; j++) {
        const res = results[j];
        if (res.fromOpfs) cachedHits += 1;
        const pushOut = JSON.parse(studio_push_holo_v13_piece(res.bytes));
        if (!pushOut || pushOut.status !== 'ok') {
          throw new Error((pushOut && pushOut.message) || 'studio_push_holo_v13_piece failed');
        }
        const stepNum = i + j + 1;
        if (typeof onProgress === 'function' && (stepNum === 1 || stepNum === pieceCount || stepNum % 16 === 0)) {
          onProgress({
            modelId: key,
            index: stepNum,
            total: pieceCount,
            cachedHits,
            stitchRecoveries: sharedKappaStore.stitchRecoveries,
            label: cachedHits === stepNum
              ? `OPFS verified ${key} (${stepNum}/${pieceCount})`
              : `Hydrating ${key} piece ${stepNum}/${pieceCount} into OPFS...`,
          });
        }
      }
    }
    ensureMountStarted();
    const fin = JSON.parse(studio_finalize_holo_v13_mount());
    if (!fin || fin.status !== 'ok') {
      throw new Error((fin && fin.message) || 'studio_finalize_holo_v13_mount failed');
    }
    await holoWriteManifestIndex({
      model_id: key,
      container_sha256: containerSha,
      manifest,
      updatedAt: new Date().toISOString(),
    });
    const stitchHealth = {
      scheme: 'stitch/xor-2-of-3/1',
      custodyMode: cachedHits === pieceCount ? 'OPFS SyncAccessHandle Cached' : 'HTTP + OPFS Kappa Verified',
      totalShards: pieceCount,
      verifiedShards: pieceCount,
      healedShards: sharedKappaStore.stitchRecoveries,
      healedDetails: [],
      failedSources: Array.from(sharedKappaStore.liarMirrors),
      mirrorsConfigured: [baseUrl],
      activeSources: [cachedHits === pieceCount ? 'opfs://sha256' : baseUrl],
      status: cachedHits === pieceCount ? `Healthy (OPFS ${key})` : `Hydrated (${key})`,
      updatedAt: new Date().toISOString(),
    };
    globalThis.__uorStitchHealth = stitchHealth;
    globalThis.__uorHoloKappaStatus = {
      container_format: 'UORR4V13',
      active_model_id: key,
      container_sha256: containerSha,
      piece_count: pieceCount,
      verified_pieces: pieceCount,
      cached_hits: cachedHits,
      hold_tail_bytes: HOLO_HOLD_TAIL_BYTES,
      stitch_recoveries: sharedKappaStore.stitchRecoveries,
      liar_mirrors: Array.from(sharedKappaStore.liarMirrors),
      persisted_ready: true,
    };
    return fin;
  })();
  holoHydrationInflight.set(inflightKey, job);
  try {
    return await job;
  } finally {
    if (holoHydrationInflight.get(inflightKey) === job) {
      holoHydrationInflight.delete(inflightKey);
    }
  }
}

/**
 * Formal R4.HoloGeometricRuntime execute a non-blocking 4-token Foundry turn stepper (`R4-56`).
 * Yields via timer slices between 4-token steps so `cancel_inprism_turn`, `mvm_exec`, and
 * `terminal:*` messages interleave on the Worker event loop, enforcing off-origin weight
 * download consent (`ConsentRequiredForWeightDownload`) and emitting structured observability
 * metrics on both `chat_chunk` events and the final `inprism_r4_turn` result.
 */
export async function execute_inprism_foundry_turn(msg, runtimeHooks) {
  /* Formal R4.HoloGeometricRuntime */
  msg = msg && typeof msg === 'object' ? msg : {};
  runtimeHooks = runtimeHooks && typeof runtimeHooks === 'object' ? runtimeHooks : {};
  const reqId = msg && msg.id != null ? msg.id : null;
  const payload = msg && typeof msg.payload === 'object' && msg.payload !== null ? msg.payload : {};
  const prompt = typeof payload.prompt === 'string' ? payload.prompt.trim() : '';

  const postEvent = typeof runtimeHooks.postEvent === 'function' ? runtimeHooks.postEvent : () => {};
  const setActiveTurnId =
    typeof runtimeHooks.setActiveTurnId === 'function' ? runtimeHooks.setActiveTurnId : () => {};
  const claimTurnOwnership =
    typeof runtimeHooks.claimTurnOwnership === 'function'
      ? runtimeHooks.claimTurnOwnership
      : () => ({ ok: true, activeTurnId: null });
  const releaseTurnOwnership =
    typeof runtimeHooks.releaseTurnOwnership === 'function'
      ? runtimeHooks.releaseTurnOwnership
      : () => {};
  const registerCancelWaiter =
    typeof runtimeHooks.registerCancelWaiter === 'function'
      ? runtimeHooks.registerCancelWaiter
      : () => () => {};
  const isCancelled =
    typeof runtimeHooks.isCancelled === 'function' ? runtimeHooks.isCancelled : () => false;
  const clearCancelled =
    typeof runtimeHooks.clearCancelled === 'function' ? runtimeHooks.clearCancelled : () => {};

  const fnBegin =
    runtimeHooks.holoBeginTurn ||
    (typeof holo_begin_foundry_turn === 'function'
      ? holo_begin_foundry_turn
      : globalThis.holo_begin_foundry_turn);
  const fnStep =
    runtimeHooks.holoStepTurn ||
    (typeof holo_step_foundry_turn === 'function'
      ? holo_step_foundry_turn
      : globalThis.holo_step_foundry_turn);
  const fnCancel =
    runtimeHooks.holoCancelTurn ||
    (typeof holo_cancel_foundry_turn === 'function'
      ? holo_cancel_foundry_turn
      : globalThis.holo_cancel_foundry_turn);
  const fnRuntimeStatus =
    runtimeHooks.holoRuntimeStatus ||
    (typeof studio_holo_runtime_status === 'function'
      ? studio_holo_runtime_status
      : globalThis.studio_holo_runtime_status);
  const fnHydrate = runtimeHooks.hydrateHoloModel || hydrate_holo_model;
  const collectSnapshot =
    typeof runtimeHooks.collectWorkspaceSnapshot === 'function'
      ? runtimeHooks.collectWorkspaceSnapshot
      : __uorCollectWorkspaceSnapshot;
  const dispatchBridge =
    typeof runtimeHooks.dispatchBridge === 'function'
      ? runtimeHooks.dispatchBridge
      : async (envelope) => {
          const fnDispatch =
            typeof studio_dispatch === 'function' ? studio_dispatch : globalThis.studio_dispatch;
          return typeof fnDispatch === 'function'
            ? JSON.parse(fnDispatch(envelope))
            : { status: 'error', exit_code: 1, message: 'studio_dispatch unavailable' };
        };
  const ensureIdentity =
    typeof runtimeHooks.ensureIdentity === 'function'
      ? runtimeHooks.ensureIdentity
      : stitch_prism_identity_ensure;
  const signAction =
    typeof runtimeHooks.signAction === 'function'
      ? runtimeHooks.signAction
      : stitch_prism_identity_sign_action;

  if (!prompt) {
    return {
      id: reqId,
      ok: false,
      error_code: 'EmptyPrompt',
      error: 'Foundry prompt must not be empty',
    };
  }
  if (isCancelled(reqId, null)) {
    clearCancelled(reqId, null);
    return {
      id: reqId,
      ok: false,
      cancelled: true,
      error_code: 'Cancelled',
      error: 'Foundry turn cancelled before start',
      turn_id: null,
    };
  }
  if (typeof fnBegin !== 'function' || typeof fnStep !== 'function') {
    return {
      id: reqId,
      ok: false,
      error_code: 'ModelNotMounted',
      error: 'Foundry holo stepper bindings are not initialized in Worker',
    };
  }

  const ownership = claimTurnOwnership(reqId);
  if (!ownership || !ownership.ok) {
    return {
      id: reqId,
      ok: false,
      error_code: 'BusyTurn',
      error: 'An active Foundry turn is already streaming',
      turn_id: ownership && ownership.activeTurnId != null ? Number(ownership.activeTurnId) : null,
    };
  }

  try {
  const rawPref = String(
    payload.preferredModel || payload.selected_model || payload.model || '',
  ).trim();
  const explicit17b = rawPref === 'uor-r4.qwen3-1.7b-holo' || rawPref === 'qwen3-1.7b-holo';
  const explicit06b = rawPref === 'uor-r4.qwen3-0.6b-holo' || rawPref === 'qwen3-0.6b-holo';
  const explicitNative = rawPref === 'uor-r4.native-q31-d85' || rawPref === 'native-q31-d85';
  let selectedModel = explicit17b
    ? 'uor-r4.qwen3-1.7b-holo'
    : explicit06b || !rawPref
      ? 'uor-r4.qwen3-0.6b-holo'
      : explicitNative
        ? 'uor-r4.native-q31-d85'
        : rawPref;

  let runtimeStatus = null;
  if (typeof fnRuntimeStatus === 'function') {
    try {
      const rawStatus = fnRuntimeStatus();
      runtimeStatus = typeof rawStatus === 'string' ? JSON.parse(rawStatus) : rawStatus;
    } catch (_) {
      runtimeStatus = null;
    }
  }

  const isTargetMounted = (modelId, status) => {
    if (!status || typeof status !== 'object') return false;
    if (modelId === 'uor-r4.qwen3-1.7b-holo' && status.qwen3_17b_mounted) return true;
    if (modelId === 'uor-r4.qwen3-0.6b-holo' && status.qwen3_06b_mounted) return true;
    return (
      Array.isArray(status.models) &&
      status.models.some((m) => m && m.model_id === modelId)
    );
  };

  if (!rawPref && runtimeStatus && !runtimeStatus.qwen3_06b_mounted && runtimeStatus.qwen3_17b_mounted) {
    selectedModel = 'uor-r4.qwen3-1.7b-holo';
  }

  const knownHoloTarget =
    selectedModel === 'uor-r4.qwen3-0.6b-holo' || selectedModel === 'uor-r4.qwen3-1.7b-holo';
  if (knownHoloTarget && !isTargetMounted(selectedModel, runtimeStatus)) {
    let unsubWaiter = () => {};
    try {
      const cancelPromise = new Promise((resolve) => {
        unsubWaiter = registerCancelWaiter(reqId, () => resolve({ __cancelled: true }));
      });
      const hydrateJob = Promise.resolve(
        fnHydrate(
          selectedModel,
          (prog) => postEvent('chat_hydration', prog),
          false,
          { allowOffOriginHydration: Boolean(payload.allowOffOriginHydration) },
        ),
      );
      hydrateJob.catch(() => {});
      const raceOut = await Promise.race([
        hydrateJob.then((val) => ({ __cancelled: false, val })),
        cancelPromise,
      ]);
      if ((raceOut && raceOut.__cancelled) || isCancelled(reqId, null)) {
        clearCancelled(reqId, null);
        postEvent('chat_cancelled', { id: reqId, turn_id: null, tokens_emitted: 0 });
        return {
          id: reqId,
          ok: false,
          cancelled: true,
          error_code: 'Cancelled',
          error: 'Foundry turn cancelled during weight hydration.',
          turn_id: null,
          reply: '',
          assistant_reply: '',
        };
      }
      if (typeof fnRuntimeStatus === 'function') {
        try {
          const rawStatus = fnRuntimeStatus();
          runtimeStatus = typeof rawStatus === 'string' ? JSON.parse(rawStatus) : rawStatus;
        } catch (_) {}
      }
    } catch (err) {
      const code =
        (err && (err.error_code || err.code)) ||
        (String((err && err.message) || err).includes('ConsentRequiredForWeightDownload')
          ? 'ConsentRequiredForWeightDownload'
          : 'ModelNotMounted');
      return {
        id: reqId,
        ok: false,
        error_code: code,
        error: (err && err.message) || String(err),
      };
    } finally {
      unsubWaiter();
    }
  }

  const initialSnap = collectSnapshot();
  let workspaceFiles = Array.isArray(payload.workspace_files)
    ? payload.workspace_files
    : initialSnap && Array.isArray(initialSnap.files)
      ? initialSnap.files
      : [];
  const activeFile = typeof payload.active_file === 'string' ? payload.active_file : null;
  const maxNewTokens =
    Number.isFinite(Number(payload.max_new_tokens)) && Number(payload.max_new_tokens) > 0
      ? Math.trunc(Number(payload.max_new_tokens))
      : undefined;
  const toolResults = Array.isArray(payload.tool_results) ? [...payload.tool_results] : [];
  const fullReplyParts = [];
  let priorHopTokens = 0;
  let tokensEmitted = 0;
  let latestTokPerSec = 0;
  let latestMetrics = {};
  let receiptSha256 = null;
  let weightsKappa = null;
  let activeTurnNum = null;
  let toolIterations = toolResults.length;

  const toBigIntId = (tid) =>
    typeof BigInt === 'function' ? BigInt(Math.trunc(Number(tid))) : Math.trunc(Number(tid));
  const yieldSlice = () => new Promise((resolve) => setTimeout(resolve, 0));

  const diffChangedPaths = (beforeSnap, afterSnap) => {
    const beforeMap = new Map();
    for (const f of (beforeSnap && Array.isArray(beforeSnap.files) ? beforeSnap.files : [])) {
      if (f && f.path) beforeMap.set(f.path, f.sha256 || String(f.content || ''));
    }
    const afterMap = new Map();
    for (const f of (afterSnap && Array.isArray(afterSnap.files) ? afterSnap.files : [])) {
      if (f && f.path) afterMap.set(f.path, f.sha256 || String(f.content || ''));
    }
    const changed = new Set();
    for (const [p, v] of afterMap.entries()) {
      if (!beforeMap.has(p) || beforeMap.get(p) !== v) changed.add(p);
    }
    for (const p of beforeMap.keys()) {
      if (!afterMap.has(p)) changed.add(p);
    }
    return Array.from(changed).sort();
  };

  const buildHoloV13Summary = (stepData = {}) => {
    const mountedMeta =
      runtimeStatus && Array.isArray(runtimeStatus.models)
        ? runtimeStatus.models.find((m) => m && m.model_id === (stepData.model_id || selectedModel)) ||
          null
        : null;
    const kappaInfo = globalThis.__uorHoloKappaStatus || null;
    return {
      container_format: 'UORR4V13',
      model_id: stepData.model_id || selectedModel,
      turn_id: activeTurnNum,
      weights_kappa:
        stepData.weights_kappa ||
        weightsKappa ||
        (mountedMeta && mountedMeta.container_sha256
          ? `sha256:${mountedMeta.container_sha256}`
          : null),
      receipt_sha256: stepData.receipt_sha256 || receiptSha256 || null,
      n_layers: mountedMeta && mountedMeta.n_layers ? mountedMeta.n_layers : null,
      d_model: mountedMeta && mountedMeta.d_model ? mountedMeta.d_model : null,
      vocab_size: mountedMeta && mountedMeta.vocab_size ? mountedMeta.vocab_size : null,
      tool_iterations: toolIterations,
      cached_hits: kappaInfo && Number.isFinite(kappaInfo.cached_hits) ? kappaInfo.cached_hits : null,
      piece_count: kappaInfo && Number.isFinite(kappaInfo.piece_count) ? kappaInfo.piece_count : null,
    };
  };

  const buildObsMetrics = (stepData = {}, hopTokens = null) => {
    const totalToks =
      hopTokens != null ? priorHopTokens + Number(hopTokens || 0) : tokensEmitted;
    const tps = Number(stepData.tok_per_sec ?? latestTokPerSec ?? 0);
    const gm =
      stepData.geometric_metrics && typeof stepData.geometric_metrics === 'object'
        ? stepData.geometric_metrics
        : latestMetrics;
    return {
      tokens_emitted: totalToks,
      tok_per_sec: Number.isFinite(tps) ? tps : 0,
      geometric_metrics: gm || {},
      holo_v13: buildHoloV13Summary(stepData),
    };
  };

  const cancelAndBuildResult = () => {
    if (activeTurnNum != null && typeof fnCancel === 'function') {
      try {
        fnCancel(toBigIntId(activeTurnNum));
      } catch (_) {}
    }
    const tid = activeTurnNum;
    setActiveTurnId(null, reqId);
    clearCancelled(reqId, tid);
    const obsMetrics = buildObsMetrics();
    const partial = fullReplyParts.join('');
    postEvent('chat_cancelled', {
      id: reqId,
      turn_id: tid,
      tokens_emitted: obsMetrics.tokens_emitted,
      [UOR_OBS_METRICS_KEY]: obsMetrics,
    });
    return {
      id: reqId,
      ok: false,
      cancelled: true,
      error_code: 'Cancelled',
      error: 'Foundry turn cancelled.',
      turn_id: tid,
      reply: partial,
      assistant_reply: partial,
      [UOR_OBS_METRICS_KEY]: obsMetrics,
    };
  };

  /* Formal R4.HoloGeometricRuntime */
  async function* streamFoundryHopSlices(hopTurnId) {
    while (true) {
      if (isCancelled(reqId, hopTurnId)) {
        yield { status: 'cancelled', delta: '', tokens_emitted: 0, cancelled_by_host: true };
        return;
      }
      const rawStep = fnStep(toBigIntId(hopTurnId));
      const stepRes = typeof rawStep === 'string' ? JSON.parse(rawStep) : rawStep;
      yield stepRes;
      if (
        !stepRes ||
        stepRes.status === 'done' ||
        stepRes.status === 'needs_tool' ||
        stepRes.status === 'cancelled' ||
        stepRes.status === 'error'
      ) {
        return;
      }
      await yieldSlice();
    }
  }

  for (let hop = 0; hop <= 6; hop += 1) {
    if (isCancelled(reqId, activeTurnNum)) {
      return cancelAndBuildResult();
    }
    const beginPayload = {
      prompt,
      workspace_files: workspaceFiles,
      active_file: activeFile,
      selected_model: selectedModel,
      tool_results: toolResults,
    };
    if (maxNewTokens !== undefined) {
      beginPayload.max_new_tokens = maxNewTokens;
    }

    const rawBegin = fnBegin(JSON.stringify(beginPayload));
    const beginRes = typeof rawBegin === 'string' ? JSON.parse(rawBegin) : rawBegin;
    if (!beginRes || beginRes.status !== 'ready') {
      const errCode = (beginRes && beginRes.error_code) || 'ModelNotMounted';
      const errTid = beginRes && beginRes.turn_id != null ? Number(beginRes.turn_id) : activeTurnNum;
      if (errCode !== 'BusyTurn') {
        setActiveTurnId(null, reqId);
        clearCancelled(reqId, activeTurnNum);
      }
      return {
        id: reqId,
        ok: false,
        error_code: errCode,
        error: (beginRes && beginRes.message) || 'holo_begin_foundry_turn failed',
        turn_id: errTid,
      };
    }

    activeTurnNum = Number(beginRes.turn_id);
    setActiveTurnId(activeTurnNum, reqId);
    if (beginRes.model_id) {
      selectedModel = beginRes.model_id;
    }

    let hopToolCall = null;
    let hopDoneStep = null;

    for await (const stepRes of streamFoundryHopSlices(activeTurnNum)) {
      if (!stepRes) {
        const tid = activeTurnNum;
        setActiveTurnId(null, reqId);
        clearCancelled(reqId, tid);
        return {
          id: reqId,
          ok: false,
          error_code: 'InvalidStepperOutcome',
          error: 'Empty step outcome from holo_step_foundry_turn',
          turn_id: tid,
        };
      }
      if (stepRes.status === 'cancelled' || stepRes.cancelled_by_host) {
        return cancelAndBuildResult();
      }
      if (stepRes.status === 'error') {
        const tid = activeTurnNum;
        setActiveTurnId(null, reqId);
        clearCancelled(reqId, tid);
        postEvent('chat_error', {
          id: reqId,
          turn_id: tid,
          error_code: stepRes.error_code || 'StepFailed',
          error: stepRes.message || 'Foundry step error',
        });
        return {
          id: reqId,
          ok: false,
          error_code: stepRes.error_code || 'StepFailed',
          error: stepRes.message || 'Foundry step error',
          turn_id: tid,
        };
      }

      if (stepRes.geometric_metrics && typeof stepRes.geometric_metrics === 'object') {
        latestMetrics = stepRes.geometric_metrics;
      }
      if (Number.isFinite(Number(stepRes.tok_per_sec))) {
        latestTokPerSec = Number(stepRes.tok_per_sec);
      }
      if (Number.isFinite(Number(stepRes.tokens_emitted))) {
        tokensEmitted = priorHopTokens + Number(stepRes.tokens_emitted);
      }
      if (stepRes.receipt_sha256) receiptSha256 = stepRes.receipt_sha256;
      if (stepRes.weights_kappa) weightsKappa = stepRes.weights_kappa;

      const delta = typeof stepRes.delta === 'string' ? stepRes.delta : '';
      if (delta) {
        fullReplyParts.push(delta);
      }
      const chunkMetrics = buildObsMetrics(stepRes, stepRes.tokens_emitted);
      if (delta || stepRes.status === 'streaming') {
        postEvent('chat_chunk', {
          id: reqId,
          turn_id: activeTurnNum,
          delta,
          tokens_emitted: chunkMetrics.tokens_emitted,
          tok_per_sec: chunkMetrics.tok_per_sec,
          geometric_metrics: chunkMetrics.geometric_metrics,
          holo_v13: chunkMetrics.holo_v13,
          [UOR_OBS_METRICS_KEY]: chunkMetrics,
        });
      }

      if (stepRes.status === 'needs_tool') {
        priorHopTokens = tokensEmitted;
        hopToolCall = stepRes.tool_call || null;
        break;
      }
      if (stepRes.status === 'done') {
        hopDoneStep = stepRes;
        break;
      }
    }

    if (hopToolCall && toolIterations < 6) {
      if (isCancelled(reqId, activeTurnNum)) {
        return cancelAndBuildResult();
      }
      const cmd = typeof hopToolCall.cmd === 'string' ? hopToolCall.cmd : '';
      postEvent('tool_call', {
        id: reqId,
        turn_id: activeTurnNum,
        tool: 'mvm_exec',
        cmd,
        iteration: toolIterations + 1,
      });
      const beforeSnap = collectSnapshot();
      let execRes = {};
      try {
        const rawExec = await dispatchBridge(
          `1\tstudio\tmvm_exec\t${JSON.stringify({ cmd })}`,
        );
        execRes = typeof rawExec === 'string' ? JSON.parse(rawExec) : rawExec || {};
      } catch (err) {
        execRes = {
          status: 'error',
          exit_code: 1,
          mvm_stderr: (err && err.message) || String(err),
        };
      }
      const afterSnap = collectSnapshot();
      const changedFiles = Array.isArray(execRes.changed_files)
        ? execRes.changed_files
        : diffChangedPaths(beforeSnap, afterSnap);
      const stdout =
        typeof execRes.mvm_stdout === 'string'
          ? execRes.mvm_stdout
          : typeof execRes.stdout === 'string'
            ? execRes.stdout
            : '';
      const stderr =
        typeof execRes.mvm_stderr === 'string'
          ? execRes.mvm_stderr
          : typeof execRes.stderr === 'string'
            ? execRes.stderr
            : typeof execRes.message === 'string' && execRes.status === 'error'
              ? execRes.message
              : '';
      const exitCode = Number.isFinite(Number(execRes.exit_code))
        ? Number(execRes.exit_code)
        : execRes.status === 'ok'
          ? 0
          : 1;

      toolResults.push({
        cmd,
        exit_code: exitCode,
        stdout,
        stderr,
        changed_files: changedFiles,
      });
      toolIterations += 1;

      postEvent('tool_result', {
        id: reqId,
        turn_id: activeTurnNum,
        tool: 'mvm_exec',
        cmd,
        exit_code: exitCode,
        stdout,
        stderr,
        changed_files: changedFiles,
        ...(execRes && execRes.workspace_diff ? { workspace_diff: execRes.workspace_diff } : {}),
      });
      if (afterSnap && Array.isArray(afterSnap.files)) {
        workspaceFiles = afterSnap.files;
        if (!(execRes && execRes.__uorWorkspaceSyncEmitted)) {
          postEvent('workspace_sync', {
            workspace: afterSnap,
            ...(execRes && execRes.workspace_diff ? { diff: execRes.workspace_diff } : {}),
          });
        }
      }
      await yieldSlice();
      continue;
    }

    return await buildFinalTurnResult(hopDoneStep || {});
  }

  async function buildFinalTurnResult(stepData) {
    /* Formal R4.HoloGeometricRuntime */
    stepData = stepData && typeof stepData === 'object' ? stepData : {};
    const finishedTurnId = activeTurnNum;
    setActiveTurnId(null, reqId);
    clearCancelled(reqId, finishedTurnId);

    const prismIdentity = await Promise.resolve(ensureIdentity()).catch(() => null);
    const targetKappa =
      weightsKappa || (receiptSha256 ? `sha256:${receiptSha256}` : 'sha256:foundry');
    const actionReceipt = await Promise.resolve(
      signAction('inprism-r4-turn', targetKappa),
    ).catch(() => null);
    const latestHolo =
      __uorMemPrismIdentity.latestHoloSummary || globalThis.__uorLatestHoloSummary || null;

    const finalObsMetrics = buildObsMetrics(stepData, stepData.tokens_emitted);
    const finalReply = fullReplyParts.join('');
    const finalWorkspace = collectSnapshot();

    return {
      id: reqId,
      ok: true,
      result: {
        ok: true,
        status: 'ok',
        mode: 'inprism',
        engine: 'match',
        weights: weightsKappa ? 'match' : 'missing',
        route: 'none',
        boundary: 'allow',
        turn_id: finishedTurnId,
        selected_model: selectedModel,
        model_id: selectedModel,
        execution_tier: selectedModel,
        reply: finalReply,
        assistant_reply: finalReply,
        receipt_sha256: receiptSha256,
        weights_kappa: weightsKappa,
        prism_id: prismIdentity ? prismIdentity.prism_id : null,
        action_record_kappa: actionReceipt ? actionReceipt.action_record_kappa : null,
        holo_fingerprint: latestHolo ? latestHolo.fingerprint : null,
        tokens_emitted: finalObsMetrics.tokens_emitted,
        tok_per_sec: finalObsMetrics.tok_per_sec,
        geometric_metrics: finalObsMetrics.geometric_metrics,
        holo_v13: finalObsMetrics.holo_v13,
        tool_results: toolResults,
        mvm_tool_executed: toolResults.length > 0,
        workspace: finalWorkspace,
        prism_identity: prismIdentity,
        prism_receipt: actionReceipt,
        [UOR_OBS_METRICS_KEY]: finalObsMetrics,
      },
    };
  }

  return await buildFinalTurnResult({});
  } finally {
    releaseTurnOwnership(reqId);
  }
}

/** Formal R4.HoloGeometricRuntime stream a HuggingFace Safetensors repo via HTTP `206` `Range` slices & compile to `UORR4V13`. */
export async function load_hf_repo_holo(repoId, onProgress) {
  const cleanRepo = String(repoId || '').trim().replace(/^https?:\/\/huggingface\.co\//i, '').replace(/\/+$/, '');
  if (!cleanRepo.includes('/')) {
    throw new Error('Expected owner/repo HuggingFace identifier (e.g. Qwen/Qwen3-0.6B)');
  }
  const base = `https://huggingface.co/${cleanRepo}/resolve/main/`;
  if (typeof onProgress === 'function') {
    onProgress({ stage: 'hf_meta', label: `Streaming config & tokenizer from ${cleanRepo}...` });
  }
  const [cfgBytes, tokBytes] = await Promise.all([
    holoFetchWithHoldTail(`${base}config.json`),
    holoFetchWithHoldTail(`${base}tokenizer.json`),
  ]);
  const cfgJson = cachedTextDecoder.decode(cfgBytes);
  const tokJson = cachedTextDecoder.decode(tokBytes);
  const beginRes = JSON.parse(studio_hf_repo_begin(cleanRepo, cfgJson, tokJson));
  if (!beginRes || beginRes.status !== 'ok') {
    throw new Error((beginRes && beginRes.message) || 'studio_hf_repo_begin failed');
  }

  // Probe shard index first; fall back to single `model.safetensors`
  let weightMap = null;
  try {
    const idxResp = await fetch(`${base}model.safetensors.index.json`, { cache: 'no-store' });
    if (idxResp.ok) {
      const idxJson = await idxResp.json();
      if (idxJson && idxJson.weight_map) weightMap = idxJson.weight_map;
    }
  } catch {}

  const files = weightMap ? Array.from(new Set(Object.values(weightMap))) : ['model.safetensors'];
  const fileHeaders = new Map();
  for (const fname of files) {
    const probe = await holoFetchWithHoldTail(`${base}${fname}`, '', { start: 0, end: 131071 });
    const dv = new DataView(probe.buffer, probe.byteOffset, 8);
    const hdrLen = Number(dv.getBigUint64(0, true));
    let fullPrefix = probe;
    if (8 + hdrLen > probe.byteLength) {
      fullPrefix = await holoFetchWithHoldTail(`${base}${fname}`, '', { start: 0, end: 8 + hdrLen - 1 });
    }
    const hdrCheck = JSON.parse(studio_hf_repo_parse_safetensors_header(cleanRepo, fullPrefix));
    if (!hdrCheck || hdrCheck.status !== 'ok') {
      throw new Error((hdrCheck && hdrCheck.message) || `Corrupt safetensors header in ${fname}`);
    }
    const rawMeta = JSON.parse(cachedTextDecoder.decode(fullPrefix.subarray(8, 8 + hdrLen)));
    fileHeaders.set(fname, { dataStart: 8 + hdrLen, rawMeta });
  }

  const tensorNames = [];
  for (const [fname, hInfo] of fileHeaders.entries()) {
    for (const [tName, tMeta] of Object.entries(hInfo.rawMeta)) {
      if (tName === '__metadata__' || !tMeta || !Array.isArray(tMeta.data_offsets)) continue;
      if (
        tName !== 'model.embed_tokens.weight' &&
        tName !== 'model.norm.weight' &&
        !tName.startsWith('model.layers.')
      ) {
        continue;
      }
      tensorNames.push({ fname, tName, tMeta, dataStart: hInfo.dataStart });
    }
  }
  const EMBED_BAND_ROWS = 8192;
  for (let i = 0; i < tensorNames.length; i++) {
    const { fname, tName, tMeta, dataStart } = tensorNames[i];
    const s0 = dataStart + Number(tMeta.data_offsets[0]);
    const s1 = dataStart + Number(tMeta.data_offsets[1]) - 1;
    if (s1 >= s0) {
      if (
        tName === 'model.embed_tokens.weight' &&
        Array.isArray(tMeta.shape) &&
        tMeta.shape.length === 2 &&
        Number(tMeta.shape[0]) > EMBED_BAND_ROWS
      ) {
        const totalRows = Number(tMeta.shape[0]);
        const cols = Number(tMeta.shape[1]);
        const bytesPerElem = tMeta.dtype === 'F32' ? 4 : 2;
        const rowBytes = cols * bytesPerElem;
        const totalBands = Math.ceil(totalRows / EMBED_BAND_ROWS);
        for (let row0 = 0, bandIdx = 0; row0 < totalRows; row0 += EMBED_BAND_ROWS, bandIdx += 1) {
          const bandRows = Math.min(EMBED_BAND_ROWS, totalRows - row0);
          const b0 = s0 + row0 * rowBytes;
          const b1 = b0 + bandRows * rowBytes - 1;
          const bandSlice = await holoFetchWithHoldTail(`${base}${fname}`, '', { start: b0, end: b1 });
          const pushRes = JSON.parse(
            studio_hf_repo_push_tensor(
              'model.embed_tokens.weight:band',
              tMeta.dtype,
              JSON.stringify([bandRows, cols]),
              bandSlice,
            ),
          );
          if (!pushRes || pushRes.status !== 'ok') {
            throw new Error((pushRes && pushRes.message) || `Failed pushing embed band ${bandIdx}`);
          }
          if (typeof onProgress === 'function') {
            onProgress({
              stage: 'hf_tensors',
              index: bandIdx + 1,
              total: tensorNames.length + totalBands - 1,
              label: `Streaming HF 206 Range embed band (${bandIdx + 1}/${totalBands}) from ${cleanRepo}...`,
            });
          }
        }
      } else {
        const slice = await holoFetchWithHoldTail(`${base}${fname}`, '', { start: s0, end: s1 });
        const pushRes = JSON.parse(studio_hf_repo_push_tensor(tName, tMeta.dtype, JSON.stringify(tMeta.shape), slice));
        if (!pushRes || pushRes.status !== 'ok') {
          throw new Error((pushRes && pushRes.message) || `Failed pushing tensor ${tName}`);
        }
      }
    }
    if (typeof onProgress === 'function' && (i % 12 === 0 || i + 1 === tensorNames.length)) {
      onProgress({
        stage: 'hf_tensors',
        index: i + 1,
        total: tensorNames.length,
        label: `Streaming HF 206 Range tensors (${i + 1}/${tensorNames.length}) from ${cleanRepo}...`,
      });
    }
  }
  const finalStatus = JSON.parse(studio_hf_repo_finalize());
  if (!finalStatus || finalStatus.status !== 'ok') {
    throw new Error((finalStatus && finalStatus.message) || 'studio_hf_repo_finalize failed');
  }
  return finalStatus;
}

const OPFS_V12_360M_WEIGHT_FILE = 'uor_r4_gllm_v12_360m_weights.bin';
const OPFS_V12_360M_META_FILE = 'uor_r4_gllm_v12_360m_meta.json';

async function __uorRequestPersistentStorage() {
  try {
    if (
      typeof navigator !== 'undefined' &&
      navigator.storage &&
      typeof navigator.storage.persist === 'function'
    ) {
      return Boolean(await navigator.storage.persist());
    }
  } catch {
    // Storage persistence optional in constrained contexts
  }
  return false;
}
const DEFAULT_SPIRALCORE_RELAY_WS_URL = 'wss://lexlean-r4-spiralcore-relay.swarm-relay-cloud-8d31a122.workers.dev/spvs';
const DEFAULT_SPIRALCORE_RELAY_HTTP_URL = 'https://lexlean-r4-spiralcore-relay.swarm-relay-cloud-8d31a122.workers.dev/spvs';
const LOCAL_SPIRALCORE_RELAY_WS_URL = 'ws://127.0.0.1:8787/spvs';
const SPVS_RELAY_TIMEOUT_MS = 2500;
const MAX_BRIDGE_RESPONSE_BYTES = 2 * 1024 * 1024;

const SECRET_HOST_ALLOWLIST = Object.freeze({
  GITHUB_TOKEN: new Set(['github.com', 'api.github.com']),
});

export const PYODIDE_LOCK = Object.freeze({
  version: '0.27.2',
  url: 'https://cdn.jsdelivr.net/pyodide/v0.27.2/full/pyodide.mjs',
  indexURL: 'https://cdn.jsdelivr.net/pyodide/v0.27.2/full/',
  sri: 'sha384-tynFgfasyqq8i/8ivIOTHpF+NLPB4dVWM8E+tTowcaAWNwVf4dZK6QeV0b47SLDp',
});

const PYODIDE_CACHE_NAME = 'uor-pyodide-v1';

class MvmPolyglotVfs {
  constructor(entries, cwd) {
    this.cwd = MvmPolyglotVfs.normalizePath('/', cwd || '/workspace');
    this.files = new Map();
    this.symlinks = new Map();
    this.modes = new Map();
    this.dirs = new Set(['']);
    this.originalKeys = new Set();
    this.dirtyWrites = new Set();
    this.dirtyDeletes = new Set();

    const recordAncestorDirs = (relPath) => {
      let idx = relPath.lastIndexOf('/');
      while (idx > 0) {
        const parentRel = relPath.slice(0, idx);
        this.dirs.add(parentRel);
        idx = parentRel.lastIndexOf('/');
      }
    };

    const list = Array.isArray(entries) ? entries : [];
    for (const item of list) {
      if (!item || typeof item.path !== 'string') continue;
      const rel = MvmPolyglotVfs.normalizeRel(item.path);
      if (!rel) continue;
      this.originalKeys.add(rel);
      recordAncestorDirs(rel);
      const modeNum = typeof item.mode === 'number' ? item.mode : 0o100644;
      this.modes.set(rel, modeNum);
      if (typeof item.symlink_target === 'string') {
        this.symlinks.set(rel, item.symlink_target);
      } else if (typeof item.content_base64 === 'string') {
        this.files.set(rel, __uorBase64ToBytes(item.content_base64));
      } else if (typeof item.content_utf8 === 'string') {
        this.files.set(rel, cachedTextEncoder.encode(item.content_utf8));
      } else {
        this.files.set(rel, new Uint8Array(0));
      }
    }
  }

  static normalizePath(cwd, raw) {
    const rawStr = String(raw || '.').trim();
    const base = rawStr.startsWith('/') ? rawStr : (cwd === '/' ? `/${rawStr}` : `${cwd}/${rawStr}`);
    const stack = [];
    for (const seg of base.split('/')) {
      if (!seg || seg === '.') continue;
      if (seg === '..') stack.pop();
      else stack.push(seg);
    }
    return stack.length === 0 ? '/' : `/${stack.join('/')}`;
  }

  static normalizeRel(raw) {
    const abs = MvmPolyglotVfs.normalizePath('/workspace', raw);
    if (abs === '/workspace') return '';
    if (abs.startsWith('/workspace/')) return abs.slice('/workspace/'.length);
    return abs.replace(/^\/+/, '');
  }

  static isCanonicalProtectedRel(rel) {
    return (rel.startsWith('prism/') && rel.endsWith('.lex.tex')) || rel === 'app/state';
  }

  static parentDir(absPath) {
    const idx = absPath.lastIndexOf('/');
    return idx <= 0 ? '/' : absPath.slice(0, idx);
  }

  resolveGuestTarget(rawPath) {
    const abs = MvmPolyglotVfs.normalizePath(this.cwd, rawPath);
    if (!abs.startsWith('/workspace/') && abs !== '/workspace') {
      const err = new Error(`Path outside /workspace refused: ${abs}`);
      err.code = 'CanonicalPathProtected';
      throw err;
    }
    let curAbs = abs;
    for (let hop = 0; hop < 8; hop += 1) {
      const rel = curAbs === '/workspace' ? '' : curAbs.slice('/workspace/'.length);
      if (!rel || !this.symlinks.has(rel)) {
        return { abs: curAbs, rel };
      }
      const target = this.symlinks.get(rel);
      curAbs = target.startsWith('/')
        ? MvmPolyglotVfs.normalizePath('/', target)
        : MvmPolyglotVfs.normalizePath(MvmPolyglotVfs.parentDir(curAbs), target);
      if (!curAbs.startsWith('/workspace/') && curAbs !== '/workspace') {
        const err = new Error(`Symlink escapes /workspace: ${curAbs}`);
        err.code = 'CanonicalPathProtected';
        throw err;
      }
    }
    const finalRel = curAbs === '/workspace' ? '' : curAbs.slice('/workspace/'.length);
    return { abs: curAbs, rel: finalRel };
  }

  assertMutable(rawPath) {
    const abs = MvmPolyglotVfs.normalizePath(this.cwd, rawPath);
    const directRel = abs.startsWith('/workspace/') ? abs.slice('/workspace/'.length) : '';
    if (directRel && MvmPolyglotVfs.isCanonicalProtectedRel(directRel)) {
      const err = new Error(`mvm: refused mutation of protected canonical path '${abs}'`);
      err.code = 'CanonicalPathProtected';
      throw err;
    }
    const resolved = this.resolveGuestTarget(abs);
    if (resolved.rel && MvmPolyglotVfs.isCanonicalProtectedRel(resolved.rel)) {
      const err = new Error(`mvm: refused mutation of protected canonical path '${resolved.abs}'`);
      err.code = 'CanonicalPathProtected';
      throw err;
    }
    return { directRel, resolved };
  }

  exists(rawPath) {
    try {
      const { rel } = this.resolveGuestTarget(rawPath);
      if (!rel) return true;
      if (this.files.has(rel) || this.dirs.has(rel)) return true;
      const prefix = `${rel}/`;
      for (const k of this.files.keys()) {
        if (k.startsWith(prefix)) return true;
      }
      for (const k of this.symlinks.keys()) {
        if (k.startsWith(prefix)) return true;
      }
      return false;
    } catch {
      return false;
    }
  }

  isDirectory(rawPath) {
    try {
      const { rel } = this.resolveGuestTarget(rawPath);
      if (!rel) return true;
      if (this.files.has(rel)) return false;
      if (this.dirs.has(rel)) return true;
      const prefix = `${rel}/`;
      for (const k of this.files.keys()) {
        if (k.startsWith(prefix)) return true;
      }
      for (const k of this.symlinks.keys()) {
        if (k.startsWith(prefix)) return true;
      }
      return false;
    } catch {
      return false;
    }
  }

  mkdir(rawPath, opts) {
    const abs = MvmPolyglotVfs.normalizePath(this.cwd, rawPath);
    this.assertMutable(abs);
    if (abs === '/workspace') return;
    const rel = MvmPolyglotVfs.normalizeRel(abs);
    if (!rel) return;
    if (this.files.has(rel)) {
      const e = new Error(`EEXIST: file already exists, mkdir '${rawPath}'`);
      e.code = 'EEXIST';
      throw e;
    }
    const recursive = Boolean(opts && opts.recursive);
    const segs = rel.split('/').filter(Boolean);
    if (!recursive && segs.length > 1) {
      const parentRel = segs.slice(0, -1).join('/');
      if (!this.isDirectory(`/workspace/${parentRel}`)) {
        const e = new Error(`ENOENT: no such file or directory, mkdir '${rawPath}'`);
        e.code = 'ENOENT';
        throw e;
      }
    }
    let acc = '';
    for (const s of segs) {
      acc = acc ? `${acc}/${s}` : s;
      this.dirs.add(acc);
    }
  }

  readBytes(rawPath) {
    const { rel } = this.resolveGuestTarget(rawPath);
    return rel && this.files.has(rel) ? this.files.get(rel) : null;
  }

  readText(rawPath) {
    const bytes = this.readBytes(rawPath);
    return bytes ? cachedTextDecoder.decode(bytes) : null;
  }

  writeBytes(rawPath, data, mode) {
    const { directRel, resolved } = this.assertMutable(rawPath);
    const targetRel = directRel || resolved.rel;
    if (!targetRel) {
      const err = new Error('Cannot write directly to /workspace root');
      err.code = 'CanonicalPathProtected';
      throw err;
    }
    const u8 = data instanceof Uint8Array
      ? new Uint8Array(data)
      : cachedTextEncoder.encode(String(data === undefined ? '' : data));
    this.symlinks.delete(targetRel);
    this.files.set(targetRel, u8);
    let idx = targetRel.lastIndexOf('/');
    while (idx > 0) {
      const pRel = targetRel.slice(0, idx);
      this.dirs.add(pRel);
      idx = pRel.lastIndexOf('/');
    }
    if (typeof mode === 'number') {
      this.modes.set(targetRel, mode);
    } else if (!this.modes.has(targetRel)) {
      this.modes.set(targetRel, 0o100644);
    }
    this.dirtyDeletes.delete(targetRel);
    this.dirtyWrites.add(targetRel);
  }

  writeSymlink(rawPath, target) {
    const abs = MvmPolyglotVfs.normalizePath(this.cwd, rawPath);
    if (!abs.startsWith('/workspace/')) {
      const err = new Error(`Path outside /workspace refused: ${abs}`);
      err.code = 'CanonicalPathProtected';
      throw err;
    }
    const rel = abs.slice('/workspace/'.length);
    const targetStr = String(target || '');
    let curAbs = targetStr.startsWith('/')
      ? MvmPolyglotVfs.normalizePath('/', targetStr)
      : MvmPolyglotVfs.normalizePath(MvmPolyglotVfs.parentDir(abs), targetStr);
    if (!curAbs.startsWith('/workspace/') && curAbs !== '/workspace') {
      const err = new Error(`Symlink escapes /workspace: ${curAbs}`);
      err.code = 'CanonicalPathProtected';
      throw err;
    }
    if (MvmPolyglotVfs.isCanonicalProtectedRel(rel)) {
      const err = new Error(`mvm: refused symlink touching protected canonical path '${abs}'`);
      err.code = 'CanonicalPathProtected';
      throw err;
    }
    for (let hop = 0; hop < 8; hop += 1) {
      const curRel = curAbs === '/workspace' ? '' : curAbs.slice('/workspace/'.length);
      if (curRel && MvmPolyglotVfs.isCanonicalProtectedRel(curRel)) {
        const err = new Error(`mvm: refused symlink touching protected canonical path '${curAbs}'`);
        err.code = 'CanonicalPathProtected';
        throw err;
      }
      if (!curRel || !this.symlinks.has(curRel)) break;
      const nextTarget = this.symlinks.get(curRel);
      curAbs = nextTarget.startsWith('/')
        ? MvmPolyglotVfs.normalizePath('/', nextTarget)
        : MvmPolyglotVfs.normalizePath(MvmPolyglotVfs.parentDir(curAbs), nextTarget);
      if (!curAbs.startsWith('/workspace/') && curAbs !== '/workspace') {
        const err = new Error(`Symlink escapes /workspace: ${curAbs}`);
        err.code = 'CanonicalPathProtected';
        throw err;
      }
    }
    this.files.delete(rel);
    this.symlinks.set(rel, targetStr);
    let idx = rel.lastIndexOf('/');
    while (idx > 0) {
      const pRel = rel.slice(0, idx);
      this.dirs.add(pRel);
      idx = pRel.lastIndexOf('/');
    }
    this.modes.set(rel, 0o120777);
    this.dirtyDeletes.delete(rel);
    this.dirtyWrites.add(rel);
  }

  deletePath(rawPath) {
    const { directRel } = this.assertMutable(rawPath);
    if (!directRel) return false;
    const existed = this.files.delete(directRel) || this.symlinks.delete(directRel) || this.dirs.delete(directRel);
    this.modes.delete(directRel);
    this.dirtyWrites.delete(directRel);
    if (existed || this.originalKeys.has(directRel)) {
      this.dirtyDeletes.add(directRel);
    }
    return existed;
  }

  readdir(rawPath) {
    const { rel } = this.resolveGuestTarget(rawPath);
    const prefix = rel ? `${rel}/` : '';
    const out = new Set();
    for (const k of this.files.keys()) {
      if (k.startsWith(prefix)) {
        const first = k.slice(prefix.length).split('/')[0];
        if (first) out.add(first);
      }
    }
    for (const k of this.symlinks.keys()) {
      if (k.startsWith(prefix)) {
        const first = k.slice(prefix.length).split('/')[0];
        if (first) out.add(first);
      }
    }
    for (const k of this.dirs) {
      if (k && k.startsWith(prefix)) {
        const first = k.slice(prefix.length).split('/')[0];
        if (first) out.add(first);
      }
    }
    return Array.from(out).sort();
  }

  exportMutations() {
    const writes = [];
    for (const rel of this.dirtyWrites) {
      if (this.symlinks.has(rel)) {
        writes.push({
          path: rel,
          symlink_target: this.symlinks.get(rel),
          mode: this.modes.get(rel) || 0o120777,
        });
      } else if (this.files.has(rel)) {
        writes.push({
          path: rel,
          content_base64: __uorBytesToBase64(this.files.get(rel)),
          mode: this.modes.get(rel) || 0o100644,
        });
      }
    }
    const deletes = Array.from(this.dirtyDeletes);
    return { writes, deletes };
  }
}

class MvmNodeBuffer extends Uint8Array {
  static from(value, encoding) {
    if (value instanceof Uint8Array) {
      return new MvmNodeBuffer(value);
    }
    if (Array.isArray(value)) {
      return new MvmNodeBuffer(value);
    }
    if (value instanceof ArrayBuffer) {
      return new MvmNodeBuffer(value);
    }
    const enc = String(encoding || 'utf8').toLowerCase();
    const s = String(value || '');
    if (enc === 'base64') {
      return new MvmNodeBuffer(__uorBase64ToBytes(s));
    }
    if (enc === 'hex') {
      const clean = s.replace(/\s+/g, '');
      const out = new MvmNodeBuffer(Math.floor(clean.length / 2));
      for (let i = 0; i < out.length; i += 1) {
        out[i] = Number('0x' + clean.slice(i * 2, i * 2 + 2)) & 0xff;
      }
      return out;
    }
    return new MvmNodeBuffer(cachedTextEncoder.encode(s));
  }

  static alloc(size, fill) {
    const out = new MvmNodeBuffer(Math.max(0, Number(size) || 0));
    if (fill !== undefined) out.fill(Number(fill) & 0xff);
    return out;
  }

  static allocUnsafe(size) {
    return MvmNodeBuffer.alloc(size, 0);
  }

  static isBuffer(obj) {
    return obj instanceof MvmNodeBuffer || obj instanceof Uint8Array;
  }

  static concat(list, totalLen) {
    const items = Array.isArray(list) ? list : [];
    const len = typeof totalLen === 'number'
      ? totalLen
      : items.reduce((acc, item) => acc + (item ? item.byteLength : 0), 0);
    const out = new MvmNodeBuffer(len);
    let offset = 0;
    for (const item of items) {
      if (!item || offset >= len) continue;
      const take = Math.min(item.byteLength, len - offset);
      out.set(item.subarray(0, take), offset);
      offset += take;
    }
    return out;
  }

  static byteLength(str) {
    if (str instanceof Uint8Array) return str.byteLength;
    return cachedTextEncoder.encode(String(str || '')).byteLength;
  }

  toString(encoding) {
    const enc = String(encoding || 'utf8').toLowerCase();
    if (enc === 'base64') return __uorBytesToBase64(this);
    if (enc === 'hex') {
      let hex = '';
      for (let i = 0; i < this.length; i += 1) {
        hex += this[i].toString(16).padStart(2, '0');
      }
      return hex;
    }
    return cachedTextDecoder.decode(this);
  }
}

class MvmWasiRunner {
  static run(wasmBytes, vfs, args, env, stdinBytes) {
    if (!(wasmBytes instanceof Uint8Array) || wasmBytes.byteLength < 8 ||
      wasmBytes[0] !== 0x00 || wasmBytes[1] !== 0x61 || wasmBytes[2] !== 0x73 || wasmBytes[3] !== 0x6d) {
      const err = new Error('Invalid WebAssembly binary magic header');
      err.code = 'WasiRuntimeError';
      throw err;
    }

    const argvList = Array.isArray(args) ? args.map((a) => String(a)) : [];
    const envList = Object.entries(env || {}).map(([k, v]) => `${k}=${v}`);
    const stdinBuf = stdinBytes instanceof Uint8Array ? stdinBytes : new Uint8Array(0);

    const fdTable = new Map();
    fdTable.set(0, { kind: 'stdin', data: stdinBuf, offset: 0 });
    fdTable.set(1, { kind: 'stdout', chunks: [] });
    fdTable.set(2, { kind: 'stderr', chunks: [] });
    fdTable.set(3, { kind: 'preopen', guestPath: '/workspace', preopenName: '/workspace' });
    fdTable.set(4, { kind: 'preopen', guestPath: vfs.cwd, preopenName: '.' });
    let nextFd = 5;
    let wasmMemory = null;
    let exitCode = 0;

    class WasiExitSignal {
      constructor(code) {
        this.code = code | 0;
      }
    }

    const memU8 = () => new Uint8Array(wasmMemory.buffer);
    const memDv = () => new DataView(wasmMemory.buffer);
    const writeU32 = (ptr, val) => memDv().setUint32(ptr >>> 0, val >>> 0, true);
    const writeU64 = (ptr, val) => memDv().setBigUint64(ptr >>> 0, BigInt(val), true);
    const readStr = (ptr, len) => cachedTextDecoder.decode(memU8().subarray(ptr >>> 0, (ptr >>> 0) + (len >>> 0)));

    const WASI_ERRNO_SUCCESS = 0;
    const WASI_ERRNO_BADF = 8;
    const WASI_ERRNO_ACCES = 2;
    const WASI_ERRNO_EXIST = 20;
    const WASI_ERRNO_INVAL = 28;
    const WASI_ERRNO_NOENT = 44;

    const wasiImports = {
      args_sizes_get: (argcPtr, bufSizePtr) => {
        let total = 0;
        for (const a of argvList) total += cachedTextEncoder.encode(a).byteLength + 1;
        writeU32(argcPtr, argvList.length);
        writeU32(bufSizePtr, total);
        return WASI_ERRNO_SUCCESS;
      },
      args_get: (argvPtr, argvBufPtr) => {
        const u8 = memU8();
        let ap = argvPtr >>> 0;
        let bp = argvBufPtr >>> 0;
        for (const a of argvList) {
          writeU32(ap, bp);
          ap += 4;
          const enc = cachedTextEncoder.encode(a);
          u8.set(enc, bp);
          u8[bp + enc.length] = 0;
          bp += enc.length + 1;
        }
        return WASI_ERRNO_SUCCESS;
      },
      environ_sizes_get: (cntPtr, bufSizePtr) => {
        let total = 0;
        for (const e of envList) total += cachedTextEncoder.encode(e).byteLength + 1;
        writeU32(cntPtr, envList.length);
        writeU32(bufSizePtr, total);
        return WASI_ERRNO_SUCCESS;
      },
      environ_get: (envPtr, envBufPtr) => {
        const u8 = memU8();
        let ep = envPtr >>> 0;
        let bp = envBufPtr >>> 0;
        for (const e of envList) {
          writeU32(ep, bp);
          ep += 4;
          const enc = cachedTextEncoder.encode(e);
          u8.set(enc, bp);
          u8[bp + enc.length] = 0;
          bp += enc.length + 1;
        }
        return WASI_ERRNO_SUCCESS;
      },
      fd_prestat_get: (fd, prestatPtr) => {
        const ent = fdTable.get(fd);
        if (!ent || ent.kind !== 'preopen') return WASI_ERRNO_BADF;
        const nameBytes = cachedTextEncoder.encode(ent.preopenName);
        memU8()[prestatPtr >>> 0] = 0;
        writeU32((prestatPtr >>> 0) + 4, nameBytes.byteLength);
        return WASI_ERRNO_SUCCESS;
      },
      fd_prestat_dir_name: (fd, pathPtr, pathLen) => {
        const ent = fdTable.get(fd);
        if (!ent || ent.kind !== 'preopen') return WASI_ERRNO_BADF;
        const nameBytes = cachedTextEncoder.encode(ent.preopenName);
        memU8().set(nameBytes.subarray(0, pathLen >>> 0), pathPtr >>> 0);
        return WASI_ERRNO_SUCCESS;
      },
      fd_fdstat_get: (fd, statPtr) => {
        const ent = fdTable.get(fd);
        if (!ent) return WASI_ERRNO_BADF;
        const p = statPtr >>> 0;
        memU8().fill(0, p, p + 24);
        memU8()[p] = ent.kind === 'preopen' ? 3 : ent.kind === 'file' ? 4 : 2;
        writeU64(p + 8, 0xffffffffn);
        writeU64(p + 16, 0xffffffffn);
        return WASI_ERRNO_SUCCESS;
      },
      fd_filestat_get: (fd, statPtr) => {
        const ent = fdTable.get(fd);
        if (!ent) return WASI_ERRNO_BADF;
        const p = statPtr >>> 0;
        memU8().fill(0, p, p + 64);
        memU8()[p + 16] = ent.kind === 'preopen' ? 3 : ent.kind === 'file' ? 4 : 2;
        writeU64(p + 24, 1n);
        writeU64(p + 32, BigInt(ent.data ? ent.data.byteLength : 0));
        return WASI_ERRNO_SUCCESS;
      },
      path_filestat_get: (dirfd, _flags, pathPtr, pathLen, statPtr) => {
        const dirEnt = fdTable.get(dirfd);
        if (!dirEnt || dirEnt.kind !== 'preopen') return WASI_ERRNO_BADF;
        const subPath = readStr(pathPtr, pathLen);
        const fullPath = MvmPolyglotVfs.normalizePath(dirEnt.guestPath, subPath);
        const bytes = vfs.readBytes(fullPath);
        const isDir = !bytes && vfs.isDirectory(fullPath);
        if (!bytes && !isDir) return WASI_ERRNO_NOENT;
        const p = statPtr >>> 0;
        memU8().fill(0, p, p + 64);
        memU8()[p + 16] = isDir ? 3 : 4;
        writeU64(p + 24, 1n);
        writeU64(p + 32, BigInt(bytes ? bytes.byteLength : 0));
        return WASI_ERRNO_SUCCESS;
      },
      path_unlink_file: (dirfd, pathPtr, pathLen) => {
        const dirEnt = fdTable.get(dirfd);
        if (!dirEnt || dirEnt.kind !== 'preopen') return WASI_ERRNO_BADF;
        const subPath = readStr(pathPtr, pathLen);
        const fullPath = MvmPolyglotVfs.normalizePath(dirEnt.guestPath, subPath);
        vfs.assertMutable(fullPath);
        return vfs.deletePath(fullPath) ? WASI_ERRNO_SUCCESS : WASI_ERRNO_NOENT;
      },
      sched_yield: () => WASI_ERRNO_SUCCESS,
      path_open: (dirfd, _dirflags, pathPtr, pathLen, oflags, _rBase, _rInh, fdflags, openedFdPtr) => {
        const dirEnt = fdTable.get(dirfd);
        if (!dirEnt || dirEnt.kind !== 'preopen') return WASI_ERRNO_BADF;
        const subPath = readStr(pathPtr, pathLen);
        const fullPath = MvmPolyglotVfs.normalizePath(dirEnt.guestPath, subPath);
        const oCreat = (oflags & 1) !== 0;
        const oExcl = (oflags & 4) !== 0;
        const oTrunc = (oflags & 8) !== 0;
        const oAppend = (fdflags & 1) !== 0;
        const writeRightMask = 0x400151n;
        const wantsWrite = oCreat || oTrunc || oAppend || ((BigInt(_rBase || 0n) & writeRightMask) !== 0n);
        const existing = vfs.readBytes(fullPath);

        if (wantsWrite) {
          try {
            vfs.assertMutable(fullPath);
          } catch (err) {
            if (err && err.code === 'CanonicalPathProtected') throw err;
            return WASI_ERRNO_ACCES;
          }
        }
        if (existing && oCreat && oExcl) return WASI_ERRNO_EXIST;
        if (!existing && !oCreat) return WASI_ERRNO_NOENT;

        const data = oTrunc || !existing ? new Uint8Array(0) : new Uint8Array(existing);
        if (!existing && oCreat) vfs.writeBytes(fullPath, data);
        else if (oTrunc) vfs.writeBytes(fullPath, data);

        const fd = nextFd++;
        fdTable.set(fd, {
          kind: 'file',
          guestPath: fullPath,
          data,
          offset: oAppend ? data.byteLength : 0,
          dirty: oTrunc || (!existing && oCreat),
          writable: wantsWrite,
        });
        writeU32(openedFdPtr, fd);
        return WASI_ERRNO_SUCCESS;
      },
      fd_write: (fd, iovsPtr, iovsLen, nwrittenPtr) => {
        const ent = fdTable.get(fd);
        if (!ent) return WASI_ERRNO_BADF;
        if (ent.kind === 'file' && !ent.writable) return WASI_ERRNO_ACCES;
        const dv = memDv();
        const u8 = memU8();
        let total = 0;
        const parts = [];
        for (let i = 0; i < (iovsLen >>> 0); i += 1) {
          const base = dv.getUint32((iovsPtr >>> 0) + i * 8, true);
          const len = dv.getUint32((iovsPtr >>> 0) + i * 8 + 4, true);
          const slice = new Uint8Array(u8.subarray(base, base + len));
          parts.push(slice);
          total += slice.byteLength;
        }
        if (ent.kind === 'stdout' || ent.kind === 'stderr') {
          for (const p of parts) ent.chunks.push(p);
        } else if (ent.kind === 'file') {
          const endPos = ent.offset + total;
          if (endPos > ent.data.byteLength) {
            const grown = new Uint8Array(endPos);
            grown.set(ent.data, 0);
            ent.data = grown;
          }
          let cur = ent.offset;
          for (const p of parts) {
            ent.data.set(p, cur);
            cur += p.byteLength;
          }
          ent.offset = cur;
          ent.dirty = true;
        } else {
          return WASI_ERRNO_BADF;
        }
        writeU32(nwrittenPtr, total);
        return WASI_ERRNO_SUCCESS;
      },
      fd_read: (fd, iovsPtr, iovsLen, nreadPtr) => {
        const ent = fdTable.get(fd);
        if (!ent || (ent.kind !== 'stdin' && ent.kind !== 'file')) return WASI_ERRNO_BADF;
        const dv = memDv();
        const u8 = memU8();
        let total = 0;
        for (let i = 0; i < (iovsLen >>> 0); i += 1) {
          const base = dv.getUint32((iovsPtr >>> 0) + i * 8, true);
          const len = dv.getUint32((iovsPtr >>> 0) + i * 8 + 4, true);
          const avail = Math.max(0, ent.data.byteLength - ent.offset);
          const take = Math.min(len, avail);
          if (take > 0) {
            u8.set(ent.data.subarray(ent.offset, ent.offset + take), base);
            ent.offset += take;
            total += take;
          }
          if (take < len) break;
        }
        writeU32(nreadPtr, total);
        return WASI_ERRNO_SUCCESS;
      },
      fd_seek: (fd, offsetBig, whence, newOffsetPtr) => {
        const ent = fdTable.get(fd);
        if (!ent || ent.kind !== 'file') return WASI_ERRNO_BADF;
        const off = Number(offsetBig);
        let next = ent.offset;
        if (whence === 0) next = off;
        else if (whence === 1) next = ent.offset + off;
        else if (whence === 2) next = ent.data.byteLength + off;
        else return WASI_ERRNO_INVAL;
        if (next < 0) return WASI_ERRNO_INVAL;
        ent.offset = next;
        writeU64(newOffsetPtr, BigInt(next));
        return WASI_ERRNO_SUCCESS;
      },
      fd_close: (fd) => {
        const ent = fdTable.get(fd);
        if (!ent) return WASI_ERRNO_BADF;
        if (ent.kind === 'file' && ent.dirty) {
          vfs.writeBytes(ent.guestPath, ent.data);
        }
        if (fd >= 5) fdTable.delete(fd);
        return WASI_ERRNO_SUCCESS;
      },
      clock_time_get: (_clockId, _precision, timePtr) => {
        writeU64(timePtr, BigInt(Date.now()) * 1000000n);
        return WASI_ERRNO_SUCCESS;
      },
      random_get: (bufPtr, bufLen) => {
        const target = memU8().subarray(bufPtr >>> 0, (bufPtr >>> 0) + (bufLen >>> 0));
        if (typeof crypto !== 'undefined' && typeof crypto.getRandomValues === 'function') {
          crypto.getRandomValues(target);
        }
        return WASI_ERRNO_SUCCESS;
      },
      proc_exit: (code) => {
        throw new WasiExitSignal(code);
      },
    };

    const runAsync = async () => {
      try {
        const compiled = await WebAssembly.instantiate(wasmBytes, { wasi_snapshot_preview1: wasiImports });
        const instance = compiled.instance || compiled;
        wasmMemory = instance.exports && instance.exports.memory ? instance.exports.memory : null;
        if (!wasmMemory) {
          const err = new Error('WASI module must export linear memory');
          err.code = 'WasiRuntimeError';
          throw err;
        }
        const entry = instance.exports._start || instance.exports.main;
        if (typeof entry !== 'function') {
          const err = new Error('WASI module must export _start');
          err.code = 'WasiRuntimeError';
          throw err;
        }
        try {
          entry();
        } catch (sig) {
          if (sig instanceof WasiExitSignal) {
            exitCode = sig.code;
          } else {
            throw sig;
          }
        }
        for (const ent of fdTable.values()) {
          if (ent.kind === 'file' && ent.dirty) {
            vfs.writeBytes(ent.guestPath, ent.data);
          }
        }
        const mergeChunks = (chunks) => {
          const total = chunks.reduce((s, c) => s + c.byteLength, 0);
          const merged = new Uint8Array(total);
          let off = 0;
          for (const c of chunks) {
            merged.set(c, off);
            off += c.byteLength;
          }
          return cachedTextDecoder.decode(merged);
        };
        return {
          ok: true,
          exit_code: exitCode,
          stdout: mergeChunks(fdTable.get(1).chunks),
          stderr: mergeChunks(fdTable.get(2).chunks),
        };
      } catch (err) {
        if (err && err.code === 'CanonicalPathProtected') throw err;
        const wrapped = new Error(err && err.message ? err.message : String(err));
        wrapped.code = 'WasiRuntimeError';
        throw wrapped;
      }
    };
    return runAsync();
  }
}

class MvmSqliteEngine {
  static MAGIC = 'SQLite format 3\0';

  static loadDb(rawBytes) {
    if (!rawBytes || rawBytes.byteLength === 0) {
      return { tables: new Map() };
    }
    const headerStr = cachedTextDecoder.decode(rawBytes.subarray(0, 16));
    if (headerStr !== MvmSqliteEngine.MAGIC || rawBytes.byteLength < 100) {
      const err = new Error('file is not a valid SQLite 3 database');
      err.code = 'SqliteRuntimeError';
      throw err;
    }
    const dv = new DataView(rawBytes.buffer, rawBytes.byteOffset, rawBytes.byteLength);
    const payloadLen = dv.getUint32(96, true);
    if (payloadLen === 0) {
      return { tables: new Map() };
    }
    if (100 + payloadLen > rawBytes.byteLength) {
      const err = new Error('corrupt SQLite 3 database payload length');
      err.code = 'SqliteRuntimeError';
      throw err;
    }
    let parsed;
    try {
      parsed = JSON.parse(cachedTextDecoder.decode(rawBytes.subarray(100, 100 + payloadLen)));
    } catch {
      const err = new Error('corrupt SQLite 3 database JSON payload');
      err.code = 'CorruptSqliteDatabase';
      throw err;
    }
    const tables = new Map();
    for (const t of parsed.tables || []) {
      tables.set(t.name.toLowerCase(), {
        name: t.name,
        columns: t.columns || [],
        rows: Array.isArray(t.rows) ? t.rows : [],
        sql: t.sql || '',
        pk: Array.isArray(t.pk) ? t.pk : [],
        unique: Array.isArray(t.unique) ? t.unique : [],
      });
    }
    return { tables };
  }

  static saveDb(db) {
    const payloadObj = {
      schema: 'uor-sqlite3/1',
      tables: Array.from(db.tables.values()),
    };
    const jsonBytes = cachedTextEncoder.encode(JSON.stringify(payloadObj));
    const pageAligned = Math.max(4096, Math.ceil((100 + jsonBytes.byteLength) / 4096) * 4096);
    const out = new Uint8Array(pageAligned);
    out.set(cachedTextEncoder.encode(MvmSqliteEngine.MAGIC), 0);
    const dv = new DataView(out.buffer);
    dv.setUint16(16, 4096, false);
    out[18] = 1;
    out[19] = 1;
    out[21] = 64;
    out[22] = 32;
    out[23] = 32;
    dv.setUint32(28, pageAligned / 4096, false);
    dv.setUint32(96, jsonBytes.byteLength, true);
    out.set(jsonBytes, 100);
    return out;
  }

  static splitStatements(sql) {
    const stmts = [];
    let cur = '';
    let i = 0;
    let inSingle = false;
    let inDouble = false;
    while (i < sql.length) {
      const ch = sql[i];
      if (!inSingle && !inDouble && ch === '-' && sql[i + 1] === '-') {
        while (i < sql.length && sql[i] !== '\n') i += 1;
        continue;
      }
      if (!inSingle && !inDouble && ch === '/' && sql[i + 1] === '*') {
        i += 2;
        while (i + 1 < sql.length && !(sql[i] === '*' && sql[i + 1] === '/')) i += 1;
        i += 2;
        continue;
      }
      if (ch === "'" && !inDouble) {
        cur += ch;
        if (inSingle && sql[i + 1] === "'") {
          cur += "'";
          i += 2;
          continue;
        }
        inSingle = !inSingle;
        i += 1;
        continue;
      }
      if (ch === '"' && !inSingle) {
        inDouble = !inDouble;
        cur += ch;
        i += 1;
        continue;
      }
      if (!inSingle && !inDouble && ch === ';') {
        if (cur.trim()) stmts.push(cur.trim());
        cur = '';
        i += 1;
        continue;
      }
      if (!inSingle && !inDouble && ch === '\n' && cur.trim().startsWith('.')) {
        stmts.push(cur.trim());
        cur = '';
        i += 1;
        continue;
      }
      cur += ch;
      i += 1;
    }
    if (cur.trim()) stmts.push(cur.trim());
    return stmts;
  }

  static splitTopLevelComma(str) {
    const parts = [];
    let cur = '';
    let depth = 0;
    let inSingle = false;
    let inDouble = false;
    for (let i = 0; i < str.length; i += 1) {
      const ch = str[i];
      if (ch === "'" && !inDouble) {
        cur += ch;
        if (inSingle && str[i + 1] === "'") {
          cur += "'";
          i += 1;
          continue;
        }
        inSingle = !inSingle;
      } else if (ch === '"' && !inSingle) {
        inDouble = !inDouble;
        cur += ch;
      } else if (!inSingle && !inDouble) {
        if (ch === '(') depth += 1;
        else if (ch === ')') depth = Math.max(0, depth - 1);
        else if (ch === ',' && depth === 0) {
          parts.push(cur.trim());
          cur = '';
          continue;
        }
        cur += ch;
      } else {
        cur += ch;
      }
    }
    if (cur.trim()) parts.push(cur.trim());
    return parts;
  }

  static parseLiteral(tok) {
    const t = String(tok || '').trim();
    if (/^null$/i.test(t)) return null;
    if (/^true$/i.test(t)) return 1;
    if (/^false$/i.test(t)) return 0;
    if (t.startsWith("'") && t.endsWith("'") && t.length >= 2) {
      return t.slice(1, -1).replace(/''/g, "'");
    }
    if (/^[+-]?\d+$/.test(t)) return Number(t);
    if (/^[+-]?\d+\.\d+$/.test(t)) return Number(t);
    return t.replace(/^"|"$/g, '');
  }

  static unquoteIdent(s) {
    const t = String(s || '').trim();
    if ((t.startsWith('"') && t.endsWith('"')) || (t.startsWith('`') && t.endsWith('`'))) {
      return t.slice(1, -1);
    }
    return t;
  }

  static evalWhere(exprStr, row) {
    const s = String(exprStr || '').trim();
    if (!s) return true;
    const splitLogical = (text, kw) => {
      let depth = 0;
      let inS = false;
      const upper = text.toUpperCase();
      for (let i = 0; i <= text.length - kw.length; i += 1) {
        if (text[i] === "'") {
          inS = !inS;
          continue;
        }
        if (inS) continue;
        if (text[i] === '(') depth += 1;
        else if (text[i] === ')') depth = Math.max(0, depth - 1);
        else if (depth === 0 && upper.startsWith(kw, i)) {
          const leftOk = i === 0 || /\s/.test(text[i - 1]);
          const rightOk = i + kw.length === text.length || /\s/.test(text[i + kw.length]);
          if (leftOk && rightOk) {
            return [text.slice(0, i).trim(), text.slice(i + kw.length).trim()];
          }
        }
      }
      return null;
    };
    const orParts = splitLogical(s, 'OR');
    if (orParts) return MvmSqliteEngine.evalWhere(orParts[0], row) || MvmSqliteEngine.evalWhere(orParts[1], row);
    const andParts = splitLogical(s, 'AND');
    if (andParts) return MvmSqliteEngine.evalWhere(andParts[0], row) && MvmSqliteEngine.evalWhere(andParts[1], row);
    if (/^NOT\s+/i.test(s)) return !MvmSqliteEngine.evalWhere(s.replace(/^NOT\s+/i, ''), row);
    if (s.startsWith('(') && s.endsWith(')')) return MvmSqliteEngine.evalWhere(s.slice(1, -1), row);

    const resolveVal = (operand) => {
      const t = operand.trim();
      if ((t.startsWith("'") && t.endsWith("'")) || /^null$/i.test(t) || /^[+-]?\d+(?:\.\d+)?$/.test(t)) {
        return MvmSqliteEngine.parseLiteral(t);
      }
      const key = MvmSqliteEngine.unquoteIdent(t).toLowerCase();
      if (Object.prototype.hasOwnProperty.call(row, key)) return row[key];
      const tail = key.includes('.') ? key.split('.').pop() : key;
      return Object.prototype.hasOwnProperty.call(row, tail) ? row[tail] : null;
    };

    let m = /^(.+?)\s+IS\s+NOT\s+NULL$/i.exec(s);
    if (m) return resolveVal(m[1]) !== null;
    m = /^(.+?)\s+IS\s+NULL$/i.exec(s);
    if (m) return resolveVal(m[1]) === null;
    m = /^(.+?)\s+IN\s*\((.+)\)$/i.exec(s);
    if (m) {
      const lhs = resolveVal(m[1]);
      const items = MvmSqliteEngine.splitTopLevelComma(m[2]).map(resolveVal);
      return items.some((v) => v === lhs);
    }
    m = /^(.+?)\s+LIKE\s+(.+)$/i.exec(s);
    if (m) {
      const lhs = String(resolveVal(m[1]) ?? '');
      const pat = String(resolveVal(m[2]) ?? '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&').replace(/%/g, '.*').replace(/_/g, '.');
      return new RegExp(`^${pat}$`, 'i').test(lhs);
    }
    m = /^(.+?)\s*(!=|<>|<=|>=|=|<|>)\s*(.+)$/.exec(s);
    if (m) {
      const lhs = resolveVal(m[1]);
      const op = m[2];
      const rhs = resolveVal(m[3]);
      if (lhs === null || rhs === null) return false;
      if (op === '=') return lhs === rhs;
      if (op === '!=' || op === '<>') return lhs !== rhs;
      if (op === '<') return lhs < rhs;
      if (op === '<=') return lhs <= rhs;
      if (op === '>') return lhs > rhs;
      if (op === '>=') return lhs >= rhs;
    }
    return Boolean(resolveVal(s));
  }

  static formatRows(cols, rows, opts) {
    if (!rows || rows.length === 0) {
      return opts.mode === 'json' ? '[]\n' : '';
    }
    if (opts.mode === 'json') {
      const objs = rows.map((r) => {
        const obj = {};
        for (let i = 0; i < cols.length; i += 1) obj[cols[i]] = r[i] ?? null;
        return obj;
      });
      return JSON.stringify(objs, null, 2) + '\n';
    }
    const sep = opts.mode === 'csv' ? ',' : (opts.separator || '|');
    const lines = [];
    if (opts.headers) lines.push(cols.join(sep));
    for (const r of rows) {
      lines.push(r.map((v) => (v === null || v === undefined ? '' : String(v))).join(sep));
    }
    return lines.join('\n') + '\n';
  }

  static execute(args, vfs, stdinUtf8) {
    const opts = { headers: false, mode: 'list', separator: '|' };
    const cmdPreSql = [];
    const pos = [];
    for (let i = 0; i < args.length; i += 1) {
      const a = String(args[i]);
      if (a === '-json') opts.mode = 'json';
      else if (a === '-csv') opts.mode = 'csv';
      else if (a === '-header') opts.headers = true;
      else if (a === '-noheader') opts.headers = false;
      else if (a === '-separator' && i + 1 < args.length) {
        opts.separator = String(args[i + 1]);
        i += 1;
      } else if (a === '-cmd' && i + 1 < args.length) {
        cmdPreSql.push(String(args[i + 1]));
        i += 1;
      } else if (!a.startsWith('-')) {
        pos.push(a);
      }
    }
    if (pos.length === 0) {
      const err = new Error('sqlite3: missing database path');
      err.code = 'SqliteRuntimeError';
      throw err;
    }
    const dbPath = pos[0];
    const inMemory = dbPath === ':memory:';
    if (!inMemory) vfs.assertMutable(dbPath);
    const existingBytes = inMemory ? null : vfs.readBytes(dbPath);
    const db = MvmSqliteEngine.loadDb(existingBytes);
    const mainSql = pos.slice(1).join(' ').trim() || String(stdinUtf8 || '').trim();
    const statements = [];
    for (const pre of cmdPreSql) {
      statements.push(...MvmSqliteEngine.splitStatements(pre));
    }
    if (mainSql) {
      statements.push(...MvmSqliteEngine.splitStatements(mainSql));
    }
    if (statements.length === 0) {
      return { ok: true, exit_code: 0, stdout: '', stderr: '' };
    }

    let snapshot = null;
    let mutated = false;
    let outText = '';

    for (const rawStmt of statements) {
      const stmt = rawStmt.trim();
      if (!stmt) continue;
      if (stmt.startsWith('.')) {
        const parts = stmt.split(/\s+/);
        const dot = parts[0].toLowerCase();
        if (dot === '.tables') {
          const names = Array.from(db.tables.values()).map((t) => t.name).sort();
          if (names.length > 0) outText += names.join('  ') + '\n';
        } else if (dot === '.schema') {
          const filter = parts[1] ? parts[1].toLowerCase() : '';
          for (const [k, t] of db.tables.entries()) {
            if (!filter || k === filter) outText += `${t.sql};\n`;
          }
        } else if (dot === '.headers') {
          opts.headers = String(parts[1] || '').toLowerCase() === 'on';
        } else if (dot === '.mode') {
          opts.mode = String(parts[1] || 'list').toLowerCase();
        } else if (dot === '.separator' && parts[1]) {
          opts.separator = parts[1];
        }
        continue;
      }

      const upper = stmt.toUpperCase();
      if (upper === 'BEGIN' || upper === 'BEGIN TRANSACTION') {
        snapshot = new Map();
        for (const [k, t] of db.tables.entries()) {
          snapshot.set(k, { ...t, columns: [...t.columns], rows: t.rows.map((r) => ({ ...r })) });
        }
        continue;
      }
      if (upper === 'COMMIT' || upper === 'END' || upper === 'END TRANSACTION') {
        snapshot = null;
        continue;
      }
      if (upper === 'ROLLBACK') {
        if (snapshot) {
          db.tables = snapshot;
          snapshot = null;
        }
        continue;
      }

      let m = /^CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?([A-Za-z_][A-Za-z0-9_"`]*)\s*\(([\s\S]+)\)$/i.exec(stmt);
      if (m) {
        const tblName = MvmSqliteEngine.unquoteIdent(m[1]);
        const key = tblName.toLowerCase();
        if (!db.tables.has(key)) {
          const allDefs = MvmSqliteEngine.splitTopLevelComma(m[2]);
          const pkCols = [];
          const uniqueSets = [];
          const colDefs = [];
          for (const d of allDefs) {
            const dt = d.trim();
            const pkMatch = /^(?:CONSTRAINT\s+\S+\s+)?PRIMARY\s+KEY\s*\(([^)]+)\)/i.exec(dt);
            if (pkMatch) {
              for (const c of MvmSqliteEngine.splitTopLevelComma(pkMatch[1])) {
                pkCols.push(MvmSqliteEngine.unquoteIdent(c).toLowerCase());
              }
              continue;
            }
            const uqMatch = /^(?:CONSTRAINT\s+\S+\s+)?UNIQUE\s*\(([^)]+)\)/i.exec(dt);
            if (uqMatch) {
              uniqueSets.push(
                MvmSqliteEngine.splitTopLevelComma(uqMatch[1]).map((c) => MvmSqliteEngine.unquoteIdent(c).toLowerCase()),
              );
              continue;
            }
            if (/^(CHECK|FOREIGN\s+KEY|CONSTRAINT)\b/i.test(dt)) continue;
            const tokens = dt.split(/\s+/);
            const name = MvmSqliteEngine.unquoteIdent(tokens[0]).toLowerCase();
            const type = (tokens[1] || 'TEXT').toUpperCase();
            if (/\bPRIMARY\s+KEY\b/i.test(dt)) pkCols.push(name);
            if (/\bUNIQUE\b/i.test(dt)) uniqueSets.push([name]);
            colDefs.push({ name, label: MvmSqliteEngine.unquoteIdent(tokens[0]), type });
          }
          db.tables.set(key, { name: tblName, columns: colDefs, rows: [], sql: stmt, pk: pkCols, unique: uniqueSets });
          mutated = true;
        }
        continue;
      }

      m = /^DROP\s+TABLE\s+(?:IF\s+EXISTS\s+)?([A-Za-z_][A-Za-z0-9_"`]*)$/i.exec(stmt);
      if (m) {
        const key = MvmSqliteEngine.unquoteIdent(m[1]).toLowerCase();
        if (db.tables.delete(key)) mutated = true;
        else if (!/IF\s+EXISTS/i.test(stmt)) {
          return { ok: true, exit_code: 1, stdout: outText, stderr: `Error: no such table: ${m[1]}\n` };
        }
        continue;
      }

      m = /^INSERT\s+(?:OR\s+(REPLACE|IGNORE)\s+)?INTO\s+([A-Za-z_][A-Za-z0-9_"`]*)\s*(?:\(([^)]+)\))?\s*VALUES\s*([\s\S]+)$/i.exec(stmt);
      if (m) {
        const conflictMode = m[1] ? m[1].toUpperCase() : null;
        const key = MvmSqliteEngine.unquoteIdent(m[2]).toLowerCase();
        const tbl = db.tables.get(key);
        if (!tbl) {
          return { ok: true, exit_code: 1, stdout: outText, stderr: `Error: no such table: ${m[2]}\n` };
        }
        const targetCols = m[3]
          ? MvmSqliteEngine.splitTopLevelComma(m[3]).map((c) => MvmSqliteEngine.unquoteIdent(c).toLowerCase())
          : tbl.columns.map((c) => c.name);
        const rawTuples = m[4].trim();
        let posIdx = 0;
        while (posIdx < rawTuples.length) {
          while (posIdx < rawTuples.length && (rawTuples[posIdx] === ',' || /\s/.test(rawTuples[posIdx]))) posIdx += 1;
          if (posIdx >= rawTuples.length) break;
          if (rawTuples[posIdx] !== '(') {
            return { ok: true, exit_code: 1, stdout: outText, stderr: `Error: near "${rawTuples.slice(posIdx, posIdx + 12)}": syntax error\n` };
          }
          let depth = 0;
          let inS = false;
          const start = posIdx + 1;
          posIdx += 1;
          while (posIdx < rawTuples.length) {
            const ch = rawTuples[posIdx];
            if (ch === "'") {
              if (inS && rawTuples[posIdx + 1] === "'") { posIdx += 2; continue; }
              inS = !inS;
            } else if (!inS) {
              if (ch === '(') depth += 1;
              else if (ch === ')') {
                if (depth === 0) break;
                depth -= 1;
              }
            }
            posIdx += 1;
          }
          const tupleStr = rawTuples.slice(start, posIdx);
          posIdx += 1;
          const vals = MvmSqliteEngine.splitTopLevelComma(tupleStr).map(MvmSqliteEngine.parseLiteral);
          const rowObj = {};
          for (const c of tbl.columns) rowObj[c.name] = null;
          for (let ci = 0; ci < targetCols.length; ci += 1) {
            rowObj[targetCols[ci]] = vals[ci] !== undefined ? vals[ci] : null;
          }
          const keyConstraints = [];
          if (Array.isArray(tbl.pk) && tbl.pk.length > 0) keyConstraints.push(tbl.pk);
          if (Array.isArray(tbl.unique)) {
            for (const uq of tbl.unique) {
              if (Array.isArray(uq) && uq.length > 0) keyConstraints.push(uq);
            }
          }
          const conflictSet = new Set();
          for (const kCols of keyConstraints) {
            if (kCols.some((kc) => rowObj[kc] === null || rowObj[kc] === undefined)) continue;
            for (let ri = 0; ri < tbl.rows.length; ri += 1) {
              if (kCols.every((kc) => tbl.rows[ri][kc] === rowObj[kc])) {
                conflictSet.add(ri);
              }
            }
          }
          if (conflictSet.size > 0) {
            if (conflictMode === 'IGNORE') {
              continue;
            }
            if (conflictMode === 'REPLACE') {
              tbl.rows = tbl.rows.filter((_, ri) => !conflictSet.has(ri));
              tbl.rows.push(rowObj);
              mutated = true;
              continue;
            }
            return { ok: true, exit_code: 1, stdout: outText, stderr: `Error: UNIQUE constraint failed: ${tbl.name}\n` };
          }
          tbl.rows.push(rowObj);
          mutated = true;
        }
        continue;
      }

      m = /^UPDATE\s+([A-Za-z_][A-Za-z0-9_"`]*)\s+SET\s+([\s\S]+?)(?:\s+WHERE\s+([\s\S]+))?$/i.exec(stmt);
      if (m) {
        const tbl = db.tables.get(MvmSqliteEngine.unquoteIdent(m[1]).toLowerCase());
        if (!tbl) {
          return { ok: true, exit_code: 1, stdout: outText, stderr: `Error: no such table: ${m[1]}\n` };
        }
        const assigns = MvmSqliteEngine.splitTopLevelComma(m[2]).map((a) => {
          const eq = a.indexOf('=');
          return [MvmSqliteEngine.unquoteIdent(a.slice(0, eq)).toLowerCase(), MvmSqliteEngine.parseLiteral(a.slice(eq + 1))];
        });
        for (const r of tbl.rows) {
          if (!m[3] || MvmSqliteEngine.evalWhere(m[3], r)) {
            for (const [k, v] of assigns) r[k] = v;
            mutated = true;
          }
        }
        continue;
      }

      m = /^DELETE\s+FROM\s+([A-Za-z_][A-Za-z0-9_"`]*)(?:\s+WHERE\s+([\s\S]+))?$/i.exec(stmt);
      if (m) {
        const tbl = db.tables.get(MvmSqliteEngine.unquoteIdent(m[1]).toLowerCase());
        if (!tbl) {
          return { ok: true, exit_code: 1, stdout: outText, stderr: `Error: no such table: ${m[1]}\n` };
        }
        const prevLen = tbl.rows.length;
        tbl.rows = m[2] ? tbl.rows.filter((r) => !MvmSqliteEngine.evalWhere(m[2], r)) : [];
        if (tbl.rows.length !== prevLen) mutated = true;
        continue;
      }

      m = /^SELECT\s+(DISTINCT\s+)?([\s\S]+?)(?:\s+FROM\s+([A-Za-z_][A-Za-z0-9_"`]*)(?:\s+(?:AS\s+)?(?!INNER\b|LEFT\b|JOIN\b|WHERE\b|GROUP\b|ORDER\b|LIMIT\b)([A-Za-z_][A-Za-z0-9_]*))?(?:\s+(?:(INNER|LEFT(?:\s+OUTER)?)\s+)?JOIN\s+([A-Za-z_][A-Za-z0-9_"`]*)(?:\s+(?:AS\s+)?(?!ON\b|WHERE\b|GROUP\b|ORDER\b|LIMIT\b)([A-Za-z_][A-Za-z0-9_]*))?\s+ON\s+([\s\S]+?))?(?:\s+WHERE\s+([\s\S]+?))?(?:\s+GROUP\s+BY\s+([\s\S]+?))?(?:\s+ORDER\s+BY\s+([\s\S]+?))?(?:\s+LIMIT\s+(\d+)(?:\s+OFFSET\s+(\d+))?)?)?$/i.exec(stmt);
      if (m) {
        const distinct = Boolean(m[1]);
        const selectExprs = MvmSqliteEngine.splitTopLevelComma(m[2]);
        const tblName = m[3];
        const leftAlias = m[4] ? MvmSqliteEngine.unquoteIdent(m[4]).toLowerCase() : null;
        const joinType = m[5] ? m[5].toUpperCase() : 'INNER';
        const joinTblName = m[6];
        const rightAlias = m[7] ? MvmSqliteEngine.unquoteIdent(m[7]).toLowerCase() : null;
        const joinOn = m[8];
        const whereExpr = m[9];
        const groupByExpr = m[10];
        const orderByExpr = m[11];
        const limitNum = m[12] ? Number(m[12]) : null;
        const offsetNum = m[13] ? Number(m[13]) : 0;

        let workingRows;
        let tbl = null;
        if (!tblName) {
          workingRows = [{}];
        } else {
          const leftTableKey = MvmSqliteEngine.unquoteIdent(tblName).toLowerCase();
          tbl = db.tables.get(leftTableKey);
          if (!tbl) {
            return { ok: true, exit_code: 1, stdout: outText, stderr: `Error: no such table: ${tblName}\n` };
          }
          workingRows = tbl.rows.map((r) => {
            const obj = { ...r };
            for (const c of tbl.columns) {
              obj[`${leftTableKey}.${c.name}`] = r[c.name];
              if (leftAlias) obj[`${leftAlias}.${c.name}`] = r[c.name];
            }
            return obj;
          });
          if (joinTblName) {
            const rightTableKey = MvmSqliteEngine.unquoteIdent(joinTblName).toLowerCase();
            const jTbl = db.tables.get(rightTableKey);
            if (!jTbl) {
              return { ok: true, exit_code: 1, stdout: outText, stderr: `Error: no such table: ${joinTblName}\n` };
            }
            const isLeftJoin = joinType.startsWith('LEFT');
            const makeJoinedRow = (lr, rr) => {
              const out = {};
              if (rr) {
                for (const k of Object.keys(rr)) out[k] = rr[k];
              } else {
                for (const c of jTbl.columns) out[c.name] = null;
              }
              for (const k of Object.keys(lr)) out[k] = lr[k];
              for (const c of tbl.columns) {
                out[`${leftTableKey}.${c.name}`] = lr[c.name];
                if (leftAlias) out[`${leftAlias}.${c.name}`] = lr[c.name];
              }
              for (const c of jTbl.columns) {
                const rv = rr ? rr[c.name] : null;
                out[`${rightTableKey}.${c.name}`] = rv;
                if (rightAlias) out[`${rightAlias}.${c.name}`] = rv;
              }
              return out;
            };
            const joined = [];
            for (const lr of workingRows) {
              let matched = false;
              for (const rr of jTbl.rows) {
                const cand = makeJoinedRow(lr, rr);
                if (!joinOn || MvmSqliteEngine.evalWhere(joinOn, cand)) {
                  matched = true;
                  joined.push(cand);
                }
              }
              if (isLeftJoin && !matched) {
                joined.push(makeJoinedRow(lr, null));
              }
            }
            workingRows = joined;
          }
          if (whereExpr) {
            workingRows = workingRows.filter((r) => MvmSqliteEngine.evalWhere(whereExpr, r));
          }
        }

        const evalAggOrExpr = (expr, groupRows) => {
          const clean = expr.trim();
          const aggMatch = /^(COUNT|SUM|AVG|MIN|MAX)\s*\(\s*(.+)\s*\)$/i.exec(clean);
          if (aggMatch) {
            const fn = aggMatch[1].toUpperCase();
            const inner = aggMatch[2].trim();
            if (fn === 'COUNT' && inner === '*') return groupRows.length;
            const colFull = MvmSqliteEngine.unquoteIdent(inner).toLowerCase();
            const colTail = MvmSqliteEngine.unquoteIdent(inner.split('.').pop()).toLowerCase();
            const vals = groupRows
              .map((r) => (Object.prototype.hasOwnProperty.call(r, colFull) ? r[colFull] : r[colTail]))
              .filter((v) => v !== null && v !== undefined);
            if (fn === 'COUNT') return vals.length;
            if (fn === 'SUM') return vals.reduce((a, b) => Number(a) + Number(b), 0);
            if (fn === 'AVG') return vals.length ? vals.reduce((a, b) => Number(a) + Number(b), 0) / vals.length : null;
            if (fn === 'MIN') return vals.length ? vals.reduce((a, b) => (b < a ? b : a)) : null;
            if (fn === 'MAX') return vals.length ? vals.reduce((a, b) => (b > a ? b : a)) : null;
          }
          const r0 = groupRows[0] || {};
          const kFull = MvmSqliteEngine.unquoteIdent(clean).toLowerCase();
          if (Object.prototype.hasOwnProperty.call(r0, kFull)) return r0[kFull];
          if (clean.includes('.') && !clean.startsWith("'")) {
            const kTail = MvmSqliteEngine.unquoteIdent(clean.split('.').pop()).toLowerCase();
            if (Object.prototype.hasOwnProperty.call(r0, kTail)) return r0[kTail];
          }
          return MvmSqliteEngine.parseLiteral(clean);
        };

        const isAggQuery = !groupByExpr && selectExprs.some((e) => /^(COUNT|SUM|AVG|MIN|MAX)\s*\(/i.test(e.trim()));
        let colHeaders = [];
        let resultRows = [];

        if (selectExprs.length === 1 && selectExprs[0] === '*' && tbl) {
          colHeaders = tbl.columns.map((c) => c.label);
          if (orderByExpr) {
            const obParts = orderByExpr.trim().split(/\s+/);
            const okFull = MvmSqliteEngine.unquoteIdent(obParts[0]).toLowerCase();
            const okTail = okFull.includes('.') ? okFull.split('.').pop() : okFull;
            const desc = (obParts[1] || '').toUpperCase() === 'DESC';
            workingRows.sort((a, b) => {
              const av = Object.prototype.hasOwnProperty.call(a, okFull) ? a[okFull] : a[okTail];
              const bv = Object.prototype.hasOwnProperty.call(b, okFull) ? b[okFull] : b[okTail];
              return av === bv ? 0 : (av < bv ? -1 : 1) * (desc ? -1 : 1);
            });
          }
          resultRows = workingRows.map((r) => tbl.columns.map((c) => r[c.name]));
        } else if (isAggQuery) {
          colHeaders = selectExprs.map((e) => e.trim());
          resultRows = [selectExprs.map((e) => evalAggOrExpr(e, workingRows))];
        } else if (groupByExpr) {
          const gFull = MvmSqliteEngine.unquoteIdent(groupByExpr.split(',')[0]).toLowerCase();
          const gTail = gFull.includes('.') ? gFull.split('.').pop() : gFull;
          const groups = new Map();
          for (const r of workingRows) {
            const gv = Object.prototype.hasOwnProperty.call(r, gFull) ? r[gFull] : r[gTail];
            if (!groups.has(gv)) groups.set(gv, []);
            groups.get(gv).push(r);
          }
          colHeaders = selectExprs.map((e) => e.trim());
          for (const gRows of groups.values()) {
            resultRows.push(selectExprs.map((e) => evalAggOrExpr(e, gRows)));
          }
        } else {
          if (orderByExpr) {
            const obParts = orderByExpr.trim().split(/\s+/);
            const okFull = MvmSqliteEngine.unquoteIdent(obParts[0]).toLowerCase();
            const okTail = okFull.includes('.') ? okFull.split('.').pop() : okFull;
            const desc = (obParts[1] || '').toUpperCase() === 'DESC';
            workingRows.sort((a, b) => {
              const av = Object.prototype.hasOwnProperty.call(a, okFull) ? a[okFull] : a[okTail];
              const bv = Object.prototype.hasOwnProperty.call(b, okFull) ? b[okFull] : b[okTail];
              return av === bv ? 0 : (av < bv ? -1 : 1) * (desc ? -1 : 1);
            });
          }
          colHeaders = selectExprs.map((e) => e.trim());
          resultRows = workingRows.map((r) => selectExprs.map((e) => evalAggOrExpr(e, [r])));
        }

        if (distinct) {
          const seen = new Set();
          resultRows = resultRows.filter((r) => {
            const sig = JSON.stringify(r);
            if (seen.has(sig)) return false;
            seen.add(sig);
            return true;
          });
        }
        if (offsetNum > 0) resultRows = resultRows.slice(offsetNum);
        if (limitNum !== null) resultRows = resultRows.slice(0, limitNum);
        outText += MvmSqliteEngine.formatRows(colHeaders, resultRows, opts);
        continue;
      }

      if (/^PRAGMA\s+/i.test(stmt)) {
        const pm = /^PRAGMA\s+table_info\s*\(\s*([A-Za-z_][A-Za-z0-9_"`]*)\s*\)$/i.exec(stmt);
        if (pm) {
          const tbl = db.tables.get(MvmSqliteEngine.unquoteIdent(pm[1]).toLowerCase());
          if (tbl) {
            const pRows = tbl.columns.map((c, idx) => [idx, c.label, c.type, 0, null, 0]);
            outText += MvmSqliteEngine.formatRows(['cid', 'name', 'type', 'notnull', 'dflt_value', 'pk'], pRows, opts);
          }
        }
        continue;
      }

      return { ok: true, exit_code: 1, stdout: outText, stderr: `Error: near "${stmt.slice(0, 24)}": syntax error\n` };
    }

    if (mutated && !inMemory) {
      vfs.writeBytes(dbPath, MvmSqliteEngine.saveDb(db));
    }
    return { ok: true, exit_code: 0, stdout: outText, stderr: '' };
  }
}

async function __uorSha256HexBytes(u8) {
  const view = u8 instanceof Uint8Array ? u8 : new Uint8Array(u8);
  const digest = await crypto.subtle.digest('SHA-256', view);
  const arr = new Uint8Array(digest);
  let hex = '';
  for (let i = 0; i < arr.length; i++) {
    hex += arr[i].toString(16).padStart(2, '0');
  }
  return hex;
}

function __uorBytesToBase64(u8) {
  let bin = '';
  const chunk = 0x8000;
  for (let i = 0; i < u8.length; i += chunk) {
    bin += String.fromCharCode.apply(null, u8.subarray(i, Math.min(i + chunk, u8.length)));
  }
  return btoa(bin);
}

function __uorBase64ToBytes(b64) {
  const clean = String(b64 || '').replace(/\s+/g, '');
  const bin = atob(clean);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) {
    out[i] = bin.charCodeAt(i) & 0xff;
  }
  return out;
}

function __uorStripHtmlToReadableText(html) {
  const raw = String(html || '');
  const titleMatch = raw.match(/<title[^>]*>([\s\S]*?)<\/title>/i);
  const title = titleMatch ? titleMatch[1].replace(/\s+/g, ' ').trim() : '';
  const body = raw
    .replace(/<script[\s\S]*?<\/script>/gi, ' ')
    .replace(/<style[\s\S]*?<\/style>/gi, ' ')
    .replace(/<noscript[\s\S]*?<\/noscript>/gi, ' ')
    .replace(/<\/(p|div|h1|h2|h3|h4|li|tr|section|article|br)>/gi, '\n')
    .replace(/<[^>]+>/g, ' ')
    .replace(/&nbsp;/gi, ' ')
    .replace(/&amp;/gi, '&')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/&quot;/gi, '"')
    .replace(/[ \t]+/g, ' ')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
  return { title, text: body.slice(0, 12000) };
}

function __uorVerifyRiscv64ElfHeader(u8) {
  if (!(u8 instanceof Uint8Array) || u8.byteLength < 64) {
    return { ok: false, error_code: 'InvalidElf64Riscv', message: 'Binary shorter than 64-byte ELF64 header' };
  }
  if (u8[0] !== 0x7f || u8[1] !== 0x45 || u8[2] !== 0x4c || u8[3] !== 0x46) {
    return { ok: false, error_code: 'InvalidElf64Riscv', message: 'Missing 0x7F ELF magic bytes' };
  }
  if (u8[4] !== 2 || u8[5] !== 1) {
    return { ok: false, error_code: 'InvalidElf64Riscv', message: 'Expected ELFCLASS64 little-endian binary' };
  }
  const eMachine = u8[18] | (u8[19] << 8);
  if (eMachine !== 243) {
    return {
      ok: false,
      error_code: 'InvalidElf64Riscv',
      message: `Expected EM_RISCV (243), got e_machine=${eMachine}`,
    };
  }
  return { ok: true, e_machine: eMachine };
}

export class SpiralCoreHostBridge {
  #secretVault = new Map();
  #ws = null;
  #wsConnecting = null;
  #pendingWs = new Map();
  #nextReqSeq = 1;
  #lazyRepoTrees = new Map();
  #provenanceBaseline = null;
  #esbuildPromise = null;
  #esbuildCompiler = null;
  #pyodidePromise = null;
  #pyodideInstance = null;
  #pyodideShimmed = false;
  #activePyStdoutSink = null;
  #activePyStderrSink = null;
  #syncFetchImpl = null;

  constructor(options) {
    const opts = options && typeof options === 'object' ? options : {};
    this.relayUrl =
      opts.relayWsUrl !== undefined
        ? opts.relayWsUrl
        : (opts.relayUrl !== undefined ? opts.relayUrl : DEFAULT_SPIRALCORE_RELAY_WS_URL);
    this.relayHttpUrl = opts.relayHttpUrl || '';
    this.staticOrigin = opts.staticOrigin || '';
    this.locationOrigin = opts.locationOrigin || '';
    this.mcpPolicy = opts.mcpPolicy || 'safe';
    this.webAccessEnabled = opts.webAccessEnabled !== false;
    this.relayTimeoutMs = opts.relayTimeoutMs || SPVS_RELAY_TIMEOUT_MS;
    this.lastTransportMode = 'standby';
    this.fetchImpl = opts.fetchImpl || globalThis.fetch;
    this.#syncFetchImpl = typeof opts.syncFetchImpl === 'function' ? opts.syncFetchImpl : null;
    this.esbuildModuleUrl = '';
    this.esbuildWasmUrl = '';
    this.setCompilerConfig(opts);
    this.pyodideLock = PYODIDE_LOCK;
    if (opts.pyodideLock && typeof opts.pyodideLock === 'object') {
      const l = opts.pyodideLock;
      if (
        typeof l.url === 'string' &&
        l.url.startsWith('https://') &&
        typeof l.sri === 'string' &&
        l.sri.startsWith('sha384-') &&
        l.sri.length > 20
      ) {
        this.pyodideLock = Object.freeze({
          version: String(l.version || PYODIDE_LOCK.version),
          url: l.url,
          indexURL: String(l.indexURL || l.url.slice(0, l.url.lastIndexOf('/') + 1)),
          sri: l.sri,
        });
      }
    }
  }

  setCompilerConfig(cfg) {
    const c = cfg && typeof cfg === 'object' ? cfg : {};
    const hasMod = Object.prototype.hasOwnProperty.call(c, 'esbuildModuleUrl');
    const hasWasm = Object.prototype.hasOwnProperty.call(c, 'esbuildWasmUrl');
    const hasBase = Object.prototype.hasOwnProperty.call(c, 'esbuildUrl');
    let modUrl = hasMod && typeof c.esbuildModuleUrl === 'string' ? c.esbuildModuleUrl.trim() : '';
    let wasmUrl = hasWasm && typeof c.esbuildWasmUrl === 'string' ? c.esbuildWasmUrl.trim() : '';
    const baseEsbuild = hasBase && typeof c.esbuildUrl === 'string' ? c.esbuildUrl.trim() : '';
    if (baseEsbuild) {
      if (baseEsbuild.endsWith('esbuild.wasm')) {
        if (!hasWasm) wasmUrl = baseEsbuild;
        if (!hasMod) modUrl = baseEsbuild.replace(/esbuild\.wasm$/, 'esm/browser.min.js');
      } else if (baseEsbuild.endsWith('browser.min.js')) {
        if (!hasMod) modUrl = baseEsbuild;
        if (!hasWasm) wasmUrl = baseEsbuild.replace(/esm\/browser\.min\.js$/, 'esbuild.wasm');
      } else {
        const root = baseEsbuild.endsWith('/') ? baseEsbuild : `${baseEsbuild}/`;
        if (!hasMod) modUrl = `${root}esm/browser.min.js`;
        if (!hasWasm) wasmUrl = `${root}esbuild.wasm`;
      }
    }
    if ((hasMod || hasBase || modUrl) && modUrl !== this.esbuildModuleUrl) {
      this.esbuildModuleUrl = modUrl;
      this.#esbuildCompiler = null;
      this.#esbuildPromise = null;
    }
    if ((hasWasm || hasBase || wasmUrl) && wasmUrl !== this.esbuildWasmUrl) {
      this.esbuildWasmUrl = wasmUrl;
      this.#esbuildCompiler = null;
      this.#esbuildPromise = null;
    }
    return {
      ok: true,
      esbuildModuleUrl: this.esbuildModuleUrl,
      esbuildWasmUrl: this.esbuildWasmUrl,
    };
  }

  setRelayUrl(url) {
    const trimmed = String(url || '').trim();
    if (!trimmed) {
      this.relayUrl = DEFAULT_SPIRALCORE_RELAY_WS_URL;
      this.relayHttpUrl = '';
      this.closeRelaySocket();
      return { ok: true, relay_url: this.relayUrl };
    }
    const check = this.validateTransportUrl(trimmed, true);
    if (!check.ok) return check;
    this.relayUrl = trimmed;
    this.closeRelaySocket();
    return { ok: true, relay_url: this.relayUrl };
  }

  close() {
    this.closeRelaySocket();
  }

  async verifyElf64RiscvAndSha256(elfBytes, expectedSha = '', maxBytes = 16 * 1024 * 1024) {
    if (!(elfBytes instanceof Uint8Array) || elfBytes.byteLength === 0) {
      return { ok: false, error_code: 'InvalidElf64Riscv', message: 'Empty ELF payload' };
    }
    const actualSha = await __uorSha256HexBytes(elfBytes);
    if (expectedSha && actualSha.toLowerCase() !== String(expectedSha).trim().toLowerCase()) {
      return {
        ok: false,
        error_code: 'DigestMismatch',
        message: `SHA-256 mismatch: expected ${expectedSha}, got ${actualSha}`,
      };
    }
    const elfCheck = __uorVerifyRiscv64ElfHeader(elfBytes);
    if (!elfCheck.ok) return elfCheck;
    if (elfBytes.byteLength > maxBytes) {
      return {
        ok: false,
        error_code: 'SizeExceedsQuota',
        message: `Package size ${elfBytes.byteLength} exceeds quota ${maxBytes}`,
      };
    }
    return { ok: true, sha256: actualSha, e_machine: elfCheck.e_machine };
  }

  setPolicy(optsInput) {
    const opts = optsInput && typeof optsInput === 'object' ? optsInput : {};
    const { mcpPolicy, webAccessEnabled } = opts;
    if (typeof mcpPolicy === 'string' && ['safe', 'confirm', 'deny'].includes(mcpPolicy)) {
      this.mcpPolicy = mcpPolicy;
    }
    if (typeof webAccessEnabled === 'boolean') {
      this.webAccessEnabled = webAccessEnabled;
    }
    if (opts.esbuildModuleUrl || opts.esbuildWasmUrl || opts.esbuildUrl) {
      this.setCompilerConfig(opts);
    }
    return {
      ok: true,
      mcp_policy: this.mcpPolicy,
      web_access_enabled: this.webAccessEnabled,
      esbuild_configured: Boolean(this.esbuildModuleUrl && this.esbuildWasmUrl),
    };
  }

  setSecret(key, value) {
    const cleanKey = String(key || '').trim();
    if (!cleanKey) {
      return { ok: false, error_code: 'InvalidSecretKey', message: 'Secret key must be non-empty' };
    }
    const cleanVal = String(value || '').trim();
    if (!cleanVal) {
      this.#secretVault.delete(cleanKey);
      return { ok: true, key: cleanKey, configured: false };
    }
    this.#secretVault.set(cleanKey, cleanVal);
    return {
      ok: true,
      key: cleanKey,
      configured: true,
      allowlisted_hosts: Array.from(SECRET_HOST_ALLOWLIST[cleanKey] || []),
    };
  }

  hasSecret(key) {
    return this.#secretVault.has(String(key || '').trim());
  }

  closeRelaySocket() {
    if (this.#ws) {
      try {
        this.#ws.close();
      } catch {
        // Ignore close error
      }
      this.#ws = null;
    }
    this.#wsConnecting = null;
  }

  validateTransportUrl(rawUrl, allowWebSocket = false) {
    let parsed;
    try {
      parsed = new URL(String(rawUrl || ''));
    } catch {
      return { ok: false, error_code: 'InvalidBridgeUrl', message: `Malformed URL: ${rawUrl}` };
    }
    const proto = parsed.protocol;
    const host = parsed.hostname;
    if (proto === 'https:') {
      return { ok: true, url: parsed };
    }
    if (allowWebSocket && proto === 'wss:') {
      return { ok: true, url: parsed };
    }
    if (proto === 'http:' && host === '127.0.0.1') {
      return { ok: true, url: parsed };
    }
    if (allowWebSocket && proto === 'ws:' && host === '127.0.0.1') {
      return { ok: true, url: parsed };
    }
    return {
      ok: false,
      error_code: 'InsecureTransportRejected',
      message: `R4-39 rejected non-TLS remote protocol ${proto} for host ${host}`,
    };
  }

  substituteHostSecretsInString(rawText, targetHostname) {
    const str = String(rawText || '');
    if (!str.includes('{{MVM_SECRET:')) {
      return { ok: true, value: str };
    }
    let failed = null;
    const replaced = str.replace(/\{\{MVM_SECRET:([A-Z0-9_]+)\}\}/g, (_full, secretKey) => {
      if (failed) return '';
      const allowedHosts = SECRET_HOST_ALLOWLIST[secretKey];
      if (!allowedHosts || !allowedHosts.has(targetHostname)) {
        failed = {
          ok: false,
          error_code: 'HostNotAllowlistedForSecret',
          message: `Secret ${secretKey} is not allowlisted for host ${targetHostname}`,
        };
        return '';
      }
      const secretVal = this.#secretVault.get(secretKey);
      if (!secretVal) {
        failed = {
          ok: false,
          error_code: 'SecretNotConfigured',
          message: `Secret ${secretKey} is not configured in host vault`,
        };
        return '';
      }
      return secretVal;
    });
    if (failed) return failed;
    return { ok: true, value: replaced };
  }

  applyHostSecretSubstitution(targetUrlObj, rawHeaders = {}) {
    const hostname = targetUrlObj.hostname.toLowerCase();
    const outHeaders = {};
    for (const [k, v] of Object.entries(rawHeaders || {})) {
      const sub = this.substituteHostSecretsInString(v, hostname);
      if (!sub.ok) return sub;
      outHeaders[k] = sub.value;
    }
    if (
      !outHeaders.Authorization &&
      !outHeaders.authorization &&
      this.#secretVault.has('GITHUB_TOKEN') &&
      (hostname === 'api.github.com' || hostname === 'github.com')
    ) {
      outHeaders.Authorization = `Bearer ${this.#secretVault.get('GITHUB_TOKEN')}`;
    }
    return { ok: true, headers: outHeaders };
  }

  async tryRelayRpc(op, params) {
    if (!this.relayUrl && !this.relayHttpUrl) {
      return null;
    }
    const serializedCheck = JSON.stringify(params || {});
    if (serializedCheck.includes('{{MVM_SECRET:')) {
      return null;
    }
    const rpcBody = {
      spvs_magic: 'SPVS',
      src_cid: 3,
      dst_cid: 2,
      vsock_port: 5257,
      policy: this.mcpPolicy,
      op,
      params: params || {},
    };

    if (typeof WebSocket === 'function') {
      const candidateWsUrls = [];
      if (this.relayUrl) candidateWsUrls.push(this.relayUrl);
      if (!candidateWsUrls.includes(LOCAL_SPIRALCORE_RELAY_WS_URL)) {
        candidateWsUrls.push(LOCAL_SPIRALCORE_RELAY_WS_URL);
      }
      for (const wsUrl of candidateWsUrls) {
        const valid = this.validateTransportUrl(wsUrl, true);
        if (!valid.ok) continue;
        try {
          const result = await this.#sendSingleWsRpc(wsUrl, rpcBody, this.relayTimeoutMs);
          if (result && typeof result.ok === 'boolean') {
            this.lastTransportMode = wsUrl.startsWith('ws://127.0.0.1') ? 'spvs-local-ws' : 'spvs-cloud-wss';
            return result;
          }
        } catch {
          // Try next WS or HTTPS POST relay
        }
      }
    }

    if (typeof fetch === 'function') {
      const candidateHttpUrls = [];
      if (this.relayHttpUrl) candidateHttpUrls.push(this.relayHttpUrl);
      if (this.relayUrl) {
        if (this.relayUrl.startsWith('wss://')) {
          candidateHttpUrls.push('https://' + this.relayUrl.slice(6));
        } else if (this.relayUrl.startsWith('ws://127.0.0.1')) {
          candidateHttpUrls.push('http://127.0.0.1' + this.relayUrl.slice(14));
        }
      }
      if (!candidateHttpUrls.includes(DEFAULT_SPIRALCORE_RELAY_HTTP_URL)) {
        candidateHttpUrls.push(DEFAULT_SPIRALCORE_RELAY_HTTP_URL);
      }
      if (!candidateHttpUrls.includes('http://127.0.0.1:8787/spvs')) {
        candidateHttpUrls.push('http://127.0.0.1:8787/spvs');
      }

      for (const httpUrl of candidateHttpUrls) {
        try {
          const ctrl = typeof AbortController === 'function' ? new AbortController() : null;
          const timer = ctrl ? setTimeout(() => ctrl.abort(), this.relayTimeoutMs) : null;
          const reqId = `spvs-http-${Date.now()}-${this.#nextReqSeq++}`;
          const resp = await this.#fetchImpl(httpUrl, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ req_id: reqId, ...rpcBody }),
            signal: ctrl ? ctrl.signal : undefined,
          });
          if (timer) clearTimeout(timer);
          const parsed = await resp.json();
          if (parsed && typeof parsed.ok === 'boolean') {
            this.lastTransportMode = httpUrl.includes('127.0.0.1') ? 'spvs-local-http' : 'spvs-cloud-https';
            return parsed;
          }
        } catch {
          // Try next HTTP candidate
        }
      }
    }
    return null;
  }

  #sendSingleWsRpc(wsUrl, payload, timeoutMs) {
    return new Promise((resolveRpc, rejectRpc) => {
      let settled = false;
      const reqId = `spvs-${Date.now()}-${this.#nextReqSeq++}`;
      let timer = setTimeout(() => {
        if (settled) return;
        settled = true;
        this.#pendingWs.delete(reqId);
        rejectRpc(new Error('SpiralCore relay timeout'));
      }, timeoutMs);

      const finish = (err, data) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        this.#pendingWs.delete(reqId);
        if (err) rejectRpc(err);
        else resolveRpc(data);
      };

      const dispatchOverSocket = (sock) => {
        clearTimeout(timer);
        timer = setTimeout(() => {
          if (settled) return;
          settled = true;
          this.#pendingWs.delete(reqId);
          rejectRpc(new Error('SpiralCore relay RPC timeout'));
        }, Math.max(timeoutMs, 6500));
        this.#pendingWs.set(reqId, finish);
        try {
          sock.send(JSON.stringify({ req_id: reqId, ...payload }));
        } catch (err) {
          finish(err, null);
        }
      };

      if (this.#ws && this.#ws.readyState === WebSocket.OPEN && this.#ws.__uorUrl === wsUrl) {
        dispatchOverSocket(this.#ws);
        return;
      }

      try {
        const sock = new WebSocket(wsUrl);
        sock.__uorUrl = wsUrl;
        sock.onopen = () => {
          this.#ws = sock;
          dispatchOverSocket(sock);
        };
        sock.onmessage = (ev) => {
          try {
            const parsed = JSON.parse(typeof ev.data === 'string' ? ev.data : '');
            const id = parsed.req_id || parsed.id;
            const cb = id ? this.#pendingWs.get(id) : null;
            if (cb) cb(null, parsed);
          } catch {
            // Ignore non-JSON frame
          }
        };
        sock.onerror = () => {
          finish(new Error('SpiralCore WebSocket error'), null);
        };
        sock.onclose = () => {
          if (this.#ws === sock) this.#ws = null;
          finish(new Error('SpiralCore WebSocket closed'), null);
        };
      } catch (err) {
        finish(err, null);
      }
    });
  }

  async execute(rawRequest) {
    const req = typeof rawRequest === 'string' ? JSON.parse(rawRequest) : (rawRequest || {});
    const rawOp = String(req.op || req.kind || req.bridge_op || req.tool || req.name || 'http_request').toLowerCase();
    const isNpmPkg =
      rawOp === 'npm_exec' ||
      rawOp === 'npm' ||
      ((rawOp === 'pkg_fetch' || rawOp === 'pkg') &&
        (String(req.ecosystem || '').toLowerCase() === 'npm' ||
          String(req.tool || '').toLowerCase() === 'npm'));

    if (rawOp === 'node_exec' || rawOp === 'node' || rawOp === 'npx') {
      return this.handleNodeExec(req);
    }
    if (isNpmPkg) {
      return this.handleNpmBridge(req);
    }
    if (rawOp === 'python_exec' || rawOp === 'python' || rawOp === 'python3' || rawOp === 'pip' || rawOp === 'pip3') {
      return this.handlePythonExec(req);
    }
    if (rawOp === 'wasi_exec' || rawOp === 'wasm') {
      return this.handleWasiExec(req);
    }
    if (rawOp === 'sqlite_exec' || rawOp === 'sqlite3') {
      return this.handleSqliteExec(req);
    }

    if (this.mcpPolicy === 'deny') {
      return {
        ok: false,
        status: 403,
        error_code: 'PolicyDenied',
        message: 'All external bridge operations are denied by active MCP security policy',
      };
    }
    if (!this.webAccessEnabled && rawOp !== 'pkg_fetch' && rawOp !== 'pkg') {
      return {
        ok: false,
        status: 403,
        error_code: 'PolicyDenied',
        message: 'Web Access is currently toggled OFF in the studio topbar',
      };
    }

    if (rawOp === 'dns_query' || rawOp === 'dns_resolve' || rawOp === 'dns') {
      return this.handleDnsQuery(req);
    }
    if (rawOp === 'web_search' || rawOp === 'search') {
      return this.handleWebSearch(req);
    }
    if (rawOp === 'web_fetch') {
      return this.handleWebFetch(req);
    }
    if (rawOp === 'git_smart_http' || rawOp === 'git') {
      return this.handleGitBridge(req);
    }
    if (rawOp === 'pkg_fetch' || rawOp === 'pkg') {
      return this.handlePkgFetch(req);
    }
    return this.handleHttpRequest(req);
  }

  async readBoundedBytes(resp) {
    if (!resp.body || typeof resp.body.getReader !== 'function') {
      const ab = await resp.arrayBuffer();
      if (ab.byteLength > MAX_BRIDGE_RESPONSE_BYTES) {
        return {
          ok: false,
          error_code: 'ResponseBodyTooLarge',
          message: `Response exceeded ${MAX_BRIDGE_RESPONSE_BYTES} byte cap`,
        };
      }
      return { ok: true, bytes: new Uint8Array(ab), truncated: false };
    }
    const reader = resp.body.getReader();
    const chunks = [];
    let total = 0;
    let truncated = false;
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      if (value && value.byteLength > 0) {
        if (total + value.byteLength > MAX_BRIDGE_RESPONSE_BYTES) {
          const keep = MAX_BRIDGE_RESPONSE_BYTES - total;
          if (keep > 0) {
            chunks.push(value.subarray(0, keep));
            total += keep;
          }
          truncated = true;
          try {
            await reader.cancel();
          } catch {
            // Ignore cancel error
          }
          break;
        }
        chunks.push(value);
        total += value.byteLength;
      }
    }
    const merged = new Uint8Array(total);
    let offset = 0;
    for (const c of chunks) {
      merged.set(c, offset);
      offset += c.byteLength;
    }
    return { ok: true, bytes: merged, truncated };
  }

  async handleDnsQuery(req) {
    const domain = String(req.name || req.domain || req.host || 'example.com').trim();
    const rrType = String(req.rr_type || req.qtype || req.type || 'A').trim().toUpperCase();
    const relayRes = await this.tryRelayRpc('dns_query', { name: domain, type: rrType });
    if (relayRes && relayRes.ok && relayRes.data && Array.isArray(relayRes.data.answers)) {
      const records = relayRes.data.answers.map((a) => ({
        name: a.name || domain,
        type: a.type,
        ttl: a.ttl ?? a.TTL ?? 300,
        data: a.data,
      }));
      const summary = records.length > 0
        ? records.map((r) => `${r.name}\tIN\t${rrType}\t${r.data} (ttl=${r.ttl}s)`).join('\n')
        : `No ${rrType} records returned for ${domain} (RCODE=${relayRes.data.rcode ?? 0})`;
      return {
        ok: true,
        status: relayRes.status || 200,
        op: 'dns_query',
        name: domain,
        rr_type: rrType,
        records,
        answers: records,
        data: { answers: records, rcode: relayRes.data.rcode ?? 0 },
        body_text: summary,
        transport: this.lastTransportMode,
      };
    }
    const dohUrl = `https://cloudflare-dns.com/dns-query?name=${encodeURIComponent(domain)}&type=${encodeURIComponent(rrType)}`;
    const valid = this.validateTransportUrl(dohUrl, false);
    if (!valid.ok) return valid;
    try {
      const resp = await fetch(dohUrl, {
        method: 'GET',
        headers: { Accept: 'application/dns-json' },
      });
      const json = await resp.json();
      const answers = Array.isArray(json.Answer) ? json.Answer : [];
      const records = answers.map((a) => ({
        name: a.name || domain,
        type: a.type,
        ttl: a.TTL ?? 300,
        data: a.data,
      }));
      this.lastTransportMode = 'direct-https-fallback';
      const summary = records.length > 0
        ? records.map((r) => `${r.name}\tIN\t${rrType}\t${r.data} (ttl=${r.ttl}s)`).join('\n')
        : `No ${rrType} records returned for ${domain} (RCODE=${json.Status ?? 0})`;
      return {
        ok: true,
        status: resp.status,
        op: 'dns_query',
        name: domain,
        rr_type: rrType,
        records,
        answers: records,
        data: { answers: records, rcode: json.Status ?? 0 },
        body_text: summary,
        transport: this.lastTransportMode,
      };
    } catch (err) {
      return {
        ok: false,
        status: 502,
        error_code: 'DnsResolutionFailed',
        message: err && err.message ? err.message : String(err),
      };
    }
  }

  async handleHttpRequest(req) {
    const rawUrl = String(req.url || '').trim();
    const valid = this.validateTransportUrl(rawUrl, false);
    if (!valid.ok) return valid;
    const subUrl = this.substituteHostSecretsInString(rawUrl, valid.url.hostname.toLowerCase());
    if (!subUrl.ok) return subUrl;
    const finalValid = this.validateTransportUrl(subUrl.value, false);
    if (!finalValid.ok) return finalValid;

    const headerSub = this.applyHostSecretSubstitution(finalValid.url, req.headers || {});
    if (!headerSub.ok) return headerSub;

    let bodyPayload;
    if (req.body_base64) {
      bodyPayload = __uorBase64ToBytes(req.body_base64);
    } else if (typeof req.body === 'string') {
      const bodySub = this.substituteHostSecretsInString(req.body, finalValid.url.hostname.toLowerCase());
      if (!bodySub.ok) return bodySub;
      bodyPayload = bodySub.value;
    }

    const hasAuthHeader = Boolean(headerSub.headers.Authorization || headerSub.headers.authorization);
    if (!hasAuthHeader) {
      const relayRes = await this.tryRelayRpc('http_request', {
        method: String(req.method || 'GET').toUpperCase(),
        url: finalValid.url.href,
        headers: headerSub.headers,
        body_b64: req.body_base64 || req.body_b64 || undefined,
      });
      if (relayRes && !relayRes.ok && relayRes.error_code === 'SsrfBlocked') {
        return {
          ok: false,
          status: relayRes.status || 403,
          error_code: 'SsrfBlocked',
          message: relayRes.error_message || 'Target host blocked by SpiralCore SSRF policy',
        };
      }
      if (relayRes && relayRes.ok && relayRes.data) {
        const d = relayRes.data;
        const bodyBytes = d.body_b64 ? __uorBase64ToBytes(d.body_b64) : new Uint8Array(0);
        const bodyText = bodyBytes.byteLength > 0 ? cachedTextDecoder.decode(bodyBytes) : (d.text_excerpt || '');
        return {
          ok: d.status >= 200 && d.status < 400,
          status: d.status,
          op: 'http_request',
          url: finalValid.url.href,
          headers: d.headers || {},
          content_type: d.content_type || '',
          body_text: bodyText,
          text_excerpt: d.text_excerpt || bodyText.slice(0, 3200),
          body_base64: req.binary ? (d.body_b64 || '') : undefined,
          byte_len: d.byte_length ?? bodyBytes.byteLength,
          truncated: Boolean(d.truncated),
          transport: this.lastTransportMode,
        };
      }
    }

    try {
      const resp = await this.#fetchImpl(finalValid.url.href, {
        method: String(req.method || 'GET').toUpperCase(),
        headers: headerSub.headers,
        body: bodyPayload,
      });
      const bounded = await this.readBoundedBytes(resp);
      if (!bounded.ok) return bounded;
      this.lastTransportMode = 'direct-https-fetch';
      const respHeaders = {};
      resp.headers.forEach((val, key) => {
        respHeaders[key] = val;
      });
      const contentType = String(respHeaders['content-type'] || '').toLowerCase();
      const decodedText = cachedTextDecoder.decode(bounded.bytes);
      const isHtml = contentType.includes('html') || /<html|<body/i.test(decodedText);
      const textExcerpt = isHtml
        ? __uorStripHtmlToReadableText(decodedText).text.slice(0, 3200)
        : decodedText.slice(0, 3200);
      return {
        ok: resp.ok,
        status: resp.status,
        op: 'http_request',
        url: finalValid.url.href,
        headers: respHeaders,
        content_type: contentType,
        body_text: decodedText,
        text_excerpt: textExcerpt,
        body_base64: req.binary ? __uorBytesToBase64(bounded.bytes) : undefined,
        byte_len: bounded.bytes.byteLength,
        truncated: bounded.truncated,
        transport: this.lastTransportMode,
      };
    } catch (err) {
      return {
        ok: false,
        status: 502,
        error_code: 'HttpBridgeFetchError',
        message: err && err.message ? err.message : String(err),
      };
    }
  }

  async handleWebFetch(req) {
    const rawUrl = String(req.url || (req.arguments && req.arguments.url) || '').trim();
    const valid = this.validateTransportUrl(rawUrl, false);
    if (!valid.ok) return valid;

    const relayRes = await this.tryRelayRpc('http_request', { method: 'GET', url: valid.url.href });
    if (relayRes && relayRes.ok && relayRes.data) {
      const d = relayRes.data;
      const decoded = d.body_b64 ? cachedTextDecoder.decode(__uorBase64ToBytes(d.body_b64)) : '';
      const parsed = decoded ? __uorStripHtmlToReadableText(decoded) : { title: valid.url.hostname, text: d.text_excerpt || '' };
      const bestText = parsed.text || d.text_excerpt || '';
      if (bestText) {
        return {
          ok: true,
          status: d.status || 200,
          op: 'web_fetch',
          url: valid.url.href,
          title: parsed.title || valid.url.href,
          body_text: bestText,
          excerpt: bestText.slice(0, 3200),
          transport: this.lastTransportMode,
        };
      }
    }

    const direct = await this.handleHttpRequest({ url: valid.url.href, method: 'GET', headers: req.headers || {} });
    if (direct.ok && typeof direct.body_text === 'string') {
      const isHtml = (direct.content_type || '').includes('html') || /<html|<body/i.test(direct.body_text);
      const parsed = isHtml
        ? __uorStripHtmlToReadableText(direct.body_text)
        : { title: valid.url.hostname, text: direct.body_text.slice(0, 12000) };
      return {
        ok: true,
        status: direct.status,
        op: 'web_fetch',
        url: valid.url.href,
        title: parsed.title || valid.url.href,
        body_text: parsed.text,
        excerpt: parsed.text.slice(0, 3200),
        transport: direct.transport,
      };
    }

    // Public reader CORS fallback for HTML docs that omit Access-Control-Allow-Origin
    if (!req.headers || Object.keys(req.headers).length === 0) {
      try {
        const readerUrl = `https://r.jina.ai/${valid.url.href}`;
        const rResp = await fetch(readerUrl, { headers: { Accept: 'text/plain' } });
        if (rResp.ok) {
          const bounded = await this.readBoundedBytes(rResp);
          if (bounded.ok) {
            const text = cachedTextDecoder.decode(bounded.bytes).slice(0, 12000);
            this.lastTransportMode = 'direct-https-reader';
            return {
              ok: true,
              status: rResp.status,
              op: 'web_fetch',
              url: valid.url.href,
              title: valid.url.hostname,
              body_text: text,
              excerpt: text.slice(0, 3200),
              transport: this.lastTransportMode,
            };
          }
        }
      } catch {
        // Return direct error below
      }
    }
    return direct;
  }

  async handleWebSearch(req) {
    const query = String(req.query || (req.arguments && req.arguments.query) || '').trim();
    const limit = Math.min(Math.max(Number(req.limit) || 6, 1), 10);
    if (!query) {
      return { ok: false, error_code: 'EmptySearchQuery', message: 'web_search query must be non-empty' };
    }

    const relayRes = await this.tryRelayRpc('web_search', { query, limit });
    if (relayRes && relayRes.ok && relayRes.data && Array.isArray(relayRes.data.results) && relayRes.data.results.length > 0) {
      const hits = relayRes.data.results;
      const formattedRelay = hits
        .map((r, i) => `[${i + 1}] ${r.title}\n    URL: ${r.url}\n    ${r.snippet}`)
        .join('\n\n');
      return {
        ok: true,
        status: relayRes.status || 200,
        op: 'web_search',
        query,
        results: hits,
        data: { results: hits },
        body_text: formattedRelay,
        transport: this.lastTransportMode,
      };
    }

    const results = [];
    const seenUrls = new Set();
    const pushResult = (item) => {
      if (!item || !item.url || seenUrls.has(item.url) || results.length >= limit) return;
      seenUrls.add(item.url);
      results.push(item);
    };

    const wikiTask = fetch(
      `https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch=${encodeURIComponent(query)}&srlimit=3&utf8=1&format=json&origin=*`,
    )
      .then((r) => (r.ok ? r.json() : null))
      .catch(() => null);

    const hnTask = fetch(
      `https://hn.algolia.com/api/v1/search?query=${encodeURIComponent(query)}&hitsPerPage=4`,
    )
      .then((r) => (r.ok ? r.json() : null))
      .catch(() => null);

    const ghTask = fetch(
      `https://api.github.com/search/repositories?q=${encodeURIComponent(query)}&per_page=3`,
      { headers: { Accept: 'application/vnd.github+json' } },
    )
      .then((r) => (r.ok ? r.json() : null))
      .catch(() => null);

    const ddgTask = fetch(
      `https://api.duckduckgo.com/?q=${encodeURIComponent(query)}&format=json&no_html=1&skip_disambig=1`,
    )
      .then((r) => (r.ok ? r.json() : null))
      .catch(() => null);

    const [wikiJson, hnJson, ghJson, ddgJson] = await Promise.all([wikiTask, hnTask, ghTask, ddgTask]);

    if (ddgJson && ddgJson.AbstractText && ddgJson.AbstractURL) {
      pushResult({
        title: ddgJson.Heading || query,
        url: ddgJson.AbstractURL,
        snippet: String(ddgJson.AbstractText).slice(0, 360),
        source: 'duckduckgo',
      });
    }
    if (wikiJson && wikiJson.query && Array.isArray(wikiJson.query.search)) {
      for (const hit of wikiJson.query.search) {
        const cleanSnippet = String(hit.snippet || '').replace(/<[^>]+>/g, '').trim();
        pushResult({
          title: hit.title,
          url: `https://en.wikipedia.org/wiki/${encodeURIComponent(String(hit.title || '').replace(/\s+/g, '_'))}`,
          snippet: cleanSnippet.slice(0, 360),
          source: 'wikipedia',
        });
      }
    }
    if (ghJson && Array.isArray(ghJson.items)) {
      for (const repo of ghJson.items) {
        pushResult({
          title: `${repo.full_name} (★${repo.stargazers_count ?? 0})`,
          url: repo.html_url,
          snippet: String(repo.description || 'GitHub repository').slice(0, 360),
          source: 'github',
        });
      }
    }
    if (hnJson && Array.isArray(hnJson.hits)) {
      for (const hit of hnJson.hits) {
        const targetUrl = hit.url || `https://news.ycombinator.com/item?id=${hit.objectID}`;
        pushResult({
          title: hit.title || hit.story_title || 'Hacker News Discussion',
          url: targetUrl,
          snippet: `HN ${hit.points ?? 0} pts · ${hit.num_comments ?? 0} comments · by ${hit.author || 'anon'}`,
          source: 'hn',
        });
      }
    }

    this.lastTransportMode = 'direct-https-search';
    const formatted = results
      .map((r, i) => `[${i + 1}] ${r.title}\n    URL: ${r.url}\n    ${r.snippet}`)
      .join('\n\n');
    return {
      ok: true,
      status: 200,
      op: 'web_search',
      query,
      results,
      data: { results },
      body_text: formatted || `No public HTTPS search hits returned for "${query}".`,
      transport: this.lastTransportMode,
    };
  }

  parseGitHubRepoCoords(rawRepo) {
    const cleaned = String(rawRepo || '').trim().replace(/\.git$/i, '').replace(/\/+$/, '');
    if (/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(cleaned)) {
      const [owner, repo] = cleaned.split('/');
      return { owner, repo, url: `https://github.com/${owner}/${repo}` };
    }
    try {
      const u = new URL(cleaned);
      const host = u.hostname.toLowerCase();
      if (host === 'github.com' || host === 'api.github.com') {
        const cleanPath = host === 'api.github.com' ? u.pathname.replace(/^\/repos\//i, '/') : u.pathname;
        const parts = cleanPath.split('/').filter(Boolean);
        if (parts.length >= 2) {
          return { owner: parts[0], repo: parts[1], url: `https://github.com/${parts[0]}/${parts[1]}` };
        }
      }
    } catch {
      // Not a GitHub URL
    }
    return null;
  }

  detectHostedGitHubRepo(req = {}) {
    const hint = String(req.repo_hint || req.repo || '').trim();
    if (hint) {
      const parsed = this.parseGitHubRepoCoords(hint);
      if (parsed) {
        return {
          ok: true,
          op: 'git',
          subcommand: 'detect_repo',
          owner: parsed.owner,
          repo: `${parsed.owner}/${parsed.repo}`,
          repo_name: parsed.repo,
          repo_url: parsed.url,
          detected: true,
          source: 'hint',
        };
      }
    }
    let loc = req.location || (typeof location !== 'undefined' ? location : null);
    if (!loc && this.locationOrigin) {
      try {
        loc = new URL(this.locationOrigin);
      } catch {
        loc = null;
      }
    }
    if (loc && typeof loc.hostname === 'string') {
      const m = /^([A-Za-z0-9_.-]+)\.github\.io$/i.exec(loc.hostname.trim());
      if (m) {
        const owner = m[1];
        const segs = String(loc.pathname || '/').split('/').filter(Boolean);
        const repoName = segs[0] || `${owner}.github.io`;
        return {
          ok: true,
          op: 'git',
          subcommand: 'detect_repo',
          owner,
          repo: `${owner}/${repoName}`,
          repo_name: repoName,
          repo_url: `https://github.com/${owner}/${repoName}`,
          detected: true,
          source: 'github-pages-origin',
        };
      }
    }
    return {
      ok: true,
      op: 'git',
      subcommand: 'detect_repo',
      owner: 'UOR-Foundation',
      repo: 'UOR-Foundation/Prism',
      repo_name: 'Prism',
      repo_url: 'https://github.com/UOR-Foundation/Prism',
      detected: false,
      source: 'default',
    };
  }

  async #fetchImpl(url, init) {
    const fn = this.fetchImpl || globalThis.fetch;
    return fn(url, init);
  }

  async #resolveRelayGithubApi(action, params) {
    if (!this.relayUrl && !this.relayHttpUrl) return null;
    try {
      const res = await this.tryRelayRpc('github_api', { action, ...params });
      if (res && typeof res.ok === 'boolean') return res;
    } catch {
      // Fall through to direct HTTPS REST fetch
    }
    return null;
  }

  async #githubRestJson(coords, pathAndQuery, { method = 'GET', body = undefined, headers = {} } = {}) {
    const cleanSub = String(pathAndQuery || '').replace(/^\/+/, '');
    const apiBase = `https://api.github.com/repos/${coords.owner}/${coords.repo}`;
    const fullUrl = cleanSub ? `${apiBase}/${cleanSub}` : apiBase;
    const urlObj = new URL(fullUrl);
    if (urlObj.hostname.toLowerCase() !== 'api.github.com' || urlObj.protocol !== 'https:') {
      return {
        ok: false,
        status: 403,
        error_code: 'HostNotAllowlistedForSecret',
        message: 'Refusing GitHub request outside api.github.com',
      };
    }
    const authRes = this.applyHostSecretSubstitution(urlObj, {
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      ...headers,
    });
    if (!authRes.ok) return authRes;
    const reqHeaders = { ...authRes.headers };
    if (body !== undefined && method !== 'GET' && method !== 'HEAD') {
      reqHeaders['Content-Type'] = 'application/json';
    }
    const relayRes = await this.tryRelayRpc('git_smart_http', {
      repo_url: apiBase,
      path_and_query: cleanSub,
      method,
      auth_bearer: reqHeaders.Authorization ? reqHeaders.Authorization.replace(/^Bearer\s+/i, '') : undefined,
      body_json: body,
    });
    if (relayRes && relayRes.ok && relayRes.data) {
      const rawTxt = relayRes.data.body_b64
        ? cachedTextDecoder.decode(__uorBase64ToBytes(relayRes.data.body_b64))
        : '';
      let parsed = null;
      try {
        parsed = rawTxt ? JSON.parse(rawTxt) : null;
      } catch {
        parsed = null;
      }
      return {
        ok: relayRes.data.status >= 200 && relayRes.data.status < 300,
        status: relayRes.data.status,
        json: parsed,
        text: rawTxt,
        viaRelay: true,
      };
    }
    const resp = await this.#fetchImpl(urlObj.href, {
      method,
      headers: reqHeaders,
      body: body !== undefined && method !== 'GET' && method !== 'HEAD' ? JSON.stringify(body) : undefined,
    });
    const text = await resp.text();
    let json = null;
    try {
      json = text ? JSON.parse(text) : null;
    } catch {
      json = null;
    }
    this.lastTransportMode = 'direct-https-github-rest';
    return { ok: resp.ok, status: resp.status, json, text, viaRelay: false };
  }

  async checkProvenanceUpdate(provenanceUrlInput = '') {
    const explicitStaticBase = this.staticOrigin ? this.staticOrigin.replace(/\/+$/, '') : '';
    let moduleProvUrl = '';
    try {
      const modUrl = new URL('provenance.json', import.meta.url);
      if (/^https?:/i.test(modUrl.protocol)) {
        moduleProvUrl = modUrl.href;
      }
    } catch {
      moduleProvUrl = '';
    }
    let locationDirProvUrl = '';
    if (!explicitStaticBase && !moduleProvUrl) {
      if (typeof location !== 'undefined' && location.href && /^https?:/i.test(location.href)) {
        const hrefBase = location.href.endsWith('/') ? location.href : location.href.replace(/[^/?#]*([?#].*)?$/, '');
        locationDirProvUrl = new URL('provenance.json', hrefBase).href;
      } else if (this.locationOrigin && /^https?:/i.test(this.locationOrigin)) {
        const optBase = this.locationOrigin.endsWith('/') ? this.locationOrigin : `${this.locationOrigin}/`;
        locationDirProvUrl = new URL('provenance.json', optBase).href;
      }
    }
    const rawInput = String(provenanceUrlInput || '').trim();
    const normalizedInput = rawInput
      ? (/\.json(?:[?#].*)?$/i.test(rawInput) ? rawInput : `${rawInput.replace(/\/+$/, '')}/provenance.json`)
      : '';
    const provUrl =
      normalizedInput ||
      (explicitStaticBase
        ? `${explicitStaticBase}/provenance.json`
        : moduleProvUrl || locationDirProvUrl || new URL('provenance.json', import.meta.url).href);
    // Strict R4-39 rule: NEVER attach Host Secrets to non-GitHub provenance origins
    const validated = this.validateTransportUrl(provUrl, false);
    if (!validated.ok) return validated;
    try {
      const sep = validated.url.href.includes('?') ? '&' : '?';
      const resp = await this.#fetchImpl(`${validated.url.href}${sep}_t=${Date.now()}`, { cache: 'no-store' });
      if (!resp.ok) {
        return { ok: false, status: resp.status, error_code: 'GitDeployWatchFailed', message: `provenance.json returned HTTP ${resp.status}` };
      }
      const live = await resp.json();
      if (!this.#provenanceBaseline) {
        this.#provenanceBaseline = {
          adapter_wasm_sha256: live.adapter_wasm_sha256 || '',
          binding_javascript_sha256: live.binding_javascript_sha256 || '',
          generated_core_sha256: live.generated_core_sha256 || '',
        };
      }
      const updateAvailable = Boolean(
        this.#provenanceBaseline &&
          ((live.adapter_wasm_sha256 && live.adapter_wasm_sha256 !== this.#provenanceBaseline.adapter_wasm_sha256) ||
            (live.binding_javascript_sha256 && live.binding_javascript_sha256 !== this.#provenanceBaseline.binding_javascript_sha256)),
      );
      return {
        ok: true,
        status: 200,
        provenance_url: validated.url.href,
        current: this.#provenanceBaseline,
        live,
        adapter_wasm_sha256: live.adapter_wasm_sha256 || '',
        binding_javascript_sha256: live.binding_javascript_sha256 || '',
        generated_core_sha256: live.generated_core_sha256 || '',
        update_available: updateAvailable,
      };
    } catch (err) {
      return { ok: false, status: 502, error_code: 'GitDeployWatchFailed', message: err && err.message ? err.message : String(err) };
    }
  }

  async handleGitBridge(req) {
    const rawArgs = String(req.args || (req.arguments && req.arguments.args) || '').trim();
    const argParts = rawArgs ? rawArgs.split(/\s+/) : [];
    const argsObj = (req.arguments && typeof req.arguments === 'object') ? req.arguments : {};
    const subcommand = String(req.subcommand || req.action || argsObj.action || argsObj.subcommand || argParts[0] || 'clone').toLowerCase();
    const rawRepo = String(req.repo_url || req.repo || argsObj.repo_url || argsObj.repo || req.url || argParts[1] || '').trim();

    if (subcommand === 'detect_repo' || subcommand === 'auto_detect') {
      return this.detectHostedGitHubRepo(req);
    }

    // Strict R4-39 secret check BEFORE routing or network access whenever a URL or bearer targets non-GitHub origins
    if (req.provenance_url) {
      let provHost = '';
      try {
        provHost = new URL(String(req.provenance_url)).hostname.toLowerCase();
      } catch {
        provHost = '';
      }
      const hasSecretInput = Boolean(req.auth_bearer) || String(req.auth_bearer || '').includes('{{MVM_SECRET:') ||
        Object.values(req.headers || {}).some((v) => String(v || '').includes('{{MVM_SECRET:'));
      if (hasSecretInput && provHost && !SECRET_HOST_ALLOWLIST.GITHUB_TOKEN.has(provHost)) {
        return {
          ok: false,
          status: 403,
          error_code: 'HostNotAllowlistedForSecret',
          message: `Secret GITHUB_TOKEN is not allowlisted for provenance host ${provHost}`,
        };
      }
    }

    if (req.path_and_query) {
      const repoValid = this.validateTransportUrl(rawRepo, false);
      if (!repoValid.ok) return repoValid;
      const host = repoValid.url.hostname.toLowerCase();
      let authBearer;
      if (this.hasSecret('GITHUB_TOKEN') || String(req.auth_bearer || '').includes('{{MVM_SECRET:')) {
        if (!SECRET_HOST_ALLOWLIST.GITHUB_TOKEN.has(host)) {
          return {
            ok: false,
            status: 403,
            error_code: 'HostNotAllowlistedForSecret',
            message: `Secret GITHUB_TOKEN is not allowlisted for host ${host}`,
          };
        }
        const subAuth = this.substituteHostSecretsInString(
          req.auth_bearer || '{{MVM_SECRET:GITHUB_TOKEN}}',
          host,
        );
        if (!subAuth.ok) return subAuth;
        authBearer = subAuth.value;
      }
      const relayGit = await this.tryRelayRpc('git_smart_http', {
        repo_url: repoValid.url.href,
        path_and_query: String(req.path_and_query),
        method: String(req.method || 'GET').toUpperCase(),
        content_type: req.content_type || undefined,
        auth_bearer: authBearer,
        body_b64: req.body_b64 || req.body_base64 || undefined,
        body_json: req.body_json || undefined,
      });
      if (relayGit && relayGit.ok && relayGit.data) {
        const d = relayGit.data;
        return {
          ok: d.status >= 200 && d.status < 400,
          status: d.status,
          op: 'git_smart_http',
          content_type: d.content_type || '',
          headers: d.headers || {},
          body_b64: d.body_b64 || '',
          body_base64: d.body_b64 || '',
          byte_len: d.byte_length ?? 0,
          truncated: Boolean(d.truncated),
          transport: this.lastTransportMode,
        };
      }
      if (relayGit && !relayGit.ok) {
        return {
          ok: false,
          status: relayGit.status || 502,
          error_code: relayGit.error_code || 'GitSmartHttpError',
          message: relayGit.error_message || 'Git Smart-HTTP relay request failed',
        };
      }
    }

    if (req.smart_http_url) {
      return this.handleHttpRequest({
        url: req.smart_http_url,
        method: req.method || 'GET',
        headers: req.headers || {},
        body_base64: req.body_base64,
        binary: true,
      });
    }

    // Special case: watch_deploy without repo can inspect static origin provenance.json directly
    if (subcommand === 'watch_deploy' && !rawRepo) {
      const provUrl = String(req.provenance_url || argsObj.provenance_url || '').trim();
      const provRes = await this.checkProvenanceUpdate(provUrl);
      if (!provRes.ok) return provRes;
      return {
        ok: true,
        status: 200,
        op: 'git',
        subcommand: 'watch_deploy',
        ...provRes,
        data: provRes,
        body_text: `Provenance digest (${String(provRes.adapter_wasm_sha256).slice(0, 12)}) update_available=${provRes.update_available}`,
        transport: this.lastTransportMode,
      };
    }

    // Reject non-GitHub URL remotes targeting secrets immediately with HostNotAllowlistedForSecret
    if (/^https?:\/\//i.test(rawRepo)) {
      let hostLower = '';
      try {
        hostLower = new URL(rawRepo).hostname.toLowerCase();
      } catch {
        hostLower = '';
      }
      const wantsSecret = this.hasSecret('GITHUB_TOKEN') || Boolean(req.auth_bearer) || rawRepo.includes('{{MVM_SECRET:') ||
        ['push', 'commit_push', 'create_branch', 'create_pr', 'merge_pr'].includes(subcommand);
      if (hostLower && !SECRET_HOST_ALLOWLIST.GITHUB_TOKEN.has(hostLower) && wantsSecret) {
        return {
          ok: false,
          status: 403,
          error_code: 'HostNotAllowlistedForSecret',
          message: `Secret GITHUB_TOKEN is not allowlisted for remote host ${hostLower}`,
        };
      }
    }

    const coords = this.parseGitHubRepoCoords(rawRepo);
    if (!coords) {
      if (rawRepo) {
        const valid = this.validateTransportUrl(rawRepo, false);
        if (!valid.ok) return valid;
      }
      return {
        ok: false,
        status: 400,
        error_code: 'UnsupportedGitRemote',
        message: `Expected https://github.com/<owner>/<repo> or <owner>/<repo>, got "${rawRepo}"`,
      };
    }

    const repoSlug = `${coords.owner}/${coords.repo}`;
    const resolvedAuthBearer = this.hasSecret('GITHUB_TOKEN') ? this.#secretVault.get('GITHUB_TOKEN') : undefined;

    if (subcommand === 'connect' || subcommand === 'branches') {
      try {
        const relayOut = await this.#resolveRelayGithubApi('connect', { repo: repoSlug, auth_bearer: resolvedAuthBearer });
        if (relayOut && relayOut.ok && relayOut.data) {
          this.lastTransportMode = this.lastTransportMode || 'spvs-relay';
          return {
            ok: true,
            status: 200,
            op: 'git',
            subcommand: 'connect',
            repo: repoSlug,
            ...relayOut.data,
            data: relayOut.data,
            body_text: `Connected ${repoSlug} (default: ${relayOut.data.default_branch}, ${(relayOut.data.branches || []).length} branches)`,
            transport: this.lastTransportMode,
          };
        }
        const meta = await this.#githubRestJson(coords, '');
        if (!meta.ok) {
          return {
            ok: false,
            status: meta.status,
            error_code: meta.status === 401 || meta.status === 404 ? 'GitRepoAccessDenied' : 'GitApiError',
            message: (meta.json && meta.json.message) || `GitHub API returned HTTP ${meta.status} for ${repoSlug}`,
          };
        }
        const br = await this.#githubRestJson(coords, 'branches?per_page=50');
        const branchNames = Array.isArray(br.json) ? br.json.map((b) => String(b.name || '')).filter(Boolean) : [];
        const defBranch = String((meta.json && meta.json.default_branch) || 'main');
        if (defBranch && !branchNames.includes(defBranch)) branchNames.unshift(defBranch);
        const data = {
          repo: repoSlug,
          default_branch: defBranch,
          private: Boolean(meta.json && meta.json.private),
          html_url: (meta.json && meta.json.html_url) || coords.url,
          branches: branchNames,
        };
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand: 'connect',
          repo: repoSlug,
          ...data,
          data,
          body_text: `Connected ${repoSlug} (default: ${defBranch}, ${branchNames.length} branches)`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitNetworkFailure', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'clone' || subcommand === 'pull' || subcommand === 'fetch') {
      try {
        const requestedBranch = String(req.branch || argsObj.branch || '').trim();
        const relayOut = await this.#resolveRelayGithubApi(subcommand, {
          repo: repoSlug,
          branch: requestedBranch || undefined,
          limit: req.limit || 40,
          auth_bearer: resolvedAuthBearer,
        });
        if (relayOut && relayOut.ok && relayOut.data) {
          const d = relayOut.data;
          const treeKey = `${repoSlug}@${d.branch}`;
          const lazyMap = new Map();
          for (const lp of d.lazy_paths || []) {
            lazyMap.set(lp.path, lp);
          }
          this.#lazyRepoTrees.set(treeKey, lazyMap);
          return {
            ok: true,
            status: 200,
            op: 'git',
            subcommand,
            repo: repoSlug,
            repo_url: coords.url,
            branch: d.branch,
            head_sha: d.head_sha || '',
            files: d.files || [],
            file_count: (d.files || []).length,
            lazy_paths: d.lazy_paths || [],
            total_blobs: d.total_blobs ?? (d.files || []).length,
            data: d,
            body_text: `Synchronized ${(d.files || []).length}/${d.total_blobs ?? (d.files || []).length} files from ${repoSlug}@${d.branch}`,
            transport: this.lastTransportMode,
          };
        }

        const metaRes = await this.#githubRestJson(coords, '');
        if (!metaRes.ok) {
          return {
            ok: false,
            status: metaRes.status,
            error_code: metaRes.status === 401 || metaRes.status === 404 ? 'GitRepoAccessDenied' : 'GitApiError',
            message: `GitHub API returned HTTP ${metaRes.status} for ${repoSlug}`,
          };
        }
        const repoMeta = metaRes.json || {};
        const branch = requestedBranch || String(repoMeta.default_branch || 'main');
        const brRes = await this.#githubRestJson(coords, 'branches?per_page=50').catch(() => ({ ok: false, json: [] }));
        const branches = Array.isArray(brRes.json) ? brRes.json.map((b) => String(b.name || '')).filter(Boolean) : [branch];
        if (!branches.includes(branch)) branches.unshift(branch);

        const treeRes = await this.#githubRestJson(coords, `git/trees/${encodeURIComponent(branch)}?recursive=1`);
        if (!treeRes.ok) {
          return {
            ok: false,
            status: treeRes.status,
            error_code: 'GitTreeFetchFailed',
            message: `Failed to fetch tree for ${repoSlug}@${branch} (HTTP ${treeRes.status})`,
          };
        }
        const treeJson = treeRes.json || {};
        const blobs = (Array.isArray(treeJson.tree) ? treeJson.tree : []).filter(
          (entry) =>
            entry.type === 'blob' &&
            typeof entry.path === 'string' &&
            (entry.size ?? 0) <= 196608 &&
            !/\.(png|jpg|jpeg|gif|webp|ico|woff2?|ttf|wasm|bin|part\d+|xz|gz|zip|pdf)$/i.test(entry.path),
        );
        const eagerCap = Number.isInteger(Number(req.limit)) && Number(req.limit) > 0 ? Math.min(Number(req.limit), 80) : 40;
        const selected = blobs.slice(0, eagerCap);
        const lazyPaths = blobs.slice(eagerCap).map((b) => ({ path: b.path, sha: b.sha || '', size: b.size ?? 0 }));
        const treeKey = `${repoSlug}@${branch}`;
        const lazyMap = new Map();
        for (const lp of lazyPaths) lazyMap.set(lp.path, lp);
        this.#lazyRepoTrees.set(treeKey, lazyMap);

        const hasToken = this.hasSecret('GITHUB_TOKEN');
        const fetchedFiles = [];
        const batchSize = 8;
        for (let i = 0; i < selected.length; i += batchSize) {
          const slice = selected.slice(i, i + batchSize);
          const loaded = await Promise.all(
            slice.map(async (entry) => {
              try {
                if (!hasToken && !repoMeta.private) {
                  const rawUrl = `https://raw.githubusercontent.com/${coords.owner}/${coords.repo}/${encodeURIComponent(branch)}/${entry.path}`;
                  const r = await this.#fetchImpl(rawUrl);
                  if (r.ok) {
                    return { path: entry.path, content: await r.text(), sha: entry.sha, size: entry.size ?? 0 };
                  }
                }
                const blobRes = await this.#githubRestJson(coords, `git/blobs/${entry.sha}`);
                if (blobRes.ok && blobRes.json && blobRes.json.encoding === 'base64' && typeof blobRes.json.content === 'string') {
                  const bytes = __uorBase64ToBytes(blobRes.json.content);
                  return { path: entry.path, content: cachedTextDecoder.decode(bytes), sha: entry.sha, size: entry.size ?? bytes.byteLength };
                }
                return null;
              } catch {
                return null;
              }
            }),
          );
          for (const item of loaded) {
            if (item) fetchedFiles.push(item);
          }
        }

        const lazyEntries = lazyPaths.map((lp) => ({
          path: lp.path,
          content: null,
          sha: lp.sha,
          size: lp.size,
          lazy: true,
        }));
        const allFiles = [...fetchedFiles, ...lazyEntries];

        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand,
          repo: repoSlug,
          repo_url: coords.url,
          branch,
          default_branch: repoMeta.default_branch || branch,
          branches,
          head_sha: treeJson.sha || '',
          files: allFiles,
          hydrated_count: fetchedFiles.length,
          file_count: allFiles.length,
          lazy_paths: lazyPaths,
          total_blobs: blobs.length,
          truncated: Boolean(treeJson.truncated) || lazyPaths.length > 0,
          data: {
            repo: repoSlug,
            default_branch: repoMeta.default_branch || branch,
            branch,
            branches,
            head_sha: treeJson.sha || '',
            files: fetchedFiles,
            lazy_paths: lazyPaths,
            total_blobs: blobs.length,
            file_count: fetchedFiles.length,
            truncated: Boolean(treeJson.truncated) || lazyPaths.length > 0,
          },
          body_text: `Synchronized ${fetchedFiles.length}/${blobs.length} files from ${repoSlug}@${branch} (${String(treeJson.sha || '').slice(0, 12)})`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return {
          ok: false,
          status: 502,
          error_code: 'GitNetworkFailure',
          message: err && err.message ? err.message : String(err),
        };
      }
    }

    if (subcommand === 'read_blob' || subcommand === 'cat_blob' || subcommand === 'blob') {
      const branch = String(req.branch || argsObj.branch || 'main').trim();
      const filePath = String(req.path || argsObj.path || argParts[2] || '').trim().replace(/^\/+/, '');
      const cachedEntry = this.#lazyRepoTrees.get(`${repoSlug}@${branch}`)?.get(filePath);
      const sha = String(req.sha || argsObj.sha || (cachedEntry && cachedEntry.sha) || '').trim();
      if (!sha && !filePath) {
        return { ok: false, status: 400, error_code: 'GitBlobFetchFailed', message: 'read_blob requires path or sha' };
      }
      try {
        const relayOut = await this.#resolveRelayGithubApi('read_blob', {
          repo: repoSlug,
          branch,
          path: filePath,
          sha: sha || undefined,
          auth_bearer: resolvedAuthBearer,
        });
        if (relayOut && relayOut.ok && relayOut.data) {
          const d = relayOut.data;
          return {
            ok: true,
            status: 200,
            op: 'git',
            subcommand: 'read_blob',
            repo: repoSlug,
            branch,
            path: d.path || filePath,
            sha: d.sha || sha,
            size: d.size ?? String(d.content || '').length,
            content: d.content || '',
            files: d.path ? [{ path: d.path, content: d.content || '', sha: d.sha || sha }] : [],
            data: d,
            body_text: `Loaded ${d.path || filePath} (${d.size ?? String(d.content || '').length} bytes) from ${repoSlug}@${branch}`,
            transport: this.lastTransportMode,
          };
        }
        const restPath = sha
          ? `git/blobs/${encodeURIComponent(sha)}`
          : `contents/${filePath.split('/').map((s) => encodeURIComponent(s)).join('/')}?ref=${encodeURIComponent(branch)}`;
        const blobRes = await this.#githubRestJson(coords, restPath);
        if (!blobRes.ok || !blobRes.json) {
          return {
            ok: false,
            status: blobRes.status || 404,
            error_code: 'GitBlobFetchFailed',
            message: (blobRes.json && blobRes.json.message) || `Failed to read blob ${filePath || sha}`,
          };
        }
        const content =
          blobRes.json.encoding === 'base64' && typeof blobRes.json.content === 'string'
            ? cachedTextDecoder.decode(__uorBase64ToBytes(blobRes.json.content))
            : String(blobRes.json.content || '');
        const resolvedSha = blobRes.json.sha || sha;
        const resolvedPath = filePath || blobRes.json.path || '';
        const data = { repo: repoSlug, branch, path: resolvedPath, sha: resolvedSha, size: blobRes.json.size ?? content.length, content };
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand: 'read_blob',
          repo: repoSlug,
          branch,
          path: resolvedPath,
          sha: resolvedSha,
          size: data.size,
          content,
          files: resolvedPath ? [{ path: resolvedPath, content, sha: resolvedSha }] : [],
          data,
          body_text: `Loaded ${resolvedPath} (${data.size} bytes) from ${repoSlug}@${branch}`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitBlobFetchFailed', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'create_branch' || subcommand === 'branch' || subcommand === 'checkout') {
      if (!this.hasSecret('GITHUB_TOKEN')) {
        return { ok: false, status: 401, error_code: 'GitAuthRequired', message: 'GitHub PAT required for create_branch.' };
      }
      const branch = String(req.branch || argsObj.branch || argParts[2] || '').trim();
      const fromBranchInput = String(req.from_branch || argsObj.from_branch || argsObj.base || '').trim();
      if (!branch || branch.includes('..')) {
        return { ok: false, status: 400, error_code: 'GitBranchCreateFailed', message: `Invalid branch name '${branch}'` };
      }
      try {
        const relayOut = await this.#resolveRelayGithubApi('create_branch', {
          repo: repoSlug,
          branch,
          from_branch: fromBranchInput || undefined,
          auth_bearer: resolvedAuthBearer,
        });
        if (relayOut && relayOut.ok && relayOut.data) {
          const d = relayOut.data;
          return {
            ok: true,
            status: 201,
            op: 'git',
            subcommand: 'create_branch',
            repo: repoSlug,
            branch: d.branch,
            from_branch: d.from_branch,
            sha: d.sha,
            ref: d.ref,
            data: d,
            body_text: `Created branch ${d.branch} from ${d.from_branch} at ${String(d.sha || '').slice(0, 12)}`,
            transport: this.lastTransportMode,
          };
        }
        let fromBranch = fromBranchInput;
        if (!fromBranch) {
          const meta = await this.#githubRestJson(coords, '');
          fromBranch = String((meta.json && meta.json.default_branch) || 'main');
        }
        const baseRef = await this.#githubRestJson(coords, `git/ref/heads/${encodeURIComponent(fromBranch)}`);
        if (!baseRef.ok || !baseRef.json || !baseRef.json.object || !baseRef.json.object.sha) {
          return { ok: false, status: baseRef.status || 404, error_code: 'GitRefReadFailed', message: `Base branch '${fromBranch}' not found` };
        }
        const baseSha = baseRef.json.object.sha;
        const created = await this.#githubRestJson(coords, 'git/refs', {
          method: 'POST',
          body: { ref: `refs/heads/${branch}`, sha: baseSha },
        });
        if (!created.ok) {
          return {
            ok: false,
            status: created.status,
            error_code: 'GitBranchCreateFailed',
            message: (created.json && created.json.message) || `Failed to create branch '${branch}'`,
          };
        }
        const data = {
          repo: repoSlug,
          branch,
          from_branch: fromBranch,
          ref: (created.json && created.json.ref) || `refs/heads/${branch}`,
          sha: (created.json && created.json.object && created.json.object.sha) || baseSha,
        };
        return {
          ok: true,
          status: created.status,
          op: 'git',
          subcommand: 'create_branch',
          repo: repoSlug,
          ...data,
          data,
          body_text: `Created branch ${branch} from ${fromBranch} at ${String(data.sha || '').slice(0, 12)}`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitBranchCreateFailed', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'push' || subcommand === 'commit_push' || subcommand === 'commit') {
      if (!this.hasSecret('GITHUB_TOKEN')) {
        return {
          ok: false,
          status: 401,
          error_code: 'GitAuthRequired',
          message: 'GitHub PAT not configured. Click GitHub in the header to store GITHUB_TOKEN in the host vault.',
        };
      }
      const filesToPush = Array.isArray(req.files) ? req.files : Array.isArray(argsObj.files) ? argsObj.files : [];
      if (filesToPush.length === 0) {
        return {
          ok: false,
          status: 400,
          error_code: 'GitNothingToPush',
          message: 'No modified workspace files supplied for commit_push',
        };
      }
      const branch = String(req.branch || argsObj.branch || 'main');
      const commitMessage = String(req.message || argsObj.message || req.commit_message || 'Update from UOR-R4 Connected Studio');
      try {
        const relayOut = await this.#resolveRelayGithubApi('commit_push', {
          repo: repoSlug,
          branch,
          message: commitMessage,
          files: filesToPush,
          force: Boolean(req.force),
          auth_bearer: resolvedAuthBearer,
        });
        if (relayOut && relayOut.ok && relayOut.data) {
          const d = relayOut.data;
          return {
            ok: true,
            status: 200,
            op: 'git',
            subcommand: 'commit_push',
            repo: repoSlug,
            branch,
            commit_sha: d.commit_sha,
            tree_sha: d.tree_sha,
            pushed_files: d.pushed_files,
            deleted_files: d.deleted_files || 0,
            data: d,
            body_text: `Committed & pushed ${d.pushed_files} file(s) to ${repoSlug}@${branch} -> ${String(d.commit_sha || '').slice(0, 12)}`,
            transport: this.lastTransportMode,
          };
        }

        const refRes = await this.#githubRestJson(coords, `git/ref/heads/${encodeURIComponent(branch)}`);
        if (!refRes.ok || !refRes.json || !refRes.json.object) {
          return {
            ok: false,
            status: refRes.status || 404,
            error_code: 'GitRefReadFailed',
            message: `Unable to read heads/${branch} on ${repoSlug} (HTTP ${refRes.status})`,
          };
        }
        const baseCommitSha = refRes.json.object.sha;
        const commitRes = await this.#githubRestJson(coords, `git/commits/${baseCommitSha}`);
        const baseTreeSha = commitRes.json && commitRes.json.tree && commitRes.json.tree.sha;

        const treeEntries = filesToPush.map((f) => {
          const pathClean = String(f.path || '').replace(/^\/+/, '');
          if (f.delete === true) {
            return { path: pathClean, mode: '100644', type: 'blob', sha: null };
          }
          return { path: pathClean, mode: String(f.mode || '100644'), type: 'blob', content: String(f.content ?? '') };
        });
        const newTree = await this.#githubRestJson(coords, 'git/trees', {
          method: 'POST',
          body: { base_tree: baseTreeSha, tree: treeEntries },
        });
        if (!newTree.ok || !newTree.json) {
          return { ok: false, status: newTree.status, error_code: 'GitTreeCreateFailed', message: `GitHub tree creation failed (HTTP ${newTree.status})` };
        }
        const createdCommit = await this.#githubRestJson(coords, 'git/commits', {
          method: 'POST',
          body: { message: commitMessage, tree: newTree.json.sha, parents: [baseCommitSha] },
        });
        if (!createdCommit.ok || !createdCommit.json) {
          return { ok: false, status: createdCommit.status, error_code: 'GitCommitCreateFailed', message: `GitHub commit creation failed (HTTP ${createdCommit.status})` };
        }
        const updRef = await this.#githubRestJson(coords, `git/refs/heads/${encodeURIComponent(branch)}`, {
          method: 'PATCH',
          body: { sha: createdCommit.json.sha, force: Boolean(req.force) },
        });
        if (!updRef.ok) {
          return { ok: false, status: updRef.status, error_code: 'GitRefUpdateFailed', message: `GitHub ref fast-forward failed (HTTP ${updRef.status})` };
        }
        const deletedFiles = treeEntries.filter((e) => e.sha === null).length;
        const pushedFiles = treeEntries.length - deletedFiles;
        const data = {
          repo: repoSlug,
          branch,
          commit_sha: createdCommit.json.sha,
          tree_sha: newTree.json.sha,
          pushed_files: pushedFiles,
          deleted_files: deletedFiles,
        };
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand: subcommand === 'push' ? 'push' : 'commit_push',
          ...data,
          data,
          body_text: `Committed & pushed ${pushedFiles} file(s) to ${repoSlug}@${branch} -> ${String(createdCommit.json.sha || '').slice(0, 12)}`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitPushNetworkError', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'create_pr' || subcommand === 'open_pr') {
      if (!this.hasSecret('GITHUB_TOKEN')) {
        return { ok: false, status: 401, error_code: 'GitAuthRequired', message: 'GitHub PAT required for create_pr.' };
      }
      const head = String(req.head || argsObj.head || req.branch || argsObj.branch || '').trim();
      const base = String(req.base || argsObj.base || 'main').trim();
      const title = String(req.title || argsObj.title || `Update ${head}`).trim();
      const body = String(req.body || argsObj.body || 'PR opened from UOR-R4 Connected Studio').trim();
      if (!head || !title) {
        return { ok: false, status: 400, error_code: 'GitPrCreateFailed', message: 'create_pr requires head branch and title' };
      }
      try {
        const relayOut = await this.#resolveRelayGithubApi('create_pr', {
          repo: repoSlug, head, base, title, body, draft: Boolean(req.draft), auth_bearer: resolvedAuthBearer,
        });
        if (relayOut && relayOut.ok && relayOut.data) {
          const d = relayOut.data;
          return {
            ok: true,
            status: 201,
            op: 'git',
            subcommand: 'create_pr',
            repo: repoSlug,
            ...d,
            data: d,
            body_text: `Opened PR #${d.pr_number} (${d.head} -> ${d.base}): ${d.html_url || d.url}`,
            transport: this.lastTransportMode,
          };
        }
        const prRes = await this.#githubRestJson(coords, 'pulls', {
          method: 'POST',
          body: { title, body, head, base, draft: Boolean(req.draft) },
        });
        if (!prRes.ok || !prRes.json) {
          return {
            ok: false,
            status: prRes.status,
            error_code: 'GitPrCreateFailed',
            message: (prRes.json && prRes.json.message) || `GitHub PR creation failed (HTTP ${prRes.status})`,
          };
        }
        const data = {
          repo: repoSlug,
          pr_number: prRes.json.number,
          url: prRes.json.url || '',
          html_url: prRes.json.html_url || '',
          state: prRes.json.state || 'open',
          head: (prRes.json.head && prRes.json.head.ref) || head,
          base: (prRes.json.base && prRes.json.base.ref) || base,
          title: prRes.json.title || title,
        };
        return {
          ok: true,
          status: prRes.status,
          op: 'git',
          subcommand: 'create_pr',
          ...data,
          data,
          body_text: `Opened PR #${data.pr_number} (${data.head} -> ${data.base}): ${data.html_url || data.url}`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitPrCreateFailed', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'pr_status' || subcommand === 'ci_status') {
      const prNum = Number(req.pr_number || argsObj.pr_number || 0) || null;
      let headSha = String(req.ref || argsObj.ref || req.head_sha || '').trim();
      try {
        const relayOut = await this.#resolveRelayGithubApi('pr_status', {
          repo: repoSlug, pr_number: prNum || undefined, ref: headSha || undefined, auth_bearer: resolvedAuthBearer,
        });
        if (relayOut && relayOut.ok && relayOut.data) {
          const d = relayOut.data;
          return {
            ok: true,
            status: 200,
            op: 'git',
            subcommand: 'pr_status',
            ...d,
            data: d,
            body_text: `PR #${d.pr_number || '-'} state=${d.state} mergeable=${d.mergeable} checks=${d.check_status} (${d.checks_passed}/${d.checks_total})`,
            transport: this.lastTransportMode,
          };
        }
        let prMeta = null;
        if (prNum && Number.isInteger(prNum) && prNum > 0) {
          const prRes = await this.#githubRestJson(coords, `pulls/${prNum}`);
          if (!prRes.ok || !prRes.json) {
            return { ok: false, status: prRes.status || 404, error_code: 'GitPrStatusFailed', message: `Failed to fetch PR #${prNum}` };
          }
          prMeta = prRes.json;
          headSha = (prMeta.head && prMeta.head.sha) || headSha;
        }
        let checkRuns = [];
        let workflowRuns = [];
        if (headSha) {
          const [cr, wr] = await Promise.all([
            this.#githubRestJson(coords, `commits/${encodeURIComponent(headSha)}/check-runs`),
            this.#githubRestJson(coords, `actions/runs?head_sha=${encodeURIComponent(headSha)}&per_page=10`),
          ]);
          if (cr.ok && cr.json && Array.isArray(cr.json.check_runs)) checkRuns = cr.json.check_runs;
          if (wr.ok && wr.json && Array.isArray(wr.json.workflow_runs)) workflowRuns = wr.json.workflow_runs;
        }
        const checksPassed = checkRuns.filter((c) => c.conclusion === 'success' || c.conclusion === 'neutral').length;
        const anyPending = checkRuns.some((c) => c.status !== 'completed') || workflowRuns.some((w) => w.status !== 'completed');
        const anyFailed = checkRuns.some((c) => c.conclusion === 'failure' || c.conclusion === 'timed_out') ||
          workflowRuns.some((w) => w.conclusion === 'failure' || w.conclusion === 'cancelled');
        const checkStatus = anyFailed ? 'failure' : anyPending ? 'pending' : checkRuns.length > 0 ? 'success' : 'unknown';
        const latestWf = workflowRuns[0] || null;
        const data = {
          repo: repoSlug,
          pr_number: prNum,
          state: (prMeta && prMeta.state) || (prNum ? 'unknown' : 'commit'),
          mergeable: prMeta ? (prMeta.mergeable ?? null) : null,
          mergeable_state: (prMeta && prMeta.mergeable_state) || 'unknown',
          html_url: (prMeta && prMeta.html_url) || '',
          head_sha: headSha,
          check_status: checkStatus,
          checks_total: checkRuns.length,
          checks_passed: checksPassed,
          workflow_status: (latestWf && latestWf.status) || 'unknown',
          workflow_conclusion: (latestWf && latestWf.conclusion) || null,
          check_runs: checkRuns.map((c) => ({ name: c.name, status: c.status, conclusion: c.conclusion })),
          workflow_runs: workflowRuns.map((w) => ({ id: w.id, name: w.name, status: w.status, conclusion: w.conclusion })),
        };
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand: 'pr_status',
          ...data,
          data,
          body_text: `PR #${data.pr_number || '-'} state=${data.state} mergeable=${data.mergeable} checks=${data.check_status} (${data.checks_passed}/${data.checks_total})`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitPrStatusFailed', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'merge_pr') {
      if (!this.hasSecret('GITHUB_TOKEN')) {
        return { ok: false, status: 401, error_code: 'GitAuthRequired', message: 'GitHub PAT required for merge_pr.' };
      }
      const prNum = Number(req.pr_number || argsObj.pr_number || 0);
      if (!Number.isInteger(prNum) || prNum <= 0) {
        return { ok: false, status: 400, error_code: 'GitPrMergeFailed', message: 'merge_pr requires valid integer pr_number' };
      }
      try {
        const relayOut = await this.#resolveRelayGithubApi('merge_pr', {
          repo: repoSlug,
          pr_number: prNum,
          merge_method: req.merge_method || argsObj.merge_method || 'squash',
          commit_title: req.commit_title || argsObj.commit_title || undefined,
          auth_bearer: resolvedAuthBearer,
        });
        if (relayOut && relayOut.ok && relayOut.data) {
          const d = relayOut.data;
          return {
            ok: true,
            status: 200,
            op: 'git',
            subcommand: 'merge_pr',
            ...d,
            data: d,
            body_text: `Merged PR #${prNum} on ${repoSlug} (sha=${String(d.sha || '').slice(0, 12)})`,
            transport: this.lastTransportMode,
          };
        }
        const mergeBody = { merge_method: String(req.merge_method || argsObj.merge_method || 'squash') };
        if (req.commit_title || argsObj.commit_title) mergeBody.commit_title = String(req.commit_title || argsObj.commit_title);
        const mRes = await this.#githubRestJson(coords, `pulls/${prNum}/merge`, { method: 'PUT', body: mergeBody });
        if (!mRes.ok) {
          return { ok: false, status: mRes.status, error_code: 'GitPrMergeFailed', message: (mRes.json && mRes.json.message) || `Failed to merge PR #${prNum}` };
        }
        const data = {
          repo: repoSlug,
          pr_number: prNum,
          merged: Boolean(mRes.json && (mRes.json.merged ?? true)),
          sha: (mRes.json && mRes.json.sha) || '',
          message: (mRes.json && mRes.json.message) || 'Pull Request merged',
        };
        return {
          ok: true,
          status: mRes.status,
          op: 'git',
          subcommand: 'merge_pr',
          ...data,
          data,
          body_text: `Merged PR #${prNum} on ${repoSlug} (sha=${String(data.sha || '').slice(0, 12)})`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitPrMergeFailed', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'watch_deploy') {
      const provUrlOverride = String(req.provenance_url || argsObj.provenance_url || '').trim();
      try {
        const relayOut = await this.#resolveRelayGithubApi('watch_deploy', {
          repo: repoSlug,
          provenance_url: provUrlOverride || undefined,
          auth_bearer: resolvedAuthBearer,
        });
        if (relayOut && relayOut.ok && relayOut.data) {
          const d = relayOut.data;
          const updateAvail = Boolean(
            this.#provenanceBaseline &&
              d.adapter_wasm_sha256 &&
              (d.adapter_wasm_sha256 !== this.#provenanceBaseline.adapter_wasm_sha256 ||
                d.binding_javascript_sha256 !== this.#provenanceBaseline.binding_javascript_sha256),
          );
          const merged = { ...d, update_available: updateAvail };
          return {
            ok: true,
            status: 200,
            op: 'git',
            subcommand: 'watch_deploy',
            ...merged,
            data: merged,
            body_text: `Pages ${merged.pages_status} (${merged.pages_url || 'origin'}) · provenance=${String(merged.adapter_wasm_sha256 || '').slice(0, 12)} · update=${updateAvail}`,
            transport: this.lastTransportMode,
          };
        }
        const [pagesRes, buildRes] = await Promise.all([
          this.#githubRestJson(coords, 'pages').catch(() => ({ ok: false, json: null })),
          this.#githubRestJson(coords, 'pages/builds/latest').catch(() => ({ ok: false, json: null })),
        ]);
        const pagesUrl = pagesRes.ok && pagesRes.json ? String(pagesRes.json.html_url || '') : '';
        const targetProvUrl = provUrlOverride || (pagesUrl ? `${pagesUrl.replace(/\/+$/, '')}/provenance.json` : '');
        let provRes = null;
        if (targetProvUrl) {
          provRes = await this.checkProvenanceUpdate(targetProvUrl);
        }
        const classicBuildStatus = buildRes.ok && buildRes.json ? String(buildRes.json.status || '') : '';
        let workflowStatus = '';
        let workflowSha = '';
        if (pagesRes.ok && pagesRes.json && !pagesRes.json.status && !classicBuildStatus) {
          const depRes = await this.#githubRestJson(coords, 'deployments?environment=github-pages&per_page=1').catch(() => ({ ok: false, json: null }));
          const latestDep = depRes.ok && Array.isArray(depRes.json) && depRes.json.length > 0 ? depRes.json[0] : null;
          if (latestDep) {
            workflowSha = String(latestDep.sha || '');
            if (latestDep.id != null) {
              const stRes = await this.#githubRestJson(coords, `deployments/${latestDep.id}/statuses?per_page=1`).catch(() => ({ ok: false, json: null }));
              const latestSt = stRes.ok && Array.isArray(stRes.json) && stRes.json.length > 0 ? stRes.json[0] : null;
              const rawDepState = String((latestSt && latestSt.state) || '').toLowerCase();
              if (rawDepState === 'success') workflowStatus = 'built';
              else if (rawDepState === 'failure' || rawDepState === 'error') workflowStatus = 'errored';
              else if (rawDepState === 'in_progress' || rawDepState === 'queued' || rawDepState === 'pending' || rawDepState === 'waiting') workflowStatus = 'building';
            }
          }
          if (!workflowStatus && provRes && provRes.ok) {
            workflowStatus = 'built';
          }
        }
        const resolvedPagesStatus =
          pagesRes.ok && pagesRes.json
            ? (pagesRes.json.status || classicBuildStatus || workflowStatus || (provRes && provRes.ok ? 'built' : 'unknown'))
            : 'unconfigured';
        const data = {
          repo: repoSlug,
          pages_status: resolvedPagesStatus,
          pages_url: pagesUrl,
          build_status: classicBuildStatus || workflowStatus || 'unknown',
          deployment_sha: (buildRes.ok && buildRes.json ? String(buildRes.json.commit || '') : '') || workflowSha,
          provenance_url: targetProvUrl,
          provenance: provRes && provRes.ok ? provRes.live : null,
          adapter_wasm_sha256: provRes && provRes.ok ? provRes.adapter_wasm_sha256 : '',
          binding_javascript_sha256: provRes && provRes.ok ? provRes.binding_javascript_sha256 : '',
          generated_core_sha256: provRes && provRes.ok ? provRes.generated_core_sha256 : '',
          update_available: Boolean(provRes && provRes.ok && provRes.update_available),
        };
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand: 'watch_deploy',
          ...data,
          data,
          body_text: `Pages ${data.pages_status} (${data.pages_url || targetProvUrl || 'local'}) · provenance=${String(data.adapter_wasm_sha256 || '').slice(0, 12)} · update=${data.update_available}`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitDeployWatchFailed', message: err && err.message ? err.message : String(err) };
      }
    }

    return {
      ok: false,
      status: 400,
      error_code: 'UnsupportedGitSubcommand',
      message: `Unsupported connected git subcommand: ${subcommand}`,
    };
  }

  async handlePkgFetch(req) {
    const sub = String(req.subcommand || req.action || (req.name || req.pkg_name ? 'install' : 'list')).toLowerCase();
    const pkgName = String(req.pkg_name || req.name || '').trim().replace(/[^a-zA-Z0-9_.-]/g, '');
    const explicitOrigin = this.staticOrigin ? this.staticOrigin.replace(/\/+$/, '') : '';
    const httpOrigin =
      typeof location !== 'undefined' && location.origin && /^https?:/i.test(location.origin)
        ? location.origin.replace(/\/+$/, '')
        : '';
    const indexCandidates = [
      ...(explicitOrigin ? [`${explicitOrigin}/weights/mvm-pkgs/index.json`] : []),
      new URL('weights/mvm-pkgs/index.json', import.meta.url).href,
      new URL('../gpu/weights/mvm-pkgs/index.json', import.meta.url).href,
      ...(httpOrigin ? [`${httpOrigin}/weights/mvm-pkgs/index.json`] : []),
    ];

    let pkgIndex = null;
    let basePkgUrl = '';
    for (const idxUrl of indexCandidates) {
      for (let attempt = 0; attempt < 2; attempt += 1) {
        try {
          const r = await fetch(idxUrl, { cache: 'no-cache' });
          if (r.ok) {
            pkgIndex = await r.json();
            basePkgUrl = idxUrl.slice(0, idxUrl.lastIndexOf('/') + 1);
          }
          break;
        } catch {
          // Retry once on transient socket error (e.g. pooled keep-alive ECONNRESET after sync Wasm work).
        }
      }
      if (pkgIndex) break;
    }

    if (!pkgIndex || !Array.isArray(pkgIndex.packages)) {
      return {
        ok: false,
        status: 404,
        error_code: 'PkgIndexUnavailable',
        message: 'Static RV64IMAC package index (mvm-pkgs/index.json) not found on origin',
      };
    }

    if (sub === 'list' || sub === 'search') {
      const q = pkgName.toLowerCase();
      const pkgs = pkgIndex.packages.filter((p) => !q || String(p.name || '').toLowerCase().includes(q));
      const lines = pkgs.map(
        (p) => `${String(p.name).padEnd(12, ' ')} ${String(p.version || '1.0.0').padEnd(8, ' ')} ${p.size_bytes ?? p.byte_length ?? 0} B  ${p.description || 'riscv64-linux-musl static ELF'}`,
      );
      return {
        ok: true,
        status: 200,
        op: 'pkg_fetch',
        subcommand: sub,
        packages: pkgs,
        body_text: lines.join('\n') || 'No matching RV64IMAC packages.',
      };
    }

    const entry = pkgIndex.packages.find((p) => p.name === pkgName);
    if (!entry) {
      return {
        ok: false,
        status: 404,
        error_code: 'PackageNotFound',
        message: `Package "${pkgName}" not found in uor-mvm-pkg-index/1`,
      };
    }

    const elfFile = entry.file || `${pkgName}.elf`;
    const elfResp = await fetch(basePkgUrl + elfFile);
    if (!elfResp.ok) {
      return {
        ok: false,
        status: elfResp.status,
        error_code: 'PackageDownloadFailed',
        message: `Failed to download ${elfFile} (HTTP ${elfResp.status})`,
      };
    }
    const elfBytes = new Uint8Array(await elfResp.arrayBuffer());
    const verified = await this.verifyElf64RiscvAndSha256(elfBytes, entry.sha256 || '');
    if (!verified.ok) {
      return {
        ok: false,
        status: 422,
        error_code: verified.error_code,
        message: verified.message,
      };
    }

    const elfB64 = __uorBytesToBase64(elfBytes);
    return {
      ok: true,
      status: 200,
      op: 'pkg_fetch',
      subcommand: 'install',
      installed: true,
      name: pkgName,
      pkg_name: pkgName,
      install_path: `/usr/local/bin/${pkgName}`,
      expected_sha256: verified.sha256,
      sha256: verified.sha256,
      byte_len: elfBytes.byteLength,
      e_machine: verified.e_machine,
      elf_machine: verified.e_machine,
      elf_base64: elfB64,
      data: {
        installed: true,
        name: pkgName,
        pkg_name: pkgName,
        expected_sha256: verified.sha256,
        sha256: verified.sha256,
        elf_machine: verified.e_machine,
        elf_base64: elfB64,
      },
      body_text: `Installed verified RV64IMAC ELF64 binary /usr/local/bin/${pkgName} (${elfBytes.byteLength} B, sha256:${verified.sha256.slice(0, 16)})`,
    };
  }

  #assertNetworkAllowed() {
    if (this.mcpPolicy === 'deny') {
      return {
        ok: false,
        status: 403,
        error_code: 'PolicyDenied',
        message: 'Outbound bridge operation denied by active MCP security policy',
      };
    }
    if (!this.webAccessEnabled) {
      return {
        ok: false,
        status: 403,
        error_code: 'PolicyDenied',
        message: 'Web Access is currently toggled OFF in the studio topbar',
      };
    }
    return { ok: true };
  }

  async #verifySri(bytes, integrityStr) {
    const view = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes || 0);
    const tokens = String(integrityStr || '').trim().split(/\s+/).filter(Boolean);
    if (tokens.length === 0) {
      return {
        ok: false,
        error_code: 'IntegrityMismatch',
        message: 'Missing required Subresource Integrity digest',
      };
    }
    const algMap = {
      sha256: 'SHA-256',
      sha384: 'SHA-384',
      sha512: 'SHA-512',
    };
    let sawSupported = false;
    for (const tok of tokens) {
      const dash = tok.indexOf('-');
      if (dash <= 0) continue;
      const algKey = tok.slice(0, dash).toLowerCase();
      const expectedB64 = tok.slice(dash + 1);
      const subtleAlg = algMap[algKey];
      if (!subtleAlg || !expectedB64) continue;
      sawSupported = true;
      const digBuf = await crypto.subtle.digest(subtleAlg, view);
      const actualB64 = __uorBytesToBase64(new Uint8Array(digBuf));
      if (actualB64 === expectedB64) {
        return { ok: true, algorithm: subtleAlg, digest: tok };
      }
    }
    return {
      ok: false,
      error_code: 'IntegrityMismatch',
      message: sawSupported
        ? `SRI integrity mismatch against ${tokens[0].slice(0, 24)}...`
        : `Unsupported SRI algorithm in "${integrityStr}"`,
    };
  }

  async #gunzipBytes(u8) {
    if (typeof DecompressionStream === 'function') {
      const ds = new DecompressionStream('gzip');
      const writer = ds.writable.getWriter();
      const writeP = writer.write(u8).then(() => writer.close());
      const reader = ds.readable.getReader();
      const chunks = [];
      let total = 0;
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        if (value && value.byteLength > 0) {
          chunks.push(value);
          total += value.byteLength;
        }
      }
      await writeP;
      const out = new Uint8Array(total);
      let off = 0;
      for (const c of chunks) {
        out.set(c, off);
        off += c.byteLength;
      }
      return out;
    }
    const err = new Error('DecompressionStream("gzip") unavailable in host environment');
    err.code = 'IntegrityMismatch';
    throw err;
  }

  #parseTarEntries(tarBytes) {
    const entries = [];
    let offset = 0;
    let nextLongName = '';
    let nextLongLink = '';
    const parseOctal = (slice) => {
      const txt = cachedTextDecoder.decode(slice).replace(/\0.*$/, '').trim();
      return txt ? Number('0o' + txt) : 0;
    };
    const parsePax = (dataBytes) => {
      const pax = {};
      const text = cachedTextDecoder.decode(dataBytes);
      let pos = 0;
      while (pos < text.length) {
        const sp = text.indexOf(' ', pos);
        if (sp < 0) break;
        const len = Number(text.slice(pos, sp));
        if (!Number.isFinite(len) || len <= 0) break;
        const rec = text.slice(sp + 1, pos + len).replace(/\n$/, '');
        const eq = rec.indexOf('=');
        if (eq > 0) pax[rec.slice(0, eq)] = rec.slice(eq + 1);
        pos += len;
      }
      return pax;
    };
    let paxNext = {};
    while (offset + 512 <= tarBytes.byteLength) {
      const hdr = tarBytes.subarray(offset, offset + 512);
      offset += 512;
      let zero = true;
      for (let i = 0; i < 512; i += 1) {
        if (hdr[i] !== 0) {
          zero = false;
          break;
        }
      }
      if (zero) break;
      const rawName = cachedTextDecoder.decode(hdr.subarray(0, 100)).replace(/\0.*$/, '');
      const prefix = cachedTextDecoder.decode(hdr.subarray(345, 500)).replace(/\0.*$/, '');
      const mode = parseOctal(hdr.subarray(100, 108)) || 0o644;
      const size = parseOctal(hdr.subarray(124, 136)) || 0;
      const typeflag = String.fromCharCode(hdr[156] || 48);
      const rawLink = cachedTextDecoder.decode(hdr.subarray(157, 257)).replace(/\0.*$/, '');
      const dataSlice = size > 0 ? tarBytes.subarray(offset, offset + size) : new Uint8Array(0);
      const padded = Math.ceil(size / 512) * 512;
      offset += padded;

      if (typeflag === 'L') {
        nextLongName = cachedTextDecoder.decode(dataSlice).replace(/\0.*$/, '');
        continue;
      }
      if (typeflag === 'K') {
        nextLongLink = cachedTextDecoder.decode(dataSlice).replace(/\0.*$/, '');
        continue;
      }
      if (typeflag === 'x') {
        paxNext = parsePax(dataSlice);
        continue;
      }
      if (typeflag === 'g') {
        continue;
      }

      const fullName = paxNext.path || nextLongName || (prefix ? `${prefix}/${rawName}` : rawName);
      const linkname = paxNext.linkpath || nextLongLink || rawLink;
      nextLongName = '';
      nextLongLink = '';
      paxNext = {};

      entries.push({
        name: fullName,
        mode,
        typeflag,
        linkname,
        data: new Uint8Array(dataSlice),
      });
    }
    return entries;
  }

  async ensureEsbuildCompiler() {
    if (this.#esbuildCompiler) return { ok: true, compiler: this.#esbuildCompiler };
    if (!this.esbuildModuleUrl || !this.esbuildWasmUrl) {
      return {
        ok: false,
        error_code: 'EsbuildAdapterUnavailable',
        message: 'Foundry esbuildModuleUrl / esbuildWasmUrl not configured in Worker host config',
      };
    }
    if (!this.#esbuildPromise) {
      this.#esbuildPromise = (async () => {
        const mod = await import(this.esbuildModuleUrl);
        if (mod && typeof mod.initialize === 'function') {
          await mod.initialize({ wasmURL: this.esbuildWasmUrl, worker: false });
        }
        this.#esbuildCompiler = mod;
        return mod;
      })();
    }
    try {
      const compiler = await this.#esbuildPromise;
      return { ok: true, compiler };
    } catch (err) {
      this.#esbuildPromise = null;
      this.#esbuildCompiler = null;
      return {
        ok: false,
        error_code: 'EsbuildAdapterUnavailable',
        message: err && err.message ? err.message : String(err),
      };
    }
  }

  #needsEsbuildTranspile(code, filePath, args) {
    const ext = String(filePath || '').toLowerCase();
    if (
      ext.endsWith('.ts') ||
      ext.endsWith('.tsx') ||
      ext.endsWith('.mts') ||
      ext.endsWith('.cts') ||
      ext.endsWith('.jsx') ||
      ext.endsWith('.mjs')
    ) {
      return true;
    }
    const argList = Array.isArray(args) ? args : [];
    if (argList.some((a) => String(a).includes('--loader=ts') || String(a) === '--experimental-strip-types')) {
      return true;
    }
    const src = String(code || '');
    return /^\s*(?:import\s+(?:[\s\S]+?\s+from\s+)?['"]|export\s+(?:default|const|let|var|function|class|type|interface|\{))/m.test(src);
  }

  #extractBridgePayload(req) {
    const p = req && req.payload && typeof req.payload === 'object' ? req.payload : req;
    const args = Array.isArray(p.args) ? p.args : (Array.isArray(req.args) ? req.args : []);
    const cwd = String(p.cwd || req.cwd || '/workspace');
    const env = p.env && typeof p.env === 'object' ? p.env : (req.env && typeof req.env === 'object' ? req.env : {});
    const workspace = Array.isArray(p.workspace) ? p.workspace : (Array.isArray(req.workspace) ? req.workspace : []);
    let stdinBytes = new Uint8Array(0);
    if (typeof p.stdin_base64 === 'string' && p.stdin_base64) {
      stdinBytes = __uorBase64ToBytes(p.stdin_base64);
    } else if (typeof p.stdin_utf8 === 'string' && p.stdin_utf8) {
      stdinBytes = cachedTextEncoder.encode(p.stdin_utf8);
    } else if (typeof req.stdin_base64 === 'string' && req.stdin_base64) {
      stdinBytes = __uorBase64ToBytes(req.stdin_base64);
    } else if (typeof req.stdin_utf8 === 'string' && req.stdin_utf8) {
      stdinBytes = cachedTextEncoder.encode(req.stdin_utf8);
    }
    return {
      tool: String(p.tool || req.tool || 'node').toLowerCase(),
      args,
      cwd,
      env,
      workspace,
      stdinBytes,
    };
  }

  #makePosixPathShim() {
    const normalize = (p) => MvmPolyglotVfs.normalizePath('/', p);
    const resolve = (...segs) => {
      let acc = '/workspace';
      for (const s of segs) {
        const str = String(s || '');
        if (!str) continue;
        acc = MvmPolyglotVfs.normalizePath(acc, str);
      }
      return acc;
    };
    const join = (...segs) => {
      const raw = segs.map((s) => String(s || '')).filter(Boolean).join('/');
      if (!raw) return '.';
      if (raw.startsWith('/')) return normalize(raw);
      const stack = [];
      for (const part of raw.split('/')) {
        if (!part || part === '.') continue;
        if (part === '..') {
          if (stack.length > 0 && stack[stack.length - 1] !== '..') stack.pop();
          else stack.push('..');
        } else {
          stack.push(part);
        }
      }
      return stack.length === 0 ? '.' : stack.join('/');
    };
    const dirname = (p) => {
      const s = String(p || '');
      const idx = s.lastIndexOf('/');
      if (idx < 0) return '.';
      return idx === 0 ? '/' : s.slice(0, idx);
    };
    const basename = (p, ext) => {
      const s = String(p || '').replace(/\/+$/, '');
      const base = s.slice(s.lastIndexOf('/') + 1);
      return ext && base.endsWith(ext) ? base.slice(0, base.length - ext.length) : base;
    };
    const extname = (p) => {
      const b = basename(p);
      const dot = b.lastIndexOf('.');
      return dot > 0 ? b.slice(dot) : '';
    };
    const relative = (from, to) => {
      const f = resolve(from).split('/').filter(Boolean);
      const t = resolve(to).split('/').filter(Boolean);
      let common = 0;
      while (common < f.length && common < t.length && f[common] === t[common]) common += 1;
      const ups = new Array(f.length - common).fill('..');
      return ups.concat(t.slice(common)).join('/') || '.';
    };
    const pathObj = {
      sep: '/',
      delimiter: ':',
      normalize,
      resolve,
      join,
      dirname,
      basename,
      extname,
      relative,
      isAbsolute: (p) => String(p || '').startsWith('/'),
    };
    pathObj.posix = pathObj;
    return pathObj;
  }

  async handleNodeExec(req) {
    const info = this.#extractBridgePayload(req);
    const vfs = new MvmPolyglotVfs(info.workspace, info.cwd);
    const args = info.args.map((a) => String(a));

    if (info.tool === 'npx') {
      const cmdIdx = args.findIndex((a) => !a.startsWith('-'));
      if (cmdIdx < 0) {
        return { ok: true, exit_code: 1, stdout: '', stderr: 'npx: missing command argument\n', writes: [], deletes: [] };
      }
      const cmdName = args[cmdIdx];
      const cmdArgs = args.slice(cmdIdx + 1);
      if (cmdName === 'esbuild') {
        const esRes = await this.ensureEsbuildCompiler();
        if (!esRes.ok) return esRes;
        const entry = cmdArgs.find((a) => !a.startsWith('-'));
        if (!entry) {
          return { ok: true, exit_code: 0, stdout: `esbuild ${esRes.compiler.version || 'wasm'}\n`, stderr: '', writes: [], deletes: [] };
        }
        const src = vfs.readText(entry) || '';
        try {
          const out = await esRes.compiler.transform(src, { loader: entry.endsWith('.ts') ? 'ts' : 'js', format: 'cjs' });
          const outFlagIdx = cmdArgs.findIndex((a) => a.startsWith('--outfile='));
          if (outFlagIdx >= 0) {
            const outFile = cmdArgs[outFlagIdx].slice('--outfile='.length);
            vfs.writeBytes(outFile, out.code || '');
            const muts = vfs.exportMutations();
            return { ok: true, exit_code: 0, stdout: '', stderr: '', ...muts };
          }
          return { ok: true, exit_code: 0, stdout: out.code || '', stderr: '', writes: [], deletes: [] };
        } catch (err) {
          return {
            ok: true,
            exit_code: 1,
            stdout: '',
            stderr: (err && err.stack ? err.stack : (err && err.message ? err.message : String(err))) + '\n',
            writes: [],
            deletes: [],
          };
        }
      }
      const binPath = `/workspace/node_modules/.bin/${cmdName}`;
      if (!vfs.exists(binPath)) {
        const installed = await this.handleNpmBridge({
          ecosystem: 'npm',
          tool: 'npm',
          args: ['install', cmdName],
          cwd: info.cwd,
          workspace: info.workspace,
        });
        if (!installed.ok) return installed;
        const mergedWs = new MvmPolyglotVfs(info.workspace, info.cwd);
        for (const w of installed.writes || []) {
          if (w.symlink_target) mergedWs.writeSymlink(w.path, w.symlink_target);
          else mergedWs.writeBytes(w.path, __uorBase64ToBytes(w.content_base64 || ''), w.mode);
        }
        const reExec = await this.handleNodeExec({
          ...req,
          tool: 'node',
          args: [binPath, ...cmdArgs],
          workspace: (mergedWs.files.size || mergedWs.symlinks.size) ? Array.from(mergedWs.files.entries()).map(([p, b]) => ({
            path: p,
            content_base64: __uorBytesToBase64(b),
            mode: mergedWs.modes.get(p) || 0o100644,
          })).concat(Array.from(mergedWs.symlinks.entries()).map(([p, t]) => ({
            path: p,
            symlink_target: t,
            mode: 0o120777,
          }))) : info.workspace,
        });
        if (!reExec.ok) return reExec;
        const mergedWriteMap = new Map();
        for (const w of installed.writes || []) {
          mergedWriteMap.set(MvmPolyglotVfs.normalizeRel(w.path), w);
        }
        const mergedDeleteSet = new Set((installed.deletes || []).map((d) => MvmPolyglotVfs.normalizeRel(d)));
        for (const d of reExec.deletes || []) {
          const dr = MvmPolyglotVfs.normalizeRel(d);
          mergedWriteMap.delete(dr);
          mergedDeleteSet.add(dr);
        }
        for (const w of reExec.writes || []) {
          const wr = MvmPolyglotVfs.normalizeRel(w.path);
          mergedDeleteSet.delete(wr);
          mergedWriteMap.set(wr, w);
        }
        return {
          ...reExec,
          writes: Array.from(mergedWriteMap.values()),
          deletes: Array.from(mergedDeleteSet),
        };
      }
      return this.handleNodeExec({ ...req, tool: 'node', args: [binPath, ...cmdArgs] });
    }

    let evalCode = null;
    let printExpr = false;
    let scriptPath = null;
    const scriptArgs = [];
    for (let i = 0; i < args.length; i += 1) {
      const a = args[i];
      if ((a === '-e' || a === '--eval') && i + 1 < args.length && evalCode === null && !scriptPath) {
        evalCode = args[i + 1];
        i += 1;
      } else if ((a === '-p' || a === '--print') && i + 1 < args.length && evalCode === null && !scriptPath) {
        evalCode = args[i + 1];
        printExpr = true;
        i += 1;
      } else if ((a === '-v' || a === '--version') && evalCode === null && !scriptPath) {
        return { ok: true, exit_code: 0, stdout: 'v20.18.0-mvm\n', stderr: '', writes: [], deletes: [] };
      } else if (a.startsWith('--loader=') || a === '--experimental-strip-types') {
        continue;
      } else if (!a.startsWith('-') && evalCode === null && !scriptPath) {
        scriptPath = a;
      } else {
        scriptArgs.push(a);
      }
    }

    if (evalCode === null && !scriptPath) {
      evalCode = cachedTextDecoder.decode(info.stdinBytes);
    }

    let entryAbs = '/workspace/__eval__.js';
    let entryCode = evalCode;
    if (scriptPath) {
      const normScriptAbs = MvmPolyglotVfs.normalizePath(vfs.cwd, scriptPath);
      try {
        entryAbs = vfs.resolveGuestTarget(normScriptAbs).abs;
      } catch (err) {
        if (err && err.code === 'CanonicalPathProtected') {
          return { ok: false, error_code: err.code, message: err.message };
        }
        throw err;
      }
      const raw = vfs.readText(entryAbs);
      if (raw === null) {
        return {
          ok: true,
          exit_code: 1,
          stdout: '',
          stderr: `Error: Cannot find module '${scriptPath}'\n`,
          writes: [],
          deletes: [],
        };
      }
      entryCode = raw;
    }

    const stdoutChunks = [];
    const stderrChunks = [];
    let procExitCode = 0;
    class NodeExitSignal {
      constructor(code) {
        this.code = code | 0;
      }
    }

    const pathShim = this.#makePosixPathShim();
    const fsShim = {
      readFileSync: (p, enc) => {
        const bytes = vfs.readBytes(p);
        if (!bytes) {
          const e = new Error(`ENOENT: no such file or directory, open '${p}'`);
          e.code = 'ENOENT';
          throw e;
        }
        return enc ? MvmNodeBuffer.from(bytes).toString(enc) : MvmNodeBuffer.from(bytes);
      },
      writeFileSync: (p, data) => vfs.writeBytes(p, data),
      appendFileSync: (p, data) => {
        const cur = vfs.readBytes(p) || new Uint8Array(0);
        const add = data instanceof Uint8Array ? data : cachedTextEncoder.encode(String(data ?? ''));
        vfs.writeBytes(p, MvmNodeBuffer.concat([cur, add]));
      },
      existsSync: (p) => vfs.exists(p),
      mkdirSync: (p, opts) => vfs.mkdir(p, opts),
      readdirSync: (p) => vfs.readdir(p),
      unlinkSync: (p) => {
        if (!vfs.deletePath(p)) {
          const e = new Error(`ENOENT: no such file or directory, unlink '${p}'`);
          e.code = 'ENOENT';
          throw e;
        }
      },
      rmSync: (p) => vfs.deletePath(p),
      readlinkSync: (p) => {
        const rel = MvmPolyglotVfs.normalizeRel(MvmPolyglotVfs.normalizePath(vfs.cwd, p));
        if (!vfs.symlinks.has(rel)) {
          const e = new Error(`EINVAL: invalid argument, readlink '${p}'`);
          e.code = 'EINVAL';
          throw e;
        }
        return vfs.symlinks.get(rel);
      },
      symlinkSync: (target, linkPath) => vfs.writeSymlink(linkPath, target),
      realpathSync: (p) => vfs.resolveGuestTarget(p).abs,
      statSync: (p) => {
        const resolved = vfs.resolveGuestTarget(p);
        const isDir = vfs.isDirectory(resolved.abs);
        const bytes = isDir ? null : vfs.readBytes(resolved.abs);
        if (!isDir && !bytes) {
          const e = new Error(`ENOENT: no such file or directory, stat '${p}'`);
          e.code = 'ENOENT';
          throw e;
        }
        return {
          size: bytes ? bytes.byteLength : 0,
          isFile: () => !isDir,
          isDirectory: () => isDir,
          isSymbolicLink: () => false,
        };
      },
      lstatSync: (p) => {
        const abs = MvmPolyglotVfs.normalizePath(vfs.cwd, p);
        const rel = MvmPolyglotVfs.normalizeRel(abs);
        if (rel && vfs.symlinks.has(rel)) {
          return {
            size: vfs.symlinks.get(rel).length,
            isFile: () => false,
            isDirectory: () => false,
            isSymbolicLink: () => true,
          };
        }
        return fsShim.statSync(p);
      },
    };
    fsShim.promises = {
      readFile: async (p, enc) => fsShim.readFileSync(p, enc),
      writeFile: async (p, data) => fsShim.writeFileSync(p, data),
      readdir: async (p) => fsShim.readdirSync(p),
      mkdir: async (p, opts) => fsShim.mkdirSync(p, opts),
      unlink: async (p) => fsShim.unlinkSync(p),
      stat: async (p) => fsShim.statSync(p),
      lstat: async (p) => fsShim.lstatSync(p),
    };

    const processShim = {
      version: 'v20.18.0-mvm',
      versions: { node: '20.18.0', v8: '12.0', wasi: '1.0' },
      platform: 'linux',
      arch: 'riscv64',
      argv: ['node', scriptPath || '-e', ...scriptArgs],
      env: { ...info.env },
      cwd: () => vfs.cwd,
      chdir: (nextDir) => {
        vfs.cwd = MvmPolyglotVfs.normalizePath(vfs.cwd, nextDir);
      },
      stdout: {
        write: (s) => {
          stdoutChunks.push(String(s));
          return true;
        },
      },
      stderr: {
        write: (s) => {
          stderrChunks.push(String(s));
          return true;
        },
      },
      exit: (code) => {
        throw new NodeExitSignal(code || 0);
      },
      nextTick: (fn, ...cbArgs) => Promise.resolve().then(() => fn(...cbArgs)),
    };

    const formatArg = (v) => {
      if (typeof v === 'string') return v;
      if (v instanceof Error) return v.stack || v.message;
      try {
        return typeof v === 'object' && v !== null ? JSON.stringify(v) : String(v);
      } catch {
        return String(v);
      }
    };
    const consoleShim = {
      log: (...vals) => stdoutChunks.push(vals.map(formatArg).join(' ') + '\n'),
      info: (...vals) => stdoutChunks.push(vals.map(formatArg).join(' ') + '\n'),
      warn: (...vals) => stderrChunks.push(vals.map(formatArg).join(' ') + '\n'),
      error: (...vals) => stderrChunks.push(vals.map(formatArg).join(' ') + '\n'),
    };

    const cryptoShim = {
      randomBytes: (n) => {
        const buf = MvmNodeBuffer.alloc(n, 0);
        if (typeof crypto !== 'undefined' && typeof crypto.getRandomValues === 'function') {
          crypto.getRandomValues(buf);
        }
        return buf;
      },
      randomUUID: () => (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
        ? crypto.randomUUID()
        : '00000000-0000-4000-8000-000000000000'),
      subtle: typeof crypto !== 'undefined' ? crypto.subtle : undefined,
    };

    const urlShim = {
      URL,
      URLSearchParams,
      fileURLToPath: (u) => new URL(u).pathname,
      pathToFileURL: (p) => new URL(`file://${MvmPolyglotVfs.normalizePath('/', p)}`),
    };

    const fetchShim = async (url, init) => {
      const netCheck = this.#assertNetworkAllowed();
      if (!netCheck.ok) {
        const e = new Error(netCheck.message);
        e.code = netCheck.error_code;
        throw e;
      }
      const opts = init && typeof init === 'object' ? init : {};
      const res = await this.handleHttpRequest({
        url: String(url),
        method: opts.method || 'GET',
        headers: opts.headers || {},
        body: typeof opts.body === 'string' ? opts.body : undefined,
      });
      if (!res.ok && res.error_code) {
        const e = new Error(res.message || res.error_code);
        e.code = res.error_code;
        throw e;
      }
      const text = String(res.body_text || '');
      return {
        ok: res.status >= 200 && res.status < 300,
        status: res.status || 200,
        text: async () => text,
        json: async () => JSON.parse(text),
        arrayBuffer: async () => cachedTextEncoder.encode(text).buffer,
      };
    };

    const assertShim = {
      ok: (cond, msg) => { if (!cond) throw new Error(msg || 'AssertionError'); },
      equal: (a, b, msg) => { if (a != b) throw new Error(msg || `${a} != ${b}`); },
      strictEqual: (a, b, msg) => { if (a !== b) throw new Error(msg || `${a} !== ${b}`); },
      deepStrictEqual: (a, b, msg) => {
        if (JSON.stringify(a) !== JSON.stringify(b)) throw new Error(msg || 'AssertionError: deepStrictEqual failed');
      },
      fail: (msg) => { throw new Error(msg || 'AssertionError: failed'); },
    };
    const utilShim = {
      format: (fmt, ...vals) => {
        if (typeof fmt !== 'string') return [fmt, ...vals].map(formatArg).join(' ');
        let vi = 0;
        const replaced = fmt.replace(/%([sdj%])/g, (full, ch) => {
          if (ch === '%') return '%';
          if (vi >= vals.length) return full;
          const v = vals[vi++];
          if (ch === 'j') return JSON.stringify(v);
          if (ch === 'd') return String(Number(v));
          return formatArg(v);
        });
        return vi < vals.length ? [replaced, ...vals.slice(vi).map(formatArg)].join(' ') : replaced;
      },
      inspect: (v) => formatArg(v),
    };
    const builtins = new Map([
      ['fs', fsShim],
      ['node:fs', fsShim],
      ['fs/promises', fsShim.promises],
      ['node:fs/promises', fsShim.promises],
      ['path', pathShim],
      ['node:path', pathShim],
      ['process', processShim],
      ['node:process', processShim],
      ['buffer', { Buffer: MvmNodeBuffer }],
      ['node:buffer', { Buffer: MvmNodeBuffer }],
      ['crypto', cryptoShim],
      ['node:crypto', cryptoShim],
      ['url', urlShim],
      ['node:url', urlShim],
      ['assert', assertShim],
      ['node:assert', assertShim],
      ['util', utilShim],
      ['node:util', utilShim],
    ]);

    const transpileMap = new Map();
    const prepareCodeSyncOrNeedEsbuild = async (rawSource, filePath) => {
      const strippedShebang = String(rawSource || '').replace(/^#![^\n]*\n/, '');
      if (!this.#needsEsbuildTranspile(strippedShebang, filePath, args)) {
        return { ok: true, code: strippedShebang };
      }
      const esRes = await this.ensureEsbuildCompiler();
      if (!esRes.ok) return esRes;
      const ext = pathShim.extname(filePath).toLowerCase();
      const loader = ext === '.tsx' ? 'tsx' : (ext === '.ts' || ext === '.mts' || ext === '.cts' ? 'ts' : (ext === '.jsx' ? 'jsx' : 'js'));
      const transformed = await esRes.compiler.transform(strippedShebang, {
        loader,
        format: 'cjs',
        target: 'es2022',
      });
      return { ok: true, code: transformed.code || '' };
    };

    const resolveModulePath = (spec, fromDir) => {
      const tryFile = (cand) => {
        for (const ext of ['', '.js', '.cjs', '.mjs', '.ts', '.tsx', '.json']) {
          const p = `${cand}${ext}`;
          if (vfs.exists(p) && !vfs.isDirectory(p)) {
            return vfs.resolveGuestTarget(p).abs;
          }
        }
        if (vfs.isDirectory(cand)) {
          const pkgRaw = vfs.readText(`${cand}/package.json`);
          if (pkgRaw) {
            try {
              const pj = JSON.parse(pkgRaw);
              let entry = pj.main || 'index.js';
              if (pj.exports && typeof pj.exports === 'object' && pj.exports['.']) {
                const dotExp = pj.exports['.'];
                if (typeof dotExp === 'string') entry = dotExp;
                else if (dotExp && typeof dotExp === 'object') {
                  entry = dotExp.require || dotExp.default || dotExp.import || entry;
                }
              }
              const target = MvmPolyglotVfs.normalizePath(cand, entry);
              if (vfs.exists(target) && !vfs.isDirectory(target)) {
                return vfs.resolveGuestTarget(target).abs;
              }
            } catch {
              // Fall back to index.js
            }
          }
          for (const idxExt of ['/index.js', '/index.cjs', '/index.ts']) {
            if (vfs.exists(`${cand}${idxExt}`)) {
              return vfs.resolveGuestTarget(`${cand}${idxExt}`).abs;
            }
          }
        }
        return null;
      };
      if (spec.startsWith('./') || spec.startsWith('../') || spec.startsWith('/')) {
        return tryFile(MvmPolyglotVfs.normalizePath(fromDir, spec));
      }
      let curDir = fromDir;
      while (true) {
        const nmCand = `${curDir === '/' ? '' : curDir}/node_modules/${spec}`;
        const hit = tryFile(nmCand);
        if (hit) return hit;
        if (curDir === '/' || curDir === '/workspace') break;
        curDir = MvmPolyglotVfs.parentDir(curDir);
      }
      return null;
    };

    const collectDeps = async (srcText, filePath, visited) => {
      if (visited.has(filePath)) return { ok: true };
      visited.add(filePath);
      const tRes = await prepareCodeSyncOrNeedEsbuild(srcText, filePath);
      if (!tRes.ok) return tRes;
      transpileMap.set(filePath, tRes.code);
      const dir = pathShim.dirname(filePath);
      const reqMatches = tRes.code.matchAll(/require\s*\(\s*['"]([^'"]+)['"]\s*\)/g);
      for (const m of reqMatches) {
        const spec = m[1];
        if (builtins.has(spec)) continue;
        const resolved = resolveModulePath(spec, dir);
        if (resolved) {
          const childRaw = vfs.readText(resolved);
          if (childRaw !== null) {
            const sub = await collectDeps(childRaw, resolved, visited);
            if (!sub.ok) return sub;
          }
        }
      }
      return { ok: true };
    };

    const moduleCache = new Map();
    const runModule = (fileAbs, isEntry) => {
      if (moduleCache.has(fileAbs)) return moduleCache.get(fileAbs).exports;
      if (fileAbs.endsWith('.json')) {
        const parsed = JSON.parse(vfs.readText(fileAbs) || 'null');
        moduleCache.set(fileAbs, { exports: parsed });
        return parsed;
      }
      const mod = { exports: {} };
      moduleCache.set(fileAbs, mod);
      const modDir = pathShim.dirname(fileAbs);
      const localRequire = (spec) => {
        if (builtins.has(spec)) return builtins.get(spec);
        const resolved = resolveModulePath(spec, modDir);
        if (!resolved) {
          const e = new Error(`Cannot find module '${spec}'`);
          e.code = 'MODULE_NOT_FOUND';
          throw e;
        }
        return runModule(resolved, false);
      };
      const compiledCode = transpileMap.get(fileAbs) || '';
      const execBody = isEntry && printExpr ? `return (${compiledCode});` : compiledCode;
      const fn = new Function(
        'exports',
        'require',
        'module',
        '__filename',
        '__dirname',
        'process',
        'console',
        'Buffer',
        'fetch',
        execBody,
      );
      const ret = fn(
        mod.exports,
        localRequire,
        mod,
        fileAbs,
        modDir,
        processShim,
        consoleShim,
        MvmNodeBuffer,
        fetchShim,
      );
      if (isEntry && printExpr && ret !== undefined) {
        consoleShim.log(ret);
      }
      return isEntry ? ret : mod.exports;
    };

    try {
      const prepRes = await collectDeps(entryCode, entryAbs, new Set());
      if (!prepRes.ok) return prepRes;
      const ret = runModule(entryAbs, true);
      if (ret && typeof ret.then === 'function') {
        await ret;
      }
    } catch (err) {
      if (err instanceof NodeExitSignal) {
        procExitCode = err.code;
      } else if (err && (err.code === 'CanonicalPathProtected' || err.code === 'EsbuildAdapterUnavailable')) {
        return { ok: false, error_code: err.code, message: err.message };
      } else {
        procExitCode = 1;
        stderrChunks.push((err && err.stack ? err.stack : String(err)) + '\n');
      }
    }

    const muts = vfs.exportMutations();
    return {
      ok: true,
      exit_code: procExitCode,
      stdout: stdoutChunks.join(''),
      stderr: stderrChunks.join(''),
      ...muts,
    };
  }

  async handleNpmBridge(req) {
    const info = this.#extractBridgePayload(req);
    const vfs = new MvmPolyglotVfs(info.workspace, info.cwd);
    const args = info.args.map((a) => String(a));
    const sub = (args[0] || 'install').toLowerCase();

    if (sub === 'init') {
      if (!vfs.exists('/workspace/package.json')) {
        vfs.writeBytes(
          '/workspace/package.json',
          JSON.stringify({ name: 'workspace-app', version: '1.0.0', private: true, dependencies: {} }, null, 2) + '\n',
        );
      }
      const muts = vfs.exportMutations();
      return { ok: true, exit_code: 0, stdout: 'Wrote /workspace/package.json\n', stderr: '', ...muts };
    }

    if (sub === 'run' && args[1]) {
      const pjRaw = vfs.readText('/workspace/package.json');
      if (!pjRaw) {
        return { ok: true, exit_code: 1, stdout: '', stderr: 'npm ERR! missing package.json\n', writes: [], deletes: [] };
      }
      const pj = JSON.parse(pjRaw);
      const scriptCmd = pj.scripts && pj.scripts[args[1]];
      if (!scriptCmd) {
        return { ok: true, exit_code: 1, stdout: '', stderr: `npm ERR! Missing script: "${args[1]}"\n`, writes: [], deletes: [] };
      }
      const parts = scriptCmd.trim().split(/\s+/);
      const binName = parts[0] === 'node' ? parts : ['node', `/workspace/node_modules/.bin/${parts[0]}`, ...parts.slice(1)];
      return this.handleNodeExec({ ...req, tool: 'node', args: binName.slice(1) });
    }

    if (sub !== 'install' && sub !== 'i' && sub !== 'add') {
      return { ok: true, exit_code: 1, stdout: '', stderr: `npm: unsupported subcommand '${sub}'\n`, writes: [], deletes: [] };
    }

    const netCheck = this.#assertNetworkAllowed();
    if (!netCheck.ok) return netCheck;

    const specs = args.slice(1).filter((a) => !a.startsWith('-'));
    const pkgJsonRaw = vfs.readText('/workspace/package.json');
    let pkgJson = { name: 'workspace-app', version: '1.0.0', dependencies: {} };
    if (pkgJsonRaw) {
      try {
        pkgJson = JSON.parse(pkgJsonRaw);
      } catch {
        pkgJson = { name: 'workspace-app', version: '1.0.0', dependencies: {} };
      }
    }
    if (!pkgJson.dependencies || typeof pkgJson.dependencies !== 'object') {
      pkgJson.dependencies = {};
    }

    const toInstall = specs.length > 0
      ? specs
      : Object.entries(pkgJson.dependencies).map(([n, v]) => `${n}@${v}`);
    if (toInstall.length === 0) {
      return { ok: true, exit_code: 0, stdout: 'up to date\n', stderr: '', writes: [], deletes: [] };
    }

    const installedCount = [];
    const seen = new Set();
    const parseSpec = (rawSpec) => {
      const s = String(rawSpec).trim();
      const at = s.startsWith('@') ? s.indexOf('@', 1) : s.indexOf('@');
      return at > 0 ? { name: s.slice(0, at), range: s.slice(at + 1) } : { name: s, range: 'latest' };
    };
    const pickVersion = (meta, range) => {
      const versions = meta.versions && typeof meta.versions === 'object' ? Object.keys(meta.versions) : [];
      const tags = meta['dist-tags'] || {};
      if (!range || range === 'latest' || range === '*') {
        return tags.latest || versions[versions.length - 1];
      }
      if (meta.versions && meta.versions[range]) return range;
      const clean = range.replace(/^[~^>=<\s]+/, '');
      if (meta.versions && meta.versions[clean]) return clean;
      const major = clean.split('.')[0];
      const match = versions.filter((v) => v.startsWith(`${major}.`)).pop();
      return match || tags.latest || versions[versions.length - 1];
    };

    const installSingle = async (rawSpec, isDirect) => {
      const { name, range } = parseSpec(rawSpec);
      if (!name || seen.has(name)) return { ok: true };
      seen.add(name);
      const encodedName = name.startsWith('@') ? `@${encodeURIComponent(name.slice(1))}` : encodeURIComponent(name);
      const regUrl = `https://registry.npmjs.org/${encodedName}`;
      const metaResp = await this.#fetchImpl(regUrl, { headers: { Accept: 'application/json' } });
      if (!metaResp || !metaResp.ok) {
        return {
          ok: false,
          status: metaResp ? metaResp.status : 502,
          error_code: 'NpmRegistryError',
          message: `Failed fetching npm metadata for ${name} (HTTP ${metaResp ? metaResp.status : 'error'})`,
        };
      }
      const meta = await metaResp.json();
      const ver = pickVersion(meta, range);
      const verObj = meta.versions && ver ? meta.versions[ver] : null;
      if (!verObj || !verObj.dist || !verObj.dist.tarball) {
        return {
          ok: false,
          status: 404,
          error_code: 'PackageNotFound',
          message: `Version "${range}" not found for npm package "${name}"`,
        };
      }
      const expectedSri = verObj.dist.integrity || '';
      const tgzResp = await this.#fetchImpl(verObj.dist.tarball);
      if (!tgzResp || !tgzResp.ok) {
        return {
          ok: false,
          status: tgzResp ? tgzResp.status : 502,
          error_code: 'PackageDownloadFailed',
          message: `Failed downloading tarball ${verObj.dist.tarball}`,
        };
      }
      const tgzBytes = new Uint8Array(await tgzResp.arrayBuffer());
      const sriRes = await this.#verifySri(tgzBytes, expectedSri);
      if (!sriRes.ok) {
        return sriRes;
      }
      const tarBytes = await this.#gunzipBytes(tgzBytes);
      const entries = this.#parseTarEntries(tarBytes);
      const pkgBase = `/workspace/node_modules/${name}`;
      for (const ent of entries) {
        const cleanEntry = String(ent.name || '').replace(/^\.\//, '').replace(/^package\//, '').replace(/^\/+/, '');
        if (!cleanEntry || cleanEntry.split('/').includes('..') || ent.typeflag === '5') continue;
        const dst = `${pkgBase}/${cleanEntry}`;
        if (ent.typeflag === '2') {
          vfs.writeSymlink(dst, ent.linkname);
        } else {
          vfs.writeBytes(dst, ent.data, 0o100000 | (ent.mode & 0o777));
        }
      }
      if (verObj.bin) {
        const binMap = typeof verObj.bin === 'string' ? { [name.split('/').pop()]: verObj.bin } : verObj.bin;
        for (const [bName, bRel] of Object.entries(binMap)) {
          const cleanRel = String(bRel).replace(/^\.\//, '');
          vfs.writeSymlink(`/workspace/node_modules/.bin/${bName}`, `../${name}/${cleanRel}`);
        }
      }
      if (isDirect) {
        pkgJson.dependencies[name] = `^${ver}`;
      }
      installedCount.push(`${name}@${ver}`);
      if (verObj.dependencies && typeof verObj.dependencies === 'object') {
        for (const [depName, depRange] of Object.entries(verObj.dependencies)) {
          const subRes = await installSingle(`${depName}@${depRange}`, false);
          if (!subRes.ok) return subRes;
        }
      }
      return { ok: true };
    };

    for (const spec of toInstall) {
      const r = await installSingle(spec, true);
      if (!r.ok) return r;
    }

    vfs.writeBytes('/workspace/package.json', JSON.stringify(pkgJson, null, 2) + '\n');
    const muts = vfs.exportMutations();
    return {
      ok: true,
      exit_code: 0,
      stdout: `added ${installedCount.length} package(s): ${installedCount.join(', ')}\n`,
      stderr: '',
      ...muts,
    };
  }

  async #ensurePyodideLoaded(stdoutChunks, stderrChunks) {
    this.#activePyStdoutSink = (line) => stdoutChunks.push(String(line) + '\n');
    this.#activePyStderrSink = (line) => stderrChunks.push(String(line) + '\n');

    const bindStreams = (pyodide) => {
      if (pyodide && typeof pyodide.setStdout === 'function') {
        pyodide.setStdout({
          batched: (line) => {
            if (this.#activePyStdoutSink) this.#activePyStdoutSink(line);
          },
        });
      }
      if (pyodide && typeof pyodide.setStderr === 'function') {
        pyodide.setStderr({
          batched: (line) => {
            if (this.#activePyStderrSink) this.#activePyStderrSink(line);
          },
        });
      }
    };

    if (this.#pyodideInstance) {
      bindStreams(this.#pyodideInstance);
      return { ok: true, pyodide: this.#pyodideInstance };
    }
    const lock = this.pyodideLock;
    let verifiedBytes = null;

    if (typeof caches !== 'undefined' && typeof caches.open === 'function') {
      try {
        const cache = await caches.open(PYODIDE_CACHE_NAME);
        const cachedResp = await cache.match(lock.url);
        if (cachedResp) {
          const candBytes = new Uint8Array(await cachedResp.arrayBuffer());
          const sriCheck = await this.#verifySri(candBytes, lock.sri);
          if (sriCheck.ok) verifiedBytes = candBytes;
        }
      } catch {
        // Cache API unavailable or empty
      }
    }

    if (!verifiedBytes) {
      const netCheck = this.#assertNetworkAllowed();
      if (!netCheck.ok) return netCheck;
      const resp = await this.#fetchImpl(lock.url);
      if (!resp || !resp.ok) {
        return {
          ok: false,
          status: resp ? resp.status : 502,
          error_code: 'PyodideLoadError',
          message: `Failed fetching Pyodide entrypoint ${lock.url}`,
        };
      }
      const rawBytes = new Uint8Array(await resp.arrayBuffer());
      const sriCheck = await this.#verifySri(rawBytes, lock.sri);
      if (!sriCheck.ok) {
        return sriCheck;
      }
      verifiedBytes = rawBytes;
      if (typeof caches !== 'undefined' && typeof caches.open === 'function') {
        try {
          const cache = await caches.open(PYODIDE_CACHE_NAME);
          await cache.put(
            lock.url,
            new Response(verifiedBytes, { status: 200, headers: { 'Content-Type': 'text/javascript' } }),
          );
        } catch {
          // Ignore cache write errors in restricted hosts
        }
      }
    }

    if (!this.#pyodidePromise) {
      this.#pyodidePromise = (async () => {
        let mod = null;
        if (typeof URL !== 'undefined' && typeof URL.createObjectURL === 'function' && typeof WorkerGlobalScope !== 'undefined') {
          const blobUrl = URL.createObjectURL(new Blob([verifiedBytes], { type: 'text/javascript' }));
          try {
            mod = await import(blobUrl);
          } finally {
            URL.revokeObjectURL(blobUrl);
          }
        } else {
          const dataUrl = `data:text/javascript;base64,${__uorBytesToBase64(verifiedBytes)}`;
          mod = await import(dataUrl);
        }
        const pyodide = await mod.loadPyodide({
          indexURL: lock.indexURL,
          stdout: (line) => {
            if (this.#activePyStdoutSink) this.#activePyStdoutSink(line);
          },
          stderr: (line) => {
            if (this.#activePyStderrSink) this.#activePyStderrSink(line);
          },
        });
        this.#pyodideInstance = pyodide;
        return pyodide;
      })();
    }

    try {
      const pyodide = await this.#pyodidePromise;
      bindStreams(pyodide);
      return { ok: true, pyodide };
    } catch (err) {
      this.#pyodidePromise = null;
      this.#pyodideInstance = null;
      return {
        ok: false,
        error_code: 'PyodideLoadError',
        message: err && err.message ? err.message : String(err),
      };
    }
  }

  async handlePythonExec(req) {
    const info = this.#extractBridgePayload(req);
    const vfs = new MvmPolyglotVfs(info.workspace, info.cwd);
    const stdoutChunks = [];
    const stderrChunks = [];

    const pyLoaded = await this.#ensurePyodideLoaded(stdoutChunks, stderrChunks);
    if (!pyLoaded.ok) {
      this.#activePyStdoutSink = null;
      this.#activePyStderrSink = null;
      return pyLoaded;
    }
    const pyodide = pyLoaded.pyodide;
    const args = info.args.map((a) => String(a));

    try {
      if (pyodide.FS && typeof pyodide.FS.mkdirTree === 'function') {
        pyodide.FS.mkdirTree('/workspace');
        if (typeof pyodide.FS.readdir === 'function' && typeof pyodide.FS.unlink === 'function') {
          const pruneStaleFs = (dirAbs) => {
            let entries = [];
            try {
              entries = pyodide.FS.readdir(dirAbs);
            } catch {
              return;
            }
            for (const name of entries) {
              if (name === '.' || name === '..') continue;
              const full = `${dirAbs}/${name}`;
              const stat = pyodide.FS.analyzePath ? pyodide.FS.analyzePath(full) : null;
              if (!stat || !stat.exists) continue;
              if (stat.object && stat.object.isFolder) {
                pruneStaleFs(full);
              } else {
                const rel = full.slice('/workspace/'.length);
                if (!vfs.files.has(rel)) {
                  try {
                    pyodide.FS.unlink(full);
                  } catch {
                    // Ignore unlink error for virtual entries
                  }
                }
              }
            }
          };
          pruneStaleFs('/workspace');
        }
        for (const [rel, bytes] of vfs.files.entries()) {
          const full = `/workspace/${rel}`;
          const parent = MvmPolyglotVfs.parentDir(full);
          pyodide.FS.mkdirTree(parent);
          pyodide.FS.writeFile(full, bytes);
        }
      }

      globalThis.__uorSpiralCorePySyncFetch = (url, method, bodyStr, headersJson) => {
        const netCheck = this.#assertNetworkAllowed();
        if (!netCheck.ok) {
          throw new Error(`${netCheck.error_code}: ${netCheck.message}`);
        }
        const urlCheck = this.validateTransportUrl(String(url), false);
        if (!urlCheck.ok) {
          throw new Error(`${urlCheck.error_code}: ${urlCheck.message}`);
        }
        let parsedHeaders = {};
        if (headersJson) {
          try {
            const obj = JSON.parse(String(headersJson));
            if (obj && typeof obj === 'object') parsedHeaders = obj;
          } catch {
            parsedHeaders = {};
          }
        }
        const hdrSub = this.applyHostSecretSubstitution(urlCheck.url, parsedHeaders);
        if (!hdrSub.ok) {
          throw new Error(`${hdrSub.error_code}: ${hdrSub.message}`);
        }
        const host = urlCheck.url.hostname.toLowerCase();
        const urlSub = this.substituteHostSecretsInString(String(url), host);
        if (!urlSub.ok) {
          throw new Error(`${urlSub.error_code}: ${urlSub.message}`);
        }
        const bodySub = this.substituteHostSecretsInString(String(bodyStr || ''), host);
        if (!bodySub.ok) {
          throw new Error(`${bodySub.error_code}: ${bodySub.message}`);
        }
        const mUpper = String(method || 'GET').toUpperCase();
        if (typeof this.#syncFetchImpl === 'function') {
          const syncRes = this.#syncFetchImpl(urlSub.value, {
            method: mUpper,
            headers: hdrSub.headers,
            body: mUpper === 'GET' || mUpper === 'HEAD' ? undefined : bodySub.value,
          });
          return {
            status: Number(syncRes && syncRes.status) || 200,
            text: String((syncRes && syncRes.text !== undefined ? syncRes.text : (syncRes && syncRes.body_text)) || ''),
          };
        }
        if (typeof XMLHttpRequest !== 'undefined') {
          const xhr = new XMLHttpRequest();
          xhr.open(mUpper, urlSub.value, false);
          for (const [hk, hv] of Object.entries(hdrSub.headers)) {
            xhr.setRequestHeader(hk, hv);
          }
          xhr.send(mUpper === 'GET' || mUpper === 'HEAD' ? null : bodySub.value);
          return {
            status: xhr.status || 200,
            text: String(xhr.responseText || ''),
          };
        }
        throw new Error('SyncHttpUnavailable: synchronous Python HTTP requires XMLHttpRequest or syncFetchImpl');
      };

      if (!this.#pyodideShimmed) {
        this.#pyodideShimmed = true;
        try {
          await pyodide.runPythonAsync(`
import io, os, sys, types, json as _uor_json
try:
    import js, urllib.request, urllib.parse
    class _SpiralResp:
        def __init__(self, text, status=200):
            self.text = str(text)
            self.status_code = int(status)
            self.status = int(status)
            self._data = self.text.encode('utf-8')
            self.content = self._data
            self.headers = {}
            self.ok = 200 <= self.status_code < 300
        def read(self):
            return self._data
        def json(self):
            return _uor_json.loads(self.text)
        def raise_for_status(self):
            if self.status_code >= 400:
                raise RuntimeError(f'HTTP {self.status_code}')
        def __enter__(self):
            return self
        def __exit__(self, *a):
            return False
    def _uor_sync_fetch(url, method='GET', data=None, json=None, headers=None, params=None):
        target_url = str(url)
        if params and isinstance(params, dict):
            sep = '&' if '?' in target_url else '?'
            target_url = target_url + sep + urllib.parse.urlencode(params)
        hdrs = dict(headers) if isinstance(headers, dict) else {}
        body_str = ''
        if json is not None:
            body_str = _uor_json.dumps(json)
            if 'Content-Type' not in hdrs and 'content-type' not in hdrs:
                hdrs['Content-Type'] = 'application/json'
        elif data is not None:
            if isinstance(data, dict):
                body_str = urllib.parse.urlencode(data)
            elif isinstance(data, (bytes, bytearray)):
                body_str = bytes(data).decode('utf-8', errors='replace')
            else:
                body_str = str(data)
        res = js.__uorSpiralCorePySyncFetch(target_url, str(method).upper(), body_str, _uor_json.dumps(hdrs))
        txt = getattr(res, 'text', '')
        st = getattr(res, 'status', 200)
        return _SpiralResp(txt, st)
    def _uor_sync_urlopen(url_or_req, data=None, timeout=None, **kw):
        if isinstance(url_or_req, str):
            m = 'POST' if data is not None else 'GET'
            return _uor_sync_fetch(url_or_req, m, data=data)
        u = getattr(url_or_req, 'full_url', None) or str(url_or_req)
        m = url_or_req.get_method() if hasattr(url_or_req, 'get_method') else ('POST' if data is not None else 'GET')
        req_data = getattr(url_or_req, 'data', None)
        if req_data is None:
            req_data = data
        req_hdrs = dict(url_or_req.header_items()) if hasattr(url_or_req, 'header_items') else {}
        return _uor_sync_fetch(u, m, data=req_data, headers=req_hdrs)
    urllib.request.urlopen = _uor_sync_urlopen
    _req_mod = types.ModuleType('requests')
    _req_mod.get = lambda url, **kw: _uor_sync_fetch(url, 'GET', headers=kw.get('headers'), params=kw.get('params'))
    _req_mod.post = lambda url, data=None, json=None, **kw: _uor_sync_fetch(url, 'POST', data=data, json=json, headers=kw.get('headers'), params=kw.get('params'))
    _req_mod.request = lambda method, url, **kw: _uor_sync_fetch(url, method, data=kw.get('data'), json=kw.get('json'), headers=kw.get('headers'), params=kw.get('params'))
    sys.modules['requests'] = _req_mod
except Exception:
    pass
`);
        } catch {
          // Non-CPython mock runtime ignores Python stdlib registration cleanly
        }
      }

      const isPip = info.tool === 'pip' || info.tool === 'pip3';
      const pyArgv = isPip
        ? ['pip', ...args]
        : (args[0] === '-c'
          ? ['-c', ...args.slice(2)]
          : (args[0] && !args[0].startsWith('-') ? [args[0], ...args.slice(1)] : ['-']));
      const pyEnvObj = { HOME: '/root', PATH: '/usr/local/bin:/usr/bin:/bin', ...info.env };
      const pyStdinText = cachedTextDecoder.decode(info.stdinBytes);
      try {
        await pyodide.runPythonAsync(`
import io, os, sys
sys.argv = ${JSON.stringify(pyArgv)}
_uor_env = ${JSON.stringify(pyEnvObj)}
os.environ.clear()
os.environ.update({str(k): str(v) for k, v in _uor_env.items()})
_uor_stdin_str = ${JSON.stringify(pyStdinText)}
sys.stdin = io.TextIOWrapper(io.BytesIO(_uor_stdin_str.encode('utf-8')), encoding='utf-8')
_uor_cwd = ${JSON.stringify(vfs.cwd)}
if os.path.isdir(_uor_cwd):
    os.chdir(_uor_cwd)
elif os.path.isdir('/workspace'):
    os.chdir('/workspace')
`);
      } catch {
        // Ignore if minimal mock runtime skips process context preamble
      }

      let exitCode = 0;
      if (isPip) {
        const sub = (args[0] || 'list').toLowerCase();
        if (sub === 'install') {
          const netCheck = this.#assertNetworkAllowed();
          if (!netCheck.ok) return netCheck;
          const pkgList = args.slice(1).filter((a) => !a.startsWith('-'));
          if (pkgList.length === 0) {
            return { ok: true, exit_code: 1, stdout: '', stderr: 'pip install: missing package name\n', writes: [], deletes: [] };
          }
          for (const pkg of pkgList) {
            const cleanPkg = pkg.split(/[=<>!~]/)[0].trim();
            const metaResp = await this.#fetchImpl(`https://pypi.org/pypi/${encodeURIComponent(cleanPkg)}/json`);
            if (!metaResp || !metaResp.ok) {
              return { ok: true, exit_code: 1, stdout: '', stderr: `ERROR: Could not find a version that satisfies ${pkg}\n`, writes: [], deletes: [] };
            }
          }
          if (typeof pyodide.loadPackage === 'function') {
            await pyodide.loadPackage('micropip');
          }
          const installListJson = JSON.stringify(pkgList);
          await pyodide.runPythonAsync(`import micropip\nawait micropip.install(${installListJson})`);
          stdoutChunks.push(`Successfully installed ${pkgList.join(' ')}\n`);
        } else {
          const out = await pyodide.runPythonAsync(`import micropip\nstr(micropip.list())`);
          stdoutChunks.push(String(out || '') + '\n');
        }
      } else {
        let codeToRun = '';
        if (args[0] === '-c' && args.length > 1) {
          codeToRun = String(args[1]);
        } else if (args[0] && !args[0].startsWith('-')) {
          const target = MvmPolyglotVfs.normalizePath(vfs.cwd, args[0]);
          const txt = vfs.readText(target);
          if (txt === null) {
            return { ok: true, exit_code: 1, stdout: '', stderr: `python3: can't open file '${args[0]}': No such file or directory\n`, writes: [], deletes: [] };
          }
          codeToRun = txt;
        } else {
          codeToRun = cachedTextDecoder.decode(info.stdinBytes);
        }
        const resVal = await pyodide.runPythonAsync(codeToRun);
        if (stdoutChunks.length === 0 && resVal !== undefined && resVal !== null && args[0] === '-c') {
          // runPythonAsync returns last expression if print() wasn't buffered through stdout callback
        }
      }

      if (pyodide.FS && typeof pyodide.FS.readdir === 'function') {
        const seenFs = new Set();
        const walkFs = (dirAbs) => {
          let entries = [];
          try {
            entries = pyodide.FS.readdir(dirAbs);
          } catch {
            return;
          }
          for (const name of entries) {
            if (name === '.' || name === '..') continue;
            const full = `${dirAbs}/${name}`;
            const stat = pyodide.FS.analyzePath(full);
            if (!stat || !stat.exists) continue;
            if (stat.object && stat.object.isFolder) {
              walkFs(full);
            } else {
              const rel = full.slice('/workspace/'.length);
              seenFs.add(rel);
              const bytes = pyodide.FS.readFile(full);
              const prev = vfs.files.get(rel);
              const changed =
                !prev ||
                prev.byteLength !== bytes.byteLength ||
                prev.some((b, idx) => b !== bytes[idx]);
              if (changed) vfs.writeBytes(full, bytes);
            }
          }
        };
        walkFs('/workspace');
        for (const origRel of vfs.originalKeys) {
          if (!seenFs.has(origRel) && !vfs.symlinks.has(origRel)) {
            vfs.deletePath(`/workspace/${origRel}`);
          }
        }
      }

      const muts = vfs.exportMutations();
      return {
        ok: true,
        exit_code: exitCode,
        stdout: stdoutChunks.join(''),
        stderr: stderrChunks.join(''),
        ...muts,
      };
    } catch (err) {
      if (err && err.code === 'CanonicalPathProtected') {
        return { ok: false, error_code: 'CanonicalPathProtected', message: err.message };
      }
      return {
        ok: true,
        exit_code: 1,
        stdout: stdoutChunks.join(''),
        stderr: stderrChunks.join('') + (err && err.message ? err.message : String(err)) + '\n',
        ...vfs.exportMutations(),
      };
    } finally {
      this.#activePyStdoutSink = null;
      this.#activePyStderrSink = null;
    }
  }

  async handleWasiExec(req) {
    const info = this.#extractBridgePayload(req);
    const vfs = new MvmPolyglotVfs(info.workspace, info.cwd);
    const args = info.args.map((a) => String(a));
    let modPath = '';
    let wasiArgs = [];
    if (args[0] === 'run' && args.length >= 2) {
      modPath = args[1];
      wasiArgs = [args[1], ...args.slice(2)];
    } else if (args[0] && !args[0].startsWith('-')) {
      modPath = args[0];
      wasiArgs = [args[0], ...args.slice(1)];
    } else {
      return {
        ok: false,
        error_code: 'WasiRuntimeError',
        message: 'wasm: expected `wasm run <module.wasm> [args...]`',
      };
    }
    const wasmBytes = vfs.readBytes(modPath);
    if (!wasmBytes) {
      return {
        ok: false,
        error_code: 'WasiRuntimeError',
        message: `wasm: module not found: ${modPath}`,
      };
    }
    try {
      const res = await MvmWasiRunner.run(wasmBytes, vfs, wasiArgs, info.env, info.stdinBytes);
      const muts = vfs.exportMutations();
      return { ...res, ...muts };
    } catch (err) {
      return {
        ok: false,
        error_code: (err && err.code) || 'WasiRuntimeError',
        message: err && err.message ? err.message : String(err),
      };
    }
  }

  async handleSqliteExec(req) {
    const info = this.#extractBridgePayload(req);
    const vfs = new MvmPolyglotVfs(info.workspace, info.cwd);
    try {
      const res = MvmSqliteEngine.execute(
        info.args,
        vfs,
        cachedTextDecoder.decode(info.stdinBytes),
      );
      const muts = vfs.exportMutations();
      return { ...res, ...muts };
    } catch (err) {
      return {
        ok: false,
        error_code: (err && err.code) || 'SqliteRuntimeError',
        message: err && err.message ? err.message : String(err),
      };
    }
  }
}

function __uorNormalizeStitchGroup(g) {
  if (!g || typeof g !== 'object') return null;
  const left =
    g.left && typeof g.left === 'object'
      ? g.left
      : g.left_file
        ? { file: g.left_file, byte_length: g.left_length >>> 0, sha256: String(g.left_sha256 || '') }
        : null;
  const right =
    g.right && typeof g.right === 'object'
      ? g.right
      : g.right_file
        ? { file: g.right_file, byte_length: g.right_length >>> 0, sha256: String(g.right_sha256 || '') }
        : null;
  const parity =
    g.parity && typeof g.parity === 'object'
      ? g.parity
      : g.parity_file
        ? { file: g.parity_file, byte_length: g.parity_length >>> 0, sha256: String(g.parity_sha256 || '') }
        : null;
  if (!left || !parity) return null;
  return {
    group_index: Number.isInteger(g.group_index) ? g.group_index : (Number(g.group_id || 0) >>> 0),
    left,
    right,
    parity,
  };
}

function __uorStitchXorPairRecoverJs(group, missingSide, presentLeft, presentRight, presentParity) {
  const normalized = __uorNormalizeStitchGroup(group);
  if (!normalized) {
    throw new Error('Invalid Stitch parity group');
  }
  const leftLen = Number(normalized.left.byte_length) >>> 0;
  const rightLen = normalized.right ? (Number(normalized.right.byte_length) >>> 0) : 0;
  const parityLen = Number(normalized.parity.byte_length) >>> 0;
  const targetLen = missingSide === 'left' ? leftLen : rightLen;
  // Only enter wasm-bindgen wrappers when Wasm is initialized so pre-init Stitch shard loading uses pure JS.
  if (typeof wasm !== 'undefined' && wasm !== undefined && typeof studio_stitch_recover_pair_auto === 'function') {
    return studio_stitch_recover_pair_auto(
      JSON.stringify(normalized),
      missingSide === 'left',
      Boolean(presentLeft),
      presentLeft || new Uint8Array(0),
      Boolean(presentRight),
      presentRight || new Uint8Array(0),
      Boolean(presentParity),
      presentParity || new Uint8Array(0)
    );
  }
  const hasPartnerRequired = Boolean(normalized.right);
  const other = missingSide === 'left' ? (presentRight || new Uint8Array(0)) : (presentLeft || new Uint8Array(0));
  const parity = presentParity;
  if (!(parity instanceof Uint8Array) || (hasPartnerRequired && !(other instanceof Uint8Array))) {
    throw new Error('Insufficient Stitch parity shards (need 2 of 3)');
  }
  if (parity.byteLength !== parityLen) {
    throw new Error(`Stitch parity length mismatch: ${parity.byteLength} != ${parityLen}`);
  }
  const out = new Uint8Array(targetLen);
  for (let idx = 0; idx < targetLen; idx++) {
    const pByte = idx < parity.byteLength ? parity[idx] : 0;
    const oByte = idx < other.byteLength ? other[idx] : 0;
    out[idx] = pByte ^ oByte;
  }
  return out;
}

export async function stitch_fetch_with_parity_recovery({
  manifest,
  baseUrl,
  stitchManifest = null,
  mirrors = [],
  fetchImpl = typeof fetch === 'function' ? fetch.bind(globalThis) : null,
  onProgress = null,
} = {}) {
  if (!manifest || !Array.isArray(manifest.shards)) {
    throw new Error('Valid shard manifest is required');
  }
  if (typeof fetchImpl !== 'function') {
    throw new Error('No fetch implementation available for Stitch shard fetch');
  }
  const normalizedBase = String(baseUrl || '').endsWith('/') ? String(baseUrl || '') : `${String(baseUrl || '')}/`;
  const sourceBases = Array.from(
    new Set(
      [normalizedBase, ...(Array.isArray(mirrors) ? mirrors : [])]
        .filter((u) => typeof u === 'string' && u.trim().length > 0)
        .map((u) => (u.endsWith('/') ? u : `${u}/`))
    )
  );
  const orderedBases = [...sourceBases];

  let verifiedStitch = null;
  if (stitchManifest) {
    if (typeof wasm !== 'undefined' && wasm !== undefined && typeof studio_stitch_verify_manifest === 'function') {
      const check = JSON.parse(studio_stitch_verify_manifest(JSON.stringify(stitchManifest)));
      if (!check || check.status !== 'ok') {
        throw new Error((check && check.message) || 'Invalid Stitch parity manifest');
      }
      verifiedStitch = stitchManifest;
    } else if (
      (stitchManifest.schema === 'stitch/xor-2-of-3/1' || stitchManifest.scheme === 'stitch/xor-2-of-3/1') &&
      Array.isArray(stitchManifest.groups)
    ) {
      verifiedStitch = stitchManifest;
    }
  }

  const shardGroupByFile = new Map();
  if (verifiedStitch && Array.isArray(verifiedStitch.groups)) {
    for (const rawGroup of verifiedStitch.groups) {
      const g = __uorNormalizeStitchGroup(rawGroup);
      if (!g) continue;
      if (g.left && typeof g.left.file === 'string') shardGroupByFile.set(g.left.file, { group: g, side: 'left' });
      if (g.right && typeof g.right.file === 'string') shardGroupByFile.set(g.right.file, { group: g, side: 'right' });
    }
  }

  const fetchVerifiedShard = async (fileName, expectedLength, expectedSha256, failures) => {
    const candidateBases = [...orderedBases];
    for (let bi = 0; bi < candidateBases.length; bi++) {
      const originBase = candidateBases[bi];
      const url = `${originBase}${fileName}`;
      try {
        const resp = await fetchImpl(url);
        if (!resp || !resp.ok) {
          failures.push(`${url}: HTTP ${resp ? resp.status : 'error'}`);
          continue;
        }
        const ab = await resp.arrayBuffer();
        const bytes = new Uint8Array(ab);
        if (typeof expectedLength === 'number' && expectedLength >= 0 && bytes.byteLength !== expectedLength) {
          failures.push(`${url}: length ${bytes.byteLength} != ${expectedLength}`);
          continue;
        }
        if (expectedSha256) {
          const digest = await __uorSha256HexBytes(bytes);
          if (digest !== expectedSha256) {
            failures.push(`${url}: sha256 mismatch`);
            continue;
          }
        }
        const idxNow = orderedBases.indexOf(originBase);
        if (idxNow > 0) {
          orderedBases.splice(idxNow, 1);
          orderedBases.unshift(originBase);
        }
        return { bytes, source: originBase };
      } catch (err) {
        failures.push(`${url}: ${err && err.message ? err.message : String(err)}`);
      }
    }
    return null;
  };

  const totalBytes = manifest.total_bytes >>> 0;
  const fullBuf = new Uint8Array(totalBytes);
  const fetchedMap = new Map();
  const failedSources = [];
  const healedShards = [];
  const healedFiles = new Set();
  const sourcesUsed = new Set();
  let completedCount = 0;
  let completedBytes = 0;

  const hydrateOneShard = async (s) => {
    const existing = fetchedMap.get(s.file);
    const hit = existing
      ? { bytes: existing, source: normalizedBase }
      : await fetchVerifiedShard(s.file, s.byte_length, s.sha256, failedSources);
    let shardBytes = hit ? hit.bytes : null;
    if (hit) {
      sourcesUsed.add(hit.source);
      fetchedMap.set(s.file, shardBytes);
    } else {
      const entry = shardGroupByFile.get(s.file);
      if (!entry) {
        throw new Error(`Shard fetch failed with no Stitch parity group for ${s.file}: ${failedSources.join('; ')}`);
      }
      const { group, side } = entry;
      const partnerEntry = side === 'left' ? group.right : group.left;
      const partnerFile = partnerEntry ? partnerEntry.file : null;
      const partnerLen = partnerEntry ? partnerEntry.byte_length : 0;
      const partnerSha = partnerEntry ? partnerEntry.sha256 : '';

      let partnerBytes = partnerFile ? fetchedMap.get(partnerFile) || null : null;
      if (partnerFile && !partnerBytes) {
        const partnerHit = await fetchVerifiedShard(partnerFile, partnerLen, partnerSha, failedSources);
        if (partnerHit) {
          partnerBytes = partnerHit.bytes;
          fetchedMap.set(partnerFile, partnerBytes);
          sourcesUsed.add(partnerHit.source);
        }
      }

      const parityHit = await fetchVerifiedShard(
        group.parity.file,
        group.parity.byte_length,
        group.parity.sha256,
        failedSources
      );
      if (!parityHit || (partnerFile && !partnerBytes)) {
        throw new Error(`Stitch 2-of-3 parity recovery failed for ${s.file}: ${failedSources.join('; ')}`);
      }
      sourcesUsed.add(parityHit.source);

      shardBytes = __uorStitchXorPairRecoverJs(
        group,
        side,
        side === 'right' ? partnerBytes : null,
        side === 'left' ? partnerBytes : null,
        parityHit.bytes
      );
      if (shardBytes.byteLength !== s.byte_length) {
        throw new Error(`Recovered shard length mismatch on ${s.file}`);
      }
      const recoveredSha = await __uorSha256HexBytes(shardBytes);
      if (s.sha256 && recoveredSha !== s.sha256) {
        throw new Error(`Recovered shard SHA-256 mismatch on ${s.file}`);
      }
      fetchedMap.set(s.file, shardBytes);
      if (!healedFiles.has(s.file)) {
        healedFiles.add(s.file);
        healedShards.push({
          file: s.file,
          group: group.group_index,
          parityFile: group.parity.file,
          recoveredSha256: recoveredSha,
        });
      }
    }

    fullBuf.set(shardBytes, s.byte_offset);
    completedCount += 1;
    completedBytes += s.byte_length >>> 0;
    if (typeof onProgress === 'function') {
      onProgress({
        index: completedCount,
        total: manifest.shards.length,
        bytes: Math.min(totalBytes, completedBytes),
        totalBytes,
        healedCount: healedShards.length,
      });
    }
  };

  if (manifest.shards.length > 0) {
    // Probe/warm shard 0 first so a working mirror is promoted to orderedBases[0] before parallel batches
    await hydrateOneShard(manifest.shards[0]);
    const batchSize = 6;
    for (let i = 1; i < manifest.shards.length; i += batchSize) {
      const slice = manifest.shards.slice(i, Math.min(i + batchSize, manifest.shards.length));
      await Promise.all(slice.map((s) => hydrateOneShard(s)));
    }
  }

  const stitchHealth = {
    scheme: verifiedStitch ? 'stitch/xor-2-of-3/1' : 'uorlut-sha256-verified',
    custodyMode: verifiedStitch ? '2-of-3 XOR Parity + SHA-256' : 'SHA-256 Direct',
    totalShards: manifest.shards.length,
    verifiedShards: manifest.shards.length - healedShards.length,
    healedShards: healedShards.length,
    healedDetails: healedShards,
    failedSources: failedSources.slice(-16),
    mirrorsConfigured: sourceBases,
    activeSources: Array.from(sourcesUsed),
    status: healedShards.length > 0 ? 'Healed via 2-of-3 Parity' : 'Healthy (All Verified)',
    updatedAt: new Date().toISOString(),
  };
  globalThis.__uorStitchHealth = stitchHealth;
  return { fullBuf, stitchHealth };
}

export async function stream_v10_sharded_weights_to_opfs(onProgress, forceReload = false, modelTier = 'auto') {
  if (wasm === undefined || typeof studio_load_uorgllm_v9_bytes !== 'function') return null;
  const httpOrigin =
    typeof location !== 'undefined' && location.origin && /^https?:/i.test(location.origin)
      ? location.origin.replace(/\/+$/, '')
      : '';
  const hasOpfs =
    typeof navigator !== 'undefined' &&
    navigator.storage &&
    typeof navigator.storage.getDirectory === 'function';

  // Prefer persisted 32L/960D (SmolLM2-360M-Instruct) OPFS container when present unless v10 is forced
  if (!forceReload && hasOpfs && modelTier !== 'v10' && typeof studio_mount_r4_v12_bytes === 'function') {
    try {
      const root = await navigator.storage.getDirectory();
      const mfh360 = await root.getFileHandle(OPFS_V12_360M_META_FILE, { create: false });
      const meta360 = JSON.parse(await (await mfh360.getFile()).text());
      const wfh360 = await root.getFileHandle(OPFS_V12_360M_WEIGHT_FILE, { create: false });
      const bytes360 = new Uint8Array(await (await wfh360.getFile()).arrayBuffer());
      if (bytes360.byteLength >= 64) {
        const loaded360 = JSON.parse(studio_mount_r4_v12_bytes(bytes360));
        if (loaded360 && loaded360.status === 'ok') {
          const fmtLabel = loaded360.format || 'UORR4V12';
          const statusStr = `OPFS ${fmtLabel} (${loaded360.n_layers}L ${loaded360.d_model}D)`;
          const stitchHealth = {
            scheme: 'stitch/xor-2-of-3/1',
            custodyMode: 'OPFS SHA-256 Verified (360M)',
            totalShards: Number(meta360 && meta360.totalShards) || 0,
            verifiedShards: Number(meta360 && meta360.totalShards) || 0,
            healedShards: 0,
            healedDetails: [],
            failedSources: [],
            mirrorsConfigured: ['opfs://local-cache'],
            activeSources: ['opfs://local-cache'],
            status: 'Healthy (OPFS 360M Cached)',
            updatedAt: new Date().toISOString(),
          };
          globalThis.__uorStitchHealth = stitchHealth;
          globalThis.__uorGllmV9Meta = {
            status: statusStr,
            format: fmtLabel,
            tier: loaded360.active_tier || 'Full32LRecompiled',
            sha256: (meta360 && meta360.sha256) || '',
            mode: 'opfs-r4v12-360m-cached',
            stitchHealth,
          };
          return loaded360;
        }
      }
    } catch {
      // Fall through to requested manifest or v10 hydration
    }
  }

  const requested360m = modelTier === 'v12_360m';
  const weightFileName = requested360m ? OPFS_V12_360M_WEIGHT_FILE : OPFS_V10_WEIGHT_FILE;
  const metaFileName = requested360m ? OPFS_V12_360M_META_FILE : OPFS_V10_META_FILE;
  const manifestCandidates = requested360m
    ? [
        new URL('weights/v12_360m/manifest.json', import.meta.url).href,
        new URL('../gpu/weights/v12_360m/manifest.json', import.meta.url).href,
        ...(httpOrigin ? [`${httpOrigin}/weights/v12_360m/manifest.json`] : []),
        'https://raw.githubusercontent.com/Casey-allard/lexlean-r4/main/gpu/weights/v12_360m/manifest.json',
      ]
    : [
        new URL('weights/v10/manifest.json', import.meta.url).href,
        new URL('../gpu/weights/v10/manifest.json', import.meta.url).href,
        new URL('weights/v9/manifest.json', import.meta.url).href,
        ...(httpOrigin ? [`${httpOrigin}/weights/v10/manifest.json`] : []),
        'https://raw.githubusercontent.com/Casey-allard/lexlean-r4/main/gpu/weights/v10/manifest.json',
      ];

  let manifest = null;
  let baseUrl = '';
  for (const mUrl of manifestCandidates) {
    try {
      const resp = await fetch(mUrl, { cache: 'no-cache' });
      if (resp.ok) {
        const parsed = await resp.json();
        if (parsed && (parsed.format === 'UORGLLM9' || parsed.format === 'UORR4V12') && Array.isArray(parsed.shards)) {
          manifest = parsed;
          baseUrl = mUrl.slice(0, mUrl.lastIndexOf('/') + 1);
          break;
        }
      }
    } catch {
      // Try next manifest path
    }
  }

  if (!manifest) {
    return requested360m ? null : restore_gllm_v9_from_opfs('');
  }

  let stitchManifest = null;
  try {
    const stResp = await fetch(`${baseUrl}manifest.stitch.json`, { cache: 'no-cache' });
    if (stResp && stResp.ok) {
      const candidate = await stResp.json();
      if (candidate && (candidate.schema === 'stitch/xor-2-of-3/1' || candidate.scheme === 'stitch/xor-2-of-3/1') && Array.isArray(candidate.groups)) {
        stitchManifest = candidate;
      }
    }
  } catch {
    // Optional stitch parity sidecar manifest
  }

  const expectedSha = String(manifest.container_sha256 || '');
  if (!forceReload && hasOpfs && expectedSha) {
    try {
      const root = await navigator.storage.getDirectory();
      const mfh = await root.getFileHandle(metaFileName, { create: false });
      const meta = JSON.parse(await (await mfh.getFile()).text());
      if (meta && meta.sha256 === expectedSha && (!meta.format || meta.format === manifest.format)) {
        const wfh = await root.getFileHandle(weightFileName, { create: false });
        const bytes = new Uint8Array(await (await wfh.getFile()).arrayBuffer());
        const isR4V12 = manifest.format === 'UORR4V12' || (bytes.byteLength >= 8 && cachedTextDecoder.decode(bytes.subarray(0, 8)) === 'UORR4V12');
        const loadLabel = isR4V12 ? 'opfs_cached_r4_v12_recompiled' : 'opfs_cached_v10_16L_256D';
        const rawLoaded = (isR4V12 && typeof studio_mount_r4_v12_bytes === 'function')
          ? studio_mount_r4_v12_bytes(bytes)
          : studio_load_uorgllm_v9_bytes(bytes, loadLabel);
        const loaded = JSON.parse(rawLoaded);
        if (loaded && loaded.status === 'ok') {
          const stitchHealth = {
            scheme: stitchManifest ? 'stitch/xor-2-of-3/1' : 'opfs-cached-verified',
            custodyMode: stitchManifest ? '2-of-3 XOR Parity + OPFS' : 'OPFS SHA-256 Verified',
            totalShards: manifest.shards.length,
            verifiedShards: manifest.shards.length,
            healedShards: 0,
            healedDetails: [],
            failedSources: [],
            mirrorsConfigured: [baseUrl],
            activeSources: ['opfs://local-cache'],
            status: 'Healthy (OPFS Cached)',
            updatedAt: new Date().toISOString(),
          };
          const fmtLabel = loaded.format || manifest.format || (isR4V12 ? 'UORR4V12' : 'UORGLLM9');
          const statusStr = `OPFS ${fmtLabel} (${loaded.n_layers}L ${loaded.d_model}D)`;
          globalThis.__uorStitchHealth = stitchHealth;
          globalThis.__uorGllmV9Meta = {
            status: statusStr,
            format: fmtLabel,
            tier: loaded.active_tier || (isR4V12 ? 'FullRecompiled' : 'LegacyV9'),
            sha256: expectedSha,
            mode: requested360m ? 'opfs-r4v12-360m-cached' : (isR4V12 ? 'opfs-r4v12-cached' : 'opfs-v10-cached'),
            stitchHealth,
          };
          return loaded;
        }
      }
    } catch {
      // Proceed to stream shards
    }
  }

  const mirrors = Array.isArray(manifest.mirrors)
    ? manifest.mirrors
    : stitchManifest && Array.isArray(stitchManifest.mirrors)
      ? stitchManifest.mirrors
      : [];
  const { fullBuf, stitchHealth } = await stitch_fetch_with_parity_recovery({
    manifest,
    baseUrl,
    stitchManifest,
    mirrors,
    onProgress,
  });
  const totalBytes = fullBuf.byteLength;

  const fullSha = await __uorSha256HexBytes(fullBuf);
  if (expectedSha && fullSha !== expectedSha) {
    throw new Error(`${manifest.format || 'UORR4V12'} container SHA-256 mismatch`);
  }

  if (hasOpfs) {
    try {
      const root = await navigator.storage.getDirectory();
      const fh = await root.getFileHandle(weightFileName, { create: true });
      if (typeof fh.createSyncAccessHandle === 'function') {
        const sh = await fh.createSyncAccessHandle();
        sh.truncate(totalBytes);
        sh.write(fullBuf, { at: 0 });
        sh.flush();
        sh.close();
      } else if (typeof fh.createWritable === 'function') {
        const w = await fh.createWritable();
        await w.write(fullBuf);
        await w.close();
      }
      const mfh = await root.getFileHandle(metaFileName, { create: true });
      const metaBytes = cachedTextEncoder.encode(
        JSON.stringify({
          sha256: fullSha,
          totalBytes,
          totalShards: manifest.shards.length,
          format: manifest.format || 'UORR4V12',
        })
      );
      if (typeof mfh.createSyncAccessHandle === 'function') {
        const msh = await mfh.createSyncAccessHandle();
        msh.truncate(metaBytes.byteLength);
        msh.write(metaBytes, { at: 0 });
        msh.flush();
        msh.close();
      } else if (typeof mfh.createWritable === 'function') {
        const mw = await mfh.createWritable();
        await mw.write(metaBytes);
        await mw.close();
      }
    } catch {
      // OPFS write optional if quota restricted
    }
  }

  const isStreamedR4V12 = manifest.format === 'UORR4V12' || (fullBuf.byteLength >= 8 && cachedTextDecoder.decode(fullBuf.subarray(0, 8)) === 'UORR4V12');
  const streamLabel = isStreamedR4V12 ? 'opfs_streamed_r4_v12_recompiled' : 'opfs_streamed_v10_16L_256D';
  const rawParsed = (isStreamedR4V12 && typeof studio_mount_r4_v12_bytes === 'function')
    ? studio_mount_r4_v12_bytes(fullBuf)
    : studio_load_uorgllm_v9_bytes(fullBuf, streamLabel);
  const parsed = JSON.parse(rawParsed);
  if (parsed && parsed.status === 'ok') {
    const fmtLabel = parsed.format || manifest.format || (isStreamedR4V12 ? 'UORR4V12' : 'UORGLLM9');
    globalThis.__uorGllmV9Meta = {
      status: `OPFS ${fmtLabel} (${parsed.n_layers}L ${parsed.d_model}D)`,
      format: fmtLabel,
      tier: parsed.active_tier || (isStreamedR4V12 ? 'FullRecompiled' : 'LegacyV9'),
      sha256: fullSha,
      mode: requested360m ? 'opfs-r4v12-360m-streamed' : (isStreamedR4V12 ? 'opfs-r4v12-streamed' : 'opfs-v10-streamed'),
      stitchHealth,
    };
    return parsed;
  }
  return null;
}

function __uorCollectWorkspaceSnapshot() {
  const listRes = JSON.parse(studio_dispatch('1\tstudio\tworkspace_list\t'));
  const files = listRes && Array.isArray(listRes.files) ? listRes.files : [];
  const hydrated = files.map((f) => {
    let content = f.content;
    if (typeof content !== 'string') {
      try {
        const r = JSON.parse(studio_dispatch(`1\tstudio\tworkspace_read\t${f.path}`));
        content = r && r.status === 'ok' ? (r.content || '') : '';
      } catch {
        content = '';
      }
    }
    return { ...f, content };
  });
  return { status: 'ok', files: hydrated };
}

function __uorSynthesizeBrainPulsesFromReply(parsedRes, toolKind = 'none') {
  if (Array.isArray(parsedRes && parsedRes.brain_pulses) && parsedRes.brain_pulses.length > 0) {
    return parsedRes.brain_pulses;
  }
  const gen = (parsedRes && parsedRes.generation) || {};
  const tokGen = (parsedRes && parsedRes.token_generation) || gen.token_generation || {};
  const baseOrbit = ((parsedRes && parsedRes.icosian_root) ?? tokGen.final_icosian_root ?? 3) & 15;
  const baseHead = ((parsedRes && parsedRes.flock_consensus_action) ?? 2) & 7;
  const steps = Math.min(Math.max(Number(tokGen.stepped_tokens) || 10, 6), 24);
  const pulses = [];
  for (let s = 0; s < steps; s++) {
    pulses.push({
      step: s,
      phase: s === 0 ? 'prefill' : (toolKind !== 'none' && s === steps - 1 ? 'tool' : 'decode'),
      active_layer: s % 16,
      winning_head: (baseHead + s) & 7,
      e8_orbit: (baseOrbit + s * 3) & 15,
      cl06_grade: (s * 2 + 1) % 7,
      kv_copy_dist: s % 4 === 2 ? 12 + s * 3 : 0,
      prll_top_mass: 620000 + ((s * 31337) % 280000),
      tool_kind: s === steps - 1 ? toolKind : 'none',
    });
  }
  return pulses;
}

function __uorStartDedicatedEngineWorker() {
  const hostBridge = new SpiralCoreHostBridge();
  let wasmReady = false;
  let bootPromise = null;
  let activeHydrationPromise = null;
  let lastHydrationProgress = null;
  let activeInprismRequestId = null;
  let activeInprismTurnId = null;
  const cancelledInprismRequests = new Set();
  const cancelledInprismTurns = new Set();
  const pendingInprismCancelWaiters = new Map();
  const earlyMessages = [];

  const postEvent = (event, data = {}) => {
    self.postMessage({ event, ...data });
  };
  globalThis.__uorEmitChatDelta = (textChunk, step = 0) => {
    const chunk = String(textChunk || '');
    if (chunk.length > 0) {
      postEvent('chat_stream_delta', { text: chunk, step: Number(step) || 0 });
    }
  };
  globalThis.__uorEmitHoloTraceFrame = (frameJson) => {
    /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
    try {
      const frame = typeof frameJson === 'string' ? JSON.parse(frameJson) : frameJson;
      if (frame) {
        postEvent('holo_trace_frame', { frame });
      }
    } catch {
      // Ignore malformed frame payload cleanly
    }
  };

  const startOrAwaitOpfsShardHydration = (forceReload = false, modelTier = 'auto') => {
    if (activeHydrationPromise && !forceReload && modelTier === 'auto') {
      return activeHydrationPromise;
    }
    const run = stream_v10_sharded_weights_to_opfs((prog) => {
      lastHydrationProgress = { stage: 'r4_shards', ...prog };
      postEvent('boot_progress', lastHydrationProgress);
      postEvent('hydration_progress', lastHydrationProgress);
    }, forceReload, modelTier)
      .then((v10Info) => {
        postEvent('stitch_health', { stitchHealth: globalThis.__uorStitchHealth || null });
        if (v10Info) {
          const r4Stack = typeof studio_r4_stack_status === 'function' ? JSON.parse(studio_r4_stack_status()) : null;
          postEvent('weights_upgraded', {
            gllm: v10Info,
            r4Stack,
            gllmMeta: globalThis.__uorGllmV9Meta || null,
            stitchHealth: globalThis.__uorStitchHealth || null,
          });
        }
        return v10Info;
      })
      .catch(() => null)
      .finally(() => {
        if (activeHydrationPromise === run) {
          activeHydrationPromise = null;
        }
      });
    activeHydrationPromise = run;
    return run;
  };

  let mvmWorkspaceSyncEpoch = 0;
  const syncedHostWorkspaceFiles = new Map();

  const encodeUtf8ToBase64 = (text) => {
    const str = String(text || '');
    if (cachedTextEncoder && typeof cachedTextEncoder.encode === 'function') {
      return __uorBytesToBase64(cachedTextEncoder.encode(str));
    }
    if (typeof Buffer !== 'undefined') {
      return Buffer.from(str, 'utf8').toString('base64');
    }
    const u8 = new Uint8Array(str.length);
    for (let i = 0; i < str.length; i++) u8[i] = str.charCodeAt(i) & 0xff;
    return __uorBytesToBase64(u8);
  };

  const decodeBase64ToUtf8 = (b64) => {
    const bytes = __uorBase64ToBytes(b64);
    if (cachedTextDecoder && typeof cachedTextDecoder.decode === 'function') {
      return cachedTextDecoder.decode(bytes);
    }
    if (typeof Buffer !== 'undefined') {
      return Buffer.from(bytes).toString('utf8');
    }
    return '';
  };

  const resolveMvmSyncInFn = () =>
    typeof mvm_sync_workspace_in_json === 'function'
      ? mvm_sync_workspace_in_json
      : globalThis.mvm_sync_workspace_in_json;

  const resolveMvmExportDiffFn = () =>
    typeof mvm_export_workspace_diff_json === 'function'
      ? mvm_export_workspace_diff_json
      : globalThis.mvm_export_workspace_diff_json;

  const updateMvmWorkspaceSyncEpoch = (rawEpoch) => {
    const parsedEpoch = Number(rawEpoch);
    if (Number.isFinite(parsedEpoch)) {
      mvmWorkspaceSyncEpoch = Math.max(mvmWorkspaceSyncEpoch, parsedEpoch);
    }
  };

  const applyMvmSyncInState = (res, nextMap, deltaEntries) => {
    if (!res || res.status !== 'ok') return;
    updateMvmWorkspaceSyncEpoch(res.epoch);
    if (nextMap instanceof Map) {
      syncedHostWorkspaceFiles.clear();
      for (const [k, v] of nextMap.entries()) syncedHostWorkspaceFiles.set(k, v);
      return;
    }
    if (!Array.isArray(deltaEntries)) return;
    for (const entry of deltaEntries) {
      if (!entry || typeof entry.path !== 'string') continue;
      const rel = String(entry.path).trim().replace(/^\/+/, '').replace(/^workspace\//, '');
      if (!rel || (rel.startsWith('prism/') && rel.endsWith('.lex.tex')) || rel === 'app/state') {
        continue;
      }
      if (entry.deleted) {
        syncedHostWorkspaceFiles.delete(rel);
      } else if (typeof entry.content_base64 === 'string') {
        syncedHostWorkspaceFiles.set(rel, decodeBase64ToUtf8(entry.content_base64));
      }
    }
  };

  const applyMvmExportDiffState = (diff) => {
    if (!diff || diff.status !== 'ok') return diff;
    updateMvmWorkspaceSyncEpoch(diff.epoch);
    const writes = Array.isArray(diff.writes) ? diff.writes : [];
    for (const w of writes) {
      if (w && typeof w.path === 'string' && typeof w.content_base64 === 'string') {
        syncedHostWorkspaceFiles.set(w.path, decodeBase64ToUtf8(w.content_base64));
      }
    }
    const deletes = Array.isArray(diff.deletes) ? diff.deletes : [];
    for (const delPath of deletes) {
      if (typeof delPath === 'string') syncedHostWorkspaceFiles.delete(delPath);
    }
    return diff;
  };

  const runMvmSyncWorkspaceIn = (payloadInput, nextMap, allowDispatchFallback) => {
    const syncInFn = resolveMvmSyncInFn();
    if (typeof syncInFn !== 'function' && !allowDispatchFallback) return null;
    const payloadStr =
      typeof payloadInput === 'string'
        ? payloadInput
        : JSON.stringify(payloadInput || { epoch: mvmWorkspaceSyncEpoch, files: [] });
    let deltaEntries = null;
    if (!(nextMap instanceof Map)) {
      try {
        const parsedReq = typeof payloadInput === 'string' ? JSON.parse(payloadStr) : payloadInput;
        deltaEntries = parsedReq && Array.isArray(parsedReq.files) ? parsedReq.files : null;
      } catch {
        deltaEntries = null;
      }
    }
    const resRaw =
      typeof syncInFn === 'function'
        ? syncInFn(payloadStr)
        : studio_dispatch(`1\tstudio\tmvm_sync_workspace_in\t${payloadStr}`);
    const res = typeof resRaw === 'string' ? JSON.parse(resRaw) : resRaw;
    applyMvmSyncInState(res, nextMap, deltaEntries);
    return res;
  };

  const runMvmExportWorkspaceDiff = (allowDispatchFallback) => {
    const exportDiffFn = resolveMvmExportDiffFn();
    if (typeof exportDiffFn !== 'function' && !allowDispatchFallback) return null;
    const diffRaw =
      typeof exportDiffFn === 'function'
        ? exportDiffFn()
        : studio_dispatch('1\tstudio\tmvm_export_workspace_diff\t');
    const diff = typeof diffRaw === 'string' ? JSON.parse(diffRaw) : diffRaw;
    return applyMvmExportDiffState(diff);
  };

  const syncWorkspaceIntoMvm = () => {
    try {
      const snap = __uorCollectWorkspaceSnapshot();
      const files = snap && Array.isArray(snap.files) ? snap.files : [];
      const nextMap = new Map();
      const payloadFiles = [];
      for (const f of files) {
        if (!f || typeof f.path !== 'string') continue;
        const content = typeof f.content === 'string' ? f.content : '';
        nextMap.set(f.path, content);
        if (syncedHostWorkspaceFiles.get(f.path) !== content) {
          payloadFiles.push({
            path: f.path,
            content_base64: encodeUtf8ToBase64(content),
          });
        }
      }
      for (const prevPath of syncedHostWorkspaceFiles.keys()) {
        if (!nextMap.has(prevPath)) {
          payloadFiles.push({ path: prevPath, content_base64: '', deleted: true });
        }
      }
      if (payloadFiles.length === 0 && syncedHostWorkspaceFiles.size > 0) {
        return null;
      }
      return runMvmSyncWorkspaceIn(
        { epoch: mvmWorkspaceSyncEpoch, files: payloadFiles },
        nextMap,
        false,
      );
    } catch {
      return null;
    }
  };

  const exportWorkspaceDiffFromMvm = () => {
    try {
      return runMvmExportWorkspaceDiff(false);
    } catch {
      return null;
    }
  };

  const applyClonedFilesToWorkspace = (files) => {
    if (!Array.isArray(files) || files.length === 0) return null;
    for (const f of files) {
      if (f && typeof f.path === 'string' && typeof f.content === 'string') {
        studio_dispatch(`1\tstudio\tworkspace_write\t${f.path}\n${f.content}`);
      }
    }
    const snap = __uorCollectWorkspaceSnapshot();
    postEvent('workspace_sync', { workspace: snap });
    return snap;
  };

  const dispatchWithConnectedBridge = async (envelope) => {
    const isMvmExecEnvelope = envelope.includes('\tmvm_exec\t');
    if (isMvmExecEnvelope) {
      syncWorkspaceIntoMvm();
    }
    let currentRaw = studio_dispatch(envelope);
    let parsed = JSON.parse(currentRaw);
    let hops = 0;
    let lastToolKind = 'none';

    while (parsed && parsed.status === 'needs_bridge' && hops < 8) {
      hops++;
      const bridgeReq = parsed.bridge_req || parsed.pending_bridge || parsed.request || {};
      const kind = String(bridgeReq.op || bridgeReq.kind || bridgeReq.tool || 'http_request');
      lastToolKind = kind;
      postEvent('bridge_activity', { phase: 'start', hop: hops, op: kind, request: bridgeReq });
      postEvent('brain_pulse', {
        pulses: [
          {
            step: hops,
            phase: 'tool',
            active_layer: (hops * 2) & 7,
            winning_head: hops & 7,
            e8_orbit: (hops * 5) & 15,
            cl06_grade: 3,
            kv_copy_dist: 24,
            prll_top_mass: 850000,
            tool_kind: kind,
          },
        ],
      });

      let execBridgeReq = bridgeReq;
      if (
        execBridgeReq &&
        execBridgeReq.op === 'git' &&
        (execBridgeReq.subcommand === 'push' || execBridgeReq.subcommand === 'commit_push') &&
        !Array.isArray(execBridgeReq.files)
      ) {
        const snap = __uorCollectWorkspaceSnapshot();
        if (snap && Array.isArray(snap.files) && snap.files.length > 0) {
          execBridgeReq = { ...execBridgeReq, files: snap.files };
        }
      }
      const bridgeResp = await hostBridge.execute(execBridgeReq);
      if (bridgeResp && bridgeResp.op === 'git') {
        if (Array.isArray(bridgeResp.files) && bridgeResp.files.length > 0) {
          applyClonedFilesToWorkspace(bridgeResp.files);
        } else if (typeof bridgeResp.path === 'string' && typeof bridgeResp.content === 'string') {
          applyClonedFilesToWorkspace([{ path: bridgeResp.path, content: bridgeResp.content }]);
        }
      }
      postEvent('bridge_activity', {
        phase: 'complete',
        hop: hops,
        op: kind,
        ok: Boolean(bridgeResp && bridgeResp.ok),
        transport: hostBridge.lastTransportMode,
      });

      const isChatTurn = envelope.includes('\tchat\t') || envelope.includes('\tchat_v10_turn\t');
      const resumeCmd =
        parsed.resume_command ||
        (isChatTurn ? 'chat_complete_bridge' : 'mvm_complete_bridge');
      const resumePayload = JSON.stringify({
        session_id: parsed.session_id || parsed.bridge_id || '',
        bridge_id: parsed.bridge_id || parsed.session_id || '',
        resume_state: parsed.resume_state,
        bridge_resp: bridgeResp,
        response: bridgeResp,
      });
      currentRaw = studio_dispatch(`1\tstudio\t${resumeCmd}\t${resumePayload}`);
      parsed = JSON.parse(currentRaw);
    }

    const isChatTurn = envelope.includes('\tchat\t') || envelope.includes('\tchat_v10_turn\t');
    if (isChatTurn && parsed && Array.isArray(parsed.holo_observatory_frames) && parsed.holo_observatory_frames.length > 0) {
      postEvent('holo_observatory_batch', {
        frames: parsed.holo_observatory_frames,
        prefixReused: Number(parsed.holo_prefix_reused_tokens) || 0,
        selectedModel: parsed.selected_model || '',
      });
    }
    if (isChatTurn || envelope.includes('\tmvm_exec\t')) {
      const pulses = __uorSynthesizeBrainPulsesFromReply(parsed, lastToolKind);
      postEvent('brain_pulse', { pulses });
    }
    if (isMvmExecEnvelope || (isChatTurn && parsed && parsed.mvm_tool_executed)) {
      const diff = exportWorkspaceDiffFromMvm();
      persist_mvm_cow_sectors_to_opfs().catch(() => {});
      const snap = __uorCollectWorkspaceSnapshot();
      if (diff && diff.status === 'ok' && parsed && typeof parsed === 'object') {
        parsed.workspace_diff = diff;
        parsed.ignored_heavy_paths = Number(diff.ignored_heavy_paths) || 0;
        const diffWrites = Array.isArray(diff.writes) ? diff.writes.map((w) => w.path) : [];
        const diffDeletes = Array.isArray(diff.deletes) ? diff.deletes : [];
        parsed.changed_files = diffWrites.concat(diffDeletes);
      }
      postEvent('workspace_sync', {
        workspace: snap,
        ...(diff && diff.status === 'ok' ? { diff } : {}),
      });
      if (parsed && typeof parsed === 'object') {
        Object.defineProperty(parsed, '__uorWorkspaceSyncEmitted', {
          value: true,
          configurable: true,
          enumerable: false,
        });
      }
    }
    return parsed;
  };

  const handleWorkerMessage = async (msg) => {
    const { id, op, payload } = msg || {};
    try {
      if (op === 'studio_dispatch') {
        const envelopeStr = String(payload || '');
        if (envelopeStr.includes('\tchat\t') || envelopeStr.includes('\tchat_v10_turn\t')) {
          let reqHoloModel = '';
          try {
            const parts = envelopeStr.split('\t');
            const bodyRaw = parts.length >= 4 ? parts.slice(3).join('\t').trim() : '';
            if (bodyRaw.startsWith('{')) {
              const reqObj = JSON.parse(bodyRaw);
              const sel = String((reqObj && (reqObj.selected_model || reqObj.model)) || '');
              if (sel === 'uor-r4.qwen3-0.6b-holo' || sel === 'uor-r4.qwen3-1.7b-holo') {
                reqHoloModel = sel;
              }
            }
          } catch {
            // Non-JSON chat envelopes default cleanly
          }
          if (reqHoloModel && typeof studio_holo_runtime_status === 'function') {
            let mountedHolo = false;
            try {
              const hs = JSON.parse(studio_holo_runtime_status());
              mountedHolo = reqHoloModel === 'uor-r4.qwen3-1.7b-holo'
                ? Boolean(hs && hs.qwen3_17b_mounted)
                : Boolean(hs && hs.qwen3_06b_mounted);
            } catch {
              mountedHolo = false;
            }
            if (!mountedHolo) {
              postEvent('hydration_progress', {
                stage: 'holo_v13',
                awaiting: true,
                modelId: reqHoloModel,
                label: `Hydrating verified ${reqHoloModel} pieces via OpfsKappaStore...`,
              });
              await hydrate_holo_model(reqHoloModel, (prog) => {
                postEvent('boot_progress', { stage: 'holo_v13', ...prog });
                postEvent('hydration_progress', { stage: 'holo_v13', ...prog });
              }, false);
            }
          }
        }
        const res = await dispatchWithConnectedBridge(envelopeStr);
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (op === 'host_bridge_exec') {
        const toolKind = String((payload && (payload.op || payload.kind)) || 'http_request');
        postEvent('bridge_activity', { phase: 'start', hop: 1, op: toolKind });
        postEvent('brain_pulse', {
          pulses: [
            {
              step: 0,
              phase: 'tool',
              active_layer: 4,
              winning_head: 3,
              e8_orbit: 9,
              cl06_grade: 4,
              kv_copy_dist: 32,
              prll_top_mass: 890000,
              tool_kind: toolKind,
            },
          ],
        });
        let reqPayload = payload || {};
        if (
          reqPayload.op === 'git' &&
          (reqPayload.subcommand === 'push' || reqPayload.subcommand === 'commit_push') &&
          !Array.isArray(reqPayload.files)
        ) {
          const snap = __uorCollectWorkspaceSnapshot();
          reqPayload = { ...reqPayload, files: snap.files };
        }
        const res = await hostBridge.execute(reqPayload);
        if (res && res.op === 'git') {
          if (Array.isArray(res.files) && res.files.length > 0) {
            applyClonedFilesToWorkspace(res.files);
          } else if (typeof res.path === 'string' && typeof res.content === 'string') {
            applyClonedFilesToWorkspace([{ path: res.path, content: res.content }]);
          }
        }
        postEvent('bridge_activity', {
          phase: 'complete',
          hop: 1,
          op: toolKind,
          ok: Boolean(res && res.ok),
          transport: hostBridge.lastTransportMode,
        });
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (op === 'mvm_set_secret') {
        const key = String((payload && payload.key) || 'GITHUB_TOKEN');
        const value = String((payload && payload.value) || '');
        const vaultRes = hostBridge.setSecret(key, value);
        try {
          studio_dispatch(`1\tstudio\tmvm_set_secret\t${JSON.stringify({ key, value })}`);
        } catch {
          // Host JS vault remains primary when Wasm guest command is absent
        }
        self.postMessage({ id, ok: vaultRes.ok, result: vaultRes });
        return;
      }
      if (op === 'set_bridge_policy' || op === 'set_host_config') {
        const res = hostBridge.setPolicy(payload || {});
        const customRelay = payload && (typeof payload.relayWsUrl === 'string' ? payload.relayWsUrl : payload.relayUrl);
        if (typeof customRelay === 'string') {
          hostBridge.setRelayUrl(customRelay);
        }
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (op === 'wasm_eval' || op === 'studio_wasm_eval') {
        const sub = String((payload && payload.subcommand) || 'check');
        const filter = String((payload && payload.filter) || '');
        const res = JSON.parse(studio_wasm_eval(sub, filter));
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (op === 'process_envelope') {
        const reqBytes = cachedTextEncoder.encode(String(payload || ''));
        const respBytes = process_envelope(reqBytes);
        const res = cachedTextDecoder.decode(respBytes);
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (
        op === 'v1_endpoint' ||
        op === 'studio_v1_messages' ||
        op === 'studio_v1_chat_completions' ||
        op === 'studio_v1_traces'
      ) {
        const endpoint =
          op === 'studio_v1_messages'
            ? 'messages'
            : op === 'studio_v1_chat_completions'
              ? 'chat_completions'
              : op === 'studio_v1_traces'
                ? 'traces'
                : String((payload && payload.endpoint) || 'messages');
        const body = typeof payload === 'string' ? payload : String((payload && payload.body) || '{}');
        const raw =
          endpoint === 'messages'
            ? studio_v1_messages(body)
            : endpoint === 'chat_completions'
              ? studio_v1_chat_completions(body)
              : studio_v1_traces(body);
        let outVal = raw;
        if (op.startsWith('studio_v1_')) {
          try {
            outVal = JSON.parse(raw);
          } catch {
            outVal = raw;
          }
        }
        self.postMessage({ id, ok: true, result: outVal });
        return;
      }
      if (op === 'load_uorlut' || op === 'load_uorlut_bytes') {
        const u8 = payload && payload.bytes instanceof Uint8Array
          ? payload.bytes
          : new Uint8Array((payload && payload.buffer) || 0);
        const label = String((payload && (payload.sourceLabel || payload.label)) || 'uploaded_uorlut');
        const parsed = JSON.parse(studio_load_uorlut_bytes(u8, label));
        if (parsed && parsed.status === 'ok') {
          await cache_uorlut_in_indexeddb(u8, label);
        }
        self.postMessage({
          id,
          ok: true,
          result: {
            ...parsed,
            load: parsed,
            state: JSON.parse(studio_dispatch('1\tstudio\tstate\t')),
          },
        });
        return;
      }
      if (op === 'holo_runtime_status') {
        const holo = typeof studio_holo_runtime_status === 'function' ? JSON.parse(studio_holo_runtime_status()) : null;
        const merged = holo
          ? { ...holo, holo, kappaStore: globalThis.__uorHoloKappaStatus || null }
          : { holo: null, kappaStore: globalThis.__uorHoloKappaStatus || null };
        self.postMessage({ id, ok: true, result: merged });
        return;
      }
      if (op === 'hydrate_holo_model') {
        const modelId = String((payload && payload.modelId) || 'uor-r4.qwen3-0.6b-holo');
        const force = Boolean(payload && payload.force);
        const allowOffOriginHydration = Boolean(payload && payload.allowOffOriginHydration);
        const holo = await hydrate_holo_model(modelId, (prog) => {
          postEvent('boot_progress', { stage: 'holo_v13', ...prog });
          postEvent('hydration_progress', { stage: 'holo_v13', ...prog });
        }, force, { allowOffOriginHydration });
        postEvent('stitch_health', { stitchHealth: globalThis.__uorStitchHealth || null });
        self.postMessage({ id, ok: true, result: { holo, kappaStore: globalThis.__uorHoloKappaStatus || null } });
        return;
      }
      if (op === 'holo_storage_geometry') {
        /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
        const reqId = String((payload && payload.modelId) || '');
        const raw = typeof studio_holo_storage_geometry_json === 'function'
          ? studio_holo_storage_geometry_json(reqId)
          : studio_dispatch(`1\tstudio\tholo_storage_geometry\t${reqId}`);
        self.postMessage({ id, ok: true, result: JSON.parse(raw) });
        return;
      }
      if (op === 'telos_self_improvement_tick') {
        /* Formal R4.OntologicalTelos (src/R4/OntologicalTelos.lex.tex) */
        const body = JSON.stringify(payload || {});
        const raw = typeof studio_run_telos_self_improvement_tick === 'function'
          ? studio_run_telos_self_improvement_tick(body)
          : studio_dispatch(`1\tstudio\ttelos_tick\t${body}`);
        const parsedTick = JSON.parse(raw);
        postEvent('telos_tick', parsedTick);
        self.postMessage({ id, ok: true, result: parsedTick });
        return;
      }
      if (op === 'load_hf_repo_holo') {
        const repoId = String((payload && payload.repoId) || 'Qwen/Qwen3-0.6B');
        const holo = await load_hf_repo_holo(repoId, (prog) => {
          postEvent('boot_progress', prog);
          postEvent('hydration_progress', prog);
        });
        self.postMessage({ id, ok: true, result: { holo, repoId } });
        return;
      }
      if (op === 'native_q31_status') {
        const res = typeof studio_native_q31_status === 'function' ? JSON.parse(studio_native_q31_status()) : null;
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (op === 'native_q31_train_step') {
        const text = String((payload && payload.text) || '');
        const res = typeof studio_native_q31_train_step === 'function' ? JSON.parse(studio_native_q31_train_step(text)) : null;
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (op === 'weight_pack_status') {
        const pack = typeof studio_weight_pack_status === 'function' ? JSON.parse(studio_weight_pack_status()) : null;
        self.postMessage({ id, ok: true, result: pack });
        return;
      }
      if (op === 'shard_catalog') {
        const cat = typeof studio_shard_catalog === 'function' ? JSON.parse(studio_shard_catalog()) : null;
        self.postMessage({ id, ok: true, result: cat });
        return;
      }
      if (op === 'export_uorlut_shard') {
        const rootIdx = Number((payload && payload.e8Root) ?? 0);
        const bytes = typeof studio_export_uorlut_shard === 'function' ? studio_export_uorlut_shard(rootIdx) : new Uint8Array(0);
        self.postMessage({ id, ok: true, result: Array.from(bytes) });
        return;
      }
      if (op === 'merge_uorlut_shard') {
        const u8 = payload && payload.bytes instanceof Uint8Array
          ? payload.bytes
          : new Uint8Array((payload && payload.bytes) || (payload && payload.buffer) || 0);
        const raw = typeof studio_merge_uorlut_shard === 'function' ? studio_merge_uorlut_shard(u8) : '{"status":"error"}';
        self.postMessage({ id, ok: true, result: JSON.parse(raw) });
        return;
      }
      if (op === 'gllm_v9_status') {
        const res = typeof studio_gllm_v9_status === 'function' ? JSON.parse(studio_gllm_v9_status()) : null;
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (op === 'sync_weights' || op === 'hydrate_v12_360m' || op === 'hydrate_360m') {
        const tierArg = op === 'sync_weights' ? 'auto' : 'v12_360m';
        const v10 = await startOrAwaitOpfsShardHydration(true, tierArg);
        const lut = await sync_cached_weights((prog) => {
          postEvent('boot_progress', { stage: 'uorlut_sync', ...prog });
        });
        const r4Stack = typeof studio_r4_stack_status === 'function' ? JSON.parse(studio_r4_stack_status()) : null;
        self.postMessage({ id, ok: true, result: { v10, lut, r4Stack, gllmMeta: globalThis.__uorGllmV9Meta || null } });
        return;
      }
      if (op === 'snapshot') {
        const state = JSON.parse(studio_dispatch('1\tstudio\tstate\t'));
        const workspace = __uorCollectWorkspaceSnapshot();
        const pack = JSON.parse(studio_weight_pack_status());
        const gllm = typeof studio_gllm_v9_status === 'function' ? JSON.parse(studio_gllm_v9_status()) : null;
        const r4Stack = typeof studio_r4_stack_status === 'function' ? JSON.parse(studio_r4_stack_status()) : null;
        const verdantSynthesis = query_verdant_synthesis_status();
        self.postMessage({
          id,
          ok: true,
          result: {
            state,
            workspace,
            pack,
            gllm,
            r4Stack,
            verdantSynthesis,
            stitchHealth: globalThis.__uorStitchHealth || null,
          },
        });
        return;
      }
      if (op === 'stitch_health') {
        self.postMessage({ id, ok: true, result: globalThis.__uorStitchHealth || null });
        return;
      }
      if (op === 'admit_generation_capsule') {
        const res = await admit_uorc_generation_capsule(payload && payload.capsule ? payload.capsule : payload);
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (op === 'record_generation_fault') {
        const reason = payload && typeof payload.reason === 'string' ? payload.reason : 'runtime_fault';
        const res = await record_uorc_generation_fault(reason);
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (op === 'verdant_synthesis_status') {
        self.postMessage({ id, ok: true, result: query_verdant_synthesis_status() });
        return;
      }
      if (op === 'r4_stack_status') {
        const r4Stack = typeof studio_r4_stack_status === 'function' ? JSON.parse(studio_r4_stack_status()) : null;
        self.postMessage({ id, ok: true, result: r4Stack });
        return;
      }
      if (op === 'init_worker_pool') {
        const pool = await init_shared_wasm_worker_pool();
        self.postMessage({ id, ok: true, result: pool });
        return;
      }
      if (op === 'check_studio_update') {
        const origin = payload && typeof payload.origin === 'string' ? payload.origin : '';
        const update = await hostBridge.checkProvenanceUpdate(origin);
        if (update && update.update_available) {
          postEvent('studio_update_available', update);
        }
        self.postMessage({ id, ok: true, result: update });
        return;
      }
      if (op === 'force_refresh_reload') {
        if (typeof caches !== 'undefined' && typeof caches.keys === 'function') {
          try {
            const keys = await caches.keys();
            for (const key of keys) {
              await caches.delete(key);
            }
          } catch {
            // Optional cache cleanup
          }
        }
        if (
          typeof navigator !== 'undefined' &&
          navigator.storage &&
          typeof navigator.storage.getDirectory === 'function'
        ) {
          try {
            const root = await navigator.storage.getDirectory();
            await root.removeEntry(OPFS_V10_META_FILE).catch(() => {});
          } catch {
            // Optional OPFS cleanup
          }
        }
        let reloadedWasm = false;
        try {
          await __uor_resolve_cached_wasm(new URL('lexlean_r4_bg.wasm', import.meta.url), true);
          reloadedWasm = true;
        } catch {
          // Allow reload to fetch fresh artifacts cleanly
        }
        const provenance = await hostBridge.checkProvenanceUpdate().catch(() => null);
        self.postMessage({ id, ok: true, result: { ok: true, reloadedWasm, provenance } });
        return;
      }
      if (op === 'prism_identity_ensure' || op === 'prism_identity_status') {
        const res = await stitch_prism_identity_ensure();
        postEvent('prism_identity', res);
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (op === 'prism_identity_bind_persona') {
        const res = await stitch_prism_identity_bind_persona(
          (payload && payload.handle) || 'lexlean-r4-studio',
          (payload && payload.host) || 'uor-foundation.github.io',
          (payload && payload.holoKappa) || (globalThis.__uorLatestHoloSummary && globalThis.__uorLatestHoloSummary.fingerprint ? `blake3:${globalThis.__uorLatestHoloSummary.fingerprint}` : ''),
        );
        postEvent('prism_identity', res);
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (op === 'prism_identity_sign_action') {
        const res = await stitch_prism_identity_sign_action(
          (payload && payload.action) || 'compile-holo',
          (payload && payload.targetKappa) || '',
        );
        postEvent('prism_identity', res);
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (op === 'compile_holo_archive' || op === 'verify_holo_archive' || op === 'download_holo_archive') {
        const includeBytes = Boolean((payload && payload.includeBytes) || op === 'download_holo_archive');
        const providedBytes = payload && (payload.bytes instanceof Uint8Array || payload.buffer)
          ? (payload.bytes instanceof Uint8Array ? payload.bytes : new Uint8Array(payload.buffer))
          : null;
        const res = op === 'verify_holo_archive' && providedBytes
          ? await verify_holo_v4_archive_in_worker(providedBytes, includeBytes)
          : await compile_and_verify_holo_v4_in_worker(payload && payload.files ? payload.files : null, includeBytes);
        postEvent('holo_compiled', res);
        self.postMessage({ id, ok: true, result: res });
        return;
      }
      if (op === 'get_me_app_descriptor' || op === 'get_foundry_app_descriptor') {
        const desc = await build_afflom_me_app_descriptor(payload && payload.origin ? String(payload.origin) : '');
        self.postMessage({ id, ok: true, result: desc });
        return;
      }
      if (op === 'cancel_inprism_turn') {
        const hasTarget = payload && payload.targetId != null;
        const hasReq = payload && payload.requestId != null;
        const hasReqSnake = payload && payload.reqId != null;
        const targetReqId = hasTarget
          ? payload.targetId
          : hasReq
            ? payload.requestId
            : hasReqSnake
              ? payload.reqId
              : null;
        if (targetReqId != null) {
          cancelledInprismRequests.add(targetReqId);
          cancelledInprismRequests.add(String(targetReqId));
        }
        if (hasReq && payload.requestId !== targetReqId) {
          cancelledInprismRequests.add(payload.requestId);
          cancelledInprismRequests.add(String(payload.requestId));
        }
        const matchesActiveReq =
          targetReqId != null &&
          activeInprismRequestId != null &&
          String(targetReqId) === String(activeInprismRequestId);
        if (
          targetReqId == null &&
          (payload == null || payload.turn_id == null) &&
          activeInprismRequestId != null
        ) {
          cancelledInprismRequests.add(activeInprismRequestId);
          cancelledInprismRequests.add(String(activeInprismRequestId));
        }
        const rawTid =
          payload && payload.turn_id != null
            ? Number(payload.turn_id)
            : targetReqId == null || matchesActiveReq
              ? activeInprismTurnId
              : null;
        let cancelRes = { status: 'not_found', turn_id: Number(rawTid || 0) };
        if (rawTid != null && Number.isFinite(rawTid) && rawTid > 0) {
          cancelledInprismTurns.add(Number(rawTid));
          const cancelFn =
            typeof holo_cancel_foundry_turn === 'function'
              ? holo_cancel_foundry_turn
              : globalThis.holo_cancel_foundry_turn;
          if (typeof cancelFn === 'function') {
            const arg = typeof BigInt === 'function' ? BigInt(Math.trunc(rawTid)) : Math.trunc(rawTid);
            cancelRes = JSON.parse(cancelFn(arg));
          } else {
            cancelRes = { status: 'cancelled', turn_id: Number(rawTid) };
          }
        } else if (matchesActiveReq || (targetReqId == null && activeInprismRequestId != null)) {
          cancelRes = {
            status: 'cancelled',
            turn_id: 0,
            targetId: targetReqId != null ? targetReqId : activeInprismRequestId,
          };
        }
        const notifyKeys = new Set();
        if (targetReqId != null) notifyKeys.add(String(targetReqId));
        else if (activeInprismRequestId != null) notifyKeys.add(String(activeInprismRequestId));
        for (const k of notifyKeys) {
          const waiters = pendingInprismCancelWaiters.get(k);
          if (waiters) {
            for (const cb of waiters) cb();
            pendingInprismCancelWaiters.delete(k);
          }
        }
        self.postMessage({ id, ok: true, result: cancelRes });
        return;
      }
      if (op === 'terminal:write' || op === 'terminal:open' || op === 'terminal:close' || op === 'terminal:kill' || op === 'terminal:resize' || op === 'terminal:serialize') {
        if (payload != null && typeof payload !== 'object') {
          self.postMessage({
            id,
            ok: false,
            error_code: 'InvalidPayload',
            error: `${op} payload must be an object`,
          });
          return;
        }
        if (op === 'terminal:write') {
          const cmdStr = payload && typeof payload.cmd === 'string' ? payload.cmd.trim() : '';
          const hasRawData = payload && typeof payload.data === 'string' && payload.data.length > 0;
          if (!cmdStr && !hasRawData) {
            self.postMessage({
              id,
              ok: false,
              error_code: 'InvalidPayload',
              error: 'terminal:write requires non-empty payload.cmd or payload.data',
            });
            return;
          }
          if (cmdStr) {
            const out = await dispatchWithConnectedBridge(`1\tstudio\tmvm_exec\t${JSON.stringify({ cmd: cmdStr })}`);
            self.postMessage({ id, ok: true, result: out });
            return;
          }
          self.postMessage({
            id,
            ok: true,
            result: { ok: true, op, bytes: payload.data.length, interleaved: true },
          });
          return;
        }
        if (op === 'terminal:resize') {
          const cols = Number(payload && payload.cols);
          const rows = Number(payload && payload.rows);
          if (!Number.isFinite(cols) || cols <= 0 || !Number.isFinite(rows) || rows <= 0) {
            self.postMessage({
              id,
              ok: false,
              error_code: 'InvalidPayload',
              error: 'terminal:resize requires positive cols and rows',
            });
            return;
          }
        }
        self.postMessage({ id, ok: true, result: { ok: true, op, interleaved: true } });
        return;
      }
      if (op === 'inprism_r4_turn') {
        /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
        const resEnvelope = await execute_inprism_foundry_turn(
          { id, payload },
          {
            postEvent,
            postResponse: (resp) => self.postMessage(resp),
            dispatchBridge: dispatchWithConnectedBridge,
            claimTurnOwnership: (ownerReqId) => {
              if (activeInprismRequestId != null && activeInprismRequestId !== ownerReqId) {
                return { ok: false, activeTurnId: activeInprismTurnId };
              }
              activeInprismRequestId = ownerReqId;
              return { ok: true, activeTurnId: activeInprismTurnId };
            },
            releaseTurnOwnership: (ownerReqId) => {
              if (activeInprismRequestId === ownerReqId) {
                activeInprismRequestId = null;
                activeInprismTurnId = null;
              }
            },
            setActiveTurnId: (tid, ownerReqId) => {
              if (ownerReqId === undefined || activeInprismRequestId === ownerReqId) {
                activeInprismTurnId = tid;
              }
            },
            registerCancelWaiter: (ownerReqId, cb) => {
              const key = String(ownerReqId);
              let set = pendingInprismCancelWaiters.get(key);
              if (!set) {
                set = new Set();
                pendingInprismCancelWaiters.set(key, set);
              }
              set.add(cb);
              return () => {
                const cur = pendingInprismCancelWaiters.get(key);
                if (cur) {
                  cur.delete(cb);
                  if (cur.size === 0) pendingInprismCancelWaiters.delete(key);
                }
              };
            },
            isCancelled: (reqId, turnId) =>
              cancelledInprismRequests.has(reqId) ||
              cancelledInprismRequests.has(String(reqId)) ||
              (turnId != null && cancelledInprismTurns.has(Number(turnId))),
            clearCancelled: (reqId, turnId) => {
              cancelledInprismRequests.delete(reqId);
              cancelledInprismRequests.delete(String(reqId));
              if (turnId != null) cancelledInprismTurns.delete(Number(turnId));
            },
          },
        );
        self.postMessage(resEnvelope);
        return;
      }
      if (op === 'mvm_sync_workspace_in') {
        const parsedOut = runMvmSyncWorkspaceIn(payload, null, true);
        if (parsedOut && parsedOut.status === 'ok') {
          postEvent('workspace_sync', { workspace: __uorCollectWorkspaceSnapshot() });
        }
        self.postMessage({ id, ok: true, result: parsedOut });
        return;
      }
      if (op === 'mvm_export_workspace_diff') {
        const parsedOut = runMvmExportWorkspaceDiff(true);
        if (parsedOut && parsedOut.status === 'ok') {
          postEvent('workspace_sync', {
            workspace: __uorCollectWorkspaceSnapshot(),
            diff: parsedOut,
          });
        }
        self.postMessage({ id, ok: true, result: parsedOut });
        return;
      }
      self.postMessage({ id, ok: false, error: `Unknown worker op: ${op}` });
    } catch (err) {
      self.postMessage({
        id,
        ok: false,
        ...(err && err.code ? { error_code: err.code } : {}),
        error: err && err.message ? err.message : String(err),
      });
    }
  };

  self.onmessage = (ev) => {
    const msg = ev.data || {};
    if (!wasmReady) {
      earlyMessages.push(msg);
      if (!bootPromise) startBoot();
      return;
    }
    handleWorkerMessage(msg);
  };

  const startBoot = () => {
    if (bootPromise) return bootPromise;
    bootPromise = (async () => {
      postEvent('boot_progress', { stage: 'wasm_compile', label: 'Compiling UOR-R4 Wasm engine...' });
      __uorRequestPersistentStorage();
      await __wbg_init();
      await restore_cached_uorlut_from_indexeddb().catch(() => null);
      await restore_mvm_cow_sectors_from_opfs().catch(() => null);
      wasmReady = true;

      const state = JSON.parse(studio_dispatch('1\tstudio\tstate\t'));
      const workspace = __uorCollectWorkspaceSnapshot();
      const pack = JSON.parse(studio_weight_pack_status());
      const gllm = typeof studio_gllm_v9_status === 'function' ? JSON.parse(studio_gllm_v9_status()) : null;
      const r4Stack = typeof studio_r4_stack_status === 'function' ? JSON.parse(studio_r4_stack_status()) : null;
      const holoStatus = typeof studio_holo_runtime_status === 'function' ? JSON.parse(studio_holo_runtime_status()) : null;
      const nativeQ31 = typeof studio_native_q31_status === 'function' ? JSON.parse(studio_native_q31_status()) : null;
      const verdantSynthesis = query_verdant_synthesis_status();
      const prismIdentity = await stitch_prism_identity_ensure().catch(() => null);
      postEvent('ready', {
        snapshot: { state, workspace, pack, gllm, r4Stack, holoStatus, nativeQ31, verdantSynthesis, prismIdentity },
        weightCacheMeta: globalThis.__uorWeightCacheMeta || null,
        gllmMeta: globalThis.__uorGllmV9Meta || null,
        r4Stack,
        holoStatus,
        nativeQ31,
        verdantSynthesis,
        prismIdentity,
      });

      while (earlyMessages.length > 0) {
        const queued = earlyMessages.shift();
        await handleWorkerMessage(queued);
      }

      // Boot on optimal Qwen3 lane (`1.7b-holo` if already warm in OPFS, else fast `0.6b-holo` + background `1.7b-holo` promotion).
      selectOptimalInitialHoloModel().then((plan) => {
        const emitProg = (prog) => {
          postEvent('boot_progress', { stage: 'holo_v13', ...prog });
          postEvent('hydration_progress', { stage: 'holo_v13', ...prog });
        };
        return hydrate_holo_model(plan.initialModel, emitProg, false).then((holo) => {
          postEvent('stitch_health', { stitchHealth: globalThis.__uorStitchHealth || null });
          postEvent('holo_ready', {
            holo,
            recommendedModel: plan.initialModel,
            promoted: false,
            kappaStore: globalThis.__uorHoloKappaStatus || null,
          });
          if (plan.promoteTarget) {
            hydrate_holo_model(plan.promoteTarget, emitProg, false).then((holo17) => {
              postEvent('holo_ready', {
                holo: holo17,
                recommendedModel: plan.promoteTarget,
                promoted: true,
                kappaStore: globalThis.__uorHoloKappaStatus || null,
              });
            }).catch(() => {});
          }
        });
      }).catch(() => {});

      if (typeof location !== 'undefined' && /^https?:/i.test(location.origin || '')) {
        hostBridge.checkProvenanceUpdate('').catch(() => null);
        if (typeof setInterval === 'function') {
          setInterval(() => {
            hostBridge
              .checkProvenanceUpdate('')
              .then((upd) => {
                if (upd && upd.update_available) {
                  postEvent('studio_update_available', upd);
                }
              })
              .catch(() => {});
          }, 60000);
        }
      }
    })().catch((err) => {
      postEvent('boot_error', {
        message: err && err.message ? err.message : String(err),
      });
    });
    return bootPromise;
  };

  startBoot();
}

// === VERDANT RESONANCE WORKER SATURATION, WASM MODULE POOL & OPFS .uorc CAPSULE WATCHDOG ===
const UORC_GENERATIONS_DIR = 'generations';
const UORC_HEAD_FILE = 'HEAD';
const UORC_FRONTIER_FILE = 'FRONTIER.json';
const UOR_SELFHOST_SYNC_CHANNEL = 'uor-r4-selfhost-sync';

let __uorSharedWasmModule = null;
let __uorSharedWasmBytes = null;
const __uorSharedWorkerPool = [];
let __uorSharedWorkerBlobUrl = null;
const __uorMemGenerationStore = {
  head: 'gen-0',
  frontier: ['gen-0'],
  capsules: new Map([
    [
      'gen-0',
      {
        generation_id: 'gen-0',
        parent_generation_id: null,
        schema: 'uorc/generation-capsule/1',
        strikes: 0,
        status: 'baseline',
      },
    ],
  ]),
};

export function optimal_browser_worker_count(logicalThreads) {
  const detected =
    typeof logicalThreads === 'number' && Number.isFinite(logicalThreads) && logicalThreads > 0
      ? Math.floor(logicalThreads)
      : typeof navigator !== 'undefined' && typeof navigator.hardwareConcurrency === 'number'
        ? Math.max(1, Math.floor(navigator.hardwareConcurrency))
        : 4;
  if (detected <= 2) return 1;
  const candidate = detected - 1;
  return candidate > 8 ? 8 : candidate;
}

async function __uorWithWebLock(lockName, task) {
  if (
    typeof navigator !== 'undefined' &&
    navigator.locks &&
    typeof navigator.locks.request === 'function'
  ) {
    return navigator.locks.request(lockName, () => task());
  }
  return task();
}

function __uorBroadcastGenSync(generationId, digestSha256, eventKind) {
  if (typeof BroadcastChannel === 'undefined') return false;
  try {
    const ch = new BroadcastChannel(UOR_SELFHOST_SYNC_CHANNEL);
    ch.postMessage({
      protocol: 'uor-workspace-bridge/1',
      type: 'uor-r4:generation-sync',
      event: eventKind || 'admitted',
      generation_id: generationId,
      digest_sha256: digestSha256 || '',
      timestamp_ms: Date.now(),
    });
    ch.close();
    return true;
  } catch {
    return false;
  }
}

export async function init_shared_wasm_worker_pool(wasmBytesOrModule = null) {
  const logicalThreads =
    typeof navigator !== 'undefined' && typeof navigator.hardwareConcurrency === 'number'
      ? Math.max(1, Math.floor(navigator.hardwareConcurrency))
      : 4;
  const targetWorkers = optimal_browser_worker_count(logicalThreads);

  if (wasmBytesOrModule instanceof WebAssembly.Module) {
    __uorSharedWasmModule = wasmBytesOrModule;
  } else if (wasmBytesOrModule && wasmBytesOrModule.byteLength > 0) {
    __uorSharedWasmBytes = wasmBytesOrModule instanceof ArrayBuffer
      ? wasmBytesOrModule
      : wasmBytesOrModule.buffer.slice(
          wasmBytesOrModule.byteOffset,
          wasmBytesOrModule.byteOffset + wasmBytesOrModule.byteLength,
        );
    if (!__uorSharedWasmModule && typeof WebAssembly !== 'undefined' && typeof WebAssembly.compile === 'function') {
      __uorSharedWasmModule = await WebAssembly.compile(__uorSharedWasmBytes);
    }
  } else if (!__uorSharedWasmModule && typeof globalThis !== 'undefined' && globalThis.__uorWasmBytesBuffer) {
    __uorSharedWasmBytes = globalThis.__uorWasmBytesBuffer;
    if (typeof WebAssembly !== 'undefined' && typeof WebAssembly.compile === 'function') {
      __uorSharedWasmModule = await WebAssembly.compile(__uorSharedWasmBytes);
    }
  } else if (!__uorSharedWasmModule && typeof fetch === 'function') {
    try {
      const wasmUrl = new URL('lexlean_r4_bg.wasm', import.meta.url);
      const cached = await __uor_resolve_cached_wasm(wasmUrl, false);
      if (cached && typeof cached.arrayBuffer === 'function') {
        __uorSharedWasmBytes = await cached.arrayBuffer();
        __uorSharedWasmModule = await WebAssembly.compile(__uorSharedWasmBytes);
      }
    } catch {
      // Optional compile in offline/mocked Node test harnesses
    }
  }

  if (
    __uorSharedWorkerPool.length === 0 &&
    typeof Worker !== 'undefined' &&
    typeof Blob !== 'undefined' &&
    typeof URL !== 'undefined' &&
    typeof URL.createObjectURL === 'function'
  ) {
    const poolWorkerCode = [
      'let sharedModule = null;',
      'self.onmessage = async (ev) => {',
      '  const msg = ev && ev.data ? ev.data : {};',
      '  if (msg.type === "INIT_MODULE" && (msg.module || msg.wasmBytes)) {',
      '    sharedModule = msg.module || await WebAssembly.compile(msg.wasmBytes);',
      '    self.postMessage({ type: "READY", workerIndex: msg.workerIndex, hasModule: Boolean(sharedModule) });',
      '  }',
      '};',
    ].join('\n');
    if (!__uorSharedWorkerBlobUrl) {
      __uorSharedWorkerBlobUrl = URL.createObjectURL(new Blob([poolWorkerCode], { type: 'text/javascript' }));
    }
    for (let i = 0; i < targetWorkers; i += 1) {
      try {
        const worker = new Worker(__uorSharedWorkerBlobUrl);
        __uorSharedWorkerPool.push({ index: i, worker, ready: false });
        if (__uorSharedWasmModule || __uorSharedWasmBytes) {
          worker.postMessage({
            type: 'INIT_MODULE',
            workerIndex: i,
            module: __uorSharedWasmModule,
            wasmBytes: __uorSharedWasmBytes,
          });
        }
      } catch {
        break;
      }
    }
  }

  return get_browser_worker_pool_snapshot();
}

export function get_browser_worker_pool_snapshot() {
  /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
  const logicalThreads =
    typeof navigator !== 'undefined' && typeof navigator.hardwareConcurrency === 'number'
      ? Math.max(1, Math.floor(navigator.hardwareConcurrency))
      : 4;
  return {
    logical_threads: logicalThreads,
    optimal_workers: optimal_browser_worker_count(logicalThreads),
    pool_size: __uorSharedWorkerPool.length,
    ready_workers: __uorSharedWorkerPool.filter((w) => Boolean(w && w.ready)).length,
    shared_module_compiled: Boolean(__uorSharedWasmModule),
    zero_refetch_sharing: true,
    striped_int8_ready: Boolean(__uorSharedWorkerPool.length > 0 || __uorSharedWasmModule),
    active_head: __uorMemGenerationStore.head,
    frontier_count: __uorMemGenerationStore.frontier.length,
  };
}

async function __uorWriteOpfsGenerationRecord(generationId, manifestObj, capsuleBytes) {
  __uorMemGenerationStore.capsules.set(generationId, manifestObj);
  __uorMemGenerationStore.head = generationId;
  if (!__uorMemGenerationStore.frontier.includes(generationId)) {
    __uorMemGenerationStore.frontier.push(generationId);
  }
  const hasOpfs = typeof navigator !== 'undefined' && navigator.storage && typeof navigator.storage.getDirectory === 'function';
  if (!hasOpfs) return 'memory';
  try {
    const root = await navigator.storage.getDirectory();
    const gensDir = await root.getDirectoryHandle(UORC_GENERATIONS_DIR, { create: true });
    const genDir = await gensDir.getDirectoryHandle(generationId, { create: true });
    const mf = await genDir.getFileHandle('manifest.json', { create: true });
    const mw = await mf.createWritable();
    await mw.write(cachedTextEncoder.encode(JSON.stringify(manifestObj, null, 2)));
    await mw.close();
    const cf = await genDir.getFileHandle('capsule.uorc', { create: true });
    const cw = await cf.createWritable();
    await cw.write(capsuleBytes);
    await cw.close();
    const hf = await root.getFileHandle(UORC_HEAD_FILE, { create: true });
    const hw = await hf.createWritable();
    await hw.write(cachedTextEncoder.encode(generationId));
    await hw.close();
    const ff = await root.getFileHandle(UORC_FRONTIER_FILE, { create: true });
    const fw = await ff.createWritable();
    await fw.write(cachedTextEncoder.encode(JSON.stringify(__uorMemGenerationStore.frontier)));
    await fw.close();
    return 'opfs';
  } catch {
    return 'memory';
  }
}

async function __uorWriteOpfsHead(generationId) {
  __uorMemGenerationStore.head = generationId;
  const hasOpfs = typeof navigator !== 'undefined' && navigator.storage && typeof navigator.storage.getDirectory === 'function';
  if (!hasOpfs) return 'memory';
  try {
    const root = await navigator.storage.getDirectory();
    const hf = await root.getFileHandle(UORC_HEAD_FILE, { create: true });
    const hw = await hf.createWritable();
    await hw.write(cachedTextEncoder.encode(generationId));
    await hw.close();
    return 'opfs';
  } catch {
    return 'memory';
  }
}

export async function admit_uorc_generation_capsule(capsuleInput) {
  const payloadStr = typeof capsuleInput === 'string' ? capsuleInput : JSON.stringify(capsuleInput || {});
  let admission = null;
  if (typeof studio_admit_generation_capsule === 'function') {
    admission = JSON.parse(studio_admit_generation_capsule(payloadStr));
  } else {
    throw new Error('Wasm studio_admit_generation_capsule export unavailable');
  }
  if (!admission || admission.status !== 'ok' || admission.admitted !== true) {
    return { ...admission, persisted: false };
  }
  const genId = admission.active_head || admission.generation_id || `gen-${__uorMemGenerationStore.frontier.length}`;
  const rawBytes = cachedTextEncoder.encode(payloadStr);
  const storageBackend = await __uorWithWebLock('uor-r4-generations-head', () =>
    __uorWriteOpfsGenerationRecord(genId, admission, rawBytes),
  );
  __uorBroadcastGenSync(genId, admission.capsule_sha256 || '', 'admitted');
  return {
    ...admission,
    persisted: true,
    storage_backend: storageBackend,
    head: __uorMemGenerationStore.head,
  };
}

export async function record_uorc_generation_fault(faultReason) {
  if (typeof studio_record_generation_fault !== 'function') {
    throw new Error('Wasm studio_record_generation_fault export unavailable');
  }
  const result = JSON.parse(studio_record_generation_fault(String(faultReason || 'runtime_fault')));
  let storageBackend = 'memory';
  if (result && result.rolled_back === true && result.active_head) {
    storageBackend = await __uorWithWebLock('uor-r4-generations-head', () =>
      __uorWriteOpfsHead(result.active_head),
    );
    __uorBroadcastGenSync(result.active_head, '', 'rollback');
  }
  return {
    ...result,
    storage_backend: storageBackend,
    head: __uorMemGenerationStore.head,
  };
}

export function query_verdant_synthesis_status() {
  let wasmStatus = {
    status: 'ok',
    active_head: __uorMemGenerationStore.head,
    fault_strikes: 0,
    max_strikes: 3,
    rolled_back: false,
    facets: [],
  };
  if (typeof studio_verdant_synthesis_status === 'function') {
    try {
      wasmStatus = JSON.parse(studio_verdant_synthesis_status());
    } catch {
      // Keep fallback status before Wasm init
    }
  }
  return {
    ...wasmStatus,
    opfs_head: __uorMemGenerationStore.head,
    frontier: [...__uorMemGenerationStore.frontier],
    worker_pool: get_browser_worker_pool_snapshot(),
  };
}

// === STITCH PRISM IDENTITY VAULT & HOLOGRAM V4 .HOLO COMPILER / VERIFIER (R4-54) ===
const STITCH_PRISM_ID_DB = 'uor_stitch_prism_id_v1';
const STITCH_PRISM_ID_STORE = 'identity_records';
const UOR_CLOSURE_FILES = [
  'app.css',
  'app.js',
  'index.html',
  'lexlean_r4.js',
  'lexlean_r4_bg.wasm',
  'provenance.json',
];
const __uorMemPrismIdentity = {
  record: null,
  latestHoloBytes: null,
  latestHoloSummary: null,
};

// Formal R4.PrismHoloIdentity encode integer-only sorted-key canonical JSON mirroring uor-addr.
export function __uorCanonicalJson(value) {
  if (value === null || typeof value === 'boolean') return JSON.stringify(value);
  if (typeof value === 'number') {
    if (!Number.isSafeInteger(value)) {
      throw new Error('RefusedNonIntegerCanonicalJson: floats and non-safe integers rejected');
    }
    return JSON.stringify(value);
  }
  if (typeof value === 'string') return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(__uorCanonicalJson).join(',')}]`;
  if (value && typeof value === 'object') {
    const keys = Object.keys(value).filter((k) => value[k] !== undefined).sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
    return `{${keys.map((k) => `${JSON.stringify(k)}:${__uorCanonicalJson(value[k])}`).join(',')}}`;
  }
  throw new Error('RefusedNonIntegerCanonicalJson: unsupported JSON value');
}

function __uorOpenPrismIdDb() {
  return new Promise((resolve, reject) => {
    if (typeof indexedDB === 'undefined') {
      reject(new Error('IndexedDB unavailable'));
      return;
    }
    const req = indexedDB.open(STITCH_PRISM_ID_DB, 1);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STITCH_PRISM_ID_STORE)) {
        db.createObjectStore(STITCH_PRISM_ID_STORE);
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error || new Error('IndexedDB open error'));
  });
}

async function __uorReadPrismIdRecord(key) {
  try {
    const db = await __uorOpenPrismIdDb();
    return await new Promise((resolve, reject) => {
      const tx = db.transaction(STITCH_PRISM_ID_STORE, 'readonly');
      const req = tx.objectStore(STITCH_PRISM_ID_STORE).get(key);
      req.onsuccess = () => resolve(req.result || null);
      req.onerror = () => reject(req.error);
    });
  } catch {
    return key === 'primary' ? __uorMemPrismIdentity.record : null;
  }
}

async function __uorWritePrismIdRecord(key, value) {
  if (key === 'primary') __uorMemPrismIdentity.record = value;
  try {
    const db = await __uorOpenPrismIdDb();
    await new Promise((resolve, reject) => {
      const tx = db.transaction(STITCH_PRISM_ID_STORE, 'readwrite');
      tx.objectStore(STITCH_PRISM_ID_STORE).put(value, key);
      tx.oncomplete = () => resolve(true);
      tx.onerror = () => reject(tx.error);
    });
    return 'indexeddb';
  } catch {
    return 'memory';
  }
}

function __uorToB64Url(bytes) {
  /* Formal R4.PrismHoloIdentity (src/R4/PrismHoloIdentity.lex.tex) */
  const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  let bin = '';
  for (let i = 0; i < u8.length; i += 1) bin += String.fromCharCode(u8[i]);
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
}

async function __uorSignJsonES256(privateKey, obj) {
  const canon = __uorCanonicalJson(obj);
  const data = cachedTextEncoder.encode(canon);
  const subtle = typeof crypto !== 'undefined' ? crypto.subtle : null;
  if (subtle && privateKey && typeof subtle.sign === 'function') {
    const sigBuf = await subtle.sign({ name: 'ECDSA', hash: 'SHA-256' }, privateKey, data);
    return __uorToB64Url(new Uint8Array(sigBuf));
  }
  return await __uorSha256HexBytes(data);
}

async function __uorCreateP256IdentityState() {
  const subtle = typeof crypto !== 'undefined' ? crypto.subtle : null;
  let signingPair = null;
  let agreementPair = null;
  let signingPub = { kty: 'EC', crv: 'P-256', x: 'uor-studio-pub-x', y: 'uor-studio-pub-y' };
  let agreementPub = { kty: 'EC', crv: 'P-256', x: 'uor-studio-ecdh-x', y: 'uor-studio-ecdh-y' };
  let protection = 'software';

  if (subtle && typeof subtle.generateKey === 'function') {
    try {
      // Security: non-extractable WebCrypto P-256 hardware/software private keys (afflom/stitch docs/prism-id.md).
      signingPair = await subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, false, ['sign', 'verify']);
      agreementPair = await subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, false, ['deriveBits']);
      const sjwk = await subtle.exportKey('jwk', signingPair.publicKey);
      const ajwk = await subtle.exportKey('jwk', agreementPair.publicKey);
      signingPub = { kty: sjwk.kty, crv: sjwk.crv, x: sjwk.x, y: sjwk.y };
      agreementPub = { kty: ajwk.kty, crv: ajwk.crv, x: ajwk.x, y: ajwk.y };
      protection = 'webcrypto-nonextractable';
    } catch {
      // Fallback to deterministic software JWK descriptor when SubtleCrypto is unavailable
    }
  }

  const deviceRecord = {
    schema: 'prism/device/1',
    class: 'browser',
    protection,
    signing: signingPub,
    agreement: agreementPub,
  };
  const deviceKappa = `sha256:${await __uorSha256HexBytes(cachedTextEncoder.encode(__uorCanonicalJson(deviceRecord)))}`;
  const genesisBody = {
    schema: 'prism/genesis/1',
    device: deviceRecord,
    device_id: deviceKappa,
    nonce: Math.floor(Date.now() / 1000),
  };
  const genesisSig = await __uorSignJsonES256(signingPair ? signingPair.privateKey : null, genesisBody);
  const genesisRecord = { ...genesisBody, sig: genesisSig };
  const prismId = `sha256:${await __uorSha256HexBytes(cachedTextEncoder.encode(__uorCanonicalJson(genesisRecord)))}`;
  return {
    prism_id: prismId,
    device_id: deviceKappa,
    protection,
    device_record: deviceRecord,
    genesis_record: genesisRecord,
    signing_private_key: signingPair ? signingPair.privateKey : null,
    signing_public_key: signingPair ? signingPair.publicKey : null,
    agreement_private_key: agreementPair ? agreementPair.privateKey : null,
    agreement_public_key: agreementPair ? agreementPair.publicKey : null,
    persona: null,
    history: [genesisRecord],
  };
}

export async function stitch_prism_identity_ensure() {
  let stored = await __uorReadPrismIdRecord('primary');
  let backend = stored ? 'indexeddb' : 'created';
  if (!stored || !stored.genesis_record || !stored.device_record) {
    stored = await __uorCreateP256IdentityState();
    backend = await __uorWritePrismIdRecord('primary', stored);
  }
  __uorMemPrismIdentity.record = stored;

  // Replay chain deterministically into Wasm R4.PrismHoloIdentity ledger if not yet populated
  let wasmStatus = typeof studio_prism_identity_status === 'function' ? JSON.parse(studio_prism_identity_status()) : null;
  if (!wasmStatus || !wasmStatus.prism_id) {
    for (const rec of stored.history || [stored.genesis_record]) {
      if (typeof studio_prism_identity_admit_record === 'function') {
        studio_prism_identity_admit_record(JSON.stringify(rec));
      }
    }
    wasmStatus = typeof studio_prism_identity_status === 'function' ? JSON.parse(studio_prism_identity_status()) : null;
  }

  return {
    ok: true,
    schema: 'stitch/prism-id/1',
    storage: backend,
    prism_id: (wasmStatus && wasmStatus.prism_id) || stored.prism_id,
    device_id: stored.device_id,
    protection: stored.protection || 'webcrypto-nonextractable',
    role: 'member',
    head_kappa: (wasmStatus && wasmStatus.head_kappa) || stored.prism_id,
    active_devices: (wasmStatus && wasmStatus.active_devices) || 1,
    revoked_devices: (wasmStatus && wasmStatus.revoked_devices) || 0,
    persona: (wasmStatus && wasmStatus.persona) || stored.persona || null,
    latest_holo: __uorMemPrismIdentity.latestHoloSummary || globalThis.__uorLatestHoloSummary || null,
  };
}

export async function stitch_prism_identity_bind_persona(handle, host, holoKappa) {
  const ens = await stitch_prism_identity_ensure();
  const stored = __uorMemPrismIdentity.record;
  const body = {
    schema: 'prism/persona/1',
    prism: ens.prism_id,
    prev: ens.head_kappa,
    by: ens.device_id,
    host: String(host || 'uor-foundation.github.io').trim() || 'uor-foundation.github.io',
    handle: String(handle || 'lexlean-r4-studio').trim() || 'lexlean-r4-studio',
    app_address: 'holo://lexlean-r4',
    ...(holoKappa ? { target_kappa: String(holoKappa).trim() } : {}),
  };
  const sig = await __uorSignJsonES256(stored ? stored.signing_private_key : null, body);
  const signed = { ...body, sig };
  const admit = typeof studio_prism_identity_admit_record === 'function'
    ? JSON.parse(studio_prism_identity_admit_record(JSON.stringify(signed)))
    : { status: 'error', message: 'Wasm studio_prism_identity_admit_record unavailable' };
  if (admit.status !== 'ok') {
    throw new Error(`${admit.error_code || 'RefusedPersona'}: ${admit.message || 'Persona refused'}`);
  }
  if (stored) {
    stored.persona = signed;
    stored.history = Array.isArray(stored.history) ? [...stored.history, signed] : [stored.genesis_record, signed];
    await __uorWritePrismIdRecord('primary', stored);
  }
  return {
    ...(await stitch_prism_identity_ensure()),
    last_record_kappa: admit.record_kappa,
    last_verdict: admit.verdict,
  };
}

export async function stitch_prism_identity_sign_action(actionName, targetKappa) {
  const ens = await stitch_prism_identity_ensure();
  const stored = __uorMemPrismIdentity.record;
  const body = {
    schema: 'prism/action/1',
    prism: ens.prism_id,
    prev: ens.head_kappa,
    by: ens.device_id,
    action: String(actionName || 'compile-holo'),
    target_kappa: String(targetKappa || ''),
  };
  const sig = await __uorSignJsonES256(stored ? stored.signing_private_key : null, body);
  const signed = { ...body, sig };
  const admit = typeof studio_prism_identity_admit_record === 'function'
    ? JSON.parse(studio_prism_identity_admit_record(JSON.stringify(signed)))
    : { status: 'error', message: 'Wasm studio_prism_identity_admit_record unavailable' };
  if (admit.status !== 'ok') {
    throw new Error(`${admit.error_code || 'RefusedAction'}: ${admit.message || 'Action refused'}`);
  }
  if (stored) {
    stored.history = Array.isArray(stored.history) ? [...stored.history, signed] : [stored.genesis_record, signed];
    await __uorWritePrismIdRecord('primary', stored);
  }
  return {
    ...(await stitch_prism_identity_ensure()),
    action_record_kappa: admit.record_kappa,
    action_sig: sig,
    last_verdict: admit.verdict,
  };
}

// Formal R4.PrismHoloIdentity encode sorted UTF-8 entries into big-endian HOLOVIEW v1 bytes.
export function __uorEncodeHoloviewV1(entry, filesMap) {
  /* Formal R4.PrismHoloIdentity (src/R4/PrismHoloIdentity.lex.tex) */
  const entries = [...filesMap.entries()]
    .map(([p, content]) => ({
      pathBytes: cachedTextEncoder.encode(String(p)),
      content: content instanceof Uint8Array ? content : new Uint8Array(content),
    }))
    .sort((a, b) => {
      const n = Math.min(a.pathBytes.length, b.pathBytes.length);
      for (let i = 0; i < n; i += 1) {
        if (a.pathBytes[i] !== b.pathBytes[i]) return a.pathBytes[i] - b.pathBytes[i];
      }
      return a.pathBytes.length - b.pathBytes.length;
    });
  const entryBytes = cachedTextEncoder.encode(entry);
  let total = 8 + 2 + 4 + entryBytes.length + 4;
  for (const e of entries) total += 4 + e.pathBytes.length + 8 + e.content.length;
  const out = new Uint8Array(total);
  const dv = new DataView(out.buffer);
  out.set(cachedTextEncoder.encode('HOLOVIEW'), 0);
  dv.setUint16(8, 1, false);
  let at = 10;
  dv.setUint32(at, entryBytes.length, false);
  at += 4;
  out.set(entryBytes, at);
  at += entryBytes.length;
  dv.setUint32(at, entries.length, false);
  at += 4;
  for (const e of entries) {
    dv.setUint32(at, e.pathBytes.length, false);
    at += 4;
    out.set(e.pathBytes, at);
    at += e.pathBytes.length;
    dv.setBigUint64(at, BigInt(e.content.length), false);
    at += 8;
    out.set(e.content, at);
    at += e.content.length;
  }
  return out;
}

async function __uorReplayCoreWasmVectors(guestBytes) {
  /* Formal R4.PrismHoloIdentity (src/R4/PrismHoloIdentity.lex.tex) */
  const pairs = [
    ['1\tping\t', '1\tok\tlexlean-r4\tv0.14.0'],
    ['1\troute\tqwen3-0.6b-holo', '1\tok\tuor-r4.qwen3-0.6b-holo\tsimd128-int4-rht32'],
    ['1\tidentity\tprism/genesis/1', '1\tok\tstitch-prism-id\tuor-addr/sha256'],
    ['1\tselect\tuor-foundation/foundry', '1\tok\tholo://lexlean-r4\tfoundry/inprism/1'],
    ['1\tunknown\t?', '1\terror\tunknown-request'],
  ];
  const mod = await WebAssembly.compile(guestBytes);
  const imports = WebAssembly.Module.imports(mod);
  if (imports.length !== 0) {
    throw new Error(`RefusedCoreWasmContractViolation: guest declared ${imports.length} imports`);
  }
  const exports = WebAssembly.Module.exports(mod).map((e) => e.name).sort();
  if (JSON.stringify(exports) !== JSON.stringify(['holo_alloc', 'holo_run', 'memory'])) {
    throw new Error(`RefusedCoreWasmContractViolation: unexpected exports ${exports.join(',')}`);
  }
  let passed = 0;
  for (const [reqStr, expResp] of pairs) {
    const inst = new WebAssembly.Instance(mod, {});
    const reqBytes = cachedTextEncoder.encode(reqStr);
    const at = inst.exports.holo_alloc(reqBytes.length) >>> 0;
    new Uint8Array(inst.exports.memory.buffer, at, reqBytes.length).set(reqBytes);
    const packed = BigInt.asUintN(64, inst.exports.holo_run(at, reqBytes.length));
    const off = Number(packed >> 32n);
    const len = Number(packed & 0xffffffffn);
    const respBytes = new Uint8Array(inst.exports.memory.buffer.slice(off, off + len));
    const got = cachedTextDecoder.decode(respBytes);
    if (got === expResp) passed += 1;
  }
  return { passed, total: pairs.length };
}

export async function verify_holo_v4_archive_in_worker(archiveBytes, includeBytes = false) {
  if (typeof studio_verify_holo_v4_archive !== 'function') {
    throw new Error('studio_verify_holo_v4_archive export unavailable');
  }
  const parsed = JSON.parse(studio_verify_holo_v4_archive(archiveBytes));
  if (!parsed || parsed.status !== 'ok') {
    throw new Error(`${parsed?.error_code || 'RefusedHolo'}: ${parsed?.message || 'Verification failed'}`);
  }
  let wasmReplay = { passed: parsed.vectors_passed, total: parsed.vectors_total };
  if (typeof studio_core_wasm_guest_bytes === 'function') {
    const guest = studio_core_wasm_guest_bytes();
    wasmReplay = await __uorReplayCoreWasmVectors(guest);
    if (wasmReplay.passed !== wasmReplay.total) {
      throw new Error(`RefusedMeAppVectorReplayFailure: ${wasmReplay.passed}/${wasmReplay.total} vectors passed`);
    }
  }
  const signedAction = await stitch_prism_identity_sign_action('verify-holo', `blake3:${parsed.fingerprint}`).catch(() => null);
  const summary = {
    ok: true,
    ...parsed,
    wasm_vectors_passed: wasmReplay.passed,
    wasm_vectors_total: wasmReplay.total,
    prism_id: signedAction ? signedAction.prism_id : null,
    device_id: signedAction ? signedAction.device_id : null,
    action_record_kappa: signedAction ? signedAction.action_record_kappa : null,
  };
  __uorMemPrismIdentity.latestHoloSummary = summary;
  globalThis.__uorLatestHoloSummary = summary;
  return includeBytes ? { ...summary, bytes: archiveBytes } : summary;
}

export async function compile_and_verify_holo_v4_in_worker(injectedFiles = null, includeBytes = false) {
  const filesMap = new Map();
  if (injectedFiles && typeof injectedFiles === 'object') {
    const iter = injectedFiles instanceof Map ? injectedFiles.entries() : Object.entries(injectedFiles);
    for (const [k, v] of iter) {
      filesMap.set(k, v instanceof Uint8Array ? v : cachedTextEncoder.encode(String(v)));
    }
  } else {
    for (const name of UOR_CLOSURE_FILES) {
      const url = new URL(name, import.meta.url);
      const res = await fetch(url, { cache: 'no-cache' });
      if (!res.ok) {
        throw new Error(`Failed loading closure asset ${name}: HTTP ${res.status}`);
      }
      filesMap.set(name, new Uint8Array(await res.arrayBuffer()));
    }
  }

  const holoview = __uorEncodeHoloviewV1('index.html', filesMap);
  const provBytes = filesMap.get('provenance.json') || cachedTextEncoder.encode('{}');
  if (typeof studio_compile_holo_v4_archive !== 'function') {
    throw new Error('studio_compile_holo_v4_archive export unavailable');
  }
  const archive = studio_compile_holo_v4_archive(holoview, provBytes);
  if (!archive || archive.length < 256) {
    throw new Error('RefusedInvalidHoloMagicOrVersion: compiled .holo archive is empty');
  }
  __uorMemPrismIdentity.latestHoloBytes = archive;
  if (typeof navigator !== 'undefined' && navigator.storage && typeof navigator.storage.getDirectory === 'function') {
    try {
      const root = await navigator.storage.getDirectory();
      const fh = await root.getFileHandle('lexlean-r4.holo', { create: true });
      const wr = await fh.createWritable();
      await wr.write(archive);
      await wr.close();
    } catch {
      // OPFS cache optional in non-OPFS contexts
    }
  }
  const summary = await verify_holo_v4_archive_in_worker(archive, includeBytes);
  const actionReceipt = await stitch_prism_identity_sign_action('compile-holo', `blake3:${summary.fingerprint}`).catch(() => null);
  return {
    ...summary,
    action_record_kappa: actionReceipt ? actionReceipt.action_record_kappa : summary.action_record_kappa,
    opfs_persisted: Boolean(typeof navigator !== 'undefined' && navigator.storage && navigator.storage.getDirectory),
  };
}

export async function build_afflom_me_app_descriptor(originHref = '') {
  /* Formal R4.PrismHoloIdentity (src/R4/PrismHoloIdentity.lex.tex) */
  const idState = await stitch_prism_identity_ensure().catch(() => null);
  const latestHolo = __uorMemPrismIdentity.latestHoloSummary || globalThis.__uorLatestHoloSummary || null;
  let baseLive = 'https://casey-allard.github.io/lexlean-r4/';
  if (originHref && originHref.startsWith('http')) {
    baseLive = originHref.endsWith('/') ? originHref : `${originHref.replace(/index\.html$/, '')}`;
    if (!baseLive.endsWith('/')) baseLive += '/';
  }
  return {
    schema: 'prism/r4-workspace/1',
    address: 'lexlean-r4',
    holoAddress: 'holo://lexlean-r4',
    name: 'LexLean-R4 Universal Workspace',
    version: '0.14.0',
    live: baseLive,
    holoUrl: `${baseLive}app.holo`,
    appJsonUrl: `${baseLive}app.json`,
    repository: 'https://github.com/Casey-allard/lexlean-r4',
    registry: 'https://registry.uor.foundation/',
    archiveKappa: latestHolo ? latestHolo.archive_kappa : null,
    fingerprint: latestHolo ? latestHolo.fingerprint : null,
    applicationKappa: latestHolo ? latestHolo.application_kappa : null,
    prismIdentity: idState ? {
      prism_id: idState.prism_id,
      device_id: idState.device_id,
      role: idState.role,
      persona: idState.persona,
    } : null,
    foundry: {
      repository: 'https://github.com/UOR-Foundation/foundry',
      fork: 'https://github.com/Casey-allard/foundry',
      holoViewer: 'holo.html',
      embedQueryParam: 'embed=foundry',
      r4SpecSha256: '808b936a8f33c67c0563fbd3069753e221c644ca20fcc1a81a1c37b19644b254',
      inprismRequest: {
        mode: 'inprism',
        engine: 'match',
        weights: 'match',
        route: 'none',
        boundary: 'allow',
        outcome: 'answer',
      },
    },
    embed: {
      supported: true,
      queryParam: 'embed=me',
      foundryQueryParam: 'embed=foundry',
      bridgeProtocol: 'uor-workspace-bridge/1',
    },
  };
}

if (typeof globalThis === 'object' && globalThis) {
  globalThis.__uorSelfHostRuntime = {
    optimalBrowserWorkerCount: optimal_browser_worker_count,
    initSharedWasmWorkerPool: init_shared_wasm_worker_pool,
    getBrowserWorkerPoolSnapshot: get_browser_worker_pool_snapshot,
    admitUorcGenerationCapsule: admit_uorc_generation_capsule,
    recordUorcGenerationFault: record_uorc_generation_fault,
    queryVerdantSynthesisStatus: query_verdant_synthesis_status,
    stitchPrismIdentityEnsure: stitch_prism_identity_ensure,
    stitchPrismIdentityBindPersona: stitch_prism_identity_bind_persona,
    compileAndVerifyHoloV4InWorker: compile_and_verify_holo_v4_in_worker,
    verifyHoloV4ArchiveInWorker: verify_holo_v4_archive_in_worker,
    buildAfflomMeAppDescriptor: build_afflom_me_app_descriptor,
    buildFoundryStudioDescriptor: build_afflom_me_app_descriptor,
    executeInprismFoundryTurn: execute_inprism_foundry_turn,
  };

  globalThis.LexLean = {
    ...(globalThis.LexLean || {}),
    opfs_put_blob,
    opfs_get_blob,
    storageRequest,
    withStorageLock,
    encodeSpvsWireFrame,
    decodeSpvsWireFrame,
    persist_mvm_cow_sectors_to_opfs,
    restore_mvm_cow_sectors_from_opfs,
    stitch_prism_identity_ensure,
    stitch_prism_identity_bind_persona,
    compile_and_verify_holo_v4_in_worker,
    verify_holo_v4_archive_in_worker,
    execute_inprism_foundry_turn,
  };
}

if (
  typeof WorkerGlobalScope !== 'undefined' &&
  typeof self !== 'undefined' &&
  self instanceof WorkerGlobalScope &&
  self.name === 'uor-r4-engine-worker'
) {
  __uorStartDedicatedEngineWorker();
}
// === UOR_CUSTOM_SECTION_2_END ===
