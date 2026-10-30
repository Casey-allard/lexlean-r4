//! Verify the complete reviewed finite-geometry control graph in the SDK.

use crate::support;

#[test]
fn finite_geometry_r4_02() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/GeometryControls.lex.tex",
        "GeometryControls",
        "e8a802e25819404c09c40e6d37274a400de3e2e1281831a5e5e525721e403359",
        &["GeometryControls", "R4.Geometry"],
    )
}
