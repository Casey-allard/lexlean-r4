//! MVM-Wasm RV64IMAC ELF64 admission and SpiralCore virtio-vsock zero-NIC controls (`R4-37`).

mod support;

#[test]
fn enforce_mvm_spiralcore_vsock_r4_37() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/SpiralCoreVsockControls.lex.tex",
        "SpiralCoreVsockControls",
        "a5e99e860965a8a6df7872d5327c75c6b4385cc74026c5ebcb66407e8b66014c",
        &["SpiralCoreVsockControls", "R4.SpiralCoreVsock"],
    )
}
