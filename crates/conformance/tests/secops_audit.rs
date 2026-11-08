//! FIPS 180-4 SHA-256 Merkle SecOps audit chain (`uor.secops.audit/1`), MCP policy gate, and container workspace sandbox controls.

mod support;

#[test]
fn complete_secops_audit_r4_30() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/SecopsAuditControls.lex.tex",
        "SecopsAuditControls",
        "349b28ceb38379a217846bcb6fdbe73dadc5e62ff83e73397867aa1b2df486c2",
        &["SecopsAuditControls", "R4.SecopsAudit"],
    )
}
