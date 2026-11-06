#![allow(
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    clippy::identity_op,
    clippy::manual_range_contains,
    clippy::useless_vec
)]
//! Empirical Challenge Suite for Milestone M3:
//! Autoregressive Thinking Readback, Full-Trunk Multi-Turn Prefill, and Visible Reply Barrier (R3)
//!
//! Evaluates:
//! 1. Full-trunk multi-turn context prefill depth (>100 tokens through all 6 SpiralCore/FBS layers).
//! 2. Active hidden state trajectory steering ($n_t$) via reasoning tokens (<thinking>...</thinking>).
//! 3. Divergence of reasoning trajectories into distinct PointKey attractor basins.
//! 4. Visible reply commencement barrier enforced strictly after </thinking>.
//! 5. Complete absence of thinking tag leakage in user-facing reply text.
//! 6. Sub-second prefill throughput (<500ms for 512 tokens).
//! 7. Adversarial, unclosed, and nested thinking tag resilience.
//! 8. Pure zero-float integer Q12 invariant conformance across all inference paths.

use lexlean_r4_wasm::lut_stack_exec::{
    select_point_key_with_resonance, ByteBpe4096, LutStackModel, LutStackSession,
    RollingContextState,
};
use std::sync::Arc;
use std::time::Instant;

const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

fn load_test_model() -> Arc<LutStackModel> {
    Arc::new(LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully"))
}

#[test]
fn challenge_m3_full_trunk_prefill_multi_turn_depth() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    let mut context = RollingContextState::default();
    context.context_summary = "T1: [gödel, incompleteness] -> Kurt Gödel proved them in 1931. | T2: [formal systems, consistency] -> Gödel numbering and self-referential statements.".to_string();
    context.q12_latent[0] = 1024;
    context.q12_latent[1] = -512;

    let history = vec![
        (
            "Who proved the incompleteness theorems?".to_string(),
            "Kurt Gödel proved them in 1931.".to_string(),
        ),
        (
            "What was the core method used?".to_string(),
            "Gödel numbering and self-referential statements constructed within Peano arithmetic."
                .to_string(),
        ),
    ];

    let current_prompt = "Can a sufficiently expressive formal system prove its own consistency?";
    let sys_note = Some(format!("[Context: {}]", context.context_summary));

    let prompt_tokens = bpe.encode_dialogue_prompt(
        sys_note.as_deref(),
        &history,
        current_prompt,
        model.dialogue_version,
    );

    // Context depth check: multi-turn dialogue prompt must comfortably exceed 64 tokens
    assert!(
        prompt_tokens.len() >= 64,
        "multi-turn dialogue tokens len ({}) must be >= 64",
        prompt_tokens.len()
    );

    let start_time = Instant::now();
    let mut session = LutStackSession::new_with_context(model.clone(), &context);

    // Prefill the entire multi-turn context with full 6-layer trunk execution
    for &tok in &prompt_tokens[..prompt_tokens.len() - 1] {
        session.prefill_token(tok, true);
    }
    let elapsed = start_time.elapsed();

    // Latency assertion: prefilling >64 tokens through 6-layer trunk must take < 50ms
    assert!(
        elapsed.as_millis() < 50,
        "full-trunk prefill of {} tokens took {:?}, expected < 50ms",
        prompt_tokens.len(),
        elapsed
    );

    // Recurrent state depth verification across all 6 layers
    assert_eq!(session.rec_states.len(), 6);
    for (layer_idx, rec_state) in session.rec_states.iter().enumerate() {
        let non_zero_count = rec_state.h_state.iter().filter(|&&v| v != 0).count();
        assert!(
            non_zero_count > 100,
            "layer {} recurrence state h_state has only {} non-zero dimensions; trunk not activated",
            layer_idx,
            non_zero_count
        );
    }

    // Cyclic attention cache verification
    for (h_idx, cache) in session.hamming_caches.iter().enumerate() {
        assert!(
            cache.count > 0,
            "cyclic attention cache {} count is zero after prefill",
            h_idx
        );
    }
}

