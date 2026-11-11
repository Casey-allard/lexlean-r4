/**
 * Stateless SpiralCore `AF_VSOCK` (`CID 3 -> CID 2`) Edge Relay (`R4-39`).
 *
 * Zero-dependency ES module compatible with Cloudflare Workers (`WebSocketPair` + `fetch`)
 * and Node.js (`relay/local-relay.mjs`). Supports:
 *   - `GET /health` -> `{ ok: true, service: "lexlean-r4-spiralcore-relay" }`
 *   - `Upgrade: websocket` on `/spvs`
 *   - CORS-enabled `POST /spvs` + `OPTIONS` preflight (`Access-Control-Allow-Origin: *`)
 *
 * Operations:
 *   1. `dns_query`      -> RFC 8484 JSON DoH (`https://cloudflare-dns.com/dns-query`)
 *   2. `http_request`   -> `https://` only, strict SSRF private/loopback/redirect blocklist, `<= 2 MiB` body cap, readable `text_excerpt`
 *   3. `web_search`     -> Multi-source search (DuckDuckGo, Wikipedia Opensearch, HN Algolia, GitHub Search) with URL deduplication
 *   4. `git_smart_http` -> Strict GitHub host allowlist (`github.com` / `api.github.com`), rejecting non-GitHub `auth_bearer` as `HostNotAllowlistedForSecret`
 */

export const SERVICE_NAME = "lexlean-r4-spiralcore-relay";
export const MAX_HTTP_BODY_BYTES = 2 * 1024 * 1024; // 2 MiB hard cap for http_request
export const MAX_GIT_BODY_BYTES = 8 * 1024 * 1024; // 8 MiB packfile cap for git_smart_http
const MAX_TEXT_EXCERPT_CHARS = 16_384;
const MAX_REDIRECT_HOPS = 5;
const RELAY_USER_AGENT = "lexlean-r4-spiralcore-relay/1.0 (+https://github.com/Casey-allard/lexlean-r4)";

export const CORS_HEADERS = Object.freeze({
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type, Authorization, X-SpiralCore-Magic",
  "Access-Control-Max-Age": "86400",
});

const ALLOWED_DNS_TYPES = new Set([
  "A",
  "AAAA",
  "TXT",
  "MX",
  "CNAME",
  "NS",
  "SOA",
  "SRV",
  "PTR",
  "CAA",
]);

const DNS_TYPE_NAMES = Object.freeze({
  1: "A",
  2: "NS",
  5: "CNAME",
  6: "SOA",
  12: "PTR",
  15: "MX",
  16: "TXT",
  28: "AAAA",
  33: "SRV",
  257: "CAA",
});

const ALLOWED_HTTP_METHODS = new Set(["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE"]);
const ALLOWLISTED_GITHUB_HOSTS = new Set(["github.com", "api.github.com"]);

class RelayError extends Error {
  constructor(errorCode, status, message) {
    super(message);
    this.name = "RelayError";
    this.errorCode = errorCode;
    this.status = status;
  }
}

export function bytesToBase64(bytes) {
  const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  if (u8.byteLength === 0) {
    return "";
  }
  const chunkSize = 16_384;
  let binary = "";
  for (let offset = 0; offset < u8.byteLength; offset += chunkSize) {
    const slice = u8.subarray(offset, Math.min(offset + chunkSize, u8.byteLength));
    binary += String.fromCharCode.apply(null, slice);
  }
  return btoa(binary);
}

export function base64ToBytes(b64) {
  if (typeof b64 !== "string") {
    throw new RelayError("InvalidBase64Payload", 400, "Base64 payload must be a string");
  }
  const trimmed = b64.replace(/\s+/g, "").replace(/-/g, "+").replace(/_/g, "/");
  if (trimmed.length === 0) {
    return new Uint8Array(0);
  }
  const padded = trimmed.length % 4 === 0 ? trimmed : trimmed + "=".repeat(4 - (trimmed.length % 4));
  let binary;
  try {
    binary = atob(padded);
  } catch {
    throw new RelayError("InvalidBase64Payload", 400, "Malformed base64 payload");
  }
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    out[i] = binary.charCodeAt(i) & 0xff;
  }
  return out;
}

function utf8ToBase64(str) {
  return bytesToBase64(new TextEncoder().encode(str));
}

function makeOkResponse(reqId, status, data) {
  return {
    req_id: reqId,
    ok: true,
    status,
    data,
    error_code: null,
    error_message: null,
  };
}

function makeErrorResponse(reqId, status, errorCode, errorMessage) {
  return {
    req_id: reqId,
    ok: false,
    status,
    data: null,
    error_code: errorCode,
    error_message: errorMessage,
  };
}

function isBlockedIpv4Octets(a, b, c, _d) {
  if (a === 0) return true; // 0.0.0.0/8 unspecified/this-host
  if (a === 10) return true; // 10.0.0.0/8 private
  if (a === 100 && b >= 64 && b <= 127) return true; // 100.64.0.0/10 CGNAT
  if (a === 127) return true; // 127.0.0.0/8 loopback
  if (a === 169 && b === 254) return true; // 169.254.0.0/16 link-local & cloud metadata
  if (a === 172 && b >= 16 && b <= 31) return true; // 172.16.0.0/12 private
  if (a === 192 && b === 0 && (c === 0 || c === 2)) return true; // 192.0.0.0/24 & TEST-NET-1
  if (a === 192 && b === 168) return true; // 192.168.0.0/16 private
  if (a === 198 && (b === 18 || b === 19)) return true; // 198.18.0.0/15 benchmark
  if (a === 198 && b === 51 && c === 100) return true; // TEST-NET-2
  if (a === 203 && b === 0 && c === 113) return true; // TEST-NET-3
  if (a >= 224) return true; // 224.0.0.0/4 multicast & reserved
  return false;
}

