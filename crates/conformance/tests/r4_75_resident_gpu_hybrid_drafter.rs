//! Conformance test for R4-75: Honest Resident-RAM Turn UX, Digest Cache-Busting,
//! Real 28-Layer WebGPU Weight Slabs, and Hybrid `UORR4V14` -> `HoloV13` Speculative Drafter.

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_resident_gpu_hybrid_drafter::{
    admit_hybrid_v14_webgpu_draft, admit_resident_turn_zero_download,
    verify_resident_gpu_hybrid_drafter_admitted, HybridDraftVerdict, ResidentTurnVerdict,
};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn check_lex_module(root: &Path, rel: &str, expected_mod: &str, required_decls: &[&str]) {
    let raw =
        std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("missing {rel}: {e}"));
    assert!(raw.contains(&format!("\\begin{{lexlean}}{{{expected_mod}}}")));
    let (_, after) = raw
        .split_once("\\semanticdata{")
        .unwrap_or_else(|| panic!("missing \\semanticdata{{ in {rel}"));
    let (json_block, _) = after
        .split_once("\n\\end{semanticmodule}")
        .unwrap_or_else(|| panic!("missing \\end{{semanticmodule}} in {rel}"));
    let json_line = json_block
        .trim()
        .strip_suffix('}')
        .unwrap_or(json_block.trim());
    let parsed: Value = serde_json::from_str(json_line).expect("valid JSON semantic module");
    assert_eq!(parsed["spec"], "lexlean/semantic-module/2");
    for sym in required_decls {
        assert!(raw.contains(sym), "{rel} must declare {sym}");
    }
}

