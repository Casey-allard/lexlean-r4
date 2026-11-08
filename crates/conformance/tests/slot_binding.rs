//! De Bruijn variable slot-binding, live kernel execution (`R4-27` / `R4-24` / AST), and `/7` token-stream substitution controls.

mod support;

#[test]
fn complete_slot_binding_r4_31() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/SlotBindingControls.lex.tex",
        "SlotBindingControls",
        "6190563219caaa384e2f90433e205fc7b1f447cc28f3f30a8277bd13deb9c939",
        &["SlotBindingControls", "R4.SlotBinding"],
    )
}
