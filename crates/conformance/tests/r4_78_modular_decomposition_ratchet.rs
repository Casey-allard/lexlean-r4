//! Conformance test for R4-78: Verdant M15 Modular Rust/JS Source Decomposition &
//! Bidirectional Residue Ratchet (`residue <= 200`, `rust_lines <= 11400`, `js_lines <= 12800`).

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_modular_decomposition_ratchet::{
    admit_modular_decomposition_ratchet, verify_modular_decomposition_ratchet_admitted,
    ModularDecompositionVerdict,
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

fn parse_ratchet_u64(raw: &str, key: &str) -> u64 {
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&format!("{key} =")) {
            return rest
                .trim()
                .parse::<u64>()
                .unwrap_or_else(|e| panic!("invalid u64 for {key}: {e}"));
        }
    }
    panic!("missing {key} in model/ratchet.toml");
}

#[test]
fn test_r4_78_modular_decomposition_ratchet_r4_78() {
    let root = repo_root();

    // 1. Formal LexLean R4-78 admission and R5 negative refusals
    assert!(verify_modular_decomposition_ratchet_admitted());
    assert_eq!(
        admit_modular_decomposition_ratchet(180, 9500, 9500, 6, true, 0),
        ModularDecompositionVerdict::ModularDecompositionAdmitted
    );
    assert_eq!(
        admit_modular_decomposition_ratchet(201, 9500, 9500, 6, true, 0),
        ModularDecompositionVerdict::RefusedResidueAbove200
    );
    assert_eq!(
        admit_modular_decomposition_ratchet(180, 11_401, 9500, 6, true, 0),
        ModularDecompositionVerdict::RefusedRustFileExceeds11400Lines
    );
    assert_eq!(
        admit_modular_decomposition_ratchet(180, 9500, 12_801, 6, true, 0),
        ModularDecompositionVerdict::RefusedJsFileExceeds12800Lines
    );
    assert_eq!(
        admit_modular_decomposition_ratchet(180, 9500, 9500, 7, true, 0),
        ModularDecompositionVerdict::RefusedBrokenSixFileBrowserClosure
    );
    assert_eq!(
        admit_modular_decomposition_ratchet(180, 9500, 9500, 6, false, 0),
        ModularDecompositionVerdict::RefusedFloatOrBundleSyncDrift
    );
    assert_eq!(
        admit_modular_decomposition_ratchet(180, 9500, 9500, 6, true, 1),
        ModularDecompositionVerdict::RefusedFloatOrBundleSyncDrift
    );

    // 2. Verify .lex.tex semantic modules exist and declare required symbols
    check_lex_module(
        &root,
        "src/R4/ModularDecompositionRatchet.lex.tex",
        "R4.ModularDecompositionRatchet",
        &[
            "ModularDecompositionVerdict",
            "admitModularDecompositionRatchet",
            "verifyModularDecompositionRatchetAdmitted",
            "breaksResidueAbove200Admitted",
            "breaksRustFileExceeds11400Admitted",
            "breaksJsFileExceeds12800Admitted",
            "breaksBrokenBrowserClosureAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/ModularDecompositionRatchetControls.lex.tex",
        "ModularDecompositionRatchetControls",
        &[
            "modularDecompositionRatchetAdmitted",
            "breaksResidueAbove200AdmittedCtrl",
            "breaksRustFileExceeds11400AdmittedCtrl",
            "breaksJsFileExceeds12800AdmittedCtrl",
            "breaksBrokenBrowserClosureAdmittedCtrl",
        ],
    );

    // 3. Verify tightened model/ratchet.toml ceilings and live lift_register.json residue <= 200
    let ratchet_toml =
        std::fs::read_to_string(root.join("model/ratchet.toml")).expect("read model/ratchet.toml");
    let max_residue = parse_ratchet_u64(&ratchet_toml, "max_residue_functions");
    let max_rust_lines = parse_ratchet_u64(&ratchet_toml, "max_rust_file_lines");
    let max_js_lines = parse_ratchet_u64(&ratchet_toml, "max_js_file_lines");
    let max_float = parse_ratchet_u64(&ratchet_toml, "max_float_serving_ops");

    assert!(
        max_residue <= 200,
        "R4-78 requires max_residue_functions <= 200 in model/ratchet.toml, got {max_residue}"
    );
    assert!(
        max_rust_lines <= 11_400,
        "R4-78 requires max_rust_file_lines <= 11400 (>= 35% reduction from 17545), got {max_rust_lines}"
    );
    assert!(
        max_js_lines <= 12_800,
        "R4-78 requires max_js_file_lines <= 12800 (>= 25% reduction from 17105), got {max_js_lines}"
    );
    assert_eq!(max_float, 0);

    let lift_json: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("model/lift_register.json"))
            .expect("read model/lift_register.json"),
    )
    .expect("parse lift_register.json");
    let live_residue = lift_json["summary"]["residue"]
        .as_u64()
        .expect("summary.residue u64");
    assert!(
        live_residue <= 200,
        "live model/lift_register.json summary.residue must be <= 200, got {live_residue}"
    );

    // 4. Verify modular JS source directories exist and 6-file public/ closure holds
    assert!(
        root.join("crates/wasm/js/bridge").is_dir() && root.join("crates/wasm/js/app").is_dir(),
        "crates/wasm/js/bridge/ and crates/wasm/js/app/ modular JS directories must exist"
    );
    let closure_files = [
        "public/index.html",
        "public/app.js",
        "public/app.css",
        "public/lexlean_r4.js",
        "public/lexlean_r4_bg.wasm",
        "public/provenance.json",
    ];
    for cf in closure_files {
        assert!(root.join(cf).is_file(), "missing browser closure file {cf}");
    }
    assert_eq!(
        admit_modular_decomposition_ratchet(
            max_residue,
            max_rust_lines,
            max_js_lines,
            closure_files.len() as u64,
            true,
            max_float,
        ),
        ModularDecompositionVerdict::ModularDecompositionAdmitted
    );
}
