//! Pure-Rust Stitch 2-of-3 XOR parity encoding, manifest validation, and custody recovery.
//!
//! Aligns `afflom/stitch` (`scheme = "xor-2-of-3"`, `threshold = 2`, `count = 3`) with the
//! `UORGLLM9` hosted weight shard layout (`gpu/weights/v10/manifest.stitch.json`, schema
//! `"stitch/xor-2-of-3/1"`). Every parity operation uses pure bytewise XOR and SHA-256
//! digest verification with typed negative errors (`R5`).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{self, Write as _};

/// Canonical schema identifier for `gpu/weights/v10/manifest.stitch.json`.
pub const STITCH_MANIFEST_SCHEMA: &str = "stitch/xor-2-of-3/1";

/// Canonical parity scheme identifier matching `afflom/stitch`.
pub const STITCH_XOR_2OF3_SCHEME: &str = "xor-2-of-3";

/// Typed failure modes for Stitch 2-of-3 parity encoding, manifest validation, and recovery (`R5`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StitchParityError {
    /// Manifest JSON could not be parsed or failed structural validation.
    InvalidManifest(String),
    /// Manifest declared an unsupported schema string.
    UnsupportedSchema(String),
    /// Manifest or custody descriptor declared a scheme other than `xor-2-of-3`.
    UnsupportedScheme(String),
    /// Threshold/count parameters were not `(threshold = 2, count = 3)`.
    InvalidThresholdConfig {
        /// Declared threshold count.
        threshold: u8,
        /// Declared total fragment count.
        count: u8,
    },
    /// Duplicate fragment index was supplied during 2-of-3 object recovery.
    DuplicateShardIndex {
        /// Repeated fragment index (`0..3`).
        index: u8,
    },
    /// Fragment index fell outside `0..3`.
    InvalidShardIndex {
        /// Supplied fragment index.
        index: u8,
        /// Configured fragment count.
        count: u8,
    },
    /// Fewer than 2 valid shards/fragments were available for recovery.
    InsufficientShards {
        /// Minimum shards required (`2`).
        required: usize,
        /// Shards supplied or verified cleanly.
        available: usize,
    },
    /// Supplied shard or fragment byte length disagreed with its specification.
    ShardLengthMismatch {
        /// Human-readable slot or fragment identifier.
        slot: String,
        /// Expected byte length.
        expected: usize,
        /// Actual byte length.
        actual: usize,
    },
    /// Supplied shard or fragment failed SHA-256 verification.
    ShardDigestMismatch {
        /// Human-readable slot or fragment identifier.
        slot: String,
        /// Expected lowercase hex SHA-256 digest.
        expected: String,
        /// Actual computed lowercase hex SHA-256 digest.
        actual: String,
    },
    /// Recovered payload length did not match declared original byte length.
    PayloadLengthMismatch {
        /// Expected payload length.
        expected: usize,
        /// Actual reconstructed length.
        actual: usize,
    },
    /// Recovered payload SHA-256 digest did not match declared object digest.
    PayloadDigestMismatch {
        /// Expected lowercase hex SHA-256 digest.
        expected: String,
        /// Actual computed lowercase hex SHA-256 digest.
        actual: String,
    },
}

impl StitchParityError {
    /// Stable machine-readable error code for Wasm, relay, and UI diagnostics.
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidManifest(_) => "StitchInvalidManifest",
            Self::UnsupportedSchema(_) => "StitchUnsupportedSchema",
            Self::UnsupportedScheme(_) => "StitchUnsupportedScheme",
            Self::InvalidThresholdConfig { .. } => "StitchInvalidThresholdConfig",
            Self::DuplicateShardIndex { .. } => "StitchDuplicateShardIndex",
            Self::InvalidShardIndex { .. } => "StitchInvalidShardIndex",
            Self::InsufficientShards { .. } => "StitchInsufficientShards",
            Self::ShardLengthMismatch { .. } => "StitchShardLengthMismatch",
            Self::ShardDigestMismatch { .. } => "StitchShardDigestMismatch",
            Self::PayloadLengthMismatch { .. } => "StitchPayloadLengthMismatch",
            Self::PayloadDigestMismatch { .. } => "StitchPayloadDigestMismatch",
        }
    }
}

impl fmt::Display for StitchParityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidManifest(msg) => write!(f, "invalid stitch manifest: {msg}"),
            Self::UnsupportedSchema(s) => write!(f, "unsupported stitch schema '{s}'"),
            Self::UnsupportedScheme(s) => write!(f, "unsupported stitch scheme '{s}'"),
            Self::InvalidThresholdConfig { threshold, count } => write!(
                f,
                "invalid threshold config ({threshold}-of-{count}), expected 2-of-3"
            ),
            Self::DuplicateShardIndex { index } => {
                write!(f, "duplicate stitch shard index {index}")
            }
            Self::InvalidShardIndex { index, count } => {
                write!(f, "stitch shard index {index} out of range 0..{count}")
            }
            Self::InsufficientShards {
                required,
                available,
            } => write!(
                f,
                "insufficient valid stitch shards: need {required}, have {available}"
            ),
            Self::ShardLengthMismatch {
                slot,
                expected,
                actual,
            } => write!(
                f,
                "stitch shard '{slot}' byte_length mismatch: expected {expected}, got {actual}"
            ),
            Self::ShardDigestMismatch {
                slot,
                expected,
                actual,
            } => write!(
                f,
                "stitch shard '{slot}' sha256 mismatch: expected {expected}, got {actual}"
            ),
            Self::PayloadLengthMismatch { expected, actual } => write!(
                f,
                "stitch reconstructed payload length mismatch: expected {expected}, got {actual}"
            ),
            Self::PayloadDigestMismatch { expected, actual } => write!(
                f,
                "stitch reconstructed payload sha256 mismatch: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for StitchParityError {}

/// Single shard or parity file metadata entry inside `manifest.stitch.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StitchShardEntry {
    /// Zero-based data shard index when referencing a `UORGLLM9` shard; `None` for parity files.
    #[serde(default)]
    pub index: Option<usize>,
    /// File basename relative to the shard manifest directory.
    pub file: String,
    /// Optional byte offset in the assembled `UORGLLM9` container.
    #[serde(default)]
    pub byte_offset: Option<usize>,
    /// Exact byte length of this shard or parity file.
    pub byte_length: usize,
    /// Lowercase 64-character hex SHA-256 digest of this shard's bytes.
    pub sha256: String,
}

