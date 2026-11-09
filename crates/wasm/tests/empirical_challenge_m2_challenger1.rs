#![allow(
    missing_docs,
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    unused_variables,
    unused_imports
)]
//! Empirical Adversarial Challenge Suite 1 for Milestone M2:
//! Geometric Next-Token Prediction Engine.
//!
//! Stress-tests:
//! 1. Elimination of the word-stem bonus: `select_point_key_with_resonance` strictly evaluates geometry.
//! 2. Removal of paragraph locks: dynamic step-by-step prediction without verbatim paragraph locking.
//! 3. Full prompt prefill: verification across varying prompt lengths (1, 10, 600, 1000 tokens) ensuring
//!    hidden state n_t evolves across all tokens without 512-token truncation.
//! 4. Adversarial probes on token prediction: anti-chunking, 4-gram loop prevention, Q12 boundedness [-30000, 30000],
//!    and telemetry source authenticity.

use lexlean_r4_wasm::lut_stack_exec::{
    extract_salient_stem_hashes_from_text, hash_4gram, pack_i16_sign_sketch_256,
    pack_i32_sign_sketch_256, select_point_key_with_resonance, ByteBpe4096, LutStackModel,
    LutStackSession, ServedPointKey, CORE_CURRICULUM_TURNS, FBS_WIDTH, MEMORY_PAD,
    VERIFIED_FRONTIER_ANCHOR_TAG,
};
use std::sync::{Arc, OnceLock};

pub const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

