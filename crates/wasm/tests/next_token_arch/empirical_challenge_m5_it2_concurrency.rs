//! Empirical Adversarial Challenge Test Suite for Milestone M5 Iteration 2:
//! Turn Admission Idempotency, Concurrency Safety Under Model Mutex,
//! Coordinate Descent Consolidation Drift, and Zero-Orphan Invariants.
//!
//! Objectives:
//! 1. Turn Admission Idempotency:
//!    - Call `compile_verified_dialogue_turn` repeatedly on identical turns after coordinate descent consolidation.
//!    - Assert `!second_admit` holds unconditionally and row count in table remains invariant.
//! 2. Concurrency Safety:
//!    - Multiple compilation attempts across threads under model mutex.
//!    - Verify that point key tables maintain zero orphan points and valid provenance spans.
//! 3. Cross-Lifecycle Serialization and Parsing Invariants:
//!    - UORLUT01 round-trip preserves turn idempotency and zero-orphan invariants.
//! 4. Adversarial Error Paths and State Invariance under Concurrency.

use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Instant;

use lexlean_r4_wasm::lut_stack_exec::{
    consolidate_lut_weights, fnv1a64, ByteBpe4096, ConsolidationConfig, LutStackModel,
    WeightConsolidationSample, FBS_WIDTH, VERIFIED_FRONTIER_ANCHOR_TAG,
};

use crate::BOOTSTRAP_BYTES;

fn load_test_model() -> LutStackModel {
    crate::shared_bootstrap_model_clone()
}

fn find_row_by_prov_span(model: &LutStackModel, prov_span: u64) -> Option<usize> {
    let pt = model.point.as_ref()?;
    let anchors = pt.canonical_anchors.as_ref()?;
    let alen = pt.anchor_len;
    let lo = (prov_span & 0xFFFF_FFFF) as u32;
    let hi = ((prov_span >> 32) & 0xFFFF_FFFF) as u32;

    for r in 0..pt.rows {
        let a_row = &anchors[r * alen..(r + 1) * alen];
        if a_row.len() >= 3 && a_row[1] == lo && a_row[2] == hi {
            return Some(r);
        }
    }
    None
}

// ============================================================================
// Challenge 1: Turn Admission Idempotency After Repeated Coordinate Descent
// ============================================================================