function parseDottedIpv4(host) {
  const parts = host.split(".");
  if (parts.length !== 4) {
    return null;
  }
  const octets = [];
  for (const part of parts) {
    if (!/^\d{1,3}$/.test(part)) {
      return null;
    }
    const val = Number(part);
    if (val < 0 || val > 255) {
      return null;
    }
    octets.push(val);
  }
  return octets;
}

function parseIpv6Bytes(rawIpv6) {
  const addr = rawIpv6.toLowerCase();
  if (addr.includes("%")) {
    // Zone identifiers (e.g. fe80::1%eth0) are always link-local/scoped
    return "zone-scoped";
  }
  const doubleColonParts = addr.split("::");
  if (doubleColonParts.length > 2) {
    return null;
  }

  function expandSide(side) {
    if (!side) return [];
    const tokens = side.split(":");
    const hextets = [];
    for (let i = 0; i < tokens.length; i += 1) {
      const tok = tokens[i];
      if (tok.includes(".")) {
        const v4 = parseDottedIpv4(tok);
        if (!v4) return null;
        hextets.push((v4[0] << 8) | v4[1], (v4[2] << 8) | v4[3]);
      } else {
        if (!/^[0-9a-f]{1,4}$/.test(tok)) return null;
        hextets.push(Number.parseInt(tok, 16));
      }
    }
    return hextets;
  }

  const left = expandSide(doubleColonParts[0]);
  if (!left) return null;

  let fullHextets;
  if (doubleColonParts.length === 2) {
    const right = expandSide(doubleColonParts[1]);
    if (!right) return null;
    const missing = 8 - (left.length + right.length);
    if (missing < 1) return null;
    fullHextets = [...left, ...new Array(missing).fill(0), ...right];
  } else {
    if (left.length !== 8) return null;
    fullHextets = left;
  }

  const bytes = new Uint8Array(16);
  for (let i = 0; i < 8; i += 1) {
    bytes[i * 2] = (fullHextets[i] >> 8) & 0xff;
    bytes[i * 2 + 1] = fullHextets[i] & 0xff;
  }
  return bytes;
}

function isBlockedIpv6(rawBracketedOrBare) {
  const bare = rawBracketedOrBare.replace(/^\[/, "").replace(/\]$/, "");
  const bytes = parseIpv6Bytes(bare);
  if (bytes === "zone-scoped" || bytes === null) {
    return true;
  }

  // ::/128 unspecified or ::1/128 loopback
  let first15Zero = true;
  for (let i = 0; i < 15; i += 1) {
    if (bytes[i] !== 0) {
      first15Zero = false;
      break;
    }
  }
  if (first15Zero && (bytes[15] === 0 || bytes[15] === 1)) {
    return true;
  }

  // ::ffff:0:0/96 IPv4-mapped IPv6 or ::/96 IPv4-compatible IPv6
  let first10Zero = true;
  for (let i = 0; i < 10; i += 1) {
    if (bytes[i] !== 0) {
      first10Zero = false;
      break;
    }
  }
  if (
    first10Zero &&
    ((bytes[10] === 0xff && bytes[11] === 0xff) || (bytes[10] === 0 && bytes[11] === 0))
  ) {
    return isBlockedIpv4Octets(bytes[12], bytes[13], bytes[14], bytes[15]);
  }

  // 64:ff9b::/96 NAT64 embedded IPv4
  if (
    bytes[0] === 0x00 &&
    bytes[1] === 0x64 &&
    bytes[2] === 0xff &&
    bytes[3] === 0x9b
  ) {
    return isBlockedIpv4Octets(bytes[12], bytes[13], bytes[14], bytes[15]);
  }

  // 2002::/16 6to4 embedded IPv4 in bytes 2..5
  if (bytes[0] === 0x20 && bytes[1] === 0x02) {
    if (isBlockedIpv4Octets(bytes[2], bytes[3], bytes[4], bytes[5])) {
      return true;
    }
  }

  // fc00::/7 Unique Local Address (fc00..fdff)
  if ((bytes[0] & 0xfe) === 0xfc) return true;
  // fe80::/10 Link-Local (fe80..febf)
  if (bytes[0] === 0xfe && (bytes[1] & 0xc0) === 0x80) return true;
  // ff00::/8 Multicast
  if (bytes[0] === 0xff) return true;
  // 2001:db8::/32 Documentation
  if (bytes[0] === 0x20 && bytes[1] === 0x01 && bytes[2] === 0x0d && bytes[3] === 0xb8) return true;
  // 100::/64 Discard-only
  if (bytes[0] === 0x01 && bytes[1] === 0x00 && bytes[2] === 0 && bytes[3] === 0) return true;

  return false;
}

export function isBlockedSsrfHost(rawHostname) {
  const host = String(rawHostname ?? "").trim().toLowerCase().replace(/\.+$/, "");
  if (!host) {
    return true;
  }
  if (
    host === "localhost" ||
    host.endsWith(".localhost") ||
    host === "local" ||
    host.endsWith(".local") ||
    host.endsWith(".internal") ||
    host === "metadata" ||
    host === "instance-data"
  ) {
    return true;
  }
  if (host.startsWith("[") || host.includes(":")) {
    return isBlockedIpv6(host);
  }
  const v4 = parseDottedIpv4(host);
  if (v4) {
    return isBlockedIpv4Octets(v4[0], v4[1], v4[2], v4[3]);
  }
  // Reject bare integer/hex/octal numeric hostnames that failed canonical IPv4 normalization
  if (/^(0x[0-9a-f]+|\d+)$/i.test(host)) {
    return true;
  }
  return false;
}

