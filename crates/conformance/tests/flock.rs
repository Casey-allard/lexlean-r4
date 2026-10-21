//! Formal Flock/Relation source contracts; no host model or GPU execution.

mod support;

#[test]
fn complete_flock_relation_r4_10() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/FlockControls.lex.tex",
        "FlockControls",
        "b5d21afc86c3a6feccda2c79d006caad7d257713536a5749d66497558a66992c",
        &[
            "FlockControls",
            "R4.Binder",
            "R4.Carrier",
            "R4.Configuration",
            "R4.Flock",
            "R4.Gates",
            "R4.Geometry",
            "R4.Objective",
            "R4.ReadArithmetic",
            "R4.Relation",
            "R4.Route",
            "R4.Types",
        ],
    )
}
