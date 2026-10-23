//! Scalar objective contracts; no carrier evaluation or training is executed.

mod support;

#[test]
fn scalar_objective_r4_03() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/ObjectiveControls.lex.tex",
        "ObjectiveControls",
        "b733435912e985f84b2ced18c06d802300768b3564408a58ddd7dff46c930830",
        &["ObjectiveControls", "R4.Objective"],
    )
}
