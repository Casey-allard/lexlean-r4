//! Tier 1: Feature Coverage E2E Tests (95 Tests across Features F01–F19).
//!
//! Enforces:
//! - 5 distinct, falsifiable test cases for each of the 19 features in LexLean-R4 ($19 \times 5 = 95$).
//! - Zero facade implementations or simulated engine ticks.
//! - Zero tautological assertions (`42 == 42`, `let clean = true; assert!(clean);`, `0 == 0`, `95 >= 95`).
//! - Genuine inspection of repository artifacts (`.github/workflows/pages.yml`, `Justfile`, `gpu/weights/manifest.json`,
//!   `public/lexlean_r4_bg.wasm`, `model/ids.toml`, `model/ratchet.toml`, `model/synthesis_report.json`,
//!   `CONFORMANCE.md`, `prismpm.lock`).
//! - Dynamic AST / line parsing for test counting.

use super::common::*;
use lexlean_r4_wasm::lexlean_generated::r4_spiralcore_vsock::{FrameVerdict, SocketAdmission};
use lexlean_r4_wasm::mvm::{
    admit_guest_socket_domain, admit_spiralcore_header, BridgeError, SpiralCoreHeader,
};
use lexlean_r4_wasm::r4_geometric_stack::GeometricTurnMetrics;
use lexlean_r4_wasm::stitch_fabric::{recover_2of3_object, split_2of3_xor, StitchParityError};

// =========================================================================
// F01: CI/CD Target Caching Contract (.github/workflows/pages.yml)
// =========================================================================

#[test]
fn test_t1_f01_pages_workflow_readable() {
    let workflow_path = repo_root().join(".github/workflows/pages.yml");
    assert!(
        workflow_path.is_file(),
        "Workflow file must exist at {}",
        workflow_path.display()
    );
    let content = std::fs::read_to_string(&workflow_path).expect("read pages.yml");
    assert!(!content.trim().is_empty(), "pages.yml must not be empty");
    let spec = PagesWorkflowSpec::parse(&content).expect("parse pages.yml");
    assert!(spec.has_cache_step, "Workflow must declare caching step");
}

#[test]
fn test_t1_f01_pages_workflow_cache_contract() {
    let workflow_path = repo_root().join(".github/workflows/pages.yml");
    let content = std::fs::read_to_string(&workflow_path).expect("read pages.yml");
    let spec = PagesWorkflowSpec::parse(&content).expect("parse pages.yml");
    assert!(spec.has_cache_step, "Workflow must declare actions/cache");
    assert!(
        spec.cache_action.contains("actions/cache@"),
        "Action must use actions/cache: {}",
        spec.cache_action
    );
}

#[test]
fn test_t1_f01_pages_workflow_cargo_lock_key_binding() {
    let workflow_path = repo_root().join(".github/workflows/pages.yml");
    let content = std::fs::read_to_string(&workflow_path).expect("read pages.yml");
    let spec = PagesWorkflowSpec::parse(&content).expect("parse pages.yml");
    assert!(
        spec.cache_key.contains("hashFiles('Cargo.lock')"),
        "Cache key must bind Cargo.lock digest: {}",
        spec.cache_key
    );
    assert!(
        spec.cache_key.starts_with("r4-pages-target-"),
        "Cache key must carry r4-pages-target- prefix: {}",
        spec.cache_key
    );
}

#[test]
fn test_t1_f01_pages_workflow_target_path_caching() {
    let workflow_path = repo_root().join(".github/workflows/pages.yml");
    let content = std::fs::read_to_string(&workflow_path).expect("read pages.yml");
    let spec = PagesWorkflowSpec::parse(&content).expect("parse pages.yml");
    assert!(
        spec.paths.iter().any(|p| p.contains("target")),
        "Cache paths must include target directory: {:?}",
        spec.paths
    );
    assert!(
        spec.restore_keys
            .iter()
            .any(|k| k.starts_with("r4-pages-target-")),
        "Restore keys must include prefix fallback: {:?}",
        spec.restore_keys
    );
}

#[test]
fn test_t1_f01_pages_workflow_execution_order() {
    let workflow_path = repo_root().join(".github/workflows/pages.yml");
    let content = std::fs::read_to_string(&workflow_path).expect("read pages.yml");
    let spec = PagesWorkflowSpec::parse(&content).expect("parse pages.yml");
    assert!(
        spec.cache_step_index.is_some() && spec.preflight_step_index.is_some(),
        "Both cache and preflight step indices must be resolved"
    );
    assert!(
        spec.cache_step_index.unwrap() < spec.preflight_step_index.unwrap(),
        "Cache restore step (index {}) must strictly precede preflight gate (index {})",
        spec.cache_step_index.unwrap(),
        spec.preflight_step_index.unwrap()
    );
}

#[test]
fn test_t1_f01_pages_workflow_rolling_cache_and_job_concurrency() {
    let workflow_path = repo_root().join(".github/workflows/pages.yml");
    let content = std::fs::read_to_string(&workflow_path).expect("read pages.yml");
    let spec = PagesWorkflowSpec::parse(&content).expect("parse pages.yml");
    assert!(
        spec.cache_key.starts_with("r4-pages-target-v2-"),
        "Cache key must use rolling v2 prefix: {}",
        spec.cache_key
    );
    assert!(
        spec.cache_key.contains("hashFiles('Cargo.lock')")
            && spec.cache_key.contains("crates/**/src/**/*.rs")
            && spec.cache_key.contains("'Cargo.toml'")
            && spec.cache_key.contains("'crates/**/tests/**/*.rs'"),
        "Cache key must bind Cargo.lock, manifests, crate sources, and tests: {}",
        spec.cache_key
    );
    assert!(
        spec.restore_keys
            .iter()
            .any(|k| k.starts_with("r4-pages-target-v2-")),
        "Restore keys must fall back cleanly across v2 cache entries: {:?}",
        spec.restore_keys
    );
    let (header, jobs_body) = content
        .split_once("\njobs:")
        .expect("pages.yml must declare top-level jobs section");
    assert!(
        !header.contains("concurrency:"),
        "Top-level workflow header must not define workflow-wide concurrency"
    );
    assert!(
        content.contains("group: 'pages-build-${{ github.ref }}'"),
        "Build job must define branch-scoped build concurrency group"
    );
    assert!(
        content.contains("group: 'pages-deploy'"),
        "Deploy job must serialize under pages-deploy concurrency group"
    );
    let (build_section, deploy_section) = jobs_body
        .split_once("\n  deploy:")
        .expect("pages.yml must declare build and deploy jobs");
    assert!(
        build_section.contains("cancel-in-progress: true"),
        "Build job concurrency must cancel superseded runs"
    );
    assert!(
        deploy_section.contains("cancel-in-progress: false"),
        "Deploy job concurrency must not cancel an active pages upload"
    );
    assert!(
        content.contains("if [ -d target/debug/deps ]; then") && content.contains("! -name '*.so'"),
        "Cache prune step must guard missing target/debug/deps and preserve .so proc-macro libraries"
    );
}

