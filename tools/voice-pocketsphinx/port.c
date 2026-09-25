/* InfinityOS native PocketSphinx adapter. Only immutable packaged model files
 * are visible; no host filesystem, microphone, network, or process API exists. */
#include <pocketsphinx.h>
#include <pocketsphinx/err.h>
#include <stdint.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <setjmp.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/times.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include <sys/resource.h>

struct model_file { const char *name; const unsigned char *data; size_t length; };
extern const struct model_file native_model_files[];
extern const size_t native_model_file_count;
static struct { const struct model_file *file; size_t offset; } files[32];
static _Alignas(16) unsigned char heap[192u*1024u*1024u];
static size_t heap_used;
static jmp_buf failure;
static int active;
static int poisoned;
static size_t live_bytes;
static size_t erased_bytes;
struct allocation { size_t bytes; size_t guard; };
struct _reent;
extern void *__real__malloc_r(struct _reent *, size_t);
extern void __real__free_r(struct _reent *, void *);
char *empty_environment[] = { NULL };
char **environ = empty_environment;

// ------------------------=
// FUNC: memset
// DESC: Supports unaligned private storage even before the native MMU enables normal-memory accesses.
// ------------------=
void *memset(void*p,int value,size_t count){volatile unsigned char*out=p;for(size_t i=0;i<count;i++)out[i]=(unsigned char)value;return p;}
// ------------------------=
// FUNC: memcpy
// DESC: Copies model bytes without unaligned vector loads or host libc instructions.
// ------------------=
void *memcpy(void*out,const void*input,size_t count){volatile unsigned char*d=out;const volatile unsigned char*s=input;for(size_t i=0;i<count;i++)d[i]=s[i];return out;}
// ------------------------=
// FUNC: memmove
// DESC: Preserves overlapping regions using alignment-safe native byte operations.
// ------------------=
void *memmove(void*out,const void*input,size_t count){volatile unsigned char*d=out;const volatile unsigned char*s=input;if((uintptr_t)d>(uintptr_t)s){while(count){count--;d[count]=s[count];}}else{for(size_t i=0;i<count;i++)d[i]=s[i];}return out;}

// ------------------------=
// FUNC: __wrap__malloc_r
// DESC: Tracks private allocation bounds so speech-derived contents can be erased on release.
// ------------------=
void *__wrap__malloc_r(struct _reent *r,size_t bytes){
    if(poisoned||bytes>sizeof(heap)-sizeof(struct allocation)){errno=ENOMEM;return NULL;}
    struct allocation *p=__real__malloc_r(r,bytes+sizeof(*p));
    if(!p)return NULL;
    p->bytes=bytes;p->guard=0x535454414c4c4f43ull;live_bytes+=bytes;return p+1;
}
// ------------------------=
// FUNC: __wrap__free_r
// DESC: Erases speech features and hypotheses before private allocator reuse.
// ------------------=
void __wrap__free_r(struct _reent *r,void *ptr){
    if(!ptr)return;
    struct allocation *p=(struct allocation*)ptr-1;
    if(heap_used<sizeof(*p)||(uintptr_t)p<(uintptr_t)heap||(uintptr_t)p>(uintptr_t)(heap+heap_used-sizeof(*p))||p->guard!=0x535454414c4c4f43ull||p->bytes>heap_used-((unsigned char*)(p+1)-heap))_exit(1);
    size_t bytes=p->bytes;memset(ptr,0,bytes);live_bytes-=bytes;erased_bytes+=bytes;
    memset(p,0,sizeof(*p));__real__free_r(r,p);
}
// ------------------------=
// FUNC: __wrap__calloc_r
// DESC: Rejects size overflow and initializes all private recognition storage.
// ------------------=
void *__wrap__calloc_r(struct _reent *r,size_t count,size_t bytes){
    if(bytes&&count>SIZE_MAX/bytes){errno=ENOMEM;return NULL;}
    size_t total=count*bytes;void *p=__wrap__malloc_r(r,total);if(p)memset(p,0,total);return p;
}
// ------------------------=
// FUNC: __wrap__realloc_r
// DESC: Preserves requested bytes while erasing the complete superseded allocation.
// ------------------=
void *__wrap__realloc_r(struct _reent *r,void *ptr,size_t bytes){
    if(!ptr)return __wrap__malloc_r(r,bytes);
    if(!bytes){__wrap__free_r(r,ptr);return NULL;}
    struct allocation *p=(struct allocation*)ptr-1;
    if(p->guard!=0x535454414c4c4f43ull)_exit(1);
    void *next=__wrap__malloc_r(r,bytes);if(!next)return NULL;
    memcpy(next,ptr,bytes<p->bytes?bytes:p->bytes);__wrap__free_r(r,ptr);return next;
}
// ------------------------=
// FUNC: native_memory_state
// DESC: Exposes structured allocation and cleanup evidence for native behavioral probes.
// ------------------=
void native_memory_state(size_t *live,size_t *erased){*live=live_bytes;*erased=erased_bytes;}

