//! Conformance test for R4-64: True Resident WebGPU Per-Layer Serving,
//! `HoloTok13` Speculative Decoding & Studio LoRA Lockstep Prompt Shrink.

#![allow(missing_docs)]

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_webgpu_resident_runtime::{
    admit_resident_layer_partition, admit_speculative_tile_step, admit_studio_lora_prompt_shrink,
    max_compact_studio_prompt_tokens, verify_webgpu_resident_runtime_admitted, ResidentSlabVerdict,
    SpeculativeVerifyVerdict, StudioLoraDeployVerdict,
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
fn test_r4_64_webgpu_resident_runtime_r4_64() {
    let root = repo_root();

    // 1. Per-layer <=128 MiB resident WebGPU slab admission & R5 negative gates.
    assert_eq!(
        admit_resident_layer_partition(0, 39_845_888, 134_217_728, 134_217_728),
        ResidentSlabVerdict::ResidentSlabAdmitted
    );
    assert_eq!(
        admit_resident_layer_partition(1, 39_845_888, 134_217_728, 134_217_728),
        ResidentSlabVerdict::RefusedFloatOpInResidentWgsl
    );
    assert_eq!(
        admit_resident_layer_partition(0, 0, 134_217_728, 134_217_728),
        ResidentSlabVerdict::RefusedLayerExceeds128MiB
    );
    assert_eq!(
        admit_resident_layer_partition(0, 150_000_000, 134_217_728, 268_435_456),
        ResidentSlabVerdict::RefusedLayerExceeds128MiB
    );
    assert_eq!(
        admit_resident_layer_partition(0, 39_845_888, 134_217_728, 16_777_216),
        ResidentSlabVerdict::RefusedInsufficientGpuBufferLimits
    );

    // 2. HoloTok13 speculative tile verification 0-LSB equivalence & KV rewind gates.
    assert_eq!(
        admit_speculative_tile_step(5, 32, 0, true),
        SpeculativeVerifyVerdict::SpeculativeExact0LsbAdmitted
    );
    assert_eq!(
        admit_speculative_tile_step(32, 32, 0, true),
        SpeculativeVerifyVerdict::RefusedDraftExceedsTile
    );
    assert_eq!(
        admit_speculative_tile_step(5, 32, 1, true),
        SpeculativeVerifyVerdict::RefusedSpeculativeParityDrift
    );
    assert_eq!(
        admit_speculative_tile_step(5, 32, 0, false),
        SpeculativeVerifyVerdict::RefusedKvRewindMismatch
    );

    // 3. Studio LoRA UORR4V13 lockstep <=40-token prompt shrink & negative gates.
    assert_eq!(max_compact_studio_prompt_tokens(), 40);
    assert_eq!(
        admit_studio_lora_prompt_shrink(0, true, true, 34),
        StudioLoraDeployVerdict::StudioLoraUorV13Admitted
    );
    assert_eq!(
        admit_studio_lora_prompt_shrink(0, true, true, 40),
        StudioLoraDeployVerdict::StudioLoraUorV13Admitted
    );
    assert_eq!(
        admit_studio_lora_prompt_shrink(1, true, true, 34),
        StudioLoraDeployVerdict::RefusedFloatOpInLoraExport
    );
    assert_eq!(
        admit_studio_lora_prompt_shrink(0, false, true, 34),
        StudioLoraDeployVerdict::RefusedStitchParityMismatch
    );
    assert_eq!(
        admit_studio_lora_prompt_shrink(0, true, false, 34),
        StudioLoraDeployVerdict::RefusedPromptShrinkWithoutFineTunedManifest
    );
    assert_eq!(
        admit_studio_lora_prompt_shrink(0, true, true, 41),
        StudioLoraDeployVerdict::RefusedPromptExceedsCompactBudget
    );
    assert!(verify_webgpu_resident_runtime_admitted());

    // 4. Verify formal .lex.tex modules and theorem controls.
    check_lex_module(
        &root,
        "src/R4/WebGpuResidentRuntime.lex.tex",
        "R4.WebGpuResidentRuntime",
        &[
            "ResidentSlabVerdict",
            "SpeculativeVerifyVerdict",
            "StudioLoraDeployVerdict",
            "maxCompactStudioPromptTokens",
            "admitResidentLayerPartition",
            "admitSpeculativeTileStep",
            "admitStudioLoraPromptShrink",
            "verifyWebGpuResidentRuntimeAdmitted",
            "breaksOver128MiBLayerAdmitted",
            "breaksSpeculativeLsbDriftAdmitted",
            "breaksUnpinnedPromptShrinkAdmitted",
            "breaksFloatLoraExportAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/WebGpuResidentRuntimeControls.lex.tex",
        "WebGpuResidentRuntimeControls",
        &[
            "webgpuResidentRuntimeAdmitted",
            "breaksOver128MiBLayerAdmittedCtrl",
            "breaksSpeculativeLsbDriftAdmittedCtrl",
            "breaksUnpinnedPromptShrinkAdmittedCtrl",
            "breaksFloatLoraExportAdmittedCtrl",
        ],
    );
}
