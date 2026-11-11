//! `UOR-R4-GLLM v9` (`UORGLLM9`) Compositional Geometric Language Model Runtime (`R4-38`).
//!
//! Executes pure-integer, matmul-free, softmax-free autoregressive next-token prediction
//! with exact bit-for-bit parity against `gpu/train_gllm_v9.cu`:
//! - **Zero `ServedTurnMemory`** (`turn_memory_entries == 0` strictly enforced by `R4.CompositionalGllm`),
//! - **Zero dense `matmul` / `GEMM`** (`Cl(0,6)` `FWHT-64` integer butterflies + 256-entry discrete T-MAC Q12 LUTs
//!   + 81-state packed-ternary T-MAC activation lookup readout),
//! - **Zero `softmax` / `exp`** (rational integer PRLL `R4-27` mass conservation `sum == 1_000_000`),
//! - **Multi-layer causal KV cache + context-gated causal anchor/induction pointer-copy** enabling genuine
//!   in-context comprehension of unseen variable names, file paths, code, session context,
//!   and live `MVM-Wasm` (`R4-37`) `<tool_response>` observations.

use std::sync::{Arc, OnceLock};

use sha2::{Digest, Sha256};

use crate::lexlean_generated::r4_compositional_gllm::{
    admit_causal_kv_step, admit_gllm_container, admit_tmac_lut_read, ContainerSummary,
    ContainerVerdict, KvAttentionCheck, KvAttentionVerdict, TmacLutReadCheck, TmacLutVerdict,
};
use crate::lut_stack_exec::ByteBpe4096;

/// Eight-byte magic header for `UOR-R4-GLLM v9` binary weight containers.
pub const UORGLLM9_MAGIC: [u8; 8] = *b"UORGLLM9";
/// Active `UORGLLM9` container format version.
pub const UORGLLM9_VERSION: u32 = 9;
/// Fixed vocabulary size (`ByteBpe4096`).
pub const GLLM_V9_VOCAB: usize = 4096;
/// Output channels per 4-channel LUT group in `Q_lut`, `K_lut`, and `V_lut`.
pub const GLLM_V9_QKV_OUT: usize = 16;
/// Rational PRLL (`R4-27`) probability mass quantum (`1_000_000`).
pub const GLLM_V9_PRLL_QUANTUM: u64 = 1_000_000;
/// Maximum causal KV sequence window per inference turn.
pub const GLLM_V9_MAX_SEQ: usize = 512;

/// Embedded 6-layer 64D `UORGLLM9` bootstrap checkpoint trained on RTX 5090 (`gpu/train_gllm_v9.cu`).
static EMBEDDED_BOOTSTRAP_V9: &[u8] = include_bytes!("bootstrap_v9.uorgllm");

/// Typed errors emitted by the `UOR-R4-GLLM v9` parser and runtime (`R5`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GllmV9Error {
    /// Container bytes are shorter than the 80-byte `UORGLLM9` header or expected tensor payload.
    TruncatedContainer,
    /// Container header was refused by `R4.CompositionalGllm.admitGllmContainer`.
    ContainerRefused(ContainerVerdict),
    /// Payload SHA-256 did not match the 32-byte header digest.
    IntegrityMismatch,
    /// Causal KV cache step violated `R4.CompositionalGllm.admitCausalKvStep`.
    CausalKvRefused(KvAttentionVerdict),
    /// Discrete T-MAC LUT read violated `R4.CompositionalGllm.admitTmacLutRead`.
    LutReadRefused(TmacLutVerdict),
}

impl std::fmt::Display for GllmV9Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TruncatedContainer => write!(f, "truncated UORGLLM9 container"),
            Self::ContainerRefused(v) => write!(f, "UORGLLM9 container refused: {v:?}"),
            Self::IntegrityMismatch => write!(f, "UORGLLM9 payload SHA-256 mismatch"),
            Self::CausalKvRefused(v) => write!(f, "causal KV step refused: {v:?}"),
            Self::LutReadRefused(v) => write!(f, "T-MAC LUT read refused: {v:?}"),
        }
    }
}

impl std::error::Error for GllmV9Error {}

/// Parsed 80-byte `UORGLLM9` binary container header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GllmV9Header {
    /// Magic bytes (`*b"UORGLLM9"`).
    pub magic: [u8; 8],
    /// Container version (`9`).
    pub version: u32,
    /// ByteBPE vocabulary cardinality (`4096`).
    pub vocab_size: u32,
    /// Hidden channel width (`64` or `128`, multiple of `64`).
    pub d_model: u32,
    /// Number of stacked causal geometric layers (`6` or `8`).
    pub n_layers: u32,
    /// Number of 8D $E_8$ causal attention heads (`d_model / 8`).
    pub n_heads: u32,
    /// Number of 4-channel subvector LUT groups (`d_model / 4`).
    pub lut_groups: u32,
    /// Canned turn-memory entry count (strictly `0` in `v9`).
    pub turn_memory_entries: u32,
    /// Dense matrix multiplication count (strictly `0`).
    pub dense_matmul_ops: u32,
    /// Exponential/softmax operation count (strictly `0`).
    pub softmax_ops: u32,
    /// Stored tensor quantization bit-width (`8`).
    pub quant_bits: u32,
    /// FIPS 180-4 SHA-256 digest of `bytes[80..]`.
    pub payload_sha256: [u8; 32],
}

#[derive(Clone, Debug)]
struct LayerTables {
    head_tau: Vec<u16>,
    head_slope: Vec<u16>,
    head_shift: Vec<u8>,
    head_copy_b: Vec<i16>,
    head_copy_gate_trits: Vec<u8>, // [n_heads * lut_groups] packed 4-trit codes 0..80
    q_lut: Vec<i16>,               // [lut_groups * 256 * 16] dequantized Q12
    k_lut: Vec<i16>,               // [lut_groups * 256 * 16] dequantized Q12
    v_lut: Vec<i16>,               // [lut_groups * 256 * 16] dequantized Q12
    ffn_lut: Vec<i16>,             // [lut_groups * 256 * d_model] dequantized Q12
}

/// Telemetry for one autoregressively predicted token step in `UOR-R4-GLLM v9`.
#[derive(Debug, Clone)]
pub struct GllmV9TokenStep {
    /// Predicted `ByteBpe4096` token ID.
    pub token_id: u32,
    /// Causal query position `t` in the KV cache.
    pub query_pos: usize,
    /// Highest-mass attended causal KV position (`<= query_pos`).
    pub top_attended_pos: usize,
    /// Winning total integer score (T-MAC LUT + prototype + causal KV pointer-copy).
    pub winning_score: i32,
    /// Contribution from context-gated causal KV pointer-copy heads.
    pub copy_score: i32,
}

/// Full trace and output of compositional autoregressive generation (`R4-38`).
#[derive(Debug, Clone)]
pub struct GllmV9Generation {
    /// Decoded UTF-8 completion text.
    pub text: String,
    /// Prompt token count admitted into the causal KV cache.
    pub kv_prompt_tokens: usize,
    /// Generated token IDs (excluding `<|eos|>`).
    pub generated_tokens: Vec<u32>,
    /// Number of causal geometric layers executed per token.
    pub layers_executed: usize,
    /// Count of `ServedTurnMemory` hits (always `0` in `UORGLLM9`).
    pub turn_memory_hits: usize,
    /// Whether every causal head conserved exact `PRLL_QUANTUM = 1_000_000` integer mass.
    pub prll_mass_conserved: bool,
    /// Per-token step telemetry.
    pub steps: Vec<GllmV9TokenStep>,
    /// SHA-256 hex digest of the active `UORGLLM9` container.
    pub model_sha256: String,
}

