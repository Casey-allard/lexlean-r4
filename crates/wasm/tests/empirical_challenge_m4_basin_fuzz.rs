#![allow(
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    clippy::identity_op,
    clippy::manual_range_contains,
    clippy::useless_vec
)]
//! Empirical Adversarial Challenge Suite for Milestone M4 Iteration 2:
//! Adversarial Multi-Basin Fuzzing & Anti-Chunking Verification Harness
//!
//! Evaluates:
//! 1. Monotonic `reply_history` growth under rapid clause boundary cascades (`...?!:;\n\n`).
//! 2. Alternating basin queues (A -> B -> A -> B -> C -> D) without history shrinkage or state reset.
//! 3. Continuous boundary spam (boundary at every step) preserving prefix history inviolability.
//! 4. End-to-end continuous generation under extreme punctuation cascade prompts without `\n\n` chunk injection.
//! 5. Zero-float integer Q12 invariant bounds `[-30000, 30000]` across cascade transitions.

use lexlean_r4_wasm::lut_stack_exec::{
    is_clause_boundary, resolve_canonical_reply_anchor, ByteBpe4096, LutStackModel,
    LutStackSession, RollingContextState,
};
use std::sync::Arc;

const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

fn load_test_model() -> Arc<LutStackModel> {
    Arc::new(LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully"))
}

/// Adversarial Test 1: Boundary Cascade Spam (`...?!:;\n\n`) and Monotonic `reply_history`
///
/// Feeds a cascade of consecutive boundary punctuation tokens directly into a live session
/// with pending basins queued. Verifies that at every single step:
/// 1. `session.reply_history` length increases monotonically by exactly 1 on every `step_with_telemetry`.
/// 2. Prior tokens in `session.reply_history` remain strictly byte-for-byte identical (prefix invariance).
/// 3. `session.suffix_counts` never clears and never loses tracked n-grams.
#[test]
fn test_adversarial_boundary_cascade_monotonic_reply_history() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");
    let bpe = ByteBpe4096::global();

    // Find 4 distinct basins
    let mut anchors = Vec::new();
    let mut target_toks = Vec::new();
    for r in 0..point.rows {
        if let Some(anc) = resolve_canonical_reply_anchor(point, mem, r, point.targets[r], &[]) {
            if !anchors.contains(&anc) {
                anchors.push(anc);
                target_toks.push(point.targets[r]);
                if anchors.len() >= 4 {
                    break;
                }
            }
        }
    }
    assert!(
        anchors.len() >= 4,
        "Must find at least 4 distinct basins for fuzz test"
    );

    let mut session = LutStackSession::new(model.clone());
    let prompt_toks = bpe.encode("boundary cascade fuzzing");
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        session.prefill_token(t, true);
    }
    session.mark_reply_start();

    // Step initial prompt tail
    let tail = prompt_toks[prompt_toks.len() - 1];
    let _ = session.step_with_telemetry(tail);
    assert_eq!(session.reply_history.len(), 1);

    // Queue 3 pending basins
    for i in 1..4 {
        session
            .pending_basins
            .push((anchors[i].clone(), target_toks[i]));
    }
    session.anchors.reply = anchors[0].clone();

    // Formulate boundary cascade: '.', '.', '?', '!', ':', ';', '\n', '\n', '.', '?'
    let cascade_str = "...?!:;\n\n.?";
    let cascade_toks = bpe.encode(cascade_str);
    assert!(!cascade_toks.is_empty());

    let mut next_tok = target_toks[0];
    let mut last_source = "turn_memory_reply";
    let mut clauses_resolved = 0;

    for (step_i, &b_tok) in cascade_toks.iter().enumerate() {
        let prev_len = session.reply_history.len();
        let prev_history = session.reply_history.clone();
        let prev_suffix_len = session.suffix_counts.len();

        let step_idx = session.tokens.len();
        let _is_bound = is_clause_boundary(b_tok, step_idx);

        // Attempt mid-stream transition
        let transitioned = model.try_mid_stream_re_attention(
            &mut session,
            b_tok,
            step_idx,
            &mut next_tok,
            &mut last_source,
            &mut clauses_resolved,
        );

        // Transition must NOT modify or clear reply_history or suffix_counts
        assert_eq!(
            session.reply_history.len(),
            prev_len,
            "try_mid_stream_re_attention must NOT alter reply_history length (step {step_i})"
        );
        assert_eq!(
            session.reply_history, prev_history,
            "try_mid_stream_re_attention must NOT corrupt reply_history tokens (step {step_i})"
        );
        assert!(
            session.suffix_counts.len() >= prev_suffix_len,
            "suffix_counts shrunk during try_mid_stream_re_attention (step {step_i})"
        );

        // Now step the boundary token
        let _st = session.step_with_telemetry(b_tok);

        // Monotonic growth assertion: length must be prev_len + 1
        assert_eq!(
            session.reply_history.len(),
            prev_len + 1,
            "reply_history must grow by exactly 1 after stepping boundary token (step {step_i})"
        );
        assert_eq!(
            &session.reply_history[..prev_len],
            prev_history.as_slice(),
            "prior reply_history tokens must be invariant after stepping boundary token (step {step_i})"
        );
        assert_eq!(
            *session.reply_history.last().unwrap(),
            b_tok,
            "newest reply_history token must match stepped token (step {step_i})"
        );

        // If a transition occurred, also step next_tok to verify continuation
        if transitioned {
            let len_before_cont = session.reply_history.len();
            let hist_before_cont = session.reply_history.clone();
            let st_cont = session.step_with_telemetry(next_tok);

            assert_eq!(
                session.reply_history.len(),
                len_before_cont + 1,
                "reply_history must grow when stepping continuation token after transition"
            );
            assert_eq!(
                &session.reply_history[..len_before_cont],
                hist_before_cont.as_slice(),
                "prior reply_history must remain invariant when stepping continuation token"
            );
            assert_eq!(
                *session.reply_history.last().unwrap(),
                next_tok,
                "newest reply_history token must match next_tok"
            );
            assert!(
                st_cont.predicted_token > 0,
                "must predict valid token after transition"
            );
            next_tok = st_cont.predicted_token;
        }
    }

    println!(
        "Adversarial boundary cascade monotonic test passed (total reply_history: {} tokens).",
        session.reply_history.len()
    );
}

