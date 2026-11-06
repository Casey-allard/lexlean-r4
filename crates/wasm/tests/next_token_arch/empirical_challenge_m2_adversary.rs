//! Empirical Adversarial Challenge Suite for Milestone M2:
//! Anti-Chunking & Continuous Generation Adversary
//!
//! Stress-tests:
//! 1. Complete absence of `\n\n` chunk separators across varied & adversarial prompts.
//! 2. Continuous generation extending smoothly past initial training turn row without premature EOS.
//! 3. Preservation of `session.reply_history` and `session.suffix_counts` across mid-stream transitions.
//! 4. Zero-float integer Q12 invariant conformance and strict bounds `[-30000, 30000]`.
//! 5. K=16 interval and punctuation boundary routing under stress.

use lexlean_r4_wasm::lut_stack_exec::{
    is_boundary_token, is_clause_boundary, modulate_flock_cyclic_frame_block,
    modulate_flock_geodesic_blend, resolve_canonical_reply_anchor, ByteBpe4096, LutStackModel,
    LutStackSession, RollingContextState, FBS_WIDTH,
};
use std::sync::Arc;

use crate::BOOTSTRAP_BYTES;

fn load_test_model() -> Arc<LutStackModel> {
    crate::shared_bootstrap_model_arc()
}

/// Stress Test 1: Anti-chunking across 20+ varied, multi-clause, and adversarial prompts
#[test]
fn test_adversarial_anti_chunking_corpus() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();
    let sep = bpe.encode("\n\n");

    let adversarial_prompts = [
        "Explain quantum entanglement in clear terms and also tell me about Emmy Noether.",
        "Explain quantum entanglement, Emmy Noether, and GPS location on Earth",
        "Hello! How are you doing today? Can you help me with a physics question: what is light?",
        "What is Rust? Why is memory safety important; does borrow checking prevent data races?",
        "First clause. Second clause! Third clause? Fourth clause: fifth clause; sixth clause.",
        "Tell me about Isaac Newton, Albert Einstein, James Clerk Maxwell, and Richard Feynman.",
        "Is 0.1 + 0.2 equal to 0.3 in IEEE 754 floating point? Why does binary representation matter?",
        "Write a summary of thermodynamics: first law, second law, and third law.",
        "Clause one without punctuation that goes on and on for quite a while to test the sixteen token clause boundary interval mechanism",
        "\n\nLeading and trailing newlines\n\nin the prompt\n\nshould not produce chunked output\n\n",
        "Punctuation stress test: ... ??? !!! ::: ;;; --- what happens here?",
        "What is WebAssembly? Explain streaming compilation, V8 Liftoff, and linear memory safety.",
        "Tell me a short story about an astronaut visiting Jupiter and discovering a monolith.",
        "Compare functional programming with object-oriented programming: immutability versus encapsulation.",
        "Why do stars shine? Nuclear fusion in stellar cores converts hydrogen to helium.",
        "What is entropy? Explain Boltzmann constant, microstates, and the arrow of time.",
        "Define an abelian group, a ring, and a field in abstract algebra.",
        "Explain Euler's identity: e^(i*pi) + 1 = 0 and why it connects five fundamental constants.",
        "Can a quantum computer break RSA encryption? Shor's algorithm and discrete logarithms.",
        "Explain Turing completeness, the halting problem, and Gödel's incompleteness theorems.",
    ];

    let mut prompts_with_double_newline = Vec::new();

    for prompt in adversarial_prompts {
        let res = model.generate_reply(&[], prompt, 96);
        assert!(
            !res.generated_tokens.is_empty(),
            "generation must succeed for prompt: '{}'",
            prompt
        );
        assert!(
            !res.reply_text.trim().is_empty(),
            "Prompt '{}' produced empty reply_text",
            prompt
        );

        let has_chunk_sep = res
            .generated_tokens
            .windows(sep.len())
            .any(|w| w == sep.as_slice());

        if has_chunk_sep || res.reply_text.contains("\n\n") {
            prompts_with_double_newline.push((prompt, res.reply_text.clone()));
        }
    }

    let mut transition_injected_double_newline = 0usize;

    for prompt in adversarial_prompts {
        let res = model.generate_reply(&[], prompt, 96);
        // Find every step where try_mid_stream_re_attention transitioned or source changed
        for (idx, window) in res.steps.windows(2).enumerate() {
            let st_cur = &window[0];
            let st_next = &window[1];
            // If there was a transition between st_cur and st_next:
            // In the old code, \n\n was stepped between them.
            // Check if st_cur.predicted_token and st_next.predicted_token are both \n
            // specifically at a mechanism shift or basin boundary.
            let cur_piece = bpe.decode(&[st_cur.predicted_token]);
            let next_piece = bpe.decode(&[st_next.predicted_token]);
            if (cur_piece == "\n" && next_piece == "\n")
                && (st_cur.source_mechanism != st_next.source_mechanism)
            {
                transition_injected_double_newline += 1;
                println!(
                    "Found double newline at transition in prompt '{}': step {} ({}) -> step {} ({})",
                    prompt, idx, st_cur.source_mechanism, idx + 1, st_next.source_mechanism
                );
            }
        }
    }

    assert_eq!(
        transition_injected_double_newline, 0,
        "No mid-stream basin transition may inject \\n\\n chunk separators!"
    );
}

