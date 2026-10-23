//! Canonical SHA-256 text used to bind conformance evidence bytes.

use sha2::{Digest, Sha256};

/// Hash bytes as exactly 64 lowercase hexadecimal digits.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
