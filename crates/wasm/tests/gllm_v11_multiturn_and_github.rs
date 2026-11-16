//! Integration tests verifying GLLM v11 pure-integer CORDIC RoPE, PRLL attention,
//! 4+ turn continuity, Stitch 2-of-3 XOR shard recovery, and GitHub agentic tool calls.

use lexlean_r4_wasm::gllm_v9::{
    apply_rope_q12_inplace, GllmV9Error, GllmV9Model, GLLM_V11_MAX_PROMPT_SEQ, GLLM_V9_MAX_SEQ,
    GLLM_V9_VOCAB, ROPE_COS_Q12, ROPE_SIN_Q12, UORGLLM9_MAGIC, UORGLLM9_VERSION,
};
use lexlean_r4_wasm::lexlean_generated::r4_compositional_gllm::ContainerVerdict;
use lexlean_r4_wasm::StudioEngine;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn restamp_container(mut raw: Vec<u8>) -> Vec<u8> {
    let digest = sha256_bytes(&raw[80..]);
    raw[48..80].copy_from_slice(&digest);
    raw
}

fn xor_recover_shard(peer: &[u8], parity: &[u8], orig_len: usize) -> Vec<u8> {
    let mut out = vec![0u8; orig_len];
    for i in 0..orig_len {
        let b_parity = parity[i];
        let b_peer = if i < peer.len() { peer[i] } else { 0 };
        out[i] = b_parity ^ b_peer;
    }
    out
}

