//! End-to-end behavioral and negative security tests for `R4-37`:
//! `MVM-Wasm` RV64IMAC Linux MicroVM, `mvm_cc` C-to-ELF64 compiler,
//! 64 KB Copy-on-Write block storage, and `SpiralCore` `virtio-vsock` zero-NIC switch.

use lexlean_r4_wasm::lexlean_generated::r4_spiralcore_vsock::{
    EgressVerdict, ElfAdmission, FrameVerdict, SocketAdmission,
};
use lexlean_r4_wasm::mvm::{
    admit_elf64_riscv, admit_guest_socket_domain, admit_spiralcore_header, MvmSandbox,
    SpiralCoreHeader, SPIRALCORE_MAGIC,
};
use lexlean_r4_wasm::studio_dispatch;

#[test]
fn test_mvm_boot_and_real_rv64_elf64_userland_r4_37() {
    let mut vm = MvmSandbox::boot_default().expect("default MVM-Wasm snapshot boots");
    assert_eq!(vm.guest_cid(), 3);
    assert_eq!(vm.architecture(), "riscv64-rv64imac");

    let agent_elf = vm
        .read_guest_file("/sbin/mvm-agent")
        .expect("/sbin/mvm-agent exists in initramfs");
    assert_eq!(&agent_elf[0..4], b"\x7fELF");
    assert_eq!(agent_elf[4], 2, "ELFCLASS64");
    assert_eq!(agent_elf[5], 1, "ELFDATA2LSB");
    assert_eq!(
        u16::from_le_bytes([agent_elf[18], agent_elf[19]]),
        243,
        "EM_RISCV (243)"
    );
    assert_eq!(admit_elf64_riscv(&agent_elf), ElfAdmission::ElfAdmitted);

    let mvmbox_elf = vm
        .read_guest_file("/bin/mvmbox")
        .expect("/bin/mvmbox exists in initramfs");
    assert_eq!(admit_elf64_riscv(&mvmbox_elf), ElfAdmission::ElfAdmitted);

    let handshake = vm
        .exec_command("/sbin/mvm-agent --handshake", "/workspace")
        .expect("mvm-agent handshake executes on RV64IMAC CPU");
    assert_eq!(handshake.exit_code, 0, "stderr: {}", handshake.stderr);
    assert!(
        handshake.stdout.contains("MVM_AGENT_ONLINE"),
        "unexpected agent stdout: {}",
        handshake.stdout
    );
    assert!(
        !handshake.vsock_frames.is_empty(),
        "mvm-agent must emit at least one AF_VSOCK SpiralCore frame to CID 2"
    );
    let hdr = &handshake.vsock_frames[0].header;
    assert_eq!(hdr.magic, SPIRALCORE_MAGIC);
    assert_eq!(hdr.src_cid, 3);
    assert_eq!(hdr.dst_cid, 2);
    assert_eq!(hdr.vsock_port, 1024);
    assert!(hdr.cl06_spin < 64);
    assert!(hdr.e8_orbit < 16);

    let uname_out = vm
        .exec_command("uname -a", "/workspace")
        .expect("uname executes via /bin/mvmbox RV64 ELF");
    assert_eq!(uname_out.exit_code, 0);
    assert!(
        uname_out.stdout.contains("Linux") && uname_out.stdout.contains("riscv64"),
        "uname output must come from RV64 Linux syscall: {}",
        uname_out.stdout
    );

    let pipe_out = vm
        .exec_command("echo -n uor-mvm-riscv64 | sha256sum", "/workspace")
        .expect("pipeline executes via clone+pipe2+dup3+execve+wait4");
    assert_eq!(pipe_out.exit_code, 0);
    // SHA-256("uor-mvm-riscv64") = 9e86504b8878f8211398739e2d2553a1d78a6a4d0ef03a792eb9714998f88b86
    let expected_sha = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(b"uor-mvm-riscv64");
        let digest = h.finalize();
        let mut s = String::with_capacity(64);
        for b in digest {
            use std::fmt::Write;
            let _ = write!(&mut s, "{b:02x}");
        }
        s
    };
    assert!(
        pipe_out.stdout.contains(&expected_sha),
        "expected SHA-256 {} in pipeline output, got: {}",
        expected_sha,
        pipe_out.stdout
    );
}

