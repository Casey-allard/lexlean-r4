//! Integration tests for R4-56: Async 4-Token Foundry Turn Stepper (`FoundryTurnSession`),
//! Foundry `<prism-write>` + `<tool_call>` prompt assembly, UTF-8 incremental flush, and cancel/busy/empty exits.

use lexlean_r4_wasm::holo_geometric_runtime::{
    int_fwht32_i64, rht_signs_packed_u32, HOLO_TOK13_MAGIC, HOLO_V13_MAGIC, HOLO_V13_VERSION,
};
use lexlean_r4_wasm::{
    format_foundry_system_prompt, FoundryToolResult, FoundryWorkspaceFile, StudioEngine,
    UorWasmStudio, BUILD_SYSTEM_PREFIX,
};
use serde_json::Value;

const EXP_RANGE: i32 = 32;
const SILU_STEP_LOG2: i32 = -8;
const SILU_RANGE_LOG2: i32 = 4;
const SIG_WORDS: usize = 8;

fn exp_neg_q30_ref(x_q30: u64) -> u32 {
    if x_q30 >= ((EXP_RANGE as u64) << 30) {
        return 0;
    }
    let scaled = (x_q30 as u128 * 9_955_836_928u128) >> 32;
    let k = (scaled >> 24) as u32;
    let f = scaled & 0xFF_FFFF;
    let t = (f * 693_147_181u128) >> 24;
    let t2 = (t * t) >> 31;
    let t3 = ((t2 * t) >> 31) / 3;
    let p = (1u128 << 30) - t + t2 - t3;
    if k >= 31 {
        0
    } else {
        (p >> k) as u32
    }
}

