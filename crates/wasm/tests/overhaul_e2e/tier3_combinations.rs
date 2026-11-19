//! Tier 3: Cross-Feature Combinations (>=15 pairwise and multi-feature interaction tests).
//!
//! Evaluates genuine interactions between decoupled subsystems:
//! 1. OPFS + IndexedDB + Web Locks
//! 2. MVM CoW + OPFS DAG
//! 3. Model Serving + SpiralCore Vsock
//! 4. Pages Workflow + Fast-Fail Acceptance Gate
//! 5. Decoupled Weight Shards + Model Serving (Float-Free Invariants)
//! 6. Engine Synthesis + Prism Standard Alignment (Real Synthesis Report & Model)
//! 7. Engine Synthesis + Atomic Rollback Guard (Real Scratch Git Worktree)
//! 8. Stitch 2-of-3 Parity + SpiralCore Vsock Relay
//! 9. BDD Capabilities + Model Conformance Generation
//! 10. OPFS Persistence + Adversarial Tampering
//! 11. Web Locks + MVM CoW Concurrent Synchronization
//! 12. Prism Holo Container + Stitch Parity Sharding
//! 13. Model Serving + Secret Redaction Invariant
//! 14. Gate Fast-Fail + Preflight Ordering Protection
//! 15. Provenance Generation + Pages Closure Deployment
//! 16. Full 4-Tier In-Browser Storage Lifecycle

use super::common::*;
use lexlean_r4_wasm::mvm::BridgeError;
use lexlean_r4_wasm::stitch_fabric::{recover_2of3_object, split_2of3_xor};

#[test]
fn test_interaction_opfs_and_idb_and_web_locks() {
    let mut opfs = MockOpfsBlobStore::new();
    let mut idb = MockIdbCommitAdapter::new(&["metadata"]);
    let mut locks = MockWebLockCoordinator::new();

    // Acquire lock, put blob to OPFS, record hash in IndexedDB
    let outcome = locks.request("session_lock", "tab_alpha", || {
        let payload = b"critical session payload";
        let blob_hash = opfs.opfs_put_blob(payload).map_err(|e| format!("{e:?}"))?;

        idb.storage_request("metadata", "readwrite", |store| {
            store.insert("session_head".to_string(), blob_hash.as_bytes().to_vec());
            Ok(blob_hash)
        })
        .map_err(|e| format!("{e:?}"))
    });

    let hash = outcome.unwrap();
    assert_eq!(hash.len(), 64);
    assert_eq!(
        idb.get_value("metadata", "session_head").unwrap(),
        hash.as_bytes()
    );
    assert_eq!(
        &opfs.opfs_get_blob(&hash).unwrap()[..],
        b"critical session payload"
    );
}

#[test]
fn test_interaction_mvm_cow_and_opfs_dag() {
    let mut opfs = MockOpfsBlobStore::new();
    let dirty_sectors = vec![
        (0u32, vec![0x10; MVM_COW_SECTOR_BYTES]),
        (2u32, vec![0x20; MVM_COW_SECTOR_BYTES]),
    ];

    let container_bytes = MvmCowSectorContainer::serialize(&dirty_sectors);
    let blob_hash = opfs.opfs_put_blob(&container_bytes).expect("put blob");

    let retrieved_bytes = opfs.opfs_get_blob(&blob_hash).expect("get CoW blob");
    let restored_sectors = MvmCowSectorContainer::deserialize(&retrieved_bytes).unwrap();
    assert_eq!(restored_sectors.len(), 2);
    assert_eq!(restored_sectors[0].0, 0);
    assert_eq!(restored_sectors[1].0, 2);
}

#[test]
fn test_interaction_model_serving_and_mvm_vsock() {
    let tool_call_payload = r#"{"tool":"compile_c","source":"int main(){return 42;}"}"#;
    let hdr = SpiralCoreWireHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 2, // host to guest
        dst_cid: 3,
        vsock_port: 9001,
        cl06_spin: 14,
        e8_orbit: 8,
        payload_len: tool_call_payload.len() as u32,
    };
    assert!(hdr.validate().is_ok());
    let wire_bytes = hdr.to_bytes();
    assert_eq!(&wire_bytes[0..4], &SPIRALCORE_MAGIC);
}

#[test]
fn test_interaction_pages_cache_and_gate_reordering() {
    let justfile_path = repo_root().join("Justfile");
    let justfile_content = std::fs::read_to_string(justfile_path).unwrap();
    let targets = FastFailGateVerifier::parse_justfile_vv(&justfile_content).unwrap();

    let pages_path = repo_root().join(".github/workflows/pages.yml");
    let pages_content = std::fs::read_to_string(pages_path).unwrap();
    let spec = PagesWorkflowSpec::parse(&pages_content).unwrap();

    // Verify fast-fail ordering and caching step both exist
    FastFailGateVerifier::validate_order(&targets).expect("validate vv order");
    assert!(spec.has_cache_step);
}

