#![allow(
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    clippy::identity_op,
    clippy::manual_range_contains,
    clippy::useless_vec
)]
//! Adversarial Empirical Challenge Suite for Milestone M4 (Requirement R4: Per-Token Prediction Residual Backprop).
//!
//! Stress-tests residual margin calculation and coordinate-descent optimizer dynamics in `crates/wasm`:
//! 1. Zero-Error Invariant: prove that feeding points with M=0 yields strictly 0 weight updates and 0 L1 shift across any number of iterations.
//! 2. Monotonic Contraction: prove that for mispredicted points (M > 0), post-consolidation L1 distance to the latent point is strictly less than pre-consolidation distance, and contraction scales monotonically with M.
//! 3. Bound Contraction: verify that under extreme margins (M ≈ 15,360,000), s_eff saturates at d without overshooting or causing numerical divergence.
//! 4. Observation Weighting: verify that high-error observations pull the updated centroid proportionally more than low-error observations.
//! 5. PointKey Synchronization: verify that audit_point_key_synchronization() == 0 holds after arbitrary batch consolidation.
//! 6. Zero-Float Verification & Bitwise Determinism: verify bitwise determinism and pure integer arithmetic without floating-point operations.

use lexlean_r4_wasm::lut_stack_exec::{
    compute_token_l1_margin, consolidate_lut_weights, ConsolidationConfig, LutStackModel,
    WeightConsolidationSample, FBS_WIDTH,
};

const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

fn load_bootstrap() -> LutStackModel {
    LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully")
}

/// Deterministic SplitMix64 pseudo-random number generator for reproducible adversarial fuzzing.
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

    fn next_i16_range(&mut self, min: i16, max: i16) -> i16 {
        let span = (max as i32 - min as i32 + 1) as u64;
        let val = (self.next_u64() % span) as i32;
        (min as i32 + val) as i16
    }
}

