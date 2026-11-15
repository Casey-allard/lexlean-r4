//! Conformance test for R4-44: Direct teacher-to-R4 geometric vector recompilation
//! and multiplier-free serving invariants.

#[path = "../../wasm/src/lexlean_generated.rs"]
mod lexlean_generated;

use lexlean_generated::r4_geometric_recompiler::{
    admit_nibble_lut_step, admit_recompiler_container, map_gqa_kv_head, NibbleLutStepCheck,
    NibbleLutVerdict, RecompilerHeaderCheck, RecompilerHeaderVerdict,
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

fn valid_smol_check() -> RecompilerHeaderCheck {
    RecompilerHeaderCheck {
        container_version: 12,
        layer_count: 30,
        q_head_count: 9,
        kv_head_count: 3,
        head_dim: 64,
        d_model: 576,
        vocab_size: 49152,
        rope_theta: 100_000,
        float_serving_ops: 0,
        dense_matmul_ops: 0,
        softmax_ops: 0,
        turn_memory_entries: 0,
        tied_lm_head: true,
        rms_norm_preserved: true,
    }
}

fn valid_lut_step_check() -> NibbleLutStepCheck {
    NibbleLutStepCheck {
        layer_idx: 0,
        layer_count: 30,
        lut_table_len: 256,
        q14_abs_sum: 8192,
        max_q14_bound: 65536,
        uses_float: false,
    }
}

#[test]
fn test_direct_teacher_geometric_recompiler_r4_44() {
    let root = repo_root();

    check_lex_module(
        &root,
        "src/R4/GeometricRecompiler.lex.tex",
        "R4.GeometricRecompiler",
        &[
            "admitRecompilerContainer",
            "mapGqaKvHead",
            "admitNibbleLutStep",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/GeometricRecompilerControls.lex.tex",
        "GeometricRecompilerControls",
        &["smolContainerAdmitted", "breaksFloatServingAdmitted"],
    );

    assert_eq!(
        admit_recompiler_container(valid_smol_check()),
        RecompilerHeaderVerdict::Admitted
    );

    // GQA 9->3 causal broadcast mapping.
    for q in 0..9 {
        assert_eq!(map_gqa_kv_head(q, 9, 3), Some(q / 3));
    }
    assert_eq!(map_gqa_kv_head(9, 9, 3), None);
    assert_eq!(map_gqa_kv_head(0, 9, 4), None);

    // Nibble-pair 256-entry LUT step.
    assert_eq!(
        admit_nibble_lut_step(valid_lut_step_check()),
        NibbleLutVerdict::StepAdmitted
    );

    // Typed R5 negative refusals.
    assert_eq!(
        admit_recompiler_container(RecompilerHeaderCheck {
            container_version: 11,
            ..valid_smol_check()
        }),
        RecompilerHeaderVerdict::RefusedInvalidVersion
    );
    assert_eq!(
        admit_recompiler_container(RecompilerHeaderCheck {
            float_serving_ops: 1,
            ..valid_smol_check()
        }),
        RecompilerHeaderVerdict::RefusedFloatServingOp
    );
    assert_eq!(
        admit_recompiler_container(RecompilerHeaderCheck {
            dense_matmul_ops: 1,
            ..valid_smol_check()
        }),
        RecompilerHeaderVerdict::RefusedDenseMatmulOp
    );
    assert_eq!(
        admit_recompiler_container(RecompilerHeaderCheck {
            softmax_ops: 1,
            ..valid_smol_check()
        }),
        RecompilerHeaderVerdict::RefusedSoftmaxOp
    );
    assert_eq!(
        admit_recompiler_container(RecompilerHeaderCheck {
            turn_memory_entries: 1,
            ..valid_smol_check()
        }),
        RecompilerHeaderVerdict::RefusedTurnMemoryHijack
    );
    assert_eq!(
        admit_recompiler_container(RecompilerHeaderCheck {
            d_model: 512,
            ..valid_smol_check()
        }),
        RecompilerHeaderVerdict::RefusedGqaDimensionMismatch
    );
    assert_eq!(
        admit_recompiler_container(RecompilerHeaderCheck {
            tied_lm_head: false,
            ..valid_smol_check()
        }),
        RecompilerHeaderVerdict::RefusedMissingNormOrTiedHead
    );
    assert_eq!(
        admit_nibble_lut_step(NibbleLutStepCheck {
            uses_float: true,
            ..valid_lut_step_check()
        }),
        NibbleLutVerdict::RefusedFloatOp
    );
    assert_eq!(
        admit_nibble_lut_step(NibbleLutStepCheck {
            layer_idx: 30,
            ..valid_lut_step_check()
        }),
        NibbleLutVerdict::RefusedInvalidLayer
    );
    assert_eq!(
        admit_nibble_lut_step(NibbleLutStepCheck {
            lut_table_len: 128,
            ..valid_lut_step_check()
        }),
        NibbleLutVerdict::RefusedInvalidLutTable
    );
    assert_eq!(
        admit_nibble_lut_step(NibbleLutStepCheck {
            q14_abs_sum: 70_000,
            ..valid_lut_step_check()
        }),
        NibbleLutVerdict::RefusedQ14Overflow
    );
}