#[test]
fn test_adv_turn_admission_idempotency_after_repeated_coordinate_descent_consolidation() {
    let mut model = load_test_model();
    let initial_rows = model.point.as_ref().expect("has point keys").rows;
    let initial_targets = model.point.as_ref().unwrap().targets.len();
    let initial_replies = model
        .memory
        .as_ref()
        .and_then(|m| m.reply.as_ref())
        .map(|r| r.entries)
        .unwrap_or(0);
    let initial_openings = model
        .memory
        .as_ref()
        .map(|m| m.opening.entries)
        .unwrap_or(0);

    let prompt = "Explain the geometric quantization of symplectic manifolds.";
    let reply = "Geometric quantization constructs a Hilbert space from a symplectic manifold by selecting a prequantum line bundle and a polarization.";

    // 1. Initial Compilation: Must return Ok(true)
    let t0 = Instant::now();
    let first_admit = model
        .compile_verified_dialogue_turn(prompt, reply)
        .expect("initial compilation succeeds");
    let initial_elapsed = t0.elapsed();

    assert!(first_admit, "first compilation must return Ok(true)");
    assert_eq!(
        model.point.as_ref().unwrap().rows,
        initial_rows + 1,
        "point key rows must increment by exactly 1"
    );
    assert_eq!(
        model.point.as_ref().unwrap().targets.len(),
        initial_targets + 1,
        "point targets length must increment by exactly 1"
    );
    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "zero orphaned point key rows required after first admission"
    );
    assert!(
        initial_elapsed.as_millis() < 15000,
        "initial compilation must finish in < 15000ms, took {:?}",
        initial_elapsed
    );

    let rows_after_first = model.point.as_ref().unwrap().rows;
    let targets_after_first = model.point.as_ref().unwrap().targets.len();
    let replies_after_first = model
        .memory
        .as_ref()
        .and_then(|m| m.reply.as_ref())
        .map(|r| r.entries)
        .unwrap_or(0);
    let openings_after_first = model
        .memory
        .as_ref()
        .map(|m| m.opening.entries)
        .unwrap_or(0);

    assert!(
        replies_after_first > initial_replies,
        "reply entries must increase after turn compilation"
    );
    assert!(
        openings_after_first >= initial_openings,
        "opening entries must be >= initial"
    );

    // 2. Immediate Re-admission without intervening changes: Must return Ok(false)
    let second_admit = model
        .compile_verified_dialogue_turn(prompt, reply)
        .expect("second compilation succeeds");
    assert!(
        !second_admit,
        "second immediate compilation must return Ok(false) unconditionally"
    );
    assert_eq!(
        model.point.as_ref().unwrap().rows,
        rows_after_first,
        "row count must remain strictly invariant on second admit"
    );
    assert_eq!(
        model.point.as_ref().unwrap().targets.len(),
        targets_after_first,
        "targets count must remain strictly invariant on second admit"
    );
    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "zero orphans required after immediate re-admission"
    );

    // 3. Online Coordinate Descent Consolidation 1 (Moderate Shift)
    let bpe = ByteBpe4096::global();
    let reply_tokens = bpe.encode(&format!(" {reply}"));
    let target_tok = reply_tokens[0];

    let mut shift_samples = Vec::new();
    let mut synthetic_latent = [8000i16; FBS_WIDTH];
    synthetic_latent[0] = -12000;
    synthetic_latent[1] = 16000;
    shift_samples.push(WeightConsolidationSample::from_point_with_residual(
        target_tok,
        target_tok.wrapping_add(1) % (model.vocab as u32),
        synthetic_latent,
        75000,
    ));

    let cfg_mod = ConsolidationConfig {
        iterations: 2,
        step_numerator: 1,
        step_denominator: 4,
        clamp_min: -30000,
        clamp_max: 30000,
        sync_point_keys: true,
        update_ternary_assoc: true,
    };
    let report_mod = consolidate_lut_weights(&mut model, &shift_samples, &cfg_mod)
        .expect("consolidation succeeds");
    assert!(report_mod.weights_updated > 0);
    assert!(report_mod.l1_shift > 0);

    // Re-admit after consolidation 1: Must return Ok(false)
    let post_shift_admit = model
        .compile_verified_dialogue_turn(prompt, reply)
        .expect("post shift compilation succeeds");
    assert!(
        !post_shift_admit,
        "post-consolidation turn admission must return Ok(false)"
    );
    assert_eq!(model.point.as_ref().unwrap().rows, rows_after_first);

    // 4. Online Coordinate Descent Consolidation 2 (Aggressive 50% Shift, 5 Iterations)
    let mut aggressive_latent = [-20000i16; FBS_WIDTH];
    aggressive_latent[2] = 25000;
    aggressive_latent[3] = -28000;
    let aggressive_samples = vec![WeightConsolidationSample::from_point_with_residual(
        target_tok,
        (target_tok + 5) % (model.vocab as u32),
        aggressive_latent,
        250000,
    )];
    let cfg_agg = ConsolidationConfig {
        iterations: 5,
        step_numerator: 1,
        step_denominator: 2,
        clamp_min: -30000,
        clamp_max: 30000,
        sync_point_keys: true,
        update_ternary_assoc: true,
    };
    let report_agg = consolidate_lut_weights(&mut model, &aggressive_samples, &cfg_agg)
        .expect("aggressive consolidation succeeds");
    assert!(report_agg.weights_updated > 0);
    assert!(report_agg.l1_shift > 0);

    // 5. Stress Loop: Re-admit the exact same turn 30 consecutive times
    // Every call must return Ok(false) unconditionally.
    // Table rows, targets, opening entries, and reply entries must remain completely invariant.
    for iter in 1..=30 {
        let readmit_res = model
            .compile_verified_dialogue_turn(prompt, reply)
            .unwrap_or_else(|e| panic!("iteration {iter} failed with: {e}"));
        assert!(
            !readmit_res,
            "iteration {iter}: turn re-admission must return Ok(false) unconditionally"
        );
        assert_eq!(
            model.point.as_ref().unwrap().rows,
            rows_after_first,
            "iteration {iter}: rows must not drift"
        );
        assert_eq!(
            model.point.as_ref().unwrap().targets.len(),
            targets_after_first,
            "iteration {iter}: targets length must not drift"
        );
        assert_eq!(
            model
                .memory
                .as_ref()
                .and_then(|m| m.reply.as_ref())
                .map(|r| r.entries)
                .unwrap_or(0),
            replies_after_first,
            "iteration {iter}: reply entries must not drift"
        );
        assert_eq!(
            model.audit_point_key_synchronization(),
            0,
            "iteration {iter}: zero orphans required"
        );
    }

    // 6. Multi-Turn Curriculum Stress Under Interleaved Consolidation
    let curriculum_turns = [
        ("Curriculum 1: Sheaf cohomology", "Sheaf cohomology computes global sections obstruction on ringed topological spaces."),
        ("Curriculum 2: Derived categories", "Derived categories invert quasi-isomorphisms in the homotopy category of complexes."),
        ("Curriculum 3: Spectral sequences", "A spectral sequence computes the homology of a filtered complex via successive page differentials."),
        ("Curriculum 4: Etale fundamental group", "The etale fundamental group classifies finite etale covers of a connected algebraic scheme."),
        ("Curriculum 5: K-theory classes", "Grothendieck groups of vector bundles generate algebraic and topological K-theory rings."),
        ("Curriculum 6: Hodge decomposition", "Harmonic differential forms on compact Kahler manifolds yield the Hodge diamond."),
        ("Curriculum 7: Calabi-Yau metrics", "Ricci-flat metrics on Kahler manifolds with vanishing first Chern class satisfy Yau's theorem."),
        ("Curriculum 8: Mirror symmetry", "Symplectic A-models on Calabi-Yau threefolds correspond to complex B-models on mirror threefolds."),
    ];

    // Admit all 8 turns
    for (idx, &(p, r)) in curriculum_turns.iter().enumerate() {
        let newly_admitted = model
            .compile_verified_dialogue_turn(p, r)
            .unwrap_or_else(|e| panic!("curriculum turn {idx} failed: {e}"));
        assert!(
            newly_admitted,
            "curriculum turn {idx} must be admitted (Ok(true))"
        );
        assert_eq!(
            model.point.as_ref().unwrap().rows,
            rows_after_first + idx + 1
        );
        assert_eq!(model.audit_point_key_synchronization(), 0);
    }

    let rows_after_curriculum = model.point.as_ref().unwrap().rows;
    assert_eq!(rows_after_curriculum, rows_after_first + 8);

    // Apply coordinate descent on each curriculum target token
    let mut curriculum_shift_samples = Vec::new();
    for &(_p, r) in &curriculum_turns {
        let r_tokens = bpe.encode(&format!(" {r}"));
        let tok = r_tokens[0];
        let mut lat = [1000i16; FBS_WIDTH];
        lat[0] = -5000;
        curriculum_shift_samples.push(WeightConsolidationSample::from_point_with_residual(
            tok,
            tok.wrapping_add(1) % (model.vocab as u32),
            lat,
            60000,
        ));
    }
    let _ = consolidate_lut_weights(&mut model, &curriculum_shift_samples, &cfg_mod)
        .expect("curriculum consolidation succeeds");

    // Re-admit all 8 turns in scrambled/reverse order 5 times
    for cycle in 1..=5 {
        for (idx, &(p, r)) in curriculum_turns.iter().rev().enumerate() {
            let re_admit = model
                .compile_verified_dialogue_turn(p, r)
                .unwrap_or_else(|e| panic!("cycle {cycle} turn {idx} re-admit failed: {e}"));
            assert!(
                !re_admit,
                "cycle {cycle} turn {idx}: re-admit must unconditionally return Ok(false)"
            );
        }
        assert_eq!(
            model.point.as_ref().unwrap().rows,
            rows_after_curriculum,
            "cycle {cycle}: row count must remain invariant"
        );
        assert_eq!(
            model.audit_point_key_synchronization(),
            0,
            "cycle {cycle}: zero orphans required"
        );
    }
}

