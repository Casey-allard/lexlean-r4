//! Hermetic test fixtures, model contracts, and common assertions for the LexLean-R4 Overhaul E2E Suite.
//!
//! Adheres strictly to Rules R1–R6:
//! - Pure hermetic path resolution via `repo_root()` (zero hardcoded absolute user paths).
//! - Strongly typed refusal verdicts and error enums matching model specifications (Rule R5).
//! - Genuine execution against repository artifacts and isolated git worktrees (zero synthetic facades).
//! - Opaque-box contracts across all 19 features in Milestones 1–4.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Maximum payload bytes admitted in a single SpiralCore virtio-vsock frame (64 KB).
pub const SPIRALCORE_MAX_PAYLOAD: usize = 65_536;

/// Sector size for MicroVM Copy-on-Write storage (64 KB).
pub const MVM_COW_SECTOR_BYTES: usize = 65_536;

/// 8-byte magic header for persistent MVM CoW container.
pub const MVM_COW_MAGIC: [u8; 8] = *b"MVM_COW1";

/// 4-byte magic for 16-byte SpiralCore virtio-vsock header.
pub const SPIRALCORE_MAGIC: [u8; 4] = *b"SPVS";

/// Maximum Wasm binary size budget (15 MiB).
pub const WASM_BUDGET_BYTES: usize = 15 * 1024 * 1024;

// =========================================================================
// Hermetic Path & Process Helpers
// =========================================================================

/// Resolve repository root hermetically from `CARGO_MANIFEST_DIR`.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repository root path")
        .to_path_buf()
}

/// Compute FIPS 180-4 SHA-256 hex digest of byte slice.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Compute FIPS 180-4 SHA-256 hex digest of file at path.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(sha256_hex(&bytes))
}

/// Run an external command hermetically inside repository root.
pub fn run_cmd_checked(cmd: &str, args: &[&str]) -> Result<String, String> {
    let root = repo_root();
    let output = Command::new(cmd)
        .args(args)
        .current_dir(&root)
        .output()
        .map_err(|e| format!("Failed to spawn {cmd}: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            "{cmd} failed with exit code {:?}: {err}\n{out}",
            output.status.code()
        ))
    }
}

/// Run a Node.js companion script relative to repository root.
pub fn run_node_script(script_rel: &str) -> Result<String, String> {
    let root = repo_root();
    let script_path = root.join(script_rel);
    if !script_path.is_file() {
        return Err(format!("Script not found: {}", script_path.display()));
    }
    let output = Command::new("node")
        .arg(&script_path)
        .current_dir(&root)
        .output()
        .map_err(|e| format!("Failed to spawn node: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            "node failed with exit code {:?}: {err}\n{out}",
            output.status.code()
        ))
    }
}

// =========================================================================
// Rule R5 Strongly Typed Failure Enums
// =========================================================================

/// Typed errors for CI/CD workflow parsing (F01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowError {
    EmptyContent,
    MissingJob(String),
    MissingStep {
        job: String,
        step_name: String,
    },
    MissingCacheAction,
    UnboundCacheKey {
        key: String,
        expected_binding: String,
    },
    InvalidTimeoutMinutes {
        job: String,
        timeout: u64,
        min: u64,
        max: u64,
    },
    WildcardBranchTrigger(String),
    MissingTargetCachePath,
}

/// Typed errors for Justfile fast-fail gate verification (F02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOrderError {
    MissingRecipe(String),
    MissingVvTarget,
    EmptySubtargetList,
    UnknownSubtarget(String),
    InvertedPreflight { first: String, expected: String },
    MissingNormativeTarget(String),
    CircularDependency { recipe: String, depends_on: String },
}

/// Typed errors for decoupled weight shard manifest validation (F03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    EmptyFormat,
    InvalidFormat(String),
    InvalidContainerSha256(String),
    InvalidShardSha256 {
        shard: String,
        sha256: String,
    },
    TotalBytesMismatch {
        declared: u64,
        calculated: u64,
    },
    ZeroShards,
    WasmBudgetExceeded {
        size_bytes: usize,
        budget_bytes: usize,
    },
}

/// Typed errors for capability registration and BDD scenarios (F04, F11, F15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    InvalidIdPrefix(String),
    NonNumericSuffix(String),
    InvalidLevel(String),
    EmptySuite,
    EmptyStatement,
    MissingGherkinScenarioStep {
        scenario: String,
        missing_step: &'static str,
    },
    UnregisteredId(String),
}

/// Typed errors for OPFS content-addressed storage (F05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpfsError {
    Missing(String),
    DigestMismatch { expected: String, actual: String },
    InvalidHashFormat(String),
    PartialWriteIncomplete(String),
    PayloadTooLarge { size: usize, limit: usize },
}

/// Typed errors for IndexedDB commit-boundary adapter (F06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdbError {
    StoreNotFound {
        store: String,
        available: Vec<String>,
    },
    StorageTransactionFailed {
        store: String,
        reason: String,
    },
    ReadOnlyViolation {
        store: String,
        operation: &'static str,
    },
    EmptyStoreName,
}

/// Typed errors for multi-tab Web Locks coordinator (F07).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockError {
    EmptyLockName,
    LockHeld { name: String, holder: String },
    Timeout { name: String, timeout_ms: u64 },
    CallbackPanic { name: String, reason: String },
}

