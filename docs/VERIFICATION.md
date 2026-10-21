# Verification record

This record concerns the configuration and subsequent formal development
slices on 2026-10-21. It is not full-template acceptance, an executable R4 engine,
research qualification, a trained artifact, or a Pages deployment.

## Environment

The initial through eleven-component verification used the public development
SDK by immutable index:

```text
ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:60226bc791d4c0e5613402a6be7e63f4963d3faf7f327befcf56fc0e41d0ce21
```

The selected arm64 child is
`sha256:2f82a04e8f8141636e03a219d3960e30d98a6f304828341dd47b553b3c23baeb`.
Publication and upstream source identities are in [UPSTREAM.md](UPSTREAM.md).
The container uses the SDK's LexLean 0.3.0, Lean 4.32.1 and independent
leanchecker. Project language 1.1, builtins, compiler semantics and Lake
workspace inputs are recorded in `lexlean.lock`. These language pins do not
stand in for the absent production PrismPM/template bindings.

The first verification attempt rejected Lake's additional output while Lake
created its missing manifest (`LLV7001`). Initializing Lake, retaining its
manifest and regenerating the language lock resolved this setup problem.
The identity check and verifier were not modified.

## Development evidence

The first red source check reported `LLT4001`: the boundary tests referred to
`R4.Types::validSpec` before its implementation existed. After implementation,
LexLean checked and built two modules and independently verified all eleven
boundary theorems without axioms.

The Rust integration test freezes the reviewed independent literal theorem file
by SHA-256 before verification. It requests machine-readable verification, requires the
selected modules, binds the fresh four identity fields to the returned receipt,
requires each theorem's empty-axiom audit, and requires successful independent
kernel replay for both modules. It does not implement configuration logic.

The shared verifier additionally checks the receipt's copied build manifest
against its hash and length, then binds every non-lexicon input to current
workspace bytes. Canonical paths must stay inside the project. The selected
control file must occur exactly once with its reviewed digest, and the audit
must contain every unique control theorem. The build audit inspects the shared
helper as well as itself and each required entrypoint. The Types control digest
is `38671c44f31d5475abb570c6dc8c4b82f57fc7937277d9d35d0a63442a31be1a`.
After this refactoring, replacing `minimumSpec` with `0 = 0` again failed the
actual integration test (exit 101, `reviewed control statements changed`). The
original fixture was restored with its digest unchanged. Independent Rust and
security review found no blocking issue in this source-binding change.

The following representative defects were planted and restored:

| Defect | Command | Required rejection observed |
| --- | --- | --- |
| Increase vocabulary maximum from 8192 to 8193 in the model | `cargo test --offline --locked -p repo-conformance --test lexlean` | Exit 101; `LLV7002`: `decide` proved `validSpec 8193 2 1 1 = false` is false |
| Replace `minimumSpec`'s assertion with `0 = 0`, preserving its name | Same command | Exit 101; `missing or altered boundary statement minimumSpec` |
| Add `#[ignore]`, `#[cfg(any())]`, `#[cfg_attr(all(), ignore)]`, or `#[should_panic]` to the required Rust test | Same command, one mutation per run | Each fails the build-time acceptance audit with its rejected attribute |
| Add file-level `#![cfg(any())]` to the integration test | Same command | Fails the build-time acceptance audit before compilation can omit the test |

Each change was restored byte-for-byte. Review first reproduced the ignored-test
bypass against the inherited test-name scanner. The project-owned build audit
now parses the mandatory test and its own source, rejects conditional/ignored/
expected-panic attributes, and requires one top-level bare test entrypoint.
It does not certify arbitrary filtered Cargo invocations or changes that remove
the audit itself; reviewed source and the full unfiltered template command remain
the trust boundary. Subsequent workspace tests passed.
The receipt for the restored two-module source is:

| Identity | Value |
| --- | --- |
| Source | `33810a116b5ceb689b22b7c13c14c9df46ca35d2f4decb0252ae491dcdeaa317` |
| Semantics | `0b1442e7046d4d84a820788e3f5058cf106eb0cbfa5c8b6a80cafef5c8d38fa2` |
| Build | `1f4eddddd06a26c93c2754610c9513e56857d13c9cfeaa344110b6a7432c5522` |
| Attestation | `f6b4aee59c51d60c0b0855ba651833619c73ae48915d48b079fb5d4b35942936` |

