/* Optional independent Linux syscall witness; never a production execution host. */
#define _GNU_SOURCE
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <sys/socket.h>
#include <sys/epoll.h>
#include <sys/prctl.h>
#include <linux/seccomp.h>
#include <linux/filter.h>
#include <stddef.h>
#include <netinet/in.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifndef MFD_EXEC
#define MFD_EXEC 0x0010U
#endif
static void die(const char *what) { perror(what); exit(1); }
struct bytes { unsigned char *p; size_t n; };
static struct bytes load(const char *path) {
    int f=open(path,O_RDONLY|O_CLOEXEC); if(f<0) die("open");
    struct stat s; if(fstat(f,&s)<0||s.st_size<0) die("stat");
    struct bytes b={malloc((size_t)s.st_size+1),(size_t)s.st_size}; if(!b.p) die("malloc");
    size_t at=0; while(at<b.n) { ssize_t n=read(f,b.p+at,b.n-at); if(n<=0) die("read"); at+=(size_t)n; }
    close(f); return b;
}
static void all(int f,const void *data,size_t n) {
    const unsigned char *p=data; while(n) { ssize_t k=write(f,p,n); if(k<=0) die("write"); p+=k; n-=k; }
}
static uint64_t word(const unsigned char *p) { uint64_t n=0; for(unsigned i=0;i<8;i++) n|=(uint64_t)p[i]<<(i*8); return n; }
static void put(unsigned char *p,uint64_t n) { for(unsigned i=0;i<8;i++) p[i]=(unsigned char)(n>>(i*8)); }
static int high(int f) { if(f<0) die("descriptor"); int h=fcntl(f,F_DUPFD_CLOEXEC,128); if(h<0) die("high descriptor"); close(f); return h; }
static int memory(const char *name,const struct bytes b,int executable,int seal) {
    int f=high(memfd_create(name,MFD_CLOEXEC|MFD_ALLOW_SEALING|(executable?MFD_EXEC:0)));
    all(f,b.p,b.n); if(executable&&fchmod(f,0755)<0) die("chmod");
    int alias=dup(f); if(alias<0) die("alias");
    if(seal) {
        if(fcntl(f,F_ADD_SEALS,F_SEAL_SEAL|F_SEAL_SHRINK|F_SEAL_GROW|F_SEAL_WRITE)<0) die("seals");
        errno=0; if(pwrite(alias,"x",1,0)!=-1||errno!=EPERM) { fprintf(stderr,"writable alias survived seal\n"); exit(1); }
        errno=0; if(ftruncate(alias,(off_t)b.n+1)!=-1||errno!=EPERM) { fprintf(stderr,"growth alias survived seal\n"); exit(1); }
        errno=0; if(ftruncate(alias,0)!=-1||errno!=EPERM) { fprintf(stderr,"shrink alias survived seal\n"); exit(1); }
        void *map=mmap(NULL,b.n?b.n:1,PROT_READ|PROT_WRITE,MAP_SHARED,alias,0);
        if(map!=MAP_FAILED) { fprintf(stderr,"writable mapping survived seal\n"); exit(1); }
    }
    close(alias); return f;
}
static int listener(void) {
    int fd=socket(AF_INET,SOCK_STREAM|SOCK_CLOEXEC,0); if(fd<0) die("socket");
    struct sockaddr_in a={.sin_family=AF_INET,.sin_port=0,.sin_addr={.s_addr=htonl(INADDR_LOOPBACK)}};
    if(bind(fd,(void *)&a,sizeof(a))<0||listen(fd,2)<0) die("listen");
    return high(fd);
}
static void install(int old,int fd) { if(dup3(old,fd,0)<0) die("dup3"); }
static void deny_syscall(int number,int error) {
    struct sock_filter filter[]={
        BPF_STMT(BPF_LD|BPF_W|BPF_ABS,offsetof(struct seccomp_data,nr)),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K,(unsigned)number,0,1),
        BPF_STMT(BPF_RET|BPF_K,SECCOMP_RET_ERRNO|(unsigned)error),
        BPF_STMT(BPF_RET|BPF_K,SECCOMP_RET_ALLOW)
    };
    struct sock_fprog program={.len=4,.filter=filter};
    if(prctl(PR_SET_NO_NEW_PRIVS,1,0,0,0)<0||syscall(SYS_seccomp,SECCOMP_SET_MODE_FILTER,0,&program)<0) die("seccomp");
}
static void unchanged(int fd,const char *name) {
    struct stat s; if(fstat(fd,&s)<0||s.st_size!=0) { fprintf(stderr,"wrote undeclared %s\\n",name); exit(1); }
}
int main(int argc,char **argv) {
    if(argc!=5) { fprintf(stderr,"IMAGE PACKET ROOT MODE\n"); return 2; }
    struct bytes image=load(argv[1]), packet=load(argv[2]);
    const char *mode=argv[4]; uint64_t count=word(packet.p+40);
    if(count>64||packet.n<48+count*24) return 2;
    uint64_t kinds[64]; for(uint64_t i=0;i<count;i++) kinds[i]=word(packet.p+48+i*24);
    int image_fd=memory("admitted",image,1,1);
    unsigned char *verify=malloc(image.n); if(!verify) die("malloc");
    if(pread(image_fd,verify,image.n,0)!=(ssize_t)image.n||memcmp(verify,image.p,image.n)) die("snapshot identity");
    free(verify);
    if(!strcmp(mode,"replace-path")) {
        if(unlink(argv[1])<0) die("unlink original");
        int wrong=open(argv[1],O_CREAT|O_EXCL|O_WRONLY,0600); if(wrong<0) die("replace");
        all(wrong,"not the admitted executable",27); close(wrong);
    }
    if(!strcmp(mode,"magic")) packet.p[0]^=1;
    if(!strcmp(mode,"manifest")) packet.p[8]^=1;
    if(!strcmp(mode,"count")) put(packet.p+40,count+1);
    if(!strcmp(mode,"kind")) put(packet.p+48,999);
    if(!strcmp(mode,"alias-offset")) {
        int seen=0; uint64_t first=0;
        for(uint64_t i=0;i<count;i++) if(word(packet.p+48+i*24+8)) {
            if(!seen++) first=word(packet.p+48+i*24+8); else { put(packet.p+48+i*24+8,first); break; }
        }
    }
    if(!strcmp(mode,"overflow")) { put(packet.p+48+8,UINT64_MAX); put(packet.p+48+16,UINT64_MAX); }
    if(!strcmp(mode,"truncated")) packet.n--;
    if(!strcmp(mode,"trailing")) packet.p[packet.n++]=0;
    int packet_fd=memory("transfer",packet,0,strcmp(mode,"unsealed")!=0);
    struct bytes empty={(unsigned char *)"",0}; int result=memory("result",empty,0,0);
    int output=high(dup(1));
    int ambient_out=memory("ambient-stdout",empty,0,0), ambient_err=memory("ambient-stderr",empty,0,0);
    pid_t child=fork(); if(child<0) die("fork");
    if(child==0) {
        int marker=fcntl(output,F_DUPFD,200); if(marker<0) die("ambient marker");
        install(ambient_out,1); install(ambient_err,2);
        install(result,3); install(packet_fd,4);
        for(uint64_t i=0;i<count;i++) {
            uint64_t kind=kinds[i]; /* original kinds for malformed packet tests */
            int fd=-1;
            if(kind==8||kind==9) fd=high(open(!strcmp(mode,"proc")?"/proc/self/fd":argv[3],O_RDONLY|O_DIRECTORY|O_CLOEXEC));
            else if(kind==4||kind==6) fd=high(dup(output));
            else if(kind==5||kind==12) fd=high(open("/dev/null",O_RDWR|O_CLOEXEC));
            else if(kind==10||kind==11) {
                char path[4096]; if(snprintf(path,sizeof(path),"%s/inside.txt",argv[3])<0) _exit(1);
                fd=high(open(path,(kind==10?O_RDWR:O_RDONLY)|O_CLOEXEC));
            } else if(kind==13) fd=listener();
            else if(kind==15) fd=high(epoll_create1(EPOLL_CLOEXEC));
            if(fd>=0) { install(fd,(int)(8+i)); close(fd); }
        }
        if(!strcmp(mode,"wrong-descriptor")) { int f=high(open("/dev/null",O_RDWR|O_CLOEXEC)); install(f,8); close(f); }
        if(!strcmp(mode,"missing-descriptor")) close(8);
        if(!strcmp(mode,"packet-as-result")) install(packet_fd,3);
        if(!strcmp(mode,"image-as-packet")) install(image_fd,4);
        if(!strcmp(mode,"packet-as-descriptor")) install(packet_fd,8);
        for(uint64_t i=0;i<count;i++) {
            if((!strcmp(mode,"readfile-directory")&&kinds[i]==11)) {
                int fd=high(open(argv[3],O_RDONLY|O_DIRECTORY|O_CLOEXEC)); install(fd,(int)(8+i)); close(fd);
            }
            if((!strcmp(mode,"listener-file")&&kinds[i]==13)||(!strcmp(mode,"readiness-file")&&kinds[i]==15)) {
                install(packet_fd,(int)(8+i));
            }
        }
        if(!strcmp(mode,"unsealed-alias")) { int f=high(memfd_create("bad-packet",MFD_CLOEXEC|MFD_ALLOW_SEALING)); all(f,packet.p,packet.n); install(f,4); close(f); }
        char *arguments[]={"child","explicit",NULL}; char *environment[]={"LKJSCRIPT_AMBIENT=forbidden",NULL};
        if(!strcmp(mode,"deny-signals")) deny_syscall(SYS_rt_sigaction,EPERM);
        if(!strcmp(mode,"deny-mapping")) deny_syscall(SYS_mmap,ENOMEM);
        syscall(SYS_execveat,image_fd,"",arguments,environment,AT_EMPTY_PATH); die("execveat");
    }
    int status; if(waitpid(child,&status,0)!=child) die("wait");
    int okay=!strcmp(mode,"valid")||!strcmp(mode,"replace-path")||!strcmp(mode,"proc");
    if(!WIFEXITED(status)||WEXITSTATUS(status)!=(okay?0:126)) {
        fprintf(stderr,"unexpected child status mode=%s raw=%d\n",mode,status); return 1;
    }
    if(okay) { unsigned char value[8]; if(pread(result,value,8,0)!=8||word(value)!=37) { fprintf(stderr,"wrong result\n"); return 1; } }
    else { struct stat s; if(fstat(result,&s)<0||s.st_size!=0) { fprintf(stderr,"invalid child reached entry\n"); return 1; } }
    unchanged(ambient_out,"stdout"); unchanged(ambient_err,"stderr");
    close(image_fd);close(packet_fd);close(result);close(output);close(ambient_out);close(ambient_err);free(image.p);free(packet.p);
    return 0;
}
