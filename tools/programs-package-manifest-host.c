/* Linux syscall and gzip bridge for the checked package inventory CLI.
 * No inventory, path, tar, hash, JSON, or publication transaction policy.
 * Length-framed protocol on private pipes; ambient host, not a sandbox.
 * Source and output namespaces must remain quiescent during invocation. */
#define _GNU_SOURCE
#include <sys/types.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <sys/prctl.h>
#include <fcntl.h>
#include <unistd.h>
#include <signal.h>
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <limits.h>
#include <zlib.h>
typedef struct { unsigned char *p; size_t n; } Bytes;
static int input_fd, output_fd;
static void die(const char *s) { fprintf(stderr, "package host: %s\n", s); exit(255); }
static void *allocate(size_t n) { void *p = malloc(n ? n : 1); if (!p) die("allocation failed"); return p; }
static Bytes bytes(const void *p, size_t n) { Bytes b = {allocate(n + 1), n}; memcpy(b.p, p, n); b.p[n] = 0; return b; }
static Bytes text(const char *s) { return bytes(s, strlen(s)); }
static void release(Bytes b) { free(b.p); }
static int read_all(int fd, void *p, size_t n) {
    size_t at = 0;
    while (at < n) { ssize_t got = read(fd, (char *)p + at, n - at);
        if (got < 0 && errno == EINTR) continue;
        if (got <= 0) return at == 0 && got == 0 ? 0 : -1;
        at += (size_t)got;
    }
    return 1;
}
static void write_all(int fd, const void *p, size_t n) {
    size_t at = 0;
    while (at < n) { ssize_t done = write(fd, (const char *)p + at, n - at);
        if (done < 0 && errno == EINTR) continue;
        if (done <= 0) die("write failed");
        at += (size_t)done;
    }
}
static int frame(Bytes *b) {
    unsigned char header[8]; int rc = read_all(input_fd, header, 8);
    if (rc != 1) return rc;
    uint64_t n = 0; for (size_t i = 0; i < 8; i++) n |= (uint64_t)header[i] << (8 * i);
    if (n >= SIZE_MAX) die("unrepresentable frame");
    b->n = (size_t)n; b->p = allocate(b->n + 1);
    if (b->n && read_all(input_fd, b->p, b->n) != 1) die("truncated frame");
    b->p[b->n] = 0; return 1;
}
static void send_frame(Bytes b) {
    unsigned char header[8]; uint64_t n = b.n;
    for (size_t i = 0; i < 8; i++) header[i] = (unsigned char)(n >> (8 * i));
    write_all(output_fd, header, 8); write_all(output_fd, b.p, b.n);
}
static void reply(const char *status, size_t n, Bytes *items) {
    Bytes s = text(status); send_frame(s); release(s);
    char count[32]; snprintf(count, sizeof count, "%zu", n);
    s = text(count); send_frame(s); release(s);
    for (size_t i = 0; i < n; i++) { send_frame(items[i]); release(items[i]); }
}
static void ok(void) { reply("ok", 0, NULL); }
static void one(Bytes b) { reply("ok", 1, &b); }
static void error_text(const char *s) { Bytes b = text(s); reply("error", 1, &b); }
static void append(Bytes *b, const void *p, size_t n) {
    if (n > SIZE_MAX - b->n - 1) die("unrepresentable buffer");
    unsigned char *next = realloc(b->p, b->n + n + 1); if (!next) die("allocation failed");
    b->p = next; memcpy(b->p + b->n, p, n); b->n += n; b->p[b->n] = 0;
}
static void fail_errno(int e, const char *op, const char *path) {
    char *message;
    if (asprintf(&message, "%d:%s %s: %s", e, op, path, strerror(e)) < 0)
        die("allocation failed");
    error_text(message); free(message);
}
static int write_bytes(int fd, Bytes b) {
    size_t at = 0;
    while (at < b.n) {
        ssize_t done = write(fd, b.p + at, b.n - at);
        if (done < 0 && errno == EINTR) continue;
        if (done <= 0) return done < 0 ? errno : EIO;
        at += (size_t)done;
    }
    return 0;
}
static int read_bytes(int fd, Bytes *result) {
    unsigned char buf[65536];
    for (;;) {
        ssize_t got = read(fd, buf, sizeof buf);
        if (got < 0 && errno == EINTR) continue;
        if (got < 0) return errno;
        if (!got) return 0;
        append(result, buf, (size_t)got);
    }
}
static void regular(const char *path) {
    int fd = open(path, O_RDONLY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC);
    if (fd < 0) { fail_errno(errno, "open", path); return; }
    struct stat st;
    if (fstat(fd, &st)) {
        int e = errno; close(fd); fail_errno(e, "stat", path); return;
    }
    if (!S_ISREG(st.st_mode)) {
        // The original Perl stream initialization leaves ESPIPE on a FIFO.
        close(fd); error_text(S_ISFIFO(st.st_mode) ? "29:not a regular file" : "255:not a regular file"); return;
    }
    Bytes result = bytes("", 0);
    int e = read_bytes(fd, &result);
    if (close(fd) && !e) e = errno;
    if (e) { release(result); fail_errno(e, "read", path); }
    else one(result);
}
static void exclusive(const char *path, Bytes content) {
    int fd = open(path, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600);
    if (fd < 0) { fail_errno(errno, "create", path); return; }
    int e = write_bytes(fd, content);
    if (close(fd) && !e) e = errno;
    if (e) {
        // This primitive only rolls back the leaf it created itself.
        if (unlink(path)) fail_errno(errno, "rollback", path);
        else fail_errno(e, "write", path);
    } else ok();
}
static void gunzip_one(Bytes compressed) {
    z_stream z = {0};
    if (inflateInit2(&z, 15 + 16) != Z_OK) { error_text("255:cannot initialize gzip decoder"); return; }
    Bytes raw = bytes("", 0);
    size_t at = 0;
    int rc;
    do {
        if (!z.avail_in && at < compressed.n) {
            size_t n = compressed.n - at;
            if (n > UINT_MAX) n = UINT_MAX;
            z.next_in = compressed.p + at;
            z.avail_in = (uInt)n;
            at += n;
        }
        unsigned char buf[65536];
        z.next_out = buf; z.avail_out = sizeof buf;
        rc = inflate(&z, Z_NO_FLUSH);
        append(&raw, buf, sizeof buf - z.avail_out);
    } while (rc == Z_OK);
    size_t consumed = at - z.avail_in;
    inflateEnd(&z);
    if (rc != Z_STREAM_END) { release(raw); error_text("255:cannot decompress pinned archive"); return; }
    char count[32];
    snprintf(count, sizeof count, "%zu", consumed);
    Bytes result[] = {text(count), raw};
    reply("ok", 2, result);
}
static void git_environment(void) {
    const char *keys[] = {"GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY", "GIT_ALTERNATE_OBJECT_DIRECTORIES", "GIT_COMMON_DIR",
        "GIT_NAMESPACE", "GIT_CONFIG_PARAMETERS", "GIT_CONFIG_COUNT"};
    for (size_t i = 0; i < sizeof keys / sizeof *keys; ++i) unsetenv(keys[i]);
    setenv("GIT_CONFIG_NOSYSTEM", "1", 1);
    setenv("GIT_CONFIG_GLOBAL", "/dev/null", 1);
    setenv("GIT_OPTIONAL_LOCKS", "0", 1);
}
static void git_run(size_t n, Bytes *a) {
    int out[2];
    if (pipe2(out, O_CLOEXEC)) die("git pipe failed");
    pid_t pid = fork();
    if (pid < 0) die("git fork failed");
    if (!pid) {
        int null = open("/dev/null", O_RDONLY);
        if (null < 0 || dup2(null, 0) < 0 || dup2(out[1], 1) < 0) _exit(255);
        close(null); close(out[0]); close(out[1]);
        git_environment();
        const char *prefix[] = {"git", "--no-replace-objects", "-c",
            "core.fsmonitor=false", "-c", "core.hooksPath=/dev/null", "-C"};
        char **argv = allocate((n + 8) * sizeof *argv);
        for (size_t i = 0; i < 7; i++) argv[i] = (char *)prefix[i];
        for (size_t i = 0; i < n; i++) argv[i+7] = (char *)a[i].p;
        argv[n+7] = NULL;
        execvp("git", argv); _exit(127);
    }
    close(out[1]);
    Bytes raw = bytes("", 0);
    int e = read_bytes(out[0], &raw);
    close(out[0]);
    int status;
    while (waitpid(pid, &status, 0) < 0) if (errno != EINTR) die("git wait failed");
    if (e || !WIFEXITED(status) || WEXITSTATUS(status)) {
        release(raw);
        char message[64];
        int code = e ? e : WIFEXITED(status) ? WEXITSTATUS(status) : 255;
        snprintf(message, sizeof message, "%d:git command failed", code);
        error_text(message);
    } else one(raw);
}
static void dispatch(const char *op, size_t n, Bytes *a) {
#define IS(name,count) (!strcmp(op,name) && n==(count))
#define P(i) ((char *)a[i].p)
    if (IS("realpath",1)) {
        char *p = realpath(P(0), NULL);
        if (!p) fail_errno(errno, "realpath", P(0));
        else { one(text(p)); free(p); }
    } else if (IS("stat",1)) {
        struct stat s;
        if (lstat(P(0), &s)) fail_errno(errno, "lstat", P(0));
        else one(text(S_ISREG(s.st_mode) ? "file" : S_ISDIR(s.st_mode) ? "dir" : S_ISLNK(s.st_mode) ? "link" : "other"));
    } else if (IS("read",1)) regular(P(0));
    else if (IS("gzip",1)) gunzip_one(a[0]);
    else if (IS("write-new",2)) exclusive(P(0), a[1]);
    else if (IS("unlink",1)) {
        if (unlink(P(0))) fail_errno(errno, "unlink", P(0)); else ok();
    } else if (IS("result",1)) { write_all(STDOUT_FILENO, a[0].p, a[0].n); ok(); }
    else if (!strcmp(op,"git") && n > 1) git_run(n,a);
    else error_text("255:invalid host request");
#undef P
#undef IS
}
int main(int argc, char **argv) {
    if (argc < 2) die("missing checked executable");
    signal(SIGPIPE, SIG_IGN);
    int request[2], response[2];
    if (pipe2(request,O_CLOEXEC) || pipe2(response,O_CLOEXEC)) die("pipe failed");
    pid_t coordinator = getpid();
    pid_t child = fork();
    if (child < 0) die("fork failed");
    if (!child) {
        if (prctl(PR_SET_PDEATHSIG, SIGKILL) || getppid()!=coordinator) _exit(255);
        signal(SIGPIPE, SIG_DFL);
        if (dup2(response[0], 0) < 0 || dup2(request[1], 1) < 0) _exit(255);
        close(request[0]); close(request[1]); close(response[0]); close(response[1]);
        execv(argv[1], argv+1); die("cannot execute checked program");
    }
    close(request[1]); close(response[0]); input_fd=request[0]; output_fd=response[1];
    Bytes op;
    int rc;
    while ((rc=frame(&op))==1) {
        Bytes count;
        if (frame(&count)!=1) die("missing request length");
        char *end=NULL; errno=0;
        unsigned long long number=strtoull((char *)count.p,&end,10);
        if (errno || !count.n || *end || number>SIZE_MAX/sizeof(Bytes)) die("invalid request length");
        size_t n=(size_t)number; release(count);
        Bytes *args=allocate(n*sizeof *args);
        for (size_t i=0;i<n;i++) if(frame(&args[i])!=1) die("missing request field");
        dispatch((char *)op.p,n,args);
        for (size_t i=0;i<n;i++) release(args[i]);
        free(args); release(op);
    }
    close(input_fd); close(output_fd);
    int status;
    while(waitpid(child,&status,0)<0) if(errno!=EINTR) die("wait failed");
    if (rc != 0) return 255;
    return WIFEXITED(status) ? WEXITSTATUS(status) : WIFSIGNALED(status) ? 128+WTERMSIG(status) : 255;
}
