#include "rt0.h"

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

static int hash_fd(int fd, const char *label) {
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
    for (int i = 0; i < 8; i++) buf[56 + i] = (u8)(bits >> (56 - i * 8));
    sha256_block(st, buf);

    const char *hex = "0123456789abcdef";
    char out[66];
    for (int i = 0; i < 8; i++) {
        u32 v = st[i];
        for (int j = 0; j < 8; j++) out[i * 8 + j] = hex[(v >> (28 - j * 4)) & 0xf];
    }
    out[64] = ' ';
    out[65] = ' ';
    m_write_fd(1, out, 66);
    m_puts(label);
    m_puts("\n");
    return 0;
}

int main(int argc, char **argv, char **envp) {
    (void)envp;
    if (argc < 2) return hash_fd(0, "-");
    int rc = 0;
    for (int i = 1; i < argc; i++) {
        if (m_strcmp(argv[i], "-") == 0) {
            rc |= hash_fd(0, "-");
            continue;
        }
        long fd = sys4(SYS_openat, AT_FDCWD, (long)argv[i], O_RDONLY, 0);
        if (fd < 0) {
            m_puts_fd(2, "sha256sum: cannot open ");
            m_puts_fd(2, argv[i]);
            m_puts_fd(2, "\n");
            rc = 1;
            continue;
        }
        rc |= hash_fd((int)fd, argv[i]);
        sys1(SYS_close, fd);
    }
    return rc;
}
