//! Finite semantic learning transitions; no training or model evaluation runs.

mod support;

#[test]
fn finite_learning_transition_r4_08() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/LearningControls.lex.tex",
        "LearningControls",
        "dcae9573caf97c5966eda17d46ce74accd454b9caf9a9d71bceab5d05f4cb66c",
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
