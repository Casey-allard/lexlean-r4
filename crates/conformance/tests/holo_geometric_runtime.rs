//! Conformance test for R4-51: Pure-integer Holo UORR4V13 geometric runtime & Worker RPC.

#[path = "../../wasm/src/lexlean_generated.rs"]
mod lexlean_generated;

use lexlean_generated::r4_holo_geometric_runtime::{
    admit_holo_geometric_runtime, admit_parity_evaluation, compute_shortlist_budget,
    AttentionOrderCheck, HoloRuntimeVerdict, ParityCheck, ParityVerdict,
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

fn valid_qwen3_06b_check() -> AttentionOrderCheck {
    AttentionOrderCheck {
        hidden_dim: 1024,
        num_q_heads: 16,
        num_kv_heads: 8,
        head_dim: 128,
        group_size: 32,
        q_norm_before_rope: true,
        k_norm_before_rope: true,
        float_ops_count: 0,
        dense_matmul_ops_count: 0,
        worker_rpc_dispatched: true,
        main_thread_wasm_call: false,
        hamming_shortlist_k: 4096,
        vocab_size: 151_936,
    }
}

#[test]
fn test_r4_holo_geometric_runtime_r4_51() {
    let root = repo_root();

    check_lex_module(
        &root,
        "src/R4/HoloGeometricRuntime.lex.tex",
        "R4.HoloGeometricRuntime",
        &[
            "admitHoloGeometricRuntime",
            "admitParityEvaluation",
            "computeShortlistBudget",
            "verifyQwen306bRuntimeAdmitted",
            "breaksMainThreadWasmDispatch",
            "breaksRoPEBeforeQkNorm",
            "breaksFloatServingOp",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/HoloGeometricRuntimeControls.lex.tex",
        "HoloGeometricRuntimeControls",
        &[
            "qwen306bRuntimeAdmitted",
            "breaksMainThreadWasmDispatch",
            "breaksRoPEBeforeQkNorm",
            "breaksFloatServingOp",
        ],
    );

    assert_eq!(
        admit_holo_geometric_runtime(valid_qwen3_06b_check()),
        HoloRuntimeVerdict::RuntimeAdmitted
    );
    assert_eq!(
        admit_holo_geometric_runtime(AttentionOrderCheck {
            main_thread_wasm_call: true,
            ..valid_qwen3_06b_check()
        }),
        HoloRuntimeVerdict::RefusedMainThreadWasmDispatch
    );
    assert_eq!(
        admit_holo_geometric_runtime(AttentionOrderCheck {
            q_norm_before_rope: false,
            ..valid_qwen3_06b_check()
        }),
        HoloRuntimeVerdict::RefusedRoPEBeforeQkNorm
    );
    assert_eq!(
        admit_holo_geometric_runtime(AttentionOrderCheck {
            float_ops_count: 1,
            ..valid_qwen3_06b_check()
        }),
        HoloRuntimeVerdict::RefusedFloatOrMatmulOp
    );
    assert_eq!(
        admit_holo_geometric_runtime(AttentionOrderCheck {
            head_dim: 64,
            ..valid_qwen3_06b_check()
        }),
        HoloRuntimeVerdict::RefusedHeadDimMisaligned
    );
    assert_eq!(
        admit_holo_geometric_runtime(AttentionOrderCheck {
            hamming_shortlist_k: 0,
            ..valid_qwen3_06b_check()
        }),
        HoloRuntimeVerdict::RefusedInvalidShortlistBounds
    );
    assert_eq!(compute_shortlist_budget(4096, 512, 151_936), 4608);

    assert_eq!(
        admit_parity_evaluation(ParityCheck {
            mean_kl_milli: 35,
            max_kl_milli: 50,
            top1_percent: 93,
            min_top1_percent: 91,
            verified_out_of_domain_prompts: 12,
            required_prompts: 12,
        }),
        ParityVerdict::ParityAdmitted
    );
    assert_eq!(
        admit_parity_evaluation(ParityCheck {
            mean_kl_milli: 65,
            max_kl_milli: 50,
            top1_percent: 93,
            min_top1_percent: 91,
            verified_out_of_domain_prompts: 12,
            required_prompts: 12,
        }),
        ParityVerdict::RefusedKlDivergenceExceeded
    );

    // Runtime & RTX 4090 OOD parity receipts (R3 fails until Steps 2 and 4 complete).
    let runtime_rs =
        std::fs::read_to_string(root.join("crates/wasm/src/holo_geometric_runtime.rs"))
            .expect("crates/wasm/src/holo_geometric_runtime.rs must exist");
    assert!(runtime_rs.contains("q_norm_before_rope") || runtime_rs.contains("q_norm"));
    assert!(runtime_rs.contains("simd128"));

    for rel in [
        "gpu/rtx4090_holo_v13_qwen3_0_6b_receipt.json",
        "gpu/rtx4090_holo_v13_qwen3_1_7b_receipt.json",
    ] {
        let receipt_str = std::fs::read_to_string(root.join(rel))
            .unwrap_or_else(|e| panic!("missing {rel}: {e}"));
        let receipt: Value = serde_json::from_str(&receipt_str).expect("valid receipt json");
        assert_eq!(receipt["float_ops"].as_u64(), Some(0));
        assert_eq!(receipt["dense_matmul_ops"].as_u64(), Some(0));
        assert!(
            receipt["mean_kl_nats"]
                .as_f64()
                .is_some_and(|kl| kl <= 0.05),
            "{rel} mean_kl_nats must be <= 0.05"
        );
        assert!(
            receipt["top1_parity_pct"]
                .as_f64()
                .is_some_and(|p| p >= 91.0),
            "{rel} top1_parity_pct must be >= 91.0"
        );
        assert!(
            receipt["top5_parity_pct"]
                .as_f64()
                .is_some_and(|p| p >= 99.0),
            "{rel} top5_parity_pct must be >= 99.0"
        );
    }

    let app_js =
        std::fs::read_to_string(root.join("public/app.js")).expect("public/app.js must exist");
    assert!(!app_js.contains("uor-r4.seed-fast"));
    assert!(!app_js.contains("uor-r4.lut-symbolic"));
    assert!(!app_js.contains("uor-r4.recompiled-v12-agent"));
}