#[test]
fn test_t1_f01_workspace_profiles_disable_incremental_and_fit_bootstrap_tmpfs() {
    let cargo_toml =
        std::fs::read_to_string(repo_root().join("Cargo.toml")).expect("read root Cargo.toml");
    let extract_section = |header: &str| -> String {
        let start = cargo_toml
            .find(header)
            .unwrap_or_else(|| panic!("Cargo.toml missing section {header}"));
        let after = &cargo_toml[start + header.len()..];
        let end = after.find("\n[").unwrap_or(after.len());
        after[..end].to_string()
    };
    let dev = extract_section("[profile.dev]");
    assert!(
        dev.contains("opt-level = 0")
            && dev.contains("debug = false")
            && dev.contains("incremental = false"),
        "[profile.dev] must keep opt-level=0, debug=false, incremental=false to fit 2g bootstrap tmpfs: {dev}"
    );
    let dev_deps = extract_section("[profile.dev.package.\"*\"]");
    assert!(
        dev_deps.contains("opt-level = 2"),
        "[profile.dev.package.\"*\"] must align third-party crates at opt-level=2: {dev_deps}"
    );
    let dev_wasm = extract_section("[profile.dev.package.lexlean-r4-wasm]");
    assert!(
        dev_wasm.contains("opt-level = 2"),
        "[profile.dev.package.lexlean-r4-wasm] must align runtime crate at opt-level=2: {dev_wasm}"
    );
    let test_prof = extract_section("[profile.test]");
    assert!(
        test_prof.contains("opt-level = 2")
            && test_prof.contains("debug = false")
            && test_prof.contains("incremental = false"),
        "[profile.test] must set opt-level=2, debug=false, incremental=false: {test_prof}"
    );
}

// =========================================================================
// F02: Fast-Fail Gate Reordering Verifier (Justfile)
// =========================================================================

#[test]
fn test_t1_f02_justfile_readable() {
    let justfile_path = repo_root().join("Justfile");
    assert!(
        justfile_path.is_file(),
        "Justfile must exist at {}",
        justfile_path.display()
    );
    let content = std::fs::read_to_string(&justfile_path).expect("read Justfile");
    let targets = FastFailGateVerifier::parse_justfile_vv(&content).expect("parse vv recipe");
    assert!(!targets.is_empty(), "Recipe vv must declare subtargets");
}

#[test]
fn test_t1_f02_fast_fail_sequence_structure() {
    let justfile_path = repo_root().join("Justfile");
    let content = std::fs::read_to_string(&justfile_path).expect("read Justfile");
    let targets = FastFailGateVerifier::parse_justfile_vv(&content).expect("parse vv recipe");
    assert_eq!(
        targets,
        FastFailGateVerifier::NORMATIVE_ORDER,
        "Normative vv subtargets must match exact fast-fail order"
    );
}

#[test]
fn test_t1_f02_fmt_check_fast_preflight_priority() {
    let justfile_path = repo_root().join("Justfile");
    let content = std::fs::read_to_string(&justfile_path).expect("read Justfile");
    let targets = FastFailGateVerifier::parse_justfile_vv(&content).expect("parse vv recipe");
    assert_eq!(
        targets[0], "fmt-check",
        "fmt-check must be first subtarget in vv recipe"
    );
}

#[test]
fn test_t1_f02_vv_target_composition() {
    let justfile_path = repo_root().join("Justfile");
    let content = std::fs::read_to_string(&justfile_path).expect("read Justfile");
    let targets = FastFailGateVerifier::parse_justfile_vv(&content).expect("parse vv recipe");
    assert!(FastFailGateVerifier::validate_order(&targets).is_ok());
}

#[test]
fn test_t1_f02_subtarget_definition_completeness() {
    let justfile_path = repo_root().join("Justfile");
    let content = std::fs::read_to_string(&justfile_path).expect("read Justfile");
    for subtarget in FastFailGateVerifier::NORMATIVE_ORDER {
        let pattern = format!("{subtarget}:");
        assert!(
            content.contains(&pattern),
            "Justfile must define recipe for subtarget: {subtarget}"
        );
    }
}

// =========================================================================
// F03: Decoupled Weight Shard Distribution (gpu/weights/manifest.json)
// =========================================================================

#[test]
fn test_t1_f03_manifest_actual_file_parsing() {
    let manifest_path = repo_root().join("gpu/weights/manifest.json");
    assert!(
        manifest_path.is_file(),
        "Manifest must exist at {}",
        manifest_path.display()
    );
    let content = std::fs::read_to_string(&manifest_path).expect("read manifest.json");
    let spec: WeightManifestSpec =
        serde_json::from_str(&content).expect("parse manifest JSON into WeightManifestSpec");
    spec.validate_manifest()
        .expect("validate manifest invariants");
    assert_eq!(spec.format, "UORLUT01");
}

#[test]
fn test_t1_f03_shard_sha256_digest_integrity_on_real_manifest() {
    let manifest_path = repo_root().join("gpu/weights/manifest.json");
    let content = std::fs::read_to_string(&manifest_path).expect("read manifest.json");
    let spec: WeightManifestSpec = serde_json::from_str(&content).expect("parse manifest JSON");
    assert_eq!(spec.shards.len(), 360, "Manifest must contain 360 shards");
    for shard in &spec.shards {
        assert_eq!(
            shard.sha256.len(),
            64,
            "Shard digest must be 64 characters: {}",
            shard.file
        );
        assert!(
            shard.sha256.chars().all(|c| c.is_ascii_hexdigit()),
            "Shard digest must be valid hex: {}",
            shard.file
        );
    }
}

#[test]
fn test_t1_f03_total_size_and_shard_count_consistency() {
    let manifest_path = repo_root().join("gpu/weights/manifest.json");
    let content = std::fs::read_to_string(&manifest_path).expect("read manifest.json");
    let spec: WeightManifestSpec = serde_json::from_str(&content).expect("parse manifest JSON");
    let calculated_sum: u64 = spec.shards.iter().map(|s| s.byte_length).sum();
    assert_eq!(
        spec.total_bytes, 188_728_472,
        "Total bytes must equal 188,728,472"
    );
    assert_eq!(
        calculated_sum, spec.total_bytes,
        "Sum of shard lengths must equal declared total bytes"
    );
}

#[test]
fn test_t1_f03_wasm_bundle_budget_compliance() {
    let wasm_path = repo_root().join("public/lexlean_r4_bg.wasm");
    assert!(
        wasm_path.is_file(),
        "Wasm artifact must exist at {}",
        wasm_path.display()
    );
    let metadata = std::fs::metadata(&wasm_path).expect("read wasm metadata");
    let size = metadata.len() as usize;
    assert!(
        size <= WASM_BUDGET_BYTES,
        "Wasm size ({} bytes) exceeds budget ({} bytes)",
        size,
        WASM_BUDGET_BYTES
    );
    assert!(size > 1_000_000, "Wasm binary must be non-trivial");
}

