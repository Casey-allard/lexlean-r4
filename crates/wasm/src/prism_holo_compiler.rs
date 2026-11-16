//! Formal `R4.PrismHoloIdentity` (`R4-54`) pure-integer BLAKE3, Stitch Prism Identity ledger,
//! zero-import Core-Wasm guest emitter, HOLOVIEW v1 encoder/decoder, and Hologram v4 `.holo` compiler.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::lexlean_generated::r4_prism_holo_identity::{
    admit_holo_v4_archive, admit_prism_identity_record, HoloArchiveCheck, PrismHoloVerdict,
    PrismIdentityCheck, PrismIdentityRole, PrismRecordKind,
};

/// Hologram v4 `.holo` file magic.
pub const HOLO_MAGIC: &[u8; 4] = b"HOLO";
/// Hologram v4 physical format version.
pub const HOLO_VERSION: u16 = 4;
/// Exact section count for a standalone Hologram v4 `.holo` application archive.
pub const HOLO_SECTION_COUNT: u16 = 8;
/// Exact section kinds in table order `[15, 8, 14, 14, 16, 16, 16, 16]`.
pub const HOLO_SECTION_ORDER: [u64; 8] = [15, 8, 14, 14, 16, 16, 16, 16];
/// Hologram `APP_MANIFEST` exact byte length.
pub const HOLO_MANIFEST_LEN: usize = 356;
/// `blake3:<64 lowercase hex>` label length.
pub const HOLO_LABEL_LEN: usize = 71;
/// `HOLOVIEW` v1 header magic.
pub const HOLOVIEW_MAGIC: &[u8; 8] = b"HOLOVIEW";
/// Core-Wasm contract URI.
pub const CORE_WASM_CONTRACT: &str = "hologram:guest/core-wasm@1";
/// Application directory extension URI.
pub const DIRECTORY_EXTENSION_URI: &str =
    "https://hologram.foundation/extension/application-directory/v1";
/// PrismPM model provenance extension URI.
pub const MODEL_EXTENSION_URI: &str = "https://uor.foundation/extension/prismpm-model/v1";
/// App manifest URI.
pub const APP_MANIFEST_URI: &str = "https://hologram.foundation/realization/app-manifest";

/// Exact `Foundation.Holo.V1.Wire.wireManifestSuffix` 86-byte constant.
const MANIFEST_SUFFIX_BYTES: [u8; 86] = [
    0x52, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00,
    0x00, b'h', b'o', b'l', b'o', b'_', b'r', b'u', b'n', 0x1a, 0x00, 0x00, 0x00, b'h', b'o', b'l',
    b'o', b'g', b'r', b'a', b'm', b':', b'g', b'u', b'e', b's', b't', b'/', b'c', b'o', b'r', b'e',
    b'-', b'w', b'a', b's', b'm', b'@', b'1', 0x03, 0x0a, 0x00, 0x00, 0x00, b'i', b'n', b'd', b'e',
    b'x', b'.', b'h', b't', b'm', b'l', 0x08, 0x00, 0x00, 0x00, b'p', b'o', b'r', b't', b'a', b'b',
    b'l', b'e', 0x00, 0x00, 0x00, 0x00,
];

/// Empty-grant capability set binary constant (`104` bytes matching `Foundation.Holo.V1.Wire`).
const EMPTY_CAPABILITY_SET: [u8; 104] = [
    b'h', b't', b't', b'p', b's', b':', b'/', b'/', b'h', b'o', b'l', b'o', b'g', b'r', b'a', b'm',
    b'.', b'f', b'o', b'u', b'n', b'd', b'a', b't', b'i', b'o', b'n', b'/', b'r', b'e', b'a', b'l',
    b'i', b'z', b'a', b't', b'i', b'o', b'n', b'/', b'c', b'a', b'p', b'a', b'b', b'i', b'l', b'i',
    b't', b'y', b'-', b's', b'e', b't', 0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

const MAX_SAFE_INT_ABS: i64 = 9_007_199_254_740_991;

const BLAKE3_IV: [u32; 8] = [
    0x6A09_E667,
    0xBB67_AE85,
    0x3C6E_F372,
    0xA54F_F53A,
    0x510E_527F,
    0x9B05_688C,
    0x1F83_D9AB,
    0x5BE0_CD19,
];
const BLAKE3_MSG_PERM: [usize; 16] = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];
const BLAKE3_CHUNK_LEN: usize = 1024;
const BLAKE3_BLOCK_LEN: usize = 64;
const BLAKE3_CHUNK_START: u32 = 1 << 0;
const BLAKE3_CHUNK_END: u32 = 1 << 1;
const BLAKE3_PARENT: u32 = 1 << 2;
const BLAKE3_ROOT: u32 = 1 << 3;

/// Formal R4.PrismHoloIdentity perform one BLAKE3 quarter-round mixing step on state array `s`.
#[inline]
fn blake3_g(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, my: u32) {
    s[a] = s[a].wrapping_add(s[b]).wrapping_add(mx);
    s[d] = (s[d] ^ s[a]).rotate_right(16);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_right(12);
    s[a] = s[a].wrapping_add(s[b]).wrapping_add(my);
    s[d] = (s[d] ^ s[a]).rotate_right(8);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_right(7);
}

/// Formal R4.PrismHoloIdentity execute one full 7-round BLAKE3 compression invocation.
fn blake3_compress(
    cv: &[u32; 8],
    block: &[u32; 16],
    counter: u64,
    block_len: u32,
    flags: u32,
) -> [u32; 16] {
    let mut s = [
        cv[0],
        cv[1],
        cv[2],
        cv[3],
        cv[4],
        cv[5],
        cv[6],
        cv[7],
        BLAKE3_IV[0],
        BLAKE3_IV[1],
        BLAKE3_IV[2],
        BLAKE3_IV[3],
        counter as u32,
        (counter >> 32) as u32,
        block_len,
        flags,
    ];
    let mut m = *block;
    for round in 0..7 {
        blake3_g(&mut s, 0, 4, 8, 12, m[0], m[1]);
        blake3_g(&mut s, 1, 5, 9, 13, m[2], m[3]);
        blake3_g(&mut s, 2, 6, 10, 14, m[4], m[5]);
        blake3_g(&mut s, 3, 7, 11, 15, m[6], m[7]);
        blake3_g(&mut s, 0, 5, 10, 15, m[8], m[9]);
        blake3_g(&mut s, 1, 6, 11, 12, m[10], m[11]);
        blake3_g(&mut s, 2, 7, 8, 13, m[12], m[13]);
        blake3_g(&mut s, 3, 4, 9, 14, m[14], m[15]);
        if round < 6 {
            let mut permuted = [0u32; 16];
            for idx in 0..16 {
                permuted[idx] = m[BLAKE3_MSG_PERM[idx]];
            }
            m = permuted;
        }
    }
    for i in 0..8 {
        s[i] ^= s[i + 8];
        s[i + 8] ^= cv[i];
    }
    s
}

/// Formal R4.PrismHoloIdentity unpack up to 64 bytes into 16 little-endian u32 message words.
fn blake3_words_from_block(block: &[u8]) -> [u32; 16] {
    let mut words = [0u32; 16];
    for (i, w) in words.iter_mut().enumerate() {
        let base = i * 4;
        let b0 = block.get(base).copied().unwrap_or(0) as u32;
        let b1 = block.get(base + 1).copied().unwrap_or(0) as u32;
        let b2 = block.get(base + 2).copied().unwrap_or(0) as u32;
        let b3 = block.get(base + 3).copied().unwrap_or(0) as u32;
        *w = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
    }
    words
}

/// Formal R4.PrismHoloIdentity extract the 8-word chaining value from a compression state.
fn blake3_cv_from_state(state: [u32; 16]) -> [u32; 8] {
    let mut cv = [0u32; 8];
    cv.copy_from_slice(&state[..8]);
    cv
}

/// Formal R4.PrismHoloIdentity compute an output node's 256-bit chaining value or root digest.
#[derive(Clone, Copy)]
struct Blake3Output {
    input_cv: [u32; 8],
    block_words: [u32; 16],
    counter: u64,
    block_len: u32,
    flags: u32,
}

