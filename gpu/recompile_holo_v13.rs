//! Pure-Rust + RTX 4090 CUDA Cholesky-GPTQ Recompiler for `UORR4V13` (`Qwen3-0.6B` & `Qwen3-1.7B`).
//!
//! Usage:
//!   recompile_holo_v13 \
//!     --cache-dir /var/tmp/qwen3_06b_cache \
//!     --hf-repo Qwen/Qwen3-0.6B \
//!     --mirror-url https://raw.githubusercontent.com/Casey-allard/lexlean-r4/weights-v13-qwen3-0.6b \
//!     --out-weights-dir gpu/weights/v13_qwen3_0_6b \
//!     --out-receipt gpu/rtx4090_holo_v13_qwen3_0_6b_receipt.json \
//!     --pieces-dir /var/tmp/v13_pieces_06b

#![allow(dead_code, clippy::all)]

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const UOR_V13_MAGIC: &[u8; 8] = b"UORR4V13";
const UOR_V13_VERSION: u32 = 13;
const UOR_TOK13_MAGIC: &[u8; 8] = b"UORTK13\0";
const PIECE_MAX_BYTES: usize = 1_048_576;
const HOLD_TAIL_BYTES: usize = 65_536;
const GROUP: usize = 32;
const RHT_BLOCK: usize = 32;
const RESIDUAL_EXP: i32 = -18;
const NORM_EXP: i32 = -22;
const ROPE_Q: i32 = 14;
const EXP_STEP_LOG2: i32 = -8;
const EXP_RANGE: i32 = 32;
const SILU_STEP_LOG2: i32 = -8;
const SILU_RANGE_LOG2: i32 = 4;
const SIG_BITS: usize = 512;
const SIG_WORDS: usize = 8;

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
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

// ============================================================================
// Minimal Exact JSON Value Parser (Zero external serde dependency needed here)
// ============================================================================

#[derive(Clone, Debug)]
enum JsonVal {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<JsonVal>),
    Obj(Vec<(String, JsonVal)>),
}