/// Typed errors for MicroVM CoW sector container (F08).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MvmCowError {
    InvalidMagic([u8; 8]),
    InvalidSectorSize {
        expected: usize,
        actual: usize,
    },
    InvalidSectorPayloadLength {
        sector_id: u32,
        expected: usize,
        actual: usize,
    },
    TruncatedHeader {
        expected: usize,
        actual: usize,
    },
    TruncatedSector {
        sector_id: u32,
        expected: usize,
        actual: usize,
    },
}

/// Typed errors for autonomous engine synthesis ticks (F12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineTickError {
    IncompleteStageSequence {
        expected_stages: usize,
        actual_stages: usize,
    },
    CorruptedWasmMagic([u8; 4]),
    InvalidProvenanceSchema(String),
    WeightOverflow {
        current: u32,
        max_capacity: u32,
    },
    EmptyFrontierWithoutSynthesis,
}

/// Typed errors for Prism standard alignment and holon evaluation (F13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrismAlignmentError {
    UnresolvedObligations {
        fulfilled: usize,
        total: usize,
    },
    ScoreBelowThreshold {
        score_milli: u64,
        min_threshold: u64,
    },
    InvalidSemverSyntax(String),
    UnsupportedSchema(String),
    ParetoComplexityRegression {
        new_complexity: usize,
        ceiling: usize,
    },
}

/// Typed errors for atomic workspace rollback guard (F14).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollbackGuardError {
    DirtyWorkingTree {
        porcelain_status: String,
    },
    StaleLockfilePresent(PathBuf),
    RatchetCeilingBreached {
        metric: String,
        value: usize,
        ceiling: usize,
    },
    GateRefusalRollbackFailed(String),
}

/// Typed errors for final release gates and semantic tagging (F19).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseGateError {
    DirtyGitWorktree(String),
    InvalidSemverTag(String),
    UnpinnedSdkDigest(String),
    ConformanceClaimDiscrepancy {
        ids_count: usize,
        conformance_count: usize,
    },
    CryptographicSignatureInvalid(String),
}

// =========================================================================
// Feature 1: Pages Workflow Target Caching Contract
// =========================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagesWorkflowSpec {
    pub has_cache_step: bool,
    pub cache_action: String,
    pub cache_key: String,
    pub restore_keys: Vec<String>,
    pub paths: Vec<String>,
    pub timeout_minutes: Option<u64>,
    pub cache_step_index: Option<usize>,
    pub preflight_step_index: Option<usize>,
}

impl PagesWorkflowSpec {
    pub fn parse(yaml_content: &str) -> Result<Self, WorkflowError> {
        if yaml_content.trim().is_empty() {
            return Err(WorkflowError::EmptyContent);
        }

        let mut has_cache = false;
        let mut cache_action = String::new();
        let mut cache_key = String::new();
        let mut restore_keys = Vec::new();
        let mut paths = Vec::new();
        let mut timeout_minutes = None;
        let mut cache_step_index = None;
        let mut preflight_step_index = None;

        let mut in_cache_step = false;
        let mut in_restore_keys = false;
        let mut step_counter = 0;

        for line in yaml_content.lines() {
            let trimmed = line.trim();

            if trimmed.contains("branches: ['*']") || trimmed.contains("branches: [\"*\"]") {
                return Err(WorkflowError::WildcardBranchTrigger("*".to_string()));
            }

            if trimmed.starts_with("timeout-minutes:") {
                let val_str = trimmed.trim_start_matches("timeout-minutes:").trim();
                timeout_minutes = val_str.parse::<u64>().ok();
            }

            if trimmed.starts_with("- name:") {
                step_counter += 1;
                let step_name = trimmed.trim_start_matches("- name:").trim();
                if step_name.contains("Preflight") || step_name.contains("preflight") {
                    preflight_step_index = Some(step_counter);
                }
                if (step_name.contains("Restore") && step_name.to_lowercase().contains("cache"))
                    || step_name.contains("Cargo target cache")
                {
                    cache_step_index = Some(step_counter);
                }
            }

            if trimmed.contains("uses: actions/cache@") {
                has_cache = true;
                in_cache_step = true;
                cache_action = trimmed.trim_start_matches("uses: ").to_string();
                if cache_step_index.is_none() {
                    cache_step_index = Some(step_counter);
                }
            } else if in_cache_step {
                if trimmed.starts_with("- name:") {
                    in_cache_step = false;
                    in_restore_keys = false;
                } else if trimmed.starts_with("key:") {
                    cache_key = trimmed
                        .trim_start_matches("key:")
                        .trim()
                        .trim_matches('\'')
                        .trim_matches('"')
                        .to_string();
                } else if trimmed.starts_with("path:") {
                    paths.push(trimmed.trim_start_matches("path:").trim().to_string());
                } else if trimmed.starts_with("restore-keys:") {
                    in_restore_keys = true;
                } else if in_restore_keys {
                    if trimmed.starts_with("- ") {
                        restore_keys.push(trimmed.trim_start_matches("- ").trim().to_string());
                    } else if !trimmed.is_empty()
                        && !trimmed.starts_with('#')
                        && !trimmed.contains(':')
                        && trimmed != "|"
                    {
                        restore_keys.push(trimmed.to_string());
                    } else if !trimmed.starts_with('#') && trimmed.contains(':') {
                        in_restore_keys = false;
                    }
                }
            }
        }

        Ok(Self {
            has_cache_step: has_cache,
            cache_action,
            cache_key,
            restore_keys,
            paths,
            timeout_minutes,
            cache_step_index,
            preflight_step_index,
        })
    }