fn build_scripted_holo_v13_bytes(chain_tokens: &[u32], extra_pieces: &[(u32, &[u8])]) -> Vec<u8> {
    assert!(
        chain_tokens.len() >= 2 && chain_tokens.len() <= 30,
        "chain length must fit within 32-dim block 0"
    );
    let vocab = 288usize;
    let dm = 128usize;
    let nl = 1usize;
    let nh = 2usize;
    let nkv = 1usize;
    let hd = 128usize;
    let ffn = 1024usize;
    let max_seq = 2048usize;

    let bos_id = 256u32;
    let eos_id = 257u32;
    let eot_id = 258u32;
    let think_start_id = 259u32;
    let think_end_id = 260u32;

    let mut raw = Vec::new();
    raw.extend_from_slice(HOLO_V13_MAGIC);
    raw.extend_from_slice(&HOLO_V13_VERSION.to_le_bytes());
    raw.extend_from_slice(&1u32.to_le_bytes());
    raw.extend_from_slice(&(vocab as u32).to_le_bytes());
    raw.extend_from_slice(&(dm as u32).to_le_bytes());
    raw.extend_from_slice(&(nl as u32).to_le_bytes());
    raw.extend_from_slice(&(nh as u32).to_le_bytes());
    raw.extend_from_slice(&(nkv as u32).to_le_bytes());
    raw.extend_from_slice(&(hd as u32).to_le_bytes());
    raw.extend_from_slice(&(ffn as u32).to_le_bytes());
    raw.extend_from_slice(&(max_seq as u32).to_le_bytes());
    raw.extend_from_slice(&bos_id.to_le_bytes());
    raw.extend_from_slice(&eos_id.to_le_bytes());
    raw.extend_from_slice(&eot_id.to_le_bytes());
    raw.extend_from_slice(&think_start_id.to_le_bytes());
    raw.extend_from_slice(&think_end_id.to_le_bytes());
    raw.extend_from_slice(&281_474_977i64.to_le_bytes());
    raw.extend_from_slice(&94_906_266i64.to_le_bytes());
    raw.extend_from_slice(&1_000_000u64.to_le_bytes());
    raw.extend_from_slice(&0b0_1111u32.to_le_bytes());

    let half = hd / 2;
    for _ in 0..(max_seq * half) {
        raw.extend_from_slice(&16_384i16.to_le_bytes());
    }
    for _ in 0..(max_seq * half) {
        raw.extend_from_slice(&0i16.to_le_bytes());
    }
    let exp_len = ((EXP_RANGE as usize) << 8) + 1;
    for _ in 0..exp_len {
        raw.extend_from_slice(&(1u32 << 30).to_le_bytes());
    }
    let silu_n = (1usize << (SILU_RANGE_LOG2 - SILU_STEP_LOG2 + 1)) + 1;
    let silu_shift = (30 + SILU_STEP_LOG2) as u32;
    let silu_q16_shift = (16 + SILU_STEP_LOG2) as u32;
    let half_s = (silu_n - 1) as i64 / 2;
    let one_q30 = 1u128 << 30;
    for i in 0..silu_n as i64 {
        let dx = i - half_s;
        let ax_q30 = dx.unsigned_abs() << silu_shift;
        let e_q30 = u128::from(exp_neg_q30_ref(ax_q30));
        let den = one_q30 + e_q30;
        let sig_q30 = if dx >= 0 {
            ((1u128 << 60) + (den >> 1)) / den
        } else {
            ((e_q30 << 30) + (den >> 1)) / den
        };
        let x_q16 = i128::from(dx) << silu_q16_shift;
        let silu_val = (x_q16 * (sig_q30 as i128)) >> 30;
        let clamped = silu_val.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32;
        raw.extend_from_slice(&clamped.to_le_bytes());
    }

    // Build UORTK13 tokenizer: 0..256 raw bytes, 256..261 specials, 261..288 extra pieces
    let added = [
        ("<|im_start|>", bos_id, true),
        ("<|im_end|>", eot_id, true),
        ("<think>", think_start_id, true),
        ("</think>", think_end_id, true),
    ];
    let mut tok = Vec::new();
    tok.extend_from_slice(HOLO_TOK13_MAGIC);
    tok.extend_from_slice(&(vocab as u32).to_le_bytes());
    tok.extend_from_slice(&0u32.to_le_bytes());
    tok.extend_from_slice(&(added.len() as u32).to_le_bytes());

    let mut raw_pieces: Vec<Vec<u8>> = Vec::with_capacity(vocab);
    for id in 0..vocab {
        if id < 256 {
            raw_pieces.push(vec![id as u8]);
        } else if let Some((_, piece)) = extra_pieces.iter().find(|&&(tid, _)| tid as usize == id) {
            raw_pieces.push(piece.to_vec());
        } else {
            raw_pieces.push(Vec::new());
        }
    }
    for p in &raw_pieces {
        tok.extend_from_slice(&(p.len() as u16).to_le_bytes());
        tok.extend_from_slice(p);
    }
    for id in 0..vocab {
        if id < 256 {
            tok.extend_from_slice(&1u16.to_le_bytes());
            let ch = if (id as u8).is_ascii_graphic() {
                id as u8
            } else {
                b'x'
            };
            tok.push(ch);
        } else {
            tok.extend_from_slice(&0u16.to_le_bytes());
        }
    }
    for (pat, aid, special) in added {
        tok.extend_from_slice(&aid.to_le_bytes());
        tok.push(u8::from(special));
        tok.extend_from_slice(&(pat.len() as u16).to_le_bytes());
        tok.extend_from_slice(pat.as_bytes());
    }
    raw.extend_from_slice(&(tok.len() as u32).to_le_bytes());
    raw.extend_from_slice(&tok);

    for v in 0..vocab {
        for w in 0..SIG_WORDS {
            let pat = 0x5555_5555_5555_5555u64 ^ ((v as u64).wrapping_mul(0x9e37_79b9) << (w * 3));
            raw.extend_from_slice(&pat.to_le_bytes());
        }
    }

    // Tied embedding matrix [vocab, dm] using orthogonal coordinates in block 0 (0..32)
    let embed_seed = 1u64;
    let embed_signs = rht_signs_packed_u32(dm, embed_seed);
    raw.extend_from_slice(&(vocab as u32).to_le_bytes());
    raw.extend_from_slice(&(dm as u32).to_le_bytes());
    raw.extend_from_slice(&embed_seed.to_le_bytes());
    raw.resize(raw.len() + vocab, (-20i8) as u8);
    for r in 0..vocab {
        for g in 0..(dm >> 5) {
            let mult: u16 = if g == 0 && chain_tokens.contains(&(r as u32)) {
                12
            } else {
                1
            };
            raw.extend_from_slice(&mult.to_le_bytes());
        }
    }
    for r in 0..vocab {
        let pos_opt = chain_tokens.iter().position(|&t| t == r as u32);
        for c in 0..dm {
            let w: i8 = match pos_opt {
                Some(idx) if c == idx => 100,
                None if c == 127 => -40,
                _ => 0,
            };
            raw.push(w as u8);
        }
    }

    // Final norm + Layer 0 norms (identity Q24 = 1 << 24)
    for _ in 0..dm {
        raw.extend_from_slice(&(1i32 << 24).to_le_bytes());
    }
    for _ in 0..(dm * 2 + hd * 2) {
        raw.extend_from_slice(&(1i32 << 24).to_le_bytes());
    }

    let push_i4 = |buf: &mut Vec<u8>,
                   rows: usize,
                   cols: usize,
                   seed: u64,
                   row_exp: i8,
                   mult_fn: &dyn Fn(usize, usize) -> u16,
                   val_fn: &dyn Fn(usize, usize) -> i8| {
        let groups = cols >> 5;
        buf.extend_from_slice(&(rows as u32).to_le_bytes());
        buf.extend_from_slice(&(cols as u32).to_le_bytes());
        buf.extend_from_slice(&seed.to_le_bytes());
        buf.resize(buf.len() + rows, row_exp as u8);
        for r in 0..rows {
            for g in 0..groups {
                buf.extend_from_slice(&mult_fn(r, g).to_le_bytes());
            }
        }
        for r in 0..rows {
            for g in 0..groups {
                for k in 0..16 {
                    let c0 = g * 32 + k;
                    let c1 = g * 32 + k + 16;
                    let q0 = (val_fn(r, c0).clamp(-8, 7) + 8) as u8;
                    let q1 = (val_fn(r, c1).clamp(-8, 7) + 8) as u8;
                    buf.push((q0 & 0x0F) | ((q1 & 0x0F) << 4));
                }
            }
        }
    };

    let q_dim = nh * hd;
    let kv_dim = nkv * hd;
    let zero_mult = |_: usize, _: usize| 0u16;
    let zero_val = |_: usize, _: usize| 0i8;
    // q_proj, k_proj, v_proj, o_proj are zero so attention does not mix tokens
    push_i4(&mut raw, q_dim, dm, 11, -18, &zero_mult, &zero_val);
    push_i4(&mut raw, kv_dim, dm, 12, -18, &zero_mult, &zero_val);
    push_i4(&mut raw, kv_dim, dm, 13, -18, &zero_mult, &zero_val);
    push_i4(&mut raw, dm, q_dim, 14, -18, &zero_mult, &zero_val);

    // gate_proj & up_proj: neuron (32 * s) fires iff embed coordinate s is active
    let steps = chain_tokens.len() - 1;
    let gate_mult = |r: usize, g: usize| -> u16 {
        if g == 0 && r.is_multiple_of(32) && (r / 32) < steps {
            100
        } else {
            0
        }
    };
    let gate_val = |r: usize, c: usize| -> i8 {
        if r.is_multiple_of(32) {
            let s = r / 32;
            if s < steps && c == s {
                return 7;
            }
        }
        0
    };
    push_i4(&mut raw, ffn, dm, embed_seed, -18, &gate_mult, &gate_val);
    push_i4(&mut raw, ffn, dm, embed_seed, -18, &gate_mult, &gate_val);

    // Precompute spatial dequant sign vector for each step's next token chain_tokens[s + 1]
    let down_seed = 17u64;
    let down_signs = rht_signs_packed_u32(ffn, down_seed);
    let mut step_spatial_signs = vec![[0i8; 32]; steps];
    for (s, spatial) in step_spatial_signs.iter_mut().enumerate().take(steps) {
        let next_idx = s + 1;
        let mut blk = [0i64; 32];
        blk[next_idx] = 100;
        int_fwht32_i64(&mut blk);
        let mask = embed_signs[0];
        for (c, out_sign) in spatial.iter_mut().enumerate() {
            let sv = if ((mask >> c) & 1) != 0 {
                blk[c]
            } else {
                -blk[c]
            };
            *out_sign = if sv >= 0 { 1 } else { -1 };
        }
    }

    let down_mult = |r: usize, g: usize| -> u16 {
        if r < 32 && g < steps {
            600
        } else {
            0
        }
    };
    let down_val = |r: usize, c: usize| -> i8 {
        let g = c >> 5;
        if r < 32 && g < steps {
            let sign_s: i8 = if (down_signs[g] & 1) != 0 { 1 } else { -1 };
            7 * sign_s * step_spatial_signs[g][r]
        } else {
            0
        }
    };
    push_i4(&mut raw, dm, ffn, down_seed, -12, &down_mult, &down_val);

    raw
}

