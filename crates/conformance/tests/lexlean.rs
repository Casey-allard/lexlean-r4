//! Configuration boundary statements verified through the SDK, never skipped.

mod support;

#[test]
fn configuration_boundaries_r4_01() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/TypesControls.lex.tex",
        "TypesControls",
        "38671c44f31d5475abb570c6dc8c4b82f57fc7937277d9d35d0a63442a31be1a",
        &["R4.Types", "TypesControls"],
    )
}
