//! Empirical stress tests challenging Milestone 1:
//! 1. Coordinate descent integer math clamping, extremes, and convergence.
//! 2. PointKey synchronization invariance and zero orphaned rows.
//! 3. Fail-closed behavior on corrupted weights and invalid configurations.

use lexlean_r4_wasm::lut_stack_exec::{
    consolidate_lut_trajectory_points, consolidate_lut_weights, ConsolidationConfig,
    LatentTrajectoryPoint, LutStackModel, RollingContextState, WeightConsolidationSample,
    FBS_WIDTH,
};
use lexlean_r4_wasm::studio_consolidate_weights;

const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

fn load_bootstrap() -> LutStackModel {
    LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully")
}

#[test]
fn challenge_coordinate_descent_clamping_and_extremes() {
    let mut model = load_bootstrap();
    let target_token = 77u32;
    let base = (target_token as usize) * FBS_WIDTH;

    // 1. Extreme positive input (i16::MAX = 32767) starting from 30000
    model.writer_table[base] = 30000;
    let mut latent_max = [0i16; FBS_WIDTH];
    latent_max.copy_from_slice(&model.writer_table[base..base + FBS_WIDTH]);
    latent_max[0] = i16::MAX;
    let points = vec![LatentTrajectoryPoint::new(target_token, latent_max)];
    let config = ConsolidationConfig::default(); // S=1, D=8, clamp=[-30000, 30000]

    let report = consolidate_lut_trajectory_points(&mut model, &points, &config)
        .expect("consolidation succeeds");
    // (7 * 30000 + 32767) / 8 = 242767 / 8 = 30345 -> clamped to 30000
    assert_eq!(model.writer_table[base], 30000);
    assert_eq!(
        report.weights_updated, 0,
        "value should stay at 30000 clamped"
    );

    // 2. Extreme negative input (i16::MIN = -32768) starting from -30000
    model.writer_table[base + 1] = -30000;
    let mut latent_min = [0i16; FBS_WIDTH];
    latent_min.copy_from_slice(&model.writer_table[base..base + FBS_WIDTH]);
    latent_min[1] = i16::MIN;
    let points = vec![LatentTrajectoryPoint::new(target_token, latent_min)];
    let report = consolidate_lut_trajectory_points(&mut model, &points, &config)
        .expect("consolidation succeeds");
    // (7 * -30000 + -32768) / 8 = -242768 / 8 = -30346 -> clamped to -30000
    assert_eq!(model.writer_table[base + 1], -30000);
    assert_eq!(
        report.weights_updated, 0,
        "value should stay at -30000 clamped"
    );

    // 3. Out-of-bounds initial weight (e.g. 32000) gets clamped into [-30000, 30000]
    model.writer_table[base + 2] = 32000;
    let mut latent_pull = [0i16; FBS_WIDTH];
    latent_pull.copy_from_slice(&model.writer_table[base..base + FBS_WIDTH]);
    latent_pull[2] = 32000;
    let points = vec![LatentTrajectoryPoint::new(target_token, latent_pull)];
    let _ = consolidate_lut_trajectory_points(&mut model, &points, &config)
        .expect("consolidation succeeds");
    assert!(model.writer_table[base + 2] <= 30000);
    assert_eq!(model.writer_table[base + 2], 30000);

    // 4. Custom tight clamping bounds [-12000, 12000]
    let tight_cfg = ConsolidationConfig {
        clamp_min: -12000,
        clamp_max: 12000,
        ..Default::default()
    };
    model.writer_table[base + 3] = 15000;
    let mut latent_tight = [0i16; FBS_WIDTH];
    latent_tight.copy_from_slice(&model.writer_table[base..base + FBS_WIDTH]);
    latent_tight[3] = 20000;
    let points = vec![LatentTrajectoryPoint::new(target_token, latent_tight)];
    let _ = consolidate_lut_trajectory_points(&mut model, &points, &tight_cfg)
        .expect("consolidation succeeds");
    assert_eq!(model.writer_table[base + 3], 12000);

    // 5. Zero delta: when target matches existing weight exactly
    let mut latent_zero = [0i16; FBS_WIDTH];
    latent_zero.copy_from_slice(&model.writer_table[base..base + FBS_WIDTH]);
    let points = vec![LatentTrajectoryPoint::new(target_token, latent_zero)];
    let report_zero = consolidate_lut_trajectory_points(&mut model, &points, &config)
        .expect("consolidation succeeds");
    assert_eq!(report_zero.weights_updated, 0);
    assert_eq!(report_zero.l1_shift, 0);
    assert_eq!(report_zero.max_delta, 0);
    assert_eq!(report_zero.tokens_updated, 0);
}

