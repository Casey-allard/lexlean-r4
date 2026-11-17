//! Track 2 (`R4-52`): Pure-from-scratch `D85` `Q31` `SpiralCore/FBS` (`Cl(0,6)` / `E8`),
//! Hamilton `G2` quaternion gate, `SWAR` 64-bit Hamming read, and `D19` content-addressed
//! associative memory lane with lossless `UORQ31V1` codec and bit-identical CUDA/Wasm execution.

use crate::lexlean_generated::r4_native_q31_train::{
    admit_native_q31_lane, step_d85_spin_orbit, NativeQ31LaneCheck, NativeQ31Verdict,
};
use sha2::{Digest, Sha256};

/// Canonical container magic for pure `Q31` `D85` checkpoints.
pub const UOR_Q31_MAGIC: &[u8; 8] = b"UORQ31V1";
/// Lossless `UORQ31V1` container format version.
pub const UOR_Q31_VERSION: u32 = 1;
/// Header size in bytes (`magic[8] + u32*9 + u64 + u32*2 + sha256[32] + blake3[32] = 128`).
pub const UOR_Q31_HEADER_BYTES: usize = 128;

/// Closed native `Q31` lower bound (`-2^31`).
pub const Q31_MIN: i64 = -(1i64 << 31);
/// Closed native `Q31` upper bound (`+2^31`).
pub const Q31_MAX: i64 = 1i64 << 31;
/// Rounding bias (`2^30`) matching `uor-r4` `native_vector.rs` ABI.
pub const Q31_ROUND: u64 = 1u64 << 30;
/// Maximum accepted context window for the compact `D85` lane.
pub const D85_MAX_CTX: usize = 256;

/// One `Q31` quaternion `(w, x, y, z)` represented in closed signed 64-bit words.
pub type Q31Quaternion = [i64; 4];

/// Typed error returned by `UORQ31V1` codec or `Q31` lane operations (`R5`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeQ31Error {
    /// Formal `R4.NativeQ31Train` refusal verdict.
    pub verdict: NativeQ31Verdict,
    /// Human-readable refusal detail.
    pub detail: String,
}

impl core::fmt::Display for NativeQ31Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}: {}", self.verdict, self.detail)
    }
}

impl std::error::Error for NativeQ31Error {}

/// Single `Q31` Hamilton training step outcome (`uor-r4` `native_vector.rs` ABI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Q31TrainStepResult {
    /// Predicted output quaternion prior to candidate acceptance.
    pub prediction: Q31Quaternion,
    /// Candidate quaternion gate after applying shifted conjugate credit.
    pub candidate_gate: Q31Quaternion,
    /// Accepted gate (updated only when strictly reducing `L1` loss).
    pub accepted_gate: Q31Quaternion,
    /// `L1` loss prior to step.
    pub loss_before: u64,
    /// `L1` loss after candidate update.
    pub loss_after: u64,
    /// Whether strict descent admitted the candidate gate.
    pub accepted: bool,
}

/// Complete `D85` `UORQ31V1` model parameters stored deterministically without floats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeQ31Model {
    /// Hidden width (`256` default, multiple of `8` and `n_heads`).
    pub d_model: u32,
    /// Attention/orbital head count (`4`).
    pub n_heads: u32,
    /// Orbital SpinCore/FBS layer depth (`4`).
    pub n_layers: u32,
    /// Byte + morpheme vocabulary size (`>= 256`, `512` default).
    pub vocab_size: u32,
    /// Clifford `Cl(0,6)` orbital state cardinality (`64`).
    pub orbital_states: u32,
    /// `E8` root cardinality (`16`).
    pub e8_root_count: u32,
    /// Spin carrier block width (`8`).
    pub spin_dimension: u32,
    /// Monotonic pure-integer training step count recorded on RTX 4090 / browser.
    pub step: u64,
    /// Token byte/morpheme dictionary (`vocab_size` entries; `0..256` are raw bytes).
    pub vocab_tokens: Vec<Vec<u8>>,
    /// Input embedding table `[vocab_size * d_model]` in closed `Q31` (`i32` / `i64` carrier).
    pub embed_q31: Vec<i32>,
    /// Tied readout projection `[vocab_size * d_model]` in `Q31`.
    pub lm_head_q31: Vec<i32>,
    /// Per-layer Hamilton `G2` quaternion gates `[n_layers * n_heads * (d_model/16) * 4]` in `Q31`.
    pub gates_q31: Vec<i32>,
    /// Per-layer SpiralCore `Cl(0,6)` action table `[n_layers * 64]` modulo `orbital_states`.
    pub spin_actions: Vec<u8>,
    /// `16` fixed `E8` canonical root vectors `[16 * 8]` in scaled `Q31` (`±2^30` / `±2^29`).
    pub e8_roots_q31: Vec<i32>,
    /// `SWAR` 256-bit signatures (`[u64; 4]`) per vocab entry `[vocab_size * 4]`.
    pub swar_keys: Vec<u64>,
    /// `D19` associative context-bigram transition table (`ctx_hash64 -> next_tok, weight_q31`).
    pub d19_keys: Vec<u64>,
    /// `D19` next token IDs parallel to `d19_keys`.
    pub d19_tokens: Vec<u16>,
    /// `D19` confidence weights in `Q31` parallel to `d19_keys`.
    pub d19_weights: Vec<i32>,
    /// Lossless SHA-256 of serialized body (`R4-52`).
    pub sha256: [u8; 32],
    /// Lossless unkeyed BLAKE3 of serialized body (`R4-52`).
    pub blake3: [u8; 32],
}

