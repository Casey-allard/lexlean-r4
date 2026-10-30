//! Formal Prediction contracts (R4-19); target-free projection of the supervised writer.

mod support;

#[test]
fn target_free_prediction_r4_19() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/PredictionControls.lex.tex",
        "PredictionControls",
        "677087de1a6b844dd353e87909d96d9feb95d8cb2d07108f527d26698d3a5cde",
        &[
            "PredictionControls",
            "R4.Admission",
            "R4.Carrier",
            "R4.Geometry",
            "R4.Objective",
            "R4.Prediction",
            "R4.Types",
        ],
    )
}
