//! Formal Object store, range read, and construction contracts (R4-13).
//!
//! Verifies all 403 canonical Object controls (311 Codec, 20 Store, 33 Read,
//! and 39 Construction controls) and independent `leanchecker` kernel replay
//! across the complete Object module closure.

mod support;

#[test]
fn canonical_content_objects_r4_13() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_control_batch(
        "tests/lexlean/ObjectBatch.lex.tex",
        "ObjectBatch",
        "cf239fc980a6034cf5a016440edd365674086329a456707abf05939ecd416fd5",
        &[
            (
                "tests/lexlean/ObjectCodecControls.lex.tex",
                "ObjectCodecControls",
                "17ca6e3d679909c9c00e19f887c0eee7e3ec18058aa5c050a1ee8e54e176f865",
            ),
            (
                "tests/lexlean/ObjectStoreControls.lex.tex",
                "ObjectStoreControls",
                "28324d3259fe927296707a99e4bcc1a64305703c7f160d289c0d214a26623687",
            ),
            (
                "tests/lexlean/ObjectReadControls.lex.tex",
                "ObjectReadControls",
                "f99c16bb472a2e543e46dcdfb99fac0582e25818b365d392fb0f7cb5588ce56e",
            ),
            (
                "tests/lexlean/ObjectControls.lex.tex",
                "ObjectControls",
                "4246311734006874b8459e781ecefca7e4ed4c336f4c40f6fa63356769b72b87",
            ),
        ],
        &[
            "ObjectBatch",
            "ObjectCodecControls",
            "ObjectControls",
            "ObjectReadControls",
            "ObjectStoreControls",
            "R4.Hash",
            "R4.HashNat",
            "R4.HashNatCore",
            "R4.HashWire",
            "R4.Object",
            "R4.ObjectCodec",
            "R4.ObjectRead",
            "R4.ObjectStore",
            "R4.ObjectTypes",
            "R4.Words",
        ],
    )
}