/// One 2-of-3 XOR parity group pairing `left` and optional `right` with `parity = left ^ right`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StitchParityGroup {
    /// Zero-based group sequence number (`0..groups.len()`).
    pub group_index: usize,
    /// Even-indexed primary data shard (`shard_{2k}`).
    pub left: StitchShardEntry,
    /// Odd-indexed partner data shard (`shard_{2k+1}`), or `None` when total data shard count is odd.
    #[serde(default)]
    pub right: Option<StitchShardEntry>,
    /// XOR parity shard (`parity_{k:03}.bin`), right-padded with zeroes when `left`/`right` differ in size.
    pub parity: StitchShardEntry,
}

/// Top-level Stitch parity manifest (`gpu/weights/v10/manifest.stitch.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StitchManifest {
    /// Manifest schema (`"stitch/xor-2-of-3/1"`).
    pub schema: String,
    /// Custody scheme (`"xor-2-of-3"`).
    pub scheme: String,
    /// Required shard quorum (`2`).
    pub threshold: u8,
    /// Total shards per group (`3`).
    pub count: u8,
    /// Associated base shard manifest filename (usually `"manifest.json"`).
    pub base_manifest: String,
    /// Target model container format (`"UORGLLM9"`).
    pub format: String,
    /// Expected SHA-256 digest of the assembled container bytes.
    pub container_sha256: String,
    /// Total assembled container byte length.
    pub total_bytes: usize,
    /// Number of sequential data shards protected by `groups`.
    pub data_shard_count: usize,
    /// Parity groups ordered strictly by `group_index`.
    pub groups: Vec<StitchParityGroup>,
    /// Optional alternate HTTPS/origin base paths for multi-source shard fetching.
    #[serde(default)]
    pub mirrors: Vec<String>,
}

/// Single fragment emitted by `split_2of3_xor` matching `afflom/stitch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StitchObjectFragment {
    /// Fragment index: `0` = left half, `1` = padded right half, `2` = XOR parity.
    pub index: u8,
    /// Fragment payload bytes (`fragment_length` bytes).
    pub bytes: Vec<u8>,
    /// Lowercase hex SHA-256 digest of `bytes`.
    pub sha256: String,
}

/// Output of `split_2of3_xor` representing full 2-of-3 custody encoding of an object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StitchObjectEncoding {
    /// Scheme identifier (`"xor-2-of-3"`).
    pub scheme: String,
    /// Recovery quorum (`2`).
    pub threshold: u8,
    /// Fragment count (`3`).
    pub count: u8,
    /// Exact original byte length before splitting/padding.
    pub original_length: usize,
    /// Exact byte length of each of the 3 fragments.
    pub fragment_length: usize,
    /// Lowercase hex SHA-256 digest of the original payload.
    pub object_sha256: String,
    /// The 3 custody fragments `[left, right, parity]`.
    pub fragments: [StitchObjectFragment; 3],
}

/// Reconstructed data shards for a single `StitchParityGroup`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StitchPairRecoveryResult {
    /// Group index processed.
    pub group_index: usize,
    /// Which shard slot was reconstructed (`"none"`, `"left"`, or `"right"`).
    pub recovered_slot: String,
    /// Verified bytes for the left shard of this group.
    pub left_bytes: Vec<u8>,
    /// Verified bytes for the right shard of this group, if the group has a right shard.
    pub right_bytes: Option<Vec<u8>>,
    /// Verified SHA-256 digest of `left_bytes`.
    pub left_sha256: String,
    /// Verified SHA-256 digest of `right_bytes`, if present.
    pub right_sha256: Option<String>,
}

/// Aggregate Stitch 2-of-3 custody health telemetry exposed to Worker and UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StitchCustodyHealth {
    /// Schema string (`"stitch/xor-2-of-3/1"`).
    pub schema: String,
    /// Scheme string (`"xor-2-of-3"`).
    pub scheme: String,
    /// Total data shards declared.
    pub data_shard_count: usize,
    /// Total parity groups declared.
    pub group_count: usize,
    /// Groups where all constituent shards verified directly without parity recovery.
    pub intact_groups: usize,
    /// Data shard indices recovered via 2-of-3 XOR parity fallback.
    pub recovered_shards: Vec<usize>,
    /// Group indices where recovery failed or quorum dropped below 2-of-3.
    pub degraded_groups: Vec<usize>,
    /// True when zero groups are degraded and all data shards passed verification.
    pub custody_healthy: bool,
}

