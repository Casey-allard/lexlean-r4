//! Pure-Rust orchestrator, Stitch 2-of-3 XOR parity generator, and receipt verifier
//! for UOR-R4-GLLM v11 (`UORGLLM9`, `version = 11`).
//!
//! Supports two modes (zero Python anywhere):
//! 1. `--shard <full_16l.uorgllm> <weights_dir> <boot_receipt.json> <full_receipt.json> <out_receipt.json>`
//!    Verifies `UORGLLM9` `v11` container integrity & CORDIC Q12 RoPE self-check,
//!    splits into 1 MiB (`1_048_576` B) shards (`shard_000.bin..shard_078.bin`),
//!    builds `manifest.json` (`uor-r4.compositional-gllm/11`),
//!    builds 2-of-3 XOR parity shards (`parity_000.bin..parity_039.bin`) + `manifest.stitch.json` (`stitch/xor-2-of-3/1`),
//!    verifies byte-exact XOR recovery on every group, and emits `rtx4090_gllm_v11_receipt.json`.
//! 2. `--run-all <work_dir> <tokenizer_json> <out_boot_model> <out_weights_dir> <out_receipt_json>`
//!    Compiles `build_v11_corpus.rs` + `train_v11_gpu.cu`, trains `6L x 128D` (`quant_bits=4`) and
//!    `16L x 256D` (`quant_bits=8`), then runs sharding + Stitch parity + receipt emission.

#[path = "bpe_json.rs"]
mod bpe_json;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use bpe_json::{json_escape, sha256_bytes, sha256_hex, JsonVal};

const SHARD_MAX_BYTES: usize = 1_048_576;
const MAX_SEQ: usize = 1024;
const CORDIC_ATAN_BAM: [u32; 28] = [
    536_870_912,
    316_933_406,
    167_458_907,
    85_004_756,
    42_667_331,
    21_354_465,
    10_679_838,
    5_340_245,
    2_670_163,
    1_335_087,
    667_544,
    333_772,
    166_886,
    83_443,
    41_722,
    20_861,
    10_430,
    5_215,
    2_608,
    1_304,
    652,
    326,
    163,
    81,
    41,
    20,
    10,
    5,
];
const ROPE_OMEGA_BAM: [u32; 8] = [
    20_506_958,
    6_835_653,
    2_050_696,
    683_565,
    205_070,
    68_357,
    20_507,
    6_836,
];

fn cordic_cos_sin_q12(bam: u32) -> (i16, i16) {
    let quad = bam >> 30;
    let rem = bam & 0x3FFF_FFFF;
    if rem == 0 {
        return match quad {
            0 => (4096, 0),
            1 => (0, 4096),
            2 => (-4096, 0),
            _ => (0, -4096),
        };
    }
    let mut x: i64 = 652_032_874;
    let mut y: i64 = 0;
    let mut z: i64 = rem as i64;
    for (i, &atan) in CORDIC_ATAN_BAM.iter().enumerate() {
        let dx = x >> i;
        let dy = y >> i;
        let dz = atan as i64;
        if z >= 0 {
            x -= dy;
            y += dx;
            z -= dz;
        } else {
            x += dy;
            y -= dx;
            z += dz;
        }
    }
    let rx = (x + if x >= 0 { 1 << 17 } else { -(1 << 17) }) >> 18;
    let ry = (y + if y >= 0 { 1 << 17 } else { -(1 << 17) }) >> 18;
    let c0 = rx.clamp(-4096, 4096) as i16;
    let s0 = ry.clamp(-4096, 4096) as i16;
    match quad {
        0 => (c0, s0),
        1 => (-s0, c0),
        2 => (-c0, -s0),
        _ => (s0, -c0),
    }
}

