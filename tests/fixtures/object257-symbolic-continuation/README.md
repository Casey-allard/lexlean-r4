# Original257 symbolic second-atom continuation

Preparation-time status: all new source checks and native proofs are **NOT_RUN**. This is the narrowly reviewed successor to the resource-failed concrete consumer. It preserves that failure and every frozen predecessor.

Only the entire second atom B=[32..63] is abstracted as one fixed List Nat parameter, suffixHeadBytes: two complete occurrences in the original consumer and one in continuation K. All surrounding append nodes and remaining constants are unchanged. The parameter propagates through K, indexed P, lifted theorem and generic actual consumer. The original concrete consumer retains its exact statement and Applies the generic theorem at exact B. Structural substitution back recovers the original statement and K body.

The generic theorem quantifies over arbitrary Nat lists, without a hidden length or byte-validity premise. This is a stronger proposed obligation, not an established result. Resource benefit is uncertain. No Simplify, invented rewrite, axiom, precomputed suffix hash/state, runtime change or limit increase is introduced.

Execute only after independent source review and parent-owned fresh resource/identity/storage admission, using candidate.json mappings and each phase config as lexlean.toml:

1. symbolic-false: require source-valid unrelated-zero base rejection. Stop on timeout/OOM or unrelated failure.
2. symbolic-lift: require actual positive lifted proof and empty-axiom audit; helper only.
3. symbolic-generic: require the generic actual production consumer and empty-axiom audit; still insufficient for original concrete acceptance.
4. symbolic-concrete: require the exact original concrete consumer, artifact/kernel replay and independent empty-axiom review.

The fixture references17 shared files and supplies5 new modules; each phase adds one config, making23 stage inputs before a genuine development lock. Per-child timeout remains300000ms, container4GiB/2CPU and existing storage gates remain unchanged. No runner or live binding is included. Source splitting may change aggregate cost and artifact identities.

manifest.json freezes preparation source/provenance; structural-audit.json records only source comparisons. design/ contains exact investigation/SPEC copies, including historical absolute evidence references, while stage mappings are repository-relative. Future native evidence must be supplemental and must not rewrite preparation-time status. All17 existing draft files remain unchanged. Full original257 digest, complete Object/Hash, production PrismPM integration and Pages remain open. After this bounded proof attempt, completion of the17 draft integration package remains next.
