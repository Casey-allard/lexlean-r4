#include "rt0.h"

/*
 * Freestanding RV64IMAC arbitrary-expression 64-bit integer calculator (bc).
 * Supports +, -, *, /, %, ^ (power), unary +/-, parentheses, comparisons,
 * semicolon/newline-separated statements, and single-letter variables a..z.
 */

static char g_inbuf[65536];
static i64 g_vars[26];
static const char *g_p;
static int g_err;

static void skip_ws(void) {
    while (*g_p == ' ' || *g_p == '\t' || *g_p == '\r') g_p++;
}

static i64 parse_cmp(void);

static i64 ipow64(i64 base, i64 exp) {
    if (exp < 0) return 0;
    i64 res = 1;
    i64 b = base;
    u64 e = (u64)exp;
    while (e > 0) {
        if (e & 1ULL) res *= b;
        b *= b;
        e >>= 1;
    }
    return res;
}

static i64 parse_primary(void) {
    skip_ws();
    if (*g_p == '(') {
        g_p++;
        i64 v = parse_cmp();
        skip_ws();
        if (*g_p == ')') g_p++;
        else g_err = 1;
        return v;
    }
    if (*g_p >= 'a' && *g_p <= 'z') {
        int idx = (int)(*g_p - 'a');
        g_p++;
        skip_ws();
        if (*g_p == '=' && g_p[1] != '=') {
            g_p++;
            i64 val = parse_cmp();
            g_vars[idx] = val;
            return val;
        }
        return g_vars[idx];
    }
    if (*g_p == '0' && (g_p[1] == 'x' || g_p[1] == 'X')) {
        g_p += 2;
        u64 uv = 0;
        int any = 0;
        while (1) {
            char c = *g_p;
            u64 d;
            if (c >= '0' && c <= '9') d = (u64)(c - '0');
            else if (c >= 'a' && c <= 'f') d = (u64)(c - 'a' + 10);
            else if (c >= 'A' && c <= 'F') d = (u64)(c - 'A' + 10);
            else break;
            uv = (uv << 4) | d;
            g_p++;
            any = 1;
        }
        if (!any) g_err = 1;
        return (i64)uv;
    }
    if (*g_p >= '0' && *g_p <= '9') {
        u64 uv = 0;
        while (*g_p >= '0' && *g_p <= '9') {
            uv = uv * 10ULL + (u64)(*g_p - '0');
            g_p++;
        }
        if (*g_p == '.') {
            g_p++;
            while (*g_p >= '0' && *g_p <= '9') g_p++;
        }
        return (i64)uv;
    }
    g_err = 1;
    return 0;
}

static i64 parse_unary(void) {
    skip_ws();
    if (*g_p == '+') { g_p++; return parse_unary(); }
    if (*g_p == '-') { g_p++; return -parse_unary(); }
    if (*g_p == '!') { g_p++; return !parse_unary(); }
    return parse_primary();
}

static i64 parse_pow(void) {
    i64 lhs = parse_unary();
    skip_ws();
    if (*g_p == '^') {
        g_p++;
        i64 rhs = parse_pow();
        return ipow64(lhs, rhs);
    }
    return lhs;
}

static i64 parse_mul(void) {
    i64 lhs = parse_pow();
    while (1) {
        skip_ws();
        char op = *g_p;
        if (op != '*' && op != '/' && op != '%') break;
        g_p++;
        i64 rhs = parse_pow();
        if (op == '*') lhs = lhs * rhs;
        else if (op == '/') {
            if (rhs == 0) { m_puts_fd(2, "bc: divide by zero\n"); g_err = 2; return 0; }
            lhs = lhs / rhs;
        } else {
            if (rhs == 0) { m_puts_fd(2, "bc: modulo by zero\n"); g_err = 2; return 0; }
            lhs = lhs % rhs;
        }
    }
    return lhs;
}

