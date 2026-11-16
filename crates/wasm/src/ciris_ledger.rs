#![forbid(unsafe_code)]
//! Deterministic `CIRISKernel` content-addressed causal ledger (`R4.CirisLedger`).
//!
//! Aligns `cirisai.github.io/CIRISKernel/` causal row graphs, content-addressed SHA-256 digests,
//! topological parent admission, monotonic `kept` flags, and causal cut fixed-point computation
//! with `uor-r4` pure-integer / zero-float state machines (`R5`).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};

/// Canonical schema identifier for `CIRISKernel` causal ledger snapshots and rows.
pub const CIRIS_LEDGER_SCHEMA: &str = "ciris/causal-ledger/1";

/// Typed negative failure modes for causal row admission and cut verification (`R5`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CirisLedgerError {
    /// Incoming JSON payload could not be deserialized as a `CirisRow`.
    InvalidRowJson(String),
    /// A digest field was not a lowercase 64-hex SHA-256 string or `kappa` was empty.
    InvalidDigestFormat {
        /// Name of the offending field.
        field: &'static str,
        /// Supplied value that failed validation.
        value: String,
    },
    /// Supplied `row_digest` disagreed with canonical content-addressed hash.
    DigestMismatch {
        /// Canonical expected SHA-256 hex digest.
        expected: String,
        /// Digest supplied on the row.
        actual: String,
    },
    /// One or more declared parent digests are absent from the ledger.
    MissingParent {
        /// Missing parent row digest.
        parent_digest: String,
    },
    /// A different row digest attempted to claim an already-committed sequence slot.
    DuplicateConflictingSequence {
        /// Sequence number already occupied.
        sequence: u64,
        /// Existing committed digest.
        existing_digest: String,
        /// Conflicting incoming digest.
        incoming_digest: String,
    },
    /// Causal cycle, backward sequence edge, or non-monotonic `kept` state rejected.
    CycleOrNonMonotonicCut {
        /// Row digest involved in the invariant failure.
        row_digest: String,
        /// Diagnostic detail explaining the cut/DAG violation.
        detail: String,
    },
}

impl CirisLedgerError {
    /// Formal R4.CirisLedger stable machine-readable error code (`R5`).
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidRowJson(_) => "InvalidRowJson",
            Self::InvalidDigestFormat { .. } => "InvalidDigestFormat",
            Self::DigestMismatch { .. } => "DigestMismatch",
            Self::MissingParent { .. } => "MissingParent",
            Self::DuplicateConflictingSequence { .. } => "DuplicateConflictingSequence",
            Self::CycleOrNonMonotonicCut { .. } => "CycleOrNonMonotonicCut",
        }
    }

    /// Formal R4.CirisLedger JSON representation of a typed rejection.
    pub fn to_error_json(&self) -> String {
        serde_json::json!({
            "status": "error",
            "schema": CIRIS_LEDGER_SCHEMA,
            "error_code": self.error_code(),
            "message": self.to_string(),
        })
        .to_string()
    }
}

impl fmt::Display for CirisLedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRowJson(msg) => write!(f, "invalid CirisRow JSON: {msg}"),
            Self::InvalidDigestFormat { field, value } => {
                write!(f, "invalid format for `{field}`: `{value}`")
            }
            Self::DigestMismatch { expected, actual } => {
                write!(f, "row_digest mismatch: expected {expected}, got {actual}")
            }
            Self::MissingParent { parent_digest } => {
                write!(f, "missing parent row in causal ledger: {parent_digest}")
            }
            Self::DuplicateConflictingSequence {
                sequence,
                existing_digest,
                incoming_digest,
            } => write!(
                f,
                "sequence {sequence} already committed as {existing_digest}, cannot overwrite with {incoming_digest}"
            ),
            Self::CycleOrNonMonotonicCut { row_digest, detail } => {
                write!(f, "causal cut / monotonicity violation at {row_digest}: {detail}")
            }
        }
    }
}

/// Content-addressed causal DAG record in the `CIRISKernel` ledger.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CirisRow {
    /// Lowercase hex SHA-256 digest over schema, sequence, kappa, payload_digest, and sorted parents.
    pub row_digest: String,
    /// Parent row digests establishing causal happens-before edges.
    pub parents: Vec<String>,
    /// Canonical `kappa` path / capability identifier associated with this row.
    pub kappa: String,
    /// Lowercase hex SHA-256 digest of the payload bytes.
    pub payload_digest: String,
    /// Strictly increasing causal sequence number relative to parents.
    pub sequence: u64,
    /// Monotonic retention bit indicating whether this row belongs to the active kept frontier.
    pub kept: bool,
}

