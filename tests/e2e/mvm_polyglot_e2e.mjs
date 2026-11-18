import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import zlib from 'node:zlib';

import {
  SpiralCoreHostBridge,
  PYODIDE_LOCK,
} from '../../crates/wasm/js/worker_host_bridge.js';

function b64Encode(bufOrStr) {
  const buf = typeof bufOrStr === 'string' ? Buffer.from(bufOrStr, 'utf8') : Buffer.from(bufOrStr);
  return buf.toString('base64');
}

function b64Decode(b64) {
  return Buffer.from(String(b64 || ''), 'base64');
}

function buildUstarTar(entries) {
  const blocks = [];
  for (const ent of entries) {
    const hdr = Buffer.alloc(512, 0);
    const name = String(ent.name || '');
    const data = ent.data ? Buffer.from(ent.data) : Buffer.alloc(0);
    const typeflag = ent.typeflag || '0';
    const mode = ent.mode !== undefined ? ent.mode : (typeflag === '2' ? 0o777 : 0o644);
    const size = typeflag === '2' || typeflag === '5' ? 0 : data.length;

    hdr.write(name.slice(0, 100), 0, 'utf8');
    hdr.write(mode.toString(8).padStart(7, '0') + '\0', 100, 'ascii');
    hdr.write('0000000\0', 108, 'ascii');
    hdr.write('0000000\0', 116, 'ascii');
    hdr.write(size.toString(8).padStart(11, '0') + '\0', 124, 'ascii');
    hdr.write('00000000000\0', 136, 'ascii');
    hdr.fill(0x20, 148, 156);
    hdr.write(typeflag, 156, 'ascii');
    if (ent.linkname) {
      hdr.write(String(ent.linkname).slice(0, 100), 157, 'utf8');
    }
    hdr.write('ustar\0', 257, 'ascii');
    hdr.write('00', 263, 'ascii');

    let sum = 0;
    for (let i = 0; i < 512; i += 1) sum += hdr[i];
    hdr.write(sum.toString(8).padStart(6, '0') + '\0 ', 148, 'ascii');
    blocks.push(hdr);

    if (size > 0) {
      blocks.push(data);
      const rem = size % 512;
      if (rem !== 0) blocks.push(Buffer.alloc(512 - rem, 0));
    }
  }
  blocks.push(Buffer.alloc(1024, 0));
  return Buffer.concat(blocks);
}

function encodeLebU32(n) {
  const out = [];
  let v = n >>> 0;
  do {
    let b = v & 0x7f;
    v >>>= 7;
    if (v !== 0) b |= 0x80;
    out.push(b);
  } while (v !== 0);
  return out;
}

function wasmSection(id, payloadBytes) {
  return [id, ...encodeLebU32(payloadBytes.length), ...payloadBytes];
}

function wasmStr(s) {
  const b = Buffer.from(s, 'utf8');
  return [...encodeLebU32(b.length), ...b];
}

// Hand-assembled valid WASI Preview 1 module exporting memory + _start:
// Writes "hello wasi\n" (at offset 64, len 11) via wasi_snapshot_preview1.fd_write(1, 0, 1, 16)
function buildMinimalWasiStdoutModule() {
  const typeSection = wasmSection(1, [
    2,
    // type 0: fd_write(i32, i32, i32, i32) -> i32
    0x60, 4, 0x7f, 0x7f, 0x7f, 0x7f, 1, 0x7f,
    // type 1: _start() -> ()
    0x60, 0, 0,
  ]);
  const importSection = wasmSection(2, [
    1,
    ...wasmStr('wasi_snapshot_preview1'),
    ...wasmStr('fd_write'),
    0x00, 0,
  ]);
  const funcSection = wasmSection(3, [1, 1]);
  const memSection = wasmSection(5, [1, 0x00, 1]);
  const exportSection = wasmSection(7, [
    2,
    ...wasmStr('memory'), 0x02, 0,
    ...wasmStr('_start'), 0x00, 1,
  ]);
  const body = [
    0, // 0 locals
    // iov[0].buf = 64 (signed LEB128: 0xc0, 0x00) at addr 0
    0x41, 0x00, // i32.const 0
    0x41, 0xc0, 0x00, // i32.const 64
    0x36, 0x02, 0x00, // i32.store
    // iov[0].len = 11 at addr 4
    0x41, 0x04, // i32.const 4
    0x41, 0x0b, // i32.const 11
    0x36, 0x02, 0x00, // i32.store
    // call fd_write(1, 0, 1, 16)
    0x41, 0x01, // fd = 1 (stdout)
    0x41, 0x00, // iovs = 0
    0x41, 0x01, // iovs_len = 1
    0x41, 0x10, // nwritten = 16
    0x10, 0x00, // call 0 (fd_write)
    0x1a,       // drop
    0x0b,       // end
  ];
  const codeSection = wasmSection(10, [1, ...encodeLebU32(body.length), ...body]);
  const msgBytes = [...Buffer.from('hello wasi\n', 'utf8')];
  const dataSegment = [
    0, // active memory 0
    0x41, 0xc0, 0x00, 0x0b, // i32.const 64 (signed LEB128), end
    ...encodeLebU32(msgBytes.length),
    ...msgBytes,
  ];
  const dataSection = wasmSection(11, [1, ...dataSegment]);

  return new Uint8Array([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    ...typeSection,
    ...importSection,
    ...funcSection,
    ...memSection,
    ...exportSection,
    ...codeSection,
    ...dataSection,
  ]);
}

