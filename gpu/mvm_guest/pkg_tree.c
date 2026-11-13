#include "rt0.h"

static long g_dirs = 0;
static long g_files = 0;

static void join_child(char *out, u64 cap, const char *dir, const char *name) {
    u64 dl = m_strlen(dir);
    u64 nl = m_strlen(name);
    u64 pos = 0;
    for (u64 i = 0; i < dl && pos + 1 < cap; i++) out[pos++] = dir[i];
    if (pos > 0 && out[pos - 1] != '/' && pos + 1 < cap) out[pos++] = '/';
    for (u64 i = 0; i < nl && pos + 1 < cap; i++) out[pos++] = name[i];
    out[pos] = 0;
}

static void walk_tree(const char *dir, const char *prefix, int depth, int max_depth) {
    if (depth >= max_depth) return;
    long dfd = sys4(SYS_openat, AT_FDCWD, (long)dir, O_RDONLY | O_DIRECTORY, 0);
    if (dfd < 0) return;

    char names[64][64];
    u8 types[64];
    int count = 0;
    u8 dbuf[1024];

    for (;;) {
        long nread = sys3(SYS_getdents64, dfd, (long)dbuf, sizeof(dbuf));
        if (nread <= 0) break;
        long pos = 0;
        while (pos < nread) {
            struct m_linux_dirent64 *d = (struct m_linux_dirent64 *)(dbuf + pos);
            if (m_strcmp(d->d_name, ".") != 0 && m_strcmp(d->d_name, "..") != 0 && count < 64) {
                u64 nl = m_strlen(d->d_name);
                if (nl >= 63) nl = 63;
                m_memcpy(names[count], d->d_name, nl);
                names[count][nl] = 0;
                types[count] = d->d_type;
                count++;
            }
            pos += d->d_reclen;
        }
    }
    sys1(SYS_close, dfd);

    for (int i = 0; i < count; i++) {
        int is_last = (i + 1 == count);
        m_puts(prefix);
        m_puts(is_last ? "`-- " : "|-- ");
        m_puts(names[i]);
        m_puts("\n");

        if (types[i] == 4) {
            g_dirs++;
            char child[512];
            char next_pref[256];
            join_child(child, sizeof(child), dir, names[i]);
            u64 pl = m_strlen(prefix);
            if (pl + 5 < sizeof(next_pref)) {
                m_memcpy(next_pref, prefix, pl);
                const char *seg = is_last ? "    " : "|   ";
                m_memcpy(next_pref + pl, seg, 4);
                next_pref[pl + 4] = 0;
                walk_tree(child, next_pref, depth + 1, max_depth);
            }
        } else {
            g_files++;
        }
    }
}

int main(int argc, char **argv, char **envp) {
    (void)envp;
    const char *root = ".";
    int max_depth = 6;
    for (int i = 1; i < argc; i++) {
        if (m_strcmp(argv[i], "-L") == 0 && i + 1 < argc) {
            int d = 0;
            for (const char *p = argv[++i]; *p >= '0' && *p <= '9'; p++) d = d * 10 + (*p - '0');
            if (d > 0) max_depth = d;
        } else if (argv[i][0] != '-') {
            root = argv[i];
        }
    }
    m_puts(root);
    m_puts("\n");
    walk_tree(root, "", 0, max_depth);
    m_puts("\n");
    m_put_i64(1, g_dirs);
    m_puts(" directories, ");
    m_put_i64(1, g_files);
    m_puts(" files\n");
    return 0;
}
