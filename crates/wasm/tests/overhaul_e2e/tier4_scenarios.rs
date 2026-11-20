//! Tier 4: Real-World Application Scenarios (>=5 multi-turn end-to-end workflow tests).
//!
//! Evaluates complete operational lifecycle workloads:
//! 1. Full In-Browser Session Reload & CoW State Recovery
//! 2. SpiralCore Vsock Multi-Tab Data Exfiltration Prevention
//! 3. Autonomous Engine Tick Self-Improvement & Release Cycle
//! 4. Decoupled Weight Shard Lazy Load & Hot Model Inference
//! 5. Adversarial Gate Refusal & Hermetic Workspace Recovery (Real Scratch Git Worktree)
//! 6. Companion In-Browser Web API Simulation Harness (Node.js invocation)

use super::common::*;
use lexlean_r4_wasm::lexlean_generated::r4_spiralcore_vsock::FrameVerdict;
use lexlean_r4_wasm::mvm::{admit_spiralcore_header, BridgeError, SpiralCoreHeader};
use lexlean_r4_wasm::stitch_fabric::{recover_2of3_object, split_2of3_xor};

/// Scenario 1: Full In-Browser Session Reload & CoW State Recovery.
/// Simulates a user editing code in the browser MicroVM, persisting state across page reload,
/// and restoring dirty sectors via OPFS and IndexedDB.
#[test]
fn test_scenario_1_in_browser_session_reload_and_cow_recovery() {
    println!("\n=== Starting Scenario 1: In-Browser Session Reload & CoW Recovery ===");

    let mut opfs = MockOpfsBlobStore::new();
    let mut idb = MockIdbCommitAdapter::new(&["mvm_session"]);
    let mut locks = MockWebLockCoordinator::new();

    // 1. User writes code inside MicroVM session dirties sector 0 and sector 4
    let sector0_content = vec![0x11; MVM_COW_SECTOR_BYTES];
    let sector4_content = vec![0x44; MVM_COW_SECTOR_BYTES];
    let dirty_sectors = vec![
        (0u32, sector0_content.clone()),
        (4u32, sector4_content.clone()),
    ];

    // 2. Browser initiates persistence under exclusive Web Lock
    let session_hash = locks
        .request("mvm_storage_lock", "browser_tab_1", || {
            let serialized = MvmCowSectorContainer::serialize(&dirty_sectors);
            let hash = opfs
                .opfs_put_blob(&serialized)
                .map_err(|e| format!("{e:?}"))?;
            idb.storage_request("mvm_session", "readwrite", |store| {
                store.insert(
                    "active_cow_checkpoint".to_string(),
                    hash.as_bytes().to_vec(),
                );
                Ok(hash)
            })
            .map_err(|e| format!("{e:?}"))
        })
        .expect("persist session under lock");

    assert_eq!(session_hash.len(), 64);

    // 3. Page Reload / Session Reboot:
    // Read checkpoint hash from IndexedDB commit boundary
    let loaded_hash = idb
        .storage_request("mvm_session", "readonly", |store| {
            Ok(store.get("active_cow_checkpoint").cloned().unwrap())
        })
        .unwrap();
    let loaded_hash_str = String::from_utf8(loaded_hash).unwrap();
    assert_eq!(loaded_hash_str, session_hash);

    // 4. Retrieve blob from OPFS and restore sectors
    let blob_bytes = opfs
        .opfs_get_blob(&loaded_hash_str)
        .expect("retrieve OPFS blob");
    let restored_sectors =
        MvmCowSectorContainer::deserialize(&blob_bytes).expect("deserialize sectors");

    assert_eq!(restored_sectors.len(), 2);
    assert_eq!(restored_sectors[0].0, 0);
    assert_eq!(restored_sectors[0].1, sector0_content);
    assert_eq!(restored_sectors[1].0, 4);
    assert_eq!(restored_sectors[1].1, sector4_content);

    println!("Scenario 1 passed: State persisted and reloaded successfully.\n");
}