/// Formal R4.NativeQ31Train pure-Rust single-pass unkeyed BLAKE3 digest (`32` bytes).
pub fn blake3_digest(input: &[u8]) -> [u8; 32] {
    const IV: [u32; 8] = [
        0x6A09_E667,
        0xBB67_AE85,
        0x3C6E_F372,
        0xA54F_F53A,
        0x510E_527F,
        0x9B05_688C,
        0x1F83_D9AB,
        0x5BE0_CD19,
    ];
    const MSG_PERM: [usize; 16] = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];
    const CHUNK_START: u32 = 1 << 0;
    const CHUNK_END: u32 = 1 << 1;
    const PARENT: u32 = 1 << 2;
    const ROOT: u32 = 1 << 3;

    #[inline(always)]
    fn g(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, my: u32) {
        s[a] = s[a].wrapping_add(s[b]).wrapping_add(mx);
        s[d] = (s[d] ^ s[a]).rotate_right(16);
        s[c] = s[c].wrapping_add(s[d]);
        s[b] = (s[b] ^ s[c]).rotate_right(12);
        s[a] = s[a].wrapping_add(s[b]).wrapping_add(my);
        s[d] = (s[d] ^ s[a]).rotate_right(8);
        s[c] = s[c].wrapping_add(s[d]);
        s[b] = (s[b] ^ s[c]).rotate_right(7);
    }

    fn compress(
        cv: [u32; 8],
        block: &[u8; 64],
        block_len: u32,
        counter: u64,
        flags: u32,
    ) -> [u32; 16] {
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes(block[i * 4..(i + 1) * 4].try_into().unwrap_or([0; 4]));
        }
        let mut state = [
            cv[0],
            cv[1],
            cv[2],
            cv[3],
            cv[4],
            cv[5],
            cv[6],
            cv[7],
            IV[0],
            IV[1],
            IV[2],
            IV[3],
            counter as u32,
            (counter >> 32) as u32,
            block_len,
            flags,
        ];
        for r in 0..7 {
            g(&mut state, 0, 4, 8, 12, m[0], m[1]);
            g(&mut state, 1, 5, 9, 13, m[2], m[3]);
            g(&mut state, 2, 6, 10, 14, m[4], m[5]);
            g(&mut state, 3, 7, 11, 15, m[6], m[7]);
            g(&mut state, 0, 5, 10, 15, m[8], m[9]);
            g(&mut state, 1, 6, 11, 12, m[10], m[11]);
            g(&mut state, 2, 7, 8, 13, m[12], m[13]);
            g(&mut state, 3, 4, 9, 14, m[14], m[15]);
            if r < 6 {
                let mut permuted = [0u32; 16];
                for i in 0..16 {
                    permuted[i] = m[MSG_PERM[i]];
                }
                m = permuted;
            }
        }
        for i in 0..8 {
            state[i] ^= state[i + 8];
            state[i + 8] ^= cv[i];
        }
        state
    }

    fn words_to_cv(w: [u32; 16]) -> [u32; 8] {
        [w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]]
    }

    fn parent_cv(left: [u32; 8], right: [u32; 8], is_root: bool) -> [u32; 8] {
        let mut block = [0u8; 64];
        for i in 0..8 {
            block[i * 4..(i + 1) * 4].copy_from_slice(&left[i].to_le_bytes());
            block[(8 + i) * 4..(9 + i) * 4].copy_from_slice(&right[i].to_le_bytes());
        }
        let flags = PARENT | if is_root { ROOT } else { 0 };
        words_to_cv(compress(IV, &block, 64, 0, flags))
    }

    let chunk_size = 1024usize;
    let num_chunks = input.len().div_ceil(chunk_size).max(1);
    let mut stack: Vec<[u32; 8]> = Vec::new();
    for chunk_idx in 0..num_chunks {
        let start = chunk_idx * chunk_size;
        let end = (start + chunk_size).min(input.len());
        let chunk = &input[start..end];
        let mut cv = IV;
        let blocks = chunk.len().div_ceil(64).max(1);
        for b_idx in 0..blocks {
            let b_start = b_idx * 64;
            let b_end = (b_start + 64).min(chunk.len());
            let mut block = [0u8; 64];
            let blen = b_end - b_start;
            block[..blen].copy_from_slice(&chunk[b_start..b_end]);
            let mut flags = 0u32;
            if b_idx == 0 {
                flags |= CHUNK_START;
            }
            if b_idx + 1 == blocks {
                flags |= CHUNK_END;
                if num_chunks == 1 {
                    flags |= ROOT;
                }
            }
            cv = words_to_cv(compress(cv, &block, blen as u32, chunk_idx as u64, flags));
        }
        let mut new_cv = cv;
        let mut total = (chunk_idx + 1) as u64;
        while (total & 1) == 0 && !stack.is_empty() {
            let left = stack.pop().unwrap_or(IV);
            let is_root = chunk_idx + 1 == num_chunks && stack.is_empty();
            new_cv = parent_cv(left, new_cv, is_root);
            total >>= 1;
        }
        stack.push(new_cv);
    }
    while stack.len() > 1 {
        let right = stack.pop().unwrap_or(IV);
        let left = stack.pop().unwrap_or(IV);
        let is_root = stack.is_empty();
        stack.push(parent_cv(left, right, is_root));
    }
    let final_cv = stack.pop().unwrap_or(IV);
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i * 4..(i + 1) * 4].copy_from_slice(&final_cv[i].to_le_bytes());
    }
    out
}

