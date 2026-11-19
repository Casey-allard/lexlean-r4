//! Track 1 (`R4-50`): Content-addressed (`kappa`) OPFS piece storage, 64 KiB `HOLD_TAIL_BYTES`
//! digest verification guard, Stitch 2-of-3 XOR parity recovery, and zero-copy Safetensors
//! header parsing for `UORR4V13` (`Qwen3-0.6B`, `Qwen3-1.7B`, and direct HuggingFace HTTP `206`
//! `Range` repo streaming).

use crate::lexlean_generated::r4_holo_kappa_store::{
    admit_holo_kappa_piece, admit_range_slice, KappaStoreCheck, KappaStoreVerdict,
    RangeWindowCheck, StoreOperationKind, DEFAULT_HOLD_BYTES, DEFAULT_PIECE_BYTES,
};
use crate::lexlean_generated::r4_webgpu_geodesic_runtime::{
    admit_holo_slab_mount, HoloSlabVerdict,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;

/// Canonical `UORR4V13` container magic validated alongside `kappa`-addressed manifests.
pub const HOLO_CONTAINER_MAGIC: &[u8; 8] = b"UORR4V13";

/// Canonical trailing verification hold buffer (64 KiB) enforced before releasing unverified bytes.
pub const HOLD_TAIL_BYTES: usize = DEFAULT_HOLD_BYTES as usize;

/// Canonical 1 MiB piece budget for content-addressed OPFS & Stitch XOR parity shards.
pub const PIECE_BUDGET_BYTES: usize = DEFAULT_PIECE_BYTES as usize;

/// Maximum Safetensors JSON header length admitted in browser streaming mode (64 MiB).
pub const MAX_SAFETENSORS_HEADER_BYTES: usize = 64 * 1024 * 1024;

/// Canonical magic identifier for single-slab zero-rehash OPFS custody (`HOLOSLAB2`, `R4-61`).
pub const HOLOSLAB2_MAGIC: &str = "HOLOSLAB2";
/// Canonical schema identifier for `manifest.verified.json` (`R4-61`).
pub const HOLOSLAB2_SCHEMA: &str = "uor-r4.holoslab/2";
/// Pre-allocated HTTP ring buffer size (2 MiB) for zero-allocation single-pass streaming (`R4-61`).
pub const HOLOSLAB2_RING_BYTES: usize = 2 * 1024 * 1024;
/// Maximum Wasm/GPU slab read staging chunk size (4 MiB, `R4-61`).
pub const HOLOSLAB2_MAX_STAGING_BYTES: usize = 4 * 1024 * 1024;

/// Typed public failure modes (`R5`) for `R4-50` `OpfsKappaStore` and `HfRepoHolo` operations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HoloKappaStoreError {
    /// Trailing `HOLD_TAIL_BYTES` buffer released before digest confirmation.
    HoldBufferUnderrun {
        /// Actual trailing bytes held.
        hold_bytes: usize,
        /// Minimum trailing bytes required by policy.
        required_hold_bytes: usize,
    },
    /// Piece SHA-256 digest differed from manifest commitment.
    PieceHashMismatch {
        /// Expected 64-char hex digest.
        expected_sha256: String,
        /// Computed actual SHA-256 hex digest.
        actual_sha256: String,
    },
    /// Upstream host refused HTTP `206 Partial Content` `Range` request.
    RangeRequestUnsupported(String),
    /// Safetensors prefix header failed magic, UTF-8, or tensor span checks.
    CorruptSafetensorsHeader(String),
    /// OPFS or IndexedDB commit transaction failed or aborted.
    StorageCommitAbort(String),
    /// Fewer than 2 of 3 Stitch XOR parity shards available to heal a piece.
    InsufficientParityShards {
        /// Shards present or uncorrupted.
        available: usize,
        /// Required shards (`2`).
        required: usize,
    },
    /// Downloaded or incoming piece exceeded `PIECE_BUDGET_BYTES` (1 MiB).
    PieceBudgetExceeded {
        /// Observed piece length in bytes.
        piece_bytes: usize,
        /// Maximum allowed piece budget.
        max_bytes: usize,
    },
}