/// Compute lowercase 64-character hex SHA-256 digest for arbitrary bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

fn is_valid_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn validate_shard_entry(entry: &StitchShardEntry, slot: &str) -> Result<(), StitchParityError> {
    if entry.file.trim().is_empty() {
        return Err(StitchParityError::InvalidManifest(format!(
            "{slot}.file must not be empty"
        )));
    }
    if !is_valid_sha256_hex(&entry.sha256) {
        return Err(StitchParityError::InvalidManifest(format!(
            "{slot}.sha256 must be 64 hex characters"
        )));
    }
    Ok(())
}

/// Parse and strictly validate a `manifest.stitch.json` document (`R5`).
pub fn parse_and_validate_manifest(
    manifest_json: &str,
) -> Result<StitchManifest, StitchParityError> {
    let manifest: StitchManifest = serde_json::from_str(manifest_json.trim())
        .map_err(|e| StitchParityError::InvalidManifest(format!("invalid JSON: {e}")))?;

    if manifest.schema != STITCH_MANIFEST_SCHEMA {
        return Err(StitchParityError::UnsupportedSchema(manifest.schema));
    }
    if manifest.scheme != STITCH_XOR_2OF3_SCHEME {
        return Err(StitchParityError::UnsupportedScheme(manifest.scheme));
    }
    if manifest.threshold != 2 || manifest.count != 3 {
        return Err(StitchParityError::InvalidThresholdConfig {
            threshold: manifest.threshold,
            count: manifest.count,
        });
    }
    if !is_valid_sha256_hex(&manifest.container_sha256) {
        return Err(StitchParityError::InvalidManifest(
            "container_sha256 must be 64 hex characters".to_string(),
        ));
    }
    if manifest.data_shard_count == 0 || manifest.groups.is_empty() {
        return Err(StitchParityError::InvalidManifest(
            "data_shard_count and groups must be non-empty".to_string(),
        ));
    }

    let expected_groups = manifest.data_shard_count.div_ceil(2);
    if manifest.groups.len() != expected_groups {
        return Err(StitchParityError::InvalidManifest(format!(
            "expected {expected_groups} parity groups for {} shards, found {}",
            manifest.data_shard_count,
            manifest.groups.len()
        )));
    }

    let mut seen_shards = 0usize;
    let mut sum_bytes = 0usize;
    for (idx, g) in manifest.groups.iter().enumerate() {
        if g.group_index != idx {
            return Err(StitchParityError::InvalidManifest(format!(
                "group_index {0} out of sequence (expected {idx})",
                g.group_index
            )));
        }
        validate_shard_entry(&g.left, &format!("groups[{idx}].left"))?;
        if g.left.index != Some(seen_shards) {
            return Err(StitchParityError::InvalidManifest(format!(
                "groups[{idx}].left.index expected {seen_shards}"
            )));
        }
        seen_shards += 1;
        sum_bytes = sum_bytes.saturating_add(g.left.byte_length);

        let right_len = if let Some(ref right) = g.right {
            validate_shard_entry(right, &format!("groups[{idx}].right"))?;
            if right.index != Some(seen_shards) {
                return Err(StitchParityError::InvalidManifest(format!(
                    "groups[{idx}].right.index expected {seen_shards}"
                )));
            }
            seen_shards += 1;
            sum_bytes = sum_bytes.saturating_add(right.byte_length);
            right.byte_length
        } else {
            if idx + 1 != manifest.groups.len() {
                return Err(StitchParityError::InvalidManifest(format!(
                    "only the final parity group may omit right shard (group {idx})"
                )));
            }
            0
        };

        validate_shard_entry(&g.parity, &format!("groups[{idx}].parity"))?;
        let expected_parity_len = g.left.byte_length.max(right_len);
        if g.parity.byte_length != expected_parity_len {
            return Err(StitchParityError::ShardLengthMismatch {
                slot: format!("groups[{idx}].parity"),
                expected: expected_parity_len,
                actual: g.parity.byte_length,
            });
        }
    }

    if seen_shards != manifest.data_shard_count {
        return Err(StitchParityError::InvalidManifest(format!(
            "groups cover {seen_shards} data shards but data_shard_count={}",
            manifest.data_shard_count
        )));
    }
    if sum_bytes != manifest.total_bytes {
        return Err(StitchParityError::PayloadLengthMismatch {
            expected: manifest.total_bytes,
            actual: sum_bytes,
        });
    }

    Ok(manifest)
}

/// Encode an arbitrary byte payload into 3 Stitch `xor-2-of-3` custody fragments.
pub fn split_2of3_xor(payload: &[u8]) -> StitchObjectEncoding {
    let original_length = payload.len();
    let fragment_length = original_length.div_ceil(2);
    let mut left = vec![0u8; fragment_length];
    let mut right = vec![0u8; fragment_length];
    let mut parity = vec![0u8; fragment_length];

    let left_take = fragment_length.min(original_length);
    left[..left_take].copy_from_slice(&payload[..left_take]);
    if original_length > left_take {
        let right_slice = &payload[left_take..];
        right[..right_slice.len()].copy_from_slice(right_slice);
    }

    for i in 0..fragment_length {
        parity[i] = left[i] ^ right[i];
    }

    let left_sha = sha256_hex(&left);
    let right_sha = sha256_hex(&right);
    let parity_sha = sha256_hex(&parity);

    StitchObjectEncoding {
        scheme: STITCH_XOR_2OF3_SCHEME.to_string(),
        threshold: 2,
        count: 3,
        original_length,
        fragment_length,
        object_sha256: sha256_hex(payload),
        fragments: [
            StitchObjectFragment {
                index: 0,
                bytes: left,
                sha256: left_sha,
            },
            StitchObjectFragment {
                index: 1,
                bytes: right,
                sha256: right_sha,
            },
            StitchObjectFragment {
                index: 2,
                bytes: parity,
                sha256: parity_sha,
            },
        ],
    }
}

