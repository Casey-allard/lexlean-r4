//! Configuration boundary statements verified through the SDK, never skipped.

mod support;

#[test]
fn configuration_boundaries_r4_01() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/TypesControls.lex.tex",
        "TypesControls",
        "638e03f84d45e927dd71aae6e6560137662601a1a6dfad58cb8b441409e53607",
        &["R4.Types", "TypesControls"],
    )
}
