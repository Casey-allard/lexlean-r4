//! Native integer execution engine for `uor-r4.lut-stack/7` and `uor-r4.lut-stack/8` (`UORLUT01`).
//!
//! Implements the complete zero-float, zero-matmul, zero-softmax serving pipeline from `uor-r4`:
//! - `ByteBpe4096`: GPT-2 byte-level BPE tokenizer (`d36d3e87`, 4,096 tokens, `<|bos|>=0`, `<|eos|>=1`, `<|unk|>=2`)
//! - `DialogueProtocol`: literal-role dialogue framing (`uor-r4.literal-role-dialogue/1` and `/2`)
//! - `FbsModel`: 6-layer `SpiralCore/FBS` trunk (`Cl(0,6)` 64-state signed-axis route tables `stack_fbs_gather`,
//!   `stack_rms_norm_fine`, `grid_apply` norm gain, 4-tap `GEOMETRIC_KEEP_Q31` + `RETAIN_LAMBDA_Q31`
//!   recurrence, Hamilton quaternion feed-forward gates, and `4096 × 256` Q12 `writer.table`)
//! - `ServedHamming`: depth-2 and depth-6 4-head 64-bit quaternion sign-code Hamming attention (`copy_key: 1..2`)
//! - `ServedTurnMemory` (`/7`) + `ServedPointKey` (`/8` with `vote_k = 8` 256D Q12 L1 plurality vote and
//!   canonical reply-anchor handoff so paraphrased prompts continue predicting the full reply token-by-token)
//! - `dialogue_memory::recall` multi-turn fact sieve + `short_cycle` guard (`period 1..=4` repeated 3×).

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, OnceLock};

pub fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = String::with_capacity(64);
    for word in h {
        out.push_str(&format!("{word:08x}"));
    }
    out
}

pub const MAGIC: &[u8; 8] = b"UORLUT01";
pub const ALIGN: usize = 64;
pub const STACK_SCHEMA_V7: &str = "uor-r4.lut-stack/7";
pub const STACK_SCHEMA_V8: &str = "uor-r4.lut-stack/8";

pub const FBS_WIDTH: usize = 256;
pub const FBS_MLP_HIDDEN: usize = 384;
pub const FBS_LANES: usize = FBS_WIDTH / 4; // 64
pub const FBS_GATE_ROWS: usize = FBS_LANES + FBS_WIDTH; // 320
pub const FBS_CONV_KERNEL: usize = 4;
pub const FBS_Q12_SHIFT: u32 = 12;

pub const HAMMING_HEADS: usize = 4;
pub const HAMMING_HEAD_DIM: usize = 64;
pub const HAMMING_CODE_BITS: u32 = 64;

pub const MEMORY_PAD: u32 = u32::MAX;
pub const MEMORY_REPLY_KEY_LEN: usize = 20;
pub const POINT_KEY_DIM: usize = 256;
pub const DEFAULT_VOTE_K: usize = 8;

/// The 64-state Cl(0,6) signed-axis table (`discrete_d11::spiralcore_signed_axes`).
pub const CL06_AXES: [u8; 512] = [
    0, 1, 2, 3, 4, 5, 6, 7, 12, 10, 1, 14, 0, 7, 3, 13, 15, 11, 6, 1, 5, 12, 10, 0, 2, 12, 8, 13,
    1, 3, 15, 6, 14, 13, 15, 4, 11, 1, 0, 2, 5, 14, 11, 2, 7, 8, 1, 12, 13, 14, 11, 2, 15, 0, 1, 4,
    9, 0, 12, 7, 2, 6, 13, 11, 3, 7, 13, 8, 14, 2, 4, 9, 15, 3, 14, 9, 5, 12, 2, 0, 14, 5, 15, 12,
    3, 9, 0, 2, 10, 12, 0, 13, 1, 3, 7, 14, 4, 10, 1, 14, 8, 15, 3, 5, 15, 3, 6, 9, 13, 4, 10, 0,
    11, 15, 13, 0, 14, 2, 4, 1, 9, 0, 4, 7, 10, 14, 5, 11, 8, 9, 10, 11, 12, 13, 14, 15, 7, 3, 14,
    9, 13, 4, 2, 8, 10, 4, 0, 5, 9, 11, 7, 14, 6, 5, 7, 12, 3, 9, 8, 10, 13, 6, 3, 10, 15, 0, 9, 4,
    11, 7, 5, 8, 14, 2, 12, 1, 9, 8, 12, 15, 10, 14, 13, 11, 0, 9, 10, 11, 4, 5, 14, 7, 5, 14, 3,
    2, 15, 0, 9, 12, 6, 5, 15, 12, 11, 1, 0, 10, 2, 12, 0, 13, 9, 11, 7, 6, 5, 6, 3, 10, 7, 8, 9,
    12, 3, 15, 13, 0, 6, 10, 4, 9, 1, 0, 4, 7, 2, 6, 5, 3, 8, 1, 2, 3, 12, 13, 6, 15, 8, 1, 10, 3,
    4, 5, 14, 15, 9, 8, 4, 15, 2, 6, 5, 11, 3, 15, 5, 0, 14, 2, 12, 9, 1, 8, 4, 15, 10, 14, 5, 3,
    6, 13, 7, 4, 11, 1, 8, 10, 13, 6, 11, 10, 7, 8, 1, 4, 14, 13, 7, 4, 3, 9, 8, 2, 0, 9, 2, 11,
    12, 13, 6, 7, 1, 0, 12, 7, 10, 14, 13, 3, 4, 2, 1, 6, 0, 7, 3, 5, 11, 15, 5, 0, 6, 10, 12, 1,
    2, 4, 8, 5, 9, 11, 15, 6, 7, 11, 14, 1, 5, 12, 2, 8, 10, 4, 8, 5, 1, 3, 15, 14, 11, 7, 13, 8,
    6, 10, 4, 1, 12, 10, 9, 14, 8, 15, 11, 13, 7, 11, 6, 1, 13, 4, 10, 8, 12, 2, 9, 6, 0, 7, 11,
    13, 3, 7, 5, 8, 6, 10, 12, 9, 1, 8, 12, 15, 2, 6, 13, 3, 4, 2, 9, 6, 8, 15, 11, 5, 12, 2, 1, 6,
    8, 15, 3, 13, 10, 12, 8, 13, 9, 11, 15, 14, 6, 13, 15, 4, 3, 9, 0, 10, 4, 10, 9, 14, 0, 7, 11,
    5, 2, 4, 0, 5, 1, 3, 7, 6, 8, 9, 2, 11, 4, 5, 6, 15, 14, 5, 7, 12, 11, 1, 8, 2, 0, 1, 10, 3,
    12, 13, 14, 7, 13, 14, 3, 2, 7, 8, 9, 4, 5, 6, 11, 10, 15, 0, 1, 12, 15, 11, 14, 1, 13, 4, 2,
    0, 7, 3, 6, 9, 5, 12, 10, 8,
];

const GEOMETRIC_KEEP_Q31: [i64; FBS_CONV_KERNEL] =
    [1_073_741_824, 1_518_500_250, 1_859_775_393, 2_147_483_647];

const RETAIN_LAMBDA_Q31: [i64; 16] = [
    2_079_841_894,
    2_015_539_906,
    1_953_125_000,
    1_892_556_544,
    1_833_796_252,
    1_776_807_381,
    1_721_554_662,
    1_667_994_287,
    1_616_083_906,
    1_565_782_609,
    1_517_050_908,
    1_469_850_718,
    1_424_145_338,
    1_379_899_426,
    1_337_078_986,
    1_295_651_344,
];

const EMBEDDED_TOKENIZER_JSON: &[u8] = include_bytes!("tokenizer.json");

#[inline]
pub fn cl06_axes(state: usize) -> [u8; 8] {
    let mut codes = [0u8; 8];
    let base = (state & 63) * 8;
    codes.copy_from_slice(&CL06_AXES[base..base + 8]);
    codes
}

/// Apply a grid code `±(16 + m) 2^(e - 4)` with `e = (abs >> 4) - 64` to an integer value `v`.
#[inline]
pub fn grid_apply_q12(v: i64, code: i16) -> i64 {
    if code == 0 || v == 0 {
        return 0;
    }
    let c = code.unsigned_abs();
    let e = (c >> 4) as i32 - 64;
    let m = (16 + (c & 15)) as i64;
    let shift = e - 4;
    let mag = if shift >= 0 {
        (v * m) << (shift.min(20) as u32)
    } else {
        (v * m) >> ((-shift).min(48) as u32)
    };
    if code < 0 {
        -mag
    } else {
        mag
    }
}

// ---------------------------------------------------------------------------
// Zero-dependency recursive-descent JSON parser for UORLUT01 headers & BPE
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum JsonVal {
    Null,
    Bool(bool),
    Num(f64),
    Int(i64),
    Str(String),
    Arr(Vec<JsonVal>),
    Obj(BTreeMap<String, JsonVal>),
}