/// Recover an original payload from any 2 distinct Stitch `xor-2-of-3` fragments (`0`, `1`, `2`),
/// enforcing fragment bounds, duplicate checks, byte lengths, and SHA-256 digests (`R5`).
pub fn recover_2of3_object(
    fragments: &[(u8, &[u8])],
    original_length: usize,
    expected_object_sha256: Option<&str>,
    expected_fragment_sha256s: Option<&[&str; 3]>,
) -> Result<Vec<u8>, StitchParityError> {
    if fragments.len() < 2 {
        return Err(StitchParityError::InsufficientShards {
            required: 2,
            available: fragments.len(),
        });
    }

    let expected_frag_len = original_length.div_ceil(2);
    let mut slots: [Option<&[u8]>; 3] = [None, None, None];

    for &(idx, bytes) in fragments {
        if idx >= 3 {
            return Err(StitchParityError::InvalidShardIndex {
                index: idx,
                count: 3,
            });
        }
        if slots[idx as usize].is_some() {
            return Err(StitchParityError::DuplicateShardIndex { index: idx });
        }
        if bytes.len() != expected_frag_len {
            return Err(StitchParityError::ShardLengthMismatch {
                slot: format!("fragment[{idx}]"),
                expected: expected_frag_len,
                actual: bytes.len(),
            });
        }
        if let Some(digests) = expected_fragment_sha256s {
            let actual_sha = sha256_hex(bytes);
            if !actual_sha.eq_ignore_ascii_case(digests[idx as usize]) {
                return Err(StitchParityError::ShardDigestMismatch {
                    slot: format!("fragment[{idx}]"),
                    expected: digests[idx as usize].to_string(),
                    actual: actual_sha,
                });
            }
        }
        slots[idx as usize] = Some(bytes);
    }

    let (left, right): (Vec<u8>, Vec<u8>) = match (slots[0], slots[1], slots[2]) {
        (Some(f0), Some(f1), _) => (f0.to_vec(), f1.to_vec()),
        (Some(f0), None, Some(f2)) => {
            let rec_right: Vec<u8> = f0.iter().zip(f2.iter()).map(|(a, p)| a ^ p).collect();
            if let Some(digests) = expected_fragment_sha256s {
                let rec_sha = sha256_hex(&rec_right);
                if !rec_sha.eq_ignore_ascii_case(digests[1]) {
                    return Err(StitchParityError::ShardDigestMismatch {
                        slot: "fragment[1]".to_string(),
                        expected: digests[1].to_string(),
                        actual: rec_sha,
                    });
                }
            }
            (f0.to_vec(), rec_right)
        }
        (None, Some(f1), Some(f2)) => {
            let rec_left: Vec<u8> = f1.iter().zip(f2.iter()).map(|(b, p)| b ^ p).collect();
            if let Some(digests) = expected_fragment_sha256s {
                let rec_sha = sha256_hex(&rec_left);
                if !rec_sha.eq_ignore_ascii_case(digests[0]) {
                    return Err(StitchParityError::ShardDigestMismatch {
                        slot: "fragment[0]".to_string(),
                        expected: digests[0].to_string(),
                        actual: rec_sha,
                    });
                }
            }
            (rec_left, f1.to_vec())
        }
        _ => {
            return Err(StitchParityError::InsufficientShards {
                required: 2,
                available: fragments.len(),
            });
        }
    };

    let mut out = Vec::with_capacity(expected_frag_len.saturating_mul(2));
    out.extend_from_slice(&left);
    out.extend_from_slice(&right);
    if out.len() < original_length {
        return Err(StitchParityError::PayloadLengthMismatch {
            expected: original_length,
            actual: out.len(),
        });
    }
    out.truncate(original_length);

    if let Some(expected_sha) = expected_object_sha256 {
        let actual_sha = sha256_hex(&out);
        if !actual_sha.eq_ignore_ascii_case(expected_sha) {
            return Err(StitchParityError::PayloadDigestMismatch {
                expected: expected_sha.to_string(),
                actual: actual_sha,
            });
        }
    }

    Ok(out)
}

fn verify_group_slot(
    entry: &StitchShardEntry,
    bytes: &[u8],
    slot: &str,
) -> Result<(), StitchParityError> {
    if bytes.len() != entry.byte_length {
        return Err(StitchParityError::ShardLengthMismatch {
            slot: slot.to_string(),
            expected: entry.byte_length,
            actual: bytes.len(),
        });
    }
    let actual_sha = sha256_hex(bytes);
    if !actual_sha.eq_ignore_ascii_case(&entry.sha256) {
        return Err(StitchParityError::ShardDigestMismatch {
            slot: slot.to_string(),
            expected: entry.sha256.clone(),
            actual: actual_sha,
        });
    }
    Ok(())
}

