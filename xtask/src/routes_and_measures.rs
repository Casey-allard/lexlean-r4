//! UORust total route table (`model/routes.toml`) and Prism-DevRel ISO 15939 / PROV-O gates (`model/measures.toml`, `model/provenance.toml`, `R4-53`).

use std::collections::BTreeSet;
use std::path::Path;

use repo_model::serde::Deserialize;
use repo_model::{toml, Model};

use crate::engine::lift_register;
use crate::lexlean_generated::r4_uorust_prism_devrel::{
    admit_route_and_measures, dense_matmul_ops, evaluate_measure_criterion, float_serving_ops,
    kappa_residue, route_table_total_count, uncovered_records, undeclared_references,
    MeasureDirection, RouteAdmissionVerdict, RouteAndMeasureCheck,
};

type Fail = Box<dyn std::error::Error>;

const VALID_FACETS: &[&str] = &[
    "arch", "features", "behavior", "design", "sec", "qual", "release",
];

#[derive(Debug, Deserialize)]
#[serde(crate = "repo_model::serde")]
struct RoutesFile {
    spec: String,
    max_request_bytes: usize,
    route: Vec<RouteEntry>,
    vector: Vec<RouteVector>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "repo_model::serde")]
struct RouteEntry {
    id: String,
    rust_fact: String,
    prism_record: String,
    facet: String,
    note: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "repo_model::serde")]
struct RouteVector {
    case: String,
    request_hex: String,
    response: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "repo_model::serde")]
struct MeasuresFile {
    spec: String,
    measure: Vec<MeasureEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "repo_model::serde")]
struct MeasureEntry {
    record: String,
    method: String,
    source: String,
    direction: String,
    threshold: u64,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "repo_model::serde")]
struct ProvenanceFile {
    spec: String,
    sources: Vec<String>,
}

