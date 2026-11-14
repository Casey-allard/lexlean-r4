//! Deterministic Compositional Training Corpus & Byte-Geometric Vocab Exporter for UOR-R4-GLLM v9 (R4-38).
//!
//! Tokenizes prompt->completion sequences using `crates/wasm/src/tokenizer.json` (ByteBPE-4096).
//! Exports:
//!   1. `v9_corpus.bin` (`UORCORP2` magic): prompt/reply token sequences plus training-only `copy_mask`.
//!   2. `v9_vocab_bytes.bin` (`UORVOCB9` magic): decoded UTF-8 byte strings for all 4,096 tokens.
//!
//! Held-out test identifiers (`sensor_drift_k9`, `base_offset_q3`, `orbital_phase_m2`, `lattice_stride_z7`,
//! and prefixes `sensor`, `base`, `orbital`) are strictly excluded from training samples.

mod bpe_json;

use bpe_json::{ByteBpe4096, Rng64};
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process;

const VOCAB_SIZE: usize = 4096;

/// Encode `(text_piece, is_copy_span)` segments by tokenizing the full concatenated reply
/// with `ByteLevel` char offsets so BPE merges match continuous autoregressive decoding 100%,
/// marking tokens whose character span overlaps any `is_copy_span` region.
fn encode_tagged_reply(tok: &ByteBpe4096, segments: &[(String, bool)]) -> (Vec<u16>, Vec<u8>) {
    let total_bytes: usize = segments.iter().map(|(p, _)| p.len()).sum();
    let mut full_text = String::with_capacity(total_bytes);
    let mut copy_intervals: Vec<(usize, usize)> = Vec::new();
    let mut pos = 0usize;
    for (piece, is_copy) in segments {
        full_text.push_str(piece);
        let nxt = pos + piece.chars().count();
        if *is_copy && nxt > pos {
            copy_intervals.push((pos, nxt));
        }
        pos = nxt;
    }

    let (mut r_ids, offsets) = tok.encode_with_offsets(&full_text);
    let mut r_copy = Vec::with_capacity(offsets.len() + 1);
    for (c_start, c_end) in offsets {
        let mut flagged = 0u8;
        for &(a, b) in &copy_intervals {
            if c_end > a && c_start < b {
                flagged = 1;
                break;
            }
        }
        r_copy.push(flagged);
    }
    r_ids.push(tok.eos_id());
    r_copy.push(0);
    (r_ids, r_copy)
}

type Sample = (String, Vec<(String, bool)>);

fn seg(text: impl Into<String>, is_copy: bool) -> (String, bool) {
    (text.into(), is_copy)
}

