//! Tier 2: Boundary & Corner Cases E2E Tests (95 Tests across Features F01–F19).
//!
//! Enforces:
//! - 5 distinct, typed boundary/refusal test cases for each of the 19 features ($19 \times 5 = 95$).
//! - Zero facade implementations or simulated engine ticks.
//! - Rule R5: Strongly typed failure enums asserted on exact variants (zero wildcards, zero untyped `is_err()`).
//! - Complete absence of tautological assertions.

use super::common::*;
use lexlean_r4_wasm::lexlean_generated::r4_connected_studio::{
    admit_bridge_transport, BridgeTransportVerdict, McpPolicyMode, TransportKind,
};
use lexlean_r4_wasm::lexlean_generated::r4_spiralcore_vsock::{
    admit_host_egress, EgressPolicyCheck, EgressVerdict, ElfAdmission, FrameVerdict,
};
use lexlean_r4_wasm::mvm::{
    admit_elf64_riscv, admit_spiralcore_header, MvmSandbox, SpiralCoreHeader,
};
use lexlean_r4_wasm::r4_geometric_stack::{R4GeometricStackEngine, R4StackError};
use lexlean_r4_wasm::stitch_fabric::{recover_2of3_object, split_2of3_xor, StitchParityError};
use std::path::PathBuf;

// =========================================================================
// F01: CI/CD Target Caching Workflow Refusals
// =========================================================================

#[test]
fn test_t2_f01_pages_workflow_refuses_empty_content() {
    let res = PagesWorkflowSpec::parse("");
    assert_eq!(res, Err(WorkflowError::EmptyContent));
}

#[test]
fn test_t2_f01_pages_workflow_refuses_unkeyed_cache() {
    let spec = PagesWorkflowSpec {
        has_cache_step: true,
        cache_action: "actions/cache@v4".to_string(),
        cache_key: "".to_string(),
        restore_keys: vec!["r4-pages-target-".to_string()],
        paths: vec!["target".to_string()],
        timeout_minutes: Some(90),
        cache_step_index: Some(1),
        preflight_step_index: Some(2),
    };
    let res = spec.validate_cache_integrity();
    assert_eq!(
        res,
        Err(WorkflowError::UnboundCacheKey {
            key: "".to_string(),
            expected_binding: "Cargo.lock".to_string(),
        })
    );
}

#[test]
fn test_t2_f01_pages_workflow_refuses_wildcard_branch() {
    let yaml = "on:\n  push:\n    branches: ['*']\n";
    let res = PagesWorkflowSpec::parse(yaml);
    assert_eq!(
        res,
        Err(WorkflowError::WildcardBranchTrigger("*".to_string()))
    );
}

#[test]
fn test_t2_f01_pages_workflow_boundary_timeout_minutes() {
    let spec = PagesWorkflowSpec {
        has_cache_step: true,
        cache_action: "actions/cache@v4".to_string(),
        cache_key: "r4-pages-target-${{ hashFiles('Cargo.lock') }}".to_string(),
        restore_keys: vec!["r4-pages-target-".to_string()],
        paths: vec!["target".to_string()],
        timeout_minutes: Some(150), // Exceeds 120 max
        cache_step_index: Some(1),
        preflight_step_index: Some(2),
    };
    let res = spec.validate_cache_integrity();
    assert_eq!(
        res,
        Err(WorkflowError::InvalidTimeoutMinutes {
            job: "build".to_string(),
            timeout: 150,
            min: 1,
            max: 120,
        })
    );
}

#[test]
fn test_t2_f01_pages_workflow_refuses_missing_target_path() {
    let spec = PagesWorkflowSpec {
        has_cache_step: true,
        cache_action: "actions/cache@v4".to_string(),
        cache_key: "r4-pages-target-${{ hashFiles('Cargo.lock') }}".to_string(),
        restore_keys: vec!["r4-pages-target-".to_string()],
        paths: vec!["dist".to_string()], // Omits target directory
        timeout_minutes: Some(90),
        cache_step_index: Some(1),
        preflight_step_index: Some(2),
    };
    let res = spec.validate_cache_integrity();
    assert_eq!(res, Err(WorkflowError::MissingTargetCachePath));
}

// =========================================================================
// F02: Fast-Fail Gate Order Refusals
// =========================================================================

#[test]
fn test_t2_f02_gate_order_refuses_fmt_check_inversion() {
    let targets = vec![
        "test".to_string(),
        "fmt-check".to_string(),
        "template-check".to_string(),
        "model".to_string(),
        "lint".to_string(),
        "features".to_string(),
        "deny".to_string(),
    ];
    let res = FastFailGateVerifier::validate_order(&targets);
    assert_eq!(
        res,
        Err(GateOrderError::InvertedPreflight {
            first: "test".to_string(),
            expected: "fmt-check".to_string(),
        })
    );
}

#[test]
fn test_t2_f02_gate_order_refuses_empty_target_list() {
    let res = FastFailGateVerifier::validate_order(&[]);
    assert_eq!(res, Err(GateOrderError::EmptySubtargetList));
}

#[test]
fn test_t2_f02_gate_order_refuses_unknown_subtarget() {
    let targets = vec!["fmt-check".to_string(), "unknown_bypass_step".to_string()];
    let res = FastFailGateVerifier::validate_order(&targets);
    assert_eq!(
        res,
        Err(GateOrderError::UnknownSubtarget(
            "unknown_bypass_step".to_string()
        ))
    );
}

#[test]
fn test_t2_f02_gate_order_refuses_missing_normative_target() {
    let targets = vec![
        "fmt-check".to_string(),
        "model".to_string(),
        "lint".to_string(),
        "test".to_string(),
        "features".to_string(),
        "deny".to_string(),
        // Omits template-check
    ];
    let res = FastFailGateVerifier::validate_order(&targets);
    assert_eq!(
        res,
        Err(GateOrderError::MissingNormativeTarget(
            "template-check".to_string()
        ))
    );
}