export function validateHttpsPublicUrl(rawUrl) {
  if (typeof rawUrl !== "string" || rawUrl.trim().length === 0) {
    throw new RelayError("InvalidUrl", 400, "Target URL must be a non-empty string");
  }
  let parsed;
  try {
    parsed = new URL(rawUrl.trim());
  } catch {
    throw new RelayError("InvalidUrl", 400, `Malformed URL: ${rawUrl}`);
  }
  if (parsed.protocol !== "https:") {
    throw new RelayError(
      "InsecureTransportRejected",
      400,
      `Disallowed protocol '${parsed.protocol}'; R4-39 admits https:// only`,
    );
  }
  if (parsed.username || parsed.password) {
    throw new RelayError("InsecureTransportRejected", 400, "Embedded URL credentials are not permitted");
  }
  if (isBlockedSsrfHost(parsed.hostname)) {
    throw new RelayError(
      "SsrfBlocked",
      403,
      `Outbound SSRF protection blocked private/loopback/link-local host '${parsed.hostname}'`,
    );
  }
  return parsed;
}

function containsSecretPlaceholder(value) {
  return typeof value === "string" && value.includes("{{MVM_SECRET:");
}

function decodeHtmlEntities(text) {
  return text
    .replace(/&nbsp;/gi, " ")
    .replace(/&amp;/gi, "&")
    .replace(/&lt;/gi, "<")
    .replace(/&gt;/gi, ">")
    .replace(/&quot;/gi, '"')
    .replace(/&#39;|&apos;/gi, "'")
    .replace(/&#x([0-9a-f]{1,6});/gi, (_, hex) => {
      const cp = Number.parseInt(hex, 16);
      return cp > 0 && cp <= 0x10ffff ? String.fromCodePoint(cp) : "";
    })
    .replace(/&#(\d{1,7});/g, (_, dec) => {
      const cp = Number.parseInt(dec, 10);
      return cp > 0 && cp <= 0x10ffff ? String.fromCodePoint(cp) : "";
    });
}

function htmlToReadableExcerpt(html) {
  const withoutNoise = html
    .replace(/<!--[\s\S]*?-->/g, " ")
    .replace(/<(script|style|noscript|svg|template|iframe)\b[^>]*>[\s\S]*?<\/\1>/gi, " ")
    .replace(/<\/(p|div|section|article|main|aside|header|footer|li|tr|h[1-6]|blockquote|pre)>/gi, "\n")
    .replace(/<br\s*\/?>/gi, "\n")
    .replace(/<[^>]+>/g, " ");
  return decodeHtmlEntities(withoutNoise)
    .replace(/[ \t\f\v]+/g, " ")
    .replace(/\n\s*\n+/g, "\n\n")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .join("\n")
    .slice(0, MAX_TEXT_EXCERPT_CHARS);
}

function extractReadableExcerpt(bytes, contentType) {
  const ct = (contentType || "").toLowerCase();
  const isHtml = ct.includes("text/html") || ct.includes("application/xhtml");
  const isTextLike =
    isHtml ||
    ct.startsWith("text/") ||
    ct.includes("json") ||
    ct.includes("xml") ||
    ct.includes("javascript") ||
    ct.includes("yaml") ||
    ct.includes("toml");
  if (!isTextLike) {
    return "";
  }
  const sample = bytes.byteLength > 262_144 ? bytes.subarray(0, 262_144) : bytes;
  const decoded = new TextDecoder("utf-8", { fatal: false }).decode(sample);
  if (isHtml) {
    return htmlToReadableExcerpt(decoded);
  }
  return decoded.replace(/\r\n/g, "\n").trim().slice(0, MAX_TEXT_EXCERPT_CHARS);
}

async function readBoundedBytes(response, maxBytes) {
  if (!response.body) {
    return { bytes: new Uint8Array(0), truncated: false };
  }
  const reader = response.body.getReader();
  const chunks = [];
  let total = 0;
  let truncated = false;

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      const chunk = value instanceof Uint8Array ? value : new Uint8Array(value);
      const remaining = maxBytes - total;
      if (chunk.byteLength > remaining) {
        if (remaining > 0) {
          chunks.push(chunk.subarray(0, remaining));
          total += remaining;
        }
        truncated = true;
        await reader.cancel().catch(() => {});
        break;
      }
      chunks.push(chunk);
      total += chunk.byteLength;
    }
  } finally {
    try {
      reader.releaseLock();
    } catch {
      // Reader already released after cancel
    }
  }

  const out = new Uint8Array(total);
  let offset = 0;
  for (const c of chunks) {
    out.set(c, offset);
    offset += c.byteLength;
  }
  return { bytes: out, truncated };
}

async function handleDnsQuery(params, fetchImpl) {
  const rawName = String(params?.name ?? params?.domain ?? "").trim();
  if (
    !rawName ||
    rawName.length > 253 ||
    !/^[a-zA-Z0-9._-]+$/.test(rawName) ||
    rawName.includes("..")
  ) {
    throw new RelayError("InvalidDnsName", 400, `Invalid DNS query domain '${rawName}'`);
  }

  const rawType = String(params?.type ?? params?.rr_type ?? "A").trim().toUpperCase();
  if (!ALLOWED_DNS_TYPES.has(rawType)) {
    throw new RelayError("InvalidDnsType", 400, `Unsupported DNS RR type '${rawType}'`);
  }

  const dohUrl = `https://cloudflare-dns.com/dns-query?name=${encodeURIComponent(rawName)}&type=${encodeURIComponent(rawType)}`;
  let resp;
  try {
    resp = await fetchImpl(dohUrl, {
      method: "GET",
      headers: {
        Accept: "application/dns-json",
        "User-Agent": RELAY_USER_AGENT,
      },
      signal: AbortSignal.timeout(8_000),
    });
  } catch (err) {
    throw new RelayError("UpstreamFetchFailed", 502, `DoH lookup failed: ${err?.message ?? String(err)}`);
  }

  if (!resp.ok) {
    throw new RelayError("UpstreamFetchFailed", 502, `DoH upstream returned HTTP ${resp.status}`);
  }

  let payload;
  try {
    payload = await resp.json();
  } catch {
    throw new RelayError("UpstreamFetchFailed", 502, "DoH upstream returned invalid JSON");
  }

  const rawAnswers = Array.isArray(payload?.Answer) ? payload.Answer : [];
  const answers = rawAnswers.map((ans) => {
    const numType = typeof ans.type === "number" ? ans.type : 0;
    const ttl = typeof ans.TTL === "number" ? ans.TTL : 0;
    return {
      name: String(ans.name ?? rawName),
      type: numType || rawType,
      type_name: DNS_TYPE_NAMES[numType] ?? rawType,
      TTL: ttl,
      ttl,
      data: String(ans.data ?? ""),
    };
  });

  return {
    status: 200,
    data: {
      name: rawName,
      type: rawType,
      rcode: typeof payload?.Status === "number" ? payload.Status : 0,
      ad: Boolean(payload?.AD),
      answers,
    },
  };
}

