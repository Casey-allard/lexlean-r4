//! Fixed Boolean word arithmetic and codecs; no hash or model execution claim.

mod support;

#[test]
fn fixed_word_algebra_r4_09() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/WordsControls.lex.tex",
        "WordsControls",
        "27d615ee3f8887ecfcaf88ae14a6fbd768a20cd87460a0546ded000147fa0ba6",
        &["R4.Words", "WordsControls"],
    )
}