#[test]
fn challenge_coordinate_descent_convergence_and_no_oscillation() {
    let mut model = load_bootstrap();
    let target_token = 128u32;
    let base = (target_token as usize) * FBS_WIDTH;

    // Start coordinate at -24000, target centroid at +24000
    model.writer_table[base] = -24000;
    let target_mu = 24000i16;

    let mut latent = [0i16; FBS_WIDTH];
    latent.copy_from_slice(&model.writer_table[base..base + FBS_WIDTH]);
    latent[0] = target_mu;
    let points = vec![LatentTrajectoryPoint::new(target_token, latent)];
    let config = ConsolidationConfig::default(); // S=1, D=8

    let mut history = vec![model.writer_table[base]];
    // Theoretical decay: 48000 * (7/8)^k < 8 requires k >= 66 steps. We run 80 steps.
    for step in 1..=80 {
        let _ = consolidate_lut_trajectory_points(&mut model, &points, &config)
            .expect("consolidation step succeeds");
        let w = model.writer_table[base];
        let prev = *history.last().unwrap();

        // 1. Monotonic approach: w must be strictly non-decreasing toward target_mu
        assert!(
            w >= prev,
            "step {step}: w ({w}) should be >= prev ({prev}) when moving toward {target_mu}"
        );
        // 2. Bounded above by target_mu (cannot overshoot)
        assert!(
            w <= target_mu,
            "step {step}: w ({w}) must not overshoot target_mu ({target_mu})"
        );

        history.push(w);
    }

    // Check that once stabilized, it reaches within 7 units and never oscillates
    let final_val = *history.last().unwrap();
    // Distance from final_val to target_mu must be < 8 (due to integer truncation)
    assert!(
        (target_mu - final_val).abs() < 8,
        "final value {final_val} should be within 7 units of {target_mu}"
    );

    // Verify tail stability across last 10 iterations
    for &val in &history[history.len() - 10..] {
        assert_eq!(val, final_val, "system must not oscillate at fixed point");
    }

    // Now test downward convergence: start at +24000, target at -24000
    model.writer_table[base] = 24000;
    let target_mu_neg = -24000i16;
    let mut latent_neg = [0i16; FBS_WIDTH];
    latent_neg.copy_from_slice(&model.writer_table[base..base + FBS_WIDTH]);
    latent_neg[0] = target_mu_neg;
    let points_neg = vec![LatentTrajectoryPoint::new(target_token, latent_neg)];

    let mut down_history = vec![model.writer_table[base]];
    for step in 1..=80 {
        let _ = consolidate_lut_trajectory_points(&mut model, &points_neg, &config)
            .expect("consolidation step succeeds");
        let w = model.writer_table[base];
        let prev = *down_history.last().unwrap();

        assert!(
            w <= prev,
            "step {step}: w ({w}) should be <= prev ({prev}) when moving down toward {target_mu_neg}"
        );
        assert!(
            w >= target_mu_neg,
            "step {step}: w ({w}) must not undershoot target_mu_neg ({target_mu_neg})"
        );
        down_history.push(w);
    }
    let final_down = *down_history.last().unwrap();
    assert!(
        (final_down - target_mu_neg).abs() < 8,
        "final downward value {final_down} should be within 7 units of {target_mu_neg}"
    );
    for &val in &down_history[down_history.len() - 10..] {
        assert_eq!(val, final_down, "downward fixed point must not oscillate");
    }
}