/// Dequantized, ready-to-execute `UOR-R4-GLLM v9` compositional model (`UORGLLM9`).
#[derive(Clone)]
pub struct GllmV9Model {
    header: GllmV9Header,
    raw_bytes: Vec<u8>,
    container_sha256: String,
    source_label: String,
    e_tok: Vec<i16>, // [4096 * d_model]
    e_pos: Vec<i16>, // [256 * d_model]
    layers: Vec<LayerTables>,
    tmac_readout_hi: Vec<u8>, // [4096 * lut_groups] packed 4-trit hi plane 0..80
    tmac_readout_lo: Vec<u8>, // [4096 * lut_groups] packed 4-trit lo plane 0..80
    copy_tok_gate: Vec<u8>,   // [4096] per-token copy eligibility 0..255
    p_out: Vec<i16>,          // [4096 * d_model]
    p_sk0: Vec<u64>,          // [4096]
    p_sk1: Vec<u64>,          // [4096]
    b_out: Vec<i16>,          // [4096]
}

impl std::fmt::Debug for GllmV9Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GllmV9Model")
            .field("header", &self.header)
            .field("container_sha256", &self.container_sha256)
            .field("source_label", &self.source_label)
            .finish()
    }
}

#[inline]
fn clamp_q12(v: i32) -> i16 {
    v.clamp(-8192, 8192) as i16
}

#[inline]
fn dequant_i8_q12(q: i8, scale_q8: u16) -> i16 {
    let sc = (scale_q8.max(16)) as i32;
    clamp_q12(((q as i32) * sc) >> 4)
}

#[inline]
fn q2_level(v: i16) -> u8 {
    let x = v as i32;
    if x < -1024 {
        0
    } else if x < 0 {
        1
    } else if x < 1024 {
        2
    } else {
        3
    }
}

#[inline]
fn code_4ch(u4: &[i16]) -> u8 {
    q2_level(u4[0]) | (q2_level(u4[1]) << 2) | (q2_level(u4[2]) << 4) | (q2_level(u4[3]) << 6)
}

/// Pure addition/subtraction construction of 81-entry (`3^4`) T-MAC ternary activation LUT (ZERO multiplications).
#[inline]
fn build_tmac_act_lut81(x4: &[i16], out81: &mut [i32; 81]) {
    let a0 = [-(x4[0] as i32), 0, x4[0] as i32];
    let a1 = [-(x4[1] as i32), 0, x4[1] as i32];
    let a2 = [-(x4[2] as i32), 0, x4[2] as i32];
    let a3 = [-(x4[3] as i32), 0, x4[3] as i32];
    let mut idx = 0usize;
    for &v3 in &a3 {
        for &v2 in &a2 {
            let s23 = v3 + v2;
            for &v1 in &a1 {
                let s123 = s23 + v1;
                out81[idx] = (s123 + a0[0]) >> 5;
                out81[idx + 1] = (s123 + a0[1]) >> 5;
                out81[idx + 2] = (s123 + a0[2]) >> 5;
                idx += 3;
            }
        }
    }
}

#[inline]
fn icosian_hinge(v: i32) -> i16 {
    let y = if v < -2048 { (v >> 2) - 1536 } else { v };
    clamp_q12(y)
}

#[inline]
fn prll_weight_order4(tau: i32, d_tot: i32) -> u64 {
    let denom = ((tau.max(256)) as u64) + ((d_tot.max(0) as u64) >> 1);
    let q1 = 1_073_741_824u64 / denom.max(1);
    let q2 = (q1 * q1) >> 20;
    ((q2 * q2) >> 16) + 1
}

#[inline]
fn pack_sign64(v64: &[i16]) -> u64 {
    let mut bits = 0u64;
    for (i, &val) in v64.iter().take(64).enumerate() {
        if val >= 0 {
            bits |= 1u64 << i;
        }
    }
    bits
}

/// Exact 64-channel Fast Walsh-Hadamard Transform (`Cl(0,6)` spin butterfly, `1/sqrt(64) = >> 3`).
pub fn fwht64_block(in64: &[i16], out64: &mut [i16]) {
    let mut u = [0i32; 64];
    for i in 0..64 {
        u[i] = in64[i] as i32;
    }
    let mut len = 1usize;
    while len < 64 {
        let step = len << 1;
        let mut i = 0usize;
        while i < 64 {
            for j in 0..len {
                let a = u[i + j];
                let b = u[i + j + len];
                u[i + j] = a + b;
                u[i + j + len] = a - b;
            }
            i += step;
        }
        len <<= 1;
    }
    for i in 0..64 {
        out64[i] = clamp_q12(u[i] >> 3);
    }
}

fn sha256_bytes_arr(bytes: &[u8]) -> [u8; 32] {
    let d = Sha256::digest(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&d);
    out
}

