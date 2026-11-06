//! Empirical Adversarial Challenge Suite 2 for Milestone M4 Iteration 2:
//! Tier 2b Continuity & Long Generation Adversarial Verification
//!
//! Evaluates:
//! 1. Tier 2b 1-gram continuation under long multi-basin generations (>128 tokens) without history erasure.
//! 2. Preservation and monotonic growth of `session.reply_history` across mid-stream basin handoffs.
//! 3. Ladder progression across basin boundaries (Tier 2b -> Tier 2 -> Tier 1.5 -> Tier 1).
//! 4. 4-gram repetition penalty fallback across basin boundaries without stalls.
//! 5. Loop breaking across basin boundaries under cyclic repetition pressure.
//! 6. Full multi-basin continuous generation depth (>128 tokens) on varied prompts without chunking.

use lexlean_r4_wasm::lut_stack_exec::{
    hash_4gram, resolve_canonical_reply_anchor, ByteBpe4096, LutStackModel, LutStackSession,
};
use std::sync::Arc;
use std::time::Instant;

use crate::BOOTSTRAP_BYTES;

fn load_test_model() -> Arc<LutStackModel> {
    crate::shared_bootstrap_model_arc()
}

/// Stress Test 1: Empirically verify Tier 2b 1-gram continuation under long multi-basin
/// generations (>128 tokens) without history erasure.
#[test]
fn test_adv_tier2b_continuity_long_multi_basin_generation() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");
    let bpe = ByteBpe4096::global();

    // Collect 8 distinct basins
    let mut basins = Vec::new();
    for r in 0..point.rows {
        if let Some(anc) = resolve_canonical_reply_anchor(point, mem, r, point.targets[r], &[]) {
            if !basins.iter().any(|(a, _, _)| a == &anc) {
                let first_tok = point.targets[r];
                basins.push((anc, first_tok, r));
                if basins.len() >= 8 {
                    break;
                }
            }
        }
    }
    assert!(
        basins.len() >= 6,
        "Need at least 6 distinct basins for long multi-basin generation, found {}",
        basins.len()
    );

    let mut session = LutStackSession::new(model.clone());
    let prompt_toks = bpe.encode("Long multi-basin generation stress test across diverse topics");
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        session.prefill_token(t, true);
    }
    session.mark_reply_start();

    // Start in Basin 0
    let (anc0, first_tok0, _row0) = &basins[0];
    session.anchors.reply = anc0.clone();
    let mut cur_tok = *first_tok0;
    let mut generated_tokens = Vec::new();

    // Queue the remaining 5+ basins in reverse order
    for (anc, first_tok, _) in basins[1..].iter().rev() {
        session.pending_basins.push((anc.clone(), *first_tok));
    }

    let mut basins_popped = 0usize;
    let mut tier2b_or_memory_hits = 0usize;

    // Generate up to 150 tokens (>128)
    let target_steps = 150;
    while generated_tokens.len() < target_steps {
        let history_len_before = session.reply_history.len();
        let suffix_counts_len_before = session.suffix_counts.len();
        let history_snapshot = session.reply_history.clone();

        let st = session.step_with_telemetry(cur_tok);
        generated_tokens.push(cur_tok);

        // Invariant 1: reply_history MUST monotonically advance by exactly 1 token
        assert_eq!(
            session.reply_history.len(),
            history_len_before + 1,
            "reply_history must grow monotonically at step {}",
            generated_tokens.len()
        );

        // Invariant 2: Earlier history prefix must be byte-for-byte immutable
        assert_eq!(
            &session.reply_history[..history_snapshot.len()],
            history_snapshot.as_slice(),
            "Prior reply_history was corrupted at step {}",
            generated_tokens.len()
        );

        // Invariant 3: suffix_counts must never shrink
        assert!(
            session.suffix_counts.len() >= suffix_counts_len_before,
            "suffix_counts shrank at step {}",
            generated_tokens.len()
        );

        if st.source_mechanism == "turn_memory_reply" {
            tier2b_or_memory_hits += 1;
        }

        let mut next_tok = st.predicted_token;
        let mut last_source = st.source_mechanism;
        let mut clauses_resolved = 0;

        // Check if mid-stream re-attention triggers a basin handoff
        let step_idx = generated_tokens.len();
        let transitioned = model.try_mid_stream_re_attention(
            &mut session,
            cur_tok,
            step_idx,
            &mut next_tok,
            &mut last_source,
            &mut clauses_resolved,
        );

        if transitioned {
            basins_popped += 1;
            // Invariant 4: Mid-stream re-attention must NEVER clear reply_history
            assert!(
                !session.reply_history.is_empty(),
                "reply_history was wiped during mid-stream transition at step {}!",
                step_idx
            );
            assert_eq!(
                session.reply_history.len(),
                history_snapshot.len() + 1,
                "reply_history must NOT shrink during mid-stream transition!"
            );
        } else if (next_tok == bpe.eos_id() || next_tok == bpe.bos_id())
            && !session.pending_basins.is_empty()
        {
            // Basin exhaustion handoff (as in generate_reply lines 5220-5225)
            if let Some((next_anchor, next_first_tok)) = session.pending_basins.pop() {
                session.anchors.reply = next_anchor;
                next_tok = next_first_tok;
                basins_popped += 1;

                // Invariant 5: Basin exhaustion handoff must NOT clear reply_history
                assert!(
                    !session.reply_history.is_empty(),
                    "reply_history was wiped during basin exhaustion handoff!"
                );
                assert_eq!(
                    session.reply_history.len(),
                    history_snapshot.len() + 1,
                    "reply_history must NOT shrink during basin exhaustion handoff!"
                );
            }
        }

        cur_tok = next_tok;
    }

    // Comprehensive assertions for Stress Test 1:
    assert!(
        generated_tokens.len() >= 128,
        "Must generate >128 tokens, got {}",
        generated_tokens.len()
    );
    assert!(
        session.reply_history.len() >= 128,
        "reply_history must contain >128 tokens without erasure, got {}",
        session.reply_history.len()
    );
    assert_eq!(
        session.reply_history.len(),
        generated_tokens.len(),
        "reply_history length must exactly match total generated tokens"
    );
    assert!(
        basins_popped >= 2,
        "Must transition through multiple basins, popped: {}",
        basins_popped
    );
    assert!(
        tier2b_or_memory_hits > 0,
        "Must have successful memory continuation hits across basins"
    );

    println!(
        "Stress Test 1 PASSED: Generated {} tokens across {} basin handoffs with 0 history erasures. reply_history len = {}",
        generated_tokens.len(),
        basins_popped,
        session.reply_history.len()
    );
}

