#!/usr/bin/env python3
"""
Cross-compile the 10 freestanding RV64IMAC static ELF64 packages for the
UOR-R4 MVM sandbox (`uor-mvm-pkg-index/1`), verify ELF64 + EM_RISCV (243) +
load-segment bounds (< 4 MiB guest RAM), and write `gpu/weights/mvm-pkgs/*.elf`
plus `gpu/weights/mvm-pkgs/index.json`.
"""

from __future__ import annotations

import hashlib
import json
import os
import struct
import subprocess
import sys
from pathlib import Path

PACKAGES = [
    ("jq", "pkg_jq.c", "1.0.0", "JSON parser, pretty-printer, and path filter"),
    ("rg", "pkg_rg.c", "1.0.0", "Recursive substring file search across directories"),
    ("tree", "pkg_tree.c", "1.0.0", "Recursive ASCII directory hierarchy viewer"),
    ("xxd", "pkg_xxd.c", "1.0.0", "Canonical 16-byte hexadecimal and ASCII dump"),
    ("sha256sum", "pkg_sha256sum.c", "1.0.0", "FIPS 180-4 SHA-256 digest utility"),
    ("bc", "pkg_bc.c", "1.0.0", "64-bit integer arithmetic and variable calculator"),
    ("wc", "pkg_wc.c", "1.0.0", "Line, word, and byte counter"),
    ("head", "pkg_head.c", "1.0.0", "Output leading lines or bytes of files"),
    ("tail", "pkg_tail.c", "1.0.0", "Output trailing lines or bytes of files"),
    ("http", "pkg_http.c", "1.0.0", "Zero-NIC AF_VSOCK SpiralCore HTTP egress client"),
]


def verify_riscv64_elf(path: Path) -> dict:
    raw = path.read_bytes()
    if len(raw) < 64:
        raise ValueError(f"{path}: truncated ELF ({len(raw)} bytes)")
    if raw[:4] != b"\x7fELF":
        raise ValueError(f"{path}: invalid ELF magic {raw[:4]!r}")
    ei_class = raw[4]
    ei_data = raw[5]
    if ei_class != 2 or ei_data != 1:
        raise ValueError(f"{path}: expected ELF64 little-endian, got class={ei_class} data={ei_data}")
    e_type, e_machine, _e_ver, e_entry, e_phoff, _e_shoff, e_flags, _ehsize, e_phentsize, e_phnum = (
        struct.unpack_from("<HHIQQQIHHH", raw, 16)
    )
    if e_type != 2:
        raise ValueError(f"{path}: expected ET_EXEC (2), got e_type={e_type}")
    if e_machine != 243:
        raise ValueError(f"{path}: expected EM_RISCV (243), got e_machine={e_machine}")
    if (e_flags & 0x6) != 0:
        raise ValueError(f"{path}: float ABI flags set (e_flags=0x{e_flags:x}); MVM is RV64IMAC integer-only")

    load_segments = 0
    max_vaddr_end = 0
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_type, _p_flags, _p_offset, p_vaddr, _p_paddr, p_filesz, p_memsz, _p_align = (
            struct.unpack_from("<IIQQQQQQ", raw, off)
        )
        if p_type == 1:  # PT_LOAD
            load_segments += 1
            seg_end = p_vaddr + p_memsz
            if p_vaddr < 0x1000 or seg_end > 0x0040_0000:
                raise ValueError(
                    f"{path}: PT_LOAD [0x{p_vaddr:x}..0x{seg_end:x}] outside MVM user text/bss window"
                )
            if p_filesz > len(raw):
                raise ValueError(f"{path}: PT_LOAD filesz exceeds binary size")
            max_vaddr_end = max(max_vaddr_end, seg_end)
        elif p_type == 3:  # PT_INTERP
            raise ValueError(f"{path}: dynamic PT_INTERP forbidden; package must be static")

    if load_segments == 0:
        raise ValueError(f"{path}: zero PT_LOAD segments")

    return {
        "entry_hex": f"0x{e_entry:x}",
        "max_vaddr_hex": f"0x{max_vaddr_end:x}",
        "e_flags_hex": f"0x{e_flags:x}",
        "byte_length": len(raw),
        "sha256": hashlib.sha256(raw).hexdigest(),
    }


def main() -> int:
    guest_dir = Path(__file__).resolve().parent
    repo_root = guest_dir.parent.parent
    out_dir = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else (repo_root / "gpu" / "weights" / "mvm-pkgs")
    out_dir.mkdir(parents=True, exist_ok=True)

    cc = os.environ.get("RISCV_CC", "riscv64-linux-gnu-gcc")
    entries = []

    for name, src_name, version, description in PACKAGES:
        src_path = guest_dir / src_name
        elf_name = f"{name}.elf"
        out_path = out_dir / elf_name
        cmd = [
            cc,
            "-march=rv64imac",
            "-mabi=lp64",
            "-static",
            "-O2",
            "-nostdlib",
            "-ffreestanding",
            "-fno-builtin",
            "-fno-tree-loop-distribute-patterns",
            "-fno-pie",
            "-no-pie",
            "-fno-stack-protector",
            "-Wl,-Ttext-segment=0x10000",
            "-Wl,--build-id=none",
            "-s",
            f"-I{guest_dir}",
            str(src_path),
            "-lgcc",
            "-o",
            str(out_path),
        ]
        subprocess.run(cmd, check=True)
        meta = verify_riscv64_elf(out_path)
        entries.append(
            {
                "name": name,
                "version": version,
                "file": elf_name,
                "path": f"/weights/mvm-pkgs/{elf_name}",
                "install_path": f"/usr/local/bin/{name}",
                "byte_length": meta["byte_length"],
                "sha256": meta["sha256"],
                "description": description,
            }
        )
        print(
            f"[mvm-pkg] built {elf_name:<14} {meta['byte_length']:>6} B  "
            f"entry={meta['entry_hex']}  end={meta['max_vaddr_hex']}  sha256={meta['sha256'][:16]}..."
        )

    index_doc = {
        "schema": "uor-mvm-pkg-index/1",
        "arch": "riscv64-linux-musl",
        "elf_machine": 243,
        "packages": entries,
    }
    index_path = out_dir / "index.json"
    index_path.write_text(json.dumps(index_doc, indent=2) + "\n", encoding="utf-8")
    print(f"[mvm-pkg] wrote {index_path} ({len(entries)} packages)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
