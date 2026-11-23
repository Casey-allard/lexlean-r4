use lexlean_r4_wasm::holo_geometric_runtime::{
    flush_incremental_utf8, HoloTok13, HoloTok13SpeculativeDrafter, HoloV13Model,
    HoloV13StreamingBuilder,
};
use lexlean_r4_wasm::lexlean_generated::r4_webgpu_geodesic_runtime::{
    MlpShortlistMode, MlpUserPolicy,
};
use lexlean_r4_wasm::stitch_fabric::{
    heal_2of3_pair, parse_and_validate_manifest, recover_2of3_object, StitchParityGroup,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    static ENGINE: RefCell<FoundryV13Engine> = RefCell::new(FoundryV13Engine::default());
}

struct FoundryV13Engine {
    builder: Option<(String, HoloV13StreamingBuilder)>,
    model: Option<HoloV13Model>,
    turn: Option<ActiveTurn>,
    next_turn_id: u64,
    speculative_default: bool,
}

impl Default for FoundryV13Engine {
    fn default() -> Self {
        Self {
            builder: None,
            model: None,
            turn: None,
            next_turn_id: 0,
            speculative_default: true,
        }
    }
}

struct ActiveTurn {
    turn_id: u64,
    model_id: String,
    kv_checkpoint_len: usize,
    prefix_reused_tokens: usize,
    pending_prefill: Vec<u32>,
    prefill_cursor: usize,
    prefill_slice_tokens: usize,
    slice_tokens: usize,
    max_new_tokens: usize,
    speculative_enabled: bool,
    accept_ema_permille: u16,
    draft_proposed_total: usize,
    draft_accepted_total: usize,
    context_tokens: Vec<u32>,
    last_scored: Vec<(u32, i32)>,
    generated_ids: Vec<u32>,
    raw_stream: Vec<u8>,
    emitted_raw: usize,
    stop_sequences: Vec<String>,
    user_key: String,
    cancelled: bool,
    active: bool,
}

#[derive(Deserialize)]
struct ChatMessageInput {
    #[serde(default)]
    role: String,
    #[serde(default)]
    content: String,
}

#[derive(Deserialize)]
struct BeginTurnRequest {
    #[serde(default)]
    system_prompt: String,
    #[serde(default)]
    messages: Vec<ChatMessageInput>,
    #[serde(default)]
    prompt: String,
    #[serde(default)]
    max_new_tokens: Option<usize>,
    #[serde(default)]
    slice_tokens: Option<usize>,
    #[serde(default)]
    prefill_slice_tokens: Option<usize>,
    #[serde(default)]
    speculative: Option<bool>,
    #[serde(default)]
    stop_sequences: Vec<String>,
}