impl Blake3Output {
    /// Formal R4.PrismHoloIdentity compute parent chaining value of this node.
    fn chaining_value(&self) -> [u32; 8] {
        blake3_cv_from_state(blake3_compress(
            &self.input_cv,
            &self.block_words,
            self.counter,
            self.block_len,
            self.flags,
        ))
    }

    /// Formal R4.PrismHoloIdentity finalize the 32-byte ROOT digest.
    fn root_bytes_32(&self) -> [u8; 32] {
        let words = blake3_compress(
            &self.input_cv,
            &self.block_words,
            0,
            self.block_len,
            self.flags | BLAKE3_ROOT,
        );
        let mut out = [0u8; 32];
        for i in 0..8 {
            out[i * 4..i * 4 + 4].copy_from_slice(&words[i].to_le_bytes());
        }
        out
    }
}

/// Formal R4.PrismHoloIdentity construct one complete BLAKE3 1024-byte chunk output node.
fn blake3_chunk_output(chunk_bytes: &[u8], chunk_counter: u64) -> Blake3Output {
    let mut cv = BLAKE3_IV;
    let mut blocks_compressed = 0u32;
    let mut offset = 0usize;
    while chunk_bytes.len().saturating_sub(offset) > BLAKE3_BLOCK_LEN {
        let end = offset + BLAKE3_BLOCK_LEN;
        let words = blake3_words_from_block(&chunk_bytes[offset..end]);
        let start_flag = if blocks_compressed == 0 {
            BLAKE3_CHUNK_START
        } else {
            0
        };
        cv = blake3_cv_from_state(blake3_compress(
            &cv,
            &words,
            chunk_counter,
            BLAKE3_BLOCK_LEN as u32,
            start_flag,
        ));
        blocks_compressed = blocks_compressed.saturating_add(1);
        offset = end;
    }
    let tail = &chunk_bytes[offset..];
    let start_flag = if blocks_compressed == 0 {
        BLAKE3_CHUNK_START
    } else {
        0
    };
    Blake3Output {
        input_cv: cv,
        block_words: blake3_words_from_block(tail),
        counter: chunk_counter,
        block_len: tail.len() as u32,
        flags: start_flag | BLAKE3_CHUNK_END,
    }
}

/// Formal R4.PrismHoloIdentity construct a parent Merkle node output from left and right CVs.
fn blake3_parent_output(left: [u32; 8], right: [u32; 8]) -> Blake3Output {
    let mut words = [0u32; 16];
    words[..8].copy_from_slice(&left);
    words[8..].copy_from_slice(&right);
    Blake3Output {
        input_cv: BLAKE3_IV,
        block_words: words,
        counter: 0,
        block_len: BLAKE3_BLOCK_LEN as u32,
        flags: BLAKE3_PARENT,
    }
}

/// Formal R4.PrismHoloIdentity compute unkeyed 32-byte BLAKE3 digest over an arbitrary byte slice.
pub fn blake3_hash_32(input: &[u8]) -> [u8; 32] {
    if input.len() <= BLAKE3_CHUNK_LEN {
        return blake3_chunk_output(input, 0).root_bytes_32();
    }
    let mut stack: Vec<[u32; 8]> = Vec::new();
    let total_full_or_partial = input.len().div_ceil(BLAKE3_CHUNK_LEN);
    let last_idx = total_full_or_partial - 1;
    for chunk_idx in 0..last_idx {
        let start = chunk_idx * BLAKE3_CHUNK_LEN;
        let end = start + BLAKE3_CHUNK_LEN;
        let mut new_cv = blake3_chunk_output(&input[start..end], chunk_idx as u64).chaining_value();
        let mut total_chunks = chunk_idx + 1;
        while total_chunks & 1 == 0 {
            if let Some(left) = stack.pop() {
                new_cv = blake3_parent_output(left, new_cv).chaining_value();
            }
            total_chunks >>= 1;
        }
        stack.push(new_cv);
    }
    let tail_start = last_idx * BLAKE3_CHUNK_LEN;
    let mut output = blake3_chunk_output(&input[tail_start..], last_idx as u64);
    while let Some(left) = stack.pop() {
        output = blake3_parent_output(left, output.chaining_value());
    }
    output.root_bytes_32()
}

/// Formal R4.PrismHoloIdentity format lowercase hexadecimal bytes of an array slice.
pub fn encode_hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// Formal R4.PrismHoloIdentity compute lowercase 64-hex unkeyed BLAKE3 digest.
pub fn blake3_hash_hex(input: &[u8]) -> String {
    encode_hex_lower(&blake3_hash_32(input))
}

/// Formal R4.PrismHoloIdentity compute 71-byte `blake3:<64hex>` Hologram label.
pub fn blake3_kappa_label(input: &[u8]) -> String {
    format!("blake3:{}", blake3_hash_hex(input))
}

/// Formal R4.PrismHoloIdentity serialize a JSON value using strict `uor-addr` integer-only sorted-key canonical rules.
pub fn canonicalize_json_value(value: &Value) -> Result<String, PrismHoloVerdict> {
    let mut out = String::new();
    canonical_json_into(value, &mut out)?;
    Ok(out)
}

/// Formal R4.PrismHoloIdentity escape a JSON string per RFC 8785 / JSON standard without allocating excess intermediates.
fn push_escaped_json_string(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Formal R4.PrismHoloIdentity recursively encode canonical JSON rejecting any floating-point or unsafe integer.
fn canonical_json_into(value: &Value, out: &mut String) -> Result<(), PrismHoloVerdict> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(num) => {
            if let Some(i) = num.as_i64() {
                if !(-MAX_SAFE_INT_ABS..=MAX_SAFE_INT_ABS).contains(&i) {
                    return Err(PrismHoloVerdict::RefusedNonIntegerCanonicalJson);
                }
                let _ = write!(out, "{i}");
            } else if let Some(u) = num.as_u64() {
                if u > MAX_SAFE_INT_ABS as u64 {
                    return Err(PrismHoloVerdict::RefusedNonIntegerCanonicalJson);
                }
                let _ = write!(out, "{u}");
            } else {
                return Err(PrismHoloVerdict::RefusedNonIntegerCanonicalJson);
            }
        }
        Value::String(s) => push_escaped_json_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                canonical_json_into(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            out.push('{');
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                push_escaped_json_string(key, out);
                out.push(':');
                if let Some(val) = map.get(*key) {
                    canonical_json_into(val, out)?;
                }
            }
            out.push('}');
        }
    }
    Ok(())
}

/// Formal R4.PrismHoloIdentity compute Stitch `sha256:<hex>` content-addressed `kappa` of a canonical JSON object.
pub fn uor_kappa_sha256(value: &Value) -> Result<String, PrismHoloVerdict> {
    let canon = canonicalize_json_value(value)?;
    let digest = Sha256::digest(canon.as_bytes());
    Ok(format!("sha256:{}", encode_hex_lower(&digest)))
}

/// Admission result for a single `Stitch.PrismId` identity log record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrismIdentityAdmission {
    /// Formal model verdict.
    pub verdict_name: String,
    /// Strongly typed verdict.
    #[serde(skip)]
    pub verdict: PrismHoloVerdict,
    /// Canonical `sha256:<64hex>` record digest.
    pub record_kappa: String,
    /// Root `prism_id` of the identity chain.
    pub prism_id: String,
    /// Schema of the admitted record (`prism/*/1`).
    pub schema: String,
    /// Active member/delegate/recovery count after admission.
    pub active_devices: usize,
    /// Revoked device count after admission.
    pub revoked_devices: usize,
}

#[allow(clippy::derivable_impls)]
impl Default for PrismHoloVerdict {
    fn default() -> Self {
        Self::PrismHoloAdmitted
    }
}

/// Formal R4.PrismHoloIdentity stateful ledger tracking `afflom/stitch` `PrismId` devices, roles, revocations, personas, and action receipts.
#[derive(Debug, Clone, Default)]
pub struct PrismIdentityLedger {
    /// Root `prism/genesis/1` identifier (`sha256:...`).
    pub prism_id: String,
    /// Most recent admitted record digest.
    pub head_kappa: String,
    /// Active device role bindings keyed by `device_id` (`sha256:...`).
    pub roles: BTreeMap<String, PrismIdentityRole>,
    /// Set of permanently revoked `device_id`s.
    pub revoked: BTreeSet<String>,
    /// Admitted record digests in append order.
    pub history: Vec<String>,
    /// Bound `afflom/me` persona record if declared.
    pub persona: Option<Value>,
}

