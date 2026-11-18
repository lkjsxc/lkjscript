#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <poll.h>
#include <netinet/in.h>
#include <sys/ptrace.h>
#include <sys/user.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/prctl.h>
#include <sys/resource.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

static pid_t child;
static void die(const char *message) {
    fprintf(stderr, "Unix witness: %s (errno=%d %s)\n", message, errno, strerror(errno));
    if (child > 0) { kill(-child, SIGKILL); kill(child, SIGKILL); while (waitpid(child, NULL, 0) < 0 && errno == EINTR) {} }
    exit(1);
}
static void demand(int condition, const char *message) { if (!condition) die(message); }
static void interrupted(int signal_number) {
    if (child > 0) { kill(-child, SIGKILL); kill(child, SIGKILL); }
    _exit(128 + signal_number);
}
static void pause_ms(unsigned ms) {
    struct timespec delay = {(time_t)(ms / 1000), (long)(ms % 1000) * 1000000};
    while (nanosleep(&delay, &delay) < 0 && errno == EINTR) {}
}
static void write_all(int fd, const void *bytes, size_t size) {
    const unsigned char *p = bytes;
    while (size) { ssize_t n = send(fd, p, size, MSG_NOSIGNAL); if (n < 0 && errno == EINTR) continue; demand(n > 0, "send bytes"); p += n; size -= (size_t)n; }
}
static int accept_consumer(int listener) {
    for (;;) {
        struct pollfd readable = {listener, POLLIN, 0}; int ready = poll(&readable, 1, 100);
        if (ready < 0 && errno == EINTR) continue;
        demand(ready >= 0, "poll consumer connection");
        if (ready) { int fd = accept4(listener, NULL, NULL, SOCK_CLOEXEC); demand(fd >= 0, "accept consumer"); return fd; }
        int status; pid_t ended = waitpid(child, &status, WNOHANG);
        demand(ended == 0, "consumer exited before connecting");
    }
}
static size_t packet(unsigned char *p, uint32_t sender, uint32_t opcode, const uint32_t *fields, size_t count) {
    uint32_t words[32]; demand(count < 30, "packet capacity");
    words[0] = sender; words[1] = (uint32_t)((count * 4 + 8) << 16) | opcode;
    memcpy(words + 2, fields, count * 4); memcpy(p, words, (count + 2) * 4); return (count + 2) * 4;
}
static size_t take(int fd, unsigned char *bytes, size_t size, int *owners, size_t *owner_count) {
    union { struct cmsghdr align; unsigned char bytes[CMSG_SPACE(32 * sizeof(int))]; } control;
    struct iovec iov = {bytes, size}; struct msghdr msg = {0};
    msg.msg_iov = &iov; msg.msg_iovlen = 1; msg.msg_control = control.bytes; msg.msg_controllen = sizeof control.bytes;
    ssize_t n; do { n = recvmsg(fd, &msg, MSG_CMSG_CLOEXEC); } while (n < 0 && errno == EINTR);
    demand(n > 0, "receive expected bytes"); demand(!(msg.msg_flags & (MSG_CTRUNC | MSG_TRUNC)), "witness receive truncated");
    for (struct cmsghdr *c = CMSG_FIRSTHDR(&msg); c; c = CMSG_NXTHDR(&msg, c)) {
        demand(c->cmsg_level == SOL_SOCKET && c->cmsg_type == SCM_RIGHTS, "unexpected control message");
        size_t count = (c->cmsg_len - CMSG_LEN(0)) / sizeof(int); demand(*owner_count + count <= 32, "too many rights");
        memcpy(owners + *owner_count, CMSG_DATA(c), count * sizeof(int)); *owner_count += count;
    }
    return (size_t)n;
}
static void exact(int fd, const void *expected, size_t size, int *owners, size_t *owner_count) {
    const unsigned char *p = expected; unsigned char bytes[4096];
    while (size) { size_t n = take(fd, bytes, size < sizeof bytes ? size : sizeof bytes, owners, owner_count); demand(!memcmp(bytes, p, n), "raw wire bytes differ"); p += n; size -= n; }
}
static void no_rights(int fd, const void *bytes, size_t size) {
    int owners[32]; size_t count = 0; exact(fd, bytes, size, owners, &count); demand(count == 0, "unexpected repeated descriptor rights");
}
static void send_rights(int fd, const void *bytes, size_t size, int owner, size_t count) {
    union { struct cmsghdr align; unsigned char bytes[CMSG_SPACE(4 * sizeof(int))]; } control;
    demand(count <= 4, "send rights capacity"); memset(&control, 0, sizeof control);
    struct iovec iov = {(void *)bytes, size}; struct msghdr msg = {0}; msg.msg_iov = &iov; msg.msg_iovlen = 1;
    msg.msg_control = control.bytes; msg.msg_controllen = CMSG_SPACE(count * sizeof(int));
    struct cmsghdr *c = CMSG_FIRSTHDR(&msg); c->cmsg_len = CMSG_LEN(count * sizeof(int)); c->cmsg_level = SOL_SOCKET; c->cmsg_type = SCM_RIGHTS;
    for (size_t i = 0; i < count; ++i) memcpy(CMSG_DATA(c) + i * sizeof(int), &owner, sizeof owner);
    ssize_t n; do { n = sendmsg(fd, &msg, MSG_NOSIGNAL); } while (n < 0 && errno == EINTR);
    demand(n > 0, "send owned descriptor"); write_all(fd, (const unsigned char *)bytes + n, size - (size_t)n);
}
static unsigned fd_count(pid_t pid) {
    char path[64]; snprintf(path, sizeof path, "/proc/%ld/fd", (long)pid); DIR *d = opendir(path); demand(d != NULL, "open peer descriptor census");
    unsigned count = 0; struct dirent *entry; while ((entry = readdir(d))) if (entry->d_name[0] != '.') ++count;
    closedir(d); return count;
}
static unsigned inode_count(pid_t pid, const struct stat *identity, int cloexec) {
    char path[128]; snprintf(path, sizeof path, "/proc/%ld/fd", (long)pid); DIR *d = opendir(path); demand(d != NULL, "open peer descriptor identity census");
    unsigned found = 0; struct dirent *entry;
    while ((entry = readdir(d))) {
        char *end; long number = strtol(entry->d_name, &end, 10); if (*end || number < 0) continue;
        snprintf(path, sizeof path, "/proc/%ld/fd/%ld", (long)pid, number); struct stat st;
        if (stat(path, &st) || st.st_dev != identity->st_dev || st.st_ino != identity->st_ino) continue;
        ++found;
        if (cloexec) {
            snprintf(path, sizeof path, "/proc/%ld/fdinfo/%ld", (long)pid, number); FILE *f = fopen(path, "r"); demand(f != NULL, "open peer fdinfo");
            char line[128]; unsigned long flags = 0; int got = 0;
            while (fgets(line, sizeof line, f)) if (sscanf(line, "flags: %lo", &flags) == 1) got = 1;
            fclose(f); demand(got && (flags & O_CLOEXEC), "received owner lacks CLOEXEC"); demand(number >= 3, "received owner aliases standard stream sentinel");
        }
    }
    closedir(d); return found;
}
static void frame_contents(int fd) {
    unsigned char bytes[4]; demand(pread(fd, bytes, sizeof bytes, 0) == 4, "frame bytes readable");
    demand(!memcmp(bytes, "\x11\x22\x33\xff", 4), "frame contents differ");
}
static void supervision(void) {
    demand(prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) == 0, "set subreaper");
    signal(SIGPIPE, SIG_IGN); signal(SIGALRM, interrupted); signal(SIGTERM, interrupted); signal(SIGINT, interrupted); alarm(50);
}
static int unix_listener(const char *path) {
    int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0); demand(fd >= 0, "replacement listener");
    struct sockaddr_un address = {0}; address.sun_family = AF_UNIX;
    demand(strlen(path) < sizeof address.sun_path, "replacement pathname length"); memcpy(address.sun_path, path, strlen(path) + 1);
    demand(bind(fd, (struct sockaddr *)&address, sizeof address) == 0 && listen(fd, 8) == 0, "replacement listener bind"); return fd;
}
struct traced { pid_t pid; int pending; int alive; };
static struct traced *tracee(struct traced *states, size_t *count, pid_t pid) {
    for (size_t i = 0; i < *count; ++i) if (states[i].pid == pid) return states + i;
    demand(*count < 64, "traced child limit"); struct traced *state = states + (*count)++;
    *state = (struct traced){pid, 0, 1}; return state;
}
static unsigned long trace_word(pid_t pid, unsigned long address) {
    errno = 0; unsigned long value = (unsigned long)ptrace(PTRACE_PEEKDATA, pid, (void *)address, NULL);
    demand(errno == 0, "read owned tracee syscall argument"); return value;
}
static int replacement(const char *root, char **command) {
    supervision(); char socket_path[PATH_MAX], held[PATH_MAX], outside[PATH_MAX];
    snprintf(socket_path, sizeof socket_path, "%s/socket", root); snprintf(held, sizeof held, "%s/held-socket", root); snprintf(outside, sizeof outside, "%s-outside", root);
    int inside = unix_listener(socket_path), escape = unix_listener(outside);
    child = fork(); demand(child >= 0, "fork traced consumer");
    if (!child) {
        setpgid(0, 0); if (ptrace(PTRACE_TRACEME, 0, NULL, NULL) < 0) _exit(125);
        raise(SIGSTOP); execvp(command[0], command); _exit(126);
    }
    setpgid(child, child); int status; demand(waitpid(child, &status, 0) == child && WIFSTOPPED(status), "owned tracer initial stop");
    unsigned long options = PTRACE_O_TRACESYSGOOD | PTRACE_O_TRACEFORK | PTRACE_O_TRACEVFORK | PTRACE_O_TRACECLONE | PTRACE_O_EXITKILL;
    demand(ptrace(PTRACE_SETOPTIONS, child, NULL, (void *)options) == 0, "owned tracer options");
    struct traced states[64]; size_t count = 0; tracee(states, &count, child);
    demand(ptrace(PTRACE_SYSCALL, child, NULL, NULL) == 0, "start owned syscall trace"); int replaced = 0, root_ok = 0;
    for (;;) {
        pid_t pid = waitpid(-1, &status, __WALL);
        if (pid < 0 && errno == EINTR) continue;
        if (pid < 0 && errno == ECHILD) break;
        demand(pid > 0, "wait owned syscall trace"); struct traced *state = tracee(states, &count, pid);
        if (WIFEXITED(status) || WIFSIGNALED(status)) {
            demand(WIFEXITED(status) && WEXITSTATUS(status) == 0, "traced consumer/broker failed"); state->alive = 0;
            if (pid == child) { root_ok = 1; child = 0; } continue;
        }
        demand(WIFSTOPPED(status), "owned trace stop kind"); int signal_number = WSTOPSIG(status);
        if (signal_number == (SIGTRAP | 0x80)) {
            struct user_regs_struct regs; demand(ptrace(PTRACE_GETREGS, pid, NULL, &regs) == 0, "read owned syscall registers");
            if (regs.orig_rax == 437) {
                if ((long)regs.rax == -ENOSYS) {
                    unsigned long path = trace_word(pid, regs.rsi), flags = trace_word(pid, regs.rdx);
                    state->pending = (path & 0x00ffffffffffffffUL) == 0x0074656b636f73UL && (flags & O_PATH) != 0;
                } else if (state->pending) {
                    state->pending = 0; demand((long)regs.rax >= 0, "Directory retained socket open succeeded");
                    demand(!replaced, "replacement applied once"); struct stat st;
                    demand(lstat(socket_path, &st) == 0 && S_ISSOCK(st.st_mode), "held pathname names original socket");
                    demand(rename(socket_path, held) == 0 && symlink(outside, socket_path) == 0, "replace resolved leaf with escaping symlink"); replaced = 1;
                }
            }
            signal_number = 0;
        } else if (signal_number == SIGTRAP || signal_number == SIGSTOP) {
            unsigned event = (unsigned)status >> 16;
            if (event == PTRACE_EVENT_FORK || event == PTRACE_EVENT_VFORK || event == PTRACE_EVENT_CLONE) {
                unsigned long next; demand(ptrace(PTRACE_GETEVENTMSG, pid, NULL, &next) == 0, "owned trace descendant"); tracee(states, &count, (pid_t)next);
            }
            signal_number = 0;
        }
        demand(ptrace(PTRACE_SYSCALL, pid, NULL, (void *)(long)signal_number) == 0, "resume owned tracee");
    }
    demand(root_ok && replaced, "runtime actually crossed pinned-path replacement boundary");
    int accepted = accept4(inside, NULL, NULL, SOCK_CLOEXEC); demand(accepted >= 0, "pinned inside socket received connection"); no_rights(accepted, "pinned", 6); close(accepted);
    int wrong = accept4(escape, NULL, NULL, SOCK_CLOEXEC); demand(wrong < 0 && errno == EAGAIN, "escaping replacement received no connection");
    close(inside); close(escape); demand(unlink(socket_path) == 0 && unlink(held) == 0 && unlink(outside) == 0, "replacement cleanup");
    fprintf(stderr, "ok Unix runtime Directory proof: socket pathname replaced by outside symlink after O_PATH success; pinned original received bytes\n"); return 0;
}
static void no_owned_inode_tree(pid_t pid, const struct stat *identity, unsigned depth) {
    demand(depth < 8, "consumer descendant depth"); demand(inode_count(pid, identity, 0) == 0, "socket-kind rejection leaked File owner");
    char path[128]; snprintf(path, sizeof path, "/proc/%ld/task/%ld/children", (long)pid, (long)pid); FILE *f = fopen(path, "r"); demand(f != NULL, "consumer children census");
    long descendant; while (fscanf(f, "%ld", &descendant) == 1) no_owned_inode_tree((pid_t)descendant, identity, depth + 1); fclose(f);
}
static int wrong_socket(const char *root, char **command) {
    supervision(); int listener = socket(AF_INET, SOCK_STREAM | SOCK_CLOEXEC, 0); demand(listener >= 0, "TCP witness socket");
    struct sockaddr_in address = {0}; address.sin_family = AF_INET; address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    demand(bind(listener, (struct sockaddr *)&address, sizeof address) == 0 && listen(listener, 4) == 0, "TCP witness bind");
    socklen_t length = sizeof address; demand(getsockname(listener, (struct sockaddr *)&address, &length) == 0, "TCP witness endpoint");
    char grant[80]; snprintf(grant, sizeof grant, "target=127.0.0.1:%u", (unsigned)ntohs(address.sin_port));
    for (size_t i = 0; command[i]; ++i) if (!strcmp(command[i], "@TCP@")) command[i] = grant;
    child = fork(); demand(child >= 0, "fork TCP refusal consumer");
    if (!child) { setpgid(0, 0); execvp(command[0], command); _exit(126); }
    setpgid(child, child); int fd = accept_consumer(listener); close(listener);
    char path[PATH_MAX]; snprintf(path, sizeof path, "%s/tcp-ready", root); unsigned attempts = 0;
    while (access(path, F_OK) && attempts++ < 5000) { pause_ms(1); }
    demand(access(path, F_OK) == 0, "TCP rejection marker");
    snprintf(path, sizeof path, "%s/regular", root); struct stat identity; demand(stat(path, &identity) == 0, "TCP rejection File identity"); no_owned_inode_tree(child, &identity, 0);
    unsigned char byte; demand(recv(fd, &byte, 1, MSG_DONTWAIT) < 0 && errno == EAGAIN, "TCP refusal transmitted no bytes/rights"); write_all(fd, "d", 1); close(fd);
    int status; while (waitpid(child, &status, 0) < 0) demand(errno == EINTR, "wait TCP consumer"); child = 0;
    demand(WIFEXITED(status) && WEXITSTATUS(status) == 0, "TCP refusal consumer failed");
    while (waitpid(-1, &status, 0) > 0) demand(WIFEXITED(status) && WEXITSTATUS(status) == 0, "TCP refusal broker failed");
    fprintf(stderr, "ok Unix methods refuse TCP before transfer, close consumed owner, preserve receive bytes; ordinary TCP read still works\n"); return 0;
}
int main(int argc, char **argv) {
    if (argc >= 5 && !strcmp(argv[1], "--replacement") && !strcmp(argv[3], "--")) return replacement(argv[2], argv + 4);
    if (argc >= 5 && !strcmp(argv[1], "--tcp") && !strcmp(argv[3], "--")) return wrong_socket(argv[2], argv + 4);
    demand(argc >= 4, "usage: witness ROOT [--close-stdin] -- COMMAND...");
    int command = 2, closed = 0; if (!strcmp(argv[command], "--close-stdin")) { closed = 1; ++command; }
    demand(!strcmp(argv[command++], "--") && command < argc, "missing command separator");
    demand(prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) == 0, "set subreaper"); signal(SIGPIPE, SIG_IGN); signal(SIGALRM, interrupted); signal(SIGTERM, interrupted); signal(SIGINT, interrupted); alarm(50);
    char path[PATH_MAX]; demand(snprintf(path, sizeof path, "%s/socket", argv[1]) > 0, "socket path");
    demand(strlen(path) < sizeof(((struct sockaddr_un *)0)->sun_path), "socket path length");
    int listener = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0); demand(listener >= 0, "listener socket");
    struct sockaddr_un address = {0}; address.sun_family = AF_UNIX; memcpy(address.sun_path, path, strlen(path) + 1); unlink(path);
    demand(bind(listener, (struct sockaddr *)&address, sizeof address) == 0 && listen(listener, 8) == 0, "bind/listen");
    demand(setenv("LKJSCRIPT_UNIX_TEST_SOCKET", path, 1) == 0 && setenv("LKJSCRIPT_UNIX_TEST_ROOT", argv[1], 1) == 0, "set endpoint grants");
    child = fork(); demand(child >= 0, "fork consumer");
    if (!child) {
        setpgid(0, 0); struct rlimit limit = {64, 64}; if (setrlimit(RLIMIT_NOFILE, &limit) < 0) _exit(125);
        if (closed) { close(STDIN_FILENO); }
        execvp(argv[command], argv + command); _exit(126);
    }
    setpgid(child, child); int fd = accept_consumer(listener); close(listener);
    struct ucred credentials; socklen_t credential_size = sizeof credentials;
    demand(getsockopt(fd, SOL_SOCKET, SO_PEERCRED, &credentials, &credential_size) == 0, "peer credentials");
    unsigned char bytes[256]; int owners[32]; size_t count, size;
    const uint32_t registry[] = {2}; size = packet(bytes, 1, 1, registry, 1); no_rights(fd, bytes, size);
    const uint32_t shm[] = {7, 7, 0x735f6c77, 0x00006d68, 1}; size = packet(bytes, 2, 0, shm, 5);
    write_all(fd, bytes, 5); pause_ms(2); write_all(fd, bytes + 5, size - 5);
    const uint32_t bind_shm[] = {7, 7, 0x735f6c77, 0x00006d68, 1, 3}; size = packet(bytes, 2, 0, bind_shm, 6); no_rights(fd, bytes, size);
    const uint32_t seat[] = {8, 8, 0x735f6c77, 0x00746165, 1}; size = packet(bytes, 2, 0, seat, 5); write_all(fd, bytes, size);
    const uint32_t bind_seat[] = {8, 8, 0x735f6c77, 0x00746165, 1, 4}; size = packet(bytes, 2, 0, bind_seat, 6); no_rights(fd, bytes, size);
    const uint32_t keyboard[] = {5}; size = packet(bytes, 4, 1, keyboard, 1); no_rights(fd, bytes, size);
    int keymap = memfd_create("unix-keymap-witness", MFD_CLOEXEC); demand(keymap >= 0, "keymap memfd"); demand(write(keymap, "key\0", 4) == 4 && lseek(keymap, 0, SEEK_SET) == 0, "keymap contents");
    struct stat key_identity; demand(fstat(keymap, &key_identity) == 0, "keymap identity");
    const uint32_t key_fields[] = {1, 4}; size = packet(bytes, 5, 0, key_fields, 2); send_rights(fd, bytes, size, keymap, 1);
    no_rights(fd, "K", 1); demand(inode_count(credentials.pid, &key_identity, 1) == 1, "keymap has exactly one owned File"); write_all(fd, "C", 1);
    const uint32_t pool[] = {6, 4}; size = packet(bytes, 3, 0, pool, 2); count = 0; exact(fd, bytes, size, owners, &count); demand(count == 1, "create_pool carries one descriptor");
    frame_contents(owners[0]); struct stat frame_identity; demand(fstat(owners[0], &frame_identity) == 0, "frame identity"); close(owners[0]); write_all(fd, "R", 1);
    no_rights(fd, "u", 1); int datagrams[2];
    demand(socketpair(AF_UNIX, SOCK_DGRAM | SOCK_CLOEXEC, 0, datagrams) == 0, "unknown File socket kind");
    send_rights(fd, "U", 1, datagrams[0], 1); close(datagrams[0]); no_rights(fd, "D", 1);
    unsigned char kind_byte; demand(recv(datagrams[1], &kind_byte, 1, MSG_DONTWAIT) < 0 && errno == EAGAIN, "datagram refusal transmitted no data"); close(datagrams[1]);
    demand(inode_count(credentials.pid, &frame_identity, 0) == 0, "datagram refusal leaked transferred File");
    count = 0; exact(fd, "n", 1, owners, &count); demand(count == 1, "explicit socket tuning owner");
    demand((fcntl(owners[0], F_GETFL) & O_NONBLOCK) != 0, "Unix connection is nonblocking");
    int send_buffer = 8192; demand(setsockopt(owners[0], SOL_SOCKET, SO_SNDBUF, &send_buffer, sizeof send_buffer) == 0, "bound sender buffer"); close(owners[0]); write_all(fd, "N", 1);
    no_rights(fd, "p", 1); write_all(fd, "go", 2);
    struct pollfd readable = {fd, POLLIN, 0}; demand(poll(&readable, 1, 5000) == 1, "partial-send bytes become readable"); pause_ms(20);
    unsigned char block[4096]; memset(block, 0x5a, sizeof block); count = 0;
    for (unsigned i = 0; i < 16; ++i) exact(fd, block, sizeof block, owners, &count);
    demand(count == 1, "partial-send rights delivered once"); frame_contents(owners[0]); close(owners[0]); write_all(fd, "Q", 1);
    no_rights(fd, "b", 1); snprintf(path, sizeof path, "%s/backpressure-ready", argv[1]);
    unsigned attempts = 0; while (access(path, F_OK) && attempts++ < 5000) pause_ms(1); demand(access(path, F_OK) == 0, "sender reached timeout under backpressure");
    demand(inode_count(credentials.pid, &frame_identity, 0) == 0, "failed send leaked broker/native owner");
    if (credentials.pid != child) demand(inode_count(child, &frame_identity, 0) == 0, "failed send leaked interpreter local owner");
    int end = 0; size_t drained = 0; count = 0;
    while (!end) {
        size_t n = take(fd, block, sizeof block, owners, &count);
        for (size_t i = 0; i < n; ++i) { if (block[i] == 1) { demand(i + 1 == n, "backpressure marker ends bytes"); end = 1; } else { demand(block[i] == 0x7f, "failed send added bytes"); ++drained; } }
    }
    demand(drained > 0 && count == 0, "failed send transferred no rights"); write_all(fd, "B", 1);
    no_rights(fd, "t", 1);
    demand(inode_count(credentials.pid, &frame_identity, 0) == 0, "empty/error sends leaked owned Files");
    demand(inode_count(credentials.pid, &key_identity, 0) == 0, "received keymap did not close on drop");
    unsigned baseline = fd_count(credentials.pid);
    for (unsigned round = 0; round < 144; ++round) {
        send_rights(fd, "x", 1, keymap, round % 3 == 0 ? 2 : 3); no_rights(fd, "a", 1);
        demand(fd_count(credentials.pid) == baseline, "truncated receive leaked descriptor");
    }
    write_all(fd, "done", 4); demand(shutdown(fd, SHUT_RDWR) == 0, "peer EOF"); close(keymap); close(fd);
    int status; while (waitpid(child, &status, 0) < 0) demand(errno == EINTR, "wait consumer"); child = 0;
    demand(WIFEXITED(status) && WEXITSTATUS(status) == 0, "consumer failed");
    while (waitpid(-1, &status, 0) > 0) demand(WIFEXITED(status) && WEXITSTATUS(status) == 0, "broker failed");
    snprintf(path, sizeof path, "%s/socket", argv[1]); demand(unlink(path) == 0, "remove socket");
    fprintf(stderr, "ok Unix witness: raw Wayland registry/binds/keymap/create_pool, partial transfer once, timeout ownership, 144 bounded truncation cleanups%s\n", closed ? ", closed stdin" : "");
    return 0;
}
