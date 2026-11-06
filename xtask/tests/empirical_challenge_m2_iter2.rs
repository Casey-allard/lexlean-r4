//! Empirical challenge test suite for Milestone 2 Iteration 2.
//!
//! Focus:
//! 1. CDATA & Inequality Fuzzing: Adversarially fuzz `strip_html_tags` and `parse_arxiv_atom_feed`
//!    with CDATA sections containing nested `<...>`, unbalanced tags, angle-bracket math (`a < b and c > d`),
//!    ampersands, and XML entities.
//! 2. Network Resilience & Fallback: Test `literature_sync` with mocked HTML error payloads
//!    (e.g. `<html><body>502 Bad Gateway</body></html>`) under `offline_fallback = true` vs `false`.
//! 3. Dry-run immutability of catalog under fallback and sync operations.

#![allow(dead_code, unused_imports, clippy::all)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/lut_stack_exec.rs"]
pub mod lut_stack_exec;

#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/otel.rs"]
pub mod otel;

#[allow(dead_code, clippy::all)]
#[path = "../src/engine.rs"]
mod engine;

use engine::{
    extract_tag_content, literature_sync, load_literature, normalize_whitespace,
    parse_arxiv_atom_feed, strip_html_tags, unescape_xml, ArxivPaper, LiteratureFile,
};

// ============================================================================
// 1. CDATA & Inequality Fuzzing
// ============================================================================

#[test]
fn challenge_fuzz_strip_html_tags_cdata_nested_tags() {
    // CDATA containing nested HTML/XML markup
    let input = "Plain text <![CDATA[<nested><tag attr=\"val\">inner</tag></nested>]]> trailing";
    let stripped = strip_html_tags(input);
    assert_eq!(
        stripped, "Plain text <nested><tag attr=\"val\">inner</tag></nested> trailing",
        "Nested tags inside CDATA must be preserved verbatim"
    );
}

#[test]
fn challenge_fuzz_strip_html_tags_cdata_angle_bracket_math() {
    // Angle-bracket math expressions inside CDATA
    let input = "Theorem: <![CDATA[for all 0 < x < 1 and y > 2, f(x, y) <= g(x)]]>.";
    let stripped = strip_html_tags(input);
    assert_eq!(
        stripped, "Theorem: for all 0 < x < 1 and y > 2, f(x, y) <= g(x).",
        "Inequality symbols inside CDATA must be fully preserved"
    );
}

#[test]
fn challenge_fuzz_strip_html_tags_cdata_unbalanced_tags() {
    // Unbalanced angle brackets inside CDATA
    let input = "<![CDATA[a < b < c and d > > e or <tag with no close]]>";
    let stripped = strip_html_tags(input);
    assert_eq!(
        stripped, "a < b < c and d > > e or <tag with no close",
        "Unbalanced angle brackets in CDATA must not trigger premature termination"
    );
}

#[test]
fn challenge_fuzz_strip_html_tags_cdata_preserves_raw_inner_tokens() {
    // CDATA with ampersands, XML entities, quotes, brackets
    let input = "<![CDATA[AT&T & R&D &amp; &#65; &#x42;]]>";
    let stripped = strip_html_tags(input);
    assert_eq!(
        stripped, "AT&T & R&D &amp; &#65; &#x42;",
        "strip_html_tags must preserve all characters inside CDATA without alteration"
    );
}

#[test]
fn challenge_fuzz_strip_html_tags_multiple_cdata_blocks() {
    let input = "Start <![CDATA[Part 1: a < b]]> Middle <![CDATA[Part 2: c > d]]> End";
    let stripped = strip_html_tags(input);
    assert_eq!(stripped, "Start Part 1: a < b Middle Part 2: c > d End");
}

#[test]
fn challenge_fuzz_strip_html_tags_empty_cdata() {
    let input = "Before<![CDATA[]]>After";
    let stripped = strip_html_tags(input);
    assert_eq!(stripped, "BeforeAfter");
}

#[test]
fn challenge_fuzz_strip_html_tags_unclosed_cdata() {
    // Unclosed CDATA should preserve the rest of string rather than panicking or dropping
    let input = "Prefix <![CDATA[This CDATA has no closing marker a < b";
    let stripped = strip_html_tags(input);
    assert_eq!(
        stripped, "Prefix This CDATA has no closing marker a < b",
        "Unclosed CDATA should append remaining text without panic"
    );
}

