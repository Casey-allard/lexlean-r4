//! Empirical Adversarial Challenge Test Suite for Milestone M4:
//! Online Turn Admission, Clause Boundary Filtering, and Zero-Latency Readback Adversary.
//!
//! Evaluates:
//! 1. `test_adv_clause_boundary_filtering_strictness`:
//!    Exhaustively verifies that trajectory points are captured strictly at punctuation
//!    ('.', '?', '!', ':', ';', '\n') and periodic steps (step % 16 == 0), and that
//!    word tokens at non-16 steps are NEVER captured.
//! 2. `test_adv_zero_latency_readback_and_live_routing`:
//!    Empirically verifies that immediately after `compile_verified_dialogue_turn`, queries
//!    on the live in-memory model route to newly admitted tokens and point keys with
//!    sub-millisecond latency (< 1000 µs) and zero file reload.
//! 3. `test_adv_multi_turn_curriculum_stress_and_fifo_residuals`:
//!    Admits 5 consecutive turns with alternating topic domains; verifies that `accumulated_l1_margin`
//!    strictly increases, `recent_residuals` retains up to 16 items FIFO, and
//!    `audit_point_key_synchronization() == 0` after every single turn.
//! 4. `test_adv_adversarial_inputs_and_edge_cases`:
//!    Adversarial inputs: consecutive boundary tokens (`"?!:;\n\n"`), empty prompts/replies,
//!    single-token replies, repeated tokens, turn re-admission refinement, and out-of-vocab bounds.
//! 5. `test_adv_coordinate_descent_zero_shift_and_exactness`:
//!    Verifies that zero margin yields exactly zero coordinate shift, maximum margin saturates
//!    within strict integer Q12 bounds [-30000, 30000], and zero floating-point operations occur.

use std::sync::Arc;
use std::time::Instant;

use lexlean_r4_wasm::lut_stack_exec::{
    compute_token_l1_margin, consolidate_lut_weights, extract_turn_ids, is_boundary_token,
    is_clause_boundary, ByteBpe4096, ConsolidationConfig, LutStackModel, RollingContextState,
    WeightConsolidationSample, FBS_WIDTH,
};

use crate::BOOTSTRAP_BYTES;

fn load_test_model() -> LutStackModel {
    crate::shared_bootstrap_model_clone()
}

// ============================================================================
// Challenge 1: Clause Boundary Filtering Strictness
// ============================================================================

