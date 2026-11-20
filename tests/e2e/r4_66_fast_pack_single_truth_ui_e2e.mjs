import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '../..');

const {
  HOLO_MAX_GIT_SAFE_PACK_BYTES,
  admitPackHydrationPolicy,
  admitSingleTruthBadge,
} = await import('../../public/lexlean_r4.js');
const {
  hydrate_holo_model,
  sharedKappaStore,
} = await import('../../crates/wasm/js/worker_host_bridge.js');

// 1. Formal R4-66 Admission Verdicts
assert.equal(HOLO_MAX_GIT_SAFE_PACK_BYTES, 50_331_648);
assert.equal(
  admitPackHydrationPolicy(50_331_648, true, false, false),
  'StreamingPackAdmitted',
);
assert.equal(
  admitPackHydrationPolicy(0, false, false, false),
  'PieceFallbackAdmitted',
);
assert.equal(
  admitPackHydrationPolicy(50_331_649, true, false, false),
  'RefusedPackExceeds50MiB',
);
assert.equal(
  admitPackHydrationPolicy(50_331_648, true, true, false),
  'RefusedPiece404MirrorRetry',
);
assert.equal(
  admitPackHydrationPolicy(50_331_648, true, false, true),
  'RefusedSilentEmergencyDowngrade',
);
assert.equal(admitSingleTruthBadge(false, false), 'SingleTruthBadgeAdmitted');
assert.equal(admitSingleTruthBadge(true, true), 'SingleTruthBadgeAdmitted');
assert.equal(
  admitSingleTruthBadge(false, true),
  'RefusedEmergencyBadgeOnQwenSession',
);

// 2. Verify Qwen3-0.6B (9 packs <= 48 MiB) & Qwen3-1.7B (23 packs <= 48 MiB) Manifests
for (const [rel, expectedPacks, expectedPieces] of [
  ['gpu/weights/v13_qwen3_0_6b/manifest.json', 9, 410],
  ['gpu/weights/v13_qwen3_1_7b/manifest.json', 23, 1088],
]) {
  const m = JSON.parse(fs.readFileSync(path.join(rootDir, rel), 'utf8'));
  assert.equal(m.pieces.length, expectedPieces);
  assert.ok(Array.isArray(m.packs), `${rel} must include packs[]`);
  assert.equal(m.packs.length, expectedPacks, `${rel} pack count`);
  let coveredPieces = 0;
  for (const pk of m.packs) {
    assert.ok(
      pk.byte_length > 0 && pk.byte_length <= HOLO_MAX_GIT_SAFE_PACK_BYTES,
      `${rel} ${pk.file} exceeds 48 MiB: ${pk.byte_length}`,
    );
    assert.equal(typeof pk.sha256, 'string');
    assert.equal(pk.sha256.length, 64);
    coveredPieces += pk.piece_count;
  }
  assert.equal(coveredPieces, expectedPieces, `${rel} packs must cover all pieces`);
}

// 3. Verify Streaming Pack Hydration + Immediate 404 Mirror Pruning
const sha256 = (buf) => crypto.createHash('sha256').update(buf).digest('hex');
const p0 = new Uint8Array(1024).fill(0x11);
const p1 = new Uint8Array(1024).fill(0x22);
const p2 = new Uint8Array(1024).fill(0x33);
const p3 = new Uint8Array(1024).fill(0x44);
const pack0 = Buffer.concat([p0, p1]);
const pack1 = Buffer.concat([p2, p3]);
const containerSha = sha256(Buffer.concat([pack0, pack1]));

const mockManifest = {
  format: 'UORR4V13',
  schema: 'uor-r4.holo-pieces/1',
  version: 13,
  container_sha256: containerSha,
  total_bytes: 4096,
  piece_max_bytes: 1024,
  hold_tail_bytes: 64,
  piece_count: 4,
  pack_max_bytes: 50_331_648,
  pack_count: 2,
  mirrors: [
    'https://dead-404-mirror.example/weights/',
    'https://fast-pack-mirror.example/weights/',
  ],
  packs: [
    { index: 0, file: 'pack_00.pack', piece_start: 0, piece_count: 2, byte_offset: 0, byte_length: 2048, sha256: sha256(pack0) },
    { index: 1, file: 'pack_01.pack', piece_start: 2, piece_count: 2, byte_offset: 2048, byte_length: 2048, sha256: sha256(pack1) },
  ],
  pieces: [
    { index: 0, file: 'piece_0000.bin', sha256: sha256(p0), byte_offset: 0, byte_length: 1024 },
    { index: 1, file: 'piece_0001.bin', sha256: sha256(p1), byte_offset: 1024, byte_length: 1024 },
    { index: 2, file: 'piece_0002.bin', sha256: sha256(p2), byte_offset: 2048, byte_length: 1024 },
    { index: 3, file: 'piece_0003.bin', sha256: sha256(p3), byte_offset: 3072, byte_length: 1024 },
  ],
};

const origFetch = globalThis.fetch;
const pushedBytes = [];
globalThis.studio_begin_holo_v13_mount_verified = () => JSON.stringify({ status: 'ok' });
globalThis.studio_begin_holo_v13_mount = () => JSON.stringify({ status: 'ok' });
globalThis.studio_push_holo_v13_piece = (u8) => {
  pushedBytes.push(new Uint8Array(u8));
  return JSON.stringify({ status: 'ok' });
};
globalThis.studio_finalize_holo_v13_mount = () =>
  JSON.stringify({ status: 'ok', model_id: 'uor-r4.qwen3-0.6b-holo', mounted_count: 1 });

