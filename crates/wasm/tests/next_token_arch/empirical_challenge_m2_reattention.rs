//! Empirical Challenge Suite for Milestone M2:
//! Dynamic Sub-Sentence Re-Attention, Anti-Chunking, and Continuous Generation (R1)
//!
//! Evaluates:
//! 1. Mid-stream dynamic transition across clause boundaries (. ? ! : ; \n, K=16).
//! 2. Complete absence of \n\n chunk separators in multi-clause replies.
//! 3. Preservation of session reply_history and suffix counts across basin hand-off.
//! 4. Continuous generation depth extending smoothly past initial training turn row.
//! 5. 8-Head R4.Flock cyclic frame block permutation and invariant properties.
//! 6. Zero-float integer Q12 invariant conformance across all inference paths.

use lexlean_r4_wasm::lut_stack_exec::{
    is_boundary_token, is_clause_boundary, modulate_flock_cyclic_attention,
    modulate_flock_cyclic_frame_block, resolve_canonical_reply_anchor, ByteBpe4096, LutStackModel,
    LutStackSession, FBS_WIDTH,
};
use std::sync::Arc;

use crate::BOOTSTRAP_BYTES;

fn load_test_model() -> Arc<LutStackModel> {
    crate::shared_bootstrap_model_arc()
}

#[test]
fn challenge_m2_multi_clause_smooth_transition_no_chunking() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    let test_prompts = [
        "Explain quantum entanglement in clear terms and also tell me about Emmy Noether.",
        "Explain quantum entanglement, Emmy Noether, and GPS location on Earth",
    ];

    for prompt in test_prompts {
        let res = model.generate_reply(&[], prompt, 256);
        assert!(
            !res.generated_tokens.is_empty(),
            "generation must succeed for: {}",
            prompt
        );

        // Anti-chunking check: no "\n\n" sequence
        let sep = bpe.encode("\n\n");
        let has_chunk_sep = res
            .generated_tokens
            .windows(sep.len())
            .any(|w| w == sep.as_slice());
        assert!(
            !has_chunk_sep,
            "prompt '{}' produced chunked reply with \\n\\n: '{}'",
            prompt, res.reply_text
        );
        assert!(
            !res.reply_text.contains("\n\n"),
            "reply text must not contain double newlines: '{}'",
            res.reply_text
        );

        // Continuous generation check: more than 16 tokens generated
        assert!(
            res.generated_tokens.len() >= 16,
            "multi-clause prompt must generate at least 16 tokens, got {}",
            res.generated_tokens.len()
        );
    }
}

#[test]
fn challenge_m2_clause_boundary_complete_token_sweep() {
    let bpe = ByteBpe4096::global();

    // Punctuation tokens must trigger boundary
    let punctuation = [".", "?", "!", ":", ";", "\n"];
    for p in punctuation {
        let enc = bpe.encode(p);
        assert!(!enc.is_empty());
        let tok = enc[enc.len() - 1];
        assert!(
            is_boundary_token(tok),
            "punctuation '{}' (tok {}) must be recognized as boundary token",
            p,
            tok
        );
        assert!(
            is_clause_boundary(tok, 1),
            "punctuation '{}' (tok {}) must be clause boundary at step 1",
            p,
            tok
        );
    }

    // Step interval K=16 must trigger for every step that is a multiple of 16
    let dummy_tok = bpe.encode("test")[0];
    for step in 1..=64 {
        let is_interval = step % 16 == 0;
        let boundary = is_clause_boundary(dummy_tok, step);
        assert_eq!(
            boundary, is_interval,
            "step {} for non-punctuation token must be {}",
            step, is_interval
        );
    }
}

#[test]
fn challenge_m2_continuous_generation_depth_exceeds_training_turn() {
    let model = load_test_model();
    let res = model.generate_reply(&[], "hello", 48);

    assert!(
        res.generated_tokens.len() >= 8,
        "even basic greetings must generate past initial memory hit without premature termination, got {}",
        res.generated_tokens.len()
    );

    // Verify fallback mechanism was reached
    let fallback_reached = res.steps.iter().any(|st| {
        st.source_mechanism != "turn_memory_opening" && st.source_mechanism != "turn_memory_reply"
    });
    assert!(
        fallback_reached,
        "fallback mechanism must be exercised without forcing EOS"
    );
}

#[test]
fn challenge_m2_zero_float_conformance_and_q12_clamping() {
    let mut n_t = [0i32; FBS_WIDTH];
    for d in 0..FBS_WIDTH {
        n_t[d] = if d % 2 == 0 { 25000 } else { -25000 };
    }

    for sweep in 0..8usize {
        let modulated = modulate_flock_cyclic_attention(&n_t, sweep);
        for &coord in &modulated {
            assert!(
                coord.abs() <= 30000,
                "Q12 coordinate out of bounds: {}",
                coord
            );
        }
    }
}

#[test]
fn challenge_m2_8head_cyclic_permutation_bijectivity() {
    let mut n_t = [0i32; FBS_WIDTH];
    for h in 0..8 {
        for d in 0..32 {
            n_t[h * 32 + d] = (h as i32 + 1) * 1000 + (d as i32);
        }
    }

    // For each sweep 0..8, verify that modulate_flock_cyclic_frame_block is a bijection
    for sweep in 0..8usize {
        let modulated = modulate_flock_cyclic_frame_block(&n_t, sweep);
        let mut seen_heads = [false; 8];
        for h in 0..8 {
            let base_val = modulated[h * 32];
            let source_head = (base_val / 1000 - 1) as usize;
            assert!(source_head < 8, "source head must be valid [0, 8)");
            assert!(
                !seen_heads[source_head],
                "source head {} must not be duplicated in sweep {}",
                source_head, sweep
            );
            seen_heads[source_head] = true;
        }
        for (sh, &seen) in seen_heads.iter().enumerate() {
            assert!(seen, "head {} must be covered in sweep {}", sh, sweep);
        }
    }
}

#[test]
fn challenge_m2_midstream_re_attention_dynamic_switch() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");
    let bpe = ByteBpe4096::global();

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

    let mut session = LutStackSession::new(model.clone());
    let prompt_toks = bpe.encode("test query");
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        session.prefill_token(t, true);
    }
    session.mark_reply_start();
    session.step_with_telemetry(prompt_toks[prompt_toks.len() - 1]);

    session.anchors.reply = anchor_a.clone();
    let first_tok_b = point.targets[row_b];
    session.pending_basins.push((anchor_b.clone(), first_tok_b));

    let period_tok = bpe.encode(".")[0];
    let _ = session.step_with_telemetry(period_tok);

    let mut next_tok = 0u32;
    let mut last_source = "turn_memory_reply";
    let mut clauses_resolved = 1;

    let transitioned = model.try_mid_stream_re_attention(
        &mut session,
        period_tok,
        1,
        &mut next_tok,
        &mut last_source,
        &mut clauses_resolved,
    );

    assert!(
        transitioned,
        "mid-stream re-attention must transition at period"
    );
    assert_eq!(
        session.anchors.reply, anchor_b,
        "anchors.reply must update to anchor_b"
    );
    assert_eq!(
        next_tok, first_tok_b,
        "next_tok must point to first token of new basin"
    );
    assert_eq!(last_source, "turn_memory_reply");
}
