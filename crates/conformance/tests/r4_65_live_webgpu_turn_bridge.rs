//! Conformance test for R4-65: Live Bound WebGPU Turn Bridge,
//! COEP Credentialless Isolation & Adaptive Speculative Drafter.

#![allow(missing_docs)]

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_live_webgpu_turn_bridge::{
    admit_adaptive_speculative_budget, admit_coi_credentialless_mode, admit_live_gpu_bound_step,
    cpu_miss_throttle_ema_permille, verify_live_webgpu_turn_bridge_admitted,
    AdaptiveDrafterVerdict, CoiIsolationVerdict, LiveGpuStepVerdict,
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
fn test_r4_65_live_webgpu_turn_bridge_r4_65() {
    let root = repo_root();

    assert_eq!(
        admit_live_gpu_bound_step(0, true, 197, 197, 0),
        LiveGpuStepVerdict::LiveGpuBoundStepAdmitted
    );
    assert_eq!(
        admit_live_gpu_bound_step(0, false, 0, 0, 0),
        LiveGpuStepVerdict::FallbackWasmSimdAdmitted
    );
    assert_eq!(
        admit_live_gpu_bound_step(1, true, 197, 197, 0),
        LiveGpuStepVerdict::RefusedFloatOpInGpuBridge
    );
    assert_eq!(
        admit_live_gpu_bound_step(0, true, 0, 0, 0),
        LiveGpuStepVerdict::RefusedUnboundComputePass
    );
    assert_eq!(
        admit_live_gpu_bound_step(0, true, 197, 196, 0),
        LiveGpuStepVerdict::RefusedUnboundComputePass
    );
    assert_eq!(
        admit_live_gpu_bound_step(0, true, 197, 197, 1),
        LiveGpuStepVerdict::RefusedGpuCpuLogitDrift
    );

    assert_eq!(
        admit_coi_credentialless_mode(true, false, 1),
        CoiIsolationVerdict::CrossOriginCredentiallessAdmitted
    );
    assert_eq!(
        admit_coi_credentialless_mode(false, false, 0),
        CoiIsolationVerdict::SingleWorkerFallbackAdmitted
    );
    assert_eq!(
        admit_coi_credentialless_mode(true, false, 2),
        CoiIsolationVerdict::SingleWorkerFallbackAdmitted
    );
    assert_eq!(
        admit_coi_credentialless_mode(true, true, 1),
        CoiIsolationVerdict::RefusedRequireCorpBreaksMirrors
    );

    assert_eq!(cpu_miss_throttle_ema_permille(), 250);
    assert_eq!(
        admit_adaptive_speculative_budget(true, 100, 4, 4),
        AdaptiveDrafterVerdict::AdaptiveTileAdmitted
    );
    assert_eq!(
        admit_adaptive_speculative_budget(false, 350, 4, 4),
        AdaptiveDrafterVerdict::AdaptiveTileAdmitted
    );
    assert_eq!(
        admit_adaptive_speculative_budget(false, 120, 1, 4),
        AdaptiveDrafterVerdict::ThrottledCpuDraftsAdmitted
    );
    assert_eq!(
        admit_adaptive_speculative_budget(false, 120, 2, 4),
        AdaptiveDrafterVerdict::RefusedUnthrottledCpuMissStorm
    );
    assert_eq!(
        admit_adaptive_speculative_budget(true, 800, 5, 4),
        AdaptiveDrafterVerdict::RefusedUnthrottledCpuMissStorm
    );
    assert!(verify_live_webgpu_turn_bridge_admitted());

    check_lex_module(
        &root,
        "src/R4/LiveWebGpuTurnBridge.lex.tex",
        "R4.LiveWebGpuTurnBridge",
        &[
            "LiveGpuStepVerdict",
            "CoiIsolationVerdict",
            "AdaptiveDrafterVerdict",
            "cpuMissThrottleEmaPermille",
            "admitLiveGpuBoundStep",
            "admitCoiCredentiallessMode",
            "admitAdaptiveSpeculativeBudget",
            "verifyLiveWebGpuTurnBridgeAdmitted",
            "breaksUnboundGpuPassAdmitted",
            "breaksRequireCorpAdmitted",
            "breaksCpuMissStormAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/LiveWebGpuTurnBridgeControls.lex.tex",
        "LiveWebGpuTurnBridgeControls",
        &[
            "liveWebGpuTurnBridgeAdmitted",
            "breaksUnboundGpuPassAdmittedCtrl",
            "breaksRequireCorpAdmittedCtrl",
            "breaksCpuMissStormAdmittedCtrl",
        ],
    );
}