/// Formal R4.UorustPrismDevrel decode lowercase/uppercase hexadecimal bytes strictly without allocations beyond output.
fn decode_hex_bytes(hex_str: &str) -> Result<Vec<u8>, Fail> {
    let s = hex_str.trim();
    if !s.len().is_multiple_of(2) {
        return Err(format!("odd-length request_hex `{hex_str}`").into());
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let raw = s.as_bytes();
    for pair in raw.chunks_exact(2) {
        let hi = (pair[0] as char)
            .to_digit(16)
            .ok_or_else(|| format!("invalid hex byte in `{hex_str}`"))?;
        let lo = (pair[1] as char)
            .to_digit(16)
            .ok_or_else(|| format!("invalid hex byte in `{hex_str}`"))?;
        out.push(((hi << 4) | lo) as u8);
    }
    Ok(out)
}

/// Formal R4.UorustPrismDevrel map string direction from `model/measures.toml` onto `MeasureDirection`.
fn parse_measure_direction(raw: &str) -> MeasureDirection {
    match raw.trim() {
        "atMost" => MeasureDirection::AtMost,
        "atLeast" => MeasureDirection::AtLeast,
        _ => MeasureDirection::InvalidDirection,
    }
}

/// Formal R4.UorustPrismDevrel check whether an identifier token appears at ASCII word boundaries.
fn contains_ascii_word(haystack: &str, word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let bytes = haystack.as_bytes();
    let mut start = 0usize;
    while let Some(rel) = haystack[start..].find(word) {
        let idx = start + rel;
        let end = idx + word.len();
        let left_ok =
            idx == 0 || !(bytes[idx - 1].is_ascii_alphanumeric() || bytes[idx - 1] == b'_');
        let right_ok =
            end == bytes.len() || !(bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_');
        if left_ok && right_ok {
            return true;
        }
        start = idx + 1;
    }
    false
}

/// Formal R4.UorustPrismDevrel verify `model/routes.toml` total route table and deterministic vectors.
pub fn check_routes(root: &Path) -> Result<(), Fail> {
    let routes_path = root.join("model/routes.toml");
    let raw = std::fs::read_to_string(&routes_path)
        .map_err(|e| format!("reading {}: {e}", routes_path.display()))?;
    let doc: RoutesFile =
        toml::from_str(&raw).map_err(|e| format!("parsing {}: {e}", routes_path.display()))?;

    if doc.spec != "uorust/routes/1" {
        return Err(format!("model/routes.toml spec `{}` != `uorust/routes/1`", doc.spec).into());
    }
    if route_table_total_count(doc.route.len() as u64) < 9 {
        return Err(format!(
            "RefusedRouteTableTooSmall: model/routes.toml declares {} routes (< 9)",
            doc.route.len()
        )
        .into());
    }

    let mut seen_ids = BTreeSet::new();
    let mut seen_facts = BTreeSet::new();
    for route in &doc.route {
        if route.id.trim().is_empty()
            || route.rust_fact.trim().is_empty()
            || route.prism_record.trim().is_empty()
            || route.note.trim().is_empty()
        {
            return Err(format!("route entry {:?} contains empty required field", route.id).into());
        }
        if !VALID_FACETS.contains(&route.facet.as_str()) {
            return Err(format!(
                "route `{}` uses unknown Prism facet `{}`",
                route.id, route.facet
            )
            .into());
        }
        if !seen_ids.insert(route.id.clone()) {
            return Err(format!(
                "{:?}: duplicate route id `{}` in model/routes.toml",
                RouteAdmissionVerdict::RefusedAmbiguousRoute,
                route.id
            )
            .into());
        }
        if !seen_facts.insert(route.rust_fact.clone()) {
            return Err(format!(
                "{:?}: duplicate rust_fact `{}` in model/routes.toml",
                RouteAdmissionVerdict::RefusedAmbiguousRoute,
                route.rust_fact
            )
            .into());
        }
    }

    for target_rel in [
        "crates/wasm/src/holo_kappa_store.rs",
        "crates/wasm/src/holo_geometric_runtime.rs",
        "crates/wasm/src/native_q31_lane.rs",
    ] {
        let src = std::fs::read_to_string(root.join(target_rel))
            .map_err(|e| format!("reading routed target {target_rel}: {e}"))?;
        if !src.contains("Formal R4.") {
            return Err(format!(
                "routed target `{target_rel}` missing formal `Formal R4.` docstrings"
            )
            .into());
        }
    }

    if doc.vector.is_empty() {
        return Err("model/routes.toml must declare conformance test vectors".into());
    }
    for vec in &doc.vector {
        let req_bytes = decode_hex_bytes(&vec.request_hex)?;
        if req_bytes.len() > doc.max_request_bytes {
            return Err(format!(
                "route vector `{}` request length {} exceeds max_request_bytes {}",
                vec.case,
                req_bytes.len(),
                doc.max_request_bytes
            )
            .into());
        }
        let req_text = std::str::from_utf8(&req_bytes)
            .map_err(|e| format!("route vector `{}` invalid UTF-8: {e}", vec.case))?;
        let facts: Vec<&str> = req_text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();

        let rendered = if facts.is_empty() {
            "error empty".to_string()
        } else {
            let mut matched: Vec<&RouteEntry> = Vec::new();
            let mut unknown: Option<&str> = None;
            for fact in &facts {
                let hits: Vec<&RouteEntry> =
                    doc.route.iter().filter(|r| r.rust_fact == *fact).collect();
                if hits.is_empty() && unknown.is_none() {
                    unknown = Some(*fact);
                }
                matched.extend(hits);
            }
            let verdict = admit_route_and_measures(RouteAndMeasureCheck {
                known_fact_match: unknown.is_none(),
                matched_route_count: matched.len() as u64,
                direction: MeasureDirection::AtLeast,
                measured_value: 9,
                threshold_value: 9,
                missing_provenance_sources: 0,
            });
            match verdict {
                RouteAdmissionVerdict::RouteAdmitted => {
                    let r = matched[0];
                    format!(
                        "route {}\nfact {}\nrecord {}\nfacet {}",
                        r.id, r.rust_fact, r.prism_record, r.facet
                    )
                }
                RouteAdmissionVerdict::RefusedUnknownRouteFact => {
                    format!("refused unknown-fact\nfact {}", unknown.unwrap_or(""))
                }
                RouteAdmissionVerdict::RefusedAmbiguousRoute => {
                    let mut lines = vec!["refused ambiguous".to_string()];
                    for r in matched {
                        lines.push(format!("fact {}", r.rust_fact));
                    }
                    lines.join("\n")
                }
                other => {
                    return Err(format!(
                        "unexpected verdict {other:?} on route vector `{}`",
                        vec.case
                    )
                    .into());
                }
            }
        };

        if rendered != vec.response {
            return Err(format!(
                "route vector `{}` mismatch:\nexpected:\n{}\ngot:\n{}",
                vec.case, vec.response, rendered
            )
            .into());
        }
    }

    println!(
        "check-routes: {} routes and {} vectors verified (uorust/routes/1)",
        doc.route.len(),
        doc.vector.len()
    );
    Ok(())
}

/// Formal R4.UorustPrismDevrel resolve provenance source paths or directory suffix globs on disk.
fn count_missing_provenance_sources(root: &Path, sources: &[String]) -> (u64, Vec<String>) {
    let mut missing = Vec::new();
    for entry in sources {
        if let Some((dir_part, glob_part)) = entry.rsplit_once('/') {
            if let Some(ext) = glob_part.strip_prefix('*') {
                let dir = root.join(dir_part);
                let found = std::fs::read_dir(&dir)
                    .ok()
                    .into_iter()
                    .flatten()
                    .filter_map(Result::ok)
                    .any(|de| {
                        de.path().is_file()
                            && de
                                .file_name()
                                .to_str()
                                .is_some_and(|name| name.ends_with(ext))
                    });
                if !found {
                    missing.push(entry.clone());
                }
                continue;
            }
        }
        if !root.join(entry).is_file() {
            missing.push(entry.clone());
        }
    }
    (missing.len() as u64, missing)
}

/// Formal R4.UorustPrismDevrel count any undeclared `R4-XX` or `R4-URXX` identifiers referenced inside `CONFORMANCE.md`.
fn count_undeclared_conformance_refs(conformance: &str, declared_ids: &BTreeSet<String>) -> u64 {
    let bytes = conformance.as_bytes();
    let mut undeclared = 0u64;
    let mut i = 0usize;
    while i + 4 <= bytes.len() {
        let prev_ok = i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'-');
        if prev_ok && &bytes[i..i + 3] == b"R4-" {
            let mut cursor = i + 3;
            if bytes[cursor..].starts_with(b"UR") {
                cursor += 2;
            }
            let digit_start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            let next_ok = cursor == bytes.len()
                || (!bytes[cursor].is_ascii_alphanumeric() && bytes[cursor] != b'-');
            if cursor > digit_start && next_ok {
                let id_str = &conformance[i..cursor];
                if !declared_ids.contains(id_str) {
                    undeclared += 1;
                }
            }
            i = cursor;
        } else {
            i += 1;
        }
    }
    undeclared
}

/// Formal R4.UorustPrismDevrel count any disallowed floating-point or `.softmax(` ops in serving code or RTX 4090 receipts.
fn count_serving_float_and_matmul_ops(root: &Path) -> Result<(u64, u64), Fail> {
    let mut float_ops = 0u64;
    let mut matmul_ops = 0u64;

    if let Ok(holo_src) =
        std::fs::read_to_string(root.join("crates/wasm/src/holo_geometric_runtime.rs"))
    {
        // Scan entire production runtime cleanly for any float or matmul ops.
        let serving_slice = holo_src.split("#[cfg(test)]").next().unwrap_or(&holo_src);
        for line in serving_slice.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue;
            }
            if contains_ascii_word(trimmed, "f32")
                || contains_ascii_word(trimmed, "f64")
                || trimmed.contains(".softmax(")
                || trimmed.contains("expf(")
            {
                float_ops += 1;
            }
        }
    }

    if let Ok(q31_src) = std::fs::read_to_string(root.join("crates/wasm/src/native_q31_lane.rs")) {
        let serving_slice = q31_src.split("#[cfg(test)]").next().unwrap_or(&q31_src);
        for line in serving_slice.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue;
            }
            if contains_ascii_word(trimmed, "f32")
                || contains_ascii_word(trimmed, "f64")
                || trimmed.contains(".softmax(")
                || trimmed.contains("expf(")
            {
                float_ops += 1;
            }
        }
    }

    for receipt_rel in [
        "gpu/rtx4090_holo_v13_qwen3_0_6b_receipt.json",
        "gpu/rtx4090_holo_v13_qwen3_1_7b_receipt.json",
    ] {
        let raw = std::fs::read_to_string(root.join(receipt_rel))
            .map_err(|e| format!("reading {receipt_rel}: {e}"))?;
        let val: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| format!("parsing {receipt_rel}: {e}"))?;
        float_ops = float_ops.saturating_add(val["float_ops"].as_u64().unwrap_or(1));
        matmul_ops = matmul_ops.saturating_add(val["dense_matmul_ops"].as_u64().unwrap_or(1));
    }

    Ok((float_serving_ops(float_ops), dense_matmul_ops(matmul_ops)))
}