// ============================================================================
// Challenge 2: Concurrent Multi-Threaded Barrier Dogpile Race
// ============================================================================

#[test]
fn test_adv_concurrent_turn_admission_barrier_dogpile_race() {
    let model = Arc::new(Mutex::new(load_test_model()));
    let initial_rows = { model.lock().unwrap().point.as_ref().unwrap().rows };

    let shared_prompt = "Adversarial high-concurrency race dogpile prompt.";
    let shared_reply = "Mutual exclusion guarantees linearizable admission: exactly one thread commits, N-1 threads receive idempotent false.";

    const NUM_THREADS: usize = 16;
    let barrier = Arc::new(Barrier::new(NUM_THREADS));
    let mut handles = Vec::with_capacity(NUM_THREADS);

    // Wave 1: 16 threads simultaneously race to compile the exact same turn
    for thread_idx in 0..NUM_THREADS {
        let model_clone = Arc::clone(&model);
        let barrier_clone = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            // Synchronize starting line
            barrier_clone.wait();
            let mut guard = model_clone.lock().unwrap();
            let res = guard
                .compile_verified_dialogue_turn(shared_prompt, shared_reply)
                .unwrap_or_else(|e| {
                    panic!("thread {thread_idx} compile_verified_dialogue_turn failed: {e}")
                });
            (thread_idx, res)
        }));
    }

    let mut true_count = 0usize;
    let mut false_count = 0usize;

    for handle in handles {
        let (_tid, admitted) = handle.join().expect("thread completed cleanly");
        if admitted {
            true_count += 1;
        } else {
            false_count += 1;
        }
    }

    // Invariant: Exactly 1 thread must have admitted the turn, 15 must have received Ok(false)
    assert_eq!(
        true_count, 1,
        "wave 1: exactly 1 thread must admit the turn (got {true_count})"
    );
    assert_eq!(
        false_count,
        NUM_THREADS - 1,
        "wave 1: exactly {} threads must receive Ok(false) (got {false_count})",
        NUM_THREADS - 1
    );

    {
        let guard = model.lock().unwrap();
        assert_eq!(
            guard.point.as_ref().unwrap().rows,
            initial_rows + 1,
            "rows must increment by exactly 1 after 16-thread race"
        );
        assert_eq!(
            guard.audit_point_key_synchronization(),
            0,
            "zero orphans required after 16-thread race"
        );
    }

    // Wave 2: Another 16-thread dogpile on the SAME turn.
    // Every single thread must now receive Ok(false)!
    let barrier2 = Arc::new(Barrier::new(NUM_THREADS));
    let mut handles2 = Vec::with_capacity(NUM_THREADS);

    for thread_idx in 0..NUM_THREADS {
        let model_clone = Arc::clone(&model);
        let barrier_clone = Arc::clone(&barrier2);
        handles2.push(thread::spawn(move || {
            barrier_clone.wait();
            let mut guard = model_clone.lock().unwrap();
            let res = guard
                .compile_verified_dialogue_turn(shared_prompt, shared_reply)
                .unwrap_or_else(|e| panic!("wave 2 thread {thread_idx} compile failed: {e}"));
            (thread_idx, res)
        }));
    }

    let mut wave2_true = 0usize;
    let mut wave2_false = 0usize;
    for handle in handles2 {
        let (_tid, admitted) = handle.join().expect("wave 2 thread finished");
        if admitted {
            wave2_true += 1;
        } else {
            wave2_false += 1;
        }
    }

    assert_eq!(
        wave2_true, 0,
        "wave 2: 0 threads should admit an already-admitted turn"
    );
    assert_eq!(
        wave2_false, NUM_THREADS,
        "wave 2: all {} threads must receive Ok(false)",
        NUM_THREADS
    );

    let guard = model.lock().unwrap();
    assert_eq!(guard.point.as_ref().unwrap().rows, initial_rows + 1);
    assert_eq!(guard.audit_point_key_synchronization(), 0);

    // Verify provenance span integrity
    let expected_prov =
        fnv1a64(format!("gllm.training.compile_turn:{shared_prompt}:{shared_reply}").as_bytes())
            .max(1);
    let row_idx = find_row_by_prov_span(&guard, expected_prov)
        .expect("must find the admitted turn row by provenance span");
    let pt = guard.point.as_ref().unwrap();
    let alen = pt.anchor_len;
    let anchors = pt.canonical_anchors.as_ref().unwrap();
    let a_row = &anchors[row_idx * alen..(row_idx + 1) * alen];
    assert_ne!(a_row[0] & 0x8000_0000, 0, "anchor must have turn bit set");
    assert_eq!(a_row[7], VERIFIED_FRONTIER_ANCHOR_TAG);
    assert_eq!(
        pt.provenance_spans.as_ref().unwrap()[row_idx],
        expected_prov
    );
}

