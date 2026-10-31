//! Complete typed byte-BPE semantics; raw JSON and identity are separate gates.

mod support;

#[test]
fn exact_tokenizer_kernel_r4_16() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (
            "tests/lexlean/TokenizerKernelSurfaceControls.lex.tex",
            "TokenizerKernelSurfaceControls",
            "d35601ce6fb324ebe3ba5277cc496b0f338b9be1e16cba61adc8bd5740159181",
        ),
        (
            "tests/lexlean/TokenizerKernelControls.lex.tex",
            "TokenizerKernelControls",
            "055d3e2dc8e19f630e4d9b9f6583dbae236e4be9796281a0e3fb5176f4da1a67",
        ),
        (
            "tests/lexlean/TokenizerKernelBoundaryControls.lex.tex",
            "TokenizerKernelBoundaryControls",
            "6505f13cf095fcc2c76e04e4380dda2e3bb9bd548fb26579b85cba0cdf98bc33",
        ),
        (
            "tests/lexlean/TokenizerKernelLengthControls.lex.tex",
            "TokenizerKernelLengthControls",
            "eab7082f59c84862c7387e23e73010a8dbe812f28483242e48aeb54031185551",
        ),
        (
            "tests/lexlean/TokenizerKernelUnicodeControls.lex.tex",
            "TokenizerKernelUnicodeControls",
            "a53867dd2739599c3befd778d0fe0b80d10e956cbccec408e8787c3565992b8e",
        ),
        (
            "tests/lexlean/TokenizerKernelProgressControls.lex.tex",
            "TokenizerKernelProgressControls",
            "0881fe66fec1b81e3f206fb3d98cc92e003c9f67447343b4ed06bf3eff5e7581",
        ),
    ];
    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let next_idx = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        let workers = 3;
        for _ in 0..workers {
            let tx = tx.clone();
            let next_idx = &next_idx;
            s.spawn(move || {
                loop {
                    let i = next_idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= cases.len() {
                        break;
                    }
                    let (path, module, digest) = cases[i];
                    if let Err(e) = support::verify_controls(
                        path,
                        module,
                        digest,
                        &[
                            "R4.TokenizerKernel",
                            "R4.TokenizerText",
                            "R4.TokenizerTextUnicode",
                            module,
                        ],
                    ) {
                        let _ = tx.send(Err(e.to_string()));
                        return;
                    }
                }
                let _ = tx.send(Ok(()));
            });
        }
    });
    drop(tx);
    while let Ok(res) = rx.recv() {
        res.map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    }
    Ok(())
}
