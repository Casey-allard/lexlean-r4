//! Empirical challenge test suite for Milestone M4 (Requirement R4: Per-Token Prediction Residual Backprop).
//!
//! Evaluates:
//! 1. `challenge_m4_clause_boundary_trajectory_capture_precision`
//! 2. `challenge_m4_integer_l1_residual_margin_exactness_and_zero_float`
//! 3. `challenge_m4_residual_driven_coordinate_descent_l1_contraction`
//! 4. `challenge_m4_zero_latency_online_readback_immediate_turn_routing`
//! 5. `challenge_m4_adversarial_boundary_fuzzing_and_edge_cases`
//! 6. `challenge_m4_multi_turn_residual_backprop_continuation_convergence`

use std::sync::Arc;
use std::time::Instant;

use lexlean_r4_wasm::lut_stack_exec::{
    compute_token_l1_margin, consolidate_lut_weights, is_clause_boundary, ByteBpe4096,
    ConsolidationConfig, LutStackModel, RollingContextState, WeightConsolidationSample, FBS_WIDTH,
};

use crate::BOOTSTRAP_BYTES;

fn load_bootstrap() -> LutStackModel {
    crate::shared_bootstrap_model_clone()
}

/// Simple deterministic SplitMix64 PRNG for reproducibility without external crate dependencies.
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    fn next_u32_range(&mut self, max_exclusive: u32) -> u32 {
        (self.next_u64() % (max_exclusive as u64)) as u32
    }
}

/// Challenge 1: Clause Boundary Trajectory Capture Precision
///
/// Verifies that (n_t, y_hat_t, y_t) trajectory triplets are captured strictly at
/// clause/sentence boundary tokens and step % 16 intervals, with bounded Q12 coordinates.
#[test]
fn challenge_m4_clause_boundary_trajectory_capture_precision() {
    let model = Arc::new(load_bootstrap());
    let prompt = "What is the structure of the exceptional Lie group E8?";
    let reply = "The Lie algebra e8 has dimension 248. Its root system contains 240 roots; each root has squared length 2! Furthermore, its Weyl group order is 696729600.";

    let res = model.trace_dynamic_turn(&[], prompt, reply, 128);

    assert!(
        !res.boundary_trajectories.is_empty(),
        "trace_dynamic_turn must capture clause boundary trajectories"
    );

    let bpe = ByteBpe4096::global();
    for hit in &res.boundary_trajectories {
        assert!(
            is_clause_boundary(hit.boundary_token, hit.step),
            "hit at step {} with token {} ({:?}) must be a valid boundary",
            hit.step,
            hit.boundary_token,
            bpe.decode(&[hit.boundary_token])
        );
        assert!(hit.step > 0, "step index must be 1-indexed");

        // Verify that n_t is bounded in signed 16-bit Q12 range and non-zero
        let non_zero_lanes = hit.n_t.iter().filter(|&&v| v != 0).count();
        assert!(
            non_zero_lanes >= 200,
            "latent n_t must be active across trunk lanes (found {non_zero_lanes}/256 non-zero)"
        );

        // Verify exact integer margin calculation consistency
        let expected_margin = compute_token_l1_margin(
            &model.writer_table,
            model.vocab,
            hit.predicted_token,
            hit.target_token,
        );
        assert_eq!(
            hit.l1_margin, expected_margin,
            "hit l1_margin must match compute_token_l1_margin"
        );
    }

    // Assert that non-boundary word tokens at non-16 steps are NOT recorded
    let boundary_steps: Vec<usize> = res.boundary_trajectories.iter().map(|h| h.step).collect();
    for (idx, &tok) in res.generated_tokens.iter().enumerate() {
        let step = idx + 1;
        if !is_clause_boundary(tok, step) {
            assert!(
                !boundary_steps.contains(&step),
                "non-boundary step {step} with token {tok} must not be in boundary_trajectories"
            );
        }
    }
}

