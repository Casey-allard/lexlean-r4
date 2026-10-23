# Tokenizer JSON number admission

The approved [number contract](specs/tokenizer-number-admission.md) separates
exact finite-number arithmetic from the [number scanner API](specs/tokenizer-number-scanner-api.md).
The arithmetic source has passed its 390 frozen formal controls, independent
source reviews and five planted-defect checks followed by exact restoration.
The complete number scanner has also passed all 218 unchanged formal controls,
independent source review and eight planted-defect checks with exact restoration. Neither component claims arbitrary JSON-value
parsing or production export acceptance.

## Exact arithmetic source

`R4.JsonNumberArithmetic` is authored as language 1.2, semantic-module version 2.
Its current source has 18 declarations and SHA-256
`4a51a2fa99452e883d5304f5a20f21ced2005a9258aec2140e2d6541197af96d`.
The module uses exposed generic natural-number multiplication, subtraction,
quotient and remainder. It uses no native UInt or Float primitive, host parser,
callback, floating-point oracle or precomputed result table.

`jsonRoundInteger53` admits naturals strictly below 2^1024. A 1024-step fold
repeatedly divides by two to obtain complete bit length; zero remaining fuel
with nonzero work returns an explicit internal `exhausted` case. The public
rounding function cannot turn that case into a successful truncated length.
Values with at most 53 bits are exact. Larger values use a power-of-two unit,
quotient and remainder; rounding increments on a greater-than-half remainder
or an exact half with odd quotient, then restores the unit. Carry into a new
exponent is retained, including a rounded output of 2^1024. Every division unit
on this public path is positive by construction.

`jsonPowerTen53` checks exponent 0 through 308 before computing the exact power
and rounding it. `jsonProductFinite` compares an exact natural product strictly
against 2^1024 minus 2^970. Equality is overflow. The complete construction
covers every entry in the pinned default-profile serde_json power table; source
literals are independently retained as control facts, not shipped as a runtime
lookup replacement.

`jsonFiniteParts` validates the u64 significand domain before the full signed
exponent domain. Positive magnitude is at most 2147483647; negative magnitude
is at most 2147483648. Negative exponent zero is admitted as zero. Only after
domain admission does zero significand or a negative-tagged exponent admit
finite output without forming a positive-exponent product. Positive-tagged zero
uses the ordinary product with the rounded power of ten equal to one. For a nonzero significand,
positive exponents above 308 return `numberOutOfRange`. Remaining cases multiply
the independently rounded significand and power of ten and apply the exact
midpoint predicate. Internal arithmetic errors propagate explicitly.

Natural arithmetic remains a formal/load-time requirement. The current
production exporter lowers Nat to checked u64, which does not cover these big
intermediates. A reviewed multiword/table lowering or compatible generic SDK
capability remains necessary before generated production acceptance. Formal
verification alone cannot clear that dependency.

## Independently frozen expectations

The parent authored 390 controls, and a non-author reviewer independently
verified their expectations before source definitions. The frozen fixture
`tests/lexlean/JsonNumberArithmeticControls.lex.tex` has SHA-256
`92d6ef7689f4370840aac9ffc38e02bf540782e68067155076482b80a87da6df`.
The supporting exact-integer facts file
`tests/fixtures/tokenizer/number-arithmetic.json` has SHA-256
`71ddc658b5793e10a553bf36263e8f4e07826c5fd34a15bd6f6013389d257d27`.

These retain all 309 rounded powers, 20 exact significand finite/nonfinite
boundaries, ties of both parities, rounding carry, u64 maximum, integer-domain
bounds, the overflow midpoint and signed exponent-domain/error precedence.
The source table is from serde_json 1.0.150 `src/de.rs`, SHA-256
`4998d7e252eb513be85b298e287048b4fb283882e9ba7df905f572f07200f3f3`,
under the approved default feature closure. Expected integer values were
independently derived and reviewed without host floating-point operations or
calls to the candidate model.

