#![allow(
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    clippy::identity_op,
    clippy::manual_range_contains,
    clippy::useless_vec
)]
//! Milestone 4 Adversarial Empirical Stress Test Suite
//! Target: Token Generation Dynamics & Quantum Entanglement Prompt Budget Stress
//!
//! Investigates:
//! 1. Exact token threshold for "quantum entanglement" emission.
//! 2. Edge-case budget handling: 0, 1, 2, 256, 512, 1024, 2048, 100_000.
//! 3. Stability and absence of panics under empty, massive, and adversarial prompts.

use lexlean_r4_wasm::lut_stack_exec::LutStackModel;
use std::sync::Arc;

const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

fn get_model() -> Arc<LutStackModel> {
    static MODEL: std::sync::OnceLock<Arc<LutStackModel>> = std::sync::OnceLock::new();
    MODEL
        .get_or_init(|| {
            Arc::new(LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully"))
        })
        .clone()
}

#[test]
fn test_quantum_entanglement_token_budget_threshold() {
    let prompt = "Can you walk me through how quantum entanglement works for a beginner?";
    let raw_model = get_model();
    let backend_model = lexlean_r4_wasm::api::default_model_backend();

    for (name, model) in [
        ("raw_model", raw_model.clone()),
        ("backend_model", backend_model.clone()),
    ] {
        println!("=== Testing with {} ===", name);
        let gen = model.generate_reply(&[], prompt, 512);
        println!(
            "{} reply len: {}, tokens: {}",
            name,
            gen.reply_text.len(),
            gen.generated_tokens.len()
        );
        println!(
            "{} contains 'quantum entanglement': {}",
            name,
            gen.reply_text
                .to_ascii_lowercase()
                .contains("quantum entanglement")
        );
        println!("{} full reply:\n{}", name, gen.reply_text);
    }

    let budgets = [
        0, 1, 10, 50, 100, 150, 200, 250, 256, 260, 280, 300, 350, 400, 512, 1024,
    ];
    let mut first_quantum_budget: Option<usize> = None;

    println!("=== Sweeping budgets on backend_model ===");
    for &budget in &budgets {
        let gen = backend_model.generate_reply(&[], prompt, budget);
        let lower = gen.reply_text.to_ascii_lowercase();
        let contains_quantum = lower.contains("quantum") && lower.contains("entanglement");
        if contains_quantum && first_quantum_budget.is_none() {
            first_quantum_budget = Some(budget);
        }
        println!(
            "Budget {:4}: tokens={:4}, contains_quantum={}, text_preview={:?}",
            budget,
            gen.generated_tokens.len(),
            contains_quantum,
            if gen.reply_text.len() > 60 {
                &gen.reply_text[..60]
            } else {
                &gen.reply_text
            }
        );
    }

    println!(
        "First budget reaching 'quantum entanglement': {:?}",
        first_quantum_budget
    );
    assert!(
        first_quantum_budget.is_some(),
        "Quantum entanglement must be reachable"
    );
}

#[test]
fn test_adversarial_prompt_budgets_no_panics() {
    let model = get_model();

    let adversarial_budgets = [0, 1, 2, 3, usize::MAX / 2, 100_000];
    let massive_prompt = "A".repeat(5000);
    let test_prompts = [
        "", // empty prompt
        "   \n\t  ", // whitespace only
        "What are you able to do?",
        "Explain quantum entanglement in clear terms and also tell me about Emmy Noether.",
        "🚀 Quantum ⚛️ Entanglement 🌟 Multi-byte UTF8 test with emojis & special chars: \u{0000}\u{FFFF}",
        massive_prompt.as_str(), // massive prompt
    ];

    for &prompt in &test_prompts {
        for &budget in &adversarial_budgets {
            let gen = model.generate_reply(&[], prompt, budget);
            // Must never panic and must produce bounded step length
            assert!(gen.generated_tokens.len() <= budget.clamp(1, 1024));
            assert!(gen.steps.len() <= gen.generated_tokens.len() + 1);
        }
    }
}

#[test]
fn test_multi_turn_history_budget_stress() {
    let model = lexlean_r4_wasm::api::default_model_backend();

    // Construct a 10-turn history
    let mut history = Vec::new();
    for i in 1..=10 {
        history.push((
            format!("User turn {i}: What is step {i}?"),
            format!("Assistant turn {i}: Step {i} involves quantum coordinate operations."),
        ));
    }

    for budget in [1, 5, 16, 64, 256, 512, 1024] {
        let gen = model.generate_reply(
            &history,
            "Can you walk me through how quantum entanglement works for a beginner?",
            budget,
        );
        println!(
            "Multi-turn budget {}: tokens={}, reply_len={}, text={:?}",
            budget,
            gen.generated_tokens.len(),
            gen.reply_text.len(),
            gen.reply_text
        );
    }
}
