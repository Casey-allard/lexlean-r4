//! Compositional multi-basin PhraseDag planner and topological clause bridge controls (`R4-34`).

mod support;

#[test]
fn complete_phrase_dag_r4_34() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/PhraseDagControls.lex.tex",
        "PhraseDagControls",
        "7ff068b4519a81f3e425fd14e63b017faa6a0c92061ef2c9efee2b2ff4ed6c3c",
        &["PhraseDagControls", "R4.PhraseDag"],
    )
}
