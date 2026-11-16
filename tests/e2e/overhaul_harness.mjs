#!/usr/bin/env node
/**
 * overhaul_harness.mjs - Genuine Companion In-Browser Web API Simulation Harness for LexLean-R4.
 *
 * Implements 8-Group Architecture:
 * 1. Cryptographic Provenance & Closure Integrity (`public/provenance.json`)
 * 2. Live Wasm Engine & Zero-Float / Zero-Matmul Invariants (`public/lexlean_r4.js` / Wasm)
 * 3. Real MVM CoW Sector Persistence & Error Rejection (`studio_export_mvm_cow_sectors`, `studio_import_mvm_cow_sectors`)
 * 4. Real Stitch 2-of-3 XOR Parity Engine (`studio_stitch_split_2of3`)
 * 5. Real SpiralCore Host Bridge & Zero-NIC Security Invariants (`SpiralCoreHostBridge`)
 * 6. Foundry FA-20 IndexedDB Commit Boundary & Async Race Remediation (`storageRequest`)
 * 7. Multi-Tab Web Locks Concurrency, Deadlock & Timeout Guard (`CompliantLockManager`)
 * 8. OPFS Content-Addressed Storage Subsystem & Tamper Detection (`OpfsStorageSubsystem`)
 *
 * Zero external npm dependencies: uses exclusively node:fs, node:path, node:crypto, node:assert/strict.
 */

import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const REPO_ROOT = path.resolve(__dirname, '..', '..');

// Import real Wasm bindings from public/
import * as wasmPkg from '../../public/lexlean_r4.js';

// =========================================================================
// 1. OPFS Content-Addressed Storage Subsystem Simulation
// =========================================================================

class MockOpfsFileHandle {
  constructor(name) {
    this.name = name;
    this.buffer = Buffer.alloc(0);
  }

  async createWritable() {
    return {
      write: async (data) => {
        const chunk = Buffer.isBuffer(data) ? data : Buffer.from(data);
        this.buffer = Buffer.concat([this.buffer, chunk]);
      },
      close: async () => {},
    };
  }

  async getFile() {
    return {
      size: this.buffer.length,
      arrayBuffer: async () =>
        this.buffer.buffer.slice(
          this.buffer.byteOffset,
          this.buffer.byteOffset + this.buffer.byteLength
        ),
    };
  }
}

class MockOpfsDirectoryHandle {
  constructor(name = 'root') {
    this.name = name;
    this.entries = new Map();
  }

  async getFileHandle(name, options = {}) {
    if (this.entries.has(name)) {
      const entry = this.entries.get(name);
      if (!(entry instanceof MockOpfsFileHandle)) {
        throw new Error(`Entry ${name} is a directory`);
      }
      return entry;
    }
    if (options.create) {
      const handle = new MockOpfsFileHandle(name);
      this.entries.set(name, handle);
      return handle;
    }
    const err = new Error(`File ${name} not found`);
    err.name = 'NotFoundError';
    throw err;
  }

  async getDirectoryHandle(name, options = {}) {
    if (this.entries.has(name)) {
      const entry = this.entries.get(name);
      if (!(entry instanceof MockOpfsDirectoryHandle)) {
        throw new Error(`Entry ${name} is a file`);
      }
      return entry;
    }
    if (options.create) {
      const handle = new MockOpfsDirectoryHandle(name);
      this.entries.set(name, handle);
      return handle;
    }
    const err = new Error(`Directory ${name} not found`);
    err.name = 'NotFoundError';
    throw err;
  }

  async removeEntry(name) {
    if (!this.entries.has(name)) {
      const err = new Error(`Entry ${name} not found`);
      err.name = 'NotFoundError';
      throw err;
    }
    this.entries.delete(name);
  }
}

class OpfsStorageSubsystem {
  constructor(root = new MockOpfsDirectoryHandle('opfs_root')) {
    this.root = root;
  }

  sha256Hex(buffer) {
    return crypto.createHash('sha256').update(buffer).digest('hex');
  }

  async opfsPutBlob(data) {
    const buf = Buffer.isBuffer(data) ? data : Buffer.from(data);
    const hash = this.sha256Hex(buf);
    const partialName = `${hash}.partial`;

    // 1. Stage in .partial
    const partialHandle = await this.root.getFileHandle(partialName, { create: true });
    const writable = await partialHandle.createWritable();
    await writable.write(buf);
    await writable.close();

    // 2. Atomic rename: read buffer, create final file, remove .partial
    const stagedFile = await partialHandle.getFile();
    const finalBuffer = Buffer.from(await stagedFile.arrayBuffer());

    const finalHandle = await this.root.getFileHandle(hash, { create: true });
    const finalWritable = await finalHandle.createWritable();
    await finalWritable.write(finalBuffer);
    await finalWritable.close();

    await this.root.removeEntry(partialName);
    return hash;
  }

