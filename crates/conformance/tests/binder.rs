//! Ordered binder metadata contracts; no carrier arithmetic is executed.

mod support;

#[test]
fn canonical_binder_r4_04() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/BinderControls.lex.tex",
        "BinderControls",
        "07d929babf6072b9f8016174179c498e9a0b1d63cc6f29f7de1c645e8743ef6d",
        &["BinderControls", "R4.Binder"],
    )
}