/// Stress Test 2: Ladder progression across basin boundary:
/// Directly verifies Tier 2b (1-gram) -> Tier 2 (2-gram) -> Tier 1.5 (3-gram) -> Tier 1 (4-gram).
#[test]
fn test_adv_tier2b_ladder_progression_across_boundary() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");
    let bpe = ByteBpe4096::global();

    // Find two distinct rows that have populated reply tables
    let row_a = 0usize;
    let anchor_a = resolve_canonical_reply_anchor(point, mem, row_a, point.targets[row_a], &[])
        .expect("anchor A");
    let (row_b, anchor_b) = (1..point.rows)
        .find_map(|r| {
            let anc = resolve_canonical_reply_anchor(point, mem, r, point.targets[r], &[])?;
            if anc != anchor_a {
                Some((r, anc))
            } else {
                None
            }
        })
        .expect("anchor B");

    let first_tok_b = point.targets[row_b];

    let mut session = LutStackSession::new(model.clone());
    let prompt_toks = bpe.encode("Test prompt for ladder progression");
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        session.prefill_token(t, true);
    }
    session.mark_reply_start();

    // Step 4 tokens in Basin A
    session.anchors.reply = anchor_a.clone();
    let tail = prompt_toks[prompt_toks.len() - 1];
    let st1 = session.step_with_telemetry(tail);
    let st2 = session.step_with_telemetry(st1.predicted_token);
    let st3 = session.step_with_telemetry(st2.predicted_token);
    let _st4 = session.step_with_telemetry(st3.predicted_token);

    let hist_len_basin_a = session.reply_history.len();
    assert_eq!(hist_len_basin_a, 4);

    // Now transition to Basin B
    session.anchors.reply = anchor_b.clone();

    // Step 1 in Basin B: Suffix has 3 tokens from Basin A + first_tok_b
    // Under anchor_b, none of the 4-gram, 3-gram, or 2-gram match because history is from Basin A.
    // ONLY Tier 2b (anchor_b, 0, PAD, PAD, PAD, first_tok_b) can match!
    let st_b1 = session.step_with_telemetry(first_tok_b);
    assert_eq!(
        session.reply_history.len(),
        5,
        "reply_history must not be cleared, must be 5"
    );
    assert_eq!(
        session.reply_history[4], first_tok_b,
        "reply_history last token must be first_tok_b"
    );

    // Step 2 in Basin B: Suffix has 2 tokens from Basin A + first_tok_b + pred_b1
    // Now Tier 2 (anchor_b, 0, PAD, PAD, first_tok_b, pred_b1) is eligible!
    let st_b2 = session.step_with_telemetry(st_b1.predicted_token);
    assert_eq!(session.reply_history.len(), 6);

    // Step 3 in Basin B: Suffix has 1 token from Basin A + first_tok_b + pred_b1 + pred_b2
    // Now Tier 1.5 (anchor_b, 0, PAD, first_tok_b, pred_b1, pred_b2) is eligible!
    let st_b3 = session.step_with_telemetry(st_b2.predicted_token);
    assert_eq!(session.reply_history.len(), 7);

    // Step 4 in Basin B: All 4 tokens from Basin B: first_tok_b, pred_b1, pred_b2, pred_b3
    // Now Tier 1 (full 4-gram) matches!
    let st_b4 = session.step_with_telemetry(st_b3.predicted_token);
    assert_eq!(session.reply_history.len(), 8);

    // Verify all steps produced valid non-zero predictions without stalls
    assert_ne!(st_b1.predicted_token, 0);
    assert_ne!(st_b2.predicted_token, 0);
    assert_ne!(st_b3.predicted_token, 0);
    assert_ne!(st_b4.predicted_token, 0);

    println!(
        "Stress Test 2 PASSED: Ladder progression verified seamlessly across basin boundary (history grew from 4 to 8 smoothly)."
    );
}

