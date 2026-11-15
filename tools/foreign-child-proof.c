#define _GNU_SOURCE
#include <elf.h>
#include <errno.h>
#include <fcntl.h>
#include <linux/audit.h>
#include <linux/filter.h>
#include <linux/landlock.h>
#include <linux/memfd.h>
#include <linux/seccomp.h>
#include <poll.h>
#include <signal.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/prctl.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/sysmacros.h>
#include <sys/timerfd.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

/* Feasibility witness only: this is not a language capability or a runtime helper.
 * Every confinement operation runs after fork in one child owned by this proof.
 * Runtime libraries and the ELF interpreter are staged inside the granted root.
 */
#ifndef MFD_EXEC
#define MFD_EXEC 0x0010U
#endif
#define LL_FS_IOCTL_DEV (1ULL << 15)
#define LL_FS_RESOLVE_UNIX (1ULL << 16)
#define X32_BIT 0x40000000U

static void fail(const char *what) {
    perror(what);
    exit(1);
}
static void require(int condition, const char *what) {
    if (!condition) {
        fprintf(stderr, "FAIL: %s\n", what);
        exit(1);
    }
}
static void write_all(int fd, const void *bytes, size_t length) {
    const unsigned char *p = bytes;
    while (length) {
        ssize_t count = write(fd, p, length);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) fail("write");
        p += count;
        length -= (size_t)count;
    }
}
static unsigned char *read_all(int fd, size_t *length) {
    size_t used = 0, capacity = 4096;
    unsigned char *bytes = malloc(capacity + 1);
    if (!bytes) fail("malloc");
    for (;;) {
        if (used == capacity) {
            require(capacity <= (SIZE_MAX - 1) / 2, "capture size overflow");
            capacity *= 2;
            bytes = realloc(bytes, capacity + 1);
            if (!bytes) fail("realloc");
        }
        ssize_t count = read(fd, bytes + used, capacity - used);
        if (count < 0 && errno == EINTR) continue;
        if (count < 0) fail("read");
        if (count == 0) break;
        used += (size_t)count;
    }
    bytes[used] = 0;
    *length = used;
    return bytes;
}
static void path_join(char *out, size_t capacity, const char *root, const char *tail) {
    int count = snprintf(out, capacity, "%s/%s", root, tail);
    require(count >= 0 && (size_t)count < capacity, "fixture path length");
}
static int executable_image(const char *path, const char *loader, const char *copy) {
    int input = open(path, O_RDONLY | O_CLOEXEC);
    if (input < 0) fail("open source executable");
    size_t length;
    unsigned char *bytes = read_all(input, &length);
    close(input);
    require(length >= sizeof(Elf64_Ehdr), "ELF header");
    Elf64_Ehdr *header = (Elf64_Ehdr *)bytes;
    require(memcmp(header->e_ident, ELFMAG, SELFMAG) == 0 &&
            header->e_ident[EI_CLASS] == ELFCLASS64 &&
            header->e_ident[EI_DATA] == ELFDATA2LSB &&
            header->e_machine == EM_X86_64 &&
            header->e_phentsize == sizeof(Elf64_Phdr), "ELF x86-64 format");
    require(header->e_phoff <= length &&
            header->e_phnum <= (length - header->e_phoff) / sizeof(Elf64_Phdr),
            "ELF program header range");
    size_t interpreter = SIZE_MAX;
    for (size_t i = 0; i < header->e_phnum; ++i) {
        size_t offset = (size_t)header->e_phoff + i * sizeof(Elf64_Phdr);
        Elf64_Phdr *program = (Elf64_Phdr *)(bytes + offset);
        if (program->p_type == PT_INTERP) {
            require(interpreter == SIZE_MAX, "one ELF interpreter");
            interpreter = offset;
        }
    }
    if (interpreter != SIZE_MAX) {
        size_t extra = strlen(loader) + 1;
        require(extra <= SIZE_MAX - length, "ELF interpreter size");
        bytes = realloc(bytes, length + extra);
        if (!bytes) fail("ELF realloc");
        memcpy(bytes + length, loader, extra);
        Elf64_Phdr *program = (Elf64_Phdr *)(bytes + interpreter);
        program->p_offset = length;
        program->p_filesz = extra;
        program->p_memsz = extra;
        length += extra;
    }
    int image = (int)syscall(SYS_memfd_create, "foreign-proof", MFD_CLOEXEC | MFD_ALLOW_SEALING | MFD_EXEC);
    if (image < 0) fail("memfd image");
    write_all(image, bytes, length);
    if (fchmod(image, 0500)) fail("image mode");
    if (fcntl(image, F_ADD_SEALS, F_SEAL_WRITE | F_SEAL_GROW | F_SEAL_SHRINK | F_SEAL_SEAL))
        fail("seal image");
    int seals = fcntl(image, F_GET_SEALS);
    require(seals == (F_SEAL_WRITE | F_SEAL_GROW | F_SEAL_SHRINK | F_SEAL_SEAL),
            "image has every immutability seal");
    require(pwrite(image, "x", 1, 0) == -1 && errno == EPERM, "sealed image cannot change");
    if (copy) {
        int output = open(copy, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0500);
        if (output < 0) fail("create reexec fixture");
        write_all(output, bytes, length);
        close(output);
    }
    free(bytes);
    return image;
}
static void child_failure(const char *phase) {
    int saved = errno;
    dprintf(2, "confinement setup: %s: errno %d\n", phase, saved);
    _exit(125);
}
static void landlock_root(int root, int null_device) {
    int abi = (int)syscall(SYS_landlock_create_ruleset, NULL, 0, LANDLOCK_CREATE_RULESET_VERSION);
    if (abi < 9) {
        errno = EOPNOTSUPP;
        child_failure("Landlock ABI >= 9 required");
    }
    /* UAPI headers may be older than the host. First 24 bytes are ABI6.
     * Handle every filesystem action through ABI9; allow only read and execute.
     * Deny all TCP/UDP port access and scope signals/abstract Unix sockets too.
     * seccomp independently denies creation/use of every socket family.
     */
    struct {
        uint64_t handled_access_fs;
        uint64_t handled_access_net;
        uint64_t scoped;
    } attr = {
        .handled_access_fs = (1ULL << 17) - 1,
        .handled_access_net = abi >= 10 ? 15 : 3,
        .scoped = 3
    };
    int rules = (int)syscall(SYS_landlock_create_ruleset, &attr, sizeof(attr), 0);
    if (rules < 0) child_failure("create Landlock ruleset");
    struct landlock_path_beneath_attr path = {
        .allowed_access = LANDLOCK_ACCESS_FS_READ_FILE | LANDLOCK_ACCESS_FS_READ_DIR |
                          LANDLOCK_ACCESS_FS_EXECUTE,
        .parent_fd = root
    };
    if (syscall(SYS_landlock_add_rule, rules, LANDLOCK_RULE_PATH_BENEATH, &path, 0))
        child_failure("add Landlock root");
    if (null_device >= 0) {
        struct landlock_path_beneath_attr sink = {
            .allowed_access = LANDLOCK_ACCESS_FS_READ_FILE | LANDLOCK_ACCESS_FS_WRITE_FILE,
            .parent_fd = null_device
        };
        if (syscall(SYS_landlock_add_rule, rules, LANDLOCK_RULE_PATH_BENEATH, &sink, 0))
            child_failure("add explicit null device");
    }
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0)) child_failure("no_new_privs");
    if (syscall(SYS_landlock_restrict_self, rules, 0)) child_failure("enforce Landlock");
    close(rules);
}
#define ALLOW(name) BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, SYS_##name, 0, 1), \
                    BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW)
