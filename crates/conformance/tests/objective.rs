//! Scalar objective contracts; no carrier evaluation or training is executed.

mod support;

#[test]
fn scalar_objective_r4_03() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/ObjectiveControls.lex.tex",
        "ObjectiveControls",
        "9934a492d9d751ed0fde07f193b5a2582e70219b8a3f2eb72b595f955b2d77fc",
        &["ObjectiveControls", "R4.Objective"],
    )
}
