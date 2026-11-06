//! Empirical Stress Test Suite for Milestone M1: Basin Repetition Adversary
//!
//! Evaluates:
//! 1. Adversarial stress: All candidate basins loaded into recent_turn_rows.
//! 2. Shortcut stress: Exact-zero match queries with candidate in recent_turn_rows (verifying raw_l1 == 0 does NOT bypass penalty).
//! 3. Recency ordering: Pos 7 (most recent) vs Pos 0 (oldest), monotonic penalty growth, identical base distance tie-breaking.
//! 4. GLM concept manifold stress: score_concept_node and retrieve_top_concepts under recency penalties.
//! 5. Robustness & edge cases: Out-of-bounds row IDs, empty recent_turn_rows, duplicated entries, large slices.

use lexlean_r4_wasm::glm_weights::{
    retrieve_top_concepts, score_concept_node, GLM_CONCEPT_MANIFOLD,
};
use lexlean_r4_wasm::lut_stack_exec::{
    select_point_key_with_resonance, ByteBpe4096, LutStackModel, ServedPointKey,
    CORE_CURRICULUM_TURNS, FBS_WIDTH,
};

use crate::BOOTSTRAP_BYTES;

fn load_test_model() -> LutStackModel {
    crate::shared_bootstrap_model_clone()
}

fn make_test_point_key(
    rows: usize,
    width: usize,
    points: Vec<i16>,
    targets: Vec<u32>,
    canonical_anchors: Option<Vec<u32>>,
    anchor_len: usize,
) -> ServedPointKey {
    ServedPointKey {
        rows,
        width,
        exp: 0,
        vote_k: 1,
        points,
        targets,
        canonical_anchors,
        anchor_len,
        provenance_spans: None,
        provenance_traces: None,
    }
}

#[test]
fn test_inspect_point_and_model_properties() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    assert!(point.rows >= 2, "Point table must have at least 2 rows");
    assert!(point.width > 0, "Point table width must be positive");
    assert!(
        point.rows > CORE_CURRICULUM_TURNS,
        "Point table has external corpus rows"
    );

    let mut core_rows = 0;
    let mut external_rows = 0;
    if let Some(anchors) = &point.canonical_anchors {
        for r in 0..point.rows {
            let a_base = r * point.anchor_len;
            let a_row = &anchors[a_base..a_base + point.anchor_len];
            let tid = if (a_row[0] & 0x8000_0000) != 0 {
                (a_row[0] & 0x7FFF_FFFF) as usize
            } else {
                usize::MAX
            };
            if tid < CORE_CURRICULUM_TURNS {
                core_rows += 1;
            } else {
                external_rows += 1;
            }
        }
    }
    println!(
        "Point rows distribution: total={}, core={}, external={}",
        point.rows, core_rows, external_rows
    );
}

