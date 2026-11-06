#![allow(
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    clippy::identity_op,
    clippy::manual_range_contains,
    clippy::useless_vec
)]
//! Empirical Challenge Suite for Milestone M5:
//! Direct Mount & Local Helper Adversarial Harness
//!
//! Stress-tests:
//! 1. `cyclic_frame_block` boundary attacks:
//!    - `blocks == 0` (zero division protection)
//!    - Saturation near `u64::MAX` without debug-mode overflow panic
//!    - Modulo wrap-around consistency and bijection across all 8 attention heads
//!    - End-to-end `modulate_flock_cyclic_frame_block` and `modulate_flock_cyclic_attention`
//! 2. `parse_thinking_block` adversarial inputs:
//!    - Standard single block and empty tag handling
//!    - Sibling tag variations (`<thought>` vs `<thinking>`, interleaved, mixed)
//!    - Multiple sequential blocks and visible interstitial content
//!    - Unclosed tags (at EOF, after closed tags, whitespace-only unclosed)
//!    - Unicode, multibyte UTF-8 boundary integrity (CJK, Emoji, Math symbols, RTL)
//!    - Malformed, near-miss, nested, and pathological fuzz inputs
//! 3. Zero-float integer Q12 invariant conformance and direct-mount contract parity

use lexlean_r4_wasm::lut_stack_exec::{
    cyclic_frame_block, modulate_flock_cyclic_attention, modulate_flock_cyclic_frame_block,
    parse_thinking_block, FBS_WIDTH,
};
use std::collections::BTreeSet;

// ============================================================================
// PART 1: cyclic_frame_block Adversarial Boundary Attacks
// ============================================================================

#[test]
fn challenge_m5_cyclic_frame_block_zero_blocks_boundary() {
    // blocks == 0 must never divide by zero or panic; it must safely evaluate to 0.
    assert_eq!(cyclic_frame_block(0, 0, 0), 0);
    assert_eq!(cyclic_frame_block(1, 0, 0), 0);
    assert_eq!(cyclic_frame_block(0, 1, 0), 0);
    assert_eq!(cyclic_frame_block(7, 7, 0), 0);
    assert_eq!(cyclic_frame_block(42, 100, 0), 0);
    assert_eq!(cyclic_frame_block(u64::MAX, u64::MAX, 0), 0);
    assert_eq!(cyclic_frame_block(u64::MAX - 1, 0, 0), 0);

    // Fuzz boundary combinations with blocks == 0
    let test_vals = [0, 1, 2, 7, 8, 15, 16, 255, 1024, u64::MAX - 1, u64::MAX];
    for &b in &test_vals {
        for &s in &test_vals {
            let res = cyclic_frame_block(b, s, 0);
            assert_eq!(
                res, 0,
                "cyclic_frame_block({}, {}, 0) expected 0, got {}",
                b, s, res
            );
        }
    }
}

#[test]
fn challenge_m5_cyclic_frame_block_saturation_near_u64_max() {
    // Extreme values near u64::MAX must use saturating arithmetic without wrapping/overflow panics
    let max = u64::MAX;

    // Saturation checks with standard blocks = 8
    // (u64::MAX + 0 + 1) saturates to u64::MAX. u64::MAX % 8 == 7.
    assert_eq!(cyclic_frame_block(max, 0, 8), max % 8);
    assert_eq!(cyclic_frame_block(0, max, 8), max % 8);
    assert_eq!(cyclic_frame_block(max, max, 8), max % 8);

    // max - 1 + 0 + 1 == max -> max % 8
    assert_eq!(cyclic_frame_block(max - 1, 0, 8), max % 8);
    // max - 2 + 0 + 1 == max - 1 -> (max - 1) % 8
    assert_eq!(cyclic_frame_block(max - 2, 0, 8), (max - 1) % 8);
    // max - 1 + 1 + 1 saturates to max -> max % 8
    assert_eq!(cyclic_frame_block(max - 1, 1, 8), max % 8);

    // Large divisors
    assert_eq!(cyclic_frame_block(max, max, max), max % max); // 0
    assert_eq!(cyclic_frame_block(max, 0, max), max % max); // 0
    assert_eq!(cyclic_frame_block(max, 0, max - 1), max % (max - 1)); // 1

    // Exhaustive sweep around the saturation boundary
    let edge_blocks = [1, 2, 3, 7, 8, 16, 64, 1024, max - 1, max];
    for offset_b in 0..10u64 {
        let b = max - offset_b;
        for offset_s in 0..10u64 {
            let s = max - offset_s;
            for &blocks in &edge_blocks {
                let res = cyclic_frame_block(b, s, blocks);
                assert!(
                    res < blocks,
                    "cyclic_frame_block({}, {}, {}) = {} violated upper bound {}",
                    b,
                    s,
                    blocks,
                    res,
                    blocks
                );
            }
        }
    }
}