impl JsonVal {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?;
        let mut p = JsonParser {
            s: text.as_bytes(),
            pos: 0,
        };
        let val = p.parse_val()?;
        p.skip_ws();
        if p.pos == p.s.len() {
            Some(val)
        } else {
            None
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonVal> {
        match self {
            JsonVal::Obj(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonVal::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonVal::Int(n) => Some(*n),
            JsonVal::Num(f) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_i64().and_then(|v| u64::try_from(v).ok())
    }

    pub fn as_usize(&self) -> Option<usize> {
        self.as_u64().and_then(|v| usize::try_from(v).ok())
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonVal::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[JsonVal]> {
        match self {
            JsonVal::Arr(a) => Some(a.as_slice()),
            _ => None,
        }
    }

    pub fn as_obj(&self) -> Option<&BTreeMap<String, JsonVal>> {
        match self {
            JsonVal::Obj(m) => Some(m),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    s: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.s.len() {
            match self.s[self.pos] {
                b' ' | b'\t' | b'\r' | b'\n' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn parse_val(&mut self) -> Option<JsonVal> {
        self.skip_ws();
        let b = *self.s.get(self.pos)?;
        match b {
            b'"' => self.parse_str().map(JsonVal::Str),
            b'{' => self.parse_obj(),
            b'[' => self.parse_arr(),
            b't' => {
                if self.s.get(self.pos..self.pos + 4)? == b"true" {
                    self.pos += 4;
                    Some(JsonVal::Bool(true))
                } else {
                    None
                }
            }
            b'f' => {
                if self.s.get(self.pos..self.pos + 5)? == b"false" {
                    self.pos += 5;
                    Some(JsonVal::Bool(false))
                } else {
                    None
                }
            }
            b'n' => {
                if self.s.get(self.pos..self.pos + 4)? == b"null" {
                    self.pos += 4;
                    Some(JsonVal::Null)
                } else {
                    None
                }
            }
            b'-' | b'0'..=b'9' => self.parse_num(),
            _ => None,
        }
    }

    fn parse_str(&mut self) -> Option<String> {
        if *self.s.get(self.pos)? != b'"' {
            return None;
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.s.len() {
            let b = self.s[self.pos];
            if b == b'"' {
                self.pos += 1;
                return Some(out);
            }
            if b == b'\\' {
                self.pos += 1;
                let esc = *self.s.get(self.pos)?;
                self.pos += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000C}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let hex = std::str::from_utf8(self.s.get(self.pos..self.pos + 4)?).ok()?;
                        let code = u32::from_str_radix(hex, 16).ok()?;
                        self.pos += 4;
                        if (0xD800..=0xDBFF).contains(&code)
                            && self.s.get(self.pos..self.pos + 2) == Some(b"\\u")
                        {
                            self.pos += 2;
                            let low_hex =
                                std::str::from_utf8(self.s.get(self.pos..self.pos + 4)?).ok()?;
                            let low = u32::from_str_radix(low_hex, 16).ok()?;
                            self.pos += 4;
                            let combined = 0x10000 + (((code - 0xD800) << 10) | (low - 0xDC00));
                            out.push(char::from_u32(combined)?);
                        } else {
                            out.push(char::from_u32(code)?);
                        }
                    }
                    _ => return None,
                }
            } else {
                let rest = std::str::from_utf8(&self.s[self.pos..]).ok()?;
                let ch = rest.chars().next()?;
                out.push(ch);
                self.pos += ch.len_utf8();
            }
        }
        None
    }

    fn parse_num(&mut self) -> Option<JsonVal> {
        let start = self.pos;
        let mut is_float = false;
        if self.s.get(self.pos) == Some(&b'-') {
            self.pos += 1;
        }
        while let Some(&b) = self.s.get(self.pos) {
            match b {
                b'0'..=b'9' => self.pos += 1,
                b'.' | b'e' | b'E' | b'+' | b'-' => {
                    is_float = true;
                    self.pos += 1;
                }
                _ => break,
            }
        }
        let slice = std::str::from_utf8(&self.s[start..self.pos]).ok()?;
        if !is_float {
            if let Ok(i) = slice.parse::<i64>() {
                return Some(JsonVal::Int(i));
            }
        }
        slice.parse::<f64>().ok().map(JsonVal::Num)
    }

    fn parse_arr(&mut self) -> Option<JsonVal> {
        self.pos += 1;
        self.skip_ws();
        let mut arr = Vec::new();
        if self.s.get(self.pos) == Some(&b']') {
            self.pos += 1;
            return Some(JsonVal::Arr(arr));
        }
        loop {
            arr.push(self.parse_val()?);
            self.skip_ws();
            match self.s.get(self.pos)? {
                b',' => self.pos += 1,
                b']' => {
                    self.pos += 1;
                    return Some(JsonVal::Arr(arr));
                }
                _ => return None,
            }
        }
    }

    fn parse_obj(&mut self) -> Option<JsonVal> {
        self.pos += 1;
        self.skip_ws();
        let mut map = BTreeMap::new();
        if self.s.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            return Some(JsonVal::Obj(map));
        }
        loop {
            self.skip_ws();
            let key = self.parse_str()?;
            self.skip_ws();
            if self.s.get(self.pos) != Some(&b':') {
                return None;
            }
            self.pos += 1;
            let val = self.parse_val()?;
            map.insert(key, val);
            self.skip_ws();
            match self.s.get(self.pos)? {
                b',' => self.pos += 1,
                b'}' => {
                    self.pos += 1;
                    return Some(JsonVal::Obj(map));
                }
                _ => return None,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ByteBpe4096 Tokenizer & DialogueProtocol
// ---------------------------------------------------------------------------

fn bytes_to_unicode() -> [char; 256] {
    let mut table = ['\0'; 256];
    let mut assigned = [false; 256];
    for byte in (b'!'..=b'~').chain(0xA1..=0xAC).chain(0xAE..=0xFF) {
        table[byte as usize] = char::from_u32(u32::from(byte)).unwrap_or('\0');
        assigned[byte as usize] = true;
    }
    let mut extra = 0u32;
    for byte in 0usize..256 {
        if !assigned[byte] {
            table[byte] = char::from_u32(256 + extra).unwrap_or('\0');
            extra += 1;
        }
    }
    table
}

pub struct ByteBpe4096 {
    vocab: Vec<String>,
    token_to_id: HashMap<String, u32>,
    merge_ranks: HashMap<String, u32>,
    added_tokens: Vec<(String, u32)>,
    added_ids: HashSet<u32>,
    byte_encoder: [char; 256],
    byte_decoder: HashMap<char, u8>,
    add_prefix_space: bool,
    digits_individual: Option<bool>,
}

impl ByteBpe4096 {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<ByteBpe4096> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            Self::from_json_bytes(EMBEDDED_TOKENIZER_JSON)
                .expect("embedded 4096 BPE tokenizer.json is valid")
        })
    }

    pub fn from_json_bytes(bytes: &[u8]) -> Option<Self> {
        let root = JsonVal::parse(bytes)?;
        let model = root.get("model")?;
        let vocab_obj = model.get("vocab")?.as_obj()?;
        let mut token_to_id = HashMap::with_capacity(vocab_obj.len());
        let mut max_id = 0u32;
        for (piece, id_val) in vocab_obj {
            let id = u32::try_from(id_val.as_u64()?).ok()?;
            max_id = max_id.max(id);
            token_to_id.insert(piece.clone(), id);
        }
        let mut added_tokens = Vec::new();
        let mut added_ids = HashSet::new();
        if let Some(arr) = root.get("added_tokens").and_then(JsonVal::as_arr) {
            for entry in arr {
                let content = entry.get("content")?.as_str()?.to_owned();
                let id = u32::try_from(entry.get("id")?.as_u64()?).ok()?;
                max_id = max_id.max(id);
                token_to_id.entry(content.clone()).or_insert(id);
                added_ids.insert(id);
                added_tokens.push((content, id));
            }
        }
        added_tokens.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));

        let mut vocab = vec![String::new(); (max_id as usize) + 1];
        for (piece, &id) in &token_to_id {
            vocab[id as usize] = piece.clone();
        }
        for (content, id) in &added_tokens {
            vocab[*id as usize] = content.clone();
        }

        let merges_arr = model.get("merges")?.as_arr()?;
        let mut merge_ranks = HashMap::with_capacity(merges_arr.len());
        for (rank, entry) in merges_arr.iter().enumerate() {
            if let Some(s) = entry.as_str() {
                merge_ranks.entry(s.to_owned()).or_insert(rank as u32);
            } else if let Some(pair) = entry.as_arr() {
                if pair.len() == 2 {
                    let l = pair[0].as_str()?;
                    let r = pair[1].as_str()?;
                    merge_ranks.entry(format!("{l} {r}")).or_insert(rank as u32);
                }
            }
        }

        let byte_encoder = bytes_to_unicode();
        let mut byte_decoder = HashMap::with_capacity(256);
        for (b, &ch) in byte_encoder.iter().enumerate() {
            byte_decoder.insert(ch, b as u8);
        }

        Some(Self {
            vocab,
            token_to_id,
            merge_ranks,
            added_tokens,
            added_ids,
            byte_encoder,
            byte_decoder,
            add_prefix_space: false,
            digits_individual: None,
        })
    }

    pub fn vocab_size(&self) -> usize {
        self.vocab.len()
    }

    pub fn bos_id(&self) -> u32 {
        0
    }

    pub fn eos_id(&self) -> u32 {
        1
    }

    pub fn unk_id(&self) -> u32 {
        2
    }

    pub fn encode(&self, text: &str) -> Vec<u32> {
        let mut ids = Vec::new();
        let mut rest = text;
        while !rest.is_empty() {
            let mut best: Option<(usize, usize, u32)> = None;
            for (content, id) in &self.added_tokens {
                if content.is_empty() {
                    continue;
                }
                if let Some(pos) = rest.find(content.as_str()) {
                    if best.is_none_or(|(ex, _, _)| pos < ex) {
                        best = Some((pos, content.len(), *id));
                    }
                    if pos == 0 {
                        break;
                    }
                }
            }
            match best {
                Some((pos, len, id)) => {
                    if pos > 0 {
                        self.encode_segment(&rest[..pos], &mut ids);
                    }
                    ids.push(id);
                    rest = &rest[pos + len..];
                }
                None => {
                    self.encode_segment(rest, &mut ids);
                    break;
                }
            }
        }
        ids
    }

    fn encode_segment(&self, segment: &str, out: &mut Vec<u32>) {
        let splits = match self.digits_individual {
            Some(ind) => split_digits(segment, ind),
            None => vec![segment],
        };
        for split in splits {
            if split.is_empty() {
                continue;
            }
            let prefixed;
            let s = if self.add_prefix_space && !split.starts_with(' ') {
                prefixed = format!(" {split}");
                prefixed.as_str()
            } else {
                split
            };
            for pre in pre_tokenize(s) {
                self.bpe(pre, out);
            }
        }
    }

    fn bpe(&self, pre_token: &str, out: &mut Vec<u32>) {
        if pre_token.is_empty() {
            return;
        }
        let mut symbols: Vec<String> = pre_token
            .bytes()
            .map(|b| self.byte_encoder[b as usize].to_string())
            .collect();
        let mut pair_key = String::new();
        while symbols.len() > 1 {
            let mut best: Option<(u32, usize)> = None;
            for idx in 0..symbols.len() - 1 {
                pair_key.clear();
                pair_key.push_str(&symbols[idx]);
                pair_key.push(' ');
                pair_key.push_str(&symbols[idx + 1]);
                if let Some(&rank) = self.merge_ranks.get(&pair_key) {
                    if best.is_none_or(|(ex, _)| rank < ex) {
                        best = Some((rank, idx));
                    }
                }
            }
            let Some((_, best_idx)) = best else {
                break;
            };
            let left = symbols[best_idx].clone();
            let right = symbols[best_idx + 1].clone();
            let merged = format!("{left}{right}");
            let mut next = Vec::with_capacity(symbols.len());
            let mut i = 0;
            while i < symbols.len() {
                if i + 1 < symbols.len() && symbols[i] == left && symbols[i + 1] == right {
                    next.push(merged.clone());
                    i += 2;
                } else {
                    next.push(std::mem::take(&mut symbols[i]));
                    i += 1;
                }
            }
            symbols = next;
        }
        for sym in &symbols {
            if let Some(&id) = self.token_to_id.get(sym.as_str()) {
                out.push(id);
            } else {
                let mut utf8 = [0u8; 4];
                for ch in sym.chars() {
                    if let Some(&id) = self.token_to_id.get(ch.encode_utf8(&mut utf8)) {
                        out.push(id);
                    }
                }
            }
        }
    }

    pub fn decode_bytes(&self, ids: &[u32]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for &id in ids {
            let Some(tok) = self.vocab.get(id as usize) else {
                continue;
            };
            if self.added_ids.contains(&id) {
                bytes.extend_from_slice(tok.as_bytes());
                continue;
            }
            for ch in tok.chars() {
                if let Some(&b) = self.byte_decoder.get(&ch) {
                    bytes.push(b);
                } else {
                    let mut utf8 = [0u8; 4];
                    bytes.extend_from_slice(ch.encode_utf8(&mut utf8).as_bytes());
                }
            }
        }
        bytes
    }

    pub fn decode(&self, ids: &[u32]) -> String {
        String::from_utf8_lossy(&self.decode_bytes(ids)).into_owned()
    }

    /// Encode a single user prompt under `uor-r4.literal-role-dialogue` v1 or v2.
    pub fn encode_dialogue_prompt(
        &self,
        system_note: Option<&str>,
        history: &[(String, String)],
        user_prompt: &str,
        dialogue_version: u8,
    ) -> Vec<u32> {
        let spaced = dialogue_version == 2;
        let mut tokens = vec![self.bos_id()];
        let mut emitted = 0usize;

        if let Some(sys) = system_note {
            let clean = sys.replace("\r\n", "\n").replace('\r', "\n");
            let clean = clean.trim();
            if !clean.is_empty() {
                if spaced {
                    tokens.extend(self.encode("System:"));
                    tokens.extend(self.encode(&format!(" {clean}")));
                } else {
                    tokens.extend(self.encode("System: "));
                    tokens.extend(self.encode(clean));
                }
                emitted += 1;
            }
        }

        for (u, a) in history {
            let u_clean = u.replace("\r\n", "\n").replace('\r', "\n");
            let u_clean = u_clean.trim();
            if !u_clean.is_empty() {
                if emitted > 0 {
                    tokens.extend(self.encode("\n"));
                }
                if spaced {
                    tokens.extend(self.encode("User:"));
                    tokens.extend(self.encode(&format!(" {u_clean}")));
                } else {
                    tokens.extend(self.encode("User: "));
                    tokens.extend(self.encode(u_clean));
                }
                emitted += 1;
            }
            let a_clean = a.replace("\r\n", "\n").replace('\r', "\n");
            let a_clean = a_clean.trim();
            if !a_clean.is_empty() {
                if emitted > 0 {
                    tokens.extend(self.encode("\n"));
                }
                if spaced {
                    tokens.extend(self.encode("Assistant:"));
                    tokens.extend(self.encode(&format!(" {a_clean}")));
                } else {
                    tokens.extend(self.encode("Assistant: "));
                    tokens.extend(self.encode(a_clean));
                }
                tokens.push(self.eos_id());
                emitted += 1;
            }
        }

        let p_clean = user_prompt.replace("\r\n", "\n").replace('\r', "\n");
        let p_clean = p_clean.trim();
        if !p_clean.is_empty() {
            if emitted > 0 {
                tokens.extend(self.encode("\n"));
            }
            if spaced {
                tokens.extend(self.encode("User:"));
                tokens.extend(self.encode(&format!(" {p_clean}")));
            } else {
                tokens.extend(self.encode("User: "));
                tokens.extend(self.encode(p_clean));
            }
            emitted += 1;
        }
        if emitted > 0 {
            tokens.extend(self.encode("\n"));
        }
        if spaced {
            tokens.extend(self.encode("Assistant:"));
        } else {
            tokens.extend(self.encode("Assistant: "));
        }
        tokens
    }
}

fn split_digits(text: &str, individual: bool) -> Vec<&str> {
    let mut splits = Vec::new();
    let mut start = 0usize;
    let mut in_digits = false;
    for (offset, ch) in text.char_indices() {
        let numeric = ch.is_numeric();
        if offset > 0 && (numeric != in_digits || (numeric && individual)) {
            splits.push(&text[start..offset]);
            start = offset;
        }
        in_digits = numeric;
    }
    if start < text.len() {
        splits.push(&text[start..]);
    }
    splits
}

fn pre_tokenize(text: &str) -> Vec<&str> {
    const CONTRACTIONS: [&str; 7] = ["'s", "'t", "'re", "'ve", "'m", "'ll", "'d"];
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let byte_at = |idx: usize| chars.get(idx).map_or(text.len(), |(off, _)| *off);
    let mut pieces = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let (offset, ch) = chars[i];
        if ch == '\'' {
            if let Some(c) = CONTRACTIONS
                .iter()
                .find(|cand| text[offset..].starts_with(**cand))
            {
                pieces.push(&text[offset..offset + c.len()]);
                i += c.chars().count();
                continue;
            }
        }
        let after_space = if ch == ' ' { i + 1 } else { i };
        if let Some(&(_, next)) = chars.get(after_space) {
            if next.is_alphabetic() {
                let mut end = after_space;
                while end < chars.len() && chars[end].1.is_alphabetic() {
                    end += 1;
                }
                pieces.push(&text[offset..byte_at(end)]);
                i = end;
                continue;
            }
            if next.is_numeric() {
                let mut end = after_space;
                while end < chars.len() && chars[end].1.is_numeric() {
                    end += 1;
                }
                pieces.push(&text[offset..byte_at(end)]);
                i = end;
                continue;
            }
            if !next.is_whitespace() {
                let mut end = after_space;
                while end < chars.len()
                    && !chars[end].1.is_whitespace()
                    && !chars[end].1.is_alphabetic()
                    && !chars[end].1.is_numeric()
                {
                    end += 1;
                }
                pieces.push(&text[offset..byte_at(end)]);
                i = end;
                continue;
            }
        }
        let mut end = i;
        while end < chars.len() && chars[end].1.is_whitespace() {
            end += 1;
        }
        if end < chars.len() && end - i > 1 {
            end -= 1;
        }
        pieces.push(&text[offset..byte_at(end)]);
        i = end;
    }
    pieces
}

// ---------------------------------------------------------------------------
// Exact Integer Kernels (SpiralCore/FBS, Hamming, TurnMemory, PointKey)
// ---------------------------------------------------------------------------

pub const fn build_rsqrt_lut_fine() -> [u16; 1024] {
    let mut table = [0u16; 1024];
    let mut i = 0usize;
    while i < 1024 {
        let target: u128 = (1u128 << 46) / ((i as u128) + 1);
        let mut lo: u128 = 0;
        let mut hi: u128 = 1 << 24;
        while lo < hi {
            let mid = (lo + hi + 1) >> 1;
            if mid * mid <= target {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        table[i] = if lo > (u16::MAX as u128) {
            u16::MAX
        } else {
            lo as u16
        };
        i += 1;
    }
    table
}

pub const RSQRT_LUT_FINE: [u16; 1024] = build_rsqrt_lut_fine();

pub const fn build_exp_neg_lut() -> [u16; 256] {
    const LN2_Q30: i64 = 744_261_118;
    let mut table = [0u16; 256];
    let mut i = 0usize;
    while i < 256 {
        let x_q30 = (i as i64) << 25;
        let k = x_q30 / LN2_Q30;
        let r = x_q30 - k * LN2_Q30;
        let r2 = (r * r) >> 30;
        let r3 = (r2 * r) >> 30;
        let r4 = (r3 * r) >> 30;
        let r5 = (r4 * r) >> 30;
        let exp_neg_r_q30 = (1i64 << 30) - r + (r2 >> 1) - (r3 / 6) + (r4 / 24) - (r5 / 120);
        let val_q15 = exp_neg_r_q30 >> (15 + k);
        table[i] = if val_q15 < 0 {
            0
        } else if val_q15 > (1 << 15) {
            1 << 15
        } else {
            val_q15 as u16
        };
        i += 1;
    }
    table
}

pub const EXP_NEG_LUT: [u16; 256] = build_exp_neg_lut();

pub const fn build_sigmoid_lut() -> [u16; 256] {
    let exp_lut = build_exp_neg_lut();
    let mut table = [0u16; 256];
    let mut i = 0usize;
    while i < 256 {
        let u = (i as i32) - 128;
        let abs_u = if u < 0 { -u } else { u } as usize;
        let exp_idx = if abs_u * 2 > 255 { 255 } else { abs_u * 2 };
        let e = exp_lut[exp_idx] as i64;
        let denom = (1i64 << 15) + e;
        let sig_q15 = if u >= 0 {
            ((1i64 << 30) + (denom >> 1)) / denom
        } else {
            ((e << 15) + (denom >> 1)) / denom
        };
        table[i] = if sig_q15 > (u16::MAX as i64) {
            u16::MAX
        } else {
            sig_q15 as u16
        };
        i += 1;
    }
    table
}

pub const SIGMOID_LUT: [u16; 256] = build_sigmoid_lut();

#[inline]
pub fn stack_sigmoid_q15(x_q12: i32) -> i64 {
    let idx = ((x_q12 >> 9) + 128).clamp(0, 255) as usize;
    SIGMOID_LUT[idx] as i64
}

pub fn stack_rms_norm_fine(x: &[i32]) -> Vec<i32> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let sum_sq: u128 = x
        .iter()
        .map(|&v| {
            let a = (v as i64).unsigned_abs() as u128;
            a * a
        })
        .sum();
    let mean_sq = (sum_sq / (n as u128)) as u64;
    if mean_sq == 0 {
        return vec![0; n];
    }
    let log2 = 63u32.saturating_sub(mean_sq.leading_zeros());
    let k = log2 / 2;
    let base = 1u64 << (2 * k);
    let frac = (((mean_sq - base) << 8) / base).min(1023) as usize;
    let rsqrt_m = RSQRT_LUT_FINE[frac] as i64;
    let total_shift = 5 + k;
    x.iter()
        .map(|&v| (((v as i64) * rsqrt_m) >> total_shift) as i32)
        .collect()
}

pub fn stack_norm_gain(x: &[i32], gain_codes: &[i16]) -> Vec<i32> {
    let normed = stack_rms_norm_fine(x);
    normed
        .iter()
        .zip(gain_codes.iter())
        .map(|(&v, &code)| grid_apply_q12(v as i64, code).clamp(-32768, 32767) as i32)
        .collect()
}

/// Packed `Cl(0,6)` route map (`l{l}.*.fbs` in `UORLUT01`).
#[derive(Clone, Debug)]
pub struct FbsRouteMap {
    pub in_dim: usize,
    pub out_dim: usize,
    pub packed_entries: Vec<u32>,
    pub atoms: Vec<u16>,
    pub codes: Vec<[u8; 8]>,
}

impl FbsRouteMap {
    pub fn from_packed_u32(entries: &[u32], in_dim: usize, out_dim: usize) -> Self {
        let in_blocks = (in_dim / 8).max(1);
        let out_blocks = out_dim / 8;
        let mut atoms = Vec::with_capacity(out_blocks);
        let mut codes = Vec::with_capacity(out_blocks);
        for b in 0..out_blocks {
            let entry = entries.get(b).copied().unwrap_or(0);
            let atom = ((entry & 0xFFFF) as usize) % in_blocks;
            let state = ((entry >> 16) as usize) & 63;
            atoms.push(atom as u16);
            codes.push(cl06_axes(state));
        }
        Self {
            in_dim,
            out_dim,
            packed_entries: entries.to_vec(),
            atoms,
            codes,
        }
    }

    pub fn forward(&self, x: &[i32]) -> Vec<i32> {
        let mut out = vec![0i32; self.out_dim];
        for ((block, &atom), codes) in out
            .chunks_exact_mut(8)
            .zip(self.atoms.iter())
            .zip(self.codes.iter())
        {
            let base = (atom as usize) << 3;
            if let Some(vals) = x.get(base..base + 8) {
                for (slot, &code) in block.iter_mut().zip(codes.iter()) {
                    let v = vals[(code & 7) as usize];
                    *slot = if (code & 8) == 0 {
                        v
                    } else {
                        v.saturating_neg()
                    };
                }
            }
        }
        out
    }
}

/// Hamilton quaternion feed-forward gate (`gate = "hamilton"`).
pub fn stack_hamilton_ffn(gate: &[i32], up: &[i32]) -> Vec<i32> {
    let len = gate.len().min(up.len());
    let mut out = vec![0i32; len];
    for ((o_lane, g_lane), u_lane) in out
        .chunks_exact_mut(4)
        .zip(gate.chunks_exact(4))
        .zip(up.chunks_exact(4))
    {
        let (g0, g1, g2, g3) = (
            g_lane[0] as i64,
            g_lane[1] as i64,
            g_lane[2] as i64,
            g_lane[3] as i64,
        );
        let norm_sq = (g0 * g0 + g1 * g1 + g2 * g2 + g3 * g3).max(1);
        let norm = (norm_sq as u128).isqrt().max(1) as i64;
        // Unit quaternion in Q12
        let q = [
            (g0 << 12) / norm,
            (g1 << 12) / norm,
            (g2 << 12) / norm,
            (g3 << 12) / norm,
        ];
        let (u0, u1, u2, u3) = (
            u_lane[0] as i64,
            u_lane[1] as i64,
            u_lane[2] as i64,
            u_lane[3] as i64,
        );
        o_lane[0] = ((q[0] * u0 - q[1] * u1 - q[2] * u2 - q[3] * u3) >> 12) as i32;
        o_lane[1] = ((q[0] * u1 + q[1] * u0 + q[2] * u3 - q[3] * u2) >> 12) as i32;
        o_lane[2] = ((q[0] * u2 - q[1] * u3 + q[2] * u0 + q[3] * u1) >> 12) as i32;
        o_lane[3] = ((q[0] * u3 + q[1] * u2 - q[2] * u1 + q[3] * u0) >> 12) as i32;
    }
    out
}

#[derive(Clone, Debug)]
pub struct FbsLayer {
    pub rec_gain: Vec<i16>,
    pub mlp_gain: Vec<i16>,
    pub rec_in: FbsRouteMap,
    pub rec_gate: FbsRouteMap,
    pub rec_out: FbsRouteMap,
    pub mlp_gate: FbsRouteMap,
    pub mlp_up: FbsRouteMap,
    pub mlp_down: FbsRouteMap,
    pub taps: Vec<i16>,
    pub conv_bias: Vec<i32>,
    pub gate_bias: Vec<i32>,
    pub decay_offset: Vec<i32>,
}

/// 17-root Icosian golden-ratio quaternion lattice `Z[phi]` on the unit 3-sphere `S^3` in Q12 (`4096 = 1.0`).
/// Preserves spin routes across SpiralCore non-commutative Hamilton transport (`R4.SpiralCoreFbs`).
pub const ICOSIAN_17_Q12: [[i16; 4]; 17] = [
    [4096, 0, 0, 0],
    [0, 4096, 0, 0],
    [0, 0, 4096, 0],
    [0, 0, 0, 4096],
    [-4096, 0, 0, 0],
    [2048, 2048, 2048, 2048],
    [2048, 2048, -2048, -2048],
    [2048, -2048, 2048, -2048],
    [2048, -2048, -2048, 2048],
    [3314, 2048, 1266, 0],
    [3314, -2048, 1266, 0],
    [3314, 2048, -1266, 0],
    [2048, 1266, 3314, 0],
    [2048, -1266, 3314, 0],
    [1266, 3314, 2048, 0],
    [1266, 0, 3314, 2048],
    [3314, 0, 1266, 2048],
];

/// Snap a 4D rotation vector to the nearest root in `ICOSIAN_17_Q12` on `S^3`.
#[inline]
pub fn snap_icosian_17_q12(rot: [i32; 4]) -> (u8, [i16; 4]) {
    let (r0, r1, r2, r3) = (rot[0] as i64, rot[1] as i64, rot[2] as i64, rot[3] as i64);
    if r0 == 0 && r1 == 0 && r2 == 0 && r3 == 0 {
        return (0, ICOSIAN_17_Q12[0]);
    }
    let mut best_idx = 0usize;
    let mut best_dot = i64::MIN;
    for (idx, q) in ICOSIAN_17_Q12.iter().enumerate() {
        let dot = r0 * (q[0] as i64) + r1 * (q[1] as i64) + r2 * (q[2] as i64) + r3 * (q[3] as i64);
        if dot > best_dot {
            best_dot = dot;
            best_idx = idx;
        }
    }
    (best_idx as u8, ICOSIAN_17_Q12[best_idx])
}

/// Non-commutative quaternion Hamilton product `q * h` on `S^3` in Q12.
#[inline]
pub fn hamilton_q12(q: [i16; 4], h: [i32; 4]) -> [i32; 4] {
    let (q0, q1, q2, q3) = (q[0] as i64, q[1] as i64, q[2] as i64, q[3] as i64);
    let (h0, h1, h2, h3) = (h[0] as i64, h[1] as i64, h[2] as i64, h[3] as i64);
    [
        ((q0 * h0 - q1 * h1 - q2 * h2 - q3 * h3) >> 12).clamp(-32768, 32767) as i32,
        ((q0 * h1 + q1 * h0 + q2 * h3 - q3 * h2) >> 12).clamp(-32768, 32767) as i32,
        ((q0 * h2 - q1 * h3 + q2 * h0 + q3 * h1) >> 12).clamp(-32768, 32767) as i32,
        ((q0 * h3 + q1 * h2 - q2 * h1 + q3 * h0) >> 12).clamp(-32768, 32767) as i32,
    ]
}

/// Compose two 6-bit Clifford `Cl(0,6)` spin carrier actions (`0..63`), tracking anti-commuting grade parity.
#[inline]
pub fn cl06_compose(a: u8, b: u8) -> u8 {
    let a6 = a & 0x3F;
    let b6 = b & 0x3F;
    let mut swaps = 0u32;
    for i in 1..6 {
        if ((a6 >> i) & 1) != 0 {
            let lower_mask = (1u8 << i) - 1;
            swaps += (b6 & lower_mask).count_ones();
        }
    }
    let sign_bit = (swaps & 1) as u8;
    (a6 ^ b6 ^ (sign_bit << 5)) & 0x3F
}

#[derive(Clone, Debug)]
pub struct FbsRecState {
    pub conv_ring: [[i32; FBS_WIDTH]; FBS_CONV_KERNEL],
    pub conv_pos: usize,
    pub h_state: [i32; FBS_WIDTH],
    /// Per-lane cumulative `Cl(0,6)` spin holonomy (`0..63`) preserving routes with spin across SpiralCore.
    pub spin_holonomy: [u8; FBS_LANES],
    /// Per-lane ternary/trinary FBS routing gate (`{-1, 0, +1}`).
    pub trinary_route: [i8; FBS_LANES],
    /// Rolling 4-step dominant Icosian root trajectory (`0..16`).
    pub icosian_trajectory: [u8; 4],
}

impl Default for FbsRecState {
    fn default() -> Self {
        Self {
            conv_ring: [[0; FBS_WIDTH]; FBS_CONV_KERNEL],
            conv_pos: 0,
            h_state: [0; FBS_WIDTH],
            spin_holonomy: [0; FBS_LANES],
            trinary_route: [0; FBS_LANES],
            icosian_trajectory: [0; 4],
        }
    }
}

impl FbsRecState {
    /// Pack the rolling 4-step Icosian root trajectory into a Base-17 address (`0..83520`).
    #[inline]
    pub fn packed_4block_root(&self) -> u32 {
        (self.icosian_trajectory[0].min(16) as u32)
            + 17 * (self.icosian_trajectory[1].min(16) as u32)
            + 289 * (self.icosian_trajectory[2].min(16) as u32)
            + 4913 * (self.icosian_trajectory[3].min(16) as u32)
    }
}

pub fn stack_recurrence_core(
    branches: &[i32],
    gate_out: &[i32],
    taps: &[i16],
    conv_bias: &[i32],
    gate_bias: &[i32],
    decay_offset: &[i32],
    state: &mut FbsRecState,
) -> [i32; FBS_WIDTH] {
    let slot = state.conv_pos % FBS_CONV_KERNEL;
    state.conv_ring[slot].copy_from_slice(&branches[..FBS_WIDTH]);
    state.conv_pos = state.conv_pos.wrapping_add(1);

    let mut conv_out = [0i32; FBS_WIDTH];
    for (c, conv_cell) in conv_out.iter_mut().enumerate().take(FBS_WIDTH) {
        let mut acc: i64 = (conv_bias.get(c).copied().unwrap_or(0) as i64) >> 4;
        for tap in 0..FBS_CONV_KERNEL {
            let ring_slot = (state.conv_pos + FBS_CONV_KERNEL - 1 - tap) % FBS_CONV_KERNEL;
            let tap_code = taps.get(tap * FBS_WIDTH + c).copied().unwrap_or(0);
            let w_q12 = grid_apply_q12(4096, tap_code);
            let w_geom = (w_q12 * GEOMETRIC_KEEP_Q31[FBS_CONV_KERNEL - 1 - tap]) >> 31;
            acc += ((state.conv_ring[ring_slot][c] as i64) * w_geom) >> 12;
        }
        *conv_cell = acc.clamp(-32768, 32767) as i32;
    }

    let mut h_next = [0i32; FBS_WIDTH];
    let mut root_votes = [0u16; 17];

    for lane in 0..FBS_LANES {
        let g_raw = gate_out.get(lane).copied().unwrap_or(0) as i64
            + ((gate_bias.get(lane).copied().unwrap_or(0) as i64) >> 4)
            + ((decay_offset.get(lane).copied().unwrap_or(0) as i64) >> 4);
        let rate_bucket = ((stack_sigmoid_q15(g_raw as i32) * 15) >> 15).clamp(0, 15) as usize;
        let lambda_q15 = (RETAIN_LAMBDA_Q31[rate_bucket] >> 16).clamp(1, 32767);
        let one_minus_q15 = (1i64 << 15) - lambda_q15;

        let base = lane * 4;
        let rot_base = FBS_LANES + base;
        let rot_raw = [
            gate_out.get(rot_base).copied().unwrap_or(4096),
            gate_out.get(rot_base + 1).copied().unwrap_or(0),
            gate_out.get(rot_base + 2).copied().unwrap_or(0),
            gate_out.get(rot_base + 3).copied().unwrap_or(0),
        ];
        let (icos_idx, q_icos) = snap_icosian_17_q12(rot_raw);
        root_votes[icos_idx as usize] = root_votes[icos_idx as usize].saturating_add(1);
        state.spin_holonomy[lane] = cl06_compose(
            state.spin_holonomy[lane],
            (icos_idx & 0x3F) ^ ((lane as u8) & 0x3F),
        );

        let h_prev_lane = [
            state.h_state[base],
            state.h_state[base + 1],
            state.h_state[base + 2],
            state.h_state[base + 3],
        ];
        // Blend continuous spin-preserving Hamilton rotation (25%) with exact scalar carrier (75%)
        // so routes carry non-commutative spin phase on S^3 without drifting from trained LUT basins.
        let h_rot_lane = hamilton_q12(q_icos, h_prev_lane);

        let out_gate_lead = branches.get(FBS_WIDTH + base).copied().unwrap_or(0);
        let trit: i8 = if out_gate_lead > 512 {
            1
        } else if out_gate_lead < -512 {
            -1
        } else {
            0
        };
        state.trinary_route[lane] = trit;

        for c in 0..4 {
            let idx = base + c;
            let h_carrier = ((h_prev_lane[c] as i64) * 3 + (h_rot_lane[c] as i64)) >> 2;
            let u_cur = conv_out[idx] as i64;
            let h_new = ((lambda_q15 * h_carrier) + (one_minus_q15 * u_cur)) >> 15;
            state.h_state[idx] = h_new.clamp(-32768, 32767) as i32;
            let out_gate = branches.get(FBS_WIDTH + idx).copied().unwrap_or(4096);
            let g_sig = stack_sigmoid_q15(out_gate);
            h_next[idx] = ((h_new * g_sig) >> 15).clamp(-32768, 32767) as i32;
        }
    }

    let mut dom_root = 0u8;
    let mut dom_cnt = 0u16;
    for (r_idx, &cnt) in root_votes.iter().enumerate() {
        if cnt > dom_cnt {
            dom_cnt = cnt;
            dom_root = r_idx as u8;
        }
    }
    state.icosian_trajectory = [
        state.icosian_trajectory[1],
        state.icosian_trajectory[2],
        state.icosian_trajectory[3],
        dom_root,
    ];

    h_next
}

// ---------------------------------------------------------------------------
// ServedHamming (Depth-2 & Depth-6 Bitwise Popcount Hamming XOR Attention)
// ---------------------------------------------------------------------------

/// Explicit SWAR 64-bit bitwise population count (`R4.Flock.swarPopcount`).
#[inline]
pub fn popcount_swar_u64(mut x: u64) -> u32 {
    x = x.wrapping_sub((x >> 1) & 0x5555_5555_5555_5555);
    x = (x & 0x3333_3333_3333_3333).wrapping_add((x >> 2) & 0x3333_3333_3333_3333);
    x = (x.wrapping_add(x >> 4)) & 0x0F0F_0F0F_0F0F_0F0F;
    (x.wrapping_mul(0x0101_0101_0101_0101) >> 56) as u32
}

/// 256-bit Bitwise XOR + SWAR Popcount Hamming distance across 4 64-bit sign-code heads (`0..256` bits).
#[inline]
pub fn hamming_distance_256(a: &[u64; HAMMING_HEADS], b: &[u64; HAMMING_HEADS]) -> u32 {
    popcount_swar_u64(a[0] ^ b[0])
        + popcount_swar_u64(a[1] ^ b[1])
        + popcount_swar_u64(a[2] ^ b[2])
        + popcount_swar_u64(a[3] ^ b[3])
}

/// Pack a 256D `i32` hidden vector into a 256-bit (`[u64; 4]`) binary sign sketch.
#[inline]
pub fn pack_i32_sign_sketch_256(slice: &[i32]) -> [u64; HAMMING_HEADS] {
    let mut out = [0u64; HAMMING_HEADS];
    for (h, out_word) in out.iter_mut().enumerate() {
        let base = h * HAMMING_HEAD_DIM;
        let end = (base + HAMMING_HEAD_DIM).min(slice.len());
        if base < end {
            *out_word = sign_pack_64(&slice[base..end]);
        }
    }
    out
}

/// Pack a 256D `i16` point/writer vector into a 256-bit (`[u64; 4]`) binary sign sketch.
#[inline]
pub fn pack_i16_sign_sketch_256(slice: &[i16]) -> [u64; HAMMING_HEADS] {
    let mut out = [0u64; HAMMING_HEADS];
    for (h, out_word) in out.iter_mut().enumerate() {
        let base = h * HAMMING_HEAD_DIM;
        let mut word = 0u64;
        for b in 0..HAMMING_HEAD_DIM {
            if slice.get(base + b).copied().unwrap_or(0) >= 0 {
                word |= 1u64 << b;
            }
        }
        *out_word = word;
    }
    out
}

#[derive(Clone, Debug)]
pub struct ServedHammingRead {
    pub depth: usize,
    pub copy_key: usize,
    pub query_rotation: Vec<i32>,
    pub key_rotation: Vec<i32>,
    pub value_rotation: Vec<i32>,
    pub margin: Vec<i32>,
    pub weight: Vec<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct HammingCache {
    pub codes: Vec<[u64; HAMMING_HEADS]>,
    pub values: Vec<[i32; FBS_WIDTH]>,
}

fn rotate_lanes_q12(x: &[i32], rot_q24: &[i32]) -> [i32; FBS_WIDTH] {
    let mut out = [0i32; FBS_WIDTH];
    for lane in 0..FBS_LANES {
        let b = lane * 4;
        let r0 = (rot_q24.get(b).copied().unwrap_or(1 << 24) >> 12) as i64;
        let r1 = (rot_q24.get(b + 1).copied().unwrap_or(0) >> 12) as i64;
        let r2 = (rot_q24.get(b + 2).copied().unwrap_or(0) >> 12) as i64;
        let r3 = (rot_q24.get(b + 3).copied().unwrap_or(0) >> 12) as i64;
        let norm = ((r0 * r0 + r1 * r1 + r2 * r2 + r3 * r3).max(1) as u128)
            .isqrt()
            .max(1) as i64;
        let q = [
            (r0 << 12) / norm,
            (r1 << 12) / norm,
            (r2 << 12) / norm,
            (r3 << 12) / norm,
        ];
        let (x0, x1, x2, x3) = (
            x[b] as i64,
            x[b + 1] as i64,
            x[b + 2] as i64,
            x[b + 3] as i64,
        );
        out[b] = ((q[0] * x0 - q[1] * x1 - q[2] * x2 - q[3] * x3) >> 12) as i32;
        out[b + 1] = ((q[0] * x1 + q[1] * x0 + q[2] * x3 - q[3] * x2) >> 12) as i32;
        out[b + 2] = ((q[0] * x2 - q[1] * x3 + q[2] * x0 + q[3] * x1) >> 12) as i32;
        out[b + 3] = ((q[0] * x3 + q[1] * x2 - q[2] * x1 + q[3] * x0) >> 12) as i32;
    }
    out
}

fn sign_pack_64(slice: &[i32]) -> u64 {
    let mut word = 0u64;
    for (b, &v) in slice.iter().take(64).enumerate() {
        if v >= 0 {
            word |= 1u64 << b;
        }
    }
    word
}

impl ServedHammingRead {
    pub fn apply(&self, x: &mut [i32; FBS_WIDTH], cache: &mut HammingCache) {
        let q_rot = rotate_lanes_q12(x, &self.query_rotation);
        let k_rot = rotate_lanes_q12(x, &self.key_rotation);
        let v_rot = rotate_lanes_q12(x, &self.value_rotation);

        let mut k_codes = [0u64; HAMMING_HEADS];
        for h in 0..HAMMING_HEADS {
            k_codes[h] = sign_pack_64(&k_rot[h * HAMMING_HEAD_DIM..(h + 1) * HAMMING_HEAD_DIM]);
        }
        cache.codes.push(k_codes);
        cache.values.push(v_rot);

        let cur_len = cache.values.len();
        if cur_len <= 1 {
            return;
        }

        let copy_shift = self.copy_key.max(1);
        for h in 0..HAMMING_HEADS {
            let h_base = h * HAMMING_HEAD_DIM;
            let q_code = sign_pack_64(&q_rot[h_base..h_base + HAMMING_HEAD_DIM]);
            let margin_bits =
                ((self.margin.get(h).copied().unwrap_or(1 << 19) >> 16).max(1)) as u32;

            // Multi-horizon FlockSelect: track top-4 lowest Bitwise XOR Popcount Hamming positions
            // alongside attention sink (0) and local causal window.
            let mut top_matches: [(usize, u32); 4] = [(0, u32::MAX); 4];
            for pos in 0..cur_len - 1 {
                let d = popcount_swar_u64(q_code ^ cache.codes[pos][h]);
                if d < top_matches[3].1 {
                    top_matches[3] = (pos, d);
                    for s in (1..4).rev() {
                        if top_matches[s].1 < top_matches[s - 1].1 {
                            top_matches.swap(s, s - 1);
                        }
                    }
                }
            }

            let best_dist = top_matches[0].1;
            if best_dist < margin_bits {
                let mut acc_head = [0i64; HAMMING_HEAD_DIM];
                let mut total_w = 0i64;
                for &(pos, dist) in &top_matches {
                    if dist >= margin_bits {
                        continue;
                    }
                    let diff = (margin_bits - dist) as i64;
                    let w = diff * diff;
                    let copy_pos = (pos + copy_shift).min(cur_len - 1);
                    let copy_val = &cache.values[copy_pos];
                    let type_val = &cache.values[pos];
                    for (d, acc_slot) in acc_head.iter_mut().enumerate() {
                        let idx = h_base + d;
                        // Combine CopyHead (causal successor induction, 3/4) + TypeHead (semantic state, 1/4)
                        let blended_v = ((copy_val[idx] as i64) * 3 + (type_val[idx] as i64)) >> 2;
                        *acc_slot += blended_v * w;
                    }
                    total_w += w;
                }
                if total_w > 0 {
                    let lead_w_num = (margin_bits - best_dist) as i64;
                    let lead_w_den = margin_bits as i64;
                    for (d, &acc_val) in acc_head.iter().enumerate() {
                        let idx = h_base + d;
                        let mean_v = acc_val / total_w;
                        let scale_q12 =
                            (self.weight.get(idx).copied().unwrap_or(1 << 22) >> 12) as i64;
                        let delta = (((mean_v * lead_w_num) / lead_w_den) * scale_q12) >> 14;
                        x[idx] = x[idx].saturating_add(delta as i32);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ServedTurnMemory (/7) and ServedPointKey (/8 + Canonical Anchor Handoff)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TurnMemoryTable {
    pub entries: usize,
    pub key_len: usize,
    pub keys: Vec<u32>,
    pub values: Vec<u32>,
    pub weight_q24: Vec<i32>,
}

impl TurnMemoryTable {
    pub fn lookup(&self, key: &[u32]) -> Option<usize> {
        if self.entries == 0 || key.len() != self.key_len {
            return None;
        }
        let mut lo = 0usize;
        let mut hi = self.entries;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let base = mid * self.key_len;
            let slice = &self.keys[base..base + self.key_len];
            match slice.cmp(key) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => return Some(mid),
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct ServedTurnMemory {
    pub k: usize,
    pub key_window: usize,
    pub reply_turn: Option<usize>,
    pub reply_key_len: usize,
    pub weight_exp: i32,
    pub opening: TurnMemoryTable,
    pub reply: Option<TurnMemoryTable>,
}

#[derive(Clone, Debug, Default)]
pub struct TurnMemoryAnchors {
    pub opening: Vec<u32>,
    pub reply: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct ServedPointKey {
    pub rows: usize,
    pub width: usize,
    pub exp: i32,
    pub vote_k: usize,
    pub points: Vec<i16>,
    pub targets: Vec<u32>,
    /// Canonical `reply` turn_ids (`rows * reply_turn`) for each point row so when `ServedPointKey`
    /// wins on a paraphrased prompt at `row + 1 == reply_start`, `anchors.reply` is updated to the
    /// matched training turn's canonical `turn_ids` and `memory.reply` continues for the entire reply!
    pub canonical_anchors: Option<Vec<u32>>,
    pub anchor_len: usize,
}

impl ServedPointKey {
    pub fn apply_with_row(&self, n_t: &[i32]) -> Option<(u32, usize, i64)> {
        if self.rows == 0 || n_t.len() < self.width {
            return None;
        }
        let k = self.vote_k.max(1).min(self.rows);
        let mut top_k: Vec<(i64, usize)> = Vec::with_capacity(k + 1);
        for r in 0..self.rows {
            let base = r * self.width;
            let row = &self.points[base..base + self.width];
            let mut dist = 0i64;
            for d in 0..self.width {
                dist += ((n_t[d] as i64) - (row[d] as i64)).abs();
            }
            let pos = top_k.partition_point(|&(d_ex, r_ex)| (d_ex, r_ex) <= (dist, r));
            if pos < k {
                top_k.insert(pos, (dist, r));
                if top_k.len() > k {
                    top_k.pop();
                }
            }
        }
        if top_k.is_empty() {
            return None;
        }
        if k == 1 {
            let (dist, r) = top_k[0];
            return Some((self.targets[r], r, dist));
        }
        let mut best_tok = self.targets[top_k[0].1];
        let mut best_row = top_k[0].1;
        let mut best_dist = top_k[0].0;
        let mut best_votes = 0usize;
        for &(dist, r) in &top_k {
            let cand = self.targets[r];
            let votes = top_k
                .iter()
                .filter(|&&(_, r2)| self.targets[r2] == cand)
                .count();
            if votes > best_votes {
                best_votes = votes;
                best_tok = cand;
                best_row = r;
                best_dist = dist;
            }
        }
        Some((best_tok, best_row, best_dist))
    }
}

// ---------------------------------------------------------------------------
// Whole-Table Nearest & Writer Readout
// ---------------------------------------------------------------------------

pub fn stack_whole_table_nearest(n_t: &[i32], writer_table: &[i16], vocab: usize) -> (u32, i64) {
    let mut best_tok = 0u32;
    let mut best_dist = i64::MAX;
    for tok in 0..vocab {
        let base = tok * FBS_WIDTH;
        if base + FBS_WIDTH > writer_table.len() {
            break;
        }
        let row = &writer_table[base..base + FBS_WIDTH];
        let mut dist = 0i64;
        for d in 0..FBS_WIDTH {
            dist += ((n_t[d] as i64) - (row[d] as i64)).abs();
        }
        if dist < best_dist {
            best_dist = dist;
            best_tok = tok as u32;
        }
    }
    (best_tok, best_dist)
}

pub fn stack_whole_table_top_k(
    n_t: &[i32],
    writer_table: &[i16],
    vocab: usize,
    k: usize,
) -> Vec<(u32, i64)> {
    let k = k.max(1);
    let mut top: Vec<(u32, i64)> = Vec::with_capacity(k + 1);
    for tok in 0..vocab {
        let base = tok * FBS_WIDTH;
        if base + FBS_WIDTH > writer_table.len() {
            break;
        }
        let row = &writer_table[base..base + FBS_WIDTH];
        let mut dist = 0i64;
        for d in 0..FBS_WIDTH {
            dist += ((n_t[d] as i64) - (row[d] as i64)).abs();
        }
        let pos = top.partition_point(|&(t_ex, d_ex)| (d_ex, t_ex) <= (dist, tok as u32));
        if pos < k {
            top.insert(pos, (tok as u32, dist));
            if top.len() > k {
                top.pop();
            }
        }
    }
    top
}

// ---------------------------------------------------------------------------
// Complete LutStackModel & LutStackSession (UORLUT01 /7 and /8 Compatible)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct LutStackModel {
    pub schema: String,
    pub artifact_sha256: String,
    pub artifact_bytes: usize,
    pub vocab: usize,
    pub width: usize,
    pub mlp: usize,
    pub context: usize,
    pub layers: Vec<FbsLayer>,
    pub final_gain: Vec<i16>,
    pub writer_table: Vec<i16>,
    pub hamming: Vec<ServedHammingRead>,
    pub memory: Option<ServedTurnMemory>,
    pub point: Option<ServedPointKey>,
    pub dialogue_version: u8,
}

impl LutStackModel {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 16 || &bytes[0..8] != MAGIC {
            return Err("invalid UORLUT01 magic header".to_owned());
        }
        let mut len_bytes = [0u8; 8];
        len_bytes.copy_from_slice(&bytes[8..16]);
        let declared = u64::from_le_bytes(len_bytes) as usize;
        let header_end = 16usize
            .checked_add(declared)
            .filter(|&end| end <= bytes.len())
            .ok_or_else(|| "UORLUT01 header_len exceeds file size".to_owned())?;
        let data_start = header_end.div_ceil(ALIGN) * ALIGN;
        if data_start > bytes.len() {
            return Err("UORLUT01 data_start exceeds file size".to_owned());
        }
        let header_bytes = &bytes[16..header_end];
        let body = &bytes[data_start..];
        let header = JsonVal::parse(header_bytes)
            .ok_or_else(|| "invalid UORLUT01 JSON header".to_owned())?;

        let schema = header
            .get("schema")
            .and_then(JsonVal::as_str)
            .unwrap_or(STACK_SCHEMA_V8)
            .to_owned();
        let shape = header.get("shape");
        let vocab = shape
            .and_then(|s| s.get("vocab"))
            .or_else(|| header.get("vocab"))
            .and_then(JsonVal::as_usize)
            .unwrap_or(4096);
        let width = shape
            .and_then(|s| s.get("width"))
            .or_else(|| header.get("width"))
            .and_then(JsonVal::as_usize)
            .unwrap_or(FBS_WIDTH);
        let mlp = shape
            .and_then(|s| s.get("mlp"))
            .and_then(JsonVal::as_usize)
            .unwrap_or(FBS_MLP_HIDDEN);
        let context = shape
            .and_then(|s| s.get("context"))
            .and_then(JsonVal::as_usize)
            .unwrap_or(192);
        let num_layers = shape
            .and_then(|s| s.get("pattern"))
            .and_then(JsonVal::as_str)
            .map(str::len)
            .unwrap_or(6);
        let dialogue_version = header
            .get("dialogue_version")
            .and_then(JsonVal::as_u64)
            .map(|v| v as u8)
            .unwrap_or(2);

        let mut tables: HashMap<String, (usize, usize)> = HashMap::new();
        if let Some(arr) = header.get("tables").and_then(JsonVal::as_arr) {
            for t in arr {
                let name = t.get("name").and_then(JsonVal::as_str);
                let span = t.get("span");
                let off = span
                    .and_then(|s| s.get("offset"))
                    .or_else(|| t.get("offset"))
                    .and_then(JsonVal::as_usize);
                let len = span
                    .and_then(|s| s.get("bytes"))
                    .or_else(|| t.get("bytes"))
                    .and_then(JsonVal::as_usize);
                if let (Some(n), Some(o), Some(b)) = (name, off, len) {
                    if o + b <= body.len() {
                        tables.insert(n.to_owned(), (o, b));
                    }
                }
            }
        }

        let read_i16 = |name: &str| -> Result<Vec<i16>, String> {
            let &(off, len) = tables
                .get(name)
                .ok_or_else(|| format!("missing table {name}"))?;
            Ok(body[off..off + len]
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]))
                .collect())
        };
        let read_i32 = |name: &str| -> Result<Vec<i32>, String> {
            let &(off, len) = tables
                .get(name)
                .ok_or_else(|| format!("missing table {name}"))?;
            Ok(body[off..off + len]
                .chunks_exact(4)
                .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect())
        };
        let read_u32 = |name: &str| -> Result<Vec<u32>, String> {
            let &(off, len) = tables
                .get(name)
                .ok_or_else(|| format!("missing table {name}"))?;
            Ok(body[off..off + len]
                .chunks_exact(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect())
        };
        let read_route =
            |name: &str, in_dim: usize, out_dim: usize| -> Result<FbsRouteMap, String> {
                let packed = read_u32(name)?;
                Ok(FbsRouteMap::from_packed_u32(&packed, in_dim, out_dim))
            };

        let final_gain = read_i16("final_norm_gain")?;
        let writer_table = read_i16("writer.table")?;
        let gate_rows = (width / 4) + width;

        let mut layers = Vec::with_capacity(num_layers);
        for l in 0..num_layers {
            layers.push(FbsLayer {
                rec_gain: read_i16(&format!("l{l}.rec_norm_gain"))?,
                mlp_gain: read_i16(&format!("l{l}.mlp_norm_gain"))?,
                rec_in: read_route(&format!("l{l}.rec_in.fbs"), width, 2 * width)?,
                rec_gate: read_route(&format!("l{l}.rec_gate.fbs"), width, gate_rows)?,
                rec_out: read_route(&format!("l{l}.rec_out.fbs"), width, width)?,
                mlp_gate: read_route(&format!("l{l}.mlp_gate.fbs"), width, mlp)?,
                mlp_up: read_route(&format!("l{l}.mlp_up.fbs"), width, mlp)?,
                mlp_down: read_route(&format!("l{l}.mlp_down.fbs"), mlp, width)?,
                taps: read_i16(&format!("l{l}.conv_taps"))?,
                conv_bias: read_i32(&format!("l{l}.conv_bias"))?,
                gate_bias: read_i32(&format!("l{l}.gate_bias"))?,
                decay_offset: read_i32(&format!("l{l}.decay_offset"))?,
            });
        }

        let mut hamming = Vec::new();
        if let Some(h_obj) = header.get("hamming") {
            let copy_key = h_obj
                .get("copy_key")
                .and_then(JsonVal::as_usize)
                .unwrap_or(2);
            if let Some(depths) = h_obj.get("depths").and_then(JsonVal::as_arr) {
                for d_val in depths {
                    if let Some(d) = d_val.as_usize() {
                        let p = format!("hamming.d{d}");
                        if tables.contains_key(&format!("{p}.query_rotation")) {
                            hamming.push(ServedHammingRead {
                                depth: d,
                                copy_key,
                                query_rotation: read_i32(&format!("{p}.query_rotation"))?,
                                key_rotation: read_i32(&format!("{p}.key_rotation"))?,
                                value_rotation: read_i32(&format!("{p}.value_rotation"))?,
                                margin: read_i32(&format!("{p}.margin"))?,
                                weight: read_i32(&format!("{p}.weight"))?,
                            });
                        }
                    }
                }
            }
        }

        let mut memory = None;
        let mut anchor_len_for_point = 8usize;
        if let Some(m_obj) = header.get("memory").or_else(|| header.get("turn_memory")) {
            if let Some(k) = m_obj.get("k").and_then(JsonVal::as_usize) {
                let key_window = m_obj
                    .get("key_window")
                    .and_then(JsonVal::as_usize)
                    .unwrap_or(192);
                let opening_entries = m_obj
                    .get("opening")
                    .or_else(|| m_obj.get("opening_entries"))
                    .and_then(JsonVal::as_usize)
                    .unwrap_or(0);
                let reply_entries = m_obj
                    .get("reply")
                    .or_else(|| m_obj.get("reply_entries"))
                    .and_then(JsonVal::as_usize);
                let reply_turn = m_obj.get("reply_turn").and_then(JsonVal::as_usize);
                anchor_len_for_point = reply_turn.unwrap_or(k);
                let reply_key_len = m_obj
                    .get("reply_key_len")
                    .and_then(JsonVal::as_usize)
                    .unwrap_or(MEMORY_REPLY_KEY_LEN);
                let weight_exp = m_obj
                    .get("weight_exp")
                    .and_then(JsonVal::as_i64)
                    .unwrap_or(-24) as i32;

                let opening = TurnMemoryTable {
                    entries: opening_entries,
                    key_len: k,
                    keys: read_u32("memory.opening.keys").unwrap_or_default(),
                    values: read_u32("memory.opening.values").unwrap_or_default(),
                    weight_q24: read_i32("memory.opening.weight").unwrap_or_default(),
                };
                let reply = if let Some(re) = reply_entries {
                    if tables.contains_key("memory.reply.keys") {
                        Some(TurnMemoryTable {
                            entries: re,
                            key_len: reply_key_len,
                            keys: read_u32("memory.reply.keys")?,
                            values: read_u32("memory.reply.values")?,
                            weight_q24: read_i32("memory.reply.weight")?,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };
                memory = Some(ServedTurnMemory {
                    k,
                    key_window,
                    reply_turn,
                    reply_key_len,
                    weight_exp,
                    opening,
                    reply,
                });
            }
        }

        let mut point = None;
        if let Some(p_obj) = header.get("point").or_else(|| header.get("point_key")) {
            let rows = p_obj
                .get("rows")
                .or_else(|| p_obj.get("count"))
                .and_then(JsonVal::as_usize)
                .unwrap_or(0);
            let p_width = p_obj
                .get("width")
                .or_else(|| p_obj.get("dim"))
                .and_then(JsonVal::as_usize)
                .unwrap_or(POINT_KEY_DIM);
            let exp = p_obj.get("exp").and_then(JsonVal::as_i64).unwrap_or(-12) as i32;
            let vote_k = p_obj
                .get("vote_k")
                .and_then(JsonVal::as_usize)
                .unwrap_or(DEFAULT_VOTE_K);
            if rows > 0 && tables.contains_key("memory.point.points") {
                let points = read_i16("memory.point.points")?;
                let targets = read_u32("memory.point.targets")?;
                let canonical_anchors = if tables.contains_key("memory.point.anchors") {
                    Some(read_u32("memory.point.anchors")?)
                } else {
                    None
                };
                let effective_anchor_len = canonical_anchors
                    .as_ref()
                    .filter(|ca| rows > 0 && ca.len() >= rows && ca.len() % rows == 0)
                    .map(|ca| ca.len() / rows)
                    .unwrap_or(anchor_len_for_point);
                point = Some(ServedPointKey {
                    rows,
                    width: p_width,
                    exp,
                    vote_k,
                    points,
                    targets,
                    canonical_anchors,
                    anchor_len: effective_anchor_len,
                });
            }
        }

        let artifact_sha256 = sha256_hex(bytes);

        Ok(Self {
            schema,
            artifact_sha256,
            artifact_bytes: bytes.len(),
            vocab,
            width,
            mlp,
            context,
            layers,
            final_gain,
            writer_table,
            hamming,
            memory,
            point,
            dialogue_version,
        })
    }

    pub fn session(self: &Arc<Self>) -> LutStackSession {
        LutStackSession::new(Arc::clone(self))
    }

    /// Forward a single token through the 6-layer `SpiralCore/FBS` + `ServedHamming` trunk,
    /// returning the 256D Q12 final normalized hidden state `n_t`.
    pub fn forward_token_hidden(
        &self,
        token: u32,
        rec_states: &mut [FbsRecState],
        hamming_caches: &mut [HammingCache],
    ) -> [i32; FBS_WIDTH] {
        let tok_idx = (token as usize).min(self.vocab.saturating_sub(1));
        let row_base = tok_idx * FBS_WIDTH;
        let mut x = [0i32; FBS_WIDTH];
        for (d, slot) in x.iter_mut().enumerate() {
            *slot = self.writer_table.get(row_base + d).copied().unwrap_or(0) as i32;
        }

        for (l_idx, layer) in self.layers.iter().enumerate() {
            let depth = l_idx + 1;
            let rec_normed = stack_norm_gain(&x, &layer.rec_gain);
            let branches = layer.rec_in.forward(&rec_normed);
            let gate_out = layer.rec_gate.forward(&rec_normed);
            let h_next = stack_recurrence_core(
                &branches,
                &gate_out,
                &layer.taps,
                &layer.conv_bias,
                &layer.gate_bias,
                &layer.decay_offset,
                &mut rec_states[l_idx],
            );
            let rec_proj = layer.rec_out.forward(&h_next);
            for d in 0..FBS_WIDTH {
                x[d] = x[d].saturating_add(rec_proj[d]);
            }

            let mlp_normed = stack_norm_gain(&x, &layer.mlp_gain);
            let gate = layer.mlp_gate.forward(&mlp_normed);
            let up = layer.mlp_up.forward(&mlp_normed);
            let hidden = stack_hamilton_ffn(&gate, &up);
            let down = layer.mlp_down.forward(&hidden);
            for d in 0..FBS_WIDTH {
                x[d] = x[d].saturating_add(down[d]);
            }

            for (h_idx, h_op) in self.hamming.iter().enumerate() {
                if h_op.depth == depth {
                    h_op.apply(&mut x, &mut hamming_caches[h_idx]);
                }
            }
        }

        let final_vec = stack_norm_gain(&x, &self.final_gain);
        let mut n_t = [0i32; FBS_WIDTH];
        n_t.copy_from_slice(&final_vec[..FBS_WIDTH]);
        n_t
    }
}

#[derive(Clone, Debug)]
pub struct StepTelemetry {
    pub position: usize,
    pub input_token: u32,
    pub predicted_token: u32,
    pub source_mechanism: &'static str,
    pub l1_distance: i64,
    pub hamming_xor_bits: u32,
    pub icosian_root: u8,
    pub spin_holonomy: u8,
    pub trinary_active_lanes: usize,
    pub top_candidates: Vec<(u32, i64)>,
}

pub struct LutStackSession {
    model: Arc<LutStackModel>,
    rec_states: Vec<FbsRecState>,
    hamming_caches: Vec<HammingCache>,
    tokens: Vec<u32>,
    reply_start: Option<usize>,
    anchors: TurnMemoryAnchors,
    reply_history: Vec<u32>,
    suffix_counts: Vec<([u32; 4], u32)>,
    pub pending_basins: Vec<(Vec<u32>, u32)>,
    pub last_matched_stems: usize,
    pub last_point_hamming_bits: u32,
}

impl LutStackSession {
    pub fn new(model: Arc<LutStackModel>) -> Self {
        let rec_states = vec![FbsRecState::default(); model.layers.len()];
        let hamming_caches = vec![HammingCache::default(); model.hamming.len()];
        Self {
            model,
            rec_states,
            hamming_caches,
            tokens: Vec::new(),
            reply_start: None,
            anchors: TurnMemoryAnchors::default(),
            reply_history: Vec::new(),
            suffix_counts: Vec::new(),
            pending_basins: Vec::new(),
            last_matched_stems: 0,
            last_point_hamming_bits: 0,
        }
    }

    pub fn reset(&mut self) {
        for s in &mut self.rec_states {
            *s = FbsRecState::default();
        }
        for c in &mut self.hamming_caches {
            *c = HammingCache::default();
        }
        self.tokens.clear();
        self.reply_start = None;
        self.anchors = TurnMemoryAnchors::default();
        self.reply_history.clear();
        self.suffix_counts.clear();
        self.pending_basins.clear();
        self.last_matched_stems = 0;
        self.last_point_hamming_bits = 0;
    }

    pub fn position(&self) -> usize {
        self.tokens.len()
    }

    pub fn mark_reply_start(&mut self) {
        self.reply_start = Some(self.tokens.len() + 1);
        self.reply_history.clear();
        self.suffix_counts.clear();
    }

    pub fn clear_reply_start(&mut self) {
        self.reply_start = None;
        self.reply_history.clear();
        self.suffix_counts.clear();
    }

    /// Prefill a prompt token into the session state (`run_trunk = true` for the active turn tail).
    pub fn prefill_token(&mut self, token: u32, run_trunk: bool) {
        self.tokens.push(token);
        if run_trunk {
            let _ = self.model.forward_token_hidden(
                token,
                &mut self.rec_states,
                &mut self.hamming_caches,
            );
        }
    }

    fn next_4gram_suffix_occurrence(&mut self, tok: u32) -> ([u32; 4], u32) {
        self.reply_history.push(tok);
        let n = self.reply_history.len();
        let s0 = if n >= 4 {
            self.reply_history[n - 4]
        } else {
            MEMORY_PAD
        };
        let s1 = if n >= 3 {
            self.reply_history[n - 3]
        } else {
            MEMORY_PAD
        };
        let s2 = if n >= 2 {
            self.reply_history[n - 2]
        } else {
            MEMORY_PAD
        };
        let s3 = self.reply_history[n - 1];
        let suffix = [s0, s1, s2, s3];
        for (existing_suf, count) in &mut self.suffix_counts {
            if *existing_suf == suffix {
                let occ = *count;
                *count = count.saturating_add(1);
                return (suffix, occ);
            }
        }
        self.suffix_counts.push((suffix, 1));
        (suffix, 0)
    }

    /// Execute one causal token step through the 6-layer Spin-Preserving `SpiralCore/FBS` +
    /// `ServedHamming` Bitwise XOR Popcount attention + Position-Free Causal 4-Gram `ServedTurnMemory` (`/7`) +
    /// `ServedPointKey` (`/8` with 32-anchor stem/BPE resonance) + `stack_whole_table_nearest` over `writer.table`.
    pub fn step_with_telemetry(&mut self, token: u32) -> StepTelemetry {
        let row = self.tokens.len();
        self.tokens.push(token);

        let mut n_t =
            self.model
                .forward_token_hidden(token, &mut self.rec_states, &mut self.hamming_caches);

        let mut source_mechanism = "whole_table_nearest";
        let mut memory_hit_token: Option<u32> = None;

        let suffix_occ = if self.reply_start.is_some_and(|rs| row >= rs) {
            Some(self.next_4gram_suffix_occurrence(token))
        } else {
            None
        };

        if let (Some(mem), Some(rs)) = (&self.model.memory, self.reply_start) {
            let reply_turn_k = mem.reply_turn.unwrap_or(mem.k);
            if row + 1 == rs {
                let open_ids = extract_turn_ids(&self.tokens, mem.key_window, mem.k);
                let reply_ids = extract_turn_ids(&self.tokens, mem.key_window, reply_turn_k);
                self.anchors.opening = open_ids.clone();
                self.anchors.reply = reply_ids.clone();

                if let Some(entry_idx) = mem.opening.lookup(&open_ids) {
                    let val_tok = mem.opening.values[entry_idx];
                    self.last_matched_stems = 4;
                    self.last_point_hamming_bits = 0;
                    if let (Some(reply_tbl), Some(point)) = (&mem.reply, &self.model.point) {
                        if !probe_reply_table_has_anchor(
                            reply_tbl,
                            mem.reply_key_len,
                            &self.anchors.reply,
                            val_tok,
                        ) {
                            let mut matched_open_anchor = None;
                            if let Some(anchors) = &point.canonical_anchors {
                                let alen = point.anchor_len.max(1);
                                if alen >= reply_turn_k + open_ids.len()
                                    && anchors.len() >= point.rows * alen
                                {
                                    for r in 0..point.rows {
                                        let base = r * alen;
                                        let row_open = &anchors[base + reply_turn_k
                                            ..base + reply_turn_k + open_ids.len()];
                                        if row_open == open_ids.as_slice() {
                                            matched_open_anchor =
                                                Some(anchors[base..base + reply_turn_k].to_vec());
                                            break;
                                        }
                                    }
                                }
                            }
                            if let Some(canon) = matched_open_anchor {
                                self.anchors.reply = canon;
                            } else if let Some(pt_hit) = select_point_key_with_resonance(
                                point,
                                &n_t,
                                &self.tokens,
                                mem.key_window,
                            ) {
                                self.last_matched_stems = pt_hit.matched_stems;
                                self.last_point_hamming_bits = pt_hit.hamming_bits;
                                if let Some(canon) = resolve_canonical_reply_anchor(
                                    point, mem, pt_hit.row, val_tok, &reply_ids,
                                ) {
                                    self.anchors.reply = canon;
                                }
                            }
                        }
                    }
                    blend_toward_writer_row(
                        &mut n_t,
                        &self.model.writer_table,
                        val_tok as usize,
                        &mem.opening.weight_q24,
                    );
                    memory_hit_token = Some(val_tok);
                    source_mechanism = "turn_memory_opening";
                } else if let Some(point) = &self.model.point {
                    if let Some(pt_hit) =
                        select_point_key_with_resonance(point, &n_t, &self.tokens, mem.key_window)
                    {
                        self.last_matched_stems = pt_hit.matched_stems;
                        self.last_point_hamming_bits = pt_hit.hamming_bits;
                        if let Some(canon) = resolve_canonical_reply_anchor(
                            point,
                            mem,
                            pt_hit.row,
                            pt_hit.target_token,
                            &reply_ids,
                        ) {
                            self.anchors.reply = canon;
                        }
                        blend_toward_writer_row(
                            &mut n_t,
                            &self.model.writer_table,
                            pt_hit.target_token as usize,
                            &mem.opening.weight_q24,
                        );
                        memory_hit_token = Some(pt_hit.target_token);
                        source_mechanism = if pt_hit.confident {
                            "point_key_vote"
                        } else {
                            "point_key_fallback"
                        };
                    }
                }
            } else if row >= rs {
                let (suffix4, occ) = suffix_occ.unwrap_or(([MEMORY_PAD; 4], 0));
                let step = (row - rs + 1) as u32;
                if let Some(reply_tbl) = &mem.reply {
                    if let Some(val_tok) = lookup_reply_continuation_4gram(
                        reply_tbl,
                        mem.reply_key_len,
                        &self.anchors.reply,
                        suffix4,
                        occ,
                        step,
                    ) {
                        blend_toward_writer_row(
                            &mut n_t,
                            &self.model.writer_table,
                            val_tok as usize,
                            &reply_tbl.weight_q24,
                        );
                        memory_hit_token = Some(val_tok);
                        source_mechanism = "turn_memory_reply";
                    }
                }
            }
        }

        let mut top_candidates =
            stack_whole_table_top_k(&n_t, &self.model.writer_table, self.model.vocab, 5);
        let (nearest_tok, nearest_dist) = top_candidates.first().copied().unwrap_or_else(|| {
            stack_whole_table_nearest(&n_t, &self.model.writer_table, self.model.vocab)
        });

        let predicted_token = memory_hit_token.unwrap_or(nearest_tok);
        let q_sketch = pack_i32_sign_sketch_256(&n_t);
        let pred_base = (predicted_token as usize) * FBS_WIDTH;
        let hamming_xor_bits = if pred_base + FBS_WIDTH <= self.model.writer_table.len() {
            let w_sketch = pack_i16_sign_sketch_256(
                &self.model.writer_table[pred_base..pred_base + FBS_WIDTH],
            );
            hamming_distance_256(&q_sketch, &w_sketch)
        } else {
            0
        };

        let l1_distance = if let Some(hit_tok) = memory_hit_token {
            let base = (hit_tok as usize) * FBS_WIDTH;
            let dist = if base + FBS_WIDTH <= self.model.writer_table.len() {
                let row_w = &self.model.writer_table[base..base + FBS_WIDTH];
                (0..FBS_WIDTH)
                    .map(|d| ((n_t[d] as i64) - (row_w[d] as i64)).abs())
                    .sum()
            } else {
                nearest_dist
            };
            if !top_candidates.iter().any(|&(t, _)| t == hit_tok) {
                top_candidates.insert(0, (hit_tok, dist));
                top_candidates.truncate(5);
            }
            dist
        } else {
            nearest_dist
        };

        let (icosian_root, spin_holonomy, trinary_active_lanes) =
            if let Some(last_layer) = self.rec_states.last() {
                let r = last_layer.icosian_trajectory[3];
                let mut h_acc = 0u8;
                let mut active = 0usize;
                for lane in 0..FBS_LANES {
                    h_acc = cl06_compose(h_acc, last_layer.spin_holonomy[lane]);
                    if last_layer.trinary_route[lane] != 0 {
                        active += 1;
                    }
                }
                (r, h_acc, active)
            } else {
                (0, 0, 0)
            };

        StepTelemetry {
            position: self.tokens.len(),
            input_token: token,
            predicted_token,
            source_mechanism,
            l1_distance,
            hamming_xor_bits,
            icosian_root,
            spin_holonomy,
            trinary_active_lanes,
            top_candidates,
        }
    }
}

/// Probe whether `reply_tbl` contains a continuation for `anchor` starting from `first_tok`.
fn probe_reply_table_has_anchor(
    reply_tbl: &TurnMemoryTable,
    reply_key_len: usize,
    anchor: &[u32],
    first_tok: u32,
) -> bool {
    lookup_reply_continuation_4gram(
        reply_tbl,
        reply_key_len,
        anchor,
        [MEMORY_PAD, MEMORY_PAD, MEMORY_PAD, first_tok],
        0,
        1,
    )
    .is_some()
}

/// Universal corpus-wide suffix anchor (`0xE000_0000`), disjoint from turn-specific anchors (`0x8000_0000 | idx`),
/// stem hashes (`0x4000_0000 | h`), BPE token IDs (`0..4095`), and `MEMORY_PAD` (`0xFFFF_FFFF`).
pub const GLOBAL_SUFFIX_ANCHOR: u32 = 0xE000_0000;

/// Multi-Scale Position-Free Causal Suffix lookup in `reply_tbl`:
/// 1. Tier 1: Anchor-bound 4-gram (`(anchor, occ, prev_3, prev_2, prev_1, curr_tok)`) + `occ = 0` backoff
/// 2. Tier 2: Anchor-bound 2-gram backoff (`(anchor, 0, PAD, PAD, prev_1, curr_tok)`)
/// 3. Tier 3: Universal corpus-wide 4-gram (`(GLOBAL_SUFFIX_ANCHOR, 0, prev_3, prev_2, prev_1, curr_tok)`)
///    and 3-gram (`(GLOBAL_SUFFIX_ANCHOR, 0, PAD, prev_2, prev_1, curr_tok)`) continuation
/// 4. Tier 4: Legacy step-index compatibility fallback
fn lookup_reply_continuation_4gram(
    reply_tbl: &TurnMemoryTable,
    reply_key_len: usize,
    anchor: &[u32],
    suffix4: [u32; 4],
    occ: u32,
    legacy_step: u32,
) -> Option<u32> {
    if reply_key_len < 6 {
        return None;
    }
    let copy_n = anchor.len().min(reply_key_len.saturating_sub(5));
    let mut key = vec![MEMORY_PAD; reply_key_len];
    key[..copy_n].copy_from_slice(&anchor[..copy_n]);
    key[reply_key_len - 5] = occ;
    key[reply_key_len - 4] = suffix4[0];
    key[reply_key_len - 3] = suffix4[1];
    key[reply_key_len - 2] = suffix4[2];
    key[reply_key_len - 1] = suffix4[3];
    if let Some(idx) = reply_tbl.lookup(&key) {
        return Some(reply_tbl.values[idx]);
    }
    if occ > 0 {
        key[reply_key_len - 5] = 0;
        if let Some(idx) = reply_tbl.lookup(&key) {
            return Some(reply_tbl.values[idx]);
        }
    }
    // Tier 2: Anchor-bound 2-gram backoff (when `prev_1 != MEMORY_PAD`)
    if suffix4[2] != MEMORY_PAD {
        key[reply_key_len - 5] = 0;
        key[reply_key_len - 4] = MEMORY_PAD;
        key[reply_key_len - 3] = MEMORY_PAD;
        key[reply_key_len - 2] = suffix4[2];
        key[reply_key_len - 1] = suffix4[3];
        if let Some(idx) = reply_tbl.lookup(&key) {
            return Some(reply_tbl.values[idx]);
        }
    }
    // Tier 3: Universal corpus-wide 4-gram and 3-gram continuation backoff
    if suffix4[1] != MEMORY_PAD {
        let mut gkey = vec![MEMORY_PAD; reply_key_len];
        gkey[0] = GLOBAL_SUFFIX_ANCHOR;
        gkey[reply_key_len - 5] = 0;
        gkey[reply_key_len - 4] = suffix4[0];
        gkey[reply_key_len - 3] = suffix4[1];
        gkey[reply_key_len - 2] = suffix4[2];
        gkey[reply_key_len - 1] = suffix4[3];
        if suffix4[0] != MEMORY_PAD {
            if let Some(idx) = reply_tbl.lookup(&gkey) {
                return Some(reply_tbl.values[idx]);
            }
        }
        gkey[reply_key_len - 4] = MEMORY_PAD;
        if let Some(idx) = reply_tbl.lookup(&gkey) {
            return Some(reply_tbl.values[idx]);
        }
    }
    // Tier 4: Compatibility fallback for step-indexed `.uorlut` tables
    let mut legacy_key = vec![MEMORY_PAD; reply_key_len];
    let leg_copy = anchor.len().min(reply_key_len.saturating_sub(4));
    legacy_key[..leg_copy].copy_from_slice(&anchor[..leg_copy]);
    legacy_key[reply_key_len - 4] = MEMORY_PAD;
    legacy_key[reply_key_len - 3] = legacy_step;
    legacy_key[reply_key_len - 2] = MEMORY_PAD;
    legacy_key[reply_key_len - 1] = suffix4[3];
    reply_tbl
        .lookup(&legacy_key)
        .map(|idx| reply_tbl.values[idx])
}

fn blend_toward_writer_row(
    n_t: &mut [i32; FBS_WIDTH],
    writer_table: &[i16],
    tok: usize,
    weight_q24: &[i32],
) {
    let base = tok * FBS_WIDTH;
    if base + FBS_WIDTH > writer_table.len() {
        return;
    }
    let row = &writer_table[base..base + FBS_WIDTH];
    for d in 0..FBS_WIDTH {
        let w = (weight_q24.get(d).copied().unwrap_or(1 << 23) >> 12).clamp(0, 4096) as i64;
        let cur = n_t[d] as i64;
        let target = row[d] as i64;
        n_t[d] = (((4096 - w) * cur + w * target) >> 12).clamp(-32768, 32767) as i32;
    }
}

/// Extract the `k` salient turn token IDs from the user prompt (`stack/memory.rs` `turn_key`).
pub fn extract_turn_ids(tokens: &[u32], key_window: usize, k: usize) -> Vec<u32> {
    let bpe = ByteBpe4096::global();
    let nl_user_v2 = bpe.encode("\nUser:");
    let nl_user_v1 = bpe.encode("\nUser: ");
    let user_v2 = bpe.encode("User:");
    let user_v1 = bpe.encode("User: ");
    let nl_asst_v2 = bpe.encode("\nAssistant:");
    let nl_asst_v1 = bpe.encode("\nAssistant: ");

    let win_start = tokens.len().saturating_sub(key_window);
    let win = &tokens[win_start..];

    let end = if win.ends_with(&nl_asst_v2) {
        win.len() - nl_asst_v2.len()
    } else if win.ends_with(&nl_asst_v1) {
        win.len() - nl_asst_v1.len()
    } else {
        win.len()
    };

    let slice = &win[..end];
    let mut content_start = 0usize;
    if slice.len() >= nl_user_v2.len() {
        for i in (0..=slice.len() - nl_user_v2.len()).rev() {
            if slice[i..].starts_with(&nl_user_v2) {
                content_start = i + nl_user_v2.len();
                break;
            }
            if slice[i..].starts_with(&nl_user_v1) {
                content_start = i + nl_user_v1.len();
                break;
            }
        }
    }
    if content_start == 0 {
        for i in 0..slice.len() {
            if slice[i..].starts_with(&user_v2) {
                content_start = i + user_v2.len();
                break;
            }
            if slice[i..].starts_with(&user_v1) {
                content_start = i + user_v1.len();
                break;
            }
        }
    }

    let user_tokens = &slice[content_start..];
    let mut out = vec![MEMORY_PAD; k];
    let take = user_tokens.len().min(k);
    let tail = &user_tokens[user_tokens.len() - take..];
    out[..take].copy_from_slice(tail);
    out
}

pub fn is_bpe_framing_stopword(word: &str) -> bool {
    matches!(
        word,
        "what"
            | "how"
            | "why"
            | "who"
            | "where"
            | "when"
            | "which"
            | "is"
            | "are"
            | "was"
            | "were"
            | "be"
            | "been"
            | "do"
            | "does"
            | "did"
            | "can"
            | "could"
            | "would"
            | "should"
            | "will"
            | "shall"
            | "may"
            | "might"
            | "tell"
            | "let"
            | "lets"
            | "talk"
            | "talking"
            | "discuss"
            | "discussing"
            | "me"
            | "us"
            | "about"
            | "explain"
            | "describe"
            | "please"
            | "the"
            | "a"
            | "an"
            | "and"
            | "or"
            | "but"
            | "in"
            | "on"
            | "at"
            | "to"
            | "for"
            | "of"
            | "with"
            | "by"
            | "from"
            | "as"
            | "into"
            | "like"
            | "between"
            | "difference"
            | "compare"
            | "vs"
            | "versus"
            | "user"
            | "assistant"
            | "system"
            | "you"
            | "your"
            | "my"
            | "i"
            | "we"
            | "they"
            | "it"
            | "its"
            | "that"
            | "this"
            | "these"
            | "those"
            | "there"
            | "here"
            | "have"
            | "has"
            | "had"
            | "not"
            | "no"
            | "yes"
            | "if"
            | "then"
            | "than"
            | "so"
            | "also"
            | "just"
            | "more"
            | "most"
            | "some"
            | "any"
            | "all"
            | "other"
            | "such"
            | "only"
            | "own"
            | "same"
            | "too"
            | "very"
            | "make"
            | "makes"
            | "made"
            | "work"
            | "works"
            | "working"
            | "use"
            | "used"
            | "using"
            | "give"
            | "show"
            | "write"
            | "clear"
            | "terms"
            | "simple"
            | "simply"
            | "briefly"
            | "today"
            | "cause"
            | "causes"
            | "caused"
            | "happen"
            | "happens"
            | "through"
            | "walk"
            | "beginner"
            | "beginners"
    )
}

/// Normalize a lowercase word into its morphological stem so inflectional variants
/// (`"superconductors"` / `"superconductor"`, `"entangled"` / `"entanglement"`) share an exact stem hash.
pub fn lut_canonical_stem(word: &str) -> String {
    let mut s: String = word
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    for suffix in [
        "izations", "ization", "fulness", "ousness", "iveness", "ational", "bilities", "bility",
        "ings", "tion", "tions", "ment", "ments", "ness", "ities", "ity", "ally", "edly", "ing",
        "ies", "ers", "ors", "ed", "es", "ly", "er", "or", "s",
    ] {
        if s.len() >= suffix.len() + 3 && s.ends_with(suffix) {
            s.truncate(s.len() - suffix.len());
            if suffix == "ies" {
                s.push('y');
            }
            break;
        }
    }
    s
}

/// Compute a 31-bit deterministic FNV-1a stem anchor hash (`0x4000_0000..0x7FFF_FFFF`),
/// disjoint from BPE token IDs (`0..4095`), turn markers (`0x8000_0000 | idx`), and `MEMORY_PAD`.
#[inline]
pub fn stem_anchor_hash(word: &str) -> u32 {
    let stem = lut_canonical_stem(word);
    let mut h: u32 = 0x811C_9DC5;
    for b in stem.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    0x4000_0000u32 | (h & 0x3FFF_FFFFu32)
}

/// Extract deduplicated `(stem_anchor_hash, specificity_weight)` pairs from natural text,
/// preserving hyphenated/plus-joined technical compounds (`"B-Trees"` -> `"btree"`, `"UTF-8"` -> `"utf8"`).
pub fn extract_salient_stem_hashes_from_text(text: &str, max_stems: usize) -> Vec<(u32, i64)> {
    let mut out: Vec<(u32, i64)> = Vec::new();
    // 1. First extract joined compounds from hyphenated/plus tokens (e.g. "B-Trees" -> "btree")
    for raw_tok in text.split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '+') {
        if raw_tok.contains('-') || raw_tok.contains('+') {
            let joined: String = raw_tok
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase();
            if joined.len() >= 3 && !is_bpe_framing_stopword(&joined) {
                let stem = lut_canonical_stem(&joined);
                if stem.len() >= 3 && !is_bpe_framing_stopword(&stem) {
                    let sh = stem_anchor_hash(&stem);
                    if !out.iter().any(|&(existing, _)| existing == sh) {
                        out.push((sh, 240_000i64));
                        if out.len() >= max_stems {
                            return out;
                        }
                    }
                }
            }
        }
    }
    // 2. Then extract individual alphanumeric words
    for raw_w in text.split(|c: char| !c.is_ascii_alphanumeric()) {
        let lower = raw_w.to_ascii_lowercase();
        if lower.len() < 2 || is_bpe_framing_stopword(&lower) {
            continue;
        }
        let stem = lut_canonical_stem(&lower);
        if stem.len() < 2 || is_bpe_framing_stopword(&stem) {
            continue;
        }
        let sh = stem_anchor_hash(&stem);
        if out.iter().any(|&(existing, _)| existing == sh) {
            continue;
        }
        let weight = if stem.len() >= 6 {
            240_000i64
        } else if stem.len() >= 4 {
            160_000i64
        } else {
            100_000i64
        };
        out.push((sh, weight));
        if out.len() >= max_stems {
            break;
        }
    }
    out
}

pub fn is_lut_self_inquiry_prompt(lower: &str) -> bool {
    lower.contains("able to do")
        || lower.contains("can you do")
        || lower.contains("tasks you")
        || lower.contains("tasks can you")
        || lower.contains("capable of")
        || lower.contains("capabilities")
        || lower.contains("who are you")
        || lower.contains("what are you")
        || lower.contains("what model are you")
        || lower.contains("introduce yourself")
        || lower.contains("about yourself")
        || lower.contains("talk about you")
        || lower.contains("tell me about you")
        || lower.contains("how do you work")
        || lower.contains("under the hood")
        || lower.contains("explain your architecture")
        || lower.contains("what is uor-r4")
        || lower.contains("how does uor-r4")
        || lower.contains("lut-stack")
        || lower.contains("are you a transformer")
        || lower.contains("how can you help")
}

pub fn is_lut_greeting_only_prompt(lower: &str) -> bool {
    let trimmed = lower
        .trim()
        .trim_matches(|c: char| !c.is_ascii_alphanumeric());
    matches!(
        trimmed,
        "hello"
            | "hi"
            | "hey"
            | "greetings"
            | "good morning"
            | "good afternoon"
            | "good evening"
            | "how are you"
            | "how are you doing"
            | "how are you doing today"
            | "how is it going"
            | "hows it going"
            | "how are things"
            | "lets chat"
            | "let us chat"
    )
}

#[derive(Clone, Copy, Debug)]
pub struct PointResonanceHit {
    pub target_token: u32,
    pub row: usize,
    pub raw_l1: i64,
    pub hamming_bits: u32,
    pub matched_stems: usize,
    pub confident: bool,
}

pub fn select_point_key_with_resonance(
    point: &ServedPointKey,
    n_t: &[i32],
    prompt_tokens: &[u32],
    key_window: usize,
) -> Option<PointResonanceHit> {
    let Some(anchors) = &point.canonical_anchors else {
        return point
            .apply_with_row(n_t)
            .map(|(t, r, d)| PointResonanceHit {
                target_token: t,
                row: r,
                raw_l1: d,
                hamming_bits: 0,
                matched_stems: usize::from(d == 0),
                confident: d == 0,
            });
    };
    let alen = point.anchor_len.max(1);
    if anchors.len() < point.rows * alen {
        return point
            .apply_with_row(n_t)
            .map(|(t, r, d)| PointResonanceHit {
                target_token: t,
                row: r,
                raw_l1: d,
                hamming_bits: 0,
                matched_stems: usize::from(d == 0),
                confident: d == 0,
            });
    }

    let bpe = ByteBpe4096::global();
    let prompt_ids = extract_turn_ids(prompt_tokens, key_window, key_window.max(128));
    let active_prompt_ids: Vec<u32> = prompt_ids
        .iter()
        .copied()
        .filter(|&t| t != MEMORY_PAD)
        .collect();
    let decoded_turn = bpe.decode(&active_prompt_ids).to_ascii_lowercase();
    let is_self_inq = is_lut_self_inquiry_prompt(&decoded_turn);
    let is_greeting = is_lut_greeting_only_prompt(&decoded_turn);

    let prompt_stems = extract_salient_stem_hashes_from_text(&decoded_turn, 16);
    let mut prompt_bpe_content: Vec<(u32, i64)> = Vec::new();
    for &t in &active_prompt_ids {
        if t < 259 || prompt_bpe_content.iter().any(|&(id, _)| id == t) {
            continue;
        }
        let dec = bpe.decode(&[t]);
        let clean: String = dec
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        if clean.len() < 2 || is_bpe_framing_stopword(&clean) {
            continue;
        }
        let w = if clean.len() >= 5 || t >= 500 {
            95_000i64
        } else {
            45_000i64
        };
        prompt_bpe_content.push((t, w));
    }

    let q_sketch = pack_i32_sign_sketch_256(n_t);
    let q_abs_sum: i64 = n_t.iter().map(|&v| (v as i64).abs()).sum();
    let q_mean_abs = (q_abs_sum / (point.width.max(1) as i64)).max(1);
    let mut q_normed = [0i64; FBS_WIDTH];
    let w_lim = point.width.min(FBS_WIDTH);
    for d in 0..w_lim {
        q_normed[d] = ((n_t[d] as i64) * 1024) / q_mean_abs;
    }

    let mut best_row = 0usize;
    let mut best_score = i64::MAX;
    let mut best_raw_l1 = i64::MAX;
    let mut best_hamming = 256u32;
    let mut best_matched_stems = 0usize;
    let mut best_matched_bpe = 0usize;
    let mut best_turn_idx = usize::MAX;
    let mut best_bonus = 0i64;
    let eff_total = prompt_stems.len().min(6);
    let mut max_core_matched_stems = 0usize;

    for r in 0..point.rows {
        let p_base = r * point.width;
        let p_row = &point.points[p_base..p_base + point.width];
        let a_base = r * alen;
        let a_row = &anchors[a_base..a_base + alen];

        let turn_idx = if (a_row[0] & 0x8000_0000) != 0 {
            (a_row[0] & 0x7FFF_FFFF) as usize
        } else {
            usize::MAX
        };
        if turn_idx >= 620 && (eff_total < 3 || eff_total < max_core_matched_stems + 2) {
            break;
        }

        // Guard self-inquiry bio rows (0..=2) and pure greeting rows (3..=4) so unrelated prompts
        // never collapse onto "I am the UOR-R4 Generalized Geometric Assistant..."
        let mut guard_penalty = 0i64;
        if turn_idx <= 2 && !is_self_inq {
            guard_penalty = 80_000_000;
        } else if (turn_idx == 3 || turn_idx == 4) && !is_greeting && !prompt_stems.is_empty() {
            guard_penalty = 60_000_000;
        }

        let primary_slice = if alen >= 22 {
            &a_row[16..22]
        } else if alen > 16 {
            &a_row[16..alen]
        } else {
            &[]
        };
        let secondary_slice = if alen >= 28 { &a_row[22..28] } else { &[] };
        let bpe_slice = if alen >= 32 { &a_row[28..32] } else { &[] };
        let row_primary_count = primary_slice
            .iter()
            .filter(|&&x| x != MEMORY_PAD)
            .count()
            .max(1);

        let mut bonus = 0i64;
        let mut matched_stems = 0usize;
        let mut matched_primary = 0usize;
        for &(sh, sw) in &prompt_stems {
            if primary_slice.contains(&sh) {
                matched_stems += 1;
                matched_primary += 1;
                bonus += sw;
            } else if secondary_slice.contains(&sh) {
                matched_stems += 1;
                bonus += sw / 2;
            }
        }
        if matched_primary > 0 {
            bonus += ((matched_primary as i64) * 160_000) / (row_primary_count as i64);
            if matched_primary == prompt_stems.len()
                && prompt_stems.len() == 1
                && row_primary_count <= 3
            {
                matched_stems = 2;
            }
        }
        let mut matched_bpe = 0usize;
        for &(pt, bw) in &prompt_bpe_content {
            if bpe_slice.contains(&pt) {
                matched_bpe += 1;
                bonus += bw;
            }
        }
        if matched_stems >= 2 {
            bonus += (matched_stems as i64) * 180_000;
        }
        // Core verified curriculum turns (`0..620`) receive deterministic priority, while external
        // corpus turns (`>= 620`) activate when they match >= 3 stems, >= 2/3 of `eff_total`, and
        // at least 2 more stems than the best core curriculum turn (`max_core_matched_stems + 2`).
        if turn_idx < 620 && matched_stems >= 1 {
            if matched_stems > max_core_matched_stems {
                max_core_matched_stems = matched_stems;
            }
            bonus += (matched_stems as i64) * 350_000;
        } else if turn_idx >= 620 {
            if matched_stems < 3
                || matched_stems < max_core_matched_stems + 2
                || matched_stems * 3 < eff_total * 2
            {
                continue;
            }
            bonus += (matched_stems as i64) * 350_000;
        }

        // Exact branch-and-bound lower bound:
        // Whenever `raw_l1 > 0` (checked in O(1) via lane 0/1 inequality), `geom_dist >= guard_penalty`,
        // so `score = geom_dist - bonus >= guard_penalty - bonus`.
        let maybe_exact_zero = turn_idx < 620
            && w_lim >= 2
            && (p_row[0] as i32) == n_t[0]
            && (p_row[1] as i32) == n_t[1];
        let max_possible_bonus = if turn_idx >= 620 && w_lim >= 2 && (p_row[0] as i32) == n_t[0] {
            bonus + 150_000
        } else {
            bonus
        };
        if !maybe_exact_zero && (guard_penalty - max_possible_bonus) >= best_score {
            continue;
        }

        let mut raw_l1 = 0i64;
        let mut p_abs_sum = 0i64;
        for d in 0..w_lim {
            let pv = p_row[d] as i64;
            raw_l1 += ((n_t[d] as i64) - pv).abs();
            p_abs_sum += pv.abs();
        }
        let p_mean_abs = (p_abs_sum / (w_lim.max(1) as i64)).max(1);

        // Scale-normalized Q12 directional L1 distance (eliminates small-norm bias across rows)
        let mut norm_l1 = 0i64;
        for d in 0..w_lim {
            let p_normed = ((p_row[d] as i64) * 1024) / p_mean_abs;
            norm_l1 += (q_normed[d] - p_normed).abs();
        }

        // 256-bit Bitwise Popcount Hamming XOR distance between query sketch and point row sketch
        let p_sketch = pack_i16_sign_sketch_256(p_row);
        let h_bits = hamming_distance_256(&q_sketch, &p_sketch);
        let geom_dist = norm_l1 + (h_bits as i64) * 1024 + guard_penalty;

        let score = if raw_l1 == 0 && guard_penalty == 0 && turn_idx < 620 {
            -(1i64 << 42) - bonus
        } else if raw_l1 == 0 && guard_penalty == 0 {
            geom_dist - bonus - 150_000
        } else {
            geom_dist - bonus
        };

        if score < best_score {
            best_score = score;
            best_row = r;
            best_raw_l1 = raw_l1;
            best_hamming = h_bits;
            best_matched_stems = matched_stems;
            best_matched_bpe = matched_bpe;
            best_turn_idx = turn_idx;
            best_bonus = bonus;
        }
    }

    if best_score < i64::MAX {
        let total_stems = prompt_stems.len();
        let confident = (best_raw_l1 == 0 && (best_turn_idx > 2 || is_self_inq))
            || (is_self_inq && best_turn_idx <= 2)
            || (is_greeting && (best_turn_idx == 3 || best_turn_idx == 4))
            || (best_matched_stems >= 2 && best_matched_stems * 3 >= total_stems)
            || (best_matched_stems == 1 && total_stems <= 2 && best_bonus >= 160_000)
            || (total_stems == 0
                && !prompt_bpe_content.is_empty()
                && best_matched_bpe >= prompt_bpe_content.len().min(2));
        Some(PointResonanceHit {
            target_token: point.targets[best_row],
            row: best_row,
            raw_l1: best_raw_l1,
            hamming_bits: best_hamming,
            matched_stems: best_matched_stems,
            confident,
        })
    } else {
        point
            .apply_with_row(n_t)
            .map(|(t, r, d)| PointResonanceHit {
                target_token: t,
                row: r,
                raw_l1: d,
                hamming_bits: 128,
                matched_stems: usize::from(d == 0),
                confident: d == 0,
            })
    }
}

fn resolve_canonical_reply_anchor(
    point: &ServedPointKey,
    mem: &ServedTurnMemory,
    pt_row: usize,
    first_reply_tok: u32,
    prompt_reply_ids: &[u32],
) -> Option<Vec<u32>> {
    let alen = point.anchor_len.max(1);
    let reply_k = mem.reply_turn.unwrap_or(mem.k).min(alen);
    if let Some(anchors) = &point.canonical_anchors {
        let base = pt_row * alen;
        if base + alen <= anchors.len() {
            return Some(anchors[base..base + reply_k].to_vec());
        }
    }
    let kw = mem.k;
    let mut best_entry = None;
    let mut best_overlap = -1i32;
    for e in 0..mem.opening.entries {
        if mem.opening.values[e] != first_reply_tok {
            continue;
        }
        let k_slice = &mem.opening.keys[e * kw..(e + 1) * kw];
        let overlap = prompt_reply_ids
            .iter()
            .filter(|&&t| t != MEMORY_PAD && k_slice.contains(&t))
            .count() as i32;
        if overlap > best_overlap {
            best_overlap = overlap;
            best_entry = Some(k_slice.to_vec());
        }
    }
    best_entry
}

/// Segment a multi-part user prompt (`"Explain X and also tell me about Y"`, `"What is A? Also how does B work?"`)
/// into ordered constituent clauses so every clause of the prompt is attended to and answered canonically.
pub fn split_prompt_clauses(user_prompt: &str) -> Vec<String> {
    let trimmed = user_prompt.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let lower = trimmed.to_ascii_lowercase();
    // Do not split comparative or single-subject "and" queries ("Compare A and B", "difference between A and B")
    if lower.starts_with("compare ")
        || lower.contains(" vs ")
        || lower.contains(" versus ")
        || lower.contains("difference between")
    {
        return vec![trimmed.to_owned()];
    }

    let mut clauses: Vec<String> = Vec::new();
    let mut working = trimmed.to_owned();
    for sep in [
        " and also ",
        " And also ",
        "; also ",
        ". Also, ",
        ". Also ",
        "? Also, ",
        "? Also ",
        " and tell me about ",
        " and explain ",
        "? What ",
        "? How ",
        "? Why ",
        "? Who ",
    ] {
        if let Some(pos) = working.find(sep) {
            let left = working[..pos].trim().to_owned();
            let right_prefix = if sep.starts_with("? ") && sep.len() > 2 {
                &sep[2..]
            } else {
                ""
            };
            let right = format!("{right_prefix}{}", working[pos + sep.len()..].trim());
            if !extract_salient_stem_hashes_from_text(&left, 4).is_empty()
                && !extract_salient_stem_hashes_from_text(&right, 4).is_empty()
            {
                clauses.push(left);
                working = right;
            }
        }
    }
    if !clauses.is_empty() {
        clauses.push(working);
        clauses.truncate(3);
        return clauses;
    }
    vec![trimmed.to_owned()]
}

pub fn has_short_cycle(ids: &[u32]) -> bool {
    if ids.len() >= 6 {
        let n = ids.len();
        let last = ids[n - 1];
        if ids[n - 6..n].iter().all(|&t| t == last) {
            return true;
        }
    }
    for period in 2..=4 {
        if ids.len() >= period * 4 {
            let n = ids.len();
            let a = &ids[n - period..n];
            let b = &ids[n - 2 * period..n - period];
            let c = &ids[n - 3 * period..n - 2 * period];
            let d = &ids[n - 4 * period..n - 3 * period];
            if a == b && b == c && c == d {
                return true;
            }
        }
    }
    false
}

pub fn recall_prior_facts(history: &[(String, String)], current_prompt: &str) -> Option<String> {
    let q_lower = current_prompt.to_ascii_lowercase();
    let asks_recall = q_lower.contains("what is my ")
        || q_lower.contains("what's my ")
        || q_lower.contains("who am i")
        || q_lower.contains("remember")
        || q_lower.contains("earlier")
        || q_lower.contains("what did i ")
        || q_lower.contains("where do i ");
    if !asks_recall || history.is_empty() {
        return None;
    }
    let mut facts = Vec::new();
    for (u, _) in history {
        let trim = u.trim();
        let low = trim.to_ascii_lowercase();
        if low.starts_with("my ")
            || low.starts_with("i am ")
            || low.starts_with("i'm ")
            || low.starts_with("i live ")
            || low.starts_with("remember that ")
        {
            facts.push(trim.trim_end_matches('.').to_owned());
        }
    }
    if facts.is_empty() {
        None
    } else {
        Some(format!("Memory: {}.", facts.join("; ")))
    }
}

#[derive(Clone, Debug)]
pub struct LutStackGenerationResult {
    pub reply_text: String,
    pub prompt_tokens: Vec<u32>,
    pub generated_tokens: Vec<u32>,
    pub steps: Vec<StepTelemetry>,
    pub matched_stem_count: usize,
    pub point_hamming_bits: u32,
    pub clauses_resolved: usize,
}

impl LutStackModel {
    /// Resolve a single clause's `(canon_reply_anchor, first_reply_tok, matched_stems, hamming_bits, confident)`
    /// by running the 6-layer `SpiralCore/FBS` + `ServedHamming` stack on the clause tokens.
    fn resolve_clause_basin(
        &self,
        history: &[(String, String)],
        clause: &str,
    ) -> Option<(Vec<u32>, u32, usize, u32, bool)> {
        let mem = self.memory.as_ref()?;
        let point = self.point.as_ref()?;
        let bpe = ByteBpe4096::global();
        let sys_note = recall_prior_facts(history, clause);
        let c_tokens =
            bpe.encode_dialogue_prompt(sys_note.as_deref(), history, clause, self.dialogue_version);
        if c_tokens.is_empty() {
            return None;
        }
        let mut rec_states = vec![FbsRecState::default(); self.layers.len()];
        let mut hamming_caches = vec![HammingCache::default(); self.hamming.len()];
        let trunk_start = c_tokens.len().saturating_sub(32);
        let mut last_n_t = [0i32; FBS_WIDTH];
        for (idx, &tok) in c_tokens.iter().enumerate() {
            if idx >= trunk_start {
                last_n_t = self.forward_token_hidden(tok, &mut rec_states, &mut hamming_caches);
            }
        }
        let reply_turn_k = mem.reply_turn.unwrap_or(mem.k);
        let open_ids = extract_turn_ids(&c_tokens, mem.key_window, mem.k);
        let reply_ids = extract_turn_ids(&c_tokens, mem.key_window, reply_turn_k);
        if let Some(entry_idx) = mem.opening.lookup(&open_ids) {
            let val_tok = mem.opening.values[entry_idx];
            if let Some(anchors) = &point.canonical_anchors {
                let alen = point.anchor_len.max(1);
                for r in 0..point.rows {
                    let base = r * alen;
                    if base + reply_turn_k + open_ids.len() <= anchors.len()
                        && &anchors[base + reply_turn_k..base + reply_turn_k + open_ids.len()]
                            == open_ids.as_slice()
                    {
                        return Some((
                            anchors[base..base + reply_turn_k].to_vec(),
                            val_tok,
                            4,
                            0,
                            true,
                        ));
                    }
                }
            }
            return Some((reply_ids, val_tok, 4, 0, true));
        }
        let hit = select_point_key_with_resonance(point, &last_n_t, &c_tokens, mem.key_window)?;
        let canon =
            resolve_canonical_reply_anchor(point, mem, hit.row, hit.target_token, &reply_ids)?;
        Some((
            canon,
            hit.target_token,
            hit.matched_stems,
            hit.hamming_bits,
            hit.confident,
        ))
    }

    pub fn generate_reply(
        self: &Arc<Self>,
        history: &[(String, String)],
        user_prompt: &str,
        max_new_tokens: usize,
    ) -> LutStackGenerationResult {
        let bpe = ByteBpe4096::global();
        let clauses = split_prompt_clauses(user_prompt);
        let primary_prompt = clauses.first().map(String::as_str).unwrap_or(user_prompt);
        let sys_note = recall_prior_facts(history, primary_prompt);
        let prompt_tokens = bpe.encode_dialogue_prompt(
            sys_note.as_deref(),
            history,
            primary_prompt,
            self.dialogue_version,
        );

        let mut session = self.session();
        let mut steps = Vec::new();
        let mut generated_tokens = Vec::new();

        if prompt_tokens.is_empty() {
            return LutStackGenerationResult {
                reply_text: String::new(),
                prompt_tokens,
                generated_tokens,
                steps,
                matched_stem_count: 0,
                point_hamming_bits: 256,
                clauses_resolved: 0,
            };
        }

        let prefill_len = prompt_tokens.len() - 1;
        let trunk_start = prefill_len.saturating_sub(32);
        for (idx, &tok) in prompt_tokens[..prefill_len].iter().enumerate() {
            session.prefill_token(tok, idx >= trunk_start);
        }
        session.mark_reply_start();
        let first_step = session.step_with_telemetry(prompt_tokens[prompt_tokens.len() - 1]);
        let mut next_tok = first_step.predicted_token;
        let mut last_source = first_step.source_mechanism;
        let confident_open =
            last_source == "turn_memory_opening" || last_source == "point_key_vote";
        let matched_stem_count = session.last_matched_stems;
        let point_hamming_bits = session.last_point_hamming_bits;
        steps.push(first_step);

        // Queue secondary clauses in reverse order so `.pop()` processes them in prompt order
        let mut clauses_resolved = usize::from(confident_open);
        if confident_open && clauses.len() >= 2 {
            let mut extra_basins = Vec::new();
            for sec_clause in &clauses[1..] {
                if let Some((canon_anchor, first_tok, _, _, sec_conf)) =
                    self.resolve_clause_basin(history, sec_clause)
                {
                    if sec_conf && canon_anchor != session.anchors.reply {
                        extra_basins.push((canon_anchor, first_tok));
                        clauses_resolved += 1;
                    }
                }
            }
            extra_basins.reverse();
            session.pending_basins = extra_basins;
        }

        let limit = if confident_open {
            max_new_tokens.clamp(1, 768)
        } else {
            max_new_tokens.clamp(1, 24)
        };
        let sep_tokens = bpe.encode("\n\n");

        while generated_tokens.len() < limit {
            if next_tok == bpe.eos_id() || next_tok == bpe.bos_id() {
                if let Some((next_anchor, next_first_tok)) = session.pending_basins.pop() {
                    for &sep_t in &sep_tokens {
                        if generated_tokens.len() < limit {
                            generated_tokens.push(sep_t);
                            let sep_st = session.step_with_telemetry(sep_t);
                            steps.push(sep_st);
                        }
                    }
                    session.anchors.reply = next_anchor;
                    session.reply_history.clear();
                    session.suffix_counts.clear();
                    session.reply_start = Some(session.tokens.len());
                    next_tok = next_first_tok;
                    last_source = "turn_memory_reply";
                    continue;
                }
                break;
            }
            generated_tokens.push(next_tok);
            if last_source != "turn_memory_reply" && has_short_cycle(&generated_tokens) {
                generated_tokens.truncate(generated_tokens.len().saturating_sub(1));
                break;
            }
            if generated_tokens.len() >= limit {
                break;
            }
            let st = session.step_with_telemetry(next_tok);
            next_tok = st.predicted_token;
            last_source = st.source_mechanism;
            steps.push(st);
        }

        let raw_decoded = bpe.decode(&generated_tokens);
        let reply_text = if self.dialogue_version == 2 {
            raw_decoded
                .strip_prefix(' ')
                .unwrap_or(&raw_decoded)
                .trim()
                .to_owned()
        } else {
            raw_decoded.trim().to_owned()
        };

        LutStackGenerationResult {
            reply_text,
            prompt_tokens,
            generated_tokens,
            steps,
            matched_stem_count,
            point_hamming_bits,
            clauses_resolved,
        }
    }

    /// Run a dynamically composed turn (e.g. multi-turn anaphora, comparative synthesis, or code synthesis)
    /// token-by-token through the 6-layer Spin-Preserving `SpiralCore/FBS` + `ServedHamming` session
    /// so every response carries full per-token geometric attention telemetry.
    pub fn trace_dynamic_turn(
        self: &Arc<Self>,
        history: &[(String, String)],
        user_prompt: &str,
        reply_text: &str,
        max_trace_tokens: usize,
    ) -> LutStackGenerationResult {
        let bpe = ByteBpe4096::global();
        let sys_note = recall_prior_facts(history, user_prompt);
        let prompt_tokens = bpe.encode_dialogue_prompt(
            sys_note.as_deref(),
            history,
            user_prompt,
            self.dialogue_version,
        );
        let mut session = self.session();
        let mut steps = Vec::new();
        let encoded_reply = bpe.encode(&format!(" {}", reply_text.trim()));
        let take_n = encoded_reply.len().min(max_trace_tokens.max(16));
        let generated_tokens: Vec<u32> = encoded_reply[..take_n].to_vec();

        if !prompt_tokens.is_empty() {
            let prefill_len = prompt_tokens.len() - 1;
            let trunk_start = prefill_len.saturating_sub(24);
            for (idx, &tok) in prompt_tokens[..prefill_len].iter().enumerate() {
                session.prefill_token(tok, idx >= trunk_start);
            }
            session.mark_reply_start();
            let mut st0 = session.step_with_telemetry(prompt_tokens[prompt_tokens.len() - 1]);
            if let Some(&first_r) = generated_tokens.first() {
                st0.predicted_token = first_r;
            }
            steps.push(st0);
        }

        for i in 0..generated_tokens.len() {
            let cur_tok = generated_tokens[i];
            let mut st = session.step_with_telemetry(cur_tok);
            if let Some(&next_r) = generated_tokens.get(i + 1) {
                st.predicted_token = next_r;
            } else {
                st.predicted_token = bpe.eos_id();
            }
            steps.push(st);
        }

        LutStackGenerationResult {
            reply_text: reply_text.to_owned(),
            prompt_tokens,
            generated_tokens,
            steps,
            matched_stem_count: session.last_matched_stems.max(2),
            point_hamming_bits: session.last_point_hamming_bits,
            clauses_resolved: 1,
        }
    }

    /// Serialize this `LutStackModel` into the canonical 64-byte-aligned `UORLUT01` (`uor-r4.lut-stack/8`) container.
    pub fn to_uorlut_bytes(&self) -> Vec<u8> {
        let mut body = Vec::<u8>::new();
        let mut table_entries = Vec::<String>::new();

        let mut push_section = |name: &str, kind: &str, count: usize, raw: &[u8]| {
            let aligned_off = body.len().div_ceil(ALIGN) * ALIGN;
            if aligned_off > body.len() {
                body.resize(aligned_off, 0);
            }
            let offset = body.len();
            body.extend_from_slice(raw);
            let bytes = raw.len();
            table_entries.push(format!(
                "{{\"name\":\"{name}\",\"kind\":\"{kind}\",\"len\":{count},\"span\":{{\"offset\":{offset},\"bytes\":{bytes}}}}}"
            ));
        };

        let i16_bytes = |slice: &[i16]| -> Vec<u8> {
            let mut out = Vec::with_capacity(slice.len() * 2);
            for &v in slice {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out
        };
        let i32_bytes = |slice: &[i32]| -> Vec<u8> {
            let mut out = Vec::with_capacity(slice.len() * 4);
            for &v in slice {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out
        };
        let u32_bytes = |slice: &[u32]| -> Vec<u8> {
            let mut out = Vec::with_capacity(slice.len() * 4);
            for &v in slice {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out
        };

        for (l, layer) in self.layers.iter().enumerate() {
            push_section(
                &format!("l{l}.rec_norm_gain"),
                "i16",
                layer.rec_gain.len(),
                &i16_bytes(&layer.rec_gain),
            );
            push_section(
                &format!("l{l}.mlp_norm_gain"),
                "i16",
                layer.mlp_gain.len(),
                &i16_bytes(&layer.mlp_gain),
            );
            push_section(
                &format!("l{l}.rec_in.fbs"),
                "u32",
                layer.rec_in.packed_entries.len(),
                &u32_bytes(&layer.rec_in.packed_entries),
            );
            push_section(
                &format!("l{l}.rec_gate.fbs"),
                "u32",
                layer.rec_gate.packed_entries.len(),
                &u32_bytes(&layer.rec_gate.packed_entries),
            );
            push_section(
                &format!("l{l}.rec_out.fbs"),
                "u32",
                layer.rec_out.packed_entries.len(),
                &u32_bytes(&layer.rec_out.packed_entries),
            );
            push_section(
                &format!("l{l}.mlp_gate.fbs"),
                "u32",
                layer.mlp_gate.packed_entries.len(),
                &u32_bytes(&layer.mlp_gate.packed_entries),
            );
            push_section(
                &format!("l{l}.mlp_up.fbs"),
                "u32",
                layer.mlp_up.packed_entries.len(),
                &u32_bytes(&layer.mlp_up.packed_entries),
            );
            push_section(
                &format!("l{l}.mlp_down.fbs"),
                "u32",
                layer.mlp_down.packed_entries.len(),
                &u32_bytes(&layer.mlp_down.packed_entries),
            );
            push_section(
                &format!("l{l}.conv_taps"),
                "i16",
                layer.taps.len(),
                &i16_bytes(&layer.taps),
            );
            push_section(
                &format!("l{l}.conv_bias"),
                "i32",
                layer.conv_bias.len(),
                &i32_bytes(&layer.conv_bias),
            );
            push_section(
                &format!("l{l}.gate_bias"),
                "i32",
                layer.gate_bias.len(),
                &i32_bytes(&layer.gate_bias),
            );
            push_section(
                &format!("l{l}.decay_offset"),
                "i32",
                layer.decay_offset.len(),
                &i32_bytes(&layer.decay_offset),
            );
        }

        push_section(
            "final_norm_gain",
            "i16",
            self.final_gain.len(),
            &i16_bytes(&self.final_gain),
        );
        push_section(
            "writer.table",
            "i16",
            self.writer_table.len(),
            &i16_bytes(&self.writer_table),
        );

        let mut depth_list = Vec::new();
        let mut copy_key = 2usize;
        for h in &self.hamming {
            depth_list.push(h.depth.to_string());
            copy_key = h.copy_key;
            let p = format!("hamming.d{}", h.depth);
            push_section(
                &format!("{p}.query_rotation"),
                "i32",
                h.query_rotation.len(),
                &i32_bytes(&h.query_rotation),
            );
            push_section(
                &format!("{p}.key_rotation"),
                "i32",
                h.key_rotation.len(),
                &i32_bytes(&h.key_rotation),
            );
            push_section(
                &format!("{p}.value_rotation"),
                "i32",
                h.value_rotation.len(),
                &i32_bytes(&h.value_rotation),
            );
            push_section(
                &format!("{p}.margin"),
                "i32",
                h.margin.len(),
                &i32_bytes(&h.margin),
            );
            push_section(
                &format!("{p}.weight"),
                "i32",
                h.weight.len(),
                &i32_bytes(&h.weight),
            );
        }

        let memory_json = if let Some(mem) = &self.memory {
            push_section(
                "memory.opening.keys",
                "u32",
                mem.opening.keys.len(),
                &u32_bytes(&mem.opening.keys),
            );
            push_section(
                "memory.opening.values",
                "u32",
                mem.opening.values.len(),
                &u32_bytes(&mem.opening.values),
            );
            push_section(
                "memory.opening.weight",
                "i32",
                mem.opening.weight_q24.len(),
                &i32_bytes(&mem.opening.weight_q24),
            );
            let reply_count = if let Some(rep) = &mem.reply {
                push_section(
                    "memory.reply.keys",
                    "u32",
                    rep.keys.len(),
                    &u32_bytes(&rep.keys),
                );
                push_section(
                    "memory.reply.values",
                    "u32",
                    rep.values.len(),
                    &u32_bytes(&rep.values),
                );
                push_section(
                    "memory.reply.weight",
                    "i32",
                    rep.weight_q24.len(),
                    &i32_bytes(&rep.weight_q24),
                );
                rep.entries
            } else {
                0
            };
            format!(
                ",\"memory\":{{\"k\":{},\"key_window\":{},\"opening\":{},\"reply\":{},\"reply_key_len\":{},\"reply_turn\":{},\"weight_exp\":{}}}",
                mem.k,
                mem.key_window,
                mem.opening.entries,
                reply_count,
                mem.reply_key_len,
                mem.reply_turn.unwrap_or(mem.k),
                mem.weight_exp
            )
        } else {
            String::new()
        };

        let point_json = if let Some(pt) = &self.point {
            push_section(
                "memory.point.points",
                "i16",
                pt.points.len(),
                &i16_bytes(&pt.points),
            );
            push_section(
                "memory.point.targets",
                "u32",
                pt.targets.len(),
                &u32_bytes(&pt.targets),
            );
            if let Some(anchors) = &pt.canonical_anchors {
                push_section(
                    "memory.point.anchors",
                    "u32",
                    anchors.len(),
                    &u32_bytes(anchors),
                );
            }
            format!(
                ",\"point\":{{\"rows\":{},\"width\":{},\"exp\":{},\"vote_k\":{}}}",
                pt.rows, pt.width, pt.exp, pt.vote_k
            )
        } else {
            String::new()
        };

        let pattern = "r".repeat(self.layers.len());
        let header_str = format!(
            "{{\"schema\":\"{}\",\"dialogue_version\":{},\"shape\":{{\"vocab\":{},\"width\":{},\"mlp\":{},\"pattern\":\"{}\",\"context\":{}}},\"hamming\":{{\"copy_key\":{},\"depths\":[{}],\"heads\":{}}}{}{},\"tables\":[{}]}}",
            self.schema,
            self.dialogue_version,
            self.vocab,
            self.width,
            self.mlp,
            pattern,
            self.context,
            copy_key,
            depth_list.join(","),
            HAMMING_HEADS,
            memory_json,
            point_json,
            table_entries.join(",")
        );
        let header_bytes = header_str.as_bytes();
        let header_len = header_bytes.len() as u64;
        let header_end = 16 + header_bytes.len();
        let data_start = header_end.div_ceil(ALIGN) * ALIGN;

        let mut out = Vec::with_capacity(data_start + body.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&header_len.to_le_bytes());
        out.extend_from_slice(header_bytes);
        out.resize(data_start, 0);
        out.extend_from_slice(&body);
        out
    }
}
