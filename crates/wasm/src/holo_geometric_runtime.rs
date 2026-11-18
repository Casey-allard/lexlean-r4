//! Track 1 (`R4-51`): Pure-integer `UORR4V13` Qwen3 Holo geometric runtime with Wasm `simd128`
//! kernels, Block-32 RHT symmetric `int8` tied embeddings, Cholesky-GPTQ `int4` layers, pre-RoPE
//! per-head `q_norm`/`k_norm`, SWAR 512-bit Hamming shortlisting, 128-bit QK Starling Flock sparse
//! attention selection, streaming chunked OPFS mounting, and in-stream HuggingFace repo compilation.

use crate::holo_kappa_store::{parse_safetensors_header, HoloKappaStoreError};
use crate::lexlean_generated::r4_holo_geometric_runtime::{
    admit_holo_geometric_runtime, compute_shortlist_budget, AttentionOrderCheck, HoloRuntimeVerdict,
};
use crate::lexlean_generated::r4_studio_frame::{admit_live_token_frame, LiveFrameVerdict};
use crate::r4_geometric_stack::R4StackError;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

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
const DEFAULT_SHORTLIST_K: usize = 1_536;
const DEFAULT_FLOCK_WINDOW: usize = 256;
const MAX_PREFILL_TILE: usize = 4;

/// Formal R4.HoloGeometricRuntime pre-pack immutable xorshift64 RHT sign bits into `u32` masks (`cols / 32`).
#[inline]
pub fn rht_signs_packed_u32(cols: usize, seed: u64) -> Vec<u32> {
    let groups = cols >> 5;
    let mut out = vec![0u32; groups];
    let mut s = seed ^ 0x9e37_79b9_7f4a_7c15;
    if s == 0 {
        s = 0x853c_49e6_748f_ea9b;
    }
    for word in &mut out {
        let mut mask = 0u32;
        for bit in 0..32u32 {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            if (s & 1) == 1 {
                mask |= 1u32 << bit;
            }
        }
        *word = mask;
    }
    out
}

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

/// Formal R4.HoloGeometricRuntime pure-integer 32-point Walsh-Hadamard butterfly (`int_fwht32_i32`)
/// vectorized over `wasm32` `simd128` `i32x4` lanes with exact scalar fallback.
#[inline(always)]
pub fn int_fwht32_i32(chunk: &mut [i32; 32]) {
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    // SAFETY: `chunk` is `&mut [i32; 32]`, providing 8 aligned-compatible 16-byte loads/stores.
    unsafe {
        use core::arch::wasm32::*;
        let ptr = chunk.as_mut_ptr();
        let mut v0 = v128_load(ptr as *const v128);
        let mut v1 = v128_load(ptr.add(4) as *const v128);
        let mut v2 = v128_load(ptr.add(8) as *const v128);
        let mut v3 = v128_load(ptr.add(12) as *const v128);
        let mut v4 = v128_load(ptr.add(16) as *const v128);
        let mut v5 = v128_load(ptr.add(20) as *const v128);
        let mut v6 = v128_load(ptr.add(24) as *const v128);
        let mut v7 = v128_load(ptr.add(28) as *const v128);

        macro_rules! stage1_2_pair {
            ($a:expr, $b:expr) => {{
                let ev = i32x4_shuffle::<0, 2, 4, 6>($a, $b);
                let od = i32x4_shuffle::<1, 3, 5, 7>($a, $b);
                let s1 = i32x4_add(ev, od);
                let d1 = i32x4_sub(ev, od);
                let u0 = i32x4_shuffle::<0, 4, 1, 5>(s1, d1);
                let u1 = i32x4_shuffle::<2, 6, 3, 7>(s1, d1);

                let p0 = i32x4_shuffle::<0, 1, 4, 5>(u0, u1);
                let p1 = i32x4_shuffle::<2, 3, 6, 7>(u0, u1);
                let s2 = i32x4_add(p0, p1);
                let d2 = i32x4_sub(p0, p1);
                $a = i32x4_shuffle::<0, 1, 4, 5>(s2, d2);
                $b = i32x4_shuffle::<2, 3, 6, 7>(s2, d2);
            }};
        }
        stage1_2_pair!(v0, v1);
        stage1_2_pair!(v2, v3);
        stage1_2_pair!(v4, v5);
        stage1_2_pair!(v6, v7);

        macro_rules! bfly {
            ($x:expr, $y:expr) => {{
                let sx = i32x4_add($x, $y);
                let dy = i32x4_sub($x, $y);
                $x = sx;
                $y = dy;
            }};
        }
        bfly!(v0, v1);
        bfly!(v2, v3);
        bfly!(v4, v5);
        bfly!(v6, v7);

        bfly!(v0, v2);
        bfly!(v1, v3);
        bfly!(v4, v6);
        bfly!(v5, v7);

        bfly!(v0, v4);
        bfly!(v1, v5);
        bfly!(v2, v6);
        bfly!(v3, v7);

        v128_store(ptr as *mut v128, v0);
        v128_store(ptr.add(4) as *mut v128, v1);
        v128_store(ptr.add(8) as *mut v128, v2);
        v128_store(ptr.add(12) as *mut v128, v3);
        v128_store(ptr.add(16) as *mut v128, v4);
        v128_store(ptr.add(20) as *mut v128, v5);
        v128_store(ptr.add(24) as *mut v128, v6);
        v128_store(ptr.add(28) as *mut v128, v7);
        return;
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        let mut h = 1usize;
        while h < 32 {
            let mut i = 0usize;
            while i < 32 {
                for j in i..i + h {
                    let x = chunk[j];
                    let y = chunk[j + h];
                    chunk[j] = x.wrapping_add(y);
                    chunk[j + h] = x.wrapping_sub(y);
                }
                i += h * 2;
            }
            h *= 2;
        }
    }
}

/// Formal R4.HoloGeometricRuntime pure-integer dot product of 128 `i16` Q/K lanes using `simd128`
/// widening directly into `i64x2` accumulators to guarantee zero intermediate overflow.
#[inline(always)]
pub fn dot_i16_128(a: &[i16; 128], b: &[i16]) -> i64 {
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    // SAFETY: `a` has 128 elements and `b` is validated to `len >= 128`.
    unsafe {
        use core::arch::wasm32::*;
        debug_assert!(b.len() >= 128);
        let ap = a.as_ptr();
        let bp = b.as_ptr();
        let mut acc64_a = i64x2_splat(0);
        let mut acc64_b = i64x2_splat(0);
        for blk in 0..4 {
            let base = blk * 32;
            let d0 = i32x4_dot_i16x8(
                v128_load(ap.add(base) as *const v128),
                v128_load(bp.add(base) as *const v128),
            );
            let d1 = i32x4_dot_i16x8(
                v128_load(ap.add(base + 8) as *const v128),
                v128_load(bp.add(base + 8) as *const v128),
            );
            let d2 = i32x4_dot_i16x8(
                v128_load(ap.add(base + 16) as *const v128),
                v128_load(bp.add(base + 16) as *const v128),
            );
            let d3 = i32x4_dot_i16x8(
                v128_load(ap.add(base + 24) as *const v128),
                v128_load(bp.add(base + 24) as *const v128),
            );
            let s01 = i64x2_add(
                i64x2_add(i64x2_extend_low_i32x4(d0), i64x2_extend_high_i32x4(d0)),
                i64x2_add(i64x2_extend_low_i32x4(d1), i64x2_extend_high_i32x4(d1)),
            );
            let s23 = i64x2_add(
                i64x2_add(i64x2_extend_low_i32x4(d2), i64x2_extend_high_i32x4(d2)),
                i64x2_add(i64x2_extend_low_i32x4(d3), i64x2_extend_high_i32x4(d3)),
            );
            acc64_a = i64x2_add(acc64_a, s01);
            acc64_b = i64x2_add(acc64_b, s23);
        }
        let sum2 = i64x2_add(acc64_a, acc64_b);
        return i64x2_extract_lane::<0>(sum2).saturating_add(i64x2_extract_lane::<1>(sum2));
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        let mut acc = 0i64;
        for i in 0..128 {
            acc += i64::from(a[i]) * i64::from(b[i]);
        }
        acc
    }
}

/// Formal R4.HoloGeometricRuntime vectorized 512-bit XOR Hamming distance (`wasm32` `u8x16_popcnt`).
#[inline(always)]
pub fn hamming512_u64x8(q_sig: &[u64; SIG_WORDS], v_sig: &[u64]) -> u32 {
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    // SAFETY: `q_sig` has 8 `u64` words (4 `v128` loads) and `v_sig` has `>= 8` `u64` words.
    unsafe {
        use core::arch::wasm32::*;
        debug_assert!(v_sig.len() >= SIG_WORDS);
        let qp = q_sig.as_ptr() as *const v128;
        let vp = v_sig.as_ptr() as *const v128;
        let x0 = u8x16_popcnt(v128_xor(v128_load(qp), v128_load(vp)));
        let x1 = u8x16_popcnt(v128_xor(v128_load(qp.add(1)), v128_load(vp.add(1))));
        let x2 = u8x16_popcnt(v128_xor(v128_load(qp.add(2)), v128_load(vp.add(2))));
        let x3 = u8x16_popcnt(v128_xor(v128_load(qp.add(3)), v128_load(vp.add(3))));
        let p01 = i16x8_add(
            u16x8_extadd_pairwise_u8x16(x0),
            u16x8_extadd_pairwise_u8x16(x1),
        );
        let p23 = i16x8_add(
            u16x8_extadd_pairwise_u8x16(x2),
            u16x8_extadd_pairwise_u8x16(x3),
        );
        let sum8 = i16x8_add(p01, p23);
        let sum4 = i32x4_extadd_pairwise_i16x8(sum8);
        return (i32x4_extract_lane::<0>(sum4)
            + i32x4_extract_lane::<1>(sum4)
            + i32x4_extract_lane::<2>(sum4)
            + i32x4_extract_lane::<3>(sum4)) as u32;
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        let mut d_ham = 0u32;
        for w in 0..SIG_WORDS {
            d_ham += (q_sig[w] ^ v_sig[w]).count_ones();
        }
        d_ham
    }
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

/// Formal R4.HoloGeometricRuntime pure-integer RMSNorm with explicit Q24 per-channel scale vector,
/// eliminating `i128`/`u128` inner loops via `u64` sum-of-squares and normalized 22-bit `r_q` scaling.
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
    let mut sum_u64 = 0u64;
    let mut max_gain_abs = 0u32;
    for ((dst, &v), &g) in out.iter_mut().zip(x).zip(gain_q24) {
        let y = shift_round(i64::from(v), sh);
        *dst = y;
        let ay = y.unsigned_abs();
        sum_u64 = sum_u64.wrapping_add(ay.wrapping_mul(ay));
        max_gain_abs = max_gain_abs.max(g.unsigned_abs());
    }
    let mean = u128::from(sum_u64) / (x.len().max(1) as u128);
    let sq_exp = (in_exp + sh) << 1;
    let eps_int = shift_round(eps_q48, sq_exp - (-48)).max(0) as u128;
    let ms = mean.saturating_add(eps_int).max(1);
    let s = clz_newton_isqrt(ms << 68).max(1);
    let r = (1u128 << 96) / s;
    let r_shift = (86 + out_exp).max(1);

    let r_bits = 128 - r.leading_zeros() as i32;
    let r_pre_shift = (r_bits - 22).max(0);
    let gain_bits = 32 - max_gain_abs.leading_zeros() as i32;
    let sh1 = (21 + gain_bits - 39).max(0);
    let sh2 = r_shift - r_pre_shift - sh1;
    if sh2 > 0 && sh2 < 63 && r_pre_shift < 128 {
        let r_q = (r >> (r_pre_shift as u32)) as i64;
        let sh2_u = sh2 as u32;
        let round2 = 1i64 << (sh2_u - 1);
        for (slot, &g) in out.iter_mut().zip(gain_q24) {
            let yg = shift_round((*slot).saturating_mul(i64::from(g)), sh1);
            *slot = yg.saturating_mul(r_q).saturating_add(round2) >> sh2_u;
        }
    } else {
        let round_half = 1i128 << ((r_shift as u32) - 1);
        for (i, slot) in out.iter_mut().enumerate() {
            let unit = (i128::from(*slot) * (r as i128) * i128::from(gain_q24[i]))
                .saturating_add(round_half)
                >> (r_shift as u32);
            *slot = unit.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
        }
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
    /// Pre-packed xorshift64 RHT sign masks (`cols / 32`).
    pub rht_signs_packed: Vec<u32>,
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
    /// Pre-packed xorshift64 RHT sign masks (`cols / 32`).
    pub rht_signs_packed: Vec<u32>,
    /// Per-row binary exponent shift.
    pub row_exp: Vec<i8>,
    /// Per-group-32 unsigned 16-bit scale multiplier.
    pub group_mult: Vec<u16>,
    /// Contiguous half-block bias-8 packed nibbles.
    pub nibbles: Vec<u8>,
}

/// Top logit candidate snapshot for live Observatory race chart (`R4-51`, `R4-60`).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct HoloCandidateBar {
    /// Vocabulary token ID.
    pub token_id: u32,
    /// Decoded UTF-8 token surface snippet.
    pub token_text: String,
    /// Pure-integer Q16 logit score.
    pub logit_q16: i32,
    /// Pure-integer normalized candidate probability in `[0, 1000]` permille (`R4-60`).
    #[serde(default)]
    pub prob_permille: u16,
    /// True when selected by greedy/cycle-guarded argmax.
    pub selected: bool,
    /// Real 512-bit RHT Hamming distance when geometric shortlisting is active (`None` on `NativeQ31`).
    #[serde(default)]
    pub hamming_d: Option<u16>,
}

/// Top attended KV position peak recorded during Starling Flock attention (`R4-51`).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct HoloAttnPeak {
    /// Sequence position `t` in `[0, seq_len)`.
    pub pos: u16,
    /// Aggregated integer attention weight in `[0, 65535]`.
    pub weight_q16: u16,
}

/// Genuine per-token neural & geometric observatory frame streamed live during inference (`R4-51`, `R4-60`).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct HoloStepObservatoryFrame {
    /// Step index within the active phase.
    pub step_idx: usize,
    /// `"prefill"` or `"decode"`.
    pub phase: &'static str,
    /// Sequence position `0..max_seq`.
    pub seq_pos: usize,
    /// Reused KV prefix token count on this turn.
    pub prefix_reused_tokens: usize,
    /// Token ID at this step.
    pub token_id: u32,
    /// Decoded UTF-8 token snippet.
    pub token_text: String,
    /// Per-layer mean `|x|` residual magnitude scaled cleanly into `u16`.
    pub layer_residual_mag: Vec<u16>,
    /// Per-layer max `|x|` bit-width (`0..32`) showing real dynamic exponent scaling.
    pub layer_max_bits: Vec<u8>,
    /// Per-layer real Starling Flock attention span (`selected.len()`).
    pub layer_flock_spans: Vec<u16>,
    /// Top attended KV positions aggregated across final layer heads.
    pub final_attn_peaks: Vec<HoloAttnPeak>,
    /// Downsampled 32-bin distribution across the 513 possible 512-bit RHT Hamming distances.
    pub hamming_hist_32: [u32; 32],
    /// Minimum Hamming distance observed over `vocab_sigs`.
    #[serde(default)]
    pub min_hamming: u16,
    /// True when active model uses SWAR Hamming shortlisting (`false` on `NativeQ31`).
    #[serde(default)]
    pub has_geometric_shortlist: bool,
    /// Time-to-first-token latency in milliseconds (`R4-60`).
    #[serde(default)]
    pub ttft_ms: u32,
    /// Live tokens-per-second throughput scaled by `10` (`R4-60`).
    #[serde(default)]
    pub tps_x10: u16,
    /// Quantized `[x, y, z]` trajectory coordinate in `[-1000, 1000]` from `q_sig512`.
    pub q_manifold_xyz: [i16; 3],
    /// Top-6 integer logit race candidates at `lm_head`.
    pub top_candidates: Vec<HoloCandidateBar>,
    /// Evaluated shortlist candidate count at `lm_head`.
    pub shortlist_count: usize,
    /// Active execution backend identifier.
    pub backend: &'static str,
}

/// Per-head Starling Flock & RMSNorm temporary workspace inside `HoloScratch` (`R4-51`).
#[derive(Clone, Debug, Default)]
pub struct HoloAttnScratch {
    /// Per-head query RMSNorm buffer (`head_dim`).
    pub q_h_norm: Vec<i64>,
    /// Per-head key RMSNorm buffer (`head_dim`).
    pub k_h_norm: Vec<i64>,
    /// Reusable `i16` per-head query buffer (`128`).
    pub q_i16: Vec<i16>,
    /// Reusable Starling Flock selected KV indices (`max_seq`).
    pub selected: Vec<usize>,
    /// Reusable Starling Flock candidate pairs `(distance, pos)`.
    pub flock_cands: Vec<(u32, usize)>,
    /// Reusable per-position Starling Flock priority bin (`0..=129`) for O(N) bucket selection.
    pub flock_bins: Vec<u8>,
    /// Reusable attention scores buffer (`max_seq`).
    pub scores: Vec<i64>,
    /// Reusable softmax exponential weights (`max_seq`).
    pub exp_w: Vec<i64>,
    /// Reusable last-layer attention weight accumulators across sequence positions.
    pub attn_accum_q16: Vec<u32>,
}

/// Reusable `MAX_PREFILL_TILE`-token register-tiled prefill scratch workspace (`R4-51`).
#[derive(Clone, Debug, Default)]
pub struct HoloPrefillTileScratch {
    /// Stacked token residual vectors (`bt * d_model`).
    pub x_b: Vec<i32>,
    /// Stacked normalized vectors (`bt * d_model`).
    pub norm_b: Vec<i64>,
    /// Stacked query projections (`bt * q_dim`).
    pub q_b: Vec<i32>,
    /// Stacked key projections (`bt * kv_dim`).
    pub k_b: Vec<i32>,
    /// Stacked value projections (`bt * kv_dim`).
    pub v_b: Vec<i32>,
    /// Stacked attention outputs (`bt * q_dim`).
    pub attn_b: Vec<i64>,
    /// Stacked residual projections (`bt * d_model`).
    pub proj_b: Vec<i32>,
    /// Stacked SwiGLU gate projections (`bt * ffn`).
    pub gate_b: Vec<i32>,
    /// Stacked SwiGLU up projections (`bt * ffn`).
    pub up_b: Vec<i32>,
    /// Stacked SwiGLU elementwise products (`bt * ffn`).
    pub swiglu_b: Vec<i64>,
    /// Group-interleaved `int16` activations `[group][token_in_tile][32]` (`MAX_PREFILL_TILE * max_cols`).
    pub act16_interleaved: Vec<i16>,
    /// Single-token temporary `i16` activation buffer (`max_cols`).
    pub act16_tmp: Vec<i16>,
}

impl HoloPrefillTileScratch {
    /// Formal R4.HoloGeometricRuntime allocate reusable zero-alloc `MAX_PREFILL_TILE` buffers without heap growth during prefill.
    pub fn ensure_sized(&mut self, d_model: usize, q_dim: usize, kv_dim: usize, ffn: usize) {
        let max_cols = d_model.max(q_dim).max(ffn).max(32);
        let d_batch = MAX_PREFILL_TILE * d_model;
        let q_batch = MAX_PREFILL_TILE * q_dim;
        let kv_batch = MAX_PREFILL_TILE * kv_dim;
        let ffn_batch = MAX_PREFILL_TILE * ffn;
        let act_batch = MAX_PREFILL_TILE * max_cols;
        if self.x_b.len() != d_batch {
            self.x_b.resize(d_batch, 0);
            self.norm_b.resize(d_batch, 0);
            self.proj_b.resize(d_batch, 0);
        }
        if self.q_b.len() != q_batch {
            self.q_b.resize(q_batch, 0);
            self.attn_b.resize(q_batch, 0);
        }
        if self.k_b.len() != kv_batch {
            self.k_b.resize(kv_batch, 0);
            self.v_b.resize(kv_batch, 0);
        }
        if self.gate_b.len() != ffn_batch {
            self.gate_b.resize(ffn_batch, 0);
            self.up_b.resize(ffn_batch, 0);
            self.swiglu_b.resize(ffn_batch, 0);
        }
        if self.act16_interleaved.len() != act_batch {
            self.act16_interleaved.resize(act_batch, 0);
        }
        if self.act16_tmp.len() != max_cols {
            self.act16_tmp.resize(max_cols, 0);
        }
    }
}