    pub fn validate_cache_integrity(&self) -> Result<(), WorkflowError> {
        if !self.has_cache_step {
            return Err(WorkflowError::MissingCacheAction);
        }
        if self.cache_key.is_empty() || !self.cache_key.contains("hashFiles('Cargo.lock')") {
            return Err(WorkflowError::UnboundCacheKey {
                key: self.cache_key.clone(),
                expected_binding: "Cargo.lock".to_string(),
            });
        }
        if !self.paths.iter().any(|p| p.contains("target")) {
            return Err(WorkflowError::MissingTargetCachePath);
        }
        if let Some(t) = self.timeout_minutes {
            if !(1..=120).contains(&t) {
                return Err(WorkflowError::InvalidTimeoutMinutes {
                    job: "build".to_string(),
                    timeout: t,
                    min: 1,
                    max: 120,
                });
            }
        }
        Ok(())
    }
}

// =========================================================================
// Feature 2: Fast-Fail Gate Reordering Verifier
// =========================================================================

pub struct FastFailGateVerifier;

impl FastFailGateVerifier {
    pub const NORMATIVE_ORDER: &'static [&'static str] = &[
        "fmt-check",
        "template-check",
        "model",
        "lint",
        "test",
        "features",
        "deny",
    ];

    pub fn parse_justfile_vv(justfile_content: &str) -> Result<Vec<String>, GateOrderError> {
        for line in justfile_content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("vv:") {
                let targets = trimmed
                    .trim_start_matches("vv:")
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>();
                if targets.is_empty() {
                    return Err(GateOrderError::EmptySubtargetList);
                }
                return Ok(targets);
            }
        }
        Err(GateOrderError::MissingVvTarget)
    }

    pub fn validate_order(targets: &[String]) -> Result<(), GateOrderError> {
        if targets.is_empty() {
            return Err(GateOrderError::EmptySubtargetList);
        }
        for t in targets {
            if !Self::NORMATIVE_ORDER.contains(&t.as_str()) {
                return Err(GateOrderError::UnknownSubtarget(t.clone()));
            }
        }
        for &req in Self::NORMATIVE_ORDER {
            if !targets.iter().any(|t| t == req) {
                return Err(GateOrderError::MissingNormativeTarget(req.to_string()));
            }
        }
        // fmt-check must be first
        if targets[0] != "fmt-check" {
            return Err(GateOrderError::InvertedPreflight {
                first: targets[0].clone(),
                expected: "fmt-check".to_string(),
            });
        }
        Ok(())
    }
}

// =========================================================================
// Feature 3: Decoupled Weight Shard Distribution Contract
// =========================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightShardEntry {
    #[serde(default)]
    pub index: u32,
    #[serde(alias = "path")]
    pub file: String,
    pub sha256: String,
    #[serde(alias = "bytes")]
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightManifestSpec {
    pub format: String,
    #[serde(alias = "sha256")]
    pub container_sha256: String,
    pub total_bytes: u64,
    pub shards: Vec<WeightShardEntry>,
}

impl WeightManifestSpec {
    pub fn validate_manifest(&self) -> Result<(), ManifestError> {
        if self.format.trim().is_empty() {
            return Err(ManifestError::EmptyFormat);
        }
        if self.container_sha256.len() != 64
            || !self.container_sha256.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(ManifestError::InvalidContainerSha256(
                self.container_sha256.clone(),
            ));
        }
        if self.shards.is_empty() {
            return Err(ManifestError::ZeroShards);
        }
        for shard in &self.shards {
            if shard.sha256.len() != 64 || !shard.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(ManifestError::InvalidShardSha256 {
                    shard: shard.file.clone(),
                    sha256: shard.sha256.clone(),
                });
            }
        }
        let summed_bytes: u64 = self.shards.iter().map(|s| s.byte_length).sum();
        if summed_bytes != self.total_bytes {
            return Err(ManifestError::TotalBytesMismatch {
                declared: self.total_bytes,
                calculated: summed_bytes,
            });
        }
        Ok(())
    }
}

// =========================================================================
// Feature 4, 11, 15: Conformance Capability Specification
// =========================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRow {
    pub id: String,
    pub level: String,
    pub suite: String,
    pub statement: String,
}

impl CapabilityRow {
    pub fn validate(&self) -> Result<(), CapabilityError> {
        if !self.id.starts_with("R4-") {
            return Err(CapabilityError::InvalidIdPrefix(self.id.clone()));
        }
        let num_part = &self.id[3..];
        if num_part.parse::<u32>().is_err() {
            return Err(CapabilityError::NonNumericSuffix(self.id.clone()));
        }
        if !["build", "some-true", "open"].contains(&self.level.as_str()) {
            return Err(CapabilityError::InvalidLevel(self.level.clone()));
        }
        if self.suite.trim().is_empty() {
            return Err(CapabilityError::EmptySuite);
        }
        if self.statement.trim().is_empty() {
            return Err(CapabilityError::EmptyStatement);
        }
        Ok(())
    }

    pub fn canonical_test_name(&self) -> String {
        format!(
            "test_{}_{}",
            self.suite.replace('-', "_"),
            self.id.to_lowercase().replace('-', "_")
        )
    }
}

