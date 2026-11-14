//! Standalone pure-Rust utilities for UOR-R4 corpus builders and tooling:
//! - Minimal recursive-descent JSON parser & serializer (`JsonVal`, `json_escape`)
//! - Exact HuggingFace ByteLevel BPE tokenizer loader & encoder (`ByteBpe4096`)
//!   with UTF-8 char-offset tracking matching `ByteLevel(trim_offsets=true)`
//! - Pure-Rust SHA-256 (`sha256_bytes`, `sha256_hex`)
//! - Deterministic SplitMix64 PRNG (`Rng64`)
//! - `curl` download/fetch helpers (`curl_download_cached`, `curl_fetch_stdout`)

#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;

// -----------------------------------------------------------------------------
// JSON Parser & Serializer
// -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum JsonVal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<JsonVal>),
    Obj(BTreeMap<String, JsonVal>),
}

impl JsonVal {
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut p = JsonParser {
            bytes: input.as_bytes(),
            pos: 0,
        };
        let val = p.parse_value()?;
        p.skip_ws();
        if p.pos != p.bytes.len() {
            return Err(format!("trailing JSON bytes at offset {}", p.pos));
        }
        Ok(val)
    }

    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, String> {
        let s = std::str::from_utf8(bytes).map_err(|e| format!("invalid UTF-8: {e}"))?;
        Self::parse(s)
    }

    pub fn get(&self, key: &str) -> Option<&JsonVal> {
        match self {
            JsonVal::Obj(m) => m.get(key),
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
            JsonVal::Int(v) => Some(*v),
            JsonVal::Float(v) => Some(*v as i64),
            _ => None,
        }
    }

    pub fn as_usize(&self) -> Option<usize> {
        self.as_i64().and_then(|v| usize::try_from(v).ok())
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonVal::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonVal]> {
        match self {
            JsonVal::Arr(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, JsonVal>> {
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
        while self.pos < self.bytes.len()
            && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\r' | b'\n')
        {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn expect(&mut self, expected: u8) -> Result<(), String> {
        self.skip_ws();
        match self.peek() {
            Some(b) if b == expected => {
                self.pos += 1;
                Ok(())
            }
            other => Err(format!(
                "expected '{}' at {}, got {:?}",
                expected as char,
                self.pos,
                other.map(|c| c as char)
            )),
        }
    }

    fn parse_value(&mut self) -> Result<JsonVal, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'n') => self.parse_lit(b"null", JsonVal::Null),
            Some(b't') => self.parse_lit(b"true", JsonVal::Bool(true)),
            Some(b'f') => self.parse_lit(b"false", JsonVal::Bool(false)),
            Some(b'"') => Ok(JsonVal::Str(self.parse_string()?)),
            Some(b'[') => self.parse_array(),
            Some(b'{') => self.parse_object(),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            other => Err(format!(
                "unexpected JSON char {:?} at {}",
                other.map(|c| c as char),
                self.pos
            )),
        }
    }

    fn parse_lit(&mut self, lit: &[u8], val: JsonVal) -> Result<JsonVal, String> {
        if self.bytes[self.pos..].starts_with(lit) {
            self.pos += lit.len();
            Ok(val)
        } else {
            Err(format!("invalid literal at {}", self.pos))
        }
    }

    fn parse_hex4(&mut self) -> Result<u16, String> {
        if self.pos + 4 > self.bytes.len() {
            return Err(format!("truncated \\u escape at {}", self.pos));
        }
        let s = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
            .map_err(|_| format!("invalid utf8 in \\u at {}", self.pos))?;
        let code = u16::from_str_radix(s, 16)
            .map_err(|_| format!("invalid hex '{}' at {}", s, self.pos))?;
        self.pos += 4;
        Ok(code)
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b'"' {
                self.pos += 1;
                return Ok(out);
            }
            if b == b'\\' {
                self.pos += 1;
                let esc = *self
                    .bytes
                    .get(self.pos)
                    .ok_or_else(|| "unterminated JSON escape".to_string())?;
                self.pos += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\x08'),
                    b'f' => out.push('\x0c'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let hi = self.parse_hex4()?;
                        if (0xD800..=0xDBFF).contains(&hi) {
                            if self.bytes.get(self.pos..self.pos + 2) == Some(b"\\u") {
                                self.pos += 2;
                                let lo = self.parse_hex4()?;
                                if (0xDC00..=0xDFFF).contains(&lo) {
                                    let cp = 0x10000
                                        + (((hi as u32 - 0xD800) << 10) | (lo as u32 - 0xDC00));
                                    if let Some(ch) = char::from_u32(cp) {
                                        out.push(ch);
                                    } else {
                                        out.push('\u{FFFD}');
                                    }
                                } else {
                                    out.push('\u{FFFD}');
                                }
                            } else {
                                out.push('\u{FFFD}');
                            }
                        } else if let Some(ch) = char::from_u32(hi as u32) {
                            out.push(ch);
                        } else {
                            out.push('\u{FFFD}');
                        }
                    }
                    other => return Err(format!("invalid JSON escape '\\{}'", other as char)),
                }
            } else if b < 0x80 {
                // Fast ASCII span scan
                let start = self.pos;
                self.pos += 1;
                while self.pos < self.bytes.len() {
                    let c = self.bytes[self.pos];
                    if c == b'"' || c == b'\\' || c >= 0x80 {
                        break;
                    }
                    self.pos += 1;
                }
                let ascii = std::str::from_utf8(&self.bytes[start..self.pos])
                    .map_err(|e| format!("utf8 error: {e}"))?;
                out.push_str(ascii);
            } else {
                let rest = std::str::from_utf8(&self.bytes[self.pos..])
                    .map_err(|e| format!("invalid utf8 in string: {e}"))?;
                let ch = rest
                    .chars()
                    .next()
                    .ok_or_else(|| "unexpected EOF in string".to_string())?;
                out.push(ch);
                self.pos += ch.len_utf8();
            }
        }
        Err("unterminated JSON string".to_string())
    }

    fn parse_array(&mut self) -> Result<JsonVal, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsonVal::Arr(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(JsonVal::Arr(items));
                }
                other => {
                    return Err(format!(
                        "expected ',' or ']' at {}, got {:?}",
                        self.pos,
                        other.map(|c| c as char)
                    ))
                }
            }
        }
    }

    fn parse_object(&mut self) -> Result<JsonVal, String> {
        self.expect(b'{')?;
        let mut map = BTreeMap::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonVal::Obj(map));
        }
        loop {
            let key = self.parse_string()?;
            self.expect(b':')?;
            let val = self.parse_value()?;
            map.insert(key, val);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(JsonVal::Obj(map));
                }
                other => {
                    return Err(format!(
                        "expected ',' or '}}' at {}, got {:?}",
                        self.pos,
                        other.map(|c| c as char)
                    ))
                }
            }
        }
    }

    fn parse_number(&mut self) -> Result<JsonVal, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.pos += 1;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        let slice = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| "invalid utf8 in number".to_string())?;
        if !is_float {
            if let Ok(iv) = slice.parse::<i64>() {
                return Ok(JsonVal::Int(iv));
            }
        }
        let fv = slice
            .parse::<f64>()
            .map_err(|e| format!("invalid JSON number '{slice}': {e}"))?;
        Ok(JsonVal::Float(fv))
    }
}

pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// -----------------------------------------------------------------------------
// ByteLevel BPE Tokenizer (tokenizer.json)
// -----------------------------------------------------------------------------

fn build_bytes_to_unicode() -> ([char; 256], HashMap<char, u8>) {
    let mut bs: Vec<u16> = Vec::with_capacity(256);
    for b in b'!'..=b'~' {
        bs.push(b as u16);
    }
    for b in 0xA1u16..=0xACu16 {
        bs.push(b);
    }
    for b in 0xAEu16..=0xFFu16 {
        bs.push(b);
    }
    let mut cs: Vec<u16> = bs.clone();
    let mut extra = 0u16;
    let mut present = [false; 256];
    for &b in &bs {
        present[b as usize] = true;
    }
    for b in 0u16..256 {
        if !present[b as usize] {
            bs.push(b);
            cs.push(256 + extra);
            extra += 1;
        }
    }
    let mut enc = ['\0'; 256];
    let mut dec = HashMap::with_capacity(256);
    for (b, c) in bs.into_iter().zip(cs.into_iter()) {
        let ch = char::from_u32(c as u32).expect("valid unicode mapping");
        enc[b as usize] = ch;
        dec.insert(ch, b as u8);
    }
    (enc, dec)
}

pub struct ByteBpe4096 {
    pub vocab_size: usize,
    piece_to_id: HashMap<String, u16>,
    pub id_to_piece: Vec<String>,
    pub id_to_bytes: Vec<Vec<u8>>,
    merges: HashMap<(String, String), usize>,
    added_tokens: Vec<(String, u16)>,
    byte_encoder: [char; 256],
    byte_decoder: HashMap<char, u8>,
    cache: RefCell<HashMap<String, Vec<u16>>>,
}

