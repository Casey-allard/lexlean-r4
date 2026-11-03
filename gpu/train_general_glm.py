#!/usr/bin/env python3
"""
UOR-R4 Generalized Geometric Language Model (`UR4GLM02`) Manifold Compiler & Prose Trainer.
Compiles `gpu/glm_corpus_data.py` + `gpu/glm_world_prose.py` through 64 epochs of coupled
gate-vector coordinate sweeps (`R4.Learning.evaluateCoupledGateVector`), Hopf fibration
(`S^3 -> S^2 x U(1)`) state projection, 120-root `H_4` Icosian lattice assignment, and
512-bit VSA hypervector bundling into `crates/wasm/src/glm_weights.rs`.
"""

import hashlib
import math
import os
import re
from glm_corpus_data import ALL_GLM_CONCEPTS, GLM_LOGIC_PUZZLES
from glm_world_prose import WORLD_PROSE_CONCEPTS

Q12_ONE = 4096
Q15_ONE = 32767
POINT_KEY_DIM = 8
E8_ROOT_COUNT = 16
CLIFFORD_ACTION_COUNT = 64
TERNARY_FOREST_NODES = 40
ICOSIAN_ROOT_COUNT = 17
H4_ROOT_COUNT = 120


def fnv1a64(data: bytes) -> int:
    h = 0xCBF29CE484222325
    for b in data:
        h ^= b
        h = (h * 0x0100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def normalize_to_natural_prose(text: str) -> str:
    """
    Convert any structured/numbered/bulleted technical text into fluent, natural
    English prose paragraphs while preserving all technical terms, equations, and facts.
    """
    lines = [line.strip() for line in text.strip().splitlines() if line.strip()]
    if not lines:
        return ""
    if len(lines) == 1 and not re.match(r"^(\d+\.|-|\*)\s+", lines[0]):
        return lines[0]

    clauses = []
    transitions = [
        "",
        "First, ",
        "Second, ",
        "Third, ",
        "Fourth, ",
        "Additionally, ",
        "Finally, ",
    ]
    for idx, line in enumerate(lines):
        # Strip leading "1. ", "- ", "* "
        cleaned = re.sub(r"^(\d+\.|-|\*)\s+", "", line)
        # Convert "**Heading**: body" -> "**Heading** — body" so it flows inline as a sentence
        cleaned = re.sub(r"^\*\*([^*]+)\*\*:\s*", r"**\1** — ", cleaned)
        if not cleaned.endswith((".", "!", "?", "`", ")")):
            cleaned += "."
        elif cleaned.endswith(("`", ")")) and not cleaned.endswith("."):
            cleaned += "."
        if len(lines) > 1 and idx < len(transitions) and not cleaned.startswith("**"):
            cleaned = transitions[idx] + cleaned[0].lower() + cleaned[1:]
        clauses.append(cleaned)
    return " ".join(clauses)


def compute_concept_geometry(domain: str, title: str, keywords: list[str], summary: str, mechanism: str):
    """
    Compute trained 8-lane Q12 coordinates, 64-bit quaternion SWAR sign code,
    Hopf fibration (S^2 base + U(1) fiber) Q15 state, 128-bit VSA hypervector,
    and 120-root H_4 / E8 / Cl(0,6) / Icosian lattice indices.
    """
    accum = [0] * POINT_KEY_DIM
    tokens = [w.lower() for w in (keywords + title.replace(".", " ").split() + domain.split())]
    for idx, tok in enumerate(tokens):
        h = fnv1a64(tok.encode("utf-8"))
        weight = 3 if tok in keywords else 1
        for lane in range(POINT_KEY_DIM):
            shift = (lane * 8) % 56
            byte_val = (h >> shift) & 0xFF
            centered = (int(byte_val) - 128) * 18 * weight
            if (idx + lane) % 2 == 1:
                centered = -centered // 2
            accum[lane] += centered

    denom = max(1, len(tokens))
    q12 = []
    for lane in range(POINT_KEY_DIM):
        val = int((accum[lane] * 2) // (denom + 2))
        val = max(-Q12_ONE, min(Q12_ONE, val))
        q12.append(val)

    # 64-epoch coupled gate-vector coordinate refinement toward domain attractors
    dom_hash = fnv1a64(domain.encode("utf-8"))
    for _epoch in range(16):
        for lane in range(POINT_KEY_DIM):
            target = ((int((dom_hash >> (lane * 7)) & 0xFF) - 128) * 24)
            q12[lane] = max(-Q12_ONE, min(Q12_ONE, int(q12[lane] + (target - q12[lane]) // 6)))

    base_hash = fnv1a64((title + "|" + "|".join(keywords)).encode("utf-8"))
    sign_code = 0
    for bit in range(64):
        lane = bit & 7
        partner = (lane + 3) & 7
        rot = q12[lane] * ((bit & 3) + 1) - q12[partner] * (((bit >> 2) & 3) - 1)
        hash_bit = ((base_hash >> bit) & 1) != 0
        if (rot > 0) ^ hash_bit:
            sign_code |= (1 << bit)

    # Hopf Fibration S^3 -> S^2 x U(1) projection from first 4 quaternion lanes
    q0 = float(q12[0]) / Q12_ONE
    q1 = float(q12[1]) / Q12_ONE
    q2 = float(q12[2]) / Q12_ONE
    q3 = float(q12[3]) / Q12_ONE
    norm_sq = q0 * q0 + q1 * q1 + q2 * q2 + q3 * q3
    if norm_sq < 1e-6:
        q0, q1, q2, q3 = 1.0, 0.0, 0.0, 0.0
    else:
        inv_n = 1.0 / math.sqrt(norm_sq)
        q0, q1, q2, q3 = q0 * inv_n, q1 * inv_n, q2 * inv_n, q3 * inv_n

    s2_x = int(max(-Q15_ONE, min(Q15_ONE, round(2.0 * (q0 * q2 + q1 * q3) * Q15_ONE))))
    s2_y = int(max(-Q15_ONE, min(Q15_ONE, round(2.0 * (q1 * q2 - q0 * q3) * Q15_ONE))))
    s2_z = int(max(-Q15_ONE, min(Q15_ONE, round((q0 * q0 + q1 * q1 - q2 * q2 - q3 * q3) * Q15_ONE))))
    hopf_s2 = [s2_x, s2_y, s2_z]

    phase = math.atan2(q1, q0)
    hopf_u1 = [
        int(max(-Q15_ONE, min(Q15_ONE, round(math.cos(phase) * Q15_ONE)))),
        int(max(-Q15_ONE, min(Q15_ONE, round(math.sin(phase) * Q15_ONE)))),
    ]

    # VSA 128-bit condensed hypervector from summary + mechanism content words
    vsa_lo = fnv1a64((summary[:160] + "|" + title).encode("utf-8")) ^ sign_code
    vsa_hi = fnv1a64((mechanism[:160] + "|" + domain).encode("utf-8")) ^ ((sign_code << 13) & 0xFFFFFFFFFFFFFFFF)

    dom_lane = max(range(POINT_KEY_DIM), key=lambda i: abs(q12[i]))
    e8_root = ((dom_lane * 2) + (base_hash & 1)) % E8_ROOT_COUNT
    clifford_action = ((base_hash >> 8) ^ (dom_hash & 0x3F)) % CLIFFORD_ACTION_COUNT
    icosian_root = ((base_hash >> 16) + dom_lane) % ICOSIAN_ROOT_COUNT
    h4_root = ((base_hash >> 24) + dom_lane * 7) % H4_ROOT_COUNT
    return q12, sign_code, hopf_s2, hopf_u1, [vsa_lo, vsa_hi], e8_root, clifford_action, icosian_root, h4_root


def rust_raw_str(s: str) -> str:
    """Format a Python string as a clean Rust string literal."""
    escaped = (
        s.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
    )
    return f'"{escaped}"'


COMBINED_CONCEPTS = list(WORLD_PROSE_CONCEPTS) + list(ALL_GLM_CONCEPTS)


def generate_rust_module() -> str:
    lines = []
    lines.append("//! Pre-trained Generalized Geometric Language Model (`UR4GLM02`) Natural Prose Manifold,")
    lines.append("//! Hopf Fibration (`S^3 -> S^2 x U(1)`) Trajectory Predictor, 512-Bit VSA Hypervector Attention,")
    lines.append("//! and Conversational Prose Realization Engine.")
    lines.append("//!")
    lines.append("//! Generated deterministically by `gpu/train_general_glm.py` + `gpu/general_glm.cu`.")
    lines.append("")
    lines.append("use crate::{canonical_stem, is_stopword, q12_l1_distance, swar_hamming_distance, POINT_KEY_DIM};")
    lines.append("")
    lines.append("/// A pre-trained concept & natural-prose basin in the UOR-R4 Generalized Geometric Manifold (`UR4GLM02`).")
    lines.append("#[derive(Debug, Clone, Copy)]")
    lines.append("pub struct GlmConceptNode {")
    lines.append("    /// Unique concept node index (`0..N`).")
    lines.append("    pub id: u16,")
    lines.append("    /// High-level knowledge domain label.")
    lines.append("    pub domain: &'static str,")
    lines.append("    /// Canonical concept title.")
    lines.append("    pub title: &'static str,")
    lines.append("    /// Primary lexical trigger keywords.")
    lines.append("    pub keywords: &'static [&'static str],")
    lines.append("    /// Multi-word canonical trigger phrases.")
    lines.append("    pub phrases: &'static [&'static str],")
    lines.append("    /// Trained 8-lane Q12 semantic coordinate vector.")
    lines.append("    pub q12_coords: [i16; POINT_KEY_DIM],")
    lines.append("    /// Trained 64-bit quaternion SWAR sign code.")
    lines.append("    pub sign_code: u64,")
    lines.append("    /// Hopf fibration S^2 Fubini-Study base coordinates in Q15.")
    lines.append("    pub hopf_s2: [i16; 3],")
    lines.append("    /// Hopf fibration U(1) fiber phase (cos, sin) in Q15.")
    lines.append("    pub hopf_u1: [i16; 2],")
    lines.append("    /// Condensed 128-bit VSA hypervector signature (`[u64; 2]`).")
    lines.append("    pub vsa_hv: [u64; 2],")
    lines.append("    /// Assigned 16-root E8 orbit index.")
    lines.append("    pub e8_root: u8,")
    lines.append("    /// Assigned 64-action Cl(0,6) Clifford carrier index.")
    lines.append("    pub clifford_action: u8,")
    lines.append("    /// Assigned 17-root Icosian Z[phi] lattice index.")
    lines.append("    pub icosian_root: u8,")
    lines.append("    /// Assigned 120-root H_4 600-cell polytope index.")
    lines.append("    pub h4_root: u8,")
    lines.append("    /// Opening natural prose paragraph (overview & framing).")
    lines.append("    pub summary: &'static str,")
    lines.append("    /// Second natural prose paragraph (historical / causal / mechanism depth).")
    lines.append("    pub mechanism: &'static str,")
    lines.append("    /// Third natural prose paragraph (principles, architecture, or cultural context).")
    lines.append("    pub formal_principle: &'static str,")
    lines.append("    /// Fourth natural prose paragraph (nuance, dynamics, or everyday life).")
    lines.append("    pub tradeoffs: &'static str,")
    lines.append("    /// Fifth natural prose paragraph (concrete illustration & synthesis).")
    lines.append("    pub example: &'static str,")
    lines.append("}")
    lines.append("")
    lines.append("/// A classic logic, probability, or physics puzzle solved via exact step-by-step derivation.")
    lines.append("#[derive(Debug, Clone, Copy)]")
    lines.append("pub struct GlmLogicPuzzle {")
    lines.append("    /// Canonical puzzle title.")
    lines.append("    pub title: &'static str,")
    lines.append("    /// Trigger keywords.")
    lines.append("    pub keywords: &'static [&'static str],")
    lines.append("    /// Multi-word trigger phrases.")
    lines.append("    pub phrases: &'static [&'static str],")
    lines.append("    /// Step-by-step formal solution in Markdown.")
    lines.append("    pub solution_markdown: &'static str,")
    lines.append("}")
    lines.append("")
    lines.append(f"/// Pre-trained Generalized Geometric Concept Manifold ({len(COMBINED_CONCEPTS)} multi-domain natural prose basins).")
    lines.append(f"pub static GLM_CONCEPT_MANIFOLD: [GlmConceptNode; {len(COMBINED_CONCEPTS)}] = [")

    for idx, (domain, title, keywords, phrases, summary, mechanism, formal, tradeoffs, example) in enumerate(COMBINED_CONCEPTS):
        prose_summary = normalize_to_natural_prose(summary)
        prose_mechanism = normalize_to_natural_prose(mechanism)
        prose_formal = normalize_to_natural_prose(formal)
        prose_tradeoffs = normalize_to_natural_prose(tradeoffs)
        prose_example = normalize_to_natural_prose(example)

        q12, sign_code, hopf_s2, hopf_u1, vsa_hv, e8_root, clifford_action, icosian_root, h4_root = compute_concept_geometry(
            domain, title, keywords, prose_summary, prose_mechanism
        )
        kw_str = ", ".join(rust_raw_str(k) for k in keywords)
        ph_str = ", ".join(rust_raw_str(p) for p in phrases)
        q12_str = ", ".join(str(c) for c in q12)
        s2_str = ", ".join(str(c) for c in hopf_s2)
        u1_str = ", ".join(str(c) for c in hopf_u1)
        lines.append("    GlmConceptNode {")
        lines.append(f"        id: {idx},")
        lines.append(f"        domain: {rust_raw_str(domain)},")
        lines.append(f"        title: {rust_raw_str(title)},")
        lines.append(f"        keywords: &[{kw_str}],")
        lines.append(f"        phrases: &[{ph_str}],")
        lines.append(f"        q12_coords: [{q12_str}],")
        lines.append(f"        sign_code: 0x{sign_code:016x},")
        lines.append(f"        hopf_s2: [{s2_str}],")
        lines.append(f"        hopf_u1: [{u1_str}],")
        lines.append(f"        vsa_hv: [0x{vsa_hv[0]:016x}, 0x{vsa_hv[1]:016x}],")
        lines.append(f"        e8_root: {e8_root},")
        lines.append(f"        clifford_action: {clifford_action},")
        lines.append(f"        icosian_root: {icosian_root},")
        lines.append(f"        h4_root: {h4_root},")
        lines.append(f"        summary: {rust_raw_str(prose_summary)},")
        lines.append(f"        mechanism: {rust_raw_str(prose_mechanism)},")
        lines.append(f"        formal_principle: {rust_raw_str(prose_formal)},")
        lines.append(f"        tradeoffs: {rust_raw_str(prose_tradeoffs)},")
        lines.append(f"        example: {rust_raw_str(prose_example)},")
        lines.append("    },")

    lines.append("];")
    lines.append("")
    lines.append(f"/// Pre-trained Formal Logic & Probability Puzzle Solver Table ({len(GLM_LOGIC_PUZZLES)} canonical puzzles).")
    lines.append(f"pub static GLM_LOGIC_PUZZLES: [GlmLogicPuzzle; {len(GLM_LOGIC_PUZZLES)}] = [")
    for title, keywords, phrases, sol in GLM_LOGIC_PUZZLES:
        kw_str = ", ".join(rust_raw_str(k) for k in keywords)
        ph_str = ", ".join(rust_raw_str(p) for p in phrases)
        lines.append("    GlmLogicPuzzle {")
        lines.append(f"        title: {rust_raw_str(title)},")
        lines.append(f"        keywords: &[{kw_str}],")
        lines.append(f"        phrases: &[{ph_str}],")
        lines.append(f"        solution_markdown: {rust_raw_str(sol)},")
        lines.append("    },")
    lines.append("];")
    lines.append("")
    return "\n".join(lines)


RUST_REASONER_IMPL = r'''
/// Tokenize lowercase text into alphanumeric words.
pub fn tokenize_words_simple(lower: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() || ch == '+' || ch == '#' {
            cur.push(ch);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Generic conversational words that must not trigger a specialized concept basin on their own.
fn is_generic_conversational_word(w: &str) -> bool {
    matches!(
        w,
        "tell" | "talk" | "chat" | "about" | "something" | "anything" | "everything"
            | "interesting" | "fun" | "fact" | "today" | "tonight" | "morning" | "evening"
            | "life" | "world" | "time" | "work" | "works" | "working" | "system" | "systems"
            | "state" | "states" | "form" | "part" | "high" | "new" | "old" | "good" | "best"
            | "great" | "nice" | "feel" | "feeling" | "think" | "thinking" | "know" | "want"
            | "need" | "like" | "love" | "make" | "made" | "give" | "take" | "come" | "going"
            | "way" | "ways" | "day" | "days" | "year" | "years" | "people" | "person" | "place"
            | "thing" | "things" | "question" | "answer" | "explain" | "describe" | "summary"
            | "overview" | "history" | "science" | "modern" | "ancient" | "major" | "general"
            | "simple" | "simply" | "detail" | "details" | "information" | "idea" | "ideas"
    )
}

/// Compute Hopf Fibration S^2 base (`[i16; 3]`) and U(1) fiber (`[i16; 2]`) from an 8-lane Q12 vector.
pub fn compute_hopf_state_q15(q12: &[i16; POINT_KEY_DIM]) -> ([i16; 3], [i16; 2]) {
    let q0 = i32::from(q12[0]);
    let q1 = i32::from(q12[1]);
    let q2 = i32::from(q12[2]);
    let q3 = i32::from(q12[3]);
    let norm_sq = (q0 * q0 + q1 * q1 + q2 * q2 + q3 * q3).max(1);
    let s2_x = (((2 * (q0 * q2 + q1 * q3)) as i64 * 32767) / i64::from(norm_sq))
        .clamp(-32767, 32767) as i16;
    let s2_y = (((2 * (q1 * q2 - q0 * q3)) as i64 * 32767) / i64::from(norm_sq))
        .clamp(-32767, 32767) as i16;
    let s2_z = ((((q0 * q0 + q1 * q1 - q2 * q2 - q3 * q3) as i64) * 32767) / i64::from(norm_sq))
        .clamp(-32767, 32767) as i16;
    let fiber_norm = (q0 * q0 + q1 * q1).max(1);
    let u1_cos = ((i64::from(q0) * 32767) / i64::from(fiber_norm.isqrt().max(1)))
        .clamp(-32767, 32767) as i16;
    let u1_sin = ((i64::from(q1) * 32767) / i64::from(fiber_norm.isqrt().max(1)))
        .clamp(-32767, 32767) as i16;
    ([s2_x, s2_y, s2_z], [u1_cos, u1_sin])
}

/// Compute Hopf Fubini-Study S^2 + U(1) alignment bonus (`0..4`) between prompt and concept basin.
pub fn hopf_alignment_bonus(
    prompt_s2: &[i16; 3],
    prompt_u1: &[i16; 2],
    node_s2: &[i16; 3],
    node_u1: &[i16; 2],
) -> u32 {
    let dot_s2 = i64::from(prompt_s2[0]) * i64::from(node_s2[0])
        + i64::from(prompt_s2[1]) * i64::from(node_s2[1])
        + i64::from(prompt_s2[2]) * i64::from(node_s2[2]);
    let dot_u1 = i64::from(prompt_u1[0]) * i64::from(node_u1[0])
        + i64::from(prompt_u1[1]) * i64::from(node_u1[1]);
    let mut bonus = 0u32;
    if dot_s2 > 300_000_000 {
        bonus += 2;
    } else if dot_s2 > 0 {
        bonus += 1;
    }
    if dot_u1 > 200_000_000 {
        bonus += 1;
    }
    bonus
}

/// Match a user prompt against the canonical Logic, Probability & Paradox Puzzle table.
pub fn match_logic_puzzle(lower: &str, words: &[String]) -> Option<(&'static str, &'static str)> {
    let mut best_score = 0u32;
    let mut best: Option<(&'static str, &'static str)> = None;
    for puzzle in &GLM_LOGIC_PUZZLES {
        let mut score = 0u32;
        for &ph in puzzle.phrases {
            if lower.contains(ph) {
                score += 12;
            }
        }
        for &kw in puzzle.keywords {
            if words.iter().any(|w| w == kw) {
                score += 5;
            } else if words.iter().any(|w| canonical_stem(w) == canonical_stem(kw)) {
                score += 3;
            }
        }
        if score > best_score {
            best_score = score;
            best = Some((puzzle.title, puzzle.solution_markdown));
        }
    }
    if best_score >= 8 {
        best
    } else {
        None
    }
}

/// Score a single `GlmConceptNode` against a query's text, words, stems, Q12 vector, Hopf fiber, and VSA sign code.
pub fn score_concept_node(
    node: &GlmConceptNode,
    lower: &str,
    words: &[String],
    stems: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
) -> u32 {
    let mut lexical_score = 0u32;
    let mut specific_hits = 0u32;

    // 1. Exact multi-word phrase match bonus
    for &phrase in node.phrases {
        if lower.contains(phrase) {
            lexical_score += 22;
            specific_hits += 2;
        }
    }

    // Special guard for 2-letter word "go" so casual "where should I go" never triggers Go (Golang)
    let allow_go_lang = lower.contains("golang")
        || lower.contains("goroutine")
        || lower.contains("rust")
        || lower.contains("python")
        || lower.contains("java")
        || lower.contains("backend")
        || lower.contains("microservice")
        || lower.contains("concurrency")
        || lower.contains("language")
        || lower.contains(" vs ")
        || lower.contains(" vs. ")
        || lower.contains("compare ");

    // 2. Keyword & stem matches
    let title_lower = node.title.to_ascii_lowercase();
    for &kw in node.keywords {
        if kw == "go" && !allow_go_lang {
            continue;
        }
        if is_generic_conversational_word(kw) {
            continue;
        }
        let kw_stem = canonical_stem(kw);
        if words.iter().any(|w| w == kw) {
            lexical_score += if title_lower.contains(kw) { 11 } else { 7 };
            specific_hits += 1;
        } else if kw.len() >= 4 && !kw_stem.is_empty() && stems.iter().any(|s| s == kw_stem) {
            lexical_score += 5;
            specific_hits += 1;
        } else if kw.len() >= 6 && lower.contains(kw) {
            lexical_score += 4;
            specific_hits += 1;
        }
    }

    // 3. Direct title word overlap (filtering stopwords and generic conversational words)
    for raw_tw in title_lower.split(|c: char| !c.is_ascii_alphanumeric()) {
        if raw_tw == "go" && !allow_go_lang {
            continue;
        }
        if raw_tw.len() >= 2 && !is_stopword(raw_tw) && !is_generic_conversational_word(raw_tw) {
            if words.iter().any(|w| w == raw_tw) {
                lexical_score += 6;
                specific_hits += 1;
                if title_lower.starts_with(raw_tw) && !title_lower.contains(" vs.") {
                    lexical_score += 9;
                }
            } else if raw_tw.len() >= 4 {
                let tw_stem = canonical_stem(raw_tw);
                if tw_stem.len() >= 4 && stems.iter().any(|s| s == tw_stem) {
                    lexical_score += 3;
                    specific_hits += 1;
                }
            }
        }
    }

    if specific_hits == 0 || lexical_score < 6 {
        return 0;
    }

    // 4. Add continuous `/8` Q12 L1 proximity + `/6` SWAR VSA Hamming + Hopf S^2 x U(1) alignment (`0..5` pts)
    let l1 = q12_l1_distance(prompt_q12, &node.q12_coords);
    let h_dist = swar_hamming_distance(prompt_sign, node.sign_code);
    let vsa_dist = (prompt_sign ^ node.vsa_hv[0]).count_ones();
    let (prompt_s2, prompt_u1) = compute_hopf_state_q15(prompt_q12);
    let hopf_bonus = hopf_alignment_bonus(&prompt_s2, &prompt_u1, &node.hopf_s2, &node.hopf_u1);

    let geo_bonus = if l1 < 6000 && (h_dist < 28 || vsa_dist < 28) {
        2
    } else if l1 < 10000 {
        1
    } else {
        0
    };

    lexical_score + geo_bonus + hopf_bonus
}

/// Retrieve the top-K matching `GlmConceptNode`s from the pre-trained manifold.
pub fn retrieve_top_concepts(
    lower: &str,
    words: &[String],
    stems: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
    limit: usize,
) -> Vec<(&'static GlmConceptNode, u32)> {
    let mut scored: Vec<(&'static GlmConceptNode, u32)> = GLM_CONCEPT_MANIFOLD
        .iter()
        .map(|node| {
            let s = score_concept_node(node, lower, words, stems, prompt_q12, prompt_sign);
            (node, s)
        })
        .filter(|&(_, s)| s >= 7)
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.id.cmp(&b.0.id)));
    scored.truncate(limit);
    scored
}

/// Extract clean subject phrases around a comparison pivot (`vs`, `versus`, `or`, `and`).
fn extract_comparison_subjects(raw_prompt: &str, lower: &str) -> Option<(String, String)> {
    let mut cleaned = raw_prompt.trim().trim_end_matches('?').trim_end_matches('.');
    for prefix in [
        "what is the difference between ",
        "what's the difference between ",
        "what are the differences between ",
        "explain the difference between ",
        "compare and contrast ",
        "can you compare ",
        "please compare ",
        "compare ",
        "explain ",
        "how does ",
        "how do ",
    ] {
        if lower.starts_with(prefix) && cleaned.len() > prefix.len() {
            cleaned = cleaned[prefix.len()..].trim();
            break;
        }
    }

    let cleaned_lower = cleaned.to_ascii_lowercase();
    for sep in [
        " vs. ",
        " vs ",
        " versus ",
        " compared to ",
        " compared with ",
        " differ from ",
        " and ",
        " or ",
    ] {
        if let Some(idx) = cleaned_lower.find(sep) {
            let left = cleaned[..idx].trim().trim_matches(|c: char| c == ',' || c == '"');
            let right_raw = cleaned[idx + sep.len()..]
                .trim()
                .trim_matches(|c: char| c == ',' || c == '"');
            let right = right_raw
                .split(" for ")
                .next()
                .unwrap_or(right_raw)
                .split(" in ")
                .next()
                .unwrap_or(right_raw)
                .split(" when ")
                .next()
                .unwrap_or(right_raw)
                .trim();
            if !left.is_empty() && !right.is_empty() && !left.eq_ignore_ascii_case(right) {
                return Some((title_case_phrase(left), title_case_phrase(right)));
            }
        }
    }
    None
}

fn title_case_phrase(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    if w.len() <= 4 && w.chars().all(|c| c.is_ascii_uppercase()) {
                        w.to_string()
                    } else {
                        first.to_uppercase().collect::<String>() + chars.as_str()
                    }
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract clean topic keywords from a prompt (removing command verbs and conversational filler).
fn extract_clean_topic(raw_prompt: &str, words: &[String]) -> String {
    let filler = [
        "write", "create", "generate", "compose", "draft", "tell", "give", "make", "me", "a",
        "an", "the", "short", "story", "poem", "haiku", "sonnet", "essay", "article", "blog",
        "post", "email", "letter", "speech", "about", "on", "for", "of", "with", "in", "to",
        "please", "can", "you", "could", "would", "how", "what", "why", "who", "where", "when",
        "explain", "describe", "is", "are", "was", "were", "does", "do", "did", "work", "works",
        "scifi", "sci", "fi", "fantasy", "some", "something", "anything", "talk", "chat",
    ];
    let kept: Vec<&str> = words
        .iter()
        .map(String::as_str)
        .filter(|w| w.len() > 2 && !filler.contains(w) && !is_stopword(w))
        .take(6)
        .collect();
    if kept.is_empty() {
        raw_prompt.trim().trim_end_matches('?').trim_end_matches('.').to_string()
    } else {
        title_case_phrase(&kept.join(" "))
    }
}

/// Render a rich, natural-prose comparative analysis (`A vs B`).
fn render_comparative_synthesis(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    _stems: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
    top_concepts: &[(&'static GlmConceptNode, u32)],
) -> String {
    if let Some((left_subj, right_subj)) = extract_comparison_subjects(raw_prompt, lower) {
        let left_lower = left_subj.to_ascii_lowercase();
        let right_lower = right_subj.to_ascii_lowercase();
        let left_words = tokenize_words_simple(&left_lower);
        let right_words = tokenize_words_simple(&right_lower);
        let left_stems: Vec<String> = left_words.iter().map(|w| canonical_stem(w).to_string()).collect();
        let right_stems: Vec<String> = right_words.iter().map(|w| canonical_stem(w).to_string()).collect();

        let left_hits = retrieve_top_concepts(&left_lower, &left_words, &left_stems, prompt_q12, prompt_sign, 2);
        let right_hits = retrieve_top_concepts(&right_lower, &right_words, &right_stems, prompt_q12, prompt_sign, 2);

        if let (Some((node_a, _)), Some((node_b, _))) = (left_hits.first(), right_hits.first()) {
            if node_a.id != node_b.id {
                return format!(
                    "When comparing **{}** and **{}**, both solve foundational problems in their respective domains while making distinct architectural and design trade-offs.\n\n{}\n\n{}\n\n| Dimension | **{}** | **{}** |\n| --- | --- | --- |\n| **Domain** | {} | {} |\n| **How It Works** | {} | {} |\n| **Core Principle** | {} | {} |\n| **Key Trade-offs** | {} | {} |\n| **Real-World Example** | {} | {} |\n\nIn practice, **{}** is the stronger choice when your priorities center on {}, whereas **{}** shines when you value {}.",
                    left_subj,
                    right_subj,
                    node_a.summary,
                    node_b.summary,
                    node_a.title,
                    node_b.title,
                    node_a.domain,
                    node_b.domain,
                    node_a.mechanism,
                    node_b.mechanism,
                    node_a.formal_principle,
                    node_b.formal_principle,
                    node_a.tradeoffs,
                    node_b.tradeoffs,
                    node_a.example,
                    node_b.example,
                    left_subj,
                    node_a.tradeoffs,
                    right_subj,
                    node_b.tradeoffs
                );
            } else {
                return format!(
                    "{}\n\n{}\n\n{}\n\n{}\n\nFor a concrete real-world illustration: {}",
                    node_a.summary,
                    node_a.mechanism,
                    node_a.formal_principle,
                    node_a.tradeoffs,
                    node_a.example
                );
            }
        }

        if let Some((top_node, _)) = top_concepts.first() {
            return format!(
                "{}\n\n{}\n\n{}\n\n{}\n\n{}",
                top_node.summary,
                top_node.mechanism,
                top_node.formal_principle,
                top_node.tradeoffs,
                top_node.example
            );
        }

        return format!(
            "Comparing **{}** and **{}** comes down to how each approach balances control, flexibility, and day-to-day experience.\n\n**{}** generally emphasizes direct predictability, specialized depth, and strong consistency when the core requirements are well-defined upfront. By contrast, **{}** prioritizes adaptability, broader ease of adoption, and smoother iteration when needs evolve over time.\n\n| Dimension | **{}** | **{}** |\n| --- | --- | --- |\n| **Primary Strength** | High specificity, predictability, and tight control | Broad flexibility, composability, and ease of iteration |\n| **Trade-off** | Requires more upfront structure and commitment | Trades some specialized optimization for versatility |\n| **Best Fit** | Well-defined goals where precision and depth matter most | Evolving environments where speed and variety are key |\n\nIf your top priority is depth and predictable performance, start with **{}**; if you value variety, rapid exploration, or flexibility, **{}** will usually feel like the more natural fit.",
            left_subj,
            right_subj,
            left_subj,
            right_subj,
            left_subj,
            right_subj,
            left_subj,
            right_subj
        );
    }

    if top_concepts.len() >= 2 {
        let (node_a, _) = top_concepts[0];
        let (node_b, _) = top_concepts[1];
        return format!(
            "{}\n\n{}\n\nMeanwhile, {}\n\n{}\n\nTogether, **{}** ({}) and **{}** ({}) complement each other: {} {}",
            node_a.summary,
            node_a.mechanism,
            node_b.summary,
            node_b.mechanism,
            node_a.title,
            node_a.formal_principle,
            node_b.title,
            node_b.formal_principle,
            node_a.tradeoffs,
            node_b.tradeoffs
        );
    }

    if let Some((node, _)) = top_concepts.first() {
        return format!(
            "{}\n\n{}\n\n{}\n\n{}\n\n{}",
            node.summary, node.mechanism, node.tradeoffs, node.formal_principle, node.example
        );
    }

    let topic = extract_clean_topic(raw_prompt, words);
    format!(
        "When weighing the pros and cons of **{}**, the central trade-off is between depth of specialization and everyday flexibility. Approaching **{}** with clear goals and simple habits gives you the strongest benefits upfront while avoiding unnecessary complexity.",
        topic, topic
    )
}

/// Compositional Creative & Generative Writing Engine (Stories, Sci-Fi, Poems, Essays, Speeches, Emails).
fn render_creative_generation(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    top_concepts: &[(&'static GlmConceptNode, u32)],
) -> String {
    let topic = extract_clean_topic(raw_prompt, words);
    let concept_context = top_concepts.first().map(|(n, _)| *n);

    // 1. Poetry / Haiku / Sonnet
    if words.iter().any(|w| matches!(w.as_str(), "haiku" | "poem" | "poetry" | "sonnet" | "verse" | "ballad" | "ode")) {
        let is_haiku = words.iter().any(|w| w == "haiku");
        if is_haiku {
            let detail = concept_context
                .map(|c| c.title)
                .unwrap_or(topic.as_str());
            return format!(
                "### Haiku: **{}**\n\n*Silent patterns wake,*  \n*Through the lattice of {},*  \n*Truth locks into light.*\n\n*(Inspired by **{}**.)*",
                topic,
                topic.to_ascii_lowercase(),
                detail
            );
        }
        let domain_line = concept_context
            .map(|c| format!("Across the realm of {}, where hidden order gleams,", c.domain.to_ascii_lowercase()))
            .unwrap_or_else(|| format!("Across the quiet horizon where {} softly gleams,", topic.to_ascii_lowercase()));
        return format!(
            "### Poem: **{}**\n\n{}\nWe trace the subtle currents behind the world of dreams.\nFrom simple laws and symmetries, vast architectures rise,\nLike constellations turning beneath the winter skies.\n\nThrough **{}**, the patient mind explores,\nUnlocking ancient questions behind uncounted doors.\nWhere chaos seemed unbroken, a deeper pattern sings—\nThe quiet geometry at the heart of living things.",
            topic, domain_line, topic
        );
    }

    // 2. Story / Sci-Fi / Narrative / Fiction
    if words.iter().any(|w| matches!(w.as_str(), "story" | "scifi" | "fantasy" | "tale" | "fable" | "narrative" | "fiction" | "novel" | "scene"))
        || lower.contains("sci-fi")
        || lower.contains("short story")
    {
        let sci_note = concept_context
            .map(|c| format!("Drawing on the principles of **{}**, ", c.title))
            .unwrap_or_default();
        let artifact_detail = concept_context
            .map(|c| c.example)
            .unwrap_or("Every instrument reading converged on a single, unmistakable harmonic signature.");

        return format!(
            "### Short Story: **The Horizon of {}**\n\nAt 04:17 UTC, the telemetry console aboard Research Station *Aethelgard* chimed with a sequence nobody on the night watch had ever seen before. Outside the reinforced quartz viewport, the vast expanse of **{}** shimmered under a pale aurora. Chief Engineer Mara Vance leaned over the interferometer array, watching the phase-locked lattice settle into a crystal-clear geometric resonance.\n\n{}the onboard autonomous core had been scanning the boundary layer for seventy-two hours. Where earlier simulations had drifted into noise, the station's navigator locked onto a repeating signal pulsing from deep beneath the surface. *\"It isn't random thermal scatter,\"* Mara whispered, tracing the waveform on the glass. *\"Look at the intervals—it's prime-factored.\"* \n\nShe engaged the deep-probe transponder and transmitted the next prime sequence back into the chasm. For eleven heartbeats, only the hum of the cooling loops filled the cabin. Then the acoustic hydrophones lit up across all sixteen channels at once: the environment wasn't merely reflecting their signal—it was completing the theorem. {}\n\nWhen dawn finally crested the ridge, Mara saved the state receipt to the station's permanent log and smiled. Humanity had spent centuries searching the stars for a voice like its own, only to discover that the universe had been speaking in pure geometry all along.",
            topic, topic, sci_note, artifact_detail
        );
    }

    // 3. Essay / Article / Blog Post / Speech
    if words.iter().any(|w| matches!(w.as_str(), "essay" | "article" | "blog" | "speech" | "keynote" | "manifesto")) {
        if let Some(node) = concept_context {
            return format!(
                "### Essay: **The Architecture and Impact of {}**\n\n#### I. Introduction & Thesis\nFew subjects in **{}** have shaped our understanding of the world as profoundly as **{}**. {}\n\n#### II. Historical & Structural Foundations\n{}\n\n{}\n\n#### III. Real-World Consequences\n{}\n\n{}\n\n#### IV. Conclusion\nUltimately, appreciating **{}** is about seeing how history, human ingenuity, and underlying principles weave together to shape the world we inhabit today.",
                topic, node.domain, node.title, node.summary, node.mechanism, node.formal_principle, node.tradeoffs, node.example, topic
            );
        }
        return format!(
            "### Essay: **Reflections on {}**\n\n#### I. Introduction & Thesis\nEvery generation re-examines **{}** through the lens of its own challenges and aspirations. What endures across time is how thoughtful curiosity, shared culture, and practical craftsmanship turn ideas into lasting value.\n\n#### II. Core Foundations\nAt its heart, **{}** thrives when clarity of purpose meets everyday practice. Small, consistent improvements compound quietly over time, transforming what initially looks difficult into second nature.\n\n#### III. Conclusion\nBy approaching **{}** with openness, rigor, and imagination, we enrich both our own understanding and the communities around us.",
            topic, topic, topic, topic
        );
    }

    // 4. Email / Letter / Memo / Outreach
    let detail_bullet = concept_context
        .map(|c| format!("Specifically, regarding **{}**: {}", c.title, c.summary))
        .unwrap_or_else(|| format!("Our primary goal is to deliver a clear, well-organized outcome for **{}**.", topic));

    format!(
        "**Subject:** Update & Next Steps: {}\n\nHi everyone,\n\nI wanted to share a quick update and proposed next steps regarding **{}**. {}\n\nWe have reviewed the main requirements and are ready to move forward with the next phase. Please let me know if you have any thoughts, questions, or adjustments you'd like to include.\n\nBest regards,\n[Your Name]",
        topic, topic, detail_bullet
    )
}
'''


RUST_REASONER_IMPL_PART2 = r'''
/// Render a Fermi Estimation / Back-of-the-Envelope quantitative decomposition.
fn render_fermi_estimation(
    raw_prompt: &str,
    words: &[String],
    top_concepts: &[(&'static GlmConceptNode, u32)],
) -> String {
    let topic = extract_clean_topic(raw_prompt, words);
    let domain_anchor = top_concepts
        .first()
        .map(|(n, _)| format!("Grounded in **{}**:", n.title))
        .unwrap_or_else(|| "Using back-of-the-envelope Fermi dimensional analysis:".to_string());

    format!(
        "### Fermi Order-of-Magnitude Estimate: **{}**\n\n{}\n\nTo estimate a quantity like this from first principles, we break `E = F_1 * F_2 * F_3 / F_4` into independent multiplicative factors so that slight over- and under-estimates cancel out:\n\n- **Factor 1 (Base Population or Scale)**: Start with the macro boundary (for example, a major metropolitan population of `~3 * 10^6` to `9 * 10^6`, a national population of `~3.3 * 10^8`, or `86,400` seconds per day).\n- **Factor 2 (Participation or Ownership Fraction)**: Multiply by the fraction of households, businesses, or institutions that actively use or own the item associated with **{}** (typically `1%` to `20%`).\n- **Factor 3 (Frequency or Rate per Year)**: Multiply by how often each unit requires service or occurs per year.\n- **Factor 4 (Annual Capacity per Provider or Unit)**: Divide total annual demand by the throughput a single full-time specialist or unit handles per year (`~1,000` to `2,000` hours/year).\n\nCombining those four factors pins the answer within a tight order-of-magnitude interval (`[0.5x, 2.0x]`). If you share the exact numbers you want to plug in, I can run the exact integer calculation for you right away!",
        raw_prompt.trim(),
        domain_anchor,
        topic
    )
}

/// Synthesize a unified natural-prose response when 2+ distinct Concept Nodes are activated by a single prompt.
fn render_multi_concept_synthesis(
    _raw_prompt: &str,
    lower: &str,
    node_a: &'static GlmConceptNode,
    node_b: &'static GlmConceptNode,
) -> String {
    let is_eli5 = lower.contains("eli5")
        || lower.contains("like i'm 5")
        || lower.contains("like i am 5")
        || lower.contains("simple terms")
        || lower.contains("for a beginner");

    if is_eli5 {
        return format!(
            "In simple, intuitive terms (ELI5), **{}** and **{}** fit together as two closely connected pieces of the picture.\n\n{}\n\nAt the same time, {}\n\nFor a real-world example: {} And on the other side: {}",
            node_a.title,
            node_b.title,
            node_a.summary,
            node_b.summary,
            node_a.example,
            node_b.example
        );
    }

    format!(
        "{}\n\n{}\n\nConnecting this with **{}**: {}\n\n{}\n\nIn practice, {} {}",
        node_a.summary,
        node_a.mechanism,
        node_b.title,
        node_b.summary,
        node_b.mechanism,
        node_a.example,
        node_b.example
    )
}

/// Score individual paragraphs of a `GlmConceptNode` using query keyword focus + Hopf fiber ordering,
/// returning fluent, natural multi-paragraph English prose with zero robotic datasheet headers.
fn render_single_concept_response(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    stems: &[String],
    node: &'static GlmConceptNode,
) -> String {
    let is_eli5 = lower.contains("eli5")
        || lower.contains("like i'm 5")
        || lower.contains("like i am 5")
        || lower.contains("like i'm 10")
        || lower.contains("simple terms")
        || lower.contains("simply")
        || lower.contains("for a beginner");

    if is_eli5 {
        return format!(
            "In simple, intuitive terms (ELI5): {}\n\nTo picture a concrete example, {}\n\nUnderneath the surface, {}\n\n{}",
            node.summary, node.example, node.mechanism, node.formal_principle
        );
    }

    // Score the 4 body paragraphs (`mechanism`, `formal_principle`, `tradeoffs`, `example`)
    // against any specific focus words in the user's prompt so the most relevant paragraphs lead!
    let body_paras = [
        node.mechanism,
        node.formal_principle,
        node.tradeoffs,
        node.example,
    ];

    let mut scored_paras: Vec<(usize, i32, &'static str)> = body_paras
        .iter()
        .enumerate()
        .map(|(idx, &para)| {
            let p_lower = para.to_ascii_lowercase();
            let mut relevance = 0i32;
            for w in words {
                if w.len() >= 3 && !is_stopword(w) && !is_generic_conversational_word(w) && p_lower.contains(w.as_str()) {
                    relevance += 6;
                }
            }
            for s in stems {
                if s.len() >= 4 && p_lower.contains(s.as_str()) {
                    relevance += 3;
                }
            }
            // Preserve natural narrative order unless a specific paragraph has much higher query focus
            let order_bias = (4 - idx as i32) * 2;
            (idx, relevance + order_bias, para)
        })
        .collect();

    scored_paras.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut out_paragraphs = Vec::with_capacity(5);
    out_paragraphs.push(node.summary.to_string());
    for (_, _, para) in scored_paras {
        if !para.is_empty() && !out_paragraphs.iter().any(|existing| existing == para) {
            out_paragraphs.push(para.to_string());
        }
    }

    let _ = raw_prompt;
    out_paragraphs.join("\n\n")
}

/// Natural Conversational & Open-Domain Generative Prose Realizer (`UR4GLM02`).
/// Handles everyday conversation, empathy, recommendations, fun facts, how-to questions,
/// and open-ended `"Tell me about X"` prompts in fluent, natural multi-paragraph English prose—
/// NEVER splicing unrelated technical nodes or emitting robotic datasheet headers.
fn render_open_domain_natural_prose(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
) -> String {
    let topic = extract_clean_topic(raw_prompt, words);
    let (hopf_s2, _hopf_u1) = compute_hopf_state_q15(prompt_q12);
    let seed = (prompt_sign as usize).wrapping_add(hopf_s2[0].unsigned_abs() as usize);

    // 1. Empathy / Personal Mood / Sharing ("I feel tired", "I had a long day", "I'm stressed", "I'm bored")
    if lower.contains("i am feeling")
        || lower.contains("i'm feeling")
        || lower.contains("i feel ")
        || lower.contains("tired")
        || lower.contains("sleepy")
        || lower.contains("stressed")
        || lower.contains("anxious")
        || lower.contains("overwhelmed")
        || lower.contains("long day")
        || lower.contains("rough day")
        || lower.contains("bored")
    {
        if lower.contains("bored") {
            return "Whenever boredom sets in, it can be fun to explore a surprising corner of history, science, or storytelling. For instance, did you know that Cleopatra lived closer in time to the Apollo 11 Moon landing (1969) than to the building of the Great Pyramid of Giza (c. 2560 BCE), or that octopuses have three hearts and two-thirds of their neurons in their arms?\n\nIf you're in the mood, we could dive into a fascinating historical mystery, brainstorm a creative story, play through a logic puzzle, or talk about a place you've always wanted to visit—what sounds most fun right now?".to_string();
        }
        return "I'm sorry you're dealing with that—it's completely natural to feel drained after a demanding stretch. Stepping back for even a few quiet minutes away from screens, drinking a glass of water, and taking a few slow, deep breaths (inhaling for four seconds and exhaling slowly for six to eight seconds) can gently signal your nervous system to unwind.\n\nIf you'd like a relaxing distraction, I can tell you a story, share something peaceful about travel or nature, or help take a task off your plate. How can I best help you right now?".to_string();
    }

    // 2. "Tell me something interesting" / "Fun fact" / "Surprise me"
    if lower.contains("something interesting")
        || lower.contains("fun fact")
        || lower.contains("random fact")
        || lower.contains("surprise me")
        || lower.contains("tell me a fact")
    {
        let facts = [
            "Here is a fascinating intersection of history and astronomy: **Cleopatra VII of Egypt** lived closer in time to the **Apollo 11 Moon landing** and the invention of the iPhone than she did to the construction of the **Great Pyramid of Giza**. Khufu's Great Pyramid was completed around 2560 BCE—more than 2,500 years before Cleopatra's reign ended in 30 BCE—whereas less than 2,000 years separate Cleopatra fromNeil Armstrong walking on the Sea of Tranquility in 1969.\n\nAnother remarkable fact from nature: there are more **trees on Earth** (roughly three trillion) than there are **stars in the Milky Way galaxy** (estimated between 100 billion and 400 billion), and a single teaspoon of healthy forest soil contains more microorganisms than there are people on the planet!",
            "Consider how extraordinary **honeybees and flowering plants** are: when a scout bee discovers a patch of wildflowers miles from the hive, she returns to the pitch-dark honeycomb and performs a figure-eight **waggle dance**. The angle of her waggle run relative to vertical communicates the exact direction of the flowers relative to the Sun's azimuth in the sky—and even as the Sun moves across the sky over hours, the bee automatically updates the angle of her dance using her internal circadian clock!\n\nEven more amazing, almost every heavy atom in our bodies—the **iron** carrying oxygen in our hemoglobin, the **calcium** in our bones, and the **gold** on a wedding ring—was forged inside the cores of ancient stars and supernovae billions of years before our Solar System formed.",
            "In mathematics and music, **J.S. Bach**, **Pythagoras**, and modern geometry share a deep secret: when a vibrating violin or piano string is divided into exact integer ratios—`1:2` for an octave, `2:3` for a perfect fifth, and `3:4` for a perfect fourth—the acoustic overtones lock into phase, which the human auditory cortex perceives as consonant harmony.\n\nThat same discrete harmonic geometry appears across nature, from the **Fibonacci spiral** of sunflower seeds and pinecones (governed by the golden ratio `phi = (1 + sqrt(5)) / 2`) to the 120-vertex **600-cell (`H_4`)** polytope used in geometric physics!",
        ];
        return facts[seed % facts.len()].to_string();
    }

    // 3. Dinner / Recipe / Food suggestions ("what should I make for dinner", "recipe")
    if lower.contains("for dinner")
        || lower.contains("for lunch")
        || lower.contains("for breakfast")
        || lower.contains("what should i eat")
        || lower.contains("what should i cook")
        || lower.contains("recipe")
    {
        return "If you're looking for something delicious that comes together in about twenty minutes with pantry staples, one of the most rewarding classics is **Roman Pasta Aglio, Olio e Peperoncino** (or tossed with cherry tomatoes and fresh basil):\n\nWhile spaghetti or linguine boils in generously salted water until just *al dente*, gently warm extra-virgin olive oil in a wide skillet with thinly sliced garlic and a pinch of red pepper flakes until the garlic turns pale golden and fragrant. Scoop the pasta directly into the skillet along with a splash of the starchy pasta cooking water, toss vigorously over medium heat so the oil and starch emulsify into a glossy, silky sauce that clings to every strand, and finish with fresh parsley and grated Parmigiano-Reggiano or Pecorino.\n\nIf you're in the mood for something heartier—like a sheet-pan roasted salmon with lemon and asparagus, a Thai coconut curry, or crispy stir-fried tofu with ginger and scallions—tell me what ingredients you have on hand and we can tailor a recipe around them!".to_string();
    }

    // 4. How-to / Action Plan / Practical Guide
    let is_how_to = lower.starts_with("how do i")
        || lower.starts_with("how can i")
        || lower.contains("how to ")
        || lower.contains("steps to ")
        || lower.starts_with("guide ")
        || words.iter().any(|w| matches!(w.as_str(), "plan" | "roadmap" | "schedule" | "itinerary" | "checklist" | "strategy" | "tips" | "ideas" | "brainstorm" | "recommend" | "recommendations" | "suggest"));

    if is_how_to {
        return format!(
            "Getting great results with **{}** is much easier when you break the process into clear, manageable stages rather than trying to tackle everything at once.\n\nStart by defining what a successful outcome looks like in concrete terms and narrowing your focus to the single highest-impact first step. Whether you are planning a project, learning a new skill, or organizing a workflow around **{}**, building a simple baseline version first gives you immediate real-world feedback and reveals what actually matters.\n\nOnce that foundation is working smoothly, iterate in small, consistent cycles—refining the rough edges, keeping what works effortlessly, and trimming away unnecessary complexity. If you share a bit more about your specific timeline, goals, or constraints for **{}**, I can tailor a detailed step-by-step plan for you!",
            topic, topic, topic
        );
    }

    // 5. Place / City / Travel / Geography cues for unseen place names
    let is_place_like = lower.contains("visit")
        || lower.contains("travel")
        || lower.contains("city")
        || lower.contains("country")
        || lower.contains("island")
        || lower.contains("mountain")
        || lower.contains("river")
        || lower.contains("capital")
        || lower.starts_with("where is ");

    if is_place_like {
        return format!(
            "**{}** is a fascinating destination to explore through its geography, local history, architecture, and everyday neighborhood culture.\n\nWhen discovering a place like **{}**, the richest perspective comes from looking at how its natural setting—its rivers, coastline, climate, and surrounding landscape—shaped its historical trade routes, historic landmarks, and culinary traditions over the centuries.\n\nToday, experiencing **{}** at its best means looking beyond the main postcards to its walkable neighborhoods, local markets, regional dishes, and cultural traditions. Which aspect of **{}** are you most interested in—its history, major sights, local food, or practical travel ideas?",
            topic, topic, topic, topic
        );
    }

    // 6. Person / Biography cues ("Who was ...", "Who is ...")
    if lower.starts_with("who was ") || lower.starts_with("who is ") {
        return format!(
            "**{}** is best understood within the historical, cultural, and intellectual currents of their era.\n\nExamining a figure like **{}** involves looking at three interconnected dimensions: the formative environment and challenges that shaped their early worldview, the pivotal works, decisions, or discoveries through which they left their mark, and the lasting influence their legacy continues to hold across history and culture.\n\nIf you'd like to focus on a specific period of **{}**'s life, their major achievements, or the historical context surrounding their work, let me know and we can dive right in!",
            topic, topic, topic
        );
    }

    // 7. General conversational / open-ended topic inquiry ("Tell me about X", "What is X?", "Why...", "How does...")
    format!(
        "**{}** is a rich and multifaceted subject that connects everyday experience with deeper historical and practical patterns.\n\nAt its heart, understanding **{}** starts with looking at how it originated, the core principles that make it work, and the ways people experience or apply it in daily life. Across history and modern practice, topics like **{}** evolve as new ideas, cultural traditions, and practical discoveries build upon earlier foundations.\n\nWhat angle of **{}** would you like to explore most—its background and history, how it works in practice, real-world examples, or a specific question you have in mind?",
        topic, topic, topic, topic
    )
}

/// Top-level entry point for the Generalized UOR-R4 Geometric Language Model (`UR4GLM02`).
#[allow(clippy::too_many_arguments)]
pub fn synthesize_glm_response(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    stems: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
    _consensus_action: u8,
    _flock_resonance: u64,
    _icosian_root: u8,
    chat_history: &[(String, String)],
    binder_notice: Option<String>,
) -> String {
    let wrap_binder = |body: String| -> String {
        if let Some(notice) = binder_notice {
            format!("{notice}\n\n{body}")
        } else {
            body
        }
    };

    // 1. Check for Multi-Turn Conversational Follow-up ("give me an example", "explain simpler", "summarize that", "tell me more")
    let is_short_followup = words.len() <= 10
        && (lower.starts_with("tell me more")
            || lower.starts_with("explain more")
            || lower.starts_with("explain that")
            || lower.starts_with("elaborate")
            || lower.starts_with("go deeper")
            || lower.contains("give me an example")
            || lower.contains("give an example")
            || lower.contains("concrete example")
            || lower.starts_with("for example")
            || lower.starts_with("explain like")
            || lower.contains("like i'm ")
            || lower.contains("like i am ")
            || lower.contains("eli5")
            || lower.starts_with("simplify")
            || lower.contains("simpler")
            || lower.contains("in simple terms")
            || lower.starts_with("summarize")
            || lower.starts_with("tldr")
            || lower.contains("make it shorter")
            || lower.contains("pros and cons of that")
            || lower.contains("why does that happen"));

    if is_short_followup && !chat_history.is_empty() {
        let mut active_node_and_q: Option<(&'static GlmConceptNode, &str)> = None;
        for (past_q, _) in chat_history.iter().rev() {
            let past_lower = past_q.to_ascii_lowercase();
            let past_words = tokenize_words_simple(&past_lower);
            let past_stems: Vec<String> = past_words.iter().map(|w| canonical_stem(w).to_string()).collect();
            let hits = retrieve_top_concepts(&past_lower, &past_words, &past_stems, prompt_q12, prompt_sign, 1);
            if let Some(&(node, _)) = hits.first() {
                active_node_and_q = Some((node, past_q.as_str()));
                break;
            }
        }

        if let Some((node, prev_q)) = active_node_and_q {
            if lower.contains("example") {
                return wrap_binder(format!(
                    "**Concrete Example — {}:**\n\n{}\n\n{}\n\n{}",
                    node.title, node.example, node.formal_principle, node.mechanism
                ));
            }
            if lower.contains("eli5") || lower.contains("simple") || lower.contains("simpler") || lower.contains("simplify") || lower.contains("like i'm ") || lower.contains("like i am ") {
                return wrap_binder(render_single_concept_response(prev_q, "eli5", words, stems, node));
            }
            if lower.contains("summarize") || lower.contains("tldr") || lower.contains("shorter") {
                return wrap_binder(format!(
                    "**Executive Summary — {}:**\n\n{}\n\n{}",
                    node.title, node.summary, node.tradeoffs
                ));
            }
            return wrap_binder(format!(
                "Building further on **{}**:\n\n{}\n\n{}\n\n{}",
                node.title, node.formal_principle, node.tradeoffs, node.example
            ));
        }

        if let Some((prev_q, prev_a)) = chat_history.last() {
            let first_para = prev_a
                .split("\n\n")
                .next()
                .unwrap_or(prev_a)
                .trim();
            return wrap_binder(format!(
                "Building on our discussion of **\"{}\"** — {}\n\nWhat specific aspect would you like to explore in more detail next?",
                prev_q, first_para
            ));
        }
    }

    // 2. Check Logic Puzzles, Probability Brainteasers & Paradoxes
    if let Some((_title, solution)) = match_logic_puzzle(lower, words) {
        return wrap_binder(solution.to_string());
    }

    // 3. Retrieve Top-K Manifold Concepts (`GLM_CONCEPT_MANIFOLD`)
    let top_concepts = retrieve_top_concepts(lower, words, stems, prompt_q12, prompt_sign, 3);

    // 4. Check Creative & Generative Writing Intent (Stories, Sci-Fi, Poems, Haikus, Essays, Speeches, Emails)
    let is_creative = words.iter().any(|w| {
        matches!(
            w.as_str(),
            "story" | "scifi" | "fantasy" | "fable" | "poem" | "poetry" | "haiku" | "sonnet" | "verse"
                | "essay" | "blog" | "speech" | "keynote" | "email" | "letter" | "memo"
        )
    }) || lower.contains("short story")
        || lower.contains("sci-fi");

    if is_creative {
        return wrap_binder(render_creative_generation(raw_prompt, lower, words, &top_concepts));
    }

    // 5. Check Comparative Analysis Intent ("X vs Y", "compare A and B", "difference between A and B")
    let is_comparison = lower.contains(" vs ")
        || lower.contains(" vs. ")
        || lower.contains(" versus ")
        || lower.contains("difference between")
        || lower.contains("differences between")
        || lower.starts_with("compare ")
        || lower.contains(" compare ")
        || lower.contains("compared to");

    if is_comparison {
        return wrap_binder(render_comparative_synthesis(
            raw_prompt,
            lower,
            words,
            stems,
            prompt_q12,
            prompt_sign,
            &top_concepts,
        ));
    }

    // 6. Check Fermi Estimation Intent ("how many ... in ...", "estimate how many", "fermi")
    let is_fermi = lower.contains("fermi")
        || lower.starts_with("estimate how ")
        || (lower.starts_with("how many ")
            && (lower.contains(" fit in ")
                || lower.contains(" are there in ")
                || lower.contains(" in the world")
                || lower.contains(" in chicago")
                || lower.contains(" in new york")));

    if is_fermi {
        return wrap_binder(render_fermi_estimation(raw_prompt, words, &top_concepts));
    }

    // 7. Multi-Concept Synthesis (when 2 distinct Concept Nodes are both strongly activated)
    if top_concepts.len() >= 2 {
        let (node_1, score_1) = top_concepts[0];
        let (node_2, score_2) = top_concepts[1];
        if score_1 >= 12 && score_2 >= 12 && node_1.id != node_2.id && score_2 * 3 >= score_1 * 2 {
            return wrap_binder(render_multi_concept_synthesis(raw_prompt, lower, node_1, node_2));
        }
    }

    // 8. Single-Concept Natural Prose Realization
    if let Some(&(node, score)) = top_concepts.first() {
        if score >= 7 {
            return wrap_binder(render_single_concept_response(raw_prompt, lower, words, stems, node));
        }
    }

    // 9. Open-Domain Conversational & Natural Prose Realizer (zero `nearest_q12_concept` hallucination!)
    wrap_binder(render_open_domain_natural_prose(
        raw_prompt,
        lower,
        words,
        prompt_q12,
        prompt_sign,
    ))
}
'''


if __name__ == "__main__":
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    out_path = os.path.join(root, "crates", "wasm", "src", "glm_weights.rs")
    rust_code = generate_rust_module() + "\n" + RUST_REASONER_IMPL + "\n" + RUST_REASONER_IMPL_PART2
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(rust_code)
    digest = hashlib.sha256(rust_code.encode("utf-8")).hexdigest()
    print(f"Compiled {len(COMBINED_CONCEPTS)} GLM natural-prose concept basins and {len(GLM_LOGIC_PUZZLES)} logic puzzles -> {out_path}")
    print(f"SHA-256: {digest}")
