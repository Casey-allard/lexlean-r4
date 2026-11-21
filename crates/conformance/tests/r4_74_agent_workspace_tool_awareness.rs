//! Conformance test for R4-74: Unified Agent `/workspace` VFS, RV64 MVM Terminal `<tool_call>`
//! Self-Awareness, Multi-Hop ChatML Continuation, and Zero-HTTP OPFS Warm Mount & KV Prefix Reuse.

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_agent_workspace_tool_awareness::{
    admit_agent_workspace_tool_prompt, admit_turn_hydration_and_kv_reuse,
    verify_agent_workspace_tool_awareness_admitted, AgentToolPromptVerdict, WarmMountCacheVerdict,
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
fn test_r4_74_agent_workspace_tool_awareness_r4_74() {
    let root = repo_root();

    // 1. Formal LexLean R4-74 admission and R5 negative refusals
    assert!(verify_agent_workspace_tool_awareness_admitted());
    assert_eq!(
        admit_agent_workspace_tool_prompt(true, true, 4, true, false, 52, 1, true),
        AgentToolPromptVerdict::WorkspaceToolPromptAdmitted
    );
    assert_eq!(
        admit_agent_workspace_tool_prompt(false, true, 4, true, false, 32, 0, true),
        AgentToolPromptVerdict::RefusedBlindSurfaceWithoutMvmSchema
    );
    assert_eq!(
        admit_agent_workspace_tool_prompt(true, true, 4, false, false, 32, 0, true),
        AgentToolPromptVerdict::RefusedMissingWorkspaceTreeWhenFilesExist
    );
    assert_eq!(
        admit_agent_workspace_tool_prompt(true, true, 4, true, false, 920, 0, true),
        AgentToolPromptVerdict::RefusedUncachedExcerptBloat
    );
    assert_eq!(
        admit_agent_workspace_tool_prompt(true, true, 4, true, false, 48, 1, false),
        AgentToolPromptVerdict::RefusedToolHopMissingAssistantCallTurn
    );

    assert_eq!(
        admit_turn_hydration_and_kv_reuse(true, true, false, false, 0, true),
        WarmMountCacheVerdict::InMemoryAlreadyMountedZeroFetch
    );
    assert_eq!(
        admit_turn_hydration_and_kv_reuse(false, true, false, false, 0, true),
        WarmMountCacheVerdict::OpfsWarmSlabMountedZeroHttp
    );
    assert_eq!(
        admit_turn_hydration_and_kv_reuse(false, true, false, false, 1, true),
        WarmMountCacheVerdict::RefusedRedundantHttpFetchOnMountedOrWarmSlab
    );
    assert_eq!(
        admit_turn_hydration_and_kv_reuse(true, true, false, false, 0, false),
        WarmMountCacheVerdict::RefusedSurfaceFlipKvCacheWipe
    );

    // 2. Verify assistant.rs binds STUDIO_AGENT_TOOL_SCHEMA (mvm_exec + /workspace <tools> + <tool_call>)
    //    and ChatML assistant <tool_call> turn on multi-hop tool continuations
    let assistant_rs = std::fs::read_to_string(root.join("crates/wasm/src/assistant.rs"))
        .expect("read assistant.rs");
    assert!(
        assistant_rs.contains("STUDIO_AGENT_TOOL_SCHEMA"),
        "assistant.rs must define and bind prewarmed STUDIO_AGENT_TOOL_SCHEMA"
    );
    assert!(
        assistant_rs.contains("\"name\":\"mvm_exec\"")
            && assistant_rs.contains("\"name\":\"workspace_read\"")
            && assistant_rs.contains("\"name\":\"workspace_write\""),
        "STUDIO_AGENT_TOOL_SCHEMA must expose mvm_exec, workspace_read, and workspace_write JSON signatures"
    );
    assert!(
        assistant_rs.contains("format_foundry_tool_hop_chatml"),
        "assistant.rs must format multi-hop tool continuations with assistant <tool_call> ChatML turns"
    );

    // 3. Verify public/app.js stops flipping surface_mode to tool-blind 'chat' and preserves history across Studio turns
    let app_js = std::fs::read_to_string(root.join("public/app.js")).expect("read public/app.js");
    assert!(
        !app_js.contains("const activeSurfaceMode = wantsWorkspaceTools ? 'studio' : 'chat';"),
        "public/app.js must not strip /workspace or flip surface_mode to tool-blind 'chat' on non-regex prompts"
    );

    // 4. Verify worker_host_bridge.js checks OPFS HOLOSLAB2 warm seal for catalog models BEFORE any HTTP manifest fetch
    let bridge_js = std::fs::read_to_string(root.join("crates/wasm/js/worker_host_bridge.js"))
        .expect("read worker_host_bridge.js");
    assert!(
        bridge_js.contains("earlyCatalogWarmCheck"),
        "worker_host_bridge.js must fast-path verified catalog OPFS HOLOSLAB2 warm seals with zero HTTP manifest fetches"
    );

    // 5. Formal LexLean spec, controls, and model register presence
    check_lex_module(
        &root,
        "src/R4/AgentWorkspaceToolAwareness.lex.tex",
        "R4.AgentWorkspaceToolAwareness",
        &[
            "AgentToolPromptVerdict",
            "WarmMountCacheVerdict",
            "admitAgentWorkspaceToolPrompt",
            "admitTurnHydrationAndKvReuse",
            "verifyAgentWorkspaceToolAwarenessAdmitted",
            "breaksBlindSurfaceWithoutMvmSchemaAdmitted",
            "breaksUncachedExcerptBloatAdmitted",
            "breaksRedundantHttpFetchOnWarmSlabAdmitted",
            "breaksSurfaceFlipKvCacheWipeAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/AgentWorkspaceToolAwarenessControls.lex.tex",
        "AgentWorkspaceToolAwarenessControls",
        &[
            "agentWorkspaceToolAwarenessAdmitted",
            "breaksBlindSurfaceWithoutMvmSchemaAdmittedCtrl",
            "breaksUncachedExcerptBloatAdmittedCtrl",
            "breaksRedundantHttpFetchOnWarmSlabAdmittedCtrl",
            "breaksSurfaceFlipKvCacheWipeAdmittedCtrl",
        ],
    );

    let ids_toml =
        std::fs::read_to_string(root.join("model/ids.toml")).expect("model/ids.toml must exist");
    assert!(
        ids_toml.contains("id = \"R4-74\""),
        "R4-74 must be registered in model/ids.toml"
    );
}
