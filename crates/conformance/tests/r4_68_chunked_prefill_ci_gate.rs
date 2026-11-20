//! Conformance test for R4-68: Chunked Async Prompt Prefill, Live-HUD Speculative Drafting,
//! and Single-Pass Deduplicated CI Gate (`R4.ChunkedPrefillCiGate`).

#![allow(missing_docs)]

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_chunked_prefill_ci_gate::{
    admit_live_hud_speculative_budget, admit_prefill_slice, admit_single_pass_ci_gate,
    verify_chunked_prefill_ci_admitted, LiveSpeculativeHudVerdict, PrefillSliceVerdict,
    SinglePassCiGateVerdict,
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
fn test_r4_68_chunked_prefill_ci_gate_r4_68() {
    let root = repo_root();

    assert_eq!(
        admit_prefill_slice(0, 18, 18, 18),
        PrefillSliceVerdict::SyncFastPrefillAdmitted
    );
    assert_eq!(
        admit_prefill_slice(0, 120, 32, 16),
        PrefillSliceVerdict::ChunkedAsyncPrefillAdmitted
    );
    assert_eq!(
        admit_prefill_slice(1, 18, 18, 18),
        PrefillSliceVerdict::RefusedFloatOpInPrefill
    );
    assert_eq!(
        admit_prefill_slice(0, 120, 16, 25),
        PrefillSliceVerdict::RefusedZeroOrOversizedPrefillSlice
    );
    assert_eq!(
        admit_prefill_slice(0, 30, 31, 16),
        PrefillSliceVerdict::RefusedPrefillCursorOverrun
    );

    assert_eq!(
        admit_live_hud_speculative_budget(true, true, 4, true),
        LiveSpeculativeHudVerdict::LiveHudSpeculativeAdmitted
    );
    assert_eq!(
        admit_live_hud_speculative_budget(true, false, 0, true),
        LiveSpeculativeHudVerdict::NoDraftsTerminalAdmitted
    );
    assert_eq!(
        admit_live_hud_speculative_budget(true, true, 0, true),
        LiveSpeculativeHudVerdict::RefusedDisabledDrafterUnderLiveHud
    );
    assert_eq!(
        admit_live_hud_speculative_budget(true, true, 4, false),
        LiveSpeculativeHudVerdict::RefusedMissingHudFramesOnDrafts
    );
    assert_eq!(
        admit_live_hud_speculative_budget(false, true, 6, false),
        LiveSpeculativeHudVerdict::RefusedOversizedSpeculativeBudget
    );

    assert_eq!(
        admit_single_pass_ci_gate(1, 1, 0),
        SinglePassCiGateVerdict::DeduplicatedGateAdmitted
    );
    assert_eq!(
        admit_single_pass_ci_gate(2, 1, 0),
        SinglePassCiGateVerdict::RefusedDuplicateMechanizerOrRatchetRun
    );
    assert_eq!(
        admit_single_pass_ci_gate(1, 1, 8),
        SinglePassCiGateVerdict::RefusedInnerLoopBpeAllocations
    );
    assert!(verify_chunked_prefill_ci_admitted());

    check_lex_module(
        &root,
        "src/R4/ChunkedPrefillCiGate.lex.tex",
        "R4.ChunkedPrefillCiGate",
        &[
            "PrefillSliceVerdict",
            "LiveSpeculativeHudVerdict",
            "SinglePassCiGateVerdict",
            "admitPrefillSlice",
            "admitLiveHudSpeculativeBudget",
            "admitSinglePassCiGate",
            "verifyChunkedPrefillCiAdmitted",
            "breaksOversizedPrefillSliceAdmitted",
            "breaksDisabledDrafterUnderLiveHudAdmitted",
            "breaksDuplicateMechanizerRunAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/ChunkedPrefillCiGateControls.lex.tex",
        "ChunkedPrefillCiGateControls",
        &[
            "chunkedPrefillCiAdmitted",
            "breaksOversizedPrefillSliceAdmittedCtrl",
            "breaksDisabledDrafterUnderLiveHudAdmittedCtrl",
            "breaksDuplicateMechanizerRunAdmittedCtrl",
        ],
    );

    // Verify single-pass synthesis deduplication & chunked prefill in source
    let synth_rs =
        std::fs::read_to_string(root.join("xtask/src/synthesis.rs")).expect("synthesis.rs");
    assert!(
        synth_rs.contains("evaluate_workspace_holon_with_reports"),
        "synthesis.rs must expose evaluate_workspace_holon_with_reports to avoid duplicate mechanizer/ratchet passes"
    );
    let asst_rs =
        std::fs::read_to_string(root.join("crates/wasm/src/assistant.rs")).expect("assistant.rs");
    assert!(
        asst_rs.contains("\"prefilling\""),
        "assistant.rs must emit chunked 'prefilling' status on >24-token prompt tails"
    );
    assert!(
        !asst_rs.contains("if !session.live_hud && need_logits"),
        "assistant.rs must not disable speculative drafting when live_hud is true"
    );
}
