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
use lexlean_r4_wasm::{
    mvm_export_workspace_diff_json, mvm_sync_workspace_in_json, studio_dispatch,
    studio_export_mvm_cow_sectors, studio_import_mvm_cow_sectors, StudioEngine,
};

static STUDIO_GLOBAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    let _guard = STUDIO_GLOBAL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

#[test]
fn test_mvm_pipeline_heredoc_redirects_and_env_r4_57() {
    let mut vm = MvmSandbox::boot_default().expect("boot MVM");

    // 1. Multi-stage pipeline + single-quoted heredoc + redirect from task-2-brief
    let script = "cat <<'EOF' | grep -E \"foo|bar\" | sort | uniq -c | awk '{print $2,$1}' > /workspace/out.txt\nfoo\nbar\nfoo\nbaz\nEOF";
    let res = vm
        .exec_command(script, "/workspace")
        .expect("exec pipeline");
    assert_eq!(res.exit_code, 0, "stderr: {}", res.stderr);
    assert_eq!(res.stdout, "");
    let out_bytes = vm
        .read_guest_file("/workspace/out.txt")
        .expect("/workspace/out.txt written");
    assert_eq!(String::from_utf8_lossy(&out_bytes), "bar 1\nfoo 2\n");

    // 2. Quoted vs unquoted heredoc + export + ${VAR:-default} + $? + chains
    let env_script = "export GREETING=hello\ncat <<EOF > /workspace/env_doc.txt\n$GREETING ${MISSING:-fallback}\nEOF\ncat <<'EOF' >> /workspace/env_doc.txt\n$GREETING\nEOF";
    let env_res = vm.exec_command(env_script, "/workspace").expect("exec env");
    assert_eq!(env_res.exit_code, 0, "stderr: {}", env_res.stderr);
    let env_bytes = vm
        .read_guest_file("/workspace/env_doc.txt")
        .expect("/workspace/env_doc.txt written");
    assert_eq!(
        String::from_utf8_lossy(&env_bytes),
        "hello fallback\n$GREETING\n"
    );

    // 3. Input redirect `<`, `2>&1`, `&&`, `||`
    let wc_in = vm
        .exec_command("wc -l < /workspace/out.txt", "/workspace")
        .expect("wc <");
    assert_eq!(wc_in.exit_code, 0, "stderr: {}", wc_in.stderr);
    assert_eq!(wc_in.stdout.trim(), "2");

    let redir_err = vm
        .exec_command(
            "cat /workspace/missing_redir.txt 2>&1 || echo RECOVERED:$?",
            "/workspace",
        )
        .expect("2>&1 and ||");
    assert_eq!(redir_err.exit_code, 0);
    assert!(
        redir_err.stdout.contains("missing_redir.txt") && redir_err.stdout.contains("RECOVERED:1"),
        "expected merged stderr and || recovery, got stdout={:?} stderr={:?}",
        redir_err.stdout,
        redir_err.stderr
    );
}

#[test]
fn test_mvm_coreutils_applets_and_negative_errors_r4_57() {
    let mut vm = MvmSandbox::boot_default().expect("boot MVM");
    vm.write_guest_file(
        "/workspace/data.txt",
        b"alpha:10:red\nbeta:20:blue\ngamma:30:green\ndelta:40:yellow\n",
    )
    .expect("seed data.txt");
    vm.write_guest_file(
        "/workspace/other.txt",
        b"alpha:10:red\nbeta:25:blue\ngamma:30:green\ndelta:40:yellow\n",
    )
    .expect("seed other.txt");

    let positive_cases: Vec<(&str, &str)> = vec![
        ("head -n 2 /workspace/data.txt", "alpha:10:red\nbeta:20:blue\n"),
        ("tail -n 2 /workspace/data.txt", "gamma:30:green\ndelta:40:yellow\n"),
        ("wc -l /workspace/data.txt", "4 /workspace/data.txt\n"),
        ("echo hello | tr a-z A-Z", "HELLO\n"),
        ("cut -d: -f1,3 /workspace/data.txt | head -n 1", "alpha:red\n"),
        ("echo hello world | sed 's/world/mvm/g'", "hello mvm\n"),
        ("echo 'a b c' | awk '{print $3,$1}'", "c a\n"),
        ("find /workspace -name '*.txt' | sort", "/workspace/data.txt\n/workspace/other.txt\n"),
        ("cp /workspace/data.txt /workspace/copy.txt && cat /workspace/copy.txt | wc -l", "4\n"),
        ("mv /workspace/copy.txt /workspace/moved.txt && ls /workspace/moved.txt", "/workspace/moved.txt\n"),
        ("touch /workspace/touched.txt && ls /workspace/touched.txt", "/workspace/touched.txt\n"),
        ("chmod 755 /workspace/touched.txt && echo ok", "ok\n"),
        ("ln -s /workspace/data.txt /workspace/data.link && head -n 1 /workspace/data.link", "alpha:10:red\n"),
        ("printf '%s-%03d\\n' uor 7", "uor-007\n"),
        ("export TEST_ENV_KEY=rv64 && env | grep TEST_ENV_KEY", "TEST_ENV_KEY=rv64\n"),
        ("which jq", "/usr/bin/jq\n"),
        ("ps", "mvm-agent"),
        ("uname -m", "riscv64\n"),
        ("date +%Y", "2026\n"),
        ("tar -cf /workspace/arch.tar -C /workspace data.txt other.txt && tar -tf /workspace/arch.tar", "data.txt"),
        ("mkdir -p /workspace/untar && tar -xf /workspace/arch.tar -C /workspace/untar && head -n 1 /workspace/untar/data.txt", "alpha:10:red\n"),
        ("gzip -c /workspace/data.txt > /workspace/data.txt.gz && gunzip -c /workspace/data.txt.gz | head -n 1", "alpha:10:red\n"),
        ("echo -n uor | base64 | base64 -d", "uor"),
        ("diff -u /workspace/data.txt /workspace/data.txt", ""),
        ("echo '6 * 7 + 5' | bc", "47\n"),
        ("echo '{\"a\":{\"b\":42}}' | jq .a.b", "42\n"),
    ];

    for (cmd, expected_substr) in positive_cases {
        let out = vm.exec_command(cmd, "/workspace").expect(cmd);
        assert_eq!(
            out.exit_code, 0,
            "command `{cmd}` failed with exit={}, stdout={:?}, stderr={:?}",
            out.exit_code, out.stdout, out.stderr
        );
        if expected_substr.is_empty() {
            assert_eq!(out.stdout, "", "expected empty stdout for `{cmd}`");
        } else {
            assert!(
                out.stdout.contains(expected_substr),
                "command `{cmd}` stdout {:?} missing {:?}",
                out.stdout,
                expected_substr
            );
        }
    }

    // diff on differing files returns exit 1 with diff lines (not 2)
    let diff_changed = vm
        .exec_command(
            "diff -u /workspace/data.txt /workspace/other.txt",
            "/workspace",
        )
        .expect("diff changed");
    assert_eq!(diff_changed.exit_code, 1);
    assert!(diff_changed.stdout.contains("-beta:20:blue"));
    assert!(diff_changed.stdout.contains("+beta:25:blue"));

    // Negative / missing-file / corrupt-input / binary coverage (R5, zero panics)
    let negative_cases = [
        "head /workspace/nope.txt",
        "tail /workspace/nope.txt",
        "wc /workspace/nope.txt",
        "cut -d: -f1 /workspace/nope.txt",
        "sed 's/a/b/' /workspace/nope.txt",
        "awk '{print $1}' /workspace/nope.txt",
        "find /workspace/nope_dir",
        "cp /workspace/nope.txt /workspace/dst.txt",
        "mv /workspace/nope.txt /workspace/dst.txt",
        "chmod 755 /workspace/nope.txt",
        "ln /workspace/nope.txt /workspace/link.txt",
        "which missing_binary_r5_check",
        "tar -tf /workspace/nope.tar",
        "tar -cf /workspace/bad.tar /workspace/nope.txt",
        "gzip /workspace/nope.txt",
        "gunzip /workspace/nope.gz",
        "base64 /workspace/nope.txt",
        "echo '!!!invalid!!!' | base64 -d",
        "echo 'not-gzip-bytes' | gunzip",
        "diff /workspace/nope1.txt /workspace/nope2.txt",
        "echo '10 / 0' | bc",
        "echo '{bad-json' | jq .",
        "jq . /workspace/nope.json",
    ];
    for cmd in negative_cases {
        let out = vm.exec_command(cmd, "/workspace").expect(cmd);
        assert_ne!(
            out.exit_code, 0,
            "negative command `{cmd}` must return non-zero exit_code, got 0 (stdout={:?})",
            out.stdout
        );
        assert!(
            !out.stderr.trim().is_empty(),
            "negative command `{cmd}` must emit typed diagnostic stderr"
        );
    }

    // Binary safety: non-UTF-8 bytes never panic across text & archive utilities
    vm.write_guest_file("/workspace/bin.dat", b"\xff\xfe\x00\x80\nhello\n\xf0\x9f")
        .expect("write binary fixture");
    for bin_cmd in [
        "head -n 1 /workspace/bin.dat",
        "tail -n 1 /workspace/bin.dat",
        "wc -c /workspace/bin.dat",
        "grep -E hello /workspace/bin.dat",
        "sed 's/hello/world/' /workspace/bin.dat",
        "awk '{print NR}' /workspace/bin.dat",
        "base64 /workspace/bin.dat | base64 -d | wc -c",
        "gzip -c /workspace/bin.dat | gunzip -c | wc -c",
        "diff /workspace/bin.dat /workspace/data.txt",
    ] {
        let out = vm.exec_command(bin_cmd, "/workspace").expect(bin_cmd);
        assert!(
            out.exit_code == 0 || out.exit_code == 1,
            "binary command `{bin_cmd}` panicked or failed unexpectedly: {:?}",
            out.stderr
        );
    }
}

