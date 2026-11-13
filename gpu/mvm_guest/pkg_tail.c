#include "rt0.h"

/*
 * Freestanding RV64IMAC tail utility using a bounded 256 KB ring buffer.
 * Supports -n <lines>, -<lines>, and -c <bytes> over stdin or files.
 */

#define RING_CAP 262144U
static u8 g_ring[RING_CAP];
static u8 g_io[8192];

static u64 parse_u64(const char *s) {
    u64 v = 0;
    while (*s >= '0' && *s <= '9') {
        v = v * 10ULL + (u64)(*s - '0');
        s++;
    }
    return v;
}

static void tail_fd(int fd, u64 max_lines, u64 max_bytes, int byte_mode) {
    u64 total = 0;
    while (1) {
        long n = sys3(SYS_read, fd, (long)g_io, sizeof(g_io));
        if (n <= 0) break;
        for (long i = 0; i < n; i++) {
            g_ring[total % RING_CAP] = g_io[i];
            total++;
        }
    }
    if (total == 0) return;

    u64 kept = (total < RING_CAP) ? total : RING_CAP;
    u64 base = total - kept;

    if (byte_mode) {
        if (max_bytes == 0) return;
        u64 want = (max_bytes < kept) ? max_bytes : kept;
        u64 start = total - want;
        for (u64 pos = start; pos < total; ) {
            u64 idx = pos % RING_CAP;
            u64 chunk = RING_CAP - idx;
            if (pos + chunk > total) chunk = total - pos;
            m_write_fd(1, &g_ring[idx], chunk);
            pos += chunk;
        }
        return;
    }

    if (max_lines == 0) return;
    u64 nl_seen = 0;
    u64 start = base;
    for (u64 k = 0; k < kept; k++) {
        u64 pos = total - 1 - k;
        u8 c = g_ring[pos % RING_CAP];
        if (c == '\n' && pos + 1 < total) {
            nl_seen++;
            if (nl_seen == max_lines) {
                start = pos + 1;
                break;
            }
        }
    }

    for (u64 pos = start; pos < total; ) {
        u64 idx = pos % RING_CAP;
        u64 chunk = RING_CAP - idx;
        if (pos + chunk > total) chunk = total - pos;
        m_write_fd(1, &g_ring[idx], chunk);
        pos += chunk;
    }
}

int main(int argc, char **argv, char **envp) {
    (void)envp;
    u64 max_lines = 10;
    u64 max_bytes = 0;
    int byte_mode = 0;
    const char *files[64];
    int nfiles = 0;

    for (int i = 1; i < argc; i++) {
        const char *a = argv[i];
        if (m_strcmp(a, "-n") == 0 && i + 1 < argc) {
            max_lines = parse_u64(argv[++i]);
            byte_mode = 0;
        } else if (m_strncmp(a, "-n", 2) == 0 && a[2] != 0) {
            max_lines = parse_u64(a + 2);
            byte_mode = 0;
        } else if (m_strcmp(a, "-c") == 0 && i + 1 < argc) {
            max_bytes = parse_u64(argv[++i]);
            byte_mode = 1;
        } else if (m_strncmp(a, "-c", 2) == 0 && a[2] != 0) {
            max_bytes = parse_u64(a + 2);
            byte_mode = 1;
        } else if (a[0] == '-' && a[1] >= '0' && a[1] <= '9') {
            max_lines = parse_u64(a + 1);
            byte_mode = 0;
        } else if (nfiles < 64) {
            files[nfiles++] = a;
        }
    }

    if (nfiles == 0) {
        tail_fd(0, max_lines, max_bytes, byte_mode);
        return 0;
    }

    int rc = 0;
    for (int i = 0; i < nfiles; i++) {
        if (nfiles > 1) {
            if (i > 0) m_puts("\n");
            m_puts("==> ");
            m_puts(files[i]);
            m_puts(" <==\n");
        }
        if (m_strcmp(files[i], "-") == 0) {
            tail_fd(0, max_lines, max_bytes, byte_mode);
        } else {
            long fd = sys4(SYS_openat, AT_FDCWD, (long)files[i], O_RDONLY, 0);
            if (fd < 0) {
                m_puts_fd(2, "tail: cannot open ");
                m_puts_fd(2, files[i]);
                m_puts_fd(2, "\n");
                rc = 1;
                continue;
            }
            tail_fd((int)fd, max_lines, max_bytes, byte_mode);
            sys1(SYS_close, fd);
        }
    }
    return rc;
}
