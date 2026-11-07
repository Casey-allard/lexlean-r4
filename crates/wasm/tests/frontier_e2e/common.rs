//! Common utilities and fail-closed assertion helpers for Frontier E2E Tests.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Maximum number of geometric buckets: 16 (E8) * 64 (Clifford) * 17 (Icosian) = 17,408
pub const GEOMETRIC_BUCKET_COUNT: usize = 17_408;

/// Size of the radix directory table: 17,408 buckets + 1 terminator = 17,409 entries
pub const RADIX_DIRECTORY_ENTRIES: usize = 17_409;

/// Fixed resident recurrence trunk size in bytes (22.5 MB)
pub const RESIDENT_TRUNK_BYTES: usize = 22_500_000;

/// Maximum active Wasm LRU shard cache budget in bytes (256 MB)
pub const WASM_LRU_CACHE_BUDGET_BYTES: usize = 256 * 1024 * 1024;

/// Maximum active shard count in Wasm linear memory
pub const MAX_ACTIVE_SHARDS: usize = 16;

/// FBS vector width (256 lanes)
pub const FBS_WIDTH: usize = 256;

/// Calculate flat shard index S = (e8 * 64 + clifford) * 17 + icosian
#[inline]
pub fn calculate_shard_id(e8_root: u8, clifford_action: u8, icosian_root: u8) -> usize {
    ((e8_root as usize * 64) + (clifford_action as usize)) * 17 + (icosian_root as usize)
}

/// Invert flat shard index into (e8_root, clifford_action, icosian_root)
#[inline]
pub fn decompose_shard_id(shard_id: usize) -> (u8, u8, u8) {
    let icosian = (shard_id % 17) as u8;
    let rem = shard_id / 17;
    let clifford = (rem % 64) as u8;
    let e8 = (rem / 64) as u8;
    (e8, clifford, icosian)
}

/// Compute 128-bit VSA role-filler dynamic XOR binding / unbinding: F_hat = C ^ R
#[inline]
pub fn vsa_xor_128(a: [u64; 2], b: [u64; 2]) -> [u64; 2] {
    [a[0] ^ b[0], a[1] ^ b[1]]
}

/// SWAR parallel bitwise Hamming distance between two 128-bit VSA hypervectors
#[inline]
pub fn swar_hamming_128(a: [u64; 2], b: [u64; 2]) -> u32 {
    let diff0 = a[0] ^ b[0];
    let diff1 = a[1] ^ b[1];
    diff0.count_ones() + diff1.count_ones()
}

/// Compute SHA-256 hex string from byte slice
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Compute SHA-256 hex string from a file
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(sha256_hex(&bytes))
}

/// Resolve repository root directory
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repository root path")
        .to_path_buf()
}

/// Run a command and return stdout string if successful
pub fn run_cmd_checked(cmd: &str, args: &[&str]) -> Result<String, String> {
    let root = repo_root();
    let xtask_bin = root.join(if cfg!(windows) {
        "target/debug/xtask.exe"
    } else {
        "target/debug/xtask"
    });
    let (resolved_cmd, resolved_args): (PathBuf, Vec<&str>) = if cmd == "cargo"
        && args.len() >= 5
        && &args[..5] == ["run", "-q", "-p", "xtask", "--"]
        && xtask_bin.is_file()
    {
        (xtask_bin, args[5..].to_vec())
    } else if cmd == "cargo" {
        let cargo_bin = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        (PathBuf::from(cargo_bin), args.to_vec())
    } else {
        (PathBuf::from(cmd), args.to_vec())
    };
    let output = Command::new(&resolved_cmd)
        .args(&resolved_args)
        .current_dir(&root)
        .output()
        .map_err(|e| format!("Failed to spawn {}: {e}", resolved_cmd.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Command {cmd} {:?} failed with exit code {:?}: {stderr}",
            args,
            output.status.code()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