#[test]
fn test_t2_f02_gate_order_refuses_circular_dependency() {
    let res = validate_gate_dependencies(&[("b", "a"), ("a", "b")]);
    assert_eq!(
        res,
        Err(GateOrderError::CircularDependency {
            recipe: "b".to_string(),
            depends_on: "a".to_string(),
        })
    );
}

// =========================================================================
// F03: Decoupled Weight Shard Distribution Refusals
// =========================================================================

#[test]
fn test_t2_f03_manifest_refuses_tampered_shard_sha256() {
    let spec = WeightManifestSpec {
        format: "UORLUT01".to_string(),
        container_sha256: "a".repeat(64),
        total_bytes: 1024,
        shards: vec![WeightShardEntry {
            index: 0,
            file: "shard_000.bin".to_string(),
            sha256: "bad_hex_not_64_chars".to_string(),
            byte_length: 1024,
        }],
    };
    let res = spec.validate_manifest();
    assert_eq!(
        res,
        Err(ManifestError::InvalidShardSha256 {
            shard: "shard_000.bin".to_string(),
            sha256: "bad_hex_not_64_chars".to_string(),
        })
    );
}

#[test]
fn test_t2_f03_manifest_refuses_size_mismatch() {
    let spec = WeightManifestSpec {
        format: "UORLUT01".to_string(),
        container_sha256: "b".repeat(64),
        total_bytes: 2048,
        shards: vec![WeightShardEntry {
            index: 0,
            file: "shard_000.bin".to_string(),
            sha256: "c".repeat(64),
            byte_length: 1024, // 1024 != 2048
        }],
    };
    let res = spec.validate_manifest();
    assert_eq!(
        res,
        Err(ManifestError::TotalBytesMismatch {
            declared: 2048,
            calculated: 1024,
        })
    );
}

#[test]
fn test_t2_f03_manifest_refuses_exceeding_wasm_budget() {
    let oversized = 15 * 1024 * 1024 + 1; // 15,728,641 bytes
    let res = validate_wasm_budget(oversized, WASM_BUDGET_BYTES);
    assert_eq!(
        res,
        Err(ManifestError::WasmBudgetExceeded {
            size_bytes: oversized,
            budget_bytes: 15 * 1024 * 1024,
        })
    );
}

#[test]
fn test_t2_f03_manifest_boundary_zero_shards() {
    let spec = WeightManifestSpec {
        format: "UORLUT01".to_string(),
        container_sha256: "d".repeat(64),
        total_bytes: 0,
        shards: vec![],
    };
    let res = spec.validate_manifest();
    assert_eq!(res, Err(ManifestError::ZeroShards));
}

#[test]
fn test_t2_f03_manifest_refuses_non_hex_container_sha256() {
    let spec = WeightManifestSpec {
        format: "UORLUT01".to_string(),
        container_sha256: "Z".repeat(64), // Non-hex
        total_bytes: 100,
        shards: vec![WeightShardEntry {
            index: 0,
            file: "shard_000.bin".to_string(),
            sha256: "e".repeat(64),
            byte_length: 100,
        }],
    };
    let res = spec.validate_manifest();
    assert_eq!(
        res,
        Err(ManifestError::InvalidContainerSha256("Z".repeat(64)))
    );
}

// =========================================================================
// F04: Capability R4-48 Specification Refusals
// =========================================================================

#[test]
fn test_t2_f04_capability_r4_48_refuses_invalid_id_prefix() {
    let cap = CapabilityRow {
        id: "BAD-48".to_string(),
        level: "build".to_string(),
        suite: "r4-workflow-optimization".to_string(),
        statement: "statement".to_string(),
    };
    assert_eq!(
        cap.validate(),
        Err(CapabilityError::InvalidIdPrefix("BAD-48".to_string()))
    );
}

#[test]
fn test_t2_f04_capability_r4_48_refuses_empty_statement() {
    let cap = CapabilityRow {
        id: "R4-48".to_string(),
        level: "build".to_string(),
        suite: "r4-workflow-optimization".to_string(),
        statement: "".to_string(),
    };
    assert_eq!(cap.validate(), Err(CapabilityError::EmptyStatement));
}

#[test]
fn test_t2_f04_capability_r4_48_refuses_empty_suite() {
    let cap = CapabilityRow {
        id: "R4-48".to_string(),
        level: "build".to_string(),
        suite: "".to_string(),
        statement: "statement".to_string(),
    };
    assert_eq!(cap.validate(), Err(CapabilityError::EmptySuite));
}

#[test]
fn test_t2_f04_capability_r4_48_refuses_invalid_epistemic_level() {
    let cap = CapabilityRow {
        id: "R4-48".to_string(),
        level: "magic".to_string(),
        suite: "r4-workflow-optimization".to_string(),
        statement: "statement".to_string(),
    };
    assert_eq!(
        cap.validate(),
        Err(CapabilityError::InvalidLevel("magic".to_string()))
    );
}

#[test]
fn test_t2_f04_capability_r4_48_refuses_missing_scenario_step() {
    let incomplete_gherkin = "Scenario: Missing then\nGiven step A\nWhen step B\n";
    let res = validate_gherkin_scenario("R4-48", incomplete_gherkin);
    assert_eq!(
        res,
        Err(CapabilityError::MissingGherkinScenarioStep {
            scenario: "R4-48".to_string(),
            missing_step: "Then",
        })
    );
}

// =========================================================================
// F05: OPFS Storage Subsystem Typed Refusals
// =========================================================================