fn xor_recover_padded(partner: &[u8], parity: &[u8], target_len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(target_len);
    for i in 0..target_len {
        let a = partner.get(i).copied().unwrap_or(0);
        let p = parity.get(i).copied().unwrap_or(0);
        out.push(a ^ p);
    }
    out
}

/// Strictly verify supplied shards for a `StitchParityGroup` and recover up to one omitted
/// data shard via 2-of-3 XOR parity. Any invalid/corrupted supplied slice causes an immediate
/// typed error (`R5`).
pub fn recover_2of3_pair(
    group: &StitchParityGroup,
    left_opt: Option<&[u8]>,
    right_opt: Option<&[u8]>,
    parity_opt: Option<&[u8]>,
) -> Result<StitchPairRecoveryResult, StitchParityError> {
    let gidx = group.group_index;
    if let Some(l) = left_opt {
        verify_group_slot(&group.left, l, &format!("group[{gidx}].left"))?;
    }
    if let (Some(ref r_spec), Some(r)) = (&group.right, right_opt) {
        verify_group_slot(r_spec, r, &format!("group[{gidx}].right"))?;
    }
    if let Some(p) = parity_opt {
        verify_group_slot(&group.parity, p, &format!("group[{gidx}].parity"))?;
    }
    heal_verified_pair_slots(group, left_opt, right_opt, parity_opt, None)
}

/// Verify up to 3 candidate slices (`left`, `right`, `parity`), discard up to one failed/corrupt
/// slice if at least 2 valid slices remain, and recover the full pair (`left`, optional `right`)
/// with strict SHA-256 verification.
pub fn heal_2of3_pair(
    group: &StitchParityGroup,
    left_opt: Option<&[u8]>,
    right_opt: Option<&[u8]>,
    parity_opt: Option<&[u8]>,
) -> Result<StitchPairRecoveryResult, StitchParityError> {
    let gidx = group.group_index;
    let mut first_err: Option<StitchParityError> = None;

    let valid_left = match left_opt {
        Some(l) => match verify_group_slot(&group.left, l, &format!("group[{gidx}].left")) {
            Ok(()) => Some(l),
            Err(e) => {
                first_err.get_or_insert(e);
                None
            }
        },
        None => None,
    };

    let valid_right = match (&group.right, right_opt) {
        (Some(r_spec), Some(r)) => {
            match verify_group_slot(r_spec, r, &format!("group[{gidx}].right")) {
                Ok(()) => Some(r),
                Err(e) => {
                    first_err.get_or_insert(e);
                    None
                }
            }
        }
        _ => None,
    };

    let valid_parity = match parity_opt {
        Some(p) => match verify_group_slot(&group.parity, p, &format!("group[{gidx}].parity")) {
            Ok(()) => Some(p),
            Err(e) => {
                first_err.get_or_insert(e);
                None
            }
        },
        None => None,
    };

    heal_verified_pair_slots(group, valid_left, valid_right, valid_parity, first_err)
}

fn heal_verified_pair_slots(
    group: &StitchParityGroup,
    valid_left: Option<&[u8]>,
    valid_right: Option<&[u8]>,
    valid_parity: Option<&[u8]>,
    prior_error: Option<StitchParityError>,
) -> Result<StitchPairRecoveryResult, StitchParityError> {
    let gidx = group.group_index;

    match &group.right {
        None => match (valid_left, valid_parity) {
            (Some(l), _) => Ok(StitchPairRecoveryResult {
                group_index: gidx,
                recovered_slot: "none".to_string(),
                left_bytes: l.to_vec(),
                right_bytes: None,
                left_sha256: group.left.sha256.clone(),
                right_sha256: None,
            }),
            (None, Some(p)) => {
                let rec = xor_recover_padded(&[], p, group.left.byte_length);
                verify_group_slot(&group.left, &rec, &format!("group[{gidx}].left"))?;
                Ok(StitchPairRecoveryResult {
                    group_index: gidx,
                    recovered_slot: "left".to_string(),
                    left_bytes: rec,
                    right_bytes: None,
                    left_sha256: group.left.sha256.clone(),
                    right_sha256: None,
                })
            }
            (None, None) => Err(
                prior_error.unwrap_or(StitchParityError::InsufficientShards {
                    required: 2,
                    available: 1,
                }),
            ),
        },
        Some(r_spec) => match (valid_left, valid_right, valid_parity) {
            (Some(l), Some(r), _) => Ok(StitchPairRecoveryResult {
                group_index: gidx,
                recovered_slot: "none".to_string(),
                left_bytes: l.to_vec(),
                right_bytes: Some(r.to_vec()),
                left_sha256: group.left.sha256.clone(),
                right_sha256: Some(r_spec.sha256.clone()),
            }),
            (None, Some(r), Some(p)) => {
                let rec_left = xor_recover_padded(r, p, group.left.byte_length);
                verify_group_slot(&group.left, &rec_left, &format!("group[{gidx}].left"))?;
                Ok(StitchPairRecoveryResult {
                    group_index: gidx,
                    recovered_slot: "left".to_string(),
                    left_bytes: rec_left,
                    right_bytes: Some(r.to_vec()),
                    left_sha256: group.left.sha256.clone(),
                    right_sha256: Some(r_spec.sha256.clone()),
                })
            }
            (Some(l), None, Some(p)) => {
                let rec_right = xor_recover_padded(l, p, r_spec.byte_length);
                verify_group_slot(r_spec, &rec_right, &format!("group[{gidx}].right"))?;
                Ok(StitchPairRecoveryResult {
                    group_index: gidx,
                    recovered_slot: "right".to_string(),
                    left_bytes: l.to_vec(),
                    right_bytes: Some(rec_right),
                    left_sha256: group.left.sha256.clone(),
                    right_sha256: Some(r_spec.sha256.clone()),
                })
            }
            _ => {
                let avail = usize::from(valid_left.is_some())
                    + usize::from(valid_right.is_some())
                    + usize::from(valid_parity.is_some());
                Err(
                    prior_error.unwrap_or(StitchParityError::InsufficientShards {
                        required: 2,
                        available: avail,
                    }),
                )
            }
        },
    }
}

