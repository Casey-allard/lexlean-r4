//! Milestone 1 Adversarial Empirical Verification Suite
//!
//! Stress-tests:
//! 1. 50-turn dialogue context retention, value bounds [-30000, 30000], thinking block parsing/leakage,
//!    summary bounding, and monotonic turn progression in `chat_assistant_json`.
//! 2. Wasm weight consolidation (`studio_consolidate_weights`) with valid, invalid, corrupted,
//!    and empty byte inputs.

use lexlean_r4_wasm::lut_stack_exec::LutStackModel;
use lexlean_r4_wasm::{dispatch_string, studio_consolidate_weights, StudioEngine};
use serde_json::Value;

#[test]
fn test_adversarial_50_turn_chat_context_stress() {
    let mut engine = StudioEngine::new_default();

    // 50 diverse dialogue turns spanning mathematical queries, knowledge retrieval,
    // multi-turn contextual follow-ups, code synthesis, edge prompts, and thinking prompts.
    let dialogue_prompts = [
        // 1-5: Arithmetic, number theory, prime factorization
        "What is gcd(144, 84)?",
        "What is lcm(12, 18)?",
        "Is 2027 a prime number?",
        "Factorize 360 into prime components",
        "Calculate gcd(1071, 462)",
        // 6-10: UOR-R4 architecture & geometry
        "What is UOR-R4?",
        "Explain the Cl(0,6) Clifford carrier action",
        "How does the E8 root lattice orbit project into 16 discrete roots?",
        "Describe the 40-node ternary routing forest",
        "What is the Q12 fixed-point representation?",
        // 11-15: Contextual followups and anaphora
        "How does that relate to the 256D latent state?",
        "Explain the previous mechanism in more detail",
        "What was the first number we calculated the gcd of?",
        "Can you summarize our discussion so far?",
        "Continue with the next mathematical implication",
        // 16-20: Code synthesis & workspace
        "Write a python function to compute fibonacci numbers",
        "Refactor it to use dynamic programming",
        "Write a rust function for modular exponentiation",
        "Add unit tests for the previous function",
        "Explain the time complexity of the generated code",
        // 21-25: Inline teaching turns with structured thinking blocks
        "Teach: What is Fermat's Little Theorem? => <thinking>Recall a^(p-1) = 1 mod p for prime p coprime to a</thinking>Fermat's Little Theorem states that if p is prime and a is not divisible by p, then a^(p-1) == 1 (mod p).",
        "What is Fermat's Little Theorem?",
        "Teach: What is Wilson's Theorem? => <thought>Wilson theorem says (p-1)! = -1 mod p</thought>Wilson's Theorem states that a natural number p > 1 is prime if and only if (p-1)! == -1 (mod p).",
        "What is Wilson's Theorem?",
        "Teach: What is Goldbach's Conjecture? => Every even integer greater than 2 is the sum of two primes.",
        // 26-30: Retrieval of taught facts and reasoning
        "What is Goldbach's Conjecture?",
        "Combine Wilson's Theorem and Fermat's Little Theorem",
        "Which theorems have we discussed in this chat?",
        "What is gcd(997, 991)?",
        "Explain why both are twin primes or close primes",
        // 31-35: Edge cases: single words, long strings, punctuation, unicode
        "help",
        "What? Why? How?",
        "   padded with leading and trailing whitespace   ",
        "Alpha Beta Gamma Delta Epsilon Zeta Eta Theta Iota Kappa Lambda Mu Nu Xi Omicron Pi Rho Sigma Tau Upsilon Phi Chi Psi Omega",
        "Explain the geometric difference between E8 and Leech lattice Λ24",
        // 36-40: Exact arithmetic stress
        "What is gcd(982451653, 982451653)?",
        "Is 999983 a prime number?",
        "Factorize 1000000",
        "What is gcd(0, 42)?",
        "What is lcm(21, 6)?",
        // 41-45: Complex reasoning & code queries
        "Write a typescript interface for a neural state vector",
        "Add serialization and deserialization methods to it",
        "Explain how Q12 values prevent floating-point nondeterminism",
        "What happens when coordinate descent steps overflow 30000?",
        "Explain the clamping bound [-30000, 30000]",
        // 46-50: Final continuous turns with client context pass-through
        "Review turn 1 to turn 45 progress",
        "State the core invariants of the UOR-R4 LUT stack",
        "Verify that all context values remain within bounds",
        "What is the current turn index?",
        "Final conclusion of the multi-turn session",
    ];

    assert_eq!(dialogue_prompts.len(), 50);

    let mut prev_turn_index = 0u32;
    let mut client_context: Option<Value> = None;
    let mut thinking_observed_count = 0;
    let mut thinking_leaked_count = 0;

    for (turn_num, &prompt) in dialogue_prompts.iter().enumerate() {
        let envelope = if let Some(ref ctx) = client_context {
            serde_json::json!({
                "prompt": prompt,
                "context": ctx
            })
            .to_string()
        } else {
            prompt.to_string()
        };

        let response_str = engine.chat_assistant_json(&envelope);
        let resp: Value = serde_json::from_str(&response_str).unwrap_or_else(|e| {
            panic!(
                "Turn {} response is not valid JSON: {e}\nRaw: {response_str}",
                turn_num + 1
            );
        });

        assert_eq!(
            resp["status"],
            "ok",
            "Turn {} status must be ok. Response: {response_str}",
            turn_num + 1
        );

        let assistant_reply = resp["assistant_reply"]
            .as_str()
            .expect("assistant_reply must be string");
        assert!(
            !assistant_reply.is_empty(),
            "Turn {} reply must not be empty",
            turn_num + 1
        );

        // Check thinking extraction and leakage
        let thinking_field = &resp["thinking"];
        if !thinking_field.is_null() {
            thinking_observed_count += 1;
            let thinking_str = thinking_field
                .as_str()
                .expect("thinking must be string if non-null");
            assert!(
                !thinking_str.is_empty(),
                "thinking trace must not be empty if present"
            );
        }

        // LEAKAGE CHECK:
        // User-facing reply text ("assistant_reply") must NOT contain raw <thinking> or <thought> tags
        if assistant_reply.contains("<thinking>")
            || assistant_reply.contains("</thinking>")
            || assistant_reply.contains("<thought>")
            || assistant_reply.contains("</thought>")
        {
            thinking_leaked_count += 1;
            eprintln!(
                "LEAKAGE DETECTED on turn {}: assistant_reply contains raw thinking tags!\nReply: {}",
                turn_num + 1,
                assistant_reply
            );
        }

        // Verify Context Object
        let context = &resp["context"];
        assert!(
            context.is_object(),
            "Turn {} context must be an object",
            turn_num + 1
        );

        // 1. Monotonic turn_index
        let turn_idx = context["turn_index"]
            .as_u64()
            .expect("turn_index must be u64") as u32;
        assert!(
            turn_idx > prev_turn_index,
            "Turn {} turn_index must strictly increase: got {}, previous was {}",
            turn_num + 1,
            turn_idx,
            prev_turn_index
        );
        prev_turn_index = turn_idx;

        // 2. Q12 Latent vector bounds: strictly within [-30000, 30000]
        let q12_latent = context["q12_latent"]
            .as_array()
            .expect("q12_latent must be an array");
        assert_eq!(
            q12_latent.len(),
            256,
            "Turn {} q12_latent must have exactly 256 dimensions",
            turn_num + 1
        );

        for (d, val) in q12_latent.iter().enumerate() {
            let num = val.as_i64().expect("q12_latent item must be integer");
            assert!(
                (-30000..=30000).contains(&num),
                "Turn {} dimension {} out of bounds [-30000, 30000]: got {}",
                turn_num + 1,
                d,
                num
            );
        }

        // 3. Context summary updates cleanly without unbounded growth
        let context_summary = context["context_summary"]
            .as_str()
            .expect("context_summary must be string");
        if turn_num > 0 {
            assert!(
                !context_summary.is_empty(),
                "Turn {} context_summary should not be empty after turn 1",
                turn_num + 1
            );
            let parts: Vec<&str> = context_summary.split(" | ").collect();
            assert!(
                parts.len() <= 3,
                "Turn {} context_summary exceeded 3-turn window: found {} parts: {:?}",
                turn_num + 1,
                parts.len(),
                parts
            );
            // Verify summary does NOT contain raw thinking blocks
            assert!(
                !context_summary.contains("<thinking>") && !context_summary.contains("</thinking>"),
                "context_summary leaked <thinking> tags: {}",
                context_summary
            );
        }

        // Pass context forward for alternate turns to test dual-mode operation
        if turn_num % 2 == 1 {
            client_context = Some(context.clone());
        } else {
            client_context = None;
        }
    }

    println!(
        "50-turn chat stress completed. Final turn_index: {prev_turn_index}, Thinking observed: {thinking_observed_count}, Thinking leaked: {thinking_leaked_count}"
    );

    // Assert zero leakage into user-facing assistant reply
    assert_eq!(
        thinking_leaked_count, 0,
        "Thinking blocks leaked into user-facing assistant_reply in {thinking_leaked_count} turns!"
    );
}

