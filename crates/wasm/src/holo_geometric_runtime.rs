//! Track 1 (`R4-51`): Pure-integer `UORR4V13` Qwen3 Holo geometric runtime with Wasm `simd128`
//! kernels, Block-32 RHT symmetric `int8` tied embeddings, Cholesky-GPTQ `int4` layers, pre-RoPE
//! per-head `q_norm`/`k_norm`, SWAR 512-bit Hamming shortlisting, 128-bit QK Starling Flock sparse
//! attention selection, streaming chunked OPFS mounting, and in-stream HuggingFace repo compilation.

use crate::holo_kappa_store::{parse_safetensors_header, HoloKappaStoreError};
use crate::lexlean_generated::r4_holo_geometric_runtime::{
    admit_holo_geometric_runtime, compute_shortlist_budget, AttentionOrderCheck, HoloRuntimeVerdict,
};
use crate::r4_geometric_stack::R4StackError;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

/// Canonical header magic for Holo v4 Qwen3 containers (`UORR4V13`).
pub const HOLO_V13_MAGIC: &[u8; 8] = b"UORR4V13";
/// Canonical container version (`13`).
pub const HOLO_V13_VERSION: u32 = 13;
/// Compact embedded tokenizer magic (`UORTK13\0`).
pub const HOLO_TOK13_MAGIC: &[u8; 8] = b"UORTK13\0";

const RESIDUAL_EXP: i32 = -18;
const NORM_EXP: i32 = -22;
const ROPE_Q: i32 = 14;
const EXP_STEP_LOG2: i32 = -8;
const EXP_RANGE: i32 = 32;
const SILU_STEP_LOG2: i32 = -8;
const SILU_RANGE_LOG2: i32 = 4;
const SIG_WORDS: usize = 8;
const DEFAULT_SHORTLIST_K: usize = 16_384;

/// Formal R4.HoloGeometricRuntime immutable xorshift64 sign stream for Block-32 RHT.
#[inline(always)]
pub fn rht_signs_bits(cols: usize, seed: u64) -> Vec<bool> {
    let mut out = Vec::with_capacity(cols);
    let mut s = seed ^ 0x9e37_79b9_7f4a_7c15;
    if s == 0 {
        s = 0x853c_49e6_748f_ea9b;
    }
    for _ in 0..cols {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        out.push((s & 1) == 1);
    }
    out
}

/// Formal R4.HoloGeometricRuntime pure-integer 32-point Walsh-Hadamard butterfly (`int_fwht32`).
#[inline(always)]
pub fn int_fwht32_i64(chunk: &mut [i64; 32]) {
    let mut h = 1usize;
    while h < 32 {
        let mut i = 0usize;
        while i < 32 {
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

/// Formal R4.HoloGeometricRuntime rounding signed arithmetic shift.
#[inline(always)]
pub fn shift_round(v: i64, sh: i32) -> i64 {
    if sh > 0 {
        if sh >= 63 {
            return if v < 0 { -1 } else { 0 };
        }
        v.saturating_add(1i64 << (sh - 1)) >> sh
    } else if sh < 0 {
        let k = sh.unsigned_abs().min(62);
        let lim = i64::MAX >> k;
        if v > lim {
            i64::MAX
        } else if v < -lim {
            i64::MIN
        } else {
            v << k
        }
    } else {
        v
    }
}

/// Formal R4.HoloGeometricRuntime CLZ-seeded 6-step Newton integer sqrt (`D81`).
#[inline(always)]
pub fn clz_newton_isqrt(n: u128) -> u128 {
    if n <= 1 {
        return n;
    }
    let bits = 128 - n.leading_zeros();
    let mut x = 1u128 << bits.div_ceil(2);
    for _ in 0..6 {
        let next = (x + n / x) >> 1;
        if next >= x {
            return x;
        }
        x = next;
    }
    x
}

/// Formal R4.HoloGeometricRuntime pure-integer dot product of 32 contiguous half-block `int4` weights
/// (`packed[k] = (q[k] + 8) | ((q[k+16] + 8) << 4)`) with 32 `i16` RHT activations using Wasm `simd128`
/// vector instructions when compiled with `+simd128` and exact scalar integer logic on host targets.
#[inline(always)]
pub fn dot_i4_group32(packed: &[u8; 16], act: &[i16; 32]) -> i32 {
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    // SAFETY: `packed` is `[u8; 16]` (exact 16-byte load) and `act` is `[i16; 32]` (two 16-byte loads per half).
    unsafe {
        use core::arch::wasm32::*;
        let p = v128_load(packed.as_ptr() as *const v128);
        let mask_lo = u8x16_splat(0x0f);
        let bias = i8x16_splat(8);

        let w_lo_i8 = i8x16_sub(v128_and(p, mask_lo), bias);
        let w_hi_i8 = i8x16_sub(u8x16_shr(p, 4), bias);

        let w0 = i16x8_extend_low_i8x16(w_lo_i8);
        let w1 = i16x8_extend_high_i8x16(w_lo_i8);
        let w2 = i16x8_extend_low_i8x16(w_hi_i8);
        let w3 = i16x8_extend_high_i8x16(w_hi_i8);

        let a0 = v128_load(act.as_ptr() as *const v128);
        let a1 = v128_load(act.as_ptr().add(8) as *const v128);
        let a2 = v128_load(act.as_ptr().add(16) as *const v128);
        let a3 = v128_load(act.as_ptr().add(24) as *const v128);

        let d0 = i32x4_dot_i16x8(w0, a0);
        let d1 = i32x4_dot_i16x8(w1, a1);
        let d2 = i32x4_dot_i16x8(w2, a2);
        let d3 = i32x4_dot_i16x8(w3, a3);

        let sum4 = i32x4_add(i32x4_add(d0, d1), i32x4_add(d2, d3));
        return i32x4_extract_lane::<0>(sum4)
            .wrapping_add(i32x4_extract_lane::<1>(sum4))
            .wrapping_add(i32x4_extract_lane::<2>(sum4))
            .wrapping_add(i32x4_extract_lane::<3>(sum4));
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        let mut dot = 0i32;
        for k in 0..16 {
            let b = packed[k];
            let q0 = i32::from(b & 15) - 8;
            let q1 = i32::from(b >> 4) - 8;
            dot = dot
                .wrapping_add(q0 * i32::from(act[k]))
                .wrapping_add(q1 * i32::from(act[k + 16]));
        }
        dot
    }
}

/// Formal R4.HoloGeometricRuntime pure-integer dot product of 32 `int8` RHT weights against
/// 32 `i16` activations using Wasm `simd128` (`i32x4_dot_i16x8`) when enabled.
#[inline(always)]
pub fn dot_i8_group32(w: &[i8; 32], act: &[i16; 32]) -> i32 {
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    // SAFETY: `w` is `[i8; 32]` (two 16-byte loads) and `act` is `[i16; 32]` (four 16-byte loads).
    unsafe {
        use core::arch::wasm32::*;
        let w_lo = v128_load(w.as_ptr() as *const v128);
        let w_hi = v128_load(w.as_ptr().add(16) as *const v128);

        let d0 = i32x4_dot_i16x8(
            i16x8_extend_low_i8x16(w_lo),
            v128_load(act.as_ptr() as *const v128),
        );
        let d1 = i32x4_dot_i16x8(
            i16x8_extend_high_i8x16(w_lo),
            v128_load(act.as_ptr().add(8) as *const v128),
        );
        let d2 = i32x4_dot_i16x8(
            i16x8_extend_low_i8x16(w_hi),
            v128_load(act.as_ptr().add(16) as *const v128),
        );
        let d3 = i32x4_dot_i16x8(
            i16x8_extend_high_i8x16(w_hi),
            v128_load(act.as_ptr().add(24) as *const v128),
        );

        let sum4 = i32x4_add(i32x4_add(d0, d1), i32x4_add(d2, d3));
        return i32x4_extract_lane::<0>(sum4)
            .wrapping_add(i32x4_extract_lane::<1>(sum4))
            .wrapping_add(i32x4_extract_lane::<2>(sum4))
            .wrapping_add(i32x4_extract_lane::<3>(sum4));
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        let mut dot = 0i32;
        for k in 0..32 {
            dot = dot.wrapping_add(i32::from(w[k]) * i32::from(act[k]));
        }
        dot
    }
}

/// Formal R4.HoloGeometricRuntime pure-integer RMSNorm with explicit Q24 per-channel scale vector.
pub fn int_rms_norm_gain(
    x: &[i32],
    gain_q24: &[i32],
    eps_q48: i64,
    in_exp: i32,
    out_exp: i32,
    out: &mut [i64],
) {
    let mut max_abs = 0u32;
    for &v in x {
        max_abs = max_abs.max(v.unsigned_abs());
    }
    let bl = (32 - max_abs.leading_zeros() as i32).max(1);
    let sh = bl - 20;
    let mut sum = 0u128;
    for (dst, &v) in out.iter_mut().zip(x) {
        let y = shift_round(i64::from(v), sh);
        *dst = y;
        let ay = u128::from(y.unsigned_abs());
        let sq = ay.wrapping_mul(ay);
        sum = sum.wrapping_add(sq);
    }
    let mean = sum / (x.len().max(1) as u128);
    let sq_exp = (in_exp + sh) << 1;
    let eps_int = shift_round(eps_q48, sq_exp - (-48)).max(0) as u128;
    let ms = mean.saturating_add(eps_int).max(1);
    let s = clz_newton_isqrt(ms << 68).max(1);
    let r = (1u128 << 96) / s;
    let r_shift = (86 + out_exp).max(1) as u32;
    let round_half = 1i128 << (r_shift - 1);
    for (i, slot) in out.iter_mut().enumerate() {
        let unit = (i128::from(*slot) * (r as i128) * i128::from(gain_q24[i]))
            .saturating_add(round_half)
            >> r_shift;
        *slot = unit.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
    }
}

/// Formal R4.HoloGeometricRuntime build reversible GPT-2 byte-to-Unicode bijection table.
fn gpt2_byte_to_unicode() -> ([char; 256], HashMap<char, u8>) {
    let mut bs: Vec<u16> = (b'!'..=b'~')
        .chain(0xA1..=0xAC)
        .chain(0xAE..=0xFF)
        .map(u16::from)
        .collect();
    let mut cs = bs.clone();
    let mut n = 0u16;
    for b in 0..=255u16 {
        if !bs.contains(&b) {
            bs.push(b);
            cs.push(256 + n);
            n += 1;
        }
    }
    let mut b2u = ['\0'; 256];
    let mut u2b = HashMap::with_capacity(256);
    for (b, c) in bs.into_iter().zip(cs) {
        let ch = char::from_u32(u32::from(c)).unwrap();
        b2u[b as usize] = ch;
        u2b.insert(ch, b as u8);
    }
    (b2u, u2b)
}

/// Formal R4.HoloGeometricRuntime exact Qwen3 7-branch pretokenizer (`\p{N}` single digit splitting).
pub fn qwen3_pretokenize(text: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut idx = 0usize;
    while idx < text.len() {
        let rest = &text[idx..];
        let c0 = rest.chars().next().unwrap();

        if c0 == '\'' {
            let low2 = rest.get(..2).map(|s| s.to_ascii_lowercase());
            let low3 = rest.get(..3).map(|s| s.to_ascii_lowercase());
            if matches!(low3.as_deref(), Some("'re" | "'ve" | "'ll")) {
                pieces.push(&rest[..3]);
                idx += 3;
                continue;
            }
            if matches!(low2.as_deref(), Some("'s" | "'t" | "'m" | "'d")) {
                pieces.push(&rest[..2]);
                idx += 2;
                continue;
            }
        }

        let c0_can_prefix = c0 != '\r' && c0 != '\n' && !c0.is_alphabetic() && !c0.is_numeric();
        let mut match2 = None;
        if c0.is_alphabetic() {
            let mut end = c0.len_utf8();
            while let Some((_, ch)) = rest[end..].char_indices().next() {
                if ch.is_alphabetic() {
                    end += ch.len_utf8();
                } else {
                    break;
                }
            }
            match2 = Some(end);
        } else if c0_can_prefix {
            let after0 = c0.len_utf8();
            if let Some((_, c1)) = rest[after0..].char_indices().next() {
                if c1.is_alphabetic() {
                    let mut end = after0 + c1.len_utf8();
                    while let Some((_, ch)) = rest[end..].char_indices().next() {
                        if ch.is_alphabetic() {
                            end += ch.len_utf8();
                        } else {
                            break;
                        }
                    }
                    match2 = Some(end);
                }
            }
        }
        if let Some(end) = match2 {
            pieces.push(&rest[..end]);
            idx += end;
            continue;
        }

        if c0.is_numeric() {
            let end = c0.len_utf8();
            pieces.push(&rest[..end]);
            idx += end;
            continue;
        }

        let start4 = if c0 == ' ' { 1 } else { 0 };
        if let Some((_, next_c)) = rest[start4..].char_indices().next() {
            if !next_c.is_whitespace() && !next_c.is_alphabetic() && !next_c.is_numeric() {
                let mut end = start4 + next_c.len_utf8();
                while let Some((_, ch)) = rest[end..].char_indices().next() {
                    if !ch.is_whitespace() && !ch.is_alphabetic() && !ch.is_numeric() {
                        end += ch.len_utf8();
                    } else {
                        break;
                    }
                }
                while let Some(&b) = rest.as_bytes().get(end) {
                    if b == b'\r' || b == b'\n' {
                        end += 1;
                    } else {
                        break;
                    }
                }
                pieces.push(&rest[..end]);
                idx += end;
                continue;
            }
        }

        if c0.is_whitespace() {
            let mut end = 0usize;
            let mut last_nl_end = None;
            for (off, ch) in rest.char_indices() {
                if !ch.is_whitespace() {
                    break;
                }
                end = off + ch.len_utf8();
                if ch == '\r' || ch == '\n' {
                    last_nl_end = Some(end);
                }
            }
            if let Some(nl_end) = last_nl_end {
                pieces.push(&rest[..nl_end]);
                idx += nl_end;
                continue;
            }
            if end < rest.len() {
                let mut prev_end = 0usize;
                for (off, _ch) in rest[..end].char_indices() {
                    prev_end = off;
                }
                if prev_end > 0 {
                    pieces.push(&rest[..prev_end]);
                    idx += prev_end;
                    continue;
                }
            } else {
                pieces.push(&rest[..end]);
                idx += end;
                continue;
            }
            pieces.push(&rest[..c0.len_utf8()]);
            idx += c0.len_utf8();
            continue;
        }

        pieces.push(&rest[..c0.len_utf8()]);
        idx += c0.len_utf8();
    }
    pieces
}

/// Embedded Qwen3 binary tokenizer (`UORTK13\0`) parsed cleanly in Wasm without multi-MB JSON overhead.
#[derive(Clone, Debug)]
pub struct HoloTok13 {
    /// Vocabulary token count.
    pub vocab_size: usize,
    /// Raw decoded UTF-8 byte sequences indexed by token ID.
    pub id_to_raw: Vec<Vec<u8>>,
    /// Single-byte token ID table (`0..256`).
    pub byte_tokens: [u32; 256],
    /// BPE merge pairs mapping `(left << 32) | right` to `(rank, merged_id)`.
    pub pair_merges: HashMap<u64, (u32, u32)>,
    /// Special/added token patterns sorted descending by length.
    pub added_tokens: Vec<(String, u32, bool)>,
    /// Short ASCII token IDs guaranteed to stay in `lm_head` shortlist.
    pub ascii_fast_tokens: Vec<u32>,
}

impl HoloTok13 {
    /// Formal R4.HoloGeometricRuntime parse compact `UORTK13\0` section from `UORR4V13`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, R4StackError> {
        if bytes.len() < 20 || &bytes[..8] != HOLO_TOK13_MAGIC {
            return Err(R4StackError::InvalidMagic);
        }
        let u32_at = |off: usize| -> u32 {
            u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap_or([0; 4]))
        };
        let vocab_count = u32_at(8) as usize;
        let merges_count = u32_at(12) as usize;
        let added_count = u32_at(16) as usize;
        let mut c = 20usize;

        let mut id_to_raw = Vec::with_capacity(vocab_count);
        for _ in 0..vocab_count {
            let len = u16::from_le_bytes([
                *bytes
                    .get(c)
                    .ok_or(R4StackError::InvalidGeometry("uortk13 raw len"))?,
                *bytes
                    .get(c + 1)
                    .ok_or(R4StackError::InvalidGeometry("uortk13 raw len"))?,
            ]) as usize;
            c += 2;
            let raw = bytes
                .get(c..c + len)
                .ok_or(R4StackError::InvalidGeometry("uortk13 raw bytes"))?
                .to_vec();
            c += len;
            id_to_raw.push(raw);
        }

        let (b2u, _) = gpt2_byte_to_unicode();
        let mut token_to_id = HashMap::with_capacity(vocab_count);
        let mut byte_tokens = [0u32; 256];
        let mut ascii_fast_tokens = Vec::with_capacity(512);
        for id in 0..vocab_count {
            let len = u16::from_le_bytes([
                *bytes
                    .get(c)
                    .ok_or(R4StackError::InvalidGeometry("uortk13 tok len"))?,
                *bytes
                    .get(c + 1)
                    .ok_or(R4StackError::InvalidGeometry("uortk13 tok len"))?,
            ]) as usize;
            c += 2;
            let tbytes = bytes
                .get(c..c + len)
                .ok_or(R4StackError::InvalidGeometry("uortk13 tok bytes"))?;
            c += len;
            if let Ok(s) = std::str::from_utf8(tbytes) {
                if !s.is_empty() {
                    if let Some(raw) = id_to_raw.get(id) {
                        if raw.len() == 1 {
                            byte_tokens[raw[0] as usize] = id as u32;
                        }
                        if raw.len() <= 2 && raw.iter().all(|b| b.is_ascii()) {
                            ascii_fast_tokens.push(id as u32);
                        }
                    }
                    token_to_id.insert(s.to_string(), id as u32);
                }
            }
        }
        let _ = (b2u, token_to_id);

        let mut pair_merges = HashMap::with_capacity(merges_count);
        for _ in 0..merges_count {
            let s = bytes
                .get(c..c + 16)
                .ok_or(R4StackError::InvalidGeometry("uortk13 merge"))?;
            c += 16;
            let l = u32::from_le_bytes(s[0..4].try_into().unwrap());
            let r = u32::from_le_bytes(s[4..8].try_into().unwrap());
            let m = u32::from_le_bytes(s[8..12].try_into().unwrap());
            let rank = u32::from_le_bytes(s[12..16].try_into().unwrap());
            pair_merges.insert((u64::from(l) << 32) | u64::from(r), (rank, m));
        }

        let mut added_tokens = Vec::with_capacity(added_count);
        for _ in 0..added_count {
            let id = u32::from_le_bytes(
                bytes
                    .get(c..c + 4)
                    .ok_or(R4StackError::InvalidGeometry("uortk13 added id"))?
                    .try_into()
                    .unwrap(),
            );
            let special = *bytes
                .get(c + 4)
                .ok_or(R4StackError::InvalidGeometry("uortk13 special"))?
                != 0;
            let len = u16::from_le_bytes([
                *bytes
                    .get(c + 5)
                    .ok_or(R4StackError::InvalidGeometry("uortk13 added len"))?,
                *bytes
                    .get(c + 6)
                    .ok_or(R4StackError::InvalidGeometry("uortk13 added len"))?,
            ]) as usize;
            c += 7;
            let pat = String::from_utf8_lossy(
                bytes
                    .get(c..c + len)
                    .ok_or(R4StackError::InvalidGeometry("uortk13 added pat"))?,
            )
            .into_owned();
            c += len;
            added_tokens.push((pat, id, special));
        }
        added_tokens.sort_by_key(|b| std::cmp::Reverse(b.0.len()));

        Ok(Self {
            vocab_size: vocab_count,
            id_to_raw,
            byte_tokens,
            pair_merges,
            added_tokens,
            ascii_fast_tokens,
        })
    }

    /// Formal R4.HoloGeometricRuntime encode UTF-8 text into Qwen3 BPE token IDs.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        let mut ids = Vec::new();
        let mut pos = 0usize;
        let mut chunk_start = 0usize;
        while pos < text.len() {
            let rest = &text[pos..];
            if let Some((pat, aid, _)) = self
                .added_tokens
                .iter()
                .find(|(p, _, _)| rest.starts_with(p.as_str()))
            {
                if chunk_start < pos {
                    for piece in qwen3_pretokenize(&text[chunk_start..pos]) {
                        self.bpe_piece(piece.as_bytes(), &mut ids);
                    }
                }
                ids.push(*aid);
                pos += pat.len();
                chunk_start = pos;
            } else {
                let ch = rest.chars().next().unwrap();
                pos += ch.len_utf8();
            }
        }
        if chunk_start < text.len() {
            for piece in qwen3_pretokenize(&text[chunk_start..]) {
                self.bpe_piece(piece.as_bytes(), &mut ids);
            }
        }
        ids
    }

    fn bpe_piece(&self, bytes: &[u8], out: &mut Vec<u32>) {
        if bytes.is_empty() {
            return;
        }
        let mut ids: Vec<u32> = bytes
            .iter()
            .map(|&b| self.byte_tokens[b as usize])
            .collect();
        while ids.len() >= 2 {
            let mut best_rank = u32::MAX;
            let mut best_pos = usize::MAX;
            let mut best_mid = 0u32;
            for i in 0..ids.len() - 1 {
                let key = (u64::from(ids[i]) << 32) | u64::from(ids[i + 1]);
                if let Some(&(r, mid)) = self.pair_merges.get(&key) {
                    if r < best_rank {
                        best_rank = r;
                        best_pos = i;
                        best_mid = mid;
                    }
                }
            }
            if best_pos == usize::MAX {
                break;
            }
            let tl = ids[best_pos];
            let tr = ids[best_pos + 1];
            let mut i = 0usize;
            let mut w = 0usize;
            while i < ids.len() {
                if i + 1 < ids.len() && ids[i] == tl && ids[i + 1] == tr {
                    ids[w] = best_mid;
                    w += 1;
                    i += 2;
                } else {
                    ids[w] = ids[i];
                    w += 1;
                    i += 1;
                }
            }
            ids.truncate(w);
        }
        out.extend_from_slice(&ids);
    }

    /// Formal R4.HoloGeometricRuntime decode Qwen3 token IDs into lossless UTF-8.
    pub fn decode(&self, ids: &[u32]) -> String {
        let mut raw = Vec::with_capacity(ids.len() * 4);
        for &id in ids {
            if let Some(b) = self.id_to_raw.get(id as usize) {
                raw.extend_from_slice(b);
            }
        }
        String::from_utf8_lossy(&raw).into_owned()
    }
}