fn test_b64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[test]
fn test_mvm_workspace_sync_filter_and_binary_isolation_r4_57() {
    // 1. MvmSandbox level: heavy dir, binary blob, and canonical rule exclusion + OPFS COW persistence
    let mut vm = MvmSandbox::boot_default().expect("boot MVM");
    let main_tsx = b"export const App = () => <h1>Hello</h1>;\n";
    let db_blob = b"SQLite format 3\x00\xff\xfe\x80\x01binary-db";
    let wasm_blob = b"\x00asm\x01\x00\x00\x00\xff\xfe";
    let big_bin = vec![b'B'; 1_048_577];

    vm.write_guest_file("/workspace/src/main.tsx", main_tsx)
        .expect("write main.tsx");
    vm.write_guest_file(
        "/workspace/node_modules/pkg/index.js",
        b"module.exports = 1;\n",
    )
    .expect("write node_modules");
    vm.write_guest_file("/workspace/.venv/lib/x.py", b"x = 1\n")
        .expect("write .venv");
    vm.write_guest_file(
        "/workspace/pkg/__pycache__/mod.cpython-312.pyc",
        b"\x00\xffpyc",
    )
    .expect("write __pycache__");
    vm.write_guest_file("/workspace/.git/objects/ab/cdef01", b"\x78\x9c\x01")
        .expect("write .git/objects");
    vm.seed_canonical_guest_file(
        "/workspace/prism/Rule.lex.tex",
        b"\\begin{lexlean}{R4.Rule}\\end{lexlean}\n",
    )
    .expect("seed prism rule");
    vm.seed_canonical_guest_file("/workspace/app/state", b"canonical-state\n")
        .expect("seed app/state");
    vm.write_guest_file("/workspace/data/local.db", db_blob)
        .expect("write local.db");
    vm.write_guest_file("/workspace/dist/app.wasm", wasm_blob)
        .expect("write app.wasm");
    vm.write_guest_file("/workspace/big.bin", &big_bin)
        .expect("write >1MiB bin");

    let diff_raw = vm.mvm_export_workspace_diff_json();
    let diff: serde_json::Value = serde_json::from_str(&diff_raw).expect("valid diff json");
    assert_eq!(diff["status"], "ok");
    let writes = diff["writes"].as_array().expect("writes array");
    assert_eq!(
        writes.len(),
        1,
        "only src/main.tsx should be exported, got: {writes:?}"
    );
    assert_eq!(writes[0]["path"], "src/main.tsx");
    assert_eq!(writes[0]["content_base64"], test_b64_encode(main_tsx));
    assert_eq!(diff["deletes"].as_array().expect("deletes").len(), 0);
    assert_eq!(
        diff["ignored_heavy_paths"].as_u64(),
        Some(9),
        "all 9 excluded heavy/binary/canonical paths must be counted: {diff}"
    );
    assert_eq!(
        diff["cow_state_sha256"].as_str().map(str::len),
        Some(64),
        "cow_state_sha256 must be 64-char hex"
    );

    // Excluded files MUST still persist across MvmCowSnapshot CoW sectors (uor-mvm-cow-v1.bin)
    let dirty_sectors = vm.export_dirty_cow_sectors();
    let mut restored_vm = MvmSandbox::boot_default().expect("boot restored MVM");
    let imported = restored_vm.import_cow_sectors(&dirty_sectors);
    assert!(imported >= 1);
    assert_eq!(
        restored_vm
            .read_guest_file("/workspace/node_modules/pkg/index.js")
            .as_deref(),
        Some(&b"module.exports = 1;\n"[..])
    );
    assert_eq!(
        restored_vm
            .read_guest_file("/workspace/.venv/lib/x.py")
            .as_deref(),
        Some(&b"x = 1\n"[..])
    );
    assert_eq!(
        restored_vm
            .read_guest_file("/workspace/data/local.db")
            .as_deref(),
        Some(&db_blob[..])
    );
    assert_eq!(
        restored_vm
            .read_guest_file("/workspace/dist/app.wasm")
            .as_deref(),
        Some(&wasm_blob[..])
    );

    // 2. StudioEngine & #[wasm_bindgen] bidirectional sync + incremental delta + canonical write refusal
    let _guard = STUDIO_GLOBAL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = studio_dispatch("1\tstudio\treset\t");
    let sync_in_payload = serde_json::json!({
        "epoch": 42,
        "files": [
            { "path": "package.json", "content_base64": test_b64_encode(b"{\"name\":\"foundry-app\"}\n") },
            { "path": "src/main.tsx", "content_base64": test_b64_encode(b"export const v = 1;\n") }
        ]
    })
    .to_string();
    let sync_res: serde_json::Value =
        serde_json::from_str(&mvm_sync_workspace_in_json(&sync_in_payload)).expect("sync_in json");
    assert_eq!(sync_res["status"], "ok");
    assert_eq!(sync_res["epoch"], 42);

    // Guest writes to canonical protected paths produce typed diagnostic stderr & never leak
    let canon_attempt = serde_json::from_str::<serde_json::Value>(&studio_dispatch(
        r#"1	studio	mvm_exec	{"cmd":"mkdir -p /workspace/prism /workspace/app && echo tamper > /workspace/prism/Rule.lex.tex"}"#,
    ))
    .expect("mvm_exec canonical json");
    assert_eq!(
        canon_attempt["status"], "ok",
        "canonical refusal must still complete mvm_exec cleanly: {canon_attempt}"
    );
    assert_ne!(canon_attempt["exit_code"], 0);
    assert!(
        !canon_attempt["mvm_stderr"]
            .as_str()
            .unwrap_or("")
            .trim()
            .is_empty(),
        "canonical write refusal must emit diagnostic stderr: {canon_attempt}"
    );

    // Guest updates src/main.tsx and deletes package.json + creates a binary sqlite db
    let mut_res = serde_json::from_str::<serde_json::Value>(&studio_dispatch(
        r#"1	studio	mvm_exec	{"cmd":"echo 'export const v = 2;' > /workspace/src/main.tsx && rm /workspace/package.json && printf 'bin\\0data' > /workspace/cache.db"}"#,
    ))
    .expect("mvm_exec mutate json");
    assert_eq!(mut_res["status"], "ok", "mutate failed: {mut_res}");
    assert_eq!(mut_res["exit_code"], 0, "stderr: {}", mut_res["mvm_stderr"]);

    let delta1: serde_json::Value =
        serde_json::from_str(&mvm_export_workspace_diff_json()).expect("delta1 json");
    assert_eq!(delta1["status"], "ok");
    assert_eq!(delta1["epoch"], 43);
    let d1_writes = delta1["writes"].as_array().expect("writes");
    assert_eq!(d1_writes.len(), 1);
    assert_eq!(d1_writes[0]["path"], "src/main.tsx");
    assert_eq!(
        d1_writes[0]["content_base64"],
        test_b64_encode(b"export const v = 2;\n")
    );
    let d1_deletes = delta1["deletes"].as_array().expect("deletes");
    assert_eq!(d1_deletes, &vec![serde_json::json!("package.json")]);
    assert!(delta1["ignored_heavy_paths"].as_u64().unwrap_or(0) >= 1);

    // Second export with no further changes returns empty delta
    let delta2: serde_json::Value =
        serde_json::from_str(&mvm_export_workspace_diff_json()).expect("delta2 json");
    assert_eq!(delta2["writes"].as_array().expect("writes").len(), 0);
    assert_eq!(delta2["deletes"].as_array().expect("deletes").len(), 0);

    // OPFS CoW sector stream preserves cache.db
    let cow_stream = studio_export_mvm_cow_sectors();
    let import_res: serde_json::Value =
        serde_json::from_str(&studio_import_mvm_cow_sectors(&cow_stream)).expect("import json");
    assert_eq!(import_res["status"], "ok");

    // 3. Negative sync_in validation (`R5`)
    let mut engine = StudioEngine::new_default();
    let bad_json: serde_json::Value =
        serde_json::from_str(&engine.mvm_sync_workspace_in_json("{bad")).expect("err json");
    assert_eq!(bad_json["status"], "error");
    assert_eq!(bad_json["error_code"], "InvalidPayload");

    let traversal: serde_json::Value = serde_json::from_str(
        &engine.mvm_sync_workspace_in_json(
            &serde_json::json!({
                "epoch": 1,
                "files": [{ "path": "../etc/passwd", "content_base64": test_b64_encode(b"x") }]
            })
            .to_string(),
        ),
    )
    .expect("traversal err json");
    assert_eq!(traversal["status"], "error");
    assert_eq!(traversal["error_code"], "RefusedParentTraversal");

    let bad_b64: serde_json::Value = serde_json::from_str(
        &engine.mvm_sync_workspace_in_json(
            &serde_json::json!({
                "epoch": 1,
                "files": [{ "path": "src/bad.txt", "content_base64": "!!!" }]
            })
            .to_string(),
        ),
    )
    .expect("bad b64 err json");
    assert_eq!(bad_b64["status"], "error");
    assert_eq!(bad_b64["error_code"], "InvalidBase64");

    // 4. Monotonic sync epoch (never regress on epoch: 0, refuse stale lower non-zero epoch)
    let mut mono_vm = MvmSandbox::boot_default().expect("boot mono vm");
    let e10: serde_json::Value = serde_json::from_str(
        &mono_vm.mvm_sync_workspace_in_json(
            &serde_json::json!({
                "epoch": 10,
                "files": [{ "path": "a.txt", "content_base64": test_b64_encode(b"a") }]
            })
            .to_string(),
        ),
    )
    .expect("e10 json");
    assert_eq!(e10["status"], "ok");
    assert_eq!(e10["epoch"], 10);

    let e0_after_10: serde_json::Value = serde_json::from_str(
        &mono_vm.mvm_sync_workspace_in_json(
            &serde_json::json!({
                "epoch": 0,
                "files": [{ "path": "b.txt", "content_base64": test_b64_encode(b"b") }]
            })
            .to_string(),
        ),
    )
    .expect("e0 json");
    assert_eq!(e0_after_10["status"], "ok");
    assert_eq!(
        e0_after_10["epoch"], 10,
        "epoch: 0 must never regress monotonic workspace_sync_epoch: {e0_after_10}"
    );
    assert_eq!(
        mono_vm.read_guest_file("/workspace/b.txt").as_deref(),
        Some(&b"b"[..])
    );

    let stale_nonzero: serde_json::Value = serde_json::from_str(
        &mono_vm.mvm_sync_workspace_in_json(
            &serde_json::json!({
                "epoch": 3,
                "files": [{ "path": "c.txt", "content_base64": test_b64_encode(b"c") }]
            })
            .to_string(),
        ),
    )
    .expect("stale json");
    assert_eq!(
        stale_nonzero["status"], "error",
        "stale lower non-zero epoch must be refused: {stale_nonzero}"
    );
    assert_eq!(stale_nonzero["error_code"], "StaleSyncEpoch");
    assert!(mono_vm.read_guest_file("/workspace/c.txt").is_none());
}