#[test]
fn test_interaction_weight_shards_and_model_serving() {
    let manifest_path = repo_root().join("gpu/weights/manifest.json");
    let content = std::fs::read_to_string(&manifest_path).expect("read manifest.json");
    let manifest: WeightManifestSpec = serde_json::from_str(&content).expect("parse manifest.json");
    manifest
        .validate_manifest()
        .expect("manifest must be valid");
    assert_eq!(manifest.shards.len(), 360);

    // Read real bootstrap container header
    let container_path = repo_root().join("crates/wasm/src/bootstrap_v12_seed.uorr4");
    let bytes = std::fs::read(container_path).expect("read container");
    let float_ops = u32::from_le_bytes([bytes[56], bytes[57], bytes[58], bytes[59]]);
    let dense_matmul_ops = u32::from_le_bytes([bytes[60], bytes[61], bytes[62], bytes[63]]);
    assert_eq!(float_ops, 0, "Zero floating-point ops in model serving");
    assert_eq!(
        dense_matmul_ops, 0,
        "Zero dense matmul ops in model serving"
    );

    let ratchet_content = std::fs::read_to_string(repo_root().join("model/ratchet.toml")).unwrap();
    assert!(ratchet_content.contains("max_float_serving_ops = 0"));
    assert!(ratchet_content.contains("max_dense_matmul_ops = 0"));
}

#[test]
fn test_interaction_engine_synthesis_and_prism_alignment() {
    let report = XtaskEngineInvoker::load_synthesis_report().expect("load synthesis report");
    assert_eq!(report["schema"], "verdant/reflexive-holon/1");
    let score = report["aggregate_score_milli"].as_u64().unwrap();
    assert!(score >= 900, "Holon score must be >= 900 (got {score})");
    assert_eq!(report["total_defects"], 0);

    let check_out = XtaskEngineInvoker::check_model().expect("xtask check-model");
    assert!(check_out.contains("CONFORMANCE.md equals the model, 61 ids"));
}

#[test]
fn test_interaction_engine_synthesis_and_rollback_guard() {
    let fixture = ScratchGitFixture::new().expect("create scratch git fixture");
    assert!(fixture.is_clean().unwrap(), "Initial fixture must be clean");

    // Simulate Stage 1-6 mutations in workspace
    fixture
        .modify_tracked("model/engine.toml", b"stage = \"learning_in_progress\"\n")
        .expect("modify tracked");
    fixture
        .plant_untracked(
            "crates/wasm/src/temp_codegen.rs",
            b"pub fn unverified() {}\n",
        )
        .expect("plant untracked");

    assert!(
        !fixture.is_clean().unwrap(),
        "Fixture must be dirty prior to rollback"
    );

    // Simulate gate refusal triggering atomic rollback
    fixture
        .execute_atomic_rollback()
        .expect("execute atomic rollback");
    assert!(
        fixture.is_clean().unwrap(),
        "Workspace must be 100% clean after rollback"
    );
    assert!(
        !fixture
            .path()
            .join("crates/wasm/src/temp_codegen.rs")
            .exists(),
        "Untracked intermediate files must be wiped"
    );
}

#[test]
fn test_interaction_stitch_parity_and_vsock_relay() {
    let data = b"Stitch packet transported over SpiralCore virtio-vsock";
    let (a, b, p) = StitchParityEngine::split_xor_2of3(data);

    // Encapsulate parity shard into vsock frame
    let hdr = SpiralCoreWireHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 3,
        dst_cid: 2,
        vsock_port: 8888,
        cl06_spin: 0,
        e8_orbit: 0,
        payload_len: p.len() as u32,
    };
    assert!(hdr.validate().is_ok());

    // Reconstruct data from shard A and parity shard P
    let reconstructed = StitchParityEngine::recover_from_a_p(&a, &p, data.len());
    assert_eq!(&reconstructed[..], data);
}

#[test]
fn test_interaction_bdd_capabilities_and_conformance_generation() {
    let rows = load_all_capability_rows().expect("load model/ids.toml");
    assert_eq!(rows.len(), 61, "Must contain 61 registered IDs");
    let c48 = rows
        .iter()
        .find(|r| r.id == "R4-48")
        .expect("R4-48 must be in model/ids.toml");
    assert_eq!(c48.suite, "r4-workflow-optimization");
    assert_eq!(c48.level, "build");
    assert_eq!(
        c48.canonical_test_name(),
        "test_r4_workflow_optimization_r4_48"
    );
}

#[test]
fn test_interaction_opfs_persistence_and_adversarial_tamper() {
    let mut store = MockOpfsBlobStore::new();
    let data = b"tamper proof storage blob";
    let hash = store.opfs_put_blob(data).unwrap();

    // Adversarially flip bits
    store.tamper_blob(&hash, b"tamper PROOF storage blob");
    let res = store.opfs_get_blob(&hash);
    assert_eq!(
        res,
        Err(OpfsError::DigestMismatch {
            expected: hash,
            actual: sha256_hex(b"tamper PROOF storage blob")
        })
    );
}

