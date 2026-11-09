#![allow(
    missing_docs,
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    unused_variables,
    unused_imports
)]
//! Empirical Adversarial Challenge Suite (Iteration 4 - Challenger 2) for Milestone M2:
//! Geometric Next-Token Prediction Engine.
//!
//! Objectives:
//! 1. Verify fine-grained prompt diversity (Pair 9: Pythagorean vs Fermat, biological mitosis vs meiosis,
//!    group vs ring, quantum entanglement vs superposition, sourdough vs coffee, French Revolution vs Renaissance)
//!    with strict non-identical continuations.
//! 2. Verify anti-algebra hijacking under adversarial prompts with math-like symbols, variables, formulas, or numbers.
//! 3. Verify multi-clause generation depth (>=32 tokens, >=2 clauses resolved) on multi-domain prompts.
//! 4. Verify multi-turn session state isolation: Turn 2 (Burj Khalifa) -> Turn 3 (Sourdough) -> Turn 4 (Mitosis)
//!    ensuring zero leakage, no suffix collapse, and no algebra hijacking.
//! 5. Verify geometric invariants: Q12 coordinate clamping [-32768, 32767], bit 31 unanchored, and valid L1/Hamming telemetry.

use lexlean_r4_wasm::lut_stack_exec::{
    ByteBpe4096, LutStackModel, LutStackSession, RollingContextState, FBS_WIDTH,
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
// Challenge 1: Fine-Grained Prompt Diversity & Semantic Discrimination
// ===========================================================================
#[test]
fn test_empirical_fine_grained_prompt_diversity() {
    let model = get_test_model();

    let prompt_pairs = [
        // 1. Biology: Mitosis vs Meiosis (Worker Iteration 4 remediation verification)
        (
            "Describe cellular mitosis stages.",
            "Describe cellular meiosis stages.",
        ),
        // 2. Biology: Photosynthesis vs Respiration
        (
            "Explain how photosynthesis captures solar energy in thylakoid membranes.",
            "Explain how cellular respiration produces ATP in mitochondria.",
        ),
        // 3. Mathematics: Group vs Ring
        (
            "Explain the axioms of a mathematical group.",
            "Explain the axioms of a mathematical ring.",
        ),
        // 4. Mathematics: Pythagorean vs Fermat (Pair 9 from Iteration 2 challenge)
        (
            "Explain the Pythagorean theorem.",
            "Explain Fermat's last theorem.",
        ),
        // 5. Physics: Entanglement vs Teleportation
        (
            "What is quantum entanglement in physics?",
            "What is quantum teleportation in physics?",
        ),
        // 6. Computer Science: Sort vs Reverse
        (
            "Write a function to sort an array in Rust.",
            "Write a function to reverse an array in Rust.",
        ),
        // 7. Computer Science: DFA vs NFA
        (
            "Define deterministic finite automata.",
            "Define non-deterministic finite automata.",
        ),
        // 8. Geography: Paris vs Berlin
        (
            "What is the capital of France?",
            "What is the capital of Germany?",
        ),
        // 9. Geology vs Oceanography: Earthquakes vs Tsunamis
        (
            "What causes earthquakes in geology?",
            "What causes tsunamis in oceanography?",
        ),
        // 10. Open-domain: Sourdough vs Pour-over Coffee
        (
            "Explain the process of making sourdough bread from scratch.",
            "How to brew pour-over coffee using a V60 ceramic dripper and fresh beans?",
        ),
        // 11. History: French Revolution vs Italian Renaissance
        (
            "What were the primary socio-economic causes of the French Revolution?",
            "Describe the cultural and political impact of the Renaissance in Florence.",
        ),
        // 12. Ethology vs Game Theory: Honeybees vs Chess
        (
            "How do honeybees communicate the location of flowers using the waggle dance?",
            "What are the rules, movements, and pieces in the classical game of chess?",
        ),
        // 13. Deep Physics: Noether's Theorem vs Clifford Algebra
        (
            "Explain Emmy Noether's theorem and how conservation laws arise from symmetries.",
            "What is the connection between quaternions, Clifford algebra Cl(0,6), and rotations?",
        ),
    ];

    for (idx, (p1, p2)) in prompt_pairs.iter().enumerate() {
        let res1 = model.generate_reply(&[], p1, 24);
        let res2 = model.generate_reply(&[], p2, 24);

        assert!(
            !res1.generated_tokens.is_empty(),
            "Pair {}: Prompt 1 '{}' generated no tokens",
            idx + 1,
            p1
        );
        assert!(
            !res2.generated_tokens.is_empty(),
            "Pair {}: Prompt 2 '{}' generated no tokens",
            idx + 1,
            p2
        );

        println!(
            "Pair {} [len: {} vs {}]:\n  P1 '{}' -> {}\n  P2 '{}' -> {}",
            idx + 1,
            res1.generated_tokens.len(),
            res2.generated_tokens.len(),
            p1,
            res1.reply_text,
            p2,
            res2.reply_text
        );

        assert_ne!(
            res1.generated_tokens, res2.generated_tokens,
            "Prompt diversity failure on Pair {}: '{}' and '{}' produced identical continuations: {:?}",
            idx + 1,
            p1,
            p2,
            res1.generated_tokens
        );

        // Neither prompt should collapse to algebra replay
        assert!(
            !res1.reply_text.contains("2x + 6")
                && !res1.reply_text.contains("Let $x$ be an integer")
                && !res1.reply_text.contains("Find all values of $x$"),
            "Pair {}: Prompt 1 '{}' was hijacked into algebra:\n{}",
            idx + 1,
            p1,
            res1.reply_text
        );
        assert!(
            !res2.reply_text.contains("2x + 6")
                && !res2.reply_text.contains("Let $x$ be an integer")
                && !res2.reply_text.contains("Find all values of $x$"),
            "Pair {}: Prompt 2 '{}' was hijacked into algebra:\n{}",
            idx + 1,
            p2,
            res2.reply_text
        );
    }
}

// ===========================================================================
// Challenge 2: Adversarial Math-like Symbols / Numbers without Algebra Hijacking
// ===========================================================================
#[test]
fn test_empirical_adversarial_math_like_symbols_no_hijacking() {
    let model = get_test_model();

    let adversarial_prompts = [
        "In physics, what does E = mc^2 mean for mass-energy equivalence?",
        "Explain the recipe for baking sourdough bread with 500g flour and 350g water.",
        "Why does Shakespeare use iambic pentameter in Sonnet 18?",
        "How many players are on a soccer team: 11 vs 11?",
        "Explain the 80/20 rule (Pareto principle) in economics.",
        "What is the chemical formula H2O + CO2 in carbonated water?",
        "In Python, what does the expression `x = [i**2 for i in range(10)]` compute?",
        "Describe the Apollo 11 moon landing in 1969.",
        "Calculate the value of 17 * 23 in mental arithmetic.",
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
        "linear equation 2x",
    ];

    for prompt in adversarial_prompts {
        let res = model.generate_reply(&[], prompt, 32);

        assert!(
            !res.generated_tokens.is_empty(),
            "Adversarial prompt '{}' generated no tokens",
            prompt
        );

        for fragment in &forbidden_algebra_fragments {
            assert!(
                !res.reply_text.contains(fragment),
                "Adversarial prompt '{}' hijacked into algebra fragment '{}'. Reply was:\n{}",
                prompt,
                fragment,
                res.reply_text
            );
        }
    }
}

// ===========================================================================
// Challenge 3: Multi-Clause Compound Queries Generation Depth
// ===========================================================================
#[test]
fn test_empirical_multi_clause_compound_queries_depth() {
    let model = get_test_model();

    let test_cases = [
        (
            "Explain quantum entanglement, Emmy Noether, and relativistic GPS time dilation.",
            64,
            2,
        ),
        (
            "First, define a topological manifold; second, explain Stokes' theorem: third, state the Poincare conjecture.",
            32,
            2,
        ),
        (
            "First discuss cellular mitosis, then explain CRISPR gene editing, and finally describe synthetic biology.",
            32,
            2,
        ),
        (
            "How do honeybees communicate using the waggle dance? Also how do computers execute instructions?",
            32,
            2,
        ),
        (
            "Explain how sourdough bread rises and explain how steam engines generate mechanical power.",
            32,
            2,
        ),
    ];

    for (prompt, min_depth, min_clauses) in test_cases {
        let res = model.generate_reply(&[], prompt, 128);

        assert!(
            res.generated_tokens.len() >= min_depth,
            "Prompt '{}' generated depth {} < min_depth {}",
            prompt,
            res.generated_tokens.len(),
            min_depth
        );
        assert!(
            res.clauses_resolved >= min_clauses,
            "Prompt '{}' clauses_resolved {} < min_clauses {}",
            prompt,
            res.clauses_resolved,
            min_clauses
        );

        // Verify telemetry invariants
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
// Challenge 4: Multi-Turn Conversation State Isolation & Suffix Independence
// ===========================================================================
#[test]
fn test_empirical_multi_turn_state_isolation_and_no_suffix_leakage() {
    let model = get_test_model();
    let mut context = RollingContextState::default();

    // Turn 1: Greeting
    let res1 = model.generate_reply_with_context(
        &mut context,
        &[],
        "Hello, can you help me explore science and history?",
        32,
    );
    assert!(!res1.generated_tokens.is_empty(), "Turn 1 reply empty");
    let t1_reply = res1.reply_text.clone();

    let history1 = vec![(
        "Hello, can you help me explore science and history?".to_string(),
        t1_reply.clone(),
    )];

    // Turn 2: Burj Khalifa in Dubai
    let res2 = model.generate_reply_with_context(
        &mut context,
        &history1,
        "Tell me about the architecture and height of the Burj Khalifa in Dubai.",
        64,
    );
    assert!(
        res2.generated_tokens.len() >= 16,
        "Turn 2 generated too few tokens: {}",
        res2.generated_tokens.len()
    );
    let t2_reply = res2.reply_text.clone();

    let mut history2 = history1;
    history2.push((
        "Tell me about the architecture and height of the Burj Khalifa in Dubai.".to_string(),
        t2_reply.clone(),
    ));

    // Turn 3: Sourdough bread fermentation (MUST NOT collapse, MUST NOT leak Burj Khalifa or algebra)
    let res3 = model.generate_reply_with_context(
        &mut context,
        &history2,
        "Explain the process of making sourdough bread from scratch.",
        64,
    );
    println!(
        "Multi-turn Turn 3 generated tokens: {}",
        res3.generated_tokens.len()
    );
    println!("Multi-turn Turn 3 reply:\n{}", res3.reply_text);

    assert!(
        res3.generated_tokens.len() >= 20,
        "Multi-turn Turn 3 collapsed to {} tokens (expected >= 20). Reply was:\n{}",
        res3.generated_tokens.len(),
        res3.reply_text
    );
    assert!(
        !res3.reply_text.to_lowercase().contains("burj khalifa"),
        "Multi-turn Turn 3 leaked Turn 2 Burj Khalifa text:\n{}",
        res3.reply_text
    );
    assert!(
        !res3.reply_text.contains("2x + 6")
            && !res3.reply_text.contains("Let $x$ be an integer")
            && !res3.reply_text.contains("Find all values of $x$"),
        "Multi-turn Turn 3 was hijacked into algebra:\n{}",
        res3.reply_text
    );

    let mut history3 = history2;
    history3.push((
        "Explain the process of making sourdough bread from scratch.".to_string(),
        res3.reply_text.clone(),
    ));

    // Turn 4: Cellular mitosis (MUST NOT leak sourdough, Burj Khalifa, or algebra)
    let res4 = model.generate_reply_with_context(
        &mut context,
        &history3,
        "Describe the biological stages of cellular mitosis.",
        64,
    );
    println!(
        "Multi-turn Turn 4 generated tokens: {}",
        res4.generated_tokens.len()
    );
    println!("Multi-turn Turn 4 reply:\n{}", res4.reply_text);

    assert!(
        res4.generated_tokens.len() >= 20,
        "Multi-turn Turn 4 collapsed to {} tokens (expected >= 20). Reply was:\n{}",
        res4.generated_tokens.len(),
        res4.reply_text
    );
    assert!(
        !res4.reply_text.to_lowercase().contains("burj khalifa"),
        "Multi-turn Turn 4 leaked Burj Khalifa text:\n{}",
        res4.reply_text
    );
    assert!(
        !res4.reply_text.to_lowercase().contains("sourdough")
            && !res4.reply_text.to_lowercase().contains("fermentation"),
        "Multi-turn Turn 4 leaked Turn 3 sourdough text:\n{}",
        res4.reply_text
    );
    assert!(
        !res4.reply_text.contains("2x + 6") && !res4.reply_text.contains("Let $x$ be an integer"),
        "Multi-turn Turn 4 was hijacked into algebra:\n{}",
        res4.reply_text
    );
}

// ===========================================================================
// Challenge 5: Next-Token Invariants & Geometric Bounds
// ===========================================================================
#[test]
fn test_empirical_geometric_invariants_and_telemetry_bounds() {
    let model = get_test_model();
    let bpe = ByteBpe4096::global();

    let test_prompts = [
        "Explain Emmy Noether's theorem and how conservation laws arise from symmetries.",
        "Calculate 17 * 23.",
        "What are the rules of classical chess?",
        "Explain the water cycle: evaporation, condensation, precipitation.",
    ];

    for prompt in test_prompts {
        let mut session = model.session();
        let prompt_tokens = bpe.encode(prompt);
        session.prefill_tokens(&prompt_tokens, true);

        let (first_step, first_tok) = session.commence_visible_prediction();

        // 1. Check bit 31 unanchored in reply anchors
        for &w in &session.anchors.reply {
            assert_eq!(
                w & 0x8000_0000,
                0,
                "Prompt '{}' left bit 31 set in session.anchors.reply: 0x{:08X}",
                prompt,
                w
            );
        }

        // 2. Step through 16 tokens and verify telemetry invariants
        let mut cur_tok = first_tok;
        for s in 0..16 {
            let st = session.step_with_telemetry(cur_tok);
            assert!(
                st.l1_distance >= 0,
                "Prompt '{}' step {}: negative L1 distance {}",
                prompt,
                s,
                st.l1_distance
            );
            assert!(
                st.hamming_xor_bits <= 256,
                "Prompt '{}' step {}: Hamming XOR bits exceeded 256: {}",
                prompt,
                s,
                st.hamming_xor_bits
            );

            for &coord in &session.last_n_t {
                assert!(
                    (-32_768..=32_767).contains(&coord),
                    "Q12 coordinate out of range at step {}: {}",
                    s,
                    coord
                );
            }

            cur_tok = st.predicted_token;
            if cur_tok == bpe.eos_id() || cur_tok == bpe.bos_id() {
                break;
            }
        }
    }
}