fn hex_32(arr: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in arr {
        use std::fmt::Write;
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}

fn read_u16_le(bytes: &[u8], cur: &mut usize) -> Result<u16, GllmV9Error> {
    if *cur + 2 > bytes.len() {
        return Err(GllmV9Error::TruncatedContainer);
    }
    let v = u16::from_le_bytes([bytes[*cur], bytes[*cur + 1]]);
    *cur += 2;
    Ok(v)
}

fn read_u32_le(bytes: &[u8], cur: &mut usize) -> Result<u32, GllmV9Error> {
    if *cur + 4 > bytes.len() {
        return Err(GllmV9Error::TruncatedContainer);
    }
    let v = u32::from_le_bytes([
        bytes[*cur],
        bytes[*cur + 1],
        bytes[*cur + 2],
        bytes[*cur + 3],
    ]);
    *cur += 4;
    Ok(v)
}

impl GllmV9Model {
    /// Obtain the shared embedded 6-layer `bootstrap_v9.uorgllm` compositional model.
    pub fn default_embedded() -> Arc<Self> {
        static CELL: OnceLock<Arc<GllmV9Model>> = OnceLock::new();
        Arc::clone(CELL.get_or_init(|| {
            Arc::new(
                Self::parse_with_label(
                    EMBEDDED_BOOTSTRAP_V9,
                    "bootstrap_v9.uorgllm (WASM embedded)",
                )
                .expect("embedded bootstrap_v9.uorgllm is valid UORGLLM9"),
            )
        }))
    }

    /// Parse and formally verify a `UORGLLM9` binary container.
    pub fn parse(bytes: &[u8]) -> Result<Self, GllmV9Error> {
        Self::parse_with_label(bytes, "UORGLLM9 container")
    }

    /// Parse and formally verify a `UORGLLM9` container with a human-readable provenance label.
    pub fn parse_with_label(bytes: &[u8], label: &str) -> Result<Self, GllmV9Error> {
        if bytes.len() < 80 {
            return Err(GllmV9Error::TruncatedContainer);
        }
        let mut magic = [0u8; 8];
        magic.copy_from_slice(&bytes[0..8]);
        let mut cur = 8usize;
        let version = read_u32_le(bytes, &mut cur)?;
        let vocab_size = read_u32_le(bytes, &mut cur)?;
        let d_model = read_u32_le(bytes, &mut cur)?;
        let n_layers = read_u32_le(bytes, &mut cur)?;
        let n_heads = read_u32_le(bytes, &mut cur)?;
        let lut_groups = read_u32_le(bytes, &mut cur)?;
        let turn_memory_entries = read_u32_le(bytes, &mut cur)?;
        let dense_matmul_ops = read_u32_le(bytes, &mut cur)?;
        let softmax_ops = read_u32_le(bytes, &mut cur)?;
        let quant_bits = read_u32_le(bytes, &mut cur)?;
        let mut payload_sha256 = [0u8; 32];
        payload_sha256.copy_from_slice(&bytes[48..80]);
        cur = 80;

        let hdr = GllmV9Header {
            magic,
            version,
            vocab_size,
            d_model,
            n_layers,
            n_heads,
            lut_groups,
            turn_memory_entries,
            dense_matmul_ops,
            softmax_ops,
            quant_bits,
            payload_sha256,
        };

        let verdict = Self::check_header_summary(&hdr);
        if verdict != ContainerVerdict::ContainerAdmitted {
            return Err(GllmV9Error::ContainerRefused(verdict));
        }

        let actual_payload_sha = sha256_bytes_arr(&bytes[80..]);
        if actual_payload_sha != payload_sha256 {
            return Err(GllmV9Error::IntegrityMismatch);
        }

        let d = d_model as usize;
        let l_cnt = n_layers as usize;
        let h_cnt = n_heads as usize;
        let g_cnt = lut_groups as usize;

        let n_scales = 3 + 4 * l_cnt;
        let mut scales = Vec::with_capacity(n_scales);
        for _ in 0..n_scales {
            scales.push(read_u16_le(bytes, &mut cur)?);
        }

        let deq_slice =
            |bytes: &[u8], cur: &mut usize, len: usize, sc: u16| -> Result<Vec<i16>, GllmV9Error> {
                if *cur + len > bytes.len() {
                    return Err(GllmV9Error::TruncatedContainer);
                }
                let mut out = Vec::with_capacity(len);
                for &b in &bytes[*cur..*cur + len] {
                    out.push(dequant_i8_q12(b as i8, sc));
                }
                *cur += len;
                Ok(out)
            };

        let read_raw_slice =
            |bytes: &[u8], cur: &mut usize, len: usize| -> Result<Vec<u8>, GllmV9Error> {
                if *cur + len > bytes.len() {
                    return Err(GllmV9Error::TruncatedContainer);
                }
                let out = bytes[*cur..*cur + len].to_vec();
                *cur += len;
                Ok(out)
            };

        let e_tok = deq_slice(bytes, &mut cur, GLLM_V9_VOCAB * d, scales[0])?;
        let e_pos = deq_slice(bytes, &mut cur, 256 * d, scales[1])?;

        let mut layers = Vec::with_capacity(l_cnt);
        for l in 0..l_cnt {
            let mut head_tau = Vec::with_capacity(h_cnt);
            for _ in 0..h_cnt {
                head_tau.push(read_u16_le(bytes, &mut cur)?);
            }
            let mut head_slope = Vec::with_capacity(h_cnt);
            for _ in 0..h_cnt {
                head_slope.push(read_u16_le(bytes, &mut cur)?);
            }
            let head_shift = read_raw_slice(bytes, &mut cur, h_cnt)?;
            let mut head_copy_b = Vec::with_capacity(h_cnt);
            for _ in 0..h_cnt {
                head_copy_b.push(read_u16_le(bytes, &mut cur)? as i16);
            }
            let head_copy_gate_trits = read_raw_slice(bytes, &mut cur, h_cnt * g_cnt)?;
            if head_copy_gate_trits.iter().any(|&c| c >= 81) {
                return Err(GllmV9Error::TruncatedContainer);
            }

            let qkv_len = g_cnt * 256 * GLLM_V9_QKV_OUT;
            let ffn_len = g_cnt * 256 * d;
            let q_lut = deq_slice(bytes, &mut cur, qkv_len, scales[3 + 4 * l])?;
            let k_lut = deq_slice(bytes, &mut cur, qkv_len, scales[3 + 4 * l + 1])?;
            let v_lut = deq_slice(bytes, &mut cur, qkv_len, scales[3 + 4 * l + 2])?;
            let ffn_lut = deq_slice(bytes, &mut cur, ffn_len, scales[3 + 4 * l + 3])?;

            layers.push(LayerTables {
                head_tau,
                head_slope,
                head_shift,
                head_copy_b,
                head_copy_gate_trits,
                q_lut,
                k_lut,
                v_lut,
                ffn_lut,
            });
        }

        let tmac_readout_hi = read_raw_slice(bytes, &mut cur, GLLM_V9_VOCAB * g_cnt)?;
        if tmac_readout_hi.iter().any(|&c| c >= 81) {
            return Err(GllmV9Error::TruncatedContainer);
        }
        let tmac_readout_lo = read_raw_slice(bytes, &mut cur, GLLM_V9_VOCAB * g_cnt)?;
        if tmac_readout_lo.iter().any(|&c| c >= 81) {
            return Err(GllmV9Error::TruncatedContainer);
        }
        let copy_tok_gate = read_raw_slice(bytes, &mut cur, GLLM_V9_VOCAB)?;
        let p_out = deq_slice(bytes, &mut cur, GLLM_V9_VOCAB * d, scales[2])?;
        let mut b_out = Vec::with_capacity(GLLM_V9_VOCAB);
        for _ in 0..GLLM_V9_VOCAB {
            b_out.push(read_u16_le(bytes, &mut cur)? as i16);
        }
        if cur != bytes.len() {
            return Err(GllmV9Error::TruncatedContainer);
        }

        let mut p_sk0 = Vec::with_capacity(GLLM_V9_VOCAB);
        let mut p_sk1 = Vec::with_capacity(GLLM_V9_VOCAB);
        for v in 0..GLLM_V9_VOCAB {
            let row = &p_out[v * d..(v + 1) * d];
            p_sk0.push(pack_sign64(&row[0..64]));
            p_sk1.push(if d >= 128 {
                pack_sign64(&row[64..128])
            } else {
                0
            });
        }

        let full_sha = hex_32(&sha256_bytes_arr(bytes));
        Ok(Self {
            header: hdr,
            raw_bytes: bytes.to_vec(),
            container_sha256: full_sha,
            source_label: label.to_string(),
            e_tok,
            e_pos,
            layers,
            tmac_readout_hi,
            tmac_readout_lo,
            copy_tok_gate,
            p_out,
            p_sk0,
            p_sk1,
            b_out,
        })
    }

    fn check_header_summary(hdr: &GllmV9Header) -> ContainerVerdict {
        let d = hdr.d_model as usize;
        let spin_aligned = d >= 64
            && d.is_multiple_of(64)
            && (hdr.n_heads as usize) * 8 == d
            && (hdr.lut_groups as usize) * 4 == d;
        admit_gllm_container(ContainerSummary {
            magic_valid: hdr.magic == UORGLLM9_MAGIC,
            version_code: hdr.version as u64,
            vocab_size: hdr.vocab_size as u64,
            d_model: hdr.d_model as u64,
            spin_aligned,
            n_layers: hdr.n_layers as u64,
            n_heads: hdr.n_heads as u64,
            turn_memory_entries: hdr.turn_memory_entries as u64,
            dense_matmul_ops: hdr.dense_matmul_ops as u64,
            softmax_ops: hdr.softmax_ops as u64,
        })
    }

    /// Return the verified `UORGLLM9` header.
    pub fn header(&self) -> &GllmV9Header {
        &self.header
    }

    /// Evaluate the formal `R4.CompositionalGllm.admitGllmContainer` predicate on this model.
    pub fn admit_container(&self) -> ContainerVerdict {
        Self::check_header_summary(&self.header)
    }

    /// Export raw `UORGLLM9` container bytes.
    pub fn to_container_bytes(&self) -> Vec<u8> {
        self.raw_bytes.clone()
    }

    /// Active provenance source label (`bootstrap_v9` or OPFS pack).
    pub fn source_label(&self) -> &str {
        &self.source_label
    }

    /// Container FIPS 180-4 SHA-256 hex digest.
    pub fn container_sha256(&self) -> &str {
        &self.container_sha256
    }

    /// Execute full compositional autoregressive generation over `prompt` and `history` (`R4-38`).
    pub fn generate_compositional(
        &self,
        prompt: &str,
        history: &[(String, String)],
        max_new_tokens: usize,
    ) -> GllmV9Generation {
        let bpe = ByteBpe4096::global();
        let mut prefix_ids: Vec<u32> = vec![bpe.bos_id()];

        // Include recent session history inside the live causal KV cache only when the
        // prompt references prior conversational state, preventing standalone coding or
        // sandbox turns from inheriting unrelated boot/assistant tokens.
        let lower_prompt = prompt.trim().to_ascii_lowercase();
        let needs_history = !history.is_empty()
            && !lower_prompt.starts_with("[agent_mode]")
            && (lower_prompt.contains("what did")
                || lower_prompt.contains("what was")
                || lower_prompt.contains("earlier")
                || lower_prompt.contains("previous")
                || lower_prompt.contains("remember")
                || lower_prompt.contains("recall")
                || lower_prompt.contains("my name"));
        if needs_history {
            let hist_start = history.len().saturating_sub(4);
            for (u, a) in &history[hist_start..] {
                let u_trim = u.trim();
                let a_trim = a.trim();
                if !u_trim.is_empty() && !a_trim.is_empty() && a_trim.len() <= 240 {
                    prefix_ids.extend(bpe.encode(&format!("User: {u_trim}\nAssistant: {a_trim}")));
                    prefix_ids.push(bpe.eos_id());
                    prefix_ids.extend(bpe.encode("\n"));
                }
            }
        }
        let copy_start = if needs_history {
            1usize
        } else {
            prefix_ids.len()
        };
        prefix_ids.extend(bpe.encode(&format!("User: {}\nAssistant:", prompt.trim())));
        let copy_start = if prefix_ids.len() > GLLM_V9_MAX_SEQ - 32 {
            let keep_from = prefix_ids.len() - (GLLM_V9_MAX_SEQ - 32);
            let mut trimmed = vec![bpe.bos_id()];
            trimmed.extend_from_slice(&prefix_ids[keep_from..]);
            prefix_ids = trimmed;
            1usize
        } else {
            copy_start.max(1)
        };

        let kv_prompt_tokens = prefix_ids.len();
        let open_obs = bpe.encode("<tool_response>");
        let close_obs = bpe.encode("</tool_response>");
        let tool_obs_range = prefix_ids[copy_start..]
            .windows(open_obs.len())
            .rposition(|w| w == open_obs.as_slice())
            .and_then(|rel_open| {
                let obs_start = copy_start + rel_open + open_obs.len();
                prefix_ids[obs_start..]
                    .windows(close_obs.len())
                    .position(|w| w == close_obs.as_slice())
                    .map(|rel_close| (obs_start, obs_start + rel_close))
            });
        let mut cache = GllmV9KvCache::new(self, kv_prompt_tokens, copy_start, tool_obs_range);
        let mut prll_ok = true;

        for &tok in &prefix_ids {
            let step_meta = cache.push_token(self, tok, false);
            if !step_meta.mass_conserved {
                prll_ok = false;
            }
        }

        let cap = max_new_tokens.clamp(1, GLLM_V9_MAX_SEQ.saturating_sub(prefix_ids.len() + 1));
        let mut generated: Vec<u32> = Vec::with_capacity(cap);
        let mut steps: Vec<GllmV9TokenStep> = Vec::with_capacity(cap);
        let eos = bpe.eos_id();
        let mut prev_copy_src: Option<usize> = None;

        for _ in 0..cap {
            let (next_tok, best_score, copy_part, next_copy_src, top_s, mass_ok) =
                cache.predict_next_token(self, &generated, prev_copy_src);
            let was_copied = next_copy_src.is_some();
            if !mass_ok {
                prll_ok = false;
            }
            let query_pos = cache.seq_len.saturating_sub(1);
            if next_tok == eos {
                break;
            }
            steps.push(GllmV9TokenStep {
                token_id: next_tok,
                query_pos,
                top_attended_pos: top_s,
                winning_score: best_score,
                copy_score: copy_part,
            });
            generated.push(next_tok);
            let step_meta = cache.push_token(self, next_tok, was_copied);
            prev_copy_src = next_copy_src;
            if !step_meta.mass_conserved {
                prll_ok = false;
            }
        }

        let decoded = bpe.decode(&generated).trim().to_string();
        GllmV9Generation {
            text: decoded,
            kv_prompt_tokens,
            generated_tokens: generated,
            layers_executed: self.header.n_layers as usize,
            turn_memory_hits: 0,
            prll_mass_conserved: prll_ok,
            steps,
            model_sha256: self.container_sha256.clone(),
        }
    }
}

struct StepDiagnostics {
    mass_conserved: bool,
}

struct GllmV9KvCache {
    d_model: usize,
    n_layers: usize,
    n_heads: usize,
    lut_groups: usize,
    prompt_len: usize,
    copy_start: usize,
    seq_len: usize,
    struct_pos: usize,
    tool_obs_range: Option<(usize, usize)>,
    tokens: Vec<u32>,
    prev_layer_in: Vec<Vec<i16>>, // [n_layers][d_model] causal 1-back input state
    k_cache: Vec<Vec<i16>>,       // [n_layers][seq * d_model]
    v_cache: Vec<Vec<i16>>,       // [n_layers][seq * d_model]
    k_sk0: Vec<Vec<u64>>,         // [n_layers][seq]
    last_top_q: Vec<i16>,
    last_top_q_sk0: u64,
    last_hidden: Vec<i16>,
}

impl GllmV9KvCache {
    fn new(
        model: &GllmV9Model,
        prompt_len: usize,
        copy_start: usize,
        tool_obs_range: Option<(usize, usize)>,
    ) -> Self {
        let d = model.header.d_model as usize;
        let l = model.header.n_layers as usize;
        let h = model.header.n_heads as usize;
        let g = model.header.lut_groups as usize;
        Self {
            d_model: d,
            n_layers: l,
            n_heads: h,
            lut_groups: g,
            prompt_len,
            copy_start,
            seq_len: 0,
            struct_pos: 0,
            tool_obs_range,
            tokens: Vec::with_capacity(GLLM_V9_MAX_SEQ),
            prev_layer_in: vec![vec![0i16; d]; l],
            k_cache: vec![Vec::with_capacity(GLLM_V9_MAX_SEQ * d); l],
            v_cache: vec![Vec::with_capacity(GLLM_V9_MAX_SEQ * d); l],
            k_sk0: vec![Vec::with_capacity(GLLM_V9_MAX_SEQ); l],
            last_top_q: vec![0i16; d],
            last_top_q_sk0: 0,
            last_hidden: vec![0i16; d],
        }
    }

    fn push_token(
        &mut self,
        model: &GllmV9Model,
        tok_id: u32,
        was_copied: bool,
    ) -> StepDiagnostics {
        let t = self.seq_len;
        let tok = (tok_id as usize).min(GLLM_V9_VOCAB - 1);
        let d = self.d_model;
        let h_cnt = self.n_heads;
        let g_cnt = self.lut_groups;

        self.tokens.push(tok as u32);
        self.seq_len += 1;

        // 1. Embedding + Prompt & Span-Length-Invariant Cl(0,6) Structural Phase Transport
        let mut x_cur = vec![0i16; d];
        let pos_idx = if t + 1 < self.prompt_len {
            0
        } else if t + 1 == self.prompt_len {
            self.struct_pos = 1;
            1
        } else {
            if !was_copied {
                self.struct_pos = (self.struct_pos + 1).min(255);
            }
            self.struct_pos
        };
        let spin_t = ((pos_idx * 7 + (tok & 63)) & 63) as u32;
        let et_row = &model.e_tok[tok * d..(tok + 1) * d];
        let ep_row = &model.e_pos[pos_idx * d..(pos_idx + 1) * d];
        let is_bound_var_start = t + 1 < self.prompt_len
            && d >= 128
            && model.copy_tok_gate[tok] >= 208
            && (1..=t)
                .any(|u| self.tokens[u] == tok as u32 && matches!(self.tokens[u - 1], 773 | 269));
        for c in 0..d {
            let perm_c = c ^ (pos_idx & 7);
            let parity = (((c as u32) & 63) & spin_t).count_ones() & 1;
            let ep = if pos_idx == 0 {
                0
            } else {
                ep_row[perm_c] as i32
            };
            let rot = if parity == 0 { ep } else { -ep };
            let et = if is_bound_var_start && c < 80 && ((c / 8) % 4 == 1) {
                0
            } else {
                et_row[c] as i32
            };
            x_cur[c] = clamp_q12(et + rot);
        }

        let mut mass_conserved = true;
        let mut u_blk = vec![0i16; d];
        let mut k_in = vec![0i16; d];
        let mut u_k = vec![0i16; d];
        let mut q_vec = vec![0i16; d];
        let mut k_vec = vec![0i16; d];
        let mut v_vec = vec![0i16; d];
        let mut attn_out = vec![0i16; d];
        let mut x_mid = vec![0i16; d];
        let mut u_ffn = vec![0i16; d];

        for (l, lt) in model.layers.iter().enumerate() {
            for c in 0..d {
                let prev_x = if t > 0 {
                    self.prev_layer_in[l][c ^ 1] as i32
                } else {
                    0
                };
                k_in[c] = clamp_q12((((x_cur[c] as i32) * 3) + prev_x) >> 2);
            }
            self.prev_layer_in[l].copy_from_slice(&x_cur);

            for blk in 0..(d / 64) {
                fwht64_block(
                    &x_cur[blk * 64..(blk + 1) * 64],
                    &mut u_blk[blk * 64..(blk + 1) * 64],
                );
                fwht64_block(
                    &k_in[blk * 64..(blk + 1) * 64],
                    &mut u_k[blk * 64..(blk + 1) * 64],
                );
            }

            let mut q_acc = vec![0i32; d];
            let mut k_acc = vec![0i32; d];
            let mut v_acc = vec![0i32; d];

            for g in 0..g_cnt {
                let q_code = code_4ch(&u_blk[g * 4..g * 4 + 4]) as usize;
                let k_code = code_4ch(&u_k[g * 4..g * 4 + 4]) as usize;
                debug_assert_eq!(
                    admit_tmac_lut_read(TmacLutReadCheck {
                        group_index: g as u64,
                        max_groups: g_cnt as u64,
                        subvector_code: q_code as u64,
                        q12_magnitude: 8192,
                    }),
                    TmacLutVerdict::LutReadAdmitted
                );
                let base_c = (g * 4) % d;
                let qv_row = (g * 256 + q_code) * GLLM_V9_QKV_OUT;
                let k_row = (g * 256 + k_code) * GLLM_V9_QKV_OUT;
                for o in 0..GLLM_V9_QKV_OUT {
                    let dst = (base_c + o) % d;
                    q_acc[dst] += lt.q_lut[qv_row + o] as i32;
                    k_acc[dst] += lt.k_lut[k_row + o] as i32;
                    v_acc[dst] += lt.v_lut[qv_row + o] as i32;
                }
            }

            for c in 0..d {
                q_vec[c] = clamp_q12((q_acc[c] >> 2) + ((u_blk[c] as i32) >> 1));
                k_vec[c] = clamp_q12((k_acc[c] >> 2) + ((u_k[c] as i32) >> 1));
                v_vec[c] = clamp_q12((v_acc[c] >> 2) + ((x_cur[c] as i32) >> 1));
            }
            let q_sk0 = pack_sign64(&q_vec[0..64]);
            let k_sk = pack_sign64(&k_vec[0..64]);

            self.k_cache[l].extend_from_slice(&k_vec);
            self.v_cache[l].extend_from_slice(&v_vec);
            self.k_sk0[l].push(k_sk);

            if l + 1 == self.n_layers {
                self.last_top_q.copy_from_slice(&q_vec);
                self.last_top_q_sk0 = q_sk0;
            }

            // Causal Multi-Head PRLL (R4-27) Attention over s in 0..=t
            let mut raw_w = vec![0u64; t + 1];
            let mut prll_w = vec![0u32; t + 1];

            for h in 0..h_cnt {
                let c_off = h * 8;
                let sh = lt.head_shift[h] as usize;
                let ks_shift = match sh {
                    2 => 2,
                    1 | 3 => 1,
                    _ => 0,
                };
                let tau = (lt.head_tau[h].max(256)) as i32;
                let slope = lt.head_slope[h] as i32;
                let q_byte = ((q_sk0 >> ((h & 7) * 8)) & 0xFF) as u8;

                let mut sum_raw = 0u64;
                let mut best_s = 0usize;
                let mut best_w = 0u64;

                for (s, rw) in raw_w.iter_mut().enumerate().take(t + 1) {
                    let ks = s.saturating_sub(ks_shift);
                    let k_row = &self.k_cache[l][ks * d + c_off..ks * d + c_off + 8];
                    let mut d_l1 = 0i32;
                    for j in 0..8 {
                        d_l1 += ((q_vec[c_off + j] as i32) - (k_row[j] as i32)).abs();
                    }
                    let k_byte = ((self.k_sk0[l][ks] >> ((h & 7) * 8)) & 0xFF) as u8;
                    let d_ham = (q_byte ^ k_byte).count_ones() as i32;
                    let t_is_anchor = model.copy_tok_gate[self.tokens[t] as usize] >= 208;
                    let d_ind = if sh == 1 {
                        if s == t {
                            4_096
                        } else if s > 1
                            && t > 0
                            && s < t
                            && self.tokens[s - 2] == self.tokens[t - 1]
                            && self.tokens[s - 1] == self.tokens[t]
                        {
                            0
                        } else if s > 0
                            && s < t
                            && t_is_anchor
                            && self.tokens[s - 1] == self.tokens[t]
                        {
                            256
                        } else {
                            65_536
                        }
                    } else if sh == 2 {
                        if s == t {
                            4_096
                        } else if s > 2
                            && t > 1
                            && s < t
                            && self.tokens[s - 3] == self.tokens[t - 2]
                            && self.tokens[s - 2] == self.tokens[t - 1]
                            && self.tokens[s - 1] == self.tokens[t]
                        {
                            0
                        } else if s > 1
                            && t > 0
                            && s < t
                            && self.tokens[s - 2] == self.tokens[t - 1]
                            && self.tokens[s - 1] == self.tokens[t]
                        {
                            256
                        } else if s > 0
                            && s < t
                            && t_is_anchor
                            && self.tokens[s - 1] == self.tokens[t]
                        {
                            512
                        } else {
                            65_536
                        }
                    } else if sh == 3 {
                        if s == t {
                            8_192
                        } else if s > 0
                            && s < t
                            && model.copy_tok_gate[self.tokens[s] as usize] >= 208
                        {
                            0
                        } else {
                            65_536
                        }
                    } else if slope <= 4 && t > 0 && s == t {
                        4_096
                    } else {
                        0
                    };
                    let d_geom = if sh == 1 || sh == 2 {
                        (d_l1 >> 3) + 32 * d_ham
                    } else {
                        d_l1 + 256 * d_ham
                    };
                    let d_tot = d_geom + slope * ((t - s) as i32) + d_ind;
                    let w = prll_weight_order4(tau, d_tot);
                    *rw = w;
                    sum_raw += w;
                    if w > best_w {
                        best_w = w;
                        best_s = s;
                    }
                }

                let mut mass_acc = 0u64;
                for s in 0..=t {
                    let wp = ((raw_w[s] * GLLM_V9_PRLL_QUANTUM) / sum_raw) as u32;
                    prll_w[s] = wp;
                    mass_acc += wp as u64;
                }
                prll_w[best_s] += (GLLM_V9_PRLL_QUANTUM - mass_acc) as u32;
                let final_mass: u64 = prll_w.iter().map(|&w| w as u64).sum();

                if admit_causal_kv_step(KvAttentionCheck {
                    seq_len: (t + 1) as u64,
                    max_seq_len: GLLM_V9_MAX_SEQ as u64,
                    attended_pos: best_s as u64,
                    query_pos: t as u64,
                    prll_mass_sum: final_mass,
                    expected_quantum: GLLM_V9_PRLL_QUANTUM,
                }) != KvAttentionVerdict::CausalKvAdmitted
                {
                    mass_conserved = false;
                }

                let mut acc_v = [0i64; 8];
                for (s, &wp_u32) in prll_w.iter().enumerate().take(t + 1) {
                    let wp = wp_u32 as i64;
                    let v_row = &self.v_cache[l][s * d + c_off..s * d + c_off + 8];
                    for j in 0..8 {
                        acc_v[j] += wp * (v_row[j] as i64);
                    }
                }
                for j in 0..8 {
                    attn_out[c_off + j] =
                        clamp_q12((acc_v[j] / (GLLM_V9_PRLL_QUANTUM as i64)) as i32);
                }
            }

            // Residual (damped >> 1) + FWHT-64 + Discrete FFN T-MAC LUT + Icosian Hinge (>> 4)
            for c in 0..d {
                x_mid[c] = clamp_q12((x_cur[c] as i32) + ((attn_out[c] as i32) >> 1));
            }
            for blk in 0..(d / 64) {
                fwht64_block(
                    &x_mid[blk * 64..(blk + 1) * 64],
                    &mut u_ffn[blk * 64..(blk + 1) * 64],
                );
            }
            let mut ffn_acc = vec![0i32; d];
            for g in 0..g_cnt {
                let code = code_4ch(&u_ffn[g * 4..g * 4 + 4]) as usize;
                let row = &lt.ffn_lut[(g * 256 + code) * d..(g * 256 + code + 1) * d];
                for c in 0..d {
                    ffn_acc[c] += row[c] as i32;
                }
            }
            for c in 0..d {
                let act = icosian_hinge(ffn_acc[c] >> 4);
                x_cur[c] = clamp_q12((x_mid[c] as i32) + (act as i32));
            }
        }

        self.last_hidden.copy_from_slice(&x_cur);
        StepDiagnostics { mass_conserved }
    }

    fn predict_next_token(
        &self,
        model: &GllmV9Model,
        generated_so_far: &[u32],
        prev_copy_src: Option<usize>,
    ) -> (u32, i32, i32, Option<usize>, usize, bool) {
        let prev_was_copied = prev_copy_src.is_some();
        let t = self.seq_len - 1;
        let d = self.d_model;
        let g_cnt = self.lut_groups;
        let top_l = self.n_layers - 1;
        let lt = &model.layers[top_l];

        // 1. Precompute 81-entry T-MAC activation lookup tables for last_hidden (ZERO multiplies!)
        let mut act_lut = vec![[0i32; 81]; g_cnt];
        for (g, lut_g) in act_lut.iter_mut().enumerate() {
            build_tmac_act_lut81(&self.last_hidden[g * 4..g * 4 + 4], lut_g);
        }

        let mut copy_logits = vec![0i32; GLLM_V9_VOCAB];
        let mut copy_best_src = vec![0usize; GLLM_V9_VOCAB];
        let mut sh3_cands = vec![[0i32; 4]; GLLM_V9_VOCAB];
        let mut sh3_cnt = vec![0u8; GLLM_V9_VOCAB];
        let mut raw_w = vec![0u64; t + 1];
        let mut prll_w = vec![0u32; t + 1];
        let mut overall_top_s = 0usize;
        let mut overall_top_mass = 0u32;
        let mut mass_ok = true;

        let copy_limit = if self.prompt_len > 2 {
            self.prompt_len - 2
        } else {
            self.prompt_len
        }
        .min(t + 1);
        let bpe = ByteBpe4096::global();
        let gen_prefix = if generated_so_far.is_empty() {
            String::new()
        } else {
            bpe.decode(&generated_so_far[..generated_so_far.len().min(6)])
        };
        let gen_trim = gen_prefix.trim_start();
        let in_code_gen = gen_trim.starts_with("```");
        let in_agent_gen = self.tool_obs_range.is_some() || gen_trim.starts_with('<');
        let in_structured_gen = in_code_gen || in_agent_gen;

        let in_tool_obs_span = prev_copy_src.and_then(|sp| {
            let (obs_lo, obs_hi) = self.tool_obs_range?;
            let ns = sp + 1;
            if sp >= obs_lo && ns < obs_hi.min(copy_limit) {
                Some(ns)
            } else {
                None
            }
        });
        let expected_cont_s = in_tool_obs_span.or_else(|| {
            prev_copy_src.and_then(|sp| {
                let ns = sp + 1;
                if ns >= copy_limit {
                    return None;
                }
                let ns_tok = self.tokens[ns] as usize;
                let ns_gate = model.copy_tok_gate[ns_tok];
                if in_code_gen {
                    let ns_piece = bpe.decode(&[ns_tok as u32]);
                    if !ns_piece.starts_with(' ') && ns_gate >= 160 {
                        Some(ns)
                    } else {
                        None
                    }
                } else if in_agent_gen && ns_gate > 0 {
                    Some(ns)
                } else {
                    None
                }
            })
        });

        // Find last newline in generated_so_far so same-line multi-argument expressions/signatures
        // (`{a} + {b}`, `fn {f}( {p1}, {p2} )`) do not re-select an already-copied span-start anchor
        // on the same line, while resetting across `\n` (`201`) for function bodies & `<tool_call>` lines.
        let line_start_gen_idx = generated_so_far
            .iter()
            .rposition(|&tok| tok == 201)
            .map_or(0, |idx| idx + 1);
        let same_line_slice = &generated_so_far[line_start_gen_idx..];

        let g3_min_open = if d >= 128 { 2_800 } else { 1_200 };

        for h in 0..self.n_heads {
            let mut gate_sum = lt.head_copy_b[h] as i32;
            let cg_row = &lt.head_copy_gate_trits[h * g_cnt..(h + 1) * g_cnt];
            for (g, &pcode) in cg_row.iter().enumerate() {
                gate_sum += act_lut[g][pcode as usize];
            }
            let c_off = h * 8;
            let sh = lt.head_shift[h] as usize;
            let cur_tok = self.tokens[t];
            let fence_or_nl = cur_tok == 66 || cur_tok == 201 || cur_tok == 29;
            let copy_gain = if !in_structured_gen {
                0
            } else if sh == 3 {
                if prev_was_copied || fence_or_nl || gate_sum < g3_min_open {
                    0
                } else {
                    gate_sum.clamp(0, 8_192)
                }
            } else if sh == 1 || sh == 2 {
                if !prev_was_copied || expected_cont_s.is_none() {
                    0
                } else if in_tool_obs_span.is_some() {
                    gate_sum.max(4_096).clamp(0, 8_192)
                } else if gate_sum <= 0 {
                    0
                } else {
                    gate_sum.clamp(0, 8_192)
                }
            } else {
                0
            };
            let ks_shift = match sh {
                2 => 2,
                1 | 3 => 1,
                _ => 0,
            };
            let tau = (lt.head_tau[h].max(256)) as i32;
            let slope = lt.head_slope[h] as i32;
            let q_byte = ((self.last_top_q_sk0 >> ((h & 7) * 8)) & 0xFF) as u8;

            let mut sum_raw = 0u64;
            let mut best_s = 0usize;
            let mut best_w = 0u64;

            for (s, rw) in raw_w.iter_mut().enumerate().take(t + 1) {
                let ks = s.saturating_sub(ks_shift);
                let k_row = &self.k_cache[top_l][ks * d + c_off..ks * d + c_off + 8];
                let mut d_l1 = 0i32;
                for (j, &kv) in k_row.iter().enumerate().take(8) {
                    d_l1 += ((self.last_top_q[c_off + j] as i32) - (kv as i32)).abs();
                }
                let k_byte = ((self.k_sk0[top_l][ks] >> ((h & 7) * 8)) & 0xFF) as u8;
                let d_ham = (q_byte ^ k_byte).count_ones() as i32;
                let t_is_anchor = model.copy_tok_gate[self.tokens[t] as usize] >= 208;
                let d_ind = if sh == 1 {
                    if s == t {
                        4_096
                    } else if let Some(ns) = expected_cont_s {
                        if s == ns {
                            0
                        } else {
                            65_536
                        }
                    } else if s > 1
                        && t > 0
                        && s < t
                        && self.tokens[s - 2] == self.tokens[t - 1]
                        && self.tokens[s - 1] == self.tokens[t]
                    {
                        0
                    } else if s > 0 && s < t && t_is_anchor && self.tokens[s - 1] == self.tokens[t]
                    {
                        256
                    } else {
                        65_536
                    }
                } else if sh == 2 {
                    if s == t {
                        4_096
                    } else if let Some(ns) = expected_cont_s {
                        if s == ns {
                            0
                        } else {
                            65_536
                        }
                    } else if s > 2
                        && t > 1
                        && s < t
                        && self.tokens[s - 3] == self.tokens[t - 2]
                        && self.tokens[s - 2] == self.tokens[t - 1]
                        && self.tokens[s - 1] == self.tokens[t]
                    {
                        0
                    } else if s > 1
                        && t > 0
                        && s < t
                        && self.tokens[s - 2] == self.tokens[t - 1]
                        && self.tokens[s - 1] == self.tokens[t]
                    {
                        256
                    } else if s > 0 && s < t && t_is_anchor && self.tokens[s - 1] == self.tokens[t]
                    {
                        512
                    } else {
                        65_536
                    }
                } else if sh == 3 {
                    if s == t {
                        8_192
                    } else if s > 0 && s < t && model.copy_tok_gate[self.tokens[s] as usize] >= 208
                    {
                        0
                    } else {
                        65_536
                    }
                } else if slope <= 4 && t > 0 && s == t {
                    4_096
                } else {
                    0
                };
                let d_geom = if sh == 1 || sh == 2 {
                    (d_l1 >> 3) + 32 * d_ham
                } else {
                    d_l1 + 256 * d_ham
                };
                let d_tot = d_geom + slope * ((t - s) as i32) + d_ind;
                let w = prll_weight_order4(tau, d_tot);
                *rw = w;
                sum_raw += w;
                if w > best_w {
                    best_w = w;
                    best_s = s;
                }
            }

            let mut mass_acc = 0u64;
            for s in 0..=t {
                let wp = ((raw_w[s] * GLLM_V9_PRLL_QUANTUM) / sum_raw) as u32;
                prll_w[s] = wp;
                mass_acc += wp as u64;
            }
            prll_w[best_s] += (GLLM_V9_PRLL_QUANTUM - mass_acc) as u32;
            let final_mass: u64 = prll_w.iter().map(|&w| w as u64).sum();
            if final_mass != GLLM_V9_PRLL_QUANTUM || best_s > t {
                mass_ok = false;
            }

            if prll_w[best_s] > overall_top_mass {
                overall_top_mass = prll_w[best_s];
                overall_top_s = best_s;
            }

            if copy_gain > 0 {
                let start_s = self.copy_start.min(copy_limit);
                for s in start_s..copy_limit {
                    let tok_s = self.tokens[s] as usize;
                    if self.tokens[start_s..s].contains(&(tok_s as u32)) {
                        continue;
                    }
                    let mut tot_wp = 0u64;
                    let mut argmax_s = s;
                    let mut max_wp = 0u32;
                    for (s2, &pw) in prll_w.iter().enumerate().take(copy_limit).skip(s) {
                        if self.tokens[s2] as usize == tok_s {
                            tot_wp += pw as u64;
                            if pw >= max_wp {
                                max_wp = pw;
                                argmax_s = s2;
                            }
                        }
                    }
                    let is_expected_tok = expected_cont_s
                        .map(|ns| self.tokens[ns] as usize == tok_s)
                        .unwrap_or(false);
                    let tok_allow = if sh != 3
                        && is_expected_tok
                        && (in_tool_obs_span.is_some()
                            || (expected_cont_s
                                .map(|ns| {
                                    ns + 1 < copy_limit
                                        && model.copy_tok_gate[self.tokens[ns + 1] as usize] >= 160
                                })
                                .unwrap_or(false)
                                && matches!(tok_s, 5 | 16 | 17)))
                    {
                        (model.copy_tok_gate[tok_s] as i64).max(192)
                    } else {
                        model.copy_tok_gate[tok_s] as i64
                    };
                    let eff_wp = if sh == 3 { max_wp as u64 } else { tot_wp };
                    let cap_h = if sh == 3 && d >= 128 { 11_264 } else { 12_288 };
                    let cand = ((((eff_wp as i64) * (copy_gain as i64) * tok_allow) >> 25) as i32)
                        .clamp(0, cap_h);
                    if sh == 3 {
                        let idx = (sh3_cnt[tok_s] as usize).min(3);
                        sh3_cands[tok_s][idx] = cand;
                        sh3_cnt[tok_s] = sh3_cnt[tok_s].saturating_add(1);
                        if sh3_cnt[tok_s] == 1 {
                            copy_best_src[tok_s] = s;
                        }
                    } else if cand > copy_logits[tok_s] {
                        copy_logits[tok_s] = cand;
                        copy_best_src[tok_s] = expected_cont_s.unwrap_or(argmax_s);
                    }
                }
            }
        }

        // Combine `sh == 3` syntactic anchor heads (`top1 + top2` on 8L; single head on 6L) so
        // 2-of-3 head agreement beats a single rogue head collision while preserving 1-head body specialization.
        for v in 0..GLLM_V9_VOCAB {
            let n = sh3_cnt[v] as usize;
            if n > 0 {
                let slice = &mut sh3_cands[v][..n.min(4)];
                slice.sort_unstable();
                let mut combined = if slice.len() >= 2 {
                    (slice[slice.len() - 1] + slice[slice.len() - 2]).clamp(0, 18_432)
                } else {
                    slice[0]
                };
                let s = copy_best_src[v];
                if same_line_slice.contains(&(v as u32)) {
                    combined >>= 3;
                } else if combined >= 1_024 {
                    combined += ((copy_limit.saturating_sub(s)) as i32) * 8;
                }
                if combined > copy_logits[v] {
                    copy_logits[v] = combined;
                }
            }
        }

        // Guard strictly against contiguous periodic loops (`>= 5` identical single tokens so Markdown
        // ` ``` ` fences and 4-space indents are untouched, or `>= 3` contiguous multi-token periods).
        let gen_len = generated_so_far.len();
        let mut blocked_cycle_next: Option<u32> = None;
        if gen_len >= 5
            && generated_so_far[gen_len - 5..]
                .iter()
                .all(|&x| x == generated_so_far[gen_len - 1])
        {
            blocked_cycle_next = Some(generated_so_far[gen_len - 1]);
        } else {
            for period in 2..=8usize {
                if gen_len >= period * 3 {
                    let b0 = &generated_so_far[gen_len - 3 * period..gen_len - 2 * period];
                    let b1 = &generated_so_far[gen_len - 2 * period..gen_len - period];
                    let b2 = &generated_so_far[gen_len - period..gen_len];
                    if b0 == b1 && b1 == b2 {
                        blocked_cycle_next = Some(b2[0]);
                        break;
                    }
                }
            }
        }

        let h_sk0 = pack_sign64(&self.last_hidden[0..64]);
        let h_sk1 = if d >= 128 {
            pack_sign64(&self.last_hidden[64..128])
        } else {
            0
        };
        let d_shift = if d >= 128 { 1u32 } else { 0u32 };

        let mut best_v = 1u32;
        let mut best_score = i32::MIN;
        let mut best_copy = 0i32;
        let mut best_winner_pure = i32::MIN;
        let mut max_pure_overall = i32::MIN;
        let mut max_pure_v = 1u32;
        let mut second_pure_overall = i32::MIN;

        let active_mid_span = expected_cont_s
            .map(|ns| copy_logits[self.tokens[ns] as usize] >= 1_024)
            .unwrap_or(false);

        for (v, &copy_logit) in copy_logits.iter().enumerate().take(GLLM_V9_VOCAB) {
            if v == 0 || v == 2 {
                continue; // never emit BOS or UNK during completion
            }
            if v == 1 && active_mid_span {
                continue; // do not emit EOS mid-span when trained window truncated at 256
            }
            if blocked_cycle_next == Some(v as u32) {
                continue;
            }

            let hi_row = &model.tmac_readout_hi[v * g_cnt..(v + 1) * g_cnt];
            let lo_row = &model.tmac_readout_lo[v * g_cnt..(v + 1) * g_cnt];
            let mut tmac_sum = 0i32;
            for g in 0..g_cnt {
                let v_hi = act_lut[g][hi_row[g] as usize];
                let v_lo = act_lut[g][lo_row[g] as usize];
                tmac_sum += (v_hi << 1) + v_hi + v_lo;
            }

            let p_row = &model.p_out[v * d..(v + 1) * d];
            let mut l1 = 0i32;
            for (c, &pv) in p_row.iter().enumerate().take(d) {
                l1 += ((self.last_hidden[c] as i32) - (pv as i32)).abs();
            }
            let mut ham = (h_sk0 ^ model.p_sk0[v]).count_ones() as i32;
            if d >= 128 {
                ham += (h_sk1 ^ model.p_sk1[v]).count_ones() as i32;
            }
            let b_val = model.b_out[v] as i32;
            let pure_score = (tmac_sum >> d_shift) + (b_val * 2)
                - ((ham * 24) >> d_shift)
                - (l1 >> (5 + d_shift));
            let score = pure_score + copy_logit;

            if pure_score > max_pure_overall {
                second_pure_overall = max_pure_overall;
                max_pure_overall = pure_score;
                max_pure_v = v as u32;
            } else if pure_score > second_pure_overall {
                second_pure_overall = pure_score;
            }

            if score > best_score {
                best_score = score;
                best_v = v as u32;
                best_copy = copy_logit;
                best_winner_pure = pure_score;
            }
        }

        let won_by_vocab_readout = !prev_was_copied
            && best_v == max_pure_v
            && best_winner_pure > -1_024
            && best_winner_pure > second_pure_overall.saturating_add(512);
        let next_copy_src = if best_copy >= 1_024 && !won_by_vocab_readout {
            Some(copy_best_src[best_v as usize])
        } else {
            None
        };

        (
            best_v,
            best_score,
            best_copy,
            next_copy_src,
            overall_top_s,
            mass_ok,
        )
    }
}
