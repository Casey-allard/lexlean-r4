//! Finite semantic learning transitions; no training or model evaluation runs.

mod support;

#[test]
fn finite_learning_transition_r4_08() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/LearningControls.lex.tex",
        "LearningControls",
        "105d80d6ec691e8b611d852c3f1a3be0f01b6ee3fcb770acb0c583b8e86f8786",
        &[
            "LearningControls",
            "R4.Admission",
            "R4.Carrier",
            "R4.Configuration",
            "R4.Gates",
            "R4.Geometry",
            "R4.Learning",
            "R4.Objective",
            "R4.Types",
        ],
    )
}