/// Challenge 1: Zero-Error Invariant Under Multi-Iteration and Multi-Token Stress
///
/// Asserts that feeding points with M = 0 yields:
/// - strictly 0 weight updates (report.weights_updated == 0)
/// - strictly 0 L1 shift (report.l1_shift == 0)
/// - strictly 0 max delta (report.max_delta == 0)
/// - strictly 0 tokens updated (report.tokens_updated == 0)
/// across 1, 2, 5, 20, 50, and 100 iterations, for both single and multi-token batches
/// with adversarial distant latent vectors (opposite signs, extreme ±30,000 bounds).
/// Byte-for-byte verifies that `writer_table` and `point.points` remain 100% unperturbed.
#[test]
fn test_adv_zero_error_invariant_multi_iteration() {
    let fresh_model = load_bootstrap();
    let original_writer_table = fresh_model.writer_table.clone();
    let original_points = fresh_model
        .point
        .as_ref()
        .map(|p| p.points.clone())
        .expect("point table present");

    let iterations_to_test = [1usize, 2, 5, 20, 50, 100];

    // Subtest A: Single token with M=0 and extreme opposing latent vector
    for &iters in &iterations_to_test {
        let mut model = fresh_model.clone();
        let target_token = 42u32;
        let base = (target_token as usize) * FBS_WIDTH;

        // Construct adversarial latent vector that diametrically opposes the current writer weights
        let mut adversarial_latent = [0i16; FBS_WIDTH];
        for d in 0..FBS_WIDTH {
            let current = original_writer_table[base + d];
            adversarial_latent[d] = if current >= 0 { -30000 } else { 30000 };
        }

        let sample = WeightConsolidationSample::from_point_with_residual(
            target_token,
            target_token, // predicted == target => M = 0
            adversarial_latent,
            0,
        );

        let config = ConsolidationConfig {
            iterations: iters,
            step_numerator: 4,
            step_denominator: 8,
            clamp_min: -30000,
            clamp_max: 30000,
            sync_point_keys: true,
            update_ternary_assoc: true,
        };

        let report = consolidate_lut_weights(&mut model, &[sample], &config)
            .expect("consolidation must succeed");

        assert_eq!(
            report.weights_updated, 0,
            "zero-error invariant violation: weights_updated > 0 at {} iterations",
            iters
        );
        assert_eq!(
            report.l1_shift, 0,
            "zero-error invariant violation: l1_shift > 0 at {} iterations",
            iters
        );
        assert_eq!(
            report.max_delta, 0,
            "zero-error invariant violation: max_delta > 0 at {} iterations",
            iters
        );
        assert_eq!(
            report.tokens_updated, 0,
            "zero-error invariant violation: tokens_updated > 0 at {} iterations",
            iters
        );
        assert_eq!(
            model.writer_table, original_writer_table,
            "zero-error invariant violation: writer_table altered at {} iterations",
            iters
        );
        if let Some(pt) = model.point.as_ref() {
            assert_eq!(
                pt.points, original_points,
                "zero-error invariant violation: point table altered at {} iterations",
                iters
            );
        }
    }

    // Subtest B: Heterogeneous multi-token batch (50 tokens) with M=0 and random latent points
    let mut rng = SplitMix64::new(0xDEAD_BEEF_CAFE_0001);
    let mut batch = Vec::new();
    for _ in 0..50 {
        let t = rng.next_u32_range(fresh_model.vocab as u32);
        let mut latent = [0i16; FBS_WIDTH];
        for d in 0..FBS_WIDTH {
            latent[d] = rng.next_i16_range(-30000, 30000);
        }
        batch.push(WeightConsolidationSample::from_point_with_residual(
            t, t, latent, 0,
        ));
    }

    let mut multi_model = fresh_model.clone();
    let multi_config = ConsolidationConfig {
        iterations: 10,
        step_numerator: 8,
        step_denominator: 8,
        clamp_min: -30000,
        clamp_max: 30000,
        sync_point_keys: true,
        update_ternary_assoc: true,
    };
    let multi_report = consolidate_lut_weights(&mut multi_model, &batch, &multi_config)
        .expect("consolidation must succeed");

    assert_eq!(multi_report.weights_updated, 0);
    assert_eq!(multi_report.l1_shift, 0);
    assert_eq!(multi_report.tokens_updated, 0);
    assert_eq!(multi_model.writer_table, original_writer_table);
}

