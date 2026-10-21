//! Read configuration metadata; no carrier or research compute is executed.

mod support;

#[test]
fn typed_configuration_r4_06() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/ConfigurationControls.lex.tex",
        "ConfigurationControls",
        "73c945a6bff74ebbaa3dd5ce54c81b20549e749bede25ac1c509b3713963a5fb",
        &[
            "ConfigurationControls",
            "R4.Configuration",
            "R4.Objective",
            "R4.Types",
        ],
    )
}
