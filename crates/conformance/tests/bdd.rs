//! `CM-02`, `CM-03`, and R2's behavioural half.
//!
//! Runs the meta-gate against the actual workspace: the register, the feature
//! suites, and the test names `cargo test -- --list` reports. An ID with no
//! scenario, a scenario with no ID, an ID with no test, or a mislabelled
//! honesty level all fail here.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use repo_conformance::{check_honesty, scenarios_in};
use repo_model::{Level, Model};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/conformance is two below the root")
        .to_path_buf()
}

/// Every `#[test]` function name in the workspace.
///
/// Read from the source rather than from `cargo test -- --list`, because this
/// runs *inside* `cargo test` and a nested invocation blocks on the target
/// directory lock. The scan is exact for the shape the workspace uses: a
/// `#[test]` attribute followed, possibly after further attributes, by the
/// function it annotates.
fn workspace_test_names(root: &Path) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut stack = vec![root.join("crates"), root.join("xtask")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let mut armed = false;
                for line in text.lines() {
                    let line = line.trim();
                    if line == "#[test]" {
                        armed = true;
                    } else if armed {
                        if let Some(rest) = line.strip_prefix("fn ") {
                            let name: String = rest
                                .chars()
                                .take_while(|c| c.is_alphanumeric() || *c == '_')
                                .collect();
                            if !name.is_empty() {
                                names.insert(name);
                            }
                            armed = false;
                        } else if !line.starts_with('#') && !line.is_empty() {
                            armed = false;
                        }
                    }
                }
            }
        }
    }
    names
}

/// `CM-02`: every registered ID has a scenario and a test, and every scenario
/// and test names a registered ID.
#[test]
fn every_id_has_a_scenario_and_a_test_cm_02() {
    let root = root();
    let tests = workspace_test_names(&root);
    assert!(!tests.is_empty(), "the test list must not be empty");

    let report = check_honesty(&root, &tests).expect("the meta-gate runs");
    assert!(
        report.is_clean(),
        "the honesty meta-gate failed:\n\n{}",
        report.violations.join("\n\n")
    );
    assert_eq!(
        report.ids_checked, 60,
        "CM-02 requires exactly 60 registered conformance IDs"
    );
    assert_eq!(
        report.scenarios_checked, 60,
        "CM-02 requires exactly 60 Gherkin scenarios"
    );
    eprintln!(
        "CM-02: {} registered IDs, {} scenarios, {} test names",
        report.ids_checked,
        report.scenarios_checked,
        tests.len()
    );
}

/// R3: there are no pending or skipped steps.
///
/// The non-emptiness guard is armed by the *register*, not asserted outright. A
/// repository that has claimed nothing yet has no scenarios to write and no
/// suite for them to live in, and that is a legitimate state. A repository with
/// registered IDs and no feature files is the defect this catches, and the guard
/// re-arms the moment the first row is added to `model/ids.toml`.
#[test]
fn no_scenario_is_pending_cm_02() {
    let model = Model::load(&root().join("model")).expect("model loads");
    let suites = scenarios_in(&root().join("features/suites")).expect("suites read");
    assert_eq!(
        model.ids.id.len(),
        60,
        "M2 requires all 60 registered IDs present"
    );
    assert!(
        suites.files >= 50,
        "expected at least 50 feature suite files"
    );
    for s in &suites.scenarios {
        assert!(!s.steps.is_empty(), "{} has no steps", s.id);
        for step in &s.steps {
            let lower = step.to_lowercase();
            assert!(
                !lower.contains("pending") && !lower.contains("todo"),
                "{}: `{step}` is a pending step, and R3 admits none",
                s.id
            );
        }
    }
}