impl CirisRow {
    /// Formal R4.CirisLedger constructor computing canonical payload and row SHA-256 digests.
    pub fn build(
        parents: Vec<String>,
        kappa: impl Into<String>,
        payload: &[u8],
        sequence: u64,
        kept: bool,
    ) -> Self {
        let kappa_str = kappa.into();
        let payload_digest = sha256_hex_bytes(payload);
        let row_digest = compute_row_digest(&parents, &kappa_str, &payload_digest, sequence);
        Self {
            row_digest,
            parents,
            kappa: kappa_str,
            payload_digest,
            sequence,
            kept,
        }
    }
}

/// Formal R4.CirisLedger lowercase SHA-256 hex digest helper.
pub fn sha256_hex_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

/// Formal R4.CirisLedger check that a string is 64 lowercase hexadecimal characters.
fn is_valid_sha256_hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}

/// Formal R4.CirisLedger canonical row digest over domain tag, sequence, kappa, payload, and parents.
pub fn compute_row_digest(
    parents: &[String],
    kappa: &str,
    payload_digest: &str,
    sequence: u64,
) -> String {
    let mut canonical_parents: Vec<&str> = parents.iter().map(String::as_str).collect();
    canonical_parents.sort_unstable();
    let mut hasher = Sha256::new();
    hasher.update(CIRIS_LEDGER_SCHEMA.as_bytes());
    hasher.update(b"\nseq:");
    hasher.update(sequence.to_le_bytes());
    hasher.update(b"\nkappa:");
    hasher.update(kappa.as_bytes());
    hasher.update(b"\npayload:");
    hasher.update(payload_digest.as_bytes());
    hasher.update(b"\nparents:");
    for p in canonical_parents {
        hasher.update(p.as_bytes());
        hasher.update(b";");
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

/// Stateful deterministic `CIRISKernel` causal ledger supporting DAG admission and fixed-point cuts.
#[derive(Clone, Debug, Default)]
pub struct CirisCausalLedger {
    rows: BTreeMap<String, CirisRow>,
    by_sequence: BTreeMap<u64, String>,
    head_cut: Vec<String>,
}

impl CirisCausalLedger {
    /// Formal R4.CirisLedger constructor returning an empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Formal R4.CirisLedger row count query.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Formal R4.CirisLedger empty predicate.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Formal R4.CirisLedger active causal cut tip digests.
    pub fn head_cut(&self) -> &[String] {
        &self.head_cut
    }

    /// Formal R4.CirisLedger lookup by row digest.
    pub fn get(&self, row_digest: &str) -> Option<&CirisRow> {
        self.rows.get(row_digest)
    }

    /// Formal R4.CirisLedger admit JSON-serialized row, returning status JSON or typed R5 error JSON.
    pub fn admit_row_json(&mut self, row_json: &str) -> String {
        let parsed: CirisRow = match serde_json::from_str(row_json.trim()) {
            Ok(r) => r,
            Err(e) => return CirisLedgerError::InvalidRowJson(e.to_string()).to_error_json(),
        };
        let digest = parsed.row_digest.clone();
        match self.admit_row(parsed) {
            Ok(_) => {
                let cut_sha = self.head_cut_digest();
                serde_json::json!({
                    "status": "ok",
                    "schema": CIRIS_LEDGER_SCHEMA,
                    "admitted_digest": digest,
                    "row_count": self.rows.len(),
                    "kept_count": self.rows.values().filter(|r| r.kept).count(),
                    "cut_tips": self.head_cut,
                    "head_cut_sha256": cut_sha,
                })
                .to_string()
            }
            Err(err) => err.to_error_json(),
        }
    }

    /// Formal R4.CirisLedger validate and admit a causal `CirisRow`, updating the cut fixed point.
    pub fn admit_row(&mut self, row: CirisRow) -> Result<&CirisRow, CirisLedgerError> {
        if row.kappa.trim().is_empty() {
            return Err(CirisLedgerError::InvalidDigestFormat {
                field: "kappa",
                value: row.kappa,
            });
        }
        if !is_valid_sha256_hex(&row.row_digest) {
            return Err(CirisLedgerError::InvalidDigestFormat {
                field: "row_digest",
                value: row.row_digest,
            });
        }
        if !is_valid_sha256_hex(&row.payload_digest) {
            return Err(CirisLedgerError::InvalidDigestFormat {
                field: "payload_digest",
                value: row.payload_digest,
            });
        }
        for p in &row.parents {
            if !is_valid_sha256_hex(p) {
                return Err(CirisLedgerError::InvalidDigestFormat {
                    field: "parents[]",
                    value: p.clone(),
                });
            }
            if p == &row.row_digest {
                return Err(CirisLedgerError::CycleOrNonMonotonicCut {
                    row_digest: row.row_digest.clone(),
                    detail: "self-parent cycle forbidden".to_string(),
                });
            }
        }

        let expected =
            compute_row_digest(&row.parents, &row.kappa, &row.payload_digest, row.sequence);
        if expected != row.row_digest {
            return Err(CirisLedgerError::DigestMismatch {
                expected,
                actual: row.row_digest,
            });
        }

        if let Some(existing_digest) = self.by_sequence.get(&row.sequence) {
            if existing_digest != &row.row_digest {
                return Err(CirisLedgerError::DuplicateConflictingSequence {
                    sequence: row.sequence,
                    existing_digest: existing_digest.clone(),
                    incoming_digest: row.row_digest,
                });
            }
            if let Some(existing) = self.rows.get(existing_digest) {
                if existing.kept && !row.kept {
                    return Err(CirisLedgerError::CycleOrNonMonotonicCut {
                        row_digest: row.row_digest,
                        detail: "cannot flip a previously kept row to kept=false".to_string(),
                    });
                }
                return Ok(self.rows.get(existing_digest).unwrap());
            }
        }

        for p in &row.parents {
            let parent = self
                .rows
                .get(p)
                .ok_or_else(|| CirisLedgerError::MissingParent {
                    parent_digest: p.clone(),
                })?;
            if parent.sequence >= row.sequence {
                return Err(CirisLedgerError::CycleOrNonMonotonicCut {
                    row_digest: row.row_digest.clone(),
                    detail: format!(
                        "parent sequence {} is not strictly less than child sequence {}",
                        parent.sequence, row.sequence
                    ),
                });
            }
            if row.kept && !parent.kept {
                return Err(CirisLedgerError::CycleOrNonMonotonicCut {
                    row_digest: row.row_digest.clone(),
                    detail: format!("kept row cannot descend from non-kept parent {p}"),
                });
            }
        }

        let key = row.row_digest.clone();
        self.by_sequence.insert(row.sequence, key.clone());
        self.rows.insert(key.clone(), row);
        self.head_cut = self.compute_causal_cut()?;
        Ok(self.rows.get(&key).unwrap())
    }

    /// Formal R4.CirisLedger fixed-point causal cut computation verifying DAG closure & monotonicity.
    pub fn compute_causal_cut(&self) -> Result<Vec<String>, CirisLedgerError> {
        // Iteratively reach the fixed point of grounded acyclic rows starting from root/closed ancestors.
        let mut grounded: BTreeSet<String> = BTreeSet::new();
        loop {
            let mut progressed = false;
            for (digest, row) in &self.rows {
                if grounded.contains(digest) {
                    continue;
                }
                let mut parents_ok = true;
                for p in &row.parents {
                    let Some(pr) = self.rows.get(p) else {
                        return Err(CirisLedgerError::MissingParent {
                            parent_digest: p.clone(),
                        });
                    };
                    if row.kept && !pr.kept {
                        return Err(CirisLedgerError::CycleOrNonMonotonicCut {
                            row_digest: digest.clone(),
                            detail: format!("non-monotonic kept dependency on {p}"),
                        });
                    }
                    if !grounded.contains(p) {
                        parents_ok = false;
                        break;
                    }
                }
                if parents_ok {
                    grounded.insert(digest.clone());
                    progressed = true;
                }
            }
            if !progressed {
                break;
            }
        }

        if grounded.len() != self.rows.len() {
            let ungrounded = self
                .rows
                .keys()
                .find(|k| !grounded.contains(*k))
                .cloned()
                .unwrap_or_default();
            return Err(CirisLedgerError::CycleOrNonMonotonicCut {
                row_digest: ungrounded,
                detail: "cyclic or ungrounded causal subgraph detected".to_string(),
            });
        }

        let mut shadowed_kept: BTreeSet<String> = BTreeSet::new();
        for row in self.rows.values() {
            if row.kept {
                for p in &row.parents {
                    shadowed_kept.insert(p.clone());
                }
            }
        }

        let tips: Vec<String> = self
            .rows
            .values()
            .filter(|row| row.kept && !shadowed_kept.contains(&row.row_digest))
            .map(|row| row.row_digest.clone())
            .collect();
        Ok(tips)
    }

    /// Formal R4.CirisLedger aggregate SHA-256 digest over the sorted causal cut tips.
    pub fn head_cut_digest(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(CIRIS_LEDGER_SCHEMA.as_bytes());
        for tip in &self.head_cut {
            hasher.update(b"\n");
            hasher.update(tip.as_bytes());
        }
        let digest = hasher.finalize();
        let mut out = String::with_capacity(64);
        for b in digest {
            let _ = write!(&mut out, "{b:02x}");
        }
        out
    }

    /// Formal R4.CirisLedger JSON status snapshot for Wasm bridge and inspector panels.
    pub fn status_json(&self) -> String {
        serde_json::json!({
            "status": "ok",
            "schema": CIRIS_LEDGER_SCHEMA,
            "row_count": self.rows.len(),
            "kept_count": self.rows.values().filter(|r| r.kept).count(),
            "cut_tips": self.head_cut,
            "head_cut_sha256": self.head_cut_digest(),
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{CirisCausalLedger, CirisLedgerError, CirisRow};

    #[test]
    fn test_ciris_causal_admission_diamond_cut_and_negative_rejections() {
        let mut ledger = CirisCausalLedger::new();
        assert!(ledger.is_empty());

        let r0 = CirisRow::build(vec![], "r4:v1:ciris:genesis", b"genesis", 1, true);
        let d0 = r0.row_digest.clone();
        ledger.admit_row(r0).expect("admit genesis row");
        assert_eq!(ledger.head_cut(), std::slice::from_ref(&d0));

        let r1 = CirisRow::build(
            vec![d0.clone()],
            "r4:v1:ciris:branch-a",
            b"branch-a",
            2,
            true,
        );
        let d1 = r1.row_digest.clone();
        ledger.admit_row(r1).expect("admit branch-a");

        let r2 = CirisRow::build(vec![d0], "r4:v1:ciris:branch-b", b"branch-b", 3, true);
        let d2 = r2.row_digest.clone();
        ledger.admit_row(r2).expect("admit branch-b");
        assert_eq!(ledger.head_cut().len(), 2);

        let r3 = CirisRow::build(
            vec![d1.clone(), d2.clone()],
            "r4:v1:ciris:merge",
            b"diamond-merge",
            4,
            true,
        );
        let d3 = r3.row_digest.clone();
        ledger.admit_row(r3).expect("admit diamond merge");
        assert_eq!(ledger.head_cut(), std::slice::from_ref(&d3));

        // Negative 1: Missing parent
        let bad_parent = "f".repeat(64);
        let r_missing = CirisRow::build(vec![bad_parent.clone()], "r4:v1:k", b"x", 5, true);
        assert_eq!(
            ledger.admit_row(r_missing),
            Err(CirisLedgerError::MissingParent {
                parent_digest: bad_parent
            })
        );

        // Negative 2: Digest mismatch
        let mut r_tampered = CirisRow::build(vec![d3.clone()], "r4:v1:k", b"y", 5, true);
        r_tampered.row_digest = "0".repeat(64);
        assert_eq!(
            ledger.admit_row(r_tampered).unwrap_err().error_code(),
            "DigestMismatch"
        );

        // Negative 3: Invalid digest format / empty kappa
        let mut r_bad_fmt = CirisRow::build(vec![d3.clone()], "r4:v1:k", b"z", 5, true);
        r_bad_fmt.row_digest = "not-hex".to_string();
        assert_eq!(
            ledger.admit_row(r_bad_fmt).unwrap_err().error_code(),
            "InvalidDigestFormat"
        );

        // Negative 4: Duplicate conflicting sequence
        let r_dup_seq = CirisRow::build(vec![d3.clone()], "r4:v1:ciris:conflict", b"dup", 4, true);
        assert_eq!(
            ledger.admit_row(r_dup_seq).unwrap_err().error_code(),
            "DuplicateConflictingSequence"
        );

        // Negative 5: Non-monotonic kept dependency (kept=true descending from kept=false)
        let r_unkept = CirisRow::build(vec![d3], "r4:v1:ciris:pruned", b"pruned", 5, false);
        let d_unkept = r_unkept.row_digest.clone();
        ledger.admit_row(r_unkept).expect("admit non-kept row");
        assert_eq!(ledger.head_cut().len(), 1);
        let r_non_mono = CirisRow::build(vec![d_unkept], "r4:v1:ciris:bad", b"bad", 6, true);
        assert_eq!(
            ledger.admit_row(r_non_mono).unwrap_err().error_code(),
            "CycleOrNonMonotonicCut"
        );

        // Negative 6: Invalid JSON via public JSON boundary
        let bad_json = ledger.admit_row_json("{invalid");
        assert!(bad_json.contains("InvalidRowJson"));
    }
}