function buildWasiCanonicalWriteOpenModule() {
  const pathStr = 'prism/Rule.lex.tex';
  const typeSection = wasmSection(1, [
    2,
    0x60, 9, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7e, 0x7e, 0x7f, 0x7f, 1, 0x7f,
    0x60, 0, 0,
  ]);
  const importSection = wasmSection(2, [
    1,
    ...wasmStr('wasi_snapshot_preview1'),
    ...wasmStr('path_open'),
    0x00, 0,
  ]);
  const funcSection = wasmSection(3, [1, 1]);
  const memSection = wasmSection(5, [1, 0x00, 1]);
  const exportSection = wasmSection(7, [
    2,
    ...wasmStr('memory'), 0x02, 0,
    ...wasmStr('_start'), 0x00, 1,
  ]);
  const body = [
    0,
    0x41, 0x03,       // dirfd = 3
    0x41, 0x00,       // dirflags = 0
    0x41, 0xc0, 0x00, // pathPtr = 64
    0x41, pathStr.length, // pathLen = 18
    0x41, 0x00,       // oflags = 0 (no O_CREAT/O_TRUNC; only FD_WRITE right!)
    0x42, 0xc0, 0x00, // rights_base = 64n (FD_WRITE)
    0x42, 0x00,       // rights_inheriting = 0n
    0x41, 0x00,       // fdflags = 0
    0x41, 0x10,       // opened_fd ptr = 16
    0x10, 0x00,       // call 0 (path_open)
    0x1a,             // drop
    0x0b,             // end
  ];
  const codeSection = wasmSection(10, [1, ...encodeLebU32(body.length), ...body]);
  const pathBytes = [...Buffer.from(pathStr, 'utf8')];
  const dataSection = wasmSection(11, [1, 0, 0x41, 0xc0, 0x00, 0x0b, ...encodeLebU32(pathBytes.length), ...pathBytes]);
  return new Uint8Array([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    ...typeSection,
    ...importSection,
    ...funcSection,
    ...memSection,
    ...exportSection,
    ...codeSection,
    ...dataSection,
  ]);
}

async function testNodeAndEsbuildAdapter() {
  console.log('1. Testing node_exec isolated scope & Foundry esbuild.wasm adapter gating...');
  assert.ok(PYODIDE_LOCK && typeof PYODIDE_LOCK.version === 'string');
  assert.ok(String(PYODIDE_LOCK.url || '').startsWith('https://'));
  assert.ok(String(PYODIDE_LOCK.sri || '').startsWith('sha384-'));

  const bridge = new SpiralCoreHostBridge({ webAccessEnabled: true });

  // 1a. Plain JS node -e writes /workspace/hi.txt with process.version
  const res = await bridge.execute({
    op: 'node_exec',
    kind: 'node_exec',
    tool: 'node',
    args: [
      '-e',
      "const fs = require('fs'); fs.writeFileSync('/workspace/hi.txt', process.version);",
    ],
    cwd: '/workspace',
    workspace: [],
  });
  assert.equal(res.ok, true, `node_exec failed: ${JSON.stringify(res)}`);
  assert.equal(res.exit_code, 0, `node stderr: ${res.stderr}`);
  const hiWrite = (res.writes || []).find((w) => w.path === 'hi.txt' || w.path === '/workspace/hi.txt');
  assert.ok(hiWrite, `expected hi.txt write, got ${JSON.stringify(res.writes)}`);
  const hiText = b64Decode(hiWrite.content_base64).toString('utf8');
  assert.ok(hiText.startsWith('v'), `expected process.version starting with 'v', got ${hiText}`);

  // 1b. TS/ESM transpilation WITHOUT configured esbuildModuleUrl/esbuildWasmUrl MUST fail with EsbuildAdapterUnavailable (`R5`)
  const unconfiguredTs = await bridge.execute({
    op: 'node_exec',
    kind: 'node_exec',
    tool: 'node',
    args: ['/workspace/app.ts'],
    cwd: '/workspace',
    workspace: [
      {
        path: 'app.ts',
        content_base64: b64Encode('const answer: number = 42;\nconsole.log(answer);\n'),
      },
    ],
  });
  assert.equal(unconfiguredTs.ok, false);
  assert.equal(unconfiguredTs.error_code, 'EsbuildAdapterUnavailable');

  // 1c. Configure esbuildModuleUrl + esbuildWasmUrl via temporary local ESM module simulating Foundry studio/shell/compiler/esm/browser.min.js
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'uor-esbuild-mock-'));
  try {
    const mockEsbuildMod = path.join(tmpDir, 'browser.min.js');
    fs.writeFileSync(
      mockEsbuildMod,
      `export let initOpts = null;
export async function initialize(opts) {
  initOpts = opts;
}
export async function transform(code, opts) {
  const stripped = String(code || '').replace(/:\\s*number\\b/g, '').replace(/export\\s+const\\s+/g, 'exports.');
  return { code: stripped, warnings: [] };
}
export async function build(opts) {
  return { outputFiles: [{ text: 'console.log("bundled");' }] };
}
`,
    );
    const modUrl = pathToFileURL(mockEsbuildMod).href;
    const wasmUrl = 'https://foundry.local/studio/shell/compiler/esbuild.wasm';
    const configuredBridge = new SpiralCoreHostBridge({
      webAccessEnabled: true,
      esbuildModuleUrl: modUrl,
      esbuildWasmUrl: wasmUrl,
    });
    const tsRes = await configuredBridge.execute({
      op: 'node_exec',
      kind: 'node_exec',
      tool: 'node',
      args: ['/workspace/app.ts'],
      cwd: '/workspace',
      workspace: [
        {
          path: 'app.ts',
          content_base64: b64Encode('const answer: number = 42;\nconsole.log("ts=" + answer);\n'),
        },
      ],
    });
    assert.equal(tsRes.ok, true, `TS node_exec failed: ${JSON.stringify(tsRes)}`);
    assert.equal(tsRes.exit_code, 0, `TS stderr: ${tsRes.stderr}`);
    assert.equal(tsRes.stdout, 'ts=42\n');
    const esMod = await import(modUrl);
    assert.equal(esMod.initOpts && esMod.initOpts.wasmURL, wasmUrl);

    // 1d. Syntax errors in esbuild.transform & npx esbuild MUST return exit_code 1 + stderr cleanly (no uncaught rejection)
    const errMockMod = path.join(tmpDir, 'browser-err.min.js');
    fs.writeFileSync(
      errMockMod,
      `export async function initialize() {}
export async function transform(code) {
  if (String(code || '').includes('SYNTAX_BOOM')) {
    throw new Error('Transform failed with 1 error: Unexpected SYNTAX_BOOM');
  }
  return { code: String(code || '').replace(/:\\s*number\\b/g, ''), warnings: [] };
}
`,
    );
    const errBridge = new SpiralCoreHostBridge({
      webAccessEnabled: true,
      esbuildModuleUrl: pathToFileURL(errMockMod).href,
      esbuildWasmUrl: wasmUrl,
    });
    const tsSyntaxErr = await errBridge.execute({
      op: 'node_exec',
      kind: 'node_exec',
      tool: 'node',
      args: ['/workspace/bad.ts'],
      cwd: '/workspace',
      workspace: [{ path: 'bad.ts', content_base64: b64Encode('const x: number = SYNTAX_BOOM;\n') }],
    });
    assert.equal(tsSyntaxErr.ok, true);
    assert.equal(tsSyntaxErr.exit_code, 1);
    assert.ok(tsSyntaxErr.stderr.includes('SYNTAX_BOOM'), `expected transform error in stderr: ${tsSyntaxErr.stderr}`);

    const npxSyntaxErr = await errBridge.execute({
      op: 'node_exec',
      kind: 'node_exec',
      tool: 'npx',
      args: ['esbuild', '/workspace/bad.ts'],
      cwd: '/workspace',
      workspace: [{ path: 'bad.ts', content_base64: b64Encode('const x: number = SYNTAX_BOOM;\n') }],
    });
    assert.equal(npxSyntaxErr.ok, true);
    assert.equal(npxSyntaxErr.exit_code, 1);
    assert.ok(npxSyntaxErr.stderr.includes('SYNTAX_BOOM'));

    // 1e. node:assert, node:util, fs.mkdirSync + existsSync(dir), statSync/lstatSync isSymbolicLink parity, & setCompilerConfig reset
    const nodeBuiltinsRes = await errBridge.execute({
      op: 'node_exec',
      kind: 'node_exec',
      tool: 'node',
      args: [
        '-e',
        [
          "const fs = require('fs');",
          "const a = require('node:assert');",
          "const u = require('node:util');",
          "fs.mkdirSync('/workspace/d/sub', { recursive: true });",
          "a.ok(fs.existsSync('/workspace/d/sub'), 'mkdirSync dir must exist');",
          "fs.writeFileSync('/workspace/target.txt', 'hi');",
          "fs.symlinkSync('/workspace/target.txt', '/workspace/sym.txt');",
          "a.strictEqual(fs.lstatSync('/workspace/sym.txt').isSymbolicLink(), true);",
          "a.strictEqual(fs.statSync('/workspace/sym.txt').isSymbolicLink(), false);",
          "console.log(u.format('built=%s', 'ok'));",
        ].join(' '),
      ],
      cwd: '/workspace',
      workspace: [],
    });
    assert.equal(nodeBuiltinsRes.ok, true, `nodeBuiltins failed: ${JSON.stringify(nodeBuiltinsRes)}`);
    assert.equal(nodeBuiltinsRes.exit_code, 0, `nodeBuiltins stderr: ${nodeBuiltinsRes.stderr}`);
    assert.equal(nodeBuiltinsRes.stdout.trim(), 'built=ok');

    // Resetting esbuildModuleUrl / esbuildWasmUrl to '' must re-arm EsbuildAdapterUnavailable
    errBridge.setCompilerConfig({ esbuildModuleUrl: '', esbuildWasmUrl: '' });
    const resetTs = await errBridge.execute({
      op: 'node_exec',
      kind: 'node_exec',
      tool: 'node',
      args: ['/workspace/app.ts'],
      cwd: '/workspace',
      workspace: [{ path: 'app.ts', content_base64: b64Encode('const n: number = 1;\nconsole.log(n);\n') }],
    });
    assert.equal(resetTs.ok, false);
    assert.equal(resetTs.error_code, 'EsbuildAdapterUnavailable');
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
  console.log('  ✓ node_exec + EsbuildAdapterUnavailable + Foundry esbuild adapter verified.');
}