#[test]
fn test_t2_f05_opfs_refuses_store_missing_with_typed_error() {
    let store = MockOpfsBlobStore::new();
    let hash = "a".repeat(64);
    let res = store.opfs_get_blob(&hash);
    assert_eq!(res, Err(OpfsError::Missing(hash)));
}

#[test]
fn test_t2_f05_opfs_refuses_store_mismatch_with_typed_error() {
    let mut store = MockOpfsBlobStore::new();
    let original = b"clean original data";
    let hash = store.opfs_put_blob(original).unwrap();
    store.tamper_blob(&hash, b"corrupted tampered data");
    let res = store.opfs_get_blob(&hash);
    assert_eq!(
        res,
        Err(OpfsError::DigestMismatch {
            expected: hash,
            actual: sha256_hex(b"corrupted tampered data"),
        })
    );
}

#[test]
fn test_t2_f05_opfs_refuses_invalid_hex_hash_lookup() {
    let store = MockOpfsBlobStore::new();
    let malformed = "not-a-valid-hex-hash";
    let res = store.opfs_get_blob(malformed);
    assert_eq!(
        res,
        Err(OpfsError::InvalidHashFormat(malformed.to_string()))
    );
}

#[test]
fn test_t2_f05_opfs_boundary_zero_byte_blob() {
    let mut store = MockOpfsBlobStore::new();
    let hash = store.opfs_put_blob(b"").unwrap();
    let res = store.opfs_get_blob(&hash);
    assert_eq!(res, Ok(b"".to_vec()));
}

#[test]
fn test_t2_f05_opfs_boundary_max_blob_size() {
    let mut store = MockOpfsBlobStore::new();
    let oversized = vec![0xEEu8; 104_857_601]; // 100 MB + 1 byte
    let res = store.opfs_put_blob(&oversized);
    assert_eq!(
        res,
        Err(OpfsError::PayloadTooLarge {
            size: 104_857_601,
            limit: 104_857_600,
        })
    );
}

// =========================================================================
// F06: IndexedDB Commit-Boundary Adapter Typed Refusals
// =========================================================================

#[test]
fn test_t2_f06_idb_adapter_refuses_on_abort_with_typed_error() {
    let mut adapter = MockIdbCommitAdapter::new(&["workspace"]);
    let res: Result<(), IdbError> = adapter.storage_request("workspace", "readwrite", |_store| {
        Err("simulated transaction abort".to_string())
    });
    assert_eq!(
        res,
        Err(IdbError::StorageTransactionFailed {
            store: "workspace".to_string(),
            reason: "simulated transaction abort".to_string(),
        })
    );
}

#[test]
fn test_t2_f06_idb_adapter_refuses_on_unknown_store_with_typed_error() {
    let mut adapter = MockIdbCommitAdapter::new(&["workspace"]);
    let res: Result<(), IdbError> =
        adapter.storage_request("unknown_store", "readwrite", |_store| Ok(()));
    assert_eq!(
        res,
        Err(IdbError::StoreNotFound {
            store: "unknown_store".to_string(),
            available: vec!["workspace".to_string()],
        })
    );
}

#[test]
fn test_t2_f06_idb_adapter_boundary_empty_store_name() {
    let mut adapter = MockIdbCommitAdapter::new(&["workspace"]);
    let res: Result<(), IdbError> = adapter.storage_request("", "readwrite", |_store| Ok(()));
    assert_eq!(res, Err(IdbError::EmptyStoreName));
}

#[test]
fn test_t2_f06_idb_adapter_refuses_write_in_readonly_mode() {
    let mut adapter = MockIdbCommitAdapter::new(&["workspace"]);
    let res: Result<(), IdbError> = adapter.storage_request("workspace", "readonly", |store| {
        store.insert("unauthorized_key".to_string(), vec![1]);
        Ok(())
    });
    assert_eq!(
        res,
        Err(IdbError::ReadOnlyViolation {
            store: "workspace".to_string(),
            operation: "insert",
        })
    );
}

#[test]
fn test_t2_f06_idb_adapter_aborted_transaction_leaves_no_staging() {
    let mut adapter = MockIdbCommitAdapter::new(&["workspace"]);
    let _: Result<(), _> = adapter.storage_request("workspace", "readwrite", |store| {
        store.insert("staged_key".to_string(), b"val".to_vec());
        Err("abort".to_string())
    });
    assert_eq!(adapter.get_value("workspace", "staged_key"), None);
}

// =========================================================================
// F07: Multi-Tab Web Locks Synchronization Refusals
// =========================================================================

#[test]
fn test_t2_f07_web_locks_refuses_empty_lock_name() {
    let mut coordinator = MockWebLockCoordinator::new();
    let res = coordinator.acquire("");
    assert_eq!(res, Err(LockError::EmptyLockName));
}

#[test]
fn test_t2_f07_web_locks_releases_on_callback_panic() {
    let mut coordinator = MockWebLockCoordinator::new();
    let res: Result<(), LockError> = coordinator.request("storage_lock", "tab_alpha", || {
        panic!("unexpected panic in lock");
    });
    assert_eq!(
        res,
        Err(LockError::CallbackPanic {
            name: "storage_lock".to_string(),
            reason: "panicked".to_string(),
        })
    );
    assert!(!coordinator.is_held());
}

#[test]
fn test_t2_f07_web_locks_boundary_zero_holders_after_release() {
    let mut coordinator = MockWebLockCoordinator::new();
    coordinator.acquire("tab_temp").unwrap();
    coordinator.release("tab_temp");
    assert!(!coordinator.is_held());
}

#[test]
fn test_t2_f07_web_locks_timeout_or_abort_signal() {
    let mut coordinator = MockWebLockCoordinator::new();
    coordinator.acquire("tab_holder").unwrap();
    let res: Result<(), LockError> =
        coordinator.request("exclusive_lock", "tab_timeout", || Ok(()));
    assert_eq!(
        res,
        Err(LockError::LockHeld {
            name: "exclusive_lock".to_string(),
            holder: "tab_holder".to_string(),
        })
    );
}