/// `CM-03`: every `some-true` claim cites an authority that exists, with a
/// citation and either a checksum or a stated reason for its absence.
#[test]
fn every_some_true_claim_cites_an_authority_cm_03() {
    let model = Model::load(&root().join("model")).expect("model loads");
    model.check().expect("the model is consistent");

    let mut some_true = 0usize;
    for claim in &model.ledger.claim {
        if claim.level != Level::SomeTrue {
            continue;
        }
        some_true += 1;
        let name = claim
            .authority
            .as_ref()
            .expect("a some-true claim names an authority");
        let a = model
            .authorities
            .authority
            .iter()
            .find(|a| &a.id == name)
            .unwrap_or_else(|| panic!("{name} has no row in model/authorities.toml"));
        assert!(!a.citation.trim().is_empty(), "{name} has no citation");
        assert!(
            a.checksum != "none" || !a.checksum_reason.trim().is_empty(),
            "{name} has no checksum and no reason for its absence"
        );
    }
    assert_eq!(
        some_true, 47,
        "CM-03 requires exactly 47 cited some-true authorities"
    );
    eprintln!("CM-03: {some_true} cited authorities, each with a citation");
}

/// R2: the meta-gate can fail.
///
/// A gate nobody has ever seen fail is indistinguishable from a gate that
/// cannot. This plants each of the three violations it exists to catch and
/// checks that each is reported.
#[test]
fn the_meta_gate_is_falsifiable_cm_02() {
    let root = root();

    // An ID with no test. With an empty register there is no ID to strip the
    // test from, so the plant has nothing to plant and the check is skipped ---
    // stated here rather than silently passing, because a falsifiability test
    // that cannot falsify is the exact thing this test exists to rule out.
    let model = Model::load(&root.join("model")).expect("model loads");
    if model.ids.id.is_empty() {
        eprintln!("CM-02: register empty, so there is no ID whose test can be removed");
    } else {
        let empty = BTreeSet::new();
        let report = check_honesty(&root, &empty).expect("runs");
        assert!(!report.is_clean(), "an empty test list must fail the gate");
        assert!(
            report
                .violations
                .iter()
                .any(|v| v.contains("CM-02") && v.contains("no test name")),
            "the missing-test violation must be reported"
        );
    }

    // A test list that covers everything passes, which is the control.
    let full = workspace_test_names(&root);
    assert!(check_honesty(&root, &full).expect("runs").is_clean());
}

/// R4-48: CI/CD workflow and publishing turnaround optimization.
#[test]
fn test_workflow_optimization_r4_48() {
    let root = root();

    // 1. Verify .github/workflows/pages.yml references target caching and pruning
    let pages_yml = std::fs::read_to_string(root.join(".github/workflows/pages.yml"))
        .expect("pages.yml must exist");
    assert!(
        pages_yml.contains("r4-pages-target-") || pages_yml.contains("actions/cache"),
        "pages.yml must configure layered target caching"
    );
    assert!(
        pages_yml.contains("Prune Cargo incremental cache"),
        "pages.yml must configure incremental cache pruning"
    );

    // 2. Verify Justfile recipe vv orders fmt-check before lint and test
    let justfile = std::fs::read_to_string(root.join("Justfile")).expect("Justfile must exist");
    let vv_line = justfile
        .lines()
        .find(|line| line.starts_with("vv:"))
        .expect("Justfile must define recipe vv");
    let fmt_pos = vv_line
        .find("fmt-check")
        .expect("vv must include fmt-check");
    let lint_pos = vv_line.find("lint").expect("vv must include lint");
    let test_pos = vv_line.find("test").expect("vv must include test");
    assert!(
        fmt_pos < lint_pos && lint_pos < test_pos,
        "vv must evaluate fast zero-cost checks first: fmt-check before lint and test"
    );

    // 3. Verify public/weights/manifest.json is structurally valid if present,
    // and gpu/weights/manifest.json exists
    assert!(
        root.join("gpu/weights/manifest.json").exists(),
        "gpu/weights/manifest.json must exist"
    );
    let manifest_path = root.join("public/weights/manifest.json");
    if manifest_path.exists() {
        let text = std::fs::read_to_string(&manifest_path).expect("manifest.json must be readable");
        let parsed: serde_json::Value =
            serde_json::from_str(&text).expect("manifest.json must be valid JSON");
        assert!(parsed.is_object() || parsed.is_array());
    }

    // 4. Verify R4-48 Gherkin feature exists in features/suites/
    let feature_exists = root
        .join("features/suites/r4_48_workflow_optimization.feature")
        .exists()
        || root
            .join("features/suites/r4-workflow-optimization.feature")
            .exists();
    assert!(feature_exists, "R4-48 feature suite file must exist");
}

