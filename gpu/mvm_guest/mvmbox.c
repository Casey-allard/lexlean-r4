#include "rt0.h"

/* FIPS 180-4 SHA-256 implementation for RV64IMAC guest sha256sum */
static const u32 K256[64] = {
    0x428a2f98u,0x71374491u,0xb5c0fbcfu,0xe9b5dba5u,0x3956c25bu,0x59f111f1u,0x923f82a4u,0xab1c5ed5u,
    0xd807aa98u,0x12835b01u,0x243185beu,0x550c7dc3u,0x72be5d74u,0x80deb1feu,0x9bdc06a7u,0xc19bf174u,
    0xe49b69c1u,0xefbe4786u,0x0fc19dc6u,0x240ca1ccu,0x2de92c6fu,0x4a7484aau,0x5cb0a9dcu,0x76f988dau,
    0x983e5152u,0xa831c66du,0xb00327c8u,0xbf597fc7u,0xc6e00bf3u,0xd5a79147u,0x06ca6351u,0x14292967u,
    0x27b70a85u,0x2e1b2138u,0x4d2c6dfcu,0x53380d13u,0x650a7354u,0x766a0abbu,0x81c2c92eu,0x92722c85u,
    0xa2bfe8a1u,0xa81a664bu,0xc24b8b70u,0xc76c51a3u,0xd192e819u,0xd6990624u,0xf40e3585u,0x106aa070u,
    0x19a4c116u,0x1e376c08u,0x2748774cu,0x34b0bcb5u,0x391c0cb3u,0x4ed8aa4au,0x5b9cca4fu,0x682e6ff3u,
    0x748f82eeu,0x78a5636fu,0x84c87814u,0x8cc70208u,0x90befffau,0xa4506cebu,0xbef9a3f7u,0xc67178f2u
};

static inline u32 rotr32(u32 x, u32 n) { return (x >> n) | (x << (32 - n)); }

static void sha256_block(u32 st[8], const u8 blk[64]) {
    u32 w[64];
    for (int i = 0; i < 16; i++) {
        w[i] = ((u32)blk[i * 4] << 24) | ((u32)blk[i * 4 + 1] << 16) |
               ((u32)blk[i * 4 + 2] << 8) | (u32)blk[i * 4 + 3];
    }
    for (int i = 16; i < 64; i++) {
        u32 s0 = rotr32(w[i - 15], 7) ^ rotr32(w[i - 15], 18) ^ (w[i - 15] >> 3);
        u32 s1 = rotr32(w[i - 2], 17) ^ rotr32(w[i - 2], 19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16] + s0 + w[i - 7] + s1;
    }
    u32 a = st[0], b = st[1], c = st[2], d = st[3];
    u32 e = st[4], f = st[5], g = st[6], h = st[7];
    for (int i = 0; i < 64; i++) {
        u32 S1 = rotr32(e, 6) ^ rotr32(e, 11) ^ rotr32(e, 25);
        u32 ch = (e & f) ^ ((~e) & g);
        u32 temp1 = h + S1 + ch + K256[i] + w[i];
        u32 S0 = rotr32(a, 2) ^ rotr32(a, 13) ^ rotr32(a, 22);
        u32 maj = (a & b) ^ (a & c) ^ (b & c);
        u32 temp2 = S0 + maj;
        h = g; g = f; f = e; e = d + temp1;
        d = c; c = b; b = a; a = temp1 + temp2;
    }
    st[0] += a; st[1] += b; st[2] += c; st[3] += d;
    st[4] += e; st[5] += f; st[6] += g; st[7] += h;
}

