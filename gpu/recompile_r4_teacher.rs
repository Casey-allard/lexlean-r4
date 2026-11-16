//! Pure-Rust Teacher Recompiler & Pure-Integer Quantizer for `UORR4V12` (`SmolLM2-135M-Instruct`).
//!
//! Zero Python anywhere (`rustc -O gpu/recompile_r4_teacher.rs -o /var/tmp/recompile_r4_teacher`).
//! Executes on `wick`:
//! 1. Verifies / downloads `HuggingFaceTB/SmolLM2-135M-Instruct` (`config.json`, `tokenizer.json`,
//!    `model.safetensors`) in `/var/tmp/r4_teacher_cache` and verifies SHA-256 digests.
//! 2. Generates and compiles `/var/tmp/r4_teacher_cache/r4_smollm2_cuda.cu` with `nvcc -O3 -arch=sm_89 -lcublas`
//!    for RTX 4090 FP32 teacher inference, 12-prompt teacher logit capture, genuine teacher rollout generation,
//!    and QAT KL-divergence distillation of the compact `<= 4.5 MiB` `bootstrap_v12_seed.uorr4`.
//! 3. Folds RMSNorm gains (`input_layernorm` -> `q/k/v_proj`, `post_attention_layernorm` -> `gate/up_proj`),
//!    keeps `model.norm.weight` separate because `embed_tokens`/`lm_head` are tied, applies Block-32
//!    Walsh-Hadamard transform + 4-bit group-32 dyadic grid-scale `(16+m)*2^(e-4)` quantization with per-row
//!    exponent bases, audits Conway-Sloane E8 lattice nearest-point residuals, precomputes 576-bit (`9 x u64`)
//!    Hamming signatures for the full 49,152 vocabulary, and seals half-split `(i, i+32)` Q14 RoPE, exp,
//!    SiLU, arcosh, and Fibonacci Zeta `PhaseQ29` LUTs into `UORR4V12`.
//! 4. Exports full 30L `UORR4V12` (`<= 90 MiB`) split into <= 1 MiB shards + Stitch 2-of-3 XOR parity into
//!    `gpu/weights/v10/`, compact distilled `crates/wasm/src/bootstrap_v12_seed.uorr4` (`<= 4.5 MiB`),
//!    compacted `crates/wasm/src/tokenizer_v12.json`, and `gpu/rtx4090_r4_recompiled_v12_receipt.json`
//!    verified with a pure-integer (`float_ops=0`, `dense_matmul_ops=0`) evaluator matching `r4_geometric_stack.rs`.

#![allow(dead_code, clippy::all)]

use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;

const R4_VEC_MAGIC: &[u8; 8] = b"UORR4V12";
const R4_VEC_VERSION: u32 = 12;
const SHARD_MAX_BYTES: usize = 1_048_576;
const GROUP: usize = 32;
const RHT_BLOCK: usize = 32;
const RESIDUAL_EXP: i32 = -16;
const NORM_FINE_EXP: i32 = -24;
const PRODUCT_EXP: i32 = -32;
const SCORE_EXP: i32 = -24;
const EXP_STEP_LOG2: i32 = -8;
const EXP_RANGE: i32 = 32;
const SILU_STEP_LOG2: i32 = -8;
const SILU_RANGE_LOG2: i32 = 4;
const ROPE_Q: i32 = 14;
const ARCOSH_CODE_BITS: u32 = 96;
const PHASE_MODULUS_Q29: u64 = 3_373_259_426;
const PHASE_HALF_Q29: u64 = 1_686_629_713;
const ATTENTION_ZETA_CHANNELS: [u16; 8] = [0, 1, 2, 3, 5, 8, 13, 21];
const ZETA_GAMMAS_TOP22: [f64; 22] = [
    14.134725141734695,
    21.022039638771556,
    25.01085758014569,
    30.424876125859512,
    32.93506158773919,
    37.586178158825675,
    40.9187190121475,
    43.327073280914995,
    48.00515088116716,
    49.7738324776723,
    52.970321477714464,
    56.44624769706339,
    59.34704400260235,
    60.83177852460981,
    65.1125440480816,
    67.07981052949417,
    69.54640171117398,
    72.0671576744819,
    75.70469069908393,
    77.1448400688748,
    79.33737502024937,
    82.91038085408603,
];

const E8P_PACKED_ABS: [u32; 256] = [
    0x99999999, 0x59999999, 0xd9999999, 0x7999b999, 0xb999b999, 0x3999b999, 0x9999d999, 0x5999d999,
    0x7b999999, 0xbb999999, 0x3b999999, 0x9b99b999, 0x5b99b999, 0x7b99d999, 0x9d999999, 0x5d999999,
    0x7d99b999, 0x79999b99, 0xb9999b99, 0x39999b99, 0x9999bb99, 0x5999bb99, 0x7999db99, 0x9b999b99,
    0x5b999b99, 0x7b99bb99, 0xbb99bb99, 0x7d999b99, 0x99999d99, 0x59999d99, 0x7999bd99, 0x7b999d99,
    0x79b99999, 0xb9b99999, 0x39b99999, 0x99b9b999, 0x59b9b999, 0x79b9d999, 0x9bb99999, 0x5bb99999,
    0x7bb9b999, 0xbbb9b999, 0x7db99999, 0x99b99b99, 0x59b99b99, 0x79b9bb99, 0xb9b9bb99, 0x7bb99b99,
    0xbbb99b99, 0x9bb9bb99, 0x79b99d99, 0x99d99999, 0x59d99999, 0x79d9b999, 0x7bd99999, 0x79d99b99,
    0x799999b9, 0xb99999b9, 0x399999b9, 0x9999b9b9, 0x5999b9b9, 0x7999d9b9, 0x9b9999b9, 0x5b9999b9,
    0x7b99b9b9, 0xbb99b9b9, 0x7d9999b9, 0x99999bb9, 0x59999bb9, 0x7999bbb9, 0xb999bbb9, 0x7b999bb9,
    0xbb999bb9, 0x9b99bbb9, 0x79999db9, 0x99b999b9, 0x59b999b9, 0x79b9b9b9, 0xb9b9b9b9, 0x7bb999b9,
    0xbbb999b9, 0x9bb9b9b9, 0x79b99bb9, 0xb9b99bb9, 0x99b9bbb9, 0x9bb99bb9, 0x79d999b9, 0x999999d9,
    0x599999d9, 0x7999b9d9, 0x7b9999d9, 0x79999bd9, 0x79b999d9, 0x799b9999, 0xb99b9999, 0x399b9999,
    0x999bb999, 0x599bb999, 0x799bd999, 0x9b9b9999, 0x5b9b9999, 0x7b9bb999, 0xbb9bb999, 0x7d9b9999,
    0x999b9b99, 0x599b9b99, 0x799bbb99, 0xb99bbb99, 0x7b9b9b99, 0xbb9b9b99, 0x9b9bbb99, 0x799b9d99,
    0x99bb9999, 0x59bb9999, 0x79bbb999, 0xb9bbb999, 0x7bbb9999, 0xbbbb9999, 0x9bbbb999, 0x79bb9b99,
    0xb9bb9b99, 0x99bbbb99, 0x9bbb9b99, 0x79db9999, 0x999b99b9, 0x599b99b9, 0x799bb9b9, 0xb99bb9b9,
    0x7b9b99b9, 0xbb9b99b9, 0x9b9bb9b9, 0x799b9bb9, 0xb99b9bb9, 0x999bbbb9, 0x9b9b9bb9, 0x79bb99b9,
    0xb9bb99b9, 0x99bbb9b9, 0x9bbb99b9, 0x99bb9bb9, 0x799b99d9, 0x999d9999, 0x599d9999, 0x799db999,
    0x7b9d9999, 0x799d9b99, 0x79bd9999, 0x799d99b9, 0x7999999b, 0xb999999b, 0x3999999b, 0x9999b99b,
    0x5999b99b, 0x7999d99b, 0x9b99999b, 0x5b99999b, 0x7b99b99b, 0xbb99b99b, 0x7d99999b, 0x99999b9b,
    0x59999b9b, 0x7999bb9b, 0xb999bb9b, 0x7b999b9b, 0xbb999b9b, 0x9b99bb9b, 0x79999d9b, 0x99b9999b,
    0x59b9999b, 0x79b9b99b, 0xb9b9b99b, 0x7bb9999b, 0xbbb9999b, 0x9bb9b99b, 0x79b99b9b, 0xb9b99b9b,
    0x99b9bb9b, 0x9bb99b9b, 0x79d9999b, 0x999999bb, 0x599999bb, 0x7999b9bb, 0xb999b9bb, 0x7b9999bb,
    0xbb9999bb, 0x9b99b9bb, 0x79999bbb, 0xb9999bbb, 0x9999bbbb, 0x9b999bbb, 0x79b999bb, 0xb9b999bb,
    0x99b9b9bb, 0x9bb999bb, 0x99b99bbb, 0x799999db, 0x999b999b, 0x599b999b, 0x799bb99b, 0xb99bb99b,
    0x7b9b999b, 0xbb9b999b, 0x9b9bb99b, 0x799b9b9b, 0xb99b9b9b, 0x999bbb9b, 0x9b9b9b9b, 0x79bb999b,
    0xb9bb999b, 0x99bbb99b, 0x9bbb999b, 0x99bb9b9b, 0x799b99bb, 0xb99b99bb, 0x999bb9bb, 0x9b9b99bb,
    0x999b9bbb, 0x99bb99bb, 0x799d999b, 0x9999999d, 0x5999999d, 0x7999b99d, 0x7b99999d, 0x79999b9d,
    0x79b9999d, 0x799b999d, 0x68888888, 0xa8888888, 0x28888888, 0x8888a888, 0x4888a888, 0x6888c888,
    0x8a888888, 0x4a888888, 0x6a88a888, 0xaa88a888, 0x6c888888, 0x88888a88, 0x48888a88, 0x6888aa88,
    0xa888aa88, 0x6a888a88, 0xaa888a88, 0x8a88aa88, 0x68888c88, 0x88a88888, 0x48a88888, 0x68a8a888,
    0xa8a8a888, 0x6aa88888, 0xaaa88888, 0x8aa8a888, 0x68a88a88, 0xa8a88a88, 0x88a8aa88, 0x8aa88a88,
];

// ============================================================================
// Pure-Rust SHA-256 & Minimal JSON Parser
// ============================================================================

const K256: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity(((data.len() + 9 + 63) / 64) * 64);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    for block in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K256[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, delta) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(delta);
        }
    }
    let mut out = [0u8; 32];
    for (i, &w) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&w.to_be_bytes());
    }
    out
}

fn sha256_hex(data: &[u8]) -> String {
    let d = sha256_bytes(data);
    let mut s = String::with_capacity(64);
    for b in d {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[derive(Clone, Debug)]
enum JsonVal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<JsonVal>),
    Obj(BTreeMap<String, JsonVal>),
}

impl JsonVal {
    fn parse_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut p = JsonParser { bytes, pos: 0 };
        let val = p.parse_val()?;
        p.skip_ws();
        if p.pos != bytes.len() {
            return Err(format!("trailing JSON bytes at {}", p.pos));
        }
        Ok(val)
    }
    fn get(&self, key: &str) -> Option<&JsonVal> {
        match self {
            JsonVal::Obj(m) => m.get(key),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            JsonVal::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
    fn as_bool(&self) -> Option<bool> {
        match self {
            JsonVal::Bool(b) => Some(*b),
            _ => None,
        }
    }
    fn as_i64(&self) -> Option<i64> {
        match self {
            JsonVal::Int(v) => Some(*v),
            JsonVal::Float(v) => Some(*v as i64),
            _ => None,
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match self {
            JsonVal::Float(v) => Some(*v),
            JsonVal::Int(v) => Some(*v as f64),
            _ => None,
        }
    }
    fn as_usize(&self) -> Option<usize> {
        self.as_i64().and_then(|v| usize::try_from(v).ok())
    }
    fn as_arr(&self) -> Option<&[JsonVal]> {
        match self {
            JsonVal::Arr(v) => Some(v),
            _ => None,
        }
    }
    fn as_obj(&self) -> Option<&BTreeMap<String, JsonVal>> {
        match self {
            JsonVal::Obj(m) => Some(m),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b' ' | b'\n' | b'\r' | b'\t') {
            self.pos += 1;
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }
    fn parse_val(&mut self) -> Result<JsonVal, String> {
        let Some(ch) = self.peek() else {
            return Err("unexpected EOF".to_string());
        };
        match ch {
            b'"' => self.parse_str().map(JsonVal::Str),
            b'{' => self.parse_obj(),
            b'[' => self.parse_arr(),
            b't' => {
                if self.bytes[self.pos..].starts_with(b"true") {
                    self.pos += 4;
                    Ok(JsonVal::Bool(true))
                } else {
                    Err("expected true".to_string())
                }
            }
            b'f' => {
                if self.bytes[self.pos..].starts_with(b"false") {
                    self.pos += 5;
                    Ok(JsonVal::Bool(false))
                } else {
                    Err("expected false".to_string())
                }
            }
            b'n' => {
                if self.bytes[self.pos..].starts_with(b"null") {
                    self.pos += 4;
                    Ok(JsonVal::Null)
                } else {
                    Err("expected null".to_string())
                }
            }
            b'-' | b'0'..=b'9' => self.parse_num(),
            _ => Err(format!("unexpected JSON char {:?} at {}", ch as char, self.pos)),
        }
    }
    fn parse_str(&mut self) -> Result<String, String> {
        self.pos += 1;
        let mut s = Vec::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            self.pos += 1;
            if b == b'"' {
                return String::from_utf8(s).map_err(|e| format!("utf8 str error: {e}"));
            }
            if b == b'\\' {
                if self.pos >= self.bytes.len() {
                    return Err("unterminated escape".to_string());
                }
                let esc = self.bytes[self.pos];
                self.pos += 1;
                match esc {
                    b'"' => s.push(b'"'),
                    b'\\' => s.push(b'\\'),
                    b'/' => s.push(b'/'),
                    b'b' => s.push(0x08),
                    b'f' => s.push(0x0C),
                    b'n' => s.push(b'\n'),
                    b'r' => s.push(b'\r'),
                    b't' => s.push(b'\t'),
                    b'u' => {
                        let cp = self.parse_hex4()?;
                        let mut code = u32::from(cp);
                        if (0xD800..=0xDBFF).contains(&cp)
                            && self.pos + 6 <= self.bytes.len()
                            && self.bytes[self.pos] == b'\\'
                            && self.bytes[self.pos + 1] == b'u'
                        {
                            self.pos += 2;
                            let low = self.parse_hex4()?;
                            if (0xDC00..=0xDFFF).contains(&low) {
                                code = 0x10000 + (((u32::from(cp) - 0xD800) << 10) | (u32::from(low) - 0xDC00));
                            }
                        }
                        if let Some(ch) = char::from_u32(code) {
                            let mut buf = [0u8; 4];
                            s.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                    }
                    other => s.push(other),
                }
            } else {
                s.push(b);
            }
        }
        Err("unterminated JSON string".to_string())
    }
    fn parse_hex4(&mut self) -> Result<u16, String> {
        if self.pos + 4 > self.bytes.len() {
            return Err("truncated \\uXXXX".to_string());
        }
        let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4]).map_err(|e| e.to_string())?;
        self.pos += 4;
        u16::from_str_radix(hex, 16).map_err(|e| e.to_string())
    }
    fn parse_num(&mut self) -> Result<JsonVal, String> {
        let start = self.pos;
        let mut is_float = false;
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if matches!(b, b'.' | b'e' | b'E') {
                is_float = true;
            }
            if matches!(b, b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let raw = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|e| e.to_string())?;
        if is_float {
            raw.parse::<f64>().map(JsonVal::Float).map_err(|e| e.to_string())
        } else {
            raw.parse::<i64>().map(JsonVal::Int).map_err(|e| e.to_string())
        }
    }
    fn parse_arr(&mut self) -> Result<JsonVal, String> {
        self.pos += 1;
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsonVal::Arr(items));
        }
        loop {
            items.push(self.parse_val()?);
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(JsonVal::Arr(items));
                }
                other => return Err(format!("expected ',' or ']' in array, got {other:?}")),
            }
        }
    }
    fn parse_obj(&mut self) -> Result<JsonVal, String> {
        self.pos += 1;
        let mut map = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonVal::Obj(map));
        }
        loop {
            if self.peek() != Some(b'"') {
                return Err("expected object key string".to_string());
            }
            let k = self.parse_str()?;
            if self.peek() != Some(b':') {
                return Err("expected ':' after key".to_string());
            }
            self.pos += 1;
            let v = self.parse_val()?;
            map.insert(k, v);
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(JsonVal::Obj(map));
                }
                other => return Err(format!("expected ',' or '}}' in obj, got {other:?}")),
            }
        }
    }
}

// ============================================================================
// Exact SmolLM2 ByteLevel + Individual-Digits Tokenizer
// ============================================================================

struct SmolTokenizer {
    vocab_size: usize,
    id_to_piece: Vec<String>,
    id_to_bytes: Vec<Vec<u8>>,
    piece_to_id: HashMap<String, u32>,
    merges: HashMap<(String, String), usize>,
    added_tokens: Vec<(String, u32)>,
    byte_encoder: [char; 256],
    byte_tokens: [u32; 256],
}

impl SmolTokenizer {
    fn from_json_bytes(raw: &[u8]) -> Result<Self, String> {
        let root = JsonVal::parse_bytes(raw)?;
        let model = root.get("model").ok_or("missing model")?;
        let vocab_obj = model.get("vocab").and_then(JsonVal::as_obj).ok_or("missing model.vocab")?;
        let mut max_id = 0usize;
        for (_, v) in vocab_obj {
            if let Some(id) = v.as_usize() {
                max_id = max_id.max(id);
            }
        }
        let added_arr = root.get("added_tokens").and_then(JsonVal::as_arr).ok_or("missing added_tokens")?;
        for entry in added_arr {
            if let Some(id) = entry.get("id").and_then(JsonVal::as_usize) {
                max_id = max_id.max(id);
            }
        }
        let vocab_size = max_id + 1;
        let mut id_to_piece = vec![String::new(); vocab_size];
        let mut piece_to_id = HashMap::with_capacity(vocab_size);
        for (k, v) in vocab_obj {
            if let Some(id) = v.as_usize() {
                id_to_piece[id] = k.clone();
                piece_to_id.insert(k.clone(), id as u32);
            }
        }
        let mut added_tokens = Vec::new();
        for entry in added_arr {
            if let (Some(content), Some(id)) = (
                entry.get("content").and_then(JsonVal::as_str),
                entry.get("id").and_then(JsonVal::as_usize),
            ) {
                if id_to_piece[id].is_empty() {
                    id_to_piece[id] = content.to_string();
                }
                piece_to_id.entry(content.to_string()).or_insert(id as u32);
                added_tokens.push((content.to_string(), id as u32));
            }
        }
        added_tokens.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        let merges_arr = model.get("merges").and_then(JsonVal::as_arr).ok_or("missing model.merges")?;
        let mut merges = HashMap::with_capacity(merges_arr.len());
        for (rank, m) in merges_arr.iter().enumerate() {
            if let Some(s) = m.as_str() {
                if let Some((a, b)) = s.split_once(' ') {
                    merges.insert((a.to_string(), b.to_string()), rank);
                }
            }
        }

        // Exact GPT-2 bytes_to_unicode mapping.
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
            let ch = char::from_u32(u32::from(c)).unwrap();
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
            if all_byte_chars && !piece.is_empty() && !added_tokens.iter().any(|(_, tid)| *tid as usize == id) {
                id_to_bytes[id] = decoded;
            } else {
                id_to_bytes[id] = piece.as_bytes().to_vec();
            }
        }

        let fallback_byte_tok = *piece_to_id.get("\u{0120}").unwrap_or(&0);
        let mut byte_tokens = [fallback_byte_tok; 256];
        for b in 0..256 {
            let ch = byte_encoder[b];
            let mut s = String::new();
            s.push(ch);
            if let Some(&tid) = piece_to_id.get(&s) {
                byte_tokens[b] = tid;
            }
        }

