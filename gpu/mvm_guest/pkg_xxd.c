#include "rt0.h"

int main(int argc, char **argv, char **envp) {
    (void)envp;
    u64 max_len = 0;
    int has_limit = 0;
    const char *path = 0;

    for (int i = 1; i < argc; i++) {
        if (m_strcmp(argv[i], "-l") == 0 && i + 1 < argc) {
            u64 v = 0;
            for (const char *p = argv[++i]; *p >= '0' && *p <= '9'; p++) v = v * 10 + (u64)(*p - '0');
            max_len = v;
            has_limit = 1;
        } else if (argv[i][0] != '-') {
            path = argv[i];
        }
    }

    int fd = 0;
    if (path) {
        long ofd = sys4(SYS_openat, AT_FDCWD, (long)path, O_RDONLY, 0);
        if (ofd < 0) {
            m_puts_fd(2, "xxd: cannot open file\n");
            return 1;
        }
        fd = (int)ofd;
    }

    const char *hex = "0123456789abcdef";
    u64 offset = 0;
    u8 row[16];

    for (;;) {
        u64 want = 16;
        if (has_limit) {
            if (offset >= max_len) break;
            if (max_len - offset < want) want = max_len - offset;
        }
        u64 got = 0;
        while (got < want) {
            long r = sys3(SYS_read, fd, (long)(row + got), (long)(want - got));
            if (r <= 0) break;
            got += (u64)r;
        }
        if (got == 0) break;

        char line[80];
        int pos = 0;
        for (int s = 28; s >= 0; s -= 4) {
            line[pos++] = hex[(offset >> s) & 0xf];
        }
        line[pos++] = ':';
        line[pos++] = ' ';

        for (u64 i = 0; i < 16; i++) {
            if (i < got) {
                line[pos++] = hex[(row[i] >> 4) & 0xf];
                line[pos++] = hex[row[i] & 0xf];
            } else {
                line[pos++] = ' ';
                line[pos++] = ' ';
            }
            if ((i & 1) == 1) line[pos++] = ' ';
        }
        line[pos++] = ' ';
        for (u64 i = 0; i < got; i++) {
            line[pos++] = (row[i] >= 32 && row[i] <= 126) ? (char)row[i] : '.';
        }
        line[pos++] = '\n';
        m_write_fd(1, line, (u64)pos);
        offset += got;
        if (got < want) break;
    }

    if (fd != 0) sys1(SYS_close, fd);
    return 0;
}