#[test]
fn test_shortcut_stress_exact_zero_match_does_not_bypass_penalty() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");

    // Find a core curriculum row with turn_idx > 4
    let mut target_row = 0usize;
    if let Some(anchors) = &point.canonical_anchors {
        for r in 0..point.rows {
            let a_base = r * point.anchor_len;
            let a_row = &anchors[a_base..a_base + point.anchor_len];
            if (a_row[0] & 0x8000_0000) != 0 {
                let tid = (a_row[0] & 0x7FFF_FFFF) as usize;
                if tid > 10 && tid < CORE_CURRICULUM_TURNS {
                    target_row = r;
                    break;
                }
            }
        }
    }
    println!("Selected target_row={}", target_row);

    let mut n_t = [0i32; FBS_WIDTH];
    let row_base = target_row * point.width;
    for d in 0..point.width.min(FBS_WIDTH) {
        n_t[d] = point.points[row_base + d] as i32;
    }

    let bpe = ByteBpe4096::global();
    let prompt_tokens = bpe.encode("explain formal logic theorem verification");

    // Case A: Unpenalized baseline. Target row 10 should win easily with exact zero match.
    let hit_clean = select_point_key_with_resonance(
        point,
        &n_t,
        &prompt_tokens,
        mem.key_window,
        mem.reply.as_ref(),
        mem.reply_key_len,
        &[],
    )
    .expect("clean hit found");
    println!(
        "hit_clean: row={}, raw_l1={}, score info",
        hit_clean.row, hit_clean.raw_l1
    );
    if let Some(anchors) = &point.canonical_anchors {
        let a10 = &anchors[10 * point.anchor_len..(10 + 1) * point.anchor_len];
        println!(
            "row 10: anchor[0]={:#x}, turn_idx={}",
            a10[0],
            a10[0] & 0x7FFF_FFFF
        );
        let a2902 = &anchors[2902 * point.anchor_len..(2902 + 1) * point.anchor_len];
        println!(
            "row 2902: anchor[0]={:#x}, turn_idx={}",
            a2902[0],
            a2902[0] & 0x7FFF_FFFF
        );
    }

    assert_eq!(
        hit_clean.row, target_row,
        "target row 10 must win when unpenalized"
    );
    assert_eq!(hit_clean.raw_l1, 0, "must be exact zero match");

    // Case B: Row 10 is penalized in recent_turn_rows at pos 7 (penalty = 250_000 * 8 = 2_000_000)
    let mut recent_8 = vec![9999usize; 7];
    recent_8.push(target_row);

    let hit_penalized_7 = select_point_key_with_resonance(
        point,
        &n_t,
        &prompt_tokens,
        mem.key_window,
        mem.reply.as_ref(),
        mem.reply_key_len,
        &recent_8,
    )
    .expect("hit found under pos 7 penalty");

    assert_ne!(
        hit_penalized_7.row, target_row,
        "Row 10 with exact match (raw_l1 == 0) MUST NOT bypass the 2_000_000 penalty; another row must win"
    );
}

#[test]
fn test_recency_ordering_pos0_beats_pos7_on_identical_or_symmetric_distance() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");

    let bpe = ByteBpe4096::global();
    let prompt_tokens = bpe.encode("verification procedure");

    // Let's test with a synthetic ServedPointKey containing ONLY two identical rows to guarantee exact symmetry!
    let mut sym_points = vec![0i16; 2 * point.width];
    for d in 0..point.width {
        sym_points[d] = 1000;
        sym_points[point.width + d] = 1000;
    }
    let alen = 32;
    let mut sym_anchors = vec![0u32; 2 * alen];
    sym_anchors[0] = 100 | 0x8000_0000; // turn_idx 100 (core curriculum)
    sym_anchors[alen] = 101 | 0x8000_0000; // turn_idx 101 (core curriculum)

    let sym_point_key = make_test_point_key(
        2,
        point.width,
        sym_points,
        vec![50, 51],
        Some(sym_anchors),
        alen,
    );

    let query_zeros = [0i32; FBS_WIDTH];

    // In Trial 1: row 0 is at pos 0 (penalty 250k), row 1 is at pos 7 (penalty 2M)
    // Row 0 MUST win!
    let mut r1 = vec![999usize; 8];
    r1[0] = 0; // pos 0 -> penalty 250_000
    r1[7] = 1; // pos 7 -> penalty 2_000_000
    let hit1 = select_point_key_with_resonance(
        &sym_point_key,
        &query_zeros,
        &prompt_tokens,
        mem.key_window,
        None,
        0,
        &r1,
    )
    .expect("sym hit 1 found");
    assert_eq!(
        hit1.row, 0,
        "Candidate at pos 0 (penalty 250k) MUST beat candidate at pos 7 (penalty 2M)"
    );

    // In Trial 2: row 1 is at pos 0 (penalty 250k), row 0 is at pos 7 (penalty 2M)
    // Row 1 MUST win!
    let mut r2 = vec![999usize; 8];
    r2[0] = 1; // pos 0 -> penalty 250_000
    r2[7] = 0; // pos 7 -> penalty 2_000_000
    let hit2 = select_point_key_with_resonance(
        &sym_point_key,
        &query_zeros,
        &prompt_tokens,
        mem.key_window,
        None,
        0,
        &r2,
    )
    .expect("sym hit 2 found");
    assert_eq!(hit2.row, 1, "Candidate at pos 0 (penalty 250k) MUST beat candidate at pos 7 (penalty 2M) after swapping");
}