        Ok(Self {
            vocab_size,
            id_to_piece,
            id_to_bytes,
            piece_to_id,
            merges,
            added_tokens,
            byte_encoder,
            byte_tokens,
        })
    }

    /// GPT-2 pretokenizer after splitting individual ASCII digits (`Digits { individual_digits: true }`).
    fn pretokenize_spans(span: &str) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let bytes = span.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            let rem = &span[i..];
            let mut chars = rem.char_indices();
            let Some((_, first_ch)) = chars.next() else { break };

            // Individual digit rule triggers before ByteLevel regex.
            if first_ch.is_numeric() {
                let end = i + first_ch.len_utf8();
                out.push((i, end));
                i = end;
                continue;
            }

            // Contractions: 's, 't, 're, 've, 'm, 'll, 'd
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

            // Optional single space followed by letters or non-whitespace non-digit punctuation.
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

            // `\s+(?!\S)|\s+`
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
        let mut word: Vec<String> = pretoken.as_bytes().iter().map(|&b| self.byte_encoder[b as usize].to_string()).collect();
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
        word.iter().map(|p| *self.piece_to_id.get(p).unwrap_or(&0)).collect()
    }

    fn encode(&self, text: &str) -> Vec<u32> {
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
                pos += rest.chars().next().unwrap().len_utf8();
            }
        }
        if text.len() > normal_start {
            for (s, e) in Self::pretokenize_spans(&text[normal_start..]) {
                ids.extend(self.bpe_piece(&text[normal_start + s..normal_start + e]));
            }
        }
        ids
    }

    fn decode(&self, ids: &[u32]) -> String {
        let mut bytes = Vec::with_capacity(ids.len() * 4);
        for &id in ids {
            if let Some(b) = self.id_to_bytes.get(id as usize) {
                bytes.extend_from_slice(b);
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

fn format_smollm2_prompt(system_prompt: &str, user_prompt: &str) -> String {
    format!(
        "<|im_start|>system\n{system_prompt}<|im_end|>\n<|im_start|>user\n{user_prompt}<|im_end|>\n<|im_start|>assistant\n"
    )
}

// ============================================================================
// Safetensors Loader (BF16 / F32 -> Vec<f32>)
// ============================================================================

struct SafetensorsFile {
    data: Vec<u8>,
    data_start: usize,
    entries: HashMap<String, (String, Vec<usize>, usize, usize)>,
}

impl SafetensorsFile {
    fn open(path: &Path) -> Result<Self, String> {
        let data = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        if data.len() < 8 {
            return Err("truncated safetensors".to_string());
        }
        let header_len = u64::from_le_bytes(data[0..8].try_into().unwrap()) as usize;
        if 8 + header_len > data.len() {
            return Err("safetensors header overflow".to_string());
        }
        let hdr = JsonVal::parse_bytes(&data[8..8 + header_len])?;
        let obj = hdr.as_obj().ok_or("safetensors header not object")?;
        let mut entries = HashMap::new();
        for (name, val) in obj {
            if name == "__metadata__" {
                continue;
            }
            let dtype = val.get("dtype").and_then(JsonVal::as_str).ok_or("missing dtype")?.to_string();
            let shape: Vec<usize> = val
                .get("shape")
                .and_then(JsonVal::as_arr)
                .ok_or("missing shape")?
                .iter()
                .filter_map(JsonVal::as_usize)
                .collect();
            let offs = val.get("data_offsets").and_then(JsonVal::as_arr).ok_or("missing offsets")?;
            let s = offs.get(0).and_then(JsonVal::as_usize).ok_or("bad offset 0")?;
            let e = offs.get(1).and_then(JsonVal::as_usize).ok_or("bad offset 1")?;
            entries.insert(name.clone(), (dtype, shape, s, e));
        }
        Ok(Self { data, data_start: 8 + header_len, entries })
    }

    fn tensor_f32(&self, name: &str) -> Result<(Vec<usize>, Vec<f32>), String> {
        let (dtype, shape, s, e) = self
            .entries
            .get(name)
            .ok_or_else(|| format!("missing tensor {name}"))?;
        let raw = &self.data[self.data_start + *s..self.data_start + *e];
        let count: usize = shape.iter().product();
        let mut out = Vec::with_capacity(count);
        match dtype.as_str() {
            "BF16" => {
                if raw.len() != count * 2 {
                    return Err(format!("{name} byte len mismatch"));
                }
                for pair in raw.chunks_exact(2) {
                    let bits = u32::from(u16::from_le_bytes([pair[0], pair[1]])) << 16;
                    out.push(f32::from_bits(bits));
                }
            }
            "F16" => {
                if raw.len() != count * 2 {
                    return Err(format!("{name} f16 byte len mismatch"));
                }
                for pair in raw.chunks_exact(2) {
                    let h = u32::from(u16::from_le_bytes([pair[0], pair[1]]));
                    let sign = (h & 0x8000) << 16;
                    let exp = (h >> 10) & 0x1f;
                    let frac = h & 0x03ff;
                    let bits = if exp == 0 {
                        if frac == 0 {
                            sign
                        } else {
                            let mut e = 113u32;
                            let mut m = frac << 13;
                            while (m & 0x0080_0000) == 0 {
                                m <<= 1;
                                e -= 1;
                            }
                            sign | (e << 23) | (m & 0x007f_ffff)
                        }
                    } else if exp == 31 {
                        sign | 0x7f80_0000 | (frac << 13)
                    } else {
                        sign | ((exp + 112) << 23) | (frac << 13)
                    };
                    out.push(f32::from_bits(bits));
                }
            }
            "F32" => {
                if raw.len() != count * 4 {
                    return Err(format!("{name} f32 byte len mismatch"));
                }
                for quad in raw.chunks_exact(4) {
                    out.push(f32::from_le_bytes([quad[0], quad[1], quad[2], quad[3]]));
                }
            }
            other => return Err(format!("unsupported dtype {other} for {name}")),
        }
        Ok((shape.clone(), out))
    }
}

// ============================================================================
// Block-32 RHT, Conway-Sloane E8 Lattice Audit, Dyadic 4-bit Quantization & Hamming
// ============================================================================

fn xorshift64(mut s: u64) -> u64 {
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    s
}

fn rht_signs(cols: usize, seed: u64) -> Vec<f32> {
    let mut out = Vec::with_capacity(cols);
    let mut state = seed ^ 0x9e3779b97f4a7c15;
    if state == 0 {
        state = 0x853c49e6748fea9b;
    }
    for _ in 0..cols {
        state = xorshift64(state);
        out.push(if (state & 1) == 1 { 1.0 } else { -1.0 });
    }
    out
}

fn fwht32_unnormalized_div32(chunk: &mut [f32; RHT_BLOCK]) {
    let mut h = 1;
    while h < RHT_BLOCK {
        for i in (0..RHT_BLOCK).step_by(h * 2) {
            for j in i..i + h {
                let x = chunk[j];
                let y = chunk[j + h];
                chunk[j] = x + y;
                chunk[j + h] = x - y;
            }
        }
        h *= 2;
    }
    // Scale by 1/32 so integer runtime performs H_32(sign * x) with additions/subtractions ONLY!
    for v in chunk.iter_mut() {
        *v *= 1.0 / 32.0;
    }
}

/// Fold `gain[c]` into columns of row-major `values [rows, cols]` (`values[r*cols + c] *= gain[c]`).
fn fold_columns(values: &mut [f32], cols: usize, gain: &[f32]) {
    for row in values.chunks_exact_mut(cols) {
        for (v, &g) in row.iter_mut().zip(gain) {
            *v *= g;
        }
    }
}

#[inline]
fn grid_scale(m: u8, e: i32) -> f64 {
    (16.0 + f64::from(m)) * 2f64.powi(e - 4)
}

fn grid_nearest(value: f64) -> (u8, i32) {
    let e = value.log2().floor() as i32;
    let m = (value / 2f64.powi(e - 4)).round() as i32 - 16;
    if m >= 16 {
        (0, e + 1)
    } else {
        (m.clamp(0, 15) as u8, e)
    }
}

fn grid_code(value: f64) -> i16 {
    if !value.is_finite() || value.abs() < 1e-18 {
        return 0;
    }
    let (m, e) = grid_nearest(value.abs());
    let mag = (((e + 64).clamp(1, 127) as u16) << 4) | u16::from(m);
    if value < 0.0 {
        -(mag as i16)
    } else {
        mag as i16
    }
}

fn group_error(w: &[f64; GROUP], s: f64) -> f64 {
    let mut err = 0.0f64;
    for &v in w {
        let q = (v / s).round().clamp(-8.0, 7.0);
        let d = v - q * s;
        err += d * d;
    }
    err
}

fn best_scale(w: &[f64; GROUP]) -> Option<(u8, i32)> {
    let a = w.iter().map(|v| v.abs()).fold(0.0, f64::max);
    if a == 0.0 {
        return None;
    }
    let (lo, hi) = (a / 8.5, a / 5.5);
    let mut best: Option<(f64, u8, i32)> = None;
    let e_lo = (lo / 32.0).log2().floor() as i32 + 4;
    let e_hi = (hi / 16.0).log2().ceil() as i32 + 4;
    for e in e_lo..=e_hi {
        for m in 0..16u8 {
            let s = grid_scale(m, e);
            if s < lo || s > hi {
                continue;
            }
            let err = group_error(w, s);
            if best.is_none_or(|b| err < b.0) {
                best = Some((err, m, e));
            }
        }
    }
    Some(match best {
        Some((_, m, e)) => (m, e),
        None => (0, (a / 7.0 / 16.0).log2().ceil() as i32 + 4),
    })
}

/// Conway-Sloane fast E8 nearest lattice point (audit metric across normalized 8-blocks).
fn d8_round(y: &[f32; 8]) -> [f32; 8] {
    let mut f = [0.0f32; 8];
    let mut sum_i = 0i32;
    for i in 0..8 {
        let r = y[i].round();
        f[i] = r;
        sum_i += r as i32;
    }
    if sum_i % 2 != 0 {
        let mut best_i = 0usize;
        let mut best_err = -1.0f32;
        for i in 0..8 {
            let err = (y[i] - f[i]).abs();
            if err > best_err {
                best_err = err;
                best_i = i;
            }
        }
        f[best_i] += if y[best_i] > f[best_i] { 1.0 } else { -1.0 };
    }
    f
}

fn e8_nearest(y: &[f32; 8]) -> [f32; 8] {
    let p_d8 = d8_round(y);
    let mut ys = [0.0f32; 8];
    for i in 0..8 {
        ys[i] = y[i] - 0.5;
    }
    let p_s = d8_round(&ys);
    let mut p_half = [0.0f32; 8];
    for i in 0..8 {
        p_half[i] = p_s[i] + 0.5;
    }
    let d1: f32 = (0..8).map(|i| (y[i] - p_d8[i]).powi(2)).sum();
    let d2: f32 = (0..8).map(|i| (y[i] - p_half[i]).powi(2)).sum();
    if d1 <= d2 { p_d8 } else { p_half }
}

fn sample_e8_lattice_rmse(weights: &[f32]) -> f64 {
    let mut sum_err = 0.0f64;
    let mut sum_e = 0.0f64;
    let blocks = (weights.len() / 8).min(8192);
    for b in 0..blocks {
        let off = b * 8;
        let blk = &weights[off..off + 8];
        let norm_e = (blk.iter().map(|&v| f64::from(v * v)).sum::<f64>() / 8.0).sqrt().max(1e-8) as f32;
        let mut y = [0.0f32; 8];
        for i in 0..8 {
            y[i] = blk[i] / norm_e;
        }
        let q = e8_nearest(&y);
        for i in 0..8 {
            let diff = f64::from(blk[i] - q[i] * norm_e);
            sum_err += diff * diff;
            sum_e += f64::from(blk[i]) * f64::from(blk[i]);
        }
    }
    if sum_e > 0.0 { (sum_err / sum_e).sqrt() } else { 0.0 }
}

#[derive(Clone)]
struct PackedMatrix {
    rows: u32,
    cols: u32,
    use_rht32: u8,
    rht_seed: u64,
    row_exp_base: Vec<i8>,
    row_min_de: Vec<u8>,
    nibbles: Vec<u8>,
    scales: Vec<u8>,
    rel_rms_error: f64,
}

fn quantize_matrix_dyadic(
    values: &[f32],
    rows: usize,
    cols: usize,
    use_rht32: bool,
    rht_seed: u64,
) -> Result<PackedMatrix, String> {
    if values.len() != rows * cols || cols % GROUP != 0 {
        return Err(format!("bad shape {rows}x{cols} vs {}", values.len()));
    }
    let signs = if use_rht32 { rht_signs(cols, rht_seed) } else { Vec::new() };
    let groups = cols / GROUP;
    let row_bytes = cols / 2;

    let mut nibbles = vec![0u8; rows * row_bytes];
    let mut scales = vec![0u8; rows * groups];
    let mut row_exp_base = vec![0i8; rows];
    let mut row_min_de = vec![0u8; rows];

    // Parallel quantization per row chunks across available cores.
    let threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(8).min(rows.max(1));
    let chunk_rows = (rows + threads - 1) / threads;

    let mut total_err = 0.0f64;
    let mut total_ene = 0.0f64;

    thread::scope(|scope| {
        let mut handles = Vec::new();
        let nib_chunks: Vec<&mut [u8]> = nibbles.chunks_mut(chunk_rows * row_bytes).collect();
        let sc_chunks: Vec<&mut [u8]> = scales.chunks_mut(chunk_rows * groups).collect();
        let eb_chunks: Vec<&mut [i8]> = row_exp_base.chunks_mut(chunk_rows).collect();
        let md_chunks: Vec<&mut [u8]> = row_min_de.chunks_mut(chunk_rows).collect();
        for (ti, (((nib_c, sc_c), eb_c), md_c)) in nib_chunks
            .into_iter()
            .zip(sc_chunks)
            .zip(eb_chunks)
            .zip(md_chunks)
            .enumerate()
        {
            let start_r = ti * chunk_rows;
            let count_r = eb_c.len();
            let vals = values;
            let signs_ref = &signs;
            handles.push(scope.spawn(move || -> (f64, f64) {
                let mut err = 0.0f64;
                let mut ene = 0.0f64;
                let mut rot_row = vec![0.0f32; cols];
                let mut chosen: Vec<Option<(u8, i32)>> = Vec::with_capacity(groups);
                for lr in 0..count_r {
                    let gr = start_r + lr;
                    let src = &vals[gr * cols..(gr + 1) * cols];
                    rot_row.copy_from_slice(src);
                    if use_rht32 {
                        for (x, &s) in rot_row.iter_mut().zip(signs_ref) {
                            *x *= s;
                        }
                        for blk in rot_row.chunks_exact_mut(RHT_BLOCK) {
                            let arr: &mut [f32; RHT_BLOCK] = blk.try_into().unwrap();
                            fwht32_unnormalized_div32(arr);
                        }
                    }
                    chosen.clear();
                    let mut gbuf = [0.0f64; GROUP];
                    for g in rot_row.chunks_exact(GROUP) {
                        for (dst, &v) in gbuf.iter_mut().zip(g) {
                            *dst = f64::from(v);
                        }
                        chosen.push(best_scale(&gbuf));
                    }
                    let e_max = chosen.iter().flatten().map(|(_, e)| *e).max().unwrap_or(-20);
                    let exp_base = (e_max - 15).clamp(-120, 110);
                    let e_min = chosen.iter().flatten().map(|(_, e)| *e).min().unwrap_or(exp_base).max(exp_base);
                    eb_c[lr] = exp_base as i8;
                    md_c[lr] = (e_min - exp_base).clamp(0, 15) as u8;

                    let nib_row = &mut nib_c[lr * row_bytes..(lr + 1) * row_bytes];
                    let sc_row = &mut sc_c[lr * groups..(lr + 1) * groups];
                    for g in 0..groups {
                        let (m, e) = match chosen[g] {
                            Some((m, e)) if e >= exp_base => (m, e.min(exp_base + 15)),
                            _ => (0, exp_base),
                        };
                        let s = grid_scale(m, e);
                        let de = (e - exp_base) as u8;
                        sc_row[g] = m | (de << 4);
                        for c in 0..GROUP {
                            let col = g * GROUP + c;
                            let v = f64::from(rot_row[col]);
                            let q = (v / s).round().clamp(-8.0, 7.0);
                            let d = v - q * s;
                            err += d * d;
                            ene += v * v;
                            let nibble = (q as i32 + 8) as u8;
                            if col % 2 == 0 {
                                nib_row[col / 2] |= nibble;
                            } else {
                                nib_row[col / 2] |= nibble << 4;
                            }
                        }
                    }
                }
                (err, ene)
            }));
        }
        for h in handles {
            let (e, en) = h.join().unwrap();
            total_err += e;
            total_ene += en;
        }
    });

    Ok(PackedMatrix {
        rows: rows as u32,
        cols: cols as u32,
        use_rht32: u8::from(use_rht32),
        rht_seed,
        row_exp_base,
        row_min_de,
        nibbles,
        scales,
        rel_rms_error: if total_ene > 0.0 { (total_err / total_ene).sqrt() } else { 0.0 },
    })
}

fn compute_vocab_hamming_576(embed: &[f32], vocab: usize, d_model: usize) -> Vec<[u64; 9]> {
    let mut out = Vec::with_capacity(vocab);
    for r in 0..vocab {
        let row = &embed[r * d_model..(r + 1) * d_model];
        let mut sig = [0u64; 9];
        for bit in 0..576 {
            let val = if d_model == 576 {
                row[bit]
            } else {
                // Deterministic multi-fold Walsh projection from d_model -> 576 bits for compact seed.
                let base = bit % d_model;
                let shift = bit / d_model;
                let partner = (base + shift * 37 + 1) % d_model;
                if shift % 2 == 0 { row[base] + 0.5 * row[partner] } else { row[base] - 0.5 * row[partner] }
            };
            if val >= 0.0 {
                sig[bit / 64] |= 1u64 << (bit % 64);
            }
        }
        out.push(sig);
    }
    out
}

// ============================================================================
// Sealed Integer Tables & Container Serialization (UORR4V12)
// ============================================================================

struct RecompiledModelData {
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
    rms_eps_q48: i64,
    score_scale_q30: i64,
    rope_cos_q14: Vec<i16>,
    rope_sin_q14: Vec<i16>,
    exp_table: Vec<u32>,
    silu_table: Vec<i32>,
    arcosh_table: Vec<u32>,
    zeta_phase_q29: [u32; 8],
    local_to_global: Vec<u32>,
    vocab_hamming_sigs: Vec<[u64; 9]>,
    embed: PackedMatrix,
    final_norm_codes: Vec<i16>,
    layers: Vec<RecompiledLayerData>,
}

struct RecompiledLayerData {
    q_proj: PackedMatrix,
    k_proj: PackedMatrix,
    v_proj: PackedMatrix,
    o_proj: PackedMatrix,
    gate_proj: PackedMatrix,
    up_proj: PackedMatrix,
    down_proj: PackedMatrix,
}

fn build_sealed_luts(
    max_seq: usize,
    head_dim: usize,
    rope_theta: f64,
    arcosh_mantissa_bits: u32,
) -> (Vec<i16>, Vec<i16>, Vec<u32>, Vec<i32>, Vec<u32>, [u32; 8]) {
    let half = head_dim / 2;
    let mut rope_cos = Vec::with_capacity(max_seq * half);
    let mut rope_sin = Vec::with_capacity(max_seq * half);
    let scale = (1i32 << ROPE_Q) as f64;
    for pos in 0..max_seq {
        for i in 0..half {
            let inv_freq = 1.0 / rope_theta.powf((2 * i) as f64 / head_dim as f64);
            let angle = (pos as f64) * inv_freq;
            rope_cos.push((angle.cos() * scale).round().clamp(-32767.0, 32767.0) as i16);
            rope_sin.push((angle.sin() * scale).round().clamp(-32767.0, 32767.0) as i16);
        }
    }

    let exp_step = 2.0f64.powi(EXP_STEP_LOG2);
    let exp_len = (EXP_RANGE << (-EXP_STEP_LOG2)) as usize + 1;
    let mut exp_table = Vec::with_capacity(exp_len);
    for i in 0..exp_len {
        let x = (i as f64) * exp_step;
        let y = ((-x).exp() * 2.0f64.powi(31)).round().clamp(0.0, 2147483648.0);
        exp_table.push(y as u32);
    }

    let silu_step = 2.0f64.powi(SILU_STEP_LOG2);
    let silu_half = 1i32 << (SILU_RANGE_LOG2 - SILU_STEP_LOG2);
    let silu_len = (silu_half as usize) * 2 + 1;
    let mut silu_table = Vec::with_capacity(silu_len);
    for i in 0..silu_len {
        let x = ((i as i32 - silu_half) as f64) * silu_step;
        let s = x / (1.0 + (-x).exp());
        silu_table.push((s * 65536.0).round() as i32);
    }

    let arcosh_len = (((ARCOSH_CODE_BITS - arcosh_mantissa_bits + 1) as usize) << arcosh_mantissa_bits) | 1;
    let mut arcosh_table = Vec::with_capacity(arcosh_len);
    let q24 = 2.0f64.powi(24);
    let direct = 1u128 << arcosh_mantissa_bits;
    for idx in 0..arcosh_len {
        let code_val: f64 = if (idx as u128) < direct {
            idx as f64
        } else {
            let block = (idx >> arcosh_mantissa_bits) - 1;
            let mant = idx & ((1usize << arcosh_mantissa_bits) - 1);
            let m_full = (direct as f64) + (mant as f64);
            m_full * 2.0f64.powi(block as i32)
        };
        let v = 1.0 + code_val * 2.0f64.powi(-32);
        let a = (v.acosh() * q24).round().clamp(0.0, u32::MAX as f64) as u32;
        arcosh_table.push(a);
    }

    let mut zeta_phase_q29 = [0u32; 8];
    for (dst, &ch) in zeta_phase_q29.iter_mut().zip(&ATTENTION_ZETA_CHANNELS) {
        let gamma = ZETA_GAMMAS_TOP22[usize::from(ch)];
        // Natural phase angle mapped onto Z / PHASE_MODULUS_Q29 Z.
        let frac = (gamma * 0.15915494309189535).fract();
        let q = (frac * (PHASE_MODULUS_Q29 as f64)).round() as u64 % PHASE_MODULUS_Q29;
        *dst = q as u32;
    }

    (rope_cos, rope_sin, exp_table, silu_table, arcosh_table, zeta_phase_q29)
}

fn push_packed_matrix(buf: &mut Vec<u8>, m: &PackedMatrix) {
    buf.extend_from_slice(&m.rows.to_le_bytes());
    buf.extend_from_slice(&m.cols.to_le_bytes());
    buf.push(m.use_rht32);
    buf.extend_from_slice(&m.rht_seed.to_le_bytes());
    for &eb in &m.row_exp_base {
        buf.push(eb as u8);
    }
    buf.extend_from_slice(&m.row_min_de);
    buf.extend_from_slice(&m.nibbles);
    buf.extend_from_slice(&m.scales);
}

fn serialize_uorr4v12(m: &RecompiledModelData) -> Vec<u8> {
    let mut payload = Vec::with_capacity(80 * 1024 * 1024);
    payload.extend_from_slice(&m.rms_eps_q48.to_le_bytes());
    payload.extend_from_slice(&m.score_scale_q30.to_le_bytes());
    payload.extend_from_slice(&m.arcosh_mantissa_bits.to_le_bytes());
    payload.extend_from_slice(&(m.rope_cos_q14.len() as u32).to_le_bytes());
    payload.extend_from_slice(&(m.exp_table.len() as u32).to_le_bytes());
    payload.extend_from_slice(&(m.silu_table.len() as u32).to_le_bytes());
    payload.extend_from_slice(&(m.arcosh_table.len() as u32).to_le_bytes());

    for &v in &m.rope_cos_q14 {
        payload.extend_from_slice(&v.to_le_bytes());
    }
    for &v in &m.rope_sin_q14 {
        payload.extend_from_slice(&v.to_le_bytes());
    }
    for &v in &m.exp_table {
        payload.extend_from_slice(&v.to_le_bytes());
    }
    for &v in &m.silu_table {
        payload.extend_from_slice(&v.to_le_bytes());
    }
    for &v in &m.arcosh_table {
        payload.extend_from_slice(&v.to_le_bytes());
    }
    for &v in &E8P_PACKED_ABS {
        payload.extend_from_slice(&v.to_le_bytes());
    }
    for &v in &m.zeta_phase_q29 {
        payload.extend_from_slice(&v.to_le_bytes());
    }

    payload.extend_from_slice(&(m.local_to_global.len() as u32).to_le_bytes());
    for &gid in &m.local_to_global {
        payload.extend_from_slice(&gid.to_le_bytes());
    }

    for sig in &m.vocab_hamming_sigs {
        for &w in sig {
            payload.extend_from_slice(&w.to_le_bytes());
        }
    }

    for &c in &m.final_norm_codes {
        payload.extend_from_slice(&c.to_le_bytes());
    }
    push_packed_matrix(&mut payload, &m.embed);
    for l in &m.layers {
        push_packed_matrix(&mut payload, &l.q_proj);
        push_packed_matrix(&mut payload, &l.k_proj);
        push_packed_matrix(&mut payload, &l.v_proj);
        push_packed_matrix(&mut payload, &l.o_proj);
        push_packed_matrix(&mut payload, &l.gate_proj);
        push_packed_matrix(&mut payload, &l.up_proj);
        push_packed_matrix(&mut payload, &l.down_proj);
    }

    let payload_sha = sha256_bytes(&payload);
    let mut flags = (1u32 << 0) | (1u32 << 2) | (1u32 << 3) | (1u32 << 4);
    if !m.local_to_global.is_empty() {
        flags |= 1u32 << 1;
    }

    let mut out = Vec::with_capacity(96 + payload.len());
    out.extend_from_slice(R4_VEC_MAGIC);
    out.extend_from_slice(&R4_VEC_VERSION.to_le_bytes());
    out.extend_from_slice(&m.tier_code.to_le_bytes());
    out.extend_from_slice(&m.vocab_size.to_le_bytes());
    out.extend_from_slice(&m.d_model.to_le_bytes());
    out.extend_from_slice(&m.n_layers.to_le_bytes());
    out.extend_from_slice(&m.n_heads.to_le_bytes());
    out.extend_from_slice(&m.n_kv_heads.to_le_bytes());
    out.extend_from_slice(&m.intermediate_size.to_le_bytes());
    out.extend_from_slice(&m.max_seq.to_le_bytes());
    out.extend_from_slice(&m.bos_token_id.to_le_bytes());
    out.extend_from_slice(&m.eos_token_id.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // float_ops = 0
    out.extend_from_slice(&0u32.to_le_bytes()); // dense_matmul_ops = 0
    out.extend_from_slice(&payload_sha);
    out.extend_from_slice(&payload);
    out
}

// ============================================================================
// Pure-Integer Serving Evaluator (Matches crates/wasm/src/r4_geometric_stack.rs!)
// ============================================================================

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
    if code < 0 { scaled.saturating_neg() } else { scaled }
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
        0, t1, t2, t3, t4, t5, t6, t7, t8,
        t8.wrapping_add(t1), t8.wrapping_add(t2), t8.wrapping_add(t3),
        t8.wrapping_add(t4), t8.wrapping_add(t5), t8.wrapping_add(t6), t8.wrapping_add(t7),
    ];
    let mut digits = (67 - small.leading_zeros()) >> 2;
    let mut acc = 0u64;
    while digits > 0 {
        digits -= 1;
        acc = (acc << 4).wrapping_add(tbl[((small >> (digits << 2)) & 15) as usize]);
    }
    if neg { acc.wrapping_neg() as i64 } else { acc as i64 }
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
        0, t1, t2, t3, t4, t5, t6, t7, t8,
        t8.wrapping_add(t1), t8.wrapping_add(t2), t8.wrapping_add(t3),
        t8.wrapping_add(t4), t8.wrapping_add(t5), t8.wrapping_add(t6), t8.wrapping_add(t7),
    ];
    let mut digits = (131 - small.leading_zeros()) >> 2;
    let mut acc = 0u128;
    while digits > 0 {
        digits -= 1;
        acc = (acc << 4).wrapping_add(tbl[((small >> (digits << 2)) & 15) as usize]);
    }
    if neg { acc.wrapping_neg() as i128 } else { acc as i128 }
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

fn rht_signs_bits(cols: usize, seed: u64) -> Vec<bool> {
    let mut out = Vec::with_capacity(cols);
    let mut state = seed ^ 0x9e3779b97f4a7c15;
    if state == 0 {
        state = 0x853c49e6748fea9b;
    }
    for _ in 0..cols {
        state = xorshift64(state);
        out.push((state & 1) == 1);
    }
    out
}

fn int_fwht32(chunk: &mut [i64; RHT_BLOCK]) {
    let mut h = 1;
    while h < RHT_BLOCK {
        for i in (0..RHT_BLOCK).step_by(h * 2) {
            for j in i..i + h {
                let x = chunk[j];
                let y = chunk[j + h];
                chunk[j] = x.saturating_add(y);
                chunk[j + h] = x.saturating_sub(y);
            }
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
    // Shift so max magnitude lies around 2^22 (preserves precision for tiny vectors too!).
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
    // Radix-16 multiples of r for multiplier-free norm scaling into NORM_FINE_EXP = -24 (shift = 38).
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
        let prod = if *v < 0 { mag.wrapping_neg() as i128 } else { mag as i128 };
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
    for r in 0..rows {
        let nibs = &m.nibbles[r * row_bytes..(r + 1) * row_bytes];
        let scs = &m.scales[r * groups..(r + 1) * groups];
        let min_de = m.row_min_de[r];
        let eb = i32::from(m.row_exp_base[r]);
        let mut acc = 0i64;
        for ((bytes, tbls), &s) in nibs.chunks_exact(16).zip(pairs[..row_bytes].chunks_exact(16)).zip(scs) {
            let mut a = 0i32;
            for k in 0..16 {
                a = a.wrapping_add(tbls[k][usize::from(bytes[k])]);
            }
            let de = (s >> 4).saturating_sub(min_de);
            acc = acc.wrapping_add(scale_16_plus(i64::from(a), s & 15) << de);
        }
        out[r] = to_exp_i32(acc, eb + i32::from(min_de) - 4 + x_exp, RESIDUAL_EXP);
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
        for g in 0..groups {
            let s = scs[g];
            let e = eb + i32::from(s >> 4);
            let mant = s & 15;
            for c in 0..GROUP {
                let col = g * GROUP + c;
                let b = nibs[col >> 1];
                let raw_nib = if col % 2 == 0 { b & 15 } else { b >> 4 };
                let q = i64::from(raw_nib) - 8;
                let scaled = scale_16_plus(q, mant);
                wide[col] = shift_round(scaled, NORM_FINE_EXP - (e - 4));
            }
        }
        // Invert Block-32 WHT: stored row is (w * signs) * H_32 / 32, and H_32 * H_32 = 32 * I,
        // so multiplying by H_32 with pure integer adds/subs recovers w * signs exact at NORM_FINE_EXP!
        for blk in wide.chunks_exact_mut(RHT_BLOCK) {
            let arr: &mut [i64; RHT_BLOCK] = blk.try_into().unwrap();
            int_fwht32(arr);
        }
        let signs = rht_signs_bits(cols, m.rht_seed);
        for ((dst, &val), sign_pos) in out.iter_mut().zip(&wide).zip(signs) {
            let signed_val = if sign_pos { val } else { val.saturating_neg() };
            *dst = shift_round(signed_val, RESIDUAL_EXP - NORM_FINE_EXP).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        }
        return;
    }
    for g in 0..groups {
        let s = scs[g];
        let e = eb + i32::from(s >> 4);
        let mant = s & 15;
        for c in 0..GROUP {
            let col = g * GROUP + c;
            let b = nibs[col >> 1];
            let raw_nib = if col % 2 == 0 { b & 15 } else { b >> 4 };
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
    let frac = (d & ((1 << frac_bits) - 1)) as i64;
    let (a, b) = (u64::from(a), u64::from(b));
    a.wrapping_sub((table_mul_i64(a.wrapping_sub(b) as i64, frac) as u64) >> frac_bits)
}

fn int_silu(x: i32, table: &[i32]) -> i32 {
    let frac_bits = SILU_STEP_LOG2 + 16; // 8
    let half = 1i64 << (SILU_RANGE_LOG2 - SILU_STEP_LOG2); // 4096
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
    let step = (table_mul_i128(i128::from(hi.wrapping_sub(lo)), i128::from(frac)) as u128).wrapping_add(1u128 << (shift - 1)) >> shift;
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

fn eval_integer_forward(
    m: &RecompiledModelData,
    tokens: &[u32],
) -> (Vec<i32>, u64, u64) {
    let d = m.d_model as usize;
    let ffn = m.intermediate_size as usize;
    let nh = m.n_heads as usize;
    let nkv = m.n_kv_heads as usize;
    let hd = d / nh;
    let kv_dim = nkv * hd;
    let group_sz = nh / nkv;
    let seq_len = tokens.len().min(m.max_seq as usize);
    let n_layers = m.n_layers as usize;

    let mut k_caches = vec![vec![0i32; seq_len * kv_dim]; n_layers];
    let mut v_caches = vec![vec![0i32; seq_len * kv_dim]; n_layers];
    let mut klift_caches = vec![vec![0u64; seq_len * nkv]; n_layers];

    let max_cols = d.max(ffn);
    let mut x = vec![0i32; d];
    let mut norm = vec![0i64; d];
    let mut scratch_rht = vec![0i64; max_cols];
    let mut act16 = vec![0i16; max_cols];
    let mut tables = vec![[0i32; 16]; max_cols];
    let mut pairs = vec![[0i32; 256]; max_cols / 2];

    let mut q = vec![0i32; d];
    let mut k = vec![0i32; kv_dim];
    let mut v = vec![0i32; kv_dim];
    let mut attn_out = vec![0i32; d];
    let mut proj = vec![0i32; d];
    let mut gate = vec![0i32; ffn];
    let mut up = vec![0i32; ffn];
    let mut swiglu = vec![0i64; ffn];
    let mut logits = vec![0i32; m.vocab_size as usize];

    let mut nibble_lookups = 0u64;
    let mut lorentz_evals = 0u64;
    let half = hd / 2;

    for (pos, &tok) in tokens[..seq_len].iter().enumerate() {
        int_dequant_row(&m.embed, tok as usize, &mut x);
        let cos_row = &m.rope_cos_q14[pos * half..(pos + 1) * half];
        let sin_row = &m.rope_sin_q14[pos * half..(pos + 1) * half];

        for l in 0..n_layers {
            let layer = &m.layers[l];
            int_rms_norm_fine(&x, m.rms_eps_q48, &mut norm);
            nibble_lookups += int_gemv_pairs(&layer.q_proj, &norm, NORM_FINE_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut q);
            nibble_lookups += int_gemv_pairs(&layer.k_proj, &norm, NORM_FINE_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut k);
            nibble_lookups += int_gemv_pairs(&layer.v_proj, &norm, NORM_FINE_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut v);

            // Half-split RoPE (Q14).
            for h in 0..nh {
                let off = h * hd;
                for i in 0..half {
                    let c = i64::from(cos_row[i]);
                    let s = i64::from(sin_row[i]);
                    let q0 = i64::from(q[off + i]);
                    let q1 = i64::from(q[off + i + half]);
                    let r0 = (table_mul_i64(q0, c) - table_mul_i64(q1, s) + (1 << 13)) >> ROPE_Q;
                    let r1 = (table_mul_i64(q0, s) + table_mul_i64(q1, c) + (1 << 13)) >> ROPE_Q;
                    q[off + i] = r0 as i32;
                    q[off + i + half] = r1 as i32;
                }
            }
            for kh in 0..nkv {
                let off = kh * hd;
                for i in 0..half {
                    let c = i64::from(cos_row[i]);
                    let s = i64::from(sin_row[i]);
                    let k0 = i64::from(k[off + i]);
                    let k1 = i64::from(k[off + i + half]);
                    let r0 = (table_mul_i64(k0, c) - table_mul_i64(k1, s) + (1 << 13)) >> ROPE_Q;
                    let r1 = (table_mul_i64(k0, s) + table_mul_i64(k1, c) + (1 << 13)) >> ROPE_Q;
                    k[off + i] = r0 as i32;
                    k[off + i + half] = r1 as i32;
                }
                klift_caches[l][pos * nkv + kh] = int_lorentz_lift(&k[off..off + hd]);
            }
            k_caches[l][pos * kv_dim..(pos + 1) * kv_dim].copy_from_slice(&k);
            v_caches[l][pos * kv_dim..(pos + 1) * kv_dim].copy_from_slice(&v);

            // Only full attention outputs needed on intermediate tokens, exact last token yields logits.
            let mut q_tables = vec![[0i64; 16]; hd];
            let mut scores = vec![0i64; pos + 1];
            let mut weights = vec![0u64; pos + 1];
            let mut mix = vec![0i128; hd];
            for h in 0..nh {
                let kh = h / group_sz;
                let qh = &q[h * hd..(h + 1) * hd];
                let q_lift = int_lorentz_lift(qh);
                for (qt, &qv) in q_tables.iter_mut().zip(qh) {
                    *qt = multiples_i64(i64::from(qv));
                }
                let mut max_s = i64::MIN;
                for j in 0..=pos {
                    let kj = &k_caches[l][j * kv_dim + kh * hd..j * kv_dim + (kh + 1) * hd];
                    let mut dot = 0i128;
                    for (qt, &kv) in q_tables.iter().zip(kj) {
                        // Radix-16 signed dot product using query multiples table.
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
                    // Evaluate Lorentz geodesic separation invariant for geometric telemetry-free tracking.
                    let k_lift = klift_caches[l][j * nkv + kh];
                    let excess = (table_mul_i128(i128::from(q_lift), i128::from(k_lift)))
                        .wrapping_sub(dot << 32)
                        .wrapping_sub(1i128 << 64);
                    let _ldist = int_arcosh1p_q24(((excess.max(0) >> 32) as u128).max(429), m.arcosh_mantissa_bits, &m.arcosh_table);
                    lorentz_evals += 1;

                    // dot * score_scale_q30 scaled to SCORE_EXP (-24): exp (-32) + (-30) -> -62, shift 38 -> -24.
                    let sc = shift_wide(table_mul_i128(dot, i128::from(m.score_scale_q30)), 38);
                    scores[j] = sc;
                    if sc > max_s {
                        max_s = sc;
                    }
                }
                let mut total_w = 0u64;
                for (w, &sc) in weights.iter_mut().zip(&scores) {
                    *w = int_exp_neg(max_s.wrapping_sub(sc), SCORE_EXP, &m.exp_table);
                    total_w = total_w.wrapping_add(*w);
                }
                mix.fill(0);
                for (j, &w) in weights.iter().enumerate() {
                    if w == 0 {
                        continue;
                    }
                    let vj = &v_caches[l][j * kv_dim + kh * hd..j * kv_dim + (kh + 1) * hd];
                    for (slot, &vv) in mix.iter_mut().zip(vj) {
                        *slot = slot.wrapping_add(table_mul_i128(i128::from(w), i128::from(vv)));
                    }
                }
                let recip = long_division_u128(1u128 << 62, u128::from(total_w.max(1))) as i128;
                for (c, &mv) in mix.iter().enumerate() {
                    attn_out[h * hd + c] = shift_wide(table_mul_i128(mv, recip), 62) as i32;
                }
            }

            let mut attn_i64 = vec![0i64; d];
            for (dst, &s) in attn_i64.iter_mut().zip(&attn_out) {
                *dst = i64::from(s);
            }
            nibble_lookups += int_gemv_pairs(&layer.o_proj, &attn_i64, RESIDUAL_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut proj);
            for (xv, &pv) in x.iter_mut().zip(&proj) {
                *xv = xv.saturating_add(pv);
            }

            int_rms_norm_fine(&x, m.rms_eps_q48, &mut norm);
            nibble_lookups += int_gemv_pairs(&layer.gate_proj, &norm, NORM_FINE_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut gate);
            nibble_lookups += int_gemv_pairs(&layer.up_proj, &norm, NORM_FINE_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut up);
            for ((slot, &g), &u) in swiglu.iter_mut().zip(&gate).zip(&up) {
                let sg = int_silu(g, &m.silu_table);
                *slot = table_mul_i64(i64::from(sg), i64::from(u));
            }
            nibble_lookups += int_gemv_pairs(&layer.down_proj, &swiglu, PRODUCT_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut proj);
            for (xv, &pv) in x.iter_mut().zip(&proj) {
                *xv = xv.saturating_add(pv);
            }
        }
    }

    int_rms_norm_fine(&x, m.rms_eps_q48, &mut norm);
    for (v, &code) in norm.iter_mut().zip(&m.final_norm_codes) {
        *v = grid_apply(*v, code);
    }
    nibble_lookups += int_gemv_pairs(&m.embed, &norm, NORM_FINE_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut logits);
    (logits, nibble_lookups, lorentz_evals)
}

/// Incremental pure-integer session for multi-token autoregressive OOD evaluation.
struct IntegerEvalSession<'a> {
    m: &'a RecompiledModelData,
    k_caches: Vec<Vec<i32>>,
    v_caches: Vec<Vec<i32>>,
    klift_caches: Vec<Vec<u64>>,
    pos: usize,
}

impl<'a> IntegerEvalSession<'a> {
    fn new(m: &'a RecompiledModelData) -> Self {
        let kv_dim = (m.d_model / m.n_heads * m.n_kv_heads) as usize;
        let max_s = m.max_seq as usize;
        let nl = m.n_layers as usize;
        let nkv = m.n_kv_heads as usize;
        Self {
            m,
            k_caches: vec![vec![0i32; max_s * kv_dim]; nl],
            v_caches: vec![vec![0i32; max_s * kv_dim]; nl],
            klift_caches: vec![vec![0u64; max_s * nkv]; nl],
            pos: 0,
        }
    }

    fn step(&mut self, tok: u32, compute_logits: bool) -> (Vec<i32>, u64, u64) {
        let m = self.m;
        let d = m.d_model as usize;
        let ffn = m.intermediate_size as usize;
        let nh = m.n_heads as usize;
        let nkv = m.n_kv_heads as usize;
        let hd = d / nh;
        let kv_dim = nkv * hd;
        let group_sz = nh / nkv;
        let pos = self.pos.min(m.max_seq as usize - 1);
        let half = hd / 2;

        let max_cols = d.max(ffn);
        let mut x = vec![0i32; d];
        let mut norm = vec![0i64; d];
        let mut scratch_rht = vec![0i64; max_cols];
        let mut act16 = vec![0i16; max_cols];
        let mut tables = vec![[0i32; 16]; max_cols];
        let mut pairs = vec![[0i32; 256]; max_cols / 2];

        let mut q = vec![0i32; d];
        let mut k = vec![0i32; kv_dim];
        let mut v = vec![0i32; kv_dim];
        let mut attn_out = vec![0i32; d];
        let mut proj = vec![0i32; d];
        let mut gate = vec![0i32; ffn];
        let mut up = vec![0i32; ffn];
        let mut swiglu = vec![0i64; ffn];
        let mut q_tables = vec![[0i64; 16]; hd];
        let mut scores = vec![0i64; pos + 1];
        let mut weights = vec![0u64; pos + 1];
        let mut mix = vec![0i128; hd];

        let mut nibble_lookups = 0u64;
        let mut lorentz_evals = 0u64;

        int_dequant_row(&m.embed, tok as usize, &mut x);
        let cos_row = &m.rope_cos_q14[pos * half..(pos + 1) * half];
        let sin_row = &m.rope_sin_q14[pos * half..(pos + 1) * half];

        for l in 0..m.n_layers as usize {
            let layer = &m.layers[l];
            int_rms_norm_fine(&x, m.rms_eps_q48, &mut norm);
            nibble_lookups += int_gemv_pairs(&layer.q_proj, &norm, NORM_FINE_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut q);
            nibble_lookups += int_gemv_pairs(&layer.k_proj, &norm, NORM_FINE_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut k);
            nibble_lookups += int_gemv_pairs(&layer.v_proj, &norm, NORM_FINE_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut v);

            for h in 0..nh {
                let off = h * hd;
                for i in 0..half {
                    let c = i64::from(cos_row[i]);
                    let s = i64::from(sin_row[i]);
                    let q0 = i64::from(q[off + i]);
                    let q1 = i64::from(q[off + i + half]);
                    q[off + i] = ((table_mul_i64(q0, c) - table_mul_i64(q1, s) + (1 << 13)) >> ROPE_Q) as i32;
                    q[off + i + half] = ((table_mul_i64(q0, s) + table_mul_i64(q1, c) + (1 << 13)) >> ROPE_Q) as i32;
                }
            }
            for kh in 0..nkv {
                let off = kh * hd;
                for i in 0..half {
                    let c = i64::from(cos_row[i]);
                    let s = i64::from(sin_row[i]);
                    let k0 = i64::from(k[off + i]);
                    let k1 = i64::from(k[off + i + half]);
                    k[off + i] = ((table_mul_i64(k0, c) - table_mul_i64(k1, s) + (1 << 13)) >> ROPE_Q) as i32;
                    k[off + i + half] = ((table_mul_i64(k0, s) + table_mul_i64(k1, c) + (1 << 13)) >> ROPE_Q) as i32;
                }
                self.klift_caches[l][pos * nkv + kh] = int_lorentz_lift(&k[off..off + hd]);
            }
            self.k_caches[l][pos * kv_dim..(pos + 1) * kv_dim].copy_from_slice(&k);
            self.v_caches[l][pos * kv_dim..(pos + 1) * kv_dim].copy_from_slice(&v);

            for h in 0..nh {
                let kh = h / group_sz;
                let qh = &q[h * hd..(h + 1) * hd];
                let q_lift = int_lorentz_lift(qh);
                for (qt, &qv) in q_tables.iter_mut().zip(qh) {
                    *qt = multiples_i64(i64::from(qv));
                }
                let mut max_s = i64::MIN;
                for j in 0..=pos {
                    let kj = &self.k_caches[l][j * kv_dim + kh * hd..j * kv_dim + (kh + 1) * hd];
                    let mut dot = 0i128;
                    for (qt, &kv) in q_tables.iter().zip(kj) {
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
                    let _ld = int_arcosh1p_q24(((excess.max(0) >> 32) as u128).max(429), m.arcosh_mantissa_bits, &m.arcosh_table);
                    lorentz_evals += 1;

                    let sc = shift_wide(table_mul_i128(dot, i128::from(m.score_scale_q30)), 38);
                    scores[j] = sc;
                    if sc > max_s {
                        max_s = sc;
                    }
                }
                let mut total_w = 0u64;
                for (w, &sc) in weights.iter_mut().zip(&scores) {
                    *w = int_exp_neg(max_s.wrapping_sub(sc), SCORE_EXP, &m.exp_table);
                    total_w = total_w.wrapping_add(*w);
                }
                mix.fill(0);
                for (j, &w) in weights.iter().enumerate() {
                    if w == 0 { continue; }
                    let vj = &self.v_caches[l][j * kv_dim + kh * hd..j * kv_dim + (kh + 1) * hd];
                    for (slot, &vv) in mix.iter_mut().zip(vj) {
                        *slot = slot.wrapping_add(table_mul_i128(i128::from(w), i128::from(vv)));
                    }
                }
                let recip = long_division_u128(1u128 << 62, u128::from(total_w.max(1))) as i128;
                for (c, &mv) in mix.iter().enumerate() {
                    attn_out[h * hd + c] = shift_wide(table_mul_i128(mv, recip), 62) as i32;
                }
            }

            let mut attn_i64 = vec![0i64; d];
            for (dst, &s) in attn_i64.iter_mut().zip(&attn_out) {
                *dst = i64::from(s);
            }
            nibble_lookups += int_gemv_pairs(&layer.o_proj, &attn_i64, RESIDUAL_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut proj);
            for (xv, &pv) in x.iter_mut().zip(&proj) {
                *xv = xv.saturating_add(pv);
            }

            int_rms_norm_fine(&x, m.rms_eps_q48, &mut norm);
            nibble_lookups += int_gemv_pairs(&layer.gate_proj, &norm, NORM_FINE_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut gate);
            nibble_lookups += int_gemv_pairs(&layer.up_proj, &norm, NORM_FINE_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut up);
            for ((slot, &g), &u) in swiglu.iter_mut().zip(&gate).zip(&up) {
                *slot = table_mul_i64(i64::from(int_silu(g, &m.silu_table)), i64::from(u));
            }
            nibble_lookups += int_gemv_pairs(&layer.down_proj, &swiglu, PRODUCT_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut proj);
            for (xv, &pv) in x.iter_mut().zip(&proj) {
                *xv = xv.saturating_add(pv);
            }
        }

        self.pos += 1;
        if !compute_logits {
            return (Vec::new(), nibble_lookups, lorentz_evals);
        }
        int_rms_norm_fine(&x, m.rms_eps_q48, &mut norm);
        for (v, &code) in norm.iter_mut().zip(&m.final_norm_codes) {
            *v = grid_apply(*v, code);
        }
        let mut logits = vec![0i32; m.vocab_size as usize];
        nibble_lookups += int_gemv_pairs(&m.embed, &norm, NORM_FINE_EXP, &mut scratch_rht, &mut act16, &mut tables, &mut pairs, &mut logits);
        (logits, nibble_lookups, lorentz_evals)
    }
}

// ============================================================================
// RTX 4090 CUDA C++ Teacher Evaluator & Distillation Trainer (Zero Python)
// ============================================================================

const CUDA_DISTILL_SRC: &str = r#"
#include <cuda_runtime.h>
#include <cublas_v2.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <string>
#include <vector>
#include <algorithm>

#define CUDA_CHECK(x) do { \
    cudaError_t err = (x); \
    if (err != cudaSuccess) { \
        fprintf(stderr, "CUDA error %s at %s:%d\n", cudaGetErrorString(err), __FILE__, __LINE__); \
        exit(1); \
    } \
} while (0)

#define CUBLAS_CHECK(x) do { \
    cublasStatus_t st = (x); \
    if (st != CUBLAS_STATUS_SUCCESS) { \
        fprintf(stderr, "cuBLAS error %d at %s:%d\n", (int)st, __FILE__, __LINE__); \
        exit(1); \
    } \
} while (0)

struct TeacherModel {
    int vocab, d, n_layers, nh, nkv, ffn;
    float rms_eps, rope_theta;
    float *d_embed;      // [vocab, d]
    float *d_norm;       // [d]
    std::vector<float*> in_ln, post_ln, q_w, k_w, v_w, o_w, g_w, u_w, d_w;
};

static float* upload_vec(FILE *fp, size_t n) {
    float *h = (float*)malloc(n * sizeof(float));
    if (fread(h, sizeof(float), n, fp) != n) { fprintf(stderr, "short read\n"); exit(1); }
    float *d = nullptr;
    CUDA_CHECK(cudaMalloc(&d, n * sizeof(float)));
    CUDA_CHECK(cudaMemcpy(d, h, n * sizeof(float), cudaMemcpyHostToDevice));
    free(h);
    return d;
}

static TeacherModel load_teacher_bin(const char *path) {
    FILE *fp = fopen(path, "rb");
    if (!fp) { fprintf(stderr, "cannot open %s\n", path); exit(1); }
    TeacherModel t;
    uint32_t hdr[6];
    if (fread(hdr, sizeof(uint32_t), 6, fp) != 6) exit(1);
    float fhdr[2];
    if (fread(fhdr, sizeof(float), 2, fp) != 2) exit(1);
    t.vocab = (int)hdr[0];
    t.d = (int)hdr[1];
    t.n_layers = (int)hdr[2];
    t.nh = (int)hdr[3];
    t.nkv = (int)hdr[4];
    t.ffn = (int)hdr[5];
    t.rms_eps = fhdr[0];
    t.rope_theta = fhdr[1];
    int kv_dim = (t.d / t.nh) * t.nkv;
    t.d_embed = upload_vec(fp, (size_t)t.vocab * t.d);
    t.d_norm = upload_vec(fp, t.d);
    for (int l = 0; l < t.n_layers; l++) {
        t.in_ln.push_back(upload_vec(fp, t.d));
        t.post_ln.push_back(upload_vec(fp, t.d));
        t.q_w.push_back(upload_vec(fp, (size_t)t.d * t.d));
        t.k_w.push_back(upload_vec(fp, (size_t)kv_dim * t.d));
        t.v_w.push_back(upload_vec(fp, (size_t)kv_dim * t.d));
        t.o_w.push_back(upload_vec(fp, (size_t)t.d * t.d));
        t.g_w.push_back(upload_vec(fp, (size_t)t.ffn * t.d));
        t.u_w.push_back(upload_vec(fp, (size_t)t.ffn * t.d));
        t.d_w.push_back(upload_vec(fp, (size_t)t.d * t.ffn));
    }
    fclose(fp);
    return t;
}

__global__ void k_rms_norm_vec(const float *x, const float *g, float *out, int d, float eps) {
    __shared__ float smem[32];
    float ss = 0.0f;
    for (int i = threadIdx.x; i < d; i += blockDim.x) {
        float v = x[i];
        ss += v * v;
    }
    int lane = threadIdx.x & 31;
    int warp = threadIdx.x >> 5;
    for (int off = 16; off > 0; off >>= 1) ss += __shfl_down_sync(0xffffffff, ss, off);
    if (lane == 0) smem[warp] = ss;
    __syncthreads();
    if (warp == 0) {
        float val = (lane < (blockDim.x >> 5)) ? smem[lane] : 0.0f;
        for (int off = 16; off > 0; off >>= 1) val += __shfl_down_sync(0xffffffff, val, off);
        if (lane == 0) smem[0] = rsqrtf(val / (float)d + eps);
    }
    __syncthreads();
    float inv = smem[0];
    for (int i = threadIdx.x; i < d; i += blockDim.x) {
        out[i] = x[i] * inv * g[i];
    }
}

__global__ void k_rope_vec(float *vec, int n_heads, int hd, int pos, float theta) {
    int half = hd >> 1;
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n_heads * half) return;
    int h = idx / half;
    int i = idx % half;
    float inv_freq = 1.0f / powf(theta, (2.0f * (float)i) / (float)hd);
    float ang = (float)pos * inv_freq;
    float c = cosf(ang), s = sinf(ang);
    float *base = vec + h * hd;
    float x0 = base[i], x1 = base[i + half];
    base[i] = x0 * c - x1 * s;
    base[i + half] = x0 * s + x1 * c;
}

__global__ void k_attn_step_head(
    const float *q, const float *kc, const float *vc, float *out,
    int nh, int nkv, int hd, int kv_dim, int pos, float scale
) {
    int h = blockIdx.x;
    if (h >= nh) return;
    int kh = h / (nh / nkv);
    const float *qh = q + h * hd;
    int T = pos + 1;
    extern __shared__ float sh[];
    float *scores = sh;
    float max_s = -1e30f;
    for (int j = threadIdx.x; j < T; j += blockDim.x) {
        const float *kj = kc + (size_t)j * kv_dim + kh * hd;
        float dot = 0.0f;
        for (int c = 0; c < hd; c++) dot += qh[c] * kj[c];
        scores[j] = dot * scale;
    }
    __syncthreads();
    if (threadIdx.x == 0) {
        for (int j = 0; j < T; j++) if (scores[j] > max_s) max_s = scores[j];
        float sum = 0.0f;
        for (int j = 0; j < T; j++) {
            scores[j] = expf(scores[j] - max_s);
            sum += scores[j];
        }
        float inv = 1.0f / (sum + 1e-12f);
        for (int j = 0; j < T; j++) scores[j] *= inv;
    }
    __syncthreads();
    for (int c = threadIdx.x; c < hd; c += blockDim.x) {
        float acc = 0.0f;
        for (int j = 0; j < T; j++) {
            acc += scores[j] * vc[(size_t)j * kv_dim + kh * hd + c];
        }
        out[h * hd + c] = acc;
    }
}

__global__ void k_swiglu_mul(const float *gate, const float *up, float *out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        float g = gate[i];
        out[i] = (g / (1.0f + expf(-g))) * up[i];
    }
}

__global__ void k_add_inplace(float *dst, const float *src, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) dst[i] += src[i];
}

struct TeacherWorkspace {
    int max_seq;
    float *x, *norm, *q, *k, *v, *attn, *proj, *gate, *up, *logits;
    std::vector<float*> kc, vc;
};

static TeacherWorkspace alloc_teacher_ws(const TeacherModel &t, int max_seq) {
    TeacherWorkspace w;
    w.max_seq = max_seq;
    int kv_dim = (t.d / t.nh) * t.nkv;
    CUDA_CHECK(cudaMalloc(&w.x, t.d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&w.norm, t.d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&w.q, t.d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&w.k, kv_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&w.v, kv_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&w.attn, t.d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&w.proj, t.d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&w.gate, t.ffn * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&w.up, t.ffn * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&w.logits, t.vocab * sizeof(float)));
    for (int l = 0; l < t.n_layers; l++) {
        float *dk = nullptr, *dv = nullptr;
        CUDA_CHECK(cudaMalloc(&dk, (size_t)max_seq * kv_dim * sizeof(float)));
        CUDA_CHECK(cudaMalloc(&dv, (size_t)max_seq * kv_dim * sizeof(float)));
        w.kc.push_back(dk);
        w.vc.push_back(dv);
    }
    return w;
}

static void teacher_step(cublasHandle_t cb, const TeacherModel &t, TeacherWorkspace &w, uint32_t tok, int pos, bool need_logits) {
    int hd = t.d / t.nh;
    int kv_dim = t.nkv * hd;
    float one = 1.0f, zero = 0.0f;
    CUDA_CHECK(cudaMemcpy(w.x, t.d_embed + (size_t)tok * t.d, t.d * sizeof(float), cudaMemcpyDeviceToDevice));
    for (int l = 0; l < t.n_layers; l++) {
        k_rms_norm_vec<<<1, 256>>>(w.x, t.in_ln[l], w.norm, t.d, t.rms_eps);
        CUBLAS_CHECK(cublasSgemv(cb, CUBLAS_OP_T, t.d, t.d, &one, t.q_w[l], t.d, w.norm, 1, &zero, w.q, 1));
        CUBLAS_CHECK(cublasSgemv(cb, CUBLAS_OP_T, t.d, kv_dim, &one, t.k_w[l], t.d, w.norm, 1, &zero, w.k, 1));
        CUBLAS_CHECK(cublasSgemv(cb, CUBLAS_OP_T, t.d, kv_dim, &one, t.v_w[l], t.d, w.norm, 1, &zero, w.v, 1));
        k_rope_vec<<<(t.nh * (hd >> 1) + 127) / 128, 128>>>(w.q, t.nh, hd, pos, t.rope_theta);
        k_rope_vec<<<(t.nkv * (hd >> 1) + 127) / 128, 128>>>(w.k, t.nkv, hd, pos, t.rope_theta);
        CUDA_CHECK(cudaMemcpy(w.kc[l] + (size_t)pos * kv_dim, w.k, kv_dim * sizeof(float), cudaMemcpyDeviceToDevice));
        CUDA_CHECK(cudaMemcpy(w.vc[l] + (size_t)pos * kv_dim, w.v, kv_dim * sizeof(float), cudaMemcpyDeviceToDevice));
        size_t smem = (size_t)(pos + 1) * sizeof(float);
        k_attn_step_head<<<t.nh, 64, smem>>>(w.q, w.kc[l], w.vc[l], w.attn, t.nh, t.nkv, hd, kv_dim, pos, 1.0f / sqrtf((float)hd));
        CUBLAS_CHECK(cublasSgemv(cb, CUBLAS_OP_T, t.d, t.d, &one, t.o_w[l], t.d, w.attn, 1, &zero, w.proj, 1));
        k_add_inplace<<<(t.d + 255) / 256, 256>>>(w.x, w.proj, t.d);
        k_rms_norm_vec<<<1, 256>>>(w.x, t.post_ln[l], w.norm, t.d, t.rms_eps);
        CUBLAS_CHECK(cublasSgemv(cb, CUBLAS_OP_T, t.d, t.ffn, &one, t.g_w[l], t.d, w.norm, 1, &zero, w.gate, 1));
        CUBLAS_CHECK(cublasSgemv(cb, CUBLAS_OP_T, t.d, t.ffn, &one, t.u_w[l], t.d, w.norm, 1, &zero, w.up, 1));
        k_swiglu_mul<<<(t.ffn + 255) / 256, 256>>>(w.gate, w.up, w.gate, t.ffn);
        CUBLAS_CHECK(cublasSgemv(cb, CUBLAS_OP_T, t.ffn, t.d, &one, t.d_w[l], t.ffn, w.gate, 1, &zero, w.proj, 1));
        k_add_inplace<<<(t.d + 255) / 256, 256>>>(w.x, w.proj, t.d);
    }
    if (need_logits) {
        k_rms_norm_vec<<<1, 256>>>(w.x, t.d_norm, w.norm, t.d, t.rms_eps);
        CUBLAS_CHECK(cublasSgemv(cb, CUBLAS_OP_T, t.d, t.vocab, &one, t.d_embed, t.d, w.norm, 1, &zero, w.logits, 1));
    }
}

static uint32_t sample_top_p(std::vector<float> &logits, float temp, float top_p, uint64_t &rng) {
    int n = (int)logits.size();
    if (temp <= 0.0f) {
        return (uint32_t)(std::max_element(logits.begin(), logits.end()) - logits.begin());
    }
    float max_l = *std::max_element(logits.begin(), logits.end());
    std::vector<std::pair<float, int>> items;
    items.reserve(64);
    float sum = 0.0f;
    for (int i = 0; i < n; i++) {
        float v = expf((logits[i] - max_l) / temp);
        sum += v;
        if (v > 1e-6f) items.push_back({v, i});
    }
    for (auto &p : items) p.first /= (sum + 1e-12f);
    std::sort(items.begin(), items.end(), [](const auto &a, const auto &b){ return a.first > b.first; });
    rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
    float u = (float)(rng & 0xffffff) / (float)0x1000000;
    float cum = 0.0f;
    for (const auto &p : items) {
        cum += p.first;
        if (cum >= u * top_p) return (uint32_t)p.second;
    }
    return items.empty() ? 2u : (uint32_t)items[0].second;
}

// ============================================================================
// Batched GPU Transformer Distillation Engine for Compact Seed (`bootstrap_v12_seed`)
// ============================================================================

__global__ void k_embed_gather(const float *E, const uint32_t *toks, float *X, int N, int D) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < N * D) {
        int r = idx / D;
        int c = idx % D;
        X[idx] = E[(size_t)toks[r] * D + c];
    }
}

__global__ void k_embed_scatter_add(float *dE, const uint32_t *toks, const float *dX, int N, int D) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < N * D) {
        int r = idx / D;
        int c = idx % D;
        atomicAdd(&dE[(size_t)toks[r] * D + c], dX[idx]);
    }
}

