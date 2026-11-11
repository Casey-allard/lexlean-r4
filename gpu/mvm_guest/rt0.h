#ifndef UOR_MVM_RT0_H
#define UOR_MVM_RT0_H

typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
typedef unsigned long u64;
typedef long i64;

/* Linux riscv64 syscall numbers (asm-generic/unistd.h) */
#define SYS_getcwd        17
#define SYS_dup3          24
#define SYS_mkdirat       34
#define SYS_unlinkat      35
#define SYS_chdir         49
#define SYS_openat        56
#define SYS_close         57
#define SYS_pipe2         59
#define SYS_getdents64    61
#define SYS_lseek         62
#define SYS_read          63
#define SYS_write         64
#define SYS_fstatat       79
#define SYS_fstat         80
#define SYS_exit          93
#define SYS_exit_group    94
#define SYS_clock_gettime 113
#define SYS_uname         160
#define SYS_getpid        172
#define SYS_getuid        174
#define SYS_geteuid       175
#define SYS_socket        198
#define SYS_bind          200
#define SYS_listen        201
#define SYS_connect       203
#define SYS_sendto        206
#define SYS_recvfrom      207
#define SYS_brk           214
#define SYS_munmap        215
#define SYS_clone         220
#define SYS_execve        221
#define SYS_mmap          222
#define SYS_accept4       242
#define SYS_wait4         260

#define AT_FDCWD          (-100)
#define O_RDONLY          0
#define O_WRONLY          1
#define O_RDWR            2
#define O_CREAT           0100
#define O_TRUNC           01000
#define O_APPEND          02000
#define O_DIRECTORY       0200000

#define AF_INET           2
#define AF_INET6          10
#define AF_PACKET         17
#define AF_VSOCK          40
#define SOCK_STREAM       1

static inline long sys6(long n, long a0, long a1, long a2, long a3, long a4, long a5) {
    register long r0 __asm__("a0") = a0;
    register long r1 __asm__("a1") = a1;
    register long r2 __asm__("a2") = a2;
    register long r3 __asm__("a3") = a3;
    register long r4 __asm__("a4") = a4;
    register long r5 __asm__("a5") = a5;
    register long r7 __asm__("a7") = n;
    __asm__ volatile("ecall"
                     : "+r"(r0)
                     : "r"(r1), "r"(r2), "r"(r3), "r"(r4), "r"(r5), "r"(r7)
                     : "memory");
    return r0;
}

static inline long sys0(long n) { return sys6(n, 0, 0, 0, 0, 0, 0); }
static inline long sys1(long n, long a0) { return sys6(n, a0, 0, 0, 0, 0, 0); }
static inline long sys2(long n, long a0, long a1) { return sys6(n, a0, a1, 0, 0, 0, 0); }
static inline long sys3(long n, long a0, long a1, long a2) { return sys6(n, a0, a1, a2, 0, 0, 0); }
static inline long sys4(long n, long a0, long a1, long a2, long a3) { return sys6(n, a0, a1, a2, a3, 0, 0); }

static inline u64 m_strlen(const char *s) {
    u64 n = 0;
    while (s && s[n]) n++;
    return n;
}

static inline int m_strcmp(const char *a, const char *b) {
    while (*a && (*a == *b)) { a++; b++; }
    return (int)(u8)*a - (int)(u8)*b;
}

static inline int m_strncmp(const char *a, const char *b, u64 n) {
    for (u64 i = 0; i < n; i++) {
        if (a[i] != b[i]) return (int)(u8)a[i] - (int)(u8)b[i];
        if (a[i] == 0) return 0;
    }
    return 0;
}

static inline const char *m_strstr(const char *hay, const char *needle) {
    if (!*needle) return hay;
    u64 nlen = m_strlen(needle);
    for (; *hay; hay++) {
        if (*hay == *needle && m_strncmp(hay, needle, nlen) == 0) return hay;
    }
    return 0;
}

