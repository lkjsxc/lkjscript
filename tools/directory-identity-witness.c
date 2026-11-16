/* Independent fstat/held-directory oracle; only owned fixture descendants exist. */
#define _GNU_SOURCE
#include <sys/prctl.h>
#include <sys/ptrace.h>
#include <sys/user.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static pid_t child=-1;
static pid_t traced[128];
static int injected[128];
static size_t live;
static volatile sig_atomic_t interrupted;
static void stop(int signal) { interrupted=signal; }
static void die(const char *message) { perror(message); exit(1); }
static void need(int value,const char *message) {
    if (!value) { fprintf(stderr,"directory identity witness: %s\n",message); exit(1); }
}
static void cleanup(void) {
    for (size_t i=0;i<128;i++) if (traced[i]) kill(traced[i],SIGKILL);
    if (child>0) {
        kill(child,SIGKILL);
        while (waitpid(child,NULL,0)<0 && errno==EINTR) {}
        child=-1;
    }
    char path[96];
    snprintf(path,sizeof(path),"/proc/self/task/%ld/children",(long)getpid());
    FILE *file=fopen(path,"r");
    if (file) {
        long pid;
        while (fscanf(file,"%ld",&pid)==1) if (pid>0) kill((pid_t)pid,SIGKILL);
        fclose(file);
    }
    while (waitpid(-1,NULL,__WALL)>0 || errno==EINTR) {}
}
static void exact(int fd,void *buffer,size_t length,int writing) {
    unsigned char *bytes=buffer;
    size_t at=0;
    while (at<length) {
        need(!interrupted,"watchdog or signal");
        ssize_t count=writing?write(fd,bytes+at,length-at):read(fd,bytes+at,length-at);
        if (count<0 && errno==EINTR) continue;
        if (count<=0) die(writing?"write fixture protocol":"read fixture protocol");
        at+=(size_t)count;
    }
}
static void identity(const char *path,unsigned char bytes[16]) {
    int fd=open(path,O_PATH|O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC);
    if (fd<0) die("open independent directory");
    struct stat value;
    if (fstat(fd,&value)<0) die("independent fstat");
    close(fd);
    uint64_t words[2]={(uint64_t)value.st_dev,(uint64_t)value.st_ino};
    for (size_t word=0;word<2;word++)
        for (size_t byte=0;byte<8;byte++) bytes[word*8+byte]=(unsigned char)(words[word]>>(byte*8));
}
static size_t remember(pid_t who) {
    for (size_t i=0;i<128;i++) if (traced[i]==who) return i;
    for (size_t i=0;i<128;i++) if (!traced[i]) { traced[i]=who;live++;return i; }
    need(0,"too many owned traced descendants");return 0;
}
static void resume(pid_t who,int signal) {
    if (ptrace(PTRACE_SYSCALL,who,0,(void *)(intptr_t)signal)<0 && errno!=ESRCH) die("resume fault trace");
}
static int same_directory(pid_t who,uint64_t descriptor,const struct stat *expected) {
    if (descriptor>INT32_MAX) return 0;
    char path[96];struct stat found;
    snprintf(path,sizeof(path),"/proc/%ld/fd/%llu",(long)who,(unsigned long long)descriptor);
    return stat(path,&found)==0 && S_ISDIR(found.st_mode)
        && found.st_dev==expected->st_dev && found.st_ino==expected->st_ino;
}
static int empty_remote(pid_t who,uint64_t address) {
    errno=0;
    long word=ptrace(PTRACE_PEEKDATA,who,(void *)(uintptr_t)address,0);
    if (word==-1 && errno) die("read metadata empty path");
    return (word&255)==0;
}
static void denied(const char *root,char **command) {
    struct stat expected;
    if (stat(root,&expected)<0) die("stat fault fixture directory");
    atexit(cleanup);
    struct sigaction action={.sa_handler=stop};
    sigemptyset(&action.sa_mask);
    sigaction(SIGINT,&action,NULL);sigaction(SIGTERM,&action,NULL);
    sigaction(SIGHUP,&action,NULL);sigaction(SIGALRM,&action,NULL);
    alarm(60);
    pid_t parent=fork();
    if (parent<0) die("fork metadata fault fixture");
    if (!parent) {
        if (ptrace(PTRACE_TRACEME,0,0,0)<0) _exit(127);
        raise(SIGSTOP);execvp(command[0],command);_exit(127);
    }
    remember(parent);
    int status;
    if (waitpid(parent,&status,0)!=parent) die("initial fault trace wait");
    need(WIFSTOPPED(status),"fault specimen did not stop");
    long options=PTRACE_O_TRACEFORK|PTRACE_O_TRACEVFORK|PTRACE_O_TRACECLONE|
        PTRACE_O_TRACEEXEC|PTRACE_O_EXITKILL|PTRACE_O_TRACESYSGOOD;
    if (ptrace(PTRACE_SETOPTIONS,parent,0,(void *)options)<0) die("set fault trace options");
    resume(parent,0);
    unsigned hits=0;
    while (live) {
        need(!interrupted,"fault tracing watchdog or signal");
        pid_t who=waitpid(-1,&status,__WALL);
        if (who<0 && errno==EINTR) continue;
        if (who<0) die("fault trace wait");
        size_t slot=remember(who);
        if (WIFEXITED(status)||WIFSIGNALED(status)) {
            need(!injected[slot],"pending injected fault at process exit");
            traced[slot]=0;live--;
            need(WIFEXITED(status)&&WEXITSTATUS(status)==0,"fault specimen or broker failed");
            continue;
        }
        need(WIFSTOPPED(status),"unexpected fault process state");
        unsigned event=(unsigned)status>>16;
        if (event==PTRACE_EVENT_FORK||event==PTRACE_EVENT_VFORK||event==PTRACE_EVENT_CLONE) {
            unsigned long next=0;
            if (ptrace(PTRACE_GETEVENTMSG,who,0,&next)<0) die("read fault trace child");
            remember((pid_t)next);
        }
        int signal=WSTOPSIG(status);
        if (signal==(SIGTRAP|0x80)) {
            struct __ptrace_syscall_info info={0};
            if (ptrace(PTRACE_GET_SYSCALL_INFO,who,(void *)sizeof(info),&info)<0) die("read fault syscall");
            if (info.op==PTRACE_SYSCALL_INFO_ENTRY) {
                int target=info.entry.nr==SYS_fstat;
                if (info.entry.nr==SYS_newfstatat || info.entry.nr==SYS_statx) {
                    uint64_t flags=info.entry.args[info.entry.nr==SYS_statx?2:3];
                    target=(flags&AT_EMPTY_PATH) && empty_remote(who,info.entry.args[1]);
                }
                if (target && same_directory(who,info.entry.args[0],&expected)) {
                    struct user_regs_struct regs;
                    if (ptrace(PTRACE_GETREGS,who,0,&regs)<0) die("read fault entry registers");
                    regs.orig_rax=(unsigned long long)-1;
                    if (ptrace(PTRACE_SETREGS,who,0,&regs)<0) die("suppress target metadata syscall");
                    need(!injected[slot],"nested metadata fault");injected[slot]=1;hits++;
                }
            } else if (info.op==PTRACE_SYSCALL_INFO_EXIT && injected[slot]) {
                struct user_regs_struct regs;
                if (ptrace(PTRACE_GETREGS,who,0,&regs)<0) die("read fault exit registers");
                regs.rax=(unsigned long long)-EIO;
                if (ptrace(PTRACE_SETREGS,who,0,&regs)<0) die("set target metadata error");
                injected[slot]=0;
            }
            signal=0;
        } else if (signal==SIGSTOP||signal==SIGTRAP) signal=0;
        resume(who,signal);
    }
    alarm(0);
    need(hits==2,"raw/helper calls did not each exercise one held-directory metadata error");
}
static void race(const char *root,char **command) {
    if (prctl(PR_SET_CHILD_SUBREAPER,1,0,0,0)<0) die("subreaper");
    atexit(cleanup);
    struct sigaction action={.sa_handler=stop};
    sigemptyset(&action.sa_mask);
    sigaction(SIGINT,&action,NULL);sigaction(SIGTERM,&action,NULL);
    sigaction(SIGHUP,&action,NULL);sigaction(SIGALRM,&action,NULL);
    signal(SIGPIPE,SIG_IGN);alarm(60);
    unsigned char expected[16],first[16],second[16],replacement[16];
    identity(root,expected);
    int input[2],output[2];
    if (pipe2(input,O_CLOEXEC)<0 || pipe2(output,O_CLOEXEC)<0) die("fixture pipes");
    child=fork();
    if (child<0) die("fork identity fixture");
    if (!child) {
        if (dup2(input[0],0)<0 || dup2(output[1],1)<0) _exit(127);
        close(input[0]);close(input[1]);close(output[0]);close(output[1]);
        execvp(command[0],command);_exit(127);
    }
    close(input[0]);close(output[1]);
    unsigned char marker='n';
    exact(input[1],&marker,1,1);
    exact(output[0],first,sizeof(first),0);
    need(!memcmp(first,expected,16),"first identity differs from independent fstat bytes");
    char *retained;
    if (asprintf(&retained,"%s-retained",root)<0) die("retained path");
    need(access(retained,F_OK)!=0 && errno==ENOENT,"retained destination already exists");
    if (syscall(SYS_renameat2,AT_FDCWD,root,AT_FDCWD,retained,1)<0) die("move held directory path");
    if (mkdir(root,0700)<0) die("replace directory path");
    identity(root,replacement);
    need(memcmp(replacement,expected,16)!=0,"replacement directory reused held identity");
    marker='x';exact(input[1],&marker,1,1);close(input[1]);
    exact(output[0],second,sizeof(second),0);
    need(!memcmp(second,expected,16),"held Directory followed its replaced pathname");
    unsigned char extra;
    ssize_t count;
    do { count=read(output[0],&extra,1); } while (count<0 && errno==EINTR && !interrupted);
    need(count==0,"identity fixture emitted extra output");
    close(output[0]);
    int status;
    pid_t waited;
    do { waited=waitpid(child,&status,0); } while (waited<0 && errno==EINTR && !interrupted);
    need(waited==child && WIFEXITED(status) && WEXITSTATUS(status)==0,"identity fixture failed");
    child=-1;alarm(0);
    printf("ok directory identity: exact dev/ino, same-inode alias and retained parent after pathname replacement\n");
    free(retained);
}
int main(int argc,char **argv) {
    need(argc>=4,"usage: witness deny/race DIRECTORY COMMAND...");
    if (!strcmp(argv[1],"deny")) denied(argv[2],argv+3);
    else if (!strcmp(argv[1],"race")) { need(argc>=4,"race command missing");race(argv[2],argv+3); }
    else need(0,"unknown witness mode");
    return 0;
}
