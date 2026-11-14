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
use crate::lexlean_generated::r4_connected_studio::{admit_worker_turn, WorkerTurnVerdict};
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
/// Maximum causal KV sequence window per inference turn (`R4-39`).
pub const GLLM_V9_MAX_SEQ: usize = 1024;
/// Immutable SHA-256 of the legacy pre-v10 `6L 64D` `bootstrap_v9.uorgllm` container.
pub const LEGACY_V9_BOOTSTRAP_SHA256: &str =
    "9fbe91bbac22db96c9b79bd087b4b1c12c4b74dd4debfcda409f79e645ac22f7";
/// Immutable SHA-256 of the legacy `R4-38` `8L 128D` `gpu/weights/v9` container.
pub const LEGACY_V9_FULL_SHA256: &str =
    "ff3021c04bde6b1e0a65d7a7390eea61a6a2f73a86bc1f232a8732dd84448a60";

/// Embedded 6-layer 128D 16H `UORGLLM9` bootstrap checkpoint trained on RTX 4090 (`gpu/train_gllm_v10.cu`).
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
    /// Worker execution thread or `1024`-token KV sequence window violated `R4.ConnectedStudio.admitWorkerTurn`.
    WorkerTurnRefused(WorkerTurnVerdict),
}

impl std::fmt::Display for GllmV9Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TruncatedContainer => write!(f, "truncated UORGLLM9 container"),
            Self::ContainerRefused(v) => write!(f, "UORGLLM9 container refused: {v:?}"),
            Self::IntegrityMismatch => write!(f, "UORGLLM9 payload SHA-256 mismatch"),
            Self::CausalKvRefused(v) => write!(f, "causal KV step refused: {v:?}"),
            Self::LutReadRefused(v) => write!(f, "T-MAC LUT read refused: {v:?}"),
            Self::WorkerTurnRefused(v) => write!(f, "worker turn refused: {v:?}"),
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
    /// Hidden channel width (`64`, `128`, or `256`, multiple of `64`).
    pub d_model: u32,
    /// Number of stacked causal geometric layers (`6`, `8`, or `16`).
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

/// Structured per-step geometric brain activity record serialized under `"brain_pulses"` (`R4-39`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BrainPulseStep {
    /// Monotonic 0-based pulse index within the turn.
    pub step: u32,
    /// Inference phase (`"encode" | "think" | "tool_call" | "tool_obs" | "reply"`).
    pub phase: &'static str,
    /// Active `ByteBpe4096` token ID.
    pub token_id: u32,
    /// Dominant active geometric layer (`0..n_layers-1`).
    pub active_layer: u8,
    /// Highest-mass causal attention head (`0..n_heads-1`).
    pub winning_head: u8,
    /// Assigned $E_8$ root orbit cell (`0..15`).
    pub e8_orbit: u8,
    /// Active Clifford `Cl(0,6)` spin grade (`0..6`).
    pub cl06_grade: u8,
    /// Causal KV distance `query_pos - attended_pos` when pointer-copy fires (`0` on vocab readout).
    pub kv_copy_dist: u16,
    /// Top attended PRLL mass out of `1_000_000`.
    pub prll_top_mass: u32,
    /// Active ReAct tool family (`"none" | "mvm" | "web" | "git" | "pkg"`).
    pub tool_kind: &'static str,
}

/// Step-level output diagnostics for one autoregressively predicted token in `UOR-R4-GLLM`.
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
    /// Margin between the winning token's pure T-MAC readout score and the runner-up (`R4-39`).
    pub pure_margin: i32,
}

/// Full trace and output of compositional autoregressive generation (`R4-38` / `R4-39`).
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
    /// Per-token step records.
    pub steps: Vec<GllmV9TokenStep>,
    /// Structured geometric brain activity pulses emitted across encode, think, tool, and reply steps.
    pub brain_pulses: Vec<BrainPulseStep>,
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
    p_sk: Vec<u64>,           // [4096 * (d_model / 64)]
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
fn prll_weight_order4_v9(tau: i32, d_tot: i32) -> u64 {
    let denom = ((tau.max(256)) as u64) + ((d_tot.max(0) as u64) >> 1);
    let q1 = 1_073_741_824u64 / denom.max(1);
    let q2 = (q1 * q1) >> 20;
    ((q2 * q2) >> 16) + 1
}

#[inline]
fn prll_weight_order4_v10(tau: i32, d_tot: i32) -> u64 {
    let t = (tau.max(256)) as u64;
    let d = (d_tot.max(0)) as u64;
    let num1 = t << 8;
    let den1 = t + d;
    let r1 = num1 / den1.max(1);
    let r2 = r1 * r1;
    let r4 = (r2 >> 4) * (r2 >> 4);
    if r4 == 0 {
        1
    } else {
        r4
    }
}