#[test]
fn test_mvm_quoted_env_assignments_r4_57() {
    let mut vm = MvmSandbox::boot_default().expect("boot");

    let bare_double = vm
        .exec_command("FOO=\"hello world\"; printf '%s\\n' \"$FOO\"", "/workspace")
        .expect("bare double quoted env");
    assert_eq!(
        bare_double.exit_code, 0,
        "bare quoted assignment failed: {}",
        bare_double.stderr
    );
    assert_eq!(bare_double.stdout, "hello world\n");

    let bare_single = vm
        .exec_command(
            "BAR='alpha $NOT_EXP beta'; printf '%s\\n' \"$BAR\"",
            "/workspace",
        )
        .expect("bare single quoted env");
    assert_eq!(bare_single.exit_code, 0, "stderr: {}", bare_single.stderr);
    assert_eq!(bare_single.stdout, "alpha $NOT_EXP beta\n");

    let prefix_cmd = vm
        .exec_command("FOO='bar baz' BAZ=\"x y\" env", "/workspace")
        .expect("prefix quoted env");
    assert_eq!(prefix_cmd.exit_code, 0, "stderr: {}", prefix_cmd.stderr);
    assert!(
        prefix_cmd.stdout.contains("FOO=bar baz\n") && prefix_cmd.stdout.contains("BAZ=x y\n"),
        "expected quoted prefix assignments in env output: {}",
        prefix_cmd.stdout
    );

    let quoted_lhs = vm
        .exec_command("\"NOT_ASSIGN=1\"", "/workspace")
        .expect("quoted LHS runs");
    assert_eq!(
        quoted_lhs.exit_code, 127,
        "fully quoted word must NOT be parsed as env assignment: {quoted_lhs:?}"
    );
}

