//! Empirical Challenger 2 Stress Suite for Milestone M1
//! Adversarial N-Gram Repetition Detection, Multi-Candidate Fallback & 64-Bit Boundary Harness
//!
//! Evaluates:
//! 1. Multi-candidate suppression cascade: Pre-populating `emitted_4grams` to suppress
//!    candidates 0, 1, 2, 3, and 4, verifying cascading fallback to candidate 1..4 and
//!    whole-table nearest neighbor fallback when all 5 top candidates repeat.
//! 2. 64-bit boundary collisions, integer safety, and extreme hash stability:
//!    `u64::MAX`, `0`, powers-of-two boundaries, position sensitivity, and 100,000-hash collision check.
//! 3. Zero false-positive rejection of valid non-repeating candidate tokens under heavy noise.
//! 4. Incomplete 4-gram guard invariance (preventing spurious bans during prompt framing).
//! 5. Exact bit-level JSON serialization/deserialization invariance of 64-bit hashes.
//! 6. FIFO capacity bounding (2048 entries) and deduplication in session memory.

use std::collections::HashSet;
use std::sync::Arc;

use lexlean_r4_wasm::lut_stack_exec::{
    hash_4gram, LutStackModel, LutStackSession, RollingContextState,
};

use crate::BOOTSTRAP_BYTES;

fn load_test_model() -> Arc<LutStackModel> {
    crate::shared_bootstrap_model_arc()
}

#[test]
fn challenge_ngram_multiple_candidate_suppression_cascade() {
    let model = load_test_model();
    let mut baseline_session = LutStackSession::new(model.clone());

    // Prefill 3 tokens to establish a known 4-gram context window: [101, 102, 103, <pred>]
    baseline_session.prefill_token(101, true);
    baseline_session.prefill_token(102, true);
    baseline_session.mark_reply_start();

    // Baseline step with token 103: determines the natural top 5 candidates
    let baseline_st = baseline_session.step_with_telemetry(103);
    let top_candidates = baseline_st.top_candidates.clone();
    assert!(
        top_candidates.len() >= 5,
        "step_with_telemetry must produce at least 5 top candidates"
    );

    let t_minus_3 = 101u32;
    let t_minus_2 = 102u32;
    let t_minus_1 = 103u32;

    // Verify baseline candidate 0 matches baseline prediction
    assert_eq!(
        baseline_st.predicted_token, top_candidates[0].0,
        "baseline unconstrained prediction must equal candidate 0"
    );

    // Compute 4-gram hashes for each of the top 5 candidates
    let cand_hashes: Vec<u64> = top_candidates
        .iter()
        .take(5)
        .map(|&(tok, _)| hash_4gram(t_minus_3, t_minus_2, t_minus_1, tok))
        .collect();

    // Verify all 5 candidates have distinct hashes under this prefix
    let hash_set: HashSet<u64> = cand_hashes.iter().copied().collect();
    assert_eq!(
        hash_set.len(),
        5,
        "top 5 candidates must yield 5 distinct 4-gram hashes"
    );

    // Cascade test:
    // Suppress candidate 0 -> expect candidate 1
    // Suppress candidates 0, 1 -> expect candidate 2
    // Suppress candidates 0, 1, 2 -> expect candidate 3
    // Suppress candidates 0, 1, 2, 3 -> expect candidate 4
    for cascade_level in 1..=4 {
        let mut session = LutStackSession::new(model.clone());
        session.prefill_token(101, true);
        session.prefill_token(102, true);
        session.mark_reply_start();

        // Suppress candidates 0..cascade_level
        for &h in &cand_hashes[0..cascade_level] {
            session.record_emitted_4gram(h);
        }

        let st = session.step_with_telemetry(103);
        let expected_cand = top_candidates[cascade_level].0;
        let expected_dist = top_candidates[cascade_level].1;

        assert_eq!(
            st.predicted_token, expected_cand,
            "at cascade level {}, engine must select candidate {}",
            cascade_level, cascade_level
        );
        assert_eq!(
            st.l1_distance, expected_dist,
            "at cascade level {}, distance must match candidate {}",
            cascade_level, cascade_level
        );
        assert_eq!(
            st.source_mechanism, "ngram_fallback_nearest",
            "at cascade level {}, source_mechanism must be 'ngram_fallback_nearest'",
            cascade_level
        );

        // Verify the newly chosen token's hash is registered in session
        let chosen_hash = hash_4gram(t_minus_3, t_minus_2, t_minus_1, st.predicted_token);
        assert!(
            session.emitted_4grams.contains(&chosen_hash),
            "chosen candidate hash must be recorded in session.emitted_4grams"
        );
    }

    // Full exhaustion test: Suppress all 5 candidates (0, 1, 2, 3, and 4)
    let mut exhaust_session = LutStackSession::new(model.clone());
    exhaust_session.prefill_token(101, true);
    exhaust_session.prefill_token(102, true);
    exhaust_session.mark_reply_start();

    for &h in &cand_hashes[0..5] {
        exhaust_session.record_emitted_4gram(h);
    }

    let exhaust_st = exhaust_session.step_with_telemetry(103);

    // When all 5 candidates repeat, engine must fall back to whole-table nearest neighbor
    assert_eq!(
        exhaust_st.source_mechanism, "ngram_fallback_nearest",
        "whole-table fallback must report 'ngram_fallback_nearest'"
    );

    // Verify distance is positive and integer-bounded
    assert!(
        exhaust_st.l1_distance > 0,
        "whole table fallback distance must be positive integer"
    );
    assert!(
        (exhaust_st.predicted_token as usize) < model.vocab,
        "predicted token must be within vocabulary"
    );
}

