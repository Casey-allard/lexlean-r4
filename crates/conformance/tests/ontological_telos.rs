//! Conformance test for R4-55: Verdant Ontological Telos synthesis, Holo prefix-KV continuity,
//! bounded ReAct steps, and non-synthetic per-layer trace emission.

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_ontological_telos::{
    admit_ontological_telos, compute_host_ceiling_excess, evaluate_total_telos_loss,
    step_parametric_telos_coordinate, verify_ontological_telos_admitted,
    verify_w5h_certificate_complete, OntologicalTelosCheck, TelosVerdict,
};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn check_lex_module(root: &Path, rel: &str, expected_mod: &str, required_decls: &[&str]) {
    let raw =
        std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("missing {rel}: {e}"));
    assert!(raw.contains(&format!("\\begin{{lexlean}}{{{expected_mod}}}")));
    let (_, after) = raw
        .split_once("\\semanticdata{")
        .unwrap_or_else(|| panic!("missing \\semanticdata{{ in {rel}"));
    let (json_block, _) = after
        .split_once("\n\\end{semanticmodule}")
        .unwrap_or_else(|| panic!("missing \\end{{semanticmodule}} in {rel}"));
    let json_line = json_block
        .trim()
        .strip_suffix('}')
        .unwrap_or(json_block.trim());
    let parsed: Value = serde_json::from_str(json_line).expect("valid JSON semantic module");
    assert_eq!(parsed["spec"], "lexlean/semantic-module/2");
    for sym in required_decls {
        assert!(raw.contains(sym), "{rel} must declare {sym}");
    }
}

fn valid_telos_check() -> OntologicalTelosCheck {
    OntologicalTelosCheck {
        opacity_debt: 0,
        legibility_gap: 0,
        observer_dissonance: 0,
        budget_micros: 50_000,
        observed_micros: 18_000,
        w5h_verified_count: 6,
        w5h_orphan_count: 0,
        coercive_count: 0,
        candidate_loss: 0,
        incumbent_loss: 4,
        cached_prefix_len: 64,
        prompt_len: 96,
        matched_prefix: 60,
        react_hop: 1,
        react_max_hops: 4,
        react_tool_valid: true,
        layer_count: 28,
        real_norms_recorded: true,
        post_hoc_modulo_synthesized: false,
        float_ops: 0,
    }
}

#[test]
fn test_r4_ontological_telos_r4_55() {
    let root = repo_root();

    check_lex_module(
        &root,
        "src/R4/OntologicalTelos.lex.tex",
        "R4.OntologicalTelos",
        &[
            "computeHostCeilingExcess",
            "evaluateTotalTelosLoss",
            "verifyW5HCertificateComplete",
            "stepParametricTelosCoordinate",
            "admitOntologicalTelos",
            "verifyOntologicalTelosAdmitted",
            "breaksIncompleteW5H",
            "breaksCoerciveStep",
            "breaksPrefixOverrun",
            "breaksReactFuelExhausted",
            "breaksSyntheticModuloPulse",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/OntologicalTelosControls.lex.tex",
        "OntologicalTelosControls",
        &[
            "ontologicalTelosAdmitted",
            "breaksIncompleteW5HCtrl",
            "breaksCoerciveStepCtrl",
            "breaksPrefixOverrunCtrl",
            "breaksSyntheticModuloPulseCtrl",
        ],
    );

    assert!(verify_ontological_telos_admitted());
    assert_eq!(compute_host_ceiling_excess(50_000, 18_000), 0);
    assert_eq!(compute_host_ceiling_excess(50_000, 62_500), 12_500);
    assert_eq!(evaluate_total_telos_loss(1, 2, 3, 4), 10);
    assert!(verify_w5h_certificate_complete(7, 2, 11, 3, 1, true));
    assert!(!verify_w5h_certificate_complete(7, 2, 0, 3, 1, true));

    // Non-coercive coordinate descent steps strictly downward in loss
    assert_eq!(
        step_parametric_telos_coordinate(4, 1, 8, 2, 5, 6, 0),
        3,
        "lossMinus < lossCurrent steps coord down by 1"
    );
    assert_eq!(
        step_parametric_telos_coordinate(4, 1, 8, 6, 5, 2, 0),
        5,
        "lossPlus < lossCurrent steps coord up by 1"
    );
    assert_eq!(
        step_parametric_telos_coordinate(4, 1, 8, 2, 5, 6, 1),
        4,
        "coercive step must freeze coordinate"
    );

    // Positive admission & typed R5 negative refusals
    assert_eq!(
        admit_ontological_telos(valid_telos_check()),
        TelosVerdict::TelosAdmitted
    );
    assert_eq!(
        admit_ontological_telos(OntologicalTelosCheck {
            w5h_orphan_count: 1,
            ..valid_telos_check()
        }),
        TelosVerdict::RefusedIncompleteW5HCertificate
    );
    assert_eq!(
        admit_ontological_telos(OntologicalTelosCheck {
            coercive_count: 1,
            ..valid_telos_check()
        }),
        TelosVerdict::RefusedCoerciveStep
    );
    assert_eq!(
        admit_ontological_telos(OntologicalTelosCheck {
            candidate_loss: 9,
            incumbent_loss: 4,
            ..valid_telos_check()
        }),
        TelosVerdict::RefusedTelosLossRegression
    );
    assert_eq!(
        admit_ontological_telos(OntologicalTelosCheck {
            matched_prefix: 96,
            prompt_len: 96,
            ..valid_telos_check()
        }),
        TelosVerdict::RefusedPrefixBoundsViolation
    );
    assert_eq!(
        admit_ontological_telos(OntologicalTelosCheck {
            react_hop: 4,
            react_max_hops: 4,
            ..valid_telos_check()
        }),
        TelosVerdict::RefusedReactFuelExhausted
    );
    assert_eq!(
        admit_ontological_telos(OntologicalTelosCheck {
            post_hoc_modulo_synthesized: true,
            ..valid_telos_check()
        }),
        TelosVerdict::RefusedSyntheticModuloPulse
    );
}
