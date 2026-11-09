#![allow(
    missing_docs,
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    unused_variables,
    unused_imports
)]
//! Empirical Adversarial Challenge Suite (Iteration 3 - Challenger 1) for Milestone M2.
//!
//! Objectives:
//! 1. Verify multi-turn conversational token generation stability.
//! 2. Reproduce the multi-turn generation collapse observed in
//!    `test_generalized_glm_multi_domain_reasoning_and_generation` (Section 13).
//! 3. Verify single-turn vs multi-turn session behavior on open-domain queries.

use lexlean_r4_wasm::lut_stack_exec::{ByteBpe4096, LutStackModel};
use std::sync::{Arc, Mutex, OnceLock};

static STUDIO_DISPATCH_LOCK: Mutex<()> = Mutex::new(());

pub const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

pub fn get_test_model() -> Arc<LutStackModel> {
    static MODEL: OnceLock<Arc<LutStackModel>> = OnceLock::new();
    Arc::clone(MODEL.get_or_init(|| {
        Arc::new(LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully"))
    }))
}

#[test]
fn test_empirical_single_turn_open_domain_sourdough_token_count() {
    let _guard = STUDIO_DISPATCH_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    // In a fresh session without multi-turn history, open-domain query generates plenty of tokens
    let raw = lexlean_r4_wasm::dispatch_string("1\tstudio\treset\t");
    let chat_raw = lexlean_r4_wasm::dispatch_string(
        "1\tstudio\tchat\tHow does sourdough bread fermentation work?",
    );
    let json: serde_json::Value = serde_json::from_str(&chat_raw).unwrap();
    let count = json["generation"]["generated_token_count"]
        .as_u64()
        .unwrap();
    assert!(
        count >= 20,
        "Fresh single-turn sourdough query generated fewer than 20 tokens: count={}",
        count
    );
}

#[test]
fn test_empirical_multiturn_chat_token_collapse_reproduction() {
    let _guard = STUDIO_DISPATCH_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    // Reset session
    let _ = lexlean_r4_wasm::dispatch_string("1\tstudio\treset\t");

    // Turn 1
    let t1_raw =
        lexlean_r4_wasm::dispatch_string("1\tstudio\tchat\tWhat is the capital of France?");
    let j1: serde_json::Value = serde_json::from_str(&t1_raw).unwrap();
    let c1 = j1["generation"]["generated_token_count"].as_u64().unwrap();
    assert!(c1 >= 10, "Turn 1 generated {} tokens", c1);

    // Turn 2
    let t2_raw =
        lexlean_r4_wasm::dispatch_string("1\tstudio\tchat\tWhat is the architecture like there?");
    let j2: serde_json::Value = serde_json::from_str(&t2_raw).unwrap();
    let c2 = j2["generation"]["generated_token_count"].as_u64().unwrap();
    assert!(c2 >= 10, "Turn 2 generated {} tokens", c2);

    // Turn 3: Open-domain query following multi-turn history
    let t3_raw = lexlean_r4_wasm::dispatch_string(
        "1\tstudio\tchat\tHow does sourdough bread fermentation work?",
    );
    let j3: serde_json::Value = serde_json::from_str(&t3_raw).unwrap();
    let c3 = j3["generation"]["generated_token_count"].as_u64().unwrap();
    let reply = j3["assistant_reply"].as_str().unwrap_or("");
    println!("Multi-turn Turn 3 generated_token_count: {}", c3);
    println!("Multi-turn Turn 3 reply: {}", reply);

    // This demonstrates the multi-turn collapse defect in M2 Iteration 3
    // In lib.rs:6444, the test requires c3 >= 20.
    // Currently, c3 is 5 or 8 tokens due to unanchored reply memory fallback.
    assert!(
        c3 >= 20,
        "REGRESSION: Multi-turn chat collapsed to only {} tokens! Reply: '{}'",
        c3,
        reply
    );
}