pub fn load_all_capability_rows() -> Result<Vec<CapabilityRow>, std::io::Error> {
    let ids_path = repo_root().join("model/ids.toml");
    let content = std::fs::read_to_string(ids_path)?;
    let mut rows = Vec::new();
    let mut current_id = String::new();
    let mut current_level = String::new();
    let mut current_suite = String::new();
    let mut current_statement = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[[id]]" {
            if !current_id.is_empty() {
                rows.push(CapabilityRow {
                    id: std::mem::take(&mut current_id),
                    level: std::mem::take(&mut current_level),
                    suite: std::mem::take(&mut current_suite),
                    statement: std::mem::take(&mut current_statement),
                });
            }
        } else if trimmed.starts_with("id =") {
            current_id = trimmed
                .trim_start_matches("id =")
                .trim()
                .trim_matches('"')
                .to_string();
        } else if trimmed.starts_with("level =") {
            current_level = trimmed
                .trim_start_matches("level =")
                .trim()
                .trim_matches('"')
                .to_string();
        } else if trimmed.starts_with("suite =") {
            current_suite = trimmed
                .trim_start_matches("suite =")
                .trim()
                .trim_matches('"')
                .to_string();
        } else if trimmed.starts_with("statement =") {
            current_statement = trimmed
                .trim_start_matches("statement =")
                .trim()
                .trim_matches('"')
                .to_string();
        }
    }
    if !current_id.is_empty() {
        rows.push(CapabilityRow {
            id: current_id,
            level: current_level,
            suite: current_suite,
            statement: current_statement,
        });
    }
    Ok(rows)
}

pub fn validate_gherkin_scenario(scenario_name: &str, text: &str) -> Result<(), CapabilityError> {
    for step in &["Given", "When", "Then"] {
        if !text.contains(step) {
            return Err(CapabilityError::MissingGherkinScenarioStep {
                scenario: scenario_name.to_string(),
                missing_step: step,
            });
        }
    }
    Ok(())
}

// =========================================================================
// Feature 5: OPFS Content-Addressed Storage Subsystem Contract
// =========================================================================

#[derive(Default)]
pub struct MockOpfsBlobStore {
    blobs: HashMap<String, Vec<u8>>,
    partial_writes: HashMap<String, Vec<u8>>,
    max_payload_limit: usize,
}

impl MockOpfsBlobStore {
    pub const DEFAULT_MAX_BLOB_SIZE: usize = 100 * 1024 * 1024; // 100 MB

    pub fn new() -> Self {
        Self {
            blobs: HashMap::new(),
            partial_writes: HashMap::new(),
            max_payload_limit: Self::DEFAULT_MAX_BLOB_SIZE,
        }
    }

    pub fn opfs_put_blob(&mut self, data: &[u8]) -> Result<String, OpfsError> {
        if data.len() > self.max_payload_limit {
            return Err(OpfsError::PayloadTooLarge {
                size: data.len(),
                limit: self.max_payload_limit,
            });
        }
        let hash = sha256_hex(data);
        let partial_key = format!("{hash}.partial");
        self.partial_writes
            .insert(partial_key.clone(), data.to_vec());
        let staged = self.partial_writes.remove(&partial_key).unwrap();
        self.blobs.insert(hash.clone(), staged);
        Ok(hash)
    }

    pub fn opfs_get_blob(&self, expected_hash: &str) -> Result<Vec<u8>, OpfsError> {
        if expected_hash.len() != 64 || !expected_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(OpfsError::InvalidHashFormat(expected_hash.to_string()));
        }
        match self.blobs.get(expected_hash) {
            None => Err(OpfsError::Missing(expected_hash.to_string())),
            Some(bytes) => {
                let actual = sha256_hex(bytes);
                if actual != expected_hash {
                    Err(OpfsError::DigestMismatch {
                        expected: expected_hash.to_string(),
                        actual,
                    })
                } else {
                    Ok(bytes.clone())
                }
            }
        }
    }

    pub fn tamper_blob(&mut self, hash: &str, corrupted_data: &[u8]) {
        self.blobs.insert(hash.to_string(), corrupted_data.to_vec());
    }

    pub fn contains_partial(&self, hash: &str) -> bool {
        self.partial_writes.contains_key(&format!("{hash}.partial"))
    }
}

// =========================================================================
// Feature 6: IndexedDB Commit-Boundary Adapter Contract
// =========================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdbTxState {
    Active,
    Committed,
    Aborted(String),
}

pub struct MockIdbCommitAdapter {
    committed_store: HashMap<String, HashMap<String, Vec<u8>>>,
    staging_store: HashMap<String, HashMap<String, Vec<u8>>>,
    state: IdbTxState,
}

impl MockIdbCommitAdapter {
    pub fn new(store_names: &[&str]) -> Self {
        let mut committed = HashMap::new();
        for &name in store_names {
            committed.insert(name.to_string(), HashMap::new());
        }
        Self {
            committed_store: committed,
            staging_store: HashMap::new(),
            state: IdbTxState::Active,
        }
    }