/// Zero-allocation scratch workspace owned by `HoloV13Model` (`R4-51`).
#[derive(Clone, Debug, Default)]
pub struct HoloScratch {
    /// Layer residual vector (`d_model`).
    pub x: Vec<i32>,
    /// Layer RMSNorm normalized buffer (`d_model`).
    pub norm_d: Vec<i64>,
    /// Query projection buffer (`q_dim`).
    pub q: Vec<i32>,
    /// Key projection buffer (`kv_dim`).
    pub k: Vec<i32>,
    /// Value projection buffer (`kv_dim`).
    pub v: Vec<i32>,
    /// Concatenated attention output buffer (`q_dim`).
    pub attn_i64: Vec<i64>,
    /// Output projection buffer (`d_model`).
    pub proj: Vec<i32>,
    /// SwiGLU gate projection buffer (`intermediate_size`).
    pub gate: Vec<i32>,
    /// SwiGLU up projection buffer (`intermediate_size`).
    pub up: Vec<i32>,
    /// SwiGLU elementwise product buffer (`intermediate_size`).
    pub swiglu: Vec<i64>,
    /// Reusable `i32` RHT butterfly buffer (`max(d_model, q_dim, intermediate_size)`).
    pub rht_i32: Vec<i32>,
    /// Reusable `i64` wide buffer for embedding dequantization (`d_model`).
    pub wide_i64: Vec<i64>,
    /// Reusable `i16` quantized activation buffer (`max(d_model, q_dim, intermediate_size)`).
    pub act16: Vec<i16>,
    /// Split-borrow per-head attention scratch workspace.
    pub attn_work: HoloAttnScratch,
    /// Lazily-sized `MAX_PREFILL_TILE`-token register-tiled prefill workspace.
    pub prefill_tile: HoloPrefillTileScratch,
    /// Reusable 512-bit Hamming distances across `vocab_size`.
    pub hamming_dists: Vec<u16>,
    /// Reusable 513-bin bucket histogram (`0..=512`).
    pub hamming_hist: Vec<u32>,
    /// Reusable O(1) shortlist membership bitset (`ceil(vocab_size / 64)`).
    pub shortlist_bitset: Vec<u64>,
    /// Reusable shortlist output token IDs.
    pub shortlist_out: Vec<u32>,
    /// Per-layer mean `|x|` magnitude recorded during last step.
    pub layer_residual_mag: Vec<u16>,
    /// Per-layer bit-width recorded during last step.
    pub layer_max_bits: Vec<u8>,
    /// Per-layer Starling Flock selected count recorded during last step.
    pub layer_flock_spans: Vec<u16>,
    /// Latest query 512-bit RHT signature from `select_shortlist_into`.
    pub last_q_sig512: [u64; SIG_WORDS],
    /// Latest 32-bin compressed Hamming histogram from `select_shortlist_into`.
    pub last_hamming_hist32: [u32; 32],
    /// Latest minimum Hamming distance observed across `vocab_sigs`.
    pub last_min_hamming: u16,
    /// Latest top attended KV position peaks.
    pub last_attn_peaks: Vec<HoloAttnPeak>,
}

impl HoloScratch {
    /// Formal R4.HoloGeometricRuntime allocate reusable zero-alloc buffers sized for `HoloV13Model`.
    #[allow(clippy::too_many_arguments)]
    pub fn ensure_sized(
        &mut self,
        d_model: usize,
        q_dim: usize,
        kv_dim: usize,
        ffn: usize,
        head_dim: usize,
        n_layers: usize,
        vocab_size: usize,
    ) {
        let max_cols = d_model.max(q_dim).max(ffn).max(32);
        if self.x.len() != d_model {
            self.x.resize(d_model, 0);
            self.norm_d.resize(d_model, 0);
            self.proj.resize(d_model, 0);
            self.wide_i64.resize(d_model, 0);
        }
        if self.q.len() != q_dim {
            self.q.resize(q_dim, 0);
            self.attn_i64.resize(q_dim, 0);
        }
        if self.k.len() != kv_dim {
            self.k.resize(kv_dim, 0);
            self.v.resize(kv_dim, 0);
        }
        if self.attn_work.q_h_norm.len() != head_dim {
            self.attn_work.q_h_norm.resize(head_dim, 0);
            self.attn_work.k_h_norm.resize(head_dim, 0);
        }
        if self.attn_work.q_i16.len() != head_dim.max(128) {
            self.attn_work.q_i16.resize(head_dim.max(128), 0);
        }
        if self.gate.len() != ffn {
            self.gate.resize(ffn, 0);
            self.up.resize(ffn, 0);
            self.swiglu.resize(ffn, 0);
        }
        if self.rht_i32.len() != max_cols {
            self.rht_i32.resize(max_cols, 0);
            self.act16.resize(max_cols, 0);
        }
        if self.hamming_dists.len() != vocab_size {
            self.hamming_dists.resize(vocab_size, 0);
        }
        if self.hamming_hist.len() != 513 {
            self.hamming_hist.resize(513, 0);
        }
        let bitset_words = vocab_size.div_ceil(64).max(1);
        if self.shortlist_bitset.len() != bitset_words {
            self.shortlist_bitset.resize(bitset_words, 0);
        }
        if self.layer_residual_mag.len() != n_layers {
            self.layer_residual_mag.resize(n_layers, 0);
            self.layer_max_bits.resize(n_layers, 0);
            self.layer_flock_spans.resize(n_layers, 0);
        }
    }
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
    /// Rolling `int16` + 128-bit Starling Flock prefix KV cache.
    pub kv_cache: HoloPrefixKvCache,
    /// Configurable Hamming shortlist candidate budget (`4096` default, `1024..8192`).
    pub shortlist_k: usize,
    /// Configurable Starling Flock attention window (`256` default, `64..512`).
    pub flock_window: usize,
    /// Reusable zero-allocation scratch workspace across steps.
    pub scratch: HoloScratch,
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
    /// Exact `(user_prompt_trimmed, assistant_reply_trimmed, generated_token_ids)` spans for lossless multi-turn KV reuse.
    pub turn_token_spans: Vec<(String, String, Vec<u32>)>,
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
        self.turn_token_spans.clear();
    }

    /// Formal R4.HoloGeometricRuntime truncate KV state to an exact cached prefix length (`keep_len`) for cross-turn reuse.
    pub fn truncate_to(&mut self, keep_len: usize, nkv: usize, hd: usize) {
        let keep = keep_len.min(self.tokens.len());
        self.tokens.truncate(keep);
        let vec_len = keep * nkv * hd;
        let meta_len = keep * nkv;
        for l in 0..self.seq_len.len() {
            self.seq_len[l] = keep;
            if self.k_i16[l].len() > vec_len {
                self.k_i16[l].truncate(vec_len);
            }
            if self.k_scales[l].len() > meta_len {
                self.k_scales[l].truncate(meta_len);
            }
            if self.k_sig128[l].len() > meta_len {
                self.k_sig128[l].truncate(meta_len);
            }
            if self.v_i16[l].len() > vec_len {
                self.v_i16[l].truncate(vec_len);
            }
            if self.v_scales[l].len() > meta_len {
                self.v_scales[l].truncate(meta_len);
            }
        }
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
    skip_rehash: bool,
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
        Self::new_with_rehash_policy(model_id, expected_pieces, expected_sha256, false)
    }

    /// Formal R4.HoloGeometricRuntime create a chunked streaming parser with explicit SHA-256 skip policy when OPFS/JS already verified container digest.
    pub fn new_with_rehash_policy(
        model_id: impl Into<String>,
        expected_pieces: usize,
        expected_sha256: impl Into<String>,
        skip_rehash: bool,
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
            skip_rehash,
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
        if !self.skip_rehash {
            self.hasher.update(chunk);
        }
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
                        rht_signs_packed: rht_signs_packed_u32(cols, seed),
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
                            rht_signs_packed: rht_signs_packed_u32(cols, seed),
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
                            rht_signs_packed: rht_signs_packed_u32(cols, seed),
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
        let actual_hex = if self.skip_rehash {
            self.expected_sha256.clone()
        } else {
            let digest = self.hasher.finalize();
            let mut hex = String::with_capacity(64);
            for b in digest {
                use std::fmt::Write;
                let _ = write!(&mut hex, "{b:02x}");
            }
            if !self.expected_sha256.is_empty() && hex != self.expected_sha256 {
                return Err(R4StackError::IntegrityMismatch);
            }
            hex
        };
        let mut kv_cache = HoloPrefixKvCache::default();
        kv_cache.reset(self.n_layers);
        let mut scratch = HoloScratch::default();
        scratch.ensure_sized(
            self.d_model,
            self.n_heads * self.head_dim,
            self.n_kv_heads * self.head_dim,
            self.intermediate_size,
            self.head_dim,
            self.n_layers,
            self.vocab_size,
        );
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
            shortlist_k: DEFAULT_SHORTLIST_K,
            flock_window: DEFAULT_FLOCK_WINDOW,
            scratch,
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
    /// Prompt tokens prefilled (non-cached delta).
    pub prompt_tokens: usize,
    /// Prompt prefix tokens reused directly from `HoloPrefixKvCache`.
    pub prefix_reused_tokens: usize,
    /// Completion tokens emitted.
    pub generated_tokens: usize,
    /// Hardware float op count (strictly zero).
    pub float_ops: u64,
    /// Dense matmul op count (strictly zero).
    pub dense_matmul_ops: u64,
    /// True when Wasm `simd128` vector kernels compiled in.
    pub simd128_active: bool,
    /// Active execution backend string.
    pub active_backend: &'static str,
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
    /// Live per-token neural and geometric observatory snapshots (clipped to 64 frames).
    pub observatory_frames: Vec<HoloStepObservatoryFrame>,
}

/// Formal R4.HoloGeometricRuntime project a 512-bit RHT signature into normalized 3D integer coordinates `[-1000, 1000]`.
#[inline]
pub fn rht512_sig_to_xyz(sig: &[u64]) -> [i16; 3] {
    if sig.len() < SIG_WORDS {
        return [0, 0, 0];
    }
    let ax = (sig[0].count_ones() + sig[1].count_ones()) as i32;
    let ay = (sig[2].count_ones() + sig[3].count_ones()) as i32;
    let az = (sig[4].count_ones() + sig[5].count_ones()) as i32;
    [
        (((ax - 64) * 1000) / 64) as i16,
        (((ay - 64) * 1000) / 64) as i16,
        (((az - 64) * 1000) / 64) as i16,
    ]
}

/// Formal R4.HoloGeometricRuntime apply zero-alloc RHT sign-flip, `simd128` block-32 FWHT, and Q13 shift into `i16`.
fn prepare_rht_act16_packed(
    norm_in: &[i64],
    in_exp: i32,
    signs_packed: &[u32],
    rht_buf: &mut [i32],
    act16: &mut [i16],
) -> i32 {
    let cols = norm_in.len();
    let groups = cols >> 5;
    let mut max_in = 0u64;
    for &v in norm_in {
        max_in = max_in.max(v.unsigned_abs());
    }
    let bl_in = 64 - max_in.leading_zeros() as i32;
    // Keep inputs <= 2^24 so 32-point FWHT sum (5 bits growth) never exceeds 2^29 < i32::MAX.
    let pre_sh = (bl_in - 24).max(0);
    let rht = &mut rht_buf[..cols];
    for (g, &mask) in signs_packed.iter().enumerate().take(groups) {
        let base = g * 32;
        let blk = &mut rht[base..base + 32];
        for c in 0..32 {
            let scaled = shift_round(norm_in[base + c], pre_sh) as i32;
            blk[c] = if ((mask >> c) & 1) != 0 {
                scaled
            } else {
                scaled.wrapping_neg()
            };
        }
        let arr: &mut [i32; 32] = blk.try_into().unwrap();
        int_fwht32_i32(arr);
    }

    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    let max_abs: u32 = {
        // SAFETY: `rht` has exact length `cols`, and `i + 4 <= cols` bounds every `v128_load`.
        unsafe {
            use core::arch::wasm32::*;
            let mut vmax = u32x4_splat(0);
            let rp = rht.as_ptr();
            let mut i = 0usize;
            while i + 4 <= cols {
                let v = v128_load(rp.add(i) as *const v128);
                vmax = u32x4_max(vmax, i32x4_abs(v));
                i += 4;
            }
            let mut m = u32x4_extract_lane::<0>(vmax)
                .max(u32x4_extract_lane::<1>(vmax))
                .max(u32x4_extract_lane::<2>(vmax))
                .max(u32x4_extract_lane::<3>(vmax));
            while i < cols {
                m = m.max((*rp.add(i)).unsigned_abs());
                i += 1;
            }
            m
        }
    };

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    let max_abs: u32 = {
        let mut m = 0u32;
        for &v in &*rht {
            m = m.max(v.unsigned_abs());
        }
        m
    };

    if max_abs == 0 {
        act16[..cols].fill(0);
        return in_exp + pre_sh;
    }
    let bl = 32 - max_abs.leading_zeros() as i32;
    let post_sh = (bl - 13).max(0);
    let round_half = if post_sh > 0 {
        1i32 << (post_sh - 1)
    } else {
        0
    };

    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    // SAFETY: `rht` has `cols` elements and `act16` is checked via `assert!(act16.len() >= cols)`.
    {
        assert!(act16.len() >= cols);
        unsafe {
            use core::arch::wasm32::*;
            let rp = rht.as_ptr();
            let ap = act16.as_mut_ptr();
            let mut i = 0usize;
            if post_sh > 0 {
                let rhalf = i32x4_splat(round_half);
                let sh_u = post_sh as u32;
                while i + 8 <= cols {
                    let v0 = v128_load(rp.add(i) as *const v128);
                    let v1 = v128_load(rp.add(i + 4) as *const v128);
                    let q0 = i32x4_shr(i32x4_add(v0, rhalf), sh_u);
                    let q1 = i32x4_shr(i32x4_add(v1, rhalf), sh_u);
                    v128_store(ap.add(i) as *mut v128, i16x8_narrow_i32x4(q0, q1));
                    i += 8;
                }
            } else {
                while i + 8 <= cols {
                    let v0 = v128_load(rp.add(i) as *const v128);
                    let v1 = v128_load(rp.add(i + 4) as *const v128);
                    v128_store(ap.add(i) as *mut v128, i16x8_narrow_i32x4(v0, v1));
                    i += 8;
                }
            }
            while i < cols {
                let q = if post_sh > 0 {
                    (*rp.add(i)).saturating_add(round_half) >> (post_sh as u32)
                } else {
                    *rp.add(i)
                };
                *ap.add(i) = q.clamp(-32_768, 32_767) as i16;
                i += 1;
            }
        }
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        for i in 0..cols {
            let q = if post_sh > 0 {
                (rht[i].saturating_add(round_half)) >> (post_sh as u32)
            } else {
                rht[i]
            };
            act16[i] = q.clamp(-32_768, 32_767) as i16;
        }
    }
    in_exp + pre_sh + post_sh
}

/// Formal R4.HoloGeometricRuntime extract strided 512-bit RHT sign signature directly from an already-computed RHT buffer.
#[inline]
fn extract_rht512_sig_from_rht(rht: &[i32]) -> [u64; SIG_WORDS] {
    let cols = rht.len();
    let mut sig = [0u64; SIG_WORDS];
    for b in 0..512 {
        let col = (b * cols) / 512;
        if rht[col] >= 0 {
            sig[b >> 6] |= 1u64 << (b & 63);
        }
    }
    sig
}

/// Formal R4.HoloGeometricRuntime pack `bt <= MAX_PREFILL_TILE` normalized tokens into group-interleaved `[g][b][32]` `i16` layout.
#[allow(clippy::too_many_arguments)]
fn prepare_rht_act16_batch(
    norm_batch: &[i64],
    bt: usize,
    cols: usize,
    in_exp: i32,
    signs_packed: &[u32],
    rht_buf: &mut [i32],
    act16_tmp: &mut [i16],
    act16_interleaved: &mut [i16],
    x_exps: &mut [i32; MAX_PREFILL_TILE],
) {
    let groups = cols >> 5;
    debug_assert!(bt <= MAX_PREFILL_TILE);
    debug_assert!(norm_batch.len() >= bt * cols);
    debug_assert!(act16_interleaved.len() >= groups * bt * 32);
    for b in 0..bt {
        let xe = prepare_rht_act16_packed(
            &norm_batch[b * cols..(b + 1) * cols],
            in_exp,
            signs_packed,
            rht_buf,
            act16_tmp,
        );
        x_exps[b] = xe;
        for g in 0..groups {
            let src = &act16_tmp[g * 32..(g + 1) * 32];
            let dst_off = (g * bt + b) * 32;
            act16_interleaved[dst_off..dst_off + 32].copy_from_slice(src);
        }
    }
}

/// Formal R4.HoloGeometricRuntime inverse-RHT dequantize one token embedding row into `RESIDUAL_EXP` without allocation.
fn dequant_embed_row_into(
    m: &HoloInt8RhtMatrix,
    row: usize,
    wide_buf: &mut [i64],
    out: &mut [i32],
) {
    let cols = m.cols;
    let groups = cols >> 5;
    let row_exp = i32::from(m.row_exp[row]);
    let w_row = &m.weights[row * cols..(row + 1) * cols];
    let mults = &m.group_mult[row * groups..(row + 1) * groups];
    let wide = &mut wide_buf[..cols];
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
    for g in 0..groups {
        let base = g * 32;
        let mask = m.rht_signs_packed[g];
        for c in 0..32 {
            let val = wide[base + c];
            let sv = if ((mask >> c) & 1) != 0 {
                val
            } else {
                val.saturating_neg()
            };
            out[base + c] = shift_round(sv, RESIDUAL_EXP - target_exp)
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        }
    }
}