The original 390 statements and proofs remain unchanged. A genuine `LLR3005`
missing-module failure was observed in the isolated official language-1.2 SDK
project after its lock existed. A preceding missing-lock setup fault is
preserved separately and is not called RED. The frozen controls, facts, initial
lock and true RED diagnostic are retained under
`/tmp/lexlean-r4-json-number-arithmetic-evidence-20261021`.

The isolated compiler profile keeps the official recursion/heartbeat policy and
300-second child timeout. Whole-fixture proof cost must be measured rather than
assumed from source size. Any partitioning or connected proof decomposition must
preserve every frozen statement and full exact algorithm; no expected-value
substitution, weakened axiom policy or unfinished computation is acceptable.

Independent intrinsic specification review and subsequent code/security review
passed at the source identity above, with no findings. The reviewer inspected
all 18 declarations and checked exact correspondence to the source-authoring
representation. The review establishes the algorithm and contract assessment;
it is separate from compiler verification and production export acceptance.

## Arithmetic verification and restoration

The complete unchanged 390-control module passed in the isolated official SDK,
then passed again after all mutations were removed. Both results bind attestation
`e91c37d25d57029998142c912a835bbc788a9bd376130ca0516cdb138d57dbd0`.
The restored audit checks all 408 declarations against the `none` axiom policy,
all 390 exact theorem names, both kernel replays, successful child processes,
and every non-lexicon manifest input against its recorded byte length and hash.
The restored source remains the reviewed identity above.

Five independently scoped, source-valid defects were rejected by the unchanged
controls with `LLV7002`: reversing tie parity (14 failures), admitting the infinity
midpoint (1), reducing bit-length coverage to 1023 steps (8), rejecting exponent
308 (4), and moving zero-significand acceptance before exponent-domain admission
(2). These are semantic negatives rather than parser or setup failures. An
initial `LLV7001` tool-identity setup fault caused by first-time Lake manifest
output is preserved separately; a fresh lock retry produced the full positive
result without changing source, controls, axiom policy or compiler limits.

The local evidence directory above retains each source-checked mutant, diagnostic,
restored receipt, build manifest and `audit-summary.json`. Parent-owned aggregate
Rust acceptance and production integration remain separate gates.

## Complete number scanner source

`R4.JsonNumber` has 36 declarations and source SHA-256
`0f99c00b07c7d4e69cb6540d98b0d119394ce11dd3128029348fbfece53398c5`.
It imports the actual arithmetic implementation and implements the approved
scanner contract using structural suffix-list recursion. Public admission checks
capacity domain, whole-input length, all byte domains, and cursor range in that
order. Successful prefix scans retain the first unconsumed cursor; the complete
wrapper rejects trailing bytes without silently extending the prefix grammar.

The scanner preserves unsigned and negative-integer classification, finite
floating-representation classification, bare negative zero, long-integer and fraction/exponent
flags. It retains the source distinction between consuming integer overflow
digits and discarding overflowing fraction digits, and between immediate
positive-exponent overflow errors and zero/negative-exponent overflow tails.
Signed exponent combination validates both domains before saturating addition.
Every finite classification that requires arithmetic calls `jsonFiniteParts`;
no host parser, floating-point oracle or precomputed classification substitutes
for that path.

All 218 independently authored expectations were frozen before definitions, in
the four fixtures listed in the approved scanner API. Each observed `LLR3005`
with the genuine initial lock present and `R4.JsonNumber` absent. The four RED
diagnostics, initial lock and original fixture bytes are retained under
`/tmp/lexlean-r4-json-number-scanner-evidence-20261021`. Initial source checking
rejected a Nat-only equality applied to Boolean operands (`LLT4001`); replacing
it with the equivalent Boolean conditional fixed the type error without changing
any expectation. The repaired complete source now passes static checking.
Independent scanner specification review and subsequent code/security review
passed at the source identity above. The reviewer checked all 36 declarations
and byte correspondence with the authoring representation. The four complete scanner control modules passed unchanged and passed again
after all planted defects were removed. Parent integration is recorded below.

The scanner fixtures retain these exact identities:

| Fixture | Controls | SHA-256 |
| --- | ---: | --- |
| `JsonNumberSyntaxControls.lex.tex` | 76 | `1641eb1060f792031c42ef1fcb60a4bc52a8bf4d0b5c93b2c18c18ce9197491e` |
| `JsonNumberExponentControls.lex.tex` | 82 | `6b597c6638fcad5151ca45aef501dcdf5e21d51b52d9e95d9ca8b48937951e52` |
| `JsonNumberBoundaryControls.lex.tex` | 36 | `2e9999ef4d3495c5be8aeffff376e6f723c699edab6e4026e456d34e2d6d5b8b` |
| `JsonNumberCombineControls.lex.tex` | 24 | `d2f96367e5483f097a5fabb74f047b55390ea523dcd4bf875031ce61c2664960` |

## Scanner verification and restoration

The restored source and all 218 original statements are byte-identical to their
reviewed pins. The audit checks the exact theorem set in each module, every
declaration against the `none` axiom policy, three successful kernel replays per
root, successful child processes, and every non-lexicon manifest input byte
length and SHA-256. Original and restored runs bind the same attestations:

| Fixture | Audited declarations | Attestation |
| --- | ---: | --- |
| Syntax | 130 | `e677d71f898f5f07a5de89326292647f6c4200b8bfbf9581c88efa951b793213` |
| Exponent | 136 | `0353e1b86cddeea91db7913ed411a2b46aa2053bf4167deac1ae584101d205e2` |
| Boundary | 90 | `386897c628722056df70fde19f73fef59b0064b8510a7049de349adbb1d78d23` |
| Combine | 78 | `278bd174f4e58927d9416bcbd91b7fc5cef59710905a5f082c6c41eee4c8faba` |

Eight source-valid mutations each changed one definition and failed unchanged
formal controls with `LLV7002`: classifying negative zero as unsigned (3 failures),
accepting a second digit after leading zero (20), double-counting the omitted
integer trigger (1), decrementing exponent for a discarded fraction digit (2),
leaving the overflowing exponent trigger unconsumed (4), rejecting the zero
significand overflow shortcut (4), shifting negative saturation by one (3), and
accepting trailing bytes in the complete wrapper (6). All four roots were
reverified after restoration, not only the module targeted by the last mutant.

The evidence directory retains each mutant and source check, exact failing
diagnostics, restored attestations/manifests and `audit-summary.json`. The
scanner project encountered the same initial Lake-manifest identity-probe setup
fault as the arithmetic project; that `LLV7001` diagnostic is retained separately
from semantic negatives. A fresh lock retry required no source, expected-value,
axiom-policy or compiler-limit change. The formal result establishes the bounded
numeric observation kernel; raw JSON parsing, full tokenizer composition and
production export remain separate obligations.

## Integrated development checkpoint

The language-1.2 migration integrates both exact number modules and all five
frozen control roots into the default graph: 16 components, 84 modules, 60 roots
and 7,871 controls. The complete formal replay passed under attestation
`5124576420efeda3241a8865501bf1eed32788f633d9a0868b84716eb5b9bdba`.
The independent audit checked 9,910 none-axiom declaration rows, 84 successful
kernel replays and all 86 current non-lexicon inputs.

The actual Number Rust gate passed all 608 controls and the receipt-language
binding test. Its five receipts were independently audited against 842
declaration rows, 14 kernel replays and their current inputs. The subsequent
complete workspace suite passed all 54 Rust tests, all 16 component gates and
two nested bootstrap checks, without ignored or filtered tests. Formatting,
workspace Clippy and all-feature/all-target checks also passed.

The unmodified `just vv` stopped at `template-check` because `prismpm.lock` is
absent. The model register, failure, deferral and ownership checks passed before
the same missing binding stopped `just model`. Production acceptance, supported
big-integer lowering, browser export and Pages deployment remain required.
See [the migration record](LANGUAGE-MIGRATION.md) for compiler identity and the
retained source, receipt and execution evidence.