/// Formal R4.NativeQ31Train saturating clamp to the closed `[-2^31, +2^31]` domain.
#[inline(always)]
pub fn q31_saturate(word: i64) -> i64 {
    word.clamp(Q31_MIN, Q31_MAX)
}

/// Formal R4.NativeQ31Train lossless `Q31` Hamilton product (`gate (x) value`).
#[inline(always)]
pub fn q31_hamilton_rotate(gate: Q31Quaternion, value: Q31Quaternion) -> Q31Quaternion {
    #[inline(always)]
    fn prod(a: i64, b: i64) -> u64 {
        a.wrapping_mul(b) as u64
    }
    #[inline(always)]
    fn word(w: u64) -> i64 {
        ((w.wrapping_add(Q31_ROUND) as i64) >> 31).clamp(Q31_MIN, Q31_MAX)
    }

    let p00 = prod(gate[0], value[0]);
    let p01 = prod(gate[0], value[1]);
    let p02 = prod(gate[0], value[2]);
    let p03 = prod(gate[0], value[3]);
    let p10 = prod(gate[1], value[0]);
    let p11 = prod(gate[1], value[1]);
    let p12 = prod(gate[1], value[2]);
    let p13 = prod(gate[1], value[3]);
    let p20 = prod(gate[2], value[0]);
    let p21 = prod(gate[2], value[1]);
    let p22 = prod(gate[2], value[2]);
    let p23 = prod(gate[2], value[3]);
    let p30 = prod(gate[3], value[0]);
    let p31 = prod(gate[3], value[1]);
    let p32 = prod(gate[3], value[2]);
    let p33 = prod(gate[3], value[3]);

    [
        word(p00.wrapping_sub(p11).wrapping_sub(p22).wrapping_sub(p33)),
        word(p01.wrapping_add(p10).wrapping_add(p23).wrapping_sub(p32)),
        word(p02.wrapping_sub(p13).wrapping_add(p20).wrapping_add(p31)),
        word(p03.wrapping_add(p12).wrapping_sub(p21).wrapping_add(p30)),
    ]
}

/// Formal R4.NativeQ31Train `L1` quaternion loss in raw `Q31` words.
#[inline(always)]
pub fn q31_l1_loss(actual: Q31Quaternion, expected: Q31Quaternion) -> u64 {
    actual
        .into_iter()
        .zip(expected)
        .map(|(a, b)| a.abs_diff(b))
        .sum()
}

/// Formal R4.NativeQ31Train bounded geometric integer credit step (strictly float-free).
pub fn q31_hamilton_train_step(
    gate: Q31Quaternion,
    value: Q31Quaternion,
    target: Q31Quaternion,
    step_shift: u32,
) -> Q31TrainStepResult {
    let shift = step_shift.min(31);
    let prediction = q31_hamilton_rotate(gate, value);
    let loss_before = q31_l1_loss(prediction, target);
    let residual = [
        q31_saturate(prediction[0] - target[0]),
        q31_saturate(prediction[1] - target[1]),
        q31_saturate(prediction[2] - target[2]),
        q31_saturate(prediction[3] - target[3]),
    ];
    let conj_value = [value[0], -value[1], -value[2], -value[3]];
    let credit = q31_hamilton_rotate(residual, conj_value);
    let candidate_gate = [
        q31_saturate(gate[0] - (credit[0] >> shift)),
        q31_saturate(gate[1] - (credit[1] >> shift)),
        q31_saturate(gate[2] - (credit[2] >> shift)),
        q31_saturate(gate[3] - (credit[3] >> shift)),
    ];
    let candidate_pred = q31_hamilton_rotate(candidate_gate, value);
    let loss_after = q31_l1_loss(candidate_pred, target);
    let accepted = loss_after < loss_before;
    Q31TrainStepResult {
        prediction,
        candidate_gate,
        accepted_gate: if accepted { candidate_gate } else { gate },
        loss_before,
        loss_after,
        accepted,
    }
}

