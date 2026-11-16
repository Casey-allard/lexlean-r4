//! Conformance test for R4-52: Pure-from-scratch D85 Q31 SpiralCore/FBS neural lane.

#[path = "../../wasm/src/lexlean_generated.rs"]
mod lexlean_generated;

use lexlean_generated::r4_native_q31_train::{
    admit_native_q31_lane, step_d85_spin_orbit, NativeQ31LaneCheck, NativeQ31Verdict,
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

fn valid_q31_check() -> NativeQ31LaneCheck {
    NativeQ31LaneCheck {
        magic_q31_valid: true,
        d_model: 256,
        n_heads: 4,
        n_layers: 4,
        vocab_size: 4096,
        orbital_states: 64,
        e8_root_count: 16,
        spin_dimension: 8,
        sha256_match: true,
        blake3_match: true,
        cuda_logit_checksum: 424_242,
        wasm_logit_checksum: 424_242,
        float_ops_in_step: 0,
    }
}

#[test]
fn test_r4_native_q31_train_r4_52() {
    let root = repo_root();

    check_lex_module(
        &root,
        "src/R4/NativeQ31Train.lex.tex",
        "R4.NativeQ31Train",
        &[
            "admitNativeQ31Lane",
            "stepD85SpinOrbit",
            "verifyNativeQ31D85Admitted",
            "breaksCudaWasmParityDrift",
            "breaksInvalidQ31OrbitalCount",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/NativeQ31TrainControls.lex.tex",
        "NativeQ31TrainControls",
        &[
            "nativeQ31D85Admitted",
            "breaksCudaWasmParityDrift",
            "breaksInvalidQ31OrbitalCount",
            "spinOrbitWrapsCl06",
        ],
    );

    assert_eq!(
        admit_native_q31_lane(valid_q31_check()),
        NativeQ31Verdict::Q31LaneAdmitted
    );
    assert_eq!(
        admit_native_q31_lane(NativeQ31LaneCheck {
            magic_q31_valid: false,
            ..valid_q31_check()
        }),
        NativeQ31Verdict::RefusedInvalidQ31Magic
    );
    assert_eq!(
        admit_native_q31_lane(NativeQ31LaneCheck {
            float_ops_in_step: 1,
            ..valid_q31_check()
        }),
        NativeQ31Verdict::RefusedNonIntegerTrainingOp
    );
    assert_eq!(
        admit_native_q31_lane(NativeQ31LaneCheck {
            sha256_match: false,
            ..valid_q31_check()
        }),
        NativeQ31Verdict::RefusedQ31ChecksumMismatch
    );
    assert_eq!(
        admit_native_q31_lane(NativeQ31LaneCheck {
            wasm_logit_checksum: 999,
            ..valid_q31_check()
        }),
        NativeQ31Verdict::RefusedCudaWasmParityDrift
    );
    assert_eq!(
        admit_native_q31_lane(NativeQ31LaneCheck {
            orbital_states: 32,
            ..valid_q31_check()
        }),
        NativeQ31Verdict::RefusedOutOfDomainQ31Config
    );
    assert_eq!(step_d85_spin_orbit(60, 10, 64), 6);

    // Runtime & CUDA/Wasm parity receipt checks (R3 fails until Step 3 completes).
    let lane_rs = std::fs::read_to_string(root.join("crates/wasm/src/native_q31_lane.rs"))
        .expect("crates/wasm/src/native_q31_lane.rs must exist");
    assert!(lane_rs.contains("UORQ31V1"));
    assert!(lane_rs.contains("q31_hamilton_train_step"));

    let receipt_str = std::fs::read_to_string(root.join("gpu/rtx4090_native_q31_d85_receipt.json"))
        .expect("gpu/rtx4090_native_q31_d85_receipt.json must exist");
    let receipt: Value = serde_json::from_str(&receipt_str).expect("valid q31 receipt json");
    assert_eq!(receipt["float_ops"].as_u64(), Some(0));
    assert_eq!(
        receipt["cuda_logit_checksum"],
        receipt["wasm_logit_checksum"]
    );
}