#[test]
fn foundry_turn_begin_validates_empty_busy_and_unmounted_errors() {
    let mut engine: UorWasmStudio = StudioEngine::new_default();

    // 1. Empty string and empty prompt JSON -> EmptyPrompt
    let empty_raw: Value = serde_json::from_str(&engine.holo_begin_foundry_turn("")).unwrap();
    assert_eq!(empty_raw["status"], "error");
    assert_eq!(empty_raw["error_code"], "EmptyPrompt");

    let empty_json: Value =
        serde_json::from_str(&engine.holo_begin_foundry_turn(r#"{"prompt":"   "}"#)).unwrap();
    assert_eq!(empty_json["status"], "error");
    assert_eq!(empty_json["error_code"], "EmptyPrompt");

    // Invalid JSON -> InvalidPayload; TurnNotFound when no active session
    let bad_json: Value =
        serde_json::from_str(&engine.holo_begin_foundry_turn("{not valid json")).unwrap();
    assert_eq!(bad_json["status"], "error");
    assert_eq!(bad_json["error_code"], "InvalidPayload");

    let step_missing: Value = serde_json::from_str(&engine.holo_step_foundry_turn(42)).unwrap();
    assert_eq!(step_missing["status"], "error");
    assert_eq!(step_missing["error_code"], "TurnNotFound");

    let retired: Value = serde_json::from_str(&engine.holo_begin_foundry_turn(
        r#"{"prompt":"hi","selected_model":"uor-r4.recompiled-v12-agent"}"#,
    ))
    .unwrap();
    assert_eq!(retired["status"], "error");
    assert_eq!(retired["error_code"], "RefusedRetiredScaffoldMode");

    // 2. No Holo model mounted -> ModelNotMounted
    let unmounted: Value = serde_json::from_str(
        &engine.holo_begin_foundry_turn(r#"{"prompt":"Add a counter button"}"#),
    )
    .unwrap();
    assert_eq!(unmounted["status"], "error");
    assert_eq!(unmounted["error_code"], "ModelNotMounted");

    // 3. Mount scripted Holo model and test BusyTurn guard
    let bytes = build_scripted_holo_v13_bytes(&[10, 65, 66, 67, 68, 69, 257], &[]);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes)
        .expect("mount scripted holo model");

    let first: Value = serde_json::from_str(
        &engine.holo_begin_foundry_turn(r#"{"prompt":"Build a counter app"}"#),
    )
    .unwrap();
    assert_eq!(first["status"], "ready");
    let turn_id = first["turn_id"].as_u64().expect("turn_id u64");
    assert!(turn_id > 0);
    assert_eq!(
        engine.active_foundry_turn_max_new_tokens(turn_id),
        Some(1536)
    );

    // Calling begin while turn is active returns BusyTurn without clobbering active turn
    let busy: Value =
        serde_json::from_str(&engine.holo_begin_foundry_turn(r#"{"prompt":"Concurrent request"}"#))
            .unwrap();
    assert_eq!(busy["status"], "error");
    assert_eq!(busy["error_code"], "BusyTurn");

    // Calling step with wrong turn_id returns TurnNotFound without clobbering active turn
    let mismatch: Value =
        serde_json::from_str(&engine.holo_step_foundry_turn(turn_id + 99)).unwrap();
    assert_eq!(mismatch["status"], "error");
    assert_eq!(mismatch["error_code"], "TurnNotFound");
    let valid_step: Value = serde_json::from_str(&engine.holo_step_foundry_turn(turn_id)).unwrap();
    assert_eq!(valid_step["status"], "streaming");
    assert_eq!(valid_step["model_id"], "uor-r4.qwen3-0.6b-holo");
}

#[test]
fn foundry_prompt_chunking_utf8_tool_stop_and_cancel_r4_56() {
    // Verify BUILD_SYSTEM_PREFIX and format_foundry_system_prompt directly
    assert!(BUILD_SYSTEM_PREFIX.contains("<prism-write"));
    assert!(BUILD_SYSTEM_PREFIX.contains("<prism-delete"));
    assert!(BUILD_SYSTEM_PREFIX.contains("<prism-rename"));
    assert!(BUILD_SYSTEM_PREFIX.contains("<prism-add-dependency"));

    let ws_files = vec![
        FoundryWorkspaceFile {
            path: "src/App.tsx".to_string(),
            content: "export default function App() { return <div>Hi</div>; }".to_string(),
            sha256: "abc123".to_string(),
        },
        FoundryWorkspaceFile {
            path: "package.json".to_string(),
            content: "{\"name\":\"demo\"}".to_string(),
            sha256: "def456".to_string(),
        },
    ];
    let sys = format_foundry_system_prompt(&ws_files, Some("src/App.tsx"));
    assert!(sys.contains("<prism-write"));
    assert!(sys.contains("<tool_call>"));
    assert!(sys.contains("mvm_exec"));
    assert!(sys.contains("src/App.tsx"));
    assert!(sys.contains("package.json"));

    let mut engine = StudioEngine::new_default();

    // Scripted model 1: splits 4-byte UTF-8 "🚀" across token 261 + 262, then emits 6 ASCII tokens -> EOS
    // Total 8 tokens = two 4-token slices!
    let extra_prose: &[(u32, &[u8])] = &[
        (261, &[0xF0, 0x9F]),
        (262, &[0x9A, 0x80]),
        (263, b" "),
        (264, b"ok"),
        (265, b"! "),
        (266, b"v1"),
        (267, b" "),
        (268, b"done"),
    ];
    let bytes_prose = build_scripted_holo_v13_bytes(
        &[10, 261, 262, 263, 264, 265, 266, 267, 268, 257],
        extra_prose,
    );
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_prose)
        .expect("mount prose model");

    let begin_payload = serde_json::json!({
        "prompt": "Update src/App.tsx",
        "workspace_files": ws_files.iter().map(|f| serde_json::json!({
            "path": f.path, "content": f.content, "sha256": f.sha256
        })).collect::<Vec<_>>(),
        "active_file": "src/App.tsx",
        "selected_model": "uor-r4.qwen3-0.6b-holo",
        "tool_results": []
    })
    .to_string();
    let begin: Value =
        serde_json::from_str(&engine.holo_begin_foundry_turn(&begin_payload)).unwrap();
    assert_eq!(begin["status"], "ready");
    let tid1 = begin["turn_id"].as_u64().unwrap();
    let session_sys = engine
        .active_foundry_turn_system_prompt(tid1)
        .expect("active session system prompt");
    assert!(session_sys.contains("<prism-write"));
    assert!(session_sys.contains("<tool_call>"));
    assert!(session_sys.contains("mvm_exec"));

    // Step 1: at most 4 tokens (261, 262, 263, 264) -> "🚀 ok"
    let s1: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid1)).unwrap();
    assert_eq!(s1["status"], "streaming");
    assert!(s1["tokens_emitted"].as_u64().unwrap() <= 4);
    assert!(s1["tokens_emitted"].as_u64().unwrap() >= 1);
    assert_eq!(s1["delta"], "🚀 ok");
    assert!(!s1["delta"].as_str().unwrap().contains('\u{FFFD}'));
    assert!(s1["tok_per_sec"].as_f64().is_some());
    assert!(s1["geometric_metrics"].is_object());

    // Step 2: remaining 4 tokens (265, 266, 267, 268) -> "! v1 done", then EOS or next step
    let s2: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid1)).unwrap();
    assert!(s2["tokens_emitted"].as_u64().unwrap() <= 8);
    let mut full_text = format!(
        "{}{}",
        s1["delta"].as_str().unwrap(),
        s2["delta"].as_str().unwrap()
    );
    let mut final_step = s2;
    while final_step["status"] == "streaming" {
        let prev_toks = final_step["tokens_emitted"].as_u64().unwrap();
        final_step = serde_json::from_str(&engine.holo_step_foundry_turn(tid1)).unwrap();
        let next_toks = final_step["tokens_emitted"].as_u64().unwrap();
        assert!(next_toks - prev_toks <= 4, "step emitted > 4 tokens");
        full_text.push_str(final_step["delta"].as_str().unwrap_or(""));
    }
    assert_eq!(final_step["status"], "done");
    assert_eq!(full_text, "🚀 ok! v1 done");
    assert!(final_step["receipt_sha256"]
        .as_str()
        .is_some_and(|s| s.len() == 64));
    assert!(final_step["weights_kappa"]
        .as_str()
        .is_some_and(|s| s.starts_with("sha256:")));

    // Scripted model 2: emits <tool_call>{"name":"mvm_exec","arguments":{"cmd":"pwd"}}</tool_call> across >4 tokens
    let extra_tool: &[(u32, &[u8])] = &[
        (261, b"Checking: "),
        (262, b"<tool_call>"),
        (263, b"{\"name\":\"mvm_exec\","),
        (264, b"\"arguments\":{\"cmd\":\"pwd\"}}"),
        (265, b"</tool_call>"),
        (266, b"after"),
    ];
    let bytes_tool =
        build_scripted_holo_v13_bytes(&[10, 261, 262, 263, 264, 265, 266, 257], extra_tool);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_tool)
        .expect("mount tool model");

    let begin2: Value =
        serde_json::from_str(&engine.holo_begin_foundry_turn(&begin_payload)).unwrap();
    assert_eq!(begin2["status"], "ready");
    let tid2 = begin2["turn_id"].as_u64().unwrap();

    // First 4 tokens: "Checking: <tool_call>{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"pwd\"}}"
    let t1: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid2)).unwrap();
    assert_eq!(t1["status"], "streaming");
    // Next slice sees </tool_call> stop-sequence and immediately returns needs_tool without emitting "after"
    let t2: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid2)).unwrap();
    assert_eq!(t2["status"], "needs_tool");
    assert_eq!(t2["tool_call"]["name"], "mvm_exec");
    assert_eq!(t2["tool_call"]["cmd"], "pwd");
    assert!(!t2["delta"].as_str().unwrap_or("").contains("after"));

    // Continuation turn with tool_results + cancel test on immediately following step
    let tool_res = vec![FoundryToolResult {
        cmd: "pwd".to_string(),
        exit_code: 0,
        stdout: "/workspace\n".to_string(),
        stderr: String::new(),
        changed_files: vec![],
    }];
    let cont_payload = serde_json::json!({
        "prompt": "Update src/App.tsx",
        "workspace_files": [],
        "active_file": null,
        "selected_model": "uor-r4.qwen3-0.6b-holo",
        "tool_results": tool_res,
        "max_new_tokens": 32
    })
    .to_string();
    let begin3: Value =
        serde_json::from_str(&engine.holo_begin_foundry_turn(&cont_payload)).unwrap();
    assert_eq!(begin3["status"], "ready");
    let tid3 = begin3["turn_id"].as_u64().unwrap();

    let c1: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid3)).unwrap();
    assert_eq!(c1["status"], "streaming");
    let cancel_ack: Value = serde_json::from_str(&engine.holo_cancel_foundry_turn(tid3)).unwrap();
    assert_eq!(cancel_ack["status"], "cancelled");
    assert_eq!(cancel_ack["turn_id"], tid3);

    // Immediately following step honors cancellation cleanly
    let c2: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid3)).unwrap();
    assert_eq!(c2["status"], "cancelled");

    // Cancelling unknown turn_id returns not_found
    let nf: Value = serde_json::from_str(&engine.holo_cancel_foundry_turn(999_999)).unwrap();
    assert_eq!(nf["status"], "not_found");

    // Subsequent turn starts and completes cleanly with uncorrupted state
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_prose)
        .expect("remount prose model");
    let begin4: Value =
        serde_json::from_str(&engine.holo_begin_foundry_turn(&begin_payload)).unwrap();
    assert_eq!(begin4["status"], "ready");
    let tid4 = begin4["turn_id"].as_u64().unwrap();
    let s4: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid4)).unwrap();
    assert_eq!(s4["delta"], "🚀 ok");
}

