let wasm;

const cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); };

let cachedUint8ArrayMemory0 = null;

function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

let WASM_VECTOR_LEN = 0;

const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder('utf-8') : { encode: () => { throw Error('TextEncoder not available') } } );

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}
/**
 * Dispatch a Studio v2 command envelope (`1\tstudio\t<cmd>\t<payload>` or `<cmd>\t<payload>`).
 * @param {string} input
 * @returns {string}
 */
export function studio_dispatch(input) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(input, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_dispatch(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Process an Anthropic-compatible `/v1/messages` JSON request in WebAssembly.
 * @param {string} request_json
 * @returns {string}
 */
export function studio_v1_messages(request_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(request_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_v1_messages(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Process an OpenAI-compatible `/v1/chat/completions` JSON request in WebAssembly.
 * @param {string} request_json
 * @returns {string}
 */
export function studio_v1_chat_completions(request_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(request_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_v1_chat_completions(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Query or export OTel GLLM traces in WebAssembly.
 * @param {string} query_or_payload
 * @returns {string}
 */
export function studio_v1_traces(query_or_payload) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(query_or_payload, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_v1_traces(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}
/**
 * Export a content-addressed `UORSHRD1` binary shard for `e8_root` (`0..15`).
 * @param {number} e8_root
 * @returns {Uint8Array}
 */
export function studio_export_uorlut_shard(e8_root) {
    const ret = wasm.studio_export_uorlut_shard(e8_root);
    var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v1;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}
/**
 * Merge a content-addressed `UORSHRD1` binary shard into the resident `LutStackModel` and rebuild its Radix `/8` index.
 * @param {Uint8Array} shard_bytes
 * @returns {string}
 */
export function studio_merge_uorlut_shard(shard_bytes) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passArray8ToWasm0(shard_bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_merge_uorlut_shard(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_export_0.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}
/**
 * Consolidate integer LUT model weights via coordinate descent over dialogue samples.
 *
 * Accepts binary `UORLUT01` container bytes and a JSON configuration string.
 * Returns the updated `UORLUT01` container bytes, or an error string on failure.
 * @param {Uint8Array} weights_bytes
 * @param {string} config_json
 * @returns {Uint8Array}
 */
export function studio_consolidate_weights(weights_bytes, config_json) {
    const ptr0 = passArray8ToWasm0(weights_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(config_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.studio_consolidate_weights(ptr0, len0, ptr1, len1);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v3 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v3;
}

/**
 * WebAssembly entrypoint processing a versioned byte envelope.
 * @param {Uint8Array} input
 * @returns {Uint8Array}
 */
export function process_envelope(input) {
    const ptr0 = passArray8ToWasm0(input, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.process_envelope(ptr0, len0);
    var v2 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v2;
}

/**
 * WebAssembly entrypoint executing an application request with JsValue representation.
 * @param {Uint8Array} input_bytes
 * @returns {any}
 */
export function execute_application(input_bytes) {
    const ptr0 = passArray8ToWasm0(input_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.execute_application(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

let cachedInt32ArrayMemory0 = null;

function getInt32ArrayMemory0() {
    if (cachedInt32ArrayMemory0 === null || cachedInt32ArrayMemory0.byteLength === 0) {
        cachedInt32ArrayMemory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32ArrayMemory0;
}

function getArrayI32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}
/**
 * Export the trained `Q12` (`rows x 256`) projection slab
 * for zero-copy transfer into the WebGPU `q12_attention_project` compute shader.
 * @param {number} max_rows
 * @returns {Int32Array}
 */
export function studio_export_q12_projection_slab(max_rows) {
    const ret = wasm.studio_export_q12_projection_slab(max_rows);
    var v1 = getArrayI32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v1;
}

/**
 * Encode a prompt into a 256-lane `Q12` query vector (`scale = 4096`) for WebGPU dispatch.
 * @param {string} prompt
 * @returns {Int32Array}
 */
export function studio_encode_prompt_q12_query(prompt) {
    const ptr0 = passStringToWasm0(prompt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.studio_encode_prompt_q12_query(ptr0, len0);
    var v2 = getArrayI32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v2;
}

let cachedUint32ArrayMemory0 = null;

function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

function passArray32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getUint32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}
/**
 * Decode WebGPU `Q12` row projection scores back through the active model token decoder.
 * @param {string} prompt
 * @param {Int32Array} gpu_row_scores
 * @param {number} max_tokens
 * @returns {string}
 */
export function studio_decode_with_webgpu_q12_scores(prompt, gpu_row_scores, max_tokens) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(prompt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray32ToWasm0(gpu_row_scores, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.studio_decode_with_webgpu_q12_scores(ptr0, len0, ptr1, len1, max_tokens);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Hot-load a binary `UORLUT01` (`uor-r4.lut-stack/8`) weight artifact directly from JS/IndexedDB bytes.
 * @param {Uint8Array} bytes
 * @param {string} source_label
 * @returns {string}
 */
export function studio_load_uorlut_bytes(bytes, source_label) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(source_label, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.studio_load_uorlut_bytes(ptr0, len0, ptr1, len1);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Query the currently loaded `UORLUT01` (`uor-r4.lut-stack/8`) weight pack status as JSON.
 * @returns {string}
 */
export function studio_weight_pack_status() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_weight_pack_status();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Hot-load a verified `UORGLLM9` (`UOR-R4-GLLM v9`) container hydrated from OPFS shards (`R4-38`).
 * @param {Uint8Array} bytes
 * @param {string} source_label
 * @returns {string}
 */
export function studio_load_uorgllm_v9_bytes(bytes, source_label) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(source_label, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.studio_load_uorgllm_v9_bytes(ptr0, len0, ptr1, len1);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Query the active `UORGLLM9` (`UOR-R4-GLLM v9`) compositional runtime status as JSON (`R4-38`).
 * @returns {string}
 */
export function studio_gllm_v9_status() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_gllm_v9_status();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Execute `cargo test`, `cargo check`, `cargo clippy`, or `cargo run` over the active virtual workspace in WebAssembly (`R4-32`).
 * @param {string} subcommand
 * @param {string} filter
 * @returns {string}
 */
export function studio_wasm_eval(subcommand, filter) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(subcommand, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(filter, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.studio_wasm_eval(ptr0, len0, ptr1, len1);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Export dirty 64 KB `MVM-Wasm` Copy-on-Write block sectors for Worker OPFS persistence (`R4-37`).
 * @returns {Uint8Array}
 */
export function studio_export_mvm_cow_sectors() {
    const ret = wasm.studio_export_mvm_cow_sectors();
    var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v1;
}

/**
 * Hydrate 64 KB `MVM-Wasm` Copy-on-Write block sectors loaded from Worker OPFS (`R4-37`).
 * @param {Uint8Array} bytes
 * @returns {string}
 */
export function studio_import_mvm_cow_sectors(bytes) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_import_mvm_cow_sectors(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Return the 16-shard `(e8_root, clifford_action, icosian_root)` `.uorlut` catalog as JSON.
 * @returns {string}
 */
export function studio_shard_catalog() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_shard_catalog();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbindgen_init_externref_table = function() {
        const table = wasm.__wbindgen_export_0;
        const offset = table.grow(4);
        table.set(0, undefined);
        table.set(offset + 0, undefined);
        table.set(offset + 1, null);
        table.set(offset + 2, true);
        table.set(offset + 3, false);
        ;
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return ret;
    };

    return imports;
}

function __wbg_init_memory(imports, memory) {

}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedInt32ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;


    wasm.__wbindgen_start();
    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (typeof module !== 'undefined') {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();

    __wbg_init_memory(imports);

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return __wbg_finalize_init(instance, module);
}

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
            if (!manifest || manifest.format !== 'UORGLLM9' || !Array.isArray(manifest.shards)) {
              self.postMessage({ type: 'error', message: 'Invalid UORGLLM9 manifest' });
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
            status: 'OPFS SyncWorker UORGLLM9 (' + parsed.n_layers + 'L ' + parsed.d_model + 'D)',
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

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (typeof module_or_path !== 'undefined') {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    const useManagedCache = (typeof module_or_path === 'undefined');
    if (useManagedCache) {
        module_or_path = new URL('lexlean_r4_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (useManagedCache) {
        module_or_path = __uor_resolve_cached_wasm(module_or_path, false);
    } else if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    __wbg_init_memory(imports);

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

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

  constructor(options = {}) {
    this.relayUrl = options.relayWsUrl || options.relayUrl || DEFAULT_SPIRALCORE_RELAY_WS_URL;
    this.relayHttpUrl = options.relayHttpUrl || '';
    this.staticOrigin = options.staticOrigin || '';
    this.mcpPolicy = options.mcpPolicy || 'safe';
    this.webAccessEnabled = options.webAccessEnabled !== false;
    this.relayTimeoutMs = options.relayTimeoutMs || SPVS_RELAY_TIMEOUT_MS;
    this.lastTransportMode = 'standby';
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
      if (u.hostname.toLowerCase() === 'github.com') {
        const parts = u.pathname.split('/').filter(Boolean);
        if (parts.length >= 2) {
          return { owner: parts[0], repo: parts[1], url: `https://github.com/${parts[0]}/${parts[1]}` };
        }
      }
    } catch {
      // Not a GitHub URL
    }
    return null;
  }

  async handleGitBridge(req) {
    const rawArgs = String(req.args || (req.arguments && req.arguments.args) || '').trim();
    const argParts = rawArgs ? rawArgs.split(/\s+/) : [];
    const subcommand = String(req.subcommand || req.action || argParts[0] || 'clone').toLowerCase();
    const rawRepo = String(req.repo_url || req.repo || req.url || argParts[1] || '').trim();

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

    const ghHeaders = { Accept: 'application/vnd.github+json' };
    const authRes = this.applyHostSecretSubstitution(new URL('https://api.github.com'), ghHeaders);
    if (!authRes.ok) return authRes;
    const headers = authRes.headers;

    if (subcommand === 'clone' || subcommand === 'pull' || subcommand === 'fetch') {
      try {
        const repoResp = await fetch(`https://api.github.com/repos/${coords.owner}/${coords.repo}`, { headers });
        if (!repoResp.ok) {
          return {
            ok: false,
            status: repoResp.status,
            error_code: repoResp.status === 401 || repoResp.status === 404 ? 'GitRepoAccessDenied' : 'GitApiError',
            message: `GitHub API returned HTTP ${repoResp.status} for ${coords.owner}/${coords.repo}`,
          };
        }
        const repoMeta = await repoResp.json();
        const branch = String(req.branch || repoMeta.default_branch || 'main');
        const treeResp = await fetch(
          `https://api.github.com/repos/${coords.owner}/${coords.repo}/git/trees/${encodeURIComponent(branch)}?recursive=1`,
          { headers },
        );
        if (!treeResp.ok) {
          return {
            ok: false,
            status: treeResp.status,
            error_code: 'GitTreeFetchFailed',
            message: `Failed to fetch tree for ${coords.owner}/${coords.repo}@${branch} (HTTP ${treeResp.status})`,
          };
        }
        const treeJson = await treeResp.json();
        const blobs = (Array.isArray(treeJson.tree) ? treeJson.tree : []).filter(
          (entry) =>
            entry.type === 'blob' &&
            typeof entry.path === 'string' &&
            (entry.size ?? 0) <= 196608 &&
            !/\.(png|jpg|jpeg|gif|webp|ico|woff2?|ttf|wasm|bin|part\d+|xz|gz|zip|pdf)$/i.test(entry.path),
        );
        const selected = blobs.slice(0, 40);
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
                  const r = await fetch(rawUrl);
                  if (r.ok) {
                    return { path: entry.path, content: await r.text(), sha: entry.sha };
                  }
                }
                const blobResp = await fetch(
                  `https://api.github.com/repos/${coords.owner}/${coords.repo}/git/blobs/${entry.sha}`,
                  { headers },
                );
                if (!blobResp.ok) return null;
                const blobJson = await blobResp.json();
                if (blobJson.encoding === 'base64' && typeof blobJson.content === 'string') {
                  const bytes = __uorBase64ToBytes(blobJson.content);
                  return { path: entry.path, content: cachedTextDecoder.decode(bytes), sha: entry.sha };
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

        this.lastTransportMode = 'direct-https-github-rest';
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand,
          repo: `${coords.owner}/${coords.repo}`,
          repo_url: coords.url,
          branch,
          head_sha: treeJson.sha || '',
          files: fetchedFiles,
          file_count: fetchedFiles.length,
          data: {
            repo: `${coords.owner}/${coords.repo}`,
            branch,
            head_sha: treeJson.sha || '',
            files: fetchedFiles,
            file_count: fetchedFiles.length,
          },
          body_text: `Synchronized ${fetchedFiles.length} files from ${coords.owner}/${coords.repo}@${branch} (${String(treeJson.sha || '').slice(0, 12)})`,
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

    if (subcommand === 'push') {
      if (!this.hasSecret('GITHUB_TOKEN')) {
        return {
          ok: false,
          status: 401,
          error_code: 'GitAuthRequired',
          message: 'GitHub PAT not configured. Click GitHub in the header to store GITHUB_TOKEN in the host vault.',
        };
      }
      const filesToPush = Array.isArray(req.files) ? req.files : [];
      if (filesToPush.length === 0) {
        return {
          ok: false,
          status: 400,
          error_code: 'GitNothingToPush',
          message: 'No modified workspace files supplied for git push',
        };
      }
      const branch = String(req.branch || 'main');
      const commitMessage = String(req.message || req.commit_message || 'Update from UOR-R4 Connected Studio');
      try {
        const refResp = await fetch(
          `https://api.github.com/repos/${coords.owner}/${coords.repo}/git/ref/heads/${encodeURIComponent(branch)}`,
          { headers },
        );
        if (!refResp.ok) {
          return {
            ok: false,
            status: refResp.status,
            error_code: 'GitRefReadFailed',
            message: `Unable to read heads/${branch} on ${coords.owner}/${coords.repo} (HTTP ${refResp.status})`,
          };
        }
        const refJson = await refResp.json();
        const baseCommitSha = refJson.object && refJson.object.sha;
        const commitResp = await fetch(
          `https://api.github.com/repos/${coords.owner}/${coords.repo}/git/commits/${baseCommitSha}`,
          { headers },
        );
        const commitJson = await commitResp.json();
        const baseTreeSha = commitJson.tree && commitJson.tree.sha;

        const treeEntries = filesToPush.map((f) => ({
          path: String(f.path || '').replace(/^\/+/, ''),
          mode: '100644',
          type: 'blob',
          content: String(f.content ?? ''),
        }));
        const createTreeResp = await fetch(
          `https://api.github.com/repos/${coords.owner}/${coords.repo}/git/trees`,
          {
            method: 'POST',
            headers: { ...headers, 'Content-Type': 'application/json' },
            body: JSON.stringify({ base_tree: baseTreeSha, tree: treeEntries }),
          },
        );
        if (!createTreeResp.ok) {
          return {
            ok: false,
            status: createTreeResp.status,
            error_code: 'GitTreeCreateFailed',
            message: `GitHub tree creation failed (HTTP ${createTreeResp.status})`,
          };
        }
        const newTree = await createTreeResp.json();
        const newCommitResp = await fetch(
          `https://api.github.com/repos/${coords.owner}/${coords.repo}/git/commits`,
          {
            method: 'POST',
            headers: { ...headers, 'Content-Type': 'application/json' },
            body: JSON.stringify({
              message: commitMessage,
              tree: newTree.sha,
              parents: [baseCommitSha],
            }),
          },
        );
        if (!newCommitResp.ok) {
          return {
            ok: false,
            status: newCommitResp.status,
            error_code: 'GitCommitCreateFailed',
            message: `GitHub commit creation failed (HTTP ${newCommitResp.status})`,
          };
        }
        const createdCommit = await newCommitResp.json();
        const updateRefResp = await fetch(
          `https://api.github.com/repos/${coords.owner}/${coords.repo}/git/refs/heads/${encodeURIComponent(branch)}`,
          {
            method: 'PATCH',
            headers: { ...headers, 'Content-Type': 'application/json' },
            body: JSON.stringify({ sha: createdCommit.sha, force: false }),
          },
        );
        if (!updateRefResp.ok) {
          return {
            ok: false,
            status: updateRefResp.status,
            error_code: 'GitRefUpdateFailed',
            message: `GitHub ref fast-forward failed (HTTP ${updateRefResp.status})`,
          };
        }
        this.lastTransportMode = 'direct-https-github-rest';
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand: 'push',
          repo: `${coords.owner}/${coords.repo}`,
          branch,
          commit_sha: createdCommit.sha,
          pushed_files: treeEntries.length,
          data: {
            repo: `${coords.owner}/${coords.repo}`,
            branch,
            commit_sha: createdCommit.sha,
            pushed_files: treeEntries.length,
          },
          body_text: `Pushed ${treeEntries.length} file(s) to ${coords.owner}/${coords.repo}@${branch} -> ${String(createdCommit.sha || '').slice(0, 12)}`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return {
          ok: false,
          status: 502,
          error_code: 'GitPushNetworkError',
          message: err && err.message ? err.message : String(err),
        };
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
      try {
        const r = await fetch(idxUrl, { cache: 'no-cache' });
        if (r.ok) {
          pkgIndex = await r.json();
          basePkgUrl = idxUrl.slice(0, idxUrl.lastIndexOf('/') + 1);
          break;
        }
      } catch {
        // Try next candidate
      }
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
        if (parsed && parsed.format === 'UORGLLM9' && Array.isArray(parsed.shards)) {
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

  const expectedSha = String(manifest.container_sha256 || '');
  if (!forceReload && hasOpfs && expectedSha) {
    try {
      const root = await navigator.storage.getDirectory();
      const mfh = await root.getFileHandle(OPFS_V10_META_FILE, { create: false });
      const meta = JSON.parse(await (await mfh.getFile()).text());
      if (meta && meta.sha256 === expectedSha) {
        const wfh = await root.getFileHandle(OPFS_V10_WEIGHT_FILE, { create: false });
        const bytes = new Uint8Array(await (await wfh.getFile()).arrayBuffer());
        const loaded = JSON.parse(studio_load_uorgllm_v9_bytes(bytes, 'opfs_cached_v10_16L_256D'));
        if (loaded && loaded.status === 'ok') {
          globalThis.__uorGllmV9Meta = {
            status: `OPFS UORGLLM v10 (${loaded.n_layers}L ${loaded.d_model}D)`,
            sha256: expectedSha,
            mode: 'opfs-v10-cached',
          };
          return loaded;
        }
      }
    } catch {
      // Proceed to stream shards
    }
  }

  const totalBytes = manifest.total_bytes >>> 0;
  const fullBuf = new Uint8Array(totalBytes);
  for (let i = 0; i < manifest.shards.length; i++) {
    const s = manifest.shards[i];
    const sResp = await fetch(baseUrl + s.file);
    if (!sResp.ok) throw new Error(`Shard HTTP ${sResp.status} on ${s.file}`);
    const ab = await sResp.arrayBuffer();
    if (ab.byteLength !== s.byte_length) throw new Error(`Shard length mismatch on ${s.file}`);
    const shardSha = await __uorSha256HexBytes(new Uint8Array(ab));
    if (shardSha !== s.sha256) throw new Error(`Shard SHA-256 mismatch on ${s.file}`);
    fullBuf.set(new Uint8Array(ab), s.byte_offset);
    if (typeof onProgress === 'function') {
      onProgress({
        index: i + 1,
        total: manifest.shards.length,
        bytes: s.byte_offset + s.byte_length,
        totalBytes,
      });
    }
  }

  const fullSha = await __uorSha256HexBytes(fullBuf);
  if (expectedSha && fullSha !== expectedSha) {
    throw new Error('UORGLLM v10 container SHA-256 mismatch');
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
      const metaBytes = cachedTextEncoder.encode(JSON.stringify({ sha256: fullSha, totalBytes }));
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

  const parsed = JSON.parse(studio_load_uorgllm_v9_bytes(fullBuf, 'opfs_streamed_v10_16L_256D'));
  if (parsed && parsed.status === 'ok') {
    globalThis.__uorGllmV9Meta = {
      status: `OPFS UORGLLM v10 (${parsed.n_layers}L ${parsed.d_model}D)`,
      sha256: fullSha,
      mode: 'opfs-v10-streamed',
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
  const earlyMessages = [];

  const postEvent = (event, data = {}) => {
    self.postMessage({ event, ...data });
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

      const bridgeResp = await hostBridge.execute(bridgeReq);
      if (bridgeResp && bridgeResp.op === 'git' && Array.isArray(bridgeResp.files) && bridgeResp.files.length > 0) {
        applyClonedFilesToWorkspace(bridgeResp.files);
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
        const res = await dispatchWithConnectedBridge(String(payload || ''));
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
        if (reqPayload.op === 'git' && reqPayload.subcommand === 'push' && !Array.isArray(reqPayload.files)) {
          const snap = __uorCollectWorkspaceSnapshot();
          reqPayload = { ...reqPayload, files: snap.files };
        }
        const res = await hostBridge.execute(reqPayload);
        if (res && res.op === 'git' && Array.isArray(res.files) && res.files.length > 0) {
          applyClonedFilesToWorkspace(res.files);
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
        const v10 = await stream_v10_sharded_weights_to_opfs((prog) => {
          postEvent('boot_progress', { stage: 'v10_shards', ...prog });
        }, true).catch(() => null);
        const lut = await sync_cached_weights((prog) => {
          postEvent('boot_progress', { stage: 'uorlut_sync', ...prog });
        });
        self.postMessage({ id, ok: true, result: { v10, lut } });
        return;
      }
      if (op === 'snapshot') {
        const state = JSON.parse(studio_dispatch('1\tstudio\tstate\t'));
        const workspace = __uorCollectWorkspaceSnapshot();
        const pack = JSON.parse(studio_weight_pack_status());
        const gllm = typeof studio_gllm_v9_status === 'function' ? JSON.parse(studio_gllm_v9_status()) : null;
        self.postMessage({ id, ok: true, result: { state, workspace, pack, gllm } });
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
      postEvent('ready', {
        snapshot: { state, workspace, pack, gllm },
        weightCacheMeta: globalThis.__uorWeightCacheMeta || null,
        gllmMeta: globalThis.__uorGllmV9Meta || null,
      });

      while (earlyMessages.length > 0) {
        const queued = earlyMessages.shift();
        await handleWorkerMessage(queued);
      }

      // Stream v10 sharded weights into OPFS in background without delaying early turns
      stream_v10_sharded_weights_to_opfs((prog) => {
        postEvent('boot_progress', { stage: 'v10_shards', ...prog });
      }, false)
        .then((v10Info) => {
          if (v10Info) {
            postEvent('weights_upgraded', {
              gllm: v10Info,
              gllmMeta: globalThis.__uorGllmV9Meta || null,
            });
          }
        })
        .catch(() => {});
    })().catch((err) => {
      postEvent('boot_error', {
        message: err && err.message ? err.message : String(err),
      });
    });
    return bootPromise;
  };

  startBoot();
}

if (
  typeof WorkerGlobalScope !== 'undefined' &&
  typeof self !== 'undefined' &&
  self instanceof WorkerGlobalScope &&
  self.name === 'uor-r4-engine-worker'
) {
  __uorStartDedicatedEngineWorker();
}

export { initSync };
export default __wbg_init;