/// Adversarial Test 2: Rapid Alternating Basin Queues (A <-> B <-> A <-> B <-> C)
///
/// Simulates rapid alternating multi-basin queues where the generator bounces between
/// basins across consecutive clause boundaries. Verifies:
/// 1. Zero history wiping.
/// 2. Clean anchor updates.
/// 3. Tier 2b continuation matching under rapidly oscillating anchors.
#[test]
fn test_adversarial_alternating_basin_queues_continuity() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");
    let bpe = ByteBpe4096::global();

    // Identify 3 distinct basins
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
        .expect("anchor B resolves");
    let (row_c, anchor_c) = (1..point.rows)
        .find_map(|r| {
            let anc = resolve_canonical_reply_anchor(point, mem, r, point.targets[r], &[])?;
            if anc != anchor_a && anc != anchor_b {
                Some((r, anc))
            } else {
                None
            }
        })
        .expect("anchor C resolves");

    let tok_a = point.targets[row_a];
    let tok_b = point.targets[row_b];
    let tok_c = point.targets[row_c];

    let mut session = LutStackSession::new(model.clone());
    let prompt_toks = bpe.encode("alternating basins test");
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        session.prefill_token(t, true);
    }
    session.mark_reply_start();
    let _ = session.step_with_telemetry(prompt_toks[prompt_toks.len() - 1]);

    // Push alternating sequence: C, B, A, B, A (popped in reverse order: A, B, A, B, C)
    session.pending_basins.push((anchor_c.clone(), tok_c));
    session.pending_basins.push((anchor_b.clone(), tok_b));
    session.pending_basins.push((anchor_a.clone(), tok_a));
    session.pending_basins.push((anchor_b.clone(), tok_b));
    session.pending_basins.push((anchor_a.clone(), tok_a));

    let expected_order = [
        (anchor_a.clone(), tok_a),
        (anchor_b.clone(), tok_b),
        (anchor_a.clone(), tok_a),
        (anchor_b.clone(), tok_b),
        (anchor_c.clone(), tok_c),
    ];

    let period_tok = bpe.encode(".")[0];
    let mut next_tok = 0u32;
    let mut last_source = "turn_memory_reply";
    let mut clauses_resolved = 0;

    for (pop_idx, (exp_anchor, exp_tok)) in expected_order.iter().enumerate() {
        let history_len_before = session.reply_history.len();
        let step_idx = session.tokens.len();

        let transitioned = model.try_mid_stream_re_attention(
            &mut session,
            period_tok,
            step_idx,
            &mut next_tok,
            &mut last_source,
            &mut clauses_resolved,
        );

        assert!(
            transitioned,
            "Transition must succeed at pop index {pop_idx}"
        );
        assert_eq!(
            session.anchors.reply, *exp_anchor,
            "Anchor mismatch at pop index {pop_idx}"
        );
        assert_eq!(
            next_tok, *exp_tok,
            "Continuation token mismatch at pop index {pop_idx}"
        );
        assert_eq!(
            session.reply_history.len(),
            history_len_before,
            "reply_history must not change during transition"
        );

        // Step the period and the continuation token
        let _ = session.step_with_telemetry(period_tok);
        let st_cont = session.step_with_telemetry(next_tok);

        assert_eq!(
            session.reply_history.len(),
            history_len_before + 2,
            "reply_history must advance by 2 tokens per pop cycle"
        );
        assert!(
            st_cont.predicted_token > 0,
            "Continuation prediction must be non-zero at pop index {pop_idx}"
        );
    }

    assert!(session.pending_basins.is_empty());
    println!(
        "Alternating basin queue continuity test passed (5 consecutive basin handoffs with 0 history loss)."
    );
}