#[test]
fn challenge_rolling_context_state_extreme_retention_blending() {
    let mut state = RollingContextState::default();

    // 1. Extreme positive input of i32::MAX / 2
    let extreme_pos = [1_000_000_000i32; FBS_WIDTH];
    for _ in 0..50 {
        state.blend_latent(&extreme_pos);
        for d in 0..FBS_WIDTH {
            assert!(
                state.q12_latent[d] <= 30000,
                "latent coordinate must never exceed 30000"
            );
        }
    }
    assert_eq!(state.q12_latent, [30000i16; FBS_WIDTH]);

    // 2. Extreme negative input of -1_000_000_000
    let extreme_neg = [-1_000_000_000i32; FBS_WIDTH];
    for _ in 0..50 {
        state.blend_latent(&extreme_neg);
        for d in 0..FBS_WIDTH {
            assert!(
                state.q12_latent[d] >= -30000,
                "latent coordinate must never drop below -30000"
            );
        }
    }
    assert_eq!(state.q12_latent, [-30000i16; FBS_WIDTH]);
}

#[test]
fn challenge_all_4096_tokens_mass_consolidation() {
    let mut model = load_bootstrap();
    let vocab = model.vocab;
    assert_eq!(vocab, 4096);

    let mut points = Vec::with_capacity(vocab);
    for t in 0..vocab as u32 {
        let mut latent = [0i16; FBS_WIDTH];
        // Alternate positive and negative extreme centroids per token
        let val = if t % 2 == 0 { 28000i16 } else { -28000i16 };
        latent.fill(val);
        points.push(LatentTrajectoryPoint::new(t, latent));
    }

    let config = ConsolidationConfig::default();
    let report = consolidate_lut_trajectory_points(&mut model, &points, &config)
        .expect("mass consolidation of all 4096 tokens succeeds");

    assert_eq!(report.tokens_updated, 4096, "all 4096 tokens updated");
    assert_eq!(report.points_consolidated, 4096);
    assert!(report.weights_updated > 1000000, "all 1M weights updated");
    assert_eq!(report.orphaned_point_rows, 0, "0 orphaned point rows");
    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "audit confirms 0 orphaned rows"
    );

    // Verify every single weight in writer_table is strictly in [-30000, 30000]
    for (idx, &w) in model.writer_table.iter().enumerate() {
        assert!(
            (-30000..=30000).contains(&w),
            "weight at {idx} = {w} outside [-30000, 30000]"
        );
    }
}

#[test]
fn challenge_point_key_synchronization_invariance_under_arbitrary_samples() {
    let mut model = load_bootstrap();
    assert_eq!(model.audit_point_key_synchronization(), 0);

    let pt = model.point.as_ref().expect("ServedPointKey present");
    let point_rows = pt.rows;
    assert!(point_rows > 0);
    let matched_targets: Vec<u32> = pt.targets.clone();

    // 1. Consolidate specifically on tokens matching PointKey rows
    let mut matched_samples = Vec::new();
    for &t in matched_targets.iter().take(20) {
        let mut latent = [0i16; FBS_WIDTH];
        latent[0] = 12345;
        matched_samples.push(WeightConsolidationSample::from_point(t, latent));
    }
    let rep = consolidate_lut_weights(
        &mut model,
        &matched_samples,
        &ConsolidationConfig::default(),
    )
    .expect("matched consolidation succeeds");
    assert_eq!(rep.orphaned_point_rows, 0);
    assert_eq!(model.audit_point_key_synchronization(), 0);

    // 2. Consolidate on tokens NOT in PointKey targets
    let unmatched_target = 3999u32;
    assert!(!matched_targets.contains(&unmatched_target));
    let mut latent_unmatched = [0i16; FBS_WIDTH];
    latent_unmatched[10] = -15000;
    let unmatched_sample =
        WeightConsolidationSample::from_point(unmatched_target, latent_unmatched);
    let rep2 = consolidate_lut_weights(
        &mut model,
        &[unmatched_sample],
        &ConsolidationConfig::default(),
    )
    .expect("unmatched consolidation succeeds");
    assert_eq!(rep2.orphaned_point_rows, 0);
    assert_eq!(model.audit_point_key_synchronization(), 0);

    // 3. Roundtrip serialize to UORLUT01 bytes and verify reparse produces 0 orphans
    let bytes = model.to_uorlut_bytes();
    let reparsed = LutStackModel::parse(&bytes).expect("reparse succeeds");
    assert_eq!(reparsed.audit_point_key_synchronization(), 0);
}