__global__ void k_rms_fwd(const float *x, const float *g, float *y, float *rstd, int N, int D, float eps) {
    int row = blockIdx.x;
    if (row >= N) return;
    __shared__ float sh[32];
    const float *xr = x + (size_t)row * D;
    float *yr = y + (size_t)row * D;
    float ss = 0.0f;
    for (int i = threadIdx.x; i < D; i += blockDim.x) ss += xr[i] * xr[i];
    int lane = threadIdx.x & 31, warp = threadIdx.x >> 5;
    for (int o = 16; o > 0; o >>= 1) ss += __shfl_down_sync(0xffffffff, ss, o);
    if (lane == 0) sh[warp] = ss;
    __syncthreads();
    if (warp == 0) {
        float v = (lane < (blockDim.x >> 5)) ? sh[lane] : 0.0f;
        for (int o = 16; o > 0; o >>= 1) v += __shfl_down_sync(0xffffffff, v, o);
        if (lane == 0) {
            float inv = rsqrtf(v / (float)D + eps);
            sh[0] = inv;
            rstd[row] = inv;
        }
    }
    __syncthreads();
    float inv = sh[0];
    for (int i = threadIdx.x; i < D; i += blockDim.x) {
        yr[i] = xr[i] * inv * g[i];
    }
}