async function handleHttpRequest(params, fetchImpl) {
  const method = String(params?.method ?? "GET").trim().toUpperCase();
  if (!ALLOWED_HTTP_METHODS.has(method)) {
    throw new RelayError("InvalidHttpMethod", 400, `Unsupported HTTP method '${method}'`);
  }

  const initialUrl = validateHttpsPublicUrl(params?.url);
  const hostLower = initialUrl.hostname.toLowerCase();
  const isAllowlistedGitHub = ALLOWLISTED_GITHUB_HOSTS.has(hostLower);

  if (
    !isAllowlistedGitHub &&
    (Boolean(params?.auth_bearer) || containsSecretPlaceholder(params?.url))
  ) {
    throw new RelayError(
      "HostNotAllowlistedForSecret",
      403,
      `Host '${hostLower}' is not allowlisted for secret injection`,
    );
  }

  const reqHeaders = new Headers();
  reqHeaders.set("User-Agent", RELAY_USER_AGENT);
  if (params?.headers && typeof params.headers === "object") {
    for (const [k, v] of Object.entries(params.headers)) {
      const valStr = String(v ?? "");
      if (containsSecretPlaceholder(valStr)) {
        if (!isAllowlistedGitHub) {
          throw new RelayError(
            "HostNotAllowlistedForSecret",
            403,
            `Host '${hostLower}' is not allowlisted for {{MVM_SECRET:...}} substitution`,
          );
        }
        throw new RelayError(
          "MissingSecretSubstitution",
          400,
          "Secret placeholder must be resolved on host switch before upstream dispatch",
        );
      }
      const keyLower = k.toLowerCase();
      if (keyLower === "host" || keyLower === "connection" || keyLower === "content-length") {
        continue;
      }
      reqHeaders.set(k, valStr);
    }
  }

  let bodyBytes = null;
  if (params?.body_b64 !== undefined && params?.body_b64 !== null && params.body_b64 !== "") {
    bodyBytes = base64ToBytes(params.body_b64);
    if (bodyBytes.byteLength > MAX_HTTP_BODY_BYTES) {
      throw new RelayError("PayloadTooLarge", 413, "Request body exceeds 2 MiB limit");
    }
    const decodedSample = new TextDecoder().decode(bodyBytes.subarray(0, Math.min(bodyBytes.byteLength, 65_536)));
    if (!isAllowlistedGitHub && containsSecretPlaceholder(decodedSample)) {
      throw new RelayError(
        "HostNotAllowlistedForSecret",
        403,
        `Host '${hostLower}' is not allowlisted for {{MVM_SECRET:...}} in body`,
      );
    }
  }

  const requestedMax = Number(params?.max_bytes);
  const effectiveMaxBytes =
    Number.isFinite(requestedMax) && requestedMax > 0
      ? Math.min(Math.floor(requestedMax), MAX_HTTP_BODY_BYTES)
      : MAX_HTTP_BODY_BYTES;

  let currentUrl = initialUrl;
  let currentMethod = method;
  let currentBody = method === "GET" || method === "HEAD" ? undefined : bodyBytes ?? undefined;
  let resp = null;

  for (let hop = 0; hop <= MAX_REDIRECT_HOPS; hop += 1) {
    try {
      resp = await fetchImpl(currentUrl.toString(), {
        method: currentMethod,
        headers: reqHeaders,
        body: currentBody,
        redirect: "manual",
        signal: AbortSignal.timeout(12_000),
      });
    } catch (err) {
      if (err instanceof RelayError) throw err;
      throw new RelayError("UpstreamFetchFailed", 502, `Upstream HTTP fetch failed: ${err?.message ?? String(err)}`);
    }

    if (resp.status >= 300 && resp.status < 400) {
      const location = resp.headers.get("location");
      if (!location) {
        break;
      }
      if (hop === MAX_REDIRECT_HOPS) {
        throw new RelayError("TooManyRedirects", 502, "Exceeded maximum redirect hops (5)");
      }
      let resolvedNext;
      try {
        resolvedNext = new URL(location, currentUrl).toString();
      } catch {
        throw new RelayError("InvalidUrl", 502, "Upstream redirect returned invalid Location header");
      }
      const nextUrl = validateHttpsPublicUrl(resolvedNext);
      if (nextUrl.origin !== currentUrl.origin) {
        reqHeaders.delete("authorization");
        reqHeaders.delete("cookie");
      }
      if (resp.status === 301 || resp.status === 302 || resp.status === 303) {
        if (currentMethod !== "GET" && currentMethod !== "HEAD") {
          currentMethod = "GET";
          currentBody = undefined;
        }
      }
      currentUrl = nextUrl;
      continue;
    }
    break;
  }

  const { bytes, truncated } = await readBoundedBytes(resp, effectiveMaxBytes);
  const responseHeaders = {};
  resp.headers.forEach((value, key) => {
    responseHeaders[key.toLowerCase()] = value;
  });
  const contentType = resp.headers.get("content-type") ?? "";
  const textExcerpt = extractReadableExcerpt(bytes, contentType);

  return {
    status: resp.status,
    data: {
      status: resp.status,
      headers: responseHeaders,
      body_b64: bytesToBase64(bytes),
      byte_length: bytes.byteLength,
      truncated,
      content_type: contentType,
      text_excerpt: textExcerpt,
    },
  };
}

