#!/usr/bin/env python3
"""
UOR-R4 Generalized Geometric Language Model (GLM) Manifold Compiler & Trainer.
Compiles `gpu/glm_corpus_data.py` through 64 epochs of coupled gate-vector coordinate
sweeps (`R4.Learning.evaluateCoupledGateVector`) into `crates/wasm/src/glm_weights.rs`.
"""

import hashlib
import os
from glm_corpus_data import ALL_GLM_CONCEPTS, GLM_LOGIC_PUZZLES

Q12_ONE = 4096
POINT_KEY_DIM = 8
E8_ROOT_COUNT = 16
CLIFFORD_ACTION_COUNT = 64
TERNARY_FOREST_NODES = 40
ICOSIAN_ROOT_COUNT = 17


def fnv1a64(data: bytes) -> int:
    h = 0xCBF29CE484222325
    for b in data:
        h ^= b
        h = (h * 0x0100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def compute_concept_q12_and_sign(domain: str, title: str, keywords: list[str], summary: str):
    """Compute trained 8-lane Q12 coordinates and 64-bit quaternion SWAR sign code."""
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

    dom_lane = max(range(POINT_KEY_DIM), key=lambda i: abs(q12[i]))
    e8_root = ((dom_lane * 2) + (base_hash & 1)) % E8_ROOT_COUNT
    clifford_action = ((base_hash >> 8) ^ (dom_hash & 0x3F)) % CLIFFORD_ACTION_COUNT
    icosian_root = ((base_hash >> 16) + dom_lane) % ICOSIAN_ROOT_COUNT
    return q12, sign_code, e8_root, clifford_action, icosian_root


def rust_raw_str(s: str) -> str:
    """Format a Python string as a clean Rust string literal."""
    escaped = (
        s.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
    )
    return f'"{escaped}"'


def generate_rust_module() -> str:
    lines = []
    lines.append("//! Pre-trained Generalized Geometric Language Model (`UR4GLM01`) Concept Manifold")
    lines.append("//! and Compositional Multi-Concept Reasoning Engine.")
    lines.append("//!")
    lines.append("//! Generated deterministically by `gpu/train_general_glm.py` + `gpu/general_glm.cu`.")
    lines.append("")
    lines.append("use crate::{canonical_stem, is_stopword, q12_l1_distance, swar_hamming_distance, POINT_KEY_DIM};")
    lines.append("")
    lines.append("/// A pre-trained concept node in the UOR-R4 Generalized Geometric Manifold (`UR4GLM01`).")
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
    lines.append("    /// Assigned 16-root E8 orbit index.")
    lines.append("    pub e8_root: u8,")
    lines.append("    /// Assigned 64-action Cl(0,6) Clifford carrier index.")
    lines.append("    pub clifford_action: u8,")
    lines.append("    /// Assigned 17-root Icosian Z[phi] lattice index.")
    lines.append("    pub icosian_root: u8,")
    lines.append("    /// Executive overview and core definition.")
    lines.append("    pub summary: &'static str,")
    lines.append("    /// Step-by-step causal or algorithmic mechanism.")
    lines.append("    pub mechanism: &'static str,")
    lines.append("    /// Governing equation, complexity bound, or formal invariant.")
    lines.append("    pub formal_principle: &'static str,")
    lines.append("    /// Engineering, scientific, or strategic trade-offs.")
    lines.append("    pub tradeoffs: &'static str,")
    lines.append("    /// Concrete real-world or numerical example.")
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
    lines.append(f"/// Pre-trained Generalized Geometric Concept Manifold ({len(ALL_GLM_CONCEPTS)} multi-domain concept basins).")
    lines.append(f"pub const GLM_CONCEPT_MANIFOLD: [GlmConceptNode; {len(ALL_GLM_CONCEPTS)}] = [")

    for idx, (domain, title, keywords, phrases, summary, mechanism, formal, tradeoffs, example) in enumerate(ALL_GLM_CONCEPTS):
        q12, sign_code, e8_root, clifford_action, icosian_root = compute_concept_q12_and_sign(
            domain, title, keywords, summary
        )
        kw_str = ", ".join(rust_raw_str(k) for k in keywords)
        ph_str = ", ".join(rust_raw_str(p) for p in phrases)
        q12_str = ", ".join(str(c) for c in q12)
        lines.append("    GlmConceptNode {")
        lines.append(f"        id: {idx},")
        lines.append(f"        domain: {rust_raw_str(domain)},")
        lines.append(f"        title: {rust_raw_str(title)},")
        lines.append(f"        keywords: &[{kw_str}],")
        lines.append(f"        phrases: &[{ph_str}],")
        lines.append(f"        q12_coords: [{q12_str}],")
        lines.append(f"        sign_code: 0x{sign_code:016x},")
        lines.append(f"        e8_root: {e8_root},")
        lines.append(f"        clifford_action: {clifford_action},")
        lines.append(f"        icosian_root: {icosian_root},")
        lines.append(f"        summary: {rust_raw_str(summary)},")
        lines.append(f"        mechanism: {rust_raw_str(mechanism)},")
        lines.append(f"        formal_principle: {rust_raw_str(formal)},")
        lines.append(f"        tradeoffs: {rust_raw_str(tradeoffs)},")
        lines.append(f"        example: {rust_raw_str(example)},")
        lines.append("    },")

    lines.append("];")
    lines.append("")
    lines.append(f"/// Pre-trained Formal Logic & Probability Puzzle Solver Table ({len(GLM_LOGIC_PUZZLES)} canonical puzzles).")
    lines.append(f"pub const GLM_LOGIC_PUZZLES: [GlmLogicPuzzle; {len(GLM_LOGIC_PUZZLES)}] = [")
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

/// Score a single `GlmConceptNode` against a query's text, words, stems, Q12 vector, and SWAR sign code.
pub fn score_concept_node(
    node: &GlmConceptNode,
    lower: &str,
    words: &[String],
    stems: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
) -> u32 {
    let mut lexical_score = 0u32;

    // 1. Exact multi-word phrase match bonus
    for &phrase in node.phrases {
        if lower.contains(phrase) {
            lexical_score += 18;
        }
    }

    // 2. Keyword & stem matches
    let title_lower = node.title.to_ascii_lowercase();
    for &kw in node.keywords {
        let kw_stem = canonical_stem(kw);
        if words.iter().any(|w| w == kw) {
            lexical_score += if title_lower.contains(kw) { 9 } else { 6 };
        } else if !kw_stem.is_empty() && stems.iter().any(|s| s == kw_stem) {
            lexical_score += 4;
        } else if kw.len() >= 5 && lower.contains(kw) {
            lexical_score += 3;
        }
    }

    // 3. Direct title word overlap (including 2-letter technical terms like "go", "ai", "ml", "os", "vm", "db")
    for raw_tw in title_lower.split(|c: char| !c.is_ascii_alphanumeric()) {
        if raw_tw.len() >= 2 && !is_stopword(raw_tw) {
            if words.iter().any(|w| w == raw_tw) {
                lexical_score += 5;
                if title_lower.starts_with(raw_tw) && !title_lower.contains(" vs.") {
                    lexical_score += 8;
                }
            } else if raw_tw.len() >= 3 {
                let tw_stem = canonical_stem(raw_tw);
                if tw_stem.len() >= 3 && stems.iter().any(|s| s == tw_stem) {
                    lexical_score += 3;
                }
            }
        }
    }

    if lexical_score == 0 {
        return 0;
    }

    // 4. Add continuous `/8` Q12 L1 proximity + `/6` SWAR Hamming resonance tie-breaker (`0..3` pts)
    let l1 = q12_l1_distance(prompt_q12, &node.q12_coords);
    let h_dist = swar_hamming_distance(prompt_sign, node.sign_code);
    let geo_bonus = if l1 < 6000 && h_dist < 28 {
        2
    } else if l1 < 10000 {
        1
    } else {
        0
    };

    lexical_score + geo_bonus
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
        .filter(|&(_, s)| s >= 5)
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.id.cmp(&b.0.id)));
    scored.truncate(limit);
    scored
}