// ============================================================================
// Challenge 3: Concurrent Multi-Turn Pipeline with Interleaved Consolidation
// ============================================================================

#[test]
fn test_adv_concurrent_multi_turn_pipeline_with_interleaved_weight_consolidation() {
    let model = Arc::new(Mutex::new(load_test_model()));
    let initial_rows = { model.lock().unwrap().point.as_ref().unwrap().rows };

    // 8 compilation threads, each responsible for 3 distinct turns = 24 distinct turns
    const WORKER_THREADS: usize = 8;
    const TURNS_PER_WORKER: usize = 3;
    const TOTAL_DISTINCT_TURNS: usize = WORKER_THREADS * TURNS_PER_WORKER;

    let total_threads = WORKER_THREADS + 2; // +2 background coordinate descent threads
    let start_barrier = Arc::new(Barrier::new(total_threads));
    let mut handles = Vec::with_capacity(total_threads);

    // Spawn 8 turn admission worker threads
    for worker_id in 0..WORKER_THREADS {
        let model_clone = Arc::clone(&model);
        let barrier_clone = Arc::clone(&start_barrier);
        handles.push(thread::spawn(move || {
            barrier_clone.wait();
            let mut admitted_count = 0usize;
            let mut recheck_false_count = 0usize;

            for turn_idx in 0..TURNS_PER_WORKER {
                let prompt = format!("Pipeline Prompt from worker {worker_id} turn {turn_idx}: State lemma {worker_id}.{turn_idx}");
                let reply = format!("Pipeline Reply from worker {worker_id} turn {turn_idx}: The invariant holds under strict mutual exclusion and zero-float integer arithmetic.");

                // First compilation: Must admit
                let first_admit = {
                    let mut guard = model_clone.lock().unwrap();
                    guard
                        .compile_verified_dialogue_turn(&prompt, &reply)
                        .unwrap_or_else(|e| panic!("worker {worker_id} turn {turn_idx} failed: {e}"))
                };
                if first_admit {
                    admitted_count += 1;
                }

                // Immediate re-compilation: Must return Ok(false)
                let second_admit = {
                    let mut guard = model_clone.lock().unwrap();
                    guard
                        .compile_verified_dialogue_turn(&prompt, &reply)
                        .unwrap_or_else(|e| panic!("worker {worker_id} turn {turn_idx} re-admit failed: {e}"))
                };
                if !second_admit {
                    recheck_false_count += 1;
                }
            }
            (worker_id, admitted_count, recheck_false_count)
        }));
    }

    // Spawn 2 background coordinate descent threads that intentionally perturb weights
    for drifter_id in 0..2 {
        let model_clone = Arc::clone(&model);
        let barrier_clone = Arc::clone(&start_barrier);
        handles.push(thread::spawn(move || {
            barrier_clone.wait();
            for step in 0..6 {
                let target_tok = ((drifter_id * 100 + step * 37) % 4096) as u32;
                let mut synthetic_latent = [500i16; FBS_WIDTH];
                synthetic_latent[step % FBS_WIDTH] = -3000;
                let samples = vec![WeightConsolidationSample::from_point_with_residual(
                    target_tok,
                    (target_tok + 1) % 4096,
                    synthetic_latent,
                    40000,
                )];
                let cfg = ConsolidationConfig {
                    iterations: 1,
                    step_numerator: 1,
                    step_denominator: 8,
                    clamp_min: -30000,
                    clamp_max: 30000,
                    sync_point_keys: true,
                    update_ternary_assoc: true,
                };
                {
                    let mut guard = model_clone.lock().unwrap();
                    let _ = consolidate_lut_weights(&mut guard, &samples, &cfg);
                }
                thread::yield_now();
            }
            (999 + drifter_id, 0, 0)
        }));
    }

    let mut total_worker_admitted = 0usize;
    let mut total_worker_recheck_false = 0usize;

    for handle in handles {
        let (id, adm, refalse) = handle.join().expect("thread finished cleanly");
        if id < 999 {
            total_worker_admitted += adm;
            total_worker_recheck_false += refalse;
        }
    }

    assert_eq!(
        total_worker_admitted, TOTAL_DISTINCT_TURNS,
        "all {} distinct turns must be admitted exactly once",
        TOTAL_DISTINCT_TURNS
    );
    assert_eq!(
        total_worker_recheck_false, TOTAL_DISTINCT_TURNS,
        "all {} immediate re-compilations must return Ok(false)",
        TOTAL_DISTINCT_TURNS
    );

    let guard = model.lock().unwrap();
    assert_eq!(
        guard.point.as_ref().unwrap().rows,
        initial_rows + TOTAL_DISTINCT_TURNS,
        "total rows must equal initial + 24"
    );
    assert_eq!(
        guard.audit_point_key_synchronization(),
        0,
        "zero orphans required after concurrent pipeline execution"
    );

    // Sequential verification: Check provenance spans and re-compile all 24 turns sequentially
    let pt = guard.point.as_ref().unwrap();
    let alen = pt.anchor_len;
    let anchors = pt.canonical_anchors.as_ref().unwrap();
    let spans = pt.provenance_spans.as_ref().unwrap();

    for worker_id in 0..WORKER_THREADS {
        for turn_idx in 0..TURNS_PER_WORKER {
            let prompt = format!("Pipeline Prompt from worker {worker_id} turn {turn_idx}: State lemma {worker_id}.{turn_idx}");
            let reply = format!("Pipeline Reply from worker {worker_id} turn {turn_idx}: The invariant holds under strict mutual exclusion and zero-float integer arithmetic.");
            let expected_prov =
                fnv1a64(format!("gllm.training.compile_turn:{prompt}:{reply}").as_bytes()).max(1);

            let row_idx = find_row_by_prov_span(&guard, expected_prov).unwrap_or_else(|| {
                panic!("failed to find row for worker {worker_id} turn {turn_idx}")
            });
            let a_row = &anchors[row_idx * alen..(row_idx + 1) * alen];
            assert_ne!(a_row[0] & 0x8000_0000, 0, "anchor must have turn bit set");
            assert_eq!(a_row[7], VERIFIED_FRONTIER_ANCHOR_TAG);
            assert_eq!(spans[row_idx], expected_prov);
        }
    }
    drop(guard);

    // Re-compilation of all 24 turns must unconditionally return Ok(false)
    let mut guard_mut = model.lock().unwrap();
    for worker_id in 0..WORKER_THREADS {
        for turn_idx in 0..TURNS_PER_WORKER {
            let prompt = format!("Pipeline Prompt from worker {worker_id} turn {turn_idx}: State lemma {worker_id}.{turn_idx}");
            let reply = format!("Pipeline Reply from worker {worker_id} turn {turn_idx}: The invariant holds under strict mutual exclusion and zero-float integer arithmetic.");
            let re_admit = guard_mut
                .compile_verified_dialogue_turn(&prompt, &reply)
                .expect("re-admission check succeeds");
            assert!(
                !re_admit,
                "worker {worker_id} turn {turn_idx} must return Ok(false) on re-admission"
            );
        }
    }
    assert_eq!(
        guard_mut.point.as_ref().unwrap().rows,
        initial_rows + TOTAL_DISTINCT_TURNS,
        "row count must remain strictly invariant after all 24 re-admissions"
    );
    assert_eq!(guard_mut.audit_point_key_synchronization(), 0);
}

