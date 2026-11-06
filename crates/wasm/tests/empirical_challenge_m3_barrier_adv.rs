#![allow(
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    clippy::identity_op,
    clippy::manual_range_contains,
    clippy::useless_vec
)]
//! Adversarial Empirical Challenge Suite for Milestone M3:
//! Visible Reply Commencement Barrier Enforcement & Anti-Leakage Adversary
//!
//! Evaluates:
//! 1. Visible reply prediction commences strictly after `</thinking>` has updated `n_t`.
//! 2. Zero leakage of reasoning tags (`<thinking>`, `</thinking>`, `<thought>`, `</thought>`)
//!    into `res.reply_text` or `res.generated_tokens` across all generation and tracing pipelines.
//! 3. `reply_history` and `suffix_counts` strictly track visible tokens emitted after `reply_start` barrier.
//! 4. 4-gram repetition fallback operates correctly on visible reply tokens without considering
//!    prefilled reasoning tokens.
//! 5. Adversarial input resilience: unclosed tags, empty tags, whitespace variations,
//!    rapid tag switching, nested tags, and orphan tags.

use lexlean_r4_wasm::lut_stack_exec::{
    hash_4gram, ByteBpe4096, LutStackModel, LutStackSession, RollingContextState, MEMORY_PAD,
};
use std::sync::Arc;

const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

fn load_test_model() -> Arc<LutStackModel> {
    Arc::new(LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully"))
}

/// Adversarial Challenge 1:
/// Verify visible reply prediction commences strictly after `</thinking>` barrier.
/// - During prefill and reasoning stepping, `reply_start` must remain `None`.
/// - `step_reasoning_trace` must establish `reply_start == Some(tokens.len() + 1)` strictly after `</thinking>`.
/// - `commence_visible_prediction()` must evaluate on the steered `last_n_t`.
/// - First visible step must initialize `reply_history` with exactly the first visible token.
#[test]
fn challenge_adv_barrier_strict_post_thinking_commencement() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    let prompt = "Solve the following: What is the prime factorization of 360?";
    let prompt_tokens = bpe.encode(prompt);

    let mut session = LutStackSession::new(model.clone());

    // Step A: Prefill prompt tokens
    for &tok in &prompt_tokens {
        session.prefill_token(tok, true);
        assert!(
            session.reply_start.is_none(),
            "barrier violation: reply_start set during prompt prefill"
        );
    }
    let pre_reasoning_n_t = session.last_n_t;
    let pre_reasoning_token_count = session.tokens.len();

    // Step B: Autoregressively step reasoning tokens
    let reasoning_text = "360 = 36 * 10 = (6^2) * (2 * 5) = (2^2 * 3^2) * 2 * 5 = 2^3 * 3^2 * 5^1.";
    let (th_tokens, steered_n_t) = session.step_reasoning_trace(reasoning_text);

    // Verify reasoning tokens were prefilled and actively steered n_t
    assert!(!th_tokens.is_empty(), "reasoning tokens must be non-empty");
    assert_eq!(
        session.tokens.len(),
        pre_reasoning_token_count + th_tokens.len(),
        "all reasoning tokens must be prefilled into session.tokens"
    );
    assert_eq!(session.last_n_t, steered_n_t);

    let l1_steer: u64 = steered_n_t
        .iter()
        .zip(pre_reasoning_n_t.iter())
        .map(|(&a, &b)| (a as i64 - b as i64).unsigned_abs())
        .sum();
    assert!(
        l1_steer > 200,
        "reasoning trace must steer hidden state (L1 delta = {}, expected > 200)",
        l1_steer
    );

    // Step C: Verify barrier state strictly after </thinking>
    let expected_reply_start = session.tokens.len() + 1;
    assert_eq!(
        session.reply_start,
        Some(expected_reply_start),
        "reply_start must be exactly tokens.len() + 1 after </thinking>"
    );
    assert!(
        session.reply_history.is_empty(),
        "reply_history must be strictly empty at barrier transition"
    );
    assert!(
        session.suffix_counts.is_empty(),
        "suffix_counts must be strictly empty at barrier transition"
    );

    // Step D: Commence visible prediction
    let (first_step, first_tok) = session.commence_visible_prediction();
    assert_ne!(first_tok, 0, "first visible token must be valid");
    assert_eq!(first_step.predicted_token, first_tok);

    // Tokens count has not incremented yet because first_tok has not been stepped
    assert_eq!(
        session.tokens.len(),
        pre_reasoning_token_count + th_tokens.len()
    );
    assert!(session.reply_history.is_empty());

    // Step E: Step the first visible token through telemetry
    let st = session.step_with_telemetry(first_tok);
    assert_eq!(session.tokens.len(), expected_reply_start);
    assert_eq!(
        session.reply_history.len(),
        1,
        "reply_history must contain exactly 1 visible token"
    );
    assert_eq!(
        session.reply_history[0], first_tok,
        "reply_history[0] must match the first visible token"
    );
    assert_eq!(
        session.suffix_counts.len(),
        1,
        "suffix_counts must contain exactly 1 entry"
    );
    assert_eq!(
        session.suffix_counts[0],
        ([MEMORY_PAD, MEMORY_PAD, MEMORY_PAD, first_tok], 1),
        "suffix_counts[0] must be padded 4-gram of first visible token"
    );
    assert_ne!(st.predicted_token, 0);
}