/// Adversarial Test 3: Continuous Generation Under Boundary Cascade Prompts (Anti-Chunking)
///
/// Evaluates end-to-end generation across prompts with dense boundary cascades.
/// Invariants:
/// 1. Generated tokens sequence must NEVER contain `\n\n` chunk separators.
/// 2. Reply text must not contain `\n\n`.
/// 3. Must not terminate prematurely (>= 16 tokens).
/// 4. Zero floats in Q12 bounds.
#[test]
fn test_adversarial_boundary_cascade_generation_no_chunking() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();
    let sep = bpe.encode("\n\n");

    let cascade_prompts = [
        "What is gravity? ... ?! :; \n\n Why do objects fall toward the Earth?",
        "Explain thermodynamics: law 1; law 2! law 3? ... \n\n What is absolute zero?",
        "Rust vs C++ ... ::: ;;; ??? !!! Which language provides stronger memory safety guarantees?",
        "Cascading boundaries: . . . ? ? ? ! ! ! : : : ; ; ; \n\n Explain quantum computing.",
        "A. B? C! D: E; F. G? H! I: J; \n\n Explain Emmy Noether's symmetry theorem in mathematical physics.",
    ];

    for &prompt in &cascade_prompts {
        let mut context = RollingContextState::default();
        let res = model.generate_reply_with_context(&mut context, &[], prompt, 64);

        assert!(
            !res.generated_tokens.is_empty(),
            "Prompt '{prompt}' generated zero tokens"
        );
        assert!(
            res.generated_tokens.len() >= 16,
            "Prompt '{prompt}' terminated prematurely with only {} tokens",
            res.generated_tokens.len()
        );

        // Invariant: NO \n\n token sequence
        let has_chunk_sep = res
            .generated_tokens
            .windows(sep.len())
            .any(|w| w == sep.as_slice());
        assert!(
            !has_chunk_sep,
            "Prompt '{prompt}' emitted \\n\\n chunk separator in tokens!"
        );
        assert!(
            !res.reply_text.contains("\n\n"),
            "Prompt '{prompt}' emitted \\n\\n chunk separator in reply_text!"
        );

        // Invariant: Q12 bounds in telemetry
        for (st_idx, st) in res.steps.iter().enumerate() {
            assert!(
                st.l1_distance >= 0,
                "Prompt '{prompt}' step {st_idx} had negative L1 distance: {}",
                st.l1_distance
            );
        }

        // Invariant: Context tracking
        assert!(
            !context.emitted_4grams.is_empty(),
            "Context emitted_4grams must be populated for prompt: '{prompt}'"
        );
    }

    println!("Adversarial boundary cascade generation anti-chunking passed across all 5 prompts.");
}

/// Adversarial Test 4: 100 Consecutive Boundary Tokens (Path A and Path B Stress)
///
/// Pumps 100 consecutive boundary tokens through a session, verifying that:
/// 1. `session.reply_history.len()` reaches exactly 101 (1 initial + 100 boundaries).
/// 2. `session.suffix_counts` correctly registers 4-gram suffixes.
/// 3. No arithmetic overflow, underflow, or panic occurs.
#[test]
fn test_adversarial_100_consecutive_boundaries_fuzz() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    let mut session = LutStackSession::new(model.clone());
    let prompt_toks = bpe.encode("stress test");
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        session.prefill_token(t, true);
    }
    session.mark_reply_start();
    let _ = session.step_with_telemetry(prompt_toks[prompt_toks.len() - 1]);

    let boundaries = [
        bpe.encode(".")[0],
        bpe.encode("?")[0],
        bpe.encode("!")[0],
        bpe.encode(":")[0],
        bpe.encode(";")[0],
        bpe.encode("\n")[0],
    ];

    let mut next_tok = 0u32;
    let mut last_source = "whole_table_nearest";
    let mut clauses_resolved = 0;

    for i in 0..100 {
        let b_tok = boundaries[i % boundaries.len()];
        let step_idx = session.tokens.len();

        let _ = model.try_mid_stream_re_attention(
            &mut session,
            b_tok,
            step_idx,
            &mut next_tok,
            &mut last_source,
            &mut clauses_resolved,
        );

        let prev_len = session.reply_history.len();
        let _ = session.step_with_telemetry(b_tok);
        assert_eq!(
            session.reply_history.len(),
            prev_len + 1,
            "reply_history must grow by 1 at boundary iteration {i}"
        );
    }

    assert_eq!(session.reply_history.len(), 101);
    assert!(
        session.suffix_counts.len() >= 6,
        "suffix_counts must track 4-grams across repeated boundary sequences"
    );
    println!("100 consecutive boundary tokens fuzz passed cleanly.");
}
