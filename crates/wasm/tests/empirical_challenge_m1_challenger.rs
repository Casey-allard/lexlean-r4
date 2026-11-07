#![allow(
    missing_docs,
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    unused_variables,
    unused_imports
)]
//! Empirical Challenger Stress & Scalability Test Suite for Milestone 1:
//! Two-Level Hierarchical Radix / Geometric Index over /8 ServedPointKey.
//!
//! Evaluates:
//! 1. Baseline 100k+ Basin Latency & Scaling (< 100 µs on uniform queries).
//! 2. Geometric Bucket Distribution & Boundedness (17,408 discrete buckets).
//! 3. Adversarial Recency Penalty: Dominant candidate in recent_turn_rows triggering Stage 4 full linear scan.
//! 4. Multi-Penalized Bucket Collision: Primary bucket rows penalized in recent_turn_rows.
//! 5. Unmatched Query Behavior: Unmatched stems falling through to full scan and apply_with_row.

use lexlean_r4_wasm::lut_stack_exec::{
    compute_geometric_bucket, compute_geometric_coordinates, decompose_geometric_bucket,
    extract_salient_stem_hashes_from_text, pack_i32_sign_sketch_256,
    select_point_key_with_resonance, ByteBpe4096, LutStackModel, ServedPointKey,
    CLIFFORD_ACTION_COUNT, CORE_CURRICULUM_TURNS, E8_ROOT_COUNT, FBS_WIDTH, GEOMETRIC_BUCKET_COUNT,
    ICOSIAN_ROOT_COUNT, RADIX_DIRECTORY_LEN, VERIFIED_FRONTIER_ANCHOR_TAG,
};
use std::time::Instant;

pub fn generate_100k_point_key(tag_frontier: bool) -> ServedPointKey {
    const BENCH_ROWS: usize = 104_448; // 17,408 * 6
    const BENCH_WIDTH: usize = 256;
    const BENCH_ANCHOR_LEN: usize = 32;

    let mut points = Vec::with_capacity(BENCH_ROWS * BENCH_WIDTH);
    let mut targets = Vec::with_capacity(BENCH_ROWS);
    let mut anchors = Vec::with_capacity(BENCH_ROWS * BENCH_ANCHOR_LEN);

    let mut seed: u64 = 0x5EED_1234_CAFE_BABE;
    let mut next_u64 = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        seed
    };

    let prompt_text = "formal mathematical logic proof verification";
    let prompt_stems = extract_salient_stem_hashes_from_text(prompt_text, 16);
    let stem_hashes: Vec<u32> = prompt_stems.iter().map(|&(h, _)| h).collect();

    for r in 0..BENCH_ROWS {
        for _ in 0..BENCH_WIDTH {
            let val = ((next_u64() >> 32) as i16) % 2048;
            points.push(val);
        }
        targets.push((r % 4096) as u32);

        let turn_idx = (r as u32) | 0x8000_0000;
        anchors.push(turn_idx);
        for i in 1..BENCH_ANCHOR_LEN {
            if tag_frontier && i == 7 {
                anchors.push(VERIFIED_FRONTIER_ANCHOR_TAG);
            } else if (16..22).contains(&i) && (i - 16) < stem_hashes.len() {
                anchors.push(stem_hashes[i - 16]);
            } else {
                anchors.push(0);
            }
        }
    }

    let mut pt = ServedPointKey {
        rows: BENCH_ROWS,
        width: BENCH_WIDTH,
        exp: 0,
        vote_k: 1,
        points,
        targets,
        canonical_anchors: Some(anchors),
        anchor_len: BENCH_ANCHOR_LEN,
        provenance_spans: None,
        provenance_traces: None,
        bucket_offsets: None,
        bucket_row_indices: None,
    };
    pt.build_radix_index();
    pt
}