/// Adversarial Challenge 2:
/// Verify `reply_history` and `suffix_counts` strictly isolate visible tokens.
/// - Prefilled reasoning tokens must NEVER appear in `reply_history`.
/// - Prefilled reasoning tokens must NEVER appear in `suffix_counts`.
/// - In a multi-step decoding sequence, `reply_history` length must strictly track visible tokens.
#[test]
fn challenge_adv_reply_history_and_suffix_counts_isolation() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    let mut session = LutStackSession::new(model.clone());

    // Prefill prompt
    let prompt_toks = bpe.encode("Compute sum 1 to 100.");
    for &tok in &prompt_toks {
        session.prefill_token(tok, true);
    }

    // Step reasoning trace
    let trace_text = "Use Gauss's formula: n*(n+1)/2 = 100*101/2 = 5050.";
    let (_reasoning_tokens, _) = session.step_reasoning_trace(trace_text);

    // Verify isolation prior to visible decoding
    assert!(session.reply_history.is_empty());
    assert!(session.suffix_counts.is_empty());

    // Execute 12 visible decoding steps
    let (first_step, first_tok) = session.commence_visible_prediction();
    let mut visible_tokens = vec![first_tok];
    let mut cur_tok = first_tok;
    let mut next_tok = first_step.predicted_token;

    for step_num in 1..=12 {
        let st = session.step_with_telemetry(cur_tok);
        assert_eq!(
            session.reply_history.len(),
            step_num,
            "reply_history length must equal number of stepped visible tokens"
        );
        assert_eq!(
            session.reply_history, visible_tokens,
            "reply_history must strictly equal visible_tokens sequence"
        );

        // Verify suffix_counts only contains combinations from reply_history
        for &(suffix, count) in &session.suffix_counts {
            assert!(count >= 1);
            // Suffix tokens must either be MEMORY_PAD or must have been emitted in visible_tokens
            for &stok in &suffix {
                if stok != MEMORY_PAD {
                    assert!(
                        visible_tokens.contains(&stok),
                        "suffix token {} not found in visible_tokens",
                        stok
                    );
                }
            }
        }

        cur_tok = next_tok;
        visible_tokens.push(cur_tok);
        next_tok = st.predicted_token;
    }
}