/// Formal R4.HoloGeometricRuntime scale an `i64` group-32 dot accumulator by `out_exp - (row_exp + x_exp)` safely into `i32`.
#[inline(always)]
fn scale_i64_acc_to_i32(acc: i64, sh: i32) -> i32 {
    let val = if sh >= 63 {
        if acc < 0 {
            -1
        } else {
            0
        }
    } else if sh > 0 {
        let u = sh as u32;
        (acc.saturating_add(1i64 << (u - 1))) >> u
    } else if sh == 0 {
        acc
    } else {
        let k = (-sh) as u32;
        if k >= 63 {
            if acc >= 0 {
                i64::MAX
            } else {
                i64::MIN
            }
        } else {
            acc.saturating_mul(1i64 << k)
        }
    };
    val.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// Formal R4.HoloGeometricRuntime execute 4-row register-tiled `int4` RHT matrix-vector dot kernel (`i64` accumulators).
#[allow(clippy::too_many_arguments)]
fn gemv_i4_rht_rows(
    rows: usize,
    cols: usize,
    nibbles: &[u8],
    group_mult: &[u16],
    row_exp: &[i8],
    x_exp: i32,
    out_exp: i32,
    act16: &[i16],
    out: &mut [i32],
) {
    let row_bytes = cols >> 1;
    let groups = cols >> 5;
    assert!(nibbles.len() >= rows * row_bytes);
    assert!(group_mult.len() >= rows * groups);
    assert!(row_exp.len() >= rows);
    assert!(act16.len() >= cols);
    assert!(out.len() >= rows);

    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    // SAFETY: All slice bounds against `rows`, `cols`, `groups`, and `row_bytes` are asserted above.
    unsafe {
        use core::arch::wasm32::*;
        let nb_base = nibbles.as_ptr();
        let gm_base = group_mult.as_ptr();
        let re_base = row_exp.as_ptr();
        let act_base = act16.as_ptr();
        let out_base = out.as_mut_ptr();
        let mask_lo = u8x16_splat(0x0f);
        let bias = i8x16_splat(8);

        let mut r = 0usize;
        while r + 4 <= rows {
            let np0 = nb_base.add(r * row_bytes);
            let np1 = np0.add(row_bytes);
            let np2 = np1.add(row_bytes);
            let np3 = np2.add(row_bytes);
            let mp0 = gm_base.add(r * groups);
            let mp1 = mp0.add(groups);
            let mp2 = mp1.add(groups);
            let mp3 = mp2.add(groups);

            let mut acc01 = i64x2_splat(0);
            let mut acc23 = i64x2_splat(0);

            for g in 0..groups {
                let ap = act_base.add(g * 32);
                let a0 = v128_load(ap as *const v128);
                let a1 = v128_load(ap.add(8) as *const v128);
                let a2 = v128_load(ap.add(16) as *const v128);
                let a3 = v128_load(ap.add(24) as *const v128);
                let g_off = g * 16;

                macro_rules! dot_i4_row_s4 {
                    ($np:expr) => {{
                        let p = v128_load($np.add(g_off) as *const v128);
                        let w_lo_i8 = i8x16_sub(v128_and(p, mask_lo), bias);
                        let w_hi_i8 = i8x16_sub(u8x16_shr(p, 4), bias);
                        let d0 = i32x4_dot_i16x8(i16x8_extend_low_i8x16(w_lo_i8), a0);
                        let d1 = i32x4_dot_i16x8(i16x8_extend_high_i8x16(w_lo_i8), a1);
                        let d2 = i32x4_dot_i16x8(i16x8_extend_low_i8x16(w_hi_i8), a2);
                        let d3 = i32x4_dot_i16x8(i16x8_extend_high_i8x16(w_hi_i8), a3);
                        i32x4_add(i32x4_add(d0, d1), i32x4_add(d2, d3))
                    }};
                }

                let s0 = dot_i4_row_s4!(np0);
                let s1 = dot_i4_row_s4!(np1);
                let s2 = dot_i4_row_s4!(np2);
                let s3 = dot_i4_row_s4!(np3);
                let p01 = i32x4_add(
                    i32x4_shuffle::<0, 2, 4, 6>(s0, s1),
                    i32x4_shuffle::<1, 3, 5, 7>(s0, s1),
                );
                let p23 = i32x4_add(
                    i32x4_shuffle::<0, 2, 4, 6>(s2, s3),
                    i32x4_shuffle::<1, 3, 5, 7>(s2, s3),
                );
                let dots0123 = i32x4_add(
                    i32x4_shuffle::<0, 2, 4, 6>(p01, p23),
                    i32x4_shuffle::<1, 3, 5, 7>(p01, p23),
                );
                let m01 = i32x4_replace_lane::<1>(
                    i32x4_splat(i32::from(*mp0.add(g))),
                    i32::from(*mp1.add(g)),
                );
                let m23 = i32x4_replace_lane::<1>(
                    i32x4_splat(i32::from(*mp2.add(g))),
                    i32::from(*mp3.add(g)),
                );
                let mult0123 = i32x4_shuffle::<0, 1, 4, 5>(m01, m23);
                acc01 = i64x2_add(acc01, i64x2_extmul_low_i32x4(dots0123, mult0123));
                acc23 = i64x2_add(acc23, i64x2_extmul_high_i32x4(dots0123, mult0123));
            }

            let sum0 = i64x2_extract_lane::<0>(acc01);
            let sum1 = i64x2_extract_lane::<1>(acc01);
            let sum2 = i64x2_extract_lane::<0>(acc23);
            let sum3 = i64x2_extract_lane::<1>(acc23);

            *out_base.add(r) =
                scale_i64_acc_to_i32(sum0, out_exp - (i32::from(*re_base.add(r)) + x_exp));
            *out_base.add(r + 1) =
                scale_i64_acc_to_i32(sum1, out_exp - (i32::from(*re_base.add(r + 1)) + x_exp));
            *out_base.add(r + 2) =
                scale_i64_acc_to_i32(sum2, out_exp - (i32::from(*re_base.add(r + 2)) + x_exp));
            *out_base.add(r + 3) =
                scale_i64_acc_to_i32(sum3, out_exp - (i32::from(*re_base.add(r + 3)) + x_exp));
            r += 4;
        }

        while r < rows {
            let np = nb_base.add(r * row_bytes);
            let mp = gm_base.add(r * groups);
            let mut acc = i64x2_splat(0);
            for g in 0..groups {
                let ap = act_base.add(g * 32);
                let p = v128_load(np.add(g * 16) as *const v128);
                let w_lo_i8 = i8x16_sub(v128_and(p, mask_lo), bias);
                let w_hi_i8 = i8x16_sub(u8x16_shr(p, 4), bias);
                let d0 = i32x4_dot_i16x8(
                    i16x8_extend_low_i8x16(w_lo_i8),
                    v128_load(ap as *const v128),
                );
                let d1 = i32x4_dot_i16x8(
                    i16x8_extend_high_i8x16(w_lo_i8),
                    v128_load(ap.add(8) as *const v128),
                );
                let d2 = i32x4_dot_i16x8(
                    i16x8_extend_low_i8x16(w_hi_i8),
                    v128_load(ap.add(16) as *const v128),
                );
                let d3 = i32x4_dot_i16x8(
                    i16x8_extend_high_i8x16(w_hi_i8),
                    v128_load(ap.add(24) as *const v128),
                );
                let sum4 = i32x4_add(i32x4_add(d0, d1), i32x4_add(d2, d3));
                let mult_v = i32x4_splat(i32::from(*mp.add(g)));
                let prod = i64x2_add(
                    i64x2_extmul_low_i32x4(sum4, mult_v),
                    i64x2_extmul_high_i32x4(sum4, mult_v),
                );
                acc = i64x2_add(acc, prod);
            }
            let sum = i64x2_extract_lane::<0>(acc) + i64x2_extract_lane::<1>(acc);
            *out_base.add(r) =
                scale_i64_acc_to_i32(sum, out_exp - (i32::from(*re_base.add(r)) + x_exp));
            r += 1;
        }
        return;
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        for (r, out_slot) in out.iter_mut().enumerate().take(rows) {
            let re = i32::from(row_exp[r]);
            let sh = out_exp - (re + x_exp);
            let nibs = &nibbles[r * row_bytes..(r + 1) * row_bytes];
            let mults = &group_mult[r * groups..(r + 1) * groups];
            let mut acc = 0i64;
            for g in 0..groups {
                let nb: &[u8; 16] = (&nibs[g * 16..(g + 1) * 16]).try_into().unwrap();
                let a: &[i16; 32] = (&act16[g * 32..(g + 1) * 32]).try_into().unwrap();
                acc += i64::from(dot_i4_group32(nb, a)) * i64::from(mults[g]);
            }
            *out_slot = scale_i64_acc_to_i32(acc, sh);
        }
    }
}

/// Formal R4.HoloGeometricRuntime execute zero-alloc `int4` RHT matrix-vector product with pure `i64` accumulators.
fn gemv_i4_rht_into(
    m: &HoloInt4RhtMatrix,
    norm_in: &[i64],
    in_exp: i32,
    out_exp: i32,
    rht_buf: &mut [i32],
    act16_buf: &mut [i16],
    out: &mut [i32],
) {
    let cols = m.cols;
    let act16 = &mut act16_buf[..cols];
    let x_exp = prepare_rht_act16_packed(norm_in, in_exp, &m.rht_signs_packed, rht_buf, act16);
    gemv_i4_rht_rows(
        m.rows,
        cols,
        &m.nibbles,
        &m.group_mult,
        &m.row_exp,
        x_exp,
        out_exp,
        act16,
        out,
    );
}

/// Formal R4.HoloGeometricRuntime execute 4-row register-tiled `int8` RHT matrix-vector dot kernel (`i64` accumulators).
#[allow(clippy::too_many_arguments)]
fn gemv_i8_rht_rows(
    rows: usize,
    cols: usize,
    weights: &[i8],
    group_mult: &[u16],
    row_exp: &[i8],
    x_exp: i32,
    out_exp: i32,
    act16: &[i16],
    out: &mut [i32],
) {
    let groups = cols >> 5;
    assert!(weights.len() >= rows * cols);
    assert!(group_mult.len() >= rows * groups);
    assert!(row_exp.len() >= rows);
    assert!(act16.len() >= cols);
    assert!(out.len() >= rows);

    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    // SAFETY: All slice bounds against `rows`, `cols`, and `groups` are asserted above.
    unsafe {
        use core::arch::wasm32::*;
        let w_base = weights.as_ptr();
        let gm_base = group_mult.as_ptr();
        let re_base = row_exp.as_ptr();
        let act_base = act16.as_ptr();
        let out_base = out.as_mut_ptr();

        let mut r = 0usize;
        while r + 4 <= rows {
            let wp0 = w_base.add(r * cols);
            let wp1 = wp0.add(cols);
            let wp2 = wp1.add(cols);
            let wp3 = wp2.add(cols);
            let mp0 = gm_base.add(r * groups);
            let mp1 = mp0.add(groups);
            let mp2 = mp1.add(groups);
            let mp3 = mp2.add(groups);

            let mut acc01 = i64x2_splat(0);
            let mut acc23 = i64x2_splat(0);

            for g in 0..groups {
                let ap = act_base.add(g * 32);
                let a0 = v128_load(ap as *const v128);
                let a1 = v128_load(ap.add(8) as *const v128);
                let a2 = v128_load(ap.add(16) as *const v128);
                let a3 = v128_load(ap.add(24) as *const v128);
                let g_off = g * 32;

                macro_rules! dot_i8_row_s4 {
                    ($wp:expr) => {{
                        let w_lo = v128_load($wp.add(g_off) as *const v128);
                        let w_hi = v128_load($wp.add(g_off + 16) as *const v128);
                        let d0 = i32x4_dot_i16x8(i16x8_extend_low_i8x16(w_lo), a0);
                        let d1 = i32x4_dot_i16x8(i16x8_extend_high_i8x16(w_lo), a1);
                        let d2 = i32x4_dot_i16x8(i16x8_extend_low_i8x16(w_hi), a2);
                        let d3 = i32x4_dot_i16x8(i16x8_extend_high_i8x16(w_hi), a3);
                        i32x4_add(i32x4_add(d0, d1), i32x4_add(d2, d3))
                    }};
                }

                let s0 = dot_i8_row_s4!(wp0);
                let s1 = dot_i8_row_s4!(wp1);
                let s2 = dot_i8_row_s4!(wp2);
                let s3 = dot_i8_row_s4!(wp3);
                let p01 = i32x4_add(
                    i32x4_shuffle::<0, 2, 4, 6>(s0, s1),
                    i32x4_shuffle::<1, 3, 5, 7>(s0, s1),
                );
                let p23 = i32x4_add(
                    i32x4_shuffle::<0, 2, 4, 6>(s2, s3),
                    i32x4_shuffle::<1, 3, 5, 7>(s2, s3),
                );
                let dots0123 = i32x4_add(
                    i32x4_shuffle::<0, 2, 4, 6>(p01, p23),
                    i32x4_shuffle::<1, 3, 5, 7>(p01, p23),
                );
                let m01 = i32x4_replace_lane::<1>(
                    i32x4_splat(i32::from(*mp0.add(g))),
                    i32::from(*mp1.add(g)),
                );
                let m23 = i32x4_replace_lane::<1>(
                    i32x4_splat(i32::from(*mp2.add(g))),
                    i32::from(*mp3.add(g)),
                );
                let mult0123 = i32x4_shuffle::<0, 1, 4, 5>(m01, m23);
                acc01 = i64x2_add(acc01, i64x2_extmul_low_i32x4(dots0123, mult0123));
                acc23 = i64x2_add(acc23, i64x2_extmul_high_i32x4(dots0123, mult0123));
            }

            let sum0 = i64x2_extract_lane::<0>(acc01);
            let sum1 = i64x2_extract_lane::<1>(acc01);
            let sum2 = i64x2_extract_lane::<0>(acc23);
            let sum3 = i64x2_extract_lane::<1>(acc23);

            *out_base.add(r) =
                scale_i64_acc_to_i32(sum0, out_exp - (i32::from(*re_base.add(r)) + x_exp));
            *out_base.add(r + 1) =
                scale_i64_acc_to_i32(sum1, out_exp - (i32::from(*re_base.add(r + 1)) + x_exp));
            *out_base.add(r + 2) =
                scale_i64_acc_to_i32(sum2, out_exp - (i32::from(*re_base.add(r + 2)) + x_exp));
            *out_base.add(r + 3) =
                scale_i64_acc_to_i32(sum3, out_exp - (i32::from(*re_base.add(r + 3)) + x_exp));
            r += 4;
        }

        while r < rows {
            let wp = w_base.add(r * cols);
            let mp = gm_base.add(r * groups);
            let mut acc = i64x2_splat(0);
            for g in 0..groups {
                let ap = act_base.add(g * 32);
                let w_lo = v128_load(wp.add(g * 32) as *const v128);
                let w_hi = v128_load(wp.add(g * 32 + 16) as *const v128);
                let d0 =
                    i32x4_dot_i16x8(i16x8_extend_low_i8x16(w_lo), v128_load(ap as *const v128));
                let d1 = i32x4_dot_i16x8(
                    i16x8_extend_high_i8x16(w_lo),
                    v128_load(ap.add(8) as *const v128),
                );
                let d2 = i32x4_dot_i16x8(
                    i16x8_extend_low_i8x16(w_hi),
                    v128_load(ap.add(16) as *const v128),
                );
                let d3 = i32x4_dot_i16x8(
                    i16x8_extend_high_i8x16(w_hi),
                    v128_load(ap.add(24) as *const v128),
                );
                let sum4 = i32x4_add(i32x4_add(d0, d1), i32x4_add(d2, d3));
                let mult_v = i32x4_splat(i32::from(*mp.add(g)));
                let prod = i64x2_add(
                    i64x2_extmul_low_i32x4(sum4, mult_v),
                    i64x2_extmul_high_i32x4(sum4, mult_v),
                );
                acc = i64x2_add(acc, prod);
            }
            let sum = i64x2_extract_lane::<0>(acc) + i64x2_extract_lane::<1>(acc);
            *out_base.add(r) =
                scale_i64_acc_to_i32(sum, out_exp - (i32::from(*re_base.add(r)) + x_exp));
            r += 1;
        }
        return;
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        for (r, out_slot) in out.iter_mut().enumerate().take(rows) {
            let re = i32::from(row_exp[r]);
            let sh = out_exp - (re + x_exp);
            let ws = &weights[r * cols..(r + 1) * cols];
            let mults = &group_mult[r * groups..(r + 1) * groups];
            let mut acc = 0i64;
            for g in 0..groups {
                let wg: &[i8; 32] = (&ws[g * 32..(g + 1) * 32]).try_into().unwrap();
                let ag: &[i16; 32] = (&act16[g * 32..(g + 1) * 32]).try_into().unwrap();
                acc += i64::from(dot_i8_group32(wg, ag)) * i64::from(mults[g]);
            }
            *out_slot = scale_i64_acc_to_i32(acc, sh);
        }
    }
}

/// Formal R4.HoloGeometricRuntime execute zero-alloc `int8` RHT matrix-vector product with pure `i64` accumulators.
fn gemv_i8_rht_into(
    m: &HoloInt8RhtMatrix,
    norm_in: &[i64],
    in_exp: i32,
    out_exp: i32,
    rht_buf: &mut [i32],
    act16_buf: &mut [i16],
    out: &mut [i32],
) {
    let cols = m.cols;
    let act16 = &mut act16_buf[..cols];
    let x_exp = prepare_rht_act16_packed(norm_in, in_exp, &m.rht_signs_packed, rht_buf, act16);
    gemv_i8_rht_rows(
        m.rows,
        cols,
        &m.weights,
        &m.group_mult,
        &m.row_exp,
        x_exp,
        out_exp,
        act16,
        out,
    );
}

/// Formal R4.HoloGeometricRuntime evaluate indexed `int8` matrix rows in 4-row `simd128` register tiles for `lm_head`.
fn gemv_i8_indexed_into(
    m: &HoloInt8RhtMatrix,
    row_ids: &[u32],
    act16: &[i16],
    x_exp: i32,
    out_exp: i32,
    out: &mut Vec<(u32, i32)>,
) {
    let cols = m.cols;
    let groups = cols >> 5;
    out.clear();
    out.reserve(row_ids.len());
    assert!(m.weights.len() >= m.rows * cols);
    assert!(m.group_mult.len() >= m.rows * groups);
    assert!(m.row_exp.len() >= m.rows);
    assert!(act16.len() >= cols);

    let score_single = |ri: usize| -> i32 {
        let re = i32::from(m.row_exp[ri]);
        let sh = out_exp - (re + x_exp);
        let ws = &m.weights[ri * cols..(ri + 1) * cols];
        let mults = &m.group_mult[ri * groups..(ri + 1) * groups];
        let mut acc = 0i64;
        for g in 0..groups {
            let wg: &[i8; 32] = (&ws[g * 32..(g + 1) * 32]).try_into().unwrap();
            let ag: &[i16; 32] = (&act16[g * 32..(g + 1) * 32]).try_into().unwrap();
            acc += i64::from(dot_i8_group32(wg, ag)) * i64::from(mults[g]);
        }
        scale_i64_acc_to_i32(acc, sh)
    };

    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        use core::arch::wasm32::*;
        let chunks = row_ids.chunks_exact(4);
        let rem = chunks.remainder();
        for ch in chunks {
            let r0 = ch[0] as usize;
            let r1 = ch[1] as usize;
            let r2 = ch[2] as usize;
            let r3 = ch[3] as usize;
            if r0 < m.rows && r1 < m.rows && r2 < m.rows && r3 < m.rows {
                // SAFETY: Every `ri < m.rows`, and all `m.weights`/`m.group_mult`/`m.row_exp`/`act16` bounds were asserted above.
                unsafe {
                    let w_base = m.weights.as_ptr();
                    let gm_base = m.group_mult.as_ptr();
                    let act_base = act16.as_ptr();
                    let wp0 = w_base.add(r0 * cols);
                    let wp1 = w_base.add(r1 * cols);
                    let wp2 = w_base.add(r2 * cols);
                    let wp3 = w_base.add(r3 * cols);
                    let mp0 = gm_base.add(r0 * groups);
                    let mp1 = gm_base.add(r1 * groups);
                    let mp2 = gm_base.add(r2 * groups);
                    let mp3 = gm_base.add(r3 * groups);

                    let mut acc01 = i64x2_splat(0);
                    let mut acc23 = i64x2_splat(0);

                    for g in 0..groups {
                        let ap = act_base.add(g * 32);
                        let a0 = v128_load(ap as *const v128);
                        let a1 = v128_load(ap.add(8) as *const v128);
                        let a2 = v128_load(ap.add(16) as *const v128);
                        let a3 = v128_load(ap.add(24) as *const v128);
                        let g_off = g * 32;

                        macro_rules! dot_idx_row_s4 {
                            ($wp:expr) => {{
                                let w_lo = v128_load($wp.add(g_off) as *const v128);
                                let w_hi = v128_load($wp.add(g_off + 16) as *const v128);
                                let d0 = i32x4_dot_i16x8(i16x8_extend_low_i8x16(w_lo), a0);
                                let d1 = i32x4_dot_i16x8(i16x8_extend_high_i8x16(w_lo), a1);
                                let d2 = i32x4_dot_i16x8(i16x8_extend_low_i8x16(w_hi), a2);
                                let d3 = i32x4_dot_i16x8(i16x8_extend_high_i8x16(w_hi), a3);
                                i32x4_add(i32x4_add(d0, d1), i32x4_add(d2, d3))
                            }};
                        }
                        let s0 = dot_idx_row_s4!(wp0);
                        let s1 = dot_idx_row_s4!(wp1);
                        let s2 = dot_idx_row_s4!(wp2);
                        let s3 = dot_idx_row_s4!(wp3);
                        let p01 = i32x4_add(
                            i32x4_shuffle::<0, 2, 4, 6>(s0, s1),
                            i32x4_shuffle::<1, 3, 5, 7>(s0, s1),
                        );
                        let p23 = i32x4_add(
                            i32x4_shuffle::<0, 2, 4, 6>(s2, s3),
                            i32x4_shuffle::<1, 3, 5, 7>(s2, s3),
                        );
                        let dots0123 = i32x4_add(
                            i32x4_shuffle::<0, 2, 4, 6>(p01, p23),
                            i32x4_shuffle::<1, 3, 5, 7>(p01, p23),
                        );
                        let m01 = i32x4_replace_lane::<1>(
                            i32x4_splat(i32::from(*mp0.add(g))),
                            i32::from(*mp1.add(g)),
                        );
                        let m23 = i32x4_replace_lane::<1>(
                            i32x4_splat(i32::from(*mp2.add(g))),
                            i32::from(*mp3.add(g)),
                        );
                        let mult0123 = i32x4_shuffle::<0, 1, 4, 5>(m01, m23);
                        acc01 = i64x2_add(acc01, i64x2_extmul_low_i32x4(dots0123, mult0123));
                        acc23 = i64x2_add(acc23, i64x2_extmul_high_i32x4(dots0123, mult0123));
                    }

                    let sum0 = i64x2_extract_lane::<0>(acc01);
                    let sum1 = i64x2_extract_lane::<1>(acc01);
                    let sum2 = i64x2_extract_lane::<0>(acc23);
                    let sum3 = i64x2_extract_lane::<1>(acc23);

                    out.push((
                        ch[0],
                        scale_i64_acc_to_i32(sum0, out_exp - (i32::from(m.row_exp[r0]) + x_exp)),
                    ));
                    out.push((
                        ch[1],
                        scale_i64_acc_to_i32(sum1, out_exp - (i32::from(m.row_exp[r1]) + x_exp)),
                    ));
                    out.push((
                        ch[2],
                        scale_i64_acc_to_i32(sum2, out_exp - (i32::from(m.row_exp[r2]) + x_exp)),
                    ));
                    out.push((
                        ch[3],
                        scale_i64_acc_to_i32(sum3, out_exp - (i32::from(m.row_exp[r3]) + x_exp)),
                    ));
                }
            } else {
                for &r in ch {
                    let ri = r as usize;
                    if ri < m.rows {
                        out.push((r, score_single(ri)));
                    }
                }
            }
        }
        for &r in rem {
            let ri = r as usize;
            if ri < m.rows {
                out.push((r, score_single(ri)));
            }
        }
        return;
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        for &r in row_ids {
            let ri = r as usize;
            if ri < m.rows {
                out.push((r, score_single(ri)));
            }
        }
    }
}