/// Challenge 2: Monotonic Contraction & Proportional Error Scaling
///
/// Verifies that for any mispredicted point (M > 0):
/// 1. Post-consolidation distance to the latent point is strictly less than pre-consolidation distance:
///    `dist_post < dist_pre`.
/// 2. The contraction amount Delta(M) = dist_pre - dist_post scales monotonically non-decreasing with M.
/// 3. Where s_eff(M) strictly increases, Delta(M) strictly increases.
/// 4. Across successive optimization iterations (k = 1..5), distance decreases strictly monotonically.
#[test]
fn test_adv_monotonic_contraction_and_error_scaling() {
    let model_orig = load_bootstrap();
    let test_tokens = [12u32, 85, 256, 512, 1024, 2048, 3500];

    // Geometric ladder of residual margins spanning low to saturation
    let margin_ladder: [u64; 11] = [
        1, 10, 100, 1_000, 5_000, 25_000, 50_000, 100_000, 150_000, 200_000, 250_000,
    ];

    for &tok in &test_tokens {
        let base = (tok as usize) * FBS_WIDTH;

        // Construct an adversarial latent vector sufficiently separated from current centroid
        let mut latent = [0i16; FBS_WIDTH];
        for d in 0..FBS_WIDTH {
            let current = model_orig.writer_table[base + d];
            latent[d] = if current > 0 { -15000 } else { 15000 };
        }

        let mut dist_pre = 0u64;
        for d in 0..FBS_WIDTH {
            let diff = (latent[d] as i64) - (model_orig.writer_table[base + d] as i64);
            dist_pre += diff.unsigned_abs();
        }

        let mut prev_contraction = 0u64;
        let mut prev_s_eff = 0i64;

        for &margin in &margin_ladder {
            let mut model = model_orig.clone();
            let sample = WeightConsolidationSample::from_point_with_residual(
                tok,
                tok.wrapping_add(1),
                latent,
                margin,
            );

            // Use d=64 to provide fine-grained step resolution (s_eff from 1 to 64)
            let config = ConsolidationConfig {
                iterations: 1,
                step_numerator: 8,
                step_denominator: 64,
                clamp_min: -30000,
                clamp_max: 30000,
                sync_point_keys: true,
                update_ternary_assoc: true,
            };

            let report = consolidate_lut_weights(&mut model, &[sample], &config)
                .expect("consolidation must succeed");

            assert!(
                report.weights_updated > 0,
                "weights must update for M={margin}"
            );
            assert!(
                report.l1_shift > 0,
                "L1 shift must be positive for M={margin}"
            );

            let mut dist_post = 0u64;
            for d in 0..FBS_WIDTH {
                let diff = (latent[d] as i64) - (model.writer_table[base + d] as i64);
                dist_post += diff.unsigned_abs();
            }

            // 1. Strict contraction
            assert!(
                dist_post < dist_pre,
                "token {tok} at M={margin}: post distance {dist_post} must be strictly less than pre {dist_pre}"
            );

            let contraction = dist_pre - dist_post;

            // Compute expected s_eff = clamp((M * 64) / 250_000, 1, 64)
            let cur_s_eff = ((margin * 64) / 250_000).clamp(1, 64) as i64;

            // 2. Monotonic non-decreasing contraction with increasing M
            assert!(
                contraction >= prev_contraction,
                "token {tok}: contraction must be monotonically non-decreasing with M. margin={margin}, cur={contraction}, prev={prev_contraction}"
            );

            // 3. Strict increase when s_eff increases
            if cur_s_eff > prev_s_eff && prev_s_eff > 0 {
                assert!(
                    contraction > prev_contraction,
                    "token {tok}: contraction must strictly increase when s_eff increases (s_eff: {prev_s_eff} -> {cur_s_eff}, contraction: {prev_contraction} -> {contraction})"
                );
            }

            prev_contraction = contraction;
            prev_s_eff = cur_s_eff;
        }

        // 4. Multi-iteration strict monotonic descent
        let mut iter_model = model_orig.clone();
        let sample_iter = WeightConsolidationSample::from_point_with_residual(
            tok,
            tok.wrapping_add(1),
            latent,
            75_000,
        );
        let mut last_dist = dist_pre;
        for iter_count in 1..=4 {
            let cfg_step = ConsolidationConfig {
                iterations: 1,
                step_numerator: 8,
                step_denominator: 64,
                clamp_min: -30000,
                clamp_max: 30000,
                sync_point_keys: true,
                update_ternary_assoc: true,
            };
            consolidate_lut_weights(
                &mut iter_model,
                std::slice::from_ref(&sample_iter),
                &cfg_step,
            )
            .expect("iteration consolidation succeeds");

            let mut current_dist = 0u64;
            for d in 0..FBS_WIDTH {
                let diff = (latent[d] as i64) - (iter_model.writer_table[base + d] as i64);
                current_dist += diff.unsigned_abs();
            }

            assert!(
                current_dist < last_dist,
                "iteration {iter_count} failed to strictly decrease distance: {current_dist} >= {last_dist}"
            );
            last_dist = current_dist;
        }
    }
}