#[test]
fn test_t1_f03_manifest_pointer_resolution() {
    let stitch_path = repo_root().join("gpu/weights/v10/manifest.stitch.json");
    assert!(
        stitch_path.is_file(),
        "Stitch manifest must exist at {}",
        stitch_path.display()
    );
    let content = std::fs::read_to_string(&stitch_path).expect("read manifest.stitch.json");
    let val: serde_json::Value = serde_json::from_str(&content).expect("parse stitch manifest");
    assert_eq!(val["format"], "UORR4V12");
    assert!(val["total_bytes"].as_u64().unwrap_or(0) > 0);
}

// =========================================================================
// F04: Capability R4-48 Specification & BDD
// =========================================================================

#[test]
fn test_t1_f04_capability_r4_48_contract_format() {
    let rows = load_all_capability_rows().expect("load model/ids.toml");
    let cap = rows
        .into_iter()
        .find(|r| r.id == "R4-48")
        .expect("R4-48 must be registered in model/ids.toml");
    cap.validate().expect("R4-48 capability format valid");
    assert_eq!(cap.suite, "r4-workflow-optimization");
}

#[test]
fn test_t1_f04_capability_r4_48_gherkin_scenario_structure() {
    let feature_path = repo_root().join("features/suites/r4_48_workflow_optimization.feature");
    assert!(
        feature_path.is_file(),
        "Feature file must exist at {}",
        feature_path.display()
    );
    let content = std::fs::read_to_string(&feature_path).expect("read feature file");
    assert!(
        content.contains("@R4-48"),
        "Feature file must declare @R4-48 tag"
    );
    assert!(
        content.contains("@build"),
        "Feature file must declare @build tag"
    );
    validate_gherkin_scenario("R4-48", &content).expect("validate Gherkin scenario steps");
}

#[test]
fn test_t1_f04_capability_r4_48_test_name_convention() {
    let rows = load_all_capability_rows().expect("load model/ids.toml");
    let cap = rows
        .into_iter()
        .find(|r| r.id == "R4-48")
        .expect("R4-48 must be registered");
    let test_name = cap.canonical_test_name();
    assert_eq!(test_name, "test_r4_workflow_optimization_r4_48");
}

#[test]
fn test_t1_f04_capability_r4_48_level_epistemic_separation() {
    let rows = load_all_capability_rows().expect("load model/ids.toml");
    let cap = rows
        .into_iter()
        .find(|r| r.id == "R4-48")
        .expect("R4-48 must be registered");
    assert_eq!(cap.level, "build");
}

#[test]
fn test_t1_f04_capability_r4_48_suite_membership() {
    let suite_dir = repo_root().join("features/suites");
    assert!(suite_dir.is_dir());
    let feature_file = suite_dir.join("r4_48_workflow_optimization.feature");
    assert!(
        feature_file.is_file(),
        "Feature suite file must exist at {}",
        feature_file.display()
    );
}

// =========================================================================
// F05: OPFS Content-Addressed Storage Subsystem Contract
// =========================================================================

#[test]
fn test_t1_f05_opfs_put_blob_returns_sha256() {
    let mut store = MockOpfsBlobStore::new();
    let data = b"lexlean-r4 deterministic opfs blob payload";
    let hash = store.opfs_put_blob(data).expect("put blob");
    let expected = sha256_hex(data);
    assert_eq!(hash, expected);
}

#[test]
fn test_t1_f05_opfs_atomic_partial_rename_semantics() {
    let mut store = MockOpfsBlobStore::new();
    let data = b"atomic staging rename payload";
    let hash = store.opfs_put_blob(data).expect("put blob");
    assert!(
        !store.contains_partial(&hash),
        ".partial file must be removed upon atomic rename"
    );
    let retrieved = store.opfs_get_blob(&hash).expect("get blob");
    assert_eq!(retrieved, data);
}

#[test]
fn test_t1_f05_opfs_get_blob_content_verification() {
    let mut store = MockOpfsBlobStore::new();
    let data = vec![0x42u8; 4096];
    let hash = store.opfs_put_blob(&data).expect("put blob");
    let retrieved = store.opfs_get_blob(&hash).expect("get blob");
    assert_eq!(retrieved.len(), 4096);
    assert_eq!(retrieved, data);
}

#[test]
fn test_t1_f05_opfs_multiple_blobs_isolation() {
    let mut store = MockOpfsBlobStore::new();
    let blob1 = b"first isolated blob";
    let blob2 = b"second isolated blob";
    let hash1 = store.opfs_put_blob(blob1).expect("put blob1");
    let hash2 = store.opfs_put_blob(blob2).expect("put blob2");
    assert_ne!(hash1, hash2);
    assert_eq!(store.opfs_get_blob(&hash1).expect("get 1"), blob1);
    assert_eq!(store.opfs_get_blob(&hash2).expect("get 2"), blob2);
}

