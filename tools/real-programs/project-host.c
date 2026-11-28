/* Linux syscall bridge for the checked projector. No Git, path, graph,
 * manifest, hash, verification or transaction policy lives here. Requests and
 * replies are length-framed byte strings; only the checked program emits them.
 * This is an ambient host adapter, not an OS sandbox. */
#define _GNU_SOURCE
#include <sys/types.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <sys/prctl.h>
#include <fcntl.h>
#include <dirent.h>
#include <unistd.h>
#include <signal.h>
#include <poll.h>
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <limits.h>

typedef struct { unsigned char *p; size_t n; } Bytes;
static int input_fd, output_fd;
static volatile sig_atomic_t interrupted;
static void signal_pending(int sig) { if (!interrupted) interrupted = sig; }
static void die(const char *s) { fprintf(stderr, "project host: %s\n", s); exit(255); }
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
static const char *code(int e) {
    switch (e) {
    case ENOENT: return "ENOENT"; case EEXIST: return "EEXIST"; case EACCES: return "EACCES";
    case EPERM: return "EPERM"; case ENOTDIR: return "ENOTDIR"; case EISDIR: return "EISDIR";
    case ELOOP: return "ELOOP"; case EXDEV: return "EXDEV"; case ENOTEMPTY: return "ENOTEMPTY";
    case ENAMETOOLONG: return "ENAMETOOLONG"; case EINVAL: return "EINVAL"; case ENOSPC: return "ENOSPC";
    case EMFILE: return "EMFILE"; case ENFILE: return "ENFILE"; case EIO: return "EIO";
    case EROFS: return "EROFS"; case ENOMEM: return "ENOMEM"; default: return "UNKNOWN";
    }
}
static const char *reason(int e) {
    switch (e) {
    case ENOENT: return "no such file or directory"; case EEXIST: return "file already exists";
    case EACCES: return "permission denied"; case EPERM: return "operation not permitted";
    case ENOTDIR: return "not a directory"; case EISDIR: return "illegal operation on a directory";
    case ELOOP: return "too many symbolic links encountered"; case EXDEV: return "cross-device link not permitted";
    case ENOTEMPTY: return "directory not empty"; case ENAMETOOLONG: return "name too long";
    case EINVAL: return "invalid argument"; case ENOSPC: return "no space left on device";
    case EIO: return "i/o error"; case EROFS: return "read-only file system"; default: return strerror(e);
    }
}
static void error_path(int e, const char *op, const char *p, const char *q) {
    char *s = NULL;
    if (q) { if (asprintf(&s, "%s: %s, %s '%s' -> '%s'", code(e), reason(e), op, p, q) < 0) die("allocation failed"); }
    else if (asprintf(&s, "%s: %s, %s '%s'", code(e), reason(e), op, p) < 0) die("allocation failed");
    error_text(s); free(s);
}
static Bytes metadata(const struct stat *s) {
    char *p = NULL;
    if (asprintf(&p, "%llu\t%llu\t%llu\t%llu\t%llu\t%llu\t%lld\t%ld\t%lld\t%ld",
        (unsigned long long)s->st_dev, (unsigned long long)s->st_ino,
        (unsigned long long)s->st_mode, (unsigned long long)s->st_size,
        (unsigned long long)s->st_nlink, (unsigned long long)(s->st_mode & S_IFMT),
        (long long)s->st_mtim.tv_sec, s->st_mtim.tv_nsec,
        (long long)s->st_ctim.tv_sec, s->st_ctim.tv_nsec) < 0) die("allocation failed");
    Bytes b = text(p); free(p); return b;
}
static void append(Bytes *b, const void *p, size_t n) {
    if (n > SIZE_MAX - b->n - 1) die("unrepresentable buffer");
    unsigned char *next = realloc(b->p, b->n + n + 1); if (!next) die("allocation failed");
    b->p = next; memcpy(b->p + b->n, p, n); b->n += n; b->p[b->n] = 0;
}
static int data_fd=-1;
static char *data_path;
static void open_file(const char *p) {
    if(data_fd>=0){error_text("file read already active");return;}
    struct stat before,opened;
    if(lstat(p,&before)){if(errno==ENOENT)error_text("@not-regular");else error_path(errno,"lstat",p,NULL);return;}
    if(!S_ISREG(before.st_mode)){error_text("@not-regular");return;}
    /* O_NONBLOCK avoids a swapped FIFO hang. Post-open fstat prevents data
       reads from special nodes; quiescent namespaces remain a requirement. */
    int fd=open(p,O_RDONLY|O_NOFOLLOW|O_NONBLOCK|O_CLOEXEC);
    if(fd<0){error_path(errno,"open",p,NULL);return;}
    if(fstat(fd,&opened)){int e=errno;close(fd);error_path(e,"fstat",p,NULL);return;}
    if(!S_ISREG(opened.st_mode)){close(fd);error_text("@identity-changed");return;}
    data_fd=fd;data_path=strdup(p);if(!data_path)die("allocation failed");
    Bytes out[]={metadata(&before),metadata(&opened)};reply("ok",2,out);
}
static void read_chunk(void) {
    if(data_fd<0){error_text("no active file read");return;}
    unsigned char buffer[65536];ssize_t n;
    do{n=read(data_fd,buffer,sizeof buffer);}while(n<0 && errno==EINTR);
    if(n<0)error_path(errno,"read",data_path,NULL);else one(bytes(buffer,(size_t)n));
}
static void close_file(void) {
    if(data_fd<0){error_text("no active file read");return;}
    struct stat after;int problem=0;
    if(fstat(data_fd,&after))problem=errno;
    if(close(data_fd) && !problem)problem=errno;
    data_fd=-1;
    if(problem)error_path(problem,"fstat",data_path,NULL);else one(metadata(&after));
    free(data_path);data_path=NULL;
}
static void read_link(const char *p) {
    struct stat before,after;
    if(lstat(p,&before)){if(errno==ENOENT)error_text("@not-link");else error_path(errno,"lstat",p,NULL);return;}
    if(!S_ISLNK(before.st_mode)){error_text("@not-link");return;}
    size_t capacity=256;Bytes payload;
    for(;;){
        unsigned char *buffer=allocate(capacity);
        ssize_t n=readlink(p,(char *)buffer,capacity);
        if(n<0){int e=errno;free(buffer);error_path(e,"readlink",p,NULL);return;}
        if((size_t)n<capacity){payload=bytes(buffer,(size_t)n);free(buffer);break;}
        free(buffer);if(capacity>SIZE_MAX/2)die("unrepresentable link");capacity*=2;
    }
    if(lstat(p,&after)){int e=errno;release(payload);if(e==ENOENT)error_text("@link-changed");else error_path(e,"lstat",p,NULL);return;}
    Bytes out[]={metadata(&before),metadata(&after),payload};reply("ok",3,out);
}
static int digits(const char *s) { if (!*s) return 0; for (; *s; s++) if (*s < '0' || *s > '9') return 0; return 1; }
static void git_environment(void) {
    const char *keys[] = {"GIT_DIR","GIT_WORK_TREE","GIT_INDEX_FILE","GIT_COMMON_DIR","GIT_OBJECT_DIRECTORY","GIT_ALTERNATE_OBJECT_DIRECTORIES","GIT_CONFIG_PARAMETERS","GIT_CONFIG_COUNT"};
    for (size_t i = 0; i < sizeof keys / sizeof *keys; i++) unsetenv(keys[i]);
    for (size_t i = 0; environ[i]; ) {
        char *key = strdup(environ[i]); if (!key) die("allocation failed");
        char *equal = strchr(key, '='); if (equal) *equal = 0;
        if ((!strncmp(key,"GIT_CONFIG_KEY_",15) && digits(key+15)) || (!strncmp(key,"GIT_CONFIG_VALUE_",17) && digits(key+17))) unsetenv(key);
        else i++;
        free(key);
    }
    setenv("GIT_OPTIONAL_LOCKS","0",1); setenv("GIT_CONFIG_NOSYSTEM","1",1); setenv("GIT_CONFIG_GLOBAL","/dev/null",1);
}
static void git_run(size_t n, Bytes *args) {
    int out[2], err[2], failed[2];
    if (pipe2(out,O_CLOEXEC) || pipe2(err,O_CLOEXEC) || pipe2(failed,O_CLOEXEC)) die("git pipe failed");
    pid_t pid = fork(); if (pid < 0) die("git fork failed");
    if (!pid) {
        setpgid(0,0); signal(SIGINT,SIG_DFL); signal(SIGTERM,SIG_DFL);
        int null = open("/dev/null",O_RDONLY); if (null >= 0) { dup2(null,0); close(null); }
        dup2(out[1],1); dup2(err[1],2);
        close(out[0]); close(out[1]); close(err[0]); close(err[1]); close(failed[0]);
        git_environment();
        const char *prefix[] = {"git","--no-replace-objects","--no-optional-locks","-c","core.fsmonitor=false","-c","core.hooksPath=/dev/null","-C"};
        char **argv = allocate((n+9)*sizeof *argv);
        for (size_t i=0;i<8;i++) argv[i]=(char *)prefix[i];
        for (size_t i=0;i<n;i++) argv[i+8]=(char *)args[i].p;
        argv[n+8]=NULL; execvp("git",argv); int e=errno; write_all(failed[1],&e,sizeof e); _exit(127);
    }
    close(out[1]); close(err[1]); close(failed[1]);
    struct pollfd fds[]={{out[0],POLLIN,0},{err[0],POLLIN,0}};
    Bytes captures[]={bytes("",0),bytes("",0)}; size_t live=2;
    while (live) {
        int rc=poll(fds,2,-1); if (rc<0 && errno==EINTR) continue; if(rc<0) die("git poll failed");
        for(size_t i=0;i<2;i++) if(fds[i].fd>=0 && fds[i].revents) {
            unsigned char buffer[65536]; ssize_t got=read(fds[i].fd,buffer,sizeof buffer);
            if(got<0 && errno==EINTR) continue;
            if(got<0) die("git read failed");
            if(!got){close(fds[i].fd);fds[i].fd=-1;live--;} else append(&captures[i],buffer,(size_t)got);
        }
    }
    int status; while(waitpid(pid,&status,0)<0) if(errno!=EINTR) die("git wait failed");
    int exec_error=0; ssize_t got; do { got=read(failed[0],&exec_error,sizeof exec_error); } while(got<0 && errno==EINTR); close(failed[0]);
    char value[32]; if(got>0 || !WIFEXITED(status)) strcpy(value,"null"); else snprintf(value,sizeof value,"%d",WEXITSTATUS(status));
    Bytes result[]={text(value),captures[0],captures[1]}; reply("ok",3,result);
}
static void dispatch(const char *op, size_t n, Bytes *a) {
#define IS(name,count) (!strcmp(op,name) && n==(count))
#define P(i) ((char *)a[i].p)
    if(IS("cwd",0)) { char *p=getcwd(NULL,0); if(!p) { error_path(errno,"getcwd","",NULL);return; } one(text(p)); free(p); }
    else if(IS("stat",1)) { struct stat s; if(lstat(P(0),&s)==0) one(metadata(&s)); else if(errno==ENOENT) one(text("missing")); else error_path(errno,"lstat",P(0),NULL); }
    else if(IS("realpath",1)) { char *p=realpath(P(0),NULL); if(!p) error_path(errno,"realpath",P(0),NULL); else {one(text(p));free(p);} }
    else if(IS("open-file",1)) open_file(P(0));
    else if(IS("read-chunk",0)) read_chunk();
    else if(IS("close-file",0)) close_file();
    else if(IS("read-link",1)) read_link(P(0));
    else if(IS("list",1)) {
        DIR *d=opendir(P(0)); if(!d){error_path(errno,"scandir",P(0),NULL);return;}
        Bytes names=bytes("",0); struct dirent *e; errno=0;
        while((e=readdir(d))) if(strcmp(e->d_name,".") && strcmp(e->d_name,"..")) append(&names,e->d_name,strlen(e->d_name)+1);
        int problem=errno; closedir(d); if(problem){release(names);error_path(problem,"scandir",P(0),NULL);}else one(names);
    }
    else if(IS("mkdir",1)) { if(mkdir(P(0),0700)) error_path(errno,"mkdir",P(0),NULL); else ok(); }
    else if(IS("link",2)) { if(link(P(0),P(1))) error_path(errno,"link",P(0),P(1)); else ok(); }
    else if(IS("unlink",1)) { if(unlink(P(0))) error_path(errno,"unlink",P(0),NULL); else ok(); }
    else if(IS("rmdir",1)) { if(rmdir(P(0))) error_path(errno,"rmdir",P(0),NULL); else ok(); }
    else if(IS("write-new",2)) {
        int fd=open(P(0),O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW|O_CLOEXEC,0600);
        if(fd<0){error_path(errno,"open",P(0),NULL);return;}
        size_t at=0; int problem=0;
        while(at<a[1].n){ssize_t done=write(fd,a[1].p+at,a[1].n-at);if(done<0 && errno==EINTR)continue;if(done<=0){problem=done<0?errno:EIO;break;}at+=(size_t)done;}
        if(close(fd) && !problem)problem=errno;
        if(problem)error_path(problem,"write",P(0),NULL);else ok();
    }
    else if(!strcmp(op,"git") && n>1) git_run(n,a);
    else if(IS("interruptible",0)) {
        struct sigaction action={0}; action.sa_handler=signal_pending; sigemptyset(&action.sa_mask);
        if(sigaction(SIGINT,&action,NULL)||sigaction(SIGTERM,&action,NULL))die("signal setup failed");
        ok();
    }
    else if(IS("checkpoint",0)) { if(interrupted)error_text(interrupted==SIGINT?"interrupted":"terminated");else ok(); }
    else if(IS("result",1)) { write_all(STDOUT_FILENO,a[0].p,a[0].n);ok(); }
    else error_text("invalid host request");
#undef P
#undef IS
}
int main(int argc, char **argv) {
    char *ephemeral=NULL;
    if(argc>3 && !strcmp(argv[1],"--ephemeral")){ephemeral=argv[2];argv+=2;argc-=2;}
    if(argc<2)die("missing checked executable");
    int image=open(argv[1],O_RDONLY|O_CLOEXEC); if(image<0)die("cannot open checked executable");
    if(ephemeral) {
        const char *names[]={"project","select","host","options"};
        for(size_t i=0;i<4;i++){char *p=NULL;if(asprintf(&p,"%s/%s",ephemeral,names[i])<0)die("allocation failed");if(unlink(p) && errno!=ENOENT)die("cannot remove build file");free(p);}
        if(rmdir(ephemeral))die("cannot remove build directory");
    }
    signal(SIGINT,SIG_DFL); signal(SIGTERM,SIG_DFL);
    signal(SIGPIPE,SIG_IGN);
    int request[2],response[2];
    if(pipe2(request,O_CLOEXEC)||pipe2(response,O_CLOEXEC))die("pipe failed");
    pid_t coordinator=getpid();
    pid_t child=fork(); if(child<0)die("fork failed");
    if(!child){
        if(prctl(PR_SET_PDEATHSIG,SIGKILL) || getppid()!=coordinator) _exit(255);
        setpgid(0,0); signal(SIGINT,SIG_DFL);signal(SIGTERM,SIG_DFL);signal(SIGPIPE,SIG_DFL);
        dup2(response[0],0);dup2(request[1],1);
        close(request[0]);close(request[1]);close(response[0]);close(response[1]);
        fexecve(image,argv+1,environ);die("cannot execute checked program");
    }
    close(image);close(request[1]);close(response[0]);input_fd=request[0];output_fd=response[1];
    Bytes op;
    while(frame(&op)==1){
        Bytes count;if(frame(&count)!=1)die("missing request length");
        char *end=NULL;errno=0;unsigned long long number=strtoull((char *)count.p,&end,10);
        if(errno || !count.n || *end || number>SIZE_MAX/sizeof(Bytes))die("invalid request length");
        size_t n=(size_t)number;release(count);Bytes *args=allocate(n*sizeof *args);
        for(size_t i=0;i<n;i++)if(frame(&args[i])!=1)die("missing request field");
        dispatch((char *)op.p,n,args);
        for(size_t i=0;i<n;i++)release(args[i]);
        free(args);release(op);
    }
    if(data_fd>=0)close(data_fd);
    free(data_path);
    close(input_fd);close(output_fd);int status;
    while(waitpid(child,&status,0)<0)if(errno!=EINTR)die("wait failed");
    return WIFEXITED(status)?WEXITSTATUS(status):255;
}