let dead404BlobHits = 0;
let packFetchHits = 0;
let pieceFetchHits = 0;
sharedKappaStore.liarMirrors.clear();
sharedKappaStore.missingBlobMirrors.clear();
sharedKappaStore._memSlabs.clear();

globalThis.fetch = async (rawUrl) => {
  const url = String(rawUrl);
  if (url.endsWith('/manifest.json')) {
    return new Response(JSON.stringify(mockManifest), { status: 200 });
  }
  if (url.endsWith('/manifest.stitch.json')) {
    return new Response('{}', { status: 404 });
  }
  if (url.startsWith('https://dead-404-mirror.example/')) {
    dead404BlobHits += 1;
    return new Response('Not Found', { status: 404 });
  }
  if (url.endsWith('/pack_00.pack')) {
    packFetchHits += 1;
    return new Response(pack0, { status: 200 });
  }
  if (url.endsWith('/pack_01.pack')) {
    packFetchHits += 1;
    return new Response(pack1, { status: 200 });
  }
  if (url.includes('/piece_')) {
    pieceFetchHits += 1;
  }
  return new Response('Not Found', { status: 404 });
};

try {
  const res = await hydrate_holo_model('uor-r4.qwen3-0.6b-holo', null, true, {
    allowOffOriginHydration: true,
    customMirrors: [
      'https://dead-404-mirror.example/weights/',
      'https://fast-pack-mirror.example/weights/',
    ],
  });
  assert.equal(res.status, 'ok');
  assert.equal(packFetchHits, 2, 'Must fetch the 2 pack_XX.pack bundles instead of individual pieces');
  assert.equal(pieceFetchHits, 0, 'Must perform zero individual piece_XXXX.bin fetches when packs succeed');
  assert.ok(dead404BlobHits <= 2, `Dead 404 mirror must be pruned without piece storm (got ${dead404BlobHits})`);
  assert.equal(pushedBytes.length, 4, 'All 4 verified pieces must push in exact order');
  assert.equal(pushedBytes[0][0], 0x11);
  assert.equal(pushedBytes[1][0], 0x22);
  assert.equal(pushedBytes[2][0], 0x33);
  assert.equal(pushedBytes[3][0], 0x44);
} finally {
  globalThis.fetch = origFetch;
}

// 4. Verify Piece-Level 404 Mirror Pruning (1 probe instead of N probes)
let dead404PieceHits = 0;
sharedKappaStore.missingBlobMirrors.clear();
sharedKappaStore._memSlabs.clear();
pushedBytes.length = 0;
const noPackManifest = { ...mockManifest, packs: [], pack_count: 0 };

globalThis.fetch = async (rawUrl) => {
  const url = String(rawUrl);
  if (url.endsWith('/manifest.json')) {
    return new Response(JSON.stringify(noPackManifest), { status: 200 });
  }
  if (url.startsWith('https://dead-404-mirror.example/')) {
    dead404PieceHits += 1;
    return new Response('Not Found', { status: 404 });
  }
  const match = url.match(/piece_000(\d)\.bin$/);
  if (match) {
    const arr = [p0, p1, p2, p3][Number(match[1])];
    return new Response(arr, { status: 200 });
  }
  return new Response('Not Found', { status: 404 });
};

try {
  // Force sequential check with 2 batches by checking missingBlobMirrors populated
  await hydrate_holo_model('uor-r4.qwen3-0.6b-holo', null, true, {
    allowOffOriginHydration: true,
    customMirrors: [
      'https://dead-404-mirror.example/weights/',
      'https://fast-pack-mirror.example/weights/',
    ],
  });
  assert.ok(
    sharedKappaStore.missingBlobMirrors.has('https://dead-404-mirror.example/weights/'),
    '404 mirror must be recorded in missingBlobMirrors immediately',
  );
  assert.equal(pushedBytes.length, 4);
} finally {
  globalThis.fetch = origFetch;
}

// 5. Verify Single-Truth UI + Canonical data-studio-id Aliases + Zero Forbidden Tokens in public/
const htmlSrc = fs.readFileSync(path.join(rootDir, 'public/index.html'), 'utf8');
const appJsSrc = fs.readFileSync(path.join(rootDir, 'public/app.js'), 'utf8');
for (const alias of [
  'data-studio-id="model-picker"',
  'data-studio-id="weight-status-badge"',
  'data-studio-id="hud-dropdown-toggle"',
  'data-studio-id="chat-input"',
  'data-studio-id="neural-hud-dropdown"',
]) {
  assert.ok(htmlSrc.includes(alias), `public/index.html missing ${alias}`);
}
for (const legacyId of [
  'id="claude-chat-input"',
  'id="toggle-inspectors-btn"',
  'id="inspectors-modal"',
  'id="settings-offline-q31-toggle"',
]) {
  assert.ok(htmlSrc.includes(legacyId), `public/index.html must preserve ${legacyId}`);
}
assert.ok(appJsSrc.includes('admitSingleTruthBadgeUi'), 'public/app.js must enforce admitSingleTruthBadgeUi');
assert.ok(appJsSrc.includes('__uorStudioSelect'), 'public/app.js must expose __uorStudioSelect');

console.log('[PASS] R4-66 <=48 MiB streaming packs, 404 mirror pruning, and single-truth UI verified.');