async function testNpmSriAndTarExtraction() {
  console.log('2. Testing npm install SHA-512 SRI verification & tgz extraction...');
  const tarBuf = buildUstarTar([
    {
      name: 'package/package.json',
      data: JSON.stringify({
        name: 'demo-pad',
        version: '1.2.3',
        main: 'index.js',
        bin: { 'demo-pad': 'bin/cli.js' },
      }),
    },
    {
      name: 'package/index.js',
      data: 'module.exports = function pad(v) { return "[" + v + "]"; };\n',
    },
    {
      name: 'package/bin/cli.js',
      data: '#!/usr/bin/env node\nconsole.log(require("../index.js")("bin"));\n',
      mode: 0o755,
    },
  ]);
  const tgzBuf = zlib.gzipSync(tarBuf);
  const validSha512 = 'sha512-' + crypto.createHash('sha512').update(tgzBuf).digest('base64');
  const badSha512 = 'sha512-' + crypto.createHash('sha512').update(Buffer.from('tampered-tarball')).digest('base64');

  const metaForPkg = (integrity) => ({
    name: 'demo-pad',
    'dist-tags': { latest: '1.2.3' },
    versions: {
      '1.2.3': {
        name: 'demo-pad',
        version: '1.2.3',
        dist: {
          tarball: 'https://registry.npmjs.org/demo-pad/-/demo-pad-1.2.3.tgz',
          integrity,
        },
      },
    },
  });

  // 2a. Tampered SRI MUST fail with IntegrityMismatch and emit zero extracted files
  const badBridge = new SpiralCoreHostBridge({
    webAccessEnabled: true,
    fetchImpl: async (url) => {
      const u = String(url);
      if (u === 'https://registry.npmjs.org/demo-pad') {
        return new Response(JSON.stringify(metaForPkg(badSha512)), { status: 200 });
      }
      if (u === 'https://registry.npmjs.org/demo-pad/-/demo-pad-1.2.3.tgz') {
        return new Response(tgzBuf, { status: 200 });
      }
      throw new Error(`Unexpected fetch: ${u}`);
    },
  });
  const badNpm = await badBridge.execute({
    op: 'pkg_fetch',
    kind: 'pkg_fetch',
    ecosystem: 'npm',
    tool: 'npm',
    args: ['install', 'demo-pad'],
    cwd: '/workspace',
    workspace: [],
  });
  assert.equal(badNpm.ok, false);
  assert.equal(badNpm.error_code, 'IntegrityMismatch');
  assert.equal((badNpm.writes || []).length, 0);

  // 2b. Valid SHA-512 SRI extracts node_modules/demo-pad/**, updates package.json, and resolves require('demo-pad')
  const goodBridge = new SpiralCoreHostBridge({
    webAccessEnabled: true,
    fetchImpl: async (url) => {
      const u = String(url);
      if (u === 'https://registry.npmjs.org/demo-pad') {
        return new Response(JSON.stringify(metaForPkg(validSha512)), { status: 200 });
      }
      if (u === 'https://registry.npmjs.org/demo-pad/-/demo-pad-1.2.3.tgz') {
        return new Response(tgzBuf, { status: 200 });
      }
      throw new Error(`Unexpected fetch: ${u}`);
    },
  });
  const goodNpm = await goodBridge.execute({
    op: 'pkg_fetch',
    kind: 'pkg_fetch',
    ecosystem: 'npm',
    tool: 'npm',
    args: ['install', 'demo-pad'],
    cwd: '/workspace',
    workspace: [
      { path: 'package.json', content_base64: b64Encode('{"name":"app","version":"0.1.0"}\n') },
    ],
  });
  assert.equal(goodNpm.ok, true, `goodNpm failed: ${JSON.stringify(goodNpm)}`);
  assert.equal(goodNpm.exit_code, 0, `npm stderr: ${goodNpm.stderr}`);

  const pkgJsonWrite = (goodNpm.writes || []).find((w) => w.path === 'package.json');
  assert.ok(pkgJsonWrite, 'npm install must update package.json');
  const pkgJson = JSON.parse(b64Decode(pkgJsonWrite.content_base64).toString('utf8'));
  assert.equal(pkgJson.dependencies['demo-pad'], '^1.2.3');

  const idxWrite = (goodNpm.writes || []).find((w) => w.path === 'node_modules/demo-pad/index.js');
  assert.ok(idxWrite, 'npm install must extract node_modules/demo-pad/index.js');

  const reqRes = await goodBridge.execute({
    op: 'node_exec',
    kind: 'node_exec',
    tool: 'node',
    args: ['-e', "const pad = require('demo-pad'); console.log(pad('ok'));"],
    cwd: '/workspace',
    workspace: goodNpm.writes,
  });
  assert.equal(reqRes.ok, true);
  assert.equal(reqRes.exit_code, 0, `require stderr: ${reqRes.stderr}`);
  assert.equal(reqRes.stdout, '[ok]\n');

  // 2c. `npx <pkg>` auto-installs pkg with relative `.bin/` symlink + preserves node_modules writes & resolves relative require('../lib/helper')
  const npxTarBuf = buildUstarTar([
    {
      name: 'package/package.json',
      data: JSON.stringify({ name: 'rel-cli', version: '2.0.0', bin: { 'rel-cli': 'bin/cli.js' } }),
    },
    {
      name: 'package/lib/helper.js',
      data: "module.exports = function fmt(v) { return 'HELPER:' + v; };\n",
    },
    {
      name: 'package/bin/cli.js',
      data: "#!/usr/bin/env node\nconst fmt = require('../lib/helper');\nconsole.log(fmt(process.argv[2] || 'default'));\n",
      mode: 0o755,
    },
  ]);
  const npxTgz = zlib.gzipSync(npxTarBuf);
  const npxSri = 'sha512-' + crypto.createHash('sha512').update(npxTgz).digest('base64');
  const npxBridge = new SpiralCoreHostBridge({
    webAccessEnabled: true,
    fetchImpl: async (url) => {
      const u = String(url);
      if (u === 'https://registry.npmjs.org/rel-cli') {
        return new Response(
          JSON.stringify({
            name: 'rel-cli',
            'dist-tags': { latest: '2.0.0' },
            versions: {
              '2.0.0': {
                name: 'rel-cli',
                version: '2.0.0',
                bin: { 'rel-cli': 'bin/cli.js' },
                dist: { tarball: 'https://registry.npmjs.org/rel-cli/-/rel-cli-2.0.0.tgz', integrity: npxSri },
              },
            },
          }),
          { status: 200 },
        );
      }
      if (u === 'https://registry.npmjs.org/rel-cli/-/rel-cli-2.0.0.tgz') {
        return new Response(npxTgz, { status: 200 });
      }
      throw new Error(`Unexpected fetch: ${u}`);
    },
  });
  const npxRes = await npxBridge.execute({
    op: 'node_exec',
    kind: 'node_exec',
    tool: 'npx',
    args: ['rel-cli', 'npx-arg'],
    cwd: '/workspace',
    workspace: [],
  });
  assert.equal(npxRes.ok, true, `npx failed: ${JSON.stringify(npxRes)}`);
  assert.equal(npxRes.exit_code, 0, `npx stderr: ${npxRes.stderr}`);
  assert.equal(npxRes.stdout, 'HELPER:npx-arg\n');
  assert.ok(
    (npxRes.writes || []).some((w) => w.path === 'node_modules/rel-cli/bin/cli.js'),
    'npx auto-install must preserve node_modules/** writes',
  );
  assert.ok(
    (npxRes.writes || []).some((w) => w.path === 'package.json'),
    'npx auto-install must preserve package.json writes',
  );
  console.log('  ✓ npm SHA-512 IntegrityMismatch rejection + tgz extraction + npx relative require() verified.');
}