static void seccomp_policy(void) {
    struct sock_filter filter[] = {
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, arch)),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, AUDIT_ARCH_X86_64, 1, 0),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS),
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr)),
        BPF_JUMP(BPF_JMP | BPF_JSET | BPF_K, X32_BIT, 0, 1),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS),
        ALLOW(read), ALLOW(write), ALLOW(readv), ALLOW(writev), ALLOW(pread64),
        ALLOW(close), ALLOW(lseek), ALLOW(fstat), ALLOW(newfstatat), ALLOW(statx),
        ALLOW(open), ALLOW(openat), ALLOW(openat2), ALLOW(access), ALLOW(faccessat),
        ALLOW(faccessat2), ALLOW(readlink), ALLOW(readlinkat), ALLOW(getdents64),
        ALLOW(getcwd), ALLOW(chdir), ALLOW(fchdir), ALLOW(dup), ALLOW(dup2), ALLOW(dup3),
        ALLOW(mmap), ALLOW(mprotect), ALLOW(munmap), ALLOW(mremap), ALLOW(madvise),
        ALLOW(brk), ALLOW(rt_sigaction), ALLOW(rt_sigprocmask), ALLOW(rt_sigreturn),
        ALLOW(sigaltstack), ALLOW(arch_prctl), ALLOW(set_tid_address),
        ALLOW(set_robust_list), ALLOW(rseq), ALLOW(futex), ALLOW(getpid),
        ALLOW(getppid), ALLOW(gettid), ALLOW(getuid), ALLOW(geteuid), ALLOW(getgid),
        ALLOW(getegid), ALLOW(clock_gettime), ALLOW(gettimeofday), ALLOW(time),
        ALLOW(nanosleep), ALLOW(clock_nanosleep), ALLOW(getrandom), ALLOW(uname),
        ALLOW(execve), ALLOW(execveat), ALLOW(exit), ALLOW(exit_group),
        /* prlimit64 may query this process, but cannot alter another process. */
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, SYS_prlimit64, 0, 9),
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args[0])),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 6),
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args[2])),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 4),
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args[2]) + 4),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 2),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ERRNO | EPERM),
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr)),
        /* fcntl cannot enable asynchronous signals or manipulate file leases. */
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, SYS_fcntl, 0, 5),
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args[1])),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, F_GETFD, 2, 0),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, F_GETFL, 1, 0),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, F_SETFD, 0, 1),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ERRNO | EPERM)
    };
    struct sock_fprog program = { .len = (unsigned short)(sizeof(filter) / sizeof(filter[0])), .filter = filter };
    if (prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &program)) child_failure("seccomp filter");
}
struct child { pid_t pid; int pidfd; int output; struct timespec started; };
static struct child launch(int image, const char *root, char *const arguments[], int grant_null) {
    char runtime[8192], home[8192], config[8192];
    require(snprintf(runtime, sizeof(runtime), "LD_LIBRARY_PATH=/proc/self/fd/5/.runtime") < (int)sizeof(runtime), "runtime environment length");
    require(snprintf(home, sizeof(home), "HOME=%s", root) < (int)sizeof(home), "home environment length");
    require(snprintf(config, sizeof(config), "GIT_CONFIG_GLOBAL=%s/.no-global-config", root) < (int)sizeof(config), "git config environment length");
    char *environment[] = {runtime, home, config, "LC_ALL=C", "GIT_CONFIG_NOSYSTEM=1",
                           "GIT_OPTIONAL_LOCKS=0", "GIT_TERMINAL_PROMPT=0", NULL};
    int channels[2], input_pipe[2];
    if (pipe2(channels, O_CLOEXEC)) fail("capture pipe");
    if (pipe2(input_pipe, O_CLOEXEC)) fail("empty input pipe");
    close(input_pipe[1]);
    int directory = open(root, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    if (directory < 0) fail("root");
    /* Keep original descriptors out of the fixed 0..5 import slots. */
    int exec_copy = fcntl(image, F_DUPFD_CLOEXEC, 10);
    int root_copy = fcntl(directory, F_DUPFD_CLOEXEC, 10);
    int output_copy = fcntl(channels[1], F_DUPFD_CLOEXEC, 10);
    int input_copy = fcntl(input_pipe[0], F_DUPFD_CLOEXEC, 10);
    if (exec_copy < 0 || root_copy < 0 || output_copy < 0 || input_copy < 0) fail("import duplicate");
    close(input_pipe[0]);
    int null_copy = -1;
    if (grant_null) {
        int null_device = open("/dev/null", O_PATH | O_CLOEXEC);
        struct stat device;
        require(null_device >= 0 && fstat(null_device, &device) == 0 && S_ISCHR(device.st_mode) &&
                major(device.st_rdev) == 1 && minor(device.st_rdev) == 3, "exact Linux null device");
        null_copy = fcntl(null_device, F_DUPFD_CLOEXEC, 10);
        if (null_copy < 0) fail("null import duplicate");
        close(null_device);
    }
    close(directory);
    struct timespec started;
    if (clock_gettime(CLOCK_MONOTONIC, &started)) fail("launch time");
    pid_t parent = getpid();
    pid_t pid = fork();
    if (pid < 0) fail("fork");
    if (pid == 0) {
        if (prctl(PR_SET_PDEATHSIG, SIGKILL)) child_failure("parent death");
        if (getppid() != parent) _exit(125); /* closes the pre-prctl death race */
        if (dup3(exec_copy, 3, O_CLOEXEC) < 0 ||
            dup3(root_copy, 5, 0) < 0 ||
            dup2(input_copy, 0) < 0 || dup2(output_copy, 1) < 0 || dup2(output_copy, 2) < 0)
            child_failure("descriptor imports");
        /* Explicit empty pipe input; no ambient stdin or terminal. */
        if (null_copy >= 0) {
            if (dup3(null_copy, 4, O_CLOEXEC) < 0) child_failure("null import");
        } else close(4);
        if (syscall(SYS_close_range, 6U, ~0U, 0)) child_failure("close inherited descriptors");
        if (fchdir(5)) child_failure("root cwd");
        landlock_root(5, null_copy >= 0 ? 4 : -1);
        close(4);
        /* Root5 is an explicit read-only import for loader lookup and reexec. */
        seccomp_policy();
        syscall(SYS_execveat, 3, "", arguments, environment, AT_EMPTY_PATH);
        child_failure("exec image");
    }
    close(exec_copy);
    close(root_copy);
    close(output_copy);
    close(input_copy);
    if (null_copy >= 0) close(null_copy);
    close(channels[1]);
    int pidfd = (int)syscall(SYS_pidfd_open, pid, 0);
    if (pidfd < 0) {
        kill(pid, SIGKILL);
        waitpid(pid, NULL, 0);
        fail("pidfd");
    }
    return (struct child){ .pid = pid, .pidfd = pidfd, .output = channels[0], .started = started };
}
struct outcome { int status; int deadline; char *output; };
static struct outcome finish(struct child child, int milliseconds) {
    int timer = timerfd_create(CLOCK_MONOTONIC, TFD_CLOEXEC);
    if (timer < 0) fail("deadline timer");
    struct itimerspec when = { .it_value = child.started };
    when.it_value.tv_sec += milliseconds / 1000;
    when.it_value.tv_nsec += (milliseconds % 1000) * 1000000L;
    if (when.it_value.tv_nsec >= 1000000000L) {
        when.it_value.tv_nsec -= 1000000000L;
        ++when.it_value.tv_sec;
    }
    if (timerfd_settime(timer, TFD_TIMER_ABSTIME, &when, NULL)) fail("absolute deadline arm");
    struct pollfd pollers[3] = {
        {.fd = child.pidfd, .events = POLLIN},
        {.fd = timer, .events = POLLIN},
        {.fd = child.output, .events = POLLIN}
    };
    size_t used = 0, capacity = 4096;
    char *output = malloc(capacity + 1);
    if (!output) fail("capture malloc");
    int deadline = 0, done = 0;
    while (!done) {
        if (poll(pollers, 3, -1) < 0) {
            if (errno == EINTR) continue;
            fail("deadline poll");
        }
        if (pollers[0].revents & POLLIN) done = 1;
        if (!done && (pollers[1].revents & POLLIN)) {
            if (syscall(SYS_pidfd_send_signal, child.pidfd, SIGKILL, NULL, 0) && errno != ESRCH)
                fail("deadline kill");
            deadline = 1;
        }
        if (pollers[2].revents & (POLLIN | POLLHUP)) {
            if (used == capacity) {
                require(capacity <= (SIZE_MAX - 1) / 2, "capture size overflow");
                capacity *= 2;
                output = realloc(output, capacity + 1);
                if (!output) fail("capture realloc");
            }
            ssize_t count = read(child.output, output + used, capacity - used);
            if (count < 0 && errno != EINTR) fail("capture read");
            if (count > 0) used += (size_t)count;
            if (count == 0) pollers[2].fd = -1;
        }
    }
    int status;
    while (waitpid(child.pid, &status, 0) < 0) {
        if (errno != EINTR) fail("reap");
    }
    size_t tail_length;
    unsigned char *tail = read_all(child.output, &tail_length);
    require(tail_length <= SIZE_MAX - used - 1, "capture tail overflow");
    output = realloc(output, used + tail_length + 1);
    if (!output) fail("capture tail realloc");
    memcpy(output + used, tail, tail_length);
    output[used + tail_length] = 0;
    free(tail);
    close(child.output);
    close(child.pidfd);
    close(timer);
    return (struct outcome){ .status = status, .deadline = deadline, .output = output };
}
static void expect_denied(int result, const char *operation) {
    if (result != -1 || (errno != EPERM && errno != EACCES && errno != EBADF)) {
        dprintf(2, "FAIL: %s result %d errno %d\n", operation, result, errno);
        _exit(3);
    }
}
static int probe(const char *root, const char *outside, int after_exec) {
    char path[8192];
    int inside = open("inside.txt", O_RDONLY);
    require(inside >= 0, "inside-root read admitted");
    char byte;
    require(read(inside, &byte, 1) == 1 && byte == 'i', "inside data");
    close(inside);
    expect_denied(open(outside, O_RDONLY), "outside content");
    expect_denied(open("escape", O_RDONLY), "symlink escape");
    expect_denied(open("/proc/self/environ", O_RDONLY), "outside proc content");
    for (int descriptor = 0; descriptor < 64; ++descriptor) {
        snprintf(path, sizeof(path), "/proc/%d/fd/%d", getppid(), descriptor);
        int parent_file = open(path, O_RDONLY | O_NONBLOCK);
        if (parent_file < 0 && errno == ENOENT) continue;
        expect_denied(parent_file, "parent descriptor cannot be reopened through proc");
    }
    expect_denied(fcntl(100, F_GETFD), "ambient inherited descriptor");
    expect_denied(open("mutated", O_CREAT | O_WRONLY, 0600), "root write");
    expect_denied(chmod(outside, 0777), "outside chmod");
    expect_denied((int)syscall(SYS_mknod, "device", S_IFCHR | 0600, makedev(1, 3)), "device creation");
    expect_denied((int)syscall(SYS_ioctl, 0, 0, 0), "device ioctl");
    int families[] = {AF_INET, AF_INET6, AF_UNIX, AF_NETLINK};
    for (size_t i = 0; i < sizeof(families) / sizeof(families[0]); ++i)
        expect_denied(socket(families[i], SOCK_STREAM, 0), "socket family");
    expect_denied(socket(AF_INET, SOCK_DGRAM, 0), "UDP socket");
    int pair[2];
    expect_denied(socketpair(AF_UNIX, SOCK_STREAM, 0, pair), "socketpair");
    expect_denied((int)syscall(SYS_fork), "fork");
    expect_denied((int)syscall(SYS_vfork), "vfork");
    expect_denied((int)syscall(SYS_clone, SIGCHLD, 0, 0, 0, 0), "clone");
    expect_denied((int)syscall(SYS_clone3, 0, 0), "clone3");
    expect_denied(prctl(PR_SET_PDEATHSIG, 0), "clear parent death");
    expect_denied(prctl(PR_SET_NO_NEW_PRIVS, 0, 0, 0, 0), "clear no_new_privs");
    expect_denied((int)syscall(SYS_io_uring_setup, 1, 0), "io_uring");
    expect_denied((int)syscall(SYS_ptrace, 0, 0, 0, 0), "ptrace");
    expect_denied(setuid(0), "credential change");
    struct stat metadata;
    require(stat(outside, &metadata) == 0, "metadata visibility is an explicit limitation");
    if (!after_exec) {
        char *arguments[] = {"forbidden", NULL};
        char *environment[] = {NULL};
        expect_denied(execve("/bin/true", arguments, environment), "outside executable");
        path_join(path, sizeof(path), root, "second-probe");
        char *again[] = {path, "--probe-after-exec", (char *)root, (char *)outside, NULL};
        extern char **environ;
        execve(path, again, environ);
        fail("reexec root image");
    }
    puts("read-root, symlink, inherited-fd, filesystem-mutation, socket, fork, privilege and reexec cases pass");
    return 0;
}
static void parent_death(int image, const char *root, const char *outside) {
    int channel[2];
    if (pipe2(channel, O_CLOEXEC)) fail("supervisor pipe");
    pid_t controller = getpid();
    pid_t owner = fork();
    if (owner < 0) fail("owner fork");
    if (owner == 0) {
        if (prctl(PR_SET_PDEATHSIG, SIGKILL) || getppid() != controller) _exit(125);
        close(channel[0]);
        char *args[] = {"probe", "--wait-owner", (char *)root, (char *)outside, NULL};
        struct child child = launch(image, root, args, 0);
        char ready[6];
        size_t used = 0;
        while (used < sizeof(ready)) {
            ssize_t count = read(child.output, ready + used, sizeof(ready) - used);
            if (count <= 0) _exit(4);
            used += (size_t)count;
        }
        if (memcmp(ready, "ready\n", 6) != 0) _exit(4);
        write_all(channel[1], &child.pid, sizeof(child.pid));
        for (;;) pause();
    }
    close(channel[1]);
    struct pollfd descriptor = {.fd = channel[0], .events = POLLIN};
    if (poll(&descriptor, 1, 3000) != 1) {
        kill(owner, SIGKILL);
        waitpid(owner, NULL, 0);
        fail("owner handshake deadline");
    }
    pid_t child;
    require(read(channel[0], &child, sizeof(child)) == sizeof(child), "owner child identity");
    close(channel[0]);
    int pinned = (int)syscall(SYS_pidfd_open, child, 0);
    if (pinned < 0) fail("pin descendant");
    require(kill(owner, SIGKILL) == 0, "kill owned supervisor");
    int owner_status;
    require(waitpid(owner, &owner_status, 0) == owner, "reap owned supervisor");
    struct pollfd death = {.fd = pinned, .events = POLLIN};
    int result = poll(&death, 1, 3000);
    if (result != 1) syscall(SYS_pidfd_send_signal, pinned, SIGKILL, NULL, 0);
    int status;
    require(waitpid(child, &status, 0) == child, "reap adopted descendant");
    close(pinned);
    require(result == 1 && WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL,
            "parent death kills child without parent cleanup");
    puts("parent-death case passes; no detached process");
}
int main(int argc, char **argv) {
    if (argc > 1 && !strcmp(argv[1], "--capture-exit")) {
        extern char **environ;
        const char *expected[] = {"LD_LIBRARY_PATH=/proc/self/fd/5/.runtime", "HOME=/proc/self/fd/5",
            "GIT_CONFIG_GLOBAL=/proc/self/fd/5/.no-global-config", "GIT_CONFIG_NOSYSTEM=1",
            "GIT_OPTIONAL_LOCKS=0", "GIT_TERMINAL_PROMPT=0", "LC_ALL=C"};
        size_t count = 0;
        while (environ[count]) ++count;
        require(count == sizeof(expected)/sizeof(expected[0]), "only explicit environment");
        for (size_t i = 0; i < count; ++i) require(!strcmp(environ[i], expected[i]), "fixed environment bytes");
        char input;
        require(read(0, &input, 1) == 0, "explicit null input");
        write_all(1, "out\0bytes", 9);
        write_all(2, "err\0bytes", 9);
        return 37;
    }
    if (argc > 1 && !strcmp(argv[1], "--capture-rewrite")) {
        write_all(1, "obsolete capture", 16);
        require(lseek(1, 0, SEEK_SET) == 0, "capture is seekable");
        write_all(1, "rewrite", 7);
        int own = open("/proc/self/fd/1", O_RDWR | O_TRUNC);
        require(own >= 0, "private anonymous capture can be reopened/truncated");
        write_all(own, "rewritten", 9);
        close(own);
        return 0;
    }
    if (argc > 1 && !strcmp(argv[1], "--memfd-reexec")) {
        int file = open("second-probe", O_RDONLY);
        if (file < 0) fail("open permitted reexec adversary");
        unsigned char buffer[8192];
        for (;;) {
            ssize_t count = read(file, buffer, sizeof(buffer));
            if (count < 0 && errno == EINTR) continue;
            if (count < 0) fail("read permitted reexec adversary");
            if (!count) break;
            write_all(1, buffer, (size_t)count);
        }
        close(file);
        char *again[] = {"ld.so", "--library-path", "/proc/self/fd/5/.runtime",
            "/proc/self/fd/1", "--probe-after-exec", argv[2], argv[3], NULL};
        extern char **environ;
        execve("/proc/self/fd/5/.runtime/ld.so", again, environ);
        fail("anonymous capture reexec");
    }
    if (argc > 1 && !strcmp(argv[1], "--wait-owner-reexec")) {
        char *again[] = {"second-probe", "--wait-owner", NULL};
        extern char **environ;
        execve("/proc/self/fd/5/second-probe", again, environ);
        fail("setid-marked reexec");
    }
    if (argc > 1 && !strcmp(argv[1], "--signal")) __builtin_trap();
    if (argc > 1 && !strcmp(argv[1], "--spin-reexec")) {
        struct timespec tick = {.tv_sec = 0, .tv_nsec = 75000000};
        while (nanosleep(&tick, &tick) < 0 && errno == EINTR) {}
        char *again[] = {"second-probe", "--spin", NULL};
        extern char **environ;
        execve("/proc/self/fd/5/second-probe", again, environ);
        fail("deadline reexec");
    }
    if (argc > 1 && !strcmp(argv[1], "--probe"))
        return probe(argv[2], argv[3], 0);
    if (argc > 1 && !strcmp(argv[1], "--probe-after-exec"))
        return probe(argv[2], argv[3], 1);
    if (argc > 1 && (!strcmp(argv[1], "--spin") || !strcmp(argv[1], "--wait-owner"))) {
        expect_denied((int)syscall(SYS_fork), "deadline fork");
        expect_denied(prctl(PR_SET_PDEATHSIG, 0), "deadline parent-death reset");
        write_all(1, "ready\n", 6);
        for (;;) {
            struct timespec tick = {.tv_sec = 0, .tv_nsec = 1000000};
            nanosleep(&tick, NULL);
        }
    }
    require(argc == 4, "usage: foreign-child-proof ROOT OUTSIDE SELF");
    if (prctl(PR_SET_CHILD_SUBREAPER, 1)) fail("proof subreaper");
    int abi = (int)syscall(SYS_landlock_create_ruleset, NULL, 0, LANDLOCK_CREATE_RULESET_VERSION);
    printf("Landlock ABI %d; policy requires ABI >= 9; fail closed\n", abi);
    require(abi >= 9, "Landlock support");
    const char *loader = "/proc/self/fd/5/.runtime/ld.so";
    char second[8192];
    path_join(second, sizeof(second), argv[1], "second-probe");
    int image = executable_image(argv[3], loader, second);
    int ambient = open(argv[2], O_RDONLY | O_CLOEXEC);
    if (ambient < 0 || dup2(ambient, 100) != 100) fail("ambient descriptor fixture");
    close(ambient);
    char *probe_args[] = {"probe", "--probe", argv[1], argv[2], NULL};
    struct outcome checked = finish(launch(image, argv[1], probe_args, 0), 3000);
    fputs(checked.output, stdout);
    require(!checked.deadline && WIFEXITED(checked.status) && WEXITSTATUS(checked.status) == 0, "confinement probes");
    free(checked.output);
    struct outcome with_null = finish(launch(image, argv[1], probe_args, 1), 3000);
    require(!with_null.deadline && WIFEXITED(with_null.status) && WEXITSTATUS(with_null.status) == 0,
            "explicit null device does not widen the other restrictions");
    free(with_null.output);
    puts("the same denial probes pass with the explicit NullDevice grant");
    char *spin_args[] = {"probe", "--spin", argv[1], argv[2], NULL};
    struct outcome spin = finish(launch(image, argv[1], spin_args, 0), 100);
    require(spin.deadline && WIFSIGNALED(spin.status) && WTERMSIG(spin.status) == SIGKILL,
            "deadline kills a child that cannot fork or disable parent death");
    free(spin.output);
    puts("100 ms deadline case passes; child reaped");
    parent_death(image, argv[1], argv[2]);
    close(image);
    int git = executable_image("/usr/bin/git", loader, NULL);
    char *git_args[] = {"git", "--no-optional-locks", "-c", "core.fsmonitor=false", "-c",
                        "core.hooksPath=/nonexistent", "status", "--porcelain=v1", NULL};
    struct outcome strict = finish(launch(git, argv[1], git_args, 0), 3000);
    require(!strict.deadline && WIFEXITED(strict.status) && WEXITSTATUS(strict.status) != 0 &&
            strstr(strict.output, "/dev/null") != NULL, "git requires explicit null device");
    free(strict.output);
    puts("strict root-only git is refused: /dev/null is a required explicit extra grant");
    struct outcome status = finish(launch(git, argv[1], git_args, 1), 3000);
    fputs(status.output, stdout);
    require(!status.deadline && WIFEXITED(status.status) && WEXITSTATUS(status.status) == 0, "git status");
    require(strstr(status.output, "?? inside.txt") != NULL, "git observed granted root");
    free(status.output);
    close(git);
    close(100);
    puts("git status passes: sealed initial image, dynamic assets inside R, explicit NullDevice");
    return 0;
}
