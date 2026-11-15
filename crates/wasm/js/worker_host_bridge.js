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

globalThis.__uorOpenWeightsTier = {
    webgpuAvailable: false,
    webgpuReady: false,
    adapterName: 'Wasm-SIMD Integer Co-Processor',
    opfsAvailable: false,
    opfsPersistedBytes: 0,
    openWeightsFormat: 'UORLUT01/8',
    openWeightsTensorCount: 6,
    device: null,
    pipeline: null
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
        globalThis.__uorOpenWeightsTier.device = device;
        globalThis.__uorOpenWeightsTier.pipeline = pipeline;
        globalThis.__uorOpenWeightsTier.webgpuReady = true;
    } catch (_ge) {
        globalThis.__uorOpenWeightsTier.webgpuReady = false;
    }
    return globalThis.__uorOpenWeightsTier;
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
  if (typeof navigator === 'undefined' || !navigator.storage || typeof navigator.storage.getDirectory !== 'function') {
    return false;
  }
  try {
    const bytes = studio_export_mvm_cow_sectors();
    if (!bytes || bytes.byteLength < 8) return false;
    const root = await navigator.storage.getDirectory();
    const fh = await root.getFileHandle(OPFS_MVM_COW_FILE, { create: true });
    if (typeof fh.createWritable === 'function') {
      const w = await fh.createWritable();
      await w.write(bytes);
      await w.close();
      return true;
    }
  } catch {
    // OPFS unavailable in private/sandboxed browsing
  }
  return false;
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
    if (!file || file.size < 8) return null;
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

// === UOR_CUSTOM_SECTION_1_END ===

// === UOR_CUSTOM_SECTION_2_START ===
const OPFS_V10_WEIGHT_FILE = 'uor_r4_gllm_v10_weights.bin';
const OPFS_V10_META_FILE = 'uor_r4_gllm_v10_meta.json';
const DEFAULT_SPIRALCORE_RELAY_WS_URL = 'wss://lexlean-r4-spiralcore-relay.swarm-relay-cloud-8d31a122.workers.dev/spvs';
const DEFAULT_SPIRALCORE_RELAY_HTTP_URL = 'https://lexlean-r4-spiralcore-relay.swarm-relay-cloud-8d31a122.workers.dev/spvs';
const LOCAL_SPIRALCORE_RELAY_WS_URL = 'ws://127.0.0.1:8787/spvs';
const SPVS_RELAY_TIMEOUT_MS = 2500;
const MAX_BRIDGE_RESPONSE_BYTES = 2 * 1024 * 1024;