static int cmd_sha256sum_fd(int fd, const char *label) {
    u32 st[8] = {
        0x6a09e667u, 0xbb67ae85u, 0x3c6ef372u, 0xa54ff53au,
        0x510e527fu, 0x9b05688cu, 0x1f83d9abu, 0x5be0cd19u
    };
    u8 buf[64];
    u64 total = 0;
    u64 carry = 0;
    for (;;) {
        long r = sys3(SYS_read, fd, (long)(buf + carry), (long)(64 - carry));
        if (r < 0) return 1;
        if (r == 0) break;
        total += (u64)r;
        carry += (u64)r;
        if (carry == 64) {
            sha256_block(st, buf);
            carry = 0;
        }
    }
    buf[carry++] = 0x80u;
    if (carry > 56) {
        while (carry < 64) buf[carry++] = 0;
        sha256_block(st, buf);
        carry = 0;
    }
    while (carry < 56) buf[carry++] = 0;
    u64 bits = total * 8;
    for (int i = 0; i < 8; i++) {
        buf[56 + i] = (u8)(bits >> (56 - i * 8));
    }
    sha256_block(st, buf);

    const char *hex = "0123456789abcdef";
    char out[66];
    for (int i = 0; i < 8; i++) {
        u32 v = st[i];
        for (int j = 0; j < 8; j++) {
            out[i * 8 + j] = hex[(v >> (28 - j * 4)) & 0xf];
        }
    }
    out[64] = ' ';
    out[65] = ' ';
    m_write_fd(1, out, 66);
    m_puts(label);
    m_puts("\n");
    return 0;
}

static const char *basename_of(const char *path) {
    const char *last = path;
    for (const char *p = path; p && *p; p++) {
        if (*p == '/') last = p + 1;
    }
    return last;
}

static int run_builtin(const char *applet, int argc, char **argv, char **envp);

static int tokenize_cmd(char *line, char *out_argv[32]) {
    int n = 0;
    char *p = line;
    while (*p && n < 31) {
        while (*p == ' ' || *p == '\t' || *p == '\n') p++;
        if (!*p) break;
        if (*p == '"' || *p == '\'') {
            char q = *p++;
            out_argv[n++] = p;
            while (*p && *p != q) p++;
            if (*p == q) *p++ = 0;
        } else {
            out_argv[n++] = p;
            while (*p && *p != ' ' && *p != '\t' && *p != '\n') p++;
            if (*p) *p++ = 0;
        }
    }
    out_argv[n] = 0;
    return n;
}

static int exec_single(char *seg, char **envp) {
    char *av[32];
    int ac = tokenize_cmd(seg, av);
    if (ac == 0) return 0;
    const char *base = basename_of(av[0]);
    if (av[0][0] == '/' || (av[0][0] == '.' && av[0][1] == '/')) {
        long pid = sys4(SYS_clone, 17, 0, 0, 0);
        if (pid == 0) {
            sys3(SYS_execve, (long)av[0], (long)av, (long)envp);
            sys1(SYS_exit_group, 127);
        }
        int st = 0;
        sys4(SYS_wait4, pid, (long)&st, 0, 0);
        return (st >> 8) & 0xff;
    }
    return run_builtin(base, ac, av, envp);
}

static int cmd_sh(int argc, char **argv, char **envp) {
    if (argc < 3 || m_strcmp(argv[1], "-c") != 0) {
        m_puts_fd(2, "sh: expected -c <script>\n");
        return 1;
    }
    char buf[2048];
    u64 len = m_strlen(argv[2]);
    if (len >= sizeof(buf)) len = sizeof(buf) - 1;
    m_memcpy(buf, argv[2], len);
    buf[len] = 0;

    /* Check for single pipe `left | right` */
    char *pipe_pos = 0;
    char quote = 0;
    for (u64 i = 0; i < len; i++) {
        char c = buf[i];
        if (c == '"' || c == '\'') {
            if (quote == c) quote = 0;
            else if (!quote) quote = c;
        } else if (!quote && c == '|' && buf[i + 1] != '|' && (i == 0 || buf[i - 1] != '|')) {
            pipe_pos = &buf[i];
            break;
        }
    }

    if (pipe_pos) {
        *pipe_pos = 0;
        char *left = buf;
        char *right = pipe_pos + 1;
        int fds[2] = { -1, -1 };
        if (sys2(SYS_pipe2, (long)fds, 0) < 0) return 1;

        long p1 = sys4(SYS_clone, 17, 0, 0, 0);
        if (p1 == 0) {
            sys1(SYS_close, fds[0]);
            sys3(SYS_dup3, fds[1], 1, 0);
            sys1(SYS_close, fds[1]);
            int rc1 = exec_single(left, envp);
            sys1(SYS_exit_group, rc1);
        }
        long p2 = sys4(SYS_clone, 17, 0, 0, 0);
        if (p2 == 0) {
            sys1(SYS_close, fds[1]);
            sys3(SYS_dup3, fds[0], 0, 0);
            sys1(SYS_close, fds[0]);
            int rc2 = exec_single(right, envp);
            sys1(SYS_exit_group, rc2);
        }
        sys1(SYS_close, fds[0]);
        sys1(SYS_close, fds[1]);
        int st1 = 0, st2 = 0;
        sys4(SYS_wait4, p1, (long)&st1, 0, 0);
        sys4(SYS_wait4, p2, (long)&st2, 0, 0);
        return (st2 >> 8) & 0xff;
    }

    return exec_single(buf, envp);
}