#[test]
fn test_adversarial_wasm_consolidate_weights_stress() {
    let bootstrap_bytes = include_bytes!("../src/bootstrap_v8.uorlut");

    // --- Sub-test 1: Valid Configurations ---
    println!("Testing valid configurations...");

    // Empty config string -> defaults
    let res_empty = studio_consolidate_weights(bootstrap_bytes, "");
    assert!(
        res_empty.is_ok(),
        "empty config string should succeed with defaults: {:?}",
        res_empty.err()
    );
    let bytes_empty = res_empty.unwrap();
    assert_eq!(&bytes_empty[0..8], b"UORLUT01");
    let model_empty = LutStackModel::parse(&bytes_empty).expect("must parse output");
    assert_eq!(model_empty.audit_point_key_synchronization(), 0);

    // Empty JSON object -> defaults
    let res_json_empty = studio_consolidate_weights(bootstrap_bytes, "{}");
    assert!(
        res_json_empty.is_ok(),
        "empty JSON object should succeed: {:?}",
        res_json_empty.err()
    );

    // Explicit valid config
    let valid_config = serde_json::json!({
        "iterations": 2,
        "step_numerator": 1,
        "step_denominator": 8,
        "clamp_min": -30000,
        "clamp_max": 30000,
        "sync_point_keys": true,
        "update_ternary_assoc": true,
        "samples": [
            {"prompt": "What is UOR-R4?", "reply": "A float-free geometric language model."},
            {"prompt": "Compute gcd(144, 84)", "reply": "gcd is 12."}
        ]
    })
    .to_string();
    let res_valid = studio_consolidate_weights(bootstrap_bytes, &valid_config);
    assert!(
        res_valid.is_ok(),
        "valid config should succeed: {:?}",
        res_valid.err()
    );
    let bytes_valid = res_valid.unwrap();
    assert_eq!(&bytes_valid[0..8], b"UORLUT01");
    let model_valid = LutStackModel::parse(&bytes_valid).expect("must parse consolidated model");
    assert_eq!(model_valid.audit_point_key_synchronization(), 0);

    // Config with `turns` instead of `samples`
    let valid_turns_config = serde_json::json!({
        "iterations": 1,
        "turns": [
            ["What is exact integer arithmetic?", "Arithmetic without IEEE-754 floats."]
        ]
    })
    .to_string();
    let res_turns = studio_consolidate_weights(bootstrap_bytes, &valid_turns_config);
    assert!(
        res_turns.is_ok(),
        "valid turns config should succeed: {:?}",
        res_turns.err()
    );

    // Empty slice as weights_bytes -> falls back to embedded bootstrap weights
    let res_empty_slice = studio_consolidate_weights(&[], "{}");
    assert!(
        res_empty_slice.is_ok(),
        "empty weights slice should fall back to default bootstrap: {:?}",
        res_empty_slice.err()
    );
    assert_eq!(&res_empty_slice.unwrap()[0..8], b"UORLUT01");

    // --- Sub-test 2: Invalid JSON Configurations ---
    println!("Testing invalid configurations...");

    let invalid_configs = [
        "{bad json",
        "{\"iterations\": \"not_a_number\"}",
        "{\"step_denominator\": 0}", // division by zero in coordinate descent
        "{\"step_numerator\": 10, \"step_denominator\": 5}", // s > d invalid step
        "{\"clamp_min\": 1000, \"clamp_max\": -1000}", // clamp_min > clamp_max
        "{\"clamp_min\": \"low\", \"clamp_max\": \"high\"}",
        "{\"samples\": \"not_an_array\"}",
        "{\"samples\": [{\"prompt\": 123}]}",
        "{\"turns\": \"invalid\"}",
        "null",
        "12345",
        "\"a string\"",
    ];

    for (idx, &bad_cfg) in invalid_configs.iter().enumerate() {
        let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            studio_consolidate_weights(bootstrap_bytes, bad_cfg)
        }));
        assert!(
            run_result.is_ok(),
            "Invalid config #{} ({}) PANICKED instead of returning Err",
            idx,
            bad_cfg
        );
        let res = run_result.unwrap();
        assert!(
            res.is_err(),
            "Invalid config #{} ({}) should have returned Err, but returned Ok",
            idx,
            bad_cfg
        );
        let err_msg = res.unwrap_err();
        assert!(!err_msg.is_empty(), "Error message must not be empty");
    }

    // --- Sub-test 3: Corrupted Byte Inputs ---
    println!("Testing corrupted byte inputs...");

    let corrupted_inputs: Vec<(&str, Vec<u8>)> = vec![
        ("1-byte input", vec![0x42]),
        ("7-byte input (short of magic)", b"UORLUT0".to_vec()),
        ("8-byte wrong magic", b"BADMAGIC".to_vec()),
        (
            "16-byte short container",
            b"UORLUT01\x00\x00\x00\x00\x00\x00\x00\x00".to_vec(),
        ),
        ("64-byte null header", {
            let mut b = vec![0u8; 64];
            b[0..8].copy_from_slice(b"UORLUT01");
            b
        }),
        ("all-zeros 1024 bytes", vec![0u8; 1024]),
        ("all-0xFF 4096 bytes", vec![0xFFu8; 4096]),
        (
            "truncated bootstrap (first 500 bytes)",
            bootstrap_bytes[..500.min(bootstrap_bytes.len())].to_vec(),
        ),
        ("corrupted magic in bootstrap", {
            let mut b = bootstrap_bytes.to_vec();
            b[0] = b'X';
            b
        }),
        ("corrupted section offset in header", {
            let mut b = bootstrap_bytes.to_vec();
            if b.len() > 32 {
                b[16] = 0xFF;
                b[17] = 0xFF;
                b[18] = 0xFF;
                b[19] = 0x7F;
            }
            b
        }),
    ];

    for (desc, corrupt_bytes) in corrupted_inputs {
        let res = studio_consolidate_weights(&corrupt_bytes, "{}");
        assert!(
            res.is_err(),
            "Corrupted input '{}' should have returned Err, but returned Ok",
            desc
        );
        let err_msg = res.unwrap_err();
        assert!(
            !err_msg.is_empty(),
            "Error message for '{}' must not be empty",
            desc
        );
    }

    println!("All Wasm export stress test cases passed successfully!");
}

