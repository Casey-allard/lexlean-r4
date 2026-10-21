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

fn require_test(root: &Path, path: &str, name: &str) {
    let tests = parse_active_source(&root.join(path));
    let required: Vec<_> = tests
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == name => Some(function),
            _ => None,
        })
        .collect();
    assert_eq!(
        required.len(),
        1,
        "one top-level {name} acceptance test is required in {path}"
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

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    let root = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo supplies the manifest directory"),
    );
    // Inspect the audit and common verifier too; neither has a conditional mode.
    parse_active_source(&root.join("build.rs"));
    parse_active_source(&root.join("tests/support/mod.rs"));
    require_test(&root, "tests/lexlean.rs", "configuration_boundaries_r4_01");
    require_test(&root, "tests/geometry.rs", "finite_geometry_r4_02");
    require_test(&root, "tests/objective.rs", "scalar_objective_r4_03");
    require_test(&root, "tests/binder.rs", "canonical_binder_r4_04");
    require_test(&root, "tests/admission.rs", "admitted_identity_writer_r4_05");
    require_test(&root, "tests/configuration.rs", "typed_configuration_r4_06");
}