#[test]
fn challenge_fail_closed_on_invalid_step_config() {
    let mut model = load_bootstrap();
    let sample = [WeightConsolidationSample::from_point(1, [0i16; FBS_WIDTH])];

    // D <= 0
    let cfg1 = ConsolidationConfig {
        step_denominator: 0,
        ..Default::default()
    };
    assert!(consolidate_lut_weights(&mut model, &sample, &cfg1).is_err());

    let cfg2 = ConsolidationConfig {
        step_denominator: -8,
        ..Default::default()
    };
    assert!(consolidate_lut_weights(&mut model, &sample, &cfg2).is_err());

    // S <= 0
    let cfg3 = ConsolidationConfig {
        step_denominator: 8,
        step_numerator: 0,
        ..Default::default()
    };
    assert!(consolidate_lut_weights(&mut model, &sample, &cfg3).is_err());

    let cfg4 = ConsolidationConfig {
        step_denominator: 8,
        step_numerator: -1,
        ..Default::default()
    };
    assert!(consolidate_lut_weights(&mut model, &sample, &cfg4).is_err());

    // S > D (overstepping)
    let cfg5 = ConsolidationConfig {
        step_numerator: 9,
        step_denominator: 8,
        ..Default::default()
    };
    assert!(consolidate_lut_weights(&mut model, &sample, &cfg5).is_err());
}

#[test]
fn challenge_fail_closed_on_corrupted_model_bytes() {
    // 1. Empty bytes
    assert!(LutStackModel::parse(&[]).is_err());

    // 2. Truncated header (< 16 bytes)
    assert!(LutStackModel::parse(b"UORLUT01").is_err());
    assert!(LutStackModel::parse(&[0u8; 15]).is_err());

    // 3. Bad magic header
    let mut bad_magic = BOOTSTRAP_BYTES[..128].to_vec();
    bad_magic[0..8].copy_from_slice(b"BADMAGIC");
    assert!(LutStackModel::parse(&bad_magic).is_err());

    // 4. Declared header length exceeds file size
    let mut bad_len = BOOTSTRAP_BYTES[..128].to_vec();
    let huge_len = 1_000_000u64.to_le_bytes();
    bad_len[8..16].copy_from_slice(&huge_len);
    assert!(LutStackModel::parse(&bad_len).is_err());

    // 5. Corrupted JSON header
    let mut bad_json = BOOTSTRAP_BYTES.to_vec();
    // Overwrite bytes 16..24 with invalid ASCII
    bad_json[16..24].copy_from_slice(b"{{{{{{{{");
    assert!(LutStackModel::parse(&bad_json).is_err());

    // 6. studio_consolidate_weights fail-closed on invalid config JSON
    let bad_config_json = "{ invalid json: true }";
    assert!(studio_consolidate_weights(BOOTSTRAP_BYTES, bad_config_json).is_err());

    // 7. studio_consolidate_weights fail-closed on invalid step denominator
    let bad_step_json = "{\"step_denominator\": 0}";
    assert!(studio_consolidate_weights(BOOTSTRAP_BYTES, bad_step_json).is_err());
}

#[test]
fn challenge_inverted_clamp_bounds_panics() {
    let mut model = load_bootstrap();
    let sample = [WeightConsolidationSample::from_point(
        1,
        [1000i16; FBS_WIDTH],
    )];
    let cfg = ConsolidationConfig {
        clamp_min: 1000,
        clamp_max: -1000,
        ..Default::default()
    };
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        consolidate_lut_weights(&mut model, &sample, &cfg)
    }));
    assert!(
        res.is_ok(),
        "inverted clamp bounds must not panic in std::panic::catch_unwind"
    );
    let inner = res.unwrap();
    assert!(
        inner.is_err(),
        "inverted clamp bounds (clamp_min > clamp_max) must return Err"
    );
    let err = inner.unwrap_err();
    assert!(
        err.contains("invalid clamp bounds")
            && err.contains("clamp_min")
            && err.contains("clamp_max"),
        "error message must describe inverted clamp bounds: {err}"
    );
}

