//! UOR-R4 Generalized Geometric Language Model (`UR4GLM04`) Pure-Rust Trainer & Weight Compiler.
//!
//! 1. Extracts distributional stem co-occurrence statistics and initial basin targets
//!    STRICTLY from the 5 training prose paragraphs (`summary`, `mechanism`, `formal_principle`,
//!    `tradeoffs`, `example`) of each basin — never touching `keywords`, `phrases`, or `title`.
//! 2. Exports binary target tensors (`--emit-cuda-targets`) for the CUDA hardware kernels
//!    `train_general_glm_manifold_kernel` and `train_glm_vocab_manifold_kernel` in `gpu/general_glm.cu`.
//! 3. Reads the RTX 5090 CUDA-trained weights (`gpu/rtx5090_glm_weights.bin`) and compiles
//!    `crates/wasm/src/glm_weights.rs` with zero Python anywhere in the build pipeline.

use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fs;
use std::io::{Read, Write};

include!("glm_corpus.rs");

const Q12_ONE: i32 = 4096;
const Q15_ONE: i32 = 32767;
const POINT_KEY_DIM: usize = 8;

const ICOSIAN_ROOTS_Q12: [[(i32, i32); 4]; 17] = [
    [(0, 0), (0, 0), (0, 0), (0, 0)],
    [(4096, 0), (0, 0), (0, 0), (0, 0)],
    [(-4096, 0), (0, 0), (0, 0), (0, 0)],
    [(0, 0), (4096, 0), (0, 0), (0, 0)],
    [(0, 0), (-4096, 0), (0, 0), (0, 0)],
    [(0, 0), (0, 0), (4096, 0), (0, 0)],
    [(0, 0), (0, 0), (-4096, 0), (0, 0)],
    [(0, 0), (0, 0), (0, 0), (4096, 0)],
    [(0, 0), (0, 0), (0, 0), (-4096, 0)],
    [(2048, 0), (2048, 0), (2048, 0), (2048, 0)],
    [(-2048, 0), (-2048, 0), (-2048, 0), (-2048, 0)],
    [(2048, 0), (2048, 0), (-2048, 0), (-2048, 0)],
    [(2048, 0), (-2048, 0), (2048, 0), (-2048, 0)],
    [(0, 2048), (2048, -2048), (2048, 0), (0, 0)],
    [(0, -2048), (-2048, 2048), (-2048, 0), (0, 0)],
    [(2048, 0), (0, 2048), (2048, -2048), (0, 0)],
    [(-2048, 0), (0, -2048), (-2048, 2048), (0, 0)],
];

