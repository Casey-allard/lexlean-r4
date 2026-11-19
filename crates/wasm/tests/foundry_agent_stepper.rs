//! Integration tests for R4-56: Async 4-Token Foundry Turn Stepper (`FoundryTurnSession`),
//! Foundry `<prism-write>` + `<tool_call>` prompt assembly, UTF-8 incremental flush, and cancel/busy/empty exits.

use lexlean_r4_wasm::holo_geometric_runtime::{
    compute_top6_prob_permille, flush_incremental_utf8, int_fwht32_i64, rht_signs_packed_u32,
    HOLO_TOK13_MAGIC, HOLO_V13_MAGIC, HOLO_V13_VERSION,
};
use lexlean_r4_wasm::lexlean_generated::r4_studio_frame::{
    admit_live_token_frame, admit_studio_tool, ComposerToolPolicy, LiveFrameVerdict,
};
use lexlean_r4_wasm::native_q31_lane::NativeQ31Model;
use lexlean_r4_wasm::{
    detect_studio_tool_call, format_foundry_system_prompt, format_studio_system_parts,
    FoundryToolResult, FoundryWorkspaceFile, StudioEngine, UorWasmStudio, BUILD_SYSTEM_PREFIX,
    STUDIO_SELF_KNOWLEDGE_PREFIX,
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

#[test]
fn test_studio_live_token_frame_prob_sum_and_native_q31_honesty_r4_60() {
    // 1. Pure integer shift-LUT top-6 probability permille helper (zero floats, sums to 1000, monotonic)
    let exp_len = ((EXP_RANGE as usize) << 8) + 1;
    let exp_table: Vec<u32> = (0..exp_len)
        .map(|i| exp_neg_q30_ref((i as u64) << 22))
        .collect();

    assert!(compute_top6_prob_permille(&exp_table, &[]).is_empty());

    let single = compute_top6_prob_permille(&exp_table, &[(42, 7 << 16)]);
    assert_eq!(single, vec![1000]);

    let equal = compute_top6_prob_permille(
        &exp_table,
        &[
            (1, 3 << 16),
            (2, 3 << 16),
            (3, 3 << 16),
            (4, 3 << 16),
            (5, 3 << 16),
            (6, 3 << 16),
        ],
    );
    assert_eq!(equal.len(), 6);
    assert_eq!(equal.iter().map(|&p| u32::from(p)).sum::<u32>(), 1000);
    assert!(equal.windows(2).all(|w| w[0] >= w[1]));

    let ranked_logits = [
        (10, 6 << 16),
        (11, 5 << 16),
        (12, 4 << 16),
        (13, 3 << 16),
        (14, 2 << 16),
        (15, 1 << 16),
    ];
    let probs_lut = compute_top6_prob_permille(&exp_table, &ranked_logits);
    let probs_direct = compute_top6_prob_permille(&[], &ranked_logits);
    for probs in [&probs_lut, &probs_direct] {
        assert_eq!(probs.len(), 6);
        assert_eq!(probs.iter().map(|&p| u32::from(p)).sum::<u32>(), 1000);
        assert!(probs.windows(2).all(|w| w[0] >= w[1]));
        assert!(probs[0] > probs[5]);
    }

    // 2. Direct NativeQ31Model::step_next_token_with_frame + Foundry stepper honesty check
    let q31_model = NativeQ31Model::from_bytes(include_bytes!("../src/native_q31_d85.uorq31"))
        .expect("embedded native_q31_d85.uorq31 valid");
    let q31_prompt_toks = q31_model.encode_bytes(b"Explain pure integer Q31 frames");
    let (direct_tok, direct_cksum, direct_frame) =
        q31_model.step_next_token_with_frame(&q31_prompt_toks, q31_prompt_toks.len(), 0);
    assert!(direct_tok.is_some());
    assert_ne!(direct_cksum, 0);
    assert!(!direct_frame.has_geometric_shortlist);
    assert_eq!(direct_frame.min_hamming, 0);
    assert_eq!(direct_frame.shortlist_count, 0);
    assert!(direct_frame.layer_flock_spans.is_empty());
    assert_eq!(direct_frame.layer_residual_mag.len(), 4);
    assert_eq!(direct_frame.top_candidates.len(), 6);
    assert!(direct_frame
        .top_candidates
        .iter()
        .all(|c| c.hamming_d.is_none()));
    assert_eq!(
        direct_frame
            .top_candidates
            .iter()
            .map(|c| u32::from(c.prob_permille))
            .sum::<u32>(),
        1000
    );

    // 3. NativeQ31Model honesty via shared Foundry stepper with `model: "uor-r4.native-q31-d85"` and `live_hud: true`
    let mut engine = StudioEngine::new_default();
    let begin_payload = serde_json::json!({
        "prompt": "Explain pure integer Q31 frames",
        "model": "uor-r4.native-q31-d85",
        "max_new_tokens": 5,
        "live_hud": true
    })
    .to_string();
    let begin: Value =
        serde_json::from_str(&engine.holo_begin_foundry_turn(&begin_payload)).unwrap();
    assert_eq!(begin["status"], "ready");
    assert_eq!(begin["model_id"], "uor-r4.native-q31-d85");
    let turn_id = begin["turn_id"].as_u64().unwrap();

    let mut all_frames: Vec<Value> = Vec::new();
    let mut step = serde_json::from_str::<Value>(&engine.holo_step_foundry_turn(turn_id)).unwrap();
    let mut prev_tokens = 0u64;
    loop {
        let cur_tokens = step["tokens_emitted"].as_u64().unwrap();
        assert!(
            cur_tokens - prev_tokens <= 2,
            "live_hud=true default slice_tokens must be 2, got delta={}",
            cur_tokens - prev_tokens
        );
        prev_tokens = cur_tokens;
        let frames = step["observatory_frames"]
            .as_array()
            .expect("native-q31 live_hud step must emit observatory_frames");
        assert!(
            !frames.is_empty(),
            "native-q31 live_hud slice must emit per-token frames"
        );
        all_frames.extend(frames.iter().cloned());
        if step["status"] != "streaming" {
            break;
        }
        step = serde_json::from_str::<Value>(&engine.holo_step_foundry_turn(turn_id)).unwrap();
    }
    assert_eq!(step["status"], "done");
    assert_eq!(all_frames.len(), 5, "must emit one frame per decoded token");

    for frame in &all_frames {
        assert_eq!(frame["backend"], "native-q31-d85");
        assert_eq!(frame["has_geometric_shortlist"], false);
        assert_eq!(frame["min_hamming"], 0);
        assert_eq!(frame["shortlist_count"], 0);
        assert!(frame["layer_flock_spans"]
            .as_array()
            .is_some_and(Vec::is_empty));
        assert_eq!(
            frame["layer_residual_mag"].as_array().map(Vec::len),
            Some(4),
            "NativeQ31 must expose 4 real layer magnitudes"
        );
        assert!(frame["tps_x10"].is_u64());
        assert!(frame["ttft_ms"].is_u64());

        let cands = frame["top_candidates"]
            .as_array()
            .expect("NativeQ31 frame must carry real top_candidates");
        assert!(!cands.is_empty() && cands.len() <= 6);
        assert!(cands[0]["prob_permille"].as_u64().unwrap() > 0);
        let perm_sum: u64 = cands
            .iter()
            .map(|c| c["prob_permille"].as_u64().unwrap())
            .sum();
        assert_eq!(perm_sum, 1000);
        let any_hamming = cands.iter().any(|c| !c["hamming_d"].is_null());
        assert!(
            !any_hamming,
            "NativeQ31 must report hamming_d == None on every candidate"
        );
        assert_eq!(
            admit_live_token_frame(
                frame["has_geometric_shortlist"].as_bool().unwrap(),
                any_hamming,
                0,
                100_000,
            ),
            LiveFrameVerdict::FrameAdmitted
        );
    }
}

#[test]
fn test_studio_stepper_splits_4byte_utf8_and_cancels_mid_prism_write_r4_60() {
    // 1. Feed raw 4-byte UTF-8 [0xF0, 0x9F, 0xA6, 0x80] ('🦀') across 2-byte flush calls
    let raw_crab = [0xF0u8, 0x9Fu8, 0xA6u8, 0x80u8];
    let mut emitted_raw = 0usize;
    let first_half = flush_incremental_utf8(&raw_crab[..2], &mut emitted_raw, false);
    assert_eq!(first_half, "");
    assert_eq!(emitted_raw, 0);
    assert!(!first_half.contains('\u{FFFD}'));
    let second_half = flush_incremental_utf8(&raw_crab, &mut emitted_raw, false);
    assert_eq!(second_half, "🦀");
    assert_eq!(emitted_raw, 4);
    assert!(!second_half.contains('\u{FFFD}'));

    // Also test 2-token live_hud slice splitting 4-byte UTF-8 across two steps cleanly
    let mut engine = StudioEngine::new_default();
    let utf8_pieces: &[(u32, &[u8])] = &[
        (261, &[0xF0]),
        (262, &[0x9F]),
        (263, &[0xA6]),
        (264, &[0x80]),
    ];
    let bytes_utf8 = build_scripted_holo_v13_bytes(&[10, 261, 262, 263, 264, 257], utf8_pieces);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_utf8)
        .expect("mount 4-byte split utf8 model");

    let b_utf8: Value = serde_json::from_str(
        &engine.holo_begin_foundry_turn(
            &serde_json::json!({
                "prompt": "Emit crab emoji",
                "selected_model": "uor-r4.qwen3-0.6b-holo",
                "surface_mode": "chat",
                "live_hud": true
            })
            .to_string(),
        ),
    )
    .unwrap();
    let tid_u = b_utf8["turn_id"].as_u64().unwrap();
    let chat_sys = engine
        .active_foundry_turn_system_prompt(tid_u)
        .expect("chat surface system prompt");
    assert!(
        !chat_sys.contains("<prism-write"),
        "surface_mode='chat' must use compact chat prompt without <prism-write rules"
    );
    let u1: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid_u)).unwrap();
    assert_eq!(u1["status"], "streaming");
    assert_eq!(u1["tokens_emitted"], 2);
    assert_eq!(u1["delta"], "");
    assert!(!u1["delta"].as_str().unwrap().contains('\u{FFFD}'));
    let u2: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid_u)).unwrap();
    assert_eq!(u2["delta"], "🦀");
    assert!(!u2["delta"].as_str().unwrap().contains('\u{FFFD}'));
    if u2["status"] == "streaming" {
        let u3: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid_u)).unwrap();
        assert_eq!(u3["status"], "done");
    }

    // 2. Cancel mid-<prism-write path="src/app.rs">incomplete and verify KV restores to kv_checkpoint_len
    let prism_pieces: &[(u32, &[u8])] = &[
        (261, b"<prism-write path=\"src/app.rs\">"),
        (262, b"incomplete"),
        (263, b"</prism-write>"),
    ];
    let bytes_prism = build_scripted_holo_v13_bytes(&[10, 261, 262, 263, 257], prism_pieces);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_prism)
        .expect("mount prism-write cancel model");

    let b_cancel: Value = serde_json::from_str(
        &engine.holo_begin_foundry_turn(
            &serde_json::json!({
                "prompt": "Write src/app.rs",
                "selected_model": "uor-r4.qwen3-0.6b-holo",
                "surface_mode": "studio",
                "composer_mode": "agent",
                "live_hud": true
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(b_cancel["status"], "ready");
    let tid_c = b_cancel["turn_id"].as_u64().unwrap();
    let expected_checkpoint = engine
        .active_foundry_turn_kv_checkpoint_len(tid_c)
        .expect("kv_checkpoint_len present");
    let prefill_kv_len = engine
        .holo_model_kv_prefix_tokens("uor-r4.qwen3-0.6b-holo")
        .unwrap()
        .len();
    assert!(prefill_kv_len > expected_checkpoint);

    let step_partial: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid_c)).unwrap();
    assert_eq!(step_partial["status"], "streaming");
    assert_eq!(
        step_partial["delta"],
        "<prism-write path=\"src/app.rs\">incomplete"
    );
    let mid_kv_len = engine
        .holo_model_kv_prefix_tokens("uor-r4.qwen3-0.6b-holo")
        .unwrap()
        .len();
    assert_eq!(mid_kv_len, prefill_kv_len + 2);

    let cancel_ack: Value = serde_json::from_str(&engine.holo_cancel_foundry_turn(tid_c)).unwrap();
    assert_eq!(cancel_ack["status"], "cancelled");
    assert_eq!(
        engine
            .holo_model_kv_prefix_tokens("uor-r4.qwen3-0.6b-holo")
            .unwrap()
            .len(),
        expected_checkpoint
    );
    let post_cancel: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid_c)).unwrap();
    assert_eq!(post_cancel["status"], "cancelled");
}

