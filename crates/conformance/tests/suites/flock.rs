//! Formal Flock/Relation source contracts; no host model or GPU execution.

use crate::support;

#[test]
fn complete_flock_relation_r4_10() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/FlockControls.lex.tex",
        "FlockControls",
        "afca841584e0221ab4b3340ae014d9e159761460e81b9c2136c220f673ac3b9a",
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