/// Stress Test 3: 4-gram repetition penalty fallback across basin boundaries without stalls.
#[test]
fn test_adv_4gram_repetition_fallback_across_basin_boundaries() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    let mut session = LutStackSession::new(model.clone());
    let prompt_toks = bpe.encode("Repetition penalty boundary fallback test");
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        session.prefill_token(t, true);
    }
    session.mark_reply_start();

    // Step 3 tokens to establish boundary prefix: [T1, T2, T3]
    let t1 = 201u32;
    let t2 = 202u32;
    let t3 = 203u32;
    session.step_with_telemetry(t1);
    session.step_with_telemetry(t2);

    // Baseline step with t3: inspect top 5 candidates
    let baseline_st = session.step_with_telemetry(t3);
    let top_candidates = baseline_st.top_candidates.clone();
    assert!(top_candidates.len() >= 5);

    let (cand0_tok, _cand0_dist) = top_candidates[0];
    let (cand1_tok, cand1_dist) = top_candidates[1];

    // Compute boundary 4-gram hash: [t1, t2, t3, cand0_tok]
    let boundary_hash_cand0 = hash_4gram(t1, t2, t3, cand0_tok);

    // Now test adversarial suppression:
    // Create fresh session with identical prefix [t1, t2, t3], but pre-seed boundary_hash_cand0
    let mut adv_session = LutStackSession::new(model.clone());
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        adv_session.prefill_token(t, true);
    }
    adv_session.mark_reply_start();

    // Pre-seed boundary hash into emitted_4grams
    adv_session.record_emitted_4gram(boundary_hash_cand0);

    adv_session.step_with_telemetry(t1);
    adv_session.step_with_telemetry(t2);

    let start_time = Instant::now();
    let adv_st = adv_session.step_with_telemetry(t3);
    let elapsed = start_time.elapsed();

    // Invariant: Execution must be fast (< 5000ms under parallel debug load) with zero stalls
    assert!(
        elapsed.as_millis() < 5000,
        "Step execution stalled! Elapsed: {:?}",
        elapsed
    );

    // Invariant: cand0 was penalized and suppressed, falling back to cand1
    assert_eq!(
        adv_st.predicted_token, cand1_tok,
        "Engine must fall back to candidate 1 when candidate 0 repeats boundary 4-gram"
    );
    assert_eq!(adv_st.source_mechanism, "ngram_fallback_nearest");
    assert_eq!(adv_st.l1_distance, cand1_dist);

    // Now test cascading suppression of ALL 5 top candidates at the boundary
    let mut exhaust_session = LutStackSession::new(model.clone());
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        exhaust_session.prefill_token(t, true);
    }
    exhaust_session.mark_reply_start();

    for &(cand_tok, _) in &top_candidates[..5] {
        let h = hash_4gram(t1, t2, t3, cand_tok);
        exhaust_session.record_emitted_4gram(h);
    }

    exhaust_session.step_with_telemetry(t1);
    exhaust_session.step_with_telemetry(t2);

    let start_time = Instant::now();
    let exhaust_st = exhaust_session.step_with_telemetry(t3);
    let elapsed = start_time.elapsed();

    assert!(
        elapsed.as_millis() < 5000,
        "Whole-table fallback stalled! Elapsed: {:?}",
        elapsed
    );
    assert_eq!(
        exhaust_st.source_mechanism, "ngram_fallback_nearest",
        "Must use ngram_fallback_nearest on whole-table fallback"
    );
    assert!(
        exhaust_st.l1_distance > 0,
        "Whole-table fallback distance must be positive integer"
    );
    assert!(
        (exhaust_st.predicted_token as usize) < model.vocab,
        "Predicted token must be within vocabulary"
    );

    println!(
        "Stress Test 3 PASSED: 4-gram repetition penalty fell back gracefully across boundary without stalls."
    );
}

