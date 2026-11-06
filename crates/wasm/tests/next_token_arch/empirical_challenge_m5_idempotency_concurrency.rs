//! Empirical Adversarial Challenge Test Suite for Milestone M5:
//! Dialogue Turn Re-Admission Idempotency, Concurrency Race Safety,
//! Coordinate Descent Drift Invariance, and Zero-Orphan Point Key Invariants.
//!
//! Objectives:
//! 1. Dialogue turn re-admission idempotency across multiple sequential and concurrent
//!    compilation cycles after online coordinate descent consolidation.
//! 2. Zero-orphan point key invariants and authentic provenance span matching in `compile_verified_dialogue_turn`.
//! 3. Coordinate descent drift invariance: verifying that significant coordinate displacement in `writer.table`
//!    and `ServedPointKey.points` does not induce duplicate row insertions or table corruption.
//! 4. UORLUT01/8 binary serialization roundtrip preservation of turn admission idempotency and zero orphans.
//! 5. Multithreaded concurrent turn admission races under mutual exclusion, verifying exactly-once insertion.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use lexlean_r4_wasm::lut_stack_exec::{
    consolidate_lut_weights, fnv1a64, ByteBpe4096, ConsolidationConfig, LutStackModel,
    WeightConsolidationSample, FBS_WIDTH,
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
// Challenge 1: Sequential Re-Admission Idempotency Across Consolidation Cycles
// ============================================================================

#[test]
fn test_m5_turn_readmission_idempotency_after_coordinate_descent_consolidation() {
    let mut model = load_test_model();
    let initial_rows = model.point.as_ref().expect("has point keys").rows;
    let initial_targets = model.point.as_ref().unwrap().targets.len();
    let initial_replies = model
        .memory
        .as_ref()
        .and_then(|m| m.reply.as_ref())
        .map(|r| r.entries)
        .unwrap_or(0);

    let prompt = "Explain deterministic zero-float fixed-point consensus in distributed systems.";
    let reply = "Deterministic zero-float consensus eliminates cross-architecture floating-point divergence by performing all state transitions via exact integer arithmetic and prefix-bounded B-tree fences.";

    // 1. Initial Compilation: Must succeed and return Ok(true)
    let t0 = Instant::now();
    let newly_compiled = model
        .compile_verified_dialogue_turn(prompt, reply)
        .expect("initial compilation succeeds");
    let initial_elapsed = t0.elapsed();

    assert!(newly_compiled, "first compilation must return Ok(true)");
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
        "initial compilation must finish in interactive time (< 15000ms, took {:?})",
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
    assert!(
        replies_after_first > initial_replies,
        "reply entries must increase after newly compiling a dialogue turn"
    );

    // 2. Direct Re-Admission (Zero Intervening Shifts): Must return Ok(false)
    let second_compile = model
        .compile_verified_dialogue_turn(prompt, reply)
        .expect("second compilation succeeds");
    assert!(
        !second_compile,
        "second immediate compilation must return Ok(false) (idempotent no-op)"
    );
    assert_eq!(
        model.point.as_ref().unwrap().rows,
        rows_after_first,
        "point key rows must remain strictly invariant on second compile"
    );
    assert_eq!(
        model.point.as_ref().unwrap().targets.len(),
        targets_after_first,
        "point targets must remain strictly invariant on second compile"
    );
    assert_eq!(
        model
            .memory
            .as_ref()
            .and_then(|m| m.reply.as_ref())
            .map(|r| r.entries)
            .unwrap_or(0),
        replies_after_first,
        "reply table entries must remain strictly invariant on second compile"
    );

    // 3. Heavy Online Coordinate Descent Consolidation:
    // Force coordinate descent with aggressive step parameters on the newly compiled turn's target token.
    let bpe = ByteBpe4096::global();
    let reply_tokens = bpe.encode(&format!(" {reply}"));
    let target_tok = reply_tokens[0];

    let mut shift_samples = Vec::new();
    let mut synthetic_latent = [12000i16; FBS_WIDTH];
    synthetic_latent[0] = -15000;
    synthetic_latent[1] = 22000;
    shift_samples.push(WeightConsolidationSample::from_point_with_residual(
        target_tok,
        target_tok.wrapping_add(1) % (model.vocab as u32),
        synthetic_latent,
        150000, // large L1 residual margin
    ));

    let cfg = ConsolidationConfig {
        iterations: 3,
        step_numerator: 1,
        step_denominator: 2, // aggressive 50% coordinate descent step
        clamp_min: -30000,
        clamp_max: 30000,
        sync_point_keys: true,
        update_ternary_assoc: true,
    };

    let report = consolidate_lut_weights(&mut model, &shift_samples, &cfg)
        .expect("weight consolidation succeeds");
    assert!(
        report.weights_updated > 0,
        "consolidation must update weights for target token"
    );
    assert!(
        report.l1_shift > 0,
        "consolidation must cause non-zero L1 coordinate shift"
    );

    // 4. Repeated Re-Admission Under Coordinate Drift:
    // Call compile_verified_dialogue_turn 10 consecutive times.
    // Every single call MUST return Ok(false), and table dimensions must remain bit-for-bit invariant.
    for iter in 1..=10 {
        let readmit = model
            .compile_verified_dialogue_turn(prompt, reply)
            .unwrap_or_else(|e| panic!("iteration {iter} failed with error: {e}"));
        assert!(
            !readmit,
            "iteration {iter}: re-admission under coordinate drift must return Ok(false)"
        );
        assert_eq!(
            model.point.as_ref().unwrap().rows,
            rows_after_first,
            "iteration {iter}: row count must not drift"
        );
        assert_eq!(
            model.point.as_ref().unwrap().targets.len(),
            targets_after_first,
            "iteration {iter}: targets length must not drift"
        );
        assert_eq!(
            model.audit_point_key_synchronization(),
            0,
            "iteration {iter}: zero orphans required"
        );
    }
}

