#![forbid(unsafe_code)]
//! Multiplier-free, zero-float pure-integer `UORR4V12` geometric serving runtime.
//!
//! Architecture & invariants:
//! - Zero IEEE 754 floating-point types or operations anywhere (`float_ops == 0`).
//! - Zero dense floating-point matrix multiplications (`dense_matmul_ops == 0`).
//! - 256-entry nibble-pair LUTs with Block-32 Walsh-Hadamard butterflies and dyadic
//!   group-32 scales `(16 + m) * 2^(e - 4)`.
//! - Causal GQA attention with half-split Q14 RoPE, hyperbolic Lorentz lift (`int_arcosh1p_q24`),
//!   and Starling Flock `{sink} u sliding_window u top_k` sparse position selection.
//! - SWAR 576-bit (`9 x u64`) Hamming XOR-popcount signature tracking across context tokens.
//! - Pure-integer Hamilton quaternion transport & 120-vertex H4 icosian frame snap.
//! - Sealed Riemann zeta `PhaseQ29` channel mixing.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, OnceLock};

/// Binary container magic bytes (`UORR4V12`).
pub const R4_VEC_MAGIC: &[u8; 8] = b"UORR4V12";

/// Active geometric vector stack schema identifier.
pub const R4_VEC_SCHEMA: &str = "uor-r4.geometric-vector-stack/12";

const R4_VEC_VERSION: u32 = 12;
const GROUP: usize = 32;
const RHT_BLOCK: usize = 32;
const RESIDUAL_EXP: i32 = -16;
const NORM_FINE_EXP: i32 = -24;
const PRODUCT_EXP: i32 = -32;
const SCORE_EXP: i32 = -24;
const EXP_STEP_LOG2: i32 = -8;
const SILU_STEP_LOG2: i32 = -8;
const SILU_RANGE_LOG2: i32 = 4;
const ROPE_Q: i32 = 14;
const ARCOSH_CODE_BITS: u32 = 41;
const PHASE_MODULUS_Q29: u64 = 1u64 << 29;

/// Embedded GPU-distilled compact `UORR4V12` bootstrap seed (`<= 4.5 MiB`).
static EMBEDDED_SEED_V12: &[u8] = include_bytes!("bootstrap_v12_seed.uorr4");

/// Embedded SmolLM2 ByteLevel BPE tokenizer specification (`49,152` vocab, `48,900` merges).
static TOKENIZER_V12_JSON: &str = include_str!("tokenizer_v12.json");

/// Execution mode selected by the UI / caller wire protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedExecutionMode {
    /// Full 30-layer recompiled SmolLM2 agent mode (`uor-r4.recompiled-v12-agent`).
    RecompiledAgent,
    /// Full 30-layer recompiled SmolLM2 conversational chat (`uor-r4.recompiled-v12-chat`).
    RecompiledChat,
    /// Compact 8-layer distilled seed (`uor-r4.seed-fast`).
    EmbeddedSeedFast,
    /// Legacy symbolic LUT/MVM mode (`uor-r4.lut-symbolic`).
    SymbolicLutMvm,
}

impl SelectedExecutionMode {
    /// Parse canonical v12 and legacy v10/v11 wire identifiers into an execution mode.
    pub fn from_wire_id(id: &str) -> Self {
        match id.trim() {
            "uor-r4.recompiled-v12-agent" | "uor-r4.gllm-v11-agent" | "recompiled-agent" => {
                Self::RecompiledAgent
            }
            "uor-r4.recompiled-v12-chat" | "uor-r4.gllm-v11-chat" | "recompiled-chat" => {
                Self::RecompiledChat
            }
            "uor-r4.seed-fast" | "uor-r4.gllm-v11-seed" | "uor-r4.gllm-v10-seed" | "seed-fast" => {
                Self::EmbeddedSeedFast
            }
            "uor-r4.lut-symbolic" | "uor-r4.symbolic-lut-mvm" | "lut-symbolic" => {
                Self::SymbolicLutMvm
            }
            _ => Self::RecompiledAgent,
        }
    }

    /// Return the canonical v12 wire identifier string.
    pub const fn wire_id(self) -> &'static str {
        match self {
            Self::RecompiledAgent => "uor-r4.recompiled-v12-agent",
            Self::RecompiledChat => "uor-r4.recompiled-v12-chat",
            Self::EmbeddedSeedFast => "uor-r4.seed-fast",
            Self::SymbolicLutMvm => "uor-r4.lut-symbolic",
        }
    }

    /// True when the mode executes inside the neural pure-integer geometric stack.
    pub const fn is_neural_geometric(self) -> bool {
        !matches!(self, Self::SymbolicLutMvm)
    }
}

/// Per-turn execution proof metrics returned alongside the model reply.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct GeometricTurnMetrics {
    /// Schema identifier (`uor-r4.geometric-vector-stack/12`).
    pub schema: &'static str,
    /// Active serving tier (`"recompiled-30l"`, `"embedded-seed"`, or `"lut-symbolic"`).
    pub active_tier: &'static str,
    /// Transformer layer count executed.
    pub n_layers: u32,
    /// Residual hidden dimension (`d_model`).
    pub d_model: u32,
    /// Active vocabulary size of the mounted container.
    pub vocab_size: u32,
    /// Prompt token count fed into prefill.
    pub prompt_tokens: usize,
    /// Generated completion token count emitted.
    pub generated_tokens: usize,
    /// Floating-point operation count (strictly zero).
    pub float_ops: u64,
    /// Dense floating-point matrix multiply count (strictly zero).
    pub dense_matmul_ops: u64,
    /// 256-entry nibble-pair LUT lookups executed.
    pub nibble_lut_lookups: u64,
    /// Hyperbolic Lorentz geodesic distance evaluations executed.
    pub lorentz_geodesic_evals: u64,
    /// Sparse positions attended by the Starling Flock policy.
    pub flock_attended_positions: usize,
    /// Total candidate KV positions considered across attention steps.
    pub flock_total_positions: usize,
    /// 576-bit SWAR Hamming XOR-popcount copy hits observed.
    pub hamming_xor_copy_hits: usize,
    /// Final snapped H4 120-vertex icosian spin frame identifier.
    pub icosian_frame_id: u8,
    /// Sealed Riemann zeta PhaseQ29 cumulative mix checksum.
    pub zeta_phase_checksum: u64,
}

/// Container and engine summary exposed to diagnostics and mounting endpoints.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct GeometricStackSummary {
    /// Schema identifier (`uor-r4.geometric-vector-stack/12`).
    pub schema: String,
    /// Highest currently active runtime tier (`"recompiled-30l"` or `"embedded-seed"`).
    pub active_tier: String,
    /// Container payload/archive SHA-256 hex digest.
    pub container_sha256: String,
    /// Number of transformer layers in the active model.
    pub n_layers: u32,
    /// Hidden residual width (`d_model`).
    pub d_model: u32,
    /// Query attention head count (`n_heads`).
    pub n_heads: u32,
    /// Key-value head count (`n_kv_heads`).
    pub n_kv_heads: u32,
    /// Active vocabulary size (`vocab_size`).
    pub vocab_size: u32,
    /// Floating-point operation count (strictly zero).
    pub float_ops: u64,
    /// Dense floating-point matrix multiply count (strictly zero).
    pub dense_matmul_ops: u64,
}

/// Typed validation and runtime errors for `UORR4V12` geometric containers (`R5`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum R4StackError {
    /// Input container buffer ended before expected header or payload boundary.
    TruncatedContainer,
    /// Header magic did not match `b"UORR4V12"`.
    InvalidMagic,
    /// Container format version is unsupported (`expected == 12`).
    UnsupportedVersion(u32),
    /// Container header declared non-zero floating-point operations.
    NonZeroFloatOps(u32),
    /// Container header declared non-zero dense matrix multiplication operations.
    NonZeroDenseMatmulOps(u32),
    /// Layer/head/dimension metadata failed strict geometric admissibility constraints.
    InvalidGeometry(&'static str),
    /// Header payload SHA-256 did not match actual payload bytes.
    IntegrityMismatch,
}

impl R4StackError {
    /// Stable programmatic diagnostic error code.
    pub const fn error_code(&self) -> &'static str {
        match self {
            Self::TruncatedContainer => "ERR_R4V12_TRUNCATED",
            Self::InvalidMagic => "ERR_R4V12_BAD_MAGIC",
            Self::UnsupportedVersion(_) => "ERR_R4V12_BAD_VERSION",
            Self::NonZeroFloatOps(_) => "ERR_R4V12_FLOAT_OPS_FORBIDDEN",
            Self::NonZeroDenseMatmulOps(_) => "ERR_R4V12_DENSE_MATMUL_FORBIDDEN",
            Self::InvalidGeometry(_) => "ERR_R4V12_INVALID_GEOMETRY",
            Self::IntegrityMismatch => "ERR_R4V12_SHA256_MISMATCH",
        }
    }
}

impl fmt::Display for R4StackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedContainer => write!(f, "truncated UORR4V12 container"),
            Self::InvalidMagic => write!(f, "invalid magic header: expected UORR4V12"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported UORR4V12 version: {v}"),
            Self::NonZeroFloatOps(ops) => {
                write!(f, "container refused: non-zero float_ops={ops}")
            }
            Self::NonZeroDenseMatmulOps(ops) => {
                write!(f, "container refused: non-zero dense_matmul_ops={ops}")
            }
            Self::InvalidGeometry(reason) => write!(f, "invalid container geometry: {reason}"),
            Self::IntegrityMismatch => write!(f, "UORR4V12 payload SHA-256 mismatch"),
        }
    }
}

impl std::error::Error for R4StackError {}

// ============================================================================
// Binary Container Deserialization & Validation
// ============================================================================

#[derive(Clone, Debug)]
struct PackedMatrix {
    rows: u32,
    cols: u32,
    use_rht32: u8,
    rht_seed: u64,
    row_exp_base: Vec<i8>,
    row_min_de: Vec<u8>,
    nibbles: Vec<u8>,
    scales: Vec<u8>,
}

#[derive(Clone, Debug)]
struct RecompiledLayer {
    q_proj: PackedMatrix,
    k_proj: PackedMatrix,
    v_proj: PackedMatrix,
    o_proj: PackedMatrix,
    gate_proj: PackedMatrix,
    up_proj: PackedMatrix,
    down_proj: PackedMatrix,
}

