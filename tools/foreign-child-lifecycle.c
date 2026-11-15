#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include <poll.h>
#include <signal.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/resource.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

/* Test supervisor only. Restrictions apply to this process immediately before
 * it execs a requested fixture; lifecycle observations use only its descendants. */
static void die(const char *what) { perror(what); exit(1); }
static long long now(void) {
    struct timespec value;
    if (clock_gettime(CLOCK_MONOTONIC, &value)) die("clock");
    return value.tv_sec * 1000000000LL + value.tv_nsec;
}
static void pause_briefly(void) {
    struct timespec delay = {.tv_sec = 0, .tv_nsec = 10000000};
    nanosleep(&delay, NULL);
}
static int waiting(pid_t pid) {
    char path[80], bytes[256];
    snprintf(path, sizeof(path), "/proc/%d/cmdline", pid);
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    ssize_t count = read(fd, bytes, sizeof(bytes));
    close(fd);
    if (count <= 0) return 0;
    for (ssize_t at = 0; at + 13 <= count; ++at)
        if ((at == 0 || bytes[at - 1] == 0) && !memcmp(bytes + at, "--wait-owner", 13)) return 1;
    return 0;
}
static pid_t descendant(pid_t parent) {
    char path[100];
    snprintf(path, sizeof(path), "/proc/%d/task/%d/children", parent, parent);
    FILE *file = fopen(path, "re");
    if (!file) return 0;
    pid_t child, found = 0;
    while (fscanf(file, "%d", &child) == 1) {
        if (waiting(child)) { found = child; break; }
        found = descendant(child);
        if (found) break;
    }
    fclose(file);
    return found;
}
static void signal_fd(int fd) {
    if (fd >= 0 && syscall(SYS_pidfd_send_signal, fd, SIGKILL, NULL, 0) && errno != ESRCH)
        die("signal owned pidfd");
}
static void cleanup_adopted(void) {
    char path[100];
    snprintf(path, sizeof(path), "/proc/%d/task/%d/children", getpid(), getpid());
    FILE *file = fopen(path, "re");
    if (file) {
        pid_t child;
        while (fscanf(file, "%d", &child) == 1) {
            int fd = (int)syscall(SYS_pidfd_open, child, 0);
            if (fd >= 0) { signal_fd(fd); close(fd); }
        }
        fclose(file);
    }
    while (waitpid(-1, NULL, 0) > 0 || errno == EINTR) {}
}
static int parent_death(char **command) {
    if (prctl(PR_SET_CHILD_SUBREAPER, 1)) die("subreaper");
    pid_t controller = getpid();
    pid_t owner = fork();
    if (owner < 0) die("fork owner");
    if (!owner) {
        if (prctl(PR_SET_PDEATHSIG, SIGKILL) || getppid() != controller) _exit(125);
        execvp(command[0], command);
        _exit(127);
    }
    int owner_fd = (int)syscall(SYS_pidfd_open, owner, 0);
    if (owner_fd < 0) { kill(owner, SIGKILL); waitpid(owner, NULL, 0); die("owner pidfd"); }
    pid_t foreign = 0;
    long long limit = now() + 15000000000LL;
    while (now() < limit) {
        foreign = descendant(owner);
        if (foreign) break;
        struct pollfd poller = {.fd = owner_fd, .events = POLLIN};
        if (poll(&poller, 1, 0) != 0) break;
        pause_briefly();
    }
    int foreign_fd = foreign ? (int)syscall(SYS_pidfd_open, foreign, 0) : -1;
    signal_fd(owner_fd);
    int owner_status = 0;
    while (waitpid(owner, &owner_status, 0) < 0 && errno == EINTR) {}
    close(owner_fd);
    if (foreign_fd < 0) {
        cleanup_adopted();
        fprintf(stderr, "FAIL: no confined descendant reached --wait-owner\n");
        return 1;
    }
    struct pollfd death = {.fd = foreign_fd, .events = POLLIN};
    int ready;
    do { ready = poll(&death, 1, 3000); } while (ready < 0 && errno == EINTR);
    if (ready != 1) signal_fd(foreign_fd);
    close(foreign_fd);
    int status = 0;
    pid_t reaped;
    do { reaped = waitpid(foreign, &status, 0); } while (reaped < 0 && errno == EINTR);
    cleanup_adopted();
    if (ready != 1 || reaped != foreign || !WIFSIGNALED(status) || WTERMSIG(status) != SIGKILL ||
            !WIFSIGNALED(owner_status) || WTERMSIG(owner_status) != SIGKILL) {
        fprintf(stderr, "FAIL: foreign descendant survived its owner or was not reaped\n");
        return 1;
    }
    puts("ok language owner death and descendant reap");
    return 0;
}
int main(int argc, char **argv) {
    struct rlimit core = {0, 0};
    if (setrlimit(RLIMIT_CORE, &core)) die("test core limit");
    if (argc >= 3 && !strcmp(argv[1], "--parent-death")) return parent_death(&argv[2]);
    if (argc >= 3 && !strcmp(argv[1], "--unavailable")) {
        struct sock_filter code[] = {
            BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr)),
            BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, SYS_landlock_create_ruleset, 0, 1),
            BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ERRNO | ENOSYS),
            BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
        };
        struct sock_fprog program = {.len = sizeof(code)/sizeof(code[0]), .filter = code};
        if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) || prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &program))
            die("test unavailable Landlock filter");
        execvp(argv[2], &argv[2]);
        die("exec unsupported-host fixture");
    }
    fprintf(stderr, "usage: foreign-child-lifecycle --parent-death|--unavailable COMMAND...\n");
    return 2;
}
