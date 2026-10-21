//! Formal writer contracts; no host model execution or GPU qualification.

mod support;

#[test]
fn admitted_identity_writer_r4_05() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/AdmissionControls.lex.tex",
        "AdmissionControls",
        "cd5cfdf48ce3f31f26348f3de57ed3afd379fc8e832571463b1426d5a791dc16",
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
