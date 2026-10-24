# BLAKE3 representation experiment disposition

The existing reviewed BLAKE3 design, section 9, supersedes the conditional UInt representation with pure Nat arithmetic. This checkpoint records the independently reviewed decision to archive the rejected UInt/ByteArray experiment. It does not accept its 20 propositions or finish the draft.

The exact original control source and 13 wrapper definitions remain in the existing committed archives named in `decision.json`. Seven exact historical failure/Nat records, the source investigation and independent review are preserved here with their identities. The Nat result concerns different propositions; no native test ran for this checkpoint.

The active draft remains byte-identical because the frozen Read33 preservation map pins it. After the Read33 phases and reviews close, a separate reviewed preservation commit may finish its physical disposition. Any later reuse of Read33 must bind the archived bytes through a reviewed preservation-context successor. Nine drafts remain pending; Object acceptance stays 331/403.

All full BLAKE3 vectors, semantic negatives, error behavior, integration, tokenizer identity and production requirements remain required. HashWire restoration does not discharge BLAKE3 qualification.