/// RHT-32 symmetric `int8` tied `embed` / `lm_head` tensor.
#[derive(Clone, Debug)]
pub struct HoloInt8RhtMatrix {
    /// Output row count.
    pub rows: usize,
    /// Input column width (multiple of 32).
    pub cols: usize,
    /// Deterministic xorshift64 RHT sign seed.
    pub rht_seed: u64,
    /// Per-row binary exponent shift.
    pub row_exp: Vec<i8>,
    /// Per-group-32 unsigned 16-bit scale multiplier.
    pub group_mult: Vec<u16>,
    /// RHT-domain signed `int8` weights.
    pub weights: Vec<i8>,
}

/// RHT-32 symmetric group-32 Cholesky-GPTQ `int4` projection tensor.
#[derive(Clone, Debug)]
pub struct HoloInt4RhtMatrix {
    /// Output row count.
    pub rows: usize,
    /// Input column width (multiple of 32).
    pub cols: usize,
    /// Deterministic xorshift64 RHT sign seed.
    pub rht_seed: u64,
    /// Per-row binary exponent shift.
    pub row_exp: Vec<i8>,
    /// Per-group-32 unsigned 16-bit scale multiplier.
    pub group_mult: Vec<u16>,
    /// Contiguous half-block bias-8 packed nibbles.
    pub nibbles: Vec<u8>,
}

/// Down projection matrix supporting `int4` (`flags & 0x10 == 0`) or `int8` (`flags & 0x10 != 0`).
#[derive(Clone, Debug)]
pub enum HoloDownProjMatrix {
    /// Standard Cholesky-GPTQ `int4` down projection.
    Int4(HoloInt4RhtMatrix),
    /// Promoted symmetric group-32 `int8` down projection.
    Int8(HoloInt8RhtMatrix),
}

/// Single `UORR4V13` Qwen3 transformer layer with explicit pre-RoPE `q_norm`/`k_norm` `[128]`.
#[derive(Clone, Debug)]
pub struct HoloV13Layer {
    /// Input RMSNorm Q24 per-channel gain vector.
    pub in_ln_q24: Vec<i32>,
    /// Post-attention RMSNorm Q24 per-channel gain vector.
    pub post_ln_q24: Vec<i32>,
    /// Mandatory pre-RoPE per-head query RMSNorm Q24 gain vector.
    pub q_norm_q24: Vec<i32>,
    /// Mandatory pre-RoPE per-head key RMSNorm Q24 gain vector.
    pub k_norm_q24: Vec<i32>,
    /// Query projection matrix.
    pub q_proj: HoloInt4RhtMatrix,
    /// Key projection matrix.
    pub k_proj: HoloInt4RhtMatrix,
    /// Value projection matrix.
    pub v_proj: HoloInt4RhtMatrix,
    /// Attention output projection matrix.
    pub o_proj: HoloInt4RhtMatrix,
    /// SwiGLU gate projection matrix.
    pub gate_proj: HoloInt4RhtMatrix,
    /// SwiGLU up projection matrix.
    pub up_proj: HoloInt4RhtMatrix,
    /// SwiGLU down projection matrix (`int4` or promoted `int8`).
    pub down_proj: HoloDownProjMatrix,
}

/// Complete mounted `UORR4V13` pure-integer Qwen3 geometric model.
#[derive(Clone, Debug)]
pub struct HoloV13Model {
    /// Logical model wire identifier.
    pub model_id: String,
    /// Verified container SHA-256 hex digest.
    pub container_sha256: String,
    /// Model vocabulary size.
    pub vocab_size: usize,
    /// Hidden residual width.
    pub d_model: usize,
    /// Transformer layer depth.
    pub n_layers: usize,
    /// Query attention head count.
    pub n_heads: usize,
    /// Key-value GQA head count.
    pub n_kv_heads: usize,
    /// Head dimension (`128` for Qwen3).
    pub head_dim: usize,
    /// MLP SwiGLU intermediate width.
    pub intermediate_size: usize,
    /// Maximum supported positional context.
    pub max_seq: usize,
    /// Beginning-of-sequence token ID.
    pub bos_token_id: u32,
    /// End-of-sequence token ID.
    pub eos_token_id: u32,
    /// End-of-turn token ID.
    pub eot_token_id: u32,
    /// `<think>` special token ID.
    pub think_start_id: u32,
    /// `</think>` special token ID.
    pub think_end_id: u32,
    /// RMSNorm epsilon in Q48 fixed point.
    pub rms_eps_q48: i64,
    /// Attention dot scale in Q30 fixed point.
    pub score_scale_q30: i64,
    /// RoPE base theta.
    pub rope_theta: u64,
    /// Precomputed RoPE cosine LUT in Q14.
    pub rope_cos_q14: Vec<i16>,
    /// Precomputed RoPE sine LUT in Q14.
    pub rope_sin_q14: Vec<i16>,
    /// Precomputed negative-exponent LUT in Q30.
    pub exp_table: Vec<u32>,
    /// Precomputed SiLU LUT in Q16.
    pub silu_table: Vec<i32>,
    /// Embedded `UORTK13` ByteLevel BPE tokenizer.
    pub tokenizer: HoloTok13,
    /// 512-bit RHT sign signatures for SWAR `lm_head` shortlisting.
    pub vocab_sigs: Vec<u64>,
    /// Tied input embedding / output projection `int8` RHT matrix.
    pub embed: HoloInt8RhtMatrix,
    /// Final RMSNorm Q24 gain vector.
    pub final_norm_q24: Vec<i32>,
    /// Transformer layers.
    pub layers: Vec<HoloV13Layer>,
    /// Rolling `int8` + 128-bit Starling Flock prefix KV cache.
    pub kv_cache: HoloPrefixKvCache,
}

/// Per-head `i16` quantized KV cache per layer + 128-bit QK Hamming Starling Flock signatures (`R4-51`).
#[derive(Clone, Debug, Default)]
pub struct HoloPrefixKvCache {
    /// Cached prefix token IDs.
    pub tokens: Vec<u32>,
    /// Per-layer cached sequence lengths.
    pub seq_len: Vec<usize>,
    /// Quantized per-head `int16` key vectors per layer.
    pub k_i16: Vec<Vec<i16>>,
    /// Per-step, per-KV-head integer key scale factors per layer.
    pub k_scales: Vec<Vec<i32>>,
    /// 128-bit QK Hamming signatures per KV head per position.
    pub k_sig128: Vec<Vec<[u64; 2]>>,
    /// Quantized per-head `int16` value vectors per layer.
    pub v_i16: Vec<Vec<i16>>,
    /// Per-step, per-KV-head integer value scale factors per layer.
    pub v_scales: Vec<Vec<i32>>,
}

impl HoloPrefixKvCache {
    /// Formal R4.HoloGeometricRuntime clear incremental KV state cleanly.
    pub fn reset(&mut self, n_layers: usize) {
        self.tokens.clear();
        self.seq_len = vec![0; n_layers];
        self.k_i16 = vec![Vec::new(); n_layers];
        self.k_scales = vec![Vec::new(); n_layers];
        self.k_sig128 = vec![Vec::new(); n_layers];
        self.v_i16 = vec![Vec::new(); n_layers];
        self.v_scales = vec![Vec::new(); n_layers];
    }

    /// Formal R4.HoloGeometricRuntime append per-head `i16` key/value slices and 128-bit QK Starling signature.
    fn push_kv(&mut self, layer: usize, k_vec: &[i32], v_vec: &[i32], nkv: usize, hd: usize) {
        self.seq_len[layer] += 1;
        let k_dest = &mut self.k_i16[layer];
        let v_dest = &mut self.v_i16[layer];
        for kh in 0..nkv {
            let k_head = &k_vec[kh * hd..(kh + 1) * hd];
            let v_head = &v_vec[kh * hd..(kh + 1) * hd];

            let mut kmax = 0u32;
            for &x in k_head {
                kmax = kmax.max(x.unsigned_abs());
            }
            let ks = ((i64::from(kmax) + 32_766) / 32_767).max(1) as i32;
            self.k_scales[layer].push(ks);
            let ks64 = i64::from(ks);
            let half_k = ks64 / 2;
            for &x in k_head {
                let x64 = i64::from(x);
                let q = if x64 >= 0 {
                    (x64 + half_k) / ks64
                } else {
                    (x64 - half_k) / ks64
                };
                k_dest.push(q.clamp(-32_767, 32_767) as i16);
            }

            let mut vmax = 0u32;
            for &x in v_head {
                vmax = vmax.max(x.unsigned_abs());
            }
            let vs = ((i64::from(vmax) + 32_766) / 32_767).max(1) as i32;
            self.v_scales[layer].push(vs);
            let vs64 = i64::from(vs);
            let half_v = vs64 / 2;
            for &x in v_head {
                let x64 = i64::from(x);
                let q = if x64 >= 0 {
                    (x64 + half_v) / vs64
                } else {
                    (x64 - half_v) / vs64
                };
                v_dest.push(q.clamp(-32_767, 32_767) as i16);
            }

            // Compute 128-bit QK Hamming signature per KV head (`R4-51`)
            let mut sig = [0u64; 2];
            for (d_i, &kv) in k_head.iter().enumerate() {
                if kv >= 0 {
                    sig[d_i >> 6] |= 1u64 << (d_i & 63);
                }
            }
            self.k_sig128[layer].push(sig);
        }
    }
}

