//! Behavioral, anti-stamping, and negative security tests for `R4-38`:
//! `UOR-R4-GLLM v9` (`UORGLLM9`) Compositional Geometric Language Model runtime
//! and live `MVM-Wasm` (`R4-37`) agentic tool-execution loop.

use lexlean_r4_wasm::gllm_v9::{GllmV9Error, GllmV9Model};
use lexlean_r4_wasm::lexlean_generated::r4_compositional_gllm::{
    admit_causal_kv_step, ContainerVerdict, KvAttentionCheck, KvAttentionVerdict,
};
use lexlean_r4_wasm::{studio_dispatch, studio_gllm_v9_status, studio_load_uorgllm_v9_bytes};
use sha2::{Digest, Sha256};

static STUDIO_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_gllm_v9_zero_stamped_turn_memory_and_negative_refusals_r4_38() {
    let model = GllmV9Model::default_embedded();
    let hdr = model.header();
    assert_eq!(&hdr.magic, b"UORGLLM9");
    assert_eq!(hdr.version, 9);
    assert_eq!(hdr.vocab_size, 4096);
    assert_eq!(hdr.d_model % 64, 0);
    assert!(hdr.n_layers >= 6);
    assert!(hdr.n_heads >= 8);
    assert_eq!(
        hdr.turn_memory_entries, 0,
        "UOR-R4-GLLM v9 strictly forbids canned ServedTurnMemory entries"
    );
    assert_eq!(hdr.dense_matmul_ops, 0, "Zero dense matmul allowed");
    assert_eq!(hdr.softmax_ops, 0, "Zero softmax allowed");
    assert_eq!(model.admit_container(), ContainerVerdict::ContainerAdmitted);

    // Negative test 1: Tamper turn_memory_entries != 0 -> RefusedStampedTurnMemory
    let mut raw = model.to_container_bytes();
    raw[32..36].copy_from_slice(&1u32.to_le_bytes());
    let err = GllmV9Model::parse(&raw).expect_err("non-zero turn_memory_entries must be refused");
    assert_eq!(
        err,
        GllmV9Error::ContainerRefused(ContainerVerdict::RefusedStampedTurnMemory)
    );

    // Negative test 2: Bad magic -> RefusedBadMagicOrVersion
    let mut bad_magic = model.to_container_bytes();
    bad_magic[0..8].copy_from_slice(b"UORLUT01");
    assert_eq!(
        GllmV9Model::parse(&bad_magic).expect_err("bad magic must fail"),
        GllmV9Error::ContainerRefused(ContainerVerdict::RefusedBadMagicOrVersion)
    );

    // Negative test 3: Payload SHA-256 corruption -> IntegrityMismatch
    let mut bad_sha = model.to_container_bytes();
    let last = bad_sha.len() - 1;
    bad_sha[last] ^= 0x5A;
    assert_eq!(
        GllmV9Model::parse(&bad_sha).expect_err("tampered weight byte must fail SHA-256"),
        GllmV9Error::IntegrityMismatch
    );

    // Negative test 4: Future token causal mask leak refused
    assert_eq!(
        admit_causal_kv_step(KvAttentionCheck {
            seq_len: 16,
            max_seq_len: 512,
            attended_pos: 10,
            query_pos: 9,
            prll_mass_sum: 1_000_000,
            expected_quantum: 1_000_000,
        }),
        KvAttentionVerdict::RefusedFutureTokenLeak
    );
}