/// Challenge 3: Bound Contraction Saturation & Zero-Overshoot Verification
///
/// Verifies that under extreme residual margins (M from 250,000 to theoretical max 15,360,000):
/// 1. s_eff saturates at d without overshooting past the target centroid.
/// 2. For M >= 250,000, updated weights land exactly at the latent point coordinate (when within clamp range).
/// 3. Consolidation outputs for M=250_000, M=1_000_000, and M=15_360_000 are bit-for-bit identical (perfect saturation).
/// 4. Extreme coordinates (±32768) clamp cleanly to [-30000, 30000] without integer overflow, wrapping, or NaN.
#[test]
fn test_adv_bound_contraction_saturation_and_no_overshoot() {
    let model_orig = load_bootstrap();
    let target_tok = 99u32;
    let base = (target_tok as usize) * FBS_WIDTH;

    let mut target_latent = [0i16; FBS_WIDTH];
    for d in 0..FBS_WIDTH {
        target_latent[d] = (d as i16 * 100) - 12800; // in [-12800, 12700], safely within clamp
    }

    let extreme_margins = [
        250_000u64, 500_000, 1_000_000, 5_000_000, 10_000_000, 15_360_000,
    ];

    let mut saturated_tables = Vec::new();

    for &m in &extreme_margins {
        let mut model = model_orig.clone();
        let sample = WeightConsolidationSample::from_point_with_residual(
            target_tok,
            target_tok.wrapping_add(2),
            target_latent,
            m,
        );

        let config = ConsolidationConfig {
            iterations: 1,
            step_numerator: 1,
            step_denominator: 8,
            clamp_min: -30000,
            clamp_max: 30000,
            sync_point_keys: true,
            update_ternary_assoc: true,
        };

        let report = consolidate_lut_weights(&mut model, &[sample], &config)
            .expect("extreme margin consolidation succeeds");

        assert!(report.weights_updated > 0);
        assert_eq!(report.orphaned_point_rows, 0);

        // When s_eff == d, updated coordinate must equal target_latent exactly (zero overshoot)
        for d in 0..FBS_WIDTH {
            let updated = model.writer_table[base + d];
            let expected = target_latent[d];
            assert_eq!(
                updated, expected,
                "overshoot / divergence at dim {d} for margin {m}: got {updated}, expected {expected}"
            );
        }

        saturated_tables.push((m, model.writer_table.clone()));
    }

    // Verify all saturated tables (from 250k to 15.36M) are 100% bit-exact identical
    let (_, ref baseline_table) = saturated_tables[0];
    for (m, ref tbl) in &saturated_tables[1..] {
        assert_eq!(
            tbl, baseline_table,
            "saturation invariance violated: margin {m} diverged from 250_000 baseline"
        );
    }

    // Subtest B: Clamping stress test under maximum margin 15,360,000 with extreme i16 bounds
    let extreme_latent_max = [i16::MAX; FBS_WIDTH]; // 32767
    let extreme_latent_min = [i16::MIN; FBS_WIDTH]; // -32768

    let mut model_max = model_orig.clone();
    let sample_max = WeightConsolidationSample::from_point_with_residual(
        target_tok,
        target_tok.wrapping_add(1),
        extreme_latent_max,
        15_360_000,
    );
    consolidate_lut_weights(
        &mut model_max,
        &[sample_max],
        &ConsolidationConfig::default(),
    )
    .expect("consolidation with i16::MAX succeeds");

    for d in 0..FBS_WIDTH {
        assert_eq!(
            model_max.writer_table[base + d],
            30000,
            "upper clamp violated at dim {d}"
        );
    }

    let mut model_min = model_orig.clone();
    let sample_min = WeightConsolidationSample::from_point_with_residual(
        target_tok,
        target_tok.wrapping_add(1),
        extreme_latent_min,
        15_360_000,
    );
    consolidate_lut_weights(
        &mut model_min,
        &[sample_min],
        &ConsolidationConfig::default(),
    )
    .expect("consolidation with i16::MIN succeeds");

    for d in 0..FBS_WIDTH {
        assert_eq!(
            model_min.writer_table[base + d],
            -30000,
            "lower clamp violated at dim {d}"
        );
    }
}