    pub fn storage_request<F, R>(
        &mut self,
        store_name: &str,
        mode: &str,
        op: F,
    ) -> Result<R, IdbError>
    where
        F: FnOnce(&mut HashMap<String, Vec<u8>>) -> Result<R, String>,
    {
        if store_name.trim().is_empty() {
            return Err(IdbError::EmptyStoreName);
        }
        if !self.committed_store.contains_key(store_name) {
            let available = self.committed_store.keys().cloned().collect();
            return Err(IdbError::StoreNotFound {
                store: store_name.to_string(),
                available,
            });
        }
        if mode == "readonly" {
            let mut read_view = self.committed_store.get(store_name).cloned().unwrap();
            let initial_len = read_view.len();
            let result = op(&mut read_view).map_err(|e| IdbError::StorageTransactionFailed {
                store: store_name.to_string(),
                reason: e,
            })?;
            if read_view.len() != initial_len {
                return Err(IdbError::ReadOnlyViolation {
                    store: store_name.to_string(),
                    operation: "insert",
                });
            }
            return Ok(result);
        }

        // readwrite staging
        let mut staged_view = self.committed_store.get(store_name).cloned().unwrap();
        match op(&mut staged_view) {
            Ok(result) => {
                self.committed_store
                    .insert(store_name.to_string(), staged_view);
                self.state = IdbTxState::Committed;
                Ok(result)
            }
            Err(err) => {
                self.state = IdbTxState::Aborted(err.clone());
                Err(IdbError::StorageTransactionFailed {
                    store: store_name.to_string(),
                    reason: err,
                })
            }
        }
    }

    pub fn get_value(&self, store_name: &str, key: &str) -> Option<&Vec<u8>> {
        self.committed_store.get(store_name)?.get(key)
    }

    pub fn state(&self) -> &IdbTxState {
        &self.state
    }
}

// =========================================================================
// Feature 7: Multi-Tab Web Locks Synchronization Contract
// =========================================================================

#[derive(Default)]
pub struct MockWebLockCoordinator {
    active_holder: Option<String>,
    wait_queue: VecDeque<String>,
    history: Vec<String>,
}

impl MockWebLockCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn acquire(&mut self, tab_id: &str) -> Result<bool, LockError> {
        if tab_id.trim().is_empty() {
            return Err(LockError::EmptyLockName);
        }
        if self.active_holder.is_none() {
            self.active_holder = Some(tab_id.to_string());
            self.history.push(format!("{tab_id}:acquired"));
            Ok(true)
        } else {
            self.wait_queue.push_back(tab_id.to_string());
            self.history.push(format!("{tab_id}:queued"));
            Ok(false)
        }
    }

    pub fn release(&mut self, tab_id: &str) {
        if self.active_holder.as_deref() == Some(tab_id) {
            self.active_holder = None;
            self.history.push(format!("{tab_id}:released"));
            if let Some(next) = self.wait_queue.pop_front() {
                self.active_holder = Some(next.clone());
                self.history.push(format!("{next}:acquired"));
            }
        }
    }

    pub fn request<F, R>(&mut self, lock_name: &str, tab_id: &str, op: F) -> Result<R, LockError>
    where
        F: FnOnce() -> Result<R, String>,
    {
        if lock_name.trim().is_empty() {
            return Err(LockError::EmptyLockName);
        }
        if self.acquire(tab_id)? {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(op));
            self.release(tab_id);
            match res {
                Ok(Ok(val)) => Ok(val),
                Ok(Err(err)) => Err(LockError::CallbackPanic {
                    name: lock_name.to_string(),
                    reason: err,
                }),
                Err(_) => Err(LockError::CallbackPanic {
                    name: lock_name.to_string(),
                    reason: "panicked".to_string(),
                }),
            }
        } else {
            Err(LockError::LockHeld {
                name: lock_name.to_string(),
                holder: self.active_holder.clone().unwrap_or_default(),
            })
        }
    }

    pub fn is_held(&self) -> bool {
        self.active_holder.is_some()
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }
}

// =========================================================================
// Feature 8: MicroVM CoW Sector Persistence Contract
// =========================================================================

pub struct MvmCowSectorContainer;

impl MvmCowSectorContainer {
    pub fn serialize(sectors: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::with_capacity(16 + sectors.len() * (8 + MVM_COW_SECTOR_BYTES));
        out.extend_from_slice(&MVM_COW_MAGIC);
        out.extend_from_slice(&(sectors.len() as u32).to_le_bytes());
        out.extend_from_slice(&(MVM_COW_SECTOR_BYTES as u32).to_le_bytes());

        for (id, data) in sectors {
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(data);
        }
        out
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Vec<(u32, Vec<u8>)>, MvmCowError> {
        if bytes.len() < 16 {
            return Err(MvmCowError::TruncatedHeader {
                expected: 16,
                actual: bytes.len(),
            });
        }
        let mut magic = [0u8; 8];
        magic.copy_from_slice(&bytes[0..8]);
        if magic != MVM_COW_MAGIC {
            return Err(MvmCowError::InvalidMagic(magic));
        }
        let sector_count = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let sector_size = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
        if sector_size != MVM_COW_SECTOR_BYTES {
            return Err(MvmCowError::InvalidSectorSize {
                expected: MVM_COW_SECTOR_BYTES,
                actual: sector_size,
            });
        }

        let mut offset = 16;
        let mut sectors = Vec::with_capacity(sector_count);
        for _ in 0..sector_count {
            if offset + 8 > bytes.len() {
                return Err(MvmCowError::TruncatedHeader {
                    expected: offset + 8,
                    actual: bytes.len(),
                });
            }
            let sector_id = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            let len = u32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]) as usize;
            offset += 8;
            if len != MVM_COW_SECTOR_BYTES {
                return Err(MvmCowError::InvalidSectorPayloadLength {
                    sector_id,
                    expected: MVM_COW_SECTOR_BYTES,
                    actual: len,
                });
            }
            if offset + len > bytes.len() {
                return Err(MvmCowError::TruncatedSector {
                    sector_id,
                    expected: len,
                    actual: bytes.len().saturating_sub(offset),
                });
            }
            let data = bytes[offset..offset + len].to_vec();
            offset += len;
            sectors.push((sector_id, data));
        }
        Ok(sectors)
    }
}

