//! Conformance test for R4-61: Pure-Integer WebGPU WGSL Compute Pipelines,
//! Geodesic SwiGLU Active-Neuron Shortlisting & Single-Slab HOLOSLAB2 OPFS Store.

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_webgpu_geodesic_runtime::{
    admit_holo_slab_mount, admit_wgsl_integer_kernel, classify_prefill_tile_width,
    select_mlp_shortlist_mode, verify_webgpu_geodesic_runtime_admitted, HoloSlabVerdict,
    MlpShortlistMode, MlpUserPolicy, WgslParityVerdict,
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
fn test_r4_61_webgpu_geodesic_slab_r4_61() {
    let root = repo_root();

    // 1. WGSL pure-integer 0-LSB parity admission + R5 typed refusals
    assert_eq!(
        admit_wgsl_integer_kernel(0, 0, 4_194_304, 4_194_304),
        WgslParityVerdict::ZeroLsbAdmitted
    );
    assert_eq!(
        admit_wgsl_integer_kernel(1, 0, 4_194_304, 4_194_304),
        WgslParityVerdict::RefusedFloatOpInWgsl
    );
    assert_eq!(
        admit_wgsl_integer_kernel(0, 1, 4_194_304, 4_194_304),
        WgslParityVerdict::RefusedWgslCpuParityDrift
    );
    assert_eq!(
        admit_wgsl_integer_kernel(0, 0, 4_194_305, 4_194_304),
        WgslParityVerdict::RefusedWasmStagingBudgetExceeded
    );

    // 2. Geodesic SwiGLU 256-bit sketch shortlist selection + R5 bounds refusal
    assert_eq!(
        select_mlp_shortlist_mode(MlpUserPolicy::AutoReceiptGated, 1536, 6144, 9167, 30_286),
        MlpShortlistMode::GeodesicSparse1536
    );
    assert_eq!(
        select_mlp_shortlist_mode(MlpUserPolicy::AutoReceiptGated, 1536, 6144, 8999, 30_286),
        MlpShortlistMode::DenseExact
    );
    assert_eq!(
        select_mlp_shortlist_mode(MlpUserPolicy::AutoReceiptGated, 1536, 6144, 9167, 35_001),
        MlpShortlistMode::DenseExact
    );
    assert_eq!(
        select_mlp_shortlist_mode(MlpUserPolicy::ForceDenseExact, 1536, 6144, 9167, 30_286),
        MlpShortlistMode::DenseExact
    );
    assert_eq!(
        select_mlp_shortlist_mode(MlpUserPolicy::ForceGeodesicSparse, 1536, 6144, 0, 100_000),
        MlpShortlistMode::GeodesicSparse1536
    );
    assert_eq!(
        select_mlp_shortlist_mode(MlpUserPolicy::ForceGeodesicSparse, 6145, 6144, 9167, 30_286),
        MlpShortlistMode::RefusedInvalidNeuronShortlist
    );

    // 3. Single-slab HOLOSLAB2 OPFS custody verdicts + R5 quota/seal/piece refusals
    assert_eq!(
        admit_holo_slab_mount(2_000_000_000, 1_140_250_782, true, false, true),
        HoloSlabVerdict::SlabSealAdmitted
    );
    assert_eq!(
        admit_holo_slab_mount(2_000_000_000, 1_140_250_782, true, true, true),
        HoloSlabVerdict::SlabFullMerkleVerified
    );
    assert_eq!(
        admit_holo_slab_mount(1_140_250_781, 1_140_250_782, true, false, true),
        HoloSlabVerdict::RefusedInsufficientStorageQuota
    );
    assert_eq!(
        admit_holo_slab_mount(2_000_000_000, 1_140_250_782, false, false, true),
        HoloSlabVerdict::RefusedSlabSealMismatch
    );
    assert_eq!(
        admit_holo_slab_mount(2_000_000_000, 1_140_250_782, true, true, false),
        HoloSlabVerdict::RefusedSlabPieceHashMismatch
    );

    // 4. Prefill tile clamp + full formal invariant
    assert_eq!(classify_prefill_tile_width(0, 32), 0);
    assert_eq!(classify_prefill_tile_width(17, 32), 17);
    assert_eq!(classify_prefill_tile_width(37, 32), 32);
    assert!(verify_webgpu_geodesic_runtime_admitted());

    // 5. Formal LexLean spec, controls, and model register presence
    check_lex_module(
        &root,
        "src/R4/WebGpuGeodesicRuntime.lex.tex",
        "R4.WebGpuGeodesicRuntime",
        &[
            "WgslParityVerdict",
            "MlpUserPolicy",
            "MlpShortlistMode",
            "HoloSlabVerdict",
            "admitWgslIntegerKernel",
            "selectMlpShortlistMode",
            "admitHoloSlabMount",
            "classifyPrefillTileWidth",
            "verifyWebGpuGeodesicRuntimeAdmitted",
            "breaksWgslDriftAdmitted",
            "breaksFloatWgslAdmitted",
            "breaksQuotaShortfallSlabAdmitted",
            "breaksInvalidMlpShortlistAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/WebGpuGeodesicRuntimeControls.lex.tex",
        "WebGpuGeodesicRuntimeControls",
        &[
            "webgpuGeodesicRuntimeAdmitted",
            "breaksWgslDriftAdmittedCtrl",
            "breaksFloatWgslAdmittedCtrl",
            "breaksQuotaShortfallSlabAdmittedCtrl",
            "breaksInvalidMlpShortlistAdmittedCtrl",
        ],
    );

    let ids_toml =
        std::fs::read_to_string(root.join("model/ids.toml")).expect("model/ids.toml must exist");
    assert!(
        ids_toml.contains("id = \"R4-61\""),
        "R4-61 must be registered in model/ids.toml"
    );

    let feature_file = root.join("features/suites/r4_61_webgpu_geodesic_slab.feature");
    assert!(feature_file.is_file(), "R4-61 feature suite must exist");
    let feature_text = std::fs::read_to_string(feature_file).expect("read feature file");
    assert!(feature_text.contains("@R4-61 @build"));

    // 6. Verify concrete runtime implementation markers (R3: fails until Steps 2–4 land)
    let runtime_rs =
        std::fs::read_to_string(root.join("crates/wasm/src/holo_geometric_runtime.rs"))
            .expect("holo_geometric_runtime.rs must exist");
    assert!(
        runtime_rs.contains("pub const MAX_PREFILL_TILE: usize = 32;"),
        "holo_geometric_runtime.rs must upgrade MAX_PREFILL_TILE to 32"
    );
    assert!(
        runtime_rs.contains("gate_row_sigs_256"),
        "HoloV13Layer must precompute 256-bit geodesic SwiGLU gate sketches"
    );

    let kappa_rs = std::fs::read_to_string(root.join("crates/wasm/src/holo_kappa_store.rs"))
        .expect("holo_kappa_store.rs must exist");
    assert!(
        kappa_rs.contains("HOLOSLAB2"),
        "holo_kappa_store.rs must validate HOLOSLAB2 single-slab manifests"
    );

    let bridge_js = std::fs::read_to_string(root.join("crates/wasm/js/worker_host_bridge.js"))
        .expect("worker_host_bridge.js must exist");
    for shader_tag in [
        "holo_rmsnorm_rht32_pack",
        "holo_int4_gemv_gemm",
        "holo_rope_flock_attn",
        "holo_hamming512_lmhead",
        "HOLOSLAB2",
        "RefusedInsufficientStorageQuota",
    ] {
        assert!(
            bridge_js.contains(shader_tag),
            "worker_host_bridge.js must implement {shader_tag}"
        );
    }
}
