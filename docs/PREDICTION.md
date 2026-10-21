# Target-free prediction refinement

Status: 31 independent controls frozen and their missing-module rejection
observed before candidate source authoring. Their isolated formal replay passed;
semantic mutation checks, mandatory integration and general refinement proofs
remain pending. This is not serving acceptance.

The [API contract](specs/target-free-writer-api.md) specifies complete input
admission, occupied-cell selection, global token ranking and the required
general connection with the supervised writer. The control fixture SHA-256 is
`6fe8c8ccc4cc05fddc67191fd9a9bb66d34517c8dfb741f27a460f23cc0f0190`.
[`controls.json`](../tests/fixtures/prediction/controls.json) records literal
provenance from the existing independently frozen Admission controls. Equal
target-free calls were deduplicated only after exact expected-output equality;
five additional mixed-invalid cases distinguish admission error order.

After successful lock creation, the unchanged official SDK rejected the frozen
root with `LLR3005`: `PredictionControls` imports the absent `R4.Prediction`.
Directory ownership and workspace setup failures preceding that check were
retained separately and were not counted as the failing-test result.

The isolated language-1.2 replay used the unchanged official development
construction, executable SHA-256
`107e02328a8052d1560d6e8addcfd1c70af6145bbf836650cac32dc044f550e5`.
Version-only migration of the prediction source and controls preserves every
semantic declaration. Its control file SHA-256 is
`677087de1a6b844dd353e87909d96d9feb95d8cb2d07108f527d26698d3a5cde`;
the candidate source SHA-256 is
`f29047cdc599643e36823ef6cc8ac98538b9e3268c26cbf7ccdca8a373719cd0`.
The other five source modules come from the independently audited language-1.2
migration. Attestation
`be039ace487033e28491c1a8cdd9d1bcbc5a7c68ebf3ecdef98cf6080582177d`
verified all 31 controls. Independent audit checked 904 none-axiom declaration
rows, seven successful module kernel replays, all nine current non-lexicon
inputs, receipt/manifest identities and compiler/language binding. An initial
unsupported CLI option was a usage error; it is retained separately and is not
a semantic negative. Durable evidence is retained under
`/Users/casey.allard/lexlean-r4-evidence/2026-10-21/prediction/finite-31/`.

The candidate wrapper reuses the actual score traversal, occupied-cell
selection and global ranking. It accepts no target or loss-policy argument.
Defensive classification of impossible shared error constructors is explicit
in the API and cannot substitute for the required unreachability proof.

General success, complete identity retention, global minimum, supervised
projection and target/policy/telemetry independence remain required over the
complete admitted domain. Passing the finite fixture alone cannot discharge
them. Checked loss bounds and lawful proof composition must be established
under the unchanged SDK axiom policy. Artifact custody, production lowering,
session serving and deployment remain separate obligations.