#[test]
fn challenge_fuzz_strip_html_tags_cdata_with_brackets_inside() {
    // CDATA containing ']' or ']]' without '>'
    let input = "<![CDATA[Array [0] and nested [[index]] value]]>";
    let stripped = strip_html_tags(input);
    assert_eq!(stripped, "Array [0] and nested [[index]] value");
}

#[test]
fn challenge_fuzz_strip_html_tags_interleaved_html_and_cdata() {
    let input = "<div><p>Title: <![CDATA[1 < 2 & 3 > 2]]></p><span>Description</span></div>";
    let stripped = strip_html_tags(input);
    assert_eq!(stripped, "Title: 1 < 2 & 3 > 2Description");
}

#[test]
fn challenge_fuzz_atom_feed_cdata_full_spectrum() {
    // Fuzz parse_arxiv_atom_feed with an adversarial feed combining:
    // - CDATA title with nested tags and math: a < b and c > d
    // - CDATA summary with entities, ampersands, and brackets
    // - CDATA author name
    // - Multiple entries
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>arXiv Search Feed</title>
  <entry>
    <id>http://arxiv.org/abs/2403.99901v1</id>
    <title><![CDATA[Non-Commutative Cohomology for 0 < k < n & Complex Invariants]]></title>
    <published>2024-03-20T12:00:00Z</published>
    <updated>2024-03-21T15:00:00Z</updated>
    <summary><![CDATA[We prove that whenever <formula>a < b and c > d</formula>, the quotient space <Q> is isomorphic to <b>R</b> & <b>C</b>. Furthermore, &#945; < &#946;.]]></summary>
    <author>
      <name><![CDATA[Alice & Bob <Researchers>]]></name>
    </author>
    <category term="cs.LO" scheme="http://arxiv.org/schemas/atom"/>
    <category term="math.LO" scheme="http://arxiv.org/schemas/atom"/>
  </entry>
  <entry>
    <id>http://arxiv.org/abs/2403.99902v2</id>
    <title><![CDATA[Deep Learning on Manifolds: [[L1 vs L2]] Optimization]]></title>
    <published>2024-03-22T08:00:00Z</published>
    <summary>Standard summary with XML entities: &lt;tag&gt; &amp; &quot;quotes&quot; and 1 &lt; 2.</summary>
    <author>
      <name>Carol</name>
    </author>
    <category term="cs.LG" scheme="http://arxiv.org/schemas/atom"/>
  </entry>
</feed>"#;

    let res = parse_arxiv_atom_feed(xml);
    assert!(res.is_ok(), "Feed parsing failed: {:?}", res.err());
    let papers = res.unwrap();
    assert_eq!(papers.len(), 2);

    // Paper 1 checks
    let p1 = &papers[0];
    assert_eq!(p1.arxiv_id, "2403.99901");
    assert_eq!(
        p1.title,
        "Non-Commutative Cohomology for 0 < k < n & Complex Invariants"
    );
    assert!(
        p1.r#abstract.contains("a < b and c > d"),
        "Abstract math must be preserved: got '{}'",
        p1.r#abstract
    );
    assert!(
        p1.r#abstract.contains("α < β"),
        "Greek entities must be unescaped: got '{}'",
        p1.r#abstract
    );
    assert_eq!(p1.authors, vec!["Alice & Bob <Researchers>".to_string()]);
    assert_eq!(p1.primary_category, "cs.LO");
    assert_eq!(
        p1.categories,
        vec!["cs.LO".to_string(), "math.LO".to_string()]
    );

    // Paper 2 checks
    let p2 = &papers[1];
    assert_eq!(p2.arxiv_id, "2403.99902");
    assert_eq!(
        p2.title,
        "Deep Learning on Manifolds: [[L1 vs L2]] Optimization"
    );
    assert!(
        p2.r#abstract.contains("<tag> & \"quotes\" and 1 < 2"),
        "Standard entity decoding must succeed: got '{}'",
        p2.r#abstract
    );
    assert_eq!(p2.primary_category, "cs.LG");
}

// ============================================================================
// 2. Network Resilience & Fallback Tests
// ============================================================================

#[test]
fn challenge_parse_arxiv_atom_feed_rejects_html_error_payloads() {
    let html_payloads = [
        "<html><body>502 Bad Gateway</body></html>",
        "<!DOCTYPE html><html><head><title>503 Service Temporarily Unavailable</title></head><body><h1>503</h1></body></html>",
        "<html><head><title>404 Not Found</title></head><body>nginx</body></html>",
        "<html><body>Cloudflare: Attention Required! | Cloudflare</body></html>",
        "   <!DOCTYPE html>\n<html><body>Access Denied</body></html>   ",
    ];

    for payload in html_payloads {
        let res = parse_arxiv_atom_feed(payload);
        assert!(
            res.is_err(),
            "HTML error payload should be rejected as non-Atom XML: {payload}"
        );
        let err = res.err().unwrap();
        assert!(
            err.contains("missing <feed> or <entry> tag"),
            "Error should identify missing feed/entry: {err}"
        );
    }
}

/// Test network resilience and fallback using a mocked curl executable that delivers
/// an HTTP 200 with HTML 502 Bad Gateway response body.
#[test]
fn challenge_mock_network_curl_html_502_offline_fallback() {
    let root = repo_model::repo_root();
    let temp_dir = std::env::temp_dir().join(format!("lexlean_test_curl_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("create temp dir for mock curl");

    let mock_curl_path = temp_dir.join("curl");
    let script = "#!/bin/sh\nprintf '<html><body>502 Bad Gateway</body></html>'\nexit 0\n";
    fs::write(&mock_curl_path, script).expect("write mock curl");
    let mut perms = fs::metadata(&mock_curl_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&mock_curl_path, perms).expect("set permissions");

    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", temp_dir.display(), original_path);
    std::env::set_var("PATH", &new_path);

    // Verify baseline catalog
    let base_lit = load_literature(&root).expect("load base literature");
    let expected_count = base_lit.paper.len();

    // 1. With offline_fallback = true:
    // Mock curl returns HTML -> parse_arxiv_atom_feed fails -> fallback triggered -> returns local catalog count
    let res_fallback = literature_sync(
        &root,
        Some("test_query"),
        5,
        false, // offline: false (attempt network)
        true,  // offline_fallback: true
        false, // write: false
    );

    // 2. With offline_fallback = false:
    // Mock curl returns HTML -> parse_arxiv_atom_feed fails -> fails unconditionally with Err
    let res_no_fallback = literature_sync(
        &root,
        Some("test_query"),
        5,
        false, // offline: false (attempt network)
        false, // offline_fallback: false
        false, // write: false
    );

    // Clean up environment and files
    std::env::set_var("PATH", original_path);
    let _ = fs::remove_file(&mock_curl_path);
    let _ = fs::remove_dir(&temp_dir);

    // Assertions
    assert!(
        res_fallback.is_ok(),
        "literature_sync with offline_fallback=true MUST succeed on HTML error, got {:?}",
        res_fallback.err()
    );
    assert_eq!(
        res_fallback.unwrap(),
        expected_count,
        "Fallback must return exactly the local literature catalog count"
    );

    assert!(
        res_no_fallback.is_err(),
        "literature_sync with offline_fallback=false MUST return Err on HTML error"
    );
    let err_msg = res_no_fallback.err().unwrap();
    assert!(
        err_msg.contains("literature-sync failed")
            && err_msg.contains("missing <feed> or <entry> tag"),
        "Error message must indicate feed parse failure: got '{err_msg}'"
    );
}

// ============================================================================
// 3. Dry-Run Immutability Verification
// ============================================================================

#[test]
fn challenge_dry_run_immutability_guarantee() {
    let root = repo_model::repo_root();
    let lit_path = root.join("model/literature.toml");
    let initial_bytes = std::fs::read(&lit_path).expect("read literature.toml");

    // Execute dry run with offline_fallback
    let res = literature_sync(&root, None, 5, false, true, false);
    assert!(res.is_ok());

    let post_bytes = std::fs::read(&lit_path).expect("read literature.toml after dry-run");
    assert_eq!(
        initial_bytes, post_bytes,
        "model/literature.toml must not be modified when write = false"
    );
}
