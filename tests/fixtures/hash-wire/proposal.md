# Checked Nat-byte hashing adapter for Object (proposal only)

Status: host-only design; no definitions, control expectations, source dispatch changes, or compiler jobs authorized by this document. Full Object403 and original257 statements remain mandatory. This is a representation experiment, not an accepted Hash replacement or object size restriction.

## Observed failure and proposed boundary

The complete balanced0..64 power experiment preserves the original recursive fallback and passes all1766 primitive/RFC controls, but the unchanged257 digest observation again exhausts the same4GiB compiler container (291.525s, recorded OOM kill). That rejects its sufficiency. It does not establish the cause of memory pressure. Source inspection shows Object currently converts each Nat-core BLAKE output into12 Boolean Byte8 records, then immediately encodes them back to Nat bytes for IDs, parent payloads and comparisons. SHA similarly passes through eight Boolean Word32 records before byte encoding. For257 bytes the chronological construction has13 node hash calls plus a five-block whole SHA computation; those are source call counts, not measured sharing or allocation counts.

The isolated candidate adds R4.HashWire. Existing Hash API/types and all legacy implementations remain unchanged. HashWire calls the same complete HashNatCore functions directly and returns checked byte lists. Only Object's existing ObjHashBytes and ObjNodeHash bodies dispatch to this adapter; their signatures, HashError mapping, and all other Object ASTs remain unchanged. Object manifest IDs, record IDs and payloads remain List Nat with current admission guards. No caller can supply a digest, verified hash, cache entry or proof token.

## Exact public API

All four functions return Result (List Nat) R4.Hash.HashError:

* hashWireSha256(input: List Nat)
* hashWireSha256Bounded(maxBytes: Nat, input: List Nat)
* hashWireBlake2b96SingleBlock(input: List Nat)
* hashWireNodeId(scale: Nat, payload: List Nat)

Success lengths are32 for SHA and12 for BLAKE/node; every output element is0..255. Public inputs may be arbitrary Nat lists. Admission/error order exactly follows the current Hash facade:

1. SHA: actual list length must be below2^61, otherwise messageTooLong(actual). Then actual core validates bytes in input order, returning the first invalidByte(index,value).
2. Bounded SHA: SHA length-domain first, then actual<=maxBytes or budgetExceeded(maxBytes,actual), then byte validation. A malformed byte beyond an exceeded budget must not override the budget error.
3. BLAKE: actual<=128 or blockTooLong(128,actual), then byte validation. This is configured12-byte BLAKE2b output, not truncation of the64-byte parameterization.
4. Node: scale<=37 or invalidScale(scale); exact payload length32 atscale0,36 otherwise, or invalidLength(expected,actual); then BLAKE over scale::payload. Byte error positions include the scale prefix, so payload index0 reports1. Invalidscale takes precedence over payload length/domain.

Use current Words.byteCount and Hash.shaLengthAdmitted for the outer checks, preserving existing count semantics and thresholds. Use HashNatCore.hashNatShaAdmitted and hashNatBlakeAdmitted for actual computation. Match every HashNatCoreError constructor exhaustively into the corresponding HashError; no catch-all, empty output or zero-success fallback. wordFailure is part of the inherited public error type but is not newly fabricated by the Nat path. No extra caller budgets or cutoffs.

## Internal serialization and preservation relation

A globally distinctive helper hashWireShaStateLow32Bytes(state: HashNatCore.HashNatShaState) serializes fields s0..s7 in order; each field contributes its low32 bits in big-endian order. Implement this with existing exact hashNatRawEncodeLE(4,field), reversed per word, then append eight words. The low32 behavior is explicit for arbitrary Nat fields; it is not advertised as validation of an arbitrary Nat state. Actual SHA rounds already mask words to32 bits. BLAKE's admitted core already emits the final twelve Nat bytes.

Required mathematical relation (not claimed proved): for every input in the public Nat-list domain, mapping Hash.encodeDigest256 over the successful legacy SHA result equals hashWireSha256, with identical errors; likewise for bounded SHA, Hash.encodeNodeId over BLAKE and node results. The serializer relation is low32(stateword) = encodeWord32(rawToWord32(stateword)) with matching endianness; Byte8 relation is encodeByte(rawToByte8(n))=n for n<256. Composition uses these bounded core outputs and unchanged checked input logic. No injectivity of hashes is assumed.

Universal source equivalence remains a separate proof obligation if the official semantic proof language cannot express its induction/congruence steps under none axioms. Finite controls establish only their exact statements; do not relabel them a universal proof. Retaining legacy Hash and all original Object statements provides independently auditable regression boundaries but does not itself discharge that universal relation.

## Independent control inventory before definitions

An independent author freezes literal expectations and the statement inventory before candidate code:

* All original30 Hash and403 Object statements remain byte-authorities. Existing1694 Nat/RFC and72 power statements remain unchanged.
*27 direct byte-adapter counterparts of original Hash observations: Cases02..19 and22..30 retain their original byte/error expectations. Case01 tests round mechanics; Cases20/21 test the SHA length predicate and stay in the original suite.
*27 cross-path statements on the same frozen inputs: actual checked adapter result versus actual legacy hash result mapped to encoded bytes, with exact error equality. Both paths compute hashes; no prevalidated caller value or injected result.
*Complete256 Byte8 low-byte roundtrip literals. Complete8x32 one-hot SHA-state field positions plus zero, all-fields MAX32 and a mixed endianness state. Add explicitly frozen above32 values to exercise the documented low32 serializer domain. Exact extra-case count follows independent packet authoring.
*Public error-priority controls combining invalidscale/length/byte, BLAKE128/129 with malformedbytes, SHA budget below/equal/above actual, first/last invalidbyte, and node prefix offset. Existing mathematical SHA length predicate boundary controls retain the unreachable-to-materialize2^61 threshold without constructing enormous lists.
*Checked result shape is observed alongside bytes on positive cases; total output lengths and byte domains are checked for the actual frozen vectors. Full public general shape theorems may be requested separately, with proof capability limitations reported honestly.

Original257digest direct rfl under unchanged limits is the decisive resource experiment; retain its full actual fold, DAG and SHA computation. If it again OOMs, classify resource failure and stop equivalent257 observations. If it passes, run remaining planned257 root/distinct observations and complete403 with actual source/helper closure audits and mutation tests before adoption. No selected96 or257 success accepts the complete Object component.

## Isolation, review, and evidence

Create a new host/container stage from the already pinned Nat-power candidate (HashNat91de8672..., Core177b5763..., facade0e475f41...) and canonical Object1.2 diagnostic snapshot. Freeze every preimage, full closure and original statement multiset. Architecture/spec approval and independent controls precede new definitions; absent-module checks are genuine RED only for new API controls. Source review checks the two Object dispatch diffs and complete checked wrappers. Formal jobs remain scheduled by the parent. Preserve sourcecheck/semantic negative/resource failure separately; keep all receipts and exact manifest inputs in durable hash-object evidence. No accepted worktree source or shared integration files change before full review/acceptance.