#[test]
fn test_foundry_agent_stepper_r4_56() {
    // Bound max 6 tool iterations enforced when >= 6 tool_results already executed
    let mut engine = StudioEngine::new_default();
    let extra_tool: &[(u32, &[u8])] = &[
        (261, b"<tool_call>"),
        (
            262,
            b"{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"ls\"}}",
        ),
        (263, b"</tool_call>"),
    ];
    let bytes_tool = build_scripted_holo_v13_bytes(&[10, 261, 262, 263, 257], extra_tool);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_tool)
        .expect("mount tool model");

    let six_results: Vec<Value> = (0..6)
        .map(|i| {
            serde_json::json!({
                "cmd": format!("echo {i}"),
                "exit_code": 0,
                "stdout": format!("{i}\n"),
                "stderr": "",
                "changed_files": []
            })
        })
        .collect();
    let payload = serde_json::json!({
        "prompt": "Inspect workspace",
        "workspace_files": [],
        "tool_results": six_results
    })
    .to_string();
    let begin: Value = serde_json::from_str(&engine.holo_begin_foundry_turn(&payload)).unwrap();
    assert_eq!(begin["status"], "ready");
    let tid = begin["turn_id"].as_u64().unwrap();
    let mut step: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid)).unwrap();
    while step["status"] == "streaming" {
        step = serde_json::from_str(&engine.holo_step_foundry_turn(tid)).unwrap();
    }
    assert_ne!(
        step["status"], "needs_tool",
        "tool iterations must be bounded to max 6"
    );
    assert_eq!(step["status"], "done");
}

