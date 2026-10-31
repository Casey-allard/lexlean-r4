//! 19-table M1 discrete trunk specification, base-17 4-block root packing, causal orbit-root successor, and Level-3 joint pointer controls.

mod support;

#[test]
fn complete_discrete_trunk_r4_25() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/DiscreteTrunkControls.lex.tex",
        "DiscreteTrunkControls",
        "92a8fea675031aad4815430cc1d3ecff706cf1f977f9dc5f075d917b0ae9b183",
        &["DiscreteTrunkControls", "R4.DiscreteTrunk"],
    )
}
