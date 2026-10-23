# Earlier project work: lossless historical archive

This archive preserves 1,821 distinct historical text contents covering 3,729
original/alias locations from the declared project-owned roots. It includes
failed proofs, negative controls, superseded plans, source versions and host
authoring/audit code. It does not enable that code or accept its results.

`inventory.json` records each original path as a root index plus relative path,
its aliases, classification and SHA-256. `manifest.json` maps each original hash
to a gzip-compressed UTF-8 text file, with both compressed and original lengths
and hashes. Decompress a chosen file with `gzip -dc files/<sha256>.txt.gz`, then
verify its original SHA-256 before use. Do not overwrite current source or run
historical scripts without reviewing their original assumptions and paths.

Twenty upstream compiler authority excerpts are historical references only;
they are not installed dependencies or substitutes for the official toolchain.
Failures, rejected candidates, old storage holds and partial results retain
their historical status. The current product plan is ../../CONTINUATION.md.

The inventory is a dated pre-archive comparison to main. All eight large
Engine/Words source occurrences already matched main. Large inventory JSON
coverage is recorded separately; excluded compiler/build trees, binaries and
unrelated plugin data are not claimed as published.