#[test]
fn foundry_prompt_pinning_escaping_malformed_prior_tool_and_native_q31_r4_56() {
    // 1. XML attribute escaping & Unicode scalar boundary snippet truncation
    let mut unicode_ws = Vec::new();
    for i in 0..16 {
        unicode_ws.push(FoundryWorkspaceFile {
            path: format!("src/\"bad_{i}\"<tag>&.tsx"),
            content: "αβγ🚀".repeat(400),
            sha256: format!("sha\"<&>{i}"),
        });
    }
    let sys = format_foundry_system_prompt(&unicode_ws, Some("src/\"bad_0\"<tag>&.tsx"));
    assert!(sys.starts_with(BUILD_SYSTEM_PREFIX));
    assert!(sys.contains("path=\"src/&quot;bad_0&quot;&lt;tag&gt;&amp;.tsx\""));
    assert!(sys.contains("sha256=\"sha&quot;&lt;&amp;&gt;0\""));

    let mut engine = StudioEngine::new_default();

    // 2. Mount prose model and check front-pinning + max_new_tokens=1 decode telemetry frame
    let extra_prose: &[(u32, &[u8])] = &[(261, b"ok"), (262, b"!")];
    let bytes_prose = build_scripted_holo_v13_bytes(&[10, 261, 262, 257], extra_prose);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_prose)
        .expect("mount prose model");
    let big_payload = serde_json::json!({
        "prompt": "Check front pinning",
        "workspace_files": unicode_ws,
        "active_file": "src/\"bad_0\"<tag>&.tsx",
        "max_new_tokens": 1
    })
    .to_string();
    let b1: Value = serde_json::from_str(&engine.holo_begin_foundry_turn(&big_payload)).unwrap();
    assert_eq!(b1["status"], "ready");
    let tid1 = b1["turn_id"].as_u64().unwrap();
    let kv_toks = engine
        .holo_model_kv_prefix_tokens("uor-r4.qwen3-0.6b-holo")
        .expect("kv tokens");
    let kv_prefix_bytes: Vec<u8> = kv_toks
        .iter()
        .take(64)
        .filter(|&&id| id < 256)
        .map(|&id| id as u8)
        .collect();
    let kv_head = String::from_utf8_lossy(&kv_prefix_bytes);
    assert!(
        kv_toks.first().copied().unwrap_or(0) >= 256,
        "KV must begin with <|im_start|> special token, got: {:?}",
        kv_toks.first()
    );
    assert!(
        kv_head.starts_with("system\n\n<role> You are Prism"),
        "system prefix must stay pinned at KV start, got: {kv_head}"
    );
    let kv_tail_bytes: Vec<u8> = kv_toks
        .iter()
        .rev()
        .take(64)
        .copied()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .filter(|&id| id < 256)
        .map(|id| id as u8)
        .collect();
    let kv_tail = String::from_utf8_lossy(&kv_tail_bytes);
    assert!(
        kv_tail
            .trim_end()
            .ends_with("user\nCheck front pinning\nassistant")
            && kv_tail.ends_with("\n\n"),
        "user prompt and assistant start tag must stay pinned at KV end, got: {kv_tail:?}"
    );

    let s1: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid1)).unwrap();
    assert_eq!(s1["status"], "done");
    assert_eq!(s1["model_id"], "uor-r4.qwen3-0.6b-holo");
    assert_eq!(s1["tokens_emitted"], 1);
    assert!(
        engine.last_foundry_observatory_frame_count() >= 2,
        "max_new_tokens=1 must emit both prefill and decode observatory frames"
    );

    // 3. Earlier malformed / non-mvm_exec <tool_call> does NOT blind later valid mvm_exec, plus continuation -> done
    let extra_multi: &[(u32, &[u8])] = &[
        (261, b"<tool_call>{bad json</tool_call> "),
        (
            262,
            b"<tool_call>{\"name\":\"noop\",\"arguments\":{}}</tool_call> ",
        ),
        (263, b"<tool_call>{\"name\":\"mvm_exec\","),
        (264, b"\"arguments\":{\"cmd\":\"uname -a\"}}"),
        (265, b"</tool_call>"),
        (266, b"ignored_after_stop"),
    ];
    let bytes_multi =
        build_scripted_holo_v13_bytes(&[10, 261, 262, 263, 264, 265, 266, 257], extra_multi);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_multi)
        .expect("mount multi tool model");
    let b2: Value =
        serde_json::from_str(&engine.holo_begin_foundry_turn(r#"{"prompt":"Inspect kernel"}"#))
            .unwrap();
    let tid2 = b2["turn_id"].as_u64().unwrap();
    let t_step1: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid2)).unwrap();
    assert_eq!(t_step1["status"], "streaming");
    assert_eq!(t_step1["model_id"], "uor-r4.qwen3-0.6b-holo");
    let t_step2: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid2)).unwrap();
    assert_eq!(t_step2["status"], "needs_tool");
    assert_eq!(t_step2["model_id"], "uor-r4.qwen3-0.6b-holo");
    assert_eq!(t_step2["tool_call"]["name"], "mvm_exec");
    assert_eq!(t_step2["tool_call"]["cmd"], "uname -a");

    // Feed tool_results into continuation turn with prose model and complete cleanly
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_prose)
        .expect("mount continuation prose");
    let cont = serde_json::json!({
        "prompt": "Inspect kernel",
        "tool_results": [{
            "cmd": "uname -a",
            "exit_code": 0,
            "stdout": "Linux riscv64\n",
            "stderr": "",
            "changed_files": []
        }]
    })
    .to_string();
    let b3: Value = serde_json::from_str(&engine.holo_begin_foundry_turn(&cont)).unwrap();
    let tid3 = b3["turn_id"].as_u64().unwrap();
    let done3: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid3)).unwrap();
    assert_eq!(done3["status"], "done");
    assert_eq!(done3["delta"], "ok!");

    // 4. Explicit uor-r4.native-q31-d85 steps incrementally (<=4 tokens per call) and honors max_new_tokens
    let n_begin: Value = serde_json::from_str(
        &engine.holo_begin_foundry_turn(
            r#"{"prompt":"Hello native Q31","selected_model":"uor-r4.native-q31-d85","max_new_tokens":6}"#,
        ),
    )
    .unwrap();
    assert_eq!(n_begin["status"], "ready");
    assert_eq!(n_begin["model_id"], "uor-r4.native-q31-d85");
    assert_eq!(n_begin["max_new_tokens"], 6);
    let n_tid = n_begin["turn_id"].as_u64().unwrap();
    let n1: Value = serde_json::from_str(&engine.holo_step_foundry_turn(n_tid)).unwrap();
    assert_eq!(n1["model_id"], "uor-r4.native-q31-d85");
    assert_eq!(n1["status"], "streaming");
    assert_eq!(n1["tokens_emitted"], 4);
    let n2: Value = serde_json::from_str(&engine.holo_step_foundry_turn(n_tid)).unwrap();
    assert_eq!(n2["model_id"], "uor-r4.native-q31-d85");
    assert_eq!(n2["status"], "done");
    assert_eq!(n2["tokens_emitted"], 6);
}