impl ByteBpe4096 {
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let data = fs::read_to_string(path)
            .map_err(|e| format!("failed reading {}: {e}", path.display()))?;
        Self::from_json_str(&data)
    }

    pub fn from_json_str(json_str: &str) -> Result<Self, String> {
        let root = JsonVal::parse(json_str)?;
        let model = root
            .get("model")
            .ok_or_else(|| "tokenizer.json missing 'model'".to_string())?;
        let vocab_obj = model
            .get("vocab")
            .and_then(|v| v.as_object())
            .ok_or_else(|| "tokenizer.json missing 'model.vocab'".to_string())?;
        let merges_arr = model
            .get("merges")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "tokenizer.json missing 'model.merges'".to_string())?;

        let (byte_encoder, byte_decoder) = build_bytes_to_unicode();

        let mut max_id = 0usize;
        for v in vocab_obj.values() {
            let id = v.as_usize().ok_or_else(|| "non-int vocab id".to_string())?;
            if id > max_id {
                max_id = id;
            }
        }
        let mut added_tokens: Vec<(String, u16)> = Vec::new();
        if let Some(added) = root.get("added_tokens").and_then(|v| v.as_array()) {
            for item in added {
                let id = item
                    .get("id")
                    .and_then(|v| v.as_usize())
                    .ok_or_else(|| "added_token missing id".to_string())?;
                let content = item
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "added_token missing content".to_string())?;
                if id > max_id {
                    max_id = id;
                }
                added_tokens.push((content.to_string(), id as u16));
            }
        }
        // Longest match first so longer special tokens win over prefixes.
        added_tokens.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.1.cmp(&b.1)));

        let vocab_size = max_id + 1;
        let mut piece_to_id = HashMap::with_capacity(vocab_size * 2);
        let mut id_to_piece = vec![String::new(); vocab_size];
        for (piece, val) in vocab_obj {
            let id = val.as_usize().unwrap();
            piece_to_id.insert(piece.clone(), id as u16);
            id_to_piece[id] = piece.clone();
        }
        for (content, id) in &added_tokens {
            piece_to_id.insert(content.clone(), *id);
            id_to_piece[*id as usize] = content.clone();
        }

        let mut id_to_bytes = vec![Vec::new(); vocab_size];
        let added_map: HashSet<u16> = added_tokens.iter().map(|(_, id)| *id).collect();
        for id in 0..vocab_size {
            let piece = &id_to_piece[id];
            if added_map.contains(&(id as u16)) {
                id_to_bytes[id] = piece.as_bytes().to_vec();
            } else {
                let mut raw = Vec::with_capacity(piece.len());
                for ch in piece.chars() {
                    if let Some(&b) = byte_decoder.get(&ch) {
                        raw.push(b);
                    }
                }
                id_to_bytes[id] = raw;
            }
        }

        let mut merges = HashMap::with_capacity(merges_arr.len() * 2);
        for (rank, entry) in merges_arr.iter().enumerate() {
            match entry {
                JsonVal::Str(s) => {
                    if let Some((a, b)) = s.split_once(' ') {
                        merges.insert((a.to_string(), b.to_string()), rank);
                    } else {
                        return Err(format!("invalid merge string '{s}' at {rank}"));
                    }
                }
                JsonVal::Arr(pair) if pair.len() == 2 => {
                    let a = pair[0]
                        .as_str()
                        .ok_or_else(|| format!("invalid merge[0] at {rank}"))?;
                    let b = pair[1]
                        .as_str()
                        .ok_or_else(|| format!("invalid merge[1] at {rank}"))?;
                    merges.insert((a.to_string(), b.to_string()), rank);
                }
                _ => return Err(format!("unsupported merge entry at {rank}")),
            }
        }

        Ok(Self {
            vocab_size,
            piece_to_id,
            id_to_piece,
            id_to_bytes,
            merges,
            added_tokens,
            byte_encoder,
            byte_decoder,
            cache: RefCell::new(HashMap::with_capacity(8192)),
        })
    }

    pub fn token_to_id(&self, tok: &str) -> Option<u16> {
        self.piece_to_id.get(tok).copied()
    }

    pub fn bos_id(&self) -> u16 {
        self.token_to_id("<|bos|>").unwrap_or(0)
    }

    pub fn eos_id(&self) -> u16 {
        self.token_to_id("<|eos|>").unwrap_or(1)
    }

    pub fn unk_id(&self) -> u16 {
        self.token_to_id("<|unk|>").unwrap_or(2)
    }

    pub fn decode_bytes(&self, ids: &[u16]) -> Vec<u8> {
        let mut out = Vec::with_capacity(ids.len() * 4);
        for &id in ids {
            if let Some(bs) = self.id_to_bytes.get(id as usize) {
                out.extend_from_slice(bs);
            }
        }
        out
    }

    pub fn decode(&self, ids: &[u16]) -> String {
        String::from_utf8_lossy(&self.decode_bytes(ids)).into_owned()
    }

    /// GPT-2 / HuggingFace ByteLevel pre-tokenization pattern:
    /// `'s|'t|'re|'ve|'m|'ll|'d| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+`
    /// Returns slices `(byte_start, byte_end)` within `segment`.
    fn gpt2_split(segment: &str) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let bytes = segment.as_bytes();
        let mut i = 0usize;
        while i < segment.len() {
            let rest = &segment[i..];
            // 1. Contractions
            let mut matched_contraction = 0usize;
            for pat in ["'re", "'ve", "'ll", "'s", "'t", "'m", "'d"] {
                if rest.starts_with(pat) {
                    matched_contraction = pat.len();
                    break;
                }
            }
            if matched_contraction > 0 {
                out.push((i, i + matched_contraction));
                i += matched_contraction;
                continue;
            }

            let mut chars = rest.char_indices();
            let (_, first_ch) = chars.next().unwrap();

            // 2. Optional single ASCII space followed by L+, N+, or non-(ws|L|N)+
            if first_ch == ' ' {
                if let Some((next_off, next_ch)) = chars.next() {
                    if !next_ch.is_whitespace() {
                        let start = i;
                        let mut end = i + next_off + next_ch.len_utf8();
                        if next_ch.is_alphabetic() {
                            for (off, ch) in chars {
                                if !ch.is_alphabetic() {
                                    break;
                                }
                                end = i + off + ch.len_utf8();
                            }
                            out.push((start, end));
                            i = end;
                            continue;
                        } else if next_ch.is_numeric() {
                            for (off, ch) in chars {
                                if !ch.is_numeric() {
                                    break;
                                }
                                end = i + off + ch.len_utf8();
                            }
                            out.push((start, end));
                            i = end;
                            continue;
                        } else {
                            for (off, ch) in chars {
                                if ch.is_whitespace() || ch.is_alphabetic() || ch.is_numeric() {
                                    break;
                                }
                                end = i + off + ch.len_utf8();
                            }
                            out.push((start, end));
                            i = end;
                            continue;
                        }
                    }
                }
            }

            // 3. Direct L+
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

            // 4. Direct N+
            if first_ch.is_numeric() {
                let mut end = i + first_ch.len_utf8();
                for (off, ch) in chars {
                    if !ch.is_numeric() {
                        break;
                    }
                    end = i + off + ch.len_utf8();
                }
                out.push((i, end));
                i = end;
                continue;
            }

            // 5. Direct [^\s\p{L}\p{N}]+
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

            // 6. `\s+(?!\S)|\s+`
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
                // Followed by `\S`, so `\s+(?!\S)` leaves the final whitespace char
                // so the next iteration can attach `' '` as ` ?...` if it is an ASCII space.
                out.push((i, last_ws_start));
                i = last_ws_start;
            } else {
                out.push((i, ws_end));
                i = ws_end;
            }
        }
        out
    }

    fn bpe_encode_pretoken(&self, pretoken: &str) -> Vec<u16> {
        if let Some(cached) = self.cache.borrow().get(pretoken) {
            return cached.clone();
        }
        let mut encoded = String::with_capacity(pretoken.len() * 2);
        for &b in pretoken.as_bytes() {
            encoded.push(self.byte_encoder[b as usize]);
        }
        let mut word: Vec<String> = encoded.chars().map(|c| c.to_string()).collect();
        while word.len() > 1 {
            let mut best_rank = usize::MAX;
            let mut best_idx = 0usize;
            for idx in 0..word.len() - 1 {
                // Borrow via tuple lookup? Allocation here only runs once per unique pretoken.
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

        let unk = self.unk_id();
        let mut ids = Vec::with_capacity(word.len());
        for piece in word {
            ids.push(*self.piece_to_id.get(&piece).unwrap_or(&unk));
        }
        let mut cache = self.cache.borrow_mut();
        if cache.len() < 200_000 {
            cache.insert(pretoken.to_string(), ids.clone());
        }
        ids
    }

    pub fn encode(&self, text: &str) -> Vec<u16> {
        let mut ids = Vec::with_capacity(text.len() / 2 + 8);
        self.encode_into(text, &mut ids, None);
        ids
    }

    /// Encodes `text` and returns `(ids, char_offsets)` matching HuggingFace
    /// `tokenizers` `ByteLevel(add_prefix_space=false, trim_offsets=true)`.
    pub fn encode_with_offsets(&self, text: &str) -> (Vec<u16>, Vec<(usize, usize)>) {
        let mut ids = Vec::with_capacity(text.len() / 2 + 8);
        let mut offsets = Vec::with_capacity(text.len() / 2 + 8);
        self.encode_into(text, &mut ids, Some(&mut offsets));
        (ids, offsets)
    }

    fn encode_into(
        &self,
        text: &str,
        out_ids: &mut Vec<u16>,
        mut out_offsets: Option<&mut Vec<(usize, usize)>>,
    ) {
        // Build O(N) byte_idx -> char_idx lookup if offsets are requested.
        let byte_to_char: Vec<usize> = if out_offsets.is_some() {
            let mut m = vec![0usize; text.len() + 1];
            let mut char_idx = 0usize;
            for (bi, ch) in text.char_indices() {
                for off in 0..ch.len_utf8() {
                    m[bi + off] = char_idx;
                }
                char_idx += 1;
            }
            m[text.len()] = char_idx;
            m
        } else {
            Vec::new()
        };

        // Split on added_tokens first, then run gpt2_split on normal spans.
        let mut pos = 0usize;
        let mut normal_start = 0usize;
        while pos < text.len() {
            let rest = &text[pos..];
            let mut matched: Option<(usize, u16)> = None;
            for (content, tid) in &self.added_tokens {
                if rest.starts_with(content.as_str()) {
                    matched = Some((content.len(), *tid));
                    break;
                }
            }
            if let Some((mlen, tid)) = matched {
                if pos > normal_start {
                    self.encode_normal_span(
                        &text[normal_start..pos],
                        normal_start,
                        &byte_to_char,
                        out_ids,
                        out_offsets.as_deref_mut(),
                    );
                }
                out_ids.push(tid);
                if let Some(ref mut offs) = out_offsets {
                    offs.push((byte_to_char[pos], byte_to_char[pos + mlen]));
                }
                pos += mlen;
                normal_start = pos;
            } else {
                let ch_len = rest.chars().next().unwrap().len_utf8();
                pos += ch_len;
            }
        }
        if text.len() > normal_start {
            self.encode_normal_span(
                &text[normal_start..],
                normal_start,
                &byte_to_char,
                out_ids,
                out_offsets,
            );
        }
    }

    fn encode_normal_span(
        &self,
        span: &str,
        base_byte: usize,
        byte_to_char: &[usize],
        out_ids: &mut Vec<u16>,
        mut out_offsets: Option<&mut Vec<(usize, usize)>>,
    ) {
        for (ps, pe) in Self::gpt2_split(span) {
            let pretoken = &span[ps..pe];
            let sub_ids = self.bpe_encode_pretoken(pretoken);
            let mut byte_cursor = base_byte + ps;
            for &tid in &sub_ids {
                out_ids.push(tid);
                let tok_bytes = &self.id_to_bytes[tid as usize];
                let raw_bs = byte_cursor;
                let raw_be = (byte_cursor + tok_bytes.len()).min(base_byte + pe);
                byte_cursor = raw_be;
                if let Some(ref mut offs) = out_offsets {
                    let cs = byte_to_char[raw_bs];
                    let ce = byte_to_char[raw_be];
                    // Match HF ByteLevel trim_offsets=true: trim leading/trailing whitespace
                    // iff the decoded token bytes form valid UTF-8 with >=1 non-whitespace char.
                    let (t_cs, t_ce) = if ce > cs {
                        if let Ok(s) = std::str::from_utf8(tok_bytes) {
                            let total_chars = s.chars().count();
                            let lead = s.chars().take_while(|c| c.is_whitespace()).count();
                            let trail = s.chars().rev().take_while(|c| c.is_whitespace()).count();
                            if lead + trail < total_chars {
                                (cs + lead, ce - trail)
                            } else {
                                (cs, ce)
                            }
                        } else {
                            (cs, ce)
                        }
                    } else {
                        (cs, ce)
                    };
                    offs.push((t_cs, t_ce));
                }
            }
        }
    }
}

/// Convert UTF-8 byte interval `[bs, be)` in `text` into Python-compatible Unicode char interval `[cs, ce)`.
pub fn byte_span_to_char_span(text: &str, bs: usize, be: usize) -> (usize, usize) {
    let cs = text[..bs].chars().count();
    let ce = cs + text[bs..be].chars().count();
    (cs, ce)
}

// -----------------------------------------------------------------------------
// SHA-256 (FIPS 180-4)
// -----------------------------------------------------------------------------

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
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
                .wrapping_add(SHA256_K[i])
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
        for (slot, delta) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(delta);
        }
    }
    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

