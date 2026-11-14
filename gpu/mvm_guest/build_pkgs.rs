//! Cross-compile the 10 freestanding RV64IMAC static ELF64 packages for the
//! UOR-R4 MVM sandbox (`uor-mvm-pkg-index/1`), verify ELF64 + EM_RISCV (243) +
//! load-segment bounds (< 4 MiB guest RAM), and write `gpu/weights/mvm-pkgs/*.elf`
//! plus `gpu/weights/mvm-pkgs/index.json`.

#[path = "../bpe_json.rs"]
mod bpe_json;

use bpe_json::{json_escape, sha256_hex, JsonVal};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};

struct PkgSpec {
    name: &'static str,
    src_name: &'static str,
    version: &'static str,
    description: &'static str,
}

const PACKAGES: &[PkgSpec] = &[
    PkgSpec {
        name: "jq",
        src_name: "pkg_jq.c",
        version: "1.0.0",
        description: "JSON parser, pretty-printer, and path filter",
    },
    PkgSpec {
        name: "rg",
        src_name: "pkg_rg.c",
        version: "1.0.0",
        description: "Recursive substring file search across directories",
    },
    PkgSpec {
        name: "tree",
        src_name: "pkg_tree.c",
        version: "1.0.0",
        description: "Recursive ASCII directory hierarchy viewer",
    },
    PkgSpec {
        name: "xxd",
        src_name: "pkg_xxd.c",
        version: "1.0.0",
        description: "Canonical 16-byte hexadecimal and ASCII dump",
    },
    PkgSpec {
        name: "sha256sum",
        src_name: "pkg_sha256sum.c",
        version: "1.0.0",
        description: "FIPS 180-4 SHA-256 digest utility",
    },
    PkgSpec {
        name: "bc",
        src_name: "pkg_bc.c",
        version: "1.0.0",
        description: "64-bit integer arithmetic and variable calculator",
    },
    PkgSpec {
        name: "wc",
        src_name: "pkg_wc.c",
        version: "1.0.0",
        description: "Line, word, and byte counter",
    },
    PkgSpec {
        name: "head",
        src_name: "pkg_head.c",
        version: "1.0.0",
        description: "Output leading lines or bytes of files",
    },
    PkgSpec {
        name: "tail",
        src_name: "pkg_tail.c",
        version: "1.0.0",
        description: "Output trailing lines or bytes of files",
    },
    PkgSpec {
        name: "http",
        src_name: "pkg_http.c",
        version: "1.0.0",
        description: "Zero-NIC AF_VSOCK SpiralCore HTTP egress client",
    },
];

struct ElfMeta {
    entry_hex: String,
    max_vaddr_hex: String,
    _e_flags_hex: String,
    byte_length: usize,
    sha256: String,
}

fn read_u16_le(buf: &[u8], off: usize) -> Result<u16, String> {
    let bytes: [u8; 2] = buf
        .get(off..off + 2)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| format!("truncated ELF u16 at offset {off}"))?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32_le(buf: &[u8], off: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = buf
        .get(off..off + 4)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| format!("truncated ELF u32 at offset {off}"))?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64_le(buf: &[u8], off: usize) -> Result<u64, String> {
    let bytes: [u8; 8] = buf
        .get(off..off + 8)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| format!("truncated ELF u64 at offset {off}"))?;
    Ok(u64::from_le_bytes(bytes))
}

