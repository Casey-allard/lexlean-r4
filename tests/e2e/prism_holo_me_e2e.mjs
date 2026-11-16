import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..', '..');
const PUBLIC_DIR = path.join(ROOT, 'public');

// Reference unkeyed BLAKE3 matching afflom/me/try/vendor/prism-code/src/blake3.mjs
const IV = new Uint32Array([
  0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
]);
const MSG_PERMUTATION = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];
const BLOCK_LEN = 64;
const CHUNK_LEN = 1024;
const CHUNK_START = 1;
const CHUNK_END = 2;
const PARENT = 4;
const ROOT_FLAG = 8;
const rotr = (x, n) => (x >>> n) | (x << (32 - n));
function g(s, a, b, c, d, mx, my) {
  s[a] = (s[a] + s[b] + mx) >>> 0;
  s[d] = rotr(s[d] ^ s[a], 16);
  s[c] = (s[c] + s[d]) >>> 0;
  s[b] = rotr(s[b] ^ s[c], 12);
  s[a] = (s[a] + s[b] + my) >>> 0;
  s[d] = rotr(s[d] ^ s[a], 8);
  s[c] = (s[c] + s[d]) >>> 0;
  s[b] = rotr(s[b] ^ s[c], 7);
}
function round(s, m) {
  g(s, 0, 4, 8, 12, m[0], m[1]);
  g(s, 1, 5, 9, 13, m[2], m[3]);
  g(s, 2, 6, 10, 14, m[4], m[5]);
  g(s, 3, 7, 11, 15, m[6], m[7]);
  g(s, 0, 5, 10, 15, m[8], m[9]);
  g(s, 1, 6, 11, 12, m[10], m[11]);
  g(s, 2, 7, 8, 13, m[12], m[13]);
  g(s, 3, 4, 9, 14, m[14], m[15]);
}
function compress(cv, blockWords, counter, blockLen, flags) {
  const s = new Uint32Array(16);
  s.set(cv.subarray(0, 8), 0);
  s.set(IV.subarray(0, 4), 8);
  s[12] = Number(BigInt.asUintN(32, BigInt(counter)));
  s[13] = Number(BigInt(counter) >> 32n);
  s[14] = blockLen;
  s[15] = flags;
  let m = Uint32Array.from(blockWords);
  for (let i = 0; i < 7; i += 1) {
    round(s, m);
    if (i < 6) {
      const n = new Uint32Array(16);
      for (let j = 0; j < 16; j += 1) n[j] = m[MSG_PERMUTATION[j]];
      m = n;
    }
  }
  for (let i = 0; i < 8; i += 1) {
    s[i] ^= s[i + 8];
    s[i + 8] ^= cv[i];
  }
  return s;
}
function wordsFromBlock(block, len) {
  const words = new Uint32Array(16);
  for (let i = 0; i < 16; i += 1) {
    const o = i * 4;
    words[i] =
      (o < len ? block[o] : 0) |
      ((o + 1 < len ? block[o + 1] : 0) << 8) |
      ((o + 2 < len ? block[o + 2] : 0) << 16) |
      ((o + 3 < len ? block[o + 3] : 0) << 24);
  }
  return words;
}
function blake3Hex(bytes) {
  const stack = [];
  let cv = Uint32Array.from(IV);
  let chunkCounter = 0;
  let block = new Uint8Array(BLOCK_LEN);
  let blockLen = 0;
  let blocksCompressed = 0;

  const pushChunkCv = (cval, totalChunks) => {
    let ncv = cval;
    let n = totalChunks;
    while ((n & 1) === 0) {
      const w = new Uint32Array(16);
      w.set(stack.pop(), 0);
      w.set(ncv, 8);
      ncv = compress(IV, w, 0, BLOCK_LEN, PARENT).slice(0, 8);
      n = Math.floor(n / 2);
    }
    stack.push(ncv);
  };

  let offset = 0;
  while (offset < bytes.length) {
    if (BLOCK_LEN * blocksCompressed + blockLen === CHUNK_LEN) {
      const words = wordsFromBlock(block, blockLen);
      const startFlag = blocksCompressed === 0 ? CHUNK_START : 0;
      const chunkCv = compress(cv, words, chunkCounter, blockLen, startFlag | CHUNK_END).slice(0, 8);
      const totalChunks = chunkCounter + 1;
      pushChunkCv(chunkCv, totalChunks);
      cv = Uint32Array.from(IV);
      chunkCounter = totalChunks;
      block = new Uint8Array(BLOCK_LEN);
      blockLen = 0;
      blocksCompressed = 0;
    }
    if (blockLen === BLOCK_LEN) {
      const words = wordsFromBlock(block, BLOCK_LEN);
      const startFlag = blocksCompressed === 0 ? CHUNK_START : 0;
      cv = compress(cv, words, chunkCounter, BLOCK_LEN, startFlag).slice(0, 8);
      blocksCompressed += 1;
      block.fill(0);
      blockLen = 0;
    }
    const take = Math.min(CHUNK_LEN - (BLOCK_LEN * blocksCompressed + blockLen), BLOCK_LEN - blockLen, bytes.length - offset);
    block.set(bytes.subarray(offset, offset + take), blockLen);
    blockLen += take;
    offset += take;
  }

  let outNode = {
    inputCv: cv,
    words: wordsFromBlock(block, blockLen),
    counter: chunkCounter,
    len: blockLen,
    flags: (blocksCompressed === 0 ? CHUNK_START : 0) | CHUNK_END,
  };
  for (let i = stack.length - 1; i >= 0; i -= 1) {
    const w = new Uint32Array(16);
    w.set(stack[i], 0);
    w.set(compress(outNode.inputCv, outNode.words, outNode.counter, outNode.len, outNode.flags).slice(0, 8), 8);
    outNode = { inputCv: IV, words: w, counter: 0, len: BLOCK_LEN, flags: PARENT };
  }
  const rootWords = compress(outNode.inputCv, outNode.words, 0, outNode.len, outNode.flags | ROOT_FLAG);
  const digest = new Uint8Array(32);
  for (let i = 0; i < 8; i += 1) {
    const w = rootWords[i];
    for (let b = 0; b < 4; b += 1) digest[i * 4 + b] = (w >>> (8 * b)) & 0xff;
  }
  return Array.from(digest, (b) => b.toString(16).padStart(2, '0')).join('');
}

