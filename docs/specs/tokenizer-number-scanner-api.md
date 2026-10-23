# Tokenizer number scanner API

Status: parent-reviewed and approved API refinement of the independently
approved [number admission and arithmetic contract](tokenizer-number-admission.md),
2026-10-21. Source-branch, consumed-cursor and boundary-order review found no
contract discrepancy. Independently frozen controls and missing-definition RED
precede scanner source.
The scanner is `R4.JsonNumber` and uses the separately controlled
`R4.JsonNumberArithmetic` API. This is the tokenizer's numeric observation
quotient, not a general JSON parser or a generic floating-point evaluator.

## Exact source profile and branch authority

Authority is the local source of serde_json 1.0.150, checked against these pins:

| File | SHA-256 |
| --- | --- |
| `src/de.rs` | `4998d7e252eb513be85b298e287048b4fb283882e9ba7df905f572f07200f3f3` |
| `src/read.rs` | `17a2efe49e5a6937c8d7d4a5c0b9eac84453de0f86bf8e7ee421387e82341bb3` |

The source profile has `std` and excludes `float_roundtrip`,
`arbitrary_precision`, `preserve_order` and `unbounded_depth`. The source
package's default feature is `std`; documentation/playground feature lists
are not this profile. The selected feature closure must later be bound in
raw-tokenizer composition, rather than inferred from Cargo.lock alone.

The relevant `de.rs` source locations are: byte reader delegates at 219–232;
`parse_integer` at 462–507; `parse_number` at 509–528; `parse_decimal` at
530–565; `parse_exponent` at 567–621; default `f64_from_parts` at 639–672;
default `parse_long_integer` at 717–739; default `parse_decimal_overflow` at
837–853; and `parse_exponent_overflow` at 859–879. Sign dispatch is visible at
1403–1424 and in the prefix of `parse_any_signed_number` at 899–914. The
non-arbitrary-precision `parse_any_number` directly delegates to `parse_integer`
at 933–935. `SliceRead::next` at `read.rs:545` consumes an existing byte but
not EOF; `peek` at 558 does not consume a byte. The port exposes consumed-byte
cursors, not serde_json's line/column presentation or its `error` versus
`peek_error` position formatting.

The later remainder check in `parse_any_signed_number` is **not** the prefix
scanner contract: that helper can replace a prior error when a suffix remains.
The complete-lexeme wrapper below explicitly preserves the prefix scanner's
first failure and checks trailing bytes only after success. This is a named
wrapper policy, not claimed equality with that internal helper's diagnostics.

## Public values and exact boundary order

`JsonNumberProfile` has `maxInputBytes : Nat`. Its admitted range is
`0..2147483647`. This is the named bounded application profile required by the
approved design; it prevents unchecked source i32 digit-count overflow without
claiming source parity beyond that range. A smaller caller limit is preserved.

`jsonScanNumber(profile, input : List Nat, start : Nat)` scans at an absolute,
zero-based byte cursor. It does not skip whitespace. Public boundary validation
always performs these global checks in order, before any lexical parsing:

1. `maxInputBytes <= 2147483647`, otherwise `invalidCapacity`.
2. The actual entire input-list length is at most `maxInputBytes`, otherwise
   `capacity`. Count with exact Nat, including bytes before `start` and suffixes.
3. Every input element is in `0..255`, in list order, otherwise
   `invalidByte(index)` for the first offending absolute index. This includes
   bytes outside the eventual numeric lexeme.
4. `start <= input length`, otherwise `invalidCursor`. Equality is admitted and
   enters the scanner at EOF. The preceding bounds also bound any valid cursor.

These are representation/application errors, not invented serde_json syntax
errors. In-domain arbitrary suffix bytes, including NUL and bytes above ASCII,
remain admitted scanner input; this component does not impose UTF-8 validation
on a whole JSON document. Numeric syntax itself is ASCII. External JSON syntax
and string UTF-8 validation remain separate required components.

`JsonNumberClass` is closed:

- `unsigned(value : Nat)`: source `ParserNumber::U64`, in `0..18446744073709551615`.
- `negativeInteger(magnitude : Nat)`: source `ParserNumber::I64`, with magnitude
  in `1..9223372036854775808`; this stores magnitude rather than a signed host value.
- `finiteOther`: a source finite floating representation; its value is not exposed.