#[test]
fn test_v11_container_geometry_rope_and_zero_float_determinism() {
    let model = GllmV9Model::default_embedded();
    let hdr = model.header();

    assert_eq!(hdr.magic, UORGLLM9_MAGIC);
    assert_eq!(hdr.version, UORGLLM9_VERSION);
    assert_eq!(hdr.version, 11);
    assert_eq!(hdr.vocab_size, GLLM_V9_VOCAB as u32);
    assert_eq!(hdr.n_layers, 6);
    assert_eq!(hdr.d_model, 128);
    assert_eq!(hdr.n_heads, 16);
    assert_eq!(hdr.lut_groups, 32);
    assert_eq!(hdr.quant_bits, 4);
    assert_eq!(hdr.turn_memory_entries, 0);
    assert_eq!(hdr.dense_matmul_ops, 0);
    assert_eq!(hdr.softmax_ops, 0);
    assert_eq!(model.admit_container(), ContainerVerdict::ContainerAdmitted);
    assert!(model.is_v11_or_later());
    assert!(!model.is_legacy_v9_container());
    assert_eq!(GLLM_V9_MAX_SEQ, 1024);
    assert_eq!(GLLM_V11_MAX_PROMPT_SEQ, 768);

    // Pure-integer CORDIC Q12 RoPE checks across 0..1024
    assert_eq!(ROPE_COS_Q12[0][0], 4096);
    assert_eq!(ROPE_SIN_Q12[0][0], 0);
    for pos in [0usize, 1, 7, 64, 255, 512, 767, 1023] {
        let mut vec = vec![0i16; 128];
        for (idx, v) in vec.iter_mut().enumerate() {
            *v = if idx % 2 == 0 { 1600 } else { -1200 };
        }
        let orig_norm_sq: i64 = vec.iter().map(|&v| (v as i64) * (v as i64)).sum();
        apply_rope_q12_inplace(&mut vec, pos);
        let rot_norm_sq: i64 = vec.iter().map(|&v| (v as i64) * (v as i64)).sum();
        let diff = (orig_norm_sq - rot_norm_sq).abs();
        assert!(
            diff * 50 <= orig_norm_sq,
            "RoPE Q12 norm drift too high at pos={pos}: orig={orig_norm_sq}, rot={rot_norm_sq}"
        );
    }

    let mut v_p1 = vec![2048i16, 1024, -1536, 768];
    let mut v_p767 = v_p1.clone();
    apply_rope_q12_inplace(&mut v_p1, 1);
    apply_rope_q12_inplace(&mut v_p767, 767);
    assert_ne!(
        v_p1, v_p767,
        "RoPE must encode distinct phase rotations at distant positions"
    );

    // Order sensitivity via RoPE in causal encoding pulses
    let p_ab = model.encode_prompt_pulses("alpha beta", &[]);
    let p_ba = model.encode_prompt_pulses("beta alpha", &[]);
    assert!(!p_ab.is_empty() && !p_ba.is_empty());
    assert_ne!(
        p_ab, p_ba,
        "Causal CORDIC RoPE + PRLL encoding must be order-sensitive"
    );

    // Zero-float determinism: two identical calls yield bit-identical output and traces
    let history = vec![
        (
            "My project is called aurora-mesh.".to_string(),
            "Noted: project codename is aurora-mesh.".to_string(),
        ),
        (
            "Keep telemetry integer only.".to_string(),
            "Telemetry remains strictly integer Q12.".to_string(),
        ),
    ];
    let g1 = model.generate_compositional("Explain what aurora-mesh is", &history, 48);
    let g2 = model.generate_compositional("Explain what aurora-mesh is", &history, 48);
    assert_eq!(g1.text, g2.text);
    assert_eq!(g1.generated_tokens, g2.generated_tokens);
    assert_eq!(g1.kv_prompt_tokens, g2.kv_prompt_tokens);
    assert_eq!(g1.layers_executed, 6);
    assert_eq!(g1.turn_memory_hits, 0);
    assert!(g1.prll_mass_conserved);
    assert_eq!(g1.brain_pulses, g2.brain_pulses);

    // R5 negative container checks
    assert!(matches!(
        GllmV9Model::parse(b"short"),
        Err(GllmV9Error::TruncatedContainer)
    ));

    let valid = model.to_container_bytes();

    let mut bad_magic = valid.clone();
    bad_magic[0..8].copy_from_slice(b"BADMAGIC");
    assert!(matches!(
        GllmV9Model::parse(&bad_magic),
        Err(GllmV9Error::ContainerRefused(_))
    ));

    let mut bad_sha = valid.clone();
    bad_sha[80] ^= 0x5A;
    assert!(matches!(
        GllmV9Model::parse(&bad_sha),
        Err(GllmV9Error::IntegrityMismatch)
    ));

    let mut bad_ver = valid.clone();
    bad_ver[8..12].copy_from_slice(&12u32.to_le_bytes());
    let bad_ver = restamp_container(bad_ver);
    assert!(matches!(
        GllmV9Model::parse(&bad_ver),
        Err(GllmV9Error::ContainerRefused(_))
    ));

    let mut stamped_mem = valid.clone();
    stamped_mem[32..36].copy_from_slice(&1u32.to_le_bytes());
    let stamped_mem = restamp_container(stamped_mem);
    assert!(matches!(
        GllmV9Model::parse(&stamped_mem),
        Err(GllmV9Error::ContainerRefused(
            ContainerVerdict::RefusedStampedTurnMemory
        ))
    ));

    // Worker turn window gates
    assert!(matches!(
        GllmV9Model::validate_worker_turn_window(false, 64, 32),
        Err(GllmV9Error::WorkerTurnRefused(_))
    ));
    assert!(matches!(
        GllmV9Model::validate_worker_turn_window(true, 800, 256),
        Err(GllmV9Error::WorkerTurnRefused(_))
    ));
    assert!(GllmV9Model::validate_worker_turn_window(true, 768, 256).is_ok());
}