const MANIFEST_SUFFIX_HEX =
  '5200000000000000020000000008000000686f6c6f5f72756e1a000000686f6c6f6772616d3a67756573742f636f72652d7761736d4031030a000000696e6465782e68746d6c08000000706f727461626c6500000000';

// Exact prism-code archive + view + core-wasm parser/replay verification
async function verifyHoloWithPrismCodeRules(bytes, expectedViewFiles) {
  const decoder = new TextDecoder('utf-8', { fatal: true });
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  assert.equal(String.fromCharCode(bytes[0], bytes[1], bytes[2], bytes[3]), 'HOLO');
  assert.equal(dv.getUint16(4, true), 4, 'physical version must be 4');
  assert.equal(dv.getUint16(6, true), 0, 'flags must be 0');
  const count = dv.getUint16(8, true);
  assert.equal(count, 8, 'must have exact 8 sections');

  const bodyEnd = bytes.length - 32;
  let cursor = 10 + count * 24;
  const sections = [];
  const kinds = [];
  for (let i = 0; i < count; i += 1) {
    const at = 10 + i * 24;
    const kind = Number(dv.getBigUint64(at, true));
    const offset = Number(dv.getBigUint64(at + 8, true));
    const length = Number(dv.getBigUint64(at + 16, true));
    assert.equal(offset, cursor, `section ${i} must be contiguous`);
    sections.push({ index: i, kind, body: bytes.subarray(offset, offset + length) });
    kinds.push(kind);
    cursor += length;
  }
  assert.equal(cursor, bodyEnd);
  assert.deepEqual(kinds, [15, 8, 14, 14, 16, 16, 16, 16]);

  const footer = Array.from(bytes.subarray(bodyEnd), (b) => b.toString(16).padStart(2, '0')).join('');
  const fingerprint = blake3Hex(bytes.subarray(0, bodyEnd));
  assert.equal(footer, fingerprint, '32-byte BLAKE3 footer must match body digest');

  // Section 0: 356-byte APP_MANIFEST
  const mf = sections[0].body;
  assert.equal(mf.length, 356);
  const nul = mf.indexOf(0);
  assert.equal(decoder.decode(mf.subarray(0, nul)), 'https://hologram.foundation/realization/app-manifest');
  assert.equal(new DataView(mf.buffer, mf.byteOffset + nul + 1, 4).getUint32(0, true), 3);
  const mfLabels = [0, 1, 2].map((i) => decoder.decode(mf.subarray(57 + i * 71, 57 + (i + 1) * 71)));
  const suffixHex = Array.from(mf.subarray(270), (b) => b.toString(16).padStart(2, '0')).join('');
  assert.equal(suffixHex, MANIFEST_SUFFIX_HEX);

  // Sections 4..7: sorted blake3: blobs
  const blobsByLabel = new Map();
  let prevLbl = null;
  for (const s of sections.slice(4)) {
    const lbl = decoder.decode(s.body.subarray(0, 71));
    const content = s.body.subarray(71);
    assert.equal(lbl, `blake3:${blake3Hex(content)}`, `blob label mismatch at section ${s.index}`);
    if (prevLbl) assert.ok(prevLbl < lbl, 'content blobs must be sorted lexically');
    prevLbl = lbl;
    blobsByLabel.set(lbl, content);
  }

  // Extensions 2 & 3
  const parseExt = (b) => {
    const nlen = b[0] | (b[1] << 8);
    return { name: decoder.decode(b.subarray(2, 2 + nlen)), json: JSON.parse(decoder.decode(b.subarray(2 + nlen))) };
  };
  const dirExt = parseExt(sections[2].body);
  const modExt = parseExt(sections[3].body);
  assert.equal(dirExt.name, 'https://hologram.foundation/extension/application-directory/v1');
  assert.equal(modExt.name, 'https://uor.foundation/extension/prismpm-model/v1');

  const wasmLayer = dirExt.json.layers.find((l) => l.kind === 'wasm');
  const viewLayer = dirExt.json.layers.find((l) => l.kind === 'view');
  assert.equal(wasmLayer.contract, 'hologram:guest/core-wasm@1');
  assert.equal(wasmLayer.entry, 'holo_run');
  assert.equal(viewLayer.entry, 'index.html');
  assert.ok(mfLabels.includes(wasmLayer.content_kappa));
  assert.ok(mfLabels.includes(viewLayer.content_kappa));
  assert.ok(mfLabels.includes(dirExt.json.requires_kappa));

  // Core-Wasm module compilation + 0-import + exact 3 exports + acceptance replay
  const guestBytes = blobsByLabel.get(wasmLayer.content_kappa);
  const mod = await WebAssembly.compile(guestBytes);
  assert.equal(WebAssembly.Module.imports(mod).length, 0, 'Core-Wasm guest must declare 0 imports');
  assert.deepEqual(
    WebAssembly.Module.exports(mod).map((e) => e.name).sort(),
    ['holo_alloc', 'holo_run', 'memory'],
  );

  const modelBytes = blobsByLabel.get(modExt.json.model_content_kappa);
  const modelSha256 = crypto.createHash('sha256').update(modelBytes).digest('hex');
  assert.equal(modExt.json.model_id, modelSha256, 'prismpm-model/v1 model_id must equal SHA-256 of model_bytes');
  const modelDoc = JSON.parse(decoder.decode(modelBytes));
  assert.equal(modelDoc.schema, 'prismpm/model-document/1');
  const vectors = modelDoc.application.acceptance_vectors;
  assert.ok(vectors.length >= 5);
  assert.equal(decoder.decode(Uint8Array.from(vectors[3].request)), '1\tselect\tuor-foundation/foundry');
  assert.equal(decoder.decode(Uint8Array.from(vectors[3].response)), '1\tok\tholo://lexlean-r4\tfoundry/inprism/1');
  for (const vec of vectors) {
    const inst = new WebAssembly.Instance(mod, {});
    const req = Uint8Array.from(vec.request);
    const exp = Uint8Array.from(vec.response);
    const ptr = inst.exports.holo_alloc(req.length) >>> 0;
    new Uint8Array(inst.exports.memory.buffer, ptr, req.length).set(req);
    const packed = BigInt.asUintN(64, inst.exports.holo_run(ptr, req.length));
    const off = Number(packed >> 32n);
    const len = Number(packed & 0xffffffffn);
    const got = new Uint8Array(inst.exports.memory.buffer.slice(off, off + len));
    assert.deepEqual(Array.from(got), Array.from(exp), 'Core-Wasm guest vector mismatch');
  }

  // HOLOVIEW v1 big-endian decoding matching all 6 Studio files byte-for-byte
  const vb = blobsByLabel.get(viewLayer.content_kappa);
  const vdv = new DataView(vb.buffer, vb.byteOffset, vb.byteLength);
  assert.equal(decoder.decode(vb.subarray(0, 8)), 'HOLOVIEW');
  assert.equal(vdv.getUint16(8, false), 1);
  let vat = 10;
  const elen = vdv.getUint32(vat, false);
  vat += 4;
  assert.equal(decoder.decode(vb.subarray(vat, vat + elen)), 'index.html');
  vat += elen;
  const fcount = vdv.getUint32(vat, false);
  vat += 4;
  assert.equal(fcount, expectedViewFiles.size);
  let prevPath = null;
  for (let i = 0; i < fcount; i += 1) {
    const plen = vdv.getUint32(vat, false);
    vat += 4;
    const p = decoder.decode(vb.subarray(vat, vat + plen));
    vat += plen;
    if (prevPath) assert.ok(prevPath < p, `HOLOVIEW path order violation at ${p}`);
    prevPath = p;
    const clen = Number(vdv.getBigUint64(vat, false));
    vat += 8;
    const content = vb.subarray(vat, vat + clen);
    vat += clen;
    assert.deepEqual(
      Array.from(content),
      Array.from(expectedViewFiles.get(p)),
      `HOLOVIEW content mismatch for ${p}`,
    );
  }
  assert.equal(vat, vb.length);

  return { fingerprint, applicationKappa: modExt.json.application_kappa, vectorsCount: vectors.length };
}