#[inline]
fn prll_weight_order4(legacy_v9: bool, tau: i32, d_tot: i32) -> u64 {
    if legacy_v9 {
        prll_weight_order4_v9(tau, d_tot)
    } else {
        prll_weight_order4_v10(tau, d_tot)
    }
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

        let n_blocks = d / 64;
        let mut p_sk = Vec::with_capacity(GLLM_V9_VOCAB * n_blocks);
        for v in 0..GLLM_V9_VOCAB {
            let row = &p_out[v * d..(v + 1) * d];
            for b in 0..n_blocks {
                p_sk.push(pack_sign64(&row[b * 64..(b + 1) * 64]));
            }
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
            p_sk,
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

    /// Return `true` when this instance carries one of the pre-v10 `R4-38` `User: ...\nAssistant:` containers.
    pub fn is_legacy_v9_container(&self) -> bool {
        self.container_sha256 == LEGACY_V9_BOOTSTRAP_SHA256
            || self.container_sha256 == LEGACY_V9_FULL_SHA256
    }

    /// Validate a Worker inference turn against `R4.ConnectedStudio.admitWorkerTurn` (`R4-39`).
    pub fn validate_worker_turn_window(
        runs_on_worker_thread: bool,
        prompt_tokens: usize,
        max_new_tokens: usize,
    ) -> Result<(), GllmV9Error> {
        let verdict = admit_worker_turn(
            runs_on_worker_thread,
            prompt_tokens as u32,
            max_new_tokens as u32,
            GLLM_V9_MAX_SEQ as u32,
        );
        if verdict == WorkerTurnVerdict::WorkerTurnAdmitted {
            Ok(())
        } else {
            Err(GllmV9Error::WorkerTurnRefused(verdict))
        }
    }

    /// Check whether `prompt` is an actual cross-turn recall or follow-up query (`R4-39`).
    /// Declarative session-fact statements (e.g. `"my repo is called nebula-os"`) return `false`.
    pub fn has_v10_cross_turn_cue(prompt: &str) -> bool {
        let lower = prompt.trim().to_ascii_lowercase();
        const DIRECT_FOLLOWUP_CUES: &[&str] = &[
            "just say",
            "just said",
            "what did i",
            "what was",
            "earlier",
            "previous",
            "above",
            "simpler",
            "explain that",
            "summarize what",
            "remind",
            "recall",
        ];
        if DIRECT_FOLLOWUP_CUES.iter().any(|cue| lower.contains(cue)) {
            return true;
        }
        let is_interrogative = lower.contains('?')
            || lower.starts_with("what ")
            || lower.starts_with("which ")
            || lower.starts_with("who ")
            || lower.starts_with("where ")
            || lower.starts_with("tell me ")
            || lower.contains("remember");
        const ENTITY_RECALL_CUES: &[&str] = &[
            "my repo",
            "my repository",
            "my project",
            "my name",
            "branch",
            "module_",
        ];
        is_interrogative && ENTITY_RECALL_CUES.iter().any(|cue| lower.contains(cue))
    }

    /// Check whether `prompt` declares a conversational session entity (`repo`, `project`, `name`, `branch`)
    /// that subsequent multi-turn KV recall queries may reference (`R4-39`).
    pub fn is_v10_session_fact_declaration(prompt: &str) -> bool {
        if Self::has_v10_cross_turn_cue(prompt) {
            return false;
        }
        let lower = prompt.trim().to_ascii_lowercase();
        const FACT_PREFIXES: &[&str] = &[
            "my repo is ",
            "my repository is ",
            "we are working on a repo ",
            "our project is ",
            "my project is ",
            "my name is ",
            "call me ",
            "we are on branch ",
            "active branch is ",
            "i am auditing ",
        ];
        FACT_PREFIXES.iter().any(|p| lower.starts_with(p))
    }

    /// Encode a multi-turn `<|user|>` / `<|assistant|>` `v10` chat dialogue into `<= 1024` `ByteBpe4096` tokens.
    pub fn encode_v10_chat_prompt(
        prompt: &str,
        history: &[(String, String)],
        max_new_tokens: usize,
    ) -> (Vec<u32>, usize) {
        Self::encode_v10_chat_prompt_with_warmup(prompt, &[], history, max_new_tokens)
    }

    /// Encode a `v10` chat dialogue with an optional live-generated KV positional warmup prefix
    /// whose tokens remain strictly excluded from pointer-copy (`copy_start >= real_history_start`).
    pub fn encode_v10_chat_prompt_with_warmup(
        prompt: &str,
        warmup_turns: &[(String, String)],
        history: &[(String, String)],
        max_new_tokens: usize,
    ) -> (Vec<u32>, usize) {
        let bpe = ByteBpe4096::global();
        let reserved_gen = max_new_tokens.clamp(16, 256);
        let max_prompt_budget = GLLM_V9_MAX_SEQ.saturating_sub(reserved_gen + 2).max(64);
        let current_turn_ids = bpe.encode(&format!("<|user|>\n{}\n<|assistant|>\n", prompt.trim()));

        let compact_assistant = |raw: &str| -> String {
            let trimmed = raw.trim();
            if trimmed.chars().count() <= 800 {
                return trimmed.to_string();
            }
            let first_line = trimmed.lines().next().unwrap_or(trimmed).trim();
            if let Some(dot_pos) = first_line.find(". ") {
                let sent = &first_line[..=dot_pos];
                if sent.chars().count() <= 400 {
                    return sent.to_string();
                }
            }
            first_line.chars().take(400).collect()
        };

        let mut warmup_encoded: Vec<Vec<u32>> = Vec::new();
        let mut warmup_tokens = 0usize;
        for (u, a) in warmup_turns {
            let u_trim = u.trim();
            let a_trim = compact_assistant(a);
            if u_trim.is_empty() || a_trim.is_empty() {
                continue;
            }
            let t_ids = bpe.encode(&format!(
                "<|user|>\n{u_trim}\n<|assistant|>\n{a_trim}<|end|>\n"
            ));
            if 1 + warmup_tokens + t_ids.len() + current_turn_ids.len() <= max_prompt_budget {
                warmup_tokens += t_ids.len();
                warmup_encoded.push(t_ids);
            }
        }

        let mut selected_turns: Vec<Vec<u32>> = Vec::new();
        let mut used = 1usize + warmup_tokens + current_turn_ids.len();
        for (u, a) in history.iter().rev() {
            let u_trim = u.trim();
            let a_trim = compact_assistant(a);
            if u_trim.is_empty() || a_trim.is_empty() {
                continue;
            }
            let turn_ids = bpe.encode(&format!(
                "<|user|>\n{u_trim}\n<|assistant|>\n{a_trim}<|end|>\n"
            ));
            if used + turn_ids.len() > max_prompt_budget {
                break;
            }
            used += turn_ids.len();
            selected_turns.push(turn_ids);
        }
        selected_turns.reverse();

        let mut prefix_ids = Vec::with_capacity(used.min(max_prompt_budget));
        prefix_ids.push(bpe.bos_id());
        for w_ids in warmup_encoded {
            prefix_ids.extend_from_slice(&w_ids);
        }
        let mut real_history_start = prefix_ids.len();
        for t_ids in selected_turns {
            prefix_ids.extend_from_slice(&t_ids);
        }
        let mut current_turn_start = prefix_ids.len();
        prefix_ids.extend_from_slice(&current_turn_ids);

        if prefix_ids.len() > max_prompt_budget {
            let keep = max_prompt_budget.saturating_sub(1);
            let tail_start = prefix_ids.len().saturating_sub(keep);
            let mut clipped = Vec::with_capacity(1 + keep);
            clipped.push(bpe.bos_id());
            clipped.extend_from_slice(&prefix_ids[tail_start..]);
            real_history_start = if real_history_start >= tail_start {
                1 + (real_history_start - tail_start)
            } else {
                1
            };
            current_turn_start = if current_turn_start >= tail_start {
                1 + (current_turn_start - tail_start)
            } else {
                1
            };
            prefix_ids = clipped;
        }
        let copy_start = if !history.is_empty() && Self::has_v10_cross_turn_cue(prompt) {
            real_history_start.max(1)
        } else {
            current_turn_start.max(1)
        };
        (prefix_ids, copy_start)
    }

    /// Execute `1024`-token multi-turn `<|user|>`/`<|assistant|>` causal KV generation (`R4-39`).
    pub fn generate_compositional_v10(
        &self,
        prompt: &str,
        history: &[(String, String)],
        max_new_tokens: usize,
    ) -> GllmV9Generation {
        self.generate_compositional_v10_with_warmup(prompt, &[], history, max_new_tokens)
    }

    /// Execute `1024`-token multi-turn `v10` causal KV generation with live-generated KV positional warmup.
    pub fn generate_compositional_v10_with_warmup(
        &self,
        prompt: &str,
        warmup_turns: &[(String, String)],
        history: &[(String, String)],
        max_new_tokens: usize,
    ) -> GllmV9Generation {
        let (prefix_ids, copy_start) =
            Self::encode_v10_chat_prompt_with_warmup(prompt, warmup_turns, history, max_new_tokens);
        self.run_kv_generation(prefix_ids, copy_start, max_new_tokens, true)
    }

    /// Encode prompt-prefill causal KV states without decoding tokens, emitting `encode` `BrainPulseStep`s
    /// for deterministic kernel turns (`Teach:`, `solve_exact_math_or_logic`, `[agent_mode]`).
    pub fn encode_prompt_pulses(
        &self,
        prompt: &str,
        history: &[(String, String)],
    ) -> Vec<BrainPulseStep> {
        let trimmed = prompt.trim();
        let (prefix_ids, copy_start) = if !self.is_legacy_v9_container() {
            Self::encode_v10_chat_prompt(trimmed, history, 64)
        } else {
            let bpe = ByteBpe4096::global();
            let mut ids = vec![bpe.bos_id()];
            ids.extend(bpe.encode(&format!("User: {trimmed}\nAssistant:")));
            if ids.len() > GLLM_V9_MAX_SEQ - 8 {
                ids.truncate(GLLM_V9_MAX_SEQ - 8);
            }
            (ids, 1usize)
        };
        let kv_prompt_tokens = prefix_ids.len();
        let safe_copy_start = copy_start.min(kv_prompt_tokens);
        let mut cache = GllmV9KvCache::new(self, kv_prompt_tokens, safe_copy_start, None);
        let encode_stride = (kv_prompt_tokens / 12).max(1);
        let mut brain_pulses = Vec::new();
        for (idx, &tok) in prefix_ids.iter().enumerate() {
            let step_meta = cache.push_token(self, tok, false);
            if (idx + 1 == kv_prompt_tokens || idx % encode_stride == 0)
                && (brain_pulses.len() < 24 || idx + 1 == kv_prompt_tokens)
            {
                brain_pulses.push(BrainPulseStep {
                    step: brain_pulses.len() as u32,
                    phase: "encode",
                    token_id: tok,
                    active_layer: step_meta.dominant_layer,
                    winning_head: step_meta.winning_head,
                    e8_orbit: step_meta.e8_orbit,
                    cl06_grade: step_meta.cl06_grade,
                    kv_copy_dist: 0,
                    prll_top_mass: step_meta.top_mass,
                    tool_kind: "none",
                });
            }
        }
        brain_pulses
    }

    /// Execute full compositional autoregressive generation over `prompt` and `history` (`R4-38` / `R4-39`).
    pub fn generate_compositional(
        &self,
        prompt: &str,
        history: &[(String, String)],
        max_new_tokens: usize,
    ) -> GllmV9Generation {
        let trimmed_prompt = prompt.trim();
        if !self.is_legacy_v9_container() && !trimmed_prompt.starts_with("[agent_mode]") {
            return self.generate_compositional_v10(trimmed_prompt, history, max_new_tokens);
        }

        let bpe = ByteBpe4096::global();
        let mut prefix_ids: Vec<u32> = vec![bpe.bos_id()];

        // Include recent session history inside the live causal KV cache only when the
        // prompt references prior conversational state, preventing standalone coding or
        // sandbox turns from inheriting unrelated boot/assistant tokens on legacy v9 weights.
        let lower_prompt = trimmed_prompt.to_ascii_lowercase();
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
        prefix_ids.extend(bpe.encode(&format!("User: {trimmed_prompt}\nAssistant:")));
        let reserved = max_new_tokens.clamp(16, 256);
        let max_prefix = GLLM_V9_MAX_SEQ.saturating_sub(reserved + 2).max(64);
        let copy_start = if prefix_ids.len() > max_prefix {
            let keep_from = prefix_ids.len() - max_prefix;
            let mut trimmed = vec![bpe.bos_id()];
            trimmed.extend_from_slice(&prefix_ids[keep_from..]);
            prefix_ids = trimmed;
            1usize
        } else {
            copy_start.max(1)
        };

        self.run_kv_generation(prefix_ids, copy_start, max_new_tokens, false)
    }

    fn run_kv_generation(
        &self,
        prefix_ids: Vec<u32>,
        copy_start: usize,
        max_new_tokens: usize,
        v10_stop_markers: bool,
    ) -> GllmV9Generation {
        let bpe = ByteBpe4096::global();
        let kv_prompt_tokens = prefix_ids.len();
        let open_obs = bpe.encode("<tool_response>");
        let close_obs = bpe.encode("</tool_response>");
        let safe_copy_start = copy_start.min(prefix_ids.len());
        let tool_obs_range = prefix_ids[safe_copy_start..]
            .windows(open_obs.len())
            .rposition(|w| w == open_obs.as_slice())
            .and_then(|rel_open| {
                let obs_start = safe_copy_start + rel_open + open_obs.len();
                prefix_ids[obs_start..]
                    .windows(close_obs.len())
                    .position(|w| w == close_obs.as_slice())
                    .map(|rel_close| (obs_start, obs_start + rel_close))
            })
            .or_else(|| {
                let mut joined = String::new();
                let mut spans =
                    Vec::with_capacity(prefix_ids.len().saturating_sub(safe_copy_start));
                for (rel_idx, &tok) in prefix_ids[safe_copy_start..].iter().enumerate() {
                    let s0 = joined.len();
                    joined.push_str(&bpe.decode(&[tok]));
                    spans.push((safe_copy_start + rel_idx, s0, joined.len()));
                }
                let open_pos = joined.rfind("<tool_response>")?;
                let inner_start = open_pos + "<tool_response>".len();
                let close_rel = joined[inner_start..].find("</tool_response>")?;
                let inner_end = inner_start + close_rel;
                let lo = spans
                    .iter()
                    .find(|&&(_, _, b1)| b1 > inner_start)
                    .map(|&(idx, _, _)| idx)?;
                let hi = spans
                    .iter()
                    .rfind(|&&(_, b0, _)| b0 < inner_end)
                    .map(|&(idx, _, _)| idx + 1)
                    .unwrap_or(lo);
                (lo < hi).then_some((lo, hi))
            });
        let mut cache = GllmV9KvCache::new(self, kv_prompt_tokens, safe_copy_start, tool_obs_range);
        let mut prll_ok = true;
        let mut brain_pulses: Vec<BrainPulseStep> = Vec::new();
        let encode_stride = (kv_prompt_tokens / 12).max(1);

        for (idx, &tok) in prefix_ids.iter().enumerate() {
            let step_meta = cache.push_token(self, tok, false);
            if !step_meta.mass_conserved {
                prll_ok = false;
            }
            let in_obs = tool_obs_range
                .map(|(lo, hi)| idx >= lo && idx < hi)
                .unwrap_or(false);
            if (idx + 1 == kv_prompt_tokens || idx % encode_stride == 0 || in_obs)
                && (brain_pulses.len() < 24 || idx + 1 == kv_prompt_tokens)
            {
                brain_pulses.push(BrainPulseStep {
                    step: brain_pulses.len() as u32,
                    phase: if in_obs { "tool_obs" } else { "encode" },
                    token_id: tok,
                    active_layer: step_meta.dominant_layer,
                    winning_head: step_meta.winning_head,
                    e8_orbit: step_meta.e8_orbit,
                    cl06_grade: step_meta.cl06_grade,
                    kv_copy_dist: 0,
                    prll_top_mass: step_meta.top_mass,
                    tool_kind: if in_obs { "mvm" } else { "none" },
                });
            }
        }

        let cap = max_new_tokens.clamp(1, GLLM_V9_MAX_SEQ.saturating_sub(prefix_ids.len() + 1));
        let mut generated: Vec<u32> = Vec::with_capacity(cap);
        let mut steps: Vec<GllmV9TokenStep> = Vec::with_capacity(cap);
        let eos = bpe.eos_id();
        let mut prev_copy_src: Option<usize> = None;

        for _ in 0..cap {
            let pred = cache.predict_next_token(self, &generated, prev_copy_src);
            let was_copied = pred.next_copy_src.is_some();
            if !pred.mass_ok {
                prll_ok = false;
            }
            let query_pos = cache.seq_len.saturating_sub(1);
            if pred.token_id == eos {
                break;
            }
            steps.push(GllmV9TokenStep {
                token_id: pred.token_id,
                query_pos,
                top_attended_pos: pred.top_attended_pos,
                winning_score: pred.best_score,
                copy_score: pred.copy_score,
                pure_margin: pred.pure_margin,
            });
            generated.push(pred.token_id);
            let step_meta = cache.push_token(self, pred.token_id, was_copied);
            prev_copy_src = pred.next_copy_src;
            if !step_meta.mass_conserved {
                prll_ok = false;
            }

            let partial_text = bpe.decode(&generated);
            let (phase, tool_kind) = classify_pulse_phase_and_tool(&partial_text);
            let copy_dist = pred
                .next_copy_src
                .map(|src| query_pos.saturating_sub(src).min(u16::MAX as usize) as u16)
                .unwrap_or(0);
            brain_pulses.push(BrainPulseStep {
                step: brain_pulses.len() as u32,
                phase,
                token_id: pred.token_id,
                active_layer: step_meta.dominant_layer,
                winning_head: pred.winning_head,
                e8_orbit: step_meta.e8_orbit,
                cl06_grade: step_meta.cl06_grade,
                kv_copy_dist: copy_dist,
                prll_top_mass: pred.top_mass,
                tool_kind,
            });

            if v10_stop_markers
                && (partial_text.contains("<|end|>") || partial_text.ends_with("</tool_call>"))
            {
                break;
            }
        }

        let mut decoded = bpe.decode(&generated);
        if v10_stop_markers {
            if let Some(idx) = decoded.find("<|end|>") {
                decoded.truncate(idx);
            }
            decoded = strip_v10_thought_block(&decoded);
        }
        let decoded = decoded.trim().to_string();
        GllmV9Generation {
            text: decoded,
            kv_prompt_tokens,
            generated_tokens: generated,
            layers_executed: self.header.n_layers as usize,
            turn_memory_hits: 0,
            prll_mass_conserved: prll_ok,
            steps,
            brain_pulses,
            model_sha256: self.container_sha256.clone(),
        }
    }
}

fn strip_v10_thought_block(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(after_open) = trimmed.strip_prefix("<thought>") {
        if let Some((_, rest)) = after_open.split_once("</thought>") {
            return rest.trim().to_string();
        }
    }
    trimmed.to_string()
}

fn classify_pulse_phase_and_tool(partial: &str) -> (&'static str, &'static str) {
    let in_thought = (partial.contains("<thought>") && !partial.contains("</thought>"))
        || (partial.contains("<thinking>") && !partial.contains("</thinking>"));
    if in_thought {
        return ("think", "none");
    }
    if let Some(idx) = partial.rfind("<tool_call>") {
        let tail = &partial[idx..];
        let tool_kind = if tail.contains("web_search") || tail.contains("web_fetch") {
            "web"
        } else if tail.contains("\"git\"") || tail.contains("git ") {
            "git"
        } else if tail.contains("pkg ") || tail.contains("\"pkg\"") {
            "pkg"
        } else {
            "mvm"
        };
        if !tail.contains("</tool_call>") {
            return ("tool_call", tool_kind);
        }
        return ("tool_call", tool_kind);
    }
    ("reply", "none")
}

struct StepDiagnostics {
    mass_conserved: bool,
    dominant_layer: u8,
    winning_head: u8,
    e8_orbit: u8,
    cl06_grade: u8,
    top_mass: u32,
}

struct PredictOutcome {
    token_id: u32,
    best_score: i32,
    copy_score: i32,
    pure_margin: i32,
    next_copy_src: Option<usize>,
    top_attended_pos: usize,
    winning_head: u8,
    top_mass: u32,
    mass_ok: bool,
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
    k_sk: Vec<Vec<u64>>,          // [n_layers][seq * sk_blocks]
    last_top_q: Vec<i16>,
    last_top_q_sk: Vec<u64>, // [sk_blocks]
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
        let sk_blocks = if d >= 256 { d / 64 } else { 1 };
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
            k_sk: vec![Vec::with_capacity(GLLM_V9_MAX_SEQ * sk_blocks); l],
            last_top_q: vec![0i16; d],
            last_top_q_sk: vec![0u64; sk_blocks],
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
        let gate3_lo = if d >= 128 { d - 48 } else { 40 };
        for c in 0..d {
            let perm_c = c ^ (pos_idx & 7);
            let parity = (((c as u32) & 63) & spin_t).count_ones() & 1;
            let ep = if pos_idx == 0 {
                0
            } else {
                ep_row[perm_c] as i32
            };
            let rot = if parity == 0 { ep } else { -ep };
            let et = if is_bound_var_start && c < gate3_lo && ((c / 8) % 4 == 1) {
                0
            } else {
                et_row[c] as i32
            };
            x_cur[c] = clamp_q12(et + rot);
        }

        let mut mass_conserved = true;
        let mut dominant_layer = 0u8;
        let mut winning_head = 0u8;
        let mut top_mass = 0u32;
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
            let sk_blocks = if d >= 256 { d / 64 } else { 1 };
            let mut q_sk = vec![0u64; sk_blocks];
            for b in 0..sk_blocks {
                q_sk[b] = pack_sign64(&q_vec[b * 64..(b + 1) * 64]);
                self.k_sk[l].push(pack_sign64(&k_vec[b * 64..(b + 1) * 64]));
            }

            self.k_cache[l].extend_from_slice(&k_vec);
            self.v_cache[l].extend_from_slice(&v_vec);

            if l + 1 == self.n_layers {
                self.last_top_q.copy_from_slice(&q_vec);
                self.last_top_q_sk.copy_from_slice(&q_sk);
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
                let h_blk = if d >= 256 { h / 8 } else { 0 };
                let q_byte = ((q_sk[h_blk] >> ((h & 7) * 8)) & 0xFF) as u8;

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
                    let k_byte =
                        ((self.k_sk[l][ks * sk_blocks + h_blk] >> ((h & 7) * 8)) & 0xFF) as u8;
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
                    let w = prll_weight_order4(model.is_legacy_v9_container(), tau, d_tot);
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
                if prll_w[best_s] >= top_mass {
                    top_mass = prll_w[best_s];
                    dominant_layer = l as u8;
                    winning_head = h as u8;
                }

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

            // Residual (damped >> 1 for <=8L, >> 2 for 16L to prevent 1.25^16 blowup) + FWHT-64 + Discrete FFN T-MAC LUT + Icosian Hinge
            let attn_res_shift = if self.n_layers >= 16 { 2 } else { 1 };
            let ffn_shift = if d >= 256 { 5 } else { 4 };
            for c in 0..d {
                x_mid[c] = clamp_q12((x_cur[c] as i32) + ((attn_out[c] as i32) >> attn_res_shift));
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
                let act = icosian_hinge(ffn_acc[c] >> ffn_shift);
                x_cur[c] = clamp_q12((x_mid[c] as i32) + (act as i32));
            }
        }

        self.last_hidden.copy_from_slice(&x_cur);
        let h_sign = pack_sign64(&x_cur[0..64]);
        let e8_orbit = (((h_sign.count_ones() as usize) + tok) & 15) as u8;
        let cl06_grade = ((spin_t & 0x3F).count_ones() as u8).min(6);
        StepDiagnostics {
            mass_conserved,
            dominant_layer,
            winning_head,
            e8_orbit,
            cl06_grade,
            top_mass: top_mass.min(GLLM_V9_PRLL_QUANTUM as u32),
        }
    }

    fn predict_next_token(
        &self,
        model: &GllmV9Model,
        generated_so_far: &[u32],
        prev_copy_src: Option<usize>,
    ) -> PredictOutcome {
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
        let mut sh3_cands = vec![[0i32; 8]; GLLM_V9_VOCAB];
        let mut sh3_cnt = vec![0u8; GLLM_V9_VOCAB];
        let mut raw_w = vec![0u64; t + 1];
        let mut prll_w = vec![0u32; t + 1];
        let mut overall_top_s = 0usize;
        let mut overall_top_mass = 0u32;
        let mut overall_winning_head = 0u8;
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
        let is_v10_model = !model.is_legacy_v9_container();

        let in_tool_obs_span = match prev_copy_src {
            Some(sp) => self.tool_obs_range.and_then(|(obs_lo, obs_hi)| {
                let ns = sp + 1;
                if sp >= obs_lo && ns < obs_hi.min(copy_limit) {
                    Some(ns)
                } else {
                    None
                }
            }),
            None if is_v10_model => self.tool_obs_range.and_then(|(obs_lo, obs_hi)| {
                if generated_so_far.len() < 3 {
                    return None;
                }
                let scan_end = obs_hi.min(copy_limit).saturating_sub(1);
                let hi_bound = obs_lo.saturating_add(4).min(scan_end);
                (obs_lo..hi_bound)
                    .find_map(|u| {
                        if self.tokens[u] == self.tokens[t]
                            && (model.copy_tok_gate[self.tokens[u] as usize] >= 160
                                || bpe
                                    .decode(&[self.tokens[u]])
                                    .chars()
                                    .any(|ch| ch.is_ascii_alphabetic()))
                        {
                            Some(u + 1)
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        if generated_so_far.len() < 4 {
                            return None;
                        }
                        let tail_txt = bpe
                            .decode(&generated_so_far[generated_so_far.len().saturating_sub(4)..]);
                        if tail_txt.ends_with(" —") || tail_txt.ends_with(" --") {
                            (obs_lo..scan_end)
                                .find_map(|u| {
                                    if self.tokens[u] == self.tokens[t]
                                        && bpe.decode(&self.tokens[obs_lo..=u]).ends_with(" —")
                                    {
                                        Some(u + 1)
                                    } else {
                                        None
                                    }
                                })
                                .or({
                                    if obs_lo < obs_hi.min(copy_limit) {
                                        Some(obs_lo)
                                    } else {
                                        None
                                    }
                                })
                        } else {
                            None
                        }
                    })
            }),
            None => None,
        };
        let mut bridged_uncalibrated_word = false;
        let in_agent_cmd_3gram = if is_v10_model
            && in_agent_gen
            && self.tool_obs_range.is_none()
            && prev_copy_src.is_none()
            && t >= 2
        {
            let start_s = self.copy_start.saturating_add(3).min(copy_limit);
            (start_s..copy_limit).find(|&s| {
                if self.tokens[s - 3] == self.tokens[t - 2]
                    && self.tokens[s - 2] == self.tokens[t - 1]
                    && self.tokens[s - 1] == self.tokens[t]
                {
                    let piece = bpe.decode(&[self.tokens[s]]);
                    if let Some(rest) = piece.strip_prefix(' ') {
                        if !rest.is_empty()
                            && !matches!(
                                piece.as_str(),
                                " into"
                                    | " inside"
                                    | " and"
                                    | " package"
                                    | " summarize"
                                    | " report"
                            )
                            && rest.chars().all(|ch| {
                                ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')
                            })
                        {
                            bridged_uncalibrated_word = true;
                            return true;
                        }
                    }
                    false
                } else {
                    false
                }
            })
        } else {
            None
        };
        let in_causal_entry_span = in_tool_obs_span.or(in_agent_cmd_3gram);
        let expected_cont_s = in_causal_entry_span.or_else(|| {
            prev_copy_src.and_then(|sp| {
                let ns = sp + 1;
                if ns >= copy_limit {
                    return None;
                }
                let ns_tok = self.tokens[ns] as usize;
                let ns_gate = model.copy_tok_gate[ns_tok];
                if in_code_gen {
                    let ns_piece = bpe.decode(&[ns_tok as u32]);
                    let is_ident_cont = !ns_piece.is_empty()
                        && !ns_piece.starts_with(char::is_whitespace)
                        && ns_piece
                            .chars()
                            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
                    if (!ns_piece.starts_with(' ') && ns_gate >= 160)
                        || (is_v10_model && is_ident_cont)
                    {
                        Some(ns)
                    } else {
                        None
                    }
                } else if in_agent_gen {
                    if ns_gate > 0 {
                        Some(ns)
                    } else if is_v10_model && self.tool_obs_range.is_none() && ns + 1 < copy_limit {
                        let ns_piece = bpe.decode(&[ns_tok as u32]);
                        let next_gate = model.copy_tok_gate[self.tokens[ns + 1] as usize];
                        if next_gate >= 96
                            && matches!(
                                ns_piece.as_str(),
                                "|" | "&" | "=" | "?" | "~" | "+" | "-" | "/" | "."
                            )
                        {
                            bridged_uncalibrated_word = true;
                            return Some(ns);
                        }
                        let rest = ns_piece.strip_prefix(' ')?;
                        if !rest.is_empty()
                            && !matches!(
                                ns_piece.as_str(),
                                " into"
                                    | " inside"
                                    | " and"
                                    | " package"
                                    | " summarize"
                                    | " report"
                            )
                            && rest.chars().all(|ch| {
                                ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')
                            })
                            && next_gate >= 160
                        {
                            bridged_uncalibrated_word = true;
                            Some(ns)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        });

        const CODE_PROSE_STOPWORDS: &[&str] = &[
            "a",
            "an",
            "the",
            "for",
            "in",
            "on",
            "to",
            "of",
            "and",
            "or",
            "is",
            "are",
            "with",
            "from",
            "by",
            "write",
            "given",
            "compute",
            "computing",
            "function",
            "concise",
            "rust",
            "python",
            "javascript",
            "typescript",
            "freestanding",
            "c",
            "long",
            "signed",
            "i64",
            "u64",
            "usize",
            "f64",
            "let",
            "const",
            "return",
            "pub",
            "fn",
            "def",
            "microvm",
            "rv64imac",
            "sandbox",
            "linux",
            "expression",
            "combining",
            "adding",
            "summing",
            "that",
            "setting",
            "snippet",
        ];
        const MVM_PKG_NAMES: &[&str] = &[
            "jq",
            "rg",
            "tree",
            "xxd",
            "sha256sum",
            "bc",
            "wc",
            "head",
            "tail",
            "http",
        ];
        let mut prompt_boost_anchor = vec![false; t + 1];
        let mut code_prose_blocked = vec![false; t + 1];
        if is_v10_model && in_structured_gen && self.tool_obs_range.is_none() {
            let p_lo = self.copy_start.min(copy_limit);
            for s in p_lo..copy_limit {
                let piece_s = bpe.decode(&[self.tokens[s]]);
                let trimmed_s = piece_s.trim_start();
                if trimmed_s.is_empty() {
                    continue;
                }
                let lower_s = trimmed_s.to_ascii_lowercase();
                if in_code_gen {
                    const CODE_TYPE_KEYWORDS: &[&str] = &[
                        "i64", "u64", "i32", "u32", "i16", "u16", "i8", "u8", "usize", "isize",
                        "f64", "f32", "bool", "str", "string", "long", "int", "short", "char",
                        "void", "const", "let", "var", "mut", "pub", "fn", "def", "return",
                        "unsigned", "signed", "uint64_t", "int64_t", "size_t",
                    ];
                    let prev_continues_ident =
                        !piece_s.starts_with(char::is_whitespace) && s > p_lo && {
                            let pp = bpe.decode(&[self.tokens[s - 1]]);
                            pp.chars()
                                .last()
                                .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                        };
                    if prev_continues_ident
                        || !trimmed_s
                            .chars()
                            .next()
                            .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
                        || !trimmed_s
                            .chars()
                            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                    {
                        code_prose_blocked[s] = true;
                    } else {
                        let mut full_ident = trimmed_s.to_string();
                        let mut u = s + 1;
                        while u < copy_limit {
                            let pu = bpe.decode(&[self.tokens[u]]);
                            if !pu.is_empty()
                                && !pu.starts_with(char::is_whitespace)
                                && pu.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                            {
                                full_ident.push_str(&pu);
                                u += 1;
                            } else {
                                break;
                            }
                        }
                        let full_lower = full_ident.to_ascii_lowercase();
                        if CODE_PROSE_STOPWORDS.contains(&full_lower.as_str())
                            || CODE_TYPE_KEYWORDS.contains(&full_lower.as_str())
                        {
                            code_prose_blocked[s] = true;
                        } else {
                            let next_is_ident_delim = u < copy_limit && {
                                let np = bpe.decode(&[self.tokens[u]]);
                                np.starts_with(['(', ':', '='])
                                    || np.trim_start().starts_with([':', '='])
                            };
                            if full_ident.contains('_') || next_is_ident_delim {
                                prompt_boost_anchor[s] = true;
                            }
                        }
                    }
                } else if in_agent_gen && MVM_PKG_NAMES.contains(&lower_s.as_str()) {
                    prompt_boost_anchor[s] = true;
                }
            }
        }

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
                if is_v10_model {
                    if expected_cont_s.is_none()
                        || (!prev_was_copied && in_causal_entry_span.is_none())
                    {
                        0
                    } else if bridged_uncalibrated_word {
                        8_192
                    } else if in_tool_obs_span.is_some()
                        || ((in_code_gen || in_agent_gen) && expected_cont_s.is_some())
                    {
                        gate_sum.max(6_144).clamp(0, 8_192)
                    } else if gate_sum <= 0 {
                        0
                    } else {
                        gate_sum.clamp(0, 8_192)
                    }
                } else if !prev_was_copied || expected_cont_s.is_none() {
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
            let sk_blocks = if d >= 256 { d / 64 } else { 1 };
            let h_blk = if d >= 256 { h / 8 } else { 0 };
            let q_byte = ((self.last_top_q_sk[h_blk] >> ((h & 7) * 8)) & 0xFF) as u8;

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
                let k_byte =
                    ((self.k_sk[top_l][ks * sk_blocks + h_blk] >> ((h & 7) * 8)) & 0xFF) as u8;
                let d_ham = (q_byte ^ k_byte).count_ones() as i32;
                let t_is_anchor = model.copy_tok_gate[self.tokens[t] as usize] >= 208;
                let d_ind = if sh == 1 {
                    if !is_v10_model && s == t {
                        4_096
                    } else if let Some(ns) = expected_cont_s {
                        if s == ns {
                            0
                        } else {
                            65_536
                        }
                    } else if s == t {
                        4_096
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
                    if !is_v10_model && s == t {
                        4_096
                    } else if let Some(ns) = expected_cont_s {
                        if s == ns {
                            0
                        } else {
                            65_536
                        }
                    } else if s == t {
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
                    } else if s > 0 && s < t && t_is_anchor && self.tokens[s - 1] == self.tokens[t]
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
                        && !code_prose_blocked[s]
                        && (model.copy_tok_gate[self.tokens[s] as usize] >= 208
                            || prompt_boost_anchor[s])
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
                let w = prll_weight_order4(!is_v10_model, tau, d_tot);
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
                overall_winning_head = h as u8;
            }

            if copy_gain > 0 {
                let (scan_lo, scan_hi) = if is_v10_model {
                    if let Some((obs_lo, obs_hi)) = self.tool_obs_range {
                        (obs_lo.min(copy_limit), obs_hi.min(copy_limit))
                    } else {
                        (self.copy_start.min(copy_limit), copy_limit)
                    }
                } else {
                    (self.copy_start.min(copy_limit), copy_limit)
                };
                let cur_ended_delim =
                    if is_v10_model && in_agent_gen && self.tool_obs_range.is_none() && sh == 3 {
                        bpe.decode(&[cur_tok])
                            .ends_with(['"', ':', '=', ' ', '\n', '/'])
                    } else {
                        true
                    };
                for s in scan_lo..scan_hi {
                    let tok_s = self.tokens[s] as usize;
                    if self.tokens[scan_lo..s].contains(&(tok_s as u32)) {
                        continue;
                    }
                    if is_v10_model {
                        if in_code_gen && sh == 3 {
                            let piece_s = bpe.decode(&[tok_s as u32]);
                            let trimmed_s = piece_s.trim_start();
                            if trimmed_s.is_empty()
                                || !trimmed_s
                                    .chars()
                                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                            {
                                continue;
                            }
                        } else if in_agent_gen
                            && self.tool_obs_range.is_none()
                            && sh == 3
                            && !cur_ended_delim
                            && !bpe.decode(&[tok_s as u32]).starts_with(' ')
                        {
                            continue;
                        }
                    }
                    let mut tot_wp = 0u64;
                    let mut argmax_s = s;
                    let mut max_wp = 0u32;
                    let mut any_unblocked = false;
                    let mut any_boost_anchor = false;
                    for (s2, &pw) in prll_w.iter().enumerate().take(scan_hi).skip(s) {
                        if self.tokens[s2] as usize == tok_s {
                            if is_v10_model && in_code_gen && sh == 3 && code_prose_blocked[s2] {
                                continue;
                            }
                            any_unblocked = true;
                            if prompt_boost_anchor[s2] {
                                any_boost_anchor = true;
                            }
                            tot_wp += pw as u64;
                            if pw >= max_wp {
                                max_wp = pw;
                                argmax_s = s2;
                            }
                        }
                    }
                    if !any_unblocked {
                        continue;
                    }
                    let is_expected_tok = expected_cont_s
                        .map(|ns| self.tokens[ns] as usize == tok_s)
                        .unwrap_or(false);
                    let next_gate_min = if is_v10_model { 96 } else { 160 };
                    let boost_floor = if is_v10_model { 224 } else { 192 };
                    let tok_allow = if is_v10_model
                        && ((sh != 3 && is_expected_tok && bridged_uncalibrated_word)
                            || (in_code_gen && any_boost_anchor))
                    {
                        255i64
                    } else if (is_v10_model
                        && (any_boost_anchor || (in_code_gen && sh != 3 && is_expected_tok)))
                        || (sh != 3
                            && is_expected_tok
                            && (in_tool_obs_span.is_some()
                                || (expected_cont_s
                                    .map(|ns| {
                                        ns + 1 < copy_limit
                                            && model.copy_tok_gate[self.tokens[ns + 1] as usize]
                                                >= next_gate_min
                                    })
                                    .unwrap_or(false)
                                    && matches!(tok_s, 5 | 16 | 17))))
                    {
                        (model.copy_tok_gate[tok_s] as i64).max(boost_floor)
                    } else {
                        model.copy_tok_gate[tok_s] as i64
                    };
                    let eff_wp = if sh == 3 { max_wp as u64 } else { tot_wp };
                    let cap_h = if sh == 3 {
                        if d >= 128 {
                            11_264
                        } else {
                            12_288
                        }
                    } else if is_v10_model {
                        16_384
                    } else {
                        12_288
                    };
                    let cand = ((((eff_wp as i64) * (copy_gain as i64) * tok_allow) >> 25) as i32)
                        .clamp(0, cap_h);
                    if sh == 3 {
                        let prev_max = sh3_cands[tok_s][..(sh3_cnt[tok_s] as usize).min(8)]
                            .iter()
                            .copied()
                            .max()
                            .unwrap_or(-1);
                        let idx = (sh3_cnt[tok_s] as usize).min(7);
                        sh3_cands[tok_s][idx] = cand;
                        sh3_cnt[tok_s] = sh3_cnt[tok_s].saturating_add(1);
                        if cand >= prev_max {
                            copy_best_src[tok_s] = argmax_s;
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
                let slice = &mut sh3_cands[v][..n.min(8)];
                slice.sort_unstable();
                let mut combined = if slice.len() >= 2 {
                    (slice[slice.len() - 1] + slice[slice.len() - 2]).clamp(0, 18_432)
                } else {
                    slice[0]
                };
                let s = copy_best_src[v];
                let v_piece_ws = bpe.decode(&[v as u32]).starts_with(char::is_whitespace);
                let used_as_span_start_on_line =
                    same_line_slice.iter().enumerate().any(|(k, &tv)| {
                        tv == v as u32
                            && (k == 0
                                || v_piece_ws
                                || !bpe
                                    .decode(&[same_line_slice[k - 1]])
                                    .chars()
                                    .last()
                                    .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_'))
                    });
                if used_as_span_start_on_line {
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

        let n_blocks = d / 64;
        let h_sk: Vec<u64> = (0..n_blocks)
            .map(|b| pack_sign64(&self.last_hidden[b * 64..(b + 1) * 64]))
            .collect();
        let d_shift = n_blocks.ilog2();

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
        let gate12_g_lo = if g_cnt >= 32 { g_cnt - 6 } else { 13 };
        let block_uncopied_span_exit_alnum = is_v10_model
            && in_agent_gen
            && self.tool_obs_range.is_none()
            && prev_was_copied
            && (expected_cont_s.is_none() || bridged_uncalibrated_word);
        let (in_terminal_json_val, close_braces_needed, close_xml_prefix, cmd_argv0_prefix): (
            bool,
            i32,
            Option<String>,
            Option<String>,
        ) = if is_v10_model && in_agent_gen && self.tool_obs_range.is_none() {
            let gen_full = bpe.decode(generated_so_far);
            let mut brace_depth = 0i32;
            let mut saw_open_brace = false;
            let mut outer_close_end: Option<usize> = None;
            let mut in_str = false;
            let mut escaped = false;
            for (b_idx, ch) in gen_full.char_indices() {
                if in_str {
                    if escaped {
                        escaped = false;
                    } else if ch == '\\' {
                        escaped = true;
                    } else if ch == '"' {
                        in_str = false;
                    }
                } else {
                    match ch {
                        '"' => in_str = true,
                        '{' => {
                            saw_open_brace = true;
                            brace_depth += 1;
                        }
                        '}' => {
                            brace_depth = (brace_depth - 1).max(0);
                            if saw_open_brace && brace_depth == 0 && outer_close_end.is_none() {
                                outer_close_end = Some(b_idx + ch.len_utf8());
                            }
                        }
                        _ => {}
                    }
                }
            }
            let xml_pfx = outer_close_end.map(|end_idx| gen_full[end_idx..].to_string());
            let tail_opt = [
                ("\"cmd\":\"", true),
                ("\"query\":\"", false),
                ("\"url\":\"", false),
            ]
            .iter()
            .filter_map(|(pat, is_cmd)| {
                gen_full
                    .rfind(pat)
                    .map(|idx| (&gen_full[idx + pat.len()..], *is_cmd))
            })
            .max_by_key(|(s, _)| usize::MAX - s.len());
            if let Some((val_tail, is_cmd)) = tail_opt {
                let in_val = in_str && !val_tail.contains('"');
                let close_state = if !in_str && val_tail.contains('"') && brace_depth > 0 {
                    brace_depth
                } else {
                    0
                };
                let argv0 = if is_cmd && in_val && !val_tail.is_empty() && !val_tail.contains(' ') {
                    Some(val_tail.to_string())
                } else {
                    None
                };
                (in_val, close_state, xml_pfx, argv0)
            } else {
                (false, 0, xml_pfx, None)
            }
        } else {
            (false, 0, None, None)
        };
        const MVM_CMD_BUILTINS: &[&str] = &[
            "uname",
            "echo",
            "printf",
            "pkg",
            "git",
            "curl",
            "wget",
            "nslookup",
            "dig",
            "ls",
            "cat",
            "pwd",
            "cd",
            "mkdir",
            "rm",
            "cp",
            "mv",
            "touch",
            "grep",
            "rg",
            "jq",
            "tree",
            "xxd",
            "sha256sum",
            "bc",
            "wc",
            "head",
            "tail",
            "http",
            "cc",
            "rustc",
            "python3",
            "node",
            "sh",
            "env",
            "export",
            "true",
            "false",
            "test",
            "sed",
            "awk",
            "sort",
            "uniq",
            "find",
            "tar",
            "gzip",
            "chmod",
            "stat",
            "id",
            "whoami",
            "date",
            "ps",
            "free",
            "df",
            "du",
            "clone",
            "status",
            "diff",
            "log",
            "add",
            "commit",
            "push",
            "pull",
            "init",
        ];
        let block_uncopied_lead_space =
            if is_v10_model && in_agent_gen && self.tool_obs_range.is_none() {
                if self.tokens[t] == 65 {
                    true
                } else if let Some(q_pos) = generated_so_far
                    .iter()
                    .rposition(|&tok| tok == 4 || tok == 638)
                {
                    let after_q = bpe.decode(&generated_so_far[q_pos + 1..]);
                    (1..=2).contains(&after_q.len())
                        && after_q.chars().all(|ch| ch.is_ascii_alphabetic())
                } else {
                    false
                }
            } else {
                false
            };

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
            if is_v10_model {
                if let Some(ref xml_so_far) = close_xml_prefix {
                    let piece = bpe.decode(&[v as u32]);
                    if piece.is_empty()
                        || !"</tool_call>".starts_with(&format!("{xml_so_far}{piece}"))
                    {
                        continue;
                    }
                } else if close_braces_needed > 0 {
                    let piece = bpe.decode(&[v as u32]);
                    if piece.is_empty()
                        || !piece.chars().all(|ch| ch == '}')
                        || (piece.len() as i32) > close_braces_needed
                    {
                        continue;
                    }
                }
            }
            if copy_logit == 0 && is_v10_model {
                if block_uncopied_lead_space && bpe.decode(&[v as u32]).starts_with(' ') {
                    continue;
                }
                if let Some(ref pfx) = cmd_argv0_prefix {
                    let piece = bpe.decode(&[v as u32]);
                    let cand_cmd = format!("{pfx}{piece}");
                    let word_part = cand_cmd.split([' ', '"']).next().unwrap_or("");
                    let valid_builtin = if cand_cmd.contains([' ', '"']) {
                        MVM_CMD_BUILTINS.contains(&word_part)
                    } else {
                        MVM_CMD_BUILTINS.iter().any(|b| b.starts_with(word_part))
                    };
                    if !valid_builtin {
                        continue;
                    }
                }
                if block_uncopied_span_exit_alnum {
                    if in_terminal_json_val && v == 638 {
                        continue;
                    }
                    if bpe
                        .decode(&[v as u32])
                        .trim_start()
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_alphanumeric())
                    {
                        continue;
                    }
                }
            }
            let is_untouched_copy_cal = copy_logit == 0
                && is_v10_model
                && hi_row[gate12_g_lo..g_cnt].iter().all(|&c| c == 40)
                && lo_row[gate12_g_lo..g_cnt].iter().all(|&c| c == 80);
            let active_g = if is_untouched_copy_cal {
                gate12_g_lo
            } else {
                g_cnt
            };
            let mut tmac_sum = 0i32;
            for g in 0..active_g {
                let v_hi = act_lut[g][hi_row[g] as usize];
                let v_lo = act_lut[g][lo_row[g] as usize];
                tmac_sum += (v_hi << 1) + v_hi + v_lo;
            }

            let p_row = &model.p_out[v * d..(v + 1) * d];
            let mut l1 = 0i32;
            for (c, &pv) in p_row.iter().enumerate().take(d) {
                l1 += ((self.last_hidden[c] as i32) - (pv as i32)).abs();
            }
            let p_sk_row = &model.p_sk[v * n_blocks..(v + 1) * n_blocks];
            let mut ham = 0i32;
            for b in 0..n_blocks {
                ham += (h_sk[b] ^ p_sk_row[b]).count_ones() as i32;
            }
            let raw_b_val = model.b_out[v] as i32;
            let b_val = if is_untouched_copy_cal {
                (raw_b_val - 400).min(raw_b_val)
            } else {
                raw_b_val
            };
            let raw_pure_score = (tmac_sum >> d_shift) + (b_val * 2)
                - ((ham * 24) >> d_shift)
                - (l1 >> (5 + d_shift));
            let pure_score = if is_v10_model
                && copy_logit >= 2_048
                && expected_cont_s
                    .map(|ns| self.tokens[ns] as usize == v)
                    .unwrap_or(false)
            {
                if bridged_uncalibrated_word {
                    raw_pure_score.max(8_192)
                } else if in_tool_obs_span.is_some() {
                    raw_pure_score.max(if prev_was_copied && d >= 128 {
                        12_288
                    } else {
                        0
                    })
                } else {
                    raw_pure_score
                }
            } else {
                raw_pure_score
            };
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
            && (!is_v10_model || in_causal_entry_span.is_none())
            && best_v == max_pure_v
            && best_winner_pure > -1_024
            && best_winner_pure > second_pure_overall.saturating_add(512);
        let next_copy_src = if best_copy >= 1_024 && !won_by_vocab_readout {
            Some(copy_best_src[best_v as usize])
        } else {
            None
        };

        PredictOutcome {
            token_id: best_v,
            best_score,
            copy_score: best_copy,
            pure_margin: best_winner_pure.saturating_sub(second_pure_overall),
            next_copy_src,
            top_attended_pos: overall_top_s,
            winning_head: overall_winning_head,
            top_mass: overall_top_mass.min(GLLM_V9_PRLL_QUANTUM as u32),
            mass_ok,
        }
    }
}