#[test]
fn challenge_large_step_numerator_overflow() {
    let mut model = load_bootstrap();
    let sample = [WeightConsolidationSample::from_point(
        1,
        [10000i16; FBS_WIDTH],
    )];
    let cfg = ConsolidationConfig {
        step_numerator: i64::MAX,
        step_denominator: i64::MAX,
        ..Default::default()
    };
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        consolidate_lut_weights(&mut model, &sample, &cfg)
    }));
    assert!(
        res.is_err(),
        "step_numerator = i64::MAX overflows s * mu_td in debug arithmetic"
    );
}

#[test]
#[allow(clippy::needless_range_loop)]
fn challenge_degenerate_clamp_intervals_0_pos30k_neg30k() {
    let mut model = load_bootstrap();
    let target_token = 42u32;
    let base = (target_token as usize) * FBS_WIDTH;

    // Test 1: clamp_min == clamp_max == 0
    // Populate writer table row and trajectory point with diverse positive/negative values
    for d in 0..FBS_WIDTH {
        model.writer_table[base + d] = ((d as i32 * 250) - 32000) as i16;
    }
    let mut latent_0 = [0i16; FBS_WIDTH];
    for d in 0..FBS_WIDTH {
        latent_0[d] = (30000 - d as i32 * 230) as i16;
    }
    let points_0 = vec![LatentTrajectoryPoint::new(target_token, latent_0)];
    let cfg_0 = ConsolidationConfig {
        clamp_min: 0,
        clamp_max: 0,
        sync_point_keys: true,
        ..Default::default()
    };
    let rep_0 = consolidate_lut_trajectory_points(&mut model, &points_0, &cfg_0)
        .expect("consolidation with clamp_min == clamp_max == 0 succeeds");
    assert!(rep_0.weights_updated > 0);
    for d in 0..FBS_WIDTH {
        assert_eq!(
            model.writer_table[base + d],
            0,
            "coordinate {d} must clamp exactly to 0"
        );
    }
    if let Some(pt) = model.point.as_ref() {
        for r in 0..pt.rows {
            if pt.targets[r] == target_token {
                let p_base = r * pt.width;
                for d in 0..pt.width.min(FBS_WIDTH) {
                    assert_eq!(
                        pt.points[p_base + d],
                        0,
                        "PointKey row {r} dim {d} must clamp exactly to 0"
                    );
                }
            }
        }
    }

    // Test 2: clamp_min == clamp_max == -30000
    for d in 0..FBS_WIDTH {
        model.writer_table[base + d] = ((d as i32 * 200) - 10000) as i16;
    }
    let mut latent_neg = [0i16; FBS_WIDTH];
    for d in 0..FBS_WIDTH {
        latent_neg[d] = ((d as i32 * 150) - 20000) as i16;
    }
    let points_neg = vec![LatentTrajectoryPoint::new(target_token, latent_neg)];
    let cfg_neg = ConsolidationConfig {
        clamp_min: -30000,
        clamp_max: -30000,
        sync_point_keys: true,
        ..Default::default()
    };
    let rep_neg = consolidate_lut_trajectory_points(&mut model, &points_neg, &cfg_neg)
        .expect("consolidation with clamp_min == clamp_max == -30000 succeeds");
    assert!(rep_neg.weights_updated > 0);
    for d in 0..FBS_WIDTH {
        assert_eq!(
            model.writer_table[base + d],
            -30000,
            "coordinate {d} must clamp exactly to -30000 without underflow"
        );
    }
    if let Some(pt) = model.point.as_ref() {
        for r in 0..pt.rows {
            if pt.targets[r] == target_token {
                let p_base = r * pt.width;
                for d in 0..pt.width.min(FBS_WIDTH) {
                    assert_eq!(
                        pt.points[p_base + d],
                        -30000,
                        "PointKey row {r} dim {d} must clamp exactly to -30000"
                    );
                }
            }
        }
    }

    // Test 3: clamp_min == clamp_max == 30000
    for d in 0..FBS_WIDTH {
        model.writer_table[base + d] = ((d as i32 * 100) - 15000) as i16;
    }
    let mut latent_pos = [0i16; FBS_WIDTH];
    for d in 0..FBS_WIDTH {
        latent_pos[d] = ((d as i32 * 200) - 5000) as i16;
    }
    let points_pos = vec![LatentTrajectoryPoint::new(target_token, latent_pos)];
    let cfg_pos = ConsolidationConfig {
        clamp_min: 30000,
        clamp_max: 30000,
        sync_point_keys: true,
        ..Default::default()
    };
    let rep_pos = consolidate_lut_trajectory_points(&mut model, &points_pos, &cfg_pos)
        .expect("consolidation with clamp_min == clamp_max == 30000 succeeds");
    assert!(rep_pos.weights_updated > 0);
    for d in 0..FBS_WIDTH {
        assert_eq!(
            model.writer_table[base + d],
            30000,
            "coordinate {d} must clamp exactly to 30000 without overflow"
        );
    }
    if let Some(pt) = model.point.as_ref() {
        for r in 0..pt.rows {
            if pt.targets[r] == target_token {
                let p_base = r * pt.width;
                for d in 0..pt.width.min(FBS_WIDTH) {
                    assert_eq!(
                        pt.points[p_base + d],
                        30000,
                        "PointKey row {r} dim {d} must clamp exactly to 30000"
                    );
                }
            }
        }
    }
}

