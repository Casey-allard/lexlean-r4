//! Empirical challenge test harness for Milestone 2: ArXiv Literature Discovery.
//!
//! Tests:
//! 1. Hypothesis witness calibration and mutual exclusivity across all 4 strategy families
//!    (curated seed papers + all dynamic fallback categories).
//! 2. Frontier induction stress testing (0, 1, 5, excess needed, incremental mining, idempotency).
//! 3. ID uniqueness, zero duplicate kappa signatures, and valid prerequisite dependency DAGs.
//! 4. Oracle sensitivity and fail-closed bounds on corrupted witnesses.

#![allow(dead_code, unused_imports, clippy::all)]

use std::collections::BTreeSet;
use std::path::Path;

#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/lut_stack_exec.rs"]
pub mod lut_stack_exec;

#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/otel.rs"]
pub mod otel;

#[allow(dead_code, clippy::all)]
#[path = "../src/engine.rs"]
mod engine;

use engine::{
    compute_claim_witnesses, evaluate_strategy_on_claim, load_frontier, load_literature,
    mine_literature_frontier_claims, ArxivPaper, ClaimWitnesses, ExtractedHypothesis,
    FrontierClaim, LiteratureFile, FAMILIES, STRATEGIES,
};

/// 1. EMPIRICAL CHALLENGE: Hypothesis witness calibration and strict mutual exclusivity
///    for all 5 curated seed papers in `model/literature.toml`.
#[test]
fn challenge_curated_seed_hypotheses_mutual_exclusivity_zero_cross_proving() {
    let root = repo_model::repo_root();
    let lit = load_literature(&root).expect("load_literature must succeed");

    let expected_strategy_for_family = |family: &str| match family {
        "dialogue-stem" => "pointkey-resonance",
        "stem-reasoning" => "reply-4gram-deduction",
        "code-synthesis" => "bvgk-writer-refinement",
        "open-basin" => "corpus-induction",
        other => panic!("unexpected family: {other}"),
    };

    let mut tested_hypotheses = 0;
    for paper in lit.paper.iter().take(5) {
        assert!(
            !paper.hypothesis.is_empty(),
            "Curated seed paper {} must have at least one hypothesis",
            paper.arxiv_id
        );
        for hyp in &paper.hypothesis {
            let window = match hyp.family.as_str() {
                "dialogue-stem" => vec![1, 7],
                "stem-reasoning" => vec![1, 8],
                "code-synthesis" => vec![1, 8],
                "open-basin" => vec![2, 14],
                other => panic!("unknown family: {other}"),
            };

            let claim = FrontierClaim {
                id: format!("CHAL-{}", hyp.id),
                status: "observed".to_string(),
                depends: vec!["R4-F01".to_string()],
                relation: hyp.family.clone(),
                window,
                kappa: format!("r4:v1:challenge:{}:k=8", hyp.id),
                lean: None,
                statement: hyp.statement.clone(),
                prompt: Some(hyp.prompt.clone()),
                target_reply: Some(hyp.target_reply.clone()),
            };

            let witnesses = compute_claim_witnesses(&claim, 0);
            let target_strategy = expected_strategy_for_family(&hyp.family);

            // Confirm that the target strategy SUCCEEDS
            let eval_target = evaluate_strategy_on_claim(&claim.id, target_strategy, &witnesses);
            assert!(
                eval_target.is_some(),
                "Claim {} (family {}) FAILED its designated strategy {}",
                claim.id,
                hyp.family,
                target_strategy
            );

            // Confirm that ALL OTHER 3 strategies yield None (zero cross-proving)
            for &competing_strategy in &STRATEGIES {
                if competing_strategy != target_strategy {
                    let eval_competing =
                        evaluate_strategy_on_claim(&claim.id, competing_strategy, &witnesses);
                    assert!(
                        eval_competing.is_none(),
                        "CRITICAL CROSS-PROVING BUG: Claim {} (family {}) satisfied competing strategy {}! Proof: {:?}",
                        claim.id,
                        hyp.family,
                        competing_strategy,
                        eval_competing
                    );
                }
            }

            tested_hypotheses += 1;
        }
    }

    assert_eq!(
        tested_hypotheses, 5,
        "Must have evaluated all 5 curated seed hypotheses"
    );
}