pub fn sha256_hex(data: &[u8]) -> String {
    let digest = sha256_bytes(data);
    let mut s = String::with_capacity(64);
    for b in digest {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

// -----------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64)
// -----------------------------------------------------------------------------

pub struct Rng64 {
    state: u64,
}

impl Rng64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn randint(&mut self, lo: i64, hi_inclusive: i64) -> i64 {
        debug_assert!(hi_inclusive >= lo);
        let span = (hi_inclusive - lo) as u64 + 1;
        lo + (self.next_u64() % span) as i64
    }

    pub fn randrange(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        (self.next_u64() % (n as u64)) as usize
    }

    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let idx = self.randrange(items.len());
        &items[idx]
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        if items.len() <= 1 {
            return;
        }
        for i in (1..items.len()).rev() {
            let j = self.randrange(i + 1);
            items.swap(i, j);
        }
    }
}

// -----------------------------------------------------------------------------
// Curl Download Helpers
// -----------------------------------------------------------------------------

pub fn curl_download_cached(url: &str, dest: &Path, label: &str) -> Result<bool, String> {
    if dest.exists() && fs::metadata(dest).map(|m| m.len()).unwrap_or(0) > 100_000 {
        return Ok(true);
    }
    if let Some(parent) = dest.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let tmp = dest.with_extension("part");
    let status = Command::new("curl")
        .args([
            "-fsSL",
            "--connect-timeout",
            "15",
            "--max-time",
            "180",
            "--retry",
            "2",
            "-A",
            "uor-r4-rust/1.0",
            "-o",
        ])
        .arg(&tmp)
        .arg(url)
        .status();
    match status {
        Ok(st) if st.success() => {
            if let Ok(meta) = fs::metadata(&tmp) {
                if meta.len() > 0 {
                    fs::rename(&tmp, dest).map_err(|e| format!("rename failed: {e}"))?;
                    let mb = meta.len() as f64 / 1_048_576.0;
                    println!("[hf-cache] downloaded {label} ({mb:.1} MiB)");
                    return Ok(true);
                }
            }
            let _ = fs::remove_file(&tmp);
            Ok(false)
        }
        Ok(st) => {
            let _ = fs::remove_file(&tmp);
            eprintln!("[hf-cache] skip {label}: curl exit {st}");
            Ok(false)
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            eprintln!("[hf-cache] skip {label}: {e}");
            Ok(false)
        }
    }
}

pub fn curl_fetch_stdout(url: &str) -> Result<String, String> {
    let out = Command::new("curl")
        .args([
            "-fsSL",
            "--connect-timeout",
            "15",
            "--max-time",
            "60",
            "--retry",
            "2",
            "-A",
            "uor-r4-rust/1.0",
            url,
        ])
        .output()
        .map_err(|e| format!("failed to spawn curl: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("curl failed ({}): {}", out.status, err.trim()));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("curl stdout invalid UTF-8: {e}"))
}
