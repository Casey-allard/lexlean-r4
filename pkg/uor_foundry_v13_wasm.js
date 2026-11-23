/* @ts-self-types="./uor_foundry_v13_wasm.d.ts" */

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

/**
 * Formal R4.HoloGeometricRuntime begin a non-blocking chunked Studio/Foundry turn session (`R4-56`, `R4-60`).
 * @param {string} payload_json
 * @returns {string}
 */
export function holo_begin_foundry_turn(payload_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(payload_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.holo_begin_foundry_turn(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Formal R4.HoloGeometricRuntime atomically cancel an active Studio/Foundry turn and restore KV checkpoint (`R4-56`, `R4-60`).
 * @param {bigint} turn_id
 * @returns {string}
 */
export function holo_cancel_foundry_turn(turn_id) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.holo_cancel_foundry_turn(turn_id);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Formal R4.LiveWebGpuTurnBridge verify and commit a bound WebGPU compute pass for an active turn (`R4-65`).
 * @param {bigint} turn_id
 * @param {bigint} total_dispatches
 * @param {bigint} bound_dispatches
 * @param {bigint} max_abs_lsb_diff
 * @returns {string}
 */
export function holo_notify_foundry_turn_gpu_bound_step(turn_id, total_dispatches, bound_dispatches, max_abs_lsb_diff) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.holo_notify_foundry_turn_gpu_bound_step(turn_id, total_dispatches, bound_dispatches, max_abs_lsb_diff);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Formal R4.LiveWebGpuTurnBridge peek active turn's 512-bit query signature (`R4-65`).
 * @param {bigint} turn_id
 * @returns {Uint32Array}
 */
export function holo_peek_foundry_turn_qsig512(turn_id) {
    const ret = wasm.holo_peek_foundry_turn_qsig512(turn_id);
    var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v1;
}

/**
 * Formal R4.HoloGeometricRuntime execute up to `slice_tokens` (`1..=4`) pure-integer token steps and emit live HUD frames on an active Studio/Foundry turn (`R4-56`, `R4-60`).
 * @param {bigint} turn_id
 * @returns {string}
 */
export function holo_step_foundry_turn(turn_id) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.holo_step_foundry_turn(turn_id);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Export minimal filtered UTF-8 `/workspace` writes and deletes since last sync (`R4-57`).
 * @returns {string}
 */
export function mvm_export_workspace_diff_json() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.mvm_export_workspace_diff_json();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Synchronize base64 `/workspace` snapshot JSON into `MVM-Wasm` and Studio (`R4-57`).
 * @param {string} snapshot_json
 * @returns {string}
 */
export function mvm_sync_workspace_in_json(snapshot_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(snapshot_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.mvm_sync_workspace_in_json(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
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
 * Formal R4.StudioFrame evaluate first-party catalog auto-hydration and non-blocking Neural HUD policy (`R4-62`).
 * @param {string} origin_kind
 * @param {boolean} explicit_consent
 * @param {boolean} emergency_offline_toggle
 * @param {boolean} silent_sticky_declined
 * @param {string} hud_mode
 * @param {boolean} has_blocking_backdrop
 * @returns {string}
 */
export function studio_admit_catalog_hydration_json(origin_kind, explicit_consent, emergency_offline_toggle, silent_sticky_declined, hud_mode, has_blocking_backdrop) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(origin_kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(hud_mode, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.studio_admit_catalog_hydration_json(ptr0, len0, explicit_consent, emergency_offline_toggle, silent_sticky_declined, ptr1, len1, has_blocking_backdrop);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
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
 * Formal R4.MvmLinuxContainer evaluate single-source MVM workspace, SpiralCore CORS preemption, and interactive command policy (`R4-63`).
 * @param {boolean} split_brain
 * @param {boolean} cow_consistent
 * @param {boolean} spiral_core_net
 * @param {boolean} is_private_or_loopback
 * @param {boolean} has_credentials
 * @param {boolean} direct_fetch_cors_blocked
 * @param {string} mcp_mode
 * @param {boolean} plan_mode
 * @param {boolean} auto_run
 * @param {boolean} mutating_cmd
 * @param {bigint} header_tokens
 * @returns {string}
 */
export function studio_admit_mvm_container_policy_json(split_brain, cow_consistent, spiral_core_net, is_private_or_loopback, has_credentials, direct_fetch_cors_blocked, mcp_mode, plan_mode, auto_run, mutating_cmd, header_tokens) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(mcp_mode, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_admit_mvm_container_policy_json(split_brain, cow_consistent, spiral_core_net, is_private_or_loopback, has_credentials, direct_fetch_cors_blocked, ptr0, len0, plan_mode, auto_run, mutating_cmd, header_tokens);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
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

/**
 * Formal R4.FoundryAgentToolSurface Wasm export executing policy-checked Studio `<tool_call>` invocations (`foundry_prove`, `foundry_publish_holo`, `vtensor_render_3d`, `mvm_exec`, `workspace_*`) (`R4-77`).
 * @param {string} tool_call_json
 * @param {string} composer_mode
 * @returns {string}
 */
export function studio_execute_agent_tool_json(tool_call_json, composer_mode) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(tool_call_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(composer_mode, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.studio_execute_agent_tool_json(ptr0, len0, ptr1, len1);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Formal R4.ResidentGpuHybridDrafter stream real packed Int4 RHT-32 + Q24 RMSNorm slab bytes for resident layer `layer_idx` (`R4-75`).
 * @param {string} model_id
 * @param {number} layer_idx
 * @returns {Uint8Array}
 */
export function studio_export_holo_layer_slab_bytes(model_id, layer_idx) {
    const ptr0 = passStringToWasm0(model_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.studio_export_holo_layer_slab_bytes(ptr0, len0, layer_idx);
    var v2 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v2;
}

/**
 * Formal R4.WebGpuResidentRuntime export per-layer (`<= 128 MiB`) resident WebGPU storage buffer partition metadata (`R4-64`).
 * @param {string} model_id
 * @param {bigint} device_binding_limit_bytes
 * @param {bigint} float_ops
 * @returns {string}
 */
export function studio_export_holo_layer_slab_meta(model_id, device_binding_limit_bytes, float_ops) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(model_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_export_holo_layer_slab_meta(ptr0, len0, device_binding_limit_bytes, float_ops);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Formal R4.LiveWebGpuTurnBridge export vocabulary 512-bit RHT signatures as `u32` words (`R4-65`).
 * @param {string} model_id
 * @returns {Uint32Array}
 */
export function studio_export_holo_vocab_sigs_u32(model_id) {
    const ptr0 = passStringToWasm0(model_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.studio_export_holo_vocab_sigs_u32(ptr0, len0);
    var v2 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v2;
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
 * Formal R4.FoundryQuantumStudio Wasm self-check verifying `FK-01..09`, `InTabProver`, `MlsChannel` (`FR-05`), and canonical `kappa` (`R4-71`).
 * @returns {string}
 */
export function studio_foundry_quantum_self_check_json() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_foundry_quantum_self_check_json();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
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
 * Formal R4.HopfZetaPoincareArchitecture Wasm export running `UORR4V14` 4-token resonant wave generation (`R4-73`).
 * @param {string} prompt
 * @param {number} max_tokens
 * @returns {string}
 */
export function studio_hopf_zeta_v14_generate_json(prompt, max_tokens) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(prompt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_hopf_zeta_v14_generate_json(ptr0, len0, max_tokens);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Formal R4.HopfZetaPoincareArchitecture Wasm self-check verifying `UORR4V14` 5-pillar invariants (`R4-72`, `R4-73`).
 * @returns {string}
 */
export function studio_hopf_zeta_v14_self_check_json() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_hopf_zeta_v14_self_check_json();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Hydrate 64 KB `MVM-Wasm` Copy-on-Write block sectors loaded from Worker OPFS (`R4-37`, `R4-60`).
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
 * Formal R4.RealQwen3LosslessStreaming Wasm export prewarming `HoloPrefixKvCache` on an idle Worker tick (`R4-82`).
 * @param {string} model_id
 * @returns {string}
 */
export function studio_prewarm_holo_prefix_idle(model_id) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(model_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_prewarm_holo_prefix_idle(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
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
 * Wasm export returning active Sovereign Dual-Surface Ratchet status JSON (`R4.DualSurfaceRatchetEvolution`, `R4-84`).
 * @returns {string}
 */
export function studio_ratchet_status_json() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_ratchet_status_json();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Wasm export executing one Rule-30 × Telos × `CAP0005` Sovereign Ratchet step (`R4.DualSurfaceRatchetEvolution`, `R4-84`).
 * @param {string} req_json
 * @returns {string}
 */
export function studio_ratchet_step_json(req_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(req_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_ratchet_step_json(ptr0, len0);
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
 * Formal R4.VtensorSemiring3d render live Neural HUD / Observatory 3D Hopf-Poincare-Zeta RGBA8 frame (`R4-70`).
 * @param {number} width
 * @param {number} height
 * @param {number} rot_phase1024
 * @param {number} lod_level
 * @param {string} trail_csv
 * @returns {Uint8Array}
 */
export function studio_render_hopf_zeta_3d_rgba(width, height, rot_phase1024, lod_level, trail_csv) {
    const ptr0 = passStringToWasm0(trail_csv, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.studio_render_hopf_zeta_3d_rgba(width, height, rot_phase1024, lod_level, ptr0, len0);
    var v2 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v2;
}

/**
 * Formal R4.VtensorSemiring3d render Studio `?surface=preview` 3D PBR scene from JSON (`R4-70`).
 * @param {string} scene_json
 * @param {number} width
 * @param {number} height
 * @returns {Uint8Array}
 */
export function studio_render_preview_scene_3d_rgba(scene_json, width, height) {
    const ptr0 = passStringToWasm0(scene_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.studio_render_preview_scene_3d_rgba(ptr0, len0, width, height);
    var v2 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v2;
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
 * Formal R4.WebGpuGeodesicRuntime configure Geodesic SwiGLU active-neuron shortlist mode across mounted `HoloV13Model`s (`R4-61`).
 * @param {string} mode
 * @param {number} shortlist_k
 * @returns {string}
 */
export function studio_set_holo_mlp_mode(mode, shortlist_k) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(mode, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_set_holo_mlp_mode(ptr0, len0, shortlist_k);
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

/**
 * Formal R4.Qwen3SparseBasinV15 Wasm export returning `UORR4V15` sparse-basin readiness and formal admission receipt (`R4-80`).
 * @returns {string}
 */
export function studio_sparse_v15_status_json() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_sparse_v15_status_json();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Formal R4.Qwen3SparseBasinV15 Wasm export executing an instant-boot `>= 10,000 tok/s` `UORR4V15` sparse-basin Studio/Chat turn (`R4-80`).
 * @param {string} req_json
 * @returns {string}
 */
export function studio_sparse_v15_turn_json(req_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(req_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_sparse_v15_turn_json(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
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
 * Formal R4.FoundryQuantumStudio Wasm export computing float-forbidding unique-key canonical JSON + `blake3:` `kappa` (`R4-71`).
 * @param {string} raw_json
 * @returns {string}
 */
export function studio_uor_canonical_kappa_json(raw_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(raw_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_uor_canonical_kappa_json(ptr0, len0);
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
 * Formal R4.WebGpuGeodesicRuntime validate a sealed `HOLOSLAB2` OPFS manifest against quota, slab byte length, and Merkle root (`R4-61`).
 * @param {string} seal_json
 * @param {bigint} quota_bytes
 * @param {bigint} required_bytes
 * @param {bigint} actual_slab_bytes
 * @param {boolean} full_scan
 * @param {boolean} piece_hashes_valid
 * @returns {string}
 */
export function studio_validate_holoslab2_seal(seal_json, quota_bytes, required_bytes, actual_slab_bytes, full_scan, piece_hashes_valid) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(seal_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.studio_validate_holoslab2_seal(ptr0, len0, quota_bytes, required_bytes, actual_slab_bytes, full_scan, piece_hashes_valid);
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
 * Formal R4.WebGpuResidentRuntime evaluate resident WebGPU slab limits, `0-LSB` speculative tile parity, and Studio LoRA prompt shrink policy (`R4-64`).
 * @param {bigint} float_ops
 * @param {bigint} layer_max_bytes
 * @param {bigint} device_binding_limit_bytes
 * @param {bigint} draft_count
 * @param {bigint} max_abs_lsb_diff
 * @param {boolean} kv_rewound_exact
 * @param {boolean} stitch_parity_valid
 * @param {boolean} fine_tuned_manifest_pinned
 * @param {bigint} prompt_tokens
 * @returns {string}
 */
export function studio_verify_resident_webgpu_step(float_ops, layer_max_bytes, device_binding_limit_bytes, draft_count, max_abs_lsb_diff, kv_rewound_exact, stitch_parity_valid, fine_tuned_manifest_pinned, prompt_tokens) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_verify_resident_webgpu_step(float_ops, layer_max_bytes, device_binding_limit_bytes, draft_count, max_abs_lsb_diff, kv_rewound_exact, stitch_parity_valid, fine_tuned_manifest_pinned, prompt_tokens);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Formal R4.WebGpuGeodesicRuntime verify pure-integer WGSL kernel parity and chunk budget (`R4-61`).
 * @param {number} float_ops
 * @param {number} max_abs_lsb_diff
 * @param {number} staging_chunk_bytes
 * @returns {string}
 */
export function studio_verify_wgsl_kernel_parity(float_ops, max_abs_lsb_diff, staging_chunk_bytes) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_verify_wgsl_kernel_parity(float_ops, max_abs_lsb_diff, staging_chunk_bytes);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Formal R4.VtensorSemiring3d emit structured self-verification JSON receipt (`R4-70`).
 * @returns {string}
 */
export function studio_vtensor_3d_self_check_json() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.studio_vtensor_3d_self_check_json();
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
 * @param {string} model_id
 * @param {number} expected_pieces
 * @param {string} expected_sha256
 * @param {boolean} skip_rehash
 * @returns {string}
 */
export function v13_begin_mount(model_id, expected_pieces, expected_sha256, skip_rehash) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(model_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(expected_sha256, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.v13_begin_mount(ptr0, len0, expected_pieces, ptr1, len1, skip_rehash);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * @param {string} payload_json
 * @returns {string}
 */
export function v13_begin_turn(payload_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(payload_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.v13_begin_turn(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @param {bigint} turn_id
 * @returns {string}
 */
export function v13_cancel_turn(turn_id) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.v13_cancel_turn(turn_id);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * @returns {string}
 */
export function v13_finalize_mount() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.v13_finalize_mount();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * @param {Uint8Array} piece
 * @returns {string}
 */
export function v13_push_piece(piece) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passArray8ToWasm0(piece, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.v13_push_piece(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * @returns {string}
 */
export function v13_status() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.v13_status();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * @param {bigint} turn_id
 * @returns {string}
 */
export function v13_step_turn(turn_id) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.v13_step_turn(turn_id);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
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
export function v13_stitch_recover_object(has_frag0, frag0, has_frag1, frag1, has_frag2, frag2, original_length, expected_object_sha256) {
    const ptr0 = passArray8ToWasm0(frag0, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(frag1, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passArray8ToWasm0(frag2, wasm.__wbindgen_malloc);
    const len2 = WASM_VECTOR_LEN;
    const ptr3 = passStringToWasm0(expected_object_sha256, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len3 = WASM_VECTOR_LEN;
    const ret = wasm.v13_stitch_recover_object(has_frag0, ptr0, len0, has_frag1, ptr1, len1, has_frag2, ptr2, len2, original_length, ptr3, len3);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v5 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v5;
}

/**
 * @param {string} group_json
 * @param {boolean} recover_left
 * @param {boolean} has_left
 * @param {Uint8Array} left
 * @param {boolean} has_right
 * @param {Uint8Array} right
 * @param {boolean} has_parity
 * @param {Uint8Array} parity
 * @param {string} expected_sha256
 * @returns {Uint8Array}
 */
export function v13_stitch_recover_pair(group_json, recover_left, has_left, left, has_right, right, has_parity, parity, expected_sha256) {
    const ptr0 = passStringToWasm0(group_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(left, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passArray8ToWasm0(right, wasm.__wbindgen_malloc);
    const len2 = WASM_VECTOR_LEN;
    const ptr3 = passArray8ToWasm0(parity, wasm.__wbindgen_malloc);
    const len3 = WASM_VECTOR_LEN;
    const ptr4 = passStringToWasm0(expected_sha256, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len4 = WASM_VECTOR_LEN;
    const ret = wasm.v13_stitch_recover_pair(ptr0, len0, recover_left, has_left, ptr1, len1, has_right, ptr2, len2, has_parity, ptr3, len3, ptr4, len4);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v6 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v6;
}

/**
 * @param {string} manifest_json
 * @returns {string}
 */
export function v13_stitch_verify_manifest(manifest_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(manifest_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.v13_stitch_verify_manifest(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_is_function_754e9f305ff6029e: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_undefined_67b456be8673d3d7: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_throw_1506f2235d1bdba0: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_call_40e4174f169eaca7: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.call(arg1, arg2, arg3);
            return ret;
        }, arguments); },
        __wbg_call_9c758de292015997: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_get_de6a0f7d4d18a304: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_now_190933fa139cc119: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_static_accessor_GLOBAL_9d53f2689e622ca1: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_THIS_a1a35cec07001a8a: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_4c59f6c7ea29a144: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_e70ae9f2eb052253: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbindgen_cast_0000000000000001: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./uor_foundry_v13_wasm_bg.js": import0,
    };
}

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function getArrayI32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedInt32ArrayMemory0 = null;
function getInt32ArrayMemory0() {
    if (cachedInt32ArrayMemory0 === null || cachedInt32ArrayMemory0.byteLength === 0) {
        cachedInt32ArrayMemory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passArray32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getUint32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

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
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedInt32ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
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

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('uor_foundry_v13_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