/// Adversarial Challenge 3:
/// Verify 4-gram repetition fallback operates correctly on visible reply tokens
/// without considering prefilled reasoning tokens.
/// - A 4-gram present in reasoning tokens must NOT be suppressed in visible reply on first emission.
/// - A 4-gram repeated within visible reply tokens MUST trigger fallback.
#[test]
fn challenge_adv_4gram_repetition_fallback_operates_on_visible_tokens_only() {
    let model = load_test_model();

    // Session A: Prefill prompt and reasoning tokens containing an explicit 4-gram sequence
    let mut session_a = LutStackSession::new(model.clone());
    session_a.prefill_token(10, true);

    // Prefill reasoning trace that includes tokens [101, 102, 103, 58]
    let reasoning_seq = [101u32, 102, 103, 58];
    for &tok in &reasoning_seq {
        session_a.prefill_token(tok, true);
    }
    let reasoning_hash = hash_4gram(101, 102, 103, 58);
    session_a.mark_reply_start();

    // Critical Invariant: Reasoning tokens must NOT populate emitted_4grams!
    assert!(
        session_a.emitted_4grams.is_empty(),
        "reasoning tokens must NOT populate emitted_4grams"
    );
    assert!(!session_a.emitted_4grams.contains(&reasoning_hash));

    // Visible prediction begins: step 101, 102, 103
    session_a.step_with_telemetry(101);
    session_a.step_with_telemetry(102);
    let st_first = session_a.step_with_telemetry(103);

    let cand0 = st_first.top_candidates[0].0;
    let cand1 = st_first.top_candidates[1].0;

    // Because reasoning tokens were not registered in emitted_4grams,
    // candidate 0 MUST be selected directly without repetition fallback!
    assert_ne!(
        st_first.source_mechanism, "ngram_fallback_nearest",
        "first visible emission must NOT trigger ngram_fallback_nearest"
    );
    assert_eq!(
        st_first.predicted_token, cand0,
        "candidate 0 must be selected on first visible emission"
    );

    // The chosen 4-gram is now recorded into emitted_4grams
    let chosen_cand0_hash = hash_4gram(101, 102, 103, cand0);
    assert!(
        session_a.emitted_4grams.contains(&chosen_cand0_hash),
        "chosen_cand0_hash must now be in emitted_4grams"
    );

    // Session B: In an identical state, pre-record chosen_cand0_hash to simulate a repeat emission
    let mut session_b = LutStackSession::new(model.clone());
    session_b.prefill_token(10, true);
    for &tok in &reasoning_seq {
        session_b.prefill_token(tok, true);
    }
    session_b.mark_reply_start();
    session_b.record_emitted_4gram(chosen_cand0_hash);

    session_b.step_with_telemetry(101);
    session_b.step_with_telemetry(102);
    let st_second = session_b.step_with_telemetry(103);

    // Now candidate 0 IS in emitted_4grams, so repetition fallback MUST trigger!
    assert_eq!(
        st_second.source_mechanism, "ngram_fallback_nearest",
        "must trigger ngram_fallback_nearest when 4-gram repeats in visible output"
    );
    assert_eq!(
        st_second.predicted_token, cand1,
        "must fall back to candidate 1"
    );
}