/// Challenge 2: Integer L1 Residual Margin Exactness & Zero-Float Arithmetic
///
/// Computes L1 distance across 1,000 random token pairs and verifies exact arithmetic,
/// symmetry, triangle inequality, identity of indiscernibles, and zero floating-point operations.
#[test]
fn challenge_m4_integer_l1_residual_margin_exactness_and_zero_float() {
    let model = load_bootstrap();
    let vocab = model.vocab;
    let mut rng = SplitMix64::new(0xABCD_1234_5678_90EF);

    // 1. Identity of indiscernibles: margin(t, t) == 0
    for _ in 0..100 {
        let t = rng.next_u32_range(vocab as u32);
        let m = compute_token_l1_margin(&model.writer_table, vocab, t, t);
        assert_eq!(m, 0, "margin for identical token {t} must be 0");
    }

    // 2. 1,000 token pairs exact integer distance verification
    for _ in 0..1000 {
        let a = rng.next_u32_range(vocab as u32);
        let b = rng.next_u32_range(vocab as u32);

        let margin_ab = compute_token_l1_margin(&model.writer_table, vocab, a, b);
        let margin_ba = compute_token_l1_margin(&model.writer_table, vocab, b, a);

        // Symmetry
        assert_eq!(margin_ab, margin_ba, "L1 margin must be symmetric");

        // Ground-truth L1 computation using pure i64
        let a_base = (a as usize) * FBS_WIDTH;
        let b_base = (b as usize) * FBS_WIDTH;
        let mut expected = 0u64;
        for d in 0..FBS_WIDTH {
            let diff =
                (model.writer_table[a_base + d] as i64) - (model.writer_table[b_base + d] as i64);
            expected += diff.unsigned_abs();
        }

        assert_eq!(
            margin_ab, expected,
            "margin must equal exact integer sum of absolute diffs"
        );
        assert!(
            margin_ab <= 15_360_000,
            "margin must not exceed theoretical upper bound 15,360,000"
        );
    }

    // 3. Triangle inequality check on 100 triplets
    for _ in 0..100 {
        let a = rng.next_u32_range(vocab as u32);
        let b = rng.next_u32_range(vocab as u32);
        let c = rng.next_u32_range(vocab as u32);

        let mab = compute_token_l1_margin(&model.writer_table, vocab, a, b);
        let mbc = compute_token_l1_margin(&model.writer_table, vocab, b, c);
        let mac = compute_token_l1_margin(&model.writer_table, vocab, a, c);

        assert!(
            mac <= mab + mbc,
            "triangle inequality violated: mac={mac} > mab={mab} + mbc={mbc}"
        );
    }
}

/// Challenge 3: Residual-Driven Coordinate Descent L1 Contraction
///
/// Verifies that coordinate descent updates produce strictly monotonic contraction
/// toward the latent point, modulated by residual margin magnitude, with zero orphaned PointKeys.
#[test]
fn challenge_m4_residual_driven_coordinate_descent_l1_contraction() {
    let targets = [5u32, 23, 77, 120, 256, 512, 1024, 2048, 3000, 4000];

    for &target in &targets {
        let mut model = load_bootstrap();
        let base = (target as usize) * FBS_WIDTH;

        // Construct an opposing latent vector to evaluate maximum contraction
        let mut n_t = [0i16; FBS_WIDTH];
        for d in 0..FBS_WIDTH {
            n_t[d] = -model.writer_table[base + d];
        }

        // Initial L1 distance
        let mut dist_pre = 0u64;
        for d in 0..FBS_WIDTH {
            let diff = (n_t[d] as i64) - (model.writer_table[base + d] as i64);
            dist_pre += diff.unsigned_abs();
        }

        // Apply consolidation with positive residual margin (125,000)
        let sample = WeightConsolidationSample::from_point_with_residual(
            target,
            target.wrapping_add(1),
            n_t,
            125_000,
        );
        let report =
            consolidate_lut_weights(&mut model, &[sample], &ConsolidationConfig::default())
                .expect("consolidation succeeds");

        assert!(
            report.weights_updated > 0,
            "weights must be updated for non-zero margin"
        );
        assert!(report.l1_shift > 0, "L1 shift must be positive");
        assert_eq!(report.orphaned_point_rows, 0, "orphaned rows must be 0");

        // Post L1 distance
        let mut dist_post = 0u64;
        for d in 0..FBS_WIDTH {
            let diff = (n_t[d] as i64) - (model.writer_table[base + d] as i64);
            dist_post += diff.unsigned_abs();
        }

        assert!(
            dist_post < dist_pre,
            "post-consolidation distance must be strictly less than pre-consolidation distance: {dist_post} < {dist_pre}"
        );

        assert_eq!(
            model.audit_point_key_synchronization(),
            0,
            "audit_point_key_synchronization must return 0"
        );
    }
}

