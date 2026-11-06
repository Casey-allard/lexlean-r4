#![allow(
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    clippy::identity_op,
    clippy::manual_range_contains,
    clippy::useless_vec
)]
//! Empirical Challenge 1 Stress Suite for Milestone 4 Iteration 2
//! Adversarial Fuzzing & Compression Stress Harness
//!
//! Evaluates:
//! 1. Fuzz hydration with extreme timestamp offsets (u64::MAX, u64::MAX - 1, boundary values, saturating arithmetic).
//! 2. Compression ratio across variable turn lengths [1, 4, 16, 32, 64] ensuring >=50.0% for <=4 tokens and >=60.0% for >=16 tokens.
//! 3. 10,000 permutations trace/span ID determinism, boundary robustness, and collision freedom.

use std::collections::HashSet;
use std::sync::Arc;

use lexlean_r4_wasm::lut_stack_exec::{self, LutStackModel};
use lexlean_r4_wasm::otel::{
    build_inference_trace, compute_uor_span_id, compute_uor_trace_id, create_bootstrap_trace,
};
use serde_json::Value;

const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

fn load_test_model() -> LutStackModel {
    LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully")
}

#[test]
fn test_adversarial_timestamp_fuzzing_extremes() {
    let base_time_ns: u64 = 1_700_000_000_000_000_000;

    let boundary_offsets: [u64; 11] = [
        0,
        1,
        99,
        100,
        101,
        1_000_000_000,
        u64::MAX / 2,
        u64::MAX - 1_000_000_000,
        u64::MAX - 100,
        u64::MAX - 1,
        u64::MAX,
    ];

    let boundary_durations: [u64; 11] = [
        0,
        1,
        99,
        100,
        101,
        1_000_000_000,
        u64::MAX / 2,
        u64::MAX - 1_000_000_000,
        u64::MAX - 100,
        u64::MAX - 1,
        u64::MAX,
    ];

    let base_trace = create_bootstrap_trace();
    assert!(!base_trace.spans.is_empty());

    // 1. Grid of all 11 x 11 = 121 boundary combinations
    for &offset in &boundary_offsets {
        for &dur in &boundary_durations {
            let mut trace = base_trace.clone();
            trace.spans[0].start_offset_ns = offset;
            trace.spans[0].duration_ns = dur;

            // Must NEVER panic under any combination
            let run_res =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| trace.hydrate_to_otlp()));
            assert!(
                run_res.is_ok(),
                "hydrate_to_otlp panicked on offset={offset}, dur={dur}"
            );
            let hydrated = run_res.unwrap();
            let span = &hydrated.resource_spans[0].scope_spans[0].spans[0];

            // Verify saturating arithmetic behavior
            let expected_start = base_time_ns.saturating_add(offset);
            let expected_end = expected_start.saturating_add(dur.max(100));

            assert_eq!(span.start_time_unix_nano, expected_start.to_string());
            assert_eq!(span.end_time_unix_nano, expected_end.to_string());

            // Check to_otlp_json serialization
            let otlp_json = trace.to_otlp_json();
            assert!(otlp_json.contains("resourceSpans"));

            // Check to_combined_value
            let comb = trace.to_combined_value();
            assert!(comb.get("resourceSpans").is_some());
        }
    }

    // 2. Fuzz 1,000 random offset and duration pairs with SplitMix64 PRNG
    let mut state = 0x1234_5678_9abc_def0_u64;
    let mut splitmix64 = || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    };

    for i in 0..1_000 {
        let fuzz_offset = splitmix64();
        let fuzz_dur = splitmix64();

        let mut trace = base_trace.clone();
        trace.spans[0].start_offset_ns = fuzz_offset;
        trace.spans[0].duration_ns = fuzz_dur;

        let run_res =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| trace.hydrate_to_otlp()));
        assert!(
            run_res.is_ok(),
            "fuzz hydration panicked at iteration {i} with offset={fuzz_offset}, dur={fuzz_dur}"
        );
        let hydrated = run_res.unwrap();
        let span = &hydrated.resource_spans[0].scope_spans[0].spans[0];

        let expected_start = base_time_ns.saturating_add(fuzz_offset);
        let expected_end = expected_start.saturating_add(fuzz_dur.max(100));
        assert_eq!(span.start_time_unix_nano, expected_start.to_string());
        assert_eq!(span.end_time_unix_nano, expected_end.to_string());
    }

    println!("Adversarial timestamp fuzzing passed (121 boundary pairs + 1,000 fuzz pairs with 0 panics).");
}

