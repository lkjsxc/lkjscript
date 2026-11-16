/* Independent Linux boundary witness for absent-only publication and retained reads.
 * Tracing is restricted to this invocation's children. No production host uses C. */
#define _GNU_SOURCE
#include <sys/ptrace.h>
#include <sys/prctl.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static pid_t owned[128];
static size_t live;
static volatile sig_atomic_t interrupted;
static void stop(int signal) { interrupted = signal; }
static void cleanup(void) {
    for (size_t i=0; i<128; i++) if (owned[i]) kill(owned[i], SIGKILL);
    while (waitpid(-1, NULL, __WALL)>0 || errno==EINTR) {}
}
static void die(const char *message) { perror(message); exit(1); }
static void need(int condition, const char *message) {
    if (!condition) { fprintf(stderr, "directory boundary witness: %s\n", message); exit(1); }
}
static void remember(pid_t pid) {
    for (size_t i=0; i<128; i++) if (owned[i]==pid) return;
    for (size_t i=0; i<128; i++) if (!owned[i]) { owned[i]=pid; live++; return; }
    need(0, "too many owned descendants");
}
static void reaped(pid_t pid) {
    for (size_t i=0; i<128; i++) if (owned[i]==pid) { owned[i]=0; live--; return; }
    need(0, "reaped an untracked process");
}
static char *path(const char *directory, const char *name) {
    char *value;
    if (asprintf(&value, "%s/%s", directory, name)<0) die("asprintf");
    return value;
}
static struct stat create(const char *name, const char *bytes) {
    int fd=open(name,O_WRONLY|O_CREAT|O_EXCL|O_CLOEXEC,0600);
    if (fd<0) die("create competing entry");
    size_t at=0, length=strlen(bytes);
    while (at<length) {
        ssize_t n=write(fd,bytes+at,length-at);
        if (n<0 && errno==EINTR) continue;
        if (n<=0) die("write competing entry");
        at+=(size_t)n;
    }
    struct stat result;
    if (fstat(fd,&result)<0) die("fstat competing entry");
    if (close(fd)<0) die("close competing entry");
    return result;
}
static void contents(const char *name, const char *expected, const struct stat *identity) {
    struct stat found;
    if (lstat(name,&found)<0) die("lstat retained entry");
    need(S_ISREG(found.st_mode),"entry is no longer regular");
    if (identity) need(found.st_dev==identity->st_dev && found.st_ino==identity->st_ino,"entry identity changed");
    int fd=open(name,O_RDONLY|O_NOFOLLOW|O_CLOEXEC);
    if (fd<0) die("open retained entry");
    char bytes[128]; ssize_t n=read(fd,bytes,sizeof(bytes));
    if (n<0) die("read retained entry");
    need((size_t)n==strlen(expected) && memcmp(bytes,expected,(size_t)n)==0,"entry bytes changed");
    close(fd);
}
static void remote_name(pid_t pid, uint64_t address, char *buffer, size_t size) {
    size_t at=0;
    while (at<size) {
        errno=0;
        uintptr_t current=(uintptr_t)(address+at);
        uintptr_t aligned=current & ~(uintptr_t)(sizeof(long)-1);
        long word=ptrace(PTRACE_PEEKDATA,pid,(void *)aligned,0);
        if (word==-1 && errno) die("read syscall basename");
        size_t offset=(size_t)(current-aligned), take=sizeof(word)-offset;
        if (take>size-at) take=size-at;
        memcpy(buffer+at,(const char *)&word+offset,take);
        for (size_t i=0;i<take;i++) if (buffer[at+i]==0) return;
        at+=take;
    }
    need(0,"syscall basename is unbounded");
}
static void resume(pid_t pid, int signal) {
    if (ptrace(PTRACE_SYSCALL,pid,0,(void *)(intptr_t)signal)<0 && errno!=ESRCH) die("resume traced process");
}
static void deny(int error, char **command) {
    need(error>0 && error<4096,"invalid injected errno");
    struct sock_filter code[]={
        BPF_STMT(BPF_LD|BPF_W|BPF_ABS,offsetof(struct seccomp_data,nr)),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K,SYS_renameat2,0,1),
        BPF_STMT(BPF_RET|BPF_K,SECCOMP_RET_ERRNO|(unsigned)error),
        BPF_STMT(BPF_RET|BPF_K,SECCOMP_RET_ALLOW),
    };
    struct sock_fprog program={.len=(unsigned short)(sizeof(code)/sizeof(code[0])),.filter=code};
    if (prctl(PR_SET_NO_NEW_PRIVS,1,0,0,0)<0 || prctl(PR_SET_SECCOMP,SECCOMP_MODE_FILTER,&program)<0) die("install renameat2 refusal");
    execvp(command[0],command);
    die("exec refusal specimen");
}
static void trace(int pin, const char *directory, char **command) {
    atexit(cleanup);
    struct sigaction action={.sa_handler=stop};
    sigemptyset(&action.sa_mask);
    sigaction(SIGINT,&action,NULL); sigaction(SIGTERM,&action,NULL);
    sigaction(SIGHUP,&action,NULL); sigaction(SIGALRM,&action,NULL);
    alarm(60);
    pid_t parent=fork();
    if (parent<0) die("fork traced specimen");
    if (!parent) {
        if (ptrace(PTRACE_TRACEME,0,0,0)<0) _exit(127);
        raise(SIGSTOP);
        execvp(command[0],command); _exit(127);
    }
    remember(parent);
    int status;
    if (waitpid(parent,&status,0)!=parent) die("initial trace wait");
    need(WIFSTOPPED(status),"specimen did not enter tracing");
    long options=PTRACE_O_TRACEFORK|PTRACE_O_TRACEVFORK|PTRACE_O_TRACECLONE|
        PTRACE_O_TRACEEXEC|PTRACE_O_EXITKILL|PTRACE_O_TRACESYSGOOD;
    if (ptrace(PTRACE_SETOPTIONS,parent,0,(void *)options)<0) die("set trace options");
    resume(parent,0);
    unsigned hits=0;
    pid_t pending=0;
    struct stat winner={0}, retained={0};
    char *destination=path(directory,pin?"observed":"winner");
    char *source=path(directory,pin?"retained":"source");
    while (live) {
        need(!interrupted,"tracing watchdog or signal interrupted specimen");
        pid_t who=waitpid(-1,&status,__WALL);
        if (who<0 && errno==EINTR) continue;
        if (who<0) die("trace wait");
        if (WIFEXITED(status)||WIFSIGNALED(status)) {
            reaped(who);
            need(WIFEXITED(status)&&WEXITSTATUS(status)==0,"specimen/owned broker failed");
            continue;
        }
        need(WIFSTOPPED(status),"unexpected process state");
        unsigned event=(unsigned)status>>16;
        if (event==PTRACE_EVENT_FORK||event==PTRACE_EVENT_VFORK||event==PTRACE_EVENT_CLONE) {
            unsigned long child=0;
            if (ptrace(PTRACE_GETEVENTMSG,who,0,&child)<0) die("read traced child");
            remember((pid_t)child);
        }
        int signal=WSTOPSIG(status);
        if (signal==(SIGTRAP|0x80)) {
            struct __ptrace_syscall_info info={0};
            if (ptrace(PTRACE_GET_SYSCALL_INFO,who,(void *)sizeof(info),&info)<0) die("read syscall stop");
            if (!pin && info.op==PTRACE_SYSCALL_INFO_ENTRY && info.entry.nr==SYS_renameat2) {
                char old[64], next[64];
                remote_name(who,info.entry.args[1],old,sizeof(old));
                remote_name(who,info.entry.args[3],next,sizeof(next));
                need(!strcmp(old,"source")&&!strcmp(next,"winner"),"unexpected rename names");
                need(info.entry.args[0]==info.entry.args[2]&&info.entry.args[4]==1,"rename is not same-parent RENAME_NOREPLACE");
                need(hits++==0,"more than one rename syscall");
                winner=create(destination,"competing\n");
            } else if (pin && info.op==PTRACE_SYSCALL_INFO_ENTRY && info.entry.nr==SYS_openat &&
                info.entry.args[2]==(O_PATH|O_NOFOLLOW|O_CLOEXEC)) {
                char name[64];
                remote_name(who,info.entry.args[1],name,sizeof(name));
                if (!strcmp(name,"observed")) {
                    need(!pending && hits==0,"more than one observed inode acquisition");
                    pending=who;
                }
            } else if (pin && who==pending && info.op==PTRACE_SYSCALL_INFO_EXIT) {
                need(info.exit.rval>=0,"O_PATH acquisition failed before race");
                char descriptor[96];
                snprintf(descriptor,sizeof(descriptor),"/proc/%ld/fd/%lld",(long)who,(long long)info.exit.rval);
                if (stat(descriptor,&retained)<0) die("inspect acquired inode");
                need(S_ISREG(retained.st_mode),"acquired inode is not regular");
                if (rename(destination,source)<0) die("replace pathname after acquisition");
                winner=create(destination,"replacement\n");
                hits++; pending=0;
            }
            signal=0;
        } else if (signal==SIGSTOP||signal==SIGTRAP) signal=0;
        resume(who,signal);
    }
    alarm(0);
    need(hits==1&&!pending,"controlled syscall race was not exercised");
    contents(destination,pin?"replacement\n":"competing\n",&winner);
    contents(source,"original\n",pin?&retained:NULL);
    printf("ok directory boundary: %s\n",pin?"read retained inode after pathname replacement":"destination created at renameat2 entry was never overwritten");
    free(destination); free(source);
}
int main(int argc, char **argv) {
    need(argc>=4,"usage: witness deny ERRNO COMMAND... | rename-race/pin-race DIRECTORY COMMAND...");
    if (!strcmp(argv[1],"deny")) deny(atoi(argv[2]),argv+3);
    else if (!strcmp(argv[1],"rename-race")) trace(0,argv[2],argv+3);
    else if (!strcmp(argv[1],"pin-race")) trace(1,argv[2],argv+3);
    else need(0,"unknown witness mode");
    return 0;
}