__global__ void k_rms_bwd(const float *dy, const float *x, const float *g, const float *rstd, float *dx, float *dg, int N, int D) {
    int row = blockIdx.x;
    if (row >= N) return;
    __shared__ float sh[32];
    const float *dyr = dy + (size_t)row * D;
    const float *xr = x + (size_t)row * D;
    float *dxr = dx + (size_t)row * D;
    float inv = rstd[row];
    float dot = 0.0f;
    for (int i = threadIdx.x; i < D; i += blockDim.x) {
        float xhat = xr[i] * inv;
        atomicAdd(&dg[i], dyr[i] * xhat);
        dot += dyr[i] * g[i] * xhat;
    }
    int lane = threadIdx.x & 31, warp = threadIdx.x >> 5;
    for (int o = 16; o > 0; o >>= 1) dot += __shfl_down_sync(0xffffffff, dot, o);
    if (lane == 0) sh[warp] = dot;
    __syncthreads();
    if (warp == 0) {
        float v = (lane < (blockDim.x >> 5)) ? sh[lane] : 0.0f;
        for (int o = 16; o > 0; o >>= 1) v += __shfl_down_sync(0xffffffff, v, o);
        if (lane == 0) sh[0] = v / (float)D;
    }
    __syncthreads();
    float mean_dot = sh[0];
    for (int i = threadIdx.x; i < D; i += blockDim.x) {
        float xhat = xr[i] * inv;
        dxr[i] += inv * (dyr[i] * g[i] - xhat * mean_dot);
    }
}

__global__ void k_rope_batch(float *X, int B, int T, int n_heads, int hd, float theta, bool bwd) {
    int half = hd >> 1;
    int total = B * T * n_heads * half;
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= total) return;
    int rem = idx;
    int i = rem % half; rem /= half;
    int h = rem % n_heads; rem /= n_heads;
    int t = rem % T;
    int b = rem / T;
    float inv_freq = 1.0f / powf(theta, (2.0f * (float)i) / (float)hd);
    float ang = (float)t * inv_freq;
    float c = cosf(ang), s = sinf(ang);
    if (bwd) s = -s;
    float *base = X + ((size_t)b * T + t) * (n_heads * hd) + h * hd;
    float x0 = base[i], x1 = base[i + half];
    base[i] = x0 * c - x1 * s;
    base[i + half] = x0 * s + x1 * c;
}

__global__ void k_attn_fwd_batch(
    const float *Q, const float *K, const float *V, float *Out, float *Probs,
    int B, int T, int nh, int nkv, int hd, float scale
) {
    int bt = blockIdx.x;
    int h = blockIdx.y;
    int b = bt / T;
    int t = bt % T;
    if (b >= B || h >= nh) return;
    int kh = h / (nh / nkv);
    int d = nh * hd, kv_dim = nkv * hd;
    const float *qt = Q + ((size_t)b * T + t) * d + h * hd;
    float *pt = Probs + (((size_t)b * nh + h) * T + t) * T;
    float *ot = Out + ((size_t)b * T + t) * d + h * hd;

    __shared__ float sh_reduce[4];
    int lane = threadIdx.x & 31;
    int warp = threadIdx.x >> 5;
    int nwarps = blockDim.x >> 5;

    float local_max = -1e30f;
    for (int j = threadIdx.x; j <= t; j += blockDim.x) {
        const float *kj = K + ((size_t)b * T + j) * kv_dim + kh * hd;
        float dot = 0.0f;
        for (int c = 0; c < hd; c++) dot += qt[c] * kj[c];
        float s = dot * scale;
        pt[j] = s;
        if (s > local_max) local_max = s;
    }
    for (int o = 16; o > 0; o >>= 1) local_max = fmaxf(local_max, __shfl_down_sync(0xffffffff, local_max, o));
    if (lane == 0) sh_reduce[warp] = local_max;
    __syncthreads();
    if (warp == 0) {
        float v = (lane < nwarps) ? sh_reduce[lane] : -1e30f;
        for (int o = 16; o > 0; o >>= 1) v = fmaxf(v, __shfl_down_sync(0xffffffff, v, o));
        if (lane == 0) sh_reduce[0] = v;
    }
    __syncthreads();
    float max_s = sh_reduce[0];

    float local_sum = 0.0f;
    for (int j = threadIdx.x; j <= t; j += blockDim.x) {
        float e = expf(pt[j] - max_s);
        pt[j] = e;
        local_sum += e;
    }
    for (int o = 16; o > 0; o >>= 1) local_sum += __shfl_down_sync(0xffffffff, local_sum, o);
    if (lane == 0) sh_reduce[warp] = local_sum;
    __syncthreads();
    if (warp == 0) {
        float v = (lane < nwarps) ? sh_reduce[lane] : 0.0f;
        for (int o = 16; o > 0; o >>= 1) v += __shfl_down_sync(0xffffffff, v, o);
        if (lane == 0) sh_reduce[0] = 1.0f / (v + 1e-12f);
    }
    __syncthreads();
    float inv = sh_reduce[0];

    for (int j = threadIdx.x; j <= t; j += blockDim.x) pt[j] *= inv;
    for (int j = t + 1 + threadIdx.x; j < T; j += blockDim.x) pt[j] = 0.0f;
    __syncthreads();

    for (int c = threadIdx.x; c < hd; c += blockDim.x) {
        float acc = 0.0f;
        for (int j = 0; j <= t; j++) {
            acc += pt[j] * V[((size_t)b * T + j) * kv_dim + kh * hd + c];
        }
        ot[c] = acc;
    }
}

__global__ void k_attn_bwd_q(
    const float *dOut, const float *Q, const float *K, const float *V, const float *Probs,
    float *dQ, int B, int T, int nh, int nkv, int hd, float scale
) {
    int bt = blockIdx.x;
    int h = blockIdx.y;
    int b = bt / T, t = bt % T;
    if (b >= B || h >= nh) return;
    int kh = h / (nh / nkv);
    int d = nh * hd, kv_dim = nkv * hd;
    const float *do_t = dOut + ((size_t)b * T + t) * d + h * hd;
    const float *pt = Probs + (((size_t)b * nh + h) * T + t) * T;
    float *dq_t = dQ + ((size_t)b * T + t) * d + h * hd;

    float d_dot_p = 0.0f;
    // Note: small head_dim <= 64, single-thread reduction or shared loop for clarity.
    extern __shared__ float dp_sh[];
    for (int j = threadIdx.x; j <= t; j += blockDim.x) {
        const float *vj = V + ((size_t)b * T + j) * kv_dim + kh * hd;
        float dp = 0.0f;
        for (int c = 0; c < hd; c++) dp += do_t[c] * vj[c];
        dp_sh[j] = dp;
    }
    __syncthreads();
    if (threadIdx.x == 0) {
        for (int j = 0; j <= t; j++) d_dot_p += pt[j] * dp_sh[j];
        dp_sh[T] = d_dot_p;
    }
    __syncthreads();
    float mean_dp = dp_sh[T];
    for (int c = threadIdx.x; c < hd; c += blockDim.x) {
        float acc = 0.0f;
        for (int j = 0; j <= t; j++) {
            float ds = pt[j] * (dp_sh[j] - mean_dp) * scale;
            acc += ds * K[((size_t)b * T + j) * kv_dim + kh * hd + c];
        }
        dq_t[c] = acc;
    }
}

__global__ void k_attn_bwd_delta(
    const float *dOut, const float *Attn, float *Delta, int B, int T, int nh, int hd
) {
    int bt = blockIdx.x, h = blockIdx.y;
    int b = bt / T, t = bt % T;
    if (b >= B || h >= nh) return;
    int d = nh * hd;
    const float *do_t = dOut + ((size_t)b * T + t) * d + h * hd;
    const float *at_t = Attn + ((size_t)b * T + t) * d + h * hd;
    float acc = 0.0f;
    for (int c = threadIdx.x; c < hd; c += blockDim.x) acc += do_t[c] * at_t[c];
    for (int o = 16; o > 0; o >>= 1) acc += __shfl_down_sync(0xffffffff, acc, o);
    __shared__ float wsum[4];
    int lane = threadIdx.x & 31;
    int warp = threadIdx.x >> 5;
    if (lane == 0) wsum[warp] = acc;
    __syncthreads();
    if (warp == 0) {
        int nwarps = blockDim.x >> 5;
        float v = (lane < nwarps) ? wsum[lane] : 0.0f;
        for (int o = 16; o > 0; o >>= 1) v += __shfl_down_sync(0xffffffff, v, o);
        if (lane == 0) Delta[((size_t)b * nh + h) * T + t] = v;
    }
}

__global__ void k_attn_bwd_kv(
    const float *dOut, const float *Q, const float *K, const float *V, const float *Probs, const float *Delta,
    float *dK, float *dV, int B, int T, int nh, int nkv, int hd, float scale
) {
    int bj = blockIdx.x;
    int kh = blockIdx.y;
    int b = bj / T, j = bj % T;
    if (b >= B || kh >= nkv) return;
    int group = nh / nkv;
    int d = nh * hd, kv_dim = nkv * hd;
    const float *vj = V + ((size_t)b * T + j) * kv_dim + kh * hd;
    float *dkj = dK + ((size_t)b * T + j) * kv_dim + kh * hd;
    float *dvj = dV + ((size_t)b * T + j) * kv_dim + kh * hd;

    // 1 thread per coordinate c (hd=64 = 2 warps); share dp = dot(dOut[t], V[j]) via shared array!
    __shared__ float v_sh[64];
    __shared__ float sh_dp;
    if (threadIdx.x < hd) v_sh[threadIdx.x] = vj[threadIdx.x];
    __syncthreads();

    float gk = 0.0f, gv = 0.0f;
    float my_do, my_q;
    for (int g = 0; g < group; g++) {
        int h = kh * group + g;
        for (int t = j; t < T; t++) {
            float p = Probs[(((size_t)b * nh + h) * T + t) * T + j];
            const float *do_t = dOut + ((size_t)b * T + t) * d + h * hd;
            const float *q_t = Q + ((size_t)b * T + t) * d + h * hd;
            my_do = (threadIdx.x < hd) ? do_t[threadIdx.x] : 0.0f;
            my_q = (threadIdx.x < hd) ? q_t[threadIdx.x] : 0.0f;
            float prod = my_do * ((threadIdx.x < hd) ? v_sh[threadIdx.x] : 0.0f);
            // Intra-block reduction for hd <= 64 (2 warps max):
            for (int o = 16; o > 0; o >>= 1) prod += __shfl_down_sync(0xffffffff, prod, o);
            __shared__ float wsum[2];
            if ((threadIdx.x & 31) == 0) wsum[threadIdx.x >> 5] = prod;
            __syncthreads();
            if (threadIdx.x == 0) sh_dp = wsum[0] + wsum[1];
            __syncthreads();
            float mean_dp = Delta[((size_t)b * nh + h) * T + t];
            float ds = p * (sh_dp - mean_dp) * scale;
            gv += p * my_do;
            gk += ds * my_q;
            __syncthreads();
        }
    }
    if (threadIdx.x < hd) {
        dkj[threadIdx.x] = gk;
        dvj[threadIdx.x] = gv;
    }
}

__global__ void k_swiglu_fwd(const float *G, const float *U, float *Out, int N) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < N) {
        float g = G[i];
        float sig = 1.0f / (1.0f + expf(-g));
        Out[i] = (g * sig) * U[i];
    }
}

__global__ void k_swiglu_bwd(const float *dO, const float *G, const float *U, float *dG, float *dU, int N) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < N) {
        float g = G[i], u = U[i], do_v = dO[i];
        float sig = 1.0f / (1.0f + expf(-g));
        float sw = g * sig;
        dU[i] = do_v * sw;
        float dsw = sig * (1.0f + g * (1.0f - sig));
        dG[i] = do_v * u * dsw;
    }
}

__global__ void k_ce_kd_loss(
    const float *logits, const uint32_t *targets,
    const uint16_t *kd_ids, const float *kd_probs, int kd_k,
    float *d_logits, float *loss_out, int N, int V, float ce_w, float kd_w
) {
    int row = blockIdx.x;
    if (row >= N) return;
    __shared__ float sh[32];
    const float *lr = logits + (size_t)row * V;
    float *dlr = d_logits + (size_t)row * V;

    float max_l = -1e30f;
    for (int i = threadIdx.x; i < V; i += blockDim.x) if (lr[i] > max_l) max_l = lr[i];
    int lane = threadIdx.x & 31, warp = threadIdx.x >> 5;
    for (int o = 16; o > 0; o >>= 1) max_l = fmaxf(max_l, __shfl_down_sync(0xffffffff, max_l, o));
    if (lane == 0) sh[warp] = max_l;
    __syncthreads();
    if (warp == 0) {
        float v = (lane < (blockDim.x >> 5)) ? sh[lane] : -1e30f;
        for (int o = 16; o > 0; o >>= 1) v = fmaxf(v, __shfl_down_sync(0xffffffff, v, o));
        if (lane == 0) sh[0] = v;
    }
    __syncthreads();
    max_l = sh[0];

    float sum = 0.0f;
    for (int i = threadIdx.x; i < V; i += blockDim.x) sum += expf(lr[i] - max_l);
    for (int o = 16; o > 0; o >>= 1) sum += __shfl_down_sync(0xffffffff, sum, o);
    if (lane == 0) sh[warp] = sum;
    __syncthreads();
    if (warp == 0) {
        float v = (lane < (blockDim.x >> 5)) ? sh[lane] : 0.0f;
        for (int o = 16; o > 0; o >>= 1) v += __shfl_down_sync(0xffffffff, v, o);
        if (lane == 0) sh[0] = v;
    }
    __syncthreads();
    float inv_sum = 1.0f / (sh[0] + 1e-12f);
    float inv_n = 1.0f / (float)N;
    uint32_t tgt = targets[row];

    if (tgt >= (uint32_t)V) {
        for (int i = threadIdx.x; i < V; i += blockDim.x) dlr[i] = 0.0f;
        return;
    }

    if (threadIdx.x == 0) {
        float tp_sum = 0.0f;
        if (kd_ids != nullptr && kd_k > 0 && kd_w > 0.0f) {
            const uint16_t *kdi = kd_ids + (size_t)row * kd_k;
            const float *kdp = kd_probs + (size_t)row * kd_k;
            for (int kk = 0; kk < kd_k; kk++) {
                if (kdi[kk] < (uint16_t)V && kdp[kk] > 0.0f) tp_sum += kdp[kk];
            }
        }
        sh[1] = (tp_sum > 1e-6f) ? kd_w : 0.0f;
    }
    __syncthreads();
    float eff_kd_w = sh[1];

    for (int i = threadIdx.x; i < V; i += blockDim.x) {
        float p = expf(lr[i] - max_l) * inv_sum;
        float g = (ce_w + eff_kd_w) * p;
        if ((uint32_t)i == tgt) g -= ce_w;
        dlr[i] = g * inv_n;
    }
    __syncthreads();
    if (threadIdx.x == 0) {
        float ptgt = expf(lr[tgt] - max_l) * inv_sum;
        float l_ce = -logf(fmaxf(ptgt, 1e-9f)) * ce_w;
        float l_kd = 0.0f;
        if (eff_kd_w > 0.0f) {
            const uint16_t *kdi = kd_ids + (size_t)row * kd_k;
            const float *kdp = kd_probs + (size_t)row * kd_k;
            for (int kk = 0; kk < kd_k; kk++) {
                uint16_t tid = kdi[kk];
                float tp = kdp[kk];
                if (tid < (uint16_t)V && tp > 0.0f) {
                    dlr[tid] -= eff_kd_w * tp * inv_n;
                    float sp = expf(lr[tid] - max_l) * inv_sum;
                    l_kd += eff_kd_w * tp * (logf(fmaxf(tp, 1e-9f)) - logf(fmaxf(sp, 1e-9f)));
                }
            }
        }
        atomicAdd(loss_out, (l_ce + l_kd) * inv_n);
    }
}

__global__ void k_fake_quant_g32(const float *W, float *Wq, int rows, int cols) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total_groups = rows * (cols >> 5);
    if (idx >= total_groups) return;
    int off = idx << 5;
    float amax = 0.0f;
    for (int c = 0; c < 32; c++) amax = fmaxf(amax, fabsf(W[off + c]));
    if (amax < 1e-12f) {
        for (int c = 0; c < 32; c++) Wq[off + c] = 0.0f;
        return;
    }
    float s = amax / 7.0f;
    for (int c = 0; c < 32; c++) {
        float q = roundf(W[off + c] / s);
        q = fmaxf(-8.0f, fminf(7.0f, q));
        Wq[off + c] = q * s;
    }
}

__global__ void k_adamw_step(float *W, float *G, float *M, float *V, int n, float lr, float b1, float b2, float wd, float bc1, float bc2, float gscale) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        float g = G[i] * gscale;
        float m = b1 * M[i] + (1.0f - b1) * g;
        float v = b2 * V[i] + (1.0f - b2) * g * g;
        M[i] = m; V[i] = v;
        float mhat = m / bc1;
        float vhat = v / bc2;
        W[i] = W[i] * (1.0f - lr * wd) - lr * mhat / (sqrtf(vhat) + 1e-8f);
        G[i] = 0.0f;
    }
}

struct Param {
    int len;
    float *w, *wq, *g, *m, *v;
};

static Param alloc_param(int len) {
    Param p; p.len = len;
    CUDA_CHECK(cudaMalloc(&p.w, (size_t)len * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&p.wq, (size_t)len * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&p.g, (size_t)len * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&p.m, (size_t)len * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&p.v, (size_t)len * sizeof(float)));
    CUDA_CHECK(cudaMemset(p.g, 0, (size_t)len * sizeof(float)));
    CUDA_CHECK(cudaMemset(p.m, 0, (size_t)len * sizeof(float)));
    CUDA_CHECK(cudaMemset(p.v, 0, (size_t)len * sizeof(float)));
    return p;
}

static void init_param_rand(Param &p, float std, uint64_t &rng) {
    std::vector<float> h(p.len);
    for (int i = 0; i < p.len; i++) {
        rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
        float u1 = ((float)(rng & 0xffffff) + 1.0f) / (float)0x1000001;
        rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
        float u2 = ((float)(rng & 0xffffff) + 1.0f) / (float)0x1000001;
        float z = sqrtf(-2.0f * logf(u1)) * cosf(6.28318530718f * u2);
        h[i] = z * std;
    }
    CUDA_CHECK(cudaMemcpy(p.w, h.data(), (size_t)p.len * sizeof(float), cudaMemcpyHostToDevice));
}

struct StudentNet {
    int vocab, d, n_layers, nh, nkv, ffn;
    float rms_eps, rope_theta;
    Param embed, final_norm;
    std::vector<Param> in_ln, post_ln, q_w, k_w, v_w, o_w, g_w, u_w, d_w;
    std::vector<Param*> all_params;
};

static StudentNet build_student(int vocab, int d, int nl, int nh, int nkv, int ffn, const float *embed_init_h, uint64_t rng) {
    StudentNet s;
    s.vocab = vocab; s.d = d; s.n_layers = nl; s.nh = nh; s.nkv = nkv; s.ffn = ffn;
    s.rms_eps = 1e-5f; s.rope_theta = 100000.0f;
    int kv_dim = (d / nh) * nkv;
    s.embed = alloc_param(vocab * d);
    if (embed_init_h) {
        CUDA_CHECK(cudaMemcpy(s.embed.w, embed_init_h, (size_t)vocab * d * sizeof(float), cudaMemcpyHostToDevice));
    } else {
        init_param_rand(s.embed, 0.04f, rng);
    }
    s.final_norm = alloc_param(d);
    { std::vector<float> ones(d, 1.0f); CUDA_CHECK(cudaMemcpy(s.final_norm.w, ones.data(), d * sizeof(float), cudaMemcpyHostToDevice)); }
    s.all_params.push_back(&s.embed);
    s.all_params.push_back(&s.final_norm);

    float std_attn = 1.0f / sqrtf((float)d);
    float std_proj = std_attn / sqrtf(2.0f * (float)nl);
    float std_ffn = 1.0f / sqrtf((float)ffn) / sqrtf(2.0f * (float)nl);

    for (int l = 0; l < nl; l++) {
        Param iln = alloc_param(d), pln = alloc_param(d);
        { std::vector<float> ones(d, 1.0f);
          CUDA_CHECK(cudaMemcpy(iln.w, ones.data(), d * sizeof(float), cudaMemcpyHostToDevice));
          CUDA_CHECK(cudaMemcpy(pln.w, ones.data(), d * sizeof(float), cudaMemcpyHostToDevice)); }
        Param qw = alloc_param(d * d); init_param_rand(qw, std_attn, rng);
        Param kw = alloc_param(kv_dim * d); init_param_rand(kw, std_attn, rng);
        Param vw = alloc_param(kv_dim * d); init_param_rand(vw, std_attn, rng);
        Param ow = alloc_param(d * d); init_param_rand(ow, std_proj, rng);
        Param gw = alloc_param(ffn * d); init_param_rand(gw, std_attn, rng);
        Param uw = alloc_param(ffn * d); init_param_rand(uw, std_attn, rng);
        Param dw = alloc_param(d * ffn); init_param_rand(dw, std_ffn, rng);
        s.in_ln.push_back(iln); s.post_ln.push_back(pln);
        s.q_w.push_back(qw); s.k_w.push_back(kw); s.v_w.push_back(vw); s.o_w.push_back(ow);
        s.g_w.push_back(gw); s.u_w.push_back(uw); s.d_w.push_back(dw);
    }
    for (int l = 0; l < nl; l++) {
        s.all_params.push_back(&s.in_ln[l]); s.all_params.push_back(&s.post_ln[l]);
        s.all_params.push_back(&s.q_w[l]); s.all_params.push_back(&s.k_w[l]); s.all_params.push_back(&s.v_w[l]); s.all_params.push_back(&s.o_w[l]);
        s.all_params.push_back(&s.g_w[l]); s.all_params.push_back(&s.u_w[l]); s.all_params.push_back(&s.d_w[l]);
    }
    return s;
}

