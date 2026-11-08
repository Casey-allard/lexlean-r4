//! Two-level hierarchical Radix `/8` index (`17,408` geometric buckets) and 5-tier OTel `u16` span deduplication controls.

mod support;

#[test]
fn complete_radix_otel_r4_29() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/RadixOtelControls.lex.tex",
        "RadixOtelControls",
        "7dc86cca75e591f2fdfb3b8888391ee11ee04e98d0e004da3bd8bfd377010155",
        &["RadixOtelControls", "R4.RadixOtel"],
    )
}