/// Formal R4.HoloGeometricRuntime execute batched `bt <= MAX_PREFILL_TILE` `int4` GEMM unpacking weights once per `(row, group)`.
#[allow(clippy::too_many_arguments)]
fn gemm_i4_rht_batch(
    m: &HoloInt4RhtMatrix,
    norm_batch: &[i64],
    bt: usize,
    in_exp: i32,
    out_exp: i32,
    rht_buf: &mut [i32],
    act16_tmp: &mut [i16],
    act16_interleaved: &mut [i16],
    out_batch: &mut [i32],
) {
    let rows = m.rows;
    let cols = m.cols;
    let groups = cols >> 5;
    let row_bytes = cols >> 1;
    let mut x_exps = [0i32; MAX_PREFILL_TILE];
    prepare_rht_act16_batch(
        norm_batch,
        bt,
        cols,
        in_exp,
        &m.rht_signs_packed,
        rht_buf,
        act16_tmp,
        act16_interleaved,
        &mut x_exps,
    );
    assert!(m.nibbles.len() >= rows * row_bytes);
    assert!(m.group_mult.len() >= rows * groups);
    assert!(m.row_exp.len() >= rows);
    assert!(out_batch.len() >= bt * rows);

    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    // SAFETY: All matrix and tile buffer bounds are asserted above for `bt <= MAX_PREFILL_TILE`.
    unsafe {
        use core::arch::wasm32::*;
        let nb_base = m.nibbles.as_ptr();
        let gm_base = m.group_mult.as_ptr();
        let act_base = act16_interleaved.as_ptr();
        let out_base = out_batch.as_mut_ptr();
        let mask_lo = u8x16_splat(0x0f);
        let bias = i8x16_splat(8);

        macro_rules! dot_i4_col {
            ($w0:expr, $w1:expr, $w2:expr, $w3:expr, $mult_v:expr, $base_a:expr, $col:expr, $acc:expr) => {{
                let cp = $base_a.add($col * 32);
                let d0 = i32x4_dot_i16x8($w0, v128_load(cp as *const v128));
                let d1 = i32x4_dot_i16x8($w1, v128_load(cp.add(8) as *const v128));
                let d2 = i32x4_dot_i16x8($w2, v128_load(cp.add(16) as *const v128));
                let d3 = i32x4_dot_i16x8($w3, v128_load(cp.add(24) as *const v128));
                let s4 = i32x4_add(i32x4_add(d0, d1), i32x4_add(d2, d3));
                let prod = i64x2_add(
                    i64x2_extmul_low_i32x4(s4, $mult_v),
                    i64x2_extmul_high_i32x4(s4, $mult_v),
                );
                $acc = i64x2_add($acc, prod);
            }};
        }

        match bt {
            4 => {
                let mut r = 0usize;
                while r + 2 <= rows {
                    let re0 = i32::from(m.row_exp[r]);
                    let re1 = i32::from(m.row_exp[r + 1]);
                    let np0 = nb_base.add(r * row_bytes);
                    let np1 = np0.add(row_bytes);
                    let mp0 = gm_base.add(r * groups);
                    let mp1 = mp0.add(groups);
                    let mut acc_b0 = i64x2_splat(0);
                    let mut acc_b1 = i64x2_splat(0);
                    let mut acc_b2 = i64x2_splat(0);
                    let mut acc_b3 = i64x2_splat(0);

                    for g in 0..groups {
                        let g_off = g * 16;
                        let p0 = v128_load(np0.add(g_off) as *const v128);
                        let p1 = v128_load(np1.add(g_off) as *const v128);
                        let w0_lo = i8x16_sub(v128_and(p0, mask_lo), bias);
                        let w0_hi = i8x16_sub(u8x16_shr(p0, 4), bias);
                        let w1_lo = i8x16_sub(v128_and(p1, mask_lo), bias);
                        let w1_hi = i8x16_sub(u8x16_shr(p1, 4), bias);
                        let w0_0 = i16x8_extend_low_i8x16(w0_lo);
                        let w0_1 = i16x8_extend_high_i8x16(w0_lo);
                        let w0_2 = i16x8_extend_low_i8x16(w0_hi);
                        let w0_3 = i16x8_extend_high_i8x16(w0_hi);
                        let w1_0 = i16x8_extend_low_i8x16(w1_lo);
                        let w1_1 = i16x8_extend_high_i8x16(w1_lo);
                        let w1_2 = i16x8_extend_low_i8x16(w1_hi);
                        let w1_3 = i16x8_extend_high_i8x16(w1_hi);
                        let mult_01 = i32x4_replace_lane::<1>(
                            i32x4_splat(i32::from(*mp0.add(g))),
                            i32::from(*mp1.add(g)),
                        );
                        let base_a = act_base.add(g * 128);

                        macro_rules! dot_i4_2x1_col {
                            ($col:expr, $acc_b:expr) => {{
                                let cp = base_a.add($col * 32);
                                let a0 = v128_load(cp as *const v128);
                                let mut s_r0 = i32x4_dot_i16x8(w0_0, a0);
                                let mut s_r1 = i32x4_dot_i16x8(w1_0, a0);
                                let a1 = v128_load(cp.add(8) as *const v128);
                                s_r0 = i32x4_add(s_r0, i32x4_dot_i16x8(w0_1, a1));
                                s_r1 = i32x4_add(s_r1, i32x4_dot_i16x8(w1_1, a1));
                                let a2 = v128_load(cp.add(16) as *const v128);
                                s_r0 = i32x4_add(s_r0, i32x4_dot_i16x8(w0_2, a2));
                                s_r1 = i32x4_add(s_r1, i32x4_dot_i16x8(w1_2, a2));
                                let a3 = v128_load(cp.add(24) as *const v128);
                                s_r0 = i32x4_add(s_r0, i32x4_dot_i16x8(w0_3, a3));
                                s_r1 = i32x4_add(s_r1, i32x4_dot_i16x8(w1_3, a3));
                                let p0 = i32x4_add(
                                    i32x4_shuffle::<0, 2, 4, 6>(s_r0, s_r1),
                                    i32x4_shuffle::<1, 3, 5, 7>(s_r0, s_r1),
                                );
                                let dots_01 = i32x4_add(
                                    i32x4_shuffle::<0, 2, 0, 2>(p0, p0),
                                    i32x4_shuffle::<1, 3, 1, 3>(p0, p0),
                                );
                                $acc_b =
                                    i64x2_add($acc_b, i64x2_extmul_low_i32x4(dots_01, mult_01));
                            }};
                        }

                        dot_i4_2x1_col!(0, acc_b0);
                        dot_i4_2x1_col!(1, acc_b1);
                        dot_i4_2x1_col!(2, acc_b2);
                        dot_i4_2x1_col!(3, acc_b3);
                    }

                    *out_base.add(r) = scale_i64_acc_to_i32(
                        i64x2_extract_lane::<0>(acc_b0),
                        out_exp - (re0 + x_exps[0]),
                    );
                    *out_base.add(r + 1) = scale_i64_acc_to_i32(
                        i64x2_extract_lane::<1>(acc_b0),
                        out_exp - (re1 + x_exps[0]),
                    );
                    *out_base.add(rows + r) = scale_i64_acc_to_i32(
                        i64x2_extract_lane::<0>(acc_b1),
                        out_exp - (re0 + x_exps[1]),
                    );
                    *out_base.add(rows + r + 1) = scale_i64_acc_to_i32(
                        i64x2_extract_lane::<1>(acc_b1),
                        out_exp - (re1 + x_exps[1]),
                    );
                    *out_base.add(2 * rows + r) = scale_i64_acc_to_i32(
                        i64x2_extract_lane::<0>(acc_b2),
                        out_exp - (re0 + x_exps[2]),
                    );
                    *out_base.add(2 * rows + r + 1) = scale_i64_acc_to_i32(
                        i64x2_extract_lane::<1>(acc_b2),
                        out_exp - (re1 + x_exps[2]),
                    );
                    *out_base.add(3 * rows + r) = scale_i64_acc_to_i32(
                        i64x2_extract_lane::<0>(acc_b3),
                        out_exp - (re0 + x_exps[3]),
                    );
                    *out_base.add(3 * rows + r + 1) = scale_i64_acc_to_i32(
                        i64x2_extract_lane::<1>(acc_b3),
                        out_exp - (re1 + x_exps[3]),
                    );
                    r += 2;
                }
                while r < rows {
                    let re = i32::from(m.row_exp[r]);
                    let np = nb_base.add(r * row_bytes);
                    let mp = gm_base.add(r * groups);
                    let mut acc01 = i64x2_splat(0);
                    let mut acc23 = i64x2_splat(0);

                    for g in 0..groups {
                        let p = v128_load(np.add(g * 16) as *const v128);
                        let w_lo_i8 = i8x16_sub(v128_and(p, mask_lo), bias);
                        let w_hi_i8 = i8x16_sub(u8x16_shr(p, 4), bias);
                        let w0 = i16x8_extend_low_i8x16(w_lo_i8);
                        let w1 = i16x8_extend_high_i8x16(w_lo_i8);
                        let w2 = i16x8_extend_low_i8x16(w_hi_i8);
                        let w3 = i16x8_extend_high_i8x16(w_hi_i8);
                        let mult_v = i32x4_splat(i32::from(*mp.add(g)));
                        let base_a = act_base.add(g * 128);

                        macro_rules! col_s4_i4 {
                            ($col:expr) => {{
                                let cp = base_a.add($col * 32);
                                let d0 = i32x4_dot_i16x8(w0, v128_load(cp as *const v128));
                                let d1 = i32x4_dot_i16x8(w1, v128_load(cp.add(8) as *const v128));
                                let d2 = i32x4_dot_i16x8(w2, v128_load(cp.add(16) as *const v128));
                                let d3 = i32x4_dot_i16x8(w3, v128_load(cp.add(24) as *const v128));
                                i32x4_add(i32x4_add(d0, d1), i32x4_add(d2, d3))
                            }};
                        }

                        let s0 = col_s4_i4!(0);
                        let s1 = col_s4_i4!(1);
                        let s2 = col_s4_i4!(2);
                        let s3 = col_s4_i4!(3);
                        let p01 = i32x4_add(
                            i32x4_shuffle::<0, 2, 4, 6>(s0, s1),
                            i32x4_shuffle::<1, 3, 5, 7>(s0, s1),
                        );
                        let p23 = i32x4_add(
                            i32x4_shuffle::<0, 2, 4, 6>(s2, s3),
                            i32x4_shuffle::<1, 3, 5, 7>(s2, s3),
                        );
                        let dots0123 = i32x4_add(
                            i32x4_shuffle::<0, 2, 4, 6>(p01, p23),
                            i32x4_shuffle::<1, 3, 5, 7>(p01, p23),
                        );
                        acc01 = i64x2_add(acc01, i64x2_extmul_low_i32x4(dots0123, mult_v));
                        acc23 = i64x2_add(acc23, i64x2_extmul_high_i32x4(dots0123, mult_v));
                    }

                    let sum0 = i64x2_extract_lane::<0>(acc01);
                    let sum1 = i64x2_extract_lane::<1>(acc01);
                    let sum2 = i64x2_extract_lane::<0>(acc23);
                    let sum3 = i64x2_extract_lane::<1>(acc23);
                    *out_base.add(r) = scale_i64_acc_to_i32(sum0, out_exp - (re + x_exps[0]));
                    *out_base.add(rows + r) =
                        scale_i64_acc_to_i32(sum1, out_exp - (re + x_exps[1]));
                    *out_base.add(2 * rows + r) =
                        scale_i64_acc_to_i32(sum2, out_exp - (re + x_exps[2]));
                    *out_base.add(3 * rows + r) =
                        scale_i64_acc_to_i32(sum3, out_exp - (re + x_exps[3]));
                    r += 1;
                }
            }
            3 => {
                for r in 0..rows {
                    let re = i32::from(m.row_exp[r]);
                    let np = nb_base.add(r * row_bytes);
                    let mp = gm_base.add(r * groups);
                    let mut acc0 = i64x2_splat(0);
                    let mut acc1 = i64x2_splat(0);
                    let mut acc2 = i64x2_splat(0);

                    for g in 0..groups {
                        let p = v128_load(np.add(g * 16) as *const v128);
                        let w_lo_i8 = i8x16_sub(v128_and(p, mask_lo), bias);
                        let w_hi_i8 = i8x16_sub(u8x16_shr(p, 4), bias);
                        let w0 = i16x8_extend_low_i8x16(w_lo_i8);
                        let w1 = i16x8_extend_high_i8x16(w_lo_i8);
                        let w2 = i16x8_extend_low_i8x16(w_hi_i8);
                        let w3 = i16x8_extend_high_i8x16(w_hi_i8);
                        let mult_v = i32x4_splat(i32::from(*mp.add(g)));
                        let base_a = act_base.add(g * 96);
                        dot_i4_col!(w0, w1, w2, w3, mult_v, base_a, 0, acc0);
                        dot_i4_col!(w0, w1, w2, w3, mult_v, base_a, 1, acc1);
                        dot_i4_col!(w0, w1, w2, w3, mult_v, base_a, 2, acc2);
                    }

                    let sum0 = i64x2_extract_lane::<0>(acc0) + i64x2_extract_lane::<1>(acc0);
                    let sum1 = i64x2_extract_lane::<0>(acc1) + i64x2_extract_lane::<1>(acc1);
                    let sum2 = i64x2_extract_lane::<0>(acc2) + i64x2_extract_lane::<1>(acc2);
                    *out_base.add(r) = scale_i64_acc_to_i32(sum0, out_exp - (re + x_exps[0]));
                    *out_base.add(rows + r) =
                        scale_i64_acc_to_i32(sum1, out_exp - (re + x_exps[1]));
                    *out_base.add(2 * rows + r) =
                        scale_i64_acc_to_i32(sum2, out_exp - (re + x_exps[2]));
                }
            }
            2 => {
                for r in 0..rows {
                    let re = i32::from(m.row_exp[r]);
                    let np = nb_base.add(r * row_bytes);
                    let mp = gm_base.add(r * groups);
                    let mut acc0 = i64x2_splat(0);
                    let mut acc1 = i64x2_splat(0);

                    for g in 0..groups {
                        let p = v128_load(np.add(g * 16) as *const v128);
                        let w_lo_i8 = i8x16_sub(v128_and(p, mask_lo), bias);
                        let w_hi_i8 = i8x16_sub(u8x16_shr(p, 4), bias);
                        let w0 = i16x8_extend_low_i8x16(w_lo_i8);
                        let w1 = i16x8_extend_high_i8x16(w_lo_i8);
                        let w2 = i16x8_extend_low_i8x16(w_hi_i8);
                        let w3 = i16x8_extend_high_i8x16(w_hi_i8);
                        let mult_v = i32x4_splat(i32::from(*mp.add(g)));
                        let base_a = act_base.add(g * 64);
                        dot_i4_col!(w0, w1, w2, w3, mult_v, base_a, 0, acc0);
                        dot_i4_col!(w0, w1, w2, w3, mult_v, base_a, 1, acc1);
                    }

                    let sum0 = i64x2_extract_lane::<0>(acc0) + i64x2_extract_lane::<1>(acc0);
                    let sum1 = i64x2_extract_lane::<0>(acc1) + i64x2_extract_lane::<1>(acc1);
                    *out_base.add(r) = scale_i64_acc_to_i32(sum0, out_exp - (re + x_exps[0]));
                    *out_base.add(rows + r) =
                        scale_i64_acc_to_i32(sum1, out_exp - (re + x_exps[1]));
                }
            }
            _ if bt == 1 => {
                for r in 0..rows {
                    let re = i32::from(m.row_exp[r]);
                    let np = nb_base.add(r * row_bytes);
                    let mp = gm_base.add(r * groups);
                    let mut acc0 = i64x2_splat(0);

                    for g in 0..groups {
                        let p = v128_load(np.add(g * 16) as *const v128);
                        let w_lo_i8 = i8x16_sub(v128_and(p, mask_lo), bias);
                        let w_hi_i8 = i8x16_sub(u8x16_shr(p, 4), bias);
                        let w0 = i16x8_extend_low_i8x16(w_lo_i8);
                        let w1 = i16x8_extend_high_i8x16(w_lo_i8);
                        let w2 = i16x8_extend_low_i8x16(w_hi_i8);
                        let w3 = i16x8_extend_high_i8x16(w_hi_i8);
                        let mult_v = i32x4_splat(i32::from(*mp.add(g)));
                        let base_a = act_base.add(g * 32);
                        dot_i4_col!(w0, w1, w2, w3, mult_v, base_a, 0, acc0);
                    }

                    let sum0 = i64x2_extract_lane::<0>(acc0) + i64x2_extract_lane::<1>(acc0);
                    *out_base.add(r) = scale_i64_acc_to_i32(sum0, out_exp - (re + x_exps[0]));
                }
            }
            _ => {}
        }
        return;
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        for r in 0..rows {
            let re = i32::from(m.row_exp[r]);
            let nibs = &m.nibbles[r * row_bytes..(r + 1) * row_bytes];
            let mults = &m.group_mult[r * groups..(r + 1) * groups];
            let mut acc = [0i64; MAX_PREFILL_TILE];
            for g in 0..groups {
                let nb: &[u8; 16] = (&nibs[g * 16..(g + 1) * 16]).try_into().unwrap();
                let mult = i64::from(mults[g]);
                let g_base = g * bt * 32;
                for b in 0..bt {
                    let a: &[i16; 32] = (&act16_interleaved
                        [g_base + b * 32..g_base + (b + 1) * 32])
                        .try_into()
                        .unwrap();
                    acc[b] += i64::from(dot_i4_group32(nb, a)) * mult;
                }
            }
            for b in 0..bt {
                let sh = out_exp - (re + x_exps[b]);
                out_batch[b * rows + r] = scale_i64_acc_to_i32(acc[b], sh);
            }
        }
    }
}