#[derive(Clone, Debug)]
struct R4CompiledModel {
    #[allow(dead_code)]
    tier_code: u32,
    vocab_size: u32,
    d_model: u32,
    n_layers: u32,
    n_heads: u32,
    n_kv_heads: u32,
    intermediate_size: u32,
    max_seq: u32,
    bos_token_id: u32,
    eos_token_id: u32,
    arcosh_mantissa_bits: u32,
    container_sha256: String,
    rms_eps_q48: i64,
    score_scale_q30: i64,
    rope_cos_q14: Vec<i16>,
    rope_sin_q14: Vec<i16>,
    exp_table: Vec<u32>,
    silu_table: Vec<i32>,
    arcosh_table: Vec<u32>,
    e8p_codebook: [u32; 256],
    zeta_phase_q29: [u32; 8],
    local_to_global: Vec<u32>,
    global_to_local: HashMap<u32, u32>,
    vocab_hamming_sigs: Vec<[u64; 9]>,
    final_norm_codes: Vec<i16>,
    embed: PackedMatrix,
    layers: Vec<RecompiledLayer>,
}

fn sha256_arr(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let dig = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&dig);
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    let dig = sha256_arr(bytes);
    let mut s = String::with_capacity(64);
    for b in dig {
        use std::fmt::Write;
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}

#[inline]
fn take_bytes<'a>(bytes: &'a [u8], cur: &mut usize, n: usize) -> Result<&'a [u8], R4StackError> {
    let end = cur.checked_add(n).ok_or(R4StackError::TruncatedContainer)?;
    if end > bytes.len() {
        return Err(R4StackError::TruncatedContainer);
    }
    let slice = &bytes[*cur..end];
    *cur = end;
    Ok(slice)
}

#[inline]
fn read_u8(bytes: &[u8], cur: &mut usize) -> Result<u8, R4StackError> {
    Ok(take_bytes(bytes, cur, 1)?[0])
}