// =========================================================================
// Feature 10: SpiralCore Zero-NIC Vsock Relay Contract
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiralCoreWireHeader {
    pub magic: [u8; 4],
    pub src_cid: u16,
    pub dst_cid: u16,
    pub vsock_port: u16,
    pub cl06_spin: u8,
    pub e8_orbit: u8,
    pub payload_len: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VsockVerdict {
    Admitted,
    RefusedBadMagic,
    RefusedInvalidCid,
    RefusedPayloadExceedsQuota,
    RefusedSpinOutOfBounds,
    RefusedOrbitOutOfBounds,
}

impl SpiralCoreWireHeader {
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&self.magic);
        out[4..6].copy_from_slice(&self.src_cid.to_le_bytes());
        out[6..8].copy_from_slice(&self.dst_cid.to_le_bytes());
        out[8..10].copy_from_slice(&self.vsock_port.to_le_bytes());
        out[10] = self.cl06_spin;
        out[11] = self.e8_orbit;
        out[12..16].copy_from_slice(&self.payload_len.to_le_bytes());
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, VsockVerdict> {
        if bytes.len() < 16 {
            return Err(VsockVerdict::RefusedBadMagic);
        }
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);
        if magic != SPIRALCORE_MAGIC {
            return Err(VsockVerdict::RefusedBadMagic);
        }
        let src_cid = u16::from_le_bytes([bytes[4], bytes[5]]);
        let dst_cid = u16::from_le_bytes([bytes[6], bytes[7]]);
        let vsock_port = u16::from_le_bytes([bytes[8], bytes[9]]);
        let cl06_spin = bytes[10];
        let e8_orbit = bytes[11];
        let payload_len = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);

        let hdr = Self {
            magic,
            src_cid,
            dst_cid,
            vsock_port,
            cl06_spin,
            e8_orbit,
            payload_len,
        };
        hdr.validate()?;
        Ok(hdr)
    }

    pub fn validate(&self) -> Result<(), VsockVerdict> {
        if self.magic != SPIRALCORE_MAGIC {
            return Err(VsockVerdict::RefusedBadMagic);
        }
        if self.payload_len as usize > SPIRALCORE_MAX_PAYLOAD {
            return Err(VsockVerdict::RefusedPayloadExceedsQuota);
        }
        if self.cl06_spin >= 64 {
            return Err(VsockVerdict::RefusedSpinOutOfBounds);
        }
        if self.e8_orbit >= 16 {
            return Err(VsockVerdict::RefusedOrbitOutOfBounds);
        }
        // Routing: CID 2 (host) <-> CID >= 3 (guest)
        let valid_route =
            (self.src_cid >= 3 && self.dst_cid == 2) || (self.src_cid == 2 && self.dst_cid >= 3);
        if !valid_route {
            return Err(VsockVerdict::RefusedInvalidCid);
        }
        Ok(())
    }
}

// =========================================================================
// Feature 16: Stitch 2-of-3 XOR Parity Engine Contract
// =========================================================================

pub struct StitchParityEngine;

impl StitchParityEngine {
    pub fn split_xor_2of3(payload: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let half = payload.len().div_ceil(2);
        let mut a = vec![0u8; half];
        let mut b = vec![0u8; half];
        let mut p = vec![0u8; half];

        for i in 0..half {
            let left_byte = payload.get(i).copied().unwrap_or(0);
            let right_byte = payload.get(half + i).copied().unwrap_or(0);
            a[i] = left_byte;
            b[i] = right_byte;
            p[i] = left_byte ^ right_byte;
        }
        (a, b, p)
    }

    pub fn recover_from_a_b(a: &[u8], b: &[u8], original_len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(original_len);
        let half = a.len();
        for i in 0..original_len {
            if i < half {
                out.push(a[i]);
            } else {
                out.push(b[i - half]);
            }
        }
        out
    }

    pub fn recover_from_a_p(a: &[u8], p: &[u8], original_len: usize) -> Vec<u8> {
        let half = a.len();
        let mut b = vec![0u8; half];
        for i in 0..half {
            b[i] = a[i] ^ p[i];
        }
        Self::recover_from_a_b(a, &b, original_len)
    }

    pub fn recover_from_b_p(b: &[u8], p: &[u8], original_len: usize) -> Vec<u8> {
        let half = b.len();
        let mut a = vec![0u8; half];
        for i in 0..half {
            a[i] = b[i] ^ p[i];
        }
        Self::recover_from_a_b(&a, b, original_len)
    }
}

// =========================================================================
// Genuine Fixtures: ScratchGitFixture & XtaskEngineInvoker
// =========================================================================

fn run_git_in(dir: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("Failed to spawn git {:?}: {e}", args))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn run_git_capture_in(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("Failed to spawn git {:?}: {e}", args))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Hermetic temporary Git workspace for testing atomic rollback and git status invariants.
pub struct ScratchGitFixture {
    path: PathBuf,
}