function unwrapDuckDuckGoUrl(rawHref) {
  const href = decodeHtmlEntities(rawHref.trim());
  const normalized = href.startsWith("//") ? `https:${href}` : href;
  try {
    const parsed = new URL(normalized);
    if (parsed.hostname.endsWith("duckduckgo.com") && parsed.pathname.startsWith("/l/")) {
      const uddg = parsed.searchParams.get("uddg");
      if (uddg) {
        const target = new URL(uddg);
        if (target.protocol === "https:") {
          return target.toString();
        }
      }
      return null;
    }
    if (parsed.protocol === "https:") {
      return parsed.toString();
    }
  } catch {
    return null;
  }
  return null;
}

function parseDuckDuckGoHtml(html, maxItems) {
  const results = [];
  const blockRegex = /<a[^>]*class="[^"]*result__a[^"]*"[^>]*href="([^"]+)"[^>]*>([\s\S]*?)<\/a>[\s\S]{0,1200}?(?:class="[^"]*result__snippet[^"]*"[^>]*>([\s\S]*?)<\/(?:a|div|span)>)?/gi;
  let match;
  while ((match = blockRegex.exec(html)) !== null && results.length < maxItems) {
    const url = unwrapDuckDuckGoUrl(match[1]);
    const title = htmlToReadableExcerpt(match[2]).replace(/\n+/g, " ").trim();
    const snippet = match[3] ? htmlToReadableExcerpt(match[3]).replace(/\n+/g, " ").trim() : "";
    if (url && title) {
      results.push({ title, url, snippet, source: "duckduckgo" });
    }
  }
  return results;
}

function normalizeUrlKey(rawUrl) {
  try {
    const u = new URL(rawUrl);
    if (u.protocol !== "https:") return null;
    u.hash = "";
    const cleanPath = u.pathname.length > 1 ? u.pathname.replace(/\/+$/, "") : u.pathname;
    return `${u.hostname.toLowerCase()}${cleanPath}${u.search}`;
  } catch {
    return null;
  }
}

async function handleWebSearch(params, fetchImpl) {
  const query = String(params?.query ?? "").trim();
  if (!query || query.length > 512) {
    throw new RelayError("InvalidSearchQuery", 400, "Search query must be 1..512 characters");
  }
  const rawLimit = Number(params?.limit);
  const limit = Number.isFinite(rawLimit) && rawLimit > 0 ? Math.min(Math.floor(rawLimit), 25) : 8;

  const commonHeaders = { "User-Agent": RELAY_USER_AGENT };

  const ddgHtmlTask = (async () => {
    const resp = await fetchImpl(
      `https://html.duckduckgo.com/html/?q=${encodeURIComponent(query)}`,
      { method: "GET", headers: commonHeaders, signal: AbortSignal.timeout(4_500) },
    );
    if (!resp.ok) return [];
    const html = await resp.text();
    return parseDuckDuckGoHtml(html, limit);
  })();

  const ddgInstantTask = (async () => {
    const resp = await fetchImpl(
      `https://api.duckduckgo.com/?q=${encodeURIComponent(query)}&format=json&no_html=1&skip_disambig=1`,
      { method: "GET", headers: commonHeaders, signal: AbortSignal.timeout(4_500) },
    );
    if (!resp.ok) return [];
    const data = await resp.json();
    const out = [];
    if (data?.AbstractURL && data?.AbstractText && String(data.AbstractURL).startsWith("https://")) {
      out.push({
        title: String(data.Heading || query),
        url: String(data.AbstractURL),
        snippet: String(data.AbstractText).slice(0, 400),
        source: "duckduckgo_ia",
      });
    }
    if (Array.isArray(data?.RelatedTopics)) {
      for (const topic of data.RelatedTopics) {
        if (out.length >= limit) break;
        if (topic?.FirstURL && topic?.Text && String(topic.FirstURL).startsWith("https://")) {
          out.push({
            title: String(topic.Text).split(" - ")[0].slice(0, 120),
            url: String(topic.FirstURL),
            snippet: String(topic.Text).slice(0, 400),
            source: "duckduckgo_ia",
          });
        }
      }
    }
    return out;
  })();

  const wikipediaTask = (async () => {
    const resp = await fetchImpl(
      `https://en.wikipedia.org/w/api.php?action=opensearch&search=${encodeURIComponent(query)}&limit=${Math.min(limit, 6)}&namespace=0&format=json`,
      { method: "GET", headers: commonHeaders, signal: AbortSignal.timeout(4_500) },
    );
    if (!resp.ok) return [];
    const arr = await resp.json();
    if (!Array.isArray(arr) || !Array.isArray(arr[1]) || !Array.isArray(arr[3])) {
      return [];
    }
    const titles = arr[1];
    const descriptions = Array.isArray(arr[2]) ? arr[2] : [];
    const urls = arr[3];
    const out = [];
    for (let i = 0; i < titles.length; i += 1) {
      const url = String(urls[i] ?? "");
      if (url.startsWith("https://")) {
        out.push({
          title: String(titles[i] ?? ""),
          url,
          snippet: String(descriptions[i] ?? `Wikipedia encyclopedia entry for ${titles[i]}`),
          source: "wikipedia",
        });
      }
    }
    return out;
  })();

  const hnAlgoliaTask = (async () => {
    const resp = await fetchImpl(
      `https://hn.algolia.com/api/v1/search?query=${encodeURIComponent(query)}&hitsPerPage=${Math.min(limit, 6)}`,
      { method: "GET", headers: commonHeaders, signal: AbortSignal.timeout(4_500) },
    );
    if (!resp.ok) return [];
    const data = await resp.json();
    const hits = Array.isArray(data?.hits) ? data.hits : [];
    const out = [];
    for (const hit of hits) {
      const candidateUrl =
        typeof hit?.url === "string" && hit.url.startsWith("https://")
          ? hit.url
          : hit?.objectID
            ? `https://news.ycombinator.com/item?id=${encodeURIComponent(String(hit.objectID))}`
            : "";
      const title = String(hit?.title || hit?.story_title || "").trim();
      if (!candidateUrl || !title) continue;
      const snippetRaw = hit?.story_text || hit?.comment_text || `Hacker News discussion (${hit?.points ?? 0} points)`;
      out.push({
        title,
        url: candidateUrl,
        snippet: htmlToReadableExcerpt(String(snippetRaw)).replace(/\n+/g, " ").slice(0, 360),
        source: "hn_algolia",
      });
    }
    return out;
  })();

  const githubSearchTask = (async () => {
    const resp = await fetchImpl(
      `https://api.github.com/search/repositories?q=${encodeURIComponent(query)}&per_page=${Math.min(limit, 5)}`,
      {
        method: "GET",
        headers: {
          ...commonHeaders,
          Accept: "application/vnd.github+json",
        },
        signal: AbortSignal.timeout(4_500),
      },
    );
    if (!resp.ok) return [];
    const data = await resp.json();
    const items = Array.isArray(data?.items) ? data.items : [];
    return items
      .filter((item) => typeof item?.html_url === "string" && item.html_url.startsWith("https://"))
      .map((item) => ({
        title: String(item.full_name || item.name || ""),
        url: String(item.html_url),
        snippet: String(item.description || `GitHub repository (${item.stargazers_count ?? 0} stars)`).slice(0, 360),
        source: "github",
      }));
  })();

  const settled = await Promise.allSettled([
    ddgHtmlTask,
    ddgInstantTask,
    wikipediaTask,
    hnAlgoliaTask,
    githubSearchTask,
  ]);

  const buckets = settled.map((s) => (s.status === "fulfilled" && Array.isArray(s.value) ? s.value : []));
  const deduplicated = [];
  const seenKeys = new Set();

  // Round-robin interleave across active providers for balanced source diversity
  const maxBucketLen = buckets.reduce((acc, b) => Math.max(acc, b.length), 0);
  for (let idx = 0; idx < maxBucketLen && deduplicated.length < limit; idx += 1) {
    for (const bucket of buckets) {
      if (idx >= bucket.length || deduplicated.length >= limit) continue;
      const item = bucket[idx];
      const key = normalizeUrlKey(item.url);
      if (!key || seenKeys.has(key) || !item.title) continue;
      seenKeys.add(key);
      deduplicated.push({
        title: item.title,
        url: item.url,
        snippet: item.snippet ?? "",
        source: item.source,
      });
    }
  }

  return {
    status: 200,
    data: {
      query,
      results: deduplicated,
    },
  };
}

