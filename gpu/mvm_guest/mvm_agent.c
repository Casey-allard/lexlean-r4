#include "rt0.h"

int main(int argc, char **argv, char **envp) {
    (void)envp;
    if (argc >= 2 && m_strcmp(argv[1], "--handshake") == 0) {
        long sock = sys3(SYS_socket, AF_VSOCK, SOCK_STREAM, 0);
        if (sock < 0) {
            m_puts_fd(2, "mvm-agent: AF_VSOCK socket failed\n");
            return 1;
        }
        struct m_sockaddr_vm addr;
        m_memset(&addr, 0, sizeof(addr));
        addr.svm_family = AF_VSOCK;
        addr.svm_cid = 2; /* Host hypervisor / SpiralCore switch CID */
        addr.svm_port = 1024;
        long rc = sys3(SYS_connect, sock, (long)&addr, sizeof(addr));
        if (rc < 0) {
            m_puts_fd(2, "mvm-agent: connect CID 2:1024 failed\n");
            sys1(SYS_close, sock);
            return 2;
        }
        const char *hello = "{\"op\":\"mvm_agent_hello\",\"guest_cid\":3,\"arch\":\"riscv64\"}";
        sys6(SYS_sendto, sock, (long)hello, (long)m_strlen(hello), 0, 0, 0);
        char reply[256];
        m_memset(reply, 0, sizeof(reply));
        long n = sys6(SYS_recvfrom, sock, (long)reply, 255, 0, 0, 0);
        sys1(SYS_close, sock);
        m_puts("MVM_AGENT_ONLINE cid=3 host_cid=2 port=1024 ack=");
        if (n > 0) {
            m_write_fd(1, reply, (u64)n);
        } else {
            m_puts("OK");
        }
        m_puts("\n");
        return 0;
    }

    if (argc >= 3 && m_strcmp(argv[1], "--exec") == 0) {
        char *sh_argv[4];
        sh_argv[0] = "/bin/sh";
        sh_argv[1] = "-c";
        sh_argv[2] = argv[2];
        sh_argv[3] = 0;
        long pid = sys4(SYS_clone, 17, 0, 0, 0);
        if (pid == 0) {
            sys3(SYS_execve, (long)"/bin/mvmbox", (long)sh_argv, (long)envp);
            sys1(SYS_exit_group, 127);
        }
        int status = 0;
        sys4(SYS_wait4, pid, (long)&status, 0, 0);
        return (status >> 8) & 0xff;
    }

    m_puts("mvm-agent: RV64IMAC guest init ready\n");
    return 0;
}
