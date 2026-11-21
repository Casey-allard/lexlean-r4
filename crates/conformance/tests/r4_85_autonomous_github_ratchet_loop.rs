//! Conformance test for R4-85: Continuous GitHub `r4-engine` Dual-Surface Ratchet Pipeline,
//! Unified `./bin/r4 ratchet` CLI, `.lexlean` CI Cache, and Bounded 30-Minute Cadence
//! (`R4.AutonomousGithubRatchetLoop`).

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_autonomous_github_ratchet_loop::{
    admit_autonomous_github_tick, verify_autonomous_github_ratchet_loop_admitted,
    GithubRatchetLoopVerdict,
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
fn test_r4_85_autonomous_github_ratchet_loop_r4_85() {
    let root = repo_root();

    // 1. Formal LexLean R4-85 admission and all 5 R5 negative refusals
    assert!(verify_autonomous_github_ratchet_loop_admitted());
    assert_eq!(
        admit_autonomous_github_tick(true, false, true, true, true, true, false),
        GithubRatchetLoopVerdict::AutonomousTickAdmitted
    );
    assert_eq!(
        admit_autonomous_github_tick(false, false, true, true, true, true, false),
        GithubRatchetLoopVerdict::RefusedStaleMainTip
    );
    assert_eq!(
        admit_autonomous_github_tick(true, true, true, true, true, true, false),
        GithubRatchetLoopVerdict::RefusedSelfPushRecursion
    );
    assert_eq!(
        admit_autonomous_github_tick(true, false, false, true, true, true, false),
        GithubRatchetLoopVerdict::RefusedMissingLexleanCiCache
    );
    assert_eq!(
        admit_autonomous_github_tick(true, false, true, true, false, true, false),
        GithubRatchetLoopVerdict::RefusedFailedDualSurfaceOrWatermarks
    );
    assert_eq!(
        admit_autonomous_github_tick(true, false, true, true, true, true, true),
        GithubRatchetLoopVerdict::RefusedUnthrottledSelfRetrigger
    );

    // 2. Verify .lex.tex semantic modules exist
    check_lex_module(
        &root,
        "src/R4/AutonomousGithubRatchetLoop.lex.tex",
        "R4.AutonomousGithubRatchetLoop",
        &[
            "GithubRatchetLoopVerdict",
            "admitAutonomousGithubTick",
            "verifyAutonomousGithubRatchetLoopAdmitted",
            "breaksStaleMainTipAdmitted",
            "breaksSelfPushRecursionAdmitted",
            "breaksMissingLexleanCiCacheAdmitted",
            "breaksUntightenedWatermarksAdmitted",
            "breaksUnthrottledSelfRetriggerAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/AutonomousGithubRatchetLoopControls.lex.tex",
        "AutonomousGithubRatchetLoopControls",
        &[
            "autonomousGithubRatchetLoopAdmitted",
            "breaksStaleMainTipAdmittedCtrl",
            "breaksSelfPushRecursionAdmittedCtrl",
            "breaksMissingLexleanCiCacheAdmittedCtrl",
            "breaksUntightenedWatermarksAdmittedCtrl",
            "breaksUnthrottledSelfRetriggerAdmittedCtrl",
        ],
    );

    // 3. Verify .github/workflows/engine.yml includes .lexlean cache, r4-engine recursion guard,
    //    pages.yml publish trigger, and NO runaway `gh workflow run engine.yml` self-chaining.
    let engine_yml = std::fs::read_to_string(root.join(".github/workflows/engine.yml"))
        .expect("read .github/workflows/engine.yml");
    assert!(
        engine_yml.contains("path: .lexlean") && engine_yml.contains("r4-pages-lexlean-v1-"),
        "engine.yml must restore and save commit-pinned .lexlean verification cache"
    );
    assert!(
        engine_yml.contains("head_author=$(git log -1 --format='%an' HEAD)")
            && engine_yml.contains(
                "[ \"${GITHUB_EVENT_NAME:-}\" = \"push\" ] && [ \"$head_author\" = \"r4-engine\" ]"
            ),
        "engine.yml must guard push events authored by r4-engine against recursive self-triggers"
    );
    assert!(
        engine_yml.contains("gh workflow run pages.yml --ref main")
            && !engine_yml.contains("gh workflow run engine.yml")
            && !engine_yml.contains("git rebase --autostash")
            && engine_yml.contains("Save LexLean verification cache"),
        "engine.yml must save post-tick .lexlean cache, trigger pages.yml, forbid unverified rebase, and forbid runaway engine.yml self-chaining"
    );

    let pages_yml = std::fs::read_to_string(root.join(".github/workflows/pages.yml"))
        .expect("read .github/workflows/pages.yml");
    assert!(
        pages_yml.contains("r4-pages-lexlean-v1-\n"),
        "pages.yml must include r4-pages-lexlean-v1- prefix fallback so autonomous RatchetGen entrypoints hit warm .olean cache"
    );

    // 4. Verify xtask/src/engine.rs runs dual_surface_ratchet + ratchet-cycle + observer-agenda,
    //    rolls back on any mid-tick failure, and stages all evolved paths
    let engine_rs =
        std::fs::read_to_string(root.join("xtask/src/engine.rs")).expect("read engine.rs");
    assert!(
        engine_rs.contains("dual_surface_ratchet::step_dual_surface_ratchet_evolution(")
            && engine_rs.contains("ratchet::run_ratchet_cycle(root, true)")
            && engine_rs.contains("observer_agenda::cmd_observer_agenda(root, true)")
            && engine_rs.contains("let res = run_engine_tick_inner(")
            && engine_rs.contains("\"model/ratchet.toml\"")
            && engine_rs.contains("\"src/R4\""),
        "engine.rs tick must execute dual_surface_ratchet, ratchet_cycle, observer_agenda, atomic rollback wrapper, and stage model/ratchet.toml & src/R4"
    );

    // 5. Verify unified `ratchet` subcommand in xtask/src/main.rs honors `--dry-run` and `validate` checks `check-wasm-sync`
    let main_rs = std::fs::read_to_string(root.join("xtask/src/main.rs")).expect("read main.rs");
    assert!(
        main_rs.contains("\"ratchet\" =>")
            && main_rs.contains("if evolve || dry_run || commit || push")
            && main_rs.contains("wasm_bundle::run_check_wasm_sync_cmd(root)?;"),
        "xtask/src/main.rs must expose unified `ratchet` CLI subcommand honoring `--dry-run` and run `check-wasm-sync` in `validate`"
    );
}
