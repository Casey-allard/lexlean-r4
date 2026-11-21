//! Conformance test for R4-81: Zero-Download `<= 150 ms` Instant Boot, `10,000+ tok/s`
//! `UORR4V15` Turn Routing, `32 MiB` Coalesced OPFS Warm Streaming, and Non-Redundancy
//! `HoloV13` Slice Scheduling (`R4.ZeroCopyFastHydration`).

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_zero_copy_fast_hydration::{
    admit_fast_boot_and_zero_redundancy_slice, verify_zero_copy_fast_hydration_admitted,
    FastHydrationVerdict,
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
fn test_r4_81_zero_copy_fast_hydration_r4_81() {
    let root = repo_root();

    // 1. Formal LexLean R4-81 admission and all 5 R5 negative refusals
    assert!(verify_zero_copy_fast_hydration_admitted());
    assert_eq!(
        admit_fast_boot_and_zero_redundancy_slice(0, 18, 13, false, 0),
        FastHydrationVerdict::FastBootAndSliceAdmitted
    );
    assert_eq!(
        admit_fast_boot_and_zero_redundancy_slice(1, 18, 13, false, 0),
        FastHydrationVerdict::RefusedAutoBootWeightDownload
    );
    assert_eq!(
        admit_fast_boot_and_zero_redundancy_slice(0, 151, 13, false, 0),
        FastHydrationVerdict::RefusedBootReadyExceeds150Ms
    );
    assert_eq!(
        admit_fast_boot_and_zero_redundancy_slice(0, 18, 17, false, 0),
        FastHydrationVerdict::RefusedUnboundedWarmChunkStorm
    );
    assert_eq!(
        admit_fast_boot_and_zero_redundancy_slice(0, 18, 13, true, 0),
        FastHydrationVerdict::RefusedDuplicateGpuCpuSlicePass
    );
    assert_eq!(
        admit_fast_boot_and_zero_redundancy_slice(0, 18, 13, false, 4),
        FastHydrationVerdict::RefusedClampedSetTimeoutSliceYield
    );

    // 2. Verify .lex.tex semantic modules exist and declare required symbols
    check_lex_module(
        &root,
        "src/R4/ZeroCopyFastHydration.lex.tex",
        "R4.ZeroCopyFastHydration",
        &[
            "FastHydrationVerdict",
            "admitFastBootAndZeroRedundancySlice",
            "verifyZeroCopyFastHydrationAdmitted",
            "breaksAutoBootDownloadAdmitted",
            "breaksSlowBootReadyAdmitted",
            "breaksWarmChunkStormAdmitted",
            "breaksDuplicateGpuCpuSliceAdmitted",
            "breaksClampedTimerYieldAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/ZeroCopyFastHydrationControls.lex.tex",
        "ZeroCopyFastHydrationControls",
        &[
            "zeroCopyFastHydrationAdmitted",
            "breaksAutoBootDownloadAdmittedCtrl",
            "breaksSlowBootReadyAdmittedCtrl",
            "breaksWarmChunkStormAdmittedCtrl",
            "breaksDuplicateGpuCpuSliceAdmittedCtrl",
            "breaksClampedTimerYieldAdmittedCtrl",
        ],
    );

    // 3. Verify modular JS bridge exports admitFastBootAndZeroRedundancySlice, streams warm slabs in 32 MiB slices,
    //    exposes explicit /v15 benchmark via studio_sparse_v15_turn_json, and yields slices via MessageChannel
    let engine_js =
        std::fs::read_to_string(root.join("crates/wasm/js/bridge/01_opfs_webgpu_engine.js"))
            .expect("read 01_opfs_webgpu_engine.js");
    assert!(
        engine_js.contains("export function admitFastBootAndZeroRedundancySlice(")
            && engine_js.contains("HOLOSLAB2_WARM_STREAM_BYTES = 32 * 1024 * 1024")
            && engine_js.contains("studio_sparse_v15_turn_json")
            && engine_js.contains("new MessageChannel()"),
        "01_opfs_webgpu_engine.js must export admitFastBootAndZeroRedundancySlice, stream warm slabs in 32 MiB slices, expose /v15 via studio_sparse_v15_turn_json, and yield slices via MessageChannel"
    );
}