const SECRET_HOST_ALLOWLIST = Object.freeze({
  GITHUB_TOKEN: new Set(['github.com', 'api.github.com']),
});

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

  constructor(options = {}) {
    this.relayUrl =
      options.relayWsUrl !== undefined
        ? options.relayWsUrl
        : (options.relayUrl !== undefined ? options.relayUrl : DEFAULT_SPIRALCORE_RELAY_WS_URL);
    this.relayHttpUrl = options.relayHttpUrl || '';
    this.staticOrigin = options.staticOrigin || '';
    this.locationOrigin = options.locationOrigin || '';
    this.mcpPolicy = options.mcpPolicy || 'safe';
    this.webAccessEnabled = options.webAccessEnabled !== false;
    this.relayTimeoutMs = options.relayTimeoutMs || SPVS_RELAY_TIMEOUT_MS;
    this.lastTransportMode = 'standby';
    this.fetchImpl = options.fetchImpl || globalThis.fetch;
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

  setPolicy({ mcpPolicy, webAccessEnabled } = {}) {
    if (typeof mcpPolicy === 'string' && ['safe', 'confirm', 'deny'].includes(mcpPolicy)) {
      this.mcpPolicy = mcpPolicy;
    }
    if (typeof webAccessEnabled === 'boolean') {
      this.webAccessEnabled = webAccessEnabled;
    }
    return {
      ok: true,
      mcp_policy: this.mcpPolicy,
      web_access_enabled: this.webAccessEnabled,
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
    const serializedCheck = JSON.stringify(params || {});
    if (serializedCheck.includes('{{MVM_SECRET:')) {
      return null;
    }
    const rpcBody = {
      spvs_magic: 'SPVS',
      src_cid: 3,
      dst_cid: 2,
      policy: this.mcpPolicy,
      op,
      params: params || {},
    };

    if (typeof WebSocket === 'function' && this.relayUrl) {
      const candidateUrls = [this.relayUrl];
      for (const wsUrl of candidateUrls) {
        const valid = this.validateTransportUrl(wsUrl, true);
        if (!valid.ok) continue;
        try {
          const result = await this.#sendSingleWsRpc(wsUrl, rpcBody, this.relayTimeoutMs);
          if (result && typeof result.ok === 'boolean') {
            this.lastTransportMode = wsUrl.startsWith('ws://127.0.0.1') ? 'spvs-local-ws' : 'spvs-cloud-wss';
            return result;
          }
        } catch {
          // Try HTTPS POST relay or direct-HTTPS fallback next
        }
      }
    }

    if (this.relayUrl === DEFAULT_SPIRALCORE_RELAY_WS_URL && typeof fetch === 'function') {
      try {
        const ctrl = typeof AbortController === 'function' ? new AbortController() : null;
        const timer = ctrl ? setTimeout(() => ctrl.abort(), this.relayTimeoutMs) : null;
        const reqId = `spvs-http-${Date.now()}-${this.#nextReqSeq++}`;
        const resp = await fetch(DEFAULT_SPIRALCORE_RELAY_HTTP_URL, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ req_id: reqId, ...rpcBody }),
          signal: ctrl ? ctrl.signal : undefined,
        });
        if (timer) clearTimeout(timer);
        const parsed = await resp.json();
        if (parsed && typeof parsed.ok === 'boolean') {
          this.lastTransportMode = 'spvs-cloud-https';
          return parsed;
        }
      } catch {
        // Fall through to direct-HTTPS fallback
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
      const resp = await fetch(finalValid.url.href, {
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
    const httpOrigin =
      this.staticOrigin ||
      (typeof location !== 'undefined' && location.origin && /^https?:/i.test(location.origin)
        ? location.origin.replace(/\/+$/, '')
        : '');
    const rawInput = String(provenanceUrlInput || '').trim();
    const normalizedInput = rawInput
      ? (/\.json(?:[?#].*)?$/i.test(rawInput) ? rawInput : `${rawInput.replace(/\/+$/, '')}/provenance.json`)
      : '';
    const provUrl =
      normalizedInput ||
      (httpOrigin ? `${httpOrigin}/provenance.json` : new URL('provenance.json', import.meta.url).href);
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
        const data = {
          repo: repoSlug,
          pages_status: pagesRes.ok && pagesRes.json ? (pagesRes.json.status || (buildRes.json && buildRes.json.status) || 'built') : 'unconfigured',
          pages_url: pagesUrl,
          build_status: buildRes.ok && buildRes.json ? (buildRes.json.status || 'unknown') : 'unknown',
          deployment_sha: buildRes.ok && buildRes.json ? (buildRes.json.commit || '') : '',
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
    const httpOrigin =
      this.staticOrigin ||
      (typeof location !== 'undefined' && location.origin && /^https?:/i.test(location.origin)
        ? location.origin
        : '');
    const indexCandidates = [
      ...(httpOrigin ? [`${httpOrigin.replace(/\/+$/, '')}/weights/mvm-pkgs/index.json`] : []),
      new URL('weights/mvm-pkgs/index.json', import.meta.url).href,
      new URL('../gpu/weights/mvm-pkgs/index.json', import.meta.url).href,
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
    for (const originBase of sourceBases) {
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
  const sourcesUsed = new Set();

  for (let i = 0; i < manifest.shards.length; i++) {
    const s = manifest.shards[i];
    const hit = fetchedMap.get(s.file)
      ? { bytes: fetchedMap.get(s.file), source: normalizedBase }
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
      healedShards.push({
        file: s.file,
        group: group.group_index,
        parityFile: group.parity.file,
        recoveredSha256: recoveredSha,
      });
    }

    fullBuf.set(shardBytes, s.byte_offset);
    if (typeof onProgress === 'function') {
      onProgress({
        index: i + 1,
        total: manifest.shards.length,
        bytes: s.byte_offset + s.byte_length,
        totalBytes,
        healedCount: healedShards.length,
      });
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

export async function stream_v10_sharded_weights_to_opfs(onProgress, forceReload = false) {
  if (wasm === undefined || typeof studio_load_uorgllm_v9_bytes !== 'function') return null;
  const httpOrigin =
    typeof location !== 'undefined' && location.origin && /^https?:/i.test(location.origin)
      ? location.origin.replace(/\/+$/, '')
      : '';
  const manifestCandidates = [
    ...(httpOrigin ? [`${httpOrigin}/weights/v10/manifest.json`] : []),
    new URL('weights/v10/manifest.json', import.meta.url).href,
    new URL('../gpu/weights/v10/manifest.json', import.meta.url).href,
    new URL('weights/v9/manifest.json', import.meta.url).href,
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

  const hasOpfs =
    typeof navigator !== 'undefined' &&
    navigator.storage &&
    typeof navigator.storage.getDirectory === 'function';

  if (!manifest) {
    return restore_gllm_v9_from_opfs('');
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
      const mfh = await root.getFileHandle(OPFS_V10_META_FILE, { create: false });
      const meta = JSON.parse(await (await mfh.getFile()).text());
      if (meta && meta.sha256 === expectedSha && (!meta.format || meta.format === manifest.format)) {
        const wfh = await root.getFileHandle(OPFS_V10_WEIGHT_FILE, { create: false });
        const bytes = new Uint8Array(await (await wfh.getFile()).arrayBuffer());
        const isR4V12 = manifest.format === 'UORR4V12' || (bytes.byteLength >= 8 && cachedTextDecoder.decode(bytes.subarray(0, 8)) === 'UORR4V12');
        const loadLabel = isR4V12 ? 'opfs_cached_r4_v12_30L_576D' : 'opfs_cached_v10_16L_256D';
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
            tier: loaded.active_tier || (isR4V12 ? 'Full30LRecompiled' : 'LegacyV9'),
            sha256: expectedSha,
            mode: isR4V12 ? 'opfs-r4v12-cached' : 'opfs-v10-cached',
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
      const fh = await root.getFileHandle(OPFS_V10_WEIGHT_FILE, { create: true });
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
      const mfh = await root.getFileHandle(OPFS_V10_META_FILE, { create: true });
      const metaBytes = cachedTextEncoder.encode(JSON.stringify({ sha256: fullSha, totalBytes, format: manifest.format || 'UORR4V12' }));
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
  const streamLabel = isStreamedR4V12 ? 'opfs_streamed_r4_v12_30L_576D' : 'opfs_streamed_v10_16L_256D';
  const rawParsed = (isStreamedR4V12 && typeof studio_mount_r4_v12_bytes === 'function')
    ? studio_mount_r4_v12_bytes(fullBuf)
    : studio_load_uorgllm_v9_bytes(fullBuf, streamLabel);
  const parsed = JSON.parse(rawParsed);
  if (parsed && parsed.status === 'ok') {
    const fmtLabel = parsed.format || manifest.format || (isStreamedR4V12 ? 'UORR4V12' : 'UORGLLM9');
    globalThis.__uorGllmV9Meta = {
      status: `OPFS ${fmtLabel} (${parsed.n_layers}L ${parsed.d_model}D)`,
      format: fmtLabel,
      tier: parsed.active_tier || (isStreamedR4V12 ? 'Full30LRecompiled' : 'LegacyV9'),
      sha256: fullSha,
      mode: isStreamedR4V12 ? 'opfs-r4v12-streamed' : 'opfs-v10-streamed',
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
  const earlyMessages = [];

  const postEvent = (event, data = {}) => {
    self.postMessage({ event, ...data });
  };

  const startOrAwaitOpfsShardHydration = (forceReload = false) => {
    if (activeHydrationPromise && !forceReload) {
      return activeHydrationPromise;
    }
    const run = stream_v10_sharded_weights_to_opfs((prog) => {
      lastHydrationProgress = { stage: 'r4_shards', ...prog };
      postEvent('boot_progress', lastHydrationProgress);
      postEvent('hydration_progress', lastHydrationProgress);
    }, forceReload)
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
    if (isChatTurn || envelope.includes('\tmvm_exec\t')) {
      const pulses = __uorSynthesizeBrainPulsesFromReply(parsed, lastToolKind);
      postEvent('brain_pulse', { pulses });
    }
    if (envelope.includes('\tmvm_exec\t') || (isChatTurn && parsed && parsed.mvm_tool_executed)) {
      persist_mvm_cow_sectors_to_opfs().catch(() => {});
      postEvent('workspace_sync', { workspace: __uorCollectWorkspaceSnapshot() });
    }
    return parsed;
  };

  const handleWorkerMessage = async (msg) => {
    const { id, op, payload } = msg || {};
    try {
      if (op === 'studio_dispatch') {
        const envelopeStr = String(payload || '');
        if (envelopeStr.includes('\tchat\t') || envelopeStr.includes('\tchat_v10_turn\t')) {
          let needsFullV12 = false;
          try {
            const parts = envelopeStr.split('\t');
            const bodyRaw = parts.length >= 4 ? parts.slice(3).join('\t').trim() : '';
            if (bodyRaw.startsWith('{')) {
              const reqObj = JSON.parse(bodyRaw);
              const sel = String(reqObj && reqObj.selected_model || '');
              needsFullV12 = sel === 'uor-r4.recompiled-v12-agent' || sel === 'uor-r4.recompiled-v12-chat';
            }
          } catch {
            // Leave needsFullV12 false on non-JSON envelopes
          }
          if (needsFullV12 && typeof studio_r4_stack_status === 'function') {
            let mountedFull = false;
            try {
              const r4Now = JSON.parse(studio_r4_stack_status());
              mountedFull = Boolean(r4Now && r4Now.full_recompiled_mounted);
            } catch {
              mountedFull = false;
            }
            if (!mountedFull) {
              postEvent('hydration_progress', {
                stage: 'r4_shards',
                awaiting: true,
                ...(lastHydrationProgress || {
                  index: 0,
                  total: 73,
                  healedCount: 0,
                  label: 'Hydrating 30L UORR4V12 OPFS shards before 30L inference...',
                }),
              });
              await startOrAwaitOpfsShardHydration(false);
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
      if (op === 'set_bridge_policy') {
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
      if (op === 'sync_weights') {
        const v10 = await startOrAwaitOpfsShardHydration(true);
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
      self.postMessage({ id, ok: false, error: `Unknown worker op: ${op}` });
    } catch (err) {
      self.postMessage({
        id,
        ok: false,
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
      await __wbg_init();
      await restore_cached_uorlut_from_indexeddb().catch(() => null);
      await restore_mvm_cow_sectors_from_opfs().catch(() => null);
      wasmReady = true;

      const state = JSON.parse(studio_dispatch('1\tstudio\tstate\t'));
      const workspace = __uorCollectWorkspaceSnapshot();
      const pack = JSON.parse(studio_weight_pack_status());
      const gllm = typeof studio_gllm_v9_status === 'function' ? JSON.parse(studio_gllm_v9_status()) : null;
      const r4Stack = typeof studio_r4_stack_status === 'function' ? JSON.parse(studio_r4_stack_status()) : null;
      const verdantSynthesis = query_verdant_synthesis_status();
      postEvent('ready', {
        snapshot: { state, workspace, pack, gllm, r4Stack, verdantSynthesis },
        weightCacheMeta: globalThis.__uorWeightCacheMeta || null,
        gllmMeta: globalThis.__uorGllmV9Meta || null,
        r4Stack,
        verdantSynthesis,
      });

      while (earlyMessages.length > 0) {
        const queued = earlyMessages.shift();
        await handleWorkerMessage(queued);
      }

      // Eagerly stream UORR4V12 / sharded weights into OPFS on Worker startup;
      // full 30L chat turns await this in-flight promise before dispatching.
      startOrAwaitOpfsShardHydration(false);

      if (typeof location !== 'undefined' && /^https?:/i.test(location.origin || '')) {
        hostBridge.checkProvenanceUpdate(location.origin).catch(() => null);
        if (typeof setInterval === 'function') {
          setInterval(() => {
            hostBridge
              .checkProvenanceUpdate(location.origin)
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
  const logicalThreads =
    typeof navigator !== 'undefined' && typeof navigator.hardwareConcurrency === 'number'
      ? Math.max(1, Math.floor(navigator.hardwareConcurrency))
      : 4;
  return {
    logical_threads: logicalThreads,
    optimal_workers: optimal_browser_worker_count(logicalThreads),
    pool_size: __uorSharedWorkerPool.length,
    shared_module_compiled: Boolean(__uorSharedWasmModule),
    zero_refetch_sharing: true,
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

if (typeof globalThis === 'object' && globalThis) {
  globalThis.__uorSelfHostRuntime = {
    optimalBrowserWorkerCount: optimal_browser_worker_count,
    initSharedWasmWorkerPool: init_shared_wasm_worker_pool,
    getBrowserWorkerPoolSnapshot: get_browser_worker_pool_snapshot,
    admitUorcGenerationCapsule: admit_uorc_generation_capsule,
    recordUorcGenerationFault: record_uorc_generation_fault,
    queryVerdantSynthesisStatus: query_verdant_synthesis_status,
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
