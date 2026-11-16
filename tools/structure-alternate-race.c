/* Deterministic negative-module-dependency witness.
 * Only this invocation's compiler and its descendants are traced. The parent
 * creates one competing alternate after the kernel has returned ENOENT; it
 * never changes registers, syscall results, or production compiler behavior. */
#define _GNU_SOURCE
#include <sys/ptrace.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static struct { pid_t pid; int pending; } owned[64];
static size_t live;
static volatile sig_atomic_t interrupted;
static const char competitor[] = "pub fn value() -> i64 { 99i64 }\n";

static void stop(int signal) { interrupted = signal; }
static void cleanup(void) {
    for (size_t i=0; i<64; i++) if (owned[i].pid) kill(owned[i].pid,SIGKILL);
    while (waitpid(-1,NULL,__WALL)>0 || errno==EINTR) {}
}
static void die(const char *message) { perror(message); exit(1); }
static void need(int condition, const char *message) {
    if (!condition) { fprintf(stderr,"alternate dependency witness: %s\n",message); exit(1); }
}
static size_t slot(pid_t pid) {
    for (size_t i=0; i<64; i++) if (owned[i].pid==pid) return i;
    need(0,"untracked traced process"); return 0;
}
static void remember(pid_t pid) {
    for (size_t i=0; i<64; i++) if (owned[i].pid==pid) return;
    for (size_t i=0; i<64; i++) if (!owned[i].pid) {
        owned[i].pid=pid; owned[i].pending=0; live++; return;
    }
    need(0,"too many owned descendants");
}
static void resume(pid_t pid, int signal) {
    if (ptrace(PTRACE_SYSCALL,pid,0,(void *)(intptr_t)signal)<0 && errno!=ESRCH) die("resume compiler");
}
static int basename_is(pid_t pid, uint64_t address, const char *expected) {
    /* Read exactly the bounded expected basename and its terminator. */
    size_t length=strlen(expected)+1;
    for (size_t at=0; at<length;) {
        uintptr_t current=(uintptr_t)(address+at);
        uintptr_t aligned=current & ~(uintptr_t)(sizeof(long)-1);
        errno=0;
        long word=ptrace(PTRACE_PEEKDATA,pid,(void *)aligned,0);
        if (word==-1 && errno) die("read syscall basename");
        size_t offset=(size_t)(current-aligned), take=sizeof(word)-offset;
        if (take>length-at) take=length-at;
        if (memcmp((const char *)&word+offset,expected+at,take)) return 0;
        at+=take;
    }
    return 1;
}
static int same_parent(pid_t pid, uint64_t descriptor, const struct stat *parent) {
    char path[96];
    int length=snprintf(path,sizeof(path),"/proc/%ld/fd/%lld",(long)pid,(long long)(int64_t)descriptor);
    need(length>0 && (size_t)length<sizeof(path),"descriptor path overflow");
    struct stat actual;
    if (stat(path,&actual)<0) die("inspect syscall parent");
    return S_ISDIR(actual.st_mode) && actual.st_dev==parent->st_dev && actual.st_ino==parent->st_ino;
}
static struct stat inject(int parent) {
    int fd=openat(parent,"mod.lkj",O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW|O_CLOEXEC,0600);
    if (fd<0) die("create competing nested module");
    size_t at=0;
    while (at<sizeof(competitor)-1) {
        ssize_t count=write(fd,competitor+at,sizeof(competitor)-1-at);
        if (count<0 && errno==EINTR) continue;
        if (count<=0) die("write competing nested module");
        at+=(size_t)count;
    }
    struct stat result;
    if (fstat(fd,&result)<0 || fsync(fd)<0) die("retain competing nested module");
    if (close(fd)<0 || fsync(parent)<0) die("sync competing module parent");
    return result;
}
static void preserved(int parent, const struct stat *identity) {
    struct stat actual;
    if (fstatat(parent,"mod.lkj",&actual,AT_SYMLINK_NOFOLLOW)<0) die("inspect competitor");
    need(S_ISREG(actual.st_mode) && actual.st_dev==identity->st_dev && actual.st_ino==identity->st_ino,
        "competing module was replaced");
    int fd=openat(parent,"mod.lkj",O_RDONLY|O_NOFOLLOW|O_CLOEXEC);
    if (fd<0) die("open competitor");
    char bytes[sizeof(competitor)]; size_t at=0;
    while (at<sizeof(bytes)) {
        ssize_t count=read(fd,bytes+at,sizeof(bytes)-at);
        if (count<0 && errno==EINTR) continue;
        if (count<0) die("read competitor");
        if (!count) break;
        at+=(size_t)count;
    }
    need(at==sizeof(competitor)-1 && memcmp(bytes,competitor,at)==0,"competing module bytes changed");
    if (close(fd)<0) die("close competitor");
}
int main(int argc, char **argv) {
    need(argc>=3,"usage: structure-alternate-race EXISTING_ADDED_DIRECTORY COMPILER [ARGS...]");
    int parent=open(argv[1],O_RDONLY|O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC);
    if (parent<0) die("pin alternate module parent");
    struct stat parent_identity, competitor_identity={0}, existing;
    if (fstat(parent,&parent_identity)<0) die("identify alternate parent");
    errno=0;
    need(fstatat(parent,"mod.lkj",&existing,AT_SYMLINK_NOFOLLOW)<0 && errno==ENOENT,
        "alternate must be absent before tracing");
    atexit(cleanup);
    struct sigaction action={.sa_handler=stop};
    sigemptyset(&action.sa_mask);
    sigaction(SIGINT,&action,NULL); sigaction(SIGTERM,&action,NULL);
    sigaction(SIGHUP,&action,NULL); sigaction(SIGALRM,&action,NULL);
    alarm(60);
    pid_t compiler=fork();
    if (compiler<0) die("fork compiler");
    if (!compiler) {
        if (ptrace(PTRACE_TRACEME,0,0,0)<0) _exit(127);
        raise(SIGSTOP);
        execvp(argv[2],argv+2); _exit(127);
    }
    remember(compiler);
    int status;
    if (waitpid(compiler,&status,0)!=compiler) die("initial compiler wait");
    need(WIFSTOPPED(status),"compiler did not enter tracing");
    long options=PTRACE_O_TRACESYSGOOD|PTRACE_O_TRACEEXEC|PTRACE_O_TRACEFORK|
        PTRACE_O_TRACEVFORK|PTRACE_O_TRACECLONE|PTRACE_O_EXITKILL;
    if (ptrace(PTRACE_SETOPTIONS,compiler,0,(void *)options)<0) die("set compiler tracing");
    resume(compiler,0);
    unsigned triggers=0; int compiler_status=-1;
    while (live) {
        need(!interrupted,"watchdog or signal interrupted compiler");
        pid_t who=waitpid(-1,&status,__WALL);
        if (who<0 && errno==EINTR) continue;
        if (who<0) die("wait for compiler syscall");
        if (WIFSTOPPED(status)) remember(who);
        size_t index=slot(who);
        if (WIFEXITED(status)||WIFSIGNALED(status)) {
            need(WIFEXITED(status),"compiler or owned child died by signal");
            if (who==compiler) compiler_status=WEXITSTATUS(status);
            else need(WEXITSTATUS(status)==0,"owned compiler child failed");
            owned[index].pid=0; owned[index].pending=0; live--; continue;
        }
        need(WIFSTOPPED(status),"unexpected traced state");
        unsigned event=(unsigned)status>>16;
        if (event==PTRACE_EVENT_FORK||event==PTRACE_EVENT_VFORK||event==PTRACE_EVENT_CLONE) {
            unsigned long child=0;
            if (ptrace(PTRACE_GETEVENTMSG,who,0,&child)<0) die("read compiler descendant");
            remember((pid_t)child);
        }
        int signal=WSTOPSIG(status);
        if (signal==(SIGTRAP|0x80)) {
            struct __ptrace_syscall_info info={0};
            if (ptrace(PTRACE_GET_SYSCALL_INFO,who,(void *)sizeof(info),&info)<0) die("inspect compiler syscall");
            if (info.op==PTRACE_SYSCALL_INFO_ENTRY) {
                owned[index].pending=0;
                if (!triggers && info.entry.nr==SYS_openat &&
                    (info.entry.args[2] & (O_PATH|O_NOFOLLOW))==(O_PATH|O_NOFOLLOW) &&
                    basename_is(who,info.entry.args[1],"mod.lkj") &&
                    same_parent(who,info.entry.args[0],&parent_identity)) {
                    owned[index].pending=1;
                }
            } else if (info.op==PTRACE_SYSCALL_INFO_EXIT && owned[index].pending) {
                need(info.exit.rval==-ENOENT,"alternate probe did not return the original ENOENT");
                need(!triggers,"alternate race injected more than once");
                /* The child remains stopped with its original ENOENT intact. */
                competitor_identity=inject(parent);
                triggers++; owned[index].pending=0;
            }
            signal=0;
        } else if (signal==SIGSTOP||signal==SIGTRAP) signal=0;
        resume(who,signal);
    }
    alarm(0);
    need(triggers==1,"the exact negative alternate probe was never exercised");
    need(compiler_status==1,"compiler did not refuse the transaction");
    preserved(parent,&competitor_identity);
    if (close(parent)<0) die("close alternate parent");
    printf("ok alternate race: triggers=1 result=-ENOENT preserved=true compiler_status=1\n");
    return 0;
}