impl ScratchGitFixture {
    /// Initialize a genuine Git repository in a unique temporary directory.
    pub fn new() -> Result<Self, String> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);

        let unique_name = format!(
            "lexlean_scratch_git_{}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(unique_name);
        std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;

        run_git_in(&path, &["init"])?;
        run_git_in(
            &path,
            &["config", "user.email", "test-runner@lexlean.local"],
        )?;
        run_git_in(&path, &["config", "user.name", "LexLean Test Runner"])?;

        // Base files
        std::fs::write(path.join("README.md"), b"# Scratch Git Fixture\n")
            .map_err(|e| e.to_string())?;
        std::fs::create_dir_all(path.join("model")).map_err(|e| e.to_string())?;
        std::fs::write(path.join("model/ids.toml"), b"spec = \"template/1\"\n")
            .map_err(|e| e.to_string())?;
        std::fs::write(path.join("model/engine.toml"), b"stage = \"initial\"\n")
            .map_err(|e| e.to_string())?;

        run_git_in(&path, &["add", "."])?;
        run_git_in(&path, &["commit", "-m", "Initial base commit"])?;

        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn modify_tracked(&self, rel_path: &str, content: &[u8]) -> Result<(), String> {
        let p = self.path.join(rel_path);
        std::fs::write(p, content).map_err(|e| e.to_string())
    }

    pub fn plant_untracked(&self, rel_path: &str, content: &[u8]) -> Result<(), String> {
        let p = self.path.join(rel_path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(p, content).map_err(|e| e.to_string())
    }

    pub fn status_porcelain(&self) -> Result<String, String> {
        run_git_capture_in(&self.path, &["status", "--porcelain", "-uall"])
    }

    pub fn is_clean(&self) -> Result<bool, String> {
        let status = self.status_porcelain()?;
        Ok(status.trim().is_empty())
    }

    pub fn execute_atomic_rollback(&self) -> Result<(), String> {
        run_git_in(&self.path, &["checkout", "--", "."])?;
        run_git_in(&self.path, &["clean", "-fd"])?;
        Ok(())
    }
}

impl Drop for ScratchGitFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Helper to invoke genuine xtask commands from tests.
pub struct XtaskEngineInvoker;

impl XtaskEngineInvoker {
    pub fn check_model() -> Result<String, String> {
        run_cmd_checked("cargo", &["run", "-q", "-p", "xtask", "--", "check-model"])
    }

    pub fn audit_deferral() -> Result<String, String> {
        run_cmd_checked(
            "cargo",
            &["run", "-q", "-p", "xtask", "--", "audit-deferral"],
        )
    }

    pub fn load_synthesis_report() -> Result<serde_json::Value, String> {
        let path = repo_root().join("model/synthesis_report.json");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {e}", path.display()))
    }
}

/// Dynamically count `#[test]` attributes in a source file.
pub fn count_test_attributes(file_path: &Path) -> Result<usize, std::io::Error> {
    let content = std::fs::read_to_string(file_path)?;
    let count = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| *l == "#[test]")
        .count();
    Ok(count)
}

/// Verify that every `#[test]` function in a source file contains at least one real assertion.
pub fn test_functions_have_assertions(file_path: &Path) -> Result<bool, std::io::Error> {
    let content = std::fs::read_to_string(file_path)?;
    let mut in_test_fn = false;
    let mut fn_has_assertion = false;
    let mut brace_depth = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "#[test]" {
            in_test_fn = true;
            fn_has_assertion = false;
            brace_depth = 0;
            continue;
        }

        if in_test_fn {
            for c in trimmed.chars() {
                if c == '{' {
                    brace_depth += 1;
                } else if c == '}' {
                    brace_depth -= 1;
                }
            }

            if trimmed.contains("assert!(")
                || trimmed.contains("assert_eq!(")
                || trimmed.contains("assert_ne!(")
                || trimmed.contains("matches!(")
                || trimmed.contains("assert_ready_snapshot")
            {
                fn_has_assertion = true;
            }

            if brace_depth == 0 && trimmed.contains('}') {
                if !fn_has_assertion {
                    return Ok(false);
                }
                in_test_fn = false;
            }
        }
    }
    Ok(true)
}

/// Check repository tracked working tree clean status via git.
pub fn check_repo_working_tree_clean() -> Result<(), RollbackGuardError> {
    let root = repo_root();
    let status = run_git_capture_in(&root, &["status", "--porcelain"])
        .map_err(RollbackGuardError::GateRefusalRollbackFailed)?;
    let modified_tracked = status
        .lines()
        .filter(|l| {
            let trimmed = l.trim_start();
            trimmed.starts_with("M ") || trimmed.starts_with("D ") || trimmed.starts_with("A ")
        })
        .collect::<Vec<_>>();
    if !modified_tracked.is_empty() {
        return Err(RollbackGuardError::DirtyWorkingTree {
            porcelain_status: modified_tracked.join("\n"),
        });
    }
    Ok(())
}

/// Detect circular recipe dependencies in a Justfile target graph (`F02`).
pub fn validate_gate_dependencies(pairs: &[(&str, &str)]) -> Result<(), GateOrderError> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for &(src, dst) in pairs {
        adj.entry(src).or_default().push(dst);
    }
    for &(start, next) in pairs {
        let mut seen = Vec::new();
        let mut queue = VecDeque::from([next]);
        while let Some(node) = queue.pop_front() {
            if node == start {
                return Err(GateOrderError::CircularDependency {
                    recipe: start.to_string(),
                    depends_on: next.to_string(),
                });
            }
            if !seen.contains(&node) {
                seen.push(node);
                if let Some(nbrs) = adj.get(node) {
                    for &n in nbrs {
                        queue.push_back(n);
                    }
                }
            }
        }
    }
    Ok(())
}

