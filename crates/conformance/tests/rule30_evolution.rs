//! Rule 30 chaotic frontier discovery and monotonic weight evolution controls (`R4-33`).

mod support;

#[test]
fn complete_rule30_evolution_r4_33() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/Rule30EvolutionControls.lex.tex",
        "Rule30EvolutionControls",
        "dad44c68d46548c3ff470bc2dca0e6924ce0c5e7fa158897064c52bcc655a847",
        &["Rule30EvolutionControls", "R4.Rule30Evolution"],
    )
}
