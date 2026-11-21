//! Conformance test for R4-73: `UORR4V14` Hopf–Poincaré–Zeta–Icosian `10,000+ tok/s`
//! Browser Geometric LLM Engine (`6`-layer `Cl(0,6)` SpiralCore, `8`-head Starling Flock,
//! `576`-bit `XOR + popcnt`, `64` Zeta ordinates, `120`-vertex Icosian `0`-LSB snap).

#![allow(missing_docs)]

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_hopf_zeta_poincare_architecture::{
    admit_ten_k_geometric_step, bandwidth_sustainable_tokens_per_sec,
    verify_hopf_zeta_poincare_engine_admitted, TenKGeometricEngineVerdict, TenKGeometricStepCheck,
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
fn test_r4_73_hopf_zeta_poincare_engine_r4_73() {
    let root = repo_root();

    // Dense 307 MB scan ceiling = 266 tok/s (< 10,000 tok/s)
    assert_eq!(
        bandwidth_sustainable_tokens_per_sec(80_000, 307_200, 1),
        266
    );
    // Active <= 1,024 KB Hopf-Zeta basin casting 4 tokens/wave = 320,000 tok/s
    assert_eq!(
        bandwidth_sustainable_tokens_per_sec(80_000, 1_024, 4),
        320_000
    );

    let ok = TenKGeometricStepCheck {
        float_ops: 0,
        dense_matmul_ops: 0,
        softmax_ops: 0,
        bandwidth_mb_per_sec: 80_000,
        active_basin_kb: 1_024,
        tokens_cast_per_wave: 4,
        target_tokens_per_sec: 10_000,
        icosian_drift_lsb: 0,
        spiral_core_layers: 6,
        flock_heads: 8,
        hamming_xor_bits: 576,
    };
    assert_eq!(
        admit_ten_k_geometric_step(ok.clone()),
        TenKGeometricEngineVerdict::TenKWaveResonanceAdmitted
    );

    let mut dense = ok.clone();
    dense.active_basin_kb = 307_200;
    dense.tokens_cast_per_wave = 1;
    assert_eq!(
        admit_ten_k_geometric_step(dense),
        TenKGeometricEngineVerdict::RefusedDenseWeightScanBandwidthGap
    );

    let mut drift = ok.clone();
    drift.icosian_drift_lsb = 1;
    assert_eq!(
        admit_ten_k_geometric_step(drift),
        TenKGeometricEngineVerdict::RefusedIcosianNormDrift
    );

    let mut banned = ok;
    banned.softmax_ops = 1;
    assert_eq!(
        admit_ten_k_geometric_step(banned),
        TenKGeometricEngineVerdict::RefusedBannedFloatMatmulOrSoftmax
    );

    assert!(verify_hopf_zeta_poincare_engine_admitted());

    check_lex_module(
        &root,
        "src/R4/HopfZetaPoincareArchitecture.lex.tex",
        "R4.HopfZetaPoincareArchitecture",
        &[
            "TenKGeometricStepCheck",
            "TenKGeometricEngineVerdict",
            "bandwidthSustainableTokensPerSec",
            "admitTenKGeometricStep",
            "verifyHopfZetaPoincareEngineAdmitted",
            "breaksDenseScanReachesTenKAdmitted",
            "breaksIcosianDriftAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/HopfZetaPoincareArchitectureControls.lex.tex",
        "HopfZetaPoincareArchitectureControls",
        &[
            "hopfZetaPoincareEngineAdmitted",
            "breaksDenseScanReachesTenKAdmittedCtrl",
            "breaksIcosianDriftAdmittedCtrl",
        ],
    );

    let eng_rs = std::fs::read_to_string(root.join("crates/wasm/src/hopf_zeta_engine.rs"))
        .expect("crates/wasm/src/hopf_zeta_engine.rs must exist");
    assert!(eng_rs.contains("pub fn studio_hopf_zeta_v14_generate_json"));
    assert!(eng_rs.contains("pub fn snap_to_icosian_120"));
    assert!(eng_rs.contains("ZETA_ZEROS_64_Q29"));
}