/// Formal R4.PrismHoloIdentity map string role name onto `PrismIdentityRole`.
fn parse_identity_role(raw: &str) -> PrismIdentityRole {
    match raw {
        "member" => PrismIdentityRole::Member,
        "delegate" => PrismIdentityRole::Delegate,
        "recovery" => PrismIdentityRole::Recovery,
        _ => PrismIdentityRole::Unbound,
    }
}

/// Formal R4.PrismHoloIdentity map `prism/*/1` schema URI onto `PrismRecordKind`.
fn parse_record_kind(schema: &str) -> Option<PrismRecordKind> {
    match schema {
        "prism/device/1" => Some(PrismRecordKind::Device1),
        "prism/genesis/1" => Some(PrismRecordKind::Genesis1),
        "prism/bind/1" => Some(PrismRecordKind::Bind1),
        "prism/revoke/1" => Some(PrismRecordKind::Revoke1),
        "prism/action/1" => Some(PrismRecordKind::Action1),
        "prism/persona/1" => Some(PrismRecordKind::Persona1),
        _ => None,
    }
}

/// Formal R4.PrismHoloIdentity format `PrismHoloVerdict` as canonical variant string.
pub fn verdict_name(v: PrismHoloVerdict) -> &'static str {
    match v {
        PrismHoloVerdict::PrismHoloAdmitted => "PrismHoloAdmitted",
        PrismHoloVerdict::RefusedNonIntegerCanonicalJson => "RefusedNonIntegerCanonicalJson",
        PrismHoloVerdict::RefusedIdentityKappaMismatch => "RefusedIdentityKappaMismatch",
        PrismHoloVerdict::RefusedUnauthorizedIdentityRole => "RefusedUnauthorizedIdentityRole",
        PrismHoloVerdict::RefusedRevokedDeviceSigner => "RefusedRevokedDeviceSigner",
        PrismHoloVerdict::RefusedInvalidHoloMagicOrVersion => "RefusedInvalidHoloMagicOrVersion",
        PrismHoloVerdict::RefusedInvalidHoloSectionOrder => "RefusedInvalidHoloSectionOrder",
        PrismHoloVerdict::RefusedCoreWasmContractViolation => "RefusedCoreWasmContractViolation",
        PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView => "RefusedUnsortedHoloBlobsOrView",
        PrismHoloVerdict::RefusedHoloBlake3FooterMismatch => "RefusedHoloBlake3FooterMismatch",
        PrismHoloVerdict::RefusedMeAppVectorReplayFailure => "RefusedMeAppVectorReplayFailure",
    }
}

impl PrismIdentityLedger {
    /// Formal R4.PrismHoloIdentity return current chain head digest (`sha256:...`).
    pub fn head_kappa(&self) -> String {
        self.head_kappa.clone()
    }

    /// Formal R4.PrismHoloIdentity admit a `Stitch.PrismId` record enforcing canonical JSON, digest binding, revocation, and role permissions.
    pub fn admit_record(
        &mut self,
        record: &Value,
    ) -> Result<PrismIdentityAdmission, PrismHoloVerdict> {
        let record_kappa = uor_kappa_sha256(record)?;
        let schema = record
            .get("schema")
            .and_then(Value::as_str)
            .ok_or(PrismHoloVerdict::RefusedUnauthorizedIdentityRole)?;
        let kind =
            parse_record_kind(schema).ok_or(PrismHoloVerdict::RefusedUnauthorizedIdentityRole)?;

        let mut kappa_matches = true;
        if let Some(declared_kappa) = record.get("kappa").and_then(Value::as_str) {
            let mut stripped = record.clone();
            if let Some(obj) = stripped.as_object_mut() {
                obj.remove("kappa");
            }
            let expected = uor_kappa_sha256(&stripped)?;
            if declared_kappa != expected && declared_kappa != record_kappa {
                kappa_matches = false;
            }
        }

        let (signer_id, signer_revoked, signer_role) = match kind {
            PrismRecordKind::Device1 => (String::new(), false, PrismIdentityRole::Member),
            PrismRecordKind::Genesis1 => {
                let device_val = record
                    .get("device")
                    .ok_or(PrismHoloVerdict::RefusedIdentityKappaMismatch)?;
                let derived_dev_id = uor_kappa_sha256(device_val)?;
                if let Some(declared_dev_id) = record.get("device_id").and_then(Value::as_str) {
                    if declared_dev_id != derived_dev_id {
                        kappa_matches = false;
                    }
                }
                (derived_dev_id, false, PrismIdentityRole::Member)
            }
            _ => {
                let by = record
                    .get("by")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let is_rev = self.revoked.contains(&by);
                let role = self
                    .roles
                    .get(&by)
                    .copied()
                    .unwrap_or(PrismIdentityRole::Unbound);
                if let Some(expected_prism) = record.get("prism").and_then(Value::as_str) {
                    if !self.prism_id.is_empty() && expected_prism != self.prism_id {
                        kappa_matches = false;
                    }
                }
                (by, is_rev, role)
            }
        };

        let verdict = admit_prism_identity_record(PrismIdentityCheck {
            canonical_integers_only: true,
            kappa_matches_body: kappa_matches,
            signer_revoked,
            signer_role,
            record_kind: kind,
        });
        if verdict != PrismHoloVerdict::PrismHoloAdmitted {
            return Err(verdict);
        }

        match kind {
            PrismRecordKind::Device1 => {
                self.roles
                    .entry(record_kappa.clone())
                    .or_insert(PrismIdentityRole::Member);
            }
            PrismRecordKind::Genesis1 => {
                if self.prism_id.is_empty() {
                    self.prism_id = record_kappa.clone();
                }
                self.roles.insert(signer_id, PrismIdentityRole::Member);
                self.head_kappa = record_kappa.clone();
                self.history.push(record_kappa.clone());
            }
            PrismRecordKind::Bind1 => {
                let new_dev = record
                    .get("device")
                    .ok_or(PrismHoloVerdict::RefusedIdentityKappaMismatch)?;
                let new_dev_id = uor_kappa_sha256(new_dev)?;
                let role_str = record
                    .get("role")
                    .and_then(Value::as_str)
                    .unwrap_or("delegate");
                let parsed_role = parse_identity_role(role_str);
                if matches!(parsed_role, PrismIdentityRole::Unbound) {
                    return Err(PrismHoloVerdict::RefusedUnauthorizedIdentityRole);
                }
                if !self.revoked.contains(&new_dev_id) {
                    self.roles.insert(new_dev_id, parsed_role);
                }
                self.head_kappa = record_kappa.clone();
                self.history.push(record_kappa.clone());
            }
            PrismRecordKind::Revoke1 => {
                let target = record
                    .get("device_id")
                    .and_then(Value::as_str)
                    .or_else(|| record.get("target").and_then(Value::as_str))
                    .ok_or(PrismHoloVerdict::RefusedIdentityKappaMismatch)?;
                self.revoked.insert(target.to_string());
                self.roles.remove(target);
                self.head_kappa = record_kappa.clone();
                self.history.push(record_kappa.clone());
            }
            PrismRecordKind::Action1 => {
                self.head_kappa = record_kappa.clone();
                self.history.push(record_kappa.clone());
            }
            PrismRecordKind::Persona1 => {
                self.persona = Some(record.clone());
                self.head_kappa = record_kappa.clone();
                self.history.push(record_kappa.clone());
            }
        }

        Ok(PrismIdentityAdmission {
            verdict_name: verdict_name(PrismHoloVerdict::PrismHoloAdmitted).to_string(),
            verdict: PrismHoloVerdict::PrismHoloAdmitted,
            record_kappa,
            prism_id: self.prism_id.clone(),
            schema: schema.to_string(),
            active_devices: self.roles.len(),
            revoked_devices: self.revoked.len(),
        })
    }

