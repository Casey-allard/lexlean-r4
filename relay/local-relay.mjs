/**
 * Local Node.js WebSocket + HTTP SpiralCore Relay (`127.0.0.1:8787`) for offline/CI dev.
 *
 * Zero external npm dependencies: implements RFC 6455 WebSocket upgrade & framing
 * directly over `node:http` + `node:crypto` and delegates all `SPVS` protocol
 * validation and dispatch to `./worker.mjs`.
 */

import { createHash } from "node:crypto";
import http from "node:http";
import { fileURLToPath } from "node:url";

import {
  CORS_HEADERS,
  SERVICE_NAME,
  handleSpvsRawMessage,
} from "./worker.mjs";

const WS_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const MAX_WS_FRAME_BYTES = 16 * 1024 * 1024; // 16 MiB cap

function encodeWsFrame(opcode, payloadBuffer) {
  const len = payloadBuffer.byteLength;
  let header;
  if (len <= 125) {
    header = Buffer.allocUnsafe(2);
    header[0] = 0x80 | (opcode & 0x0f);
    header[1] = len;
  } else if (len <= 65_535) {
    header = Buffer.allocUnsafe(4);
    header[0] = 0x80 | (opcode & 0x0f);
    header[1] = 126;
    header.writeUInt16BE(len, 2);
  } else {
    header = Buffer.allocUnsafe(10);
    header[0] = 0x80 | (opcode & 0x0f);
    header[1] = 127;
    header.writeBigUInt64BE(BigInt(len), 2);
  }
  return Buffer.concat([header, payloadBuffer]);
}

function encodeWsText(text) {
  return encodeWsFrame(0x01, Buffer.from(text, "utf8"));
}

function attachWebSocketSession(socket, initialHead, options) {
  let recvBuffer = initialHead && initialHead.byteLength > 0 ? Buffer.from(initialHead) : Buffer.alloc(0);
  let fragmentChunks = [];
  let fragmentOpcode = 0;
  let fragmentBytes = 0;
  let closed = false;

  function safeWrite(frame) {
    if (!closed && !socket.destroyed && socket.writable) {
      socket.write(frame);
    }
  }

  function closeWithCode(code) {
    if (closed) return;
    closed = true;
    const payload = Buffer.allocUnsafe(2);
    payload.writeUInt16BE(code, 0);
    try {
      socket.end(encodeWsFrame(0x08, payload));
    } catch {
      socket.destroy();
    }
  }

  function dispatchMessage(utf8Message) {
    handleSpvsRawMessage(utf8Message, options)
      .then((reply) => {
        safeWrite(encodeWsText(JSON.stringify(reply)));
      })
      .catch(() => {
        safeWrite(
          encodeWsText(
            JSON.stringify({
              req_id: "",
              ok: false,
              status: 500,
              data: null,
              error_code: "InternalRelayError",
              error_message: "Unhandled local relay exception",
            }),
          ),
        );
      });
  }

  function drainFrames() {
    while (recvBuffer.byteLength >= 2 && !closed) {
      const b0 = recvBuffer[0];
      const b1 = recvBuffer[1];
      const fin = (b0 & 0x80) !== 0;
      const opcode = b0 & 0x0f;
      const masked = (b1 & 0x80) !== 0;
      let payloadLen = b1 & 0x7f;
      let offset = 2;

      if (payloadLen === 126) {
        if (recvBuffer.byteLength < 4) return;
        payloadLen = recvBuffer.readUInt16BE(2);
        offset = 4;
      } else if (payloadLen === 127) {
        if (recvBuffer.byteLength < 10) return;
        const bigLen = recvBuffer.readBigUInt64BE(2);
        if (bigLen > BigInt(MAX_WS_FRAME_BYTES)) {
          closeWithCode(1009);
          return;
        }
        payloadLen = Number(bigLen);
        offset = 10;
      }

      if (payloadLen > MAX_WS_FRAME_BYTES) {
        closeWithCode(1009);
        return;
      }

      const maskBytes = masked ? 4 : 0;
      const frameTotal = offset + maskBytes + payloadLen;
      if (recvBuffer.byteLength < frameTotal) {
        return;
      }

      const payload = Buffer.from(recvBuffer.subarray(offset + maskBytes, frameTotal));
      if (masked) {
        const maskKey = recvBuffer.subarray(offset, offset + 4);
        for (let i = 0; i < payload.byteLength; i += 1) {
          payload[i] ^= maskKey[i & 3];
        }
      }
      recvBuffer = recvBuffer.subarray(frameTotal);

      // Control frames (Close = 0x8, Ping = 0x9, Pong = 0xA)
      if (opcode === 0x08) {
        closed = true;
        try {
          socket.end(encodeWsFrame(0x08, payload.subarray(0, Math.min(payload.byteLength, 2))));
        } catch {
          socket.destroy();
        }
        return;
      }
      if (opcode === 0x09) {
        safeWrite(encodeWsFrame(0x0a, payload));
        continue;
      }
      if (opcode === 0x0a) {
        continue;
      }

      if (opcode === 0x01 || opcode === 0x02) {
        if (fin && fragmentOpcode === 0) {
          dispatchMessage(payload.toString("utf8"));
        } else {
          fragmentOpcode = opcode;
          fragmentChunks = [payload];
          fragmentBytes = payload.byteLength;
        }
      } else if (opcode === 0x00 && fragmentOpcode !== 0) {
        fragmentBytes += payload.byteLength;
        if (fragmentBytes > MAX_WS_FRAME_BYTES) {
          closeWithCode(1009);
          return;
        }
        fragmentChunks.push(payload);
        if (fin) {
          const full = Buffer.concat(fragmentChunks, fragmentBytes);
          fragmentChunks = [];
          fragmentOpcode = 0;
          fragmentBytes = 0;
          dispatchMessage(full.toString("utf8"));
        }
      } else {
        closeWithCode(1002);
        return;
      }
    }
  }

  socket.on("data", (chunk) => {
    recvBuffer = recvBuffer.byteLength === 0 ? chunk : Buffer.concat([recvBuffer, chunk]);
    drainFrames();
  });

  socket.on("error", () => {
    closed = true;
    socket.destroy();
  });

  socket.on("close", () => {
    closed = true;
  });

  if (recvBuffer.byteLength > 0) {
    drainFrames();
  }
}