fn verify_riscv64_elf(path: &Path) -> Result<ElfMeta, String> {
    let raw = fs::read(path).map_err(|e| format!("failed reading {}: {e}", path.display()))?;
    if raw.len() < 64 {
        return Err(format!(
            "{}: truncated ELF ({} bytes)",
            path.display(),
            raw.len()
        ));
    }
    if &raw[..4] != b"\x7fELF" {
        return Err(format!(
            "{}: invalid ELF magic {:?}",
            path.display(),
            &raw[..4]
        ));
    }
    let ei_class = raw[4];
    let ei_data = raw[5];
    if ei_class != 2 || ei_data != 1 {
        return Err(format!(
            "{}: expected ELF64 little-endian, got class={ei_class} data={ei_data}",
            path.display()
        ));
    }

    let e_type = read_u16_le(&raw, 16)?;
    let e_machine = read_u16_le(&raw, 18)?;
    let e_entry = read_u64_le(&raw, 24)?;
    let e_phoff = read_u64_le(&raw, 32)?;
    let e_flags = read_u32_le(&raw, 48)?;
    let e_phentsize = read_u16_le(&raw, 54)? as usize;
    let e_phnum = read_u16_le(&raw, 56)? as usize;

    if e_type != 2 {
        return Err(format!(
            "{}: expected ET_EXEC (2), got e_type={e_type}",
            path.display()
        ));
    }
    if e_machine != 243 {
        return Err(format!(
            "{}: expected EM_RISCV (243), got e_machine={e_machine}",
            path.display()
        ));
    }
    if (e_flags & 0x6) != 0 {
        return Err(format!(
            "{}: float ABI flags set (e_flags=0x{e_flags:x}); MVM is RV64IMAC integer-only",
            path.display()
        ));
    }
    if e_phnum > 0 && e_phentsize < 56 {
        return Err(format!(
            "{}: invalid e_phentsize={e_phentsize}",
            path.display()
        ));
    }

    let mut load_segments = 0usize;
    let mut exec_load_segments = 0usize;
    let mut max_vaddr_end = 0u64;
    let ph_base = usize::try_from(e_phoff)
        .map_err(|_| format!("{}: e_phoff overflows usize", path.display()))?;

    for i in 0..e_phnum {
        let off = ph_base
            .checked_add(i * e_phentsize)
            .ok_or_else(|| format!("{}: phdr offset overflow", path.display()))?;
        if off + 56 > raw.len() {
            return Err(format!(
                "{}: program header {i} truncated beyond EOF",
                path.display()
            ));
        }
        let p_type = read_u32_le(&raw, off)?;
        let p_flags = read_u32_le(&raw, off + 4)?;
        let p_offset = read_u64_le(&raw, off + 8)?;
        let p_vaddr = read_u64_le(&raw, off + 16)?;
        let p_filesz = read_u64_le(&raw, off + 32)?;
        let p_memsz = read_u64_le(&raw, off + 40)?;

        if p_type == 1 {
            // PT_LOAD
            load_segments += 1;
            if (p_flags & 1) != 0 {
                // PF_X
                exec_load_segments += 1;
            }
            let seg_end = p_vaddr
                .checked_add(p_memsz)
                .ok_or_else(|| format!("{}: PT_LOAD vaddr+memsz overflow", path.display()))?;
            if p_vaddr < 0x1000 || seg_end > 0x0040_0000 {
                return Err(format!(
                    "{}: PT_LOAD [0x{p_vaddr:x}..0x{seg_end:x}] outside MVM user text/bss window",
                    path.display()
                ));
            }
            let file_end = p_offset
                .checked_add(p_filesz)
                .ok_or_else(|| format!("{}: PT_LOAD offset+filesz overflow", path.display()))?;
            if p_filesz > raw.len() as u64 || file_end > raw.len() as u64 {
                return Err(format!(
                    "{}: PT_LOAD filesz exceeds binary size",
                    path.display()
                ));
            }
            if seg_end > max_vaddr_end {
                max_vaddr_end = seg_end;
            }
        } else if p_type == 3 {
            // PT_INTERP
            return Err(format!(
                "{}: dynamic PT_INTERP forbidden; package must be static",
                path.display()
            ));
        }
    }

    if load_segments == 0 {
        return Err(format!("{}: zero PT_LOAD segments", path.display()));
    }
    if exec_load_segments == 0 {
        return Err(format!(
            "{}: zero executable PT_LOAD (PF_X) segments",
            path.display()
        ));
    }

    Ok(ElfMeta {
        entry_hex: format!("0x{e_entry:x}"),
        max_vaddr_hex: format!("0x{max_vaddr_end:x}"),
        _e_flags_hex: format!("0x{e_flags:x}"),
        byte_length: raw.len(),
        sha256: sha256_hex(&raw),
    })
}