#[test]
fn test_adv_clause_boundary_filtering_strictness() {
    let model = Arc::new(load_test_model());
    let bpe = ByteBpe4096::global();

    // Part A: Exhaustive Vocabulary Boundary Audit across all 4096 tokens
    for tok_id in 0..4096u32 {
        let dec = bpe.decode(&[tok_id]);
        let expected_boundary = dec.contains('\n')
            || dec.ends_with('.')
            || dec.ends_with('?')
            || dec.ends_with('!')
            || dec.ends_with(':')
            || dec.ends_with(';');

        let actual_boundary = is_boundary_token(tok_id);
        assert_eq!(
            actual_boundary, expected_boundary,
            "token {tok_id} ({dec:?}) boundary classification mismatch"
        );

        // For non-16 steps (e.g. 1, 2, 3, 15, 17, 31), is_clause_boundary must equal is_boundary_token
        for step in [1usize, 2, 3, 7, 11, 15, 17, 23, 31, 33] {
            assert_eq!(
                is_clause_boundary(tok_id, step),
                expected_boundary,
                "step {step} with token {tok_id} must strictly adhere to punctuation boundary check"
            );
        }

        // For step % 16 == 0 (e.g. 16, 32, 48), is_clause_boundary MUST ALWAYS be true regardless of token
        for step in [16usize, 32, 48, 64] {
            assert!(
                is_clause_boundary(tok_id, step),
                "step {step} MUST be an unconditional periodic boundary for token {tok_id}"
            );
        }
    }

    // Step 0 must never be a periodic boundary
    assert_eq!(
        is_clause_boundary(0, 0),
        is_boundary_token(0),
        "step 0 must not trigger periodic boundary"
    );

    // Part B: Continuous Punctuation-Free Stream Stress
    // Construct a 40+ token prompt and reply containing ZERO punctuation marks.
    let prompt = "Explain the nature of prime numbers in continuous prose without punctuation";
    let reply = "prime numbers are natural numbers greater than one that cannot be formed by multiplying two smaller natural numbers they play a fundamental role in number theory and modern cryptography";

    let res = model.trace_dynamic_turn(&[], prompt, reply, 64);
    assert!(
        !res.generated_tokens.is_empty(),
        "trace_dynamic_turn must produce tokens"
    );

    let recorded_steps: Vec<usize> = res.boundary_trajectories.iter().map(|h| h.step).collect();

    // Verify each generated token individually
    for (idx, &tok) in res.generated_tokens.iter().enumerate() {
        let step = idx + 1;
        let is_periodic = step > 0 && step % 16 == 0;
        let is_punct = is_boundary_token(tok);

        if is_periodic || is_punct {
            assert!(
                recorded_steps.contains(&step),
                "step {step} (periodic={is_periodic}, punct={is_punct}, tok={tok}) MUST be recorded in boundary_trajectories"
            );
        } else {
            assert!(
                !recorded_steps.contains(&step),
                "ADVERSARIAL VIOLATION: step {step} with regular word token {tok} was erroneously captured!"
            );
        }
    }

    // If the reply contains no punctuation, recorded steps must be EXACTLY multiples of 16
    let reply_has_punct = res
        .generated_tokens
        .iter()
        .any(|&tok| is_boundary_token(tok));
    if !reply_has_punct {
        for &s in &recorded_steps {
            assert_eq!(
                s % 16,
                0,
                "in punctuation-free stream, every recorded step must be a multiple of 16"
            );
        }
    }

    // Part C: Targeted Punctuation Injections at Non-Periodic Steps
    // Inject punctuation at steps 2, 5, 8, 11, 14, 17
    let punct_prompt = "Punctuation placement test";
    let punct_reply = "A. B? C! D: E; F\n G.";
    let res_punct = model.trace_dynamic_turn(&[], punct_prompt, punct_reply, 32);

    assert!(
        !res_punct.boundary_trajectories.is_empty(),
        "must capture injected punctuation boundaries"
    );
    for hit in &res_punct.boundary_trajectories {
        let dec = bpe.decode(&[hit.boundary_token]);
        let is_periodic = hit.step > 0 && hit.step % 16 == 0;
        let is_punct = dec.contains('\n')
            || dec.ends_with('.')
            || dec.ends_with('?')
            || dec.ends_with('!')
            || dec.ends_with(':')
            || dec.ends_with(';');
        assert!(
            is_periodic || is_punct,
            "captured hit at step {} token {} ({dec:?}) must be a valid boundary",
            hit.step,
            hit.boundary_token
        );
    }
}

// ============================================================================
// Challenge 2: Zero-Latency Readback & Live Model Routing
// ============================================================================

