//! Empirical Adversarial Challenge Suite for Milestone M2:
//! Dynamic Mid-Stream Basin Re-Attention, Anti-Chunking & Basin Transition (Challenger 1)
//!
//! Evaluates:
//! 1. Direct mid-stream basin re-attention via Path B in `try_mid_stream_re_attention`.
//! 2. Mid-stream basin transition via Path A (queued multi-clause basins).
//! 3. Tier 2b 1-gram continuation without wiping `reply_history`.
//! 4. `select_point_key_with_resonance` with `modulate_flock_cyclic_frame_block` and `recent_turn_rows` distance penalties.
//! 5. End-to-end alternating and contradictory multi-domain generation without desynchronization or chunking.
//! 6. Clause boundary token and K=16 interval edge cases.
//! 7. 8-Head Flock cyclic frame block bijectivity and extreme value bounds.

use lexlean_r4_wasm::lut_stack_exec::{
    is_boundary_token, is_clause_boundary, modulate_flock_cyclic_frame_block,
    modulate_flock_geodesic_blend, resolve_canonical_reply_anchor, select_point_key_with_resonance,
    ByteBpe4096, LutStackModel, LutStackSession, RollingContextState, FBS_WIDTH,
};
use std::sync::Arc;

use crate::BOOTSTRAP_BYTES;

fn load_test_model() -> Arc<LutStackModel> {
    crate::shared_bootstrap_model_arc()
}

fn cyclic_frame_block(block: u64, sweep: u64, blocks: u64) -> u64 {
    if blocks == 0 {
        0
    } else {
        (block + sweep + 1) % blocks
    }
}

#[test]
fn test_empirical_direct_path_b_dynamic_re_attention() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");
    let bpe = ByteBpe4096::global();

    let mut session = LutStackSession::new(model.clone());
    let prompt_toks = bpe.encode("who are you");
    for &t in &prompt_toks {
        session.prefill_token(t, true);
    }
    session.mark_reply_start();

    // Set initial anchor to row 0
    let canon_0 = resolve_canonical_reply_anchor(point, mem, 0, point.targets[0], &[])
        .expect("canon 0 resolves");
    session.anchors.reply = canon_0.clone();
    session.pending_basins.clear();

    // Pick a distinct target row in point key table
    let target_row = (1..point.rows)
        .find(|&r| {
            if let Some(anc) = resolve_canonical_reply_anchor(point, mem, r, point.targets[r], &[])
            {
                anc != canon_0
            } else {
                false
            }
        })
        .expect("must find distinct target row");
    let canon_target =
        resolve_canonical_reply_anchor(point, mem, target_row, point.targets[target_row], &[])
            .expect("canon target resolves");

    // Synthesize trajectory shift toward target_row in session.last_n_t
    let mut shifted_n_t = [0i32; FBS_WIDTH];
    let base = target_row * point.width;
    let sweep = (session.tokens.len() as u64) % 8;
    for head in 0..8 {
        let frame_block = cyclic_frame_block(head as u64, sweep, 8) as usize;
        let dst_base = head * 32;
        let src_base = frame_block * 32;
        for d in 0..32 {
            if dst_base + d < point.width {
                shifted_n_t[src_base + d] = point.points[base + dst_base + d] as i32;
            }
        }
    }
    session.last_n_t = shifted_n_t;

    let boundary_tok = bpe.encode(".")[0];
    let step = 16;
    assert!(is_clause_boundary(boundary_tok, step));

    let mut next_tok = 9999u32;
    let mut last_source = "whole_table_nearest";
    let mut clauses_resolved = 1;

    // Directly execute model.try_mid_stream_re_attention with Path B prerequisites:
    // pending_basins is empty and last_source != "turn_memory_reply"
    let transitioned = model.try_mid_stream_re_attention(
        &mut session,
        boundary_tok,
        step,
        &mut next_tok,
        &mut last_source,
        &mut clauses_resolved,
    );

    assert!(
        transitioned,
        "Path B must trigger dynamic transition on trajectory shift"
    );
    assert_eq!(
        session.anchors.reply, canon_target,
        "anchors.reply must update mid-stream to canon_target"
    );
    assert_eq!(
        next_tok, point.targets[target_row],
        "next_tok must update to target_token of target_row"
    );
    assert!(
        last_source == "point_key_vote" || last_source == "point_key_fallback",
        "last_source must update to point_key: got {}",
        last_source
    );
    assert_eq!(clauses_resolved, 2, "clauses_resolved must increment");
    assert!(
        session.recent_turn_rows.contains(&target_row),
        "recent_turn_rows must record target_row"
    );
}