/// Self-verify integer Q12 CORDIC RoPE invariants across `0..1024`.
fn verify_cordic_rope_selftest() -> Result<(i16, i16, i16, i16), String> {
    let (c0, s0) = cordic_cos_sin_q12(0);
    if c0 != 4096 || s0 != 0 {
        return Err(format!("pos=0 pair0 expected (4096, 0), got ({c0}, {s0})"));
    }
    for pos in 0..MAX_SEQ {
        for &omega in &ROPE_OMEGA_BAM {
            let (c, s) = cordic_cos_sin_q12((pos as u32).wrapping_mul(omega));
            let r2 = (c as i64) * (c as i64) + (s as i64) * (s as i64);
            let target = 4096_i64 * 4096_i64;
            if (r2 - target).abs() > 18_000 {
                return Err(format!("CORDIC radius drift at pos={pos}: r2={r2}"));
            }
        }
    }
    let (c1, s1) = cordic_cos_sin_q12(ROPE_OMEGA_BAM[0]);
    let (cl, sl) = cordic_cos_sin_q12((1023u32).wrapping_mul(ROPE_OMEGA_BAM[0]));
    Ok((c1, s1, cl, sl))
}

#[derive(Clone, Debug)]
struct UorgllmHeader {
    version: u32,
    vocab_size: u32,
    d_model: u32,
    n_layers: u32,
    n_heads: u32,
    lut_groups: u32,
    quant_bits: u32,
}

fn read_u32_le(bytes: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
}

fn parse_and_verify_container(bytes: &[u8]) -> Result<(UorgllmHeader, String), String> {
    if bytes.len() < 80 {
        return Err(format!("container truncated: {} bytes", bytes.len()));
    }
    if &bytes[0..8] != b"UORGLLM9" {
        return Err("invalid UORGLLM9 magic".to_string());
    }
    let hdr = UorgllmHeader {
        version: read_u32_le(bytes, 8),
        vocab_size: read_u32_le(bytes, 12),
        d_model: read_u32_le(bytes, 16),
        n_layers: read_u32_le(bytes, 20),
        n_heads: read_u32_le(bytes, 24),
        lut_groups: read_u32_le(bytes, 28),
        quant_bits: read_u32_le(bytes, 44),
    };
    let turn_mem = read_u32_le(bytes, 32);
    let dense_mm = read_u32_le(bytes, 36);
    let softmax = read_u32_le(bytes, 40);
    if hdr.version != 11 {
        return Err(format!("expected version=11, got {}", hdr.version));
    }
    if turn_mem != 0 || dense_mm != 0 || softmax != 0 {
        return Err("non-zero turn_memory/dense_matmul/softmax header counter".to_string());
    }
    if hdr.vocab_size != 4096 || hdr.d_model < 64 || hdr.n_layers < 4 || hdr.n_heads < 4 {
        return Err("invalid UORGLLM9 geometry".to_string());
    }
    if !(hdr.quant_bits == 4 || hdr.quant_bits == 8) {
        return Err(format!("unsupported quant_bits={}", hdr.quant_bits));
    }
    let payload_digest = sha256_bytes(&bytes[80..]);
    if payload_digest != bytes[48..80] {
        return Err("UORGLLM9 payload SHA-256 mismatch".to_string());
    }
    Ok((hdr, sha256_hex(bytes)))
}

struct ShardMeta {
    index: usize,
    file: String,
    offset: usize,
    byte_length: usize,
    sha256: String,
    data: Vec<u8>,
}

fn xor_pair(left: &[u8], right: &[u8]) -> Vec<u8> {
    let n = left.len().max(right.len());
    let mut out = vec![0u8; n];
    for i in 0..n {
        let a = left.get(i).copied().unwrap_or(0);
        let b = right.get(i).copied().unwrap_or(0);
        out[i] = a ^ b;
    }
    out
}

fn clear_stale_shards(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        fs::create_dir_all(dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
        return Ok(());
    }
    let rd = fs::read_dir(dir).map_err(|e| format!("readdir {}: {e}", dir.display()))?;
    for ent in rd {
        let ent = ent.map_err(|e| format!("readdir entry: {e}"))?;
        let name = ent.file_name().to_string_lossy().to_string();
        if (name.starts_with("shard_") || name.starts_with("parity_")) && name.ends_with(".bin") {
            fs::remove_file(ent.path()).map_err(|e| format!("rm {name}: {e}"))?;
        }
    }
    Ok(())
}