These are observed development identities, not a replacement for rerunning the
verifier. Generated `.lexlean/` evidence is ignored; no compiled artifacts are
committed. The test never accepts an arbitrary old attestation from that directory.

After restoring the defects, the following development checks passed with
networking disconnected: `cargo fmt --all -- --check`,
`cargo clippy --offline --locked --workspace --all-targets -- -D warnings`,
`cargo test --offline --locked --workspace` (23 Rust tests, no ignored tests;
the bootstrap test also runs two Node checks), and
`cargo check --offline --locked --workspace --all-features --all-targets`.
`just model-write` regenerated `CONFORMANCE.md`. The broader `just model`
also stops at the missing SDK lock; its bootstrap checks were not waived.

## Finite geometry: R4-02

The [geometry verification record](GEOMETRY-VERIFICATION.md) records the 774
theorem controls, exact source and receipt identities, passing Cargo integration
test and three rejected composition defects. Independent specification and
code reviews approved the final source. The connected certificates cover every
discovery and transport step; the finite controls additionally cover closure,
inverse lookup and orbit preservation. These are static operator metadata.

## Scalar objective: R4-03

The registered scenario and controls preceded the implementation; the first
source check rejected the absent module with `LLR3005`. The completed source
has 91 independent theorem controls. Its receipt audits all 103 declarations
(12 source declarations and 91 controls) with empty observed axiom sets and
successful independent replay of both modules. The targeted Cargo integration
test passed with no ignored or filtered tests.

The objective preserves absent-rival hinge behavior, truncated rank violations,
policy selection, all three sentinel-aware product checks and both sum checks
before selection. Source-defined natural arithmetic avoids opaque SDK helper
reduction. The chunked scaling recursion computes the complete result for every
fuel value; zero fuel executes the complete residual. Controls cover all eight
residue classes, small and zero fuel, and every valid geometric width. See
[OBJECTIVE.md](OBJECTIVE.md) for the exact source-domain and integration limits.

| Planted defect | Command | Required rejection observed |
| --- | --- | --- |
| Remove the address sum overflow check, leaving the rank policy selected | `lexlean --diagnostic-format json verify tests/lexlean/ObjectiveControls.lex.tex` | Exit 1, `LLV7002` at `unselectedAddressSum` |
| Accept an address product exactly equal to the UInt64 sentinel by replacing `>=` with `>` | Same command | Exit 1, `LLV7002` at `productSentinelCollision` |

Both mutations were restored byte-for-byte. Independent specification and code
reviews approved source digest
`43af667dc55f173ef1629192156315ca90b7ff0f599f901250e21c42c5ea12ce`
and control digest
`9934a492d9d751ed0fde07f193b5a2582e70219b8a3f2eb72b595f955b2d77fc`.

| Identity | Value |
| --- | --- |
| Source | `58d64647419459772d06707063431108f16c0e00c2d8b7b4287f6e85263d6510` |
| Semantics | `27fcba8a8c815314317f38ed2f0a0764401a626f0100ffb24e5defb0e0ca8759` |
| Build | `9dce1d2819dbd3b46ee5300877839d76016b3d031fa92159bfd0f6ef17a56a39` |
| Attestation | `92cabf5852d44a8964c3e316475fb14b177fc5e178f1857e2bc31291c9bfc0dd` |

This establishes scalar formal evidence. Writer eligibility, policy-dependent
rank computation, caller status conversion, learning, GPU parity and performance
remain independent obligations. No model or training execution was run.

## Read configuration: R4-06

[CONFIGURATION.md](CONFIGURATION.md) records the full source contract and
75 controls, including validation ordering, exact mode metadata, independent
initialization and checked gate counts at UInt32 boundaries. Its Relation
factor mutation was rejected and restored. Independent specification and
code/Rust/security reviews approved the recorded source and fixture digests.
After the parent registered the mandatory test in the build audit, the restored
integration command passed again with one test and no ignored or filtered tests.

## Binder and writer: R4-04 and R4-05