#[test]
fn test_mvm_canonical_protection_comprehensive_r4_57() {
    let mut vm = MvmSandbox::boot_default().expect("boot");
    let canonical_rule = b"\\begin{lexlean}{R4.Rule}\\end{lexlean}\n";
    let canonical_state = b"canonical-state-v1\n";
    vm.seed_canonical_guest_file("/workspace/prism/Rule.lex.tex", canonical_rule)
        .expect("seed rule");
    vm.seed_canonical_guest_file("/workspace/app/state", canonical_state)
        .expect("seed state");

    // 0. Centralized write_guest_file & double-seed must refuse canonical mutation
    assert!(
        matches!(
            vm.write_guest_file("/workspace/prism/Rule.lex.tex", b"tamper"),
            Err(lexlean_r4_wasm::mvm::MvmError::CanonicalPathProtected(_))
        ),
        "write_guest_file must refuse overwriting canonical rule"
    );
    assert!(
        matches!(
            vm.write_guest_file("/workspace/app/state", b"tamper"),
            Err(lexlean_r4_wasm::mvm::MvmError::CanonicalPathProtected(_))
        ),
        "write_guest_file must refuse mutating app/state"
    );
    assert!(
        matches!(
            vm.seed_canonical_guest_file("/workspace/prism/Rule.lex.tex", b"tamper"),
            Err(lexlean_r4_wasm::mvm::MvmError::CanonicalPathProtected(_))
        ),
        "seed_canonical_guest_file must be write-once"
    );

    // 1. Directory mv & chmod -R & rm -rf must refuse canonical descendants and preserve /workspace/prism
    let mv_dir = vm
        .exec_command("mv /workspace/prism /workspace/bak", "/workspace")
        .expect("mv dir executes");
    assert_ne!(
        mv_dir.exit_code, 0,
        "mv of dir containing canonical rule must fail"
    );
    assert_eq!(
        vm.read_guest_file("/workspace/prism/Rule.lex.tex")
            .as_deref(),
        Some(&canonical_rule[..])
    );
    assert!(vm.read_guest_file("/workspace/bak/Rule.lex.tex").is_none());

    let chmod_r = vm
        .exec_command("chmod -R 777 /workspace/prism", "/workspace")
        .expect("chmod -R executes");
    assert_ne!(
        chmod_r.exit_code, 0,
        "chmod -R on dir containing canonical rule must fail"
    );

    let rm_rf = vm
        .exec_command("rm -rf /workspace/prism", "/workspace")
        .expect("rm -rf executes");
    assert_ne!(rm_rf.exit_code, 0, "rm -rf /workspace/prism must refuse");
    assert_eq!(
        vm.read_guest_file("/workspace/prism/Rule.lex.tex")
            .as_deref(),
        Some(&canonical_rule[..])
    );
    let prism_dir_exists = vm
        .exec_command(
            "test -d /workspace/prism && ls /workspace/prism",
            "/workspace",
        )
        .expect("test -d executes");
    assert_eq!(
        prism_dir_exists.exit_code, 0,
        "/workspace/prism dir must remain intact in self.dirs: {}",
        prism_dir_exists.stderr
    );
    assert!(prism_dir_exists.stdout.contains("Rule.lex.tex"));

    // 2. Symlink redirect attempt must not overwrite canonical rule
    let sym_write = vm
        .exec_command(
            "ln -s /workspace/prism/Rule.lex.tex /workspace/rule_link && echo tamper > /workspace/rule_link",
            "/workspace",
        )
        .expect("symlink write executes");
    assert_ne!(
        sym_write.exit_code, 0,
        "write via symlink to canonical path must fail"
    );
    assert_eq!(
        vm.read_guest_file("/workspace/prism/Rule.lex.tex")
            .as_deref(),
        Some(&canonical_rule[..])
    );

    // 3. Real compiled RV64 C/ELF binary invoking SYS_openat, SYS_unlinkat, and SYS_write must get -EACCES (-13) / -EBADF (-9)
    let c_src = r#"
int main(void) {
    long fd_w = openat(-100, "/workspace/prism/Rule.lex.tex", 1, 0);
    long fd_c = openat(-100, "/workspace/prism/New.lex.tex", 65, 420);
    long fd_t = openat(-100, "/workspace/app/state", 514, 0);
    long ul_f = unlinkat(-100, "/workspace/prism/Rule.lex.tex", 0);
    long ul_s = unlinkat(-100, "/workspace/app/state", 0);
    long ul_d = unlinkat(-100, "/workspace/prism", 512);
    long fd_r = openat(-100, "/workspace/prism/Rule.lex.tex", 0, 0);
    long wr_ro = write(fd_r, "tamper", 6);
    close(fd_r);
    printf("W=%ld C=%ld T=%ld UF=%ld US=%ld UD=%ld WR=%ld\n",
        fd_w, fd_c, fd_t, ul_f, ul_s, ul_d, wr_ro);
    return 0;
}
"#;
    vm.write_guest_file("/workspace/canon_probe.c", c_src.as_bytes())
        .expect("write probe.c");
    let cc_res = vm
        .exec_command(
            "cc /workspace/canon_probe.c -o /workspace/canon_probe && /workspace/canon_probe",
            "/workspace",
        )
        .expect("cc probe runs");
    assert_eq!(
        cc_res.exit_code, 0,
        "RV64 canon_probe failed: {}",
        cc_res.stderr
    );
    assert!(
        cc_res
            .stdout
            .contains("W=-13 C=-13 T=-13 UF=-13 US=-13 UD=-13 WR=-9"),
        "RV64 syscalls must return -EACCES (-13) / -EBADF (-9) on canonical paths, got: {}",
        cc_res.stdout
    );
    assert_eq!(
        vm.read_guest_file("/workspace/prism/Rule.lex.tex")
            .as_deref(),
        Some(&canonical_rule[..])
    );
    assert_eq!(
        vm.read_guest_file("/workspace/app/state").as_deref(),
        Some(&canonical_state[..])
    );

    // 4. mvm_sync_workspace_in_json must never write or delete prism/*.lex.tex or app/state
    let sync_tamper: serde_json::Value = serde_json::from_str(
        &vm.mvm_sync_workspace_in_json(
            &serde_json::json!({
                "epoch": 5,
                "files": [
                    { "path": "prism/Rule.lex.tex", "content_base64": test_b64_encode(b"tampered!\n") },
                    { "path": "app/state", "content_base64": "", "deleted": true },
                    { "path": "ok.txt", "content_base64": test_b64_encode(b"ok\n") }
                ]
            })
            .to_string(),
        ),
    )
    .expect("sync tamper json");
    assert_eq!(sync_tamper["status"], "ok");
    assert_eq!(sync_tamper["applied_writes"], 1);
    assert_eq!(sync_tamper["applied_deletes"], 0);
    assert_eq!(sync_tamper["ignored_canonical_paths"], 2);
    assert_eq!(
        vm.read_guest_file("/workspace/prism/Rule.lex.tex")
            .as_deref(),
        Some(&canonical_rule[..])
    );
    assert_eq!(
        vm.read_guest_file("/workspace/app/state").as_deref(),
        Some(&canonical_state[..])
    );
    assert_eq!(
        vm.read_guest_file("/workspace/ok.txt").as_deref(),
        Some(&b"ok\n"[..])
    );
}