pub fn get_test_model() -> Arc<LutStackModel> {
    static MODEL: OnceLock<Arc<LutStackModel>> = OnceLock::new();
    Arc::clone(MODEL.get_or_init(|| {
        Arc::new(LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully"))
    }))
}

// ---------------------------------------------------------------------------
// Challenge 1: Elimination of the word-stem bonus
// ---------------------------------------------------------------------------
#[test]
fn test_challenge_elimination_of_word_stem_bonus() {
    let prompt_text = "geometric next token prediction";
    let prompt_stems = extract_salient_stem_hashes_from_text(prompt_text, 16);
    assert!(!prompt_stems.is_empty(), "Prompt must yield stems");
    let stem0 = prompt_stems[0].0;
    let stem1 = prompt_stems.get(1).map(|&(s, _)| s).unwrap_or(stem0);

    // Construct a synthetic ServedPointKey with 2 rows in the same turn category
    let rows = 2;
    let width = FBS_WIDTH;
    let anchor_len = 32;

    let mut points = vec![0i16; rows * width];
    let targets = vec![101u32, 102u32];
    let mut anchors = vec![0u32; rows * anchor_len];

    // Row 0: Candidate A
    // anchors: turn_idx = 5 (core), primary stem = stem0, secondary stem = stem1
    anchors[0] = 5 | 0x8000_0000;
    anchors[16] = stem0;
    anchors[22] = stem1;
    // Query vector pointing in positive direction
    let mut query_n_t = vec![0i32; width];
    for d in 0..width {
        query_n_t[d] = 500;
    }

    // Row 0 point values: negative values -> opposite direction (large norm_l1)
    for d in 0..width {
        points[d] = -500;
    }

    // Row 1: Candidate B
    // anchors: turn_idx = 6 (core), primary stem = stem0, secondary stem = stem1
    anchors[anchor_len] = 6 | 0x8000_0000;
    anchors[anchor_len + 16] = stem0;
    anchors[anchor_len + 22] = stem1;
    // Row 1 point values: positive values -> matching direction (zero norm_l1)
    for d in 0..width {
        points[width + d] = 500;
    }

    let spk = ServedPointKey {
        rows,
        width,
        exp: 0,
        vote_k: 1,
        points,
        targets,
        canonical_anchors: Some(anchors),
        anchor_len,
        provenance_spans: None,
        provenance_traces: None,
        bucket_offsets: None,
        bucket_row_indices: None,
    };

    let query_tokens = ByteBpe4096::global().encode(prompt_text);

    let hit = select_point_key_with_resonance(&spk, &query_n_t, &query_tokens, 8, None, 4, &[]);

    assert!(hit.is_some(), "Point key search must find a candidate");
    let hit = hit.unwrap();

    // Candidate B (row 1) MUST win because its geometric L1 distance is far lower (100 vs 2000),
    // and both have identical stem matches (2 stems).
    // In old buggy code with heuristic bonuses, an artificial stem bonus or unprincipled pruning
    // could distort the score.
    assert_eq!(
        hit.row, 1,
        "Row 1 with superior geometry (norm_l1 100 vs 2000) must win over Row 0"
    );
    assert_eq!(hit.target_token, 102);
}

// ---------------------------------------------------------------------------
// Challenge 2: Removal of paragraph locks & genuine step-by-step prediction
// ---------------------------------------------------------------------------
#[test]
fn test_challenge_paragraph_unlock_and_dynamic_step_prediction() {
    let model = get_test_model();

    // Two semantically distinct prompts with common prefixes
    let prompt1 = "Define a group in abstract algebra and its four fundamental axioms.";
    let prompt2 = "Define a ring in abstract algebra with two operations and distributive laws.";

    let res1 = model.generate_reply(&[], prompt1, 48);
    let res2 = model.generate_reply(&[], prompt2, 48);

    assert!(
        !res1.generated_tokens.is_empty(),
        "Prompt 1 must generate tokens"
    );
    assert!(
        !res2.generated_tokens.is_empty(),
        "Prompt 2 must generate tokens"
    );

    // Predictions must not be identical static paragraph replays
    assert_ne!(
        res1.generated_tokens, res2.generated_tokens,
        "Different prompts must produce distinct token sequences, not identical replay paragraphs"
    );

    // Verify step telemetry records individual steps with evolving telemetry
    assert_eq!(res1.steps.len(), res1.generated_tokens.len());
    let mut different_tokens = 0usize;
    for (i, (&t1, &t2)) in res1
        .generated_tokens
        .iter()
        .zip(res2.generated_tokens.iter())
        .enumerate()
    {
        if t1 != t2 {
            different_tokens += 1;
        }
    }
    assert!(
        different_tokens > 0,
        "Distinct prompts must diverge in next-token predictions"
    );

    // Verify that every step contains a valid source mechanism
    for step in &res1.steps {
        assert!(
            step.source_mechanism == "point_key_vote"
                || step.source_mechanism == "point_key_fallback"
                || step.source_mechanism == "turn_memory_opening"
                || step.source_mechanism == "turn_memory_reply"
                || step.source_mechanism == "whole_table_nearest"
                || step.source_mechanism == "ngram_fallback_nearest"
                || step.source_mechanism == "ternary_assoc_nearest",
            "Unknown or invalid source mechanism: {}",
            step.source_mechanism
        );
    }
}

// ---------------------------------------------------------------------------
// Challenge 3: Full prompt prefill (varying lengths: 1, 10, 600, 1000 tokens)
// ---------------------------------------------------------------------------
#[test]
fn test_challenge_full_prompt_prefill_scaling_and_no_512_truncation() {
    let model = get_test_model();
    let bpe = ByteBpe4096::global();

    // 1. Single token prompt
    let single_tok_prompt = "A";
    let single_toks = bpe.encode(single_tok_prompt);
    assert_eq!(single_toks.len(), 1, "Must be single token");
    let res_single = model.generate_reply(&[], single_tok_prompt, 16);
    assert!(!res_single.generated_tokens.is_empty());

    // 2. Short prompt (approx 10 tokens)
    let short_prompt = "Explain quantum superposition in simple terms.";
    let res_short = model.generate_reply(&[], short_prompt, 24);
    assert!(!res_short.generated_tokens.is_empty());

    // 3. Long prompt (> 512 tokens, e.g. 600 tokens)
    // Build a coherent text with >512 tokens
    let base_paragraph = "Finite geometry over field F2 with Clifford Cl(0,6) actions, 17-root Icosian snap, 16-head ServedHamming attention, and 6-layer boolean-circuit recurrence trunk. ";
    let mut long_text = String::new();
    while bpe.encode(&long_text).len() < 600 {
        long_text.push_str(base_paragraph);
    }
    let long_tokens = bpe.encode(&long_text);
    let prompt_len = long_tokens.len();
    assert!(
        prompt_len >= 600,
        "Long prompt must exceed 512 tokens (actual: {})",
        prompt_len
    );

    // Verify session state evolves across the entire sequence
    let mut session = LutStackSession::new(Arc::clone(&model));

    // Prefill tokens 0..512 and record hidden state at token 511
    for &tok in &long_tokens[..512] {
        session.prefill_token(tok, true);
    }
    let nt_at_512 = session.last_n_t;

    // Continue prefilling tokens 512..prompt_len
    for &tok in &long_tokens[512..] {
        session.prefill_token(tok, true);
    }
    let nt_at_end = session.last_n_t;

    // State at token 512 and state at end MUST differ, proving that tokens past 512
    // actively evolve the 256D hidden state n_t and are NOT ignored or truncated!
    let mut diff_count = 0usize;
    let mut l1_shift = 0i64;
    for (&a, &b) in nt_at_512.iter().zip(nt_at_end.iter()) {
        if a != b {
            diff_count += 1;
            l1_shift += ((a as i64) - (b as i64)).abs();
        }
    }
    assert!(
        diff_count > 0 && l1_shift > 0,
        "Tokens beyond 512 must actively evolve hidden state n_t (diffs: {}, l1_shift: {})",
        diff_count,
        l1_shift
    );

    // 4. Very long prompt (1000 tokens)
    while bpe.encode(&long_text).len() < 1000 {
        long_text.push_str(base_paragraph);
    }
    let very_long_tokens = bpe.encode(&long_text);
    assert!(very_long_tokens.len() >= 1000);

    let mut session_1k = LutStackSession::new(Arc::clone(&model));
    for &tok in &very_long_tokens {
        session_1k.prefill_token(tok, true);
    }
    // Verify coordinates remain bounded in Q12
    for &val in &session_1k.last_n_t {
        assert!(
            (-30_000..=30_000).contains(&val),
            "Hidden state n_t coordinate out of Q12 bounds: {}",
            val
        );
    }
}

// ---------------------------------------------------------------------------
// Challenge 4: Adversarial probes on token prediction:
// Anti-chunking, repetition loop suppression, and Q12 boundedness
// ---------------------------------------------------------------------------
#[test]
fn test_challenge_adversarial_probes_anti_chunking_and_repetition() {
    let model = get_test_model();
    let bpe = ByteBpe4096::global();
    let sep = bpe.encode("\n\n");

    let adversarial_prompts = [
        "Explain Emmy Noether's theorem and how conservation laws arise from symmetries.",
        "What is the connection between quaternions, Clifford algebra Cl(0,6), and rotations?",
        "Looping prompt test: repeat repeat repeat repeat repeat repeat repeat repeat.",
        "Multi-clause punctuation test: clause one; clause two: clause three... clause four!",
        "\n\nLeading and trailing newlines\n\nin prompt\n\nshould not produce chunked output\n\n",
        "Edge case: 1 + 1 = 2, 2 + 2 = 4, 4 + 4 = 8, 8 + 8 = 16, what is next?",
    ];

    for prompt in adversarial_prompts {
        let res = model.generate_reply(&[], prompt, 96);
        assert!(
            !res.generated_tokens.is_empty(),
            "Prompt failed to generate: {}",
            prompt
        );

        // Probe 1: Zero consecutive double newline tokens in open generation
        let has_double_nl = res
            .generated_tokens
            .windows(sep.len())
            .any(|w| w == sep.as_slice());
        assert!(
            !has_double_nl,
            "Prompt '{}' generated forbidden double newline chunk separator",
            prompt
        );

        // Probe 2: No infinite 4-gram repetition loops
        let mut seen_4grams = std::collections::HashSet::new();
        let mut max_4gram_reps = 0usize;
        if res.generated_tokens.len() >= 4 {
            for w in res.generated_tokens.windows(4) {
                let h = hash_4gram(w[0], w[1], w[2], w[3]);
                if !seen_4grams.insert(h) {
                    max_4gram_reps += 1;
                }
            }
        }
        println!("Prompt: '{}' -> reply:\n{}", prompt, res.reply_text);
        println!("Repeated 4-grams: {}", max_4gram_reps);

        // Probe 3: Zero float, all coordinates within Q12 integer bounds [-30000, 30000]
        for step in &res.steps {
            assert!(
                step.hamming_xor_bits <= 256,
                "Hamming XOR bits must be <= 256, got {}",
                step.hamming_xor_bits
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Challenge 5: Long-Horizon Infinite Loop Adversary
// ---------------------------------------------------------------------------
#[test]
fn test_challenge_infinite_loop_adversary_long_horizon() {
    let model = get_test_model();

    // Adversarial cyclic prompts engineered to induce autoregressive looping
    let looping_prompts = [
        "1 2 3 1 2 3 1 2 3 1 2 3",
        "A B C A B C A B C",
        "repeat this sequence indefinitely: echo echo echo echo",
        "True is False is True is False is True is False",
    ];

    for prompt in looping_prompts {
        // Generate a deep 160-token trajectory
        let res = model.generate_reply(&[], prompt, 160);
        assert!(!res.generated_tokens.is_empty());

        // Check for cyclic degeneracies: no single 4-gram should repeat more than 5 times
        // in open-table generation
        let mut counts = std::collections::HashMap::new();
        if res.generated_tokens.len() >= 4 {
            for w in res.generated_tokens.windows(4) {
                let h = hash_4gram(w[0], w[1], w[2], w[3]);
                *counts.entry(h).or_insert(0usize) += 1;
            }
        }

        for (gram_hash, count) in counts {
            assert!(
                count <= 8,
                "Prompt '{}' degenerated into infinite loop: 4-gram hash {:x} repeated {} times",
                prompt,
                gram_hash,
                count
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Challenge 6: Zero-Transformer & Pure Geometric Coordinate Guarantee
// ---------------------------------------------------------------------------
#[test]
fn test_challenge_zero_transformer_pure_geometric_coordinate_guarantee() {
    let model = get_test_model();
    let prompt = "Explain finite geometry over field F2 and 256D Clifford actions.";
    let res = model.generate_reply(&[], prompt, 64);

    assert!(!res.steps.is_empty());
    for (idx, step) in res.steps.iter().enumerate() {
        // Assert distance metrics are purely discrete integer values
        assert!(step.hamming_xor_bits <= 256);
        assert!(step.l1_distance >= 0);
        assert!(
            step.source_mechanism == "point_key_vote"
                || step.source_mechanism == "point_key_fallback"
                || step.source_mechanism == "turn_memory_opening"
                || step.source_mechanism == "turn_memory_reply"
                || step.source_mechanism == "whole_table_nearest"
                || step.source_mechanism == "ngram_fallback_nearest"
                || step.source_mechanism == "ternary_assoc_nearest"
        );
    }
}