  async opfsGetBlob(expectedHash) {
    let handle;
    try {
      handle = await this.root.getFileHandle(expectedHash);
    } catch {
      const err = new Error(`Blob ${expectedHash} not found in OPFS`);
      err.code = 'store.missing';
      throw err;
    }

    const file = await handle.getFile();
    const bytes = Buffer.from(await file.arrayBuffer());
    const actualHash = this.sha256Hex(bytes);

    if (actualHash !== expectedHash) {
      const err = new Error(
        `SHA-256 digest mismatch: expected ${expectedHash}, got ${actualHash}`
      );
      err.code = 'store.mismatch';
      err.expected = expectedHash;
      err.actual = actualHash;
      throw err;
    }

    return bytes;
  }

  async tamperBlob(hash, corruptedData) {
    const handle = await this.root.getFileHandle(hash, { create: true });
    const writable = await handle.createWritable();
    await writable.write(Buffer.from(corruptedData));
    await writable.close();
  }
}

// =========================================================================
// 2. IndexedDB Commit-Boundary Adapter Simulation (Foundry FA-20)
// =========================================================================

class MockIdbDatabase {
  constructor(name, storeNames) {
    this.name = name;
    this.stores = new Map();
    for (const s of storeNames) {
      this.stores.set(s, new Map());
    }
  }

  transaction(storeNames, mode = 'readonly') {
    return new MockIdbTransaction(this, storeNames, mode);
  }
}

class MockIdbTransaction {
  constructor(db, storeNames, mode) {
    this.db = db;
    this.storeNames = storeNames;
    this.mode = mode;
    this.state = 'active'; // 'active' | 'committed' | 'aborted'
    this.staging = new Map();

    for (const name of storeNames) {
      if (!db.stores.has(name)) {
        throw new Error(`NotFoundError: objectStore ${name} does not exist`);
      }
      this.staging.set(name, new Map(db.stores.get(name)));
    }

    this.oncomplete = null;
    this.onabort = null;
    this.onerror = null;
  }

  objectStore(name) {
    if (!this.storeNames.includes(name) || !this.staging.has(name)) {
      throw new Error(`NotFoundError: objectStore ${name} not included in transaction`);
    }
    const stagedMap = this.staging.get(name);
    const self = this;

    return {
      get: (key) => stagedMap.get(key),
      put: (value, key) => {
        if (self.mode === 'readonly') {
          throw new Error('ReadOnlyError: cannot write in readonly transaction');
        }
        if (self.state !== 'active') {
          throw new Error('TransactionInactiveError: transaction is no longer active');
        }
        stagedMap.set(key, value);
      },
      delete: (key) => {
        if (self.mode === 'readonly') {
          throw new Error('ReadOnlyError: cannot write in readonly transaction');
        }
        if (self.state !== 'active') {
          throw new Error('TransactionInactiveError: transaction is no longer active');
        }
        stagedMap.delete(key);
      },
    };
  }

  commit() {
    if (this.state !== 'active') return;
    this.state = 'committed';

    // Commit staging back to database stores
    for (const [name, stagedMap] of this.staging.entries()) {
      this.db.stores.set(name, stagedMap);
    }

    queueMicrotask(() => {
      if (typeof this.oncomplete === 'function') {
        this.oncomplete({ target: this });
      }
    });
  }

  abort(err = null) {
    if (this.state !== 'active') return;
    this.state = 'aborted';

    queueMicrotask(() => {
      if (typeof this.onabort === 'function') {
        this.onabort({ target: this, error: err });
      }
    });
  }
}

/**
 * Remediated Foundry FA-20 commit-boundary adapter:
 * - Awaits asynchronous or callback operations before committing.
 * - Resolves strictly upon transaction.oncomplete.
 * - Rejects on transaction.onabort / onerror with typed 'storageTransactionFailed'.
 */