function buildGitUpstreamUrl(parsedRepo, rawPathAndQuery, method, serviceFallback) {
  const host = parsedRepo.hostname.toLowerCase();
  let sub = String(rawPathAndQuery ?? "").trim();
  if (!sub && serviceFallback) {
    const svc = String(serviceFallback).trim();
    sub = method === "POST" ? svc : `info/refs?service=${encodeURIComponent(svc)}`;
  }

  if (sub.includes("..") || sub.includes("\\") || sub.includes("@") || /^https?:/i.test(sub) || sub.startsWith("//")) {
    throw new RelayError("InvalidGitSmartHttpRequest", 400, "Disallowed path characters in git_smart_http request");
  }

  if (host === "github.com") {
    const repoMatch = /^\/([A-Za-z0-9_.-]+)\/([A-Za-z0-9_.-]+?)(?:\.git)?\/?$/.exec(parsedRepo.pathname);
    if (!repoMatch || repoMatch[1] === "." || repoMatch[1] === ".." || repoMatch[2] === "." || repoMatch[2] === "..") {
      throw new RelayError(
        "InvalidGitSmartHttpRequest",
        400,
        "Repository URL on github.com must have shape https://github.com/<owner>/<repo>[.git]",
      );
    }
    const cleanSub = sub.replace(/^\/+/, "");
    const allowedGet =
      cleanSub === "info/refs?service=git-upload-pack" ||
      cleanSub === "info/refs?service=git-receive-pack" ||
      cleanSub === "HEAD";
    const allowedPost = cleanSub === "git-upload-pack" || cleanSub === "git-receive-pack";

    if ((method === "GET" || method === "HEAD") && !allowedGet) {
      throw new RelayError(
        "InvalidGitSmartHttpRequest",
        400,
        `Unsupported Git Smart-HTTP GET target '${cleanSub}'`,
      );
    }
    if (method === "POST" && !allowedPost) {
      throw new RelayError(
        "InvalidGitSmartHttpRequest",
        400,
        `Unsupported Git Smart-HTTP POST target '${cleanSub}'`,
      );
    }
    if (method !== "GET" && method !== "HEAD" && method !== "POST") {
      throw new RelayError("InvalidGitSmartHttpRequest", 400, `Unsupported Git Smart-HTTP method '${method}'`);
    }

    return new URL(`https://github.com/${repoMatch[1]}/${repoMatch[2]}.git/${cleanSub}`);
  }

  // host === "api.github.com"
  if (!ALLOWED_HTTP_METHODS.has(method)) {
    throw new RelayError("InvalidGitSmartHttpRequest", 400, `Unsupported GitHub REST method '${method}'`);
  }
  if (parsedRepo.pathname.includes("..")) {
    throw new RelayError("InvalidGitSmartHttpRequest", 400, "Path traversal rejected on api.github.com");
  }
  if (!sub) {
    return new URL(parsedRepo.toString());
  }
  const basePath = parsedRepo.pathname.replace(/\/+$/, "");
  const cleanSub = sub.replace(/^\/+/, "");
  const combined = new URL(`https://api.github.com${basePath}/${cleanSub}`);
  if (combined.hostname.toLowerCase() !== "api.github.com" || combined.protocol !== "https:") {
    throw new RelayError("InvalidGitSmartHttpRequest", 400, "Constructed GitHub REST URL escaped allowlist");
  }
  return combined;
}