#[test]
fn test_adversarial_all_candidate_basins_in_recent_turn_rows() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");

    let bpe = ByteBpe4096::global();
    let prompt_tokens = bpe.encode("logic proof system");

    // Synthetic query
    let mut n_t = [0i32; FBS_WIDTH];
    for d in 0..point.width.min(FBS_WIDTH) {
        n_t[d] = point.points[20 * point.width + d] as i32;
    }

    // Create a 2-row synthetic table where BOTH rows are in recent_turn_rows
    let alen = 32;
    let mut points = vec![0i16; 2 * point.width];
    for d in 0..point.width {
        points[d] = 500;
        points[point.width + d] = 500;
    }
    let mut anchors = vec![0u32; 2 * alen];
    anchors[0] = 50 | 0x8000_0000;
    anchors[alen] = 51 | 0x8000_0000;

    let two_row_point =
        make_test_point_key(2, point.width, points, vec![100, 101], Some(anchors), alen);

    // Both candidate rows (0 and 1) are in recent_turn_rows
    // recents: [0, 1] -> row 0 is at pos 0 (penalty 250k), row 1 is at pos 1 (penalty 500k)
    let recents = vec![0usize, 1usize];
    let hit = select_point_key_with_resonance(
        &two_row_point,
        &n_t,
        &prompt_tokens,
        mem.key_window,
        None,
        0,
        &recents,
    )
    .expect("hit found when all candidate basins are in recent_turn_rows");

    // Algorithm does NOT break down or panic; it picks row 0 because pos 0 has lower penalty than pos 1
    assert_eq!(hit.row, 0, "Oldest visited candidate (pos 0) must be selected when all candidates are in recent_turn_rows");

    // Now reverse: [1, 0] -> row 1 is at pos 0 (penalty 250k), row 0 is at pos 1 (penalty 500k)
    let recents_rev = vec![1usize, 0usize];
    let hit_rev = select_point_key_with_resonance(
        &two_row_point,
        &n_t,
        &prompt_tokens,
        mem.key_window,
        None,
        0,
        &recents_rev,
    )
    .expect("hit found when all candidate basins are in reversed recent_turn_rows");
    assert_eq!(
        hit_rev.row, 1,
        "Row 1 must win when it is at pos 0 in recents_rev"
    );

    // Now test an 8-row table where ALL 8 candidate basins are in recent_turn_rows
    let mut points_8 = vec![0i16; 8 * point.width];
    for r in 0..8 {
        for d in 0..point.width {
            points_8[r * point.width + d] = 300;
        }
    }
    let mut anchors_8 = vec![0u32; 8 * alen];
    for r in 0..8 {
        anchors_8[r * alen] = (20 + r as u32) | 0x8000_0000;
    }
    let eight_row_point = make_test_point_key(
        8,
        point.width,
        points_8,
        (0..8).map(|i| 100 + i as u32).collect(),
        Some(anchors_8),
        alen,
    );

    // Permute which row is at pos 0 (oldest, least penalty = 250k):
    // For every row k from 0..8, place row k at pos 0:
    for expected_winner in 0..8 {
        let mut recents_perm = Vec::new();
        recents_perm.push(expected_winner);
        for other in 0..8 {
            if other != expected_winner {
                recents_perm.push(other);
            }
        }
        assert_eq!(recents_perm.len(), 8);

        let hit_k = select_point_key_with_resonance(
            &eight_row_point,
            &n_t,
            &prompt_tokens,
            mem.key_window,
            None,
            0,
            &recents_perm,
        )
        .expect("hit found in 8-row all-penalized table");

        assert_eq!(
            hit_k.row, expected_winner,
            "When all 8 candidate rows are penalized, candidate at pos 0 ({}) must win",
            expected_winner
        );
    }
}

