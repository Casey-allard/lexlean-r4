//! Conformance test for R4-62: Compact Cursor-Grade Studio UI, Non-Blocking Dropdown
//! Neural HUD & Zero-Prompt First-Party Qwen3 Auto-Hydration.

#![allow(missing_docs)]

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_studio_frame::{
    admit_catalog_hydration, admit_hud_presentation, max_catalog_hydration_concurrency,
    verify_compact_studio_auto_hydration_admitted, CatalogHydrationVerdict, HudBackdropVerdict,
    HudPresentationMode, HydrationOriginPolicy,
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
fn test_r4_62_compact_studio_auto_hydration_r4_62() {
    let root = repo_root();

    // 1. Concurrency bound & positive/negative catalog hydration policy (R4-62, R5).
    assert_eq!(max_catalog_hydration_concurrency(), 3);
    assert_eq!(
        admit_catalog_hydration(HydrationOriginPolicy::SameOrigin, false, false, false),
        CatalogHydrationVerdict::HydrationAdmitted
    );
    assert_eq!(
        admit_catalog_hydration(
            HydrationOriginPolicy::FirstPartyCatalog,
            false,
            false,
            false
        ),
        CatalogHydrationVerdict::HydrationAdmitted
    );
    assert_eq!(
        admit_catalog_hydration(
            HydrationOriginPolicy::ExternalUntrustedOrigin,
            true,
            false,
            false
        ),
        CatalogHydrationVerdict::HydrationAdmitted
    );
    assert_eq!(
        admit_catalog_hydration(
            HydrationOriginPolicy::ExternalUntrustedOrigin,
            false,
            false,
            false
        ),
        CatalogHydrationVerdict::RefusedExternalOriginConsentRequired
    );
    assert_eq!(
        admit_catalog_hydration(HydrationOriginPolicy::FirstPartyCatalog, false, false, true),
        CatalogHydrationVerdict::RefusedSilentModelDowngrade
    );
    assert_eq!(
        admit_catalog_hydration(HydrationOriginPolicy::FirstPartyCatalog, false, true, false),
        CatalogHydrationVerdict::EmergencyOfflineQ31Admitted
    );
    assert_eq!(
        admit_catalog_hydration(HydrationOriginPolicy::FirstPartyCatalog, false, true, true),
        CatalogHydrationVerdict::EmergencyOfflineQ31Admitted
    );

    // 2. Non-blocking Neural HUD presentation verdict & typed blocking-backdrop refusal (R5).
    for mode in [
        HudPresentationMode::DropdownCompactStrip,
        HudPresentationMode::DropdownExpandedDeck,
        HudPresentationMode::ChromelessEmbed,
    ] {
        assert_eq!(
            admit_hud_presentation(mode, false),
            HudBackdropVerdict::HudNonBlockingAdmitted
        );
        assert_eq!(
            admit_hud_presentation(mode, true),
            HudBackdropVerdict::RefusedBlockingBackdropOnLiveHud
        );
    }
    assert!(verify_compact_studio_auto_hydration_admitted());

    // 3. Formal LexLean spec, controls, model/ids.toml, and Gherkin suite presence.
    check_lex_module(
        &root,
        "src/R4/StudioFrame.lex.tex",
        "R4.StudioFrame",
        &[
            "HydrationOriginPolicy",
            "CatalogHydrationVerdict",
            "HudPresentationMode",
            "HudBackdropVerdict",
            "maxCatalogHydrationConcurrency",
            "admitCatalogHydration",
            "admitHudPresentation",
            "verifyCompactStudioAutoHydrationAdmitted",
            "breaksSilentModelDowngradeAdmitted",
            "breaksExternalUntrustedWithoutConsentAdmitted",
            "breaksBlockingBackdropOnLiveHudAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/StudioFrameControls.lex.tex",
        "StudioFrameControls",
        &[
            "compactStudioAutoHydrationAdmittedCtrl",
            "breaksSilentModelDowngradeAdmittedCtrl",
            "breaksExternalUntrustedWithoutConsentAdmittedCtrl",
            "breaksBlockingBackdropOnLiveHudAdmittedCtrl",
        ],
    );

    let ids_toml =
        std::fs::read_to_string(root.join("model/ids.toml")).expect("model/ids.toml must exist");
    assert!(
        ids_toml.contains("id = \"R4-62\""),
        "R4-62 must be registered in model/ids.toml"
    );
    let feature_file = root.join("features/suites/r4_62_compact_studio_auto_hydration.feature");
    assert!(feature_file.is_file(), "R4-62 feature suite must exist");
    let feature_text = std::fs::read_to_string(feature_file).expect("read R4-62 feature file");
    assert!(feature_text.contains("@R4-62 @build"));
}