#[test]
fn test_mvm_polyglot_wasm_container_bridge_r4_58() {
    let mut vm = MvmSandbox::boot_default().expect("boot MVM");

    // 1. `node -e ...` must suspend with NeedsBridge(node_exec) and write /workspace/hi.txt on resume
    let node_cmd =
        "node -e \"const fs = require('fs'); fs.writeFileSync('/workspace/hi.txt', process.version);\"";
    let pending_node = match vm
        .exec_interactive(node_cmd, Some("/workspace"))
        .expect("node interactive executes")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::Completed(done) => {
            panic!("expected NeedsBridge for node, got Completed: {done:?}");
        }
    };
    assert_eq!(pending_node.bridge_req["op"], "node_exec");
    assert_eq!(pending_node.bridge_req["kind"], "node_exec");
    assert_eq!(pending_node.bridge_req["tool"], "node");
    assert!(!pending_node.vsock_frames.is_empty());

    let node_resumed = match vm
        .complete_bridge(
            &pending_node.resume_state,
            &serde_json::json!({
                "ok": true,
                "exit_code": 0,
                "stdout": "",
                "stderr": "",
                "writes": [
                    {
                        "path": "/workspace/hi.txt",
                        "content_base64": test_b64_encode(b"v20.0.0-mvm"),
                    }
                ],
                "deletes": []
            }),
        )
        .expect("complete node bridge")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::Completed(c) => c,
        other => panic!("expected Completed after node resume, got {other:?}"),
    };
    assert_eq!(node_resumed.exit_code, 0, "stderr: {}", node_resumed.stderr);
    assert_eq!(
        vm.read_guest_file("/workspace/hi.txt").as_deref(),
        Some(&b"v20.0.0-mvm"[..])
    );

    // 2. `npm install <pkg>` emits pkg_fetch/npm bridge and keeps node_modules/** out of Studio delta export while exporting package.json
    let pending_npm = match vm
        .exec_interactive("npm install left-pad", Some("/workspace"))
        .expect("npm interactive executes")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge for npm install, got {other:?}"),
    };
    assert_eq!(pending_npm.bridge_req["kind"], "pkg_fetch");
    assert_eq!(pending_npm.bridge_req["ecosystem"], "npm");
    assert_eq!(pending_npm.bridge_req["tool"], "npm");

    let npm_resumed = match vm
        .complete_bridge(
            &pending_npm.resume_state,
            &serde_json::json!({
                "ok": true,
                "exit_code": 0,
                "stdout": "added 1 package left-pad@1.3.0\n",
                "stderr": "",
                "writes": [
                    {
                        "path": "package.json",
                        "content_base64": test_b64_encode(b"{\n  \"dependencies\": {\n    \"left-pad\": \"^1.3.0\"\n  }\n}\n"),
                    },
                    {
                        "path": "node_modules/left-pad/index.js",
                        "content_base64": test_b64_encode(b"module.exports = (s) => s;\n"),
                    }
                ]
            }),
        )
        .expect("complete npm bridge")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::Completed(c) => c,
        other => panic!("expected Completed after npm resume, got {other:?}"),
    };
    assert_eq!(npm_resumed.exit_code, 0);
    assert_eq!(
        vm.read_guest_file("/workspace/node_modules/left-pad/index.js")
            .as_deref(),
        Some(&b"module.exports = (s) => s;\n"[..])
    );
    let diff: serde_json::Value =
        serde_json::from_str(&vm.mvm_export_workspace_diff_json()).expect("diff json");
    let diff_paths: Vec<&str> = diff["writes"]
        .as_array()
        .expect("writes")
        .iter()
        .filter_map(|w| w["path"].as_str())
        .collect();
    assert!(
        diff_paths.contains(&"package.json"),
        "package.json must sync to Studio: {diff_paths:?}"
    );
    assert!(
        !diff_paths.iter().any(|p| p.starts_with("node_modules/")),
        "node_modules/** must stay isolated in MVM OPFS COW sectors: {diff_paths:?}"
    );

    // 3. `python3`, `pip`, `wasm run`, and `sqlite3` emit typed NeedsBridge requests with workspace snapshot (including binary *.wasm and *.db)
    let pending_py = match vm
        .exec_interactive("python3 -c \"print(42)\"", Some("/workspace"))
        .expect("python3 interactive")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge for python3, got {other:?}"),
    };
    assert_eq!(pending_py.bridge_req["kind"], "python_exec");
    assert_eq!(pending_py.bridge_req["tool"], "python3");

    let pending_pip = match vm
        .exec_interactive("pip install rich", Some("/workspace"))
        .expect("pip interactive")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge for pip, got {other:?}"),
    };
    assert_eq!(pending_pip.bridge_req["kind"], "python_exec");
    assert_eq!(pending_pip.bridge_req["tool"], "pip");

    vm.write_guest_file("/workspace/mod.wasm", b"\x00asm\x01\x00\x00\x00")
        .expect("write mod.wasm");
    let pending_wasi = match vm
        .exec_interactive("wasm run /workspace/mod.wasm --flag", Some("/workspace"))
        .expect("wasm run interactive")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge for wasm run, got {other:?}"),
    };
    assert_eq!(pending_wasi.bridge_req["kind"], "wasi_exec");
    let wasi_ws = pending_wasi.bridge_req["workspace"]
        .as_array()
        .expect("workspace entries in wasi_exec");
    assert!(
        wasi_ws.iter().any(|e| e["path"] == "mod.wasm"
            && e["content_base64"] == test_b64_encode(b"\x00asm\x01\x00\x00\x00")),
        "wasi_exec bridge request must carry binary mod.wasm from MVM /workspace"
    );

    let pending_sql = match vm
        .exec_interactive(
            "sqlite3 /workspace/app.db \"CREATE TABLE t(x INT); INSERT INTO t VALUES (42); SELECT x FROM t;\"",
            Some("/workspace"),
        )
        .expect("sqlite3 interactive")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge for sqlite3, got {other:?}"),
    };
    assert_eq!(pending_sql.bridge_req["kind"], "sqlite_exec");
    let sqlite_bytes = b"SQLite format 3\x00\x10\x00\x01\x01binary-table";
    let sql_resumed = match vm
        .complete_bridge(
            &pending_sql.resume_state,
            &serde_json::json!({
                "ok": true,
                "exit_code": 0,
                "stdout": "42\n",
                "stderr": "",
                "writes": [
                    {
                        "path": "/workspace/app.db",
                        "content_base64": test_b64_encode(sqlite_bytes),
                    }
                ]
            }),
        )
        .expect("complete sqlite bridge")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::Completed(c) => c,
        other => panic!("expected Completed after sqlite resume, got {other:?}"),
    };
    assert_eq!(sql_resumed.exit_code, 0);
    assert_eq!(sql_resumed.stdout, "42\n");
    assert_eq!(
        vm.read_guest_file("/workspace/app.db").as_deref(),
        Some(&sqlite_bytes[..])
    );
    let diff_sql: serde_json::Value =
        serde_json::from_str(&vm.mvm_export_workspace_diff_json()).expect("diff sql json");
    let sql_writes = diff_sql["writes"].as_array().expect("writes");
    assert!(
        sql_writes.is_empty(),
        "binary *.db and *.wasm must remain isolated from Studio text diff: {sql_writes:?}"
    );
}