/// Adversarial Challenge 4:
/// Assert zero leakage of reasoning tags into `res.reply_text` or `res.generated_tokens`
/// across `generate_reply` and `generate_reply_with_context`.
#[test]
fn challenge_adv_zero_leakage_across_generation_pipelines() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();

    // Adversarial Case A: Prompt containing <thinking>...</thinking>
    let prompt_a = "Define entropy. <thinking>\nEntropy in thermodynamics measures unavailable energy: S = k ln Omega.\n</thinking>";
    let res_a = model.generate_reply(&[], prompt_a, 24);

    assert!(
        !res_a.reply_text.contains("<thinking>"),
        "leakage in reply_text: {}",
        res_a.reply_text
    );
    assert!(
        !res_a.reply_text.contains("</thinking>"),
        "leakage in reply_text: {}",
        res_a.reply_text
    );
    assert!(
        !res_a.reply_text.contains("<thought>"),
        "leakage in reply_text: {}",
        res_a.reply_text
    );
    assert!(
        !res_a.reply_text.contains("</thought>"),
        "leakage in reply_text: {}",
        res_a.reply_text
    );

    let decoded_toks_a = bpe.decode(&res_a.generated_tokens);
    assert!(
        !decoded_toks_a.contains("<thinking>"),
        "leakage in generated_tokens: {}",
        decoded_toks_a
    );
    assert!(
        !decoded_toks_a.contains("</thinking>"),
        "leakage in generated_tokens: {}",
        decoded_toks_a
    );

    assert!(
        res_a.thinking_text.is_some(),
        "thinking_text must be captured"
    );
    assert!(
        !res_a.thinking_tokens.is_empty(),
        "thinking_tokens must be captured"
    );

    // Adversarial Case B: Prompt containing <thought>...</thought>
    let prompt_b = "State Euler's formula. <thought>\ne^(i*pi) + 1 = 0 connects five fundamental constants.\n</thought>";
    let mut context_b = RollingContextState::default();
    let res_b = model.generate_reply_with_context(&mut context_b, &[], prompt_b, 24);

    assert!(
        !res_b.reply_text.contains("<thinking>"),
        "leakage in reply_text: {}",
        res_b.reply_text
    );
    assert!(
        !res_b.reply_text.contains("</thinking>"),
        "leakage in reply_text: {}",
        res_b.reply_text
    );
    assert!(
        !res_b.reply_text.contains("<thought>"),
        "leakage in reply_text: {}",
        res_b.reply_text
    );
    assert!(
        !res_b.reply_text.contains("</thought>"),
        "leakage in reply_text: {}",
        res_b.reply_text
    );

    let decoded_toks_b = bpe.decode(&res_b.generated_tokens);
    assert!(
        !decoded_toks_b.contains("<thought>"),
        "leakage in generated_tokens: {}",
        decoded_toks_b
    );
    assert!(
        !decoded_toks_b.contains("</thought>"),
        "leakage in generated_tokens: {}",
        decoded_toks_b
    );

    // Adversarial Case C: Tracing dynamic turn with reasoning tags in reply_text
    let mock_reply = "<thinking>\nStep 1: Parse input.\nStep 2: Emit response.\n</thinking>\nEntropy is a measure of molecular disorder.";
    let trace_res = model.trace_dynamic_turn(&[], "Define entropy.", mock_reply, 32);

    assert!(
        !trace_res.reply_text.contains("<thinking>"),
        "trace_dynamic_turn leaked <thinking>: {}",
        trace_res.reply_text
    );
    assert!(
        !trace_res.reply_text.contains("</thinking>"),
        "trace_dynamic_turn leaked </thinking>: {}",
        trace_res.reply_text
    );
    assert_eq!(
        trace_res.reply_text,
        "Entropy is a measure of molecular disorder."
    );
    assert!(trace_res.thinking_text.is_some());
    assert_eq!(
        trace_res.thinking_text.as_deref().unwrap(),
        "Step 1: Parse input.\nStep 2: Emit response."
    );
}

