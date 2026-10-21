//! Complete imported catalog, declared coverage and structural report descriptions.
//! Caller-supplied reports never authenticate themselves or authorize execution.

mod support;

use sha2::{Digest, Sha256};

#[test]
fn exact_evidence_r4_17() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_model::repo_root();
    assert_eq!(
        format!(
            "{:x}",
            Sha256::digest(std::fs::read(
                root.join("tests/fixtures/evidence/upstream-board.json")
            )?)
        ),
        "357cdfc926a220239193b19c6d2fd86beea4e6c0d843926066e6c185e1fdc5d2",
        "reviewed complete upstream mechanism evidence changed"
    );
    support::verify_controls(
        "tests/lexlean/EvidenceCatalogControls.lex.tex",
        "EvidenceCatalogControls",
        "171feb942c88b96189667afad97eefb4613b13d252124f55d52c6845cfb36324",
        &["EvidenceCatalogControls", "R4.Evidence"],
    )?;
    support::verify_controls(
        "tests/lexlean/EvidenceCoverageControls.lex.tex",
        "EvidenceCoverageControls",
        "475fb9138594c96b5d485f19025d7219359e8ad0e59169f0fab4cae522acb28e",
        &[
            "EvidenceCoverageControls",
            "R4.Evidence",
            "R4.EvidenceCoverage",
        ],
    )?;
    support::verify_controls(
        "tests/lexlean/EvidenceCoverageBoundaryControls.lex.tex",
        "EvidenceCoverageBoundaryControls",
        "f602de7fc17acfe413ac55251ada3251cbdbed1f2e046ec758e946b6a7040ecb",
        &[
            "EvidenceCoverageBoundaryControls",
            "R4.Evidence",
            "R4.EvidenceCoverage",
        ],
    )?;
    support::verify_controls(
        "tests/lexlean/EvidenceDescriptionIdentityControls.lex.tex",
        "EvidenceDescriptionIdentityControls",
        "2e8f4f92d9148a063fbff3f84247e63eb37164a2e0ccecfdaec000fc73fd0b44",
        &[
            "EvidenceDescriptionIdentityControls",
            "R4.Evidence",
            "R4.EvidenceDescription",
            "R4.TokenizerText",
            "R4.TokenizerTextUnicode",
        ],
    )?;
    support::verify_controls(
        "tests/lexlean/EvidenceDescriptionRunControls.lex.tex",
        "EvidenceDescriptionRunControls",
        "355418af17e04fe34087182db878378f88c1414b2083b8efc971ca9ee7b791fb",
        &[
            "EvidenceDescriptionRunControls",
            "R4.Evidence",
            "R4.EvidenceDescription",
            "R4.TokenizerText",
            "R4.TokenizerTextUnicode",
        ],
    )?;
    support::verify_controls(
        "tests/lexlean/EvidenceDescriptionGraphControls.lex.tex",
        "EvidenceDescriptionGraphControls",
        "6fd332a1910efc32632da64ff6d671278d50bea19c71e3eb8f36045d7727812d",
        &[
            "EvidenceDescriptionGraphControls",
            "R4.Evidence",
            "R4.EvidenceDescription",
            "R4.TokenizerText",
            "R4.TokenizerTextUnicode",
        ],
    )?;
    support::verify_controls(
        "tests/lexlean/EvidenceDescriptionBoundaryControls.lex.tex",
        "EvidenceDescriptionBoundaryControls",
        "c39d353c5a1812ab1805415fe0dbb6c80595fe3dd113b395784469411d3cb3c7",
        &[
            "EvidenceDescriptionBoundaryControls",
            "R4.Evidence",
            "R4.EvidenceDescription",
            "R4.TokenizerText",
            "R4.TokenizerTextUnicode",
        ],
    )?;
    Ok(())
}