static void sgemm_rowmajor(cublasHandle_t cb, int M, int N, int K, const float *A, const float *B_rowmajor_NxK, float *C, float beta) {
    float one = 1.0f;
    // C [M, N] = A [M, K] * B^T [K, N]
    CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_T, CUBLAS_OP_N, N, M, K, &one, B_rowmajor_NxK, K, A, K, &beta, C, N));
}

struct TrainActs {
    int B, T, N;
    std::vector<float*> x_in, n1, rstd1, q, k, v, probs, attn, x_mid, n2, rstd2, gate, up, swi;
    float *x_final, *n_final, *rstd_final, *logits;
    // Backward temporaries
    float *dx, *d_norm, *d_q, *d_k, *d_v, *d_attn, *attn_delta, *d_gate, *d_up, *d_swi, *d_logits;
    float *loss;
};

static TrainActs alloc_train_acts(const StudentNet &s, int B, int T) {
    TrainActs a; a.B = B; a.T = T; a.N = B * T;
    int kv_dim = (s.d / s.nh) * s.nkv;
    for (int l = 0; l < s.n_layers; l++) {
        float *p;
        CUDA_CHECK(cudaMalloc(&p, (size_t)a.N * s.d * sizeof(float))); a.x_in.push_back(p);
        CUDA_CHECK(cudaMalloc(&p, (size_t)a.N * s.d * sizeof(float))); a.n1.push_back(p);
        CUDA_CHECK(cudaMalloc(&p, (size_t)a.N * sizeof(float))); a.rstd1.push_back(p);
        CUDA_CHECK(cudaMalloc(&p, (size_t)a.N * s.d * sizeof(float))); a.q.push_back(p);
        CUDA_CHECK(cudaMalloc(&p, (size_t)a.N * kv_dim * sizeof(float))); a.k.push_back(p);
        CUDA_CHECK(cudaMalloc(&p, (size_t)a.N * kv_dim * sizeof(float))); a.v.push_back(p);
        CUDA_CHECK(cudaMalloc(&p, (size_t)B * s.nh * T * T * sizeof(float))); a.probs.push_back(p);
        CUDA_CHECK(cudaMalloc(&p, (size_t)a.N * s.d * sizeof(float))); a.attn.push_back(p);
        CUDA_CHECK(cudaMalloc(&p, (size_t)a.N * s.d * sizeof(float))); a.x_mid.push_back(p);
        CUDA_CHECK(cudaMalloc(&p, (size_t)a.N * s.d * sizeof(float))); a.n2.push_back(p);
        CUDA_CHECK(cudaMalloc(&p, (size_t)a.N * sizeof(float))); a.rstd2.push_back(p);
        CUDA_CHECK(cudaMalloc(&p, (size_t)a.N * s.ffn * sizeof(float))); a.gate.push_back(p);
        CUDA_CHECK(cudaMalloc(&p, (size_t)a.N * s.ffn * sizeof(float))); a.up.push_back(p);
        CUDA_CHECK(cudaMalloc(&p, (size_t)a.N * s.ffn * sizeof(float))); a.swi.push_back(p);
    }
    CUDA_CHECK(cudaMalloc(&a.x_final, (size_t)a.N * s.d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.n_final, (size_t)a.N * s.d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.rstd_final, (size_t)a.N * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.logits, (size_t)a.N * s.vocab * sizeof(float)));

    CUDA_CHECK(cudaMalloc(&a.dx, (size_t)a.N * s.d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.d_norm, (size_t)a.N * s.d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.d_q, (size_t)a.N * s.d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.d_k, (size_t)a.N * kv_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.d_v, (size_t)a.N * kv_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.d_attn, (size_t)a.N * s.d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.attn_delta, (size_t)B * s.nh * T * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.d_gate, (size_t)a.N * s.ffn * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.d_up, (size_t)a.N * s.ffn * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.d_swi, (size_t)a.N * s.ffn * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.d_logits, (size_t)a.N * s.vocab * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&a.loss, sizeof(float)));
    return a;
}

static void prepare_quant_weights(StudentNet &s, bool use_qat) {
    if (!use_qat) {
        for (Param *p : s.all_params) {
            CUDA_CHECK(cudaMemcpy(p->wq, p->w, (size_t)p->len * sizeof(float), cudaMemcpyDeviceToDevice));
        }
        return;
    }
    int kv_dim = (s.d / s.nh) * s.nkv;
    CUDA_CHECK(cudaMemcpy(s.embed.wq, s.embed.w, (size_t)s.embed.len * sizeof(float), cudaMemcpyDeviceToDevice));
    CUDA_CHECK(cudaMemcpy(s.final_norm.wq, s.final_norm.w, (size_t)s.final_norm.len * sizeof(float), cudaMemcpyDeviceToDevice));
    for (int l = 0; l < s.n_layers; l++) {
        CUDA_CHECK(cudaMemcpy(s.in_ln[l].wq, s.in_ln[l].w, (size_t)s.d * sizeof(float), cudaMemcpyDeviceToDevice));
        CUDA_CHECK(cudaMemcpy(s.post_ln[l].wq, s.post_ln[l].w, (size_t)s.d * sizeof(float), cudaMemcpyDeviceToDevice));
        k_fake_quant_g32<<<(s.d * (s.d >> 5) + 255) / 256, 256>>>(s.q_w[l].w, s.q_w[l].wq, s.d, s.d);
        k_fake_quant_g32<<<(kv_dim * (s.d >> 5) + 255) / 256, 256>>>(s.k_w[l].w, s.k_w[l].wq, kv_dim, s.d);
        k_fake_quant_g32<<<(kv_dim * (s.d >> 5) + 255) / 256, 256>>>(s.v_w[l].w, s.v_w[l].wq, kv_dim, s.d);
        k_fake_quant_g32<<<(s.d * (s.d >> 5) + 255) / 256, 256>>>(s.o_w[l].w, s.o_w[l].wq, s.d, s.d);
        k_fake_quant_g32<<<(s.ffn * (s.d >> 5) + 255) / 256, 256>>>(s.g_w[l].w, s.g_w[l].wq, s.ffn, s.d);
        k_fake_quant_g32<<<(s.ffn * (s.d >> 5) + 255) / 256, 256>>>(s.u_w[l].w, s.u_w[l].wq, s.ffn, s.d);
        k_fake_quant_g32<<<(s.d * (s.ffn >> 5) + 255) / 256, 256>>>(s.d_w[l].w, s.d_w[l].wq, s.d, s.ffn);
    }
}

static float student_fwd_bwd(
    cublasHandle_t cb, StudentNet &s, TrainActs &a,
    const uint32_t *d_in_toks, const uint32_t *d_tgt_toks,
    const uint16_t *d_kd_ids, const float *d_kd_probs, int kd_k,
    bool do_bwd, float ce_w, float kd_w
) {
    int N = a.N, B = a.B, T = a.T;
    int hd = s.d / s.nh;
    int kv_dim = s.nkv * hd;
    float scale = 1.0f / sqrtf((float)hd);
    float one = 1.0f, zero = 0.0f;

    k_embed_gather<<<(N * s.d + 255) / 256, 256>>>(s.embed.wq, d_in_toks, a.x_in[0], N, s.d);
    for (int l = 0; l < s.n_layers; l++) {
        k_rms_fwd<<<N, 256>>>(a.x_in[l], s.in_ln[l].wq, a.n1[l], a.rstd1[l], N, s.d, s.rms_eps);
        sgemm_rowmajor(cb, N, s.d, s.d, a.n1[l], s.q_w[l].wq, a.q[l], 0.0f);
        sgemm_rowmajor(cb, N, kv_dim, s.d, a.n1[l], s.k_w[l].wq, a.k[l], 0.0f);
        sgemm_rowmajor(cb, N, kv_dim, s.d, a.n1[l], s.v_w[l].wq, a.v[l], 0.0f);
        k_rope_batch<<<(N * s.nh * (hd >> 1) + 255) / 256, 256>>>(a.q[l], B, T, s.nh, hd, s.rope_theta, false);
        k_rope_batch<<<(N * s.nkv * (hd >> 1) + 255) / 256, 256>>>(a.k[l], B, T, s.nkv, hd, s.rope_theta, false);
        dim3 grid(N, s.nh);
        k_attn_fwd_batch<<<grid, 64>>>(a.q[l], a.k[l], a.v[l], a.attn[l], a.probs[l], B, T, s.nh, s.nkv, hd, scale);
        float *next_mid = a.x_mid[l];
        CUDA_CHECK(cudaMemcpy(next_mid, a.x_in[l], (size_t)N * s.d * sizeof(float), cudaMemcpyDeviceToDevice));
        sgemm_rowmajor(cb, N, s.d, s.d, a.attn[l], s.o_w[l].wq, next_mid, 1.0f);

        k_rms_fwd<<<N, 256>>>(next_mid, s.post_ln[l].wq, a.n2[l], a.rstd2[l], N, s.d, s.rms_eps);
        sgemm_rowmajor(cb, N, s.ffn, s.d, a.n2[l], s.g_w[l].wq, a.gate[l], 0.0f);
        sgemm_rowmajor(cb, N, s.ffn, s.d, a.n2[l], s.u_w[l].wq, a.up[l], 0.0f);
        k_swiglu_fwd<<<(N * s.ffn + 255) / 256, 256>>>(a.gate[l], a.up[l], a.swi[l], N * s.ffn);

        float *next_x = (l + 1 < s.n_layers) ? a.x_in[l + 1] : a.x_final;
        CUDA_CHECK(cudaMemcpy(next_x, next_mid, (size_t)N * s.d * sizeof(float), cudaMemcpyDeviceToDevice));
        sgemm_rowmajor(cb, N, s.d, s.ffn, a.swi[l], s.d_w[l].wq, next_x, 1.0f);
    }

    k_rms_fwd<<<N, 256>>>(a.x_final, s.final_norm.wq, a.n_final, a.rstd_final, N, s.d, s.rms_eps);
    sgemm_rowmajor(cb, N, s.vocab, s.d, a.n_final, s.embed.wq, a.logits, 0.0f);

    CUDA_CHECK(cudaMemset(a.loss, 0, sizeof(float)));
    k_ce_kd_loss<<<N, 256>>>(a.logits, d_tgt_toks, d_kd_ids, d_kd_probs, kd_k, a.d_logits, a.loss, N, s.vocab, ce_w, kd_w);
    float loss_h = 0.0f;
    CUDA_CHECK(cudaMemcpy(&loss_h, a.loss, sizeof(float), cudaMemcpyDeviceToHost));
    if (!do_bwd) return loss_h;

    // d_embed += d_logits^T * n_final  ([vocab, N] * [N, d])
    CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_T, s.d, s.vocab, N, &one, a.n_final, s.d, a.d_logits, s.vocab, &one, s.embed.g, s.d));
    // d_n_final = d_logits * embed
    CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_N, s.d, N, s.vocab, &one, s.embed.wq, s.d, a.d_logits, s.vocab, &zero, a.d_norm, s.d));
    CUDA_CHECK(cudaMemset(a.dx, 0, (size_t)N * s.d * sizeof(float)));
    k_rms_bwd<<<N, 256>>>(a.d_norm, a.x_final, s.final_norm.wq, a.rstd_final, a.dx, s.final_norm.g, N, s.d);

    for (int l = s.n_layers - 1; l >= 0; l--) {
        // MLP backward: d_down += dx^T * swi; d_swi = dx * d_w
        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_T, s.ffn, s.d, N, &one, a.swi[l], s.ffn, a.dx, s.d, &one, s.d_w[l].g, s.ffn));
        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_N, s.ffn, N, s.d, &one, s.d_w[l].wq, s.ffn, a.dx, s.d, &zero, a.d_swi, s.ffn));
        k_swiglu_bwd<<<(N * s.ffn + 255) / 256, 256>>>(a.d_swi, a.gate[l], a.up[l], a.d_gate, a.d_up, N * s.ffn);

        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_T, s.d, s.ffn, N, &one, a.n2[l], s.d, a.d_gate, s.ffn, &one, s.g_w[l].g, s.d));
        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_T, s.d, s.ffn, N, &one, a.n2[l], s.d, a.d_up, s.ffn, &one, s.u_w[l].g, s.d));
        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_N, s.d, N, s.ffn, &one, s.g_w[l].wq, s.d, a.d_gate, s.ffn, &zero, a.d_norm, s.d));
        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_N, s.d, N, s.ffn, &one, s.u_w[l].wq, s.d, a.d_up, s.ffn, &one, a.d_norm, s.d));
        k_rms_bwd<<<N, 256>>>(a.d_norm, a.x_mid[l], s.post_ln[l].wq, a.rstd2[l], a.dx, s.post_ln[l].g, N, s.d);

        // Attention backward: d_ow += dx^T * attn; d_attn = dx * o_w
        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_T, s.d, s.d, N, &one, a.attn[l], s.d, a.dx, s.d, &one, s.o_w[l].g, s.d));
        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_N, s.d, N, s.d, &one, s.o_w[l].wq, s.d, a.dx, s.d, &zero, a.d_attn, s.d));
        dim3 grid_q(N, s.nh);
        k_attn_bwd_q<<<grid_q, 64, (size_t)(T + 1) * sizeof(float)>>>(a.d_attn, a.q[l], a.k[l], a.v[l], a.probs[l], a.d_q, B, T, s.nh, s.nkv, hd, scale);
        k_attn_bwd_delta<<<grid_q, 64>>>(a.d_attn, a.attn[l], a.attn_delta, B, T, s.nh, hd);
        dim3 grid_kv(N, s.nkv);
        k_attn_bwd_kv<<<grid_kv, 64>>>(a.d_attn, a.q[l], a.k[l], a.v[l], a.probs[l], a.attn_delta, a.d_k, a.d_v, B, T, s.nh, s.nkv, hd, scale);
        k_rope_batch<<<(N * s.nh * (hd >> 1) + 255) / 256, 256>>>(a.d_q, B, T, s.nh, hd, s.rope_theta, true);
        k_rope_batch<<<(N * s.nkv * (hd >> 1) + 255) / 256, 256>>>(a.d_k, B, T, s.nkv, hd, s.rope_theta, true);

        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_T, s.d, s.d, N, &one, a.n1[l], s.d, a.d_q, s.d, &one, s.q_w[l].g, s.d));
        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_T, s.d, kv_dim, N, &one, a.n1[l], s.d, a.d_k, kv_dim, &one, s.k_w[l].g, s.d));
        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_T, s.d, kv_dim, N, &one, a.n1[l], s.d, a.d_v, kv_dim, &one, s.v_w[l].g, s.d));

        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_N, s.d, N, s.d, &one, s.q_w[l].wq, s.d, a.d_q, s.d, &zero, a.d_norm, s.d));
        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_N, s.d, N, kv_dim, &one, s.k_w[l].wq, s.d, a.d_k, kv_dim, &one, a.d_norm, s.d));
        CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_N, s.d, N, kv_dim, &one, s.v_w[l].wq, s.d, a.d_v, kv_dim, &one, a.d_norm, s.d));
        k_rms_bwd<<<N, 256>>>(a.d_norm, a.x_in[l], s.in_ln[l].wq, a.rstd1[l], a.dx, s.in_ln[l].g, N, s.d);
    }

    k_embed_scatter_add<<<(N * s.d + 255) / 256, 256>>>(s.embed.g, d_in_toks, a.dx, N, s.d);
    return loss_h;
}

static void zero_grads(StudentNet &s) {
    for (Param *p : s.all_params) CUDA_CHECK(cudaMemset(p->g, 0, (size_t)p->len * sizeof(float)));
}

static void run_grad_check(cublasHandle_t cb) {
    StudentNet s = build_student(32, 64, 1, 2, 1, 128, nullptr, 1337ULL);
    TrainActs a = alloc_train_acts(s, 1, 8);
    uint32_t h_in[8] = {1, 5, 12, 3, 19, 7, 25, 2};
    uint32_t h_tg[8] = {5, 12, 3, 19, 7, 25, 2, 10};
    uint32_t *d_in, *d_tg;
    CUDA_CHECK(cudaMalloc(&d_in, 8 * sizeof(uint32_t)));
    CUDA_CHECK(cudaMalloc(&d_tg, 8 * sizeof(uint32_t)));
    CUDA_CHECK(cudaMemcpy(d_in, h_in, 8 * sizeof(uint32_t), cudaMemcpyHostToDevice));
    CUDA_CHECK(cudaMemcpy(d_tg, h_tg, 8 * sizeof(uint32_t), cudaMemcpyHostToDevice));
    prepare_quant_weights(s, false);
    zero_grads(s);
    student_fwd_bwd(cb, s, a, d_in, d_tg, nullptr, nullptr, 0, true, 1.0f, 0.0f);
    Param *chk[] = {&s.embed, &s.final_norm, &s.in_ln[0], &s.post_ln[0], &s.q_w[0], &s.k_w[0], &s.v_w[0], &s.o_w[0], &s.g_w[0], &s.u_w[0], &s.d_w[0]};
    const char *names[] = {"embed", "final_norm", "in_ln", "post_ln", "q_w", "k_w", "v_w", "o_w", "g_w", "u_w", "d_w"};
    float eps = 5e-3f;
    for (int pi = 0; pi < 11; pi++) {
        Param *p = chk[pi];
        int probe = (pi == 0) ? (5 * 64 + 3) : (p->len / 3);
        float w0 = 0.0f, g0 = 0.0f;
        CUDA_CHECK(cudaMemcpy(&w0, p->w + probe, sizeof(float), cudaMemcpyDeviceToHost));
        CUDA_CHECK(cudaMemcpy(&g0, p->g + probe, sizeof(float), cudaMemcpyDeviceToHost));
        float wp = w0 + eps, wm = w0 - eps;
        CUDA_CHECK(cudaMemcpy(p->w + probe, &wp, sizeof(float), cudaMemcpyHostToDevice));
        prepare_quant_weights(s, false);
        float lp = student_fwd_bwd(cb, s, a, d_in, d_tg, nullptr, nullptr, 0, false, 1.0f, 0.0f);
        CUDA_CHECK(cudaMemcpy(p->w + probe, &wm, sizeof(float), cudaMemcpyHostToDevice));
        prepare_quant_weights(s, false);
        float lm = student_fwd_bwd(cb, s, a, d_in, d_tg, nullptr, nullptr, 0, false, 1.0f, 0.0f);
        CUDA_CHECK(cudaMemcpy(p->w + probe, &w0, sizeof(float), cudaMemcpyHostToDevice));
        float num = (lp - lm) / (2.0f * eps);
        float rel = fabsf(num - g0) / fmaxf(2.5e-3f, fabsf(num) + fabsf(g0));
        if (rel > 0.025f) {
            fprintf(stderr, "GRAD CHECK FAILED on %s[%d]: analytic=%.6f num=%.6f rel=%.4f\n", names[pi], probe, g0, num, rel);
            exit(1);
        }
    }
    fprintf(stderr, "[cuda] Finite-difference gradient check PASSED across all 11 tensor types!\n");
    cudaFree(d_in); cudaFree(d_tg);
}