/// Adversarial Challenge 5:
/// Stress-test adversarial reasoning inputs through the public generation pipeline:
/// - Unclosed tags: `<thinking>unclosed`
/// - Whitespace variations: `\n<thinking>\n...\n</thinking>\n`
/// - Rapid tag switching: `<thinking>1</thinking> <thought>2</thought> <thinking>3</thinking>`
/// - Empty tags: `<thinking></thinking>` and `<thought></thought>`
/// - Incomplete tags: `<thinking`, `</thinking`
#[test]
fn challenge_adv_adversarial_reasoning_inputs_and_edge_cases() {
    let model = load_test_model();

    // 1. Unclosed thinking tag in generate_reply
    let unclosed_prompt = "What is pi? <thinking>pi is circumference over diameter";
    let res_unclosed = model.generate_reply(&[], unclosed_prompt, 16);
    assert!(!res_unclosed.reply_text.contains("<thinking>"));
    assert!(!res_unclosed.reply_text.contains("</thinking>"));
    assert!(res_unclosed.thinking_text.is_some());
    assert_eq!(
        res_unclosed.thinking_text.as_deref(),
        Some("pi is circumference over diameter")
    );

    // 2. Whitespace variations around tags in trace_dynamic_turn
    let ws_reply = "\n\n<thinking>\n   multiline reasoning with extra whitespace   \n</thinking>\n\nAnswer text.";
    let res_ws = model.trace_dynamic_turn(&[], "Prompt", ws_reply, 16);
    assert!(!res_ws.reply_text.contains("<thinking>"));
    assert!(!res_ws.reply_text.contains("</thinking>"));
    assert_eq!(res_ws.reply_text, "Answer text.");
    assert_eq!(
        res_ws.thinking_text.as_deref(),
        Some("multiline reasoning with extra whitespace")
    );

    // 3. Rapid tag switching and multiple mixed blocks
    let mixed_reply = "<thinking>Trace 1</thinking> middle prose <thought>Trace 2</thought> final prose <thinking>Trace 3</thinking>";
    let res_mixed = model.trace_dynamic_turn(&[], "Prompt", mixed_reply, 32);
    assert!(!res_mixed.reply_text.contains("<thinking>"));
    assert!(!res_mixed.reply_text.contains("</thinking>"));
    assert!(!res_mixed.reply_text.contains("<thought>"));
    assert!(!res_mixed.reply_text.contains("</thought>"));
    assert_eq!(
        res_mixed.thinking_text.as_deref(),
        Some("Trace 1\n\nTrace 2\n\nTrace 3")
    );
    assert_eq!(res_mixed.reply_text, "middle prose  final prose");

    // 4. Empty tags
    let empty_prompt = "Query? <thinking></thinking> <thought>   </thought>";
    let res_empty = model.generate_reply(&[], empty_prompt, 16);
    assert!(!res_empty.reply_text.contains("<thinking>"));
    assert!(!res_empty.reply_text.contains("<thought>"));
    assert!(res_empty.thinking_text.is_none());

    // 5. Incomplete tags like `<thinking` (missing closing bracket)
    let incomplete_prompt = "Query? <thinking reasoning without bracket";
    let res_incomplete = model.generate_reply(&[], incomplete_prompt, 16);
    assert!(!res_incomplete.reply_text.is_empty());
}

/// Adversarial Challenge 6:
/// Stress-test multi-turn context persistence and anti-leakage invariants:
/// - Multi-turn reasoning across consecutive turns with `RollingContextState`.
/// - Verify `thinking_trace` persists in context state without contaminating visible reply.
/// - Verify that previous turns' thinking traces do not leak into subsequent turn prompt or reply text.
#[test]
fn challenge_adv_multi_turn_context_isolation_and_no_leakage() {
    let model = load_test_model();
    let bpe = ByteBpe4096::global();
    let mut context = RollingContextState::default();

    // Turn 1: user prompt with thinking tag
    let turn1_prompt = "What is Gödel's First Incompleteness Theorem? <thinking>\nIn any consistent formal system capable of arithmetic, there are true statements unprovable within the system.\n</thinking>";
    let res1 = model.generate_reply_with_context(&mut context, &[], turn1_prompt, 20);

    assert!(!res1.reply_text.contains("<thinking>"));
    assert!(!res1.reply_text.contains("</thinking>"));
    assert!(res1.thinking_text.is_some());

    // Record Turn 1 in context
    context.blend_latent(&res1.final_n_t);
    if let Some(ref th) = res1.thinking_text {
        context.record_thinking(th.clone());
    }

    assert_eq!(
        context.thinking_trace.last().map(|s| s.as_str()),
        Some("In any consistent formal system capable of arithmetic, there are true statements unprovable within the system.")
    );

    // Turn 2: Follow-up question referencing Turn 1
    let history = vec![(turn1_prompt.to_string(), res1.reply_text.clone())];
    let turn2_prompt = "Does it apply to Presburger arithmetic? <thought>\nPresburger arithmetic is complete and decidable because it lacks multiplication.\n</thought>";
    let res2 = model.generate_reply_with_context(&mut context, &history, turn2_prompt, 20);

    assert!(!res2.reply_text.contains("<thinking>"));
    assert!(!res2.reply_text.contains("</thinking>"));
    assert!(!res2.reply_text.contains("<thought>"));
    assert!(!res2.reply_text.contains("</thought>"));
    assert!(res2.thinking_text.is_some());

    let decoded_toks2 = bpe.decode(&res2.generated_tokens);
    assert!(!decoded_toks2.contains("<thought>"));
    assert!(!decoded_toks2.contains("</thought>"));
}