// ============================================================================
// Challenge 4: PointKey Table Provenance and Zero-Orphan Invariants
// ============================================================================

#[test]
fn test_adv_zero_orphan_invariants_and_provenance_audit() {
    let mut model = load_test_model();
    let initial_rows = model.point.as_ref().unwrap().rows;

    let turns = [
        ("Theorem A", "In any Hilbert space H, every closed convex subset contains a unique element of minimal norm."),
        ("Theorem B", "The Banach-Steinhaus theorem guarantees uniform boundedness of pointwise bounded families of continuous linear operators."),
        ("Theorem C", "The Hahn-Banach extension theorem extends bounded linear functionals from subspaces to the whole normed space without increasing operator norm."),
    ];

    for &(p, r) in &turns {
        let adm = model
            .compile_verified_dialogue_turn(p, r)
            .expect("admission succeeds");
        assert!(adm);
    }

    let pt = model.point.as_ref().expect("has points");
    let alen = pt.anchor_len;
    let p_width = pt.width;
    let rows = pt.rows;
    assert_eq!(rows, initial_rows + turns.len());

    // Verify vector dimensional invariants
    assert_eq!(pt.points.len(), rows * p_width);
    assert_eq!(pt.targets.len(), rows);
    let anchors = pt
        .canonical_anchors
        .as_ref()
        .expect("has canonical anchors");
    assert_eq!(anchors.len(), rows * alen);

    let spans = pt.provenance_spans.as_ref().expect("has provenance spans");
    assert_eq!(spans.len(), rows);

    let traces = pt
        .provenance_traces
        .as_ref()
        .expect("has provenance traces");
    assert_eq!(traces.len(), rows);

    // Verify zero orphans on in-memory model
    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "zero orphans required on in-memory model"
    );

    // Verify each admitted turn row
    for &(p, r) in &turns {
        let expected_prov =
            fnv1a64(format!("gllm.training.compile_turn:{p}:{r}").as_bytes()).max(1);
        let lo = (expected_prov & 0xFFFF_FFFF) as u32;
        let hi = ((expected_prov >> 32) & 0xFFFF_FFFF) as u32;

        let r_idx = find_row_by_prov_span(&model, expected_prov)
            .expect("must find admitted row by provenance span");
        let a_row = &anchors[r_idx * alen..(r_idx + 1) * alen];

        assert_ne!(
            a_row[0] & 0x8000_0000,
            0,
            "anchor ID must have high turn bit set"
        );
        assert_eq!(a_row[1], lo, "provenance lo match");
        assert_eq!(a_row[2], hi, "provenance hi match");
        assert_eq!(
            a_row[7], VERIFIED_FRONTIER_ANCHOR_TAG,
            "frontier tag must match"
        );
        assert_eq!(
            spans[r_idx], expected_prov,
            "provenance span array must match"
        );
        assert_ne!(traces[r_idx], 0, "provenance trace must be non-zero");
    }

    // Binary serialization roundtrip
    let uorlut_bytes = model.to_uorlut_bytes();
    assert_eq!(&uorlut_bytes[0..8], b"UORLUT01");

    let mut reparsed = LutStackModel::parse(&uorlut_bytes).expect("parse UORLUT01 succeeds");
    assert_eq!(reparsed.point.as_ref().unwrap().rows, rows);
    assert_eq!(
        reparsed.audit_point_key_synchronization(),
        0,
        "zero orphans required on deserialized model"
    );

    // Re-admit all turns on deserialized model
    for &(p, r) in &turns {
        let re_admit = reparsed
            .compile_verified_dialogue_turn(p, r)
            .expect("re-admission on deserialized model succeeds");
        assert!(
            !re_admit,
            "deserialized model must recognize turn as already admitted"
        );
    }
    assert_eq!(reparsed.point.as_ref().unwrap().rows, rows);
    assert_eq!(reparsed.audit_point_key_synchronization(), 0);
}

