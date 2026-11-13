#include "rt0.h"

/*
 * Freestanding RV64IMAC head utility.
 * Supports -n <lines>, -<lines>, and -c <bytes> over stdin or files.
 */

static u8 g_buf[8192];

static u64 parse_u64(const char *s) {
    u64 v = 0;
    while (*s >= '0' && *s <= '9') {
        v = v * 10ULL + (u64)(*s - '0');
        s++;
    }
    return v;
}

static void head_fd(int fd, u64 max_lines, u64 max_bytes, int byte_mode) {
    u64 lines = 0;
    u64 bytes = 0;
    while (1) {
        if (byte_mode && bytes >= max_bytes) break;
        if (!byte_mode && lines >= max_lines) break;
        long n = sys3(SYS_read, fd, (long)g_buf, sizeof(g_buf));
        if (n <= 0) break;
        if (byte_mode) {
            u64 rem = max_bytes - bytes;
            u64 take = ((u64)n < rem) ? (u64)n : rem;
            m_write_fd(1, g_buf, take);
            bytes += take;
            if (bytes >= max_bytes) break;
        } else {
            u64 take = 0;
            for (long i = 0; i < n; i++) {
                take++;
                if (g_buf[i] == '\n') {
                    lines++;
                    if (lines >= max_lines) break;
                }
            }
            m_write_fd(1, g_buf, take);
            if (lines >= max_lines) break;
        }
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
        head_fd(0, max_lines, max_bytes, byte_mode);
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
            head_fd(0, max_lines, max_bytes, byte_mode);
        } else {
            long fd = sys4(SYS_openat, AT_FDCWD, (long)files[i], O_RDONLY, 0);
            if (fd < 0) {
                m_puts_fd(2, "head: cannot open ");
                m_puts_fd(2, files[i]);
                m_puts_fd(2, "\n");
                rc = 1;
                continue;
            }
            head_fd((int)fd, max_lines, max_bytes, byte_mode);
            sys1(SYS_close, fd);
        }
    }
    return rc;
}
