# Tokenizer JSON number admission

Status: independently approved refinement of the tokenizer contract,
2026-10-21. Source and arithmetic review covered the exact parser branches,
numeric observation quotient and overflow midpoint. Controls and definitions
remain pending. This
component implements the standalone serde_json 1.0.150 default profile's
number admission and the numeric distinctions actually consumed by the pinned
tokenizer. It does not introduce model floating-point operations or a generic
JSON numeric evaluation service.

## Source and observable contract

Authority is serde_json 1.0.150 `src/de.rs`, SHA-256
`4998d7e252eb513be85b298e287048b4fb283882e9ba7df905f572f07200f3f3`,
with `std` and without `float_roundtrip`, `arbitrary_precision`,
`preserve_order`, or `unbounded_depth`. The source tokenizer revision remains
`4611538bb4843947363ccb9cce5f388e32c46883`. The admitted profile must be bound
in tokenizer identity/composition; Cargo.lock alone does not select it.

All tokenizer numeric field reads use `Value::as_u64`; nonnumeric fields use
string, Boolean, object or array observations. Therefore this parser preserves
an explicit numeric class and raw lexeme, with an exact unsigned value only
for a source `U64`. Negative integer and floating representations never become
unsigned IDs, including negative zero, `0.0`, `0e0`, and positive integer
syntax that overflowed u64 before floating fallback. The JSON layer may use
three numeric cases: `unsigned(Nat)`, `negativeInteger(Nat magnitude)`, and
`finiteOther`. It must retain original input bytes for raw tokenizer identity.
This is an observation-preserving tokenizer-specific JSON representation; it
is not a claim to reproduce the numeric values of arbitrary JSON consumers.

The parser takes an admitted byte list, a cursor, and an explicit input-capacity
profile. It returns a numeric class and first unconsumed cursor, or a typed
syntax/range/capacity error. The surrounding JSON parser validates delimiters
and complete consumption. The byte and cursor domain checks cannot be skipped
by a public entry point. Parse the entire value, including overwritten or
ignored metadata, before applying object-key last-wins semantics.

## Integer scanning and representation

The optional sign is ASCII `-`; `+` at the start is invalid. A single leading
zero is allowed, but a following decimal digit before a decimal point or
exponent is invalid. At least one integer digit is required. Accumulate into
an exact Nat while checking each step against `18446744073709551615`.

On the first overflowing decimal digit, preserve the last nonoverflowed u64
significand, leave that digit to the long-integer path, and increment the
starting decimal exponent for every remaining integer digit. Discarded digits
still undergo lexical scanning. They never get rounded back into the stored
significand: this is the selected source profile's truncation behavior.

Without a decimal point or exponent, nonnegative nonoverflowed syntax yields
`unsigned`. Negative syntax with magnitude from 1 through 9223372036854775808
inclusive yields `negativeInteger`. Negative zero or a greater u64 magnitude
yields `finiteOther`, matching the source's wrapping-negation test. A long
integer always yields `finiteOther` if its finite-number admission succeeds.

## Fraction and exponent scanning

After `.` require at least one decimal digit. Continue appending fraction digits
to the same u64 significand while they fit, decrementing the decimal exponent
for every accepted digit. Once a digit would overflow, ignore all remaining
fraction digits without decrementing the exponent. Combine the number of
omitted integer digits with the negative count of retained fraction digits.
This combination is the starting exponent passed to source `parse_exponent`.

After `e` or `E`, accept an optional `+` or `-`, then require at least one digit.
Accumulate exponent magnitude with an exact i32-positive bound, 2147483647.
If the next digit overflows that bound, follow `parse_exponent_overflow`
before combining with the starting exponent: a nonzero significand and positive
exponent fail with `numberOutOfRange` immediately after consuming the triggering
overflow digit; zero significand or a negative exponent is finite after scanning
the rest of the exponent digits. Preserve this cursor distinction in controls. A large positive
exponent is not canceled by a negative starting exponent after this overflow.

Otherwise combine signed exponent and starting exponent with source i32
saturating addition/subtraction. Represent signed exponents by explicit sign
and natural magnitude, or another exact admitted source representation. Do not
use host floating-point operations. The source's unchecked digit-count i32
counters are restricted by an explicit application input limit of at most
2147483647 bytes; larger inputs return `capacity`. This is a named bounded
application profile, not a claim about source behavior on multi-gigabyte
counter overflow. The exact application budget may be smaller and is bound
as an input, never silently hardcoded into a source-compatibility claim.

## Exact finite-number admission using integers