#[test]
fn test_studio_live_hud_per_token_emission_and_slice_clamp_r4_60() {
    let mut engine = StudioEngine::new_default();
    let extra: &[(u32, &[u8])] = &[
        (261, b"a"),
        (262, b"b"),
        (263, b"c"),
        (264, b"d"),
        (265, b"e"),
    ];
    let bytes = build_scripted_holo_v13_bytes(&[10, 261, 262, 263, 264, 265, 257], extra);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes)
        .expect("mount 5-token holo model");

    // slice_tokens=99 clamps to 4; live_hud=true emits one decode frame per token
    let begin: Value = serde_json::from_str(
        &engine.holo_begin_foundry_turn(
            &serde_json::json!({
                "prompt": "Count tokens live",
                "selected_model": "uor-r4.qwen3-0.6b-holo",
                "live_hud": true,
                "slice_tokens": 99
            })
            .to_string(),
        ),
    )
    .unwrap();
    let tid = begin["turn_id"].as_u64().unwrap();
    let s1: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid)).unwrap();
    assert_eq!(s1["status"], "streaming");
    assert_eq!(s1["tokens_emitted"], 4);
    let frames1 = s1["observatory_frames"].as_array().unwrap();
    // 1 prefill frame + 4 decode frames = 5 frames in first slice!
    assert_eq!(frames1.len(), 5);
    assert_eq!(frames1[0]["phase"], "prefill");
    for df in &frames1[1..] {
        assert_eq!(df["phase"], "decode");
        assert_eq!(df["has_geometric_shortlist"], true);
        let cands = df["top_candidates"].as_array().unwrap();
        assert!(!cands.is_empty());
        assert!(cands.iter().all(|c| c["hamming_d"].is_u64()));
        let sum_perm: u64 = cands
            .iter()
            .map(|c| c["prob_permille"].as_u64().unwrap())
            .sum();
        assert_eq!(sum_perm, 1000);
        assert_eq!(
            admit_live_token_frame(true, true, 0, 100_000),
            LiveFrameVerdict::FrameAdmitted
        );
    }

    let s2: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid)).unwrap();
    assert_eq!(s2["status"], "done");
    assert_eq!(s2["tokens_emitted"], 5);
    let frames2 = s2["observatory_frames"].as_array().unwrap();
    assert_eq!(frames2.len(), 1);
    assert_eq!(frames2[0]["phase"], "decode");

    // slice_tokens=0 clamps to 1
    let begin_min: Value = serde_json::from_str(
        &engine.holo_begin_foundry_turn(
            &serde_json::json!({
                "prompt": "Single token clamp",
                "selected_model": "uor-r4.qwen3-0.6b-holo",
                "live_hud": true,
                "slice_tokens": 0
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(begin_min["slice_tokens"], 1);
    let tid_min = begin_min["turn_id"].as_u64().unwrap();
    let m1: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid_min)).unwrap();
    assert_eq!(m1["tokens_emitted"], 1);
    assert_eq!(m1["status"], "streaming");
}

#[test]
fn test_studio_prewarm_prefix_and_plan_tool_policy_r4_60() {
    // 1. STUDIO_SELF_KNOWLEDGE_PREFIX matches default_holo_chat_system_prompt byte-for-byte
    let default_sys = StudioEngine::default_holo_chat_system_prompt("uor-r4.qwen3-0.6b-holo");
    assert_eq!(default_sys, STUDIO_SELF_KNOWLEDGE_PREFIX);
    assert!(STUDIO_SELF_KNOWLEDGE_PREFIX.contains("MVM"));
    assert!(STUDIO_SELF_KNOWLEDGE_PREFIX.contains("<prism-write"));
    assert!(STUDIO_SELF_KNOWLEDGE_PREFIX.contains("Block-32 RHT"));
    assert!(STUDIO_SELF_KNOWLEDGE_PREFIX.contains("preview_inspect"));
    assert!(STUDIO_SELF_KNOWLEDGE_PREFIX.contains("github_git"));
    assert!(
        STUDIO_SELF_KNOWLEDGE_PREFIX.contains("wrapping")
            && STUDIO_SELF_KNOWLEDGE_PREFIX.contains("Q15")
            && STUDIO_SELF_KNOWLEDGE_PREFIX.contains("Q16"),
        "STUDIO_SELF_KNOWLEDGE_PREFIX must teach wrapping/Q15/Q16 optimal operands"
    );

    // 2. format_studio_system_parts puts repo@branch + tree after STUDIO_SELF_KNOWLEDGE_PREFIX,
    //    while surface_mode == "foundry" preserves BUILD_SYSTEM_PREFIX verbatim.
    let ws_files = vec![FoundryWorkspaceFile {
        path: "src/lib.rs".to_string(),
        content: "pub fn add(a: i32, b: i32) -> i32 { a.wrapping_add(b) }".to_string(),
        sha256: "deadbeef".to_string(),
    }];
    let (studio_core, studio_ws_hdr, studio_excerpts) = format_studio_system_parts(
        "studio",
        "plan",
        "afflom/demo",
        "feat/test",
        &ws_files,
        Some("src/lib.rs"),
    );
    assert_eq!(studio_core, STUDIO_SELF_KNOWLEDGE_PREFIX);
    assert!(studio_ws_hdr.contains("afflom/demo@feat/test"));
    assert!(studio_ws_hdr.contains("src/lib.rs"));
    assert!(!studio_excerpts.is_empty());

    let (foundry_core, _, _) = format_studio_system_parts(
        "foundry",
        "agent",
        "afflom/demo",
        "main",
        &ws_files,
        Some("src/lib.rs"),
    );
    assert!(foundry_core.starts_with(BUILD_SYSTEM_PREFIX));

    // 3. Turn-1 prewarm reuse in studio + plan mode, permitting workspace_read and rejecting mutating mvm_exec
    let mut engine = StudioEngine::new_default();
    let ws_read_pieces: &[(u32, &[u8])] = &[
        (261, b"<tool_call>"),
        (
            262,
            b"{\"name\":\"workspace_read\",\"arguments\":{\"path\":\"src/engine.rs\"}}",
        ),
        (263, b"</tool_call>"),
    ];
    let bytes_read = build_scripted_holo_v13_bytes(&[10, 261, 262, 263, 257], ws_read_pieces);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_read)
        .expect("mount workspace_read scripted model");

    let begin_plan_read: Value = serde_json::from_str(
        &engine.holo_begin_foundry_turn(
            &serde_json::json!({
                "prompt": "Inspect src/engine.rs",
                "selected_model": "uor-r4.qwen3-0.6b-holo",
                "surface_mode": "studio",
                "composer_mode": "plan",
                "repo": "afflom/lexlean-r4",
                "branch": "main",
                "workspace_files": ws_files,
                "active_file": "src/lib.rs"
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(begin_plan_read["status"], "ready");
    let reused = begin_plan_read["prefix_reused_tokens"]
        .as_u64()
        .unwrap_or(0);
    assert!(
        reused > 0,
        "Turn 1 in studio mode after prewarm must hit prewarmed KV cache (prefix_reused_tokens={reused})"
    );
    let tid_read = begin_plan_read["turn_id"].as_u64().unwrap();
    let active_sys = engine
        .active_foundry_turn_system_prompt(tid_read)
        .expect("active system prompt");
    assert!(
        active_sys.starts_with(STUDIO_SELF_KNOWLEDGE_PREFIX),
        "studio active system prompt must preserve STUDIO_SELF_KNOWLEDGE_PREFIX verbatim at start"
    );
    assert!(active_sys.contains("afflom/lexlean-r4@main"));

    let step_read: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid_read)).unwrap();
    assert_eq!(step_read["status"], "needs_tool");
    assert_eq!(step_read["tool_call"]["name"], "workspace_read");
    assert_eq!(step_read["tool_call"]["arguments"]["path"], "src/engine.rs");

    // Local StudioEngine tool execution for workspace_read, workspace_search, and read-only mvm_exec
    assert!(admit_studio_tool(ComposerToolPolicy::PlanReadOnly, false));
    assert!(!admit_studio_tool(ComposerToolPolicy::PlanReadOnly, true));
    let local_read: Value = serde_json::from_str(
        &engine.execute_studio_tool_json(&step_read["tool_call"].to_string(), "plan"),
    )
    .unwrap();
    assert_eq!(local_read["status"], "ok");
    assert!(local_read["stdout"]
        .as_str()
        .unwrap_or("")
        .contains("GeometricRouter"));

    let local_search: Value = serde_json::from_str(&engine.execute_studio_tool_json(
        r#"{"name":"workspace_search","arguments":{"query":"GeometricRouter"}}"#,
        "plan",
    ))
    .unwrap();
    assert_eq!(local_search["status"], "ok");
    assert!(local_search["stdout"]
        .as_str()
        .unwrap_or("")
        .contains("src/engine.rs"));

    // Bridge-bound tools yield needs_tool cleanly
    for bridge_tool in ["preview_inspect", "github_git", "spiralcore_fetch"] {
        let raw = format!(
            "<tool_call>{{\"name\":\"{bridge_tool}\",\"arguments\":{{\"subcommand\":\"status\",\"url\":\"https://example.org\"}}}}</tool_call>"
        );
        let det = detect_studio_tool_call(raw.as_bytes(), "studio", "plan", 0)
            .expect("bridge tool detected");
        assert!(
            det.admitted,
            "{bridge_tool} read-only must be admitted in plan mode"
        );
        assert_eq!(det.name, bridge_tool);
    }

    // Mutating mvm_exec in plan mode must be rejected via admit_studio_tool without modifying /workspace
    let mut_mvm_pieces: &[(u32, &[u8])] = &[
        (261, b"<tool_call>"),
        (
            262,
            b"{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"echo tamper > /workspace/src/engine.rs\"}}",
        ),
        (263, b"</tool_call>"),
    ];
    let bytes_mut_mvm = build_scripted_holo_v13_bytes(&[10, 261, 262, 263, 257], mut_mvm_pieces);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_mut_mvm)
        .expect("mount mutating mvm_exec scripted model");

    let ws_before = engine.workspace_list_json();
    let begin_mut: Value = serde_json::from_str(
        &engine.holo_begin_foundry_turn(
            &serde_json::json!({
                "prompt": "Try mutating workspace in plan mode",
                "selected_model": "uor-r4.qwen3-0.6b-holo",
                "surface_mode": "studio",
                "composer_mode": "plan"
            })
            .to_string(),
        ),
    )
    .unwrap();
    let tid_mut = begin_mut["turn_id"].as_u64().unwrap();
    let step_mut: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid_mut)).unwrap();
    assert_eq!(step_mut["status"], "error");
    assert_eq!(step_mut["error_code"], "RefusedPlanModeMutation");
    assert_eq!(
        engine.workspace_list_json(),
        ws_before,
        "RefusedPlanModeMutation must leave /workspace untouched"
    );

    // Mutating <prism-write>, workspace_write, and mutating github_git also refuse in plan mode
    let prism_write_det = detect_studio_tool_call(
        b"<prism-write path=\"src/lib.rs\">pub fn x() {}</prism-write>",
        "studio",
        "plan",
        0,
    )
    .expect("prism-write caught in plan mode");
    assert!(!prism_write_det.admitted);
    assert_eq!(
        prism_write_det.refusal_code,
        Some("RefusedPlanModeMutation")
    );

    let ws_write_det = detect_studio_tool_call(
        br#"<tool_call>{"name":"workspace_write","arguments":{"path":"src/lib.rs","content":"x"}}</tool_call>"#,
        "studio",
        "plan",
        0,
    )
    .expect("workspace_write caught in plan mode");
    assert!(!ws_write_det.admitted);
    assert_eq!(ws_write_det.refusal_code, Some("RefusedPlanModeMutation"));

    let git_push_det = detect_studio_tool_call(
        br#"<tool_call>{"name":"github_git","arguments":{"subcommand":"commit_push"}}</tool_call>"#,
        "studio",
        "plan",
        0,
    )
    .expect("mutating github_git caught in plan mode");
    assert!(!git_push_det.admitted);
    assert_eq!(git_push_det.refusal_code, Some("RefusedPlanModeMutation"));

    // 4. Beginning a turn with surface_mode == "foundry" still emits BUILD_SYSTEM_PREFIX
    let begin_foundry: Value = serde_json::from_str(
        &engine.holo_begin_foundry_turn(
            &serde_json::json!({
                "prompt": "Build web app",
                "selected_model": "uor-r4.qwen3-0.6b-holo",
                "surface_mode": "foundry",
                "composer_mode": "agent"
            })
            .to_string(),
        ),
    )
    .unwrap();
    let tid_foundry = begin_foundry["turn_id"].as_u64().unwrap();
    let foundry_sys = engine
        .active_foundry_turn_system_prompt(tid_foundry)
        .expect("foundry active system prompt");
    assert!(foundry_sys.starts_with(BUILD_SYSTEM_PREFIX));
    let _ = engine.holo_cancel_foundry_turn(tid_foundry);

    // 5. Prefix-pinned truncation preserves STUDIO_SELF_KNOWLEDGE_PREFIX in multi-turn chat history
    let mut model = lexlean_r4_wasm::holo_geometric_runtime::HoloV13Model::from_bytes(
        "uor-r4.qwen3-0.6b-holo".to_string(),
        &bytes_read,
    )
    .expect("construct model");
    model.prewarm_system_prefix(STUDIO_SELF_KNOWLEDGE_PREFIX);
    let long_hist: Vec<(String, String)> = (0..6)
        .map(|i| {
            (
                format!("turn_{i} {}", "q".repeat(180)),
                format!("ans_{i} {}", "a".repeat(180)),
            )
        })
        .collect();
    let (_r_trunc, m_trunc) = model.generate_chat_turn_with_history(
        "latest question",
        &long_hist,
        STUDIO_SELF_KNOWLEDGE_PREFIX,
        4,
    );
    assert!(
        m_trunc.prefix_reused_tokens > 0,
        "prefix-pinned truncation must keep STUDIO_SELF_KNOWLEDGE_PREFIX pinned across long history: {m_trunc:?}"
    );

    // 6. Multi-hop continuation across 6 hops + bridge-bound preview_inspect via stepper
    let preview_pieces: &[(u32, &[u8])] = &[
        (261, b"<tool_call>"),
        (
            262,
            b"{\"name\":\"preview_inspect\",\"arguments\":{\"selector\":\"#app\"}}",
        ),
        (263, b"</tool_call>"),
    ];
    let bytes_preview = build_scripted_holo_v13_bytes(&[10, 261, 262, 263, 257], preview_pieces);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_preview)
        .expect("mount preview_inspect scripted model");
    let five_prior: Vec<Value> = (0..5)
        .map(|i| {
            serde_json::json!({
                "tool": if i == 0 { "workspace_read" } else { "workspace_search" },
                "cmd": format!("hop_{i}"),
                "exit_code": 0,
                "stdout": "ok",
                "stderr": "",
                "changed_files": []
            })
        })
        .collect();
    let b_hop6: Value = serde_json::from_str(
        &engine.holo_begin_foundry_turn(
            &serde_json::json!({
                "prompt": "Inspect preview on hop 6",
                "selected_model": "uor-r4.qwen3-0.6b-holo",
                "surface_mode": "studio",
                "composer_mode": "plan",
                "tool_results": five_prior
            })
            .to_string(),
        ),
    )
    .unwrap();
    let tid_hop6 = b_hop6["turn_id"].as_u64().unwrap();
    let s_hop6: Value = serde_json::from_str(&engine.holo_step_foundry_turn(tid_hop6)).unwrap();
    assert_eq!(s_hop6["status"], "needs_tool");
    assert_eq!(s_hop6["tool_call"]["name"], "preview_inspect");
    let bridge_ack: Value = serde_json::from_str(
        &engine.execute_studio_tool_json(&s_hop6["tool_call"].to_string(), "plan"),
    )
    .unwrap();
    assert_eq!(bridge_ack["status"], "needs_tool");
    assert_eq!(bridge_ack["tool"], "preview_inspect");
    assert!(
        detect_studio_tool_call(
            br#"<tool_call>{"name":"preview_inspect","arguments":{}}</tool_call>"#,
            "studio",
            "plan",
            6
        )
        .is_none(),
        "7th hop (iterations_used=6) must be bounded"
    );
}