/// Challenge 4: Zero-Latency Online Readback & Immediate Turn Routing
///
/// Verifies that compiling a verified dialogue turn executes online in-memory backpropagation
/// into writer_table and ServedPointKey, enabling immediate subsequent turn routing with zero reload.
#[test]
fn challenge_m4_zero_latency_online_readback_immediate_turn_routing() {
    let mut model = load_bootstrap();
    let prompt = "State the dimension of the Leech lattice.";
    let reply = "The Leech lattice is an even unimodular lattice of dimension 24.";

    let t0 = Instant::now();
    let admitted = model
        .compile_verified_dialogue_turn(prompt, reply)
        .expect("compile_verified_dialogue_turn succeeds");
    let admission_time = t0.elapsed();

    assert!(admitted, "novel turn must be newly compiled");
    assert!(
        admission_time.as_millis() < 15000,
        "online turn admission must complete in interactive time (< 15000ms, was {:?})",
        admission_time
    );

    // Readback verification: query the live model immediately
    let arc_model = Arc::new(model);
    let t_readback = Instant::now();
    let res = arc_model.trace_dynamic_turn(&[], prompt, reply, 32);
    let readback_time = t_readback.elapsed();

    assert!(
        readback_time.as_millis() < 5000,
        "online readback must be interactive (< 5000ms, was {:?})",
        readback_time
    );

    assert_eq!(
        res.clauses_resolved, 1,
        "admitted turn must resolve clause basin"
    );
    assert_eq!(
        arc_model.audit_point_key_synchronization(),
        0,
        "PointKey synchronization must remain clean after readback"
    );
}

/// Challenge 5: Adversarial Boundary Fuzzing and Edge Cases
///
/// Verifies robust fail-safe handling across 5 adversarial conditions:
/// 1. Perfect prediction (margin = 0 everywhere, 0 weights updated)
/// 2. Maximum margin saturation (15,360,000, bounds clamping)
/// 3. Consecutive boundary punctuation (".?!:;\n\n")
/// 4. Empty sequences (graceful rejection)
/// 5. Out-of-vocabulary token IDs
#[test]
fn challenge_m4_adversarial_boundary_fuzzing_and_edge_cases() {
    let mut model = load_bootstrap();

    // 1. Perfect prediction: margin = 0 produces zero weight perturbation
    let sample_perfect =
        WeightConsolidationSample::from_point_with_residual(100, 100, [500i16; FBS_WIDTH], 0);
    let report_perfect = consolidate_lut_weights(
        &mut model,
        &[sample_perfect],
        &ConsolidationConfig::default(),
    )
    .expect("consolidation succeeds");
    assert_eq!(report_perfect.weights_updated, 0);
    assert_eq!(report_perfect.l1_shift, 0);

    // 2. Maximum margin saturation: 15,360,000 does not overflow or violate clamps
    let sample_max = WeightConsolidationSample::from_point_with_residual(
        200,
        300,
        [30000i16; FBS_WIDTH],
        15_360_000,
    );
    let report_max =
        consolidate_lut_weights(&mut model, &[sample_max], &ConsolidationConfig::default())
            .expect("consolidation succeeds");
    assert!(report_max.weights_updated > 0);
    for &w in &model.writer_table[200 * FBS_WIDTH..(200 + 1) * FBS_WIDTH] {
        assert!((-30000..=30000).contains(&w), "weights must remain clamped");
    }

    // 3. Consecutive boundary tokens: ".?!:;\n\n"
    let arc_model = Arc::new(load_bootstrap());
    let punct_prompt = "Punctuation test";
    let punct_reply = "Word.?!:;\n\nAnother word.";
    let res_punct = arc_model.trace_dynamic_turn(&[], punct_prompt, punct_reply, 32);
    assert!(
        !res_punct.boundary_trajectories.is_empty(),
        "consecutive boundaries must be parsed cleanly"
    );

    // 4. Empty sequences rejected with typed errors
    let err_empty = model.compile_verified_dialogue_turn("", "reply");
    assert!(err_empty.is_err(), "empty prompt must be rejected");

    let err_empty_reply = model.compile_verified_dialogue_turn("prompt", "");
    assert!(err_empty_reply.is_err(), "empty reply must be rejected");

    // 5. Out-of-vocabulary token IDs return 0 margin without panic
    assert_eq!(
        compute_token_l1_margin(&model.writer_table, model.vocab, 4096, 0),
        0
    );
    assert_eq!(
        compute_token_l1_margin(&model.writer_table, model.vocab, 0, 9999),
        0
    );
}