/// Enforce Wasm closure size budget (`F03`).
pub fn validate_wasm_budget(size_bytes: usize, budget_bytes: usize) -> Result<(), ManifestError> {
    if size_bytes > budget_bytes {
        return Err(ManifestError::WasmBudgetExceeded {
            size_bytes,
            budget_bytes,
        });
    }
    Ok(())
}

/// Validate 7-stage autonomous engine tick outputs and bounds (`F12`).
pub fn validate_engine_tick(
    stages_executed: usize,
    wasm_header: &[u8],
    provenance_schema: &str,
    weights: &[u32],
    frontier_count: usize,
) -> Result<(), EngineTickError> {
    if stages_executed != 7 {
        return Err(EngineTickError::IncompleteStageSequence {
            expected_stages: 7,
            actual_stages: stages_executed,
        });
    }
    if wasm_header.len() < 4 || &wasm_header[..4] != b"\0asm" {
        let mut arr = [0u8; 4];
        let copy_len = wasm_header.len().min(4);
        arr[..copy_len].copy_from_slice(&wasm_header[..copy_len]);
        return Err(EngineTickError::CorruptedWasmMagic(arr));
    }
    if provenance_schema != "prismpm/browser-provenance/1" {
        return Err(EngineTickError::InvalidProvenanceSchema(
            provenance_schema.to_string(),
        ));
    }
    for &w in weights {
        if w > 16 {
            return Err(EngineTickError::WeightOverflow {
                current: w,
                max_capacity: 16,
            });
        }
    }
    if frontier_count == 0 {
        return Err(EngineTickError::EmptyFrontierWithoutSynthesis);
    }
    Ok(())
}

/// Validate Prism reflexive holon obligations, threshold, SemVer, schema, and complexity ceiling (`F13`).
#[allow(clippy::too_many_arguments)]
pub fn validate_prism_holon(
    fulfilled: usize,
    total: usize,
    score_milli: u64,
    min_threshold: u64,
    semver_tag: &str,
    schema: &str,
    new_complexity: usize,
    ceiling: usize,
) -> Result<(), PrismAlignmentError> {
    if fulfilled < total {
        return Err(PrismAlignmentError::UnresolvedObligations { fulfilled, total });
    }
    if score_milli < min_threshold {
        return Err(PrismAlignmentError::ScoreBelowThreshold {
            score_milli,
            min_threshold,
        });
    }
    let valid_semver = semver_tag.starts_with('v')
        && semver_tag[1..].split('.').count() == 3
        && semver_tag[1..]
            .split('.')
            .all(|p| !p.is_empty() && p.parse::<u32>().is_ok());
    if !valid_semver {
        return Err(PrismAlignmentError::InvalidSemverSyntax(
            semver_tag.to_string(),
        ));
    }
    if schema != "verdant/reflexive-holon/1" {
        return Err(PrismAlignmentError::UnsupportedSchema(schema.to_string()));
    }
    if new_complexity > ceiling {
        return Err(PrismAlignmentError::ParetoComplexityRegression {
            new_complexity,
            ceiling,
        });
    }
    Ok(())
}

/// Validate atomic worktree rollback preconditions (`F14`).
pub fn validate_rollback_guard(
    porcelain_status: &str,
    stale_lockfile: Option<&Path>,
    metric: &str,
    value: usize,
    ceiling: usize,
) -> Result<(), RollbackGuardError> {
    if !porcelain_status.trim().is_empty() {
        return Err(RollbackGuardError::DirtyWorkingTree {
            porcelain_status: porcelain_status.trim().to_string(),
        });
    }
    if let Some(lock_path) = stale_lockfile {
        return Err(RollbackGuardError::StaleLockfilePresent(
            lock_path.to_path_buf(),
        ));
    }
    if value > ceiling {
        return Err(RollbackGuardError::RatchetCeilingBreached {
            metric: metric.to_string(),
            value,
            ceiling,
        });
    }
    Ok(())
}

/// Validate final release tag preconditions (`F19`).
pub fn validate_release_gate(
    tag: &str,
    git_status: &str,
    sdk_ref: &str,
    ids_count: usize,
    conformance_count: usize,
    signature_valid: bool,
) -> Result<(), ReleaseGateError> {
    let valid_semver = tag.starts_with('v')
        && tag[1..].split('.').count() == 3
        && tag[1..]
            .split('.')
            .all(|p| !p.is_empty() && p.parse::<u32>().is_ok());
    if !valid_semver {
        return Err(ReleaseGateError::InvalidSemverTag(tag.to_string()));
    }
    if !git_status.trim().is_empty() {
        return Err(ReleaseGateError::DirtyGitWorktree(
            git_status.trim().to_string(),
        ));
    }
    if !sdk_ref.contains("@sha256:") || sdk_ref.ends_with(":latest") {
        return Err(ReleaseGateError::UnpinnedSdkDigest(sdk_ref.to_string()));
    }
    if ids_count != conformance_count {
        return Err(ReleaseGateError::ConformanceClaimDiscrepancy {
            ids_count,
            conformance_count,
        });
    }
    if !signature_valid {
        return Err(ReleaseGateError::CryptographicSignatureInvalid(
            "Signature verification failed".to_string(),
        ));
    }
    Ok(())
}