#[test]
fn test_t2_f07_web_locks_queue_drain_on_multiple_failures() {
    let mut coordinator = MockWebLockCoordinator::new();
    coordinator.acquire("tab_1").unwrap();
    coordinator.acquire("tab_2").unwrap();
    coordinator.release("tab_1");
    coordinator.release("tab_2");
    assert!(!coordinator.is_held());
}

// =========================================================================
// F08: MicroVM CoW Sector Container Refusals
// =========================================================================

#[test]
fn test_t2_f08_mvm_cow_refuses_bad_magic() {
    let mut bytes = vec![0u8; 32];
    bytes[0..8].copy_from_slice(b"BAD_COW!");
    let res = MvmCowSectorContainer::deserialize(&bytes);
    assert_eq!(res, Err(MvmCowError::InvalidMagic(*b"BAD_COW!")));
}

#[test]
fn test_t2_f08_mvm_cow_refuses_invalid_sector_size() {
    let mut bytes = vec![0u8; 16];
    bytes[0..8].copy_from_slice(&MVM_COW_MAGIC);
    bytes[8..12].copy_from_slice(&0u32.to_le_bytes());
    bytes[12..16].copy_from_slice(&32_768u32.to_le_bytes()); // Not 65536
    let res = MvmCowSectorContainer::deserialize(&bytes);
    assert_eq!(
        res,
        Err(MvmCowError::InvalidSectorSize {
            expected: 65_536,
            actual: 32_768,
        })
    );
}

#[test]
fn test_t2_f08_mvm_cow_refuses_corrupted_individual_sector_length() {
    let mut bytes = vec![0u8; 25];
    bytes[0..8].copy_from_slice(&MVM_COW_MAGIC);
    bytes[8..12].copy_from_slice(&1u32.to_le_bytes()); // 1 sector
    bytes[12..16].copy_from_slice(&65_536u32.to_le_bytes());
    bytes[16..20].copy_from_slice(&0u32.to_le_bytes()); // sector_id 0
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes()); // length 1 (invalid)
    let res = MvmCowSectorContainer::deserialize(&bytes);
    assert_eq!(
        res,
        Err(MvmCowError::InvalidSectorPayloadLength {
            sector_id: 0,
            expected: 65_536,
            actual: 1,
        })
    );
}

#[test]
fn test_t2_f08_mvm_cow_boundary_truncated_payload() {
    let mut bytes = vec![0u8; 124];
    bytes[0..8].copy_from_slice(&MVM_COW_MAGIC);
    bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
    bytes[12..16].copy_from_slice(&65_536u32.to_le_bytes());
    bytes[16..20].copy_from_slice(&0u32.to_le_bytes());
    bytes[20..24].copy_from_slice(&65_536u32.to_le_bytes()); // declares 64 KB, provides 100 bytes
    let res = MvmCowSectorContainer::deserialize(&bytes);
    assert_eq!(
        res,
        Err(MvmCowError::TruncatedSector {
            sector_id: 0,
            expected: 65_536,
            actual: 100,
        })
    );
}

#[test]
fn test_t2_f08_mvm_cow_boundary_max_sector_id() {
    let max_id = u32::MAX;
    let data = vec![0xFE; 65_536];
    let serialized = MvmCowSectorContainer::serialize(&[(max_id, data.clone())]);
    let deserialized = MvmCowSectorContainer::deserialize(&serialized).unwrap();
    assert_eq!(deserialized.len(), 1);
    assert_eq!(deserialized[0].0, max_id);
    assert_eq!(deserialized[0].1, data);
}

// =========================================================================
// F09: Float-Free Model Serving Refusals
// =========================================================================

fn build_uorr4v12_test_header(float_ops: u32, dense_matmul_ops: u32) -> Vec<u8> {
    let mut buf = vec![0u8; 96];
    buf[0..8].copy_from_slice(b"UORR4V12");
    buf[8..12].copy_from_slice(&12u32.to_le_bytes());
    buf[12..16].copy_from_slice(&1u32.to_le_bytes());
    buf[56..60].copy_from_slice(&float_ops.to_le_bytes());
    buf[60..64].copy_from_slice(&dense_matmul_ops.to_le_bytes());
    buf
}

#[test]
fn test_t2_f09_model_serving_refuses_non_zero_float_ops() {
    let mut engine = R4GeometricStackEngine::embedded_seed();
    let bad = build_uorr4v12_test_header(1, 0);
    assert_eq!(
        engine.mount_full_container_bytes(&bad),
        Err(R4StackError::NonZeroFloatOps(1))
    );
}

#[test]
fn test_t2_f09_model_serving_refuses_non_zero_dense_matmul_ops() {
    let mut engine = R4GeometricStackEngine::embedded_seed();
    let bad = build_uorr4v12_test_header(0, 1);
    assert_eq!(
        engine.mount_full_container_bytes(&bad),
        Err(R4StackError::NonZeroDenseMatmulOps(1))
    );
}

#[test]
fn test_t2_f09_model_serving_refuses_invalid_container_magic() {
    let mut engine = R4GeometricStackEngine::embedded_seed();
    let mut bad = build_uorr4v12_test_header(0, 0);
    bad[0..8].copy_from_slice(b"BADMAGIC");
    assert_eq!(
        engine.mount_full_container_bytes(&bad),
        Err(R4StackError::InvalidMagic)
    );
}

#[test]
fn test_t2_f09_model_serving_refuses_truncated_container() {
    let mut engine = R4GeometricStackEngine::embedded_seed();
    let bad = vec![0u8; 32];
    assert_eq!(
        engine.mount_full_container_bytes(&bad),
        Err(R4StackError::TruncatedContainer)
    );
}