#[test]
fn test_adversarial_conversational_context_carry_forward_and_agent_mode() {
    let mut engine = StudioEngine::new_default();

    // 1. Initial normal turn
    let r1 = engine.chat_assistant_json("What is UOR-R4?");
    let v1: Value = serde_json::from_str(&r1).expect("valid JSON");
    assert_eq!(v1["status"], "ok");
    assert!(
        v1["thinking"].is_null(),
        "thinking should be null on normal turn"
    );
    let rep1 = v1["assistant_reply"].as_str().unwrap();
    assert!(!rep1.contains("<thinking>") && !rep1.contains("<thought>"));

    // 2. Teach command with multiple structured thinking blocks
    let teach_prompt = "Teach: What is Fermat's Last Theorem? => <thinking>Fermat conjectured this in 1637</thinking><thought>Andrew Wiles proved it in 1994 using modular elliptic curves</thought>No three positive integers a, b, c satisfy a^n + b^n = c^n for n > 2.";
    let r2 = engine.chat_assistant_json(teach_prompt);
    let v2: Value = serde_json::from_str(&r2).expect("valid JSON");
    assert_eq!(v2["status"], "ok");
    let rep2 = v2["assistant_reply"].as_str().unwrap();

    // Sanitization check on Teach confirmation notice:
    assert!(
        !rep2.contains("<thinking>")
            && !rep2.contains("</thinking>")
            && !rep2.contains("<thought>")
            && !rep2.contains("</thought>"),
        "Teach confirmation notice leaked raw thinking tags: {rep2}"
    );
    assert!(
        rep2.contains("Learned Response"),
        "confirmation notice must mention learned response"
    );
    assert!(
        rep2.contains("No three positive integers a, b, c satisfy a^n + b^n = c^n for n > 2."),
        "confirmation notice must contain clean reply: {rep2}"
    );

    // Verify thinking trace extracted into turn metadata
    assert!(
        !v2["thinking"].is_null(),
        "thinking trace must be recorded from Teach prompt"
    );
    let trace2 = v2["thinking"].as_str().unwrap();
    assert!(
        trace2.contains("Fermat conjectured this in 1637")
            && trace2.contains("Andrew Wiles proved it in 1994 using modular elliptic curves"),
        "thinking trace must contain all combined blocks: {trace2}"
    );

    // 3. Query the taught fact
    let r3 = engine.chat_assistant_json("What is Fermat's Last Theorem?");
    let v3: Value = serde_json::from_str(&r3).expect("valid JSON");
    assert_eq!(v3["status"], "ok");
    let rep3 = v3["assistant_reply"].as_str().unwrap();
    assert!(
        !rep3.contains("<thinking>")
            && !rep3.contains("</thinking>")
            && !rep3.contains("<thought>")
            && !rep3.contains("</thought>"),
        "Subsequent turn querying taught fact leaked thinking tags: {rep3}"
    );
    assert!(
        rep3.contains("No three positive integers a, b, c satisfy a^n + b^n = c^n for n > 2."),
        "Subsequent turn must recall taught response: {rep3}"
    );

    // 4. Teach command with UNCLOSED thinking tag (must be cleanly truncated without panic)
    let unclosed_prompt = "Teach: What is Euler's Identity? => <thinking>e^(i*pi) + 1 = 0 unclosed reasoning trace...e^(i*pi) + 1 = 0 is Euler's identity.";
    let r4 = engine.chat_assistant_json(unclosed_prompt);
    let v4: Value = serde_json::from_str(&r4).expect("valid JSON");
    assert_eq!(v4["status"], "ok");
    let rep4 = v4["assistant_reply"].as_str().unwrap();
    assert!(
        !rep4.contains("<thinking>")
            && !rep4.contains("</thinking>")
            && !rep4.contains("<thought>")
            && !rep4.contains("</thought>"),
        "Unclosed thinking tag leaked into confirmation notice: {rep4}"
    );
    let trace4 = v4["thinking"].as_str().expect("thinking trace extracted");
    assert!(trace4.contains("unclosed reasoning trace"));

    // 5. ReAct Agent Mode turn via 1\tstudio\tagent\t
    // Must PRESERVE <thought> and <tool_call> as required by agentic protocol!
    let agent_cmd = "1\tstudio\tagent\tAvailable tools: read_file, run_tests, edit_file. Please inspect src/engine.rs to verify the router implementation.";
    let agent_raw = dispatch_string(agent_cmd);
    let agent_json: Value = serde_json::from_str(&agent_raw).expect("valid agent JSON");
    assert_eq!(agent_json["status"], "ok");
    let agent_rep = agent_json["assistant_reply"].as_str().unwrap();
    assert!(
        agent_rep.contains("<thought>") && agent_rep.contains("</thought>"),
        "Agent turn via 1\\tstudio\\tagent\\t must preserve <thought> blocks: {agent_rep}"
    );
    assert!(
        agent_rep.contains("<tool_call>") && agent_rep.contains("read_file"),
        "Agent turn must emit <tool_call>: {agent_rep}"
    );

    // 6. Agent Mode turn via [agent_mode] in prompt
    let agent_mode_prompt =
        "[agent_mode] Available tools: read_file, run_tests. Inspect src/lib.rs";
    let r6 = engine.chat_assistant_json(agent_mode_prompt);
    let v6: Value = serde_json::from_str(&r6).expect("valid JSON");
    assert_eq!(v6["status"], "ok");
    let rep6 = v6["assistant_reply"].as_str().unwrap();
    assert!(
        rep6.contains("<thought>") && rep6.contains("</thought>"),
        "[agent_mode] prompt must preserve <thought> blocks: {rep6}"
    );

    // 7. Tool Response / Observation turn
    let obs_prompt = "<tool_response>{\"status\": \"ok\", \"content\": \"pub struct GeometricRouter\"}</tool_response> Summarize the observation.";
    let r7 = engine.chat_assistant_json(obs_prompt);
    let v7: Value = serde_json::from_str(&r7).expect("valid JSON");
    assert_eq!(v7["status"], "ok");
    let rep7 = v7["assistant_reply"].as_str().unwrap();
    assert!(
        rep7.contains("<thought>") && rep7.contains("</thought>"),
        "Tool response turn must preserve <thought> blocks: {rep7}"
    );

    // 8. Subsequent conversational turn returning to normal dialogue
    let norm_after_agent = "Summarize the findings and explain next steps for testing.";
    let r8 = engine.chat_assistant_json(norm_after_agent);
    let v8: Value = serde_json::from_str(&r8).expect("valid JSON");
    assert_eq!(v8["status"], "ok");
    let rep8 = v8["assistant_reply"].as_str().unwrap();
    assert!(
        !rep8.contains("<thinking>")
            && !rep8.contains("</thinking>")
            && !rep8.contains("<thought>")
            && !rep8.contains("</thought>"),
        "Normal conversational turn after agent turns must NOT leak thinking blocks: {rep8}"
    );
    println!("Conversational context carry-forward & agent mode stress passed!");
}