impl HoloKappaStoreError {
    /// Formal R4.HoloKappaStore map runtime error to canonical LexLean `KappaStoreVerdict`.
    pub fn verdict(&self) -> KappaStoreVerdict {
        match self {
            Self::HoldBufferUnderrun { .. } => KappaStoreVerdict::RefusedHoldBufferUnderrun,
            Self::PieceHashMismatch { .. } => KappaStoreVerdict::RefusedPieceHashMismatch,
            Self::RangeRequestUnsupported(_) => KappaStoreVerdict::RefusedRangeRequestUnsupported,
            Self::CorruptSafetensorsHeader(_) => KappaStoreVerdict::RefusedCorruptSafetensorsHeader,
            Self::StorageCommitAbort(_) => KappaStoreVerdict::RefusedStorageCommitAbort,
            Self::InsufficientParityShards { .. } => {
                KappaStoreVerdict::RefusedInsufficientParityShards
            }
            Self::PieceBudgetExceeded { .. } => KappaStoreVerdict::RefusedPieceBudgetExceeded,
        }
    }
}

impl fmt::Display for HoloKappaStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HoldBufferUnderrun {
                hold_bytes,
                required_hold_bytes,
            } => write!(
                f,
                "hold buffer underrun: {hold_bytes} bytes < required {required_hold_bytes} bytes"
            ),
            Self::PieceHashMismatch {
                expected_sha256,
                actual_sha256,
            } => write!(
                f,
                "SHA-256 piece mismatch: expected {expected_sha256}, got {actual_sha256}"
            ),
            Self::RangeRequestUnsupported(msg) => write!(f, "HTTP 206 range unsupported: {msg}"),
            Self::CorruptSafetensorsHeader(msg) => write!(f, "corrupt safetensors header: {msg}"),
            Self::StorageCommitAbort(msg) => write!(f, "OPFS/IDB commit aborted: {msg}"),
            Self::InsufficientParityShards {
                available,
                required,
            } => write!(
                f,
                "insufficient Stitch parity shards: available {available} < required {required}"
            ),
            Self::PieceBudgetExceeded {
                piece_bytes,
                max_bytes,
            } => write!(
                f,
                "piece budget exceeded: {piece_bytes} bytes > max {max_bytes} bytes"
            ),
        }
    }
}

impl std::error::Error for HoloKappaStoreError {}

/// Formal R4.HoloKappaStore compute lowercase hexadecimal SHA-256 digest of a byte slice.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{b:02x}");
    }
    hex
}

/// Formal R4.HoloKappaStore constant-time byte slice equality for digest comparisons.
pub fn constant_time_eq_bytes(a: &[u8], b: &[u8]) -> bool {
    let len_diff = a.len() ^ b.len();
    let max_len = a.len().max(b.len());
    let mut diff = len_diff;
    for i in 0..max_len {
        let ba = a.get(i).copied().unwrap_or(0);
        let bb = b.get(i).copied().unwrap_or(0);
        diff |= usize::from(ba ^ bb);
    }
    diff == 0
}

/// Formal R4.HoloKappaStore derive deterministic hierarchical OPFS path (`sha256/<hh>/<hh>/<hex>`).
pub fn opfs_kappa_path(sha256_hex_str: &str) -> Result<String, HoloKappaStoreError> {
    let lower = sha256_hex_str.trim().to_ascii_lowercase();
    if lower.len() != 64 || !lower.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(HoloKappaStoreError::PieceHashMismatch {
            expected_sha256: "64-hex SHA-256".to_string(),
            actual_sha256: sha256_hex_str.to_string(),
        });
    }
    Ok(format!(
        "sha256/{}/{}/{}",
        &lower[0..2],
        &lower[2..4],
        lower
    ))
}