#[test]
fn challenge_m5_cyclic_frame_block_modulo_wraparound_and_bijection() {
    // In R4.Flock, for each sweep 0..8, routing all 8 heads must form a strict permutation (bijection) of 0..8.
    for sweep in 0..64u64 {
        let mut mapped = BTreeSet::new();
        for head in 0..8u64 {
            let target = cyclic_frame_block(head, sweep, 8);
            assert!(
                target < 8,
                "Head {} mapped to out-of-range target {}",
                head,
                target
            );
            mapped.insert(target);
        }
        assert_eq!(
            mapped.len(),
            8,
            "Sweep {} did not produce a bijective permutation of 8 heads: {:?}",
            sweep,
            mapped
        );
        let expected: BTreeSet<u64> = (0..8).collect();
        assert_eq!(
            mapped, expected,
            "Sweep {} permutation did not cover 0..8 exactly",
            sweep
        );
    }

    // Verify multi-frame cyclic periodicity:
    // (head + sweep + 1) % 8 has period 8 in sweep (prior to saturation).
    for head in 0..8u64 {
        for sweep in 0..100u64 {
            let r1 = cyclic_frame_block(head, sweep, 8);
            let r2 = cyclic_frame_block(head, sweep + 8, 8);
            let r3 = cyclic_frame_block(head, sweep + 80, 8);
            assert_eq!(
                r1,
                r2,
                "Periodicity broken for head {} between sweep {} and {}",
                head,
                sweep,
                sweep + 8
            );
            assert_eq!(
                r1,
                r3,
                "Periodicity broken for head {} between sweep {} and {}",
                head,
                sweep,
                sweep + 80
            );
        }
    }
}

#[test]
fn challenge_m5_modulate_flock_cyclic_frame_block_end_to_end() {
    // Construct identifiable 256D hidden state where each head h (32 dimensions) carries (h + 1) * 100
    let mut n_t = [0i32; FBS_WIDTH];
    for h in 0..8 {
        for d in 0..32 {
            n_t[h * 32 + d] = ((h + 1) * 100) as i32;
        }
    }

    for tokens_len in 0..32 {
        let modulated = modulate_flock_cyclic_frame_block(&n_t, tokens_len);
        let modulated_alias = modulate_flock_cyclic_attention(&n_t, tokens_len);
        assert_eq!(
            modulated, modulated_alias,
            "modulate_flock_cyclic_attention must match modulate_flock_cyclic_frame_block"
        );

        let sweep = (tokens_len as u64) % 8;
        for dst_head in 0..8 {
            let src_head = cyclic_frame_block(dst_head as u64, sweep, 8) as usize;
            let expected_val = ((src_head + 1) * 100) as i32;

            for d in 0..32 {
                assert_eq!(
                    modulated[dst_head * 32 + d],
                    expected_val,
                    "tokens_len {}: dst_head {} dim {} expected {} from src_head {}, got {}",
                    tokens_len,
                    dst_head,
                    d,
                    expected_val,
                    src_head,
                    modulated[dst_head * 32 + d]
                );
            }
        }

        // Energy conservation check: L1 sum across all 256 dimensions must be strictly invariant under permutation
        let orig_sum: i64 = n_t.iter().map(|&x| x as i64).sum();
        let mod_sum: i64 = modulated.iter().map(|&x| x as i64).sum();
        assert_eq!(
            orig_sum, mod_sum,
            "L1 sum not conserved across modulation at tokens_len {}",
            tokens_len
        );
    }
}

// ============================================================================
// PART 2: parse_thinking_block Adversarial Inputs
// ============================================================================

#[test]
fn challenge_m5_parse_thinking_block_standard_and_empty() {
    // 1. Standard single block
    let (trace, clean) = parse_thinking_block("<thinking>Deduce step 1</thinking>Visible answer.");
    assert_eq!(trace.as_deref(), Some("Deduce step 1"));
    assert_eq!(clean, "Visible answer.");

    // 2. Absent thinking tags
    let (trace, clean) = parse_thinking_block("Plain visible answer with no thinking.");
    assert_eq!(trace, None);
    assert_eq!(clean, "Plain visible answer with no thinking.");

    // 3. Completely empty tag
    let (trace, clean) = parse_thinking_block("<thinking></thinking>Answer after empty tag.");
    assert_eq!(trace, None);
    assert_eq!(clean, "Answer after empty tag.");

    // 4. Whitespace-only tag contents
    let (trace, clean) =
        parse_thinking_block("<thinking>  \n\t \r\n </thinking>Answer after whitespace tag.");
    assert_eq!(trace, None);
    assert_eq!(clean, "Answer after whitespace tag.");
}