/// Build an aggregate custody health report from a validated manifest and recovery outcomes.
pub fn assess_custody_health(
    manifest: &StitchManifest,
    recovered_shards: Vec<usize>,
    degraded_groups: Vec<usize>,
) -> StitchCustodyHealth {
    let mut recovered_group_count = 0usize;
    for g in &manifest.groups {
        let left_rec = g.left.index.is_some_and(|i| recovered_shards.contains(&i));
        let right_rec = g
            .right
            .as_ref()
            .and_then(|r| r.index)
            .is_some_and(|i| recovered_shards.contains(&i));
        if (left_rec || right_rec) && !degraded_groups.contains(&g.group_index) {
            recovered_group_count += 1;
        }
    }
    let intact_groups = manifest
        .groups
        .len()
        .saturating_sub(degraded_groups.len())
        .saturating_sub(recovered_group_count);
    StitchCustodyHealth {
        schema: manifest.schema.clone(),
        scheme: manifest.scheme.clone(),
        data_shard_count: manifest.data_shard_count,
        group_count: manifest.groups.len(),
        intact_groups,
        custody_healthy: degraded_groups.is_empty(),
        recovered_shards,
        degraded_groups,
    }
}

/// Generated parity shard blobs paired with their output filenames.
pub type StitchParityBlobs = Vec<(String, Vec<u8>)>;

/// Validated Stitch manifest paired with generated parity shard blobs.
pub type StitchManifestBundle = (StitchManifest, StitchParityBlobs);