#[derive(Deserialize, Default)]
struct ConfigureRequest {
    #[serde(default)]
    shortlist_k: Option<usize>,
    #[serde(default)]
    flock_window: Option<usize>,
    #[serde(default)]
    mlp_policy: Option<String>,
    #[serde(default)]
    mlp_shortlist_k: Option<usize>,
    #[serde(default)]
    speculative: Option<bool>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn err_json(code: &str, message: impl Into<String>) -> String {
    serde_json::json!({
        "ok": false,
        "status": "error",
        "error_code": code,
        "message": message.into(),
        "float_ops": 0,
        "dense_matmul_ops": 0,
    })
    .to_string()
}

fn token_causes_cycle(generated: &[u32], cand: u32, tokenizer: &HoloTok13) -> bool {
    let n = generated.len();
    if n >= 1 && generated[n - 1] == cand {
        let dec = tokenizer.decode(&[cand]);
        if matches!(dec.as_str(), "," | ";") {
            return true;
        }
        if n >= 2 && generated[n - 2] == cand && dec.chars().any(|c| c.is_alphanumeric()) {
            return true;
        }
    }
    for p in 2usize..=8 {
        if n + 1 < 2 * p {
            continue;
        }
        let b1_start = n + 1 - 2 * p;
        let b2_start = n + 1 - p;
        let mut all_eq = cand == generated[b1_start + p - 1];
        if all_eq {
            for k in 0..(p - 1) {
                if generated[b1_start + k] != generated[b2_start + k] {
                    all_eq = false;
                    break;
                }
            }
        }
        if all_eq {
            return true;
        }
    }
    false
}

fn pick_next_holo_token(
    last_scored: &[(u32, i32)],
    generated: &[u32],
    eos_token_id: u32,
    eot_token_id: u32,
    tokenizer: &HoloTok13,
) -> u32 {
    let mut blocked_ids: Vec<u32> = Vec::new();
    loop {
        let mut best_id = eos_token_id;
        let mut best_logit = i32::MIN;
        let mut found_any = false;
        for &(tid, logit) in last_scored {
            if blocked_ids.contains(&tid) {
                continue;
            }
            if !found_any || logit > best_logit {
                best_logit = logit;
                best_id = tid;
                found_any = true;
            }
        }
        if !found_any || best_id == eos_token_id || best_id == eot_token_id {
            return best_id;
        }
        if blocked_ids.len() < 8 && token_causes_cycle(generated, best_id, tokenizer) {
            blocked_ids.push(best_id);
            continue;
        }
        return best_id;
    }
}

fn strip_think_prefix(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("<think>\n\n</think>\n\n") {
        rest.trim()
    } else if let Some(rest) = trimmed.strip_prefix("<think></think>") {
        rest.trim()
    } else {
        trimmed
    }
}

fn longest_common_prefix(a: &[u32], b: &[u32]) -> usize {
    let mut i = 0usize;
    for (&x, &y) in a.iter().zip(b.iter()) {
        if x == y {
            i += 1;
        } else {
            break;
        }
    }
    i
}

#[wasm_bindgen]
pub fn v13_begin_mount(
    model_id: &str,
    expected_pieces: usize,
    expected_sha256: &str,
    skip_rehash: bool,
) -> String {
    let key = if model_id.trim().is_empty() {
        "uor-r4:qwen3-0.6b-foundry".to_string()
    } else {
        model_id.trim().to_string()
    };
    ENGINE.with(|cell| {
        let mut eng = cell.borrow_mut();
        eng.turn = None;
        eng.model = None;
        let builder = HoloV13StreamingBuilder::new_with_rehash_policy(
            key.clone(),
            expected_pieces,
            expected_sha256.trim().to_ascii_lowercase(),
            skip_rehash,
        );
        eng.builder = Some((key.clone(), builder));
        serde_json::json!({
            "ok": true,
            "model_id": key,
            "expected_pieces": expected_pieces,
            "float_ops": 0,
        })
        .to_string()
    })
}

#[wasm_bindgen]
pub fn v13_push_piece(piece: &[u8]) -> String {
    ENGINE.with(|cell| {
        let mut eng = cell.borrow_mut();
        let Some((key, mut builder)) = eng.builder.take() else {
            return err_json("MountNotInitialized", "Call v13_begin_mount before pushing pieces");
        };
        match builder.push_piece(piece) {
            Ok(()) => {
                eng.builder = Some((key, builder));
                serde_json::json!({ "ok": true }).to_string()
            }
            Err(err) => err_json(err.error_code(), err.to_string()),
        }
    })
}

#[wasm_bindgen]
pub fn v13_finalize_mount() -> String {
    ENGINE.with(|cell| {
        let mut eng = cell.borrow_mut();
        let Some((key, builder)) = eng.builder.take() else {
            return err_json("MountNotInitialized", "No active V13 streaming builder");
        };
        match builder.finalize() {
            Ok(model) => {
                let summary = serde_json::json!({
                    "ok": true,
                    "model_id": key,
                    "container_sha256": model.container_sha256,
                    "n_layers": model.n_layers,
                    "d_model": model.d_model,
                    "vocab": model.vocab_size,
                    "max_seq": model.max_seq,
                    "promote_down_int8_layers": model.layers.iter().filter(|l| matches!(l.down_proj, lexlean_r4_wasm::holo_geometric_runtime::HoloDownProjMatrix::Int8(_))).count(),
                    "float_ops": 0,
                    "dense_matmul_ops": 0,
                });
                eng.model = Some(model);
                summary.to_string()
            }
            Err(err) => err_json(err.error_code(), err.to_string()),
        }
    })
}

#[wasm_bindgen]
pub fn v13_status() -> String {
    ENGINE.with(|cell| {
        let eng = cell.borrow();
        match &eng.model {
            Some(m) => serde_json::json!({
                "ok": true,
                "mounted": true,
                "model_id": m.model_id,
                "container_sha256": m.container_sha256,
                "n_layers": m.n_layers,
                "d_model": m.d_model,
                "vocab": m.vocab_size,
                "max_seq": m.max_seq,
                "kv_cached_tokens": m.kv_cache.tokens.len(),
                "float_ops": 0,
                "dense_matmul_ops": 0,
            })
            .to_string(),
            None => serde_json::json!({
                "ok": true,
                "mounted": false,
                "float_ops": 0,
                "dense_matmul_ops": 0,
            })
            .to_string(),
        }
    })
}

#[wasm_bindgen]
pub fn v13_prewarm_prefix(system_prompt: &str) -> String {
    ENGINE.with(|cell| {
        let mut eng = cell.borrow_mut();
        if let Some(prev) = &eng.turn {
            if prev.active {
                return err_json("BusyTurn", "Cannot prewarm while a turn is active");
            }
        }
        let Some(model) = eng.model.as_mut() else {
            return err_json("ModelNotMounted", "No UORR4V13 model mounted");
        };
        let sys = if system_prompt.trim().is_empty() {
            "You are UOR-R4 Foundry Assistant."
        } else {
            system_prompt.trim()
        };
        let prefix = format!("<|im_start|>system\n{sys}<|im_end|>\n<|im_start|>user\n");
        let ids = model.tokenizer.encode(&prefix);
        if !ids.is_empty()
            && model.kv_cache.tokens.len() >= ids.len()
            && model.kv_cache.tokens[..ids.len()] == ids[..]
        {
            return serde_json::json!({
                "ok": true,
                "prewarmed_tokens": ids.len(),
                "reused": true,
                "kv_cached_tokens": model.kv_cache.tokens.len(),
                "float_ops": 0,
                "dense_matmul_ops": 0,
            })
            .to_string();
        }
        model.prewarm_system_prefix(sys);
        serde_json::json!({
            "ok": true,
            "prewarmed_tokens": model.kv_cache.tokens.len(),
            "reused": false,
            "kv_cached_tokens": model.kv_cache.tokens.len(),
            "float_ops": 0,
            "dense_matmul_ops": 0,
        })
        .to_string()
    })
}

#[wasm_bindgen]
pub fn v13_configure(payload_json: &str) -> String {
    let req: ConfigureRequest = if payload_json.trim().is_empty() {
        ConfigureRequest::default()
    } else {
        match serde_json::from_str(payload_json.trim()) {
            Ok(r) => r,
            Err(e) => return err_json("InvalidPayload", format!("Invalid configure JSON: {e}")),
        }
    };
    ENGINE.with(|cell| {
        let mut eng = cell.borrow_mut();
        if let Some(spec) = req.speculative {
            eng.speculative_default = spec;
        }
        let spec_out = eng.speculative_default;
        let Some(model) = eng.model.as_mut() else {
            return serde_json::json!({
                "ok": true,
                "mounted": false,
                "speculative": spec_out,
                "float_ops": 0,
                "dense_matmul_ops": 0,
            })
            .to_string();
        };
        if req.shortlist_k.is_some() || req.flock_window.is_some() {
            let sk = req.shortlist_k.unwrap_or(model.shortlist_k);
            let fw = req.flock_window.unwrap_or(model.flock_window);
            model.set_budgets(sk, fw);
        }
        let mut mlp_verdict_label: Option<&'static str> = None;
        if req.mlp_policy.is_some() || req.mlp_shortlist_k.is_some() {
            let policy = match req
                .mlp_policy
                .as_deref()
                .map(str::trim)
                .map(str::to_ascii_lowercase)
                .as_deref()
            {
                Some("dense") | Some("dense_exact") => MlpUserPolicy::ForceDenseExact,
                Some("sparse") | Some("geodesic_sparse") => MlpUserPolicy::ForceGeodesicSparse,
                _ => model.mlp_policy,
            };
            let mk = req.mlp_shortlist_k.unwrap_or(model.mlp_shortlist_k);
            let verdict = model.set_mlp_policy(policy, mk);
            mlp_verdict_label = Some(match verdict {
                MlpShortlistMode::GeodesicSparse1536 => "GeodesicSparse1536",
                MlpShortlistMode::DenseExact => "DenseExact",
                MlpShortlistMode::RefusedInvalidNeuronShortlist => {
                    "RefusedInvalidNeuronShortlist"
                }
            });
        }
        serde_json::json!({
            "ok": true,
            "mounted": true,
            "speculative": spec_out,
            "shortlist_k": model.shortlist_k,
            "flock_window": model.flock_window,
            "mlp_mode": model.mlp_mode_label(),
            "mlp_verdict": mlp_verdict_label,
            "mlp_shortlist_k": model.mlp_shortlist_k,
            "float_ops": 0,
            "dense_matmul_ops": 0,
        })
        .to_string()
    })
}

#[wasm_bindgen]
pub fn v13_begin_turn(payload_json: &str) -> String {
    let req: BeginTurnRequest = match serde_json::from_str(payload_json.trim()) {
        Ok(r) => r,
        Err(e) => return err_json("InvalidPayload", format!("Invalid turn JSON: {e}")),
    };

    ENGINE.with(|cell| {
        let mut eng = cell.borrow_mut();
        if let Some(prev) = &eng.turn {
            if prev.active {
                return err_json("BusyTurn", "A V13 generation turn is already active");
            }
        }
        let speculative_default = eng.speculative_default;
        let Some(model) = eng.model.as_mut() else {
            return err_json("ModelNotMounted", "No UORR4V13 model mounted");
        };

        let sys = if req.system_prompt.trim().is_empty() {
            "You are UOR-R4 Foundry Assistant."
        } else {
            req.system_prompt.trim()
        };
        let sys_chunk = format!("<|im_start|>system\n{sys}<|im_end|>\n");
        let sys_ids = model.tokenizer.encode(&sys_chunk);
        let eot_nl_ids = model.tokenizer.encode("<|im_end|>\n");

        let mut msgs: Vec<(String, String)> = req
            .messages
            .iter()
            .filter_map(|m| {
                let c = m.content.trim();
                if c.is_empty() {
                    return None;
                }
                let role = match m.role.trim().to_ascii_lowercase().as_str() {
                    "assistant" => "assistant".to_string(),
                    "tool" => "user".to_string(),
                    "system" => return None,
                    _ => "user".to_string(),
                };
                Some((role, c.to_string()))
            })
            .collect();

        if !req.prompt.trim().is_empty() {
            let needs_push = msgs
                .last()
                .map(|(r, c)| r != "user" || c != req.prompt.trim())
                .unwrap_or(true);
            if needs_push {
                msgs.push(("user".to_string(), req.prompt.trim().to_string()));
            }
        }
        if msgs.is_empty() {
            return err_json("EmptyPrompt", "No user prompt supplied");
        }

        // Pair/encode prior turns while keeping the last user message as the open generation prompt.
        let (prior_turns, last_user_prompt) = match msgs.last() {
            Some((r, c)) if r == "user" => (&msgs[..msgs.len() - 1], c.clone()),
            _ => (&msgs[..], req.prompt.trim().to_string()),
        };
        if last_user_prompt.is_empty() {
            return err_json("EmptyPrompt", "User prompt cannot be empty");
        }

        let mut encoded_prior: Vec<Vec<u32>> = Vec::new();
        let mut idx = 0usize;
        while idx < prior_turns.len() {
            if prior_turns[idx].0 == "user"
                && idx + 1 < prior_turns.len()
                && prior_turns[idx + 1].0 == "assistant"
            {
                let u_trim = prior_turns[idx].1.trim();
                let a_trim = strip_think_prefix(&prior_turns[idx + 1].1);
                let mut t_ids = model.tokenizer.encode(&format!(
                    "<|im_start|>user\n{u_trim}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
                ));
                if let Some((_, _, exact_ids)) = model
                    .kv_cache
                    .turn_token_spans
                    .iter()
                    .rev()
                    .find(|(cu, ca, _)| cu == u_trim && ca == a_trim)
                {
                    t_ids.extend_from_slice(exact_ids);
                } else {
                    t_ids.extend_from_slice(&model.tokenizer.encode(a_trim));
                }
                t_ids.extend_from_slice(&eot_nl_ids);
                encoded_prior.push(t_ids);
                idx += 2;
            } else {
                let (role, content) = &prior_turns[idx];
                let chunk = if role == "assistant" {
                    format!(
                        "<|im_start|>assistant\n<think>\n\n</think>\n\n{}<|im_end|>\n",
                        strip_think_prefix(content)
                    )
                } else {
                    format!("<|im_start|>user\n{}<|im_end|>\n", content.trim())
                };
                encoded_prior.push(model.tokenizer.encode(&chunk));
                idx += 1;
            }
        }

        let cur_u_chunk = format!(
            "<|im_start|>user\n{last_user_prompt}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
        let cur_u_ids = model.tokenizer.encode(&cur_u_chunk);

        let max_new = req.max_new_tokens.unwrap_or(512).clamp(1, model.max_seq / 2);
        let max_ctx = model.max_seq.saturating_sub(max_new).max(32);

        let mut prior_len: usize = encoded_prior.iter().map(Vec::len).sum();
        let mut prior_start = 0usize;
        while prior_start < encoded_prior.len()
            && sys_ids.len() + prior_len + cur_u_ids.len() > max_ctx
        {
            prior_len = prior_len.saturating_sub(encoded_prior[prior_start].len());
            prior_start += 1;
        }

        let mut prompt_ids = Vec::with_capacity(max_ctx);
        if sys_ids.len() + prior_len + cur_u_ids.len() <= max_ctx {
            prompt_ids.extend_from_slice(&sys_ids);
            for t in &encoded_prior[prior_start..] {
                prompt_ids.extend_from_slice(t);
            }
            prompt_ids.extend_from_slice(&cur_u_ids);
        } else {
            let user_reserve = cur_u_ids
                .len()
                .min((max_ctx * 3 / 8).max(64).min(max_ctx / 2));
            let kept_u = if cur_u_ids.len() > user_reserve {
                &cur_u_ids[cur_u_ids.len() - user_reserve..]
            } else {
                &cur_u_ids[..]
            };
            let sys_cap = max_ctx.saturating_sub(kept_u.len());
            prompt_ids.extend_from_slice(&sys_ids[..sys_ids.len().min(sys_cap)]);
            prompt_ids.extend_from_slice(kept_u);
        }

        let prefix_match = longest_common_prefix(&model.kv_cache.tokens, &prompt_ids);
        let reuse_len = prefix_match.min(prompt_ids.len().saturating_sub(1));
        if reuse_len == 0 {
            model.kv_cache.reset(model.n_layers);
        } else {
            model
                .kv_cache
                .truncate_to(reuse_len, model.n_kv_heads, model.head_dim);
        }

        let pending_prefill = prompt_ids[reuse_len..].to_vec();
        let prefill_slice_tokens = req.prefill_slice_tokens.unwrap_or(32).clamp(1, 32);
        let slice_tokens = req.slice_tokens.unwrap_or(4).clamp(1, 8);
        let speculative_enabled = req.speculative.unwrap_or(speculative_default);
        let stop_sequences = if req.stop_sequences.is_empty() {
            vec!["</tool_call>".to_string()]
        } else {
            req.stop_sequences
        };

        let model_id = model.model_id.clone();
        eng.next_turn_id = eng.next_turn_id.saturating_add(1);
        let turn_id = eng.next_turn_id;
        let prompt_total = pending_prefill.len();

        eng.turn = Some(ActiveTurn {
            turn_id,
            model_id: model_id.clone(),
            kv_checkpoint_len: reuse_len,
            prefix_reused_tokens: reuse_len,
            pending_prefill,
            prefill_cursor: 0,
            prefill_slice_tokens,
            slice_tokens,
            max_new_tokens: max_new,
            speculative_enabled,
            accept_ema_permille: 500,
            draft_proposed_total: 0,
            draft_accepted_total: 0,
            context_tokens: prompt_ids,
            last_scored: Vec::new(),
            generated_ids: Vec::new(),
            raw_stream: Vec::new(),
            emitted_raw: 0,
            stop_sequences,
            user_key: last_user_prompt,
            cancelled: false,
            active: true,
        });

        serde_json::json!({
            "ok": true,
            "status": "ready",
            "turn_id": turn_id,
            "model_id": model_id,
            "prefill_total": prompt_total,
            "prefix_reused_tokens": reuse_len,
            "max_new_tokens": max_new,
            "float_ops": 0,
            "dense_matmul_ops": 0,
        })
        .to_string()
    })
}

#[wasm_bindgen]
pub fn v13_step_turn(turn_id: u64) -> String {
    ENGINE.with(|cell| {
        let mut eng = cell.borrow_mut();
        let Some(mut turn) = eng.turn.take() else {
            return err_json("TurnNotFound", "No active V13 turn session");
        };
        if turn.turn_id != turn_id {
            eng.turn = Some(turn);
            return err_json("TurnNotFound", "V13 turn_id mismatch");
        }
        if turn.cancelled {
            return serde_json::json!({
                "ok": true,
                "status": "cancelled",
                "turn_id": turn_id,
                "delta": "",
                "full_text": String::from_utf8_lossy(&turn.raw_stream).into_owned(),
                "tokens_emitted": turn.generated_ids.len(),
                "float_ops": 0,
                "dense_matmul_ops": 0,
            })
            .to_string();
        }
        if !turn.active {
            return err_json("TurnNotFound", "V13 turn already completed");
        }
        let Some(model) = eng.model.as_mut() else {
            return err_json("ModelNotMounted", "UORR4V13 model unmounted mid-turn");
        };

        if turn.prefill_cursor < turn.pending_prefill.len() {
            let total = turn.pending_prefill.len();
            let cur = turn.prefill_cursor;
            let end = (cur + turn.prefill_slice_tokens).min(total);
            let is_last = end == total;
            let chunk = &turn.pending_prefill[cur..end];
            let (sc_opt, _, _) =
                model.prefill_tokens_tiled(chunk, &turn.context_tokens, is_last, true);
            if let Some(sc) = sc_opt {
                turn.last_scored = sc;
            }
            turn.prefill_cursor = end;
            let reused = turn.prefix_reused_tokens;
            let model_id = turn.model_id.clone();
            eng.turn = Some(turn);
            return serde_json::json!({
                "ok": true,
                "status": if is_last { "streaming" } else { "prefilling" },
                "turn_id": turn_id,
                "model_id": model_id,
                "delta": "",
                "full_text": "",
                "tokens_emitted": 0,
                "prompt_done": end,
                "prompt_total": total,
                "prefix_reused_tokens": reused,
                "float_ops": 0,
                "dense_matmul_ops": 0,
            })
            .to_string();
        }

        let mut slice_delta = String::new();
        let mut status = "streaming";
        let mut stop_reason: Option<String> = None;
        let mut emitted_in_slice = 0usize;

        while emitted_in_slice < turn.slice_tokens {
            if turn.generated_ids.len() >= turn.max_new_tokens
                || model.kv_cache.tokens.len() >= model.max_seq
            {
                status = "done";
                stop_reason = Some("length".to_string());
                break;
            }

            let chosen_id = pick_next_holo_token(
                &turn.last_scored,
                &turn.generated_ids,
                model.eos_token_id,
                model.eot_token_id,
                &model.tokenizer,
            );
            if chosen_id == model.eos_token_id || chosen_id == model.eot_token_id {
                status = "done";
                stop_reason = Some("eos".to_string());
                break;
            }

            let remaining_gen = turn
                .max_new_tokens
                .saturating_sub(turn.generated_ids.len() + 1);
            let remaining_kv = model
                .max_seq
                .saturating_sub(model.kv_cache.tokens.len() + 1);

            let drafts = if turn.speculative_enabled && remaining_gen > 0 && remaining_kv > 0 {
                HoloTok13SpeculativeDrafter::propose_adaptive_draft_tokens(
                    &model.tokenizer,
                    &turn.context_tokens,
                    &turn.raw_stream,
                    chosen_id,
                    remaining_gen.min(remaining_kv).min(4),
                    false,
                    turn.accept_ema_permille,
                    model.eos_token_id,
                    model.eot_token_id,
                    model.think_end_id,
                )
            } else {
                Vec::new()
            };

            if !drafts.is_empty() {
                let kv_before = model.kv_cache.tokens.len();
                let outcome = model.step_speculative_tile(
                    chosen_id,
                    &drafts,
                    &turn.context_tokens,
                    &turn.generated_ids,
                    true,
                    true,
                );
                turn.draft_proposed_total = turn
                    .draft_proposed_total
                    .saturating_add(outcome.draft_proposed);
                turn.draft_accepted_total = turn
                    .draft_accepted_total
                    .saturating_add(outcome.draft_accepted);
                if outcome.draft_proposed > 0 {
                    let ratio_permille = ((outcome.draft_accepted as u64) * 1000
                        / (outcome.draft_proposed as u64))
                        as u16;
                    turn.accept_ema_permille = ((u32::from(turn.accept_ema_permille) * 3
                        + u32::from(ratio_permille))
                        / 4) as u16;
                }

                let mut stopped_early = false;
                for (idx, &tok_id) in outcome.accepted_tokens.iter().enumerate() {
                    turn.generated_ids.push(tok_id);
                    turn.context_tokens.push(tok_id);
                    emitted_in_slice += 1;
                    if let Some(tok_bytes) = model.tokenizer.id_to_raw.get(tok_id as usize) {
                        turn.raw_stream.extend_from_slice(tok_bytes);
                        let flushed =
                            flush_incremental_utf8(&turn.raw_stream, &mut turn.emitted_raw, false);
                        slice_delta.push_str(&flushed);
                    }
                    let mut matched_stop: Option<String> = None;
                    {
                        let cur_text = String::from_utf8_lossy(&turn.raw_stream);
                        for seq in &turn.stop_sequences {
                            if !seq.is_empty() && cur_text.ends_with(seq) {
                                matched_stop = Some(seq.clone());
                                break;
                            }
                        }
                    }
                    if let Some(seq) = matched_stop {
                        let keep_kv = kv_before + idx + 1;
                        if model.kv_cache.tokens.len() > keep_kv {
                            model.kv_cache.truncate_to(
                                keep_kv,
                                model.n_kv_heads,
                                model.head_dim,
                            );
                        }
                        status = "done";
                        stop_reason = Some(format!("stop:{seq}"));
                        stopped_early = true;
                        break;
                    }
                    if turn.generated_ids.len() >= turn.max_new_tokens
                        || model.kv_cache.tokens.len() >= model.max_seq
                    {
                        let keep_kv = kv_before + idx + 1;
                        if model.kv_cache.tokens.len() > keep_kv {
                            model.kv_cache.truncate_to(
                                keep_kv,
                                model.n_kv_heads,
                                model.head_dim,
                            );
                        }
                        status = "done";
                        stop_reason = Some("length".to_string());
                        stopped_early = true;
                        break;
                    }
                }
                if stopped_early {
                    break;
                }
                if let Some(sc) = outcome.next_scored {
                    turn.last_scored = sc;
                }
            } else {
                turn.generated_ids.push(chosen_id);
                turn.context_tokens.push(chosen_id);
                emitted_in_slice += 1;
                if let Some(tok_bytes) = model.tokenizer.id_to_raw.get(chosen_id as usize) {
                    turn.raw_stream.extend_from_slice(tok_bytes);
                    let flushed =
                        flush_incremental_utf8(&turn.raw_stream, &mut turn.emitted_raw, false);
                    slice_delta.push_str(&flushed);
                }

                let mut matched_stop: Option<String> = None;
                {
                    let cur_text = String::from_utf8_lossy(&turn.raw_stream);
                    for seq in &turn.stop_sequences {
                        if !seq.is_empty() && cur_text.ends_with(seq) {
                            matched_stop = Some(seq.clone());
                            break;
                        }
                    }
                }

                let need_logits = matched_stop.is_none()
                    && turn.generated_ids.len() < turn.max_new_tokens
                    && model.kv_cache.tokens.len() + 1 < model.max_seq;

                let (sc_opt, _, _) =
                    model.step_token(chosen_id, &turn.context_tokens, need_logits, true);
                if let Some(sc) = sc_opt {
                    turn.last_scored = sc;
                }

                if let Some(seq) = matched_stop {
                    status = "done";
                    stop_reason = Some(format!("stop:{seq}"));
                    break;
                }
            }
        }

        if status == "done" {
            let tail = flush_incremental_utf8(&turn.raw_stream, &mut turn.emitted_raw, true);
            slice_delta.push_str(&tail);
            turn.active = false;
            let final_text = String::from_utf8_lossy(&turn.raw_stream).trim().to_string();
            model.kv_cache.turn_token_spans.push((
                turn.user_key.clone(),
                final_text.clone(),
                turn.generated_ids.clone(),
            ));
            if model.kv_cache.turn_token_spans.len() > 16 {
                let extra = model.kv_cache.turn_token_spans.len() - 16;
                model.kv_cache.turn_token_spans.drain(..extra);
            }
            let out = serde_json::json!({
                "ok": true,
                "status": "done",
                "turn_id": turn_id,
                "model_id": turn.model_id,
                "delta": slice_delta,
                "full_text": final_text,
                "tokens_emitted": turn.generated_ids.len(),
                "prompt_done": turn.pending_prefill.len(),
                "prompt_total": turn.pending_prefill.len(),
                "prefix_reused_tokens": turn.prefix_reused_tokens,
                "draft_proposed_total": turn.draft_proposed_total,
                "draft_accepted_total": turn.draft_accepted_total,
                "mlp_mode": model.mlp_mode_label(),
                "stop_reason": stop_reason,
                "float_ops": 0,
                "dense_matmul_ops": 0,
            })
            .to_string();
            eng.turn = None;
            return out;
        }

        let full_text = String::from_utf8_lossy(&turn.raw_stream).into_owned();
        let tokens_emitted = turn.generated_ids.len();
        let prompt_total = turn.pending_prefill.len();
        let model_id = turn.model_id.clone();
        eng.turn = Some(turn);

        serde_json::json!({
            "ok": true,
            "status": "streaming",
            "turn_id": turn_id,
            "model_id": model_id,
            "delta": slice_delta,
            "full_text": full_text,
            "tokens_emitted": tokens_emitted,
            "prompt_done": prompt_total,
            "prompt_total": prompt_total,
            "stop_reason": null,
            "float_ops": 0,
            "dense_matmul_ops": 0,
        })
        .to_string()
    })
}

#[wasm_bindgen]
pub fn v13_cancel_turn(turn_id: u64) -> String {
    ENGINE.with(|cell| {
        let mut eng = cell.borrow_mut();
        let Some(mut turn) = eng.turn.take() else {
            return serde_json::json!({ "ok": true, "status": "idle", "turn_id": turn_id })
                .to_string();
        };
        if turn.turn_id != turn_id {
            eng.turn = Some(turn);
            return err_json("TurnNotFound", "V13 turn_id mismatch");
        }
        turn.cancelled = true;
        turn.active = false;
        let checkpoint = turn.kv_checkpoint_len;
        if let Some(model) = eng.model.as_mut() {
            if checkpoint == 0 {
                model.kv_cache.reset(model.n_layers);
            } else {
                model
                    .kv_cache
                    .truncate_to(checkpoint, model.n_kv_heads, model.head_dim);
            }
        }
        serde_json::json!({
            "ok": true,
            "status": "cancelled",
            "turn_id": turn_id,
            "tokens_emitted": turn.generated_ids.len(),
            "float_ops": 0,
        })
        .to_string()
    })
}

#[wasm_bindgen]
pub fn v13_stitch_verify_manifest(manifest_json: &str) -> String {
    let normalized = match serde_json::from_str::<serde_json::Value>(manifest_json.trim()) {
        Ok(serde_json::Value::Object(mut obj)) => {
            if !obj.contains_key("data_shard_count") {
                if let Some(pc) = obj.get("piece_count").cloned() {
                    obj.insert("data_shard_count".to_string(), pc);
                }
            }
            obj.entry("scheme".to_string())
                .or_insert_with(|| serde_json::Value::String("xor-2-of-3".to_string()));
            obj.entry("threshold".to_string())
                .or_insert_with(|| serde_json::Value::Number(2.into()));
            obj.entry("count".to_string())
                .or_insert_with(|| serde_json::Value::Number(3.into()));
            obj.entry("base_manifest".to_string())
                .or_insert_with(|| serde_json::Value::String("manifest.json".to_string()));
            serde_json::Value::Object(obj).to_string()
        }
        _ => manifest_json.to_string(),
    };
    match parse_and_validate_manifest(&normalized) {
        Ok(m) => serde_json::json!({
            "ok": true,
            "schema": m.schema,
            "format": m.format,
            "container_sha256": m.container_sha256,
            "group_count": m.groups.len(),
        })
        .to_string(),
        Err(e) => err_json(e.error_code(), e.to_string()),
    }
}

#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn v13_stitch_recover_pair(
    group_json: &str,
    recover_left: bool,
    has_left: bool,
    left: &[u8],
    has_right: bool,
    right: &[u8],
    has_parity: bool,
    parity: &[u8],
    expected_sha256: &str,
) -> Result<Vec<u8>, JsValue> {
    let group: StitchParityGroup = serde_json::from_str(group_json.trim())
        .map_err(|e| JsValue::from_str(&format!("StitchInvalidManifest: {e}")))?;
    let res = heal_2of3_pair(
        &group,
        has_left.then_some(left),
        has_right.then_some(right),
        has_parity.then_some(parity),
    )
    .map_err(|e| JsValue::from_str(&format!("{}: {e}", e.error_code())))?;
    let bytes = if recover_left {
        res.left_bytes
    } else {
        res.right_bytes
            .ok_or_else(|| JsValue::from_str("StitchInvalidManifest: group has no right shard"))?
    };
    let expected = expected_sha256.trim().to_ascii_lowercase();
    if !expected.is_empty() && sha256_hex(&bytes) != expected {
        return Err(JsValue::from_str(
            "StitchShaMismatch: recovered piece SHA-256 mismatch",
        ));
    }
    Ok(bytes)
}

#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn v13_stitch_recover_object(
    has_frag0: bool,
    frag0: &[u8],
    has_frag1: bool,
    frag1: &[u8],
    has_frag2: bool,
    frag2: &[u8],
    original_length: usize,
    expected_object_sha256: &str,
) -> Result<Vec<u8>, JsValue> {
    let mut provided: Vec<(u8, &[u8])> = Vec::with_capacity(3);
    if has_frag0 {
        provided.push((0, frag0));
    }
    if has_frag1 {
        provided.push((1, frag1));
    }
    if has_frag2 {
        provided.push((2, frag2));
    }
    let sha_opt = if expected_object_sha256.trim().is_empty() {
        None
    } else {
        Some(expected_object_sha256.trim())
    };
    recover_2of3_object(&provided, original_length, sha_opt, None)
        .map_err(|e| JsValue::from_str(&format!("{}: {e}", e.error_code())))
}