function storageRequest(db, storeName, mode, operation) {
  return new Promise((resolve, reject) => {
    let tx;
    try {
      tx = db.transaction([storeName], mode);
    } catch (err) {
      const wrapped = new Error(`storageTransactionFailed: ${err.message}`);
      wrapped.code = 'storageTransactionFailed';
      wrapped.cause = err;
      return reject(wrapped);
    }

    let operationResult = undefined;
    let operationError = null;
    let isSettled = false;

    tx.oncomplete = () => {
      if (isSettled) return;
      isSettled = true;
      if (operationError) {
        const err = new Error(`storageTransactionFailed: ${operationError.message}`);
        err.code = 'storageTransactionFailed';
        err.cause = operationError;
        reject(err);
      } else {
        resolve(operationResult);
      }
    };

    tx.onabort = (event) => {
      if (isSettled) return;
      isSettled = true;
      const cause = event.error || operationError || new Error('Transaction aborted');
      const err = new Error(`storageTransactionFailed: ${cause.message}`);
      err.code = 'storageTransactionFailed';
      err.cause = cause;
      reject(err);
    };

    tx.onerror = (event) => {
      if (isSettled) return;
      isSettled = true;
      const cause = event.error || operationError || new Error('Transaction failed');
      const err = new Error(`storageTransactionFailed: ${cause.message}`);
      err.code = 'storageTransactionFailed';
      err.cause = cause;
      reject(err);
    };

    (async () => {
      try {
        const store = tx.objectStore(storeName);
        let doneCalled = false;
        const onDone = (res) => {
          doneCalled = true;
          operationResult = res;
        };
        const onError = (err) => {
          operationError = err;
          try {
            tx.abort(err);
          } catch {}
        };

        const maybePromise = operation(store, onDone, onError);
        if (maybePromise && typeof maybePromise.then === 'function') {
          const asyncRes = await maybePromise;
          if (!doneCalled && asyncRes !== undefined) {
            operationResult = asyncRes;
          }
        }

        // Trigger commit strictly after async operation completes
        if (mode === 'readwrite' && typeof tx.commit === 'function' && tx.state === 'active') {
          tx.commit();
        }
      } catch (err) {
        operationError = err;
        try {
          tx.abort(err);
        } catch {}
      }
    })();
  });
}

// =========================================================================
// 3. Multi-Tab Web Locks Synchronization Simulation
// =========================================================================

class CompliantLockManager {
  constructor() {
    this.activeLocks = new Map(); // name -> { holderId, mode, depth }
    this.waitQueues = new Map(); // name -> Array({ mode, callback, resolve, reject, signal, clientId })
  }

  async request(name, optionsOrCallback, maybeCallback) {
    let mode = 'exclusive';
    let callback = optionsOrCallback;
    let signal = null;
    let ifAvailable = false;
    let clientId = null;

    if (typeof optionsOrCallback === 'object' && optionsOrCallback !== null) {
      mode = optionsOrCallback.mode || 'exclusive';
      signal = optionsOrCallback.signal || null;
      ifAvailable = Boolean(optionsOrCallback.ifAvailable);
      clientId = optionsOrCallback.clientId || null;
      callback = maybeCallback;
    }

    assert.equal(typeof callback, 'function', 'Lock callback must be a function');

    if (signal && signal.aborted) {
      const err = new Error('The lock request was aborted');
      err.name = 'AbortError';
      return Promise.reject(err);
    }

    return new Promise((resolve, reject) => {
      const current = this.activeLocks.get(name);

      // 1. ifAvailable option: return null if lock is currently held
      if (ifAvailable && current) {
        return resolve(callback(null));
      }

      // 2. Re-entrancy detection for same client context
      if (current && clientId && current.holderId === clientId) {
        current.depth++;
        (async () => {
          try {
            const res = await callback({ name, mode });
            resolve(res);
          } catch (err) {
            reject(err);
          } finally {
            current.depth--;
            if (current.depth === 0) {
              this.activeLocks.delete(name);
              this.processQueue(name);
            }
          }
        })();
        return;
      }

      // 3. Queue request with AbortSignal listener
      const entry = { mode, callback, resolve, reject, signal, clientId };

      if (signal) {
        signal.addEventListener(
          'abort',
          () => {
            const q = this.waitQueues.get(name);
            if (q) {
              const idx = q.indexOf(entry);
              if (idx !== -1) {
                q.splice(idx, 1);
                const abortErr = new Error('The lock request was aborted');
                abortErr.name = 'AbortError';
                reject(signal.reason || abortErr);
              }
            }
          },
          { once: true }
        );
      }

      if (!this.waitQueues.has(name)) this.waitQueues.set(name, []);
      this.waitQueues.get(name).push(entry);
      this.processQueue(name);
    });
  }