#[test]
fn test_r4_75_resident_gpu_hybrid_drafter_r4_75() {
    let root = repo_root();

    // 1. Formal LexLean R4-75 admission and R5 negative refusals
    assert!(verify_resident_gpu_hybrid_drafter_admitted());
    assert_eq!(
        admit_resident_turn_zero_download(true, false, 0, 0, true, true, false),
        ResidentTurnVerdict::ResidentTurnZeroDownloadAdmitted
    );
    assert_eq!(
        admit_resident_turn_zero_download(false, true, 410_000_000, 0, true, true, false),
        ResidentTurnVerdict::ColdOrUnmountedHydrationAdmitted
    );
    assert_eq!(
        admit_resident_turn_zero_download(true, true, 1024, 0, true, true, false),
        ResidentTurnVerdict::RefusedHydrationOnResidentModel
    );
    assert_eq!(
        admit_resident_turn_zero_download(true, false, 0, 0, false, true, false),
        ResidentTurnVerdict::RefusedDesyncedModelSelectors
    );
    assert_eq!(
        admit_resident_turn_zero_download(true, false, 0, 0, true, false, false),
        ResidentTurnVerdict::RefusedStaleStaticAssetQuery
    );
    assert_eq!(
        admit_resident_turn_zero_download(true, false, 0, 0, true, true, true),
        ResidentTurnVerdict::RefusedMisleadingStreamingLabel
    );

    assert_eq!(
        admit_hybrid_v14_webgpu_draft(true, 11_796_480, 28, 4, 0, 0, false),
        HybridDraftVerdict::HybridV14WebGpuDraftAdmitted
    );
    assert_eq!(
        admit_hybrid_v14_webgpu_draft(false, 0, 28, 4, 0, 0, false),
        HybridDraftVerdict::HybridV14WasmSimdFallbackAdmitted
    );
    assert_eq!(
        admit_hybrid_v14_webgpu_draft(true, 64, 28, 4, 0, 0, false),
        HybridDraftVerdict::RefusedPlaceholderZeroLayerSlab
    );
    assert_eq!(
        admit_hybrid_v14_webgpu_draft(true, 11_796_480, 28, 4, 1, 0, false),
        HybridDraftVerdict::RefusedFloatOrNonZeroVerifierDrift
    );
    assert_eq!(
        admit_hybrid_v14_webgpu_draft(true, 11_796_480, 28, 4, 0, 0, true),
        HybridDraftVerdict::RefusedPrefillMapAsyncStall
    );
    assert_eq!(
        admit_hybrid_v14_webgpu_draft(true, 11_796_480, 28, 7, 0, 0, false),
        HybridDraftVerdict::RefusedDraftWaveExceedsTile
    );

    // 2. Verify .lex.tex semantic modules exist and declare all required symbols
    check_lex_module(
        &root,
        "src/R4/ResidentGpuHybridDrafter.lex.tex",
        "R4.ResidentGpuHybridDrafter",
        &[
            "ResidentTurnVerdict",
            "HybridDraftVerdict",
            "admitResidentTurnZeroDownload",
            "admitHybridV14WebGpuDraft",
            "verifyResidentGpuHybridDrafterAdmitted",
            "breaksHydrationOnResidentModelAdmitted",
            "breaksDesyncedSelectorsOrStaleQueryAdmitted",
            "breaksPlaceholder64ByteGpuSlabAdmitted",
            "breaksPrefillMapAsyncOrVerifierDriftAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/ResidentGpuHybridDrafterControls.lex.tex",
        "ResidentGpuHybridDrafterControls",
        &[
            "residentGpuHybridDrafterAdmitted",
            "breaksHydrationOnResidentModelAdmittedCtrl",
            "breaksDesyncedSelectorsOrStaleQueryAdmittedCtrl",
            "breaksPlaceholder64ByteGpuSlabAdmittedCtrl",
            "breaksPrefillMapAsyncOrVerifierDriftAdmittedCtrl",
        ],
    );

    // 3. Verify content-addressed ?v=v12r4-<js_sha12> stamping in public/index.html & public/app.js
    let prov_raw =
        std::fs::read_to_string(root.join("public/provenance.json")).expect("read provenance.json");
    let prov: Value = serde_json::from_str(&prov_raw).expect("parse provenance.json");
    let js_sha12 = &prov["binding_javascript_sha256"]
        .as_str()
        .expect("binding_javascript_sha256")[..12];
    let expected_ver = format!("v=v12r4-{js_sha12}");

    let index_html =
        std::fs::read_to_string(root.join("public/index.html")).expect("read public/index.html");
    assert!(
        index_html.contains(&format!("app.css?{expected_ver}"))
            && index_html.contains(&format!("app.js?{expected_ver}")),
        "public/index.html must stamp digest cache-busting query `{expected_ver}`"
    );

    let app_js = std::fs::read_to_string(root.join("public/app.js")).expect("read public/app.js");
    assert!(
        app_js.contains(&format!("./lexlean_r4.js?{expected_ver}")),
        "public/app.js must stamp digest cache-busting query `./lexlean_r4.js?{expected_ver}`"
    );

    // 4. Verify public/app.js reads visible globalModelSelect ahead of composerModelSelect
    //    and uses honest Resident · Generating (0 B dl) header instead of misleading Streaming on mounted models
    assert!(
        app_js.contains("globalModelSelect && globalModelSelect.value"),
        "public/app.js runClaudeChat must read visible globalModelSelect ahead of hidden composerModelSelect"
    );
    assert!(
        app_js.contains("Resident · Generating (0 B dl)"),
        "public/app.js must display `Resident · Generating (0 B dl)` instead of misleading `· Streaming` when target model is mounted"
    );
    assert!(
        !app_js.contains("`${selectedModel} · ${activeTierTag} · Streaming`"),
        "public/app.js must remove misleading `{{selectedModel}} · {{activeTierTag}} · Streaming` header"
    );

    // 5. Verify worker_host_bridge.js emits holo_ready on op === 'hydrate_holo_model'
    //    and streams real per-layer slabs via studio_export_holo_layer_slab_bytes (not 64-byte zero placeholders)
    let bridge_js = std::fs::read_to_string(root.join("crates/wasm/js/worker_host_bridge.js"))
        .expect("read worker_host_bridge.js");
    assert!(
        bridge_js.contains("mountResidentModelSlabsFromExporter")
            && bridge_js.contains("studio_export_holo_layer_slab_bytes"),
        "worker_host_bridge.js must stream real 28-layer Int4 RHT-32 slabs via studio_export_holo_layer_slab_bytes"
    );
    assert!(
        !bridge_js.contains("new Uint8Array(nL * 64)"),
        "worker_host_bridge.js must not mount 64-byte zero placeholder slabs during model hydration"
    );

    // 6. Verify Wasm exports studio_export_holo_layer_slab_bytes and wires HopfZetaPoincareEngineV14::propose_speculative_wave_holo_ids
    let lib_rs = std::fs::read_to_string(root.join("crates/wasm/src/lib.rs")).expect("read lib.rs");
    assert!(
        lib_rs.contains("pub fn studio_export_holo_layer_slab_bytes("),
        "crates/wasm/src/lib.rs must export studio_export_holo_layer_slab_bytes"
    );

    let hopf_rs = std::fs::read_to_string(root.join("crates/wasm/src/hopf_zeta_engine.rs"))
        .expect("read hopf_zeta_engine.rs");
    assert!(
        hopf_rs.contains("pub fn propose_speculative_wave_holo_ids"),
        "hopf_zeta_engine.rs must implement propose_speculative_wave_holo_ids"
    );
}