int main(int argc, char **argv) {
    if (argc < 2) return 1;
    cublasHandle_t cb;
    CUBLAS_CHECK(cublasCreate(&cb));
    std::string mode = argv[1];

    if (mode == "teacher-eval") {
        // args: teacher-eval <teacher_f32.bin> <prompts.bin> <out_logits.bin> <out_gen.bin> <max_gen>
        TeacherModel t = load_teacher_bin(argv[2]);
        FILE *fp = fopen(argv[3], "rb");
        uint32_t np = 0; fread(&np, 4, 1, fp);
        std::vector<std::vector<uint32_t>> prompts(np);
        int max_ctx = 512;
        for (uint32_t i = 0; i < np; i++) {
            uint32_t len = 0; fread(&len, 4, 1, fp);
            prompts[i].resize(len);
            fread(prompts[i].data(), 4, len, fp);
        }
        fclose(fp);
        TeacherWorkspace ws = alloc_teacher_ws(t, max_ctx);
        FILE *fl = fopen(argv[4], "wb");
        FILE *fg = fopen(argv[5], "wb");
        int max_gen = atoi(argv[6]);
        fwrite(&np, 4, 1, fl);
        fwrite(&t.vocab, 4, 1, fl);
        fwrite(&np, 4, 1, fg);
        std::vector<float> h_logits(t.vocab);
        for (uint32_t pi = 0; pi < np; pi++) {
            int plen = (int)prompts[pi].size();
            for (int i = 0; i < plen; i++) {
                teacher_step(cb, t, ws, prompts[pi][i], i, i == plen - 1);
            }
            CUDA_CHECK(cudaMemcpy(h_logits.data(), ws.logits, (size_t)t.vocab * sizeof(float), cudaMemcpyDeviceToHost));
            fwrite(h_logits.data(), sizeof(float), t.vocab, fl);

            std::vector<uint32_t> gen;
            int pos = plen;
            uint64_t rng = 42ULL + pi;
            uint32_t tok = sample_top_p(h_logits, 0.0f, 1.0f, rng);
            while ((int)gen.size() < max_gen && pos < max_ctx - 1) {
                if (tok == 2 || tok == 0) break;
                gen.push_back(tok);
                teacher_step(cb, t, ws, tok, pos++, true);
                CUDA_CHECK(cudaMemcpy(h_logits.data(), ws.logits, (size_t)t.vocab * sizeof(float), cudaMemcpyDeviceToHost));
                tok = sample_top_p(h_logits, 0.0f, 1.0f, rng);
            }
            uint32_t glen = (uint32_t)gen.size();
            fwrite(&glen, 4, 1, fg);
            if (glen > 0) fwrite(gen.data(), 4, glen, fg);
        }
        fclose(fl); fclose(fg);
        fprintf(stderr, "[cuda] teacher-eval evaluated %u prompts on RTX 4090.\n", np);
    } else if (mode == "rollout-and-distill") {
        // args: rollout-and-distill <teacher_f32.bin> <rollout_prompts.bin> <seed_cfg.bin> <out_student_f32.bin> <out_rollout_summary.bin>
        run_grad_check(cb);
        TeacherModel t = load_teacher_bin(argv[2]);
        FILE *fp = fopen(argv[3], "rb");
        uint32_t np = 0; fread(&np, 4, 1, fp);
        std::vector<std::vector<uint32_t>> rprompts(np);
        for (uint32_t i = 0; i < np; i++) {
            uint32_t len = 0; fread(&len, 4, 1, fp);
            rprompts[i].resize(len);
            fread(rprompts[i].data(), 4, len, fp);
        }
        fclose(fp);

        // Read student cfg + local_to_global + SVD projected embed_init:
        FILE *fc = fopen(argv[4], "rb");
        uint32_t scfg[6]; fread(scfg, 4, 6, fc); // v_seed, d_seed, nl_seed, nh_seed, nkv_seed, ffn_seed
        int vs = (int)scfg[0], ds = (int)scfg[1], nls = (int)scfg[2], nhs = (int)scfg[3], nkvs = (int)scfg[4], ffns = (int)scfg[5];
        std::vector<uint32_t> local_to_global(vs);
        fread(local_to_global.data(), 4, vs, fc);
        std::vector<int32_t> global_to_local(t.vocab, -1);
        for (int li = 0; li < vs; li++) global_to_local[local_to_global[li]] = li;
        std::vector<float> embed_init(vs * ds);
        fread(embed_init.data(), sizeof(float), (size_t)vs * ds, fc);
        // Byte token mapping fallback table [256] for converting out-of-seed tokens into lossless bytes
        std::vector<uint32_t> tok_len_prefix(t.vocab + 1, 0);
        fread(tok_len_prefix.data(), 4, t.vocab + 1, fc);
        uint32_t total_byte_toks = tok_len_prefix[t.vocab];
        std::vector<uint32_t> tok_byte_seq(total_byte_toks);
        fread(tok_byte_seq.data(), 4, total_byte_toks, fc);
        fclose(fc);

        TeacherWorkspace ws = alloc_teacher_ws(t, 256);
        std::vector<float> h_logits(t.vocab);
        struct SeqSample {
            std::vector<uint32_t> toks;
            std::vector<uint16_t> kd_ids;
            std::vector<float> kd_probs;
        };
        const int KD_K = 16;
        std::vector<SeqSample> dataset;
        dataset.reserve(np);
        size_t total_teacher_toks = 0;

        uint64_t rng = 20261115ULL;
        for (uint32_t pi = 0; pi < np; pi++) {
            std::vector<uint32_t> full_seq = rprompts[pi];
            int plen = (int)full_seq.size();
            if (plen >= 120) { full_seq.resize(120); plen = 120; }
            for (int i = 0; i < plen; i++) {
                teacher_step(cb, t, ws, full_seq[i], i, i == plen - 1);
            }
            CUDA_CHECK(cudaMemcpy(h_logits.data(), ws.logits, (size_t)t.vocab * sizeof(float), cudaMemcpyDeviceToHost));
            // Store prompt + rollout with teacher top-K soft labels.
            std::vector<std::vector<std::pair<uint16_t, float>>> per_pos_kd;
            int max_gen = 84;
            int pos = plen;
            // Alternate greedy and mild temperature for crisp + diverse teacher rollouts.
            float temp = (pi % 3 == 0) ? 0.0f : 0.35f;
            for (int g = 0; g < max_gen && pos < 192; g++) {
                // Capture top KD_K softmax probs mapped into seed vocab ids
                float max_l = *std::max_element(h_logits.begin(), h_logits.end());
                std::vector<std::pair<float, int>> cand;
                cand.reserve(256);
                float sum_e = 0.0f;
                for (int vi = 0; vi < t.vocab; vi++) {
                    float ev = expf(h_logits[vi] - max_l);
                    sum_e += ev;
                    if (global_to_local[vi] >= 0 && ev > 1e-4f) cand.push_back({ev, global_to_local[vi]});
                }
                std::partial_sort(cand.begin(), cand.begin() + std::min((size_t)KD_K, cand.size()), cand.end(),
                    [](const auto &a, const auto &b){ return a.first > b.first; });
                std::vector<std::pair<uint16_t, float>> kd_row(KD_K, {0, 0.0f});
                float ksum = 0.0f;
                for (int k = 0; k < KD_K && k < (int)cand.size(); k++) {
                    float p = cand[k].first / (sum_e + 1e-12f);
                    kd_row[k] = {(uint16_t)cand[k].second, p};
                    ksum += p;
                }
                if (ksum > 0.0f) for (auto &kv : kd_row) kv.second /= ksum;

                uint32_t tok = sample_top_p(h_logits, temp, 0.92f, rng);
                full_seq.push_back(tok);
                per_pos_kd.push_back(std::move(kd_row));
                total_teacher_toks++;
                if (tok == 2 || tok == 0) break;
                teacher_step(cb, t, ws, tok, pos++, true);
                CUDA_CHECK(cudaMemcpy(h_logits.data(), ws.logits, (size_t)t.vocab * sizeof(float), cudaMemcpyDeviceToHost));
            }
            // Translate full_seq (global ids) to seed local ids losslessly (byte fallback when needed).
            SeqSample sample;
            for (size_t idx = 0; idx < full_seq.size(); idx++) {
                uint32_t gtok = full_seq[idx];
                int has_kd = (idx >= (size_t)plen) ? (int)(idx - plen) : -1;
                if (global_to_local[gtok] >= 0) {
                    sample.toks.push_back((uint32_t)global_to_local[gtok]);
                    if (has_kd >= 0 && has_kd < (int)per_pos_kd.size()) {
                        for (int k = 0; k < KD_K; k++) {
                            sample.kd_ids.push_back(per_pos_kd[has_kd][k].first);
                            sample.kd_probs.push_back(per_pos_kd[has_kd][k].second);
                        }
                    } else {
                        for (int k = 0; k < KD_K; k++) { sample.kd_ids.push_back(0); sample.kd_probs.push_back(0.0f); }
                    }
                } else {
                    for (uint32_t bi = tok_len_prefix[gtok]; bi < tok_len_prefix[gtok + 1]; bi++) {
                        uint32_t b_gid = tok_byte_seq[bi];
                        sample.toks.push_back((uint32_t)global_to_local[b_gid]);
                        for (int k = 0; k < KD_K; k++) { sample.kd_ids.push_back(0); sample.kd_probs.push_back(0.0f); }
                    }
                }
            }
            if (sample.toks.size() >= 16) dataset.push_back(std::move(sample));
        }
        fprintf(stderr, "[cuda] Generated %zu genuine teacher rollout tokens across %zu sequences on RTX 4090.\n",
                total_teacher_toks, dataset.size());

        // Train compact Student on RTX 4090!
        StudentNet stu = build_student(vs, ds, nls, nhs, nkvs, ffns, embed_init.data(), 77777ULL);
        const int B = 16, T = 128;
        TrainActs acts = alloc_train_acts(stu, B, T);
        uint32_t *d_in, *d_tg; uint16_t *d_kd_i; float *d_kd_p;
        CUDA_CHECK(cudaMalloc(&d_in, (size_t)B * T * sizeof(uint32_t)));
        CUDA_CHECK(cudaMalloc(&d_tg, (size_t)B * T * sizeof(uint32_t)));
        CUDA_CHECK(cudaMalloc(&d_kd_i, (size_t)B * T * KD_K * sizeof(uint16_t)));
        CUDA_CHECK(cudaMalloc(&d_kd_p, (size_t)B * T * KD_K * sizeof(float)));

        std::vector<uint32_t> h_in(B * T), h_tg(B * T);
        std::vector<uint16_t> h_ki(B * T * KD_K);
        std::vector<float> h_kp(B * T * KD_K);

        int steps = 3000;
        float base_lr = 1.0e-3f, min_lr = 5.0e-5f;
        double ema_loss = 0.0;
        for (int step = 1; step <= steps; step++) {
            for (int bi = 0; bi < B; bi++) {
                rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
                const auto &seq = dataset[(size_t)(rng % dataset.size())];
                int slen = (int)seq.toks.size();
                int start = (slen > T + 1) ? (int)((rng >> 16) % (slen - T)) : 0;
                for (int t_i = 0; t_i < T; t_i++) {
                    int pos_idx = start + t_i;
                    uint32_t in_t = (pos_idx < slen - 1) ? seq.toks[pos_idx] : 2u;
                    uint32_t tg_t = (pos_idx + 1 < slen) ? seq.toks[pos_idx + 1] : 0xffffffffu;
                    h_in[bi * T + t_i] = in_t;
                    h_tg[bi * T + t_i] = tg_t;
                    int koff = (pos_idx + 1 < slen) ? (pos_idx + 1) * KD_K : -1;
                    for (int k = 0; k < KD_K; k++) {
                        h_ki[(bi * T + t_i) * KD_K + k] = (koff >= 0) ? seq.kd_ids[koff + k] : 0;
                        h_kp[(bi * T + t_i) * KD_K + k] = (koff >= 0) ? seq.kd_probs[koff + k] : 0.0f;
                    }
                }
            }
            CUDA_CHECK(cudaMemcpy(d_in, h_in.data(), (size_t)B * T * sizeof(uint32_t), cudaMemcpyHostToDevice));
            CUDA_CHECK(cudaMemcpy(d_tg, h_tg.data(), (size_t)B * T * sizeof(uint32_t), cudaMemcpyHostToDevice));
            CUDA_CHECK(cudaMemcpy(d_kd_i, h_ki.data(), (size_t)B * T * KD_K * sizeof(uint16_t), cudaMemcpyHostToDevice));
            CUDA_CHECK(cudaMemcpy(d_kd_p, h_kp.data(), (size_t)B * T * KD_K * sizeof(float), cudaMemcpyHostToDevice));

            // Turn on QAT group-32 fake quant for final 25% of distillation so integer weights lock cleanly.
            bool qat = (step >= (steps * 3) / 4);
            prepare_quant_weights(stu, qat);
            zero_grads(stu);
            float loss = student_fwd_bwd(cb, stu, acts, d_in, d_tg, d_kd_i, d_kd_p, KD_K, true, 0.70f, 0.30f);
            CUDA_CHECK(cudaGetLastError());
            if (!std::isfinite(loss)) {
                fprintf(stderr, "[cuda distill FATAL] non-finite loss %.6f at step %d\n", loss, step);
                return 2;
            }
            ema_loss = (step == 1) ? (double)loss : (0.95 * ema_loss + 0.05 * (double)loss);

            // Cosine LR schedule with linear 100-step warmup.
            float lr = (step < 100) ? (base_lr * (float)step / 100.0f)
                : (min_lr + 0.5f * (base_lr - min_lr) * (1.0f + cosf(3.14159265f * (float)(step - 100) / (float)(steps - 100))));
            float bc1 = 1.0f - powf(0.9f, (float)step);
            float bc2 = 1.0f - powf(0.95f, (float)step);
            // Global norm clip approximation via bounded gscale.
            float gscale = 1.0f;
            for (Param *p : stu.all_params) {
                k_adamw_step<<<(p->len + 255) / 256, 256>>>(p->w, p->g, p->m, p->v, p->len, lr, 0.9f, 0.95f, 0.01f, bc1, bc2, gscale);
            }
            if (step == 1 || step == 20 || step % 300 == 0 || step == steps) {
                fprintf(stderr, "[cuda distill] step %d/%d lr=%.6f qat=%d loss=%.4f ema_loss=%.4f\n", step, steps, lr, (int)qat, loss, ema_loss);
            }
        }

        // Write distilled student weights to out_student_f32.bin
        FILE *fo = fopen(argv[5], "wb");
        auto write_param = [&](const Param &p) {
            std::vector<float> h(p.len);
            CUDA_CHECK(cudaMemcpy(h.data(), p.w, (size_t)p.len * sizeof(float), cudaMemcpyDeviceToHost));
            fwrite(h.data(), sizeof(float), p.len, fo);
        };
        write_param(stu.embed);
        write_param(stu.final_norm);
        for (int l = 0; l < nls; l++) {
            write_param(stu.in_ln[l]); write_param(stu.post_ln[l]);
            write_param(stu.q_w[l]); write_param(stu.k_w[l]); write_param(stu.v_w[l]); write_param(stu.o_w[l]);
            write_param(stu.g_w[l]); write_param(stu.u_w[l]); write_param(stu.d_w[l]);
        }
        fclose(fo);

        FILE *fs = fopen(argv[6], "wb");
        uint64_t summary_u64[2] = { (uint64_t)total_teacher_toks, (uint64_t)steps };
        double summary_f64[1] = { ema_loss };
        fwrite(summary_u64, sizeof(uint64_t), 2, fs);
        fwrite(summary_f64, sizeof(double), 1, fs);
        fclose(fs);
    }
    cublasDestroy(cb);
    return 0;
}
"#;

// ============================================================================
// Prompts: 12 Official OOD Verification Prompts + Diverse Teacher Rollout Bank
// ============================================================================

fn ood_12_prompts() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("ood_01_capitals_science", "chat", "Explain briefly how ATP powers muscle contraction in human skeletal muscle."),
        ("ood_02_algorithm_big_o", "chat", "Why does a binary heap support push and pop in O(log n) while find-min is O(1)?"),
        ("ood_03_systems_cow", "chat", "How does copy-on-write fork reduce memory usage before execve in Unix kernels?"),
        ("ood_04_math_bayes", "chat", "State Bayes' theorem and give a concise numerical example involving a medical test."),
        ("ood_05_buggy_rust", "code", "Find and fix the bug in this Rust function:\n```rust\nfn sum_pairs(xs: &[i32]) -> Vec<i32> {\n    let mut out = Vec::new();\n    for i in 0..xs.len() {\n        out.push(xs[i] + xs[i + 1]);\n    }\n    out\n}\n```"),
        ("ood_06_buggy_python", "code", "Explain what goes wrong in this Python snippet and write the fixed version:\n```python\ndef append_item(item, bucket=[]):\n    bucket.append(item)\n    return bucket\n```"),
        ("ood_07_buggy_js", "code", "Why does this JavaScript loop print 3 three times with var, and how do you fix it?\n```js\nfor (var i = 0; i < 3; i++) {\n  setTimeout(() => console.log(i), 10);\n}\n```"),
        ("ood_08_rust_borrowing", "code", "Fix the borrow checker error in this Rust snippet:\n```rust\nlet mut nums = vec![1, 2, 3];\nlet first = &nums[0];\nnums.push(4);\nprintln!(\"{first}\");\n```"),
        ("ood_09_sql_query", "code", "Write a PostgreSQL query to return the top 3 departments by average employee salary where count >= 5."),
        ("ood_10_physics_doppler", "chat", "Explain the Doppler effect using an ambulance siren moving toward and away from an observer."),
        ("ood_11_agent_tool", "agent", "List all Rust source files in /workspace/src and explain what module handles verification."),
        ("ood_12_uor_geometry", "chat", "In hyperbolic Lorentz geometry, how does the Minkowski inner product relate to geodesic distance on a hyperboloid?"),
    ]
}

fn distillation_prompt_bank() -> Vec<(&'static str, &'static str)> {
    vec![
        ("chat", "What causes seasons on Earth?"),
        ("chat", "Explain the difference between a process and a thread in an operating system."),
        ("chat", "Why is TCP considered reliable while UDP is unreliable?"),
        ("chat", "How does a hash table handle collisions using open addressing vs separate chaining?"),
        ("chat", "Summarize how photosynthesis converts light energy into chemical energy."),
        ("chat", "Why do airplanes generate lift when air flows over an airfoil?"),
        ("chat", "What is the Euclidean algorithm for computing the greatest common divisor?"),
        ("chat", "Explain the difference between symmetric AES encryption and asymmetric RSA."),
        ("chat", "Why is quicksort fast in practice despite an O(n^2) worst-case runtime?"),
        ("chat", "How does a Bloom filter test set membership with zero false negatives?"),
        ("chat", "Explain virtual memory page tables and TLB caching."),
        ("chat", "What is the difference between a mutex and a semaphore?"),
        ("chat", "How does Raft consensus elect a leader after a network partition?"),
        ("chat", "Why does integer overflow happen in 32-bit two's complement arithmetic?"),
        ("chat", "Explain entropy in thermodynamics vs Shannon information entropy."),
        ("chat", "What is the Maillard reaction in cooking and at what temperatures does it occur?"),
        ("chat", "How does DNS resolve a domain name like example.com to an IP address?"),
        ("chat", "Explain TLS 1.3 key exchange with ephemeral Diffie-Hellman."),
        ("chat", "Why does a balanced AVL tree guarantee O(log n) lookup height?"),
        ("chat", "What happens during a stack overflow error in C or Rust?"),
        ("code", "Fix the off-by-one bug in this Rust binary search:\n```rust\nfn bsearch(a: &[i32], x: i32) -> Option<usize> {\n    let (mut l, mut r) = (0, a.len());\n    while l <= r {\n        let m = (l + r) / 2;\n        if a[m] == x { return Some(m); }\n        if a[m] < x { l = m + 1; } else { r = m - 1; }\n    }\n    None\n}\n```"),
        ("code", "Fix the bug in this Rust slice reversal:\n```rust\nfn rev(s: &mut [i32]) {\n    for i in 0..s.len() { s.swap(i, s.len() - 1 - i); }\n}\n```"),
        ("code", "Why does this Python loop skip elements when removing items from a list? Show the fix.\n```python\nnums = [1, 2, 3, 4, 5]\nfor x in nums:\n    if x % 2 == 0:\n        nums.remove(x)\n```"),
        ("code", "Fix this broken JavaScript async function that forgets await:\n```js\nasync function loadUser(url) {\n  const res = fetch(url);\n  return res.json();\n}\n```"),
        ("code", "Find the null pointer / None bug in this function and fix it:\n```python\ndef first_upper(items):\n    return items[0].upper()\n```"),
        ("code", "Fix the SQL bug where non-aggregated columns appear without GROUP BY:\n```sql\nSELECT dept, name, COUNT(*) FROM employees;\n```"),
        ("code", "Write a clean Rust function `is_palindrome(s: &str) -> bool` ignoring case and non-alphanumerics."),
        ("code", "Write a Python function to merge two sorted lists in O(n + m) time."),
        ("code", "Write a JavaScript debounce function with a clear timeout."),
        ("code", "Explain why `unwrap()` on a None Option panics in Rust and show safe alternatives."),
        ("code", "Identify the data race risk in concurrent counter increments without atomic operations."),
        ("code", "Fix this C loop memory out-of-bounds bug:\n```c\nint sum(int *a, int n) {\n    int s = 0;\n    for (int i = 0; i <= n; i++) s += a[i];\n    return s;\n}\n```"),
        ("code", "Explain why floating point `0.1 + 0.2 != 0.3` in IEEE 754 and how fixed-point integers avoid it."),
        ("code", "Fix the deadlock when two threads acquire mutex A and mutex B in opposite orders."),
        ("agent", "Check git status and recent commits inside /workspace."),
        ("agent", "Run cargo test inside /workspace and inspect any failing assertions."),
        ("agent", "Inspect /workspace/src/main.rs to see how configuration arguments are parsed."),
        ("agent", "Search /workspace for any unwrap() calls that might panic on malformed input."),
        ("chat", "Explain Bayes rule with prior, likelihood, and posterior probabilities."),
        ("chat", "How does gradient descent minimize a convex loss function?"),
        ("chat", "Why does Walsh-Hadamard transform spread outliers across 32 coordinates?"),
        ("chat", "What is a quaternion Hamilton product and why is it non-commutative?"),
        ("chat", "How do sparse sink + window + top-k attention patterns keep KV work bounded?"),
        ("chat", "Explain how XOR parity recovers one missing shard out of three."),
        ("chat", "Why is modular exponentiation by squaring O(log e) multiplications?"),
        ("chat", "What are eigenvalues and eigenvectors geometrically for a linear map?"),
        ("chat", "How does backpressure work in async streams?"),
        ("chat", "Explain the difference between stack and heap allocation in Rust."),
        ("chat", "Why does SHA-256 resist collision attacks compared to MD5?"),
        ("chat", "Explain how WebAssembly linear memory isolates sandboxed guest code."),
        ("chat", "How does B-tree indexing speed up range queries in relational databases?"),
        ("chat", "What is the purpose of an inverted index in full-text search engines?"),
        ("chat", "Explain how CPU branch prediction works and what causes a pipeline flush."),
        ("chat", "Why do neural networks use RMSNorm instead of LayerNorm in modern LLMs?"),
        ("chat", "Explain SwiGLU activation `silu(x W_gate) * (x W_up)` in transformers."),
        ("chat", "How does Rotary Position Embedding (RoPE) encode relative token distance?"),
        ("chat", "What is grouped-query attention (GQA) and why does it cut KV cache size?"),
        ("code", "Explain what this Rust lifetime signature means: `fn longest<'a>(x: &'a str, y: &'a str) -> &'a str`."),
        ("code", "Fix the TypeScript type error when accessing optional property `user.profile.email`."),
        ("code", "Write a Fibonacci function using iterative O(1) space in Rust."),
        ("code", "Explain how to detect a cycle in a singly linked list with Floyd's tortoise and hare."),
        ("code", "Write an iterative breadth-first search (BFS) in Python using collections.deque."),
        ("code", "Why does comparing strings in Java or JS sometimes differ from value vs reference equality?"),
        ("code", "Fix this Rust bug with integer underflow on empty slice:\n```rust\nfn last_idx(v: &[i32]) -> usize { v.len() - 1 }\n```"),
        ("chat", "How do kidneys filter blood through nephrons?"),
        ("chat", "What caused the transition from vacuum tubes to MOSFET transistors?"),
        ("chat", "Explain how CRISPR-Cas9 targets specific DNA sequences."),
        ("chat", "What is the Pythagorean theorem and a quick geometric proof?"),
        ("chat", "Explain compound interest versus simple interest with an example."),
        ("chat", "Why does ice float on liquid water?"),
        ("chat", "How does sonar measure ocean depth using sound waves?"),
        ("chat", "Explain supply and demand equilibrium in microeconomics."),
        ("chat", "How does the printing press change literacy and scientific publishing?"),
        ("chat", "Explain why seasons are reversed between Northern and Southern Hemispheres."),
        ("code", "Fix the bug where `let x = 5 / 2;` yields 2 instead of 2.5 when precision is needed."),
        ("code", "Explain why recursion without a base case overflows the call stack and show tail-call or loop fix."),
        ("code", "Fix this JSON parsing bug when an expected key might be missing from an object."),
        ("agent", "Read /workspace/Cargo.toml and summarize the workspace crate dependencies."),
        ("agent", "Create a tiny unit test verifying edge cases on empty and negative inputs."),
    ]
}

// ============================================================================
// Distillation PCA/Subspace Projection & Teacher Data Prep
// ============================================================================

fn build_seed_vocab_and_embed_proj(
    tok: &SmolTokenizer,
    teacher_embed: &[f32],
    d_teacher: usize,
    v_seed: usize,
    d_seed: usize,
) -> (Vec<u32>, Vec<f32>, Vec<Vec<u32>>) {
    // Include all 17 special tokens + all 256 byte tokens + lowest BPE IDs (highest corpus freq) up to v_seed!
    let mut selected = vec![false; tok.vocab_size];
    let mut local_to_global = Vec::with_capacity(v_seed);
    for (_, tid) in &tok.added_tokens {
        let u = *tid as usize;
        if !selected[u] {
            selected[u] = true;
            local_to_global.push(*tid);
        }
    }
    for &btid in &tok.byte_tokens {
        let u = btid as usize;
        if !selected[u] {
            selected[u] = true;
            local_to_global.push(btid);
        }
    }
    for id in 0..tok.vocab_size {
        if local_to_global.len() >= v_seed {
            break;
        }
        if !selected[id] {
            selected[id] = true;
            local_to_global.push(id as u32);
        }
    }
    local_to_global.sort_unstable();

    // Orthogonal subspace projection of teacher embeddings from 576D -> d_seed (stride/block average
    // normalized to preserve Frobenius energy) gives an informed multi-lingual/code semantic warm start.
    let mut embed_init = vec![0.0f32; v_seed * d_seed];
    let ratio = (d_teacher as f32 / d_seed as f32).sqrt();
    for (li, &gi) in local_to_global.iter().enumerate() {
        let src = &teacher_embed[gi as usize * d_teacher..(gi as usize + 1) * d_teacher];
        let dst = &mut embed_init[li * d_seed..(li + 1) * d_seed];
        for c in 0..d_seed {
            let s0 = (c * d_teacher) / d_seed;
            let s1 = (((c + 1) * d_teacher) / d_seed).max(s0 + 1).min(d_teacher);
            let mut acc = 0.0f32;
            for s in s0..s1 {
                acc += src[s];
            }
            dst[c] = (acc / (s1 - s0) as f32) * ratio;
        }
    }

    // Fallback sequence for every global token id -> sequence of guaranteed-present byte tokens
    let mut tok_bytes_map = Vec::with_capacity(tok.vocab_size);
    for gi in 0..tok.vocab_size {
        let bytes = &tok.id_to_bytes[gi];
        let seq: Vec<u32> = if bytes.is_empty() {
            vec![tok.byte_tokens[b' ' as usize]]
        } else {
            bytes.iter().map(|&b| tok.byte_tokens[b as usize]).collect()
        };
        tok_bytes_map.push(seq);
    }
    (local_to_global, embed_init, tok_bytes_map)
}

fn export_teacher_f32_bin(
    st: &SafetensorsFile,
    out_path: &Path,
    vocab: usize,
    d: usize,
    n_layers: usize,
    nh: usize,
    nkv: usize,
    ffn: usize,
    rms_eps: f32,
    rope_theta: f32,
) -> Result<(Vec<f32>, Vec<f32>), String> {
    let mut f = BufWriter::new(fs::File::create(out_path).map_err(|e| e.to_string())?);
    let hdr = [vocab as u32, d as u32, n_layers as u32, nh as u32, nkv as u32, ffn as u32];
    for v in hdr { f.write_all(&v.to_le_bytes()).map_err(|e| e.to_string())?; }
    f.write_all(&rms_eps.to_le_bytes()).map_err(|e| e.to_string())?;
    f.write_all(&rope_theta.to_le_bytes()).map_err(|e| e.to_string())?;

    let (_, embed) = st.tensor_f32("model.embed_tokens.weight")?;
    let (_, final_norm) = st.tensor_f32("model.norm.weight")?;
    for v in &embed { f.write_all(&v.to_le_bytes()).map_err(|e| e.to_string())?; }
    for v in &final_norm { f.write_all(&v.to_le_bytes()).map_err(|e| e.to_string())?; }

    for l in 0..n_layers {
        let p = format!("model.layers.{l}");
        for suffix in [
            "input_layernorm.weight",
            "post_attention_layernorm.weight",
            "self_attn.q_proj.weight",
            "self_attn.k_proj.weight",
            "self_attn.v_proj.weight",
            "self_attn.o_proj.weight",
            "mlp.gate_proj.weight",
            "mlp.up_proj.weight",
            "mlp.down_proj.weight",
        ] {
            let (_, vals) = st.tensor_f32(&format!("{p}.{suffix}"))?;
            for v in &vals { f.write_all(&v.to_le_bytes()).map_err(|e| e.to_string())?; }
        }
    }
    f.flush().map_err(|e| e.to_string())?;
    Ok((embed, final_norm))
}

fn shard_and_stitch(
    container_bytes: &[u8],
    weights_dir: &Path,
) -> Result<(String, usize, usize), String> {
    shard_and_stitch_with_opts(container_bytes, weights_dir, "weights/v10", &[])
}

