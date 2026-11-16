//! Conformance test for R4-46: Honest model-routed Studio plumbing, shard magic
//! container admission, OPFS `.uorc` generation capsule admission, and 3-strike watchdog rollback.

#[path = "../../wasm/src/lexlean_generated.rs"]
mod lexlean_generated;

use lexlean_generated::r4_honest_plumbing::{
    admit_model_routing, admit_shard_container_magic, admit_uorc_generation_capsule,
    evaluate_generation_watchdog, optimal_browser_worker_count,
    should_rollback_generation_watchdog, ComposerRouteMode, ModelRoutingCheck, ModelRoutingVerdict,
    ShardMagicKind, UorcCapsuleCheck, UorcCapsuleVerdict, WatchdogStateCheck, WatchdogVerdict,
    WATCHDOG_STRIKE_THRESHOLD,
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

fn valid_capsule() -> UorcCapsuleCheck {
    UorcCapsuleCheck {
        magic_uorc_valid: true,
        gates_passed: 6,
        gates_total: 6,
        residue_count: 0,
        coercive_patterns: 0,
        wasm_bytes: 512_000,
        max_wasm_bytes: 15 * 1024 * 1024,
        frame_time_micros: 4_000,
        frame_budget_micros: 16_667,
        candidate_loss: 90,
        incumbent_loss: 100,
        candidate_steps: 20,
        incumbent_steps: 22,
    }
}

#[test]
fn test_honest_model_routed_plumbing_and_uorc_watchdog_r4_46() {
    let root = repo_root();

    check_lex_module(
        &root,
        "src/R4/HonestPlumbing.lex.tex",
        "R4.HonestPlumbing",
        &[
            "watchdogStrikeThreshold",
            "admitModelRouting",
            "admitShardContainerMagic",
            "admitUorcGenerationCapsule",
            "evaluateGenerationWatchdog",
            "shouldRollbackGenerationWatchdog",
            "optimalBrowserWorkerCount",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/HonestPlumbingControls.lex.tex",
        "HonestPlumbingControls",
        &[
            "holoV13RoutingAdmitted",
            "breaksRetiredScaffoldAdmitted",
            "breaksTemplateHijackInNeuralMode",
            "breaksThreeStrikeWatchdogNoRollback",
            "watchdogRollsBackAtStrikeThreshold",
            "watchdogRefusesGenZeroRollback",
        ],
    );

    // All four clean v13 UI/runtime modes round-trip wire IDs honestly.
    let modes = [
        (
            ComposerRouteMode::Qwen306bHolo,
            "uor-r4.qwen3-0.6b-holo",
            true,
        ),
        (
            ComposerRouteMode::Qwen317bHolo,
            "uor-r4.qwen3-1.7b-holo",
            true,
        ),
        (
            ComposerRouteMode::NativeQ31D85,
            "uor-r4.native-q31-d85",
            true,
        ),
        (ComposerRouteMode::HfRepoHolo, "uor-r4.hf-repo-holo", true),
    ];
    for (mode, wire, is_neural) in modes {
        assert_eq!(mode.wire_id(), wire);
        assert_eq!(ComposerRouteMode::from_wire_id(wire), Some(mode));
        assert_eq!(mode.is_neural_geometric(), is_neural);
        assert_eq!(
            admit_model_routing(ModelRoutingCheck {
                requested_mode: mode,
                executed_mode: mode,
                retired_scaffold_requested: false,
                template_override_used: false,
                turn_memory_hijack_used: false,
                stale_banner_present: false,
                requested_weights_mounted: true,
            }),
            ModelRoutingVerdict::RoutingAdmitted
        );
    }
    for retired in [
        "uor-r4.seed-fast",
        "uor-r4.lut-symbolic",
        "uor-r4.recompiled-v12-agent",
        "uor-r4.recompiled-v12-chat",
        "uor-r4.gllm-v10",
    ] {
        assert_eq!(ComposerRouteMode::from_wire_id(retired), None);
        assert!(ComposerRouteMode::is_retired_scaffold_wire(retired));
    }

    // Typed routing refusals (R5).
    assert_eq!(
        admit_model_routing(ModelRoutingCheck {
            requested_mode: ComposerRouteMode::Qwen306bHolo,
            executed_mode: ComposerRouteMode::NativeQ31D85,
            retired_scaffold_requested: false,
            template_override_used: false,
            turn_memory_hijack_used: false,
            stale_banner_present: false,
            requested_weights_mounted: true,
        }),
        ModelRoutingVerdict::RefusedModeMismatch
    );
    assert_eq!(
        admit_model_routing(ModelRoutingCheck {
            requested_mode: ComposerRouteMode::Qwen306bHolo,
            executed_mode: ComposerRouteMode::Qwen306bHolo,
            retired_scaffold_requested: true,
            template_override_used: false,
            turn_memory_hijack_used: false,
            stale_banner_present: false,
            requested_weights_mounted: true,
        }),
        ModelRoutingVerdict::RefusedRetiredScaffoldMode
    );
    assert_eq!(
        admit_model_routing(ModelRoutingCheck {
            requested_mode: ComposerRouteMode::Qwen317bHolo,
            executed_mode: ComposerRouteMode::Qwen317bHolo,
            retired_scaffold_requested: false,
            template_override_used: true,
            turn_memory_hijack_used: false,
            stale_banner_present: false,
            requested_weights_mounted: true,
        }),
        ModelRoutingVerdict::RefusedTemplateHijackInNeuralMode
    );
    assert_eq!(
        admit_model_routing(ModelRoutingCheck {
            requested_mode: ComposerRouteMode::NativeQ31D85,
            executed_mode: ComposerRouteMode::NativeQ31D85,
            retired_scaffold_requested: false,
            template_override_used: false,
            turn_memory_hijack_used: false,
            stale_banner_present: true,
            requested_weights_mounted: true,
        }),
        ModelRoutingVerdict::RefusedStaleVersionBanner
    );
    assert_eq!(
        admit_model_routing(ModelRoutingCheck {
            requested_mode: ComposerRouteMode::HfRepoHolo,
            executed_mode: ComposerRouteMode::HfRepoHolo,
            retired_scaffold_requested: false,
            template_override_used: false,
            turn_memory_hijack_used: false,
            stale_banner_present: false,
            requested_weights_mounted: false,
        }),
        ModelRoutingVerdict::RefusedRequestedModelUnmounted
    );

    // Container magic routing accepts UORR4V13, UORR4V12, and legacy UORGLLM9 while refusing unknown headers.
    assert_eq!(
        admit_shard_container_magic(b"UORR4V13\x00\x00"),
        ShardMagicKind::UorR4V13
    );
    assert_eq!(
        admit_shard_container_magic(b"UORR4V12\x00\x00"),
        ShardMagicKind::UorR4V12
    );
    assert_eq!(
        admit_shard_container_magic(b"UORGLLM9\x00\x00"),
        ShardMagicKind::UorGllm9
    );
    assert_eq!(
        admit_shard_container_magic(b"GARBAGE!"),
        ShardMagicKind::Unknown
    );

    // OPFS UORC\x01 generation capsule admission & R5 negative tests.
    assert_eq!(
        admit_uorc_generation_capsule(valid_capsule()),
        UorcCapsuleVerdict::CapsuleAdmitted
    );
    assert_eq!(
        admit_uorc_generation_capsule(UorcCapsuleCheck {
            magic_uorc_valid: false,
            ..valid_capsule()
        }),
        UorcCapsuleVerdict::RefusedInvalidArchiveMagic
    );
    assert_eq!(
        admit_uorc_generation_capsule(UorcCapsuleCheck {
            gates_passed: 5,
            ..valid_capsule()
        }),
        UorcCapsuleVerdict::RefusedFailedVerificationGates
    );
    assert_eq!(
        admit_uorc_generation_capsule(UorcCapsuleCheck {
            residue_count: 1,
            ..valid_capsule()
        }),
        UorcCapsuleVerdict::RefusedNonZeroResidueOrCoercion
    );
    assert_eq!(
        admit_uorc_generation_capsule(UorcCapsuleCheck {
            wasm_bytes: 16 * 1024 * 1024,
            ..valid_capsule()
        }),
        UorcCapsuleVerdict::RefusedWasmByteBudgetExceeded
    );
    assert_eq!(
        admit_uorc_generation_capsule(UorcCapsuleCheck {
            frame_time_micros: 20_000,
            ..valid_capsule()
        }),
        UorcCapsuleVerdict::RefusedFrameBudgetExceeded
    );
    assert_eq!(
        admit_uorc_generation_capsule(UorcCapsuleCheck {
            candidate_loss: 100,
            candidate_steps: 22,
            ..valid_capsule()
        }),
        UorcCapsuleVerdict::RefusedParetoRegression
    );

    // 3-strike watchdog rollback semantics.
    assert_eq!(WATCHDOG_STRIKE_THRESHOLD, 3);
    assert_eq!(
        evaluate_generation_watchdog(WatchdogStateCheck {
            generation_index: 2,
            consecutive_faults: 0,
            has_certified_parent: true,
            watchdog_strike_limit: WATCHDOG_STRIKE_THRESHOLD,
        }),
        WatchdogVerdict::Healthy
    );
    assert_eq!(
        evaluate_generation_watchdog(WatchdogStateCheck {
            generation_index: 2,
            consecutive_faults: 2,
            has_certified_parent: true,
            watchdog_strike_limit: WATCHDOG_STRIKE_THRESHOLD,
        }),
        WatchdogVerdict::FaultRecordedContinue
    );
    assert_eq!(
        evaluate_generation_watchdog(WatchdogStateCheck {
            generation_index: 2,
            consecutive_faults: 3,
            has_certified_parent: true,
            watchdog_strike_limit: WATCHDOG_STRIKE_THRESHOLD,
        }),
        WatchdogVerdict::RollbackToParentHead
    );
    assert_eq!(
        evaluate_generation_watchdog(WatchdogStateCheck {
            generation_index: 0,
            consecutive_faults: 3,
            has_certified_parent: false,
            watchdog_strike_limit: WATCHDOG_STRIKE_THRESHOLD,
        }),
        WatchdogVerdict::RefusedRollbackAtGenZero
    );
    assert!(!should_rollback_generation_watchdog(1, 2));
    assert!(should_rollback_generation_watchdog(1, 3));
    assert!(!should_rollback_generation_watchdog(0, 5));

    // Browser Worker pool concurrency scaling bounded within [1..8].
    assert_eq!(optimal_browser_worker_count(1, 8, true), 1);
    assert_eq!(optimal_browser_worker_count(8, 4, true), 4);
    assert_eq!(optimal_browser_worker_count(16, 16, true), 8);
    assert_eq!(optimal_browser_worker_count(16, 16, false), 2);
}