[BINDER.md](BINDER.md) records 165 controls for canonical construction, ordered
general-forest validation and node serialization. The maximum context of 243
atoms constructs 364 nodes and a six-stage plan. A duplicate/previously consumed
child guard defect was rejected and restored. The controls do not constitute
a universal theorem over every context or a window-framing implementation.

[ADMISSION.md](ADMISSION.md) records 72 controls for the complete writer over
explicit admitted queries and codebooks. Ranking retains token identities,
including duplicate vectors, and virtual moves affect distances and cell
membership consistently. Review corrected exact-sentinel classification:
selected UInt64 maximum is a sentinel failure, while greater sums are arithmetic
failures. The composed eligibility control rejected the old classification.
Cell tie reversal and suppressed virtual moves were also rejected and restored.
Both slices passed independent specification and code/Rust/security review and
their required Cargo tests with no ignored or filtered tests.

## Default verification graph

The default entrypoints now include all six control roots in lexical order,
as required by the SDK configuration schema. The only changed language-lock
field is the project configuration digest:
`971e052051a2d642c3fd8eec5a6a5b8c9a1ff831603faa864fa00a8b98af9c6d`.
The entrypoint array uses the SDK's canonical single-line TOML encoding. A
multiline encoding passed source verification but correctly failed the stricter
current-byte receipt binding because the SDK manifests canonical configuration
bytes. Canonicalizing the file preserves the full input check and lock digest.
Running `lexlean --diagnostic-format json verify` without an explicit root
successfully verified thirteen modules and all 1,188 controls with no diagnostics.

| Identity | Value |
| --- | --- |
| Source | `b6f1e900cd3a2ad9e5aabc73db09c8ce80a451b629430533e62eac3ce042c942` |
| Semantics | `f387963b4cb23badb1a6fa9534ffc6a5141b383438123960cea63142a1210ea8` |
| Build | `c2917d9573d5f64c8e63fb4656e2df0703d372793a9f512db404aaa9c8fa9ba9` |
| Attestation | `552ece5e40a6fcc92f592829516471416c7ee15b6ab41ce06655cd6c2bbb0631` |

Earlier per-slice receipts above and in their contract documents are historical
observations before this entrypoint update. Fresh verification remains required;
these recorded identities never replace the live acceptance tests.

## Clean committed-source replay

[LEARNING.md](LEARNING.md) records the subsequently integrated R4-08 component:
154 controls, the shared Gates dependency, three rejected defects, and explicit
producer/authentication limits. Independent specification and code/Rust/security
reviews passed. An isolated nine-slice snapshot, with the mandatory test and
default root registered, passed formatting, model generation, canonical lock
checking and the targeted learning test: one passed, none failed or ignored.
This targeted result does not replace the whole-workspace replay below, whose
eight-slice revision remains explicitly identified.

Commit `44088c2` was exported with `git archive` into a new directory inside the
network-disconnected development container. This excluded uncommitted binder
work and all previously generated project proof/build artifacts. On that exact
snapshot, formatting, warning-denying Clippy, all workspace tests, all-feature
and all-target compilation, and generated-model equality passed. The run
contained 25 Rust tests and two Node checks, with none ignored or filtered.
Its three formal integration tests reverified the 876 theorem controls from
fresh project outputs. Dependency caches and the installed SDK remained shared;
this is a clean source/build replay, not a new SDK build or supply-chain audit.

The six-slice replay used committed source `6c38f00`, again exported through
`git archive` into a fresh container directory. Formatting, warning-denying
Clippy, all workspace tests, all-feature/all-target compilation, generated-model
equality and the canonical language-lock check passed. This run contained
28 Rust tests and two Node checks, with no ignored or filtered tests. The six
formal integration tests independently reverified all 1,188 controls. The first
attempt at its predecessor `e9ec52e` stopped at one rustfmt line-wrap mismatch
in the admission test registration; the formatting-only correction was reviewed
before this replay. Full `just vv` on `6c38f00` still failed at the missing
production `prismpm.lock` after successful six-ID model equality.