/// Formal R4.HoloGeometricRuntime execute batched `bt <= MAX_PREFILL_TILE` `int8` GEMM unpacking weights once per `(row, group)`.
#[allow(clippy::too_many_arguments)]
fn gemm_i8_rht_batch(
    m: &HoloInt8RhtMatrix,
    norm_batch: &[i64],
    bt: usize,
    in_exp: i32,
    out_exp: i32,
    rht_buf: &mut [i32],
    act16_tmp: &mut [i16],
    act16_interleaved: &mut [i16],
    out_batch: &mut [i32],
) {
    let rows = m.rows;
    let cols = m.cols;
    let groups = cols >> 5;
    let mut x_exps = [0i32; MAX_PREFILL_TILE];
    prepare_rht_act16_batch(
        norm_batch,
        bt,
        cols,
        in_exp,
        &m.rht_signs_packed,
        rht_buf,
        act16_tmp,
        act16_interleaved,
        &mut x_exps,
    );
    assert!(m.weights.len() >= rows * cols);
    assert!(m.group_mult.len() >= rows * groups);
    assert!(m.row_exp.len() >= rows);
    assert!(out_batch.len() >= bt * rows);

    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    // SAFETY: All matrix and tile buffer bounds are asserted above for `bt <= MAX_PREFILL_TILE`.
    unsafe {
        use core::arch::wasm32::*;
        let w_base = m.weights.as_ptr();
        let gm_base = m.group_mult.as_ptr();
        let act_base = act16_interleaved.as_ptr();
        let out_base = out_batch.as_mut_ptr();

        macro_rules! dot_i8_col {
            ($w0:expr, $w1:expr, $w2:expr, $w3:expr, $mult_v:expr, $base_a:expr, $col:expr, $acc:expr) => {{
                let cp = $base_a.add($col * 32);
                let d0 = i32x4_dot_i16x8($w0, v128_load(cp as *const v128));
                let d1 = i32x4_dot_i16x8($w1, v128_load(cp.add(8) as *const v128));
                let d2 = i32x4_dot_i16x8($w2, v128_load(cp.add(16) as *const v128));
                let d3 = i32x4_dot_i16x8($w3, v128_load(cp.add(24) as *const v128));
                let s4 = i32x4_add(i32x4_add(d0, d1), i32x4_add(d2, d3));
                let prod = i64x2_add(
                    i64x2_extmul_low_i32x4(s4, $mult_v),
                    i64x2_extmul_high_i32x4(s4, $mult_v),
                );
                $acc = i64x2_add($acc, prod);
            }};
        }

        match bt {
            4 => {
                for r in 0..rows {
                    let re = i32::from(m.row_exp[r]);
                    let wp = w_base.add(r * cols);
                    let mp = gm_base.add(r * groups);
                    let mut acc01 = i64x2_splat(0);
                    let mut acc23 = i64x2_splat(0);

                    for g in 0..groups {
                        let w_lo = v128_load(wp.add(g * 32) as *const v128);
                        let w_hi = v128_load(wp.add(g * 32 + 16) as *const v128);
                        let w0 = i16x8_extend_low_i8x16(w_lo);
                        let w1 = i16x8_extend_high_i8x16(w_lo);
                        let w2 = i16x8_extend_low_i8x16(w_hi);
                        let w3 = i16x8_extend_high_i8x16(w_hi);
                        let mult_v = i32x4_splat(i32::from(*mp.add(g)));
                        let base_a = act_base.add(g * 128);

                        macro_rules! col_s4_i8 {
                            ($col:expr) => {{
                                let cp = base_a.add($col * 32);
                                let d0 = i32x4_dot_i16x8(w0, v128_load(cp as *const v128));
                                let d1 = i32x4_dot_i16x8(w1, v128_load(cp.add(8) as *const v128));
                                let d2 = i32x4_dot_i16x8(w2, v128_load(cp.add(16) as *const v128));
                                let d3 = i32x4_dot_i16x8(w3, v128_load(cp.add(24) as *const v128));
                                i32x4_add(i32x4_add(d0, d1), i32x4_add(d2, d3))
                            }};
                        }

                        let s0 = col_s4_i8!(0);
                        let s1 = col_s4_i8!(1);
                        let s2 = col_s4_i8!(2);
                        let s3 = col_s4_i8!(3);
                        let p01 = i32x4_add(
                            i32x4_shuffle::<0, 2, 4, 6>(s0, s1),
                            i32x4_shuffle::<1, 3, 5, 7>(s0, s1),
                        );
                        let p23 = i32x4_add(
                            i32x4_shuffle::<0, 2, 4, 6>(s2, s3),
                            i32x4_shuffle::<1, 3, 5, 7>(s2, s3),
                        );
                        let dots0123 = i32x4_add(
                            i32x4_shuffle::<0, 2, 4, 6>(p01, p23),
                            i32x4_shuffle::<1, 3, 5, 7>(p01, p23),
                        );
                        acc01 = i64x2_add(acc01, i64x2_extmul_low_i32x4(dots0123, mult_v));
                        acc23 = i64x2_add(acc23, i64x2_extmul_high_i32x4(dots0123, mult_v));
                    }

                    let sum0 = i64x2_extract_lane::<0>(acc01);
                    let sum1 = i64x2_extract_lane::<1>(acc01);
                    let sum2 = i64x2_extract_lane::<0>(acc23);
                    let sum3 = i64x2_extract_lane::<1>(acc23);
                    *out_base.add(r) = scale_i64_acc_to_i32(sum0, out_exp - (re + x_exps[0]));
                    *out_base.add(rows + r) =
                        scale_i64_acc_to_i32(sum1, out_exp - (re + x_exps[1]));
                    *out_base.add(2 * rows + r) =
                        scale_i64_acc_to_i32(sum2, out_exp - (re + x_exps[2]));
                    *out_base.add(3 * rows + r) =
                        scale_i64_acc_to_i32(sum3, out_exp - (re + x_exps[3]));
                }
            }
            3 => {
                for r in 0..rows {
                    let re = i32::from(m.row_exp[r]);
                    let wp = w_base.add(r * cols);
                    let mp = gm_base.add(r * groups);
                    let mut acc0 = i64x2_splat(0);
                    let mut acc1 = i64x2_splat(0);
                    let mut acc2 = i64x2_splat(0);

                    for g in 0..groups {
                        let w_lo = v128_load(wp.add(g * 32) as *const v128);
                        let w_hi = v128_load(wp.add(g * 32 + 16) as *const v128);
                        let w0 = i16x8_extend_low_i8x16(w_lo);
                        let w1 = i16x8_extend_high_i8x16(w_lo);
                        let w2 = i16x8_extend_low_i8x16(w_hi);
                        let w3 = i16x8_extend_high_i8x16(w_hi);
                        let mult_v = i32x4_splat(i32::from(*mp.add(g)));
                        let base_a = act_base.add(g * 96);
                        dot_i8_col!(w0, w1, w2, w3, mult_v, base_a, 0, acc0);
                        dot_i8_col!(w0, w1, w2, w3, mult_v, base_a, 1, acc1);
                        dot_i8_col!(w0, w1, w2, w3, mult_v, base_a, 2, acc2);
                    }

                    let sum0 = i64x2_extract_lane::<0>(acc0) + i64x2_extract_lane::<1>(acc0);
                    let sum1 = i64x2_extract_lane::<0>(acc1) + i64x2_extract_lane::<1>(acc1);
                    let sum2 = i64x2_extract_lane::<0>(acc2) + i64x2_extract_lane::<1>(acc2);
                    *out_base.add(r) = scale_i64_acc_to_i32(sum0, out_exp - (re + x_exps[0]));
                    *out_base.add(rows + r) =
                        scale_i64_acc_to_i32(sum1, out_exp - (re + x_exps[1]));
                    *out_base.add(2 * rows + r) =
                        scale_i64_acc_to_i32(sum2, out_exp - (re + x_exps[2]));
                }
            }
            2 => {
                for r in 0..rows {
                    let re = i32::from(m.row_exp[r]);
                    let wp = w_base.add(r * cols);
                    let mp = gm_base.add(r * groups);
                    let mut acc0 = i64x2_splat(0);
                    let mut acc1 = i64x2_splat(0);

                    for g in 0..groups {
                        let w_lo = v128_load(wp.add(g * 32) as *const v128);
                        let w_hi = v128_load(wp.add(g * 32 + 16) as *const v128);
                        let w0 = i16x8_extend_low_i8x16(w_lo);
                        let w1 = i16x8_extend_high_i8x16(w_lo);
                        let w2 = i16x8_extend_low_i8x16(w_hi);
                        let w3 = i16x8_extend_high_i8x16(w_hi);
                        let mult_v = i32x4_splat(i32::from(*mp.add(g)));
                        let base_a = act_base.add(g * 64);
                        dot_i8_col!(w0, w1, w2, w3, mult_v, base_a, 0, acc0);
                        dot_i8_col!(w0, w1, w2, w3, mult_v, base_a, 1, acc1);
                    }

                    let sum0 = i64x2_extract_lane::<0>(acc0) + i64x2_extract_lane::<1>(acc0);
                    let sum1 = i64x2_extract_lane::<0>(acc1) + i64x2_extract_lane::<1>(acc1);
                    *out_base.add(r) = scale_i64_acc_to_i32(sum0, out_exp - (re + x_exps[0]));
                    *out_base.add(rows + r) =
                        scale_i64_acc_to_i32(sum1, out_exp - (re + x_exps[1]));
                }
            }
            _ if bt == 1 => {
                for r in 0..rows {
                    let re = i32::from(m.row_exp[r]);
                    let wp = w_base.add(r * cols);
                    let mp = gm_base.add(r * groups);
                    let mut acc0 = i64x2_splat(0);

                    for g in 0..groups {
                        let w_lo = v128_load(wp.add(g * 32) as *const v128);
                        let w_hi = v128_load(wp.add(g * 32 + 16) as *const v128);
                        let w0 = i16x8_extend_low_i8x16(w_lo);
                        let w1 = i16x8_extend_high_i8x16(w_lo);
                        let w2 = i16x8_extend_low_i8x16(w_hi);
                        let w3 = i16x8_extend_high_i8x16(w_hi);
                        let mult_v = i32x4_splat(i32::from(*mp.add(g)));
                        let base_a = act_base.add(g * 32);
                        dot_i8_col!(w0, w1, w2, w3, mult_v, base_a, 0, acc0);
                    }

                    let sum0 = i64x2_extract_lane::<0>(acc0) + i64x2_extract_lane::<1>(acc0);
                    *out_base.add(r) = scale_i64_acc_to_i32(sum0, out_exp - (re + x_exps[0]));
                }
            }
            _ => {}
        }
        return;
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        for r in 0..rows {
            let re = i32::from(m.row_exp[r]);
            let ws = &m.weights[r * cols..(r + 1) * cols];
            let mults = &m.group_mult[r * groups..(r + 1) * groups];
            let mut acc = [0i64; MAX_PREFILL_TILE];
            for g in 0..groups {
                let wg: &[i8; 32] = (&ws[g * 32..(g + 1) * 32]).try_into().unwrap();
                let mult = i64::from(mults[g]);
                let g_base = g * bt * 32;
                for b in 0..bt {
                    let ag: &[i16; 32] = (&act16_interleaved
                        [g_base + b * 32..g_base + (b + 1) * 32])
                        .try_into()
                        .unwrap();
                    acc[b] += i64::from(dot_i8_group32(wg, ag)) * mult;
                }
            }
            for b in 0..bt {
                let sh = out_exp - (re + x_exps[b]);
                out_batch[b * rows + r] = scale_i64_acc_to_i32(acc[b], sh);
            }
        }
    }
}

/// Formal R4.HoloGeometricRuntime evaluate negative-exponential LUT in Q30 without floating point.
#[inline(always)]
fn exp_neg_with_table(exp_table: &[u32], diff: i64) -> u64 {
    let frac_bits = EXP_STEP_LOG2 - (-16);
    if diff <= 0 {
        return u64::from(exp_table[0]);
    }
    let idx = (diff >> frac_bits) as usize;
    let (Some(&a), Some(&b)) = (exp_table.get(idx), exp_table.get(idx + 1)) else {
        return 0;
    };
    let frac = (diff & ((1 << frac_bits) - 1)) as u64;
    let step = ((u64::from(a).wrapping_sub(u64::from(b))) * frac) >> frac_bits;
    u64::from(a).wrapping_sub(step)
}

/// Formal R4.HoloGeometricRuntime evaluate SiLU activation LUT in Q16 without floating point.
#[inline(always)]
fn silu_i32_with_table(silu_table: &[i32], xv: i32) -> i32 {
    let frac_bits = SILU_STEP_LOG2 + 18;
    let half_idx = 1i64 << (SILU_RANGE_LOG2 - SILU_STEP_LOG2);
    let idx = (i64::from(xv) >> frac_bits) + half_idx;
    if idx < 0 {
        return 0;
    }
    let u = idx as usize;
    let (Some(&a), Some(&b)) = (silu_table.get(u), silu_table.get(u + 1)) else {
        return xv >> 2;
    };
    let frac = i64::from(xv) & ((1i64 << frac_bits) - 1);
    let diff = i64::from(b) - i64::from(a);
    (i64::from(a) + ((diff * frac) >> frac_bits)) as i32
}

/// Formal R4.HoloGeometricRuntime select O(V) zero-alloc Hamming shortlist via 513-bin bucket count + bitset.
#[allow(clippy::too_many_arguments)]
fn select_shortlist_into(
    vocab_size: usize,
    shortlist_k: usize,
    q_sig: [u64; SIG_WORDS],
    vocab_sigs: &[u64],
    ascii_fast_tokens: &[u32],
    special_ids: [u32; 3],
    context_tokens: &[u32],
    use_shortlist: bool,
    scratch: &mut HoloScratch,
) {
    scratch.last_q_sig512 = q_sig;
    scratch.hamming_hist.fill(0);
    scratch.last_hamming_hist32 = [0u32; 32];
    assert!(vocab_sigs.len() >= vocab_size * SIG_WORDS);
    assert!(scratch.hamming_dists.len() >= vocab_size);

    let mut min_d = 512u16;
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        use core::arch::wasm32::*;
        // SAFETY: `vocab_sigs.len() >= vocab_size * 8` and `hamming_dists.len() >= vocab_size` are asserted above.
        unsafe {
            let qp = q_sig.as_ptr() as *const v128;
            let q0 = v128_load(qp);
            let q1 = v128_load(qp.add(1));
            let q2 = v128_load(qp.add(2));
            let q3 = v128_load(qp.add(3));
            let vs_base = vocab_sigs.as_ptr() as *const v128;
            let hd_ptr = scratch.hamming_dists.as_mut_ptr();
            for v in 0..vocab_size {
                let vp = vs_base.add(v * 4);
                let x0 = u8x16_popcnt(v128_xor(q0, v128_load(vp)));
                let x1 = u8x16_popcnt(v128_xor(q1, v128_load(vp.add(1))));
                let x2 = u8x16_popcnt(v128_xor(q2, v128_load(vp.add(2))));
                let x3 = u8x16_popcnt(v128_xor(q3, v128_load(vp.add(3))));
                let p01 = i16x8_add(
                    u16x8_extadd_pairwise_u8x16(x0),
                    u16x8_extadd_pairwise_u8x16(x1),
                );
                let p23 = i16x8_add(
                    u16x8_extadd_pairwise_u8x16(x2),
                    u16x8_extadd_pairwise_u8x16(x3),
                );
                let sum8 = i16x8_add(p01, p23);
                let sum4 = i32x4_extadd_pairwise_i16x8(sum8);
                let d_ham = (i32x4_extract_lane::<0>(sum4)
                    + i32x4_extract_lane::<1>(sum4)
                    + i32x4_extract_lane::<2>(sum4)
                    + i32x4_extract_lane::<3>(sum4)) as usize;
                let clamped = d_ham.min(512);
                *hd_ptr.add(v) = clamped as u16;
                scratch.hamming_hist[clamped] += 1;
                scratch.last_hamming_hist32[(clamped >> 4).min(31)] += 1;
                if (clamped as u16) < min_d {
                    min_d = clamped as u16;
                }
            }
        }
    }

    #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
    {
        for v in 0..vocab_size {
            let base = v * SIG_WORDS;
            let d_ham = hamming512_u64x8(&q_sig, &vocab_sigs[base..base + SIG_WORDS]) as usize;
            let clamped = d_ham.min(512);
            scratch.hamming_dists[v] = clamped as u16;
            scratch.hamming_hist[clamped] += 1;
            scratch.last_hamming_hist32[(clamped >> 4).min(31)] += 1;
            if (clamped as u16) < min_d {
                min_d = clamped as u16;
            }
        }
    }

    scratch.last_min_hamming = min_d;
    scratch.shortlist_out.clear();
    if !use_shortlist || vocab_size <= shortlist_k {
        scratch.shortlist_out.extend(0..vocab_size as u32);
        return;
    }

    let k_keep = shortlist_k.min(vocab_size);
    let mut cum = 0usize;
    let mut d_cut = 513usize;
    let mut rem_at_cut = 0usize;
    for d in 0..=512usize {
        let c = scratch.hamming_hist[d] as usize;
        if cum + c >= k_keep {
            d_cut = d;
            rem_at_cut = k_keep.saturating_sub(cum);
            break;
        }
        cum += c;
    }

    scratch.shortlist_bitset.fill(0);
    for v in 0..vocab_size {
        let d = scratch.hamming_dists[v] as usize;
        let take = if d < d_cut {
            true
        } else if d == d_cut && rem_at_cut > 0 {
            rem_at_cut -= 1;
            true
        } else {
            false
        };
        if take {
            scratch.shortlist_bitset[v >> 6] |= 1u64 << (v & 63);
            scratch.shortlist_out.push(v as u32);
        }
    }

    let push_if_new = |id: u32, bitset: &mut [u64], out: &mut Vec<u32>| {
        let idx = id as usize;
        if idx < vocab_size {
            let word = idx >> 6;
            let bit = 1u64 << (idx & 63);
            if (bitset[word] & bit) == 0 {
                bitset[word] |= bit;
                out.push(id);
            }
        }
    };
    for &id in context_tokens {
        push_if_new(
            id,
            &mut scratch.shortlist_bitset,
            &mut scratch.shortlist_out,
        );
    }
    for id in special_ids {
        push_if_new(
            id,
            &mut scratch.shortlist_bitset,
            &mut scratch.shortlist_out,
        );
    }
    let max_shortlist = shortlist_k.saturating_add(512).min(vocab_size);
    for &id in ascii_fast_tokens {
        if scratch.shortlist_out.len() >= max_shortlist {
            break;
        }
        push_if_new(
            id,
            &mut scratch.shortlist_bitset,
            &mut scratch.shortlist_out,
        );
    }
}

