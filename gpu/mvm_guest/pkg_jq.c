#include "rt0.h"

static u64 read_all(int fd, char *buf, u64 cap) {
    u64 off = 0;
    while (off + 1 < cap) {
        long r = sys3(SYS_read, fd, (long)(buf + off), (long)(cap - 1 - off));
        if (r <= 0) break;
        off += (u64)r;
    }
    buf[off] = 0;
    return off;
}

static const char *skip_ws(const char *p) {
    while (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r') p++;
    return p;
}

static const char *skip_value(const char *p) {
    p = skip_ws(p);
    if (*p == '"') {
        p++;
        while (*p) {
            if (*p == '\\' && p[1]) { p += 2; continue; }
            if (*p == '"') return p + 1;
            p++;
        }
        return p;
    }
    if (*p == '{' || *p == '[') {
        char open = *p;
        char close = (open == '{') ? '}' : ']';
        int depth = 1;
        p++;
        while (*p && depth > 0) {
            if (*p == '"') {
                p++;
                while (*p) {
                    if (*p == '\\' && p[1]) { p += 2; continue; }
                    if (*p == '"') { p++; break; }
                    p++;
                }
                continue;
            }
            if (*p == open) depth++;
            else if (*p == close) depth--;
            p++;
        }
        return p;
    }
    while (*p && *p != ',' && *p != '}' && *p != ']' && *p != ' ' && *p != '\n' && *p != '\r' && *p != '\t') p++;
    return p;
}

static const char *find_key_in_obj(const char *obj, const char *key, u64 klen) {
    const char *p = skip_ws(obj);
    if (*p != '{') return 0;
    p = skip_ws(p + 1);
    while (*p && *p != '}') {
        p = skip_ws(p);
        if (*p != '"') return 0;
        const char *ks = p + 1;
        const char *ke = ks;
        while (*ke) {
            if (*ke == '\\' && ke[1]) { ke += 2; continue; }
            if (*ke == '"') break;
            ke++;
        }
        u64 len = (u64)(ke - ks);
        p = (*ke == '"') ? (ke + 1) : ke;
        p = skip_ws(p);
        if (*p != ':') return 0;
        p = skip_ws(p + 1);
        if (len == klen && m_strncmp(ks, key, klen) == 0) {
            return p;
        }
        p = skip_value(p);
        p = skip_ws(p);
        if (*p == ',') p++;
    }
    return 0;
}

static const char *find_idx_in_arr(const char *arr, int target_idx) {
    const char *p = skip_ws(arr);
    if (*p != '[') return 0;
    p = skip_ws(p + 1);
    int idx = 0;
    while (*p && *p != ']') {
        p = skip_ws(p);
        if (idx == target_idx) return p;
        p = skip_value(p);
        p = skip_ws(p);
        if (*p == ',') p++;
        idx++;
    }
    return 0;
}

static void print_indent(int n) {
    for (int i = 0; i < n * 2; i++) m_write_fd(1, " ", 1);
}

static void emit_value(const char *val, u64 len, int raw_mode, int compact) {
    if (raw_mode && len >= 2 && val[0] == '"' && val[len - 1] == '"') {
        for (u64 i = 1; i + 1 < len; i++) {
            if (val[i] == '\\' && i + 2 < len) {
                char nxt = val[i + 1];
                if (nxt == 'n') { m_puts("\n"); i++; continue; }
                if (nxt == 't') { m_puts("\t"); i++; continue; }
                if (nxt == '"' || nxt == '\\') { m_write_fd(1, &nxt, 1); i++; continue; }
            }
            m_write_fd(1, &val[i], 1);
        }
        m_puts("\n");
        return;
    }
    if (compact) {
        int in_str = 0;
        for (u64 i = 0; i < len; i++) {
            char c = val[i];
            if (c == '"' && (i == 0 || val[i - 1] != '\\')) in_str = !in_str;
            if (!in_str && (c == ' ' || c == '\n' || c == '\r' || c == '\t')) continue;
            m_write_fd(1, &c, 1);
        }
        m_puts("\n");
        return;
    }
    int depth = 0;
    int in_str = 0;
    for (u64 i = 0; i < len; i++) {
        char c = val[i];
        if (c == '"' && (i == 0 || val[i - 1] != '\\')) {
            in_str = !in_str;
            m_write_fd(1, &c, 1);
            continue;
        }
        if (in_str) {
            m_write_fd(1, &c, 1);
            continue;
        }
        if (c == ' ' || c == '\n' || c == '\r' || c == '\t') continue;
        if (c == '{' || c == '[') {
            m_write_fd(1, &c, 1);
            depth++;
            m_puts("\n");
            print_indent(depth);
        } else if (c == '}' || c == ']') {
            depth = (depth > 0) ? (depth - 1) : 0;
            m_puts("\n");
            print_indent(depth);
            m_write_fd(1, &c, 1);
        } else if (c == ',') {
            m_puts(",\n");
            print_indent(depth);
        } else if (c == ':') {
            m_puts(": ");
        } else {
            m_write_fd(1, &c, 1);
        }
    }
    m_puts("\n");
}

int main(int argc, char **argv, char **envp) {
    (void)envp;
    int raw_mode = 0;
    int compact = 0;
    const char *filter = ".";
    const char *file_path = 0;
    int filter_set = 0;

    for (int i = 1; i < argc; i++) {
        if (m_strcmp(argv[i], "-r") == 0) raw_mode = 1;
        else if (m_strcmp(argv[i], "-c") == 0) compact = 1;
        else if (!filter_set) { filter = argv[i]; filter_set = 1; }
        else file_path = argv[i];
    }

    u64 cap = 262144;
    long mem = sys6(SYS_mmap, 0, (long)cap, 3, 0x22, -1, 0);
    if (mem < 0) return 1;
    char *buf = (char *)mem;

    int fd = 0;
    if (file_path) {
        long ofd = sys4(SYS_openat, AT_FDCWD, (long)file_path, O_RDONLY, 0);
        if (ofd < 0) {
            m_puts_fd(2, "jq: cannot open input file\n");
            return 2;
        }
        fd = (int)ofd;
    }
    read_all(fd, buf, cap);
    if (fd != 0) sys1(SYS_close, fd);

    const char *cur = skip_ws(buf);
    const char *fp = filter;
    if (*fp == '.') fp++;
    while (*fp && cur) {
        if (*fp == '[') {
            fp++;
            int idx = 0;
            while (*fp >= '0' && *fp <= '9') {
                idx = idx * 10 + (*fp - '0');
                fp++;
            }
            if (*fp == ']') fp++;
            if (*fp == '.') fp++;
            cur = find_idx_in_arr(cur, idx);
        } else {
            const char *ks = fp;
            while (*fp && *fp != '.' && *fp != '[') fp++;
            u64 klen = (u64)(fp - ks);
            if (*fp == '.') fp++;
            if (klen > 0) cur = find_key_in_obj(cur, ks, klen);
        }
    }

    if (!cur || !*cur) {
        m_puts("null\n");
        return 0;
    }
    const char *end = skip_value(cur);
    emit_value(cur, (u64)(end - cur), raw_mode, compact);
    return 0;
}