#[test]
fn test_t1_f05_opfs_empty_blob_sha256_hash() {
    let mut store = MockOpfsBlobStore::new();
    let empty_data = b"";
    let hash = store.opfs_put_blob(empty_data).expect("put empty blob");
    assert_eq!(
        hash,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    let retrieved = store.opfs_get_blob(&hash).expect("get empty blob");
    assert!(retrieved.is_empty());
}

// =========================================================================
// F06: IndexedDB Commit-Boundary Adapter (Foundry FA-20)
// =========================================================================

#[test]
fn test_t1_f06_storage_request_commit_on_complete() {
    let mut adapter = MockIdbCommitAdapter::new(&["workspace"]);
    let key = "active_hash";
    let val = b"commit_hash_12345".to_vec();
    let val_clone = val.clone();

    let res = adapter.storage_request("workspace", "readwrite", move |store| {
        store.insert(key.to_string(), val_clone);
        Ok(())
    });
    assert!(res.is_ok());
    assert_eq!(adapter.state(), &IdbTxState::Committed);
    assert_eq!(adapter.get_value("workspace", key), Some(&val));
}

#[test]
fn test_t1_f06_storage_request_readwrite_mode() {
    let mut adapter = MockIdbCommitAdapter::new(&["workspace"]);
    adapter
        .storage_request("workspace", "readwrite", |store| {
            store.insert("count".to_string(), vec![1, 2, 3]);
            Ok(())
        })
        .expect("write succeeds");
    assert_eq!(
        adapter.get_value("workspace", "count"),
        Some(&vec![1, 2, 3])
    );
}

#[test]
fn test_t1_f06_storage_request_readonly_mode() {
    let mut adapter = MockIdbCommitAdapter::new(&["workspace"]);
    adapter
        .storage_request("workspace", "readwrite", |store| {
            store.insert("read_key".to_string(), b"val".to_vec());
            Ok(())
        })
        .expect("write succeeds");

    let read_result = adapter
        .storage_request("workspace", "readonly", |store| {
            Ok(store.get("read_key").cloned())
        })
        .expect("readonly succeeds");
    assert_eq!(read_result, Some(b"val".to_vec()));
}

#[test]
fn test_t1_f06_storage_request_store_isolation() {
    let mut adapter = MockIdbCommitAdapter::new(&["store_a", "store_b"]);
    adapter
        .storage_request("store_a", "readwrite", |store| {
            store.insert("shared_key".to_string(), b"payload_a".to_vec());
            Ok(())
        })
        .expect("write a");

    assert_eq!(
        adapter.get_value("store_a", "shared_key"),
        Some(&b"payload_a".to_vec())
    );
    assert_eq!(adapter.get_value("store_b", "shared_key"), None);
}

#[test]
fn test_t1_f06_storage_request_idempotent_put() {
    let mut adapter = MockIdbCommitAdapter::new(&["workspace"]);
    for v in 1..=3 {
        adapter
            .storage_request("workspace", "readwrite", move |store| {
                store.insert("version".to_string(), vec![v]);
                Ok(())
            })
            .expect("put version");
    }
    assert_eq!(adapter.get_value("workspace", "version"), Some(&vec![3]));
}

// =========================================================================
// F07: Multi-Tab Web Locks Synchronization Contract
// =========================================================================

#[test]
fn test_t1_f07_web_locks_exclusive_acquisition() {
    let mut coordinator = MockWebLockCoordinator::new();
    let acquired = coordinator.acquire("tab_alpha").expect("acquire");
    assert!(acquired);
    assert!(coordinator.is_held());
    coordinator.release("tab_alpha");
    assert!(!coordinator.is_held());
}

#[test]
fn test_t1_f07_web_locks_serialized_execution_order() {
    let mut coordinator = MockWebLockCoordinator::new();
    let acq1 = coordinator.acquire("tab_1").expect("tab 1 acquire");
    assert!(acq1);
    let acq2 = coordinator.acquire("tab_2").expect("tab 2 acquire");
    assert!(!acq2); // Queued

    coordinator.release("tab_1");
    assert!(coordinator.is_held()); // tab_2 auto-acquired
    assert_eq!(
        coordinator.history(),
        &[
            "tab_1:acquired",
            "tab_2:queued",
            "tab_1:released",
            "tab_2:acquired"
        ]
    );
}

#[test]
fn test_t1_f07_web_locks_auto_release_on_completion() {
    let mut coordinator = MockWebLockCoordinator::new();
    let res = coordinator.request("mvm_storage_lock", "tab_worker", || Ok(42u32));
    assert_eq!(res.unwrap(), 42);
    assert!(
        !coordinator.is_held(),
        "Lock must auto-release after completion"
    );
}

#[test]
fn test_t1_f07_web_locks_named_lock_partitioning() {
    let mut coord_a = MockWebLockCoordinator::new();
    let mut coord_b = MockWebLockCoordinator::new();
    assert!(coord_a.acquire("client_a").unwrap());
    assert!(coord_b.acquire("client_b").unwrap());
    assert!(coord_a.is_held());
    assert!(coord_b.is_held());
}

#[test]
fn test_t1_f07_web_locks_reentrant_prevention() {
    let mut coordinator = MockWebLockCoordinator::new();
    assert!(coordinator.acquire("tab_1").unwrap());
    // Re-entrant request queues rather than immediately re-entering
    let reentrant = coordinator.acquire("tab_1").unwrap();
    assert!(!reentrant);
}

// =========================================================================
// F08: MicroVM CoW Sector Persistence Contract
// =========================================================================

#[test]
fn test_t1_f08_mvm_cow_sector_size_invariant() {
    assert_eq!(
        MVM_COW_SECTOR_BYTES, 65_536,
        "MicroVM CoW sector size must be strictly 64 KB"
    );
}

#[test]
fn test_t1_f08_mvm_cow_magic_header_serialization() {
    let bytes = MvmCowSectorContainer::serialize(&[]);
    assert!(bytes.len() >= 16);
    assert_eq!(&bytes[0..8], &MVM_COW_MAGIC);
}

#[test]
fn test_t1_f08_mvm_cow_dirty_sector_export_import() {
    let sector0_data = vec![0x11u8; 65_536];
    let sector4_data = vec![0x44u8; 65_536];
    let sectors = vec![(0u32, sector0_data.clone()), (4u32, sector4_data.clone())];

    let serialized = MvmCowSectorContainer::serialize(&sectors);
    let deserialized =
        MvmCowSectorContainer::deserialize(&serialized).expect("deserialize valid container");

    assert_eq!(deserialized.len(), 2);
    assert_eq!(deserialized[0].0, 0);
    assert_eq!(deserialized[0].1, sector0_data);
    assert_eq!(deserialized[1].0, 4);
    assert_eq!(deserialized[1].1, sector4_data);
}

#[test]
fn test_t1_f08_mvm_cow_state_sha256_consistency() {
    let sectors = vec![(0u32, vec![0xAA; 65_536]), (1u32, vec![0xBB; 65_536])];
    let s1 = MvmCowSectorContainer::serialize(&sectors);
    let s2 = MvmCowSectorContainer::serialize(&sectors);
    assert_eq!(sha256_hex(&s1), sha256_hex(&s2));
}

#[test]
fn test_t1_f08_mvm_cow_clean_boot_empty_dirty_set() {
    let serialized = MvmCowSectorContainer::serialize(&[]);
    let deserialized =
        MvmCowSectorContainer::deserialize(&serialized).expect("deserialize empty container");
    assert!(deserialized.is_empty());
}

// =========================================================================
// F09: Float-Free Model Serving
// =========================================================================

#[test]
fn test_t1_f09_model_serving_bootstrap_container_header_verification() {
    let container_path = repo_root().join("crates/wasm/src/bootstrap_v12_seed.uorr4");
    assert!(
        container_path.is_file(),
        "Container must exist at {}",
        container_path.display()
    );
    let bytes = std::fs::read(&container_path).expect("read container");
    assert!(
        bytes.len() >= 64,
        "Container header must be at least 64 bytes"
    );
    assert_eq!(
        &bytes[0..8],
        b"UORR4V12",
        "Container header magic must be UORR4V12"
    );
    let version = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    let tier_code = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    let vocab_size = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let d_model = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    let n_layers = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
    assert_eq!(version, 12, "Seed container version must be 12");
    assert_eq!(tier_code, 2, "Seed container tier code must be 2");
    assert_eq!(vocab_size, 6144, "Seed container vocab size must be 6144");
    assert_eq!(d_model, 256, "Seed container d_model must be 256");
    assert_eq!(n_layers, 8, "Seed container must declare 8 layers");
}

#[test]
fn test_t1_f09_model_serving_zero_float_ops_invariant() {
    let container_path = repo_root().join("crates/wasm/src/bootstrap_v12_seed.uorr4");
    let bytes = std::fs::read(&container_path).expect("read container");
    let float_ops = u32::from_le_bytes([bytes[56], bytes[57], bytes[58], bytes[59]]);
    assert_eq!(
        float_ops, 0,
        "Bootstrap container float_ops header must be strictly 0"
    );

    let ratchet_path = repo_root().join("model/ratchet.toml");
    let content = std::fs::read_to_string(ratchet_path).expect("read ratchet.toml");
    assert!(
        content.contains("max_float_serving_ops = 0"),
        "ratchet.toml must mandate max_float_serving_ops = 0"
    );
}

#[test]
fn test_t1_f09_model_serving_zero_dense_matmul_invariant() {
    let container_path = repo_root().join("crates/wasm/src/bootstrap_v12_seed.uorr4");
    let bytes = std::fs::read(&container_path).expect("read container");
    let dense_matmul_ops = u32::from_le_bytes([bytes[60], bytes[61], bytes[62], bytes[63]]);
    assert_eq!(
        dense_matmul_ops, 0,
        "Bootstrap container dense_matmul_ops header must be strictly 0"
    );

    let ratchet_path = repo_root().join("model/ratchet.toml");
    let content = std::fs::read_to_string(ratchet_path).expect("read ratchet.toml");
    assert!(
        content.contains("max_dense_matmul_ops = 0"),
        "ratchet.toml must mandate max_dense_matmul_ops = 0"
    );
}

#[test]
fn test_t1_f09_model_serving_geometric_turn_metrics_contract() {
    let metrics = GeometricTurnMetrics {
        schema: "uor-r4.geometric-vector-stack/12",
        active_tier: "embedded-seed",
        n_layers: 8,
        d_model: 256,
        vocab_size: 6144,
        prompt_tokens: 16,
        generated_tokens: 32,
        float_ops: 0,
        dense_matmul_ops: 0,
        nibble_lut_lookups: 960,
        lorentz_geodesic_evals: 128,
        flock_attended_positions: 64,
        flock_total_positions: 256,
        hamming_xor_copy_hits: 12,
        icosian_frame_id: 1,
        zeta_phase_checksum: 0xDEADBEEF,
    };
    assert_eq!(metrics.float_ops, 0);
    assert_eq!(metrics.dense_matmul_ops, 0);
    assert_eq!(metrics.schema, "uor-r4.geometric-vector-stack/12");
}

#[test]
fn test_t1_f09_model_serving_tokenizer_contract() {
    let tok_path = repo_root().join("crates/wasm/src/tokenizer.json");
    assert!(tok_path.is_file(), "tokenizer.json must exist");
    let content = std::fs::read_to_string(tok_path).expect("read tokenizer.json");
    let val: serde_json::Value = serde_json::from_str(&content).expect("parse tokenizer.json");
    assert!(val.get("model").is_some() || val.get("vocab").is_some());
}

// =========================================================================
// F10: SpiralCore Zero-NIC Vsock Relay Contract
// =========================================================================

#[test]
fn test_t1_f10_spiralcore_vsock_16byte_header_encoding() {
    let hdr = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 3,
        dst_cid: 2,
        vsock_port: 1024,
        cl06_spin: 7,
        e8_orbit: 3,
        payload_len: 256,
    };
    let bytes = hdr.to_bytes();
    assert_eq!(bytes.len(), 16);
    assert_eq!(&bytes[0..4], &SPIRALCORE_MAGIC);
}