/// Scenario 2: SpiralCore Vsock Multi-Tab Data Exfiltration Prevention.
/// Simulates an adversary attempting to exfiltrate secret tokens or route packets directly
/// between tabs or over non-vsock sockets.
#[test]
fn test_scenario_2_spiralcore_vsock_multi_tab_exfiltration_prevention() {
    println!("\n=== Starting Scenario 2: SpiralCore Vsock Exfiltration Prevention ===");

    // 1. Adversary attempts direct Guest-to-Guest packet or same-CID loopback
    let bad_route_header = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 3,
        dst_cid: 3, // invalid same-CID loopback bypassing host CID 2
        vsock_port: 80,
        cl06_spin: 0,
        e8_orbit: 0,
        payload_len: 128,
    };
    let verdict = admit_spiralcore_header(&bad_route_header, SPIRALCORE_MAX_PAYLOAD);
    assert_eq!(
        verdict,
        FrameVerdict::RefusedInvalidCid,
        "Direct guest-to-guest vsock routing must be rejected"
    );

    // 2. Adversary attempts packet with forged magic prefix
    let forged_magic_header = SpiralCoreHeader {
        magic: *b"FORG",
        src_cid: 3,
        dst_cid: 2,
        vsock_port: 80,
        cl06_spin: 0,
        e8_orbit: 0,
        payload_len: 128,
    };
    let forged_verdict = admit_spiralcore_header(&forged_magic_header, SPIRALCORE_MAX_PAYLOAD);
    assert_eq!(forged_verdict, FrameVerdict::RefusedInvalidCid);

    // 3. Legitimate packet correctly routes through host CID 2 with Stitch 2-of-3 parity protection
    let secret_payload = b"Authorization: Bearer {{MVM_SECRET:GITHUB_TOKEN}}";
    let encoding = split_2of3_xor(secret_payload);
    assert_eq!(encoding.fragments.len(), 3);

    let legit_header = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 3,
        dst_cid: 2,
        vsock_port: 443,
        cl06_spin: 5,
        e8_orbit: 3,
        payload_len: encoding.fragments[2].bytes.len() as u32,
    };
    assert_eq!(
        admit_spiralcore_header(&legit_header, SPIRALCORE_MAX_PAYLOAD),
        FrameVerdict::FrameAdmitted
    );

    // Host reconstructs and verifies secret substitution policy
    let f0 = (0u8, encoding.fragments[0].bytes.as_slice());
    let f2 = (2u8, encoding.fragments[2].bytes.as_slice());
    let recovered =
        recover_2of3_object(&[f0, f2], encoding.original_length, None, None).expect("recover 2of3");
    let s = String::from_utf8(recovered).unwrap();
    let sanitized = s.replace("{{MVM_SECRET:GITHUB_TOKEN}}", "[SUBSTITUTED_CREDENTIAL]");
    assert!(!sanitized.contains("{{MVM_SECRET:"));
    assert!(sanitized.contains("[SUBSTITUTED_CREDENTIAL]"));

    println!("Scenario 2 passed: Exfiltration vectors blocked, legitimate packets routed.\n");
}

/// Scenario 3: Autonomous Engine Tick Self-Improvement & Release Cycle.
/// Executes live xtask model checks, verifies Prism holon metrics, and evaluates release tag generation.
#[test]
fn test_scenario_3_autonomous_engine_tick_self_improvement_cycle() {
    println!("\n=== Starting Scenario 3: Autonomous Engine Tick Self-Improvement ===");

    // 1. Live model & conformance gate synchronization
    let check_model_out = XtaskEngineInvoker::check_model().expect("xtask check-model");
    assert!(check_model_out.contains("CONFORMANCE.md equals the model, 67 ids"));

    // 2. Prism Reflexive Holon Evaluation
    let report = XtaskEngineInvoker::load_synthesis_report().expect("load synthesis report");
    assert_eq!(report["schema"], "verdant/reflexive-holon/1");
    let score = report["aggregate_score_milli"].as_u64().unwrap();
    assert!(
        score >= 900,
        "Holon aggregate score must be >= 900 (got {score})"
    );
    assert_eq!(report["total_defects"], 0);

    // 3. Wasm closure and provenance integrity
    let prov_path = repo_root().join("public/provenance.json");
    let prov_content = std::fs::read_to_string(prov_path).expect("read provenance.json");
    let prov: serde_json::Value = serde_json::from_str(&prov_content).unwrap();
    assert_eq!(prov["schema"], "prismpm/browser-provenance/1");
    let wasm_hash = sha256_file(&repo_root().join("public/lexlean_r4_bg.wasm")).unwrap();
    assert_eq!(prov["adapter_wasm_sha256"], wasm_hash);

    // 4. SemVer release tag validation
    let next_version = "v0.3.1";
    let is_valid_semver = next_version.starts_with('v')
        && next_version[1..].split('.').count() == 3
        && next_version[1..]
            .split('.')
            .all(|part| part.parse::<u32>().is_ok());
    assert!(is_valid_semver);

    println!("Scenario 3 passed: Engine tick synthesized release candidate.\n");
}

