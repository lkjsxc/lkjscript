/* Optional independent Linux witness. Only this program's descendants are traced.
 * ptrace(2): https://man7.org/linux/man-pages/man2/ptrace.2.html
 * unix(7):   https://man7.org/linux/man-pages/man7/unix.7.html
 * This is never a production host or an admission proof. */
#define _GNU_SOURCE
#include <sys/ptrace.h>
#include <sys/prctl.h>
#include <sys/socket.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <time.h>
#include <poll.h>
#include <signal.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#ifndef MFD_EXEC
#define MFD_EXEC 0x0010U
#endif

struct owned { pid_t pid; int live; };
static struct owned *owned;
static size_t owned_count;
static volatile sig_atomic_t interrupted;
static void signal_stop(int number) { interrupted=number; }
static void remember(pid_t pid) {
    struct owned *next=realloc(owned,(owned_count+1)*sizeof(*owned));
    if(!next) { kill(pid,SIGKILL); perror("realloc child list"); exit(1); }
    owned=next; owned[owned_count++]=(struct owned){pid,1};
}
static void reaped(pid_t pid) {
    for(size_t i=0;i<owned_count;i++) if(owned[i].pid==pid) owned[i].live=0;
}
static void cleanup(void) {
    for(size_t i=0;i<owned_count;i++) if(owned[i].live) {
        kill(owned[i].pid,SIGKILL);
        (void)ptrace(PTRACE_CONT,owned[i].pid,0,0);
    }
    for(;;) {
        int status; pid_t pid=waitpid(-1,&status,__WALL);
        if(pid>0) { if(WIFSTOPPED(status)) { kill(pid,SIGKILL); (void)ptrace(PTRACE_CONT,pid,0,0); } else reaped(pid); continue; }
        if(pid<0&&errno==EINTR) continue;
        break;
    }
    free(owned); owned=NULL; owned_count=0;
}
static void die(const char *what) { perror(what); exit(1); }
static void require(int truth,const char *message) {
    if(!truth) { fprintf(stderr,"process witness: %s\n",message); exit(1); }
}
static uint64_t now(void) {
    struct timespec time; if(clock_gettime(CLOCK_MONOTONIC,&time)<0) die("clock");
    return (uint64_t)time.tv_sec*1000000000u+(uint64_t)time.tv_nsec;
}
static void pause_tick(void) {
    struct timespec delay={0,1000000}; nanosleep(&delay,NULL);
}
static void all(int fd,const void *bytes,size_t length) {
    const unsigned char *data=bytes;
    while(length) { ssize_t count=write(fd,data,length); if(count<0&&errno==EINTR) continue; if(count<=0) die("write"); data+=count; length-=(size_t)count; }
}
static int high(int fd) {
    if(fd<0) die("open descriptor");
    int moved=fcntl(fd,F_DUPFD_CLOEXEC,128); if(moved<0) die("promote");
    close(fd); return moved;
}
static int memory(const char *name,const unsigned char *bytes,size_t length,int executable,int sealed) {
    int fd=high(memfd_create(name,MFD_CLOEXEC|MFD_ALLOW_SEALING|(executable?MFD_EXEC:0)));
    all(fd,bytes,length);
    if(executable&&fchmod(fd,0755)<0) die("chmod image");
    if(sealed&&fcntl(fd,F_ADD_SEALS,F_SEAL_SEAL|F_SEAL_SHRINK|F_SEAL_GROW|F_SEAL_WRITE)<0) die("seal");
    return fd;
}
static int load(const char *path,int executable) {
    int input=open(path,O_RDONLY|O_CLOEXEC); if(input<0) die("open input");
    struct stat info; if(fstat(input,&info)<0||info.st_size<0) die("input stat");
    size_t length=(size_t)info.st_size; unsigned char *data=malloc(length?length:1); if(!data) die("input allocation");
    size_t at=0;
    while(at<length) { ssize_t count=read(input,data+at,length-at); if(count<0&&errno==EINTR) continue; if(count<=0) die("input read"); at+=(size_t)count; }
    close(input); int fd=memory("programs-witness-input",data,length,executable,1); free(data); return fd;
}
static void install(int source,int target) { if(dup3(source,target,0)<0) die("install"); }
static unsigned fd_count(pid_t pid) {
    char path[80]; snprintf(path,sizeof(path),"/proc/%ld/fd",(long)pid);
    DIR *directory=opendir(path); if(!directory) die("child descriptors");
    unsigned count=0; struct dirent *entry;
    while((entry=readdir(directory))) if(entry->d_name[0]!='.') count++;
    closedir(directory); return count;
}
static void wait_packet(int fd,unsigned char expected) {
    uint64_t deadline=now()+5000000000ull;
    for(;;) {
        require(!interrupted&&now()<deadline,"channel reply watchdog expired");
        struct pollfd poller={fd,POLLIN,0}; int ready=poll(&poller,1,10);
        if(ready<0&&errno==EINTR) continue;
        if(ready<0) die("reply poll");
        if(!ready) continue;
        unsigned char answer[8]; ssize_t count=recv(fd,answer,sizeof(answer),0);
        if(count<0&&(errno==EINTR||errno==EAGAIN)) continue;
        require(count==1&&answer[0]==expected,"wrong channel receiver result");
        return;
    }
}
static void send_rights(int fd,int file,unsigned count,unsigned char marker) {
    int descriptors[4]={file,file,file,file};
    union { struct cmsghdr alignment; unsigned char bytes[CMSG_SPACE(sizeof(descriptors))]; } control;
    memset(&control,0,sizeof(control));
    struct iovec iov={&marker,1};
    struct msghdr message={0}; message.msg_iov=&iov; message.msg_iovlen=1;
    if(count) {
        message.msg_control=control.bytes; message.msg_controllen=CMSG_SPACE(count*sizeof(int));
        struct cmsghdr *header=CMSG_FIRSTHDR(&message);
        header->cmsg_level=SOL_SOCKET; header->cmsg_type=SCM_RIGHTS; header->cmsg_len=CMSG_LEN(count*sizeof(int));
        memcpy(CMSG_DATA(header),descriptors,count*sizeof(int));
    }
    require(sendmsg(fd,&message,MSG_NOSIGNAL)==1,"send ancillary fixture");
}
static unsigned hex_digit(char digit) {
    if(digit>='0'&&digit<='9') return (unsigned)(digit-'0');
    if(digit>='a'&&digit<='f') return (unsigned)(digit-'a'+10);
    require(0,"invalid manifest hex"); return 0;
}
static int channel_packet(const char *manifest) {
    require(strlen(manifest)==64,"manifest identity must be 32 bytes");
    unsigned char bytes[72]={0};
    memcpy(bytes,"LKJCHD1",7);
    for(size_t i=0;i<32;i++) bytes[8+i]=(unsigned char)((hex_digit(manifest[2*i])<<4)|hex_digit(manifest[2*i+1]));
    bytes[40]=1; bytes[48]=17; /* one descriptor, Channel ABI kind 17 */
    return memory("programs-witness-packet",bytes,sizeof(bytes),0,1);
}
static void ancillary(const char *image,const char *manifest) {
    int executable=load(image,1), transfer=channel_packet(manifest);
    int result=memory("programs-witness-result",(const unsigned char *)"",0,0,0);
    int pair[2]; if(socketpair(AF_UNIX,SOCK_SEQPACKET|SOCK_CLOEXEC|SOCK_NONBLOCK,0,pair)<0) die("channel pair");
    pair[0]=high(pair[0]); pair[1]=high(pair[1]);
    pid_t child=fork(); if(child<0) die("receiver fork");
    if(child==0) {
        install(result,3); install(transfer,4); install(pair[1],8);
        char *arguments[]={"receiver",NULL}; char *environment[]={NULL};
        syscall(SYS_execveat,executable,"",arguments,environment,AT_EMPTY_PATH);
        _exit(127);
    }
    remember(child);
    wait_packet(pair[0],'R');
    unsigned baseline=fd_count(child);
    unsigned char value=42;
    int file=memory("programs-rights-sentinel",&value,1,0,0);
    require(lseek(file,0,SEEK_SET)==0,"rewind rights fixture");
    const unsigned counts[]={0,2,4,1};
    const unsigned char markers[]={0xf1,0xf1,0xf1,0xaa};
    for(size_t i=0;i<sizeof(counts)/sizeof(counts[0]);i++) {
        send_rights(pair[0],file,counts[i],markers[i]);
        wait_packet(pair[0],EPROTO);
        require(fd_count(child)==baseline,"malformed ancillary leaked or closed a descriptor");
    }
    int enabled=1;
    if(setsockopt(pair[1],SOL_SOCKET,SO_PASSCRED,&enabled,sizeof(enabled))<0) die("enable credentials");
    send_rights(pair[0],file,0,0xf1);
    wait_packet(pair[0],EPROTO);
    require(fd_count(child)==baseline,"credential control leaked or closed a descriptor");
    enabled=0;
    if(setsockopt(pair[1],SOL_SOCKET,SO_PASSCRED,&enabled,sizeof(enabled))<0) die("disable credentials");
    send_rights(pair[0],file,1,0xf1);
    wait_packet(pair[0],0);
    require(fd_count(child)==baseline,"valid received file was not closed after use");
    value=255; require(pwrite(file,&value,1,0)==1,"write termination file");
    require(lseek(file,0,SEEK_SET)==0,"rewind termination file");
    send_rights(pair[0],file,1,0xf1);
    wait_packet(pair[0],0);
    wait_packet(pair[0],'B');
    require(send(pair[0],"oversize",8,MSG_NOSIGNAL)==8,"send oversized byte packet");
    wait_packet(pair[0],EMSGSIZE);
    require(fd_count(child)==baseline,"oversized receive changed descriptor set");
    send_rights(pair[0],file,1,'x');
    wait_packet(pair[0],EMSGSIZE);
    require(fd_count(child)==baseline,"byte receive discarded rights without closing them");
    require(send(pair[0],"x",1,MSG_NOSIGNAL)==1,"send short byte packet");
    wait_packet(pair[0],0);
    require(send(pair[0],"done",4,MSG_NOSIGNAL)==4,"send byte terminator");
    wait_packet(pair[0],0);
    int status; if(waitpid(child,&status,0)!=child) die("receiver wait"); reaped(child);
    require(WIFEXITED(status)&&WEXITSTATUS(status)==0,"receiver did not finish successfully");
    unsigned char scalar[8]; require(pread(result,scalar,8,0)==8&&scalar[0]==42,"receiver private result missing");
    for(size_t i=1;i<sizeof(scalar);i++) require(scalar[i]==0,"unexpected receiver result");
    close(file); close(pair[0]); close(pair[1]); close(result); close(transfer); close(executable);
    printf("ok independent channel ancillary and bytes: malformed rights, credentials, truncation, untouched error buffers, zero-length refusal; no descriptor leak\n");
}
static void resume(pid_t pid,int signal) {
    if(ptrace(PTRACE_CONT,pid,0,(void *)(intptr_t)signal)<0&&errno!=ESRCH) die("ptrace continue");
}
static void traced(const char *mode,const char *program,const char *root) {
    int death=!strcmp(mode,"legacy-death"), ambient=!strcmp(mode,"ambient");
    pid_t parent=fork(); if(parent<0) die("trace fork");
    if(parent==0) {
        if(ambient) {
            int marker=memfd_create("programs-ambient-sentinel",0);
            if(marker<0||dup2(marker,200)<0) _exit(127);
            if(marker!=200) close(marker);
        }
        if(ptrace(PTRACE_TRACEME,0,0,0)<0) _exit(127);
        raise(SIGSTOP);
        char *arguments[]={(char *)program,(char *)root,NULL};
        execv(program,arguments); _exit(127);
    }
    remember(parent);
    int status; if(waitpid(parent,&status,0)!=parent) die("initial trace wait");
    require(WIFSTOPPED(status),"fixture did not stop for tracing");
    long options=PTRACE_O_TRACEFORK|PTRACE_O_TRACEVFORK|PTRACE_O_TRACECLONE|PTRACE_O_TRACEEXEC|PTRACE_O_EXITKILL;
    if(ptrace(PTRACE_SETOPTIONS,parent,0,(void *)options)<0) die("ptrace options");
    resume(parent,0);
    pid_t worker=0;
    int parent_done=0,worker_done=0,held=0,exec_seen=0,parent_status=0,worker_status=0,sentinel_seen=0;
    uint64_t deadline=now()+10000000000ull;
    while(!parent_done||!worker_done) {
        require(!interrupted&&now()<deadline,"traced fixture watchdog expired");
        pid_t who=waitpid(-1,&status,__WALL|WNOHANG);
        if(who<0&&errno==EINTR) continue;
        if(who<0) die("trace wait");
        if(who==0) { pause_tick(); continue; }
        if(WIFEXITED(status)||WIFSIGNALED(status)) {
            reaped(who);
            if(who==parent) {
                parent_done=1; parent_status=status;
                if(death&&held&&!worker_done) { resume(worker,0); held=0; }
            } else if(who==worker) { worker_done=1; worker_status=status; }
            continue;
        }
        require(WIFSTOPPED(status),"unexpected tracing wait state");
        unsigned event=(unsigned)status>>16;
        if(event==PTRACE_EVENT_FORK||event==PTRACE_EVENT_VFORK||event==PTRACE_EVENT_CLONE) {
            unsigned long child=0;
            if(ptrace(PTRACE_GETEVENTMSG,who,0,&child)<0) die("fork event child");
            remember((pid_t)child);
            require(who==parent&&!worker,"fixture created an unexpected descendant");
            worker=(pid_t)child;
            if(ambient) {
                char path[80]; snprintf(path,sizeof(path),"/proc/%ld/fd/200",(long)parent);
                require(access(path,F_OK)==0,"ambient sentinel was absent before Process spawn");
                sentinel_seen=1;
            }
            if(death) { require(kill(parent,SIGKILL)==0,"kill original parent"); }
            else resume(parent,0);
            continue;
        }
        if(who==worker&&event==PTRACE_EVENT_EXEC) {
            exec_seen=1;
            if(!ambient) { kill(worker,SIGKILL); require(0,"stopped child reached exec"); }
        }
        if(who==worker&&!ambient&&!exec_seen) {
            if(death&&parent_done) resume(worker,0);
            else held=1;
        } else {
            int signal=WSTOPSIG(status);
            if(signal==SIGSTOP||signal==SIGTRAP) signal=0;
            resume(who,signal);
        }
    }
    require(worker!=0,"fixture did not fork its worker");
    if(death) {
        require(WIFSIGNALED(parent_status)&&WTERMSIG(parent_status)==SIGKILL,"original parent was not killed");
        require(!exec_seen,"orphan child executed user code");
        require(WIFEXITED(worker_status)&&WEXITSTATUS(worker_status)!=0,"orphan child did not reject stale parent");
        printf("ok independent legacy pre-prctl parent-death race: child rejected before exec; reaped\n");
    } else if(ambient) {
        require(sentinel_seen&&exec_seen,"ambient test did not exercise generic child exec");
        require(WIFEXITED(parent_status)&&WEXITSTATUS(parent_status)==0,"parent failed ambient descriptor proof");
        require(WIFEXITED(worker_status)&&WEXITSTATUS(worker_status)==0,"generic child received ambient descriptor");
        printf("ok independent generic Process child: high ambient descriptor closed\n");
    } else {
        require(!exec_seen,"pre-exec deadline did not contain child");
        require(WIFEXITED(parent_status)&&WEXITSTATUS(parent_status)==0,"spawn did not return timeout to parent");
        require(WIFSIGNALED(worker_status)&&WTERMSIG(worker_status)==SIGKILL,"stopped child was not killed at deadline");
        printf("ok independent Image spawn deadline: child held before exec, killed and reaped\n");
    }
}
int main(int argc,char **argv) {
    if(argc==2&&!strcmp(argv[1],"--oracle-body")) {
        errno=0;
        if(fcntl(200,F_GETFD)!=-1||errno!=EBADF) return 91;
        all(STDOUT_FILENO,"body executed\n",14);
        return 0;
    }
    if(argc!=4) {
        fprintf(stderr,"usage: witness ancillary IMAGE MANIFEST_HEX | spawn-deadline PARENT ROOT | legacy-death PARENT ROOT | ambient PARENT ROOT\n");
        return 2;
    }
    if(prctl(PR_SET_CHILD_SUBREAPER,1)<0) die("subreaper");
    if(atexit(cleanup)!=0) die("atexit");
    signal(SIGINT,signal_stop); signal(SIGTERM,signal_stop);
    if(!strcmp(argv[1],"ancillary")) ancillary(argv[2],argv[3]);
    else if(!strcmp(argv[1],"spawn-deadline")||!strcmp(argv[1],"legacy-death")||!strcmp(argv[1],"ambient")) traced(argv[1],argv[2],argv[3]);
    else require(0,"unknown witness mode");
    return 0;
}