#[test]
fn test_v11_rolling_history_budget_and_uorlut08_isolation() {
    // Rolling 6-turn history exceeds 768 tokens if untruncated; v11 trims cleanly within 768
    let long_history: Vec<(String, String)> = (0..6)
        .map(|i| {
            (
                format!(
                    "Turn {i}: analyze shard subsystem modulus-{} with checksum-{} and deterministic trace-{}",
                    100 + i,
                    900 + i,
                    "detail ".repeat(35)
                ),
                format!(
                    "Reply {i}: verified modulus-{} invariant and pure integer execution trace {}",
                    100 + i,
                    "ack ".repeat(35)
                ),
            )
        })
        .collect();

    // Even WITHOUT a cross-turn cue (`"Continue concisely."`), v11 retains rolling history within <= 768
    let (ids_no_cue, copy_start) =
        GllmV9Model::encode_v11_chat_prompt("Continue concisely.", &long_history, 256);
    assert!(
        ids_no_cue.len() <= GLLM_V11_MAX_PROMPT_SEQ,
        "rolling prompt must fit <= {GLLM_V11_MAX_PROMPT_SEQ}, got {}",
        ids_no_cue.len()
    );
    assert!(
        ids_no_cue.len() > 80,
        "v11 must keep rolling recent history instead of cue-stripping; len={}",
        ids_no_cue.len()
    );
    assert!(copy_start < ids_no_cue.len());

    // UORLUT08 must NOT hijack normal conversational turns in version >= 11,
    // and only routes when explicitly prefixed with `uor:`.
    let mut engine = StudioEngine::new_default();
    let natural: serde_json::Value = serde_json::from_str(&engine.chat_assistant_json(
        "Hello! Can you summarize why deterministic integer arithmetic matters?",
    ))
    .expect("valid assistant json");
    let nat_reply = natural["assistant_reply"].as_str().unwrap_or_default();
    assert!(!nat_reply.trim().is_empty());
    assert!(
        !nat_reply.contains("UORLUT08"),
        "UORLUT08 must not hijack natural conversation in v11: {nat_reply}"
    );
    assert_eq!(natural["generation"]["mode"], "uorgllm_v9");

    let uor_query: serde_json::Value =
        serde_json::from_str(&engine.chat_assistant_json("uor: prime-lattice 42"))
            .expect("valid uor query json");
    let uor_reply = uor_query["assistant_reply"].as_str().unwrap_or_default();
    assert!(
        !uor_reply.trim().is_empty(),
        "explicit uor: query should still produce an answer"
    );
}

