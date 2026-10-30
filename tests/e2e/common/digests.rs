//! Digest calculation and hex encoding utilities for E2E tests.

use sha2::{Digest, Sha256};

/// Compute SHA-256 digest of byte slice and return lowercase hex string.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in result {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

/// Verify that data produces the expected SHA-256 digest hex string.
pub fn verify_sha256(data: &[u8], expected_hex: &str) -> bool {
    sha256_hex(data) == expected_hex.to_ascii_lowercase()
}
