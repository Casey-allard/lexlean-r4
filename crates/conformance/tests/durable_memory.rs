//! M3 scoped durable turn memory (`UORMEM01`/`UR4SESS1`), sorted-merge PrimeAddress GCD sieve, and +3-sigma manifold sketch controls.

mod support;

#[test]
fn complete_durable_memory_r4_26() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/DurableMemoryControls.lex.tex",
        "DurableMemoryControls",
        "407cfd4af2f40bcfdb480e715e2a1a09e8a51e890bf52476d337cb674f6e69c7",
        &["DurableMemoryControls", "R4.DurableMemory"],
    )
}