#[test]
fn test_plan_shell_hardening_workspace_mutations_and_chat_refusal_r4_60() {
    let mut engine = StudioEngine::new_default();

    // 1. Tightened PlanReadOnly shell allowlist: refuse `env rm -rf ...`, `touch foo`, `sort -o`, `sed -ni` / write commands
    for mut_cmd in [
        "env rm -rf /workspace/src/main.rs",
        "touch foo",
        "sort -o out.txt src/engine.rs",
        "sort --output=out.txt src/engine.rs",
        "sed -ni 's/a/b/' src/engine.rs",
        "sed -Ei 's/a/b/' src/engine.rs",
        "sed 's/a/b/w out.txt' src/engine.rs",
        "sed 's/a/b/; w out.txt' src/engine.rs",
        "sed 's/a b/c/w out.txt' src/engine.rs",
        "sed '1w out.txt' src/engine.rs",
        "sed '/pat/w out.txt' src/engine.rs",
        "sed '1,3w out.txt' src/engine.rs",
        "sed '1s/a/b/w out.txt' src/engine.rs",
        r"sed 's/a\/b/c/w out.txt' src/engine.rs",
        "sed -nf write.sed src/engine.rs",
        "sed -ne 'w out.txt' src/engine.rs",
        "sed --expression='w out.txt' src/engine.rs",
        "sed -l 80 'w out.txt' src/engine.rs",
        "sed -l 80 '1e rm -rf /workspace' src/engine.rs",
    ] {
        let raw = format!(
            "<tool_call>{}</tool_call>",
            serde_json::json!({
                "name": "mvm_exec",
                "arguments": { "cmd": mut_cmd }
            })
        );
        let det = detect_studio_tool_call(raw.as_bytes(), "studio", "plan", 0)
            .unwrap_or_else(|| panic!("expected detection for `{mut_cmd}`"));
        assert!(!det.admitted, "`{mut_cmd}` must be refused in plan mode");
        assert_eq!(det.refusal_code, Some("RefusedPlanModeMutation"));

        let exec_res: Value = serde_json::from_str(&engine.execute_studio_tool_json(&raw, "plan"))
            .expect("valid tool json");
        assert_eq!(exec_res["status"], "error");
        assert_eq!(exec_res["error_code"], "RefusedPlanModeMutation");
    }

    // Read-only `ls`, `pwd`, `env`, `sort -rn`, and `sed -n '1,2p'` / `sed -n -e '1,5p'` are admitted & executable in plan mode
    for ro_cmd in [
        "ls",
        "pwd",
        "env",
        "sort -rn src/engine.rs",
        "sed -n '1,2p' src/engine.rs",
        "sed -n -e '1,5p' src/engine.rs",
    ] {
        let raw = serde_json::json!({
            "name": "mvm_exec",
            "arguments": { "cmd": ro_cmd }
        })
        .to_string();
        let res: Value = serde_json::from_str(&engine.execute_studio_tool_json(&raw, "plan"))
            .expect("valid read-only mvm_exec json");
        assert_eq!(
            res["status"], "ok",
            "`{ro_cmd}` must execute in plan mode: {res}"
        );
        assert_eq!(res["exit_code"], 0, "`{ro_cmd}` failed: {res}");
    }

    // 2. Local mvm_exec changed_files reporting + workspace_write / workspace_rename / workspace_delete parity
    let mvm_write_res: Value = serde_json::from_str(&engine.execute_studio_tool_json(
        r#"{"name":"mvm_exec","arguments":{"cmd":"printf 'pub fn mvm_ok() -> i32 { 1 }\n' > /workspace/src/created_by_mvm.rs"}}"#,
        "agent",
    ))
    .unwrap();
    assert_eq!(mvm_write_res["status"], "ok");
    let changed = mvm_write_res["changed_files"].as_array().unwrap();
    assert!(
        changed.iter().any(|p| p == "src/created_by_mvm.rs"),
        "execute_studio_tool_json must report real changed_files, got: {changed:?}"
    );

    // 3. chat_assistant_json refuses mutating tool in `plan` mode with status="error" & RefusedPlanModeMutation
    let mut_plan_pieces: &[(u32, &[u8])] = &[
        (261, b"<tool_call>"),
        (
            262,
            b"{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"env rm -rf /workspace/src/engine.rs\"}}",
        ),
        (263, b"</tool_call>"),
    ];
    let bytes_plan_mut = build_scripted_holo_v13_bytes(&[10, 261, 262, 263, 257], mut_plan_pieces);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_plan_mut)
        .expect("mount plan refusal model");
    let ws_before = engine.list_workspace();
    let plan_chat_res: Value = serde_json::from_str(
        &engine.chat_assistant_json(
            &serde_json::json!({
                "prompt": "Delete src/engine.rs",
                "selected_model": "uor-r4.qwen3-0.6b-holo",
                "composer_mode": "plan"
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(plan_chat_res["status"], "error");
    assert_eq!(plan_chat_res["error_code"], "RefusedPlanModeMutation");
    let upper_plan_chat_res: Value = serde_json::from_str(
        &engine.chat_assistant_json(
            &serde_json::json!({
                "prompt": "Delete src/engine.rs",
                "selected_model": "uor-r4.qwen3-0.6b-holo",
                "composer_mode": "PLAN"
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(upper_plan_chat_res["status"], "error");
    assert_eq!(upper_plan_chat_res["error_code"], "RefusedPlanModeMutation");
    assert_eq!(
        engine.list_workspace(),
        ws_before,
        "RefusedPlanModeMutation in chat_assistant_json must not mutate workspace"
    );

    // 4. chat_assistant_json handles workspace_write -> workspace_rename -> workspace_delete in agent mode
    let write_pieces: &[(u32, &[u8])] = &[
        (261, b"<tool_call>"),
        (
            262,
            b"{\"name\":\"workspace_write\",\"arguments\":{\"path\":\"src/draft.rs\",\"content\":\"pub fn v() -> i32 { 7 }\\n\"}}",
        ),
        (263, b"</tool_call>"),
    ];
    let bytes_write = build_scripted_holo_v13_bytes(&[10, 261, 262, 263, 257], write_pieces);
    engine
        .mount_holo_v13_bytes("uor-r4.qwen3-0.6b-holo", &bytes_write)
        .expect("mount workspace_write model");
    let chat_write: Value = serde_json::from_str(
        &engine.chat_assistant_json(
            &serde_json::json!({
                "prompt": "Create src/draft.rs",
                "selected_model": "uor-r4.qwen3-0.6b-holo",
                "composer_mode": "agent"
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(chat_write["status"], "ok");
    assert!(
        engine
            .list_workspace()
            .contains(&"src/draft.rs".to_string()),
        "chat_assistant_json must apply workspace_write"
    );

    let rename_res: Value = serde_json::from_str(&engine.execute_studio_tool_json(
        r#"{"name":"workspace_rename","arguments":{"from":"src/draft.rs","to":"src/renamed.rs"}}"#,
        "agent",
    ))
    .unwrap();
    assert_eq!(rename_res["status"], "ok");
    assert_eq!(
        rename_res["changed_files"],
        serde_json::json!(["src/draft.rs", "src/renamed.rs"]),
        "workspace_rename changed_files must contain real paths [from, to]"
    );
    assert!(!engine
        .list_workspace()
        .contains(&"src/draft.rs".to_string()));
    assert!(engine
        .list_workspace()
        .contains(&"src/renamed.rs".to_string()));

    let del_res: Value = serde_json::from_str(&engine.execute_studio_tool_json(
        r#"{"name":"workspace_delete","arguments":{"path":"src/renamed.rs"}}"#,
        "agent",
    ))
    .unwrap();
    assert_eq!(del_res["status"], "ok");
    assert!(!engine
        .list_workspace()
        .contains(&"src/renamed.rs".to_string()));

    // workspace_rename and workspace_delete in plan mode must refuse cleanly
    for op in [
        r#"{"name":"workspace_rename","arguments":{"from":"src/engine.rs","to":"src/x.rs"}}"#,
        r#"{"name":"workspace_delete","arguments":{"path":"src/engine.rs"}}"#,
    ] {
        let rej: Value =
            serde_json::from_str(&engine.execute_studio_tool_json(op, "plan")).unwrap();
        assert_eq!(rej["status"], "error");
        assert_eq!(rej["error_code"], "RefusedPlanModeMutation");
    }
}
