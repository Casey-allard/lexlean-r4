//! Fixed Boolean word arithmetic and codecs; no hash or model execution claim.

mod support;

#[test]
fn fixed_word_algebra_r4_09() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/WordsControls.lex.tex",
        "WordsControls",
        "1aed483df67028b30fca7fb28ecae0610807652ac4d224ef0af85986f1df7434",
        &["R4.Words", "WordsControls"],
    )
}
