//! Formal writer contracts; no host model execution or GPU qualification.

mod support;

#[test]
fn admitted_identity_writer_r4_05() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/AdmissionControls.lex.tex",
        "AdmissionControls",
        "e7fa4b65a1bcdaafaa6ec5b42a477d1230c2a4bbea9422eb41baaf63ef6b995c",
        &[
            "AdmissionControls",
            "R4.Admission",
            "R4.Carrier",
            "R4.Geometry",
            "R4.Objective",
            "R4.Types",
        ],
    )
}