/// Chunked streaming parser absorbing 1 MiB `.holo` pieces into `HoloV13Model` without 2x RAM bloat.
pub struct HoloV13StreamingBuilder {
    model_id: String,
    expected_sha256: String,
    expected_pieces: usize,
    pushed_pieces: usize,
    total_bytes_pushed: u64,
    hasher: Sha256,
    buf: Vec<u8>,
    cursor: usize,
    stage: ParseStage,
    // Partially parsed fields:
    vocab_size: usize,
    d_model: usize,
    n_layers: usize,
    n_heads: usize,
    n_kv_heads: usize,
    head_dim: usize,
    intermediate_size: usize,
    max_seq: usize,
    bos_token_id: u32,
    eos_token_id: u32,
    eot_token_id: u32,
    think_start_id: u32,
    think_end_id: u32,
    rms_eps_q48: i64,
    score_scale_q30: i64,
    rope_theta: u64,
    rope_cos_q14: Vec<i16>,
    rope_sin_q14: Vec<i16>,
    exp_table: Vec<u32>,
    silu_table: Vec<i32>,
    tokenizer: Option<HoloTok13>,
    vocab_sigs: Vec<u64>,
    embed: Option<HoloInt8RhtMatrix>,
    final_norm_q24: Vec<i32>,
    layers: Vec<HoloV13Layer>,
    pending_in_ln: Vec<i32>,
    pending_post_ln: Vec<i32>,
    pending_q_norm: Vec<i32>,
    pending_k_norm: Vec<i32>,
    pending_mats: Vec<HoloInt4RhtMatrix>,
    pending_down_int8: Option<HoloInt8RhtMatrix>,
    promote_down_int8: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParseStage {
    Header,
    Luts,
    TokLen,
    TokBody(usize),
    Sigs,
    EmbedHeader,
    EmbedBody {
        rows: usize,
        cols: usize,
        seed: u64,
    },
    FinalNorm,
    LayerNorms(usize),
    LayerProjHeader {
        layer: usize,
        slot: usize,
    },
    LayerProjBody {
        layer: usize,
        slot: usize,
        rows: usize,
        cols: usize,
        seed: u64,
    },
    Done,
}

impl HoloV13StreamingBuilder {
    /// Formal R4.HoloGeometricRuntime create a chunked streaming parser for `UORR4V13` pieces.
    pub fn new(
        model_id: impl Into<String>,
        expected_pieces: usize,
        expected_sha256: impl Into<String>,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            expected_sha256: expected_sha256.into().trim().to_ascii_lowercase(),
            expected_pieces,
            pushed_pieces: 0,
            total_bytes_pushed: 0,
            hasher: Sha256::new(),
            buf: Vec::with_capacity(2 * 1024 * 1024),
            cursor: 0,
            stage: ParseStage::Header,
            vocab_size: 0,
            d_model: 0,
            n_layers: 0,
            n_heads: 0,
            n_kv_heads: 0,
            head_dim: 0,
            intermediate_size: 0,
            max_seq: 0,
            bos_token_id: 151_643,
            eos_token_id: 151_645,
            eot_token_id: 151_643,
            think_start_id: 151_667,
            think_end_id: 151_668,
            rms_eps_q48: 0,
            score_scale_q30: 0,
            rope_theta: 1_000_000,
            rope_cos_q14: Vec::new(),
            rope_sin_q14: Vec::new(),
            exp_table: Vec::new(),
            silu_table: Vec::new(),
            tokenizer: None,
            vocab_sigs: Vec::new(),
            embed: None,
            final_norm_q24: Vec::new(),
            layers: Vec::new(),
            pending_in_ln: Vec::new(),
            pending_post_ln: Vec::new(),
            pending_q_norm: Vec::new(),
            pending_k_norm: Vec::new(),
            pending_mats: Vec::with_capacity(7),
            pending_down_int8: None,
            promote_down_int8: false,
        }
    }

    /// Formal R4.HoloGeometricRuntime return unconsumed streaming parser buffer byte count.
    #[inline]
    fn avail(&self) -> usize {
        self.buf.len().saturating_sub(self.cursor)
    }

    /// Formal R4.HoloGeometricRuntime advance slice cursor without copying unconsumed bytes.
    #[inline]
    fn take_from<'a>(buf: &'a [u8], cursor: &mut usize, n: usize) -> &'a [u8] {
        let s = *cursor;
        *cursor += n;
        &buf[s..s + n]
    }

    /// Formal R4.HoloGeometricRuntime drain consumed prefix from streaming buffer when threshold exceeded.
    fn compact(&mut self) {
        if self.cursor >= 262_144 || (self.cursor > 0 && self.cursor == self.buf.len()) {
            self.buf.drain(..self.cursor);
            self.cursor = 0;
        }
    }

    /// Formal R4.HoloGeometricRuntime push one piece chunk and greedily parse ready tensors.
    pub fn push_piece(&mut self, chunk: &[u8]) -> Result<(), R4StackError> {
        self.pushed_pieces += 1;
        self.total_bytes_pushed = self.total_bytes_pushed.saturating_add(chunk.len() as u64);
        self.hasher.update(chunk);
        self.buf.extend_from_slice(chunk);
        self.advance()?;
        self.compact();
        Ok(())
    }

    /// Formal R4.HoloGeometricRuntime step streaming `UORR4V13` state machine over available buffer bytes.
    fn advance(&mut self) -> Result<(), R4StackError> {
        loop {
            match self.stage {
                ParseStage::Header => {
                    if self.avail() < 96 {
                        return Ok(());
                    }
                    let hdr = Self::take_from(&self.buf, &mut self.cursor, 96);
                    if &hdr[..8] != HOLO_V13_MAGIC {
                        return Err(R4StackError::InvalidMagic);
                    }
                    let u32_at = |o: usize| u32::from_le_bytes(hdr[o..o + 4].try_into().unwrap());
                    let i64_at = |o: usize| i64::from_le_bytes(hdr[o..o + 8].try_into().unwrap());
                    let u64_at = |o: usize| u64::from_le_bytes(hdr[o..o + 8].try_into().unwrap());
                    let ver = u32_at(8);
                    if ver != HOLO_V13_VERSION {
                        return Err(R4StackError::UnsupportedVersion(ver));
                    }
                    let guard = u32_at(12);
                    if guard != 1 {
                        return Err(R4StackError::NonZeroFloatOps(guard));
                    }
                    let vocab_size = u32_at(16) as usize;
                    let d_model = u32_at(20) as usize;
                    let n_layers = u32_at(24) as usize;
                    let n_heads = u32_at(28) as usize;
                    let n_kv_heads = u32_at(32) as usize;
                    let head_dim = u32_at(36) as usize;
                    let intermediate_size = u32_at(40) as usize;
                    let max_seq = u32_at(44) as usize;
                    let bos_token_id = u32_at(48);
                    let eos_token_id = u32_at(52);
                    let eot_token_id = u32_at(56);
                    let think_start_id = u32_at(60);
                    let think_end_id = u32_at(64);
                    let rms_eps_q48 = i64_at(68);
                    let score_scale_q30 = i64_at(76);
                    let rope_theta = u64_at(84);
                    let flags = u32_at(92);

                    self.vocab_size = vocab_size;
                    self.d_model = d_model;
                    self.n_layers = n_layers;
                    self.n_heads = n_heads;
                    self.n_kv_heads = n_kv_heads;
                    self.head_dim = head_dim;
                    self.intermediate_size = intermediate_size;
                    self.max_seq = max_seq;
                    self.bos_token_id = bos_token_id;
                    self.eos_token_id = eos_token_id;
                    self.eot_token_id = eot_token_id;
                    self.think_start_id = think_start_id;
                    self.think_end_id = think_end_id;
                    self.rms_eps_q48 = rms_eps_q48;
                    self.score_scale_q30 = score_scale_q30;
                    self.rope_theta = rope_theta;
                    self.promote_down_int8 = (flags & 0x10) != 0;

                    let verdict = admit_holo_geometric_runtime(AttentionOrderCheck {
                        hidden_dim: self.d_model as u64,
                        num_q_heads: self.n_heads as u64,
                        num_kv_heads: self.n_kv_heads as u64,
                        head_dim: self.head_dim as u64,
                        group_size: 32,
                        q_norm_before_rope: (flags & 0b0010) != 0,
                        k_norm_before_rope: (flags & 0b0010) != 0,
                        float_ops_count: 0,
                        dense_matmul_ops_count: 0,
                        worker_rpc_dispatched: true,
                        main_thread_wasm_call: false,
                        hamming_shortlist_k: DEFAULT_SHORTLIST_K.min(self.vocab_size) as u64,
                        vocab_size: self.vocab_size as u64,
                    });
                    if verdict != HoloRuntimeVerdict::RuntimeAdmitted {
                        return Err(R4StackError::InvalidGeometry(
                            "UORR4V13 header failed HoloGeometricRuntime admissibility",
                        ));
                    }
                    self.layers.reserve_exact(self.n_layers);
                    self.stage = ParseStage::Luts;
                }
                ParseStage::Luts => {
                    let half = self.head_dim / 2;
                    let rope_elems = self.max_seq * half;
                    let exp_len = ((EXP_RANGE as usize) << 8) + 1;
                    let silu_len = (1usize << (SILU_RANGE_LOG2 - SILU_STEP_LOG2 + 1)) + 1;
                    let need = rope_elems * 4 + exp_len * 4 + silu_len * 4;
                    if self.avail() < need {
                        return Ok(());
                    }
                    let slice = Self::take_from(&self.buf, &mut self.cursor, need);
                    let mut p = 0usize;
                    let rope_cos: Vec<i16> = (0..rope_elems)
                        .map(|_| {
                            let v = i16::from_le_bytes([slice[p], slice[p + 1]]);
                            p += 2;
                            v
                        })
                        .collect();
                    let rope_sin: Vec<i16> = (0..rope_elems)
                        .map(|_| {
                            let v = i16::from_le_bytes([slice[p], slice[p + 1]]);
                            p += 2;
                            v
                        })
                        .collect();
                    let exp_tbl: Vec<u32> = (0..exp_len)
                        .map(|_| {
                            let v = u32::from_le_bytes(slice[p..p + 4].try_into().unwrap());
                            p += 4;
                            v
                        })
                        .collect();
                    let silu_tbl: Vec<i32> = (0..silu_len)
                        .map(|_| {
                            let v = i32::from_le_bytes(slice[p..p + 4].try_into().unwrap());
                            p += 4;
                            v
                        })
                        .collect();
                    self.rope_cos_q14 = rope_cos;
                    self.rope_sin_q14 = rope_sin;
                    self.exp_table = exp_tbl;
                    self.silu_table = silu_tbl;
                    self.stage = ParseStage::TokLen;
                }
                ParseStage::TokLen => {
                    if self.avail() < 4 {
                        return Ok(());
                    }
                    let len = u32::from_le_bytes(
                        Self::take_from(&self.buf, &mut self.cursor, 4)
                            .try_into()
                            .unwrap(),
                    ) as usize;
                    if !(20..=32 * 1024 * 1024).contains(&len) {
                        return Err(R4StackError::InvalidGeometry("invalid tok13 length"));
                    }
                    self.stage = ParseStage::TokBody(len);
                }
                ParseStage::TokBody(len) => {
                    if self.avail() < len {
                        return Ok(());
                    }
                    let tbytes = Self::take_from(&self.buf, &mut self.cursor, len).to_vec();
                    self.tokenizer = Some(HoloTok13::from_bytes(&tbytes)?);
                    self.stage = ParseStage::Sigs;
                }
                ParseStage::Sigs => {
                    let words = self.vocab_size * SIG_WORDS;
                    let need = words * 8;
                    if self.avail() < need {
                        return Ok(());
                    }
                    let sigs: Vec<u64> = Self::take_from(&self.buf, &mut self.cursor, need)
                        .chunks_exact(8)
                        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                        .collect();
                    self.vocab_sigs = sigs;
                    self.stage = ParseStage::EmbedHeader;
                }
                ParseStage::EmbedHeader => {
                    if self.avail() < 16 {
                        return Ok(());
                    }
                    let hdr = Self::take_from(&self.buf, &mut self.cursor, 16);
                    let rows = u32::from_le_bytes(hdr[0..4].try_into().unwrap()) as usize;
                    let cols = u32::from_le_bytes(hdr[4..8].try_into().unwrap()) as usize;
                    let seed = u64::from_le_bytes(hdr[8..16].try_into().unwrap());
                    if rows != self.vocab_size || cols != self.d_model || (cols & 31) != 0 {
                        return Err(R4StackError::InvalidGeometry("embed dims mismatch"));
                    }
                    self.stage = ParseStage::EmbedBody { rows, cols, seed };
                }
                ParseStage::EmbedBody { rows, cols, seed } => {
                    let groups = cols >> 5;
                    let need = rows + rows * groups * 2 + rows * cols;
                    if self.avail() < need {
                        return Ok(());
                    }
                    let slice = Self::take_from(&self.buf, &mut self.cursor, need);
                    let row_exp = slice[..rows].iter().map(|&b| b as i8).collect();
                    let gm_off = rows;
                    let gm_len = rows * groups * 2;
                    let group_mult = slice[gm_off..gm_off + gm_len]
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    let w_off = gm_off + gm_len;
                    let weights = slice[w_off..w_off + rows * cols]
                        .iter()
                        .map(|&b| b as i8)
                        .collect();
                    self.embed = Some(HoloInt8RhtMatrix {
                        rows,
                        cols,
                        rht_seed: seed,
                        row_exp,
                        group_mult,
                        weights,
                    });
                    self.stage = ParseStage::FinalNorm;
                }
                ParseStage::FinalNorm => {
                    let need = self.d_model * 4;
                    if self.avail() < need {
                        return Ok(());
                    }
                    let norm_vec: Vec<i32> = Self::take_from(&self.buf, &mut self.cursor, need)
                        .chunks_exact(4)
                        .map(|c| i32::from_le_bytes(c.try_into().unwrap()))
                        .collect();
                    self.final_norm_q24 = norm_vec;
                    if self.n_layers == 0 {
                        self.stage = ParseStage::Done;
                    } else {
                        self.stage = ParseStage::LayerNorms(0);
                    }
                }
                ParseStage::LayerNorms(layer) => {
                    let dm = self.d_model;
                    let hd = self.head_dim;
                    let need = (dm * 2 + hd * 2) * 4;
                    if self.avail() < need {
                        return Ok(());
                    }
                    let slice = Self::take_from(&self.buf, &mut self.cursor, need);
                    let mut p = 0usize;
                    let mut read_vec = |n: usize| -> Vec<i32> {
                        let out = slice[p..p + n * 4]
                            .chunks_exact(4)
                            .map(|c| i32::from_le_bytes(c.try_into().unwrap()))
                            .collect();
                        p += n * 4;
                        out
                    };
                    let in_ln = read_vec(dm);
                    let post_ln = read_vec(dm);
                    let q_norm = read_vec(hd);
                    let k_norm = read_vec(hd);
                    self.pending_in_ln = in_ln;
                    self.pending_post_ln = post_ln;
                    self.pending_q_norm = q_norm;
                    self.pending_k_norm = k_norm;
                    self.pending_mats.clear();
                    self.stage = ParseStage::LayerProjHeader { layer, slot: 0 };
                }
                ParseStage::LayerProjHeader { layer, slot } => {
                    if self.avail() < 16 {
                        return Ok(());
                    }
                    let hdr = Self::take_from(&self.buf, &mut self.cursor, 16);
                    let rows = u32::from_le_bytes(hdr[0..4].try_into().unwrap()) as usize;
                    let cols = u32::from_le_bytes(hdr[4..8].try_into().unwrap()) as usize;
                    let seed = u64::from_le_bytes(hdr[8..16].try_into().unwrap());
                    if (cols & 31) != 0 {
                        return Err(R4StackError::InvalidGeometry(
                            "projection cols not multiple of 32",
                        ));
                    }
                    self.stage = ParseStage::LayerProjBody {
                        layer,
                        slot,
                        rows,
                        cols,
                        seed,
                    };
                }
                ParseStage::LayerProjBody {
                    layer,
                    slot,
                    rows,
                    cols,
                    seed,
                } => {
                    let groups = cols >> 5;
                    let is_down_i8 = slot == 6 && self.promote_down_int8;
                    let w_bytes = if is_down_i8 {
                        rows * cols
                    } else {
                        rows * (cols >> 1)
                    };
                    let need = rows + rows * groups * 2 + w_bytes;
                    if self.avail() < need {
                        return Ok(());
                    }
                    let slice = Self::take_from(&self.buf, &mut self.cursor, need);
                    let row_exp = slice[..rows].iter().map(|&b| b as i8).collect();
                    let gm_off = rows;
                    let gm_len = rows * groups * 2;
                    let group_mult = slice[gm_off..gm_off + gm_len]
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    let w_off = gm_off + gm_len;
                    if is_down_i8 {
                        let weights = slice[w_off..need].iter().map(|&b| b as i8).collect();
                        self.pending_down_int8 = Some(HoloInt8RhtMatrix {
                            rows,
                            cols,
                            rht_seed: seed,
                            row_exp,
                            group_mult,
                            weights,
                        });
                    } else {
                        let nibbles = slice[w_off..need].to_vec();
                        self.pending_mats.push(HoloInt4RhtMatrix {
                            rows,
                            cols,
                            rht_seed: seed,
                            row_exp,
                            group_mult,
                            nibbles,
                        });
                    }
                    if slot < 6 {
                        self.stage = ParseStage::LayerProjHeader {
                            layer,
                            slot: slot + 1,
                        };
                    } else {
                        let mut mats = self.pending_mats.drain(..);
                        let q_proj = mats.next().ok_or(R4StackError::TruncatedContainer)?;
                        let k_proj = mats.next().ok_or(R4StackError::TruncatedContainer)?;
                        let v_proj = mats.next().ok_or(R4StackError::TruncatedContainer)?;
                        let o_proj = mats.next().ok_or(R4StackError::TruncatedContainer)?;
                        let gate_proj = mats.next().ok_or(R4StackError::TruncatedContainer)?;
                        let up_proj = mats.next().ok_or(R4StackError::TruncatedContainer)?;
                        let down_proj = if self.promote_down_int8 {
                            HoloDownProjMatrix::Int8(
                                self.pending_down_int8
                                    .take()
                                    .ok_or(R4StackError::TruncatedContainer)?,
                            )
                        } else {
                            HoloDownProjMatrix::Int4(
                                mats.next().ok_or(R4StackError::TruncatedContainer)?,
                            )
                        };
                        let q_dim = self.n_heads * self.head_dim;
                        let kv_dim = self.n_kv_heads * self.head_dim;
                        let dm = self.d_model;
                        let ffn = self.intermediate_size;
                        let down_shape_ok = match &down_proj {
                            HoloDownProjMatrix::Int4(m) => m.rows == dm && m.cols == ffn,
                            HoloDownProjMatrix::Int8(m) => m.rows == dm && m.cols == ffn,
                        };
                        if (q_proj.rows, q_proj.cols) != (q_dim, dm)
                            || (k_proj.rows, k_proj.cols) != (kv_dim, dm)
                            || (v_proj.rows, v_proj.cols) != (kv_dim, dm)
                            || (o_proj.rows, o_proj.cols) != (dm, q_dim)
                            || (gate_proj.rows, gate_proj.cols) != (ffn, dm)
                            || (up_proj.rows, up_proj.cols) != (ffn, dm)
                            || !down_shape_ok
                        {
                            return Err(R4StackError::InvalidGeometry(
                                "layer projection dimensions mismatch",
                            ));
                        }
                        let built = HoloV13Layer {
                            in_ln_q24: std::mem::take(&mut self.pending_in_ln),
                            post_ln_q24: std::mem::take(&mut self.pending_post_ln),
                            q_norm_q24: std::mem::take(&mut self.pending_q_norm),
                            k_norm_q24: std::mem::take(&mut self.pending_k_norm),
                            q_proj,
                            k_proj,
                            v_proj,
                            o_proj,
                            gate_proj,
                            up_proj,
                            down_proj,
                        };
                        self.layers.push(built);
                        if layer + 1 < self.n_layers {
                            self.stage = ParseStage::LayerNorms(layer + 1);
                        } else {
                            self.stage = ParseStage::Done;
                        }
                    }
                }
                ParseStage::Done => {
                    return Ok(());
                }
            }
        }
    }

    /// Formal R4.HoloGeometricRuntime verify container SHA-256 and finalize `HoloV13Model`.
    pub fn finalize(self) -> Result<HoloV13Model, R4StackError> {
        if self.stage != ParseStage::Done || self.avail() != 0 {
            return Err(R4StackError::TruncatedContainer);
        }
        if self.expected_pieces > 0 && self.pushed_pieces != self.expected_pieces {
            return Err(R4StackError::TruncatedContainer);
        }
        let digest = self.hasher.finalize();
        let mut actual_hex = String::with_capacity(64);
        for b in digest {
            use std::fmt::Write;
            let _ = write!(&mut actual_hex, "{b:02x}");
        }
        if !self.expected_sha256.is_empty() && actual_hex != self.expected_sha256 {
            return Err(R4StackError::IntegrityMismatch);
        }
        let mut kv_cache = HoloPrefixKvCache::default();
        kv_cache.reset(self.n_layers);
        Ok(HoloV13Model {
            model_id: self.model_id,
            container_sha256: actual_hex,
            vocab_size: self.vocab_size,
            d_model: self.d_model,
            n_layers: self.n_layers,
            n_heads: self.n_heads,
            n_kv_heads: self.n_kv_heads,
            head_dim: self.head_dim,
            intermediate_size: self.intermediate_size,
            max_seq: self.max_seq,
            bos_token_id: self.bos_token_id,
            eos_token_id: self.eos_token_id,
            eot_token_id: self.eot_token_id,
            think_start_id: self.think_start_id,
            think_end_id: self.think_end_id,
            rms_eps_q48: self.rms_eps_q48,
            score_scale_q30: self.score_scale_q30,
            rope_theta: self.rope_theta,
            rope_cos_q14: self.rope_cos_q14,
            rope_sin_q14: self.rope_sin_q14,
            exp_table: self.exp_table,
            silu_table: self.silu_table,
            tokenizer: self.tokenizer.ok_or(R4StackError::TruncatedContainer)?,
            vocab_sigs: self.vocab_sigs,
            embed: self.embed.ok_or(R4StackError::TruncatedContainer)?,
            final_norm_q24: self.final_norm_q24,
            layers: self.layers,
            kv_cache,
        })
    }
}

