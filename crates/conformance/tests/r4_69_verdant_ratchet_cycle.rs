//! Conformance test for R4-69: Verdant M15 Bidirectional Self-Improvement Ratchet-Cycle,
//! Live Source-Tree SHA-256 Binding, 7-Category Lifecycle, and DR-01/DR-02 Coverage.

#![allow(missing_docs)]

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_verdant_ratchet_cycle::{
    admit_dev_rel_catalog_coverage, admit_verdant_ratchet_cycle, tighten_ceiling_down,
    tighten_floor_up, verify_verdant_ratchet_cycle_admitted, DevRelCatalogVerdict,
    RatchetCycleVerdict, VerdantRatchetCycleCheck,
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
fn test_r4_69_verdant_ratchet_cycle_r4_69() {
    let root = repo_root();

    assert_eq!(tighten_floor_up(2090, 2103), 2103);
    assert_eq!(tighten_floor_up(2103, 2090), 2103);
    assert_eq!(tighten_ceiling_down(355, 341), 341);
    assert_eq!(tighten_ceiling_down(341, 355), 341);

    let base = VerdantRatchetCycleCheck {
        rust_tree_sha256_matches: true,
        js_tree_sha256_matches: true,
        lifecycle_categories: 7,
        min_lifted_floor: 2100,
        curr_lifted: 2103,
        min_verified_modules_floor: 150,
        curr_verified_modules: 162,
        max_residue_ceiling: 341,
        curr_residue: 340,
        max_rust_lines_ceiling: 15000,
        curr_max_rust_lines: 14700,
        max_js_lines_ceiling: 17000,
        curr_max_js_lines: 16030,
        max_wasm_bytes_ceiling: 14_155_776,
        curr_wasm_bytes: 11_700_000,
        classical_choice_count: 0,
        float_serving_ops: 0,
        dense_matmul_ops: 0,
    };
    assert_eq!(
        admit_verdant_ratchet_cycle(base.clone()),
        RatchetCycleVerdict::RatchetCycleAdmitted
    );

    let mut stale_rust = base.clone();
    stale_rust.rust_tree_sha256_matches = false;
    assert_eq!(
        admit_verdant_ratchet_cycle(stale_rust),
        RatchetCycleVerdict::RefusedStaleSourceTreeSha256
    );

    let mut short_cat = base.clone();
    short_cat.lifecycle_categories = 6;
    assert_eq!(
        admit_verdant_ratchet_cycle(short_cat),
        RatchetCycleVerdict::RefusedIncompleteLifecycleCategories
    );

    let mut floor_reg = base.clone();
    floor_reg.curr_lifted = 2099;
    assert_eq!(
        admit_verdant_ratchet_cycle(floor_reg),
        RatchetCycleVerdict::RefusedLiftedFloorRegression
    );

    let mut ceil_inf = base.clone();
    ceil_inf.curr_residue = 342;
    assert_eq!(
        admit_verdant_ratchet_cycle(ceil_inf),
        RatchetCycleVerdict::RefusedWatermarkCeilingInflation
    );

    let mut choice_drift = base;
    choice_drift.classical_choice_count = 1;
    assert_eq!(
        admit_verdant_ratchet_cycle(choice_drift),
        RatchetCycleVerdict::RefusedClassicalChoiceOrOntologyDrift
    );

    assert_eq!(
        admit_dev_rel_catalog_coverage(0, 0),
        DevRelCatalogVerdict::CatalogCoveredAdmitted
    );
    assert_eq!(
        admit_dev_rel_catalog_coverage(1, 0),
        DevRelCatalogVerdict::RefusedUncoveredSubjectRecord
    );
    assert_eq!(
        admit_dev_rel_catalog_coverage(0, 1),
        DevRelCatalogVerdict::RefusedUndeclaredCoverResource
    );
    assert!(verify_verdant_ratchet_cycle_admitted());

    check_lex_module(
        &root,
        "src/R4/VerdantRatchetCycle.lex.tex",
        "R4.VerdantRatchetCycle",
        &[
            "VerdantRatchetCycleCheck",
            "RatchetCycleVerdict",
            "DevRelCatalogVerdict",
            "tightenFloorUp",
            "tightenCeilingDown",
            "admitVerdantRatchetCycle",
            "admitDevRelCatalogCoverage",
            "verifyVerdantRatchetCycleAdmitted",
            "breaksStaleRustSourceTreeAdmitted",
            "breaksResidueCeilingInflationAdmitted",
            "breaksUncoveredDevRelRecordAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/VerdantRatchetCycleControls.lex.tex",
        "VerdantRatchetCycleControls",
        &[
            "verdantRatchetCycleAdmitted",
            "breaksStaleRustSourceTreeAdmittedCtrl",
            "breaksResidueCeilingInflationAdmittedCtrl",
            "breaksUncoveredDevRelRecordAdmittedCtrl",
        ],
    );

    // Verify upgraded model/ratchet.toml & xtask ratchet-cycle implementation
    let ratchet_toml =
        std::fs::read_to_string(root.join("model/ratchet.toml")).expect("model/ratchet.toml");
    assert!(
        ratchet_toml.contains("rust_tree_sha256 = \""),
        "model/ratchet.toml must bind live rust_tree_sha256"
    );
    assert!(
        ratchet_toml.contains("js_tree_sha256 = \""),
        "model/ratchet.toml must bind live js_tree_sha256"
    );
    assert!(
        ratchet_toml.contains("min_lifted_functions = "),
        "model/ratchet.toml must enforce min_lifted_functions floor"
    );
    assert!(
        ratchet_toml.contains("lifecycle_categories = 7"),
        "model/ratchet.toml must declare 7 lifted lifecycle categories"
    );

    let ratchet_rs =
        std::fs::read_to_string(root.join("xtask/src/ratchet.rs")).expect("xtask/src/ratchet.rs");
    assert!(
        ratchet_rs.contains("pub fn run_ratchet_cycle"),
        "xtask/src/ratchet.rs must expose run_ratchet_cycle"
    );
    assert!(
        ratchet_rs.contains("pub fn observer_refine_highest_debt_target"),
        "xtask/src/ratchet.rs must expose observer_refine_highest_debt_target"
    );
}