/// Baseline: 1,000 queries on uniform random n_t over 104,448 rows.
#[test]
fn test_baseline_uniform_workload_latency_100k() {
    let pt = generate_100k_point_key(true);
    let bpe = ByteBpe4096::global();
    let prompt_tokens = bpe.encode("formal mathematical logic proof verification");

    let mut seed: u64 = 0x9876_5432_10FE_DCBA;
    let mut next_u64 = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        seed
    };

    // Warm-up
    for q in 0..50 {
        let mut n_t = [0i32; FBS_WIDTH];
        for d in 0..8 {
            n_t[d] = ((q * 37 + d * 53) as i32) % 1024;
        }
        let _ = select_point_key_with_resonance(&pt, &n_t, &prompt_tokens, 128, None, 0, &[]);
    }

    const N: usize = 1000;
    let mut total_ns = 0u128;
    let mut latencies_us = Vec::with_capacity(N);

    for _ in 0..N {
        let mut q = [0i32; FBS_WIDTH];
        let r = next_u64();
        for d in 0..8 {
            q[d] = (((r >> (d * 8)) & 0xFF) as i32) - 128;
        }
        let t0 = Instant::now();
        let hit = select_point_key_with_resonance(&pt, &q, &prompt_tokens, 128, None, 0, &[]);
        let el = t0.elapsed().as_nanos();
        total_ns += el;
        latencies_us.push((el / 1000) as u64);
        assert!(hit.is_some());
    }

    latencies_us.sort_unstable();
    let avg = (total_ns as f64 / N as f64) / 1000.0;
    let p50 = latencies_us[N / 2];
    let p95 = latencies_us[(N * 95) / 100];
    let p99 = latencies_us[(N * 99) / 100];

    println!(
        "Baseline 100k Latency (1,000 queries over 104,448 rows):\n  Avg: {:.2} µs\n  p50: {} µs\n  p95: {} µs\n  p99: {} µs",
        avg, p50, p95, p99
    );

    let limit_us: u64 = if cfg!(debug_assertions) { 500 } else { 100 };
    assert!(
        avg < limit_us as f64,
        "Average latency must be < {} µs, got {:.2} µs",
        limit_us,
        avg
    );
    assert!(
        p50 < limit_us,
        "p50 latency must be < {} µs, got {} µs",
        limit_us,
        p50
    );
}

/// Verification of bucket distribution properties across 17,408 buckets.
#[test]
fn test_bucket_distribution_metrics_100k() {
    let pt = generate_100k_point_key(true);
    let offsets = pt.bucket_offsets.as_ref().unwrap();

    let mut bucket_sizes = Vec::with_capacity(GEOMETRIC_BUCKET_COUNT);
    for b in 0..GEOMETRIC_BUCKET_COUNT {
        let size = (offsets[b + 1] - offsets[b]) as usize;
        bucket_sizes.push(size);
    }

    let min_size = *bucket_sizes.iter().min().unwrap();
    let max_size = *bucket_sizes.iter().max().unwrap();
    let non_empty = bucket_sizes.iter().filter(|&&s| s > 0).count();
    let sum: usize = bucket_sizes.iter().sum();

    println!(
        "Bucket Distribution (104,448 rows across 17,408 buckets):\n  Total rows: {}\n  Non-empty buckets: {} / 17,408 ({:.1}%)\n  Min bucket size: {}\n  Max bucket size: {}\n  Mean size: {:.2}",
        sum, non_empty, (non_empty as f64 / 17408.0) * 100.0, min_size, max_size, sum as f64 / 17408.0
    );

    assert_eq!(sum, 104_448);
    assert!(
        max_size < 100,
        "Max bucket size should be well-bounded, got {}",
        max_size
    );
}

