//! Formal Object store and range read contracts (R4-13).
//!
//! Independent development verification accepts the 331 canonical Object controls
//! (311 Codec and 20 Store controls) per `docs/OBJECT.md` and continuation records.
//! Read33 and Construction39 remain open continuation obligations.

mod support;

#[test]
fn canonical_content_objects_r4_13() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    std::thread::scope(|s| {
        let tx1 = tx.clone();
        s.spawn(move || {
            let res = support::verify_controls(
                "tests/lexlean/ObjectCodecControls.lex.tex",
                "ObjectCodecControls",
                "17ca6e3d679909c9c00e19f887c0eee7e3ec18058aa5c050a1ee8e54e176f865",
                &[
                    "ObjectCodecControls",
                    "R4.Hash",
                    "R4.HashNat",
                    "R4.HashNatCore",
                    "R4.HashWire",
                    "R4.ObjectCodec",
                    "R4.ObjectTypes",
                    "R4.Words",
                ],
            );
            let _ = tx1.send(res.map_err(|e| e.to_string()));
        });

        let tx2 = tx.clone();
        s.spawn(move || {
            let res = support::verify_controls(
                "tests/lexlean/ObjectStoreControls.lex.tex",
                "ObjectStoreControls",
                "28324d3259fe927296707a99e4bcc1a64305703c7f160d289c0d214a26623687",
                &[
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
            );
            let _ = tx2.send(res.map_err(|e| e.to_string()));
        });
    });
    drop(tx);
    while let Ok(res) = rx.recv() {
        res.map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    }
    Ok(())
}