impl JsonVal {
    fn get(&self, key: &str) -> Option<&JsonVal> {
        match self {
            JsonVal::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            JsonVal::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match self {
            JsonVal::Num(n) => Some(*n),
            _ => None,
        }
    }
    fn as_u64(&self) -> Option<u64> {
        self.as_f64().map(|n| n.round() as u64)
    }
    fn as_bool(&self) -> Option<bool> {
        match self {
            JsonVal::Bool(b) => Some(*b),
            _ => None,
        }
    }
    fn as_arr(&self) -> Option<&[JsonVal]> {
        match self {
            JsonVal::Arr(a) => Some(a.as_slice()),
            _ => None,
        }
    }
    fn as_obj(&self) -> Option<&[(String, JsonVal)]> {
        match self {
            JsonVal::Obj(o) => Some(o.as_slice()),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(s: &'a str) -> Self {
        Self { b: s.as_bytes(), pos: 0 }
    }
    fn skip_ws(&mut self) {
        while self.pos < self.b.len() && self.b[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }
    fn parse_str(&mut self) -> Result<String, String> {
        self.skip_ws();
        if self.b.get(self.pos) != Some(&b'"') {
            return Err(format!("expected '\"' at {}", self.pos));
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.b.len() {
            let c = self.b[self.pos];
            self.pos += 1;
            if c == b'"' {
                return Ok(out);
            }
            if c == b'\\' {
                let esc = *self.b.get(self.pos).ok_or("EOF in escape")?;
                self.pos += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let hex = std::str::from_utf8(
                            self.b.get(self.pos..self.pos + 4).ok_or("EOF \\u")?,
                        )
                        .map_err(|_| "bad utf8 in \\u")?;
                        self.pos += 4;
                        let code = u16::from_str_radix(hex, 16).map_err(|_| "bad \\u hex")?;
                        if (0xD800..=0xDBFF).contains(&code)
                            && self.b.get(self.pos..self.pos + 2) == Some(b"\\u")
                        {
                            self.pos += 2;
                            let hex2 = std::str::from_utf8(
                                self.b.get(self.pos..self.pos + 4).ok_or("EOF \\u2")?,
                            )
                            .map_err(|_| "bad utf8 \\u2")?;
                            self.pos += 4;
                            let low = u16::from_str_radix(hex2, 16).map_err(|_| "bad \\u2")?;
                            let cp = 0x10000
                                + (((u32::from(code) - 0xD800) << 10) | (u32::from(low) - 0xDC00));
                            out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                        } else {
                            out.push(char::from_u32(u32::from(code)).unwrap_or('\u{FFFD}'));
                        }
                    }
                    other => out.push(other as char),
                }
            } else if c < 0x80 {
                out.push(c as char);
            } else {
                let start = self.pos - 1;
                while self.pos < self.b.len() && (self.b[self.pos] & 0xC0) == 0x80 {
                    self.pos += 1;
                }
                let s = std::str::from_utf8(&self.b[start..self.pos]).map_err(|_| "bad utf8")?;
                out.push_str(s);
            }
        }
        Err("unterminated string".to_string())
    }
    fn parse_val(&mut self) -> Result<JsonVal, String> {
        self.skip_ws();
        let c = *self.b.get(self.pos).ok_or("unexpected EOF")?;
        match c {
            b'n' => {
                self.pos += 4;
                Ok(JsonVal::Null)
            }
            b't' => {
                self.pos += 4;
                Ok(JsonVal::Bool(true))
            }
            b'f' => {
                self.pos += 5;
                Ok(JsonVal::Bool(false))
            }
            b'"' => Ok(JsonVal::Str(self.parse_str()?)),
            b'[' => {
                self.pos += 1;
                let mut arr = Vec::new();
                loop {
                    self.skip_ws();
                    if self.b.get(self.pos) == Some(&b']') {
                        self.pos += 1;
                        return Ok(JsonVal::Arr(arr));
                    }
                    arr.push(self.parse_val()?);
                    self.skip_ws();
                    if self.b.get(self.pos) == Some(&b',') {
                        self.pos += 1;
                    }
                }
            }
            b'{' => {
                self.pos += 1;
                let mut obj = Vec::new();
                loop {
                    self.skip_ws();
                    if self.b.get(self.pos) == Some(&b'}') {
                        self.pos += 1;
                        return Ok(JsonVal::Obj(obj));
                    }
                    let k = self.parse_str()?;
                    self.skip_ws();
                    if self.b.get(self.pos) == Some(&b':') {
                        self.pos += 1;
                    }
                    let v = self.parse_val()?;
                    obj.push((k, v));
                    self.skip_ws();
                    if self.b.get(self.pos) == Some(&b',') {
                        self.pos += 1;
                    }
                }
            }
            _ => {
                let start = self.pos;
                while self.pos < self.b.len()
                    && (self.b[self.pos].is_ascii_digit()
                        || b"+-.eE".contains(&self.b[self.pos]))
                {
                    self.pos += 1;
                }
                let s = std::str::from_utf8(&self.b[start..self.pos]).map_err(|_| "bad num")?;
                Ok(JsonVal::Num(s.parse::<f64>().map_err(|e| format!("{e} in {s}"))?))
            }
        }
    }
}

// ============================================================================
// Exact Qwen3 Byte-Level BPE Tokenizer (`UORTOK13`)
// ============================================================================

fn gpt2_byte_to_unicode() -> ([char; 256], HashMap<char, u8>) {
    let mut bs: Vec<u16> = (b'!'..=b'~')
        .chain(0xA1..=0xAC)
        .chain(0xAE..=0xFF)
        .map(u16::from)
        .collect();
    let mut cs: Vec<u16> = bs.clone();
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
    for (b, c) in bs.into_iter().zip(cs.into_iter()) {
        let ch = char::from_u32(u32::from(c)).unwrap();
        b2u[b as usize] = ch;
        u2b.insert(ch, b as u8);
    }
    (b2u, u2b)
}

fn qwen3_pretokenize(text: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut idx = 0usize;
    let bytes = text.as_bytes();
    while idx < bytes.len() {
        let rest = &text[idx..];
        let mut chars = rest.char_indices();
        let (_, c0) = chars.next().unwrap();

        // 1. Contractions `(?i:'s|'t|'re|'ve|'m|'ll|'d)`
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

        // 2. `[^\r\n\p{L}\p{N}]?\p{L}+`
        let c0_can_prefix_letter =
            c0 != '\r' && c0 != '\n' && !c0.is_alphabetic() && !c0.is_numeric();
        let mut match_end_2 = None;
        if c0.is_alphabetic() {
            let mut end = c0.len_utf8();
            while let Some((off, ch)) = rest[end..].char_indices().next() {
                if off == 0 && ch.is_alphabetic() {
                    end += ch.len_utf8();
                } else {
                    break;
                }
            }
            match_end_2 = Some(end);
        } else if c0_can_prefix_letter {
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
                    match_end_2 = Some(end);
                }
            }
        }
        if let Some(end) = match_end_2 {
            pieces.push(&rest[..end]);
            idx += end;
            continue;
        }

        // 3. `\p{N}` (single numeric character)
        if c0.is_numeric() {
            let end = c0.len_utf8();
            pieces.push(&rest[..end]);
            idx += end;
            continue;
        }

        // 4. ` ?[^\s\p{L}\p{N}]+[\r\n]*`
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

        // 5. `\s*[\r\n]+`
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
            // 6. `\s+(?!\S)` (if trailing non-ws exists and len > 1 char, keep last ws for next token)
            if end < rest.len() {
                let mut prev_end = 0usize;
                for (off, ch) in rest[..end].char_indices() {
                    prev_end = off;
                    let _ = ch;
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
            // 7. `\s+`
            pieces.push(&rest[..c0.len_utf8()]);
            idx += c0.len_utf8();
            continue;
        }

        pieces.push(&rest[..c0.len_utf8()]);
        idx += c0.len_utf8();
    }
    pieces
}

struct Qwen3Tokenizer {
    vocab_size: usize,
    id_to_token: Vec<Vec<u8>>, // GPT-2 byte-level UTF-8 per ID
    id_to_raw: Vec<Vec<u8>>,   // raw UTF-8/byte sequence per ID
    token_to_id: HashMap<String, u32>,
    pair_merges: HashMap<u64, (u32, u32)>, // (left<<32|right) -> (rank, merged_id)
    merge_triples: Vec<(u32, u32, u32, u32)>,
    added_tokens: Vec<(String, u32, bool)>, // sorted longest first
}

impl Qwen3Tokenizer {
    fn from_json_file(path: &Path, config_vocab_size: usize) -> Result<Self, String> {
        let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let root = JsonParser::new(&raw).parse_val()?;
        let model = root.get("model").ok_or("missing tokenizer.model")?;
        let vocab_obj = model.get("vocab").and_then(|v| v.as_obj()).ok_or("missing model.vocab")?;
        let merges_arr = model.get("merges").and_then(|v| v.as_arr()).ok_or("missing model.merges")?;
        let added_arr = root.get("added_tokens").and_then(|v| v.as_arr()).unwrap_or(&[]);

        let (_b2u, u2b) = gpt2_byte_to_unicode();
        let mut max_id = config_vocab_size;
        let mut id_to_token_map: BTreeMap<usize, String> = BTreeMap::new();
        let mut token_to_id = HashMap::with_capacity(vocab_obj.len() + added_arr.len());

        for (tok, idv) in vocab_obj {
            let id = idv.as_u64().ok_or("bad vocab id")? as usize;
            max_id = max_id.max(id + 1);
            id_to_token_map.insert(id, tok.clone());
            token_to_id.insert(tok.clone(), id as u32);
        }
        let mut added_tokens = Vec::with_capacity(added_arr.len());
        for a in added_arr {
            let content = a.get("content").and_then(|v| v.as_str()).ok_or("added content")?.to_string();
            let id = a.get("id").and_then(|v| v.as_u64()).ok_or("added id")? as u32;
            let special = a.get("special").and_then(|v| v.as_bool()).unwrap_or(false);
            max_id = max_id.max(id as usize + 1);
            added_tokens.push((content, id, special));
        }
        added_tokens.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        let mut id_to_token = vec![Vec::new(); max_id];
        let mut id_to_raw = vec![Vec::new(); max_id];
        for (id, tok) in &id_to_token_map {
            id_to_token[*id] = tok.as_bytes().to_vec();
            let mut dec = Vec::with_capacity(tok.len());
            for ch in tok.chars() {
                if let Some(&b) = u2b.get(&ch) {
                    dec.push(b);
                } else {
                    let mut buf = [0u8; 4];
                    dec.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
            }
            id_to_raw[*id] = dec;
        }
        for (content, id, _) in &added_tokens {
            id_to_raw[*id as usize] = content.as_bytes().to_vec();
            id_to_token[*id as usize] = content.as_bytes().to_vec();
        }

        let mut pair_merges = HashMap::with_capacity(merges_arr.len());
        let mut merge_triples = Vec::with_capacity(merges_arr.len());
        for (rank, item) in merges_arr.iter().enumerate() {
            let (l_str, r_str) = match item {
                JsonVal::Arr(parts) if parts.len() == 2 => (
                    parts[0].as_str().ok_or("bad merge[0]")?,
                    parts[1].as_str().ok_or("bad merge[1]")?,
                ),
                JsonVal::Str(s) => s.split_once(' ').ok_or("bad str merge")?,
                _ => return Err("unexpected merge item".to_string()),
            };
            let lid = *token_to_id.get(l_str).ok_or_else(|| format!("missing left merge {l_str}"))?;
            let rid = *token_to_id.get(r_str).ok_or_else(|| format!("missing right merge {r_str}"))?;
            let mut merged = String::with_capacity(l_str.len() + r_str.len());
            merged.push_str(l_str);
            merged.push_str(r_str);
            let mid = *token_to_id.get(&merged).ok_or_else(|| format!("missing merged {merged}"))?;
            let key = (u64::from(lid) << 32) | u64::from(rid);
            pair_merges.insert(key, (rank as u32, mid));
            merge_triples.push((lid, rid, mid, rank as u32));
        }

        Ok(Self {
            vocab_size: max_id,
            id_to_token,
            id_to_raw,
            token_to_id,
            pair_merges,
            merge_triples,
            added_tokens,
        })
    }

    fn bpe_encode_piece(&self, piece_bytes: &[u8], b2u: &[char; 256], out: &mut Vec<u32>) {
        if piece_bytes.is_empty() {
            return;
        }
        let mut ids: Vec<u32> = Vec::with_capacity(piece_bytes.len());
        let mut tmp = [0u8; 4];
        for &b in piece_bytes {
            let s = b2u[b as usize].encode_utf8(&mut tmp);
            if let Some(&tid) = self.token_to_id.get(s) {
                ids.push(tid);
            }
        }
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
            let target_l = ids[best_pos];
            let target_r = ids[best_pos + 1];
            let mut i = 0usize;
            let mut w = 0usize;
            while i < ids.len() {
                if i + 1 < ids.len() && ids[i] == target_l && ids[i + 1] == target_r {
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

    fn encode(&self, text: &str) -> Vec<u32> {
        let (b2u, _) = gpt2_byte_to_unicode();
        let mut ids = Vec::new();
        let mut pos = 0usize;
        let mut chunk_start = 0usize;
        while pos < text.len() {
            let rest = &text[pos..];
            let matched = self
                .added_tokens
                .iter()
                .find(|(pat, _, _)| rest.starts_with(pat.as_str()));
            if let Some((pat, aid, _)) = matched {
                if chunk_start < pos {
                    for p in qwen3_pretokenize(&text[chunk_start..pos]) {
                        self.bpe_encode_piece(p.as_bytes(), &b2u, &mut ids);
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
            for p in qwen3_pretokenize(&text[chunk_start..]) {
                self.bpe_encode_piece(p.as_bytes(), &b2u, &mut ids);
            }
        }
        ids
    }

    fn decode(&self, ids: &[u32]) -> String {
        let mut raw = Vec::with_capacity(ids.len() * 4);
        for &id in ids {
            if let Some(bytes) = self.id_to_raw.get(id as usize) {
                raw.extend_from_slice(bytes);
            }
        }
        String::from_utf8_lossy(&raw).into_owned()
    }

    fn serialize_uortok13(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(4 * 1024 * 1024);
        buf.extend_from_slice(UOR_TOK13_MAGIC);
        buf.extend_from_slice(&(self.vocab_size as u32).to_le_bytes());
        buf.extend_from_slice(&(self.merge_triples.len() as u32).to_le_bytes());
        buf.extend_from_slice(&(self.added_tokens.len() as u32).to_le_bytes());
        for raw in &self.id_to_raw {
            let len = raw.len().min(u16::MAX as usize) as u16;
            buf.extend_from_slice(&len.to_le_bytes());
            buf.extend_from_slice(&raw[..len as usize]);
        }
        for tok in &self.id_to_token {
            let len = tok.len().min(u16::MAX as usize) as u16;
            buf.extend_from_slice(&len.to_le_bytes());
            buf.extend_from_slice(&tok[..len as usize]);
        }
        for &(l, r, m, rank) in &self.merge_triples {
            buf.extend_from_slice(&l.to_le_bytes());
            buf.extend_from_slice(&r.to_le_bytes());
            buf.extend_from_slice(&m.to_le_bytes());
            buf.extend_from_slice(&rank.to_le_bytes());
        }
        for (content, id, special) in &self.added_tokens {
            let bytes = content.as_bytes();
            let len = bytes.len().min(u16::MAX as usize) as u16;
            buf.extend_from_slice(&id.to_le_bytes());
            buf.push(u8::from(*special));
            buf.extend_from_slice(&len.to_le_bytes());
            buf.extend_from_slice(&bytes[..len as usize]);
        }
        buf
    }
}

// ============================================================================
// Multi-Shard Safetensors Reader (BF16 / F16 / F32 -> F32)
// ============================================================================

struct SafetensorsShard {
    data: Vec<u8>,
    data_start: usize,
    tensors: HashMap<String, (String, Vec<usize>, usize, usize)>,
}

struct SafetensorsStore {
    shards: Vec<SafetensorsShard>,
    by_name: HashMap<String, usize>,
}

#[inline]
fn bf16_to_f32(bits: u16) -> f32 {
    f32::from_bits(u32::from(bits) << 16)
}

#[inline]
fn f16_to_f32(bits: u16) -> f32 {
    let sign = u32::from((bits >> 15) & 1) << 31;
    let exp = u32::from((bits >> 10) & 0x1F);
    let mant = u32::from(bits & 0x3FF);
    if exp == 0 {
        if mant == 0 {
            return f32::from_bits(sign);
        }
        let mut m = mant;
        let mut e = 1i32;
        while (m & 0x400) == 0 {
            m <<= 1;
            e -= 1;
        }
        let new_exp = (127 - 15 + e) as u32;
        return f32::from_bits(sign | (new_exp << 23) | ((m & 0x3FF) << 13));
    }
    if exp == 31 {
        return f32::from_bits(sign | (0xFF << 23) | (mant << 13));
    }
    let new_exp = exp + (127 - 15);
    f32::from_bits(sign | (new_exp << 23) | (mant << 13))
}

impl SafetensorsStore {
    fn open(dir: &Path) -> Result<Self, String> {
        let single = dir.join("model.safetensors");
        let index = dir.join("model.safetensors.index.json");
        let mut files: Vec<PathBuf> = Vec::new();
        if index.exists() {
            let raw = fs::read_to_string(&index).map_err(|e| e.to_string())?;
            let j = JsonParser::new(&raw).parse_val()?;
            let wm = j
                .get("weight_map")
                .and_then(|v| v.as_obj())
                .ok_or("missing weight_map")?;
            let mut seen = BTreeMap::new();
            for (_, fv) in wm {
                if let Some(fname) = fv.as_str() {
                    seen.insert(fname.to_string(), ());
                }
            }
            for fname in seen.keys() {
                files.push(dir.join(fname));
            }
        } else if single.exists() {
            files.push(single);
        } else {
            return Err(format!("no safetensors in {}", dir.display()));
        }

        let mut shards = Vec::with_capacity(files.len());
        let mut by_name = HashMap::new();
        for (s_idx, path) in files.iter().enumerate() {
            let data = fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
            let hdr_len = u64::from_le_bytes(data[..8].try_into().unwrap()) as usize;
            let hdr_str = std::str::from_utf8(&data[8..8 + hdr_len]).map_err(|_| "hdr utf8")?;
            let hdr_val = JsonParser::new(hdr_str).parse_val()?;
            let mut tensors = HashMap::new();
            if let Some(entries) = hdr_val.as_obj() {
                for (name, val) in entries {
                    if name == "__metadata__" {
                        continue;
                    }
                    let dtype = val.get("dtype").and_then(|v| v.as_str()).unwrap_or("BF16").to_string();
                    let shape: Vec<usize> = val
                        .get("shape")
                        .and_then(|v| v.as_arr())
                        .unwrap_or(&[])
                        .iter()
                        .filter_map(|x| x.as_u64().map(|u| u as usize))
                        .collect();
                    let offs = val.get("data_offsets").and_then(|v| v.as_arr()).ok_or("data_offsets")?;
                    let s0 = offs[0].as_u64().unwrap() as usize;
                    let s1 = offs[1].as_u64().unwrap() as usize;
                    tensors.insert(name.clone(), (dtype, shape, s0, s1));
                    by_name.insert(name.clone(), s_idx);
                }
            }
            shards.push(SafetensorsShard {
                data,
                data_start: 8 + hdr_len,
                tensors,
            });
        }
        Ok(Self { shards, by_name })
    }

    fn tensor_f32(&self, name: &str) -> Result<(Vec<usize>, Vec<f32>), String> {
        let &s_idx = self.by_name.get(name).ok_or_else(|| format!("missing tensor {name}"))?;
        let sh = &self.shards[s_idx];
        let (dtype, shape, s0, s1) = sh.tensors.get(name).ok_or("missing tensor meta")?;
        let slice = &sh.data[sh.data_start + *s0..sh.data_start + *s1];
        let mut out = Vec::with_capacity(slice.len() / 2);
        match dtype.as_str() {
            "BF16" => {
                for ch in slice.chunks_exact(2) {
                    out.push(bf16_to_f32(u16::from_le_bytes([ch[0], ch[1]])));
                }
            }
            "F16" => {
                for ch in slice.chunks_exact(2) {
                    out.push(f16_to_f32(u16::from_le_bytes([ch[0], ch[1]])));
                }
            }
            "F32" => {
                for ch in slice.chunks_exact(4) {
                    out.push(f32::from_le_bytes([ch[0], ch[1], ch[2], ch[3]]));
                }
            }
            other => return Err(format!("unsupported dtype {other} for {name}")),
        }
        Ok((shape.clone(), out))
    }
}

// ============================================================================
// Pure-Integer Parity Forward Evaluator (Exact Match to Wasm Holo Runtime)
// ============================================================================

#[inline(always)]
fn xorshift64(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

#[inline(always)]
fn rht_signs_bits(cols: usize, seed: u64) -> Vec<bool> {
    let mut out = Vec::with_capacity(cols);
    let mut s = seed ^ 0x9e37_79b9_7f4a_7c15;
    if s == 0 {
        s = 0x853c_49e6_748f_ea9b;
    }
    for _ in 0..cols {
        s = xorshift64(s);
        out.push((s & 1) == 1);
    }
    out
}

#[inline(always)]
fn int_fwht32_i64(chunk: &mut [i64; 32]) {
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

#[inline(always)]
fn shift_round(v: i64, sh: i32) -> i64 {
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

/// Exact CLZ-seeded 5-step Newton integer sqrt (matches uor-r4 D81, zero bit-loops).
#[inline(always)]
fn clz_newton_isqrt(n: u128) -> u128 {
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

/// Hardware-integer RMSNorm with explicit Q24 per-channel gain vector.
fn int_rms_norm_gain(x: &[i32], gain_q24: &[i32], eps_q48: i64, in_exp: i32, out_exp: i32, out: &mut [i64]) {
    let mut max_abs = 0u32;
    for &v in x {
        max_abs = max_abs.max(v.unsigned_abs());
    }
    let bl = (32 - max_abs.leading_zeros() as i32).max(1);
    let sh = bl - 20;
    let mut sum = 0u128;
    let mut scaled = vec![0i64; x.len()];
    for (dst, &v) in scaled.iter_mut().zip(x) {
        let y = shift_round(i64::from(v), sh);
        *dst = y;
        let sq = (y.abs() as u128).wrapping_mul(y.abs() as u128);
        sum = sum.wrapping_add(sq);
    }
    let mean = sum / (x.len().max(1) as u128);
    let sq_exp = (in_exp + sh) << 1;
    let eps_int = shift_round(eps_q48, sq_exp - (-48)).max(0) as u128;
    let ms = mean.saturating_add(eps_int).max(1);
    let s = clz_newton_isqrt(ms << 68).max(1);
    let r = (1u128 << 96) / s;
    // y * r at exp = (in_exp + sh) + (34 + sq_exp/2 - 96) = -62; then * gain_q24 -> -86 -> shift to out_exp!
    // Check: rms(y) = sqrt(ms) * 2^(in_exp + sh), so (s * 2^-34) * 2^(in_exp+sh) = sqrt(ms)*2^(in_exp+sh).
    // Thus r = 2^96 / s satisfies r * 2^(34 - 96 - in_exp - sh) = 1 / rms(x).
    // Therefore (y * 2^(in_exp+sh)) * (r * 2^(34-96-in_exp-sh)) = y * r * 2^-62! Exact cancellation!!
    let final_shift = (-62 - 24) - out_exp; // negative => right shift `-final_shift = -86 - out_exp` -> positive right shift (86 + out_exp)
    let r_shift = (-final_shift) as u32;
    let round_half = 1i128 << (r_shift - 1);
    for i in 0..x.len() {
        let unit = (i128::from(scaled[i]) * i128::from(r as i128) * i128::from(gain_q24[i]))
            .saturating_add(round_half)
            >> r_shift;
        out[i] = unit.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
    }
}

struct PackedInt8Matrix {
    rows: usize,
    cols: usize,
    rht_seed: u64,
    row_exp: Vec<i8>,
    group_mult: Vec<u16>,
    weights: Vec<i8>, // RHT-32 transformed quantized values in [-127..127]
}

struct PackedInt4Matrix {
    rows: usize,
    cols: usize,
    rht_seed: u64,
    row_exp: Vec<i8>,
    group_mult: Vec<u16>,
    // Packed contiguous half-blocks: for g in 0..cols/32, byte k in 0..16 holds (q[k]+8) | ((q[k+16]+8)<<4)
    nibbles: Vec<u8>,
}

enum DownProjMatrix {
    Int4(PackedInt4Matrix),
    Int8(PackedInt8Matrix),
}

struct HoloLayerV13 {
    in_ln_q24: Vec<i32>,
    post_ln_q24: Vec<i32>,
    q_norm_q24: Vec<i32>,
    k_norm_q24: Vec<i32>,
    q_proj: PackedInt4Matrix,
    k_proj: PackedInt4Matrix,
    v_proj: PackedInt4Matrix,
    o_proj: PackedInt4Matrix,
    gate_proj: PackedInt4Matrix,
    up_proj: PackedInt4Matrix,
    down_proj: DownProjMatrix,
}

struct HoloModelV13 {
    promote_down_int8: bool,
    vocab_size: u32,
    d_model: u32,
    n_layers: u32,
    n_heads: u32,
    n_kv_heads: u32,
    head_dim: u32,
    intermediate_size: u32,
    max_seq: u32,
    bos_token_id: u32,
    eos_token_id: u32,
    eot_token_id: u32,
    think_start_id: u32,
    think_end_id: u32,
    rms_eps_q48: i64,
    score_scale_q30: i64,
    rope_theta_u64: u64,
    rope_cos_q14: Vec<i16>,
    rope_sin_q14: Vec<i16>,
    exp_table: Vec<u32>,
    silu_table: Vec<i32>,
    tok13_bytes: Vec<u8>,
    vocab_sigs: Vec<u64>, // vocab_size * 8 (512-bit RHT sign signatures)
    embed: PackedInt8Matrix,
    final_norm_q24: Vec<i32>,
    layers: Vec<HoloLayerV13>,
}

impl HoloModelV13 {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(420 * 1024 * 1024);
        out.extend_from_slice(UOR_V13_MAGIC);
        out.extend_from_slice(&UOR_V13_VERSION.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&self.vocab_size.to_le_bytes());
        out.extend_from_slice(&self.d_model.to_le_bytes());
        out.extend_from_slice(&self.n_layers.to_le_bytes());
        out.extend_from_slice(&self.n_heads.to_le_bytes());
        out.extend_from_slice(&self.n_kv_heads.to_le_bytes());
        out.extend_from_slice(&self.head_dim.to_le_bytes());
        out.extend_from_slice(&self.intermediate_size.to_le_bytes());
        out.extend_from_slice(&self.max_seq.to_le_bytes());
        out.extend_from_slice(&self.bos_token_id.to_le_bytes());
        out.extend_from_slice(&self.eos_token_id.to_le_bytes());
        out.extend_from_slice(&self.eot_token_id.to_le_bytes());
        out.extend_from_slice(&self.think_start_id.to_le_bytes());
        out.extend_from_slice(&self.think_end_id.to_le_bytes());
        out.extend_from_slice(&self.rms_eps_q48.to_le_bytes());
        out.extend_from_slice(&self.score_scale_q30.to_le_bytes());
        out.extend_from_slice(&self.rope_theta_u64.to_le_bytes());
        let flags = if self.promote_down_int8 { 0b1_1111u32 } else { 0b0_1111u32 };
        out.extend_from_slice(&flags.to_le_bytes()); // flags
        debug_assert_eq!(out.len(), 96);

        for &v in &self.rope_cos_q14 { out.extend_from_slice(&v.to_le_bytes()); }
        for &v in &self.rope_sin_q14 { out.extend_from_slice(&v.to_le_bytes()); }
        for &v in &self.exp_table { out.extend_from_slice(&v.to_le_bytes()); }
        for &v in &self.silu_table { out.extend_from_slice(&v.to_le_bytes()); }

        out.extend_from_slice(&(self.tok13_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.tok13_bytes);
        for &w in &self.vocab_sigs { out.extend_from_slice(&w.to_le_bytes()); }

        // Embed int8 matrix
        out.extend_from_slice(&(self.embed.rows as u32).to_le_bytes());
        out.extend_from_slice(&(self.embed.cols as u32).to_le_bytes());
        out.extend_from_slice(&self.embed.rht_seed.to_le_bytes());
        for &e in &self.embed.row_exp { out.push(e as u8); }
        for &m in &self.embed.group_mult { out.extend_from_slice(&m.to_le_bytes()); }
        for &q in &self.embed.weights { out.push(q as u8); }

        for &v in &self.final_norm_q24 { out.extend_from_slice(&v.to_le_bytes()); }

        let push_i4 = |dst: &mut Vec<u8>, m: &PackedInt4Matrix| {
            dst.extend_from_slice(&(m.rows as u32).to_le_bytes());
            dst.extend_from_slice(&(m.cols as u32).to_le_bytes());
            dst.extend_from_slice(&m.rht_seed.to_le_bytes());
            for &e in &m.row_exp { dst.push(e as u8); }
            for &gm in &m.group_mult { dst.extend_from_slice(&gm.to_le_bytes()); }
            dst.extend_from_slice(&m.nibbles);
        };
        let push_i8 = |dst: &mut Vec<u8>, m: &PackedInt8Matrix| {
            dst.extend_from_slice(&(m.rows as u32).to_le_bytes());
            dst.extend_from_slice(&(m.cols as u32).to_le_bytes());
            dst.extend_from_slice(&m.rht_seed.to_le_bytes());
            for &e in &m.row_exp { dst.push(e as u8); }
            for &gm in &m.group_mult { dst.extend_from_slice(&gm.to_le_bytes()); }
            for &q in &m.weights { dst.push(q as u8); }
        };
        for l in &self.layers {
            for &v in &l.in_ln_q24 { out.extend_from_slice(&v.to_le_bytes()); }
            for &v in &l.post_ln_q24 { out.extend_from_slice(&v.to_le_bytes()); }
            for &v in &l.q_norm_q24 { out.extend_from_slice(&v.to_le_bytes()); }
            for &v in &l.k_norm_q24 { out.extend_from_slice(&v.to_le_bytes()); }
            push_i4(&mut out, &l.q_proj);
            push_i4(&mut out, &l.k_proj);
            push_i4(&mut out, &l.v_proj);
            push_i4(&mut out, &l.o_proj);
            push_i4(&mut out, &l.gate_proj);
            push_i4(&mut out, &l.up_proj);
            match &l.down_proj {
                DownProjMatrix::Int4(m) => push_i4(&mut out, m),
                DownProjMatrix::Int8(m) => push_i8(&mut out, m),
            }
        }
        out
    }
}

fn int_dequant_embed_row(m: &PackedInt8Matrix, row: usize, out: &mut [i32]) {
    let cols = m.cols;
    let groups = cols >> 5;
    let row_exp = i32::from(m.row_exp[row]);
    let w_row = &m.weights[row * cols..(row + 1) * cols];
    let mults = &m.group_mult[row * groups..(row + 1) * groups];
    let mut wide = vec![0i64; cols];
    // Transform back: w_store = (w * signs) * H_32 / 32 -> w_store * H_32 = w * signs at NORM_EXP (-22) before shift!
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

fn prepare_rht_act16(norm_in: &[i64], in_exp: i32, rht_seed: u64, act16: &mut [i16]) -> i32 {
    let cols = norm_in.len();
    let signs = rht_signs_bits(cols, rht_seed);
    let mut rht = vec![0i64; cols];
    for i in 0..cols {
        rht[i] = if signs[i] { norm_in[i] } else { norm_in[i].saturating_neg() };
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
    let sh = (bl - 13).max(0); // max magnitude <= 8191 -> zero i32 inner overflow!
    for i in 0..cols {
        act16[i] = shift_round(rht[i], sh).clamp(-32768, 32767) as i16;
    }
    in_exp + sh
}

fn int_gemv_i4_rht(m: &PackedInt4Matrix, norm_in: &[i64], in_exp: i32, out_exp: i32, out: &mut [i32]) {
    let cols = m.cols;
    let mut act16 = vec![0i16; cols];
    let x_exp = prepare_rht_act16(norm_in, in_exp, m.rht_seed, &mut act16);
    let row_bytes = cols >> 1;
    let groups = cols >> 5;
    for r in 0..m.rows {
        let re = i32::from(m.row_exp[r]);
        let sh = out_exp - (re + x_exp);
        let nibs = &m.nibbles[r * row_bytes..(r + 1) * row_bytes];
        let mults = &m.group_mult[r * groups..(r + 1) * groups];
        let mut acc = 0i128;
        for g in 0..groups {
            let nb = &nibs[g * 16..(g + 1) * 16];
            let a = &act16[g * 32..(g + 1) * 32];
            let mut dot = 0i32;
            for k in 0..16 {
                let byte = nb[k];
                let q0 = i32::from(byte & 15) - 8;
                let q1 = i32::from(byte >> 4) - 8;
                dot += q0 * i32::from(a[k]) + q1 * i32::from(a[k + 16]);
            }
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
        out[r] = val.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32;
    }
}

fn int_gemv_i8_rht(m: &PackedInt8Matrix, norm_in: &[i64], in_exp: i32, out_exp: i32, out: &mut [i32]) {
    let cols = m.cols;
    let mut act16 = vec![0i16; cols];
    let x_exp = prepare_rht_act16(norm_in, in_exp, m.rht_seed, &mut act16);
    let groups = cols >> 5;
    for r in 0..m.rows {
        let re = i32::from(m.row_exp[r]);
        let sh = out_exp - (re + x_exp);
        let ws = &m.weights[r * cols..(r + 1) * cols];
        let mults = &m.group_mult[r * groups..(r + 1) * groups];
        let mut acc = 0i128;
        for g in 0..groups {
            let wg = &ws[g * 32..(g + 1) * 32];
            let ag = &act16[g * 32..(g + 1) * 32];
            let mut dot = 0i32;
            for k in 0..32 {
                dot += i32::from(wg[k]) * i32::from(ag[k]);
            }
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
        out[r] = val.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32;
    }
}

fn int_gemv_i8_subset(
    m: &PackedInt8Matrix,
    norm_in: &[i64],
    in_exp: i32,
    rows_subset: Option<&[usize]>,
    out: &mut [i32],
) {
    let cols = m.cols;
    let mut act16 = vec![0i16; cols];
    let x_exp = prepare_rht_act16(norm_in, in_exp, m.rht_seed, &mut act16);
    let groups = cols >> 5;
    let process_row = |r: usize| -> i32 {
        let re = i32::from(m.row_exp[r]);
        let sh = -16 - (re + x_exp); // output Q16 logits!
        let ws = &m.weights[r * cols..(r + 1) * cols];
        let mults = &m.group_mult[r * groups..(r + 1) * groups];
        let mut acc = 0i128;
        for g in 0..groups {
            let wg = &ws[g * 32..(g + 1) * 32];
            let ag = &act16[g * 32..(g + 1) * 32];
            let mut dot = 0i32;
            for k in 0..32 {
                dot += i32::from(wg[k]) * i32::from(ag[k]);
            }
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
        val.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32
    };
    if let Some(subset) = rows_subset {
        out.fill(i32::MIN / 2);
        for &r in subset {
            out[r] = process_row(r);
        }
    } else {
        for r in 0..m.rows {
            out[r] = process_row(r);
        }
    }
}

struct KvEvalSession<'a> {
    m: &'a HoloModelV13,
    k_caches: Vec<Vec<i32>>,
    v_caches: Vec<Vec<i32>>,
    pos: usize,
}

impl<'a> KvEvalSession<'a> {
    fn new(m: &'a HoloModelV13) -> Self {
        let nl = m.n_layers as usize;
        Self {
            m,
            k_caches: vec![Vec::new(); nl],
            v_caches: vec![Vec::new(); nl],
            pos: 0,
        }
    }

    fn step_token(&mut self, tok: u32, compute_logits: bool) -> Vec<i32> {
        let m = self.m;
        let d = m.d_model as usize;
        let nh = m.n_heads as usize;
        let nkv = m.n_kv_heads as usize;
        let hd = m.head_dim as usize;
        let q_dim = nh * hd;
        let kv_dim = nkv * hd;
        let ffn = m.intermediate_size as usize;
        let group_sz = nh / nkv;
        let nl = m.n_layers as usize;
        let half = hd / 2;
        let pos = self.pos;

        let mut x = vec![0i32; d];
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

        let exp_neg = |diff: i64| -> u64 {
            let frac_bits = EXP_STEP_LOG2 - (-16);
            if diff <= 0 {
                return u64::from(m.exp_table[0]);
            }
            let idx = (diff >> frac_bits) as usize;
            let (Some(&a), Some(&b)) = (m.exp_table.get(idx), m.exp_table.get(idx + 1)) else {
                return 0;
            };
            let frac = (diff & ((1 << frac_bits) - 1)) as u64;
            let step = ((u64::from(a).wrapping_sub(u64::from(b))) * frac) >> frac_bits;
            u64::from(a).wrapping_sub(step)
        };

        let silu_i32 = |xv: i32| -> i32 {
            let frac_bits = SILU_STEP_LOG2 + 18;
            let half_idx = 1i64 << (SILU_RANGE_LOG2 - SILU_STEP_LOG2);
            let idx = (i64::from(xv) >> frac_bits) + half_idx;
            if idx < 0 {
                return 0;
            }
            let u = idx as usize;
            let (Some(&a), Some(&b)) = (m.silu_table.get(u), m.silu_table.get(u + 1)) else {
                return xv >> 2;
            };
            let frac = i64::from(xv) & ((1i64 << frac_bits) - 1);
            let diff = i64::from(b) - i64::from(a);
            (i64::from(a) + ((diff * frac) >> frac_bits)) as i32
        };

        int_dequant_embed_row(&m.embed, tok as usize, &mut x);
        let cos_row = &m.rope_cos_q14[pos * half..(pos + 1) * half];
        let sin_row = &m.rope_sin_q14[pos * half..(pos + 1) * half];

        for l in 0..nl {
            let layer = &m.layers[l];
            int_rms_norm_gain(&x, &layer.in_ln_q24, m.rms_eps_q48, RESIDUAL_EXP, NORM_EXP, &mut norm_d);
            int_gemv_i4_rht(&layer.q_proj, &norm_d, NORM_EXP, RESIDUAL_EXP, &mut q);
            int_gemv_i4_rht(&layer.k_proj, &norm_d, NORM_EXP, RESIDUAL_EXP, &mut k);
            int_gemv_i4_rht(&layer.v_proj, &norm_d, NORM_EXP, RESIDUAL_EXP, &mut v);

            for h in 0..nh {
                let off = h * hd;
                int_rms_norm_gain(&q[off..off + hd], &layer.q_norm_q24, m.rms_eps_q48, RESIDUAL_EXP, RESIDUAL_EXP, &mut q_h_norm);
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
                int_rms_norm_gain(&k[off..off + hd], &layer.k_norm_q24, m.rms_eps_q48, RESIDUAL_EXP, RESIDUAL_EXP, &mut k_h_norm);
                for i in 0..half {
                    let c = i64::from(cos_row[i]);
                    let s = i64::from(sin_row[i]);
                    let k0 = k_h_norm[i];
                    let k1 = k_h_norm[i + half];
                    k[off + i] = ((k0 * c - k1 * s + (1 << 13)) >> ROPE_Q) as i32;
                    k[off + i + half] = ((k0 * s + k1 * c + (1 << 13)) >> ROPE_Q) as i32;
                }
            }
            self.k_caches[l].extend_from_slice(&k);
            self.v_caches[l].extend_from_slice(&v);

            for h in 0..nh {
                let kh = h / group_sz;
                let q_h = &q[h * hd..(h + 1) * hd];
                let mut scores = vec![0i64; pos + 1];
                let mut max_s = i64::MIN;
                for t in 0..=pos {
                    let k_t = &self.k_caches[l][t * kv_dim + kh * hd..t * kv_dim + (kh + 1) * hd];
                    let mut dot = 0i128;
                    for d_i in 0..hd {
                        dot += i128::from(q_h[d_i]) * i128::from(k_t[d_i]);
                    }
                    let s_q16 = ((dot * i128::from(m.score_scale_q30) + (1i128 << 49)) >> 50) as i64;
                    scores[t] = s_q16;
                    if s_q16 > max_s { max_s = s_q16; }
                }
                let mut num = [0i128; 128];
                let mut den = 0u128;
                for t in 0..=pos {
                    let w = (exp_neg(max_s - scores[t]) >> 14) as i128;
                    if w == 0 { continue; }
                    den += w as u128;
                    let v_t = &self.v_caches[l][t * kv_dim + kh * hd..t * kv_dim + (kh + 1) * hd];
                    for d_i in 0..hd { num[d_i] += w * i128::from(v_t[d_i]); }
                }
                let den_s = den.max(1) as i128;
                for d_i in 0..hd {
                    attn_i64[h * hd + d_i] = (num[d_i] / den_s) as i64;
                }
            }
            int_gemv_i4_rht(&layer.o_proj, &attn_i64, RESIDUAL_EXP, RESIDUAL_EXP, &mut proj);
            for i in 0..d { x[i] = x[i].saturating_add(proj[i]); }

            int_rms_norm_gain(&x, &layer.post_ln_q24, m.rms_eps_q48, RESIDUAL_EXP, NORM_EXP, &mut norm_d);
            int_gemv_i4_rht(&layer.gate_proj, &norm_d, NORM_EXP, RESIDUAL_EXP, &mut gate);
            int_gemv_i4_rht(&layer.up_proj, &norm_d, NORM_EXP, RESIDUAL_EXP, &mut up);
            for i in 0..ffn {
                let sg = silu_i32(gate[i]);
                swiglu[i] = (i64::from(sg) * i64::from(up[i]) + (1 << 13)) >> 14;
            }
            match &layer.down_proj {
                DownProjMatrix::Int4(m) => int_gemv_i4_rht(m, &swiglu, -20, RESIDUAL_EXP, &mut proj),
                DownProjMatrix::Int8(m) => int_gemv_i8_rht(m, &swiglu, -20, RESIDUAL_EXP, &mut proj),
            }
            for i in 0..d { x[i] = x[i].saturating_add(proj[i]); }
        }
        self.pos += 1;

        if !compute_logits {
            return Vec::new();
        }
        int_rms_norm_gain(&x, &m.final_norm_q24, m.rms_eps_q48, RESIDUAL_EXP, NORM_EXP, &mut norm_d);
        let mut logits = vec![0i32; m.vocab_size as usize];
        int_gemv_i8_subset(&m.embed, &norm_d, NORM_EXP, None, &mut logits);
        logits
    }
}

// ============================================================================
// CUDA FP32 Teacher + Cholesky Hessian GPTQ Engine
// ============================================================================

const CUDA_QWEN3_SOURCE: &str = r#"
#include <cuda_runtime.h>
#include <cublas_v2.h>
#include <cusolverDn.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <vector>

#define CUDA_CHECK(x) do { cudaError_t e = (x); if (e != cudaSuccess) { fprintf(stderr, "CUDA error %d at %s:%d: %s\n", (int)e, __FILE__, __LINE__, cudaGetErrorString(e)); exit(1); } } while (0)
#define CUBLAS_CHECK(x) do { cublasStatus_t e = (x); if (e != CUBLAS_STATUS_SUCCESS) { fprintf(stderr, "cuBLAS error %d at %s:%d\n", (int)e, __FILE__, __LINE__); exit(1); } } while (0)
#define CUSOLVER_CHECK(x) do { cusolverStatus_t e = (x); if (e != CUSOLVER_STATUS_SUCCESS) { fprintf(stderr, "cuSOLVER error %d at %s:%d\n", (int)e, __FILE__, __LINE__); exit(1); } } while (0)

__global__ void k_embed_gather(const float* embed, const uint32_t* toks, float* out, int N, int d) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= N * d) return;
    int t = i / d, c = i % d;
    out[i] = embed[(size_t)toks[t] * d + c];
}

__global__ void k_rms_norm(const float* x, const float* w, float* out, int rows, int d, float eps) {
    int r = blockIdx.x;
    if (r >= rows) return;
    const float* xr = x + (size_t)r * d;
    float* orow = out + (size_t)r * d;
    __shared__ float ssum[256];
    float acc = 0.0f;
    for (int i = threadIdx.x; i < d; i += blockDim.x) {
        float v = xr[i];
        acc += v * v;
    }
    ssum[threadIdx.x] = acc;
    __syncthreads();
    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (threadIdx.x < s) ssum[threadIdx.x] += ssum[threadIdx.x + s];
        __syncthreads();
    }
    float inv = rsqrtf(ssum[0] / (float)d + eps);
    for (int i = threadIdx.x; i < d; i += blockDim.x) {
        orow[i] = xr[i] * inv * w[i];
    }
}

__global__ void k_neox_rope(float* x, int B, int T, int nh, int hd, float theta) {
    int half = hd >> 1;
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = B * T * nh * half;
    if (idx >= total) return;
    int pair = idx % half;
    int tmp = idx / half;
    int h = tmp % nh;
    tmp /= nh;
    int pos = tmp % T;
    float inv_f = powf(theta, -2.0f * (float)pair / (float)hd);
    float ang = (float)pos * inv_f;
    float c = cosf(ang), s = sinf(ang);
    size_t base = ((size_t)(tmp * nh + h)) * hd + pair;
    float v0 = x[base], v1 = x[base + half];
    x[base] = v0 * c - v1 * s;
    x[base + half] = v0 * s + v1 * c;
}

__global__ void k_causal_gqa(
    const float* q, const float* k, const float* v, float* out,
    const uint32_t* seq_lens, int B, int T, int nh, int nkv, int hd, float scale
) {
    int bt = blockIdx.x;
    int h = blockIdx.y;
    int b = bt / T, pos = bt % T;
    if (pos >= (int)seq_lens[b]) return;
    int kh = h / (nh / nkv);
    const float* qh = q + ((size_t)bt * nh + h) * hd;
    float* oh = out + ((size_t)bt * nh + h) * hd;

    float max_s = -1e30f;
    for (int t = 0; t <= pos; ++t) {
        const float* kt = k + (((size_t)b * T + t) * nkv + kh) * hd;
        float dot = 0.0f;
        for (int i = 0; i < hd; ++i) dot += qh[i] * kt[i];
        dot *= scale;
        if (dot > max_s) max_s = dot;
    }
    float den = 0.0f;
    float acc[128];
    for (int i = 0; i < hd; ++i) acc[i] = 0.0f;
    for (int t = 0; t <= pos; ++t) {
        const float* kt = k + (((size_t)b * T + t) * nkv + kh) * hd;
        const float* vt = v + (((size_t)b * T + t) * nkv + kh) * hd;
        float dot = 0.0f;
        for (int i = 0; i < hd; ++i) dot += qh[i] * kt[i];
        float w = expf(dot * scale - max_s);
        den += w;
        for (int i = 0; i < hd; ++i) acc[i] += w * vt[i];
    }
    float inv = 1.0f / fmaxf(den, 1e-12f);
    for (int i = 0; i < hd; ++i) oh[i] = acc[i] * inv;
}

__global__ void k_swiglu(const float* g, const float* u, float* out, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    float gv = g[i];
    float sig = 1.0f / (1.0f + expf(-gv));
    out[i] = (gv * sig) * u[i];
}

__global__ void k_add(float* dst, const float* src, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) dst[i] += src[i];
}

__device__ __forceinline__ float fwht32_lane(float val, int lane) {
    #pragma unroll
    for (int s = 1; s < 32; s <<= 1) {
        float peer = __shfl_xor_sync(0xffffffffu, val, s);
        val = (lane & s) ? (peer - val) : (val + peer);
    }
    return val;
}

__global__ void k_rht32_inplace(float* x, const uint8_t* signs, int rows, int cols, float mul) {
    int b_idx = blockIdx.x;
    int total_blocks = rows * (cols >> 5);
    if (b_idx >= total_blocks) return;
    int col_blk = (b_idx % (cols >> 5)) << 5;
    int lane = threadIdx.x;
    size_t idx = (size_t)b_idx * 32 + lane;
    float v = x[idx] * (signs[col_blk + lane] ? 1.0f : -1.0f);
    x[idx] = fwht32_lane(v, lane) * mul;
}

__global__ void k_mask_pad_acts(float* acts, const uint32_t* clens, int T, int N, int cols) {
    size_t idx = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= (size_t)N * cols) return;
    int row = (int)(idx / cols);
    int b = row / T;
    int t = row % T;
    if (t >= (int)clens[b]) acts[idx] = 0.0f;
}

__global__ void k_add_damp_diag(float* h, int n, float damp) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) h[(size_t)i * n + i] += damp;
}

__global__ void k_symmetrize_from_lower(float* a, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * n) return;
    int col = idx / n, row = idx % n;
    if (row < col) {
        // cuSolver lower fills row >= col in col-major `a[col * n + row]`; upper is symmetric!
        a[col * n + row] = a[row * n + col];
    }
}

__global__ void k_zero_lower_colmajor(float* a, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * n) return;
    int col = idx / n, row = idx % n;
    if (row > col) a[col * n + row] = 0.0f;
}

// Parallel Cholesky-GPTQ int4 quantization in RHT-32 domain (`w_rht = w * signs * H32 / 32`):
// One thread handles one output row across block-32 groups using upper Cholesky factor U (U^T U = H^-1,
// stored col-major as `u_colmajor[col * cols + row]` for `row <= col`).
__global__ void k_gptq_int4_rows(
    float* w_rht, const float* u_colmajor, int rows, int cols,
    int8_t* row_exp_out, uint16_t* group_mult_out, uint8_t* nibbles_out
) {
    int r = blockIdx.x * blockDim.x + threadIdx.x;
    if (r >= rows) return;
    float* wr = w_rht + (size_t)r * cols;
    int groups = cols >> 5;

    // 1. Choose row_exp with +1 bit headroom so dynamic post-compensation group scales never saturate u16.
    float max_s = 1e-12f;
    for (int g = 0; g < groups; ++g) {
        float gmax = 0.0f;
        for (int c = 0; c < 32; ++c) gmax = fmaxf(gmax, fabsf(wr[g * 32 + c]));
        float s = fmaxf(gmax / 7.0f, 1e-10f);
        if (s > max_s) max_s = s;
    }
    int re = (int)floorf(log2f(max_s)) - 14;
    float base_pow = ldexpf(1.0f, re);
    row_exp_out[r] = (int8_t)re;

    // 2. Column-by-column GPTQ with post-compensation dynamic per-group MSE scale search.
    const float shrink_grid[7] = {0.82f, 0.86f, 0.90f, 0.94f, 0.97f, 1.00f, 1.03f};
    int row_bytes = cols >> 1;
    uint8_t* nib = nibbles_out + (size_t)r * row_bytes;
    int qbuf[32];
    for (int g = 0; g < groups; ++g) {
        int col0 = g * 32;
        float gmax = 0.0f;
        for (int c = 0; c < 32; ++c) gmax = fmaxf(gmax, fabsf(wr[col0 + c]));
        float raw_s = fmaxf(gmax / 7.0f, 1e-10f);
        int best_mult = (int)rintf(raw_s / base_pow);
        if (best_mult < 1) best_mult = 1;
        if (best_mult > 65535) best_mult = 65535;
        float best_err = 1e38f;
        for (int a = 0; a < 7; ++a) {
            int cand_mult = (int)rintf((raw_s * shrink_grid[a]) / base_pow);
            if (cand_mult < 1) cand_mult = 1;
            if (cand_mult > 65535) cand_mult = 65535;
            float cand_sc = (float)cand_mult * base_pow;
            float inv_cand = 1.0f / cand_sc;
            float loss = 0.0f;
            for (int c = 0; c < 32; ++c) {
                int j = col0 + c;
                int q = (int)rintf(wr[j] * inv_cand);
                if (q > 7) q = 7;
                if (q < -8) q = -8;
                float diff = wr[j] - (float)q * cand_sc;
                float u_jj = fmaxf(u_colmajor[(size_t)j * cols + j], 1e-8f);
                loss += (diff * diff) / (u_jj * u_jj);
            }
            if (loss < best_err) {
                best_err = loss;
                best_mult = cand_mult;
            }
        }
        group_mult_out[(size_t)r * groups + g] = (uint16_t)best_mult;
        float sc = (float)best_mult * base_pow;
        float inv_sc = 1.0f / sc;
        for (int c = 0; c < 32; ++c) {
            int j = col0 + c;
            float wv = wr[j];
            int q = (int)rintf(wv * inv_sc);
            if (q > 7) q = 7;
            if (q < -8) q = -8;
            qbuf[c] = q;
            float qw = (float)q * sc;
            float u_jj = u_colmajor[(size_t)j * cols + j];
            float err = (wv - qw) / fmaxf(u_jj, 1e-8f);
            for (int k = j + 1; k < cols; ++k) {
                wr[k] -= err * u_colmajor[(size_t)k * cols + j];
            }
        }
        for (int k = 0; k < 16; ++k) {
            uint8_t lo = (uint8_t)(qbuf[k] + 8);
            uint8_t hi = (uint8_t)(qbuf[k + 16] + 8);
            nib[g * 16 + k] = (lo & 15) | ((hi & 15) << 4);
        }
    }
}

__global__ void k_quant_int8_rht_rows(
    const float* w_rht, int rows, int cols,
    int8_t* row_exp_out, uint16_t* group_mult_out, int8_t* q8_out, uint64_t* sigs_out
) {
    int r = blockIdx.x * blockDim.x + threadIdx.x;
    if (r >= rows) return;
    const float* wr = w_rht + (size_t)r * cols;
    int groups = cols >> 5;
    float max_s = 1e-12f;
    float grp_scales[256];
    for (int g = 0; g < groups; ++g) {
        float gmax = 0.0f;
        for (int c = 0; c < 32; ++c) gmax = fmaxf(gmax, fabsf(wr[g * 32 + c]));
        float s = fmaxf(gmax / 127.0f, 1e-10f);
        grp_scales[g] = s;
        if (s > max_s) max_s = s;
    }
    int re = (int)floorf(log2f(max_s)) - 15;
    float base_pow = ldexpf(1.0f, re);
    row_exp_out[r] = (int8_t)re;
    int8_t* qdst = q8_out + (size_t)r * cols;
    for (int g = 0; g < groups; ++g) {
        int mult = (int)rintf(grp_scales[g] / base_pow);
        if (mult < 1) mult = 1;
        if (mult > 65535) mult = 65535;
        group_mult_out[(size_t)r * groups + g] = (uint16_t)mult;
        float sc = (float)mult * base_pow;
        float inv_sc = 1.0f / sc;
        for (int c = 0; c < 32; ++c) {
            int q = (int)rintf(wr[g * 32 + c] * inv_sc);
            if (q > 127) q = 127;
            if (q < -127) q = -127;
            qdst[g * 32 + c] = (int8_t)q;
        }
    }
    // Strided 512-bit RHT sign signature across full d_model
    if (sigs_out != nullptr) {
        uint64_t* sig = sigs_out + (size_t)r * 8;
        for (int w = 0; w < 8; ++w) sig[w] = 0ULL;
        for (int b = 0; b < 512; ++b) {
            int col = (b * cols) / 512;
            if (wr[col] >= 0.0f) sig[b >> 6] |= (1ULL << (b & 63));
        }
    }
}

static void sgemm_rowmajor(
    cublasHandle_t cb, int m, int n, int k, const float* a, const float* b, float* c, float beta
) {
    // Row-major C[m,n] = alpha * A[m,k] * B[n,k]^T + beta * C[m,n]
    float alpha = 1.0f;
    CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_T, CUBLAS_OP_N, n, m, k, &alpha, b, k, a, k, &beta, c, n));
}

static void compute_rht_hessian_u(
    cublasHandle_t cb, cusolverDnHandle_t cs,
    const float* d_acts_rht, int n_tokens, int cols, float* d_u_out
) {
    // Compute H = (2.0 / N) * X^T * X + damp * I cleanly on GPU (col-major = row-major since symmetric!)
    float* d_h = nullptr;
    CUDA_CHECK(cudaMalloc(&d_h, (size_t)cols * cols * sizeof(float)));
    float scale = 2.0f / (float)n_tokens;
    float zero = 0.0f;
    // Note: in col-major notation, X_rowmajor [N, cols] is X^T [cols, N]. Thus C = X^T * X = A * A^T with OP_N, OP_T!
    CUBLAS_CHECK(cublasSgemm(cb, CUBLAS_OP_N, CUBLAS_OP_T, cols, cols, n_tokens, &scale, d_acts_rht, cols, d_acts_rht, cols, &zero, d_h, cols));
    // Download diagonal to pick stable 1% damping
    std::vector<float> h_mat((size_t)cols * cols);
    CUDA_CHECK(cudaMemcpy(h_mat.data(), d_h, (size_t)cols * cols * sizeof(float), cudaMemcpyDeviceToHost));
    double diag_sum = 0.0;
    for (int i = 0; i < cols; ++i) diag_sum += h_mat[(size_t)i * cols + i];
    float damp = fmaxf(1e-4f, (float)(0.01 * (diag_sum / (double)cols)));
    k_add_damp_diag<<<(cols + 255) / 256, 256>>>(d_h, cols, damp);

    int lwork_potrf = 0, lwork_potri = 0;
    CUSOLVER_CHECK(cusolverDnSpotrf_bufferSize(cs, CUBLAS_FILL_MODE_LOWER, cols, d_h, cols, &lwork_potrf));
    CUSOLVER_CHECK(cusolverDnSpotri_bufferSize(cs, CUBLAS_FILL_MODE_LOWER, cols, d_h, cols, &lwork_potri));
    int lwork = lwork_potrf > lwork_potri ? lwork_potrf : lwork_potri;
    float* d_work = nullptr;
    int* d_info = nullptr;
    CUDA_CHECK(cudaMalloc(&d_work, (size_t)lwork * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_info, sizeof(int)));
    CUSOLVER_CHECK(cusolverDnSpotrf(cs, CUBLAS_FILL_MODE_LOWER, cols, d_h, cols, d_work, lwork, d_info));
    CUSOLVER_CHECK(cusolverDnSpotri(cs, CUBLAS_FILL_MODE_LOWER, cols, d_h, cols, d_work, lwork, d_info));
    k_symmetrize_from_lower<<<(cols * cols + 255) / 256, 256>>>(d_h, cols);

    // Copy H^{-1} -> d_u_out and run UPPER Cholesky (U^T U = H^{-1})
    CUDA_CHECK(cudaMemcpy(d_u_out, d_h, (size_t)cols * cols * sizeof(float), cudaMemcpyDeviceToDevice));
    CUSOLVER_CHECK(cusolverDnSpotrf_bufferSize(cs, CUBLAS_FILL_MODE_UPPER, cols, d_u_out, cols, &lwork_potrf));
    if (lwork_potrf > lwork) {
        cudaFree(d_work);
        CUDA_CHECK(cudaMalloc(&d_work, (size_t)lwork_potrf * sizeof(float)));
        lwork = lwork_potrf;
    }
    CUSOLVER_CHECK(cusolverDnSpotrf(cs, CUBLAS_FILL_MODE_UPPER, cols, d_u_out, cols, d_work, lwork, d_info));
    k_zero_lower_colmajor<<<(cols * cols + 255) / 256, 256>>>(d_u_out, cols);
    int h_info = 0;
    CUDA_CHECK(cudaMemcpy(&h_info, d_info, sizeof(int), cudaMemcpyDeviceToHost));
    if (h_info != 0) {
        fprintf(stderr, "cuSOLVER Cholesky failed with info=%d for cols=%d\n", h_info, cols);
        exit(1);
    }
    cudaFree(d_work);
    cudaFree(d_info);
    cudaFree(d_h);
}

int main(int argc, char** argv) {
    // Subcommand: qwen3_cuda <weights_f32.bin> <calib_seqs.bin> <ood_seqs.bin> <out_quant.bin> <out_eval.bin> [promote_down_int8]
    if (argc < 6) return 2;
    int promote_down_int8 = (argc > 6 && atoi(argv[6]) != 0) ? 1 : 0;
    FILE* fw = fopen(argv[1], "rb");
    uint32_t hdr[8];
    fread(hdr, sizeof(uint32_t), 8, fw);
    int vocab = hdr[0], d = hdr[1], nl = hdr[2], nh = hdr[3], nkv = hdr[4], hd = hdr[5], ffn = hdr[6];
    float rms_eps = 1e-6f, rope_theta = 1e6f;
    fread(&rms_eps, sizeof(float), 1, fw);
    fread(&rope_theta, sizeof(float), 1, fw);
    int q_dim = nh * hd, kv_dim = nkv * hd;

    auto read_vec = [&](size_t len) -> std::vector<float> {
        std::vector<float> v(len);
        size_t got = fread(v.data(), sizeof(float), len, fw);
        if (got != len) { fprintf(stderr, "EOF weights (%zu != %zu)\n", got, len); exit(1); }
        return v;
    };
    auto h_embed = read_vec((size_t)vocab * d);
    auto h_final_norm = read_vec(d);

    cublasHandle_t cb; CUBLAS_CHECK(cublasCreate(&cb));
    cusolverDnHandle_t cs; CUSOLVER_CHECK(cusolverDnCreate(&cs));

    float *d_embed = nullptr, *d_fn = nullptr;
    CUDA_CHECK(cudaMalloc(&d_embed, h_embed.size() * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_fn, d * sizeof(float)));
    CUDA_CHECK(cudaMemcpy(d_embed, h_embed.data(), h_embed.size() * sizeof(float), cudaMemcpyHostToDevice));
    CUDA_CHECK(cudaMemcpy(d_fn, h_final_norm.data(), d * sizeof(float), cudaMemcpyHostToDevice));

    // Read calibration batch `[B, T]` + OOD prompts `[N_ood, T_ood]`
    FILE* fc = fopen(argv[2], "rb");
    uint32_t chdr[2]; fread(chdr, sizeof(uint32_t), 2, fc);
    int B = chdr[0], T = chdr[1], N = B * T;
    std::vector<uint32_t> h_ctoks(N), h_clens(B);
    fread(h_ctoks.data(), sizeof(uint32_t), N, fc);
    fread(h_clens.data(), sizeof(uint32_t), B, fc);
    fclose(fc);

    uint64_t real_tok_sum = 0;
    for (int b = 0; b < B; ++b) real_tok_sum += h_clens[b];
    int n_real_tokens = real_tok_sum > 0 ? (int)real_tok_sum : N;

    uint32_t *d_ctoks = nullptr, *d_clens = nullptr;
    CUDA_CHECK(cudaMalloc(&d_ctoks, N * sizeof(uint32_t)));
    CUDA_CHECK(cudaMalloc(&d_clens, B * sizeof(uint32_t)));
    CUDA_CHECK(cudaMemcpy(d_ctoks, h_ctoks.data(), N * sizeof(uint32_t), cudaMemcpyHostToDevice));
    CUDA_CHECK(cudaMemcpy(d_clens, h_clens.data(), B * sizeof(uint32_t), cudaMemcpyHostToDevice));

    float *d_x = nullptr, *d_n1 = nullptr, *d_q = nullptr, *d_k = nullptr, *d_v = nullptr;
    float *d_attn = nullptr, *d_proj = nullptr, *d_g = nullptr, *d_u = nullptr, *d_swi = nullptr;
    CUDA_CHECK(cudaMalloc(&d_x, (size_t)N * d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_n1, (size_t)N * d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_q, (size_t)N * q_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_k, (size_t)N * kv_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_v, (size_t)N * kv_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_attn, (size_t)N * q_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_proj, (size_t)N * d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_g, (size_t)N * ffn * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_u, (size_t)N * ffn * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_swi, (size_t)N * ffn * sizeof(float)));

    k_embed_gather<<<(N * d + 255) / 256, 256>>>(d_embed, d_ctoks, d_x, N, d);

    FILE* fq = fopen(argv[4], "wb");
    // Read signs generated deterministically in Rust from calib stream! First quantize embed in RHT-32.
    uint32_t qseed_count = 1 + (uint32_t)nl * 7;
    std::vector<uint8_t> h_signs(ffn > q_dim ? ffn : q_dim);

    // Pre-pass calibration signs read from argv[2] (appended to calib_seqs.bin after h_clens!)
    FILE* fcs = fopen(argv[2], "rb");
    fseek(fcs, (2 + N + B) * sizeof(uint32_t), SEEK_SET);
    std::vector<uint8_t> embed_signs(d);
    fread(embed_signs.data(), 1, d, fcs);

    // Quantize embed into int8 RHT-32 on GPU
    {
        uint8_t* d_sg = nullptr; CUDA_CHECK(cudaMalloc(&d_sg, d));
        CUDA_CHECK(cudaMemcpy(d_sg, embed_signs.data(), d, cudaMemcpyHostToDevice));
        float* d_erht = nullptr; CUDA_CHECK(cudaMalloc(&d_erht, (size_t)vocab * d * sizeof(float)));
        CUDA_CHECK(cudaMemcpy(d_erht, d_embed, (size_t)vocab * d * sizeof(float), cudaMemcpyDeviceToDevice));
        k_rht32_inplace<<<(vocab * (d >> 5)), 32>>>(d_erht, d_sg, vocab, d, 1.0f / 32.0f);
        int8_t* d_re = nullptr; uint16_t* d_gm = nullptr; int8_t* d_q8 = nullptr; uint64_t* d_sigs = nullptr;
        CUDA_CHECK(cudaMalloc(&d_re, vocab));
        CUDA_CHECK(cudaMalloc(&d_gm, (size_t)vocab * (d >> 5) * sizeof(uint16_t)));
        CUDA_CHECK(cudaMalloc(&d_q8, (size_t)vocab * d));
        CUDA_CHECK(cudaMalloc(&d_sigs, (size_t)vocab * 8 * sizeof(uint64_t)));
        k_quant_int8_rht_rows<<<(vocab + 255) / 256, 256>>>(d_erht, vocab, d, d_re, d_gm, d_q8, d_sigs);
        std::vector<int8_t> re(vocab); std::vector<uint16_t> gm((size_t)vocab * (d >> 5));
        std::vector<int8_t> q8((size_t)vocab * d); std::vector<uint64_t> sigs((size_t)vocab * 8);
        CUDA_CHECK(cudaMemcpy(re.data(), d_re, vocab, cudaMemcpyDeviceToHost));
        CUDA_CHECK(cudaMemcpy(gm.data(), d_gm, gm.size() * 2, cudaMemcpyDeviceToHost));
        CUDA_CHECK(cudaMemcpy(q8.data(), d_q8, q8.size(), cudaMemcpyDeviceToHost));
        CUDA_CHECK(cudaMemcpy(sigs.data(), d_sigs, sigs.size() * 8, cudaMemcpyDeviceToHost));
        fwrite(re.data(), 1, re.size(), fq);
        fwrite(gm.data(), 2, gm.size(), fq);
        fwrite(q8.data(), 1, q8.size(), fq);
        fwrite(sigs.data(), 8, sigs.size(), fq);
        cudaFree(d_sg); cudaFree(d_erht); cudaFree(d_re); cudaFree(d_gm); cudaFree(d_q8); cudaFree(d_sigs);
    }

    int max_dim = ffn > q_dim ? ffn : q_dim;
    float *d_w = nullptr, *d_ln = nullptr, *d_u_chol = nullptr, *d_acts_tmp = nullptr;
    uint8_t *d_sg = nullptr, *d_nib = nullptr;
    int8_t *d_re = nullptr, *d_q8_layer = nullptr;
    uint16_t *d_gm = nullptr;
    CUDA_CHECK(cudaMalloc(&d_w, (size_t)max_dim * max_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_ln, d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_u_chol, (size_t)max_dim * max_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_acts_tmp, (size_t)N * max_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_sg, max_dim));
    CUDA_CHECK(cudaMalloc(&d_re, max_dim));
    CUDA_CHECK(cudaMalloc(&d_gm, (size_t)max_dim * (max_dim >> 5) * sizeof(uint16_t)));
    CUDA_CHECK(cudaMalloc(&d_nib, (size_t)max_dim * (max_dim >> 1)));
    CUDA_CHECK(cudaMalloc(&d_q8_layer, (size_t)max_dim * max_dim));

    auto quant_int8_rht_layer = [&](const std::vector<float>& w_host, int rows, int cols, uint8_t* sg_host) {
        CUDA_CHECK(cudaMemcpy(d_sg, sg_host, cols, cudaMemcpyHostToDevice));
        CUDA_CHECK(cudaMemcpy(d_w, w_host.data(), (size_t)rows * cols * sizeof(float), cudaMemcpyHostToDevice));
        k_rht32_inplace<<<(rows * (cols >> 5)), 32>>>(d_w, d_sg, rows, cols, 1.0f / 32.0f);
        k_quant_int8_rht_rows<<<(rows + 255) / 256, 256>>>(d_w, rows, cols, d_re, d_gm, d_q8_layer, nullptr);
        std::vector<int8_t> re(rows); std::vector<uint16_t> gm((size_t)rows * (cols >> 5));
        std::vector<int8_t> q8((size_t)rows * cols);
        CUDA_CHECK(cudaMemcpy(re.data(), d_re, rows, cudaMemcpyDeviceToHost));
        CUDA_CHECK(cudaMemcpy(gm.data(), d_gm, gm.size() * 2, cudaMemcpyDeviceToHost));
        CUDA_CHECK(cudaMemcpy(q8.data(), d_q8_layer, q8.size(), cudaMemcpyDeviceToHost));
        fwrite(re.data(), 1, re.size(), fq);
        fwrite(gm.data(), 2, gm.size(), fq);
        fwrite(q8.data(), 1, q8.size(), fq);
    };

    // Keep layer FP32 weights only layer-by-layer so even 1.7B runs inside 24GB VRAM with headroom!
    auto quant_proj_on_acts = [&](const std::vector<float>& w_host, int rows, int cols, const float* d_acts, uint8_t* sg_host) {
        CUDA_CHECK(cudaMemcpy(d_sg, sg_host, cols, cudaMemcpyHostToDevice));
        CUDA_CHECK(cudaMemcpy(d_acts_tmp, d_acts, (size_t)N * cols * sizeof(float), cudaMemcpyDeviceToDevice));
        k_mask_pad_acts<<<((size_t)N * cols + 255) / 256, 256>>>(d_acts_tmp, d_clens, T, N, cols);
        k_rht32_inplace<<<(N * (cols >> 5)), 32>>>(d_acts_tmp, d_sg, N, cols, 1.0f / sqrtf(32.0f));
        compute_rht_hessian_u(cb, cs, d_acts_tmp, n_real_tokens, cols, d_u_chol);

        CUDA_CHECK(cudaMemcpy(d_w, w_host.data(), (size_t)rows * cols * sizeof(float), cudaMemcpyHostToDevice));
        k_rht32_inplace<<<(rows * (cols >> 5)), 32>>>(d_w, d_sg, rows, cols, 1.0f / 32.0f);
        k_gptq_int4_rows<<<(rows + 127) / 128, 128>>>(d_w, d_u_chol, rows, cols, d_re, d_gm, d_nib);

        std::vector<int8_t> re(rows); std::vector<uint16_t> gm((size_t)rows * (cols >> 5));
        std::vector<uint8_t> nib((size_t)rows * (cols >> 1));
        CUDA_CHECK(cudaMemcpy(re.data(), d_re, rows, cudaMemcpyDeviceToHost));
        CUDA_CHECK(cudaMemcpy(gm.data(), d_gm, gm.size() * 2, cudaMemcpyDeviceToHost));
        CUDA_CHECK(cudaMemcpy(nib.data(), d_nib, nib.size(), cudaMemcpyDeviceToHost));
        fwrite(re.data(), 1, re.size(), fq);
        fwrite(gm.data(), 2, gm.size(), fq);
        fwrite(nib.data(), 1, nib.size(), fq);
    };

    float scale = 1.0f / sqrtf((float)hd);
    long layer_file_offsets_start = ftell(fw);

    for (int l = 0; l < nl; ++l) {
        auto h_in_ln = read_vec(d);
        auto h_post_ln = read_vec(d);
        auto h_qn = read_vec(hd);
        auto h_kn = read_vec(hd);
        auto h_wq = read_vec((size_t)q_dim * d);
        auto h_wk = read_vec((size_t)kv_dim * d);
        auto h_wv = read_vec((size_t)kv_dim * d);
        auto h_wo = read_vec((size_t)d * q_dim);
        auto h_wg = read_vec((size_t)ffn * d);
        auto h_wu = read_vec((size_t)ffn * d);
        auto h_wd = read_vec((size_t)d * ffn);

        // 1. RMSNorm1 + quantize Q/K/V against d_n1
        CUDA_CHECK(cudaMemcpy(d_ln, h_in_ln.data(), d * sizeof(float), cudaMemcpyHostToDevice));
        k_rms_norm<<<N, 256>>>(d_x, d_ln, d_n1, N, d, rms_eps);
        for (int p = 0; p < 3; ++p) {
            std::vector<uint8_t> sg(d); fread(sg.data(), 1, d, fcs);
            if (p == 0) quant_proj_on_acts(h_wq, q_dim, d, d_n1, sg.data());
            if (p == 1) quant_proj_on_acts(h_wk, kv_dim, d, d_n1, sg.data());
            if (p == 2) quant_proj_on_acts(h_wv, kv_dim, d, d_n1, sg.data());
        }

        // Teacher FP32 forward through attention + pre-RoPE q_norm / k_norm!
        CUDA_CHECK(cudaMemcpy(d_w, h_wq.data(), h_wq.size() * 4, cudaMemcpyHostToDevice));
        sgemm_rowmajor(cb, N, q_dim, d, d_n1, d_w, d_q, 0.0f);
        CUDA_CHECK(cudaMemcpy(d_w, h_wk.data(), h_wk.size() * 4, cudaMemcpyHostToDevice));
        sgemm_rowmajor(cb, N, kv_dim, d, d_n1, d_w, d_k, 0.0f);
        CUDA_CHECK(cudaMemcpy(d_w, h_wv.data(), h_wv.size() * 4, cudaMemcpyHostToDevice));
        sgemm_rowmajor(cb, N, kv_dim, d, d_n1, d_w, d_v, 0.0f);

        CUDA_CHECK(cudaMemcpy(d_ln, h_qn.data(), hd * 4, cudaMemcpyHostToDevice));
        k_rms_norm<<<N * nh, 128>>>(d_q, d_ln, d_q, N * nh, hd, rms_eps);
        CUDA_CHECK(cudaMemcpy(d_ln, h_kn.data(), hd * 4, cudaMemcpyHostToDevice));
        k_rms_norm<<<N * nkv, 128>>>(d_k, d_ln, d_k, N * nkv, hd, rms_eps);

        k_neox_rope<<<(N * nh * (hd >> 1) + 255) / 256, 256>>>(d_q, B, T, nh, hd, rope_theta);
        k_neox_rope<<<(N * nkv * (hd >> 1) + 255) / 256, 256>>>(d_k, B, T, nkv, hd, rope_theta);
        dim3 grid(N, nh);
        k_causal_gqa<<<grid, 1>>>(d_q, d_k, d_v, d_attn, d_clens, B, T, nh, nkv, hd, scale);

        // 2. Quantize O_proj against d_attn
        {
            std::vector<uint8_t> sg(q_dim); fread(sg.data(), 1, q_dim, fcs);
            quant_proj_on_acts(h_wo, d, q_dim, d_attn, sg.data());
        }
        CUDA_CHECK(cudaMemcpy(d_w, h_wo.data(), h_wo.size() * 4, cudaMemcpyHostToDevice));
        sgemm_rowmajor(cb, N, d, q_dim, d_attn, d_w, d_x, 1.0f);

        // 3. RMSNorm2 + quantize Gate/Up against d_n1
        CUDA_CHECK(cudaMemcpy(d_ln, h_post_ln.data(), d * 4, cudaMemcpyHostToDevice));
        k_rms_norm<<<N, 256>>>(d_x, d_ln, d_n1, N, d, rms_eps);
        for (int p = 0; p < 2; ++p) {
            std::vector<uint8_t> sg(d); fread(sg.data(), 1, d, fcs);
            if (p == 0) quant_proj_on_acts(h_wg, ffn, d, d_n1, sg.data());
            if (p == 1) quant_proj_on_acts(h_wu, ffn, d, d_n1, sg.data());
        }
        CUDA_CHECK(cudaMemcpy(d_w, h_wg.data(), h_wg.size() * 4, cudaMemcpyHostToDevice));
        sgemm_rowmajor(cb, N, ffn, d, d_n1, d_w, d_g, 0.0f);
        CUDA_CHECK(cudaMemcpy(d_w, h_wu.data(), h_wu.size() * 4, cudaMemcpyHostToDevice));
        sgemm_rowmajor(cb, N, ffn, d, d_n1, d_w, d_u, 0.0f);
        k_swiglu<<<(N * ffn + 255) / 256, 256>>>(d_g, d_u, d_swi, N * ffn);

        // 4. Quantize Down_proj against d_swi
        {
            std::vector<uint8_t> sg(ffn); fread(sg.data(), 1, ffn, fcs);
            if (promote_down_int8) {
                quant_int8_rht_layer(h_wd, d, ffn, sg.data());
            } else {
                quant_proj_on_acts(h_wd, d, ffn, d_swi, sg.data());
            }
        }
        CUDA_CHECK(cudaMemcpy(d_w, h_wd.data(), h_wd.size() * 4, cudaMemcpyHostToDevice));
        sgemm_rowmajor(cb, N, d, ffn, d_swi, d_w, d_x, 1.0f);
        fprintf(stderr, "  [cuda] Cholesky-GPTQ quantized layer %d/%d\n", l + 1, nl);
    }
    fclose(fq);
    fclose(fcs);

    // OOD prompt FP32 teacher logits evaluation + greedy 32-token teacher rollouts
    FILE* fo = fopen(argv[3], "rb");
    uint32_t ohdr[3]; fread(ohdr, sizeof(uint32_t), 3, fo);
    int n_ood = ohdr[0], max_tok_ood = ohdr[1];
    uint32_t eos_id = ohdr[2];
    std::vector<uint32_t> ood_lens(n_ood);
    fread(ood_lens.data(), sizeof(uint32_t), n_ood, fo);
    std::vector<uint32_t> ood_toks((size_t)n_ood * max_tok_ood);
    fread(ood_toks.data(), sizeof(uint32_t), ood_toks.size(), fo);
    fclose(fo);

    // Reallocate teacher scratch for batched OOD greedy generation (B_o = n_ood, T_o = max_tok_ood + 32)
    int gen_steps = 32;
    int T_o = max_tok_ood + gen_steps;
    int N_o = n_ood * T_o;
    cudaFree(d_x); cudaFree(d_n1); cudaFree(d_q); cudaFree(d_k); cudaFree(d_v);
    cudaFree(d_attn); cudaFree(d_proj); cudaFree(d_g); cudaFree(d_u); cudaFree(d_swi);
    cudaFree(d_ctoks); cudaFree(d_clens);

    float* d_logits = nullptr;
    CUDA_CHECK(cudaMalloc(&d_x, (size_t)N_o * d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_n1, (size_t)N_o * d * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_q, (size_t)N_o * q_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_k, (size_t)N_o * kv_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_v, (size_t)N_o * kv_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_attn, (size_t)N_o * q_dim * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_g, (size_t)N_o * ffn * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_u, (size_t)N_o * ffn * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_swi, (size_t)N_o * ffn * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_logits, (size_t)n_ood * vocab * sizeof(float)));
    CUDA_CHECK(cudaMalloc(&d_ctoks, N_o * sizeof(uint32_t)));
    CUDA_CHECK(cudaMalloc(&d_clens, n_ood * sizeof(uint32_t)));

    std::vector<uint32_t> cur_toks(N_o, eos_id);
    for (int i = 0; i < n_ood; ++i) {
        memcpy(&cur_toks[i * T_o], &ood_toks[i * max_tok_ood], ood_lens[i] * sizeof(uint32_t));
    }
    std::vector<float> initial_logits((size_t)n_ood * vocab);
    std::vector<uint32_t> teacher_gens((size_t)n_ood * gen_steps, eos_id);

    for (int step = 0; step <= gen_steps; ++step) {
        std::vector<uint32_t> cur_lens(n_ood);
        for (int i = 0; i < n_ood; ++i) cur_lens[i] = ood_lens[i] + step;
        CUDA_CHECK(cudaMemcpy(d_ctoks, cur_toks.data(), N_o * sizeof(uint32_t), cudaMemcpyHostToDevice));
        CUDA_CHECK(cudaMemcpy(d_clens, cur_lens.data(), n_ood * sizeof(uint32_t), cudaMemcpyHostToDevice));
        k_embed_gather<<<(N_o * d + 255) / 256, 256>>>(d_embed, d_ctoks, d_x, N_o, d);

        fseek(fw, layer_file_offsets_start, SEEK_SET);
        for (int l = 0; l < nl; ++l) {
            auto h_in_ln = read_vec(d);
            auto h_post_ln = read_vec(d);
            auto h_qn = read_vec(hd);
            auto h_kn = read_vec(hd);
            auto h_wq = read_vec((size_t)q_dim * d);
            auto h_wk = read_vec((size_t)kv_dim * d);
            auto h_wv = read_vec((size_t)kv_dim * d);
            auto h_wo = read_vec((size_t)d * q_dim);
            auto h_wg = read_vec((size_t)ffn * d);
            auto h_wu = read_vec((size_t)ffn * d);
            auto h_wd = read_vec((size_t)d * ffn);

            CUDA_CHECK(cudaMemcpy(d_ln, h_in_ln.data(), d * 4, cudaMemcpyHostToDevice));
            k_rms_norm<<<N_o, 256>>>(d_x, d_ln, d_n1, N_o, d, rms_eps);
            CUDA_CHECK(cudaMemcpy(d_w, h_wq.data(), h_wq.size() * 4, cudaMemcpyHostToDevice));
            sgemm_rowmajor(cb, N_o, q_dim, d, d_n1, d_w, d_q, 0.0f);
            CUDA_CHECK(cudaMemcpy(d_w, h_wk.data(), h_wk.size() * 4, cudaMemcpyHostToDevice));
            sgemm_rowmajor(cb, N_o, kv_dim, d, d_n1, d_w, d_k, 0.0f);
            CUDA_CHECK(cudaMemcpy(d_w, h_wv.data(), h_wv.size() * 4, cudaMemcpyHostToDevice));
            sgemm_rowmajor(cb, N_o, kv_dim, d, d_n1, d_w, d_v, 0.0f);

            CUDA_CHECK(cudaMemcpy(d_ln, h_qn.data(), hd * 4, cudaMemcpyHostToDevice));
            k_rms_norm<<<N_o * nh, 128>>>(d_q, d_ln, d_q, N_o * nh, hd, rms_eps);
            CUDA_CHECK(cudaMemcpy(d_ln, h_kn.data(), hd * 4, cudaMemcpyHostToDevice));
            k_rms_norm<<<N_o * nkv, 128>>>(d_k, d_ln, d_k, N_o * nkv, hd, rms_eps);

            k_neox_rope<<<(N_o * nh * (hd >> 1) + 255) / 256, 256>>>(d_q, n_ood, T_o, nh, hd, rope_theta);
            k_neox_rope<<<(N_o * nkv * (hd >> 1) + 255) / 256, 256>>>(d_k, n_ood, T_o, nkv, hd, rope_theta);
            dim3 grid(N_o, nh);
            k_causal_gqa<<<grid, 1>>>(d_q, d_k, d_v, d_attn, d_clens, n_ood, T_o, nh, nkv, hd, scale);

            CUDA_CHECK(cudaMemcpy(d_w, h_wo.data(), h_wo.size() * 4, cudaMemcpyHostToDevice));
            sgemm_rowmajor(cb, N_o, d, q_dim, d_attn, d_w, d_x, 1.0f);

            CUDA_CHECK(cudaMemcpy(d_ln, h_post_ln.data(), d * 4, cudaMemcpyHostToDevice));
            k_rms_norm<<<N_o, 256>>>(d_x, d_ln, d_n1, N_o, d, rms_eps);
            CUDA_CHECK(cudaMemcpy(d_w, h_wg.data(), h_wg.size() * 4, cudaMemcpyHostToDevice));
            sgemm_rowmajor(cb, N_o, ffn, d, d_n1, d_w, d_g, 0.0f);
            CUDA_CHECK(cudaMemcpy(d_w, h_wu.data(), h_wu.size() * 4, cudaMemcpyHostToDevice));
            sgemm_rowmajor(cb, N_o, ffn, d, d_n1, d_w, d_u, 0.0f);
            k_swiglu<<<(N_o * ffn + 255) / 256, 256>>>(d_g, d_u, d_swi, N_o * ffn);
            CUDA_CHECK(cudaMemcpy(d_w, h_wd.data(), h_wd.size() * 4, cudaMemcpyHostToDevice));
            sgemm_rowmajor(cb, N_o, d, ffn, d_swi, d_w, d_x, 1.0f);
        }

        // Gather last valid token position for each OOD sequence
        std::vector<float> h_last((size_t)n_ood * d);
        for (int i = 0; i < n_ood; ++i) {
            int last_pos = cur_lens[i] - 1;
            CUDA_CHECK(cudaMemcpy(&h_last[(size_t)i * d], d_x + ((size_t)i * T_o + last_pos) * d, d * sizeof(float), cudaMemcpyDeviceToHost));
        }
        CUDA_CHECK(cudaMemcpy(d_n1, h_last.data(), h_last.size() * 4, cudaMemcpyHostToDevice));
        k_rms_norm<<<n_ood, 256>>>(d_n1, d_fn, d_n1, n_ood, d, rms_eps);
        sgemm_rowmajor(cb, n_ood, vocab, d, d_n1, d_embed, d_logits, 0.0f);

        std::vector<float> step_logits((size_t)n_ood * vocab);
        CUDA_CHECK(cudaMemcpy(step_logits.data(), d_logits, step_logits.size() * 4, cudaMemcpyDeviceToHost));
        if (step == 0) initial_logits = step_logits;
        if (step < gen_steps) {
            for (int i = 0; i < n_ood; ++i) {
                const float* lp = &step_logits[(size_t)i * vocab];
                int best = 0;
                for (int v = 1; v < vocab; ++v) if (lp[v] > lp[best]) best = v;
                teacher_gens[i * gen_steps + step] = (uint32_t)best;
                cur_toks[i * T_o + cur_lens[i]] = (uint32_t)best;
            }
        }
        fprintf(stderr, "  [cuda] teacher rollout step %d/%d\n", step, gen_steps);
    }
    fclose(fw);

    FILE* fe = fopen(argv[5], "wb");
    fwrite(initial_logits.data(), sizeof(float), initial_logits.size(), fe);
    fwrite(teacher_gens.data(), sizeof(uint32_t), teacher_gens.size(), fe);
    fclose(fe);
    return 0;
}
"#;

fn build_luts(max_seq: usize, hd: usize, theta: f64) -> (Vec<i16>, Vec<i16>, Vec<u32>, Vec<i32>) {
    let half = hd / 2;
    let mut cos_q14 = Vec::with_capacity(max_seq * half);
    let mut sin_q14 = Vec::with_capacity(max_seq * half);
    for pos in 0..max_seq {
        for i in 0..half {
            let inv_f = theta.powf(-2.0 * (i as f64) / (hd as f64));
            let ang = (pos as f64) * inv_f;
            cos_q14.push((ang.cos() * 16384.0).round().clamp(-32768.0, 32767.0) as i16);
            sin_q14.push((ang.sin() * 16384.0).round().clamp(-32768.0, 32767.0) as i16);
        }
    }
    let exp_n = (EXP_RANGE << 8) + 1; // 8193
    let mut exp_tbl = Vec::with_capacity(exp_n as usize);
    for i in 0..exp_n {
        let d = (i as f64) * 2.0f64.powi(EXP_STEP_LOG2);
        exp_tbl.push(((-d).exp() * (1u64 << 30) as f64).round().max(0.0) as u32);
    }
    let silu_n = ((SILU_RANGE_LOG2 << 9) << 0) + 1; // -16..16 step 2^-8 -> wait: (32 << 8) + 1 = 8193
    let silu_len = (1usize << (SILU_RANGE_LOG2 - SILU_STEP_LOG2 + 1)) + 1;
    let _ = silu_n;
    let mut silu_tbl = Vec::with_capacity(silu_len);
    let half_s = (silu_len - 1) as i64 / 2;
    for i in 0..silu_len as i64 {
        let x = ((i - half_s) as f64) * 2.0f64.powi(SILU_STEP_LOG2);
        let sig = 1.0 / (1.0 + (-x).exp());
        let val = (x * sig * 65536.0).round().clamp(i32::MIN as f64, i32::MAX as f64) as i32;
        silu_tbl.push(val);
    }
    (cos_q14, sin_q14, exp_tbl, silu_tbl)
}

fn format_qwen3_prompt(user_prompt: &str) -> String {
    format!(
        "<|im_start|>system\nYou are a helpful assistant.<|im_end|>\n<|im_start|>user\n{user_prompt}<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
    )
}

fn calibration_prompts() -> Vec<String> {
    let topics = [
        "Explain how content-addressed storage verifies data integrity using cryptographic SHA-256 digests and deduplicates identical content blocks.",
        "Write an iterative binary search function in Rust over a sorted slice of integers returning Option<usize> cleanly.",
        "What is the architectural difference between L1 cache, L2 shared memory, and tensor cores on an NVIDIA RTX 4090 GPU?",
        "How does the Cholesky decomposition invert a symmetric positive-definite Hessian matrix for weight error compensation?",
        "Why does applying the Walsh-Hadamard transform spread outlier weights evenly across 32 orthogonal coordinates?",
        "Explain how WebAssembly SIMD128 accelerates integer dot products inside a Dedicated Web Worker without SharedArrayBuffer.",
        "Describe how Origin Private File System SyncAccessHandle delivers durable high-speed persistence inside modern browsers.",
        "Write a TypeScript async generator that buffers trailing 64 KiB bytes until a SHA-256 checksum validates.",
        "What is grouped-query attention (GQA) and how does sharing key-value heads cut the KV cache memory footprint?",
        "Why must Qwen3 apply per-head RMSNorm to 128-dim query and key projections before applying half-split NeoX rotary embeddings?",
        "Explain ownership, lifetimes, and borrowing rules in Rust when mutating a struct inside nested helper functions.",
        "How does 2-of-3 XOR parity recovery heal a missing or corrupt binary shard without re-transferring unaffected pieces?",
        "Compare TCP congestion control algorithms CUBIC and BBR under high-latency asymmetric links.",
        "Explain how B+trees preserve logarithmic range query complexity on block-addressed SSD storage.",
        "What is the geometric meaning of eigenvectors and eigenvalues for real symmetric linear operators?",
        "Write a Python shortest-path BFS function using collections.deque over an unweighted adjacency list.",
        "Fix the TypeScript narrowing error when accessing nested optional fields under strictNullChecks.",
        "How do database write-ahead logs ensure crash recovery and ACID durability on power failure?",
        "Explain zero-copy parsing of Safetensors binary headers using explicit byte offsets and dtype metadata.",
        "What HTTP headers control CORS origin allowlists and byte-range partial responses for large static blobs?",
        "How does an event loop schedule microtasks, timers, and Web Worker message channels in JavaScript?",
        "Describe the IEEE 754 bfloat16 exponent and mantissa bit layout and how it converts cleanly to float32.",
        "Write an SQL window query computing 7-day rolling revenue per account ordered by timestamp.",
        "Why do cryptographic constant-time equality checks avoid early returns when comparing HMAC digests?",
        "Explain how virtual memory page tables map guest physical frames to host pages using multi-level radix trees.",
        "How does cosine-annealed learning rate scheduling stabilize late-stage optimization trajectories?",
        "Write a clean C++ templated ring buffer with lock-free single-producer single-consumer atomic indices.",
        "What causes hash table clustering under linear probing and how does Robin Hood hashing reduce variance?",
        "Explain how Merkle DAGs verify immutable directory trees in Git and content-addressed package managers.",
        "Describe the physics of Rayleigh scattering and why Earth's daytime sky appears blue and sunsets red.",
        "How do ATP synthase proton gradients convert chemiosmotic energy across mitochondrial membranes?",
        "Solve the recurrence T(n) = 2T(n/2) + O(n log n) step by step using recursion tree summation.",
    ];
    let mut out = Vec::with_capacity(topics.len() * 6);
    for (i, t) in topics.iter().enumerate() {
        out.push((*t).to_string());
        out.push(format!(
            "Question #{}: Provide a concise, technical explanation and worked example for: {t}",
            i + 1
        ));
        out.push(format!(
            "Write clean, well-structured code or pseudocode demonstrating: {t}"
        ));
        out.push(format!(
            "Compare common failure modes, edge cases, and verification invariants related to: {t}"
        ));
        out.push(format!(
            "Certainly! Let's break down the core intuition, mathematical formulation, and implementation details for: {t}"
        ));
        out.push(format!(
            "Sure! Here is a step-by-step reference answer covering correctness, performance, and memory safety: {t}"
        ));
    }
    out
}

fn ood_prompts() -> &'static [(&'static str, &'static str)] {
    &[
        ("ood_01_wasm", "Explain in two sentences how LexLean-R4 executes neural inference in the browser without floating-point operations."),
        ("ood_02_rust", "Write a short Rust function `gcd(a: u64, b: u64) -> u64` using Euclid's algorithm."),
        ("ood_03_math", "What is the Pythagorean theorem and why does it hold for right triangles?"),
        ("ood_04_opfs", "How does OPFS atomic file writing protect downloaded binary pieces against partial corruption?"),
        ("ood_05_js", "Fix the JavaScript bug where `arr.sort()` sorts numbers `[10, 2, 1]` lexicographically instead of numerically."),
        ("ood_06_rope", "Briefly explain how Rotary Position Embeddings encode relative token distance."),
        ("ood_07_git", "What git command shows only the staged diff before committing?"),
        ("ood_08_http", "What HTTP status code and header instruct a server to return a partial byte slice?"),
        ("ood_09_sql", "Write an SQL query returning the top 3 users by total order amount from `orders(user_id, amount)`."),
        ("ood_10_css", "In CSS Flexbox, how do you center a child element both horizontally and vertically?"),
        ("ood_11_sec", "Why is constant-time byte comparison important when verifying HMAC authentication tags?"),
        ("ood_12_chat", "Name three planets in our Solar System that have prominent ring systems or gas compositions."),
    ]
}

fn kl_and_parity(teacher_logits: &[f32], int_logits_q16: &[i32]) -> (bool, bool, f64, u32, u32) {
    let mut t_best = 0usize;
    let mut i_best = 0usize;
    let mut t_max = f32::NEG_INFINITY;
    for (idx, &v) in teacher_logits.iter().enumerate() {
        if v > t_max {
            t_max = v;
            t_best = idx;
        }
        if int_logits_q16[idx] > int_logits_q16[i_best] {
            i_best = idx;
        }
    }
    let mut indexed_i: Vec<(usize, i32)> = int_logits_q16.iter().copied().enumerate().collect();
    indexed_i.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    let top5 = indexed_i[..5.min(indexed_i.len())].iter().any(|&(id, _)| id == t_best);

    let mut t_probs = vec![0.0f64; teacher_logits.len()];
    let mut t_sum = 0.0f64;
    for (dst, &v) in t_probs.iter_mut().zip(teacher_logits) {
        let e = f64::from(v - t_max).exp();
        *dst = e;
        t_sum += e;
    }
    let q_max = indexed_i[0].1 as f64 / 65536.0;
    let mut q_sum = 0.0f64;
    for &v in int_logits_q16 {
        let qv = (v as f64) / 65536.0;
        q_sum += (qv - q_max).exp();
    }
    let log_t_sum = t_sum.ln();
    let log_q_sum = q_sum.ln();
    let mut kl = 0.0f64;
    for (idx, &p_unnorm) in t_probs.iter().enumerate() {
        let p = p_unnorm / t_sum;
        if p > 1e-12 {
            let log_p = f64::from(teacher_logits[idx] - t_max) - log_t_sum;
            let log_q = ((int_logits_q16[idx] as f64) / 65536.0 - q_max) - log_q_sum;
            kl += p * (log_p - log_q);
        }
    }
    (t_best == i_best, top5, kl.max(0.0), t_best as u32, i_best as u32)
}

fn slice_pieces_and_stitch(
    container: &[u8],
    pieces_dir: &Path,
    manifest_dir: &Path,
    mirror_url: &str,
) -> Result<(String, usize, usize), String> {
    fs::create_dir_all(pieces_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(manifest_dir).map_err(|e| e.to_string())?;
    for dir in [pieces_dir, manifest_dir] {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            if let Ok(e) = entry {
                let n = e.file_name().to_string_lossy().into_owned();
                if n.ends_with(".bin") || n.ends_with(".json") {
                    let _ = fs::remove_file(e.path());
                }
            }
        }
    }

    let c_sha = sha256_hex(container);
    let total = container.len();
    let piece_count = total.div_ceil(PIECE_MAX_BYTES);
    struct Piece {
        index: usize,
        file: String,
        kappa: String,
        byte_offset: usize,
        byte_length: usize,
        data: Vec<u8>,
    }
    let mut pieces = Vec::with_capacity(piece_count);
    for idx in 0..piece_count {
        let s = idx * PIECE_MAX_BYTES;
        let e = (s + PIECE_MAX_BYTES).min(total);
        let data = container[s..e].to_vec();
        let sha = sha256_hex(&data);
        let file = format!("piece_{idx:04}.bin");
        fs::write(pieces_dir.join(&file), &data).map_err(|err| err.to_string())?;
        pieces.push(Piece {
            index: idx,
            file,
            kappa: format!("sha256:{sha}"),
            byte_offset: s,
            byte_length: data.len(),
            data,
        });
    }

    let base_mirror = mirror_url.trim_end_matches('/');
    let mut mj = String::new();
    mj.push_str("{\n");
    mj.push_str("  \"format\": \"UORR4V13\",\n");
    mj.push_str("  \"schema\": \"uor-r4.holo-pieces/1\",\n");
    mj.push_str("  \"version\": 13,\n");
    mj.push_str(&format!("  \"container_sha256\": \"{c_sha}\",\n"));
    mj.push_str(&format!("  \"total_bytes\": {total},\n"));
    mj.push_str(&format!("  \"piece_max_bytes\": {PIECE_MAX_BYTES},\n"));
    mj.push_str(&format!("  \"hold_tail_bytes\": {HOLD_TAIL_BYTES},\n"));
    mj.push_str(&format!("  \"piece_count\": {piece_count},\n"));
    mj.push_str(&format!("  \"mirrors\": [\"{}\"],\n", json_escape(base_mirror)));
    mj.push_str("  \"pieces\": [\n");
    for (i, p) in pieces.iter().enumerate() {
        let comma = if i + 1 < pieces.len() { "," } else { "" };
        mj.push_str(&format!(
            "    {{\"index\": {}, \"file\": \"{}\", \"kappa\": \"{}\", \"sha256\": \"{}\", \"byte_offset\": {}, \"byte_length\": {}}}{}\n",
            p.index, p.file, p.kappa, &p.kappa[7..], p.byte_offset, p.byte_length, comma
        ));
    }
    mj.push_str("  ]\n}\n");
    fs::write(manifest_dir.join("manifest.json"), &mj).map_err(|e| e.to_string())?;
    fs::write(pieces_dir.join("manifest.json"), &mj).map_err(|e| e.to_string())?;

    let group_count = piece_count.div_ceil(2);
    let mut gj = Vec::with_capacity(group_count);
    for g in 0..group_count {
        let left = &pieces[g * 2];
        let right_opt = pieces.get(g * 2 + 1);
        let plen = left.data.len().max(right_opt.map(|r| r.data.len()).unwrap_or(0));
        let mut parity = vec![0u8; plen];
        for i in 0..plen {
            let lb = left.data.get(i).copied().unwrap_or(0);
            let rb = right_opt.and_then(|r| r.data.get(i).copied()).unwrap_or(0);
            parity[i] = lb ^ rb;
        }
        let pfile = format!("parity_{g:04}.bin");
        fs::write(pieces_dir.join(&pfile), &parity).map_err(|e| e.to_string())?;
        let psha = sha256_hex(&parity);
        let rj = match right_opt {
            Some(r) => format!(
                "{{\"index\": {}, \"file\": \"{}\", \"sha256\": \"{}\", \"byte_offset\": {}, \"byte_length\": {}}}",
                r.index, r.file, &r.kappa[7..], r.byte_offset, r.byte_length
            ),
            None => "null".to_string(),
        };
        gj.push(format!(
            "    {{\"group_index\": {g}, \"left\": {{\"index\": {}, \"file\": \"{}\", \"sha256\": \"{}\", \"byte_offset\": {}, \"byte_length\": {}}}, \"right\": {rj}, \"parity\": {{\"index\": {g}, \"file\": \"{pfile}\", \"sha256\": \"{psha}\", \"byte_length\": {plen}}}}}",
            left.index, left.file, &left.kappa[7..], left.byte_offset, left.byte_length
        ));
    }
    let sj = format!(
        "{{\n  \"schema\": \"stitch/xor-2-of-3/1\",\n  \"format\": \"UORR4V13\",\n  \"container_sha256\": \"{c_sha}\",\n  \"total_bytes\": {total},\n  \"piece_count\": {piece_count},\n  \"parity_groups\": {group_count},\n  \"mirrors\": [\"{}\"],\n  \"groups\": [\n{}\n  ]\n}}\n",
        json_escape(base_mirror),
        gj.join(",\n")
    );
    fs::write(manifest_dir.join("manifest.stitch.json"), &sj).map_err(|e| e.to_string())?;
    fs::write(pieces_dir.join("manifest.stitch.json"), &sj).map_err(|e| e.to_string())?;
    Ok((c_sha, piece_count, group_count))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let arg_val = |flag: &str| -> Option<String> {
        args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
    };
    let cache_dir = PathBuf::from(arg_val("--cache-dir").unwrap_or_else(|| "/var/tmp/qwen3_06b_cache".into()));
    let hf_repo = arg_val("--hf-repo").unwrap_or_else(|| "Qwen/Qwen3-0.6B".into());
    let mirror_url = arg_val("--mirror-url").unwrap_or_else(|| "https://raw.githubusercontent.com/Casey-allard/lexlean-r4/weights-v13-qwen3-0.6b".into());
    let out_weights = PathBuf::from(arg_val("--out-weights-dir").unwrap_or_else(|| "gpu/weights/v13_qwen3_0_6b".into()));
    let out_receipt = PathBuf::from(arg_val("--out-receipt").unwrap_or_else(|| "gpu/rtx4090_holo_v13_qwen3_0_6b_receipt.json".into()));
    let pieces_dir = PathBuf::from(arg_val("--pieces-dir").unwrap_or_else(|| "/var/tmp/v13_pieces_06b".into()));
    let promote_down_int8 = args.iter().any(|a| a == "--promote-down-int8");

    let cfg_raw = fs::read_to_string(cache_dir.join("config.json")).expect("read config.json");
    let cfg = JsonParser::new(&cfg_raw).parse_val().expect("parse config.json");
    let vocab_size = cfg.get("vocab_size").and_then(|v| v.as_u64()).expect("vocab_size") as usize;
    let d_model = cfg.get("hidden_size").and_then(|v| v.as_u64()).expect("hidden_size") as usize;
    let n_layers = cfg.get("num_hidden_layers").and_then(|v| v.as_u64()).expect("num_hidden_layers") as usize;
    let n_heads = cfg.get("num_attention_heads").and_then(|v| v.as_u64()).expect("num_attention_heads") as usize;
    let n_kv_heads = cfg.get("num_key_value_heads").and_then(|v| v.as_u64()).expect("num_key_value_heads") as usize;
    let head_dim = cfg.get("head_dim").and_then(|v| v.as_u64()).unwrap_or((d_model / n_heads) as u64) as usize;
    let intermediate_size = cfg.get("intermediate_size").and_then(|v| v.as_u64()).expect("intermediate_size") as usize;
    let bos_id = cfg.get("bos_token_id").and_then(|v| v.as_u64()).unwrap_or(151643) as u32;
    let eos_id = cfg.get("eos_token_id").and_then(|v| v.as_u64()).unwrap_or(151645) as u32;
    let rms_eps = cfg.get("rms_norm_eps").and_then(|v| v.as_f64()).unwrap_or(1e-6);
    let rope_theta = cfg.get("rope_theta").and_then(|v| v.as_f64()).unwrap_or(1_000_000.0);
    let q_dim = n_heads * head_dim;
    let kv_dim = n_kv_heads * head_dim;

    eprintln!("[v13-recompiler] Model={hf_repo} {n_layers}L hidden={d_model} q_heads={n_heads} kv_heads={n_kv_heads} hd={head_dim} ffn={intermediate_size} vocab={vocab_size}");

    let tokenizer = Qwen3Tokenizer::from_json_file(&cache_dir.join("tokenizer.json"), vocab_size)
        .expect("parse Qwen3 tokenizer");
    let st = SafetensorsStore::open(&cache_dir).expect("open safetensors");

    let cu_path = cache_dir.join("qwen3_cuda.cu");
    let bin_path = cache_dir.join("qwen3_cuda");
    fs::write(&cu_path, CUDA_QWEN3_SOURCE).expect("write qwen3_cuda.cu");
    let nvcc_st = Command::new("nvcc")
        .args(["-O3", "-arch=sm_89"])
        .arg(&cu_path)
        .args(["-lcublas", "-lcusolver", "-o"])
        .arg(&bin_path)
        .status()
        .expect("run nvcc");
    assert!(nvcc_st.success(), "nvcc compile failed");

    // Export teacher F32 stream + keep norm gains for UORR4V13
    let weights_bin = cache_dir.join("teacher_f32.bin");
    let (_, final_norm_f32) = st.tensor_f32("model.norm.weight").expect("final norm");
    let final_norm_q24: Vec<i32> = final_norm_f32.iter().map(|&v| (f64::from(v) * (1u64 << 24) as f64).round() as i32).collect();
    let mut layer_norms: Vec<(Vec<i32>, Vec<i32>, Vec<i32>, Vec<i32>)> = Vec::with_capacity(n_layers);
    {
        let mut fw = BufWriter::new(fs::File::create(&weights_bin).expect("create teacher_f32.bin"));
        for v in [vocab_size as u32, d_model as u32, n_layers as u32, n_heads as u32, n_kv_heads as u32, head_dim as u32, intermediate_size as u32, 0u32] {
            fw.write_all(&v.to_le_bytes()).unwrap();
        }
        fw.write_all(&(rms_eps as f32).to_le_bytes()).unwrap();
        fw.write_all(&(rope_theta as f32).to_le_bytes()).unwrap();
        let (_, emb) = st.tensor_f32("model.embed_tokens.weight").expect("embed");
        for &v in &emb { fw.write_all(&v.to_le_bytes()).unwrap(); }
        for &v in &final_norm_f32 { fw.write_all(&v.to_le_bytes()).unwrap(); }
        for l in 0..n_layers {
            let p = format!("model.layers.{l}");
            let (_, in_ln) = st.tensor_f32(&format!("{p}.input_layernorm.weight")).unwrap();
            let (_, post_ln) = st.tensor_f32(&format!("{p}.post_attention_layernorm.weight")).unwrap();
            let (_, qn) = st.tensor_f32(&format!("{p}.self_attn.q_norm.weight")).unwrap();
            let (_, kn) = st.tensor_f32(&format!("{p}.self_attn.k_norm.weight")).unwrap();
            let q24 = |v: &[f32]| -> Vec<i32> {
                v.iter().map(|&x| (f64::from(x) * (1u64 << 24) as f64).round() as i32).collect()
            };
            layer_norms.push((q24(&in_ln), q24(&post_ln), q24(&qn), q24(&kn)));
            for vals in [in_ln, post_ln, qn, kn] {
                for v in vals { fw.write_all(&v.to_le_bytes()).unwrap(); }
            }
            for suffix in [
                "self_attn.q_proj.weight", "self_attn.k_proj.weight", "self_attn.v_proj.weight",
                "self_attn.o_proj.weight", "mlp.gate_proj.weight", "mlp.up_proj.weight", "mlp.down_proj.weight",
            ] {
                let (_, wv) = st.tensor_f32(&format!("{p}.{suffix}")).unwrap();
                for v in wv { fw.write_all(&v.to_le_bytes()).unwrap(); }
            }
        }
        fw.flush().unwrap();
    }

    // Build calibration batch + deterministic RHT seeds/signs
    let calib_seqs: Vec<Vec<u32>> = calibration_prompts()
        .iter()
        .map(|p| tokenizer.encode(&format_qwen3_prompt(p)))
        .collect();
    let cb_len = calib_seqs.len();
    let max_ct = calib_seqs.iter().map(|s| s.len()).max().unwrap_or(64);
    let embed_rht_seed = 0x517cc1b7_27220a95u64;
    let proj_seed = |l: usize, tag: u64| -> u64 {
        0x9e37_79b9_7f4a_7c15u64 ^ ((l as u64 + 1) << 16) ^ tag
    };

    let calib_bin = cache_dir.join("calib_seqs.bin");
    {
        let mut fc = BufWriter::new(fs::File::create(&calib_bin).unwrap());
        fc.write_all(&(cb_len as u32).to_le_bytes()).unwrap();
        fc.write_all(&(max_ct as u32).to_le_bytes()).unwrap();
        for seq in &calib_seqs {
            for t in 0..max_ct {
                let v = *seq.get(t).unwrap_or(&eos_id);
                fc.write_all(&v.to_le_bytes()).unwrap();
            }
        }
        for seq in &calib_seqs {
            fc.write_all(&(seq.len() as u32).to_le_bytes()).unwrap();
        }
        let write_signs = |w: &mut BufWriter<fs::File>, cols: usize, seed: u64| {
            let bits: Vec<u8> = rht_signs_bits(cols, seed).into_iter().map(u8::from).collect();
            w.write_all(&bits).unwrap();
        };
        write_signs(&mut fc, d_model, embed_rht_seed);
        for l in 0..n_layers {
            write_signs(&mut fc, d_model, proj_seed(l, 1));
            write_signs(&mut fc, d_model, proj_seed(l, 2));
            write_signs(&mut fc, d_model, proj_seed(l, 3));
            write_signs(&mut fc, q_dim, proj_seed(l, 4));
            write_signs(&mut fc, d_model, proj_seed(l, 5));
            write_signs(&mut fc, d_model, proj_seed(l, 6));
            write_signs(&mut fc, intermediate_size, proj_seed(l, 7));
        }
        fc.flush().unwrap();
    }

    // Build 12-prompt OOD sequences
    let ood_list = ood_prompts();
    let ood_toks: Vec<Vec<u32>> = ood_list
        .iter()
        .map(|(_, p)| tokenizer.encode(&format_qwen3_prompt(p)))
        .collect();
    let max_ot = ood_toks.iter().map(|s| s.len()).max().unwrap_or(64);
    let ood_bin = cache_dir.join("ood_seqs.bin");
    {
        let mut fo = BufWriter::new(fs::File::create(&ood_bin).unwrap());
        fo.write_all(&(ood_list.len() as u32).to_le_bytes()).unwrap();
        fo.write_all(&(max_ot as u32).to_le_bytes()).unwrap();
        fo.write_all(&eos_id.to_le_bytes()).unwrap();
        for s in &ood_toks {
            fo.write_all(&(s.len() as u32).to_le_bytes()).unwrap();
        }
        for s in &ood_toks {
            for t in 0..max_ot {
                fo.write_all(&(*s.get(t).unwrap_or(&eos_id)).to_le_bytes()).unwrap();
            }
        }
        fo.flush().unwrap();
    }

    let quant_bin = cache_dir.join("quant_gptq.bin");
    let eval_bin = cache_dir.join("eval_teacher.bin");
    eprintln!("[v13-recompiler] Running RTX 4090 CUDA Cholesky-GPTQ + FP32 teacher rollouts (promote_down_int8={promote_down_int8})...");
    let promo_arg = if promote_down_int8 { "1" } else { "0" };
    let cu_st = Command::new(&bin_path)
        .args([
            weights_bin.to_str().unwrap(),
            calib_bin.to_str().unwrap(),
            ood_bin.to_str().unwrap(),
            quant_bin.to_str().unwrap(),
            eval_bin.to_str().unwrap(),
            promo_arg,
        ])
        .status()
        .expect("run qwen3_cuda");
    assert!(cu_st.success(), "qwen3_cuda failed");

    // Read quantized GPU output into HoloModelV13
    let qb = fs::read(&quant_bin).expect("read quant_gptq.bin");
    let mut qcur = 0usize;
    let take_i8 = |c: &mut usize, n: usize| -> Vec<i8> {
        let res = qb[*c..*c + n].iter().map(|&b| b as i8).collect();
        *c += n;
        res
    };
    let take_u8 = |c: &mut usize, n: usize| -> Vec<u8> {
        let res = qb[*c..*c + n].to_vec();
        *c += n;
        res
    };
    let take_u16 = |c: &mut usize, n: usize| -> Vec<u16> {
        let res = qb[*c..*c + n * 2].chunks_exact(2).map(|x| u16::from_le_bytes([x[0], x[1]])).collect();
        *c += n * 2;
        res
    };
    let take_u64 = |c: &mut usize, n: usize| -> Vec<u64> {
        let res = qb[*c..*c + n * 8].chunks_exact(8).map(|x| u64::from_le_bytes(x.try_into().unwrap())).collect();
        *c += n * 8;
        res
    };

    let e_re = take_i8(&mut qcur, vocab_size);
    let e_gm = take_u16(&mut qcur, vocab_size * (d_model >> 5));
    let e_w = take_i8(&mut qcur, vocab_size * d_model);
    let vocab_sigs = take_u64(&mut qcur, vocab_size * SIG_WORDS);
    let embed = PackedInt8Matrix {
        rows: vocab_size,
        cols: d_model,
        rht_seed: embed_rht_seed,
        row_exp: e_re,
        group_mult: e_gm,
        weights: e_w,
    };

    let take_i4 = |c: &mut usize, rows: usize, cols: usize, seed: u64| -> PackedInt4Matrix {
        let row_exp = take_i8(c, rows);
        let group_mult = take_u16(c, rows * (cols >> 5));
        let nibbles = take_u8(c, rows * (cols >> 1));
        PackedInt4Matrix { rows, cols, rht_seed: seed, row_exp, group_mult, nibbles }
    };
    let take_i8_mat = |c: &mut usize, rows: usize, cols: usize, seed: u64| -> PackedInt8Matrix {
        let row_exp = take_i8(c, rows);
        let group_mult = take_u16(c, rows * (cols >> 5));
        let weights = take_i8(c, rows * cols);
        PackedInt8Matrix { rows, cols, rht_seed: seed, row_exp, group_mult, weights }
    };

    let mut layers = Vec::with_capacity(n_layers);
    for l in 0..n_layers {
        let (in_ln_q24, post_ln_q24, q_norm_q24, k_norm_q24) = layer_norms[l].clone();
        layers.push(HoloLayerV13 {
            in_ln_q24,
            post_ln_q24,
            q_norm_q24,
            k_norm_q24,
            q_proj: take_i4(&mut qcur, q_dim, d_model, proj_seed(l, 1)),
            k_proj: take_i4(&mut qcur, kv_dim, d_model, proj_seed(l, 2)),
            v_proj: take_i4(&mut qcur, kv_dim, d_model, proj_seed(l, 3)),
            o_proj: take_i4(&mut qcur, d_model, q_dim, proj_seed(l, 4)),
            gate_proj: take_i4(&mut qcur, intermediate_size, d_model, proj_seed(l, 5)),
            up_proj: take_i4(&mut qcur, intermediate_size, d_model, proj_seed(l, 6)),
            down_proj: if promote_down_int8 {
                DownProjMatrix::Int8(take_i8_mat(&mut qcur, d_model, intermediate_size, proj_seed(l, 7)))
            } else {
                DownProjMatrix::Int4(take_i4(&mut qcur, d_model, intermediate_size, proj_seed(l, 7)))
            },
        });
    }

    let max_seq = 2048usize;
    let (rope_cos_q14, rope_sin_q14, exp_table, silu_table) = build_luts(max_seq, head_dim, rope_theta);
    let model = HoloModelV13 {
        promote_down_int8,
        vocab_size: vocab_size as u32,
        d_model: d_model as u32,
        n_layers: n_layers as u32,
        n_heads: n_heads as u32,
        n_kv_heads: n_kv_heads as u32,
        head_dim: head_dim as u32,
        intermediate_size: intermediate_size as u32,
        max_seq: max_seq as u32,
        bos_token_id: bos_id,
        eos_token_id: eos_id,
        eot_token_id: bos_id,
        think_start_id: *tokenizer.token_to_id.get("<think>").unwrap_or(&151667),
        think_end_id: *tokenizer.token_to_id.get("</think>").unwrap_or(&151668),
        rms_eps_q48: (rms_eps * 2.0f64.powi(48)).round() as i64,
        score_scale_q30: (2.0f64.powi(30) / (head_dim as f64).sqrt()).round() as i64,
        rope_theta_u64: rope_theta.round() as u64,
        rope_cos_q14,
        rope_sin_q14,
        exp_table,
        silu_table,
        tok13_bytes: tokenizer.serialize_uortok13(),
        vocab_sigs,
        embed,
        final_norm_q24,
        layers,
    };

    // Pure-integer evaluation vs RTX 4090 FP32 teacher
    let eval_raw = fs::read(&eval_bin).expect("read eval_teacher.bin");
    let t_logits_all: Vec<f32> = eval_raw[..ood_list.len() * vocab_size * 4]
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let t_gens_all: Vec<u32> = eval_raw[ood_list.len() * vocab_size * 4..]
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    let mut top1_hits = 0usize;
    let mut top5_hits = 0usize;
    let mut sum_kl = 0.0f64;
    let mut ood_records = Vec::with_capacity(ood_list.len());

    for (idx, ((pid, prompt_txt), prompt_ids)) in ood_list.iter().zip(ood_toks.iter()).enumerate() {
        let mut sess = KvEvalSession::new(&model);
        let mut int_logits = Vec::new();
        for (ti, &tid) in prompt_ids.iter().enumerate() {
            int_logits = sess.step_token(tid, ti + 1 == prompt_ids.len());
        }
        let t_slice = &t_logits_all[idx * vocab_size..(idx + 1) * vocab_size];
        let (m1, m5, kl, t_top, q_top) = kl_and_parity(t_slice, &int_logits);
        if m1 { top1_hits += 1; }
        if m5 { top5_hits += 1; }
        sum_kl += kl;

        let t_gen_ids: Vec<u32> = t_gens_all[idx * 32..(idx + 1) * 32]
            .iter().copied().take_while(|&id| id != eos_id).collect();
        let t_reply = tokenizer.decode(&t_gen_ids);

        let mut int_gen_ids = Vec::new();
        if idx < 3 {
            let mut next = q_top;
            for _ in 0..24 {
                if next == eos_id { break; }
                int_gen_ids.push(next);
                let nl = sess.step_token(next, true);
                next = nl.iter().enumerate().max_by_key(|&(_, &v)| v).map(|(i, _)| i as u32).unwrap_or(eos_id);
            }
        } else {
            int_gen_ids.push(q_top);
        }
        let int_reply = tokenizer.decode(&int_gen_ids);
        eprintln!("  [{pid}] top1={m1} top5={m5} kl={kl:.4} t={t_top} q={q_top} | t_reply={:?} | q_reply={:?}", t_reply, int_reply);
        ood_records.push((pid.to_string(), prompt_txt.to_string(), m1, m5, kl, t_reply, int_reply));
    }

    let top1_pct = (top1_hits as f64 / ood_list.len() as f64) * 100.0;
    let top5_pct = (top5_hits as f64 / ood_list.len() as f64) * 100.0;
    let mean_kl = sum_kl / (ood_list.len() as f64);
    eprintln!("[v13-recompiler] Parity: top1={top1_pct:.2}% top5={top5_pct:.2}% mean_KL={mean_kl:.6}");

    let container_bytes = model.to_bytes();
    eprintln!("[v13-recompiler] UORR4V13 container={} bytes ({:.2} MiB)", container_bytes.len(), container_bytes.len() as f64 / (1024.0 * 1024.0));
    let (c_sha, piece_count, parity_groups) =
        slice_pieces_and_stitch(&container_bytes, &pieces_dir, &out_weights, &mirror_url).expect("slice pieces");

    let mut rj = String::new();
    rj.push_str("{\n");
    rj.push_str("  \"schema\": \"uor-r4.holo-v13-receipt/1\",\n");
    rj.push_str("  \"format\": \"UORR4V13\",\n");
    rj.push_str(&format!("  \"hf_repo\": \"{}\",\n", json_escape(&hf_repo)));
    rj.push_str("  \"hardware\": \"NVIDIA GeForce RTX 4090\",\n");
    rj.push_str("  \"float_ops\": 0,\n");
    rj.push_str("  \"dense_matmul_ops\": 0,\n");
    rj.push_str(&format!("  \"container_sha256\": \"{c_sha}\",\n"));
    rj.push_str(&format!("  \"total_bytes\": {},\n", container_bytes.len()));
    rj.push_str(&format!("  \"piece_count\": {piece_count},\n"));
    rj.push_str(&format!("  \"parity_groups\": {parity_groups},\n"));
    rj.push_str(&format!("  \"vocab_size\": {vocab_size},\n"));
    rj.push_str(&format!("  \"d_model\": {d_model},\n"));
    rj.push_str(&format!("  \"n_layers\": {n_layers},\n"));
    rj.push_str(&format!("  \"n_heads\": {n_heads},\n"));
    rj.push_str(&format!("  \"n_kv_heads\": {n_kv_heads},\n"));
    rj.push_str(&format!("  \"head_dim\": {head_dim},\n"));
    rj.push_str(&format!("  \"top1_parity_pct\": {top1_pct:.2},\n"));
    rj.push_str(&format!("  \"top5_parity_pct\": {top5_pct:.2},\n"));
    rj.push_str(&format!("  \"mean_kl_nats\": {mean_kl:.6},\n"));
    rj.push_str("  \"ood_samples\": [\n");
    for (idx, (pid, ptxt, m1, m5, kl, tr, ir)) in ood_records.iter().enumerate() {
        let comma = if idx + 1 < ood_records.len() { "," } else { "" };
        rj.push_str(&format!(
            "    {{\"id\": \"{}\", \"prompt\": \"{}\", \"top1_match\": {}, \"top5_match\": {}, \"kl_divergence\": {:.6}, \"teacher_reply\": \"{}\", \"integer_reply\": \"{}\"}}{}\n",
            json_escape(pid), json_escape(ptxt), m1, m5, kl, json_escape(tr), json_escape(ir), comma
        ));
    }
    rj.push_str("  ]\n}\n");
    if let Some(parent) = out_receipt.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&out_receipt, &rj).expect("write receipt");
    eprintln!("[v13-recompiler] Wrote receipt {}", out_receipt.display());
    ExitCode::SUCCESS
}