#[test]
fn challenge_ngram_64bit_boundary_collisions_and_integer_safety() {
    // 1. Extreme token values: u32::MAX, 0, boundary combinations
    let extreme_tuples = [
        (0u32, 0u32, 0u32, 0u32),
        (u32::MAX, u32::MAX, u32::MAX, u32::MAX),
        (0, u32::MAX, 0, u32::MAX),
        (u32::MAX, 0, u32::MAX, 0),
        (1, 0, 0, 0),
        (0, 1, 0, 0),
        (0, 0, 1, 0),
        (0, 0, 0, 1),
        (u32::MAX - 1, u32::MAX, 0, 1),
        (0x8000_0000, 0x7fff_ffff, 0xdead_beef, 0xcafe_babe),
    ];

    let mut hashes = Vec::new();
    for &(t0, t1, t2, t3) in &extreme_tuples {
        let h = hash_4gram(t0, t1, t2, t3);
        hashes.push(h);
    }

    // Check all extreme hashes are non-zero and pairwise distinct
    let unique_extreme: HashSet<u64> = hashes.iter().copied().collect();
    assert_eq!(
        unique_extreme.len(),
        extreme_tuples.len(),
        "extreme boundary tuples must produce strictly pairwise distinct hashes"
    );

    // 2. Position sensitivity: permutations of the same 4 tokens MUST produce distinct hashes
    let (a, b, c, d) = (100u32, 200u32, 300u32, 400u32);
    let p1 = hash_4gram(a, b, c, d);
    let p2 = hash_4gram(d, c, b, a);
    let p3 = hash_4gram(b, a, d, c);
    let p4 = hash_4gram(c, d, a, b);
    let p5 = hash_4gram(a, c, b, d);
    let p6 = hash_4gram(b, c, d, a);

    let perm_set: HashSet<u64> = [p1, p2, p3, p4, p5, p6].iter().copied().collect();
    assert_eq!(
        perm_set.len(),
        6,
        "permutations of identical tokens must yield completely distinct 4-gram hashes"
    );

    // 3. Sliding window uniqueness: [a, b, c, d] vs [b, c, d, e]
    let s1 = hash_4gram(10, 20, 30, 40);
    let s2 = hash_4gram(20, 30, 40, 50);
    assert_ne!(s1, s2, "adjacent sliding windows must have distinct hashes");

    // 4. Large scale collision stress test: 100,000 distinct 4-grams
    let mut large_set = HashSet::with_capacity(100_000);
    for i in 0..100_000u32 {
        let h = hash_4gram(i, i.wrapping_mul(7), i.wrapping_add(13), i ^ 0xaa55);
        let inserted = large_set.insert(h);
        assert!(
            inserted,
            "hash collision detected at index {} for hash {}",
            i, h
        );
    }
    assert_eq!(large_set.len(), 100_000);
}