/// Formal R4.UorustPrismDevrel collect every `@ID` scenario tag declared in `features/suites/*.feature`.
fn collect_feature_scenario_ids(root: &Path) -> Result<BTreeSet<String>, Fail> {
    let mut ids = BTreeSet::new();
    let dir = root.join("features/suites");
    if !dir.is_dir() {
        return Ok(ids);
    }
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "feature") {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("reading {}: {e}", path.display()))?;
        for raw in text.lines() {
            let trimmed = raw.trim();
            if let Some(rest) = trimmed.strip_prefix('@') {
                if let Some(first) = rest.split_whitespace().next() {
                    let clean = first.trim_start_matches('@');
                    if !clean.is_empty() {
                        ids.insert(clean.to_string());
                    }
                }
            }
        }
    }
    Ok(ids)
}

/// Formal R4.UorustPrismDevrel enforce Prism-DevRel ISO 15939 measurement thresholds and PROV-O provenance closure.
pub fn check_measures(root: &Path) -> Result<(), Fail> {
    let measures_path = root.join("model/measures.toml");
    let measures_raw = std::fs::read_to_string(&measures_path)
        .map_err(|e| format!("reading {}: {e}", measures_path.display()))?;
    let measures_doc: MeasuresFile = toml::from_str(&measures_raw)
        .map_err(|e| format!("parsing {}: {e}", measures_path.display()))?;
    if measures_doc.spec != "prism/measures/1" {
        return Err(format!(
            "model/measures.toml spec `{}` != `prism/measures/1`",
            measures_doc.spec
        )
        .into());
    }

    let prov_path = root.join("model/provenance.toml");
    let prov_raw = std::fs::read_to_string(&prov_path)
        .map_err(|e| format!("reading {}: {e}", prov_path.display()))?;
    let prov_doc: ProvenanceFile =
        toml::from_str(&prov_raw).map_err(|e| format!("parsing {}: {e}", prov_path.display()))?;
    if prov_doc.spec != "prism/provenance-sources/1" {
        return Err(format!(
            "model/provenance.toml spec `{}` != `prism/provenance-sources/1`",
            prov_doc.spec
        )
        .into());
    }

    let (missing_prov_count, missing_prov_paths) =
        count_missing_provenance_sources(root, &prov_doc.sources);

    let routes_raw = std::fs::read_to_string(root.join("model/routes.toml"))?;
    let routes_doc: RoutesFile = toml::from_str(&routes_raw)?;
    let route_count = route_table_total_count(routes_doc.route.len() as u64);

    let model = Model::load(&root.join("model"))?;
    model.check()?;

    let mut declared_ids: BTreeSet<String> = model.ids.id.iter().map(|r| r.id.clone()).collect();
    for r in &routes_doc.route {
        declared_ids.insert(r.id.clone());
    }

    let ledger_ids: BTreeSet<String> = model.ledger.claim.iter().map(|c| c.id.clone()).collect();
    let scenario_ids = collect_feature_scenario_ids(root)?;
    let covered_count = model
        .ids
        .id
        .iter()
        .filter(|r| ledger_ids.contains(&r.id) || scenario_ids.contains(&r.id))
        .count() as u64;
    let uncovered = uncovered_records(model.ids.id.len() as u64, covered_count);

    let conformance_raw = std::fs::read_to_string(root.join("CONFORMANCE.md")).unwrap_or_default();
    let undeclared = undeclared_references(
        count_undeclared_conformance_refs(&conformance_raw, &declared_ids),
        0,
    );

    let (measured_float_ops, measured_matmul_ops) = count_serving_float_and_matmul_ops(root)?;
    let lift_outcome = lift_register::scan_codebase_lift_register(root)?;
    let measured_residue = kappa_residue(lift_outcome.document.summary.residue);

    for m in &measures_doc.measure {
        if !root.join(&m.source).is_file() {
            return Err(format!(
                "measure `{}` references missing model source `{}`",
                m.record, m.source
            )
            .into());
        }
        let dir = parse_measure_direction(&m.direction);
        let measured = match m.method.as_str() {
            "R4.UorustPrismDevrel.uncoveredRecords" => uncovered,
            "R4.UorustPrismDevrel.undeclaredReferences" => undeclared,
            "R4.UorustPrismDevrel.floatServingOps" => measured_float_ops,
            "R4.UorustPrismDevrel.denseMatmulOps" => measured_matmul_ops,
            "R4.UorustPrismDevrel.kappaResidue" => measured_residue,
            "R4.UorustPrismDevrel.routeTableTotalCount" => route_count,
            other => {
                return Err(
                    format!("measure `{}` names unknown method `{other}`", m.record).into(),
                );
            }
        };

        let crit_ok = evaluate_measure_criterion(dir, measured, m.threshold);
        let verdict = admit_route_and_measures(RouteAndMeasureCheck {
            known_fact_match: true,
            matched_route_count: 1,
            direction: dir,
            measured_value: measured,
            threshold_value: m.threshold,
            missing_provenance_sources: missing_prov_count,
        });
        if !crit_ok || verdict != RouteAdmissionVerdict::RouteAdmitted {
            return Err(format!(
                "check-measures failed on `{}` ({}): verdict={verdict:?}, measured={measured}, threshold={}, missing_provenance={missing_prov_paths:?}",
                m.record, m.method, m.threshold
            )
            .into());
        }
    }

    println!(
        "check-measures: {} measures admitted (uncovered={uncovered}, undeclared={undeclared}, float_ops={measured_float_ops}, matmul_ops={measured_matmul_ops}, kappa_residue={measured_residue}, routes={route_count}, provenance_sources={})",
        measures_doc.measure.len(),
        prov_doc.sources.len()
    );
    Ok(())
}