static i64 parse_add(void) {
    i64 lhs = parse_mul();
    while (1) {
        skip_ws();
        char op = *g_p;
        if (op != '+' && op != '-') break;
        g_p++;
        i64 rhs = parse_mul();
        if (op == '+') lhs = lhs + rhs;
        else lhs = lhs - rhs;
    }
    return lhs;
}

static i64 parse_cmp(void) {
    i64 lhs = parse_add();
    while (1) {
        skip_ws();
        if (g_p[0] == '=' && g_p[1] == '=') { g_p += 2; lhs = (lhs == parse_add()); }
        else if (g_p[0] == '!' && g_p[1] == '=') { g_p += 2; lhs = (lhs != parse_add()); }
        else if (g_p[0] == '<' && g_p[1] == '=') { g_p += 2; lhs = (lhs <= parse_add()); }
        else if (g_p[0] == '>' && g_p[1] == '=') { g_p += 2; lhs = (lhs >= parse_add()); }
        else if (g_p[0] == '<') { g_p += 1; lhs = (lhs < parse_add()); }
        else if (g_p[0] == '>') { g_p += 1; lhs = (lhs > parse_add()); }
        else break;
    }
    return lhs;
}

static int eval_stream(const char *src) {
    g_p = src;
    int rc = 0;
    while (*g_p) {
        while (*g_p == ' ' || *g_p == '\t' || *g_p == '\r' || *g_p == '\n' || *g_p == ';') {
            g_p++;
        }
        if (!*g_p) break;
        if (*g_p == '#') {
            while (*g_p && *g_p != '\n') g_p++;
            continue;
        }
        if (m_strncmp(g_p, "quit", 4) == 0 && (g_p[4] == 0 || g_p[4] == '\n' || g_p[4] == ';')) {
            break;
        }
        if (m_strncmp(g_p, "scale", 5) == 0) {
            const char *q = g_p + 5;
            while (*q == ' ' || *q == '\t') q++;
            if (*q == '=') {
                g_p = q + 1;
                (void)parse_cmp();
                continue;
            }
        }
        g_err = 0;
        i64 v = parse_cmp();
        skip_ws();
        if (*g_p && *g_p != '\n' && *g_p != ';') {
            g_err = 1;
            while (*g_p && *g_p != '\n' && *g_p != ';') g_p++;
        }
        if (g_err == 1) {
            m_puts_fd(2, "bc: syntax error\n");
            rc = 1;
        } else if (g_err == 2) {
            rc = 1;
        } else {
            m_put_i64(1, v);
            m_puts("\n");
        }
    }
    return rc;
}

static int eval_fd(int fd) {
    u64 off = 0;
    while (off + 1 < sizeof(g_inbuf)) {
        long n = sys3(SYS_read, fd, (long)(g_inbuf + off), (long)(sizeof(g_inbuf) - 1 - off));
        if (n <= 0) break;
        off += (u64)n;
    }
    g_inbuf[off] = 0;
    return eval_stream(g_inbuf);
}

int main(int argc, char **argv, char **envp) {
    (void)envp;
    int files = 0;
    int rc = 0;
    for (int i = 1; i < argc; i++) {
        if (m_strcmp(argv[i], "-l") == 0 || m_strcmp(argv[i], "-q") == 0) {
            continue;
        }
        if (m_strcmp(argv[i], "-e") == 0 && i + 1 < argc) {
            files++;
            if (eval_stream(argv[++i]) != 0) rc = 1;
            continue;
        }
        files++;
        long fd = sys4(SYS_openat, AT_FDCWD, (long)argv[i], O_RDONLY, 0);
        if (fd < 0) {
            m_puts_fd(2, "bc: cannot open ");
            m_puts_fd(2, argv[i]);
            m_puts_fd(2, "\n");
            rc = 1;
            continue;
        }
        if (eval_fd((int)fd) != 0) rc = 1;
        sys1(SYS_close, fd);
    }
    if (files == 0) {
        if (eval_fd(0) != 0) rc = 1;
    }
    return rc;
}