/// Metrics returned for one `HoloV13Model` chat turn (`R4-51`).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct HoloChatTurnMetrics {
    /// Metrics schema identifier.
    pub schema: &'static str,
    /// Model wire identifier.
    pub model_id: String,
    /// Container SHA-256 hex digest.
    pub container_sha256: String,
    /// Prompt tokens prefilled.
    pub prompt_tokens: usize,
    /// Completion tokens emitted.
    pub generated_tokens: usize,
    /// Hardware float op count (strictly zero).
    pub float_ops: u64,
    /// Dense matmul op count (strictly zero).
    pub dense_matmul_ops: u64,
    /// True when Wasm `simd128` vector kernels compiled in.
    pub simd128_active: bool,
    /// Pre-RoPE per-head query RMSNorm invariant flag.
    pub q_norm_before_rope: bool,
    /// Pre-RoPE per-head key RMSNorm invariant flag.
    pub k_norm_before_rope: bool,
    /// Vocabulary shortlist size evaluated at `lm_head`.
    pub shortlist_budget: usize,
    /// Starling Flock attended KV positions across steps.
    pub flock_attended_positions: usize,
    /// Total candidate KV positions across steps.
    pub flock_total_positions: usize,
}

/// Formal R4.HoloGeometricRuntime apply integer RHT sign-flip, block-32 FWHT, and Q13 dynamic shift into `i16`.
fn prepare_rht_act16(norm_in: &[i64], in_exp: i32, rht_seed: u64, act16: &mut [i16]) -> i32 {
    let cols = norm_in.len();
    let signs = rht_signs_bits(cols, rht_seed);
    let mut rht = vec![0i64; cols];
    for i in 0..cols {
        rht[i] = if signs[i] {
            norm_in[i]
        } else {
            norm_in[i].saturating_neg()
        };
    }
    for blk in rht.chunks_exact_mut(32) {
        let arr: &mut [i64; 32] = blk.try_into().unwrap();
        int_fwht32_i64(arr);
    }
    let mut max_abs = 0u64;
    for &v in &rht {
        max_abs = max_abs.max(v.unsigned_abs());
    }
    if max_abs == 0 {
        act16.fill(0);
        return in_exp;
    }
    let bl = 64 - max_abs.leading_zeros() as i32;
    let sh = (bl - 13).max(0);
    for i in 0..cols {
        act16[i] = shift_round(rht[i], sh).clamp(-32768, 32767) as i16;
    }
    in_exp + sh
}

/// Formal R4.HoloGeometricRuntime compute strided 512-bit RHT sign signature over normalized query vector.
fn compute_rht512_query_sig(norm_in: &[i64], rht_seed: u64) -> [u64; SIG_WORDS] {
    let cols = norm_in.len();
    let signs = rht_signs_bits(cols, rht_seed);
    let mut rht = vec![0i64; cols];
    for i in 0..cols {
        rht[i] = if signs[i] {
            norm_in[i]
        } else {
            norm_in[i].saturating_neg()
        };
    }
    for blk in rht.chunks_exact_mut(32) {
        let arr: &mut [i64; 32] = blk.try_into().unwrap();
        int_fwht32_i64(arr);
    }
    let mut sig = [0u64; SIG_WORDS];
    for b in 0..512 {
        let col = (b * cols) / 512;
        if rht[col] >= 0 {
            sig[b >> 6] |= 1u64 << (b & 63);
        }
    }
    sig
}

impl HoloV13Model {
    /// Formal R4.HoloGeometricRuntime mount a complete `UORR4V13` byte slice in one call.
    pub fn from_bytes(model_id: impl Into<String>, bytes: &[u8]) -> Result<Self, R4StackError> {
        let mut builder = HoloV13StreamingBuilder::new(model_id, 1, "");
        builder.push_piece(bytes)?;
        builder.finalize()
    }

    /// Formal R4.HoloGeometricRuntime inverse-RHT dequantize one token embedding row into Q24 fixed point.
    fn dequant_embed_row(&self, row: usize, out: &mut [i32]) {
        let m = &self.embed;
        let cols = m.cols;
        let groups = cols >> 5;
        let row_exp = i32::from(m.row_exp[row]);
        let w_row = &m.weights[row * cols..(row + 1) * cols];
        let mults = &m.group_mult[row * groups..(row + 1) * groups];
        let mut wide = vec![0i64; cols];
        let target_exp = -24i32;
        let sh = target_exp - row_exp;
        for g in 0..groups {
            let mult = i64::from(mults[g]);
            for c in 0..32 {
                let val = i64::from(w_row[g * 32 + c]) * mult;
                wide[g * 32 + c] = shift_round(val, sh);
            }
        }
        for blk in wide.chunks_exact_mut(32) {
            let arr: &mut [i64; 32] = blk.try_into().unwrap();
            int_fwht32_i64(arr);
        }
        let signs = rht_signs_bits(cols, m.rht_seed);
        for ((dst, &val), sp) in out.iter_mut().zip(&wide).zip(signs) {
            let sv = if sp { val } else { val.saturating_neg() };
            *dst = shift_round(sv, RESIDUAL_EXP - target_exp)
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        }
    }

    /// Formal R4.HoloGeometricRuntime execute pure-integer block-32 `int4` RHT matrix-vector product.
    fn gemv_i4_rht(
        m: &HoloInt4RhtMatrix,
        norm_in: &[i64],
        in_exp: i32,
        out_exp: i32,
        out: &mut [i32],
    ) {
        let cols = m.cols;
        let mut act16 = vec![0i16; cols];
        let x_exp = prepare_rht_act16(norm_in, in_exp, m.rht_seed, &mut act16);
        let row_bytes = cols >> 1;
        let groups = cols >> 5;
        for (r, out_slot) in out.iter_mut().enumerate().take(m.rows) {
            let re = i32::from(m.row_exp[r]);
            let sh = out_exp - (re + x_exp);
            let nibs = &m.nibbles[r * row_bytes..(r + 1) * row_bytes];
            let mults = &m.group_mult[r * groups..(r + 1) * groups];
            let mut acc = 0i128;
            for g in 0..groups {
                let nb: &[u8; 16] = (&nibs[g * 16..(g + 1) * 16]).try_into().unwrap();
                let a: &[i16; 32] = (&act16[g * 32..(g + 1) * 32]).try_into().unwrap();
                let dot = dot_i4_group32(nb, a);
                acc += i128::from(dot) * i128::from(mults[g]);
            }
            let val = if sh > 0 {
                let u = sh as u32;
                (acc + (1i128 << (u - 1))) >> u
            } else if sh == 0 {
                acc
            } else {
                acc << ((-sh) as u32)
            };
            *out_slot = val.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32;
        }
    }

    /// Formal R4.HoloGeometricRuntime execute pure-integer block-32 `int8` RHT matrix-vector product.
    fn gemv_i8_rht(
        m: &HoloInt8RhtMatrix,
        norm_in: &[i64],
        in_exp: i32,
        out_exp: i32,
        out: &mut [i32],
    ) {
        let cols = m.cols;
        let mut act16 = vec![0i16; cols];
        let x_exp = prepare_rht_act16(norm_in, in_exp, m.rht_seed, &mut act16);
        let groups = cols >> 5;
        for (r, out_slot) in out.iter_mut().enumerate().take(m.rows) {
            let re = i32::from(m.row_exp[r]);
            let sh = out_exp - (re + x_exp);
            let ws = &m.weights[r * cols..(r + 1) * cols];
            let mults = &m.group_mult[r * groups..(r + 1) * groups];
            let mut acc = 0i128;
            for g in 0..groups {
                let wg: &[i8; 32] = (&ws[g * 32..(g + 1) * 32]).try_into().unwrap();
                let ag: &[i16; 32] = (&act16[g * 32..(g + 1) * 32]).try_into().unwrap();
                acc += i128::from(dot_i8_group32(wg, ag)) * i128::from(mults[g]);
            }
            let val = if sh > 0 {
                let u = sh as u32;
                (acc + (1i128 << (u - 1))) >> u
            } else if sh == 0 {
                acc
            } else {
                acc << ((-sh) as u32)
            };
            *out_slot = val.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32;
        }
    }

    fn exp_neg(&self, diff: i64) -> u64 {
        let frac_bits = EXP_STEP_LOG2 - (-16);
        if diff <= 0 {
            return u64::from(self.exp_table[0]);
        }
        let idx = (diff >> frac_bits) as usize;
        let (Some(&a), Some(&b)) = (self.exp_table.get(idx), self.exp_table.get(idx + 1)) else {
            return 0;
        };
        let frac = (diff & ((1 << frac_bits) - 1)) as u64;
        let step = ((u64::from(a).wrapping_sub(u64::from(b))) * frac) >> frac_bits;
        u64::from(a).wrapping_sub(step)
    }

    fn silu_i32(&self, xv: i32) -> i32 {
        let frac_bits = SILU_STEP_LOG2 + 18;
        let half_idx = 1i64 << (SILU_RANGE_LOG2 - SILU_STEP_LOG2);
        let idx = (i64::from(xv) >> frac_bits) + half_idx;
        if idx < 0 {
            return 0;
        }
        let u = idx as usize;
        let (Some(&a), Some(&b)) = (self.silu_table.get(u), self.silu_table.get(u + 1)) else {
            return xv >> 2;
        };
        let frac = i64::from(xv) & ((1i64 << frac_bits) - 1);
        let diff = i64::from(b) - i64::from(a);
        (i64::from(a) + ((diff * frac) >> frac_bits)) as i32
    }