/// 2. EMPIRICAL CHALLENGE: Dynamically generated fallback claims across all 4 category families
///    satisfy ONLY their designated strategy and yield None for all 3 competing strategies.
#[test]
fn challenge_dynamic_fallback_claims_mutual_exclusivity_zero_cross_proving() {
    let root = repo_model::repo_root();

    let test_categories = [
        ("cs.LO", "stem-reasoning", "reply-4gram-deduction"),
        ("math.LO", "stem-reasoning", "reply-4gram-deduction"),
        ("cs.CR", "code-synthesis", "bvgk-writer-refinement"),
        ("cs.PL", "code-synthesis", "bvgk-writer-refinement"),
        ("cs.DC", "code-synthesis", "bvgk-writer-refinement"),
        ("cs.DS", "code-synthesis", "bvgk-writer-refinement"),
        ("cs.AI", "dialogue-stem", "pointkey-resonance"),
        ("cs.CL", "dialogue-stem", "pointkey-resonance"),
        ("cs.LG", "dialogue-stem", "pointkey-resonance"),
        ("physics.gen-ph", "open-basin", "corpus-induction"),
        ("math.PR", "open-basin", "corpus-induction"),
    ];

    let mut temp_lit = LiteratureFile {
        spec: "r4/literature/1".to_string(),
        synced_at: "2026-11-05T00:00:00Z".to_string(),
        query: "test".to_string(),
        paper: Vec::new(),
    };

    for (i, &(cat, _, _)) in test_categories.iter().enumerate() {
        temp_lit.paper.push(ArxivPaper {
            arxiv_id: format!("9999.{i:05}"),
            title: format!("Synthesized Test Paper for {cat}"),
            authors: vec!["Challenger".to_string()],
            published: "2026-11-05T00:00:00Z".to_string(),
            updated: None,
            primary_category: cat.to_string(),
            categories: vec![cat.to_string()],
            r#abstract: "Abstract content for testing dynamic fallback heuristics.".to_string(),
            hypothesis: Vec::new(), // Empty: forces dynamic fallback generation
        });
    }

    let temp_dir = std::env::temp_dir().join(format!("test_m2_chal_{}", std::process::id()));
    let _ = std::fs::create_dir_all(temp_dir.join("model"));
    engine::save_literature(&temp_dir, &temp_lit).expect("save test literature");

    let frontier = load_frontier(&root).expect("load_frontier");
    let mined = mine_literature_frontier_claims(&temp_dir, &frontier, test_categories.len())
        .expect("mine fallback claims");

    assert_eq!(
        mined.len(),
        test_categories.len(),
        "Must mine fallback claim for every unannotated paper"
    );

    for (c, &(cat, expected_family, expected_strategy)) in mined.iter().zip(test_categories.iter())
    {
        assert_eq!(
            c.relation, expected_family,
            "Category {cat} mapped to wrong family {}",
            c.relation
        );

        let w = compute_claim_witnesses(c, 0);

        // Intended strategy must succeed
        let eval_intended = evaluate_strategy_on_claim(&c.id, expected_strategy, &w);
        assert!(
            eval_intended.is_some(),
            "Fallback claim for category {} (family {}) failed its intended strategy {}",
            cat,
            expected_family,
            expected_strategy
        );

        // Competing strategies must ALL yield None
        for &competing in &STRATEGIES {
            if competing != expected_strategy {
                let eval_competing = evaluate_strategy_on_claim(&c.id, competing, &w);
                assert!(
                    eval_competing.is_none(),
                    "CROSS-PROVING BUG: Fallback claim {} for category {} (family {}) matched competing strategy {}!",
                    c.id,
                    cat,
                    expected_family,
                    competing
                );
            }
        }
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// 3. STRESS TEST: Frontier induction boundary counts (0, 1, 5, excess needed).
#[test]
fn challenge_frontier_induction_boundary_counts() {
    let root = repo_model::repo_root();
    let frontier = load_frontier(&root).expect("frontier loads");

    // Case A: needed = 0
    let mined_0 = mine_literature_frontier_claims(&root, &frontier, 0).expect("mine with needed=0");
    assert!(
        mined_0.is_empty(),
        "needed=0 must return empty vector immediately"
    );

    // Case B: needed = 1
    let mined_1 = mine_literature_frontier_claims(&root, &frontier, 1).expect("mine with needed=1");
    assert_eq!(mined_1.len(), 1, "needed=1 must return exactly 1 claim");
    assert_eq!(mined_1[0].id, "R4-F32");

    // Case C: needed = 5
    let mined_5 = mine_literature_frontier_claims(&root, &frontier, 5).expect("mine with needed=5");
    assert_eq!(mined_5.len(), 5, "needed=5 must return exactly 5 claims");

    // Case D: excess needed (e.g. 100)
    let lit = load_literature(&root).expect("load lit");
    let total_available_hypotheses = lit.paper.len(); // Each paper yields at least 1 hypothesis
    let mined_excess =
        mine_literature_frontier_claims(&root, &frontier, 100).expect("mine with needed=100");
    assert_eq!(
        mined_excess.len(),
        total_available_hypotheses,
        "Excess needed must return all available paper hypotheses without overflowing or looping"
    );
}

/// 4. STRESS TEST: Unique ID generation, zero duplicate kappas, and valid prerequisite trees.
#[test]
fn challenge_id_uniqueness_kappa_uniqueness_and_prerequisite_trees() {
    let root = repo_model::repo_root();
    let frontier = load_frontier(&root).expect("frontier loads");

    let mined = mine_literature_frontier_claims(&root, &frontier, 50).expect("mine claims");
    assert!(!mined.is_empty());

    let mut existing_ids: BTreeSet<String> = frontier.claim.iter().map(|c| c.id.clone()).collect();
    let mut existing_kappas: BTreeSet<String> =
        frontier.claim.iter().map(|c| c.kappa.clone()).collect();

    for c in &mined {
        // ID format check
        assert!(
            c.id.starts_with("R4-F"),
            "Claim ID {} must start with 'R4-F'",
            c.id
        );
        let num: usize =
            c.id.strip_prefix("R4-F")
                .unwrap()
                .parse()
                .expect("valid integer");
        assert!(num >= 32, "Induced claims must be >= R4-F32");

        // Unique ID check
        assert!(
            !existing_ids.contains(&c.id),
            "Duplicate claim ID detected: {}",
            c.id
        );
        existing_ids.insert(c.id.clone());

        // Unique Kappa check
        assert!(
            !existing_kappas.contains(&c.kappa),
            "Duplicate kappa detected: {}",
            c.kappa
        );
        assert!(
            c.kappa.starts_with("r4:v1:literature:"),
            "Literature kappa must start with 'r4:v1:literature:', got {}",
            c.kappa
        );
        existing_kappas.insert(c.kappa.clone());

        // Prerequisite dependency tree check
        assert!(!c.depends.is_empty(), "Claim {} has empty depends", c.id);
        match c.relation.as_str() {
            "dialogue-stem" => assert_eq!(c.depends, vec!["R4-F01"]),
            "stem-reasoning" => assert_eq!(c.depends, vec!["R4-F03"]),
            "code-synthesis" => assert_eq!(c.depends, vec!["R4-F04"]),
            "open-basin" => assert_eq!(c.depends, vec!["R4-F02", "R4-F24"]),
            other => panic!("unknown relation: {other}"),
        }

        // All prerequisites MUST exist and be proven in frontier
        for dep in &c.depends {
            let parent = frontier
                .claim
                .iter()
                .find(|p| p.id == *dep)
                .unwrap_or_else(|| {
                    panic!(
                        "Prerequisite {} for {} does not exist in frontier!",
                        dep, c.id
                    )
                });
            assert_eq!(
                parent.status, "proved",
                "Prerequisite {} for {} is not 'proved' (status: {})",
                dep, c.id, parent.status
            );
        }
    }
}

/// 5. STRESS TEST: Incremental mining and idempotent deduplication.
#[test]
fn challenge_incremental_mining_and_idempotency() {
    let root = repo_model::repo_root();
    let mut frontier = load_frontier(&root).expect("frontier loads");

    // Mine batch 1 (2 claims)
    let batch_1 = mine_literature_frontier_claims(&root, &frontier, 2).expect("batch 1");
    assert_eq!(batch_1.len(), 2);
    assert_eq!(batch_1[0].id, "R4-F32");
    assert_eq!(batch_1[1].id, "R4-F33");

    // Append batch 1 into frontier
    for c in batch_1 {
        frontier.claim.push(c);
    }

    // Mine batch 2 (3 claims)
    let batch_2 = mine_literature_frontier_claims(&root, &frontier, 3).expect("batch 2");
    assert_eq!(batch_2.len(), 3);
    assert_eq!(batch_2[0].id, "R4-F34");
    assert_eq!(batch_2[1].id, "R4-F35");
    assert_eq!(batch_2[2].id, "R4-F36");

    // Append batch 2 into frontier
    for c in batch_2 {
        frontier.claim.push(c);
    }

    // If all curated literature has been mined, further mining with only curated papers returns 0 or remaining
    let lit = load_literature(&root).expect("load lit");
    if lit.paper.len() <= 5 {
        let batch_3 = mine_literature_frontier_claims(&root, &frontier, 5).expect("batch 3");
        assert!(
            batch_3.is_empty(),
            "Idempotency violation: After mining all papers, subsequent mine must return 0 claims"
        );
    }
}

/// 6. ORACLE SENSITIVITY & PLANTED DEFECTS:
///    Verifies that corrupting witnesses causes strategies to reject claims (fail-closed).
#[test]
fn challenge_oracle_sensitivity_planted_witness_defects() {
    let root = repo_model::repo_root();
    let frontier = load_frontier(&root).expect("frontier");
    let mined = mine_literature_frontier_claims(&root, &frontier, 5).expect("mine");

    for c in mined {
        let base_w = compute_claim_witnesses(&c, 0);

        match c.relation.as_str() {
            "dialogue-stem" => {
                // Defect 1: Orphan rows > 0 must fail pointkey-resonance
                let mut bad_w = base_w.clone();
                bad_w.orphan_count = 1;
                assert!(
                    evaluate_strategy_on_claim(&c.id, "pointkey-resonance", &bad_w).is_none(),
                    "Oracle failed to reject orphaned rows"
                );

                // Defect 2: reply_words > 32 must fail pointkey-resonance
                let mut bad_w2 = base_w.clone();
                bad_w2.reply_words = 33;
                assert!(
                    evaluate_strategy_on_claim(&c.id, "pointkey-resonance", &bad_w2).is_none(),
                    "Oracle failed to reject excessive reply words"
                );

                // Defect 3: window_hi < 6 must fail pointkey-resonance
                let mut bad_w3 = base_w.clone();
                bad_w3.window_hi = 5;
                assert!(
                    evaluate_strategy_on_claim(&c.id, "pointkey-resonance", &bad_w3).is_none(),
                    "Oracle failed to reject narrow window"
                );
            }
            "stem-reasoning" => {
                // Defect 1: cycle_count > 0 must fail reply-4gram-deduction
                let mut bad_w = base_w.clone();
                bad_w.cycle_count = 1;
                assert!(
                    evaluate_strategy_on_claim(&c.id, "reply-4gram-deduction", &bad_w).is_none(),
                    "Oracle failed to reject cyclic replies"
                );

                // Defect 2: reply_words > 36 must fail reply-4gram-deduction
                let mut bad_w2 = base_w.clone();
                bad_w2.reply_words = 37;
                assert!(
                    evaluate_strategy_on_claim(&c.id, "reply-4gram-deduction", &bad_w2).is_none(),
                    "Oracle failed to reject oversized reply words"
                );
            }
            "code-synthesis" => {
                // Defect 1: symbolic_count = 0 must fail bvgk-writer-refinement
                let mut bad_w = base_w.clone();
                bad_w.symbolic_count = 0;
                assert!(
                    evaluate_strategy_on_claim(&c.id, "bvgk-writer-refinement", &bad_w).is_none(),
                    "Oracle failed to reject zero symbolic tokens"
                );

                // Defect 2: reply_words < 14 must fail bvgk-writer-refinement
                let mut bad_w2 = base_w.clone();
                bad_w2.reply_words = 13;
                assert!(
                    evaluate_strategy_on_claim(&c.id, "bvgk-writer-refinement", &bad_w2).is_none(),
                    "Oracle failed to reject truncated reply"
                );
            }
            "open-basin" => {
                // Defect 1: window_span < 8 must fail corpus-induction
                let mut bad_w = base_w.clone();
                bad_w.window_span = 7;
                assert!(
                    evaluate_strategy_on_claim(&c.id, "corpus-induction", &bad_w).is_none(),
                    "Oracle failed to reject insufficient window span"
                );

                // Defect 2: reply_words < 20 must fail corpus-induction
                let mut bad_w2 = base_w.clone();
                bad_w2.reply_words = 19;
                assert!(
                    evaluate_strategy_on_claim(&c.id, "corpus-induction", &bad_w2).is_none(),
                    "Oracle failed to reject reply_words < 20"
                );
            }
            _ => {}
        }
    }
}