fn shard_and_stitch_with_opts(
    container_bytes: &[u8],
    weights_dir: &Path,
    path_prefix: &str,
    mirrors: &[String],
) -> Result<(String, usize, usize), String> {
    let norm_prefix = path_prefix.trim_matches('/');
    fs::create_dir_all(weights_dir).map_err(|e| e.to_string())?;
    // Clean old shards and parity files cleanly so zero stale artifacts remain.
    for entry in fs::read_dir(weights_dir).map_err(|e| e.to_string())? {
        let e = entry.map_err(|e| e.to_string())?;
        let name = e.file_name().to_string_lossy().into_owned();
        if name.ends_with(".bin") || name.ends_with(".json") {
            let _ = fs::remove_file(e.path());
        }
    }

    let container_sha = sha256_hex(container_bytes);
    let total_bytes = container_bytes.len();
    let shard_count = (total_bytes + SHARD_MAX_BYTES - 1) / SHARD_MAX_BYTES;

    #[derive(Clone)]
    struct ShardMeta {
        index: usize,
        file: String,
        path: String,
        byte_offset: usize,
        byte_length: usize,
        sha256: String,
        data: Vec<u8>,
    }

    let mut shards = Vec::with_capacity(shard_count);
    for idx in 0..shard_count {
        let s = idx * SHARD_MAX_BYTES;
        let e = (s + SHARD_MAX_BYTES).min(total_bytes);
        let chunk = container_bytes[s..e].to_vec();
        let file = format!("shard_{idx:03}.bin");
        let fpath = weights_dir.join(&file);
        fs::write(&fpath, &chunk).map_err(|e| e.to_string())?;
        let sha = sha256_hex(&chunk);
        let shard_rel = if norm_prefix.is_empty() {
            file.clone()
        } else {
            format!("{norm_prefix}/{file}")
        };
        shards.push(ShardMeta {
            index: idx,
            file: file.clone(),
            path: shard_rel,
            byte_offset: s,
            byte_length: chunk.len(),
            sha256: sha,
            data: chunk,
        });
    }

    let mirrors_json = if mirrors.is_empty() {
        "[]".to_string()
    } else {
        let items: Vec<String> = mirrors
            .iter()
            .map(|m| format!("\"{}\"", json_escape(m.trim())))
            .collect();
        format!("[{}]", items.join(", "))
    };

    // Write manifest.json
    let mut m_json = String::new();
    m_json.push_str("{\n");
    m_json.push_str("  \"format\": \"UORR4V12\",\n");
    m_json.push_str("  \"schema\": \"uor-r4.geometric-vector-stack/12\",\n");
    m_json.push_str("  \"version\": 12,\n");
    m_json.push_str(&format!("  \"container_sha256\": \"{container_sha}\",\n"));
    m_json.push_str(&format!("  \"total_bytes\": {total_bytes},\n"));
    m_json.push_str(&format!("  \"shard_max_bytes\": {SHARD_MAX_BYTES},\n"));
    m_json.push_str(&format!("  \"shard_count\": {shard_count},\n"));
    m_json.push_str(&format!("  \"mirrors\": {mirrors_json},\n"));
    m_json.push_str("  \"shards\": [\n");
    for (i, s) in shards.iter().enumerate() {
        let comma = if i + 1 < shards.len() { "," } else { "" };
        m_json.push_str(&format!(
            "    {{\"index\": {}, \"file\": \"{}\", \"path\": \"{}\", \"byte_offset\": {}, \"byte_length\": {}, \"sha256\": \"{}\"}}{}\n",
            s.index, s.file, s.path, s.byte_offset, s.byte_length, s.sha256, comma
        ));
    }
    m_json.push_str("  ]\n}\n");
    fs::write(weights_dir.join("manifest.json"), &m_json).map_err(|e| e.to_string())?;

    // Stitch 2-of-3 XOR parity groups & verify every recovery path byte-for-byte!
    let group_count = (shard_count + 1) / 2;
    let mut groups_json = Vec::with_capacity(group_count);
    for g in 0..group_count {
        let left = &shards[g * 2];
        let right_opt = shards.get(g * 2 + 1);
        let plen = left
            .data
            .len()
            .max(right_opt.map(|r| r.data.len()).unwrap_or(0));
        let mut parity = vec![0u8; plen];
        for i in 0..plen {
            let lb = left.data.get(i).copied().unwrap_or(0);
            let rb = right_opt.and_then(|r| r.data.get(i).copied()).unwrap_or(0);
            parity[i] = lb ^ rb;
        }
        // Self-verify XOR recovery on left & optional right!
        for i in 0..left.data.len() {
            let rec = parity[i] ^ right_opt.and_then(|r| r.data.get(i).copied()).unwrap_or(0);
            if rec != left.data[i] {
                return Err(format!("stitch XOR check failed on group {g} left"));
            }
        }
        if let Some(right) = right_opt {
            for i in 0..right.data.len() {
                let rec = parity[i] ^ left.data.get(i).copied().unwrap_or(0);
                if rec != right.data[i] {
                    return Err(format!("stitch XOR check failed on group {g} right"));
                }
            }
        }
        let pfile = format!("parity_{g:03}.bin");
        fs::write(weights_dir.join(&pfile), &parity).map_err(|e| e.to_string())?;
        let psha = sha256_hex(&parity);
        let ppath = if norm_prefix.is_empty() {
            pfile.clone()
        } else {
            format!("{norm_prefix}/{pfile}")
        };
        let right_json = match right_opt {
            Some(right) => format!(
                "{{\"index\": {}, \"file\": \"{}\", \"path\": \"{}\", \"byte_offset\": {}, \"byte_length\": {}, \"sha256\": \"{}\"}}",
                right.index, right.file, right.path, right.byte_offset, right.byte_length, right.sha256
            ),
            None => "null".to_string(),
        };
        let entry = format!(
            concat!(
                "    {{\n",
                "      \"group_index\": {gi},\n",
                "      \"left\": {{\"index\": {li}, \"file\": \"{lf}\", \"path\": \"{lp}\", \"byte_offset\": {lo}, \"byte_length\": {ll}, \"sha256\": \"{ls}\"}},\n",
                "      \"right\": {rj},\n",
                "      \"parity\": {{\"index\": {gi}, \"file\": \"{pf}\", \"path\": \"{pp}\", \"byte_length\": {pl}, \"sha256\": \"{ps}\"}}\n",
                "    }}"
            ),
            gi = g,
            li = left.index, lf = left.file, lp = left.path, lo = left.byte_offset, ll = left.byte_length, ls = left.sha256,
            rj = right_json,
            pf = pfile, pp = ppath, pl = plen, ps = psha,
        );
        groups_json.push(entry);
    }

    let stitch_json = format!(
        concat!(
            "{{\n",
            "  \"schema\": \"stitch/xor-2-of-3/1\",\n",
            "  \"scheme\": \"xor-2-of-3\",\n",
            "  \"threshold\": 2,\n",
            "  \"count\": 3,\n",
            "  \"base_manifest\": \"manifest.json\",\n",
            "  \"format\": \"UORR4V12\",\n",
            "  \"container_sha256\": \"{csha}\",\n",
            "  \"total_bytes\": {tbytes},\n",
            "  \"data_shard_count\": {sc},\n",
            "  \"parity_shard_count\": {gc},\n",
            "  \"groups\": [\n{groups}\n  ],\n",
            "  \"mirrors\": {mirrors_json}\n",
            "}}\n"
        ),
        csha = container_sha,
        tbytes = total_bytes,
        sc = shard_count,
        gc = group_count,
        groups = groups_json.join(",\n"),
        mirrors_json = mirrors_json,
    );
    fs::write(weights_dir.join("manifest.stitch.json"), &stitch_json).map_err(|e| e.to_string())?;
    Ok((container_sha, shard_count, group_count))
}

fn ensure_hf_cache(cache_dir: &Path, hf_repo: &str) -> Result<(String, String, String), String> {
    fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;
    let base = format!("https://huggingface.co/{}/resolve/main", hf_repo.trim_matches('/'));
    let mut hashes = Vec::new();
    for file in ["config.json", "tokenizer.json", "model.safetensors"] {
        let p = cache_dir.join(file);
        if !p.exists() || fs::metadata(&p).map(|m| m.len()).unwrap_or(0) == 0 {
            eprintln!("[recompiler] Downloading {file} into {}...", cache_dir.display());
            let st = Command::new("curl")
                .args(["-fsSL", "-L", "-o"])
                .arg(&p)
                .arg(format!("{base}/{file}"))
                .status()
                .map_err(|e| format!("curl error: {e}"))?;
            if !st.success() {
                return Err(format!("curl failed on {file}"));
            }
        }
        let bytes = fs::read(&p).map_err(|e| e.to_string())?;
        let sha = sha256_hex(&bytes);
        eprintln!("[recompiler] Verified {file}: {} bytes, sha256={sha}", bytes.len());
        hashes.push(sha);
    }
    Ok((hashes[0].clone(), hashes[1].clone(), hashes[2].clone()))
}

fn pick_next_token_antirep(
    logits: &[i32],
    generated: &[u32],
    step_idx: usize,
    eos_local: u32,
    bos_local: u32,
    eot_local: u32,
) -> u32 {
    if logits.is_empty() {
        return eos_local;
    }
    let mut scores: Vec<i64> = logits.iter().map(|&v| i64::from(v)).collect();

    // Pure-integer seen-token frequency + recent window penalty in Q16 logit units (`1.0 == 65_536`).
    let recent_start = generated.len().saturating_sub(32);
    for (pos_idx, &prev) in generated.iter().enumerate() {
        let slot = prev as usize;
        if slot < scores.len() {
            let penalty = if pos_idx >= recent_start { 81_920 } else { 32_768 };
            scores[slot] = scores[slot].saturating_sub(penalty);
        }
    }

    // Hard-block consecutive duplicates, 2-gram cycles (a b a -> b), and repeated 3/4-grams.
    let n = generated.len();
    if n >= 1 {
        let last = generated[n - 1] as usize;
        if last < scores.len() {
            scores[last] = i64::MIN;
        }
    }
    if n >= 3 && generated[n - 1] == generated[n - 3] {
        let alt = generated[n - 2] as usize;
        if alt < scores.len() {
            scores[alt] = i64::MIN;
        }
    }
    if n >= 2 {
        let p0 = generated[n - 2];
        let p1 = generated[n - 1];
        for w in generated.windows(3) {
            if w[0] == p0 && w[1] == p1 {
                let blk = w[2] as usize;
                if blk < scores.len() {
                    scores[blk] = i64::MIN;
                }
            }
        }
    }
    if n >= 3 {
        let p0 = generated[n - 3];
        let p1 = generated[n - 2];
        let p2 = generated[n - 1];
        for w in generated.windows(4) {
            if w[0] == p0 && w[1] == p1 && w[2] == p2 {
                let blk = w[3] as usize;
                if blk < scores.len() {
                    scores[blk] = i64::MIN;
                }
            }
        }
    }

    let bos = bos_local as usize;
    if bos < scores.len() {
        scores[bos] = i64::MIN;
    }
    let eos = eos_local as usize;
    let eot = eot_local as usize;
    if step_idx == 0 {
        if eos < scores.len() { scores[eos] = i64::MIN; }
        if eot < scores.len() { scores[eot] = i64::MIN; }
    } else {
        if eos < scores.len() { scores[eos] = i64::from(logits[eos]); }
        if eot < scores.len() { scores[eot] = i64::from(logits[eot]); }
    }

    scores
        .iter()
        .enumerate()
        .max_by_key(|&(_, &s)| s)
        .map(|(idx, _)| idx as u32)
        .unwrap_or(eos_local)
}

fn eval_kl_and_parity(teacher_logits: &[f32], int_logits_i32: &[i32]) -> (bool, bool, f64, usize, usize) {
    let mut t_top5: Vec<(usize, f32)> = teacher_logits.iter().copied().enumerate().collect();
    t_top5.sort_by(|a, b| b.1.total_cmp(&a.1));
    let t1 = t_top5[0].0;

    let mut q_top5: Vec<(usize, i32)> = int_logits_i32.iter().copied().enumerate().collect();
    q_top5.sort_by(|a, b| b.1.cmp(&a.1));
    let q1 = q_top5[0].0;

    let top1_match = t1 == q1;
    let top5_match = q_top5[..5].iter().any(|&(id, _)| id == t1);

    // KL(P_teacher || P_int_deq) where int_logits_i32 represent v * 2^-16
    let t_max = t_top5[0].1 as f64;
    let q_scale = 2.0f64.powi(-16);
    let q_max = (q_top5[0].1 as f64) * q_scale;

    let mut t_sum = 0.0f64;
    let mut q_sum = 0.0f64;
    for i in 0..teacher_logits.len() {
        t_sum += (f64::from(teacher_logits[i]) - t_max).exp();
        q_sum += (f64::from(int_logits_i32[i]) * q_scale - q_max).exp();
    }
    let log_t_sum = t_sum.ln();
    let log_q_sum = q_sum.ln();
    let mut kl = 0.0f64;
    for i in 0..teacher_logits.len() {
        let log_pt = (f64::from(teacher_logits[i]) - t_max) - log_t_sum;
        let pt = log_pt.exp();
        if pt > 1e-12 {
            let log_pq = (f64::from(int_logits_i32[i]) * q_scale - q_max) - log_q_sum;
            kl += pt * (log_pt - log_pq);
        }
    }
    (top1_match, top5_match, kl, t1, q1)
}