/// Formal R4.HoloKappaStore verify a piece against its byte budget and expected SHA-256 digest.
pub fn verify_kappa_piece(
    bytes: &[u8],
    expected_sha256: &str,
) -> Result<String, HoloKappaStoreError> {
    if bytes.len() > PIECE_BUDGET_BYTES {
        return Err(HoloKappaStoreError::PieceBudgetExceeded {
            piece_bytes: bytes.len(),
            max_bytes: PIECE_BUDGET_BYTES,
        });
    }
    let actual = sha256_hex(bytes);
    let expected_norm = expected_sha256.trim().to_ascii_lowercase();
    if !constant_time_eq_bytes(actual.as_bytes(), expected_norm.as_bytes()) {
        return Err(HoloKappaStoreError::PieceHashMismatch {
            expected_sha256: expected_norm,
            actual_sha256: actual,
        });
    }
    let verdict = admit_holo_kappa_piece(KappaStoreCheck {
        operation: StoreOperationKind::PutVerifiedPiece,
        piece_bytes: bytes.len() as u64,
        max_piece_bytes: PIECE_BUDGET_BYTES as u64,
        hold_bytes: HOLD_TAIL_BYTES as u64,
        hold_released_before_digest: false,
        sha256_match: true,
        range_http_206: true,
        safetensors_header_valid: true,
        idb_transaction_completed: true,
        stitch_available_shards: 3,
        stitch_required_shards: 2,
    });
    if verdict != KappaStoreVerdict::PieceAdmitted {
        return Err(HoloKappaStoreError::StorageCommitAbort(format!(
            "formal kappa piece admission rejected: {verdict:?}"
        )));
    }
    Ok(actual)
}

/// Formal R4.HoloKappaStore validate an HTTP `206` `Range` window against total length & `HOLD_TAIL_BYTES`.
pub fn validate_range_window(
    offset: u64,
    length: u64,
    total_file_bytes: u64,
) -> Result<(), HoloKappaStoreError> {
    if length == 0 {
        return Err(HoloKappaStoreError::RangeRequestUnsupported(
            "zero-length byte range".to_string(),
        ));
    }
    let verdict = admit_range_slice(RangeWindowCheck {
        requested_offset: offset,
        requested_length: length,
        total_file_bytes,
        hold_tail_bytes: HOLD_TAIL_BYTES as u64,
    });
    if verdict == KappaStoreVerdict::PieceAdmitted {
        Ok(())
    } else if verdict == KappaStoreVerdict::RefusedPieceBudgetExceeded {
        Err(HoloKappaStoreError::PieceBudgetExceeded {
            piece_bytes: length as usize,
            max_bytes: PIECE_BUDGET_BYTES,
        })
    } else {
        Err(HoloKappaStoreError::RangeRequestUnsupported(format!(
            "range [{offset}..+{length}) over {total_file_bytes} bytes refused ({verdict:?})"
        )))
    }
}

/// Streaming ring/tail guard holding the trailing `HOLD_TAIL_BYTES` (64 KiB) until SHA-256 validates.
pub struct HoldTailStreamGuard {
    hold_bytes: usize,
    tail: Vec<u8>,
    released: Vec<u8>,
    hasher: Sha256,
    finalized: bool,
}

impl Default for HoldTailStreamGuard {
    fn default() -> Self {
        Self::new(HOLD_TAIL_BYTES)
    }
}

impl HoldTailStreamGuard {
    /// Formal R4.HoloKappaStore construct a new digest-gated trailing hold guard.
    pub fn new(hold_bytes: usize) -> Self {
        Self {
            hold_bytes,
            tail: Vec::with_capacity(hold_bytes + 4096),
            released: Vec::new(),
            hasher: Sha256::new(),
            finalized: false,
        }
    }