`JsonNumberScan` contains `number : JsonNumberClass`, `start : Nat`, `next : Nat`,
`negative : Bool`, `longInteger : Bool`, `hasFraction : Bool`,
`hasExponent : Bool`, and `bareNegativeZero : Bool`. These tags are derived by
the scanner; no caller-supplied flag selects classification. `negative` records
the initial minus sign. `longInteger` means integer accumulation overflowed u64
and entered the source long-integer fallback. Fraction overflow alone does not
set it. The two syntax flags record an encountered decimal point/exponent marker
on a successful scan. `bareNegativeZero` is true exactly for successful bare
integer syntax `-0`, without a fraction or exponent. It is not a claim about
underflowed float values or every syntax that would produce negative float zero.

The exact raw lexeme is the input slice `[start,next)`. The caller retains the
original input; the scanner never reserializes, normalizes or hashes it. This
preserves raw tokenizer identity independently of the number observation.
A `finiteOther` value can never be read back as an unsigned ID by this API.

`JsonNumberError` is closed: `invalidCapacity`, `capacity`,
`invalidByte(index : Nat)`, `invalidCursor`, `unexpectedEnd`, `invalidNumber`,
`numberOutOfRange`, `trailingBytes`, and
`arithmeticError(JsonNumberArithmeticError)`.
`JsonNumberResult` is `scanned(JsonNumberScan)` or
`error(JsonNumberError, cursor : Nat)`.

For the four pre-parse boundary error categories, the returned cursor is the
original `start` argument: no scanning occurred, and that value need not itself
be an admitted cursor. An invalid byte carries its separate absolute index.
For syntax/range errors, the cursor is exactly the first unconsumed position
when the source branch fails, after all already-consumed bytes. EOF does not
advance it. An error has no successful scan or partial numeric class.
`unexpectedEnd` corresponds to numeric `EofWhileParsingValue`, `invalidNumber`
to numeric `InvalidNumber`, and `numberOutOfRange` to `NumberOutOfRange`.

## Sign, leading zero and integer transitions

At `start`, EOF gives `unexpectedEnd` without consumption. A minus sign is
consumed and enters `parse_integer(false)`. An initial decimal digit enters
`parse_integer(true)`. Any other initial byte, including plus or whitespace,
gives `invalidNumber` without consumption; this is a numeric-only entry point.
`parse_integer` then uses a consuming read: EOF gives `unexpectedEnd`; a
non-digit is consumed and gives `invalidNumber`. Thus `-x` consumes both bytes,
whereas initial `x` consumes neither numeric nor suffix bytes.

An initial zero consumes that one zero. If the next byte is a digit, fail
`invalidNumber` while leaving that next digit unconsumed. Otherwise proceed
to number finishing. An initial digit 1–9 initializes the u64 significand.
For each subsequent decimal digit `d`, compute the exact candidate `10*s+d`:

- At most u64::MAX: consume this digit and retain the candidate.
- Greater than u64::MAX: do **not** consume the triggering digit or replace `s`;
  set `longInteger`, and enter long-integer scanning with starting exponent zero.

The long-integer scanner consumes every remaining integer digit, including the
triggering digit, incrementing its decimal exponent once per consumed digit.
Those digits never round or alter the retained significand. On a dot it enters
fraction scanning; on `e`/`E` it enters exponent scanning; otherwise it calls
finite admission with the retained significand and omitted-digit exponent.
Even if later syntax is absent, a successful long integer is `finiteOther`.

For a nonoverflowed integer with neither fraction nor exponent, positive syntax
returns `unsigned`. Negative magnitude 1 through 2^63 inclusive returns
`negativeInteger`. Negative zero and magnitude above 2^63 return `finiteOther`,
matching the source wrapping-negation branch; these source floating conversions
are finite for every retained u64 value. In particular, `-0` is never unsigned.
The bare negative-zero flag applies only to that exact successful bare syntax.

## Fraction transitions, including first-digit overflow

The dot is consumed. Each fraction digit is first peeked. If `10*s+d` fits u64,
consume it, update `s`, and decrement the fraction exponent by one. If it would
overflow, leave the digit unconsumed and enter decimal-overflow scanning with
`integerOmittedCount - retainedFractionCount` as its starting exponent.
That scanner consumes all remaining fraction digits, including the triggering
one, without changing the significand or exponent. It then handles an exponent
marker or performs finite admission.

A decimal point requires at least one *actual* following digit, not necessarily
one retained digit. If the very first fraction digit overflows, the overflow
path is valid and consumes it. A rule requiring a positive retained-digit count
would incorrectly reject `18446744073709551615.0`. Without a first digit, EOF
gives `unexpectedEnd`; another byte gives `invalidNumber` without consuming it.
This follows the ordinary decimal loop's distinct EOF/peek errors. All successful
fraction paths produce `finiteOther`, including `0.0` and `1.00`.

## Exponent transitions and signed saturation