The next isolated snapshot contains the eight slices committed at `cb454d6`.
All 96 tracked files were compared by Git blob identity with that revision;
there were no mismatches. Formatting, warning-denying Clippy, all workspace
tests, all-feature/all-target compilation, model equality and the language-lock
check passed. The run contained 30 Rust tests and two Node checks, with none
ignored or filtered; its eight formal tests reverified 2,241 controls. Route
and Words also passed their targeted tests after registration in both the
mandatory build audit and the default verification roots. This snapshot used
separate project output directories and the same cached development SDK and
dependencies. Full `just vv` again failed at the missing production lock after
successful eight-ID model equality.

Default verification without an explicit root also passed for that eight-slice
snapshot, selecting seventeen modules with no diagnostics:

| Identity | Value |
| --- | --- |
| Source | `8d4961dfe7cb325c56035502ae8e10463bd36da3b14fc08bbdef259af10935b8` |
| Semantics | `21fba03cebbee5b71efa7e9c9db5e48f46653a2ab90c5cdc8cbb5a1d24c7dec8` |
| Build | `25d8c115646fc606dc2bf752989e6468d5e3ab04e3799e2a6311b53090b56774` |
| Attestation | `56030a9e4a5c5b8ce5f139ae87e29046aba63c94afe2fc3fc01f76ed29014c31` |

## Complete acceptance remains blocked

`just vv` was invoked inside the candidate with networking disconnected. It
passed the model equality check, then rejected bootstrap acceptance:

```text
gate failed: prismpm.lock is required: No such file or directory (os error 2)
error: recipe `template-check` failed on line 17 with exit code 1
```

The universal gate is retained unchanged. Passing development checks does not
waive this failure. Production delivery needs the accepted SDK and matching
rendered template contract. The published development candidate cannot provide
the current template renderer. [The design](specs/2026-10-21-lexlean-r4-design.md)
also requires a generic GPU execution target and accepted generated browser
persistence. Those dependencies remain unfulfilled; no substitute runtime or
placeholder Pages application has been published.