#[test]
#[allow(clippy::needless_range_loop)]
fn challenge_mass_consolidation_4096_tokens_pointkey_and_anchors() {
    let mut model = load_bootstrap();
    let vocab = model.vocab;
    assert_eq!(vocab, 4096);

    let orig_anchors = model
        .point
        .as_ref()
        .expect("point key exists")
        .canonical_anchors
        .clone()
        .expect("canonical anchors exist");
    let orig_packed12 = model
        .memory
        .as_ref()
        .expect("memory exists")
        .reply
        .as_ref()
        .expect("reply exists")
        .packed12
        .clone();
    let orig_pt_points = model.point.as_ref().unwrap().points.clone();

    // Create 4096 distinct trajectory points across the entire vocabulary
    let mut points = Vec::with_capacity(vocab);
    for t in 0..vocab as u32 {
        let mut latent = [0i16; FBS_WIDTH];
        for d in 0..FBS_WIDTH {
            let v = (((t as i32 * 17 + d as i32 * 31) % 50000) - 25000) as i16;
            latent[d] = v;
        }
        points.push(LatentTrajectoryPoint::new(t, latent));
    }

    let config = ConsolidationConfig {
        sync_point_keys: true,
        update_ternary_assoc: true,
        ..Default::default()
    };
    let report = consolidate_lut_trajectory_points(&mut model, &points, &config)
        .expect("4096 token mass consolidation succeeds");

    assert_eq!(report.tokens_updated, 4096);
    assert_eq!(report.points_consolidated, 4096);
    assert_eq!(report.orphaned_point_rows, 0);
    assert_eq!(model.audit_point_key_synchronization(), 0);

    // INVARIANT 1: canonical_anchors are completely unchanged (byte-for-byte identical)
    let current_anchors = model
        .point
        .as_ref()
        .unwrap()
        .canonical_anchors
        .as_ref()
        .unwrap();
    assert_eq!(
        current_anchors, &orig_anchors,
        "canonical_anchors must remain completely unchanged across consolidation"
    );

    // INVARIANT 2: memory.reply.packed12 are completely unchanged (byte-for-byte identical)
    let current_packed12 = &model
        .memory
        .as_ref()
        .unwrap()
        .reply
        .as_ref()
        .unwrap()
        .packed12;
    assert_eq!(
        current_packed12, &orig_packed12,
        "memory.reply.packed12 must remain completely unchanged across consolidation"
    );

    // INVARIANT 3: PointKey rows (/8) update toward token centroids according to exact formula
    let pt = model.point.as_ref().unwrap();
    let p_width = pt.width;
    let s = config.step_numerator;
    let d = config.step_denominator;
    for r in 0..pt.rows {
        let target = pt.targets[r] as usize;
        let c_base = target * FBS_WIDTH;
        let p_base = r * p_width;
        for dim in 0..p_width.min(FBS_WIDTH) {
            let old_p = orig_pt_points[p_base + dim];
            let new_p = pt.points[p_base + dim];
            let centroid_val = model.writer_table[c_base + dim];

            // Verify exact integer arithmetic step
            let expected_p = (((d - s) * (old_p as i64) + s * (centroid_val as i64)) / d)
                .clamp(config.clamp_min as i64, config.clamp_max as i64)
                as i16;
            assert_eq!(
                new_p, expected_p,
                "PointKey row {r} dim {dim} did not match expected /8 update step"
            );

            // Verify monotonicity toward centroid
            let dist_old = (old_p as i32 - centroid_val as i32).abs();
            let dist_new = (new_p as i32 - centroid_val as i32).abs();
            assert!(
                dist_new <= dist_old,
                "PointKey row {r} dim {dim} moved away from centroid! dist_new: {dist_new}, dist_old: {dist_old}"
            );
        }
    }
}

