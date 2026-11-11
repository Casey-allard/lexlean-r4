# MVM-Wasm Sandbox and Compositional UOR-R4-GLLM v9 Implementation Plan

**Goal:** Deliver (1) `R4-37` `MVM-Wasm` `RV64IMAC` 64-bit RISC-V Linux MicroVM with `SpiralCore` 16-byte `virtio-vsock` zero-NIC networking, 64 KB Copy-on-Write block storage, and an in-sandbox C-to-`EM_RISCV` ELF64 compiler wired into the Studio harness, followed in order by (2) `R4-38` `UOR-R4-GLLM v9` compositional multi-layer geometric language model trained/distilled on Vast.ai RTX 5090 with `Cl(0,6)` Spin-Hadamard rotations, T-MAC group-LUT integer projections, Spin-RoPE GQA KV cache, `ServedHamming` 256-bit KV coarse filtering, OPFS `FileSystemSyncAccessHandle` shard streaming, WebGPU WGSL acceleration, and zero `ServedTurnMemory` canned reply replay.

**Architecture:**
- `R4-37` (`src/R4/SpiralCoreVsock.lex.tex`, `crates/wasm/src/mvm.rs`, `crates/wasm/src/mvm_cc.rs`, `crates/wasm/src/mvm_initramfs.bin`): Pure safe Rust `RV64IMAC` processor and Linux `riscv64` syscall executive booting real `riscv64-linux-gnu-gcc` ELF64 binaries (`/sbin/mvm-agent`, `/bin/mvmbox`, `/bin/vsock_probe`), enforcing `AF_VSOCK = 40` admission and `AF_INET` refusal, routing 16-byte `SpiralCore` frames across CIDs with host secret placeholder injection, and syncing 64 KB CoW blocks with Studio `/workspace` and Worker OPFS handles.
- `R4-38` (`src/R4/CompositionalGllm.lex.tex`, `crates/wasm/src/gllm_v9.rs`, `gpu/weights/v9/`, `public/lexlean_r4.js`): Multi-layer transformer-equivalent geometric decoder (`UORGLLM9`) with `turn_memory_entries == 0`, reading the full tokenized prompt into a multi-layer GQA KV cache, rotating activations via `Cl(0,6)` fast Walsh-Hadamard transforms, evaluating `2-bit`/`3-bit` Q12 group-LUT projections compiled on RTX 5090 from an Apache-2.0 instruction/coding backbone, and executing `<tool_call>` JSON loops against `MVM-Wasm`.

---

## Task 1: Authoritative Model and Failing Tests for `R4-37` (`SpiralCoreVsock` & `MVM-Wasm`)
1. Author `src/R4/SpiralCoreVsock.lex.tex` and `tests/lexlean/SpiralCoreVsockControls.lex.tex` (`lexlean/semantic-module/2`).
2. Register `R4-37` in `model/ids.toml`, add `features/suites/r4-spiralcore-vsock.feature`, `crates/conformance/tests/spiralcore_vsock.rs`, `crates/wasm/src/lexlean_generated.rs` (`r4_spiralcore_vsock`), update `crates/conformance/tests/bdd.rs`, and regenerate `CONFORMANCE.md` via `cargo run -q -p xtask -- check-model --write`.
3. Write `crates/wasm/tests/mvm_spiralcore_sandbox.rs` covering ELF64 `EM_RISCV` execution, shell pipeline syscalls, C-to-ELF64 compilation/execution, CoW sector snapshots, `AF_INET` zero-NIC refusal, `AF_VSOCK` `SpiralCore` 16-byte framing, East-West CID routing, and host secret placeholder substitution; run `cargo test` to observe RED failure before implementing `mvm.rs`.