#[test]
fn test_adversarial_compression_ratio_across_variable_turn_lengths() {
    let model = Arc::new(load_test_model());

    let rep_str = "Repeat after me: the quick brown fox jumps over the lazy dog. ".repeat(4);
    let test_prompts = [
        ("minimal_1char", "1"),
        ("short_query", "What is 2+2?"),
        ("standard_math", "Explain Euler's identity in mathematics."),
        ("repetitive_text", rep_str.as_str()),
        (
            "long_technical",
            "Define prime numbers, composite numbers, algebraic geometry, Clifford algebras, multivectors, spin holonomy, and ternary associative memory basins in detail.",
        ),
    ];

    let max_token_limits = [1, 4, 16, 32, 64];

    let mut context = lut_stack_exec::RollingContextState::default();
    let mut failures = Vec::new();
    let mut measurements = Vec::new();

    for (prompt_name, prompt) in &test_prompts {
        for &max_toks in &max_token_limits {
            let ctx_before = context.q12_latent;
            let gen = model.generate_reply_with_context(&mut context, &[], prompt, max_toks);
            let ctx_after = context.q12_latent;

            let trace = build_inference_trace(1, prompt, &ctx_before, &ctx_after, &gen, &model);

            let pct = trace.metrics.savings_percentage;
            measurements.push((
                *prompt_name,
                max_toks,
                trace.metrics.uncompressed_bytes,
                trace.metrics.compressed_bytes,
                pct,
            ));

            let min_required_pct = if max_toks <= 4 { 50.0 } else { 60.0 };
            if pct < min_required_pct {
                failures.push((*prompt_name, max_toks, pct, min_required_pct));
            }

            assert!(
                trace.metrics.saved_bytes_estimate > 0,
                "Saved bytes estimate must be strictly positive"
            );
            assert!(
                trace.metrics.uncompressed_bytes > trace.metrics.compressed_bytes,
                "Uncompressed bytes must exceed compressed bytes"
            );

            // Lossless round-trip check
            let otlp_json = trace.to_otlp_json();
            let parsed: Value = serde_json::from_str(&otlp_json).expect("valid OTLP JSON");
            assert!(parsed.get("resourceSpans").is_some());
        }
    }

    println!("--- Variable Turn Length Compression Measurements ---");
    for (name, toks, uncomp, comp, pct) in &measurements {
        println!(
            "{:<16} | toks={:>2} | uncomp={:>5} B | comp={:>5} B | savings={:>5.2}%",
            name, toks, uncomp, comp, pct
        );
    }

    assert!(
        failures.is_empty(),
        "Compression failures detected: {:?}",
        failures
    );
}

#[test]
fn test_adversarial_10000_permutations_determinism_and_collision_freedom() {
    let mut state = 0x853c_49e6_748f_ea9b_u64;
    let mut splitmix64 = || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    };

    let mut seen_traces = HashSet::with_capacity(10_000);
    let mut seen_spans = HashSet::with_capacity(10_000);
    let mut trace_records = Vec::with_capacity(10_000);
    let mut span_records = Vec::with_capacity(10_000);

    for i in 0..10_000u32 {
        let t = (splitmix64() & 0xffff_ffff) as u32;
        let ctx = splitmix64();
        let prompt_d = splitmix64();

        let (hi, lo, tid) = compute_uor_trace_id(t, ctx, prompt_d);
        assert_eq!(tid.len(), 32);
        assert!(
            tid.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "trace_id must be lowercase hex"
        );
        assert_ne!(hi, 0, "hi must be non-zero");
        assert_ne!(lo, 0, "lo must be non-zero");
        assert!(
            seen_traces.insert(tid.clone()),
            "Collision detected in trace_id at iteration {i}"
        );
        trace_records.push((t, ctx, prompt_d, tid));

        let parent = splitmix64();
        let disc = splitmix64();
        let (h, sid) = compute_uor_span_id(parent, "gllm.decode.step", disc);
        assert_eq!(sid.len(), 16);
        assert!(
            sid.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "span_id must be lowercase hex"
        );
        assert_ne!(h, 0, "span_id must be non-zero");
        assert!(
            seen_spans.insert(sid.clone()),
            "Collision detected in span_id at iteration {i}"
        );
        span_records.push((parent, disc, sid));
    }

    assert_eq!(seen_traces.len(), 10_000);
    assert_eq!(seen_spans.len(), 10_000);

    // Repeatability / Determinism: re-run all 10,000 and ensure bit-for-bit reproduction
    for (t, ctx, prompt_d, expected_tid) in trace_records {
        let (_, _, tid) = compute_uor_trace_id(t, ctx, prompt_d);
        assert_eq!(tid, expected_tid, "Determinism failure on trace_id");
    }

    for (parent, disc, expected_sid) in span_records {
        let (_, sid) = compute_uor_span_id(parent, "gllm.decode.step", disc);
        assert_eq!(sid, expected_sid, "Determinism failure on span_id");
    }

    // Boundary input testing
    let boundary_cases = [
        (0u32, 0u64, 0u64),
        (u32::MAX, u64::MAX, u64::MAX),
        (u32::MAX, 0, u64::MAX),
        (0, u64::MAX, 0),
        (1, 1, 1),
        (100, 0x5555_5555_5555_5555, 0xaaaa_aaaa_aaaa_aaaa),
    ];

    for (t, ctx, p) in boundary_cases {
        let (hi, lo, tid) = compute_uor_trace_id(t, ctx, p);
        assert_ne!(hi, 0);
        assert_ne!(lo, 0);
        assert_eq!(tid.len(), 32);

        let (h1, sid1) = compute_uor_span_id(0, "", 0);
        assert_ne!(h1, 0);
        assert_eq!(sid1.len(), 16);

        let (h2, sid2) = compute_uor_span_id(u64::MAX, "", u64::MAX);
        assert_ne!(h2, 0);
        assert_eq!(sid2.len(), 16);

        let (h3, sid3) = compute_uor_span_id(u64::MAX, "\0\0\0", u64::MAX);
        assert_ne!(h3, 0);
        assert_eq!(sid3.len(), 16);
    }

    println!("Adversarial 10,000 permutations determinism & collision freedom passed cleanly.");
}
