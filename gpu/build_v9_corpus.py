#!/usr/bin/env python3
"""
Deterministic Compositional Training Corpus & Byte-Geometric Vocab Exporter for UOR-R4-GLLM v9 (R4-38).

Tokenizes prompt->completion sequences using `crates/wasm/src/tokenizer.json` (ByteBPE-4096).
Exports:
  1. `v9_corpus.bin` (`UORCORP2` magic): prompt/reply token sequences plus training-only `copy_mask`
     so readout hyperplanes never memorize random training identifier names while causal KV
     pointer-copy heads learn genuine zero-shot structural & suffix-induction copying.
  2. `v9_vocab_bytes.bin` (`UORVOCB9` magic): decoded UTF-8 byte strings for all 4,096 tokens so
     `train_gllm_v9.cu` initializes `e_tok` and `p_out` from deterministic `Cl(0,6)` byte geometry
     across the entire vocabulary (zero unseen-token noise handicap).

Held-out test identifiers (`sensor_drift_k9`, `base_offset_q3`, `orbital_phase_m2`, `lattice_stride_z7`,
and prefixes `sensor`, `base`, `orbital`) are strictly excluded from training samples.
"""

import pathlib
import random
import struct
import sys
from tokenizers import Tokenizer


def encode_tagged_reply(tok: Tokenizer, segments):
    """
    Encode `(text_piece, is_copy_span)` segments by tokenizing the full concatenated reply
    with `ByteLevel` offsets so BPE merges match continuous autoregressive decoding 100%,
    marking tokens whose character span overlaps any `is_copy_span` region.
    """
    full_text = "".join(piece for piece, _ in segments)
    copy_intervals = []
    pos = 0
    for piece, is_copy in segments:
        nxt = pos + len(piece)
        if is_copy and nxt > pos:
            copy_intervals.append((pos, nxt))
        pos = nxt

    enc = tok.encode(full_text)
    r_ids = list(enc.ids)
    r_copy = []
    for (c_start, c_end) in enc.offsets:
        flagged = 0
        for (a, b) in copy_intervals:
            if c_end > a and c_start < b:
                flagged = 1
                break
        r_copy.append(flagged)
    r_ids.append(1)  # <|eos|>
    r_copy.append(0)
    return r_ids, r_copy