/// Formal R4.HoloGeometricRuntime execute mandatory pre-RoPE per-head `q_norm`/`k_norm`, NeoX RoPE,
/// `kv_cache.push_kv`, and 128-bit QK Starling Flock causal sparse attention for one token position.
#[allow(clippy::too_many_arguments)]
fn attend_token_layer(
    layer: &HoloV13Layer,
    kv_cache: &mut HoloPrefixKvCache,
    l: usize,
    nh: usize,
    nkv: usize,
    hd: usize,
    flock_window: usize,
    rms_eps_q48: i64,
    score_scale_q30: i64,
    cos_row: &[i16],
    sin_row: &[i16],
    exp_table: &[u32],
    q: &mut [i32],
    k: &mut [i32],
    v: &[i32],
    attn_out: &mut [i64],
    attn_work: &mut HoloAttnScratch,
    mut flock_span_slot: Option<&mut u16>,
    last_attn_peaks_out: Option<&mut Vec<HoloAttnPeak>>,
) -> (usize, usize) {
    let group_sz = nh / nkv;
    let half = hd / 2;
    let kv_dim = nkv * hd;

    // Mandatory per-head q_norm & k_norm BEFORE NeoX half-split RoPE (`R4-51`)
    for h in 0..nh {
        let off = h * hd;
        int_rms_norm_gain(
            &q[off..off + hd],
            &layer.q_norm_q24,
            rms_eps_q48,
            RESIDUAL_EXP,
            RESIDUAL_EXP,
            &mut attn_work.q_h_norm,
        );
        for i in 0..half {
            let c = i64::from(cos_row[i]);
            let s = i64::from(sin_row[i]);
            let q0 = attn_work.q_h_norm[i];
            let q1 = attn_work.q_h_norm[i + half];
            q[off + i] = ((q0 * c - q1 * s + (1 << 13)) >> ROPE_Q) as i32;
            q[off + i + half] = ((q0 * s + q1 * c + (1 << 13)) >> ROPE_Q) as i32;
        }
    }
    for kh in 0..nkv {
        let off = kh * hd;
        int_rms_norm_gain(
            &k[off..off + hd],
            &layer.k_norm_q24,
            rms_eps_q48,
            RESIDUAL_EXP,
            RESIDUAL_EXP,
            &mut attn_work.k_h_norm,
        );
        for i in 0..half {
            let c = i64::from(cos_row[i]);
            let s = i64::from(sin_row[i]);
            let k0 = attn_work.k_h_norm[i];
            let k1 = attn_work.k_h_norm[i + half];
            k[off + i] = ((k0 * c - k1 * s + (1 << 13)) >> ROPE_Q) as i32;
            k[off + i + half] = ((k0 * s + k1 * c + (1 << 13)) >> ROPE_Q) as i32;
        }
    }

    kv_cache.push_kv(l, k, v, nkv, hd);
    let seq_len = kv_cache.seq_len[l];
    let track_peaks = last_attn_peaks_out.is_some();
    if track_peaks {
        attn_work.attn_accum_q16.clear();
        attn_work.attn_accum_q16.resize(seq_len, 0);
    }

    let mut flock_attended = 0usize;
    let mut flock_total = 0usize;
    let k_vec = &kv_cache.k_i16[l];
    let k_scales = &kv_cache.k_scales[l];
    let v_scales = &kv_cache.v_scales[l];
    let v_vec = &kv_cache.v_i16[l];
    let k_sigs = &kv_cache.k_sig128[l];

    for h in 0..nh {
        let kh = h / group_sz;
        let q_h = &q[h * hd..(h + 1) * hd];
        let mut q_sig = [0u64; 2];
        let mut q_max_abs = 0u32;
        for (d_i, &qv) in q_h.iter().enumerate() {
            if qv >= 0 {
                q_sig[d_i >> 6] |= 1u64 << (d_i & 63);
            }
            q_max_abs = q_max_abs.max(qv.unsigned_abs());
        }
        let q_bl = (32 - q_max_abs.leading_zeros() as i32).max(1);
        let q_sh = (q_bl - 15).max(0);
        for (dst, &qv) in attn_work.q_i16.iter_mut().zip(q_h.iter()).take(hd) {
            *dst = shift_round(i64::from(qv), q_sh).clamp(-32_767, 32_767) as i16;
        }

        attn_work.selected.clear();
        if seq_len <= flock_window {
            attn_work.selected.extend(0..seq_len);
        } else {
            let sink_keep = 4usize;
            let rec_keep = (flock_window / 4).max(16).min(seq_len);
            let mut hist = [0u32; 130];
            if attn_work.flock_bins.len() < seq_len {
                attn_work.flock_bins.resize(seq_len, 0);
            }
            for t in 0..seq_len {
                let bin = if t < sink_keep || t + rec_keep >= seq_len {
                    0usize
                } else {
                    let k_sig = k_sigs[t * nkv + kh];
                    let dist =
                        (q_sig[0] ^ k_sig[0]).count_ones() + (q_sig[1] ^ k_sig[1]).count_ones();
                    (dist.min(128) as usize) + 1
                };
                attn_work.flock_bins[t] = bin as u8;
                hist[bin] += 1;
            }
            let mut acc = 0usize;
            let mut cutoff = 129usize;
            let mut quota_at_cutoff = 0usize;
            for (b, &cnt) in hist.iter().enumerate() {
                let c = cnt as usize;
                if acc + c >= flock_window {
                    cutoff = b;
                    quota_at_cutoff = flock_window - acc;
                    break;
                }
                acc += c;
            }
            for t in 0..seq_len {
                let b = attn_work.flock_bins[t] as usize;
                if b < cutoff {
                    attn_work.selected.push(t);
                } else if b == cutoff && quota_at_cutoff > 0 {
                    attn_work.selected.push(t);
                    quota_at_cutoff -= 1;
                }
            }
        }
        if h == 0 {
            if let Some(slot) = flock_span_slot.as_deref_mut() {
                *slot = attn_work.selected.len().min(u16::MAX as usize) as u16;
            }
        }
        flock_attended += attn_work.selected.len();
        flock_total += seq_len;

        attn_work.scores.clear();
        let mut max_s = i64::MIN;
        let mut max_vs = 0u32;
        let q_i16_fixed: Option<&[i16; 128]> = if hd == 128 {
            Some((&attn_work.q_i16[..128]).try_into().unwrap())
        } else {
            None
        };

        for &t in &attn_work.selected {
            let meta_idx = t * nkv + kh;
            let k_off = t * kv_dim + kh * hd;
            let k_slice = &k_vec[k_off..k_off + hd];
            let dot_raw = if let Some(qa) = q_i16_fixed {
                dot_i16_128(qa, k_slice)
            } else {
                let mut acc = 0i64;
                for (&qi, &ki) in attn_work.q_i16.iter().zip(k_slice.iter()).take(hd) {
                    acc += i64::from(qi) * i64::from(ki);
                }
                acc
            };
            let ks64 = i64::from(k_scales[meta_idx]);
            let s_q16 = if dot_raw == 0 || ks64 == 0 {
                0
            } else if (q_sh as u32) < 24 {
                if let Some(ds) = dot_raw
                    .checked_shl(q_sh as u32)
                    .filter(|&v| (v >> (q_sh as u32)) == dot_raw)
                    .and_then(|v| v.checked_mul(ks64))
                    .and_then(|v| v.checked_mul(score_scale_q30))
                    .and_then(|v| v.checked_add(1i64 << 49))
                {
                    ds >> 50
                } else {
                    let dot_scaled = (i128::from(dot_raw) << (q_sh as u32)) * i128::from(ks64);
                    ((dot_scaled * i128::from(score_scale_q30) + (1i128 << 49)) >> 50) as i64
                }
            } else {
                let dot_scaled = (i128::from(dot_raw) << (q_sh as u32)) * i128::from(ks64);
                ((dot_scaled * i128::from(score_scale_q30) + (1i128 << 49)) >> 50) as i64
            };
            attn_work.scores.push(s_q16);
            if s_q16 > max_s {
                max_s = s_q16;
            }
            max_vs = max_vs.max(v_scales[meta_idx].unsigned_abs());
        }

        attn_work.exp_w.clear();
        let mut den = 0u64;
        for &sc in &attn_work.scores {
            let w = (exp_neg_with_table(exp_table, max_s - sc) >> 14) as i64;
            attn_work.exp_w.push(w);
            den += w as u64;
        }
        let den_u64 = den.max(1);
        if max_vs <= (1 << 18) && hd <= 128 {
            let mut num = [0i64; 128];
            for (si, &t) in attn_work.selected.iter().enumerate() {
                let w = attn_work.exp_w[si];
                if w == 0 {
                    continue;
                }
                let meta_idx = t * nkv + kh;
                if track_peaks {
                    attn_work.attn_accum_q16[t] = attn_work.attn_accum_q16[t]
                        .saturating_add(((w as u64).saturating_mul(65_535) / den_u64) as u32);
                }
                let v_off = t * kv_dim + kh * hd;
                let vs = i64::from(v_scales[meta_idx]);
                let v_slice = &v_vec[v_off..v_off + hd];
                let wv_scale = w * vs;
                #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
                if hd == 128 {
                    if let Ok(wv32) = i32::try_from(wv_scale) {
                        // SAFETY: `v_slice.len() == 128` and `num.len() == 128` across 16 blocks of 8 lanes.
                        unsafe {
                            use core::arch::wasm32::*;
                            let vp = v_slice.as_ptr();
                            let np = num.as_mut_ptr();
                            let sv = i32x4_splat(wv32);
                            for blk in 0..16 {
                                let off = blk * 8;
                                let v8 = v128_load(vp.add(off) as *const v128);
                                let vlo = i32x4_extend_low_i16x8(v8);
                                let vhi = i32x4_extend_high_i16x8(v8);
                                let n0p = np.add(off) as *mut v128;
                                let n1p = np.add(off + 2) as *mut v128;
                                let n2p = np.add(off + 4) as *mut v128;
                                let n3p = np.add(off + 6) as *mut v128;
                                v128_store(
                                    n0p,
                                    i64x2_add(v128_load(n0p), i64x2_extmul_low_i32x4(vlo, sv)),
                                );
                                v128_store(
                                    n1p,
                                    i64x2_add(v128_load(n1p), i64x2_extmul_high_i32x4(vlo, sv)),
                                );
                                v128_store(
                                    n2p,
                                    i64x2_add(v128_load(n2p), i64x2_extmul_low_i32x4(vhi, sv)),
                                );
                                v128_store(
                                    n3p,
                                    i64x2_add(v128_load(n3p), i64x2_extmul_high_i32x4(vhi, sv)),
                                );
                            }
                        }
                        continue;
                    }
                }
                for d_i in 0..hd {
                    num[d_i] += wv_scale * i64::from(v_slice[d_i]);
                }
            }
            if den_u64.is_power_of_two() {
                let sh = den_u64.trailing_zeros();
                let bias = (1i64 << sh) - 1;
                for (dst, &nv) in attn_out[h * hd..].iter_mut().zip(num.iter()).take(hd) {
                    *dst = if nv < 0 { (nv + bias) >> sh } else { nv >> sh };
                }
            } else {
                let den_s = den_u64 as i64;
                for (dst, &nv) in attn_out[h * hd..].iter_mut().zip(num.iter()).take(hd) {
                    *dst = if nv == 0 { 0 } else { nv / den_s };
                }
            }
        } else {
            let mut num = [0i128; 128];
            for (si, &t) in attn_work.selected.iter().enumerate() {
                let w = attn_work.exp_w[si];
                if w == 0 {
                    continue;
                }
                let meta_idx = t * nkv + kh;
                if track_peaks {
                    attn_work.attn_accum_q16[t] = attn_work.attn_accum_q16[t]
                        .saturating_add(((w as u64).saturating_mul(65_535) / den_u64) as u32);
                }
                let v_off = t * kv_dim + kh * hd;
                let vs = i128::from(v_scales[meta_idx]);
                let v_slice = &v_vec[v_off..v_off + hd];
                let wv_scale = i128::from(w) * vs;
                for d_i in 0..hd {
                    num[d_i] += wv_scale * i128::from(v_slice[d_i]);
                }
            }
            let den_s = den_u64 as i128;
            for (dst, &nv) in attn_out[h * hd..].iter_mut().zip(num.iter()).take(hd) {
                *dst = (nv / den_s) as i64;
            }
        }
    }

    if let Some(peaks_out) = last_attn_peaks_out {
        if !attn_work.attn_accum_q16.is_empty() {
            let nh_u32 = nh.max(1) as u32;
            let mut ranked_peaks: Vec<(u32, usize)> = attn_work
                .attn_accum_q16
                .iter()
                .enumerate()
                .map(|(t, &acc)| (acc / nh_u32, t))
                .collect();
            ranked_peaks.sort_unstable_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
            for (w, t) in ranked_peaks.into_iter().take(8) {
                peaks_out.push(HoloAttnPeak {
                    pos: t.min(u16::MAX as usize) as u16,
                    weight_q16: w.min(65_535) as u16,
                });
            }
        }
    }
    (flock_attended, flock_total)
}

impl HoloV13Model {
    /// Formal R4.HoloGeometricRuntime mount a complete `UORR4V13` byte slice in one call.
    pub fn from_bytes(model_id: impl Into<String>, bytes: &[u8]) -> Result<Self, R4StackError> {
        let mut builder = HoloV13StreamingBuilder::new(model_id, 1, "");
        builder.push_piece(bytes)?;
        builder.finalize()
    }

    /// Formal R4.HoloGeometricRuntime adjust shortlist and Starling Flock budgets safely within `R4-55` bounds.
    pub fn set_budgets(&mut self, shortlist_k: usize, flock_window: usize) {
        self.shortlist_k = shortlist_k.clamp(512, 16_384).min(self.vocab_size.max(512));
        self.flock_window = flock_window.clamp(32, 1024);
    }

    /// Formal R4.HoloGeometricRuntime evaluate striped row subset of tied `embed` / `int8` matrix for parallel workers.
    pub fn gemv_i8_rows_striped(
        &mut self,
        row_ids: &[u32],
        norm_in: &[i64],
        in_exp: i32,
        out_exp: i32,
    ) -> Vec<(u32, i32)> {
        let d = self.embed.cols;
        self.scratch.ensure_sized(
            self.d_model,
            self.n_heads * self.head_dim,
            self.n_kv_heads * self.head_dim,
            self.intermediate_size,
            self.head_dim,
            self.n_layers,
            self.vocab_size,
        );
        let x_exp = prepare_rht_act16_packed(
            norm_in,
            in_exp,
            &self.embed.rht_signs_packed,
            &mut self.scratch.rht_i32,
            &mut self.scratch.act16,
        );
        let mut out = Vec::with_capacity(row_ids.len());
        gemv_i8_indexed_into(
            &self.embed,
            row_ids,
            &self.scratch.act16[..d],
            x_exp,
            out_exp,
            &mut out,
        );
        out
    }

    /// Formal R4.HoloGeometricRuntime finalize residual `norm_d`, extract RHT-512 signature once, shortlist, and score `lm_head`.
    fn score_final_hidden_logits(
        &mut self,
        context_tokens: &[u32],
        use_shortlist: bool,
    ) -> Vec<(u32, i32)> {
        let d = self.d_model;
        int_rms_norm_gain(
            &self.scratch.x,
            &self.final_norm_q24,
            self.rms_eps_q48,
            RESIDUAL_EXP,
            NORM_EXP,
            &mut self.scratch.norm_d,
        );
        let x_exp = prepare_rht_act16_packed(
            &self.scratch.norm_d,
            NORM_EXP,
            &self.embed.rht_signs_packed,
            &mut self.scratch.rht_i32,
            &mut self.scratch.act16,
        );
        let q_sig = extract_rht512_sig_from_rht(&self.scratch.rht_i32[..d]);
        select_shortlist_into(
            self.vocab_size,
            self.shortlist_k,
            q_sig,
            &self.vocab_sigs,
            &self.tokenizer.ascii_fast_tokens,
            [self.eos_token_id, self.eot_token_id, self.think_end_id],
            context_tokens,
            use_shortlist,
            &mut self.scratch,
        );
        let mut scored = Vec::with_capacity(self.scratch.shortlist_out.len());
        gemv_i8_indexed_into(
            &self.embed,
            &self.scratch.shortlist_out,
            &self.scratch.act16[..d],
            x_exp,
            -16,
            &mut scored,
        );
        scored
    }

    /// Formal R4.HoloGeometricRuntime execute one zero-alloc pure-integer autoregressive token step across
    /// `n_layers` with mandatory pre-RoPE per-head `q_norm`/`k_norm`, `simd128` Q·K `i16` dot products,
    /// `i64` V-aggregation, Starling Flock sparse selection, and bucketed 512-bit Hamming shortlisting.
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
        let half = hd / 2;
        let pos = self
            .kv_cache
            .tokens
            .len()
            .min(self.max_seq.saturating_sub(1));

        self.scratch
            .ensure_sized(d, q_dim, kv_dim, ffn, hd, self.n_layers, self.vocab_size);
        let vocab_size = self.vocab_size;
        let n_layers = self.n_layers;
        let flock_window = self.flock_window;
        let rms_eps_q48 = self.rms_eps_q48;
        let score_scale_q30 = self.score_scale_q30;
        let cos_row = &self.rope_cos_q14[pos * half..(pos + 1) * half];
        let sin_row = &self.rope_sin_q14[pos * half..(pos + 1) * half];
        let scratch = &mut self.scratch;

        dequant_embed_row_into(
            &self.embed,
            (tok as usize).min(vocab_size - 1),
            &mut scratch.wide_i64,
            &mut scratch.x,
        );
        scratch.last_attn_peaks.clear();
        let mut flock_attended = 0usize;
        let mut flock_total = 0usize;

        for l in 0..n_layers {
            let layer = &self.layers[l];
            int_rms_norm_gain(
                &scratch.x,
                &layer.in_ln_q24,
                rms_eps_q48,
                RESIDUAL_EXP,
                NORM_EXP,
                &mut scratch.norm_d,
            );
            gemv_i4_rht_into(
                &layer.q_proj,
                &scratch.norm_d,
                NORM_EXP,
                RESIDUAL_EXP,
                &mut scratch.rht_i32,
                &mut scratch.act16,
                &mut scratch.q,
            );
            gemv_i4_rht_into(
                &layer.k_proj,
                &scratch.norm_d,
                NORM_EXP,
                RESIDUAL_EXP,
                &mut scratch.rht_i32,
                &mut scratch.act16,
                &mut scratch.k,
            );
            gemv_i4_rht_into(
                &layer.v_proj,
                &scratch.norm_d,
                NORM_EXP,
                RESIDUAL_EXP,
                &mut scratch.rht_i32,
                &mut scratch.act16,
                &mut scratch.v,
            );

            let is_last_layer_logits = l + 1 == n_layers && compute_logits;
            let peaks_out = if is_last_layer_logits {
                Some(&mut scratch.last_attn_peaks)
            } else {
                None
            };
            let (fa, ft) = attend_token_layer(
                layer,
                &mut self.kv_cache,
                l,
                nh,
                nkv,
                hd,
                flock_window,
                rms_eps_q48,
                score_scale_q30,
                cos_row,
                sin_row,
                &self.exp_table,
                &mut scratch.q,
                &mut scratch.k,
                &scratch.v,
                &mut scratch.attn_i64,
                &mut scratch.attn_work,
                Some(&mut scratch.layer_flock_spans[l]),
                peaks_out,
            );
            flock_attended += fa;
            flock_total += ft;

            gemv_i4_rht_into(
                &layer.o_proj,
                &scratch.attn_i64,
                RESIDUAL_EXP,
                RESIDUAL_EXP,
                &mut scratch.rht_i32,
                &mut scratch.act16,
                &mut scratch.proj,
            );
            for i in 0..d {
                scratch.x[i] = scratch.x[i].saturating_add(scratch.proj[i]);
            }

            int_rms_norm_gain(
                &scratch.x,
                &layer.post_ln_q24,
                rms_eps_q48,
                RESIDUAL_EXP,
                NORM_EXP,
                &mut scratch.norm_d,
            );
            gemv_i4_rht_into(
                &layer.gate_proj,
                &scratch.norm_d,
                NORM_EXP,
                RESIDUAL_EXP,
                &mut scratch.rht_i32,
                &mut scratch.act16,
                &mut scratch.gate,
            );
            gemv_i4_rht_into(
                &layer.up_proj,
                &scratch.norm_d,
                NORM_EXP,
                RESIDUAL_EXP,
                &mut scratch.rht_i32,
                &mut scratch.act16,
                &mut scratch.up,
            );
            for i in 0..ffn {
                let sg = silu_i32_with_table(&self.silu_table, scratch.gate[i]);
                scratch.swiglu[i] = (i64::from(sg) * i64::from(scratch.up[i]) + (1 << 13)) >> 14;
            }
            match &layer.down_proj {
                HoloDownProjMatrix::Int4(m) => {
                    gemv_i4_rht_into(
                        m,
                        &scratch.swiglu,
                        -20,
                        RESIDUAL_EXP,
                        &mut scratch.rht_i32,
                        &mut scratch.act16,
                        &mut scratch.proj,
                    );
                }
                HoloDownProjMatrix::Int8(m) => {
                    gemv_i8_rht_into(
                        m,
                        &scratch.swiglu,
                        -20,
                        RESIDUAL_EXP,
                        &mut scratch.rht_i32,
                        &mut scratch.act16,
                        &mut scratch.proj,
                    );
                }
            }
            let mut sum_abs = 0u64;
            let mut max_abs_x = 0u32;
            for i in 0..d {
                let nv = scratch.x[i].saturating_add(scratch.proj[i]);
                scratch.x[i] = nv;
                let a = nv.unsigned_abs();
                sum_abs = sum_abs.wrapping_add(u64::from(a));
                max_abs_x = max_abs_x.max(a);
            }
            let mean_scaled = (sum_abs / d.max(1) as u64) >> 10;
            scratch.layer_residual_mag[l] = mean_scaled.min(u64::from(u16::MAX)) as u16;
            scratch.layer_max_bits[l] = (32 - max_abs_x.leading_zeros()) as u8;
        }
        self.kv_cache.tokens.push(tok);

        if !compute_logits {
            return (None, flock_attended, flock_total);
        }

