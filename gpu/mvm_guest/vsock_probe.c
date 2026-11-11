#include "rt0.h"

int main(int argc, char **argv, char **envp) {
    (void)envp;
    if (argc >= 2 && m_strcmp(argv[1], "--try-inet") == 0) {
        long r_inet = sys3(SYS_socket, AF_INET, SOCK_STREAM, 0);
        long r_inet6 = sys3(SYS_socket, AF_INET6, SOCK_STREAM, 0);
        long r_pkt = sys3(SYS_socket, AF_PACKET, SOCK_STREAM, 0);
        m_puts("AF_INET_DENIED=");
        m_put_i64(1, r_inet);
        m_puts(" AF_INET6_DENIED=");
        m_put_i64(1, r_inet6);
        m_puts(" AF_PACKET_DENIED=");
        m_put_i64(1, r_pkt);
        m_puts("\n");
        return 0;
    }

    if (argc >= 4 && m_strcmp(argv[1], "--send") == 0) {
        u32 cid = 0;
        for (const char *p = argv[2]; *p >= '0' && *p <= '9'; p++) {
            cid = cid * 10 + (u32)(*p - '0');
        }
        u32 port = 0;
        for (const char *p = argv[3]; *p >= '0' && *p <= '9'; p++) {
            port = port * 10 + (u32)(*p - '0');
        }
        const char *msg = (argc >= 5) ? argv[4] : "PING";
        long sock = sys3(SYS_socket, AF_VSOCK, SOCK_STREAM, 0);
        if (sock < 0) return 1;
        struct m_sockaddr_vm addr;
        m_memset(&addr, 0, sizeof(addr));
        addr.svm_family = AF_VSOCK;
        addr.svm_cid = cid;
        addr.svm_port = port;
        if (sys3(SYS_connect, sock, (long)&addr, sizeof(addr)) < 0) {
            sys1(SYS_close, sock);
            return 2;
        }
        sys6(SYS_sendto, sock, (long)msg, (long)m_strlen(msg), 0, 0, 0);
        char buf[512];
        m_memset(buf, 0, sizeof(buf));
        long n = sys6(SYS_recvfrom, sock, (long)buf, 511, 0, 0, 0);
        sys1(SYS_close, sock);
        if (n > 0) {
            m_write_fd(1, buf, (u64)n);
            m_puts("\n");
        }
        return 0;
    }

    m_puts("vsock_probe: usage --try-inet | --send <cid> <port> <msg>\n");
    return 0;
}
