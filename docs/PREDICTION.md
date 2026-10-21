# Target-free prediction refinement

Status: 31 independent controls frozen and their missing-module rejection
observed before candidate source authoring. Formal verification and the general
refinement proofs remain pending. This is not serving acceptance.

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
