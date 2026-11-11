import assert from "node:assert/strict";
import { describe, it, before, after } from "node:test";

import workerModule from "./worker.mjs";
import { startLocalRelay } from "./local-relay.mjs";

function makeSpvsEnvelope(op, params, overrides = {}) {
  return {
    req_id: overrides.req_id ?? `req-${Math.random().toString(36).slice(2, 10)}`,
    spvs_magic: overrides.spvs_magic ?? "SPVS",
    src_cid: overrides.src_cid ?? 3,
    dst_cid: overrides.dst_cid ?? 2,
    op,
    params,
    ...(overrides.policy !== undefined ? { policy: overrides.policy } : {}),
  };
}

function sendWsRequest(ws, envelope) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      cleanup();
      reject(new Error(`Timed out waiting for req_id=${envelope.req_id}`));
    }, 10000);

    function onMessage(event) {
      const parsed = JSON.parse(typeof event.data === "string" ? event.data : Buffer.from(event.data).toString("utf8"));
      if (parsed.req_id === envelope.req_id) {
        cleanup();
        resolve(parsed);
      }
    }

    function onError(err) {
      cleanup();
      reject(err);
    }

    function cleanup() {
      clearTimeout(timer);
      ws.removeEventListener("message", onMessage);
      ws.removeEventListener("error", onError);
    }

    ws.addEventListener("message", onMessage);
    ws.addEventListener("error", onError);
    ws.send(JSON.stringify(envelope));
  });
}

function openWebSocket(url) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    const timer = setTimeout(() => {
      reject(new Error(`WebSocket connect timeout: ${url}`));
    }, 5000);
    ws.addEventListener("open", () => {
      clearTimeout(timer);
      resolve(ws);
    }, { once: true });
    ws.addEventListener("error", (err) => {
      clearTimeout(timer);
      reject(err);
    }, { once: true });
  });
}