// ============================================================================
// Challenge 2: Interleaved Multi-Turn Compilation and Selective Idempotency
// ============================================================================

#[test]
fn test_m5_interleaved_multi_turn_compilation_and_selective_idempotency() {
    let mut model = load_test_model();
    let initial_rows = model.point.as_ref().unwrap().rows;

    let turns = [
        (
            "Turn 1 Prompt: Define the root lattice E8.",
            "Turn 1 Reply: E8 is the unique even unimodular lattice in dimension 8, possessing 240 minimal vectors.",
        ),
        (
            "Turn 2 Prompt: Explain the Leech lattice construction.",
            "Turn 2 Reply: The Leech lattice is the unique unimodular lattice in dimension 24 with minimal squared norm 4.",
        ),
        (
            "Turn 3 Prompt: Describe Golay code decoding.",
            "Turn 3 Reply: The extended binary Golay code G24 corrects up to 3 errors using syndrome decoding.",
        ),
        (
            "Turn 4 Prompt: State the Monster group order.",
            "Turn 4 Reply: The order of the Monster group M is approximately 8.08 times 10 to the power of 53.",
        ),
        (
            "Turn 5 Prompt: What is the McKay-Thompson series?",
            "Turn 5 Reply: The McKay-Thompson series gives the Hauptmodul for genus-zero modular curves in Monstrous Moonshine.",
        ),
    ];

    // Sequentially admit turns while interleaving re-admission checks
    for (i, &(p, r)) in turns.iter().enumerate() {
        // First admission of Turn i must return true
        let first_res = model
            .compile_verified_dialogue_turn(p, r)
            .expect("turn admission succeeds");
        assert!(
            first_res,
            "turn {i} must be newly admitted on its first compilation"
        );
        assert_eq!(
            model.point.as_ref().unwrap().rows,
            initial_rows + i + 1,
            "turn {i} must increment rows by exactly 1"
        );
        assert_eq!(
            model.audit_point_key_synchronization(),
            0,
            "turn {i} must maintain zero orphans"
        );

        // Immediate re-admission of Turn i must return false
        let second_res = model
            .compile_verified_dialogue_turn(p, r)
            .expect("turn re-admission succeeds");
        assert!(
            !second_res,
            "turn {i} must return Ok(false) on immediate re-admission"
        );

        // Re-admission of ALL previously admitted turns (0..=i) must return false
        for (prev_idx, &(prev_p, prev_r)) in turns[..=i].iter().enumerate() {
            let prev_check = model
                .compile_verified_dialogue_turn(prev_p, prev_r)
                .expect("previous turn check succeeds");
            assert!(
                !prev_check,
                "turn {prev_idx} must remain recognized as admitted when tested after turn {i}"
            );
        }
    }

    assert_eq!(
        model.point.as_ref().unwrap().rows,
        initial_rows + turns.len(),
        "total rows must equal initial + 5"
    );
    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "final synchronization audit must be 0 orphans"
    );
}