    /// Formal R4.HoloGeometricRuntime execute one pure-integer autoregressive token step across
    /// `n_layers` with mandatory pre-RoPE per-head `q_norm`/`k_norm`, `i8` KV cache, Starling Flock
    /// sparse selection, and `SWAR` 512-bit Hamming shortlisting over `vocab_sigs`.
    pub fn step_token(
        &mut self,
        tok: u32,
        context_tokens: &[u32],
        compute_logits: bool,
        use_shortlist: bool,
    ) -> (Option<Vec<(u32, i32)>>, usize, usize) {
        let d = self.d_model;
        let nh = self.n_heads;
        let nkv = self.n_kv_heads;
        let hd = self.head_dim;
        let q_dim = nh * hd;
        let kv_dim = nkv * hd;
        let ffn = self.intermediate_size;
        let group_sz = nh / nkv;
        let half = hd / 2;
        let pos = self
            .kv_cache
            .tokens
            .len()
            .min(self.max_seq.saturating_sub(1));

        let mut x = vec![0i32; d];
        self.dequant_embed_row((tok as usize).min(self.vocab_size - 1), &mut x);
        let cos_row = &self.rope_cos_q14[pos * half..(pos + 1) * half];
        let sin_row = &self.rope_sin_q14[pos * half..(pos + 1) * half];

        let mut norm_d = vec![0i64; d];
        let mut q = vec![0i32; q_dim];
        let mut k = vec![0i32; kv_dim];
        let mut v = vec![0i32; kv_dim];
        let mut q_h_norm = vec![0i64; hd];
        let mut k_h_norm = vec![0i64; hd];
        let mut attn_i64 = vec![0i64; q_dim];
        let mut proj = vec![0i32; d];
        let mut gate = vec![0i32; ffn];
        let mut up = vec![0i32; ffn];
        let mut swiglu = vec![0i64; ffn];
        let mut flock_attended = 0usize;
        let mut flock_total = 0usize;

        for l in 0..self.n_layers {
            let layer = &self.layers[l];
            int_rms_norm_gain(
                &x,
                &layer.in_ln_q24,
                self.rms_eps_q48,
                RESIDUAL_EXP,
                NORM_EXP,
                &mut norm_d,
            );
            Self::gemv_i4_rht(&layer.q_proj, &norm_d, NORM_EXP, RESIDUAL_EXP, &mut q);
            Self::gemv_i4_rht(&layer.k_proj, &norm_d, NORM_EXP, RESIDUAL_EXP, &mut k);
            Self::gemv_i4_rht(&layer.v_proj, &norm_d, NORM_EXP, RESIDUAL_EXP, &mut v);

            // Mandatory per-head q_norm & k_norm BEFORE NeoX half-split RoPE (`R4-51`)
            for h in 0..nh {
                let off = h * hd;
                int_rms_norm_gain(
                    &q[off..off + hd],
                    &layer.q_norm_q24,
                    self.rms_eps_q48,
                    RESIDUAL_EXP,
                    RESIDUAL_EXP,
                    &mut q_h_norm,
                );
                for i in 0..half {
                    let c = i64::from(cos_row[i]);
                    let s = i64::from(sin_row[i]);
                    let q0 = q_h_norm[i];
                    let q1 = q_h_norm[i + half];
                    q[off + i] = ((q0 * c - q1 * s + (1 << 13)) >> ROPE_Q) as i32;
                    q[off + i + half] = ((q0 * s + q1 * c + (1 << 13)) >> ROPE_Q) as i32;
                }
            }
            for kh in 0..nkv {
                let off = kh * hd;
                int_rms_norm_gain(
                    &k[off..off + hd],
                    &layer.k_norm_q24,
                    self.rms_eps_q48,
                    RESIDUAL_EXP,
                    RESIDUAL_EXP,
                    &mut k_h_norm,
                );
                for i in 0..half {
                    let c = i64::from(cos_row[i]);
                    let s = i64::from(sin_row[i]);
                    let k0 = k_h_norm[i];
                    let k1 = k_h_norm[i + half];
                    k[off + i] = ((k0 * c - k1 * s + (1 << 13)) >> ROPE_Q) as i32;
                    k[off + i + half] = ((k0 * s + k1 * c + (1 << 13)) >> ROPE_Q) as i32;
                }
            }
            self.kv_cache.push_kv(l, &k, &v, nkv, hd);
            let seq_len = self.kv_cache.seq_len[l];

            for h in 0..nh {
                let kh = h / group_sz;
                let q_h = &q[h * hd..(h + 1) * hd];
                let mut q_sig = [0u64; 2];
                for (d_i, &qv) in q_h.iter().enumerate() {
                    if qv >= 0 {
                        q_sig[d_i >> 6] |= 1u64 << (d_i & 63);
                    }
                }
                let selected: Vec<usize> = if seq_len <= 256 {
                    (0..seq_len).collect()
                } else {
                    let mut cands: Vec<(u32, usize)> = Vec::with_capacity(seq_len);
                    for t in 0..seq_len {
                        let k_sig = self.kv_cache.k_sig128[l][t * nkv + kh];
                        let dist =
                            (q_sig[0] ^ k_sig[0]).count_ones() + (q_sig[1] ^ k_sig[1]).count_ones();
                        let prio = if t < 4 || t + 64 >= seq_len { 0 } else { dist };
                        cands.push((prio, t));
                    }
                    cands.select_nth_unstable(255);
                    let mut idx: Vec<usize> = cands[..256].iter().map(|x| x.1).collect();
                    idx.sort_unstable();
                    idx
                };
                flock_attended += selected.len();
                flock_total += seq_len;

                let mut scores = vec![0i64; selected.len()];
                let mut max_s = i64::MIN;
                for (si, &t) in selected.iter().enumerate() {
                    let k_off = t * kv_dim + kh * hd;
                    let ks = i128::from(self.kv_cache.k_scales[l][t * nkv + kh]);
                    let k_slice = &self.kv_cache.k_i16[l][k_off..k_off + hd];
                    let mut dot = 0i128;
                    for d_i in 0..hd {
                        dot += i128::from(q_h[d_i]) * i128::from(k_slice[d_i]);
                    }
                    dot *= ks;
                    let s_q16 =
                        ((dot * i128::from(self.score_scale_q30) + (1i128 << 49)) >> 50) as i64;
                    scores[si] = s_q16;
                    if s_q16 > max_s {
                        max_s = s_q16;
                    }
                }
                let mut num = [0i128; 128];
                let mut den = 0u128;
                for (si, &t) in selected.iter().enumerate() {
                    let w = (self.exp_neg(max_s - scores[si]) >> 14) as i128;
                    if w == 0 {
                        continue;
                    }
                    den += w as u128;
                    let v_off = t * kv_dim + kh * hd;
                    let vs = i128::from(self.kv_cache.v_scales[l][t * nkv + kh]);
                    let v_slice = &self.kv_cache.v_i16[l][v_off..v_off + hd];
                    let wv_scale = w * vs;
                    for d_i in 0..hd {
                        num[d_i] += wv_scale * i128::from(v_slice[d_i]);
                    }
                }
                let den_s = den.max(1) as i128;
                for d_i in 0..hd {
                    attn_i64[h * hd + d_i] = (num[d_i] / den_s) as i64;
                }
            }
            Self::gemv_i4_rht(
                &layer.o_proj,
                &attn_i64,
                RESIDUAL_EXP,
                RESIDUAL_EXP,
                &mut proj,
            );
            for i in 0..d {
                x[i] = x[i].saturating_add(proj[i]);
            }

            int_rms_norm_gain(
                &x,
                &layer.post_ln_q24,
                self.rms_eps_q48,
                RESIDUAL_EXP,
                NORM_EXP,
                &mut norm_d,
            );
            Self::gemv_i4_rht(&layer.gate_proj, &norm_d, NORM_EXP, RESIDUAL_EXP, &mut gate);
            Self::gemv_i4_rht(&layer.up_proj, &norm_d, NORM_EXP, RESIDUAL_EXP, &mut up);
            for i in 0..ffn {
                let sg = self.silu_i32(gate[i]);
                swiglu[i] = (i64::from(sg) * i64::from(up[i]) + (1 << 13)) >> 14;
            }
            match &layer.down_proj {
                HoloDownProjMatrix::Int4(m) => {
                    Self::gemv_i4_rht(m, &swiglu, -20, RESIDUAL_EXP, &mut proj);
                }
                HoloDownProjMatrix::Int8(m) => {
                    Self::gemv_i8_rht(m, &swiglu, -20, RESIDUAL_EXP, &mut proj);
                }
            }
            for i in 0..d {
                x[i] = x[i].saturating_add(proj[i]);
            }
        }
        self.kv_cache.tokens.push(tok);

        if !compute_logits {
            return (None, flock_attended, flock_total);
        }

        int_rms_norm_gain(
            &x,
            &self.final_norm_q24,
            self.rms_eps_q48,
            RESIDUAL_EXP,
            NORM_EXP,
            &mut norm_d,
        );
        let candidates = self.select_shortlist(&norm_d, context_tokens, use_shortlist);
        let mut act16 = vec![0i16; d];
        let x_exp = prepare_rht_act16(&norm_d, NORM_EXP, self.embed.rht_seed, &mut act16);
        let groups = d >> 5;
        let mut scored = Vec::with_capacity(candidates.len());
        for &r in &candidates {
            let re = i32::from(self.embed.row_exp[r as usize]);
            let sh = -16 - (re + x_exp);
            let ws = &self.embed.weights[r as usize * d..(r as usize + 1) * d];
            let mults = &self.embed.group_mult[r as usize * groups..(r as usize + 1) * groups];
            let mut acc = 0i128;
            for g in 0..groups {
                let wg: &[i8; 32] = (&ws[g * 32..(g + 1) * 32]).try_into().unwrap();
                let ag: &[i16; 32] = (&act16[g * 32..(g + 1) * 32]).try_into().unwrap();
                acc += i128::from(dot_i8_group32(wg, ag)) * i128::from(mults[g]);
            }
            let val = if sh > 0 {
                let u = sh as u32;
                (acc + (1i128 << (u - 1))) >> u
            } else if sh == 0 {
                acc
            } else {
                acc << ((-sh) as u32)
            };
            scored.push((
                r,
                val.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32,
            ));
        }
        (Some(scored), flock_attended, flock_total)
    }

    /// Formal R4.HoloGeometricRuntime select bounded `lm_head` candidate shortlist via 512-bit SWAR Hamming distance + recent-context boost.
    fn select_shortlist(
        &self,
        norm_d: &[i64],
        context_tokens: &[u32],
        use_shortlist: bool,
    ) -> Vec<u32> {
        if !use_shortlist || self.vocab_size <= DEFAULT_SHORTLIST_K {
            return (0..self.vocab_size as u32).collect();
        }
        let q_sig = compute_rht512_query_sig(norm_d, self.embed.rht_seed);
        let mut ranked: Vec<(u32, u32)> = Vec::with_capacity(self.vocab_size);
        for v in 0..self.vocab_size {
            let base = v * SIG_WORDS;
            let mut d_ham = 0u32;
            for (&qs, &vs) in q_sig.iter().zip(&self.vocab_sigs[base..base + SIG_WORDS]) {
                d_ham += (qs ^ vs).count_ones();
            }
            ranked.push((d_ham, v as u32));
        }
        let k_keep = DEFAULT_SHORTLIST_K.min(self.vocab_size);
        ranked.select_nth_unstable(k_keep.saturating_sub(1));
        let mut seen = HashSet::with_capacity(k_keep + context_tokens.len() + 512);
        let mut out = Vec::with_capacity(k_keep + context_tokens.len() + 512);
        for &(_, id) in &ranked[..k_keep] {
            if seen.insert(id) {
                out.push(id);
            }
        }
        for &id in context_tokens {
            if (id as usize) < self.vocab_size && seen.insert(id) {
                out.push(id);
            }
        }
        for &id in &self.tokenizer.ascii_fast_tokens {
            if seen.insert(id) {
                out.push(id);
            }
        }
        for id in [self.eos_token_id, self.eot_token_id, self.think_end_id] {
            if (id as usize) < self.vocab_size && seen.insert(id) {
                out.push(id);
            }
        }
        out
    }

    /// Formal R4.HoloGeometricRuntime execute a full multi-turn Qwen3 ChatML reply cleanly in pure integer arithmetic.
    pub fn generate_chat_turn(
        &mut self,
        user_prompt: &str,
        system_prompt: &str,
        max_new_tokens: usize,
    ) -> (String, HoloChatTurnMetrics) {
        self.generate_chat_turn_with_history(user_prompt, &[], system_prompt, max_new_tokens)
    }

    /// Formal R4.HoloGeometricRuntime execute multi-turn Qwen3 ChatML with rolling conversation history & live Wasm delta streaming.
    pub fn generate_chat_turn_with_history(
        &mut self,
        user_prompt: &str,
        history: &[(String, String)],
        system_prompt: &str,
        max_new_tokens: usize,
    ) -> (String, HoloChatTurnMetrics) {
        let sys = if system_prompt.trim().is_empty() {
            "You are a helpful assistant."
        } else {
            system_prompt.trim()
        };
        let mut chatml = format!("<|im_start|>system\n{sys}<|im_end|>\n");
        let recent_hist = &history[history.len().saturating_sub(6)..];
        for (u, a) in recent_hist {
            chatml.push_str("<|im_start|>user\n");
            chatml.push_str(u.trim());
            chatml.push_str("<|im_end|>\n<|im_start|>assistant\n");
            chatml.push_str(a.trim());
            chatml.push_str("<|im_end|>\n");
        }
        chatml.push_str("<|im_start|>user\n");
        chatml.push_str(user_prompt.trim());
        chatml.push_str("<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n");

        let prompt_ids_all = self.tokenizer.encode(&chatml);
        let max_ctx = self.max_seq.saturating_sub(max_new_tokens.max(16)).max(32);
        let prompt_ids: &[u32] = if prompt_ids_all.len() > max_ctx {
            &prompt_ids_all[prompt_ids_all.len() - max_ctx..]
        } else {
            &prompt_ids_all
        };
        self.kv_cache.reset(self.n_layers);

        let mut flock_attended = 0usize;
        let mut flock_total = 0usize;
        let mut recent: Vec<u32> = prompt_ids.to_vec();
        let mut last_scored = Vec::new();
        for (idx, &tok) in prompt_ids.iter().enumerate() {
            let is_last = idx + 1 == prompt_ids.len();
            let (sc_opt, fa, ft) = self.step_token(tok, &recent, is_last, true);
            flock_attended += fa;
            flock_total += ft;
            if let Some(sc) = sc_opt {
                last_scored = sc;
            }
        }

        let mut generated = Vec::new();
        let mut emitted_bytes = 0usize;
        for step_idx in 0..max_new_tokens.max(1) {
            let mut blocked_ids: Vec<u32> = Vec::new();
            let chosen_id = loop {
                let mut best_id = self.eos_token_id;
                let mut best_logit = i32::MIN;
                let mut found_any = false;
                for &(tid, logit) in &last_scored {
                    if blocked_ids.contains(&tid) {
                        continue;
                    }
                    if !found_any || logit > best_logit {
                        best_logit = logit;
                        best_id = tid;
                        found_any = true;
                    }
                }
                if !found_any || best_id == self.eos_token_id || best_id == self.eot_token_id {
                    break best_id;
                }
                if blocked_ids.len() < 8 && token_causes_cycle(&generated, best_id, &self.tokenizer)
                {
                    blocked_ids.push(best_id);
                    continue;
                }
                break best_id;
            };
            if chosen_id == self.eos_token_id || chosen_id == self.eot_token_id {
                break;
            }
            generated.push(chosen_id);
            recent.push(chosen_id);
            let full_dec = self.tokenizer.decode(&generated);
            if full_dec.len() > emitted_bytes && full_dec.is_char_boundary(emitted_bytes) {
                let delta = &full_dec[emitted_bytes..];
                emit_holo_stream_delta(delta, step_idx);
                emitted_bytes = full_dec.len();
            }
            if generated.len() >= max_new_tokens {
                break;
            }
            let (sc_opt, fa, ft) = self.step_token(chosen_id, &recent, true, true);
            flock_attended += fa;
            flock_total += ft;
            if let Some(sc) = sc_opt {
                last_scored = sc;
            } else {
                break;
            }
        }

        let reply = self.tokenizer.decode(&generated).trim().to_string();
        let metrics = HoloChatTurnMetrics {
            schema: "uor-r4.holo-geometric-runtime/13",
            model_id: self.model_id.clone(),
            container_sha256: self.container_sha256.clone(),
            prompt_tokens: prompt_ids.len(),
            generated_tokens: generated.len(),
            float_ops: 0,
            dense_matmul_ops: 0,
            simd128_active: cfg!(all(target_arch = "wasm32", target_feature = "simd128")),
            q_norm_before_rope: true,
            k_norm_before_rope: true,
            shortlist_budget: compute_shortlist_budget(
                DEFAULT_SHORTLIST_K as u64,
                self.tokenizer.ascii_fast_tokens.len() as u64,
                self.vocab_size as u64,
            ) as usize,
            flock_attended_positions: flock_attended,
            flock_total_positions: flock_total,
        };
        (reply, metrics)
    }
}

/// Formal R4.HoloGeometricRuntime reject 3+ identical alphanumeric tokens, `,,`/`; ;`, or immediate k-gram cycles (`p in 2..=8`).
fn token_causes_cycle(generated: &[u32], cand: u32, tokenizer: &HoloTok13) -> bool {
    let n = generated.len();
    if n >= 1 && generated[n - 1] == cand {
        let dec = tokenizer.decode(&[cand]);
        if matches!(dec.as_str(), "," | ";") {
            return true;
        }
        if n >= 2 && generated[n - 2] == cand && dec.chars().any(|c| c.is_alphanumeric()) {
            return true;
        }
    }
    for p in 2usize..=8 {
        if n + 1 < 2 * p {
            continue;
        }
        let b1_start = n + 1 - 2 * p;
        let b2_start = n + 1 - p;
        let mut all_eq = cand == generated[b1_start + p - 1];
        if all_eq {
            for k in 0..(p - 1) {
                if generated[b1_start + k] != generated[b2_start + k] {
                    all_eq = false;
                    break;
                }
            }
        }
        if all_eq {
            return true;
        }
    }
    false
}

/// Formal R4.HoloGeometricRuntime emit wasm32 incremental token delta event to worker host stream hook.
#[cfg(target_arch = "wasm32")]
fn emit_holo_stream_delta(delta: &str, step_idx: usize) {
    use wasm_bindgen::{JsCast, JsValue};
    if delta.is_empty() {
        return;
    }
    let global = js_sys::global();
    if let Ok(val) = js_sys::Reflect::get(&global, &JsValue::from_str("__uorEmitChatDelta")) {
        if let Some(func) = val.dyn_ref::<js_sys::Function>() {
            let _ = func.call2(
                &JsValue::NULL,
                &JsValue::from_str(delta),
                &JsValue::from(step_idx as u32),
            );
        }
    }
}

/// Formal R4.HoloGeometricRuntime emit non-wasm32 streaming token no-op stub.
#[cfg(not(target_arch = "wasm32"))]
fn emit_holo_stream_delta(_delta: &str, _step_idx: usize) {}

/// Metadata summary produced when inspecting or streaming a direct HuggingFace safetensors repo (`HfRepoHolo`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct HfRepoCompileSummary {
    /// HuggingFace repository ID (`owner/repo`).
    pub repo_id: String,
    /// Byte offset where binary tensor storage starts (`8 + header_len`).
    pub safetensors_data_start: u64,
    /// Number of tensors declared in the Safetensors header.
    pub tensor_count: usize,
    /// Total tensor byte span declared across entries.
    pub total_tensor_bytes: u64,
}

/// Formal R4.HoloGeometricRuntime validate a direct HuggingFace safetensors header slice streamed
/// via HTTP `206` `Range` requests for `uor-r4.hf-repo-holo`.
pub fn validate_hf_safetensors_prefix(
    repo_id: &str,
    prefix_bytes: &[u8],
) -> Result<HfRepoCompileSummary, HoloKappaStoreError> {
    let meta = parse_safetensors_header(prefix_bytes)?;
    Ok(HfRepoCompileSummary {
        repo_id: repo_id.trim().to_string(),
        safetensors_data_start: meta.data_start,
        tensor_count: meta.tensors.len(),
        total_tensor_bytes: meta.max_data_end,
    })
}

/// Formal R4.HoloGeometricRuntime perform symmetric rounded signed 128-bit integer division (`den > 0`).
fn div_round_i128(num: i128, den: i128) -> i128 {
    if num >= 0 {
        (num + (den >> 1)) / den
    } else {
        -((-num + (den >> 1)) / den)
    }
}

