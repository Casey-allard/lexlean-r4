//! Conformance test for R4-53: UORust total route table and Prism-DevRel ISO 15939 / PROV-O gates.

#[path = "../../wasm/src/lexlean_generated.rs"]
mod lexlean_generated;

use lexlean_generated::r4_uorust_prism_devrel::{
    admit_route_and_measures, dense_matmul_ops, evaluate_measure_criterion, float_serving_ops,
    kappa_residue, route_table_total_count, uncovered_records, undeclared_references,
    MeasureDirection, RouteAdmissionVerdict, RouteAndMeasureCheck,
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

fn valid_route_check() -> RouteAndMeasureCheck {
    RouteAndMeasureCheck {
        known_fact_match: true,
        matched_route_count: 1,
        direction: MeasureDirection::AtMost,
        measured_value: 0,
        threshold_value: 0,
        missing_provenance_sources: 0,
    }
}

#[test]
fn test_r4_uorust_prism_devrel_r4_53() {
    let root = repo_root();

    check_lex_module(
        &root,
        "src/R4/UorustPrismDevrel.lex.tex",
        "R4.UorustPrismDevrel",
        &[
            "uncoveredRecords",
            "undeclaredReferences",
            "floatServingOps",
            "denseMatmulOps",
            "kappaResidue",
            "routeTableTotalCount",
            "evaluateMeasureCriterion",
            "admitRouteAndMeasures",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/UorustPrismDevrelControls.lex.tex",
        "UorustPrismDevrelControls",
        &[
            "routeAndMeasureAdmitted",
            "breaksUnknownRouteFact",
            "breaksAmbiguousRoute",
            "breaksMeasureThresholdViolation",
        ],
    );

    assert_eq!(
        admit_route_and_measures(valid_route_check()),
        RouteAdmissionVerdict::RouteAdmitted
    );
    assert_eq!(
        admit_route_and_measures(RouteAndMeasureCheck {
            known_fact_match: false,
            ..valid_route_check()
        }),
        RouteAdmissionVerdict::RefusedUnknownRouteFact
    );
    assert_eq!(
        admit_route_and_measures(RouteAndMeasureCheck {
            matched_route_count: 2,
            ..valid_route_check()
        }),
        RouteAdmissionVerdict::RefusedAmbiguousRoute
    );
    assert_eq!(
        admit_route_and_measures(RouteAndMeasureCheck {
            direction: MeasureDirection::InvalidDirection,
            ..valid_route_check()
        }),
        RouteAdmissionVerdict::RefusedInvalidMeasureDirection
    );
    assert_eq!(
        admit_route_and_measures(RouteAndMeasureCheck {
            missing_provenance_sources: 1,
            ..valid_route_check()
        }),
        RouteAdmissionVerdict::RefusedMissingProvenanceSource
    );
    assert_eq!(
        admit_route_and_measures(RouteAndMeasureCheck {
            measured_value: 1,
            ..valid_route_check()
        }),
        RouteAdmissionVerdict::RefusedMeasureThresholdViolated
    );

    assert_eq!(uncovered_records(53, 53), 0);
    assert_eq!(undeclared_references(12, 12), 0);
    assert_eq!(float_serving_ops(0), 0);
    assert_eq!(dense_matmul_ops(0), 0);
    assert_eq!(kappa_residue(0), 0);
    assert_eq!(route_table_total_count(9), 9);
    assert!(evaluate_measure_criterion(MeasureDirection::AtLeast, 9, 9));
    assert!(!evaluate_measure_criterion(MeasureDirection::AtMost, 1, 0));

    let routes_toml = std::fs::read_to_string(root.join("model/routes.toml"))
        .expect("model/routes.toml must exist");
    assert!(routes_toml.contains("spec = \"uorust/routes/1\""));
    let measures_toml = std::fs::read_to_string(root.join("model/measures.toml"))
        .expect("model/measures.toml must exist");
    assert!(measures_toml.contains("spec = \"prism/measures/1\""));
    let prov_toml = std::fs::read_to_string(root.join("model/provenance.toml"))
        .expect("model/provenance.toml must exist");
    assert!(prov_toml.contains("spec = \"prism/provenance-sources/1\""));

    // Gate integration check (R3 fails until Step 5 completes).
    let gate_rs = std::fs::read_to_string(root.join("xtask/src/routes_and_measures.rs"))
        .expect("xtask/src/routes_and_measures.rs must exist");
    assert!(gate_rs.contains("check_routes"));
    assert!(gate_rs.contains("check_measures"));
}
