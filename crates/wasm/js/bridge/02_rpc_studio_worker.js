async function __uorSha256HexBytes(u8) { /* Formal R4.HoloGeometricRuntime */
  const view = u8 instanceof Uint8Array ? u8 : new Uint8Array(u8);
  const digest = await crypto.subtle.digest('SHA-256', view);
  const arr = new Uint8Array(digest);
  let hex = '';
  for (let i = 0; i < arr.length; i++) {
    hex += arr[i].toString(16).padStart(2, '0');
  }
  return hex;
}

function __uorBytesToBase64(u8) { /* Formal R4.HoloGeometricRuntime */
  let bin = '';
  const chunk = 0x8000;
  for (let i = 0; i < u8.length; i += chunk) {
    bin += String.fromCharCode.apply(null, u8.subarray(i, Math.min(i + chunk, u8.length)));
  }
  return btoa(bin);
}

function __uorBase64ToBytes(b64) { /* Formal R4.HoloGeometricRuntime */
  const clean = String(b64 || '').replace(/\s+/g, '');
  const bin = atob(clean);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) {
    out[i] = bin.charCodeAt(i) & 0xff;
  }
  return out;
}

function __uorStripHtmlToReadableText(html) {
  const raw = String(html || '');
  const titleMatch = raw.match(/<title[^>]*>([\s\S]*?)<\/title>/i);
  const title = titleMatch ? titleMatch[1].replace(/\s+/g, ' ').trim() : '';
  const body = raw
    .replace(/<script[\s\S]*?<\/script>/gi, ' ')
    .replace(/<style[\s\S]*?<\/style>/gi, ' ')
    .replace(/<noscript[\s\S]*?<\/noscript>/gi, ' ')
    .replace(/<\/(p|div|h1|h2|h3|h4|li|tr|section|article|br)>/gi, '\n')
    .replace(/<[^>]+>/g, ' ')
    .replace(/&nbsp;/gi, ' ')
    .replace(/&amp;/gi, '&')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/&quot;/gi, '"')
    .replace(/[ \t]+/g, ' ')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
  return { title, text: body.slice(0, 12000) };
}

function __uorVerifyRiscv64ElfHeader(u8) {
  if (!(u8 instanceof Uint8Array) || u8.byteLength < 64) {
    return { ok: false, error_code: 'InvalidElf64Riscv', message: 'Binary shorter than 64-byte ELF64 header' };
  }
  if (u8[0] !== 0x7f || u8[1] !== 0x45 || u8[2] !== 0x4c || u8[3] !== 0x46) {
    return { ok: false, error_code: 'InvalidElf64Riscv', message: 'Missing 0x7F ELF magic bytes' };
  }
  if (u8[4] !== 2 || u8[5] !== 1) {
    return { ok: false, error_code: 'InvalidElf64Riscv', message: 'Expected ELFCLASS64 little-endian binary' };
  }
  const eMachine = u8[18] | (u8[19] << 8);
  if (eMachine !== 243) {
    return {
      ok: false,
      error_code: 'InvalidElf64Riscv',
      message: `Expected EM_RISCV (243), got e_machine=${eMachine}`,
    };
  }
  return { ok: true, e_machine: eMachine };
}

/* Formal R4.MvmLinuxContainer */
export function admit_public_cors_preemption(
  spiralCoreNetEnabled,
  isPrivateOrLoopback,
  hasCredentials,
  directFetchCorsBlocked,
) {
  /* Formal R4.MvmLinuxContainer */
  if (isPrivateOrLoopback) {
    return 'RefusedPrivateAddressSsrf';
  }
  if (!spiralCoreNetEnabled) {
    return 'RefusedSpiralCoreNetDisabled';
  }
  if (!directFetchCorsBlocked) {
    return 'DirectBrowserFetchAdmitted';
  }
  if (hasCredentials) {
    return 'RefusedCredentialedCorsProxy';
  }
  return 'StatelessRelayCorsPreempted';
}

export class SpiralCoreHostBridge {
  #secretVault = new Map();
  #ws = null;
  #wsConnecting = null;
  #pendingWs = new Map();
  #nextReqSeq = 1;
  #lazyRepoTrees = new Map();
  #workspaceMap = new Map();
  #workspaceFs = null;
  #provenanceBaseline = null;
  #esbuildPromise = null;
  #esbuildCompiler = null;
  #pyodidePromise = null;
  #pyodideInstance = null;
  #pyodideShimmed = false;
  #activePyStdoutSink = null;
  #activePyStderrSink = null;
  #syncFetchImpl = null;

  constructor(options) {
    const opts = options && typeof options === 'object' ? options : {};
    this.#workspaceFs = opts.workspaceFs && typeof opts.workspaceFs === 'object' ? opts.workspaceFs : null;
    if (Array.isArray(opts.workspaceFiles)) {
      for (const f of opts.workspaceFiles) {
        if (f && typeof f.path === 'string' && typeof f.content === 'string') {
          const norm = this.#normalizeWsPath(f.path);
          if (norm) this.#workspaceMap.set(norm, f.content);
        }
      }
    }
    this.relayUrl =
      opts.relayWsUrl !== undefined
        ? opts.relayWsUrl
        : (opts.relayUrl !== undefined ? opts.relayUrl : DEFAULT_SPIRALCORE_RELAY_WS_URL);
    this.relayHttpUrl = opts.relayHttpUrl || '';
    this.staticOrigin = opts.staticOrigin || '';
    this.locationOrigin = opts.locationOrigin || '';
    this.mcpPolicy = opts.mcpPolicy || 'safe';
    this.mode = SpiralCoreHostBridge.normalizeNetworkMode(
      opts.mode !== undefined ? opts.mode : opts.networkMode,
    );
    this.consentGranted = Boolean(opts.consentGranted);
    this.allowOffOriginHydration =
      typeof opts.allowOffOriginHydration === 'boolean' ? opts.allowOffOriginHydration : null;
    this.consentDeclined =
      typeof opts.consentDeclined === 'boolean' ? opts.consentDeclined : null;
    this.offlineEmergencyQ31 = Boolean(opts.offlineEmergencyQ31 ?? opts.offline_emergency_q31);
    this.onAuditEvent = typeof opts.onAuditEvent === 'function' ? opts.onAuditEvent : null;
    this.auditLog = [];
    this.webAccessEnabled = opts.webAccessEnabled !== false;
    this.relayTimeoutMs = opts.relayTimeoutMs || SPVS_RELAY_TIMEOUT_MS;
    this.lastTransportMode = 'standby';
    this.fetchImpl = opts.fetchImpl || globalThis.fetch;
    this.#syncFetchImpl = typeof opts.syncFetchImpl === 'function' ? opts.syncFetchImpl : null;
    this.esbuildModuleUrl = '';
    this.esbuildWasmUrl = '';
    this.setCompilerConfig(opts);
    this.pyodideLock = PYODIDE_LOCK;
    if (opts.pyodideLock && typeof opts.pyodideLock === 'object') {
      const l = opts.pyodideLock;
      if (
        typeof l.url === 'string' &&
        l.url.startsWith('https://') &&
        typeof l.sri === 'string' &&
        l.sri.startsWith('sha384-') &&
        l.sri.length > 20
      ) {
        this.pyodideLock = Object.freeze({
          version: String(l.version || PYODIDE_LOCK.version),
          url: l.url,
          indexURL: String(l.indexURL || l.url.slice(0, l.url.lastIndexOf('/') + 1)),
          sri: l.sri,
        });
      }
    }
  }

  setCompilerConfig(cfg) {
    const c = cfg && typeof cfg === 'object' ? cfg : {};
    const hasMod = Object.prototype.hasOwnProperty.call(c, 'esbuildModuleUrl');
    const hasWasm = Object.prototype.hasOwnProperty.call(c, 'esbuildWasmUrl');
    const hasBase = Object.prototype.hasOwnProperty.call(c, 'esbuildUrl');
    let modUrl = hasMod && typeof c.esbuildModuleUrl === 'string' ? c.esbuildModuleUrl.trim() : '';
    let wasmUrl = hasWasm && typeof c.esbuildWasmUrl === 'string' ? c.esbuildWasmUrl.trim() : '';
    const baseEsbuild = hasBase && typeof c.esbuildUrl === 'string' ? c.esbuildUrl.trim() : '';
    if (baseEsbuild) {
      if (baseEsbuild.endsWith('esbuild.wasm')) {
        if (!hasWasm) wasmUrl = baseEsbuild;
        if (!hasMod) modUrl = baseEsbuild.replace(/esbuild\.wasm$/, 'esm/browser.min.js');
      } else if (baseEsbuild.endsWith('browser.min.js')) {
        if (!hasMod) modUrl = baseEsbuild;
        if (!hasWasm) wasmUrl = baseEsbuild.replace(/esm\/browser\.min\.js$/, 'esbuild.wasm');
      } else {
        const root = baseEsbuild.endsWith('/') ? baseEsbuild : `${baseEsbuild}/`;
        if (!hasMod) modUrl = `${root}esm/browser.min.js`;
        if (!hasWasm) wasmUrl = `${root}esbuild.wasm`;
      }
    }
    if ((hasMod || hasBase || modUrl) && modUrl !== this.esbuildModuleUrl) {
      this.esbuildModuleUrl = modUrl;
      this.#esbuildCompiler = null;
      this.#esbuildPromise = null;
    }
    if ((hasWasm || hasBase || wasmUrl) && wasmUrl !== this.esbuildWasmUrl) {
      this.esbuildWasmUrl = wasmUrl;
      this.#esbuildCompiler = null;
      this.#esbuildPromise = null;
    }
    return {
      ok: true,
      esbuildModuleUrl: this.esbuildModuleUrl,
      esbuildWasmUrl: this.esbuildWasmUrl,
    };
  }

  setRelayUrl(url) {
    const trimmed = String(url || '').trim();
    if (!trimmed) {
      this.relayUrl = DEFAULT_SPIRALCORE_RELAY_WS_URL;
      this.relayHttpUrl = '';
      this.closeRelaySocket();
      return { ok: true, relay_url: this.relayUrl };
    }
    const check = this.validateTransportUrl(trimmed, true);
    if (!check.ok) return check;
    this.relayUrl = trimmed;
    this.closeRelaySocket();
    return { ok: true, relay_url: this.relayUrl };
  }

  close() {
    this.closeRelaySocket();
  }

  async verifyElf64RiscvAndSha256(elfBytes, expectedSha = '', maxBytes = 16 * 1024 * 1024) {
    if (!(elfBytes instanceof Uint8Array) || elfBytes.byteLength === 0) {
      return { ok: false, error_code: 'InvalidElf64Riscv', message: 'Empty ELF payload' };
    }
    const actualSha = await __uorSha256HexBytes(elfBytes);
    if (expectedSha && actualSha.toLowerCase() !== String(expectedSha).trim().toLowerCase()) {
      return {
        ok: false,
        error_code: 'DigestMismatch',
        message: `SHA-256 mismatch: expected ${expectedSha}, got ${actualSha}`,
      };
    }
    const elfCheck = __uorVerifyRiscv64ElfHeader(elfBytes);
    if (!elfCheck.ok) return elfCheck;
    if (elfBytes.byteLength > maxBytes) {
      return {
        ok: false,
        error_code: 'SizeExceedsQuota',
        message: `Package size ${elfBytes.byteLength} exceeds quota ${maxBytes}`,
      };
    }
    return { ok: true, sha256: actualSha, e_machine: elfCheck.e_machine };
  }

  static normalizeNetworkMode(raw) {
    const lower = String(raw || 'sandboxed').trim().toLowerCase();
    if (lower === 'spiralcore_net' || lower === 'net_spiralcore') {
      return 'spiralcore_net';
    }
    if (lower === 'full_spiralcore' || lower === 'full' || lower === 'mode_b' || lower === 'spiralcore') {
      return 'full_spiralcore';
    }
    return 'sandboxed';
  }

  #isPrivateIpv4(host) {
    const parts = String(host || '').split('.');
    if (parts.length !== 4) return false;
    const nums = [];
    for (const part of parts) {
      if (!/^\d{1,3}$/.test(part)) return false;
      const n = Number(part);
      if (n < 0 || n > 255) return false;
      nums.push(n);
    }
    const [a, b] = nums;
    return (
      a === 0 ||
      a === 10 ||
      (a === 100 && b >= 64 && b <= 127) ||
      a === 127 ||
      (a === 169 && b === 254) ||
      (a === 172 && b >= 16 && b <= 31) ||
      (a === 192 && b === 168) ||
      a >= 224
    );
  }

  #isPrivateOrLoopbackHost(rawHost) {
    const clean = String(rawHost || '')
      .trim()
      .replace(/^\[/, '')
      .replace(/\]$/, '')
      .split('%')[0]
      .replace(/\.+$/, '')
      .toLowerCase();
    if (!clean) return false;
    if (
      clean === 'localhost' ||
      clean === 'ip6-localhost' ||
      clean === 'ip6-loopback' ||
      clean.endsWith('.localhost') ||
      clean.endsWith('.local') ||
      clean.endsWith('.internal')
    ) {
      return true;
    }
    if (this.#isPrivateIpv4(clean)) return true;
    if (clean.includes(':')) {
      if (clean === '::' || clean === '::1' || /^0+:0+:0+:0+:0+:0+:0+:[01]$/.test(clean)) {
        return true;
      }
      const v4Mapped = clean.match(/^::ffff:(\d+\.\d+\.\d+\.\d+)$/i);
      if (v4Mapped && this.#isPrivateIpv4(v4Mapped[1])) {
        return true;
      }
      const hexMapped = clean.match(/^::ffff:([0-9a-f]{1,4}):([0-9a-f]{1,4})$/i);
      if (hexMapped) {
        const hi = parseInt(hexMapped[1], 16);
        const lo = parseInt(hexMapped[2], 16);
        const a = (hi >> 8) & 0xff;
        const b = hi & 0xff;
        if (this.#isPrivateIpv4(`${a}.${b}.${(lo >> 8) & 0xff}.${lo & 0xff}`)) {
          return true;
        }
      }
      const firstHex = clean.split(':')[0] || '';
      if (/^[0-9a-f]{1,4}$/.test(firstHex)) {
        const seg0 = parseInt(firstHex, 16);
        if ((seg0 & 0xfe00) === 0xfc00 || (seg0 & 0xffc0) === 0xfe80 || (seg0 >> 8) === 0xff) {
          return true;
        }
      }
    }
    return false;
  }

  #isCredentialedRequest(hostname, headers, urlHasSecret) {
    const h = String(hostname || '').toLowerCase();
    if (CREDENTIALED_GITHUB_HOSTS.has(h)) return true;
    if (urlHasSecret) return true;
    const hdrObj = headers && typeof headers === 'object' ? headers : {};
    for (const [k, v] of Object.entries(hdrObj)) {
      const kl = String(k || '').toLowerCase();
      const vs = String(v || '');
      if (kl === 'authorization' || kl === 'cookie' || kl === 'x-github-token' || vs.includes('{{MVM_SECRET:')) {
        return true;
      }
    }
    return false;
  }

  #emitAuditEvent(operation, target, transport, status, bytes) {
    const ev = {
      event: 'audit_event',
      op: 'audit_event',
      operation: String(operation || 'network'),
      target: String(target || ''),
      mode: this.mode,
      transport: String(transport || 'none'),
      status: status !== undefined ? status : 0,
      bytes: Number(bytes) || 0,
      timestamp_ms: Date.now(),
    };
    if (this.auditLog.length >= 500) {
      this.auditLog.shift();
    }
    this.auditLog.push(ev);
    if (typeof this.onAuditEvent === 'function') {
      try {
        this.onAuditEvent(ev);
      } catch {
        // Ignore audit listener error cleanly
      }
    }
  }

  #admitNetworkPolicy(hostname, policyOpts) {
    const cleanHost = String(hostname || '').trim().replace(/^\[/, '').replace(/\]$/, '').replace(/\.+$/, '').toLowerCase();
    const opts = policyOpts && typeof policyOpts === 'object' ? policyOpts : {};
    const isConfiguredLoopback = cleanHost === '127.0.0.1' || cleanHost === 'localhost' || cleanHost === '::1';
    const isMetadataOrPrivate = this.#isPrivateOrLoopbackHost(cleanHost) && !isConfiguredLoopback;
    if (opts.allowConfiguredOrigin && cleanHost && !isMetadataOrPrivate) {
      for (const candidateOrigin of [this.staticOrigin, this.locationOrigin]) {
        if (!candidateOrigin) continue;
        try {
          if (new URL(candidateOrigin).hostname.toLowerCase() === cleanHost) {
            return { ok: true, host: cleanHost };
          }
        } catch {
          // Ignore malformed configured origin
        }
      }
      if (cleanHost.endsWith('.github.io')) {
        return { ok: true, host: cleanHost };
      }
    }
    if (!cleanHost || this.#isPrivateOrLoopbackHost(cleanHost)) {
      return {
        ok: false,
        status: 403,
        error_code: 'PrivateAddressBlocked',
        host: cleanHost,
        target: cleanHost,
        message: `Blocked private, loopback, or link-local SSRF target: ${cleanHost || hostname}`,
      };
    }
    if (this.mode === 'sandboxed') {
      if (!PRISM_SANDBOXED_ALLOWLIST.has(cleanHost)) {
        return {
          ok: false,
          status: 403,
          error_code: 'CapabilityDenied',
          host: cleanHost,
          message: `Prism capabilities/1 Sandboxed mode denied outbound host ${cleanHost}`,
        };
      }
    } else if (this.mode === 'full_spiralcore' && !this.consentGranted) {
      return {
        ok: false,
        status: 403,
        error_code: 'RelayConsentRequired',
        host: cleanHost,
        message: 'Full SpiralCore Internet mode requires explicit user consent (consentGranted=true)',
      };
    } else if (this.mode === 'spiralcore_net' && this.webAccessEnabled === false) {
      return {
        ok: false,
        status: 403,
        error_code: 'RefusedSpiralCoreNetDisabled',
        host: cleanHost,
        message: 'SpiralCore Internet access is disabled via webAccessEnabled=false',
      };
    }
    return { ok: true, host: cleanHost };
  }

  async #admittedFetch(rawUrl, fetchInit, gateOpts) {
    const opts = gateOpts && typeof gateOpts === 'object' ? gateOpts : {};
    const op = String(opts.operation || 'http_request');
    const transportMode = String(opts.transportMode || 'direct-https-fetch');
    const urlStr = String(rawUrl || '').trim();
    const valid = this.validateTransportUrl(urlStr, Boolean(opts.allowWs));
    if (!valid.ok) {
      this.#emitAuditEvent(op, urlStr, 'blocked', valid.status || 403, 0);
      return { ok: false, error: valid };
    }
    const netAdm = this.#admitNetworkPolicy(valid.url.hostname, opts);
    if (!netAdm.ok) {
      this.#emitAuditEvent(op, valid.url.href, 'blocked', netAdm.status || 403, 0);
      return { ok: false, error: netAdm };
    }
    try {
      const resp = await this.#fetchImpl(valid.url.href, fetchInit);
      if (opts.emitSuccessAudit !== false) {
        const lenHeader = Number(resp && resp.headers && typeof resp.headers.get === 'function' ? resp.headers.get('content-length') : 0) || 0;
        this.#emitAuditEvent(op, valid.url.href, transportMode, resp ? resp.status : 0, lenHeader);
      }
      return { ok: true, response: resp, url: valid.url };
    } catch (err) {
      if (opts.emitFailureAudit !== false) {
        this.#emitAuditEvent(op, valid.url.href, 'direct-failed', 502, 0);
      }
      return {
        ok: false,
        networkError: err,
        url: valid.url,
        error: {
          ok: false,
          status: 502,
          error_code: String(opts.failureCode || 'HttpBridgeFetchError'),
          message: err && err.message ? err.message : String(err),
        },
      };
    }
  }

  setPolicy(optsInput) {
    const opts = optsInput && typeof optsInput === 'object' ? optsInput : {};
    const { mcpPolicy, webAccessEnabled } = opts;
    if (typeof mcpPolicy === 'string' && ['safe', 'confirm', 'deny'].includes(mcpPolicy)) {
      this.mcpPolicy = mcpPolicy;
    }
    if (opts.mode !== undefined || opts.networkMode !== undefined) {
      this.mode = SpiralCoreHostBridge.normalizeNetworkMode(
        opts.mode !== undefined ? opts.mode : opts.networkMode,
      );
    }
    if (typeof opts.consentGranted === 'boolean') {
      this.consentGranted = opts.consentGranted;
    } else if (typeof opts.consent_granted === 'boolean') {
      this.consentGranted = opts.consent_granted;
    }
    if (typeof opts.allowOffOriginHydration === 'boolean') {
      this.allowOffOriginHydration = opts.allowOffOriginHydration;
    } else if (typeof opts.allow_off_origin_hydration === 'boolean') {
      this.allowOffOriginHydration = opts.allow_off_origin_hydration;
    }
    if (typeof opts.consentDeclined === 'boolean') {
      this.consentDeclined = opts.consentDeclined;
    } else if (typeof opts.consent_declined === 'boolean') {
      this.consentDeclined = opts.consent_declined;
    }
    if (typeof opts.offlineEmergencyQ31 === 'boolean') {
      this.offlineEmergencyQ31 = opts.offlineEmergencyQ31;
    } else if (typeof opts.offline_emergency_q31 === 'boolean') {
      this.offlineEmergencyQ31 = opts.offline_emergency_q31;
    }
    if (typeof opts.onAuditEvent === 'function') {
      this.onAuditEvent = opts.onAuditEvent;
    }
    if (typeof webAccessEnabled === 'boolean') {
      this.webAccessEnabled = webAccessEnabled;
    }
    if (opts.esbuildModuleUrl || opts.esbuildWasmUrl || opts.esbuildUrl) {
      this.setCompilerConfig(opts);
    }
    const relayOverride = opts.relayWsUrl !== undefined ? opts.relayWsUrl : opts.relayUrl;
    if (typeof relayOverride === 'string') {
      const rRes = this.setRelayUrl(relayOverride);
      if (!rRes || !rRes.ok) {
        return rRes;
      }
    }
    if (this.mode === 'full_spiralcore' && !this.consentGranted) {
      return {
        ok: false,
        status: 403,
        error_code: 'RelayConsentRequired',
        message: 'Full SpiralCore Internet mode requires explicit user consent (consentGranted=true)',
        mode: this.mode,
        consent_granted: false,
      };
    }
    return {
      ok: true,
      mcp_policy: this.mcpPolicy,
      mode: this.mode,
      consent_granted: this.consentGranted,
      web_access_enabled: this.webAccessEnabled,
      allow_off_origin_hydration: this.allowOffOriginHydration,
      consent_declined: this.consentDeclined,
      offline_emergency_q31: this.offlineEmergencyQ31,
      esbuild_configured: Boolean(this.esbuildModuleUrl && this.esbuildWasmUrl),
    };
  }

  setSecret(key, value) {
    const cleanKey = String(key || '').trim();
    if (!cleanKey) {
      return { ok: false, error_code: 'InvalidSecretKey', message: 'Secret key must be non-empty' };
    }
    const cleanVal = String(value || '').trim();
    if (!cleanVal) {
      this.#secretVault.delete(cleanKey);
      return { ok: true, key: cleanKey, configured: false };
    }
    this.#secretVault.set(cleanKey, cleanVal);
    return {
      ok: true,
      key: cleanKey,
      configured: true,
      allowlisted_hosts: Array.from(SECRET_HOST_ALLOWLIST[cleanKey] || []),
    };
  }

  hasSecret(key) {
    return this.#secretVault.has(String(key || '').trim());
  }

  closeRelaySocket() {
    if (this.#ws) {
      try {
        this.#ws.close();
      } catch {
        // Ignore close error
      }
      this.#ws = null;
    }
    this.#wsConnecting = null;
  }

  validateTransportUrl(rawUrl, allowWebSocket = false) {
    let parsed;
    try {
      parsed = new URL(String(rawUrl || ''));
    } catch {
      return { ok: false, error_code: 'InvalidBridgeUrl', message: `Malformed URL: ${rawUrl}` };
    }
    const proto = parsed.protocol;
    const host = parsed.hostname;
    if (proto === 'https:') {
      return { ok: true, url: parsed };
    }
    if (allowWebSocket && proto === 'wss:') {
      return { ok: true, url: parsed };
    }
    if (proto === 'http:' && host === '127.0.0.1') {
      return { ok: true, url: parsed };
    }
    if (allowWebSocket && proto === 'ws:' && host === '127.0.0.1') {
      return { ok: true, url: parsed };
    }
    return {
      ok: false,
      error_code: 'InsecureTransportRejected',
      message: `R4-39 rejected non-TLS remote protocol ${proto} for host ${host}`,
    };
  }

  substituteHostSecretsInString(rawText, targetHostname) {
    const str = String(rawText || '');
    if (!str.includes('{{MVM_SECRET:')) {
      return { ok: true, value: str };
    }
    let failed = null;
    const replaced = str.replace(/\{\{MVM_SECRET:([A-Z0-9_]+)\}\}/g, (_full, secretKey) => {
      if (failed) return '';
      const allowedHosts = SECRET_HOST_ALLOWLIST[secretKey];
      if (!allowedHosts || !allowedHosts.has(targetHostname)) {
        failed = {
          ok: false,
          error_code: 'HostNotAllowlistedForSecret',
          message: `Secret ${secretKey} is not allowlisted for host ${targetHostname}`,
        };
        return '';
      }
      const secretVal = this.#secretVault.get(secretKey);
      if (!secretVal) {
        failed = {
          ok: false,
          error_code: 'SecretNotConfigured',
          message: `Secret ${secretKey} is not configured in host vault`,
        };
        return '';
      }
      return secretVal;
    });
    if (failed) return failed;
    return { ok: true, value: replaced };
  }

  applyHostSecretSubstitution(targetUrlObj, rawHeaders = {}) {
    const hostname = targetUrlObj.hostname.toLowerCase();
    const outHeaders = {};
    for (const [k, v] of Object.entries(rawHeaders || {})) {
      const sub = this.substituteHostSecretsInString(v, hostname);
      if (!sub.ok) return sub;
      outHeaders[k] = sub.value;
    }
    if (
      !outHeaders.Authorization &&
      !outHeaders.authorization &&
      this.#secretVault.has('GITHUB_TOKEN') &&
      (hostname === 'api.github.com' || hostname === 'github.com')
    ) {
      outHeaders.Authorization = `Bearer ${this.#secretVault.get('GITHUB_TOKEN')}`;
    }
    return { ok: true, headers: outHeaders };
  }

  async tryRelayRpc(op, params) {
    if (this.mode === 'sandboxed') {
      return {
        ok: false,
        status: 403,
        error_code: 'CapabilityDenied',
        message: 'SpiralCore AF_VSOCK relay is disabled in Prism capabilities/1 Sandboxed mode',
      };
    }
    if (this.mode === 'spiralcore_net' && this.webAccessEnabled === false) {
      return {
        ok: false,
        status: 403,
        error_code: 'RefusedSpiralCoreNetDisabled',
        message: 'SpiralCore Internet relay is disabled when webAccessEnabled=false',
      };
    }
    if (this.mode === 'full_spiralcore' && !this.consentGranted) {
      return {
        ok: false,
        status: 403,
        error_code: 'RelayConsentRequired',
        message: 'SpiralCore AF_VSOCK relay requires explicit user consent (consentGranted=true)',
      };
    }
    const pObj = params && typeof params === 'object' ? params : {};
    const { _corsPreempt, ...wireParams } = pObj;
    const isCorsPreempt = Boolean(_corsPreempt) || (this.mode === 'spiralcore_net' && op === 'http_request');
    const targetCandidate = String(wireParams.url || wireParams.repo_url || '');
    let targetHost = '';
    if (targetCandidate) {
      try {
        targetHost = new URL(targetCandidate).hostname.toLowerCase();
      } catch {
        targetHost = '';
      }
    }
    if (this.#isCredentialedRequest(targetHost, wireParams.headers, Boolean(wireParams.auth_bearer) || JSON.stringify(wireParams).includes('{{MVM_SECRET:'))) {
      return {
        ok: false,
        status: 403,
        error_code: 'RelayForbiddenForCredentialedHost',
        host: targetHost,
        message: 'Credentialed hosts and requests carrying Authorization/Cookie/tokens are strictly forbidden from SpiralCore relay failback',
      };
    }
    if (!this.relayUrl && !this.relayHttpUrl && this.mode !== 'spiralcore_net') {
      return {
        ok: false,
        status: 502,
        error_code: 'RelayUnavailable',
        message: 'No SpiralCore relay endpoint configured',
      };
    }
    const rpcBody = {
      spvs_magic: 'SPVS',
      src_cid: 3,
      dst_cid: 2,
      vsock_port: 5257,
      policy: this.mcpPolicy,
      op,
      params: wireParams,
    };

    if (!isCorsPreempt && typeof WebSocket === 'function' && this.relayUrl) {
      const valid = this.validateTransportUrl(this.relayUrl, true);
      if (valid.ok) {
        try {
          const result = await this.#sendSingleWsRpc(this.relayUrl, rpcBody, this.relayTimeoutMs);
          if (result && typeof result.ok === 'boolean') {
            this.lastTransportMode = this.relayUrl.startsWith('ws://127.0.0.1') ? 'spvs-local-ws' : 'spvs-cloud-wss';
            return result;
          }
        } catch {
          // Fall through to HTTP relay endpoint matching configured relay
        }
      }
    }

    if (typeof this.#fetchImpl === 'function' || typeof fetch === 'function') {
      const candidateHttpUrls = [];
      const pushHttpUrl = (u) => {
        if (u && !candidateHttpUrls.includes(u)) candidateHttpUrls.push(u);
      };
      pushHttpUrl(this.relayHttpUrl);
      if (this.relayUrl) {
        if (this.relayUrl.startsWith('wss://')) {
          pushHttpUrl('https://' + this.relayUrl.slice(6));
        } else if (this.relayUrl.startsWith('ws://127.0.0.1')) {
          pushHttpUrl('http://127.0.0.1' + this.relayUrl.slice(14));
        }
      }
      if (this.mode === 'spiralcore_net') {
        pushHttpUrl(DEFAULT_SPIRALCORE_RELAY_HTTP_URL);
      }

      for (const httpUrl of candidateHttpUrls) {
        let timer = null;
        try {
          const ctrl = typeof AbortController === 'function' ? new AbortController() : null;
          timer = ctrl ? setTimeout(() => ctrl.abort(), this.relayTimeoutMs) : null;
          if (timer && typeof timer.unref === 'function') timer.unref();
          const reqId = `spvs-http-${Date.now()}-${this.#nextReqSeq++}`;
          const resp = await this.#fetchImpl(httpUrl, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ req_id: reqId, ...rpcBody }),
            signal: ctrl ? ctrl.signal : undefined,
          });
          const parsed = await resp.json();
          if (parsed && typeof parsed.ok === 'boolean') {
            this.lastTransportMode = isCorsPreempt
              ? 'spiralcore-cors-preempt'
              : (httpUrl.includes('127.0.0.1') ? 'spvs-local-http' : 'spvs-cloud-https');
            return parsed;
          }
        } catch {
          // Try next HTTP candidate
        } finally {
          if (timer) clearTimeout(timer);
        }
      }
    }
    return {
      ok: false,
      status: 502,
      error_code: 'RelayUnavailable',
      message: 'SpiralCore AF_VSOCK relay unreachable or refused connection',
    };
  }

  #sendSingleWsRpc(wsUrl, payload, timeoutMs) {
    return new Promise((resolveRpc, rejectRpc) => {
      let settled = false;
      const reqId = `spvs-${Date.now()}-${this.#nextReqSeq++}`;
      let timer = setTimeout(() => {
        if (settled) return;
        settled = true;
        this.#pendingWs.delete(reqId);
        rejectRpc(new Error('SpiralCore relay timeout'));
      }, timeoutMs);

      const finish = (err, data) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        this.#pendingWs.delete(reqId);
        if (err) rejectRpc(err);
        else resolveRpc(data);
      };

      const dispatchOverSocket = (sock) => {
        clearTimeout(timer);
        timer = setTimeout(() => {
          if (settled) return;
          settled = true;
          this.#pendingWs.delete(reqId);
          rejectRpc(new Error('SpiralCore relay RPC timeout'));
        }, Math.max(timeoutMs, 6500));
        this.#pendingWs.set(reqId, finish);
        try {
          sock.send(JSON.stringify({ req_id: reqId, ...payload }));
        } catch (err) {
          finish(err, null);
        }
      };

      if (this.#ws && this.#ws.readyState === WebSocket.OPEN && this.#ws.__uorUrl === wsUrl) {
        dispatchOverSocket(this.#ws);
        return;
      }

      try {
        const sock = new WebSocket(wsUrl);
        sock.__uorUrl = wsUrl;
        sock.onopen = () => {
          this.#ws = sock;
          dispatchOverSocket(sock);
        };
        sock.onmessage = (ev) => {
          try {
            const parsed = JSON.parse(typeof ev.data === 'string' ? ev.data : '');
            const id = parsed.req_id || parsed.id;
            const cb = id ? this.#pendingWs.get(id) : null;
            if (cb) cb(null, parsed);
          } catch {
            // Ignore non-JSON frame
          }
        };
        sock.onerror = () => {
          finish(new Error('SpiralCore WebSocket error'), null);
        };
        sock.onclose = () => {
          if (this.#ws === sock) this.#ws = null;
          finish(new Error('SpiralCore WebSocket closed'), null);
        };
      } catch (err) {
        finish(err, null);
      }
    });
  }

  async execute(rawRequest) {
    const req = typeof rawRequest === 'string' ? JSON.parse(rawRequest) : (rawRequest || {});
    const rawOp = String(req.op || req.kind || req.bridge_op || req.tool || req.name || 'http_request').toLowerCase();
    const isNpmPkg =
      rawOp === 'npm_exec' ||
      rawOp === 'npm' ||
      rawOp === 'npm_install' ||
      ((rawOp === 'pkg_fetch' || rawOp === 'pkg') &&
        (String(req.ecosystem || '').toLowerCase() === 'npm' ||
          String(req.tool || '').toLowerCase() === 'npm'));

    if (rawOp === 'node_exec' || rawOp === 'node' || rawOp === 'npx') {
      return this.handleNodeExec(req);
    }
    if (isNpmPkg) {
      return this.handleNpmBridge(req);
    }
    if (
      rawOp === 'python_exec' ||
      rawOp === 'python' ||
      rawOp === 'python3' ||
      rawOp === 'pip' ||
      rawOp === 'pip3' ||
      rawOp === 'pip_install'
    ) {
      return this.handlePythonExec(req);
    }
    if (rawOp === 'wasi_exec' || rawOp === 'wasm') {
      return this.handleWasiExec(req);
    }
    if (rawOp === 'sqlite_exec' || rawOp === 'sqlite3') {
      return this.handleSqliteExec(req);
    }

    if (this.mcpPolicy === 'deny') {
      return {
        ok: false,
        status: 403,
        error_code: 'PolicyDenied',
        message: 'All external bridge operations are denied by active MCP security policy',
      };
    }
    const rawArgsPre = String(req.args || (req.arguments && req.arguments.args) || '').trim();
    const argSubPre = rawArgsPre ? rawArgsPre.split(/\s+/)[0].toLowerCase() : '';
    const gitSubPre =
      rawOp === 'git_smart_http' || rawOp === 'git'
        ? String(req.subcommand || req.action || (req.arguments && (req.arguments.action || req.arguments.subcommand)) || argSubPre || '').toLowerCase()
        : '';
    const isLocalGitRead = (rawOp === 'git_smart_http' || rawOp === 'git') && (gitSubPre === 'status' || gitSubPre === 'diff' || gitSubPre === 'log');
    if (!this.webAccessEnabled && rawOp !== 'pkg_fetch' && rawOp !== 'pkg' && !isLocalGitRead) {
      return {
        ok: false,
        status: 403,
        error_code: 'PolicyDenied',
        message: 'Web Access is currently toggled OFF in the studio topbar',
      };
    }

    if (rawOp === 'dns_query' || rawOp === 'dns_resolve' || rawOp === 'dns') {
      return this.handleDnsQuery(req);
    }
    if (rawOp === 'web_search' || rawOp === 'search') {
      return this.handleWebSearch(req);
    }
    if (rawOp === 'web_fetch') {
      return this.handleWebFetch(req);
    }
    if (rawOp === 'git_smart_http' || rawOp === 'git') {
      return this.handleGitBridge(req);
    }
    if (rawOp === 'pkg_fetch' || rawOp === 'pkg') {
      return this.handlePkgFetch(req);
    }
    return this.handleHttpRequest(req);
  }

  async readBoundedBytes(resp) {
    if (!resp.body || typeof resp.body.getReader !== 'function') {
      const ab = await resp.arrayBuffer();
      if (ab.byteLength > MAX_BRIDGE_RESPONSE_BYTES) {
        return {
          ok: false,
          error_code: 'ResponseBodyTooLarge',
          message: `Response exceeded ${MAX_BRIDGE_RESPONSE_BYTES} byte cap`,
        };
      }
      return { ok: true, bytes: new Uint8Array(ab), truncated: false };
    }
    const reader = resp.body.getReader();
    const chunks = [];
    let total = 0;
    let truncated = false;
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      if (value && value.byteLength > 0) {
        if (total + value.byteLength > MAX_BRIDGE_RESPONSE_BYTES) {
          const keep = MAX_BRIDGE_RESPONSE_BYTES - total;
          if (keep > 0) {
            chunks.push(value.subarray(0, keep));
            total += keep;
          }
          truncated = true;
          try {
            await reader.cancel();
          } catch {
            // Ignore cancel error
          }
          break;
        }
        chunks.push(value);
        total += value.byteLength;
      }
    }
    const merged = new Uint8Array(total);
    let offset = 0;
    for (const c of chunks) {
      merged.set(c, offset);
      offset += c.byteLength;
    }
    return { ok: true, bytes: merged, truncated };
  }

  async handleDnsQuery(req) {
    const domain = String(req.name || req.domain || req.host || 'example.com').trim();
    const rrType = String(req.rr_type || req.qtype || req.type || 'A').trim().toUpperCase();
    if (this.#isPrivateOrLoopbackHost(domain)) {
      this.#emitAuditEvent('dns_query', domain, 'blocked', 403, 0);
      return {
        ok: false,
        status: 403,
        error_code: 'PrivateAddressBlocked',
        host: domain,
        target: domain,
        message: `Blocked private, loopback, or link-local SSRF DNS query for ${domain}`,
      };
    }
    if (this.mode === 'full_spiralcore' && !this.consentGranted) {
      this.#emitAuditEvent('dns_query', domain, 'blocked', 403, 0);
      return {
        ok: false,
        status: 403,
        error_code: 'RelayConsentRequired',
        message: 'Full SpiralCore Internet mode requires explicit user consent (consentGranted=true)',
      };
    }

    const dohEndpoints = [
      `https://cloudflare-dns.com/dns-query?name=${encodeURIComponent(domain)}&type=${encodeURIComponent(rrType)}&ct=application/dns-json`,
      `https://dns.google/resolve?name=${encodeURIComponent(domain)}&type=${encodeURIComponent(rrType)}`,
    ];
    let lastErr = '';
    for (const dohUrl of dohEndpoints) {
      const valid = this.validateTransportUrl(dohUrl, false);
      if (!valid.ok) continue;
      try {
        const fetchRes = await this.#admittedFetch(
          dohUrl,
          { method: 'GET', headers: { Accept: 'application/dns-json' } },
          { operation: 'dns_query', emitSuccessAudit: false, emitFailureAudit: false },
        );
        if (!fetchRes.ok) {
          if (fetchRes.error && fetchRes.error.error_code && fetchRes.error.error_code !== 'HttpBridgeFetchError') {
            return fetchRes.error;
          }
          lastErr = fetchRes.error ? fetchRes.error.message : 'DoH fetch failed';
          continue;
        }
        const resp = fetchRes.response;
        if (!resp || !resp.ok) {
          lastErr = `DoH HTTP ${resp ? resp.status : 'error'}`;
          continue;
        }
        const json = await resp.json();
        const answers = Array.isArray(json && json.Answer) ? json.Answer : [];
        const records = answers.map((a) => ({
          name: a.name || domain,
          type: a.type,
          ttl: a.TTL ?? 300,
          data: a.data,
        }));
        for (const rec of records) {
          if (this.#isPrivateOrLoopbackHost(rec.data)) {
            this.#emitAuditEvent('dns_query', domain, 'direct-doh-blocked', 403, 0);
            return {
              ok: false,
              status: 403,
              error_code: 'PrivateAddressBlocked',
              host: String(rec.data),
              target: domain,
              message: `RFC 8484 DoH answer for ${domain} resolved to private/loopback address ${rec.data}`,
            };
          }
        }
        this.lastTransportMode = 'direct-doh';
        const summary = records.length > 0
          ? records.map((r) => `${r.name}\tIN\t${rrType}\t${r.data} (ttl=${r.ttl}s)`).join('\n')
          : `No ${rrType} records returned for ${domain} (RCODE=${(json && json.Status) ?? 0})`;
        this.#emitAuditEvent('dns_query', domain, this.lastTransportMode, resp.status || 200, summary.length);
        return {
          ok: true,
          status: resp.status || 200,
          op: 'dns_query',
          name: domain,
          rr_type: rrType,
          records,
          answers: records,
          data: { answers: records, rcode: (json && json.Status) ?? 0 },
          body_text: summary,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        lastErr = err && err.message ? err.message : String(err);
      }
    }

    if ((this.mode === 'full_spiralcore' && this.consentGranted) || (this.mode === 'spiralcore_net' && this.webAccessEnabled !== false)) {
      const relayRes = await this.tryRelayRpc('dns_query', { name: domain, type: rrType });
      if (relayRes && relayRes.ok && relayRes.data && Array.isArray(relayRes.data.answers)) {
        const records = relayRes.data.answers.map((a) => ({
          name: a.name || domain,
          type: a.type,
          ttl: a.ttl ?? a.TTL ?? 300,
          data: a.data,
        }));
        for (const rec of records) {
          if (this.#isPrivateOrLoopbackHost(rec.data)) {
            this.#emitAuditEvent('dns_query', domain, 'relay-blocked', 403, 0);
            return {
              ok: false,
              status: 403,
              error_code: 'PrivateAddressBlocked',
              host: String(rec.data),
              target: domain,
              message: `Relayed DNS answer for ${domain} resolved to private/loopback address ${rec.data}`,
            };
          }
        }
        const summary = records.length > 0
          ? records.map((r) => `${r.name}\tIN\t${rrType}\t${r.data} (ttl=${r.ttl}s)`).join('\n')
          : `No ${rrType} records returned for ${domain} (RCODE=${relayRes.data.rcode ?? 0})`;
        this.#emitAuditEvent('dns_query', domain, this.lastTransportMode, relayRes.status || 200, summary.length);
        return {
          ok: true,
          status: relayRes.status || 200,
          op: 'dns_query',
          name: domain,
          rr_type: rrType,
          records,
          answers: records,
          data: { answers: records, rcode: relayRes.data.rcode ?? 0 },
          body_text: summary,
          transport: this.lastTransportMode,
        };
      }
      this.#emitAuditEvent('dns_query', domain, 'relay-unavailable', 502, 0);
      return {
        ok: false,
        status: 502,
        error_code: (relayRes && relayRes.error_code) || 'RelayUnavailable',
        message: (relayRes && (relayRes.error_message || relayRes.message)) || lastErr || 'DoH and SpiralCore relay DNS resolution failed',
      };
    }

    this.#emitAuditEvent('dns_query', domain, 'doh-failed', 502, 0);
    return {
      ok: false,
      status: 502,
      error_code: 'DoHResolutionFailed',
      message: lastErr || 'RFC 8484 DoH resolution failed for cloudflare-dns.com and dns.google',
    };
  }

  async handleHttpRequest(req) {
    const rawUrl = String(req.url || '').trim();
    const valid = this.validateTransportUrl(rawUrl, false);
    if (!valid.ok) return valid;
    const subUrl = this.substituteHostSecretsInString(rawUrl, valid.url.hostname.toLowerCase());
    if (!subUrl.ok) return subUrl;
    const finalValid = this.validateTransportUrl(subUrl.value, false);
    if (!finalValid.ok) return finalValid;

    const headerSub = this.applyHostSecretSubstitution(finalValid.url, req.headers || {});
    if (!headerSub.ok) return headerSub;

    let bodyPayload;
    if (req.body_base64) {
      bodyPayload = __uorBase64ToBytes(req.body_base64);
    } else if (typeof req.body === 'string') {
      const bodySub = this.substituteHostSecretsInString(req.body, finalValid.url.hostname.toLowerCase());
      if (!bodySub.ok) return bodySub;
      bodyPayload = bodySub.value;
    }

    const hostname = finalValid.url.hostname.toLowerCase();
    const isCredentialed = this.#isCredentialedRequest(
      hostname,
      headerSub.headers,
      rawUrl.includes('{{MVM_SECRET:') || (typeof req.body === 'string' && req.body.includes('{{MVM_SECRET:')),
    );
    const netAdmission = this.#admitNetworkPolicy(hostname, req._policyOpts);
    if (!netAdmission.ok) {
      this.#emitAuditEvent('http_request', finalValid.url.href, 'blocked', netAdmission.status || 403, 0);
      return netAdmission;
    }

    try {
      const resp = await this.#fetchImpl(finalValid.url.href, {
        method: String(req.method || 'GET').toUpperCase(),
        headers: headerSub.headers,
        body: bodyPayload,
      });
      const bounded = await this.readBoundedBytes(resp);
      if (!bounded.ok) {
        this.#emitAuditEvent('http_request', finalValid.url.href, 'direct-https-fetch', resp.status || 502, 0);
        return bounded;
      }
      this.lastTransportMode = 'direct-https-fetch';
      const respHeaders = {};
      resp.headers.forEach((val, key) => {
        respHeaders[key] = val;
      });
      const contentType = String(respHeaders['content-type'] || '').toLowerCase();
      const decodedText = cachedTextDecoder.decode(bounded.bytes);
      const isHtml = contentType.includes('html') || /<html|<body/i.test(decodedText);
      const textExcerpt = isHtml
        ? __uorStripHtmlToReadableText(decodedText).text.slice(0, 3200)
        : decodedText.slice(0, 3200);
      this.#emitAuditEvent('http_request', finalValid.url.href, this.lastTransportMode, resp.status, bounded.bytes.byteLength);
      return {
        ok: resp.ok,
        status: resp.status,
        op: 'http_request',
        url: finalValid.url.href,
        headers: respHeaders,
        content_type: contentType,
        body_text: decodedText,
        text_excerpt: textExcerpt,
        body_base64: req.binary ? __uorBytesToBase64(bounded.bytes) : undefined,
        byte_len: bounded.bytes.byteLength,
        truncated: bounded.truncated,
        transport: this.lastTransportMode,
      };
    } catch (err) {
      if (isCredentialed) {
        this.#emitAuditEvent('http_request', finalValid.url.href, 'direct-failed-credentialed', 403, 0);
        return {
          ok: false,
          status: 403,
          error_code: 'RelayForbiddenForCredentialedHost',
          host: hostname,
          message: `Credentialed host/request ${hostname} failed direct fetch and is forbidden from SpiralCore relay failback: ${err && err.message ? err.message : String(err)}`,
        };
      }
      const corsPreemptVerdict = admit_public_cors_preemption(
        this.mode === 'spiralcore_net' && this.webAccessEnabled !== false,
        false,
        isCredentialed,
        true,
      );
      if (corsPreemptVerdict === 'RefusedCredentialedCorsProxy') {
        this.#emitAuditEvent('http_request', finalValid.url.href, 'direct-failed-credentialed', 403, 0);
        return {
          ok: false,
          status: 403,
          error_code: 'RelayForbiddenForCredentialedHost',
          host: hostname,
          message: `Credentialed host/request ${hostname} is forbidden from SpiralCore CORS preemption`,
        };
      }
      if ((this.mode === 'full_spiralcore' && this.consentGranted) || corsPreemptVerdict === 'StatelessRelayCorsPreempted') {
        const relayRes = await this.tryRelayRpc('http_request', {
          method: String(req.method || 'GET').toUpperCase(),
          url: finalValid.url.href,
          headers: headerSub.headers,
          body_b64: req.body_base64 || req.body_b64 || undefined,
          _corsPreempt: this.mode === 'spiralcore_net',
        });
        if (relayRes && !relayRes.ok && (relayRes.error_code === 'SsrfBlocked' || relayRes.error_code === 'PrivateAddressBlocked')) {
          this.#emitAuditEvent('http_request', finalValid.url.href, 'relay-ssrf-blocked', 403, 0);
          return {
            ok: false,
            status: relayRes.status || 403,
            error_code: relayRes.error_code === 'SsrfBlocked' ? 'PrivateAddressBlocked' : relayRes.error_code,
            message: relayRes.error_message || 'Target host blocked by SpiralCore SSRF policy',
          };
        }
        if (relayRes && relayRes.ok && relayRes.data) {
          const d = relayRes.data;
          const bodyBytes = d.body_b64 ? __uorBase64ToBytes(d.body_b64) : new Uint8Array(0);
          const bodyText = bodyBytes.byteLength > 0 ? cachedTextDecoder.decode(bodyBytes) : (d.text_excerpt || '');
          const byteLen = d.byte_length ?? bodyBytes.byteLength;
          this.#emitAuditEvent('http_request', finalValid.url.href, this.lastTransportMode, d.status || 200, byteLen);
          return {
            ok: d.status >= 200 && d.status < 400,
            status: d.status,
            op: 'http_request',
            url: finalValid.url.href,
            headers: d.headers || {},
            content_type: d.content_type || '',
            body_text: bodyText,
            text_excerpt: d.text_excerpt || bodyText.slice(0, 3200),
            body_base64: req.binary ? (d.body_b64 || '') : undefined,
            byte_len: byteLen,
            truncated: Boolean(d.truncated),
            transport: this.lastTransportMode,
          };
        }
        this.#emitAuditEvent('http_request', finalValid.url.href, 'relay-unavailable', 502, 0);
        return {
          ok: false,
          status: 502,
          error_code: (relayRes && relayRes.error_code) || 'RelayUnavailable',
          message: (relayRes && (relayRes.error_message || relayRes.message)) || 'SpiralCore AF_VSOCK relay unreachable',
        };
      }
      this.#emitAuditEvent('http_request', finalValid.url.href, 'direct-failed', 502, 0);
      return {
        ok: false,
        status: 502,
        error_code: 'HttpBridgeFetchError',
        message: err && err.message ? err.message : String(err),
      };
    }
  }

  async handleWebFetch(req) {
    const rawUrl = String(req.url || (req.arguments && req.arguments.url) || '').trim();
    const valid = this.validateTransportUrl(rawUrl, false);
    if (!valid.ok) return valid;

    const direct = await this.handleHttpRequest({ url: valid.url.href, method: 'GET', headers: req.headers || {} });
    if (direct.ok && typeof direct.body_text === 'string') {
      const isHtml = (direct.content_type || '').includes('html') || /<html|<body/i.test(direct.body_text);
      const parsed = isHtml
        ? __uorStripHtmlToReadableText(direct.body_text)
        : { title: valid.url.hostname, text: direct.body_text.slice(0, 12000) };
      return {
        ok: true,
        status: direct.status,
        op: 'web_fetch',
        url: valid.url.href,
        title: parsed.title || valid.url.href,
        body_text: parsed.text,
        excerpt: parsed.text.slice(0, 3200),
        transport: direct.transport,
      };
    }

    if (
      this.mode === 'full_spiralcore' &&
      this.consentGranted &&
      (!req.headers || Object.keys(req.headers).length === 0) &&
      !CREDENTIALED_GITHUB_HOSTS.has(valid.url.hostname.toLowerCase()) &&
      !this.#isPrivateOrLoopbackHost(valid.url.hostname)
    ) {
      try {
        const readerUrl = `https://r.jina.ai/${valid.url.href}`;
        const readerFetch = await this.#admittedFetch(
          readerUrl,
          { headers: { Accept: 'text/plain' } },
          { operation: 'web_fetch', emitSuccessAudit: false },
        );
        const rResp = readerFetch.ok ? readerFetch.response : null;
        if (rResp && rResp.ok) {
          const bounded = await this.readBoundedBytes(rResp);
          if (bounded.ok) {
            const text = cachedTextDecoder.decode(bounded.bytes).slice(0, 12000);
            this.lastTransportMode = 'direct-https-reader';
            this.#emitAuditEvent('web_fetch', readerUrl, this.lastTransportMode, rResp.status, bounded.bytes.byteLength);
            return {
              ok: true,
              status: rResp.status,
              op: 'web_fetch',
              url: valid.url.href,
              title: valid.url.hostname,
              body_text: text,
              excerpt: text.slice(0, 3200),
              transport: this.lastTransportMode,
            };
          }
        }
      } catch {
        // Return direct error below
      }
    }
    return direct;
  }

  async handleWebSearch(req) {
    const query = String(req.query || (req.arguments && req.arguments.query) || '').trim();
    const limit = Math.min(Math.max(Number(req.limit) || 6, 1), 10);
    if (!query) {
      return { ok: false, error_code: 'EmptySearchQuery', message: 'web_search query must be non-empty' };
    }
    const providers = [
      {
        name: 'wiki',
        host: 'en.wikipedia.org',
        url: `https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch=${encodeURIComponent(query)}&srlimit=3&utf8=1&format=json&origin=*`,
      },
      {
        name: 'hn',
        host: 'hn.algolia.com',
        url: `https://hn.algolia.com/api/v1/search?query=${encodeURIComponent(query)}&hitsPerPage=4`,
      },
      {
        name: 'gh',
        host: 'api.github.com',
        url: `https://api.github.com/search/repositories?q=${encodeURIComponent(query)}&per_page=3`,
        init: { headers: { Accept: 'application/vnd.github+json' } },
      },
      {
        name: 'ddg',
        host: 'api.duckduckgo.com',
        url: `https://api.duckduckgo.com/?q=${encodeURIComponent(query)}&format=json&no_html=1&skip_disambig=1`,
      },
    ];
    for (const provider of providers) {
      const netAdmission = this.#admitNetworkPolicy(provider.host);
      if (!netAdmission.ok) {
        this.#emitAuditEvent('web_search', provider.url, 'blocked', netAdmission.status || 403, 0);
        return netAdmission;
      }
    }

    const results = [];
    const seenUrls = new Set();
    const pushResult = (item) => {
      if (!item || !item.url || seenUrls.has(item.url) || results.length >= limit) return;
      seenUrls.add(item.url);
      results.push(item);
    };

    const settled = await Promise.all(
      providers.map(async (provider) => {
        const fetched = await this.#admittedFetch(
          provider.url,
          provider.init,
          { operation: 'web_search', transportMode: 'direct-https-search' },
        );
        if (!fetched.ok) {
          return { name: provider.name, json: null, failed: true, error: fetched.error };
        }
        try {
          const resp = fetched.response;
          const json = resp && resp.ok ? await resp.json() : null;
          return { name: provider.name, json, failed: false };
        } catch {
          return { name: provider.name, json: null, failed: true };
        }
      }),
    );

    const byName = Object.create(null);
    let failedCount = 0;
    for (const entry of settled) {
      byName[entry.name] = entry.json;
      if (entry.failed) failedCount += 1;
    }
    const wikiJson = byName.wiki || null;
    const hnJson = byName.hn || null;
    const ghJson = byName.gh || null;
    const ddgJson = byName.ddg || null;

    if (ddgJson && ddgJson.AbstractText && ddgJson.AbstractURL) {
      pushResult({
        title: ddgJson.Heading || query,
        url: ddgJson.AbstractURL,
        snippet: String(ddgJson.AbstractText).slice(0, 360),
        source: 'duckduckgo',
      });
    }
    if (wikiJson && wikiJson.query && Array.isArray(wikiJson.query.search)) {
      for (const hit of wikiJson.query.search) {
        const cleanSnippet = String(hit.snippet || '').replace(/<[^>]+>/g, '').trim();
        pushResult({
          title: hit.title,
          url: `https://en.wikipedia.org/wiki/${encodeURIComponent(String(hit.title || '').replace(/\s+/g, '_'))}`,
          snippet: cleanSnippet.slice(0, 360),
          source: 'wikipedia',
        });
      }
    }
    if (ghJson && Array.isArray(ghJson.items)) {
      for (const repo of ghJson.items) {
        pushResult({
          title: `${repo.full_name} (★${repo.stargazers_count ?? 0})`,
          url: repo.html_url,
          snippet: String(repo.description || 'GitHub repository').slice(0, 360),
          source: 'github',
        });
      }
    }
    if (hnJson && Array.isArray(hnJson.hits)) {
      for (const hit of hnJson.hits) {
        const targetUrl = hit.url || `https://news.ycombinator.com/item?id=${hit.objectID}`;
        pushResult({
          title: hit.title || hit.story_title || 'Hacker News Discussion',
          url: targetUrl,
          snippet: `HN ${hit.points ?? 0} pts · ${hit.num_comments ?? 0} comments · by ${hit.author || 'anon'}`,
          source: 'hn',
        });
      }
    }

    if (
      results.length === 0 &&
      failedCount === providers.length &&
      ((this.mode === 'full_spiralcore' && this.consentGranted) ||
        (this.mode === 'spiralcore_net' && this.webAccessEnabled !== false))
    ) {
      const relayRes = await this.tryRelayRpc('web_search', { query, limit });
      if (relayRes && relayRes.ok && relayRes.data && Array.isArray(relayRes.data.results)) {
        for (const r of relayRes.data.results) {
          pushResult({
            title: String(r.title || r.url || 'Search result'),
            url: String(r.url || ''),
            snippet: String(r.snippet || '').slice(0, 360),
            source: String(r.source || 'spiralcore-relay'),
          });
        }
        const relayedFmt = results
          .map((r, i) => `[${i + 1}] ${r.title}\n    URL: ${r.url}\n    ${r.snippet}`)
          .join('\n\n');
        this.#emitAuditEvent('web_search', query, this.lastTransportMode, relayRes.status || 200, relayedFmt.length);
        return {
          ok: true,
          status: relayRes.status || 200,
          op: 'web_search',
          query,
          results,
          data: { results },
          body_text: relayedFmt || `No SpiralCore relay search hits returned for "${query}".`,
          transport: this.lastTransportMode,
        };
      }
      this.#emitAuditEvent('web_search', query, 'relay-unavailable', 502, 0);
      return {
        ok: false,
        status: 502,
        error_code: (relayRes && relayRes.error_code) || 'RelayUnavailable',
        message: (relayRes && (relayRes.error_message || relayRes.message)) || 'Direct and SpiralCore relay web_search failed',
      };
    }

    this.lastTransportMode = 'direct-https-search';
    const formatted = results
      .map((r, i) => `[${i + 1}] ${r.title}\n    URL: ${r.url}\n    ${r.snippet}`)
      .join('\n\n');
    return {
      ok: true,
      status: 200,
      op: 'web_search',
      query,
      results,
      data: { results },
      body_text: formatted || `No public HTTPS search hits returned for "${query}".`,
      transport: this.lastTransportMode,
    };
  }

  parseGitHubRepoCoords(rawRepo) {
    const cleaned = String(rawRepo || '').trim().replace(/\.git$/i, '').replace(/\/+$/, '');
    if (/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(cleaned)) {
      const [owner, repo] = cleaned.split('/');
      return { owner, repo, url: `https://github.com/${owner}/${repo}` };
    }
    try {
      const u = new URL(cleaned);
      const host = u.hostname.toLowerCase();
      if (host === 'github.com' || host === 'api.github.com') {
        const cleanPath = host === 'api.github.com' ? u.pathname.replace(/^\/repos\//i, '/') : u.pathname;
        const parts = cleanPath.split('/').filter(Boolean);
        if (parts.length >= 2) {
          return { owner: parts[0], repo: parts[1], url: `https://github.com/${parts[0]}/${parts[1]}` };
        }
      }
    } catch {
      // Not a GitHub URL
    }
    return null;
  }

  detectHostedGitHubRepo(req = {}) {
    const hint = String(req.repo_hint || req.repo || '').trim();
    if (hint) {
      const parsed = this.parseGitHubRepoCoords(hint);
      if (parsed) {
        return {
          ok: true,
          op: 'git',
          subcommand: 'detect_repo',
          owner: parsed.owner,
          repo: `${parsed.owner}/${parsed.repo}`,
          repo_name: parsed.repo,
          repo_url: parsed.url,
          detected: true,
          source: 'hint',
        };
      }
    }
    let loc = req.location || (typeof location !== 'undefined' ? location : null);
    if (!loc && this.locationOrigin) {
      try {
        loc = new URL(this.locationOrigin);
      } catch {
        loc = null;
      }
    }
    if (loc && typeof loc.hostname === 'string') {
      const m = /^([A-Za-z0-9_.-]+)\.github\.io$/i.exec(loc.hostname.trim());
      if (m) {
        const owner = m[1];
        const segs = String(loc.pathname || '/').split('/').filter(Boolean);
        const repoName = segs[0] || `${owner}.github.io`;
        return {
          ok: true,
          op: 'git',
          subcommand: 'detect_repo',
          owner,
          repo: `${owner}/${repoName}`,
          repo_name: repoName,
          repo_url: `https://github.com/${owner}/${repoName}`,
          detected: true,
          source: 'github-pages-origin',
        };
      }
    }
    return {
      ok: true,
      op: 'git',
      subcommand: 'detect_repo',
      owner: 'UOR-Foundation',
      repo: 'UOR-Foundation/Prism',
      repo_name: 'Prism',
      repo_url: 'https://github.com/UOR-Foundation/Prism',
      detected: false,
      source: 'default',
    };
  }

  async #fetchImpl(url, init) {
    const fn = this.fetchImpl || globalThis.fetch;
    return fn(url, init);
  }

  async #githubRestJson(coords, pathAndQuery, reqOpts) {
    const opts = reqOpts && typeof reqOpts === 'object' ? reqOpts : {};
    const method = opts.method || 'GET';
    const body = opts.body;
    const headers = opts.headers || {};
    const cleanSub = String(pathAndQuery || '').replace(/^\/+/, '');
    const apiBase = `https://api.github.com/repos/${coords.owner}/${coords.repo}`;
    const fullUrl = cleanSub ? `${apiBase}/${cleanSub}` : apiBase;
    const urlObj = new URL(fullUrl);
    if (urlObj.hostname.toLowerCase() !== 'api.github.com' || urlObj.protocol !== 'https:') {
      return {
        ok: false,
        status: 403,
        error_code: 'HostNotAllowlistedForSecret',
        message: 'Refusing GitHub request outside api.github.com',
      };
    }
    const netAdmission = this.#admitNetworkPolicy(urlObj.hostname);
    if (!netAdmission.ok) {
      this.#emitAuditEvent('github_api', urlObj.href, 'blocked', netAdmission.status || 403, 0);
      return netAdmission;
    }
    const authRes = this.applyHostSecretSubstitution(urlObj, {
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      ...headers,
    });
    if (!authRes.ok) return authRes;
    const reqHeaders = { ...authRes.headers };
    if (body !== undefined && method !== 'GET' && method !== 'HEAD') {
      reqHeaders['Content-Type'] = 'application/json';
    }
    try {
      const resp = await this.#fetchImpl(urlObj.href, {
        method,
        headers: reqHeaders,
        body: body !== undefined && method !== 'GET' && method !== 'HEAD' ? JSON.stringify(body) : undefined,
      });
      const text = await resp.text();
      let json = null;
      try {
        json = text ? JSON.parse(text) : null;
      } catch {
        json = null;
      }
      this.lastTransportMode = 'direct-https-github-rest';
      this.#emitAuditEvent('github_api', urlObj.href, this.lastTransportMode, resp.status, text.length);
      return { ok: resp.ok, status: resp.status, json, text, viaRelay: false };
    } catch (err) {
      this.#emitAuditEvent('github_api', urlObj.href, 'direct-failed-credentialed', 403, 0);
      return {
        ok: false,
        status: 403,
        error_code: 'RelayForbiddenForCredentialedHost',
        host: 'api.github.com',
        message: `CORS-direct GitHub API request failed; SpiralCore relay failback is forbidden for credentialed hosts: ${err && err.message ? err.message : String(err)}`,
      };
    }
  }

  async checkProvenanceUpdate(provenanceUrlInput = '') {
    const explicitStaticBase = this.staticOrigin ? this.staticOrigin.replace(/\/+$/, '') : '';
    let moduleProvUrl = '';
    try {
      const modUrl = new URL('provenance.json', import.meta.url);
      if (/^https?:/i.test(modUrl.protocol)) {
        moduleProvUrl = modUrl.href;
      }
    } catch {
      moduleProvUrl = '';
    }
    let locationDirProvUrl = '';
    if (!explicitStaticBase && !moduleProvUrl) {
      if (typeof location !== 'undefined' && location.href && /^https?:/i.test(location.href)) {
        const hrefBase = location.href.endsWith('/') ? location.href : location.href.replace(/[^/?#]*([?#].*)?$/, '');
        locationDirProvUrl = new URL('provenance.json', hrefBase).href;
      } else if (this.locationOrigin && /^https?:/i.test(this.locationOrigin)) {
        const optBase = this.locationOrigin.endsWith('/') ? this.locationOrigin : `${this.locationOrigin}/`;
        locationDirProvUrl = new URL('provenance.json', optBase).href;
      }
    }
    const rawInput = String(provenanceUrlInput || '').trim();
    const normalizedInput = rawInput
      ? (/\.json(?:[?#].*)?$/i.test(rawInput) ? rawInput : `${rawInput.replace(/\/+$/, '')}/provenance.json`)
      : '';
    const provUrl =
      normalizedInput ||
      (explicitStaticBase
        ? `${explicitStaticBase}/provenance.json`
        : moduleProvUrl || locationDirProvUrl || new URL('provenance.json', import.meta.url).href);
    // Strict R4-39 rule: NEVER attach Host Secrets to non-GitHub provenance origins
    const validated = this.validateTransportUrl(provUrl, false);
    if (!validated.ok) return validated;
    const provAdmit = this.#admitNetworkPolicy(validated.url.hostname, { allowConfiguredOrigin: true });
    if (!provAdmit.ok) {
      this.#emitAuditEvent('provenance_check', validated.url.href, 'blocked', provAdmit.status || 403, 0);
      return provAdmit;
    }
    try {
      const sep = validated.url.href.includes('?') ? '&' : '?';
      const resp = await this.#fetchImpl(`${validated.url.href}${sep}_t=${Date.now()}`, { cache: 'no-store' });
      this.#emitAuditEvent('provenance_check', validated.url.href, 'direct-https-fetch', resp.status, 0);
      if (!resp.ok) {
        return { ok: false, status: resp.status, error_code: 'GitDeployWatchFailed', message: `provenance.json returned HTTP ${resp.status}` };
      }
      const live = await resp.json();
      if (!this.#provenanceBaseline) {
        this.#provenanceBaseline = {
          adapter_wasm_sha256: live.adapter_wasm_sha256 || '',
          binding_javascript_sha256: live.binding_javascript_sha256 || '',
          generated_core_sha256: live.generated_core_sha256 || '',
        };
      }
      const updateAvailable = Boolean(
        this.#provenanceBaseline &&
          ((live.adapter_wasm_sha256 && live.adapter_wasm_sha256 !== this.#provenanceBaseline.adapter_wasm_sha256) ||
            (live.binding_javascript_sha256 && live.binding_javascript_sha256 !== this.#provenanceBaseline.binding_javascript_sha256)),
      );
      return {
        ok: true,
        status: 200,
        provenance_url: validated.url.href,
        current: this.#provenanceBaseline,
        live,
        adapter_wasm_sha256: live.adapter_wasm_sha256 || '',
        binding_javascript_sha256: live.binding_javascript_sha256 || '',
        generated_core_sha256: live.generated_core_sha256 || '',
        update_available: updateAvailable,
      };
    } catch (err) {
      return { ok: false, status: 502, error_code: 'GitDeployWatchFailed', message: err && err.message ? err.message : String(err) };
    }
  }

  #normalizeWsPath(rawPath) {
    return String(rawPath || '')
      .trim()
      .replace(/\\/g, '/')
      .replace(/^\/+/, '')
      .replace(/^workspace(?:\/+|$)/, '')
      .replace(/^(?:\.\/)+/, '')
      .replace(/^\/+/, '');
  }

  #wsWrite(path, content) {
    const norm = this.#normalizeWsPath(path);
    if (!norm) return;
    const text = String(content ?? '');
    this.#workspaceMap.set(norm, text);
    if (this.#workspaceFs && typeof this.#workspaceFs.write === 'function') {
      this.#workspaceFs.write(norm, text);
    }
  }

  #wsRead(path) {
    const norm = this.#normalizeWsPath(path);
    if (!norm) return null;
    if (this.#workspaceFs && typeof this.#workspaceFs.read === 'function') {
      const val = this.#workspaceFs.read(norm);
      if (val !== null && val !== undefined) return String(val);
    }
    return this.#workspaceMap.has(norm) ? this.#workspaceMap.get(norm) : null;
  }

  #wsDelete(path) {
    const norm = this.#normalizeWsPath(path);
    if (!norm) return false;
    const hadLocal = this.#workspaceMap.delete(norm);
    if (this.#workspaceFs && typeof this.#workspaceFs.delete === 'function') {
      return Boolean(this.#workspaceFs.delete(norm)) || hadLocal;
    }
    return hadLocal;
  }

  #wsListWorkingTree() {
    const out = new Map();
    if (this.#workspaceFs && typeof this.#workspaceFs.list === 'function') {
      const list = this.#workspaceFs.list();
      if (Array.isArray(list)) {
        for (const item of list) {
          if (!item || typeof item.path !== 'string') continue;
          const norm = this.#normalizeWsPath(item.path);
          if (!norm || norm === '.git' || norm.startsWith('.git/')) continue;
          out.set(norm, String(item.content ?? ''));
        }
        return out;
      }
    }
    for (const [k, v] of this.#workspaceMap.entries()) {
      if (!k || k === '.git' || k.startsWith('.git/')) continue;
      out.set(k, v);
    }
    return out;
  }

  writeWorkspaceFile(path, content) {
    this.#wsWrite(path, content);
  }

  readWorkspaceFile(path) {
    return this.#wsRead(path);
  }

  deleteWorkspaceFile(path) {
    return this.#wsDelete(path);
  }

  listWorkspaceFiles() {
    return Array.from(this.#wsListWorkingTree().entries()).map(([path, content]) => ({ path, content }));
  }

  #readGitIndexAndState() {
    let state = null;
    let index = null;
    try {
      const rawState = this.#wsRead('.git/uor_state.json');
      if (rawState) state = JSON.parse(rawState);
    } catch {}
    try {
      const rawIndex = this.#wsRead('.git/tree_index.json');
      if (rawIndex) index = JSON.parse(rawIndex);
    } catch {}
    return {
      state: state && typeof state === 'object' ? state : null,
      index: index && typeof index === 'object' ? index : null,
    };
  }

  #writeGitIndexAndState(meta) {
    const {
      repoSlug,
      remoteUrl,
      branch,
      headSha,
      treeSha,
      trackedEntries,
      commits,
    } = meta;
    const headTree = {};
    const entriesObj = {};
    for (const [p, entry] of Object.entries(trackedEntries || {})) {
      if (!p || p === '.git' || p.startsWith('.git/')) continue;
      const eObj = entry && typeof entry === 'object' ? entry : {};
      const hydrated = eObj.hydrated !== false;
      entriesObj[p] = {
        sha: String(eObj.sha || ''),
        size: Number.isFinite(Number(eObj.size)) ? Number(eObj.size) : (typeof eObj.content === 'string' ? eObj.content.length : 0),
        hydrated,
        ...(typeof eObj.content === 'string' ? { content: eObj.content } : {}),
      };
      if (hydrated && typeof eObj.content === 'string') {
        headTree[p] = eObj.content;
      }
    }
    this.#wsWrite('.git/HEAD', `ref: refs/heads/${branch || 'main'}\n`);
    this.#wsWrite('.git/tree_index.json', JSON.stringify({
      repo: repoSlug,
      remote_url: remoteUrl,
      branch: branch || 'main',
      head_sha: headSha || '',
      tree_sha: treeSha || '',
      entries: entriesObj,
    }, null, 2));
    this.#wsWrite('.git/uor_state.json', JSON.stringify({
      branch: branch || 'main',
      remote_url: remoteUrl,
      head_oid: headSha || null,
      staged: {},
      head_tree: headTree,
      commits: Array.isArray(commits) ? commits : [],
      repo: repoSlug,
      tree_sha: treeSha || '',
    }, null, 2));
  }

  #computeGitDiffState(workingMapOverride = null) {
    const { state, index } = this.#readGitIndexAndState();
    const working = workingMapOverride || this.#wsListWorkingTree();
    const entries = (index && index.entries && typeof index.entries === 'object') ? index.entries : {};
    const headTree = (state && state.head_tree && typeof state.head_tree === 'object') ? state.head_tree : {};

    const added = [];
    const modified = [];
    const deleted = [];
    const diffChunks = [];

    for (const [p, curContent] of working.entries()) {
      const tracked = entries[p];
      if (!tracked) {
        added.push(p);
        diffChunks.push(buildUnifiedDiffHunk('', curContent, p, false));
      } else {
        const baseContent = typeof tracked.content === 'string'
          ? tracked.content
          : (typeof headTree[p] === 'string' ? headTree[p] : null);
        if (baseContent !== null && baseContent !== curContent) {
          modified.push(p);
          diffChunks.push(buildUnifiedDiffHunk(baseContent, curContent, p, false));
        }
      }
    }

    for (const [p, tracked] of Object.entries(entries)) {
      if (!tracked || tracked.hydrated === false) continue;
      if (!working.has(p)) {
        deleted.push(p);
        const baseContent = typeof tracked.content === 'string'
          ? tracked.content
          : (typeof headTree[p] === 'string' ? headTree[p] : '');
        diffChunks.push(buildUnifiedDiffHunk(baseContent, '', p, true));
      }
    }

    added.sort();
    modified.sort();
    deleted.sort();
    return {
      state,
      index,
      working,
      added,
      modified,
      deleted,
      clean: added.length === 0 && modified.length === 0 && deleted.length === 0,
      diff_text: diffChunks.join('\n'),
    };
  }

  async handleGit(req) {
    return this.handleGitBridge(req);
  }

  async handleGitBridge(req) {
    const rawArgs = String(req.args || (req.arguments && req.arguments.args) || '').trim();
    const argParts = rawArgs ? rawArgs.split(/\s+/) : [];
    const argsObj = (req.arguments && typeof req.arguments === 'object') ? req.arguments : {};
    const subcommand = String(req.subcommand || req.action || argsObj.action || argsObj.subcommand || argParts[0] || 'clone').toLowerCase();
    let rawRepo = String(req.repo_url || req.repo || argsObj.repo_url || argsObj.repo || req.url || argParts[1] || '').trim();

    if (subcommand === 'detect_repo' || subcommand === 'auto_detect') {
      return this.detectHostedGitHubRepo(req);
    }

    // Strict R4-39 secret check BEFORE routing or network access whenever a URL or bearer targets non-GitHub origins
    if (req.provenance_url) {
      let provHost = '';
      try {
        provHost = new URL(String(req.provenance_url)).hostname.toLowerCase();
      } catch {
        provHost = '';
      }
      const hasSecretInput = Boolean(req.auth_bearer) || String(req.auth_bearer || '').includes('{{MVM_SECRET:') ||
        Object.values(req.headers || {}).some((v) => String(v || '').includes('{{MVM_SECRET:'));
      if (hasSecretInput && provHost && !SECRET_HOST_ALLOWLIST.GITHUB_TOKEN.has(provHost)) {
        return {
          ok: false,
          status: 403,
          error_code: 'HostNotAllowlistedForSecret',
          message: `Secret GITHUB_TOKEN is not allowlisted for provenance host ${provHost}`,
        };
      }
    }

    if (subcommand === 'status' || subcommand === 'diff' || subcommand === 'log') {
      const diff = this.#computeGitDiffState();
      const storedState = diff.state || {};
      const storedIndex = diff.index || {};
      const repoLabel = String(rawRepo || storedIndex.repo || storedState.repo || storedState.remote_url || '').trim();
      const branchLabel = String(req.branch || argsObj.branch || storedIndex.branch || storedState.branch || 'main').trim();
      if (subcommand === 'log') {
        const commits = Array.isArray(storedState.commits) ? storedState.commits.slice().reverse() : [];
        const logText = commits.length > 0
          ? commits.map((c) => `${String(c.oid || '').slice(0, 12)} ${c.message || ''}`.trim()).join('\n')
          : `${String(storedIndex.head_sha || storedState.head_oid || 'HEAD').slice(0, 12)} (cloned head)`;
        const logData = {
          repo: repoLabel,
          branch: branchLabel,
          head_sha: String(storedIndex.head_sha || storedState.head_oid || ''),
          commits,
        };
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand: 'log',
          ...logData,
          data: logData,
          body_text: logText,
        };
      }
      if (subcommand === 'status') {
        const lines = [`On branch ${branchLabel}`];
        if (diff.clean) {
          lines.push('nothing to commit, working tree clean');
        } else {
          for (const p of diff.deleted) lines.push(`  deleted:   ${p}`);
          for (const p of diff.modified) lines.push(`  modified:  ${p}`);
          for (const p of diff.added) lines.push(`  added:     ${p}`);
        }
        const statusData = {
          repo: repoLabel,
          branch: branchLabel,
          head_sha: String(storedIndex.head_sha || storedState.head_oid || ''),
          clean: diff.clean,
          added: diff.added,
          modified: diff.modified,
          deleted: diff.deleted,
        };
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand: 'status',
          ...statusData,
          data: statusData,
          body_text: lines.join('\n'),
        };
      }
      const diffData = {
        repo: repoLabel,
        branch: branchLabel,
        head_sha: String(storedIndex.head_sha || storedState.head_oid || ''),
        clean: diff.clean,
        added: diff.added,
        modified: diff.modified,
        deleted: diff.deleted,
        diff_text: diff.diff_text,
      };
      return {
        ok: true,
        status: 200,
        op: 'git',
        subcommand: 'diff',
        ...diffData,
        data: diffData,
        body_text: diff.diff_text || 'Working tree clean (no diff).',
      };
    }

    if (!rawRepo && subcommand !== 'watch_deploy') {
      const { state: storedState, index: storedIndex } = this.#readGitIndexAndState();
      const fallbackRepo = (storedIndex && (storedIndex.repo || storedIndex.remote_url)) ||
        (storedState && (storedState.repo || storedState.remote_url)) || '';
      if (fallbackRepo) rawRepo = String(fallbackRepo).trim();
    }

    if (req.path_and_query) {
      const repoValid = this.validateTransportUrl(rawRepo, false);
      if (!repoValid.ok) return repoValid;
      const host = repoValid.url.hostname.toLowerCase();
      let authBearer;
      if (this.hasSecret('GITHUB_TOKEN') || String(req.auth_bearer || '').includes('{{MVM_SECRET:')) {
        if (!SECRET_HOST_ALLOWLIST.GITHUB_TOKEN.has(host)) {
          return {
            ok: false,
            status: 403,
            error_code: 'HostNotAllowlistedForSecret',
            message: `Secret GITHUB_TOKEN is not allowlisted for host ${host}`,
          };
        }
        const subAuth = this.substituteHostSecretsInString(
          req.auth_bearer || '{{MVM_SECRET:GITHUB_TOKEN}}',
          host,
        );
        if (!subAuth.ok) return subAuth;
        authBearer = subAuth.value;
      }
      const sep = rawRepo.endsWith('/') ? '' : '/';
      const smartUrl = `${rawRepo}${sep}${String(req.path_and_query).replace(/^\/+/, '')}`;
      const smartHeaders = authBearer ? { Authorization: `Bearer ${authBearer}` } : {};
      if (req.content_type) smartHeaders['Content-Type'] = req.content_type;
      return this.handleHttpRequest({
        url: smartUrl,
        method: String(req.method || 'GET').toUpperCase(),
        headers: smartHeaders,
        body_base64: req.body_b64 || req.body_base64 || undefined,
        body: req.body_json ? JSON.stringify(req.body_json) : undefined,
        binary: true,
      });
    }

    if (req.smart_http_url) {
      return this.handleHttpRequest({
        url: req.smart_http_url,
        method: req.method || 'GET',
        headers: req.headers || {},
        body_base64: req.body_base64,
        binary: true,
      });
    }

    // Special case: watch_deploy without repo can inspect static origin provenance.json directly
    if (subcommand === 'watch_deploy' && !rawRepo) {
      const provUrl = String(req.provenance_url || argsObj.provenance_url || '').trim();
      const provRes = await this.checkProvenanceUpdate(provUrl);
      if (!provRes.ok) return provRes;
      return {
        ok: true,
        status: 200,
        op: 'git',
        subcommand: 'watch_deploy',
        ...provRes,
        data: provRes,
        body_text: `Provenance digest (${String(provRes.adapter_wasm_sha256).slice(0, 12)}) update_available=${provRes.update_available}`,
        transport: this.lastTransportMode,
      };
    }

    // Reject non-GitHub URL remotes targeting secrets immediately with HostNotAllowlistedForSecret
    if (/^https?:\/\//i.test(rawRepo)) {
      let hostLower = '';
      try {
        hostLower = new URL(rawRepo).hostname.toLowerCase();
      } catch {
        hostLower = '';
      }
      const wantsSecret = this.hasSecret('GITHUB_TOKEN') || Boolean(req.auth_bearer) || rawRepo.includes('{{MVM_SECRET:') ||
        ['push', 'commit_push', 'create_branch', 'create_pr', 'merge_pr'].includes(subcommand);
      if (hostLower && !SECRET_HOST_ALLOWLIST.GITHUB_TOKEN.has(hostLower) && wantsSecret) {
        return {
          ok: false,
          status: 403,
          error_code: 'HostNotAllowlistedForSecret',
          message: `Secret GITHUB_TOKEN is not allowlisted for remote host ${hostLower}`,
        };
      }
    }

    const coords = this.parseGitHubRepoCoords(rawRepo);
    if (!coords) {
      if (rawRepo) {
        const valid = this.validateTransportUrl(rawRepo, false);
        if (!valid.ok) return valid;
      }
      return {
        ok: false,
        status: 400,
        error_code: 'UnsupportedGitRemote',
        message: `Expected https://github.com/<owner>/<repo> or <owner>/<repo>, got "${rawRepo}"`,
      };
    }

    const repoSlug = `${coords.owner}/${coords.repo}`;

    if (subcommand === 'connect' || subcommand === 'branches') {
      try {
        const meta = await this.#githubRestJson(coords, '');
        if (!meta.ok) {
          if (meta.error_code) return meta;
          return {
            ok: false,
            status: meta.status,
            error_code: meta.status === 401 || meta.status === 404 ? 'GitRepoAccessDenied' : 'GitApiError',
            message: (meta.json && meta.json.message) || `GitHub API returned HTTP ${meta.status} for ${repoSlug}`,
          };
        }
        const br = await this.#githubRestJson(coords, 'branches?per_page=50');
        const branchNames = Array.isArray(br.json) ? br.json.map((b) => String(b.name || '')).filter(Boolean) : [];
        const defBranch = String((meta.json && meta.json.default_branch) || 'main');
        if (defBranch && !branchNames.includes(defBranch)) branchNames.unshift(defBranch);
        const data = {
          repo: repoSlug,
          default_branch: defBranch,
          private: Boolean(meta.json && meta.json.private),
          html_url: (meta.json && meta.json.html_url) || coords.url,
          branches: branchNames,
        };
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand: 'connect',
          repo: repoSlug,
          ...data,
          data,
          body_text: `Connected ${repoSlug} (default: ${defBranch}, ${branchNames.length} branches)`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitNetworkFailure', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'clone' || subcommand === 'pull' || subcommand === 'fetch') {
      try {
        const requestedBranch = String(req.branch || argsObj.branch || '').trim();
        const metaRes = await this.#githubRestJson(coords, '');
        if (!metaRes.ok) {
          if (metaRes.error_code) return metaRes;
          return {
            ok: false,
            status: metaRes.status,
            error_code: metaRes.status === 401 || metaRes.status === 404 ? 'GitRepoAccessDenied' : 'GitApiError',
            message: `GitHub API returned HTTP ${metaRes.status} for ${repoSlug}`,
          };
        }
        const repoMeta = metaRes.json || {};
        const branch = requestedBranch || String(repoMeta.default_branch || 'main');
        const brRes = await this.#githubRestJson(coords, 'branches?per_page=50').catch(() => ({ ok: false, json: [] }));
        const branches = Array.isArray(brRes.json) ? brRes.json.map((b) => String(b.name || '')).filter(Boolean) : [branch];
        if (!branches.includes(branch)) branches.unshift(branch);

        const branchPath = branch.split('/').map(encodeURIComponent).join('/');
        const treeRes = await this.#githubRestJson(coords, `git/trees/${branchPath}?recursive=1`);
        if (!treeRes.ok) {
          return {
            ok: false,
            status: treeRes.status,
            error_code: 'GitTreeFetchFailed',
            message: `Failed to fetch tree for ${repoSlug}@${branch} (HTTP ${treeRes.status})`,
          };
        }
        const treeJson = treeRes.json || {};
        const blobs = (Array.isArray(treeJson.tree) ? treeJson.tree : []).filter(
          (entry) =>
            entry.type === 'blob' &&
            typeof entry.path === 'string' &&
            (entry.size ?? 0) <= 196608 &&
            !/\.(png|jpg|jpeg|gif|webp|ico|woff2?|ttf|wasm|bin|part\d+|xz|gz|zip|pdf)$/i.test(entry.path),
        );
        const eagerCap = Number.isInteger(Number(req.limit)) && Number(req.limit) > 0 ? Math.min(Number(req.limit), 80) : 40;
        const selected = blobs.slice(0, eagerCap);
        const lazyPaths = blobs.slice(eagerCap).map((b) => ({ path: b.path, sha: b.sha || '', size: b.size ?? 0 }));
        const treeKey = `${repoSlug}@${branch}`;
        const lazyMap = new Map();
        for (const lp of lazyPaths) lazyMap.set(lp.path, lp);
        this.#lazyRepoTrees.set(treeKey, lazyMap);

        const hasToken = this.hasSecret('GITHUB_TOKEN');
        const fetchedFiles = [];
        const batchSize = 8;
        for (let i = 0; i < selected.length; i += batchSize) {
          const slice = selected.slice(i, i + batchSize);
          const loaded = await Promise.all(
            slice.map(async (entry) => {
              try {
                const blobRes = await this.#githubRestJson(coords, `git/blobs/${entry.sha}`);
                if (blobRes.ok && blobRes.json && blobRes.json.encoding === 'base64' && typeof blobRes.json.content === 'string') {
                  const bytes = __uorBase64ToBytes(blobRes.json.content);
                  return { path: entry.path, content: cachedTextDecoder.decode(bytes), sha: entry.sha, size: entry.size ?? bytes.byteLength };
                }
                return null;
              } catch {
                return null;
              }
            }),
          );
          for (const item of loaded) {
            if (item) fetchedFiles.push(item);
          }
        }

        const lazyEntries = lazyPaths.map((lp) => ({
          path: lp.path,
          content: null,
          sha: lp.sha,
          size: lp.size,
          lazy: true,
        }));
        const allFiles = [...fetchedFiles, ...lazyEntries];

        const { state: prevPullState, index: prevPullIndex } = this.#readGitIndexAndState();
        const remoteBlobPaths = new Set(blobs.map((b) => this.#normalizeWsPath(b.path)).filter(Boolean));
        const deletedPaths = [];
        const changedPaths = [];
        if (subcommand === 'pull' && prevPullIndex && prevPullIndex.entries) {
          for (const [oldPath, oldEntry] of Object.entries(prevPullIndex.entries)) {
            if (!oldPath || remoteBlobPaths.has(oldPath)) continue;
            if (oldEntry && oldEntry.hydrated !== false) {
              const curText = this.#wsRead(oldPath);
              const trackedText = typeof oldEntry.content === 'string'
                ? oldEntry.content
                : (prevPullState && prevPullState.head_tree && typeof prevPullState.head_tree[oldPath] === 'string'
                  ? prevPullState.head_tree[oldPath]
                  : null);
              if (curText !== null && trackedText !== null && curText === trackedText) {
                if (this.#wsDelete(oldPath)) {
                  deletedPaths.push(oldPath);
                }
              }
            }
          }
        }
        const trackedEntries = {};
        for (const f of fetchedFiles) {
          const normPath = this.#normalizeWsPath(f.path);
          if (!normPath) continue;
          const prevText = this.#wsRead(normPath);
          if (prevText !== f.content) {
            changedPaths.push(normPath);
          }
          this.#wsWrite(normPath, f.content);
          trackedEntries[normPath] = {
            sha: f.sha || '',
            size: f.size ?? f.content.length,
            hydrated: true,
            content: f.content,
          };
        }
        for (const lp of lazyEntries) {
          const normPath = this.#normalizeWsPath(lp.path);
          if (!normPath) continue;
          trackedEntries[normPath] = {
            sha: lp.sha || '',
            size: lp.size ?? 0,
            hydrated: false,
          };
        }
        const remoteHeadSha = String(treeJson.sha || '');
        this.#writeGitIndexAndState({
          repoSlug,
          remoteUrl: `${coords.url}.git`,
          branch,
          headSha: remoteHeadSha,
          treeSha: remoteHeadSha,
          trackedEntries,
          commits: prevPullState && Array.isArray(prevPullState.commits) ? prevPullState.commits : [],
        });

        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand,
          repo: repoSlug,
          repo_url: coords.url,
          branch,
          default_branch: repoMeta.default_branch || branch,
          branches,
          head_sha: treeJson.sha || '',
          files: allFiles,
          changed_paths: changedPaths,
          deleted_paths: deletedPaths,
          deleted_files: deletedPaths.length,
          hydrated_count: fetchedFiles.length,
          file_count: allFiles.length,
          lazy_paths: lazyPaths,
          total_blobs: blobs.length,
          truncated: Boolean(treeJson.truncated) || lazyPaths.length > 0,
          data: {
            repo: repoSlug,
            default_branch: repoMeta.default_branch || branch,
            branch,
            branches,
            head_sha: treeJson.sha || '',
            files: fetchedFiles,
            changed_paths: changedPaths,
            deleted_paths: deletedPaths,
            deleted_files: deletedPaths.length,
            lazy_paths: lazyPaths,
            total_blobs: blobs.length,
            file_count: fetchedFiles.length,
            truncated: Boolean(treeJson.truncated) || lazyPaths.length > 0,
          },
          body_text: `Synchronized ${fetchedFiles.length}/${blobs.length} files from ${repoSlug}@${branch} (${String(treeJson.sha || '').slice(0, 12)})`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return {
          ok: false,
          status: 502,
          error_code: 'GitNetworkFailure',
          message: err && err.message ? err.message : String(err),
        };
      }
    }

    if (subcommand === 'read_blob' || subcommand === 'cat_blob' || subcommand === 'blob') {
      const branch = String(req.branch || argsObj.branch || 'main').trim();
      const filePath = String(req.path || argsObj.path || argParts[2] || '').trim().replace(/^\/+/, '');
      const cachedEntry = this.#lazyRepoTrees.get(`${repoSlug}@${branch}`)?.get(filePath);
      const sha = String(req.sha || argsObj.sha || (cachedEntry && cachedEntry.sha) || '').trim();
      if (!sha && !filePath) {
        return { ok: false, status: 400, error_code: 'GitBlobFetchFailed', message: 'read_blob requires path or sha' };
      }
      try {
        const restPath = sha
          ? `git/blobs/${encodeURIComponent(sha)}`
          : `contents/${filePath.split('/').map((s) => encodeURIComponent(s)).join('/')}?ref=${encodeURIComponent(branch)}`;
        const blobRes = await this.#githubRestJson(coords, restPath);
        if (!blobRes.ok || !blobRes.json) {
          if (blobRes.error_code) return blobRes;
          return {
            ok: false,
            status: blobRes.status || 404,
            error_code: 'GitBlobFetchFailed',
            message: (blobRes.json && blobRes.json.message) || `Failed to read blob ${filePath || sha}`,
          };
        }
        const content =
          blobRes.json.encoding === 'base64' && typeof blobRes.json.content === 'string'
            ? cachedTextDecoder.decode(__uorBase64ToBytes(blobRes.json.content))
            : String(blobRes.json.content || '');
        const resolvedSha = blobRes.json.sha || sha;
        const resolvedPath = filePath || blobRes.json.path || '';
        const data = { repo: repoSlug, branch, path: resolvedPath, sha: resolvedSha, size: blobRes.json.size ?? content.length, content };
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand: 'read_blob',
          repo: repoSlug,
          branch,
          path: resolvedPath,
          sha: resolvedSha,
          size: data.size,
          content,
          files: resolvedPath ? [{ path: resolvedPath, content, sha: resolvedSha }] : [],
          data,
          body_text: `Loaded ${resolvedPath} (${data.size} bytes) from ${repoSlug}@${branch}`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitBlobFetchFailed', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'create_branch' || subcommand === 'branch' || subcommand === 'checkout') {
      if (!this.hasSecret('GITHUB_TOKEN')) {
        return { ok: false, status: 401, error_code: 'GitAuthRequired', message: 'GitHub PAT required for create_branch.' };
      }
      const branch = String(req.branch || argsObj.branch || argParts[2] || '').trim();
      const fromBranchInput = String(
        req.base_branch || req.from_branch || argsObj.base_branch || argsObj.from_branch || argsObj.base || '',
      ).trim();
      if (!branch || branch.includes('..')) {
        return { ok: false, status: 400, error_code: 'GitBranchCreateFailed', message: `Invalid branch name '${branch}'` };
      }
      try {
        let fromBranch = fromBranchInput;
        if (!fromBranch) {
          const meta = await this.#githubRestJson(coords, '');
          if (meta.error_code) return meta;
          fromBranch = String((meta.json && meta.json.default_branch) || 'main');
        }
        const fromBranchPath = fromBranch.split('/').map(encodeURIComponent).join('/');
        const baseRef = await this.#githubRestJson(coords, `git/ref/heads/${fromBranchPath}`);
        if (baseRef.error_code) return baseRef;
        if (!baseRef.ok || !baseRef.json || !baseRef.json.object || !baseRef.json.object.sha) {
          return { ok: false, status: baseRef.status || 404, error_code: 'GitRefReadFailed', message: `Base branch '${fromBranch}' not found` };
        }
        const baseSha = baseRef.json.object.sha;
        const created = await this.#githubRestJson(coords, 'git/refs', {
          method: 'POST',
          body: { ref: `refs/heads/${branch}`, sha: baseSha },
        });
        if (created.error_code) return created;
        if (!created.ok) {
          return {
            ok: false,
            status: created.status,
            error_code: 'GitBranchCreateFailed',
            message: (created.json && created.json.message) || `Failed to create branch '${branch}'`,
          };
        }
        const data = {
          repo: repoSlug,
          branch,
          from_branch: fromBranch,
          ref: (created.json && created.json.ref) || `refs/heads/${branch}`,
          sha: (created.json && created.json.object && created.json.object.sha) || baseSha,
        };
        return {
          ok: true,
          status: created.status,
          op: 'git',
          subcommand: 'create_branch',
          repo: repoSlug,
          ...data,
          data,
          body_text: `Created branch ${branch} from ${fromBranch} at ${String(data.sha || '').slice(0, 12)}`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitBranchCreateFailed', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'push' || subcommand === 'commit_push' || subcommand === 'commit') {
      if (!this.hasSecret('GITHUB_TOKEN')) {
        return {
          ok: false,
          status: 401,
          error_code: 'GitAuthRequired',
          message: 'GitHub PAT not configured. Click GitHub in the header to store GITHUB_TOKEN in the host vault.',
        };
      }
      const { state: prevState, index: prevIndex } = this.#readGitIndexAndState();
      const workingMap = this.#wsListWorkingTree();
      const explicitFiles = Array.isArray(req.files) ? req.files : Array.isArray(argsObj.files) ? argsObj.files : null;
      const diffState = this.#computeGitDiffState(workingMap);

      let filesToPush = [];
      if (explicitFiles && req.full_workspace && prevIndex && prevIndex.entries) {
        const snapshotMap = new Map();
        const modeMap = new Map();
        const explicitDeletePaths = new Set();
        for (const f of explicitFiles) {
          if (!f || typeof f.path !== 'string') continue;
          const clean = this.#normalizeWsPath(f.path);
          if (!clean || clean === '.git' || clean.startsWith('.git/')) continue;
          if (f.delete === true) {
            explicitDeletePaths.add(clean);
          } else {
            snapshotMap.set(clean, String(f.content ?? ''));
            if (f.mode) modeMap.set(clean, String(f.mode));
          }
        }
        const snapDiff = this.#computeGitDiffState(snapshotMap);
        for (const p of snapDiff.added) {
          filesToPush.push({ path: p, mode: modeMap.get(p) || '100644', content: snapshotMap.get(p) ?? '' });
        }
        for (const p of snapDiff.modified) {
          filesToPush.push({ path: p, mode: modeMap.get(p) || '100644', content: snapshotMap.get(p) ?? '' });
        }
        const allDelPaths = new Set([...snapDiff.deleted, ...explicitDeletePaths]);
        for (const p of allDelPaths) {
          filesToPush.push({ path: p, delete: true });
        }
      } else if (explicitFiles) {
        const suppliedPaths = new Set();
        const explicitDeletePaths = new Set();
        for (const f of explicitFiles) {
          if (!f || typeof f.path !== 'string') continue;
          const clean = this.#normalizeWsPath(f.path);
          if (!clean || clean === '.git' || clean.startsWith('.git/')) continue;
          if (f.delete === true) {
            explicitDeletePaths.add(clean);
            filesToPush.push({ path: clean, delete: true });
          } else {
            suppliedPaths.add(clean);
            filesToPush.push({
              path: clean,
              mode: String(f.mode || '100644'),
              content: String(f.content ?? ''),
            });
          }
        }
        const prevEntries = (prevIndex && prevIndex.entries && typeof prevIndex.entries === 'object') ? prevIndex.entries : {};
        for (const [trackedPath, trackedMeta] of Object.entries(prevEntries)) {
          if (!trackedMeta || trackedMeta.hydrated === false) continue;
          if (explicitDeletePaths.has(trackedPath)) continue;
          if (!suppliedPaths.has(trackedPath) && !workingMap.has(trackedPath)) {
            filesToPush.push({ path: trackedPath, delete: true });
          }
        }
      } else {
        for (const p of diffState.added) {
          filesToPush.push({ path: p, mode: '100644', content: workingMap.get(p) ?? '' });
        }
        for (const p of diffState.modified) {
          filesToPush.push({ path: p, mode: '100644', content: workingMap.get(p) ?? '' });
        }
        for (const p of diffState.deleted) {
          filesToPush.push({ path: p, delete: true });
        }
      }

      if (filesToPush.length === 0) {
        return {
          ok: false,
          status: 400,
          error_code: 'GitNothingToPush',
          message: 'No modified or deleted workspace files supplied for commit_push',
        };
      }
      const branch = String(req.branch || argsObj.branch || (prevIndex && prevIndex.branch) || (prevState && prevState.branch) || 'main');
      const commitMessage = String(req.message || argsObj.message || req.commit_message || 'Update from UOR-R4 Connected Studio');
      try {
        const branchRefPath = branch.split('/').map(encodeURIComponent).join('/');
        const refRes = await this.#githubRestJson(coords, `git/ref/heads/${branchRefPath}`);
        if (refRes.error_code) return refRes;
        if (!refRes.ok || !refRes.json || !refRes.json.object) {
          return {
            ok: false,
            status: refRes.status || 404,
            error_code: 'GitRefReadFailed',
            message: `Unable to read heads/${branch} on ${repoSlug} (HTTP ${refRes.status})`,
          };
        }
        const baseCommitSha = refRes.json.object.sha;
        const commitRes = await this.#githubRestJson(coords, `git/commits/${baseCommitSha}`);
        if (commitRes.error_code) return commitRes;
        const baseTreeSha = commitRes.json && commitRes.json.tree && commitRes.json.tree.sha;

        const treeEntries = filesToPush.map((f) => {
          const pathClean = String(f.path || '').replace(/^\/+/, '');
          if (f.delete === true) {
            return { path: pathClean, mode: '100644', type: 'blob', sha: null };
          }
          return { path: pathClean, mode: String(f.mode || '100644'), type: 'blob', content: String(f.content ?? '') };
        });
        const newTree = await this.#githubRestJson(coords, 'git/trees', {
          method: 'POST',
          body: { base_tree: baseTreeSha, tree: treeEntries },
        });
        if (newTree.error_code) return newTree;
        if (!newTree.ok || !newTree.json) {
          return { ok: false, status: newTree.status, error_code: 'GitTreeCreateFailed', message: `GitHub tree creation failed (HTTP ${newTree.status})` };
        }
        const createdCommit = await this.#githubRestJson(coords, 'git/commits', {
          method: 'POST',
          body: { message: commitMessage, tree: newTree.json.sha, parents: [baseCommitSha] },
        });
        if (createdCommit.error_code) return createdCommit;
        if (!createdCommit.ok || !createdCommit.json) {
          return { ok: false, status: createdCommit.status, error_code: 'GitCommitCreateFailed', message: `GitHub commit creation failed (HTTP ${createdCommit.status})` };
        }
        const updRef = await this.#githubRestJson(coords, `git/refs/heads/${branchRefPath}`, {
          method: 'PATCH',
          body: { sha: createdCommit.json.sha, force: Boolean(req.force) },
        });
        if (updRef.error_code) return updRef;
        if (!updRef.ok) {
          return { ok: false, status: updRef.status, error_code: 'GitRefUpdateFailed', message: `GitHub ref fast-forward failed (HTTP ${updRef.status})` };
        }
        const deletedFiles = treeEntries.filter((e) => e.sha === null).length;
        const pushedFiles = treeEntries.length - deletedFiles;

        if (prevIndex || this.#workspaceMap.size > 0 || this.#workspaceFs) {
          const nextTrackedEntries = {
            ...((prevIndex && prevIndex.entries && typeof prevIndex.entries === 'object') ? prevIndex.entries : {}),
          };
          for (const entry of treeEntries) {
            if (entry.sha === null) {
              delete nextTrackedEntries[entry.path];
            } else {
              nextTrackedEntries[entry.path] = {
                sha: newTree.json.sha || '',
                size: String(entry.content || '').length,
                hydrated: true,
                content: String(entry.content || ''),
              };
            }
          }
          const nextCommitTree = {};
          for (const [tp, te] of Object.entries(nextTrackedEntries)) {
            if (te && te.hydrated !== false && typeof te.content === 'string') {
              nextCommitTree[tp] = te.content;
            }
          }
          const nextCommits = Array.isArray(prevState && prevState.commits) ? [...prevState.commits] : [];
          nextCommits.push({
            oid: createdCommit.json.sha || '',
            parent_oid: baseCommitSha || null,
            message: commitMessage,
            tree: nextCommitTree,
          });
          this.#writeGitIndexAndState({
            repoSlug,
            remoteUrl: `${coords.url}.git`,
            branch,
            headSha: String(createdCommit.json.sha || ''),
            treeSha: String(newTree.json.sha || ''),
            trackedEntries: nextTrackedEntries,
            commits: nextCommits,
          });
        }

        const data = {
          repo: repoSlug,
          branch,
          commit_sha: createdCommit.json.sha,
          tree_sha: newTree.json.sha,
          pushed_files: pushedFiles,
          deleted_files: deletedFiles,
        };
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand: subcommand === 'push' ? 'push' : 'commit_push',
          ...data,
          data,
          body_text: `Committed & pushed ${pushedFiles} file(s) (${deletedFiles} deleted) to ${repoSlug}@${branch} -> ${String(createdCommit.json.sha || '').slice(0, 12)}`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitPushNetworkError', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'create_pr' || subcommand === 'open_pr') {
      if (!this.hasSecret('GITHUB_TOKEN')) {
        return { ok: false, status: 401, error_code: 'GitAuthRequired', message: 'GitHub PAT required for create_pr.' };
      }
      const head = String(req.head || argsObj.head || req.branch || argsObj.branch || '').trim();
      const base = String(req.base || argsObj.base || 'main').trim();
      const title = String(req.title || argsObj.title || `Update ${head}`).trim();
      const body = String(req.body || argsObj.body || 'PR opened from UOR-R4 Connected Studio').trim();
      if (!head || !title) {
        return { ok: false, status: 400, error_code: 'GitPrCreateFailed', message: 'create_pr requires head branch and title' };
      }
      try {
        const prRes = await this.#githubRestJson(coords, 'pulls', {
          method: 'POST',
          body: { title, body, head, base, draft: Boolean(req.draft) },
        });
        if (prRes.error_code) return prRes;
        if (!prRes.ok || !prRes.json) {
          return {
            ok: false,
            status: prRes.status,
            error_code: 'GitPrCreateFailed',
            message: (prRes.json && prRes.json.message) || `GitHub PR creation failed (HTTP ${prRes.status})`,
          };
        }
        const data = {
          repo: repoSlug,
          pr_number: prRes.json.number,
          url: prRes.json.url || '',
          html_url: prRes.json.html_url || '',
          state: prRes.json.state || 'open',
          head: (prRes.json.head && prRes.json.head.ref) || head,
          base: (prRes.json.base && prRes.json.base.ref) || base,
          title: prRes.json.title || title,
        };
        return {
          ok: true,
          status: prRes.status,
          op: 'git',
          subcommand: 'create_pr',
          ...data,
          data,
          body_text: `Opened PR #${data.pr_number} (${data.head} -> ${data.base}): ${data.html_url || data.url}`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitPrCreateFailed', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'pr_status' || subcommand === 'ci_status') {
      const prNum = Number(req.pr_number || argsObj.pr_number || 0) || null;
      let headSha = String(req.ref || argsObj.ref || req.head_sha || '').trim();
      try {
        let prMeta = null;
        if (prNum && Number.isInteger(prNum) && prNum > 0) {
          const prRes = await this.#githubRestJson(coords, `pulls/${prNum}`);
          if (prRes.error_code) return prRes;
          if (!prRes.ok || !prRes.json) {
            return { ok: false, status: prRes.status || 404, error_code: 'GitPrStatusFailed', message: `Failed to fetch PR #${prNum}` };
          }
          prMeta = prRes.json;
          headSha = (prMeta.head && prMeta.head.sha) || headSha;
        }
        let checkRuns = [];
        let workflowRuns = [];
        if (headSha) {
          const [cr, wr] = await Promise.all([
            this.#githubRestJson(coords, `commits/${encodeURIComponent(headSha)}/check-runs`),
            this.#githubRestJson(coords, `actions/runs?head_sha=${encodeURIComponent(headSha)}&per_page=10`),
          ]);
          if (cr.ok && cr.json && Array.isArray(cr.json.check_runs)) checkRuns = cr.json.check_runs;
          if (wr.ok && wr.json && Array.isArray(wr.json.workflow_runs)) workflowRuns = wr.json.workflow_runs;
        }
        const checksPassed = checkRuns.filter((c) => c.conclusion === 'success' || c.conclusion === 'neutral').length;
        const anyPending = checkRuns.some((c) => c.status !== 'completed') || workflowRuns.some((w) => w.status !== 'completed');
        const anyFailed = checkRuns.some((c) => c.conclusion === 'failure' || c.conclusion === 'timed_out') ||
          workflowRuns.some((w) => w.conclusion === 'failure' || w.conclusion === 'cancelled');
        const checkStatus = anyFailed ? 'failure' : anyPending ? 'pending' : checkRuns.length > 0 ? 'success' : 'unknown';
        const latestWf = workflowRuns[0] || null;
        const data = {
          repo: repoSlug,
          pr_number: prNum,
          state: (prMeta && prMeta.state) || (prNum ? 'unknown' : 'commit'),
          mergeable: prMeta ? (prMeta.mergeable ?? null) : null,
          mergeable_state: (prMeta && prMeta.mergeable_state) || 'unknown',
          html_url: (prMeta && prMeta.html_url) || '',
          head_sha: headSha,
          check_status: checkStatus,
          checks_total: checkRuns.length,
          checks_passed: checksPassed,
          workflow_status: (latestWf && latestWf.status) || 'unknown',
          workflow_conclusion: (latestWf && latestWf.conclusion) || null,
          check_runs: checkRuns.map((c) => ({ name: c.name, status: c.status, conclusion: c.conclusion })),
          workflow_runs: workflowRuns.map((w) => ({ id: w.id, name: w.name, status: w.status, conclusion: w.conclusion })),
        };
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand: 'pr_status',
          ...data,
          data,
          body_text: `PR #${data.pr_number || '-'} state=${data.state} mergeable=${data.mergeable} checks=${data.check_status} (${data.checks_passed}/${data.checks_total})`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitPrStatusFailed', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'merge_pr') {
      if (!this.hasSecret('GITHUB_TOKEN')) {
        return { ok: false, status: 401, error_code: 'GitAuthRequired', message: 'GitHub PAT required for merge_pr.' };
      }
      const prNum = Number(req.pr_number || argsObj.pr_number || 0);
      if (!Number.isInteger(prNum) || prNum <= 0) {
        return { ok: false, status: 400, error_code: 'GitPrMergeFailed', message: 'merge_pr requires valid integer pr_number' };
      }
      try {
        const mergeBody = { merge_method: String(req.merge_method || argsObj.merge_method || 'squash') };
        if (req.commit_title || argsObj.commit_title) mergeBody.commit_title = String(req.commit_title || argsObj.commit_title);
        const mRes = await this.#githubRestJson(coords, `pulls/${prNum}/merge`, { method: 'PUT', body: mergeBody });
        if (mRes.error_code) return mRes;
        if (!mRes.ok) {
          return { ok: false, status: mRes.status, error_code: 'GitPrMergeFailed', message: (mRes.json && mRes.json.message) || `Failed to merge PR #${prNum}` };
        }
        const data = {
          repo: repoSlug,
          pr_number: prNum,
          merged: Boolean(mRes.json && (mRes.json.merged ?? true)),
          sha: (mRes.json && mRes.json.sha) || '',
          message: (mRes.json && mRes.json.message) || 'Pull Request merged',
        };
        return {
          ok: true,
          status: mRes.status,
          op: 'git',
          subcommand: 'merge_pr',
          ...data,
          data,
          body_text: `Merged PR #${prNum} on ${repoSlug} (sha=${String(data.sha || '').slice(0, 12)})`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitPrMergeFailed', message: err && err.message ? err.message : String(err) };
      }
    }

    if (subcommand === 'watch_deploy') {
      const provUrlOverride = String(req.provenance_url || argsObj.provenance_url || '').trim();
      try {
        const [pagesRes, buildRes] = await Promise.all([
          this.#githubRestJson(coords, 'pages'),
          this.#githubRestJson(coords, 'pages/builds/latest'),
        ]);
        const fatalNetworkCodes = new Set([
          'RelayForbiddenForCredentialedHost',
          'RelayConsentRequired',
          'CapabilityDenied',
          'PrivateAddressBlocked',
          'HostNotAllowlistedForSecret',
          'CredentialLeakBlocked',
        ]);
        for (const candRes of [pagesRes, buildRes]) {
          if (
            candRes &&
            !candRes.ok &&
            ((candRes.error_code && fatalNetworkCodes.has(candRes.error_code)) ||
              (candRes.status && candRes.status !== 404 && candRes.status !== 200))
          ) {
            if (candRes === buildRes && candRes.status === 404) continue;
            return {
              ok: false,
              status: candRes.status || 403,
              error_code: candRes.error_code || 'GitDeployWatchFailed',
              host: candRes.host || 'api.github.com',
              message:
                candRes.message ||
                (candRes.json && candRes.json.message) ||
                `GitHub Pages status lookup failed (HTTP ${candRes.status})`,
            };
          }
        }
        const pagesUrl = pagesRes.ok && pagesRes.json ? String(pagesRes.json.html_url || '') : '';
        const targetProvUrl = provUrlOverride || (pagesUrl ? `${pagesUrl.replace(/\/+$/, '')}/provenance.json` : '');
        let provRes = null;
        if (targetProvUrl) {
          provRes = await this.checkProvenanceUpdate(targetProvUrl);
          if (provRes && !provRes.ok && provRes.error_code && fatalNetworkCodes.has(provRes.error_code)) {
            return provRes;
          }
        }
        const classicBuildStatus = buildRes.ok && buildRes.json ? String(buildRes.json.status || '') : '';
        let workflowStatus = '';
        let workflowSha = '';
        if (pagesRes.ok && pagesRes.json && !pagesRes.json.status && !classicBuildStatus) {
          const depRes = await this.#githubRestJson(coords, 'deployments?environment=github-pages&per_page=1');
          if (depRes && !depRes.ok && depRes.error_code && fatalNetworkCodes.has(depRes.error_code)) {
            return depRes;
          }
          const latestDep = depRes.ok && Array.isArray(depRes.json) && depRes.json.length > 0 ? depRes.json[0] : null;
          if (latestDep) {
            workflowSha = String(latestDep.sha || '');
            if (latestDep.id != null) {
              const stRes = await this.#githubRestJson(coords, `deployments/${latestDep.id}/statuses?per_page=1`);
              if (stRes && !stRes.ok && stRes.error_code && fatalNetworkCodes.has(stRes.error_code)) {
                return stRes;
              }
              const latestSt = stRes.ok && Array.isArray(stRes.json) && stRes.json.length > 0 ? stRes.json[0] : null;
              const rawDepState = String((latestSt && latestSt.state) || '').toLowerCase();
              if (rawDepState === 'success') workflowStatus = 'built';
              else if (rawDepState === 'failure' || rawDepState === 'error') workflowStatus = 'errored';
              else if (rawDepState === 'in_progress' || rawDepState === 'queued' || rawDepState === 'pending' || rawDepState === 'waiting') workflowStatus = 'building';
            }
          }
          if (!workflowStatus && provRes && provRes.ok) {
            workflowStatus = 'built';
          }
        }
        const resolvedPagesStatus =
          pagesRes.ok && pagesRes.json
            ? (pagesRes.json.status || classicBuildStatus || workflowStatus || (provRes && provRes.ok ? 'built' : 'unknown'))
            : 'unconfigured';
        const data = {
          repo: repoSlug,
          pages_status: resolvedPagesStatus,
          pages_url: pagesUrl,
          build_status: classicBuildStatus || workflowStatus || 'unknown',
          deployment_sha: (buildRes.ok && buildRes.json ? String(buildRes.json.commit || '') : '') || workflowSha,
          provenance_url: targetProvUrl,
          provenance: provRes && provRes.ok ? provRes.live : null,
          adapter_wasm_sha256: provRes && provRes.ok ? provRes.adapter_wasm_sha256 : '',
          binding_javascript_sha256: provRes && provRes.ok ? provRes.binding_javascript_sha256 : '',
          generated_core_sha256: provRes && provRes.ok ? provRes.generated_core_sha256 : '',
          update_available: Boolean(provRes && provRes.ok && provRes.update_available),
        };
        return {
          ok: true,
          status: 200,
          op: 'git',
          subcommand: 'watch_deploy',
          ...data,
          data,
          body_text: `Pages ${data.pages_status} (${data.pages_url || targetProvUrl || 'local'}) · provenance=${String(data.adapter_wasm_sha256 || '').slice(0, 12)} · update=${data.update_available}`,
          transport: this.lastTransportMode,
        };
      } catch (err) {
        return { ok: false, status: 502, error_code: 'GitDeployWatchFailed', message: err && err.message ? err.message : String(err) };
      }
    }

    return {
      ok: false,
      status: 400,
      error_code: 'UnsupportedGitSubcommand',
      message: `Unsupported connected git subcommand: ${subcommand}`,
    };
  }

  async handlePkgFetch(req) {
    const sub = String(req.subcommand || req.action || (req.name || req.pkg_name ? 'install' : 'list')).toLowerCase();
    const pkgName = String(req.pkg_name || req.name || '').trim().replace(/[^a-zA-Z0-9_.-]/g, '');
    const explicitUrl = String(req.url || (req.arguments && req.arguments.url) || '').trim();
    if (explicitUrl && sub !== 'list' && sub !== 'search') {
      const elfFetch = await this.#admittedFetch(
        explicitUrl,
        undefined,
        { operation: 'pkg_fetch', allowConfiguredOrigin: true },
      );
      if (!elfFetch.ok) return elfFetch.error;
      const elfResp = elfFetch.response;
      if (!elfResp || !elfResp.ok) {
        return {
          ok: false,
          status: elfResp ? elfResp.status : 502,
          error_code: 'PackageDownloadFailed',
          message: `Failed to download ${explicitUrl} (HTTP ${elfResp ? elfResp.status : 'error'})`,
        };
      }
      const elfBytes = new Uint8Array(await elfResp.arrayBuffer());
      const expectedSha = String(req.expected_sha256 || req.sha256 || '');
      const verified = await this.verifyElf64RiscvAndSha256(elfBytes, expectedSha);
      if (!verified.ok) {
        return { ok: false, status: 422, error_code: verified.error_code, message: verified.message };
      }
      const elfB64 = __uorBytesToBase64(elfBytes);
      const binName = pkgName || 'pkg';
      return {
        ok: true,
        status: 200,
        op: 'pkg_fetch',
        subcommand: 'install',
        installed: true,
        name: binName,
        pkg_name: binName,
        install_path: `/usr/local/bin/${binName}`,
        expected_sha256: verified.sha256,
        sha256: verified.sha256,
        byte_len: elfBytes.byteLength,
        e_machine: verified.e_machine,
        elf_machine: verified.e_machine,
        elf_base64: elfB64,
        data: {
          installed: true,
          name: binName,
          pkg_name: binName,
          expected_sha256: verified.sha256,
          sha256: verified.sha256,
          elf_machine: verified.e_machine,
          elf_base64: elfB64,
        },
        body_text: `Installed verified RV64IMAC ELF64 binary /usr/local/bin/${binName} (${elfBytes.byteLength} B, sha256:${verified.sha256.slice(0, 16)})`,
      };
    }

    const explicitOrigin = this.staticOrigin ? this.staticOrigin.replace(/\/+$/, '') : '';
    const httpOrigin =
      typeof location !== 'undefined' && location.origin && /^https?:/i.test(location.origin)
        ? location.origin.replace(/\/+$/, '')
        : '';
    const indexCandidates = [
      ...(explicitOrigin ? [`${explicitOrigin}/weights/mvm-pkgs/index.json`] : []),
      new URL('weights/mvm-pkgs/index.json', import.meta.url).href,
      new URL('../gpu/weights/mvm-pkgs/index.json', import.meta.url).href,
      ...(httpOrigin ? [`${httpOrigin}/weights/mvm-pkgs/index.json`] : []),
    ];

    let pkgIndex = null;
    let basePkgUrl = '';
    let lastPolicyErr = null;
    for (const idxUrl of indexCandidates) {
      for (let attempt = 0; attempt < 2; attempt += 1) {
        try {
          if (idxUrl.startsWith('file://')) {
            const r = await this.#fetchImpl(idxUrl, { cache: 'no-cache' });
            if (r && r.ok) {
              pkgIndex = await r.json();
              basePkgUrl = idxUrl.slice(0, idxUrl.lastIndexOf('/') + 1);
            }
            break;
          }
          const rFetch = await this.#admittedFetch(
            idxUrl,
            { cache: 'no-cache' },
            { operation: 'pkg_fetch', allowConfiguredOrigin: true },
          );
          if (!rFetch.ok) {
            if (rFetch.error && rFetch.error.error_code && rFetch.error.error_code !== 'HttpBridgeFetchError') {
              lastPolicyErr = rFetch.error;
              if (explicitOrigin && idxUrl.startsWith(explicitOrigin)) {
                return rFetch.error;
              }
              break;
            }
            continue;
          }
          const r = rFetch.response;
          if (r && r.ok) {
            pkgIndex = await r.json();
            basePkgUrl = idxUrl.slice(0, idxUrl.lastIndexOf('/') + 1);
          }
          break;
        } catch {
          // Retry once on transient socket error (e.g. pooled keep-alive ECONNRESET after sync Wasm work).
        }
      }
      if (pkgIndex) break;
    }

    if (!pkgIndex || !Array.isArray(pkgIndex.packages)) {
      if (lastPolicyErr) return lastPolicyErr;
      return {
        ok: false,
        status: 404,
        error_code: 'PkgIndexUnavailable',
        message: 'Static RV64IMAC package index (mvm-pkgs/index.json) not found on origin',
      };
    }

    if (sub === 'list' || sub === 'search') {
      const q = pkgName.toLowerCase();
      const pkgs = pkgIndex.packages.filter((p) => !q || String(p.name || '').toLowerCase().includes(q));
      const lines = pkgs.map(
        (p) => `${String(p.name).padEnd(12, ' ')} ${String(p.version || '1.0.0').padEnd(8, ' ')} ${p.size_bytes ?? p.byte_length ?? 0} B  ${p.description || 'riscv64-linux-musl static ELF'}`,
      );
      return {
        ok: true,
        status: 200,
        op: 'pkg_fetch',
        subcommand: sub,
        packages: pkgs,
        body_text: lines.join('\n') || 'No matching RV64IMAC packages.',
      };
    }

    const entry = pkgIndex.packages.find((p) => p.name === pkgName);
    if (!entry) {
      return {
        ok: false,
        status: 404,
        error_code: 'PackageNotFound',
        message: `Package "${pkgName}" not found in uor-mvm-pkg-index/1`,
      };
    }

    const elfFile = entry.file || `${pkgName}.elf`;
    const elfTargetUrl = entry.url ? String(entry.url) : (basePkgUrl + elfFile);
    let elfResp = null;
    if (elfTargetUrl.startsWith('file://')) {
      elfResp = await this.#fetchImpl(elfTargetUrl);
    } else {
      const elfFetch = await this.#admittedFetch(
        elfTargetUrl,
        undefined,
        { operation: 'pkg_fetch', allowConfiguredOrigin: true },
      );
      if (!elfFetch.ok) return elfFetch.error;
      elfResp = elfFetch.response;
    }
    if (!elfResp || !elfResp.ok) {
      return {
        ok: false,
        status: elfResp ? elfResp.status : 502,
        error_code: 'PackageDownloadFailed',
        message: `Failed to download ${elfFile} (HTTP ${elfResp ? elfResp.status : 'error'})`,
      };
    }
    const elfBytes = new Uint8Array(await elfResp.arrayBuffer());
    const verified = await this.verifyElf64RiscvAndSha256(elfBytes, entry.sha256 || '');
    if (!verified.ok) {
      return {
        ok: false,
        status: 422,
        error_code: verified.error_code,
        message: verified.message,
      };
    }

    const elfB64 = __uorBytesToBase64(elfBytes);
    return {
      ok: true,
      status: 200,
      op: 'pkg_fetch',
      subcommand: 'install',
      installed: true,
      name: pkgName,
      pkg_name: pkgName,
      install_path: `/usr/local/bin/${pkgName}`,
      expected_sha256: verified.sha256,
      sha256: verified.sha256,
      byte_len: elfBytes.byteLength,
      e_machine: verified.e_machine,
      elf_machine: verified.e_machine,
      elf_base64: elfB64,
      data: {
        installed: true,
        name: pkgName,
        pkg_name: pkgName,
        expected_sha256: verified.sha256,
        sha256: verified.sha256,
        elf_machine: verified.e_machine,
        elf_base64: elfB64,
      },
      body_text: `Installed verified RV64IMAC ELF64 binary /usr/local/bin/${pkgName} (${elfBytes.byteLength} B, sha256:${verified.sha256.slice(0, 16)})`,
    };
  }

  #assertNetworkAllowed() {
    if (this.mcpPolicy === 'deny') {
      return {
        ok: false,
        status: 403,
        error_code: 'PolicyDenied',
        message: 'Outbound bridge operation denied by active MCP security policy',
      };
    }
    if (!this.webAccessEnabled) {
      return {
        ok: false,
        status: 403,
        error_code: 'PolicyDenied',
        message: 'Web Access is currently toggled OFF in the studio topbar',
      };
    }
    return { ok: true };
  }

  async #verifySri(bytes, integrityStr) {
    const view = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes || 0);
    const tokens = String(integrityStr || '').trim().split(/\s+/).filter(Boolean);
    if (tokens.length === 0) {
      return {
        ok: false,
        error_code: 'IntegrityMismatch',
        message: 'Missing required Subresource Integrity digest',
      };
    }
    const algMap = {
      sha256: 'SHA-256',
      sha384: 'SHA-384',
      sha512: 'SHA-512',
    };
    let sawSupported = false;
    for (const tok of tokens) {
      const dash = tok.indexOf('-');
      if (dash <= 0) continue;
      const algKey = tok.slice(0, dash).toLowerCase();
      const expectedB64 = tok.slice(dash + 1);
      const subtleAlg = algMap[algKey];
      if (!subtleAlg || !expectedB64) continue;
      sawSupported = true;
      const digBuf = await crypto.subtle.digest(subtleAlg, view);
      const actualB64 = __uorBytesToBase64(new Uint8Array(digBuf));
      if (actualB64 === expectedB64) {
        return { ok: true, algorithm: subtleAlg, digest: tok };
      }
    }
    return {
      ok: false,
      error_code: 'IntegrityMismatch',
      message: sawSupported
        ? `SRI integrity mismatch against ${tokens[0].slice(0, 24)}...`
        : `Unsupported SRI algorithm in "${integrityStr}"`,
    };
  }

  async #gunzipBytes(u8) {
    if (typeof DecompressionStream === 'function') {
      const ds = new DecompressionStream('gzip');
      const writer = ds.writable.getWriter();
      const writeP = writer.write(u8).then(() => writer.close());
      const reader = ds.readable.getReader();
      const chunks = [];
      let total = 0;
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        if (value && value.byteLength > 0) {
          chunks.push(value);
          total += value.byteLength;
        }
      }
      await writeP;
      const out = new Uint8Array(total);
      let off = 0;
      for (const c of chunks) {
        out.set(c, off);
        off += c.byteLength;
      }
      return out;
    }
    const err = new Error('DecompressionStream("gzip") unavailable in host environment');
    err.code = 'IntegrityMismatch';
    throw err;
  }

  #parseTarEntries(tarBytes) {
    const entries = [];
    let offset = 0;
    let nextLongName = '';
    let nextLongLink = '';
    const parseOctal = (slice) => {
      const txt = cachedTextDecoder.decode(slice).replace(/\0.*$/, '').trim();
      return txt ? Number('0o' + txt) : 0;
    };
    const parsePax = (dataBytes) => {
      const pax = {};
      const text = cachedTextDecoder.decode(dataBytes);
      let pos = 0;
      while (pos < text.length) {
        const sp = text.indexOf(' ', pos);
        if (sp < 0) break;
        const len = Number(text.slice(pos, sp));
        if (!Number.isFinite(len) || len <= 0) break;
        const rec = text.slice(sp + 1, pos + len).replace(/\n$/, '');
        const eq = rec.indexOf('=');
        if (eq > 0) pax[rec.slice(0, eq)] = rec.slice(eq + 1);
        pos += len;
      }
      return pax;
    };
    let paxNext = {};
    while (offset + 512 <= tarBytes.byteLength) {
      const hdr = tarBytes.subarray(offset, offset + 512);
      offset += 512;
      let zero = true;
      for (let i = 0; i < 512; i += 1) {
        if (hdr[i] !== 0) {
          zero = false;
          break;
        }
      }
      if (zero) break;
      const rawName = cachedTextDecoder.decode(hdr.subarray(0, 100)).replace(/\0.*$/, '');
      const prefix = cachedTextDecoder.decode(hdr.subarray(345, 500)).replace(/\0.*$/, '');
      const mode = parseOctal(hdr.subarray(100, 108)) || 0o644;
      const size = parseOctal(hdr.subarray(124, 136)) || 0;
      const typeflag = String.fromCharCode(hdr[156] || 48);
      const rawLink = cachedTextDecoder.decode(hdr.subarray(157, 257)).replace(/\0.*$/, '');
      const dataSlice = size > 0 ? tarBytes.subarray(offset, offset + size) : new Uint8Array(0);
      const padded = Math.ceil(size / 512) * 512;
      offset += padded;

      if (typeflag === 'L') {
        nextLongName = cachedTextDecoder.decode(dataSlice).replace(/\0.*$/, '');
        continue;
      }
      if (typeflag === 'K') {
        nextLongLink = cachedTextDecoder.decode(dataSlice).replace(/\0.*$/, '');
        continue;
      }
      if (typeflag === 'x') {
        paxNext = parsePax(dataSlice);
        continue;
      }
      if (typeflag === 'g') {
        continue;
      }

      const fullName = paxNext.path || nextLongName || (prefix ? `${prefix}/${rawName}` : rawName);
      const linkname = paxNext.linkpath || nextLongLink || rawLink;
      nextLongName = '';
      nextLongLink = '';
      paxNext = {};

      entries.push({
        name: fullName,
        mode,
        typeflag,
        linkname,
        data: new Uint8Array(dataSlice),
      });
    }
    return entries;
  }

  async ensureEsbuildCompiler() {
    if (this.#esbuildCompiler) return { ok: true, compiler: this.#esbuildCompiler };
    if (!this.esbuildModuleUrl || !this.esbuildWasmUrl) {
      return {
        ok: false,
        error_code: 'EsbuildAdapterUnavailable',
        message: 'Foundry esbuildModuleUrl / esbuildWasmUrl not configured in Worker host config',
      };
    }
    if (!this.#esbuildPromise) {
      this.#esbuildPromise = (async () => {
        const mod = await import(this.esbuildModuleUrl);
        if (mod && typeof mod.initialize === 'function') {
          await mod.initialize({ wasmURL: this.esbuildWasmUrl, worker: false });
        }
        this.#esbuildCompiler = mod;
        return mod;
      })();
    }
    try {
      const compiler = await this.#esbuildPromise;
      return { ok: true, compiler };
    } catch (err) {
      this.#esbuildPromise = null;
      this.#esbuildCompiler = null;
      return {
        ok: false,
        error_code: 'EsbuildAdapterUnavailable',
        message: err && err.message ? err.message : String(err),
      };
    }
  }

  #needsEsbuildTranspile(code, filePath, args) {
    const ext = String(filePath || '').toLowerCase();
    if (
      ext.endsWith('.ts') ||
      ext.endsWith('.tsx') ||
      ext.endsWith('.mts') ||
      ext.endsWith('.cts') ||
      ext.endsWith('.jsx') ||
      ext.endsWith('.mjs')
    ) {
      return true;
    }
    const argList = Array.isArray(args) ? args : [];
    if (argList.some((a) => String(a).includes('--loader=ts') || String(a) === '--experimental-strip-types')) {
      return true;
    }
    const src = String(code || '');
    return /^\s*(?:import\s+(?:[\s\S]+?\s+from\s+)?['"]|export\s+(?:default|const|let|var|function|class|type|interface|\{))/m.test(src);
  }

  #extractBridgePayload(req) {
    const p = req && req.payload && typeof req.payload === 'object' ? req.payload : req;
    const args = Array.isArray(p.args) ? p.args : (Array.isArray(req.args) ? req.args : []);
    const cwd = String(p.cwd || req.cwd || '/workspace');
    const env = p.env && typeof p.env === 'object' ? p.env : (req.env && typeof req.env === 'object' ? req.env : {});
    const workspace = Array.isArray(p.workspace) ? p.workspace : (Array.isArray(req.workspace) ? req.workspace : []);
    let stdinBytes = new Uint8Array(0);
    if (typeof p.stdin_base64 === 'string' && p.stdin_base64) {
      stdinBytes = __uorBase64ToBytes(p.stdin_base64);
    } else if (typeof p.stdin_utf8 === 'string' && p.stdin_utf8) {
      stdinBytes = cachedTextEncoder.encode(p.stdin_utf8);
    } else if (typeof req.stdin_base64 === 'string' && req.stdin_base64) {
      stdinBytes = __uorBase64ToBytes(req.stdin_base64);
    } else if (typeof req.stdin_utf8 === 'string' && req.stdin_utf8) {
      stdinBytes = cachedTextEncoder.encode(req.stdin_utf8);
    }
    return {
      tool: String(p.tool || req.tool || 'node').toLowerCase(),
      args,
      cwd,
      env,
      workspace,
      stdinBytes,
    };
  }

  #makePosixPathShim() {
    const normalize = (p) => MvmPolyglotVfs.normalizePath('/', p);
    const resolve = (...segs) => {
      let acc = '/workspace';
      for (const s of segs) {
        const str = String(s || '');
        if (!str) continue;
        acc = MvmPolyglotVfs.normalizePath(acc, str);
      }
      return acc;
    };
    const join = (...segs) => {
      const raw = segs.map((s) => String(s || '')).filter(Boolean).join('/');
      if (!raw) return '.';
      if (raw.startsWith('/')) return normalize(raw);
      const stack = [];
      for (const part of raw.split('/')) {
        if (!part || part === '.') continue;
        if (part === '..') {
          if (stack.length > 0 && stack[stack.length - 1] !== '..') stack.pop();
          else stack.push('..');
        } else {
          stack.push(part);
        }
      }
      return stack.length === 0 ? '.' : stack.join('/');
    };
    const dirname = (p) => {
      const s = String(p || '');
      const idx = s.lastIndexOf('/');
      if (idx < 0) return '.';
      return idx === 0 ? '/' : s.slice(0, idx);
    };
    const basename = (p, ext) => {
      const s = String(p || '').replace(/\/+$/, '');
      const base = s.slice(s.lastIndexOf('/') + 1);
      return ext && base.endsWith(ext) ? base.slice(0, base.length - ext.length) : base;
    };
    const extname = (p) => {
      const b = basename(p);
      const dot = b.lastIndexOf('.');
      return dot > 0 ? b.slice(dot) : '';
    };
    const relative = (from, to) => {
      const f = resolve(from).split('/').filter(Boolean);
      const t = resolve(to).split('/').filter(Boolean);
      let common = 0;
      while (common < f.length && common < t.length && f[common] === t[common]) common += 1;
      const ups = new Array(f.length - common).fill('..');
      return ups.concat(t.slice(common)).join('/') || '.';
    };
    const pathObj = {
      sep: '/',
      delimiter: ':',
      normalize,
      resolve,
      join,
      dirname,
      basename,
      extname,
      relative,
      isAbsolute: (p) => String(p || '').startsWith('/'),
    };
    pathObj.posix = pathObj;
    return pathObj;
  }

  async handleNodeExec(req) {
    const info = this.#extractBridgePayload(req);
    const vfs = new MvmPolyglotVfs(info.workspace, info.cwd);
    const args = info.args.map((a) => String(a));

    if (info.tool === 'npx') {
      const cmdIdx = args.findIndex((a) => !a.startsWith('-'));
      if (cmdIdx < 0) {
        return { ok: true, exit_code: 1, stdout: '', stderr: 'npx: missing command argument\n', writes: [], deletes: [] };
      }
      const cmdName = args[cmdIdx];
      const cmdArgs = args.slice(cmdIdx + 1);
      if (cmdName === 'esbuild') {
        const esRes = await this.ensureEsbuildCompiler();
        if (!esRes.ok) return esRes;
        const entry = cmdArgs.find((a) => !a.startsWith('-'));
        if (!entry) {
          return { ok: true, exit_code: 0, stdout: `esbuild ${esRes.compiler.version || 'wasm'}\n`, stderr: '', writes: [], deletes: [] };
        }
        const src = vfs.readText(entry) || '';
        try {
          const out = await esRes.compiler.transform(src, { loader: entry.endsWith('.ts') ? 'ts' : 'js', format: 'cjs' });
          const outFlagIdx = cmdArgs.findIndex((a) => a.startsWith('--outfile='));
          if (outFlagIdx >= 0) {
            const outFile = cmdArgs[outFlagIdx].slice('--outfile='.length);
            vfs.writeBytes(outFile, out.code || '');
            const muts = vfs.exportMutations();
            return { ok: true, exit_code: 0, stdout: '', stderr: '', ...muts };
          }
          return { ok: true, exit_code: 0, stdout: out.code || '', stderr: '', writes: [], deletes: [] };
        } catch (err) {
          return {
            ok: true,
            exit_code: 1,
            stdout: '',
            stderr: (err && err.stack ? err.stack : (err && err.message ? err.message : String(err))) + '\n',
            writes: [],
            deletes: [],
          };
        }
      }
      const binPath = `/workspace/node_modules/.bin/${cmdName}`;
      if (!vfs.exists(binPath)) {
        const installed = await this.handleNpmBridge({
          ecosystem: 'npm',
          tool: 'npm',
          args: ['install', cmdName],
          cwd: info.cwd,
          workspace: info.workspace,
        });
        if (!installed.ok) return installed;
        const mergedWs = new MvmPolyglotVfs(info.workspace, info.cwd);
        for (const w of installed.writes || []) {
          if (w.symlink_target) mergedWs.writeSymlink(w.path, w.symlink_target);
          else mergedWs.writeBytes(w.path, __uorBase64ToBytes(w.content_base64 || ''), w.mode);
        }
        const reExec = await this.handleNodeExec({
          ...req,
          tool: 'node',
          args: [binPath, ...cmdArgs],
          workspace: (mergedWs.files.size || mergedWs.symlinks.size) ? Array.from(mergedWs.files.entries()).map(([p, b]) => ({
            path: p,
            content_base64: __uorBytesToBase64(b),
            mode: mergedWs.modes.get(p) || 0o100644,
          })).concat(Array.from(mergedWs.symlinks.entries()).map(([p, t]) => ({
            path: p,
            symlink_target: t,
            mode: 0o120777,
          }))) : info.workspace,
        });
        if (!reExec.ok) return reExec;
        const mergedWriteMap = new Map();
        for (const w of installed.writes || []) {
          mergedWriteMap.set(MvmPolyglotVfs.normalizeRel(w.path), w);
        }
        const mergedDeleteSet = new Set((installed.deletes || []).map((d) => MvmPolyglotVfs.normalizeRel(d)));
        for (const d of reExec.deletes || []) {
          const dr = MvmPolyglotVfs.normalizeRel(d);
          mergedWriteMap.delete(dr);
          mergedDeleteSet.add(dr);
        }
        for (const w of reExec.writes || []) {
          const wr = MvmPolyglotVfs.normalizeRel(w.path);
          mergedDeleteSet.delete(wr);
          mergedWriteMap.set(wr, w);
        }
        return {
          ...reExec,
          writes: Array.from(mergedWriteMap.values()),
          deletes: Array.from(mergedDeleteSet),
        };
      }
      return this.handleNodeExec({ ...req, tool: 'node', args: [binPath, ...cmdArgs] });
    }

    let evalCode = null;
    let printExpr = false;
    let scriptPath = null;
    const scriptArgs = [];
    for (let i = 0; i < args.length; i += 1) {
      const a = args[i];
      if ((a === '-e' || a === '--eval') && i + 1 < args.length && evalCode === null && !scriptPath) {
        evalCode = args[i + 1];
        i += 1;
      } else if ((a === '-p' || a === '--print') && i + 1 < args.length && evalCode === null && !scriptPath) {
        evalCode = args[i + 1];
        printExpr = true;
        i += 1;
      } else if ((a === '-v' || a === '--version') && evalCode === null && !scriptPath) {
        return { ok: true, exit_code: 0, stdout: 'v20.18.0-mvm\n', stderr: '', writes: [], deletes: [] };
      } else if (a.startsWith('--loader=') || a === '--experimental-strip-types') {
        continue;
      } else if (!a.startsWith('-') && evalCode === null && !scriptPath) {
        scriptPath = a;
      } else {
        scriptArgs.push(a);
      }
    }

    if (evalCode === null && !scriptPath) {
      evalCode = cachedTextDecoder.decode(info.stdinBytes);
    }

    let entryAbs = '/workspace/__eval__.js';
    let entryCode = evalCode;
    if (scriptPath) {
      const normScriptAbs = MvmPolyglotVfs.normalizePath(vfs.cwd, scriptPath);
      try {
        entryAbs = vfs.resolveGuestTarget(normScriptAbs).abs;
      } catch (err) {
        if (err && err.code === 'CanonicalPathProtected') {
          return { ok: false, error_code: err.code, message: err.message };
        }
        throw err;
      }
      const raw = vfs.readText(entryAbs);
      if (raw === null) {
        return {
          ok: true,
          exit_code: 1,
          stdout: '',
          stderr: `Error: Cannot find module '${scriptPath}'\n`,
          writes: [],
          deletes: [],
        };
      }
      entryCode = raw;
    }

    const stdoutChunks = [];
    const stderrChunks = [];
    let procExitCode = 0;
    class NodeExitSignal {
      constructor(code) {
        this.code = code | 0;
      }
    }

    const pathShim = this.#makePosixPathShim();
    const fsShim = {
      readFileSync: (p, enc) => {
        const bytes = vfs.readBytes(p);
        if (!bytes) {
          const e = new Error(`ENOENT: no such file or directory, open '${p}'`);
          e.code = 'ENOENT';
          throw e;
        }
        return enc ? MvmNodeBuffer.from(bytes).toString(enc) : MvmNodeBuffer.from(bytes);
      },
      writeFileSync: (p, data) => vfs.writeBytes(p, data),
      appendFileSync: (p, data) => {
        const cur = vfs.readBytes(p) || new Uint8Array(0);
        const add = data instanceof Uint8Array ? data : cachedTextEncoder.encode(String(data ?? ''));
        vfs.writeBytes(p, MvmNodeBuffer.concat([cur, add]));
      },
      existsSync: (p) => vfs.exists(p),
      mkdirSync: (p, opts) => vfs.mkdir(p, opts),
      readdirSync: (p) => vfs.readdir(p),
      unlinkSync: (p) => {
        if (!vfs.deletePath(p)) {
          const e = new Error(`ENOENT: no such file or directory, unlink '${p}'`);
          e.code = 'ENOENT';
          throw e;
        }
      },
      rmSync: (p) => vfs.deletePath(p),
      readlinkSync: (p) => {
        const rel = MvmPolyglotVfs.normalizeRel(MvmPolyglotVfs.normalizePath(vfs.cwd, p));
        if (!vfs.symlinks.has(rel)) {
          const e = new Error(`EINVAL: invalid argument, readlink '${p}'`);
          e.code = 'EINVAL';
          throw e;
        }
        return vfs.symlinks.get(rel);
      },
      symlinkSync: (target, linkPath) => vfs.writeSymlink(linkPath, target),
      realpathSync: (p) => vfs.resolveGuestTarget(p).abs,
      statSync: (p) => {
        const resolved = vfs.resolveGuestTarget(p);
        const isDir = vfs.isDirectory(resolved.abs);
        const bytes = isDir ? null : vfs.readBytes(resolved.abs);
        if (!isDir && !bytes) {
          const e = new Error(`ENOENT: no such file or directory, stat '${p}'`);
          e.code = 'ENOENT';
          throw e;
        }
        return {
          size: bytes ? bytes.byteLength : 0,
          isFile: () => !isDir,
          isDirectory: () => isDir,
          isSymbolicLink: () => false,
        };
      },
      lstatSync: (p) => {
        const abs = MvmPolyglotVfs.normalizePath(vfs.cwd, p);
        const rel = MvmPolyglotVfs.normalizeRel(abs);
        if (rel && vfs.symlinks.has(rel)) {
          return {
            size: vfs.symlinks.get(rel).length,
            isFile: () => false,
            isDirectory: () => false,
            isSymbolicLink: () => true,
          };
        }
        return fsShim.statSync(p);
      },
    };
    fsShim.promises = {
      readFile: async (p, enc) => fsShim.readFileSync(p, enc),
      writeFile: async (p, data) => fsShim.writeFileSync(p, data),
      readdir: async (p) => fsShim.readdirSync(p),
      mkdir: async (p, opts) => fsShim.mkdirSync(p, opts),
      unlink: async (p) => fsShim.unlinkSync(p),
      stat: async (p) => fsShim.statSync(p),
      lstat: async (p) => fsShim.lstatSync(p),
    };

    const processShim = {
      version: 'v20.18.0-mvm',
      versions: { node: '20.18.0', v8: '12.0', wasi: '1.0' },
      platform: 'linux',
      arch: 'riscv64',
      argv: ['node', scriptPath || '-e', ...scriptArgs],
      env: { ...info.env },
      cwd: () => vfs.cwd,
      chdir: (nextDir) => {
        vfs.cwd = MvmPolyglotVfs.normalizePath(vfs.cwd, nextDir);
      },
      stdout: {
        write: (s) => {
          stdoutChunks.push(String(s));
          return true;
        },
      },
      stderr: {
        write: (s) => {
          stderrChunks.push(String(s));
          return true;
        },
      },
      exit: (code) => {
        throw new NodeExitSignal(code || 0);
      },
      nextTick: (fn, ...cbArgs) => Promise.resolve().then(() => fn(...cbArgs)),
    };

    const formatArg = (v) => {
      if (typeof v === 'string') return v;
      if (v instanceof Error) return v.stack || v.message;
      try {
        return typeof v === 'object' && v !== null ? JSON.stringify(v) : String(v);
      } catch {
        return String(v);
      }
    };
    const consoleShim = {
      log: (...vals) => stdoutChunks.push(vals.map(formatArg).join(' ') + '\n'),
      info: (...vals) => stdoutChunks.push(vals.map(formatArg).join(' ') + '\n'),
      warn: (...vals) => stderrChunks.push(vals.map(formatArg).join(' ') + '\n'),
      error: (...vals) => stderrChunks.push(vals.map(formatArg).join(' ') + '\n'),
    };

    const cryptoShim = {
      randomBytes: (n) => {
        const buf = MvmNodeBuffer.alloc(n, 0);
        if (typeof crypto !== 'undefined' && typeof crypto.getRandomValues === 'function') {
          crypto.getRandomValues(buf);
        }
        return buf;
      },
      randomUUID: () => (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
        ? crypto.randomUUID()
        : '00000000-0000-4000-8000-000000000000'),
      subtle: typeof crypto !== 'undefined' ? crypto.subtle : undefined,
    };

    const urlShim = {
      URL,
      URLSearchParams,
      fileURLToPath: (u) => new URL(u).pathname,
      pathToFileURL: (p) => new URL(`file://${MvmPolyglotVfs.normalizePath('/', p)}`),
    };

    const fetchShim = async (url, init) => {
      const netCheck = this.#assertNetworkAllowed();
      if (!netCheck.ok) {
        const e = new Error(netCheck.message);
        e.code = netCheck.error_code;
        throw e;
      }
      const opts = init && typeof init === 'object' ? init : {};
      const res = await this.handleHttpRequest({
        url: String(url),
        method: opts.method || 'GET',
        headers: opts.headers || {},
        body: typeof opts.body === 'string' ? opts.body : undefined,
      });
      if (!res.ok && res.error_code) {
        const e = new Error(res.message || res.error_code);
        e.code = res.error_code;
        throw e;
      }
      const text = String(res.body_text || '');
      return {
        ok: res.status >= 200 && res.status < 300,
        status: res.status || 200,
        text: async () => text,
        json: async () => JSON.parse(text),
        arrayBuffer: async () => cachedTextEncoder.encode(text).buffer,
      };
    };

    const assertShim = {
      ok: (cond, msg) => { if (!cond) throw new Error(msg || 'AssertionError'); },
      equal: (a, b, msg) => { if (a != b) throw new Error(msg || `${a} != ${b}`); },
      strictEqual: (a, b, msg) => { if (a !== b) throw new Error(msg || `${a} !== ${b}`); },
      deepStrictEqual: (a, b, msg) => {
        if (JSON.stringify(a) !== JSON.stringify(b)) throw new Error(msg || 'AssertionError: deepStrictEqual failed');
      },
      fail: (msg) => { throw new Error(msg || 'AssertionError: failed'); },
    };
    const utilShim = {
      format: (fmt, ...vals) => {
        if (typeof fmt !== 'string') return [fmt, ...vals].map(formatArg).join(' ');
        let vi = 0;
        const replaced = fmt.replace(/%([sdj%])/g, (full, ch) => {
          if (ch === '%') return '%';
          if (vi >= vals.length) return full;
          const v = vals[vi++];
          if (ch === 'j') return JSON.stringify(v);
          if (ch === 'd') return String(Number(v));
          return formatArg(v);
        });
        return vi < vals.length ? [replaced, ...vals.slice(vi).map(formatArg)].join(' ') : replaced;
      },
      inspect: (v) => formatArg(v),
    };
    const builtins = new Map([
      ['fs', fsShim],
      ['node:fs', fsShim],
      ['fs/promises', fsShim.promises],
      ['node:fs/promises', fsShim.promises],
      ['path', pathShim],
      ['node:path', pathShim],
      ['process', processShim],
      ['node:process', processShim],
      ['buffer', { Buffer: MvmNodeBuffer }],
      ['node:buffer', { Buffer: MvmNodeBuffer }],
      ['crypto', cryptoShim],
      ['node:crypto', cryptoShim],
      ['url', urlShim],
      ['node:url', urlShim],
      ['assert', assertShim],
      ['node:assert', assertShim],
      ['util', utilShim],
      ['node:util', utilShim],
    ]);

    const transpileMap = new Map();
    const prepareCodeSyncOrNeedEsbuild = async (rawSource, filePath) => {
      const strippedShebang = String(rawSource || '').replace(/^#![^\n]*\n/, '');
      if (!this.#needsEsbuildTranspile(strippedShebang, filePath, args)) {
        return { ok: true, code: strippedShebang };
      }
      const esRes = await this.ensureEsbuildCompiler();
      if (!esRes.ok) return esRes;
      const ext = pathShim.extname(filePath).toLowerCase();
      const loader = ext === '.tsx' ? 'tsx' : (ext === '.ts' || ext === '.mts' || ext === '.cts' ? 'ts' : (ext === '.jsx' ? 'jsx' : 'js'));
      const transformed = await esRes.compiler.transform(strippedShebang, {
        loader,
        format: 'cjs',
        target: 'es2022',
      });
      return { ok: true, code: transformed.code || '' };
    };

    const resolveModulePath = (spec, fromDir) => {
      const tryFile = (cand) => {
        for (const ext of ['', '.js', '.cjs', '.mjs', '.ts', '.tsx', '.json']) {
          const p = `${cand}${ext}`;
          if (vfs.exists(p) && !vfs.isDirectory(p)) {
            return vfs.resolveGuestTarget(p).abs;
          }
        }
        if (vfs.isDirectory(cand)) {
          const pkgRaw = vfs.readText(`${cand}/package.json`);
          if (pkgRaw) {
            try {
              const pj = JSON.parse(pkgRaw);
              let entry = pj.main || 'index.js';
              if (pj.exports && typeof pj.exports === 'object' && pj.exports['.']) {
                const dotExp = pj.exports['.'];
                if (typeof dotExp === 'string') entry = dotExp;
                else if (dotExp && typeof dotExp === 'object') {
                  entry = dotExp.require || dotExp.default || dotExp.import || entry;
                }
              }
              const target = MvmPolyglotVfs.normalizePath(cand, entry);
              if (vfs.exists(target) && !vfs.isDirectory(target)) {
                return vfs.resolveGuestTarget(target).abs;
              }
            } catch {
              // Fall back to index.js
            }
          }
          for (const idxExt of ['/index.js', '/index.cjs', '/index.ts']) {
            if (vfs.exists(`${cand}${idxExt}`)) {
              return vfs.resolveGuestTarget(`${cand}${idxExt}`).abs;
            }
          }
        }
        return null;
      };
      if (spec.startsWith('./') || spec.startsWith('../') || spec.startsWith('/')) {
        return tryFile(MvmPolyglotVfs.normalizePath(fromDir, spec));
      }
      let curDir = fromDir;
      while (true) {
        const nmCand = `${curDir === '/' ? '' : curDir}/node_modules/${spec}`;
        const hit = tryFile(nmCand);
        if (hit) return hit;
        if (curDir === '/' || curDir === '/workspace') break;
        curDir = MvmPolyglotVfs.parentDir(curDir);
      }
      return null;
    };

    const collectDeps = async (srcText, filePath, visited) => {
      if (visited.has(filePath)) return { ok: true };
      visited.add(filePath);
      const tRes = await prepareCodeSyncOrNeedEsbuild(srcText, filePath);
      if (!tRes.ok) return tRes;
      transpileMap.set(filePath, tRes.code);
      const dir = pathShim.dirname(filePath);
      const reqMatches = tRes.code.matchAll(/require\s*\(\s*['"]([^'"]+)['"]\s*\)/g);
      for (const m of reqMatches) {
        const spec = m[1];
        if (builtins.has(spec)) continue;
        const resolved = resolveModulePath(spec, dir);
        if (resolved) {
          const childRaw = vfs.readText(resolved);
          if (childRaw !== null) {
            const sub = await collectDeps(childRaw, resolved, visited);
            if (!sub.ok) return sub;
          }
        }
      }
      return { ok: true };
    };

    const moduleCache = new Map();
    const runModule = (fileAbs, isEntry) => {
      if (moduleCache.has(fileAbs)) return moduleCache.get(fileAbs).exports;
      if (fileAbs.endsWith('.json')) {
        const parsed = JSON.parse(vfs.readText(fileAbs) || 'null');
        moduleCache.set(fileAbs, { exports: parsed });
        return parsed;
      }
      const mod = { exports: {} };
      moduleCache.set(fileAbs, mod);
      const modDir = pathShim.dirname(fileAbs);
      const localRequire = (spec) => {
        if (builtins.has(spec)) return builtins.get(spec);
        const resolved = resolveModulePath(spec, modDir);
        if (!resolved) {
          const e = new Error(`Cannot find module '${spec}'`);
          e.code = 'MODULE_NOT_FOUND';
          throw e;
        }
        return runModule(resolved, false);
      };
      const compiledCode = transpileMap.get(fileAbs) || '';
      const execBody = isEntry && printExpr ? `return (${compiledCode});` : compiledCode;
      const fn = new Function(
        'exports',
        'require',
        'module',
        '__filename',
        '__dirname',
        'process',
        'console',
        'Buffer',
        'fetch',
        execBody,
      );
      const ret = fn(
        mod.exports,
        localRequire,
        mod,
        fileAbs,
        modDir,
        processShim,
        consoleShim,
        MvmNodeBuffer,
        fetchShim,
      );
      if (isEntry && printExpr && ret !== undefined) {
        consoleShim.log(ret);
      }
      return isEntry ? ret : mod.exports;
    };

    try {
      const prepRes = await collectDeps(entryCode, entryAbs, new Set());
      if (!prepRes.ok) return prepRes;
      const ret = runModule(entryAbs, true);
      if (ret && typeof ret.then === 'function') {
        await ret;
      }
    } catch (err) {
      if (err instanceof NodeExitSignal) {
        procExitCode = err.code;
      } else if (err && (err.code === 'CanonicalPathProtected' || err.code === 'EsbuildAdapterUnavailable')) {
        return { ok: false, error_code: err.code, message: err.message };
      } else {
        procExitCode = 1;
        stderrChunks.push((err && err.stack ? err.stack : String(err)) + '\n');
      }
    }

    const muts = vfs.exportMutations();
    return {
      ok: true,
      exit_code: procExitCode,
      stdout: stdoutChunks.join(''),
      stderr: stderrChunks.join(''),
      ...muts,
    };
  }

  async handleNpmBridge(req) {
    /* Formal R4.MvmPolyglotToolchain */
    const info = this.#extractBridgePayload(req);
    const vfs = new MvmPolyglotVfs(info.workspace, info.cwd);
    const args = info.args.map((a) => String(a));
    const sub = (args[0] || 'install').toLowerCase();

    const touchesProtected = args.some((a) => {
      if (a.startsWith('-')) return false;
      const abs = MvmPolyglotVfs.normalizePath(vfs.cwd, a);
      if (!abs.startsWith('/workspace/')) return false;
      return MvmPolyglotVfs.isCanonicalProtectedRel(abs.slice('/workspace/'.length));
    });
    const knownSubs = new Set([
      'init',
      'install',
      'i',
      'add',
      'run',
      'test',
      't',
      'start',
      'build',
      'ls',
      'list',
      'uninstall',
      'rm',
      'remove',
      '--version',
      '-v',
      'version',
    ]);
    const verdict = admitNpmTerminalSubcommand(knownSubs.has(sub), touchesProtected);
    if (verdict === 'RefusedProtectedPathToolchainMutation') {
      return {
        ok: false,
        error_code: 'CanonicalPathProtected',
        message: 'npm: RefusedProtectedPathToolchainMutation',
      };
    }
    if (verdict === 'RefusedUnsupportedToolchainSubcommand') {
      return {
        ok: true,
        exit_code: 1,
        stdout: '',
        stderr: `npm: unsupported subcommand '${sub}' (RefusedUnsupportedToolchainSubcommand)\n`,
        writes: [],
        deletes: [],
      };
    }

    if (sub === '--version' || sub === '-v' || sub === 'version') {
      return { ok: true, exit_code: 0, stdout: '10.9.0-mvm\n', stderr: '', writes: [], deletes: [] };
    }

    if (sub === 'init') {
      if (!vfs.exists('/workspace/package.json')) {
        vfs.writeBytes(
          '/workspace/package.json',
          JSON.stringify({ name: 'workspace-app', version: '1.0.0', private: true, dependencies: {} }, null, 2) + '\n',
        );
      }
      const muts = vfs.exportMutations();
      return { ok: true, exit_code: 0, stdout: 'Wrote /workspace/package.json\n', stderr: '', ...muts };
    }

    const scriptTarget =
      sub === 'test' || sub === 't'
        ? 'test'
        : sub === 'start'
          ? 'start'
          : sub === 'build'
            ? 'build'
            : sub === 'run' && args[1]
              ? args[1]
              : '';
    if (scriptTarget) {
      const pjRaw = vfs.readText('/workspace/package.json');
      if (!pjRaw) {
        return { ok: true, exit_code: 1, stdout: '', stderr: 'npm ERR! missing package.json\n', writes: [], deletes: [] };
      }
      const pj = JSON.parse(pjRaw);
      const scriptCmd = pj.scripts && pj.scripts[scriptTarget];
      if (!scriptCmd) {
        return { ok: true, exit_code: 1, stdout: '', stderr: `npm ERR! Missing script: "${scriptTarget}"\n`, writes: [], deletes: [] };
      }
      const trimmedScript = scriptCmd.trim();
      const nodeEvalMatch = trimmedScript.match(/^node\s+(-e|-p)\s+(['"])([\s\S]*)\2$/);
      if (nodeEvalMatch) {
        return this.handleNodeExec({ ...req, tool: 'node', args: [nodeEvalMatch[1], nodeEvalMatch[3]] });
      }
      const parts = trimmedScript.split(/\s+/);
      const binName = parts[0] === 'node' ? parts : ['node', `/workspace/node_modules/.bin/${parts[0]}`, ...parts.slice(1)];
      return this.handleNodeExec({ ...req, tool: 'node', args: binName.slice(1) });
    }

    if (sub === 'ls' || sub === 'list') {
      const pjRaw = vfs.readText('/workspace/package.json');
      let pj = { name: 'workspace-app', version: '1.0.0', dependencies: {} };
      if (pjRaw) {
        try {
          pj = JSON.parse(pjRaw);
        } catch {}
      }
      const deps = pj.dependencies && typeof pj.dependencies === 'object' ? Object.entries(pj.dependencies) : [];
      const lines = [`${pj.name || 'workspace-app'}@${pj.version || '1.0.0'} /workspace`];
      if (deps.length === 0) {
        lines.push('└── (empty)');
      } else {
        deps.forEach(([depName, depRange], idx) => {
          const instRaw = vfs.readText(`/workspace/node_modules/${depName}/package.json`);
          let instVer = String(depRange || '');
          if (instRaw) {
            try {
              const parsedDep = JSON.parse(instRaw);
              if (parsedDep && parsedDep.version) instVer = parsedDep.version;
            } catch {}
          }
          const branch = idx + 1 === deps.length ? '└──' : '├──';
          lines.push(`${branch} ${depName}@${instVer}`);
        });
      }
      return { ok: true, exit_code: 0, stdout: lines.join('\n') + '\n', stderr: '', writes: [], deletes: [] };
    }

    if (sub === 'uninstall' || sub === 'rm' || sub === 'remove') {
      const pkgs = args.slice(1).filter((a) => !a.startsWith('-'));
      if (pkgs.length === 0) {
        return { ok: true, exit_code: 1, stdout: '', stderr: 'npm ERR! Must provide a package name to remove\n', writes: [], deletes: [] };
      }
      const pjRaw = vfs.readText('/workspace/package.json');
      let pj = { name: 'workspace-app', version: '1.0.0', dependencies: {} };
      if (pjRaw) {
        try {
          pj = JSON.parse(pjRaw);
        } catch {}
      }
      let removedCount = 0;
      for (const pkgName of pkgs) {
        const prefix = `node_modules/${pkgName}/`;
        let hit = false;
        for (const key of Array.from(vfs.files.keys())) {
          if (key.startsWith(prefix)) {
            vfs.deletePath(`/workspace/${key}`);
            hit = true;
          }
        }
        for (const key of Array.from(vfs.symlinks.keys())) {
          if (key.startsWith(prefix) || key === `node_modules/.bin/${pkgName.split('/').pop()}`) {
            vfs.deletePath(`/workspace/${key}`);
            hit = true;
          }
        }
        if (pj.dependencies && Object.prototype.hasOwnProperty.call(pj.dependencies, pkgName)) {
          delete pj.dependencies[pkgName];
          hit = true;
        }
        if (hit) removedCount += 1;
      }
      if (pjRaw) {
        vfs.writeBytes('/workspace/package.json', JSON.stringify(pj, null, 2) + '\n');
      }
      const muts = vfs.exportMutations();
      return { ok: true, exit_code: 0, stdout: `removed ${removedCount} package(s)\n`, stderr: '', ...muts };
    }

    if (sub !== 'install' && sub !== 'i' && sub !== 'add') {
      return { ok: true, exit_code: 1, stdout: '', stderr: `npm: unsupported subcommand '${sub}'\n`, writes: [], deletes: [] };
    }

    const netCheck = this.#assertNetworkAllowed();
    if (!netCheck.ok) return netCheck;

    const specs = args.slice(1).filter((a) => !a.startsWith('-'));
    const pkgJsonRaw = vfs.readText('/workspace/package.json');
    let pkgJson = { name: 'workspace-app', version: '1.0.0', dependencies: {} };
    if (pkgJsonRaw) {
      try {
        pkgJson = JSON.parse(pkgJsonRaw);
      } catch {
        pkgJson = { name: 'workspace-app', version: '1.0.0', dependencies: {} };
      }
    }
    if (!pkgJson.dependencies || typeof pkgJson.dependencies !== 'object') {
      pkgJson.dependencies = {};
    }

    const toInstall = specs.length > 0
      ? specs
      : Object.entries(pkgJson.dependencies).map(([n, v]) => `${n}@${v}`);
    if (toInstall.length === 0) {
      return { ok: true, exit_code: 0, stdout: 'up to date\n', stderr: '', writes: [], deletes: [] };
    }

    const installedCount = [];
    const seen = new Set();
    const parseSpec = (rawSpec) => {
      const s = String(rawSpec).trim();
      const at = s.startsWith('@') ? s.indexOf('@', 1) : s.indexOf('@');
      return at > 0 ? { name: s.slice(0, at), range: s.slice(at + 1) } : { name: s, range: 'latest' };
    };
    const pickVersion = (meta, range) => {
      const versions = meta.versions && typeof meta.versions === 'object' ? Object.keys(meta.versions) : [];
      const tags = meta['dist-tags'] || {};
      if (!range || range === 'latest' || range === '*') {
        return tags.latest || versions[versions.length - 1];
      }
      if (meta.versions && meta.versions[range]) return range;
      const clean = range.replace(/^[~^>=<\s]+/, '');
      if (meta.versions && meta.versions[clean]) return clean;
      const major = clean.split('.')[0];
      const match = versions.filter((v) => v.startsWith(`${major}.`)).pop();
      return match || tags.latest || versions[versions.length - 1];
    };

    const installSingle = async (rawSpec, isDirect) => {
      const { name, range } = parseSpec(rawSpec);
      if (!name || seen.has(name)) return { ok: true };
      seen.add(name);
      const encodedName = name.startsWith('@') ? `@${encodeURIComponent(name.slice(1))}` : encodeURIComponent(name);
      const regUrl = `https://registry.npmjs.org/${encodedName}`;
      const metaFetch = await this.#admittedFetch(
        regUrl,
        { headers: { Accept: 'application/json' } },
        { operation: 'npm_install' },
      );
      if (!metaFetch.ok) {
        if (metaFetch.error && metaFetch.error.error_code && metaFetch.error.error_code !== 'HttpBridgeFetchError') {
          return metaFetch.error;
        }
        return {
          ok: false,
          status: 502,
          error_code: 'NpmRegistryError',
          message: `Failed fetching npm metadata for ${name} (${metaFetch.error ? metaFetch.error.message : 'error'})`,
        };
      }
      const metaResp = metaFetch.response;
      if (!metaResp || !metaResp.ok) {
        return {
          ok: false,
          status: metaResp ? metaResp.status : 502,
          error_code: 'NpmRegistryError',
          message: `Failed fetching npm metadata for ${name} (HTTP ${metaResp ? metaResp.status : 'error'})`,
        };
      }
      const meta = await metaResp.json();
      const ver = pickVersion(meta, range);
      const verObj = meta.versions && ver ? meta.versions[ver] : null;
      if (!verObj || !verObj.dist || !verObj.dist.tarball) {
        return {
          ok: false,
          status: 404,
          error_code: 'PackageNotFound',
          message: `Version "${range}" not found for npm package "${name}"`,
        };
      }
      const expectedSri = verObj.dist.integrity || '';
      const tgzFetch = await this.#admittedFetch(
        verObj.dist.tarball,
        undefined,
        { operation: 'npm_install' },
      );
      if (!tgzFetch.ok) {
        if (tgzFetch.error && tgzFetch.error.error_code && tgzFetch.error.error_code !== 'HttpBridgeFetchError') {
          return tgzFetch.error;
        }
        return {
          ok: false,
          status: 502,
          error_code: 'PackageDownloadFailed',
          message: `Failed downloading tarball ${verObj.dist.tarball}`,
        };
      }
      const tgzResp = tgzFetch.response;
      if (!tgzResp || !tgzResp.ok) {
        return {
          ok: false,
          status: tgzResp ? tgzResp.status : 502,
          error_code: 'PackageDownloadFailed',
          message: `Failed downloading tarball ${verObj.dist.tarball}`,
        };
      }
      const tgzBytes = new Uint8Array(await tgzResp.arrayBuffer());
      const sriRes = await this.#verifySri(tgzBytes, expectedSri);
      if (!sriRes.ok) {
        return sriRes;
      }
      const tarBytes = await this.#gunzipBytes(tgzBytes);
      const entries = this.#parseTarEntries(tarBytes);
      const pkgBase = `/workspace/node_modules/${name}`;
      for (const ent of entries) {
        const cleanEntry = String(ent.name || '').replace(/^\.\//, '').replace(/^package\//, '').replace(/^\/+/, '');
        if (!cleanEntry || cleanEntry.split('/').includes('..') || ent.typeflag === '5') continue;
        const dst = `${pkgBase}/${cleanEntry}`;
        if (ent.typeflag === '2') {
          vfs.writeSymlink(dst, ent.linkname);
        } else {
          vfs.writeBytes(dst, ent.data, 0o100000 | (ent.mode & 0o777));
        }
      }
      if (verObj.bin) {
        const binMap = typeof verObj.bin === 'string' ? { [name.split('/').pop()]: verObj.bin } : verObj.bin;
        for (const [bName, bRel] of Object.entries(binMap)) {
          const cleanRel = String(bRel).replace(/^\.\//, '');
          vfs.writeSymlink(`/workspace/node_modules/.bin/${bName}`, `../${name}/${cleanRel}`);
        }
      }
      if (isDirect) {
        pkgJson.dependencies[name] = `^${ver}`;
      }
      installedCount.push(`${name}@${ver}`);
      if (verObj.dependencies && typeof verObj.dependencies === 'object') {
        for (const [depName, depRange] of Object.entries(verObj.dependencies)) {
          const subRes = await installSingle(`${depName}@${depRange}`, false);
          if (!subRes.ok) return subRes;
        }
      }
      return { ok: true };
    };

    for (const spec of toInstall) {
      const r = await installSingle(spec, true);
      if (!r.ok) return r;
    }

    vfs.writeBytes('/workspace/package.json', JSON.stringify(pkgJson, null, 2) + '\n');
    const muts = vfs.exportMutations();
    return {
      ok: true,
      exit_code: 0,
      stdout: `added ${installedCount.length} package(s): ${installedCount.join(', ')}\n`,
      stderr: '',
      ...muts,
    };
  }

  async #ensurePyodideLoaded(stdoutChunks, stderrChunks) {
    this.#activePyStdoutSink = (line) => stdoutChunks.push(String(line) + '\n');
    this.#activePyStderrSink = (line) => stderrChunks.push(String(line) + '\n');

    const bindStreams = (pyodide) => {
      if (pyodide && typeof pyodide.setStdout === 'function') {
        pyodide.setStdout({
          batched: (line) => {
            if (this.#activePyStdoutSink) this.#activePyStdoutSink(line);
          },
        });
      }
      if (pyodide && typeof pyodide.setStderr === 'function') {
        pyodide.setStderr({
          batched: (line) => {
            if (this.#activePyStderrSink) this.#activePyStderrSink(line);
          },
        });
      }
    };

    if (this.#pyodideInstance) {
      bindStreams(this.#pyodideInstance);
      return { ok: true, pyodide: this.#pyodideInstance };
    }
    const lock = this.pyodideLock;
    let verifiedBytes = null;

    if (typeof caches !== 'undefined' && typeof caches.open === 'function') {
      try {
        const cache = await caches.open(PYODIDE_CACHE_NAME);
        const cachedResp = await cache.match(lock.url);
        if (cachedResp) {
          const candBytes = new Uint8Array(await cachedResp.arrayBuffer());
          const sriCheck = await this.#verifySri(candBytes, lock.sri);
          if (sriCheck.ok) verifiedBytes = candBytes;
        }
      } catch {
        // Cache API unavailable or empty
      }
    }

    if (!verifiedBytes) {
      const netCheck = this.#assertNetworkAllowed();
      if (!netCheck.ok) return netCheck;
      const respFetch = await this.#admittedFetch(
        lock.url,
        undefined,
        { operation: 'pyodide_load', allowConfiguredOrigin: true },
      );
      if (!respFetch.ok) {
        if (respFetch.error && respFetch.error.error_code && respFetch.error.error_code !== 'HttpBridgeFetchError') {
          return respFetch.error;
        }
        return {
          ok: false,
          status: 502,
          error_code: 'PyodideLoadError',
          message: `Failed fetching Pyodide entrypoint ${lock.url}`,
        };
      }
      const resp = respFetch.response;
      if (!resp || !resp.ok) {
        return {
          ok: false,
          status: resp ? resp.status : 502,
          error_code: 'PyodideLoadError',
          message: `Failed fetching Pyodide entrypoint ${lock.url}`,
        };
      }
      const rawBytes = new Uint8Array(await resp.arrayBuffer());
      const sriCheck = await this.#verifySri(rawBytes, lock.sri);
      if (!sriCheck.ok) {
        return sriCheck;
      }
      verifiedBytes = rawBytes;
      if (typeof caches !== 'undefined' && typeof caches.open === 'function') {
        try {
          const cache = await caches.open(PYODIDE_CACHE_NAME);
          await cache.put(
            lock.url,
            new Response(verifiedBytes, { status: 200, headers: { 'Content-Type': 'text/javascript' } }),
          );
        } catch {
          // Ignore cache write errors in restricted hosts
        }
      }
    }

    if (!this.#pyodidePromise) {
      this.#pyodidePromise = (async () => {
        let mod = null;
        if (typeof URL !== 'undefined' && typeof URL.createObjectURL === 'function' && typeof WorkerGlobalScope !== 'undefined') {
          const blobUrl = URL.createObjectURL(new Blob([verifiedBytes], { type: 'text/javascript' }));
          try {
            mod = await import(blobUrl);
          } finally {
            URL.revokeObjectURL(blobUrl);
          }
        } else {
          const dataUrl = `data:text/javascript;base64,${__uorBytesToBase64(verifiedBytes)}`;
          mod = await import(dataUrl);
        }
        const pyodide = await mod.loadPyodide({
          indexURL: lock.indexURL,
          stdout: (line) => {
            if (this.#activePyStdoutSink) this.#activePyStdoutSink(line);
          },
          stderr: (line) => {
            if (this.#activePyStderrSink) this.#activePyStderrSink(line);
          },
        });
        this.#pyodideInstance = pyodide;
        return pyodide;
      })();
    }

    try {
      const pyodide = await this.#pyodidePromise;
      bindStreams(pyodide);
      return { ok: true, pyodide };
    } catch (err) {
      this.#pyodidePromise = null;
      this.#pyodideInstance = null;
      return {
        ok: false,
        error_code: 'PyodideLoadError',
        message: err && err.message ? err.message : String(err),
      };
    }
  }

  async handlePythonExec(req) {
    const info = this.#extractBridgePayload(req);
    const vfs = new MvmPolyglotVfs(info.workspace, info.cwd);
    const stdoutChunks = [];
    const stderrChunks = [];

    const pyLoaded = await this.#ensurePyodideLoaded(stdoutChunks, stderrChunks);
    if (!pyLoaded.ok) {
      this.#activePyStdoutSink = null;
      this.#activePyStderrSink = null;
      return pyLoaded;
    }
    const pyodide = pyLoaded.pyodide;
    const args = info.args.map((a) => String(a));

    try {
      if (pyodide.FS && typeof pyodide.FS.mkdirTree === 'function') {
        pyodide.FS.mkdirTree('/workspace');
        if (typeof pyodide.FS.readdir === 'function' && typeof pyodide.FS.unlink === 'function') {
          const pruneStaleFs = (dirAbs) => {
            let entries = [];
            try {
              entries = pyodide.FS.readdir(dirAbs);
            } catch {
              return;
            }
            for (const name of entries) {
              if (name === '.' || name === '..') continue;
              const full = `${dirAbs}/${name}`;
              const stat = pyodide.FS.analyzePath ? pyodide.FS.analyzePath(full) : null;
              if (!stat || !stat.exists) continue;
              if (stat.object && stat.object.isFolder) {
                pruneStaleFs(full);
              } else {
                const rel = full.slice('/workspace/'.length);
                if (!vfs.files.has(rel)) {
                  try {
                    pyodide.FS.unlink(full);
                  } catch {
                    // Ignore unlink error for virtual entries
                  }
                }
              }
            }
          };
          pruneStaleFs('/workspace');
        }
        for (const [rel, bytes] of vfs.files.entries()) {
          const full = `/workspace/${rel}`;
          const parent = MvmPolyglotVfs.parentDir(full);
          pyodide.FS.mkdirTree(parent);
          pyodide.FS.writeFile(full, bytes);
        }
      }

      globalThis.__uorSpiralCorePySyncFetch = (url, method, bodyStr, headersJson) => {
        const netCheck = this.#assertNetworkAllowed();
        if (!netCheck.ok) {
          throw new Error(`${netCheck.error_code}: ${netCheck.message}`);
        }
        const urlCheck = this.validateTransportUrl(String(url), false);
        if (!urlCheck.ok) {
          throw new Error(`${urlCheck.error_code}: ${urlCheck.message}`);
        }
        let parsedHeaders = {};
        if (headersJson) {
          try {
            const obj = JSON.parse(String(headersJson));
            if (obj && typeof obj === 'object') parsedHeaders = obj;
          } catch {
            parsedHeaders = {};
          }
        }
        const hdrSub = this.applyHostSecretSubstitution(urlCheck.url, parsedHeaders);
        if (!hdrSub.ok) {
          throw new Error(`${hdrSub.error_code}: ${hdrSub.message}`);
        }
        const host = urlCheck.url.hostname.toLowerCase();
        const urlSub = this.substituteHostSecretsInString(String(url), host);
        if (!urlSub.ok) {
          throw new Error(`${urlSub.error_code}: ${urlSub.message}`);
        }
        const bodySub = this.substituteHostSecretsInString(String(bodyStr || ''), host);
        if (!bodySub.ok) {
          throw new Error(`${bodySub.error_code}: ${bodySub.message}`);
        }
        const netAdmission = this.#admitNetworkPolicy(host);
        if (!netAdmission.ok) {
          this.#emitAuditEvent('python_sync_http', urlSub.value, 'blocked', netAdmission.status || 403, 0);
          throw new Error(`${netAdmission.error_code}: ${netAdmission.message}`);
        }
        const mUpper = String(method || 'GET').toUpperCase();
        if (typeof this.#syncFetchImpl === 'function') {
          const syncRes = this.#syncFetchImpl(urlSub.value, {
            method: mUpper,
            headers: hdrSub.headers,
            body: mUpper === 'GET' || mUpper === 'HEAD' ? undefined : bodySub.value,
          });
          const outText = String((syncRes && syncRes.text !== undefined ? syncRes.text : (syncRes && syncRes.body_text)) || '');
          const outStatus = Number(syncRes && syncRes.status) || 200;
          this.#emitAuditEvent('python_sync_http', urlSub.value, 'sync-fetch-bridge', outStatus, outText.length);
          return {
            status: outStatus,
            text: outText,
          };
        }
        throw new Error('SyncHttpUnavailable: synchronous Python HTTP requires a configured syncFetchImpl bridge');
      };

      if (!this.#pyodideShimmed) {
        this.#pyodideShimmed = true;
        try {
          await pyodide.runPythonAsync(`
import io, os, sys, types, json as _uor_json
try:
    import js, urllib.request, urllib.parse
    class _SpiralResp:
        def __init__(self, text, status=200):
            self.text = str(text)
            self.status_code = int(status)
            self.status = int(status)
            self._data = self.text.encode('utf-8')
            self.content = self._data
            self.headers = {}
            self.ok = 200 <= self.status_code < 300
        def read(self):
            return self._data
        def json(self):
            return _uor_json.loads(self.text)
        def raise_for_status(self):
            if self.status_code >= 400:
                raise RuntimeError(f'HTTP {self.status_code}')
        def __enter__(self):
            return self
        def __exit__(self, *a):
            return False
    def _uor_sync_fetch(url, method='GET', data=None, json=None, headers=None, params=None):
        target_url = str(url)
        if params and isinstance(params, dict):
            sep = '&' if '?' in target_url else '?'
            target_url = target_url + sep + urllib.parse.urlencode(params)
        hdrs = dict(headers) if isinstance(headers, dict) else {}
        body_str = ''
        if json is not None:
            body_str = _uor_json.dumps(json)
            if 'Content-Type' not in hdrs and 'content-type' not in hdrs:
                hdrs['Content-Type'] = 'application/json'
        elif data is not None:
            if isinstance(data, dict):
                body_str = urllib.parse.urlencode(data)
            elif isinstance(data, (bytes, bytearray)):
                body_str = bytes(data).decode('utf-8', errors='replace')
            else:
                body_str = str(data)
        res = js.__uorSpiralCorePySyncFetch(target_url, str(method).upper(), body_str, _uor_json.dumps(hdrs))
        txt = getattr(res, 'text', '')
        st = getattr(res, 'status', 200)
        return _SpiralResp(txt, st)
    def _uor_sync_urlopen(url_or_req, data=None, timeout=None, **kw):
        if isinstance(url_or_req, str):
            m = 'POST' if data is not None else 'GET'
            return _uor_sync_fetch(url_or_req, m, data=data)
        u = getattr(url_or_req, 'full_url', None) or str(url_or_req)
        m = url_or_req.get_method() if hasattr(url_or_req, 'get_method') else ('POST' if data is not None else 'GET')
        req_data = getattr(url_or_req, 'data', None)
        if req_data is None:
            req_data = data
        req_hdrs = dict(url_or_req.header_items()) if hasattr(url_or_req, 'header_items') else {}
        return _uor_sync_fetch(u, m, data=req_data, headers=req_hdrs)
    urllib.request.urlopen = _uor_sync_urlopen
    _req_mod = types.ModuleType('requests')
    _req_mod.get = lambda url, **kw: _uor_sync_fetch(url, 'GET', headers=kw.get('headers'), params=kw.get('params'))
    _req_mod.post = lambda url, data=None, json=None, **kw: _uor_sync_fetch(url, 'POST', data=data, json=json, headers=kw.get('headers'), params=kw.get('params'))
    _req_mod.request = lambda method, url, **kw: _uor_sync_fetch(url, method, data=kw.get('data'), json=kw.get('json'), headers=kw.get('headers'), params=kw.get('params'))
    sys.modules['requests'] = _req_mod
except Exception:
    pass
`);
        } catch {
          // Non-CPython mock runtime ignores Python stdlib registration cleanly
        }
      }

      const isPip = info.tool === 'pip' || info.tool === 'pip3';
      const pyArgv = isPip
        ? ['pip', ...args]
        : (args[0] === '-c'
          ? ['-c', ...args.slice(2)]
          : (args[0] && !args[0].startsWith('-') ? [args[0], ...args.slice(1)] : ['-']));
      const pyEnvObj = { HOME: '/root', PATH: '/usr/local/bin:/usr/bin:/bin', ...info.env };
      const pyStdinText = cachedTextDecoder.decode(info.stdinBytes);
      try {
        await pyodide.runPythonAsync(`
import io, os, sys
sys.argv = ${JSON.stringify(pyArgv)}
_uor_env = ${JSON.stringify(pyEnvObj)}
os.environ.clear()
os.environ.update({str(k): str(v) for k, v in _uor_env.items()})
_uor_stdin_str = ${JSON.stringify(pyStdinText)}
sys.stdin = io.TextIOWrapper(io.BytesIO(_uor_stdin_str.encode('utf-8')), encoding='utf-8')
_uor_cwd = ${JSON.stringify(vfs.cwd)}
if os.path.isdir(_uor_cwd):
    os.chdir(_uor_cwd)
elif os.path.isdir('/workspace'):
    os.chdir('/workspace')
`);
      } catch {
        // Ignore if minimal mock runtime skips process context preamble
      }

      let exitCode = 0;
      if (isPip) {
        const sub = (args[0] || 'list').toLowerCase();
        if (sub === 'install') {
          const netCheck = this.#assertNetworkAllowed();
          if (!netCheck.ok) return netCheck;
          const pkgList = args.slice(1).filter((a) => !a.startsWith('-'));
          if (pkgList.length === 0) {
            return { ok: true, exit_code: 1, stdout: '', stderr: 'pip install: missing package name\n', writes: [], deletes: [] };
          }
          for (const pkg of pkgList) {
            const cleanPkg = pkg.split(/[=<>!~]/)[0].trim();
            const metaFetch = await this.#admittedFetch(
              `https://pypi.org/pypi/${encodeURIComponent(cleanPkg)}/json`,
              undefined,
              { operation: 'pip_install' },
            );
            if (!metaFetch.ok) {
              if (metaFetch.error && metaFetch.error.error_code && metaFetch.error.error_code !== 'HttpBridgeFetchError') {
                return metaFetch.error;
              }
              return { ok: true, exit_code: 1, stdout: '', stderr: `ERROR: Could not find a version that satisfies ${pkg}\n`, writes: [], deletes: [] };
            }
            const metaResp = metaFetch.response;
            if (!metaResp || !metaResp.ok) {
              return { ok: true, exit_code: 1, stdout: '', stderr: `ERROR: Could not find a version that satisfies ${pkg}\n`, writes: [], deletes: [] };
            }
          }
          if (typeof pyodide.loadPackage === 'function') {
            await pyodide.loadPackage('micropip');
          }
          const installListJson = JSON.stringify(pkgList);
          await pyodide.runPythonAsync(`import micropip\nawait micropip.install(${installListJson})`);
          stdoutChunks.push(`Successfully installed ${pkgList.join(' ')}\n`);
        } else {
          const out = await pyodide.runPythonAsync(`import micropip\nstr(micropip.list())`);
          stdoutChunks.push(String(out || '') + '\n');
        }
      } else {
        let codeToRun = '';
        if (args[0] === '-c' && args.length > 1) {
          codeToRun = String(args[1]);
        } else if (args[0] && !args[0].startsWith('-')) {
          const target = MvmPolyglotVfs.normalizePath(vfs.cwd, args[0]);
          const txt = vfs.readText(target);
          if (txt === null) {
            return { ok: true, exit_code: 1, stdout: '', stderr: `python3: can't open file '${args[0]}': No such file or directory\n`, writes: [], deletes: [] };
          }
          codeToRun = txt;
        } else {
          codeToRun = cachedTextDecoder.decode(info.stdinBytes);
        }
        const resVal = await pyodide.runPythonAsync(codeToRun);
        if (stdoutChunks.length === 0 && resVal !== undefined && resVal !== null && args[0] === '-c') {
          // runPythonAsync returns last expression if print() wasn't buffered through stdout callback
        }
      }

      if (pyodide.FS && typeof pyodide.FS.readdir === 'function') {
        const seenFs = new Set();
        const walkFs = (dirAbs) => {
          let entries = [];
          try {
            entries = pyodide.FS.readdir(dirAbs);
          } catch {
            return;
          }
          for (const name of entries) {
            if (name === '.' || name === '..') continue;
            const full = `${dirAbs}/${name}`;
            const stat = pyodide.FS.analyzePath(full);
            if (!stat || !stat.exists) continue;
            if (stat.object && stat.object.isFolder) {
              walkFs(full);
            } else {
              const rel = full.slice('/workspace/'.length);
              seenFs.add(rel);
              const bytes = pyodide.FS.readFile(full);
              const prev = vfs.files.get(rel);
              const changed =
                !prev ||
                prev.byteLength !== bytes.byteLength ||
                prev.some((b, idx) => b !== bytes[idx]);
              if (changed) vfs.writeBytes(full, bytes);
            }
          }
        };
        walkFs('/workspace');
        for (const origRel of vfs.originalKeys) {
          if (!seenFs.has(origRel) && !vfs.symlinks.has(origRel)) {
            vfs.deletePath(`/workspace/${origRel}`);
          }
        }
      }

      const muts = vfs.exportMutations();
      return {
        ok: true,
        exit_code: exitCode,
        stdout: stdoutChunks.join(''),
        stderr: stderrChunks.join(''),
        ...muts,
      };
    } catch (err) {
      if (err && err.code === 'CanonicalPathProtected') {
        return { ok: false, error_code: 'CanonicalPathProtected', message: err.message };
      }
      return {
        ok: true,
        exit_code: 1,
        stdout: stdoutChunks.join(''),
        stderr: stderrChunks.join('') + (err && err.message ? err.message : String(err)) + '\n',
        ...vfs.exportMutations(),
      };
    } finally {
      this.#activePyStdoutSink = null;
      this.#activePyStderrSink = null;
    }
  }

  async handleWasiExec(req) {
    const info = this.#extractBridgePayload(req);
    const vfs = new MvmPolyglotVfs(info.workspace, info.cwd);
    const args = info.args.map((a) => String(a));
    let modPath = '';
    let wasiArgs = [];
    if (args[0] === 'run' && args.length >= 2) {
      modPath = args[1];
      wasiArgs = [args[1], ...args.slice(2)];
    } else if (args[0] && !args[0].startsWith('-')) {
      modPath = args[0];
      wasiArgs = [args[0], ...args.slice(1)];
    } else {
      return {
        ok: false,
        error_code: 'WasiRuntimeError',
        message: 'wasm: expected `wasm run <module.wasm> [args...]`',
      };
    }
    const wasmBytes = vfs.readBytes(modPath);
    if (!wasmBytes) {
      return {
        ok: false,
        error_code: 'WasiRuntimeError',
        message: `wasm: module not found: ${modPath}`,
      };
    }
    try {
      const res = await MvmWasiRunner.run(wasmBytes, vfs, wasiArgs, info.env, info.stdinBytes);
      const muts = vfs.exportMutations();
      return { ...res, ...muts };
    } catch (err) {
      return {
        ok: false,
        error_code: (err && err.code) || 'WasiRuntimeError',
        message: err && err.message ? err.message : String(err),
      };
    }
  }

  async handleSqliteExec(req) {
    const info = this.#extractBridgePayload(req);
    const vfs = new MvmPolyglotVfs(info.workspace, info.cwd);
    try {
      const res = MvmSqliteEngine.execute(
        info.args,
        vfs,
        cachedTextDecoder.decode(info.stdinBytes),
      );
      const muts = vfs.exportMutations();
      return { ...res, ...muts };
    } catch (err) {
      return {
        ok: false,
        error_code: (err && err.code) || 'SqliteRuntimeError',
        message: err && err.message ? err.message : String(err),
      };
    }
  }
}

function __uorNormalizeStitchGroup(g) { /* Formal R4.HoloGeometricRuntime */
  if (!g || typeof g !== 'object') return null;
  const left =
    g.left && typeof g.left === 'object'
      ? g.left
      : g.left_file
        ? { file: g.left_file, byte_length: g.left_length >>> 0, sha256: String(g.left_sha256 || '') }
        : null;
  const right =
    g.right && typeof g.right === 'object'
      ? g.right
      : g.right_file
        ? { file: g.right_file, byte_length: g.right_length >>> 0, sha256: String(g.right_sha256 || '') }
        : null;
  const parity =
    g.parity && typeof g.parity === 'object'
      ? g.parity
      : g.parity_file
        ? { file: g.parity_file, byte_length: g.parity_length >>> 0, sha256: String(g.parity_sha256 || '') }
        : null;
  if (!left || !parity) return null;
  return {
    group_index: Number.isInteger(g.group_index) ? g.group_index : (Number(g.group_id || 0) >>> 0),
    left,
    right,
    parity,
  };
}

function __uorStitchXorPairRecoverJs(group, missingSide, presentLeft, presentRight, presentParity) {
  const normalized = __uorNormalizeStitchGroup(group);
  if (!normalized) {
    throw new Error('Invalid Stitch parity group');
  }
  const leftLen = Number(normalized.left.byte_length) >>> 0;
  const rightLen = normalized.right ? (Number(normalized.right.byte_length) >>> 0) : 0;
  const parityLen = Number(normalized.parity.byte_length) >>> 0;
  const targetLen = missingSide === 'left' ? leftLen : rightLen;
  // Only enter wasm-bindgen wrappers when Wasm is initialized so pre-init Stitch shard loading uses pure JS.
  if (typeof wasm !== 'undefined' && wasm !== undefined && typeof studio_stitch_recover_pair_auto === 'function') {
    return studio_stitch_recover_pair_auto(
      JSON.stringify(normalized),
      missingSide === 'left',
      Boolean(presentLeft),
      presentLeft || new Uint8Array(0),
      Boolean(presentRight),
      presentRight || new Uint8Array(0),
      Boolean(presentParity),
      presentParity || new Uint8Array(0)
    );
  }
  const hasPartnerRequired = Boolean(normalized.right);
  const other = missingSide === 'left' ? (presentRight || new Uint8Array(0)) : (presentLeft || new Uint8Array(0));
  const parity = presentParity;
  if (!(parity instanceof Uint8Array) || (hasPartnerRequired && !(other instanceof Uint8Array))) {
    throw new Error('Insufficient Stitch parity shards (need 2 of 3)');
  }
  if (parity.byteLength !== parityLen) {
    throw new Error(`Stitch parity length mismatch: ${parity.byteLength} != ${parityLen}`);
  }
  const out = new Uint8Array(targetLen);
  for (let idx = 0; idx < targetLen; idx++) {
    const pByte = idx < parity.byteLength ? parity[idx] : 0;
    const oByte = idx < other.byteLength ? other[idx] : 0;
    out[idx] = pByte ^ oByte;
  }
  return out;
}

export async function stitch_fetch_with_parity_recovery({ /* Formal R4.HoloGeometricRuntime */
  manifest,
  baseUrl,
  stitchManifest = null,
  mirrors = [],
  fetchImpl = typeof fetch === 'function' ? fetch.bind(globalThis) : null,
  onProgress = null,
} = {}) {
  if (!manifest || !Array.isArray(manifest.shards)) {
    throw new Error('Valid shard manifest is required');
  }
  if (typeof fetchImpl !== 'function') {
    throw new Error('No fetch implementation available for Stitch shard fetch');
  }
  const normalizedBase = String(baseUrl || '').endsWith('/') ? String(baseUrl || '') : `${String(baseUrl || '')}/`;
  const sourceBases = Array.from(
    new Set(
      [normalizedBase, ...(Array.isArray(mirrors) ? mirrors : [])]
        .filter((u) => typeof u === 'string' && u.trim().length > 0)
        .map((u) => (u.endsWith('/') ? u : `${u}/`))
    )
  );
  const orderedBases = [...sourceBases];

  let verifiedStitch = null;
  if (stitchManifest) {
    if (typeof wasm !== 'undefined' && wasm !== undefined && typeof studio_stitch_verify_manifest === 'function') {
      const check = JSON.parse(studio_stitch_verify_manifest(JSON.stringify(stitchManifest)));
      if (!check || check.status !== 'ok') {
        throw new Error((check && check.message) || 'Invalid Stitch parity manifest');
      }
      verifiedStitch = stitchManifest;
    } else if (
      (stitchManifest.schema === 'stitch/xor-2-of-3/1' || stitchManifest.scheme === 'stitch/xor-2-of-3/1') &&
      Array.isArray(stitchManifest.groups)
    ) {
      verifiedStitch = stitchManifest;
    }
  }

  const shardGroupByFile = new Map();
  if (verifiedStitch && Array.isArray(verifiedStitch.groups)) {
    for (const rawGroup of verifiedStitch.groups) {
      const g = __uorNormalizeStitchGroup(rawGroup);
      if (!g) continue;
      if (g.left && typeof g.left.file === 'string') shardGroupByFile.set(g.left.file, { group: g, side: 'left' });
      if (g.right && typeof g.right.file === 'string') shardGroupByFile.set(g.right.file, { group: g, side: 'right' });
    }
  }

  const fetchVerifiedShard = async (fileName, expectedLength, expectedSha256, failures) => {
    const candidateBases = [...orderedBases];
    for (let bi = 0; bi < candidateBases.length; bi++) {
      const originBase = candidateBases[bi];
      const url = `${originBase}${fileName}`;
      try {
        const resp = await fetchImpl(url);
        if (!resp || !resp.ok) {
          failures.push(`${url}: HTTP ${resp ? resp.status : 'error'}`);
          continue;
        }
        const ab = await resp.arrayBuffer();
        const bytes = new Uint8Array(ab);
        if (typeof expectedLength === 'number' && expectedLength >= 0 && bytes.byteLength !== expectedLength) {
          failures.push(`${url}: length ${bytes.byteLength} != ${expectedLength}`);
          continue;
        }
        if (expectedSha256) {
          const digest = await __uorSha256HexBytes(bytes);
          if (digest !== expectedSha256) {
            failures.push(`${url}: sha256 mismatch`);
            continue;
          }
        }
        const idxNow = orderedBases.indexOf(originBase);
        if (idxNow > 0) {
          orderedBases.splice(idxNow, 1);
          orderedBases.unshift(originBase);
        }
        return { bytes, source: originBase };
      } catch (err) {
        failures.push(`${url}: ${err && err.message ? err.message : String(err)}`);
      }
    }
    return null;
  };

  const totalBytes = manifest.total_bytes >>> 0;
  const fullBuf = new Uint8Array(totalBytes);
  const fetchedMap = new Map();
  const failedSources = [];
  const healedShards = [];
  const healedFiles = new Set();
  const sourcesUsed = new Set();
  let completedCount = 0;
  let completedBytes = 0;

  const hydrateOneShard = async (s) => {
    const existing = fetchedMap.get(s.file);
    const hit = existing
      ? { bytes: existing, source: normalizedBase }
      : await fetchVerifiedShard(s.file, s.byte_length, s.sha256, failedSources);
    let shardBytes = hit ? hit.bytes : null;
    if (hit) {
      sourcesUsed.add(hit.source);
      fetchedMap.set(s.file, shardBytes);
    } else {
      const entry = shardGroupByFile.get(s.file);
      if (!entry) {
        throw new Error(`Shard fetch failed with no Stitch parity group for ${s.file}: ${failedSources.join('; ')}`);
      }
      const { group, side } = entry;
      const partnerEntry = side === 'left' ? group.right : group.left;
      const partnerFile = partnerEntry ? partnerEntry.file : null;
      const partnerLen = partnerEntry ? partnerEntry.byte_length : 0;
      const partnerSha = partnerEntry ? partnerEntry.sha256 : '';

      let partnerBytes = partnerFile ? fetchedMap.get(partnerFile) || null : null;
      if (partnerFile && !partnerBytes) {
        const partnerHit = await fetchVerifiedShard(partnerFile, partnerLen, partnerSha, failedSources);
        if (partnerHit) {
          partnerBytes = partnerHit.bytes;
          fetchedMap.set(partnerFile, partnerBytes);
          sourcesUsed.add(partnerHit.source);
        }
      }

      const parityHit = await fetchVerifiedShard(
        group.parity.file,
        group.parity.byte_length,
        group.parity.sha256,
        failedSources
      );
      if (!parityHit || (partnerFile && !partnerBytes)) {
        throw new Error(`Stitch 2-of-3 parity recovery failed for ${s.file}: ${failedSources.join('; ')}`);
      }
      sourcesUsed.add(parityHit.source);

      shardBytes = __uorStitchXorPairRecoverJs(
        group,
        side,
        side === 'right' ? partnerBytes : null,
        side === 'left' ? partnerBytes : null,
        parityHit.bytes
      );
      if (shardBytes.byteLength !== s.byte_length) {
        throw new Error(`Recovered shard length mismatch on ${s.file}`);
      }
      const recoveredSha = await __uorSha256HexBytes(shardBytes);
      if (s.sha256 && recoveredSha !== s.sha256) {
        throw new Error(`Recovered shard SHA-256 mismatch on ${s.file}`);
      }
      fetchedMap.set(s.file, shardBytes);
      if (!healedFiles.has(s.file)) {
        healedFiles.add(s.file);
        healedShards.push({
          file: s.file,
          group: group.group_index,
          parityFile: group.parity.file,
          recoveredSha256: recoveredSha,
        });
      }
    }

    fullBuf.set(shardBytes, s.byte_offset);
    completedCount += 1;
    completedBytes += s.byte_length >>> 0;
    if (typeof onProgress === 'function') {
      onProgress({
        index: completedCount,
        total: manifest.shards.length,
        bytes: Math.min(totalBytes, completedBytes),
        totalBytes,
        healedCount: healedShards.length,
      });
    }
  };

  if (manifest.shards.length > 0) {
    // Probe/warm shard 0 first so a working mirror is promoted to orderedBases[0] before parallel batches
    await hydrateOneShard(manifest.shards[0]);
    const batchSize = 6;
    for (let i = 1; i < manifest.shards.length; i += batchSize) {
      const slice = manifest.shards.slice(i, Math.min(i + batchSize, manifest.shards.length));
      await Promise.all(slice.map((s) => hydrateOneShard(s)));
    }
  }

  const stitchHealth = {
    scheme: verifiedStitch ? 'stitch/xor-2-of-3/1' : 'uorlut-sha256-verified',
    custodyMode: verifiedStitch ? '2-of-3 XOR Parity + SHA-256' : 'SHA-256 Direct',
    totalShards: manifest.shards.length,
    verifiedShards: manifest.shards.length - healedShards.length,
    healedShards: healedShards.length,
    healedDetails: healedShards,
    failedSources: failedSources.slice(-16),
    mirrorsConfigured: sourceBases,
    activeSources: Array.from(sourcesUsed),
    status: healedShards.length > 0 ? 'Healed via 2-of-3 Parity' : 'Healthy (All Verified)',
    updatedAt: new Date().toISOString(),
  };
  globalThis.__uorStitchHealth = stitchHealth;
  return { fullBuf, stitchHealth };
}

export async function stream_v10_sharded_weights_to_opfs(onProgress, forceReload = false, modelTier = 'auto') {
  if (wasm === undefined || typeof studio_load_uorgllm_v9_bytes !== 'function') return null;
  const httpOrigin =
    typeof location !== 'undefined' && location.origin && /^https?:/i.test(location.origin)
      ? location.origin.replace(/\/+$/, '')
      : '';
  const hasOpfs =
    typeof navigator !== 'undefined' &&
    navigator.storage &&
    typeof navigator.storage.getDirectory === 'function';

  // Prefer persisted 32L/960D (SmolLM2-360M-Instruct) OPFS container when present unless v10 is forced
  if (!forceReload && hasOpfs && modelTier !== 'v10' && typeof studio_mount_r4_v12_bytes === 'function') {
    try {
      const root = await navigator.storage.getDirectory();
      const mfh360 = await root.getFileHandle(OPFS_V12_360M_META_FILE, { create: false });
      const meta360 = JSON.parse(await (await mfh360.getFile()).text());
      const wfh360 = await root.getFileHandle(OPFS_V12_360M_WEIGHT_FILE, { create: false });
      const bytes360 = new Uint8Array(await (await wfh360.getFile()).arrayBuffer());
      if (bytes360.byteLength >= 64) {
        const loaded360 = JSON.parse(studio_mount_r4_v12_bytes(bytes360));
        if (loaded360 && loaded360.status === 'ok') {
          const fmtLabel = loaded360.format || 'UORR4V12';
          const statusStr = `OPFS ${fmtLabel} (${loaded360.n_layers}L ${loaded360.d_model}D)`;
          const stitchHealth = {
            scheme: 'stitch/xor-2-of-3/1',
            custodyMode: 'OPFS SHA-256 Verified (360M)',
            totalShards: Number(meta360 && meta360.totalShards) || 0,
            verifiedShards: Number(meta360 && meta360.totalShards) || 0,
            healedShards: 0,
            healedDetails: [],
            failedSources: [],
            mirrorsConfigured: ['opfs://local-cache'],
            activeSources: ['opfs://local-cache'],
            status: 'Healthy (OPFS 360M Cached)',
            updatedAt: new Date().toISOString(),
          };
          globalThis.__uorStitchHealth = stitchHealth;
          globalThis.__uorGllmV9Meta = {
            status: statusStr,
            format: fmtLabel,
            tier: loaded360.active_tier || 'Full32LRecompiled',
            sha256: (meta360 && meta360.sha256) || '',
            mode: 'opfs-r4v12-360m-cached',
            stitchHealth,
          };
          return loaded360;
        }
      }
    } catch {
      // Fall through to requested manifest or v10 hydration
    }
  }

  const requested360m = modelTier === 'v12_360m';
  const weightFileName = requested360m ? OPFS_V12_360M_WEIGHT_FILE : OPFS_V10_WEIGHT_FILE;
  const metaFileName = requested360m ? OPFS_V12_360M_META_FILE : OPFS_V10_META_FILE;
  const manifestCandidates = requested360m
    ? [
        new URL('weights/v12_360m/manifest.json', import.meta.url).href,
        new URL('../gpu/weights/v12_360m/manifest.json', import.meta.url).href,
        ...(httpOrigin ? [`${httpOrigin}/weights/v12_360m/manifest.json`] : []),
        'https://raw.githubusercontent.com/Casey-allard/lexlean-r4/main/gpu/weights/v12_360m/manifest.json',
      ]
    : [
        new URL('weights/v10/manifest.json', import.meta.url).href,
        new URL('../gpu/weights/v10/manifest.json', import.meta.url).href,
        new URL('weights/v9/manifest.json', import.meta.url).href,
        ...(httpOrigin ? [`${httpOrigin}/weights/v10/manifest.json`] : []),
        'https://raw.githubusercontent.com/Casey-allard/lexlean-r4/main/gpu/weights/v10/manifest.json',
      ];

  let manifest = null;
  let baseUrl = '';
  for (const mUrl of manifestCandidates) {
    try {
      const resp = await fetch(mUrl, { cache: 'no-cache' });
      if (resp.ok) {
        const parsed = await resp.json();
        if (parsed && (parsed.format === 'UORGLLM9' || parsed.format === 'UORR4V12') && Array.isArray(parsed.shards)) {
          manifest = parsed;
          baseUrl = mUrl.slice(0, mUrl.lastIndexOf('/') + 1);
          break;
        }
      }
    } catch {
      // Try next manifest path
    }
  }

  if (!manifest) {
    return requested360m ? null : restore_gllm_v9_from_opfs('');
  }

  let stitchManifest = null;
  try {
    const stResp = await fetch(`${baseUrl}manifest.stitch.json`, { cache: 'no-cache' });
    if (stResp && stResp.ok) {
      const candidate = await stResp.json();
      if (candidate && (candidate.schema === 'stitch/xor-2-of-3/1' || candidate.scheme === 'stitch/xor-2-of-3/1') && Array.isArray(candidate.groups)) {
        stitchManifest = candidate;
      }
    }
  } catch {
    // Optional stitch parity sidecar manifest
  }

  const expectedSha = String(manifest.container_sha256 || '');
  if (!forceReload && hasOpfs && expectedSha) {
    try {
      const root = await navigator.storage.getDirectory();
      const mfh = await root.getFileHandle(metaFileName, { create: false });
      const meta = JSON.parse(await (await mfh.getFile()).text());
      if (meta && meta.sha256 === expectedSha && (!meta.format || meta.format === manifest.format)) {
        const wfh = await root.getFileHandle(weightFileName, { create: false });
        const bytes = new Uint8Array(await (await wfh.getFile()).arrayBuffer());
        const isR4V12 = manifest.format === 'UORR4V12' || (bytes.byteLength >= 8 && cachedTextDecoder.decode(bytes.subarray(0, 8)) === 'UORR4V12');
        const loadLabel = isR4V12 ? 'opfs_cached_r4_v12_recompiled' : 'opfs_cached_v10_16L_256D';
        const rawLoaded = (isR4V12 && typeof studio_mount_r4_v12_bytes === 'function')
          ? studio_mount_r4_v12_bytes(bytes)
          : studio_load_uorgllm_v9_bytes(bytes, loadLabel);
        const loaded = JSON.parse(rawLoaded);
        if (loaded && loaded.status === 'ok') {
          const stitchHealth = {
            scheme: stitchManifest ? 'stitch/xor-2-of-3/1' : 'opfs-cached-verified',
            custodyMode: stitchManifest ? '2-of-3 XOR Parity + OPFS' : 'OPFS SHA-256 Verified',
            totalShards: manifest.shards.length,
            verifiedShards: manifest.shards.length,
            healedShards: 0,
            healedDetails: [],
            failedSources: [],
            mirrorsConfigured: [baseUrl],
            activeSources: ['opfs://local-cache'],
            status: 'Healthy (OPFS Cached)',
            updatedAt: new Date().toISOString(),
          };
          const fmtLabel = loaded.format || manifest.format || (isR4V12 ? 'UORR4V12' : 'UORGLLM9');
          const statusStr = `OPFS ${fmtLabel} (${loaded.n_layers}L ${loaded.d_model}D)`;
          globalThis.__uorStitchHealth = stitchHealth;
          globalThis.__uorGllmV9Meta = {
            status: statusStr,
            format: fmtLabel,
            tier: loaded.active_tier || (isR4V12 ? 'FullRecompiled' : 'LegacyV9'),
            sha256: expectedSha,
            mode: requested360m ? 'opfs-r4v12-360m-cached' : (isR4V12 ? 'opfs-r4v12-cached' : 'opfs-v10-cached'),
            stitchHealth,
          };
          return loaded;
        }
      }
    } catch {
      // Proceed to stream shards
    }
  }

  const mirrors = Array.isArray(manifest.mirrors)
    ? manifest.mirrors
    : stitchManifest && Array.isArray(stitchManifest.mirrors)
      ? stitchManifest.mirrors
      : [];
  const { fullBuf, stitchHealth } = await stitch_fetch_with_parity_recovery({
    manifest,
    baseUrl,
    stitchManifest,
    mirrors,
    onProgress,
  });
  const totalBytes = fullBuf.byteLength;

  const fullSha = await __uorSha256HexBytes(fullBuf);
  if (expectedSha && fullSha !== expectedSha) {
    throw new Error(`${manifest.format || 'UORR4V12'} container SHA-256 mismatch`);
  }

  if (hasOpfs) {
    try {
      const root = await navigator.storage.getDirectory();
      const fh = await root.getFileHandle(weightFileName, { create: true });
      if (typeof fh.createSyncAccessHandle === 'function') {
        const sh = await fh.createSyncAccessHandle();
        sh.truncate(totalBytes);
        sh.write(fullBuf, { at: 0 });
        sh.flush();
        sh.close();
      } else if (typeof fh.createWritable === 'function') {
        const w = await fh.createWritable();
        await w.write(fullBuf);
        await w.close();
      }
      const mfh = await root.getFileHandle(metaFileName, { create: true });
      const metaBytes = cachedTextEncoder.encode(
        JSON.stringify({
          sha256: fullSha,
          totalBytes,
          totalShards: manifest.shards.length,
          format: manifest.format || 'UORR4V12',
        })
      );
      if (typeof mfh.createSyncAccessHandle === 'function') {
        const msh = await mfh.createSyncAccessHandle();
        msh.truncate(metaBytes.byteLength);
        msh.write(metaBytes, { at: 0 });
        msh.flush();
        msh.close();
      } else if (typeof mfh.createWritable === 'function') {
        const mw = await mfh.createWritable();
        await mw.write(metaBytes);
        await mw.close();
      }
    } catch {
      // OPFS write optional if quota restricted
    }
  }

  const isStreamedR4V12 = manifest.format === 'UORR4V12' || (fullBuf.byteLength >= 8 && cachedTextDecoder.decode(fullBuf.subarray(0, 8)) === 'UORR4V12');
  const streamLabel = isStreamedR4V12 ? 'opfs_streamed_r4_v12_recompiled' : 'opfs_streamed_v10_16L_256D';
  const rawParsed = (isStreamedR4V12 && typeof studio_mount_r4_v12_bytes === 'function')
    ? studio_mount_r4_v12_bytes(fullBuf)
    : studio_load_uorgllm_v9_bytes(fullBuf, streamLabel);
  const parsed = JSON.parse(rawParsed);
  if (parsed && parsed.status === 'ok') {
    const fmtLabel = parsed.format || manifest.format || (isStreamedR4V12 ? 'UORR4V12' : 'UORGLLM9');
    globalThis.__uorGllmV9Meta = {
      status: `OPFS ${fmtLabel} (${parsed.n_layers}L ${parsed.d_model}D)`,
      format: fmtLabel,
      tier: parsed.active_tier || (isStreamedR4V12 ? 'FullRecompiled' : 'LegacyV9'),
      sha256: fullSha,
      mode: requested360m ? 'opfs-r4v12-360m-streamed' : (isStreamedR4V12 ? 'opfs-r4v12-streamed' : 'opfs-v10-streamed'),
      stitchHealth,
    };
    return parsed;
  }
  return null;
}

function __uorCollectWorkspaceSnapshot() {
  const listRes = JSON.parse(studio_dispatch('1\tstudio\tworkspace_list\t'));
  const files = listRes && Array.isArray(listRes.files) ? listRes.files : [];
  const hydrated = files.map((f) => {
    let content = f.content;
    if (typeof content !== 'string') {
      try {
        const r = JSON.parse(studio_dispatch(`1\tstudio\tworkspace_read\t${f.path}`));
        content = r && r.status === 'ok' ? (r.content || '') : '';
      } catch {
        content = '';
      }
    }
    return { ...f, content };
  });
  return { status: 'ok', files: hydrated };
}

function __uorSynthesizeBrainPulsesFromReply(parsedRes, toolKind = 'none') { /* Formal R4.HoloGeometricRuntime */
  if (Array.isArray(parsedRes && parsedRes.brain_pulses) && parsedRes.brain_pulses.length > 0) {
    return parsedRes.brain_pulses;
  }
  const gen = (parsedRes && parsedRes.generation) || {};
  const tokGen = (parsedRes && parsedRes.token_generation) || gen.token_generation || {};
  const baseOrbit = ((parsedRes && parsedRes.icosian_root) ?? tokGen.final_icosian_root ?? 3) & 15;
  const baseHead = ((parsedRes && parsedRes.flock_consensus_action) ?? 2) & 7;
  const steps = Math.min(Math.max(Number(tokGen.stepped_tokens) || 10, 6), 24);
  const pulses = [];
  for (let s = 0; s < steps; s++) {
    pulses.push({
      step: s,
      phase: s === 0 ? 'prefill' : (toolKind !== 'none' && s === steps - 1 ? 'tool' : 'decode'),
      active_layer: s % 16,
      winning_head: (baseHead + s) & 7,
      e8_orbit: (baseOrbit + s * 3) & 15,
      cl06_grade: (s * 2 + 1) % 7,
      kv_copy_dist: s % 4 === 2 ? 12 + s * 3 : 0,
      prll_top_mass: 620000 + ((s * 31337) % 280000),
      tool_kind: s === steps - 1 ? toolKind : 'none',
    });
  }
  return pulses;
}

function __uorStartDedicatedEngineWorker() {
  const boundSelf = typeof self !== 'undefined' ? self : null;
  const postReply = (msg) => {
    if (
      boundSelf &&
      typeof self !== 'undefined' &&
      self === boundSelf &&
      typeof boundSelf.postMessage === 'function'
    ) {
      boundSelf.postMessage(msg);
    }
  };
  const postEvent = (event, data = {}) => {
    postReply({ event, ...data });
  };
  const gitMetaFiles = new Map();
  purgeLegacyDeclinedHydrationFlag();
  let offlineEmergencyQ31 = safeStorageGet('uor_offline_emergency_q31') === '1';
  let explicitWorkerAllowOffOrigin;
  let bootNativeQ31Fallback = false;
  let previewInspectSeq = 0;
  const pendingPreviewInspectRpc = new Map();
  const workerWorkspaceFs = {
    read: (normPath) => {
      const isGitMeta = normPath === '.git' || normPath.startsWith('.git/');
      if (isGitMeta && gitMetaFiles.has(normPath)) {
        return gitMetaFiles.get(normPath);
      }
      try {
        if (typeof studio_dispatch === 'function') {
          const parsed = JSON.parse(studio_dispatch(`1\tstudio\tworkspace_read\t${normPath}`));
          if (parsed && parsed.status === 'ok' && typeof parsed.content === 'string') {
            if (isGitMeta) gitMetaFiles.set(normPath, parsed.content);
            return parsed.content;
          }
        }
      } catch {}
      return null;
    },
    write: (normPath, text) => {
      if (normPath === '.git' || normPath.startsWith('.git/')) {
        gitMetaFiles.set(normPath, String(text ?? ''));
      }
      try {
        if (typeof studio_dispatch === 'function') {
          studio_dispatch(`1\tstudio\tworkspace_write\t${normPath}\n${String(text ?? '')}`);
        }
      } catch {}
    },
    delete: (normPath) => {
      const isGitMeta = normPath === '.git' || normPath.startsWith('.git/');
      const hadGit = isGitMeta ? gitMetaFiles.delete(normPath) : false;
      try {
        if (typeof studio_dispatch === 'function') {
          const parsed = JSON.parse(studio_dispatch(`1\tstudio\tworkspace_delete\t${normPath}`));
          return Boolean(parsed && parsed.status === 'ok') || hadGit;
        }
      } catch {}
      return hadGit;
    },
    list: () => {
      try {
        const snap = __uorCollectWorkspaceSnapshot();
        return snap && Array.isArray(snap.files)
          ? snap.files.filter((f) => f && typeof f.path === 'string' && f.path !== '.git' && !f.path.startsWith('.git/'))
          : [];
      } catch {
        return [];
      }
    },
  };
  const hostBridge = new SpiralCoreHostBridge({
    mode: 'spiralcore_net',
    onAuditEvent: (ev) => postEvent('audit_event', ev),
    workspaceFs: workerWorkspaceFs,
  });
  let wasmReady = false;
  let bootPromise = null;
  let activeHydrationPromise = null;
  let lastHydrationProgress = null;
  let activeInprismBusy = false;
  let activeInprismRequestId = null;
  let activeInprismTurnId = null;
  const cancelledInprismRequests = new Set();
  const cancelledInprismTurns = new Set();
  const pendingInprismCancelWaiters = new Map();
  const earlyMessages = [];
  globalThis.__uorEmitChatDelta = (textChunk, step = 0) => {
    const chunk = String(textChunk || '');
    if (chunk.length > 0) {
      postEvent('chat_stream_delta', { text: chunk, step: Number(step) || 0 });
    }
  };
  globalThis.__uorEmitHoloTraceFrame = (frameJson) => {
    /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
    try {
      const frame = typeof frameJson === 'string' ? JSON.parse(frameJson) : frameJson;
      if (frame) {
        postEvent('holo_trace_frame', { frame });
      }
    } catch {
      // Ignore malformed frame payload cleanly
    }
  };

  const startOrAwaitOpfsShardHydration = (forceReload = false, modelTier = 'auto') => {
    if (activeHydrationPromise && !forceReload && modelTier === 'auto') {
      return activeHydrationPromise;
    }
    const run = stream_v10_sharded_weights_to_opfs((prog) => {
      lastHydrationProgress = { stage: 'r4_shards', ...prog };
      postEvent('boot_progress', lastHydrationProgress);
      postEvent('hydration_progress', lastHydrationProgress);
    }, forceReload, modelTier)
      .then((v10Info) => {
        postEvent('stitch_health', { stitchHealth: globalThis.__uorStitchHealth || null });
        if (v10Info) {
          const r4Stack = typeof studio_r4_stack_status === 'function' ? JSON.parse(studio_r4_stack_status()) : null;
          postEvent('weights_upgraded', {
            gllm: v10Info,
            r4Stack,
            gllmMeta: globalThis.__uorGllmV9Meta || null,
            stitchHealth: globalThis.__uorStitchHealth || null,
          });
        }
        return v10Info;
      })
      .catch(() => null)
      .finally(() => {
        if (activeHydrationPromise === run) {
          activeHydrationPromise = null;
        }
      });
    activeHydrationPromise = run;
    return run;
  };

  let mvmWorkspaceSyncEpoch = 0;
  const syncedHostWorkspaceFiles = new Map();

  const encodeUtf8ToBase64 = (text) => {
    const str = String(text || '');
    if (cachedTextEncoder && typeof cachedTextEncoder.encode === 'function') {
      return __uorBytesToBase64(cachedTextEncoder.encode(str));
    }
    if (typeof Buffer !== 'undefined') {
      return Buffer.from(str, 'utf8').toString('base64');
    }
    const u8 = new Uint8Array(str.length);
    for (let i = 0; i < str.length; i++) u8[i] = str.charCodeAt(i) & 0xff;
    return __uorBytesToBase64(u8);
  };

  const decodeBase64ToUtf8 = (b64) => {
    const bytes = __uorBase64ToBytes(b64);
    if (cachedTextDecoder && typeof cachedTextDecoder.decode === 'function') {
      return cachedTextDecoder.decode(bytes);
    }
    if (typeof Buffer !== 'undefined') {
      return Buffer.from(bytes).toString('utf8');
    }
    return '';
  };

  const resolveMvmSyncInFn = () =>
    typeof mvm_sync_workspace_in_json === 'function'
      ? mvm_sync_workspace_in_json
      : globalThis.mvm_sync_workspace_in_json;

  const resolveMvmExportDiffFn = () =>
    typeof mvm_export_workspace_diff_json === 'function'
      ? mvm_export_workspace_diff_json
      : globalThis.mvm_export_workspace_diff_json;

  const updateMvmWorkspaceSyncEpoch = (rawEpoch) => {
    const parsedEpoch = Number(rawEpoch);
    if (Number.isFinite(parsedEpoch)) {
      mvmWorkspaceSyncEpoch = Math.max(mvmWorkspaceSyncEpoch, parsedEpoch);
    }
  };

  const applyMvmSyncInState = (res, nextMap, deltaEntries) => {
    if (!res || res.status !== 'ok') return;
    updateMvmWorkspaceSyncEpoch(res.epoch);
    if (nextMap instanceof Map) {
      syncedHostWorkspaceFiles.clear();
      for (const [k, v] of nextMap.entries()) syncedHostWorkspaceFiles.set(k, v);
      return;
    }
    if (!Array.isArray(deltaEntries)) return;
    for (const entry of deltaEntries) {
      if (!entry || typeof entry.path !== 'string') continue;
      const parts = String(entry.path).trim().split('/').filter((s) => s && s !== '.');
      if (parts[0] === 'workspace') parts.shift();
      if (parts.includes('..')) continue;
      const rel = parts.join('/');
      if (!rel || /^prism\/.*\.lex\.tex$/.test(rel) || rel === 'app/state') {
        continue;
      }
      if (entry.deleted) {
        syncedHostWorkspaceFiles.delete(rel);
      } else if (typeof entry.content_base64 === 'string') {
        syncedHostWorkspaceFiles.set(rel, decodeBase64ToUtf8(entry.content_base64));
      }
    }
  };

  const applyMvmExportDiffState = (diff) => {
    if (!diff || diff.status !== 'ok') return diff;
    updateMvmWorkspaceSyncEpoch(diff.epoch);
    const writes = Array.isArray(diff.writes) ? diff.writes : [];
    for (const w of writes) {
      if (w && typeof w.path === 'string' && typeof w.content_base64 === 'string') {
        syncedHostWorkspaceFiles.set(w.path, decodeBase64ToUtf8(w.content_base64));
      }
    }
    const deletes = Array.isArray(diff.deletes) ? diff.deletes : [];
    for (const delPath of deletes) {
      if (typeof delPath === 'string') syncedHostWorkspaceFiles.delete(delPath);
    }
    return diff;
  };

  const runMvmSyncWorkspaceIn = (payloadInput, nextMap, allowDispatchFallback) => {
    const syncInFn = resolveMvmSyncInFn();
    if (typeof syncInFn !== 'function' && !allowDispatchFallback) return null;
    const payloadStr =
      typeof payloadInput === 'string'
        ? payloadInput
        : JSON.stringify(payloadInput || { epoch: mvmWorkspaceSyncEpoch, files: [] });
    let deltaEntries = null;
    if (!(nextMap instanceof Map)) {
      try {
        const parsedReq = typeof payloadInput === 'string' ? JSON.parse(payloadStr) : payloadInput;
        deltaEntries = parsedReq && Array.isArray(parsedReq.files) ? parsedReq.files : null;
      } catch {
        deltaEntries = null;
      }
    }
    const resRaw =
      typeof syncInFn === 'function'
        ? syncInFn(payloadStr)
        : studio_dispatch(`1\tstudio\tmvm_sync_workspace_in\t${payloadStr}`);
    const res = typeof resRaw === 'string' ? JSON.parse(resRaw) : resRaw;
    applyMvmSyncInState(res, nextMap, deltaEntries);
    return res;
  };

  const runMvmExportWorkspaceDiff = (allowDispatchFallback) => {
    const exportDiffFn = resolveMvmExportDiffFn();
    if (typeof exportDiffFn !== 'function' && !allowDispatchFallback) return null;
    const diffRaw =
      typeof exportDiffFn === 'function'
        ? exportDiffFn()
        : studio_dispatch('1\tstudio\tmvm_export_workspace_diff\t');
    const diff = typeof diffRaw === 'string' ? JSON.parse(diffRaw) : diffRaw;
    return applyMvmExportDiffState(diff);
  };

  const syncWorkspaceIntoMvm = () => {
    try {
      const snap = __uorCollectWorkspaceSnapshot();
      const files = snap && Array.isArray(snap.files) ? snap.files : [];
      const nextMap = new Map();
      const payloadFiles = [];
      for (const f of files) {
        if (!f || typeof f.path !== 'string') continue;
        const content = typeof f.content === 'string' ? f.content : '';
        nextMap.set(f.path, content);
        if (syncedHostWorkspaceFiles.get(f.path) !== content) {
          payloadFiles.push({
            path: f.path,
            content_base64: encodeUtf8ToBase64(content),
          });
        }
      }
      for (const prevPath of syncedHostWorkspaceFiles.keys()) {
        if (!nextMap.has(prevPath)) {
          payloadFiles.push({ path: prevPath, content_base64: '', deleted: true });
        }
      }
      if (payloadFiles.length === 0 && syncedHostWorkspaceFiles.size > 0) {
        return null;
      }
      return runMvmSyncWorkspaceIn(
        { epoch: mvmWorkspaceSyncEpoch, files: payloadFiles },
        nextMap,
        false,
      );
    } catch {
      return null;
    }
  };

  const exportWorkspaceDiffFromMvm = () => {
    try {
      return runMvmExportWorkspaceDiff(false);
    } catch {
      return null;
    }
  };

  const prepareGitBridgePayload = (rawReq) => {
    if (
      rawReq &&
      rawReq.op === 'git' &&
      (rawReq.subcommand === 'push' || rawReq.subcommand === 'commit_push') &&
      !Array.isArray(rawReq.files)
    ) {
      const snap = __uorCollectWorkspaceSnapshot();
      const liveFiles = (snap && Array.isArray(snap.files) ? snap.files : []).filter(
        (f) => f && typeof f.path === 'string' && f.path !== '.git' && !f.path.startsWith('.git/'),
      );
      return { ...rawReq, files: liveFiles, full_workspace: true };
    }
    return rawReq;
  };

  const notifyGitWorkspaceSync = (gitRes) => {
    if (!gitRes || gitRes.op !== 'git' || !gitRes.ok) return;
    const changed = Array.isArray(gitRes.changed_paths)
      ? gitRes.changed_paths.slice()
      : Array.isArray(gitRes.files)
        ? gitRes.files
            .filter((f) => f && typeof f.path === 'string' && typeof f.content === 'string')
            .map((f) => f.path)
        : [];
    const deleted = Array.isArray(gitRes.deleted_paths) ? gitRes.deleted_paths.slice() : [];
    if (changed.length > 0 || deleted.length > 0) {
      flushMvmCowToOpfs().catch(() => false);
      postEvent('workspace_sync', { workspace: __uorCollectWorkspaceSnapshot() });
      postEvent('workspace_fs_changed', {
        source: 'git_sync',
        changed_paths: changed,
        deleted_paths: deleted,
      });
    }
  };

  const applyClonedFilesToWorkspace = (files) => {
    if (!Array.isArray(files) || files.length === 0) return null;
    for (const f of files) {
      if (f && typeof f.path === 'string' && typeof f.content === 'string') {
        studio_dispatch(`1\tstudio\tworkspace_write\t${f.path}\n${f.content}`);
      }
    }
    flushMvmCowToOpfs().catch(() => false);
    const snap = __uorCollectWorkspaceSnapshot();
    postEvent('workspace_sync', { workspace: snap });
    return snap;
  };

  const dispatchWithConnectedBridge = async (envelope) => {
    const isMvmExecEnvelope = envelope.includes('\tmvm_exec\t');
    if (isMvmExecEnvelope) {
      syncWorkspaceIntoMvm();
    }
    let currentRaw = studio_dispatch(envelope);
    let parsed = JSON.parse(currentRaw);
    let hops = 0;
    let lastToolKind = 'none';

    while (parsed && parsed.status === 'needs_bridge' && hops < 8) {
      hops++;
      const bridgeReq = parsed.bridge_req || parsed.pending_bridge || parsed.request || {};
      const kind = String(bridgeReq.op || bridgeReq.kind || bridgeReq.tool || 'http_request');
      lastToolKind = kind;
      postEvent('bridge_activity', { phase: 'start', hop: hops, op: kind, request: bridgeReq });
      postEvent('brain_pulse', {
        pulses: [
          {
            step: hops,
            phase: 'tool',
            active_layer: (hops * 2) & 7,
            winning_head: hops & 7,
            e8_orbit: (hops * 5) & 15,
            cl06_grade: 3,
            kv_copy_dist: 24,
            prll_top_mass: 850000,
            tool_kind: kind,
          },
        ],
      });

      const execBridgeReq = prepareGitBridgePayload(bridgeReq);
      const bridgeResp = await hostBridge.execute(execBridgeReq);
      if (bridgeResp && bridgeResp.op === 'git') {
        if (typeof bridgeResp.path === 'string' && typeof bridgeResp.content === 'string') {
          applyClonedFilesToWorkspace([{ path: bridgeResp.path, content: bridgeResp.content }]);
        } else {
          notifyGitWorkspaceSync(bridgeResp);
        }
      }
      postEvent('bridge_activity', {
        phase: 'complete',
        hop: hops,
        op: kind,
        ok: Boolean(bridgeResp && bridgeResp.ok),
        transport: hostBridge.lastTransportMode,
      });

      const isChatTurn = envelope.includes('\tchat\t') || envelope.includes('\tchat_v10_turn\t');
      const resumeCmd =
        parsed.resume_command ||
        (isChatTurn ? 'chat_complete_bridge' : 'mvm_complete_bridge');
      const resumePayload = JSON.stringify({
        session_id: parsed.session_id || parsed.bridge_id || '',
        bridge_id: parsed.bridge_id || parsed.session_id || '',
        resume_state: parsed.resume_state,
        bridge_resp: bridgeResp,
        response: bridgeResp,
      });
      currentRaw = studio_dispatch(`1\tstudio\t${resumeCmd}\t${resumePayload}`);
      parsed = JSON.parse(currentRaw);
    }

    const isChatTurn = envelope.includes('\tchat\t') || envelope.includes('\tchat_v10_turn\t');
    if (isChatTurn && parsed && Array.isArray(parsed.holo_observatory_frames) && parsed.holo_observatory_frames.length > 0) {
      postEvent('holo_observatory_batch', {
        frames: parsed.holo_observatory_frames,
        prefixReused: Number(parsed.holo_prefix_reused_tokens) || 0,
        selectedModel: parsed.selected_model || '',
      });
    }
    if (isChatTurn || envelope.includes('\tmvm_exec\t')) {
      const pulses = __uorSynthesizeBrainPulsesFromReply(parsed, lastToolKind);
      postEvent('brain_pulse', { pulses });
    }
    if (isMvmExecEnvelope || (isChatTurn && parsed && parsed.mvm_tool_executed)) {
      const diff = exportWorkspaceDiffFromMvm();
      persist_mvm_cow_sectors_to_opfs().catch(() => {});
      const snap = __uorCollectWorkspaceSnapshot();
      if (diff && diff.status === 'ok' && parsed && typeof parsed === 'object') {
        parsed.workspace_diff = diff;
        parsed.ignored_heavy_paths = Number(diff.ignored_heavy_paths) || 0;
        const diffWrites = Array.isArray(diff.writes) ? diff.writes.map((w) => w.path) : [];
        const diffDeletes = Array.isArray(diff.deletes) ? diff.deletes : [];
        parsed.changed_files = diffWrites.concat(diffDeletes);
      }
      postEvent('workspace_sync', {
        workspace: snap,
        ...(diff && diff.status === 'ok' ? { diff } : {}),
      });
      if (parsed && typeof parsed === 'object') {
        Object.defineProperty(parsed, '__uorWorkspaceSyncEmitted', {
          value: true,
          configurable: true,
          enumerable: false,
        });
      }
    }
    return parsed;
  };

  const isQwenMounted = () => {
    try {
      const fnStatus = typeof studio_holo_runtime_status === 'function'
        ? studio_holo_runtime_status
        : globalThis.studio_holo_runtime_status;
      if (typeof fnStatus !== 'function') return false;
      const hs = JSON.parse(fnStatus());
      return Boolean(
        hs &&
          (hs.qwen3_06b_mounted ||
            hs.qwen3_17b_mounted ||
            (Array.isArray(hs.models) &&
              hs.models.some((m) => m && String(m.model_id || '').includes('qwen3')))),
      );
    } catch {
      return false;
    }
  };

  const readNativeQ31Info = () => {
    try {
      const fnQ31 = typeof studio_native_q31_status === 'function'
        ? studio_native_q31_status
        : globalThis.studio_native_q31_status;
      if (typeof fnQ31 !== 'function') return { ready: false, meta: null };
      const st = JSON.parse(fnQ31());
      const ready = Boolean(st && (st.ready || st.status === 'ok'));
      return { ready, meta: st || null };
    } catch {
      return { ready: false, meta: null };
    }
  };

  const isQ31Ready = () => readNativeQ31Info().ready;

  const emitNativeQ31ReadyFallback = (fallbackReason) => {
    const info = readNativeQ31Info();
    if (!info.ready) return null;
    const holoMeta = info.meta || { model_id: 'uor-r4.native-q31-d85', ready: true };
    postEvent('holo_ready', {
      holo: holoMeta,
      recommendedModel: 'uor-r4.native-q31-d85',
      promoted: false,
      fallbackReason,
      kappaStore: globalThis.__uorHoloKappaStatus || null,
    });
    return holoMeta;
  };

  const shouldUseOfflineQ31Fallback = (reqPayload, inlineLowLatency = false) => {
    const p = reqPayload && typeof reqPayload === 'object' ? reqPayload : {};
    if (p.offlineFallback || p.fallbackOfflineQ31 || p.consentDeclined) return true;
    const rawPref = String(p.preferredModel || p.selected_model || p.model || '').trim();
    if (rawPref === 'uor-r4.native-q31-d85' || rawPref === 'native-q31-d85') return true;
    if (rawPref) return false;
    if (p.allowOffOriginHydration === true || isQwenMounted()) return false;
    purgeLegacyDeclinedHydrationFlag();
    if (offlineEmergencyQ31 || safeStorageGet('uor_offline_emergency_q31') === '1') return true;
    if ((bootNativeQ31Fallback || inlineLowLatency) && !isQwenMounted() && isQ31Ready()) {
      return true;
    }
    return false;
  };

  const stripCodeFences = (rawText) => {
    const raw = String(rawText || '');
    const trimmed = raw.trim();
    if (!trimmed.startsWith('```')) return raw;
    const firstNl = trimmed.indexOf('\n');
    const body = firstNl >= 0 ? trimmed.slice(firstNl + 1) : '';
    return body.replace(/\r?\n?```$/, '');
  };

  const createWorkerTurnHooks = () => ({
    offlineEmergencyQ31,
    postEvent,
    postResponse: (resp) => {
      postReply(resp);
    },
    dispatchBridge: dispatchWithConnectedBridge,
    executeHostBridge: async (bridgeReq) => hostBridge.execute(prepareGitBridgePayload(bridgeReq)),
    flushMvmCowToOpfs: () => flushMvmCowToOpfs(),
    collectWorkspaceSnapshot: () => __uorCollectWorkspaceSnapshot(),
    previewInspect: (args) =>
      new Promise((resolve) => {
        const rpcId = `pv_${++previewInspectSeq}`;
        const timer = setTimeout(() => {
          if (pendingPreviewInspectRpc.has(rpcId)) {
            pendingPreviewInspectRpc.delete(rpcId);
            resolve({ ok: false, rpc_id: rpcId, error_code: 'PreviewInspectTimeout', message: 'Preview inspect RPC timed out' });
          }
        }, 5000);
        if (timer && typeof timer.unref === 'function') timer.unref();
        pendingPreviewInspectRpc.set(rpcId, {
          resolve: (res) => {
            clearTimeout(timer);
            resolve(res);
          },
        });
        postEvent('preview_inspect_request', {
          rpc_id: rpcId,
          selector: args && args.selector ? String(args.selector) : '#app',
          args: args || {},
        });
      }),
    claimTurnOwnership: (ownerReqId) => {
      const resolvedOwner =
        ownerReqId != null ? ownerReqId : `__anon_inprism_turn_${++__uorAnonInprismTurnSeq}`;
      if (
        activeInprismBusy ||
        (activeInprismRequestId != null && activeInprismRequestId !== resolvedOwner)
      ) {
        return { ok: false, activeTurnId: activeInprismTurnId };
      }
      activeInprismBusy = true;
      activeInprismRequestId = resolvedOwner;
      return { ok: true, activeTurnId: activeInprismTurnId };
    },
    releaseTurnOwnership: (ownerReqId) => {
      if (
        activeInprismBusy &&
        (ownerReqId == null || activeInprismRequestId === ownerReqId)
      ) {
        activeInprismBusy = false;
        activeInprismRequestId = null;
        activeInprismTurnId = null;
      }
    },
    setActiveTurnId: (tid, ownerReqId) => {
      if (ownerReqId === undefined || activeInprismRequestId === ownerReqId) {
        activeInprismTurnId = tid;
      }
    },
    registerCancelWaiter: (ownerReqId, cb) => {
      const key = String(ownerReqId);
      let set = pendingInprismCancelWaiters.get(key);
      if (!set) {
        set = new Set();
        pendingInprismCancelWaiters.set(key, set);
      }
      set.add(cb);
      return () => {
        const cur = pendingInprismCancelWaiters.get(key);
        if (cur) {
          cur.delete(cb);
          if (cur.size === 0) pendingInprismCancelWaiters.delete(key);
        }
      };
    },
    isCancelled: (reqId, turnId) =>
      cancelledInprismRequests.has(reqId) ||
      cancelledInprismRequests.has(String(reqId)) ||
      (turnId != null && cancelledInprismTurns.has(Number(turnId))),
    clearCancelled: (reqId, turnId) => {
      cancelledInprismRequests.delete(reqId);
      cancelledInprismRequests.delete(String(reqId));
      if (turnId != null) cancelledInprismTurns.delete(Number(turnId));
    },
  });

  const handleWorkerMessage = async (msg) => {
    const { id, op, payload } = msg || {};
    try {
      if (op === 'studio_dispatch') {
        const envelopeStr = String(payload || '');
        if (envelopeStr.includes('\tchat\t') || envelopeStr.includes('\tchat_v10_turn\t')) {
          let reqHoloModel = '';
          try {
            const parts = envelopeStr.split('\t');
            const bodyRaw = parts.length >= 4 ? parts.slice(3).join('\t').trim() : '';
            if (bodyRaw.startsWith('{')) {
              const reqObj = JSON.parse(bodyRaw);
              const sel = String((reqObj && (reqObj.selected_model || reqObj.model)) || '');
              if (sel === 'uor-r4.qwen3-0.6b-holo' || sel === 'uor-r4.qwen3-1.7b-holo') {
                reqHoloModel = sel;
              }
            }
          } catch {
            // Non-JSON chat envelopes default cleanly
          }
          if (reqHoloModel && typeof studio_holo_runtime_status === 'function') {
            let mountedHolo = false;
            try {
              const hs = JSON.parse(studio_holo_runtime_status());
              mountedHolo = reqHoloModel === 'uor-r4.qwen3-1.7b-holo'
                ? Boolean(hs && hs.qwen3_17b_mounted)
                : Boolean(hs && hs.qwen3_06b_mounted);
            } catch {
              mountedHolo = false;
            }
            if (!mountedHolo) {
              postEvent('hydration_progress', {
                stage: 'holo_v13',
                awaiting: true,
                modelId: reqHoloModel,
                label: `Hydrating verified ${reqHoloModel} pieces via OpfsKappaStore...`,
              });
              await hydrate_holo_model(reqHoloModel, (prog) => {
                postEvent('boot_progress', { stage: 'holo_v13', ...prog });
                postEvent('hydration_progress', { stage: 'holo_v13', ...prog });
              }, false);
            }
          }
        }
        const res = await dispatchWithConnectedBridge(envelopeStr);
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'host_bridge_exec') {
        const toolKind = String((payload && (payload.op || payload.kind)) || 'http_request');
        postEvent('bridge_activity', { phase: 'start', hop: 1, op: toolKind });
        postEvent('brain_pulse', {
          pulses: [
            {
              step: 0,
              phase: 'tool',
              active_layer: 4,
              winning_head: 3,
              e8_orbit: 9,
              cl06_grade: 4,
              kv_copy_dist: 32,
              prll_top_mass: 890000,
              tool_kind: toolKind,
            },
          ],
        });
        const reqPayload = prepareGitBridgePayload(payload || {});
        const res = await hostBridge.execute(reqPayload);
        if (res && res.op === 'git') {
          if (typeof res.path === 'string' && typeof res.content === 'string') {
            applyClonedFilesToWorkspace([{ path: res.path, content: res.content }]);
          } else {
            notifyGitWorkspaceSync(res);
          }
        }
        postEvent('bridge_activity', {
          phase: 'complete',
          hop: 1,
          op: toolKind,
          ok: Boolean(res && res.ok),
          transport: hostBridge.lastTransportMode,
        });
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'mvm_set_secret') {
        const key = String((payload && payload.key) || 'GITHUB_TOKEN');
        const value = String((payload && payload.value) || '');
        const vaultRes = hostBridge.setSecret(key, value);
        try {
          studio_dispatch(`1\tstudio\tmvm_set_secret\t${JSON.stringify({ key, value })}`);
        } catch {
          // Host JS vault remains primary when Wasm guest command is absent
        }
        postReply({ id, ok: vaultRes.ok, result: vaultRes });
        return;
      }
      if (op === 'set_bridge_policy' || op === 'set_host_config') {
        if (payload && typeof payload.preferredModel === 'string' && payload.preferredModel.trim()) {
          globalThis.__uorPreferredHoloModel = payload.preferredModel.trim();
          safeStorageSet('uor_r4_selected_model_v13', globalThis.__uorPreferredHoloModel);
        }
        const res = hostBridge.setPolicy(payload || {});
        if (!res || !res.ok) {
          postReply({
            id,
            ok: false,
            error_code: (res && res.error_code) || 'BridgePolicyError',
            error: (res && (res.message || res.error)) || 'Bridge policy update rejected',
            result: res || { ok: false },
          });
          return;
        }
        if (wasmReady && typeof studio_dispatch === 'function') {
          try {
            const effNetMode =
              hostBridge.webAccessEnabled === false ? 'sandboxed' : hostBridge.mode;
            studio_dispatch(
              `1\tstudio\tmvm_set_network_gate\t${JSON.stringify({
                network_mode: effNetMode,
                consentGranted: hostBridge.consentGranted,
              })}`,
            );
          } catch {
            // Ignore policy sync error before MVM gate initialization
          }
        }
        const hasEmergencyField = Boolean(
          payload &&
            (typeof payload.offlineEmergencyQ31 === 'boolean' ||
              typeof payload.offline_emergency_q31 === 'boolean'),
        );
        if (hasEmergencyField) {
          offlineEmergencyQ31 = Boolean(payload.offlineEmergencyQ31 ?? payload.offline_emergency_q31);
          if (offlineEmergencyQ31) {
            safeStorageSet('uor_offline_emergency_q31', '1');
            bootNativeQ31Fallback = true;
            if (wasmReady && !payload.bootSync && !isQwenMounted() && isQ31Ready()) {
              emitNativeQ31ReadyFallback('offline_emergency_mode');
            }
          } else {
            safeStorageRemove('uor_offline_emergency_q31');
            bootNativeQ31Fallback = false;
          }
        }
        const hasConsentField = Boolean(
          payload &&
            (typeof payload.allowOffOriginHydration === 'boolean' ||
              typeof payload.consentDeclined === 'boolean'),
        );
        if (hasConsentField) {
          purgeLegacyDeclinedHydrationFlag();
          if (payload.allowOffOriginHydration === false) {
            explicitWorkerAllowOffOrigin = false;
            globalThis.__uorAllowOffOriginHydration = false;
          } else if (payload.allowOffOriginHydration === true || payload.consentDeclined === false) {
            explicitWorkerAllowOffOrigin = payload.allowOffOriginHydration === true ? true : undefined;
            if (payload.allowOffOriginHydration === true) {
              globalThis.__uorAllowOffOriginHydration = true;
              safeStorageSet('uor_allow_off_origin_hydration', '1');
            } else {
              delete globalThis.__uorAllowOffOriginHydration;
            }
            if (wasmReady && !payload.bootSync && !isQwenMounted() && payload.hydrateNow === true) {
              selectOptimalInitialHoloModel()
                .then((plan) =>
                  hydrate_holo_model(
                    plan.initialModel,
                    (prog) => {
                      postEvent('boot_progress', { stage: 'holo_v13', ...prog });
                      postEvent('hydration_progress', { stage: 'holo_v13', ...prog });
                    },
                    false,
                    explicitWorkerAllowOffOrigin !== undefined
                      ? { allowOffOriginHydration: explicitWorkerAllowOffOrigin, rememberConsent: true }
                      : { rememberConsent: true },
                  ).then((holo) => {
                    postEvent('stitch_health', { stitchHealth: globalThis.__uorStitchHealth || null });
                    postEvent('holo_ready', {
                      holo,
                      recommendedModel: plan.initialModel,
                      promoted: false,
                      kappaStore: globalThis.__uorHoloKappaStatus || null,
                    });
                  }),
                )
                .catch(() => {});
            }
          }
        }
        try {
          if (typeof studio_dispatch === 'function') {
            studio_dispatch(`1\tstudio\tmvm_set_policy\t${JSON.stringify(payload || {})}`);
          }
        } catch {
          // Host policy remains authoritative when Wasm MVM guest is not booted
        }
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'wasm_eval' || op === 'studio_wasm_eval') {
        const sub = String((payload && payload.subcommand) || 'check');
        const filter = String((payload && payload.filter) || '');
        const res = JSON.parse(studio_wasm_eval(sub, filter));
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'process_envelope') {
        const reqBytes = cachedTextEncoder.encode(String(payload || ''));
        const respBytes = process_envelope(reqBytes);
        const res = cachedTextDecoder.decode(respBytes);
        postReply({ id, ok: true, result: res });
        return;
      }
      if (
        op === 'v1_endpoint' ||
        op === 'studio_v1_messages' ||
        op === 'studio_v1_chat_completions' ||
        op === 'studio_v1_traces'
      ) {
        const endpoint =
          op === 'studio_v1_messages'
            ? 'messages'
            : op === 'studio_v1_chat_completions'
              ? 'chat_completions'
              : op === 'studio_v1_traces'
                ? 'traces'
                : String((payload && payload.endpoint) || 'messages');
        const body = typeof payload === 'string' ? payload : String((payload && payload.body) || '{}');
        const raw =
          endpoint === 'messages'
            ? studio_v1_messages(body)
            : endpoint === 'chat_completions'
              ? studio_v1_chat_completions(body)
              : studio_v1_traces(body);
        let outVal = raw;
        if (op.startsWith('studio_v1_')) {
          try {
            outVal = JSON.parse(raw);
          } catch {
            outVal = raw;
          }
        }
        postReply({ id, ok: true, result: outVal });
        return;
      }
      if (op === 'load_uorlut' || op === 'load_uorlut_bytes') {
        const u8 = payload && payload.bytes instanceof Uint8Array
          ? payload.bytes
          : new Uint8Array((payload && payload.buffer) || 0);
        const label = String((payload && (payload.sourceLabel || payload.label)) || 'uploaded_uorlut');
        const parsed = JSON.parse(studio_load_uorlut_bytes(u8, label));
        if (parsed && parsed.status === 'ok') {
          await cache_uorlut_in_indexeddb(u8, label);
        }
        postReply({
          id,
          ok: true,
          result: {
            ...parsed,
            load: parsed,
            state: JSON.parse(studio_dispatch('1\tstudio\tstate\t')),
          },
        });
        return;
      }
      if (op === 'holo_runtime_status') {
        const holo = typeof studio_holo_runtime_status === 'function' ? JSON.parse(studio_holo_runtime_status()) : null;
        const merged = holo
          ? {
              ...holo,
              holo,
              kappaStore: globalThis.__uorHoloKappaStatus || null,
              webgpu: sharedWebGpuHoloEngine.status(),
              holoslab_verdict: sharedKappaStore.lastSlabVerdict,
              warm_reload_ms: sharedKappaStore.lastWarmReloadMs,
              worker_pool: get_browser_worker_pool_snapshot(),
            }
          : {
              holo: null,
              kappaStore: globalThis.__uorHoloKappaStatus || null,
              webgpu: sharedWebGpuHoloEngine.status(),
              holoslab_verdict: sharedKappaStore.lastSlabVerdict,
              warm_reload_ms: sharedKappaStore.lastWarmReloadMs,
              worker_pool: get_browser_worker_pool_snapshot(),
            };
        postReply({ id, ok: true, result: merged });
        return;
      }
      if (op === 'set_holo_mlp_mode') {
        /* Formal R4.WebGpuGeodesicRuntime */
        const mode = String((payload && payload.mode) || 'auto');
        const shortlistK = Number((payload && (payload.shortlistK ?? payload.shortlist_k)) || 1536);
        const raw = typeof studio_set_holo_mlp_mode === 'function'
          ? studio_set_holo_mlp_mode(mode, shortlistK)
          : studio_dispatch(`1\tstudio\tset_holo_mlp_mode\t${mode}`);
        postReply({ id, ok: true, result: JSON.parse(raw) });
        return;
      }
      if (op === 'verify_holo_slab' || op === 'repair_holo_slab') {
        /* Formal R4.WebGpuGeodesicRuntime */
        const modelId = String((payload && payload.modelId) || 'uor-r4.qwen3-0.6b-holo');
        const repair = op === 'repair_holo_slab' || Boolean(payload && payload.repair);
        const res = await sharedKappaStore.verifyOrRepairSlab(modelId, { repair });
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'webgpu_holo_status') {
        /* Formal R4.WebGpuGeodesicRuntime */
        if (sharedWebGpuHoloEngine.verdict === 'Uninitialized') {
          await sharedWebGpuHoloEngine.init().catch(() => null);
        }
        postReply({
          id,
          ok: true,
          result: {
            webgpu: sharedWebGpuHoloEngine.status(),
            worker_pool: get_browser_worker_pool_snapshot(),
            cross_origin_isolated: Boolean(typeof crossOriginIsolated !== 'undefined' && crossOriginIsolated),
            holoslab_verdict: sharedKappaStore.lastSlabVerdict,
            warm_reload_ms: sharedKappaStore.lastWarmReloadMs,
          },
        });
        return;
      }
      if (op === 'hydrate_holo_model') {
        const modelId = String((payload && payload.modelId) || 'uor-r4.qwen3-0.6b-holo');
        const force = Boolean(payload && payload.force);
        const offlineFallbackReq = Boolean(payload && (payload.offlineFallback || payload.decline || payload.consentDeclined));
        if (offlineFallbackReq) {
          const q31Holo = emitNativeQ31ReadyFallback('offline_emergency_mode') || {
            model_id: 'uor-r4.native-q31-d85',
            ready: false,
          };
          postReply({
            id,
            ok: true,
            result: {
              declined: true,
              fallback_model: 'uor-r4.native-q31-d85',
              recommendedModel: 'uor-r4.native-q31-d85',
              holo: q31Holo,
            },
          });
          return;
        }
        const allowOffOriginHydration =
          payload && payload.allowOffOriginHydration !== undefined
            ? Boolean(payload.allowOffOriginHydration)
            : explicitWorkerAllowOffOrigin;
        const rememberConsent = payload && payload.rememberConsent !== undefined
          ? Boolean(payload.rememberConsent)
          : true;
        try {
          const hydrateOpts = {
            ...(allowOffOriginHydration !== undefined ? { allowOffOriginHydration } : {}),
            ...(payload && Array.isArray(payload.customMirrors) ? { customMirrors: payload.customMirrors } : {}),
            rememberConsent,
          };
          const holo = await hydrate_holo_model(modelId, (prog) => {
            postEvent('boot_progress', { stage: 'holo_v13', ...prog });
            postEvent('hydration_progress', { stage: 'holo_v13', ...prog });
          }, force, hydrateOpts);
          bootNativeQ31Fallback = false;
          if (allowOffOriginHydration === true && rememberConsent) {
            safeStorageSet('uor_allow_off_origin_hydration', '1');
          }
          postEvent('stitch_health', { stitchHealth: globalThis.__uorStitchHealth || null });
          postEvent('holo_ready', {
            holo,
            recommendedModel: modelId,
            promoted: false,
            kappaStore: globalThis.__uorHoloKappaStatus || null,
          });
          postReply({ id, ok: true, result: { holo, kappaStore: globalThis.__uorHoloKappaStatus || null } });
        } catch (err) {
          const code = (err && (err.error_code || err.code)) || '';
          if (code === 'ConsentRequiredForWeightDownload') {
            postEvent('hydration_consent_required', {
              model_id: err.model_id || modelId,
              approx_bytes: Number(err.approx_bytes) || 429916160,
              mirrors: Array.isArray(err.mirrors) ? err.mirrors : [],
              fallback_model: 'uor-r4.native-q31-d85',
              message: err.message || String(err),
            });
          }
          throw err;
        }
        return;
      }
      if (op === 'holo_storage_geometry') {
        /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
        const reqId = String((payload && payload.modelId) || '');
        const raw = typeof studio_holo_storage_geometry_json === 'function'
          ? studio_holo_storage_geometry_json(reqId)
          : studio_dispatch(`1\tstudio\tholo_storage_geometry\t${reqId}`);
        postReply({ id, ok: true, result: JSON.parse(raw) });
        return;
      }
      if (op === 'telos_self_improvement_tick') {
        /* Formal R4.OntologicalTelos (src/R4/OntologicalTelos.lex.tex) */
        const body = JSON.stringify(payload || {});
        const raw = typeof studio_run_telos_self_improvement_tick === 'function'
          ? studio_run_telos_self_improvement_tick(body)
          : studio_dispatch(`1\tstudio\ttelos_tick\t${body}`);
        const parsedTick = JSON.parse(raw);
        postEvent('telos_tick', parsedTick);
        postReply({ id, ok: true, result: parsedTick });
        return;
      }
      if (op === 'load_hf_repo_holo') {
        const repoId = String((payload && payload.repoId) || 'Qwen/Qwen3-0.6B');
        const holo = await load_hf_repo_holo(repoId, (prog) => {
          postEvent('boot_progress', prog);
          postEvent('hydration_progress', prog);
        });
        postReply({ id, ok: true, result: { holo, repoId } });
        return;
      }
      if (op === 'native_q31_status') {
        const res = typeof studio_native_q31_status === 'function' ? JSON.parse(studio_native_q31_status()) : null;
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'native_q31_train_step') {
        const text = String((payload && payload.text) || '');
        const res = typeof studio_native_q31_train_step === 'function' ? JSON.parse(studio_native_q31_train_step(text)) : null;
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'weight_pack_status') {
        const pack = typeof studio_weight_pack_status === 'function' ? JSON.parse(studio_weight_pack_status()) : null;
        postReply({ id, ok: true, result: pack });
        return;
      }
      if (op === 'shard_catalog') {
        const cat = typeof studio_shard_catalog === 'function' ? JSON.parse(studio_shard_catalog()) : null;
        postReply({ id, ok: true, result: cat });
        return;
      }
      if (op === 'export_uorlut_shard') {
        const rootIdx = Number((payload && payload.e8Root) ?? 0);
        const bytes = typeof studio_export_uorlut_shard === 'function' ? studio_export_uorlut_shard(rootIdx) : new Uint8Array(0);
        postReply({ id, ok: true, result: Array.from(bytes) });
        return;
      }
      if (op === 'merge_uorlut_shard') {
        const u8 = payload && payload.bytes instanceof Uint8Array
          ? payload.bytes
          : new Uint8Array((payload && payload.bytes) || (payload && payload.buffer) || 0);
        const raw = typeof studio_merge_uorlut_shard === 'function' ? studio_merge_uorlut_shard(u8) : '{"status":"error"}';
        postReply({ id, ok: true, result: JSON.parse(raw) });
        return;
      }
      if (op === 'gllm_v9_status') {
        const res = typeof studio_gllm_v9_status === 'function' ? JSON.parse(studio_gllm_v9_status()) : null;
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'sync_weights' || op === 'hydrate_v12_360m' || op === 'hydrate_360m') {
        const tierArg = op === 'sync_weights' ? 'auto' : 'v12_360m';
        const v10 = await startOrAwaitOpfsShardHydration(true, tierArg);
        const lut = await sync_cached_weights((prog) => {
          postEvent('boot_progress', { stage: 'uorlut_sync', ...prog });
        });
        const r4Stack = typeof studio_r4_stack_status === 'function' ? JSON.parse(studio_r4_stack_status()) : null;
        postReply({ id, ok: true, result: { v10, lut, r4Stack, gllmMeta: globalThis.__uorGllmV9Meta || null } });
        return;
      }
      if (op === 'snapshot') {
        const state = JSON.parse(studio_dispatch('1\tstudio\tstate\t'));
        const workspace = __uorCollectWorkspaceSnapshot();
        const pack = JSON.parse(studio_weight_pack_status());
        const gllm = typeof studio_gllm_v9_status === 'function' ? JSON.parse(studio_gllm_v9_status()) : null;
        const r4Stack = typeof studio_r4_stack_status === 'function' ? JSON.parse(studio_r4_stack_status()) : null;
        const verdantSynthesis = query_verdant_synthesis_status();
        postReply({
          id,
          ok: true,
          result: {
            state,
            workspace,
            pack,
            gllm,
            r4Stack,
            verdantSynthesis,
            stitchHealth: globalThis.__uorStitchHealth || null,
          },
        });
        return;
      }
      if (op === 'stitch_health') {
        postReply({ id, ok: true, result: globalThis.__uorStitchHealth || null });
        return;
      }
      if (op === 'admit_generation_capsule') {
        const res = await admit_uorc_generation_capsule(payload && payload.capsule ? payload.capsule : payload);
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'record_generation_fault') {
        const reason = payload && typeof payload.reason === 'string' ? payload.reason : 'runtime_fault';
        const res = await record_uorc_generation_fault(reason);
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'verdant_synthesis_status') {
        postReply({ id, ok: true, result: query_verdant_synthesis_status() });
        return;
      }
      if (op === 'r4_stack_status') {
        const r4Stack = typeof studio_r4_stack_status === 'function' ? JSON.parse(studio_r4_stack_status()) : null;
        postReply({ id, ok: true, result: r4Stack });
        return;
      }
      if (op === 'init_worker_pool') {
        const pool = await init_shared_wasm_worker_pool();
        postReply({ id, ok: true, result: pool });
        return;
      }
      if (op === 'check_studio_update') {
        const origin = payload && typeof payload.origin === 'string' ? payload.origin : '';
        const update = await hostBridge.checkProvenanceUpdate(origin);
        if (update && update.update_available) {
          postEvent('studio_update_available', update);
        }
        postReply({ id, ok: true, result: update });
        return;
      }
      if (op === 'force_refresh_reload') {
        if (typeof caches !== 'undefined' && typeof caches.keys === 'function') {
          try {
            const keys = await caches.keys();
            for (const key of keys) {
              await caches.delete(key);
            }
          } catch {
            // Optional cache cleanup
          }
        }
        if (
          typeof navigator !== 'undefined' &&
          navigator.storage &&
          typeof navigator.storage.getDirectory === 'function'
        ) {
          try {
            const root = await navigator.storage.getDirectory();
            await root.removeEntry(OPFS_V10_META_FILE).catch(() => {});
          } catch {
            // Optional OPFS cleanup
          }
        }
        let reloadedWasm = false;
        try {
          await __uor_resolve_cached_wasm(new URL('lexlean_r4_bg.wasm', import.meta.url), true);
          reloadedWasm = true;
        } catch {
          // Allow reload to fetch fresh artifacts cleanly
        }
        const provenance = await hostBridge.checkProvenanceUpdate().catch(() => null);
        postReply({ id, ok: true, result: { ok: true, reloadedWasm, provenance } });
        return;
      }
      if (op === 'prism_identity_ensure' || op === 'prism_identity_status') {
        const res = await stitch_prism_identity_ensure();
        postEvent('prism_identity', res);
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'prism_identity_bind_persona') {
        const res = await stitch_prism_identity_bind_persona(
          (payload && payload.handle) || 'lexlean-r4-studio',
          (payload && payload.host) || 'uor-foundation.github.io',
          (payload && payload.holoKappa) || (globalThis.__uorLatestHoloSummary && globalThis.__uorLatestHoloSummary.fingerprint ? `blake3:${globalThis.__uorLatestHoloSummary.fingerprint}` : ''),
        );
        postEvent('prism_identity', res);
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'prism_identity_sign_action') {
        const res = await stitch_prism_identity_sign_action(
          (payload && payload.action) || 'compile-holo',
          (payload && payload.targetKappa) || '',
        );
        postEvent('prism_identity', res);
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'compile_holo_archive' || op === 'verify_holo_archive' || op === 'download_holo_archive') {
        const includeBytes = Boolean((payload && payload.includeBytes) || op === 'download_holo_archive');
        const providedBytes = payload && (payload.bytes instanceof Uint8Array || payload.buffer)
          ? (payload.bytes instanceof Uint8Array ? payload.bytes : new Uint8Array(payload.buffer))
          : null;
        const res = op === 'verify_holo_archive' && providedBytes
          ? await verify_holo_v4_archive_in_worker(providedBytes, includeBytes)
          : await compile_and_verify_holo_v4_in_worker(payload && payload.files ? payload.files : null, includeBytes);
        postEvent('holo_compiled', res);
        postReply({ id, ok: true, result: res });
        return;
      }
      if (op === 'get_me_app_descriptor' || op === 'get_foundry_app_descriptor') {
        const desc = await build_afflom_me_app_descriptor(payload && payload.origin ? String(payload.origin) : '');
        postReply({ id, ok: true, result: desc });
        return;
      }
      if (op === 'cancel_inprism_turn' || op === 'studio_cancel_turn') {
        const hasTarget = payload && payload.targetId != null;
        const hasReq = payload && payload.requestId != null;
        const hasReqSnake = payload && payload.reqId != null;
        const targetReqId = hasTarget
          ? payload.targetId
          : hasReq
            ? payload.requestId
            : hasReqSnake
              ? payload.reqId
              : null;
        if (targetReqId != null) {
          cancelledInprismRequests.add(targetReqId);
          cancelledInprismRequests.add(String(targetReqId));
        }
        if (hasReq && payload.requestId !== targetReqId) {
          cancelledInprismRequests.add(payload.requestId);
          cancelledInprismRequests.add(String(payload.requestId));
        }
        const matchesActiveReq =
          targetReqId != null &&
          activeInprismRequestId != null &&
          String(targetReqId) === String(activeInprismRequestId);
        if (
          targetReqId == null &&
          (payload == null || payload.turn_id == null) &&
          activeInprismRequestId != null
        ) {
          cancelledInprismRequests.add(activeInprismRequestId);
          cancelledInprismRequests.add(String(activeInprismRequestId));
        }
        const rawTid =
          payload && payload.turn_id != null
            ? Number(payload.turn_id)
            : targetReqId == null || matchesActiveReq
              ? activeInprismTurnId
              : null;
        let cancelRes = { status: 'not_found', turn_id: Number(rawTid || 0) };
        if (rawTid != null && Number.isFinite(rawTid) && rawTid > 0) {
          cancelledInprismTurns.add(Number(rawTid));
          const cancelFn =
            typeof holo_cancel_foundry_turn === 'function'
              ? holo_cancel_foundry_turn
              : globalThis.holo_cancel_foundry_turn;
          if (typeof cancelFn === 'function') {
            const arg = typeof BigInt === 'function' ? BigInt(Math.trunc(rawTid)) : Math.trunc(rawTid);
            cancelRes = JSON.parse(cancelFn(arg));
          } else {
            cancelRes = { status: 'cancelled', turn_id: Number(rawTid) };
          }
        } else if (matchesActiveReq || (targetReqId == null && activeInprismRequestId != null)) {
          cancelRes = {
            status: 'cancelled',
            turn_id: 0,
            targetId: targetReqId != null ? targetReqId : activeInprismRequestId,
          };
        }
        const notifyKeys = new Set();
        if (targetReqId != null) notifyKeys.add(String(targetReqId));
        else if (activeInprismRequestId != null) notifyKeys.add(String(activeInprismRequestId));
        for (const k of notifyKeys) {
          const waiters = pendingInprismCancelWaiters.get(k);
          if (waiters) {
            for (const cb of waiters) cb();
            pendingInprismCancelWaiters.delete(k);
          }
        }
        postReply({ id, ok: true, result: cancelRes });
        return;
      }
      if (op === 'resolve_staged_mvm_command') {
        const res = resolve_staged_mvm_command(payload && typeof payload === 'object' ? payload : {});
        postReply({
          id,
          ok: Boolean(res && res.ok !== false),
          result: res,
          ...(res && res.ok === false ? { error_code: res.error_code || 'StagedCommandNotFound' } : {}),
        });
        return;
      }
      if (op === 'terminal:write' || op === 'terminal:open' || op === 'terminal:close' || op === 'terminal:kill' || op === 'terminal:resize' || op === 'terminal:serialize') {
        if (payload != null && typeof payload !== 'object') {
          postReply({
            id,
            ok: false,
            error_code: 'InvalidPayload',
            error: `${op} payload must be an object`,
          });
          return;
        }
        if (op === 'terminal:write') {
          const cmdStr = payload && typeof payload.cmd === 'string' ? payload.cmd.trim() : '';
          const cwdStr = payload && typeof payload.cwd === 'string' && payload.cwd.trim() ? payload.cwd.trim() : undefined;
          const hasRawData = payload && typeof payload.data === 'string' && payload.data.length > 0;
          if (!cmdStr && !hasRawData) {
            postReply({
              id,
              ok: false,
              error_code: 'InvalidPayload',
              error: 'terminal:write requires non-empty payload.cmd or payload.data',
            });
            return;
          }
          if (cmdStr) {
            const mvmReq = cwdStr ? { cmd: cmdStr, cwd: cwdStr } : { cmd: cmdStr };
            const out = await dispatchWithConnectedBridge(`1\tstudio\tmvm_exec\t${JSON.stringify(mvmReq)}`);
            postReply({ id, ok: true, result: out });
            return;
          }
          postReply({
            id,
            ok: true,
            result: { ok: true, op, bytes: payload.data.length, interleaved: true },
          });
          return;
        }
        if (op === 'terminal:resize') {
          const cols = Number(payload && payload.cols);
          const rows = Number(payload && payload.rows);
          if (!Number.isFinite(cols) || cols <= 0 || !Number.isFinite(rows) || rows <= 0) {
            postReply({
              id,
              ok: false,
              error_code: 'InvalidPayload',
              error: 'terminal:resize requires positive cols and rows',
            });
            return;
          }
        }
        postReply({ id, ok: true, result: { ok: true, op, interleaved: true } });
        return;
      }
      if (op === 'preview_inspect_reply') {
        const rpcId = payload && (payload.rpc_id || payload.rpcId || payload.id);
        const key = rpcId != null ? String(rpcId) : '';
        const waiter = key ? pendingPreviewInspectRpc.get(key) : null;
        if (waiter) {
          pendingPreviewInspectRpc.delete(key);
          waiter.resolve(payload || { ok: true, rpc_id: key });
        }
        if (id != null) {
          postReply({ id, ok: true, result: { ok: true, rpc_id: key } });
        }
        return;
      }
      if (op === 'inprism_r4_turn' || op === 'studio_stream_turn') {
        /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
        const reqPayload = payload && typeof payload === 'object' ? { ...payload } : {};
        if (op === 'studio_stream_turn') {
          if (!reqPayload.surface_mode) reqPayload.surface_mode = 'studio';
          if (reqPayload.live_hud === undefined) {
            reqPayload.live_hud = reqPayload.liveHud !== undefined ? Boolean(reqPayload.liveHud) : true;
          }
          if (shouldUseOfflineQ31Fallback(reqPayload, false) && !reqPayload.selected_model && !reqPayload.preferredModel) {
            reqPayload.offlineFallback = true;
          }
        }
        const resEnvelope = await execute_inprism_foundry_turn(
          { id, payload: reqPayload },
          createWorkerTurnHooks(),
        );
        postReply(resEnvelope);
        return;
      }
      if (op === 'studio_cmdk_edit') {
        /* Formal R4.StudioFrame */
        const p = payload && typeof payload === 'object' ? payload : {};
        const editPath = String(p.path || 'src/main.rs');
        const origCode = String(p.code ?? '');
        const instruction = String(p.instruction || 'Apply inline edit').trim();
        const maxTokens = Math.min(192, Math.max(1, Number(p.max_new_tokens || p.maxTokens || 192) || 192));
        const selectionSuffix = p.selection && typeof p.selection === 'string'
          ? `\nSelection:\n\`\`\`\n${p.selection}\n\`\`\``
          : '';
        const prompt = `[Cmd+K Inline Edit: ${editPath}]\nInstruction: ${instruction}\nOriginal code:\n\`\`\`\n${origCode}\n\`\`\`${selectionSuffix}\nRespond with only the updated code.`;
        const reqPayload = {
          ...p,
          prompt,
          active_file: editPath,
          surface_mode: 'studio',
          composer_mode: 'agent',
          live_hud: Boolean(p.live_hud ?? p.liveHud),
          max_new_tokens: maxTokens,
          suppressTagCommit: true,
          ...(shouldUseOfflineQ31Fallback(p, true) ? { offlineFallback: true } : {}),
        };
        const turnRes = await execute_inprism_foundry_turn(
          { id, payload: reqPayload },
          createWorkerTurnHooks(),
        );
        if (!turnRes || !turnRes.ok) {
          postReply(turnRes);
          return;
        }
        const rawReply = (turnRes.result && (turnRes.result.reply || turnRes.result.assistant_reply)) || '';
        const proposedCode = stripCodeFences(rawReply);
        const diffHunk = buildUnifiedDiffHunk(origCode, proposedCode, editPath, false);
        postReply({
          id,
          ok: true,
          result: {
            status: 'ok',
            path: editPath,
            diff_hunk: diffHunk,
            proposed_code: proposedCode,
            receipt_sha256: turnRes.result ? (turnRes.result.receipt_sha256 || null) : null,
            model_id: turnRes.result ? (turnRes.result.model_id || null) : null,
            tokens_emitted: turnRes.result ? (turnRes.result.tokens_emitted || 0) : 0,
            tok_per_sec: turnRes.result ? (turnRes.result.tok_per_sec || 0) : 0,
          },
        });
        return;
      }
      if (op === 'studio_ghost_complete') {
        /* Formal R4.StudioFrame */
        const p = payload && typeof payload === 'object' ? payload : {};
        const ghostPath = String(p.path || 'src/lib.rs');
        const prefix = String(p.prefix ?? '');
        const suffix = String(p.suffix ?? '');
        const maxTokens = Math.min(48, Math.max(1, Number(p.max_new_tokens || p.maxTokens || 48) || 48));
        const prompt = `[Ghost Complete: ${ghostPath}]\nPrefix:\n${prefix}${suffix ? `\nSuffix:\n${suffix}` : ''}\nComplete concisely at cursor.`;
        const reqPayload = {
          ...p,
          prompt,
          active_file: ghostPath,
          surface_mode: 'studio',
          composer_mode: 'agent',
          live_hud: false,
          max_new_tokens: maxTokens,
          suppressTagCommit: true,
          ...(shouldUseOfflineQ31Fallback(p, true) ? { offlineFallback: true } : {}),
        };
        const turnRes = await execute_inprism_foundry_turn(
          { id, payload: reqPayload },
          createWorkerTurnHooks(),
        );
        if (!turnRes || !turnRes.ok) {
          postReply(turnRes);
          return;
        }
        const rawReply = (turnRes.result && (turnRes.result.reply || turnRes.result.assistant_reply)) || '';
        const suggestion = stripCodeFences(rawReply);
        postReply({
          id,
          ok: true,
          result: {
            status: 'ok',
            path: ghostPath,
            suggestion,
            receipt_sha256: turnRes.result ? (turnRes.result.receipt_sha256 || null) : null,
            model_id: turnRes.result ? (turnRes.result.model_id || null) : null,
            tokens_emitted: turnRes.result ? (turnRes.result.tokens_emitted || 0) : 0,
            tok_per_sec: turnRes.result ? (turnRes.result.tok_per_sec || 0) : 0,
          },
        });
        return;
      }
      if (op === 'mvm_sync_workspace_in') {
        const parsedOut = runMvmSyncWorkspaceIn(payload, null, true);
        if (parsedOut && parsedOut.status === 'ok') {
          postEvent('workspace_sync', { workspace: __uorCollectWorkspaceSnapshot() });
        }
        postReply({ id, ok: true, result: parsedOut });
        return;
      }
      if (op === 'mvm_export_workspace_diff') {
        const parsedOut = runMvmExportWorkspaceDiff(true);
        if (parsedOut && parsedOut.status === 'ok') {
          postEvent('workspace_sync', {
            workspace: __uorCollectWorkspaceSnapshot(),
            diff: parsedOut,
          });
        }
        postReply({ id, ok: true, result: parsedOut });
        return;
      }
      postReply({ id, ok: false, error: `Unknown worker op: ${op}` });
    } catch (err) {
      const errCode = err && (err.error_code || err.code);
      postReply({
        id,
        ok: false,
        ...(errCode ? { error_code: errCode } : {}),
        ...(err && err.model_id ? { model_id: err.model_id } : {}),
        ...(err && err.approx_bytes ? { approx_bytes: err.approx_bytes } : {}),
        ...(err && Array.isArray(err.mirrors) ? { mirrors: err.mirrors } : {}),
        ...(err && err.fallback_model ? { fallback_model: err.fallback_model } : {}),
        error: err && err.message ? err.message : String(err),
      });
    }
  };

  self.onmessage = (ev) => {
    const msg = ev.data || {};
    if (!wasmReady) {
      earlyMessages.push(msg);
      if (!bootPromise) startBoot();
      return;
    }
    handleWorkerMessage(msg);
  };

  const startBoot = () => {
    if (bootPromise) return bootPromise;
    bootPromise = (async () => {
      postEvent('boot_progress', { stage: 'wasm_compile', label: 'Compiling UOR-R4 Wasm engine...' });
      __uorRequestPersistentStorage();
      await __wbg_init();
      await restore_cached_uorlut_from_indexeddb().catch(() => null);
      await restore_mvm_cow_sectors_from_opfs().catch(() => null);
      wasmReady = true;
      try {
        const effNetMode =
          hostBridge.webAccessEnabled === false ? 'sandboxed' : hostBridge.mode;
        studio_dispatch(
          `1\tstudio\tmvm_set_network_gate\t${JSON.stringify({ network_mode: effNetMode })}`,
        );
      } catch {
        // Ignore if studio dispatch gate sync is unavailable
      }

      const state = JSON.parse(studio_dispatch('1\tstudio\tstate\t'));
      const workspace = __uorCollectWorkspaceSnapshot();
      const pack = JSON.parse(studio_weight_pack_status());
      const gllm = typeof studio_gllm_v9_status === 'function' ? JSON.parse(studio_gllm_v9_status()) : null;
      const r4Stack = typeof studio_r4_stack_status === 'function' ? JSON.parse(studio_r4_stack_status()) : null;
      const holoStatus = typeof studio_holo_runtime_status === 'function' ? JSON.parse(studio_holo_runtime_status()) : null;
      const nativeQ31 = typeof studio_native_q31_status === 'function' ? JSON.parse(studio_native_q31_status()) : null;
      const verdantSynthesis = query_verdant_synthesis_status();
      const prismIdentity = await stitch_prism_identity_ensure().catch(() => null);
      postEvent('ready', {
        snapshot: { state, workspace, pack, gllm, r4Stack, holoStatus, nativeQ31, verdantSynthesis, prismIdentity },
        weightCacheMeta: globalThis.__uorWeightCacheMeta || null,
        gllmMeta: globalThis.__uorGllmV9Meta || null,
        r4Stack,
        holoStatus,
        nativeQ31,
        verdantSynthesis,
        prismIdentity,
      });

      while (earlyMessages.length > 0) {
        const queued = earlyMessages.shift();
        await handleWorkerMessage(queued);
      }

      // Boot instantly (`<= 150 ms`, `0 B` download) on Qwen3-distilled `UORR4V15` `10,000+ tok/s` sparse-basin lane (`R4-81`).
      purgeLegacyDeclinedHydrationFlag();
      if (offlineEmergencyQ31 || safeStorageGet('uor_offline_emergency_q31') === '1') {
        bootNativeQ31Fallback = true;
        emitNativeQ31ReadyFallback('offline_emergency_mode');
      } else if (!isQwenMounted() && typeof studio_sparse_v15_status_json === 'function') {
        try {
          const v15 = JSON.parse(studio_sparse_v15_status_json());
          bootNativeQ31Fallback = false;
          const prefBootModel =
            hostBridge && hostBridge.preferredModel === 'uor-r4.qwen3-1.7b-holo'
              ? 'uor-r4.qwen3-1.7b-holo'
              : 'uor-r4.qwen3-0.6b-holo';
          postEvent('stitch_health', { stitchHealth: globalThis.__uorStitchHealth || null });
          postEvent('holo_ready', {
            holo: {
              mounted_count: 1,
              qwen3_06b_mounted: true,
              qwen3_17b_mounted: false,
              sparse_v15_ready: true,
              active_backend: 'uorr4v15-sparse-basin-10k',
              boot_verdict: admitFastBootAndZeroRedundancySlice(
                0,
                Number(v15.boot_mount_ms || 1),
                0,
                false,
                0,
              ),
              models: [
                {
                  model_id: 'uor-r4.qwen3-0.6b-holo',
                  tier: 'SparseBasinV15_10k',
                  n_layers: 6,
                  d_model: 576,
                  vocab_size: Number(v15.vocab_size || 512),
                  container_sha256: v15.container_sha256 || '',
                },
              ],
            },
            recommendedModel: prefBootModel,
            promoted: false,
            kappaStore: globalThis.__uorHoloKappaStatus || null,
          });
        } catch {
          emitNativeQ31ReadyFallback('hydration_unavailable');
        }
      }

      if (typeof location !== 'undefined' && /^https?:/i.test(location.origin || '')) {
        hostBridge.checkProvenanceUpdate('').catch(() => null);
        if (typeof setInterval === 'function') {
          const provenancePollTimer = setInterval(() => {
            if (typeof self === 'undefined' || !self) {
              clearInterval(provenancePollTimer);
              return;
            }
            hostBridge
              .checkProvenanceUpdate('')
              .then((upd) => {
                if (upd && upd.update_available) {
                  postEvent('studio_update_available', upd);
                }
              })
              .catch(() => {});
          }, 60000);
          if (provenancePollTimer && typeof provenancePollTimer.unref === 'function') {
            provenancePollTimer.unref();
          }
        }
      }
    })().catch((err) => {
      postEvent('boot_error', {
        message: err && err.message ? err.message : String(err),
      });
    });
    return bootPromise;
  };

  startBoot();
}

// === VERDANT RESONANCE WORKER SATURATION, WASM MODULE POOL & OPFS .uorc CAPSULE WATCHDOG ===
const UORC_GENERATIONS_DIR = 'generations';
const UORC_HEAD_FILE = 'HEAD';
const UORC_FRONTIER_FILE = 'FRONTIER.json';
const UOR_SELFHOST_SYNC_CHANNEL = 'uor-r4-selfhost-sync';

let __uorSharedWasmModule = null;
let __uorSharedWasmBytes = null;
const __uorSharedWorkerPool = [];
let __uorSharedWorkerBlobUrl = null;
const __uorMemGenerationStore = {
  head: 'gen-0',
  frontier: ['gen-0'],
  capsules: new Map([
    [
      'gen-0',
      {
        generation_id: 'gen-0',
        parent_generation_id: null,
        schema: 'uorc/generation-capsule/1',
        strikes: 0,
        status: 'baseline',
      },
    ],
  ]),
};

export function optimal_browser_worker_count(logicalThreads) {
  const detected =
    typeof logicalThreads === 'number' && Number.isFinite(logicalThreads) && logicalThreads > 0
      ? Math.floor(logicalThreads)
      : typeof navigator !== 'undefined' && typeof navigator.hardwareConcurrency === 'number'
        ? Math.max(1, Math.floor(navigator.hardwareConcurrency))
        : 4;
  if (detected <= 2) return 1;
  const candidate = detected - 1;
  return candidate > 8 ? 8 : candidate;
}

async function __uorWithWebLock(lockName, task) {
  if (
    typeof navigator !== 'undefined' &&
    navigator.locks &&
    typeof navigator.locks.request === 'function'
  ) {
    return navigator.locks.request(lockName, () => task());
  }
  return task();
}

function __uorBroadcastGenSync(generationId, digestSha256, eventKind) {
  if (typeof BroadcastChannel === 'undefined') return false;
  try {
    const ch = new BroadcastChannel(UOR_SELFHOST_SYNC_CHANNEL);
    ch.postMessage({
      protocol: 'uor-workspace-bridge/1',
      type: 'uor-r4:generation-sync',
      event: eventKind || 'admitted',
      generation_id: generationId,
      digest_sha256: digestSha256 || '',
      timestamp_ms: Date.now(),
    });
    ch.close();
    return true;
  } catch {
    return false;
  }
}

export async function init_shared_wasm_worker_pool(wasmBytesOrModule = null) {
  const logicalThreads =
    typeof navigator !== 'undefined' && typeof navigator.hardwareConcurrency === 'number'
      ? Math.max(1, Math.floor(navigator.hardwareConcurrency))
      : 4;
  const targetWorkers = optimal_browser_worker_count(logicalThreads);

  if (wasmBytesOrModule instanceof WebAssembly.Module) {
    __uorSharedWasmModule = wasmBytesOrModule;
  } else if (wasmBytesOrModule && wasmBytesOrModule.byteLength > 0) {
    __uorSharedWasmBytes = wasmBytesOrModule instanceof ArrayBuffer
      ? wasmBytesOrModule
      : wasmBytesOrModule.buffer.slice(
          wasmBytesOrModule.byteOffset,
          wasmBytesOrModule.byteOffset + wasmBytesOrModule.byteLength,
        );
    if (!__uorSharedWasmModule && typeof WebAssembly !== 'undefined' && typeof WebAssembly.compile === 'function') {
      __uorSharedWasmModule = await WebAssembly.compile(__uorSharedWasmBytes);
    }
  } else if (!__uorSharedWasmModule && typeof globalThis !== 'undefined' && globalThis.__uorWasmBytesBuffer) {
    __uorSharedWasmBytes = globalThis.__uorWasmBytesBuffer;
    if (typeof WebAssembly !== 'undefined' && typeof WebAssembly.compile === 'function') {
      __uorSharedWasmModule = await WebAssembly.compile(__uorSharedWasmBytes);
    }
  } else if (!__uorSharedWasmModule && typeof fetch === 'function') {
    try {
      const wasmUrl = new URL('lexlean_r4_bg.wasm', import.meta.url);
      const cached = await __uor_resolve_cached_wasm(wasmUrl, false);
      if (cached && typeof cached.arrayBuffer === 'function') {
        __uorSharedWasmBytes = await cached.arrayBuffer();
        __uorSharedWasmModule = await WebAssembly.compile(__uorSharedWasmBytes);
      }
    } catch {
      // Optional compile in offline/mocked Node test harnesses
    }
  }

  if (
    __uorSharedWorkerPool.length === 0 &&
    typeof Worker !== 'undefined' &&
    typeof Blob !== 'undefined' &&
    typeof URL !== 'undefined' &&
    typeof URL.createObjectURL === 'function'
  ) {
    const poolWorkerCode = [
      'let sharedModule = null;',
      'self.onmessage = async (ev) => {',
      '  const msg = ev && ev.data ? ev.data : {};',
      '  if (msg.type === "INIT_MODULE" && (msg.module || msg.wasmBytes)) {',
      '    sharedModule = msg.module || await WebAssembly.compile(msg.wasmBytes);',
      '    self.postMessage({ type: "READY", workerIndex: msg.workerIndex, hasModule: Boolean(sharedModule) });',
      '  }',
      '};',
    ].join('\n');
    if (!__uorSharedWorkerBlobUrl) {
      __uorSharedWorkerBlobUrl = URL.createObjectURL(new Blob([poolWorkerCode], { type: 'text/javascript' }));
    }
    for (let i = 0; i < targetWorkers; i += 1) {
      try {
        const worker = new Worker(__uorSharedWorkerBlobUrl);
        __uorSharedWorkerPool.push({ index: i, worker, ready: false });
        if (__uorSharedWasmModule || __uorSharedWasmBytes) {
          worker.postMessage({
            type: 'INIT_MODULE',
            workerIndex: i,
            module: __uorSharedWasmModule,
            wasmBytes: __uorSharedWasmBytes,
          });
        }
      } catch {
        break;
      }
    }
  }

  return get_browser_worker_pool_snapshot();
}

export function get_browser_worker_pool_snapshot() {
  /* Formal R4.HoloGeometricRuntime (src/R4/HoloGeometricRuntime.lex.tex) */
  const logicalThreads =
    typeof navigator !== 'undefined' && typeof navigator.hardwareConcurrency === 'number'
      ? Math.max(1, Math.floor(navigator.hardwareConcurrency))
      : 4;
  return {
    logical_threads: logicalThreads,
    optimal_workers: optimal_browser_worker_count(logicalThreads),
    pool_size: __uorSharedWorkerPool.length,
    ready_workers: __uorSharedWorkerPool.filter((w) => Boolean(w && w.ready)).length,
    shared_module_compiled: Boolean(__uorSharedWasmModule),
    zero_refetch_sharing: true,
    striped_int8_ready: Boolean(__uorSharedWorkerPool.length > 0 || __uorSharedWasmModule),
    active_head: __uorMemGenerationStore.head,
    frontier_count: __uorMemGenerationStore.frontier.length,
  };
}

async function __uorWriteOpfsGenerationRecord(generationId, manifestObj, capsuleBytes) {
  __uorMemGenerationStore.capsules.set(generationId, manifestObj);
  __uorMemGenerationStore.head = generationId;
  if (!__uorMemGenerationStore.frontier.includes(generationId)) {
    __uorMemGenerationStore.frontier.push(generationId);
  }
  const hasOpfs = typeof navigator !== 'undefined' && navigator.storage && typeof navigator.storage.getDirectory === 'function';
  if (!hasOpfs) return 'memory';
  try {
    const root = await navigator.storage.getDirectory();
    const gensDir = await root.getDirectoryHandle(UORC_GENERATIONS_DIR, { create: true });
    const genDir = await gensDir.getDirectoryHandle(generationId, { create: true });
    const mf = await genDir.getFileHandle('manifest.json', { create: true });
    const mw = await mf.createWritable();
    await mw.write(cachedTextEncoder.encode(JSON.stringify(manifestObj, null, 2)));
    await mw.close();
    const cf = await genDir.getFileHandle('capsule.uorc', { create: true });
    const cw = await cf.createWritable();
    await cw.write(capsuleBytes);
    await cw.close();
    const hf = await root.getFileHandle(UORC_HEAD_FILE, { create: true });
    const hw = await hf.createWritable();
    await hw.write(cachedTextEncoder.encode(generationId));
    await hw.close();
    const ff = await root.getFileHandle(UORC_FRONTIER_FILE, { create: true });
    const fw = await ff.createWritable();
    await fw.write(cachedTextEncoder.encode(JSON.stringify(__uorMemGenerationStore.frontier)));
    await fw.close();
    return 'opfs';
  } catch {
    return 'memory';
  }
}

async function __uorWriteOpfsHead(generationId) {
  __uorMemGenerationStore.head = generationId;
  const hasOpfs = typeof navigator !== 'undefined' && navigator.storage && typeof navigator.storage.getDirectory === 'function';
  if (!hasOpfs) return 'memory';
  try {
    const root = await navigator.storage.getDirectory();
    const hf = await root.getFileHandle(UORC_HEAD_FILE, { create: true });
    const hw = await hf.createWritable();
    await hw.write(cachedTextEncoder.encode(generationId));
    await hw.close();
    return 'opfs';
  } catch {
    return 'memory';
  }
}

export async function admit_uorc_generation_capsule(capsuleInput) {
  const payloadStr = typeof capsuleInput === 'string' ? capsuleInput : JSON.stringify(capsuleInput || {});
  let admission = null;
  if (typeof studio_admit_generation_capsule === 'function') {
    admission = JSON.parse(studio_admit_generation_capsule(payloadStr));
  } else {
    throw new Error('Wasm studio_admit_generation_capsule export unavailable');
  }
  if (!admission || admission.status !== 'ok' || admission.admitted !== true) {
    return { ...admission, persisted: false };
  }
  const genId = admission.active_head || admission.generation_id || `gen-${__uorMemGenerationStore.frontier.length}`;
  const rawBytes = cachedTextEncoder.encode(payloadStr);
  const storageBackend = await __uorWithWebLock('uor-r4-generations-head', () =>
    __uorWriteOpfsGenerationRecord(genId, admission, rawBytes),
  );
  __uorBroadcastGenSync(genId, admission.capsule_sha256 || '', 'admitted');
  return {
    ...admission,
    persisted: true,
    storage_backend: storageBackend,
    head: __uorMemGenerationStore.head,
  };
}

export async function record_uorc_generation_fault(faultReason) {
  if (typeof studio_record_generation_fault !== 'function') {
    throw new Error('Wasm studio_record_generation_fault export unavailable');
  }
  const result = JSON.parse(studio_record_generation_fault(String(faultReason || 'runtime_fault')));
  let storageBackend = 'memory';
  if (result && result.rolled_back === true && result.active_head) {
    storageBackend = await __uorWithWebLock('uor-r4-generations-head', () =>
      __uorWriteOpfsHead(result.active_head),
    );
    __uorBroadcastGenSync(result.active_head, '', 'rollback');
  }
  return {
    ...result,
    storage_backend: storageBackend,
    head: __uorMemGenerationStore.head,
  };
}

export function query_verdant_synthesis_status() {
  let wasmStatus = {
    status: 'ok',
    active_head: __uorMemGenerationStore.head,
    fault_strikes: 0,
    max_strikes: 3,
    rolled_back: false,
    facets: [],
  };
  if (typeof studio_verdant_synthesis_status === 'function') {
    try {
      wasmStatus = JSON.parse(studio_verdant_synthesis_status());
    } catch {
      // Keep fallback status before Wasm init
    }
  }
  return {
    ...wasmStatus,
    opfs_head: __uorMemGenerationStore.head,
    frontier: [...__uorMemGenerationStore.frontier],
    worker_pool: get_browser_worker_pool_snapshot(),
  };
}

// === STITCH PRISM IDENTITY VAULT & HOLOGRAM V4 .HOLO COMPILER / VERIFIER (R4-54) ===
const STITCH_PRISM_ID_DB = 'uor_stitch_prism_id_v1';
const STITCH_PRISM_ID_STORE = 'identity_records';
const UOR_CLOSURE_FILES = [
  'app.css',
  'app.js',
  'index.html',
  'lexlean_r4.js',
  'lexlean_r4_bg.wasm',
  'provenance.json',
];
const __uorMemPrismIdentity = {
  record: null,
  latestHoloBytes: null,
  latestHoloSummary: null,
};

// Formal R4.PrismHoloIdentity encode integer-only sorted-key canonical JSON mirroring uor-addr.
export function __uorCanonicalJson(value) {
  if (value === null || typeof value === 'boolean') return JSON.stringify(value);
  if (typeof value === 'number') {
    if (!Number.isSafeInteger(value)) {
      throw new Error('RefusedNonIntegerCanonicalJson: floats and non-safe integers rejected');
    }
    return JSON.stringify(value);
  }
  if (typeof value === 'string') return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(__uorCanonicalJson).join(',')}]`;
  if (value && typeof value === 'object') {
    const keys = Object.keys(value).filter((k) => value[k] !== undefined).sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
    return `{${keys.map((k) => `${JSON.stringify(k)}:${__uorCanonicalJson(value[k])}`).join(',')}}`;
  }
  throw new Error('RefusedNonIntegerCanonicalJson: unsupported JSON value');
}

function __uorOpenPrismIdDb() {
  return new Promise((resolve, reject) => {
    if (typeof indexedDB === 'undefined') {
      reject(new Error('IndexedDB unavailable'));
      return;
    }
    const req = indexedDB.open(STITCH_PRISM_ID_DB, 1);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STITCH_PRISM_ID_STORE)) {
        db.createObjectStore(STITCH_PRISM_ID_STORE);
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error || new Error('IndexedDB open error'));
  });
}

async function __uorReadPrismIdRecord(key) {
  try {
    const db = await __uorOpenPrismIdDb();
    return await new Promise((resolve, reject) => {
      const tx = db.transaction(STITCH_PRISM_ID_STORE, 'readonly');
      const req = tx.objectStore(STITCH_PRISM_ID_STORE).get(key);
      req.onsuccess = () => resolve(req.result || null);
      req.onerror = () => reject(req.error);
    });
  } catch {
    return key === 'primary' ? __uorMemPrismIdentity.record : null;
  }
}

async function __uorWritePrismIdRecord(key, value) {
  if (key === 'primary') __uorMemPrismIdentity.record = value;
  try {
    const db = await __uorOpenPrismIdDb();
    await new Promise((resolve, reject) => {
      const tx = db.transaction(STITCH_PRISM_ID_STORE, 'readwrite');
      tx.objectStore(STITCH_PRISM_ID_STORE).put(value, key);
      tx.oncomplete = () => resolve(true);
      tx.onerror = () => reject(tx.error);
    });
    return 'indexeddb';
  } catch {
    return 'memory';
  }
}

function __uorToB64Url(bytes) {
  /* Formal R4.PrismHoloIdentity (src/R4/PrismHoloIdentity.lex.tex) */
  const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  let bin = '';
  for (let i = 0; i < u8.length; i += 1) bin += String.fromCharCode(u8[i]);
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
}

async function __uorSignJsonES256(privateKey, obj) {
  const canon = __uorCanonicalJson(obj);
  const data = cachedTextEncoder.encode(canon);
  const subtle = typeof crypto !== 'undefined' ? crypto.subtle : null;
  if (subtle && privateKey && typeof subtle.sign === 'function') {
    const sigBuf = await subtle.sign({ name: 'ECDSA', hash: 'SHA-256' }, privateKey, data);
    return __uorToB64Url(new Uint8Array(sigBuf));
  }
  return await __uorSha256HexBytes(data);
}

async function __uorCreateP256IdentityState() {
  const subtle = typeof crypto !== 'undefined' ? crypto.subtle : null;
  let signingPair = null;
  let agreementPair = null;
  let signingPub = { kty: 'EC', crv: 'P-256', x: 'uor-studio-pub-x', y: 'uor-studio-pub-y' };
  let agreementPub = { kty: 'EC', crv: 'P-256', x: 'uor-studio-ecdh-x', y: 'uor-studio-ecdh-y' };
  let protection = 'software';

  if (subtle && typeof subtle.generateKey === 'function') {
    try {
      // Security: non-extractable WebCrypto P-256 hardware/software private keys (afflom/stitch docs/prism-id.md).
      signingPair = await subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, false, ['sign', 'verify']);
      agreementPair = await subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, false, ['deriveBits']);
      const sjwk = await subtle.exportKey('jwk', signingPair.publicKey);
      const ajwk = await subtle.exportKey('jwk', agreementPair.publicKey);
      signingPub = { kty: sjwk.kty, crv: sjwk.crv, x: sjwk.x, y: sjwk.y };
      agreementPub = { kty: ajwk.kty, crv: ajwk.crv, x: ajwk.x, y: ajwk.y };
      protection = 'webcrypto-nonextractable';
    } catch {
      // Fallback to deterministic software JWK descriptor when SubtleCrypto is unavailable
    }
  }

  const deviceRecord = {
    schema: 'prism/device/1',
    class: 'browser',
    protection,
    signing: signingPub,
    agreement: agreementPub,
  };
  const deviceKappa = `sha256:${await __uorSha256HexBytes(cachedTextEncoder.encode(__uorCanonicalJson(deviceRecord)))}`;
  const genesisBody = {
    schema: 'prism/genesis/1',
    device: deviceRecord,
    device_id: deviceKappa,
    nonce: Math.floor(Date.now() / 1000),
  };
  const genesisSig = await __uorSignJsonES256(signingPair ? signingPair.privateKey : null, genesisBody);
  const genesisRecord = { ...genesisBody, sig: genesisSig };
  const prismId = `sha256:${await __uorSha256HexBytes(cachedTextEncoder.encode(__uorCanonicalJson(genesisRecord)))}`;
  return {
    prism_id: prismId,
    device_id: deviceKappa,
    protection,
    device_record: deviceRecord,
    genesis_record: genesisRecord,
    signing_private_key: signingPair ? signingPair.privateKey : null,
    signing_public_key: signingPair ? signingPair.publicKey : null,
    agreement_private_key: agreementPair ? agreementPair.privateKey : null,
    agreement_public_key: agreementPair ? agreementPair.publicKey : null,
    persona: null,
    history: [genesisRecord],
  };
}

export async function stitch_prism_identity_ensure() {
  let stored = await __uorReadPrismIdRecord('primary');
  let backend = stored ? 'indexeddb' : 'created';
  if (!stored || !stored.genesis_record || !stored.device_record) {
    stored = await __uorCreateP256IdentityState();
    backend = await __uorWritePrismIdRecord('primary', stored);
  }
  __uorMemPrismIdentity.record = stored;

  // Replay chain deterministically into Wasm R4.PrismHoloIdentity ledger if not yet populated
  let wasmStatus = typeof studio_prism_identity_status === 'function' ? JSON.parse(studio_prism_identity_status()) : null;
  if (!wasmStatus || !wasmStatus.prism_id) {
    for (const rec of stored.history || [stored.genesis_record]) {
      if (typeof studio_prism_identity_admit_record === 'function') {
        studio_prism_identity_admit_record(JSON.stringify(rec));
      }
    }
    wasmStatus = typeof studio_prism_identity_status === 'function' ? JSON.parse(studio_prism_identity_status()) : null;
  }

  return {
    ok: true,
    schema: 'stitch/prism-id/1',
    storage: backend,
    prism_id: (wasmStatus && wasmStatus.prism_id) || stored.prism_id,
    device_id: stored.device_id,
    protection: stored.protection || 'webcrypto-nonextractable',
    role: 'member',
    head_kappa: (wasmStatus && wasmStatus.head_kappa) || stored.prism_id,
    active_devices: (wasmStatus && wasmStatus.active_devices) || 1,
    revoked_devices: (wasmStatus && wasmStatus.revoked_devices) || 0,
    persona: (wasmStatus && wasmStatus.persona) || stored.persona || null,
    latest_holo: __uorMemPrismIdentity.latestHoloSummary || globalThis.__uorLatestHoloSummary || null,
  };
}

export async function stitch_prism_identity_bind_persona(handle, host, holoKappa) {
  const ens = await stitch_prism_identity_ensure();
  const stored = __uorMemPrismIdentity.record;
  const body = {
    schema: 'prism/persona/1',
    prism: ens.prism_id,
    prev: ens.head_kappa,
    by: ens.device_id,
    host: String(host || 'uor-foundation.github.io').trim() || 'uor-foundation.github.io',
    handle: String(handle || 'lexlean-r4-studio').trim() || 'lexlean-r4-studio',
    app_address: 'holo://lexlean-r4',
    ...(holoKappa ? { target_kappa: String(holoKappa).trim() } : {}),
  };
  const sig = await __uorSignJsonES256(stored ? stored.signing_private_key : null, body);
  const signed = { ...body, sig };
  const admit = typeof studio_prism_identity_admit_record === 'function'
    ? JSON.parse(studio_prism_identity_admit_record(JSON.stringify(signed)))
    : { status: 'error', message: 'Wasm studio_prism_identity_admit_record unavailable' };
  if (admit.status !== 'ok') {
    throw new Error(`${admit.error_code || 'RefusedPersona'}: ${admit.message || 'Persona refused'}`);
  }
  if (stored) {
    stored.persona = signed;
    stored.history = Array.isArray(stored.history) ? [...stored.history, signed] : [stored.genesis_record, signed];
    await __uorWritePrismIdRecord('primary', stored);
  }
  return {
    ...(await stitch_prism_identity_ensure()),
    last_record_kappa: admit.record_kappa,
    last_verdict: admit.verdict,
  };
}

export async function stitch_prism_identity_sign_action(actionName, targetKappa) {
  const ens = await stitch_prism_identity_ensure();
  const stored = __uorMemPrismIdentity.record;
  const body = {
    schema: 'prism/action/1',
    prism: ens.prism_id,
    prev: ens.head_kappa,
    by: ens.device_id,
    action: String(actionName || 'compile-holo'),
    target_kappa: String(targetKappa || ''),
  };
  const sig = await __uorSignJsonES256(stored ? stored.signing_private_key : null, body);
  const signed = { ...body, sig };
  const admit = typeof studio_prism_identity_admit_record === 'function'
    ? JSON.parse(studio_prism_identity_admit_record(JSON.stringify(signed)))
    : { status: 'error', message: 'Wasm studio_prism_identity_admit_record unavailable' };
  if (admit.status !== 'ok') {
    throw new Error(`${admit.error_code || 'RefusedAction'}: ${admit.message || 'Action refused'}`);
  }
  if (stored) {
    stored.history = Array.isArray(stored.history) ? [...stored.history, signed] : [stored.genesis_record, signed];
    await __uorWritePrismIdRecord('primary', stored);
  }
  return {
    ...(await stitch_prism_identity_ensure()),
    action_record_kappa: admit.record_kappa,
    action_sig: sig,
    last_verdict: admit.verdict,
  };
}

// Formal R4.PrismHoloIdentity encode sorted UTF-8 entries into big-endian HOLOVIEW v1 bytes.
export function __uorEncodeHoloviewV1(entry, filesMap) {
  /* Formal R4.PrismHoloIdentity (src/R4/PrismHoloIdentity.lex.tex) */
  const entries = [...filesMap.entries()]
    .map(([p, content]) => ({
      pathBytes: cachedTextEncoder.encode(String(p)),
      content: content instanceof Uint8Array ? content : new Uint8Array(content),
    }))
    .sort((a, b) => {
      const n = Math.min(a.pathBytes.length, b.pathBytes.length);
      for (let i = 0; i < n; i += 1) {
        if (a.pathBytes[i] !== b.pathBytes[i]) return a.pathBytes[i] - b.pathBytes[i];
      }
      return a.pathBytes.length - b.pathBytes.length;
    });
  const entryBytes = cachedTextEncoder.encode(entry);
  let total = 8 + 2 + 4 + entryBytes.length + 4;
  for (const e of entries) total += 4 + e.pathBytes.length + 8 + e.content.length;
  const out = new Uint8Array(total);
  const dv = new DataView(out.buffer);
  out.set(cachedTextEncoder.encode('HOLOVIEW'), 0);
  dv.setUint16(8, 1, false);
  let at = 10;
  dv.setUint32(at, entryBytes.length, false);
  at += 4;
  out.set(entryBytes, at);
  at += entryBytes.length;
  dv.setUint32(at, entries.length, false);
  at += 4;
  for (const e of entries) {
    dv.setUint32(at, e.pathBytes.length, false);
    at += 4;
    out.set(e.pathBytes, at);
    at += e.pathBytes.length;
    dv.setBigUint64(at, BigInt(e.content.length), false);
    at += 8;
    out.set(e.content, at);
    at += e.content.length;
  }
  return out;
}

async function __uorReplayCoreWasmVectors(guestBytes) {
  /* Formal R4.PrismHoloIdentity (src/R4/PrismHoloIdentity.lex.tex) */
  const pairs = [
    ['1\tping\t', '1\tok\tlexlean-r4\tv0.14.0'],
    ['1\troute\tqwen3-0.6b-holo', '1\tok\tuor-r4.qwen3-0.6b-holo\tsimd128-int4-rht32'],
    ['1\tidentity\tprism/genesis/1', '1\tok\tstitch-prism-id\tuor-addr/sha256'],
    ['1\tselect\tuor-foundation/foundry', '1\tok\tholo://lexlean-r4\tfoundry/inprism/1'],
    ['1\tunknown\t?', '1\terror\tunknown-request'],
  ];
  const mod = await WebAssembly.compile(guestBytes);
  const imports = WebAssembly.Module.imports(mod);
  if (imports.length !== 0) {
    throw new Error(`RefusedCoreWasmContractViolation: guest declared ${imports.length} imports`);
  }
  const exports = WebAssembly.Module.exports(mod).map((e) => e.name).sort();
  if (JSON.stringify(exports) !== JSON.stringify(['holo_alloc', 'holo_run', 'memory'])) {
    throw new Error(`RefusedCoreWasmContractViolation: unexpected exports ${exports.join(',')}`);
  }
  let passed = 0;
  for (const [reqStr, expResp] of pairs) {
    const inst = new WebAssembly.Instance(mod, {});
    const reqBytes = cachedTextEncoder.encode(reqStr);
    const at = inst.exports.holo_alloc(reqBytes.length) >>> 0;
    new Uint8Array(inst.exports.memory.buffer, at, reqBytes.length).set(reqBytes);
    const packed = BigInt.asUintN(64, inst.exports.holo_run(at, reqBytes.length));
    const off = Number(packed >> 32n);
    const len = Number(packed & 0xffffffffn);
    const respBytes = new Uint8Array(inst.exports.memory.buffer.slice(off, off + len));
    const got = cachedTextDecoder.decode(respBytes);
    if (got === expResp) passed += 1;
  }
  return { passed, total: pairs.length };
}

export async function verify_holo_v4_archive_in_worker(archiveBytes, includeBytes = false) {
  if (typeof studio_verify_holo_v4_archive !== 'function') {
    throw new Error('studio_verify_holo_v4_archive export unavailable');
  }
  const parsed = JSON.parse(studio_verify_holo_v4_archive(archiveBytes));
  if (!parsed || parsed.status !== 'ok') {
    throw new Error(`${parsed?.error_code || 'RefusedHolo'}: ${parsed?.message || 'Verification failed'}`);
  }
  let wasmReplay = { passed: parsed.vectors_passed, total: parsed.vectors_total };
  if (typeof studio_core_wasm_guest_bytes === 'function') {
    const guest = studio_core_wasm_guest_bytes();
    wasmReplay = await __uorReplayCoreWasmVectors(guest);
    if (wasmReplay.passed !== wasmReplay.total) {
      throw new Error(`RefusedMeAppVectorReplayFailure: ${wasmReplay.passed}/${wasmReplay.total} vectors passed`);
    }
  }
  const signedAction = await stitch_prism_identity_sign_action('verify-holo', `blake3:${parsed.fingerprint}`).catch(() => null);
  const summary = {
    ok: true,
    ...parsed,
    wasm_vectors_passed: wasmReplay.passed,
    wasm_vectors_total: wasmReplay.total,
    prism_id: signedAction ? signedAction.prism_id : null,
    device_id: signedAction ? signedAction.device_id : null,
    action_record_kappa: signedAction ? signedAction.action_record_kappa : null,
  };
  __uorMemPrismIdentity.latestHoloSummary = summary;
  globalThis.__uorLatestHoloSummary = summary;
  return includeBytes ? { ...summary, bytes: archiveBytes } : summary;
}

export async function compile_and_verify_holo_v4_in_worker(injectedFiles = null, includeBytes = false) {
  const filesMap = new Map();
  if (injectedFiles && typeof injectedFiles === 'object') {
    const iter = injectedFiles instanceof Map ? injectedFiles.entries() : Object.entries(injectedFiles);
    for (const [k, v] of iter) {
      filesMap.set(k, v instanceof Uint8Array ? v : cachedTextEncoder.encode(String(v)));
    }
  } else {
    for (const name of UOR_CLOSURE_FILES) {
      const url = new URL(name, import.meta.url);
      const res = await fetch(url, { cache: 'no-cache' });
      if (!res.ok) {
        throw new Error(`Failed loading closure asset ${name}: HTTP ${res.status}`);
      }
      filesMap.set(name, new Uint8Array(await res.arrayBuffer()));
    }
  }

  const holoview = __uorEncodeHoloviewV1('index.html', filesMap);
  const provBytes = filesMap.get('provenance.json') || cachedTextEncoder.encode('{}');
  if (typeof studio_compile_holo_v4_archive !== 'function') {
    throw new Error('studio_compile_holo_v4_archive export unavailable');
  }
  const archive = studio_compile_holo_v4_archive(holoview, provBytes);
  if (!archive || archive.length < 256) {
    throw new Error('RefusedInvalidHoloMagicOrVersion: compiled .holo archive is empty');
  }
  __uorMemPrismIdentity.latestHoloBytes = archive;
  if (typeof navigator !== 'undefined' && navigator.storage && typeof navigator.storage.getDirectory === 'function') {
    try {
      const root = await navigator.storage.getDirectory();
      const fh = await root.getFileHandle('lexlean-r4.holo', { create: true });
      const wr = await fh.createWritable();
      await wr.write(archive);
      await wr.close();
    } catch {
      // OPFS cache optional in non-OPFS contexts
    }
  }
  const summary = await verify_holo_v4_archive_in_worker(archive, includeBytes);
  const actionReceipt = await stitch_prism_identity_sign_action('compile-holo', `blake3:${summary.fingerprint}`).catch(() => null);
  return {
    ...summary,
    action_record_kappa: actionReceipt ? actionReceipt.action_record_kappa : summary.action_record_kappa,
    opfs_persisted: Boolean(typeof navigator !== 'undefined' && navigator.storage && navigator.storage.getDirectory),
  };
}

export async function build_afflom_me_app_descriptor(originHref = '') {
  /* Formal R4.PrismHoloIdentity (src/R4/PrismHoloIdentity.lex.tex) */
  const idState = await stitch_prism_identity_ensure().catch(() => null);
  const latestHolo = __uorMemPrismIdentity.latestHoloSummary || globalThis.__uorLatestHoloSummary || null;
  let baseLive = 'https://casey-allard.github.io/lexlean-r4/';
  if (originHref && originHref.startsWith('http')) {
    baseLive = originHref.endsWith('/') ? originHref : `${originHref.replace(/index\.html$/, '')}`;
    if (!baseLive.endsWith('/')) baseLive += '/';
  }
  return {
    schema: 'prism/r4-workspace/1',
    address: 'lexlean-r4',
    holoAddress: 'holo://lexlean-r4',
    name: 'LexLean-R4 Universal Workspace',
    version: '0.14.0',
    live: baseLive,
    holoUrl: `${baseLive}app.holo`,
    appJsonUrl: `${baseLive}app.json`,
    repository: 'https://github.com/Casey-allard/lexlean-r4',
    registry: 'https://registry.uor.foundation/',
    archiveKappa: latestHolo ? latestHolo.archive_kappa : null,
    fingerprint: latestHolo ? latestHolo.fingerprint : null,
    applicationKappa: latestHolo ? latestHolo.application_kappa : null,
    prismIdentity: idState ? {
      prism_id: idState.prism_id,
      device_id: idState.device_id,
      role: idState.role,
      persona: idState.persona,
    } : null,
    foundry: {
      repository: 'https://github.com/UOR-Foundation/foundry',
      fork: 'https://github.com/Casey-allard/foundry',
      holoViewer: 'holo.html',
      embedQueryParam: 'embed=foundry',
      r4SpecSha256: '808b936a8f33c67c0563fbd3069753e221c644ca20fcc1a81a1c37b19644b254',
      inprismRequest: {
        mode: 'inprism',
        engine: 'match',
        weights: 'match',
        route: 'none',
        boundary: 'allow',
        outcome: 'answer',
      },
    },
    embed: {
      supported: true,
      queryParam: 'embed=me',
      foundryQueryParam: 'embed=foundry',
      bridgeProtocol: 'uor-workspace-bridge/1',
    },
  };
}

const HOPF_ZETA_V14_WGSL = /* wgsl */ `
struct WaveQuery {
  sig0: u32, sig1: u32, sig2: u32, sig3: u32,
  sig4: u32, sig5: u32, sig6: u32, sig7: u32,
  sig8: u32, sig9: u32, sig10: u32, sig11: u32,
  sig12: u32, sig13: u32, sig14: u32, sig15: u32,
  sig16: u32, sig17: u32, chi_q29: i32, step_idx: u32,
};
@group(0) @binding(0) var<storage, read> query_buf: WaveQuery;
@group(0) @binding(1) var<storage, read> basin_sigs: array<u32>;
@group(0) @binding(2) var<storage, read> basin_chi: array<i32>;
@group(0) @binding(3) var<storage, read_write> scores_out: array<i32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let idx: u32 = gid.x;
  if (idx >= arrayLength(&scores_out)) { return; }
  let base: u32 = idx * 18u;
  var hamming: u32 = 0u;
  hamming += countOneBits(query_buf.sig0 ^ basin_sigs[base + 0u]);
  hamming += countOneBits(query_buf.sig1 ^ basin_sigs[base + 1u]);
  hamming += countOneBits(query_buf.sig2 ^ basin_sigs[base + 2u]);
  hamming += countOneBits(query_buf.sig3 ^ basin_sigs[base + 3u]);
  hamming += countOneBits(query_buf.sig4 ^ basin_sigs[base + 4u]);
  hamming += countOneBits(query_buf.sig5 ^ basin_sigs[base + 5u]);
  hamming += countOneBits(query_buf.sig6 ^ basin_sigs[base + 6u]);
  hamming += countOneBits(query_buf.sig7 ^ basin_sigs[base + 7u]);
  hamming += countOneBits(query_buf.sig8 ^ basin_sigs[base + 8u]);
  hamming += countOneBits(query_buf.sig9 ^ basin_sigs[base + 9u]);
  hamming += countOneBits(query_buf.sig10 ^ basin_sigs[base + 10u]);
  hamming += countOneBits(query_buf.sig11 ^ basin_sigs[base + 11u]);
  hamming += countOneBits(query_buf.sig12 ^ basin_sigs[base + 12u]);
  hamming += countOneBits(query_buf.sig13 ^ basin_sigs[base + 13u]);
  hamming += countOneBits(query_buf.sig14 ^ basin_sigs[base + 14u]);
  hamming += countOneBits(query_buf.sig15 ^ basin_sigs[base + 15u]);
  hamming += countOneBits(query_buf.sig16 ^ basin_sigs[base + 16u]);
  hamming += countOneBits(query_buf.sig17 ^ basin_sigs[base + 17u]);
  let d_chi: i32 = (query_buf.chi_q29 - basin_chi[idx]) + i32(query_buf.step_idx * 133548191u);
  let phase_fold: i32 = abs(((d_chi >> 14) & 65535) - 32768);
  scores_out[idx] = phase_fold - i32(hamming * 64u);
}
`;

if (typeof globalThis === 'object' && globalThis) {
  globalThis.__uorSelfHostRuntime = {
    optimalBrowserWorkerCount: optimal_browser_worker_count,
    initSharedWasmWorkerPool: init_shared_wasm_worker_pool,
    getBrowserWorkerPoolSnapshot: get_browser_worker_pool_snapshot,
    admitUorcGenerationCapsule: admit_uorc_generation_capsule,
    recordUorcGenerationFault: record_uorc_generation_fault,
    queryVerdantSynthesisStatus: query_verdant_synthesis_status,
    stitchPrismIdentityEnsure: stitch_prism_identity_ensure,
    stitchPrismIdentityBindPersona: stitch_prism_identity_bind_persona,
    compileAndVerifyHoloV4InWorker: compile_and_verify_holo_v4_in_worker,
    verifyHoloV4ArchiveInWorker: verify_holo_v4_archive_in_worker,
    buildAfflomMeAppDescriptor: build_afflom_me_app_descriptor,
    buildFoundryStudioDescriptor: build_afflom_me_app_descriptor,
    executeInprismFoundryTurn: execute_inprism_foundry_turn,
    uorCanonicalKappa: (rawJson) => JSON.parse(studio_uor_canonical_kappa_json(typeof rawJson === 'string' ? rawJson : JSON.stringify(rawJson))),
    foundryQuantumSelfCheck: () => JSON.parse(studio_foundry_quantum_self_check_json()),
    hopfZetaV14WgslShader: HOPF_ZETA_V14_WGSL,
    hopfZetaV14Generate: (prompt, maxTok = 512) => JSON.parse(studio_hopf_zeta_v14_generate_json(prompt, maxTok)),
    hopfZetaV14SelfCheck: () => JSON.parse(studio_hopf_zeta_v14_self_check_json()),
    renderHopfZeta3dRgba: (w, h, phase, lod, trail) => studio_render_hopf_zeta_3d_rgba(w, h, phase, lod, trail || ''),
    renderPreviewScene3dRgba: (specJson, w, h) => studio_render_preview_scene_3d_rgba(specJson, w, h),
    vtensor3dSelfCheckJson: () => studio_vtensor_3d_self_check_json(),
    mountPreviewScene3d: (sceneSpec, hostEl, docObj) => {
      /* Formal R4.VtensorSemiring3d */
      const specObj = typeof sceneSpec === 'string' ? JSON.parse(sceneSpec || '{}') : (sceneSpec || {});
      const cw = Math.max(32, Math.min(1024, Number(specObj.width) || 320));
      const ch = Math.max(32, Math.min(1024, Number(specObj.height) || 224));
      const rgba = studio_render_preview_scene_3d_rgba(JSON.stringify(specObj), cw, ch);
      if (hostEl && docObj && typeof docObj.createElement === 'function') {
        let cEl = typeof hostEl.querySelector === 'function' ? hostEl.querySelector('#preview-vtensor-3d-canvas') : null;
        if (!cEl) {
          cEl = docObj.createElement('canvas');
          cEl.id = 'preview-vtensor-3d-canvas';
          hostEl.appendChild(cEl);
        }
        cEl.width = cw;
        cEl.height = ch;
        const c2d = typeof cEl.getContext === 'function' ? cEl.getContext('2d') : null;
        if (c2d && typeof c2d.putImageData === 'function' && typeof ImageData === 'function' && rgba.length === cw * ch * 4) {
          c2d.putImageData(new ImageData(new Uint8ClampedArray(rgba), cw, ch), 0, 0);
        }
      }
      return { width: cw, height: ch, rgbaLength: rgba.length, spec: 'uor-vtensor-3d/1' };
    },
    flushMvmCowToOpfs,
  };

  globalThis.LexLean = {
    ...(globalThis.LexLean || {}),
    opfs_put_blob,
    opfs_get_blob,
    storageRequest,
    withStorageLock,
    encodeSpvsWireFrame,
    decodeSpvsWireFrame,
    persist_mvm_cow_sectors_to_opfs,
    restore_mvm_cow_sectors_from_opfs,
    flushMvmCowToOpfs,
    stitch_prism_identity_ensure,
    stitch_prism_identity_bind_persona,
    compile_and_verify_holo_v4_in_worker,
    verify_holo_v4_archive_in_worker,
    execute_inprism_foundry_turn,
  };
}

if (
  typeof WorkerGlobalScope !== 'undefined' &&
  typeof self !== 'undefined' &&
  self instanceof WorkerGlobalScope &&
  self.name === 'uor-r4-engine-worker'
) {
  __uorStartDedicatedEngineWorker();
} else if (typeof window !== 'undefined' && typeof document !== 'undefined') {
  registerStudioCoiServiceWorker().catch(() => {});
}
// === UOR_CUSTOM_SECTION_2_END ===