#[test]
fn challenge_m5_parse_thinking_block_sibling_tags() {
    // 1. Sibling tag <thought>...</thought>
    let (trace, clean) =
        parse_thinking_block("<thought>Pondering lemma</thought>The theorem holds.");
    assert_eq!(trace.as_deref(), Some("Pondering lemma"));
    assert_eq!(clean, "The theorem holds.");

    // 2. Empty sibling tag
    let (trace, clean) = parse_thinking_block("<thought></thought>Post thought.");
    assert_eq!(trace, None);
    assert_eq!(clean, "Post thought.");

    // 3. Mixed sequential sibling tags
    let (trace, clean) = parse_thinking_block(
        "<thinking>First idea</thinking> Middle text <thought>Second idea</thought> Final reply.",
    );
    assert_eq!(trace.as_deref(), Some("First idea\n\nSecond idea"));
    assert_eq!(clean, "Middle text  Final reply.");

    // 4. Reversed sibling order: <thought> first, then <thinking>
    let (trace, clean) = parse_thinking_block(
        "<thought>Alpha</thought> Intermediate <thinking>Beta</thinking> Conclusion.",
    );
    assert_eq!(trace.as_deref(), Some("Alpha\n\nBeta"));
    assert_eq!(clean, "Intermediate  Conclusion.");
}

#[test]
fn challenge_m5_parse_thinking_block_multiple_sequential_blocks() {
    // 1. Multiple adjacent thinking blocks without interstitial text
    let (trace, clean) = parse_thinking_block(
        "<thinking>Block A</thinking><thinking>Block B</thinking><thinking>Block C</thinking>Answer."
    );
    assert_eq!(trace.as_deref(), Some("Block A\n\nBlock B\n\nBlock C"));
    assert_eq!(clean, "Answer.");

    // 2. Multiple blocks interspersed with visible text
    let (trace, clean) = parse_thinking_block(
        "Intro <thinking>Step 1</thinking> Part 1 <thought>Step 2</thought> Part 2 <thinking>Step 3</thinking> Done."
    );
    assert_eq!(trace.as_deref(), Some("Step 1\n\nStep 2\n\nStep 3"));
    assert_eq!(clean, "Intro  Part 1  Part 2  Done.");

    // 3. High block count stress (20 sequential alternating blocks)
    let mut input = String::new();
    let mut expected_trace_parts = Vec::new();
    for i in 0..20 {
        if i % 2 == 0 {
            input.push_str(&format!("<thinking>Thought {}</thinking>", i));
        } else {
            input.push_str(&format!("<thought>Thought {}</thought>", i));
        }
        expected_trace_parts.push(format!("Thought {}", i));
    }
    input.push_str("Final output.");

    let (trace, clean) = parse_thinking_block(&input);
    assert_eq!(
        trace.as_deref(),
        Some(expected_trace_parts.join("\n\n").as_str())
    );
    assert_eq!(clean, "Final output.");
}

#[test]
fn challenge_m5_parse_thinking_block_unclosed_tags() {
    // 1. Unclosed <thinking> tag at end of response
    let (trace, clean) =
        parse_thinking_block("Visible answer <thinking>Trailing stream never closed");
    assert_eq!(trace.as_deref(), Some("Trailing stream never closed"));
    assert_eq!(clean, "Visible answer");

    // 2. Unclosed <thought> tag at end of response
    let (trace, clean) =
        parse_thinking_block("Initial content <thought>Incomplete internal monologue");
    assert_eq!(trace.as_deref(), Some("Incomplete internal monologue"));
    assert_eq!(clean, "Initial content");

    // 3. Entire response is an unclosed tag
    let (trace, clean) =
        parse_thinking_block("<thinking>Only reasoning, model cut off by max tokens");
    assert_eq!(
        trace.as_deref(),
        Some("Only reasoning, model cut off by max tokens")
    );
    assert_eq!(clean, "");

    // 4. Closed tag followed by unclosed tag
    let (trace, clean) = parse_thinking_block(
        "<thinking>First reasoning</thinking> Middle text <thought>Cut off second thought",
    );
    assert_eq!(
        trace.as_deref(),
        Some("First reasoning\n\nCut off second thought")
    );
    assert_eq!(clean, "Middle text");

    // 5. Unclosed tag with only whitespace
    let (trace, clean) = parse_thinking_block("Prefix text <thinking>   \n\t   ");
    assert_eq!(trace, None);
    assert_eq!(clean, "Prefix text");
}

