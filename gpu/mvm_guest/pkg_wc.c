#include "rt0.h"

/*
 * Freestanding RV64IMAC word/line/byte counter (wc).
 * Supports -l, -w, -c combinations over stdin or multiple files.
 */

static u8 g_buf[8192];

struct wc_counts {
    u64 lines;
    u64 words;
    u64 bytes;
};

static int is_space(u8 c) {
    return c == ' ' || c == '\n' || c == '\t' || c == '\r' || c == '\v' || c == '\f';
}

static void count_fd(int fd, struct wc_counts *out) {
    out->lines = 0;
    out->words = 0;
    out->bytes = 0;
    int in_word = 0;
    while (1) {
        long n = sys3(SYS_read, fd, (long)g_buf, sizeof(g_buf));
        if (n <= 0) break;
        out->bytes += (u64)n;
        for (long i = 0; i < n; i++) {
            u8 c = g_buf[i];
            if (c == '\n') out->lines++;
            if (is_space(c)) {
                in_word = 0;
            } else if (!in_word) {
                in_word = 1;
                out->words++;
            }
        }
    }
}

static void print_counts(const struct wc_counts *c, int show_l, int show_w, int show_c, const char *label) {
    int first = 1;
    if (show_l) {
        m_put_i64(1, (long)c->lines);
        first = 0;
    }
    if (show_w) {
        if (!first) m_puts(" ");
        m_put_i64(1, (long)c->words);
        first = 0;
    }
    if (show_c) {
        if (!first) m_puts(" ");
        m_put_i64(1, (long)c->bytes);
        first = 0;
    }
    if (label) {
        m_puts(" ");
        m_puts(label);
    }
    m_puts("\n");
}

int main(int argc, char **argv, char **envp) {
    (void)envp;
    int show_l = 0, show_w = 0, show_c = 0;
    const char *files[64];
    int nfiles = 0;

    for (int i = 1; i < argc; i++) {
        const char *a = argv[i];
        if (a[0] == '-' && a[1] != 0) {
            for (int j = 1; a[j]; j++) {
                if (a[j] == 'l') show_l = 1;
                else if (a[j] == 'w') show_w = 1;
                else if (a[j] == 'c' || a[j] == 'm') show_c = 1;
            }
        } else if (nfiles < 64) {
            files[nfiles++] = a;
        }
    }

    if (!show_l && !show_w && !show_c) {
        show_l = 1;
        show_w = 1;
        show_c = 1;
    }

    if (nfiles == 0) {
        struct wc_counts c;
        count_fd(0, &c);
        print_counts(&c, show_l, show_w, show_c, 0);
        return 0;
    }

    struct wc_counts total = {0, 0, 0};
    int rc = 0;
    for (int i = 0; i < nfiles; i++) {
        struct wc_counts c;
        if (m_strcmp(files[i], "-") == 0) {
            count_fd(0, &c);
            print_counts(&c, show_l, show_w, show_c, "-");
        } else {
            long fd = sys4(SYS_openat, AT_FDCWD, (long)files[i], O_RDONLY, 0);
            if (fd < 0) {
                m_puts_fd(2, "wc: cannot open ");
                m_puts_fd(2, files[i]);
                m_puts_fd(2, "\n");
                rc = 1;
                continue;
            }
            count_fd((int)fd, &c);
            sys1(SYS_close, fd);
            print_counts(&c, show_l, show_w, show_c, files[i]);
        }
        total.lines += c.lines;
        total.words += c.words;
        total.bytes += c.bytes;
    }

    if (nfiles > 1) {
        print_counts(&total, show_l, show_w, show_c, "total");
    }
    return rc;
}