#[test]
fn test_v11_multiturn_four_turn_continuity_embedded_and_hosted() {
    // Part A: Embedded 6L x 128D 4+ turn continuity in StudioEngine
    let mut emb = StudioEngine::new_default();
    let status_emb: serde_json::Value =
        serde_json::from_str(&emb.gllm_v9_status_json()).expect("status json");
    assert_eq!(status_emb["version"], 11);
    assert_eq!(status_emb["n_layers"], 6);
    assert_eq!(status_emb["d_model"], 128);
    assert_eq!(status_emb["turn_memory_entries"], 0);

    let prompts = [
        "Remember that our project codename is aurora-mesh.",
        "Keep every telemetry counter purely deterministic integer Q12.",
        "Update the project codename to quantum-ledger now.",
        "Thanks, keep going concisely.",
        "Summarize our codename and arithmetic setup cleanly.",
    ];
    for (idx, p) in prompts.iter().enumerate() {
        let resp: serde_json::Value =
            serde_json::from_str(&emb.chat_assistant_json(p)).expect("chat turn json");
        assert_eq!(resp["status"], "ok");
        assert_eq!(resp["conversation_turn_count"], idx + 1);
        let reply = resp["assistant_reply"].as_str().unwrap_or_default();
        assert!(!reply.trim().is_empty(), "turn {idx} reply empty");
        assert!(
            !reply.contains("UORLUT08"),
            "turn {idx} hijacked by UORLUT08: {reply}"
        );
    }

    // Part B: Hosted 30L x 576D UORR4V12 shards from gpu/weights/v10/
    let weights_dir = workspace_root().join("gpu/weights/v10");
    let manifest_str =
        std::fs::read_to_string(weights_dir.join("manifest.json")).expect("read manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_str).expect("parse manifest.json");
    assert_eq!(manifest["format"], "UORR4V12");
    assert_eq!(manifest["total_bytes"], 75_684_407);
    let shard_meta = manifest["shards"].as_array().expect("manifest shards");
    assert_eq!(
        shard_meta.len(),
        73,
        "expected 73 1MiB shards for 30L x 576D UORR4V12"
    );

    let mut shard_blobs = Vec::with_capacity(shard_meta.len());
    for entry in shard_meta {
        let rel = entry["file"]
            .as_str()
            .or_else(|| entry["path"].as_str().and_then(|p| p.rsplit('/').next()))
            .expect("shard file");
        let bytes = std::fs::read(weights_dir.join(rel)).expect("read shard file");
        shard_blobs.push(bytes);
    }
    let shard_refs: Vec<&[u8]> = shard_blobs.iter().map(|b| b.as_slice()).collect();

    let mut hosted = StudioEngine::new_default();
    let loaded_raw = hosted
        .gllm_load_v10_shards(&manifest_str, &shard_refs)
        .expect("shard load must succeed");
    let loaded_val: serde_json::Value =
        serde_json::from_str(&loaded_raw).expect("parse gllm_load_v10_shards status");
    assert_eq!(loaded_val["status"], "ok");
    assert_eq!(loaded_val["format"], "UORR4V12");
    assert_eq!(loaded_val["full_recompiled_mounted"], true);
    assert_eq!(loaded_val["container_sha256"], manifest["container_sha256"]);

    let hosted_status: serde_json::Value =
        serde_json::from_str(&hosted.r4_stack_status_json()).expect("hosted r4 stack status");
    assert_eq!(hosted_status["format"], "UORR4V12");
    assert_eq!(hosted_status["full_recompiled_mounted"], true);
    assert_eq!(hosted_status["n_layers"], 30);
    assert_eq!(hosted_status["d_model"], 576);
    assert_eq!(hosted_status["float_ops"], 0);
    assert_eq!(hosted_status["dense_matmul_ops"], 0);

    for (idx, p) in prompts.iter().enumerate() {
        let resp: serde_json::Value =
            serde_json::from_str(&hosted.chat_assistant_json(p)).expect("hosted chat turn");
        assert_eq!(resp["status"], "ok");
        assert_eq!(resp["conversation_turn_count"], idx + 1);
        let reply = resp["assistant_reply"].as_str().unwrap_or_default();
        assert!(!reply.trim().is_empty(), "hosted turn {idx} reply empty");
    }

    // Verify pure-integer deterministic generation via selected_model = "uor-r4.native-q31-d85"
    // and strict refusal of retired v12 scaffold wire IDs.
    let retired_req = serde_json::json!({
        "prompt": "What is our updated codename?",
        "composer_mode": "agentic",
        "selected_model": "uor-r4.recompiled-v12-agent",
    })
    .to_string();
    let retired_resp: serde_json::Value =
        serde_json::from_str(&hosted.chat_assistant_json(&retired_req)).expect("retired json");
    assert_eq!(retired_resp["status"], "error");
    assert_eq!(retired_resp["error_code"], "RefusedRetiredScaffoldMode");

    let det_req = serde_json::json!({
        "prompt": "What is our updated codename?",
        "composer_mode": "agentic",
        "selected_model": "uor-r4.native-q31-d85",
    })
    .to_string();
    let mut g1_engine = StudioEngine::new_default();
    g1_engine
        .gllm_load_v10_shards(&manifest_str, &shard_refs)
        .expect("mount g1");
    let mut g2_engine = StudioEngine::new_default();
    g2_engine
        .gllm_load_v10_shards(&manifest_str, &shard_refs)
        .expect("mount g2");
    let g1: serde_json::Value =
        serde_json::from_str(&g1_engine.chat_assistant_json(&det_req)).expect("gen json 1");
    let g2: serde_json::Value =
        serde_json::from_str(&g2_engine.chat_assistant_json(&det_req)).expect("gen json 2");
    assert_eq!(g1["status"], "ok");
    assert_eq!(g1["geometric_metrics"]["n_layers"], 4);
    assert_eq!(g1["geometric_metrics"]["d_model"], 256);
    assert_eq!(g1["geometric_metrics"]["float_ops"], 0);
    assert_eq!(g1["geometric_metrics"]["dense_matmul_ops"], 0);
    assert_eq!(g1["assistant_reply"], g2["assistant_reply"]);
    assert_eq!(
        hosted_status["container_sha256"],
        manifest["container_sha256"]
    );
}

#[test]
fn test_v11_stitch_xor_2of3_shard_recovery_and_load() {
    let weights_dir = workspace_root().join("gpu/weights/v10");
    let manifest_str =
        std::fs::read_to_string(weights_dir.join("manifest.json")).expect("read manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_str).expect("parse manifest.json");
    assert_eq!(manifest["format"], "UORR4V12");
    assert_eq!(manifest["total_bytes"], 75_684_407);

    let stitch_str = std::fs::read_to_string(weights_dir.join("manifest.stitch.json"))
        .expect("read manifest.stitch.json");
    let stitch: serde_json::Value =
        serde_json::from_str(&stitch_str).expect("parse manifest.stitch.json");

    let n_data = stitch["data_shard_count"]
        .as_u64()
        .expect("data_shard_count") as usize;
    assert_eq!(n_data, 73);
    let groups = stitch["groups"].as_array().expect("stitch groups");
    assert_eq!(groups.len(), 37);
    assert_eq!(stitch["parity_shard_count"], groups.len());

    let data_arr = manifest["shards"].as_array().expect("data shards");
    let mut data_shards: Vec<Vec<u8>> = data_arr
        .iter()
        .map(|s| {
            let rel = s["file"]
                .as_str()
                .or_else(|| s["path"].as_str().and_then(|p| p.rsplit('/').next()))
                .unwrap();
            std::fs::read(weights_dir.join(rel)).unwrap()
        })
        .collect();
    let parity_shards: Vec<Vec<u8>> = groups
        .iter()
        .map(|g| {
            let pfile = g["parity"]["file"]
                .as_str()
                .or_else(|| g["parity_path"].as_str())
                .unwrap();
            std::fs::read(weights_dir.join(pfile)).unwrap()
        })
        .collect();

    // Case 1: Drop left shard of group 0 (`shard_000.bin`)
    let orig_0 = data_shards[0].clone();
    let rec_0 = xor_recover_shard(&data_shards[1], &parity_shards[0], orig_0.len());
    assert_eq!(rec_0, orig_0, "XOR recovery failed on left shard_000");
    assert_eq!(sha256_hex(&rec_0), data_arr[0]["sha256"].as_str().unwrap());
    data_shards[0] = rec_0;

    // Case 2: Drop right shard of group 8 (`shard_017.bin`)
    let orig_17 = data_shards[17].clone();
    let rec_17 = xor_recover_shard(&data_shards[16], &parity_shards[8], orig_17.len());
    assert_eq!(rec_17, orig_17, "XOR recovery failed on right shard_017");
    assert_eq!(
        sha256_hex(&rec_17),
        data_arr[17]["sha256"].as_str().unwrap()
    );
    data_shards[17] = rec_17;

    // Case 3: Drop unpaired tail shard (`shard_072.bin`, recovered against zeros + `parity_036`)
    let orig_72 = data_shards[72].clone();
    let rec_72 = xor_recover_shard(&[], &parity_shards[36], orig_72.len());
    assert_eq!(rec_72, orig_72, "XOR recovery failed on tail shard_072");
    assert_eq!(
        sha256_hex(&rec_72),
        data_arr[72]["sha256"].as_str().unwrap()
    );
    data_shards[72] = rec_72;

    // Load XOR-recovered shards into StudioEngine and verify identical SHA-256 + 30L mount
    let refs: Vec<&[u8]> = data_shards.iter().map(|b| b.as_slice()).collect();
    let mut engine = StudioEngine::new_default();
    let rec_raw = engine
        .gllm_load_v10_shards(&manifest_str, &refs)
        .expect("load recovered shards");
    let rec_val: serde_json::Value =
        serde_json::from_str(&rec_raw).expect("parse recovered status json");
    assert_eq!(rec_val["status"], "ok");
    assert_eq!(rec_val["format"], "UORR4V12");
    assert_eq!(rec_val["full_recompiled_mounted"], true);
    assert_eq!(rec_val["n_layers"], 30);
    assert_eq!(rec_val["d_model"], 576);
    assert_eq!(rec_val["container_sha256"], manifest["container_sha256"]);
}

#[test]
fn test_v11_github_tool_call_and_bridge_continuation_and_r5_negatives() {
    let mut engine = StudioEngine::new_default();
    let pol = engine.mvm_set_policy_json("safe");
    assert!(pol.contains("\"policy\":\"safe\""), "policy failed: {pol}");

    let wres =
        engine.workspace_write_json("src/main.rs\nfn main() { println!(\"aurora-mesh\"); }\n");
    assert!(
        wres.contains("\"status\":\"ok\""),
        "workspace write failed: {wres}"
    );

    // Test structured `<tool_call>` JSON emission + bridge roundtrip for all GitHub actions
    let branch_prompt = concat!(
        "<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"create_branch\",",
        "\"repo\":\"afflom/uor-r4-test\",\"branch\":\"feat/v11-test\",\"from_branch\":\"main\"}}</tool_call>"
    );
    let branch_resp: serde_json::Value =
        serde_json::from_str(&engine.chat_assistant_json(branch_prompt)).expect("branch json");
    assert_eq!(
        branch_resp["status"], "needs_bridge",
        "create_branch must enter bridge: {branch_resp}"
    );
    let resume_branch = branch_resp["resume_state"].clone();
    assert_eq!(resume_branch["tool"], "git");
    assert_eq!(resume_branch["action"], "create_branch");
    let breq_branch = &branch_resp["bridge_req"];
    assert_eq!(breq_branch["op"], "git");
    assert_eq!(breq_branch["subcommand"], "create_branch");
    assert_eq!(breq_branch["repo"], "afflom/uor-r4-test");
    assert_eq!(breq_branch["branch"], "feat/v11-test");
    assert_eq!(breq_branch["from_branch"], "main");
    assert_eq!(breq_branch["policy"], "https_github_api");

    let comp_branch = engine
        .complete_v11_git_bridge_turn(
            branch_prompt,
            "create_branch",
            &serde_json::json!({
                "ok": true,
                "status_code": 201,
                "body_text": "{\"ref\":\"refs/heads/feat/v11-test\"}"
            }),
        )
        .expect("complete create_branch");
    assert!(
        comp_branch.contains("feat/v11-test"),
        "branch continuation missing ref: {comp_branch}"
    );

    // Test create_pr + multi-turn completion
    let pr_prompt = concat!(
        "<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"create_pr\",",
        "\"repo\":\"afflom/uor-r4-test\",\"title\":\"GLLM v11 Upgrade\",",
        "\"head\":\"feat/v11-test\",\"base\":\"main\"}}</tool_call>"
    );
    let pr_resp: serde_json::Value =
        serde_json::from_str(&engine.chat_assistant_json(pr_prompt)).expect("pr json");
    assert_eq!(pr_resp["status"], "needs_bridge");
    assert_eq!(pr_resp["bridge_req"]["subcommand"], "create_pr");
    let comp_pr = engine
        .complete_v11_git_bridge_turn(
            pr_prompt,
            "create_pr",
            &serde_json::json!({
                "ok": true,
                "status_code": 201,
                "data": { "number": 108, "html_url": "https://github.com/afflom/uor-r4-test/pull/108" }
            }),
        )
        .expect("complete create_pr");
    assert!(
        comp_pr.contains("108"),
        "PR continuation missing #108: {comp_pr}"
    );

    // Test pr_status
    let status_prompt = concat!(
        "<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"pr_status\",",
        "\"repo\":\"afflom/uor-r4-test\",\"pr_number\":108}}</tool_call>"
    );
    let st_resp: serde_json::Value =
        serde_json::from_str(&engine.chat_assistant_json(status_prompt)).expect("pr_status json");
    assert_eq!(st_resp["status"], "needs_bridge");
    assert_eq!(st_resp["bridge_req"]["subcommand"], "pr_status");
    let comp_st = engine
        .complete_v11_git_bridge_turn(
            status_prompt,
            "pr_status",
            &serde_json::json!({
                "ok": true,
                "status_code": 200,
                "body_text": "{\"number\":108,\"state\":\"open\",\"mergeable\":true}"
            }),
        )
        .expect("complete pr_status");
    assert!(comp_st.contains("108") || comp_st.contains("open"));

    // Test watch_deploy WITH OMITTED and EMPTY provenance_url (#171 optional provenance_url fix!)
    let watch_prompt = concat!(
        "<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"watch_deploy\",",
        "\"repo\":\"afflom/uor-r4-test\",\"branch\":\"main\"}}</tool_call>"
    );
    let wd_resp: serde_json::Value =
        serde_json::from_str(&engine.chat_assistant_json(watch_prompt)).expect("watch_deploy json");
    assert_eq!(
        wd_resp["status"], "needs_bridge",
        "watch_deploy without provenance_url must still work: {wd_resp}"
    );
    assert_eq!(wd_resp["bridge_req"]["subcommand"], "watch_deploy");
    assert!(wd_resp["bridge_req"].get("provenance_url").is_none());
    let empty_watch_prompt = concat!(
        "<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"watch_deploy\",",
        "\"repo\":\"afflom/uor-r4-test\",\"branch\":\"main\",\"provenance_url\":\"\"}}</tool_call>"
    );
    let empty_wd_resp: serde_json::Value =
        serde_json::from_str(&engine.chat_assistant_json(empty_watch_prompt))
            .expect("empty provenance_url watch_deploy json");
    assert_eq!(empty_wd_resp["status"], "needs_bridge");
    assert!(empty_wd_resp["bridge_req"].get("provenance_url").is_none());
    let comp_wd = engine
        .complete_v11_git_bridge_turn(
            watch_prompt,
            "watch_deploy",
            &serde_json::json!({
                "ok": true,
                "status_code": 200,
                "body_text": "{\"workflow_runs\":[{\"status\":\"completed\",\"conclusion\":\"success\"}]}",
                "data": { "body_text": "stale nested body_text should be secondary" }
            }),
        )
        .expect("complete watch_deploy");
    assert!(comp_wd.contains("completed") || comp_wd.contains("success"));
    assert!(
        !comp_wd.contains("stale nested body_text"),
        "top-level body_text must take precedence over data.body_text: {comp_wd}"
    );

    // Test commit_push workspace hydration when explicit files array is omitted
    let push_prompt = concat!(
        "<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"commit_push\",",
        "\"repo\":\"afflom/uor-r4-test\",\"branch\":\"feat/v11-test\",",
        "\"message\":\"update main\",\"path\":\"src/main.rs\"}}</tool_call>"
    );
    let push_resp: serde_json::Value =
        serde_json::from_str(&engine.chat_assistant_json(push_prompt)).expect("commit_push json");
    assert_eq!(
        push_resp["status"], "needs_bridge",
        "commit_push workspace hydration must succeed: {push_resp}"
    );
    let hydrated_files = push_resp["bridge_req"]["files"]
        .as_array()
        .expect("hydrated workspace files");
    assert!(!hydrated_files.is_empty());

    // R5 negative 1: watch_deploy with plain http:// provenance_url must be refused
    let bad_watch_prompt = concat!(
        "<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"watch_deploy\",",
        "\"repo\":\"afflom/uor-r4-test\",\"provenance_url\":\"http://insecure.example/run\"}}</tool_call>"
    );
    let bad_wd: serde_json::Value =
        serde_json::from_str(&engine.chat_assistant_json(bad_watch_prompt))
            .expect("bad watch_deploy json");
    assert_ne!(bad_wd["status"], "needs_bridge");
    assert!(
        bad_wd["assistant_reply"]
            .as_str()
            .unwrap_or_default()
            .contains("PlainHttpRefused"),
        "plain http provenance_url must fail with PlainHttpRefused: {bad_wd}"
    );

    // R5 negative 2: bridge error response with status=="error" must not be reported as ok
    let wd_err_prompt = concat!(
        "<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"pr_status\",",
        "\"repo\":\"afflom/uor-r4-test\",\"pr_number\":999}}</tool_call>"
    );
    let st_err: serde_json::Value =
        serde_json::from_str(&engine.chat_assistant_json(wd_err_prompt)).expect("pr_status 999");
    assert_eq!(st_err["status"], "needs_bridge");
    let fail_err = engine
        .complete_v11_git_bridge_turn(
            wd_err_prompt,
            "pr_status",
            &serde_json::json!({
                "status": "error",
                "error_code": "HttpStatus404",
                "message": "Pull Request not found"
            }),
        )
        .expect_err("bridge error payload must fail cleanly");
    assert!(
        fail_err.contains("HttpStatus404"),
        "expected HttpStatus404 error, got: {fail_err}"
    );

    // R5 negative 3: MVM policy = deny blocks git bridge egress cleanly
    let _ = engine.mvm_set_policy_json("deny");
    let denied: serde_json::Value =
        serde_json::from_str(&engine.chat_assistant_json(branch_prompt)).expect("denied json");
    assert_ne!(
        denied["status"], "needs_bridge",
        "policy=deny must never open bridge: {denied}"
    );
    let denied_reply = denied["assistant_reply"].as_str().unwrap_or_default();
    assert!(
        denied_reply.contains("Policy")
            || denied_reply.contains("denied")
            || denied_reply.contains("Refused"),
        "expected policy denial message: {denied}"
    );
}
