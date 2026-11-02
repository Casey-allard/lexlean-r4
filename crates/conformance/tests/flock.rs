//! Formal Flock/Relation source contracts; no host model or GPU execution.

mod support;

#[test]
fn complete_flock_relation_r4_10() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/FlockControls.lex.tex",
        "FlockControls",
        "764cd8a142980877c52ed8ef3f2aa89128c460b618d34fa2f20946df96f15187",
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