#[test]
fn test_adversarial_wasm_consolidate_weights_fuzzing() {
    let bootstrap_bytes = include_bytes!("../src/bootstrap_v8.uorlut");

    // 1. Extreme and boundary clamp intervals (Must return Err with 0 panics)
    let inverted_clamps = [
        (
            "clamp_min > clamp_max (positive)",
            serde_json::json!({"clamp_min": 100, "clamp_max": 50}),
        ),
        (
            "clamp_min > clamp_max (negative)",
            serde_json::json!({"clamp_min": -50, "clamp_max": -100}),
        ),
        (
            "extreme inverted bounds i16::MAX / i16::MIN",
            serde_json::json!({"clamp_min": 32767, "clamp_max": -32768}),
        ),
        (
            "adjacent inverted bounds",
            serde_json::json!({"clamp_min": 1, "clamp_max": 0}),
        ),
        (
            "adjacent negative inverted bounds",
            serde_json::json!({"clamp_min": -1, "clamp_max": -2}),
        ),
    ];

    for (desc, cfg) in inverted_clamps {
        let cfg_str = cfg.to_string();
        let run_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            studio_consolidate_weights(bootstrap_bytes, &cfg_str)
        }));
        assert!(run_res.is_ok(), "Panicked on inverted clamp: {desc}");
        let res = run_res.unwrap();
        assert!(res.is_err(), "Must return Err for {desc}, got Ok");
        let err = res.unwrap_err();
        assert!(
            err.contains("invalid clamp bounds"),
            "Error must mention invalid clamp bounds: {err}"
        );
    }

    // 2. Degenerate clamp bounds where clamp_min == clamp_max (Must NOT panic, must return valid UORLUT01)
    let degenerate_clamps = [
        (
            "zero clamp [0, 0]",
            serde_json::json!({"clamp_min": 0, "clamp_max": 0, "iterations": 1}),
        ),
        (
            "positive degenerate [1000, 1000]",
            serde_json::json!({"clamp_min": 1000, "clamp_max": 1000, "iterations": 1}),
        ),
        (
            "negative degenerate [-1000, -1000]",
            serde_json::json!({"clamp_min": -1000, "clamp_max": -1000, "iterations": 1}),
        ),
        (
            "extreme boundary [-30000, -30000]",
            serde_json::json!({"clamp_min": -30000, "clamp_max": -30000, "iterations": 1}),
        ),
        (
            "extreme boundary [30000, 30000]",
            serde_json::json!({"clamp_min": 30000, "clamp_max": 30000, "iterations": 1}),
        ),
    ];

    for (desc, cfg) in degenerate_clamps {
        let cfg_str = cfg.to_string();
        let run_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            studio_consolidate_weights(bootstrap_bytes, &cfg_str)
        }));
        assert!(run_res.is_ok(), "Panicked on degenerate clamp: {desc}");
        let res = run_res.unwrap();
        assert!(
            res.is_ok(),
            "Degenerate clamp interval {desc} should succeed, got error: {:?}",
            res.err()
        );
        let bytes = res.unwrap();
        assert_eq!(&bytes[0..8], b"UORLUT01");
    }

    // 3. Step configurations fuzzing
    let invalid_steps = [
        (
            "zero denominator",
            serde_json::json!({"step_denominator": 0}),
        ),
        (
            "negative denominator",
            serde_json::json!({"step_denominator": -8}),
        ),
        ("zero numerator", serde_json::json!({"step_numerator": 0})),
        (
            "negative numerator",
            serde_json::json!({"step_numerator": -1}),
        ),
        (
            "numerator > denominator",
            serde_json::json!({"step_numerator": 10, "step_denominator": 5}),
        ),
        (
            "both negative",
            serde_json::json!({"step_numerator": -4, "step_denominator": -4}),
        ),
    ];

    for (desc, cfg) in invalid_steps {
        let cfg_str = cfg.to_string();
        let run_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            studio_consolidate_weights(bootstrap_bytes, &cfg_str)
        }));
        assert!(run_res.is_ok(), "Panicked on invalid step: {desc}");
        let res = run_res.unwrap();
        assert!(res.is_err(), "Must return Err for {desc}");
        let err = res.unwrap_err();
        assert!(
            err.contains("invalid step config"),
            "Error must mention invalid step: {err}"
        );
    }

    // 4. Valid boundary step configurations (s == d is allowed)
    let valid_steps = [
        (
            "s == d (1/1)",
            serde_json::json!({"step_numerator": 1, "step_denominator": 1, "iterations": 1}),
        ),
        (
            "s == d (8/8)",
            serde_json::json!({"step_numerator": 8, "step_denominator": 8, "iterations": 1}),
        ),
    ];

    for (desc, cfg) in valid_steps {
        let cfg_str = cfg.to_string();
        let run_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            studio_consolidate_weights(bootstrap_bytes, &cfg_str)
        }));
        assert!(run_res.is_ok(), "Panicked on valid step: {desc}");
        let res = run_res.unwrap();
        assert!(
            res.is_ok(),
            "Valid step {desc} should succeed, got: {:?}",
            res.err()
        );
    }

    // 5. Malformed, corrupted, and adversarial JSON payloads
    let adversarial_payloads = [
        (
            "deeply nested array",
            "[[[[[[[[[[[[[[[[[[[[{}]]]]]]]]]]]]]]]]]]]]",
        ),
        (
            "unicode control chars",
            "{\"iterations\": 1, \"\u{0000}\": \"\u{001F}\"}",
        ),
        ("non-object primitive int", "999999999"),
        ("non-object boolean", "true"),
        ("escaped quotes chaos", "\"\\\"\\\"\\\"\\\"\""),
        ("negative iterations", "{\"iterations\": -5}"),
        (
            "overflow iterations",
            "{\"iterations\": 999999999999999999999999999999}",
        ),
        (
            "type confusion clamp_min bool",
            "{\"clamp_min\": true, \"clamp_max\": false}",
        ),
        (
            "type confusion clamp_min array",
            "{\"clamp_min\": [100], \"clamp_max\": [200]}",
        ),
        (
            "type confusion samples object instead of array",
            "{\"samples\": {\"prompt\": \"a\"}}",
        ),
        (
            "type confusion turns object instead of array",
            "{\"turns\": {\"turn\": 1}}",
        ),
    ];

    for (desc, payload) in adversarial_payloads {
        let run_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            studio_consolidate_weights(bootstrap_bytes, payload)
        }));
        assert!(run_res.is_ok(), "Panicked on adversarial payload: {desc}");
        let res = run_res.unwrap();
        // Either gracefully parses or returns Err; ZERO panics permitted!
        if let Err(e) = res {
            assert!(!e.is_empty());
        }
    }

    println!("All Wasm consolidation fuzzing and clamp boundary tests passed!");
}