async function testPyodideLazySriAndPythonPip() {
  console.log('3. Testing lazy Pyodide SHA-384 SRI enforcement, Cache API & python/pip...');
  let pyodideFetchCount = 0;
  const cacheMap = new Map();
  const prevCaches = globalThis.caches;

  try {
    globalThis.caches = {
      async open(name) {
        assert.equal(name, 'uor-pyodide-v1');
        return {
          async match(reqUrl) {
            return cacheMap.get(String(reqUrl)) || undefined;
          },
          async put(reqUrl, resp) {
            const bytes = new Uint8Array(await resp.arrayBuffer());
            cacheMap.set(String(reqUrl), new Response(bytes));
          },
        };
      },
    };

    // 3a. Tampered Pyodide bytes MUST fail against locked PYODIDE_LOCK.sri without touching Cache API
    const badBridge = new SpiralCoreHostBridge({
      webAccessEnabled: true,
      fetchImpl: async (url) => {
        if (String(url) === PYODIDE_LOCK.url) {
          pyodideFetchCount += 1;
          return new Response('console.log("tampered pyodide");', { status: 200 });
        }
        throw new Error(`Unexpected fetch: ${url}`);
      },
    });
    // Non-python commands must NOT trigger Pyodide fetch (lazy load only)
    await badBridge.execute({
      op: 'sqlite_exec',
      kind: 'sqlite_exec',
      args: ['/workspace/lazy_check.db', 'SELECT 1;'],
      cwd: '/workspace',
      workspace: [],
    });
    assert.equal(pyodideFetchCount, 0, 'Pyodide must remain lazy before python/pip');

    const badPy = await badBridge.execute({
      op: 'python_exec',
      kind: 'python_exec',
      tool: 'python3',
      args: ['-c', 'print(1)'],
      cwd: '/workspace',
      workspace: [],
    });
    assert.equal(pyodideFetchCount, 1);
    assert.equal(badPy.ok, false);
    assert.equal(badPy.error_code, 'IntegrityMismatch');
    assert.equal(cacheMap.size, 0, 'Tampered Pyodide asset must NEVER be cached');

    // 3b. Valid SRI-verified Pyodide bootstrap + cached 2nd-call stdout/stderr + sys.argv/environ/stdin + FS prune + sync HTTP
    const mockPyJs = Buffer.from(
      `export async function loadPyodide(opts) {
        const fsStore = new Map();
        const pyEnv = { HOME: '/root' };
        let pyArgv = [];
        let pyStdin = '';
        let shimCode = '';
        let stdoutCb = (s) => opts && opts.stdout && opts.stdout(s);
        let stderrCb = (s) => opts && opts.stderr && opts.stderr(s);
        return {
          FS: {
            mkdirTree() {},
            writeFile(p, d) { fsStore.set(p, Buffer.from(d)); },
            readFile(p) { return new Uint8Array(fsStore.get(p) || []); },
            unlink(p) { fsStore.delete(p); },
            readdir(dir) {
              const out = ['.', '..'];
              for (const k of fsStore.keys()) {
                if (k.startsWith(dir + '/')) {
                  const rest = k.slice(dir.length + 1).split('/')[0];
                  if (rest && !out.includes(rest)) out.push(rest);
                }
              }
              return out;
            },
            analyzePath(p) {
              const isFile = fsStore.has(p);
              return { exists: isFile || [...fsStore.keys()].some((k) => k.startsWith(p + '/')), object: { isFolder: !isFile } };
            },
          },
          setStdout(cfg) { if (cfg && typeof cfg.batched === 'function') stdoutCb = cfg.batched; },
          setStderr(cfg) { if (cfg && typeof cfg.batched === 'function') stderrCb = cfg.batched; },
          async loadPackage() {},
          async runPythonAsync(code) {
            const c = String(code || '');
            if (c.includes('def _uor_sync_urlopen')) {
              shimCode = c;
            }
            if (c.includes('sys.argv =')) {
              const mArgv = /sys\\.argv = (\\[[^\\n]*\\])/.exec(c);
              if (mArgv) pyArgv = JSON.parse(mArgv[1]);
              const mEnv = /_uor_env = (\\{[^\\n]*\\})/.exec(c);
              if (mEnv) Object.assign(pyEnv, JSON.parse(mEnv[1]));
              const mIn = /_uor_stdin_str = ("[^\\n]*")/.exec(c);
              if (mIn) pyStdin = JSON.parse(mIn[1]);
              return null;
            }
            if (c.includes('micropip')) {
              return 'installed:rich';
            }
            if (c.includes('CALL_1')) {
              stdoutCb('out:call1:' + JSON.stringify(pyArgv) + ':' + pyEnv.PY_MODE + ':' + pyStdin);
              stderrCb('err:call1');
              fsStore.set('/workspace/temp_py.txt', Buffer.from('ephemeral\\n', 'utf8'));
              fsStore.set('/workspace/py_out.txt', Buffer.from('py-ok\\n', 'utf8'));
              return '1';
            }
            if (c.includes('CALL_2')) {
              stdoutCb('out:call2:' + JSON.stringify(pyArgv) + ':' + (fsStore.has('/workspace/temp_py.txt') ? 'STALE' : 'PRUNED') + ':' + pyStdin);
              stderrCb('err:call2');
              if (!shimCode || shimCode.includes('async def _uor_async_fetch')) {
                throw new Error('Pyodide HTTP shim must be synchronous');
              }
              if (typeof globalThis.__uorSpiralCorePySyncFetch !== 'function') {
                throw new Error('Missing synchronous __uorSpiralCorePySyncFetch hook');
              }
              const getRes = globalThis.__uorSpiralCorePySyncFetch('https://api.github.com/uor', 'GET', '', '{}');
              const postRes = globalThis.__uorSpiralCorePySyncFetch('https://api.github.com/uor', 'POST', '{"k":1}', JSON.stringify({ Authorization: 'Bearer {{MVM_SECRET:GITHUB_TOKEN}}' }));
              stdoutCb('sync:' + getRes.text + ':' + postRes.text);
              return '2';
            }
            fsStore.set('/workspace/py_out.txt', Buffer.from('py-ok\\n', 'utf8'));
            return '42';
          },
        };
      }
      `,
      'utf8',
    );
    const mockSri = 'sha384-' + crypto.createHash('sha384').update(mockPyJs).digest('base64');
    const verifiedBridge = new SpiralCoreHostBridge({
      webAccessEnabled: true,
      pyodideLock: { version: '0.27.2-test', url: 'https://cdn.jsdelivr.net/pyodide/test/pyodide.mjs', sri: mockSri },
      syncFetchImpl: (url, init) => {
        if (init && init.method === 'POST') {
          return { status: 200, text: `post-auth=${init.headers && init.headers.Authorization}` };
        }
        return { status: 200, text: `get-ok=${url}` };
      },
      fetchImpl: async (url) => {
        if (String(url) === 'https://cdn.jsdelivr.net/pyodide/test/pyodide.mjs') {
          return new Response(mockPyJs, { status: 200 });
        }
        if (String(url).includes('pypi.org/pypi/')) {
          return new Response(JSON.stringify({ info: { name: 'rich', version: '13.9.4' } }), { status: 200 });
        }
        throw new Error(`Unexpected fetch: ${url}`);
      },
    });
    verifiedBridge.setSecret('GITHUB_TOKEN', 'ghp_sync_tok');
    const pyOk1 = await verifiedBridge.execute({
      op: 'python_exec',
      kind: 'python_exec',
      tool: 'python3',
      args: ['-c', 'CALL_1', 'flagA'],
      cwd: '/workspace',
      env: { PY_MODE: 'alpha' },
      stdin_utf8: 'in-one',
      workspace: [],
    });
    assert.equal(pyOk1.ok, true, `pyOk1 failed: ${JSON.stringify(pyOk1)}`);
    assert.equal(pyOk1.exit_code, 0, `pyOk1 stderr: ${pyOk1.stderr}`);
    assert.ok(cacheMap.has('https://cdn.jsdelivr.net/pyodide/test/pyodide.mjs'), 'Verified Pyodide must be cached in uor-pyodide-v1');
    assert.ok(pyOk1.stdout.includes('out:call1:["-c","flagA"]:alpha:in-one'), `unexpected call1 stdout: ${pyOk1.stdout}`);
    assert.ok(pyOk1.stderr.includes('err:call1'), `unexpected call1 stderr: ${pyOk1.stderr}`);
    assert.ok((pyOk1.writes || []).some((w) => w.path === 'py_out.txt'), 'Python /workspace writes must propagate');

    // Call #2 on cached Pyodide: stdout/stderr MUST stay intact, temp_py.txt MUST be pruned from pyodide.FS, & sync fetch must work
    const pyOk2 = await verifiedBridge.execute({
      op: 'python_exec',
      kind: 'python_exec',
      tool: 'python3',
      args: ['-c', 'CALL_2', 'flagB'],
      cwd: '/workspace',
      env: { PY_MODE: 'beta' },
      stdin_utf8: 'in-two',
      workspace: [{ path: 'py_out.txt', content_base64: b64Encode('py-ok\n') }],
    });
    assert.equal(pyOk2.ok, true, `pyOk2 failed: ${JSON.stringify(pyOk2)}`);
    assert.equal(pyOk2.exit_code, 0, `pyOk2 stderr: ${pyOk2.stderr}`);
    assert.ok(pyOk2.stdout.includes('out:call2:["-c","flagB"]:PRUNED:in-two'), `Call 2 lost stdout or stale FS file persisted: ${pyOk2.stdout}`);
    assert.ok(pyOk2.stdout.includes('sync:get-ok=https://api.github.com/uor:post-auth=Bearer ghp_sync_tok'), `Sync HTTP hook failed: ${pyOk2.stdout}`);
    assert.ok(pyOk2.stderr.includes('err:call2'), `Call 2 lost stderr: ${pyOk2.stderr}`);
    assert.ok(!(pyOk2.writes || []).some((w) => w.path === 'temp_py.txt'), 'Deleted workspace file must NOT be resurrected by pyodide.FS');

    const pipOk = await verifiedBridge.execute({
      op: 'python_exec',
      kind: 'python_exec',
      tool: 'pip',
      args: ['install', 'rich'],
      cwd: '/workspace',
      workspace: [],
    });
    assert.equal(pipOk.ok, true);
    assert.equal(pipOk.exit_code, 0, `pip stderr: ${pipOk.stderr}`);
  } finally {
    globalThis.caches = prevCaches;
  }
  console.log('  ✓ Lazy Pyodide SRI rejection, Cache API storage & python/pip verified.');
}

