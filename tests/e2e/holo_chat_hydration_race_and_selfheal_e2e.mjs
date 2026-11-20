import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '../..');

function sha256Hex(buf) {
  return crypto.createHash('sha256').update(buf).digest('hex');
}

function buildSyntheticHoloV13Container() {
  const vocab = 256;
  const dm = 128;
  const nl = 1;
  const nh = 2;
  const nkv = 1;
  const hd = 128;
  const ffn = 192;
  const maxSeq = 256;
  const chunks = [];
  const pushU32 = (v) => {
    const b = Buffer.alloc(4);
    b.writeUInt32LE(v >>> 0, 0);
    chunks.push(b);
  };
  const pushI32 = (v) => {
    const b = Buffer.alloc(4);
    b.writeInt32LE(v | 0, 0);
    chunks.push(b);
  };
  const pushI64 = (v) => {
    const b = Buffer.alloc(8);
    b.writeBigInt64LE(BigInt(v), 0);
    chunks.push(b);
  };
  const pushU64 = (v) => {
    const b = Buffer.alloc(8);
    b.writeBigUInt64LE(BigInt(v), 0);
    chunks.push(b);
  };

  // 96-byte UORR4V13 header
  chunks.push(Buffer.from('UORR4V13', 'ascii'));
  pushU32(13);
  pushU32(1);
  pushU32(vocab);
  pushU32(dm);
  pushU32(nl);
  pushU32(nh);
  pushU32(nkv);
  pushU32(hd);
  pushU32(ffn);
  pushU32(maxSeq);
  pushU32(0); // bos
  pushU32(1); // eos
  pushU32(0); // eot
  pushU32(2); // think_start
  pushU32(3); // think_end
  pushI64(281_474_977n);
  pushI64(94_906_266n);
  pushU64(1_000_000n);
  pushU32(0b01111);

  // RoPE + exp + silu LUTs
  const half = hd / 2;
  const ropeCos = Buffer.alloc(maxSeq * half * 2);
  for (let i = 0; i < maxSeq * half; i++) ropeCos.writeInt16LE(16_384, i * 2);
  chunks.push(ropeCos);
  chunks.push(Buffer.alloc(maxSeq * half * 2, 0));

  const expLen = (32 << 8) + 1;
  const expBuf = Buffer.alloc(expLen * 4);
  for (let i = 0; i < expLen; i++) expBuf.writeUInt32LE(1 << 30, i * 4);
  chunks.push(expBuf);

  const siluLen = (1 << (4 - (-8) + 1)) + 1;
  chunks.push(Buffer.alloc(siluLen * 4, 0));

  // UORTK13\0 tokenizer
  const tokParts = [Buffer.from('UORTK13\0', 'binary')];
  const tokHdr = Buffer.alloc(12);
  tokHdr.writeUInt32LE(vocab, 0);
  tokHdr.writeUInt32LE(0, 4);
  tokHdr.writeUInt32LE(0, 8);
  tokParts.push(tokHdr);
  for (let b = 0; b < vocab; b++) {
    const e = Buffer.alloc(3);
    e.writeUInt16LE(1, 0);
    e[2] = b;
    tokParts.push(e);
  }
  for (let b = 0; b < vocab; b++) {
    const e = Buffer.alloc(3);
    e.writeUInt16LE(1, 0);
    e[2] = b >= 0x21 && b <= 0x7e ? b : 0x78;
    tokParts.push(e);
  }
  const tokBuf = Buffer.concat(tokParts);
  pushU32(tokBuf.length);
  chunks.push(tokBuf);

  // 512-bit vocab RHT signatures
  const sigBuf = Buffer.alloc(vocab * 8 * 8);
  let sp = 0;
  for (let v = 0; v < vocab; v++) {
    for (let w = 0; w < 8; w++) {
      const pat = (0x5555_5555_5555_5555n ^ (((BigInt(v) * 0x9e37_79b9n) & 0xffff_ffff_ffff_ffffn) << BigInt(w * 3))) & 0xffff_ffff_ffff_ffffn;
      sigBuf.writeBigUInt64LE(pat, sp);
      sp += 8;
    }
  }
  chunks.push(sigBuf);

  // Tied embedding matrix [vocab, dm]
  pushU32(vocab);
  pushU32(dm);
  pushU64(1n);
  chunks.push(Buffer.alloc(vocab, (-20) & 0xff));
  const embGm = Buffer.alloc(vocab * (dm >> 5) * 2);
  for (let i = 0; i < vocab * (dm >> 5); i++) embGm.writeUInt16LE(3, i * 2);
  chunks.push(embGm);
  const embW = Buffer.alloc(vocab * dm);
  let wp = 0;
  for (let r = 0; r < vocab; r++) {
    for (let c = 0; c < dm; c++) {
      embW[wp++] = (((r * 7 + c * 13) % 41) - 20) & 0xff;
    }
  }
  chunks.push(embW);

  // Final RMSNorm + Layer 0 norms
  for (let i = 0; i < dm; i++) pushI32(1 << 24);
  for (let i = 0; i < dm * 2 + hd * 2; i++) pushI32(1 << 24);

  // Layer 0 Int4 projections
  const pushI4 = (rows, cols, seed) => {
    pushU32(rows);
    pushU32(cols);
    pushU64(BigInt(seed));
    chunks.push(Buffer.alloc(rows, (-18) & 0xff));
    const gm = Buffer.alloc(rows * (cols >> 5) * 2);
    for (let i = 0; i < rows * (cols >> 5); i++) gm.writeUInt16LE((i % 5) + 1, i * 2);
    chunks.push(gm);
    const nib = Buffer.alloc(rows * (cols >> 1));
    for (let i = 0; i < nib.length; i++) {
      nib[i] = (((i * 17 + 3) % 15) & 0x0f) | (((((i * 23 + 5) % 15) & 0x0f)) << 4);
    }
    chunks.push(nib);
  };
  const qDim = nh * hd;
  const kvDim = nkv * hd;
  pushI4(qDim, dm, 11);
  pushI4(kvDim, dm, 12);
  pushI4(kvDim, dm, 13);
  pushI4(dm, qDim, 14);
  pushI4(ffn, dm, 15);
  pushI4(ffn, dm, 16);
  pushI4(dm, ffn, 17);

  return new Uint8Array(Buffer.concat(chunks));
}

