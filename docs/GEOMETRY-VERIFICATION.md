# Finite geometry development verification

This record covers R4-02's fixed operator and root metadata on 2026-10-21.
It is formal/static development evidence. It establishes neither learned-model
capability nor GPU, production SDK, exported serving, or browser acceptance.
See [GEOMETRY.md](GEOMETRY.md) for the modeled representation and certificate
obligations.

## Exact inputs and verifier

| Input | Identity |
| --- | --- |
| `src/R4/Geometry.lex.tex` SHA-256 | `25d3b493659732c3a687838c20c75f9acc458c62abd81993d8d512d74f2ee526` |
| `tests/lexlean/GeometryControls.lex.tex` SHA-256 | `5574f2c697d141d78e2d099bace7b8a0d5fd3b1c9be6449607de547295837a9e` |
| Development LexLean revision | `01088ef00d9d0d0227775e0346a852c2d19cb67b` |
| Candidate SDK image | `ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:60226bc791d4c0e5613402a6be7e63f4963d3faf7f327befcf56fc0e41d0ce21` |
| Executed ARM64 image manifest | `sha256:2f82a04e8f8141636e03a219d3960e30d98a6f304828341dd47b553b3c23baeb` |

The existing development container had networking disconnected. No SDK source,
compiler limits, production locks, or generated Lean files were edited. This
candidate supports the language 1.1 semantic source used here; it is not an
accepted production template/SDK binding.

## Results

The final `lexlean --diagnostic-format json verify
tests/lexlean/GeometryControls.lex.tex` completed successfully, with zero
diagnostics, for modules `GeometryControls` and `R4.Geometry`. The control source
contains 774 theorem declarations.

| Receipt field | Value |
| --- | --- |
| Attestation | `fc02ed87e08075db85e223124b5047221a9e225cb84f62b50f174e34a75043a0` |
| Build | `8b740cc6147a30a9a6b28433e799a4450038ac5ee86ba6cebdad2caeb914facc` |
| Source | `a6e9b3be6020ddcf8a4bb910f9427d4daf5f2dc40291192055b9e7549d652c02` |
| Semantic | `c0736cb1c3964c19eee0cbd1c747a16fe4171893784c4204fd7dcf1a6dcb985d` |

The SDK command `cargo test --offline --locked -p repo-conformance --test
geometry` passed its single `finite_geometry_r4_02` integration test in 57.16
seconds. The shared verifier checks the frozen control-source digest, all
declared theorem names and empty-axiom audits, fresh receipt identities, build
manifest bindings, and independent Lean replay. Receipt files remain generated
development artifacts; this document records their identities rather than
committing generated binaries.

## Independent failure controls

The R4-02 identifier and scenario preceded implementation. The initial control
source failed checking with `LLR3005` because `R4.Geometry` did not yet exist.
The inverse lookup controls were likewise added before their API and failed
with `LLT4001` for the missing `R4.Geometry::inverseState` declaration.

After the positive implementation passed, three source mutations were planted
one at a time. Each was rejected by the actual SDK verifier and restored before
the final successful verification above.

| Planted defect | Observed rejection |
| --- | --- |
| Force every composed selector's sign to positive | Integration test exited 101; `LLV7002` rejected the independent square and ordered composition expectations, including `composeState 1 1 = some 16`, plus trace equalities. |
| Reverse the two operands throughout action composition | Verification exited 1; `LLV7002` rejected independent generator literals, beginning with the first generator `[12,10,1,14,0,7,3,13]`. |
| Reverse only the transport path accumulation from generator-times-path to path-times-generator | Verification exited 1; `LLV7002` rejected direct record transition equalities, including `transportStep transportTrace0step1 = transportTrace0step2`. |

The controls therefore exercise sign handling, action composition orientation,
and the distinct transport BFS accumulation order. Finite proofs and these
mutation failures make no claim about unimplemented accumulated carriers or
research-compute qualification.
