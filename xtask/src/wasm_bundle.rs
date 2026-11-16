//! Deterministic Wasm bundle builder and browser provenance verifier (`R4-40`, `R4-42`, `R4-43`).
//!
//! Compiles `crates/wasm` (`lexlean-r4-wasm`) for `wasm32-unknown-unknown`, runs the pinned
//! `wasm-bindgen` CLI, splices `crates/wasm/js/worker_host_bridge.js` plus the managed
//! IndexedDB/Cache Wasm resolver into `lexlean_r4.js`, and writes `provenance.json`.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

type Fail = Box<dyn std::error::Error>;

/// Provenance JSON schema identifier verified by `pages.yml` and conformance tests.
pub const PROVENANCE_SCHEMA: &str = "prismpm/browser-provenance/1";

const SECTION_1_START: &str = "// === UOR_CUSTOM_SECTION_1_START ===";
const SECTION_1_END: &str = "// === UOR_CUSTOM_SECTION_1_END ===";
const SECTION_2_START: &str = "// === UOR_CUSTOM_SECTION_2_START ===";
const SECTION_2_END: &str = "// === UOR_CUSTOM_SECTION_2_END ===";

/// Browser bundle provenance manifest (`public/provenance.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "repo_model::serde")]
pub struct ProvenanceManifest {
    /// Schema marker (`prismpm/browser-provenance/1`).
    pub schema: String,
    /// Authoritative model identity digest.
    pub model_id: String,
    /// Authoritative view manifest digest.
    pub view_manifest_sha256: String,
    /// Authoritative view model digest.
    pub view_model_id: String,
    /// SHA-256 of generated formal core (`crates/wasm/src/lexlean_generated.rs`).
    pub generated_core_sha256: String,
    /// SHA-256 of compiled browser Wasm (`public/lexlean_r4_bg.wasm`).
    pub adapter_wasm_sha256: String,
    /// SHA-256 of spliced JavaScript bindings (`public/lexlean_r4.js`).
    pub binding_javascript_sha256: String,
    /// SHA-256 of workspace dependency lockfile (`Cargo.lock`).
    pub adapter_lock_sha256: String,
}

/// Compute lowercase hexadecimal SHA-256 digest of a byte slice.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