function createMockOpfsRoot() {
  function makeDir() {
    const dirs = new Map();
    const files = new Map();
    return {
      _dirs: dirs,
      _files: files,
      async getDirectoryHandle(name, opts = {}) {
        if (!dirs.has(name)) {
          if (!opts.create) throw new Error(`Missing OPFS dir: ${name}`);
          dirs.set(name, makeDir());
        }
        return dirs.get(name);
      },
      async getFileHandle(name, opts = {}) {
        if (!files.has(name)) {
          if (!opts.create) throw new Error(`Missing OPFS file: ${name}`);
          files.set(name, { bytes: new Uint8Array(0), locked: false });
        }
        const entry = files.get(name);
        return {
          async getFile() {
            return new Blob([entry.bytes]);
          },
          async createSyncAccessHandle() {
            if (entry.locked) {
              const err = new Error(`OPFS SyncAccessHandle already locked: ${name}`);
              err.name = 'NoModificationAllowedError';
              throw err;
            }
            entry.locked = true;
            return {
              getSize() {
                return entry.bytes.byteLength;
              },
              truncate(sz) {
                const next = new Uint8Array(sz);
                next.set(entry.bytes.subarray(0, Math.min(sz, entry.bytes.byteLength)), 0);
                entry.bytes = next;
              },
              read(target, { at = 0 } = {}) {
                const avail = Math.max(0, Math.min(target.byteLength, entry.bytes.byteLength - at));
                if (avail > 0) target.set(entry.bytes.subarray(at, at + avail), 0);
                return avail;
              },
              write(src, { at = 0 } = {}) {
                const u8 = src instanceof Uint8Array ? src : new Uint8Array(src);
                if (at + u8.byteLength > entry.bytes.byteLength) {
                  const grown = new Uint8Array(at + u8.byteLength);
                  grown.set(entry.bytes, 0);
                  entry.bytes = grown;
                }
                entry.bytes.set(u8, at);
                return u8.byteLength;
              },
              flush() {},
              close() {
                entry.locked = false;
              },
            };
          },
          async move(targetDir, newName) {
            if (entry.locked) throw new Error('Cannot move locked OPFS file');
            targetDir._files.set(newName, { bytes: entry.bytes.slice(), locked: false });
            files.delete(name);
          },
        };
      },
      async removeEntry(name) {
        files.delete(name);
        dirs.delete(name);
      },
    };
  }
  return makeDir();
}

const wasmMod = await import('../../public/lexlean_r4.js');
const wasmBytes = fs.readFileSync(path.join(rootDir, 'public/lexlean_r4_bg.wasm'));
await wasmMod.default({ module_or_path: wasmBytes });

const {
  hydrate_holo_model,
  execute_inprism_foundry_turn,
  sharedKappaStore,
  studio_begin_holo_v13_mount_verified,
  studio_push_holo_v13_piece,
} = wasmMod;

const fullContainer = buildSyntheticHoloV13Container();
const containerSha = sha256Hex(fullContainer);
const quarter = Math.floor(fullContainer.byteLength / 4);
const piecesBytes = [
  fullContainer.subarray(0, quarter),
  fullContainer.subarray(quarter, quarter * 2),
  fullContainer.subarray(quarter * 2, quarter * 3),
  fullContainer.subarray(quarter * 3),
];
const pack0 = Buffer.concat([Buffer.from(piecesBytes[0]), Buffer.from(piecesBytes[1])]);
const pack1 = Buffer.concat([Buffer.from(piecesBytes[2]), Buffer.from(piecesBytes[3])]);