async function handleGitSmartHttp(params, fetchImpl) {
  const rawRepoUrl = String(params?.repo_url ?? params?.url ?? "").trim();
  const rawAuthBearer = String(params?.auth_bearer ?? params?.auth_header ?? "").trim();
  const hasSecret =
    rawAuthBearer.length > 0 ||
    containsSecretPlaceholder(rawRepoUrl) ||
    containsSecretPlaceholder(params?.path_and_query);

  if (!rawRepoUrl) {
    throw new RelayError("InvalidGitSmartHttpRequest", 400, "Missing repo_url in git_smart_http");
  }

  let parsedRepo;
  try {
    parsedRepo = new URL(rawRepoUrl);
  } catch {
    throw new RelayError("InvalidUrl", 400, `Malformed repo_url '${rawRepoUrl}'`);
  }

  const host = parsedRepo.hostname.toLowerCase();
  const isAllowlistedGitHub = ALLOWLISTED_GITHUB_HOSTS.has(host);

  // R4-39 invariant: any secret material directed to a non-GitHub host MUST fail immediately
  // with HostNotAllowlistedForSecret before checking anything else or touching fetch().
  if (!isAllowlistedGitHub) {
    if (hasSecret) {
      throw new RelayError(
        "HostNotAllowlistedForSecret",
        403,
        `Refusing secret transmission to non-allowlisted host '${host}'; only github.com and api.github.com are permitted`,
      );
    }
    throw new RelayError(
      "HostNotAllowlisted",
      403,
      `git_smart_http strictly permits github.com and api.github.com (got '${host}')`,
    );
  }

  if (parsedRepo.protocol !== "https:") {
    throw new RelayError(
      "InsecureTransportRejected",
      400,
      `git_smart_http requires https:// transport (got '${parsedRepo.protocol}')`,
    );
  }
  if (parsedRepo.username || parsedRepo.password) {
    throw new RelayError("InsecureTransportRejected", 400, "Embedded URL credentials are not permitted");
  }
  if (containsSecretPlaceholder(rawAuthBearer)) {
    throw new RelayError(
      "MissingSecretSubstitution",
      400,
      "Raw {{MVM_SECRET:...}} placeholder must be substituted on host before upstream transmission",
    );
  }

  const method = String(params?.method ?? "GET").trim().toUpperCase();
  const upstreamUrl = buildGitUpstreamUrl(parsedRepo, params?.path_and_query, method, params?.service);

  const headers = new Headers();
  headers.set("User-Agent", `git/2.43.0 ${RELAY_USER_AGENT}`);

  if (host === "api.github.com") {
    headers.set("Accept", params?.accept ? String(params.accept) : "application/vnd.github+json");
    headers.set("X-GitHub-Api-Version", "2022-11-28");
    if (params?.content_type) {
      headers.set("Content-Type", String(params.content_type));
    } else if (method !== "GET" && method !== "HEAD") {
      headers.set("Content-Type", "application/json");
    }
    if (rawAuthBearer) {
      const authVal = /^(Bearer|Basic|token)\s+/i.test(rawAuthBearer)
        ? rawAuthBearer
        : `Bearer ${rawAuthBearer}`;
      headers.set("Authorization", authVal);
    }
  } else {
    // github.com Smart-HTTP
    if (params?.content_type) {
      headers.set("Content-Type", String(params.content_type));
    } else if (method === "POST") {
      const rpcName = upstreamUrl.pathname.endsWith("/git-receive-pack")
        ? "git-receive-pack"
        : "git-upload-pack";
      headers.set("Content-Type", `application/x-${rpcName}-request`);
      headers.set("Accept", `application/x-${rpcName}-result`);
    } else {
      headers.set("Accept", "*/*");
    }
    if (rawAuthBearer) {
      if (/^Basic\s+/i.test(rawAuthBearer)) {
        headers.set("Authorization", rawAuthBearer);
      } else {
        const cleanToken = rawAuthBearer.replace(/^Bearer\s+/i, "").trim();
        headers.set("Authorization", `Basic ${utf8ToBase64(`x-access-token:${cleanToken}`)}`);
      }
    }
  }

  let bodyBytes;
  if (method !== "GET" && method !== "HEAD" && params?.body_b64) {
    bodyBytes = base64ToBytes(params.body_b64);
    if (bodyBytes.byteLength > MAX_GIT_BODY_BYTES) {
      throw new RelayError("PayloadTooLarge", 413, "Git packfile payload exceeds 8 MiB limit");
    }
  }

  let resp;
  try {
    resp = await fetchImpl(upstreamUrl.toString(), {
      method,
      headers,
      body: bodyBytes,
      redirect: "manual",
      signal: AbortSignal.timeout(20_000),
    });
  } catch (err) {
    if (err instanceof RelayError) throw err;
    throw new RelayError("UpstreamFetchFailed", 502, `GitHub upstream fetch failed: ${err?.message ?? String(err)}`);
  }

  // Follow safe github.com canonical repository redirects (e.g., renamed repo on github.com)
  if (resp.status >= 300 && resp.status < 400) {
    const loc = resp.headers.get("location");
    if (loc) {
      const nextUrl = new URL(loc, upstreamUrl);
      if (
        nextUrl.protocol === "https:" &&
        ALLOWLISTED_GITHUB_HOSTS.has(nextUrl.hostname.toLowerCase())
      ) {
        resp = await fetchImpl(nextUrl.toString(), {
          method,
          headers,
          body: bodyBytes,
          redirect: "manual",
          signal: AbortSignal.timeout(20_000),
        });
      } else {
        throw new RelayError("HostNotAllowlistedForSecret", 403, "Blocked cross-host redirect from GitHub");
      }
    }
  }

  const { bytes, truncated } = await readBoundedBytes(resp, MAX_GIT_BODY_BYTES);
  const respHeaders = {};
  resp.headers.forEach((v, k) => {
    respHeaders[k.toLowerCase()] = v;
  });

  return {
    status: resp.status,
    data: {
      status: resp.status,
      content_type: resp.headers.get("content-type") ?? "",
      headers: respHeaders,
      body_b64: bytesToBase64(bytes),
      byte_length: bytes.byteLength,
      truncated,
    },
  };
}

