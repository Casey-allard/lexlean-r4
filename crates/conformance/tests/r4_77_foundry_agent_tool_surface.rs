//! Conformance test for R4-77: Policy-Checked Foundry Quantum Studio `InTabProver` (`FK-01..09`),
//! `foundry_publish_holo`, and `vtensor_render_3d` Agent `<tool_call>` Surface.

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_foundry_agent_tool_surface::{
    admit_foundry_agent_tool_call, verify_foundry_agent_tool_surface_admitted,
    FoundryAgentToolKind, FoundryAgentToolVerdict,
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
fn test_r4_77_foundry_agent_tool_surface_r4_77() {
    let root = repo_root();

    // 1. Formal LexLean R4-77 admission and R5 negative refusals
    assert!(verify_foundry_agent_tool_surface_admitted());
    assert_eq!(
        admit_foundry_agent_tool_call(
            FoundryAgentToolKind::ProveWorkspaceInTab,
            true,
            false,
            0,
            0,
            0
        ),
        FoundryAgentToolVerdict::FoundryProveAdmitted
    );
    assert_eq!(
        admit_foundry_agent_tool_call(
            FoundryAgentToolKind::PublishHoloCapsule,
            false,
            true,
            0,
            0,
            0
        ),
        FoundryAgentToolVerdict::FoundryPublishHoloAdmitted
    );
    assert_eq!(
        admit_foundry_agent_tool_call(
            FoundryAgentToolKind::RenderVtensor3dPreview,
            false,
            false,
            0,
            4,
            0
        ),
        FoundryAgentToolVerdict::VtensorRender3dAdmitted
    );
    assert_eq!(
        admit_foundry_agent_tool_call(
            FoundryAgentToolKind::PublishHoloCapsule,
            true,
            true,
            0,
            0,
            0
        ),
        FoundryAgentToolVerdict::RefusedPlanModeMutatingPublish
    );
    assert_eq!(
        admit_foundry_agent_tool_call(
            FoundryAgentToolKind::PublishHoloCapsule,
            false,
            false,
            0,
            0,
            0
        ),
        FoundryAgentToolVerdict::RefusedPublishWithoutProvedWorkspace
    );
    assert_eq!(
        admit_foundry_agent_tool_call(
            FoundryAgentToolKind::ProveWorkspaceInTab,
            false,
            false,
            1,
            0,
            0
        ),
        FoundryAgentToolVerdict::RefusedUnsoundAxiomsInAgentProve
    );
    assert_eq!(
        admit_foundry_agent_tool_call(
            FoundryAgentToolKind::RenderVtensor3dPreview,
            false,
            false,
            0,
            5,
            0
        ),
        FoundryAgentToolVerdict::RefusedOutOfBoundsLodOrFloat3d
    );

    // 2. Verify .lex.tex semantic modules exist and declare all required symbols
    check_lex_module(
        &root,
        "src/R4/FoundryAgentToolSurface.lex.tex",
        "R4.FoundryAgentToolSurface",
        &[
            "FoundryAgentToolKind",
            "FoundryAgentToolVerdict",
            "admitFoundryAgentToolCall",
            "verifyFoundryAgentToolSurfaceAdmitted",
            "breaksPlanModePublishHoloAdmitted",
            "breaksPublishWithoutProveAdmitted",
            "breaksUnsoundAxiomsInAgentProveAdmitted",
            "breaksOutOfBounds3dLodOrFloatAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/FoundryAgentToolSurfaceControls.lex.tex",
        "FoundryAgentToolSurfaceControls",
        &[
            "foundryAgentToolSurfaceAdmitted",
            "breaksPlanModePublishHoloAdmittedCtrl",
            "breaksPublishWithoutProveAdmittedCtrl",
            "breaksUnsoundAxiomsInAgentProveAdmittedCtrl",
            "breaksOutOfBounds3dLodOrFloatAdmittedCtrl",
        ],
    );

    // 3. Verify STUDIO_AGENT_TOOL_SCHEMA, detect_studio_tool_call, StudioEngine::execute_studio_tool_json,
    //    and worker_host_bridge.js wire foundry_prove, foundry_publish_holo, and vtensor_render_3d
    let assistant_rs = std::fs::read_to_string(root.join("crates/wasm/src/assistant.rs"))
        .expect("read assistant.rs");
    assert!(
        assistant_rs.contains("\"name\":\"foundry_prove\"")
            && assistant_rs.contains("\"name\":\"foundry_publish_holo\"")
            && assistant_rs.contains("\"name\":\"vtensor_render_3d\""),
        "STUDIO_AGENT_TOOL_SCHEMA and detect_studio_tool_call must wire foundry_prove, foundry_publish_holo, and vtensor_render_3d"
    );

    let lib_rs = std::fs::read_to_string(root.join("crates/wasm/src/lib.rs")).expect("read lib.rs");
    assert!(
        lib_rs.contains("\"foundry_prove\"")
            && lib_rs.contains("\"foundry_publish_holo\"")
            && lib_rs.contains("\"vtensor_render_3d\""),
        "StudioEngine::execute_studio_tool_json must execute foundry_prove, foundry_publish_holo, and vtensor_render_3d"
    );

    let bridge_js = std::fs::read_to_string(root.join("crates/wasm/js/worker_host_bridge.js"))
        .expect("read worker_host_bridge.js");
    assert!(
        bridge_js.contains("foundry_prove")
            && bridge_js.contains("foundry_publish_holo")
            && bridge_js.contains("vtensor_render_3d"),
        "worker_host_bridge.js must execute foundry_prove, foundry_publish_holo, and vtensor_render_3d tool calls"
    );
}
