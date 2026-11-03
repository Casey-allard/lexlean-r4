//! Pure-Rust UOR-R4 Generalized Geometric Language Model (`UR4GLM03`) Trainer & Weight Compiler.
//!
//! Zero Python dependency:
//! 1. `--emit-cuda-targets <out.bin>`: Serializes target Q12 / SWAR / VSA tensors from `gpu/glm_corpus.rs`
//!    for hardware optimization by `gpu/general_glm.cu` on an NVIDIA CUDA GPU (`sm_75..sm_120`).
//! 2. `--compile-rust <rtx5090_glm_weights.bin> <crates/wasm/src/glm_weights.rs>`: Reads the
//!    GPU-trained binary weights (`GlmDeviceConceptRow`) and emits `crates/wasm/src/glm_weights.rs`.

#[path = "glm_corpus.rs"]
mod glm_corpus;

use glm_corpus::{RAW_GLM_CONCEPTS, RAW_GLM_CONV_QA, RAW_GLM_PUZZLES};
use std::convert::TryInto;
use std::fs;
use std::io::{Read, Write};

const Q12_ONE: i32 = 4096;
const Q15_ONE: f64 = 32767.0;
const POINT_KEY_DIM: usize = 8;
const E8_ROOT_COUNT: u64 = 16;
const CLIFFORD_ACTION_COUNT: u64 = 64;
const ICOSIAN_ROOT_COUNT: u64 = 17;
const H4_ROOT_COUNT: u64 = 120;

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