Consume `e` or `E`, then an optional plus or minus sign. Read the first exponent
byte with a consuming read. EOF gives `unexpectedEnd`; a non-digit is consumed
and gives `invalidNumber`. Initialize exponent magnitude from that digit.
For each further decimal digit, **consume it before** testing whether `10*e+d`
exceeds 2147483647. Leading zeros in this exponent are allowed.

On the first overflowing exponent digit, do not combine with the starting
exponent. Enter the source overflow branch immediately:

- Nonzero retained significand and positive explicit exponent: return
  `numberOutOfRange` at the cursor immediately after the triggering digit.
  Leave any further exponent digits or suffix bytes unconsumed.
- Zero retained significand, or a negative explicit exponent: consume every
  remaining exponent digit and return `finiteOther` at the next unconsumed byte.
  The starting exponent cannot cancel or reverse this branch.

Without magnitude overflow, combine the starting exponent and explicit signed
magnitude using exact i32 saturating addition/subtraction. The starting exponent
is the omitted integer count minus retained fraction count, both bounded by the
admitted input size. Represent results using imported `JsonDecimalExponent`:
negative magnitude may reach 2147483648; positive magnitude at most 2147483647.
Normalize zero output to `negative=false, magnitude=0`.

The independently controlled scanner helper is
`jsonCombineExponent(starting : JsonDecimalExponent, explicitNegative : Bool,
magnitude : Nat) -> JsonExponentResult`, where the result is
`exponent(JsonDecimalExponent)` or `error(JsonNumberArithmeticError)`.
Validate the starting signed exponent domain first and explicit magnitude
`<=2147483647` second; either violation returns `outsideExponent` before
arithmetic. Negative zero in the admitted starting exponent denotes zero.
Compute exact signed addition/subtraction and clamp to [-2147483648,2147483647].
This helper permits symbolic extreme-boundary controls without allocating
multi-gigabyte input. It does not accept explicit magnitude 2147483648: that
magnitude belongs to the scanner's earlier overflow branch, not saturation.

## Finite admission and totality

All ordinary fraction/exponent and long-integer completions call
`R4.JsonNumberArithmetic.jsonFiniteParts(retainedSignificand, finalExponent)`.
The imported API validates significand/exponent domains and implements exact
rounded-factor finite admission, including the strict overflow midpoint.
`finite` produces `finiteOther`; its `numberOutOfRange` maps to the scanner's
same named error at the current cursor. Any other arithmetic error is retained
as `arithmeticError(error)` and cannot be replaced by a default classification.
Such an error indicates an internal state/admission violation; a lawful scanner
path derives parameters inside the imported domains. The source exponent-
overflow shortcut above is handled before ordinary finite admission.

No host floating-point conversion, serde parser execution, native float oracle,
approximate threshold or arbitrary decimal truncation replaces these algorithms.
The scanner consumes structural suffixes or decreases a fuel bound derived from
admitted input length. If implementation uses fuel, exhaustion with unfinished
work must be a typed internal failure, never a success; the implementation API
must add that explicit constructor before controls are frozen if needed.
Prefer structural suffix recursion so no public fuel policy is required. Exact
Nat operations implement decimal accumulation and signed combination; no linear
recursion on an unbounded exponent value is justified by input recursion.

Formal/load-time arithmetic acceptance does not resolve production lowering:
the approved arithmetic dependency uses big natural intermediates, while the
current production exporter lowers Nat to checked u64. Reviewed multiword/table
lowering or a compatible generic SDK capability remains required for generated
production acceptance. This scanner document claims neither that resolution nor
acceptance of arbitrary JSON values.

## Complete lexeme wrapper and compositional boundary

`jsonCompleteNumber(profile, input)` calls `jsonScanNumber(profile,input,0)`.
On any error it preserves that exact error and cursor. On success it requires
`next == input length`; otherwise it returns `trailingBytes` at `next`.
It accepts no leading/trailing whitespace or JSON delimiters. It returns the
same successful scan record when the entire list is one number.

The prefix scanner does not validate an external JSON delimiter: `1x`, `1]`,
`1  ` and `1\u0000` all scan the numeric prefix at cursor 1, leaving their
suffixes for the surrounding parser. Leading-zero refusal still happens inside
the scanner: `01` fails instead of scanning only `0`. The complete wrapper
rejects every trailing suffix after a successful prefix, but cannot override
a prior numeric syntax/range error. The complete JSON parser must separately
validate containers, punctuation, whitespace, string escapes/UTF-8, depth,
all ignored/overwritten values and object last-wins behavior before extraction.

## Source-derived cursor and classification control plan

The following source-derived examples start at cursor zero. An ASCII `#` marks
an ordinary suffix byte; it is not part of the numeric lexeme. These are literal
expectations to review before implementation, not measurements of a host parser.

