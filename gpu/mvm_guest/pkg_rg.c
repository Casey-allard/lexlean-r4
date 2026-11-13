#include "rt0.h"

static char to_lo(char c) {
    return (c >= 'A' && c <= 'Z') ? (char)(c + 32) : c;
}

static int match_sub(const char *hay, const char *needle, int icase) {
    if (!*needle) return 1;
    u64 nlen = m_strlen(needle);
    for (; *hay; hay++) {
        u64 j = 0;
        while (j < nlen && hay[j]) {
            char a = icase ? to_lo(hay[j]) : hay[j];
            char b = icase ? to_lo(needle[j]) : needle[j];
            if (a != b) break;
            j++;
        }
        if (j == nlen) return 1;
    }
    return 0;
}

static int search_fd(int fd, const char *label, const char *pat, int icase, int show_path, int show_lno) {
    char line[2048];
    u64 lpos = 0;
    long lno = 1;
    int hits = 0;
    char rbuf[512];
    for (;;) {
        long r = sys3(SYS_read, fd, (long)rbuf, sizeof(rbuf));
        if (r <= 0) {
            if (lpos > 0) {
                line[lpos] = 0;
                if (match_sub(line, pat, icase)) {
                    if (show_path && label) { m_puts(label); m_puts(":"); }
                    if (show_lno) { m_put_i64(1, lno); m_puts(":"); }
                    m_puts(line); m_puts("\n");
                    hits++;
                }
            }
            break;
        }
        for (long i = 0; i < r; i++) {
            char ch = rbuf[i];
            if (ch == '\n') {
                line[lpos] = 0;
                if (match_sub(line, pat, icase)) {
                    if (show_path && label) { m_puts(label); m_puts(":"); }
                    if (show_lno) { m_put_i64(1, lno); m_puts(":"); }
                    m_puts(line); m_puts("\n");
                    hits++;
                }
                lpos = 0;
                lno++;
            } else if (lpos + 1 < sizeof(line)) {
                line[lpos++] = ch;
            }
        }
    }
    return hits;
}

static void join_path(char *out, u64 cap, const char *dir, const char *name) {
    u64 dl = m_strlen(dir);
    u64 nl = m_strlen(name);
    u64 pos = 0;
    if (!(dl == 1 && dir[0] == '.')) {
        for (u64 i = 0; i < dl && pos + 1 < cap; i++) out[pos++] = dir[i];
        if (pos > 0 && out[pos - 1] != '/' && pos + 1 < cap) out[pos++] = '/';
    }
    for (u64 i = 0; i < nl && pos + 1 < cap; i++) out[pos++] = name[i];
    out[pos] = 0;
}

static int walk_target(const char *path, const char *pat, int icase, int show_lno, int depth) {
    if (depth > 8) return 0;
    long dfd = sys4(SYS_openat, AT_FDCWD, (long)path, O_RDONLY | O_DIRECTORY, 0);
    if (dfd >= 0) {
        u8 dbuf[1024];
        int total_hits = 0;
        for (;;) {
            long nread = sys3(SYS_getdents64, dfd, (long)dbuf, sizeof(dbuf));
            if (nread <= 0) break;
            long pos = 0;
            while (pos < nread) {
                struct m_linux_dirent64 *d = (struct m_linux_dirent64 *)(dbuf + pos);
                if (m_strcmp(d->d_name, ".") != 0 && m_strcmp(d->d_name, "..") != 0 && d->d_name[0] != '.') {
                    char child[512];
                    join_path(child, sizeof(child), path, d->d_name);
                    total_hits += walk_target(child, pat, icase, show_lno, depth + 1);
                }
                pos += d->d_reclen;
            }
        }
        sys1(SYS_close, dfd);
        return total_hits;
    }

    long ffd = sys4(SYS_openat, AT_FDCWD, (long)path, O_RDONLY, 0);
    if (ffd < 0) return 0;
    /* Skip ELF binaries */
    u8 hdr[4] = {0, 0, 0, 0};
    long hr = sys3(SYS_read, ffd, (long)hdr, 4);
    if (hr == 4 && hdr[0] == 0x7f && hdr[1] == 'E' && hdr[2] == 'L' && hdr[3] == 'F') {
        sys1(SYS_close, ffd);
        return 0;
    }
    sys3(SYS_lseek, ffd, 0, 0);
    int hits = search_fd((int)ffd, path, pat, icase, 1, show_lno);
    sys1(SYS_close, ffd);
    return hits;
}

int main(int argc, char **argv, char **envp) {
    (void)envp;
    int icase = 0;
    int show_lno = 1;
    const char *pat = 0;
    const char *target = ".";

    for (int i = 1; i < argc; i++) {
        if (m_strcmp(argv[i], "-i") == 0) icase = 1;
        else if (m_strcmp(argv[i], "-n") == 0) show_lno = 1;
        else if (m_strcmp(argv[i], "-N") == 0) show_lno = 0;
        else if (!pat) pat = argv[i];
        else target = argv[i];
    }
    if (!pat) {
        m_puts_fd(2, "rg: usage: rg [-i] [-n] <pattern> [path]\n");
        return 2;
    }
    if (m_strcmp(target, "-") == 0) {
        int h = search_fd(0, 0, pat, icase, 0, show_lno);
        return h > 0 ? 0 : 1;
    }
    int hits = walk_target(target, pat, icase, show_lno, 0);
    return hits > 0 ? 0 : 1;
}