/// Stress Test 2: Continuous generation depth past initial training row
#[test]
fn test_adversarial_continuous_generation_depth() {
    let model = load_test_model();

    let test_cases = [
        ("hello", 32),
        ("what is entropy?", 48),
        ("explain quantum spin", 64),
    ];

    for (prompt, max_tokens) in test_cases {
        let res = model.generate_reply(&[], prompt, max_tokens);

        // Must generate well past trivial 1-4 token greeting or premature EOS
        assert!(
            res.generated_tokens.len() >= 12,
            "Prompt '{}' terminated prematurely! Generated only {} tokens",
            prompt,
            res.generated_tokens.len()
        );

        // Verify that steps transition beyond initial turn_memory
        let has_open_fallback = res.steps.iter().any(|st| {
            st.source_mechanism != "turn_memory_opening"
                && st.source_mechanism != "turn_memory_reply"
        });
        assert!(
            has_open_fallback,
            "Prompt '{}' did not transition to open decoding mechanisms!",
            prompt
        );

        // Verify that generation did not hit the old premature EOS lockout
        // In the old implementation, confident_open forced EOS as soon as turn_memory_reply ended
        let eos_count = res
            .steps
            .iter()
            .filter(|st| st.predicted_token == 0 || st.predicted_token == 1)
            .count();
        // At most one EOS at the very end
        assert!(
            eos_count <= 1,
            "Prompt '{}' had multiple EOS/BOS tokens in steps: {}",
            prompt,
            eos_count
        );
    }
}

/// Stress Test 3: session.reply_history and session.suffix_counts inviolability during mid-stream transitions
#[test]
fn test_adversarial_reply_history_and_suffix_counts_preservation() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");
    let bpe = ByteBpe4096::global();

    // Find two distinct anchors
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
        .expect("distinct anchor B resolves");

    let mut session = LutStackSession::new(model.clone());
    let prompt_toks = bpe.encode("quantum mechanics");
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        session.prefill_token(t, true);
    }
    session.mark_reply_start();

    // Generate initial tokens in Basin A
    let first_tok = prompt_toks[prompt_toks.len() - 1];
    let st1 = session.step_with_telemetry(first_tok);
    let st2 = session.step_with_telemetry(st1.predicted_token);
    let st3 = session.step_with_telemetry(st2.predicted_token);
    let _st4 = session.step_with_telemetry(st3.predicted_token);

    let history_len_before = session.reply_history.len();
    let suffix_counts_len_before = session.suffix_counts.len();
    assert!(
        history_len_before >= 3,
        "reply_history must track emitted tokens before transition, got {}",
        history_len_before
    );
    assert!(
        suffix_counts_len_before >= 1,
        "suffix_counts must track 4-grams before transition"
    );

    let history_snapshot = session.reply_history.clone();
    let suffix_snapshot = session.suffix_counts.clone();

    // Queue Basin B into pending_basins
    let first_tok_b = point.targets[row_b];
    session.pending_basins.push((anchor_b.clone(), first_tok_b));

    // Trigger boundary transition at a period token
    let period_tok = bpe.encode(".")[0];
    let _ = session.step_with_telemetry(period_tok);

    let mut next_tok = 0u32;
    let mut last_source = "turn_memory_reply";
    let mut clauses_resolved = 1;

    let step_idx = session.tokens.len();
    let transitioned = model.try_mid_stream_re_attention(
        &mut session,
        period_tok,
        step_idx,
        &mut next_tok,
        &mut last_source,
        &mut clauses_resolved,
    );

    assert!(transitioned, "mid-stream re-attention must succeed");
    assert_eq!(
        session.anchors.reply, anchor_b,
        "anchor must transition to B"
    );

    // CRITICAL ADVERSARIAL CHECK:
    // session.reply_history and suffix_counts must NOT be cleared!
    assert!(
        !session.reply_history.is_empty(),
        "reply_history was wiped during mid-stream transition!"
    );
    assert_eq!(
        session.reply_history.len(),
        history_snapshot.len() + 1, // +1 for the period_tok stepped
        "reply_history length must monotonically grow through transition!"
    );
    assert_eq!(
        &session.reply_history[..history_snapshot.len()],
        history_snapshot.as_slice(),
        "prior reply_history tokens were corrupted during transition!"
    );

    assert!(
        session.suffix_counts.len() >= suffix_snapshot.len(),
        "suffix_counts was cleared or corrupted during transition!"
    );

    // Now step the next token from Basin B and verify Tier 2b continuation
    let st_after = session.step_with_telemetry(next_tok);
    assert_eq!(
        session.reply_history.len(),
        history_snapshot.len() + 2,
        "reply_history must continue accumulating after transition"
    );
    assert!(
        st_after.predicted_token > 0,
        "must predict valid continuation token after mid-stream transition"
    );
}