| Input bytes shown as text | Prefix result | Cursor |
| --- | --- | ---: |
| empty | `unexpectedEnd` | 0 |
| `-` | `unexpectedEnd` | 1 |
| `+1`, `x` | `invalidNumber` | 0 |
| `-x` | `invalidNumber` | 2 |
| `00`, `01` | `invalidNumber` | 1 |
| `1.` | `unexpectedEnd` | 2 |
| `1.x` | `invalidNumber` | 2 |
| `1e` | `unexpectedEnd` | 2 |
| `1e+` | `unexpectedEnd` | 3 |
| `1eX` | `invalidNumber` | 3 |
| `1e+X` | `invalidNumber` | 4 |
| `-0#` | `finiteOther`, bare-negative-zero true | 2 |
| `0.0#`, `0e0#` | `finiteOther`, bare-negative-zero false | 3 |
| `18446744073709551616#` | `finiteOther`, long-integer true | 20 |
| `18446744073709551615.0#` | `finiteOther`, long-integer false, fraction true | 22 |
| `1e309#` | `numberOutOfRange` | 5 |
| `1e21474836480#` | `numberOutOfRange` | 12 |
| `0e21474836480#` | `finiteOther` | 13 |
| `1e-21474836480#` | `finiteOther` | 14 |

Also freeze nonzero-start versions, with exact raw spans and untouched prefix
bytes; empty input at max capacity zero; invalid profile versus overcapacity;
actual capacity before invalid byte; invalid byte anywhere before invalid cursor;
EOF cursor equality and beyond-end cursors. Exercise every digit boundary,
minus-only input, upper/lower exponent marker, each exponent sign, negative-zero
syntax flags, signed/u64 class boundaries, and whole-wrapper whitespace/suffix
refusal. Preserve the source's lack of exponent cancellation after overflow and
the distinct consumption of first invalid exponent versus fraction bytes.

Helper controls cover both signed exponent extrema, negative zero, exact zero
cancellation, both saturation directions and invalid domains. Scanner controls
must additionally exercise real lexical combination and overflow paths; helper
controls alone do not validate scanning. Finite-boundary controls use the exact
rounded-factor oracle approved for `JsonNumberArithmetic`, including `1e308`,
zero/negative huge exponents, long/fraction truncation, and the source-specific
last finite/first nonfinite boundary. Do not substitute a correctly rounded
full-decimal parser boundary for the selected source algorithm.

Required plants include unsigned classification of float syntax or negative zero,
forgetting the first omitted integer digit, decrementing the exponent for
discarded fraction digits, rounding omitted digits into the significand, canceling a
positive overflowed exponent with the starting exponent, losing exponent-error
cursor precision, accepting midpoint infinity, and applying suffix validation
inside the prefix scanner. Every accepted control statement remains frozen;
formal proof cost must not reduce the supported input semantics.

## Frozen pre-implementation controls

The parent authored 218 literal controls before `R4.JsonNumber` definitions.
Two non-author reviews checked the complete packet against this contract and
the pinned source. No host JSON parser, floating-point conversion or candidate
implementation supplied the expected results. The twenty finite/nonfinite
significand pairs reuse the independently reviewed exact-integer arithmetic
facts, then exercise actual lexical scanner transitions and returned cursors.

| Fixture | Controls | SHA-256 |
| --- | ---: | --- |
| `JsonNumberSyntaxControls` | 76 | `1641eb1060f792031c42ef1fcb60a4bc52a8bf4d0b5c93b2c18c18ce9197491e` |
| `JsonNumberExponentControls` | 82 | `6b597c6638fcad5151ca45aef501dcdf5e21d51b52d9e95d9ca8b48937951e52` |
| `JsonNumberBoundaryControls` | 36 | `2e9999ef4d3495c5be8aeffff376e6f723c699edab6e4026e456d34e2d6d5b8b` |
| `JsonNumberCombineControls` | 24 | `d2f96367e5483f097a5fabb74f047b55390ea523dcd4bf875031ce61c2664960` |

Every fixture is a theorem-only semantic-module/2 document with empty declared
axiom sets. Every expectation uses reflexivity; no custom oracle or acceptance
axiom is embedded. After generating the isolated project's lock, all four
source checks failed with `LLR3005` naming the absent `R4.JsonNumber` import.
The source was still absent when these failures were recorded. Their exact
diagnostics and initial lock are preserved in
`/tmp/lexlean-r4-json-number-scanner-evidence-20261021`. Source implementation,
formal verification, planted defects and integration remain separate stages.
The full language-1.2 production graph migration has
not been performed or inferred from this isolated packet.