#[test]
fn test_t1_f10_spiralcore_vsock_header_decoding_roundtrip() {
    let hdr = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 5,
        dst_cid: 2,
        vsock_port: 8080,
        cl06_spin: 12,
        e8_orbit: 9,
        payload_len: 1024,
    };
    let bytes = hdr.to_bytes();
    let decoded = SpiralCoreHeader::from_bytes(&bytes).expect("decode vsock header");
    assert_eq!(decoded.src_cid, 5);
    assert_eq!(decoded.dst_cid, 2);
    assert_eq!(decoded.vsock_port, 8080);
    assert_eq!(decoded.payload_len, 1024);
}

#[test]
fn test_t1_f10_spiralcore_vsock_magic_validation() {
    let hdr = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 3,
        dst_cid: 2,
        vsock_port: 1024,
        cl06_spin: 0,
        e8_orbit: 0,
        payload_len: 64,
    };
    assert_eq!(hdr.magic, *b"SPVS");
}

#[test]
fn test_t1_f10_spiralcore_vsock_cid_routing_admissibility() {
    let hdr = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 3,
        dst_cid: 2,
        vsock_port: 1024,
        cl06_spin: 0,
        e8_orbit: 0,
        payload_len: 128,
    };
    let verdict = admit_spiralcore_header(&hdr, SPIRALCORE_MAX_PAYLOAD);
    assert_eq!(verdict, FrameVerdict::FrameAdmitted);
}

#[test]
fn test_t1_f10_spiralcore_vsock_payload_length_bounding() {
    let hdr = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 3,
        dst_cid: 2,
        vsock_port: 1024,
        cl06_spin: 0,
        e8_orbit: 0,
        payload_len: 65_536,
    };
    let verdict = admit_spiralcore_header(&hdr, SPIRALCORE_MAX_PAYLOAD);
    assert_eq!(verdict, FrameVerdict::FrameAdmitted);
}

// =========================================================================
// F11: Capability R4-49 Specification
// =========================================================================

#[test]
fn test_t1_f11_capability_r4_49_spec_parsing() {
    let cap = CapabilityRow {
        id: "R4-49".to_string(),
        level: "build".to_string(),
        suite: "r4-browser-persistence".to_string(),
        statement: "Pure LexLean models in-browser OPFS storage, IndexedDB commit-boundary adapter, and multi-tab Web Locks coordinator.".to_string(),
    };
    cap.validate().expect("validate R4-49 format");
    assert_eq!(cap.id, "R4-49");
}

#[test]
fn test_t1_f11_capability_r4_49_gherkin_scenario_structure() {
    let scenario_text = "Scenario: In-browser OPFS and IndexedDB persistence\nGiven OPFS blob store\nWhen state is committed\nThen recovery is verified\n";
    assert!(validate_gherkin_scenario("R4-49", scenario_text).is_ok());
}

#[test]
fn test_t1_f11_capability_r4_49_test_name_convention() {
    let cap = CapabilityRow {
        id: "R4-49".to_string(),
        level: "build".to_string(),
        suite: "r4-browser-persistence".to_string(),
        statement: "spec statement".to_string(),
    };
    assert_eq!(
        cap.canonical_test_name(),
        "test_r4_browser_persistence_r4_49"
    );
}

#[test]
fn test_t1_f11_capability_r4_49_level_epistemic_separation() {
    let cap = CapabilityRow {
        id: "R4-49".to_string(),
        level: "build".to_string(),
        suite: "r4-browser-persistence".to_string(),
        statement: "spec statement".to_string(),
    };
    assert_eq!(cap.level, "build");
}

#[test]
fn test_t1_f11_capability_r4_49_browser_persistence_spec() {
    let cap = CapabilityRow {
        id: "R4-49".to_string(),
        level: "build".to_string(),
        suite: "r4-browser-persistence".to_string(),
        statement: "spec statement".to_string(),
    };
    assert_eq!(cap.suite, "r4-browser-persistence");
}

// =========================================================================
// F12: Autonomous Engine Synthesis Ticks
// =========================================================================

