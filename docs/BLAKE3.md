# BLAKE3 implementation evidence

R4-15 is registered for the complete unkeyed BLAKE3-256 algorithm described in
`specs/blake3-design.md`. Implementation is in progress. No BLAKE3 digest,
streaming behavior or tokenizer identity is accepted yet.

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
project and `/tmp/lexlean-r4-blake3-evidence-20261021`. The next step is independent
compiler diagnosis before choosing a source-authored alternative such as the
already admitted Boolean word operations. Neither proof-policy relaxation nor
a native hash fallback is permitted. Full BLAKE3 definitions, all 35 complete
public digest proofs, streaming controls, semantic defects, Rust acceptance
and tokenizer composition remain outstanding.

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
SDK. The application project language and generated Lean remain unchanged.