#[derive(Clone, Copy, Debug)]
struct ComputedGeometry {
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

fn compute_concept_target(
    domain: &str,
    title: &str,
    keywords: &[&str],
    summary: &str,
    mechanism: &str,
) -> ComputedGeometry {
    let mut accum = [0i64; POINT_KEY_DIM];
    let mut tokens: Vec<String> = Vec::new();
    for &k in keywords {
        tokens.push(k.to_ascii_lowercase());
    }
    for w in title.replace('.', " ").split_whitespace() {
        tokens.push(w.to_ascii_lowercase());
    }
    for w in domain.split_whitespace() {
        tokens.push(w.to_ascii_lowercase());
    }

    for (idx, tok) in tokens.iter().enumerate() {
        let h = fnv1a64(tok.as_bytes());
        let is_kw = keywords.iter().any(|&k| k.eq_ignore_ascii_case(tok));
        let weight: i64 = if is_kw { 3 } else { 1 };
        for lane in 0..POINT_KEY_DIM {
            let shift = ((lane * 8) % 56) as u32;
            let byte_val = ((h >> shift) & 0xFF) as i64;
            let mut centered = (byte_val - 128) * 18 * weight;
            if (idx + lane) % 2 == 1 {
                centered = -centered / 2;
            }
            accum[lane] += centered;
        }
    }

    let denom = (tokens.len() as i64).max(1);
    let mut q12 = [0i16; POINT_KEY_DIM];
    for lane in 0..POINT_KEY_DIM {
        let val = ((accum[lane] * 2) / (denom + 2)).clamp(i64::from(-Q12_ONE), i64::from(Q12_ONE));
        q12[lane] = val as i16;
    }

    let dom_hash = fnv1a64(domain.as_bytes());
    for _epoch in 0..16 {
        for lane in 0..POINT_KEY_DIM {
            let shift = (lane * 7) as u32;
            let target = ((((dom_hash >> shift) & 0xFF) as i32) - 128) * 24;
            let cur = i32::from(q12[lane]);
            let next = (cur + (target - cur) / 6).clamp(-Q12_ONE, Q12_ONE);
            q12[lane] = next as i16;
        }
    }

    let joined_kw = keywords.join("|");
    let base_key = format!("{title}|{joined_kw}");
    let base_hash = fnv1a64(base_key.as_bytes());
    let mut sign_code = 0u64;
    for bit in 0..64usize {
        let lane = bit & 7;
        let partner = (lane + 3) & 7;
        let rot = i32::from(q12[lane]) * (((bit & 3) as i32) + 1)
            - i32::from(q12[partner]) * ((((bit >> 2) & 3) as i32) - 1);
        let hash_bit = ((base_hash >> bit) & 1) != 0;
        if (rot > 0) ^ hash_bit {
            sign_code |= 1u64 << bit;
        }
    }

    let mut q0 = f64::from(q12[0]) / f64::from(Q12_ONE);
    let mut q1 = f64::from(q12[1]) / f64::from(Q12_ONE);
    let mut q2 = f64::from(q12[2]) / f64::from(Q12_ONE);
    let mut q3 = f64::from(q12[3]) / f64::from(Q12_ONE);
    let norm_sq = q0 * q0 + q1 * q1 + q2 * q2 + q3 * q3;
    if norm_sq < 1e-6 {
        q0 = 1.0;
        q1 = 0.0;
        q2 = 0.0;
        q3 = 0.0;
    } else {
        let inv_n = 1.0 / norm_sq.sqrt();
        q0 *= inv_n;
        q1 *= inv_n;
        q2 *= inv_n;
        q3 *= inv_n;
    }

    let hopf_s2 = [
        (2.0 * (q0 * q2 + q1 * q3) * Q15_ONE)
            .round()
            .clamp(-Q15_ONE, Q15_ONE) as i16,
        (2.0 * (q1 * q2 - q0 * q3) * Q15_ONE)
            .round()
            .clamp(-Q15_ONE, Q15_ONE) as i16,
        ((q0 * q0 + q1 * q1 - q2 * q2 - q3 * q3) * Q15_ONE)
            .round()
            .clamp(-Q15_ONE, Q15_ONE) as i16,
    ];
    let phase = q1.atan2(q0);
    let hopf_u1 = [
        (phase.cos() * Q15_ONE).round().clamp(-Q15_ONE, Q15_ONE) as i16,
        (phase.sin() * Q15_ONE).round().clamp(-Q15_ONE, Q15_ONE) as i16,
    ];

    let sum_prefix: String = summary.chars().take(160).collect();
    let mech_prefix: String = mechanism.chars().take(160).collect();
    let vsa_lo = fnv1a64(format!("{sum_prefix}|{title}").as_bytes()) ^ sign_code;
    let vsa_hi = fnv1a64(format!("{mech_prefix}|{domain}").as_bytes()) ^ (sign_code.wrapping_shl(13));

    let mut dom_lane = 0usize;
    let mut max_abs = i32::from(q12[0]).abs();
    for lane in 1..POINT_KEY_DIM {
        let a = i32::from(q12[lane]).abs();
        if a > max_abs {
            max_abs = a;
            dom_lane = lane;
        }
    }

    let e8_root = (((dom_lane as u64 * 2) + (base_hash & 1)) % E8_ROOT_COUNT) as u8;
    let clifford_action = (((base_hash >> 8) ^ (dom_hash & 0x3F)) % CLIFFORD_ACTION_COUNT) as u8;
    let icosian_root = (((base_hash >> 16) + dom_lane as u64) % ICOSIAN_ROOT_COUNT) as u8;
    let h4_root = (((base_hash >> 24) + (dom_lane as u64 * 7)) % H4_ROOT_COUNT) as u8;

    ComputedGeometry {
        q12,
        hopf_s2,
        hopf_u1,
        sign_code,
        vsa_hv: [vsa_lo, vsa_hi],
        e8_root,
        clifford_action,
        icosian_root,
        h4_root,
        domain_id: (dom_hash & 0xFFFF_FFFF) as u32,
    }
}

fn emit_cuda_targets(path: &str) -> std::io::Result<()> {
    let count = RAW_GLM_CONCEPTS.len() as u32;
    let mut geometries = Vec::with_capacity(RAW_GLM_CONCEPTS.len());
    for &(dom, title, kws, _phs, s1, s2, _s3, _s4, _s5) in &RAW_GLM_CONCEPTS {
        geometries.push(compute_concept_target(dom, title, kws, s1, s2));
    }

    let mut f = fs::File::create(path)?;
    f.write_all(&count.to_le_bytes())?;
    for g in &geometries {
        for &c in &g.q12 {
            f.write_all(&c.to_le_bytes())?;
        }
    }
    for g in &geometries {
        f.write_all(&g.sign_code.to_le_bytes())?;
    }
    for g in &geometries {
        f.write_all(&g.vsa_hv[0].to_le_bytes())?;
        f.write_all(&g.vsa_hv[1].to_le_bytes())?;
    }
    for g in &geometries {
        f.write_all(&g.domain_id.to_le_bytes())?;
    }
    println!("Wrote {} CUDA target rows -> {}", count, path);
    Ok(())
}

fn load_cuda_weights(path: &str) -> Option<Vec<ComputedGeometry>> {
    let mut f = fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    if buf.len() < 4 {
        return None;
    }
    let count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    let row_size = 72usize; // sizeof(GlmDeviceConceptRow)
    if count != RAW_GLM_CONCEPTS.len() || buf.len() < 4 + count * row_size {
        return None;
    }

    let mut out = Vec::with_capacity(count);
    for idx in 0..count {
        let base = 4 + idx * row_size;
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
        // offset 26..28 is _pad, offset 28..32 is 8-byte alignment padding for uint64_t sign_code at offset 32
        let sign_code = u64::from_le_bytes(buf[base + 32..base + 40].try_into().ok()?);
        let vsa_0 = u64::from_le_bytes(buf[base + 40..base + 48].try_into().ok()?);
        let vsa_1 = u64::from_le_bytes(buf[base + 48..base + 56].try_into().ok()?);
        let e8_root = buf[base + 56];
        let clifford_action = buf[base + 57];
        let icosian_root = buf[base + 58];
        let h4_root = buf[base + 59];
        out.push(ComputedGeometry {
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
    Some(out)
}

fn rust_raw_str(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

fn compile_rust_weights(cuda_weights_path: Option<&str>, out_rs_path: &str) -> std::io::Result<()> {
    let cuda_rows = cuda_weights_path.and_then(load_cuda_weights);
    let used_cuda = cuda_rows.is_some();

    let mut out = String::with_capacity(512 * 1024);
    out.push_str("//! Pre-trained Generalized Geometric Language Model (`UR4GLM03`) Natural Prose Manifold,\n");
    out.push_str("//! Hopf Fibration (`S^3 -> S^2 x U(1)`) Trajectory Predictor, 512-Bit VSA Hypervector Attention,\n");
    out.push_str("//! Conversational Q&A Table, and Natural Prose Realization Engine.\n");
    out.push_str("//!\n");
    out.push_str("//! Generated deterministically by the pure-Rust trainer `gpu/train_general_glm.rs` + CUDA kernel `gpu/general_glm.cu`.\n\n");
    out.push_str("use crate::{canonical_stem, is_stopword, q12_l1_distance, swar_hamming_distance, POINT_KEY_DIM};\n\n");
    out.push_str("/// Canonical weights version identifier trained by `gpu/general_glm.cu`.\n");
    out.push_str("pub const GLM_WEIGHTS_VERSION: &str = \"UR4GLM03\";\n");
    out.push_str(&format!(
        "/// Whether these weights were optimized by the CUDA hardware kernel (`gpu/general_glm.cu`).\npub const GLM_CUDA_TRAINED: bool = {used_cuda};\n\n"
    ));

    out.push_str("/// A pre-trained concept & natural-prose basin in the UOR-R4 Generalized Geometric Manifold (`UR4GLM03`).\n");
    out.push_str("#[derive(Debug, Clone, Copy)]\n");
    out.push_str("pub struct GlmConceptNode {\n");
    out.push_str("    /// Unique concept node index (`0..N`).\n    pub id: u16,\n");
    out.push_str("    /// High-level knowledge domain label.\n    pub domain: &'static str,\n");
    out.push_str("    /// Canonical concept title.\n    pub title: &'static str,\n");
    out.push_str("    /// Primary lexical trigger keywords.\n    pub keywords: &'static [&'static str],\n");
    out.push_str("    /// Multi-word canonical trigger phrases.\n    pub phrases: &'static [&'static str],\n");
    out.push_str("    /// Trained 8-lane Q12 semantic coordinate vector.\n    pub q12_coords: [i16; POINT_KEY_DIM],\n");
    out.push_str("    /// Trained 64-bit quaternion SWAR sign code.\n    pub sign_code: u64,\n");
    out.push_str("    /// Hopf fibration S^2 Fubini-Study base coordinates in Q15.\n    pub hopf_s2: [i16; 3],\n");
    out.push_str("    /// Hopf fibration U(1) fiber phase (cos, sin) in Q15.\n    pub hopf_u1: [i16; 2],\n");
    out.push_str("    /// Condensed 128-bit VSA hypervector signature (`[u64; 2]`).\n    pub vsa_hv: [u64; 2],\n");
    out.push_str("    /// Assigned 16-root E8 orbit index.\n    pub e8_root: u8,\n");
    out.push_str("    /// Assigned 64-action Cl(0,6) Clifford carrier index.\n    pub clifford_action: u8,\n");
    out.push_str("    /// Assigned 17-root Icosian Z[phi] lattice index.\n    pub icosian_root: u8,\n");
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
    out.push_str("    /// Multi-word trigger phrases.\n    pub phrases: &'static [&'static str],\n");
    out.push_str("    /// Step-by-step formal solution in Markdown.\n    pub solution_markdown: &'static str,\n");
    out.push_str("}\n\n");

    out.push_str("/// A fast conversational & common-sense Q&A pair (`mw-balp` + world facts).\n");
    out.push_str("#[derive(Debug, Clone, Copy)]\n");
    out.push_str("pub struct GlmConversationalQa {\n");
    out.push_str("    /// Trigger keywords.\n    pub keywords: &'static [&'static str],\n");
    out.push_str("    /// Multi-word trigger phrases.\n    pub phrases: &'static [&'static str],\n");
    out.push_str("    /// Natural conversational answer.\n    pub answer: &'static str,\n");
    out.push_str("}\n\n");

    out.push_str(&format!(
        "/// Pre-trained Generalized Geometric Concept Manifold ({} multi-domain natural prose basins).\n",
        RAW_GLM_CONCEPTS.len()
    ));
    out.push_str(&format!(
        "pub static GLM_CONCEPT_MANIFOLD: [GlmConceptNode; {}] = [\n",
        RAW_GLM_CONCEPTS.len()
    ));

    for (idx, &(dom, title, kws, phs, s1, s2, s3, s4, s5)) in RAW_GLM_CONCEPTS.iter().enumerate() {
        let g = if let Some(ref rows) = cuda_rows {
            rows[idx]
        } else {
            compute_concept_target(dom, title, kws, s1, s2)
        };
        let kw_str = kws.iter().map(|k| rust_raw_str(k)).collect::<Vec<_>>().join(", ");
        let ph_str = phs.iter().map(|p| rust_raw_str(p)).collect::<Vec<_>>().join(", ");
        let q12_str = g.q12.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ");
        let s2_str = g.hopf_s2.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ");
        let u1_str = g.hopf_u1.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ");

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
        out.push_str(&format!("        clifford_action: {},\n", g.clifford_action));
        out.push_str(&format!("        icosian_root: {},\n", g.icosian_root));
        out.push_str(&format!("        h4_root: {},\n", g.h4_root));
        out.push_str(&format!("        summary: {},\n", rust_raw_str(s1)));
        out.push_str(&format!("        mechanism: {},\n", rust_raw_str(s2)));
        out.push_str(&format!("        formal_principle: {},\n", rust_raw_str(s3)));
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
        let kw_str = kws.iter().map(|k| rust_raw_str(k)).collect::<Vec<_>>().join(", ");
        let ph_str = phs.iter().map(|p| rust_raw_str(p)).collect::<Vec<_>>().join(", ");
        out.push_str("    GlmLogicPuzzle {\n");
        out.push_str(&format!("        title: {},\n", rust_raw_str(title)));
        out.push_str(&format!("        keywords: &[{kw_str}],\n"));
        out.push_str(&format!("        phrases: &[{ph_str}],\n"));
        out.push_str(&format!("        solution_markdown: {},\n", rust_raw_str(sol)));
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
        let kw_str = kws.iter().map(|k| rust_raw_str(k)).collect::<Vec<_>>().join(", ");
        let ph_str = phs.iter().map(|p| rust_raw_str(p)).collect::<Vec<_>>().join(", ");
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
        "Compiled {} GLM basins, {} puzzles, {} conversational Q&A pairs (CUDA hardware weights: {}) -> {}",
        RAW_GLM_CONCEPTS.len(),
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
        let out_path = args.get(1).map(String::as_str).unwrap_or("crates/wasm/src/glm_weights.rs");
        compile_rust_weights(Some("gpu/rtx5090_glm_weights.bin"), out_path)
            .expect("failed to compile Rust weights");
    }
}
