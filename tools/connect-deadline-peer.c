// Independent Linux peer: a full accept queue keeps SYNs pending on loopback.
#define _GNU_SOURCE
#include <arpa/inet.h>
#include <errno.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

static void fail(const char *operation) { perror(operation); exit(1); }
int main(int argc, char **argv) {
    if (argc != 2) return 2;
    int hold = strcmp(argv[1], "timeout") == 0;
    if (!hold && strcmp(argv[1], "reply") != 0) return 2;
    alarm(120);
    int server = socket(AF_INET, SOCK_STREAM | SOCK_CLOEXEC, 0);
    if (server < 0) fail("socket");
    struct sockaddr_in address = { .sin_family = AF_INET, .sin_addr.s_addr = htonl(INADDR_LOOPBACK) };
    if (bind(server, (struct sockaddr *)&address, sizeof address) < 0) fail("bind");
    if (listen(server, hold ? 0 : 8) < 0) fail("listen");
    socklen_t length = sizeof address;
    if (getsockname(server, (struct sockaddr *)&address, &length) < 0) fail("getsockname");
    if (hold) {
        int filler = socket(AF_INET, SOCK_STREAM | SOCK_CLOEXEC, 0);
        if (filler < 0 || connect(filler, (struct sockaddr *)&address, length) < 0) fail("fill backlog");
        int probe = socket(AF_INET, SOCK_STREAM | SOCK_NONBLOCK | SOCK_CLOEXEC, 0);
        if (probe < 0) fail("probe socket");
        if (connect(probe, (struct sockaddr *)&address, length) != -1 || errno != EINPROGRESS) {
            fputs("loopback queue did not retain a pending connect\n", stderr); return 1;
        }
        struct pollfd pending = { .fd = probe, .events = POLLOUT };
        if (poll(&pending, 1, 50) != 0) {
            fputs("loopback queue did not remain pending for 50 ms\n", stderr); return 1;
        }
        close(probe);
        printf("%u\n", (unsigned)ntohs(address.sin_port)); fflush(stdout);
        for (;;) pause();
    }
    printf("%u\n", (unsigned)ntohs(address.sin_port)); fflush(stdout);
    for (;;) {
        int client = accept4(server, NULL, NULL, SOCK_CLOEXEC);
        if (client < 0) { if (errno == EINTR) continue; fail("accept"); }
        struct timespec remaining = { .tv_nsec = 30000000 };
        while (nanosleep(&remaining, &remaining) < 0) if (errno != EINTR) fail("nanosleep");
        if (write(client, "R", 1) != 1) fail("delayed reply");
        char byte;
        if (read(client, &byte, 1) != 1 || byte != 'Q') { fputs("wrong client request\n", stderr); return 1; }
        close(client);
    }
}
