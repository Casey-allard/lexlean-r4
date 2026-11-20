//! Conformance test for R4-67: MVM Polyglot Toolchain Completion
//! (`cargo` / `rustc` AST execution, `npm` lifecycle subcommands, C99 `<stdlib.h>`).

#![allow(missing_docs)]

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_mvm_polyglot_toolchain::{
    admit_c99_stdlib_brk_align, admit_cargo_terminal_subcommand, admit_npm_terminal_subcommand,
    verify_mvm_polyglot_toolchain_admitted, PolyglotToolchainVerdict,
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
fn test_r4_67_mvm_polyglot_toolchain_r4_67() {
    let root = repo_root();

    assert_eq!(
        admit_cargo_terminal_subcommand(true, false),
        PolyglotToolchainVerdict::CargoAstToolchainAdmitted
    );
    assert_eq!(
        admit_cargo_terminal_subcommand(false, false),
        PolyglotToolchainVerdict::RefusedUnsupportedToolchainSubcommand
    );
    assert_eq!(
        admit_cargo_terminal_subcommand(true, true),
        PolyglotToolchainVerdict::RefusedProtectedPathToolchainMutation
    );

    assert_eq!(
        admit_npm_terminal_subcommand(true, false),
        PolyglotToolchainVerdict::NpmSubcommandAdmitted
    );
    assert_eq!(
        admit_npm_terminal_subcommand(false, false),
        PolyglotToolchainVerdict::RefusedUnsupportedToolchainSubcommand
    );
    assert_eq!(
        admit_npm_terminal_subcommand(true, true),
        PolyglotToolchainVerdict::RefusedProtectedPathToolchainMutation
    );

    assert_eq!(
        admit_c99_stdlib_brk_align(0, false),
        PolyglotToolchainVerdict::C99StdlibHeapAdmitted
    );
    assert_eq!(
        admit_c99_stdlib_brk_align(4, false),
        PolyglotToolchainVerdict::RefusedUnalignedHeapBreak
    );
    assert_eq!(
        admit_c99_stdlib_brk_align(0, true),
        PolyglotToolchainVerdict::RefusedProtectedPathToolchainMutation
    );
    assert!(verify_mvm_polyglot_toolchain_admitted());

    check_lex_module(
        &root,
        "src/R4/MvmPolyglotToolchain.lex.tex",
        "R4.MvmPolyglotToolchain",
        &[
            "PolyglotToolchainVerdict",
            "admitCargoTerminalSubcommand",
            "admitNpmTerminalSubcommand",
            "admitC99StdlibBrkAlign",
            "verifyMvmPolyglotToolchainAdmitted",
            "breaksProtectedCargoMutationAdmitted",
            "breaksUnsupportedNpmSubcommandAdmitted",
            "breaksUnalignedHeapBreakAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/MvmPolyglotToolchainControls.lex.tex",
        "MvmPolyglotToolchainControls",
        &[
            "mvmPolyglotToolchainAdmitted",
            "breaksProtectedCargoMutationAdmittedCtrl",
            "breaksUnsupportedNpmSubcommandAdmittedCtrl",
            "breaksUnalignedHeapBreakAdmittedCtrl",
        ],
    );
}