#[test]
fn test_gllm_v9_prompt_sensitive_compositional_prediction_unseen_identifiers_r4_38() {
    let model = GllmV9Model::default_embedded();

    let prompt_a = "Given sensor_drift_k9 = 41 and base_offset_q3 = 17, write a concise Rust expression combining sensor_drift_k9 and base_offset_q3.";
    let prompt_b = "Given orbital_phase_m2 = 88 and lattice_stride_z7 = 9, write a concise Rust expression combining orbital_phase_m2 and lattice_stride_z7.";

    let out_a = model.generate_compositional(prompt_a, &[], 96);
    let out_b = model.generate_compositional(prompt_b, &[], 96);

    assert_eq!(out_a.turn_memory_hits, 0);
    assert_eq!(out_b.turn_memory_hits, 0);
    assert!(out_a.layers_executed >= 6);
    assert!(out_a.kv_prompt_tokens >= 10);
    assert!(out_a.prll_mass_conserved);
    assert!(out_b.prll_mass_conserved);

    assert_ne!(
        out_a.text, out_b.text,
        "Compositional GLLM v9 must produce distinct prompt-conditioned completions"
    );
    assert!(
        out_a.text.contains("sensor_drift_k9") && out_a.text.contains("base_offset_q3"),
        "Causal KV induction attention must copy unseen identifiers from prompt A, got: {}",
        out_a.text
    );
    assert!(
        !out_a.text.contains("orbital_phase_m2"),
        "Output A must not contain prompt B identifiers: {}",
        out_a.text
    );
    assert!(
        out_b.text.contains("orbital_phase_m2") && out_b.text.contains("lattice_stride_z7"),
        "Causal KV induction attention must copy unseen identifiers from prompt B, got: {}",
        out_b.text
    );

    for step in &out_a.steps {
        assert!(
            step.top_attended_pos <= step.query_pos,
            "causal leak: attended {} > query {}",
            step.top_attended_pos,
            step.query_pos
        );
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest.as_slice() {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[test]
fn test_gllm_v9_opfs_8l_128d_shard_reassembly_and_hotload_r4_38() {
    let _guard = STUDIO_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = studio_dispatch("1\tstudio\treset\t");
    let v9_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gpu/weights/v9");
    let manifest_raw = std::fs::read_to_string(v9_dir.join("manifest.json"))
        .expect("gpu/weights/v9/manifest.json must exist");
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_raw).expect("valid v9 manifest JSON");

    assert_eq!(manifest["format"], "UORGLLM9");
    assert_eq!(manifest["turn_memory_entries"], 0);
    assert_eq!(manifest["dense_matmul_ops"], 0);
    assert_eq!(manifest["softmax_ops"], 0);

    let shards = manifest["shards"].as_array().expect("shards array");
    assert!(!shards.is_empty(), "must emit >= 1 OPFS shard");
    let mut full_v9 = Vec::new();
    for s in shards {
        let fname = s["file"].as_str().expect("shard filename");
        let expected_len = s["byte_length"].as_u64().expect("byte_length") as usize;
        let expected_sha = s["sha256"].as_str().expect("shard sha256");
        assert!(
            expected_len <= 50 * 1024 * 1024,
            "shard {fname} exceeds 50 MB GitHub Pages limit: {expected_len}"
        );
        let part = std::fs::read(v9_dir.join(fname)).expect("read v9 shard");
        assert_eq!(part.len(), expected_len, "shard {fname} length mismatch");
        let actual_sha = sha256_hex(&part);
        assert_eq!(actual_sha, expected_sha, "shard {fname} SHA-256 mismatch");
        full_v9.extend_from_slice(&part);
    }

    let total_sha = sha256_hex(&full_v9);
    assert_eq!(
        total_sha,
        manifest["container_sha256"].as_str().unwrap_or(""),
        "reassembled OPFS container SHA-256 mismatch"
    );

    let load_json: serde_json::Value =
        serde_json::from_str(&studio_load_uorgllm_v9_bytes(&full_v9, "opfs_v9_8L_128D"))
            .expect("valid studio_load_uorgllm_v9_bytes JSON");
    assert_eq!(load_json["status"], "ok", "load_json: {load_json}");
    assert_eq!(load_json["d_model"], 128);
    assert_eq!(load_json["n_layers"], 8);
    assert_eq!(load_json["n_heads"], 16);
    assert_eq!(load_json["turn_memory_entries"], 0);

    let status_json: serde_json::Value =
        serde_json::from_str(&studio_gllm_v9_status()).expect("valid studio_gllm_v9_status JSON");
    assert_eq!(status_json["d_model"], 128);
    assert_eq!(status_json["n_layers"], 8);

    // Verify end-to-end Studio chat dispatch uses the hot-loaded 8L 128D UORGLLM9 engine
    let chat_raw = studio_dispatch(
        "1\tstudio\tchat\tGiven telemetry_gain_v4 = 63 and phase_bias_r2 = 19, write a concise Rust expression combining telemetry_gain_v4 and phase_bias_r2.",
    );
    let chat_res: serde_json::Value = serde_json::from_str(&chat_raw).expect("valid chat JSON");
    assert_eq!(chat_res["status"], "ok", "chat_res: {chat_res}");
    let reply = chat_res["assistant_reply"].as_str().unwrap_or("");
    assert!(
        reply.contains("telemetry_gain_v4") && reply.contains("phase_bias_r2"),
        "8L 128D Studio chat must copy unseen prompt identifiers via causal KV induction, got: {reply}"
    );
    let _ = studio_load_uorgllm_v9_bytes(
        include_bytes!("../src/bootstrap_v9.uorgllm"),
        "bootstrap_v9_restored",
    );
}

#[test]
fn test_gllm_v9_agentic_mvm_sandbox_tool_loop_r4_38() {
    let _guard = STUDIO_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = studio_dispatch("1\tstudio\treset\t");
    let res_raw = studio_dispatch(
        "1\tstudio\tgllm_v9_agent\tRun uname -a and compute echo -n gllm-v9-mvm | sha256sum inside the Linux MicroVM sandbox.",
    );
    let res: serde_json::Value = serde_json::from_str(&res_raw).expect("valid gllm_v9_agent JSON");
    assert_eq!(res["status"], "ok", "res: {res}");
    assert_eq!(res["turn_memory_hits"], 0);
    assert_eq!(res["mvm_tool_executed"], true, "res: {res}");
    assert_eq!(res["mvm_exit_code"], 0);
    let stdout = res["mvm_stdout"].as_str().unwrap_or("");
    assert!(
        stdout.contains("riscv64"),
        "expected live RV64IMAC MicroVM stdout, got: {stdout}"
    );
    let reply = res["assistant_reply"].as_str().unwrap_or("");
    assert!(
        reply.contains("riscv64"),
        "final compositional answer must incorporate live MVM-Wasm tool observation, got: {reply}"
    );
}