fn sha256_file(path: &Path) -> Result<String, Fail> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("reading {} for sha256: {e}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

fn extract_marked_section(source: &str, start_marker: &str, end_marker: &str) -> Option<String> {
    let start = source.find(start_marker)?;
    let end_rel = source[start..].find(end_marker)?;
    let end = start + end_rel + end_marker.len();
    Some(source[start..end].trim().to_string())
}

/// Inject the managed Wasm cache hook inside `__wbg_init` and splice the host worker bridge.
pub fn transform_wasm_bindgen_js(generated_js: &str, bridge_js: &str) -> Result<String, String> {
    if generated_js.trim().is_empty() {
        return Err("generated wasm-bindgen JavaScript is empty".to_string());
    }
    let init_fn_marker = "async function __wbg_init(module_or_path)";
    if !generated_js.contains(init_fn_marker) {
        return Err(
            "missing `async function __wbg_init(module_or_path)` in wasm-bindgen JS".into(),
        );
    }
    let export_marker = "export { initSync };";
    if !generated_js.contains(export_marker) {
        return Err("missing `export { initSync };` in wasm-bindgen JS".into());
    }

    let orig_undef = "    if (typeof module_or_path === 'undefined') {\n        module_or_path = new URL('lexlean_r4_bg.wasm', import.meta.url);\n    }";
    let patched_undef = "    const useManagedCache = (typeof module_or_path === 'undefined');\n    if (useManagedCache) {\n        module_or_path = new URL('lexlean_r4_bg.wasm', import.meta.url);\n    }";
    let mut out = if generated_js
        .contains("const useManagedCache = (typeof module_or_path === 'undefined');")
    {
        generated_js.to_string()
    } else if generated_js.contains(orig_undef) {
        generated_js.replacen(orig_undef, patched_undef, 1)
    } else {
        return Err("unable to locate default module_or_path initialization in __wbg_init".into());
    };

    let orig_fetch = "    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {\n        module_or_path = fetch(module_or_path);\n    }";
    let patched_fetch = "    if (useManagedCache) {\n        module_or_path = __uor_resolve_cached_wasm(module_or_path, false);\n    } else if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {\n        module_or_path = fetch(module_or_path);\n    }";
    if !out.contains("__uor_resolve_cached_wasm(module_or_path, false)") {
        if out.contains(orig_fetch) {
            out = out.replacen(orig_fetch, patched_fetch, 1);
        } else {
            return Err("unable to locate fetch(module_or_path) branch in __wbg_init".into());
        }
    }

    let trimmed_bridge = bridge_js.trim();
    if trimmed_bridge.is_empty() {
        return Err("worker_host_bridge.js is empty".into());
    }

    if let (Some(sec1), Some(sec2)) = (
        extract_marked_section(trimmed_bridge, SECTION_1_START, SECTION_1_END),
        extract_marked_section(trimmed_bridge, SECTION_2_START, SECTION_2_END),
    ) {
        out = out.replacen(init_fn_marker, &format!("{sec1}\n\n{init_fn_marker}"), 1);
        out = out.replacen(export_marker, &format!("{sec2}\n\n{export_marker}"), 1);
    } else {
        out = out.replacen(
            export_marker,
            &format!("{trimmed_bridge}\n\n{export_marker}"),
            1,
        );
    }

    Ok(out)
}

fn find_wasm_bindgen() -> Result<PathBuf, Fail> {
    if let Some(explicit) = std::env::var_os("WASM_BINDGEN") {
        let p = PathBuf::from(explicit);
        if p.is_file() {
            return Ok(p);
        }
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("wasm-bindgen");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        for cache_root in [
            home.join("Library/Caches/.wasm-pack"),
            home.join(".cache/.wasm-pack"),
        ] {
            if let Ok(entries) = std::fs::read_dir(cache_root) {
                let mut candidates: Vec<PathBuf> = entries
                    .flatten()
                    .map(|e| e.path().join("wasm-bindgen"))
                    .filter(|p| p.is_file())
                    .collect();
                candidates.sort();
                if let Some(found) = candidates.into_iter().next() {
                    return Ok(found);
                }
            }
        }
    }
    Err("wasm-bindgen CLI not found on PATH, WASM_BINDGEN, or .wasm-pack cache".into())
}

fn pinned_wasm_bindgen_version(root: &Path) -> Option<String> {
    let lock = std::fs::read_to_string(root.join("Cargo.lock")).ok()?;
    let mut in_pkg = false;
    for line in lock.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            in_pkg = false;
        } else if trimmed == "name = \"wasm-bindgen\"" {
            in_pkg = true;
        } else if in_pkg && trimmed.starts_with("version = \"") {
            return trimmed
                .strip_prefix("version = \"")
                .and_then(|s| s.strip_suffix('"'))
                .map(ToString::to_string);
        }
    }
    None
}

/// Build the `ProvenanceManifest` from on-disk inputs, preserving locked view identity hashes.
pub fn build_provenance_manifest(root: &Path, out_dir: &Path) -> Result<ProvenanceManifest, Fail> {
    let existing_path = if out_dir.join("provenance.json").exists() {
        out_dir.join("provenance.json")
    } else {
        root.join("public/provenance.json")
    };
    let existing: Option<ProvenanceManifest> = std::fs::read_to_string(&existing_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());

    let generated_core_sha256 = sha256_file(&root.join("crates/wasm/src/lexlean_generated.rs"))?;
    let adapter_wasm_sha256 = sha256_file(&out_dir.join("lexlean_r4_bg.wasm"))?;
    let binding_javascript_sha256 = sha256_file(&out_dir.join("lexlean_r4.js"))?;
    let adapter_lock_sha256 = sha256_file(&root.join("Cargo.lock"))?;

    let (model_id, view_manifest_sha256, view_model_id) = match existing {
        Some(prev) => (prev.model_id, prev.view_manifest_sha256, prev.view_model_id),
        None => (
            generated_core_sha256.clone(),
            adapter_lock_sha256.clone(),
            binding_javascript_sha256.clone(),
        ),
    };

    Ok(ProvenanceManifest {
        schema: PROVENANCE_SCHEMA.to_string(),
        model_id,
        view_manifest_sha256,
        view_model_id,
        generated_core_sha256,
        adapter_wasm_sha256,
        binding_javascript_sha256,
        adapter_lock_sha256,
    })
}

