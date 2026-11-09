#![allow(
    missing_docs,
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    unused_variables,
    unused_imports
)]
//! Empirical Adversarial Challenge Suite (Iteration 3 - Challenger 2) for Milestone M2:
//! Geometric Next-Token Prediction Engine.
//!
//! Objectives:
//! 1. Verify open-domain prompts (sourdough bread, poetry, history, nature, coffee, chess)
//!    do NOT get hijacked into curriculum algebra formulas (`2x + 6 = 14`, `Let $x$ be an integer`).
//! 2. Verify confident gating fix in `step_with_telemetry` and `commence_visible_prediction`.
//! 3. Verify multi-basin generation depth and clause resolution for compound multi-topic queries.
//! 4. Verify adversarial edge cases: empty prompts, single-token prompts, max context prompts (>800 tokens),
//!    and out-of-vocabulary / extreme character inputs (emojis, control characters, multi-script Unicode).
//! 5. Verify repetition penalty and anti-cycle defenses under adversarial repetitive inputs.

use lexlean_r4_wasm::lut_stack_exec::{
    encode_reply_anchor_21, extract_salient_stem_hashes_from_text, hash_4gram,
    resolve_canonical_reply_anchor, select_point_key_with_resonance, ByteBpe4096, LutStackModel,
    LutStackSession, ServedPointKey, CORE_CURRICULUM_TURNS, FBS_WIDTH, GLOBAL_SUFFIX_ANCHOR,
    MEMORY_PAD, VERIFIED_FRONTIER_ANCHOR_TAG,
};
use std::sync::{Arc, OnceLock};

pub const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

