/* Optional independent Linux publication/owner-death oracle.
 * Only this program's newly created descendants are killed or reaped.
 * The language parent and its workers do all computation and publication. */
#define _GNU_SOURCE
#include <sys/prctl.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <time.h>
#include <poll.h>
#include <signal.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static pid_t parent = -1;
static volatile sig_atomic_t interrupted;
static unsigned reaped_workers;
static const char old_bytes[] = "old-complete\n";

static void on_signal(int number) { interrupted = number; }
static uint64_t now(void) {
    struct timespec value;
    if (clock_gettime(CLOCK_MONOTONIC, &value) < 0) { perror("clock"); exit(1); }
    return (uint64_t)value.tv_sec * 1000000000u + (uint64_t)value.tv_nsec;
}
static void tick(void) {
    struct timespec value = {0, 1000000};
    nanosleep(&value, NULL);
}
static void kill_owned_children(void) {
    char path[128];
    snprintf(path, sizeof(path), "/proc/self/task/%ld/children", (long)getpid());
    FILE *file = fopen(path, "r");
    if (!file) return;
    long child;
    while (fscanf(file, "%ld", &child) == 1) if (child > 0) kill((pid_t)child, SIGKILL);
    fclose(file);
}
static void cleanup(void) {
    if (parent > 0) {
        kill(parent, SIGKILL);
        while (waitpid(parent, NULL, 0) < 0 && errno == EINTR) {}
        parent = -1;
    }
    kill_owned_children();
    while (waitpid(-1, NULL, 0) > 0 || errno == EINTR) {}
}
static void fail(const char *message) {
    fprintf(stderr, "publication witness: %s (%s)\n", message, strerror(errno));
    exit(1);
}
static void require(int value, const char *message) { if (!value) fail(message); }
static char *join(const char *directory, const char *name) {
    char *result;
    if (asprintf(&result, "%s/%s", directory, name) < 0) fail("path allocation");
    return result;
}
static void write_all(int fd, const void *data, size_t length) {
    const unsigned char *bytes = data;
    while (length) {
        ssize_t count = write(fd, bytes, length);
        if (count < 0 && errno == EINTR && !interrupted) continue;
        require(count > 0, "write");
        bytes += count;
        length -= (size_t)count;
    }
}
static size_t read_file(const char *path, unsigned char *bytes, size_t capacity) {
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    require(fd >= 0, "open published result");
    size_t total = 0;
    for (;;) {
        unsigned char extra;
        ssize_t count = read(fd, total == capacity ? &extra : bytes + total,
            total == capacity ? 1 : capacity - total);
        if (count < 0 && errno == EINTR) continue;
        require(count >= 0, "read published result");
        if (!count) break;
        require(total < capacity, "published result has trailing data");
        total += (size_t)count;
    }
    require(close(fd) == 0, "close published result");
    return total;
}
static void phase(int fd, const char *expected) {
    uint64_t deadline = now() + 60000000000ull;
    size_t index = 0;
    for (;;) {
        require(!interrupted, "interrupted");
        uint64_t time = now();
        require(time < deadline, "phase deadline");
        struct pollfd item = {fd, POLLIN, 0};
        int ready = poll(&item, 1, (int)((deadline-time)/1000000u + 1u));
        if (ready < 0 && errno == EINTR) continue;
        require(ready > 0, "phase poll");
        char byte;
        ssize_t count = read(fd, &byte, 1);
        if (count < 0 && errno == EINTR) continue;
        require(count == 1, "phase ended early");
        require(byte == expected[index], "unexpected phase");
        if (byte == '\n') return;
        index++;
        require(expected[index] != 0, "phase has trailing bytes");
    }
}
static void wait_for_worker(pid_t pid) {
    char path[128];
    snprintf(path, sizeof(path), "/proc/%ld/task/%ld/children", (long)pid, (long)pid);
    uint64_t deadline = now() + 5000000000ull;
    while (now() < deadline && !interrupted) {
        FILE *file = fopen(path, "r");
        require(file != NULL, "parent disappeared before worker");
        long child = 0;
        int found = fscanf(file, "%ld", &child);
        fclose(file);
        if (found == 1 && child > 0) return;
        tick();
    }
    fail("no worker appeared");
}
static void reap_killed(void) {
    require(parent > 0, "no owned parent");
    require(kill(parent, SIGKILL) == 0, "kill owned parent");
    int status;
    pid_t result;
    do { result = waitpid(parent, &status, 0); } while (result < 0 && errno == EINTR);
    require(result == parent && WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL,
        "parent kill status");
    parent = -1;
    uint64_t deadline = now() + 5000000000ull;
    for (;;) {
        result = waitpid(-1, &status, WNOHANG);
        if (result > 0) { reaped_workers++; continue; }
        if (result < 0 && errno == EINTR) continue;
        if (result < 0 && errno == ECHILD) return;
        require(result == 0, "wait worker");
        require(now() < deadline && !interrupted, "worker survived owner death");
        tick();
    }
}
static void run_case(const char *executable, const char *work,
    const unsigned char *golden, size_t golden_size, int target, unsigned number) {
    const char *names[] = {"pool\n","collected\n","partial\n","synced\n","renamed\n","committed\n"};
    char leaf[64];
    snprintf(leaf, sizeof(leaf), "kill-%u", number);
    char *directory = join(work, leaf);
    require(mkdir(directory, 0700) == 0, "fresh state directory");
    char *published = join(directory, "published");
    int fd = open(published, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0600);
    require(fd >= 0, "create old publication");
    write_all(fd, old_bytes, sizeof(old_bytes)-1);
    require(fsync(fd) == 0 && close(fd) == 0, "sync old publication");
    int in[2], out[2];
    require(pipe2(in, O_CLOEXEC) == 0 && pipe2(out, O_CLOEXEC) == 0, "pipes");
    pid_t owner = getpid();
    parent = fork();
    require(parent >= 0, "fork owned parent");
    if (!parent) {
        if (prctl(PR_SET_PDEATHSIG, SIGKILL) < 0 || getppid() != owner) _exit(125);
        if (dup2(in[0], STDIN_FILENO) < 0 || dup2(out[1], STDOUT_FILENO) < 0) _exit(125);
        close(in[0]); close(in[1]); close(out[0]); close(out[1]);
        if (setenv("PROGRAMS_PUBLICATION_ROOT", directory, 1) < 0) _exit(125);
        execl(executable, executable, (char *)NULL);
        _exit(127);
    }
    close(in[0]); close(out[1]);
    for (int current = 0; current < 6; current++) {
        phase(out[0], names[current]);
        if (target == current) break;
        write_all(in[1], "x", 1);
        if (target < 0) { wait_for_worker(parent); break; }
    }
    reap_killed();
    close(in[1]); close(out[0]);
    unsigned char actual[128];
    size_t actual_size = read_file(published, actual, sizeof(actual));
    if (target >= 4) {
        require(actual_size == golden_size && memcmp(actual, golden, golden_size) == 0,
            "renamed publication is not the complete new result");
    } else {
        require(actual_size == sizeof(old_bytes)-1 &&
            memcmp(actual, old_bytes, sizeof(old_bytes)-1) == 0,
            "uncommitted work changed the old publication");
    }
    free(published); free(directory);
}
int main(int argc, char **argv) {
    require(argc == 4, "usage: executable work golden");
    require(prctl(PR_SET_CHILD_SUBREAPER, 1) == 0, "own subreaper");
    atexit(cleanup);
    struct sigaction action = {0};
    action.sa_handler = on_signal;
    sigemptyset(&action.sa_mask);
    sigaction(SIGINT, &action, NULL); sigaction(SIGTERM, &action, NULL);
    signal(SIGPIPE, SIG_IGN);
    unsigned char golden[128];
    size_t size = read_file(argv[3], golden, sizeof(golden));
    require(size == sizeof(golden), "wrong complete result size");
    unsigned cases = 0;
    for (int target = 0; target < 6; target++) {
        for (unsigned repeat = 0; repeat < 2; repeat++) {
            run_case(argv[1], argv[2], golden, size, target, cases++);
        }
    }
    for (unsigned repeat = 0; repeat < 8; repeat++)
        run_case(argv[1], argv[2], golden, size, -1, cases++);
    printf("ok publication kill campaign: %u kills, %u adopted workers reaped; old or new complete result at all phases\n",
        cases, reaped_workers);
    return 0;
}