/// Generate parity group bytes and a validated `StitchManifest` JSON document for any sequential
/// `UORGLLM9` shard set (`R4` custody tooling & tests).
pub fn build_pair_manifest(
    container_sha256: &str,
    data_shards: &[(String, usize, &[u8])],
    mirrors: Vec<String>,
) -> Result<StitchManifestBundle, StitchParityError> {
    if data_shards.is_empty() {
        return Err(StitchParityError::InvalidManifest(
            "cannot build stitch manifest from empty shard set".to_string(),
        ));
    }
    if !is_valid_sha256_hex(container_sha256) {
        return Err(StitchParityError::InvalidManifest(
            "container_sha256 must be 64 hex characters".to_string(),
        ));
    }
    let mut groups = Vec::with_capacity(data_shards.len().div_ceil(2));
    let mut parity_blobs = Vec::with_capacity(data_shards.len().div_ceil(2));
    let mut total_bytes = 0usize;

    for (gidx, chunk) in data_shards.chunks(2).enumerate() {
        let (left_file, left_off, left_bytes) = &chunk[0];
        total_bytes = total_bytes.saturating_add(left_bytes.len());
        let left_entry = StitchShardEntry {
            index: Some(gidx * 2),
            file: left_file.clone(),
            byte_offset: Some(*left_off),
            byte_length: left_bytes.len(),
            sha256: sha256_hex(left_bytes),
        };

        let (right_entry, right_slice): (Option<StitchShardEntry>, &[u8]) = if chunk.len() == 2 {
            let (rf, ro, rb) = &chunk[1];
            total_bytes = total_bytes.saturating_add(rb.len());
            (
                Some(StitchShardEntry {
                    index: Some(gidx * 2 + 1),
                    file: rf.clone(),
                    byte_offset: Some(*ro),
                    byte_length: rb.len(),
                    sha256: sha256_hex(rb),
                }),
                rb,
            )
        } else {
            (None, &[])
        };

        let parity_len = left_bytes.len().max(right_slice.len());
        let mut parity_bytes = Vec::with_capacity(parity_len);
        for i in 0..parity_len {
            let a = left_bytes.get(i).copied().unwrap_or(0);
            let b = right_slice.get(i).copied().unwrap_or(0);
            parity_bytes.push(a ^ b);
        }
        let parity_file = format!("parity_{gidx:03}.bin");
        let parity_entry = StitchShardEntry {
            index: None,
            file: parity_file.clone(),
            byte_offset: None,
            byte_length: parity_bytes.len(),
            sha256: sha256_hex(&parity_bytes),
        };
        parity_blobs.push((parity_file, parity_bytes));
        groups.push(StitchParityGroup {
            group_index: gidx,
            left: left_entry,
            right: right_entry,
            parity: parity_entry,
        });
    }

    let manifest = StitchManifest {
        schema: STITCH_MANIFEST_SCHEMA.to_string(),
        scheme: STITCH_XOR_2OF3_SCHEME.to_string(),
        threshold: 2,
        count: 3,
        base_manifest: "manifest.json".to_string(),
        format: "UORGLLM9".to_string(),
        container_sha256: container_sha256.to_string(),
        total_bytes,
        data_shard_count: data_shards.len(),
        groups,
        mirrors,
    };
    Ok((manifest, parity_blobs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stitch_object_split_and_all_three_recovery_pairs() {
        let payload = b"Stitch 2-of-3 XOR odd length verification payload!1";
        assert_eq!(payload.len() % 2, 1);

        let enc = split_2of3_xor(payload);
        assert_eq!(enc.scheme, "xor-2-of-3");
        assert_eq!(enc.threshold, 2);
        assert_eq!(enc.count, 3);
        assert_eq!(enc.original_length, payload.len());
        assert_eq!(enc.fragment_length, payload.len().div_ceil(2));

        let frag_digests = [
            enc.fragments[0].sha256.as_str(),
            enc.fragments[1].sha256.as_str(),
            enc.fragments[2].sha256.as_str(),
        ];

        // Pair (0, 1): missing parity
        let r01 = recover_2of3_object(
            &[
                (0, enc.fragments[0].bytes.as_slice()),
                (1, enc.fragments[1].bytes.as_slice()),
            ],
            enc.original_length,
            Some(&enc.object_sha256),
            Some(&frag_digests),
        )
        .expect("(0, 1) must recover");
        assert_eq!(r01, payload);

        // Pair (0, 2): missing right half -> recovers right from left ^ parity
        let r02 = recover_2of3_object(
            &[
                (0, enc.fragments[0].bytes.as_slice()),
                (2, enc.fragments[2].bytes.as_slice()),
            ],
            enc.original_length,
            Some(&enc.object_sha256),
            Some(&frag_digests),
        )
        .expect("(0, 2) must recover");
        assert_eq!(r02, payload);

        // Pair (1, 2): missing left half -> recovers left from right ^ parity
        let r12 = recover_2of3_object(
            &[
                (1, enc.fragments[1].bytes.as_slice()),
                (2, enc.fragments[2].bytes.as_slice()),
            ],
            enc.original_length,
            Some(&enc.object_sha256),
            Some(&frag_digests),
        )
        .expect("(1, 2) must recover");
        assert_eq!(r12, payload);
    }

    #[test]
    fn test_stitch_object_negative_errors_r5() {
        let payload = b"neg-test-stitch-object-custody";
        let enc = split_2of3_xor(payload);
        let frag_digests = [
            enc.fragments[0].sha256.as_str(),
            enc.fragments[1].sha256.as_str(),
            enc.fragments[2].sha256.as_str(),
        ];

        // InsufficientShards
        let err_insuff = recover_2of3_object(
            &[(0, enc.fragments[0].bytes.as_slice())],
            enc.original_length,
            None,
            None,
        )
        .expect_err("single fragment must fail");
        assert_eq!(err_insuff.error_code(), "StitchInsufficientShards");

        // InvalidShardIndex
        let err_idx = recover_2of3_object(
            &[
                (0, enc.fragments[0].bytes.as_slice()),
                (3, enc.fragments[1].bytes.as_slice()),
            ],
            enc.original_length,
            None,
            None,
        )
        .expect_err("index 3 must fail");
        assert_eq!(err_idx.error_code(), "StitchInvalidShardIndex");

        // DuplicateShardIndex
        let err_dup = recover_2of3_object(
            &[
                (1, enc.fragments[1].bytes.as_slice()),
                (1, enc.fragments[1].bytes.as_slice()),
            ],
            enc.original_length,
            None,
            None,
        )
        .expect_err("duplicate index must fail");
        assert_eq!(err_dup.error_code(), "StitchDuplicateShardIndex");

        // ShardLengthMismatch
        let short = &enc.fragments[1].bytes[..enc.fragments[1].bytes.len() - 1];
        let err_len = recover_2of3_object(
            &[(0, enc.fragments[0].bytes.as_slice()), (1, short)],
            enc.original_length,
            None,
            None,
        )
        .expect_err("short fragment must fail");
        assert_eq!(err_len.error_code(), "StitchShardLengthMismatch");

        // ShardDigestMismatch
        let mut tampered = enc.fragments[2].bytes.clone();
        tampered[0] ^= 0xFF;
        let err_sdig = recover_2of3_object(
            &[(0, enc.fragments[0].bytes.as_slice()), (2, &tampered)],
            enc.original_length,
            Some(&enc.object_sha256),
            Some(&frag_digests),
        )
        .expect_err("tampered parity digest must fail");
        assert_eq!(err_sdig.error_code(), "StitchShardDigestMismatch");

        // PayloadDigestMismatch (when fragment digests omitted, object sha256 still catches tamper)
        let mut tampered_raw = enc.fragments[0].bytes.clone();
        tampered_raw[0] ^= 0x01;
        let err_pdig = recover_2of3_object(
            &[(0, &tampered_raw), (1, enc.fragments[1].bytes.as_slice())],
            enc.original_length,
            Some(&enc.object_sha256),
            None,
        )
        .expect_err("tampered payload digest must fail");
        assert_eq!(err_pdig.error_code(), "StitchPayloadDigestMismatch");
    }

    #[test]
    fn test_manifest_validation_and_pair_recovery_with_odd_tail() {
        let s0 = vec![0xAAu8; 64];
        let s1 = vec![0x55u8; 40]; // shorter right shard tests zero-pad XOR parity
        let s2 = vec![0x7Eu8; 19]; // trailing odd shard with no right partner
        let mut full = Vec::new();
        full.extend_from_slice(&s0);
        full.extend_from_slice(&s1);
        full.extend_from_slice(&s2);
        let full_sha = sha256_hex(&full);

        let (manifest, parity_blobs) = build_pair_manifest(
            &full_sha,
            &[
                ("shard_000.bin".to_string(), 0, s0.as_slice()),
                ("shard_001.bin".to_string(), 64, s1.as_slice()),
                ("shard_002.bin".to_string(), 104, s2.as_slice()),
            ],
            vec!["https://mirror.example.org/weights/v10/".to_string()],
        )
        .expect("build manifest");

        let json = serde_json::to_string_pretty(&manifest).expect("serialize manifest");
        let parsed = parse_and_validate_manifest(&json).expect("manifest valid");
        assert_eq!(parsed.groups.len(), 2);

        // Group 0: missing left, supply right + parity -> recovers left accurately
        let g0_rec_left =
            recover_2of3_pair(&parsed.groups[0], None, Some(&s1), Some(&parity_blobs[0].1))
                .expect("recover missing left shard");
        assert_eq!(g0_rec_left.recovered_slot, "left");
        assert_eq!(g0_rec_left.left_bytes, s0);

        // Group 0: corrupt left + valid right + valid parity -> heal_2of3_pair drops corrupt left
        let mut bad_s0 = s0.clone();
        bad_s0[5] ^= 0xFF;
        let g0_heal = heal_2of3_pair(
            &parsed.groups[0],
            Some(&bad_s0),
            Some(&s1),
            Some(&parity_blobs[0].1),
        )
        .expect("auto-heal single corrupted left shard");
        assert_eq!(g0_heal.recovered_slot, "left");
        assert_eq!(g0_heal.left_bytes, s0);

        // Group 0: strict recover_2of3_pair rejects corrupted left explicitly
        let err_strict = recover_2of3_pair(
            &parsed.groups[0],
            Some(&bad_s0),
            Some(&s1),
            Some(&parity_blobs[0].1),
        )
        .expect_err("strict recover must reject corrupt input slice");
        assert_eq!(err_strict.error_code(), "StitchShardDigestMismatch");

        // Group 0: recover shorter right shard cleanly without trailing zero leakage
        let g0_rec_right =
            recover_2of3_pair(&parsed.groups[0], Some(&s0), None, Some(&parity_blobs[0].1))
                .expect("recover missing shorter right shard");
        assert_eq!(g0_rec_right.recovered_slot, "right");
        assert_eq!(g0_rec_right.right_bytes.as_deref(), Some(s1.as_slice()));

        // Group 1 (odd tail): missing left recovers from parity_001
        let g1_rec = recover_2of3_pair(&parsed.groups[1], None, None, Some(&parity_blobs[1].1))
            .expect("recover odd tail shard from parity");
        assert_eq!(g1_rec.recovered_slot, "left");
        assert_eq!(g1_rec.left_bytes, s2);

        // Double corruption fails with typed ShardDigestMismatch
        let mut bad_parity = parity_blobs[0].1.clone();
        bad_parity[0] ^= 0x11;
        let err_double = heal_2of3_pair(
            &parsed.groups[0],
            Some(&bad_s0),
            Some(&s1),
            Some(&bad_parity),
        )
        .expect_err("double corruption across group must fail");
        assert_eq!(err_double.error_code(), "StitchShardDigestMismatch");

        // Custody health summary
        let health = assess_custody_health(&parsed, vec![0], vec![]);
        assert_eq!(health.intact_groups, 1);
        assert_eq!(health.recovered_shards, vec![0]);
        assert!(health.custody_healthy);
    }

    #[test]
    fn test_manifest_validation_negative_errors_r5() {
        let e_json =
            parse_and_validate_manifest("{not json").expect_err("malformed json must fail");
        assert_eq!(e_json.error_code(), "StitchInvalidManifest");

        let s0 = vec![1u8; 32];
        let s1 = vec![2u8; 32];
        let sha = sha256_hex(&[s0.as_slice(), s1.as_slice()].concat());
        let (base, _) = build_pair_manifest(
            &sha,
            &[
                ("shard_000.bin".to_string(), 0, &s0),
                ("shard_001.bin".to_string(), 32, &s1),
            ],
            vec![],
        )
        .unwrap();

        let mut bad_schema = base.clone();
        bad_schema.schema = "stitch/unknown/0".to_string();
        assert_eq!(
            parse_and_validate_manifest(&serde_json::to_string(&bad_schema).unwrap())
                .unwrap_err()
                .error_code(),
            "StitchUnsupportedSchema"
        );

        let mut bad_scheme = base.clone();
        bad_scheme.scheme = "reed-solomon".to_string();
        assert_eq!(
            parse_and_validate_manifest(&serde_json::to_string(&bad_scheme).unwrap())
                .unwrap_err()
                .error_code(),
            "StitchUnsupportedScheme"
        );

        let mut bad_cfg = base.clone();
        bad_cfg.threshold = 1;
        assert_eq!(
            parse_and_validate_manifest(&serde_json::to_string(&bad_cfg).unwrap())
                .unwrap_err()
                .error_code(),
            "StitchInvalidThresholdConfig"
        );

        let mut bad_len = base.clone();
        bad_len.total_bytes = 999;
        assert_eq!(
            parse_and_validate_manifest(&serde_json::to_string(&bad_len).unwrap())
                .unwrap_err()
                .error_code(),
            "StitchPayloadLengthMismatch"
        );
    }
}