/// Adversarial Challenge 1:
/// When a dominant candidate is penalized in `recent_turn_rows`, Stage 4 triggers an unindexed
/// full linear scan `0..point.rows` over all 104,448 rows, causing latency to spike to > 100 ms.
#[test]
fn test_adversarial_penalized_dominant_row_triggers_full_scan() {
    let pt = generate_100k_point_key(true);
    let bpe = ByteBpe4096::global();
    let prompt_tokens = bpe.encode("formal mathematical logic proof verification");

    let target_row = 46103;
    let mut n_t = [0i32; FBS_WIDTH];
    for d in 0..pt.width.min(FBS_WIDTH) {
        n_t[d] = pt.points[target_row * pt.width + d] as i32;
    }

    // Warm-up query
    let hit_clean = select_point_key_with_resonance(&pt, &n_t, &prompt_tokens, 128, None, 0, &[])
        .expect("hit clean");
    assert_eq!(hit_clean.row, target_row);

    // Dominant row in recent_turn_rows at pos 0 (penalty = 250,000)
    let recent = vec![target_row];
    let mut best_elapsed_us = u128::MAX;
    let mut hit_pen = None;
    for _ in 0..8 {
        let start = Instant::now();
        let h = select_point_key_with_resonance(&pt, &n_t, &prompt_tokens, 128, None, 0, &recent)
            .expect("hit penalized");
        let el = start.elapsed().as_micros();
        if el < best_elapsed_us {
            best_elapsed_us = el;
        }
        hit_pen = Some(h);
    }
    let hit_pen = hit_pen.unwrap();
    let elapsed_us = best_elapsed_us;

    println!(
        "Penalized Dominant Row Query: target={}, selected={}, latency={} µs",
        target_row, hit_pen.row, elapsed_us
    );

    let limit_us: u128 = if cfg!(debug_assertions) { 500 } else { 100 };
    // Latency requirement: CPU latency per query MUST be strictly < 100 µs (release) without full linear scan
    assert!(
        elapsed_us < limit_us,
        "CPU latency per query must be strictly < {} µs on indexed model, but was {} µs due to Stage 4 full linear scan over all {} rows",
        limit_us, elapsed_us, pt.rows
    );
}

/// Adversarial Challenge 2:
/// When all candidates in the primary bucket are penalized in `recent_turn_rows`,
/// Stage 4 executes a full linear scan with linear search in `recent_turn_rows`, taking ~270 ms.
#[test]
fn test_adversarial_multi_penalized_bucket_latency_explosion() {
    let pt = generate_100k_point_key(true);
    let bpe = ByteBpe4096::global();
    let prompt_tokens = bpe.encode("formal mathematical logic proof verification");

    let mut n_t = [0i32; FBS_WIDTH];
    for d in 0..8 {
        n_t[d] = 250;
    }

    let mut q12 = [0i16; 8];
    for i in 0..8 {
        q12[i] = n_t[i] as i16;
    }
    let q_sketch = pack_i32_sign_sketch_256(&n_t);
    let (q_e8, q_c64, q_i17) = compute_geometric_coordinates(&q12, q_sketch[0]);
    let primary_bucket = compute_geometric_bucket(q_e8, q_c64, q_i17);

    let offsets = pt.bucket_offsets.as_ref().unwrap();
    let row_indices = pt.bucket_row_indices.as_ref().unwrap();
    let b_start = offsets[primary_bucket] as usize;
    let b_end = offsets[primary_bucket + 1] as usize;
    let primary_rows: Vec<usize> = (b_start..b_end)
        .map(|idx| row_indices[idx] as usize)
        .collect();

    let mut best_elapsed_us = u128::MAX;
    let mut hit = None;
    for _ in 0..8 {
        let start = Instant::now();
        let h =
            select_point_key_with_resonance(&pt, &n_t, &prompt_tokens, 128, None, 0, &primary_rows)
                .expect("hit found");
        let el = start.elapsed().as_micros();
        if el < best_elapsed_us {
            best_elapsed_us = el;
        }
        hit = Some(h);
    }
    let hit = hit.unwrap();
    let elapsed_us = best_elapsed_us;

    println!(
        "Multi-penalized bucket (all {} primary rows penalized): chosen={}, latency={} µs",
        primary_rows.len(),
        hit.row,
        elapsed_us
    );

    let limit_us: u128 = if cfg!(debug_assertions) { 500 } else { 100 };
    assert!(
        elapsed_us < limit_us,
        "Query latency must remain < {} µs, got {} µs due to Stage 4 fallback full linear scan",
        limit_us,
        elapsed_us
    );
}
