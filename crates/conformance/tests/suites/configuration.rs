//! Read configuration metadata; no carrier or research compute is executed.

use crate::support;

#[test]
fn typed_configuration_r4_06() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/ConfigurationControls.lex.tex",
        "ConfigurationControls",
        "7a77d8f112af2aadda0da5e3b41c6e6db34970bf1a8e7a3268a418a00d8743d5",
        &[
            "ConfigurationControls",
            "R4.Configuration",
            "R4.Objective",
            "R4.Types",
        ],
    )
}
