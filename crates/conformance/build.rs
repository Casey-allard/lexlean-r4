//! Reject disabled R4 acceptance entrypoints before Cargo can skip their tests.
//! The immutable template's unfiltered `just vv` remains the acceptance root.

use std::path::Path;

use syn::visit::Visit;

struct ActiveAttributes;

impl<'ast> Visit<'ast> for ActiveAttributes {
    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        for forbidden in ["cfg", "cfg_attr", "ignore", "should_panic"] {
            assert!(
                !attribute.path().is_ident(forbidden),
                "R4 acceptance source must not use #[{forbidden}]"
            );
        }
        syn::visit::visit_attribute(self, attribute);
    }
}

fn parse_active_source(path: &Path) -> syn::File {
    println!("cargo:rerun-if-changed={}", path.display());
    let source = std::fs::read_to_string(path).expect("required acceptance source exists");
    let parsed = syn::parse_file(&source).expect("acceptance source parses as Rust");
    ActiveAttributes.visit_file(&parsed);
    parsed
}

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    let root = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo supplies the manifest directory"),
    );
    // Inspect the audit's own attributes too; it has no conditional mode.
    parse_active_source(&root.join("build.rs"));
    let tests = parse_active_source(&root.join("tests/lexlean.rs"));
    let required: Vec<_> = tests
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == "configuration_boundaries_r4_01" => {
                Some(function)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        required.len(),
        1,
        "one top-level R4 acceptance test is required"
    );
    let function = required[0];
    assert!(function.sig.inputs.is_empty() && function.sig.generics.params.is_empty());
    assert_eq!(
        function
            .attrs
            .iter()
            .filter(|attribute| {
                matches!(&attribute.meta, syn::Meta::Path(path) if path.is_ident("test"))
            })
            .count(),
        1,
        "the required R4 entrypoint must be a bare #[test]"
    );
    assert!(
        function.attrs.iter().all(|attribute| {
            attribute.path().is_ident("doc") || attribute.path().is_ident("test")
        }),
        "the required R4 entrypoint must not have attributes that alter execution"
    );
}
