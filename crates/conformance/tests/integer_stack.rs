//! Integer serving stack (`/6` Hamming read, `/7` turn memory, `/8` point-key plurality, nearest readout) controls.

mod support;

#[test]
fn complete_integer_stack_serving_r4_23() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/IntegerStackControls.lex.tex",
        "IntegerStackControls",
        "f52d44556c3ee649640813cb4520f637f8e4f5480015f9ec6ec84a1806b48856",
        &["IntegerStackControls", "R4.IntegerStack"],
    )
}
