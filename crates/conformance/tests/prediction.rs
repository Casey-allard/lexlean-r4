//! Formal Prediction contracts (R4-19); target-free projection of the supervised writer.

mod support;

#[test]
fn target_free_prediction_r4_19() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/PredictionControls.lex.tex",
        "PredictionControls",
        "07d1bd83243a26b3e5defc665c1579252e0765eb495ec4f6f04064f284fea8e2",
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