For the selected profile, preserving finite admission does not require
computing an unused floating-point value. Source `f64_from_parts` starts by
rounding the u64 significand to binary64. A nonnegative exponent selects a
binary64 power-of-ten literal from a table covering 0 through 308 and multiplies
once, rejecting infinity. Negative exponents only divide finite nonnegative
values by finite positive powers, so their result stays finite, including
underflow to zero. If the power index is absent, source first accepts a zero
significand, then rejects a positive exponent or repeatedly divides for a
negative exponent. These ordered cases must remain explicit.

Define `roundInteger53(n)` with exact naturals. If the bit length is at most
53, return n. Otherwise let k be bitLength(n)-53, q = n div 2^k and
r = n mod 2^k. Increment q exactly when 2*r > 2^k, or when 2*r = 2^k and q is
odd. Return the rounded q times 2^k. A carry to a new exponent is retained.
This is round-to-nearest, ties-to-even for a nonnegative integer representable
in the finite range needed here.

Let S = roundInteger53(significand), and for 0 <= e <= 308 let
P(e) = roundInteger53(10^e). Powers in this range are finite binary64 values
and have integer values, so P describes the source's exactly rounded decimal
literal without a host float table. The overflow midpoint is

    T = 2^1024 - 2^970.

For a positive-exponent multiplication, finite admission is exactly

    S * P(e) < T.

Equality overflows: the midpoint ties to the upper, even significand at
2^1024, which is infinity at the binary64 exponent boundary. This inequality
must be independently reviewed and controlled at T-1, T and T+1. No decimal
exponent heuristic or approximate maximum-number comparison may replace it.
For exponent zero the same rule is valid. For negative exponents admission is
finite; no rounded value is exposed by this component. A zero significand is
finite for every exponent, including the overflow path above.

All arithmetic in this admission definition is Nat arithmetic in LexLean.
The required bit-length, quotient/remainder, bounded exponentiation and
rounding definitions must have termination measures and none-axiom controls.
Big natural intermediates are a formal/load-time requirement. The current
production exporter lowers Nat to checked u64, so generated production
acceptance needs a reviewed multiword/table lowering or a compatible generic
SDK capability. Formal verification must not be described as resolving that
export dependency.

## Controls required before definitions

Freeze independent literals and classifications for ordinary integers,
0, -0, both signed-64 boundaries, both unsigned-64 boundaries, unsigned overflow,
decimal/exponent zero, leading-zero refusal, missing fraction/exponent digits,
explicit exponent signs, trailing delimiters and complete-consumption errors.
Keep lexer offsets and failure categories separate from upstream's Option
constructor result, which exposes no detailed tokenizer error ordering.

Arithmetic controls include exact and inexact conversion at 2^53, both tie
parities, significand carry, the rounding of u64::MAX, every source power-of-ten
literal or an independently reviewed complete construction of those values,
and the exact overflow midpoint. Use published integer/binary64 facts or
independent exact-integer derivation; do not manufacture expectations by
running the candidate implementation.

Number controls include `1e308`, `1e309`, `0e9999`, `1e-9999`, exponent magnitude
2147483647 and 2147483648 in both signs, exponent overflow with zero significand,
long integer truncation, fraction truncation, a large positive exponent with a
negative starting exponent, and values immediately on both sides of the source
profile's finite overflow boundary. Record the actual rounded-factor boundary,
not an assumed correctly-rounded-decimal parser boundary.

The JSON integration controls must include a duplicate key whose first value
is `1e9999`: it fails even if the last value would otherwise replace it. The
same pattern with a wrong-typed but finite first value is accepted after
replacement. Numeric metadata remains finite-validated even when ignored.
Plant unsigned acceptance of float syntax, first-overflow digit loss,
rounding-tie inversion, exponent cancellation after overflow, and acceptance
of midpoint infinity. Each plant must fail an unchanged semantic control.

Independent review must confirm the observation quotient, all source branches,
finite-product proof, exact input bounds and production limitation before
control generation or implementation. Complete JSON syntax, strict string
UTF-8/escapes, depth-127 container admission, duplicate object-key resolution,
typed kernel extraction and raw BLAKE3 identity remain additional components.

## Arithmetic references

The [Rust numeric-cast reference](https://doc.rust-lang.org/reference/expressions/operator-expr.html#numeric-cast)
specifies nearest, ties-to-even integer-to-float conversion. The
[Rust f64 arithmetic contract](https://doc.rust-lang.org/std/primitive.f64.html)
specifies the same rounding direction for primitive arithmetic. The binary64
overflow midpoint is independently described by
[Oracle's Double conversion contract](https://docs.oracle.com/en/java/javase/24/docs/api/java.base/java/lang/Double.html#valueOf(java.lang.String))
as maximum finite plus half its unit in the last place. Substituting maximum
finite = 2^1024 - 2^971 and its unit = 2^971 derives T above. The proposed
integer product predicate is our inference from these arithmetic contracts and
the pinned serde_json branch structure; independent source/arithmetical review
and LexLean controls are still required. No Java runtime is a dependency.
