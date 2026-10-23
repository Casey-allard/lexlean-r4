//! Host-only regression for canonical conformance evidence digest identities.

#[path = "support/sha256.rs"]
mod sha256;

#[test]
fn sha256_identity_is_lowercase_and_zero_padded() {
    for (input, expected) in [
        (
            "",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            "abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            "286",
            "00328ce57bbc14b33bd6695bc8eb32cdf2fb5f3a7d89ec14a42825e15d39df60",
        ),
    ] {
        assert_eq!(sha256::sha256_hex(input.as_bytes()), expected);
    }
}
