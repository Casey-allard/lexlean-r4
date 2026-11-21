//! Conformance test for R4-79: Content-Addressed GitHub Actions `.lexlean` Verification
//! Artifact Cache in `.github/workflows/pages.yml` with Immutable `bootstrap.yml`.

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_content_addressed_ci_cache::{
    admit_lexlean_ci_artifact_cache, verify_content_addressed_ci_cache_admitted,
    LexleanCiCacheVerdict,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
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

fn is_pinned_action_uses_line(line: &str) -> bool {
    let Some(rest) = line.trim().strip_prefix("uses:") else {
        return true;
    };
    let clean = rest.split('#').next().unwrap_or("").trim();
    let Some((name, sha)) = clean.rsplit_once('@') else {
        return false;
    };
    !name.is_empty()
        && sha.len() == 40
        && sha
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

#[test]
fn test_r4_79_content_addressed_ci_cache_r4_79() {
    let root = repo_root();

    // 1. Formal LexLean R4-79 admission and R5 negative refusals
    assert!(verify_content_addressed_ci_cache_admitted());
    assert_eq!(
        admit_lexlean_ci_artifact_cache(true, true, true, true),
        LexleanCiCacheVerdict::LexleanCiCacheAdmitted
    );
    assert_eq!(
        admit_lexlean_ci_artifact_cache(false, true, true, true),
        LexleanCiCacheVerdict::RefusedModifiedUniversalBootstrapWorkflow
    );
    assert_eq!(
        admit_lexlean_ci_artifact_cache(true, false, true, true),
        LexleanCiCacheVerdict::RefusedUnpinnedCacheActionReference
    );
    assert_eq!(
        admit_lexlean_ci_artifact_cache(true, true, false, true),
        LexleanCiCacheVerdict::RefusedMissingSdkOrLockfileCacheBinding
    );
    assert_eq!(
        admit_lexlean_ci_artifact_cache(true, true, true, false),
        LexleanCiCacheVerdict::RefusedMissingLexSourceGlobBinding
    );

    // 2. Verify .lex.tex semantic modules exist and declare required symbols
    check_lex_module(
        &root,
        "src/R4/ContentAddressedCiCache.lex.tex",
        "R4.ContentAddressedCiCache",
        &[
            "LexleanCiCacheVerdict",
            "admitLexleanCiArtifactCache",
            "verifyContentAddressedCiCacheAdmitted",
            "breaksModifiedBootstrapWorkflowAdmitted",
            "breaksUnpinnedCacheActionAdmitted",
            "breaksMissingSdkOrLockBindingAdmitted",
            "breaksMissingLexSourcesBindingAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/ContentAddressedCiCacheControls.lex.tex",
        "ContentAddressedCiCacheControls",
        &[
            "contentAddressedCiCacheAdmitted",
            "breaksModifiedBootstrapWorkflowAdmittedCtrl",
            "breaksUnpinnedCacheActionAdmittedCtrl",
            "breaksMissingSdkOrLockBindingAdmittedCtrl",
            "breaksMissingLexSourcesBindingAdmittedCtrl",
        ],
    );

    // 3. Verify universal bootstrap.yml matches template.lock byte-for-byte
    let template_lock: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("template.lock")).expect("read template.lock"),
    )
    .expect("parse template.lock");
    let expected_bootstrap_sha = template_lock["policy_files"]
        .as_array()
        .expect("policy_files array")
        .iter()
        .find(|row| row["path"] == ".github/workflows/bootstrap.yml")
        .and_then(|row| row["sha256"].as_str())
        .expect("bootstrap.yml sha256 in template.lock");
    let bootstrap_bytes =
        std::fs::read(root.join(".github/workflows/bootstrap.yml")).expect("read bootstrap.yml");
    let actual_bootstrap_sha = format!(
        "sha256:{}",
        Sha256::digest(&bootstrap_bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let bootstrap_untouched = actual_bootstrap_sha == expected_bootstrap_sha;
    assert!(
        bootstrap_untouched,
        "bootstrap.yml must remain untouched: expected {expected_bootstrap_sha}, got {actual_bootstrap_sha}"
    );

    // 4. Verify pages.yml caches path: .lexlean with 40-hex pinned actions/cache and full input bindings
    let pages_yml =
        std::fs::read_to_string(root.join(".github/workflows/pages.yml")).expect("read pages.yml");
    let all_pinned = pages_yml.lines().all(is_pinned_action_uses_line);
    let has_lexlean_path = pages_yml.contains("path: .lexlean");
    let sdk_and_lock_bound = has_lexlean_path
        && pages_yml.contains("r4-pages-lexlean-v1-")
        && pages_yml.contains("'prismpm.lock'")
        && pages_yml.contains("'lexlean.toml'")
        && pages_yml.contains("'lexlean.lock'");
    let lex_sources_bound = has_lexlean_path
        && pages_yml.contains("'src/R4/**/*.lex.tex'")
        && pages_yml.contains("'tests/lexlean/**/*.lex.tex'");

    assert_eq!(
        admit_lexlean_ci_artifact_cache(
            bootstrap_untouched,
            all_pinned,
            sdk_and_lock_bound,
            lex_sources_bound,
        ),
        LexleanCiCacheVerdict::LexleanCiCacheAdmitted,
        "pages.yml must configure commit-pinned content-addressed .lexlean caching"
    );
}
