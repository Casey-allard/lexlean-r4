#![allow(
    missing_docs,
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    unused_variables,
    unused_imports
)]
//! Empirical Adversarial Challenge Suite (Iteration 4 - Challenger 1) for Milestone M2.
//!
//! Objectives:
//! 1. Verify deep multi-turn chat horizon (5+ turns) without token count collapse or degradation.
//! 2. Verify strict context isolation across turns (no topic leakage across unrelated turns).
//! 3. Verify session reset cleanliness and parity with cold start.
//! 4. Verify continuation stability on adversarial crossover prompts.
//! 5. Verify token generation bounds across varying max_new_tokens budgets.
//! 6. Verify resilience to repetitive user inputs across multiple turns.

use lexlean_r4_wasm::lut_stack_exec::{ByteBpe4096, LutStackModel, RollingContextState};
use std::sync::{Arc, Mutex, OnceLock};

static STUDIO_DISPATCH_LOCK: Mutex<()> = Mutex::new(());

pub const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

pub fn get_test_model() -> Arc<LutStackModel> {
    static MODEL: OnceLock<Arc<LutStackModel>> = OnceLock::new();
    Arc::clone(MODEL.get_or_init(|| {
        Arc::new(LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully"))
    }))
}

// ===========================================================================
// Challenge 1: Deep Multi-Turn Conversation Token Sustainability (5 Turns)
// ===========================================================================
#[test]
fn test_empirical_deep_multiturn_conversation_token_sustainability() {
    let _guard = STUDIO_DISPATCH_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    // Reset session
    let _ = lexlean_r4_wasm::dispatch_string("1\tstudio\treset\t");

    let turns = [
        // Turn 1: Geography
        ("What is the capital of France?", 10, vec![]),
        // Turn 2: Architecture continuation
        ("What is the architecture like there?", 10, vec![]),
        // Turn 3: Sourdough fermentation (open domain culinary / biochemistry)
        (
            "How does sourdough bread fermentation work?",
            20,
            vec!["burj khalifa", "2x + 6", "algebra"],
        ),
        // Turn 4: Quantum physics
        (
            "Explain the Heisenberg uncertainty principle in quantum physics.",
            15,
            vec!["sourdough", "fermentation", "burj khalifa", "2x + 6"],
        ),
        // Turn 5: Cellular biology
        (
            "What are the stages of cellular mitosis in biology?",
            15,
            vec![
                "sourdough",
                "fermentation",
                "burj khalifa",
                "heisenberg",
                "2x + 6",
            ],
        ),
    ];

    for (turn_idx, (prompt, min_tokens, forbidden_substrings)) in turns.iter().enumerate() {
        let dispatch_cmd = format!("1\tstudio\tchat\t{}", prompt);
        let raw_resp = lexlean_r4_wasm::dispatch_string(&dispatch_cmd);
        let json_val: serde_json::Value = serde_json::from_str(&raw_resp).unwrap_or_else(|e| {
            panic!(
                "Turn {}: invalid JSON response '{}': {}",
                turn_idx + 1,
                raw_resp,
                e
            )
        });

        let token_count = json_val["generation"]["generated_token_count"]
            .as_u64()
            .unwrap_or(0);
        let reply_text = json_val["assistant_reply"].as_str().unwrap_or("");

        println!(
            "Turn {} [tokens={}]: Prompt: '{}'\nReply:\n{}\n---",
            turn_idx + 1,
            token_count,
            prompt,
            reply_text
        );

        assert!(
            token_count >= *min_tokens,
            "Turn {} ('{}') collapsed to {} tokens (expected >= {})! Reply: '{}'",
            turn_idx + 1,
            prompt,
            token_count,
            min_tokens,
            reply_text
        );

        let lower_reply = reply_text.to_lowercase();
        for forbidden in forbidden_substrings {
            assert!(
                !lower_reply.contains(forbidden),
                "Turn {} ('{}') leaked prior topic '{}'! Reply: '{}'",
                turn_idx + 1,
                prompt,
                forbidden,
                reply_text
            );
        }
    }
}

// ===========================================================================
// Challenge 2: Session Reset Cleanliness and Parity
// ===========================================================================
#[test]
fn test_empirical_session_reset_cleanliness_and_parity() {
    let _guard = STUDIO_DISPATCH_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    // 1. Pollute session with algebra and Dubai queries
    let _ = lexlean_r4_wasm::dispatch_string("1\tstudio\treset\t");
    let _ = lexlean_r4_wasm::dispatch_string(
        "1\tstudio\tchat\tTell me about the Burj Khalifa in Dubai.",
    );
    let _ =
        lexlean_r4_wasm::dispatch_string("1\tstudio\tchat\tSolve the linear equation 2x + 6 = 14.");

    // 2. Perform reset
    let reset_resp = lexlean_r4_wasm::dispatch_string("1\tstudio\treset\t");
    assert!(!reset_resp.is_empty(), "Reset must return response");

    // 3. Query sourdough post-reset
    let post_reset_raw = lexlean_r4_wasm::dispatch_string(
        "1\tstudio\tchat\tHow does sourdough bread fermentation work?",
    );
    let post_reset_json: serde_json::Value = serde_json::from_str(&post_reset_raw).unwrap();
    let post_reset_count = post_reset_json["generation"]["generated_token_count"]
        .as_u64()
        .unwrap();
    let post_reset_reply = post_reset_json["assistant_reply"].as_str().unwrap_or("");

    assert!(
        post_reset_count >= 20,
        "Post-reset sourdough query generated fewer than 20 tokens: count={}",
        post_reset_count
    );
    assert!(
        !post_reset_reply.to_lowercase().contains("burj khalifa"),
        "Post-reset reply leaked lingering Burj Khalifa anchor!"
    );
    assert!(
        !post_reset_reply.contains("2x + 6"),
        "Post-reset reply leaked lingering algebra equation!"
    );
}

// ===========================================================================
// Challenge 3: Context Isolation with Adversarial Crossover Prompts
// ===========================================================================
#[test]
fn test_empirical_context_isolation_with_adversarial_crossover_prompts() {
    let model = get_test_model();
    let mut context = RollingContextState::default();

    // Turn 1: Topic A
    let res1 = model.generate_reply_with_context(
        &mut context,
        &[],
        "Tell me about the architecture of the Eiffel Tower in Paris.",
        48,
    );
    assert!(!res1.generated_tokens.is_empty(), "Turn 1 reply empty");
    let t1_reply = res1.reply_text.clone();

    let history1 = vec![(
        "Tell me about the architecture of the Eiffel Tower in Paris.".to_string(),
        t1_reply,
    )];

    // Turn 2: Crossover prompt that mentions Eiffel Tower but asks about Sourdough bread
    let res2 = model.generate_reply_with_context(
        &mut context,
        &history1,
        "Now shifting away from the Eiffel Tower, explain the biochemistry of sourdough bread fermentation.",
        64,
    );

    println!(
        "Crossover Turn 2 [tokens={}]:\n{}",
        res2.generated_tokens.len(),
        res2.reply_text
    );

    assert!(
        res2.generated_tokens.len() >= 20,
        "Crossover prompt collapsed to {} tokens (expected >= 20). Reply was:\n{}",
        res2.generated_tokens.len(),
        res2.reply_text
    );

    // Verify it doesn't get hijacked into algebra
    assert!(
        !res2.reply_text.contains("2x + 6") && !res2.reply_text.contains("Let $x$ be an integer"),
        "Crossover prompt was hijacked into algebra:\n{}",
        res2.reply_text
    );
}

// ===========================================================================
// Challenge 4: Varying Max New Tokens Bounds & Monotonicity
// ===========================================================================
#[test]
fn test_empirical_varying_max_new_tokens_bounds() {
    let model = get_test_model();
    let prompt = "How does sourdough bread fermentation work?";

    let token_budgets = [1usize, 5, 10, 20, 50, 100, 256];

    for &budget in &token_budgets {
        let res = model.generate_reply(&[], prompt, budget);

        assert!(
            !res.reply_text.is_empty() || res.generated_tokens.is_empty(),
            "Budget {}: invalid empty reply with non-empty tokens",
            budget
        );

        // Generated tokens should not exceed the requested budget + small margin for thinking tags
        assert!(
            res.generated_tokens.len() <= budget.max(1),
            "Budget {}: generated {} tokens, exceeding limit!",
            budget,
            res.generated_tokens.len()
        );

        println!(
            "Budget {} -> generated {} tokens: '{}'",
            budget,
            res.generated_tokens.len(),
            if res.reply_text.len() > 60 {
                format!("{}...", &res.reply_text[..60])
            } else {
                res.reply_text.clone()
            }
        );
    }
}

// ===========================================================================
// Challenge 5: Repetitive User Turns Resilience (No Deadlock / Latching)
// ===========================================================================
#[test]
fn test_empirical_repetitive_user_turns_resilience() {
    let _guard = STUDIO_DISPATCH_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    // Reset session
    let _ = lexlean_r4_wasm::dispatch_string("1\tstudio\treset\t");

    // Send the same prompt three times in a row
    for turn in 1..=3 {
        let resp_raw = lexlean_r4_wasm::dispatch_string("1\tstudio\tchat\tHello, what can you do?");
        let json_val: serde_json::Value = serde_json::from_str(&resp_raw)
            .unwrap_or_else(|e| panic!("Repetitive turn {}: invalid JSON: {}", turn, e));

        let token_count = json_val["generation"]["generated_token_count"]
            .as_u64()
            .unwrap_or(0);
        let reply_text = json_val["assistant_reply"].as_str().unwrap_or("");

        assert!(
            token_count > 0,
            "Repetitive turn {} generated 0 tokens!",
            turn
        );
        assert!(
            !reply_text.is_empty(),
            "Repetitive turn {} produced empty reply!",
            turn
        );
        println!(
            "Repetitive turn {} [tokens={}]: {}",
            turn, token_count, reply_text
        );
    }
}
