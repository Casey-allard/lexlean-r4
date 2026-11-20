//! Conformance test for R4-66: Fast <= 48 MiB Streaming Pack Hydration,
//! 404 Mirror Pruning, Queue-Not-Downgrade Turn 1 & Single-Truth UI.

#![allow(missing_docs)]

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_fast_pack_hydration::{
    admit_pack_hydration_policy, admit_single_truth_badge, max_git_safe_pack_bytes,
    verify_fast_pack_hydration_admitted, PackHydrationVerdict, SingleTruthBadgeVerdict,
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
fn test_r4_66_fast_pack_hydration_r4_66() {
    let root = repo_root();

    assert_eq!(max_git_safe_pack_bytes(), 50_331_648);
    assert_eq!(
        admit_pack_hydration_policy(50_331_648, true, false, false),
        PackHydrationVerdict::StreamingPackAdmitted
    );
    assert_eq!(
        admit_pack_hydration_policy(0, false, false, false),
        PackHydrationVerdict::PieceFallbackAdmitted
    );
    assert_eq!(
        admit_pack_hydration_policy(50_331_649, true, false, false),
        PackHydrationVerdict::RefusedPackExceeds50MiB
    );
    assert_eq!(
        admit_pack_hydration_policy(50_331_648, true, true, false),
        PackHydrationVerdict::RefusedPiece404MirrorRetry
    );
    assert_eq!(
        admit_pack_hydration_policy(50_331_648, true, false, true),
        PackHydrationVerdict::RefusedSilentEmergencyDowngrade
    );

    assert_eq!(
        admit_single_truth_badge(false, false),
        SingleTruthBadgeVerdict::SingleTruthBadgeAdmitted
    );
    assert_eq!(
        admit_single_truth_badge(true, true),
        SingleTruthBadgeVerdict::SingleTruthBadgeAdmitted
    );
    assert_eq!(
        admit_single_truth_badge(false, true),
        SingleTruthBadgeVerdict::RefusedEmergencyBadgeOnQwenSession
    );
    assert!(verify_fast_pack_hydration_admitted());

    check_lex_module(
        &root,
        "src/R4/FastPackHydration.lex.tex",
        "R4.FastPackHydration",
        &[
            "PackHydrationVerdict",
            "SingleTruthBadgeVerdict",
            "maxGitSafePackBytes",
            "admitPackHydrationPolicy",
            "admitSingleTruthBadge",
            "verifyFastPackHydrationAdmitted",
            "breaksOversizedPackAdmitted",
            "breaks404MirrorRetryAdmitted",
            "breaksEmergencyBadgeMismatchAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/FastPackHydrationControls.lex.tex",
        "FastPackHydrationControls",
        &[
            "fastPackHydrationAdmitted",
            "breaksOversizedPackAdmittedCtrl",
            "breaks404MirrorRetryAdmittedCtrl",
            "breaksEmergencyBadgeMismatchAdmittedCtrl",
        ],
    );
}
