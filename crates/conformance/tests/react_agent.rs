//! Fuel-bounded ReAct self-correcting agent controller and verified patch gate controls (`R4-35`).

mod support;

#[test]
fn complete_react_agent_r4_35() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/ReactAgentControls.lex.tex",
        "ReactAgentControls",
        "43baddcc1193367a9f9c2d84160d9d4b69d91c2caf7f6ee5d76ab0ac6ec4aee8",
        &["ReactAgentControls", "R4.ReactAgent"],
    )
}