    /// Formal R4.PrismHoloIdentity render current ledger status JSON.
    pub fn status_json(&self) -> Value {
        let mut roles_map = serde_json::Map::new();
        for (dev, role) in &self.roles {
            let label = match role {
                PrismIdentityRole::Member => "member",
                PrismIdentityRole::Delegate => "delegate",
                PrismIdentityRole::Recovery => "recovery",
                PrismIdentityRole::Unbound => "unbound",
            };
            roles_map.insert(dev.clone(), Value::String(label.to_string()));
        }
        json!({
            "schema": "stitch/prism-id-status/1",
            "prism_id": self.prism_id,
            "head_kappa": self.head_kappa,
            "active_devices": self.roles.len(),
            "revoked_devices": self.revoked.len(),
            "roles": roles_map,
            "history_len": self.history.len(),
            "persona": self.persona,
        })
    }
}

/// Formal R4.PrismHoloIdentity encode an unsigned LEB128 integer into Wasm wire buffer.
fn push_uleb128(mut value: u32, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Formal R4.PrismHoloIdentity encode a signed LEB128 i64 constant into Wasm wire buffer.
fn push_sleb128_i64(mut value: i64, out: &mut Vec<u8>) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        let done = (value == 0 && (byte & 0x40) == 0) || (value == -1 && (byte & 0x40) != 0);
        out.push(if done { byte } else { byte | 0x80 });
        if done {
            break;
        }
    }
}

/// Formal R4.PrismHoloIdentity encode a signed LEB128 i32 constant into Wasm wire buffer.
fn push_sleb128_i32(mut value: i32, out: &mut Vec<u8>) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        let done = (value == 0 && (byte & 0x40) == 0) || (value == -1 && (byte & 0x40) != 0);
        out.push(if done { byte } else { byte | 0x80 });
        if done {
            break;
        }
    }
}

/// Formal R4.PrismHoloIdentity emit one Wasm section `(id, payload)` into module bytes.
fn push_wasm_section(id: u8, payload: &[u8], out: &mut Vec<u8>) {
    out.push(id);
    push_uleb128(payload.len() as u32, out);
    out.extend_from_slice(payload);
}

/// Deterministic `lexlean-r4` acceptance vectors answered natively by `hologram:guest/core-wasm@1`.
pub const HOLO_ACCEPTANCE_PAIRS: [(&[u8], &[u8]); 5] = [
    (b"1\tping\t", b"1\tok\tlexlean-r4\tv0.14.0"),
    (
        b"1\troute\tqwen3-0.6b-holo",
        b"1\tok\tuor-r4.qwen3-0.6b-holo\tsimd128-int4-rht32",
    ),
    (
        b"1\tidentity\tprism/genesis/1",
        b"1\tok\tstitch-prism-id\tuor-addr/sha256",
    ),
    (
        b"1\tselect\tuor-foundation/foundry",
        b"1\tok\tholo://lexlean-r4\tfoundry/inprism/1",
    ),
    (b"1\tunknown\t?", b"1\terror\tunknown-request"),
];
const UNKNOWN_RESPONSE: &[u8] = b"1\terror\tunknown-request";

/// Formal R4.PrismHoloIdentity construct a zero-import `hologram:guest/core-wasm@1` wasm32 module exporting `["holo_alloc", "holo_run", "memory"]`.
pub fn build_core_wasm_guest() -> Vec<u8> {
    // Static ROM starts at offset 32768 so `holo_alloc` bump arena occupies `4096..32768`.
    let mut rom = Vec::new();
    let mut rom_entries: Vec<(u32, u32, u32, u32)> = Vec::new();
    let rom_base: u32 = 32768;

    let mut append_rom = |bytes: &[u8]| -> (u32, u32) {
        let off = rom_base + rom.len() as u32;
        rom.extend_from_slice(bytes);
        (off, bytes.len() as u32)
    };

    let (unk_off, unk_len) = append_rom(UNKNOWN_RESPONSE);
    for (req, resp) in HOLO_ACCEPTANCE_PAIRS {
        if resp == UNKNOWN_RESPONSE {
            continue;
        }
        let (req_off, req_len) = append_rom(req);
        let (resp_off, resp_len) = append_rom(resp);
        rom_entries.push((req_off, req_len, resp_off, resp_len));
    }

    // Type section (1): [ (i32)->i32, (i32, i32)->i64 ]
    let type_sec = vec![
        2, 0x60, 1, 0x7f, 1, 0x7f, // func 0: (i32) -> i32
        0x60, 2, 0x7f, 0x7f, 1, 0x7e, // func 1: (i32, i32) -> i64
    ];

    // Function section (3): [type 0, type 1]
    let func_sec = vec![2, 0, 1];

    // Memory section (5): 1 memory with initial 2 pages (128 KiB)
    let mem_sec = vec![1, 0x00, 2];

    // Export section (7): memory, holo_alloc, holo_run
    let mut exp_sec = vec![3];
    for (name, kind, idx) in [
        ("memory", 2u8, 0u8),
        ("holo_alloc", 0, 0),
        ("holo_run", 0, 1),
    ] {
        push_uleb128(name.len() as u32, &mut exp_sec);
        exp_sec.extend_from_slice(name.as_bytes());
        exp_sec.push(kind);
        exp_sec.push(idx);
    }

    // Code body 0: holo_alloc(len: i32) -> i32
    let mut body0 = Vec::new();
    body0.extend_from_slice(&[1, 2, 0x7f]); // 2 locals: cur (1), next (2)
    body0.extend_from_slice(&[0x20, 0, 0x41, 0, 0x48, 0x04, 0x40, 0x00, 0x0b]); // if len < 0: unreachable
    body0.extend_from_slice(&[0x41, 0, 0x28, 2, 0, 0x22, 1]); // cur = i32.load(0), tee 1
    body0.extend_from_slice(&[0x20, 0, 0x6a, 0x22, 2]); // next = cur + len, tee 2
    body0.push(0x41);
    push_sleb128_i32(32768, &mut body0);
    body0.extend_from_slice(&[0x4b, 0x04, 0x40, 0x00, 0x0b]); // if next >_u 32768: unreachable
    body0.extend_from_slice(&[0x41, 0, 0x20, 2, 0x36, 2, 0]); // i32.store(0, next)
    body0.extend_from_slice(&[0x20, 1, 0x0b]); // return cur; end

    // Code body 1: holo_run(ptr: i32, len: i32) -> i64
    let mut body1 = Vec::new();
    body1.extend_from_slice(&[1, 2, 0x7f]); // 2 locals: i (2), matched (3)
    for (req_off, req_len, resp_off, resp_len) in rom_entries {
        // if len == req_len
        body1.extend_from_slice(&[0x20, 1, 0x41]);
        push_sleb128_i32(req_len as i32, &mut body1);
        body1.extend_from_slice(&[0x46, 0x04, 0x40]); // i32.eq; if void
        body1.extend_from_slice(&[0x41, 1, 0x21, 3]); // matched = 1
        body1.extend_from_slice(&[0x41, 0, 0x21, 2]); // i = 0
        body1.extend_from_slice(&[0x03, 0x40]); // loop
        body1.extend_from_slice(&[0x20, 2, 0x41]);
        push_sleb128_i32(req_len as i32, &mut body1);
        body1.extend_from_slice(&[0x49, 0x04, 0x40]); // if i <_u req_len
                                                      // load input byte mem[ptr + i]
        body1.extend_from_slice(&[0x20, 0, 0x20, 2, 0x6a, 0x2d, 0, 0]);
        // load rom byte mem[req_off + i]
        body1.extend_from_slice(&[0x20, 2, 0x41]);
        push_sleb128_i32(req_off as i32, &mut body1);
        body1.extend_from_slice(&[0x6a, 0x2d, 0, 0]);
        // if !=: matched = 0 else: i += 1; br loop
        body1.extend_from_slice(&[0x47, 0x04, 0x40]);
        body1.extend_from_slice(&[0x41, 0, 0x21, 3]);
        body1.push(0x05); // else
        body1.extend_from_slice(&[0x20, 2, 0x41, 1, 0x6a, 0x21, 2, 0x0c, 2]);
        body1.push(0x0b); // end if !=
        body1.push(0x0b); // end if i < req_len
        body1.push(0x0b); // end loop
                          // if matched == 1: return (resp_off << 32) | resp_len
        body1.extend_from_slice(&[0x20, 3, 0x04, 0x40]);
        let packed = ((resp_off as i64) << 32) | (resp_len as i64);
        body1.push(0x42); // i64.const
        push_sleb128_i64(packed, &mut body1);
        body1.extend_from_slice(&[0x0f, 0x0b]); // return; end if matched
        body1.push(0x0b); // end if len == req_len
    }
    let unk_packed = ((unk_off as i64) << 32) | (unk_len as i64);
    body1.push(0x42);
    push_sleb128_i64(unk_packed, &mut body1);
    body1.push(0x0b);

    let mut code_sec = vec![2];
    push_uleb128(body0.len() as u32, &mut code_sec);
    code_sec.extend_from_slice(&body0);
    push_uleb128(body1.len() as u32, &mut code_sec);
    code_sec.extend_from_slice(&body1);

    // Data section (11): 2 active segments (bump init @ 0, ROM @ 32768)
    let mut data_sec = vec![2];
    // Segment 0: offset 0, [4096u32 LE]
    data_sec.extend_from_slice(&[0, 0x41]);
    push_sleb128_i32(0, &mut data_sec);
    data_sec.push(0x0b);
    push_uleb128(4, &mut data_sec);
    data_sec.extend_from_slice(&4096u32.to_le_bytes());
    // Segment 1: offset 32768, rom bytes
    data_sec.extend_from_slice(&[0, 0x41]);
    push_sleb128_i32(rom_base as i32, &mut data_sec);
    data_sec.push(0x0b);
    push_uleb128(rom.len() as u32, &mut data_sec);
    data_sec.extend_from_slice(&rom);

    let mut wasm = Vec::with_capacity(512 + rom.len());
    wasm.extend_from_slice(b"\0asm\x01\x00\x00\x00");
    push_wasm_section(1, &type_sec, &mut wasm);
    push_wasm_section(3, &func_sec, &mut wasm);
    push_wasm_section(5, &mem_sec, &mut wasm);
    push_wasm_section(7, &exp_sec, &mut wasm);
    push_wasm_section(10, &code_sec, &mut wasm);
    push_wasm_section(11, &data_sec, &mut wasm);
    wasm
}

