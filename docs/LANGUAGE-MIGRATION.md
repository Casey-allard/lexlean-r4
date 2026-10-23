# Official language 1.2 migration

Status: independently reviewed; the complete formal replay and mandatory workspace
integration tests passed. This file does not record production
acceptance or change the template's required SDK binding.

The migration starts from revision `333986e` and retains the declarations in
all 77 selected language-1.1 modules exactly in the first, version-only phase.
That phase is limited to
glossary references `@1.1.0` becoming `@1.2.0` and the semantic document spec
changing from `lexlean/semantic-module/1` to `/2`. Project language becomes
`1.2`. This first phase changes no semantic declaration.

The official linker then rejected 23 local parameter names that collide with
top-level declarations or the built-in Lean name `id`. The separate
`tests/fixtures/language-1.2-alpha-migration.json` records each actual diagnostic
repair across eight source modules: the local parameter name, its bound variable
occurrences, and its recursive-argument annotation where applicable. Each repair
checked unique binding and absence of nested shadowing. Reversing every recorded
rename restores the complete pre-repair AST exactly. No function call, record
field, literal, branch, type, theorem statement, proof, axiom policy or import
edge changes. The original version-only manifest remains unchanged.

The complete 84-module default graph passed the official linked-IR check after
these repairs. Complete formal replay then verified all 7,871 controls across
84 modules under attestation
`5124576420efeda3241a8865501bf1eed32788f633d9a0868b84716eb5b9bdba`.
Independent receipt audit checked all 9,910 none-axiom declaration rows, exactly
one successful kernel replay per module, and all 86 current non-lexicon inputs.
The initial mandatory Number gate then rejected the new attestation/2 format
at its former hardcoded attestation/1 assertion. The source-bound compatibility
repair is documented in ATTESTATION.md; its adversarial unit controls and three
planted defects passed independent review and restored execution. The repaired
actual Number target then passed both tests (608 controls and receipt-language
binding), with no filtered tests, in 70.91 seconds. The subsequent complete
`cargo test --workspace` run passed all 54 Rust tests, all 16 component gates,
and two nested bootstrap checks, with no ignored or filtered tests. The Number
gate passed again in 64.85 seconds; the complete 30-case Hash gate passed in
673.40 seconds. This establishes development workspace integration, while the
production binding and export obligations below remain open.

`tests/fixtures/language-1.2-migration.json` binds the original and migrated
file hashes plus each unchanged declaration-array hash. An independent review
compared the original files with actual Git objects, reversed the two permitted
text substitutions, and compared every declaration. The file-byte pins in the
mandatory Rust controls and HashCase inventory are updated to the migrated
bytes. The archival thirty-theorem HashControls source remains byte-identical;
the complete declaration-multiset comparison still applies. Shared receipt
validation now binds the receipt version and manifest language to the immutable
control schema; all existing identity, source, axiom and replay checks remain.

The official development SDK construction contains LexLean revision
`0865df4e540ca643b090a29e9df6d319d6906da6`. Its executable SHA-256 is
`107e02328a8052d1560d6e8addcfd1c70af6145bbf836650cac32dc044f550e5`;
the actual language-1.2 semantic identity is
`09f4b95d5994ca00acd413e6f9e8381c8a18d4fd778249d2c8654f7dc99dce86`.
This is the unchanged official compiler. No patched backend, injected Lean,
extended resource limit, native UInt axiom waiver or local production SDK
substitute is included.

Exact JSON numeric admission requires the supported language-1.2 arithmetic
semantics. Its two source modules and five independently frozen control roots
add 608 theorems to the existing 7,263. The combined default graph therefore
has 84 modules, 60 roots and 7,871 controls. Both number modules already passed
their isolated formal controls, source-valid semantic plants and restored
replays; those results do not substitute for this integrated migration.

Acceptance of this development migration requires a fresh official lock,
complete default-graph replay, all mandatory Rust receipt tests, source/model
and ownership checks, formatting and lint. The unmodified `just vv` must also
be attempted and its actual result reported separately. Missing `prismpm.lock`
or template production binding cannot be replaced by this development record.
All generated runtime, checked-u64 versus big-Nat, fixed-memory, browser and
Pages obligations remain required.

The staged formatting, workspace Clippy, all-feature/all-target checks, four
library tests and four BDD/meta tests passed after the receipt repair. The
unmodified `just vv` was attempted and stopped in `template-check` because
`prismpm.lock` is absent. `just model` independently reached the same missing
binding after its register, failure, deferral and ownership checks. These
blocked complete gates are not reported as passed. Durable source/receipt
evidence, including the complete workspace execution log, is retained under
`/Users/casey.allard/lexlean-r4-evidence/2026-10-21/language-1.2/`.