/// True when a `wasm-bindgen` binary matching `Cargo.lock`'s pinned version is available.
#[allow(dead_code)]
pub fn has_matching_wasm_bindgen(root: &Path) -> bool {
    let Ok(bindgen) = find_wasm_bindgen() else {
        return false;
    };
    let Some(expected_ver) = pinned_wasm_bindgen_version(root) else {
        return true;
    };
    Command::new(&bindgen)
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&expected_ver))
        .unwrap_or(false)
}

/// Compile `crates/wasm`, splice `worker_host_bridge.js`, and emit `lexlean_r4.js`,
/// `lexlean_r4_bg.wasm`, and `provenance.json` into `out_dir`.
pub fn build_wasm(root: &Path, out_dir: &Path) -> Result<ProvenanceManifest, Fail> {
    let bridge_path = root.join("crates/wasm/js/worker_host_bridge.js");
    let bridge_js = std::fs::read_to_string(&bridge_path).map_err(|e| {
        format!(
            "missing {} (run after host bridge extraction lands): {e}",
            bridge_path.display()
        )
    })?;

    let bindgen = find_wasm_bindgen()?;
    if let Some(expected_ver) = pinned_wasm_bindgen_version(root) {
        let ver_out = Command::new(&bindgen).arg("--version").output()?;
        let actual = String::from_utf8_lossy(&ver_out.stdout).trim().to_string();
        if !actual.contains(&expected_ver) {
            return Err(format!(
                "wasm-bindgen CLI version mismatch: Cargo.lock pins {expected_ver}, found `{actual}` at {}",
                bindgen.display()
            )
            .into());
        }
    }

    let mut rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
    if !rustflags.contains("simd128") {
        if !rustflags.is_empty() {
            rustflags.push(' ');
        }
        rustflags.push_str("-C target-feature=+simd128");
    }

    let status = Command::new("cargo")
        .current_dir(root)
        .env("RUSTFLAGS", &rustflags)
        .args([
            "build",
            "-p",
            "lexlean-r4-wasm",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--locked",
        ])
        .status()?;
    if !status.success() {
        return Err(
            "cargo build -p lexlean-r4-wasm --target wasm32-unknown-unknown --release failed"
                .into(),
        );
    }

    let staging = out_dir.join(".wasm-bindgen-staging");
    std::fs::create_dir_all(&staging)?;
    let wasm_artifact = root.join("target/wasm32-unknown-unknown/release/lexlean_r4_wasm.wasm");
    let bg_status = Command::new(&bindgen)
        .current_dir(root)
        .arg(&wasm_artifact)
        .args(["--out-dir"])
        .arg(&staging)
        .args([
            "--out-name",
            "lexlean_r4",
            "--target",
            "web",
            "--no-typescript",
        ])
        .status()?;
    if !bg_status.success() {
        return Err("wasm-bindgen execution failed".into());
    }

    let raw_js = std::fs::read_to_string(staging.join("lexlean_r4.js"))?;
    let final_js = transform_wasm_bindgen_js(&raw_js, &bridge_js)?;
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join("lexlean_r4.js"), final_js)?;
    std::fs::copy(
        staging.join("lexlean_r4_bg.wasm"),
        out_dir.join("lexlean_r4_bg.wasm"),
    )?;
    let _ = std::fs::remove_dir_all(&staging);

    let manifest = build_provenance_manifest(root, out_dir)?;
    let encoded = format!("{}\n", serde_json::to_string_pretty(&manifest)?);
    std::fs::write(out_dir.join("provenance.json"), encoded)?;
    Ok(manifest)
}