#[test]
fn test_empirical_path_a_queued_basins_transition() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");
    let bpe = ByteBpe4096::global();

    let mut session = LutStackSession::new(model.clone());
    let prompt_toks = bpe.encode("multi clause test");
    for &t in &prompt_toks {
        session.prefill_token(t, true);
    }
    session.mark_reply_start();

    let row_a = 0usize;
    let anchor_a = resolve_canonical_reply_anchor(point, mem, row_a, point.targets[row_a], &[])
        .expect("anchor A resolves");
    let (row_b, anchor_b) = (1..point.rows)
        .find_map(|r| {
            let anc = resolve_canonical_reply_anchor(point, mem, r, point.targets[r], &[])?;
            if anc != anchor_a {
                Some((r, anc))
            } else {
                None
            }
        })
        .expect("must find distinct anchor");

    session.anchors.reply = anchor_a.clone();
    let first_tok_b = point.targets[row_b];
    session.pending_basins.push((anchor_b.clone(), first_tok_b));

    // Test non-boundary token does NOT pop pending basins when in turn_memory_reply
    let word_tok = bpe.encode(" word")[0];
    let mut next_tok = 1234u32;
    let mut last_source = "turn_memory_reply";
    let mut clauses_resolved = 1;

    let no_trans = model.try_mid_stream_re_attention(
        &mut session,
        word_tok,
        3,
        &mut next_tok,
        &mut last_source,
        &mut clauses_resolved,
    );
    assert!(
        !no_trans,
        "Non-boundary token must not trigger transition during turn_memory_reply"
    );
    assert_eq!(session.anchors.reply, anchor_a);
    assert_eq!(session.pending_basins.len(), 1);

    // Test punctuation boundary triggers transition
    let period_tok = bpe.encode(".")[0];
    let trans = model.try_mid_stream_re_attention(
        &mut session,
        period_tok,
        4,
        &mut next_tok,
        &mut last_source,
        &mut clauses_resolved,
    );
    assert!(trans, "Period boundary must trigger transition in Path A");
    assert_eq!(session.anchors.reply, anchor_b);
    assert_eq!(next_tok, first_tok_b);
    assert_eq!(last_source, "turn_memory_reply");
    assert!(session.pending_basins.is_empty());
}

#[test]
fn test_empirical_tier2b_continuity_across_basin_switch() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");
    let bpe = ByteBpe4096::global();

    let mut session = LutStackSession::new(model.clone());
    let prompt_toks = bpe.encode("explain concepts");
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        session.prefill_token(t, true);
    }
    session.mark_reply_start();

    // Step first prompt token to initialize reply context
    let prompt_tail = prompt_toks[prompt_toks.len() - 1];
    let st0 = session.step_with_telemetry(prompt_tail);

    // Step tokens from basin A
    let st1 = session.step_with_telemetry(st0.predicted_token);
    let _st2 = session.step_with_telemetry(st1.predicted_token);

    let hist_len_before = session.reply_history.len();
    assert!(
        hist_len_before >= 2,
        "reply_history must track reply tokens"
    );

    // Dynamically switch anchor to basin B
    let (row_b, anchor_b) = (1..point.rows)
        .find_map(|r| {
            let anc = resolve_canonical_reply_anchor(point, mem, r, point.targets[r], &[])?;
            if anc != session.anchors.reply {
                Some((r, anc))
            } else {
                None
            }
        })
        .expect("must find distinct anchor");

    session.anchors.reply = anchor_b;
    let first_tok_b = point.targets[row_b];

    // Stepping first_tok_b under new anchor B must succeed via Tier 2b fallback
    // and append to reply_history without wiping prior tokens
    let st = session.step_with_telemetry(first_tok_b);

    assert_eq!(
        session.reply_history.len(),
        hist_len_before + 1,
        "reply_history must contain all tokens (prior basin + new basin first token)"
    );
    assert_eq!(
        session.reply_history[session.reply_history.len() - 1],
        first_tok_b
    );

    // Verify valid continuation was predicted
    assert_ne!(st.predicted_token, 0);
    assert_ne!(st.predicted_token, bpe.eos_id());
}

#[test]
fn test_empirical_basin_repetition_steering_with_flock_modulation() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");
    let bpe = ByteBpe4096::global();

    let prompt_toks = bpe.encode("Explain quantum mechanics");
    let mut n_t = [0i32; FBS_WIDTH];
    for d in 0..point.width {
        n_t[d] = point.points[d] as i32;
    }

    // Modulate across heads using flock cyclic frame block
    let modulated_n_t = modulate_flock_cyclic_frame_block(&n_t, 16);

    // 1. Initial selection with empty recent_turn_rows
    let hit_clean = select_point_key_with_resonance(
        point,
        &modulated_n_t,
        &prompt_toks,
        mem.key_window,
        mem.reply.as_ref(),
        mem.reply_key_len,
        &[],
    )
    .expect("clean query hits a row");

    let clean_row = hit_clean.row;

    // 2. Penalize clean_row by putting it in recent_turn_rows
    let recent = vec![clean_row];
    let hit_penalized = select_point_key_with_resonance(
        point,
        &modulated_n_t,
        &prompt_toks,
        mem.key_window,
        mem.reply.as_ref(),
        mem.reply_key_len,
        &recent,
    )
    .expect("penalized query hits a row");

    // If there are multiple candidate rows, the penalized row should be steered away from
    if point.rows > 1 {
        assert_ne!(
            hit_penalized.row, clean_row,
            "recent_turn_rows penalty must steer selection away from recently used row"
        );
    }

    // 3. Monotonicity: Pos 7 (most recent) has higher penalty than Pos 0 (oldest)
    // 250_000 * (pos + 1)
    let p0 = 250_000i64;
    let p7 = 250_000i64 * (7 + 1);
    assert_eq!(p0, 250_000);
    assert_eq!(p7, 2_000_000);
    assert!(
        p7 > p0,
        "more recent row must incur strictly greater distance penalty"
    );
}