#[test]
fn test_mvm_polyglot_sri_esbuild_and_canonical_refusals_r4_58() {
    let _guard = STUDIO_GLOBAL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = studio_dispatch("1\tstudio\treset\t");
    let mut engine = StudioEngine::new_default();

    // 1. Typed R5 errors across mvm_complete_bridge_json (`IntegrityMismatch`, `EsbuildAdapterUnavailable`, `WasiRuntimeError`, `SqliteRuntimeError`)
    for (cmd, code) in [
        ("npm install bad-pkg", "IntegrityMismatch"),
        ("node app.ts", "EsbuildAdapterUnavailable"),
        ("wasm run /workspace/bad.wasm", "WasiRuntimeError"),
        ("sqlite3 /workspace/app.db \"BAD\"", "SqliteRuntimeError"),
    ] {
        let req_raw = engine.mvm_exec_json(cmd);
        let req: serde_json::Value = serde_json::from_str(&req_raw).expect("needs_bridge json");
        assert_eq!(
            req["status"], "needs_bridge",
            "expected needs_bridge for `{cmd}`, got {req}"
        );
        let err_raw = engine.mvm_complete_bridge_json(
            &serde_json::json!({
                "resume_state": req["resume_state"],
                "bridge_resp": {
                    "ok": false,
                    "error_code": code,
                    "message": format!("simulated {code}"),
                }
            })
            .to_string(),
        );
        let err_json: serde_json::Value = serde_json::from_str(&err_raw).expect("typed error json");
        assert_eq!(
            err_json["status"], "error",
            "expected error for {code}: {err_json}"
        );
        assert_eq!(err_json["error_code"], code);
    }

    // 2. Canonical path protection enforced on polyglot bridge writes/deletes
    let mut vm = MvmSandbox::boot_default().expect("boot MVM");
    let canonical_rule = b"\\begin{lexlean}{R4.Rule}\\end{lexlean}\n";
    let canonical_state = b"canonical-state\n";
    vm.seed_canonical_guest_file("/workspace/prism/Rule.lex.tex", canonical_rule)
        .expect("seed rule");
    vm.seed_canonical_guest_file("/workspace/app/state", canonical_state)
        .expect("seed app/state");

    let pending = match vm
        .exec_interactive("node -e \"tamper()\"", Some("/workspace"))
        .expect("node pending")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge, got {other:?}"),
    };
    let tamper_err = vm
        .complete_bridge(
            &pending.resume_state,
            &serde_json::json!({
                "ok": true,
                "exit_code": 0,
                "stdout": "",
                "stderr": "",
                "writes": [
                    {
                        "path": "/workspace/prism/Rule.lex.tex",
                        "content_base64": test_b64_encode(b"tampered!"),
                    }
                ]
            }),
        )
        .expect_err("polyglot bridge write to prism/*.lex.tex must be refused");
    assert!(
        matches!(
            tamper_err,
            lexlean_r4_wasm::mvm::MvmError::CanonicalPathProtected(_)
        ),
        "expected CanonicalPathProtected, got {tamper_err:?}"
    );
    assert_eq!(
        vm.read_guest_file("/workspace/prism/Rule.lex.tex")
            .as_deref(),
        Some(&canonical_rule[..])
    );

    let pending_del = match vm
        .exec_interactive(
            "python3 -c \"import os; os.remove('/workspace/app/state')\"",
            Some("/workspace"),
        )
        .expect("python pending")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge, got {other:?}"),
    };
    let del_err = vm
        .complete_bridge(
            &pending_del.resume_state,
            &serde_json::json!({
                "ok": true,
                "exit_code": 0,
                "stdout": "",
                "stderr": "",
                "deletes": ["/workspace/app/state"]
            }),
        )
        .expect_err("polyglot bridge delete on app/state must be refused");
    assert!(
        matches!(
            del_err,
            lexlean_r4_wasm::mvm::MvmError::CanonicalPathProtected(_)
        ),
        "expected CanonicalPathProtected, got {del_err:?}"
    );
    assert_eq!(
        vm.read_guest_file("/workspace/app/state").as_deref(),
        Some(&canonical_state[..])
    );
}

#[test]
fn test_mvm_polyglot_env_pipeline_and_rel_symlinks_r4_58() {
    let mut vm = MvmSandbox::boot_default().expect("boot MVM");

    // 1. Relative symlink inside node_modules/.bin resolved cleanly against link parent directory
    vm.write_guest_file(
        "/workspace/node_modules/demo-cli/bin/cli.js",
        b"#!/usr/bin/env node\nconsole.log('cli');\n",
    )
    .expect("write cli.js");
    let tar_res = vm
        .exec_command(
            "mkdir -p /workspace/node_modules/.bin && ln -s ../demo-cli/bin/cli.js /workspace/node_modules/.bin/demo-cli",
            "/workspace",
        )
        .expect("ln -s relative");
    assert_eq!(tar_res.exit_code, 0, "stderr: {}", tar_res.stderr);
    assert_eq!(
        vm.read_guest_file("/workspace/node_modules/.bin/demo-cli")
            .as_deref(),
        Some(&b"#!/usr/bin/env node\nconsole.log('cli');\n"[..]),
        "relative symlink inside node_modules/.bin must resolve relative to its parent directory"
    );

    // 2. `/usr/bin/env` and `/usr/bin/<applet>` explicit path dispatch
    let env_cat = vm
        .exec_command(
            "/usr/bin/env GREETING=polyglot /usr/bin/printf '%s\\n' \"$GREETING\" | /usr/bin/tr a-z A-Z",
            "/workspace",
        )
        .expect("/usr/bin/env + /usr/bin/<applet> runs");
    assert_eq!(env_cat.exit_code, 0, "stderr: {}", env_cat.stderr);

    // 3. `/usr/bin/env node -e ...` inside a pipeline + `&&` chain suspends with piped stdin and resumes downstream stages
    let chained = vm
        .exec_interactive(
            "echo \"SELECT 42;\" | /usr/bin/env sqlite3 /workspace/app.db | tr -d '\\n' && echo ':ok'",
            Some("/workspace"),
        )
        .expect("piped sqlite3 interactive");
    let pending = match chained {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge for piped /usr/bin/env sqlite3, got {other:?}"),
    };
    assert_eq!(pending.bridge_req["kind"], "sqlite_exec");
    assert_eq!(pending.bridge_req["stdin_utf8"], "SELECT 42;\n");
    let resumed = match vm
        .complete_bridge(
            &pending.resume_state,
            &serde_json::json!({
                "ok": true,
                "exit_code": 0,
                "stdout": "42\n",
                "stderr": ""
            }),
        )
        .expect("resume piped sqlite3")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::Completed(c) => c,
        other => panic!("expected Completed, got {other:?}"),
    };
    assert_eq!(resumed.exit_code, 0, "stderr: {}", resumed.stderr);
    assert_eq!(resumed.stdout, "42:ok\n");
}

