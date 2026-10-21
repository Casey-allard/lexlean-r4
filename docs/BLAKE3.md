# BLAKE3 implementation evidence

R4-15 is registered in the pending development model for the complete unkeyed
BLAKE3-256 algorithm described in `specs/blake3-design.md`. It is not part of
the committed sixteen-component default graph. The isolated pure Nat
implementation has passed
255 of its 272 frozen formal controls, including 18 of all 35 required
published digest cases. Complete BLAKE3 and tokenizer identity acceptance
remain pending; these formal checks establish no production execution claim.

## Independent fixture freeze

`tests/fixtures/blake3/official-1.5.0.json` preserves the complete published
vector file at BLAKE3 revision `5aa53f07f7188a569cadfc5daf1522972d9a9630`,
SHA-256 `dcb91ea8accc77e6d6e632af7cdc1a99a9f3ae78cf648da595c7d064db32f624`.
`inputs-digests.json` freezes every one of its 35 cases, with literal input
bytes following the published repeating 0..250 pattern and the first 32 bytes
of each published unkeyed output. Its SHA-256 is
`40232027089ea6333220389b178deeaba3acc068fb48052d483d84cb659f53bf`.
No hash implementation ran to produce these expected outputs.

The public complete digest theorem files will use these same byte inputs and
expectations after the supported representation is selected. Choosing a word
or input representation cannot remove a case, shorten a message or change a
digest. The largest case contains 102400 bytes and needs a compact or segmented
literal representation compatible with the project's source limits.

## Primitive representation diagnostic

An isolated project at `/tmp/lexlean-r4-blake3-dev-20261021` contains 20 original
primitive controls. Before definitions were written, verification failed
with `LLR3005`: imported `R4.Blake3Primitives` did not exist. The original
controls have SHA-256
`da4af7c9815682182b1299492efbfef77602ce6e1bee222cdff790330ba59931`.

Tests use the official current SDK, LexLean revision
`0865df4e540ca643b090a29e9df6d319d6906da6`, language-1.1 lock semantic identity
`4658bc8564c14b99f771a758276c578e8acd5ef5f24a3c43e69e4e6eb7df370e`,
with unchanged none-axiom policy and project resource limits. Scratch setup
first needed its Lake files/manifest and corresponding fresh lock; those
setup failures are not the semantic RED evidence above.

After writing direct primitive wrappers in the isolated project, 16 native
UInt32/UInt64 arithmetic/bit/shift controls and byte length elaborated by
reflexivity. Byte index at valid/invalid offsets and byte slice failed with
`LLV7002` because imported expressions were not definitionally equal to the
expected literals. A separate fixed-word-only module retained the 16 word
statements while excluding byte operations. Its theorem elaboration completed,
but verification rejected the first audited theorem, `checked32Sum`, with
`LLV7005`: observed axioms `[Quot.sound, propext]` violate the none policy.
There is no accepted native-word attestation. This result does not establish
which remaining primitive wrappers share that axiom dependency because the
audit stopped at its first failure.

The exact diagnostic source and command outputs are preserved in the isolated
project and `/tmp/lexlean-r4-blake3-evidence-20261021`. This historical failure
led to the independent compiler diagnosis and pure Nat implementation below.
Neither proof-policy relaxation nor a native hash fallback is permitted.
The complete published-vector inventory, semantic mutation checks, Rust
acceptance and tokenizer composition remain required.

## Independent primitive isolation

Further isolated diagnostics separate the failure classes. Four UInt32/UInt64
literal and generic identity probes passed. Each of sixteen independently
isolated checked-arithmetic, conversion, bitwise or shift definitions failed
`LLV7005` with `Quot.sound` and `propext`. Ordinary semantic `Add` on UInt32
failed type checking with `LLT4001`; its type is Nat. The generated generic
`Fixed` instance combines conversion and bit-operation dependencies, so this
does not identify a unique Lean leaf declaration as the cause. No native-word
wrapper is accepted by these diagnostics.

Eleven separate Nat probes passed the unchanged none-axiom policy: four
generic operation dependency checks and seven numeric quotient, remainder,
subtraction, multiplication and modulo-2^32 wrapping controls. In particular,
MAX+1 wraps to zero, two MAX terms to MAX-1 and three MAX terms to MAX-2.
The seven identical numeric statements all failed imported `reflexivity`
under language 1.1 with `LLV7002`. This is a separate reduction limitation;
it is not the native-word axiom failure.

