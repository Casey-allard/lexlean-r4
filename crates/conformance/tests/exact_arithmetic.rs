//! M6 multi-digit radix-B exact geometric arithmetic, causal ablation arms, PRLL lawful likelihood, and `uor-naf/1-draft.6` admission controls.

mod support;

#[test]
fn complete_exact_arithmetic_r4_27() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/ExactArithmeticControls.lex.tex",
        "ExactArithmeticControls",
        "b995ae88a1e917685ba7ecf55ffb30770f26798e0248a77e619324cebfab746f",
        &["ExactArithmeticControls", "R4.ExactArithmetic"],
    )
}