fn resolve_guest_dir() -> Result<PathBuf, String> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = env::current_exe() {
        for anc in exe.ancestors().take(6) {
            roots.push(anc.to_path_buf());
        }
    }
    if let Ok(cwd) = env::current_dir() {
        for anc in cwd.ancestors().take(6) {
            roots.push(anc.to_path_buf());
        }
    }
    for r in &roots {
        for cand in [
            r.to_path_buf(),
            r.join("gpu").join("mvm_guest"),
            r.join("mvm_guest"),
        ] {
            if cand.join("pkg_jq.c").is_file() && cand.join("rt0.h").is_file() {
                return fs::canonicalize(&cand)
                    .map_err(|e| format!("failed canonicalizing {}: {e}", cand.display()));
            }
        }
    }
    Err("could not locate gpu/mvm_guest directory containing pkg_jq.c and rt0.h".to_string())
}

fn find_riscv_cc() -> Result<String, String> {
    if let Ok(cc) = env::var("RISCV_CC") {
        let trimmed = cc.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    for cand in [
        "/opt/riscv/bin/riscv64-linux-gnu-gcc",
        "riscv64-linux-gnu-gcc",
        "riscv64-unknown-linux-gnu-gcc",
    ] {
        if let Ok(st) = Command::new(cand)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            if st.success() {
                return Ok(cand.to_string());
            }
        }
    }
    Err("no RISC-V cross-compiler found (set RISCV_CC or install riscv64-linux-gnu-gcc / riscv64-unknown-linux-gnu-gcc)".to_string())
}

struct PkgEntry<'a> {
    spec: &'a PkgSpec,
    elf_name: String,
    meta: ElfMeta,
}

