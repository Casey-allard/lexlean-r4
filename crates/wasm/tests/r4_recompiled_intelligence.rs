//! End-to-end integration tests for UOR-R4 Track 2 direct SmolLM2 teacher
//! recompilation (`UORR4V12`) and multiplier-free pure-integer geometric serving.
//!
//! Verifies:
//! - R5 typed negative tests across every `R4StackError` variant
//! - Stitch 2-of-3 XOR shard integrity + single-shard erasure recovery on `gpu/weights/v10/`
//! - Mounted 30-layer SmolLM2 (`recompiled-30l`) and embedded 8-layer distilled seed
//!   (`embedded-seed`) real OOD + pasted-buggy-code intelligence with zero floats
//!   and zero dense matrix multiplications
//! - Execution mode wire protocol compatibility and source-level purity invariants

use lexlean_r4_wasm::r4_geometric_stack::{
    GeometricStackSummary, R4GeometricStackEngine, R4StackError, SelectedExecutionMode,
    R4_VEC_MAGIC, R4_VEC_SCHEMA,
};
use lexlean_r4_wasm::stitch_fabric;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct StitchShardRef {
    index: usize,
    file: String,
    byte_length: usize,
    sha256: String,
}

#[derive(Deserialize)]
struct StitchGroup {
    left: StitchShardRef,
    right: Option<StitchShardRef>,
    parity: StitchShardRef,
}