    /// Formal R4.HoloKappaStore ingest incoming bytes while holding back at least `hold_bytes`.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), HoloKappaStoreError> {
        if self.finalized {
            return Err(HoloKappaStoreError::StorageCommitAbort(
                "stream already finalized".to_string(),
            ));
        }
        self.hasher.update(chunk);
        self.tail.extend_from_slice(chunk);
        if self.tail.len() > self.hold_bytes {
            let drain_len = self.tail.len() - self.hold_bytes;
            self.released.extend_from_slice(&self.tail[..drain_len]);
            self.tail.drain(..drain_len);
        }
        Ok(())
    }

    /// Formal R4.HoloKappaStore peek bytes already safe to inspect before trailing hold boundary.
    pub fn released_prefix(&self) -> &[u8] {
        &self.released
    }

    /// Formal R4.HoloKappaStore return how many bytes remain held pending digest verification.
    pub fn held_len(&self) -> usize {
        self.tail.len()
    }

    /// Formal R4.HoloKappaStore release the held tail ONLY when SHA-256 matches `expected_sha256`.
    pub fn finish_verified(
        mut self,
        expected_sha256: Option<&str>,
    ) -> Result<(Vec<u8>, String), HoloKappaStoreError> {
        self.finalized = true;
        let digest = self.hasher.finalize();
        let mut actual_hex = String::with_capacity(64);
        for b in digest {
            use std::fmt::Write;
            let _ = write!(&mut actual_hex, "{b:02x}");
        }
        if let Some(exp) = expected_sha256 {
            let exp_norm = exp.trim().to_ascii_lowercase();
            let hold_ok = self.hold_bytes >= HOLD_TAIL_BYTES;
            let verdict = admit_holo_kappa_piece(KappaStoreCheck {
                operation: StoreOperationKind::CommitIdbTransaction,
                piece_bytes: self.tail.len().min(PIECE_BUDGET_BYTES) as u64,
                max_piece_bytes: PIECE_BUDGET_BYTES as u64,
                hold_bytes: self.hold_bytes as u64,
                hold_released_before_digest: false,
                sha256_match: constant_time_eq_bytes(actual_hex.as_bytes(), exp_norm.as_bytes()),
                range_http_206: true,
                safetensors_header_valid: true,
                idb_transaction_completed: hold_ok,
                stitch_available_shards: 3,
                stitch_required_shards: 2,
            });
            if verdict != KappaStoreVerdict::PieceAdmitted {
                self.tail.clear();
                self.released.clear();
                return Err(HoloKappaStoreError::PieceHashMismatch {
                    expected_sha256: exp_norm,
                    actual_sha256: actual_hex,
                });
            }
        }
        self.released.extend_from_slice(&self.tail);
        Ok((self.released, actual_hex))
    }
}

/// Formal R4.HoloKappaStore recover a corrupt or missing piece from its peer & Stitch XOR parity shard.
pub fn recover_piece_xor_2of3(
    peer_piece: &[u8],
    parity_piece: &[u8],
    expected_len: usize,
    expected_sha256: &str,
) -> Result<Vec<u8>, HoloKappaStoreError> {
    if peer_piece.len() > PIECE_BUDGET_BYTES || parity_piece.len() > PIECE_BUDGET_BYTES {
        return Err(HoloKappaStoreError::PieceBudgetExceeded {
            piece_bytes: peer_piece.len().max(parity_piece.len()),
            max_bytes: PIECE_BUDGET_BYTES,
        });
    }
    if expected_len > PIECE_BUDGET_BYTES || parity_piece.len() < expected_len {
        return Err(HoloKappaStoreError::InsufficientParityShards {
            available: 1,
            required: 2,
        });
    }
    let mut recovered = vec![0u8; expected_len];
    for (idx, dst) in recovered.iter_mut().enumerate() {
        let p = peer_piece.get(idx).copied().unwrap_or(0);
        let par = parity_piece.get(idx).copied().unwrap_or(0);
        *dst = p ^ par;
    }
    verify_kappa_piece(&recovered, expected_sha256)?;
    Ok(recovered)
}

/// Metadata entry for one tensor described inside a HuggingFace Safetensors JSON header.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetensorsTensorMeta {
    /// Parameter key inside Safetensors file.
    pub name: String,
    /// Element storage format (`BF16`, `F16`, `F32`, etc.).
    pub dtype: String,
    /// Tensor dimensions.
    pub shape: Vec<usize>,
    /// Inclusive byte offset relative to `data_start`.
    pub data_offset_start: u64,
    /// Exclusive byte offset relative to `data_start`.
    pub data_offset_end: u64,
}

