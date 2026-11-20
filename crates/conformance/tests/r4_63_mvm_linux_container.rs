//! Conformance test for R4-63: Single-Source MVM Linux `/workspace`,
//! SpiralCore CORS Preemption, Interactive Execution Precedence & <=120-Token State Header.

#![allow(missing_docs)]

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_mvm_linux_container::{
    admit_interactive_command, admit_live_state_header, admit_mvm_socket,
    admit_public_cors_preemption, admit_workspace_authority, max_live_state_header_tokens,
    verify_mvm_linux_container_admitted, AgentPlanMode, CorsPreemptionVerdict,
    InteractiveCommandVerdict, McpGateMode, MvmNetworkMode, MvmSocketVerdict, ShellCommandKind,
    WorkspaceAuthorityMode, WorkspaceAuthorityVerdict,
};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn check_lex_module(root: &Path, rel: &str, expected_mod: &str, required_decls: &[&str]) {
    let raw =
        std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("missing {rel}: {e}"));
    assert!(raw.contains(&format!("\\begin{{lexlean}}{{{expected_mod}}}")));
    let (_, after) = raw
        .split_once("\\semanticdata{")
        .unwrap_or_else(|| panic!("missing \\semanticdata{{ in {rel}"));
    let (json_block, _) = after
        .split_once("\n\\end{semanticmodule}")
        .unwrap_or_else(|| panic!("missing \\end{{semanticmodule}} in {rel}"));
    let json_line = json_block
        .trim()
        .strip_suffix('}')
        .unwrap_or(json_block.trim());
    let parsed: Value = serde_json::from_str(json_line).expect("valid JSON semantic module");
    assert_eq!(parsed["spec"], "lexlean/semantic-module/2");
    for sym in required_decls {
        assert!(raw.contains(sym), "{rel} must declare {sym}");
    }
}