export async function handleSpvsEnvelope(envelope, options = {}) {
  const fetchImpl = options.fetchImpl ?? globalThis.fetch;
  if (!envelope || typeof envelope !== "object" || Array.isArray(envelope)) {
    return makeErrorResponse("", 400, "InvalidEnvelope", "SPVS frame must be a JSON object");
  }

  const reqId =
    typeof envelope.req_id === "string" || typeof envelope.req_id === "number"
      ? String(envelope.req_id)
      : "";
  if (!reqId) {
    return makeErrorResponse("", 400, "InvalidEnvelope", "Missing non-empty req_id in SPVS envelope");
  }

  if (envelope.spvs_magic !== "SPVS") {
    return makeErrorResponse(
      reqId,
      400,
      "InvalidSpiralCoreMagic",
      "SPVS frame rejected: spvs_magic must equal 'SPVS'",
    );
  }

  const srcCid = envelope.src_cid;
  const dstCid = envelope.dst_cid;
  if (
    typeof srcCid !== "number" ||
    !Number.isInteger(srcCid) ||
    srcCid < 3 ||
    srcCid > 65_535 ||
    dstCid !== 2
  ) {
    return makeErrorResponse(
      reqId,
      400,
      "InvalidVsockCid",
      `SPVS frame rejected: expected guest CID >= 3 -> host CID 2 (got ${String(srcCid)} -> ${String(dstCid)})`,
    );
  }

  if (typeof envelope.policy === "string" && envelope.policy.trim().toLowerCase() === "deny") {
    return makeErrorResponse(
      reqId,
      403,
      "PolicyDenied",
      "Outbound operation denied by active MCP security policy",
    );
  }

  const op = String(envelope.op ?? "");
  const params = envelope.params && typeof envelope.params === "object" ? envelope.params : {};

  try {
    switch (op) {
      case "dns_query": {
        const res = await handleDnsQuery(params, fetchImpl);
        return makeOkResponse(reqId, res.status, res.data);
      }
      case "http_request": {
        const res = await handleHttpRequest(params, fetchImpl);
        return makeOkResponse(reqId, res.status, res.data);
      }
      case "web_search": {
        const res = await handleWebSearch(params, fetchImpl);
        return makeOkResponse(reqId, res.status, res.data);
      }
      case "git_smart_http": {
        const res = await handleGitSmartHttp(params, fetchImpl);
        return makeOkResponse(reqId, res.status, res.data);
      }
      default:
        return makeErrorResponse(
          reqId,
          400,
          "UnsupportedOperation",
          `Unsupported SpiralCore relay operation '${op}'`,
        );
    }
  } catch (err) {
    if (err instanceof RelayError) {
      return makeErrorResponse(reqId, err.status, err.errorCode, err.message);
    }
    return makeErrorResponse(
      reqId,
      500,
      "InternalRelayError",
      err?.message ? String(err.message) : "Unexpected internal relay failure",
    );
  }
}

export async function handleSpvsRawMessage(rawText, options = {}) {
  let parsed;
  try {
    parsed = JSON.parse(rawText);
  } catch {
    return makeErrorResponse("", 400, "MalformedJson", "SPVS payload is not valid JSON");
  }
  return handleSpvsEnvelope(parsed, options);
}

function jsonHttpResponse(bodyObj, status = 200) {
  return new Response(JSON.stringify(bodyObj), {
    status,
    headers: {
      ...CORS_HEADERS,
      "Content-Type": "application/json; charset=utf-8",
      "Cache-Control": "no-store",
    },
  });
}

export async function handleWorkerFetch(request, _env, _ctx, options = {}) {
  if (request.method === "OPTIONS") {
    return new Response(null, { status: 204, headers: CORS_HEADERS });
  }

  let url;
  try {
    url = new URL(request.url);
  } catch {
    return jsonHttpResponse(makeErrorResponse("", 400, "InvalidUrl", "Invalid request URL"), 400);
  }

  if (request.method === "GET" && (url.pathname === "/health" || url.pathname === "/")) {
    return jsonHttpResponse({ ok: true, service: SERVICE_NAME }, 200);
  }

  if (url.pathname === "/spvs") {
    const upgradeHeader = request.headers.get("Upgrade");
    if (upgradeHeader && upgradeHeader.toLowerCase() === "websocket") {
      if (typeof globalThis.WebSocketPair !== "function") {
        return jsonHttpResponse(
          makeErrorResponse("", 426, "UpgradeUnavailable", "WebSocketPair runtime unavailable"),
          426,
        );
      }
      const pair = new globalThis.WebSocketPair();
      const client = pair[0];
      const server = pair[1];
      server.accept();

      server.addEventListener("message", async (event) => {
        const rawText =
          typeof event.data === "string"
            ? event.data
            : new TextDecoder().decode(
                event.data instanceof ArrayBuffer ? new Uint8Array(event.data) : event.data,
              );
        const reply = await handleSpvsRawMessage(rawText, options);
        try {
          server.send(JSON.stringify(reply));
        } catch {
          // Client closed socket prior to reply delivery
        }
      });

      return new Response(null, { status: 101, webSocket: client, headers: CORS_HEADERS });
    }

    if (request.method === "POST") {
      const rawText = await request.text();
      const reply = await handleSpvsRawMessage(rawText, options);
      const httpStatus = reply.ok ? 200 : reply.status || 400;
      return jsonHttpResponse(reply, httpStatus);
    }

    return jsonHttpResponse(
      makeErrorResponse("", 405, "MethodNotAllowed", "Use WebSocket Upgrade or POST on /spvs"),
      405,
    );
  }

  return jsonHttpResponse(
    makeErrorResponse("", 404, "NotFound", `Unknown route '${url.pathname}'`),
    404,
  );
}

export default {
  fetch: handleWorkerFetch,
  handleSpvsEnvelope,
  handleSpvsRawMessage,
};
