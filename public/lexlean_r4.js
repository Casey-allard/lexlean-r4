let wasm;

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_export_2.set(idx, obj);
    return idx;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

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

function isLikeNone(x) {
    return x === undefined || x === null;
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
 * Begin in-stream compilation of a HuggingFace Safetensors repo into `UORR4V13` (`R4-51`).
 * @param {string} repo_id
 * @param {string} config_json
 * @param {string} tokenizer_json
 * @returns {string}
 */
export function studio_hf_repo_begin(repo_id, config_json, tokenizer_json) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(repo_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(config_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(tokenizer_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.studio_hf_repo_begin(ptr0, len0, ptr1, len1, ptr2, len2);
        deferred4_0 = ret[0];
        deferred4_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}
/**
 * Stream one HTTP `206` Safetensors tensor slice into the active `HfRepoHoloStreamCompiler` (`R4-51`).
 * @param {string} name
 * @param {string} dtype
 * @param {string} shape_json
 * @param {Uint8Array} raw_bytes
 * @returns {string}
 */
export function studio_hf_repo_push_tensor(name, dtype, shape_json, raw_bytes) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(dtype, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(shape_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ptr3 = passArray8ToWasm0(raw_bytes, wasm.__wbindgen_malloc);
        const len3 = WASM_VECTOR_LEN;
        const ret = wasm.studio_hf_repo_push_tensor(ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
        deferred5_0 = ret[0];
        deferred5_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}

/**
 * Finalize streamed Safetensors compilation and mount `uor-r4.hf-repo-holo` (`R4-51`).
 * @returns {string}
 */
export function studio_hf_repo_finalize() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_hf_repo_finalize();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
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
    const value = wasm.__wbindgen_export_2.get(idx);
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
 * Query the active `UORR4V12` geometric vector stack status as JSON (`R4-46`).
 * @returns {string}
 */
export function studio_r4_stack_status() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_r4_stack_status();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Admit or refuse a `.uorc` Verdant Synthesis generation capsule (`R4-46`).
 * @param {string} capsule_json
 * @returns {string}
 */
export function studio_admit_generation_capsule(capsule_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(capsule_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_admit_generation_capsule(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Record a runtime generation fault strike and trigger 3-strike parent-head rollback when applicable (`R4-46`).
 * @param {string} fault_reason
 * @returns {string}
 */
export function studio_record_generation_fault(fault_reason) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(fault_reason, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_record_generation_fault(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Return structured 4-facet Verdant Synthesis and OPFS watchdog status JSON (`R4-45`, `R4-46`).
 * @returns {string}
 */
export function studio_verdant_synthesis_status() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_verdant_synthesis_status();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Formal R4.CirisLedger Wasm export admitting a causal row and updating the fixed-point cut (`R5`).
 * @param {string} row_json
 * @returns {string}
 */
export function studio_ciris_admit_row(row_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(row_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_ciris_admit_row(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Formal R4.CirisLedger Wasm export querying current causal ledger cut status.
 * @returns {string}
 */
export function studio_ciris_status() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_ciris_status();
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
 * Parse and validate a `manifest.stitch.json` (`stitch/xor-2-of-3/1`) document.
 * @param {string} manifest_json
 * @returns {string}
 */
export function studio_stitch_verify_manifest(manifest_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(manifest_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_stitch_verify_manifest(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Encode an arbitrary payload into 3 Stitch `xor-2-of-3` custody fragments and return JSON metadata.
 * @param {Uint8Array} payload
 * @returns {string}
 */
export function studio_stitch_split_2of3(payload) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passArray8ToWasm0(payload, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_stitch_split_2of3(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Heal and recover a single requested shard (`left` if `recover_left == true`, else `right`)
 * from a `StitchParityGroup` JSON descriptor and up to 3 candidate byte slices using 2-of-3 XOR
 * parity + strict SHA-256 verification.
 * @param {string} group_json
 * @param {boolean} recover_left
 * @param {boolean} has_left
 * @param {Uint8Array} left_bytes
 * @param {boolean} has_right
 * @param {Uint8Array} right_bytes
 * @param {boolean} has_parity
 * @param {Uint8Array} parity_bytes
 * @returns {Uint8Array}
 */
export function studio_stitch_recover_pair_auto(group_json, recover_left, has_left, left_bytes, has_right, right_bytes, has_parity, parity_bytes) {
    const ptr0 = passStringToWasm0(group_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(left_bytes, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passArray8ToWasm0(right_bytes, wasm.__wbindgen_malloc);
    const len2 = WASM_VECTOR_LEN;
    const ptr3 = passArray8ToWasm0(parity_bytes, wasm.__wbindgen_malloc);
    const len3 = WASM_VECTOR_LEN;
    const ret = wasm.studio_stitch_recover_pair_auto(ptr0, len0, recover_left, has_left, ptr1, len1, has_right, ptr2, len2, has_parity, ptr3, len3);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v5 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v5;
}

/**
 * Recover an original payload from any 2 of 3 raw `afflom/stitch` `xor-2-of-3` fragments and verify
 * its SHA-256 digest when `expected_object_sha256` is non-empty.
 * @param {boolean} has_frag0
 * @param {Uint8Array} frag0
 * @param {boolean} has_frag1
 * @param {Uint8Array} frag1
 * @param {boolean} has_frag2
 * @param {Uint8Array} frag2
 * @param {number} original_length
 * @param {string} expected_object_sha256
 * @returns {Uint8Array}
 */
export function studio_stitch_recover_2of3_object(has_frag0, frag0, has_frag1, frag1, has_frag2, frag2, original_length, expected_object_sha256) {
    const ptr0 = passArray8ToWasm0(frag0, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(frag1, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passArray8ToWasm0(frag2, wasm.__wbindgen_malloc);
    const len2 = WASM_VECTOR_LEN;
    const ptr3 = passStringToWasm0(expected_object_sha256, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len3 = WASM_VECTOR_LEN;
    const ret = wasm.studio_stitch_recover_2of3_object(has_frag0, ptr0, len0, has_frag1, ptr1, len1, has_frag2, ptr2, len2, original_length, ptr3, len3);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v5 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v5;
}

/**
 * Formal `R4.PrismHoloIdentity` query active `afflom/stitch` Prism Identity ledger state (`R4-54`).
 * @returns {string}
 */
export function studio_prism_identity_status() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_prism_identity_status();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Formal `R4.PrismHoloIdentity` admit a `Stitch.PrismId` JSON record into the session ledger (`R4-54`).
 * @param {string} record_json
 * @returns {string}
 */
export function studio_prism_identity_admit_record(record_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(record_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_prism_identity_admit_record(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Formal `R4.PrismHoloIdentity` return the import-free `hologram:guest/core-wasm@1` wasm32 module (`R4-54`).
 * @returns {Uint8Array}
 */
export function studio_core_wasm_guest_bytes() {
    const ret = wasm.studio_core_wasm_guest_bytes();
    var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v1;
}

/**
 * Formal `R4.PrismHoloIdentity` compile a big-endian `HOLOVIEW` v1 closure and provenance JSON into a Hologram v4 `.holo` archive (`R4-54`).
 * @param {Uint8Array} holoview_bytes
 * @param {Uint8Array} provenance_bytes
 * @returns {Uint8Array}
 */
export function studio_compile_holo_v4_archive(holoview_bytes, provenance_bytes) {
    const ptr0 = passArray8ToWasm0(holoview_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(provenance_bytes, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.studio_compile_holo_v4_archive(ptr0, len0, ptr1, len1);
    var v3 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v3;
}

/**
 * Formal `R4.PrismHoloIdentity` verify a Hologram v4 `.holo` archive and return structured JSON verdict (`R4-54`).
 * @param {Uint8Array} archive_bytes
 * @returns {string}
 */
export function studio_verify_holo_v4_archive(archive_bytes) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passArray8ToWasm0(archive_bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_verify_holo_v4_archive(ptr0, len0);
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

/**
 * Formal R4.HoloGeometricRuntime adjust shortlist K and Starling Flock window dynamically (`R4-51`, `R4-55`).
 * @param {number} shortlist_k
 * @param {number} flock_window
 * @returns {string}
 */
export function studio_set_holo_budgets(shortlist_k, flock_window) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_set_holo_budgets(shortlist_k, flock_window);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Formal R4.HoloGeometricRuntime query OPFS stationary layout + MVM CoW dirty sectors + RHT vocab clusters.
 * @param {string} model_id
 * @returns {string}
 */
export function studio_holo_storage_geometry_json(model_id) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(model_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_holo_storage_geometry_json(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Formal R4.OntologicalTelos execute one verified parametric coordinate-descent self-improvement tick (`R4-55`).
 * @param {string} payload
 * @returns {string}
 */
export function studio_run_telos_self_improvement_tick(payload) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(payload, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_run_telos_self_improvement_tick(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Stream one SHA-256 verified OPFS `.holo` piece into the active streaming builder (`R4-50`).
 * @param {Uint8Array} piece
 * @returns {string}
 */
export function studio_push_holo_v13_piece(piece) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passArray8ToWasm0(piece, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_push_holo_v13_piece(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Finalize the streamed `UORR4V13` container mount and verify end-to-end SHA-256 (`R4-51`).
 * @returns {string}
 */
export function studio_finalize_holo_v13_mount() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_finalize_holo_v13_mount();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Query mounted `UORR4V13` Holo runtimes and `NativeQ31` status as JSON (`R4-51`, `R4-52`).
 * @returns {string}
 */
export function studio_holo_runtime_status() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_holo_runtime_status();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Query Track 2 `D85` Native `Q31` SpiralCore/FBS status as JSON (`R4-52`).
 * @returns {string}
 */
export function studio_native_q31_status() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_native_q31_status();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Run one online browser `q31_hamilton_train_step` update on `D85` Native `Q31` (`R4-52`).
 * @param {string} text
 * @returns {string}
 */
export function studio_native_q31_train_step(text) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_native_q31_train_step(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Parse & validate a streamed HuggingFace Safetensors header slice (`R4-50`, `R4-51`).
 * @param {string} repo_id
 * @param {Uint8Array} prefix_bytes
 * @returns {string}
 */
export function studio_hf_repo_parse_safetensors_header(repo_id, prefix_bytes) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(repo_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(prefix_bytes, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.studio_hf_repo_parse_safetensors_header(ptr0, len0, ptr1, len1);
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
 * Verify and hot-mount a `UORR4V12` geometric container into the active Studio engine (`R4-46`).
 * @param {Uint8Array} bytes
 * @returns {string}
 */
export function studio_mount_r4_v12_bytes(bytes) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_mount_r4_v12_bytes(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Mount and verify a complete `UORR4V13` geometric Qwen3 container (`R4-51`).
 * @param {string} model_id
 * @param {Uint8Array} bytes
 * @returns {string}
 */
export function studio_mount_holo_v13_bytes(model_id, bytes) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(model_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.studio_mount_holo_v13_bytes(ptr0, len0, ptr1, len1);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Formal R4.HoloGeometricRuntime begin incremental streaming mount of 1 MiB OPFS `.holo` pieces into Wasm (`R4-50`, `R4-51`).
 * @param {string} model_id
 * @param {number} expected_pieces
 * @param {string} expected_sha256
 * @returns {string}
 */
export function studio_begin_holo_v13_mount(model_id, expected_pieces, expected_sha256) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(model_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(expected_sha256, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.studio_begin_holo_v13_mount(ptr0, len0, expected_pieces, ptr1, len1);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Formal R4.HoloGeometricRuntime begin incremental streaming mount with verified SHA-256 skip-rehash flag.
 * @param {string} model_id
 * @param {number} expected_pieces
 * @param {string} expected_sha256
 * @param {boolean} skip_rehash
 * @returns {string}
 */
export function studio_begin_holo_v13_mount_verified(model_id, expected_pieces, expected_sha256, skip_rehash) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(model_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(expected_sha256, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.studio_begin_holo_v13_mount_verified(ptr0, len0, expected_pieces, ptr1, len1, skip_rehash);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
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
    imports.wbg.__wbg_call_672a4d21634d4a24 = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.call(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_call_7cccdd69e0791ae2 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.call(arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_call_833bed5770ea2041 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = arg0.call(arg1, arg2, arg3);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_get_67b2ba62fc30de12 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_newnoargs_105ed471475aaf50 = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return ret;
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_88a902d13a557d07 = function() {
        const ret = typeof global === 'undefined' ? null : global;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_THIS_56578be7e9f832b0 = function() {
        const ret = typeof globalThis === 'undefined' ? null : globalThis;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_SELF_37c5d418e4bf5819 = function() {
        const ret = typeof self === 'undefined' ? null : self;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_WINDOW_5de37043a91a9c40 = function() {
        const ret = typeof window === 'undefined' ? null : window;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbindgen_init_externref_table = function() {
        const table = wasm.__wbindgen_export_2;
        const offset = table.grow(4);
        table.set(0, undefined);
        table.set(offset + 0, undefined);
        table.set(offset + 1, null);
        table.set(offset + 2, true);
        table.set(offset + 3, false);
        ;
    };
    imports.wbg.__wbindgen_is_function = function(arg0) {
        const ret = typeof(arg0) === 'function';
        return ret;
    };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        const ret = arg0 === undefined;
        return ret;
    };
    imports.wbg.__wbindgen_number_new = function(arg0) {
        const ret = arg0;
        return ret;
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return ret;
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
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

/** Formal R4.HoloGeometricRuntime select the highest-capability Qwen3 boot lane (warm 1.7B immediate or 0.6B->1.7B idle promotion). */
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
    initialModel: warm17b ? 'uor-r4.qwen3-1.7b-holo' : 'uor-r4.qwen3-0.6b-holo',
    promoteTarget: !warm17b && capableDesktop17b ? 'uor-r4.qwen3-1.7b-holo' : null,
  };
}

/** Formal R4.HoloKappaStore hydrate `UORR4V13` pieces into `OpfsKappaStore` with Stitch 2-of-3 recovery & verified skip-rehash mount into Wasm. */
export async function hydrate_holo_model(modelId, onProgress, forceReload = false) {
  const key = String(modelId || 'uor-r4.qwen3-0.6b-holo').trim();
  if (!forceReload && holoHydrationInflight.has(key)) {
    return await holoHydrationInflight.get(key);
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
    const candidates = [
      ...spec.mirrors.map((m) => (m.startsWith('http') ? m : new URL(m, import.meta.url).href)),
      ...(httpOrigin ? [`${httpOrigin}/weights/${slug}/`] : []),
    ];
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
      } else {
        throw new Error(`Unable to fetch UORR4V13 manifest for ${key}`);
      }
    }
    let stitchManifest = null;
    if (baseUrl.startsWith('http')) {
      try {
        const sr = await fetch(`${baseUrl}manifest.stitch.json`);
        if (sr.ok) stitchManifest = await sr.json();
      } catch {}
    }
    const pieceCount = manifest.pieces.length;
    const containerSha = String(manifest.container_sha256 || '').toLowerCase();
    if (typeof studio_begin_holo_v13_mount_verified === 'function') {
      studio_begin_holo_v13_mount_verified(key, pieceCount, containerSha, true);
    } else {
      studio_begin_holo_v13_mount(key, pieceCount, containerSha);
    }

    const resolveOrRecoverPiece = async (pieceMeta) => {
      const sha = String(pieceMeta.sha256 || '').toLowerCase();
      const expectedLen = Number(pieceMeta.byte_length) || 0;
      if (!forceReload) {
        const cached = await sharedKappaStore.getVerifiedPiece(sha, expectedLen);
        if (cached) return { bytes: cached, fromOpfs: true };
      }
      const urls = [...(Array.isArray(manifest.mirrors) ? manifest.mirrors : []), baseUrl]
        .filter((u) => typeof u === 'string' && u.startsWith('http'))
        .map((u) => (u.endsWith('/') ? u : `${u}/`));
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
      const pieceMirrorBase = urls[0] || baseUrl;
      if (stitchManifest && Array.isArray(stitchManifest.groups)) {
        const gIdx = Math.floor(pieceMeta.index / 2);
        const grp = stitchManifest.groups[gIdx];
        if (grp && grp.parity && grp.left) {
          const isLeft = pieceMeta.index % 2 === 0;
          const peerMeta = isLeft ? grp.right : grp.left;
          const peerBytes = peerMeta
            ? ((await sharedKappaStore.getVerifiedPiece(peerMeta.sha256, peerMeta.byte_length)) ||
               (pieceMirrorBase.startsWith('http') ? await holoFetchWithHoldTail(`${pieceMirrorBase}${peerMeta.file}`, peerMeta.sha256).catch(() => null) : null))
            : new Uint8Array(0);
          const parityBytes = pieceMirrorBase.startsWith('http')
            ? await holoFetchWithHoldTail(`${pieceMirrorBase}${grp.parity.file}`, grp.parity.sha256).catch(() => null)
            : null;
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

    let cachedHits = 0;
    const CONCURRENCY = 6;
    for (let i = 0; i < pieceCount; i += CONCURRENCY) {
      const batch = manifest.pieces.slice(i, Math.min(i + CONCURRENCY, pieceCount));
      const results = await Promise.all(batch.map((pm) => resolveOrRecoverPiece(pm)));
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
  holoHydrationInflight.set(key, job);
  try {
    return await job;
  } finally {
    if (holoHydrationInflight.get(key) === job) {
      holoHydrationInflight.delete(key);
    }
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
      if (frame && (frame.phase !== 'prefill' || frame.top_candidates?.length > 0 || (Number(frame.step_idx) % 8 === 0))) {
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
        const holo = await hydrate_holo_model(modelId, (prog) => {
          postEvent('boot_progress', { stage: 'holo_v13', ...prog });
          postEvent('hydration_progress', { stage: 'holo_v13', ...prog });
        }, force);
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
      if (op === 'inprism_r4_turn') {
        /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
        const prompt = String((payload && payload.prompt) || '').trim();
        if (!prompt) {
          self.postMessage({ id, ok: false, error: 'Prompt must not be empty for inprism_r4_turn' });
          return;
        }
        const preferred = String((payload && payload.preferredModel) || '').trim();
        const idState = await stitch_prism_identity_ensure().catch(() => null);
        let hs = typeof studio_holo_runtime_status === 'function' ? JSON.parse(studio_holo_runtime_status()) : null;
        let mountedList = hs && Array.isArray(hs.models) ? hs.models : [];
        let activeModel = '';
        if (preferred && (preferred === 'uor-r4.qwen3-1.7b-holo' || preferred === 'uor-r4.qwen3-0.6b-holo' || preferred === 'uor-r4.native-q31-d85')) {
          activeModel = preferred;
        } else if (hs && hs.qwen3_17b_mounted) {
          activeModel = 'uor-r4.qwen3-1.7b-holo';
        } else if (hs && hs.qwen3_06b_mounted) {
          activeModel = 'uor-r4.qwen3-0.6b-holo';
        } else {
          const opt = await selectOptimalInitialHoloModel().catch(() => null);
          activeModel = (opt && opt.initialModel) || 'uor-r4.qwen3-0.6b-holo';
        }
        if ((activeModel === 'uor-r4.qwen3-1.7b-holo' || activeModel === 'uor-r4.qwen3-0.6b-holo')
            && !mountedList.some((m) => m && m.model_id === activeModel)) {
          try {
            await hydrate_holo_model(activeModel, (prog) => {
              postEvent('boot_progress', { stage: 'holo_v13', ...prog });
              postEvent('hydration_progress', { stage: 'holo_v13', ...prog });
            }, false);
            hs = typeof studio_holo_runtime_status === 'function' ? JSON.parse(studio_holo_runtime_status()) : hs;
            mountedList = hs && Array.isArray(hs.models) ? hs.models : mountedList;
          } catch {
            activeModel = hs && hs.qwen3_06b_mounted ? 'uor-r4.qwen3-0.6b-holo' : 'uor-r4.native-q31-d85';
          }
        }
        const mountedMeta = mountedList.find((m) => m && m.model_id === activeModel) || null;
        const chatReq = JSON.stringify({
          prompt,
          selected_model: activeModel,
          composer_mode: 'chat',
          operating_mode: 'chat',
          connected_repo: (payload && payload.repo) || '',
          connected_branch: (payload && payload.branch) || 'main',
        });
        const chatRes = JSON.parse(studio_dispatch(`1\tstudio\tchat\t${chatReq}`));
        const weightsSha = (mountedMeta && mountedMeta.container_sha256) || chatRes.receipt_sha256 || '';
        const weightsKappa = weightsSha ? `sha256:${weightsSha}` : '';
        const latestHolo = __uorMemPrismIdentity.latestHoloSummary || globalThis.__uorLatestHoloSummary || null;
        const actionRes = weightsKappa
          ? await stitch_prism_identity_sign_action('inprism-r4-turn', weightsKappa).catch(() => null)
          : null;
        self.postMessage({
          id,
          ok: chatRes && chatRes.status === 'ok',
          result: {
            ok: Boolean(chatRes && chatRes.status === 'ok'),
            mode: 'inprism',
            engine: 'match',
            weights: weightsKappa ? 'match' : 'missing',
            route: 'none',
            boundary: 'allow',
            model_id: chatRes.selected_model || activeModel,
            execution_tier: chatRes.execution_tier || activeModel,
            reply: chatRes.assistant_reply || '',
            receipt_sha256: chatRes.receipt_sha256 || '',
            weights_kappa: weightsKappa,
            prism_id: idState ? idState.prism_id : null,
            action_record_kappa: actionRes ? actionRes.action_record_kappa : null,
            holo_fingerprint: latestHolo ? latestHolo.fingerprint : null,
            geometric_metrics: chatRes.geometric_metrics || null,
          },
          ...(chatRes && chatRes.status !== 'ok' ? { error: chatRes.message || 'inprism turn failed' } : {}),
        });
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

export { initSync };
export default __wbg_init;