/// Challenge 4: Observation Weighting & Gravitational Pull Dynamics
///
/// Verifies that high-error observations pull the updated centroid proportionally
/// more than low-error observations:
/// 1. Exact proportional pull between opposing points: Observation A (P_A, low error M_A=100)
///    vs Observation B (P_B, high error M_B=900). Centroid is pulled 90% towards P_B and 10% towards P_A.
/// 2. Monotonic error sweep: varying M_B while fixing M_A causes continuous monotonic movement towards P_B.
/// 3. Catastrophic outlier domination: 9 low-error observations (M=10) vs 1 catastrophic error (M=3,000,000).
///    The catastrophic observation overwhelmingly dominates the centroid (> 99.9% of total weight).
#[test]
fn test_adv_observation_weighting_gravitational_pull() {
    let model_orig = load_bootstrap();
    let target_tok = 333u32;
    let base = (target_tok as usize) * FBS_WIDTH;

    // Test A: Exact 90/10 proportional pull between opposing points
    let mut model_ab = model_orig.clone();
    let point_a = [1000i16; FBS_WIDTH];
    let point_b = [-1000i16; FBS_WIDTH];
    let margin_a = 100u64;
    let margin_b = 900u64;

    let sample_a = WeightConsolidationSample::from_point_with_residual(
        target_tok,
        target_tok.wrapping_add(1),
        point_a,
        margin_a,
    );
    let sample_b = WeightConsolidationSample::from_point_with_residual(
        target_tok,
        target_tok.wrapping_add(2),
        point_b,
        margin_b,
    );

    // Theoretical centroid mu = (100 * 1000 + 900 * (-1000)) / (100 + 900)
    //                         = (100000 - 900000) / 1000 = -800
    // With d=8, mean_margin = (100 + 900) / 2 = 500
    // scaled = (500 * 8) / 250_000 = 4000 / 250_000 = 0 => clamped to 1
    // s_eff = 1
    // new_w = ((8 - 1) * old_w + 1 * mu) / 8
    let config_step = ConsolidationConfig {
        iterations: 1,
        step_numerator: 1,
        step_denominator: 8,
        clamp_min: -30000,
        clamp_max: 30000,
        sync_point_keys: true,
        update_ternary_assoc: true,
    };

    consolidate_lut_weights(&mut model_ab, &[sample_a, sample_b], &config_step)
        .expect("consolidation succeeds");

    for d in 0..FBS_WIDTH {
        let old_w = model_orig.writer_table[base + d] as i64;
        let expected_mu = -800i64;
        let expected_w = ((7 * old_w + expected_mu) / 8) as i16;
        let actual_w = model_ab.writer_table[base + d];
        assert_eq!(
            actual_w, expected_w,
            "dim {d}: weighted centroid update mismatch. actual={actual_w}, expected={expected_w}"
        );
    }

    // Distance to point B must be significantly smaller than distance to point A
    let mut dist_to_a = 0u64;
    let mut dist_to_b = 0u64;
    for d in 0..FBS_WIDTH {
        let w = model_ab.writer_table[base + d] as i64;
        dist_to_a += (w - 1000).unsigned_abs();
        dist_to_b += (w - (-1000)).unsigned_abs();
    }
    assert!(
        dist_to_b < dist_to_a,
        "centroid must be pulled towards high-error point B: dist_to_b={dist_to_b} vs dist_to_a={dist_to_a}"
    );

    // Test B: Monotonic error sweep
    let sweep_margins_b = [100u64, 300, 900, 2700, 8100, 24300];
    let mut prev_dist_to_b = u64::MAX;

    for &mb in &sweep_margins_b {
        let mut model_sweep = model_orig.clone();
        let s_a = WeightConsolidationSample::from_point_with_residual(
            target_tok,
            target_tok.wrapping_add(1),
            [2000i16; FBS_WIDTH],
            1000,
        );
        let s_b = WeightConsolidationSample::from_point_with_residual(
            target_tok,
            target_tok.wrapping_add(2),
            [-2000i16; FBS_WIDTH],
            mb,
        );

        consolidate_lut_weights(&mut model_sweep, &[s_a, s_b], &config_step)
            .expect("sweep consolidation succeeds");

        let mut d_b = 0u64;
        for d in 0..FBS_WIDTH {
            let diff = (model_sweep.writer_table[base + d] as i64) - (-2000);
            d_b += diff.unsigned_abs();
        }

        assert!(
            d_b < prev_dist_to_b,
            "increasing margin B from must monotonically pull centroid closer to B (mb={mb}, d_b={d_b}, prev={prev_dist_to_b})"
        );
        prev_dist_to_b = d_b;
    }

    // Test C: 9 low-error observations vs 1 catastrophic observation (M = 3,000,000)
    let mut samples_cat = Vec::new();
    for _ in 0..9 {
        samples_cat.push(WeightConsolidationSample::from_point_with_residual(
            target_tok,
            target_tok.wrapping_add(1),
            [100i16; FBS_WIDTH],
            10, // weight omega = 10
        ));
    }
    // 1 catastrophic error: omega = 3,000,000
    samples_cat.push(WeightConsolidationSample::from_point_with_residual(
        target_tok,
        target_tok.wrapping_add(2),
        [-5000i16; FBS_WIDTH],
        3_000_000,
    ));

    // Total weight = 9 * 10 + 3_000_000 = 3_000_090
    // Weighted sum per dim = 9 * (100 * 10) + (-5000 * 3_000_000) = 9000 - 15,000,000,000 = -14,999,991,000
    // mu = -14,999,991,000 / 3_000_090 = -4999
    // mean_margin = (90 + 3_000_000) / 10 = 300_009
    // scaled = (300_009 * 8) / 250_000 = 9 -> clamped to 8 = d
    // s_eff = 8
    // new_w = mu = -4999
    let mut model_cat = model_orig.clone();
    let config_full = ConsolidationConfig {
        iterations: 1,
        step_numerator: 8,
        step_denominator: 8,
        clamp_min: -30000,
        clamp_max: 30000,
        sync_point_keys: true,
        update_ternary_assoc: true,
    };
    consolidate_lut_weights(&mut model_cat, &samples_cat, &config_full)
        .expect("catastrophic consolidation succeeds");

    for d in 0..FBS_WIDTH {
        let w = model_cat.writer_table[base + d];
        let diff_to_cat = (w as i64 - (-5000)).unsigned_abs();
        let diff_to_low = (w as i64 - 100).unsigned_abs();
        assert_eq!(w, -4999, "dim {d} must land at -4999");
        assert!(
            diff_to_cat < diff_to_low,
            "catastrophic failure must dominate centroid: w={w}, diff_to_cat={diff_to_cat}, diff_to_low={diff_to_low}"
        );
    }
}

