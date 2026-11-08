//! Prism Factory autonomous self-improvement, content-addressed registry, receipt routing, and 28-obligation fulfillment controls (`R4-36`).

mod support;

#[test]
fn complete_prism_factory_r4_36() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/PrismFactoryControls.lex.tex",
        "PrismFactoryControls",
        "19d020cf0cd445885b0b4545e0127702d5a2a0f5220ff6e3de833538775650e3",
        &["PrismFactoryControls", "R4.PrismFactory"],
    )
}