fn format_shard_ref_json(file: &str, byte_length: usize, sha256: &str) -> String {
    format!(
        "{{\"file\": {}, \"byte_length\": {}, \"sha256\": {}}}",
        json_escape(file),
        byte_length,
        json_escape(sha256)
    )
}

fn shard_and_build_stitch(
    full_model_path: &Path,
    weights_dir: &Path,
    boot_receipt_path: &Path,
    full_receipt_path: &Path,
    out_receipt_path: &Path,
    rope_selftest: (i16, i16, i16, i16),
) -> Result<(), String> {
    let full_bytes = fs::read(full_model_path)
        .map_err(|e| format!("read {}: {e}", full_model_path.display()))?;
    let (hdr, container_sha) = parse_and_verify_container(&full_bytes)?;
    clear_stale_shards(weights_dir)?;

    let mut shards: Vec<ShardMeta> = Vec::new();
    let mut offset = 0usize;
    let mut idx = 0usize;
    while offset < full_bytes.len() {
        let end = (offset + SHARD_MAX_BYTES).min(full_bytes.len());
        let chunk = full_bytes[offset..end].to_vec();
        let file = format!("shard_{idx:03}.bin");
        let sha = sha256_hex(&chunk);
        fs::write(weights_dir.join(&file), &chunk).map_err(|e| format!("write {file}: {e}"))?;
        shards.push(ShardMeta {
            index: idx,
            file,
            offset,
            byte_length: chunk.len(),
            sha256: sha,
            data: chunk,
        });
        offset = end;
        idx += 1;
    }

    // Write `manifest.json` matching `uor-r4.compositional-gllm/11` & `gllm_load_v10_shards`
    let mut manifest_json = String::new();
    manifest_json.push_str("{\n");
    manifest_json.push_str("  \"format\": \"UORGLLM9\",\n");
    manifest_json.push_str("  \"schema\": \"uor-r4.compositional-gllm/11\",\n");
    manifest_json.push_str(&format!("  \"version\": {},\n", hdr.version));
    manifest_json.push_str("  \"model_version\": \"v11\",\n");
    manifest_json.push_str(&format!(
        "  \"container_sha256\": {},\n",
        json_escape(&container_sha)
    ));
    manifest_json.push_str(&format!("  \"total_bytes\": {},\n", full_bytes.len()));
    manifest_json.push_str(&format!("  \"shard_max_bytes\": {SHARD_MAX_BYTES},\n"));
    manifest_json.push_str(&format!("  \"shard_count\": {},\n", shards.len()));
    manifest_json.push_str(&format!("  \"vocab_size\": {},\n", hdr.vocab_size));
    manifest_json.push_str(&format!("  \"d_model\": {},\n", hdr.d_model));
    manifest_json.push_str(&format!("  \"n_layers\": {},\n", hdr.n_layers));
    manifest_json.push_str(&format!("  \"n_heads\": {},\n", hdr.n_heads));
    manifest_json.push_str(&format!("  \"lut_groups\": {},\n", hdr.lut_groups));
    manifest_json.push_str("  \"turn_memory_entries\": 0,\n");
    manifest_json.push_str("  \"dense_matmul_ops\": 0,\n");
    manifest_json.push_str("  \"softmax_ops\": 0,\n");
    manifest_json.push_str(&format!("  \"quant_bits\": {},\n", hdr.quant_bits));
    manifest_json.push_str(&format!("  \"max_seq_len\": {MAX_SEQ},\n"));
    manifest_json.push_str("  \"shards\": [\n");
    for (i, s) in shards.iter().enumerate() {
        let comma = if i + 1 == shards.len() { "" } else { "," };
        let path_str = format!("/weights/v10/{}", s.file);
        manifest_json.push_str(&format!(
            "    {{\"index\": {}, \"file\": {}, \"path\": {}, \"byte_offset\": {}, \"byte_length\": {}, \"sha256\": {}}}{}\n",
            s.index,
            json_escape(&s.file),
            json_escape(&path_str),
            s.offset,
            s.byte_length,
            json_escape(&s.sha256),
            comma
        ));
    }
    manifest_json.push_str("  ]\n}\n");
    fs::write(weights_dir.join("manifest.json"), manifest_json.as_bytes())
        .map_err(|e| format!("write manifest.json: {e}"))?;

    // Write 2-of-3 XOR parity shards (`parity_000.bin ..`) and `manifest.stitch.json`
    let group_count = shards.len().div_ceil(2);
    let mut groups_json = String::new();
    let mut recovered_all: Vec<u8> = Vec::with_capacity(full_bytes.len());
    for k in 0..group_count {
        let left = &shards[2 * k];
        let right = shards.get(2 * k + 1);
        let right_bytes: &[u8] = right.map(|r| r.data.as_slice()).unwrap_or(&[]);
        let parity_bytes = xor_pair(&left.data, right_bytes);
        let pfile = format!("parity_{k:03}.bin");
        let psha = sha256_hex(&parity_bytes);
        fs::write(weights_dir.join(&pfile), &parity_bytes)
            .map_err(|e| format!("write {pfile}: {e}"))?;

        // Exhaustive self-verification: recover lost left AND lost right via XOR
        let mut rec_left = xor_pair(right_bytes, &parity_bytes);
        rec_left.truncate(left.byte_length);
        if rec_left != left.data || sha256_hex(&rec_left) != left.sha256 {
            return Err(format!(
                "Stitch recovery failed for left shard {}",
                left.file
            ));
        }
        recovered_all.extend_from_slice(&rec_left);

        if let Some(r) = right {
            let mut rec_right = xor_pair(&left.data, &parity_bytes);
            rec_right.truncate(r.byte_length);
            if rec_right != r.data || sha256_hex(&rec_right) != r.sha256 {
                return Err(format!("Stitch recovery failed for right shard {}", r.file));
            }
            recovered_all.extend_from_slice(&rec_right);
        }

        let right_json = match right {
            Some(r) => format_shard_ref_json(&r.file, r.byte_length, &r.sha256),
            None => "null".to_string(),
        };
        let comma = if k + 1 == group_count { "" } else { "," };
        groups_json.push_str(&format!(
            "    {{\"group_index\": {k}, \"left\": {}, \"right\": {}, \"parity\": {}}}{comma}\n",
            format_shard_ref_json(&left.file, left.byte_length, &left.sha256),
            right_json,
            format_shard_ref_json(&pfile, parity_bytes.len(), &psha),
        ));
    }
    if recovered_all != full_bytes || sha256_hex(&recovered_all) != container_sha {
        return Err("reassembled Stitch-recovered container SHA-256 mismatch".to_string());
    }

    let stitch_json = format!(
        "{{\n\
  \"schema\": \"stitch/xor-2-of-3/1\",\n\
  \"scheme\": \"xor-2-of-3\",\n\
  \"threshold\": 2,\n\
  \"count\": 3,\n\
  \"base_manifest\": \"manifest.json\",\n\
  \"format\": \"UORGLLM9\",\n\
  \"container_sha256\": {},\n\
  \"total_bytes\": {},\n\
  \"data_shard_count\": {},\n\
  \"parity_shard_count\": {},\n\
  \"groups\": [\n{}\
  ],\n\
  \"mirrors\": []\n\
}}\n",
        json_escape(&container_sha),
        full_bytes.len(),
        shards.len(),
        group_count,
        groups_json
    );
    fs::write(
        weights_dir.join("manifest.stitch.json"),
        stitch_json.as_bytes(),
    )
    .map_err(|e| format!("write manifest.stitch.json: {e}"))?;

    // Validate both JSON sub-receipts parse cleanly and combine into consolidated receipt
    let boot_raw = fs::read_to_string(boot_receipt_path)
        .map_err(|e| format!("read {}: {e}", boot_receipt_path.display()))?;
    let full_raw = fs::read_to_string(full_receipt_path)
        .map_err(|e| format!("read {}: {e}", full_receipt_path.display()))?;
    let boot_val = JsonVal::parse(&boot_raw)
        .map_err(|e| format!("parse {}: {e}", boot_receipt_path.display()))?;
    let full_val = JsonVal::parse(&full_raw)
        .map_err(|e| format!("parse {}: {e}", full_receipt_path.display()))?;

    let boot_bytes = boot_val
        .get("container_bytes")
        .and_then(|v| v.as_usize())
        .unwrap_or(0);
    let combined = format!(
        "{{\n\
  \"schema\": \"uor-r4.gllm-v11.rtx4090-receipt/1\",\n\
  \"format\": \"UORGLLM9\",\n\
  \"version\": 11,\n\
  \"zero_python\": true,\n\
  \"zero_float_runtime\": true,\n\
  \"max_seq_len\": {MAX_SEQ},\n\
  \"rope_cordic_q12_selftest\": {{\"pos1_pair0\": [{}, {}], \"pos1023_pair0\": [{}, {}]}},\n\
  \"embedded_6l_128d\": {},\n\
  \"hosted_16l_256d\": {},\n\
  \"hosted_shards\": {{\"shard_max_bytes\": {SHARD_MAX_BYTES}, \"shard_count\": {}, \"parity_shard_count\": {}, \"container_sha256\": {}, \"stitch_parity_verified\": true}},\n\
  \"embedded_bytes\": {boot_bytes}\n\
}}\n",
        rope_selftest.0,
        rope_selftest.1,
        rope_selftest.2,
        rope_selftest.3,
        boot_raw.trim(),
        full_raw.trim(),
        shards.len(),
        group_count,
        json_escape(&container_sha),
    );
    if let Some(par) = out_receipt_path.parent() {
        if !par.as_os_str().is_empty() {
            fs::create_dir_all(par).map_err(|e| format!("mkdir receipt dir: {e}"))?;
        }
    }
    fs::write(out_receipt_path, combined.as_bytes())
        .map_err(|e| format!("write {}: {e}", out_receipt_path.display()))?;
    let _ = full_val;
    println!(
        "[train-v11] Sharded {} ({} bytes) -> {} data shards + {} Stitch parity shards (SHA-256={container_sha}); receipt={}",
        full_model_path.display(),
        full_bytes.len(),
        shards.len(),
        group_count,
        out_receipt_path.display(),
    );
    Ok(())
}