// ============================================================================
// Challenge 5: Adversarial Error Paths and Concurrency State Resilience
// ============================================================================

#[test]
fn test_adv_concurrency_error_paths_and_state_resilience() {
    let model = Arc::new(Mutex::new(load_test_model()));
    let initial_rows = { model.lock().unwrap().point.as_ref().unwrap().rows };

    let invalid_inputs = [
        ("", "Valid reply"),
        ("Valid prompt", ""),
        ("   \t  \n  ", "Valid reply"),
        ("Valid prompt", "   \n\r  "),
        ("", ""),
    ];

    const NUM_THREADS: usize = 10;
    let barrier = Arc::new(Barrier::new(NUM_THREADS));
    let mut handles = Vec::with_capacity(NUM_THREADS);

    // Half the threads submit invalid turns, half submit valid turns
    for tid in 0..NUM_THREADS {
        let model_clone = Arc::clone(&model);
        let barrier_clone = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier_clone.wait();
            let mut guard = model_clone.lock().unwrap();
            if tid % 2 == 0 {
                let (p, r) = invalid_inputs[(tid / 2) % invalid_inputs.len()];
                let res = guard.compile_verified_dialogue_turn(p, r);
                assert!(res.is_err(), "thread {tid} invalid turn must return Err");
                (tid, false)
            } else {
                let p = format!("Valid prompt from tid {tid}");
                let r = format!("Valid reply from tid {tid} confirming robustness.");
                let res = guard
                    .compile_verified_dialogue_turn(&p, &r)
                    .unwrap_or_else(|e| panic!("thread {tid} valid turn failed: {e}"));
                assert!(res, "thread {tid} valid turn must be admitted");
                (tid, true)
            }
        }));
    }

    let mut valid_count = 0usize;
    for handle in handles {
        let (_tid, is_valid) = handle.join().expect("thread finished cleanly");
        if is_valid {
            valid_count += 1;
        }
    }

    assert_eq!(valid_count, NUM_THREADS / 2);

    let guard = model.lock().unwrap();
    assert_eq!(
        guard.point.as_ref().unwrap().rows,
        initial_rows + valid_count,
        "only valid turns must increment row count"
    );
    assert_eq!(
        guard.audit_point_key_synchronization(),
        0,
        "zero orphans required after error stress"
    );
}