The retained results are under host paths
`/tmp/fixedword-diagnostic-results/tmp/` and
`/tmp/nat-diagnostic-results/tmp/`. The latter includes setup diagnostics
separately from the successful probes. Representative numeric receipts are
`7a22444b84e157f792b71e5dce492f00a022c0ded5f16e72eb777b252becf4cb`
(quotient) and
`771462f470946cd1780c025370386ee9f4e9483e432911f412963ad2036d2eda`
(wrapping carry). No BLAKE3 algorithm or speed claim follows.

Exact upstream backend inspection identifies an existing supported exposure
path: `portable_runtime_1_2` adds `expose` to public runtime definitions while
retaining `noinline`, and the renderer selects it for
`lexlean/semantic-module/2`. Changing project language alone while retaining
semantic-module/1 is insufficient. An isolated official language-1.2/spec-2
diagnostic and imported exact-certificate `Apply` diagnostic subsequently
passed. All seven original numeric statements passed imported `reflexivity`
under language 1.2/spec 2; exact certificate application passed under both
1.1 and 1.2. Each result has 21 empty-axiom declaration audits, eight successful
independent kernel replays and ten independently checked manifest input files.

- 1.2 imported reflexivity: `2e5bcc56a7f9399595e9c7fa8e6abb02056ef95fcf321a119f57bfee342d42af`
- 1.1 imported Apply: `35a53ff5d37be6c0360662c2fc1783d8e5023268d29785cd986d38da3e5b626b`
- 1.2 imported Apply: `06007401f72145d50cf535b11353b29b63e081b06947c3999b80a053b698b112`

The isolated 1.2 lock records compiler semantics
`09f4b95d5994ca00acd413e6f9e8381c8a18d4fd778249d2c8654f7dc99dce86`.
Migration also requires the officially matching builtin glossary versions.
The parent independently compared every declaration in the twelve migrated
diagnostic modules with its original: all operation bodies and theorem
statements are unchanged. Apply controls are separately authored proofs of
the same numeric statements, not reflexive self-equalities. Full evidence is
retained at `/tmp/lexlean-r4-nat-formal-evidence-20261021`.

This establishes a supported arithmetic path for the next BLAKE3 implementation
investigation. It does not establish BLAKE3, a project migration or a production
SDK. Those diagnostics did not change the application project language or
generated Lean; application migration is a separate parent integration gate.

## Isolated pure Nat implementation, 2026-10-21

The development-only language-1.2 implementation is retained at
`/tmp/lexlean-r4-blake3-nat-20261021`, independently of the parent application
migration. Its four source modules contain 91 declarations: the exact
Nat word/byte representation, compression, complete streaming semantics, and
a generic compact literal-input container. Full public conformance remains
pending until all published vectors, additional controls, mutation checks,
independent review, and the parent acceptance gate complete.

The original 125 primitive controls were frozen before definitions
(SHA-256 `c53bf66b28d2be6a2978aa04b69962d9bc36fde7dac5887b860ee679a625e62f`).
Missing-module `LLR3005` is retained as the genuine RED. They subsequently
passed under the unchanged none-axiom policy with attestation
`360c1eafeaef9ff7eaa9074641962b22a86d524aff46ba1bec0ebd9e1499907e`.
Independent receipt inspection checked all 143 declarations have empty
observed axiom sets, both module kernel replays succeeded, and every
nonlexicon manifest input matches the actual bytes. Nat source SHA-256 is
`91df70496689c63732341fad46abd39a3c91f8d756db0665136f789b74e9a68b`.