fn format_index_json(entries: &[PkgEntry<'_>]) -> String {
    let mut out = String::from(
        "{\n  \"schema\": \"uor-mvm-pkg-index/1\",\n  \"arch\": \"riscv64-linux-musl\",\n  \"elf_machine\": 243,\n  \"packages\": [\n",
    );
    for (idx, e) in entries.iter().enumerate() {
        let path_val = format!("/weights/mvm-pkgs/{}", e.elf_name);
        let install_path = format!("/usr/local/bin/{}", e.spec.name);
        out.push_str("    {\n");
        out.push_str(&format!("      \"name\": {},\n", json_escape(e.spec.name)));
        out.push_str(&format!(
            "      \"version\": {},\n",
            json_escape(e.spec.version)
        ));
        out.push_str(&format!("      \"file\": {},\n", json_escape(&e.elf_name)));
        out.push_str(&format!("      \"path\": {},\n", json_escape(&path_val)));
        out.push_str(&format!(
            "      \"install_path\": {},\n",
            json_escape(&install_path)
        ));
        out.push_str(&format!("      \"byte_length\": {},\n", e.meta.byte_length));
        out.push_str(&format!(
            "      \"sha256\": {},\n",
            json_escape(&e.meta.sha256)
        ));
        out.push_str(&format!(
            "      \"description\": {}\n",
            json_escape(e.spec.description)
        ));
        if idx + 1 < entries.len() {
            out.push_str("    },\n");
        } else {
            out.push_str("    }\n");
        }
    }
    out.push_str("  ]\n}\n");
    out
}

fn verify_index_json(index_path: &Path, entries: &[PkgEntry<'_>]) -> Result<(), String> {
    let raw = fs::read_to_string(index_path)
        .map_err(|e| format!("failed reading {}: {e}", index_path.display()))?;
    let doc = JsonVal::parse(&raw)
        .map_err(|e| format!("invalid JSON in {}: {e}", index_path.display()))?;
    let schema = doc.get("schema").and_then(|v| v.as_str()).unwrap_or("");
    if schema != "uor-mvm-pkg-index/1" {
        return Err(format!(
            "{}: unexpected schema '{schema}'",
            index_path.display()
        ));
    }
    let elf_machine = doc
        .get("elf_machine")
        .and_then(|v| v.as_i64())
        .unwrap_or(-1);
    if elf_machine != 243 {
        return Err(format!(
            "{}: expected elf_machine=243, got {elf_machine}",
            index_path.display()
        ));
    }
    let pkgs = doc
        .get("packages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("{}: missing 'packages' array", index_path.display()))?;
    for e in entries {
        let found = pkgs
            .iter()
            .find(|p| p.get("name").and_then(|v| v.as_str()) == Some(e.spec.name))
            .ok_or_else(|| {
                format!(
                    "{}: missing package '{}'",
                    index_path.display(),
                    e.spec.name
                )
            })?;
        let blen = found
            .get("byte_length")
            .and_then(|v| v.as_usize())
            .unwrap_or(0);
        let sha = found.get("sha256").and_then(|v| v.as_str()).unwrap_or("");
        if blen != e.meta.byte_length || sha != e.meta.sha256 {
            return Err(format!(
                "{}: digest/size mismatch for '{}' (index: {} B / {}, elf: {} B / {})",
                index_path.display(),
                e.spec.name,
                blen,
                sha,
                e.meta.byte_length,
                e.meta.sha256
            ));
        }
    }
    Ok(())
}

struct CliArgs {
    out_dir: Option<PathBuf>,
    clean: bool,
    verify_only: bool,
    pkg_filter: Vec<String>,
}

fn print_help() {
    println!("Usage: build_pkgs [OUT_DIR] [OPTIONS]");
    println!("Options:");
    println!("  OUT_DIR        Output directory (default: gpu/weights/mvm-pkgs)");
    println!("  --out <DIR>    Explicit output directory");
    println!("  --pkg <NAME>   Filter package(s) to build/verify (comma-separated or repeated)");
    println!("  --verify-only  Verify existing .elf binaries and index.json without compiling");
    println!("  --clean        Remove built .elf binaries and index.json in OUT_DIR");
    println!("  -h, --help     Show this help message and exit");
}

fn parse_args() -> Result<Option<CliArgs>, String> {
    let mut out_dir: Option<PathBuf> = None;
    let mut clean = false;
    let mut verify_only = false;
    let mut pkg_filter: Vec<String> = Vec::new();

    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0usize;
    while i < args.len() {
        let arg = &args[i];
        if arg == "-h" || arg == "--help" {
            print_help();
            return Ok(None);
        } else if arg == "--clean" {
            clean = true;
        } else if arg == "--verify-only" {
            verify_only = true;
        } else if let Some(val) = arg.strip_prefix("--out=") {
            out_dir = Some(PathBuf::from(val));
        } else if arg == "--out" {
            i += 1;
            let val = args
                .get(i)
                .ok_or_else(|| "--out requires a value".to_string())?;
            out_dir = Some(PathBuf::from(val));
        } else if let Some(val) = arg.strip_prefix("--pkg=") {
            for part in val.split(',').filter(|s| !s.is_empty()) {
                pkg_filter.push(part.to_string());
            }
        } else if arg == "--pkg" {
            i += 1;
            let val = args
                .get(i)
                .ok_or_else(|| "--pkg requires a value".to_string())?;
            for part in val.split(',').filter(|s| !s.is_empty()) {
                pkg_filter.push(part.to_string());
            }
        } else if arg.starts_with('-') {
            return Err(format!("unknown option: {arg}"));
        } else if out_dir.is_none() {
            out_dir = Some(PathBuf::from(arg));
        } else {
            return Err(format!("unexpected positional argument: {arg}"));
        }
        i += 1;
    }

    for name in &pkg_filter {
        if !PACKAGES.iter().any(|p| p.name == name) {
            return Err(format!("unknown package '{name}'"));
        }
    }

    Ok(Some(CliArgs {
        out_dir,
        clean,
        verify_only,
        pkg_filter,
    }))
}

fn run() -> Result<(), String> {
    let Some(args) = parse_args()? else {
        return Ok(());
    };

    let guest_dir = resolve_guest_dir()?;
    let repo_root = guest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| format!("unexpected guest_dir layout: {}", guest_dir.display()))?;

    let raw_out_dir = match args.out_dir {
        Some(p) if p.is_absolute() => p,
        Some(p) => env::current_dir()
            .map_err(|e| format!("cwd error: {e}"))?
            .join(p),
        None => repo_root.join("gpu").join("weights").join("mvm-pkgs"),
    };

    let selected: Vec<&PkgSpec> = if args.pkg_filter.is_empty() {
        PACKAGES.iter().collect()
    } else {
        PACKAGES
            .iter()
            .filter(|p| args.pkg_filter.iter().any(|f| f == p.name))
            .collect()
    };

    if args.clean {
        let mut removed = 0usize;
        for spec in &selected {
            let p = raw_out_dir.join(format!("{}.elf", spec.name));
            if p.exists() {
                fs::remove_file(&p).map_err(|e| format!("failed removing {}: {e}", p.display()))?;
                removed += 1;
            }
        }
        let idx = raw_out_dir.join("index.json");
        if idx.exists() && args.pkg_filter.is_empty() {
            fs::remove_file(&idx).map_err(|e| format!("failed removing {}: {e}", idx.display()))?;
            removed += 1;
        }
        println!(
            "[mvm-pkg] cleaned {removed} artifacts in {}",
            raw_out_dir.display()
        );
        return Ok(());
    }

    if !args.verify_only {
        fs::create_dir_all(&raw_out_dir)
            .map_err(|e| format!("failed creating {}: {e}", raw_out_dir.display()))?;
    }
    let out_dir = fs::canonicalize(&raw_out_dir).unwrap_or(raw_out_dir);

    let cc = if args.verify_only {
        String::new()
    } else {
        find_riscv_cc()?
    };

    let mut entries: Vec<PkgEntry<'_>> = Vec::with_capacity(selected.len());
    for spec in selected {
        let src_path = guest_dir.join(spec.src_name);
        let elf_name = format!("{}.elf", spec.name);
        let out_path = out_dir.join(&elf_name);

        if !args.verify_only {
            let inc_flag = format!("-I{}", guest_dir.display());
            let status = Command::new(&cc)
                .args([
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
                    &inc_flag,
                ])
                .arg(&src_path)
                .arg("-lgcc")
                .arg("-o")
                .arg(&out_path)
                .status()
                .map_err(|e| format!("failed executing '{cc}': {e}"))?;
            if !status.success() {
                return Err(format!("{cc} failed building {elf_name}: {status}"));
            }
        }

        let meta = verify_riscv64_elf(&out_path)?;
        println!(
            "[mvm-pkg] {} {:<14} {:>6} B  entry={}  end={}  sha256={}...",
            if args.verify_only {
                "verified"
            } else {
                "built"
            },
            elf_name,
            meta.byte_length,
            meta.entry_hex,
            meta.max_vaddr_hex,
            &meta.sha256[..16]
        );
        entries.push(PkgEntry {
            spec,
            elf_name,
            meta,
        });
    }

    let index_path = out_dir.join("index.json");
    if args.verify_only {
        if index_path.is_file() {
            verify_index_json(&index_path, &entries)?;
            println!(
                "[mvm-pkg] verified {} ({} packages)",
                index_path.display(),
                entries.len()
            );
        }
    } else {
        let json_doc = format_index_json(&entries);
        fs::write(&index_path, json_doc)
            .map_err(|e| format!("failed writing {}: {e}", index_path.display()))?;
        println!(
            "[mvm-pkg] wrote {} ({} packages)",
            index_path.display(),
            entries.len()
        );
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
