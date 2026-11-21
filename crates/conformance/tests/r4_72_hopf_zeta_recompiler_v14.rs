//! Conformance test for R4-72: `UORR4V14` 4D Signed Hopf, Golden-Ratio Poincaré, Zeta-Zero,
//! 120-Vertex Icosian, and `C1/S5` Whole-Context Teacher Recompiler (`Qwen3-0.6B` & `Qwen3-1.7B`).

#![allow(missing_docs)]

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_hopf_zeta_recompiler_v14::{
    admit_uor_r4_v14_atlas_export, verify_hopf_zeta_recompiler_v14_admitted,
    HopfZetaAtlasExportCheck, HopfZetaRecompilerVerdict,
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
fn test_r4_72_hopf_zeta_recompiler_v14_r4_72() {
    let root = repo_root();

    let ok = HopfZetaAtlasExportCheck {
        exported_float_ops: 0,
        hopf_spinor_dim: 4,
        signed_orthants: 16,
        storage_angle_preserved: true,
        golden_ratio_poincare_exact: true,
        zeta_ordinate_count: 64,
        icosian_vertex_count: 120,
        c1_s5_distance_buckets: 8,
        context_count_bytes: 524_288,
        max_deployed_context_bytes: 1_048_576,
    };
    assert_eq!(
        admit_uor_r4_v14_atlas_export(ok.clone()),
        HopfZetaRecompilerVerdict::UorR4V14AtlasAdmitted
    );

    let mut ablated = ok.clone();
    ablated.hopf_spinor_dim = 2;
    ablated.storage_angle_preserved = false;
    assert_eq!(
        admit_uor_r4_v14_atlas_export(ablated),
        HopfZetaRecompilerVerdict::RefusedAblatedTwoDimOrUnsignedHopf
    );

    let mut float_exp = ok.clone();
    float_exp.exported_float_ops = 1;
    assert_eq!(
        admit_uor_r4_v14_atlas_export(float_exp),
        HopfZetaRecompilerVerdict::RefusedFloatInExportedAtlas
    );

    let mut over_ctx = ok;
    over_ctx.context_count_bytes = 2_000_000;
    assert_eq!(
        admit_uor_r4_v14_atlas_export(over_ctx),
        HopfZetaRecompilerVerdict::RefusedContextCountExceedsByteLimit
    );

    assert!(verify_hopf_zeta_recompiler_v14_admitted());

    check_lex_module(
        &root,
        "src/R4/HopfZetaRecompilerV14.lex.tex",
        "R4.HopfZetaRecompilerV14",
        &[
            "HopfZetaAtlasExportCheck",
            "HopfZetaRecompilerVerdict",
            "admitUorR4V14AtlasExport",
            "verifyHopfZetaRecompilerV14Admitted",
            "breaksAblatedTurboQuantExportAdmitted",
            "breaksContextCountByteOverflowAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/HopfZetaRecompilerV14Controls.lex.tex",
        "HopfZetaRecompilerV14Controls",
        &[
            "hopfZetaRecompilerV14Admitted",
            "breaksAblatedTurboQuantExportAdmittedCtrl",
            "breaksContextCountByteOverflowAdmittedCtrl",
        ],
    );

    let recomp_rs = std::fs::read_to_string(root.join("xtask/src/recompiler_v14.rs"))
        .expect("xtask/src/recompiler_v14.rs must exist");
    assert!(recomp_rs.contains("pub fn run_recompile_v14_cmd"));
    assert!(recomp_rs.contains("uor-r4.qwen3-0.6b-holo"));
    assert!(recomp_rs.contains("uor-r4.qwen3-1.7b-holo"));
}