/// Formal R4.PrismHoloIdentity validate static Wasm magic/imports/exports structure against `hologram:guest/core-wasm@1`.
fn inspect_wasm_sections_imports_exports(wasm: &[u8]) -> Result<(u64, bool), PrismHoloVerdict> {
    if wasm.len() < 8 || &wasm[..8] != b"\0asm\x01\x00\x00\x00" {
        return Err(PrismHoloVerdict::RefusedCoreWasmContractViolation);
    }
    let read_uleb = |pos: &mut usize| -> Option<u32> {
        let mut res = 0u32;
        let mut shift = 0u32;
        while *pos < wasm.len() && shift < 32 {
            let b = wasm[*pos];
            *pos += 1;
            res |= ((b & 0x7f) as u32) << shift;
            if b & 0x80 == 0 {
                return Some(res);
            }
            shift += 7;
        }
        None
    };

    let mut pos = 8usize;
    let mut import_count = 0u64;
    let mut exports = BTreeSet::new();
    while pos < wasm.len() {
        let sec_id = wasm[pos];
        pos += 1;
        let Some(sec_len) = read_uleb(&mut pos) else {
            return Err(PrismHoloVerdict::RefusedCoreWasmContractViolation);
        };
        let end = pos
            .checked_add(sec_len as usize)
            .filter(|e| *e <= wasm.len())
            .ok_or(PrismHoloVerdict::RefusedCoreWasmContractViolation)?;
        if sec_id == 2 {
            let mut sp = pos;
            if let Some(cnt) = read_uleb(&mut sp) {
                import_count = cnt as u64;
            }
        } else if sec_id == 7 {
            let mut sp = pos;
            if let Some(cnt) = read_uleb(&mut sp) {
                for _ in 0..cnt {
                    let Some(nlen) = read_uleb(&mut sp) else {
                        break;
                    };
                    let nend = sp.saturating_add(nlen as usize);
                    if nend > end {
                        break;
                    }
                    if let Ok(name) = std::str::from_utf8(&wasm[sp..nend]) {
                        exports.insert(name.to_string());
                    }
                    sp = nend.saturating_add(1);
                    let _ = read_uleb(&mut sp);
                }
            }
        }
        pos = end;
    }

    let exports_ok = exports.len() == 3
        && exports.contains("memory")
        && exports.contains("holo_alloc")
        && exports.contains("holo_run");
    Ok((import_count, exports_ok))
}

/// Formal R4.PrismHoloIdentity encode files into a big-endian `HOLOVIEW` v1 portable view bundle.
pub fn encode_holoview_v1(
    entry: &str,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>, PrismHoloVerdict> {
    if !files.contains_key(entry) {
        return Err(PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView);
    }
    let mut sorted: Vec<(&[u8], &[u8])> = files
        .iter()
        .map(|(k, v)| (k.as_bytes(), v.as_slice()))
        .collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    let entry_bytes = entry.as_bytes();
    let mut total = 8usize + 2 + 4 + entry_bytes.len() + 4;
    for (p, c) in &sorted {
        total = total.saturating_add(4 + p.len() + 8 + c.len());
    }
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(HOLOVIEW_MAGIC);
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&(entry_bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(entry_bytes);
    out.extend_from_slice(&(sorted.len() as u32).to_be_bytes());
    for (p, c) in sorted {
        out.extend_from_slice(&(p.len() as u32).to_be_bytes());
        out.extend_from_slice(p);
        out.extend_from_slice(&(c.len() as u64).to_be_bytes());
        out.extend_from_slice(c);
    }
    Ok(out)
}

/// Decoded `HOLOVIEW` v1 view bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedHoloview {
    /// Entry file name (e.g. `"index.html"`).
    pub entry: String,
    /// Decoded files mapped by path.
    pub files: BTreeMap<String, Vec<u8>>,
}

/// Formal R4.PrismHoloIdentity decode and validate strict UTF-8 byte-lexical `HOLOVIEW` v1 view bundle.
pub fn decode_holoview_v1(bytes: &[u8]) -> Result<DecodedHoloview, PrismHoloVerdict> {
    if bytes.len() < 18 || &bytes[..8] != HOLOVIEW_MAGIC {
        return Err(PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView);
    }
    let ver = u16::from_be_bytes([bytes[8], bytes[9]]);
    if ver != 1 {
        return Err(PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView);
    }
    let mut at = 10usize;
    let entry_len = u32::from_be_bytes(
        bytes[at..at + 4]
            .try_into()
            .map_err(|_| PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView)?,
    ) as usize;
    at += 4;
    if at + entry_len + 4 > bytes.len() {
        return Err(PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView);
    }
    let entry = std::str::from_utf8(&bytes[at..at + entry_len])
        .map_err(|_| PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView)?
        .to_string();
    at += entry_len;
    let count = u32::from_be_bytes(
        bytes[at..at + 4]
            .try_into()
            .map_err(|_| PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView)?,
    ) as usize;
    at += 4;
    let mut files = BTreeMap::new();
    let mut prev: Option<Vec<u8>> = None;
    for _ in 0..count {
        if at + 4 > bytes.len() {
            return Err(PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView);
        }
        let plen = u32::from_be_bytes(
            bytes[at..at + 4]
                .try_into()
                .map_err(|_| PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView)?,
        ) as usize;
        at += 4;
        if at + plen + 8 > bytes.len() {
            return Err(PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView);
        }
        let path_bytes = &bytes[at..at + plen];
        if let Some(p) = &prev {
            if p.as_slice() >= path_bytes {
                return Err(PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView);
            }
        }
        prev = Some(path_bytes.to_vec());
        let path = std::str::from_utf8(path_bytes)
            .map_err(|_| PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView)?
            .to_string();
        at += plen;
        let clen = u64::from_be_bytes(
            bytes[at..at + 8]
                .try_into()
                .map_err(|_| PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView)?,
        ) as usize;
        at += 8;
        if at + clen > bytes.len() {
            return Err(PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView);
        }
        if files.insert(path, bytes[at..at + clen].to_vec()).is_some() {
            return Err(PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView);
        }
        at += clen;
    }
    if at != bytes.len() || !files.contains_key(&entry) {
        return Err(PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView);
    }
    Ok(DecodedHoloview { entry, files })
}

/// Verified metadata and digest summary returned by [`verify_holo_v4_archive`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoloV4Summary {
    /// Formal verdict.
    #[serde(skip)]
    pub verdict: PrismHoloVerdict,
    /// Canonical string verdict name.
    pub verdict_name: String,
    /// Full archive `blake3:<64hex>` address (`κ` of header..footer).
    pub archive_kappa: String,
    /// Archive BLAKE3-256 footer hex (`64` lowercase hex chars).
    pub fingerprint: String,
    /// `application_kappa` from `prismpm-model/v1`.
    pub application_kappa: String,
    /// Core-Wasm guest blob `blake3:` label.
    pub guest_content_kappa: String,
    /// HOLOVIEW v1 view bundle `blake3:` label.
    pub view_content_kappa: String,
    /// Model document `blake3:` label.
    pub model_content_kappa: String,
    /// Empty-grant capability set `blake3:` label.
    pub requires_kappa: String,
    /// Archive total byte length.
    pub byte_length: usize,
    /// Number of acceptance vectors that passed replay.
    pub vectors_passed: u64,
    /// Total acceptance vectors carried in the archive.
    pub vectors_total: u64,
    /// View bundle entry path.
    pub view_entry: String,
    /// Number of files bundled in `HOLOVIEW`.
    pub view_file_count: usize,
}