#[test]
fn test_r4_63_mvm_linux_container_r4_63() {
    let root = repo_root();

    // 1. Single-source MVM workspace authority & negative R5 verdicts.
    assert_eq!(
        admit_workspace_authority(WorkspaceAuthorityMode::SingleSourceMvmCow, true),
        WorkspaceAuthorityVerdict::SingleSourceAdmitted
    );
    assert_eq!(
        admit_workspace_authority(WorkspaceAuthorityMode::SplitBrainDuplicateStore, true),
        WorkspaceAuthorityVerdict::RefusedSplitBrainWorkspace
    );
    assert_eq!(
        admit_workspace_authority(WorkspaceAuthorityMode::SingleSourceMvmCow, false),
        WorkspaceAuthorityVerdict::RefusedInconsistentCowState
    );

    // 2. Socket domain admission preserving ZeroNicIsolated (-97) alongside SpiralCoreEncapsulated.
    assert_eq!(
        admit_mvm_socket(MvmNetworkMode::ZeroNicIsolated, false, true),
        MvmSocketVerdict::AdmittedDirectVsock
    );
    assert_eq!(
        admit_mvm_socket(MvmNetworkMode::ZeroNicIsolated, true, false),
        MvmSocketVerdict::RefusedZeroNicIsolated
    );
    assert_eq!(
        admit_mvm_socket(MvmNetworkMode::SpiralCoreEncapsulated, true, false),
        MvmSocketVerdict::AdmittedSpiralCoreInetEncapsulation
    );
    assert_eq!(
        admit_mvm_socket(MvmNetworkMode::SpiralCoreEncapsulated, false, false),
        MvmSocketVerdict::RefusedUnsupportedDomain
    );

    // 3. Unprompted public CORS preemption & strict SSRF / credentialed refusal.
    assert_eq!(
        admit_public_cors_preemption(true, false, false, false),
        CorsPreemptionVerdict::DirectBrowserFetchAdmitted
    );
    assert_eq!(
        admit_public_cors_preemption(true, false, false, true),
        CorsPreemptionVerdict::StatelessRelayCorsPreempted
    );
    assert_eq!(
        admit_public_cors_preemption(true, false, true, true),
        CorsPreemptionVerdict::RefusedCredentialedCorsProxy
    );
    assert_eq!(
        admit_public_cors_preemption(true, true, false, true),
        CorsPreemptionVerdict::RefusedPrivateAddressSsrf
    );
    assert_eq!(
        admit_public_cors_preemption(false, false, false, true),
        CorsPreemptionVerdict::RefusedSpiralCoreNetDisabled
    );

    // 4. Interactive Agent/Plan/Auto-run/Deny precedence & <=120-token KV state header bound.
    assert_eq!(max_live_state_header_tokens(), 120);
    assert!(admit_live_state_header(120));
    assert!(!admit_live_state_header(121));
    assert_eq!(
        admit_interactive_command(
            McpGateMode::Deny,
            AgentPlanMode::AgentFull,
            true,
            ShellCommandKind::ReadOnlyInspection
        ),
        InteractiveCommandVerdict::RefusedByDenyPolicy
    );
    assert_eq!(
        admit_interactive_command(
            McpGateMode::Safe,
            AgentPlanMode::PlanReadOnly,
            true,
            ShellCommandKind::MutatingOrNetwork
        ),
        InteractiveCommandVerdict::RefusedInPlanMode
    );
    assert_eq!(
        admit_interactive_command(
            McpGateMode::Safe,
            AgentPlanMode::PlanReadOnly,
            false,
            ShellCommandKind::ReadOnlyInspection
        ),
        InteractiveCommandVerdict::AutoRunAdmitted
    );
    assert_eq!(
        admit_interactive_command(
            McpGateMode::Safe,
            AgentPlanMode::AgentFull,
            false,
            ShellCommandKind::MutatingOrNetwork
        ),
        InteractiveCommandVerdict::StagedPendingUserGo
    );
    assert_eq!(
        admit_interactive_command(
            McpGateMode::Confirm,
            AgentPlanMode::AgentFull,
            true,
            ShellCommandKind::MutatingOrNetwork
        ),
        InteractiveCommandVerdict::StagedPendingUserGo
    );
    assert_eq!(
        admit_interactive_command(
            McpGateMode::Safe,
            AgentPlanMode::AgentFull,
            true,
            ShellCommandKind::MutatingOrNetwork
        ),
        InteractiveCommandVerdict::AutoRunAdmitted
    );

    assert!(verify_mvm_linux_container_admitted());

    // 5. Formal LexLean module, controls, model/ids.toml, and Gherkin suite checks.
    check_lex_module(
        &root,
        "src/R4/MvmLinuxContainer.lex.tex",
        "R4.MvmLinuxContainer",
        &[
            "WorkspaceAuthorityMode",
            "WorkspaceAuthorityVerdict",
            "MvmNetworkMode",
            "MvmSocketVerdict",
            "CorsPreemptionVerdict",
            "McpGateMode",
            "AgentPlanMode",
            "ShellCommandKind",
            "InteractiveCommandVerdict",
            "verifyMvmLinuxContainerAdmitted",
            "breaksSplitBrainWorkspaceAdmitted",
            "breaksZeroNicBypassWhenIsolatedAdmitted",
            "breaksCredentialedCorsProxyAdmitted",
            "breaksPrivateSsrfOnSpiralCoreAdmitted",
            "breaksPlanMutationEvenWithAutoRunAdmitted",
            "breaksDenyPolicyBypassAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/MvmLinuxContainerControls.lex.tex",
        "MvmLinuxContainerControls",
        &[
            "mvmLinuxContainerAdmittedCtrl",
            "breaksSplitBrainWorkspaceAdmittedCtrl",
            "breaksZeroNicBypassWhenIsolatedAdmittedCtrl",
            "breaksCredentialedCorsProxyAdmittedCtrl",
            "breaksPrivateSsrfOnSpiralCoreAdmittedCtrl",
            "breaksPlanMutationEvenWithAutoRunAdmittedCtrl",
            "breaksDenyPolicyBypassAdmittedCtrl",
        ],
    );

    let ids_toml =
        std::fs::read_to_string(root.join("model/ids.toml")).expect("model/ids.toml must exist");
    assert!(
        ids_toml.contains("id = \"R4-63\""),
        "R4-63 must be registered in model/ids.toml"
    );
    let feature_file = root.join("features/suites/r4_63_mvm_linux_container.feature");
    assert!(feature_file.is_file(), "R4-63 feature suite must exist");
    let feature_text = std::fs::read_to_string(feature_file).expect("read R4-63 feature file");
    assert!(feature_text.contains("@R4-63 @build"));
}