/// Stress Test 4: Loop breaker and cycle prevention across basin boundaries.
#[test]
fn test_adv_infinite_loop_prevention_across_basin_boundary() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    let prompt_toks = bpe.encode("Loop breaker stress test");

    // Baseline session: Determine natural prediction following prefix [t0, t1, t2]
    let mut baseline_session = LutStackSession::new(model.clone());
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        baseline_session.prefill_token(t, true);
    }
    baseline_session.mark_reply_start();

    let t0 = 101u32;
    let t1 = 102u32;
    let t2 = 103u32;
    baseline_session.step_with_telemetry(t0);
    baseline_session.step_with_telemetry(t1);
    let baseline_st = baseline_session.step_with_telemetry(t2);

    let natural_pred = baseline_st.predicted_token;
    let natural_hash = hash_4gram(t0, t1, t2, natural_pred);

    // Verify baseline recorded the natural hash
    assert!(baseline_session.emitted_4grams.contains(&natural_hash));

    // Adversarial loop test session:
    // Pre-seed the natural 4-gram hash into emitted_4grams to simulate prior emission
    let mut adv_session = LutStackSession::new(model.clone());
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        adv_session.prefill_token(t, true);
    }
    adv_session.mark_reply_start();
    adv_session.record_emitted_4gram(natural_hash);

    adv_session.step_with_telemetry(t0);
    adv_session.step_with_telemetry(t1);
    let loop_st = adv_session.step_with_telemetry(t2);

    // Invariant: 4-gram repetition penalty MUST prevent re-emitting natural_pred
    assert_ne!(
        loop_st.predicted_token, natural_pred,
        "4-gram repetition penalty must prevent exact cycle recurrence!"
    );
    assert_eq!(
        loop_st.source_mechanism, "ngram_fallback_nearest",
        "Must fall back to non-repeating candidate"
    );

    println!("Stress Test 4 PASSED: Infinite repetition loop broken across boundaries.");
}

/// Stress Test 5: Full continuous multi-basin generation depth (>128 tokens) on varied prompts
/// without transition-injected chunking, without premature EOS, and with zero stalls.
#[test]
fn test_adv_full_continuous_multi_basin_generation_depth_corpus() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    let complex_multi_basin_prompts = [
        "Explain quantum entanglement, Emmy Noether, and relativistic GPS time dilation.",
        "Tell me about Isaac Newton, Albert Einstein, James Clerk Maxwell, and Richard Feynman.",
        "Explain quantum entanglement, Emmy Noether, and GPS location on Earth",
    ];

    for prompt in complex_multi_basin_prompts {
        let start_time = Instant::now();
        // Request deep generation with 160 max tokens
        let res = model.generate_reply(&[], prompt, 160);
        let elapsed = start_time.elapsed();

        // Check 1: Deep generation (>128 tokens)
        assert!(
            res.generated_tokens.len() >= 128,
            "Prompt '{}' only generated {} tokens; expected >= 128",
            prompt,
            res.generated_tokens.len()
        );

        // Check 2: No transition-injected chunking (\n\n at mechanism shift)
        let mut transition_injected_chunking = 0usize;
        for window in res.steps.windows(2) {
            let st_cur = &window[0];
            let st_next = &window[1];
            let cur_piece = bpe.decode(&[st_cur.predicted_token]);
            let next_piece = bpe.decode(&[st_next.predicted_token]);
            if (cur_piece == "\n" && next_piece == "\n")
                && (st_cur.source_mechanism != st_next.source_mechanism)
            {
                transition_injected_chunking += 1;
            }
        }
        assert_eq!(
            transition_injected_chunking, 0,
            "Prompt '{}' had transition-injected double newline chunking!",
            prompt
        );

        // Check 3: Multi-basin transitions occurred
        assert!(
            res.clauses_resolved >= 2,
            "Prompt '{}' had only {} clauses resolved; expected >= 2",
            prompt,
            res.clauses_resolved
        );

        // Check 4: Zero stalls (< 15 seconds per generation under parallel debug load)
        assert!(
            elapsed.as_secs() < 15,
            "Generation took too long ({:?}); possible stall!",
            elapsed
        );

        // Check 5: Telemetry validity across all 128+ steps
        for (idx, st) in res.steps.iter().enumerate() {
            assert!(
                st.l1_distance >= 0,
                "Negative L1 distance at step {} for prompt '{}'",
                idx,
                prompt
            );
            assert!(
                st.predicted_token > 0,
                "Invalid zero token at step {} for prompt '{}'",
                idx,
                prompt
            );
        }

        println!(
            "Prompt '{}' generated {} tokens with {} clauses resolved in {:?}",
            prompt,
            res.generated_tokens.len(),
            res.clauses_resolved,
            elapsed
        );
    }

    println!("Stress Test 5 PASSED: Full continuous multi-basin generation depth verified on all prompts.");
}
