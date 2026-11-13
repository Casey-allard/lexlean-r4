# SpiralCore Stateless `wss://` & HTTPS Edge Relay (`R4-39`)

Zero-dependency stateless `SpiralCore` `AF_VSOCK` (`CID 3 -> CID 2`) edge relay for `lexlean-r4`, implemented as both a Cloudflare Worker (`relay/worker.mjs`) and a local Node.js WebSocket + HTTP relay (`relay/local-relay.mjs`) sharing a single protocol and security core.

## Endpoints

- **Live Cloudflare Worker WebSocket**: `wss://lexlean-r4-spiralcore-relay.swarm-relay-cloud-8d31a122.workers.dev/spvs`
- **Live Cloudflare Worker HTTPS POST**: `https://lexlean-r4-spiralcore-relay.swarm-relay-cloud-8d31a122.workers.dev/spvs`
- **Live Cloudflare Worker Health**: `https://lexlean-r4-spiralcore-relay.swarm-relay-cloud-8d31a122.workers.dev/health`
- `GET /health` -> `200 OK` `{ "ok": true, "service": "lexlean-r4-spiralcore-relay" }` (`Access-Control-Allow-Origin: *`)
- `Upgrade: websocket` on `/spvs` -> Multiplexed JSON-framed `SpiralCore` `CID 3 -> CID 2` transport
- `POST /spvs` + `OPTIONS /spvs` -> CORS-enabled (`Access-Control-Allow-Origin: *`) HTTP transport

## Envelope Contract

### Request

```json
{
  "req_id": "req-01",
  "spvs_magic": "SPVS",
  "src_cid": 3,
  "dst_cid": 2,
  "op": "dns_query | http_request | web_search | git_smart_http",
  "params": {}
}
```

### Response

```json
{
  "req_id": "req-01",
  "ok": true,
  "status": 200,
  "data": {},
  "error_code": null,
  "error_message": null
}
```

## Supported Operations

1. **`dns_query`** (`{ name, type }`):
   Queries `https://cloudflare-dns.com/dns-query` (`Accept: application/dns-json`) per RFC 8484 and returns `{ name, type, rcode, ad, answers: [{ name, type, type_name, TTL, ttl, data }] }`.
2. **`http_request`** (`{ method, url, headers, body_b64, max_bytes }`):
   Enforces `https://` transport only (`InsecureTransportRejected` on `http://` or redirect downgrades), blocks loopback, RFC 1918 private, CGNAT, link-local/cloud-metadata (`169.254.*`), and IPv6 local/mapped SSRF targets (`SsrfBlocked` on initial URL and every redirect hop), caps bodies strictly at `<= 2 MiB` (`2,097,152` bytes), extracts clean readable `text_excerpt` from HTML/text payloads, and returns `{ status, headers, body_b64, byte_length, truncated, content_type, text_excerpt }`.
3. **`web_search`** (`{ query, limit }`):
   Queries DuckDuckGo HTML & Instant Answer, Wikipedia Opensearch, Hacker News Algolia, and GitHub Repositories Search concurrently, returning deduplicated `https://` results `{ query, results: [{ title, url, snippet, source }] }`.
4. **`git_smart_http`** (`{ repo_url, path_and_query, method, content_type, auth_bearer, body_b64 }`):
   Strictly allowlists `https://github.com/` (`info/refs?service=git-upload-pack`, `git-upload-pack`, `info/refs?service=git-receive-pack`, `git-receive-pack`) and `https://api.github.com/` REST endpoints. Any request carrying `auth_bearer` or `{{MVM_SECRET:...}}` toward a host outside `github.com` and `api.github.com` is immediately rejected with `HostNotAllowlistedForSecret` before any network call. Formats PAT credentials as HTTP Basic `x-access-token:<token>` on `github.com` Smart-HTTP and `Bearer <token>` on `api.github.com`.

## Typed Error Codes (`R5`)

| `error_code` | HTTP Status | Condition |
| --- | --- | --- |
| `MalformedJson` | `400` | Request payload is not valid JSON |
| `InvalidEnvelope` | `400` | Missing `req_id` or non-object envelope |
| `InvalidSpiralCoreMagic` | `400` | `spvs_magic !== "SPVS"` |
| `InvalidVsockCid` | `400` | `src_cid < 3`, `src_cid > 65535`, or `dst_cid !== 2` |
| `PolicyDenied` | `403` | Envelope carries `policy: "deny"` |
| `UnsupportedOperation` | `400` | Unrecognized `op` value |
| `InvalidDnsName` / `InvalidDnsType` | `400` | Malformed domain or unsupported DNS RR type |
| `InvalidUrl` / `InvalidHttpMethod` / `InvalidBase64Payload` | `400` | Malformed URL, disallowed HTTP verb, or invalid Base64 |
| `InsecureTransportRejected` | `400` | Non-`https://` target URL, embedded URL credentials, or plaintext redirect |
| `SsrfBlocked` | `403` | Loopback, private, link-local, metadata, or mapped IPv6 SSRF host/redirect |
| `HostNotAllowlistedForSecret` | `403` | `auth_bearer` or `{{MVM_SECRET:...}}` directed to a host outside `github.com` / `api.github.com` |
| `HostNotAllowlisted` | `403` | `git_smart_http` directed to a host outside `github.com` / `api.github.com` |
| `InvalidGitSmartHttpRequest` | `400` | Path traversal or disallowed endpoint on `github.com` / `api.github.com` |
| `MissingSecretSubstitution` | `400` | Unresolved `{{MVM_SECRET:...}}` placeholder reached upstream boundary |
| `PayloadTooLarge` | `413` | Request body exceeds bounded relay quota |
| `TooManyRedirects` / `UpstreamFetchFailed` | `502` | Redirect loop (`> 5` hops) or upstream network failure |

## Local Development & Verification

```sh
# Run the full E2E & security boundary suite (9 tests across Worker + Local WebSocket/HTTP relay)
node --test relay/relay.test.mjs

# Start local WebSocket + HTTP relay on ws://127.0.0.1:8787/spvs
node relay/local-relay.mjs

# Deploy live to Cloudflare Workers
npx wrangler deploy --config relay/wrangler.toml
```

### Planted-Defect Gate Evidence

- **Planted Defect**: Added `"gitlab.com"` to `ALLOWLISTED_GITHUB_HOSTS` in `relay/worker.mjs`.
- **Failing Command**: `node --test relay/relay.test.mjs`
- **Observed Diagnostic**: `AssertionError [ERR_ASSERTION]: Expected rejection for https://gitlab.com/Casey-allard/lexlean-r4.git (true !== false)` in `enforces git_smart_http GitHub allowlist, rejects non-GitHub auth_bearer as HostNotAllowlistedForSecret`.
- **Restored Result**: Restored strict `Set(["github.com", "api.github.com"])`; `node --test relay/relay.test.mjs` passed `9/9` (`fail 0`).