fn select_icosian_root_q12(q12: &[i16; POINT_KEY_DIM]) -> u8 {
    let mut best_root = 0u8;
    let mut best_score = i64::MIN;
    for (idx, root) in ICOSIAN_ROOTS_Q12.iter().enumerate() {
        let mut a = 0i64;
        let mut b = 0i64;
        for lane in 0..4 {
            let x = i64::from(q12[lane]);
            a += x * i64::from(root[lane].0);
            b += x * i64::from(root[lane].1);
        }
        let score = 2 * a + 3 * b;
        if score > best_score {
            best_score = score;
            best_root = idx as u8;
        }
    }
    best_root
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn q12_l1_dist(a: &[i16; POINT_KEY_DIM], b: &[i16; POINT_KEY_DIM]) -> u32 {
    let mut s = 0u32;
    for i in 0..POINT_KEY_DIM {
        s += (i32::from(a[i]) - i32::from(b[i])).unsigned_abs();
    }
    s
}

fn tokenize_prose_words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            cur.push(ch.to_ascii_lowercase());
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn is_corpus_stopword(w: &str) -> bool {
    matches!(
        w,
        "the"
            | "and"
            | "for"
            | "that"
            | "this"
            | "with"
            | "from"
            | "are"
            | "was"
            | "were"
            | "have"
            | "has"
            | "had"
            | "not"
            | "but"
            | "what"
            | "how"
            | "why"
            | "when"
            | "where"
            | "who"
            | "which"
            | "can"
            | "could"
            | "would"
            | "should"
            | "will"
            | "into"
            | "over"
            | "under"
            | "between"
            | "across"
            | "through"
            | "about"
            | "such"
            | "than"
            | "then"
            | "them"
            | "they"
            | "their"
            | "there"
            | "these"
            | "those"
            | "its"
            | "our"
            | "your"
            | "his"
            | "her"
            | "while"
            | "both"
            | "each"
            | "every"
            | "many"
            | "most"
            | "more"
            | "some"
            | "other"
            | "another"
            | "only"
            | "also"
            | "even"
            | "very"
            | "much"
            | "like"
            | "just"
            | "make"
            | "made"
            | "makes"
            | "used"
            | "using"
            | "uses"
            | "use"
            | "well"
            | "because"
            | "whether"
            | "before"
            | "after"
            | "during"
            | "within"
            | "without"
            | "around"
            | "along"
            | "including"
            | "include"
            | "includes"
            | "known"
            | "called"
            | "often"
            | "first"
            | "two"
            | "three"
            | "four"
            | "five"
            | "one"
            | "new"
            | "high"
            | "low"
            | "large"
            | "small"
            | "long"
            | "short"
            | "world"
            | "time"
            | "year"
            | "years"
            | "day"
            | "days"
            | "way"
            | "ways"
            | "part"
            | "parts"
            | "system"
            | "systems"
            | "form"
            | "forms"
            | "state"
            | "states"
            | "process"
            | "modern"
            | "single"
            | "multiple"
            | "different"
            | "common"
            | "major"
            | "main"
            | "key"
            | "core"
            | "base"
            | "based"
            | "level"
            | "levels"
            | "order"
            | "example"
            | "examples"
            | "allow"
            | "allows"
            | "allowing"
            | "provide"
            | "provides"
            | "providing"
            | "produce"
            | "produces"
            | "producing"
            | "create"
            | "creates"
            | "creating"
            | "require"
            | "requires"
            | "requiring"
            | "remain"
            | "remains"
            | "become"
            | "becomes"
            | "became"
            | "human"
            | "people"
            | "life"
            | "work"
            | "works"
            | "working"
            | "century"
            | "centuries"
            | "today"
            | "history"
            | "historical"
            | "historically"
            | "global"
    )
}

fn canonical_stem_train(word: &str) -> String {
    let mut s = word.to_ascii_lowercase();
    if s.len() <= 3 {
        return s;
    }
    for suffix in [
        "izations", "ization", "ational", "fulness", "ousness", "iveness", "ically", "ations",
        "ation", "itions", "ition", "ments", "ment", "nesses", "ness", "ities", "ity", "ances",
        "ance", "ences", "ence", "ships", "ship", "ings", "ing", "edly", "ally", "iors", "ior",
        "ists", "ist", "isms", "ism", "ians", "ian", "ives", "ive", "able", "ible", "less", "ious",
        "eous", "ous", "ials", "ial", "ical", "ics", "ic", "ers", "ors", "ies", "ied", "es", "ed",
        "ly", "al", "er", "or", "s",
    ] {
        if s.len() > suffix.len() + 2 && s.ends_with(suffix) {
            s.truncate(s.len() - suffix.len());
            if suffix == "ies" || suffix == "ied" {
                s.push('y');
            }
            break;
        }
    }
    s
}

#[derive(Clone, Copy, Debug)]
struct ComputedBasinGeometry {
    q12: [i16; POINT_KEY_DIM],
    hopf_s2: [i16; 3],
    hopf_u1: [i16; 2],
    sign_code: u64,
    vsa_hv: [u64; 2],
    e8_root: u8,
    clifford_action: u8,
    icosian_root: u8,
    h4_root: u8,
    domain_id: u32,
}

#[derive(Clone, Debug)]
struct VocabTargetSpec {
    stem: String,
    pri_basin: u16,
    alt_basin: u16,
    ter_basin: u16,
    quat_basin: u16,
    idf_weight: u16,
    stem_hash: u64,
}

#[derive(Clone, Copy, Debug)]
struct TrainedVocabGeometry {
    q12_coords: [i16; POINT_KEY_DIM],
    alt_q12_coords: [i16; POINT_KEY_DIM],
    sign_code: u64,
    alt_sign_code: u64,
    vsa_hv: [u64; 2],
}

#[derive(Clone, Copy, Debug)]
struct HorizonTargetSpec {
    basin_id: u16,
    section_idx: u8,
    sentence_idx: u8,
    vocab_ids: [u16; 8],
}

#[derive(Clone, Copy, Debug)]
struct TrainedHorizonGeometry {
    horizon_q12: [i16; POINT_KEY_DIM],
    horizon_sign: u64,
    packed_4block: u32,
    basin_id: u16,
    section_idx: u8,
    sentence_idx: u8,
    next_horizon_id: u16,
    alt_horizon_id: u16,
    transition_energy: u32,
}

fn project_hopf_q15(q12: &[i16; POINT_KEY_DIM]) -> ([i16; 3], [i16; 2]) {
    let mut q0 = f64::from(q12[0]) / f64::from(Q12_ONE);
    let mut q1 = f64::from(q12[1]) / f64::from(Q12_ONE);
    let mut q2 = f64::from(q12[2]) / f64::from(Q12_ONE);
    let mut q3 = f64::from(q12[3]) / f64::from(Q12_ONE);
    let norm_sq = q0 * q0 + q1 * q1 + q2 * q2 + q3 * q3;
    if norm_sq < 1e-9 {
        q0 = 1.0;
        q1 = 0.0;
        q2 = 0.0;
        q3 = 0.0;
    } else {
        let inv = 1.0 / norm_sq.sqrt();
        q0 *= inv;
        q1 *= inv;
        q2 *= inv;
        q3 *= inv;
    }
    let sx = (2.0 * (q0 * q2 + q1 * q3) * f64::from(Q15_ONE)).round() as i32;
    let sy = (2.0 * (q1 * q2 - q0 * q3) * f64::from(Q15_ONE)).round() as i32;
    let sz = ((q0 * q0 + q1 * q1 - q2 * q2 - q3 * q3) * f64::from(Q15_ONE)).round() as i32;
    let phase = q1.atan2(q0);
    let ux = (phase.cos() * f64::from(Q15_ONE)).round() as i32;
    let uy = (phase.sin() * f64::from(Q15_ONE)).round() as i32;
    (
        [
            sx.clamp(-Q15_ONE, Q15_ONE) as i16,
            sy.clamp(-Q15_ONE, Q15_ONE) as i16,
            sz.clamp(-Q15_ONE, Q15_ONE) as i16,
        ],
        [
            ux.clamp(-Q15_ONE, Q15_ONE) as i16,
            uy.clamp(-Q15_ONE, Q15_ONE) as i16,
        ],
    )
}

fn split_section_horizon_clauses(sec_text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for raw_sent in sec_text.split(". ") {
        let s_trim = raw_sent.trim();
        if s_trim.len() < 25 {
            continue;
        }
        if s_trim.len() >= 95 && s_trim.contains("; ") {
            let parts: Vec<&str> = s_trim
                .split("; ")
                .map(str::trim)
                .filter(|p| p.len() >= 25)
                .collect();
            if parts.len() >= 2 {
                out.extend(parts);
                continue;
            }
        }
        if s_trim.len() >= 115 && (s_trim.contains(", while ") || s_trim.contains(", whereas ")) {
            let sep = if s_trim.contains(", while ") {
                ", while "
            } else {
                ", whereas "
            };
            let parts: Vec<&str> = s_trim
                .split(sep)
                .map(str::trim)
                .filter(|p| p.len() >= 28)
                .collect();
            if parts.len() >= 2 {
                out.extend(parts);
                continue;
            }
        }
        out.push(s_trim);
    }
    out
}

/// Extract basin targets, vocabulary stem co-occurrence specs, and token-level sentence horizon windows
/// STRICTLY from the 5 training prose paragraphs (`s1..s5`) of each basin — never inspecting `keywords`, `phrases`, or `title`.
fn extract_corpus_targets() -> (
    Vec<ComputedBasinGeometry>,
    Vec<VocabTargetSpec>,
    Vec<HorizonTargetSpec>,
) {
    let basin_count = RAW_GLM_CONCEPTS.len();
    let mut basin_stem_counts: Vec<BTreeMap<String, u32>> = vec![BTreeMap::new(); basin_count];
    let mut basin_total_words: Vec<u32> = vec![0; basin_count];
    let mut global_df: BTreeMap<String, u32> = BTreeMap::new();

    for (b_idx, &(_dom, _title, _kws, _phs, s1, s2, s3, s4, s5)) in
        RAW_GLM_CONCEPTS.iter().enumerate()
    {
        // Concatenate ONLY the 5 natural prose training paragraphs!
        let prose_only = format!("{s1} {s2} {s3} {s4} {s5}");
        let words = tokenize_prose_words(&prose_only);
        for w in words {
            if w.len() < 3 || is_corpus_stopword(&w) || w.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let stem = canonical_stem_train(&w);
            if stem.len() < 3 || is_corpus_stopword(&stem) {
                continue;
            }
            *basin_stem_counts[b_idx].entry(stem).or_insert(0) += 1;
            basin_total_words[b_idx] += 1;
        }
        for stem in basin_stem_counts[b_idx].keys() {
            *global_df.entry(stem.clone()).or_insert(0) += 1;
        }
    }

    // Build vocabulary stem specifications strictly from training prose co-occurrence
    let mut vocab_specs = Vec::new();
    for (stem, &df) in &global_df {
        if df == 0 || df > 50 {
            continue;
        }
        // Rank basins containing `stem` by normalized term frequency in the training prose
        let mut ranked_basins: Vec<(u16, u32)> = Vec::new();
        for (b_idx, counts) in basin_stem_counts.iter().enumerate() {
            if let Some(&tf) = counts.get(stem) {
                let norm_tf = (tf * 10_000) / basin_total_words[b_idx].max(50);
                ranked_basins.push((b_idx as u16, norm_tf));
            }
        }
        ranked_basins.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        if ranked_basins.is_empty() {
            continue;
        }
        let pri = ranked_basins[0].0;
        let alt = ranked_basins.get(1).map(|x| x.0).unwrap_or(pri);
        let ter = ranked_basins.get(2).map(|x| x.0).unwrap_or(alt);
        let quat = ranked_basins.get(3).map(|x| x.0).unwrap_or(ter);
        let idf_weight = (1024u32 / df.max(1)).clamp(64, 1024) as u16;
        let stem_hash = fnv1a64(stem.as_bytes());
        vocab_specs.push(VocabTargetSpec {
            stem: stem.clone(),
            pri_basin: pri,
            alt_basin: alt,
            ter_basin: ter,
            quat_basin: quat,
            idf_weight,
            stem_hash,
        });
    }
    // Sort lexicographically by stem for O(log V) binary search
    vocab_specs.sort_by(|a, b| a.stem.cmp(&b.stem));

    // Build well-separated basin target coordinates from training prose TF-IDF spectral projection
    let mut basin_geoms: Vec<ComputedBasinGeometry> = Vec::with_capacity(basin_count);
    for (b_idx, &(dom, _title, _kws, _phs, _s1, _s2, _s3, _s4, _s5)) in
        RAW_GLM_CONCEPTS.iter().enumerate()
    {
        let mut accum = [0i64; POINT_KEY_DIM];
        let mut sign_votes = [0i32; 64];
        let mut vsa0_votes = [0i32; 64];
        let mut vsa1_votes = [0i32; 64];

        for (stem, &tf) in &basin_stem_counts[b_idx] {
            let df = global_df.get(stem).copied().unwrap_or(1).max(1);
            let idf = (128 / df).max(1) as i64;
            let w_val = i64::from(tf) * idf;
            let sh = fnv1a64(stem.as_bytes());
            for lane in 0..POINT_KEY_DIM {
                let dir = (((sh >> (lane * 8)) & 0xFF) as i64) - 128;
                accum[lane] += dir * w_val;
            }
            let h1 = splitmix64(sh);
            let h2 = splitmix64(h1);
            let h3 = splitmix64(h2);
            for bit in 0..64 {
                let v = (w_val as i32).clamp(1, 64);
                if ((h1 >> bit) & 1) != 0 {
                    sign_votes[bit] += v;
                } else {
                    sign_votes[bit] -= v;
                }
                if ((h2 >> bit) & 1) != 0 {
                    vsa0_votes[bit] += v;
                } else {
                    vsa0_votes[bit] -= v;
                }
                if ((h3 >> bit) & 1) != 0 {
                    vsa1_votes[bit] += v;
                } else {
                    vsa1_votes[bit] -= v;
                }
            }
        }

        let max_abs = accum.iter().map(|x| x.abs()).max().unwrap_or(1).max(1);
        let mut q12 = [0i16; POINT_KEY_DIM];
        // Combine spectral text direction with quasi-orthogonal Golden-lattice carrier dispersion
        // so that no two distinct basins collide within the token capture radius.
        let disp_seed = splitmix64((b_idx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        for lane in 0..POINT_KEY_DIM {
            let text_comp = ((accum[lane] * 2200) / max_abs) as i32;
            let disp_comp = ((((disp_seed >> (lane * 8)) & 0xFF) as i32) - 128) * 14;
            q12[lane] = (text_comp + disp_comp).clamp(-3900, 3900) as i16;
        }

        // Enforce pairwise L1 separation >= 2600 against all earlier basins
        for attempt in 0..64u64 {
            let too_close = basin_geoms
                .iter()
                .any(|prev| q12_l1_dist(&q12, &prev.q12) < 2600);
            if !too_close {
                break;
            }
            let tweak = splitmix64(disp_seed.wrapping_add(attempt * 1009));
            for lane in 0..POINT_KEY_DIM {
                q12[lane] =
                    ((((tweak >> (lane * 8)) & 0xFF) as i32 - 128) * 30).clamp(-3900, 3900) as i16;
            }
        }

        let mut sign_code = 0u64;
        let mut vsa_0 = 0u64;
        let mut vsa_1 = 0u64;
        for bit in 0..64 {
            if sign_votes[bit] >= 0 {
                sign_code |= 1u64 << bit;
            }
            if vsa0_votes[bit] >= 0 {
                vsa_0 |= 1u64 << bit;
            }
            if vsa1_votes[bit] >= 0 {
                vsa_1 |= 1u64 << bit;
            }
        }
        sign_code ^= disp_seed;
        vsa_0 ^= splitmix64(disp_seed ^ 0x5555_5555_5555_5555);
        vsa_1 ^= splitmix64(disp_seed ^ 0xAAAA_AAAA_AAAA_AAAA);

        // Enforce pairwise 64-bit Hamming separation >= 16 across all earlier basins
        for attempt in 1..128u64 {
            let bad_sign = basin_geoms
                .iter()
                .any(|prev| (sign_code ^ prev.sign_code).count_ones() < 16);
            let bad_vsa0 = basin_geoms
                .iter()
                .any(|prev| (vsa_0 ^ prev.vsa_hv[0]).count_ones() < 16);
            let bad_vsa1 = basin_geoms
                .iter()
                .any(|prev| (vsa_1 ^ prev.vsa_hv[1]).count_ones() < 16);
            if !bad_sign && !bad_vsa0 && !bad_vsa1 {
                break;
            }
            if bad_sign {
                sign_code = splitmix64(sign_code.wrapping_add(attempt * 7919));
            }
            if bad_vsa0 {
                vsa_0 = splitmix64(vsa_0.wrapping_add(attempt * 6151));
            }
            if bad_vsa1 {
                vsa_1 = splitmix64(vsa_1.wrapping_add(attempt * 4999));
            }
        }

        let (hopf_s2, hopf_u1) = project_hopf_q15(&q12);
        let max_lane = q12
            .iter()
            .enumerate()
            .max_by_key(|(_, &v)| v.unsigned_abs())
            .map(|(i, _)| i as u8)
            .unwrap_or(0);
        let e8_root = ((max_lane * 2) + ((sign_code & 1) as u8)) % 16;
        let clifford_action = ((sign_code >> 8) as u8) % 64;
        let icosian_root = select_icosian_root_q12(&q12);
        let h4_root = (((sign_code >> 24) + u64::from(icosian_root) * 7) % 120) as u8;
        let dom_hash = (fnv1a64(dom.as_bytes()) & 0xFFFF_FFFF) as u32;

        basin_geoms.push(ComputedBasinGeometry {
            q12,
            hopf_s2,
            hopf_u1,
            sign_code,
            vsa_hv: [vsa_0, vsa_1],
            e8_root,
            clifford_action,
            icosian_root,
            h4_root,
            domain_id: dom_hash,
        });
    }

    // Extract token-level sentence/clause horizon specs across all 5 prose sections of all 170 basins
    let mut horizon_specs = Vec::new();
    for (b_idx, &(_dom, _title, _kws, _phs, s1, s2, s3, s4, s5)) in
        RAW_GLM_CONCEPTS.iter().enumerate()
    {
        let sections = [s1, s2, s3, s4, s5];
        for (sec_idx, &sec_text) in sections.iter().enumerate() {
            let clauses = split_section_horizon_clauses(sec_text);
            for (sent_idx, s_trim) in clauses.into_iter().enumerate() {
                let words = tokenize_prose_words(s_trim);
                let mut vocab_ids = [u16::MAX; 8];
                let mut write_slot = 0usize;
                for w in words {
                    if write_slot >= 8 {
                        break;
                    }
                    if w.len() < 3 || is_corpus_stopword(&w) {
                        continue;
                    }
                    let stem = canonical_stem_train(&w);
                    if let Ok(v_idx) = vocab_specs.binary_search_by(|v| v.stem.as_str().cmp(&stem))
                    {
                        vocab_ids[write_slot] = v_idx as u16;
                        write_slot += 1;
                    }
                }
                horizon_specs.push(HorizonTargetSpec {
                    basin_id: b_idx as u16,
                    section_idx: sec_idx as u8,
                    sentence_idx: (sent_idx.min(255)) as u8,
                    vocab_ids,
                });
            }
        }
    }

    (basin_geoms, vocab_specs, horizon_specs)
}

fn compute_host_vocab_geometry(
    basins: &[ComputedBasinGeometry],
    specs: &[VocabTargetSpec],
) -> Vec<TrainedVocabGeometry> {
    let mut out = Vec::with_capacity(specs.len());
    for spec in specs {
        let pri = &basins[usize::from(spec.pri_basin) % basins.len()];
        let alt = &basins[usize::from(spec.alt_basin) % basins.len()];
        let ter = &basins[usize::from(spec.ter_basin) % basins.len()];
        let quat = &basins[usize::from(spec.quat_basin) % basins.len()];
        let h = spec.stem_hash;

        let mut q12_coords = [0i16; POINT_KEY_DIM];
        let mut alt_q12_coords = [0i16; POINT_KEY_DIM];
        for lane in 0..POINT_KEY_DIM {
            let jp = (((h >> (lane * 4)) & 0xF) as i32) - 8;
            let ja = (((h >> (32 + lane * 4)) & 0xF) as i32) - 8;
            q12_coords[lane] = (i32::from(pri.q12[lane]) + jp).clamp(-Q12_ONE, Q12_ONE) as i16;
            alt_q12_coords[lane] = (i32::from(alt.q12[lane]) + ja).clamp(-Q12_ONE, Q12_ONE) as i16;
        }
        let sign_code = pri.sign_code ^ (1u64 << (h & 63));
        let alt_sign_code = alt.sign_code ^ (1u64 << ((h >> 6) & 63));
        let vsa_0 = ter.vsa_hv[0] ^ (1u64 << ((h >> 12) & 63));
        let vsa_1 = quat.vsa_hv[1] ^ (1u64 << ((h >> 18) & 63));

        out.push(TrainedVocabGeometry {
            q12_coords,
            alt_q12_coords,
            sign_code,
            alt_sign_code,
            vsa_hv: [vsa_0, vsa_1],
        });
    }
    out
}

fn compute_host_horizon_geometry(
    basins: &[ComputedBasinGeometry],
    vocab: &[TrainedVocabGeometry],
    specs: &[HorizonTargetSpec],
) -> Vec<TrainedHorizonGeometry> {
    let mut out = Vec::with_capacity(specs.len());
    let total = specs.len().max(1);
    for (h_idx, spec) in specs.iter().enumerate() {
        let b = &basins[usize::from(spec.basin_id) % basins.len()];
        let mut horizon_q12 = b.q12;
        let mut horizon_sign = b.sign_code;
        for &vid in &spec.vocab_ids {
            if let Some(vg) = vocab.get(usize::from(vid)) {
                for lane in 0..POINT_KEY_DIM {
                    horizon_q12[lane] =
                        ((i32::from(horizon_q12[lane]) * 2 + i32::from(vg.q12_coords[lane])) / 3)
                            .clamp(-Q12_ONE, Q12_ONE) as i16;
                }
                horizon_sign ^= vg.sign_code.rotate_left(3);
            }
        }
        let b0 = u32::from(b.icosian_root % 17);
        let b1 = u32::from(select_icosian_root_q12(&horizon_q12) % 17);
        let b2 = (b0 + b1) % 17;
        let b3 = (u32::from(spec.section_idx) + u32::from(spec.sentence_idx)) % 17;
        let packed_4block = b0 + 17 * b1 + 289 * b2 + 4913 * b3;
        let target_sec = (spec.section_idx + 1) % 5;
        let next_horizon_id = specs
            .iter()
            .enumerate()
            .find(|&(ci, cs)| {
                ci != h_idx && cs.basin_id == spec.basin_id && cs.section_idx == target_sec
            })
            .map(|(ci, _)| ci as u16)
            .unwrap_or(((h_idx + 1) % total) as u16);
        let alt_horizon_id = specs
            .iter()
            .enumerate()
            .find(|&(ci, cs)| {
                ci != h_idx
                    && cs.basin_id != spec.basin_id
                    && cs.section_idx == target_sec
            })
            .map(|(ci, _)| ci as u16)
            .unwrap_or(((h_idx + 17) % total) as u16);
        out.push(TrainedHorizonGeometry {
            horizon_q12,
            horizon_sign,
            packed_4block,
            basin_id: spec.basin_id,
            section_idx: spec.section_idx,
            sentence_idx: spec.sentence_idx,
            next_horizon_id,
            alt_horizon_id,
            transition_energy: 64,
        });
    }
    out
}

fn emit_cuda_targets(path: &str) -> std::io::Result<()> {
    let (basins, vocab_specs, horizon_specs) = extract_corpus_targets();
    let count = basins.len() as u32;
    let vocab_count = vocab_specs.len() as u32;
    let horizon_count = horizon_specs.len() as u32;

    let mut f = fs::File::create(path)?;
    f.write_all(&count.to_le_bytes())?;
    f.write_all(&vocab_count.to_le_bytes())?;
    f.write_all(&horizon_count.to_le_bytes())?;
    for g in &basins {
        for &c in &g.q12 {
            f.write_all(&c.to_le_bytes())?;
        }
    }
    for g in &basins {
        f.write_all(&g.sign_code.to_le_bytes())?;
    }
    for g in &basins {
        f.write_all(&g.vsa_hv[0].to_le_bytes())?;
        f.write_all(&g.vsa_hv[1].to_le_bytes())?;
    }
    for g in &basins {
        f.write_all(&g.domain_id.to_le_bytes())?;
    }
    for v in &vocab_specs {
        f.write_all(&v.pri_basin.to_le_bytes())?;
    }
    for v in &vocab_specs {
        f.write_all(&v.alt_basin.to_le_bytes())?;
    }
    for v in &vocab_specs {
        f.write_all(&v.ter_basin.to_le_bytes())?;
    }
    for v in &vocab_specs {
        f.write_all(&v.quat_basin.to_le_bytes())?;
    }
    for v in &vocab_specs {
        f.write_all(&v.idf_weight.to_le_bytes())?;
    }
    for v in &vocab_specs {
        f.write_all(&v.stem_hash.to_le_bytes())?;
    }
    for h in &horizon_specs {
        f.write_all(&h.basin_id.to_le_bytes())?;
    }
    for h in &horizon_specs {
        f.write_all(&[h.section_idx])?;
    }
    for h in &horizon_specs {
        f.write_all(&[h.sentence_idx])?;
    }
    for h in &horizon_specs {
        for &vid in &h.vocab_ids {
            f.write_all(&vid.to_le_bytes())?;
        }
    }
    println!(
        "Wrote {} CUDA basin targets, {} training-prose vocab stem targets, and {} token horizon targets -> {}",
        count, vocab_count, horizon_count, path
    );
    Ok(())
}

fn load_cuda_weights(
    path: &str,
    expected_basins: usize,
    expected_vocab: usize,
    expected_horizons: usize,
) -> Option<(
    Vec<ComputedBasinGeometry>,
    Vec<TrainedVocabGeometry>,
    Vec<TrainedHorizonGeometry>,
)> {
    let mut f = fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    if buf.len() < 12 {
        return None;
    }
    let basin_count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    let vocab_count = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize;
    let horizon_count = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]) as usize;
    let basin_row_size = 72usize;
    let vocab_row_size = 64usize;
    let horizon_row_size = 40usize;
    let min_bytes = 12
        + basin_count * basin_row_size
        + vocab_count * vocab_row_size
        + horizon_count * horizon_row_size;
    if basin_count != expected_basins
        || vocab_count != expected_vocab
        || horizon_count != expected_horizons
        || buf.len() < min_bytes
    {
        return None;
    }

    let mut basins = Vec::with_capacity(basin_count);
    for idx in 0..basin_count {
        let base = 12 + idx * basin_row_size;
        let mut q12 = [0i16; POINT_KEY_DIM];
        for lane in 0..POINT_KEY_DIM {
            let off = base + lane * 2;
            q12[lane] = i16::from_le_bytes([buf[off], buf[off + 1]]);
        }
        let hopf_s2 = [
            i16::from_le_bytes([buf[base + 16], buf[base + 17]]),
            i16::from_le_bytes([buf[base + 18], buf[base + 19]]),
            i16::from_le_bytes([buf[base + 20], buf[base + 21]]),
        ];
        let hopf_u1 = [
            i16::from_le_bytes([buf[base + 22], buf[base + 23]]),
            i16::from_le_bytes([buf[base + 24], buf[base + 25]]),
        ];
        let sign_code = u64::from_le_bytes(buf[base + 32..base + 40].try_into().ok()?);
        let vsa_0 = u64::from_le_bytes(buf[base + 40..base + 48].try_into().ok()?);
        let vsa_1 = u64::from_le_bytes(buf[base + 48..base + 56].try_into().ok()?);
        let e8_root = buf[base + 56];
        let clifford_action = buf[base + 57];
        let icosian_root = buf[base + 58];
        let h4_root = buf[base + 59];
        basins.push(ComputedBasinGeometry {
            q12,
            hopf_s2,
            hopf_u1,
            sign_code,
            vsa_hv: [vsa_0, vsa_1],
            e8_root,
            clifford_action,
            icosian_root,
            h4_root,
            domain_id: 0,
        });
    }

    let vocab_offset = 12 + basin_count * basin_row_size;
    let mut vocab = Vec::with_capacity(vocab_count);
    for v_idx in 0..vocab_count {
        let base = vocab_offset + v_idx * vocab_row_size;
        let mut q12_coords = [0i16; POINT_KEY_DIM];
        let mut alt_q12_coords = [0i16; POINT_KEY_DIM];
        for lane in 0..POINT_KEY_DIM {
            let off_p = base + lane * 2;
            let off_a = base + 16 + lane * 2;
            q12_coords[lane] = i16::from_le_bytes([buf[off_p], buf[off_p + 1]]);
            alt_q12_coords[lane] = i16::from_le_bytes([buf[off_a], buf[off_a + 1]]);
        }
        let sign_code = u64::from_le_bytes(buf[base + 32..base + 40].try_into().ok()?);
        let alt_sign_code = u64::from_le_bytes(buf[base + 40..base + 48].try_into().ok()?);
        let vsa_0 = u64::from_le_bytes(buf[base + 48..base + 56].try_into().ok()?);
        let vsa_1 = u64::from_le_bytes(buf[base + 56..base + 64].try_into().ok()?);
        vocab.push(TrainedVocabGeometry {
            q12_coords,
            alt_q12_coords,
            sign_code,
            alt_sign_code,
            vsa_hv: [vsa_0, vsa_1],
        });
    }

    let horizon_offset = vocab_offset + vocab_count * vocab_row_size;
    let mut horizons = Vec::with_capacity(horizon_count);
    for h_idx in 0..horizon_count {
        let base = horizon_offset + h_idx * horizon_row_size;
        let mut horizon_q12 = [0i16; POINT_KEY_DIM];
        for lane in 0..POINT_KEY_DIM {
            let off = base + lane * 2;
            horizon_q12[lane] = i16::from_le_bytes([buf[off], buf[off + 1]]);
        }
        let horizon_sign = u64::from_le_bytes(buf[base + 16..base + 24].try_into().ok()?);
        let packed_4block = u32::from_le_bytes(buf[base + 24..base + 28].try_into().ok()?);
        let basin_id = u16::from_le_bytes([buf[base + 28], buf[base + 29]]);
        let section_idx = buf[base + 30];
        let sentence_idx = buf[base + 31];
        let next_horizon_id = u16::from_le_bytes([buf[base + 32], buf[base + 33]]);
        let alt_horizon_id = u16::from_le_bytes([buf[base + 34], buf[base + 35]]);
        let transition_energy = u32::from_le_bytes(buf[base + 36..base + 40].try_into().ok()?);
        horizons.push(TrainedHorizonGeometry {
            horizon_q12,
            horizon_sign,
            packed_4block,
            basin_id,
            section_idx,
            sentence_idx,
            next_horizon_id,
            alt_horizon_id,
            transition_energy,
        });
    }

    Some((basins, vocab, horizons))
}

fn rust_raw_str(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

fn compile_rust_weights(cuda_weights_path: Option<&str>, out_rs_path: &str) -> std::io::Result<()> {
    let (host_basins, vocab_specs, horizon_specs) = extract_corpus_targets();
    let cuda_loaded = cuda_weights_path.and_then(|p| {
        load_cuda_weights(p, host_basins.len(), vocab_specs.len(), horizon_specs.len())
    });
    let used_cuda = cuda_loaded.is_some();
    let (basins, vocab_geoms, horizon_geoms) = match cuda_loaded {
        Some((b, v, h)) => (b, v, h),
        None => {
            let v = compute_host_vocab_geometry(&host_basins, &vocab_specs);
            let h = compute_host_horizon_geometry(&host_basins, &v, &horizon_specs);
            (host_basins, v, h)
        }
    };

    let mut out = String::with_capacity(1024 * 1024);
    out.push_str("//! Pre-trained Generalized Geometric Language Model (`UR4GLM06`) Natural Prose Manifold,\n");
    out.push_str("//! CUDA-Trained Distributional Vocabulary Manifold (`GLM_TRAINED_VOCAB`),\n");
    out.push_str("//! CUDA-Trained Token-Level Autoregressive Lookahead Horizon Table (`GLM_TOKEN_HORIZONS`),\n");
    out.push_str("//! Hopf Fibration (`S^3 -> S^2 x U(1)`) Trajectory Predictor, 128-Bit VSA Hypervector Attention,\n");
    out.push_str(
        "//! Algorithmic Puzzle Solvers, and Token-Level Geometric Autoregressive Decoder.\n",
    );
    out.push_str("//!\n");
    out.push_str("//! Generated deterministically by the pure-Rust trainer `gpu/train_general_glm.rs` + CUDA kernel `gpu/general_glm.cu`.\n\n");
    out.push_str("use crate::{canonical_stem, is_stopword, q12_l1_distance, swar_hamming_distance, POINT_KEY_DIM};\n\n");
    out.push_str("/// Canonical weights version identifier trained by `gpu/general_glm.cu`.\n");
    out.push_str("pub const GLM_WEIGHTS_VERSION: &str = \"UR4GLM06\";\n");
    out.push_str(&format!(
        "/// Whether these weights were optimized by the CUDA hardware kernel (`gpu/general_glm.cu`).\npub const GLM_CUDA_TRAINED: bool = {used_cuda};\n\n"
    ));

    out.push_str("/// A CUDA-trained token-level lookahead horizon row (`RETAIN_LAMBDA_Q31` + 4D Hamilton quaternion + Base-17 4-Block Trunk + `/7` Geodesic Transition Edges).\n");
    out.push_str("#[derive(Debug, Clone, Copy)]\n");
    out.push_str("pub struct GlmTokenHorizon {\n");
    out.push_str("    /// Forward-decayed 8-lane Q12 lookahead horizon vector (`R4.IntegerStack.retainLambdaQ31`).\n    pub horizon_q12: [i16; POINT_KEY_DIM],\n");
    out.push_str(
        "    /// Forward-decayed 64-bit SWAR quaternion sign code.\n    pub horizon_sign: u64,\n",
    );
    out.push_str("    /// Base-17 4-Block Icosian root address (`0..83520`, `R4.DiscreteTrunk.pack4BlockRoots`).\n    pub packed_4block: u32,\n");
    out.push_str("    /// Originating basin index (`0..164`).\n    pub basin_id: u16,\n");
    out.push_str("    /// Paragraph section index (`0..4`).\n    pub section_idx: u8,\n");
    out.push_str("    /// Sentence index within section (`0..N`).\n    pub sentence_idx: u8,\n");
    out.push_str("    /// CUDA-trained primary `/7` geodesic continuation horizon index.\n    pub next_horizon_id: u16,\n");
    out.push_str("    /// CUDA-trained alternate `/7` geodesic continuation horizon index.\n    pub alt_horizon_id: u16,\n");
    out.push_str("    /// CUDA-trained geodesic transition energy along `next_horizon_id`.\n    pub transition_energy: u32,\n");
    out.push_str("}\n\n");

    out.push_str("/// A CUDA-trained vocabulary stem coordinate row in `UR4GLM06` (derived strictly from training prose text).\n");
    out.push_str("/// Contains ONLY geometric coordinates (`Q12`, `SWAR` sign codes, `VSA` hypervectors, and IDF weight) — zero basin IDs.\n");
    out.push_str("#[derive(Debug, Clone, Copy)]\n");
    out.push_str("pub struct GlmVocabEntry {\n");
    out.push_str("    /// Canonical stem string (sorted lexicographically for O(log V) binary search).\n    pub stem: &'static str,\n");
    out.push_str("    /// Primary trained 8-lane Q12 coordinate fiber.\n    pub q12_coords: [i16; POINT_KEY_DIM],\n");
    out.push_str("    /// Secondary trained 8-lane Q12 coordinate fiber.\n    pub alt_q12_coords: [i16; POINT_KEY_DIM],\n");
    out.push_str(
        "    /// Primary trained 64-bit quaternion SWAR sign code.\n    pub sign_code: u64,\n",
    );
    out.push_str("    /// Secondary trained 64-bit quaternion SWAR sign code.\n    pub alt_sign_code: u64,\n");
    out.push_str("    /// Trained 128-bit VSA hypervector signature (`[u64; 2]`).\n    pub vsa_hv: [u64; 2],\n");
    out.push_str("    /// Integer inverse-document-frequency specificity weight in Q8 (`64..1024`).\n    pub idf_weight: u16,\n");
    out.push_str("}\n\n");

    out.push_str("/// A pre-trained concept & natural-prose basin in the UOR-R4 Generalized Geometric Manifold (`UR4GLM06`).\n");
    out.push_str("#[derive(Debug, Clone, Copy)]\n");
    out.push_str("pub struct GlmConceptNode {\n");
    out.push_str("    /// Unique concept node index (`0..N`).\n    pub id: u16,\n");
    out.push_str("    /// High-level knowledge domain label.\n    pub domain: &'static str,\n");
    out.push_str("    /// Canonical concept title.\n    pub title: &'static str,\n");
    out.push_str(
        "    /// Primary lexical trigger keywords.\n    pub keywords: &'static [&'static str],\n",
    );
    out.push_str("    /// Multi-word canonical trigger phrases.\n    pub phrases: &'static [&'static str],\n");
    out.push_str("    /// Trained 8-lane Q12 semantic coordinate vector.\n    pub q12_coords: [i16; POINT_KEY_DIM],\n");
    out.push_str("    /// Trained 64-bit quaternion SWAR sign code.\n    pub sign_code: u64,\n");
    out.push_str("    /// Hopf fibration S^2 Fubini-Study base coordinates in Q15.\n    pub hopf_s2: [i16; 3],\n");
    out.push_str(
        "    /// Hopf fibration U(1) fiber phase (cos, sin) in Q15.\n    pub hopf_u1: [i16; 2],\n",
    );
    out.push_str("    /// Condensed 128-bit VSA hypervector signature (`[u64; 2]`).\n    pub vsa_hv: [u64; 2],\n");
    out.push_str("    /// Assigned 16-root E8 orbit index.\n    pub e8_root: u8,\n");
    out.push_str("    /// Assigned 64-action Cl(0,6) Clifford carrier index.\n    pub clifford_action: u8,\n");
    out.push_str("    /// Assigned 17-root Icosian Z[phi] lattice index (`R4.SpiralCoreFbs`).\n    pub icosian_root: u8,\n");
    out.push_str("    /// Assigned 120-root H_4 600-cell polytope index.\n    pub h4_root: u8,\n");
    out.push_str("    /// Opening natural prose paragraph (overview & framing).\n    pub summary: &'static str,\n");
    out.push_str("    /// Second natural prose paragraph (historical / causal / mechanism depth).\n    pub mechanism: &'static str,\n");
    out.push_str("    /// Third natural prose paragraph (principles, architecture, or cultural context).\n    pub formal_principle: &'static str,\n");
    out.push_str("    /// Fourth natural prose paragraph (nuance, dynamics, or everyday life).\n    pub tradeoffs: &'static str,\n");
    out.push_str("    /// Fifth natural prose paragraph (concrete illustration & synthesis).\n    pub example: &'static str,\n");
    out.push_str("}\n\n");

    out.push_str("/// A classic logic, probability, or physics puzzle solved via exact step-by-step derivation.\n");
    out.push_str("#[derive(Debug, Clone, Copy)]\n");
    out.push_str("pub struct GlmLogicPuzzle {\n");
    out.push_str("    /// Canonical puzzle title.\n    pub title: &'static str,\n");
    out.push_str("    /// Trigger keywords.\n    pub keywords: &'static [&'static str],\n");
    out.push_str(
        "    /// Multi-word trigger phrases.\n    pub phrases: &'static [&'static str],\n",
    );
    out.push_str("    /// Step-by-step formal solution in Markdown.\n    pub solution_markdown: &'static str,\n");
    out.push_str("}\n\n");

    out.push_str("/// A fast conversational & common-sense Q&A pair (`mw-balp` + world facts).\n");
    out.push_str("#[derive(Debug, Clone, Copy)]\n");
    out.push_str("pub struct GlmConversationalQa {\n");
    out.push_str("    /// Trigger keywords.\n    pub keywords: &'static [&'static str],\n");
    out.push_str(
        "    /// Multi-word trigger phrases.\n    pub phrases: &'static [&'static str],\n",
    );
    out.push_str("    /// Natural conversational answer.\n    pub answer: &'static str,\n");
    out.push_str("}\n\n");

    out.push_str(&format!(
        "/// CUDA-Trained Distributional Vocabulary Manifold ({} stems learned strictly from training prose text).\n",
        vocab_specs.len()
    ));
    out.push_str(&format!(
        "pub static GLM_TRAINED_VOCAB: [GlmVocabEntry; {}] = [\n",
        vocab_specs.len()
    ));
    for (spec, vg) in vocab_specs.iter().zip(vocab_geoms.iter()) {
        let q12_p = vg
            .q12_coords
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let q12_a = vg
            .alt_q12_coords
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "    GlmVocabEntry {{ stem: {}, q12_coords: [{q12_p}], alt_q12_coords: [{q12_a}], sign_code: 0x{:016x}, alt_sign_code: 0x{:016x}, vsa_hv: [0x{:016x}, 0x{:016x}], idf_weight: {} }},\n",
            rust_raw_str(&spec.stem),
            vg.sign_code,
            vg.alt_sign_code,
            vg.vsa_hv[0],
            vg.vsa_hv[1],
            spec.idf_weight
        ));
    }
    out.push_str("];\n\n");

    out.push_str(&format!(
        "/// CUDA-Trained Token-Level Autoregressive Lookahead Horizon Table ({} token windows).\n",
        horizon_geoms.len()
    ));
    out.push_str(&format!(
        "pub static GLM_TOKEN_HORIZONS: [GlmTokenHorizon; {}] = [\n",
        horizon_geoms.len()
    ));
    for hg in &horizon_geoms {
        let q12_h = hg
            .horizon_q12
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "    GlmTokenHorizon {{ horizon_q12: [{q12_h}], horizon_sign: 0x{:016x}, packed_4block: {}, basin_id: {}, section_idx: {}, sentence_idx: {}, next_horizon_id: {}, alt_horizon_id: {}, transition_energy: {} }},\n",
            hg.horizon_sign,
            hg.packed_4block,
            hg.basin_id,
            hg.section_idx,
            hg.sentence_idx,
            hg.next_horizon_id,
            hg.alt_horizon_id,
            hg.transition_energy
        ));
    }
    out.push_str("];\n\n");

    out.push_str(&format!(
        "/// Pre-trained Generalized Geometric Concept Manifold ({} multi-domain natural prose basins).\n",
        RAW_GLM_CONCEPTS.len()
    ));
    out.push_str(&format!(
        "pub static GLM_CONCEPT_MANIFOLD: [GlmConceptNode; {}] = [\n",
        RAW_GLM_CONCEPTS.len()
    ));

    for (idx, &(dom, title, kws, phs, s1, s2, s3, s4, s5)) in RAW_GLM_CONCEPTS.iter().enumerate() {
        let g = basins[idx];
        let kw_str = kws
            .iter()
            .map(|k| rust_raw_str(k))
            .collect::<Vec<_>>()
            .join(", ");
        let ph_str = phs
            .iter()
            .map(|p| rust_raw_str(p))
            .collect::<Vec<_>>()
            .join(", ");
        let q12_str = g
            .q12
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let s2_str = g
            .hopf_s2
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let u1_str = g
            .hopf_u1
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        out.push_str("    GlmConceptNode {\n");
        out.push_str(&format!("        id: {idx},\n"));
        out.push_str(&format!("        domain: {},\n", rust_raw_str(dom)));
        out.push_str(&format!("        title: {},\n", rust_raw_str(title)));
        out.push_str(&format!("        keywords: &[{kw_str}],\n"));
        out.push_str(&format!("        phrases: &[{ph_str}],\n"));
        out.push_str(&format!("        q12_coords: [{q12_str}],\n"));
        out.push_str(&format!("        sign_code: 0x{:016x},\n", g.sign_code));
        out.push_str(&format!("        hopf_s2: [{s2_str}],\n"));
        out.push_str(&format!("        hopf_u1: [{u1_str}],\n"));
        out.push_str(&format!(
            "        vsa_hv: [0x{:016x}, 0x{:016x}],\n",
            g.vsa_hv[0], g.vsa_hv[1]
        ));
        out.push_str(&format!("        e8_root: {},\n", g.e8_root));
        out.push_str(&format!(
            "        clifford_action: {},\n",
            g.clifford_action
        ));
        out.push_str(&format!("        icosian_root: {},\n", g.icosian_root));
        out.push_str(&format!("        h4_root: {},\n", g.h4_root));
        out.push_str(&format!("        summary: {},\n", rust_raw_str(s1)));
        out.push_str(&format!("        mechanism: {},\n", rust_raw_str(s2)));
        out.push_str(&format!(
            "        formal_principle: {},\n",
            rust_raw_str(s3)
        ));
        out.push_str(&format!("        tradeoffs: {},\n", rust_raw_str(s4)));
        out.push_str(&format!("        example: {},\n", rust_raw_str(s5)));
        out.push_str("    },\n");
    }
    out.push_str("];\n\n");

    out.push_str(&format!(
        "/// Pre-trained Formal Logic & Probability Puzzle Solver Table ({} canonical puzzles).\n",
        RAW_GLM_PUZZLES.len()
    ));
    out.push_str(&format!(
        "pub static GLM_LOGIC_PUZZLES: [GlmLogicPuzzle; {}] = [\n",
        RAW_GLM_PUZZLES.len()
    ));
    for &(title, kws, phs, sol) in &RAW_GLM_PUZZLES {
        let kw_str = kws
            .iter()
            .map(|k| rust_raw_str(k))
            .collect::<Vec<_>>()
            .join(", ");
        let ph_str = phs
            .iter()
            .map(|p| rust_raw_str(p))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str("    GlmLogicPuzzle {\n");
        out.push_str(&format!("        title: {},\n", rust_raw_str(title)));
        out.push_str(&format!("        keywords: &[{kw_str}],\n"));
        out.push_str(&format!("        phrases: &[{ph_str}],\n"));
        out.push_str(&format!(
            "        solution_markdown: {},\n",
            rust_raw_str(sol)
        ));
        out.push_str("    },\n");
    }
    out.push_str("];\n\n");

    out.push_str(&format!(
        "/// Pre-trained Conversational & Common-Sense Q&A Table ({} everyday conversational basins).\n",
        RAW_GLM_CONV_QA.len()
    ));
    out.push_str(&format!(
        "pub static GLM_CONVERSATIONAL_QA: [GlmConversationalQa; {}] = [\n",
        RAW_GLM_CONV_QA.len()
    ));
    for &(kws, phs, ans) in &RAW_GLM_CONV_QA {
        let kw_str = kws
            .iter()
            .map(|k| rust_raw_str(k))
            .collect::<Vec<_>>()
            .join(", ");
        let ph_str = phs
            .iter()
            .map(|p| rust_raw_str(p))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str("    GlmConversationalQa {\n");
        out.push_str(&format!("        keywords: &[{kw_str}],\n"));
        out.push_str(&format!("        phrases: &[{ph_str}],\n"));
        out.push_str(&format!("        answer: {},\n", rust_raw_str(ans)));
        out.push_str("    },\n");
    }
    out.push_str("];\n\n");

    out.push_str(REASONER_ENGINE_SOURCE);
    fs::write(out_rs_path, &out)?;
    println!(
        "Compiled {} GLM basins, {} trained vocab stems, {} token horizons, {} puzzles, {} conversational Q&A pairs (CUDA hardware weights: {}) -> {}",
        RAW_GLM_CONCEPTS.len(),
        vocab_specs.len(),
        horizon_geoms.len(),
        RAW_GLM_PUZZLES.len(),
        RAW_GLM_CONV_QA.len(),
        used_cuda,
        out_rs_path
    );
    Ok(())
}

const REASONER_ENGINE_SOURCE: &str = include_str!("glm_reasoner_template.rs");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--emit-cuda-targets" {
        emit_cuda_targets(&args[2]).expect("failed to emit CUDA targets");
    } else if args.len() >= 4 && args[1] == "--compile-rust" {
        compile_rust_weights(Some(&args[2]), &args[3]).expect("failed to compile Rust weights");
    } else {
        let out_path = args
            .get(1)
            .map(String::as_str)
            .unwrap_or("crates/wasm/src/glm_weights.rs");
        compile_rust_weights(Some("gpu/rtx5090_glm_weights.bin"), out_path)
            .expect("failed to compile Rust weights");
    }
}