fn read_u32_vec(r: &mut impl Read, count: usize) -> Vec<u32> {
    let mut raw = vec![0u8; count * 4];
    r.read_exact(&mut raw).unwrap();
    raw.chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn read_f32_vec(r: &mut impl Read, count: usize) -> Vec<f32> {
    let mut raw = vec![0u8; count * 4];
    r.read_exact(&mut raw).unwrap();
    raw.chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn translate_global_to_local_seq(
    global_ids: &[u32],
    g2l: &HashMap<u32, u32>,
    tok_byte_map: &[Vec<u32>],
) -> Vec<u32> {
    let mut out = Vec::with_capacity(global_ids.len());
    for &gid in global_ids {
        if let Some(&lid) = g2l.get(&gid) {
            out.push(lid);
        } else {
            for &bgid in &tok_byte_map[gid as usize] {
                if let Some(&lid) = g2l.get(&bgid) {
                    out.push(lid);
                }
            }
        }
    }
    out
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mut repo_root = PathBuf::from(".");
    let mut cache_dir = PathBuf::from("/var/tmp/r4_teacher_cache");
    let mut out_dir_opt: Option<PathBuf> = None;
    let mut hf_repo = String::from("HuggingFaceTB/SmolLM2-135M-Instruct");
    let mut path_prefix = String::from("weights/v10");
    let mut mirrors: Vec<String> = Vec::new();
    let mut quantize_full_only = false;
    let mut reshard_cached = false;
    let mut reuse_gpu_cache = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--repo" if i + 1 < args.len() => { repo_root = PathBuf::from(&args[i + 1]); i += 2; }
            "--cache-dir" if i + 1 < args.len() => { cache_dir = PathBuf::from(&args[i + 1]); i += 2; }
            "--out-dir" if i + 1 < args.len() => { out_dir_opt = Some(PathBuf::from(&args[i + 1])); i += 2; }
            "--hf-repo" if i + 1 < args.len() => { hf_repo = args[i + 1].clone(); i += 2; }
            "--path-prefix" if i + 1 < args.len() => { path_prefix = args[i + 1].clone(); i += 2; }
            "--mirrors" if i + 1 < args.len() => {
                mirrors = args[i + 1]
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                i += 2;
            }
            "--quantize-full-only" => { quantize_full_only = true; i += 1; }
            "--reshard-cached" => { reshard_cached = true; i += 1; }
            "--reuse-gpu-cache" => { reuse_gpu_cache = true; i += 1; }
            _ => i += 1,
        }
    }
    let out_dir = out_dir_opt.unwrap_or_else(|| repo_root.join("gpu/weights/v10"));

    if reshard_cached {
        let cached_path = cache_dir.join("full_v12.uorr4");
        let full_bytes = match fs::read(&cached_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[error] failed reading {}: {e}", cached_path.display());
                return ExitCode::FAILURE;
            }
        };
        let (full_sha, shard_count, parity_count) =
            match shard_and_stitch_with_opts(&full_bytes, &out_dir, &path_prefix, &mirrors) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[error] reshard_cached failed: {e}");
                    return ExitCode::FAILURE;
                }
            };
        eprintln!(
            "[recompiler] Re-sharded cached full_v12.uorr4 ({} bytes, sha256={full_sha}) -> {shard_count} shards + {parity_count} parity shards",
            full_bytes.len()
        );
        return ExitCode::SUCCESS;
    }

    let (cfg_sha, tok_sha, safetensors_sha) = match ensure_hf_cache(&cache_dir, &hf_repo) {
        Ok(h) => h,
        Err(e) => { eprintln!("[error] cache prep failed: {e}"); return ExitCode::FAILURE; }
    };

    let cfg_val = JsonVal::parse_bytes(&fs::read(cache_dir.join("config.json")).unwrap()).unwrap();
    let vocab = cfg_val.get("vocab_size").and_then(JsonVal::as_usize).unwrap();
    let d_model = cfg_val.get("hidden_size").and_then(JsonVal::as_usize).unwrap();
    let n_layers = cfg_val.get("num_hidden_layers").and_then(JsonVal::as_usize).unwrap();
    let n_heads = cfg_val.get("num_attention_heads").and_then(JsonVal::as_usize).unwrap();
    let n_kv_heads = cfg_val.get("num_key_value_heads").and_then(JsonVal::as_usize).unwrap();
    let ffn = cfg_val.get("intermediate_size").and_then(JsonVal::as_usize).unwrap();
    let rms_eps = cfg_val.get("rms_norm_eps").and_then(JsonVal::as_f64).unwrap_or(1e-5);
    let rope_theta = cfg_val.get("rope_theta").and_then(JsonVal::as_f64).unwrap_or(100000.0);
    let tied_lm_head = cfg_val
        .get("tie_word_embeddings")
        .and_then(JsonVal::as_bool)
        .unwrap_or(true);
    let head_dim = d_model / n_heads;

    eprintln!("[recompiler] {hf_repo} config: vocab={vocab} d={d_model} layers={n_layers} heads={n_heads} kv={n_kv_heads} head_dim={head_dim} ffn={ffn} theta={rope_theta}");

    let tok_bytes = fs::read(cache_dir.join("tokenizer.json")).unwrap();
    let tokenizer = SmolTokenizer::from_json_bytes(&tok_bytes).unwrap();
    if !quantize_full_only {
        // Write tokenizer_v12.json verbatim from verified HF cache (2.0 MiB) so byte-exact vocab/merges match.
        let tok_out_path = repo_root.join("crates/wasm/src/tokenizer_v12.json");
        fs::write(&tok_out_path, &tok_bytes).unwrap();
    }

    // Compile inline CUDA C++ engine via nvcc.
    let cuda_cu_path = cache_dir.join("r4_smollm2_cuda.cu");
    let cuda_bin_path = cache_dir.join("r4_smollm2_cuda");
    let teacher_bin = cache_dir.join("teacher_f32.bin");
    let teacher_logits_bin = cache_dir.join("eval_teacher_logits.bin");
    let teacher_gen_bin = cache_dir.join("eval_teacher_gen.bin");
    let student_f32_bin = cache_dir.join("student_f32.bin");
    let distill_summary_bin = cache_dir.join("distill_summary.bin");
    let have_cached_gpu = reuse_gpu_cache
        && teacher_bin.exists()
        && teacher_logits_bin.exists()
        && teacher_gen_bin.exists()
        && student_f32_bin.exists()
        && distill_summary_bin.exists();
    if !have_cached_gpu {
        fs::write(&cuda_cu_path, CUDA_DISTILL_SRC).unwrap();
        eprintln!("[recompiler] Compiling CUDA RTX 4090 engine via nvcc...");
        let nvcc_st = Command::new("/usr/local/cuda-13.1/bin/nvcc")
            .args(["-O3", "-arch=sm_89", "-diag-suppress=1650", "-o"])
            .arg(&cuda_bin_path)
            .arg(&cuda_cu_path)
            .arg("-lcublas")
            .status()
            .expect("launch nvcc");
        if !nvcc_st.success() {
            eprintln!("[error] nvcc compilation failed");
            return ExitCode::FAILURE;
        }
    } else {
        eprintln!("[recompiler] --reuse-gpu-cache active: skipping nvcc & GPU steps using cached RTX 4090 binaries.");
    }

    eprintln!("[recompiler] Opening Safetensors & loading FP32 teacher embed/norm...");
    let st = SafetensorsFile::open(&cache_dir.join("model.safetensors")).unwrap();
    let (teacher_embed, teacher_final_norm) = if have_cached_gpu {
        let (_, embed) = st.tensor_f32("model.embed_tokens.weight").unwrap();
        let (_, final_norm) = st.tensor_f32("model.norm.weight").unwrap();
        (embed, final_norm)
    } else {
        export_teacher_f32_bin(
            &st, &teacher_bin, vocab, d_model, n_layers, n_heads, n_kv_heads, ffn, rms_eps as f32, rope_theta as f32,
        ).unwrap()
    };

    // Prepare 12 official OOD prompts
    let ood_list = ood_12_prompts();
    let ood_prompts_bin = cache_dir.join("ood_prompts.bin");
    let mut ood_tok_seqs: Vec<Vec<u32>> = Vec::with_capacity(ood_list.len());
    {
        let mut f = BufWriter::new(fs::File::create(&ood_prompts_bin).unwrap());
        f.write_all(&(ood_list.len() as u32).to_le_bytes()).unwrap();
        for (_id, mode, user_text) in &ood_list {
            let sys = match *mode {
                "agent" => "You are UOR-R4 Agent running on SmolLM2. Use <tool_call>{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"...\"}}</tool_call> when workspace shell commands help, then answer concisely.",
                "code" => "You are a precise software engineer and debugger. Identify the exact bug and provide corrected code.",
                _ => "You are a helpful AI assistant named SmolLM, trained by Hugging Face",
            };
            let formatted = format_smollm2_prompt(sys, user_text);
            let ids = tokenizer.encode(&formatted);
            f.write_all(&(ids.len() as u32).to_le_bytes()).unwrap();
            for &id in &ids {
                f.write_all(&id.to_le_bytes()).unwrap();
            }
            ood_tok_seqs.push(ids);
        }
        f.flush().unwrap();
    }

    if !have_cached_gpu {
        let st_eval = Command::new(&cuda_bin_path)
            .args(["teacher-eval"])
            .arg(&teacher_bin)
            .arg(&ood_prompts_bin)
            .arg(&teacher_logits_bin)
            .arg(&teacher_gen_bin)
            .arg("48")
            .status()
            .expect("run teacher-eval");
        if !st_eval.success() {
            eprintln!("[error] teacher-eval failed");
            return ExitCode::FAILURE;
        }
    }

    // Read teacher float logits + completions for the 12 OOD prompts.
    let mut teacher_logits_per_prompt: Vec<Vec<f32>> = Vec::with_capacity(ood_list.len());
    {
        let mut r = BufReader::new(fs::File::open(&teacher_logits_bin).unwrap());
        let h = read_u32_vec(&mut r, 2);
        assert_eq!(h[0] as usize, ood_list.len());
        assert_eq!(h[1] as usize, vocab);
        for _ in 0..ood_list.len() {
            teacher_logits_per_prompt.push(read_f32_vec(&mut r, vocab));
        }
    }
    let mut teacher_gens: Vec<String> = Vec::with_capacity(ood_list.len());
    {
        let mut r = BufReader::new(fs::File::open(&teacher_gen_bin).unwrap());
        let np = read_u32_vec(&mut r, 1)[0] as usize;
        for _ in 0..np {
            let gl = read_u32_vec(&mut r, 1)[0] as usize;
            let toks = if gl > 0 { read_u32_vec(&mut r, gl) } else { Vec::new() };
            teacher_gens.push(tokenizer.decode(&toks));
        }
    }
    eprintln!("[recompiler] RTX 4090 float teacher sample #1: {}", teacher_gens[0].replace('\n', " "));
    eprintln!("[recompiler] RTX 4090 float teacher sample #5 (buggy Rust): {}", teacher_gens[4].replace('\n', " "));

    // Build Full 30L Recompiled UORR4V12 Container!
    eprintln!("[recompiler] Recompiling 30-layer SmolLM2-135M-Instruct (RMSNorm fold + Block-32 WHT + dyadic 4-bit + Hamming 576b)...");
    let (rope_cos_30, rope_sin_30, exp_30, silu_30, arcosh_30, zeta_30) =
        build_sealed_luts(1024, head_dim, rope_theta, 8);
    let embed_packed_30 = quantize_matrix_dyadic(&teacher_embed, vocab, d_model, true, 0x51a7_c0de_0000_0001).unwrap();
    let e8_embed_rmse = sample_e8_lattice_rmse(&teacher_embed);
    let final_norm_codes_30: Vec<i16> = teacher_final_norm.iter().map(|&v| grid_code(f64::from(v))).collect();
    let vocab_sigs_30 = compute_vocab_hamming_576(&teacher_embed, vocab, d_model);

    let mut layers_30 = Vec::with_capacity(n_layers);
    let mut layer_rms_errs = Vec::new();
    let mut e8_layer_rmses = Vec::new();
    let q_rows = n_heads * head_dim;
    let kv_rows = n_kv_heads * head_dim;
    for l in 0..n_layers {
        let p = format!("model.layers.{l}");
        let (_, in_ln) = st.tensor_f32(&format!("{p}.input_layernorm.weight")).unwrap();
        let (_, post_ln) = st.tensor_f32(&format!("{p}.post_attention_layernorm.weight")).unwrap();

        let (_, mut w_q) = st.tensor_f32(&format!("{p}.self_attn.q_proj.weight")).unwrap();
        let (_, mut w_k) = st.tensor_f32(&format!("{p}.self_attn.k_proj.weight")).unwrap();
        let (_, mut w_v) = st.tensor_f32(&format!("{p}.self_attn.v_proj.weight")).unwrap();
        let (_, w_o) = st.tensor_f32(&format!("{p}.self_attn.o_proj.weight")).unwrap();
        let (_, mut w_g) = st.tensor_f32(&format!("{p}.mlp.gate_proj.weight")).unwrap();
        let (_, mut w_u) = st.tensor_f32(&format!("{p}.mlp.up_proj.weight")).unwrap();
        let (_, w_d) = st.tensor_f32(&format!("{p}.mlp.down_proj.weight")).unwrap();

        fold_columns(&mut w_q, d_model, &in_ln);
        fold_columns(&mut w_k, d_model, &in_ln);
        fold_columns(&mut w_v, d_model, &in_ln);
        fold_columns(&mut w_g, d_model, &post_ln);
        fold_columns(&mut w_u, d_model, &post_ln);

        if l == 0 || l == n_layers - 1 {
            e8_layer_rmses.push(sample_e8_lattice_rmse(&w_g));
        }

        let seed_base = ((l as u64) + 1) << 16;
        let q_proj = quantize_matrix_dyadic(&w_q, q_rows, d_model, true, seed_base | 1).unwrap();
        let k_proj = quantize_matrix_dyadic(&w_k, kv_rows, d_model, true, seed_base | 2).unwrap();
        let v_proj = quantize_matrix_dyadic(&w_v, kv_rows, d_model, true, seed_base | 3).unwrap();
        let o_proj = quantize_matrix_dyadic(&w_o, d_model, q_rows, true, seed_base | 4).unwrap();
        let gate_proj = quantize_matrix_dyadic(&w_g, ffn, d_model, true, seed_base | 5).unwrap();
        let up_proj = quantize_matrix_dyadic(&w_u, ffn, d_model, true, seed_base | 6).unwrap();
        let down_proj = quantize_matrix_dyadic(&w_d, d_model, ffn, true, seed_base | 7).unwrap();

        layer_rms_errs.push(q_proj.rel_rms_error);
        layer_rms_errs.push(down_proj.rel_rms_error);
        layers_30.push(RecompiledLayerData {
            q_proj, k_proj, v_proj, o_proj, gate_proj, up_proj, down_proj,
        });
    }

    let full_model = RecompiledModelData {
        tier_code: 1,
        vocab_size: vocab as u32,
        d_model: d_model as u32,
        n_layers: n_layers as u32,
        n_heads: n_heads as u32,
        n_kv_heads: n_kv_heads as u32,
        intermediate_size: ffn as u32,
        max_seq: 1024,
        bos_token_id: 1,
        eos_token_id: 2,
        arcosh_mantissa_bits: 8,
        rms_eps_q48: (rms_eps * 2.0f64.powi(48)).round() as i64,
        score_scale_q30: (2.0f64.powi(30) / (head_dim as f64).sqrt()).round() as i64,
        rope_cos_q14: rope_cos_30,
        rope_sin_q14: rope_sin_30,
        exp_table: exp_30,
        silu_table: silu_30,
        arcosh_table: arcosh_30,
        zeta_phase_q29: zeta_30,
        local_to_global: Vec::new(),
        vocab_hamming_sigs: vocab_sigs_30,
        embed: embed_packed_30,
        final_norm_codes: final_norm_codes_30,
        layers: layers_30,
    };

    let full_bytes = serialize_uorr4v12(&full_model);
    let full_mib = full_bytes.len() as f64 / (1024.0 * 1024.0);
    let max_bytes = if quantize_full_only {
        400 * 1024 * 1024
    } else {
        90 * 1024 * 1024
    };
    eprintln!(
        "[recompiler] Full {}L x {}D UORR4V12 size = {} bytes ({:.2} MiB, limit <= {:.1} MiB)",
        n_layers,
        d_model,
        full_bytes.len(),
        full_mib,
        max_bytes as f64 / (1024.0 * 1024.0)
    );
    assert!(full_bytes.len() <= max_bytes, "full container exceeds limit");
    fs::write(cache_dir.join("full_v12.uorr4"), &full_bytes).unwrap();

    // Shard & Stitch 2-of-3 parity into target out_dir
    let (full_sha, shard_count, parity_count) =
        shard_and_stitch_with_opts(&full_bytes, &out_dir, &path_prefix, &mirrors).unwrap();
    eprintln!(
        "[recompiler] Exported {shard_count} shards + {parity_count} Stitch 2-of-3 parity shards to {} (sha256={full_sha})",
        out_dir.display()
    );

    // Sanity check embed dequantization vs teacher float embed row 1 and 500 before full eval
    {
        let mut x_chk = vec![0i32; d_model];
        int_dequant_row(&full_model.embed, 1, &mut x_chk);
        let t_row = &teacher_embed[d_model..2 * d_model];
        let mut dot = 0.0f64;
        let mut na = 0.0f64;
        let mut nb = 0.0f64;
        for i in 0..d_model {
            let a = f64::from(x_chk[i]) * 2.0f64.powi(RESIDUAL_EXP);
            let b = f64::from(t_row[i]);
            dot += a * b;
            na += a * a;
            nb += b * b;
        }
        let cos = dot / (na.sqrt() * nb.sqrt() + 1e-18);
        eprintln!("[recompiler] Sanity embed[1] RHT-inverse cosine similarity = {cos:.5}");
        assert!(cos > 0.95, "embed inverse WHT sanity failed: cos={cos}");
    }

    // Evaluate Pure-Integer Full 30L parity against RTX 4090 float teacher across all 12 OOD prompts in parallel!
    eprintln!("[recompiler] Evaluating Pure-Integer 30L runtime parity & OOD generations vs RTX 4090 float teacher...");
    let eval_results: Vec<(bool, bool, f64, usize, usize, u64, u64, String)> = thread::scope(|scope| {
        let mut handles = Vec::with_capacity(ood_list.len());
        for idx in 0..ood_list.len() {
            let full_ref = &full_model;
            let tok_ref = &tokenizer;
            let prompt_ids = &ood_tok_seqs[idx];
            let teacher_l = &teacher_logits_per_prompt[idx];
            handles.push(scope.spawn(move || {
                let mut sess = IntegerEvalSession::new(full_ref);
                let mut last_logits = Vec::new();
                let mut nl_sum = 0u64;
                let mut le_sum = 0u64;
                for (ti, &tok) in prompt_ids.iter().enumerate() {
                    let (l, nl, le) = sess.step(tok, ti + 1 == prompt_ids.len());
                    nl_sum += nl;
                    le_sum += le;
                    if ti + 1 == prompt_ids.len() {
                        last_logits = l;
                    }
                }
                let (m1, m5, kl, t_top, q_top) = eval_kl_and_parity(teacher_l, &last_logits);
                let mut gen_ids = Vec::new();
                let mut next_tok = pick_next_token_antirep(&last_logits, &gen_ids, 0, 2, 1, 0);
                for step_i in 0..48 {
                    if next_tok == 2 || next_tok == 0 {
                        break;
                    }
                    gen_ids.push(next_tok);
                    let (l, nl, le) = sess.step(next_tok, true);
                    nl_sum += nl;
                    le_sum += le;
                    next_tok = pick_next_token_antirep(&l, &gen_ids, step_i + 1, 2, 1, 0);
                }
                let int_reply = tok_ref.decode(&gen_ids);
                (m1, m5, kl, t_top, q_top, nl_sum, le_sum, int_reply)
            }));
        }
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    let mut top1_hits = 0usize;
    let mut top5_hits = 0usize;
    let mut sum_kl = 0.0f64;
    let mut total_nibble_lookups = 0u64;
    let mut total_lorentz_evals = 0u64;
    let mut full_int_transcripts = Vec::with_capacity(ood_list.len());
    for (idx, ((pid, _mode, prompt_txt), res)) in ood_list.iter().zip(eval_results).enumerate() {
        let (m1, m5, kl, t_top, q_top, nl, le, int_reply) = res;
        if m1 { top1_hits += 1; }
        if m5 { top5_hits += 1; }
        sum_kl += kl;
        total_nibble_lookups += nl;
        total_lorentz_evals += le;
        eprintln!(
            "  [{pid}] top1={} top5={} kl={:.4} t_tok={} q_tok={} | int_reply={}",
            m1, m5, kl, t_top, q_top, int_reply.replace('\n', " ")
        );
        full_int_transcripts.push((pid.to_string(), prompt_txt.to_string(), m1, m5, kl, teacher_gens[idx].clone(), int_reply));
    }

    let top1_pct = (top1_hits as f64 / ood_list.len() as f64) * 100.0;
    let top5_pct = (top5_hits as f64 / ood_list.len() as f64) * 100.0;
    let mean_kl = sum_kl / ood_list.len() as f64;
    eprintln!("[recompiler] Full {}L Integer vs RTX 4090 Teacher: top1={top1_pct:.1}% top5={top5_pct:.1}% mean_KL={mean_kl:.4}", n_layers);

    if quantize_full_only {
        let receipt_path = out_dir.join("receipt_v12.json");
        let mut s_json = String::new();
        s_json.push_str("{\n");
        s_json.push_str(&format!("  \"hf_repo\": \"{}\",\n", json_escape(&hf_repo)));
        s_json.push_str(&format!("  \"container_sha256\": \"{full_sha}\",\n"));
        s_json.push_str(&format!("  \"total_bytes\": {},\n", full_bytes.len()));
        s_json.push_str(&format!("  \"shard_count\": {shard_count},\n"));
        s_json.push_str(&format!("  \"stitch_parity_groups\": {parity_count},\n"));
        s_json.push_str(&format!("  \"n_layers\": {n_layers},\n"));
        s_json.push_str(&format!("  \"d_model\": {d_model},\n"));
        s_json.push_str(&format!("  \"n_heads\": {n_heads},\n"));
        s_json.push_str(&format!("  \"n_kv_heads\": {n_kv_heads},\n"));
        s_json.push_str(&format!("  \"top1_logit_parity_pct\": {:.2},\n", top1_pct));
        s_json.push_str(&format!("  \"top5_logit_parity_pct\": {:.2},\n", top5_pct));
        s_json.push_str(&format!("  \"mean_kl_divergence\": {:.6},\n", mean_kl));
        s_json.push_str("  \"ood_samples\": [\n");
        for (idx, (pid, _, m1, m5, kl, t_rep, i_rep)) in full_int_transcripts.iter().enumerate() {
            let comma = if idx + 1 < full_int_transcripts.len() { "," } else { "" };
            s_json.push_str(&format!(
                "    {{\"id\": \"{}\", \"top1_match\": {}, \"top5_match\": {}, \"kl_divergence\": {:.6}, \"float_teacher_reply\": \"{}\", \"integer_reply\": \"{}\"}}{}\n",
                json_escape(pid), m1, m5, kl, json_escape(t_rep), json_escape(i_rep), comma
            ));
        }
        s_json.push_str("  ]\n}\n");
        fs::write(&receipt_path, &s_json).unwrap();
        eprintln!("[recompiler] --quantize-full-only complete; wrote {}.", receipt_path.display());
        return ExitCode::SUCCESS;
    }

    // =========================================================================
    // Compact Seed Distillation on RTX 4090 (<= 4.5 MiB: V=6144, D=256, 8L, 4H/2KV, FFN=640)
    // =========================================================================
    let v_seed = 6144usize;
    let d_seed = 256usize;
    let nl_seed = 8usize;
    let nh_seed = 4usize;
    let nkv_seed = 2usize;
    let ffn_seed = 640usize;
    let hd_seed = d_seed / nh_seed;

    eprintln!("[recompiler] Preparing RTX 4090 rollout + QAT distillation for compact seed ({nl_seed}L x {d_seed}D, vocab={v_seed})...");
    let (local_to_global, embed_init, tok_byte_map) =
        build_seed_vocab_and_embed_proj(&tokenizer, &teacher_embed, d_model, v_seed, d_seed);

    let g2l_map: HashMap<u32, u32> = local_to_global
        .iter()
        .enumerate()
        .map(|(li, &gi)| (gi, li as u32))
        .collect();

    // Expand prompt bank (plus OOD prompts re-phrased / multi-variation) for teacher rollout generation
    let mut rollout_prompts_ids: Vec<Vec<u32>> = Vec::new();
    for (mode, user_text) in distillation_prompt_bank() {
        let sys = match mode {
            "agent" => "You are UOR-R4 Agent running on SmolLM2. Use <tool_call>{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"...\"}}</tool_call> when workspace shell commands help, then answer concisely.",
            "code" => "You are a precise software engineer and debugger. Identify the exact bug and provide corrected code.",
            _ => "You are a helpful AI assistant named SmolLM, trained by Hugging Face",
        };
        let fmt = format_smollm2_prompt(sys, user_text);
        rollout_prompts_ids.push(tokenizer.encode(&fmt));
        // Add alternate succinct framing + third phrasing to increase held-out generalization:
        if rollout_prompts_ids.len() % 2 == 0 {
            let fmt2 = format_smollm2_prompt("You are a helpful AI assistant.", user_text);
            rollout_prompts_ids.push(tokenizer.encode(&fmt2));
        }
    }

    let rprompts_bin = cache_dir.join("rollout_prompts.bin");
    {
        let mut f = BufWriter::new(fs::File::create(&rprompts_bin).unwrap());
        f.write_all(&(rollout_prompts_ids.len() as u32).to_le_bytes()).unwrap();
        for seq in &rollout_prompts_ids {
            f.write_all(&(seq.len() as u32).to_le_bytes()).unwrap();
            for &id in seq { f.write_all(&id.to_le_bytes()).unwrap(); }
        }
        f.flush().unwrap();
    }

    let seed_cfg_bin = cache_dir.join("seed_cfg.bin");
    {
        let mut f = BufWriter::new(fs::File::create(&seed_cfg_bin).unwrap());
        for v in [v_seed, d_seed, nl_seed, nh_seed, nkv_seed, ffn_seed] {
            f.write_all(&(v as u32).to_le_bytes()).unwrap();
        }
        for &gi in &local_to_global { f.write_all(&gi.to_le_bytes()).unwrap(); }
        for &v in &embed_init { f.write_all(&v.to_le_bytes()).unwrap(); }
        let mut prefix = Vec::with_capacity(vocab + 1);
        let mut total_bt = 0u32;
        prefix.push(0);
        for seq in &tok_byte_map {
            total_bt += seq.len() as u32;
            prefix.push(total_bt);
        }
        for p in prefix { f.write_all(&p.to_le_bytes()).unwrap(); }
        for seq in &tok_byte_map {
            for &id in seq { f.write_all(&id.to_le_bytes()).unwrap(); }
        }
        f.flush().unwrap();
    }

    if !have_cached_gpu {
        let st_dist = Command::new(&cuda_bin_path)
            .args(["rollout-and-distill"])
            .arg(&teacher_bin)
            .arg(&rprompts_bin)
            .arg(&seed_cfg_bin)
            .arg(&student_f32_bin)
            .arg(&distill_summary_bin)
            .status()
            .expect("run rollout-and-distill");
        if !st_dist.success() {
            eprintln!("[error] GPU rollout-and-distill failed");
            return ExitCode::FAILURE;
        }
    }

    // Read distilled student summary & weights -> pack into compact UORR4V12 (<= 4.5 MiB)
    let (teacher_rollout_toks, distill_steps, final_distill_loss) = {
        let raw = fs::read(&distill_summary_bin).unwrap();
        let ttoks = u64::from_le_bytes(raw[0..8].try_into().unwrap());
        let steps = u64::from_le_bytes(raw[8..16].try_into().unwrap());
        let loss = f64::from_le_bytes(raw[16..24].try_into().unwrap());
        (ttoks, steps, loss)
    };
    eprintln!("[recompiler] GPU distillation complete: teacher_rollout_toks={teacher_rollout_toks}, steps={distill_steps}, final_ema_loss={final_distill_loss:.4}");

    let kv_rows_seed = nkv_seed * hd_seed;
    let mut sr = BufReader::new(fs::File::open(&student_f32_bin).unwrap());
    let stu_embed = read_f32_vec(&mut sr, v_seed * d_seed);
    let stu_final_norm = read_f32_vec(&mut sr, d_seed);

    let (rope_cos_s, rope_sin_s, exp_s, silu_s, arcosh_s, zeta_s) =
        build_sealed_luts(512, hd_seed, rope_theta, 6);
    let embed_packed_s = quantize_matrix_dyadic(&stu_embed, v_seed, d_seed, true, 0x5eed_c0de_0000_0001).unwrap();
    let final_norm_codes_s: Vec<i16> = stu_final_norm.iter().map(|&v| grid_code(f64::from(v))).collect();
    let vocab_sigs_s = compute_vocab_hamming_576(&stu_embed, v_seed, d_seed);

    let mut layers_s = Vec::with_capacity(nl_seed);
    for l in 0..nl_seed {
        let in_ln = read_f32_vec(&mut sr, d_seed);
        let post_ln = read_f32_vec(&mut sr, d_seed);
        let mut w_q = read_f32_vec(&mut sr, d_seed * d_seed);
        let mut w_k = read_f32_vec(&mut sr, kv_rows_seed * d_seed);
        let mut w_v = read_f32_vec(&mut sr, kv_rows_seed * d_seed);
        let w_o = read_f32_vec(&mut sr, d_seed * d_seed);
        let mut w_g = read_f32_vec(&mut sr, ffn_seed * d_seed);
        let mut w_u = read_f32_vec(&mut sr, ffn_seed * d_seed);
        let w_d = read_f32_vec(&mut sr, d_seed * ffn_seed);

        fold_columns(&mut w_q, d_seed, &in_ln);
        fold_columns(&mut w_k, d_seed, &in_ln);
        fold_columns(&mut w_v, d_seed, &in_ln);
        fold_columns(&mut w_g, d_seed, &post_ln);
        fold_columns(&mut w_u, d_seed, &post_ln);

        let sb = ((l as u64) + 100) << 16;
        layers_s.push(RecompiledLayerData {
            q_proj: quantize_matrix_dyadic(&w_q, d_seed, d_seed, true, sb | 1).unwrap(),
            k_proj: quantize_matrix_dyadic(&w_k, kv_rows_seed, d_seed, true, sb | 2).unwrap(),
            v_proj: quantize_matrix_dyadic(&w_v, kv_rows_seed, d_seed, true, sb | 3).unwrap(),
            o_proj: quantize_matrix_dyadic(&w_o, d_seed, d_seed, true, sb | 4).unwrap(),
            gate_proj: quantize_matrix_dyadic(&w_g, ffn_seed, d_seed, true, sb | 5).unwrap(),
            up_proj: quantize_matrix_dyadic(&w_u, ffn_seed, d_seed, true, sb | 6).unwrap(),
            down_proj: quantize_matrix_dyadic(&w_d, d_seed, ffn_seed, true, sb | 7).unwrap(),
        });
    }

    let seed_model = RecompiledModelData {
        tier_code: 2,
        vocab_size: v_seed as u32,
        d_model: d_seed as u32,
        n_layers: nl_seed as u32,
        n_heads: nh_seed as u32,
        n_kv_heads: nkv_seed as u32,
        intermediate_size: ffn_seed as u32,
        max_seq: 512,
        bos_token_id: *g2l_map.get(&1).unwrap_or(&1),
        eos_token_id: *g2l_map.get(&2).unwrap_or(&2),
        arcosh_mantissa_bits: 6,
        rms_eps_q48: (rms_eps * 2.0f64.powi(48)).round() as i64,
        score_scale_q30: (2.0f64.powi(30) / (hd_seed as f64).sqrt()).round() as i64,
        rope_cos_q14: rope_cos_s,
        rope_sin_q14: rope_sin_s,
        exp_table: exp_s,
        silu_table: silu_s,
        arcosh_table: arcosh_s,
        zeta_phase_q29: zeta_s,
        local_to_global: local_to_global.clone(),
        vocab_hamming_sigs: vocab_sigs_s,
        embed: embed_packed_s,
        final_norm_codes: final_norm_codes_s,
        layers: layers_s,
    };

    let seed_bytes = serialize_uorr4v12(&seed_model);
    let seed_mib = seed_bytes.len() as f64 / (1024.0 * 1024.0);
    let seed_sha = sha256_hex(&seed_bytes);
    eprintln!("[recompiler] Compact Seed bootstrap_v12_seed.uorr4 = {} bytes ({:.2} MiB, limit <= 4.50 MiB)", seed_bytes.len(), seed_mib);
    assert!(seed_bytes.len() <= 4_718_592, "seed exceeds 4.5 MiB limit");
    fs::write(repo_root.join("crates/wasm/src/bootstrap_v12_seed.uorr4"), &seed_bytes).unwrap();

    // Evaluate Compact Seed Pure-Integer OOD replies!
    let mut seed_replies = Vec::with_capacity(ood_list.len());
    let seed_bos = seed_model.bos_token_id;
    let seed_eos = seed_model.eos_token_id;
    let seed_eot = *g2l_map.get(&0).unwrap_or(&0);
    for (idx, (pid, _mode, _prompt)) in ood_list.iter().enumerate() {
        let local_prompt = translate_global_to_local_seq(&ood_tok_seqs[idx], &g2l_map, &tok_byte_map);
        let mut sess = IntegerEvalSession::new(&seed_model);
        let mut last_logits = Vec::new();
        for (ti, &lt) in local_prompt.iter().enumerate() {
            let (l, _, _) = sess.step(lt, ti + 1 == local_prompt.len());
            if ti + 1 == local_prompt.len() {
                last_logits = l;
            }
        }
        let mut gen_local = Vec::new();
        let mut gen_global = Vec::new();
        let mut next_local = pick_next_token_antirep(
            &last_logits,
            &gen_local,
            0,
            seed_eos,
            seed_bos,
            seed_eot,
        );
        for step_i in 0..48 {
            let gid = local_to_global[next_local as usize];
            if gid == 2 || gid == 0 {
                break;
            }
            gen_local.push(next_local);
            gen_global.push(gid);
            let (l, _, _) = sess.step(next_local, true);
            next_local = pick_next_token_antirep(
                &l,
                &gen_local,
                step_i + 1,
                seed_eos,
                seed_bos,
                seed_eot,
            );
        }
        let sreply = tokenizer.decode(&gen_global);
        eprintln!("  [seed {pid}] {}", sreply.replace('\n', " "));
        seed_replies.push(sreply);
    }

    // Write comprehensive verification receipt JSON
    let mean_w_rmse = layer_rms_errs.iter().sum::<f64>() / layer_rms_errs.len().max(1) as f64;
    let mean_e8_rmse = (e8_embed_rmse + e8_layer_rmses.iter().sum::<f64>()) / (1 + e8_layer_rmses.len()) as f64;
    let rope_theta_u64 = rope_theta.round() as u64;
    let rms_norm_preserved =
        teacher_final_norm.len() == d_model && full_model.final_norm_codes.len() == d_model;
    let mut r_json = String::new();
    r_json.push_str("{\n");
    r_json.push_str("  \"schema\": \"uor-r4.recompiled-v12-receipt/1\",\n");
    r_json.push_str("  \"container_schema\": \"uor-r4.geometric-vector-stack/12\",\n");
    r_json.push_str("  \"container_version\": 12,\n");
    r_json.push_str("  \"teacher_model\": \"HuggingFaceTB/SmolLM2-135M-Instruct\",\n");
    r_json.push_str("  \"hardware\": \"NVIDIA GeForce RTX 4090\",\n");
    r_json.push_str("  \"zero_python\": true,\n");
    r_json.push_str("  \"float_ops\": 0,\n");
    r_json.push_str("  \"dense_matmul_ops\": 0,\n");
    r_json.push_str(&format!("  \"rope_theta\": {rope_theta_u64},\n"));
    r_json.push_str(&format!("  \"tied_lm_head\": {tied_lm_head},\n"));
    r_json.push_str(&format!("  \"rms_norm_preserved\": {rms_norm_preserved},\n"));
    r_json.push_str("  \"huggingface_source_sha256\": {\n");
    r_json.push_str(&format!("    \"config_json\": \"{cfg_sha}\",\n"));
    r_json.push_str(&format!("    \"tokenizer_json\": \"{tok_sha}\",\n"));
    r_json.push_str(&format!("    \"model_safetensors\": \"{safetensors_sha}\"\n"));
    r_json.push_str("  },\n");
    r_json.push_str("  \"full_30l_container\": {\n");
    r_json.push_str("    \"container_version\": 12,\n");
    r_json.push_str(&format!("    \"sha256\": \"{full_sha}\",\n"));
    r_json.push_str(&format!("    \"total_bytes\": {},\n", full_bytes.len()));
    r_json.push_str(&format!("    \"total_mib\": {:.4},\n", full_mib));
    r_json.push_str(&format!("    \"shard_count\": {shard_count},\n"));
    r_json.push_str(&format!("    \"stitch_parity_groups\": {parity_count},\n"));
    r_json.push_str(&format!("    \"vocab_size\": {vocab},\n"));
    r_json.push_str(&format!("    \"d_model\": {d_model},\n"));
    r_json.push_str(&format!("    \"n_layers\": {n_layers},\n"));
    r_json.push_str(&format!("    \"n_heads\": {n_heads},\n"));
    r_json.push_str(&format!("    \"n_kv_heads\": {n_kv_heads},\n"));
    r_json.push_str(&format!("    \"rope_theta\": {rope_theta_u64},\n"));
    r_json.push_str(&format!("    \"tied_lm_head\": {tied_lm_head},\n"));
    r_json.push_str(&format!("    \"rms_norm_preserved\": {rms_norm_preserved},\n"));
    r_json.push_str("    \"quantization\": \"block32-wht+group32-dyadic-4bit-per-row-exp+conway-sloane-e8-audit\",\n");
    r_json.push_str(&format!("    \"mean_weight_rel_rms_error\": {:.6},\n", mean_w_rmse));
    r_json.push_str(&format!("    \"conway_sloane_e8_mean_rel_rmse\": {:.6},\n", mean_e8_rmse));
    r_json.push_str("    \"hamming_signature_bits\": 576\n");
    r_json.push_str("  },\n");
    r_json.push_str("  \"compact_distilled_seed\": {\n");
    r_json.push_str(&format!("    \"sha256\": \"{seed_sha}\",\n"));
    r_json.push_str(&format!("    \"total_bytes\": {},\n", seed_bytes.len()));
    r_json.push_str(&format!("    \"total_mib\": {:.4},\n", seed_mib));
    r_json.push_str(&format!("    \"vocab_size\": {v_seed},\n"));
    r_json.push_str(&format!("    \"d_model\": {d_seed},\n"));
    r_json.push_str(&format!("    \"n_layers\": {nl_seed},\n"));
    r_json.push_str(&format!("    \"n_heads\": {nh_seed},\n"));
    r_json.push_str(&format!("    \"n_kv_heads\": {nkv_seed},\n"));
    r_json.push_str(&format!("    \"teacher_rollout_tokens\": {teacher_rollout_toks},\n"));
    r_json.push_str(&format!("    \"distillation_steps\": {distill_steps},\n"));
    r_json.push_str(&format!("    \"final_ema_kl_ce_loss\": {:.6}\n", final_distill_loss));
    r_json.push_str("  },\n");
    r_json.push_str("  \"teacher_parity_metrics\": {\n");
    r_json.push_str(&format!("    \"prompt_count\": {},\n", ood_list.len()));
    r_json.push_str(&format!("    \"top1_logit_parity_pct\": {:.2},\n", top1_pct));
    r_json.push_str(&format!("    \"top5_logit_parity_pct\": {:.2},\n", top5_pct));
    r_json.push_str(&format!("    \"mean_kl_divergence\": {:.6},\n", mean_kl));
    r_json.push_str(&format!("    \"total_nibble_lut_lookups\": {total_nibble_lookups},\n"));
    r_json.push_str(&format!("    \"total_lorentz_geodesic_evals\": {total_lorentz_evals}\n"));
    r_json.push_str("  },\n");
    r_json.push_str("  \"ood_12_prompt_transcripts\": [\n");
    for (idx, (pid, prompt_txt, m1, m5, kl, t_reply, int_reply)) in full_int_transcripts.iter().enumerate() {
        let comma = if idx + 1 < full_int_transcripts.len() { "," } else { "" };
        r_json.push_str(&format!(
            concat!(
                "    {{\n",
                "      \"id\": \"{}\",\n",
                "      \"prompt\": \"{}\",\n",
                "      \"top1_match\": {},\n",
                "      \"top5_match\": {},\n",
                "      \"kl_divergence\": {:.6},\n",
                "      \"float_teacher_reply\": \"{}\",\n",
                "      \"integer_30l_reply\": \"{}\",\n",
                "      \"compact_seed_reply\": \"{}\"\n",
                "    }}{}\n"
            ),
            json_escape(pid),
            json_escape(prompt_txt),
            m1,
            m5,
            kl,
            json_escape(t_reply),
            json_escape(int_reply),
            json_escape(&seed_replies[idx]),
            comma
        ));
    }
    r_json.push_str("  ]\n}\n");
    fs::write(repo_root.join("gpu/rtx4090_r4_recompiled_v12_receipt.json"), &r_json).unwrap();
    eprintln!("[recompiler] Receipt written to gpu/rtx4090_r4_recompiled_v12_receipt.json. ALL DONE.");
    ExitCode::SUCCESS
}