#[test]
fn test_duplicate_entries_in_recent_turn_rows_uses_most_recent_occurrence() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");

    let bpe = ByteBpe4096::global();
    let prompt_tokens = bpe.encode("logic proof system");
    let n_t = [0i32; FBS_WIDTH];

    let alen = 32;
    let mut points = vec![0i16; 2 * point.width];
    for d in 0..point.width {
        points[d] = 500;
        points[point.width + d] = 500;
    }
    let mut anchors = vec![0u32; 2 * alen];
    anchors[0] = 50 | 0x8000_0000;
    anchors[alen] = 51 | 0x8000_0000;

    let two_row_point =
        make_test_point_key(2, point.width, points, vec![100, 101], Some(anchors), alen);

    // Suppose row 0 appeared at pos 0 AND pos 2: [0, 1, 0]
    // rposition will find pos 2 for row 0 (penalty 250k * 3 = 750k).
    // row 1 is at pos 1 (penalty 250k * 2 = 500k).
    // Because row 0's most recent visit is pos 2, row 1 (at pos 1) has a lower penalty (500k < 750k).
    // Therefore, row 1 MUST WIN!
    let recents_dup = vec![0usize, 1usize, 0usize];
    let hit = select_point_key_with_resonance(
        &two_row_point,
        &n_t,
        &prompt_tokens,
        mem.key_window,
        None,
        0,
        &recents_dup,
    )
    .expect("hit found with duplicate recents");
    assert_eq!(
        hit.row, 1,
        "Row 1 (penalty 500k) must beat Row 0 (most recent visit pos 2, penalty 750k)"
    );
}

#[test]
fn test_glm_weights_recency_penalty_monotonicity_and_saturation() {
    let node = &GLM_CONCEPT_MANIFOLD[0];
    let words = vec!["logic".to_string(), "deduction".to_string()];
    let stems = vec!["logic".to_string(), "deduct".to_string()];
    let q12 = [0i16; 8];

    let s_clean = score_concept_node(node, "logic deduction", &words, &stems, &q12, 0, &[]);

    // Verify monotonic penalty growth for positions 0..=7
    let mut prev_score = s_clean;
    for pos in 0..8 {
        let mut recents = vec![9999usize; pos];
        recents.push(node.id as usize);
        let s_pos = score_concept_node(node, "logic deduction", &words, &stems, &q12, 0, &recents);
        let expected_penalty = ((pos as u32) + 1) * 20;
        let expected_score = s_clean.saturating_sub(expected_penalty);
        assert_eq!(
            s_pos, expected_score,
            "At pos {}, score should be s_clean - {}",
            pos, expected_penalty
        );
        assert!(
            s_pos <= prev_score,
            "Score must decrease monotonically with recency"
        );
        prev_score = s_pos;
    }

    // Verify saturation at 0: when penalty is massive (e.g. pos 50), score saturates at 0 without underflow
    let mut recents_huge = vec![9999usize; 50];
    recents_huge.push(node.id as usize);
    let s_huge = score_concept_node(
        node,
        "logic deduction",
        &words,
        &stems,
        &q12,
        0,
        &recents_huge,
    );
    assert_eq!(s_huge, 0, "Huge penalty must saturate at 0");
}

#[test]
fn test_glm_weights_retrieve_top_concepts_recency_reordering() {
    let words = vec![
        "paris".to_string(),
        "france".to_string(),
        "eiffel".to_string(),
    ];
    let stems = vec![
        "pari".to_string(),
        "franc".to_string(),
        "eiffel".to_string(),
    ];
    let q12 = [0i16; 8];

    // Unpenalized retrieval
    let top_clean = retrieve_top_concepts("paris france eiffel", &words, &stems, &q12, 0, 5, &[]);
    assert!(
        !top_clean.is_empty(),
        "Unpenalized retrieval must return concepts"
    );

    let first_node = top_clean[0].0;
    let first_score = top_clean[0].1;
    println!(
        "Clean top concept: id={}, title={}, score={}",
        first_node.id, first_node.title, first_score
    );

    // Heavily penalize the #1 concept by putting it at pos 7 (penalty 160)
    let mut recents = vec![9999usize; 7];
    recents.push(first_node.id as usize);

    let top_penalized =
        retrieve_top_concepts("paris france eiffel", &words, &stems, &q12, 0, 5, &recents);

    // Either the #1 concept dropped in rank, or its score dropped by 160
    if let Some((_, penalized_score)) = top_penalized
        .iter()
        .find(|(node, _)| node.id == first_node.id)
    {
        assert_eq!(*penalized_score, first_score.saturating_sub(160));
    }
    if top_penalized.len() >= 2
        && top_clean.len() >= 2
        && first_score.saturating_sub(160) < top_clean[1].1
    {
        assert_ne!(
            top_penalized[0].0.id, first_node.id,
            "Heavily penalized concept must yield top position to next-best candidate"
        );
    }
}