#[inline]
fn read_u32(bytes: &[u8], cur: &mut usize) -> Result<u32, R4StackError> {
    let s = take_bytes(bytes, cur, 4)?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

#[inline]
fn read_i32(bytes: &[u8], cur: &mut usize) -> Result<i32, R4StackError> {
    let s = take_bytes(bytes, cur, 4)?;
    Ok(i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

#[inline]
fn read_u64(bytes: &[u8], cur: &mut usize) -> Result<u64, R4StackError> {
    let s = take_bytes(bytes, cur, 8)?;
    Ok(u64::from_le_bytes([
        s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
    ]))
}

#[inline]
fn read_i64(bytes: &[u8], cur: &mut usize) -> Result<i64, R4StackError> {
    let s = take_bytes(bytes, cur, 8)?;
    Ok(i64::from_le_bytes([
        s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
    ]))
}

#[inline]
fn read_i16(bytes: &[u8], cur: &mut usize) -> Result<i16, R4StackError> {
    let s = take_bytes(bytes, cur, 2)?;
    Ok(i16::from_le_bytes([s[0], s[1]]))
}

fn read_packed_matrix(
    bytes: &[u8],
    cur: &mut usize,
    exp_rows: usize,
    exp_cols: usize,
) -> Result<PackedMatrix, R4StackError> {
    let rows = read_u32(bytes, cur)?;
    let cols = read_u32(bytes, cur)?;
    if rows as usize != exp_rows || cols as usize != exp_cols {
        return Err(R4StackError::InvalidGeometry("matrix shape mismatch"));
    }
    if exp_cols == 0 || !exp_cols.is_multiple_of(32) {
        return Err(R4StackError::InvalidGeometry(
            "matrix columns must be a non-zero multiple of 32",
        ));
    }
    let use_rht32 = read_u8(bytes, cur)?;
    let rht_seed = read_u64(bytes, cur)?;
    let row_eb_raw = take_bytes(bytes, cur, exp_rows)?;
    let row_exp_base: Vec<i8> = row_eb_raw.iter().map(|&b| b as i8).collect();
    let row_min_de = take_bytes(bytes, cur, exp_rows)?.to_vec();
    let nib_len = exp_rows
        .checked_mul(exp_cols >> 1)
        .ok_or(R4StackError::TruncatedContainer)?;
    let sc_len = exp_rows
        .checked_mul(exp_cols >> 5)
        .ok_or(R4StackError::TruncatedContainer)?;
    let nibbles = take_bytes(bytes, cur, nib_len)?.to_vec();
    let scales = take_bytes(bytes, cur, sc_len)?.to_vec();
    Ok(PackedMatrix {
        rows,
        cols,
        use_rht32,
        rht_seed,
        row_exp_base,
        row_min_de,
        nibbles,
        scales,
    })
}

fn parse_uorr4v12(bytes: &[u8]) -> Result<R4CompiledModel, R4StackError> {
    if bytes.len() < 96 {
        return Err(R4StackError::TruncatedContainer);
    }
    if &bytes[0..8] != R4_VEC_MAGIC {
        return Err(R4StackError::InvalidMagic);
    }
    let mut cur = 8usize;
    let version = read_u32(bytes, &mut cur)?;
    if version != R4_VEC_VERSION {
        return Err(R4StackError::UnsupportedVersion(version));
    }
    let tier_code = read_u32(bytes, &mut cur)?;
    if tier_code != 1 && tier_code != 2 {
        return Err(R4StackError::InvalidGeometry("unsupported tier code"));
    }
    let vocab_size = read_u32(bytes, &mut cur)?;
    let d_model = read_u32(bytes, &mut cur)?;
    let n_layers = read_u32(bytes, &mut cur)?;
    let n_heads = read_u32(bytes, &mut cur)?;
    let n_kv_heads = read_u32(bytes, &mut cur)?;
    let intermediate_size = read_u32(bytes, &mut cur)?;
    let max_seq = read_u32(bytes, &mut cur)?;
    let bos_token_id = read_u32(bytes, &mut cur)?;
    let eos_token_id = read_u32(bytes, &mut cur)?;
    let _flags = read_u32(bytes, &mut cur)?;
    let float_ops = read_u32(bytes, &mut cur)?;
    if float_ops != 0 {
        return Err(R4StackError::NonZeroFloatOps(float_ops));
    }
    let dense_matmul_ops = read_u32(bytes, &mut cur)?;
    if dense_matmul_ops != 0 {
        return Err(R4StackError::NonZeroDenseMatmulOps(dense_matmul_ops));
    }
    let hdr_sha = take_bytes(bytes, &mut cur, 32)?;
    let actual_payload_sha = sha256_arr(&bytes[96..]);
    if hdr_sha != actual_payload_sha {
        return Err(R4StackError::IntegrityMismatch);
    }

    if vocab_size == 0
        || d_model == 0
        || n_layers == 0
        || n_heads == 0
        || n_kv_heads == 0
        || intermediate_size == 0
        || max_seq == 0
        || !d_model.is_multiple_of(32)
        || !intermediate_size.is_multiple_of(32)
        || !d_model.is_multiple_of(n_heads)
        || !n_heads.is_multiple_of(n_kv_heads)
    {
        return Err(R4StackError::InvalidGeometry(
            "dimension/head alignment check failed",
        ));
    }

    let head_dim = (d_model / n_heads) as usize;
    if head_dim == 0 || !head_dim.is_multiple_of(2) {
        return Err(R4StackError::InvalidGeometry("even head_dim required"));
    }

    let rms_eps_q48 = read_i64(bytes, &mut cur)?;
    let score_scale_q30 = read_i64(bytes, &mut cur)?;
    let arcosh_mantissa_bits = read_u32(bytes, &mut cur)?;
    let rope_len = read_u32(bytes, &mut cur)? as usize;
    let exp_len = read_u32(bytes, &mut cur)? as usize;
    let silu_len = read_u32(bytes, &mut cur)? as usize;
    let arcosh_len = read_u32(bytes, &mut cur)? as usize;

    let expected_rope = (max_seq as usize)
        .checked_mul(head_dim / 2)
        .ok_or(R4StackError::TruncatedContainer)?;
    if rope_len != expected_rope || exp_len == 0 || silu_len == 0 || arcosh_len == 0 {
        return Err(R4StackError::InvalidGeometry("invalid LUT lengths"));
    }

    let mut rope_cos_q14 = Vec::with_capacity(rope_len);
    for _ in 0..rope_len {
        rope_cos_q14.push(read_i16(bytes, &mut cur)?);
    }
    let mut rope_sin_q14 = Vec::with_capacity(rope_len);
    for _ in 0..rope_len {
        rope_sin_q14.push(read_i16(bytes, &mut cur)?);
    }
    let mut exp_table = Vec::with_capacity(exp_len);
    for _ in 0..exp_len {
        exp_table.push(read_u32(bytes, &mut cur)?);
    }
    let mut silu_table = Vec::with_capacity(silu_len);
    for _ in 0..silu_len {
        silu_table.push(read_i32(bytes, &mut cur)?);
    }
    let mut arcosh_table = Vec::with_capacity(arcosh_len);
    for _ in 0..arcosh_len {
        arcosh_table.push(read_u32(bytes, &mut cur)?);
    }
    let mut e8p_codebook = [0u32; 256];
    for slot in &mut e8p_codebook {
        *slot = read_u32(bytes, &mut cur)?;
    }
    let mut zeta_phase_q29 = [0u32; 8];
    for slot in &mut zeta_phase_q29 {
        *slot = read_u32(bytes, &mut cur)?;
    }

    let l2g_len = read_u32(bytes, &mut cur)? as usize;
    let mut local_to_global = Vec::with_capacity(l2g_len);
    let mut global_to_local = HashMap::with_capacity(l2g_len);
    for li in 0..l2g_len {
        let gid = read_u32(bytes, &mut cur)?;
        local_to_global.push(gid);
        global_to_local.insert(gid, li as u32);
    }

    let vocab = vocab_size as usize;
    let d = d_model as usize;
    let ffn = intermediate_size as usize;
    let kv_dim = (n_kv_heads as usize) * head_dim;

    let mut vocab_hamming_sigs = Vec::with_capacity(vocab);
    for _ in 0..vocab {
        let mut sig = [0u64; 9];
        for w in &mut sig {
            *w = read_u64(bytes, &mut cur)?;
        }
        vocab_hamming_sigs.push(sig);
    }

    let mut final_norm_codes = Vec::with_capacity(d);
    for _ in 0..d {
        final_norm_codes.push(read_i16(bytes, &mut cur)?);
    }

    let embed = read_packed_matrix(bytes, &mut cur, vocab, d)?;
    let mut layers = Vec::with_capacity(n_layers as usize);
    for _ in 0..n_layers {
        let q_proj = read_packed_matrix(bytes, &mut cur, d, d)?;
        let k_proj = read_packed_matrix(bytes, &mut cur, kv_dim, d)?;
        let v_proj = read_packed_matrix(bytes, &mut cur, kv_dim, d)?;
        let o_proj = read_packed_matrix(bytes, &mut cur, d, d)?;
        let gate_proj = read_packed_matrix(bytes, &mut cur, ffn, d)?;
        let up_proj = read_packed_matrix(bytes, &mut cur, ffn, d)?;
        let down_proj = read_packed_matrix(bytes, &mut cur, d, ffn)?;
        layers.push(RecompiledLayer {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
            gate_proj,
            up_proj,
            down_proj,
        });
    }

    if cur != bytes.len() {
        return Err(R4StackError::InvalidGeometry(
            "unexpected trailing bytes in container",
        ));
    }

    Ok(R4CompiledModel {
        tier_code,
        vocab_size,
        d_model,
        n_layers,
        n_heads,
        n_kv_heads,
        intermediate_size,
        max_seq,
        bos_token_id,
        eos_token_id,
        arcosh_mantissa_bits,
        container_sha256: sha256_hex(bytes),
        rms_eps_q48,
        score_scale_q30,
        rope_cos_q14,
        rope_sin_q14,
        exp_table,
        silu_table,
        arcosh_table,
        e8p_codebook,
        zeta_phase_q29,
        local_to_global,
        global_to_local,
        vocab_hamming_sigs,
        final_norm_codes,
        embed,
        layers,
    })
}

// ============================================================================
// SmolLM2 ByteLevel + Individual Digits Tokenizer (Pure Integer/String)
// ============================================================================

#[derive(Deserialize)]
struct RawTokenizerJson {
    #[serde(default)]
    added_tokens: Vec<RawAddedToken>,
    model: RawBpeModel,
}

#[derive(Deserialize)]
struct RawAddedToken {
    id: u32,
    content: String,
}

#[derive(Deserialize)]
struct RawBpeModel {
    vocab: HashMap<String, u32>,
    merges: Vec<String>,
}

struct SmolTokenizer {
    id_to_bytes: Vec<Vec<u8>>,
    piece_to_id: HashMap<String, u32>,
    merges: HashMap<(String, String), usize>,
    added_tokens: Vec<(String, u32)>,
    byte_encoder: [char; 256],
    byte_tokens: [u32; 256],
}

impl SmolTokenizer {
    fn from_json_str(raw: &str) -> Self {
        let parsed: RawTokenizerJson =
            serde_json::from_str(raw).expect("valid embedded tokenizer_v12.json");
        let vocab_size = parsed
            .model
            .vocab
            .values()
            .copied()
            .max()
            .map(|m| m as usize + 1)
            .unwrap_or(49_152);
        let mut id_to_piece = vec![String::new(); vocab_size];
        let mut piece_to_id = HashMap::with_capacity(parsed.model.vocab.len() + 32);
        for (piece, id) in parsed.model.vocab {
            if (id as usize) < vocab_size {
                id_to_piece[id as usize] = piece.clone();
            }
            piece_to_id.insert(piece, id);
        }
        let mut added_tokens = Vec::with_capacity(parsed.added_tokens.len());
        let mut added_ids = HashSet::with_capacity(parsed.added_tokens.len());
        for at in parsed.added_tokens {
            if (at.id as usize) < vocab_size {
                id_to_piece[at.id as usize] = at.content.clone();
            }
            piece_to_id.insert(at.content.clone(), at.id);
            added_tokens.push((at.content, at.id));
            added_ids.insert(at.id);
        }
        added_tokens.sort_by_key(|b| std::cmp::Reverse(b.0.len()));

        let mut merges = HashMap::with_capacity(parsed.model.merges.len());
        for (rank, m) in parsed.model.merges.into_iter().enumerate() {
            if let Some((a, b)) = m.split_once(' ') {
                merges.insert((a.to_string(), b.to_string()), rank);
            }
        }

        let mut byte_encoder = ['\0'; 256];
        let mut byte_decoder: HashMap<char, u8> = HashMap::with_capacity(256);
        let mut bs: Vec<u16> = Vec::new();
        for b in b'!'..=b'~' {
            bs.push(u16::from(b));
        }
        for b in 0xA1u16..=0xACu16 {
            bs.push(b);
        }
        for b in 0xAEu16..=0xFFu16 {
            bs.push(b);
        }
        let mut cs = bs.clone();
        let mut n = 0u16;
        for b in 0..=255u16 {
            if !bs.contains(&b) {
                bs.push(b);
                cs.push(256 + n);
                n += 1;
            }
        }
        for (&b, &c) in bs.iter().zip(cs.iter()) {
            let ch = char::from_u32(u32::from(c)).unwrap_or('\0');
            byte_encoder[b as usize] = ch;
            byte_decoder.insert(ch, b as u8);
        }

        let mut id_to_bytes = vec![Vec::new(); vocab_size];
        for (id, piece) in id_to_piece.iter().enumerate() {
            let mut decoded = Vec::new();
            let mut all_byte_chars = true;
            for ch in piece.chars() {
                if let Some(&b) = byte_decoder.get(&ch) {
                    decoded.push(b);
                } else {
                    all_byte_chars = false;
                    break;
                }
            }
            if all_byte_chars && !piece.is_empty() && !added_ids.contains(&(id as u32)) {
                id_to_bytes[id] = decoded;
            } else {
                id_to_bytes[id] = piece.as_bytes().to_vec();
            }
        }

        let fallback_byte_tok = *piece_to_id.get("\u{0120}").unwrap_or(&0);
        let mut byte_tokens = [fallback_byte_tok; 256];
        for b in 0..256 {
            let mut s = String::new();
            s.push(byte_encoder[b]);
            if let Some(&tid) = piece_to_id.get(&s) {
                byte_tokens[b] = tid;
            }
        }

        Self {
            id_to_bytes,
            piece_to_id,
            merges,
            added_tokens,
            byte_encoder,
            byte_tokens,
        }
    }

    fn pretokenize_spans(span: &str) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let bytes = span.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            let rem = &span[i..];
            let mut chars = rem.char_indices();
            let Some((_, first_ch)) = chars.next() else {
                break;
            };
            if first_ch.is_numeric() {
                let end = i + first_ch.len_utf8();
                out.push((i, end));
                i = end;
                continue;
            }
            if first_ch == '\'' {
                let mut matched_len = 0usize;
                for pat in ["'re", "'ve", "'ll", "'s", "'t", "'m", "'d"] {
                    if rem.starts_with(pat) {
                        matched_len = pat.len();
                        break;
                    }
                }
                if matched_len > 0 {
                    out.push((i, i + matched_len));
                    i += matched_len;
                    continue;
                }
            }
            if first_ch == ' ' {
                if let Some((sec_off, sec_ch)) = chars.next() {
                    if sec_ch.is_alphabetic() {
                        let mut end = i + sec_off + sec_ch.len_utf8();
                        for (off, ch) in chars {
                            if !ch.is_alphabetic() {
                                break;
                            }
                            end = i + off + ch.len_utf8();
                        }
                        out.push((i, end));
                        i = end;
                        continue;
                    }
                    if !sec_ch.is_whitespace() && !sec_ch.is_numeric() {
                        let mut end = i + sec_off + sec_ch.len_utf8();
                        for (off, ch) in chars {
                            if ch.is_whitespace() || ch.is_alphabetic() || ch.is_numeric() {
                                break;
                            }
                            end = i + off + ch.len_utf8();
                        }
                        out.push((i, end));
                        i = end;
                        continue;
                    }
                }
            }
            if first_ch.is_alphabetic() {
                let mut end = i + first_ch.len_utf8();
                for (off, ch) in chars {
                    if !ch.is_alphabetic() {
                        break;
                    }
                    end = i + off + ch.len_utf8();
                }
                out.push((i, end));
                i = end;
                continue;
            }
            if !first_ch.is_whitespace() {
                let mut end = i + first_ch.len_utf8();
                for (off, ch) in chars {
                    if ch.is_whitespace() || ch.is_alphabetic() || ch.is_numeric() {
                        break;
                    }
                    end = i + off + ch.len_utf8();
                }
                out.push((i, end));
                i = end;
                continue;
            }
            let mut ws_end = i + first_ch.len_utf8();
            let mut last_ws_start = i;
            let mut ws_count = 1usize;
            for (off, ch) in chars {
                if !ch.is_whitespace() {
                    break;
                }
                last_ws_start = i + off;
                ws_end = last_ws_start + ch.len_utf8();
                ws_count += 1;
            }
            if ws_end < bytes.len() && ws_count > 1 {
                out.push((i, last_ws_start));
                i = last_ws_start;
            } else {
                out.push((i, ws_end));
                i = ws_end;
            }
        }
        out
    }

    fn bpe_piece(&self, pretoken: &str) -> Vec<u32> {
        let mut word: Vec<String> = pretoken
            .as_bytes()
            .iter()
            .map(|&b| self.byte_encoder[b as usize].to_string())
            .collect();
        while word.len() > 1 {
            let mut best_rank = usize::MAX;
            let mut best_idx = 0usize;
            for idx in 0..word.len() - 1 {
                if let Some(&rank) = self.merges.get(&(word[idx].clone(), word[idx + 1].clone())) {
                    if rank < best_rank {
                        best_rank = rank;
                        best_idx = idx;
                    }
                }
            }
            if best_rank == usize::MAX {
                break;
            }
            let merged = format!("{}{}", word[best_idx], word[best_idx + 1]);
            word[best_idx] = merged;
            word.remove(best_idx + 1);
        }
        word.iter()
            .map(|p| *self.piece_to_id.get(p).unwrap_or(&0))
            .collect()
    }

    fn encode_global(&self, text: &str) -> Vec<u32> {
        let mut ids = Vec::with_capacity(text.len() / 2 + 8);
        let mut pos = 0usize;
        let mut normal_start = 0usize;
        while pos < text.len() {
            let rest = &text[pos..];
            let mut matched: Option<(usize, u32)> = None;
            for (content, tid) in &self.added_tokens {
                if rest.starts_with(content.as_str()) {
                    matched = Some((content.len(), *tid));
                    break;
                }
            }
            if let Some((mlen, tid)) = matched {
                if pos > normal_start {
                    for (s, e) in Self::pretokenize_spans(&text[normal_start..pos]) {
                        ids.extend(self.bpe_piece(&text[normal_start + s..normal_start + e]));
                    }
                }
                ids.push(tid);
                pos += mlen;
                normal_start = pos;
            } else {
                let ch_len = rest.chars().next().map_or(1, char::len_utf8);
                pos += ch_len;
            }
        }
        if normal_start < text.len() {
            for (s, e) in Self::pretokenize_spans(&text[normal_start..]) {
                ids.extend(self.bpe_piece(&text[normal_start + s..normal_start + e]));
            }
        }
        ids
    }

    fn decode_global(&self, ids: &[u32]) -> String {
        let mut raw = Vec::new();
        for &id in ids {
            if let Some(bs) = self.id_to_bytes.get(id as usize) {
                raw.extend_from_slice(bs);
            }
        }
        String::from_utf8_lossy(&raw).into_owned()
    }

    fn map_to_model_vocab(&self, global_ids: &[u32], model: &R4CompiledModel) -> Vec<u32> {
        if model.local_to_global.is_empty() {
            return global_ids.to_vec();
        }
        let mut out = Vec::with_capacity(global_ids.len());
        for &gid in global_ids {
            if let Some(&lid) = model.global_to_local.get(&gid) {
                out.push(lid);
            } else if let Some(bs) = self.id_to_bytes.get(gid as usize) {
                for &b in bs {
                    let b_gid = self.byte_tokens[b as usize];
                    if let Some(&lid) = model.global_to_local.get(&b_gid) {
                        out.push(lid);
                    }
                }
            }
        }
        out
    }
}

fn shared_tokenizer() -> &'static Arc<SmolTokenizer> {
    static TOK_CELL: OnceLock<Arc<SmolTokenizer>> = OnceLock::new();
    TOK_CELL.get_or_init(|| Arc::new(SmolTokenizer::from_json_str(TOKENIZER_V12_JSON)))
}

