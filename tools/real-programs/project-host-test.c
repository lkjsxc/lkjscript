/* Disposable syscall fault witness; never used by project.sh. */
#define main projection_host_main
#define fstat fixture_fstat
#define lstat fixture_lstat
#define readlink fixture_readlink
#define mkdir fixture_mkdir
#define link fixture_link
#include "project-host.c"
#undef main
#undef fstat
#undef lstat
#undef readlink
#undef mkdir
#undef link
#include <sys/syscall.h>

static unsigned stat_calls;
static int read_link_seen;
static int fault_is(const char *name) { const char *v=getenv("PROJECT_FAULT"); return v && !strcmp(v,name); }
static void marker(const char *name) {
    const char *root=getenv("PROJECT_FAULT_ROOT"); if(!root)die("missing fixture root");
    char *p=NULL;if(asprintf(&p,"%s/%s",root,name)<0)die("allocation failed");
    int fd=open(p,O_WRONLY|O_CREAT|O_EXCL,0600);if(fd<0)die("fixture marker open failed");
    write_all(fd,"keep\n",5);close(fd);free(p);
}
int fixture_fstat(int fd,struct stat *s) {
    int rc=(int)syscall(SYS_fstat,fd,s);if(rc)return rc;stat_calls++;
    if(stat_calls==1){
        if(fault_is("opened-inode"))s->st_ino++;
        if(fault_is("opened-type"))s->st_mode=S_IFIFO|0600;
        if(fault_is("verify-signal")){
            marker("ready");
            /* Only a fixture synchronization point, no production timeout. */
            for(;;)pause();
        }
    }
    if(stat_calls==2){
        if(fault_is("read-size"))s->st_size++;
        if(fault_is("read-mtime"))s->st_mtim.tv_nsec^=1;
        if(fault_is("read-ctime"))s->st_ctim.tv_nsec^=1;
    }
    return rc;
}
ssize_t fixture_readlink(const char *path,char *out,size_t n) {
    ssize_t rc=(ssize_t)syscall(SYS_readlink,path,out,n);if(rc>=0)read_link_seen=1;return rc;
}
int fixture_lstat(const char *path,struct stat *s) {
    int rc=(int)syscall(SYS_newfstatat,AT_FDCWD,path,s,AT_SYMLINK_NOFOLLOW);
    if(!rc && read_link_seen){
        read_link_seen=0;
        if(fault_is("link-missing")){errno=ENOENT;return -1;}
        if(fault_is("link-inode"))s->st_ino++;
        if(fault_is("link-mtime"))s->st_mtim.tv_nsec^=1;
        if(fault_is("link-ctime"))s->st_ctim.tv_nsec^=1;
        if(fault_is("link-type"))s->st_mode=S_IFREG|0600;
    }
    return rc;
}
int fixture_mkdir(const char *path,mode_t mode) {
    const char *root=getenv("PROJECT_FAULT_ROOT");
    int rc=(int)syscall(SYS_mkdir,path,mode);
    if(!rc && root && !strcmp(path,root) && fault_is("mkdir-race")){marker("sentinel");errno=EEXIST;return -1;}
    return rc;
}
int fixture_link(const char *from,const char *to) {
    int rc=(int)syscall(SYS_link,from,to);if(rc)return rc;
    struct stat s;if(syscall(SYS_newfstatat,AT_FDCWD,from,&s,AT_SYMLINK_NOFOLLOW)||!S_ISLNK(s.st_mode))return rc;
    if(fault_is("cleanup-invalid")){marker("raw-\xff");errno=EIO;return -1;}
    if(fault_is("cleanup-replaced")){
        const char *root=getenv("PROJECT_FAULT_ROOT");if(!root)die("missing fixture root");
        char *moved=NULL;if(asprintf(&moved,"%s.owned",root)<0)die("allocation failed");
        if(rename(root,moved)||syscall(SYS_mkdir,root,0700))die("fixture root replacement failed");
        free(moved);marker("sentinel");errno=EIO;return -1;
    }
    return rc;
}
int main(int argc,char **argv){
    if(argc>2 && !strcmp(argv[1],"--launcher")){
        /* Background shell jobs inherit SIG_IGN; emulate a normal caller. */
        signal(SIGINT,SIG_DFL);signal(SIGTERM,SIG_DFL);
        execv(argv[2],argv+2);die("fixture launcher exec failed");
    }
    return projection_host_main(argc,argv);
}