The 42 compression-layer controls were likewise frozen before definitions
(SHA-256 `b4a6b5a5d3978f90828b4532e5dea0a3e8cda2035ce5454b1080b7dbf213d80d`)
and observed missing-module RED. They include the independent first-round
32-bit state from [RFC 7693 Appendix B](https://www.rfc-editor.org/rfc/rfc7693.txt)
(shared G operations and identity message schedule only), the complete
message permutation, typed admission failures, high/low counter splitting,
and all 16 output words of the official BLAKE3 empty-message XOF block.
The RFC control does not substitute BLAKE2 hashing for BLAKE3. The empty
compression control executes all seven BLAKE3 rounds and its actual initial
state and feed-forward.

These 42 controls passed with attestation
`0e092bdd94e4223c509d0a13794f3d3de4ad2e425226e8523d0808c88821a9c7`.
Independent inspection checked 82 empty-axiom declarations, all three module
kernel replays, and five exact nonlexicon manifest inputs. Core source SHA-256
is `64242d1e99d648b420f33ca190f949308a151b1e52eb6a58af559f3d8561c6a0`.
These receipts establish primitive and compression controls only, not the
complete public streaming hash or any runtime performance result.

The complete 35 published messages are represented by the reviewed generic
literal tree described in design section 10. A separate static decoder checked
all 225899 input bytes across the cases and every unchanged expected digest.
The largest theorem file is 687618 bytes. Streaming controls preserve the
original long-list statements via balanced ordinary append syntax; the
original source-admission recursion failure and full expansion comparison are
retained separately. No proof/resource limit changed. Source checks have
passed for streaming, literal input, and the largest published case before
their formal verification began. No host BLAKE3 implementation ran to
supply expected answers.

The 48 original streaming statements subsequently passed with attestation
`0b87d7c9a496446ffd96de3ac620dc4f236cc3baf0d26f4ada49b3c66cd4cf38`.
The proof covers ordinary-list published digests through 129 bytes, partitioned
hashing at 64/65/128/1024/1025 bytes, pending-full-block behavior, and ordered
state/length/budget/byte admission. The independent receipt audit checked all
129 declarations have empty observed axiom sets, all four module kernel
replays succeeded, and all six nonlexicon inputs match their manifest hashes.
The balanced streaming fixture SHA-256 is
`ed8cef63cec1f587c44243abe65407b2cce691587a4a87325a69f0f1c146c970`.

Before that success, source checking had accepted a recursive list scrutinee
reference that the backend emitted as an unbound Lean variable. The failed
source and `lean.unknownIdentifier` result are preserved. The exact repair is
one expression in the `List.cons left tail` branch of `blake3MergeStack`:
reconstruct `cons(left,tail)` instead of referring to the whole `stack` binder.
A complete AST comparison confirms no other source change. This is a compiler
emission workaround with unchanged semantics and expected controls, not a
semantic negative or a relaxed acceptance gate. Streaming source SHA-256 after
the repair is `a940ced45024e3b3c73da5ec373c27a57afd9abdbd0f1e87ff52dbbd0878754e`.

The current finite inventory has 272 controls: 125 primitives, 42 compression,
48 streaming, 13 literal-container admissions, three digest codecs, two
repeated-finalize/resume checks, four maximum-domain checks, and 35 original
published literal-tree digests. All 237 additional controls and the first
18 published cases have passed: 255 of 272. The remaining 17 published
cases, nine staged semantic mutants, restored positive verification and
the complete BLAKE3 Rust integration gate remain pending.
The new maximum-domain checks include actual public update overflow on a
structurally valid u64-maximum state, without claiming that a caller-supplied
state authenticates any prefix history.

The remaining 22 small controls passed without source or fixture changes:

| Controls | Attestation | Empty-axiom declarations | Successful module replays |
| --- | --- | ---: | ---: |
| Literal input (13) | `f17e7af9611d4f3bd3de710eb3a4d8d67393862b050eab459b30e2e24978ebf8` | 104 | 5 |
| Digest encoding (3) | `336660bc9ad9ce5640ca879e6ab3678bda475dea3e9a9c7d09e8bfc12608a1bd` | 84 | 4 |
| Finalize/resume (2) | `3f00204bb4beb86ff25e8ab893f892ee9eeb56ab733fb1863033900429973de8` | 83 | 4 |
| Maximum domains (4) | `92981d1dd47f3c2e78c0004cf31f748f1f0a7bc9d699fd43ed5e1c49fe169aa7` | 95 | 5 |

Each retained receipt was inspected for the none-axiom policy, empty observed
axiom sets, exact module replay coverage, attestation/build identity, and
every nonlexicon manifest input's actual byte length and SHA-256. These are
isolated language-1.2 formal checks; complete published-vector acceptance,
mutation checks, BLAKE3 Rust integration and production lowering remain
unfinished. The parent application migration is a separate gate.

The first nine published literal-tree cases also passed, with every input and
expected digest unchanged. Each receipt has 92 empty-axiom declarations, five
successful module kernel replays, and seven nonlexicon inputs independently
checked against their actual byte lengths and SHA-256 values.

| Published input length | Attestation |
| ---: | --- |
| 0 | `adb8b4bbe234596a7f911ed1b45691a68461af452bc5f81cabec845c67b60632` |
| 1 | `388747d21a51b843a664929dbefaeba17c6c95f93c576bef707e45cfbfb898f3` |
| 2 | `74f0a053789107b4118747e47b5ec85fca8fbfe89911436959da4165e9471033` |
| 3 | `8875434b4fef450df8855acb9834ae2cefb8d832a323def1dd6b81bb3419e3c1` |
| 4 | `5b99c35ced580f8fe18cc6cdd6d7a191f721a5254d4baeb5934b5a2bc3c7b8e8` |
| 5 | `88fbd05144a4503961519016e8cceed3fede69401889249d875be79697f38ff4` |
| 6 | `27fdad23926569b6a8d3be676a29ba68a194ed63c5a755750c66ad6305af7a5a` |
| 7 | `b0be87ef75803482303054e3a99fa89c883fa573ec4662071f7f4b390cda8a72` |
| 8 | `a7faf80fdadd907d31b5192f075b0f86fbbee67b8c0d913483bd2a41a2535eb6` |

The next six published cases passed at the exact block-boundary lengths below.
Each independently audited receipt again has 92 empty-axiom declarations,
five successful module kernel replays, seven matching nonlexicon inputs, and
25 matching retained build outputs. Source and original expected digests did
not change.

| Published input length | Attestation |
| ---: | --- |
| 63 | `a3a6c95fc377cf296af5f6379251888666dd8acd88a4a48ed284e61cfc3ee89f` |
| 64 | `be2fd053ede3ed77d94a9cf0f525e09024dd6994d07d8e4b9cff6c8820c67325` |
| 65 | `ef0ec8aa928fab859414c7ffab9bdc4e63e431acdd3e8b2f1c21488724f6f46d` |
| 127 | `786f2aee0d94f676acda173eb0b00456aae4987916c0d31a9b5ca4a1b76bedac` |
| 128 | `71251847a75ea65cd503519bb70c82edc930018c0462480b37c47e4e0da599e7` |
| 129 | `3d630a82e58353d4c88356a575b1f3aa3cf33be21f2be843a936edacf24eb434` |

## Chunk-boundary receipts and retained evidence

Published cases16–18 passed without source, input, expected-digest or proof-policy
changes. These are the original complete digest statements at the first chunk
boundary, not intermediate compression observations.

| Case | Published input bytes | Verification wall time | Attestation |
| --- | ---: | ---: | --- |
| 16 | 1023 | 91.97 s | `d8e57c00ef77a586bfb165d81c58f0a71708a486e6eeefbc609bb81f52aedbaa` |
| 17 | 1024 | 75.21 s | `605c4231d22476de2d9c2ea1d6d93f10d4e7aa1aa458ab805d3b3b51e8264c3d` |
| 18 | 1025 | 89.92 s | `f33c89213f6e1204417c7a2dfd7af10073bf5bf44bc86637614352dbc3f2411d` |

Each receipt contains 92 declarations with the none-axiom policy and empty
observed axiom sets, five successful kernel replays, seven exact nonlexicon
input bindings, 25 retained build-output bindings, and five checked compiled
module hashes. The recorded wall times describe formal verification commands;
they are not hashing throughput or serving measurements. Compiler executable
SHA-256 is `107e02328a8052d1560d6e8addcfd1c70af6145bbf836650cac32dc044f550e5`,
with the official language-1.2 semantics `09f4b95d5994ca00acd413e6f9e8381c8a18d4fd778249d2c8654f7dc99dce86`.

Retained local evidence is under
`/Users/casey.allard/lexlean-r4-evidence/2026-10-21/blake3-codebook/`.
Original scratch files remain in place. The original preservation manifest
is unchanged; cases16–18 are an append-only addendum.

| Retained manifest | SHA-256 | Scope |
| --- | --- | --- |
| `preservation-manifest.json` | `a99b0f8af5cb988e8cdcc58e64c773db388a9e37a7b8b592f99614ba16a68134` | Earlier candidate/control bytes, receipts, original representations, mutation staging and logs; preservation only |
| `dependency-diagnostics/manifest.json` | `86e22db20c3d48f67b37442b64264afcbaa830c9dd6887f30c397bfc33711eeb` | Native-word and language-exposure diagnostics, including retained failures |
| `cases16-18/manifest.json` | `6bf1eed35a3885f518dd10b98547bf3ac1f5c03b3a54bd5ffaff8e48dbcd1c8f` | 186 files, 8726283 bytes: exact sources/controls/configuration/lock, all three attestations, manifests, replay outputs and receipt audits |

All preserved copies were reread against their original SHA-256 and byte
lengths. Preservation is evidence custody, not an additional conformance result.
The parent source review and full declaration/reference map are recorded in
`specs/blake3-source-review-map.md`; this author-maintained evidence update
is not a substitute for independent acceptance review.

The outstanding published input lengths are
`2048,2049,3072,3073,4096,4097,5120,5121,6144,6145,7168,7169,8192,8193,16384,31744,102400`.
All 17 retain their original complete messages and digest expectations. The
nine staged mutants have not been executed: dropped ROOT, nonzero ROOT output
counter, padded block length, extra empty chunk, block-based chunk counter,
parent CV substitution, reversed children, wrong permutation, and BE words.
Each still needs wellformed source admission, failure of the unchanged
intended equality, and restored positive verification. Full inventory/Rust
integration and tokenizer identity composition remain required. No production
artifact, generated serving root, GPU parity, browser execution or performance
claim follows from the current finite formal results.