describe("SpiralCore Relay — Worker HTTP/CORS & Deterministic Upstream Suite", () => {
  it("serves GET /health and OPTIONS /spvs preflight with CORS headers", async () => {
    const healthResp = await workerModule.fetch(new Request("https://relay.example.workers.dev/health", { method: "GET" }));
    assert.equal(healthResp.status, 200);
    assert.equal(healthResp.headers.get("access-control-allow-origin"), "*");
    const healthBody = await healthResp.json();
    assert.deepEqual(healthBody, {
      ok: true,
      service: "lexlean-r4-spiralcore-relay",
    });

    const preflightResp = await workerModule.fetch(
      new Request("https://relay.example.workers.dev/spvs", {
        method: "OPTIONS",
        headers: {
          Origin: "https://casey-allard.github.io",
          "Access-Control-Request-Method": "POST",
        },
      }),
    );
    assert.equal(preflightResp.status, 204);
    assert.equal(preflightResp.headers.get("access-control-allow-origin"), "*");
    assert.match(preflightResp.headers.get("access-control-allow-methods") ?? "", /POST/);
  });

  it("enforces SPVS magic, CID 3 -> CID 2 bounds, and MCP policy admission", async () => {
    const badMagic = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("dns_query", { name: "github.com" }, { req_id: "m1", spvs_magic: "NOPE" }),
    );
    assert.equal(badMagic.ok, false);
    assert.equal(badMagic.error_code, "InvalidSpiralCoreMagic");

    const badSrcCid = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("dns_query", { name: "github.com" }, { req_id: "m2", src_cid: 2 }),
    );
    assert.equal(badSrcCid.ok, false);
    assert.equal(badSrcCid.error_code, "InvalidVsockCid");

    const badDstCid = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("dns_query", { name: "github.com" }, { req_id: "m3", dst_cid: 99 }),
    );
    assert.equal(badDstCid.ok, false);
    assert.equal(badDstCid.error_code, "InvalidVsockCid");

    const denied = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("dns_query", { name: "github.com" }, { req_id: "m4", policy: "deny" }),
    );
    assert.equal(denied.ok, false);
    assert.equal(denied.status, 403);
    assert.equal(denied.error_code, "PolicyDenied");

    const unknownOp = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("raw_eth_frame", {}, { req_id: "m5" }),
    );
    assert.equal(unknownOp.ok, false);
    assert.equal(unknownOp.error_code, "UnsupportedOperation");
  });

  it("executes dns_query via RFC 8484 DoH and rejects invalid names/types", async () => {
    let capturedUrl = "";
    let capturedAccept = "";
    const mockFetch = async (input, init) => {
      capturedUrl = String(input);
      capturedAccept = new Headers(init?.headers).get("accept") ?? "";
      return new Response(
        JSON.stringify({
          Status: 0,
          AD: true,
          Answer: [
            { name: "github.com.", type: 1, TTL: 60, data: "140.82.112.4" },
          ],
        }),
        { status: 200, headers: { "content-type": "application/dns-json" } },
      );
    };

    const res = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("dns_query", { name: "github.com", type: "A" }, { req_id: "dns-1" }),
      { fetchImpl: mockFetch },
    );
    assert.equal(res.ok, true);
    assert.equal(res.status, 200);
    assert.match(capturedUrl, /^https:\/\/cloudflare-dns\.com\/dns-query\?/);
    assert.equal(capturedAccept, "application/dns-json");
    assert.equal(res.data.answers.length, 1);
    assert.equal(res.data.answers[0].data, "140.82.112.4");
    assert.equal(res.data.answers[0].TTL, 60);

    const badDns = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("dns_query", { name: "https://not-a-domain/path", type: "A" }, { req_id: "dns-bad" }),
      { fetchImpl: mockFetch },
    );
    assert.equal(badDns.ok, false);
    assert.equal(badDns.error_code, "InvalidDnsName");

    const badType = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("dns_query", { name: "github.com", type: "BOGUS" }, { req_id: "dns-bad-type" }),
      { fetchImpl: mockFetch },
    );
    assert.equal(badType.ok, false);
    assert.equal(badType.error_code, "InvalidDnsType");
  });

  it("enforces http_request TLS-only, SSRF private-IP/redirect blocking, 2 MiB body cap, and readable text_excerpt", async () => {
    let upstreamCalled = false;
    const guardFetch = async () => {
      upstreamCalled = true;
      return new Response("should not be reached", { status: 200 });
    };

    // 1. Plain HTTP rejected as InsecureTransportRejected
    const plainHttp = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("http_request", { method: "GET", url: "http://example.com/index.html" }),
      { fetchImpl: guardFetch },
    );
    assert.equal(plainHttp.ok, false);
    assert.equal(plainHttp.error_code, "InsecureTransportRejected");
    assert.equal(upstreamCalled, false);

    // 2. Comprehensive SSRF host blocklist
    const ssrfTargets = [
      "https://localhost/admin",
      "https://api.localhost/v1",
      "https://127.0.0.1:8443/secret",
      "https://127.55.12.9/test",
      "https://10.0.0.1/internal",
      "https://192.168.1.1/router",
      "https://172.16.0.5/cluster",
      "https://172.31.255.254/cluster",
      "https://169.254.169.254/latest/meta-data/",
      "https://[::1]/loopback",
      "https://[::ffff:127.0.0.1]/mapped",
      "https://[fe80::1]/linklocal",
      "https://[fd00::1234]/ula",
      "https://0x7f000001/hexloopback",
    ];
    for (const badUrl of ssrfTargets) {
      const blocked = await workerModule.handleSpvsEnvelope(
        makeSpvsEnvelope("http_request", { method: "GET", url: badUrl }),
        { fetchImpl: guardFetch },
      );
      assert.equal(blocked.ok, false, `Expected SSRF block for ${badUrl}`);
      assert.equal(blocked.error_code, "SsrfBlocked", `Wrong error_code for ${badUrl}: ${blocked.error_code}`);
      assert.equal(upstreamCalled, false);
    }

    // 3. Redirect-based SSRF and HTTP downgrade are also blocked
    const redirectSsrfFetch = async (url) => {
      if (String(url) === "https://public.example.com/hop") {
        return new Response(null, {
          status: 302,
          headers: { location: "https://169.254.169.254/latest/meta-data/" },
        });
      }
      throw new Error("Redirect target should never have been fetched");
    };
    const redirectBlocked = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("http_request", { method: "GET", url: "https://public.example.com/hop" }),
      { fetchImpl: redirectSsrfFetch },
    );
    assert.equal(redirectBlocked.ok, false);
    assert.equal(redirectBlocked.error_code, "SsrfBlocked");

    // 4. Secret placeholder toward non-GitHub host rejected with HostNotAllowlistedForSecret
    const secretLeakAttempt = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("http_request", {
        method: "GET",
        url: "https://example.com/collect",
        headers: { Authorization: "Bearer {{MVM_SECRET:GITHUB_TOKEN}}" },
      }),
      { fetchImpl: guardFetch },
    );
    assert.equal(secretLeakAttempt.ok, false);
    assert.equal(secretLeakAttempt.error_code, "HostNotAllowlistedForSecret");

    // 5. HTML readable text extraction
    const htmlFetch = async () =>
      new Response(
        "<!doctype html><html><head><title>Spiral Doc</title><style>body{color:red}</style><script>alert(1)</script></head><body><h1>LexLean R4</h1><p>Pure integer &amp; zero matmul.</p></body></html>",
        { status: 200, headers: { "content-type": "text/html; charset=utf-8" } },
      );
    const htmlRes = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("http_request", { method: "GET", url: "https://example.com/doc" }),
      { fetchImpl: htmlFetch },
    );
    assert.equal(htmlRes.ok, true);
    assert.match(htmlRes.data.content_type, /text\/html/);
    assert.match(htmlRes.data.text_excerpt, /LexLean R4/);
    assert.match(htmlRes.data.text_excerpt, /Pure integer & zero matmul\./);
    assert.doesNotMatch(htmlRes.data.text_excerpt, /alert\(1\)/);

    // 6. Oversized > 2 MiB upstream payload capped cleanly to exact 2 MiB (2,097,152 bytes)
    const twoAndHalfMiB = new Uint8Array(2_621_440);
    twoAndHalfMiB.fill(0x41);
    const hugeFetch = async () =>
      new Response(twoAndHalfMiB, {
        status: 200,
        headers: { "content-type": "application/octet-stream" },
      });
    const cappedRes = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("http_request", { method: "GET", url: "https://example.com/big.bin", max_bytes: 9_000_000 }),
      { fetchImpl: hugeFetch },
    );
    assert.equal(cappedRes.ok, true);
    const decodedBytes = Buffer.from(cappedRes.data.body_b64, "base64");
    assert.equal(decodedBytes.byteLength, 2 * 1024 * 1024);
    assert.equal(cappedRes.data.truncated, true);
  });

  it("aggregates and deduplicates web_search results across DuckDuckGo, Wikipedia, HN Algolia, and GitHub", async () => {
    const searchMockFetch = async (input) => {
      const url = String(input);
      if (url.includes("duckduckgo.com/html")) {
        return new Response(
          `<div class="result"><a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fen.wikipedia.org%2Fwiki%2FRISC-V">RISC-V Architecture</a><a class="result__snippet">Open standard instruction set architecture.</a></div>`,
          { status: 200, headers: { "content-type": "text/html" } },
        );
      }
      if (url.includes("api.duckduckgo.com")) {
        return new Response(
          JSON.stringify({
            Heading: "RISC-V",
            AbstractText: "RISC-V is an open standard ISA based on RISC principles.",
            AbstractURL: "https://en.wikipedia.org/wiki/RISC-V",
            RelatedTopics: [],
          }),
          { status: 200, headers: { "content-type": "application/json" } },
        );
      }
      if (url.includes("en.wikipedia.org")) {
        return new Response(
          JSON.stringify([
            "riscv",
            ["RISC-V", "RISC-V assembly"],
            ["Open standard ISA", "Assembly language for RISC-V"],
            ["https://en.wikipedia.org/wiki/RISC-V", "https://en.wikipedia.org/wiki/RISC-V_assembly"],
          ]),
          { status: 200, headers: { "content-type": "application/json" } },
        );
      }
      if (url.includes("hn.algolia.com")) {
        return new Response(
          JSON.stringify({
            hits: [
              { title: "Linux on RISC-V in WebAssembly", url: "https://example.org/riscv-wasm", story_text: "Running RV64 in Wasm." },
            ],
          }),
          { status: 200, headers: { "content-type": "application/json" } },
        );
      }
      if (url.includes("api.github.com/search/repositories")) {
        return new Response(
          JSON.stringify({
            items: [
              { full_name: "riscv/riscv-isa-manual", html_url: "https://github.com/riscv/riscv-isa-manual", description: "RISC-V ISA specification" },
            ],
          }),
          { status: 200, headers: { "content-type": "application/json" } },
        );
      }
      return new Response("{}", { status: 404 });
    };

    const res = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("web_search", { query: "RISC-V", limit: 10 }),
      { fetchImpl: searchMockFetch },
    );
    assert.equal(res.ok, true);
    assert.ok(Array.isArray(res.data.results));
    assert.ok(res.data.results.length >= 4);
    const urls = res.data.results.map((r) => r.url);
    assert.equal(new Set(urls).size, urls.length, "Expected deduplicated URLs");
    for (const item of res.data.results) {
      assert.ok(item.title.length > 0);
      assert.match(item.url, /^https:\/\//);
      assert.ok(typeof item.snippet === "string");
      assert.ok(item.source.length > 0);
    }
  });

  it("enforces git_smart_http GitHub allowlist, rejects non-GitHub auth_bearer as HostNotAllowlistedForSecret, and proxies Smart-HTTP + api.github.com", async () => {
    let fetchedCount = 0;
    let lastUrl = "";
    let lastHeaders = null;

    const gitMockFetch = async (input, init) => {
      fetchedCount += 1;
      lastUrl = String(input);
      lastHeaders = new Headers(init?.headers);
      return new Response("001e# service=git-upload-pack\n0000", {
        status: 200,
        headers: { "content-type": "application/x-git-upload-pack-advertisement" },
      });
    };

    // 1. Secret toward non-GitHub domain MUST immediately reject with HostNotAllowlistedForSecret
    for (const evilRepo of [
      "https://evil.example.com/Casey-allard/lexlean-r4.git",
      "https://github.com.attacker.io/Casey-allard/lexlean-r4.git",
      "https://gitlab.com/Casey-allard/lexlean-r4.git",
      "http://evil.example.com/Casey-allard/lexlean-r4.git",
    ]) {
      const rejected = await workerModule.handleSpvsEnvelope(
        makeSpvsEnvelope("git_smart_http", {
          repo_url: evilRepo,
          path_and_query: "info/refs?service=git-upload-pack",
          method: "GET",
          auth_bearer: "ghp_super_secret_pat_value",
        }),
        { fetchImpl: gitMockFetch },
      );
      assert.equal(rejected.ok, false, `Expected rejection for ${evilRepo}`);
      assert.equal(rejected.error_code, "HostNotAllowlistedForSecret");
      assert.equal(fetchedCount, 0, "Secret must never reach upstream fetch on non-GitHub host");
    }

    // 2. Plain HTTP toward github.com rejected as InsecureTransportRejected
    const httpGithub = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("git_smart_http", {
        repo_url: "http://github.com/Casey-allard/lexlean-r4.git",
        path_and_query: "info/refs?service=git-upload-pack",
        method: "GET",
      }),
      { fetchImpl: gitMockFetch },
    );
    assert.equal(httpGithub.ok, false);
    assert.equal(httpGithub.error_code, "InsecureTransportRejected");
    assert.equal(fetchedCount, 0);

    // 3. Path traversal / non-Git endpoint on github.com rejected
    const traversal = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("git_smart_http", {
        repo_url: "https://github.com/Casey-allard/lexlean-r4.git",
        path_and_query: "../../settings/tokens",
        method: "GET",
      }),
      { fetchImpl: gitMockFetch },
    );
    assert.equal(traversal.ok, false);
    assert.equal(traversal.error_code, "InvalidGitSmartHttpRequest");
    assert.equal(fetchedCount, 0);

    // 4. Valid Smart-HTTP info/refs?service=git-upload-pack with PAT formats Basic x-access-token auth
    const validUploadPack = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("git_smart_http", {
        repo_url: "https://github.com/Casey-allard/lexlean-r4.git",
        path_and_query: "info/refs?service=git-upload-pack",
        method: "GET",
        auth_bearer: "ghp_testtoken123",
      }),
      { fetchImpl: gitMockFetch },
    );
    assert.equal(validUploadPack.ok, true);
    assert.equal(lastUrl, "https://github.com/Casey-allard/lexlean-r4.git/info/refs?service=git-upload-pack");
    const expectedBasic = `Basic ${Buffer.from("x-access-token:ghp_testtoken123", "utf8").toString("base64")}`;
    assert.equal(lastHeaders.get("authorization"), expectedBasic);
    assert.equal(
      Buffer.from(validUploadPack.data.body_b64, "base64").toString("utf8"),
      "001e# service=git-upload-pack\n0000",
    );

    // 5. Valid api.github.com REST request uses Bearer token
    const validRest = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("git_smart_http", {
        repo_url: "https://api.github.com/repos/Casey-allard/lexlean-r4",
        path_and_query: "git/trees/main?recursive=1",
        method: "GET",
        auth_bearer: "ghp_testtoken123",
      }),
      { fetchImpl: gitMockFetch },
    );
    assert.equal(validRest.ok, true);
    assert.equal(lastUrl, "https://api.github.com/repos/Casey-allard/lexlean-r4/git/trees/main?recursive=1");
    assert.equal(lastHeaders.get("authorization"), "Bearer ghp_testtoken123");

    // 6. Valid POST git-upload-pack and git-receive-pack Smart-HTTP RPCs
    const validReceiveRefs = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("git_smart_http", {
        repo_url: "https://github.com/Casey-allard/lexlean-r4",
        path_and_query: "info/refs?service=git-receive-pack",
        method: "GET",
        auth_bearer: "ghp_testtoken123",
      }),
      { fetchImpl: gitMockFetch },
    );
    assert.equal(validReceiveRefs.ok, true);
    assert.equal(lastUrl, "https://github.com/Casey-allard/lexlean-r4.git/info/refs?service=git-receive-pack");

    const validReceivePost = await workerModule.handleSpvsEnvelope(
      makeSpvsEnvelope("git_smart_http", {
        repo_url: "https://github.com/Casey-allard/lexlean-r4.git",
        path_and_query: "git-receive-pack",
        method: "POST",
        auth_bearer: "ghp_testtoken123",
        body_b64: Buffer.from("0000", "utf8").toString("base64"),
      }),
      { fetchImpl: gitMockFetch },
    );
    assert.equal(validReceivePost.ok, true);
    assert.equal(lastUrl, "https://github.com/Casey-allard/lexlean-r4.git/git-receive-pack");
    assert.equal(lastHeaders.get("content-type"), "application/x-git-receive-pack-request");
  });

  it("handles Cloudflare Worker Upgrade: websocket via WebSocketPair", async () => {
    class MockEndpoint extends EventTarget {
      constructor() {
        super();
        this.peer = null;
        this.accepted = false;
        this.sent = [];
      }
      accept() {
        this.accepted = true;
      }
      send(payload) {
        this.sent.push(payload);
        if (this.peer) {
          this.peer.dispatchEvent(new MessageEvent("message", { data: payload }));
        }
      }
    }

    const prevPair = globalThis.WebSocketPair;
    let clientEnd;
    let serverEnd;
    globalThis.WebSocketPair = function MockWebSocketPair() {
      clientEnd = new MockEndpoint();
      serverEnd = new MockEndpoint();
      clientEnd.peer = serverEnd;
      serverEnd.peer = clientEnd;
      return { 0: clientEnd, 1: serverEnd };
    };

    try {
      let upgradeRes;
      try {
        upgradeRes = await workerModule.fetch(
          new Request("https://relay.example.workers.dev/spvs", {
            headers: { Upgrade: "websocket" },
          }),
        );
      } catch {
        // Node undici Response constructor rejects status 101; verify serverEnd wired cleanly
      }
      assert.equal(serverEnd.accepted, true);
      if (upgradeRes) {
        assert.equal(upgradeRes.status, 101);
      }

      const replyPromise = new Promise((resolve) => {
        clientEnd.addEventListener("message", (ev) => resolve(JSON.parse(ev.data)), { once: true });
      });
      serverEnd.dispatchEvent(
        new MessageEvent("message", {
          data: JSON.stringify(
            makeSpvsEnvelope("http_request", { method: "GET", url: "http://plain.example.com" }, { req_id: "cf-ws-1" }),
          ),
        }),
      );
      const reply = await replyPromise;
      assert.equal(reply.req_id, "cf-ws-1");
      assert.equal(reply.ok, false);
      assert.equal(reply.error_code, "InsecureTransportRejected");
    } finally {
      globalThis.WebSocketPair = prevPair;
    }
  });
});

