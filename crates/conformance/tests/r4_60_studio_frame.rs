//! Conformance test for R4-60: Responsive Studio Frame, Agent/Plan tool policy,
//! off-origin hydration consent, and live token HUD frame invariants.

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_studio_frame::{
    admit_live_token_frame, admit_off_origin_hydration, admit_studio_tool, classify_window_width,
    surface_is_chromeless_embed, verify_studio_frame_admitted, ComposerToolPolicy,
    HydrationConsentVerdict, LiveFrameVerdict, StudioSurfaceMode, WindowSizeClass,
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
fn test_r4_60_studio_frame_live_hud_r4_60() {
    let root = repo_root();

    // 1. Responsive window size classification breakpoints
    assert_eq!(classify_window_width(0), WindowSizeClass::Compact);
    assert_eq!(classify_window_width(390), WindowSizeClass::Compact);
    assert_eq!(classify_window_width(599), WindowSizeClass::Compact);
    assert_eq!(classify_window_width(600), WindowSizeClass::Medium);
    assert_eq!(classify_window_width(839), WindowSizeClass::Medium);
    assert_eq!(classify_window_width(840), WindowSizeClass::Expanded);
    assert_eq!(classify_window_width(1440), WindowSizeClass::Expanded);

    // Ensure all StudioSurfaceMode variants and embed helper behave cleanly
    let surfaces = [
        StudioSurfaceMode::Chat,
        StudioSurfaceMode::Studio,
        StudioSurfaceMode::ObservatoryEmbed,
        StudioSurfaceMode::PreviewEmbed,
    ];
    assert_eq!(surfaces.len(), 4);
    assert!(!surface_is_chromeless_embed(StudioSurfaceMode::Chat));
    assert!(!surface_is_chromeless_embed(StudioSurfaceMode::Studio));
    assert!(surface_is_chromeless_embed(
        StudioSurfaceMode::ObservatoryEmbed
    ));
    assert!(surface_is_chromeless_embed(StudioSurfaceMode::PreviewEmbed));
    assert!(verify_studio_frame_admitted());

    // 2. Composer Agent/Plan tool policy (with R5 negative refusal for Plan mutating tools)
    assert!(admit_studio_tool(ComposerToolPolicy::AgentFull, true));
    assert!(admit_studio_tool(ComposerToolPolicy::AgentFull, false));
    assert!(admit_studio_tool(ComposerToolPolicy::PlanReadOnly, false));
    assert!(!admit_studio_tool(ComposerToolPolicy::PlanReadOnly, true));

    // 3. Off-origin hydration consent gating & offline Q31 fallback
    assert_eq!(
        admit_off_origin_hydration(true, false, false),
        HydrationConsentVerdict::AdmittedSameOrigin
    );
    assert_eq!(
        admit_off_origin_hydration(false, true, false),
        HydrationConsentVerdict::AdmittedConsentedOffOrigin
    );
    assert_eq!(
        admit_off_origin_hydration(false, false, true),
        HydrationConsentVerdict::FallbackOfflineQ31
    );
    assert_eq!(
        admit_off_origin_hydration(false, false, false),
        HydrationConsentVerdict::RefusedOffOriginConsentRequired
    );

    // 4. Live token HUD frame admission & typed R5 refusals
    assert_eq!(
        admit_live_token_frame(false, true, 10, 100_000),
        LiveFrameVerdict::RefusedSyntheticHammingOnQ31
    );
    assert_eq!(
        admit_live_token_frame(true, true, 100_001, 100_000),
        LiveFrameVerdict::RefusedPreviewFuelExhausted
    );
    assert_eq!(
        admit_live_token_frame(true, true, 120, 100_000),
        LiveFrameVerdict::FrameAdmitted
    );
    assert_eq!(
        admit_live_token_frame(false, false, 10, 100_000),
        LiveFrameVerdict::FrameAdmitted
    );

    // 5. Formal LexLean spec, controls, and model register presence
    check_lex_module(
        &root,
        "src/R4/StudioFrame.lex.tex",
        "R4.StudioFrame",
        &[
            "WindowSizeClass",
            "StudioSurfaceMode",
            "ComposerToolPolicy",
            "HydrationConsentVerdict",
            "LiveFrameVerdict",
            "classifyWindowWidth",
            "admitStudioTool",
            "admitOffOriginHydration",
            "admitLiveTokenFrame",
            "verifyStudioFrameAdmitted",
            "breaksPlanMutationAdmitted",
            "breaksOffOriginUnconsentedAdmitted",
            "breaksSyntheticHammingOnQ31Admitted",
            "breaksPreviewFuelExhaustedAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/StudioFrameControls.lex.tex",
        "StudioFrameControls",
        &[
            "studioFrameAdmitted",
            "breaksPlanMutationAdmittedCtrl",
            "breaksOffOriginUnconsentedAdmittedCtrl",
            "breaksSyntheticHammingOnQ31AdmittedCtrl",
            "breaksPreviewFuelExhaustedAdmittedCtrl",
        ],
    );

    let ids_toml =
        std::fs::read_to_string(root.join("model/ids.toml")).expect("model/ids.toml must exist");
    assert!(
        ids_toml.contains("id = \"R4-60\""),
        "R4-60 must be registered in model/ids.toml"
    );

    let feature_file = root.join("features/suites/r4_60_studio_frame_live_hud.feature");
    assert!(feature_file.is_file(), "R4-60 feature suite must exist");
    let feature_text = std::fs::read_to_string(feature_file).expect("read feature file");
    assert!(feature_text.contains("@R4-60 @build"));
}