## Task 2: Cross-Compile Real `riscv64` ELF64 Userland and Implement `MVM-Wasm` + `mvm_cc`
1. Cross-compile freestanding `RV64IMAC` ELF64 binaries (`/sbin/mvm-agent`, `/bin/mvmbox`, `/bin/vsock_probe`) on `198.53.64.194:34233` with `riscv64-linux-gnu-gcc` and pack `crates/wasm/src/mvm_initramfs.bin`.
2. Implement `crates/wasm/src/mvm.rs` (`RV64IMAC` CPU, Linux `riscv64` syscalls, 64 KB CoW block device, `SpiralCore` 16-byte `virtio-vsock` switch, secret placeholder proxy) and `crates/wasm/src/mvm_cc.rs` (C-to-`EM_RISCV` ELF64 compiler).
3. Wire `mvm_exec`, `mvm_compile_run`, `mvm_vsock_frame`, and `mvm_status` into `crates/wasm/src/lib.rs`, `crates/wasm/src/api.rs`, `public/lexlean_r4.js` (Dedicated Worker OPFS CoW persistence), and `public/app.js` (`LexLeanTerminal` + Composer tool cards).
4. Verify GREEN across `mvm_spiralcore_sandbox` and `spiralcore_vsock`, then plant and revert a gate defect.

## Task 3: Authoritative Model and Failing Tests for `R4-38` (`CompositionalGllm` & `UOR-R4-GLLM v9`)
1. Author `src/R4/CompositionalGllm.lex.tex` and `tests/lexlean/CompositionalGllmControls.lex.tex` modeling `UORGLLM9` admission, `turn_memory_entries == 0` anti-stamping enforcement, `Cl(0,6)` Hadamard energy conservation, group-LUT bounds, and `ServedHamming` 2-stage KV cache admission.
2. Register `R4-38` in `model/ids.toml`, `features/suites/r4-compositional-gllm.feature`, `crates/conformance/tests/compositional_gllm.rs`, `crates/wasm/src/lexlean_generated.rs` (`r4_compositional_gllm`), update `crates/conformance/tests/bdd.rs`, and run `cargo run -q -p xtask -- check-model --write`.
3. Write `crates/wasm/tests/gllm_v9_compositional.rs` testing unseen variable bindings, unseen code comprehension, anti-stamping invariance (`turn_memory_entries == 0`), and `<tool_call>` roundtrip execution with `MVM-Wasm`; confirm RED failure first.

## Task 4: GPU QAT / `Cl(0,6)` Spin-Hadamard / Group-LUT Compilation on RTX 5090 & Runtime Integration
1. On Vast.ai RTX 5090 (`198.53.64.194:34233`), fine-tune and calibrate an Apache-2.0 instruction/coding backbone (`Qwen/Qwen2.5-0.5B-Instruct`) on Studio coding, reasoning, and `MVM-Wasm` `<tool_call>` trajectories, prune to a byte-complete 32,768 BPE vocabulary, apply `Cl(0,6)` Walsh-Hadamard block rotations, quantize linear projections into Hessian-optimized Q12 group-LUT tables (`UORGLLM9`), and emit verified 512 KB shards to `gpu/weights/v9/` plus a compact embedded `bootstrap_v9.uorgllm` for offline/unit-test parity.
2. Implement `crates/wasm/src/gllm_v9.rs` (multi-layer Spin-RoPE GQA KV cache, `Cl(0,6)` fast Walsh-Hadamard transform, `wasm_simd128` T-MAC LUT matvec, `ServedHamming` 256-bit KV filter, SwiGLU FFN, `R4.Prediction` token admission, and live `MVM-Wasm` `<tool_call>` loop) and route `studio_v1_messages`, `studio_v1_chat_completions`, and Studio `chat` through `gllm_v9` with zero `ServedTurnMemory` string replay.
3. Add WebGPU WGSL LUT projection shaders and Dedicated Worker `FileSystemSyncAccessHandle` OPFS shard hydration in `public/lexlean_r4.js` while preserving the strict 6-file `public/` closure and zero forbidden substrings.

## Task 5: Rebuild Browser Closure, Plant Defects, Full Gate Verification, and Deploy
1. Rebuild `public/lexlean_r4_bg.wasm` and update `public/provenance.json` SHA-256 digests.
2. Plant and verify gate rejection for `R4-37` and `R4-38`, restore clean code, and record verification receipts.
3. Run `cargo run -q -p xtask -- validate`, `cargo clippy`, `cargo fmt --check`, `cargo test --workspace`, all Node E2E harnesses (`tests/e2e/*.mjs`), and live browser verification via `chrome-devtools-mcp`.
4. Commit, rebase cleanly onto `origin/main`, push, and verify green GitHub Actions `pages` deployment.
