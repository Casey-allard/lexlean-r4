//! Ordered binder metadata contracts; no carrier arithmetic is executed.

use crate::support;

#[test]
fn canonical_binder_r4_04() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/BinderControls.lex.tex",
        "BinderControls",
        "2787a4a06335c0a3222b0585d478afc807c7abd327122dde7ffe993860d207af",
        &["BinderControls", "R4.Binder"],
    )
}