#[test]
fn foundry_step_emits_incremental_observatory_frames_and_prefix_reuse_r4_56() {
    let mut engine = StudioEngine::new_default();
    let extra: &[(u32, &[u8])] = &[
        (261, b"one "),
        (262, b"two "),
        (263, b"three "),
        (264, b"four "),
        (265, b"five "),
        (266, b"six"),
    ];
    let bytes = build_scripted_holo_v13_bytes(&[10, 261, 262, 263, 264, 265, 266, 257], extra);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes)
        .expect("mount 6-token scripted model");

    let begin_payload = serde_json::json!({
        "prompt": "Stream incremental observatory frames",
        "workspace_files": [],
        "max_new_tokens": 6
    })
    .to_string();
    let b1: Value = serde_json::from_str(&engine.holo_begin_foundry_turn(&begin_payload)).unwrap();
    assert_eq!(b1["status"], "ready");
    let tid1 = b1["turn_id"].as_u64().unwrap();

    // Slice 1 (tokens 1..=4): must include prefill + decode(step=1) frames incrementally
    let s1: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid1)).unwrap();
    assert_eq!(s1["status"], "streaming");
    assert!(
        s1["prefix_reused_tokens"].is_u64(),
        "step JSON must include prefix_reused_tokens: {s1}"
    );
    let frames1 = s1["observatory_frames"]
        .as_array()
        .expect("streaming step must include observatory_frames array");
    assert!(
        frames1.len() >= 2,
        "first slice must include prefill + decode frame, got {frames1:?}"
    );
    assert_eq!(frames1[0]["phase"], "prefill");
    assert_eq!(frames1[1]["phase"], "decode");
    assert!(
        s1["frame"].is_object(),
        "step JSON must include latest frame object"
    );

    // Slice 2 (tokens 5..=6 -> done): must include ONLY newly recorded decode frames (no prefill duplicate)
    let s2: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid1)).unwrap();
    assert_eq!(s2["status"], "done");
    let frames2 = s2["observatory_frames"]
        .as_array()
        .expect("done step must include observatory_frames array");
    assert!(
        !frames2.is_empty(),
        "second slice must emit new decode frames"
    );
    assert!(
        frames2.iter().all(|f| f["phase"] == "decode"),
        "second slice must not re-emit prefill frame: {frames2:?}"
    );
    assert_eq!(
        frames1.len() + frames2.len(),
        engine.last_foundry_observatory_frame_count(),
        "incremental step frames must partition total recorded frames cleanly"
    );
}