async function testWasiPreview1AndSqlite3() {
  console.log('4. Testing WASI Preview 1 wasm run & persistent binary sqlite3...');
  const bridge = new SpiralCoreHostBridge({ webAccessEnabled: true });

  // 4a. WASI Preview 1 minimal module
  const wasmBytes = buildMinimalWasiStdoutModule();
  const wasiRes = await bridge.execute({
    op: 'wasi_exec',
    kind: 'wasi_exec',
    tool: 'wasm',
    args: ['run', '/workspace/mod.wasm', '--hello'],
    cwd: '/workspace',
    workspace: [{ path: 'mod.wasm', content_base64: b64Encode(wasmBytes) }],
  });
  assert.equal(wasiRes.ok, true, `wasi_exec failed: ${JSON.stringify(wasiRes)}`);
  assert.equal(wasiRes.exit_code, 0, `wasi stderr: ${wasiRes.stderr}`);
  assert.equal(wasiRes.stdout, 'hello wasi\n');

  // 4b. Missing wasm file returns non-zero typed WasiRuntimeError or clean error
  const badWasi = await bridge.execute({
    op: 'wasi_exec',
    kind: 'wasi_exec',
    tool: 'wasm',
    args: ['run', '/workspace/missing.wasm'],
    cwd: '/workspace',
    workspace: [],
  });
  assert.equal(badWasi.ok, false);
  assert.equal(badWasi.error_code, 'WasiRuntimeError');

  // 4b-bis. WASI path_open with writable rights on canonical file MUST fail upfront before fd_write
  const canonWasiBytes = buildWasiCanonicalWriteOpenModule();
  const canonWasi = await bridge.execute({
    op: 'wasi_exec',
    kind: 'wasi_exec',
    tool: 'wasm',
    args: ['run', '/workspace/canon_open.wasm'],
    cwd: '/workspace',
    workspace: [
      { path: 'canon_open.wasm', content_base64: b64Encode(canonWasiBytes) },
      { path: 'prism/Rule.lex.tex', content_base64: b64Encode('canonical\n') },
    ],
  });
  assert.equal(canonWasi.ok, false);
  assert.equal(canonWasi.error_code, 'CanonicalPathProtected');

  // 4c. sqlite3 create + insert + select, then reopen persisted binary /workspace/app.db
  const sqlRes1 = await bridge.execute({
    op: 'sqlite_exec',
    kind: 'sqlite_exec',
    tool: 'sqlite3',
    args: ['/workspace/app.db', 'CREATE TABLE t(x INT); INSERT INTO t VALUES (42); SELECT x FROM t;'],
    cwd: '/workspace',
    workspace: [],
  });
  assert.equal(sqlRes1.ok, true, `sqlite_exec failed: ${JSON.stringify(sqlRes1)}`);
  assert.equal(sqlRes1.exit_code, 0, `sqlite stderr: ${sqlRes1.stderr}`);
  assert.equal(sqlRes1.stdout.trim(), '42');
  const dbWrite = (sqlRes1.writes || []).find((w) => w.path === 'app.db');
  assert.ok(dbWrite, 'sqlite3 must write app.db');
  const dbBytes = b64Decode(dbWrite.content_base64);
  assert.equal(dbBytes.subarray(0, 16).toString('utf8'), 'SQLite format 3\0', 'Persisted DB must carry SQLite 3 binary header');

  const sqlRes2 = await bridge.execute({
    op: 'sqlite_exec',
    kind: 'sqlite_exec',
    tool: 'sqlite3',
    args: ['/workspace/app.db', 'SELECT x FROM t;'],
    cwd: '/workspace',
    workspace: [dbWrite],
  });
  assert.equal(sqlRes2.ok, true);
  assert.equal(sqlRes2.exit_code, 0);
  assert.equal(sqlRes2.stdout.trim(), '42');

  // 4d. Canonical protection in polyglot adapters (sqlite3 targeting prism/Rule.lex.tex or app/state)
  const canonSql = await bridge.execute({
    op: 'sqlite_exec',
    kind: 'sqlite_exec',
    tool: 'sqlite3',
    args: ['/workspace/prism/Rule.lex.tex', 'CREATE TABLE x(a INT);'],
    cwd: '/workspace',
    workspace: [],
  });
  assert.equal(canonSql.ok, false);
  assert.equal(canonSql.error_code, 'CanonicalPathProtected');

  // 4e. Corrupt binary .db refusal (bad magic -> SqliteRuntimeError, corrupt JSON payload -> CorruptSqliteDatabase)
  const corruptSql = await bridge.execute({
    op: 'sqlite_exec',
    kind: 'sqlite_exec',
    tool: 'sqlite3',
    args: ['/workspace/corrupt.db', 'SELECT 1;'],
    cwd: '/workspace',
    workspace: [{ path: 'corrupt.db', content_base64: b64Encode('NOT_SQLITE_BYTES') }],
  });
  assert.equal(corruptSql.ok, false);
  assert.equal(corruptSql.error_code, 'SqliteRuntimeError');

  const corruptJsonBuf = Buffer.alloc(4096, 0);
  corruptJsonBuf.write('SQLite format 3\0', 0, 'ascii');
  corruptJsonBuf.writeUInt32LE(8, 96);
  corruptJsonBuf.write('{badjson', 100, 'utf8');
  const corruptJsonSql = await bridge.execute({
    op: 'sqlite_exec',
    kind: 'sqlite_exec',
    tool: 'sqlite3',
    args: ['/workspace/corrupt_json.db', 'SELECT 1;'],
    cwd: '/workspace',
    workspace: [{ path: 'corrupt_json.db', content_base64: b64Encode(corruptJsonBuf) }],
  });
  assert.equal(corruptJsonSql.ok, false);
  assert.equal(corruptJsonSql.error_code, 'CorruptSqliteDatabase');

  // 4f. `-cmd` before positional dbPath + INSERT OR REPLACE / INSERT OR IGNORE + true LEFT JOIN
  const cmdSql = await bridge.execute({
    op: 'sqlite_exec',
    kind: 'sqlite_exec',
    tool: 'sqlite3',
    args: [
      '-cmd',
      '.mode json',
      '/workspace/adv.db',
      [
        'CREATE TABLE users(id INT PRIMARY KEY, name TEXT);',
        'CREATE TABLE orders(oid INT PRIMARY KEY, user_id INT, item TEXT);',
        "INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob');",
        "INSERT OR IGNORE INTO users VALUES (1, 'IgnoredAlice');",
        "INSERT OR REPLACE INTO users VALUES (2, 'Bobby');",
        "INSERT INTO orders VALUES (10, 1, 'Book');",
        'SELECT u.name, o.item FROM users u LEFT JOIN orders o ON u.id = o.user_id ORDER BY u.id;',
      ].join(' '),
    ],
    cwd: '/workspace',
    workspace: [],
  });
  assert.equal(cmdSql.ok, true, `cmdSql failed: ${JSON.stringify(cmdSql)}`);
  assert.equal(cmdSql.exit_code, 0, `cmdSql stderr: ${cmdSql.stderr}`);
  const joinedRows = JSON.parse(cmdSql.stdout);
  assert.deepEqual(joinedRows, [
    { 'u.name': 'Alice', 'o.item': 'Book' },
    { 'u.name': 'Bobby', 'o.item': null },
  ]);
  console.log('  ✓ WASI Preview 1 & persistent binary sqlite3 (-cmd, CorruptSqliteDatabase, OR REPLACE|IGNORE, LEFT JOIN) verified.');
}