// ------------------------=
// FUNC: lookup
// DESC: Resolves only exact immutable packaged model names.
// ------------------=
static const struct model_file *lookup(const char *name) {
    for(size_t i=0;i<native_model_file_count;i++) if(!strcmp(name,native_model_files[i].name)) return &native_model_files[i];
    return NULL;
}
// ------------------------=
// FUNC: _open
// DESC: Opens a bounded read-only model descriptor; writes and unknown paths fail closed.
// ------------------=
int _open(const char *name,int flags,...) {
    if(flags&(O_WRONLY|O_RDWR|O_CREAT|O_TRUNC)){errno=EACCES;return -1;}
    const struct model_file *f=lookup(name); if(!f){errno=ENOENT;return -1;}
    for(int i=3;i<32;i++)if(!files[i].file){files[i].file=f;files[i].offset=0;return i;}
    errno=EMFILE;return -1;
}
// ------------------------=
// FUNC: _close
// DESC: Releases one private model descriptor.
// ------------------=
int _close(int fd){if(fd<3||fd>=32||!files[fd].file){errno=EBADF;return -1;}files[fd].file=NULL;files[fd].offset=0;return 0;}
// ------------------------=
// FUNC: _read
// DESC: Reads immutable packaged bytes within file bounds.
// ------------------=
ssize_t _read(int fd,void *out,size_t count){
    if(fd<3||fd>=32||!files[fd].file){errno=EBADF;return -1;}
    size_t remaining=files[fd].file->length-files[fd].offset;if(count>remaining)count=remaining;
    memcpy(out,files[fd].file->data+files[fd].offset,count);files[fd].offset+=count;return count;
}
// ------------------------=
// FUNC: _lseek
// DESC: Seeks only within a model's immutable extent.
// ------------------=
off_t _lseek(int fd,off_t offset,int origin){
    if(fd<3||fd>=32||!files[fd].file){errno=EBADF;return -1;}
    int64_t base=origin==SEEK_SET?0:origin==SEEK_CUR?(int64_t)files[fd].offset:origin==SEEK_END?(int64_t)files[fd].file->length:-1;
    if(base<0||offset < -base || offset > (int64_t)files[fd].file->length-base){errno=EINVAL;return -1;}
    files[fd].offset=(size_t)(base+offset);return files[fd].offset;
}
// ------------------------=
// FUNC: _fstat
// DESC: Reports actual immutable model length, never host metadata.
// ------------------=
int _fstat(int fd,struct stat *st){
    memset(st,0,sizeof(*st));if(fd>=0&&fd<3){st->st_mode=S_IFCHR;return 0;}
    if(fd<3||fd>=32||!files[fd].file){errno=EBADF;return -1;}
    st->st_mode=S_IFREG|0444;st->st_size=files[fd].file->length;return 0;
}
// ------------------------=
// FUNC: _stat
// DESC: Reports only registered model resources.
// ------------------=
int _stat(const char *name,struct stat *st){const struct model_file*f=lookup(name);if(!f){errno=ENOENT;return -1;}memset(st,0,sizeof(*st));st->st_mode=S_IFREG|0444;st->st_size=f->length;return 0;}
// ------------------------=
// FUNC: _write
// DESC: Discards library diagnostics; it cannot write files or contact a service.
// ------------------=
ssize_t _write(int fd,const void *p,size_t n){(void)p;if(fd==1||fd==2)return n;errno=EACCES;return -1;}
// ------------------------=
// FUNC: _isatty
// DESC: No host terminal is exposed to the recognizer.
// ------------------=
int _isatty(int fd){(void)fd;return 0;}
// ------------------------=
// FUNC: _sbrk
// DESC: Bounds the private libc heap independently of kernel and model inference memory.
// ------------------=
void *_sbrk(ptrdiff_t change){if(change<0 || (size_t)change>sizeof(heap)-heap_used){errno=ENOMEM;return(void*)-1;}void*p=heap+heap_used;heap_used+=(size_t)change;return p;}
// ------------------------=
// FUNC: _gettimeofday
// DESC: Disables library wall-clock profiling; native caller measures real deadlines and latency.
// ------------------=
int _gettimeofday(struct timeval*t,void*z){(void)z;t->tv_sec=0;t->tv_usec=0;return 0;}
// ------------------------=
// FUNC: _times
// DESC: Leaves CPU accounting to the native service rather than fabricating process counters.
// ------------------=
clock_t _times(struct tms*t){memset(t,0,sizeof(*t));return 0;}
// ------------------------=
// FUNC: _getpid
// DESC: Supplies a non-security local identity for libc compatibility only.
// ------------------=
int _getpid(void){return 1;}
// ------------------------=
// FUNC: _kill
// DESC: Refuses process signalling from the recognizer.
// ------------------=
int _kill(int p,int s){(void)p;(void)s;errno=ENOSYS;return -1;}
// ------------------------=
// FUNC: _exit
// DESC: Converts library fatal exits into provider failure without terminating the kernel.
// ------------------=
__attribute__((noreturn)) void _exit(int code){(void)code;if(active)longjmp(failure,1);__builtin_trap();}
// ------------------------=
// FUNC: sysconf
// DESC: Reports only the fixed alignment used by private model mappings.
// ------------------=
long sysconf(int key){(void)key;return 4096;}
// ------------------------=
// FUNC: mmap
// DESC: Exposes bounded read-only model bytes with no general memory mapping authority.
// ------------------=
void *mmap(void*address,size_t length,int protection,int flags,int fd,off_t offset){
    (void)address;(void)flags;
    if(protection!=1||fd<3||fd>=32||!files[fd].file||offset<0||(size_t)offset>files[fd].file->length||length>files[fd].file->length-(size_t)offset){errno=EINVAL;return(void*)-1;}
    return(void*)(files[fd].file->data+(size_t)offset);
}
// ------------------------=
// FUNC: munmap
// DESC: Immutable packaged mappings have kernel lifetime and require no host operation.
// ------------------=
int munmap(void*p,size_t n){(void)p;(void)n;return 0;}
// ------------------------=
// FUNC: getrusage
// DESC: Leaves profiling to the native service rather than claiming host process usage.
// ------------------=
int getrusage(int who,struct rusage*out){(void)who;memset(out,0,sizeof(*out));return 0;}
// ------------------------=
// FUNC: popen
// DESC: Refuses subprocesses and compressed-file shell commands.
// ------------------=
FILE *popen(const char*c,const char*m){(void)c;(void)m;errno=ENOSYS;return NULL;}
// ------------------------=
// FUNC: pclose
// DESC: No subprocess can be opened through this provider.
// ------------------=
int pclose(FILE*f){(void)f;errno=ENOSYS;return -1;}
// ------------------------=
// FUNC: native_recognize
// DESC: Decodes a completed bounded utterance with batch normalization and cancellation gates, without host dependencies.
// ------------------=
int native_recognize(const int16_t *pcm,size_t samples,char *text,size_t capacity,size_t *length,size_t *memory,int(*cancel)(void)){
    if(active||poisoned||!pcm||!samples||samples>160000||!text||capacity<2||!length||!memory)return 1;
    *length=0;*memory=heap_used;memset(text,0,capacity);
    if(cancel&&cancel())return 2;
    /* Digital silence contains no speech. Do not let language-model priors
     * invent a transcript when a capture device returns zero-filled PCM. */
    size_t nonzero=0;
    for(size_t i=0;i<samples;i++)nonzero|=(uint16_t)pcm[i];
    if(!nonzero)return 5;
    active=1;
    if(setjmp(failure)){
        /* libc may retain pointers after a fatal exit. Quarantine this provider
         * until reboot, rather than reusing corrupted allocator state. */
        poisoned=1;memset(heap,0,heap_used);memset(files,0,sizeof(files));
        active=0;*memory=heap_used;return 4;
    }
    ps_config_t*config=ps_config_init(NULL);
    if(!config){active=0;return 3;}
    ps_config_set_str(config,"hmm","/model/en-us/en-us");
    ps_config_set_str(config,"lm","/model/en-us/en-us.lm.bin");
    ps_config_set_str(config,"dict","/model/en-us/cmudict-en-us.dict");
    ps_config_set_float(config,"samprate",16000);
    ps_config_set_bool(config,"mmap",1);
    err_set_loglevel(ERR_FATAL);
    ps_decoder_t*decoder=ps_init(config);
    ps_config_free(config);
    if(!decoder){active=0;*memory=heap_used;return 3;}
    int result=ps_start_utt(decoder)<0?3:0;
    /* VAD already completed this recording. Streaming normalization restarts
     * from generic channel statistics on every short phrase and can change
     * its words. Batch mode estimates the channel from the entire recording.
     * This bounded upstream call stays on an AP; cancellation is checked on
     * both sides, before any transcript can be published. */
    if(!result&&cancel&&cancel())result=2;
    if(!result&&ps_process_raw(decoder,pcm,samples,0,1)<0)result=3;
    if(!result&&cancel&&cancel())result=2;
    if(!result&&ps_end_utt(decoder)<0)result=3;
    if(!result){const char*hyp=ps_get_hyp(decoder,NULL);size_t n=hyp?strlen(hyp):0;
        if(!n)result=5;else if(n>=capacity)result=6;else{memcpy(text,hyp,n);text[n]=0;*length=n;}}
    ps_free(decoder);*memory=heap_used;active=0;return result;
}