let runningOff = 0;
const piecesMeta = piecesBytes.map((b, index) => {
  const rec = {
    index,
    file: `piece_000${index}.bin`,
    sha256: sha256Hex(b),
    byte_offset: runningOff,
    byte_length: b.byteLength,
  };
  runningOff += b.byteLength;
  return rec;
});

const manifest = {
  format: 'UORR4V13',
  schema: 'uor-r4.holo-pieces/1',
  version: 13,
  container_sha256: containerSha,
  total_bytes: fullContainer.byteLength,
  piece_max_bytes: 1_048_576,
  hold_tail_bytes: 65_536,
  piece_count: 4,
  pack_max_bytes: 50_331_648,
  pack_count: 2,
  mirrors: ['https://raw.githubusercontent.com/Casey-allard/lexlean-r4/weights-v13-qwen3-0.6b/'],
  packs: [
    {
      index: 0,
      file: 'pack_00.pack',
      piece_start: 0,
      piece_count: 2,
      byte_offset: 0,
      byte_length: pack0.byteLength,
      sha256: sha256Hex(pack0),
    },
    {
      index: 1,
      file: 'pack_01.pack',
      piece_start: 2,
      piece_count: 2,
      byte_offset: pack0.byteLength,
      byte_length: pack1.byteLength,
      sha256: sha256Hex(pack1),
    },
  ],
  pieces: piecesMeta,
};

const origFetch = globalThis.fetch;
const origNav = Object.getOwnPropertyDescriptor(globalThis, 'navigator');

let mockRoot = createMockOpfsRoot();
Object.defineProperty(globalThis, 'navigator', {
  configurable: true,
  value: {
    hardwareConcurrency: 8,
    deviceMemory: 8,
    storage: {
      async estimate() {
        return { quota: 4_000_000_000, usage: 0 };
      },
      async getDirectory() {
        return mockRoot;
      },
    },
  },
});

let packFetches = 0;
let networkOnline = true;
globalThis.fetch = async (rawUrl) => {
  if (!networkOnline) {
    return new Response('Offline', { status: 503 });
  }
  const url = String(rawUrl);
  await new Promise((r) => setTimeout(r, 15));
  if (url.endsWith('/manifest.json')) {
    return new Response(JSON.stringify(manifest), { status: 200 });
  }
  if (url.endsWith('/manifest.stitch.json')) {
    return new Response('{}', { status: 404 });
  }
  if (url.endsWith('/pack_00.pack')) {
    packFetches += 1;
    await new Promise((r) => setTimeout(r, 25));
    return new Response(pack0, { status: 200 });
  }
  if (url.endsWith('/pack_01.pack')) {
    packFetches += 1;
    await new Promise((r) => setTimeout(r, 10));
    return new Response(pack1, { status: 200 });
  }
  return new Response('Not Found', { status: 404 });
};