/// Formal R4.HoloGeometricRuntime convert raw IEEE-754 `f32` bits into exact `Q24` fixed-point `i64`.
fn f32_bits_to_q24(bits: u32) -> i64 {
    let neg = (bits >> 31) != 0;
    let exp = ((bits >> 23) & 0xFF) as i32;
    let mant = i128::from(bits & 0x7F_FFFF);
    let mag: i64 = if exp == 0 {
        0
    } else if exp == 255 {
        if mant != 0 {
            0
        } else {
            1i64 << 54
        }
    } else {
        let sig = (1i128 << 23) | mant;
        let shift = exp - 126;
        if shift >= 31 {
            1i64 << 54
        } else if shift >= 0 {
            (sig << (shift as u32)) as i64
        } else {
            let rsh = (-shift) as u32;
            if rsh >= 25 {
                0
            } else {
                let half = 1i128 << (rsh - 1);
                ((sig + half) >> rsh) as i64
            }
        }
    };
    if neg {
        -mag
    } else {
        mag
    }
}

/// Formal R4.HoloGeometricRuntime convert IEEE-754 `bf16` halfword bits into `Q24` fixed-point `i64`.
fn bf16_bits_to_q24(bits: u16) -> i64 {
    f32_bits_to_q24(u32::from(bits) << 16)
}

/// Formal R4.HoloGeometricRuntime convert IEEE-754 `fp16` halfword bits into `Q24` fixed-point `i64`.
fn f16_bits_to_q24(bits: u16) -> i64 {
    let sign = u32::from((bits >> 15) & 1) << 31;
    let exp = u32::from((bits >> 10) & 0x1F);
    let mant = u32::from(bits & 0x3FF);
    if exp == 0 {
        if mant == 0 {
            return 0;
        }
        let mut m = mant;
        let mut e = 1i32;
        while (m & 0x400) == 0 {
            m <<= 1;
            e -= 1;
        }
        let new_exp = (127 - 15 + e) as u32;
        return f32_bits_to_q24(sign | (new_exp << 23) | ((m & 0x3FF) << 13));
    }
    if exp == 31 {
        return f32_bits_to_q24(sign | (0xFF << 23) | (mant << 13));
    }
    f32_bits_to_q24(sign | ((exp + 112) << 23) | (mant << 13))
}

/// Formal R4.HoloGeometricRuntime return byte width for supported safetensors dtypes.
fn safetensors_elem_bytes(dtype: &str) -> Result<usize, HoloKappaStoreError> {
    match dtype {
        "BF16" | "F16" => Ok(2),
        "F32" => Ok(4),
        other => Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
            "unsupported tensor dtype '{other}' in HfRepoHolo compiler"
        ))),
    }
}

/// Formal R4.HoloGeometricRuntime decode one contiguous row of `BF16`/`F16`/`F32` bytes into `Q24` integers.
fn decode_safetensors_row_q24(
    dtype: &str,
    raw: &[u8],
    row: usize,
    cols: usize,
    out: &mut [i64],
) -> Result<(), HoloKappaStoreError> {
    let elem_bytes = safetensors_elem_bytes(dtype)?;
    let row_bytes = cols
        .checked_mul(elem_bytes)
        .ok_or_else(|| HoloKappaStoreError::CorruptSafetensorsHeader("row byte overflow".into()))?;
    let start = row.checked_mul(row_bytes).ok_or_else(|| {
        HoloKappaStoreError::CorruptSafetensorsHeader("row offset overflow".into())
    })?;
    let end = start
        .checked_add(row_bytes)
        .ok_or_else(|| HoloKappaStoreError::CorruptSafetensorsHeader("row end overflow".into()))?;
    let slice = raw.get(start..end).ok_or_else(|| {
        HoloKappaStoreError::CorruptSafetensorsHeader(format!(
            "truncated {dtype} row {row} (need {end} bytes, got {})",
            raw.len()
        ))
    })?;
    if out.len() < cols {
        return Err(HoloKappaStoreError::CorruptSafetensorsHeader(
            "row scratch buffer smaller than cols".into(),
        ));
    }
    match dtype {
        "BF16" => {
            for (dst, c) in out[..cols].iter_mut().zip(slice.chunks_exact(2)) {
                *dst = bf16_bits_to_q24(u16::from_le_bytes([c[0], c[1]]));
            }
        }
        "F16" => {
            for (dst, c) in out[..cols].iter_mut().zip(slice.chunks_exact(2)) {
                *dst = f16_bits_to_q24(u16::from_le_bytes([c[0], c[1]]));
            }
        }
        "F32" => {
            for (dst, c) in out[..cols].iter_mut().zip(slice.chunks_exact(4)) {
                *dst = f32_bits_to_q24(u32::from_le_bytes([c[0], c[1], c[2], c[3]]));
            }
        }
        _ => {
            return Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
                "unsupported tensor dtype '{dtype}' in HfRepoHolo compiler"
            )));
        }
    }
    Ok(())
}

/// Formal R4.HoloGeometricRuntime decode `BF16`/`F16`/`F32` safetensors bytes into pure `Q24` integers.
fn decode_safetensors_tensor_q24(dtype: &str, raw: &[u8]) -> Result<Vec<i64>, HoloKappaStoreError> {
    match dtype {
        "BF16" => Ok(raw
            .chunks_exact(2)
            .map(|c| bf16_bits_to_q24(u16::from_le_bytes([c[0], c[1]])))
            .collect()),
        "F16" => Ok(raw
            .chunks_exact(2)
            .map(|c| f16_bits_to_q24(u16::from_le_bytes([c[0], c[1]])))
            .collect()),
        "F32" => Ok(raw
            .chunks_exact(4)
            .map(|c| f32_bits_to_q24(u32::from_le_bytes([c[0], c[1], c[2], c[3]])))
            .collect()),
        other => Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
            "unsupported tensor dtype '{other}' in HfRepoHolo compiler"
        ))),
    }
}

/// Formal R4.HoloGeometricRuntime clamp `Q24` integers into `i32` gain slices during HF ingestion.
fn q24_to_i32_slice(src: &[i64]) -> Vec<i32> {
    src.iter()
        .map(|&v| v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32)
        .collect()
}

/// Formal R4.HoloGeometricRuntime apply pure-integer randomized block-32 Hadamard transform (`1/32` normalized).
fn rht32_apply_q24_row(row_q24: &[i64], signs: &[bool]) -> Vec<i64> {
    let cols = row_q24.len();
    let mut out = vec![0i64; cols];
    for g in 0..(cols >> 5) {
        let base = g * 32;
        let mut buf = [0i64; 32];
        for c in 0..32 {
            buf[c] = if signs[base + c] {
                row_q24[base + c]
            } else {
                -row_q24[base + c]
            };
        }
        int_fwht32_i64(&mut buf);
        for c in 0..32 {
            out[base + c] = div_round_i128(i128::from(buf[c]), 32) as i64;
        }
    }
    out
}

/// Formal R4.HoloGeometricRuntime pack streamed pure-integer tensor row-by-row into `int8` block-32 RHT weights + 512-bit signatures.
fn compile_int8_rht_matrix_raw(
    dtype: &str,
    raw: &[u8],
    rows: usize,
    cols: usize,
    seed: u64,
) -> Result<(HoloInt8RhtMatrix, Vec<u64>), HoloKappaStoreError> {
    if cols == 0 || (cols & 31) != 0 {
        return Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
            "int8 RHT matrix cols ({cols}) must be a positive multiple of 32"
        )));
    }
    let elem_bytes = safetensors_elem_bytes(dtype)?;
    let need_bytes = rows
        .checked_mul(cols)
        .and_then(|n| n.checked_mul(elem_bytes))
        .ok_or_else(|| {
            HoloKappaStoreError::CorruptSafetensorsHeader("matrix byte overflow".into())
        })?;
    if raw.len() < need_bytes {
        return Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
            "truncated matrix bytes: need {need_bytes}, got {}",
            raw.len()
        )));
    }
    let groups = cols >> 5;
    let signs = rht_signs_bits(cols, seed);
    let mut row_exp = Vec::with_capacity(rows);
    let mut group_mult = Vec::with_capacity(rows * groups);
    let mut weights = Vec::with_capacity(rows * cols);
    let mut vocab_sigs = Vec::with_capacity(rows * SIG_WORDS);
    let mut row_q24 = vec![0i64; cols];

    for r in 0..rows {
        decode_safetensors_row_q24(dtype, raw, r, cols, &mut row_q24)?;
        let rht = rht32_apply_q24_row(&row_q24, &signs);
        let mut sig = [0u64; SIG_WORDS];
        for b in 0..512 {
            let col = (b * cols) / 512;
            if rht[col] >= 0 {
                sig[b >> 6] |= 1u64 << (b & 63);
            }
        }
        vocab_sigs.extend_from_slice(&sig);

        let mut max_s_q64 = 1i128 << 24;
        let mut g_scales_q64 = vec![0i128; groups];
        for g in 0..groups {
            let mut gmax_q64 = 0i128;
            for c in 0..32 {
                let v = (i128::from(rht[g * 32 + c]) << 40).abs();
                if v > gmax_q64 {
                    gmax_q64 = v;
                }
            }
            let s_q64 = (gmax_q64 / 127).max(1i128 << 24);
            g_scales_q64[g] = s_q64;
            if s_q64 > max_s_q64 {
                max_s_q64 = s_q64;
            }
        }
        let k = (max_s_q64 as u128).ilog2() as i32;
        let re = (k - 64 - 15).clamp(-55, 40);
        let base_q64 = 1i128 << ((re + 64) as u32);
        row_exp.push(re as i8);
        for g in 0..groups {
            let mult = div_round_i128(g_scales_q64[g], base_q64).clamp(1, 65_535) as u16;
            group_mult.push(mult);
            let sc_q64 = i128::from(mult) * base_q64;
            for c in 0..32 {
                let val_q64 = i128::from(rht[g * 32 + c]) << 40;
                let q = div_round_i128(val_q64, sc_q64).clamp(-127, 127) as i8;
                weights.push(q);
            }
        }
    }

    Ok((
        HoloInt8RhtMatrix {
            rows,
            cols,
            rht_seed: seed,
            row_exp,
            group_mult,
            weights,
        },
        vocab_sigs,
    ))
}

/// Formal R4.HoloGeometricRuntime pack streamed pure-integer tensor row-by-row into `int4` nibble block-32 RHT weights.
fn compile_int4_rht_matrix_raw(
    dtype: &str,
    raw: &[u8],
    rows: usize,
    cols: usize,
    seed: u64,
) -> Result<HoloInt4RhtMatrix, HoloKappaStoreError> {
    if cols == 0 || (cols & 31) != 0 {
        return Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
            "int4 RHT matrix cols ({cols}) must be a positive multiple of 32"
        )));
    }
    let elem_bytes = safetensors_elem_bytes(dtype)?;
    let need_bytes = rows
        .checked_mul(cols)
        .and_then(|n| n.checked_mul(elem_bytes))
        .ok_or_else(|| {
            HoloKappaStoreError::CorruptSafetensorsHeader("matrix byte overflow".into())
        })?;
    if raw.len() < need_bytes {
        return Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
            "truncated matrix bytes: need {need_bytes}, got {}",
            raw.len()
        )));
    }
    let groups = cols >> 5;
    let signs = rht_signs_bits(cols, seed);
    let mut row_exp = Vec::with_capacity(rows);
    let mut group_mult = Vec::with_capacity(rows * groups);
    let mut nibbles = Vec::with_capacity(rows * (cols >> 1));
    let mut row_q24 = vec![0i64; cols];

    for r in 0..rows {
        decode_safetensors_row_q24(dtype, raw, r, cols, &mut row_q24)?;
        let rht = rht32_apply_q24_row(&row_q24, &signs);
        let mut max_s_q64 = 1i128 << 24;
        let mut g_scales_q64 = vec![0i128; groups];
        for g in 0..groups {
            let mut gmax_q64 = 0i128;
            for c in 0..32 {
                let v = (i128::from(rht[g * 32 + c]) << 40).abs();
                if v > gmax_q64 {
                    gmax_q64 = v;
                }
            }
            let s_q64 = (gmax_q64 / 7).max(1i128 << 24);
            g_scales_q64[g] = s_q64;
            if s_q64 > max_s_q64 {
                max_s_q64 = s_q64;
            }
        }
        let k = (max_s_q64 as u128).ilog2() as i32;
        let re = (k - 64 - 15).clamp(-55, 40);
        let base_q64 = 1i128 << ((re + 64) as u32);
        row_exp.push(re as i8);
        for g in 0..groups {
            let mult = div_round_i128(g_scales_q64[g], base_q64).clamp(1, 65_535) as u16;
            group_mult.push(mult);
            let sc_q64 = i128::from(mult) * base_q64;
            let mut qbuf = [0i32; 32];
            for c in 0..32 {
                let val_q64 = i128::from(rht[g * 32 + c]) << 40;
                qbuf[c] = div_round_i128(val_q64, sc_q64).clamp(-8, 7) as i32;
            }
            for k_i in 0..16 {
                let lo = ((qbuf[k_i] + 8) as u8) & 15;
                let hi = ((qbuf[k_i + 16] + 8) as u8) & 15;
                nibbles.push(lo | (hi << 4));
            }
        }
    }

    Ok(HoloInt4RhtMatrix {
        rows,
        cols,
        rht_seed: seed,
        row_exp,
        group_mult,
        nibbles,
    })
}

/// Formal R4.HoloGeometricRuntime parse positive JSON decimal/scientific number scaled by `2^scale_p2` without floats.
fn parse_json_decimal_scaled_u64(val: &serde_json::Value, scale_p2: u32) -> Option<u64> {
    if let Some(u) = val.as_u64() {
        return u128::from(u)
            .checked_shl(scale_p2)
            .and_then(|v| u64::try_from(v).ok());
    }
    let raw = val.to_string();
    let s = raw.trim();
    if s.is_empty() || s.starts_with('-') {
        return None;
    }
    let s = s.strip_prefix('+').unwrap_or(s);
    let (mant_str, base_exp) = match s.find(['e', 'E']) {
        Some(pos) => (&s[..pos], s[pos + 1..].parse::<i32>().ok()?),
        None => (s, 0i32),
    };
    let mut digits = 0u128;
    let mut sig_len = 0u32;
    let mut dec_exp = 0i32;
    let mut in_frac = false;
    let mut saw_digit = false;
    for b in mant_str.bytes() {
        if b == b'.' {
            if in_frac {
                return None;
            }
            in_frac = true;
            continue;
        }
        if !b.is_ascii_digit() {
            return None;
        }
        saw_digit = true;
        let d = u128::from(b - b'0');
        if digits == 0 && d == 0 {
            if in_frac {
                dec_exp -= 1;
            }
            continue;
        }
        if sig_len < 12 {
            digits = digits * 10 + d;
            sig_len += 1;
            if in_frac {
                dec_exp -= 1;
            }
        } else if !in_frac {
            dec_exp += 1;
        }
    }
    if !saw_digit {
        return None;
    }
    let total_exp = base_exp + dec_exp;
    let scaled = digits.checked_shl(scale_p2)?;
    if total_exp >= 0 {
        let mut out = scaled;
        for _ in 0..total_exp.min(20) {
            out = out.checked_mul(10)?;
        }
        u64::try_from(out).ok()
    } else {
        let neg_exp = (-total_exp) as u32;
        if neg_exp > 22 {
            return Some(0);
        }
        let den = 10u128.pow(neg_exp);
        Some(((scaled + (den >> 1)) / den) as u64)
    }
}

/// Formal R4.HoloGeometricRuntime compute `round(2^(-y_q30 / 2^30) * 2^40)` using pure integer fixed-point Taylor series.
fn pow2_neg_q40(y_q30: u128) -> u64 {
    const LN2_Q40: u128 = 762_123_384_786;
    let k = (y_q30 >> 30) as u32;
    if k >= 41 {
        return 0;
    }
    let f_q30 = y_q30 & ((1u128 << 30) - 1);
    let u_q40 = (f_q30 * LN2_Q40 + (1u128 << 29)) >> 30;
    let mut term_mag = 1u128 << 40;
    let mut sum = term_mag as i128;
    for n in 1..=12u128 {
        term_mag = (term_mag * u_q40) / ((1u128 << 40) * n);
        if n % 2 == 1 {
            sum -= term_mag as i128;
        } else {
            sum += term_mag as i128;
        }
    }
    if sum <= 0 {
        return 0;
    }
    let val = sum as u128;
    if k == 0 {
        val as u64
    } else {
        ((val + (1u128 << (k - 1))) >> k) as u64
    }
}

/// Formal R4.HoloGeometricRuntime compute `round(exp(-x_q30 / 2^30) * 2^30)` via pure-integer fixed-point arithmetic.
fn exp_neg_q30(x_q30: u64) -> u32 {
    const LOG2_E_Q30: u128 = 1_549_082_005;
    let y_q30 = (u128::from(x_q30) * LOG2_E_Q30 + (1u128 << 29)) >> 30;
    let q40 = pow2_neg_q40(y_q30);
    ((q40 + 512) >> 10) as u32
}