impl SafetensorsTensorMeta {
    /// Formal R4.HoloKappaStore compute byte width per element for validated Safetensors dtypes.
    pub fn dtype_bytes(&self) -> Result<usize, HoloKappaStoreError> {
        match self.dtype.as_str() {
            "BF16" | "F16" | "I16" | "U16" => Ok(2),
            "F32" | "I32" | "U32" => Ok(4),
            "F64" | "I64" | "U64" => Ok(8),
            "I8" | "U8" | "F8_E4M3" | "F8_E5M2" => Ok(1),
            other => Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
                "unsupported dtype '{other}' for tensor '{}'",
                self.name
            ))),
        }
    }

    /// Formal R4.HoloKappaStore compute exact byte span `[start..end)` declared by `data_offsets`.
    pub fn byte_len(&self) -> u64 {
        self.data_offset_end.saturating_sub(self.data_offset_start)
    }
}

/// Parsed Safetensors header index used by `HfRepoHolo` HTTP `206` `Range` streaming.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetensorsHeaderMetadata {
    /// Byte length of raw JSON header.
    pub header_len: usize,
    /// File byte offset where tensor binary payload begins (`8 + header_len`).
    pub data_start: u64,
    /// Map from tensor name to validated dtype/shape/offset metadata.
    pub tensors: BTreeMap<String, SafetensorsTensorMeta>,
    /// Highest relative byte offset across all tensors.
    pub max_data_end: u64,
}

#[derive(Deserialize)]
struct RawTensorEntry {
    dtype: String,
    shape: Vec<usize>,
    data_offsets: [u64; 2],
}

/// Formal R4.HoloKappaStore parse & validate a binary Safetensors prefix (`u64 len` + JSON header).
pub fn parse_safetensors_header(
    prefix_bytes: &[u8],
) -> Result<SafetensorsHeaderMetadata, HoloKappaStoreError> {
    if prefix_bytes.len() < 8 {
        return Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
            "prefix too short ({} bytes < 8-byte header length)",
            prefix_bytes.len()
        )));
    }
    let header_len_u64 = u64::from_le_bytes(
        prefix_bytes[0..8]
            .try_into()
            .map_err(|_| HoloKappaStoreError::CorruptSafetensorsHeader("u64 slice".into()))?,
    );
    if header_len_u64 == 0 || header_len_u64 > MAX_SAFETENSORS_HEADER_BYTES as u64 {
        return Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
            "invalid header length {header_len_u64}"
        )));
    }
    let header_len = header_len_u64 as usize;
    let total_hdr = 8usize.saturating_add(header_len);
    if prefix_bytes.len() < total_hdr {
        return Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
            "truncated safetensors header: have {} bytes, need {total_hdr}",
            prefix_bytes.len()
        )));
    }
    let hdr_str = std::str::from_utf8(&prefix_bytes[8..total_hdr]).map_err(|e| {
        HoloKappaStoreError::CorruptSafetensorsHeader(format!("non-UTF-8 header JSON: {e}"))
    })?;
    let raw_map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(hdr_str)
        .map_err(|e| HoloKappaStoreError::CorruptSafetensorsHeader(format!("invalid JSON: {e}")))?;

    let mut tensors = BTreeMap::new();
    let mut max_data_end = 0u64;
    for (name, val) in raw_map {
        if name == "__metadata__" {
            continue;
        }
        let raw: RawTensorEntry = serde_json::from_value(val).map_err(|e| {
            HoloKappaStoreError::CorruptSafetensorsHeader(format!("tensor '{name}' schema: {e}"))
        })?;
        if raw.data_offsets[1] < raw.data_offsets[0] {
            return Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
                "tensor '{name}' descending offsets {:?}",
                raw.data_offsets
            )));
        }
        let meta = SafetensorsTensorMeta {
            name: name.clone(),
            dtype: raw.dtype,
            shape: raw.shape,
            data_offset_start: raw.data_offsets[0],
            data_offset_end: raw.data_offsets[1],
        };
        let elem_bytes = meta.dtype_bytes()? as u64;
        let elems = meta.shape.iter().try_fold(1u64, |acc, &d| {
            acc.checked_mul(d as u64).ok_or_else(|| {
                HoloKappaStoreError::CorruptSafetensorsHeader(format!(
                    "tensor '{name}' shape overflow"
                ))
            })
        })?;
        let expected_bytes = elems.checked_mul(elem_bytes).ok_or_else(|| {
            HoloKappaStoreError::CorruptSafetensorsHeader(format!(
                "tensor '{name}' byte count overflow"
            ))
        })?;
        if meta.byte_len() != expected_bytes {
            return Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
                "tensor '{name}' span {} != shape*dtype {}",
                meta.byte_len(),
                expected_bytes
            )));
        }
        max_data_end = max_data_end.max(meta.data_offset_end);
        tensors.insert(name, meta);
    }

    if tensors.is_empty() {
        return Err(HoloKappaStoreError::CorruptSafetensorsHeader(
            "safetensors header contains zero tensors".to_string(),
        ));
    }

    Ok(SafetensorsHeaderMetadata {
        header_len,
        data_start: total_hdr as u64,
        tensors,
        max_data_end,
    })
}

