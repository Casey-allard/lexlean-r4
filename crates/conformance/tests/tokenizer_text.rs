//! Exact tokenizer text primitives, not a complete tokenizer or identity claim.

mod support;

use sha2::{Digest, Sha256};

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
            format!("{:x}", Sha256::digest(std::fs::read(root.join(path))?)),
            expected,
            "reviewed imported Unicode evidence changed: {path}"
        );
    }
    support::verify_controls(
        "tests/lexlean/TokenizerTextControls.lex.tex",
        "TokenizerTextControls",
        "ec3b221ab55f1cb5b36460d0eac2208d4e26de76b1b390ce8dbb44a4064a4044",
        &[
            "R4.TokenizerText",
            "R4.TokenizerTextUnicode",
            "TokenizerTextControls",
        ],
    )
}
