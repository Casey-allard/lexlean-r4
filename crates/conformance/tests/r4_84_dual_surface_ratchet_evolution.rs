//! Conformance test for R4-84: Dual-Surface Proof + Application Ratchet Co-Evolution,
//! CAP0005 Per-Root Falsifiers, SDK `wasm-bindgen 0.2.122` Parity, and Studio `/ratchet` Spore
//! (`R4.DualSurfaceRatchetEvolution`).

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_dual_surface_ratchet_evolution::{
    admit_dual_surface_generation, verify_dual_surface_ratchet_evolution_admitted,
    DualSurfaceVerdict,
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
fn test_r4_84_dual_surface_ratchet_evolution_r4_84() {
    let root = repo_root();

    // 1. Formal LexLean R4-84 admission and all 5 R5 negative refusals
    assert!(verify_dual_surface_ratchet_evolution_admitted());
    assert_eq!(
        admit_dual_surface_generation(
            2, 2, 1, 1, 2449, 2455, 129, 128, 80, 96, 12_100_000, 14_155_776
        ),
        DualSurfaceVerdict::GenerationPromoted
    );
    assert_eq!(
        admit_dual_surface_generation(
            1, 2, 1, 1, 2449, 2455, 129, 128, 80, 96, 12_100_000, 14_155_776
        ),
        DualSurfaceVerdict::RefusedUnverifiedDecideGate
    );
    assert_eq!(
        admit_dual_surface_generation(
            2, 2, 0, 1, 2449, 2455, 129, 128, 80, 96, 12_100_000, 14_155_776
        ),
        DualSurfaceVerdict::RefusedVacuousFalsifierRoot
    );
    assert_eq!(
        admit_dual_surface_generation(
            2, 2, 1, 1, 2449, 2455, 129, 129, 80, 96, 12_100_000, 14_155_776
        ),
        DualSurfaceVerdict::RefusedStagnantDualSurfaceLift
    );
    assert_eq!(
        admit_dual_surface_generation(
            2, 2, 1, 1, 2449, 2455, 129, 128, 120, 96, 12_100_000, 14_155_776
        ),
        DualSurfaceVerdict::RefusedTelosLossRegression
    );
    assert_eq!(
        admit_dual_surface_generation(
            2, 2, 1, 1, 2449, 2455, 129, 128, 80, 96, 15_000_000, 14_155_776
        ),
        DualSurfaceVerdict::RefusedWasmBudgetExceeded
    );

    // 2. Verify .lex.tex semantic modules exist
    check_lex_module(
        &root,
        "src/R4/DualSurfaceRatchetEvolution.lex.tex",
        "R4.DualSurfaceRatchetEvolution",
        &[
            "DualSurfaceVerdict",
            "admitDualSurfaceGeneration",
            "verifyDualSurfaceRatchetEvolutionAdmitted",
            "breaksUnverifiedDecideGateAdmitted",
            "breaksVacuousFalsifierRootAdmitted",
            "breaksStagnantDualSurfaceLiftAdmitted",
            "breaksTelosLossRegressionAdmitted",
            "breaksWasmBudgetExceededAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/DualSurfaceRatchetEvolutionControls.lex.tex",
        "DualSurfaceRatchetEvolutionControls",
        &[
            "dualSurfaceRatchetEvolutionAdmitted",
            "breaksUnverifiedDecideGateAdmittedCtrl",
            "breaksVacuousFalsifierRootAdmittedCtrl",
            "breaksStagnantDualSurfaceLiftAdmittedCtrl",
            "breaksTelosLossRegressionAdmittedCtrl",
            "breaksWasmBudgetExceededAdmittedCtrl",
        ],
    );

    // 3. Verify dual_surface_ratchet.rs, sovereign_ratchet.rs, Generation >= 1 manifest, and Studio /ratchet
    let dsr_rs = std::fs::read_to_string(root.join("xtask/src/dual_surface_ratchet.rs"))
        .expect("xtask/src/dual_surface_ratchet.rs must exist");
    assert!(
        dsr_rs.contains("pub fn verify_lex_module_falsifiers(")
            && dsr_rs.contains("pub fn step_dual_surface_ratchet_evolution("),
        "dual_surface_ratchet.rs must expose CAP0005 verify_lex_module_falsifiers and step_dual_surface_ratchet_evolution"
    );

    let sov_rs = std::fs::read_to_string(root.join("crates/wasm/src/sovereign_ratchet.rs"))
        .expect("crates/wasm/src/sovereign_ratchet.rs must exist");
    assert!(
        sov_rs.contains("R4SPORE1")
            && sov_rs.contains("pub fn sovereign_ratchet_status_json(")
            && sov_rs.contains("pub fn sovereign_ratchet_step_json("),
        "sovereign_ratchet.rs must implement R4SPORE1 sovereign ratchet status and step exports"
    );

    let gens_toml = std::fs::read_to_string(root.join("model/sovereign_generations.toml"))
        .expect("model/sovereign_generations.toml must exist");
    assert!(
        gens_toml.contains("spec = \"r4/sovereign-generations/1\"")
            && gens_toml.contains("id = \"gen-1\""),
        "model/sovereign_generations.toml must record at least promoted gen-1"
    );

    // Verify Generation 1 synthesized LexLean module exists & residue ceiling dropped below 129
    assert!(
        root.join("src/R4/RatchetGen1.lex.tex").is_file()
            && root
                .join("tests/lexlean/RatchetGen1Controls.lex.tex")
                .is_file(),
        "Autonomous Dual-Surface Ratchet must materialize src/R4/RatchetGen1.lex.tex + Controls"
    );

    let engine_js =
        std::fs::read_to_string(root.join("crates/wasm/js/bridge/01_opfs_webgpu_engine.js"))
            .expect("read 01_opfs_webgpu_engine.js");
    let studio_js =
        std::fs::read_to_string(root.join("crates/wasm/js/app/01_studio_chat_preview.js"))
            .expect("read 01_studio_chat_preview.js");
    assert!(
        engine_js.contains("studio_ratchet_step_json")
            && engine_js.contains("prompt === '/ratchet' || prompt.startsWith('/ratchet ')")
            && studio_js.contains("command: '/ratchet'")
            && studio_js.contains("studio_ratchet_step_json"),
        "Browser worker & Studio main-thread fallback must wire exact /ratchet command to studio_ratchet_step_json"
    );

    // Verify sovereign_ratchet.rs typed R5 SporeCapsuleError & synchronized repository counts
    let lift_raw = std::fs::read_to_string(root.join("model/lift_register.json"))
        .expect("read model/lift_register.json");
    let lift_doc: Value = serde_json::from_str(&lift_raw).expect("parse lift_register.json");
    let want_lifted = format!(
        "pub const ACTIVE_REPO_LIFTED_COUNT: u64 = {};",
        lift_doc["summary"]["lifted"].as_u64().unwrap()
    );
    let want_residue = format!(
        "pub const ACTIVE_REPO_RESIDUE_COUNT: u64 = {};",
        lift_doc["summary"]["residue"].as_u64().unwrap()
    );
    assert!(
        sov_rs.contains("pub enum SporeCapsuleError")
            && sov_rs.contains(&want_lifted)
            && sov_rs.contains(&want_residue),
        "sovereign_ratchet.rs must define typed R5 SporeCapsuleError and stay synchronized with lift_register.json ({want_lifted}, {want_residue})"
    );

    // Verify crates/wasm/Cargo.toml aligns wasm-bindgen with the locked SDK image (0.2.122)
    let wasm_cargo = std::fs::read_to_string(root.join("crates/wasm/Cargo.toml"))
        .expect("read crates/wasm/Cargo.toml");
    assert!(
        wasm_cargo.contains("wasm-bindgen = \"=0.2.122\""),
        "crates/wasm/Cargo.toml must pin wasm-bindgen = \"=0.2.122\" matching prismpm.lock SDK image"
    );
}