// ============================================================================
// Challenge 3: Concurrent Multi-Threaded Dialogue Turn Compilation Race
// ============================================================================

#[test]
fn test_m5_concurrent_dialogue_turn_compilation_race_and_mutual_exclusion() {
    let model = Arc::new(Mutex::new(load_test_model()));
    let initial_rows = { model.lock().unwrap().point.as_ref().unwrap().rows };

    let shared_prompt = "Concurrent race condition verification prompt.";
    let shared_reply = "Deterministic execution under mutex guarantees exactly one admission and N-1 idempotent skips.";

    const THREAD_COUNT: usize = 8;
    let mut handles = Vec::with_capacity(THREAD_COUNT);

    // 8 concurrent threads simultaneously attempt to compile the EXACT same turn
    for thread_idx in 0..THREAD_COUNT {
        let model_clone = Arc::clone(&model);
        handles.push(thread::spawn(move || {
            let mut guard = model_clone.lock().unwrap();
            let res = guard
                .compile_verified_dialogue_turn(shared_prompt, shared_reply)
                .unwrap_or_else(|e| {
                    panic!("thread {thread_idx} compile_verified_dialogue_turn error: {e}")
                });
            (thread_idx, res)
        }));
    }

    let mut true_count = 0usize;
    let mut false_count = 0usize;

    for handle in handles {
        let (_tid, admitted) = handle.join().expect("thread completed successfully");
        if admitted {
            true_count += 1;
        } else {
            false_count += 1;
        }
    }

    // Exactly one thread must have succeeded in newly compiling the turn.
    // All other 7 threads must have detected the turn as already admitted and returned false!
    assert_eq!(
        true_count, 1,
        "exactly 1 thread must admit the turn (true_count={true_count})"
    );
    assert_eq!(
        false_count,
        THREAD_COUNT - 1,
        "exactly {} threads must receive Ok(false) idempotent response (false_count={false_count})",
        THREAD_COUNT - 1
    );

    let guard = model.lock().unwrap();
    assert_eq!(
        guard.point.as_ref().unwrap().rows,
        initial_rows + 1,
        "point key rows must have grown by exactly 1 under 8-thread concurrent contention"
    );
    assert_eq!(
        guard.audit_point_key_synchronization(),
        0,
        "zero orphans required after concurrent race"
    );
}

// ============================================================================
// Challenge 4: Zero-Orphan Invariants and UORLUT01/8 Roundtrip Idempotency
// ============================================================================

