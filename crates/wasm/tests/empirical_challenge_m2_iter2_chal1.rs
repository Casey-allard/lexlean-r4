#![allow(
    missing_docs,
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    unused_variables,
    unused_imports
)]
//! Empirical Adversarial Challenge Suite (Iteration 2) for Milestone M2:
//! Geometric Next-Token Prediction Engine.
//!
//! Objectives:
//! 1. Verify empirical challenge: `self.anchors.reply` contains NO turn-lock flags
//!    (`w & 0x8000_0000 == 0`) across varied prompts, opening entries, and generation steps.
//! 2. Verify token generation diversity: distinct prompts produce distinct continuation
//!    tokens and do not collapse into verbatim canned replays.
//! 3. Test edge conditions: short prompts (len 1), long prompts (>512 tokens), and
//!    mid-stream clause transitions.
//! 4. Invariant testing: `encode_reply_anchor_21` mappings and Q12 boundedness.

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

// ---------------------------------------------------------------------------
// Challenge 1: Verify `anchors.reply` contains NO turn-lock flags across diverse prompts
// ---------------------------------------------------------------------------
#[test]
fn test_empirical_f8_anchors_reply_never_has_turn_lock_across_massive_corpus() {
    let model = get_test_model();
    let bpe = ByteBpe4096::global();

    let test_prompts = [
        // Short / boundary prompts
        "A",
        "?",
        "1",
        "Hi",
        "Hello!",
        "What is 2 + 2?",
        "Calculate 17 * 23",
        // Multi-domain queries
        "Explain Emmy Noether's theorem and how conservation laws arise from symmetries.",
        "What is the connection between quaternions, Clifford algebra Cl(0,6), and rotations?",
        "How does sourdough bread fermentation work?",
        "What are the main differences between TCP and UDP?",
        "Define an abelian group and give two non-trivial examples.",
        "Write a function in Rust that computes the Fibonacci sequence.",
        "Explain how chloroplasts perform photosynthesis in plant cells.",
        "What is the significance of the Rosetta Stone in archaeology?",
        "Describe the cosmological constant problem in general relativity.",
        "How does zero-knowledge proof verification work in cryptography?",
        // Multi-clause and punctuation-heavy prompts
        "Step 1: initialize state; Step 2: evolve geometry; Step 3: decode token.",
        "First, define a group; second, define a ring: third, define a field!",
        "Why is the sky blue? Is it Rayleigh scattering, or is there another factor?",
        // Structured math/arithmetic prompts (triggers executive thinking)
        "Solve for x: 3*x + 12 = 48.",
        "If a train travels at 60 mph for 3 hours, how far does it go?",
    ];

    for prompt in test_prompts {
        let mut session = model.session();
        let tokens = bpe.encode(prompt);
        session.prefill_tokens(&tokens, true);

        // Check anchors right after commence_visible_prediction
        let (first_step, first_tok) = session.commence_visible_prediction();
        for &w in &session.anchors.reply {
            assert_eq!(
                w & 0x8000_0000,
                0,
                "Prompt '{}' produced turn-lock bit 31 in anchors.reply right after commence: 0x{:08X}",
                prompt,
                w
            );
        }

        // Now step for 15 tokens and verify anchors.reply at EVERY step
        let mut current_tok = first_tok;
        for step_idx in 0..15 {
            let step = session.step_with_telemetry(current_tok);
            for &w in &session.anchors.reply {
                assert_eq!(
                    w & 0x8000_0000,
                    0,
                    "Prompt '{}' step {} produced turn-lock bit 31 in anchors.reply: 0x{:08X}",
                    prompt,
                    step_idx,
                    w
                );
            }
            current_tok = step.predicted_token;
            if current_tok == bpe.eos_id() || current_tok == bpe.bos_id() {
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Challenge 2: Test opening memory entries with executive slot trace
// ---------------------------------------------------------------------------
#[test]
fn test_empirical_f8_all_opening_memory_entries_unlock() {
    let model = get_test_model();
    let mem = model.memory.as_ref().expect("model must have memory");
    let kw = mem.k;

    // Test a sample of opening table entries directly through session
    let step = (mem.opening.entries / 50).max(1);
    for e in (0..mem.opening.entries).step_by(step) {
        let open_slice = &mem.opening.keys[e * kw..(e + 1) * kw];
        let mut session = model.session();
        session.prefill_tokens(open_slice, true);

        let (_, _) = session.commence_visible_prediction();
        for &w in &session.anchors.reply {
            assert_eq!(
                w & 0x8000_0000,
                0,
                "Opening entry {} produced turn-lock in anchors.reply: 0x{:08X}",
                e,
                w
            );
        }

        // Also test with executive_slot_trace enabled
        let mut exec_session = model.session();
        exec_session.has_executive_slot_trace = true;
        exec_session.prefill_tokens(open_slice, true);
        let (_, _) = exec_session.commence_visible_prediction();
        for &w in &exec_session.anchors.reply {
            assert_eq!(
                w & 0x8000_0000,
                0,
                "Opening entry {} with exec_slot produced turn-lock in anchors.reply: 0x{:08X}",
                e,
                w
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Challenge 3: Verify token generation diversity across distinct prompts
// ---------------------------------------------------------------------------
#[test]
fn test_empirical_f8_token_generation_diversity() {
    let model = get_test_model();

    let prompt_pairs = [
        (
            "How does sourdough bread fermentation work?",
            "How does sourdough starter rise in the jar?",
        ),
        (
            "Explain the axioms of a mathematical group.",
            "Explain the axioms of a mathematical ring.",
        ),
        (
            "What is quantum entanglement in physics?",
            "What is quantum teleportation in physics?",
        ),
        (
            "Write a function to sort an array in Rust.",
            "Write a function to reverse an array in Rust.",
        ),
        (
            "What is the capital of France?",
            "What is the capital of Germany?",
        ),
        (
            "Describe cellular mitosis stages.",
            "Describe cellular meiosis stages.",
        ),
        (
            "How do compilers optimize loops?",
            "How do compilers optimize recursion?",
        ),
        (
            "What causes earthquakes in geology?",
            "What causes tsunamis in oceanography?",
        ),
        (
            "Define deterministic finite automata.",
            "Define non-deterministic finite automata.",
        ),
        (
            "Explain the Pythagorean theorem.",
            "Explain Fermat's last theorem.",
        ),
    ];

    for (p1, p2) in prompt_pairs {
        let res1 = model.generate_reply(&[], p1, 16);
        let res2 = model.generate_reply(&[], p2, 16);

        assert!(
            !res1.generated_tokens.is_empty(),
            "Prompt 1 '{}' generated no tokens",
            p1
        );
        assert!(
            !res2.generated_tokens.is_empty(),
            "Prompt 2 '{}' generated no tokens",
            p2
        );

        println!("Prompt 1: '{}' -> reply:\n{}", p1, res1.reply_text);
        println!("Prompt 2: '{}' -> reply:\n{}", p2, res2.reply_text);
        assert_ne!(
            res1.generated_tokens, res2.generated_tokens,
            "Prompt diversity failure: '{}' and '{}' produced identical continuations: {:?}",
            p1, p2, res1.generated_tokens
        );

        // Verify that neither prompt generated a canned algebraic equation problem
        assert!(
            !res1.reply_text.contains("Let $x$ be an integer")
                && !res1.reply_text.contains("Find all values of $x$"),
            "Prompt '{}' fell back into curriculum algebra verbatim paragraph replay",
            p1
        );
        assert!(
            !res2.reply_text.contains("Let $x$ be an integer")
                && !res2.reply_text.contains("Find all values of $x$"),
            "Prompt '{}' fell back into curriculum algebra verbatim paragraph replay",
            p2
        );
    }
}

// ---------------------------------------------------------------------------
// Challenge 4: Edge conditions: Length 1, Exactly 512, Long (>600 tokens)
// ---------------------------------------------------------------------------
#[test]
fn test_empirical_edge_conditions_length_1_and_long_prompts() {
    let model = get_test_model();
    let bpe = ByteBpe4096::global();

    // 1. Length 1 prompts (single tokens)
    let single_tokens = ["X", "?", " ", "9", "!"];
    for s in single_tokens {
        let toks = bpe.encode(s);
        assert_eq!(toks.len(), 1, "Expected single token for '{}'", s);

        let mut session = model.session();
        session.prefill_token(toks[0], true);
        let (first_step, first_tok) = session.commence_visible_prediction();

        for &w in &session.anchors.reply {
            assert_eq!(
                w & 0x8000_0000,
                0,
                "Length 1 prompt '{}' produced turn-lock in anchors.reply: 0x{:08X}",
                s,
                w
            );
        }

        // Coordinates bounded in Q12
        for &coord in &session.last_n_t {
            assert!(
                (-32_768..=32_767).contains(&coord),
                "Coordinate out of Q12 bounds in single-token prefill: {}",
                coord
            );
        }

        let step1 = session.step_with_telemetry(first_tok);
        assert!(step1.predicted_token < 4096);
    }

    // 2. Long prompts: >512 tokens
    let text_chunk =
        "Clifford Cl(0,6) actions with 17-root Icosian snap and 16-head ServedHamming attention. ";
    let mut long_text = String::new();
    while bpe.encode(&long_text).len() < 650 {
        long_text.push_str(text_chunk);
    }
    let long_toks = bpe.encode(&long_text);
    assert!(long_toks.len() >= 650);

    let mut session_long = model.session();
    session_long.prefill_tokens(&long_toks, true);
    let (step0, tok0) = session_long.commence_visible_prediction();

    for &w in &session_long.anchors.reply {
        assert_eq!(
            w & 0x8000_0000,
            0,
            "Long prompt produced turn-lock in anchors.reply: 0x{:08X}",
            w
        );
    }

    for &coord in &session_long.last_n_t {
        assert!(
            (-30_000..=30_000).contains(&coord),
            "Coordinate out of Q12 bounds in long prompt prefill: {}",
            coord
        );
    }

    let res = model.generate_reply(&[], &long_text, 16);
    assert!(!res.generated_tokens.is_empty());
}

// ---------------------------------------------------------------------------
// Challenge 5: Mid-stream clause transitions and pending basins
// ---------------------------------------------------------------------------
#[test]
fn test_empirical_mid_stream_clause_transitions_and_pending_basins() {
    let model = get_test_model();
    let bpe = ByteBpe4096::global();

    let multi_clause_prompt =
        "Define linear independence in vector spaces; explain the rank-nullity theorem: finally, state the spectral theorem.";

    let res = model.generate_reply(&[], multi_clause_prompt, 48);
    assert!(
        !res.generated_tokens.is_empty(),
        "Multi-clause generation produced no tokens"
    );

    // Verify all steps have valid bounded telemetry and zero floats
    for step in &res.steps {
        assert!(step.hamming_xor_bits <= 256);
        assert!(step.l1_distance >= 0);
    }

    // Manually run a session with simulated clause transition and verify anchors.reply
    let mut session = model.session();
    let toks = bpe.encode(multi_clause_prompt);
    session.prefill_tokens(&toks, true);

    let (_, mut next_tok) = session.commence_visible_prediction();
    let mut clauses_resolved = 0usize;
    let mut last_source = "point_key_vote";

    for step_idx in 0..32 {
        let step = session.step_with_telemetry(next_tok);
        for &w in &session.anchors.reply {
            assert_eq!(
                w & 0x8000_0000,
                0,
                "anchors.reply contained turn-lock bit at step {}: 0x{:08X}",
                step_idx,
                w
            );
        }

        // Test mid-stream re-attention
        model.try_mid_stream_re_attention(
            &mut session,
            step_idx,
            next_tok as usize,
            &mut next_tok,
            &mut last_source,
            &mut clauses_resolved,
        );

        for &w in &session.anchors.reply {
            assert_eq!(
                w & 0x8000_0000,
                0,
                "anchors.reply contained turn-lock bit after try_mid_stream_re_attention at step {}: 0x{:08X}",
                step_idx,
                w
            );
        }

        next_tok = step.predicted_token;
        if next_tok == bpe.eos_id() || next_tok == bpe.bos_id() {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Challenge 6: Invariant verification on `encode_reply_anchor_21`
// ---------------------------------------------------------------------------
#[test]
fn test_empirical_encode_reply_anchor_21_invariants() {
    // 1. Special anchors
    assert_eq!(encode_reply_anchor_21(GLOBAL_SUFFIX_ANCHOR), 0x1F_FFFF);
    assert_eq!(encode_reply_anchor_21(MEMORY_PAD), 0x1F_FFFE);

    // 2. Normal anchor IDs (0..=0x1E_FFFF) with and without high-bit turn lock
    for id in [0u32, 1, 42, 1000, 0x10_0000, 0x1E_FFFF] {
        let sanitized = encode_reply_anchor_21(id);
        let locked = encode_reply_anchor_21(id | 0x8000_0000);
        assert_eq!(
            sanitized, id,
            "Sanitized ID 0x{:08X} mapped incorrectly to 0x{:08X}",
            id, sanitized
        );
        assert_eq!(
            locked,
            id,
            "Locked ID 0x{:08X} did not match sanitized ID 0x{:08X}",
            id | 0x8000_0000,
            locked
        );
    }

    // 3. Out-of-range IDs capped at 0x1E_FFFF
    assert_eq!(encode_reply_anchor_21(0x1F_0000), 0x1E_FFFF);
    assert_eq!(encode_reply_anchor_21(0x1F_AAAA), 0x1E_FFFF);
}