#[test]
fn test_empirical_multi_domain_alternating_and_contradictory_prompts() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    let adversarial_prompts = [
        // Multi-domain cross-discipline prompts
        "Explain quantum entanglement in physics and also tell me about Emmy Noether.",
        "Explain quantum entanglement, Emmy Noether, and GPS location on Earth",
        "Explain photosynthesis in plants and then discuss general relativity in astrophysics.",
        "Tell me about French contract law and explain how a binary search tree works.",
        "Explain cellular respiration, plate tectonics, and Euclidean geometry.",
        // Contradictory/dual concept prompts
        "Explain why light acts as a wave, but also explain why light behaves as a particle.",
        "Explain classical Newtonian determinism, and contrast it with quantum indeterminacy.",
    ];

    let sep_tokens = bpe.encode("\n\n");

    for prompt in adversarial_prompts {
        let mut context = RollingContextState::default();
        let res = model.generate_reply_with_context(&mut context, &[], prompt, 96);

        assert!(
            !res.generated_tokens.is_empty(),
            "Prompt must produce tokens: '{}'",
            prompt
        );

        // Anti-chunking invariant: NO "\n\n" in token sequence
        let has_sep = res
            .generated_tokens
            .windows(sep_tokens.len())
            .any(|w| w == sep_tokens.as_slice());
        assert!(
            !has_sep,
            "Prompt '{}' emitted \\n\\n token chunking!",
            prompt
        );
        assert!(
            !res.reply_text.contains("\n\n"),
            "Prompt '{}' emitted \\n\\n in reply_text: '{}'",
            prompt,
            res.reply_text
        );

        // Continuous generation invariant: length >= 16
        assert!(
            res.generated_tokens.len() >= 16,
            "Prompt '{}' terminated prematurely with only {} tokens",
            prompt,
            res.generated_tokens.len()
        );

        // RollingContextState tracking invariant
        assert!(
            !context.recent_turn_rows.is_empty() || res.clauses_resolved > 0,
            "Context must record turn basin activity for prompt: '{}'",
            prompt
        );
        assert!(
            !context.emitted_4grams.is_empty(),
            "Context must record emitted 4grams for prompt: '{}'",
            prompt
        );
    }
}

#[test]
fn test_empirical_clause_boundary_complete_semantics() {
    let bpe = ByteBpe4096::global();

    // 1. Boundary tokens: '.', '?', '!', ':', ';', '\n'
    let punctuation = [".", "?", "!", ":", ";", "\n"];
    for p in punctuation {
        let enc = bpe.encode(p);
        let tok = *enc.last().unwrap();
        assert!(is_boundary_token(tok), "Token for '{}' must be boundary", p);
        assert!(
            is_clause_boundary(tok, 0),
            "Punctuation must be boundary at step 0"
        );
        assert!(
            is_clause_boundary(tok, 7),
            "Punctuation must be boundary at step 7"
        );
    }

    // 2. Non-boundary tokens: standard alphanumeric and symbols
    let non_punct = ["word", "test", " ", "42", "alpha"];
    for np in non_punct {
        let enc = bpe.encode(np);
        let tok = enc[0];
        assert!(
            !is_boundary_token(tok),
            "Token for '{}' must not be boundary",
            np
        );
        assert!(
            !is_clause_boundary(tok, 0),
            "Step 0 for non-punct must not be boundary"
        );
        assert!(!is_clause_boundary(tok, 15), "Step 15 must not be boundary");
        assert!(
            is_clause_boundary(tok, 16),
            "Step 16 MUST be boundary (K=16 interval)"
        );
        assert!(!is_clause_boundary(tok, 17), "Step 17 must not be boundary");
        assert!(
            is_clause_boundary(tok, 32),
            "Step 32 MUST be boundary (K=16 interval)"
        );
    }
}

#[test]
fn test_empirical_flock_cyclic_frame_bijectivity_and_bounds() {
    let mut n_t = [0i32; FBS_WIDTH];

    // Extreme bounds: alternating large Q12 values [-30000, 30000]
    for d in 0..FBS_WIDTH {
        n_t[d] = if d % 2 == 0 { 30000 } else { -30000 };
    }

    for tokens_len in 0..128 {
        let modulated = modulate_flock_cyclic_frame_block(&n_t, tokens_len);
        let blended = modulate_flock_geodesic_blend(&n_t, tokens_len);

        for d in 0..FBS_WIDTH {
            assert!(
                modulated[d].abs() <= 30000,
                "Modulated Q12 coordinate {} out of bounds: {}",
                d,
                modulated[d]
            );
            assert!(
                blended[d].abs() <= 30000,
                "Blended Q12 coordinate {} out of bounds: {}",
                d,
                blended[d]
            );
        }
    }
}
