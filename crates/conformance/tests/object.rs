//! Formal Object store and range read contracts (R4-13).

mod support;

#[test]
fn canonical_content_objects_r4_13() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/ObjectCodecControls.lex.tex",
        "ObjectCodecControls",
        "17ca6e3d679909c9c00e19f887c0eee7e3ec18058aa5c050a1ee8e54e176f865",
        &[
            "ObjectCodecControls",
            "R4.Hash",
            "R4.ObjectCodec",
            "R4.ObjectTypes",
        ],
    )?;
    support::verify_controls(
        "tests/lexlean/ObjectStoreControls.lex.tex",
        "ObjectStoreControls",
        "28324d3259fe927296707a99e4bcc1a64305703c7f160d289c0d214a26623687",
        &[
            "ObjectStoreControls",
            "R4.Object",
            "R4.ObjectCodec",
            "R4.ObjectTypes",
        ],
    )?;
    support::verify_controls(
        "tests/lexlean/ObjectReadControls.lex.tex",
        "ObjectReadControls",
        "98f2a63ebdbe7649543b08c61e153eab6f978af0e80b44c6810181dbe113365c",
        &[
            "ObjectReadControls",
            "ObjectWholeComposedResult",
            "R4.Object",
            "R4.ObjectCodec",
            "R4.ObjectRead",
            "R4.ObjectTypes",
        ],
    )?;
    support::verify_controls(
        "tests/lexlean/ObjectControls.lex.tex",
        "ObjectControls",
        "4246311734006874b8459e781ecefca7e4ed4c336f4c40f6fa63356769b72b87",
        &[
            "ObjectControls",
            "R4.Object",
            "R4.ObjectCodec",
            "R4.ObjectTypes",
        ],
    )
}
