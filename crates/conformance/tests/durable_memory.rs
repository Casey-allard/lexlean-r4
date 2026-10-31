//! M3 scoped durable turn memory (`UORMEM01`/`UR4SESS1`), sorted-merge PrimeAddress GCD sieve, and +3-sigma manifold sketch controls.

mod support;

#[test]
fn complete_durable_memory_r4_26() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/DurableMemoryControls.lex.tex",
        "DurableMemoryControls",
        "b3786cb1962339e482830d2cae4288fbdd2ad3c9774abedf82ebd33f17449913",
        &["DurableMemoryControls", "R4.DurableMemory"],
    )
}