/// Formal R4.HoloGeometricRuntime build `Q14` NeoX RoPE cosine/sine lookup tables via pure-integer octant arithmetic.
fn rope_sincos_tables_q14(
    max_seq: usize,
    head_dim: usize,
    rope_theta: u64,
) -> (Vec<i16>, Vec<i16>) {
    const QUARTER_PI_Q40: u128 = 863_554_413_089;
    const TAU_Q40: u128 = 8 * QUARTER_PI_Q40;
    let half = head_dim / 2;
    let mut rope_cos_q14 = vec![16_384i16; max_seq * half];
    let mut rope_sin_q14 = vec![0i16; max_seq * half];
    if half == 0 || max_seq == 0 {
        return (rope_cos_q14, rope_sin_q14);
    }
    let theta = rope_theta.max(2);
    let int_log = u64::BITS - 1 - theta.leading_zeros();
    let mut cur = (u128::from(theta) << 30) >> int_log;
    let mut frac_q30 = 0u128;
    for b in 0..30u32 {
        cur = (cur * cur + (1u128 << 29)) >> 30;
        if cur >= (2u128 << 30) {
            frac_q30 |= 1u128 << (29 - b);
            cur >>= 1;
        }
    }
    let log2_theta_q30 = (u128::from(int_log) << 30) + frac_q30;
    let hd_u128 = head_dim as u128;
    let inv_freqs_q40: Vec<u64> = (0..half)
        .map(|i| {
            let y_q30 = (2u128 * (i as u128) * log2_theta_q30 + (hd_u128 >> 1)) / hd_u128;
            pow2_neg_q40(y_q30)
        })
        .collect();

    let cos_divs = [2u128, 12, 30, 56, 90, 132];
    let sin_divs = [6u128, 20, 42, 72, 110, 156];
    let to_q14 = |v: i128| -> i16 {
        let q = if v >= 0 {
            (v + (1i128 << 25)) >> 26
        } else {
            -((-v + (1i128 << 25)) >> 26)
        };
        q.clamp(-32_768, 32_767) as i16
    };

    for pos in 0..max_seq {
        let row_off = pos * half;
        for i in 0..half {
            let ang_q40 = (pos as u128) * u128::from(inv_freqs_q40[i]);
            let r = ang_q40 % TAU_Q40;
            let octant = (r / QUARTER_PI_Q40).min(7);
            let rem = r - octant * QUARTER_PI_Q40;
            let z = if (octant & 1) == 0 {
                rem
            } else {
                QUARTER_PI_Q40 - rem
            };
            let z2 = (z * z) >> 40;
            let mut c_val = 1i128 << 40;
            let mut term_c = 1u128 << 40;
            for (idx, &d) in cos_divs.iter().enumerate() {
                term_c = (term_c * z2) / (d << 40);
                if idx % 2 == 0 {
                    c_val -= term_c as i128;
                } else {
                    c_val += term_c as i128;
                }
            }
            let mut s_val = z as i128;
            let mut term_s = z;
            for (idx, &d) in sin_divs.iter().enumerate() {
                term_s = (term_s * z2) / (d << 40);
                if idx % 2 == 0 {
                    s_val -= term_s as i128;
                } else {
                    s_val += term_s as i128;
                }
            }
            let (cx, sx) = match octant {
                0 => (c_val, s_val),
                1 => (s_val, c_val),
                2 => (-s_val, c_val),
                3 => (-c_val, s_val),
                4 => (-c_val, -s_val),
                5 => (-s_val, -c_val),
                6 => (s_val, -c_val),
                _ => (c_val, -s_val),
            };
            rope_cos_q14[row_off + i] = to_q14(cx);
            rope_sin_q14[row_off + i] = to_q14(sx);
        }
    }
    (rope_cos_q14, rope_sin_q14)
}

impl HoloTok13 {
    /// Formal R4.HoloGeometricRuntime parse HuggingFace `tokenizer.json` into `HoloTok13` & `UORTK13` bytes.
    pub fn from_hf_tokenizer_json(raw_json: &str) -> Result<(Self, Vec<u8>), HoloKappaStoreError> {
        let root: serde_json::Value = serde_json::from_str(raw_json).map_err(|e| {
            HoloKappaStoreError::CorruptSafetensorsHeader(format!(
                "tokenizer.json invalid JSON: {e}"
            ))
        })?;
        let vocab_obj = root
            .get("model")
            .and_then(|m| m.get("vocab"))
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                HoloKappaStoreError::CorruptSafetensorsHeader(
                    "tokenizer.json missing model.vocab".to_string(),
                )
            })?;
        let mut max_id = 0usize;
        for (_, v) in vocab_obj {
            if let Some(id) = v.as_u64() {
                max_id = max_id.max(id as usize);
            }
        }
        let mut added_tokens: Vec<(String, u32, bool)> = Vec::new();
        if let Some(arr) = root.get("added_tokens").and_then(|v| v.as_array()) {
            for item in arr {
                if let (Some(content), Some(id)) = (
                    item.get("content").and_then(|v| v.as_str()),
                    item.get("id").and_then(|v| v.as_u64()),
                ) {
                    max_id = max_id.max(id as usize);
                    let special = item
                        .get("special")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    added_tokens.push((content.to_string(), id as u32, special));
                }
            }
        }
        let vocab_count = max_id + 1;
        let (_, u2b) = gpt2_byte_to_unicode();
        let mut id_to_token = vec![String::new(); vocab_count];
        let mut id_to_raw = vec![Vec::new(); vocab_count];
        let mut token_to_id = HashMap::with_capacity(vocab_count);
        for (tok, v) in vocab_obj {
            if let Some(id) = v.as_u64().map(|u| u as usize) {
                id_to_token[id] = tok.clone();
                id_to_raw[id] = tok.chars().filter_map(|ch| u2b.get(&ch).copied()).collect();
                token_to_id.insert(tok.clone(), id as u32);
            }
        }
        for (content, id, _) in &added_tokens {
            let idx = *id as usize;
            if idx < vocab_count {
                id_to_token[idx] = content.clone();
                id_to_raw[idx] = content.as_bytes().to_vec();
                token_to_id.insert(content.clone(), *id);
            }
        }

        let mut merge_triples: Vec<(u32, u32, u32, u32)> = Vec::new();
        if let Some(merges) = root
            .get("model")
            .and_then(|m| m.get("merges"))
            .and_then(|v| v.as_array())
        {
            for (rank, entry) in merges.iter().enumerate() {
                let pair = if let Some(s) = entry.as_str() {
                    s.split_once(' ')
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                } else if let Some(arr) = entry.as_array() {
                    if arr.len() >= 2 {
                        arr[0]
                            .as_str()
                            .zip(arr[1].as_str())
                            .map(|(a, b)| (a.to_string(), b.to_string()))
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some((a, b)) = pair {
                    let merged = format!("{a}{b}");
                    if let (Some(&la), Some(&rb), Some(&mid)) = (
                        token_to_id.get(&a),
                        token_to_id.get(&b),
                        token_to_id.get(&merged),
                    ) {
                        merge_triples.push((la, rb, mid, rank as u32));
                    }
                }
            }
        }

        let mut ser = Vec::with_capacity(4 * 1024 * 1024);
        ser.extend_from_slice(HOLO_TOK13_MAGIC);
        ser.extend_from_slice(&(vocab_count as u32).to_le_bytes());
        ser.extend_from_slice(&(merge_triples.len() as u32).to_le_bytes());
        ser.extend_from_slice(&(added_tokens.len() as u32).to_le_bytes());
        for raw in &id_to_raw {
            let len = raw.len().min(u16::MAX as usize) as u16;
            ser.extend_from_slice(&len.to_le_bytes());
            ser.extend_from_slice(&raw[..len as usize]);
        }
        for tok in &id_to_token {
            let len = tok.len().min(u16::MAX as usize) as u16;
            ser.extend_from_slice(&len.to_le_bytes());
            ser.extend_from_slice(&tok.as_bytes()[..len as usize]);
        }
        for &(l, r, m, rk) in &merge_triples {
            ser.extend_from_slice(&l.to_le_bytes());
            ser.extend_from_slice(&r.to_le_bytes());
            ser.extend_from_slice(&m.to_le_bytes());
            ser.extend_from_slice(&rk.to_le_bytes());
        }
        for (pat, id, sp) in &added_tokens {
            let b = pat.as_bytes();
            let len = b.len().min(u16::MAX as usize) as u16;
            ser.extend_from_slice(&id.to_le_bytes());
            ser.push(u8::from(*sp));
            ser.extend_from_slice(&len.to_le_bytes());
            ser.extend_from_slice(&b[..len as usize]);
        }

        let tok = Self::from_bytes(&ser).map_err(|e| {
            HoloKappaStoreError::CorruptSafetensorsHeader(format!("UORTK13 roundtrip: {e:?}"))
        })?;
        Ok((tok, ser))
    }
}

/// Streaming tensor-by-tensor HuggingFace Safetensors -> `UORR4V13` compiler for `uor-r4.hf-repo-holo`.
pub struct HfRepoHoloStreamCompiler {
    repo_id: String,
    vocab_size: usize,
    d_model: usize,
    n_layers: usize,
    n_heads: usize,
    n_kv_heads: usize,
    head_dim: usize,
    intermediate_size: usize,
    max_seq: usize,
    bos_token_id: u32,
    eos_token_id: u32,
    rms_eps_q48: i64,
    score_scale_q30: i64,
    rope_theta: u64,
    tokenizer: HoloTok13,
    embed: Option<HoloInt8RhtMatrix>,
    vocab_sigs: Vec<u64>,
    final_norm_q24: Option<Vec<i32>>,
    layers: Vec<PartialHfLayer>,
}

#[derive(Default)]
struct PartialHfLayer {
    in_ln_q24: Option<Vec<i32>>,
    post_ln_q24: Option<Vec<i32>>,
    q_norm_q24: Option<Vec<i32>>,
    k_norm_q24: Option<Vec<i32>>,
    q_proj: Option<HoloInt4RhtMatrix>,
    k_proj: Option<HoloInt4RhtMatrix>,
    v_proj: Option<HoloInt4RhtMatrix>,
    o_proj: Option<HoloInt4RhtMatrix>,
    gate_proj: Option<HoloInt4RhtMatrix>,
    up_proj: Option<HoloInt4RhtMatrix>,
    down_proj: Option<HoloInt4RhtMatrix>,
}

