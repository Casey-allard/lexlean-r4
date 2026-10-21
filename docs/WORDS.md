# Fixed cryptographic words

`R4.Words` supplies the pure LexLean word operations needed by the planned
SHA-256 and BLAKE2b-96 definitions. R4-09 models fixed 32-bit and 64-bit words,
wrapping addition, Boolean operations, shifts, rotations and byte codecs.
It does not implement a digest, evaluate a model, or qualify a production
runtime. All computation is defined in the authoritative LexLean source;
the Rust integration test verifies proofs and receipts only.

## Authority and representation

The upstream dependency is `crates/uor-r4-object-store/src/hash.rs` at
`4611538bb4843947363ccb9cce5f388e32c46883`, SHA-256
`f82cc1b4d4ef9ed2f0f0b933a9c8009611fe774de3666299a5f977fc0ed94b5f`.
The cited BLAKE2b construction requires 64-bit word operations. Planned generic
SHA-256 support additionally requires 32-bit word operations. BLAKE2b configures
a twelve-byte digest; truncating a separately configured 64-byte digest is
not equivalent. Digest rounds, padding, counters, domain separation and
published complete digest vectors remain future obligations documented in
`docs/specs/memory-protocol-design.md`.

`Byte8` contains exactly eight Boolean fields, `b0` through `b7`, with `b0`
least significant. `Word32` contains four such bytes, `a0` through `a3`;
`Word64` contains eight, `a0` through `a7`. Byte `a0` is least significant.
Consequently every inhabitant is already within its unsigned word domain.
There are no extra bits, signed cases or invalid word representations.

The public numeric input boundary is a list of natural-valued bytes. Each
byte must be in 0..255. This slice does not expose an arbitrary-natural to
word conversion or silently truncate a natural. Word literals are fixed
Boolean records; encoded outputs are natural-valued byte lists.

## Operations and totality

`addBit` is a full adder. Eight explicit stages carry from bit 0 to bit 7,
then four or eight byte stages carry from `a0` upward. `add32` and `add64`
discard only the final word carry, giving unsigned addition modulo 2^32
and 2^64 respectively. The intermediate sum structures retain their carry
for the following stage. `and32/64`, `or32/64`, `xor32/64` and `not32/64`
operate field by field.

The one-bit permutation for `rotateRight32/64` sends input bit 0 to the
highest output bit and input bit i+1 to output bit i. Logical right shift
uses the same gather except its highest output bit is false. Logical left
shift supplies false at bit 0 and input bit i-1 at output bit i. Structural
recursion repeats that exact one-bit operation the requested number of
times.

The checked rotation and shift APIs admit amounts 0..31 for Word32 and
0..63 for Word64. Amount zero is identity; an amount equal to the width or
larger returns `invalidAmount(width, amount)` before any repeated operation.
This API deliberately does not normalize counts modulo the width. Future
hash rounds use admitted fixed constants. Its work is bounded by 31 or 63
one-bit permutations even when the rejected natural is extremely large.
The internal structural step helpers are total on every natural amount;
calling them directly bypasses this resource bound.

`encodeByte` sums the eight fixed powers of two selected by the Boolean
fields. `decodeByte` first checks the byte domain, then tests each power of
two from 128 down to 1 and subtracts it when present. The subtraction is
source-defined truncated natural subtraction, with no opaque SDK primitive.
It uses sixteen-predecessor chunks with a separate structurally decreasing
fuel argument so the pinned compiler accepts termination. Residual counts
0..15 use the complete linear subtraction helper. Zero fuel also computes
the complete remaining subtraction; it never returns a guessed value.
`subtractSmall` supplies the count as fuel. Public byte decoding therefore
has a fixed, small bound independent of an invalid input's magnitude.

The first linear subtraction implementation exceeded the development Lean
normalizer's default recursion depth on admitted byte controls. Chunking
fixed the source reduction depth while preserving the expectations and
normalizer settings. No generated Lean, compiler helper, recursion limit,
axiom list or theorem expectation was changed to obtain acceptance.

## Exact codecs and errors