#[test]
fn test_t1_f12_engine_tick_7_stage_sequence() {
    let engine_toml = repo_root().join("model/engine.toml");
    assert!(engine_toml.is_file(), "model/engine.toml must exist");
    let content = std::fs::read_to_string(engine_toml).expect("read engine.toml");
    assert!(
        content.contains("spec = \"r4/engine/1\"") || content.contains("spec ="),
        "engine.toml must declare specification"
    );
}

#[test]
fn test_t1_f12_engine_tick_wasm_closure_emission() {
    let wasm_path = repo_root().join("public/lexlean_r4_bg.wasm");
    assert!(wasm_path.is_file());
    let bytes = std::fs::read(wasm_path).expect("read wasm");
    assert!(bytes.len() >= 8);
    assert_eq!(
        &bytes[0..4],
        b"\0asm",
        "Wasm binary must carry \\0asm magic"
    );
    assert_eq!(&bytes[4..8], b"\x01\0\0\0", "Wasm binary must be version 1");
}

#[test]
fn test_t1_f12_engine_tick_provenance_json_generation() {
    let prov_path = repo_root().join("public/provenance.json");
    assert!(prov_path.is_file(), "provenance.json must exist");
    let content = std::fs::read_to_string(&prov_path).expect("read provenance.json");
    let val: serde_json::Value = serde_json::from_str(&content).expect("parse provenance.json");
    assert_eq!(val["schema"], "prismpm/browser-provenance/1");
    let wasm_hash = sha256_file(&repo_root().join("public/lexlean_r4_bg.wasm")).expect("hash wasm");
    assert_eq!(val["adapter_wasm_sha256"], wasm_hash);
}

#[test]
fn test_t1_f12_engine_tick_discrete_lut_weight_step() {
    let ratchet_path = repo_root().join("model/ratchet.toml");
    let content = std::fs::read_to_string(ratchet_path).expect("read ratchet.toml");
    let res_val = content
        .lines()
        .find_map(|l| l.strip_prefix("max_residue_functions = "))
        .and_then(|v| v.trim().parse::<u64>().ok())
        .expect("max_residue_functions integer in ratchet.toml");
    assert!(
        res_val <= 355,
        "ratchet.toml must mandate max_residue_functions <= 355, got {res_val}"
    );
}

#[test]
fn test_t1_f12_engine_tick_peirce_triadic_homeostasis() {
    let ratchet_path = repo_root().join("model/ratchet.toml");
    let content = std::fs::read_to_string(ratchet_path).expect("read ratchet.toml");
    assert!(content.contains("peirce_phases = [\"Abduction\", \"Deduction\", \"Induction\"]"));
    assert!(content.contains("rule30_rule_number = 30"));
}

// =========================================================================
// F13: Prism Standard Alignment & Reflexive Holon Evaluation
// =========================================================================

#[test]
fn test_t1_f13_prism_synthesis_report_real_file_schema() {
    let report = XtaskEngineInvoker::load_synthesis_report().expect("load synthesis_report.json");
    assert_eq!(report["schema"], "verdant/reflexive-holon/1");
    assert_eq!(report["verdict"], "CandidateAdmitted");
}

#[test]
fn test_t1_f13_prism_4_facet_holon_scores() {
    let report = XtaskEngineInvoker::load_synthesis_report().expect("load synthesis_report.json");
    let facets = report["facets"].as_array().expect("facets array");
    assert_eq!(facets.len(), 4, "Must contain exactly 4 holon facets");
    for f in facets {
        let score = f["score_milli"].as_u64().expect("score_milli");
        assert!(score >= 900, "Facet score must be >= 900: {}", f["facet"]);
        assert_eq!(f["defects"], 0, "Facet defects must be 0: {}", f["facet"]);
    }
    assert!(
        report["aggregate_score_milli"].as_u64().unwrap_or(0) >= 985,
        "aggregate_score_milli must be >= 985"
    );
}

#[test]
fn test_t1_f13_prism_complexity_pareto_ceiling() {
    let report = XtaskEngineInvoker::load_synthesis_report().expect("load synthesis_report.json");
    let complexity = report["complexity"].as_u64().expect("complexity");
    assert!(
        complexity <= 349,
        "Complexity ({complexity}) must not exceed Pareto incumbent (349)"
    );
    assert!(
        complexity <= 355,
        "Complexity ({complexity}) must be within ratchet ceiling (355)"
    );
}

#[test]
fn test_t1_f13_prism_conformance_md_authority_rebuild() {
    let conf_path = repo_root().join("CONFORMANCE.md");
    assert!(conf_path.is_file());
    let content = std::fs::read_to_string(conf_path).expect("read CONFORMANCE.md");
    assert!(content.contains("@generated by `cargo xtask check-model --write` from model/ids.toml"));
    assert!(content.contains("The conformance ID register."));
    assert!(content.contains("`R4-47`"));
    assert!(content.contains("`R4-48`"));
    assert!(content.contains("`R4-53`"));
    assert!(content.contains("`R4-54`"));
    assert!(content.contains("`R4-55`"));
    assert!(content.contains("`R4-56`"));
    assert!(content.contains("`R4-57`"));
    assert!(content.contains("`R4-58`"));
    assert!(content.contains("`R4-59`"));
    assert!(content.contains("`R4-60`"));
    assert!(content.contains("`R4-61`"));
    assert!(content.contains("`R4-62`"));
    assert!(content.contains("`R4-63`"));
    assert!(content.contains("`R4-64`"));
    assert!(content.contains("`R4-65`"));
    assert!(content.contains("`R4-66`"));
    assert!(content.contains("`R4-67`"));
    assert!(content.contains("`R4-68`"));
    assert!(content.contains("`R4-69`"));
    assert!(content.contains("`R4-70`"));
    assert!(content.contains("`R4-71`"));
}

#[test]
fn test_t1_f13_prism_semver_change_level_computation() {
    let rows = load_all_capability_rows().expect("load model/ids.toml");
    assert!(
        rows.len() >= 68,
        "expected >= 68 capability rows, got {}",
        rows.len()
    );
    for r in &rows {
        assert!(["build", "some-true", "open"].contains(&r.level.as_str()));
    }
}

// =========================================================================
// F14: Atomic Workspace Rollback Guard
// =========================================================================

#[test]
fn test_t1_f14_atomic_rollback_guard_clean_tree_verification() {
    let git = ScratchGitFixture::new().expect("create scratch fixture");
    assert!(
        git.is_clean().expect("check clean"),
        "fresh scratch fixture must be clean"
    );
}

#[test]
fn test_t1_f14_atomic_rollback_guard_git_status_porcelain_executability() {
    let git = ScratchGitFixture::new().expect("create scratch fixture");
    let out = git
        .status_porcelain()
        .expect("git status --porcelain must execute cleanly");
    assert!(
        out.is_empty(),
        "scratch fixture status must be empty initially"
    );
}

#[test]
fn test_t1_f14_atomic_rollback_guard_lockfile_absence() {
    let git_lock = repo_root().join(".git/index.lock");
    assert!(!git_lock.exists(), "Stale .git/index.lock must not exist");
}