#[test]
fn test_mvm_c_compiler_emits_and_executes_real_rv64_elf64_r4_37() {
    let _ = studio_dispatch("1\tstudio\treset\t");
    let c_src = r#"
#include <stdio.h>

long fib(long n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

long collatz_steps(long x) {
    long steps = 0;
    while (x > 1) {
        if ((x % 2) == 0) {
            x = x / 2;
        } else {
            x = 3 * x + 1;
        }
        steps = steps + 1;
    }
    return steps;
}

int main(void) {
    long f12 = fib(12);
    long c27 = collatz_steps(27);
    printf("FIB12=%ld COLLATZ27=%ld SUM=%ld\n", f12, c27, f12 + c27);
    return 0;
}
"#;
    let write_res: serde_json::Value = serde_json::from_str(&studio_dispatch(&format!(
        "1\tstudio\tworkspace_write\tsrc/bench.c\n{c_src}"
    )))
    .expect("workspace_write json");
    assert_eq!(write_res["status"], "ok");

    let exec_res: serde_json::Value = serde_json::from_str(&studio_dispatch(
        r#"1	studio	mvm_exec	{"cmd":"cc /workspace/src/bench.c -o /workspace/bench && /workspace/bench","cwd":"/workspace"}"#,
    ))
    .expect("mvm_exec json");
    assert_eq!(exec_res["status"], "ok", "res: {exec_res}");
    assert_eq!(exec_res["exit_code"], 0, "res: {exec_res}");
    let stdout = exec_res["mvm_stdout"].as_str().unwrap_or("");
    assert!(
        stdout.contains("FIB12=144 COLLATZ27=111 SUM=255"),
        "expected exact compiled RV64 execution output, got: {stdout}"
    );
    assert_eq!(exec_res["compiled_elf_machine"], 243);
}

#[test]
fn test_mvm_zero_nic_af_inet_refusal_and_spiralcore_vsock_secrets_r4_37() {
    // 1. Formal & runtime Zero-NIC checks (R5 negative coverage)
    assert_eq!(
        admit_guest_socket_domain(40),
        SocketAdmission::AdmittedVsock
    );
    assert_eq!(
        admit_guest_socket_domain(2),
        SocketAdmission::RefusedNicDisabled
    );
    assert_eq!(
        admit_guest_socket_domain(10),
        SocketAdmission::RefusedNicDisabled
    );
    assert_eq!(
        admit_guest_socket_domain(17),
        SocketAdmission::RefusedNicDisabled
    );
    assert_eq!(
        admit_guest_socket_domain(999),
        SocketAdmission::RefusedUnsupportedDomain
    );

    let mut vm = MvmSandbox::boot_default().expect("boot");
    let inet_probe = vm
        .exec_command("/bin/vsock_probe --try-inet", "/workspace")
        .expect("vsock_probe executes");
    assert_eq!(inet_probe.exit_code, 0);
    assert!(
        inet_probe.stdout.contains("AF_INET_DENIED=-97"),
        "guest AF_INET socket syscall must return -EAFNOSUPPORT (-97): {}",
        inet_probe.stdout
    );

    // 2. Non-RISC-V and corrupt ELF negative tests
    let mut fake_x86_elf = vm.read_guest_file("/sbin/mvm-agent").expect("read agent");
    fake_x86_elf[18] = 62; // EM_X86_64
    fake_x86_elf[19] = 0;
    assert_eq!(
        admit_elf64_riscv(&fake_x86_elf),
        ElfAdmission::RefusedWrongMachine
    );
    assert_eq!(admit_elf64_riscv(b"NOPE"), ElfAdmission::RefusedBadMagic);

    // 3. SpiralCore 16-byte frame negative tests
    let bad_self_loop = SpiralCoreHeader {
        magic: SPIRALCORE_MAGIC,
        src_cid: 3,
        dst_cid: 3,
        vsock_port: 1024,
        cl06_spin: 0,
        e8_orbit: 0,
        payload_len: 16,
    };
    assert_eq!(
        admit_spiralcore_header(&bad_self_loop, 65536),
        FrameVerdict::RefusedInvalidCid
    );

    let bad_spin = SpiralCoreHeader {
        cl06_spin: 64,
        dst_cid: 2,
        ..bad_self_loop
    };
    assert_eq!(
        admit_spiralcore_header(&bad_spin, 65536),
        FrameVerdict::RefusedSpinOutOfRange
    );

    // 4. Secret placeholder injection & zero guest-RAM leak guarantee
    vm.register_host_secret(
        "API_TOKEN",
        "sk-live-super-secret-998877",
        &["api.github.com"],
    );
    let denied_egress = vm.proxy_vsock_egress(
        "evil.attacker.invalid",
        "Authorization: Bearer {{MVM_SECRET:API_TOKEN}}",
    );
    assert_eq!(
        denied_egress.verdict,
        EgressVerdict::RefusedHostNotAllowlisted
    );
    assert!(denied_egress.outbound_payload.is_none());

    let allowed_egress = vm.proxy_vsock_egress(
        "api.github.com",
        "Authorization: Bearer {{MVM_SECRET:API_TOKEN}}",
    );
    assert_eq!(allowed_egress.verdict, EgressVerdict::ProxyAdmitted);
    assert_eq!(
        allowed_egress.outbound_payload.as_deref(),
        Some("Authorization: Bearer sk-live-super-secret-998877")
    );
    assert!(
        !vm.guest_ram_contains(b"sk-live-super-secret-998877"),
        "raw secret must NEVER reside in guest RAM pages"
    );

    // 5. East-West CID-to-CID SpiralCore switch routing (CID 3 -> CID 4)
    let mut verifier_vm = MvmSandbox::boot_with_cid(4).expect("boot CID 4");
    let frame_bytes = vm
        .build_vsock_packet(4, 4096, 19, 7, b"VERIFY_DIFF_SHA256:cafe1234")
        .expect("valid SpiralCore frame");
    verifier_vm
        .deliver_vsock_packet(&frame_bytes)
        .expect("CID 4 receives frame");
    let received = verifier_vm
        .pop_vsock_inbox(4096)
        .expect("port 4096 message");
    assert_eq!(received.header.src_cid, 3);
    assert_eq!(received.header.dst_cid, 4);
    assert_eq!(received.header.cl06_spin, 19);
    assert_eq!(received.header.e8_orbit, 7);
    assert_eq!(received.payload, b"VERIFY_DIFF_SHA256:cafe1234");
}

#[test]
fn test_mvm_cow_block_snapshot_and_rollback_r4_37() {
    let mut vm = MvmSandbox::boot_default().expect("boot");
    vm.write_guest_file("/workspace/state.txt", b"baseline-v1")
        .expect("write baseline");
    let snap = vm.snapshot_cow();
    let baseline_digest = snap.state_sha256.clone();

    vm.write_guest_file("/workspace/state.txt", b"mutated-v2-dirty-sector")
        .expect("write mutated");
    assert!(vm.dirty_cow_sector_count() >= 1);
    assert_eq!(
        vm.read_guest_file("/workspace/state.txt").as_deref(),
        Some(&b"mutated-v2-dirty-sector"[..])
    );

    vm.restore_cow(&snap);
    assert_eq!(vm.cow_state_sha256(), baseline_digest);
    assert_eq!(
        vm.read_guest_file("/workspace/state.txt").as_deref(),
        Some(&b"baseline-v1"[..])
    );
}