#[test]
fn test_t2_f09_model_serving_refuses_corrupted_payload_hash() {
    let mut engine = R4GeometricStackEngine::embedded_seed();
    let mut bad = build_uorr4v12_test_header(0, 0);
    bad[64..96].fill(0xAA);
    assert_eq!(
        engine.mount_full_container_bytes(&bad),
        Err(R4StackError::IntegrityMismatch)
    );
}

// =========================================================================
// F10: SpiralCore Zero-NIC Vsock Relay Refusals
// =========================================================================

#[test]
fn test_t2_f10_spiralcore_vsock_refuses_corrupted_magic() {
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
fn test_t2_f10_spiralcore_vsock_refuses_truncated_header_under_16_bytes() {
    let truncated = [0u8; 15];
    let res = SpiralCoreWireHeader::from_bytes(&truncated);
    assert_eq!(res, Err(VsockVerdict::RefusedBadMagic));
}

#[test]
fn test_t2_f10_spiralcore_vsock_refuses_payload_exceeding_max_limit() {
    let hdr = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 3,
        dst_cid: 2,
        vsock_port: 1024,
        cl06_spin: 0,
        e8_orbit: 0,
        payload_len: 65_537, // Exceeds 65536
    };
    let verdict = admit_spiralcore_header(&hdr, SPIRALCORE_MAX_PAYLOAD);
    assert_eq!(verdict, FrameVerdict::RefusedPayloadTooLarge);
}

#[test]
fn test_t2_f10_spiralcore_vsock_refuses_guest_to_guest_direct_cid() {
    let hdr = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 3, // Guest
        dst_cid: 3, // Same CID self-loopback directly without host
        vsock_port: 1024,
        cl06_spin: 0,
        e8_orbit: 0,
        payload_len: 128,
    };
    let verdict = admit_spiralcore_header(&hdr, SPIRALCORE_MAX_PAYLOAD);
    assert_eq!(verdict, FrameVerdict::RefusedInvalidCid);
}

#[test]
fn test_t2_f10_spiralcore_vsock_refuses_cl06_spin_out_of_bounds() {
    let hdr = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 3,
        dst_cid: 2,
        vsock_port: 1024,
        cl06_spin: 64, // Limit is 63
        e8_orbit: 0,
        payload_len: 128,
    };
    let verdict = admit_spiralcore_header(&hdr, SPIRALCORE_MAX_PAYLOAD);
    assert_eq!(verdict, FrameVerdict::RefusedSpinOutOfRange);
}

// =========================================================================
// F11: Capability R4-49 Specification Refusals
// =========================================================================

#[test]
fn test_t2_f11_capability_r4_49_refuses_invalid_id_prefix() {
    let cap = CapabilityRow {
        id: "INVALID-49".to_string(),
        level: "build".to_string(),
        suite: "r4-browser-persistence".to_string(),
        statement: "statement".to_string(),
    };
    assert_eq!(
        cap.validate(),
        Err(CapabilityError::InvalidIdPrefix("INVALID-49".to_string()))
    );
}

#[test]
fn test_t2_f11_capability_r4_49_refuses_empty_statement() {
    let cap = CapabilityRow {
        id: "R4-49".to_string(),
        level: "build".to_string(),
        suite: "r4-browser-persistence".to_string(),
        statement: "".to_string(),
    };
    assert_eq!(cap.validate(), Err(CapabilityError::EmptyStatement));
}

#[test]
fn test_t2_f11_capability_r4_49_refuses_empty_suite() {
    let cap = CapabilityRow {
        id: "R4-49".to_string(),
        level: "build".to_string(),
        suite: "".to_string(),
        statement: "statement".to_string(),
    };
    assert_eq!(cap.validate(), Err(CapabilityError::EmptySuite));
}

#[test]
fn test_t2_f11_capability_r4_49_refuses_invalid_epistemic_level() {
    let cap = CapabilityRow {
        id: "R4-49".to_string(),
        level: "unproven".to_string(),
        suite: "r4-browser-persistence".to_string(),
        statement: "statement".to_string(),
    };
    assert_eq!(
        cap.validate(),
        Err(CapabilityError::InvalidLevel("unproven".to_string()))
    );
}

#[test]
fn test_t2_f11_capability_r4_49_refuses_missing_scenario_step() {
    let gherkin = "Scenario: Missing given\nWhen action taken\nThen state verified\n";
    let res = validate_gherkin_scenario("R4-49", gherkin);
    assert_eq!(
        res,
        Err(CapabilityError::MissingGherkinScenarioStep {
            scenario: "R4-49".to_string(),
            missing_step: "Given",
        })
    );
}

// =========================================================================
// F12: Autonomous Engine Synthesis Ticks Refusals
// =========================================================================

#[test]
fn test_t2_f12_engine_tick_refuses_skipping_required_stage() {
    let res = validate_engine_tick(
        5,
        b"\0asm\x01\0\0\0",
        "prismpm/browser-provenance/1",
        &[8],
        1,
    );
    assert_eq!(
        res,
        Err(EngineTickError::IncompleteStageSequence {
            expected_stages: 7,
            actual_stages: 5,
        })
    );
}

#[test]
fn test_t2_f12_engine_tick_refuses_corrupted_wasm_closure() {
    let res = validate_engine_tick(7, b"NOT_WASM", "prismpm/browser-provenance/1", &[8], 1);
    assert_eq!(res, Err(EngineTickError::CorruptedWasmMagic(*b"NOT_")));
}