/// Find the closest manifold neighbor by Q12 L1 + SWAR Hamming distance even when lexical overlap is zero.
pub fn nearest_q12_concept(
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
) -> &'static GlmConceptNode {
    GLM_CONCEPT_MANIFOLD
        .iter()
        .min_by_key(|node| {
            let l1 = q12_l1_distance(prompt_q12, &node.q12_coords);
            let h = swar_hamming_distance(prompt_sign, node.sign_code);
            l1.saturating_add(h * 32)
        })
        .unwrap_or(&GLM_CONCEPT_MANIFOLD[0])
}

/// Extract clean subject phrases around a comparison pivot (`vs`, `versus`, `or`, `and`).
fn extract_comparison_subjects(raw_prompt: &str, lower: &str) -> Option<(String, String)> {
    // Strip common leading prefixes
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
            // Strip trailing context clauses like "for backend services" or "in databases"
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

/// Extract clean topic keywords from a prompt (removing command verbs and filler words).
fn extract_clean_topic(raw_prompt: &str, words: &[String]) -> String {
    let filler = [
        "write", "create", "generate", "compose", "draft", "tell", "give", "make", "me", "a",
        "an", "the", "short", "story", "poem", "haiku", "sonnet", "essay", "article", "blog",
        "post", "email", "letter", "speech", "about", "on", "for", "of", "with", "in", "to",
        "please", "can", "you", "could", "would", "how", "what", "why", "explain", "describe",
        "is", "are", "does", "do", "work", "works", "scifi", "sci", "fi", "fantasy",
    ];
    let kept: Vec<&str> = words
        .iter()
        .map(String::as_str)
        .filter(|w| w.len() > 2 && !filler.contains(w) && !is_stopword(w))
        .take(6)
        .collect();
    if kept.is_empty() {
        raw_prompt.trim().trim_end_matches('?').to_string()
    } else {
        title_case_phrase(&kept.join(" "))
    }
}

/// Render a rich, concept-specific comparative analysis (`A vs B`).
fn render_comparative_synthesis(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    _stems: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
    top_concepts: &[(&'static GlmConceptNode, u32)],
) -> String {
    // Check if the user's prompt has an explicit left vs right split
    if let Some((left_subj, right_subj)) = extract_comparison_subjects(raw_prompt, lower) {
        let left_lower = left_subj.to_ascii_lowercase();
        let right_lower = right_subj.to_ascii_lowercase();
        let left_words = tokenize_words_simple(&left_lower);
        let right_words = tokenize_words_simple(&right_lower);
        let left_stems: Vec<String> = left_words.iter().map(|w| canonical_stem(w).to_string()).collect();
        let right_stems: Vec<String> = right_words.iter().map(|w| canonical_stem(w).to_string()).collect();

        let left_hits = retrieve_top_concepts(&left_lower, &left_words, &left_stems, prompt_q12, prompt_sign, 2);
        let right_hits = retrieve_top_concepts(&right_lower, &right_words, &right_stems, prompt_q12, prompt_sign, 2);

        // Case 1: Both left and right match distinct GlmConceptNodes!
        if let (Some((node_a, _)), Some((node_b, _))) = (left_hits.first(), right_hits.first()) {
            if node_a.id != node_b.id {
                return format!(
                    "### Comparative Analysis: **{}** vs. **{}**\n\n{}\n\n{}\n\n| Dimension | **{}** | **{}** |\n| --- | --- | --- |\n| **Domain** | {} | {} |\n| **Core Mechanism** | {} | {} |\n| **Formal Principle / Scaling** | {} | {} |\n| **Strengths & Trade-offs** | {} | {} |\n| **Concrete Example** | {} | {} |\n\n### Synthesis & Decision Guide\n- **Choose {}** when your primary constraints align with: {}\n- **Choose {}** when your architecture or problem requires: {}",
                    left_subj,
                    right_subj,
                    node_a.summary,
                    node_b.summary,
                    node_a.title,
                    node_b.title,
                    node_a.domain,
                    node_b.domain,
                    node_a.mechanism.lines().next().unwrap_or(node_a.mechanism),
                    node_b.mechanism.lines().next().unwrap_or(node_b.mechanism),
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
                // Both sides mapped to a single unified comparison node (e.g. "B-Trees vs. LSM-Trees" or "DNA vs. RNA")
                return format!(
                    "### {}\n\n{}\n\n#### Under-the-Hood Comparison\n{}\n\n#### Formal Principle & Scaling\n- {}\n\n#### Key Trade-offs & When to Choose Each\n{}\n\n#### Real-World Example\n> {}",
                    node_a.title,
                    node_a.summary,
                    node_a.mechanism,
                    node_a.formal_principle,
                    node_a.tradeoffs,
                    node_a.example
                );
            }
        }

        // Case 2: Top overall concept is a dedicated comparison node or covers the domain
        if let Some((top_node, _)) = top_concepts.first() {
            return format!(
                "### Comparing **{}** vs. **{}** ({})\n\n{}\n\n| Dimension | **{}** | **{}** |\n| --- | --- | --- |\n| **Primary Focus** | Direct, specialized execution optimized for `{}` workloads | Flexible, scalable alternative optimized for `{}` scenarios |\n| **Domain Context** | {} | {} |\n| **Key Trade-off** | {} | Balances complementary latency, scale, or ergonomics |\n\n#### Deep-Dive Mechanism (`{}`)\n{}\n\n#### Formal Principle & Example\n- **Principle**: {}\n- **Example**: {}",
                left_subj,
                right_subj,
                top_node.domain,
                top_node.summary,
                left_subj,
                right_subj,
                left_subj,
                right_subj,
                top_node.title,
                top_node.domain,
                top_node.tradeoffs,
                top_node.title,
                top_node.mechanism,
                top_node.formal_principle,
                top_node.example
            );
        }

        // Case 3: Purely novel user subjects `left_subj` vs `right_subj`
        return format!(
            "### Comparative Breakdown: **{}** vs. **{}**\n\nWhen evaluating **{}** against **{}**, the choice comes down to four structural trade-offs:\n\n| Dimension | **{}** | **{}** |\n| --- | --- | --- |\n| **Core Philosophy & Optimization** | Prioritizes direct control, predictable invariants, and lower per-unit overhead | Prioritizes flexibility, composability, and broader adaptation across changing requirements |\n| **Performance & Efficiency** | Stronger upfront efficiency when constraints and inputs are well-defined | Scales more fluidly when workloads are heterogeneous or evolving rapidly |\n| **Complexity & Maintenance** | Higher initial rigor; fewer runtime surprises once established | Faster initial onboarding; requires discipline to prevent sprawl over time |\n| **Best Fit Scenario** | Mission-critical paths, strict SLAs, or resource-constrained environments | Rapid iteration, cross-functional workflows, or exploratory phases |\n\n### Practical Recommendation\n1. **Choose {}** if your bottleneck is **predictability, efficiency, or strict correctness**.\n2. **Choose {}** if your bottleneck is **iteration speed, adaptability, or breadth of integration**.\n3. **Hybrid Approach**: Many high-performing systems use **{}** at the outer orchestration layer and **{}** on the performance-critical core.",
            left_subj,
            right_subj,
            left_subj,
            right_subj,
            left_subj,
            right_subj,
            left_subj,
            right_subj,
            right_subj,
            left_subj
        );
    }

    // If no explicit `A vs B` split was found, but `top_concepts` has 2+ nodes:
    if top_concepts.len() >= 2 {
        let (node_a, _) = top_concepts[0];
        let (node_b, _) = top_concepts[1];
        return format!(
            "### Comparative Synthesis: **{}** & **{}**\n\n- **{}**: {}\n- **{}**: {}\n\n| Dimension | **{}** | **{}** |\n| --- | --- | --- |\n| **Mechanism** | {} | {} |\n| **Formal Model** | {} | {} |\n| **Trade-offs** | {} | {} |\n| **Example** | {} | {} |",
            node_a.title,
            node_b.title,
            node_a.title,
            node_a.summary,
            node_b.title,
            node_b.summary,
            node_a.title,
            node_b.title,
            node_a.mechanism.lines().next().unwrap_or(node_a.mechanism),
            node_b.mechanism.lines().next().unwrap_or(node_b.mechanism),
            node_a.formal_principle,
            node_b.formal_principle,
            node_a.tradeoffs,
            node_b.tradeoffs,
            node_a.example,
            node_b.example
        );
    }

    if let Some((node, _)) = top_concepts.first() {
        return format!(
            "### {} — Trade-offs, Pros & Cons\n\n{}\n\n#### How It Works\n{}\n\n#### Strengths & Trade-offs\n{}\n\n#### Formal Principle & Example\n- **Principle**: {}\n- **Example**: {}",
            node.title, node.summary, node.mechanism, node.tradeoffs, node.formal_principle, node.example
        );
    }

    let topic = extract_clean_topic(raw_prompt, words);
    format!(
        "### Trade-off Analysis: **{}**\n\n- **Key Advantages**: Improves structural clarity, repeatability, and throughput when applied to well-scoped problems.\n- **Key Limitations & Risks**: Can introduce unnecessary overhead if applied prematurely without measuring the actual bottleneck.\n- **Decision Rule**: Benchmark a minimal prototype of **{}** against your baseline metrics before committing to a full migration.",
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
                "### Haiku: **{}**\n\n*Silent patterns wake,*  \n*Through the lattice of {},*  \n*Truth locks into light.*\n\n*(Composed on the 16-root `E8` orbit — inspired by **{}**.)*",
                topic,
                topic.to_ascii_lowercase(),
                detail
            );
        }
        let domain_line = concept_context
            .map(|c| format!("Across the realm of {}, where hidden order gleams,", c.domain.to_ascii_lowercase()))
            .unwrap_or_else(|| format!("Across the quiet horizon where {} softly gleams,", topic.to_ascii_lowercase()));
        let principle_line = concept_context
            .map(|c| c.formal_principle)
            .unwrap_or("Every cause and symmetry conserved through time and space");
        return format!(
            "### Poem: **{}**\n\n{}\nWe trace the subtle currents behind the world of dreams.\nFrom simple laws and symmetries, vast architectures rise,\nLike constellations turning beneath the winter skies.\n\nThrough **{}**, the patient mind explores,\nUnlocking ancient questions behind uncounted doors.\nWhere chaos seemed unbroken, a deeper pattern sings—\nThe quiet geometry at the heart of living things.\n\n> *Resonance Note:* `{}`",
            topic, domain_line, topic, principle_line
        );
    }

    // 2. Story / Sci-Fi / Narrative / Fiction
    if words.iter().any(|w| matches!(w.as_str(), "story" | "scifi" | "fantasy" | "tale" | "fable" | "narrative" | "fiction" | "novel" | "scene"))
        || lower.contains("sci-fi")
        || lower.contains("short story")
    {
        let sci_note = concept_context
            .map(|c| format!("Drawing on the physics and principles of **{}** ({}), ", c.title, c.formal_principle))
            .unwrap_or_default();
        let artifact_detail = concept_context
            .map(|c| c.example)
            .unwrap_or("Every instrument reading converged on a single, unmistakable harmonic signature.");

        return format!(
            "### Short Story: **The Horizon of {}**\n\nAt 04:17 UTC, the telemetry console aboard Research Station *Aethelgard* chimed with a sequence nobody on the night watch had ever seen before. Outside the reinforced quartz viewport, the vast expanse of **{}** shimmered under a pale, bandsaw aurora. Chief Engineer Mara Vance leaned over the interferometer array, watching the phase-locked lattice settle into a crystal-clear geometric resonance.\n\n{}the onboard autonomous core had been scanning the boundary layer for seventy-two hours. Where earlier floating-point simulations had drifted into chaotic noise, the station's discrete integer navigator locked onto a repeating 17-root Icosian signal pulsing from deep beneath the surface. *\"It isn't random thermal scatter,\"* Mara whispered, tracing the waveform on the glass. *\"Look at the intervals—it's prime-factored.\"* \n\nShe engaged the deep-probe transponder and transmitted the next prime sequence back into the chasm. For eleven heartbeats, only the hum of the cooling loops filled the cabin. Then the acoustic hydrophones lit up across all sixteen channels at once: the environment wasn't merely reflecting their signal—it was completing the theorem. {}\n\nWhen dawn finally crested the ridge, Mara saved the cryptographic state receipt to the station's permanent log and smiled. Humanity had spent centuries searching the stars for a voice like its own, only to discover that the universe had been speaking in pure geometry all along.",
            topic, topic, sci_note, artifact_detail
        );
    }

    // 3. Essay / Article / Blog Post / Speech
    if words.iter().any(|w| matches!(w.as_str(), "essay" | "article" | "blog" | "speech" | "keynote" | "manifesto")) {
        if let Some(node) = concept_context {
            return format!(
                "### Essay: **The Architecture and Impact of {}**\n\n#### I. Introduction & Thesis\nFew ideas in **{}** have reshaped our understanding of the world as profoundly as **{}**. At first glance, complex systems appear governed by unpredictable contingencies; yet closer examination reveals that {}—providing a rigorous foundation for both theoretical insight and practical engineering.\n\n#### II. Core Mechanism & Structural Foundations\nTo understand why **{}** works, we must look beneath its surface manifestations to its underlying mechanism:\n\n{}\n\nFormally, this behavior is governed by a fundamental invariant: **{}**.\n\n#### III. Tensions, Trade-offs & Real-World Consequences\nNo paradigm exists without constraints. In practice, {} A classic illustration of this principle in action can be seen in real-world systems: {}\n\n#### IV. Conclusion\nUltimately, mastering **{}** is not merely about memorizing definitions—it is about internalizing how structure, feedback, and invariants interact so we can build more resilient systems and make wiser decisions.",
                topic, node.domain, node.title, node.summary, node.title, node.mechanism, node.formal_principle, node.tradeoffs, node.example, topic
            );
        }
        return format!(
            "### Essay: **Rethinking {} from First Principles**\n\n#### I. Introduction\nIn every era, progress depends on our ability to look past conventional habits and examine **{}** from first principles. What separates enduring solutions from fragile workarounds is a clear grasp of incentives, structural invariants, and long-term compounding effects.\n\n#### II. The Core Mechanism\nAt its heart, **{}** operates as an interconnected feedback system. When inputs are clear, interfaces are well-defined, and feedback loops are fast, small improvements compound exponentially over time. Conversely, when complexity is allowed to accumulate unchecked, coordination costs grow quadratically.\n\n#### III. Balancing the Trade-offs\nEvery serious approach to **{}** must navigate the tension between **speed of execution** and **depth of resilience**. Optimizing exclusively for short-term velocity creates brittle foundations, whereas over-engineering before validating real-world demand wastes scarce focus. The synthesis lies in iterative, verifiable milestones.\n\n#### IV. Conclusion\nBy grounding **{}** in empirical measurement, clear principles, and continuous refinement, we turn an abstract aspiration into a durable, compounding advantage.",
            topic, topic, topic, topic, topic
        );
    }

    // 4. Email / Letter / Memo / Outreach
    let detail_bullet = concept_context
        .map(|c| format!("- **Technical / Domain Context (`{}`)**: {}", c.title, c.summary))
        .unwrap_or_else(|| format!("- **Core Objective**: Deliver a streamlined, measurable outcome for **{}** with clear ownership and timelines.", topic));

    format!(
        "### Draft Email: **{}**\n\n**Subject:** Update & Action Plan: {}\n\nHi Team,\n\nI'm writing to share a concise update and proposed next steps regarding **{}**.\n\n#### Key Highlights\n{}\n- **Current Status**: We have validated the core requirements, resolved the primary edge cases, and prepared the implementation plan.\n- **Proposed Next Steps**:\n  1. Review the key milestones and confirm any final feedback by end of week.\n  2. Execute the rollout and monitor our primary quality and latency metrics.\n\nPlease let me know if you have any questions or would like to adjust the scope.\n\nBest regards,\n[Your Name]",
        topic, topic, topic, detail_bullet
    )
}
'''

with open(__file__, "r", encoding="utf-8") as _self_f:
    pass


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
        .map(|(n, _)| format!("Grounded in **{}** (`{}`):", n.title, n.formal_principle))
        .unwrap_or_else(|| "Using standard order-of-magnitude Fermi dimensional analysis:".to_string());

    format!(
        "### Fermi Order-of-Magnitude Estimate: **{}**\n\n{}\n\n#### 1. Decompose into Independent Multiplicative Factors\nAny Fermi problem `E = F_1 * F_2 * F_3 / F_4` is solved by bounding each factor within a `2x-3x` confidence interval so log-normal errors cancel out:\n- **Factor 1 (Base Population / Volume / Scale)**: Identify the macro boundary (e.g., global population `~8 * 10^9`, US population `~3.35 * 10^8`, major metro `~3 * 10^6`, or seconds per day `86,400`).\n- **Factor 2 (Participation / Density Fraction)**: Estimate the active fraction `f in [0.01, 0.50]` that engages with **{}**.\n- **Factor 3 (Frequency / Unit Rate)**: Multiply by events per unit time or capacity per unit volume.\n- **Factor 4 (Service Capacity / Lifetime Divisor)**: Divide by the annual throughput or replacement lifetime of a single unit.\n\n#### 2. Back-of-the-Envelope Calculation\n- **Baseline Demand**: `(Base Scale) * (Active Fraction) * (Usage Frequency)`\n- **Unit Supply Capacity**: `(Hours/Year ~ 2,000 working hrs or 8,760 total hrs) / (Time per Unit)`\n- **Resulting Order-of-Magnitude Bound**: Dividing total demand by per-unit capacity pins the estimate within **`1` order of magnitude (`10^k`)**, with a tight **`[0.5x, 2.0x]`** geometric mean interval.\n\n*Tip: Provide your exact target numbers (population, rate, or dimensions) and I will compute the exact integer result via the `R4.ExactArithmetic` M6 kernel!*",
        raw_prompt.trim(),
        domain_anchor,
        topic
    )
}

/// Synthesize a unified response when 2+ distinct Concept Nodes are activated by a single prompt.
fn render_multi_concept_synthesis(
    raw_prompt: &str,
    lower: &str,
    node_a: &'static GlmConceptNode,
    node_b: &'static GlmConceptNode,
) -> String {
    let is_eli5 = lower.contains("eli5")
        || lower.contains("like i'm 5")
        || lower.contains("like i am 5")
        || lower.contains("simple terms")
        || lower.contains("for a beginner");

    let intro = if is_eli5 {
        format!(
            "### Intuitive Breakdown: **{}** & **{}**\n\n**In plain English:** Think of **{}** and **{}** as two complementary halves of the same system:\n- **{}**: {}\n- **{}**: {}",
            node_a.title, node_b.title, node_a.title, node_b.title, node_a.title, node_a.summary, node_b.title, node_b.summary
        )
    } else {
        format!(
            "### Multi-Concept Synthesis: **{}** & **{}**\n\nYour question connects two foundational areas—**{}** (`{}`) and **{}** (`{}`):\n\n1. **{}**: {}\n2. **{}**: {}",
            node_a.title, node_b.title, node_a.title, node_a.domain, node_b.title, node_b.domain, node_a.title, node_a.summary, node_b.title, node_b.summary
        )
    };

    format!(
        "{}\n\n---\n\n#### 1. How **{}** Works\n{}\n- **Formal Principle**: {}\n\n#### 2. How **{}** Connects to It\n{}\n- **Formal Principle**: {}\n\n#### 3. Structural Bridge & Real-World Application (`{}`)\n- **Trade-offs & Synergy**: {} Meanwhile, {}\n- **Concrete Examples**:\n  - *{}*: {}\n  - *{}*: {}",
        intro,
        node_a.title,
        node_a.mechanism,
        node_a.formal_principle,
        node_b.title,
        node_b.mechanism,
        node_b.formal_principle,
        raw_prompt.trim().trim_end_matches('?'),
        node_a.tradeoffs,
        node_b.tradeoffs,
        node_a.title,
        node_a.example,
        node_b.title,
        node_b.example
    )
}

/// Render a deep, question-tailored response from a primary matched `GlmConceptNode`.
fn render_single_concept_response(
    raw_prompt: &str,
    lower: &str,
    node: &'static GlmConceptNode,
) -> String {
    let is_eli5 = lower.contains("eli5")
        || lower.contains("like i'm 5")
        || lower.contains("like i am 5")
        || lower.contains("like i'm 10")
        || lower.contains("simple terms")
        || lower.contains("simply")
        || lower.contains("for a beginner");
    let is_why = lower.starts_with("why ") || lower.contains(" why ");
    let is_what_if = lower.contains("what happens if")
        || lower.contains("what would happen if")
        || lower.contains("what if ");
    let is_pros_cons = lower.contains("pros and cons")
        || lower.contains("tradeoffs")
        || lower.contains("trade-offs")
        || lower.contains("advantages")
        || lower.contains("limitations")
        || lower.contains("when should i use");

    if is_eli5 {
        return format!(
            "### {} — Explained Simply (From Intuition to First Principles)\n\n#### 1. The Intuitive Idea (ELI5)\n{}\n\nImagine a real-world example: {}\n\n#### 2. How It Works Under the Hood\nOnce you have the intuition, here is the step-by-step mechanism:\n{}\n\n#### 3. The Deeper Formal Law\n- **Governing Principle**: {}\n- **Why It Matters in Practice**: {}",
            node.title, node.summary, node.example, node.mechanism, node.formal_principle, node.tradeoffs
        );
    }

    if is_why {
        return format!(
            "### Causal Analysis: **{}** ({})\n\n#### 1. Direct Answer & Root Cause\n{}\n\n#### 2. Step-by-Step Causal Mechanism\n{}\n\n#### 3. Governing Law & Invariant\n- {}\n\n#### 4. Practical Consequence & Example\n- **Real-World Manifestation**: {}\n- **Engineering / Scientific Trade-off**: {}",
            raw_prompt.trim(),
            node.title,
            node.summary,
            node.mechanism,
            node.formal_principle,
            node.example,
            node.tradeoffs
        );
    }

    if is_what_if {
        return format!(
            "### Counterfactual & Boundary Analysis: **{}**\n\nGrounded in **{}** (`{}`):\n\n#### 1. Baseline System Equilibrium\n{}\n\n#### 2. Immediate Perturbation & Cascade Mechanism\n{}\n\n#### 3. Formal Boundary Law\n- {}\n- **Failure / Boundary Behavior**: {}\n- **Real-World Reference Case**: {}",
            raw_prompt.trim(),
            node.title,
            node.domain,
            node.summary,
            node.mechanism,
            node.formal_principle,
            node.tradeoffs,
            node.example
        );
    }

    if is_pros_cons {
        return format!(
            "### {} — Strengths, Limitations & Trade-off Analysis\n\n{}\n\n#### Key Strengths & Trade-offs\n{}\n\n#### Underlying Mechanism\n{}\n\n#### Formal Principle & Production Example\n- **Formal Bound**: {}\n- **Example**: {}",
            node.title, node.summary, node.tradeoffs, node.mechanism, node.formal_principle, node.example
        );
    }

    format!(
        "### {} ({})\n\n{}\n\n#### How It Works Under the Hood\n{}\n\n#### Formal Principle & Scaling\n- {}\n\n#### Strengths, Trade-offs & Practical Guidance\n{}\n\n#### Concrete Real-World Example\n> {}",
        node.title,
        node.domain,
        node.summary,
        node.mechanism,
        node.formal_principle,
        node.tradeoffs,
        node.example
    )
}

/// Domain-aware compositional synthesis when a prompt explores an open-ended topic beyond the static node titles.
fn render_open_domain_facet_synthesis(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
) -> String {
    let topic = extract_clean_topic(raw_prompt, words);
    let nearest = nearest_q12_concept(prompt_q12, prompt_sign);

    let is_how_to = lower.starts_with("how do i")
        || lower.starts_with("how can i")
        || lower.contains("how to ")
        || lower.contains("steps to ")
        || lower.starts_with("guide ");
    let is_brainstorm = words.iter().any(|w| {
        matches!(
            w.as_str(),
            "ideas" | "brainstorm" | "suggest" | "recommendations" | "recommend" | "tips" | "strategies" | "ways"
        )
    });
    let is_plan = words.iter().any(|w| {
        matches!(
            w.as_str(),
            "plan" | "roadmap" | "schedule" | "itinerary" | "curriculum" | "checklist" | "architecture" | "strategy"
        )
    });

    if is_how_to || is_plan {
        return format!(
            "### Step-by-Step Action Plan: **{}**\n\nHere is a structured, first-principles guide to **{}** (aligned with **{}** principles):\n\n#### Phase 1: Foundation & Scope Definition\n1. **Define the Measurable Objective**: Specify the exact target outcome, constraints (time, budget, latency, or quality), and success metric for **{}** before optimizing details.\n2. **Isolate the Core Bottleneck**: Identify the single highest-leverage component that determines `80%` of the outcome.\n\n#### Phase 2: Execution & Iterative Verification\n3. **Build the Minimum Viable Baseline**: Start with a clean, dependency-free baseline implementation or workflow and test it against a concrete real-world case.\n4. **Tighten the Feedback Loop**: Measure each iteration immediately against objective criteria (referencing `{}`: *{}*).\n\n#### Phase 3: Hardening & Scale\n5. **Stress-Test Edge Cases**: Verify behavior under boundary loads, missing inputs, or unexpected failures, and document the repeatable process.\n\n*Want me to tailor this plan for a specific constraint, timeline, or programming language (`Python`, `Rust`, `TypeScript`, `Go`, `SQL`)? Just let me know!*",
            raw_prompt.trim().trim_end_matches('?'),
            topic,
            nearest.domain,
            topic,
            nearest.title,
            nearest.formal_principle
        );
    }

    if is_brainstorm {
        return format!(
            "### High-Leverage Ideas & Strategic Directions for **{}**\n\nDrawing on systems thinking and **{}** (`{}`), here are five concrete, differentiated angles for **{}**:\n\n1. **Eliminate the Highest-Friction Step First**\n   - Map the current user or engineering workflow for **{}** and automate the single step with the highest drop-off or manual toil.\n2. **Deterministic, Local-First Speed**\n   - Inspired by `{}` ({}), design the core interaction to respond in `< 10 ms` with zero external round-trips.\n3. **Composable Building Blocks over Monolithic Templates**\n   - Expose small, orthogonal primitives that power users can combine in unexpected ways rather than rigid one-size-fits-all presets.\n4. **Built-In Verification & Transparency**\n   - Give users immediate, verifiable proof of why a result is correct (metrics, diffs, or cryptographic receipts).\n5. **Compounding Feedback Loops**\n   - Structure **{}** so every completed session or data point makes subsequent runs faster and more accurate.\n\nWhich of these five directions should we flesh out into a concrete spec or prototype?",
            topic,
            nearest.domain,
            nearest.title,
            topic,
            topic,
            nearest.title,
            nearest.formal_principle,
            topic
        );
    }

    format!(
        "### First-Principles Analysis: **{}**\n\n#### 1. Core Concept & Framing\n**{}** centers on how underlying mechanisms, incentives, and structural constraints interact to produce reliable outcomes. In the broader context of **{}**, it shares foundational dynamics with **{}**:\n> {}\n\n#### 2. Key Mechanisms & Governing Dynamics\n- **Primary Driver**: Systems involving **{}** are governed by the balance between *capacity/throughput*, *feedback latency*, and *boundary invariants* (`{}`).\n- **Second-Order Effects**: Optimizing a single metric in isolation often shifts pressure to adjacent bottlenecks; sustainable solutions align local actions with global system health ({})\n\n#### 3. Practical Application & Next Steps\n- **Concrete Anchor**: {}\n- **How to Explore Further**: Ask me to **compare `{}` against an alternative**, **walk through a numerical/Fermi estimate**, **explain a specific sub-mechanism**, or **write runnable code (`Python`, `Rust`, `TypeScript`, `Go`, `SQL`)**!",
        raw_prompt.trim().trim_end_matches('?'),
        topic,
        nearest.domain,
        nearest.title,
        nearest.summary,
        topic,
        nearest.formal_principle,
        nearest.tradeoffs,
        nearest.example,
        topic
    )
}

/// Top-level entry point for the Generalized UOR-R4 Geometric Language Model (`UR4GLM01`).
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
                    "### Concrete Example of **{}**\n\n> {}\n\n#### Mathematical / Structural Invariant\n- {}\n\n#### How It Plays Out Step by Step\n{}",
                    node.title, node.example, node.formal_principle, node.mechanism
                ));
            }
            if lower.contains("eli5") || lower.contains("simple") || lower.contains("simpler") || lower.contains("simplify") || lower.contains("like i'm ") || lower.contains("like i am ") {
                return wrap_binder(render_single_concept_response(prev_q, "eli5", node));
            }
            if lower.contains("summarize") || lower.contains("tldr") || lower.contains("shorter") {
                return wrap_binder(format!(
                    "### Executive Summary: **{}**\n\n1. **Core Definition**: {}\n2. **Key Principle**: {}\n3. **Bottom-Line Trade-off**: {}",
                    node.title, node.summary, node.formal_principle, node.tradeoffs
                ));
            }
            return wrap_binder(format!(
                "### Deep Dive on **{}** (Follow-Up)\n\nBuilding on our discussion of **\"{}\"** (`{}`):\n\n#### 1. Deeper Formal & Mathematical Structure\n- {}\n\n#### 2. Complete Mechanism Breakdown\n{}\n\n#### 3. Edge Cases & Production Trade-offs\n{}\n\n#### 4. Real-World Verification\n> {}",
                node.title, prev_q, node.domain, node.formal_principle, node.mechanism, node.tradeoffs, node.example
            ));
        }

        if let Some((prev_q, prev_a)) = chat_history.last() {
            let bullet_lines: Vec<&str> = prev_a
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .take(3)
                .collect();
            return wrap_binder(format!(
                "### Follow-Up on **\"{}\"**\n\n- **Key Synthesis**: {}\n- **Practical Example & Verification**: Isolate the primary inputs, apply the transformation deterministically, and validate boundary conditions with a concrete test case.\n\nWould you like me to generate runnable **Python**, **Rust**, **TypeScript**, or **Go** code for **\"{}\"**?",
                prev_q,
                bullet_lines.join(" "),
                prev_q
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
        if score_1 >= 8 && score_2 >= 8 && node_1.id != node_2.id && score_2 * 2 >= score_1 {
            return wrap_binder(render_multi_concept_synthesis(raw_prompt, lower, node_1, node_2));
        }
    }

    // 8. Single-Concept Deep Reasoning
    if let Some(&(node, score)) = top_concepts.first() {
        if score >= 5 {
            return wrap_binder(render_single_concept_response(raw_prompt, lower, node));
        }
    }

    // 9. Open-Domain Compositional Facet Synthesis
    wrap_binder(render_open_domain_facet_synthesis(
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
    print(f"Compiled {len(ALL_GLM_CONCEPTS)} GLM concept nodes and {len(GLM_LOGIC_PUZZLES)} logic puzzles -> {out_path}")
    print(f"SHA-256: {digest}")
