#![allow(
    missing_docs,
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    unused_variables,
    unused_imports
)]
//! Empirical Adversarial Challenge Suite 1 for Milestone M3:
//! Eliminate Interceptors & Native Routing.
//!
//! Stress-tests:
//! 1. Probe for leftover bypasses:
//!    Verifies keywords like `teach:`, arithmetic expressions, `/status`, `who are you`,
//!    capital cities, coding templates, B-Trees, and AGI questions do NOT trigger canned responses
//!    and all route through `LutStackSession`.
//! 2. Probe telemetry authenticity:
//!    Verifies telemetry spans have monotonically increasing token positions and step counts equal
//!    to emitted tokens (never hardcoded 128-token fake traces).
//! 3. Test prose generation diversity:
//!    Verifies `generate_prose_json` does not fall back to stored paragraphs via GCD or stem sorting,
//!    and handles zero stem overlap prompts gracefully.

use lexlean_r4_wasm::lut_stack_exec::{LutStackModel, LutStackSession};
use lexlean_r4_wasm::StudioEngine;
use std::sync::{Arc, OnceLock};

pub const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

pub fn get_test_model() -> Arc<LutStackModel> {
    static MODEL: OnceLock<Arc<LutStackModel>> = OnceLock::new();
    Arc::clone(MODEL.get_or_init(|| {
        Arc::new(LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully"))
    }))
}

// ---------------------------------------------------------------------------
// Objective 1: Probe for leftover bypasses and canned responses
// ---------------------------------------------------------------------------
#[test]
fn test_adversarial_probe_leftover_bypasses() {
    let mut engine = StudioEngine::new_default();

    let test_cases = [
        // 1. teach: prefix
        (
            "Teach: The speed of light is 300,000 km/s => c = 3e8 m/s",
            &[
                "Admitted new knowledge turn into",
                "Learned Response",
                "Vocabulary Size",
                "Try asking",
            ][..],
            "teach: prefix must not trigger canned admission text",
        ),
        (
            "teach: 2+2 => 4",
            &["Admitted new knowledge turn into", "Learned Response"][..],
            "teach: lowercase prefix must not trigger canned admission text",
        ),
        // 2. Exact arithmetic expressions (must not trigger assistant.rs M6 canned interceptor)
        (
            "What is 2 + 2?",
            &[
                "Computed via **UOR-R4 M6 Exact Place-Value Arithmetic",
                "M6 FullExact Radix-10 Carry Trace",
                "PRLL Lawful Likelihood Mass",
            ][..],
            "arithmetic 2+2 must not trigger assistant.rs canned M6 calculator response",
        ),
        (
            "Calculate 45 * 12",
            &[
                "Computed via **UOR-R4 M6 Exact Place-Value Arithmetic",
                "M6 FullExact Radix-10 Carry Trace",
                "PRLL Lawful Likelihood Mass",
            ][..],
            "arithmetic multiplication must not trigger assistant.rs canned M6 calculator response",
        ),
        (
            "solve 100 - 37",
            &[
                "Computed via **UOR-R4 M6 Exact Place-Value Arithmetic",
                "M6 FullExact Radix-10 Carry Trace",
                "PRLL Lawful Likelihood Mass",
            ][..],
            "arithmetic subtraction must not trigger assistant.rs canned M6 calculator response",
        ),
        (
            "What is 2^10?",
            &[
                "Computed via **UOR-R4 M6 Exact Place-Value Arithmetic",
                "M6 FullExact Radix-10 Carry Trace",
                "PRLL Lawful Likelihood Mass",
            ][..],
            "arithmetic power must not trigger assistant.rs canned M6 calculator response",
        ),
        // 3. /status and status
        (
            "/status",
            &[
                "All **27 LexLean/PrismPM conformance suites",
                "occupied E8 product cells",
            ][..],
            "/status must not trigger canned conformance string",
        ),
        (
            "status",
            &[
                "All **27 LexLean/PrismPM conformance suites",
                "occupied E8 product cells",
            ][..],
            "status must not trigger canned conformance string",
        ),
        // 4. Identity queries (who are you, who am i, what is my name)
        (
            "who are you",
            &[
                "I am the LexLean/PrismPM Studio Assistant",
                "GLM_CONCEPT_MANIFOLD",
            ][..],
            "who are you must not trigger canned assistant identity",
        ),
        (
            "who am i",
            &[
                "You haven't told me your name yet",
                "stored in the active `R4.Binder`",
            ][..],
            "who am i must not trigger canned R4.Binder name notice",
        ),
        (
            "what is my name",
            &[
                "You haven't told me your name yet",
                "stored in the active `R4.Binder`",
            ][..],
            "what is my name must not trigger canned R4.Binder name notice",
        ),
        (
            "what did i tell you",
            &["We just started this session!", "Tell me *\"Remember that"][..],
            "session recall must not trigger canned intro string",
        ),
        (
            "conversation history",
            &["So far we have exchanged", "Active Geometric Memory"][..],
            "conversation history must not trigger canned history template",
        ),
        // 5. Capital cities
        (
            "What is the capital of France?",
            &["Paris is the capital of France.", "### Capital of France"][..],
            "capital city query must not trigger static knowledge table",
        ),
        (
            "What is the capital city of Japan?",
            &["Tokyo is the capital", "### Capital of Japan"][..],
            "capital city query must not trigger static knowledge table",
        ),
        // 6. Coding templates and AST synthesis
        (
            "write a function in rust to reverse a string",
            &[
                "[R4-32 Self-Repair Trace]",
                "Live In-Wasm AST Execution Proof (`R4-32`)",
            ][..],
            "coding prompt must not trigger hardcoded self-repair trace template",
        ),
        (
            "pub fn max_paren_depth(input: &str) -> i64",
            &[
                "[R4-32 Self-Repair Trace] Verified on attempt #2",
                "delimiter_kv_sum rejected",
            ][..],
            "paren depth coding query must not trigger canned AST repair string",
        ),
        // 7. B-Trees
        (
            "explain B-tree balance invariant",
            &["GLM_CONCEPT_MANIFOLD"][..],
            "B-tree explanation must not trigger static GLM concept dump",
        ),
        (
            "B-tree node split algorithm",
            &["GLM_CONCEPT_MANIFOLD"][..],
            "B-tree split query must not trigger static GLM concept dump",
        ),
        // 8. AGI and open-domain reasoning
        (
            "when will AGI arrive?",
            &[
                "GLM_CONCEPT_MANIFOLD",
                "I am the **UOR-R4 Generalized Geometric Assistant",
            ][..],
            "AGI question must not trigger static concept node paragraph or GLM QA",
        ),
        (
            "Are you an AGI?",
            &[
                "GLM_CONCEPT_MANIFOLD",
                "I am the **UOR-R4 Generalized Geometric Assistant",
            ][..],
            "AGI identity question must not trigger static concept node paragraph or GLM QA",
        ),
    ];

    for (prompt, forbidden_phrases, msg) in test_cases {
        let res_json = engine.chat_assistant_json(prompt);
        let val: serde_json::Value = serde_json::from_str(&res_json)
            .unwrap_or_else(|e| panic!("invalid JSON for '{prompt}': {e}"));
        let reply = val["reply"]
            .as_str()
            .or_else(|| val["assistant_reply"].as_str())
            .unwrap_or("");

        if prompt == "What is 2 + 2?" {
            assert_ne!(
                reply.trim(),
                "2 + 2 = 4",
                "Exact arithmetic query returned canned arithmetic string '2 + 2 = 4'"
            );
        }

        for &forbidden in forbidden_phrases {
            assert!(
                !reply.contains(forbidden),
                "Adversarial violation for prompt '{prompt}': response contains '{forbidden}'. Detail: {msg}. Got reply: '{reply}'"
            );
        }

        // Verify routing through LutStackSession by inspecting geometric telemetry in JSON
        let geom_thinking = val["geometric_thinking"].as_str().unwrap_or("");
        assert!(
            geom_thinking.contains("PointKey") || geom_thinking.contains("uor.gllm.trace"),
            "Prompt '{prompt}' failed to produce authentic geometric thinking in chat reply"
        );
        let span_count = val["otel_span_count"].as_u64().unwrap_or(0);
        assert!(span_count > 0, "Prompt '{prompt}' produced 0 otel spans");
    }
}

// ---------------------------------------------------------------------------
// Objective 2: Probe telemetry authenticity
// ---------------------------------------------------------------------------
#[test]
fn test_adversarial_probe_telemetry_authenticity() {
    let model = get_test_model();
    let mut engine = StudioEngine::new_default();

    let test_prompts = [
        ("Hello", 4),
        ("Describe the structure of an atom in quantum physics", 16),
        (
            "Write a short poetic phrase about mountains and autumn wind",
            24,
        ),
        ("What are the conservation laws in physics?", 12),
    ];

    for (prompt, budget) in test_prompts {
        // Test direct LutStackModel generation
        let gen = model.generate_reply(&[], prompt, budget);
        let token_count = gen.generated_tokens.len();
        let step_count = gen.steps.len();

        // 1. Step count must match emitted tokens (+0 or +1 for completion step)
        assert!(
            step_count == token_count || step_count == token_count + 1,
            "For prompt '{prompt}': step count ({step_count}) must equal emitted token count ({token_count})"
        );

        // 2. Never a hardcoded fixed 128-token fake trace
        if token_count < 100 {
            assert_ne!(
                step_count, 128,
                "For prompt '{prompt}': small token generation produced fake 128-token trace!"
            );
        }

        // 3. Monotonically increasing token positions
        for window in gen.steps.windows(2) {
            assert!(
                window[0].position < window[1].position,
                "For prompt '{prompt}': step positions must increase strictly monotonically, got {} then {}",
                window[0].position,
                window[1].position
            );
        }

        // 4. Each step's predicted_token matches the emitted token in generated_tokens
        for (idx, step) in gen.steps.iter().enumerate() {
            if idx < token_count {
                assert_eq!(
                    step.predicted_token, gen.generated_tokens[idx],
                    "For prompt '{prompt}': step predicted token must match emitted token at index {idx}"
                );
            }
        }

        // 5. Authentic geometric metrics in telemetry
        for step in &gen.steps {
            assert!(
                step.hamming_xor_bits <= 256,
                "Hamming distance must be <= 256 bits, got {}",
                step.hamming_xor_bits
            );
        }

        // Test StudioEngine chat_assistant_json telemetry integration
        let res_json = engine.chat_assistant_json(prompt);
        let val: serde_json::Value =
            serde_json::from_str(&res_json).expect("chat_assistant_json must produce valid JSON");

        let otel_span_count = val["otel_span_count"].as_u64().unwrap_or(0);
        assert!(
            otel_span_count > 0,
            "For prompt '{prompt}': otel_span_count must be > 0"
        );
        if budget < 50 {
            assert_ne!(
                otel_span_count, 128,
                "For prompt '{prompt}': otel_span_count must NOT be fixed fake 128"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Objective 3: Test prose generation diversity
// ---------------------------------------------------------------------------
#[test]
fn test_adversarial_probe_prose_generation_diversity() {
    let mut engine = StudioEngine::new_default();

    // 1. Zero stem overlap prompt must not abort or error
    let zero_overlap_json = engine.generate_prose_json("Zanzibar xylophone quokka labyrinth");
    let zero_val: serde_json::Value =
        serde_json::from_str(&zero_overlap_json).expect("valid JSON for zero-overlap prose");
    assert_ne!(
        zero_val["status"], "error",
        "Zero stem overlap must not return error status"
    );
    let zero_text = zero_val["generated_text"]
        .as_str()
        .or_else(|| zero_val["reply"].as_str())
        .unwrap_or("");
    assert!(
        !zero_text.is_empty(),
        "Zero stem overlap must produce non-empty generated text"
    );

    // 2. Multiple distinct prompts must produce distinct texts and distinct trajectories
    let prompts = [
        "The lighthouse keeper watched the stormy waves crashing upon the shore.",
        "A quantum harmonic oscillator exhibits discrete energy levels proportional to Planck's constant.",
        "Ancient philosophers debated whether space was continuous or composed of indivisible atoms.",
    ];

    let mut outputs = Vec::new();

    for &p in &prompts {
        let p_json = engine.generate_prose_json(p);
        let val: serde_json::Value =
            serde_json::from_str(&p_json).unwrap_or_else(|e| panic!("invalid JSON for '{p}': {e}"));

        assert_ne!(
            val["status"], "error",
            "Prose generation returned error for '{p}'"
        );

        let gen_text = val["generated_text"]
            .as_str()
            .or_else(|| val["reply"].as_str())
            .unwrap_or("")
            .to_string();

        assert!(
            !gen_text.is_empty(),
            "Generated text must not be empty for '{p}'"
        );

        // Verify no static GLM manifold marker or GCD shared prime replay
        assert!(
            !gen_text.contains("GLM_CONCEPT_MANIFOLD"),
            "Prose must not contain GLM_CONCEPT_MANIFOLD"
        );
        assert!(
            !p_json.contains("gcd_shared_primes_replay"),
            "Prose must not contain GCD prime replay markers"
        );

        outputs.push((p, gen_text, val));
    }

    // Pairwise diversity: no two distinct prompts should return identical text
    for i in 0..outputs.len() {
        for j in (i + 1)..outputs.len() {
            assert_ne!(
                outputs[i].1, outputs[j].1,
                "Prose outputs for prompt '{}' and prompt '{}' must not be identical verbatim replays!",
                outputs[i].0, outputs[j].0
            );
        }
    }
}