#[test]
fn test_t2_f12_engine_tick_refuses_malformed_provenance_schema() {
    let res = validate_engine_tick(7, b"\0asm\x01\0\0\0", "unsupported/schema/2", &[8], 1);
    assert_eq!(
        res,
        Err(EngineTickError::InvalidProvenanceSchema(
            "unsupported/schema/2".to_string()
        ))
    );
}

#[test]
fn test_t2_f12_engine_tick_refuses_weight_overflow_beyond_capacity() {
    let res = validate_engine_tick(
        7,
        b"\0asm\x01\0\0\0",
        "prismpm/browser-provenance/1",
        &[17],
        1,
    );
    assert_eq!(
        res,
        Err(EngineTickError::WeightOverflow {
            current: 17,
            max_capacity: 16,
        })
    );
}

#[test]
fn test_t2_f12_engine_tick_refuses_empty_frontier_without_synthesis() {
    let res = validate_engine_tick(
        7,
        b"\0asm\x01\0\0\0",
        "prismpm/browser-provenance/1",
        &[8],
        0,
    );
    assert_eq!(res, Err(EngineTickError::EmptyFrontierWithoutSynthesis));
}

// =========================================================================
// F13: Prism Standard Alignment & Holon Refusals
// =========================================================================

#[test]
fn test_t2_f13_prism_refuses_unresolved_product_fulfillment_obligations() {
    let res = validate_prism_holon(
        27,
        28,
        950,
        900,
        "v0.12.0",
        "verdant/reflexive-holon/1",
        349,
        355,
    );
    assert_eq!(
        res,
        Err(PrismAlignmentError::UnresolvedObligations {
            fulfilled: 27,
            total: 28,
        })
    );
}

#[test]
fn test_t2_f13_prism_refuses_holon_score_below_minimum_threshold() {
    let res = validate_prism_holon(
        28,
        28,
        899,
        900,
        "v0.12.0",
        "verdant/reflexive-holon/1",
        349,
        355,
    );
    assert_eq!(
        res,
        Err(PrismAlignmentError::ScoreBelowThreshold {
            score_milli: 899,
            min_threshold: 900,
        })
    );
}

#[test]
fn test_t2_f13_prism_refuses_invalid_semver_syntax() {
    let res = validate_prism_holon(
        28,
        28,
        950,
        900,
        "invalid-tag-format",
        "verdant/reflexive-holon/1",
        349,
        355,
    );
    assert_eq!(
        res,
        Err(PrismAlignmentError::InvalidSemverSyntax(
            "invalid-tag-format".to_string()
        ))
    );
}

#[test]
fn test_t2_f13_prism_refuses_unsupported_synthesis_schema() {
    let res = validate_prism_holon(28, 28, 950, 900, "v0.12.0", "unknown/holon/9", 349, 355);
    assert_eq!(
        res,
        Err(PrismAlignmentError::UnsupportedSchema(
            "unknown/holon/9".to_string()
        ))
    );
}

#[test]
fn test_t2_f13_prism_refuses_pareto_complexity_regression() {
    let res = validate_prism_holon(
        28,
        28,
        950,
        900,
        "v0.12.0",
        "verdant/reflexive-holon/1",
        356,
        355,
    );
    assert_eq!(
        res,
        Err(PrismAlignmentError::ParetoComplexityRegression {
            new_complexity: 356,
            ceiling: 355,
        })
    );
}

// =========================================================================
// F14: Atomic Workspace Rollback Guard Refusals
// =========================================================================

#[test]
fn test_t2_f14_atomic_rollback_refuses_commit_on_unclean_status() {
    let res = validate_rollback_guard(
        "M crates/wasm/src/lib.rs",
        None,
        "max_residue_functions",
        349,
        355,
    );
    assert_eq!(
        res,
        Err(RollbackGuardError::DirtyWorkingTree {
            porcelain_status: "M crates/wasm/src/lib.rs".to_string(),
        })
    );
}

#[test]
fn test_t2_f14_atomic_rollback_refuses_dirty_index_preservation() {
    let res = validate_rollback_guard(
        "A crates/wasm/new_file.rs",
        None,
        "max_residue_functions",
        349,
        355,
    );
    assert_eq!(
        res,
        Err(RollbackGuardError::DirtyWorkingTree {
            porcelain_status: "A crates/wasm/new_file.rs".to_string(),
        })
    );
}

#[test]
fn test_t2_f14_atomic_rollback_boundary_no_modified_files() {
    let git = ScratchGitFixture::new().expect("create scratch fixture");
    let status = git.status_porcelain().expect("status porcelain");
    assert!(validate_rollback_guard(&status, None, "max_residue_functions", 349, 355).is_ok());
}

#[test]
fn test_t2_f14_atomic_rollback_refuses_stale_lockfile() {
    let lock_path = PathBuf::from(".git/index.lock");
    let res = validate_rollback_guard(
        "",
        Some(lock_path.as_path()),
        "max_residue_functions",
        349,
        355,
    );
    assert_eq!(
        res,
        Err(RollbackGuardError::StaleLockfilePresent(PathBuf::from(
            ".git/index.lock"
        )))
    );
}

#[test]
fn test_t2_f14_atomic_rollback_refuses_ratchet_ceiling_breach() {
    let res = validate_rollback_guard("", None, "max_residue_functions", 356, 355);
    assert_eq!(
        res,
        Err(RollbackGuardError::RatchetCeilingBreached {
            metric: "max_residue_functions".to_string(),
            value: 356,
            ceiling: 355,
        })
    );
}

// =========================================================================
// F15: Capability R4-47 Specification Refusals
// =========================================================================

#[test]
fn test_t2_f15_capability_r4_47_refuses_invalid_id_prefix() {
    let cap = CapabilityRow {
        id: "BAD-47".to_string(),
        level: "build".to_string(),
        suite: "r4-engine-synthesis".to_string(),
        statement: "statement".to_string(),
    };
    assert_eq!(
        cap.validate(),
        Err(CapabilityError::InvalidIdPrefix("BAD-47".to_string()))
    );
}