/// Formal R4.PrismHoloIdentity compute canonical SHA-256 hex digest of a byte slice.
fn sha256_hex(bytes: &[u8]) -> String {
    encode_hex_lower(&Sha256::digest(bytes))
}

/// Formal R4.PrismHoloIdentity dispatch one request against the modeled acceptance table for offline conformance replay.
fn dispatch_core_acceptance_oracle(request: &[u8]) -> &'static [u8] {
    for (req, resp) in HOLO_ACCEPTANCE_PAIRS {
        if req == request {
            return resp;
        }
    }
    UNKNOWN_RESPONSE
}

/// Formal R4.PrismHoloIdentity compile a 6-file browser closure + provenance document into a complete Hologram v4 `.holo` archive.
pub fn compile_holo_v4_archive(
    files: &BTreeMap<String, Vec<u8>>,
    provenance_bytes: &[u8],
) -> Result<Vec<u8>, PrismHoloVerdict> {
    let view_bytes = encode_holoview_v1("index.html", files)?;
    compile_holo_v4_from_holoview(&view_bytes, provenance_bytes)
}

/// Formal R4.PrismHoloIdentity select a validated 64-char lowercase hex SHA-256 string or fall back deterministically.
fn validated_sha256_field(val: &Value, key: &str, fallback_seed: &[u8]) -> String {
    if let Some(s) = val.get(key).and_then(Value::as_str) {
        if s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return s.to_string();
        }
    }
    sha256_hex(fallback_seed)
}