/// Summary status returned by `OpfsKappaStore` Worker RPC status queries.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoloKappaStoreStatus {
    /// Active container magic format (`UORR4V13`).
    pub container_format: String,
    /// Active model identifier.
    pub active_model_id: String,
    /// Content-addressed SHA-256 digest of container.
    pub container_sha256: String,
    /// Root OPFS directory path.
    pub opfs_root_dir: String,
    /// Total 1 MiB piece count in manifest.
    pub piece_count: usize,
    /// Number of locally cached and verified pieces.
    pub verified_pieces: usize,
    /// Total cached bytes in OPFS.
    pub cached_bytes: u64,
    /// Enforced trailing digest hold buffer in bytes (`65536`).
    pub hold_tail_bytes: usize,
    /// Stitch 2-of-3 XOR parity recoveries performed.
    pub stitch_recoveries: usize,
    /// Quarantined mirror origins flagged for digest mismatch.
    pub liar_mirrors: Vec<String>,
    /// True once all required pieces are verified in OPFS.
    pub persisted_ready: bool,
}

/// Piece offset entry inside a sealed `HOLOSLAB2` single-file OPFS container (`R4-61`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoloSlab2PieceEntry {
    /// Zero-based piece index.
    pub index: usize,
    /// Sequential byte offset inside `model.holoslab`.
    pub byte_offset: u64,
    /// Exact piece length in bytes.
    pub byte_length: u64,
    /// Lowercase SHA-256 hex digest of this piece.
    pub sha256: String,
}

/// Sealed `manifest.verified.json` metadata for `HOLOSLAB2` single-slab custody (`R4-61`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoloSlab2Seal {
    /// Schema marker (`uor-r4.holoslab/2`).
    pub schema: String,
    /// Format magic (`HOLOSLAB2`).
    pub magic: String,
    /// Container SHA-256 digest committed by the upstream manifest.
    pub container_sha256: String,
    /// Linear Merkle root SHA-256 over sequential piece SHA-256 digests.
    pub merkle_root_sha256: String,
    /// Expected contiguous slab byte size.
    pub total_bytes: u64,
    /// Number of pieces covered inside the slab.
    pub piece_count: usize,
    /// Logical model wire identifier.
    pub model_id: String,
    /// Ordered piece map inside `model.holoslab`.
    #[serde(default)]
    pub pieces: Vec<HoloSlab2PieceEntry>,
}

/// Outcome of validating a `HOLOSLAB2` OPFS slab seal via `R4.WebGpuGeodesicRuntime.admitHoloSlabMount`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoloSlab2ValidationReport {
    /// True when the slab seal / Merkle check passed.
    pub ok: bool,
    /// Canonical `R4-61` verdict name (`SlabSealAdmitted`, `SlabFullMerkleVerified`, or typed `Refused*`).
    pub verdict: String,
    /// Expected contiguous byte size of `model.holoslab`.
    pub total_bytes: u64,
    /// Piece count recorded in the seal.
    pub piece_count: usize,
    /// Computed linear Merkle root over `pieces` digests (when present).
    pub computed_merkle_root: String,
}