The draft PR's inherited bootstrap workflow independently reproduced the absent
lock failure on both amd64 and arm64 runners in
[run 39553208326](https://github.com/Casey-allard/lexlean-r4/actions/runs/39553208326).
Both jobs failed while reading `prismpm.lock`, before SDK execution. This is a
dependency-binding failure, not a model proof result.

## Ten-component isolated replay

The isolated staged tree matched all 113 tracked file blobs of commit
`f1c9df4e97a9df28bebf771f98f81e2e16b6f208`, with zero mismatches. It includes
the reviewed Flock/Relation component and the exact byte-count reduction repair.
The snapshot had its own Cargo target directory and was extracted without
host metadata into a fresh container directory; no future Hash or Engine
claims or sources were included.

Inside the same immutable development SDK candidate, these checks passed:

```text
cargo fmt --all --check
cargo run --offline --locked -q -p xtask -- check-model --write
lexlean lock --check
cargo clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
cargo test --offline --locked --workspace --all-features
cargo run --offline --locked -q -p xtask -- check-model
lexlean --diagnostic-format json verify
```

The workspace suite passed 32 Rust tests and two inherited Node tests, with
zero failed, ignored or filtered tests. The ten formal component tests
reverified 2,514 controls. The combined default verification separately
verified all 24 modules, with no diagnostic. Its receipt contains 3,786 audited
declarations: every observed-axiom list is empty, every policy is `none` with
an empty allowed list, and every result is `ok`. All 24 independent kernel
checker processes exited zero.

Combined default receipt:

- Attestation: `68fcfb6a11a1fc82d2bdb2ce44ed01abd81ad5b49e14bb6c3abc3e9ef8777b31`
- Build: `803b37432f1033b22f8662681a0d0dfbee9203f68658777970bb558664d69681`
- Source: `95f63d13631dddaff08df2708f99d9f748c181bee531f01a9bb2b39cc04dd4cf`
- Semantic: `7a1a2f041ad32ccd2332d01e0595bffa2bdc403ee2a2af70be1c2921a4ff8db1`
- Configuration SHA-256: `0a725c21a70f374fe8a3658bd27456ddfb9d13771dc1332947675b5cbf6e053c`
- Lock SHA-256: `15b78263af6877e3416652c44dd38beaf9d72c7f5b5ea5de6bfe62f76888cb59`

The full `just vv` was also attempted in this snapshot. Model consistency and
dependency ownership checks passed; `template-check` then stopped because
`prismpm.lock` is absent. This remains a failed production acceptance boundary.
No GPU model execution, parity, training, generated browser acceptance or
Pages deployment is established by the formal development replay.

## Eleven-component composition replay, 2026-10-21

The next isolated snapshot matched all 119 tracked file blobs of commit
`82437555f9fd78a8a701e85ad29e6ff2e398811e`, with zero mismatches. It adds the
reviewed Engine, its mandatory test and default proof root, and the compiler's
regenerated lock. Pending Hash/Object sources and claims were excluded.

Within the same immutable development candidate, formatting, lock checking,
model generation, Clippy with all targets/features and warnings denied, and
the workspace tests with all features passed. The refreshed Engine integration
test passed independently, then the workspace suite passed **33 Rust tests
and two inherited Node tests**, with zero failed, ignored or filtered tests.
All eleven formal component tests ran, covering **2,614 theorem controls**.
The five final-fixture Engine plants and restored-source checks are recorded
in [ENGINE.md](ENGINE.md); the fixture transformation expanded six constant
definitions into the same 100 theorems, without changing their semantics.

Combined default verification also passed: **26 modules**, 4,043 audited
declarations, empty observed axiom lists, `none` policies with empty allowed
lists, and 26 successful independent kernel replays. A separate receipt audit
checked the exact identities, build-manifest hash/length and every current
non-lexicon input against the frozen snapshot.

- Attestation: `ad3c5e68142cea8bb8ba382af93f94700f27d2f7d87d881441fcedc571bf48a5`
- Build: `773b615ae4ea52a6d81ffaa80b4fc3be1ccbec1f4adccdd5d06d2d82bd5545fd`
- Source: `e8178e798ee0d4a448ba66d706122ad9341981ec64030cb748b83395e8529dd1`
- Semantic: `2ad4242d5905cf47d7071e51ce6eba7ef2201d9f8358d54a14d8782d2a532ae4`
- Configuration SHA-256: `c0c90ffc3cb5f6314726545d7cf4f031a0beb47b5ba8132f9906cecae7543e4d`
- Lock SHA-256: `08f29b30caa785bf60b812aadcf8976e5bb981ead74639128abc55c60fce5b3e`

The actual full `just vv` attempt passed model consistency for eleven IDs and
dependency-ownership auditing, then failed because `prismpm.lock` is absent.
Production acceptance remains failed. Formal Engine composition establishes
neither generated runtime custody, authorized GPU parity/training, an exported
serving artifact, durable browser behavior nor a Pages deployment.

## Twelve-component replay under the newer construction

Hash commit `a8599460127a8805f482a7f890fb3625f662f401` adds the complete hash
component and its thirty original theorem controls. This snapshot uses the
official construction artifact described in [UPSTREAM.md](UPSTREAM.md), OCI
manifest `716370f20b6b01948700a16380668f5328b854e3764188916fc8802ef27ca7c6`,
with executable SHA-256
`107e02328a8052d1560d6e8addcfd1c70af6145bbf836650cac32dc044f550e5`.
Its genuine language-1.1 lock records compiler semantics
`4658bc8564c14b99f771a758276c578e8acd5ef5f24a3c43e69e4e6eb7df370e`;
the version banner's compatibility-language-1.0 identity is a separate value.
The fixed SDK limits, no-axiom policy and shared receipt validator are unchanged.

Formatting, twelve-ID model generation, lock checking and all-target/all-feature
Clippy passed. The actual mandatory hash target passed all thirty cases in
1010.80 seconds. A separate workspace regression run passed the previous
33 Rust tests and two inherited Node tests under the same compiler; that
regression command explicitly excluded the already completed hash target.
It therefore reported one filtered test and is not represented as an unfiltered
single workspace run. Together, the two runs exercised all twelve component
tests; neither is the production acceptance boundary.

```text
cargo test --offline --locked -p repo-conformance --test hash
cargo test --offline --locked --workspace --all-features -- --skip complete_content_hashes_r4_11
lexlean --diagnostic-format json verify
```

The separate, unfiltered default LexLean verification passed **57 modules**
and **2,644 controls**. Its receipt contains **4,254** unique declaration audits,
all with empty observed axioms, policy `none` with an empty allowed list, and
result `ok`. All **57** independent kernel replays passed. The parent separately
audited result/receipt/manifest identities, manifest bytes and length, every
control's presence and every one of the **59 non-lexicon input files** against
a frozen checkout of `cc2d32fbcbb5b2460ec5c1130ad924efb02ebd73`. That later
revision adds documentation without changing the verified program inputs.
This is a proof-input comparison, not a claim that the original staging
directory contained every later documentation file.

- Attestation: `f61211a420ef6b88811b417b88bf64f5bbd93a94f553df0d6fa903553eb54d12`
- Build: `0a9c994de308ec2b231bd02ac33b54369f3a8682b15d6f2186ef71e2e9be9a75`
- Source: `c753c4b71c3e9e3308bb5ee8c7565d4da3f66f0a724103d627a1442410c07177`
- Semantic: `7ec14ecd5c2dd520428d0651d1e5c355c865455fdd464e31b48fb78e47a81915`
- Configuration SHA-256: `e6dcbfc21b3eb3a336aa48165b6b135b56cb28ca34fe1f4a1b82d6d0f92a6e36`
- Lock SHA-256: `9ed30743817f93569d958b9ea95d43a25e98aa0a1d5819fc8df51e089b1a7ac6`

The actual `just vv` attempt passed twelve-ID model consistency and dependency
ownership, then failed at `template-check` line 17 because `prismpm.lock` is
absent. The construction artifact remains explicitly unaccepted for production.
No generated execution, GPU research, trained capability, persistent browser
behavior or Pages deployment follows from these formal results.

## Thirteenth component: Unicode and UTF-8

Commit `9cfb1ce` adds R4-14 Text after independent specification, complete
Unicode-inventory, implementation, Rust and input-boundary reviews. The
unchanged 3,865 controls cover all imported Unicode boundaries, the reversible
byte alphabet, scalar/byte admission and strict/lossy UTF-8 behavior. Four
semantic plants failed their intended proof controls and restored sources
passed, as recorded in [TOKENIZER.md](TOKENIZER.md).

The actual newer-SDK mandatory target
`cargo test --offline --locked -p repo-conformance --test tokenizer_text`
passed in 65.20 seconds, with one test and zero failures, ignored or filtered
tests. The shared verifier checked 3,915 declaration audits with empty observed
axioms and three successful independent kernel replays. Its attestation is
`150e1654ec1351ef0b483d7085d058ef5d75f650c0c0376258e1574ca7d7ac22`.
The project configuration SHA-256 is
`582dd99a4afa394ef4adccb717610982db1686ab9f7dfa6c8c0e2515a198e59c`;
the genuine language-1.1 lock SHA-256 is
`b6e36d0c495ddfd9732f70c643e3ec7e00d2ed97372269d546a3ceb648bd0c64`.

Formatting, thirteen-ID model consistency, lock checking and all-target/
all-feature Clippy passed in the isolated integration snapshot. Default source
checking also passed all 60 modules without a diagnostic. Its source identity
is `ef2de251daef347fa2a736ec968f390001563e00f394c12400d76ab069c4467f`
and semantic identity is
`96c3e2d40a8c57cbd23f47407bda9a74c39891a6d077dad879f4f2df9a5e22de`.
The combined graph now has 6,509 controls, but its combined proof verification
has not yet run. The previous twelve-component replay and this targeted Text
replay are separate results. No complete tokenizer, trusted tokenizer identity,
production acceptance or browser deployment is established.

The actual thirteen-component `just vv` attempt passed model consistency and
dependency ownership, then failed at `template-check` line 17 because
`prismpm.lock` is absent. An initial invocation used the login shell's default
Cargo home and was interrupted without a result; the recorded attempt used
the retained offline cache. Neither attempt establishes production acceptance.

## Fourteenth component: typed tokenizer kernel

R4-16 adds the reviewed 115-declaration typed tokenizer kernel and 127 frozen
controls in six fixtures. The mandatory Rust integration target passed in
124.48 seconds with no ignored or filtered tests. Every fixture was checked by
the unchanged shared receipt validator, including exact input identities,
none-policy axiom audits and independent kernel replay. Full receipt identities,
Unicode regression evidence and four semantic plants appear in
[TOKENIZER-KERNEL.md](TOKENIZER-KERNEL.md).

The isolated integration snapshot passed fourteen-ID model generation,
formatting, lock checking and all-target/all-feature Clippy with warnings denied.
Default source checking passed all 67 modules without diagnostics:
source identity `325887ba206611bbf61c0122766490d0d303b4fc411320186f5556e6981f4e81`,
semantic identity `c8bebc52f37d2fe6e57a297691eda6ab42ab952f93dbe6f1d2a5f459022c4a8b`.
Configuration SHA-256 is `61608e2debc1e76b89258bd2c74d8188f26d7d2454a8d6f40894f7b161a6ae2e`;
lock SHA-256 is `1b35d04ceb567c6fbe03885ff1814be73a60e0863b47056843bea13663365c6e`.

The combined graph contains 6,636 theorem controls. Its combined formal replay
has not yet run; the last full combined result remains the twelve-component
replay recorded above. Full `just vv` again passed model consistency and
ownership and failed at template-check line 17 on missing `prismpm.lock`.
This establishes neither raw JSON/tokenizer identity nor accepted generated
execution, research qualification, persistent application behavior or Pages.

## Fifteenth component: complete mechanism and evidence descriptions

R4-17 adds the full imported catalog, coverage admission and report-description
model. All 627 frozen controls passed the mandatory aggregate Rust target in
66.57 seconds. The seven invocations checked exact current inputs, empty axiom
audits and independent kernel replay. Fifteen source-valid semantic plants
were rejected across the catalog, coverage and description modules; byte-exact
restoration passed. Complete identities and boundaries are in
[EVIDENCE.md](EVIDENCE.md).

The isolated integration snapshot passed fifteen-ID model generation, formatting,
lock checking and all-target/all-feature Clippy with warnings denied. Default
source checking passed all 77 modules with no diagnostics:
source `846bb525d3d534bcd43c65f59195700bc5ff8fe69a442fb7cbef17c5c903b3b6`,
semantic `ba59d5036bc389a95681f0c4b3716d9e6e3863f515d53b803a86742cd329196b`.
Configuration SHA-256 is `2d09b3cfbaffd840e89cd4d93c5e02fcc98427f69f36f00a6114c7905591a497`;
lock SHA-256 is `39d2c63edb4979e9aa6357ad63034302e9dcbce3e855bde9e90aac2d8a24e551`.

The combined graph contains 7,263 theorem controls; its combined formal replay
is pending. The last full combined replay remains twelve components. Full
`just vv` passed model consistency and dependency ownership and again stopped
at template-check line 17 because `prismpm.lock` is missing. Structural report
admission, upstream status, source proof, operational authority and actual
production deployment remain separate claims.

## Eighteenth component: Snapshot storage codec

The [Snapshot record](SNAPSHOT-JSON.md) binds the 143-declaration implementation,
476 unchanged controls, all eleven source-valid semantic negative results, exact
restoration, mandatory Rust integration and complete default-graph verification.
The target passed two tests with no ignored or filtered tests. The combined graph
passed 8,746 controls, 11,038 exact none-axiom declaration audits and 108 kernel
replays. Model generation/checking, formatting, Clippy and all-feature/all-target
checking also passed.

Attestation: `e532e1cfd597ae6d18ab4e33b601bf12c031217ad63ae80617ca680c36ee4afd`.
Configuration: `cd87a5aa342663566f53e4089f8252e0c66de98725a01961c35df3218f5fbcdb`.
Lock: `f35f51860d64c0cd703b35a6ecb912ed4c116399d5d920433a663907fd1da04e`.

The combined graph required six GiB of container memory in the existing eight-GiB
VM. Four- and five-GiB attempts exited 137 in the compiler front end; their kernel
evidence and the corrected offline-Cargo invocation are retained separately. No
proof limits, source controls or axiom policies changed. This is development
evidence; actual `just model` and `just vv` still fail on absent `prismpm.lock`,
and no whole-workspace run, production acceptance or deployment is claimed.