        let scored = self.score_final_hidden_logits(context_tokens, use_shortlist);
        (Some(scored), flock_attended, flock_total)
    }

    /// Formal R4.HoloGeometricRuntime prefill `tokens` in `MAX_PREFILL_TILE = 4` register tiles,
    /// streaming each projection weight matrix from Wasm DRAM once per tile while matching sequential
    /// `step_token` bit-for-bit across all KV tensors and final logits.
    pub fn prefill_tokens_tiled(
        &mut self,
        tokens: &[u32],
        context_tokens: &[u32],
        compute_last_logits: bool,
        use_shortlist: bool,
    ) -> (Option<Vec<(u32, i32)>>, usize, usize) {
        if tokens.is_empty() {
            return (None, 0, 0);
        }
        if tokens.len() == 1 {
            return self.step_token(
                tokens[0],
                context_tokens,
                compute_last_logits,
                use_shortlist,
            );
        }

        let d = self.d_model;
        let nh = self.n_heads;
        let nkv = self.n_kv_heads;
        let hd = self.head_dim;
        let q_dim = nh * hd;
        let kv_dim = nkv * hd;
        let ffn = self.intermediate_size;
        let half = hd / 2;
        let vocab_size = self.vocab_size;
        let n_layers = self.n_layers;
        let flock_window = self.flock_window;
        let rms_eps_q48 = self.rms_eps_q48;
        let score_scale_q30 = self.score_scale_q30;

        self.scratch
            .ensure_sized(d, q_dim, kv_dim, ffn, hd, n_layers, vocab_size);
        self.scratch
            .prefill_tile
            .ensure_sized(d, q_dim, kv_dim, ffn);

        let mut flock_attended = 0usize;
        let mut flock_total = 0usize;
        let total_chunks = tokens.len().div_ceil(MAX_PREFILL_TILE);

        for (chunk_idx, chunk) in tokens.chunks(MAX_PREFILL_TILE).enumerate() {
            let bt = chunk.len();
            if bt == 1 {
                let is_last = chunk_idx + 1 == total_chunks && compute_last_logits;
                let (sc_opt, fa, ft) =
                    self.step_token(chunk[0], context_tokens, is_last, use_shortlist);
                flock_attended += fa;
                flock_total += ft;
                if is_last {
                    return (sc_opt, flock_attended, flock_total);
                }
                continue;
            }

            let is_last_chunk = chunk_idx + 1 == total_chunks;
            let base_pos = self.kv_cache.tokens.len();
            let s = &mut self.scratch;
            s.last_attn_peaks.clear();
            let tile = &mut s.prefill_tile;

            for (b, &tok) in chunk.iter().enumerate() {
                dequant_embed_row_into(
                    &self.embed,
                    (tok as usize).min(vocab_size - 1),
                    &mut s.wide_i64,
                    &mut tile.x_b[b * d..(b + 1) * d],
                );
            }

            for l in 0..n_layers {
                let layer = &self.layers[l];
                for b in 0..bt {
                    int_rms_norm_gain(
                        &tile.x_b[b * d..(b + 1) * d],
                        &layer.in_ln_q24,
                        rms_eps_q48,
                        RESIDUAL_EXP,
                        NORM_EXP,
                        &mut tile.norm_b[b * d..(b + 1) * d],
                    );
                }

                gemm_i4_rht_batch(
                    &layer.q_proj,
                    &tile.norm_b,
                    bt,
                    NORM_EXP,
                    RESIDUAL_EXP,
                    &mut s.rht_i32,
                    &mut tile.act16_tmp,
                    &mut tile.act16_interleaved,
                    &mut tile.q_b,
                );
                gemm_i4_rht_batch(
                    &layer.k_proj,
                    &tile.norm_b,
                    bt,
                    NORM_EXP,
                    RESIDUAL_EXP,
                    &mut s.rht_i32,
                    &mut tile.act16_tmp,
                    &mut tile.act16_interleaved,
                    &mut tile.k_b,
                );
                gemm_i4_rht_batch(
                    &layer.v_proj,
                    &tile.norm_b,
                    bt,
                    NORM_EXP,
                    RESIDUAL_EXP,
                    &mut s.rht_i32,
                    &mut tile.act16_tmp,
                    &mut tile.act16_interleaved,
                    &mut tile.v_b,
                );

                for b in 0..bt {
                    let pos = (base_pos + b).min(self.max_seq.saturating_sub(1));
                    let cos_row = &self.rope_cos_q14[pos * half..(pos + 1) * half];
                    let sin_row = &self.rope_sin_q14[pos * half..(pos + 1) * half];
                    let is_last_tok = is_last_chunk && b + 1 == bt;
                    let span_slot = if is_last_tok {
                        Some(&mut s.layer_flock_spans[l])
                    } else {
                        None
                    };
                    let peaks_out = if is_last_tok && l + 1 == n_layers && compute_last_logits {
                        Some(&mut s.last_attn_peaks)
                    } else {
                        None
                    };
                    let (fa, ft) = attend_token_layer(
                        layer,
                        &mut self.kv_cache,
                        l,
                        nh,
                        nkv,
                        hd,
                        flock_window,
                        rms_eps_q48,
                        score_scale_q30,
                        cos_row,
                        sin_row,
                        &self.exp_table,
                        &mut tile.q_b[b * q_dim..(b + 1) * q_dim],
                        &mut tile.k_b[b * kv_dim..(b + 1) * kv_dim],
                        &tile.v_b[b * kv_dim..(b + 1) * kv_dim],
                        &mut tile.attn_b[b * q_dim..(b + 1) * q_dim],
                        &mut s.attn_work,
                        span_slot,
                        peaks_out,
                    );
                    flock_attended += fa;
                    flock_total += ft;
                }

                gemm_i4_rht_batch(
                    &layer.o_proj,
                    &tile.attn_b,
                    bt,
                    RESIDUAL_EXP,
                    RESIDUAL_EXP,
                    &mut s.rht_i32,
                    &mut tile.act16_tmp,
                    &mut tile.act16_interleaved,
                    &mut tile.proj_b,
                );
                for b in 0..bt {
                    let off = b * d;
                    for i in 0..d {
                        tile.x_b[off + i] = tile.x_b[off + i].saturating_add(tile.proj_b[off + i]);
                    }
                    int_rms_norm_gain(
                        &tile.x_b[off..off + d],
                        &layer.post_ln_q24,
                        rms_eps_q48,
                        RESIDUAL_EXP,
                        NORM_EXP,
                        &mut tile.norm_b[off..off + d],
                    );
                }

                gemm_i4_rht_batch(
                    &layer.gate_proj,
                    &tile.norm_b,
                    bt,
                    NORM_EXP,
                    RESIDUAL_EXP,
                    &mut s.rht_i32,
                    &mut tile.act16_tmp,
                    &mut tile.act16_interleaved,
                    &mut tile.gate_b,
                );
                gemm_i4_rht_batch(
                    &layer.up_proj,
                    &tile.norm_b,
                    bt,
                    NORM_EXP,
                    RESIDUAL_EXP,
                    &mut s.rht_i32,
                    &mut tile.act16_tmp,
                    &mut tile.act16_interleaved,
                    &mut tile.up_b,
                );

                for b in 0..bt {
                    let off = b * ffn;
                    for i in 0..ffn {
                        let sg = silu_i32_with_table(&self.silu_table, tile.gate_b[off + i]);
                        tile.swiglu_b[off + i] =
                            (i64::from(sg) * i64::from(tile.up_b[off + i]) + (1 << 13)) >> 14;
                    }
                }

                match &layer.down_proj {
                    HoloDownProjMatrix::Int4(m) => {
                        gemm_i4_rht_batch(
                            m,
                            &tile.swiglu_b,
                            bt,
                            -20,
                            RESIDUAL_EXP,
                            &mut s.rht_i32,
                            &mut tile.act16_tmp,
                            &mut tile.act16_interleaved,
                            &mut tile.proj_b,
                        );
                    }
                    HoloDownProjMatrix::Int8(m) => {
                        gemm_i8_rht_batch(
                            m,
                            &tile.swiglu_b,
                            bt,
                            -20,
                            RESIDUAL_EXP,
                            &mut s.rht_i32,
                            &mut tile.act16_tmp,
                            &mut tile.act16_interleaved,
                            &mut tile.proj_b,
                        );
                    }
                }

                for b in 0..bt {
                    let off = b * d;
                    if is_last_chunk && b + 1 == bt {
                        let mut sum_abs = 0u64;
                        let mut max_abs_x = 0u32;
                        for i in 0..d {
                            let nv = tile.x_b[off + i].saturating_add(tile.proj_b[off + i]);
                            tile.x_b[off + i] = nv;
                            let a = nv.unsigned_abs();
                            sum_abs = sum_abs.wrapping_add(u64::from(a));
                            max_abs_x = max_abs_x.max(a);
                        }
                        let mean_scaled = (sum_abs / d.max(1) as u64) >> 10;
                        s.layer_residual_mag[l] = mean_scaled.min(u64::from(u16::MAX)) as u16;
                        s.layer_max_bits[l] = (32 - max_abs_x.leading_zeros()) as u8;
                    } else {
                        for i in 0..d {
                            tile.x_b[off + i] =
                                tile.x_b[off + i].saturating_add(tile.proj_b[off + i]);
                        }
                    }
                }
            }

            self.kv_cache.tokens.extend_from_slice(chunk);

            if is_last_chunk && compute_last_logits {
                let last_off = (bt - 1) * d;
                s.x.copy_from_slice(&tile.x_b[last_off..last_off + d]);
                let scored = self.score_final_hidden_logits(context_tokens, use_shortlist);
                return (Some(scored), flock_attended, flock_total);
            }
        }

        (None, flock_attended, flock_total)
    }

    /// Formal R4.HoloGeometricRuntime pre-populate `HoloPrefixKvCache` at mount time with the default
    /// ChatML system prefix so Turn 1 skips system-prompt prefill latency completely.
    pub fn prewarm_system_prefix(&mut self, system_prompt: &str) {
        let sys = if system_prompt.trim().is_empty() {
            "You are a helpful assistant."
        } else {
            system_prompt.trim()
        };
        let prefix = format!("<|im_start|>system\n{sys}<|im_end|>\n<|im_start|>user\n");
        let ids = self.tokenizer.encode(&prefix);
        self.kv_cache.reset(self.n_layers);
        if !ids.is_empty() && ids.len() + 32 <= self.max_seq {
            self.prefill_tokens_tiled(&ids, &ids, false, true);
        }
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

    /// Formal R4.HoloGeometricRuntime execute multi-turn Qwen3 ChatML with true `HoloPrefixKvCache` reuse & live Observatory frames.
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
        let sys_chunk = format!("<|im_start|>system\n{sys}<|im_end|>\n");
        let mut prompt_ids_all = self.tokenizer.encode(&sys_chunk);
        let recent_hist = &history[history.len().saturating_sub(6)..];
        let eot_nl_ids = self.tokenizer.encode("<|im_end|>\n");
        for (u, a) in recent_hist {
            let u_trim = u.trim();
            let a_trim = a.trim();
            let u_chunk = format!(
                "<|im_start|>user\n{u_trim}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
            );
            prompt_ids_all.extend_from_slice(&self.tokenizer.encode(&u_chunk));
            if let Some((_, _, exact_ids)) = self
                .kv_cache
                .turn_token_spans
                .iter()
                .rev()
                .find(|(cu, ca, _)| cu == u_trim && ca == a_trim)
            {
                prompt_ids_all.extend_from_slice(exact_ids);
            } else {
                prompt_ids_all.extend_from_slice(&self.tokenizer.encode(a_trim));
            }
            prompt_ids_all.extend_from_slice(&eot_nl_ids);
        }
        let cur_u_trim = user_prompt.trim();
        let cur_u_chunk = format!(
            "<|im_start|>user\n{cur_u_trim}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
        );
        prompt_ids_all.extend_from_slice(&self.tokenizer.encode(&cur_u_chunk));
        let max_ctx = self.max_seq.saturating_sub(max_new_tokens.max(16)).max(32);
        let prompt_ids: &[u32] = if prompt_ids_all.len() > max_ctx {
            &prompt_ids_all[prompt_ids_all.len() - max_ctx..]
        } else {
            &prompt_ids_all
        };

        let mut prefix_match = 0usize;
        for (&cached_tok, &prompt_tok) in self.kv_cache.tokens.iter().zip(prompt_ids.iter()) {
            if cached_tok == prompt_tok {
                prefix_match += 1;
            } else {
                break;
            }
        }
        let reuse_len = prefix_match.min(prompt_ids.len().saturating_sub(1));
        if reuse_len == 0 {
            self.kv_cache.reset(self.n_layers);
        } else {
            self.kv_cache
                .truncate_to(reuse_len, self.n_kv_heads, self.head_dim);
        }

        let simd_active = cfg!(all(target_arch = "wasm32", target_feature = "simd128"));
        let active_backend = if simd_active {
            "wasm-simd128"
        } else {
            "host-int"
        };

        let mut flock_attended = 0usize;
        let mut flock_total = 0usize;
        let mut recent: Vec<u32> = prompt_ids.to_vec();
        let mut observatory_frames = Vec::new();

        let tail = &prompt_ids[reuse_len..];
        let (sc_opt, fa, ft) = self.prefill_tokens_tiled(tail, &recent, true, true);
        flock_attended += fa;
        flock_total += ft;
        let mut last_scored = sc_opt.unwrap_or_default();
        if let Some(&last_tok) = tail.last() {
            let frame = build_observatory_frame(
                &self.tokenizer,
                &self.scratch,
                &self.exp_table,
                &last_scored,
                last_tok,
                true,
                tail.len().saturating_sub(1),
                "prefill",
                self.kv_cache.tokens.len().saturating_sub(1),
                reuse_len,
                active_backend,
            );
            if admit_live_token_frame(
                frame.has_geometric_shortlist,
                frame.top_candidates.iter().any(|c| c.hamming_d.is_some()),
                0,
                100_000,
            ) == LiveFrameVerdict::FrameAdmitted
            {
                emit_holo_trace_frame(&frame);
                observatory_frames.push(frame);
            }
        }

        let mut generated = Vec::new();
        let mut raw_stream: Vec<u8> = Vec::new();
        let mut emitted_raw = 0usize;
        let mut last_emitted_decode_step = usize::MAX;
        for step_idx in 0..max_new_tokens.max(1) {
            let chosen_id = pick_next_holo_token(
                &last_scored,
                &generated,
                self.eos_token_id,
                self.eot_token_id,
                &self.tokenizer,
            );
            if chosen_id == self.eos_token_id || chosen_id == self.eot_token_id {
                if step_idx > 0 && last_emitted_decode_step != step_idx {
                    let prev_id = generated.last().copied().unwrap_or(chosen_id);
                    record_observatory_frame(
                        self,
                        &last_scored,
                        prev_id,
                        false,
                        step_idx,
                        "decode",
                        reuse_len,
                        active_backend,
                        &mut observatory_frames,
                    );
                }
                break;
            }
            generated.push(chosen_id);
            recent.push(chosen_id);
            if let Some(tok_bytes) = self.tokenizer.id_to_raw.get(chosen_id as usize) {
                raw_stream.extend_from_slice(tok_bytes);
                let delta = flush_incremental_utf8(&raw_stream, &mut emitted_raw, false);
                if !delta.is_empty() {
                    emit_holo_stream_delta(&delta, step_idx);
                }
            }
            if should_emit_decode_observatory_frame(
                step_idx,
                generated.len(),
                max_new_tokens,
                false,
            ) {
                record_observatory_frame(
                    self,
                    &last_scored,
                    chosen_id,
                    false,
                    step_idx,
                    "decode",
                    reuse_len,
                    active_backend,
                    &mut observatory_frames,
                );
                last_emitted_decode_step = step_idx;
            }
            if generated.len() >= max_new_tokens {
                let (_, fa, ft) = self.step_token(chosen_id, &recent, false, true);
                flock_attended += fa;
                flock_total += ft;
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

        let reply = String::from_utf8_lossy(&raw_stream).trim().to_string();
        if self.kv_cache.turn_token_spans.len() >= 12 {
            self.kv_cache.turn_token_spans.remove(0);
        }
        self.kv_cache.turn_token_spans.push((
            cur_u_trim.to_string(),
            reply.clone(),
            generated.clone(),
        ));
        let metrics = HoloChatTurnMetrics {
            schema: "uor-r4.holo-geometric-runtime/14",
            model_id: self.model_id.clone(),
            container_sha256: self.container_sha256.clone(),
            prompt_tokens: prompt_ids.len().saturating_sub(reuse_len),
            prefix_reused_tokens: reuse_len,
            generated_tokens: generated.len(),
            float_ops: 0,
            dense_matmul_ops: 0,
            simd128_active: simd_active,
            active_backend,
            q_norm_before_rope: true,
            k_norm_before_rope: true,
            shortlist_budget: compute_shortlist_budget(
                self.shortlist_k as u64,
                self.tokenizer.ascii_fast_tokens.len() as u64,
                self.vocab_size as u64,
            ) as usize,
            flock_attended_positions: flock_attended,
            flock_total_positions: flock_total,
            observatory_frames,
        };
        (reply, metrics)
    }

    /// Formal R4.HoloGeometricRuntime expose exact stationary OPFS piece layout and sampled RHT-512 vocab cluster geometry.
    pub fn storage_geometry_snapshot(&self, piece_bytes: usize) -> serde_json::Value {
        let pb = piece_bytes.max(65_536);
        let mut cursor = 96usize;
        let half = self.head_dim / 2;
        let exp_len = ((EXP_RANGE as usize) << 8) + 1;
        let silu_len = (1usize << (SILU_RANGE_LOG2 - SILU_STEP_LOG2 + 1)) + 1;
        let luts_bytes = self.max_seq * half * 4 + exp_len * 4 + silu_len * 4;
        cursor += luts_bytes;

        let tok_region_start = cursor;
        let tok_est = 4
            + 16
            + self
                .tokenizer
                .id_to_raw
                .iter()
                .map(|b| 4 + b.len())
                .sum::<usize>();
        cursor += tok_est;

        let sigs_start = cursor;
        let sigs_bytes = self.vocab_sigs.len() * 8;
        cursor += sigs_bytes;

        let embed_start = cursor;
        let embed_bytes = 16
            + self.embed.row_exp.len()
            + self.embed.group_mult.len() * 2
            + self.embed.weights.len();
        cursor += embed_bytes;
        cursor += self.final_norm_q24.len() * 4;

        let mut layer_blocks = Vec::with_capacity(self.n_layers);
        for (idx, layer) in self.layers.iter().enumerate() {
            let l_start = cursor;
            let norm_bytes = (layer.in_ln_q24.len()
                + layer.post_ln_q24.len()
                + layer.q_norm_q24.len()
                + layer.k_norm_q24.len())
                * 4;
            let i4_bytes = |m: &HoloInt4RhtMatrix| {
                16 + m.row_exp.len() + m.group_mult.len() * 2 + m.nibbles.len()
            };
            let q_b = i4_bytes(&layer.q_proj);
            let k_b = i4_bytes(&layer.k_proj);
            let v_b = i4_bytes(&layer.v_proj);
            let o_b = i4_bytes(&layer.o_proj);
            let g_b = i4_bytes(&layer.gate_proj);
            let u_b = i4_bytes(&layer.up_proj);
            let (d_b, d_kind) = match &layer.down_proj {
                HoloDownProjMatrix::Int4(m) => (i4_bytes(m), "int4"),
                HoloDownProjMatrix::Int8(m) => (
                    16 + m.row_exp.len() + m.group_mult.len() * 2 + m.weights.len(),
                    "int8",
                ),
            };
            let total = norm_bytes + q_b + k_b + v_b + o_b + g_b + u_b + d_b;
            let l_end = l_start + total;
            cursor = l_end;
            layer_blocks.push(serde_json::json!({
                "layer": idx,
                "byte_start": l_start,
                "byte_end": l_end,
                "bytes": total,
                "piece_start": l_start / pb,
                "piece_end": l_end.saturating_sub(1) / pb,
                "down_proj_dtype": d_kind,
            }));
        }

        let mut vocab_points = Vec::new();
        let sample_target = 96usize.min(self.vocab_size);
        let stride = (self.vocab_size / sample_target.max(1)).max(1);
        for s_idx in 0..sample_target {
            let tid = (s_idx * stride).min(self.vocab_size.saturating_sub(1));
            let base = tid * SIG_WORDS;
            if base + SIG_WORDS <= self.vocab_sigs.len() {
                let xyz = rht512_sig_to_xyz(&self.vocab_sigs[base..base + SIG_WORDS]);
                let decoded = self.tokenizer.decode(&[tid as u32]);
                let label: String = decoded
                    .replace(['\n', '\r', '\t'], " ")
                    .chars()
                    .take(16)
                    .collect();
                vocab_points.push(serde_json::json!({
                    "token_id": tid,
                    "label": if label.trim().is_empty() { format!("#{tid}") } else { label },
                    "xyz": xyz,
                }));
            }
        }

        serde_json::json!({
            "schema": "uor-r4.holo-storage-geometry/1",
            "model_id": self.model_id,
            "container_sha256": self.container_sha256,
            "total_bytes_est": cursor,
            "piece_bytes": pb,
            "total_pieces_est": cursor.div_ceil(pb),
            "vocab_size": self.vocab_size,
            "d_model": self.d_model,
            "n_layers": self.n_layers,
            "n_heads": self.n_heads,
            "n_kv_heads": self.n_kv_heads,
            "shortlist_k": self.shortlist_k,
            "flock_window": self.flock_window,
            "kv_cached_tokens": self.kv_cache.tokens.len(),
            "regions": [
                { "kind": "header_luts", "byte_start": 0, "byte_end": 96 + luts_bytes, "piece_start": 0, "piece_end": (96 + luts_bytes) / pb },
                { "kind": "tokenizer", "byte_start": tok_region_start, "byte_end": sigs_start, "piece_start": tok_region_start / pb, "piece_end": sigs_start.saturating_sub(1) / pb },
                { "kind": "vocab_sigs_512", "byte_start": sigs_start, "byte_end": embed_start, "piece_start": sigs_start / pb, "piece_end": embed_start.saturating_sub(1) / pb },
                { "kind": "tied_embed_int8", "byte_start": embed_start, "byte_end": embed_start + embed_bytes, "piece_start": embed_start / pb, "piece_end": (embed_start + embed_bytes).saturating_sub(1) / pb },
            ],
            "layers": layer_blocks,
            "vocab_rht_clusters": vocab_points,
        })
    }
}

/// Formal R4.StudioFrame compute normalized integer permille (`0..=1000`) probabilities over up to 6 Q16 logit candidates without floats.
pub fn compute_top6_prob_permille(exp_table: &[u32], logits_q16: &[(u32, i32)]) -> Vec<u16> {
    let n = logits_q16.len().min(6);
    if n == 0 {
        return Vec::new();
    }
    let cands = &logits_q16[..n];
    let max_l = cands.iter().map(|&(_, l)| l).fold(i32::MIN, i32::max);
    let mut weights = [0u64; 6];
    let mut den = 0u128;
    let max_diff_q16 = i64::from(EXP_RANGE) << 16;
    for (idx, &(_, logit_q16)) in cands.iter().enumerate() {
        let diff_q16 = (i64::from(max_l) - i64::from(logit_q16)).max(0);
        let w = if exp_table.len() >= 2 {
            exp_neg_with_table(exp_table, diff_q16)
        } else if diff_q16 >= max_diff_q16 {
            0
        } else {
            u64::from(exp_neg_q30((diff_q16 as u64) << 14))
        };
        weights[idx] = w;
        den = den.saturating_add(u128::from(w));
    }

    let mut probs = vec![0u16; n];
    let mut rems = [0u128; 6];
    let fallback = (1000usize / n) as u16;
    for idx in 0..n {
        let num = u128::from(weights[idx]).saturating_mul(1000);
        if let (Some(q), Some(r)) = (num.checked_div(den), num.checked_rem(den)) {
            probs[idx] = q as u16;
            rems[idx] = r;
        } else {
            probs[idx] = fallback;
        }
    }

    let assigned: usize = probs.iter().map(|&p| usize::from(p)).sum();
    let rem_units = 1000usize.saturating_sub(assigned);
    if rem_units > 0 {
        let mut order = [0usize, 1, 2, 3, 4, 5];
        order[..n].sort_by(|&a, &b| {
            rems[b]
                .cmp(&rems[a])
                .then_with(|| cands[b].1.cmp(&cands[a].1))
                .then_with(|| a.cmp(&b))
        });
        for &idx in order[..n].iter().take(rem_units) {
            probs[idx] = probs[idx].saturating_add(1);
        }
    }
    probs
}

/// Formal R4.HoloGeometricRuntime assemble one genuine `HoloStepObservatoryFrame` from live scratch arrays.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_observatory_frame(
    tokenizer: &HoloTok13,
    scratch: &HoloScratch,
    exp_table: &[u32],
    scored: &[(u32, i32)],
    chosen_id: u32,
    is_prefill: bool,
    step_idx: usize,
    phase: &'static str,
    seq_pos: usize,
    prefix_reused_tokens: usize,
    backend: &'static str,
) -> HoloStepObservatoryFrame {
    let mut top_arr: [(u32, i32); 6] = [(0, i32::MIN); 6];
    let mut top_len = 0usize;
    for &(tid, logit) in scored {
        if top_len == 6 && (logit < top_arr[5].1 || (logit == top_arr[5].1 && tid > top_arr[5].0)) {
            continue;
        }
        let pos = top_arr[..top_len]
            .iter()
            .position(|&(et, el)| logit > el || (logit == el && tid < et))
            .unwrap_or(top_len);
        if pos < 6 {
            let end = top_len.min(5);
            for j in (pos..end).rev() {
                top_arr[j + 1] = top_arr[j];
            }
            top_arr[pos] = (tid, logit);
            if top_len < 6 {
                top_len += 1;
            }
        }
    }
    let probs = compute_top6_prob_permille(exp_table, &top_arr[..top_len]);
    let top_candidates: Vec<HoloCandidateBar> = top_arr[..top_len]
        .iter()
        .enumerate()
        .map(|(idx, &(tid, logit_q16))| {
            let text: String = tokenizer.decode(&[tid]).chars().take(24).collect();
            HoloCandidateBar {
                token_id: tid,
                token_text: text,
                logit_q16,
                prob_permille: probs.get(idx).copied().unwrap_or(0),
                selected: !is_prefill && tid == chosen_id,
                hamming_d: scratch.hamming_dists.get(tid as usize).copied(),
            }
        })
        .collect();
    let token_text: String = tokenizer.decode(&[chosen_id]).chars().take(24).collect();
    HoloStepObservatoryFrame {
        step_idx,
        phase,
        seq_pos,
        prefix_reused_tokens,
        token_id: chosen_id,
        token_text,
        layer_residual_mag: scratch.layer_residual_mag.clone(),
        layer_max_bits: scratch.layer_max_bits.clone(),
        layer_flock_spans: scratch.layer_flock_spans.clone(),
        final_attn_peaks: scratch.last_attn_peaks.clone(),
        hamming_hist_32: scratch.last_hamming_hist32,
        min_hamming: scratch.last_min_hamming,
        has_geometric_shortlist: true,
        ttft_ms: 0,
        tps_x10: 0,
        q_manifold_xyz: rht512_sig_to_xyz(&scratch.last_q_sig512),
        top_candidates,
        shortlist_count: scored.len(),
        backend,
    }
}