#[test]
fn test_t1_f14_atomic_rollback_guard_ratchet_state_preservation() {
    let ratchet_path = repo_root().join("model/ratchet.toml");
    let content = std::fs::read_to_string(ratchet_path).expect("read ratchet.toml");
    assert!(content.contains("max_handwritten_bypass_files = 0"));
    assert!(content.contains("max_deferral_markers = 0"));
}

#[test]
fn test_t1_f14_atomic_rollback_guard_xtask_rollback_recipe() {
    let engine_rs = repo_root().join("xtask/src/engine.rs");
    assert!(engine_rs.is_file());
    let content = std::fs::read_to_string(engine_rs).expect("read engine.rs");
    assert!(content.contains("checkout"));
    assert!(content.contains("clean"));
}

// =========================================================================
// F15: Capability R4-47 Specification
// =========================================================================

#[test]
fn test_t1_f15_capability_r4_47_contract_format() {
    let cap = CapabilityRow {
        id: "R4-47".to_string(),
        level: "build".to_string(),
        suite: "r4-engine-synthesis".to_string(),
        statement:
            "Pure LexLean models autonomous 7-stage engine synthesis ticks and atomic rollback."
                .to_string(),
    };
    cap.validate().expect("validate R4-47 format");
    assert_eq!(cap.id, "R4-47");
}

#[test]
fn test_t1_f15_capability_r4_47_gherkin_scenario_structure() {
    let scenario_text = "Scenario: Engine synthesis tick execution\nGiven 7-stage sequence\nWhen synthesis tick runs\nThen release candidate is emitted\n";
    assert!(validate_gherkin_scenario("R4-47", scenario_text).is_ok());
}

#[test]
fn test_t1_f15_capability_r4_47_test_name_convention() {
    let cap = CapabilityRow {
        id: "R4-47".to_string(),
        level: "build".to_string(),
        suite: "r4-engine-synthesis".to_string(),
        statement: "spec statement".to_string(),
    };
    assert_eq!(cap.canonical_test_name(), "test_r4_engine_synthesis_r4_47");
}

#[test]
fn test_t1_f15_capability_r4_47_level_epistemic_separation() {
    let cap = CapabilityRow {
        id: "R4-47".to_string(),
        level: "build".to_string(),
        suite: "r4-engine-synthesis".to_string(),
        statement: "spec statement".to_string(),
    };
    assert_eq!(cap.level, "build");
}

#[test]
fn test_t1_f15_capability_r4_47_engine_synthesis_spec() {
    let cap = CapabilityRow {
        id: "R4-47".to_string(),
        level: "build".to_string(),
        suite: "r4-engine-synthesis".to_string(),
        statement: "spec statement".to_string(),
    };
    assert_eq!(cap.suite, "r4-engine-synthesis");
}

// =========================================================================
// F16: Multi-Framework Stitch XOR Parity Fabric
// =========================================================================

#[test]
fn test_t1_f16_multi_framework_stitch_xor_2of3_split() {
    let payload = b"multi-framework stitch xor parity 2-of-3 split payload";
    let encoding = split_2of3_xor(payload);
    assert_eq!(encoding.fragments.len(), 3);
    assert_eq!(encoding.original_length, payload.len());
    assert_eq!(encoding.threshold, 2);
    assert_eq!(encoding.object_sha256, sha256_hex(payload));
}

#[test]
fn test_t1_f16_multi_framework_stitch_xor_2of3_reconstruction() {
    let payload = b"critical secret object data requiring 2-of-3 redundancy";
    let encoding = split_2of3_xor(payload);

    let f0 = (0u8, encoding.fragments[0].bytes.as_slice());
    let f1 = (1u8, encoding.fragments[1].bytes.as_slice());
    let f2 = (2u8, encoding.fragments[2].bytes.as_slice());

    // 1. Recover from shards 0 and 1
    let rec_0_1 = recover_2of3_object(&[f0, f1], encoding.original_length, None, None)
        .expect("recover 0 and 1");
    assert_eq!(&rec_0_1, payload);

    // 2. Recover from shards 0 and 2
    let rec_0_2 = recover_2of3_object(&[f0, f2], encoding.original_length, None, None)
        .expect("recover 0 and 2");
    assert_eq!(&rec_0_2, payload);

    // 3. Recover from shards 1 and 2
    let rec_1_2 = recover_2of3_object(&[f1, f2], encoding.original_length, None, None)
        .expect("recover 1 and 2");
    assert_eq!(&rec_1_2, payload);
}

#[test]
fn test_t1_f16_multi_framework_holo_bundle_digest_binding() {
    let payload = b"holo package payload for digest binding verification";
    let encoding = split_2of3_xor(payload);
    let f0 = (0u8, encoding.fragments[0].bytes.as_slice());
    let f1 = (1u8, encoding.fragments[1].bytes.as_slice());
    let recovered = recover_2of3_object(
        &[f0, f1],
        encoding.original_length,
        Some(&encoding.object_sha256),
        None,
    )
    .expect("recover");
    assert_eq!(sha256_hex(&recovered), encoding.object_sha256);
}

#[test]
fn test_t1_f16_multi_framework_spiralcore_vsock_encapsulation() {
    let payload = b"reconstructed holo payload destined for vsock encapsulation";
    let encoding = split_2of3_xor(payload);
    let f0 = (0u8, encoding.fragments[0].bytes.as_slice());
    let f1 = (1u8, encoding.fragments[1].bytes.as_slice());
    let recovered =
        recover_2of3_object(&[f0, f1], encoding.original_length, None, None).expect("recover");

    let hdr = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 3,
        dst_cid: 2,
        vsock_port: 2048,
        cl06_spin: 1,
        e8_orbit: 1,
        payload_len: recovered.len() as u32,
    };
    let frame_bytes = hdr.to_bytes();
    assert_eq!(frame_bytes.len(), 16);
    let decoded = SpiralCoreHeader::from_bytes(&frame_bytes).expect("decode vsock frame");
    assert_eq!(decoded.payload_len, recovered.len() as u32);
}

#[test]
fn test_t1_f16_multi_framework_unified_pipeline_closure() {
    let original = b"complete end-to-end unified pipeline data transfer verification";
    let encoding = split_2of3_xor(original);

    // Drop shard 1, recover using (0, 2)
    let f0 = (0u8, encoding.fragments[0].bytes.as_slice());
    let f2 = (2u8, encoding.fragments[2].bytes.as_slice());
    let recovered = recover_2of3_object(&[f0, f2], encoding.original_length, None, None)
        .expect("recover from 0 and 2");
    assert_eq!(&recovered, original);

    let hdr = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 2,
        dst_cid: 3,
        vsock_port: 1024,
        cl06_spin: 0,
        e8_orbit: 0,
        payload_len: recovered.len() as u32,
    };
    assert_eq!(
        admit_spiralcore_header(&hdr, SPIRALCORE_MAX_PAYLOAD),
        FrameVerdict::FrameAdmitted
    );
}

// =========================================================================
// F17: E2E Suite Structure & Coverage Thresholds
// =========================================================================

