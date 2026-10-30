//! Complete imported catalog, declared coverage and structural report descriptions.
//! Caller-supplied reports never authenticate themselves or authorize execution.

mod support;

#[test]
fn exact_evidence_r4_17() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_model::repo_root();
    assert_eq!(
        support::sha256_hex(&std::fs::read(
            root.join("tests/fixtures/evidence/upstream-board.json"),
        )?),
        "357cdfc926a220239193b19c6d2fd86beea4e6c0d843926066e6c185e1fdc5d2",
        "reviewed complete upstream mechanism evidence changed"
    );
    let cases: [(&str, &str, &str, &[&str]); 7] = [
        (
            "tests/lexlean/EvidenceCatalogControls.lex.tex",
            "EvidenceCatalogControls",
            "5ac609e85576334d6502c35f807ad1fd11e04f253925ada0f89ea5ccf46f870f",
            &["EvidenceCatalogControls", "R4.Evidence"],
        ),
        (
            "tests/lexlean/EvidenceCoverageControls.lex.tex",
            "EvidenceCoverageControls",
            "1e2c3bb701b27fb15b729ac9b688d5d32d1f728178f46325d4cd6a9bc00bcc12",
            &[
                "EvidenceCoverageControls",
                "R4.Evidence",
                "R4.EvidenceCoverage",
            ],
        ),
        (
            "tests/lexlean/EvidenceCoverageBoundaryControls.lex.tex",
            "EvidenceCoverageBoundaryControls",
            "60547b76953030c72e7cc85bab53c381c688a448efaa812365afac60af80c38a",
            &[
                "EvidenceCoverageBoundaryControls",
                "R4.Evidence",
                "R4.EvidenceCoverage",
            ],
        ),
        (
            "tests/lexlean/EvidenceDescriptionIdentityControls.lex.tex",
            "EvidenceDescriptionIdentityControls",
            "d3c652ba9e127441ea2f9887c3a766c8efc58f944dd336a575ec08742b08f3f8",
            &[
                "EvidenceDescriptionIdentityControls",
                "R4.Evidence",
                "R4.EvidenceDescription",
                "R4.TokenizerText",
                "R4.TokenizerTextUnicode",
            ],
        ),
        (
            "tests/lexlean/EvidenceDescriptionRunControls.lex.tex",
            "EvidenceDescriptionRunControls",
            "3b3de7cf1a8784805a527f686bb3fc8030ac51d84d23fc61e48464bcdf65e497",
            &[
                "EvidenceDescriptionRunControls",
                "R4.Evidence",
                "R4.EvidenceDescription",
                "R4.TokenizerText",
                "R4.TokenizerTextUnicode",
            ],
        ),
        (
            "tests/lexlean/EvidenceDescriptionGraphControls.lex.tex",
            "EvidenceDescriptionGraphControls",
            "1042a6d9ca1ca0f03faa42602fa88b35f8879c093f52b8890817c69d3023f9f8",
            &[
                "EvidenceDescriptionGraphControls",
                "R4.Evidence",
                "R4.EvidenceDescription",
                "R4.TokenizerText",
                "R4.TokenizerTextUnicode",
            ],
        ),
        (
            "tests/lexlean/EvidenceDescriptionBoundaryControls.lex.tex",
            "EvidenceDescriptionBoundaryControls",
            "a614abcb5b7dd68eace29a7ae1c0e70f525b9107a76b661e872387fd30096f02",
            &[
                "EvidenceDescriptionBoundaryControls",
                "R4.Evidence",
                "R4.EvidenceDescription",
                "R4.TokenizerText",
                "R4.TokenizerTextUnicode",
            ],
        ),
    ];

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let next_idx = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        let workers = 4;
        for _ in 0..workers {
            let tx = tx.clone();
            let next_idx = &next_idx;
            s.spawn(move || {
                loop {
                    let i = next_idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= cases.len() {
                        break;
                    }
                    let (path, module, digest, modules) = cases[i];
                    if let Err(e) = support::verify_controls(path, module, digest, modules) {
                        let _ = tx.send(Err(e.to_string()));
                        return;
                    }
                }
                let _ = tx.send(Ok(()));
            });
        }
    });
    drop(tx);
    while let Ok(res) = rx.recv() {
        res.map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    }
    Ok(())
}
