//! Conformance test for R4-40: arbitrary-repo GitHub git loop & ecosystem CI workflows.

#[path = "../../wasm/src/lexlean_generated.rs"]
mod lexlean_generated;

use lexlean_generated::r4_github_agent_loop::{
    admit_github_action, GithubActionCheck, GithubActionKind, GithubLoopVerdict,
};
use serde_json::Value;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn valid_action_check(action: GithubActionKind) -> GithubActionCheck {
    GithubActionCheck {
        action,
        policy_read_write: true,
        transport_https: true,
        secret_in_guest: false,
        host_allowlisted: true,
        repo_slug_valid: true,
        branch_valid: true,
        ci_and_deploy_green: true,
    }
}

#[test]
fn test_arbitrary_repo_github_git_loop_r4_40() {
    let root = repo_root();

    // Verify LexLean module exists and matches lexlean/semantic-module/2.
    let module_path = root.join("src/R4/GithubAgentLoop.lex.tex");
    let raw = std::fs::read_to_string(&module_path)
        .unwrap_or_else(|e| panic!("missing {}: {e}", module_path.display()));
    let (_, after) = raw
        .split_once("\\semanticdata{")
        .expect("semanticdata tag in GithubAgentLoop.lex.tex");
    let (json_block, _) = after
        .split_once("\n\\end{semanticmodule}")
        .expect("end semanticmodule in GithubAgentLoop.lex.tex");
    let json_line = json_block
        .trim()
        .strip_suffix('}')
        .unwrap_or(json_block.trim());
    let parsed: Value = serde_json::from_str(json_line).expect("valid JSON semantic module");
    assert!(raw.contains("\\begin{lexlean}{R4.GithubAgentLoop}"));
    assert_eq!(parsed["spec"], "lexlean/semantic-module/2");
    assert!(raw.contains("admitGithubAction"));

    // All six lifecycle actions admit cleanly when policy, transport, host, slug, branch, and CI hold.
    for action in [
        GithubActionKind::ConnectRepo,
        GithubActionKind::CreateBranch,
        GithubActionKind::CommitPush,
        GithubActionKind::OpenPullRequest,
        GithubActionKind::WatchCiAndDeploy,
        GithubActionKind::RefreshRepoState,
    ] {
        assert_eq!(
            admit_github_action(valid_action_check(action)),
            GithubLoopVerdict::ActionAdmitted,
            "expected valid {action:?} to admit"
        );
    }

    // Typed negative admission tests (R5).
    assert_eq!(
        admit_github_action(GithubActionCheck {
            policy_read_write: false,
            ..valid_action_check(GithubActionKind::CommitPush)
        }),
        GithubLoopVerdict::RefusedPolicyDenied
    );
    assert_eq!(
        admit_github_action(GithubActionCheck {
            transport_https: false,
            ..valid_action_check(GithubActionKind::ConnectRepo)
        }),
        GithubLoopVerdict::RefusedInsecureTransport
    );
    assert_eq!(
        admit_github_action(GithubActionCheck {
            secret_in_guest: true,
            ..valid_action_check(GithubActionKind::OpenPullRequest)
        }),
        GithubLoopVerdict::RefusedGuestSecretLeak
    );
    assert_eq!(
        admit_github_action(GithubActionCheck {
            host_allowlisted: false,
            ..valid_action_check(GithubActionKind::ConnectRepo)
        }),
        GithubLoopVerdict::RefusedHostNotAllowlisted
    );
    assert_eq!(
        admit_github_action(GithubActionCheck {
            repo_slug_valid: false,
            ..valid_action_check(GithubActionKind::CreateBranch)
        }),
        GithubLoopVerdict::RefusedInvalidSlugOrRef
    );
    assert_eq!(
        admit_github_action(GithubActionCheck {
            branch_valid: false,
            ..valid_action_check(GithubActionKind::CreateBranch)
        }),
        GithubLoopVerdict::RefusedInvalidSlugOrRef
    );
    assert_eq!(
        admit_github_action(GithubActionCheck {
            ci_and_deploy_green: false,
            ..valid_action_check(GithubActionKind::WatchCiAndDeploy)
        }),
        GithubLoopVerdict::RefusedCiOrDeployFailed
    );

    // Ecosystem workflows exist and honor ubuntu-24.04 + site/app.json sync.
    for wf in ["automerge.yml", "deploy-watch.yml", "pages.yml"] {
        let wf_path = root.join(".github/workflows").join(wf);
        let body = std::fs::read_to_string(&wf_path)
            .unwrap_or_else(|e| panic!("missing workflow {}: {e}", wf_path.display()));
        assert!(
            body.contains("ubuntu-24.04"),
            "{wf} must target ubuntu-24.04"
        );
    }
    let pages =
        std::fs::read_to_string(root.join(".github/workflows/pages.yml")).expect("read pages.yml");
    assert!(
        pages.contains("if [ -f site/app.json ]; then cp site/app.json public/app.json; fi"),
        "pages.yml must copy optional site/app.json after closure check"
    );
    assert!(
        pages.contains("cp -R gpu/weights public/weights"),
        "pages.yml must keep copying gpu/weights"
    );
}