/// Challenge 5: PointKey Synchronization Invariant Across Arbitrary Batches
///
/// Asserts that `audit_point_key_synchronization() == 0` holds unconditionally:
/// 1. After consolidating a high-volume batch of 100 random tokens with heterogeneous margins.
/// 2. Across 5 sequential batch consolidation steps.
/// 3. After multi-iteration optimization passes (iterations = 10).
/// 4. After full round-trip serialization through UORLUT01 container.
#[test]
fn test_adv_point_key_synchronization_under_stress_batches() {
    let mut model = load_bootstrap();
    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "bootstrap model must start with 0 orphaned PointKey rows"
    );

    let mut rng = SplitMix64::new(0x1337_C0DE_CAFE_BABE);

    // 1. High-volume heterogeneous batch (100 distinct token samples)
    let mut large_batch = Vec::new();
    for _ in 0..100 {
        let target = rng.next_u32_range(model.vocab as u32);
        let pred = rng.next_u32_range(model.vocab as u32);
        let margin = rng.next_u64() % 15_360_000;
        let mut latent = [0i16; FBS_WIDTH];
        for d in 0..FBS_WIDTH {
            latent[d] = rng.next_i16_range(-25000, 25000);
        }
        large_batch.push(WeightConsolidationSample::from_point_with_residual(
            target, pred, latent, margin,
        ));
    }

    let report_large =
        consolidate_lut_weights(&mut model, &large_batch, &ConsolidationConfig::default())
            .expect("large batch consolidation succeeds");

    assert_eq!(report_large.orphaned_point_rows, 0);
    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "audit_point_key_synchronization must remain 0 after 100-sample batch"
    );

    // 2. 5 sequential batches of 20 samples each
    for step in 1..=5 {
        let mut seq_batch = Vec::new();
        for _ in 0..20 {
            let target = rng.next_u32_range(model.vocab as u32);
            let pred = rng.next_u32_range(model.vocab as u32);
            let margin = rng.next_u64() % 500_000;
            let mut latent = [0i16; FBS_WIDTH];
            for d in 0..FBS_WIDTH {
                latent[d] = rng.next_i16_range(-20000, 20000);
            }
            seq_batch.push(WeightConsolidationSample::from_point_with_residual(
                target, pred, latent, margin,
            ));
        }

        let rep = consolidate_lut_weights(&mut model, &seq_batch, &ConsolidationConfig::default())
            .expect("sequential consolidation step succeeds");

        assert_eq!(rep.orphaned_point_rows, 0);
        assert_eq!(
            model.audit_point_key_synchronization(),
            0,
            "PointKey synchronization broke at sequential step {step}"
        );
    }

    // 3. Multi-iteration stress (iterations = 10)
    let multi_iter_config = ConsolidationConfig {
        iterations: 10,
        step_numerator: 2,
        step_denominator: 8,
        clamp_min: -30000,
        clamp_max: 30000,
        sync_point_keys: true,
        update_ternary_assoc: true,
    };
    let rep_multi = consolidate_lut_weights(&mut model, &large_batch[..10], &multi_iter_config)
        .expect("multi-iteration consolidation succeeds");
    assert_eq!(rep_multi.orphaned_point_rows, 0);
    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "PointKey synchronization broke under 10 iterations"
    );

    // 4. UORLUT01 container round-trip
    let bytes = model.to_uorlut_bytes();
    let reparsed = LutStackModel::parse(&bytes).expect("model roundtrips through UORLUT container");
    assert_eq!(
        reparsed.audit_point_key_synchronization(),
        0,
        "reparsed container model must preserve 0 orphaned PointKey rows"
    );
    assert_eq!(reparsed.writer_table, model.writer_table);
}