fn build_samples(rng: &mut Rng64) -> Vec<Sample> {
    let mut samples: Vec<Sample> = Vec::with_capacity(5300);

    // Strictly exclude "sensor", "base", "orbital", "telemetry" so held-out tests remain 100% zero-shot unseen!
    let id_prefixes: &[&str] = &[
        "alpha",
        "beta",
        "gamma",
        "delta",
        "omega",
        "sigma",
        "theta",
        "lambda",
        "flux",
        "phase",
        "drift",
        "offset",
        "stride",
        "sector",
        "orbit",
        "spinor",
        "packet",
        "cursor",
        "vertex",
        "lattice",
        "frame",
        "quota",
        "window",
        "buffer",
        "metric",
        "weight",
        "signal",
        "anchor",
        "router",
        "channel",
        "digest",
        "counter",
        "scalar",
        "tensor",
        "matrix",
        "vector",
        "kernel",
        "shader",
        "socket",
        "stream",
        "header",
        "footer",
        "margin",
        "radius",
        "origin",
        "target",
        "source",
        "handle",
        "module",
        "symbol",
        "branch",
        "commit",
        "ledger",
        "quorum",
        "cohort",
        "beacon",
        "cipher",
        "nonce",
        "ticket",
        "token",
        "sample",
        "bucket",
        "column",
        "record",
        "thread",
        "worker",
        "queue",
        "ring",
        "arena",
        "slab",
        "region",
        "page",
        "calib",
        "thermo",
        "chrono",
        "gyro",
        "piezo",
        "magneto",
        "vibro",
        "prism",
        "vortex",
        "helix",
        "quanta",
        "baryon",
        "lepton",
        "boson",
        "soliton",
        "plasmon",
        "magnetometer",
        "chronometer",
        "accelerometer",
        "calorimeter",
        "spectrometer",
        "interferometer",
        "potentiometer",
        "galvanometer",
    ];
    let id_mids: &[&str] = &[
        "val", "rate", "step", "idx", "len", "cap", "sum", "norm", "gain", "scale", "shift",
        "mask", "port", "cid", "node", "bias", "min", "max", "cnt", "ptr", "key", "tag", "seq",
        "pos", "crest", "trough", "pulse", "chirp", "skew", "jitter", "damp", "span", "vm", "vcpu",
        "vring", "vnet",
    ];
    let mut suffixes: Vec<String> = Vec::with_capacity(22 * 9);
    for ch in "abcdefghkmnpqrstuvwxyz".chars() {
        for d in 1..=9 {
            suffixes.push(format!("{ch}{d}"));
        }
    }

    let forbidden: HashSet<&str> = [
        "sensor_drift_k9",
        "base_offset_q3",
        "orbital_phase_m2",
        "lattice_stride_z7",
        "telemetry_gain_v4",
        "phase_bias_r2",
    ]
    .into_iter()
    .collect();

    let rand_ident = |rng: &mut Rng64| -> String {
        let arities = [2usize, 3, 3];
        loop {
            let arity = *rng.choose(&arities);
            let cand = if arity == 2 {
                format!("{}_{}", rng.choose(id_prefixes), rng.choose(&suffixes))
            } else {
                format!(
                    "{}_{}_{}",
                    rng.choose(id_prefixes),
                    rng.choose(id_mids),
                    rng.choose(&suffixes)
                )
            };
            if !forbidden.contains(cand.as_str()) {
                return cand;
            }
        }
    };

    // 1. Randomized Unseen Variable Binding & Rust Expression Composition (1,800 samples)
    let ops: &[(&str, &str)] = &[
        ("combining", "+"),
        ("adding", "+"),
        ("subtracting", "-"),
        ("multiplying", "*"),
        ("mixing via XOR", "^"),
    ];
    for _ in 0..1800 {
        let a = rand_ident(rng);
        let mut b = rand_ident(rng);
        while b == a {
            b = rand_ident(rng);
        }
        let va = rng.randint(3, 997);
        let vb = rng.randint(2, 511);
        let (verb, sym) = *rng.choose(ops);
        let prompt = format!(
            "Given {a} = {va} and {b} = {vb}, write a concise Rust expression {verb} {a} and {b}."
        );
        let segments = vec![
            seg(" ```rust\nlet result =", false),
            seg(format!(" {a}"), true),
            seg(format!(" {sym}"), false),
            seg(format!(" {b}"), true),
            seg(";\n```\nCombines", false),
            seg(format!(" {a}"), true),
            seg(" and", false),
            seg(format!(" {b}"), true),
            seg(" via exact integer arithmetic.", false),
        ];
        samples.push((prompt, segments));
    }

    // 2. Parameterized Rust / C / Python Function Synthesis with Unseen Identifiers (1,200 samples)
    let fn_verbs: &[&str] = &[
        "compute",
        "evaluate",
        "step",
        "fold",
        "accumulate",
        "project",
        "route",
        "clamp",
    ];
    let fn_nouns: &[&str] = &[
        "flux", "stride", "distance", "checksum", "margin", "energy", "momentum", "orbit",
    ];
    let rust_tys: &[&str] = &["i64", "u64", "i32", "usize"];
    let rust_ops: &[&str] = &["saturating_add", "wrapping_add", "saturating_mul"];

    for _ in 0..600 {
        let fn_name = format!(
            "{}_{}_{}",
            rng.choose(fn_verbs),
            rng.choose(fn_nouns),
            rng.choose(&suffixes)
        );
        let p1 = rand_ident(rng);
        let mut p2 = rand_ident(rng);
        while p2 == p1 {
            p2 = rand_ident(rng);
        }
        let ty = *rng.choose(rust_tys);
        let op = *rng.choose(rust_ops);
        let prompt = format!("Write a Rust function {fn_name}( {p1}: {ty}, {p2}: {ty} ) -> {ty}.");
        let segments = vec![
            seg(" ```rust\npub fn", false),
            seg(format!(" {fn_name}"), true),
            seg("(", false),
            seg(format!(" {p1}"), true),
            seg(format!(": {ty},"), false),
            seg(format!(" {p2}"), true),
            seg(format!(": {ty} ) -> {ty} {{\n   "), false),
            seg(format!(" {p1}"), true),
            seg(format!(".{op}("), false),
            seg(format!(" {p2}"), true),
            seg(" )\n}\n```", false),
        ];
        samples.push((prompt, segments));
    }

    for _ in 0..300 {
        let fn_name = format!("{}_{}", rng.choose(fn_verbs), rng.choose(fn_nouns));
        let p1 = rand_ident(rng);
        let mut p2 = rand_ident(rng);
        while p2 == p1 {
            p2 = rand_ident(rng);
        }
        let prompt = format!(
            "Write a Python function {fn_name}( {p1}, {p2} ) that returns their integer combination."
        );
        let segments = vec![
            seg(" ```python\ndef", false),
            seg(format!(" {fn_name}"), true),
            seg("(", false),
            seg(format!(" {p1}"), true),
            seg(": int,", false),
            seg(format!(" {p2}"), true),
            seg(": int ) -> int:\n    return", false),
            seg(format!(" {p1}"), true),
            seg(" +", false),
            seg(format!(" {p2}"), true),
            seg("\n```", false),
        ];
        samples.push((prompt, segments));
    }

    for _ in 0..300 {
        let fn_name = format!("{}_{}", rng.choose(fn_verbs), rng.choose(fn_nouns));
        let p1 = rand_ident(rng);
        let prompt = format!(
            "Write a freestanding C function {fn_name}(long {p1}) for the RV64IMAC MicroVM."
        );
        let segments = vec![
            seg(" ```c\nlong", false),
            seg(format!(" {fn_name}"), true),
            seg("(long", false),
            seg(format!(" {p1}"), true),
            seg(") {\n    return (", false),
            seg(format!(" {p1}"), true),
            seg(" * 3) + 1;\n}\n```", false),
        ];
        samples.push((prompt, segments));
    }

    // 3. Agentic MVM-Wasm Sandbox Tool Call & Live Observation Comprehension (1,200 samples)
    let mvm_fixed_cmds: &[&str] = &[
        "uname -srm",
        "ls -la /workspace",
        "/sbin/mvm-agent --handshake",
        "/bin/vsock_probe --try-inet",
        "cc /workspace/src/bench.c -o /workspace/bench && /workspace/bench",
    ];
    for _ in 0..700 {
        let tag = if rng.randrange(2) == 0 {
            format!(
                "{}-{}-{}",
                rng.choose(id_prefixes),
                rng.choose(id_mids),
                rng.randint(2, 99)
            )
        } else {
            format!(
                "{}-{}-{}",
                rng.choose(id_prefixes),
                rng.choose(&suffixes),
                rng.choose(id_mids)
            )
        };
        let cmd_idx = rng.randrange(10);
        let cmd = match cmd_idx {
            0..=4 => mvm_fixed_cmds[cmd_idx].to_string(),
            5 => format!("echo -n {tag} | sha256sum"),
            6 => format!("uname -a && echo -n {tag} | sha256sum"),
            7 | 8 => format!("uname -a and compute echo -n {tag} | sha256sum"),
            _ => format!("ls -la /workspace && echo -n {tag} | sha256sum"),
        };
        let prompt = format!("[agent_mode] Run {cmd} inside the Linux MicroVM sandbox.");
        let segments = vec![
            seg(" <thought>\nExecuting:", false),
            seg(format!(" {cmd}"), true),
            seg(
                " inside MVM-Wasm RV64IMAC over SpiralCore AF_VSOCK.\n</thought>\n<tool_call>{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"",
                false,
            ),
            seg(format!(" {cmd}"), true),
            seg("\"}}</tool_call>", false),
        ];
        samples.push((prompt, segments));
    }

    let sample_hashes: &[&str] = &[
        "9e86504b8878f8211398739e2d2553a1d78a6a4d0ef03a792eb9714998f88b86",
        "4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    ];
    for _ in 0..500 {
        let h_hex = *rng.choose(sample_hashes);
        let obs_out = match rng.randrange(5) {
            0 => format!(
                "Linux mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64 GNU/Linux | {h_hex}  -"
            ),
            1 => format!("Linux mvm-wasm 6.6.0-uor-rv64imac #1 SMP riscv64 GNU/Linux | {h_hex}  -"),
            2 => "MVM_AGENT_ONLINE cid=3 vsock_port=1024 riscv64".to_string(),
            3 => format!("FIB12={} COLLATZ27=111 riscv64", rng.randint(100, 200)),
            _ => format!("Linux mvm-riscv64 6.6.0 riscv64 | sha256={}", &h_hex[..16]),
        };
        let prompt = format!(
            "[agent_mode] Verify sandbox execution.\n<tool_response> {obs_out}</tool_response>"
        );
        let segments = vec![
            seg(
                " <thought>\nVerified live RV64IMAC MicroVM stdout from SpiralCore AF_VSOCK.\n</thought>\nSandbox execution succeeded (`exit_code = 0`):",
                false,
            ),
            seg(format!(" {obs_out}"), true),
            seg(".", false),
        ];
        samples.push((prompt, segments));
    }

    // 4. Technical Engineering, Computer Science, Mathematics & UOR-R4 Architecture Q&A (1,050 samples)
    let core_qa: &[(&str, &str)] = &[
        (
            "What is UOR-R4?",
            "UOR-R4 (`UORGLLM9`) is a pure-geometric language model and in-browser RV64IMAC Linux Studio that predicts tokens compositionally using causal PRLL KV attention, Cl(0,6) Walsh-Hadamard butterflies, and discrete Q12 T-MAC lookup tables with zero dense matrix multiplication and zero softmax.",
        ),
        (
            "How does UOR-R4 attention work without softmax or matmul?",
            "Each layer transforms 64-channel Q12 states via the self-adjoint Fast Walsh-Hadamard Transform (`FWHT-64`) and 256-entry T-MAC integer lookup tables, measures causal query-key proximity by L1 Manhattan plus 64-bit sign-sketch Hamming popcount distance, and normalizes weights via exact rational PRLL (`R4-27`) integer mass conservation.",
        ),
        (
            "How does the MVM-Wasm Linux sandbox isolate network access?",
            "Following `tinylabscom/mvm`, MVM-Wasm boots a 64-bit `RV64IMAC` guest with zero virtual NIC (`AF_INET` and `AF_INET6` return `-EAFNOSUPPORT = -97`). All host and inter-VM traffic traverses `AF_VSOCK` (`domain 40`) framed in 16-byte `SpiralCore` headers (`Cl(0,6)` spin + `E8` orbit), with host-side secret placeholder substitution.",
        ),
        (
            "How are model weights stored in the browser without exceeding memory limits?",
            "Weights are partitioned into SHA-256-verified 512 KB shards hydrated by a Dedicated Web Worker into the Origin Private File System (`OPFS`) via synchronous `FileSystemSyncAccessHandle`, keeping active Q12 T-MAC tables and the causal KV cache compact inside WebAssembly and WebGPU.",
        ),
        (
            "Explain quantum superposition.",
            "Quantum superposition is the principle that a quantum state lives in a linear combination of basis eigenstates until measurement projects the state onto a single observable outcome with probability governed by the squared amplitude norm.",
        ),
        (
            "Explain quantum mechanics.",
            "Quantum mechanics models physical systems as state vectors in Hilbert space evolving unitarily under the Schrodinger operator, where observables correspond to Hermitian operators with quantized spectra.",
        ),
        (
            "Explain spacetime curvature.",
            "In general relativity, mass-energy curves the four-dimensional Lorentzian metric tensor via Einstein's field equations, causing freely falling bodies and light rays to follow geodesics of curved spacetime.",
        ),
        (
            "Explain cellular mitosis.",
            "Cellular mitosis partitions duplicated chromosomes equally into two genetically identical daughter nuclei across prophase, metaphase, anaphase, and telophase, followed by cytokinesis.",
        ),
        (
            "What is an invariant manifold?",
            "An invariant manifold is a topological submanifold of phase space preserved under the flow of a dynamical system: trajectories starting on the manifold remain within it for all forward and backward time.",
        ),
        (
            "What is an affine transformation?",
            "An affine transformation combines a linear map with a translation (`y = A x + b`), preserving collinearity, parallelism, and ratios of distances along parallel lines.",
        ),
        (
            "Explain quantum tunneling.",
            "Quantum tunneling occurs when a wavefunction penetrates a finite potential energy barrier higher than the particle's classical energy, decaying exponentially inside the barrier and emerging with non-zero transmission amplitude.",
        ),
        (
            "Explain the principle of least action.",
            "The principle of least action states that the physical trajectory between two configurations makes the action functional `S = integral L(q, q_dot, t) dt` stationary (`delta S = 0`), yielding the Euler-Lagrange equations.",
        ),
        (
            "How do I structure a Rust workspace?",
            "Define a root `Cargo.toml` with `[workspace] members = [\"crates/*\"]` and `[workspace.dependencies]` so crates share one `Cargo.lock`, unified lint policies, and clean library boundaries.",
        ),
        (
            "What are workspace dependencies?",
            "Workspace dependencies declare shared package versions once under `[workspace.dependencies]` in the root `Cargo.toml`, allowing member crates to inherit them with `dep.workspace = true`.",
        ),
        (
            "Prove that the square root of 2 is irrational.",
            "Suppose `sqrt(2) = p / q` in lowest terms (`gcd(p, q) = 1`). Squaring gives `2 q^2 = p^2`, so `p` is even (`p = 2k`). Substituting gives `2 q^2 = 4 k^2` or `q^2 = 2 k^2`, making `q` even as well, contradicting `gcd(p, q) = 1`.",
        ),
        (
            "What is Wilson's Theorem?",
            "Wilson's Theorem states that an integer `p > 1` is prime if and only if `(p - 1)! == -1 (mod p)`, because every residue in `{2, ..., p - 2}` pairs uniquely with its multiplicative inverse modulo prime `p`.",
        ),
        (
            "What is the formal principle of Rayleigh scattering?",
            "Elastic electromagnetic scattering by particles much smaller than the incident wavelength exhibits cross-section scaling inversely with the fourth power of wavelength, preferentially scattering shorter blue wavelengths.",
        ),
        (
            "What is the capital of France?",
            "The capital of France is Paris, situated along the Seine River in the Ile-de-France region as the country's constitutional, cultural, and economic seat.",
        ),
        (
            "status",
            "Active runtime: `UOR-R4-GLLM v9` (`UORGLLM9`, zero stamped turn-memory, causal PRLL KV cache + `FWHT-64` T-MAC Q12 LUTs) paired with `MVM-Wasm` `RV64IMAC` Linux MicroVM over `SpiralCore` `AF_VSOCK`.",
        ),
        (
            "who am i",
            "You are working inside the local-first LexLean UOR-R4 AI Studio with full in-browser `RV64IMAC` MicroVM execution, `SpiralCore` `virtio-vsock` isolation, and deterministic SHA-256 receipts.",
        ),
        (
            "Entropy",
            "Information entropy `H(X) = -sum p(x) log2 p(x)` measures average uncertainty per symbol in bits, while thermodynamic Boltzmann-Gibbs entropy quantifies accessible microstate multiplicity.",
        ),
    ];
    for _ in 0..50 {
        for &(q, a) in core_qa {
            samples.push((q.to_string(), vec![seg(format!(" {a}"), false)]));
        }
    }

    rng.shuffle(&mut samples);
    samples
}

fn export_vocab_bytes(tok: &ByteBpe4096, vocab_out: &Path) -> Result<(), String> {
    let f = File::create(vocab_out)
        .map_err(|e| format!("failed creating {}: {e}", vocab_out.display()))?;
    let mut vf = BufWriter::new(f);
    vf.write_all(b"UORVOCB9")
        .map_err(|e| format!("write error: {e}"))?;
    vf.write_all(&(VOCAB_SIZE as u32).to_le_bytes())
        .map_err(|e| format!("write error: {e}"))?;
    for tid in 0..VOCAB_SIZE {
        let mut raw = if tid <= 2 {
            format!("<|special_{tid}|>").into_bytes()
        } else {
            tok.decode(&[tid as u16]).into_bytes()
        };
        if raw.len() > 255 {
            raw.truncate(255);
        }
        vf.write_all(&[raw.len() as u8])
            .map_err(|e| format!("write error: {e}"))?;
        vf.write_all(&raw)
            .map_err(|e| format!("write error: {e}"))?;
    }
    vf.flush().map_err(|e| format!("flush error: {e}"))
}

fn find_tokenizer_json() -> Result<PathBuf, String> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        for anc in cwd.ancestors().take(6) {
            roots.push(anc.to_path_buf());
        }
    }
    if let Ok(exe) = env::current_exe() {
        for anc in exe.ancestors().take(6) {
            roots.push(anc.to_path_buf());
        }
    }
    for rel in [
        "crates/wasm/src/tokenizer.json",
        "gpu/tokenizer.json",
        "tokenizer.json",
    ] {
        for r in &roots {
            let cand = r.join(rel);
            if cand.is_file() {
                return Ok(cand);
            }
        }
    }
    Err("could not locate crates/wasm/src/tokenizer.json or tokenizer.json".to_string())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("Usage: build_v9_corpus [OUT_PATH]");
        println!("  OUT_PATH  Output corpus binary path (default: v9_corpus.bin)");
        return Ok(());
    }
    if args.len() > 1 {
        return Err(format!("unexpected extra arguments: {:?}", &args[1..]));
    }

    let tok_path = find_tokenizer_json()?;
    let tok = ByteBpe4096::from_file(&tok_path)?;

    let out_path = PathBuf::from(args.first().map(|s| s.as_str()).unwrap_or("v9_corpus.bin"));
    let vocab_out = match out_path.parent() {
        Some(p) if !p.as_os_str().is_empty() => {
            fs::create_dir_all(p).map_err(|e| format!("failed creating {}: {e}", p.display()))?;
            p.join("v9_vocab_bytes.bin")
        }
        _ => PathBuf::from("v9_vocab_bytes.bin"),
    };
    export_vocab_bytes(&tok, &vocab_out)?;

    let mut rng = Rng64::new(20261110);
    let samples = build_samples(&mut rng);

    let bos_id = tok.bos_id();
    let eos_id = tok.eos_id();
    let mut encoded_records: Vec<(Vec<u16>, Vec<u16>, Vec<u8>)> = Vec::with_capacity(samples.len());
    let mut total_tokens = 0usize;

    for (prompt, segments) in &samples {
        let prompt_str = format!("User: {prompt}\nAssistant:");
        let enc_prompt = tok.encode(&prompt_str);
        let mut p_ids = Vec::with_capacity(1 + enc_prompt.len());
        p_ids.push(bos_id);
        p_ids.extend_from_slice(&enc_prompt);

        let (mut r_ids, mut r_copy) = encode_tagged_reply(&tok, segments);
        if p_ids.len() + r_ids.len() > 256 {
            let keep = (256isize - (p_ids.len() as isize) - 1).max(8) as usize;
            r_ids.truncate(keep);
            r_ids.push(eos_id);
            r_copy.truncate(keep);
            r_copy.push(0);
        }
        if p_ids.len() >= 4 && r_ids.len() >= 2 {
            total_tokens += p_ids.len() + r_ids.len();
            encoded_records.push((p_ids, r_ids, r_copy));
        }
    }

    {
        let f = File::create(&out_path)
            .map_err(|e| format!("failed creating {}: {e}", out_path.display()))?;
        let mut bf = BufWriter::new(f);
        bf.write_all(b"UORCORP2")
            .map_err(|e| format!("write error: {e}"))?;
        bf.write_all(&(encoded_records.len() as u32).to_le_bytes())
            .map_err(|e| format!("write error: {e}"))?;
        for (p_ids, r_ids, r_copy) in &encoded_records {
            bf.write_all(&(p_ids.len() as u16).to_le_bytes())
                .map_err(|e| format!("write error: {e}"))?;
            bf.write_all(&(r_ids.len() as u16).to_le_bytes())
                .map_err(|e| format!("write error: {e}"))?;
            for &tid in p_ids {
                bf.write_all(&tid.to_le_bytes())
                    .map_err(|e| format!("write error: {e}"))?;
            }
            for &tid in r_ids {
                bf.write_all(&tid.to_le_bytes())
                    .map_err(|e| format!("write error: {e}"))?;
            }
            bf.write_all(r_copy)
                .map_err(|e| format!("write error: {e}"))?;
        }
        bf.flush().map_err(|e| format!("flush error: {e}"))?;
    }

    let out_size = fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
    println!(
        "Wrote {} compositional sequences ({} ByteBPE-4096 tokens) -> {} ({} bytes) + {}",
        encoded_records.len(),
        total_tokens,
        out_path.display(),
        out_size,
        vocab_out.display()
    );
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