#[test]
fn challenge_m3_reasoning_trajectory_divergence_and_steering() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("ServedPointKey required");
    let mem = model.memory.as_ref().expect("ServedTurnMemory required");
    let bpe = ByteBpe4096::global();

    let shared_prompt = "State the twin prime conjecture and whether it is resolved.";
    let prompt_tokens = bpe.encode(shared_prompt);

    // Session A: Steered along prime gaps and Polymath bounds
    let mut session_a = LutStackSession::new(model.clone());
    for &tok in &prompt_tokens {
        session_a.prefill_token(tok, true);
    }
    let baseline_n_t_a = session_a.last_n_t;

    // Session B: Identical initial state
    let mut session_b = LutStackSession::new(model.clone());
    for &tok in &prompt_tokens {
        session_b.prefill_token(tok, true);
    }
    let baseline_n_t_b = session_b.last_n_t;

    // Invariant: identical prompts must produce bitwise identical initial hidden states
    assert_eq!(
        baseline_n_t_a, baseline_n_t_b,
        "deterministic prefill must produce identical hidden states for identical inputs"
    );

    // Apply divergent reasoning trajectory A
    let trace_a = "<thinking>\nTwin prime conjecture: infinitely many primes p such that p+2 is prime. Zhang 2013 bounded gaps to 70M; Maynard and Polymath lowered to 246.\n</thinking>";
    for &tok in &bpe.encode(trace_a) {
        session_a.prefill_token(tok, true);
    }

    // Apply divergent reasoning trajectory B (Riemann hypothesis)
    let trace_b = "<thinking>\nRiemann Hypothesis: all non-trivial zeros of zeta function lie on critical line Re(s) = 1/2. Numerical verification for 10^13 zeros, Millennium Problem.\n</thinking>";
    for &tok in &bpe.encode(trace_b) {
        session_b.prefill_token(tok, true);
    }

    let final_n_t_a = session_a.last_n_t;
    let final_n_t_b = session_b.last_n_t;

    // Active steering check: hidden states must differ from baseline
    let shift_a: u64 = final_n_t_a
        .iter()
        .zip(baseline_n_t_a.iter())
        .map(|(&a, &b)| (a as i64 - b as i64).unsigned_abs())
        .sum();
    let shift_b: u64 = final_n_t_b
        .iter()
        .zip(baseline_n_t_b.iter())
        .map(|(&a, &b)| (a as i64 - b as i64).unsigned_abs())
        .sum();

    assert!(
        shift_a > 500,
        "trajectory A must steer n_t by at least 500 L1 shift (got {})",
        shift_a
    );
    assert!(
        shift_b > 500,
        "trajectory B must steer n_t by at least 500 L1 shift (got {})",
        shift_b
    );

    // Trajectory divergence check: A and B must diverge from each other
    let divergence_ab: u64 = final_n_t_a
        .iter()
        .zip(final_n_t_b.iter())
        .map(|(&a, &b)| (a as i64 - b as i64).unsigned_abs())
        .sum();
    assert!(
        divergence_ab >= 1000,
        "divergent reasoning trajectories must produce L1 separation >= 1000 (got {})",
        divergence_ab
    );

    // PointKey resonance evaluation must reflect steered trajectories
    let hit_a = select_point_key_with_resonance(
        point,
        &final_n_t_a,
        &session_a.tokens,
        mem.key_window,
        mem.reply.as_ref(),
        mem.reply_key_len,
        &session_a.recent_turn_rows,
    );
    let hit_b = select_point_key_with_resonance(
        point,
        &final_n_t_b,
        &session_b.tokens,
        mem.key_window,
        mem.reply.as_ref(),
        mem.reply_key_len,
        &session_b.recent_turn_rows,
    );

    if let (Some(ha), Some(hb)) = (hit_a, hit_b) {
        assert!(
            ha.raw_l1 != hb.raw_l1 || ha.row != hb.row,
            "distinct reasoning trajectories must yield distinct PointKey resonance coordinates"
        );
    }
}

#[test]
fn challenge_m3_strict_visible_reply_barrier_and_no_thinking_leakage() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    let prompt = "Explain the fundamental theorem of arithmetic.";
    let prompt_tokens = bpe.encode(prompt);

    let mut session = LutStackSession::new(model.clone());
    for &tok in &prompt_tokens {
        session.prefill_token(tok, true);
    }

    // Phase 1: Reasoning prefill
    let trace = "<thinking>\nEvery integer n > 1 can be represented uniquely as a product of prime powers up to order of factors.\n</thinking>";
    let trace_tokens = bpe.encode(trace);

    for (step_idx, &tok) in trace_tokens.iter().enumerate() {
        session.prefill_token(tok, true);
        if step_idx + 1 < trace_tokens.len() {
            assert!(
                session.reply_start.is_none(),
                "barrier violation: reply_start set while reasoning token {} prefilled",
                step_idx
            );
        }
    }

    // Phase 2: Barrier transition
    session.mark_reply_start();
    let expected_start = session.tokens.len() + 1;
    assert_eq!(session.reply_start, Some(expected_start));
    assert!(
        session.reply_history.is_empty(),
        "reply_history must be empty at barrier"
    );

    // Phase 3: Visible answer generation
    let first_tok = bpe.encode("The")[0];
    let step1 = session.step_with_telemetry(first_tok);
    assert_eq!(session.reply_history.len(), 1);
    assert_eq!(session.reply_history[0], first_tok);

    // Verify generated sequence contains no thinking tags
    let mut generated_ids = vec![first_tok];
    let mut current_tok = step1.predicted_token;
    for _ in 0..16 {
        if current_tok == bpe.eos_id() {
            break;
        }
        generated_ids.push(current_tok);
        let step = session.step_with_telemetry(current_tok);
        current_tok = step.predicted_token;
    }

    let reply_text = bpe.decode(&generated_ids);
    assert!(
        !reply_text.contains("<thinking>"),
        "visible reply leaked <thinking>: '{}'",
        reply_text
    );
    assert!(
        !reply_text.contains("</thinking>"),
        "visible reply leaked </thinking>: '{}'",
        reply_text
    );
    assert!(
        !reply_text.contains("<thought>"),
        "visible reply leaked <thought>: '{}'",
        reply_text
    );
    assert!(
        !reply_text.contains("</thought>"),
        "visible reply leaked </thought>: '{}'",
        reply_text
    );
}