async function testWorkerPolyglotLoop() {
  console.log('5. Testing full Worker terminal:write / set_host_config / NeedsBridge loop...');
  const prevWgs = globalThis.WorkerGlobalScope;
  const prevSelf = globalThis.self;
  const prevWbg = globalThis.__wbg_init;
  const prevPanic = globalThis.init_panic_hook;
  const prevBoot = globalThis.studio_bootstrap;
  const prevPack = globalThis.studio_weight_pack_status;
  const prevDispatch = globalThis.studio_dispatch;
  const prevSyncIn = globalThis.mvm_sync_workspace_in_json;
  const prevExportDiff = globalThis.mvm_export_workspace_diff_json;
  const prevCowExport = globalThis.studio_export_mvm_cow_sectors;
  const prevCowImport = globalThis.studio_import_mvm_cow_sectors;

  const rpcWaiters = new Map();
  const waitForRpc = (id) =>
    new Promise((resolve, reject) => {
      const t = setTimeout(() => reject(new Error(`Timeout waiting for Worker RPC ${id}`)), 3000);
      rpcWaiters.set(id, (msg) => {
        clearTimeout(t);
        resolve(msg);
      });
    });

  let readyResolve;
  const readyPromise = new Promise((r) => {
    readyResolve = r;
  });

  try {
    class MockWorkerGlobalScope {
      constructor() {
        this.name = 'uor-r4-engine-worker';
      }
      postMessage(msg) {
        if (msg && msg.event === 'ready') readyResolve(msg);
        if (msg && msg.id != null && typeof msg.ok === 'boolean' && rpcWaiters.has(msg.id)) {
          const cb = rpcWaiters.get(msg.id);
          rpcWaiters.delete(msg.id);
          cb(msg);
        }
      }
    }
    const mockSelf = new MockWorkerGlobalScope();
    globalThis.WorkerGlobalScope = MockWorkerGlobalScope;
    globalThis.self = mockSelf;
    globalThis.__wbg_init = async () => ({});
    globalThis.init_panic_hook = () => {};
    globalThis.studio_bootstrap = () => '{}';
    globalThis.studio_weight_pack_status = () => JSON.stringify({ mounted_packs: [] });
    globalThis.studio_export_mvm_cow_sectors = () => new Uint8Array([77, 86, 77]);
    globalThis.studio_import_mvm_cow_sectors = () => JSON.stringify({ status: 'ok', sectors: 0 });
    globalThis.mvm_sync_workspace_in_json = () => JSON.stringify({ status: 'ok', epoch: 1, applied_writes: 0, applied_deletes: 0 });

    const virtualMvmFiles = new Map();
    globalThis.mvm_export_workspace_diff_json = () => {
      const writes = [];
      let ignoredHeavy = 0;
      for (const [k, b64] of virtualMvmFiles.entries()) {
        if (k.endsWith('.db') || k.endsWith('.wasm') || k.startsWith('node_modules/')) {
          ignoredHeavy += 1;
        } else {
          writes.push({ path: k, content_base64: b64 });
        }
      }
      return JSON.stringify({ status: 'ok', epoch: 2, writes, deletes: [], ignored_heavy_paths: ignoredHeavy });
    };

    let lastBridgeReq = null;
    globalThis.studio_dispatch = (env) => {
      if (env.includes('\tworkspace_list\t')) {
        return JSON.stringify({ status: 'ok', files: [] });
      }
      if (env.includes('\tmvm_exec\t')) {
        const payload = JSON.parse(env.split('\t').slice(3).join('\t'));
        const cmd = String(payload.cmd || '');
        if (cmd.startsWith('node ')) {
          lastBridgeReq = {
            op: 'node_exec',
            kind: 'node_exec',
            tool: 'node',
            args: ['-e', "const fs = require('fs'); fs.writeFileSync('/workspace/hi.txt', process.version);"],
            cwd: '/workspace',
            workspace: [],
          };
          return JSON.stringify({
            status: 'needs_bridge',
            session_id: 'w-node-1',
            resume_state: { bridge_kind: 'node_exec' },
            bridge_req: lastBridgeReq,
          });
        }
        if (cmd.startsWith('sqlite3 ')) {
          lastBridgeReq = {
            op: 'sqlite_exec',
            kind: 'sqlite_exec',
            tool: 'sqlite3',
            args: ['/workspace/app.db', 'CREATE TABLE t(x INT); INSERT INTO t VALUES (42); SELECT x FROM t;'],
            cwd: '/workspace',
            workspace: [],
          };
          return JSON.stringify({
            status: 'needs_bridge',
            session_id: 'w-sql-1',
            resume_state: { bridge_kind: 'sqlite_exec' },
            bridge_req: lastBridgeReq,
          });
        }
      }
      if (env.includes('\tmvm_complete_bridge\t')) {
        const comp = JSON.parse(env.split('\t').slice(3).join('\t'));
        const resp = comp.bridge_resp || {};
        for (const w of resp.writes || []) {
          virtualMvmFiles.set(String(w.path).replace(/^\/workspace\//, ''), w.content_base64);
        }
        return JSON.stringify({
          status: 'ok',
          exit_code: resp.exit_code ?? 0,
          mvm_stdout: resp.stdout || '',
          mvm_stderr: resp.stderr || '',
        });
      }
      return JSON.stringify({ status: 'ok' });
    };

    await import('../../crates/wasm/js/worker_host_bridge.js?polyglot_worker_e2e=1');
    await readyPromise;

    // 1. Configure Foundry esbuild URLs via set_host_config
    const cfgP = waitForRpc('cfg-1');
    mockSelf.onmessage({
      data: {
        id: 'cfg-1',
        op: 'set_host_config',
        payload: {
          esbuildModuleUrl: 'https://foundry.local/studio/shell/compiler/esm/browser.min.js',
          esbuildWasmUrl: 'https://foundry.local/studio/shell/compiler/esbuild.wasm',
        },
      },
    });
    const cfgRes = await cfgP;
    assert.equal(cfgRes.ok, true);
    assert.equal(cfgRes.result.esbuild_configured, true);

    // 2. terminal:write node -e -> NeedsBridge -> mvm_complete_bridge -> workspace_diff contains hi.txt
    const nodeP = waitForRpc('term-node-1');
    mockSelf.onmessage({
      data: {
        id: 'term-node-1',
        op: 'terminal:write',
        payload: { cmd: 'node -e "const fs = require(\'fs\'); fs.writeFileSync(\'/workspace/hi.txt\', process.version);"' },
      },
    });
    const nodeRes = await nodeP;
    assert.equal(nodeRes.ok, true);
    assert.equal(nodeRes.result.exit_code, 0);
    assert.ok(nodeRes.result.changed_files.includes('hi.txt'));

    // 3. terminal:write sqlite3 -> app.db stays isolated from Studio text diff (ignored_heavy_paths >= 1)
    const sqlP = waitForRpc('term-sql-1');
    mockSelf.onmessage({
      data: {
        id: 'term-sql-1',
        op: 'terminal:write',
        payload: { cmd: 'sqlite3 /workspace/app.db "CREATE TABLE t(x INT); INSERT INTO t VALUES (42); SELECT x FROM t;"' },
      },
    });
    const sqlRes = await sqlP;
    assert.equal(sqlRes.ok, true);
    assert.equal(sqlRes.result.exit_code, 0);
    assert.equal(sqlRes.result.mvm_stdout.trim(), '42');
    assert.ok(sqlRes.result.ignored_heavy_paths >= 1);
    assert.ok(!sqlRes.result.changed_files.includes('app.db'));
  } finally {
    globalThis.WorkerGlobalScope = prevWgs;
    globalThis.self = prevSelf;
    globalThis.__wbg_init = prevWbg;
    globalThis.init_panic_hook = prevPanic;
    globalThis.studio_bootstrap = prevBoot;
    globalThis.studio_weight_pack_status = prevPack;
    globalThis.studio_dispatch = prevDispatch;
    globalThis.mvm_sync_workspace_in_json = prevSyncIn;
    globalThis.mvm_export_workspace_diff_json = prevExportDiff;
    globalThis.studio_export_mvm_cow_sectors = prevCowExport;
    globalThis.studio_import_mvm_cow_sectors = prevCowImport;
  }
  console.log('  ✓ Worker terminal:write / set_host_config / NeedsBridge polyglot loop verified.');
}

async function main() {
  console.log('=== E2E: MVM Polyglot Wasm Container (R4-58) ===');
  await testNodeAndEsbuildAdapter();
  await testNpmSriAndTarExtraction();
  await testPyodideLazySriAndPythonPip();
  await testWasiPreview1AndSqlite3();
  await testWorkerPolyglotLoop();
  console.log('=== ALL MVM POLYGLOT E2E CHECKS PASSED ===');
}

main().catch((err) => {
  console.error('MVM POLYGLOT E2E FAILED:', err);
  process.exit(1);
});
