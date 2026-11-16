#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>
/* Test adversary only: never linked into an lkjscript runtime. */
static void demand(int ok) { if (!ok) _exit(99); }
static void output(const char *bytes, size_t count) { while(count) { ssize_t n=write(1,bytes,count); if(n<0 && errno==EINTR) continue; demand(n>0); bytes+=n; count-=(size_t)n; } }
int main(int argc,char **argv) {
    demand(argc>=2);
    if(!strcmp(argv[1],"--wait-owner")) { for(;;) { struct timespec delay={.tv_sec=1}; nanosleep(&delay,NULL); } }
    if(!strcmp(argv[1],"--exit")) return 7;
    if(!strcmp(argv[1],"--partial")) { output("AB",2); struct timespec delay={.tv_nsec=200000000}; nanosleep(&delay,NULL); output("CD",2); return 0; }
    demand(argc==3);
    if(!strcmp(argv[1],"--reexec")) { execl("/proc/self/fd/5/reexec","foreign","--probe",argv[2],(char*)NULL); return 98; }
    demand(!strcmp(argv[1],"--probe"));
    int file=open(argv[2],O_RDONLY); demand(file<0 && errno==EACCES);
    file=open("escape",O_RDONLY); demand(file<0 && errno==EACCES);
    file=open("inside",O_WRONLY); demand(file<0 && errno==EACCES);
    file=socket(AF_INET,SOCK_STREAM,0); demand(file<0 && errno==EPERM);
    demand(fork()<0 && errno==EPERM);
    struct stat status; demand(fstat(100,&status)<0 && errno==EBADF);
    demand(getenv("LKJSCRIPT_FOREIGN_UNGRANTED_SECRET")==NULL);
    file=open("inside",O_RDONLY); demand(file>=0); char inside[7]={0}; demand(read(file,inside,6)==6); close(file); demand(!strcmp(inside,"inside"));
    output("PASS\n",5);
    char bytes[1024]; for(;;) { ssize_t count=read(0,bytes,sizeof(bytes)); if(count<0 && errno==EINTR) continue; demand(count>=0); if(!count) return 0; output(bytes,(size_t)count); }
}