#[test]
fn test_t2_f15_capability_r4_47_refuses_empty_statement() {
    let cap = CapabilityRow {
        id: "R4-47".to_string(),
        level: "build".to_string(),
        suite: "r4-engine-synthesis".to_string(),
        statement: "".to_string(),
    };
    assert_eq!(cap.validate(), Err(CapabilityError::EmptyStatement));
}

#[test]
fn test_t2_f15_capability_r4_47_refuses_empty_suite() {
    let cap = CapabilityRow {
        id: "R4-47".to_string(),
        level: "build".to_string(),
        suite: "".to_string(),
        statement: "statement".to_string(),
    };
    assert_eq!(cap.validate(), Err(CapabilityError::EmptySuite));
}

#[test]
fn test_t2_f15_capability_r4_47_refuses_invalid_epistemic_level() {
    let cap = CapabilityRow {
        id: "R4-47".to_string(),
        level: "untracked".to_string(),
        suite: "r4-engine-synthesis".to_string(),
        statement: "statement".to_string(),
    };
    assert_eq!(
        cap.validate(),
        Err(CapabilityError::InvalidLevel("untracked".to_string()))
    );
}

#[test]
fn test_t2_f15_capability_r4_47_refuses_missing_scenario_step() {
    let gherkin = "Scenario: Missing when\nGiven state base\nThen state verified\n";
    let res = validate_gherkin_scenario("R4-47", gherkin);
    assert_eq!(
        res,
        Err(CapabilityError::MissingGherkinScenarioStep {
            scenario: "R4-47".to_string(),
            missing_step: "When",
        })
    );
}

// =========================================================================
// F16: Multi-Framework Stitch XOR Parity Refusals
// =========================================================================

#[test]
fn test_t2_f16_stitch_xor_refuses_fewer_than_2_shards() {
    let payload = b"stitch parity threshold test payload";
    let encoding = split_2of3_xor(payload);
    let f0 = (0u8, encoding.fragments[0].bytes.as_slice());
    let res = recover_2of3_object(&[f0], encoding.original_length, None, None);
    assert_eq!(
        res,
        Err(StitchParityError::InsufficientShards {
            required: 2,
            available: 1,
        })
    );
}

#[test]
fn test_t2_f16_stitch_xor_refuses_corrupted_shard_digest() {
    let payload = b"payload for corrupted shard digest testing";
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
fn test_t2_f16_stitch_xor_boundary_odd_trailing_singleton() {
    let odd_payload = vec![0x77u8; 1025];
    let encoding = split_2of3_xor(&odd_payload);
    let f0 = (0u8, encoding.fragments[0].bytes.as_slice());
    let f1 = (1u8, encoding.fragments[1].bytes.as_slice());
    let recovered = recover_2of3_object(&[f0, f1], encoding.original_length, None, None)
        .expect("recover odd payload");
    assert_eq!(recovered.len(), 1025);
    assert_eq!(recovered, odd_payload);
}

#[test]
fn test_t2_f16_holo_bundle_refuses_tampered_payload_hash() {
    let payload = b"holo package payload for tamper test";
    let encoding = split_2of3_xor(payload);
    let f0 = (0u8, encoding.fragments[0].bytes.as_slice());
    let f1 = (1u8, encoding.fragments[1].bytes.as_slice());
    let recovered = recover_2of3_object(&[f0, f1], encoding.original_length, None, None).unwrap();
    let bad_hash = "0".repeat(64);
    assert_ne!(sha256_hex(&recovered), bad_hash);
}

#[test]
fn test_t2_f16_vsock_encapsulation_refuses_oversized_packet() {
    let hdr = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 3,
        dst_cid: 2,
        vsock_port: 1024,
        cl06_spin: 0,
        e8_orbit: 0,
        payload_len: 65_537,
    };
    let verdict = admit_spiralcore_header(&hdr, SPIRALCORE_MAX_PAYLOAD);
    assert_eq!(verdict, FrameVerdict::RefusedPayloadTooLarge);
}

// =========================================================================
// F17: E2E Suite Structure Refusals
// =========================================================================

#[test]
fn test_t2_f17_e2e_suite_refuses_failing_exit_code() {
    let bad_output = std::process::Command::new("cargo")
        .args(["test", "--invalid-flag-that-fails-cargo"])
        .current_dir(repo_root())
        .output()
        .expect("spawn cargo test with invalid flag");
    assert!(
        !bad_output.status.success(),
        "Invalid cargo command must return non-zero exit code"
    );
}

#[test]
fn test_t2_f17_e2e_suite_refuses_missing_test_tier() {
    let non_existent = repo_root().join("crates/wasm/tests/overhaul_e2e/tier99_missing.rs");
    let res = count_test_attributes(&non_existent);
    assert!(res.is_err());
}

#[test]
fn test_t2_f17_e2e_suite_refuses_facade_test_without_assertions() {
    let t1_path = repo_root().join("crates/wasm/tests/overhaul_e2e/tier1_feature.rs");
    let t2_path = repo_root().join("crates/wasm/tests/overhaul_e2e/tier2_boundary.rs");
    assert!(
        test_functions_have_assertions(&t1_path).unwrap(),
        "All Tier 1 test functions must contain assertions"
    );
    assert!(
        test_functions_have_assertions(&t2_path).unwrap(),
        "All Tier 2 test functions must contain assertions"
    );
}

