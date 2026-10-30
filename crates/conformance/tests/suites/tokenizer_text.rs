//! Exact tokenizer text primitives, not a complete tokenizer or identity claim.

use crate::support;

#[test]
fn exact_tokenizer_text_r4_14() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_model::repo_root();
    for (path, expected) in [
        (
            "tests/fixtures/tokenizer/unicode-inventory.json",
            "e5d09aaa3d8edd50b0fedd1ea887f779fc319787a9c5f9e74b108f4c6a251b7d",
        ),
        (
            "tests/fixtures/tokenizer/unicode-boundaries.json",
            "e2d378cac11a10f0028a49c270181ee738d5b0506b76c8291c1f5780fcfb6655",
        ),
    ] {
        assert_eq!(
            support::sha256_hex(&std::fs::read(root.join(path))?),
            expected,
            "reviewed imported Unicode evidence changed: {path}"
        );
    }
    support::verify_controls(
        "tests/lexlean/TokenizerTextControls.lex.tex",
        "TokenizerTextControls",
        "d9eeab51a7f4df6d39f4594e80dadda3f5126e2b1324ab9a265faff0d5c733ba",
        &[
            "R4.TokenizerText",
            "R4.TokenizerTextUnicode",
            "TokenizerTextControls",
        ],
    )
}