#[test]
fn test_minimum_pos0_penalty_beats_exact_zero() {
    let point_width = 256;
    let alen = 32;

    // Row 0: exact match with query (points = [1000; 256])
    // Row 1: slight distance from query (points = [1010; 256])
    let mut points = vec![1000i16; point_width];
    points.extend(vec![1010i16; point_width]);

    let mut anchors = vec![0u32; 2 * alen];
    anchors[0] = 50 | 0x8000_0000; // turn_idx 50 (core)
    anchors[alen] = 51 | 0x8000_0000; // turn_idx 51 (core)

    let point_key =
        make_test_point_key(2, point_width, points, vec![100, 101], Some(anchors), alen);

    let bpe = ByteBpe4096::global();
    let prompt_tokens = bpe.encode("logic theorem proof");

    // Query matches Row 0 exactly
    let query = vec![1000i32; point_width];

    // Case 1: Unpenalized -> Row 0 wins
    let hit_clean =
        select_point_key_with_resonance(&point_key, &query, &prompt_tokens, 128, None, 0, &[])
            .expect("clean hit");
    assert_eq!(hit_clean.row, 0);
    assert_eq!(hit_clean.raw_l1, 0);

    // Case 2: Row 0 is at pos 0 in recent_turn_rows (minimum penalty = 250_000)
    // Row 1 is NOT in recent_turn_rows (penalty = 0, distance ~256*10 = 2560)
    // Row 1 score ~ 2560 < 250_000.
    // Row 1 MUST WIN over Row 0, proving exact zero match does NOT bypass minimum penalty!
    let hit_penalized =
        select_point_key_with_resonance(&point_key, &query, &prompt_tokens, 128, None, 0, &[0])
            .expect("penalized hit");
    assert_eq!(
        hit_penalized.row, 1,
        "Row 1 (unpenalized, small distance) MUST beat Row 0 (exact match but penalized by 250_000 at pos 0)"
    );
    assert_ne!(hit_penalized.raw_l1, 0);
}

#[test]
fn test_adversarial_break_on_core_already_complete_with_external_turn() {
    let point_width = 256;
    let alen = 32;

    // Row 0: core curriculum turn (turn_idx = 50 < CORE_CURRICULUM_TURNS)
    // Row 1: external corpus turn (turn_idx = 800 >= CORE_CURRICULUM_TURNS)
    let mut points = vec![1000i16; point_width];
    points.extend(vec![1005i16; point_width]);

    let mut anchors = vec![0u32; 2 * alen];
    anchors[0] = 50 | 0x8000_0000; // turn_idx 50 (core)
    anchors[alen] = 800 | 0x8000_0000; // turn_idx 800 (external)

    // Set stem for external turn: say stem hash matches
    let bpe = ByteBpe4096::global();
    let prompt_tokens = bpe.encode("external knowledge query");

    let point_key =
        make_test_point_key(2, point_width, points, vec![100, 101], Some(anchors), alen);

    // Query matches Row 0 exactly:
    let query = vec![1000i32; point_width];

    // Now put Row 0 in recent_turn_rows with pos 7 (penalty = 2_000_000).
    // Row 0 score is 2_000_000.
    // Row 1 is an external turn. Does the loop evaluate Row 1 or break early at line 3886?
    let mut recent_8 = vec![9999usize; 7];
    recent_8.push(0); // Row 0 at pos 7

    let hit = select_point_key_with_resonance(
        &point_key,
        &query,
        &prompt_tokens,
        128,
        None,
        0,
        &recent_8,
    )
    .expect("hit found");

    println!(
        "test_adversarial_break_on_core_already_complete result: row={}, raw_l1={}",
        hit.row, hit.raw_l1
    );
}