#[test]
#[allow(clippy::needless_range_loop)]
fn challenge_point_key_zero_orphan_invariance_and_oracle_sensitivity() {
    let mut model = load_bootstrap();
    assert_eq!(model.audit_point_key_synchronization(), 0);

    // Part 1: Zero-orphan invariant under diverse arbitrary sample inputs
    let test_token_sets = vec![
        vec![0u32, 1, 2, 3, 4],
        vec![100u32, 200, 300, 400],
        vec![1000u32, 2000, 3000, 4000],
        (0..50).map(|i| i * 73 % 4096).collect::<Vec<u32>>(),
    ];

    for (case_idx, token_set) in test_token_sets.into_iter().enumerate() {
        let mut samples = Vec::new();
        for &tok in &token_set {
            let mut latent = [0i16; FBS_WIDTH];
            for dim in 0..FBS_WIDTH {
                latent[dim] = match (case_idx + dim) % 5 {
                    0 => 0,
                    1 => 30000,
                    2 => -30000,
                    3 => 14321,
                    _ => -19876,
                };
            }
            samples.push(WeightConsolidationSample::from_point(tok, latent));
        }

        let cfg = ConsolidationConfig {
            iterations: 3,
            step_numerator: 1,
            step_denominator: 8,
            sync_point_keys: true,
            ..Default::default()
        };
        let rep = consolidate_lut_weights(&mut model, &samples, &cfg)
            .expect("consolidation with arbitrary samples succeeds");
        assert_eq!(rep.orphaned_point_rows, 0);
        assert_eq!(
            model.audit_point_key_synchronization(),
            0,
            "case {case_idx} must yield 0 orphaned rows"
        );
    }

    // Part 2: Oracle sensitivity (prove the audit oracle detects corrupted anchors)
    let mut tampered_model = model.clone();
    let reply_anchors: std::collections::HashSet<u32> = tampered_model
        .memory
        .as_ref()
        .unwrap()
        .reply
        .as_ref()
        .unwrap()
        .packed12
        .iter()
        .map(|chunk| chunk[0] >> 11)
        .collect();

    let mut bogus_raw = None;
    for cand in 1..=0x1E_FFFFu32 {
        let raw = cand | 0x8000_0000;
        let enc = lexlean_r4_wasm::lut_stack_exec::encode_reply_anchor_21(raw);
        if !reply_anchors.contains(&enc) {
            bogus_raw = Some(raw);
            break;
        }
    }
    let bogus_raw = bogus_raw.expect("found unmapped anchor value");

    if let Some(pt) = tampered_model.point.as_mut() {
        if let Some(anchors) = pt.canonical_anchors.as_mut() {
            anchors[0] = bogus_raw;
        }
    }
    let detected_orphans = tampered_model.audit_point_key_synchronization();
    assert!(
        detected_orphans >= 1,
        "audit_point_key_synchronization MUST detect deliberately corrupted anchor: got {detected_orphans}"
    );

    // Also tamper by clearing the reply table, verifying all rows become orphaned
    let mut empty_mem_model = model.clone();
    if let Some(mem) = empty_mem_model.memory.as_mut() {
        if let Some(reply) = mem.reply.as_mut() {
            reply.packed12.clear();
        }
    }
    let all_orphans = empty_mem_model.audit_point_key_synchronization();
    let expected_total_rows = model.point.as_ref().unwrap().rows;
    assert_eq!(
        all_orphans, expected_total_rows,
        "clearing reply table must cause all {expected_total_rows} point rows to be detected as orphaned"
    );
}