static int run_builtin(const char *applet, int argc, char **argv, char **envp) {
    if (m_strcmp(applet, "true") == 0) return 0;
    if (m_strcmp(applet, "false") == 0) return 1;
    if (m_strcmp(applet, "sh") == 0) return cmd_sh(argc, argv, envp);

    if (m_strcmp(applet, "uname") == 0) {
        struct m_utsname u;
        m_memset(&u, 0, sizeof(u));
        sys1(SYS_uname, (long)&u);
        int all = (argc >= 2 && m_strcmp(argv[1], "-a") == 0);
        int mach = (argc >= 2 && m_strcmp(argv[1], "-m") == 0);
        int rel = (argc >= 2 && m_strcmp(argv[1], "-r") == 0);
        if (all) {
            m_puts(u.sysname); m_puts(" ");
            m_puts(u.nodename); m_puts(" ");
            m_puts(u.release); m_puts(" ");
            m_puts(u.version); m_puts(" ");
            m_puts(u.machine); m_puts(" GNU/Linux\n");
        } else if (mach) {
            m_puts(u.machine); m_puts("\n");
        } else if (rel) {
            m_puts(u.release); m_puts("\n");
        } else {
            m_puts(u.sysname); m_puts("\n");
        }
        return 0;
    }

    if (m_strcmp(applet, "echo") == 0) {
        int start = 1;
        int newline = 1;
        if (argc >= 2 && m_strcmp(argv[1], "-n") == 0) {
            newline = 0;
            start = 2;
        }
        for (int i = start; i < argc; i++) {
            if (i > start) m_puts(" ");
            m_puts(argv[i]);
        }
        if (newline) m_puts("\n");
        return 0;
    }

    if (m_strcmp(applet, "pwd") == 0) {
        char cwd[256];
        m_memset(cwd, 0, sizeof(cwd));
        if (sys2(SYS_getcwd, (long)cwd, sizeof(cwd)) >= 0) {
            m_puts(cwd); m_puts("\n");
            return 0;
        }
        return 1;
    }

    if (m_strcmp(applet, "whoami") == 0) {
        m_puts("lexlean\n");
        return 0;
    }

    if (m_strcmp(applet, "id") == 0) {
        long uid = sys0(SYS_getuid);
        m_puts("uid="); m_put_i64(1, uid); m_puts("(lexlean) gid=");
        m_put_i64(1, uid); m_puts("(lexlean)\n");
        return 0;
    }

    if (m_strcmp(applet, "cat") == 0) {
        if (argc < 2) {
            char b[512];
            for (;;) {
                long r = sys3(SYS_read, 0, (long)b, sizeof(b));
                if (r <= 0) break;
                m_write_fd(1, b, (u64)r);
            }
            return 0;
        }
        for (int i = 1; i < argc; i++) {
            long fd = sys4(SYS_openat, AT_FDCWD, (long)argv[i], O_RDONLY, 0);
            if (fd < 0) {
                m_puts_fd(2, "cat: cannot open ");
                m_puts_fd(2, argv[i]);
                m_puts_fd(2, "\n");
                return 1;
            }
            char b[512];
            for (;;) {
                long r = sys3(SYS_read, fd, (long)b, sizeof(b));
                if (r <= 0) break;
                m_write_fd(1, b, (u64)r);
            }
            sys1(SYS_close, fd);
        }
        return 0;
    }

    if (m_strcmp(applet, "ls") == 0) {
        const char *target = ".";
        for (int i = 1; i < argc; i++) {
            if (argv[i][0] != '-') { target = argv[i]; break; }
        }
        long fd = sys4(SYS_openat, AT_FDCWD, (long)target, O_RDONLY | O_DIRECTORY, 0);
        if (fd < 0) {
            m_puts_fd(2, "ls: cannot access ");
            m_puts_fd(2, target);
            m_puts_fd(2, "\n");
            return 1;
        }
        u8 dbuf[1024];
        for (;;) {
            long nread = sys3(SYS_getdents64, fd, (long)dbuf, sizeof(dbuf));
            if (nread <= 0) break;
            long pos = 0;
            while (pos < nread) {
                struct m_linux_dirent64 *d = (struct m_linux_dirent64 *)(dbuf + pos);
                if (m_strcmp(d->d_name, ".") != 0 && m_strcmp(d->d_name, "..") != 0) {
                    m_puts(d->d_name);
                    m_puts("\n");
                }
                pos += d->d_reclen;
            }
        }
        sys1(SYS_close, fd);
        return 0;
    }

    if (m_strcmp(applet, "grep") == 0) {
        if (argc < 2) return 2;
        const char *pat = argv[1];
        int fd = 0;
        if (argc >= 3) {
            long ofd = sys4(SYS_openat, AT_FDCWD, (long)argv[2], O_RDONLY, 0);
            if (ofd < 0) return 2;
            fd = (int)ofd;
        }
        char line[1024];
        u64 lpos = 0;
        int matched = 0;
        char ch;
        for (;;) {
            long r = sys3(SYS_read, fd, (long)&ch, 1);
            if (r <= 0) {
                if (lpos > 0) {
                    line[lpos] = 0;
                    if (m_strstr(line, pat)) {
                        m_puts(line); m_puts("\n");
                        matched = 1;
                    }
                }
                break;
            }
            if (ch == '\n') {
                line[lpos] = 0;
                if (m_strstr(line, pat)) {
                    m_puts(line); m_puts("\n");
                    matched = 1;
                }
                lpos = 0;
            } else if (lpos + 1 < sizeof(line)) {
                line[lpos++] = ch;
            }
        }
        if (fd != 0) sys1(SYS_close, fd);
        return matched ? 0 : 1;
    }

    if (m_strcmp(applet, "wc") == 0) {
        u64 lines = 0, bytes = 0;
        char b[256];
        for (;;) {
            long r = sys3(SYS_read, 0, (long)b, sizeof(b));
            if (r <= 0) break;
            bytes += (u64)r;
            for (long i = 0; i < r; i++) if (b[i] == '\n') lines++;
        }
        if (argc >= 2 && m_strcmp(argv[1], "-l") == 0) {
            m_put_i64(1, (long)lines); m_puts("\n");
        } else {
            m_put_i64(1, (long)lines); m_puts(" "); m_put_i64(1, (long)bytes); m_puts("\n");
        }
        return 0;
    }

    if (m_strcmp(applet, "sha256sum") == 0) {
        if (argc < 2) {
            return cmd_sha256sum_fd(0, "-");
        }
        for (int i = 1; i < argc; i++) {
            long fd = sys4(SYS_openat, AT_FDCWD, (long)argv[i], O_RDONLY, 0);
            if (fd < 0) return 1;
            cmd_sha256sum_fd((int)fd, argv[i]);
            sys1(SYS_close, fd);
        }
        return 0;
    }

    if (m_strcmp(applet, "mkdir") == 0) {
        const char *dir = (argc >= 3 && argv[1][0] == '-') ? argv[2] : (argc >= 2 ? argv[1] : 0);
        if (!dir) return 1;
        return sys3(SYS_mkdirat, AT_FDCWD, (long)dir, 0755) < 0 ? 1 : 0;
    }

    if (m_strcmp(applet, "rm") == 0) {
        const char *target = (argc >= 3 && argv[1][0] == '-') ? argv[2] : (argc >= 2 ? argv[1] : 0);
        if (!target) return 1;
        return sys3(SYS_unlinkat, AT_FDCWD, (long)target, 0) < 0 ? 1 : 0;
    }

    m_puts_fd(2, "mvmbox: applet not found: ");
    m_puts_fd(2, applet);
    m_puts_fd(2, "\n");
    return 127;
}

int main(int argc, char **argv, char **envp) {
    if (argc < 1) return 1;
    const char *base = basename_of(argv[0]);
    if (m_strcmp(base, "mvmbox") == 0) {
        if (argc < 2) {
            m_puts("mvmbox multi-call RV64IMAC userland (sh, uname, ls, cat, grep, wc, echo, pwd, sha256sum, mkdir, rm, id, whoami)\n");
            return 0;
        }
        return run_builtin(basename_of(argv[1]), argc - 1, &argv[1], envp);
    }
    return run_builtin(base, argc, argv, envp);
}
