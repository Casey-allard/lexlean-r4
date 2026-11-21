//! Conformance test for R4-83: Generative Second-Order Observer Refinement Agenda
//! (`R4.ObserverRefinementAgenda` / `xtask/src/observer_agenda.rs` / `model/observer_refinement_agenda.json`).

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_observer_refinement_agenda::{
    admit_observer_refinement_agenda, part_health_score_permille,
    verify_observer_refinement_agenda_admitted, AgendaVerdict,
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
fn test_r4_83_observer_refinement_agenda_r4_83() {
    let root = repo_root();

    // 1. Formal LexLean R4-83 admission, permille modularity health, and all 4 R5 refusals
    assert!(verify_observer_refinement_agenda_admitted());
    assert_eq!(part_health_score_permille(1200, 1500, 10000), 1000);
    assert_eq!(part_health_score_permille(3000, 1500, 10000), 900);
    assert_eq!(part_health_score_permille(8000, 1500, 10000), 800);
    assert_eq!(part_health_score_permille(10500, 1500, 10000), 750);

    assert_eq!(
        admit_observer_refinement_agenda(6, 4, 2443, 2442, 128, 129, true),
        AgendaVerdict::AgendaAdmitted
    );
    assert_eq!(
        admit_observer_refinement_agenda(5, 4, 2443, 2442, 128, 129, true),
        AgendaVerdict::RefusedIncompleteOrganCoverage
    );
    assert_eq!(
        admit_observer_refinement_agenda(6, 0, 2443, 2442, 128, 129, true),
        AgendaVerdict::RefusedEmptyProposalSet
    );
    assert_eq!(
        admit_observer_refinement_agenda(6, 4, 2442, 2442, 129, 129, true),
        AgendaVerdict::RefusedNonMonotonicWatermarkTarget
    );
    assert_eq!(
        admit_observer_refinement_agenda(6, 4, 2443, 2442, 128, 129, false),
        AgendaVerdict::RefusedUnboundAgendaKappa
    );

    // 2. Verify .lex.tex semantic modules exist and declare required symbols
    check_lex_module(
        &root,
        "src/R4/ObserverRefinementAgenda.lex.tex",
        "R4.ObserverRefinementAgenda",
        &[
            "AgendaVerdict",
            "partHealthScorePermille",
            "admitObserverRefinementAgenda",
            "verifyObserverRefinementAgendaAdmitted",
            "breaksIncompleteOrganCoverageAdmitted",
            "breaksEmptyProposalSetAdmitted",
            "breaksNonMonotonicWatermarkTargetAdmitted",
            "breaksUnboundAgendaKappaAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/ObserverRefinementAgendaControls.lex.tex",
        "ObserverRefinementAgendaControls",
        &[
            "observerRefinementAgendaAdmitted",
            "breaksIncompleteOrganCoverageAdmittedCtrl",
            "breaksEmptyProposalSetAdmittedCtrl",
            "breaksNonMonotonicWatermarkTargetAdmittedCtrl",
            "breaksUnboundAgendaKappaAdmittedCtrl",
        ],
    );

    // 3. Verify xtask/src/observer_agenda.rs and committed model/observer_refinement_agenda.json
    let agenda_rs = std::fs::read_to_string(root.join("xtask/src/observer_agenda.rs"))
        .expect("xtask/src/observer_agenda.rs must exist");
    assert!(
        agenda_rs.contains("pub fn synthesize_observer_refinement_agenda(")
            && agenda_rs.contains("pub fn cmd_observer_agenda("),
        "observer_agenda.rs must expose synthesize_observer_refinement_agenda and cmd_observer_agenda"
    );

    let agenda_json_raw =
        std::fs::read_to_string(root.join("model/observer_refinement_agenda.json"))
            .expect("model/observer_refinement_agenda.json must exist");
    let agenda: Value =
        serde_json::from_str(&agenda_json_raw).expect("valid observer_refinement_agenda.json");
    assert_eq!(agenda["spec"], "r4/observer-refinement-agenda/1");
    let organs = agenda["part_health"].as_array().expect("part_health array");
    assert_eq!(organs.len(), 6, "must inspect all 6 accountable organs");
    let proposals = agenda["proposals"].as_array().expect("proposals array");
    assert!(
        !proposals.is_empty(),
        "ObserverRefinementAgenda must emit prioritized self-refinement proposals"
    );
    let hex = agenda["agenda_kappa_hex"]
        .as_str()
        .expect("agenda_kappa_hex string");
    assert_eq!(hex.len(), 64, "agenda_kappa_hex must be 64-hex SHA-256");

    // 4. Recompute per-proposal SHA-256 kappa_hex and cross-check live_metrics against model/lift_register.json
    use sha2::{Digest, Sha256};
    for prop in proposals {
        let preimage = format!(
            "{}|{}|{}|{}|{}|{}",
            prop["proposal_id"].as_str().unwrap(),
            prop["owning_organ"].as_str().unwrap(),
            prop["cited_authority"].as_str().unwrap(),
            prop["current_value"].as_str().unwrap(),
            prop["target_value"].as_str().unwrap(),
            prop["summary"].as_str().unwrap(),
        );
        let expected: String = Sha256::digest(preimage.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(
            prop["kappa_hex"].as_str().unwrap(),
            expected,
            "proposal kappa_hex must match SHA-256 over canonical preimage"
        );
    }

    let lift_raw = std::fs::read_to_string(root.join("model/lift_register.json"))
        .expect("read model/lift_register.json");
    let lift_doc: Value = serde_json::from_str(&lift_raw).expect("parse lift_register.json");
    assert_eq!(
        agenda["live_metrics"]["lifted_functions"], lift_doc["summary"]["lifted"],
        "observer_refinement_agenda.json lifted_functions must equal model/lift_register.json"
    );
    assert_eq!(
        agenda["live_metrics"]["residue_functions"], lift_doc["summary"]["residue"],
        "observer_refinement_agenda.json residue_functions must equal model/lift_register.json"
    );
}