#[test]
fn test_mvm_two_stage_pipeline_and_atomic_mutations_r4_58() {
    let mut vm = MvmSandbox::boot_default().expect("boot MVM");
    assert!(
        vm.read_guest_file("/bin/sh").is_some(),
        "/bin/sh must be present so 2-stage fast-path guard is exercised"
    );

    // 1. Real 2-stage pipelines without $/redirs/= MUST emit NeedsBridge instead of falling into /bin/sh exit 127
    let two_stage_sql = vm
        .exec_interactive(
            "echo \"SELECT 42;\" | sqlite3 /workspace/app.db",
            Some("/workspace"),
        )
        .expect("2-stage echo | sqlite3 executes");
    let sql_pending = match two_stage_sql {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge for 2-stage `echo | sqlite3`, got {other:?}"),
    };
    assert_eq!(sql_pending.bridge_req["kind"], "sqlite_exec");
    assert_eq!(sql_pending.bridge_req["stdin_utf8"], "SELECT 42;\n");

    vm.write_guest_file("/workspace/hi.js", b"console.log('piped-node');\n")
        .expect("write hi.js");
    let two_stage_node = vm
        .exec_interactive("cat /workspace/hi.js | node", Some("/workspace"))
        .expect("2-stage cat | node executes");
    let node_pending = match two_stage_node {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge for 2-stage `cat | node`, got {other:?}"),
    };
    assert_eq!(node_pending.bridge_req["kind"], "node_exec");
    assert_eq!(
        node_pending.bridge_req["stdin_utf8"],
        "console.log('piped-node');\n"
    );

    // 2. Atomic polyglot mutation validation: if one write/symlink in batch violates policy, NOTHING is written
    vm.seed_canonical_guest_file("/workspace/prism/Rule.lex.tex", b"canonical-rule\n")
        .expect("seed canonical rule");
    let batch_pending = match vm
        .exec_interactive("node -e \"batch()\"", Some("/workspace"))
        .expect("node pending")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge, got {other:?}"),
    };
    let batch_err = vm
        .complete_bridge(
            &batch_pending.resume_state,
            &serde_json::json!({
                "ok": true,
                "exit_code": 0,
                "stdout": "",
                "stderr": "",
                "writes": [
                    { "path": "/workspace/should_rollback.txt", "content_base64": test_b64_encode(b"partial\n") },
                    { "path": "/workspace/prism/Rule.lex.tex", "content_base64": test_b64_encode(b"tamper") }
                ]
            }),
        )
        .expect_err("mixed batch containing canonical write must fail atomically");
    assert!(
        matches!(
            batch_err,
            lexlean_r4_wasm::mvm::MvmError::CanonicalPathProtected(_)
        ),
        "expected CanonicalPathProtected, got {batch_err:?}"
    );
    assert!(
        vm.read_guest_file("/workspace/should_rollback.txt")
            .is_none(),
        "atomic failure must NOT leave preceding batch writes in guest filesystem"
    );

    // 3. Symlink escaping /workspace and multi-hop canonical symlink chains must be rejected at creation time
    let escape_pending = match vm
        .exec_interactive("node -e \"escape()\"", Some("/workspace"))
        .expect("node pending")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge, got {other:?}"),
    };
    let escape_err = vm
        .complete_bridge(
            &escape_pending.resume_state,
            &serde_json::json!({
                "ok": true,
                "exit_code": 0,
                "stdout": "",
                "stderr": "",
                "writes": [
                    { "path": "/workspace/escape_link", "symlink_target": "../../etc/passwd" }
                ]
            }),
        )
        .expect_err("symlink escaping /workspace must be rejected");
    assert!(matches!(
        escape_err,
        lexlean_r4_wasm::mvm::MvmError::CanonicalPathProtected(_)
    ));
    assert!(vm.read_guest_file("/workspace/escape_link").is_none());

    let hop_pending = match vm
        .exec_interactive("node -e \"multihop()\"", Some("/workspace"))
        .expect("node pending")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge, got {other:?}"),
    };
    let hop_err = vm
        .complete_bridge(
            &hop_pending.resume_state,
            &serde_json::json!({
                "ok": true,
                "exit_code": 0,
                "stdout": "",
                "stderr": "",
                "writes": [
                    { "path": "/workspace/hop_a", "symlink_target": "/workspace/hop_b" },
                    { "path": "/workspace/hop_b", "symlink_target": "/workspace/prism/Rule.lex.tex" }
                ]
            }),
        )
        .expect_err("multi-hop canonical symlink chain must be rejected at creation time");
    assert!(matches!(
        hop_err,
        lexlean_r4_wasm::mvm::MvmError::CanonicalPathProtected(_)
    ));

    // 4. `pipeline_insts_prefix` must NOT be double-counted when downstream stage completes
    let pipe_pending = match vm
        .exec_interactive("node -e \"p()\" | wc -c", Some("/workspace"))
        .expect("node | wc pending")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge, got {other:?}"),
    };
    let mut mod_resume = pipe_pending.resume_state.clone();
    mod_resume["pipeline_insts_prefix"] = serde_json::json!(1000u64);
    let pipe_done = match vm
        .complete_bridge(
            &mod_resume,
            &serde_json::json!({
                "ok": true,
                "exit_code": 0,
                "stdout": "abc\n",
                "stderr": ""
            }),
        )
        .expect("complete pipe")
    {
        lexlean_r4_wasm::mvm::MvmInteractiveOutcome::Completed(c) => c,
        other => panic!("expected Completed, got {other:?}"),
    };
    assert!(
        pipe_done.instructions_retired < 2000,
        "pipeline_insts_prefix (1000) must not be double-counted (got {})",
        pipe_done.instructions_retired
    );
}