#[test]
fn test_m5_zero_orphan_invariants_and_uorlut_roundtrip_idempotency() {
    let mut model = load_test_model();
    let initial_rows = model.point.as_ref().unwrap().rows;

    let prompt = "How does zero-float B-tree consensus scale across distributed shards?";
    let reply = "Zero-float B-tree consensus scales across distributed shards by combining O(log_B N) page branching with deterministic quorum replication and prefix-compressed fence keys.";

    // 1. Initial compile
    let first_res = model
        .compile_verified_dialogue_turn(prompt, reply)
        .expect("compile succeeds");
    assert!(first_res, "first compilation must succeed");
    assert_eq!(model.point.as_ref().unwrap().rows, initial_rows + 1);

    // 2. Binary serialization roundtrip
    let uorlut_bytes = model.to_uorlut_bytes();
    assert!(
        uorlut_bytes.len() > 1024,
        "serialized UORLUT01/8 bytes must be non-empty"
    );
    assert_eq!(
        &uorlut_bytes[0..8],
        b"UORLUT01",
        "must have canonical UORLUT01 header magic"
    );

    // 3. Parse into a brand new in-memory model
    let mut reparsed =
        LutStackModel::parse(&uorlut_bytes).expect("parse serialized UORLUT01/8 succeeds");
    let pk = reparsed.point.as_ref().expect("has point keys");
    assert_eq!(
        pk.rows,
        initial_rows + 1,
        "reparsed model must have initial_rows + 1"
    );

    // 4. Verify zero-orphan prefix invariant on all canonical anchors
    let anchors = pk
        .canonical_anchors
        .as_ref()
        .expect("has canonical anchors");
    let alen = pk.anchor_len;
    for (r, a_row) in anchors.chunks(alen).enumerate() {
        assert!(
            a_row[0] != 0 || a_row[1] != 0 || a_row[2] != 0,
            "row {r} must have non-zero anchor prefix: words 0..3 = {:?}",
            &a_row[0..3.min(a_row.len())]
        );
    }

    assert_eq!(
        reparsed.audit_point_key_synchronization(),
        0,
        "audit_point_key_synchronization on reparsed model must report 0 orphans"
    );

    // 5. Cross-Lifecycle Turn Re-Admission on the Reparsed Model:
    // Compiling the SAME turn on the freshly parsed model must recognize it as already admitted!
    let recompile_on_parsed = reparsed
        .compile_verified_dialogue_turn(prompt, reply)
        .expect("recompile on parsed model succeeds");
    assert!(
        !recompile_on_parsed,
        "freshly deserialized model must recognize turn as already admitted (Ok(false))"
    );
    assert_eq!(
        reparsed.point.as_ref().unwrap().rows,
        initial_rows + 1,
        "reparsed model row count must not increase after re-admission attempt"
    );
    assert_eq!(
        reparsed.audit_point_key_synchronization(),
        0,
        "zero orphans maintained after re-admission attempt on reparsed model"
    );
}

// ============================================================================
// Challenge 5: Provenance Span Matching Robustness and FNV-1a64 Invariants
// ============================================================================

#[test]
fn test_m5_provenance_span_matching_and_adversarial_variations() {
    let mut model = load_test_model();

    let p_clean = "What is the rank of the Leech lattice?";
    let r_clean = "The rank of the Leech lattice is 24.";

    // 1. Compile clean turn
    let admitted = model
        .compile_verified_dialogue_turn(p_clean, r_clean)
        .expect("compile succeeds");
    assert!(admitted);

    // Verify provenance span computation
    let expected_prov =
        fnv1a64(format!("gllm.training.compile_turn:{p_clean}:{r_clean}").as_bytes()).max(1);
    let expected_lo = (expected_prov & 0xFFFF_FFFF) as u32;
    let expected_hi = ((expected_prov >> 32) & 0xFFFF_FFFF) as u32;

    let pt = model.point.as_ref().unwrap();
    let anchors = pt.canonical_anchors.as_ref().unwrap();
    let alen = pt.anchor_len;

    // The newly inserted row has the exact provenance span
    let mut found_span_match = false;
    for r in 0..pt.rows {
        let a_row = &anchors[r * alen..(r + 1) * alen];
        if a_row[1] == expected_lo && a_row[2] == expected_hi {
            found_span_match = true;
            if let Some(spans) = pt.provenance_spans.as_ref() {
                assert_eq!(spans[r], expected_prov);
            }
            break;
        }
    }
    assert!(
        found_span_match,
        "anchors must contain the exact FNV-1a64 provenance span in words [1..3]"
    );

    // 2. Whitespace adversarial variations: Leading/trailing tabs and newlines must normalize
    let p_dirty = "  \t\n What is the rank of the Leech lattice? \n\t  ";
    let r_dirty = "\n\n The rank of the Leech lattice is 24. \t\r\n";
    let whitespace_readmit = model
        .compile_verified_dialogue_turn(p_dirty, r_dirty)
        .expect("compilation with whitespace succeeds");
    assert!(
        !whitespace_readmit,
        "whitespace-padded inputs must be trimmed and recognized as identical turn (Ok(false))"
    );

    // 3. Different reply for the same prompt: must NOT collide
    let r_different = "The Leech lattice has rank 24 and contains no roots of squared length 2.";
    let diff_reply_res = model
        .compile_verified_dialogue_turn(p_clean, r_different)
        .expect("compile with different reply succeeds");
    assert!(
        diff_reply_res,
        "turn with different reply text must be admitted as a novel turn (Ok(true))"
    );

    // 4. Same reply for a different prompt: must NOT collide
    let p_different = "State the dimension of the unimodular Leech lattice.";
    let diff_prompt_res = model
        .compile_verified_dialogue_turn(p_different, r_clean)
        .expect("compile with different prompt succeeds");
    assert!(
        diff_prompt_res,
        "turn with different prompt text must be admitted as a novel turn (Ok(true))"
    );
}