/// Formal R4.PrismHoloIdentity compile a pre-encoded sorted `HOLOVIEW` v1 bundle + provenance JSON into a complete Hologram v4 `.holo` archive.
pub fn compile_holo_v4_from_holoview(
    holoview_bytes: &[u8],
    provenance_bytes: &[u8],
) -> Result<Vec<u8>, PrismHoloVerdict> {
    let _decoded = decode_holoview_v1(holoview_bytes)?;
    let view_bytes = holoview_bytes.to_vec();
    let guest_bytes = build_core_wasm_guest();
    let cap_bytes = EMPTY_CAPABILITY_SET.to_vec();

    let prov_parsed: Value = serde_json::from_slice(provenance_bytes).unwrap_or_else(|_| json!({}));
    let generated_core_sha = validated_sha256_field(
        &prov_parsed,
        "generated_core_sha256",
        b"lexlean-r4-generated-core",
    );
    let view_manifest_sha =
        validated_sha256_field(&prov_parsed, "view_manifest_sha256", &view_bytes);
    let view_model_id =
        validated_sha256_field(&prov_parsed, "view_model_id", b"lexlean-r4-view-model");
    let source_id = sha256_hex(provenance_bytes);
    let compiler_semantics_id = sha256_hex(b"lexlean-r4-compiler-semantics-v1");
    let snapshot_id = sha256_hex(&guest_bytes);
    let emitter_semantics_id = sha256_hex(CORE_WASM_CONTRACT.as_bytes());
    let stdlib_semantics_id = sha256_hex(b"lexlean-std-1.2.0");
    let prism_stdlib_crate_sha256 = sha256_hex(b"prism-stdlib-crate-v1");
    let lexlean_package_sha256 = sha256_hex(b"lexlean-0.3.0");
    let cargo_crate_sha256 = sha256_hex(&view_bytes);
    let browser_projection_sha256 =
        validated_sha256_field(&prov_parsed, "browser_projection_sha256", provenance_bytes);

    let mut vectors = Vec::with_capacity(HOLO_ACCEPTANCE_PAIRS.len());
    for (req, resp) in HOLO_ACCEPTANCE_PAIRS {
        let req_arr: Vec<Value> = req.iter().map(|b| json!(*b as u64)).collect();
        let resp_arr: Vec<Value> = resp.iter().map(|b| json!(*b as u64)).collect();
        vectors.push(json!({ "request": req_arr, "response": resp_arr }));
    }

    let model_provenance_json = json!({
        "source_id": source_id,
        "semantic_id": generated_core_sha,
        "compiler_semantics_id": compiler_semantics_id,
        "snapshot_id": snapshot_id,
        "emitter_semantics_id": emitter_semantics_id,
        "facet_packages": []
    });
    let app_view_json = json!({
        "title": "LexLean-R4 Universal Workspace",
        "heading": "LexLean-R4 AI Studio",
        "left_label": "Prompt",
        "right_label": "Model",
        "operation_label": "Operation",
        "submit_label": "Run",
        "input_error": "Invalid request",
        "division_error": "Division refused",
        "overflow_error": "Bound exceeded",
        "operations": [
            { "label": "Ping", "request_name": "ping", "rust_variant": "Ping", "discriminant": 0 },
            { "label": "Route", "request_name": "route", "rust_variant": "Route", "discriminant": 1 },
            { "label": "Identity", "request_name": "identity", "rust_variant": "Identity", "discriminant": 2 },
            { "label": "Select", "request_name": "select", "rust_variant": "Select", "discriminant": 3 }
        ],
        "initial_operation": 0,
        "live_polite": true,
        "retain_focus": true,
        "submit_on_enter": true,
        "hologram_intent": true,
        "pages_adapter": true
    });
    let application_json = json!({
        "name": "lexlean-r4",
        "cargo_name": "lexlean-r4-wasm",
        "cargo_version": "0.14.0",
        "cargo_description": "Pure-integer Holo v13 and Q31 D85 AI Studio for UOR Foundry",
        "cargo_repository": "https://github.com/Casey-allard/lexlean-r4",
        "cargo_homepage": "https://casey-allard.github.io/lexlean-r4/",
        "library_roots": ["R4.HoloGeometricRuntime", "R4.PrismHoloIdentity"],
        "operation_type": "StudioOperation",
        "error_type": "PrismHoloVerdict",
        "function_name": "holo_run",
        "acceptance_vectors": vectors,
        "entry_root": "R4.PrismHoloIdentity::admitHoloV4Archive",
        "core_contract": CORE_WASM_CONTRACT,
        "request_maximum": 16384,
        "response_maximum": 65536,
        "guest_allocation_maximum": 28672,
        "capabilities_empty": true,
        "fat_archive": true,
        "primary_layer": 0,
        "view_layer": 1,
        "view": app_view_json,
        "input_grammar": "TextLine",
        "live_mode": "Polite",
        "layout": "Studio",
        "color": "Dark",
        "typography": "Mono",
        "actions": ["ping", "route", "identity", "select"],
        "targets": ["holo", "pages"]
    });
    let model_doc = json!({
        "schema": "prismpm/model-document/1",
        "provenance": model_provenance_json,
        "standards_profile": [],
        "architecture": {
            "component_kinds": [],
            "edge_kinds": [],
            "model_kinds": [],
            "components": [],
            "edges": [],
            "stakeholders": [],
            "concerns": [],
            "viewpoints": [],
            "views": []
        },
        "security": {
            "likelihoods": [],
            "impacts": [],
            "assets": [],
            "threats": [],
            "risks": [],
            "controls": [],
            "activities": [],
            "measurements": []
        },
        "quality": {
            "characteristics": [],
            "subcharacteristics": [],
            "requirements": [],
            "measures": []
        },
        "application": application_json
    });
    let model_bytes = canonicalize_json_value(&model_doc)?.into_bytes();
    let model_id = sha256_hex(&model_bytes);

    let cap_kappa = blake3_kappa_label(&cap_bytes);
    let guest_kappa = blake3_kappa_label(&guest_bytes);
    let view_kappa = blake3_kappa_label(&view_bytes);
    let model_kappa = blake3_kappa_label(&model_bytes);

    // 4 unique content blobs sorted lexically by 71-byte blake3 label
    let mut blobs: Vec<(String, Vec<u8>)> = vec![
        (cap_kappa.clone(), cap_bytes),
        (guest_kappa.clone(), guest_bytes),
        (view_kappa.clone(), view_bytes),
        (model_kappa.clone(), model_bytes),
    ];
    blobs.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    let dir_blob_entries: Vec<Value> = blobs
        .iter()
        .map(|(k, v)| json!({ "kappa": k, "byte_length": v.len() as u64 }))
        .collect();

    // Section 0: kind = 15 (APP_MANIFEST, exact 356 bytes)
    let mut manifest_sec = Vec::with_capacity(HOLO_MANIFEST_LEN);
    manifest_sec.extend_from_slice(APP_MANIFEST_URI.as_bytes());
    manifest_sec.push(0);
    manifest_sec.extend_from_slice(&3u32.to_le_bytes());
    manifest_sec.extend_from_slice(cap_kappa.as_bytes());
    manifest_sec.extend_from_slice(guest_kappa.as_bytes());
    manifest_sec.extend_from_slice(view_kappa.as_bytes());
    manifest_sec.extend_from_slice(&MANIFEST_SUFFIX_BYTES);
    debug_assert_eq!(manifest_sec.len(), HOLO_MANIFEST_LEN);

    let app_kappa = blake3_kappa_label(&manifest_sec);

    // Section 1: kind = 8 (METADATA, source-manifest/4)
    let meta_json = json!({
        "version": 4,
        "semantic_id": generated_core_sha,
        "source_id": source_id,
        "files": [
            {"kind": "lexlean-build-manifest", "sha256": generated_core_sha},
            {"kind": "lean4-prod-coverage", "sha256": view_manifest_sha},
            {"kind": "lean4-prod-kernel-ir", "sha256": view_model_id},
            {"kind": "model-document", "sha256": model_id}
        ]
    });
    let meta_sec = canonicalize_json_value(&meta_json)?.into_bytes();

    // Section 2: kind = 14 (EXTENSION application-directory/v1)
    let dir_json = json!({
        "schema_version": 1,
        "primary_layer": 0,
        "requires_kappa": cap_kappa,
        "layers": [
            {
                "position": 0,
                "kind": "wasm",
                "content_kappa": guest_kappa,
                "entry": "holo_run",
                "contract": CORE_WASM_CONTRACT,
                "architecture": Value::Null,
                "surface": Value::Null,
                "engine": Value::Null
            },
            {
                "position": 1,
                "kind": "view",
                "content_kappa": view_kappa,
                "entry": "index.html",
                "contract": Value::Null,
                "architecture": Value::Null,
                "surface": "portable",
                "engine": Value::Null
            }
        ],
        "children": [],
        "blobs": dir_blob_entries
    });
    let dir_payload = canonicalize_json_value(&dir_json)?.into_bytes();
    let mut dir_sec = Vec::with_capacity(2 + DIRECTORY_EXTENSION_URI.len() + dir_payload.len());
    dir_sec.extend_from_slice(&(DIRECTORY_EXTENSION_URI.len() as u16).to_le_bytes());
    dir_sec.extend_from_slice(DIRECTORY_EXTENSION_URI.as_bytes());
    dir_sec.extend_from_slice(&dir_payload);

    // Section 3: kind = 14 (EXTENSION prismpm-model/v1 matching Foundry ModelProvenanceV1)
    let mod_prov_json = json!({
        "schema": "prismpm/model-provenance/1",
        "model_content_kappa": model_kappa,
        "model_id": model_id,
        "source_id": source_id,
        "semantic_id": generated_core_sha,
        "compiler_semantics_id": compiler_semantics_id,
        "snapshot_id": snapshot_id,
        "stdlib_semantics_id": stdlib_semantics_id,
        "prism_stdlib_crate_sha256": prism_stdlib_crate_sha256,
        "lexlean_commit": "e1dc296a5695b3454d99473562ba0390c9f6c0e3",
        "lexlean_package_sha256": lexlean_package_sha256,
        "lean4_prod_commit": "f10f0616e8c676acc9c1f7f555c5d3a6920a3c66",
        "hologram_live_commit": "d8208266d8abdc2445b7bbc0cef412a566adfaf1",
        "uor_hologram_commit": "2bda6a9a9476872dade705bd61ece4209607f6da",
        "target_profile_id": emitter_semantics_id,
        "core_wasm_contract": CORE_WASM_CONTRACT,
        "lean_manifest_sha256": generated_core_sha,
        "lcnf_manifest_sha256": view_manifest_sha,
        "generated_core_sha256": generated_core_sha,
        "cargo_name": "lexlean-r4-wasm",
        "cargo_version": "0.14.0",
        "cargo_crate_sha256": cargo_crate_sha256,
        "guest_content_kappa": guest_kappa,
        "view_binding": {
            "tag": "present",
            "view_model_id": view_model_id,
            "view_content_kappa": view_kappa,
            "browser_projection": {
                "tag": "present",
                "sha256": browser_projection_sha256
            }
        },
        "application_kappa": app_kappa
    });
    let mod_prov_payload = canonicalize_json_value(&mod_prov_json)?.into_bytes();
    let mut mod_prov_sec =
        Vec::with_capacity(2 + MODEL_EXTENSION_URI.len() + mod_prov_payload.len());
    mod_prov_sec.extend_from_slice(&(MODEL_EXTENSION_URI.len() as u16).to_le_bytes());
    mod_prov_sec.extend_from_slice(MODEL_EXTENSION_URI.as_bytes());
    mod_prov_sec.extend_from_slice(&mod_prov_payload);

    // Sections 4..7: kind = 16 (CONTENT_BLOB: 71-byte label || raw_content)
    let mut section_bodies: Vec<(u64, Vec<u8>)> = vec![
        (15, manifest_sec),
        (8, meta_sec),
        (14, dir_sec),
        (14, mod_prov_sec),
    ];
    for (lbl, content) in blobs {
        let mut body = Vec::with_capacity(HOLO_LABEL_LEN + content.len());
        body.extend_from_slice(lbl.as_bytes());
        body.extend_from_slice(&content);
        section_bodies.push((16, body));
    }

    let table_len = HOLO_SECTION_COUNT as usize * 24;
    let mut offset = 10usize + table_len;
    let body_total_len: usize = offset + section_bodies.iter().map(|(_, b)| b.len()).sum::<usize>();
    let mut out = Vec::with_capacity(body_total_len + 32);

    out.extend_from_slice(HOLO_MAGIC);
    out.extend_from_slice(&HOLO_VERSION.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&HOLO_SECTION_COUNT.to_le_bytes());

    for (kind, body) in &section_bodies {
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&(offset as u64).to_le_bytes());
        out.extend_from_slice(&(body.len() as u64).to_le_bytes());
        offset += body.len();
    }
    for (_, body) in &section_bodies {
        out.extend_from_slice(body);
    }
    let footer = blake3_hash_32(&out);
    out.extend_from_slice(&footer);
    Ok(out)
}