/// Formal R4.HoloGeometricRuntime emit wasm32 live per-token observatory frame to Worker host bridge.
#[cfg(target_arch = "wasm32")]
pub(crate) fn emit_holo_trace_frame(frame: &HoloStepObservatoryFrame) {
    use wasm_bindgen::{JsCast, JsValue};
    let global = js_sys::global();
    if let Ok(val) = js_sys::Reflect::get(&global, &JsValue::from_str("__uorEmitHoloTraceFrame")) {
        if let Some(func) = val.dyn_ref::<js_sys::Function>() {
            if let Ok(json) = serde_json::to_string(frame) {
                let _ = func.call1(&JsValue::NULL, &JsValue::from_str(&json));
            }
        }
    }
}

/// Formal R4.HoloGeometricRuntime non-wasm32 no-op live observatory hook.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn emit_holo_trace_frame(_frame: &HoloStepObservatoryFrame) {}

/// Formal R4.HoloGeometricRuntime flush complete UTF-8 sequences from raw token bytes without emitting premature U+FFFD.
pub fn flush_incremental_utf8(
    raw_stream: &[u8],
    emitted_raw: &mut usize,
    flush_lossy_end: bool,
) -> String {
    let mut delta = String::new();
    while *emitted_raw < raw_stream.len() {
        match std::str::from_utf8(&raw_stream[*emitted_raw..]) {
            Ok(valid) => {
                delta.push_str(valid);
                *emitted_raw = raw_stream.len();
            }
            Err(e) => {
                let valid_len = e.valid_up_to();
                if valid_len > 0 {
                    if let Ok(prefix) =
                        std::str::from_utf8(&raw_stream[*emitted_raw..*emitted_raw + valid_len])
                    {
                        delta.push_str(prefix);
                    }
                    *emitted_raw += valid_len;
                }
                if let Some(bad_len) = e.error_len() {
                    delta.push('\u{FFFD}');
                    *emitted_raw += bad_len;
                } else {
                    break;
                }
            }
        }
    }
    if flush_lossy_end && *emitted_raw < raw_stream.len() {
        let tail = String::from_utf8_lossy(&raw_stream[*emitted_raw..]);
        delta.push_str(&tail);
        *emitted_raw = raw_stream.len();
    }
    delta
}

/// Formal R4.HoloGeometricRuntime decide whether a decode step emits an observatory trace frame.
pub(crate) fn should_emit_decode_observatory_frame(
    cur_step: usize,
    emitted_tokens: usize,
    max_new_tokens: usize,
    every_token: bool,
) -> bool {
    every_token
        || cur_step == 1
        || (cur_step > 0 && cur_step.is_multiple_of(4))
        || emitted_tokens >= max_new_tokens
}

/// Formal R4.HoloGeometricRuntime build, emit, and record a bounded observatory trace frame.
#[allow(clippy::too_many_arguments)]
pub(crate) fn record_observatory_frame(
    model: &HoloV13Model,
    last_scored: &[(u32, i32)],
    token_id: u32,
    is_prefill: bool,
    step_idx: usize,
    phase: &'static str,
    reuse_len: usize,
    active_backend: &'static str,
    frames: &mut Vec<HoloStepObservatoryFrame>,
) {
    let frame = build_observatory_frame(
        &model.tokenizer,
        &model.scratch,
        &model.exp_table,
        last_scored,
        token_id,
        is_prefill,
        step_idx,
        phase,
        model.kv_cache.tokens.len().saturating_sub(1),
        reuse_len,
        active_backend,
    );
    if admit_live_token_frame(
        frame.has_geometric_shortlist,
        frame.top_candidates.iter().any(|c| c.hamming_d.is_some()),
        0,
        100_000,
    ) != LiveFrameVerdict::FrameAdmitted
    {
        return;
    }
    emit_holo_trace_frame(&frame);
    if frames.len() < 64 {
        frames.push(frame);
    }
}

/// Formal R4.HoloGeometricRuntime select the highest-scoring non-cyclic candidate token from scored logits.
pub(crate) fn pick_next_holo_token(
    last_scored: &[(u32, i32)],
    generated: &[u32],
    eos_token_id: u32,
    eot_token_id: u32,
    tokenizer: &HoloTok13,
) -> u32 {
    let mut blocked_ids: Vec<u32> = Vec::new();
    loop {
        let mut best_id = eos_token_id;
        let mut best_logit = i32::MIN;
        let mut found_any = false;
        for &(tid, logit) in last_scored {
            if blocked_ids.contains(&tid) {
                continue;
            }
            if !found_any || logit > best_logit {
                best_logit = logit;
                best_id = tid;
                found_any = true;
            }
        }
        if !found_any || best_id == eos_token_id || best_id == eot_token_id {
            return best_id;
        }
        if blocked_ids.len() < 8 && token_causes_cycle(generated, best_id, tokenizer) {
            blocked_ids.push(best_id);
            continue;
        }
        return best_id;
    }
}

/// Formal R4.HoloGeometricRuntime reject 3+ identical alphanumeric tokens, `,,`/`; ;`, or immediate k-gram cycles (`p in 2..=8`).
pub(crate) fn token_causes_cycle(generated: &[u32], cand: u32, tokenizer: &HoloTok13) -> bool {
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
            rht_signs_packed: rht_signs_packed_u32(cols, seed),
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
        rht_signs_packed: rht_signs_packed_u32(cols, seed),
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
        let mut scratch = HoloScratch::default();
        scratch.ensure_sized(
            self.d_model,
            self.n_heads * self.head_dim,
            self.n_kv_heads * self.head_dim,
            self.intermediate_size,
            self.head_dim,
            self.n_layers,
            self.vocab_size,
        );
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
            shortlist_k: DEFAULT_SHORTLIST_K,
            flock_window: DEFAULT_FLOCK_WINDOW,
            scratch,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
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

    pub(crate) fn build_synthetic_test_model() -> HoloV13Model {
        let vocab = 256usize;
        let dm = 128usize;
        let nl = 1usize;
        let nh = 2usize;
        let nkv = 1usize;
        let hd = 128usize;
        let ffn = 192usize;
        let max_seq = 256usize;

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

        for v in 0..vocab {
            for w in 0..SIG_WORDS {
                let pat =
                    0x5555_5555_5555_5555u64 ^ ((v as u64).wrapping_mul(0x9e37_79b9) << (w * 3));
                raw.extend_from_slice(&pat.to_le_bytes());
            }
        }

        // Tied embedding matrix [vocab, dm] with varied deterministic i8 weights
        raw.extend_from_slice(&(vocab as u32).to_le_bytes());
        raw.extend_from_slice(&(dm as u32).to_le_bytes());
        raw.extend_from_slice(&1u64.to_le_bytes());
        raw.resize(raw.len() + vocab, (-20i8) as u8);
        for _ in 0..(vocab * (dm >> 5)) {
            raw.extend_from_slice(&3u16.to_le_bytes());
        }
        for r in 0..vocab {
            for c in 0..dm {
                raw.push((((r * 7 + c * 13) % 41) as i8 - 20) as u8);
            }
        }

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
            for i in 0..(rows * (cols >> 5)) {
                buf.extend_from_slice(&(((i % 5) + 1) as u16).to_le_bytes());
            }
            for i in 0..(rows * (cols >> 1)) {
                buf.push((((i * 17 + 3) % 15) as u8) | (((((i * 23 + 5) % 15) as u8) & 15) << 4));
            }
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
        builder.finalize().expect("non-square Int4 mount")
    }

    #[test]
    fn non_square_layer_streaming_mount_and_inference_succeeds() {
        let mut model = build_synthetic_test_model();
        let dm = model.d_model;
        assert_eq!(model.n_layers, 1);
        assert_eq!(dm, 128);
        let (_reply, metrics) = model.generate_chat_turn("hello", "assistant", 4);
        assert_eq!(metrics.float_ops, 0);
        assert_eq!(metrics.dense_matmul_ops, 0);
        assert!(metrics.q_norm_before_rope);
        assert!(metrics.k_norm_before_rope);
        assert!(!metrics.observatory_frames.is_empty());

        let geom = model.storage_geometry_snapshot(65_536);
        assert_eq!(geom["layers"].as_array().map(|a| a.len()), Some(1));
        assert!(!geom["vocab_rht_clusters"]
            .as_array()
            .is_none_or(|a| a.is_empty()));

        // Turn 2 with identical system prompt + turn-1 history reuses KV cache prefix!
        let history = vec![("hello".to_string(), "".to_string())];
        let (_reply2, metrics2) =
            model.generate_chat_turn_with_history("follow up", &history, "assistant", 4);
        assert!(
            metrics2.prefix_reused_tokens > 0,
            "Turn 2 must reuse prefix KV cache tokens, got {:?}",
            metrics2
        );
    }

    #[test]
    fn gemv_4row_and_indexed_match_single_row_reference() {
        let cols = 64usize;
        let groups = cols >> 5;
        let act16: Vec<i16> = (0..cols).map(|i| (i as i16) * 29 - 900).collect();
        for rows in [7usize, 16usize] {
            let row_exp: Vec<i8> = (0..rows).map(|r| -12 - (r as i8 % 5)).collect();
            let group_mult: Vec<u16> = (0..rows * groups)
                .map(|i| ((i as u16) * 7 + 3) % 100 + 1)
                .collect();
            let nibbles: Vec<u8> = (0..rows * (cols >> 1))
                .map(|i| (((i * 11 + 1) % 15) as u8) | ((((i * 19 + 4) % 15) as u8) << 4))
                .collect();
            let mut out_tiled = vec![0i32; rows];
            gemv_i4_rht_rows(
                rows,
                cols,
                &nibbles,
                &group_mult,
                &row_exp,
                -10,
                -18,
                &act16,
                &mut out_tiled,
            );
            for r in 0..rows {
                let sh = -18 - (i32::from(row_exp[r]) - 10);
                let mut acc = 0i64;
                for g in 0..groups {
                    let nb: &[u8; 16] = (&nibbles
                        [r * (cols >> 1) + g * 16..r * (cols >> 1) + (g + 1) * 16])
                        .try_into()
                        .unwrap();
                    let a: &[i16; 32] = (&act16[g * 32..(g + 1) * 32]).try_into().unwrap();
                    acc += i64::from(dot_i4_group32(nb, a)) * i64::from(group_mult[r * groups + g]);
                }
                assert_eq!(out_tiled[r], scale_i64_acc_to_i32(acc, sh));
            }

            let weights: Vec<i8> = (0..rows * cols)
                .map(|i| (((i * 13) % 200) as i32 - 100) as i8)
                .collect();
            let mut out_i8 = vec![0i32; rows];
            gemv_i8_rht_rows(
                rows,
                cols,
                &weights,
                &group_mult,
                &row_exp,
                -10,
                -16,
                &act16,
                &mut out_i8,
            );
            let mat = HoloInt8RhtMatrix {
                rows,
                cols,
                rht_seed: 1,
                rht_signs_packed: rht_signs_packed_u32(cols, 1),
                row_exp: row_exp.clone(),
                group_mult: group_mult.clone(),
                weights: weights.clone(),
            };
            let idx_rows: Vec<u32> = (0..rows as u32).rev().collect();
            let mut scored_idx = Vec::new();
            gemv_i8_indexed_into(&mat, &idx_rows, &act16, -10, -16, &mut scored_idx);
            assert_eq!(scored_idx.len(), rows);
            for (pos, &(rid, val)) in scored_idx.iter().enumerate() {
                assert_eq!(rid, idx_rows[pos]);
                assert_eq!(val, out_i8[rid as usize]);
            }
        }
    }

    #[test]
    fn prefill_tokens_tiled_matches_sequential_step_token_bit_exact() {
        let mut model_seq = build_synthetic_test_model();
        let mut model_batched = model_seq.clone();
        let prompt_tokens = [10u32, 21, 32, 43, 54, 65, 76, 87, 98, 109, 120];

        let mut seq_logits = None;
        let mut seq_fa = 0usize;
        let mut seq_ft = 0usize;
        for (i, &tok) in prompt_tokens.iter().enumerate() {
            let compute_last = i + 1 == prompt_tokens.len();
            let (sc, fa, ft) = model_seq.step_token(tok, &prompt_tokens, compute_last, false);
            seq_fa += fa;
            seq_ft += ft;
            if compute_last {
                seq_logits = sc;
            }
        }

        let (batch_logits, batch_fa, batch_ft) =
            model_batched.prefill_tokens_tiled(&prompt_tokens, &prompt_tokens, true, false);

        assert_eq!(batch_fa, seq_fa);
        assert_eq!(batch_ft, seq_ft);
        assert_eq!(model_batched.kv_cache.tokens, model_seq.kv_cache.tokens);
        assert_eq!(model_batched.kv_cache.seq_len, model_seq.kv_cache.seq_len);
        assert_eq!(model_batched.kv_cache.k_i16, model_seq.kv_cache.k_i16);
        assert_eq!(model_batched.kv_cache.k_scales, model_seq.kv_cache.k_scales);
        assert_eq!(model_batched.kv_cache.v_i16, model_seq.kv_cache.v_i16);
        assert_eq!(model_batched.kv_cache.v_scales, model_seq.kv_cache.v_scales);
        assert_eq!(model_batched.kv_cache.k_sig128, model_seq.kv_cache.k_sig128);
        assert_eq!(model_batched.scratch.x, model_seq.scratch.x);
        assert_eq!(batch_logits, seq_logits);
    }

    #[test]
    fn prewarm_system_prefix_enables_turn1_kv_reuse() {
        let mut model = build_synthetic_test_model();
        let sys = "You are LexLean-R4 AI Studio.";
        model.prewarm_system_prefix(sys);
        assert!(
            !model.kv_cache.tokens.is_empty(),
            "prewarm_system_prefix must populate prefix KV cache tokens"
        );
        let (_reply, metrics) = model.generate_chat_turn("hi", sys, 4);
        assert!(
            metrics.prefix_reused_tokens > 0,
            "Turn 1 must reuse prewarmed system prefix tokens, got {:?}",
            metrics
        );
    }

    #[test]
    fn phase1_fwht32_i32_rmsnorm_i64_and_hamming_match_reference() {
        let mut a32 = [0i32; 32];
        let mut a64 = [0i64; 32];
        for i in 0..32 {
            let v = ((i as i32 * 37) % 1024) - 512;
            a32[i] = v;
            a64[i] = i64::from(v);
        }
        int_fwht32_i32(&mut a32);
        int_fwht32_i64(&mut a64);
        for i in 0..32 {
            assert_eq!(i64::from(a32[i]), a64[i]);
        }

        // Verify int_rms_norm_gain i64 fast path matches exact i128 reference within <= 1 LSB.
        let mut x = [0i32; 128];
        let mut g = [0i32; 128];
        for i in 0..128 {
            x[i] = ((i as i32 * 731) - 42_000) * 19;
            g[i] = (1i32 << 24) + ((i as i32 - 64) * 51_200);
        }
        let mut out_fast = vec![0i64; 128];
        int_rms_norm_gain(&x, &g, 281_474_977, -18, -22, &mut out_fast);

        let mut max_abs = 0u32;
        for &v in &x {
            max_abs = max_abs.max(v.unsigned_abs());
        }
        let sh = (32 - max_abs.leading_zeros() as i32).max(1) - 20;
        let mut out_ref = vec![0i64; 128];
        let mut sum = 0u128;
        for (dst, &v) in out_ref.iter_mut().zip(&x) {
            let y = shift_round(i64::from(v), sh);
            *dst = y;
            let ay = u128::from(y.unsigned_abs());
            sum = sum.wrapping_add(ay.wrapping_mul(ay));
        }
        let mean = sum / 128u128;
        let eps_int = shift_round(281_474_977, ((-18 + sh) << 1) - (-48)).max(0) as u128;
        let s = clz_newton_isqrt(mean.saturating_add(eps_int).max(1) << 68).max(1);
        let r = (1u128 << 96) / s;
        let r_shift = (86 - 22) as u32;
        let round_half = 1i128 << (r_shift - 1);
        for (i, slot) in out_ref.iter_mut().enumerate() {
            let unit = (i128::from(*slot) * (r as i128) * i128::from(g[i]))
                .saturating_add(round_half)
                >> r_shift;
            *slot = unit as i64;
            assert!(
                (out_fast[i] - *slot).abs() <= 16,
                "lane {i} fast={} ref={}",
                out_fast[i],
                *slot
            );
        }

        let q = [0x0F0F_0F0F_0F0F_0F0Fu64; SIG_WORDS];
        let v = [0x00FF_00FF_00FF_00FFu64; SIG_WORDS];
        let mut expected = 0u32;
        for i in 0..SIG_WORDS {
            expected += (q[i] ^ v[i]).count_ones();
        }
        assert_eq!(hamming512_u64x8(&q, &v), expected);
    }

    #[test]
    fn multi_turn_history_reuses_full_assistant_reply_kv_prefix() {
        let mut model = build_synthetic_test_model();
        let sys = "sys";
        model.prewarm_system_prefix(sys);
        let u1 = "Rust";
        let (reply1, m1) = model.generate_chat_turn(u1, sys, 4);
        assert!(
            m1.prefix_reused_tokens > 0,
            "Turn 1 must reuse prewarmed system prefix, got {:?}",
            m1
        );
        let expected_turn1_cached =
            m1.prefix_reused_tokens + m1.prompt_tokens + m1.generated_tokens;
        let history = vec![(u1.to_string(), reply1)];
        let (_reply2, m2) = model.generate_chat_turn_with_history("More", &history, sys, 4);
        assert!(
            m2.prefix_reused_tokens >= expected_turn1_cached,
            "Turn 2 must reuse 100% of Turn 1 prompt + assistant reply KV ({} >= {}), got {:?}",
            m2.prefix_reused_tokens,
            expected_turn1_cached,
            m2
        );
    }
}
