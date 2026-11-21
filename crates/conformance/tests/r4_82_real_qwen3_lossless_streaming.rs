//! Conformance test for R4-82: 100% Real `Qwen3-0.6B` / `1.7B` Lossless Streaming,
//! Deferred Idle Prefix Prewarming, Instant Token-1 TTFT Flush, and Zero Canned Prose
//! (`R4.RealQwen3LosslessStreaming`).

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_real_qwen3_lossless_streaming::{
    admit_real_qwen3_lossless_turn, verify_real_qwen3_lossless_streaming_admitted,
    RealQwen3StreamingVerdict,
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
fn test_r4_82_real_qwen3_lossless_streaming_r4_82() {
    let root = repo_root();

    // 1. Formal LexLean R4-82 admission and all 4 R5 negative refusals
    assert!(verify_real_qwen3_lossless_streaming_admitted());
    assert_eq!(
        admit_real_qwen3_lossless_turn(false, false, true, false),
        RealQwen3StreamingVerdict::RealQwen3LosslessTurnAdmitted
    );
    assert_eq!(
        admit_real_qwen3_lossless_turn(true, false, true, false),
        RealQwen3StreamingVerdict::RefusedPreplannedCannedProseOnQwenLane
    );
    assert_eq!(
        admit_real_qwen3_lossless_turn(false, true, true, false),
        RealQwen3StreamingVerdict::RefusedBlockingSyncPrefixPrewarmOnMount
    );
    assert_eq!(
        admit_real_qwen3_lossless_turn(false, false, false, false),
        RealQwen3StreamingVerdict::RefusedDelayedMultiTokenFirstFlush
    );
    assert_eq!(
        admit_real_qwen3_lossless_turn(false, false, true, true),
        RealQwen3StreamingVerdict::RefusedColdBadgeOverwriteAfterReady
    );

    // 2. Verify .lex.tex semantic modules exist and declare required symbols
    check_lex_module(
        &root,
        "src/R4/RealQwen3LosslessStreaming.lex.tex",
        "R4.RealQwen3LosslessStreaming",
        &[
            "RealQwen3StreamingVerdict",
            "admitRealQwen3LosslessTurn",
            "verifyRealQwen3LosslessStreamingAdmitted",
            "breaksCannedProseOnQwenLaneAdmitted",
            "breaksSyncPrefixPrewarmBlockingAdmitted",
            "breaksDelayedFirstTokenFlushAdmitted",
            "breaksColdBadgeOverwriteAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/RealQwen3LosslessStreamingControls.lex.tex",
        "RealQwen3LosslessStreamingControls",
        &[
            "realQwen3LosslessStreamingAdmitted",
            "breaksCannedProseOnQwenLaneAdmittedCtrl",
            "breaksSyncPrefixPrewarmBlockingAdmittedCtrl",
            "breaksDelayedFirstTokenFlushAdmittedCtrl",
            "breaksColdBadgeOverwriteAdmittedCtrl",
        ],
    );

    // 3. Verify 01_opfs_webgpu_engine.js NEVER intercepts Qwen3 turns with useSparseBasinV15,
    //    exports admitRealQwen3LosslessTurn, and enables instant_ttft.
    let engine_js =
        std::fs::read_to_string(root.join("crates/wasm/js/bridge/01_opfs_webgpu_engine.js"))
            .expect("read 01_opfs_webgpu_engine.js");
    assert!(
        !engine_js.contains("useSparseBasinV15"),
        "01_opfs_webgpu_engine.js must NEVER intercept uor-r4.qwen3-0.6b-holo with useSparseBasinV15 preplanned passages"
    );
    assert!(
        engine_js.contains("export function admitRealQwen3LosslessTurn(")
            && engine_js.contains("beginPayload.instant_ttft =")
            && engine_js.contains("studio_prewarm_holo_prefix_idle"),
        "01_opfs_webgpu_engine.js must export admitRealQwen3LosslessTurn, pass instant_ttft, and schedule idle prefix prewarm"
    );

    // 4. Verify 02_rpc_studio_worker.js auto-hydrates real HoloV13 on boot
    let worker_js =
        std::fs::read_to_string(root.join("crates/wasm/js/bridge/02_rpc_studio_worker.js"))
            .expect("read 02_rpc_studio_worker.js");
    assert!(
        worker_js.contains("selectOptimalInitialHoloModel().then((plan) =>")
            && !worker_js.contains("SparseBasinV15_10k"),
        "02_rpc_studio_worker.js must auto-mount real Qwen3 HoloV13 on boot rather than emitting fake SparseBasinV15_10k mounted status"
    );

    // 5. Verify lib.rs exports studio_prewarm_holo_prefix_idle and assistant.rs supports instant_ttft
    let lib_rs = std::fs::read_to_string(root.join("crates/wasm/src/lib.rs")).expect("read lib.rs");
    let asst_rs = std::fs::read_to_string(root.join("crates/wasm/src/assistant.rs"))
        .expect("read assistant.rs");
    assert!(
        lib_rs.contains("pub fn studio_prewarm_holo_prefix_idle("),
        "lib.rs must export studio_prewarm_holo_prefix_idle"
    );
    assert!(
        asst_rs.contains("pub instant_ttft: bool"),
        "assistant.rs must implement instant_ttft Token-1 immediate slice flush"
    );
}
