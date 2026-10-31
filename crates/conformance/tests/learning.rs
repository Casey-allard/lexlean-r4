//! Finite semantic learning transitions; no training or model evaluation runs.

mod support;

#[test]
fn finite_learning_transition_r4_08() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/LearningControls.lex.tex",
        "LearningControls",
        "443572a7cb4cd0c60956546914d958113d810af7863f6cf0ff4002b06dfb1e3a",
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