async function run() {
  console.log('=== Running R4-54 Stitch PrismId, .holo v4 Compiler & afflom/me E2E Harness ===');

  const glue = await import(path.join(PUBLIC_DIR, 'lexlean_r4.js'));
  const wasmBytes = fs.readFileSync(path.join(PUBLIC_DIR, 'lexlean_r4_bg.wasm'));
  glue.initSync({ module: wasmBytes });

  // 1. Load exact 6-file public/ closure
  const closureNames = ['app.css', 'app.js', 'index.html', 'lexlean_r4.js', 'lexlean_r4_bg.wasm', 'provenance.json'];
  const closureFiles = new Map();
  for (const name of closureNames) {
    closureFiles.set(name, new Uint8Array(fs.readFileSync(path.join(PUBLIC_DIR, name))));
  }

  // 2. Stitch.PrismId creation + WebCrypto non-extractable P-256 + persona binding + role refusals
  const idState = await glue.stitch_prism_identity_ensure();
  assert.equal(idState.ok, true);
  assert.ok(idState.prism_id.startsWith('sha256:'));
  assert.ok(idState.device_id.startsWith('sha256:'));
  assert.equal(idState.role, 'member');

  const personaRes = await glue.stitch_prism_identity_bind_persona('casey-r4', 'uor-foundation.github.io', '');
  assert.equal(personaRes.persona.schema, 'prism/persona/1');
  assert.equal(personaRes.persona.handle, 'casey-r4');
  assert.equal(personaRes.persona.host, 'uor-foundation.github.io');
  assert.equal(personaRes.persona.app_address, 'holo://lexlean-r4');

  // Negative test: float canonical JSON refused
  const floatRefused = JSON.parse(glue.studio_prism_identity_admit_record(JSON.stringify({
    schema: 'prism/action/1',
    prism: idState.prism_id,
    by: idState.device_id,
    action: 'compile-holo',
    bad: 1.5,
  })));
  assert.equal(floatRefused.status, 'error');
  assert.equal(floatRefused.error_code, 'RefusedNonIntegerCanonicalJson');

  // Negative test: bind recovery key and prove recovery cannot sign prism/action/1
  const recDev = {
    schema: 'prism/device/1',
    class: 'browser',
    protection: 'software',
    signing: { kty: 'EC', crv: 'P-256', x: 'rx', y: 'ry' },
    agreement: { kty: 'EC', crv: 'P-256', x: 'rax', y: 'ray' },
  };
  const recId = `sha256:${crypto.createHash('sha256').update(glue.__uorCanonicalJson(recDev)).digest('hex')}`;
  const bindRes = JSON.parse(glue.studio_prism_identity_admit_record(JSON.stringify({
    schema: 'prism/bind/1',
    prism: idState.prism_id,
    by: idState.device_id,
    role: 'recovery',
    device: recDev,
  })));
  assert.equal(bindRes.status, 'ok');
  const recActionRes = JSON.parse(glue.studio_prism_identity_admit_record(JSON.stringify({
    schema: 'prism/action/1',
    prism: idState.prism_id,
    by: recId,
    action: 'compile-holo',
    target_kappa: 'blake3:0',
  })));
  assert.equal(recActionRes.status, 'error');
  assert.equal(recActionRes.error_code, 'RefusedUnauthorizedIdentityRole');

  // 3. Compile + verify .holo v4 archive via Wasm Worker bridge
  const compiled = await glue.compile_and_verify_holo_v4_in_worker(closureFiles, true);
  assert.equal(compiled.ok, true);
  assert.equal(compiled.vectors_passed, 5);
  assert.equal(compiled.vectors_total, 5);
  assert.equal(compiled.wasm_vectors_passed, 5);
  assert.equal(compiled.view_file_count, 6);
  assert.ok(compiled.action_record_kappa && compiled.action_record_kappa.startsWith('sha256:'));

  // 4. Parse compiled bytes with exact prism-code specification & replay Core-Wasm
  const parsedCheck = await verifyHoloWithPrismCodeRules(compiled.bytes, closureFiles);
  assert.equal(parsedCheck.fingerprint, compiled.fingerprint);

  // Verify xtask build-holo output & site/app.json match compiled Worker bytes bit-for-bit
  const xtaskHoloPath = '/tmp/lexlean-r4-app.holo';
  if (fs.existsSync(xtaskHoloPath)) {
    const xtaskBytes = new Uint8Array(fs.readFileSync(xtaskHoloPath));
    const xtaskCheck = await verifyHoloWithPrismCodeRules(xtaskBytes, closureFiles);
    assert.equal(xtaskCheck.fingerprint, compiled.fingerprint, 'xtask .holo fingerprint must match Worker .holo fingerprint');
  }

  const appJson = JSON.parse(fs.readFileSync(path.join(ROOT, 'site', 'app.json'), 'utf8'));
  assert.equal(appJson.address, 'lexlean-r4');
  assert.equal(appJson.holoAddress, 'holo://lexlean-r4');
  assert.equal(appJson.fingerprint, compiled.fingerprint, 'site/app.json fingerprint must match compiled .holo');
  assert.equal(appJson.archiveKappa, compiled.archive_kappa, 'site/app.json archiveKappa must match compiled .holo');
  assert.equal(appJson.embed.queryParam, 'embed=me');
  assert.equal(appJson.embed.foundryQueryParam, 'embed=foundry');
  assert.equal(appJson.foundry.repository, 'https://github.com/UOR-Foundation/foundry');
  assert.equal(appJson.foundry.inprismRequest.mode, 'inprism');

  // 5. Negative refusal: tampered BLAKE3 footer rejected by Wasm verifier
  const tampered = new Uint8Array(compiled.bytes);
  tampered[tampered.length - 1] ^= 0xff;
  await assert.rejects(
    () => glue.verify_holo_v4_archive_in_worker(tampered, false),
    /RefusedHoloBlake3FooterMismatch/,
  );

  // 6. UOR Foundry + afflom/me app descriptor & Native-Q31 inprism turn verification
  const desc = await glue.build_afflom_me_app_descriptor('https://casey-allard.github.io/lexlean-r4/');
  assert.equal(desc.holoAddress, 'holo://lexlean-r4');
  assert.equal(desc.holoUrl, 'https://casey-allard.github.io/lexlean-r4/app.holo');
  assert.equal(desc.archiveKappa, compiled.archive_kappa);
  assert.equal(desc.fingerprint, compiled.fingerprint);
  assert.equal(desc.prismIdentity.prism_id, idState.prism_id);
  assert.equal(desc.foundry.repository, 'https://github.com/UOR-Foundation/foundry');
  assert.equal(desc.foundry.inprismRequest.mode, 'inprism');
  assert.equal(desc.foundry.inprismRequest.outcome, 'answer');

  const q31Turn = JSON.parse(glue.studio_dispatch(`1\tstudio\tchat\t${JSON.stringify({
    prompt: 'Explain UOR-R4 Foundry integration briefly.',
    selected_model: 'uor-r4.native-q31-d85',
    composer_mode: 'chat',
    operating_mode: 'chat',
  })}`));
  assert.equal(q31Turn.status, 'ok');
  assert.equal(q31Turn.selected_model, 'uor-r4.native-q31-d85');
  assert.ok(typeof q31Turn.assistant_reply === 'string' && q31Turn.assistant_reply.trim().length > 0);
  assert.equal(q31Turn.geometric_metrics.float_ops, 0);
  assert.equal(q31Turn.geometric_metrics.dense_matmul_ops, 0);

  console.log(`  ✓ Passed Stitch.PrismId + HOLO v4 + Core-Wasm (5/5) + HOLOVIEW v1 + UOR Foundry (${compiled.bytes.length} bytes, blake3:${compiled.fingerprint.slice(0, 16)}...)`);
}

run().catch((err) => {
  console.error('prism_holo_me_e2e failed:', err);
  process.exit(1);
});