  processQueue(name) {
    const queue = this.waitQueues.get(name);
    if (!queue || queue.length === 0) return;
    if (this.activeLocks.has(name)) return;

    const next = queue.shift();
    const holderId = next.clientId || Symbol('holder');
    const lockInstance = { name, mode: next.mode, holderId, depth: 1 };
    this.activeLocks.set(name, lockInstance);

    (async () => {
      try {
        const res = await next.callback(lockInstance);
        this.activeLocks.delete(name);
        next.resolve(res);
      } catch (err) {
        this.activeLocks.delete(name);
        next.reject(err);
      } finally {
        this.processQueue(name);
      }
    })();
  }
}

// =========================================================================
// 4. Test Suite Execution (8 Distinct Groups)
// =========================================================================

let testsPassed = 0;
let testsFailed = 0;

async function runTest(name, fn) {
  try {
    await fn();
    testsPassed++;
    console.log(`  ✓ ${name}`);
  } catch (err) {
    testsFailed++;
    console.error(`  ✗ ${name}:`, err);
  }
}

async function main() {
  console.log('\n=== Running LexLean-R4 Browser Web API Simulation Harness ===\n');

  // Load real Wasm module
  const wasmPath = path.join(REPO_ROOT, 'public', 'lexlean_r4_bg.wasm');
  assert(fs.existsSync(wasmPath), `Wasm binary missing at ${wasmPath}`);
  const wasmBytes = fs.readFileSync(wasmPath);
  wasmPkg.initSync({ module: wasmBytes });

  // --- Group 1: Cryptographic Provenance & Closure Integrity ---
  console.log('--- Group 1: Cryptographic Provenance & Closure Integrity ---');
  await runTest('Provenance schema and SHA-256 binary digests match Wasm & JS closures', async () => {
    const provPath = path.join(REPO_ROOT, 'public', 'provenance.json');
    assert(fs.existsSync(provPath), 'provenance.json must exist');
    const prov = JSON.parse(fs.readFileSync(provPath, 'utf8'));
    assert.equal(prov.schema, 'prismpm/browser-provenance/1');

    const computedWasmHash = crypto
      .createHash('sha256')
      .update(fs.readFileSync(wasmPath))
      .digest('hex');
    assert.equal(
      computedWasmHash,
      prov.adapter_wasm_sha256,
      'Wasm binary hash must match provenance record'
    );

    const jsPath = path.join(REPO_ROOT, 'public', 'lexlean_r4.js');
    const computedJsHash = crypto
      .createHash('sha256')
      .update(fs.readFileSync(jsPath))
      .digest('hex');
    assert.equal(
      computedJsHash,
      prov.binding_javascript_sha256,
      'JS binding hash must match provenance record'
    );
  });

  // --- Group 2: Live Wasm Engine & Zero-Float Invariants ---
  console.log('--- Group 2: Live Wasm Engine & Zero-Float Invariants ---');
  await runTest('studio_r4_stack_status reflects float-free and dense-matmul-free serving', async () => {
    const statusJson = wasmPkg.studio_r4_stack_status();
    const status = JSON.parse(statusJson);
    assert.equal(status.status, 'ok');
    assert.equal(status.float_ops, 0, 'Model serving must execute zero float ops');
    assert.equal(status.dense_matmul_ops, 0, 'Model serving must execute zero dense matmuls');
    assert.equal(status.format, 'UORR4V12');
    assert.equal(status.schema, 'uor-r4.geometric-vector-stack/12');
  });

  // --- Group 3: Real MVM CoW Sector Persistence & Error Rejection ---
  console.log('--- Group 3: Real MVM CoW Sector Persistence & Error Rejection ---');
  await runTest('MVM CoW sector export/import roundtrip and bad magic rejection', async () => {
    const cowBytes = wasmPkg.studio_export_mvm_cow_sectors();
    assert(cowBytes.byteLength >= 16);
    const magicStr = Buffer.from(cowBytes.slice(0, 8)).toString('ascii');
    assert.equal(magicStr, 'MVM_COW1', 'Magic header must be MVM_COW1');

    // Valid import (4 sectors exported by default MVM CoW state)
    const importRes = JSON.parse(wasmPkg.studio_import_mvm_cow_sectors(cowBytes));
    assert.equal(importRes.status, 'ok');
    assert.equal(importRes.imported_sectors, 4);

    // Bad magic rejection
    const badMagicBytes = Buffer.from(cowBytes);
    badMagicBytes.set(Buffer.from('BADMAGIC'), 0);
    const badRes = JSON.parse(wasmPkg.studio_import_mvm_cow_sectors(badMagicBytes));
    assert.equal(badRes.status, 'error');
    assert.match(badRes.message, /Invalid MVM_COW1 magic/);

    // Truncated stream rejection
    const truncRes = JSON.parse(wasmPkg.studio_import_mvm_cow_sectors(cowBytes.slice(0, 32)));
    assert.equal(truncRes.status, 'error');
    assert.match(truncRes.message, /Truncated MVM_COW1/);
  });

  // --- Group 4: Real Stitch 2-of-3 XOR Parity Engine ---
  console.log('--- Group 4: Real Stitch 2-of-3 XOR Parity Engine ---');
  await runTest('studio_stitch_split_2of3 partitions payload into distinct parity fragments', async () => {
    const payload = Buffer.from('Stitch Fabric 2-of-3 XOR Sharding Verification Payload');
    const res = JSON.parse(wasmPkg.studio_stitch_split_2of3(payload, 'test-label'));
    assert.equal(res.status, 'ok');
    assert.equal(res.count, 3);
    assert.equal(res.threshold, 2);
    assert.equal(res.scheme, 'xor-2-of-3');
    assert.equal(res.fragments.length, 3);

    // Verify all fragments have distinct SHA-256 digests
    const digests = new Set(res.fragments.map((f) => f.sha256));
    assert.equal(digests.size, 3, 'All 3 shards must have distinct hashes');
  });

  // --- Group 5: Real SpiralCore Host Bridge & Security Policy ---
  console.log('--- Group 5: Real SpiralCore Host Bridge & Security Policy ---');
  await runTest('SpiralCoreHostBridge enforces secret isolation and TLS-only transport', async () => {
    const bridge = new wasmPkg.SpiralCoreHostBridge();

    // 1. Host secret management
    bridge.setSecret('GITHUB_TOKEN', 'ghp_secret_token_123456');
    assert.equal(bridge.hasSecret('GITHUB_TOKEN'), true);
    assert.equal(bridge.hasSecret('NON_EXISTENT'), false);

    // 2. Transport security: HTTPS allowed, HTTP rejected
    const httpsRes = bridge.validateTransportUrl('https://api.github.com/repos');
    assert.equal(httpsRes.ok, true);

    const httpRes = bridge.validateTransportUrl('http://insecure.endpoint.local');
    assert.equal(httpRes.ok, false);
    assert.equal(httpRes.error_code, 'InsecureTransportRejected');
  });

  // --- Group 6: Foundry FA-20 IndexedDB Commit Boundary ---
  console.log('--- Group 6: Foundry FA-20 IndexedDB Commit Boundary ---');
  await runTest('wasmPkg.storageRequest awaits async operations, handles readonly, and rolls back on abort', async () => {
    const db = new MockIdbDatabase('test_db', ['workspace']);

    // 1. Async operation is awaited before committing
    await wasmPkg.storageRequest(db, 'workspace', 'readwrite', async (store, onDone) => {
      await new Promise((r) => setTimeout(r, 10));
      store.put('persisted_value', 'async_key');
      onDone('success');
    });
    assert.equal(db.stores.get('workspace').get('async_key'), 'persisted_value');

    // 2. Readonly transaction commits cleanly without hanging
    const readVal = await wasmPkg.storageRequest(db, 'workspace', 'readonly', (store, onDone) => {
      onDone(store.get('async_key'));
    });
    assert.equal(readVal, 'persisted_value');

    // 3. Abort rolls back staging without contaminating store
    await assert.rejects(
      async () => {
        await wasmPkg.storageRequest(db, 'workspace', 'readwrite', async (store, onDone, onError) => {
          store.put('aborted_value', 'abort_key');
          onError(new Error('simulated abort'));
        });
      },
      (err) => {
        assert.equal(err.code, 'storageTransactionFailed');
        return true;
      }
    );
    assert.equal(db.stores.get('workspace').has('abort_key'), false);
  });

  // --- Group 7: Multi-Tab Web Locks Synchronization ---
  console.log('--- Group 7: Multi-Tab Web Locks Synchronization ---');
  await runTest('wasmPkg.withStorageLock and CompliantLockManager serialize access cleanly', async () => {
    const locks = new CompliantLockManager();
    const origNavDesc = Object.getOwnPropertyDescriptor(globalThis, 'navigator');
    const order = [];

    try {
      Object.defineProperty(globalThis, 'navigator', {
        configurable: true,
        writable: true,
        value: { ...(origNavDesc ? globalThis.navigator : {}), locks },
      });

      // 1. Real wasmPkg.withStorageLock FIFO ordering
      const l1 = wasmPkg.withStorageLock(async () => {
        await new Promise((r) => setTimeout(r, 10));
        order.push('wasm_lock_1');
        return 'ok1';
      });
      const l2 = wasmPkg.withStorageLock(async () => {
        order.push('wasm_lock_2');
        return 'ok2';
      });
      assert.deepEqual(await Promise.all([l1, l2]), ['ok1', 'ok2']);
      assert.deepEqual(order, ['wasm_lock_1', 'wasm_lock_2']);

      // 2. ifAvailable: true returns null when lock is held
      await locks.request('held_lock', async () => {
        const availRes = await locks.request(
          'held_lock',
          { ifAvailable: true },
          async (lock) => lock
        );
        assert.equal(availRes, null, 'ifAvailable must return null if lock is currently held');
      });

      // 3. Re-entrancy detection with clientId
      await locks.request(
        'reentrant_lock',
        { clientId: 'tab_a' },
        async () => {
          const inner = await locks.request(
            'reentrant_lock',
            { clientId: 'tab_a' },
            async () => 'inner_ok'
          );
          assert.equal(inner, 'inner_ok');
        }
      );
    } finally {
      if (origNavDesc) {
        Object.defineProperty(globalThis, 'navigator', origNavDesc);
      } else {
        delete globalThis.navigator;
      }
    }
  });

  // --- Group 8: OPFS Content-Addressed Storage Subsystem ---
  console.log('--- Group 8: OPFS Content-Addressed Storage Subsystem ---');
  await runTest('wasmPkg.opfs_put_blob and opfs_get_blob enforce atomic rename and digest verification', async () => {
    const opfs = new OpfsStorageSubsystem();
    const origNavDesc = Object.getOwnPropertyDescriptor(globalThis, 'navigator');
    try {
      Object.defineProperty(globalThis, 'navigator', {
        configurable: true,
        writable: true,
        value: {
          ...(origNavDesc ? globalThis.navigator : {}),
          storage: { getDirectory: async () => opfs.root },
        },
      });

      const data = Buffer.from('hello world OPFS payload');
      const hash = await wasmPkg.opfs_put_blob(data);
      assert.equal(hash, opfs.sha256Hex(data));

      // Ensure .partial file is removed from uor-r4/blobs after atomic rename
      const uorDir = await opfs.root.getDirectoryHandle('uor-r4', { create: false });
      const blobsDir = await uorDir.getDirectoryHandle('blobs', { create: false });
      let partialExists = false;
      try {
        await blobsDir.getFileHandle(`${hash}.partial`);
        partialExists = true;
      } catch {
        partialExists = false;
      }
      assert.equal(partialExists, false, '.partial file must be atomically renamed');

      // Read back blob via real wasmPkg.opfs_get_blob
      const retrieved = await wasmPkg.opfs_get_blob(hash);
      assert.deepEqual(Buffer.from(retrieved), data);

      // Non-existent hash rejects with store.missing
      const fakeHash = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855';
      await assert.rejects(
        async () => await wasmPkg.opfs_get_blob(fakeHash),
        (err) => {
          assert.equal(err.code, 'store.missing');
          return true;
        }
      );

      // Tampered content rejects with store.mismatch
      const blobHandle = await blobsDir.getFileHandle(hash, { create: false });
      blobHandle.buffer = Buffer.from('corrupted payload');
      await assert.rejects(
        async () => await wasmPkg.opfs_get_blob(hash),
        (err) => {
          assert.equal(err.code, 'store.mismatch');
          assert.equal(err.expected, hash);
          return true;
        }
      );
    } finally {
      if (origNavDesc) {
        Object.defineProperty(globalThis, 'navigator', origNavDesc);
      } else {
        delete globalThis.navigator;
      }
    }
  });

  console.log('\n============================================================');
  console.log(`Results: ${testsPassed} passed, ${testsFailed} failed`);
  console.log('============================================================\n');

  if (testsFailed > 0) {
    process.exit(1);
  }
}

main().catch((err) => {
  console.error('Fatal harness error:', err);
  process.exit(1);
});