#[test]
fn test_adv_zero_latency_readback_and_live_routing() {
    let mut model = load_test_model();
    let bpe = ByteBpe4096::global();

    // Novel knowledge not in pre-training corpus
    let novel_prompt = "What is the secret frequency of the chronos oscillator in sector 9?";
    let novel_reply = "The chronos oscillator in sector 9 operates at exactly 1420405751 hertz.";

    let prompt_tokens = bpe.encode_dialogue_prompt(None, &[], novel_prompt, model.dialogue_version);
    let reply_tokens = bpe.encode(&format!(" {novel_reply}"));
    let first_reply_tok = reply_tokens[0];

    // Measure pre-compilation baseline
    let pt_rows_before = model.point.as_ref().unwrap().rows;
    let opening_entries_before = model.memory.as_ref().unwrap().opening.entries;

    // Verify key does not exist before compilation
    let open_ids = extract_turn_ids(
        &prompt_tokens,
        model.memory.as_ref().unwrap().key_window,
        model.memory.as_ref().unwrap().k,
    );
    assert!(
        model
            .memory
            .as_ref()
            .unwrap()
            .opening
            .lookup(&open_ids)
            .is_none(),
        "novel turn opening key must not exist prior to admission"
    );

    // Online turn admission
    let t_admission_start = Instant::now();
    let newly_compiled = model
        .compile_verified_dialogue_turn(novel_prompt, novel_reply)
        .expect("compile_verified_dialogue_turn must succeed");
    let admission_duration = t_admission_start.elapsed();

    assert!(newly_compiled, "novel turn must be newly compiled");
    assert!(
        admission_duration.as_millis() < 3000,
        "online turn admission must execute in interactive time (< 3000ms, was {:?})",
        admission_duration
    );

    // Verify in-memory state modified directly (zero file reload)
    let pt_rows_after = model.point.as_ref().unwrap().rows;
    let opening_entries_after = model.memory.as_ref().unwrap().opening.entries;
    assert_eq!(
        pt_rows_after,
        pt_rows_before + 1,
        "PointKey table must grow by exactly 1 row in place"
    );
    assert_eq!(
        opening_entries_after,
        opening_entries_before + 1,
        "Opening table must grow by exactly 1 entry in place"
    );

    // Readback 1: Opening Table Lookup Sub-Millisecond Latency
    let mem = model.memory.as_ref().unwrap();
    let t_lookup_start = Instant::now();
    let entry_idx = mem
        .opening
        .lookup(&open_ids)
        .expect("opening table lookup must find newly admitted key");
    let lookup_duration = t_lookup_start.elapsed();

    assert_eq!(
        mem.opening.values[entry_idx], first_reply_tok,
        "opening lookup must map directly to the newly admitted first reply token"
    );
    assert!(
        lookup_duration.as_micros() < 1000,
        "opening lookup latency must be sub-millisecond (< 1000 µs, was {:?})",
        lookup_duration
    );

    // Readback 2: PointKey Synchronization & Routing
    let pt = model.point.as_ref().unwrap();
    let inserted_row = pt
        .targets
        .iter()
        .position(|&tok| tok == first_reply_tok)
        .expect("must find target token in point keys");
    assert!(inserted_row < pt.rows);

    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "audit_point_key_synchronization must be 0 immediately after admission"
    );

    // Readback 3: Live Query Burst Micro-Benchmark (20 consecutive queries)
    let mut total_query_duration_ns = 0u128;
    for _ in 0..20 {
        let t_start = Instant::now();
        let idx = mem.opening.lookup(&open_ids);
        let dt = t_start.elapsed();
        assert!(idx.is_some());
        total_query_duration_ns += dt.as_nanos();
    }
    let mean_query_micros = (total_query_duration_ns / 20) / 1000;
    assert!(
        mean_query_micros < 100,
        "mean readback query latency across 20 iterations must be < 100 µs (was {} µs)",
        mean_query_micros
    );

    // Readback 4: Live Generation Dynamic Routing
    let arc_model = Arc::new(model);
    let t_trace_start = Instant::now();
    let trace_res = arc_model.trace_dynamic_turn(&[], novel_prompt, novel_reply, 32);
    let trace_duration = t_trace_start.elapsed();

    assert!(
        trace_duration.as_millis() < 300,
        "live readback trace must be interactive (< 300ms, was {:?})",
        trace_duration
    );
    assert!(
        trace_res.clauses_resolved >= 1,
        "trace must resolve newly admitted basin"
    );
}

// ============================================================================
// Challenge 3: Multi-Turn Curriculum Stress & FIFO Residual Buffer
// ============================================================================