// ============================================================================
// Challenge 6: Coordinate Descent Drift Invariance & No Duplicate Insertion
// ============================================================================

#[test]
fn test_m5_coordinate_descent_drift_invariance_and_no_duplicate_insertion() {
    let mut model = load_test_model();
    let initial_rows = model.point.as_ref().unwrap().rows;

    let prompt = "Explain the connection between E8 and the icosahedral group.";
    let reply = "The McKay correspondence connects the binary icosahedral group to the affine E8 Dynkin diagram.";

    // Step 1: Admit Turn
    let res = model
        .compile_verified_dialogue_turn(prompt, reply)
        .expect("admission succeeds");
    assert!(res);
    let rows_after_turn = model.point.as_ref().unwrap().rows;
    assert_eq!(rows_after_turn, initial_rows + 1);

    let prov_span =
        fnv1a64(format!("gllm.training.compile_turn:{prompt}:{reply}").as_bytes()).max(1);
    let row_idx = find_row_by_prov_span(&model, prov_span)
        .expect("must find newly compiled row by provenance span");

    let pt = model.point.as_ref().unwrap();
    let p_width = pt.width;
    let target_tok = pt.targets[row_idx];
    let original_coords = pt.points[row_idx * p_width..(row_idx + 1) * p_width].to_vec();

    // Step 2: Subject the model to 5 successive rounds of coordinate descent
    // with divergent trajectory samples on the same target token.
    for cycle in 1..=5 {
        let mut divergent_latent = [0i16; FBS_WIDTH];
        for d in 0..FBS_WIDTH {
            divergent_latent[d] = if (d + cycle) % 2 == 0 { 18000 } else { -18000 };
        }

        let samples = vec![WeightConsolidationSample::from_point_with_residual(
            target_tok,
            (target_tok + cycle as u32) % (model.vocab as u32),
            divergent_latent,
            200000,
        )];

        let cfg = ConsolidationConfig {
            iterations: 2,
            step_numerator: 1,
            step_denominator: 4, // 25% shift per cycle
            clamp_min: -30000,
            clamp_max: 30000,
            sync_point_keys: true,
            update_ternary_assoc: true,
        };

        let rep = consolidate_lut_weights(&mut model, &samples, &cfg)
            .expect("coordinate descent step succeeds");
        assert!(rep.weights_updated > 0);
    }

    // Step 3: Verify that point coordinates have indeed DRIFTED significantly
    let shifted_coords =
        &model.point.as_ref().unwrap().points[row_idx * p_width..(row_idx + 1) * p_width];
    let mut total_l1_drift = 0u64;
    for (a, b) in original_coords.iter().zip(shifted_coords.iter()) {
        total_l1_drift += (*a as i32 - *b as i32).unsigned_abs() as u64;
    }
    assert!(
        total_l1_drift > 0,
        "point coordinates must have experienced significant coordinate descent drift (drift={total_l1_drift})"
    );

    // Step 4: Re-admit the exact same turn.
    // Despite coordinates having drifted away from n_t_i16, idempotency MUST hold via provenance span matching!
    let re_admit = model
        .compile_verified_dialogue_turn(prompt, reply)
        .expect("re-admission succeeds");
    assert!(
        !re_admit,
        "turn must be recognized as already compiled despite significant coordinate drift"
    );

    // Verify row count did NOT increase
    assert_eq!(
        model.point.as_ref().unwrap().rows,
        rows_after_turn,
        "row count must not increase after drifted re-admission"
    );
    assert_eq!(
        model.point.as_ref().unwrap().targets.len(),
        rows_after_turn,
        "targets vector length must not increase"
    );
    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "zero orphan points invariant must be strictly preserved"
    );
}