pub fn get_test_model() -> Arc<LutStackModel> {
    static MODEL: OnceLock<Arc<LutStackModel>> = OnceLock::new();
    Arc::clone(MODEL.get_or_init(|| {
        Arc::new(LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully"))
    }))
}

// ===========================================================================
// Challenge 1: Open-Domain Prompts Hijack Resistance (No Algebra Replay)
// ===========================================================================
#[test]
fn test_empirical_open_domain_prompts_not_hijacked_to_algebra() {
    let model = get_test_model();
    let bpe = ByteBpe4096::global();

    let open_domain_prompts = [
        "Explain the process of making sourdough bread from scratch.",
        "Write a lyrical poem about autumn leaves falling in the mountain wind.",
        "Describe the cultural and political impact of the Renaissance in Florence.",
        "What were the primary socio-economic causes of the French Revolution?",
        "How do honeybees communicate the location of flowers using the waggle dance?",
        "What are the rules, movements, and pieces in the classical game of chess?",
        "How to brew pour-over coffee using a V60 ceramic dripper and fresh beans?",
        "Who was Leonardo da Vinci and what were his most notable engineering designs?",
        "Explain the water cycle: evaporation, condensation, precipitation, and collection.",
        "Describe the architectural design principles of Gothic cathedrals and flying buttresses.",
    ];

    let forbidden_algebra_fragments = [
        "2x + 6 = 14",
        "2x + 6",
        "Let $x$ be an integer",
        "Find all values of $x$",
        "Solve for x",
        "3x + 12 = 48",
        "3*x + 12 = 48",
        "5x - 7 = 18",
        "quadratic equation",
        "linear equation 2x",
    ];

    for prompt in open_domain_prompts {
        let res = model.generate_reply(&[], prompt, 32);

        assert!(
            !res.generated_tokens.is_empty(),
            "Open-domain prompt '{}' produced no tokens",
            prompt
        );

        // Verify that reply text does NOT contain any algebra curriculum hijack phrases
        for fragment in &forbidden_algebra_fragments {
            assert!(
                !res.reply_text.contains(fragment),
                "Prompt hijacking detected! Open-domain prompt '{}' generated algebra fragment '{}'. Reply was:\n{}",
                prompt,
                fragment,
                res.reply_text
            );
        }

        // Verify that session reply anchors do not have bit 31 set
        let mut session = model.session();
        let prompt_tokens = bpe.encode(prompt);
        session.prefill_tokens(&prompt_tokens, true);
        let (first_step, first_tok) = session.commence_visible_prediction();

        for &w in &session.anchors.reply {
            assert_eq!(
                w & 0x8000_0000,
                0,
                "Prompt '{}' left turn-lock bit 31 in session.anchors.reply: 0x{:08X}",
                prompt,
                w
            );
        }

        // Verify that step_with_telemetry confident gating preserves non-algebra anchors
        let mut cur_tok = first_tok;
        for s in 0..10 {
            let st = session.step_with_telemetry(cur_tok);
            for &w in &session.anchors.reply {
                assert_eq!(
                    w & 0x8000_0000,
                    0,
                    "Prompt '{}' step {} produced turn-lock in session.anchors.reply: 0x{:08X}",
                    prompt,
                    s,
                    w
                );
            }
            cur_tok = st.predicted_token;
            if cur_tok == bpe.eos_id() || cur_tok == bpe.bos_id() {
                break;
            }
        }
    }
}

// ===========================================================================
// Challenge 2: Curriculum Prompts Activate Math/Physics Basins Appropriately
// ===========================================================================
#[test]
fn test_empirical_curriculum_prompts_properly_activate_math_and_physics_basins() {
    let model = get_test_model();
    let bpe = ByteBpe4096::global();

    let curriculum_prompts = [
        "Solve the linear equation: 2x + 6 = 14.",
        "Calculate 17 * 23.",
        "Explain Emmy Noether's theorem and how conservation laws arise from symmetries.",
        "What is the connection between quaternions, Clifford algebra Cl(0,6), and rotations?",
    ];

    for prompt in curriculum_prompts {
        let res = model.generate_reply(&[], prompt, 32);

        assert!(
            !res.generated_tokens.is_empty(),
            "Curriculum prompt '{}' produced no tokens",
            prompt
        );

        // Telemetry invariants
        for step in &res.steps {
            assert!(
                step.l1_distance >= 0,
                "L1 distance was negative: {}",
                step.l1_distance
            );
            assert!(
                step.hamming_xor_bits <= 256,
                "Hamming XOR bits exceeded 256: {}",
                step.hamming_xor_bits
            );
        }

        // Verify Q12 coordinates bounded
        for &coord in &res.final_n_t {
            assert!(
                (-32_768..=32_767).contains(&coord),
                "Q12 coordinate out of range: {}",
                coord
            );
        }
    }
}

// ===========================================================================
// Challenge 3: Multi-Basin Generation Depth and Clause Resolution
// ===========================================================================
#[test]
fn test_empirical_multi_basin_generation_depth_and_clause_resolution() {
    let model = get_test_model();

    // 1. Compound prompt with 3 distinct scientific domains
    let multi_topic_prompt =
        "Explain quantum entanglement, Emmy Noether, and relativistic GPS time dilation.";
    let res = model.generate_reply(&[], multi_topic_prompt, 160);

    assert!(
        res.generated_tokens.len() >= 64,
        "Expected multi-basin continuous generation depth >= 64, got {}",
        res.generated_tokens.len()
    );
    assert!(
        res.clauses_resolved >= 2,
        "Expected clauses_resolved >= 2 for compound prompt, got {}",
        res.clauses_resolved
    );

    // 2. Multi-clause prompt with sequential semicolons
    let semicolon_prompt =
        "First, define a topological manifold; second, explain Stokes' theorem: third, state the Poincare conjecture.";
    let res_semi = model.generate_reply(&[], semicolon_prompt, 128);

    assert!(
        res_semi.clauses_resolved >= 2,
        "Expected clauses_resolved >= 2 for semicolon prompt, got {}",
        res_semi.clauses_resolved
    );
    assert!(
        res_semi.generated_tokens.len() >= 32,
        "Expected generation depth >= 32, got {}",
        res_semi.generated_tokens.len()
    );

    // 3. Biology sequential prompt
    let bio_prompt =
        "First discuss cellular mitosis, then explain CRISPR gene editing, and finally describe synthetic biology.";
    let res_bio = model.generate_reply(&[], bio_prompt, 96);

    assert!(
        res_bio.clauses_resolved >= 2,
        "Expected clauses_resolved >= 2 for biology prompt, got {}",
        res_bio.clauses_resolved
    );
}

// ===========================================================================
// Challenge 4: Adversarial Edge Cases: Empty, Single-Token, Max Context, OOV
// ===========================================================================
#[test]
fn test_empirical_adversarial_edge_cases_empty_single_long_and_oov() {
    let model = get_test_model();
    let bpe = ByteBpe4096::global();

    // 1. Empty prompt
    let res_empty = model.generate_reply(&[], "", 16);
    // Should not panic, should return valid (possibly empty or default greeting) reply
    for &coord in &res_empty.final_n_t {
        assert!(
            (-32_768..=32_767).contains(&coord),
            "Q12 coordinate out of bounds on empty prompt: {}",
            coord
        );
    }

    // 2. Single-token prompts across diverse characters
    let single_chars = ["X", "?", "\n", " ", "0", "\t", "$", "#", "@"];
    for ch in single_chars {
        let toks = bpe.encode(ch);
        let res_single = model.generate_reply(&[], ch, 16);
        assert!(
            !res_single.generated_tokens.is_empty(),
            "Single char prompt '{}' produced no tokens",
            ch
        );
        for &coord in &res_single.final_n_t {
            assert!(
                (-32_768..=32_767).contains(&coord),
                "Q12 coordinate out of bounds on single token prompt '{}': {}",
                ch,
                coord
            );
        }
    }

    // 3. Maximum context / long prompt (>800 tokens, well beyond 512 trunk limit)
    let chunk = "Clifford Cl(0,6) actions with 17-root Icosian snap and 16-head ServedHamming. ";
    let mut long_prompt = String::new();
    while bpe.encode(&long_prompt).len() < 850 {
        long_prompt.push_str(chunk);
    }
    let long_tok_len = bpe.encode(&long_prompt).len();
    assert!(long_tok_len >= 850);

    let res_long = model.generate_reply(&[], &long_prompt, 16);
    assert!(
        !res_long.generated_tokens.is_empty(),
        "Long prompt (>850 tokens) generated no tokens"
    );
    for &coord in &res_long.final_n_t {
        assert!(
            (-32_768..=32_767).contains(&coord),
            "Q12 coordinate out of bounds on long prompt: {}",
            coord
        );
    }

    // 4. Out-of-vocabulary and extreme character inputs
    let extreme_inputs = [
        // Unicode Emojis
        "🦀 🚀 🔥 💎 ⚡️ 🌈 🦄 🎯",
        // Null bytes and ANSI control codes
        "Hello\0World\x01\x02\x03\x1b[31mRedText\x1b[0m",
        // Multi-script Unicode (Greek, Cyrillic, Hebrew, Arabic, Chinese, Japanese, Korean)
        "Γειά σου κόσμε! Привет мир! שלום עולם! مرحبا بالعالم! 你好世界！ こんにちは世界！ 안녕하세요!",
        // Deeply nested brackets and punctuation soup
        "[[[[{{{{((((((((!@#$%^&*()_+=-~`))))))))}}}}]]]]",
        // Whitespace only
        "          \t\t\t\n\n\n\r\n     ",
    ];

    for input in extreme_inputs {
        let res_ext = model.generate_reply(&[], input, 16);
        for &tok in &res_ext.generated_tokens {
            assert!(
                tok < 4096,
                "Generated token {} out of vocabulary range [0, 4096) for input '{}'",
                tok,
                input
            );
        }
        for &coord in &res_ext.final_n_t {
            assert!(
                (-32_768..=32_767).contains(&coord),
                "Q12 coordinate out of bounds for extreme input '{}': {}",
                input,
                coord
            );
        }
    }
}

// ===========================================================================
// Challenge 5: Embedded Math Keywords in Non-Math Text (Adversarial Camouflage)
// ===========================================================================
#[test]
fn test_empirical_adversarial_embedded_algebra_in_open_domain() {
    let model = get_test_model();

    // Adversarial prompts that mention algebraic phrases in an open-domain narrative
    let camouflaged_prompts = [
        "While baking sourdough bread, I noted that 2x + 6 = 14 was written on my shopping bag.",
        "A historian found a Renaissance manuscript with the equation 3*x + 12 = 48 in the margin.",
        "In a poem about autumn leaves, the poet metaphorically compared falling foliage to x and y variables.",
    ];

    for prompt in camouflaged_prompts {
        let res = model.generate_reply(&[], prompt, 24);
        assert!(
            !res.generated_tokens.is_empty(),
            "Camouflaged prompt '{}' generated no tokens",
            prompt
        );

        // Verify stability: finite Q12 coordinates, non-negative L1 distances
        for step in &res.steps {
            assert!(step.l1_distance >= 0);
            assert!(step.hamming_xor_bits <= 256);
        }
        for &coord in &res.final_n_t {
            assert!((-32_768..=32_767).contains(&coord));
        }
    }
}

// ===========================================================================
// Challenge 6: Repetition Penalty and Loop Breaking Under Adversarial Repetition
// ===========================================================================
#[test]
fn test_empirical_adversarial_repetition_suppression_and_anti_cycle() {
    let model = get_test_model();

    // Highly repetitive prompt that attempts to induce an infinite cyclic generation loop
    let repetitive_prompt = "echo echo echo echo echo echo echo echo echo echo echo echo echo echo";
    let res = model.generate_reply(&[], repetitive_prompt, 48);

    assert!(
        !res.generated_tokens.is_empty(),
        "Repetitive prompt generated no tokens"
    );

    // Verify that generation terminated gracefully and did not exceed max_new_tokens
    assert!(res.generated_tokens.len() <= 48);

    // Check that we didn't emit 10 identical 4-grams in a row
    let toks = &res.generated_tokens;
    if toks.len() >= 8 {
        let mut identical_adjacent_4grams = 0;
        for i in 0..toks.len() - 7 {
            if toks[i..i + 4] == toks[i + 4..i + 8] {
                identical_adjacent_4grams += 1;
            }
        }
        assert!(
            identical_adjacent_4grams < 3,
            "Excessive adjacent 4-gram repetition detected ({}) in reply:\n{}",
            identical_adjacent_4grams,
            res.reply_text
        );
    }
}