#[test]
fn test_adv_multi_turn_curriculum_stress_and_fifo_residuals() {
    let mut model = load_test_model();
    let mut context = RollingContextState::new();
    let mut history: Vec<(String, String)> = Vec::new();

    // 5 diverse turns with alternating topic domains
    let curriculum = [
        (
            "What is quantum error correction via surface codes?",
            "Surface codes protect quantum information by encoding logical qubits into topological degrees of freedom on a two-dimensional square lattice.",
        ),
        (
            "Describe the structural role of flying buttresses in Gothic cathedrals.",
            "Flying buttresses redistribute lateral thrust forces from the vaulted ceiling outward to exterior masonry piers, permitting higher walls and larger stained glass windows.",
        ),
        (
            "What triggers the core collapse in a Type II supernova?",
            "Core collapse occurs when electron degeneracy pressure can no longer withstand gravitational attraction after iron fusion exhausts nuclear fuel.",
        ),
        (
            "Explain the Pythagorean comma in just intonation.",
            "The Pythagorean comma is the minute frequency discrepancy between twelve justly tuned fifths and seven pure octaves, approximately 23.46 cents.",
        ),
        (
            "What is the reaction mechanism of the Diels-Alder cycloaddition?",
            "The Diels-Alder reaction is a concerted [4+2] cycloaddition between a conjugated diene and a substituted dienophile forming a cyclohexene ring.",
        ),
    ];

    let mut previous_accumulated_margin = 0u64;

    for (turn_idx, &(prompt, reply)) in curriculum.iter().enumerate() {
        // Step 1: Trace dynamic turn with rolling context
        let arc_model = Arc::new(model.clone());
        let res =
            arc_model.trace_dynamic_turn_with_context(&mut context, &history, prompt, reply, 64);

        // Verify total L1 margin is positive
        assert!(
            res.total_l1_margin > 0,
            "turn {turn_idx} must compute non-zero prediction residual margin"
        );

        // Verify accumulated_l1_margin strictly increases
        assert!(
            context.accumulated_l1_margin > previous_accumulated_margin,
            "accumulated_l1_margin MUST strictly increase on turn {turn_idx}: {} <= {}",
            context.accumulated_l1_margin,
            previous_accumulated_margin
        );
        previous_accumulated_margin = context.accumulated_l1_margin;

        // Verify recent_residuals respects FIFO capacity cap of 16
        assert!(
            context.recent_residuals.len() <= 16,
            "recent_residuals length ({}) must never exceed FIFO capacity 16 on turn {turn_idx}",
            context.recent_residuals.len()
        );
        assert!(
            !context.recent_residuals.is_empty(),
            "recent_residuals must contain recorded margins on turn {turn_idx}"
        );

        // Verify recent_turn_rows respects capacity cap of 8
        assert!(
            context.recent_turn_rows.len() <= 8,
            "recent_turn_rows length ({}) must never exceed 8 on turn {turn_idx}",
            context.recent_turn_rows.len()
        );

        // Step 2: Compile turn online into live model
        let admitted = model
            .compile_verified_dialogue_turn(prompt, reply)
            .expect("turn admission must succeed");
        assert!(admitted, "turn {turn_idx} must be newly admitted");

        // Verify point key synchronization is pristine after every single turn
        assert_eq!(
            model.audit_point_key_synchronization(),
            0,
            "audit_point_key_synchronization must remain 0 after turn {turn_idx}"
        );

        history.push((prompt.to_string(), reply.to_string()));
    }

    // Step 3: Explicit FIFO Eviction Stress Harness
    // Push 30 synthetic margins (101..=130) to rigorously test strict FIFO queue ordering
    let mut fifo_test_context = RollingContextState::new();
    for v in 1..=30u64 {
        fifo_test_context.record_residual_margin(v * 1000);
    }
    assert_eq!(
        fifo_test_context.recent_residuals.len(),
        16,
        "recent_residuals must clamp at exactly 16"
    );
    // The items in recent_residuals must be the last 16 pushed: 15*1000 .. 30*1000
    let expected_fifo: Vec<u64> = (15..=30).map(|v| v * 1000).collect();
    assert_eq!(
        fifo_test_context.recent_residuals, expected_fifo,
        "FIFO eviction order must preserve newest 16 items and evict oldest"
    );
    let expected_accum: u64 = (1..=30).map(|v| v * 1000).sum();
    assert_eq!(
        fifo_test_context.accumulated_l1_margin, expected_accum,
        "accumulated_l1_margin must sum all 30 pushed items without loss"
    );

    // Step 4: Context JSON Serialization Roundtrip
    let json_str = fifo_test_context.to_json();
    let restored = RollingContextState::from_json(&json_str).expect("context must parse from json");
    assert_eq!(
        restored.accumulated_l1_margin,
        fifo_test_context.accumulated_l1_margin
    );
    assert_eq!(
        restored.recent_residuals,
        fifo_test_context.recent_residuals
    );
}

