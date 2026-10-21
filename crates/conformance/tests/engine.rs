//! Formal Engine composition contracts; no host model or GPU execution.

mod support;

#[test]
fn complete_engine_composition_r4_12() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/EngineControls.lex.tex",
        "EngineControls",
        "f7d77d5fc81768a7f9376e67cfc871ed82a39d00ff9e7b2b5cb2013df963ce9c",
        &[
            "EngineControls",
            "R4.Admission",
            "R4.Binder",
            "R4.Carrier",
            "R4.Configuration",
            "R4.Engine",
            "R4.Flock",
            "R4.Gates",
            "R4.Geometry",
            "R4.Learning",
            "R4.Objective",
            "R4.ReadArithmetic",
            "R4.Relation",
            "R4.Route",
            "R4.Types",
            "R4.Words",
        ],
    )
}
