//! Conformance test for R4-41: Verdant-Resonance Peirce triadic cycle and shrink-only ratchet.

#[path = "../../wasm/src/lexlean_generated.rs"]
mod lexlean_generated;

use lexlean_generated::r4_autonomous_engine::{
    admit_peirce_triadic_cycle, PeircePhase, PeirceTriadicCheck, PeirceTriadicVerdict,
};
use lexlean_generated::r4_prism_factory::admit_ratchet_obligation;
use lexlean_generated::r4_rule30_evolution::{admit_shrink_only_ratchet, RatchetMetricVerdict};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn check_module_decl(root: &Path, rel: &str, expected_mod: &str, expected_symbol: &str) {
    let raw =
        std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("missing {rel}: {e}"));
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
    assert!(raw.contains(&format!("\\begin{{lexlean}}{{{expected_mod}}}")));
    assert_eq!(parsed["spec"], "lexlean/semantic-module/2");
    assert!(
        raw.contains(expected_symbol),
        "{rel} must declare {expected_symbol}"
    );
}

#[test]
fn test_verdant_peirce_triadic_and_shrink_only_ratchet_r4_41() {
    let root = repo_root();

    // Verify authoritative ratchet register exists and adheres to r4/ratchet/1.
    let ratchet_toml = std::fs::read_to_string(root.join("model/ratchet.toml"))
        .expect("model/ratchet.toml must exist");
    assert!(ratchet_toml.contains("spec = \"r4/ratchet/1\""));
    assert!(ratchet_toml.contains("rule30_rule_number = 30"));
    assert!(ratchet_toml.contains("Abduction"));
    assert!(ratchet_toml.contains("Deduction"));
    assert!(ratchet_toml.contains("Induction"));

    // Verify LexLean models carry the triadic and shrink-only definitions.
    check_module_decl(
        &root,
        "src/R4/AutonomousEngine.lex.tex",
        "R4.AutonomousEngine",
        "admitPeirceTriadicCycle",
    );
    check_module_decl(
        &root,
        "src/R4/PrismFactory.lex.tex",
        "R4.PrismFactory",
        "admitRatchetObligation",
    );
    check_module_decl(
        &root,
        "src/R4/Rule30Evolution.lex.tex",
        "R4.Rule30Evolution",
        "admitShrinkOnlyRatchet",
    );

    // Positive Peirce triadic steps.
    for phase in [
        PeircePhase::Abduction,
        PeircePhase::Deduction,
        PeircePhase::Induction,
    ] {
        assert_eq!(
            admit_peirce_triadic_cycle(PeirceTriadicCheck {
                phase,
                witness_present: true,
                invariant_preserved: true,
                metric_improved_or_held: true,
                rule30_nonzero: true,
            }),
            PeirceTriadicVerdict::TriadicStepAdmitted
        );
    }

    // Typed negative checks for Peirce triadic admission (R5).
    assert_eq!(
        admit_peirce_triadic_cycle(PeirceTriadicCheck {
            phase: PeircePhase::Abduction,
            witness_present: false,
            invariant_preserved: true,
            metric_improved_or_held: true,
            rule30_nonzero: true,
        }),
        PeirceTriadicVerdict::RefusedMissingWitness
    );
    assert_eq!(
        admit_peirce_triadic_cycle(PeirceTriadicCheck {
            phase: PeircePhase::Deduction,
            witness_present: true,
            invariant_preserved: false,
            metric_improved_or_held: true,
            rule30_nonzero: true,
        }),
        PeirceTriadicVerdict::RefusedInvariantViolation
    );
    assert_eq!(
        admit_peirce_triadic_cycle(PeirceTriadicCheck {
            phase: PeircePhase::Induction,
            witness_present: true,
            invariant_preserved: true,
            metric_improved_or_held: false,
            rule30_nonzero: true,
        }),
        PeirceTriadicVerdict::RefusedMetricRegression
    );
    assert_eq!(
        admit_peirce_triadic_cycle(PeirceTriadicCheck {
            phase: PeircePhase::Abduction,
            witness_present: true,
            invariant_preserved: true,
            metric_improved_or_held: true,
            rule30_nonzero: false,
        }),
        PeirceTriadicVerdict::RefusedZeroRule30Seed
    );

    // Shrink-only obligation and residual checks.
    assert!(admit_ratchet_obligation(12, 12, true));
    assert!(admit_ratchet_obligation(8, 12, true));
    assert!(!admit_ratchet_obligation(13, 12, true));
    assert!(!admit_ratchet_obligation(8, 12, false));

    assert_eq!(
        admit_shrink_only_ratchet(50, 42, true),
        RatchetMetricVerdict::ShrinkAdmitted
    );
    assert_eq!(
        admit_shrink_only_ratchet(50, 50, true),
        RatchetMetricVerdict::ShrinkAdmitted
    );
    assert_eq!(
        admit_shrink_only_ratchet(50, 51, true),
        RatchetMetricVerdict::RefusedResidualRegression
    );
    assert_eq!(
        admit_shrink_only_ratchet(50, 42, false),
        RatchetMetricVerdict::RefusedDeadRule30Seed
    );
}