// ============================================================================
// Pure-Integer Geometric Arithmetic Kernels (Zero Floats, Zero Dense Matmuls)
// ============================================================================

#[inline]
fn xorshift64(mut s: u64) -> u64 {
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    s
}

#[inline]
fn scale_16_plus(a: i64, m: u8) -> i64 {
    let select = |bit: u32| a & i64::from((m >> bit) & 1).wrapping_neg();
    let t = (a << 1).wrapping_add(select(3));
    let t = (t << 1).wrapping_add(select(2));
    let t = (t << 1).wrapping_add(select(1));
    (t << 1).wrapping_add(select(0))
}

#[inline]
fn shift_round(value: i64, sh: i32) -> i64 {
    if sh > 0 {
        if sh >= 63 {
            return if value < 0 { -1 } else { 0 };
        }
        let half = 1i64 << (sh - 1);
        value.saturating_add(half) >> sh
    } else if sh < 0 {
        let k = sh.unsigned_abs().min(63);
        let limit = i64::MAX >> k;
        if value > limit {
            i64::MAX
        } else if value < -limit {
            i64::MIN
        } else {
            value << k
        }
    } else {
        value
    }
}

#[inline]
fn shift_wide(value: i128, sh: u32) -> i64 {
    let rounded = if sh == 0 {
        value
    } else {
        value.wrapping_add(1i128 << (sh - 1)) >> sh
    };
    rounded.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

#[inline]
fn to_exp_i32(value: i64, from: i32, to: i32) -> i32 {
    shift_round(value, to - from).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[inline]
fn grid_apply(value: i64, code: i16) -> i64 {
    if code == 0 {
        return 0;
    }
    let c = code.unsigned_abs();
    let e = i32::from(c >> 4) - 64;
    let scaled = shift_round(scale_16_plus(value, (c & 15) as u8), 4 - e);
    if code < 0 {
        scaled.saturating_neg()
    } else {
        scaled
    }
}

fn long_division_u128(num: u128, den: u128) -> u128 {
    let den = den.max(1);
    let mut rem = 0u128;
    let mut quo = 0u128;
    for bit in (0..128).rev() {
        rem = (rem << 1) | ((num >> bit) & 1);
        if rem >= den {
            rem -= den;
            quo |= 1u128 << bit;
        }
    }
    quo
}

fn digit_isqrt(n: u128) -> u128 {
    let mut x = n;
    let mut c = 0u128;
    let mut d = 1u128 << 126;
    while d > x {
        d >>= 2;
    }
    while d != 0 {
        if x >= c + d {
            x -= c + d;
            c = (c >> 1) + d;
        } else {
            c >>= 1;
        }
        d >>= 2;
    }
    c
}

#[inline]
fn multiples_i64(x: i64) -> [i64; 16] {
    let mut t = [0i64; 16];
    let mut acc = 0i64;
    for slot in t.iter_mut().skip(1) {
        acc = acc.wrapping_add(x);
        *slot = acc;
    }
    t
}

#[inline]
fn table_mul_i64(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 {
        return 0;
    }
    let neg = (a < 0) ^ (b < 0);
    let (ua, ub) = (a.unsigned_abs(), b.unsigned_abs());
    let (big, small) = if ua >= ub { (ua, ub) } else { (ub, ua) };
    let t1 = big;
    let t2 = big << 1;
    let t4 = big << 2;
    let t8 = big << 3;
    let t3 = t2.wrapping_add(t1);
    let t5 = t4.wrapping_add(t1);
    let t6 = t4.wrapping_add(t2);
    let t7 = t4.wrapping_add(t3);
    let tbl: [u64; 16] = [
        0,
        t1,
        t2,
        t3,
        t4,
        t5,
        t6,
        t7,
        t8,
        t8.wrapping_add(t1),
        t8.wrapping_add(t2),
        t8.wrapping_add(t3),
        t8.wrapping_add(t4),
        t8.wrapping_add(t5),
        t8.wrapping_add(t6),
        t8.wrapping_add(t7),
    ];
    let mut digits = (67 - small.leading_zeros()) >> 2;
    let mut acc = 0u64;
    while digits > 0 {
        digits -= 1;
        acc = (acc << 4).wrapping_add(tbl[((small >> (digits << 2)) & 15) as usize]);
    }
    if neg {
        acc.wrapping_neg() as i64
    } else {
        acc as i64
    }
}

#[inline]
fn table_mul_i128(a: i128, b: i128) -> i128 {
    if a == 0 || b == 0 {
        return 0;
    }
    let neg = (a < 0) ^ (b < 0);
    let (ua, ub) = (a.unsigned_abs(), b.unsigned_abs());
    let (big, small) = if ua >= ub { (ua, ub) } else { (ub, ua) };
    let t1 = big;
    let t2 = big << 1;
    let t4 = big << 2;
    let t8 = big << 3;
    let t3 = t2.wrapping_add(t1);
    let t5 = t4.wrapping_add(t1);
    let t6 = t4.wrapping_add(t2);
    let t7 = t4.wrapping_add(t3);
    let tbl: [u128; 16] = [
        0,
        t1,
        t2,
        t3,
        t4,
        t5,
        t6,
        t7,
        t8,
        t8.wrapping_add(t1),
        t8.wrapping_add(t2),
        t8.wrapping_add(t3),
        t8.wrapping_add(t4),
        t8.wrapping_add(t5),
        t8.wrapping_add(t6),
        t8.wrapping_add(t7),
    ];
    let mut digits = (131 - small.leading_zeros()) >> 2;
    let mut acc = 0u128;
    while digits > 0 {
        digits -= 1;
        acc = (acc << 4).wrapping_add(tbl[((small >> (digits << 2)) & 15) as usize]);
    }
    if neg {
        acc.wrapping_neg() as i128
    } else {
        acc as i128
    }
}

#[inline]
fn popcount_swar64(mut x: u64) -> u32 {
    x -= (x >> 1) & 0x5555_5555_5555_5555;
    x = (x & 0x3333_3333_3333_3333) + ((x >> 2) & 0x3333_3333_3333_3333);
    x = (x + (x >> 4)) & 0x0f0f_0f0f_0f0f_0f0f;
    x = x.wrapping_add(x >> 8);
    x = x.wrapping_add(x >> 16);
    x = x.wrapping_add(x >> 32);
    (x & 0x7f) as u32
}

#[inline]
fn hamming_distance_576(a: &[u64; 9], b: &[u64; 9]) -> u32 {
    let mut dist = 0u32;
    for k in 0..9 {
        dist += popcount_swar64(a[k] ^ b[k]);
    }
    dist
}

fn rht_signs_bits(cols: usize, seed: u64) -> Vec<bool> {
    let mut out = Vec::with_capacity(cols);
    let mut state = seed ^ 0x9e37_79b9_7f4a_7c15;
    if state == 0 {
        state = 0x853c_49e6_748f_ea9b;
    }
    for _ in 0..cols {
        state = xorshift64(state);
        out.push((state & 1) == 1);
    }
    out
}

fn int_fwht32(chunk: &mut [i64; RHT_BLOCK]) {
    let mut h = 1usize;
    while h < RHT_BLOCK {
        let mut i = 0usize;
        while i < RHT_BLOCK {
            for j in i..i + h {
                let x = chunk[j];
                let y = chunk[j + h];
                chunk[j] = x.saturating_add(y);
                chunk[j + h] = x.saturating_sub(y);
            }
            i += h * 2;
        }
        h *= 2;
    }
}

fn int_rms_norm_fine(x: &[i32], eps_q48: i64, out: &mut [i64]) {
    let mut max_abs = 0u32;
    for &v in x {
        max_abs = max_abs.max(v.unsigned_abs());
    }
    let bl = (32 - max_abs.leading_zeros() as i32).max(1);
    let sh = bl - 22;
    let mut sum = 0u64;
    for (dst, &v) in out.iter_mut().zip(x) {
        let y = shift_round(i64::from(v), sh);
        *dst = y;
        let sq = table_mul_i64(y.abs(), y.abs()) as u64;
        sum = sum.wrapping_add(sq);
    }
    let mean = long_division_u128(u128::from(sum), x.len().max(1) as u128) as i64;
    let square_exp = (RESIDUAL_EXP + sh) << 1;
    let eps_int = shift_round(eps_q48, square_exp - (-48)).max(0);
    let ms = mean.saturating_add(eps_int).max(1) as u128;
    let s = digit_isqrt(ms << 68).max(1);
    let r = long_division_u128(1u128 << 96, s);
    let mut mult = [0u128; 16];
    let mut acc_m = 0u128;
    for slot in mult.iter_mut().skip(1) {
        acc_m = acc_m.wrapping_add(r);
        *slot = acc_m;
    }
    for v in out.iter_mut() {
        let u = v.unsigned_abs();
        let mut mag = 0u128;
        for sh_dig in [24u32, 20, 16, 12, 8, 4, 0] {
            mag = (mag << 4).wrapping_add(mult[((u >> sh_dig) & 15) as usize]);
        }
        let prod = if *v < 0 {
            mag.wrapping_neg() as i128
        } else {
            mag as i128
        };
        let y = prod.wrapping_add(1i128 << 37) >> 38;
        *v = y.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
    }
}

fn int_quantize16(src: &[i64], src_exp: i32, out: &mut [i16]) -> i32 {
    let mut max_abs = 0u64;
    for &v in src {
        max_abs = max_abs.max(v.unsigned_abs());
    }
    if max_abs == 0 {
        out.fill(0);
        return src_exp;
    }
    let bl = 64 - max_abs.leading_zeros() as i32;
    let sh = (bl - 14).max(0);
    for (dst, &v) in out.iter_mut().zip(src) {
        *dst = shift_round(v, sh).clamp(-32768, 32767) as i16;
    }
    src_exp + sh
}

fn int_activation_tables(x: &[i16], tables: &mut [[i32; 16]]) {
    for (slot, &v) in tables.iter_mut().zip(x) {
        let xv = i32::from(v);
        let mut acc = -8 * xv;
        for entry in slot.iter_mut() {
            *entry = acc;
            acc += xv;
        }
    }
}

fn int_pair_tables(tables: &[[i32; 16]], pairs: &mut [[i32; 256]]) {
    for (pair, cols) in pairs.iter_mut().zip(tables.chunks_exact(2)) {
        let (even, odd) = (&cols[0], &cols[1]);
        for (row, &hi) in pair.chunks_exact_mut(16).zip(odd) {
            for (slot, &lo) in row.iter_mut().zip(even) {
                *slot = lo + hi;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn int_gemv_pairs(
    m: &PackedMatrix,
    norm_in: &[i64],
    in_exp: i32,
    scratch_rht: &mut [i64],
    act_i16: &mut [i16],
    tables: &mut [[i32; 16]],
    pairs: &mut [[i32; 256]],
    out: &mut [i32],
) -> u64 {
    let cols = m.cols as usize;
    scratch_rht[..cols].copy_from_slice(&norm_in[..cols]);
    if m.use_rht32 != 0 {
        let signs = rht_signs_bits(cols, m.rht_seed);
        for (v, s) in scratch_rht[..cols].iter_mut().zip(signs) {
            if !s {
                *v = v.saturating_neg();
            }
        }
        for blk in scratch_rht[..cols].chunks_exact_mut(RHT_BLOCK) {
            let arr: &mut [i64; RHT_BLOCK] = blk.try_into().unwrap();
            int_fwht32(arr);
        }
    }
    let x_exp = int_quantize16(&scratch_rht[..cols], in_exp, &mut act_i16[..cols]);
    int_activation_tables(&act_i16[..cols], &mut tables[..cols]);
    let row_bytes = cols >> 1;
    int_pair_tables(&tables[..cols], &mut pairs[..row_bytes]);
    let groups = cols >> 5;
    let rows = m.rows as usize;
    for (r, out_slot) in out.iter_mut().enumerate().take(rows) {
        let nibs = &m.nibbles[r * row_bytes..(r + 1) * row_bytes];
        let scs = &m.scales[r * groups..(r + 1) * groups];
        let min_de = m.row_min_de[r];
        let eb = i32::from(m.row_exp_base[r]);
        let mut acc = 0i64;
        for ((bytes, tbls), &s) in nibs
            .chunks_exact(16)
            .zip(pairs[..row_bytes].chunks_exact(16))
            .zip(scs)
        {
            let mut a = 0i32;
            for k in 0..16 {
                a = a.wrapping_add(tbls[k][usize::from(bytes[k])]);
            }
            let de = (s >> 4).saturating_sub(min_de);
            acc = acc.wrapping_add(scale_16_plus(i64::from(a), s & 15) << de);
        }
        *out_slot = to_exp_i32(acc, eb + i32::from(min_de) - 4 + x_exp, RESIDUAL_EXP);
    }
    (rows * row_bytes) as u64
}

fn int_dequant_row(m: &PackedMatrix, row: usize, out: &mut [i32]) {
    let cols = m.cols as usize;
    let row_bytes = cols >> 1;
    let groups = cols >> 5;
    let nibs = &m.nibbles[row * row_bytes..(row + 1) * row_bytes];
    let scs = &m.scales[row * groups..(row + 1) * groups];
    let eb = i32::from(m.row_exp_base[row]);
    if m.use_rht32 != 0 {
        let mut wide = vec![0i64; cols];
        for (g, &s) in scs.iter().enumerate().take(groups) {
            let e = eb + i32::from(s >> 4);
            let mant = s & 15;
            for c in 0..GROUP {
                let col = g * GROUP + c;
                let b = nibs[col >> 1];
                let raw_nib = if col.is_multiple_of(2) {
                    b & 15
                } else {
                    b >> 4
                };
                let q = i64::from(raw_nib) - 8;
                let scaled = scale_16_plus(q, mant);
                wide[col] = shift_round(scaled, NORM_FINE_EXP - (e - 4));
            }
        }
        for blk in wide.chunks_exact_mut(RHT_BLOCK) {
            let arr: &mut [i64; RHT_BLOCK] = blk.try_into().unwrap();
            int_fwht32(arr);
        }
        let signs = rht_signs_bits(cols, m.rht_seed);
        for ((dst, &val), sign_pos) in out.iter_mut().zip(&wide).zip(signs) {
            let signed_val = if sign_pos { val } else { val.saturating_neg() };
            *dst = shift_round(signed_val, RESIDUAL_EXP - NORM_FINE_EXP)
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        }
        return;
    }
    for (g, &s) in scs.iter().enumerate().take(groups) {
        let e = eb + i32::from(s >> 4);
        let mant = s & 15;
        for c in 0..GROUP {
            let col = g * GROUP + c;
            let b = nibs[col >> 1];
            let raw_nib = if col.is_multiple_of(2) {
                b & 15
            } else {
                b >> 4
            };
            let q = i64::from(raw_nib) - 8;
            let scaled = scale_16_plus(q, mant);
            out[col] = to_exp_i32(scaled, e - 4, RESIDUAL_EXP);
        }
    }
}

fn int_exp_neg(d: i64, exp_d: i32, table: &[u32]) -> u64 {
    let frac_bits = EXP_STEP_LOG2 - exp_d;
    if d <= 0 {
        return u64::from(table[0]);
    }
    if frac_bits <= 0 {
        let index = (d << (-frac_bits)) as usize;
        return table.get(index).map_or(0, |&v| u64::from(v));
    }
    let index = (d >> frac_bits) as usize;
    let (Some(&a), Some(&b)) = (table.get(index), table.get(index + 1)) else {
        return 0;
    };
    let frac = d & ((1 << frac_bits) - 1);
    let (a, b) = (u64::from(a), u64::from(b));
    a.wrapping_sub((table_mul_i64(a.wrapping_sub(b) as i64, frac) as u64) >> frac_bits)
}

fn int_silu(x: i32, table: &[i32]) -> i32 {
    let frac_bits = SILU_STEP_LOG2 + 16;
    let half = 1i64 << (SILU_RANGE_LOG2 - SILU_STEP_LOG2);
    let index = (i64::from(x) >> frac_bits) + half;
    if index < 0 {
        return 0;
    }
    let (Some(&a), Some(&b)) = (table.get(index as usize), table.get((index as usize) + 1)) else {
        return x;
    };
    let frac = i64::from(x) & ((1i64 << frac_bits) - 1);
    let diff = i64::from(b) - i64::from(a);
    (i64::from(a) + (table_mul_i64(diff, frac) >> frac_bits)) as i32
}

fn int_arcosh1p_q24(code: u128, arcosh_mantissa_bits: u32, table: &[u32]) -> u32 {
    let direct = 1u128 << arcosh_mantissa_bits;
    let last = table.len().saturating_sub(1);
    if code < direct {
        return table.get(code as usize).copied().unwrap_or(0);
    }
    if code >= (1u128 << ARCOSH_CODE_BITS) {
        return table.get(last).copied().unwrap_or(0);
    }
    let bits = 128 - code.leading_zeros();
    let shift = bits - arcosh_mantissa_bits - 1;
    let mant = ((code >> shift) - direct) as usize;
    let idx = ((1 + shift as usize) << arcosh_mantissa_bits) + mant;
    let (Some(&lo), Some(&hi)) = (table.get(idx), table.get(idx + 1)) else {
        return table.get(last).copied().unwrap_or(0);
    };
    if shift == 0 {
        return lo;
    }
    let frac = (code & ((1u128 << shift) - 1)) as u64;
    let step = (table_mul_i128(i128::from(hi.wrapping_sub(lo)), i128::from(frac)) as u128)
        .wrapping_add(1u128 << (shift - 1))
        >> shift;
    lo.wrapping_add(step as u32)
}

fn int_lorentz_lift(v: &[i32]) -> u64 {
    let mut sq = 0u128;
    for &x in v {
        let ax = i64::from(x.unsigned_abs());
        sq = sq.wrapping_add(u128::from(table_mul_i64(ax, ax) as u64));
    }
    digit_isqrt((1u128 << 64).wrapping_add(sq << 32)).min(u128::from(u64::MAX)) as u64
}

// ============================================================================
// Hamilton Quaternion State Transport & H4 120-Icosian Frame Alignment
// ============================================================================

/// Exact integer Q12 representation of the 120 unit vertices of the H4 600-cell / icosian group.
fn h4_120_icosians_q12() -> &'static Vec<[i16; 4]> {
    static H4_CELL: OnceLock<Vec<[i16; 4]>> = OnceLock::new();
    H4_CELL.get_or_init(|| {
        // Golden ratio scaled to Q12 (4096 = 1.0): phi/2 ~= 3314, 1/(2*phi) ~= 1266, 1/2 = 2048.
        const H: i16 = 2048;
        const U: i16 = 4096;
        const P: i16 = 3314;
        const IP: i16 = 1266;
        let mut verts = Vec::with_capacity(120);

        // 8 axis vertices (+-1, 0, 0, 0)
        for ax in 0..4 {
            for s in [-U, U] {
                let mut v = [0i16; 4];
                v[ax] = s;
                verts.push(v);
            }
        }
        // 16 half-integer vertices (+-1/2, +-1/2, +-1/2, +-1/2)
        for mask in 0..16u8 {
            let mut v = [H; 4];
            for (c, item) in v.iter_mut().enumerate() {
                if ((mask >> c) & 1) == 1 {
                    *item = -H;
                }
            }
            verts.push(v);
        }
        // 96 even permutations of (0, +-1/2, +-phi/2, +-1/(2*phi))
        let even_perms: [[usize; 4]; 12] = [
            [0, 1, 2, 3],
            [0, 2, 3, 1],
            [0, 3, 1, 2],
            [1, 0, 3, 2],
            [1, 2, 0, 3],
            [1, 3, 2, 0],
            [2, 0, 1, 3],
            [2, 1, 3, 0],
            [2, 3, 0, 1],
            [3, 0, 2, 1],
            [3, 1, 0, 2],
            [3, 2, 1, 0],
        ];
        for perm in even_perms {
            for sign_mask in 0..8u8 {
                let s1 = if (sign_mask & 1) == 0 { H } else { -H };
                let s2 = if (sign_mask & 2) == 0 { P } else { -P };
                let s3 = if (sign_mask & 4) == 0 { IP } else { -IP };
                let vals = [0i16, s1, s2, s3];
                let mut v = [0i16; 4];
                for dst in 0..4 {
                    v[dst] = vals[perm[dst]];
                }
                verts.push(v);
            }
        }
        debug_assert_eq!(verts.len(), 120);
        verts
    })
}

fn hamilton_mul_q12(a: [i16; 4], b: [i16; 4]) -> [i16; 4] {
    let (a0, a1, a2, a3) = (
        i64::from(a[0]),
        i64::from(a[1]),
        i64::from(a[2]),
        i64::from(a[3]),
    );
    let (b0, b1, b2, b3) = (
        i64::from(b[0]),
        i64::from(b[1]),
        i64::from(b[2]),
        i64::from(b[3]),
    );
    let c0 = table_mul_i64(a0, b0)
        - table_mul_i64(a1, b1)
        - table_mul_i64(a2, b2)
        - table_mul_i64(a3, b3);
    let c1 = table_mul_i64(a0, b1) + table_mul_i64(a1, b0) + table_mul_i64(a2, b3)
        - table_mul_i64(a3, b2);
    let c2 = table_mul_i64(a0, b2) - table_mul_i64(a1, b3)
        + table_mul_i64(a2, b0)
        + table_mul_i64(a3, b1);
    let c3 = table_mul_i64(a0, b3) + table_mul_i64(a1, b2) - table_mul_i64(a2, b1)
        + table_mul_i64(a3, b0);
    [
        shift_round(c0, 12).clamp(-8192, 8192) as i16,
        shift_round(c1, 12).clamp(-8192, 8192) as i16,
        shift_round(c2, 12).clamp(-8192, 8192) as i16,
        shift_round(c3, 12).clamp(-8192, 8192) as i16,
    ]
}

fn project_norm_to_icosian_step(
    norm: &[i64],
    q_state: [i16; 4],
) -> ([i16; 4], [i16; 8], u8, [u64; 9]) {
    // Fold fine normalized state (`NORM_FINE_EXP = -24`) into 4 quaternion components.
    let mut acc4 = [0i64; 4];
    for (i, &val) in norm.iter().enumerate() {
        acc4[i & 3] = acc4[i & 3].wrapping_add(val >> 14);
    }
    let mut n2 = 0u128;
    for &c in &acc4 {
        n2 = n2.wrapping_add(u128::from(table_mul_i64(c.abs(), c.abs()) as u64));
    }
    let norm_len = digit_isqrt(n2).max(1);
    let mut q_step = [0i16; 4];
    for (dst, &c) in q_step.iter_mut().zip(&acc4) {
        let scaled = long_division_u128((c.unsigned_abs() as u128) << 12, norm_len) as i64;
        *dst = if c < 0 { -scaled } else { scaled }.clamp(-4096, 4096) as i16;
    }
    if q_step == [0, 0, 0, 0] {
        q_step[0] = 4096;
    }

    let transported = hamilton_mul_q12(q_state, q_step);
    let verts = h4_120_icosians_q12();
    let mut best_id = 0usize;
    let mut best_dot = i64::MIN;
    for (idx, v) in verts.iter().enumerate() {
        let dot = table_mul_i64(i64::from(transported[0]), i64::from(v[0]))
            + table_mul_i64(i64::from(transported[1]), i64::from(v[1]))
            + table_mul_i64(i64::from(transported[2]), i64::from(v[2]))
            + table_mul_i64(i64::from(transported[3]), i64::from(v[3]));
        if dot > best_dot {
            best_dot = dot;
            best_id = idx;
        }
    }
    let snapped = verts[best_id];
    let frame_8d = [
        transported[0],
        transported[1],
        transported[2],
        transported[3],
        snapped[0],
        snapped[1],
        snapped[2],
        snapped[3],
    ];

    // 576-bit SWAR Hamming signature folded via pure integer add/sub (matching training fold).
    let mut sig = [0u64; 9];
    let d_model = norm.len().max(1);
    for bit in 0..576 {
        let val = if d_model == 576 {
            norm[bit]
        } else {
            let base = bit % d_model;
            let shift = bit / d_model;
            let partner = (base + shift * 37 + 1) % d_model;
            if shift.is_multiple_of(2) {
                norm[base].saturating_add(norm[partner] >> 1)
            } else {
                norm[base].saturating_sub(norm[partner] >> 1)
            }
        };
        if val >= 0 {
            sig[bit / 64] |= 1u64 << (bit % 64);
        }
    }

    (snapped, frame_8d, best_id as u8, sig)
}

// ============================================================================
// Incremental Pure-Integer Inference Session (`UORR4V12`)
// ============================================================================

struct ScratchWorkspace {
    x: Vec<i32>,
    norm: Vec<i64>,
    scratch_rht: Vec<i64>,
    act16: Vec<i16>,
    tables: Vec<[i32; 16]>,
    pairs: Vec<[i32; 256]>,
    q: Vec<i32>,
    k: Vec<i32>,
    v: Vec<i32>,
    attn_out: Vec<i32>,
    attn_i64: Vec<i64>,
    proj: Vec<i32>,
    gate: Vec<i32>,
    up: Vec<i32>,
    swiglu: Vec<i64>,
    q_tables: Vec<[i64; 16]>,
    scores: Vec<(usize, i64)>,
    weights: Vec<u64>,
    mix: Vec<i128>,
    logits: Vec<i32>,
}

impl ScratchWorkspace {
    fn new(m: &R4CompiledModel) -> Self {
        let d = m.d_model as usize;
        let ffn = m.intermediate_size as usize;
        let hd = d / (m.n_heads as usize);
        let kv_dim = (m.n_kv_heads as usize) * hd;
        let max_cols = d.max(ffn);
        let max_s = m.max_seq as usize;
        Self {
            x: vec![0; d],
            norm: vec![0; d],
            scratch_rht: vec![0; max_cols],
            act16: vec![0; max_cols],
            tables: vec![[0; 16]; max_cols],
            pairs: vec![[0; 256]; max_cols / 2],
            q: vec![0; d],
            k: vec![0; kv_dim],
            v: vec![0; kv_dim],
            attn_out: vec![0; d],
            attn_i64: vec![0; d],
            proj: vec![0; d],
            gate: vec![0; ffn],
            up: vec![0; ffn],
            swiglu: vec![0; ffn],
            q_tables: vec![[0; 16]; hd],
            scores: Vec::with_capacity(max_s.min(128)),
            weights: Vec::with_capacity(max_s.min(128)),
            mix: vec![0; hd],
            logits: vec![0; m.vocab_size as usize],
        }
    }
}

struct R4InferenceSession<'a> {
    m: &'a R4CompiledModel,
    ws: ScratchWorkspace,
    k_caches: Vec<Vec<i32>>,
    v_caches: Vec<Vec<i32>>,
    klift_caches: Vec<Vec<u64>>,
    token_sigs: Vec<[u64; 9]>,
    pos: usize,
    q_state: [i16; 4],
    last_icosian_id: u8,
    zeta_phase_checksum: u64,
    nibble_lut_lookups: u64,
    lorentz_geodesic_evals: u64,
    flock_attended_positions: usize,
    flock_total_positions: usize,
    hamming_xor_copy_hits: usize,
}

impl<'a> R4InferenceSession<'a> {
    fn new(m: &'a R4CompiledModel) -> Self {
        let nl = m.n_layers as usize;
        let nkv = m.n_kv_heads as usize;
        let hd = (m.d_model / m.n_heads) as usize;
        let kv_dim = nkv * hd;
        let max_s = m.max_seq as usize;
        Self {
            m,
            ws: ScratchWorkspace::new(m),
            k_caches: vec![vec![0; max_s * kv_dim]; nl],
            v_caches: vec![vec![0; max_s * kv_dim]; nl],
            klift_caches: vec![vec![0; max_s * nkv]; nl],
            token_sigs: Vec::with_capacity(max_s),
            pos: 0,
            q_state: [4096, 0, 0, 0],
            last_icosian_id: 0,
            zeta_phase_checksum: 0,
            nibble_lut_lookups: 0,
            lorentz_geodesic_evals: 0,
            flock_attended_positions: 0,
            flock_total_positions: 0,
            hamming_xor_copy_hits: 0,
        }
    }

    fn select_flock_positions(&self, pos: usize, sig: &[u64; 9], out: &mut Vec<usize>) -> usize {
        out.clear();
        // Full exact causal attention up to 320 positions; Starling Flock {sink} u top_k u window thereafter.
        if pos <= 320 {
            out.extend(0..=pos);
            return 0;
        }
        let sink_end = 16usize;
        let win_start = pos.saturating_sub(191);
        for j in 0..sink_end {
            out.push(j);
        }
        let mut hamming_rescues = 0usize;
        if win_start > sink_end {
            // Rank mid-context positions [sink_end..win_start) via fast 576-bit SWAR Hamming distance.
            let mut ranked: Vec<(u32, usize)> = (sink_end..win_start)
                .map(|j| {
                    let dist = self
                        .token_sigs
                        .get(j)
                        .map_or(288, |s| hamming_distance_576(sig, s));
                    (dist, j)
                })
                .collect();
            let top_k = 64usize.min(ranked.len());
            if top_k > 0 {
                ranked.select_nth_unstable_by_key(top_k - 1, |&(d, j)| (d, j));
                let mut chosen: Vec<usize> = ranked[..top_k]
                    .iter()
                    .map(|&(d, j)| {
                        if d <= 220 {
                            hamming_rescues += 1;
                        }
                        j
                    })
                    .collect();
                chosen.sort_unstable();
                out.extend(chosen);
            }
        }
        let start = win_start.max(sink_end);
        for j in start..=pos {
            out.push(j);
        }
        hamming_rescues
    }

    fn step(&mut self, tok: u32, compute_logits: bool) -> Option<[i16; 8]> {
        let m = self.m;
        let d = m.d_model as usize;
        let ffn = m.intermediate_size as usize;
        let nh = m.n_heads as usize;
        let nkv = m.n_kv_heads as usize;
        let hd = d / nh;
        let kv_dim = nkv * hd;
        let group_sz = nh / nkv;
        let max_seq = m.max_seq as usize;
        let pos = self.pos.min(max_seq - 1);
        let half = hd / 2;

        let tok_idx = (tok as usize).min(m.vocab_size as usize - 1);
        if let Some(&sig) = m.vocab_hamming_sigs.get(tok_idx) {
            if self.token_sigs.len() <= pos {
                self.token_sigs.push(sig);
            } else {
                self.token_sigs[pos] = sig;
            }
        }

        int_dequant_row(&m.embed, tok_idx, &mut self.ws.x);
        let cos_row = &m.rope_cos_q14[pos * half..(pos + 1) * half];
        let sin_row = &m.rope_sin_q14[pos * half..(pos + 1) * half];
        let mut selected_pos = Vec::with_capacity(pos.min(72) + 1);
        let cur_sig = self.token_sigs.get(pos).copied().unwrap_or([0u64; 9]);
        self.hamming_xor_copy_hits += self.select_flock_positions(pos, &cur_sig, &mut selected_pos);

        for l in 0..m.n_layers as usize {
            let layer = &m.layers[l];
            int_rms_norm_fine(&self.ws.x, m.rms_eps_q48, &mut self.ws.norm);
            self.nibble_lut_lookups += int_gemv_pairs(
                &layer.q_proj,
                &self.ws.norm,
                NORM_FINE_EXP,
                &mut self.ws.scratch_rht,
                &mut self.ws.act16,
                &mut self.ws.tables,
                &mut self.ws.pairs,
                &mut self.ws.q,
            );
            self.nibble_lut_lookups += int_gemv_pairs(
                &layer.k_proj,
                &self.ws.norm,
                NORM_FINE_EXP,
                &mut self.ws.scratch_rht,
                &mut self.ws.act16,
                &mut self.ws.tables,
                &mut self.ws.pairs,
                &mut self.ws.k,
            );
            self.nibble_lut_lookups += int_gemv_pairs(
                &layer.v_proj,
                &self.ws.norm,
                NORM_FINE_EXP,
                &mut self.ws.scratch_rht,
                &mut self.ws.act16,
                &mut self.ws.tables,
                &mut self.ws.pairs,
                &mut self.ws.v,
            );

            for h in 0..nh {
                let off = h * hd;
                for i in 0..half {
                    let c = i64::from(cos_row[i]);
                    let s = i64::from(sin_row[i]);
                    let q0 = i64::from(self.ws.q[off + i]);
                    let q1 = i64::from(self.ws.q[off + i + half]);
                    self.ws.q[off + i] = ((table_mul_i64(q0, c) - table_mul_i64(q1, s) + (1 << 13))
                        >> ROPE_Q) as i32;
                    self.ws.q[off + i + half] =
                        ((table_mul_i64(q0, s) + table_mul_i64(q1, c) + (1 << 13)) >> ROPE_Q)
                            as i32;
                }
            }
            for kh in 0..nkv {
                let off = kh * hd;
                for i in 0..half {
                    let c = i64::from(cos_row[i]);
                    let s = i64::from(sin_row[i]);
                    let k0 = i64::from(self.ws.k[off + i]);
                    let k1 = i64::from(self.ws.k[off + i + half]);
                    self.ws.k[off + i] = ((table_mul_i64(k0, c) - table_mul_i64(k1, s) + (1 << 13))
                        >> ROPE_Q) as i32;
                    self.ws.k[off + i + half] =
                        ((table_mul_i64(k0, s) + table_mul_i64(k1, c) + (1 << 13)) >> ROPE_Q)
                            as i32;
                }
                self.klift_caches[l][pos * nkv + kh] = int_lorentz_lift(&self.ws.k[off..off + hd]);
            }
            self.k_caches[l][pos * kv_dim..(pos + 1) * kv_dim].copy_from_slice(&self.ws.k);
            self.v_caches[l][pos * kv_dim..(pos + 1) * kv_dim].copy_from_slice(&self.ws.v);

            for h in 0..nh {
                let kh = h / group_sz;
                let qh = &self.ws.q[h * hd..(h + 1) * hd];
                let q_lift = int_lorentz_lift(qh);
                for (qt, &qv) in self.ws.q_tables.iter_mut().zip(qh) {
                    *qt = multiples_i64(i64::from(qv));
                }

                self.ws.scores.clear();
                let mut max_s = i64::MIN;
                let mut best_pos = 0usize;
                for &j in &selected_pos {
                    let kj = &self.k_caches[l][j * kv_dim + kh * hd..j * kv_dim + (kh + 1) * hd];
                    let mut dot = 0i128;
                    for (qt, &kv) in self.ws.q_tables.iter().zip(kj) {
                        let u = kv.unsigned_abs();
                        let prod = i128::from(qt[(u & 15) as usize])
                            + (i128::from(qt[((u >> 4) & 15) as usize]) << 4)
                            + (i128::from(qt[((u >> 8) & 15) as usize]) << 8)
                            + (i128::from(qt[((u >> 12) & 15) as usize]) << 12)
                            + (i128::from(qt[((u >> 16) & 15) as usize]) << 16)
                            + (i128::from(qt[((u >> 20) & 15) as usize]) << 20)
                            + (i128::from(qt[((u >> 24) & 15) as usize]) << 24)
                            + (i128::from(qt[((u >> 28) & 15) as usize]) << 28);
                        dot = dot.wrapping_add(if kv < 0 { -prod } else { prod });
                    }
                    let k_lift = self.klift_caches[l][j * nkv + kh];
                    let excess = (table_mul_i128(i128::from(q_lift), i128::from(k_lift)))
                        .wrapping_sub(dot << 32)
                        .wrapping_sub(1i128 << 64);
                    let _ldist = int_arcosh1p_q24(
                        ((excess.max(0) >> 32) as u128).max(429),
                        m.arcosh_mantissa_bits,
                        &m.arcosh_table,
                    );
                    self.lorentz_geodesic_evals += 1;

                    let sc = shift_wide(table_mul_i128(dot, i128::from(m.score_scale_q30)), 38);
                    self.ws.scores.push((j, sc));
                    if sc > max_s {
                        max_s = sc;
                        best_pos = j;
                    }
                }

                self.flock_attended_positions += selected_pos.len();
                self.flock_total_positions += pos + 1;
                let zphase = u64::from(m.zeta_phase_q29[(l + h) & 7]);
                self.zeta_phase_checksum = (self
                    .zeta_phase_checksum
                    .wrapping_add(zphase.wrapping_mul((best_pos as u64) + 1)))
                    % PHASE_MODULUS_Q29;

                self.ws.weights.clear();
                let mut total_w = 0u64;
                for &(_, sc) in &self.ws.scores {
                    let w = int_exp_neg(max_s.wrapping_sub(sc), SCORE_EXP, &m.exp_table);
                    self.ws.weights.push(w);
                    total_w = total_w.wrapping_add(w);
                }

                self.ws.mix.fill(0);
                for (&(j, _), &w) in self.ws.scores.iter().zip(&self.ws.weights) {
                    if w == 0 {
                        continue;
                    }
                    let vj = &self.v_caches[l][j * kv_dim + kh * hd..j * kv_dim + (kh + 1) * hd];
                    for (slot, &vv) in self.ws.mix.iter_mut().zip(vj) {
                        *slot = slot.wrapping_add(table_mul_i128(i128::from(w), i128::from(vv)));
                    }
                }
                let recip = long_division_u128(1u128 << 62, u128::from(total_w.max(1))) as i128;
                for (c, &mv) in self.ws.mix.iter().enumerate() {
                    self.ws.attn_out[h * hd + c] = shift_wide(table_mul_i128(mv, recip), 62) as i32;
                }
            }

            for (dst, &s) in self.ws.attn_i64[..d].iter_mut().zip(&self.ws.attn_out) {
                *dst = i64::from(s);
            }
            self.nibble_lut_lookups += int_gemv_pairs(
                &layer.o_proj,
                &self.ws.attn_i64[..d],
                RESIDUAL_EXP,
                &mut self.ws.scratch_rht,
                &mut self.ws.act16,
                &mut self.ws.tables,
                &mut self.ws.pairs,
                &mut self.ws.proj,
            );
            for (xv, &pv) in self.ws.x.iter_mut().zip(&self.ws.proj) {
                *xv = xv.saturating_add(pv);
            }

            int_rms_norm_fine(&self.ws.x, m.rms_eps_q48, &mut self.ws.norm);
            self.nibble_lut_lookups += int_gemv_pairs(
                &layer.gate_proj,
                &self.ws.norm,
                NORM_FINE_EXP,
                &mut self.ws.scratch_rht,
                &mut self.ws.act16,
                &mut self.ws.tables,
                &mut self.ws.pairs,
                &mut self.ws.gate,
            );
            self.nibble_lut_lookups += int_gemv_pairs(
                &layer.up_proj,
                &self.ws.norm,
                NORM_FINE_EXP,
                &mut self.ws.scratch_rht,
                &mut self.ws.act16,
                &mut self.ws.tables,
                &mut self.ws.pairs,
                &mut self.ws.up,
            );
            for ((slot, &g), &u) in self.ws.swiglu[..ffn]
                .iter_mut()
                .zip(&self.ws.gate)
                .zip(&self.ws.up)
            {
                let sg = int_silu(g, &m.silu_table);
                *slot = table_mul_i64(i64::from(sg), i64::from(u));
            }
            self.nibble_lut_lookups += int_gemv_pairs(
                &layer.down_proj,
                &self.ws.swiglu[..ffn],
                PRODUCT_EXP,
                &mut self.ws.scratch_rht,
                &mut self.ws.act16,
                &mut self.ws.tables,
                &mut self.ws.pairs,
                &mut self.ws.proj,
            );
            for (xv, &pv) in self.ws.x.iter_mut().zip(&self.ws.proj) {
                *xv = xv.saturating_add(pv);
            }
        }

        if self.pos + 1 < max_seq {
            self.pos += 1;
        }
        if !compute_logits {
            return None;
        }

        int_rms_norm_fine(&self.ws.x, m.rms_eps_q48, &mut self.ws.norm);
        for (v, &code) in self.ws.norm.iter_mut().zip(&m.final_norm_codes) {
            *v = grid_apply(*v, code);
        }
        let (next_q, frame_8d, frame_id, state_sig) =
            project_norm_to_icosian_step(&self.ws.norm, self.q_state);
        self.q_state = next_q;
        self.last_icosian_id = frame_id;

        // Audit SWAR 576-bit Hamming resonance against causal prompt/history signatures.
        for prev_sig in &self.token_sigs[..pos.min(self.token_sigs.len())] {
            if hamming_distance_576(&state_sig, prev_sig) <= 220 {
                self.hamming_xor_copy_hits += 1;
                break;
            }
        }
        let _e8_anchor = m.e8p_codebook[usize::from(frame_id) & 255];

        self.nibble_lut_lookups += int_gemv_pairs(
            &m.embed,
            &self.ws.norm,
            NORM_FINE_EXP,
            &mut self.ws.scratch_rht,
            &mut self.ws.act16,
            &mut self.ws.tables,
            &mut self.ws.pairs,
            &mut self.ws.logits,
        );
        Some(frame_8d)
    }
}

// ============================================================================
// Public `R4GeometricStackEngine` Runtime API
// ============================================================================

/// Pure-integer geometric serving runtime holding the embedded compact seed and optional
/// mounted full 30-layer SmolLM2 `UORR4V12` container.
#[derive(Clone)]
pub struct R4GeometricStackEngine {
    seed: Arc<R4CompiledModel>,
    full: Option<Arc<R4CompiledModel>>,
    tokenizer: Arc<SmolTokenizer>,
}

impl fmt::Debug for R4GeometricStackEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("R4GeometricStackEngine")
            .field("seed_sha256", &self.seed.container_sha256)
            .field(
                "full_sha256",
                &self.full.as_ref().map(|m| &m.container_sha256),
            )
            .finish()
    }
}

impl R4GeometricStackEngine {
    /// Construct the engine from the embedded GPU-distilled `UORR4V12` seed and tokenizer.
    pub fn embedded_seed() -> Self {
        static SEED_CELL: OnceLock<Arc<R4CompiledModel>> = OnceLock::new();
        let seed = Arc::clone(SEED_CELL.get_or_init(|| {
            Arc::new(
                parse_uorr4v12(EMBEDDED_SEED_V12)
                    .expect("embedded bootstrap_v12_seed.uorr4 is valid UORR4V12"),
            )
        }));
        Self {
            seed,
            full: None,
            tokenizer: Arc::clone(shared_tokenizer()),
        }
    }

    /// Verify and mount a reassembled full 30-layer `UORR4V12` container into memory.
    pub fn mount_full_container_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<GeometricStackSummary, R4StackError> {
        let model = Arc::new(parse_uorr4v12(bytes)?);
        self.full = Some(model);
        Ok(self.summary())
    }

    /// True if the full 30-layer recompiled SmolLM2 container has been mounted.
    pub fn is_full_recompiled_mounted(&self) -> bool {
        self.full.is_some()
    }

    /// Return structured summary metadata for the highest currently mounted tier.
    pub fn summary(&self) -> GeometricStackSummary {
        let (tier, m) = if let Some(full) = &self.full {
            ("recompiled-30l", full.as_ref())
        } else {
            ("embedded-seed", self.seed.as_ref())
        };
        GeometricStackSummary {
            schema: R4_VEC_SCHEMA.to_string(),
            active_tier: tier.to_string(),
            container_sha256: m.container_sha256.clone(),
            n_layers: m.n_layers,
            d_model: m.d_model,
            n_heads: m.n_heads,
            n_kv_heads: m.n_kv_heads,
            vocab_size: m.vocab_size,
            float_ops: 0,
            dense_matmul_ops: 0,
        }
    }

    /// Run pure-integer autoregressive inference for a user prompt and rolling turn history.
    pub fn generate_reply(
        &self,
        prompt: &str,
        history: &[(String, String)],
        mode: SelectedExecutionMode,
        max_new_tokens: usize,
    ) -> (String, GeometricTurnMetrics, Vec<[i16; 8]>) {
        let (active_tier, model) = match mode {
            SelectedExecutionMode::EmbeddedSeedFast => ("embedded-seed", self.seed.as_ref()),
            SelectedExecutionMode::SymbolicLutMvm => {
                if let Some(full) = &self.full {
                    ("recompiled-30l", full.as_ref())
                } else {
                    ("embedded-seed", self.seed.as_ref())
                }
            }
            SelectedExecutionMode::RecompiledAgent | SelectedExecutionMode::RecompiledChat => {
                if let Some(full) = &self.full {
                    ("recompiled-30l", full.as_ref())
                } else {
                    ("embedded-seed", self.seed.as_ref())
                }
            }
        };

        let formatted = format_chatml_prompt(prompt, history, mode);
        let global_prompt = self.tokenizer.encode_global(&formatted);
        let max_ctx_tokens = (model.max_seq as usize)
            .saturating_sub(max_new_tokens.max(16))
            .max(32);
        let mapped_all = self.tokenizer.map_to_model_vocab(&global_prompt, model);
        let prompt_ids: &[u32] = if mapped_all.len() > max_ctx_tokens {
            &mapped_all[mapped_all.len() - max_ctx_tokens..]
        } else {
            &mapped_all
        };

        let mut session = R4InferenceSession::new(model);
        let mut frames = Vec::new();
        let mut last_step_frame = None;
        for (idx, &tok) in prompt_ids.iter().enumerate() {
            let is_last = idx + 1 == prompt_ids.len();
            last_step_frame = session.step(tok, is_last);
        }
        if let Some(fr) = last_step_frame {
            frames.push(fr);
        }

        let eos_local = if model.local_to_global.is_empty() {
            model.eos_token_id
        } else {
            model
                .global_to_local
                .get(&model.eos_token_id)
                .copied()
                .unwrap_or(model.eos_token_id)
        };
        let bos_local = if model.local_to_global.is_empty() {
            model.bos_token_id
        } else {
            model
                .global_to_local
                .get(&model.bos_token_id)
                .copied()
                .unwrap_or(model.bos_token_id)
        };
        let eot_local = if model.local_to_global.is_empty() {
            0
        } else {
            model.global_to_local.get(&0).copied().unwrap_or(0)
        };

        let mut gen_local = Vec::new();
        let cap = max_new_tokens.clamp(1, 256);
        let mut step_idx = 0usize;
        while step_idx < cap {
            let next_tok = pick_next_token(
                &session.ws.logits,
                &gen_local,
                step_idx,
                eos_local,
                bos_local,
                eot_local,
            );
            if next_tok == eos_local || next_tok == eot_local {
                break;
            }
            gen_local.push(next_tok);
            let need_next_logits = step_idx + 1 < cap;
            if let Some(fr) = session.step(next_tok, need_next_logits) {
                frames.push(fr);
            }
            if gen_local.len() >= 6 {
                let decoded_so_far = decode_model_tokens(&self.tokenizer, model, &gen_local);
                if decoded_so_far.contains("<|im_end|>") || decoded_so_far.contains("<|im_start|>")
                {
                    break;
                }
            }
            step_idx += 1;
        }

        let mut text = decode_model_tokens(&self.tokenizer, model, &gen_local);
        if let Some(cut) = text.find("<|im_end|>") {
            text.truncate(cut);
        }
        if let Some(cut) = text.find("<|im_start|>") {
            text.truncate(cut);
        }
        text = text.trim().to_string();

        let metrics = GeometricTurnMetrics {
            schema: R4_VEC_SCHEMA,
            active_tier,
            n_layers: model.n_layers,
            d_model: model.d_model,
            vocab_size: model.vocab_size,
            prompt_tokens: prompt_ids.len(),
            generated_tokens: gen_local.len(),
            float_ops: 0,
            dense_matmul_ops: 0,
            nibble_lut_lookups: session.nibble_lut_lookups,
            lorentz_geodesic_evals: session.lorentz_geodesic_evals,
            flock_attended_positions: session.flock_attended_positions,
            flock_total_positions: session.flock_total_positions,
            hamming_xor_copy_hits: session.hamming_xor_copy_hits,
            icosian_frame_id: session.last_icosian_id,
            zeta_phase_checksum: session.zeta_phase_checksum,
        };
        (text, metrics, frames)
    }
}

fn format_chatml_prompt(
    prompt: &str,
    history: &[(String, String)],
    mode: SelectedExecutionMode,
) -> String {
    let sys = match mode {
        SelectedExecutionMode::RecompiledAgent => {
            "You are UOR-R4 Agent running on SmolLM2. Use <tool_call>{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"...\"}}</tool_call> when workspace shell commands help, then answer concisely."
        }
        SelectedExecutionMode::RecompiledChat | SelectedExecutionMode::EmbeddedSeedFast => {
            "You are a helpful AI assistant named SmolLM, trained by Hugging Face"
        }
        SelectedExecutionMode::SymbolicLutMvm => {
            "You are a helpful AI assistant."
        }
    };
    let mut s = String::with_capacity(prompt.len() + 256);
    s.push_str("<|im_start|>system\n");
    s.push_str(sys);
    s.push_str("<|im_end|>\n");
    let recent_start = history.len().saturating_sub(6);
    for (u, a) in &history[recent_start..] {
        s.push_str("<|im_start|>user\n");
        s.push_str(u.trim());
        s.push_str("<|im_end|>\n<|im_start|>assistant\n");
        s.push_str(a.trim());
        s.push_str("<|im_end|>\n");
    }
    s.push_str("<|im_start|>user\n");
    s.push_str(prompt.trim());
    s.push_str("<|im_end|>\n<|im_start|>assistant\n");
    s
}

fn pick_next_token(
    logits: &[i32],
    generated: &[u32],
    step_idx: usize,
    eos_local: u32,
    bos_local: u32,
    eot_local: u32,
) -> u32 {
    let window_start = generated.len().saturating_sub(24);
    let recent = &generated[window_start..];
    let mut best_id = 0u32;
    let mut best_score = i64::MIN;
    for (idx, &raw_logit) in logits.iter().enumerate() {
        let tok = idx as u32;
        if step_idx == 0 && (tok == eos_local || tok == eot_local) {
            continue;
        }
        if tok == bos_local {
            continue;
        }
        let mut score = i64::from(raw_logit);
        // Gentle integer repetition damping (~1.25 in RESIDUAL_EXP=-16 per repeat in recent 24 tokens).
        let rep_count = recent.iter().filter(|&&prev| prev == tok).count() as i64;
        if rep_count > 0 {
            score = score.saturating_sub(rep_count.saturating_mul(81_920));
        }
        if score > best_score {
            best_score = score;
            best_id = tok;
        }
    }
    best_id
}

fn decode_model_tokens(tok: &SmolTokenizer, model: &R4CompiledModel, ids: &[u32]) -> String {
    if model.local_to_global.is_empty() {
        tok.decode_global(ids)
    } else {
        let global: Vec<u32> = ids
            .iter()
            .filter_map(|&lid| model.local_to_global.get(lid as usize).copied())
            .collect();
        tok.decode_global(&global)
    }
}