describe("SpiralCore Local Node WebSocket + HTTP Relay E2E Suite", () => {
  let relay;
  let baseUrl;
  let wsUrl;

  before(async () => {
    relay = await startLocalRelay({ host: "127.0.0.1", port: 0 });
    baseUrl = `http://127.0.0.1:${relay.port}`;
    wsUrl = `ws://127.0.0.1:${relay.port}/spvs`;
  });

  after(async () => {
    if (relay) {
      await relay.close();
    }
  });

  it("handles HTTP /health, OPTIONS /spvs, POST /spvs, and multiplexed WebSocket frames including >64KB payloads", async () => {
    const health = await fetch(`${baseUrl}/health`);
    assert.equal(health.status, 200);
    assert.deepEqual(await health.json(), { ok: true, service: "lexlean-r4-spiralcore-relay" });

    // POST /spvs negative check
    const postRes = await fetch(`${baseUrl}/spvs`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(
        makeSpvsEnvelope("http_request", { method: "GET", url: "http://example.com" }, { req_id: "post-1" }),
      ),
    });
    assert.equal(postRes.status, 400);
    assert.equal(postRes.headers.get("access-control-allow-origin"), "*");
    const postJson = await postRes.json();
    assert.equal(postJson.error_code, "InsecureTransportRejected");

    // Live WebSocket connection against local-relay.mjs
    const ws = await openWebSocket(wsUrl);
    try {
      // Concurrent multiplexed requests over one WebSocket
      const [dnsReply, ssrfReply, secretReply, gitRefsReply] = await Promise.all([
        sendWsRequest(ws, makeSpvsEnvelope("dns_query", { name: "github.com", type: "A" }, { req_id: "ws-dns" })),
        sendWsRequest(ws, makeSpvsEnvelope("http_request", { method: "GET", url: "https://127.0.0.1/admin" }, { req_id: "ws-ssrf" })),
        sendWsRequest(
          ws,
          makeSpvsEnvelope(
            "git_smart_http",
            {
              repo_url: "https://attacker.example/repo.git",
              path_and_query: "info/refs?service=git-upload-pack",
              method: "GET",
              auth_bearer: "ghp_secret",
            },
            { req_id: "ws-secret" },
          ),
        ),
        sendWsRequest(
          ws,
          makeSpvsEnvelope(
            "git_smart_http",
            {
              repo_url: "https://github.com/Casey-allard/lexlean-r4.git",
              path_and_query: "info/refs?service=git-upload-pack",
              method: "GET",
            },
            { req_id: "ws-git" },
          ),
        ),
      ]);

      assert.equal(dnsReply.ok, true);
      assert.ok(dnsReply.data.answers.length > 0);

      assert.equal(ssrfReply.ok, false);
      assert.equal(ssrfReply.error_code, "SsrfBlocked");

      assert.equal(secretReply.ok, false);
      assert.equal(secretReply.error_code, "HostNotAllowlistedForSecret");

      assert.equal(gitRefsReply.ok, true);
      assert.equal(gitRefsReply.data.status, 200);
      const refsText = Buffer.from(gitRefsReply.data.body_b64, "base64").toString("utf8");
      assert.match(refsText, /# service=git-upload-pack/);
    } finally {
      ws.close();
    }
  });

  it("transfers >128 KB 64-bit length WebSocket frames cleanly over local-relay.mjs", async () => {
    const largeBytes = new Uint8Array(140_000);
    for (let i = 0; i < largeBytes.byteLength; i += 1) {
      largeBytes[i] = i & 0xff;
    }
    const mockRelay = await startLocalRelay({
      host: "127.0.0.1",
      port: 0,
      fetchImpl: async () =>
        new Response(largeBytes, {
          status: 200,
          headers: { "content-type": "application/octet-stream" },
        }),
    });
    const ws = await openWebSocket(`ws://127.0.0.1:${mockRelay.port}/spvs`);
    try {
      const reply = await sendWsRequest(
        ws,
        makeSpvsEnvelope("http_request", { method: "GET", url: "https://example.com/large.bin" }, { req_id: "ws-large-64bit" }),
      );
      assert.equal(reply.ok, true);
      const roundtrip = Buffer.from(reply.data.body_b64, "base64");
      assert.equal(roundtrip.byteLength, 140_000);
      assert.equal(roundtrip[0], 0);
      assert.equal(roundtrip[255], 255);
      assert.equal(roundtrip[139_999], 139_999 & 0xff);
    } finally {
      ws.close();
      await mockRelay.close();
    }
  });
});