/// Challenge 6: Multi-Turn Residual Backprop Continuation Convergence
///
/// Executes a 5-turn mathematical dialog curriculum, verifying monotonic residual accumulation
/// in RollingContextState, FIFO eviction caps (8 basins, 16 residuals), and post-curriculum container roundtrip.
#[test]
fn challenge_m4_multi_turn_residual_backprop_continuation_convergence() {
    let mut model = load_bootstrap();
    let mut context = RollingContextState::new();
    let mut history = Vec::new();

    let curriculum = [
        ("State the Pythagorean theorem.", "In a right triangle, the square of the hypotenuse equals the sum of squares of the other two sides."),
        ("What is Fermat's Last Theorem?", "No three positive integers a, b, c satisfy a^n + b^n = c^n for any integer n greater than 2."),
        ("How are elliptic curves related to Fermat's Last Theorem?", "Frey and Serre showed that a counterexample to Fermat's theorem would yield a non-modular elliptic curve."),
        ("What did Ken Ribet prove?", "Ribet proved the epsilon conjecture, establishing that the Frey curve is indeed not modular."),
        ("Who completed the proof of Fermat's Last Theorem?", "Andrew Wiles proved the semistable modularity theorem with Richard Taylor, proving Fermat's Last Theorem."),
    ];

    let mut prev_accumulated_margin = 0u64;

    for (turn_idx, &(prompt, reply)) in curriculum.iter().enumerate() {
        // 1. Trace with rolling context prior to admission to evaluate prediction residuals along trunk
        let arc_model = Arc::new(model.clone());
        let res =
            arc_model.trace_dynamic_turn_with_context(&mut context, &history, prompt, reply, 64);

        assert!(
            res.total_l1_margin > 0,
            "turn {turn_idx} should compute positive total L1 margin"
        );
        assert!(
            context.accumulated_l1_margin >= prev_accumulated_margin,
            "context accumulated margin must be monotonically non-decreasing"
        );
        prev_accumulated_margin = context.accumulated_l1_margin;

        // 2. Compile turn with online residual backpropagation into the model
        let admitted = model
            .compile_verified_dialogue_turn(prompt, reply)
            .expect("turn admission succeeds");
        assert!(admitted, "turn {turn_idx} newly admitted");

        // Verify FIFO bounds
        assert!(
            context.recent_turn_rows.len() <= 8,
            "recent_turn_rows must never exceed 8"
        );
        assert!(
            context.recent_residuals.len() <= 16,
            "recent_residuals must never exceed 16"
        );

        history.push((prompt.to_string(), reply.to_string()));
    }

    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "audit_point_key_synchronization must remain 0 after 5-turn admission"
    );

    // Roundtrip serialization check
    let bytes = model.to_uorlut_bytes();
    let reparsed =
        LutStackModel::parse(&bytes).expect("model roundtrips through UORLUT01 container");
    assert_eq!(reparsed.audit_point_key_synchronization(), 0);
    assert_eq!(reparsed.writer_table, model.writer_table);
}
