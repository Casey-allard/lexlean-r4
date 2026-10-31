//! 19-table M1 discrete trunk specification, base-17 4-block root packing, causal orbit-root successor, and Level-3 joint pointer controls.

mod support;

#[test]
fn complete_discrete_trunk_r4_25() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/DiscreteTrunkControls.lex.tex",
        "DiscreteTrunkControls",
        "8abd1d264aa982fe6c56635516324596a3a34924c440b7d6bcbfdd20124d15f9",
        &["DiscreteTrunkControls", "R4.DiscreteTrunk"],
    )
}