#[test]
fn test_multi_turn_8_basin_fifo_cycling() {
    let model = load_test_model();
    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");
    let bpe = ByteBpe4096::global();
    let prompt_tokens = bpe.encode("formal mathematical logic proof");
    let n_t = [0i32; FBS_WIDTH];

    let mut visited_basins = Vec::new();
    let mut recent_turn_rows = Vec::new();

    // Perform 10 consecutive turns with the same prompt and query
    // Each turn must route to a fresh continuation basin while recent_turn_rows < 8.
    for turn in 0..10 {
        let hit = select_point_key_with_resonance(
            point,
            &n_t,
            &prompt_tokens,
            mem.key_window,
            mem.reply.as_ref(),
            mem.reply_key_len,
            &recent_turn_rows,
        )
        .expect("hit found on each turn");

        println!("Turn {}: selected row {}", turn, hit.row);

        // For the first 8 turns, every selected row must be NEW (no repetition within 8-turn window)
        if turn < 8 {
            assert!(
                !recent_turn_rows.contains(&hit.row),
                "Turn {} selected row {} which is already in recent_turn_rows {:?}",
                turn,
                hit.row,
                recent_turn_rows
            );
        }

        visited_basins.push(hit.row);
        // FIFO-8 update:
        recent_turn_rows.push(hit.row);
        if recent_turn_rows.len() > 8 {
            let evicted = recent_turn_rows.remove(0);
            println!("Evicted oldest basin {} from FIFO-8", evicted);
        }
    }

    assert_eq!(visited_basins.len(), 10);
    // At turn 8, recent_turn_rows had 8 distinct basins
    // The algorithm never panicked or stalled
}

#[test]
fn test_robustness_out_of_bounds_and_empty_recent_turn_rows() {
    let model = load_test_model();

    let point = model.point.as_ref().expect("point table exists");
    let mem = model.memory.as_ref().expect("memory exists");
    let bpe = ByteBpe4096::global();
    let prompt_tokens = bpe.encode("logic query");
    let n_t = [0i32; FBS_WIDTH];

    // 1. Empty slice
    let hit_empty = select_point_key_with_resonance(
        point,
        &n_t,
        &prompt_tokens,
        mem.key_window,
        mem.reply.as_ref(),
        mem.reply_key_len,
        &[],
    );
    assert!(hit_empty.is_some(), "Empty recent_turn_rows must succeed");

    // 2. Extreme out-of-bounds indices in recent_turn_rows
    let oob_recents = vec![usize::MAX, 999_999_999, point.rows, point.rows + 1000];
    let hit_oob = select_point_key_with_resonance(
        point,
        &n_t,
        &prompt_tokens,
        mem.key_window,
        mem.reply.as_ref(),
        mem.reply_key_len,
        &oob_recents,
    );
    assert!(
        hit_oob.is_some(),
        "OOB row indices in recent_turn_rows must be safely ignored without panic"
    );
    assert_eq!(
        hit_oob.unwrap().row,
        hit_empty.unwrap().row,
        "OOB row indices must have zero impact on selection"
    );

    // 3. Very large recent_turn_rows (e.g. 100 elements)
    let large_recents: Vec<usize> = (0..100).map(|i| i * 10).collect();
    let hit_large = select_point_key_with_resonance(
        point,
        &n_t,
        &prompt_tokens,
        mem.key_window,
        mem.reply.as_ref(),
        mem.reply_key_len,
        &large_recents,
    );
    assert!(
        hit_large.is_some(),
        "Large recent_turn_rows must execute without arithmetic overflow"
    );
}