fn run_cmd(prog: &str, args: &[&str], cwd: &Path, label: &str) -> Result<(), String> {
    println!("[train-v11] running {label}: {prog} {}", args.join(" "));
    let status = Command::new(prog)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|e| format!("spawn {label}: {e}"))?;
    if !status.success() {
        return Err(format!("{label} exited with {status}"));
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let rope_selftest = verify_cordic_rope_selftest()?;
    println!(
        "[train-v11] CORDIC Q12 RoPE self-test OK: pos1=({}, {}), pos1023=({}, {})",
        rope_selftest.0, rope_selftest.1, rope_selftest.2, rope_selftest.3
    );

    let args: Vec<String> = env::args().collect();
    if args.len() >= 7 && args[1] == "--shard" {
        return shard_and_build_stitch(
            Path::new(&args[2]),
            Path::new(&args[3]),
            Path::new(&args[4]),
            Path::new(&args[5]),
            Path::new(&args[6]),
            rope_selftest,
        );
    }

    // Full end-to-end pure-Rust + CUDA pipeline (`--run-all <work_dir> <tokenizer.json> <boot_out> <weights_dir> <receipt_out>`)
    let work_dir = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let tok_json = args
        .get(3)
        .map(PathBuf::from)
        .unwrap_or_else(|| work_dir.join("tokenizer.json"));
    let boot_out = args
        .get(4)
        .map(PathBuf::from)
        .unwrap_or_else(|| work_dir.join("bootstrap_v9.uorgllm"));
    let weights_dir = args
        .get(5)
        .map(PathBuf::from)
        .unwrap_or_else(|| work_dir.join("weights_v11"));
    let receipt_out = args
        .get(6)
        .map(PathBuf::from)
        .unwrap_or_else(|| work_dir.join("rtx4090_gllm_v11_receipt.json"));

    let corpus_bin = work_dir.join("build_v11_corpus");
    let gpu_bin = work_dir.join("train_v11_gpu");
    run_cmd(
        "rustc",
        &[
            "--edition",
            "2021",
            "-O",
            "build_v11_corpus.rs",
            "-o",
            corpus_bin.to_str().unwrap_or("build_v11_corpus"),
        ],
        &work_dir,
        "compile build_v11_corpus",
    )?;
    let full_corpus = work_dir.join("v11_corpus.bin");
    let tok_s = tok_json.to_str().ok_or("non-utf8 tokenizer path")?;
    let status = Command::new(&corpus_bin)
        .arg(&full_corpus)
        .env("UOR_TOKENIZER_JSON", tok_s)
        .current_dir(&work_dir)
        .status()
        .map_err(|e| format!("run build_v11_corpus: {e}"))?;
    if !status.success() {
        return Err(format!("build_v11_corpus exited with {status}"));
    }

    run_cmd(
        "/usr/local/cuda/bin/nvcc",
        &[
            "-O3",
            "-arch=sm_89",
            "train_v11_gpu.cu",
            "-o",
            gpu_bin.to_str().unwrap_or("train_v11_gpu"),
        ],
        &work_dir,
        "compile train_v11_gpu",
    )?;

    let boot_corpus = work_dir.join("v11_boot_corpus.bin");
    let boot_eval = work_dir.join("v11_boot_eval.bin");
    let full_eval = work_dir.join("v11_eval.bin");
    let vocab_bin = work_dir.join("v11_vocab_bytes.bin");
    let prompts_bin = work_dir.join("v11_eval_prompts.bin");
    let boot_receipt = work_dir.join("boot_receipt.json");
    let full_model = work_dir.join("v11_16l_256d.uorgllm");
    let full_receipt = work_dir.join("full_receipt.json");

    run_cmd(
        gpu_bin.to_str().unwrap_or("./train_v11_gpu"),
        &[
            boot_corpus.to_str().unwrap(),
            boot_eval.to_str().unwrap(),
            vocab_bin.to_str().unwrap(),
            prompts_bin.to_str().unwrap(),
            boot_out.to_str().unwrap(),
            boot_receipt.to_str().unwrap(),
            "--layers",
            "6",
            "--d_model",
            "128",
            "--quant_bits",
            "4",
            "--epochs",
            "6",
            "--batch",
            "128",
        ],
        &work_dir,
        "train 6L x 128D",
    )?;

    run_cmd(
        gpu_bin.to_str().unwrap_or("./train_v11_gpu"),
        &[
            full_corpus.to_str().unwrap(),
            full_eval.to_str().unwrap(),
            vocab_bin.to_str().unwrap(),
            prompts_bin.to_str().unwrap(),
            full_model.to_str().unwrap(),
            full_receipt.to_str().unwrap(),
            "--layers",
            "16",
            "--d_model",
            "256",
            "--quant_bits",
            "8",
            "--epochs",
            "5",
            "--batch",
            "64",
        ],
        &work_dir,
        "train 16L x 256D",
    )?;

    shard_and_build_stitch(
        &full_model,
        &weights_dir,
        &boot_receipt,
        &full_receipt,
        &receipt_out,
        rope_selftest,
    )
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("[train-v11] error: {err}");
            ExitCode::FAILURE
        }
    }
}
