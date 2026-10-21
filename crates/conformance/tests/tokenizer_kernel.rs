//! Complete typed byte-BPE semantics; raw JSON and identity are separate gates.

mod support;

#[test]
fn exact_tokenizer_kernel_r4_16() -> Result<(), Box<dyn std::error::Error>> {
    for (path, module, digest) in [
        (
            "tests/lexlean/TokenizerKernelControls.lex.tex",
            "TokenizerKernelControls",
            "dfa8e53738a4d2c12a57b6800185e0136e009a5de53ba4342f8a0ce13af052c9",
        ),
        (
            "tests/lexlean/TokenizerKernelBoundaryControls.lex.tex",
            "TokenizerKernelBoundaryControls",
            "b70eb35d74ca6f6eeacc590d21f8803d1b8edae07fad30849be1e81e7d511da4",
        ),
        (
            "tests/lexlean/TokenizerKernelSurfaceControls.lex.tex",
            "TokenizerKernelSurfaceControls",
            "c46d8dac6721578f6281de87b13431a600eb2be9fc322dc35ad00502df130bd3",
        ),
        (
            "tests/lexlean/TokenizerKernelProgressControls.lex.tex",
            "TokenizerKernelProgressControls",
            "8286c01b3c7092b8940f2163a2a07e29dd3ecafa0326a4912c634cb56ccb89a9",
        ),
        (
            "tests/lexlean/TokenizerKernelLengthControls.lex.tex",
            "TokenizerKernelLengthControls",
            "dc74c7be04391e9264356ec3e053c9440580ffceab4ec92f097d158d6254fde5",
        ),
        (
            "tests/lexlean/TokenizerKernelUnicodeControls.lex.tex",
            "TokenizerKernelUnicodeControls",
            "4e84bd416d6daa435fe46c959000f217c92efa7e24b3edf1f84c6ad06388a91a",
        ),
    ] {
        support::verify_controls(
            path,
            module,
            digest,
            &[
                "R4.TokenizerKernel",
                "R4.TokenizerText",
                "R4.TokenizerTextUnicode",
                module,
            ],
        )?;
    }
    Ok(())
}