/// Verify `dir/provenance.json` matches on-disk files in `root` and `dir`.
pub fn check_wasm_sync_in(root: &Path, dir: &Path) -> Result<ProvenanceManifest, Fail> {
    let prov_path = dir.join("provenance.json");
    let raw = std::fs::read_to_string(&prov_path)
        .map_err(|e| format!("reading {}: {e}", prov_path.display()))?;
    let manifest: ProvenanceManifest = serde_json::from_str(&raw)?;
    if manifest.schema != PROVENANCE_SCHEMA {
        return Err(format!(
            "provenance schema mismatch: expected {PROVENANCE_SCHEMA}, got {}",
            manifest.schema
        )
        .into());
    }

    let expected_lock = sha256_file(&root.join("Cargo.lock"))?;
    let expected_wasm = sha256_file(&dir.join("lexlean_r4_bg.wasm"))?;
    let expected_js = sha256_file(&dir.join("lexlean_r4.js"))?;
    let expected_core = sha256_file(&root.join("crates/wasm/src/lexlean_generated.rs"))?;

    let mut mismatches = Vec::new();
    if manifest.adapter_lock_sha256 != expected_lock {
        mismatches.push(format!(
            "adapter_lock_sha256 drifted: manifest={} actual={expected_lock}",
            manifest.adapter_lock_sha256
        ));
    }
    if manifest.adapter_wasm_sha256 != expected_wasm {
        mismatches.push(format!(
            "adapter_wasm_sha256 drifted: manifest={} actual={expected_wasm}",
            manifest.adapter_wasm_sha256
        ));
    }
    if manifest.binding_javascript_sha256 != expected_js {
        mismatches.push(format!(
            "binding_javascript_sha256 drifted: manifest={} actual={expected_js}",
            manifest.binding_javascript_sha256
        ));
    }
    if manifest.generated_core_sha256 != expected_core {
        mismatches.push(format!(
            "generated_core_sha256 drifted: manifest={} actual={expected_core}",
            manifest.generated_core_sha256
        ));
    }

    let js_text = std::fs::read_to_string(dir.join("lexlean_r4.js"))?;
    if !js_text.contains("__uor_resolve_cached_wasm(module_or_path, false)") {
        mismatches.push("lexlean_r4.js missing __uor_resolve_cached_wasm hook".to_string());
    }

    if !mismatches.is_empty() {
        return Err(format!("check-wasm-sync failed:\n  - {}", mismatches.join("\n  - ")).into());
    }
    Ok(manifest)
}

/// CLI entry for `build-wasm [--out-dir <path>]`.
pub fn run_build_wasm_cmd(root: &Path, args: &[String]) -> Result<(), Fail> {
    let mut out_dir = root.join("public");
    let mut idx = 0;
    while idx < args.len() {
        if args[idx] == "--out-dir" {
            let val = args
                .get(idx + 1)
                .ok_or("--out-dir requires a directory argument")?;
            let p = PathBuf::from(val);
            out_dir = if p.is_absolute() { p } else { root.join(p) };
            idx += 2;
        } else {
            return Err(format!("unknown build-wasm argument: {}", args[idx]).into());
        }
    }
    let manifest = build_wasm(root, &out_dir)?;
    println!(
        "build-wasm: wrote {} (wasm={} js={} core={})",
        out_dir.display(),
        &manifest.adapter_wasm_sha256[..12],
        &manifest.binding_javascript_sha256[..12],
        &manifest.generated_core_sha256[..12]
    );
    Ok(())
}