try {
  // 1. Accurate UORR4V13 magic diagnostic (never misreports UORR4V12)
  console.log('1. Verifying accurate UORR4V13 magic diagnostic on corrupt header...');
  studio_begin_holo_v13_mount_verified('uor-r4.qwen3-0.6b-holo', 1, '0'.repeat(64), true);
  const badPush = JSON.parse(studio_push_holo_v13_piece(new Uint8Array(128)));
  assert.equal(badPush.status, 'error');
  assert.ok(
    String(badPush.message).includes('UORR4V13') && !String(badPush.message).includes('UORR4V12'),
    `Expected UORR4V13 magic error message, got: ${badPush.message}`,
  );

  // 2. Concurrent boot(:auto) + chat(:ext) race must deduplicate & succeed without corrupting mount or OPFS
  console.log('2. Verifying concurrent boot(:auto) + chat(:ext) single-flight hydration & live turn...');
  sharedKappaStore.liarMirrors.clear();
  sharedKappaStore.missingBlobMirrors.clear();
  sharedKappaStore._memSlabs.clear();
  packFetches = 0;

  const bootProg = [];
  const chatProg = [];
  const pBoot = hydrate_holo_model('uor-r4.qwen3-0.6b-holo', (e) => bootProg.push(e), false, {});
  await new Promise((r) => setTimeout(r, 8));
  const pChat = hydrate_holo_model(
    'uor-r4.qwen3-0.6b-holo',
    (e) => chatProg.push(e),
    false,
    { allowOffOriginHydration: true },
  );
  const [bootOut, chatOut] = await Promise.all([pBoot, pChat]);
  assert.equal(bootOut.status, 'ok');
  assert.equal(chatOut.status, 'ok');
  assert.equal(packFetches, 2, `Expected single-flight 2 pack fetches, got ${packFetches}`);
  assert.ok(bootProg.length > 0, 'Boot hydration listener must receive progress');
  assert.ok(chatProg.length > 0, 'Joined chat hydration listener must receive progress');

  const turn1 = await execute_inprism_foundry_turn({
    id: 101,
    payload: {
      prompt: 'Hello Studio!',
      selected_model: 'uor-r4.qwen3-0.6b-holo',
      allowOffOriginHydration: true,
      max_new_tokens: 6,
    },
  });
  assert.equal(turn1.ok, true, `Expected turn1 ok, got error: ${turn1.error}`);
  assert.ok((turn1.result?.tokens_emitted || 0) >= 1);

  // 3. Partial piece cache hit (pack_00 in getVerifiedPiece + pack_01 over HTTP) must NOT write zero-hole model.holoslab
  console.log('3. Verifying mixed OPFS piece-cache + HTTP pack hydration seals complete UORR4V13 slab...');
  mockRoot = createMockOpfsRoot();
  sharedKappaStore._memSlabs.clear();
  await sharedKappaStore.putPiece(piecesMeta[0].sha256, piecesBytes[0]);
  await sharedKappaStore.putPiece(piecesMeta[1].sha256, piecesBytes[1]);
  packFetches = 0;

  const mixedOut = await hydrate_holo_model('uor-r4.qwen3-0.6b-holo', null, false, {
    allowOffOriginHydration: true,
  });
  assert.equal(mixedOut.status, 'ok');
  assert.equal(packFetches, 1, 'pack_00 came from OPFS piece cache; only pack_01 should fetch');

  const modelsDir = await mockRoot.getDirectoryHandle('models', { create: false });
  const slugDir = await modelsDir.getDirectoryHandle('uor-r4.qwen3-0.6b-holo', { create: false });
  const sealedSlabBytes = slugDir._files.get('model.holoslab')?.bytes;
  assert.ok(sealedSlabBytes, 'model.holoslab must exist after mixed hydration');
  assert.equal(
    Buffer.from(sealedSlabBytes.subarray(0, 8)).toString('ascii'),
    'UORR4V13',
    'model.holoslab must begin with UORR4V13 (never sparse zeros when pack_00 was cached)',
  );

  // Warm mount with network completely offline must succeed from sealed model.holoslab
  networkOnline = false;
  const warmOfflineOut = await hydrate_holo_model('uor-r4.qwen3-0.6b-holo', null, false, {
    allowOffOriginHydration: true,
  });
  assert.equal(warmOfflineOut.status, 'ok');

  // 4. Self-healing poisoned zero-header model.holoslab persisted in OPFS when user submits chat message
  console.log('4. Verifying automatic self-healing when OPFS model.holoslab has zeroed/corrupt header...');
  networkOnline = true;
  packFetches = 0;
  // Poison first 96 bytes of model.holoslab with zeros while preserving exact file size & manifest.verified.json
  sealedSlabBytes.fill(0, 0, 96);

  // Force re-mount into a fresh model slot or re-hydration
  const healedOut = await hydrate_holo_model('uor-r4.qwen3-0.6b-holo', null, false, {
    allowOffOriginHydration: true,
  });
  assert.equal(healedOut.status, 'ok', 'Poisoned warm model.holoslab must self-heal via pack re-hydration');
  assert.ok(packFetches >= 1 || slugDir._files.get('model.holoslab'), 'Corrupt slab must be replaced');
  const healedSlabBytes = slugDir._files.get('model.holoslab')?.bytes;
  assert.ok(healedSlabBytes, 'Self-healed model.holoslab must be re-sealed');
  assert.equal(
    Buffer.from(healedSlabBytes.subarray(0, 8)).toString('ascii'),
    'UORR4V13',
    'Self-healed model.holoslab must start with UORR4V13',
  );

  const turnAfterHeal = await execute_inprism_foundry_turn({
    id: 102,
    payload: {
      prompt: 'Test reply after self-heal',
      selected_model: 'uor-r4.qwen3-0.6b-holo',
      allowOffOriginHydration: true,
      max_new_tokens: 6,
    },
  });
  assert.equal(turnAfterHeal.ok, true, `Expected turnAfterHeal ok, got: ${turnAfterHeal.error}`);
  assert.ok((turnAfterHeal.result?.tokens_emitted || 0) >= 1);

  console.log('[PASS] Holo chat hydration race, mixed cache slab sealing, and poisoned OPFS self-healing verified.');
} finally {
  globalThis.fetch = origFetch;
  if (origNav === undefined) {
    delete globalThis.navigator;
  } else {
    Object.defineProperty(globalThis, 'navigator', origNav);
  }
}