static inline void m_memcpy(void *dst, const void *src, u64 n) {
    u8 *d = (u8 *)dst;
    const u8 *s = (const u8 *)src;
    for (u64 i = 0; i < n; i++) d[i] = s[i];
}

static inline void m_memset(void *dst, u8 v, u64 n) {
    u8 *d = (u8 *)dst;
    for (u64 i = 0; i < n; i++) d[i] = v;
}

__attribute__((used)) void *memcpy(void *dst, const void *src, unsigned long n) {
    u8 *d = (u8 *)dst;
    const u8 *s = (const u8 *)src;
    for (unsigned long i = 0; i < n; i++) d[i] = s[i];
    return dst;
}

__attribute__((used)) void *memset(void *dst, int c, unsigned long n) {
    u8 *d = (u8 *)dst;
    for (unsigned long i = 0; i < n; i++) d[i] = (u8)c;
    return dst;
}

__attribute__((used)) void *memmove(void *dst, const void *src, unsigned long n) {
    u8 *d = (u8 *)dst;
    const u8 *s = (const u8 *)src;
    if (d < s) {
        for (unsigned long i = 0; i < n; i++) d[i] = s[i];
    } else if (d > s) {
        for (unsigned long i = n; i > 0; i--) d[i - 1] = s[i - 1];
    }
    return dst;
}

__attribute__((used)) int memcmp(const void *a, const void *b, unsigned long n) {
    const u8 *pa = (const u8 *)a;
    const u8 *pb = (const u8 *)b;
    for (unsigned long i = 0; i < n; i++) {
        if (pa[i] != pb[i]) return (int)pa[i] - (int)pb[i];
    }
    return 0;
}

static inline void m_write_fd(int fd, const void *buf, u64 len) {
    const u8 *p = (const u8 *)buf;
    u64 off = 0;
    while (off < len) {
        long w = sys3(SYS_write, fd, (long)(p + off), (long)(len - off));
        if (w <= 0) break;
        off += (u64)w;
    }
}

static inline void m_puts_fd(int fd, const char *s) {
    m_write_fd(fd, s, m_strlen(s));
}

static inline void m_puts(const char *s) {
    m_puts_fd(1, s);
}

static inline void m_put_i64(int fd, long v) {
    char buf[32];
    int pos = 31;
    buf[pos] = 0;
    unsigned long uv = (v < 0) ? (unsigned long)(-v) : (unsigned long)v;
    if (uv == 0) {
        buf[--pos] = '0';
    } else {
        while (uv > 0) {
            buf[--pos] = (char)('0' + (uv % 10));
            uv /= 10;
        }
    }
    if (v < 0) buf[--pos] = '-';
    m_puts_fd(fd, &buf[pos]);
}

struct m_utsname {
    char sysname[65];
    char nodename[65];
    char release[65];
    char version[65];
    char machine[65];
    char domainname[65];
};

struct m_sockaddr_vm {
    u16 svm_family;
    u16 svm_reserved1;
    u32 svm_port;
    u32 svm_cid;
    u8  svm_zero[4];
};

struct m_linux_dirent64 {
    u64 d_ino;
    i64 d_off;
    u16 d_reclen;
    u8  d_type;
    char d_name[];
};

int main(int argc, char **argv, char **envp);

__attribute__((noreturn)) void __mvm_entry(long *sp) {
    int argc = (int)sp[0];
    char **argv = (char **)&sp[1];
    char **envp = &argv[argc + 1];
    int rc = main(argc, argv, envp);
    sys1(SYS_exit_group, rc);
    __builtin_unreachable();
}

__asm__(
    ".section .text._start,\"ax\",@progbits\n"
    ".globl _start\n"
    ".type _start, @function\n"
    "_start:\n"
    "    mv a0, sp\n"
    "    andi sp, sp, -16\n"
    "    call __mvm_entry\n"
);

#endif