`encode32LE/BE`, `encode64LE/BE` return four or eight bytes respectively.
Little-endian output visits `a0` first; big-endian output visits the highest
byte first. Decoding enforces exact list length before inspecting bytes.
It then consumes bytes in their original input order, assigning each byte
to the appropriate destination field. Thus both endian variants report
the first invalid byte's original zero-based position.

| Error | Meaning and precedence |
| --- | --- |
| `invalidLength(expected, actual)` | The whole input has a short or trailing byte count; precedes byte validation. |
| `invalidByte(index, value)` | An input byte exceeds 255; the earliest original input position wins. |
| `invalidAmount(width, amount)` | A shift or rotation amount is outside 0..width-1. |

There is no default for absent bytes. Internal parser steps also return a
typed length error for premature exhaustion or trailing bytes. Public codec
length checking traverses the supplied finite list to report its exact
length; callers imposing a wire-size policy must enforce it at their framing
boundary. This API does not itself impose an arbitrary transport-size cap.

Independent literal codec examples are:

| Word | Little-endian bytes | Big-endian bytes |
| --- | --- | --- |
| `0x12345678` | `78 56 34 12` | `12 34 56 78` |
| `0x0123456789abcdef` | `ef cd ab 89 67 45 23 01` | `01 23 45 67 89 ab cd ef` |

## Controls and observed verification

R4-09 and its scenario preceded source implementation. The initial frozen
controls failed checking with `LLR3005` because `R4.Words` did not yet exist.
The subtraction repair's residual controls separately failed with `LLT4001`
before the new helper was implemented.

The final 995 theorem controls cover all 256 byte decodings and encodings;
all eight full-adder truth cases; each word carry boundary; zero, high carry,
maximum-plus-one and maximum-plus-maximum addition; mixed Boolean patterns;
every single-bit basis vector for one-bit rotation and both logical shifts;
zero and maximal admitted shifts; SHA/BLAKE rotation constants; rejected
width and huge counts; independent literal endian encodings and decodings;
short and trailing lengths; every invalid byte position; length-before-byte
and first-invalid-byte priority; and residual subtraction counts 0..17 with
fuel 0/1 and both saturating and nonsaturating left operands.

Two defects were planted in the actual source with the controls held fixed:

1. Replacing full-adder carry with false caused 18 `LLV7002` proof failures,
   including carry propagation and full-adder controls.
2. Giving `encode64LE` the big-endian body caused `LLV7002` at its independent
   little-endian literal control.

Both verification commands exited 1 with no accepted attestation. Source
bytes were restored after each defect. The restored command
`cargo test --offline --locked -p repo-conformance --test words` then passed
its single `fixed_word_algebra_r4_09` integration test. The shared verifier
freezes all control statements, requires every theorem's empty axiom audit,
checks fresh source/configuration/lock and build-manifest identities, and
replays the generated Lean modules independently.

The final development receipt is:

- Attestation: `4a775a3c01c940ddbe0dae3933d015b9349042e007eaa1446ba2c271a233ba42`
- Build: `11bd7ec078b45362782de087d363ab5d222f5f1e31676b129b6e164cefcdf2c1`
- Source: `9bd79c5ed59d72b647c4e5ab0bc175f881dd92c729bfff6bc8311b729c50b348`
- Semantic: `9a50da36f7528cc86a5f450e4811151bd9495eed9a60e3f9bec0b4548f5b7cc4`
- Words source SHA-256: `894f42762f6e02755e6b020edd4c5a39fb08fef871e8ff5145e798c8cf41810e`
- Controls SHA-256: `27d615ee3f8887ecfcaf88ae14a6fbd768a20cd87460a0546ded000147fa0ba6`

Verification used the pinned development SDK candidate
`ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:60226bc791d4c0e5613402a6be7e63f4963d3faf7f327befcf56fc0e41d0ce21`,
arm64 child digest
`sha256:2f82a04e8f8141636e03a219d3960e30d98a6f304828341dd47b553b3c23baeb`,
containing LexLean revision `01088ef00d9d0d0227775e0346a852c2d19cb67b`.
The 995 finite controls are observed proof evidence, not a universal algebraic
correctness theorem or complete hash conformance result. Production SDK and
template binding, resource acceptance for complete hashes, generated runtime
integration, GPU research and browser delivery remain separate obligations.