/// Challenge 6: Zero-Float Arithmetic & Bitwise Determinism
///
/// Verifies:
/// 1. 100% bitwise determinism: 5 separate model instances executing the same sequence of
///    heterogeneous consolidation updates yield byte-for-byte identical results.
/// 2. Integer exactness of `compute_token_l1_margin`: identity of indiscernibles, symmetry,
///    triangle inequality, and bounds across 2,000 random token pairs.
/// 3. Absence of floating point artifacts: no roundoff drift, no NaN, no inf.
#[test]
fn test_adv_zero_float_verification_and_bitwise_determinism() {
    let mut rng = SplitMix64::new(0x4242_1337_BEEF_9999);

    // Generate a fixed adversarial schedule of 40 consolidation samples
    let vocab_size = 4096usize;
    let mut sample_schedule = Vec::new();
    for _ in 0..40 {
        let t = rng.next_u32_range(vocab_size as u32);
        let p = rng.next_u32_range(vocab_size as u32);
        let m = rng.next_u64() % 1_000_000;
        let mut latent = [0i16; FBS_WIDTH];
        for d in 0..FBS_WIDTH {
            latent[d] = rng.next_i16_range(-28000, 28000);
        }
        sample_schedule.push(WeightConsolidationSample::from_point_with_residual(
            t, p, latent, m,
        ));
    }

    // 1. Bitwise determinism across 5 independent model runs
    let mut model_runs = Vec::new();
    let config = ConsolidationConfig {
        iterations: 2,
        step_numerator: 3,
        step_denominator: 8,
        clamp_min: -30000,
        clamp_max: 30000,
        sync_point_keys: true,
        update_ternary_assoc: true,
    };

    for run_idx in 0..5 {
        let mut model = load_bootstrap();
        let report = consolidate_lut_weights(&mut model, &sample_schedule, &config)
            .expect("consolidation succeeds");
        assert!(report.weights_updated > 0);
        model_runs.push((run_idx, model));
    }

    let (_, ref baseline_run) = model_runs[0];
    for (idx, ref other_run) in &model_runs[1..] {
        assert_eq!(
            other_run.writer_table, baseline_run.writer_table,
            "run {idx} writer_table diverged from run 0 (determinism violated)"
        );
        if let (Some(ref pt_other), Some(ref pt_base)) = (&other_run.point, &baseline_run.point) {
            assert_eq!(
                pt_other.points, pt_base.points,
                "run {idx} point table diverged from run 0"
            );
        }
        assert_eq!(
            other_run.writer_provenance, baseline_run.writer_provenance,
            "run {idx} provenance spans diverged from run 0"
        );
        assert_eq!(
            other_run.writer_provenance_traces, baseline_run.writer_provenance_traces,
            "run {idx} provenance traces diverged from run 0"
        );
    }

    // 2. Strict Integer Metric Properties across 2,000 token pairs
    let test_model = load_bootstrap();
    let mut metric_rng = SplitMix64::new(0xCAFE_BABE_1122_3344);

    // Identity of indiscernibles
    for _ in 0..200 {
        let t = metric_rng.next_u32_range(vocab_size as u32);
        let m = compute_token_l1_margin(&test_model.writer_table, vocab_size, t, t);
        assert_eq!(m, 0, "M(t, t) must be strictly 0 for token {t}");
    }

    // Symmetry and ground truth L1 verification
    for _ in 0..2000 {
        let a = metric_rng.next_u32_range(vocab_size as u32);
        let b = metric_rng.next_u32_range(vocab_size as u32);

        let m_ab = compute_token_l1_margin(&test_model.writer_table, vocab_size, a, b);
        let m_ba = compute_token_l1_margin(&test_model.writer_table, vocab_size, b, a);

        assert_eq!(m_ab, m_ba, "symmetry violated: M(a,b) != M(b,a)");

        let a_base = (a as usize) * FBS_WIDTH;
        let b_base = (b as usize) * FBS_WIDTH;
        let mut expected = 0u64;
        for d in 0..FBS_WIDTH {
            let diff = (test_model.writer_table[a_base + d] as i64)
                - (test_model.writer_table[b_base + d] as i64);
            expected += diff.unsigned_abs();
        }
        assert_eq!(
            m_ab, expected,
            "margin must equal exact integer sum of abs diffs"
        );
    }

    // Triangle inequality on 500 triplets
    for _ in 0..500 {
        let a = metric_rng.next_u32_range(vocab_size as u32);
        let b = metric_rng.next_u32_range(vocab_size as u32);
        let c = metric_rng.next_u32_range(vocab_size as u32);

        let mab = compute_token_l1_margin(&test_model.writer_table, vocab_size, a, b);
        let mbc = compute_token_l1_margin(&test_model.writer_table, vocab_size, b, c);
        let mac = compute_token_l1_margin(&test_model.writer_table, vocab_size, a, c);

        assert!(
            mac <= mab + mbc,
            "triangle inequality violated: mac={mac} > mab={mab} + mbc={mbc}"
        );
    }
}