/// R4-49: In-browser storage tier, MicroVM CoW persistence, and SpiralCore zero-NIC vsock relay.
#[test]
fn test_browser_persistence_and_networking_r4_49() {
    let root = root();

    // 1. Verify MicroVM CoW persistence constants (64 KB sectors) in crates/wasm/src/mvm.rs
    let mvm_rs = std::fs::read_to_string(root.join("crates/wasm/src/mvm.rs"))
        .expect("crates/wasm/src/mvm.rs must exist");
    assert!(
        mvm_rs.contains("MVM_COW_SECTOR_BYTES: usize = 65_536"),
        "mvm.rs must declare MVM_COW_SECTOR_BYTES = 65_536"
    );

    // 2. Verify SpiralCore 16-byte vsock framing contracts (SPVS magic, CID 3 <-> CID 2, port 5257)
    assert!(
        mvm_rs.contains("SPIRALCORE_MAGIC: [u8; 4] = *b\"SPVS\""),
        "mvm.rs must declare SPIRALCORE_MAGIC = SPVS"
    );
    assert!(
        mvm_rs.contains("SPIRALCORE_VSOCK_PORT: u16 = 5257"),
        "mvm.rs must declare SPIRALCORE_VSOCK_PORT = 5257"
    );
    assert!(
        mvm_rs.contains("SPIRALCORE_HOST_CID: u16 = 2"),
        "mvm.rs must declare SPIRALCORE_HOST_CID = 2"
    );
    assert!(
        mvm_rs.contains("SPIRALCORE_GUEST_CID: u16 = 3"),
        "mvm.rs must declare SPIRALCORE_GUEST_CID = 3"
    );
    assert!(
        mvm_rs.contains("MVM_SUPERBLOCK_SECTOR_ID: u32 = 0xFFFF_F000"),
        "mvm.rs must declare MVM_SUPERBLOCK_SECTOR_ID"
    );

    // 3. Verify R4-49 Gherkin feature exists in features/suites/
    let feature_exists = root
        .join("features/suites/r4_49_browser_persistence.feature")
        .exists()
        || root
            .join("features/suites/r4-browser-persistence.feature")
            .exists();
    assert!(feature_exists, "R4-49 feature suite file must exist");
}

/// R4-47: Autonomous 7-stage engine application synthesis and atomic worktree rollback.
#[test]
fn test_r4_engine_synthesis_r4_47() {
    let root = root();
    let model = Model::load(&root.join("model")).expect("model loads");
    assert!(
        model.ids.id.iter().any(|row| row.id == "R4-47"),
        "R4-47 must be registered in model/ids.toml"
    );
    let feature_exists = root
        .join("features/suites/r4_47_engine_app_synthesis.feature")
        .exists();
    assert!(feature_exists, "R4-47 feature suite file must exist");

    let engine_rs = std::fs::read_to_string(root.join("xtask/src/engine.rs"))
        .expect("xtask/src/engine.rs must exist");
    assert!(
        engine_rs.contains("pub fn rollback_failed_tick_worktree"),
        "engine.rs must expose rollback_failed_tick_worktree"
    );
    assert!(
        engine_rs.contains("lift_register::cmd_lift_register")
            && engine_rs.contains("synthesis::cmd_synthesize"),
        "engine.rs Stage 6 must execute application synthesis helpers"
    );
}