#[test]
fn challenge_m5_parse_thinking_block_unicode_and_multibyte() {
    // Multibyte UTF-8 characters: CJK, Emoji, Mathematical symbols, RTL Arabic
    // Must preserve UTF-8 byte boundary integrity and never panic on string slicing.

    // 1. Japanese text with Emoji inside thinking tag and clean response
    let input =
        "こんにちは！ <thinking>推論中：🤖 考える人 🧠 答えは42です。</thinking> さようなら！ 🎉";
    let (trace, clean) = parse_thinking_block(input);
    assert_eq!(
        trace.as_deref(),
        Some("推論中：🤖 考える人 🧠 答えは42です。")
    );
    assert_eq!(clean, "こんにちは！  さようなら！ 🎉");

    // 2. Mathematical symbols and Greek letters
    let math_input =
        "∀ε > 0 ∃δ > 0 <thought>f(x) - L < ε when |x - c| < δ</thought> Limit verified ∎";
    let (math_trace, math_clean) = parse_thinking_block(math_input);
    assert_eq!(math_trace.as_deref(), Some("f(x) - L < ε when |x - c| < δ"));
    assert_eq!(math_clean, "∀ε > 0 ∃δ > 0  Limit verified ∎");

    // 3. Arabic (RTL) text
    let arabic_input =
        "مرحبا بالعالم <thinking>تحليل أعمق للمسألة الرياضية</thinking> مع أطيب التحيات";
    let (ar_trace, ar_clean) = parse_thinking_block(arabic_input);
    assert_eq!(ar_trace.as_deref(), Some("تحليل أعمق للمسألة الرياضية"));
    assert_eq!(ar_clean, "مرحبا بالعالم  مع أطيب التحيات");

    // 4. Multibyte UTF-8 immediately adjacent to opening/closing tags
    let adj_input = "🦀<thinking>🔥</thinking>🚀";
    let (adj_trace, adj_clean) = parse_thinking_block(adj_input);
    assert_eq!(adj_trace.as_deref(), Some("🔥"));
    assert_eq!(adj_clean, "🦀🚀");
}

#[test]
fn challenge_m5_parse_thinking_block_adversarial_malformed_and_nested() {
    // 1. Near-miss tag: `<think>` (not a recognized tag)
    let (t1, c1) = parse_thinking_block("<think>Should not match</think>Text");
    assert_eq!(t1, None);
    assert_eq!(c1, "<think>Should not match</think>Text");

    // 2. Case sensitivity: `<Thinking>` must not match
    let (t2, c2) = parse_thinking_block("<Thinking>Not matched</Thinking>Text");
    assert_eq!(t2, None);
    assert_eq!(c2, "<Thinking>Not matched</Thinking>Text");

    // 3. Orphaned closing tag without opening tag
    let (t3, c3) = parse_thinking_block("Orphaned </thinking> tag in response.");
    assert_eq!(t3, None);
    assert_eq!(c3, "Orphaned </thinking> tag in response.");

    // 4. Double angle brackets variants:
    // a. `<<thinking>Inner</thinking>>`
    let (t4a, c4a) = parse_thinking_block("<<thinking>Double bracketed</thinking>>");
    assert_eq!(t4a.as_deref(), Some("Double bracketed"));
    assert_eq!(c4a, "<>");

    // b. `<<thinking>>Inner</thinking>>` (leading '>' is inside tag contents)
    let (t4b, c4b) = parse_thinking_block("<<thinking>>Double bracketed</thinking>>");
    assert_eq!(t4b.as_deref(), Some(">Double bracketed"));
    assert_eq!(c4b, "<>");

    // 5. Nested tags: inner closing tag terminates outer block safely
    let nested = "<thinking>Outer <thinking>Inner</thinking> Trailing</thinking>";
    let (t5, c5) = parse_thinking_block(nested);
    assert_eq!(t5.as_deref(), Some("Outer <thinking>Inner"));
    assert_eq!(c5, "Trailing</thinking>");

    // 6. Cross-mismatched opening and closing tags
    // `<thinking> reasoning <thought> more reasoning </thinking>`
    // Here `thinking` finds its closing `</thinking>` at the end, consuming `<thought>` as part of reasoning.
    let cross = "<thinking>Step A <thought>Nested</thought> Step B</thinking> Reply.";
    let (t6, c6) = parse_thinking_block(cross);
    assert_eq!(
        t6.as_deref(),
        Some("Step A <thought>Nested</thought> Step B")
    );
    assert_eq!(c6, "Reply.");
}