#[test]
fn challenge_ngram_json_lossless_64bit_boundary_roundtrip() {
    let mut state = RollingContextState::default();

    // Populate with extreme boundary 64-bit integer values
    let boundary_hashes = vec![
        0u64,
        1u64,
        2u64,
        (1u64 << 31) - 1,
        1u64 << 31,
        (1u64 << 32) - 1,
        1u64 << 32,
        (1u64 << 63) - 1, // i64::MAX boundary
        1u64 << 63,       // i64::MAX + 1 (highest bit set)
        (1u64 << 63) + 1,
        u64::MAX - 1000,
        u64::MAX - 1,
        u64::MAX,
    ];

    state.emitted_4grams = boundary_hashes.clone();
    state.recent_turn_rows = vec![0, 1, 2, 3, 4, 5, 6, 7];

    let json_str = state.to_json();

    // Verify raw JSON string contains exact text of u64::MAX and 1 << 63
    assert!(
        json_str.contains("18446744073709551615"),
        "JSON must contain u64::MAX in text"
    );
    assert!(
        json_str.contains("9223372036854775808"),
        "JSON must contain 1<<63 in text"
    );

    // Parse back
    let parsed = RollingContextState::from_json(&json_str)
        .expect("RollingContextState parses from JSON successfully");

    // Assert exact bit-for-bit equality across all boundary hashes
    assert_eq!(
        parsed.emitted_4grams, boundary_hashes,
        "all 64-bit boundary hashes must survive JSON serialization without loss"
    );
    assert_eq!(parsed.recent_turn_rows, vec![0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn challenge_ngram_zero_false_positive_rejection_under_noise() {
    let model = load_test_model();
    let mut session = LutStackSession::new(model.clone());

    session.prefill_token(201, true);
    session.prefill_token(202, true);
    session.mark_reply_start();

    // Baseline step
    let baseline_st = session.step_with_telemetry(203);
    let cand0 = baseline_st.top_candidates[0].0;
    assert_eq!(baseline_st.predicted_token, cand0);

    // Create session populated with 500 noise hashes that DO NOT match candidate 0's 4-gram
    let mut noise_session = LutStackSession::new(model.clone());
    noise_session.prefill_token(201, true);
    noise_session.prefill_token(202, true);
    noise_session.mark_reply_start();

    let cand0_hash = hash_4gram(201, 202, 203, cand0);

    for i in 1..=500u64 {
        let noise_h = i.wrapping_mul(0xdead_beef_cafe_babe).wrapping_add(1);
        if noise_h != cand0_hash {
            noise_session.record_emitted_4gram(noise_h);
        }
    }

    // Add boundary noise
    noise_session.record_emitted_4gram(0);
    noise_session.record_emitted_4gram(u64::MAX);
    noise_session.record_emitted_4gram(1 << 63);

    let noise_st = noise_session.step_with_telemetry(203);

    // Verify candidate 0 was NOT falsely rejected
    assert_eq!(
        noise_st.predicted_token, cand0,
        "candidate 0 must NOT be falsely rejected when its 4-gram is not in emitted_4grams"
    );
    assert_eq!(
        noise_st.source_mechanism, baseline_st.source_mechanism,
        "source mechanism must remain original (not downgraded to fallback)"
    );
    assert_eq!(
        noise_st.l1_distance, baseline_st.l1_distance,
        "l1_distance must match unpenalized candidate 0"
    );
}

#[test]
fn challenge_ngram_incomplete_prefix_guard_invariance() {
    let model = load_test_model();

    // Test step 1: only 1 token in history (t_minus_3 == MEMORY_PAD, t_minus_2 == MEMORY_PAD)
    let mut fresh_session = LutStackSession::new(model.clone());
    let mut noise_session = LutStackSession::new(model.clone());

    // Pre-populate noise session with hashes
    for i in 1..=100u64 {
        noise_session.record_emitted_4gram(i.wrapping_mul(123456789));
    }

    let st_fresh = fresh_session.step_with_telemetry(50);
    let st_noise = noise_session.step_with_telemetry(50);

    // Both must match exactly: has_full_4gram is false, so n-gram suppression is inactive
    assert_eq!(
        st_fresh.predicted_token, st_noise.predicted_token,
        "incomplete prefix must ignore emitted_4grams and produce identical prediction"
    );
    assert_eq!(
        st_fresh.source_mechanism, st_noise.source_mechanism,
        "mechanism must not be altered when full 4-gram is absent"
    );

    // Test step 2: 2 tokens in history (t_minus_3 == MEMORY_PAD)
    let st_fresh_2 = fresh_session.step_with_telemetry(60);
    let st_noise_2 = noise_session.step_with_telemetry(60);

    assert_eq!(
        st_fresh_2.predicted_token, st_noise_2.predicted_token,
        "step 2 with incomplete 4-gram must ignore emitted_4grams"
    );
}

#[test]
fn challenge_ngram_session_fifo_capping_and_deduplication() {
    let model = load_test_model();
    let mut session = LutStackSession::new(model);

    // 1. Deduplication test
    session.record_emitted_4gram(0x1234_5678);
    session.record_emitted_4gram(0x1234_5678);
    session.record_emitted_4gram(0x1234_5678);
    assert_eq!(
        session.emitted_4grams.len(),
        1,
        "duplicate hashes must be ignored"
    );

    // 2. FIFO Capping to 2048 entries
    for i in 1..=2500u64 {
        session.record_emitted_4gram(i);
    }
    assert_eq!(
        session.emitted_4grams.len(),
        2048,
        "session emitted_4grams must be strictly capped at 2048 entries"
    );

    // The oldest entries (0x1234_5678 and 1..452) must have been evicted FIFO
    assert!(
        !session.emitted_4grams.contains(&0x1234_5678),
        "oldest entry must have been evicted"
    );
    assert!(
        !session.emitted_4grams.contains(&1),
        "early entry 1 must have been evicted"
    );
    assert!(
        session.emitted_4grams.contains(&2500),
        "newest entry 2500 must be present"
    );
    assert!(
        session.emitted_4grams.contains(&2000),
        "entry 2000 must be present"
    );
}

#[test]
fn challenge_ngram_loop_breaker_and_infinite_repetition_prevention() {
    let model = load_test_model();
    let mut session = LutStackSession::new(model.clone());

    // Phase 1: Emit a 4-gram [301, 302, 303, pred1]
    session.prefill_token(301, true);
    session.prefill_token(302, true);
    session.mark_reply_start();

    let st1 = session.step_with_telemetry(303);
    let pred1 = st1.predicted_token;
    let hash1 = hash_4gram(301, 302, 303, pred1);

    // Verify hash1 is now in emitted_4grams
    assert!(
        session.emitted_4grams.contains(&hash1),
        "emitted 4-gram must be registered after step"
    );

    // Phase 2: In the same or fresh session with identical prefix, step 303 again
    let mut replay_session = LutStackSession::new(model);
    replay_session.prefill_token(301, true);
    replay_session.prefill_token(302, true);
    replay_session.mark_reply_start();
    replay_session.record_emitted_4gram(hash1);

    let st2 = replay_session.step_with_telemetry(303);

    // Verify the repeating token is broken: pred2 != pred1
    assert_ne!(
        st2.predicted_token, pred1,
        "engine must break repeating loop by rejecting previously emitted token"
    );
    assert_eq!(
        st2.source_mechanism, "ngram_fallback_nearest",
        "loop breaking must report 'ngram_fallback_nearest'"
    );
}

#[test]
fn challenge_ngram_cross_turn_persistence_and_suppression() {
    let model = load_test_model();

    // Turn 1
    let mut session1 = LutStackSession::new(model.clone());
    session1.prefill_token(401, true);
    session1.prefill_token(402, true);
    session1.mark_reply_start();

    let st1 = session1.step_with_telemetry(403);
    let original_token = st1.predicted_token;
    let expected_fallback_token = st1.top_candidates[1].0;

    // Persist to RollingContextState
    let mut context = RollingContextState::default();
    context.emitted_4grams = session1.emitted_4grams.clone();
    context.recent_turn_rows = session1.recent_turn_rows.clone();

    // Roundtrip through JSON envelope (simulating API/frontend boundary)
    let json_envelope = context.to_json();
    let reloaded_context =
        RollingContextState::from_json(&json_envelope).expect("context deserializes cleanly");

    // Turn 2
    let mut session2 = LutStackSession::new_with_context(model, &reloaded_context);
    session2.prefill_token(401, true);
    session2.prefill_token(402, true);
    session2.mark_reply_start();

    let st2 = session2.step_with_telemetry(403);

    // Verify cross-turn suppression of original_token
    assert_ne!(
        st2.predicted_token, original_token,
        "token emitted in Turn 1 must be suppressed in Turn 2"
    );
    assert_eq!(
        st2.predicted_token, expected_fallback_token,
        "Turn 2 must select the next-best non-repeating candidate"
    );
    assert_eq!(
        st2.source_mechanism, "ngram_fallback_nearest",
        "source mechanism in Turn 2 must indicate fallback"
    );
}
