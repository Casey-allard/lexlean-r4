#include "rt0.h"

/*
 * Freestanding RV64IMAC Zero-NIC AF_VSOCK HTTP egress client (http).
 * Connects solely over AF_VSOCK (domain 40) to host CID 2 port 1024, where
 * MvmSandbox wraps the request into a 16-byte SpiralCore SPVS frame and
 * substitutes host-side {{MVM_SECRET:<name>}} placeholders outside guest RAM.
 */

#define VSOCK_HOST_CID  2U
#define VSOCK_HTTP_PORT 1024U

static char g_req[16384];
static char g_rx[32768];

static u64 buf_append(char *dst, u64 cap, u64 pos, const char *s) {
    u64 n = m_strlen(s);
    if (pos + n >= cap) {
        n = (pos + 1 < cap) ? (cap - 1 - pos) : 0;
    }
    if (n > 0) {
        m_memcpy(dst + pos, s, n);
        pos += n;
    }
    dst[pos] = 0;
    return pos;
}

static u64 buf_append_u64(char *dst, u64 cap, u64 pos, u64 v) {
    char tmp[32];
    int p = 31;
    tmp[p] = 0;
    if (v == 0) {
        tmp[--p] = '0';
    } else {
        while (v > 0) {
            tmp[--p] = (char)('0' + (v % 10ULL));
            v /= 10ULL;
        }
    }
    return buf_append(dst, cap, pos, &tmp[p]);
}

static void extract_host_and_path(const char *url, char *host_out, u64 host_cap, const char **path_out) {
    const char *p = url;
    if (m_strncmp(p, "https://", 8) == 0) p += 8;
    else if (m_strncmp(p, "http://", 7) == 0) p += 7;
    u64 i = 0;
    while (p[i] && p[i] != '/' && p[i] != '?') {
        if (i + 1 < host_cap) host_out[i] = p[i];
        i++;
    }
    host_out[(i < host_cap) ? i : (host_cap - 1)] = 0;
    *path_out = p[i] ? (p + i) : "/";
}

int main(int argc, char **argv, char **envp) {
    (void)envp;
    const char *method = "GET";
    const char *url = 0;
    const char *body = 0;
    const char *headers[16];
    int nheaders = 0;
    u32 port = VSOCK_HTTP_PORT;

    for (int i = 1; i < argc; i++) {
        const char *a = argv[i];
        if (m_strcmp(a, "-I") == 0 || m_strcmp(a, "--head") == 0) {
            method = "HEAD";
        } else if ((m_strcmp(a, "-X") == 0 || m_strcmp(a, "--request") == 0) && i + 1 < argc) {
            method = argv[++i];
        } else if ((m_strcmp(a, "-H") == 0 || m_strcmp(a, "--header") == 0) && i + 1 < argc) {
            if (nheaders < 16) headers[nheaders++] = argv[++i];
            else i++;
        } else if ((m_strcmp(a, "-d") == 0 || m_strcmp(a, "--data") == 0) && i + 1 < argc) {
            body = argv[++i];
            if (m_strcmp(method, "GET") == 0) method = "POST";
        } else if (m_strcmp(a, "--port") == 0 && i + 1 < argc) {
            port = 0;
            for (const char *q = argv[++i]; *q >= '0' && *q <= '9'; q++) {
                port = port * 10U + (u32)(*q - '0');
            }
        } else if (!url && (m_strcmp(a, "GET") == 0 || m_strcmp(a, "POST") == 0 ||
                            m_strcmp(a, "HEAD") == 0 || m_strcmp(a, "PUT") == 0 ||
                            m_strcmp(a, "DELETE") == 0)) {
            method = a;
        } else if (!url) {
            url = a;
        }
    }

    if (!url) {
        m_puts_fd(2, "usage: http [-X METHOD] [-H HEADER] [-d DATA] <url>\n");
        return 1;
    }

    char host[256];
    const char *path = "/";
    extract_host_and_path(url, host, sizeof(host), &path);
    if (host[0] == 0) {
        m_puts_fd(2, "http: invalid url\n");
        return 1;
    }

    u64 len = 0;
    len = buf_append(g_req, sizeof(g_req), len, method);
    len = buf_append(g_req, sizeof(g_req), len, " ");
    len = buf_append(g_req, sizeof(g_req), len, url);
    len = buf_append(g_req, sizeof(g_req), len, " HTTP/1.1\r\nHost: ");
    len = buf_append(g_req, sizeof(g_req), len, host);
    len = buf_append(g_req, sizeof(g_req), len, "\r\nX-MVM-Path: ");
    len = buf_append(g_req, sizeof(g_req), len, path);
    len = buf_append(g_req, sizeof(g_req), len, "\r\nUser-Agent: uor-mvm-http/1.0\r\n");
    for (int i = 0; i < nheaders; i++) {
        len = buf_append(g_req, sizeof(g_req), len, headers[i]);
        len = buf_append(g_req, sizeof(g_req), len, "\r\n");
    }
    if (body) {
        u64 blen = m_strlen(body);
        len = buf_append(g_req, sizeof(g_req), len, "Content-Length: ");
        len = buf_append_u64(g_req, sizeof(g_req), len, blen);
        len = buf_append(g_req, sizeof(g_req), len, "\r\n\r\n");
        len = buf_append(g_req, sizeof(g_req), len, body);
    } else {
        len = buf_append(g_req, sizeof(g_req), len, "\r\n");
    }

    long sock = sys3(SYS_socket, AF_VSOCK, SOCK_STREAM, 0);
    if (sock < 0) {
        m_puts_fd(2, "http: AF_VSOCK socket refused\n");
        return 2;
    }

    struct m_sockaddr_vm addr;
    m_memset(&addr, 0, sizeof(addr));
    addr.svm_family = AF_VSOCK;
    addr.svm_cid = VSOCK_HOST_CID;
    addr.svm_port = port;

    if (sys3(SYS_connect, sock, (long)&addr, sizeof(addr)) < 0) {
        sys1(SYS_close, sock);
        m_puts_fd(2, "http: AF_VSOCK connect failed\n");
        return 3;
    }

    long sent = sys6(SYS_sendto, sock, (long)g_req, (long)len, 0, 0, 0);
    if (sent < 0) {
        sys1(SYS_close, sock);
        m_puts_fd(2, "http: AF_VSOCK sendto failed\n");
        return 4;
    }

    int got_any = 0;
    while (1) {
        long n = sys6(SYS_recvfrom, sock, (long)g_rx, sizeof(g_rx), 0, 0, 0);
        if (n <= 0) break;
        m_write_fd(1, g_rx, (u64)n);
        got_any = 1;
        if (g_rx[n - 1] != '\n') {
            m_puts("\n");
        }
    }

    sys1(SYS_close, sock);
    return got_any ? 0 : 0;
}