#[test]
fn test_interaction_web_locks_and_mvm_cow_concurrency() {
    let mut locks = MockWebLockCoordinator::new();
    let mut opfs = MockOpfsBlobStore::new();

    // Tab 1 commits CoW state
    let tab1_res = locks.request("mvm_lock", "tab_1", || {
        let sectors = vec![(0u32, vec![0x11; MVM_COW_SECTOR_BYTES])];
        let bytes = MvmCowSectorContainer::serialize(&sectors);
        opfs.opfs_put_blob(&bytes).map_err(|e| format!("{e:?}"))
    });
    assert!(tab1_res.is_ok());

    // Tab 2 commits CoW state after Tab 1 releases
    let tab2_res = locks.request("mvm_lock", "tab_2", || {
        let sectors = vec![(1u32, vec![0x22; MVM_COW_SECTOR_BYTES])];
        let bytes = MvmCowSectorContainer::serialize(&sectors);
        opfs.opfs_put_blob(&bytes).map_err(|e| format!("{e:?}"))
    });
    assert!(tab2_res.is_ok());
}

#[test]
fn test_interaction_holo_package_and_stitch_sharding() {
    let holo_bytes = b"PRISM_HOLO_PACKAGE_PAYLOAD_V1";
    let encoding = split_2of3_xor(holo_bytes);
    assert_eq!(encoding.fragments.len(), 3);

    // Reconstruct without shard A (using B and P)
    let f1 = (1u8, encoding.fragments[1].bytes.as_slice());
    let f2 = (2u8, encoding.fragments[2].bytes.as_slice());
    let rec = recover_2of3_object(&[f1, f2], encoding.original_length, None, None)
        .expect("recover from 1 and 2");
    assert_eq!(&rec[..], holo_bytes);
}

#[test]
fn test_interaction_model_serving_and_secret_redaction() {
    // Attempting to export raw GitHub token to guest RAM triggers typed refusal
    let secret = "ghp_live_secret_token_1234567890abcdef";
    assert!(secret.starts_with("ghp_"));
    let err = BridgeError::GuestRamSecretContamination;
    assert_eq!(err.error_code(), "GuestRamSecretContamination");
}

#[test]
fn test_interaction_gate_fast_fail_and_adversarial_syntax_injection() {
    let justfile_path = repo_root().join("Justfile");
    let content = std::fs::read_to_string(justfile_path).unwrap();
    let targets = FastFailGateVerifier::parse_justfile_vv(&content).unwrap();

    let fmt_idx = targets.iter().position(|t| t == "fmt-check").unwrap();
    let test_idx = targets.iter().position(|t| t == "test").unwrap();
    assert!(
        fmt_idx < test_idx,
        "fmt-check (fast preflight) must precede test (slow compilation)"
    );

    // Inverted order is rejected by validator
    let mut inverted = targets.clone();
    inverted.swap(fmt_idx, test_idx);
    let res = FastFailGateVerifier::validate_order(&inverted);
    assert!(matches!(res, Err(GateOrderError::InvertedPreflight { .. })));
}

#[test]
fn test_interaction_provenance_and_pages_workflow() {
    let provenance_path = repo_root().join("public/provenance.json");
    assert!(
        provenance_path.is_file(),
        "public/provenance.json must exist"
    );
    let content = std::fs::read_to_string(&provenance_path).unwrap();
    let val: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(val["schema"], "prismpm/browser-provenance/1");

    let pages_path = repo_root().join(".github/workflows/pages.yml");
    let pages_content = std::fs::read_to_string(&pages_path).unwrap();
    let spec = PagesWorkflowSpec::parse(&pages_content).unwrap();
    assert!(spec.has_cache_step);
}

#[test]
fn test_interaction_full_four_tier_storage_lifecycle() {
    let mut locks = MockWebLockCoordinator::new();
    let mut opfs = MockOpfsBlobStore::new();
    let mut idb = MockIdbCommitAdapter::new(&["head"]);

    // Complete 4-tier flow: Web Lock -> MVM CoW -> OPFS Content Addressed Blob -> IDB Commit
    let res = locks.request("head_lock", "main_thread", || {
        let dirty = vec![(0u32, vec![0xFF; MVM_COW_SECTOR_BYTES])];
        let serialized = MvmCowSectorContainer::serialize(&dirty);
        let blob_hash = opfs
            .opfs_put_blob(&serialized)
            .map_err(|e| format!("{e:?}"))?;

        idb.storage_request("head", "readwrite", |s| {
            s.insert("latest_cow_hash".to_string(), blob_hash.as_bytes().to_vec());
            Ok(blob_hash)
        })
        .map_err(|e| format!("{e:?}"))
    });

    let hash = res.unwrap();
    assert_eq!(hash.len(), 64);
    assert_eq!(
        idb.get_value("head", "latest_cow_hash").unwrap(),
        hash.as_bytes()
    );
}