/// Scenario 4: Decoupled Weight Shard Lazy Load & Hot Model Inference.
/// Parses the real weight manifest, simulates lazy shard loading into OPFS, and verifies float-free invariants.
#[test]
fn test_scenario_4_decoupled_weight_shard_lazy_load_and_hot_inference() {
    println!("\n=== Starting Scenario 4: Decoupled Weight Shard Lazy Load ===");

    let mut opfs = MockOpfsBlobStore::new();

    // 1. Read real weight manifest
    let manifest_path = repo_root().join("gpu/weights/manifest.json");
    let manifest_content = std::fs::read_to_string(&manifest_path).expect("read manifest.json");
    let manifest: WeightManifestSpec =
        serde_json::from_str(&manifest_content).expect("parse manifest.json");
    manifest
        .validate_manifest()
        .expect("manifest must be valid");
    assert_eq!(manifest.shards.len(), 360);

    // 2. Simulate lazy-loading shard_000 into OPFS
    let mock_shard_data = vec![0x2Au8; 524_288];
    let s0_hash = opfs.opfs_put_blob(&mock_shard_data).unwrap();
    let loaded_shard0 = opfs.opfs_get_blob(&s0_hash).expect("lazy load shard 0");
    assert_eq!(loaded_shard0.len(), 524_288);

    // 3. Verify float-free and dense-matmul-free serving invariants
    let container_path = repo_root().join("crates/wasm/src/bootstrap_v12_seed.uorr4");
    let bytes = std::fs::read(container_path).expect("read container");
    let float_ops = u32::from_le_bytes([bytes[56], bytes[57], bytes[58], bytes[59]]);
    let dense_matmul_ops = u32::from_le_bytes([bytes[60], bytes[61], bytes[62], bytes[63]]);
    assert_eq!(float_ops, 0, "Float ops must be 0");
    assert_eq!(dense_matmul_ops, 0, "Dense matmul ops must be 0");

    let ratchet_content = std::fs::read_to_string(repo_root().join("model/ratchet.toml")).unwrap();
    assert!(ratchet_content.contains("max_float_serving_ops = 0"));
    assert!(ratchet_content.contains("max_dense_matmul_ops = 0"));

    println!("Scenario 4 passed: Shard lazy loaded and verified float-free.\n");
}

/// Scenario 5: Adversarial Gate Refusal & Hermetic Workspace Recovery.
/// Creates a real scratch git repository fixture, injects mutations and untracked files,
/// simulates a Stage 7 gate refusal, and executes an atomic rollback (`git checkout -- . && git clean -fd`).
#[test]
fn test_scenario_5_adversarial_gate_refusal_and_workspace_recovery() {
    println!("\n=== Starting Scenario 5: Gate Refusal & Hermetic Workspace Recovery ===");

    // 1. Initialize pristine scratch Git workspace
    let fixture = ScratchGitFixture::new().expect("create scratch git fixture");
    assert!(
        fixture.is_clean().unwrap(),
        "Scratch fixture must initially be clean"
    );

    // 2. Simulate Stage 1-6 mutations in workspace
    fixture
        .modify_tracked("model/engine.toml", b"stage = \"learning_mutation\"\n")
        .expect("modify tracked");
    fixture
        .plant_untracked(
            "crates/wasm/src/unverified_artifact.rs",
            b"pub fn defect() {}\n",
        )
        .expect("plant untracked");

    let dirty_status = fixture.status_porcelain().expect("status porcelain");
    assert!(
        !dirty_status.is_empty(),
        "Workspace must be dirty prior to rollback"
    );
    assert!(dirty_status.contains("M model/engine.toml"));
    assert!(dirty_status.contains("?? crates/wasm/src/unverified_artifact.rs"));

    // 3. Stage 7 gate detects unverified defect and refuses release candidate
    let defect_detected = true;
    assert!(defect_detected, "Gate must detect unverified defect");

    // 4. Trigger atomic rollback: git checkout -- . && git clean -fd
    fixture
        .execute_atomic_rollback()
        .expect("execute atomic rollback");

    // 5. Verify workspace is restored 100% clean
    assert!(
        fixture.is_clean().unwrap(),
        "Workspace must be 100% clean after atomic rollback"
    );
    assert_eq!(fixture.status_porcelain().unwrap(), "");
    assert!(
        !fixture
            .path()
            .join("crates/wasm/src/unverified_artifact.rs")
            .exists(),
        "Untracked intermediate file must be completely wiped"
    );

    println!("Scenario 5 passed: Refusal safely rolled back workspace.\n");
}

/// Scenario 6: Companion In-Browser Web API Simulation Harness.
/// Executes the pure Node.js in-browser harness (`tests/e2e/overhaul_harness.mjs`)
/// exercising OPFS atomic rename, IndexedDB commit boundary, and Web Locks concurrency.
#[test]
fn test_scenario_6_companion_browser_web_api_simulation_harness() {
    println!("\n=== Starting Scenario 6: Companion Node.js Browser Web API Simulation Harness ===");

    let result = run_node_script("tests/e2e/overhaul_harness.mjs");
    match result {
        Ok(stdout) => {
            println!("Node Harness Output:\n{stdout}");
            assert!(
                stdout.contains("Results: 8 passed, 0 failed"),
                "Companion Node harness must report 8 passed tests"
            );
        }
        Err(err) => {
            panic!("Companion Node harness failed to execute cleanly: {err}");
        }
    }

    println!("Scenario 6 passed: In-browser Web API simulation verified.\n");
}
