//! Verify the complete reviewed finite-geometry control graph in the SDK.

mod support;

#[test]
fn finite_geometry_r4_02() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/GeometryControls.lex.tex",
        "GeometryControls",
        "5574f2c697d141d78e2d099bace7b8a0d5fd3b1c9be6449607de547295837a9e",
        &["GeometryControls", "R4.Geometry"],
    )
}