// ============================================================================
// PART 3: Direct Mount Invariant & Zero-Float Arithmetic Check
// ============================================================================

#[test]
fn challenge_m5_pure_integer_q12_and_direct_mount_contract() {
    // Invariant: cyclic_frame_block must remain strictly integer-based with zero floats.
    // Verify that modulate_flock_cyclic_frame_block output contains no NaN or infinite values.
    let mut n_t = [0i32; FBS_WIDTH];
    for i in 0..FBS_WIDTH {
        n_t[i] = (i as i32) * 16 - 2048; // Valid Q12 range values
    }

    for sweep in 0..16 {
        let modulated = modulate_flock_cyclic_frame_block(&n_t, sweep);
        for &coord in modulated.iter() {
            // Must be within integer bounds
            assert!((-2048..=4096).contains(&coord));
        }
    }
}

// ============================================================================
// PART 4: Empirical Generators, Oracles, and Fuzzing Harnesses
// ============================================================================

#[test]
fn challenge_m5_cyclic_frame_block_generator_oracle_fuzz() {
    // Deterministic pseudo-random LCG generator for reproducible stress testing
    let mut state: u64 = 0xCAFE_BABE_DEAD_BEEF;
    let mut next_u64 = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state
    };

    for _ in 0..10_000 {
        let b = next_u64();
        let s = next_u64();
        let m = next_u64();

        let res = cyclic_frame_block(b, s, m);

        // Oracle 1: Zero divisor
        if m == 0 {
            assert_eq!(res, 0, "m=0 must return 0");
        } else {
            // Oracle 2: Result must strictly be less than modulus m
            assert!(
                res < m,
                "cyclic_frame_block({}, {}, {}) = {} not strictly less than {}",
                b,
                s,
                m,
                res,
                m
            );

            // Oracle 3: If no saturation occurs, matches pure mathematical modulo
            if let Some(sum1) = b.checked_add(s) {
                if let Some(sum2) = sum1.checked_add(1) {
                    assert_eq!(
                        res,
                        sum2 % m,
                        "Unsaturated addition must match (b + s + 1) % m"
                    );
                }
            } else {
                // If saturated, must equal u64::MAX % m
                assert_eq!(
                    res,
                    u64::MAX % m,
                    "Saturated addition must match u64::MAX % m"
                );
            }
        }
    }
}

#[test]
fn challenge_m5_parse_thinking_block_fuzz_leakage_and_invariants() {
    let mut state: u64 = 0x1234_5678_9ABC_DEF0;
    let mut next_u64 = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state
    };

    let fragments = [
        "Hello",
        " world ",
        "<thinking>",
        "</thinking>",
        "<thought>",
        "</thought>",
        "<think>",
        "</think>",
        "reasoning about Q12",
        " 🦀 🚀 🤖 ",
        "∀x∃y(x < y)",
        "日本語テスト",
        "\n\n\t",
        "unclosed thought stream",
        "answer token basin",
    ];

    for _ in 0..1_000 {
        let mut document = String::new();
        let num_fragments = (next_u64() % 15) + 1;
        for _ in 0..num_fragments {
            let idx = (next_u64() as usize) % fragments.len();
            document.push_str(fragments[idx]);
        }

        let (trace, clean) = parse_thinking_block(&document);

        // Invariant 1: If trace is Some, it must never be empty or pure whitespace
        if let Some(ref t) = trace {
            assert!(
                !t.trim().is_empty(),
                "Extracted trace must not be empty or whitespace-only"
            );
        }

        // Invariant 2: Safety barrier - clean output must NEVER leak unclosed or open thinking tags
        assert!(
            !clean.contains("<thinking>"),
            "Clean text leaked <thinking> tag: {:?}",
            clean
        );
        assert!(
            !clean.contains("<thought>"),
            "Clean text leaked <thought> tag: {:?}",
            clean
        );

        // Invariant 3: Clean text must be trimmed (no leading/trailing whitespace)
        assert_eq!(
            clean,
            clean.trim(),
            "Clean text must be strictly trimmed: {:?}",
            clean
        );
    }
}