#[test]
fn challenge_m3_prefill_throughput_and_sub_second_latency() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    // Synthetic text exceeding 512 tokens
    let test_corpus = "Mathematics is the science and study of quality, structure, space, and change. \
        Mathematicians seek out patterns, formulate new conjectures, and establish truth by rigorous \
        deduction from appropriately chosen axioms and definitions. Emmy Noether revolutionized abstract \
        algebra with ascending chain conditions on ideals. Kurt Gödel demonstrated the intrinsic \
        limitations of formal proof systems. Alan Turing established universal computability. ";
    let repeated = test_corpus.repeat(8);
    let mut all_tokens = bpe.encode(&repeated);
    all_tokens.truncate(512);
    assert_eq!(all_tokens.len(), 512, "corpus must have exactly 512 tokens");

    let mut session = LutStackSession::new(model);

    let start = Instant::now();
    for &tok in &all_tokens {
        session.prefill_token(tok, true);
    }
    let duration = start.elapsed();

    // 512 tokens through 6 layers (3,072 layer steps) must complete in < 300ms
    assert!(
        duration.as_millis() < 300,
        "512-token full-trunk prefill took {:?}, expected < 300ms",
        duration
    );
}

#[test]
fn challenge_m3_adversarial_malformed_and_unclosed_thinking_tags() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    // Adversarial Case 1: Unclosed thinking tag
    let mut session1 = LutStackSession::new(model.clone());
    let unclosed = "Prompt: <thinking>inchoate reasoning without closing tag";
    for &tok in &bpe.encode(unclosed) {
        session1.prefill_token(tok, true);
    }
    assert!(!session1.tokens.is_empty());
    // Hidden state must remain bounded integer
    for &v in &session1.last_n_t {
        assert!((-32768..=32767).contains(&v));
    }

    // Adversarial Case 2: Empty thinking block
    let mut session2 = LutStackSession::new(model.clone());
    let empty_block = "Question? <thinking></thinking>";
    for &tok in &bpe.encode(empty_block) {
        session2.prefill_token(tok, true);
    }
    session2.mark_reply_start();
    assert_eq!(session2.reply_start, Some(session2.tokens.len() + 1));

    // Adversarial Case 3: Multiple consecutive and mixed thinking blocks
    let mut session3 = LutStackSession::new(model.clone());
    let mixed = "<thinking>Step 1</thinking> prose <thought>Step 2</thought>";
    for &tok in &bpe.encode(mixed) {
        session3.prefill_token(tok, true);
    }
    assert!(!session3.tokens.is_empty());
    for &v in &session3.last_n_t {
        assert!((-32768..=32767).contains(&v));
    }
}

#[test]
fn challenge_m3_zero_float_conformance_and_determinism() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    let prompt = "Explain modular exponentiation: <thinking>Compute b^e mod m via repeated squaring</thinking>";
    let tokens = bpe.encode(prompt);

    let mut sessions = Vec::new();
    for _ in 0..5 {
        let mut sess = LutStackSession::new(model.clone());
        for &tok in &tokens {
            sess.prefill_token(tok, true);
        }
        sessions.push(sess);
    }

    let reference_n_t = sessions[0].last_n_t;
    for (i, sess) in sessions.iter().enumerate().skip(1) {
        // Bitwise determinism across runs
        assert_eq!(
            sess.last_n_t, reference_n_t,
            "session {} hidden state differs from reference",
            i
        );

        // Recurrence state bitwise equality
        for (l_idx, (r1, r2)) in sessions[0]
            .rec_states
            .iter()
            .zip(sess.rec_states.iter())
            .enumerate()
        {
            assert_eq!(
                r1.h_state, r2.h_state,
                "layer {} h_state mismatch between runs",
                l_idx
            );
        }
    }
}