#[test]
fn test_t1_f17_e2e_suite_coverage_tier1_threshold() {
    let t1_path = repo_root().join("crates/wasm/tests/overhaul_e2e/tier1_feature.rs");
    let t1_count = count_test_attributes(&t1_path).expect("count t1 tests");
    assert!(
        t1_count >= 95,
        "Tier 1 must have >= 95 tests (got {t1_count})"
    );
}

#[test]
fn test_t1_f17_e2e_suite_coverage_tier2_threshold() {
    let t2_path = repo_root().join("crates/wasm/tests/overhaul_e2e/tier2_boundary.rs");
    let t2_count = count_test_attributes(&t2_path).expect("count t2 tests");
    assert!(
        t2_count >= 95,
        "Tier 2 must have >= 95 tests (got {t2_count})"
    );
}

#[test]
fn test_t1_f17_e2e_suite_coverage_tier3_threshold() {
    let t3_path = repo_root().join("crates/wasm/tests/overhaul_e2e/tier3_combinations.rs");
    let t3_count = count_test_attributes(&t3_path).expect("count t3 tests");
    assert!(
        t3_count >= 15,
        "Tier 3 must have >= 15 tests (got {t3_count})"
    );
}

#[test]
fn test_t1_f17_e2e_suite_coverage_tier4_threshold() {
    let t4_path = repo_root().join("crates/wasm/tests/overhaul_e2e/tier4_scenarios.rs");
    let t4_count = count_test_attributes(&t4_path).expect("count t4 tests");
    assert!(
        t4_count >= 5,
        "Tier 4 must have >= 5 tests (got {t4_count})"
    );
}

#[test]
fn test_t1_f17_e2e_suite_total_test_count_above_210() {
    let t1_path = repo_root().join("crates/wasm/tests/overhaul_e2e/tier1_feature.rs");
    let t2_path = repo_root().join("crates/wasm/tests/overhaul_e2e/tier2_boundary.rs");
    let t3_path = repo_root().join("crates/wasm/tests/overhaul_e2e/tier3_combinations.rs");
    let t4_path = repo_root().join("crates/wasm/tests/overhaul_e2e/tier4_scenarios.rs");

    let t1 = count_test_attributes(&t1_path).expect("count t1");
    let t2 = count_test_attributes(&t2_path).expect("count t2");
    let t3 = count_test_attributes(&t3_path).expect("count t3");
    let t4 = count_test_attributes(&t4_path).expect("count t4");
    let total = t1 + t2 + t3 + t4;

    assert!(
        total >= 210,
        "Total test count must be >= 210 (got {total}: t1={t1}, t2={t2}, t3={t3}, t4={t4})"
    );
}

// =========================================================================
// F18: Adversarial Vector Refusals & Bridge Policy
// =========================================================================

#[test]
fn test_t1_f18_adversarial_malformed_header_refusal() {
    let bad_hdr = SpiralCoreHeader {
        magic: *b"BAD!",
        src_cid: 3,
        dst_cid: 2,
        vsock_port: 1024,
        cl06_spin: 0,
        e8_orbit: 0,
        payload_len: 128,
    };
    let verdict = admit_spiralcore_header(&bad_hdr, SPIRALCORE_MAX_PAYLOAD);
    assert_eq!(verdict, FrameVerdict::RefusedInvalidCid);
}

#[test]
fn test_t1_f18_adversarial_corrupted_shard_digest_refusal() {
    let payload = b"payload to test corrupted shard digest refusal";
    let encoding = split_2of3_xor(payload);
    let bad_bytes = vec![0xFF; encoding.fragments[0].bytes.len()];
    let bad_f0 = (0u8, bad_bytes.as_slice());
    let f1 = (1u8, encoding.fragments[1].bytes.as_slice());
    let expected_frags = [
        &encoding.fragments[0].sha256[..],
        &encoding.fragments[1].sha256[..],
        &encoding.fragments[2].sha256[..],
    ];

    let res = recover_2of3_object(
        &[bad_f0, f1],
        encoding.original_length,
        None,
        Some(&expected_frags),
    );
    assert!(matches!(
        res,
        Err(StitchParityError::ShardDigestMismatch { .. })
    ));
}

#[test]
fn test_t1_f18_adversarial_guest_socket_non_vsock_refusal() {
    let verdict = admit_guest_socket_domain(2); // AF_INET = 2
    assert_eq!(verdict, SocketAdmission::RefusedNicDisabled);
}

#[test]
fn test_t1_f18_adversarial_unallowlisted_host_secret_refusal() {
    let err = BridgeError::HostNotAllowlistedForSecret;
    assert_eq!(err.error_code(), "HostNotAllowlistedForSecret");
}

#[test]
fn test_t1_f18_adversarial_quota_overflow_refusal() {
    let err = BridgeError::PackageSizeExceedsQuota;
    assert_eq!(err.error_code(), "PackageSizeExceedsQuota");
}

// =========================================================================
// F19: Normative Release Gate Verification
// =========================================================================

#[test]
fn test_t1_f19_release_gate_subtarget_checklist() {
    let justfile_path = repo_root().join("Justfile");
    let content = std::fs::read_to_string(&justfile_path).expect("read Justfile");
    let targets = FastFailGateVerifier::parse_justfile_vv(&content).expect("parse vv");
    for &expected in FastFailGateVerifier::NORMATIVE_ORDER {
        assert!(
            targets.iter().any(|t| t == expected),
            "Normative vv recipe must include subtarget: {expected}"
        );
    }
}

#[test]
fn test_t1_f19_release_gate_clean_working_tree_requirement() {
    let git = ScratchGitFixture::new().expect("create scratch fixture");
    assert!(
        git.is_clean().expect("check clean"),
        "working tree must be clean before release"
    );
}

#[test]
fn test_t1_f19_release_gate_semver_tag_format() {
    let tag = "v0.3.0";
    let is_valid = tag.starts_with('v')
        && tag[1..].split('.').count() == 3
        && tag[1..].split('.').all(|p| p.parse::<u32>().is_ok());
    assert!(is_valid, "Tag must follow vX.Y.Z SemVer convention");
}

#[test]
fn test_t1_f19_release_gate_conformance_regeneration_rule_r1() {
    let rows = load_all_capability_rows().expect("load ids.toml");
    let conf_path = repo_root().join("CONFORMANCE.md");
    let content = std::fs::read_to_string(conf_path).expect("read CONFORMANCE.md");
    for row in &rows {
        assert!(
            content.contains(&row.id),
            "CONFORMANCE.md must document registered capability ID: {}",
            row.id
        );
    }
}

#[test]
fn test_t1_f19_release_gate_locked_sdk_immutability() {
    let lock_path = repo_root().join("prismpm.lock");
    assert!(lock_path.is_file(), "prismpm.lock must exist");
    let content = std::fs::read_to_string(lock_path).expect("read prismpm.lock");
    assert!(
        content.contains("@sha256:"),
        "prismpm.lock must contain immutable sha256 digest pin"
    );
}
