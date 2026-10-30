//! Fixture loading and workspace directory helpers.

use std::path::{Path, PathBuf};

/// Get the repository root directory path.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|p| p.join("Cargo.lock").exists() && p.join("Justfile").exists())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Read text file relative to repository root.
pub fn read_fixture(rel_path: &str) -> std::io::Result<String> {
    let full_path = repo_root().join(rel_path);
    std::fs::read_to_string(full_path)
}

/// Read binary bytes relative to repository root.
pub fn read_fixture_bytes(rel_path: &str) -> std::io::Result<Vec<u8>> {
    let full_path = repo_root().join(rel_path);
    std::fs::read(full_path)
}