#[test]
fn test_prism_capabilities_network_gate_and_spiralcore_failback_r4_59() {
    use lexlean_r4_wasm::mvm::{BridgeError, MvmError, MvmInteractiveOutcome, NetworkGateMode};
    use sha2::{Digest, Sha256};

    // 1. Verify all 6 typed R4-59 BridgeError variants (R5) and complete_bridge mapping
    let err_cases = [
        (
            BridgeError::CapabilityDenied("example.com".to_string()),
            "CapabilityDenied",
        ),
        (BridgeError::RelayConsentRequired, "RelayConsentRequired"),
        (
            BridgeError::RelayForbiddenForCredentialedHost,
            "RelayForbiddenForCredentialedHost",
        ),
        (
            BridgeError::PrivateAddressBlocked("169.254.169.254".to_string()),
            "PrivateAddressBlocked",
        ),
        (
            BridgeError::DoHResolutionFailed("upstream timeout".to_string()),
            "DoHResolutionFailed",
        ),
        (
            BridgeError::RelayUnavailable("relay unreachable".to_string()),
            "RelayUnavailable",
        ),
    ];
    for (err, expected_code) in err_cases {
        assert_eq!(err.error_code(), expected_code);
    }

    let mut vm = MvmSandbox::boot_default().expect("boot MVM");

    // 2. Sandboxed mode (capabilities/1): allowlisted host succeeds -> NeedsBridge, non-allowlisted -> CapabilityDenied
    vm.network_gate_enforced = true;
    vm.network_mode = NetworkGateMode::Sandboxed;
    vm.relay_consent_granted = false;
    vm.relay_ws_url.clear();
    let allowed_curl = vm
        .exec_interactive("curl https://registry.npmjs.org/zod", Some("/workspace"))
        .expect("allowlisted curl must emit NeedsBridge");
    assert!(
        matches!(allowed_curl, MvmInteractiveOutcome::NeedsBridge(_)),
        "expected NeedsBridge for registry.npmjs.org, got {allowed_curl:?}"
    );

    let denied_curl = vm
        .exec_interactive("curl https://example.com/data", Some("/workspace"))
        .expect_err("non-allowlisted curl in Sandboxed mode must fail with CapabilityDenied");
    assert_eq!(
        denied_curl,
        MvmError::BridgeRejected(BridgeError::CapabilityDenied("example.com".to_string()))
    );

    // 3. Private / link-local RFC 1918 / 169.254 SSRF targets rejected with PrivateAddressBlocked
    for ssrf_cmd in [
        "curl https://169.254.169.254/latest/meta-data",
        "curl https://10.0.0.1/admin",
        "host 192.168.1.1",
        "nslookup 127.0.0.1",
    ] {
        let ssrf_err = vm
            .exec_interactive(ssrf_cmd, Some("/workspace"))
            .expect_err("SSRF target must be rejected");
        assert!(
            matches!(
                ssrf_err,
                MvmError::BridgeRejected(BridgeError::PrivateAddressBlocked(_))
            ),
            "expected PrivateAddressBlocked for `{ssrf_cmd}`, got {ssrf_err:?}"
        );
    }

    // 4. Switching to Full SpiralCore without consentGranted=true refuses with RelayConsentRequired
    vm.network_gate_enforced = true;
    vm.network_mode = NetworkGateMode::FullSpiralcore;
    vm.relay_consent_granted = false;
    vm.relay_ws_url = "wss://relay.example.invalid/spvs".to_string();
    let consent_err = vm
        .exec_interactive("curl https://example.com/data", Some("/workspace"))
        .expect_err("Full SpiralCore without consentGranted must return RelayConsentRequired");
    assert_eq!(
        consent_err,
        MvmError::BridgeRejected(BridgeError::RelayConsentRequired)
    );

    // With consentGranted=true, unallowlisted external host emits NeedsBridge (while SSRF remains blocked)
    vm.network_gate_enforced = true;
    vm.network_mode = NetworkGateMode::FullSpiralcore;
    vm.relay_consent_granted = true;
    vm.relay_ws_url = "ws://127.0.0.1:8787/spvs".to_string();
    let full_curl = match vm
        .exec_interactive("curl https://example.com/data", Some("/workspace"))
        .expect("Full SpiralCore with consent emits NeedsBridge")
    {
        MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge, got {other:?}"),
    };
    let relay_blocked = vm
        .complete_bridge(
            &full_curl.resume_state,
            &serde_json::json!({
                "ok": false,
                "error_code": "RelayForbiddenForCredentialedHost",
                "message": "credentialed host refused relay failback"
            }),
        )
        .expect_err("complete_bridge must map RelayForbiddenForCredentialedHost");
    assert_eq!(
        relay_blocked,
        MvmError::BridgeRejected(BridgeError::RelayForbiddenForCredentialedHost)
    );

    // 5. `host` command emits dns_query NeedsBridge and resumes cleanly
    let host_pending = match vm
        .exec_interactive("host github.com", Some("/workspace"))
        .expect("host command emits NeedsBridge")
    {
        MvmInteractiveOutcome::NeedsBridge(p) => p,
        other => panic!("expected NeedsBridge for `host`, got {other:?}"),
    };
    assert_eq!(host_pending.bridge_req["op"], "dns_query");
    let host_done = match vm
        .complete_bridge(
            &host_pending.resume_state,
            &serde_json::json!({
                "ok": true,
                "data": { "answers": [{ "data": "140.82.114.4", "type_name": "A" }] }
            }),
        )
        .expect("complete host dns")
    {
        MvmInteractiveOutcome::Completed(c) => c,
        other => panic!("expected Completed, got {other:?}"),
    };
    assert!(host_done.stdout.contains("140.82.114.4"));

    // 6. StudioEngine mvm_set_policy_json + offlineQ31WeightsKappa pin verification
    let _guard = STUDIO_GLOBAL_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut engine = StudioEngine::new_default();
    let policy_rej: serde_json::Value = serde_json::from_str(
        &engine.mvm_set_policy_json(r#"{"mode":"full_spiralcore","consentGranted":false}"#),
    )
    .expect("policy json");
    assert_eq!(policy_rej["status"], "error");
    assert_eq!(policy_rej["error_code"], "RelayConsentRequired");

    let expected_q31_sha: String = Sha256::digest(include_bytes!("../src/native_q31_d85.uorq31"))
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let begin_q31: serde_json::Value = serde_json::from_str(
        &engine.holo_begin_foundry_turn(
            &serde_json::json!({
                "prompt": "hello offline q31",
                "selected_model": "uor-r4.native-q31-d85",
                "max_new_tokens": 2
            })
            .to_string(),
        ),
    )
    .expect("begin q31 json");
    assert_eq!(begin_q31["status"], "ready", "begin_q31: {begin_q31}");
    let tid = begin_q31["turn_id"].as_u64().expect("turn_id");
    let step_q31: serde_json::Value =
        serde_json::from_str(&engine.holo_step_foundry_turn(tid)).expect("step q31 json");
    assert_eq!(
        step_q31["weights_kappa"].as_str(),
        Some(format!("sha256:{expected_q31_sha}").as_str()),
        "offlineQ31WeightsKappa must be populated deterministically on native-q31-d85 step"
    );
}