/// Formal R4.WebGpuGeodesicRuntime compute deterministic Merkle-chain SHA-256 over ordered piece hashes.
pub fn compute_holoslab2_merkle_root(pieces: &[HoloSlab2PieceEntry]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(HOLOSLAB2_MAGIC.as_bytes());
    hasher.update((pieces.len() as u64).to_le_bytes());
    for entry in pieces {
        hasher.update((entry.index as u64).to_le_bytes());
        hasher.update(entry.byte_offset.to_le_bytes());
        hasher.update(entry.byte_length.to_le_bytes());
        hasher.update(entry.sha256.trim().to_ascii_lowercase().as_bytes());
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

/// Formal R4.WebGpuGeodesicRuntime map `HoloSlabVerdict` to its canonical `R4-61` verdict string.
pub fn holoslab_verdict_label(verdict: HoloSlabVerdict) -> &'static str {
    match verdict {
        HoloSlabVerdict::SlabSealAdmitted => "SlabSealAdmitted",
        HoloSlabVerdict::SlabFullMerkleVerified => "SlabFullMerkleVerified",
        HoloSlabVerdict::RefusedInsufficientStorageQuota => "RefusedInsufficientStorageQuota",
        HoloSlabVerdict::RefusedSlabSealMismatch => "RefusedSlabSealMismatch",
        HoloSlabVerdict::RefusedSlabPieceHashMismatch => "RefusedSlabPieceHashMismatch",
    }
}

/// Formal R4.WebGpuGeodesicRuntime validate a `HOLOSLAB2` single-slab manifest seal, byte length, quota, and Merkle root.
pub fn validate_holoslab2_mount(
    seal: &HoloSlab2Seal,
    quota_bytes: u64,
    required_bytes: u64,
    actual_slab_bytes: u64,
    full_scan_requested: bool,
    piece_hashes_valid: bool,
) -> HoloSlab2ValidationReport {
    let computed_merkle = compute_holoslab2_merkle_root(&seal.pieces);
    let mut running_offset = 0u64;
    let mut layout_contiguous = true;
    for (idx, p) in seal.pieces.iter().enumerate() {
        if p.index != idx || p.byte_offset != running_offset {
            layout_contiguous = false;
            break;
        }
        running_offset = running_offset.saturating_add(p.byte_length);
    }
    let seal_matches = seal.magic == HOLOSLAB2_MAGIC
        && seal.schema == HOLOSLAB2_SCHEMA
        && seal.total_bytes > 0
        && actual_slab_bytes == seal.total_bytes
        && seal.pieces.len() == seal.piece_count
        && running_offset == seal.total_bytes
        && layout_contiguous
        && constant_time_eq_bytes(
            computed_merkle.as_bytes(),
            seal.merkle_root_sha256
                .trim()
                .to_ascii_lowercase()
                .as_bytes(),
        );

    let verdict = admit_holo_slab_mount(
        quota_bytes,
        required_bytes,
        seal_matches,
        full_scan_requested,
        piece_hashes_valid,
    );
    let ok = matches!(
        verdict,
        HoloSlabVerdict::SlabSealAdmitted | HoloSlabVerdict::SlabFullMerkleVerified
    );
    HoloSlab2ValidationReport {
        ok,
        verdict: holoslab_verdict_label(verdict).to_string(),
        total_bytes: seal.total_bytes,
        piece_count: seal.piece_count,
        computed_merkle_root: computed_merkle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_tail_guard_blocks_mismatch_and_releases_verified() {
        let mut payload = vec![0x42u8; HOLD_TAIL_BYTES + 4096];
        for (i, b) in payload.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(31).wrapping_add(7);
        }
        let digest = sha256_hex(&payload);

        let mut ok_guard = HoldTailStreamGuard::default();
        ok_guard.push(&payload).expect("push ok");
        assert_eq!(ok_guard.held_len(), HOLD_TAIL_BYTES);
        let (out, actual) = ok_guard.finish_verified(Some(&digest)).expect("verified");
        assert_eq!(actual, digest);
        assert_eq!(out, payload);

        let mut bad_guard = HoldTailStreamGuard::default();
        bad_guard.push(&payload).expect("push ok");
        let bad_digest = "0".repeat(64);
        let err = bad_guard
            .finish_verified(Some(&bad_digest))
            .expect_err("mismatch must fail");
        assert_eq!(err.verdict(), KappaStoreVerdict::RefusedPieceHashMismatch);
    }

    #[test]
    fn xor_2of3_recovers_corrupt_piece_and_checks_digest() {
        let a = b"UORR4V13_piece_a_payload_0123456789".to_vec();
        let b = b"UORR4V13_piece_b_payload_abcdefghij".to_vec();
        let parity: Vec<u8> = a.iter().zip(&b).map(|(&x, &y)| x ^ y).collect();
        let recovered = recover_piece_xor_2of3(&b, &parity, a.len(), &sha256_hex(&a))
            .expect("2-of-3 recovery must succeed");
        assert_eq!(recovered, a);

        let err = recover_piece_xor_2of3(&b, &parity, a.len(), &"f".repeat(64))
            .expect_err("corrupt recovery digest must fail");
        assert_eq!(err.verdict(), KappaStoreVerdict::RefusedPieceHashMismatch);
    }

    #[test]
    fn safetensors_header_validates_offsets_and_dtypes() {
        let hdr = r#"{"model.norm.weight":{"dtype":"BF16","shape":[4],"data_offsets":[0,8]}}"#;
        let mut prefix = Vec::new();
        prefix.extend_from_slice(&(hdr.len() as u64).to_le_bytes());
        prefix.extend_from_slice(hdr.as_bytes());
        let parsed = parse_safetensors_header(&prefix).expect("valid header");
        assert_eq!(parsed.data_start, prefix.len() as u64);
        assert_eq!(parsed.tensors["model.norm.weight"].byte_len(), 8);

        let bad_hdr = r#"{"model.norm.weight":{"dtype":"BF16","shape":[4],"data_offsets":[0,16]}}"#;
        let mut bad = Vec::new();
        bad.extend_from_slice(&(bad_hdr.len() as u64).to_le_bytes());
        bad.extend_from_slice(bad_hdr.as_bytes());
        let err = parse_safetensors_header(&bad).expect_err("size mismatch must fail");
        assert_eq!(
            err.verdict(),
            KappaStoreVerdict::RefusedCorruptSafetensorsHeader
        );
    }

    #[test]
    fn holoslab2_seal_enforces_quota_merkle_and_piece_hashes() {
        let p0 = HoloSlab2PieceEntry {
            index: 0,
            byte_offset: 0,
            byte_length: 1024,
            sha256: sha256_hex(b"piece-0"),
        };
        let p1 = HoloSlab2PieceEntry {
            index: 1,
            byte_offset: 1024,
            byte_length: 512,
            sha256: sha256_hex(b"piece-1"),
        };
        let pieces = vec![p0, p1];
        let merkle = compute_holoslab2_merkle_root(&pieces);
        let seal = HoloSlab2Seal {
            schema: HOLOSLAB2_SCHEMA.to_string(),
            magic: HOLOSLAB2_MAGIC.to_string(),
            container_sha256: "a".repeat(64),
            merkle_root_sha256: merkle,
            total_bytes: 1536,
            piece_count: 2,
            model_id: "uor-r4.qwen3-0.6b-holo".to_string(),
            pieces,
        };

        let warm = validate_holoslab2_mount(&seal, 4096, 1536, 1536, false, true);
        assert!(warm.ok);
        assert_eq!(warm.verdict, "SlabSealAdmitted");

        let full = validate_holoslab2_mount(&seal, 4096, 1536, 1536, true, true);
        assert!(full.ok);
        assert_eq!(full.verdict, "SlabFullMerkleVerified");

        let low_quota = validate_holoslab2_mount(&seal, 1000, 1536, 1536, false, true);
        assert!(!low_quota.ok);
        assert_eq!(low_quota.verdict, "RefusedInsufficientStorageQuota");

        let bad_size = validate_holoslab2_mount(&seal, 4096, 1536, 1535, false, true);
        assert!(!bad_size.ok);
        assert_eq!(bad_size.verdict, "RefusedSlabSealMismatch");

        let bad_piece = validate_holoslab2_mount(&seal, 4096, 1536, 1536, true, false);
        assert!(!bad_piece.ok);
        assert_eq!(bad_piece.verdict, "RefusedSlabPieceHashMismatch");
    }
}
