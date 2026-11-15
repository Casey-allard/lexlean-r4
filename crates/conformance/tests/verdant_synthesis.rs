//! Conformance test for R4-45: Verdant-Resonance AST mechanizer, UORust
//! `uorust/function-kappa/1` lift ratchet, and 4-facet Pareto synthesis engine.

#[path = "../../wasm/src/lexlean_generated.rs"]
mod lexlean_generated;

use lexlean_generated::r4_verdant_synthesis::{
    admit_lift_register_bounds, admit_lift_transition, admit_mechanizer_graph,
    admit_pareto_synthesis_tick, update_bandit_weight, LiftRatchetVerdict, LiftStatusCode,
    MechanizerCheck, MechanizerVerdict, ParetoSynthesisCheck, ParetoSynthesisVerdict,
    SynthesisFacetKind,
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

#[test]
fn test_verdant_mechanizer_lift_ratchet_and_synthesis_r4_45() {
    let root = repo_root();

    check_lex_module(
        &root,
        "src/R4/VerdantSynthesis.lex.tex",
        "R4.VerdantSynthesis",
        &[
            "admitMechanizerGraph",
            "admitLiftTransition",
            "admitLiftRegisterBounds",
            "admitParetoSynthesisTick",
            "updateBanditWeight",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/VerdantSynthesisControls.lex.tex",
        "VerdantSynthesisControls",
        &[
            "paretoZeroDefectAdmitted",
            "breaksCycleMechanizerAdmitted",
            "breaksNonZeroDefectParetoAdmitted",
        ],
    );

    // Verify model/ratchet.toml enforces zero floats, zero dense matmuls, zero unwired controls.
    let ratchet_toml =
        std::fs::read_to_string(root.join("model/ratchet.toml")).expect("read model/ratchet.toml");
    for required in [
        "max_residue_functions",
        "max_float_serving_ops = 0",
        "max_dense_matmul_ops = 0",
        "max_unwired_ui_controls = 0",
    ] {
        assert!(
            ratchet_toml.contains(required),
            "model/ratchet.toml must enforce `{required}`"
        );
    }

    // Verify model/lift_register.json uses uorust/function-kappa/1 schema cleanly.
    let lift_raw = std::fs::read_to_string(root.join("model/lift_register.json"))
        .expect("model/lift_register.json must be committed");
    let lift_json: Value = serde_json::from_str(&lift_raw).expect("valid lift_register.json");
    assert_eq!(lift_json["domain"], "uorust/function-kappa/1");
    assert!(lift_json["summary"]["total_functions"]
        .as_u64()
        .is_some_and(|n| n > 0));

    // Verify mechanizer admission and typed refusals (R5).
    let valid_mech = MechanizerCheck {
        dag_acyclic: true,
        all_roots_executable: true,
        duplicate_decls: 0,
        breaks_triggered: 0,
        verify_passed: 8,
        verify_total: 8,
    };
    assert_eq!(
        admit_mechanizer_graph(valid_mech),
        MechanizerVerdict::ClosureAdmitted
    );
    assert_eq!(
        admit_mechanizer_graph(MechanizerCheck {
            dag_acyclic: false,
            ..valid_mech
        }),
        MechanizerVerdict::RefusedCycleInModuleGraph
    );
    assert_eq!(
        admit_mechanizer_graph(MechanizerCheck {
            duplicate_decls: 1,
            ..valid_mech
        }),
        MechanizerVerdict::RefusedDuplicateDeclaration
    );
    assert_eq!(
        admit_mechanizer_graph(MechanizerCheck {
            all_roots_executable: false,
            ..valid_mech
        }),
        MechanizerVerdict::RefusedNonExecutableRoot
    );
    assert_eq!(
        admit_mechanizer_graph(MechanizerCheck {
            breaks_triggered: 1,
            ..valid_mech
        }),
        MechanizerVerdict::RefusedCounterexampleTriggered
    );

    // Verify UE-16 kappa-lift transition monotonicity & bounds.
    assert_eq!(
        admit_lift_transition(true, LiftStatusCode::Lifted, true, false),
        LiftRatchetVerdict::LiftAdmitted
    );
    assert_eq!(
        admit_lift_transition(true, LiftStatusCode::Residue, true, false),
        LiftRatchetVerdict::RefusedKappaRegressed
    );
    assert_eq!(
        admit_lift_transition(true, LiftStatusCode::Lifted, true, true),
        LiftRatchetVerdict::RefusedKappaRegressed
    );
    assert_eq!(
        admit_lift_register_bounds(0, 0, true),
        LiftRatchetVerdict::LiftAdmitted
    );
    assert_eq!(
        admit_lift_register_bounds(1, 0, true),
        LiftRatchetVerdict::RefusedResidueCeilingExceeded
    );
    assert_eq!(
        admit_lift_register_bounds(0, 0, false),
        LiftRatchetVerdict::RefusedMissingRowType
    );

    // Verify all 4 SynthesisFacetKind variants & Pareto/Lyapunov tick admission.
    let _facets = [
        SynthesisFacetKind::ModelAndTraining,
        SynthesisFacetKind::ServingRuntime,
        SynthesisFacetKind::UiAndPlumbing,
        SynthesisFacetKind::EcosystemStandards,
    ];
    let valid_pareto = ParetoSynthesisCheck {
        candidate_defects: 0,
        candidate_score_milli: 910,
        incumbent_score_milli: 900,
        candidate_complexity: 12,
        incumbent_complexity: 12,
        bandit_weight: 8,
        ratchet_clean: true,
    };
    assert_eq!(
        admit_pareto_synthesis_tick(valid_pareto),
        ParetoSynthesisVerdict::CandidateAdmitted
    );
    assert_eq!(
        admit_pareto_synthesis_tick(ParetoSynthesisCheck {
            candidate_defects: 1,
            ..valid_pareto
        }),
        ParetoSynthesisVerdict::RefusedNonZeroDefect
    );
    assert_eq!(
        admit_pareto_synthesis_tick(ParetoSynthesisCheck {
            ratchet_clean: false,
            ..valid_pareto
        }),
        ParetoSynthesisVerdict::RefusedRatchetViolation
    );
    assert_eq!(
        admit_pareto_synthesis_tick(ParetoSynthesisCheck {
            bandit_weight: 0,
            ..valid_pareto
        }),
        ParetoSynthesisVerdict::RefusedWeightOutOfBounds
    );
    assert_eq!(
        admit_pareto_synthesis_tick(ParetoSynthesisCheck {
            candidate_score_milli: 900,
            candidate_complexity: 12,
            ..valid_pareto
        }),
        ParetoSynthesisVerdict::RefusedNotParetoImproving
    );

    // Verify [1..16] +2/-1 Hebbian bandit plasticity.
    assert_eq!(update_bandit_weight(8, true), 10);
    assert_eq!(update_bandit_weight(15, true), 16);
    assert_eq!(update_bandit_weight(8, false), 7);
    assert_eq!(update_bandit_weight(1, false), 1);
}