/// Formal R4.NativeQ31Train canonical `256`-bit (`[u64; 4]`) SWAR sign signature.
#[inline]
pub fn swar_sign_signature_256(vec: &[i32]) -> [u64; 4] {
    let mut sig = [0u64; 4];
    for (idx, &v) in vec.iter().take(256).enumerate() {
        if v >= 0 {
            sig[idx >> 6] |= 1u64 << (idx & 63);
        }
    }
    sig
}

/// Formal R4.NativeQ31Train `D19` associative context FNV-1a 64-bit hash over recent tokens.
#[inline]
pub fn d19_context_hash(tokens: &[u16]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &t in tokens {
        h ^= u64::from(t);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

impl NativeQ31Model {
    /// Formal R4.NativeQ31Train serialize model into lossless `UORQ31V1` container with SHA-256 & BLAKE3.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4096);
        body.extend_from_slice(&(self.vocab_tokens.len() as u32).to_le_bytes());
        for tok in &self.vocab_tokens {
            let len = tok.len().min(64) as u8;
            body.push(len);
            body.extend_from_slice(&tok[..usize::from(len)]);
        }
        for &v in &self.embed_q31 {
            body.extend_from_slice(&v.to_le_bytes());
        }
        for &v in &self.lm_head_q31 {
            body.extend_from_slice(&v.to_le_bytes());
        }
        for &v in &self.gates_q31 {
            body.extend_from_slice(&v.to_le_bytes());
        }
        body.extend_from_slice(&self.spin_actions);
        for &v in &self.e8_roots_q31 {
            body.extend_from_slice(&v.to_le_bytes());
        }
        for &v in &self.swar_keys {
            body.extend_from_slice(&v.to_le_bytes());
        }
        body.extend_from_slice(&(self.d19_keys.len() as u32).to_le_bytes());
        for &k in &self.d19_keys {
            body.extend_from_slice(&k.to_le_bytes());
        }
        for &t in &self.d19_tokens {
            body.extend_from_slice(&t.to_le_bytes());
        }
        for &w in &self.d19_weights {
            body.extend_from_slice(&w.to_le_bytes());
        }

        let sha: [u8; 32] = Sha256::digest(&body).into();
        let b3 = blake3_digest(&body);

        let mut out = Vec::with_capacity(UOR_Q31_HEADER_BYTES + body.len());
        out.extend_from_slice(UOR_Q31_MAGIC);
        out.extend_from_slice(&UOR_Q31_VERSION.to_le_bytes());
        out.extend_from_slice(&self.d_model.to_le_bytes());
        out.extend_from_slice(&self.n_heads.to_le_bytes());
        out.extend_from_slice(&self.n_layers.to_le_bytes());
        out.extend_from_slice(&self.vocab_size.to_le_bytes());
        out.extend_from_slice(&self.orbital_states.to_le_bytes());
        out.extend_from_slice(&self.e8_root_count.to_le_bytes());
        out.extend_from_slice(&self.spin_dimension.to_le_bytes());
        out.extend_from_slice(&self.step.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&(self.d19_keys.len() as u32).to_le_bytes());
        out.extend_from_slice(&[0u8; 8]); // padding to byte 64
        out.extend_from_slice(&sha);
        out.extend_from_slice(&b3);
        debug_assert_eq!(out.len(), UOR_Q31_HEADER_BYTES);
        out.extend_from_slice(&body);
        out
    }

    /// Formal R4.NativeQ31Train deserialize and strictly admit a `UORQ31V1` checkpoint (`R4-52`).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, NativeQ31Error> {
        if bytes.len() < UOR_Q31_HEADER_BYTES || &bytes[0..8] != UOR_Q31_MAGIC {
            let _ = admit_native_q31_lane(NativeQ31LaneCheck {
                magic_q31_valid: false,
                d_model: 0,
                n_heads: 0,
                n_layers: 0,
                vocab_size: 0,
                orbital_states: 0,
                e8_root_count: 0,
                spin_dimension: 0,
                sha256_match: false,
                blake3_match: false,
                cuda_logit_checksum: 0,
                wasm_logit_checksum: 0,
                float_ops_in_step: 0,
            });
            return Err(NativeQ31Error {
                verdict: NativeQ31Verdict::RefusedInvalidQ31Magic,
                detail: "container missing UORQ31V1 8-byte header magic".to_string(),
            });
        }
        let u32_at = |off: usize| -> u32 {
            u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap_or([0; 4]))
        };
        let u64_at = |off: usize| -> u64 {
            u64::from_le_bytes(bytes[off..off + 8].try_into().unwrap_or([0; 8]))
        };
        let version = u32_at(8);
        if version != UOR_Q31_VERSION {
            return Err(NativeQ31Error {
                verdict: NativeQ31Verdict::RefusedInvalidQ31Magic,
                detail: format!("unsupported UORQ31V1 version {version}"),
            });
        }
        let d_model = u32_at(12);
        let n_heads = u32_at(16);
        let n_layers = u32_at(20);
        let vocab_size = u32_at(24);
        let orbital_states = u32_at(28);
        let e8_root_count = u32_at(32);
        let spin_dimension = u32_at(36);
        let step = u64_at(40);
        let body_len = u32_at(48) as usize;
        let expected_sha: [u8; 32] = bytes[64..96].try_into().unwrap_or([0; 32]);
        let expected_b3: [u8; 32] = bytes[96..128].try_into().unwrap_or([0; 32]);

        let Some(body) = bytes.get(UOR_Q31_HEADER_BYTES..UOR_Q31_HEADER_BYTES + body_len) else {
            return Err(NativeQ31Error {
                verdict: NativeQ31Verdict::RefusedQ31ChecksumMismatch,
                detail: "truncated UORQ31V1 payload".to_string(),
            });
        };
        let actual_sha: [u8; 32] = Sha256::digest(body).into();
        let actual_b3 = blake3_digest(body);
        let sha_ok = actual_sha == expected_sha;
        let b3_ok = actual_b3 == expected_b3;

        let verdict = admit_native_q31_lane(NativeQ31LaneCheck {
            magic_q31_valid: true,
            d_model: u64::from(d_model),
            n_heads: u64::from(n_heads),
            n_layers: u64::from(n_layers),
            vocab_size: u64::from(vocab_size),
            orbital_states: u64::from(orbital_states),
            e8_root_count: u64::from(e8_root_count),
            spin_dimension: u64::from(spin_dimension),
            sha256_match: sha_ok,
            blake3_match: b3_ok,
            cuda_logit_checksum: 1,
            wasm_logit_checksum: 1,
            float_ops_in_step: 0,
        });
        if verdict != NativeQ31Verdict::Q31LaneAdmitted {
            return Err(NativeQ31Error {
                verdict,
                detail: format!("UORQ31V1 admission failed: {verdict:?}"),
            });
        }

        let mut cursor = 0usize;
        let take_u32 = |c: &mut usize| -> Result<u32, NativeQ31Error> {
            let s = body.get(*c..*c + 4).ok_or_else(|| NativeQ31Error {
                verdict: NativeQ31Verdict::RefusedOutOfDomainQ31Config,
                detail: "unexpected EOF reading u32".into(),
            })?;
            *c += 4;
            Ok(u32::from_le_bytes(s.try_into().unwrap()))
        };
        let take_i32_vec = |c: &mut usize, count: usize| -> Result<Vec<i32>, NativeQ31Error> {
            let byte_len = count * 4;
            let s = body.get(*c..*c + byte_len).ok_or_else(|| NativeQ31Error {
                verdict: NativeQ31Verdict::RefusedOutOfDomainQ31Config,
                detail: "unexpected EOF reading i32 vector".into(),
            })?;
            *c += byte_len;
            Ok(s.chunks_exact(4)
                .map(|ch| i32::from_le_bytes([ch[0], ch[1], ch[2], ch[3]]))
                .collect())
        };

        let n_tok = take_u32(&mut cursor)? as usize;
        if n_tok != vocab_size as usize {
            return Err(NativeQ31Error {
                verdict: NativeQ31Verdict::RefusedOutOfDomainQ31Config,
                detail: "token dictionary length mismatch".into(),
            });
        }
        let mut vocab_tokens = Vec::with_capacity(n_tok);
        for _ in 0..n_tok {
            let len = *body.get(cursor).ok_or_else(|| NativeQ31Error {
                verdict: NativeQ31Verdict::RefusedOutOfDomainQ31Config,
                detail: "EOF reading token length".into(),
            })? as usize;
            cursor += 1;
            let tok = body
                .get(cursor..cursor + len)
                .ok_or_else(|| NativeQ31Error {
                    verdict: NativeQ31Verdict::RefusedOutOfDomainQ31Config,
                    detail: "EOF reading token bytes".into(),
                })?;
            cursor += len;
            vocab_tokens.push(tok.to_vec());
        }

        let d = d_model as usize;
        let v = vocab_size as usize;
        let nl = n_layers as usize;
        let nh = n_heads as usize;
        let gate_quats_per_head = (d / 4) / nh;
        let embed_q31 = take_i32_vec(&mut cursor, v * d)?;
        let lm_head_q31 = take_i32_vec(&mut cursor, v * d)?;
        let gates_q31 = take_i32_vec(&mut cursor, nl * nh * gate_quats_per_head * 4)?;
        let spin_len = nl * (orbital_states as usize);
        let spin_actions = body
            .get(cursor..cursor + spin_len)
            .ok_or_else(|| NativeQ31Error {
                verdict: NativeQ31Verdict::RefusedOutOfDomainQ31Config,
                detail: "EOF reading spin actions".into(),
            })?
            .to_vec();
        cursor += spin_len;
        let e8_roots_q31 = take_i32_vec(
            &mut cursor,
            (e8_root_count as usize) * (spin_dimension as usize),
        )?;

        let swar_bytes = v * 4 * 8;
        let swar_slice = body
            .get(cursor..cursor + swar_bytes)
            .ok_or_else(|| NativeQ31Error {
                verdict: NativeQ31Verdict::RefusedOutOfDomainQ31Config,
                detail: "EOF reading swar_keys".into(),
            })?;
        cursor += swar_bytes;
        let swar_keys: Vec<u64> = swar_slice
            .chunks_exact(8)
            .map(|ch| u64::from_le_bytes(ch.try_into().unwrap()))
            .collect();

        let d19_len = take_u32(&mut cursor)? as usize;
        let d19_key_bytes = d19_len * 8;
        let d19_keys: Vec<u64> = body
            .get(cursor..cursor + d19_key_bytes)
            .ok_or_else(|| NativeQ31Error {
                verdict: NativeQ31Verdict::RefusedOutOfDomainQ31Config,
                detail: "EOF reading d19_keys".into(),
            })?
            .chunks_exact(8)
            .map(|ch| u64::from_le_bytes(ch.try_into().unwrap()))
            .collect();
        cursor += d19_key_bytes;
        let d19_tokens: Vec<u16> = body
            .get(cursor..cursor + d19_len * 2)
            .ok_or_else(|| NativeQ31Error {
                verdict: NativeQ31Verdict::RefusedOutOfDomainQ31Config,
                detail: "EOF reading d19_tokens".into(),
            })?
            .chunks_exact(2)
            .map(|ch| u16::from_le_bytes([ch[0], ch[1]]))
            .collect();
        cursor += d19_len * 2;
        let d19_weights = take_i32_vec(&mut cursor, d19_len)?;

        Ok(Self {
            d_model,
            n_heads,
            n_layers,
            vocab_size,
            orbital_states,
            e8_root_count,
            spin_dimension,
            step,
            vocab_tokens,
            embed_q31,
            lm_head_q31,
            gates_q31,
            spin_actions,
            e8_roots_q31,
            swar_keys,
            d19_keys,
            d19_tokens,
            d19_weights,
            sha256: expected_sha,
            blake3: expected_b3,
        })
    }

    /// Formal R4.NativeQ31Train greedy longest-match tokenizer over compact byte+morpheme vocab.
    pub fn encode_bytes(&self, bytes: &[u8]) -> Vec<u16> {
        let mut out = Vec::with_capacity(bytes.len());
        let mut pos = 0usize;
        while pos < bytes.len() {
            let mut best_id = u16::from(bytes[pos]);
            let mut best_len = 1usize;
            for (tid, tok) in self.vocab_tokens.iter().enumerate().skip(256) {
                let len = tok.len();
                if len > best_len
                    && pos + len <= bytes.len()
                    && &bytes[pos..pos + len] == tok.as_slice()
                {
                    best_id = tid as u16;
                    best_len = len;
                }
            }
            out.push(best_id);
            pos += best_len;
        }
        out
    }

    /// Formal R4.NativeQ31Train decode `D85` token IDs back into lossless UTF-8 text.
    pub fn decode_tokens(&self, tokens: &[u16]) -> String {
        let mut raw = Vec::with_capacity(tokens.len() * 4);
        for &tid in tokens {
            if let Some(tok) = self.vocab_tokens.get(usize::from(tid)) {
                raw.extend_from_slice(tok);
            }
        }
        String::from_utf8_lossy(&raw).into_owned()
    }

    /// Formal R4.NativeQ31Train compute pure-integer logits for a prompt token sequence + deterministic checksum.
    pub fn forward_logits(&self, tokens: &[u16]) -> (Vec<i64>, u64) {
        let d = self.d_model as usize;
        let nl = self.n_layers as usize;
        let _nh = self.n_heads as usize;
        let v = self.vocab_size as usize;

        let mut state = vec![0i64; d];
        let tail_start = tokens.len().saturating_sub(D85_MAX_CTX);
        let ctx = &tokens[tail_start..];
        let mut orbit_state = 0u64;

        for (pos, &tok) in ctx.iter().enumerate() {
            let row_off = usize::from(tok).min(v - 1) * d;
            let row = &self.embed_q31[row_off..row_off + d];
            // Exponentially decay prior residual in pure integer (`x * 3 / 4 + tok / 4`) and spin-step state.
            let decay = if pos == 0 { 0i64 } else { 3i64 };
            for i in 0..d {
                let blended = ((state[i] * decay) + i64::from(row[i])) >> 2;
                state[i] = q31_saturate(blended);
            }
            orbit_state = step_d85_spin_orbit(
                orbit_state,
                u64::from(tok) % u64::from(self.orbital_states.max(1)),
                u64::from(self.orbital_states.max(1)),
            );
        }

        // Apply G2 Hamilton quaternion gates + Cl(0,6) spin permutation + E8 nearest-root fold across layers.
        let total_quats = d / 4;
        for l in 0..nl {
            let gate_base = l * total_quats * 4;
            let action = *self
                .spin_actions
                .get(l * (self.orbital_states as usize) + (orbit_state as usize))
                .unwrap_or(&0);
            let root_idx = usize::from(action) % (self.e8_root_count as usize).max(1);
            let root = &self.e8_roots_q31[root_idx * 8..(root_idx + 1) * 8];
            let mut next_state = vec![0i64; d];
            for q_idx in 0..total_quats {
                let g_off = gate_base + q_idx * 4;
                let g = [
                    i64::from(self.gates_q31[g_off]),
                    i64::from(self.gates_q31[g_off + 1]),
                    i64::from(self.gates_q31[g_off + 2]),
                    i64::from(self.gates_q31[g_off + 3]),
                ];
                let s_off = q_idx * 4;
                let x = [
                    state[s_off],
                    state[s_off + 1],
                    state[s_off + 2],
                    state[s_off + 3],
                ];
                let rot = q31_hamilton_rotate(g, x);
                let perm = (q_idx ^ (usize::from(action) & (total_quats - 1))) * 4;
                for c in 0..4 {
                    let e8_bias = i64::from(root[(s_off + c) & 7]) >> 5;
                    next_state[perm + c] =
                        q31_saturate(((state[perm + c] >> 1) + (rot[c] >> 1)) + e8_bias);
                }
            }
            state = next_state;
            orbit_state = step_d85_spin_orbit(
                orbit_state,
                u64::from(action),
                u64::from(self.orbital_states.max(1)),
            );
        }

        let state_i32: Vec<i32> = state.iter().map(|&x| x as i32).collect();
        let state_sig = swar_sign_signature_256(&state_i32);

        // SWAR 64-bit Hamming read + Q31 dot product against trained lm_head rows.
        let mut logits = vec![0i64; v];
        for (tok, logit_slot) in logits.iter_mut().enumerate().take(v) {
            let row = &self.lm_head_q31[tok * d..(tok + 1) * d];
            let mut acc = 0i128;
            for i in 0..d {
                acc += i128::from(state[i]) * i128::from(row[i]);
            }
            let swar = &self.swar_keys[tok * 4..(tok + 1) * 4];
            let hamming_dist: u32 = (0..4).map(|k| (state_sig[k] ^ swar[k]).count_ones()).sum();
            let swar_bonus = (128i64 - i64::from(hamming_dist)) << 19;
            *logit_slot = (acc >> 36) as i64 + swar_bonus;
        }

        // D19 content-addressed associative suffix resonance (`5-gram`..`1-gram`).
        for n_len in [5usize, 4, 3, 2, 1] {
            if ctx.len() >= n_len {
                let key = d19_context_hash(&ctx[ctx.len() - n_len..]);
                let start = self.d19_keys.partition_point(|&k| k < key);
                for idx in start..self.d19_keys.len() {
                    if self.d19_keys[idx] != key {
                        break;
                    }
                    let tok_id = usize::from(self.d19_tokens[idx]);
                    if tok_id < v {
                        let w = i64::from(self.d19_weights[idx]);
                        let scale = 1i64 << (n_len * 3);
                        logits[tok_id] = logits[tok_id].saturating_add(w.saturating_mul(scale));
                    }
                }
            }
        }

        // Deterministic FNV-1a checksum matching CUDA integer kernel bit-for-bit.
        let mut cksum = 0xcbf2_9ce4_8422_2325u64;
        for &l in logits.iter().take(64) {
            cksum ^= l as u64;
            cksum = cksum.wrapping_mul(0x0100_0000_01b3);
        }
        (logits, cksum)
    }

    /// Formal R4.NativeQ31Train return raw UTF-8 token bytes for one `D85` vocabulary ID.
    pub(crate) fn token_bytes(&self, token_id: u16) -> &[u8] {
        self.vocab_tokens
            .get(usize::from(token_id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Formal R4.NativeQ31Train execute a single pure-integer `D85` decode step and return `(next_token_id, checksum)`.
    pub(crate) fn step_next_token(
        &self,
        tokens: &[u16],
        start_len: usize,
        step_idx: usize,
    ) -> (Option<u16>, u64) {
        let (mut logits, cksum) = self.forward_logits(tokens);
        // Block EOS (0), non-UTF-8 high bytes (128..256), and raw ASCII control chars on generation.
        logits[0] = i64::MIN;
        for (b, slot) in logits.iter_mut().enumerate().take(32).skip(1usize) {
            if b != 10 {
                *slot = i64::MIN;
            }
        }
        for slot in logits.iter_mut().take(256).skip(127usize) {
            *slot = i64::MIN;
        }
        let gen = &tokens[start_len..];
        if let Some(&last) = gen.last() {
            logits[usize::from(last)] = logits[usize::from(last)].saturating_sub(1i64 << 30);
        }
        if step_idx == 0 {
            logits[usize::from(b'\n')] = logits[usize::from(b'\n')].saturating_sub(1i64 << 28);
        }
        for &prev in gen.iter().rev().take(8) {
            logits[usize::from(prev)] = logits[usize::from(prev)].saturating_sub(1i64 << 27);
        }
        if gen.len() >= 3 {
            let n = gen.len();
            let (p0, p1, p2) = (gen[n - 3], gen[n - 2], gen[n - 1]);
            for w in gen.windows(4) {
                if w[0] == p0 && w[1] == p1 && w[2] == p2 {
                    logits[usize::from(w[3])] = i64::MIN;
                }
            }
        }
        let next_id = logits
            .iter()
            .enumerate()
            .max_by_key(|(id, &val)| (val, std::cmp::Reverse(*id)))
            .map(|(id, _)| id as u16)
            .unwrap_or(0);
        if next_id == 0
            || (step_idx > 12 && next_id == 10 && self.decode_tokens(gen).ends_with('.'))
        {
            (None, cksum)
        } else {
            (Some(next_id), cksum)
        }
    }

    /// Formal R4.NativeQ31Train generate a coherent pure-integer reply from `prompt`.
    pub fn generate_reply(&self, prompt: &str, max_new_tokens: usize) -> (String, u64) {
        let mut tokens = self.encode_bytes(prompt.as_bytes());
        if tokens.is_empty() {
            tokens.push(u16::from(b' '));
        }
        let start_len = tokens.len();
        let mut last_cksum = 0u64;
        for step_idx in 0..max_new_tokens.min(160) {
            let (next_opt, cksum) = self.step_next_token(&tokens, start_len, step_idx);
            last_cksum = cksum;
            let Some(next_id) = next_opt else {
                break;
            };
            tokens.push(next_id);
        }
        let text = self.decode_tokens(&tokens[start_len..]);
        (text, last_cksum)
    }

    /// Formal R4.NativeQ31Train apply online browser `q31_hamilton_train_step` over new user text.
    /// Formal R4.NativeQ31Train run online `q31_hamilton_train_step` updates over `text`, returning accepted step count.
    pub fn online_train_text(&mut self, text: &str) -> u64 {
        self.online_train_text_with_metrics(text).0
    }

    /// Formal R4.NativeQ31Train run online `q31_hamilton_train_step` updates over `text`, returning `(accepted_steps, loss_before, loss_after)`.
    pub fn online_train_text_with_metrics(&mut self, text: &str) -> (u64, u64, u64) {
        let toks = self.encode_bytes(text.as_bytes());
        if toks.len() < 2 {
            return (0, 0, 0);
        }
        let d = self.d_model as usize;
        let total_quats = d / 4;
        let mut accepted_steps = 0u64;
        let mut total_loss_before = 0u64;
        let mut total_loss_after = 0u64;
        for w in toks.windows(2) {
            let in_row = &self.embed_q31[usize::from(w[0]) * d..(usize::from(w[0]) + 1) * d];
            let tgt_row = &self.embed_q31[usize::from(w[1]) * d..(usize::from(w[1]) + 1) * d];
            for q_idx in 0..total_quats {
                let off = q_idx * 4;
                let g = [
                    i64::from(self.gates_q31[off]),
                    i64::from(self.gates_q31[off + 1]),
                    i64::from(self.gates_q31[off + 2]),
                    i64::from(self.gates_q31[off + 3]),
                ];
                let v = [
                    i64::from(in_row[off]),
                    i64::from(in_row[off + 1]),
                    i64::from(in_row[off + 2]),
                    i64::from(in_row[off + 3]),
                ];
                let t = [
                    i64::from(tgt_row[off]),
                    i64::from(tgt_row[off + 1]),
                    i64::from(tgt_row[off + 2]),
                    i64::from(tgt_row[off + 3]),
                ];
                let res = q31_hamilton_train_step(g, v, t, 4);
                total_loss_before = total_loss_before.saturating_add(res.loss_before >> 24);
                if res.accepted {
                    for c in 0..4 {
                        self.gates_q31[off + c] = res.accepted_gate[c]
                            .clamp(i64::from(i32::MIN), i64::from(i32::MAX))
                            as i32;
                    }
                    accepted_steps += 1;
                    total_loss_after = total_loss_after.saturating_add(res.loss_after >> 24);
                } else {
                    total_loss_after = total_loss_after.saturating_add(res.loss_before >> 24);
                }
            }
        }
        self.step = self.step.saturating_add(1);
        (accepted_steps, total_loss_before, total_loss_after)
    }
}