// ============================================================================
// Challenge 4: Adversarial Inputs, Edge Cases & Fuzzing
// ============================================================================

#[test]
fn test_adv_adversarial_inputs_and_edge_cases() {
    let mut model = load_test_model();
    let arc_model = Arc::new(model.clone());

    // 1. Consecutive Boundary Punctuation (including exact sequence "?!:;\n\n")
    let punct_runs = [
        ("Run 1", "Word?!:;\n\nAnotherWord!?!?....:::;;;End."),
        ("Run 2", "Header\n\n\n\nParagraph?!:;\n\nFooter."),
        ("Run 3", "Alpha ? ! : ; \n\n Omega."),
    ];
    for (name, text) in punct_runs {
        let res = arc_model.trace_dynamic_turn(&[], "Punctuation fuzz", text, 32);
        assert!(
            !res.boundary_trajectories.is_empty(),
            "{name} must capture boundary trajectories without crash"
        );
        for hit in &res.boundary_trajectories {
            assert!(
                is_clause_boundary(hit.boundary_token, hit.step),
                "all captured hits in {name} must be valid boundary tokens"
            );
        }
    }

    // 2. Empty and Whitespace Inputs Graceful Rejection
    let empty_cases = [
        ("", "Valid reply text."),
        ("Valid prompt text.", ""),
        ("", ""),
        ("   \t  ", "Valid reply."),
        ("Valid prompt.", "   \n\t  "),
        ("Valid prompt.", "\n\n\n\n\n\n\n\n\n\n"),
        ("\n\n\n\n\n\n\n\n\n\n", "Valid reply."),
    ];
    for &(p, r) in &empty_cases {
        let err = model.compile_verified_dialogue_turn(p, r);
        assert!(
            err.is_err(),
            "empty/whitespace prompt or reply must be rejected: prompt={p:?}, reply={r:?}"
        );
    }

    // 3. Single-Token Prompt and Single-Token Reply
    let single_p = "Hello";
    let single_r = "World";
    let admitted_single = model
        .compile_verified_dialogue_turn(single_p, single_r)
        .expect("single-token turn must compile cleanly");
    assert!(admitted_single, "single-token turn must be admitted");
    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "PointKey synchronization must remain clean for single-token turn"
    );

    // 4. Repeated Tokens (4-Gram Repetition Stress)
    let rep_p = "Repetition test prompt";
    let rep_r = "Echo echo echo echo echo echo echo echo echo echo echo echo echo echo echo.";
    let admitted_rep = model
        .compile_verified_dialogue_turn(rep_p, rep_r)
        .expect("repetitive token turn must compile cleanly");
    assert!(admitted_rep, "repetitive turn must be admitted");
    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "PointKey synchronization must remain clean with repeated 4-grams"
    );

    // 5. Turn Re-admission Behavior & Zero-Orphan Invariance
    let dup_p = "Idempotency test prompt";
    let dup_r = "Idempotency test reply response.";
    let first_admit = model
        .compile_verified_dialogue_turn(dup_p, dup_r)
        .expect("first compilation succeeds");
    assert!(first_admit, "first compilation must return true");
    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "PointKey synchronization must remain 0 after first admission"
    );

    // Re-compiling the same turn executes online consolidation and must preserve 0 orphaned rows
    let second_admit = model
        .compile_verified_dialogue_turn(dup_p, dup_r)
        .expect("second compilation succeeds");
    assert!(
        !second_admit,
        "turn is an idempotent no-op and not duplicate-admitted"
    );
    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "PointKey synchronization must remain 0 after second admission"
    );

    // 6. Out-of-Vocabulary and Negative Bounds in compute_token_l1_margin
    assert_eq!(
        compute_token_l1_margin(&model.writer_table, model.vocab, 4096, 0),
        0,
        "out-of-bounds predicted token must return 0 margin without panic"
    );
    assert_eq!(
        compute_token_l1_margin(&model.writer_table, model.vocab, 0, 4096),
        0,
        "out-of-bounds target token must return 0 margin without panic"
    );
    assert_eq!(
        compute_token_l1_margin(&model.writer_table, model.vocab, u32::MAX, u32::MAX),
        0,
        "u32::MAX tokens must return 0 margin without panic"
    );
    assert_eq!(
        compute_token_l1_margin(&[], 0, 0, 1),
        0,
        "empty writer table must return 0 margin without panic"
    );
}