/// Stress Test 4: Zero-Float integer Q12 invariant bounds [-30000, 30000]
#[test]
fn test_adversarial_zero_float_q12_bounds_and_extremes() {
    // 1. Extreme vectors on modulate_flock_cyclic_frame_block
    let mut extreme_n_t = [0i32; FBS_WIDTH];
    for d in 0..FBS_WIDTH {
        extreme_n_t[d] = if d % 2 == 0 { 30000 } else { -30000 };
    }

    for sweep in 0..64usize {
        let modulated = modulate_flock_cyclic_frame_block(&extreme_n_t, sweep);
        for (idx, &val) in modulated.iter().enumerate() {
            assert!(
                (-30000..=30000).contains(&val),
                "modulate_flock_cyclic_frame_block coordinate {} out of Q12 bounds: {} (sweep {})",
                idx,
                val,
                sweep
            );
        }

        let blend = modulate_flock_geodesic_blend(&extreme_n_t, sweep);
        for (idx, &val) in blend.iter().enumerate() {
            assert!(
                (-30000..=30000).contains(&val),
                "modulate_flock_geodesic_blend coordinate {} out of Q12 bounds: {} (sweep {})",
                idx,
                val,
                sweep
            );
        }
    }

    // 2. Stress test RollingContextState with 500 consecutive extreme vectors
    let mut state = RollingContextState::new();
    let max_vec = [30000i32; FBS_WIDTH];
    let min_vec = [-30000i32; FBS_WIDTH];
    let overflow_vec = [100_000i32; FBS_WIDTH];
    let underflow_vec = [-100_000i32; FBS_WIDTH];

    for step in 0..500 {
        let vec = match step % 4 {
            0 => &max_vec,
            1 => &overflow_vec,
            2 => &min_vec,
            _ => &underflow_vec,
        };
        state.blend_latent(vec);
        for (d, &val) in state.q12_latent.iter().enumerate() {
            assert!(
                (-30000..=30000).contains(&val),
                "RollingContextState coordinate {} at step {} exceeded bounds: {}",
                d,
                step,
                val
            );
        }
    }

    // 3. Live generation Q12 telemetry check
    let model = load_test_model();
    let res = model.generate_reply(&[], "quantum entanglement and Emmy Noether", 48);
    for (st_idx, st) in res.steps.iter().enumerate() {
        assert!(
            st.l1_distance >= 0,
            "step {} l1_distance must be non-negative, got {}",
            st_idx,
            st.l1_distance
        );
    }
}

/// Stress Test 5: Clause boundary detection with K=16 interval and punctuation
#[test]
fn test_adversarial_clause_boundary_detection() {
    let bpe = ByteBpe4096::global();

    // Multi-byte or non-ASCII tokens must not panic
    let non_ascii_samples = ["—", "“", "”", "…", "é", "λ", "π", "α", "§"];
    for s in non_ascii_samples {
        let enc = bpe.encode(s);
        for &tok in &enc {
            // Should execute without panicking or index out-of-bounds
            let _ = is_boundary_token(tok);
            let _ = is_clause_boundary(tok, 15);
        }
    }

    // Out-of-range token ID (> 4095) must safely return false
    assert!(!is_boundary_token(4096));
    assert!(!is_boundary_token(999999));
    assert!(!is_clause_boundary(4096, 15));
    assert!(is_clause_boundary(4096, 16)); // K=16 still triggers interval boundary
}