/// CLI entry for `check-wasm-sync`.
pub fn run_check_wasm_sync_cmd(root: &Path) -> Result<(), Fail> {
    let manifest = check_wasm_sync_in(root, &root.join("public"))?;
    println!(
        "check-wasm-sync: public/ provenance verified (core={} wasm={} js={})",
        &manifest.generated_core_sha256[..12],
        &manifest.adapter_wasm_sha256[..12],
        &manifest.binding_javascript_sha256[..12]
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_generated_js() -> &'static str {
        concat!(
            "function __wbg_get_imports() { return {}; }\n",
            "async function __wbg_init(module_or_path) {\n",
            "    if (typeof module_or_path === 'undefined') {\n",
            "        module_or_path = new URL('lexlean_r4_bg.wasm', import.meta.url);\n",
            "    }\n",
            "    const imports = __wbg_get_imports();\n",
            "    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {\n",
            "        module_or_path = fetch(module_or_path);\n",
            "    }\n",
            "    return imports;\n",
            "}\n",
            "export { initSync };\n",
            "export default __wbg_init;\n",
        )
    }

    #[test]
    fn splices_custom_sections_and_cache_hook() {
        let bridge = format!(
            "{SECTION_1_START}\nasync function __uor_resolve_cached_wasm(u, r) {{ return fetch(u); }}\n{SECTION_1_END}\n\
             {SECTION_2_START}\nfunction __uorStartDedicatedEngineWorker() {{}}\n{SECTION_2_END}\n"
        );
        let spliced = transform_wasm_bindgen_js(sample_generated_js(), &bridge).expect("splices");
        assert!(
            spliced.contains("const useManagedCache = (typeof module_or_path === 'undefined');")
        );
        assert!(spliced.contains("__uor_resolve_cached_wasm(module_or_path, false)"));
        let sec1_pos = spliced.find(SECTION_1_START).expect("sec1");
        let init_pos = spliced.find("async function __wbg_init").expect("init");
        let sec2_pos = spliced.find(SECTION_2_START).expect("sec2");
        let exp_pos = spliced.find("export { initSync };").expect("export");
        assert!(sec1_pos < init_pos);
        assert!(init_pos < sec2_pos);
        assert!(sec2_pos < exp_pos);
    }

    #[test]
    fn refuses_missing_init_or_export_anchor() {
        let err = transform_wasm_bindgen_js("export { initSync };", "// bridge").unwrap_err();
        assert!(err.contains("__wbg_init"));
        let err2 =
            transform_wasm_bindgen_js("async function __wbg_init(module_or_path) {}", "// bridge")
                .unwrap_err();
        assert!(err2.contains("export { initSync }"));
    }

    #[test]
    fn detects_provenance_sha256_drift() {
        let tmp = std::env::temp_dir().join(format!("uor-wasm-bundle-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let fake_root = tmp.join("repo");
        let pub_dir = fake_root.join("public");
        std::fs::create_dir_all(fake_root.join("crates/wasm/src")).unwrap();
        std::fs::create_dir_all(&pub_dir).unwrap();
        std::fs::write(fake_root.join("Cargo.lock"), b"lock-v1").unwrap();
        std::fs::write(
            fake_root.join("crates/wasm/src/lexlean_generated.rs"),
            b"// generated v1",
        )
        .unwrap();
        let js = transform_wasm_bindgen_js(sample_generated_js(), "const bridge = true;").unwrap();
        std::fs::write(pub_dir.join("lexlean_r4.js"), &js).unwrap();
        std::fs::write(pub_dir.join("lexlean_r4_bg.wasm"), b"\0asm\x01\0\0\0").unwrap();

        let manifest = build_provenance_manifest(&fake_root, &pub_dir).unwrap();
        std::fs::write(
            pub_dir.join("provenance.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(check_wasm_sync_in(&fake_root, &pub_dir).is_ok());

        // Plant drift in lexlean_generated.rs and prove check rejects it.
        std::fs::write(
            fake_root.join("crates/wasm/src/lexlean_generated.rs"),
            b"// modified core",
        )
        .unwrap();
        let err = check_wasm_sync_in(&fake_root, &pub_dir)
            .unwrap_err()
            .to_string();
        assert!(err.contains("generated_core_sha256 drifted"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