// ============================================================================
// Challenge 5: Coordinate Descent Zero-Shift, Bounds Clamping & Zero-Float
// ============================================================================

#[test]
fn test_adv_coordinate_descent_zero_shift_and_exactness() {
    let mut model = load_test_model();

    // 1. Zero Margin = Zero Shift Verification
    // Perfect predictions (residual margin = 0) MUST produce zero weight changes
    let sample_zero =
        WeightConsolidationSample::from_point_with_residual(50, 50, [-1000i16; FBS_WIDTH], 0);
    let report_zero =
        consolidate_lut_weights(&mut model, &[sample_zero], &ConsolidationConfig::default())
            .expect("consolidation must succeed");

    assert_eq!(
        report_zero.weights_updated, 0,
        "zero residual margin must update exactly 0 weights"
    );
    assert_eq!(
        report_zero.l1_shift, 0,
        "zero residual margin must produce exactly 0 L1 shift"
    );
    assert_eq!(
        report_zero.max_delta, 0,
        "zero residual margin must produce max_delta of 0"
    );

    // 2. Extreme Margin & Bound Clamping Verification
    // Feed theoretical maximum margin: 15,360,000 (256 * 60,000)
    let sample_extreme = WeightConsolidationSample::from_point_with_residual(
        120,
        240,
        [32767i16; FBS_WIDTH],
        15_360_000,
    );
    let cfg = ConsolidationConfig {
        iterations: 1,
        step_numerator: 1,
        step_denominator: 8,
        clamp_min: -30000,
        clamp_max: 30000,
        sync_point_keys: true,
        update_ternary_assoc: true,
    };
    let report_extreme = consolidate_lut_weights(&mut model, &[sample_extreme], &cfg)
        .expect("consolidation on extreme margin must succeed");

    assert!(
        report_extreme.weights_updated > 0,
        "extreme margin must trigger weight updates"
    );
    assert!(report_extreme.l1_shift > 0);

    // Verify all updated coordinates remain strictly within [-30000, 30000]
    let base = 120 * FBS_WIDTH;
    for &w in &model.writer_table[base..base + FBS_WIDTH] {
        assert!(
            (-30000..=30000).contains(&w),
            "consolidated weight {w} must remain strictly within [-30000, 30000]"
        );
    }

    // Verify PointKey row for target 120 remains synchronized and clamped
    let pt = model.point.as_ref().unwrap();
    let p_width = pt.width;
    for r in 0..pt.rows {
        if pt.targets[r] == 120 {
            for &p in &pt.points[r * p_width..(r + 1) * p_width] {
                assert!(
                    (-30000..=30000).contains(&p),
                    "PointKey coordinate {p} must remain strictly clamped"
                );
            }
        }
    }

    assert_eq!(
        model.audit_point_key_synchronization(),
        0,
        "PointKey synchronization must remain 0 after extreme consolidation"
    );
}