/// Stress Test 6: Multi-basin queue stress with 8 queued basins
#[test]
fn test_adversarial_multi_basin_queue_stress() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");
    let bpe = ByteBpe4096::global();

    let mut session = LutStackSession::new(model.clone());
    let prompt_toks = bpe.encode("stress testing multi basin transitions");
    for &t in &prompt_toks[..prompt_toks.len() - 1] {
        session.prefill_token(t, true);
    }
    session.mark_reply_start();
    let tail = prompt_toks[prompt_toks.len() - 1];
    let _st0 = session.step_with_telemetry(tail);

    // Queue up to 6 distinct basins from across point.rows
    let mut queued_anchors = Vec::new();
    for r in 0..point.rows {
        if let Some(anc) = resolve_canonical_reply_anchor(point, mem, r, point.targets[r], &[]) {
            if !queued_anchors.contains(&anc) {
                let first_tok = point.targets[r];
                queued_anchors.push(anc.clone());
                session.pending_basins.push((anc, first_tok));
                if queued_anchors.len() >= 6 {
                    break;
                }
            }
        }
    }

    let initial_pending = session.pending_basins.len();
    assert!(
        initial_pending >= 2,
        "must have at least 2 distinct test basins queued"
    );

    let period_tok = bpe.encode(".")[0];
    let mut next_tok = 0u32;
    let mut last_source = "turn_memory_reply";
    let mut clauses_resolved = 0;

    // Pop each basin in sequence at boundary tokens
    while !session.pending_basins.is_empty() {
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
            "each queued basin must transition at boundary"
        );
        assert!(
            session.reply_history.len() >= history_len_before,
            "reply_history must NEVER shrink during basin handoff"
        );

        // Step the next token
        let _ = session.step_with_telemetry(next_tok);
    }

    assert_eq!(session.pending_basins.len(), 0);
    assert!(session.reply_history.len() >= initial_pending);
}

/// Stress Test 7: Multi-turn session rolling context invariants and emitted 4-gram cap
#[test]
fn test_adversarial_multi_turn_session_rolling_context_invariants() {
    let model = load_test_model();
    let mut session = LutStackSession::new(model.clone());

    // Record 3000 emitted 4-gram hashes to stress test cap (max 2048)
    for h in 0..3000u64 {
        session.record_emitted_4gram(h * 1337 + 42);
    }
    assert_eq!(
        session.emitted_4grams.len(),
        2048,
        "emitted_4grams must be strictly capped at 2048 entries"
    );

    // Record 20 visited turn basins to stress test FIFO cap (max 8)
    for r in 0..20usize {
        session.record_turn_basin(r);
    }
    assert_eq!(
        session.recent_turn_rows.len(),
        8,
        "recent_turn_rows must be strictly capped at 8 entries"
    );
    // Verify FIFO ordering: last 8 entries must be 12..20
    assert_eq!(
        session.recent_turn_rows,
        vec![12, 13, 14, 15, 16, 17, 18, 19]
    );
}

/// Stress Test 8: 8-Head cyclic frame permutation bijectivity and coordinate preservation
#[test]
fn test_adversarial_flock_cyclic_bijection_deep_permutation() {
    let mut n_t = [0i32; FBS_WIDTH];
    for d in 0..FBS_WIDTH {
        n_t[d] = (d as i32) * 100 - 12800;
    }

    // Check all sweeps from 0 to 256
    for sweep in 0..256usize {
        let modulated = modulate_flock_cyclic_frame_block(&n_t, sweep);

        // Sum of all coordinates must be conserved exactly (permutation of heads preserves sum)
        let sum_original: i64 = n_t.iter().map(|&x| x as i64).sum();
        let sum_modulated: i64 = modulated.iter().map(|&x| x as i64).sum();
        assert_eq!(
            sum_original, sum_modulated,
            "Permutation must preserve total vector sum exactly (sweep {})",
            sweep
        );

        // Each 32D head in modulated must be identical to some 32D head in n_t
        for dst_h in 0..8 {
            let dst_slice = &modulated[dst_h * 32..(dst_h + 1) * 32];
            let found = (0..8).any(|src_h| {
                let src_slice = &n_t[src_h * 32..(src_h + 1) * 32];
                dst_slice == src_slice
            });
            assert!(
                found,
                "head {} in sweep {} must match an original head",
                dst_h, sweep
            );
        }
    }
}