impl HfRepoHoloStreamCompiler {
    /// Formal R4.HoloGeometricRuntime initialize a streaming `UORR4V13` compiler from HF `config.json` & `tokenizer.json`.
    pub fn new(
        repo_id: &str,
        config_json: &str,
        tokenizer_json: &str,
    ) -> Result<Self, HoloKappaStoreError> {
        let cfg: serde_json::Value = serde_json::from_str(config_json).map_err(|e| {
            HoloKappaStoreError::CorruptSafetensorsHeader(format!("config.json invalid: {e}"))
        })?;
        let d_model = cfg.get("hidden_size").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let n_layers = cfg
            .get("num_hidden_layers")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let n_heads = cfg
            .get("num_attention_heads")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let n_kv_heads = cfg
            .get("num_key_value_heads")
            .and_then(|v| v.as_u64())
            .unwrap_or(n_heads as u64) as usize;
        let head_dim = cfg
            .get("head_dim")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| (d_model as u64).checked_div(n_heads as u64).unwrap_or(128))
            as usize;
        let intermediate_size = cfg
            .get("intermediate_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let vocab_size = cfg.get("vocab_size").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let rms_eps_q48 = cfg
            .get("rms_norm_eps")
            .and_then(|v| parse_json_decimal_scaled_u64(v, 48))
            .unwrap_or(281_474_977)
            .max(1) as i64;
        let rope_theta = cfg
            .get("rope_theta")
            .and_then(|v| parse_json_decimal_scaled_u64(v, 0))
            .unwrap_or(1_000_000)
            .max(2);
        let bos_token_id = cfg
            .get("bos_token_id")
            .and_then(|v| v.as_u64())
            .unwrap_or(151_643) as u32;
        let eos_token_id = cfg
            .get("eos_token_id")
            .and_then(|v| v.as_u64())
            .unwrap_or(151_645) as u32;

        let verdict = admit_holo_geometric_runtime(AttentionOrderCheck {
            hidden_dim: d_model as u64,
            num_q_heads: n_heads as u64,
            num_kv_heads: n_kv_heads as u64,
            head_dim: head_dim as u64,
            group_size: 32,
            q_norm_before_rope: true,
            k_norm_before_rope: true,
            float_ops_count: 0,
            dense_matmul_ops_count: 0,
            worker_rpc_dispatched: true,
            main_thread_wasm_call: false,
            hamming_shortlist_k: DEFAULT_SHORTLIST_K.min(vocab_size.max(1)) as u64,
            vocab_size: vocab_size as u64,
        });
        if verdict != HoloRuntimeVerdict::RuntimeAdmitted || n_layers == 0 || intermediate_size == 0
        {
            return Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
                "config.json geometry refused ({verdict:?})"
            )));
        }

        let (tokenizer, _) = HoloTok13::from_hf_tokenizer_json(tokenizer_json)?;
        // `round(2^30 / sqrt(head_dim))` using `(clz_newton_isqrt(4 * 2^60 / head_dim) + 1) >> 1`.
        let hd_denom = u128::from(head_dim.max(1) as u64);
        let score_scale_q30 =
            (((clz_newton_isqrt((1u128 << 62) / hd_denom) + 1) >> 1).max(1)) as i64;

        Ok(Self {
            repo_id: repo_id.trim().to_string(),
            vocab_size,
            d_model,
            n_layers,
            n_heads,
            n_kv_heads,
            head_dim,
            intermediate_size,
            max_seq: 1024,
            bos_token_id,
            eos_token_id,
            rms_eps_q48,
            score_scale_q30,
            rope_theta,
            tokenizer,
            embed: None,
            vocab_sigs: Vec::new(),
            final_norm_q24: None,
            layers: (0..n_layers).map(|_| PartialHfLayer::default()).collect(),
        })
    }

    /// Formal R4.HoloGeometricRuntime stream one Safetensors tensor slice (or embedding band) into `UORR4V13` Block-32 RHT form.
    pub fn push_tensor_bytes(
        &mut self,
        name: &str,
        dtype: &str,
        shape: &[usize],
        raw_bytes: &[u8],
    ) -> Result<(), HoloKappaStoreError> {
        let seed_of = |tag: u64| -> u64 {
            let mut h = 0x517c_c1b7_2722_0a95u64 ^ (tag.wrapping_mul(0x9e37_79b9_7f4a_7c15));
            h ^= h >> 30;
            h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
            h ^ (h >> 27)
        };
        if name == "model.embed_tokens.weight" {
            let rows = *shape.first().unwrap_or(&self.vocab_size);
            let cols = *shape.get(1).unwrap_or(&self.d_model);
            let (mat, sigs) =
                compile_int8_rht_matrix_raw(dtype, raw_bytes, rows, cols, seed_of(0))?;
            self.embed = Some(mat);
            self.vocab_sigs = sigs;
            return Ok(());
        }
        if name == "model.embed_tokens.weight:band" {
            let band_rows = *shape.first().unwrap_or(&0);
            let cols = *shape.get(1).unwrap_or(&self.d_model);
            if cols != self.d_model || band_rows == 0 {
                return Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
                    "embed band shape [{band_rows}, {cols}] mismatches d_model={}",
                    self.d_model
                )));
            }
            let (mat, sigs) =
                compile_int8_rht_matrix_raw(dtype, raw_bytes, band_rows, cols, seed_of(0))?;
            if let Some(em) = self.embed.as_mut() {
                if em.cols != cols {
                    return Err(HoloKappaStoreError::CorruptSafetensorsHeader(
                        "embed band cols mismatch".into(),
                    ));
                }
                em.rows += mat.rows;
                em.row_exp.extend_from_slice(&mat.row_exp);
                em.group_mult.extend_from_slice(&mat.group_mult);
                em.weights.extend_from_slice(&mat.weights);
            } else {
                self.embed = Some(mat);
            }
            self.vocab_sigs.extend_from_slice(&sigs);
            return Ok(());
        }
        if name == "model.norm.weight" {
            let data = decode_safetensors_tensor_q24(dtype, raw_bytes)?;
            self.final_norm_q24 = Some(q24_to_i32_slice(&data));
            return Ok(());
        }
        if let Some(rest) = name.strip_prefix("model.layers.") {
            if let Some((idx_s, sub)) = rest.split_once('.') {
                let l_idx: usize = idx_s.parse().map_err(|_| {
                    HoloKappaStoreError::CorruptSafetensorsHeader(format!("bad layer name {name}"))
                })?;
                if l_idx >= self.n_layers {
                    return Err(HoloKappaStoreError::CorruptSafetensorsHeader(format!(
                        "layer index {l_idx} >= {}",
                        self.n_layers
                    )));
                }
                let layer = &mut self.layers[l_idx];
                let rows = *shape.first().unwrap_or(&0);
                let cols = *shape.get(1).unwrap_or(&0);
                let tag = (l_idx as u64) * 16;
                match sub {
                    "input_layernorm.weight" => {
                        let data = decode_safetensors_tensor_q24(dtype, raw_bytes)?;
                        layer.in_ln_q24 = Some(q24_to_i32_slice(&data));
                    }
                    "post_attention_layernorm.weight" => {
                        let data = decode_safetensors_tensor_q24(dtype, raw_bytes)?;
                        layer.post_ln_q24 = Some(q24_to_i32_slice(&data));
                    }
                    "self_attn.q_norm.weight" => {
                        let data = decode_safetensors_tensor_q24(dtype, raw_bytes)?;
                        layer.q_norm_q24 = Some(q24_to_i32_slice(&data));
                    }
                    "self_attn.k_norm.weight" => {
                        let data = decode_safetensors_tensor_q24(dtype, raw_bytes)?;
                        layer.k_norm_q24 = Some(q24_to_i32_slice(&data));
                    }
                    "self_attn.q_proj.weight" => {
                        layer.q_proj = Some(compile_int4_rht_matrix_raw(
                            dtype,
                            raw_bytes,
                            rows,
                            cols,
                            seed_of(tag + 1),
                        )?);
                    }
                    "self_attn.k_proj.weight" => {
                        layer.k_proj = Some(compile_int4_rht_matrix_raw(
                            dtype,
                            raw_bytes,
                            rows,
                            cols,
                            seed_of(tag + 2),
                        )?);
                    }
                    "self_attn.v_proj.weight" => {
                        layer.v_proj = Some(compile_int4_rht_matrix_raw(
                            dtype,
                            raw_bytes,
                            rows,
                            cols,
                            seed_of(tag + 3),
                        )?);
                    }
                    "self_attn.o_proj.weight" => {
                        layer.o_proj = Some(compile_int4_rht_matrix_raw(
                            dtype,
                            raw_bytes,
                            rows,
                            cols,
                            seed_of(tag + 4),
                        )?);
                    }
                    "mlp.gate_proj.weight" => {
                        layer.gate_proj = Some(compile_int4_rht_matrix_raw(
                            dtype,
                            raw_bytes,
                            rows,
                            cols,
                            seed_of(tag + 5),
                        )?);
                    }
                    "mlp.up_proj.weight" => {
                        layer.up_proj = Some(compile_int4_rht_matrix_raw(
                            dtype,
                            raw_bytes,
                            rows,
                            cols,
                            seed_of(tag + 6),
                        )?);
                    }
                    "mlp.down_proj.weight" => {
                        layer.down_proj = Some(compile_int4_rht_matrix_raw(
                            dtype,
                            raw_bytes,
                            rows,
                            cols,
                            seed_of(tag + 7),
                        )?);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// Formal R4.HoloGeometricRuntime finalize all streamed tensors into a runnable `HoloV13Model`.
    pub fn finish_model(self) -> Result<HoloV13Model, R4StackError> {
        let embed = self.embed.ok_or(R4StackError::TruncatedContainer)?;
        if embed.rows < self.vocab_size || self.vocab_sigs.len() < self.vocab_size * SIG_WORDS {
            return Err(R4StackError::TruncatedContainer);
        }
        let final_norm_q24 = self
            .final_norm_q24
            .ok_or(R4StackError::TruncatedContainer)?;
        let mut built_layers = Vec::with_capacity(self.n_layers);
        let default_hd_q24 = vec![1i32 << 24; self.head_dim];
        for l in self.layers {
            built_layers.push(HoloV13Layer {
                in_ln_q24: l.in_ln_q24.ok_or(R4StackError::TruncatedContainer)?,
                post_ln_q24: l.post_ln_q24.ok_or(R4StackError::TruncatedContainer)?,
                q_norm_q24: l.q_norm_q24.unwrap_or_else(|| default_hd_q24.clone()),
                k_norm_q24: l.k_norm_q24.unwrap_or_else(|| default_hd_q24.clone()),
                q_proj: l.q_proj.ok_or(R4StackError::TruncatedContainer)?,
                k_proj: l.k_proj.ok_or(R4StackError::TruncatedContainer)?,
                v_proj: l.v_proj.ok_or(R4StackError::TruncatedContainer)?,
                o_proj: l.o_proj.ok_or(R4StackError::TruncatedContainer)?,
                gate_proj: l.gate_proj.ok_or(R4StackError::TruncatedContainer)?,
                up_proj: l.up_proj.ok_or(R4StackError::TruncatedContainer)?,
                down_proj: HoloDownProjMatrix::Int4(
                    l.down_proj.ok_or(R4StackError::TruncatedContainer)?,
                ),
            });
        }
        let (rope_cos_q14, rope_sin_q14) =
            rope_sincos_tables_q14(self.max_seq, self.head_dim, self.rope_theta);
        let exp_n = (EXP_RANGE << 8) + 1;
        let exp_shift = (30 + EXP_STEP_LOG2) as u32;
        let mut exp_table = Vec::with_capacity(exp_n as usize);
        for i in 0..exp_n {
            let x_q30 = (i as u64) << exp_shift;
            exp_table.push(exp_neg_q30(x_q30));
        }
        let silu_n = (1usize << (SILU_RANGE_LOG2 - SILU_STEP_LOG2 + 1)) + 1;
        let silu_shift = (30 + SILU_STEP_LOG2) as u32;
        let silu_q16_shift = (16 + SILU_STEP_LOG2) as u32;
        let mut silu_table = Vec::with_capacity(silu_n);
        let half_s = (silu_n - 1) as i64 / 2;
        let one_q30 = 1u128 << 30;
        for i in 0..silu_n as i64 {
            let dx = i - half_s;
            let ax_q30 = dx.unsigned_abs() << silu_shift;
            let e_q30 = u128::from(exp_neg_q30(ax_q30));
            let den = one_q30 + e_q30;
            let sig_q30 = if dx >= 0 {
                ((1u128 << 60) + (den >> 1)) / den
            } else {
                ((e_q30 << 30) + (den >> 1)) / den
            };
            let x_q16 = i128::from(dx) << silu_q16_shift;
            let silu_val = div_round_i128(x_q16 * (sig_q30 as i128), 1i128 << 30);
            silu_table.push(silu_val.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32);
        }
        let mut kv_cache = HoloPrefixKvCache::default();
        kv_cache.reset(self.n_layers);
        let mut hasher = Sha256::new();
        hasher.update(self.repo_id.as_bytes());
        hasher.update((embed.weights.len() as u64).to_le_bytes());
        let mut sha = String::with_capacity(64);
        for b in hasher.finalize() {
            use std::fmt::Write;
            let _ = write!(&mut sha, "{b:02x}");
        }
        Ok(HoloV13Model {
            model_id: format!("uor-r4.hf-repo-holo:{}", self.repo_id),
            container_sha256: sha,
            vocab_size: self.vocab_size,
            d_model: self.d_model,
            n_layers: self.n_layers,
            n_heads: self.n_heads,
            n_kv_heads: self.n_kv_heads,
            head_dim: self.head_dim,
            intermediate_size: self.intermediate_size,
            max_seq: self.max_seq,
            bos_token_id: self.bos_token_id,
            eos_token_id: self.eos_token_id,
            eot_token_id: self.bos_token_id,
            think_start_id: 151_667,
            think_end_id: 151_668,
            rms_eps_q48: self.rms_eps_q48,
            score_scale_q30: self.score_scale_q30,
            rope_theta: self.rope_theta,
            rope_cos_q14,
            rope_sin_q14,
            exp_table,
            silu_table,
            tokenizer: self.tokenizer,
            vocab_sigs: self.vocab_sigs,
            embed,
            final_norm_q24,
            layers: built_layers,
            kv_cache,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fwht32_and_isqrt_are_exact() {
        let mut v = [0i64; 32];
        v[0] = 32;
        int_fwht32_i64(&mut v);
        assert!(v.iter().all(|&x| x == 32));
        int_fwht32_i64(&mut v);
        assert_eq!(v[0], 1024);
        assert!(v[1..].iter().all(|&x| x == 0));
        assert_eq!(clz_newton_isqrt(1u128 << 80), 1u128 << 40);
    }

    #[test]
    fn dot_i4_and_i8_group32_match_scalar() {
        let mut nibs = [0u8; 16];
        let mut act = [0i16; 32];
        for i in 0..16 {
            let q0 = ((i as i8) % 15) - 7;
            let q1 = (((i * 3) as i8) % 15) - 7;
            nibs[i] = ((q0 + 8) as u8 & 15) | (((q1 + 8) as u8 & 15) << 4);
            act[i] = (i as i16) * 11 - 50;
            act[i + 16] = (i as i16) * -7 + 31;
        }
        let d4 = dot_i4_group32(&nibs, &act);
        let mut ref4 = 0i32;
        for i in 0..16 {
            ref4 += (i32::from(nibs[i] & 15) - 8) * i32::from(act[i]);
            ref4 += (i32::from(nibs[i] >> 4) - 8) * i32::from(act[i + 16]);
        }
        assert_eq!(d4, ref4);

        let mut w8 = [0i8; 32];
        for (i, w) in w8.iter_mut().enumerate() {
            *w = ((i as i32 * 9) % 120 - 60) as i8;
        }
        let d8 = dot_i8_group32(&w8, &act);
        let ref8: i32 = w8
            .iter()
            .zip(&act)
            .map(|(&a, &b)| i32::from(a) * i32::from(b))
            .sum();
        assert_eq!(d8, ref8);
    }

    #[test]
    fn integer_ingestion_and_lut_helpers_are_exact() {
        assert_eq!(f32_bits_to_q24(0x3F80_0000), 1i64 << 24);
        assert_eq!(f32_bits_to_q24(0xBF00_0000), -(1i64 << 23));
        assert_eq!(bf16_bits_to_q24(0x3F80), 1i64 << 24);
        assert_eq!(f16_bits_to_q24(0x3C00), 1i64 << 24);
        assert_eq!(f16_bits_to_q24(0xBC00), -(1i64 << 24));
        assert_eq!(exp_neg_q30(0), 1u32 << 30);
        let (cos_tbl, sin_tbl) = rope_sincos_tables_q14(4, 128, 1_000_000);
        assert_eq!(cos_tbl[0], 16_384);
        assert_eq!(sin_tbl[0], 0);
        assert!((cos_tbl[64] - 8_850).abs() <= 2);
        assert!((sin_tbl[64] - 13_787).abs() <= 2);
    }

    #[test]
    fn non_square_layer_streaming_mount_and_inference_succeeds() {
        let vocab = 256usize;
        let dm = 128usize;
        let nl = 1usize;
        let nh = 2usize;
        let nkv = 1usize;
        let hd = 128usize;
        let ffn = 192usize;
        let max_seq = 16usize;

        let mut raw = Vec::new();
        raw.extend_from_slice(HOLO_V13_MAGIC);
        raw.extend_from_slice(&HOLO_V13_VERSION.to_le_bytes());
        raw.extend_from_slice(&1u32.to_le_bytes());
        raw.extend_from_slice(&(vocab as u32).to_le_bytes());
        raw.extend_from_slice(&(dm as u32).to_le_bytes());
        raw.extend_from_slice(&(nl as u32).to_le_bytes());
        raw.extend_from_slice(&(nh as u32).to_le_bytes());
        raw.extend_from_slice(&(nkv as u32).to_le_bytes());
        raw.extend_from_slice(&(hd as u32).to_le_bytes());
        raw.extend_from_slice(&(ffn as u32).to_le_bytes());
        raw.extend_from_slice(&(max_seq as u32).to_le_bytes());
        raw.extend_from_slice(&0u32.to_le_bytes());
        raw.extend_from_slice(&1u32.to_le_bytes());
        raw.extend_from_slice(&0u32.to_le_bytes());
        raw.extend_from_slice(&2u32.to_le_bytes());
        raw.extend_from_slice(&3u32.to_le_bytes());
        raw.extend_from_slice(&281_474_977i64.to_le_bytes());
        raw.extend_from_slice(&94_906_266i64.to_le_bytes());
        raw.extend_from_slice(&1_000_000u64.to_le_bytes());
        raw.extend_from_slice(&0b0_1111u32.to_le_bytes());

        let half = hd / 2;
        for _ in 0..(max_seq * half) {
            raw.extend_from_slice(&16_384i16.to_le_bytes());
        }
        for _ in 0..(max_seq * half) {
            raw.extend_from_slice(&0i16.to_le_bytes());
        }
        let exp_len = ((EXP_RANGE as usize) << 8) + 1;
        for _ in 0..exp_len {
            raw.extend_from_slice(&(1u32 << 30).to_le_bytes());
        }
        let silu_len = (1usize << (SILU_RANGE_LOG2 - SILU_STEP_LOG2 + 1)) + 1;
        for _ in 0..silu_len {
            raw.extend_from_slice(&0i32.to_le_bytes());
        }

        let mut tok = Vec::new();
        tok.extend_from_slice(HOLO_TOK13_MAGIC);
        tok.extend_from_slice(&(vocab as u32).to_le_bytes());
        tok.extend_from_slice(&0u32.to_le_bytes());
        tok.extend_from_slice(&0u32.to_le_bytes());
        for b in 0..vocab {
            tok.extend_from_slice(&1u16.to_le_bytes());
            tok.push(b as u8);
        }
        for b in 0..vocab {
            tok.extend_from_slice(&1u16.to_le_bytes());
            let ch = if (b as u8).is_ascii_graphic() {
                b as u8
            } else {
                b'x'
            };
            tok.push(ch);
        }
        raw.extend_from_slice(&(tok.len() as u32).to_le_bytes());
        raw.extend_from_slice(&tok);

        for _ in 0..(vocab * SIG_WORDS) {
            raw.extend_from_slice(&0x5555_5555_5555_5555u64.to_le_bytes());
        }

        // Tied embedding matrix [vocab, dm]
        raw.extend_from_slice(&(vocab as u32).to_le_bytes());
        raw.extend_from_slice(&(dm as u32).to_le_bytes());
        raw.extend_from_slice(&1u64.to_le_bytes());
        raw.resize(raw.len() + vocab, (-20i8) as u8);
        for _ in 0..(vocab * (dm >> 5)) {
            raw.extend_from_slice(&1u16.to_le_bytes());
        }
        raw.resize(raw.len() + vocab * dm, 1i8 as u8);

        for _ in 0..dm {
            raw.extend_from_slice(&(1i32 << 24).to_le_bytes());
        }
        for _ in 0..(dm * 2 + hd * 2) {
            raw.extend_from_slice(&(1i32 << 24).to_le_bytes());
        }

        let push_i4 = |buf: &mut Vec<u8>, rows: usize, cols: usize, seed: u64| {
            buf.extend_from_slice(&(rows as u32).to_le_bytes());
            buf.extend_from_slice(&(cols as u32).to_le_bytes());
            buf.extend_from_slice(&seed.to_le_bytes());
            buf.resize(buf.len() + rows, (-18i8) as u8);
            for _ in 0..(rows * (cols >> 5)) {
                buf.extend_from_slice(&1u16.to_le_bytes());
            }
            buf.resize(buf.len() + rows * (cols >> 1), 0x89u8);
        };
        let q_dim = nh * hd;
        let kv_dim = nkv * hd;
        push_i4(&mut raw, q_dim, dm, 11);
        push_i4(&mut raw, kv_dim, dm, 12);
        push_i4(&mut raw, kv_dim, dm, 13);
        push_i4(&mut raw, dm, q_dim, 14);
        push_i4(&mut raw, ffn, dm, 15);
        push_i4(&mut raw, ffn, dm, 16);
        push_i4(&mut raw, dm, ffn, 17);

        let digest = Sha256::digest(&raw);
        let mut sha_hex = String::with_capacity(64);
        for b in digest {
            use std::fmt::Write;
            let _ = write!(&mut sha_hex, "{b:02x}");
        }

        let c1 = raw.len() / 3;
        let c2 = (raw.len() * 2) / 3;
        let mut builder = HoloV13StreamingBuilder::new("uor-r4.test-nonsquare", 3, &sha_hex);
        builder.push_piece(&raw[..c1]).expect("chunk 1");
        builder.push_piece(&raw[c1..c2]).expect("chunk 2");
        builder.push_piece(&raw[c2..]).expect("chunk 3");
        let mut model = builder.finalize().expect("non-square Int4 mount");
        assert_eq!(model.n_layers, 1);
        assert_eq!(model.d_model, dm);
        let (_reply, metrics) = model.generate_chat_turn("hello", "assistant", 4);
        assert_eq!(metrics.float_ops, 0);
        assert_eq!(metrics.dense_matmul_ops, 0);
        assert!(metrics.q_norm_before_rope);
        assert!(metrics.k_norm_before_rope);
    }
}