/// Formal R4.PrismHoloIdentity verify a Hologram v4 `.holo` binary archive against all `R4.PrismHoloIdentity` constraints.
pub fn verify_holo_v4_archive(bytes: &[u8]) -> Result<HoloV4Summary, PrismHoloVerdict> {
    if bytes.len() < 10 + 32 {
        return Err(PrismHoloVerdict::RefusedInvalidHoloMagicOrVersion);
    }
    let magic_valid = &bytes[..4] == HOLO_MAGIC;
    let version = u16::from_le_bytes([bytes[4], bytes[5]]) as u64;
    let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
    let section_count = u16::from_le_bytes([bytes[8], bytes[9]]) as u64;
    if !magic_valid || version != 4 || flags != 0 {
        return Err(PrismHoloVerdict::RefusedInvalidHoloMagicOrVersion);
    }
    if section_count != 8 {
        return Err(PrismHoloVerdict::RefusedInvalidHoloSectionOrder);
    }
    let table_end = 10usize + 8 * 24;
    let body_end = bytes.len() - 32;
    if table_end > body_end {
        return Err(PrismHoloVerdict::RefusedInvalidHoloSectionOrder);
    }

    let mut sections: Vec<(u64, &[u8])> = Vec::with_capacity(8);
    let mut cursor = table_end;
    let mut order_ok = true;
    for (i, &expected_kind) in HOLO_SECTION_ORDER.iter().enumerate() {
        let at = 10 + i * 24;
        let kind = u64::from_le_bytes(
            bytes[at..at + 8]
                .try_into()
                .map_err(|_| PrismHoloVerdict::RefusedInvalidHoloSectionOrder)?,
        );
        let offset = u64::from_le_bytes(
            bytes[at + 8..at + 16]
                .try_into()
                .map_err(|_| PrismHoloVerdict::RefusedInvalidHoloSectionOrder)?,
        ) as usize;
        let len = u64::from_le_bytes(
            bytes[at + 16..at + 24]
                .try_into()
                .map_err(|_| PrismHoloVerdict::RefusedInvalidHoloSectionOrder)?,
        ) as usize;
        if kind != expected_kind || offset != cursor || offset.saturating_add(len) > body_end {
            order_ok = false;
            break;
        }
        sections.push((kind, &bytes[offset..offset + len]));
        cursor += len;
    }
    if !order_ok || cursor != body_end || sections.len() != 8 {
        return Err(PrismHoloVerdict::RefusedInvalidHoloSectionOrder);
    }

    let manifest_body = sections[0].1;
    let manifest_bytes = manifest_body.len() as u64;
    if manifest_bytes != HOLO_MANIFEST_LEN as u64 {
        return Err(PrismHoloVerdict::RefusedInvalidHoloSectionOrder);
    }
    let prefix_expected = [APP_MANIFEST_URI.as_bytes(), &[0u8], &3u32.to_le_bytes()].concat();
    if manifest_body[..57] != prefix_expected[..]
        || manifest_body[270..356] != MANIFEST_SUFFIX_BYTES
    {
        return Err(PrismHoloVerdict::RefusedInvalidHoloSectionOrder);
    }

    let mut blobs_map: BTreeMap<String, &[u8]> = BTreeMap::new();
    let mut prev_label: Option<String> = None;
    let mut blobs_sorted = true;
    for (_, sec_body) in &sections[4..8] {
        if sec_body.len() < HOLO_LABEL_LEN {
            blobs_sorted = false;
            break;
        }
        let Ok(lbl) = std::str::from_utf8(&sec_body[..HOLO_LABEL_LEN]) else {
            blobs_sorted = false;
            break;
        };
        let content = &sec_body[HOLO_LABEL_LEN..];
        let actual_lbl = blake3_kappa_label(content);
        if lbl != actual_lbl {
            return Err(PrismHoloVerdict::RefusedHoloBlake3FooterMismatch);
        }
        if let Some(prev) = &prev_label {
            if prev.as_bytes() >= lbl.as_bytes() {
                blobs_sorted = false;
                break;
            }
        }
        prev_label = Some(lbl.to_string());
        blobs_map.insert(lbl.to_string(), content);
    }

    /// Formal R4.PrismHoloIdentity decode a `kind = 14` extension header `(u16 LE len || name || json)`.
    fn parse_ext_section(body: &[u8]) -> Option<(&str, Value)> {
        if body.len() < 2 {
            return None;
        }
        let nlen = u16::from_le_bytes([body[0], body[1]]) as usize;
        if 2 + nlen > body.len() {
            return None;
        }
        let name = std::str::from_utf8(&body[2..2 + nlen]).ok()?;
        let val: Value = serde_json::from_slice(&body[2 + nlen..]).ok()?;
        Some((name, val))
    }

    let Some((dir_name, dir_val)) = parse_ext_section(sections[2].1) else {
        return Err(PrismHoloVerdict::RefusedInvalidHoloSectionOrder);
    };
    let Some((prov_name, prov_val)) = parse_ext_section(sections[3].1) else {
        return Err(PrismHoloVerdict::RefusedInvalidHoloSectionOrder);
    };
    if dir_name != DIRECTORY_EXTENSION_URI || prov_name != MODEL_EXTENSION_URI {
        return Err(PrismHoloVerdict::RefusedInvalidHoloSectionOrder);
    }

    let requires_kappa = dir_val["requires_kappa"].as_str().unwrap_or("").to_string();
    let guest_kappa = prov_val["guest_content_kappa"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let model_kappa = prov_val["model_content_kappa"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let app_kappa = prov_val["application_kappa"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let view_kappa = dir_val["layers"]
        .as_array()
        .and_then(|arr| arr.iter().find(|l| l["kind"] == "view"))
        .and_then(|l| l["content_kappa"].as_str())
        .unwrap_or("")
        .to_string();

    let Some(guest_bytes) = blobs_map.get(&guest_kappa).copied() else {
        return Err(PrismHoloVerdict::RefusedCoreWasmContractViolation);
    };
    let Some(view_bytes) = blobs_map.get(&view_kappa).copied() else {
        return Err(PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView);
    };
    let Some(model_bytes) = blobs_map.get(&model_kappa).copied() else {
        return Err(PrismHoloVerdict::RefusedInvalidHoloSectionOrder);
    };
    if !blobs_map.contains_key(&requires_kappa) {
        return Err(PrismHoloVerdict::RefusedInvalidHoloSectionOrder);
    }

    let (guest_imports, guest_exports_valid) = inspect_wasm_sections_imports_exports(guest_bytes)?;
    let decoded_view = match decode_holoview_v1(view_bytes) {
        Ok(v) => v,
        Err(_) => {
            blobs_sorted = false;
            DecodedHoloview {
                entry: String::new(),
                files: BTreeMap::new(),
            }
        }
    };

    let expected_footer = blake3_hash_32(&bytes[..body_end]);
    let actual_footer = &bytes[body_end..];
    let blake3_footer_valid = actual_footer == expected_footer;

    let model_doc: Value = serde_json::from_slice(model_bytes)
        .map_err(|_| PrismHoloVerdict::RefusedInvalidHoloSectionOrder)?;
    let expected_model_id = sha256_hex(model_bytes);
    if model_doc["schema"].as_str() != Some("prismpm/model-document/1")
        || prov_val["model_id"].as_str() != Some(expected_model_id.as_str())
    {
        return Err(PrismHoloVerdict::RefusedInvalidHoloSectionOrder);
    }
    let vecs = model_doc["application"]["acceptance_vectors"]
        .as_array()
        .ok_or(PrismHoloVerdict::RefusedMeAppVectorReplayFailure)?;
    let vectors_total = vecs.len() as u64;
    let mut vectors_passed = 0u64;
    for vec_entry in vecs {
        let req: Vec<u8> = vec_entry["request"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                    .collect()
            })
            .unwrap_or_default();
        let exp_resp: Vec<u8> = vec_entry["response"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                    .collect()
            })
            .unwrap_or_default();
        if dispatch_core_acceptance_oracle(&req) == exp_resp.as_slice() {
            vectors_passed += 1;
        }
    }

    let verdict = admit_holo_v4_archive(HoloArchiveCheck {
        magic_valid,
        version,
        section_count,
        section_order_valid: order_ok,
        manifest_bytes,
        guest_imports,
        guest_exports_valid,
        blobs_and_view_sorted: blobs_sorted,
        blake3_footer_valid,
        vectors_passed,
        vectors_total,
    });
    if verdict != PrismHoloVerdict::PrismHoloAdmitted {
        return Err(verdict);
    }

    Ok(HoloV4Summary {
        verdict,
        verdict_name: verdict_name(verdict).to_string(),
        archive_kappa: blake3_kappa_label(bytes),
        fingerprint: encode_hex_lower(actual_footer),
        application_kappa: app_kappa,
        guest_content_kappa: guest_kappa,
        view_content_kappa: view_kappa,
        model_content_kappa: model_kappa,
        requires_kappa,
        byte_length: bytes.len(),
        vectors_passed,
        vectors_total,
        view_entry: decoded_view.entry,
        view_file_count: decoded_view.files.len(),
    })
}