function writeJson(res, statusCode, obj) {
  const body = Buffer.from(JSON.stringify(obj), "utf8");
  res.writeHead(statusCode, {
    ...CORS_HEADERS,
    "Content-Type": "application/json; charset=utf-8",
    "Content-Length": String(body.byteLength),
    "Cache-Control": "no-store",
  });
  res.end(body);
}

export function startLocalRelay(options = {}) {
  const host = options.host ?? process.env.HOST ?? "127.0.0.1";
  const requestedPort = options.port ?? Number(process.env.PORT ?? 8787);
  const workerOptions = options.fetchImpl ? { fetchImpl: options.fetchImpl } : {};
  const openSockets = new Set();

  const server = http.createServer((req, res) => {
    if (req.method === "OPTIONS") {
      res.writeHead(204, CORS_HEADERS);
      res.end();
      return;
    }

    const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "127.0.0.1"}`);

    if (req.method === "GET" && (url.pathname === "/health" || url.pathname === "/")) {
      writeJson(res, 200, { ok: true, service: SERVICE_NAME });
      return;
    }

    if (url.pathname === "/spvs" && req.method === "POST") {
      const chunks = [];
      let total = 0;
      let aborted = false;

      req.on("data", (chunk) => {
        if (aborted) return;
        total += chunk.byteLength;
        if (total > MAX_WS_FRAME_BYTES) {
          aborted = true;
          writeJson(res, 413, {
            req_id: "",
            ok: false,
            status: 413,
            data: null,
            error_code: "PayloadTooLarge",
            error_message: "Request body exceeds 16 MiB limit",
          });
          req.destroy();
          return;
        }
        chunks.push(chunk);
      });

      req.on("end", async () => {
        if (aborted) return;
        const rawText = Buffer.concat(chunks, total).toString("utf8");
        const reply = await handleSpvsRawMessage(rawText, workerOptions);
        const httpStatus = reply.ok ? 200 : reply.status || 400;
        writeJson(res, httpStatus, reply);
      });
      return;
    }

    writeJson(res, 404, {
      req_id: "",
      ok: false,
      status: 404,
      data: null,
      error_code: "NotFound",
      error_message: `Unknown route '${url.pathname}'`,
    });
  });

  server.on("connection", (socket) => {
    openSockets.add(socket);
    socket.on("close", () => openSockets.delete(socket));
  });

  server.on("upgrade", (req, socket, head) => {
    const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "127.0.0.1"}`);
    const wsKey = req.headers["sec-websocket-key"];
    if (url.pathname !== "/spvs" || typeof wsKey !== "string" || !wsKey.trim()) {
      socket.write("HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n");
      socket.destroy();
      return;
    }

    const accept = createHash("sha1")
      .update(wsKey.trim() + WS_GUID)
      .digest("base64");

    socket.setNoDelay(true);
    socket.write(
      [
        "HTTP/1.1 101 Switching Protocols",
        "Upgrade: websocket",
        "Connection: Upgrade",
        `Sec-WebSocket-Accept: ${accept}`,
        "\r\n",
      ].join("\r\n"),
    );

    openSockets.add(socket);
    socket.on("close", () => openSockets.delete(socket));
    attachWebSocketSession(socket, head, workerOptions);
  });

  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(requestedPort, host, () => {
      const addr = server.address();
      const boundPort = typeof addr === "object" && addr ? addr.port : requestedPort;
      resolve({
        host,
        port: boundPort,
        server,
        close: () =>
          new Promise((resClose) => {
            for (const sock of openSockets) {
              sock.destroy();
            }
            openSockets.clear();
            server.close(() => resClose());
          }),
      });
    });
  });
}

const isDirectCliRun =
  process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];

if (isDirectCliRun) {
  startLocalRelay()
    .then(({ host, port }) => {
      console.log(
        `[${SERVICE_NAME}] listening on ws://${host}:${port}/spvs and http://${host}:${port}/health`,
      );
    })
    .catch((err) => {
      console.error(`[${SERVICE_NAME}] failed to bind:`, err);
      process.exitCode = 1;
    });
}
