//! Compositional Geometric Language Model (`UORGLLM9`) container, causal PRLL KV cache, and T-MAC LUT controls (`R4-38`).

mod support;

#[test]
fn enforce_compositional_gllm_v9_r4_38() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/CompositionalGllmControls.lex.tex",
        "CompositionalGllmControls",
        "ec42f6ebe6f8c0b576d45cb99f43e10e3fd6c0daab78141616329b4d79ef4658",
        &["CompositionalGllmControls", "R4.CompositionalGllm"],
    )
}