def build_samples(rng: random.Random):
    samples = []

    # Strictly exclude "sensor", "base", "orbital", "telemetry" so held-out tests remain 100% zero-shot unseen!
    id_prefixes = [
        "alpha", "beta", "gamma", "delta", "omega", "sigma", "theta", "lambda",
        "flux", "phase", "drift", "offset", "stride", "sector", "orbit", "spinor",
        "packet", "cursor", "vertex", "lattice", "frame", "quota", "window", "buffer",
        "metric", "weight", "signal", "anchor", "router", "channel", "digest", "counter",
        "scalar", "tensor", "matrix", "vector", "kernel", "shader", "socket", "stream",
        "header", "footer", "margin", "radius", "origin", "target", "source", "handle",
        "module", "symbol", "branch", "commit", "ledger", "quorum", "cohort", "beacon",
        "cipher", "nonce", "ticket", "token", "sample", "bucket", "column", "record",
        "thread", "worker", "queue", "ring", "arena", "slab", "region", "page",
        "calib", "thermo", "chrono", "gyro", "piezo", "magneto", "vibro", "prism",
        "vortex", "helix", "quanta", "baryon", "lepton", "boson", "soliton", "plasmon",
        "magnetometer", "chronometer", "accelerometer", "calorimeter",
        "spectrometer", "interferometer", "potentiometer", "galvanometer",
    ]
    id_mids = [
        "val", "rate", "step", "idx", "len", "cap", "sum", "norm",
        "gain", "scale", "shift", "mask", "port", "cid", "node", "bias",
        "min", "max", "cnt", "ptr", "key", "tag", "seq", "pos",
        "crest", "trough", "pulse", "chirp", "skew", "jitter", "damp", "span",
        "vm", "vcpu", "vring", "vnet",
    ]
    suffixes = [f"{ch}{d}" for ch in "abcdefghkmnpqrstuvwxyz" for d in range(1, 10)]

    forbidden = {
        "sensor_drift_k9",
        "base_offset_q3",
        "orbital_phase_m2",
        "lattice_stride_z7",
        "telemetry_gain_v4",
        "phase_bias_r2",
    }

    def rand_ident():
        while True:
            arity = rng.choice([2, 3, 3])
            if arity == 2:
                cand = f"{rng.choice(id_prefixes)}_{rng.choice(suffixes)}"
            else:
                cand = f"{rng.choice(id_prefixes)}_{rng.choice(id_mids)}_{rng.choice(suffixes)}"
            if cand not in forbidden:
                return cand

    # 1. Randomized Unseen Variable Binding & Rust Expression Composition (1,800 samples)
    ops = [
        ("combining", "+"),
        ("adding", "+"),
        ("subtracting", "-"),
        ("multiplying", "*"),
        ("mixing via XOR", "^"),
    ]
    for _ in range(1800):
        a = rand_ident()
        b = rand_ident()
        while b == a:
            b = rand_ident()
        va = rng.randint(3, 997)
        vb = rng.randint(2, 511)
        verb, sym = rng.choice(ops)
        prompt = f"Given {a} = {va} and {b} = {vb}, write a concise Rust expression {verb} {a} and {b}."
        segments = [
            (" ```rust\nlet result =", False),
            (f" {a}", True),
            (f" {sym}", False),
            (f" {b}", True),
            (";\n```\nCombines", False),
            (f" {a}", True),
            (" and", False),
            (f" {b}", True),
            (" via exact integer arithmetic.", False),
        ]
        samples.append((prompt, segments))

    # 2. Parameterized Rust / C / Python Function Synthesis with Unseen Identifiers (1,200 samples)
    fn_verbs = ["compute", "evaluate", "step", "fold", "accumulate", "project", "route", "clamp"]
    fn_nouns = ["flux", "stride", "distance", "checksum", "margin", "energy", "momentum", "orbit"]
    for _ in range(600):
        fn_name = f"{rng.choice(fn_verbs)}_{rng.choice(fn_nouns)}_{rng.choice(suffixes)}"
        p1 = rand_ident()
        p2 = rand_ident()
        while p2 == p1:
            p2 = rand_ident()
        ty = rng.choice(["i64", "u64", "i32", "usize"])
        op = rng.choice(["saturating_add", "wrapping_add", "saturating_mul"])
        prompt = f"Write a Rust function {fn_name}( {p1}: {ty}, {p2}: {ty} ) -> {ty}."
        segments = [
            (" ```rust\npub fn", False),
            (f" {fn_name}", True),
            ("(", False),
            (f" {p1}", True),
            (f": {ty},", False),
            (f" {p2}", True),
            (f": {ty} ) -> {ty} {{\n   ", False),
            (f" {p1}", True),
            (f".{op}(", False),
            (f" {p2}", True),
            (" )\n}\n```", False),
        ]
        samples.append((prompt, segments))

    for _ in range(300):
        fn_name = f"{rng.choice(fn_verbs)}_{rng.choice(fn_nouns)}"
        p1 = rand_ident()
        p2 = rand_ident()
        while p2 == p1:
            p2 = rand_ident()
        prompt = f"Write a Python function {fn_name}( {p1}, {p2} ) that returns their integer combination."
        segments = [
            (" ```python\ndef", False),
            (f" {fn_name}", True),
            ("(", False),
            (f" {p1}", True),
            (": int,", False),
            (f" {p2}", True),
            (": int ) -> int:\n    return", False),
            (f" {p1}", True),
            (" +", False),
            (f" {p2}", True),
            ("\n```", False),
        ]
        samples.append((prompt, segments))

    for _ in range(300):
        fn_name = f"{rng.choice(fn_verbs)}_{rng.choice(fn_nouns)}"
        p1 = rand_ident()
        prompt = f"Write a freestanding C function {fn_name}(long {p1}) for the RV64IMAC MicroVM."
        segments = [
            (" ```c\nlong", False),
            (f" {fn_name}", True),
            ("(long", False),
            (f" {p1}", True),
            (") {\n    return (", False),
            (f" {p1}", True),
            (" * 3) + 1;\n}\n```", False),
        ]
        samples.append((prompt, segments))

    # 3. Agentic MVM-Wasm Sandbox Tool Call & Live Observation Comprehension (1,200 samples)
    mvm_cmds = [
        "uname -srm",
        "ls -la /workspace",
        "/sbin/mvm-agent --handshake",
        "/bin/vsock_probe --try-inet",
        "cc /workspace/src/bench.c -o /workspace/bench && /workspace/bench",
    ]
    for _ in range(700):
        tag = rng.choice([
            f"{rng.choice(id_prefixes)}-{rng.choice(id_mids)}-{rng.randint(2, 99)}",
            f"{rng.choice(id_prefixes)}-{rng.choice(suffixes)}-{rng.choice(id_mids)}",
        ])
        cmd = rng.choice(mvm_cmds + [
            f"echo -n {tag} | sha256sum",
            f"uname -a && echo -n {tag} | sha256sum",
            f"uname -a and compute echo -n {tag} | sha256sum",
            f"uname -a and compute echo -n {tag} | sha256sum",
            f"ls -la /workspace && echo -n {tag} | sha256sum",
        ])
        prompt = f"[agent_mode] Run {cmd} inside the Linux MicroVM sandbox."
        segments = [
            (" <thought>\nExecuting:", False),
            (f" {cmd}", True),
            (" inside MVM-Wasm RV64IMAC over SpiralCore AF_VSOCK.\n</thought>\n<tool_call>{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"", False),
            (f" {cmd}", True),
            ("\"}}</tool_call>", False),
        ]
        samples.append((prompt, segments))

    sample_hashes = [
        "9e86504b8878f8211398739e2d2553a1d78a6a4d0ef03a792eb9714998f88b86",
        "4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    ]
    for _ in range(500):
        h_hex = rng.choice(sample_hashes)
        obs_out = rng.choice([
            f"Linux mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64 GNU/Linux | {h_hex}  -",
            f"Linux mvm-wasm 6.6.0-uor-rv64imac #1 SMP riscv64 GNU/Linux | {h_hex}  -",
            "MVM_AGENT_ONLINE cid=3 vsock_port=1024 riscv64",
            f"FIB12={rng.randint(100,200)} COLLATZ27=111 riscv64",
            f"Linux mvm-riscv64 6.6.0 riscv64 | sha256={h_hex[:16]}",
        ])
        prompt = (
            f"[agent_mode] Verify sandbox execution.\n"
            f"<tool_response> {obs_out}</tool_response>"
        )
        segments = [
            (" <thought>\nVerified live RV64IMAC MicroVM stdout from SpiralCore AF_VSOCK.\n</thought>\nSandbox execution succeeded (`exit_code = 0`):", False),
            (f" {obs_out}", True),
            (".", False),
        ]
        samples.append((prompt, segments))

    # 4. Technical Engineering, Computer Science, Mathematics & UOR-R4 Architecture Q&A (1,100 samples)
    core_qa = [
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
    ]
    for _ in range(50):
        for q, a in core_qa:
            samples.append((q, [(f" {a}", False)]))

    rng.shuffle(samples)
    return samples


def export_vocab_bytes(tok: Tokenizer, vocab_out: pathlib.Path):
    with vocab_out.open("wb") as vf:
        vf.write(b"UORVOCB9")
        vf.write(struct.pack("<I", 4096))
        for tid in range(4096):
            if tid in (0, 1, 2):
                raw = f"<|special_{tid}|>".encode("utf-8")
            else:
                decoded = tok.decode([tid], skip_special_tokens=False)
                raw = decoded.encode("utf-8", errors="replace")
            if len(raw) > 255:
                raw = raw[:255]
            vf.write(struct.pack("<B", len(raw)))
            vf.write(raw)


def main():
    repo_root = pathlib.Path(__file__).resolve().parent.parent
    tok_path = repo_root / "crates" / "wasm" / "src" / "tokenizer.json"
    if not tok_path.exists():
        tok_path = pathlib.Path("tokenizer.json")
    tok = Tokenizer.from_file(str(tok_path))

    out_path = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "v9_corpus.bin")
    vocab_out = out_path.parent / "v9_vocab_bytes.bin"
    export_vocab_bytes(tok, vocab_out)

    rng = random.Random(20261110)
    samples = build_samples(rng)

    bos_id = 0
    encoded_records = []
    total_tokens = 0
    for prompt, segments in samples:
        p_ids = [bos_id] + tok.encode(f"User: {prompt}\nAssistant:").ids
        r_ids, r_copy = encode_tagged_reply(tok, segments)
        if len(p_ids) + len(r_ids) > 256:
            keep = max(8, 256 - len(p_ids) - 1)
            r_ids = r_ids[:keep] + [1]
            r_copy = r_copy[:keep] + [0]
        if len(p_ids) >= 4 and len(r_ids) >= 2:
            encoded_records.append((p_ids, r_ids, r_copy))
            total_tokens += len(p_ids) + len(r_ids)

    with out_path.open("wb") as f:
        f.write(b"UORCORP2")
        f.write(struct.pack("<I", len(encoded_records)))
        for p_ids, r_ids, r_copy in encoded_records:
            f.write(struct.pack("<HH", len(p_ids), len(r_ids)))
            for tid in p_ids:
                f.write(struct.pack("<H", tid))
            for tid in r_ids:
                f.write(struct.pack("<H", tid))
            f.write(bytes(r_copy))

    print(
        f"Wrote {len(encoded_records)} compositional sequences ({total_tokens} ByteBPE-4096 tokens) -> {out_path} ({out_path.stat().st_size} bytes) + {vocab_out}"
    )


if __name__ == "__main__":
    main()
