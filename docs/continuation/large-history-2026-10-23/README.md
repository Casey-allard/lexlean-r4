# Large historical JSON provenance

These seven gzip files preserve all fourteen large JSON locations excluded
from the first bounded text inventory. They retain 99,221,243 unique original
bytes, including any embedded historical source and negative results.
`inventory.json` maps originals and aliases; `manifest.json` binds compressed
and decompressed bytes. Both copying and verification streamed bounded chunks.

Decompress with `gzip -dc files/<sha256>.json.gz` and verify the original hash
before use. This preserves historical data without parsing or endorsing its
schemas, embedded programs or claims. Do not execute historical content.
No compiler binaries, build cache or unrelated plugin changes are included.
