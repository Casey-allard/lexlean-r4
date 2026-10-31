//! SpiralCore 17-root Icosian Z[phi] snap kernels, UORLUT01 `/1`..`/8` format, and model-source admission controls.

mod support;

#[test]
fn complete_spiralcore_fbs_r4_24() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/SpiralCoreFbsControls.lex.tex",
        "SpiralCoreFbsControls",
        "5a235e3d4b325a72f7875e3c4abd124606ce3d844520024e83f458ace5a2863d",
        &["R4.SpiralCoreFbs", "SpiralCoreFbsControls"],
    )
}