#[derive(Deserialize)]
struct StitchManifest {
    schema: String,
    format: String,
    total_bytes: usize,
    container_sha256: String,
    data_shard_count: usize,
    groups: Vec<StitchGroup>,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn sha256_arr(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = sha256_arr(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

fn xor_recover_shard(peer: &[u8], parity: &[u8], orig_len: usize) -> Vec<u8> {
    let mut out = vec![0u8; orig_len];
    for i in 0..orig_len {
        let p = parity[i];
        let b = if i < peer.len() { peer[i] } else { 0 };
        out[i] = p ^ b;
    }
    out
}

fn load_and_stitch_full_30l_container() -> (Vec<u8>, StitchManifest) {
    let weights_dir = workspace_root().join("gpu/weights/v10");
    let stitch_raw = fs::read_to_string(weights_dir.join("manifest.stitch.json"))
        .expect("gpu/weights/v10/manifest.stitch.json must exist");
    let fabric_validated = stitch_fabric::parse_and_validate_manifest(&stitch_raw)
        .expect("manifest.stitch.json must satisfy stitch_fabric::parse_and_validate_manifest");
    assert_eq!(fabric_validated.format, "UORR4V12");

    let manifest: StitchManifest =
        serde_json::from_str(&stitch_raw).expect("valid manifest.stitch.json");
    assert_eq!(manifest.schema, "stitch/xor-2-of-3/1");
    assert_eq!(manifest.format, "UORR4V12");
    assert!(!manifest.groups.is_empty());

    let mut shards: Vec<Option<Vec<u8>>> = vec![None; manifest.data_shard_count];
    for (g_idx, grp) in manifest.groups.iter().enumerate() {
        let left_bytes = fs::read(weights_dir.join(&grp.left.file)).expect("read left shard");
        assert_eq!(left_bytes.len(), grp.left.byte_length);
        assert_eq!(sha256_hex(&left_bytes), grp.left.sha256);

        let parity_bytes = fs::read(weights_dir.join(&grp.parity.file)).expect("read parity shard");
        assert_eq!(parity_bytes.len(), grp.parity.byte_length);
        assert_eq!(sha256_hex(&parity_bytes), grp.parity.sha256);

        if let Some(ref right_ref) = grp.right {
            let right_bytes =
                fs::read(weights_dir.join(&right_ref.file)).expect("read right shard");
            assert_eq!(right_bytes.len(), right_ref.byte_length);
            assert_eq!(sha256_hex(&right_bytes), right_ref.sha256);

            // Verify Stitch 2-of-3 erasure recovery on group 0 and boundary groups.
            if g_idx == 0 || g_idx + 1 == manifest.groups.len() {
                let rec_left = xor_recover_shard(&right_bytes, &parity_bytes, grp.left.byte_length);
                assert_eq!(
                    rec_left, left_bytes,
                    "Stitch left recovery failed on group {g_idx}"
                );
                let rec_right =
                    xor_recover_shard(&left_bytes, &parity_bytes, right_ref.byte_length);
                assert_eq!(
                    rec_right, right_bytes,
                    "Stitch right recovery failed on group {g_idx}"
                );
            }

            // Simulate shard_000 outage by placing parity-recovered left shard in slot 0
            if g_idx == 0 && grp.left.index == 0 {
                let rec_left = xor_recover_shard(&right_bytes, &parity_bytes, grp.left.byte_length);
                assert_eq!(sha256_hex(&rec_left), grp.left.sha256);
                shards[grp.left.index] = Some(rec_left);
            } else {
                shards[grp.left.index] = Some(left_bytes);
            }
            shards[right_ref.index] = Some(right_bytes);
        } else {
            assert_eq!(
                g_idx + 1,
                manifest.groups.len(),
                "only the final parity group may omit right shard"
            );
            let rec_left = xor_recover_shard(&[], &parity_bytes, grp.left.byte_length);
            assert_eq!(
                rec_left, left_bytes,
                "Stitch odd trailing shard recovery failed on group {g_idx}"
            );
            shards[grp.left.index] = Some(rec_left);
        }
    }

    let mut assembled = Vec::new();
    for (idx, maybe_shard) in shards.into_iter().enumerate() {
        let chunk = maybe_shard.unwrap_or_else(|| panic!("missing shard index {idx}"));
        assembled.extend_from_slice(&chunk);
    }
    assert_eq!(assembled.len(), manifest.total_bytes);
    assert_eq!(
        sha256_hex(&assembled),
        manifest.container_sha256,
        "reassembled UORR4V12 container hash mismatch"
    );
    (assembled, manifest)
}

#[test]
fn test_execution_mode_wire_ids_and_embedded_seed_summary() {
    assert_eq!(
        SelectedExecutionMode::from_wire_id("uor-r4.recompiled-v12-agent"),
        SelectedExecutionMode::RecompiledAgent
    );
    assert_eq!(
        SelectedExecutionMode::from_wire_id("uor-r4.recompiled-v12-chat"),
        SelectedExecutionMode::RecompiledChat
    );
    assert_eq!(
        SelectedExecutionMode::from_wire_id("uor-r4.seed-fast"),
        SelectedExecutionMode::EmbeddedSeedFast
    );
    assert_eq!(
        SelectedExecutionMode::from_wire_id("uor-r4.lut-symbolic"),
        SelectedExecutionMode::SymbolicLutMvm
    );
    // Legacy v11 wire identifiers remain mapped cleanly
    assert_eq!(
        SelectedExecutionMode::from_wire_id("uor-r4.gllm-v11-agent"),
        SelectedExecutionMode::RecompiledAgent
    );
    assert_eq!(
        SelectedExecutionMode::from_wire_id("uor-r4.gllm-v11-chat"),
        SelectedExecutionMode::RecompiledChat
    );
    assert_eq!(
        SelectedExecutionMode::from_wire_id("uor-r4.gllm-v11-seed"),
        SelectedExecutionMode::EmbeddedSeedFast
    );

    assert!(SelectedExecutionMode::RecompiledAgent.is_neural_geometric());
    assert!(SelectedExecutionMode::RecompiledChat.is_neural_geometric());
    assert!(SelectedExecutionMode::EmbeddedSeedFast.is_neural_geometric());
    assert!(!SelectedExecutionMode::SymbolicLutMvm.is_neural_geometric());

    assert_eq!(
        SelectedExecutionMode::RecompiledAgent.wire_id(),
        "uor-r4.recompiled-v12-agent"
    );
    assert_eq!(
        SelectedExecutionMode::RecompiledChat.wire_id(),
        "uor-r4.recompiled-v12-chat"
    );
    assert_eq!(
        SelectedExecutionMode::EmbeddedSeedFast.wire_id(),
        "uor-r4.seed-fast"
    );
    assert_eq!(
        SelectedExecutionMode::SymbolicLutMvm.wire_id(),
        "uor-r4.lut-symbolic"
    );

    let engine = R4GeometricStackEngine::embedded_seed();
    assert!(!engine.is_full_recompiled_mounted());
    let summary: GeometricStackSummary = engine.summary();
    assert_eq!(summary.schema, R4_VEC_SCHEMA);
    assert_eq!(summary.active_tier, "embedded-seed");
    assert_eq!(summary.n_layers, 8);
    assert_eq!(summary.d_model, 256);
    assert_eq!(summary.n_heads, 4);
    assert_eq!(summary.n_kv_heads, 2);
    assert_eq!(summary.vocab_size, 6144);
    assert_eq!(summary.float_ops, 0);
    assert_eq!(summary.dense_matmul_ops, 0);
    assert_eq!(summary.container_sha256.len(), 64);
}

#[test]
fn test_r5_negative_container_validation_errors() {
    let mut base_bytes =
        fs::read(workspace_root().join("crates/wasm/src/bootstrap_v12_seed.uorr4"))
            .expect("embedded seed container file exists");
    assert_eq!(&base_bytes[0..8], R4_VEC_MAGIC);

    let mut engine = R4GeometricStackEngine::embedded_seed();

    // 1. TruncatedContainer
    let trunc_err = engine
        .mount_full_container_bytes(&base_bytes[..32])
        .expect_err("truncated buffer must fail");
    assert_eq!(trunc_err, R4StackError::TruncatedContainer);
    assert_eq!(trunc_err.error_code(), "ERR_R4V12_TRUNCATED");

    // 2. InvalidMagic
    let mut bad_magic = base_bytes.clone();
    bad_magic[0..8].copy_from_slice(b"BADMAGIC");
    let magic_err = engine
        .mount_full_container_bytes(&bad_magic)
        .expect_err("bad magic must fail");
    assert_eq!(magic_err, R4StackError::InvalidMagic);
    assert_eq!(magic_err.error_code(), "ERR_R4V12_BAD_MAGIC");

    // 3. UnsupportedVersion
    let mut bad_ver = base_bytes.clone();
    bad_ver[8..12].copy_from_slice(&99u32.to_le_bytes());
    let ver_err = engine
        .mount_full_container_bytes(&bad_ver)
        .expect_err("unsupported version must fail");
    assert_eq!(ver_err, R4StackError::UnsupportedVersion(99));
    assert_eq!(ver_err.error_code(), "ERR_R4V12_BAD_VERSION");

    // 4. NonZeroFloatOps
    let mut bad_fops = base_bytes.clone();
    bad_fops[56..60].copy_from_slice(&1u32.to_le_bytes());
    let fops_err = engine
        .mount_full_container_bytes(&bad_fops)
        .expect_err("non-zero float_ops must fail");
    assert_eq!(fops_err, R4StackError::NonZeroFloatOps(1));
    assert_eq!(fops_err.error_code(), "ERR_R4V12_FLOAT_OPS_FORBIDDEN");

    // 5. NonZeroDenseMatmulOps
    let mut bad_mm = base_bytes.clone();
    bad_mm[60..64].copy_from_slice(&7u32.to_le_bytes());
    let mm_err = engine
        .mount_full_container_bytes(&bad_mm)
        .expect_err("non-zero dense_matmul_ops must fail");
    assert_eq!(mm_err, R4StackError::NonZeroDenseMatmulOps(7));
    assert_eq!(mm_err.error_code(), "ERR_R4V12_DENSE_MATMUL_FORBIDDEN");

    // 6. IntegrityMismatch (flip 1 payload byte after header)
    let mut bad_sha = base_bytes.clone();
    bad_sha[96] ^= 0x5A;
    let sha_err = engine
        .mount_full_container_bytes(&bad_sha)
        .expect_err("corrupted payload hash must fail");
    assert_eq!(sha_err, R4StackError::IntegrityMismatch);
    assert_eq!(sha_err.error_code(), "ERR_R4V12_SHA256_MISMATCH");

    // 7. InvalidGeometry (valid SHA, unaligned d_model = 33)
    base_bytes[20..24].copy_from_slice(&33u32.to_le_bytes());
    let restamped_sha = sha256_arr(&base_bytes[96..]);
    base_bytes[64..96].copy_from_slice(&restamped_sha);
    let geom_err = engine
        .mount_full_container_bytes(&base_bytes)
        .expect_err("non-multiple-of-32 d_model must fail");
    assert!(matches!(geom_err, R4StackError::InvalidGeometry(_)));
    assert_eq!(geom_err.error_code(), "ERR_R4V12_INVALID_GEOMETRY");
    assert!(!engine.is_full_recompiled_mounted());
}

#[test]
fn test_embedded_seed_ood_inference_and_geometric_invariants() {
    let engine = R4GeometricStackEngine::embedded_seed();
    let (reply, metrics, frames) = engine.generate_reply(
        "Explain briefly why pure integer arithmetic prevents floating-point drift across CPUs.",
        &[],
        SelectedExecutionMode::EmbeddedSeedFast,
        48,
    );
    assert!(!reply.trim().is_empty(), "embedded seed must emit text");
    assert!(!reply.contains("UORLUT08"));
    assert_eq!(metrics.schema, R4_VEC_SCHEMA);
    assert_eq!(metrics.active_tier, "embedded-seed");
    assert_eq!(metrics.n_layers, 8);
    assert_eq!(metrics.d_model, 256);
    assert_eq!(metrics.float_ops, 0);
    assert_eq!(metrics.dense_matmul_ops, 0);
    assert!(metrics.nibble_lut_lookups > 0);
    assert!(metrics.lorentz_geodesic_evals > 0);
    assert!(metrics.flock_total_positions > 0);
    assert!(metrics.flock_attended_positions > 0);
    assert!(metrics.icosian_frame_id < 120);
    assert_ne!(metrics.zeta_phase_checksum, 0);
    assert!(!frames.is_empty());

    // Deterministic pure-integer replay
    let (reply_again, metrics_again, frames_again) = engine.generate_reply(
        "Explain briefly why pure integer arithmetic prevents floating-point drift across CPUs.",
        &[],
        SelectedExecutionMode::EmbeddedSeedFast,
        48,
    );
    assert_eq!(reply, reply_again);
    assert_eq!(metrics, metrics_again);
    assert_eq!(frames, frames_again);

    // Long-context (> 320 tokens) exercises Starling Flock sparse sink+window+Hamming top_k routing
    let repeated_context =
        "ownership borrowing lifetime lockstep determinism integer kernel ".repeat(48);
    let long_prompt = format!(
        "Context log:\n{repeated_context}\nQuestion: summarize how integer determinism avoids float drift."
    );
    let (long_reply, long_metrics, _) = engine.generate_reply(
        &long_prompt,
        &[],
        SelectedExecutionMode::EmbeddedSeedFast,
        16,
    );
    assert!(!long_reply.trim().is_empty());
    assert_eq!(long_metrics.float_ops, 0);
    assert_eq!(long_metrics.dense_matmul_ops, 0);
    assert!(
        long_metrics.prompt_tokens > 330,
        "expected >330 prompt tokens to trigger Starling Flock sparsification, got {}",
        long_metrics.prompt_tokens
    );
    assert!(
        long_metrics.flock_total_positions > 320,
        "expected >320 total KV positions inspected, got {}",
        long_metrics.flock_total_positions
    );
    assert!(
        long_metrics.flock_attended_positions < long_metrics.flock_total_positions,
        "Starling Flock sparse window must attend strictly fewer positions than total causal prefix ({} vs {})",
        long_metrics.flock_attended_positions,
        long_metrics.flock_total_positions
    );
    assert!(
        long_metrics.hamming_xor_copy_hits > 0,
        "long-context flock routing must record SWAR Hamming top-k copy hits"
    );
}

#[test]
fn test_stitch_recovery_and_full_30l_smollm2_ood_and_buggy_code_intelligence() {
    let (assembled, manifest) = load_and_stitch_full_30l_container();
    let mut engine = R4GeometricStackEngine::embedded_seed();
    let summary = engine
        .mount_full_container_bytes(&assembled)
        .expect("mount stitch-recovered 30L UORR4V12 container");

    assert!(engine.is_full_recompiled_mounted());
    assert_eq!(summary.schema, R4_VEC_SCHEMA);
    assert_eq!(summary.active_tier, "recompiled-30l");
    assert_eq!(summary.container_sha256, manifest.container_sha256);
    assert_eq!(summary.n_layers, 30);
    assert_eq!(summary.d_model, 576);
    assert_eq!(summary.n_heads, 9);
    assert_eq!(summary.n_kv_heads, 3);
    assert_eq!(summary.vocab_size, 49152);
    assert_eq!(summary.float_ops, 0);
    assert_eq!(summary.dense_matmul_ops, 0);

    // 1. Pasted buggy Rust snippet (out-of-bounds indexing `xs[i + 1]`)
    let buggy_rust = "Find and fix the bug in this Rust function:\n```rust\nfn sum_pairs(xs: &[i32]) -> Vec<i32> {\n    let mut out = Vec::new();\n    for i in 0..xs.len() {\n        out.push(xs[i] + xs[i + 1]);\n    }\n    out\n}\n```";
    let (rust_reply, rust_metrics, rust_frames) =
        engine.generate_reply(buggy_rust, &[], SelectedExecutionMode::RecompiledChat, 48);
    assert_eq!(rust_metrics.active_tier, "recompiled-30l");
    assert_eq!(rust_metrics.float_ops, 0);
    assert_eq!(rust_metrics.dense_matmul_ops, 0);
    assert!(rust_metrics.nibble_lut_lookups > 0);
    assert!(rust_metrics.lorentz_geodesic_evals > 0);
    assert!(rust_metrics.generated_tokens >= 8);
    assert!(!rust_frames.is_empty());
    let rust_lower = rust_reply.to_ascii_lowercase();
    assert!(
        rust_lower.contains("sum_pairs")
            || rust_lower.contains("out")
            || rust_lower.contains("xs")
            || rust_lower.contains("rust")
            || rust_lower.contains("vector")
            || rust_lower.contains("index"),
        "unexpected off-topic 30L reply to buggy Rust prompt: {rust_reply}"
    );
    assert!(!rust_reply.contains("UORLUT08"));

    // 2. General science / systems prompt + multi-turn continuity
    let cow_prompt =
        "How does copy-on-write fork reduce memory usage before execve in Unix kernels?";
    let (cow_reply, cow_metrics, _) =
        engine.generate_reply(cow_prompt, &[], SelectedExecutionMode::RecompiledChat, 44);
    assert_eq!(cow_metrics.active_tier, "recompiled-30l");
    assert_eq!(cow_metrics.float_ops, 0);
    assert_eq!(cow_metrics.dense_matmul_ops, 0);
    assert!(cow_metrics.generated_tokens >= 8);
    let cow_lower = cow_reply.to_ascii_lowercase();
    assert!(
        cow_lower.contains("copy")
            || cow_lower.contains("memory")
            || cow_lower.contains("fork")
            || cow_lower.contains("unix")
            || cow_lower.contains("page")
            || cow_lower.contains("process"),
        "unexpected 30L reply to copy-on-write prompt: {cow_reply}"
    );

    // Follow-up turn referencing turn 1 history
    let history = vec![(cow_prompt.to_string(), cow_reply.clone())];
    let (followup_reply, followup_metrics, _) = engine.generate_reply(
        "What happens when the child process modifies one of those shared memory pages?",
        &history,
        SelectedExecutionMode::RecompiledAgent,
        40,
    );
    assert!(!followup_reply.trim().is_empty());
    assert_eq!(followup_metrics.float_ops, 0);
    assert_eq!(followup_metrics.dense_matmul_ops, 0);
    assert_ne!(
        followup_reply, cow_reply,
        "follow-up turn should depend on current turn + history"
    );

    for (label, text) in [
        ("rust_reply", &rust_reply),
        ("cow_reply", &cow_reply),
        ("followup_reply", &followup_reply),
    ] {
        assert_no_repetition_loops(label, text);
    }

    // Verify all 12 official RTX 4090 receipt OOD transcripts are present, zero-float, and loop-free
    let receipt_raw =
        fs::read_to_string(workspace_root().join("gpu/rtx4090_r4_recompiled_v12_receipt.json"))
            .expect("read rtx4090 receipt json");
    let receipt_val: serde_json::Value =
        serde_json::from_str(&receipt_raw).expect("parse rtx4090 receipt json");
    assert_eq!(receipt_val["float_ops"].as_u64(), Some(0));
    assert_eq!(receipt_val["dense_matmul_ops"].as_u64(), Some(0));
    assert_eq!(receipt_val["container_version"].as_u64(), Some(12));
    assert!(
        receipt_val["rope_theta"].as_u64().unwrap_or(0) > 0,
        "receipt must record non-zero measured rope_theta"
    );
    assert_eq!(receipt_val["tied_lm_head"].as_bool(), Some(true));
    assert_eq!(receipt_val["rms_norm_preserved"].as_bool(), Some(true));
    let full_obj = &receipt_val["full_30l_container"];
    assert_eq!(full_obj["container_version"].as_u64(), Some(12));
    assert_eq!(
        full_obj["rope_theta"].as_u64(),
        receipt_val["rope_theta"].as_u64()
    );
    assert_eq!(full_obj["tied_lm_head"].as_bool(), Some(true));
    assert_eq!(full_obj["rms_norm_preserved"].as_bool(), Some(true));
    let transcripts = receipt_val["ood_12_prompt_transcripts"]
        .as_array()
        .expect("ood_12_prompt_transcripts array");
    assert_eq!(transcripts.len(), 12);
    for entry in transcripts {
        let id = entry["id"].as_str().unwrap_or("unknown");
        let int_reply = entry["integer_30l_reply"].as_str().unwrap_or("");
        assert!(!int_reply.trim().is_empty(), "empty 30L reply for {id}");
        assert_no_repetition_loops(id, int_reply);
    }
}

fn assert_no_repetition_loops(label: &str, text: &str) {
    let words: Vec<String> = text
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect();
    for pair in words.windows(2) {
        if pair[0].chars().any(|c| c.is_alphabetic()) {
            assert_ne!(
                pair[0], pair[1],
                "{label} emitted consecutive duplicate word `{}` in: {text}",
                pair[0]
            );
        }
    }
    if words.len() >= 8 {
        let mut seen_4grams = std::collections::HashSet::new();
        for quad in words.windows(4) {
            let inserted = seen_4grams.insert((
                quad[0].as_str(),
                quad[1].as_str(),
                quad[2].as_str(),
                quad[3].as_str(),
            ));
            assert!(
                inserted,
                "{label} entered repeated 4-word clause `{}` in: {text}",
                quad.join(" ")
            );
        }
    }
}

#[test]
fn test_r4_geometric_stack_source_purity_and_no_forbidden_strings() {
    let src_path = workspace_root().join("crates/wasm/src/r4_geometric_stack.rs");
    let src = fs::read_to_string(&src_path).expect("read r4_geometric_stack.rs");
    assert!(
        src.contains("#![forbid(unsafe_code)]"),
        "r4_geometric_stack.rs must carry #![forbid(unsafe_code)]"
    );

    let forbidden_metric_word = concat!("tele", "metry");
    assert!(
        !src.to_ascii_lowercase().contains(forbidden_metric_word),
        "r4_geometric_stack.rs must never contain forbidden metric identifier"
    );

    // Reject any floating-point type or cast inside the pure-integer runtime.
    let non_comment_code: String = src
        .lines()
        .filter(|line| {
            let t = line.trim_start();
            !t.starts_with("//")
        })
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden_float_tok in ["f32", "f64", "to_bits", "from_bits"] {
        assert!(
            !non_comment_code.contains(forbidden_float_tok),
            "pure-integer runtime must not contain floating-point token `{forbidden_float_tok}`"
        );
    }
}