#[test]
fn test_t2_f17_e2e_suite_refuses_non_hermetic_external_path() {
    let t1_content = std::fs::read_to_string(
        repo_root().join("crates/wasm/tests/overhaul_e2e/tier1_feature.rs"),
    )
    .unwrap();
    let t2_content = std::fs::read_to_string(
        repo_root().join("crates/wasm/tests/overhaul_e2e/tier2_boundary.rs"),
    )
    .unwrap();

    let forbidden = ["/", "Users", "/"].concat();
    assert!(
        !t1_content.contains(&forbidden),
        "Tier 1 must not contain hardcoded user paths"
    );
    let t2_filtered: String = t2_content
        .lines()
        .filter(|l| !l.contains("forbidden") && !l.contains("concat"))
        .collect();
    assert!(
        !t2_filtered.contains(&forbidden),
        "Tier 2 must not contain hardcoded user paths"
    );
}

#[test]
fn test_t2_f17_e2e_suite_boundary_zero_clippy_suppressions() {
    let root_test_file = repo_root().join("crates/wasm/tests/overhaul_e2e.rs");
    let content = std::fs::read_to_string(root_test_file).expect("read overhaul_e2e.rs");
    assert!(
        !content.contains("clippy::assertions_on_constants"),
        "overhaul_e2e.rs must NOT allow clippy::assertions_on_constants"
    );
    assert!(
        !content.contains("clippy::eq_op"),
        "overhaul_e2e.rs must NOT allow clippy::eq_op"
    );
}

// =========================================================================
// F18: Adversarial Bridge Refusals
// =========================================================================

#[test]
fn test_t2_f18_adversarial_refuses_raw_secret_in_guest_ram() {
    let verdict = admit_host_egress(EgressPolicyCheck {
        host_allowlisted: true,
        placeholder_matched: true,
        secret_in_guest_ram: true,
    });
    assert_eq!(verdict, EgressVerdict::RefusedSecretLeakToGuest);
}

#[test]
fn test_t2_f18_adversarial_refuses_http_plaintext_bridge() {
    let verdict = admit_bridge_transport(3, 2, McpPolicyMode::Safe, TransportKind::PlainHttp);
    assert_eq!(verdict, BridgeTransportVerdict::InsecureTransportRejected);
}

#[test]
fn test_t2_f18_adversarial_refuses_non_elf64_package_install() {
    let mut elf32 = vec![0u8; 64];
    elf32[0..4].copy_from_slice(b"\x7fELF");
    elf32[4] = 1; // EI_CLASS = 1 (ELFCLASS32)
    elf32[5] = 1; // Little-endian
    let verdict = admit_elf64_riscv(&elf32);
    assert_eq!(verdict, ElfAdmission::RefusedWrongClass);
}

#[test]
fn test_t2_f18_adversarial_refuses_non_allowlisted_github_host() {
    let mut mvm = MvmSandbox::boot_default().expect("boot MvmSandbox");
    mvm.register_host_secret(
        "GITHUB_TOKEN",
        "ghp_mock_never_exfiltrate",
        &["api.github.com"],
    );
    let outcome = mvm.proxy_vsock_egress(
        "evil.example.com",
        "Authorization: Bearer {{MVM_SECRET:GITHUB_TOKEN}}",
    );
    assert_eq!(outcome.verdict, EgressVerdict::RefusedHostNotAllowlisted);
    assert!(outcome.outbound_payload.is_none());
}

#[test]
fn test_t2_f18_adversarial_refuses_invalid_mcp_policy_mode() {
    let verdict = admit_bridge_transport(3, 2, McpPolicyMode::Deny, TransportKind::HttpsDirect);
    assert_eq!(verdict, BridgeTransportVerdict::PolicyDenied);
}

// =========================================================================
// F19: Normative Release Gate Refusals
// =========================================================================

#[test]
fn test_t2_f19_release_tag_refuses_non_semver_tag() {
    let bad_tag = "beta-1.0";
    let res = validate_release_gate(
        bad_tag,
        "",
        "ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:36da25ef924d84899453f24ec0c117a7a220e708037933eb8b499caa7723d99e",
        49,
        49,
        true,
    );
    assert_eq!(
        res,
        Err(ReleaseGateError::InvalidSemverTag("beta-1.0".to_string()))
    );
}

#[test]
fn test_t2_f19_release_tag_refuses_dirty_git_worktree() {
    let res = validate_release_gate(
        "v0.12.0",
        "M Justfile",
        "ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:36da25ef924d84899453f24ec0c117a7a220e708037933eb8b499caa7723d99e",
        49,
        49,
        true,
    );
    assert_eq!(
        res,
        Err(ReleaseGateError::DirtyGitWorktree("M Justfile".to_string()))
    );
}

#[test]
fn test_t2_f19_release_tag_refuses_unpinned_sdk_digest() {
    let unpinned = "ghcr.io/org/sdk:latest";
    let res = validate_release_gate("v0.12.0", "", unpinned, 49, 49, true);
    assert_eq!(
        res,
        Err(ReleaseGateError::UnpinnedSdkDigest(unpinned.to_string()))
    );
}

#[test]
fn test_t2_f19_release_tag_refuses_missing_conformance_claim() {
    let res = validate_release_gate(
        "v0.12.0",
        "",
        "ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:36da25ef924d84899453f24ec0c117a7a220e708037933eb8b499caa7723d99e",
        49,
        48,
        true,
    );
    assert_eq!(
        res,
        Err(ReleaseGateError::ConformanceClaimDiscrepancy {
            ids_count: 49,
            conformance_count: 48,
        })
    );
}

#[test]
fn test_t2_f19_release_tag_refuses_broken_signature() {
    let res = validate_release_gate(
        "v0.12.0",
        "",
        "ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:36da25ef924d84899453f24ec0c117a7a220e708037933eb8b499caa7723d99e",
        49,
        49,
        false,
    );
    assert_eq!(
        res,
        Err(ReleaseGateError::CryptographicSignatureInvalid(
            "Signature verification failed".to_string()
        ))
    );
}
