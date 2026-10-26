# Case19 first64-byte full-state proof pair

SOURCE ONLY. No compiler source admission, intended negative, positive proof,
artifact, kernel replay or empty-axiom audit has run for these new modules.

The positive states actual production blake3FeedLiteralRaw(P,blake3Init(maxu64))
equals Result.ok(S64). P is the exact leftmost packed leaf of original case19.
S64 explicitly supplies all seven B3Hasher fields: original b3CoreIV, zero chunk
counter and compressed blocks, exact buffer0..63, totalBytes64, original maxu64
budget, and empty CV stack. It supplies no computed intermediate digest/CV.

The negative source is authored alongside the positive. Its semantic AST changes
only the expected totalBytes64 to65; module wrapper names differ for isolated
entrypoint selection. It is an intended negative, not an observed failure.
Canonical semantic JSON, exact first-leaf identity and the one-field AST delta
were checked locally without theorem evaluation. Each phase configuration
changes only the default entrypoint from the original case19 configuration.
Runtime, limits, source input tree and expected final digest stay unchanged.

The seven shared inputs (four original gzip runtime modules and three exact
Lake/toolchain metadata files) are pinned by reference in candidate.json. Each
phase adds its selected theorem and config, for nine stage inputs. Do not stage
both proof modules merely to infer an accepted combined closure. Generate a
genuine development lock with the official compiler only after independent
source, invocation and fresh resource reviews. The current case19 failure epoch
has nonzero memory events and supplies no execution authority.

After review, run the intended negative first, classify the actual outcome, then
separately verify the exact positive and audit its actual artifact/axioms/kernel
replays. No automatic retries or raised limits. This pair alone does not prove
the original2048-byte digest, repair OOM, reduce compression cost or increase
historical BLAKE3 counts. Feeding the first64 bytes performs no compression.

Later fixed symbolic continuation, negative Apply control, actual generic feed
consumer and mandatory original-tree consumer are documented in candidate.json
and remain NOT_PREPARED/NOT_RUN. Original validation, length, finalization and
digest encoding obligations remain open. No many-certificate expansion or
replacement hash implementation is included.
