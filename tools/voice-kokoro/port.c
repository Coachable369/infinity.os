/* InfinityOS native Kokoro adapter. Only immutable packaged model files
 * are visible; no host filesystem, microphone, network, or process API exists. */
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
#include <dirent.h>
#include <time.h>
#include <malloc.h>
#include <pthread.h>
#include <infinity/compiler_host.h>
#include <infinity/serial_tls.h>

struct model_file { const char *name; const unsigned char *data; size_t length; };
extern const struct model_file native_model_files[];
extern const size_t native_model_file_count;
static struct { const struct model_file *file; size_t offset; } files[32];
/* Keep the large arena after ordinary data on x86, so PC-relative references
 * to small objects remain within the native compiler's 2 GiB code model. */
#if defined(__x86_64__)
__attribute__((section(".speech_heap")))
#endif
static _Alignas(16) unsigned char heap[1024u*1024u*1024u];
static size_t heap_used;
static jmp_buf failure;
static int active;
static int poisoned;
static int (*cancel_callback)(void *);
static void *cancel_context;
static size_t erased_bytes;
static uintptr_t fatal_address;
static size_t failed_allocation;
static uintptr_t allocation_callers[8];
unsigned int native_phase;
// ------------------------=
// FUNC: allocation_failure
// DESC: Records bounded frame addresses for native probe diagnosis without copying stack contents or speech data.
// ------------------=
static void allocation_failure(size_t n){
    failed_allocation=n;
    uintptr_t *frame=__builtin_frame_address(0);
    uintptr_t limit=(uintptr_t)frame+65536;
    for(size_t i=0;i<8;i++){
        allocation_callers[i]=0;
        if(!frame)continue;
        allocation_callers[i]=frame[1];
        uintptr_t next=frame[0];
        frame=next>(uintptr_t)frame&&next<limit&&!(next&15)?(uintptr_t*)next:NULL;
    }
}
char *empty_environment[] = { NULL };
char **environ = empty_environment;

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
    const struct model_file *f=lookup(name); if(!f){errno=ENOENT;return -1;}if(!f->data){errno=EISDIR;return -1;}
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
int _stat(const char *name,struct stat *st){const struct model_file*f=lookup(name);if(!f){errno=ENOENT;return -1;}memset(st,0,sizeof(*st));st->st_mode=(f->data?S_IFREG|0444:S_IFDIR|0555);st->st_size=f->length;return 0;}
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
// FUNC: native_resource
// DESC: Borrows only a registered immutable model extent.
// ------------------=
const unsigned char *native_resource(const char *name,size_t *length) {
    const struct model_file *f=lookup(name); *length=f?f->length:0; return f?f->data:NULL;
}
// ------------------------=
// FUNC: _getpid
// DESC: Supplies an image-local identity, not a host process.
// ------------------=
int _getpid(void){return 1;}
// ------------------------=
// FUNC: _kill
// DESC: Denies process signalling.
// ------------------=
int _kill(int p,int s){(void)p;(void)s;errno=ENOSYS;return -1;}
// ------------------------=
// FUNC: _exit
// DESC: Quarantines fatal library errors inside the native C boundary.
// ------------------=
__attribute__((noreturn)) void _exit(int code){(void)code;if(!fatal_address)fatal_address=(uintptr_t)__builtin_return_address(0);if(active)longjmp(failure,1);__builtin_trap();}
void *__dso_handle;
// ------------------------=
// FUNC: __cxa_allocate_exception
// DESC: Fails closed on an exception request instead of allocating an unwinder or escaping into Rust.
// ------------------=
void *__cxa_allocate_exception(size_t bytes){(void)bytes;_exit(1);}
// ------------------------=
// FUNC: __cxa_throw
// DESC: Quarantines unexpected C++ throws within the native provider.
// ------------------=
__attribute__((noreturn)) void __cxa_throw(void*exception,void*type,void(*destroy)(void*)){(void)exception;(void)type;(void)destroy;_exit(1);}
// ------------------------=
// FUNC: _unlink
// DESC: Refuses mutation of immutable packaged resources.
// ------------------=
int _unlink(const char*path){(void)path;errno=EROFS;return -1;}
// ------------------------=
// FUNC: posix_memalign
// DESC: Satisfies GGML alignment from private bounded memory only.
// ------------------=
int posix_memalign(void**out,size_t alignment,size_t size){if(!out||alignment<sizeof(void*)||(alignment&(alignment-1)))return EINVAL;void*p=memalign(alignment,size);if(!p)return ENOMEM;*out=p;return 0;}
// ------------------------=
// FUNC: pthread_cond_init
// DESC: Initializes a serial-worker condition without granting thread creation.
// ------------------=
int pthread_cond_init(pthread_cond_t*cond,const pthread_condattr_t*attr){if(!cond||attr)return EINVAL;*cond=PTHREAD_COND_INITIALIZER;return 0;}
// ------------------------=
// FUNC: sysconf
// DESC: Reports the explicitly single-worker resource contract and rejects unknown queries.
// ------------------=
long sysconf(int key){if(key==_SC_PAGESIZE)return 4096;if(key==_SC_NPROCESSORS_ONLN)return 1;errno=EINVAL;return -1;}

extern void *__real__malloc_r(struct _reent *,size_t);
extern void __real__free_r(struct _reent *,void *);
extern size_t _malloc_usable_size_r(struct _reent *,void *);
// ------------------------=
// FUNC: __wrap__malloc_r
// DESC: Allocates only from the private bounded native heap.
// ------------------=
void *__wrap__malloc_r(struct _reent *r,size_t n){if(poisoned||n>sizeof(heap)){allocation_failure(n);errno=ENOMEM;return NULL;}void*p=__real__malloc_r(r,n);if(!p)allocation_failure(n);return p;}
// ------------------------=
// FUNC: native_diagnostics
// DESC: Reports numeric failure stage, heap commitment and fault site without exposing speech text or model content.
// ------------------=
void native_diagnostics(size_t*out){out[0]=native_phase;out[1]=heap_used;out[2]=failed_allocation;out[3]=fatal_address;for(size_t i=0;i<8;i++)out[4+i]=allocation_callers[i];}
// ------------------------=
// FUNC: __wrap__free_r
// DESC: Erases complete speech-derived allocations before reuse, including aligned allocations.
// ------------------=
void __wrap__free_r(struct _reent *r,void *p){
    if(!p)return;
    if((uintptr_t)p<(uintptr_t)heap||(uintptr_t)p>=(uintptr_t)(heap+heap_used))_exit(1);
    size_t n=_malloc_usable_size_r(r,p);
    if(n>heap_used-((unsigned char*)p-heap))_exit(1);
    memset(p,0,n);erased_bytes+=n;__real__free_r(r,p);
}
// ------------------------=
// FUNC: __wrap__calloc_r
// DESC: Rejects overflow and initializes allocated private storage.
// ------------------=
void *__wrap__calloc_r(struct _reent*r,size_t count,size_t n){if(n&&count>SIZE_MAX/n){errno=ENOMEM;return NULL;}size_t bytes=count*n;void*p=__wrap__malloc_r(r,bytes);if(p)memset(p,0,bytes);return p;}
// ------------------------=
// FUNC: __wrap__realloc_r
// DESC: Erases superseded buffers rather than leaving speech features behind.
// ------------------=
void *__wrap__realloc_r(struct _reent*r,void*p,size_t n){
    if(!p)return __wrap__malloc_r(r,n);
    if(!n){__wrap__free_r(r,p);return NULL;}
    if((uintptr_t)p<(uintptr_t)heap||(uintptr_t)p>=(uintptr_t)(heap+heap_used))_exit(1);
    size_t old=_malloc_usable_size_r(r,p);void*next=__wrap__malloc_r(r,n);
    if(next){memcpy(next,p,old<n?old:n);__wrap__free_r(r,p);}return next;
}

struct InfinityDirectory {size_t next;const char *prefix;size_t length;struct dirent entry;};
static struct InfinityDirectory directories[16];
// ------------------------=
// FUNC: opendir
// DESC: Opens only a packaged resource namespace, never a host directory.
// ------------------=
DIR *opendir(const char *path){
    const struct model_file*f=lookup(path);if(!f||f->data){errno=ENOENT;return NULL;}
    for(size_t i=0;i<16;i++)if(!directories[i].prefix){directories[i].prefix=f->name;directories[i].length=strlen(path);directories[i].next=0;return &directories[i];}
    errno=EMFILE;return NULL;
}
// ------------------------=
// FUNC: readdir
// DESC: Enumerates exact direct children of the immutable resource namespace.
// ------------------=
struct dirent *readdir(DIR*d){
    if(!d||!d->prefix){errno=EBADF;return NULL;}
    while(d->next<native_model_file_count){const char*n=native_model_files[d->next++].name;
        if(strncmp(n,d->prefix,d->length)||n[d->length]!='/')continue;
        n+=d->length+1;if(!*n||strchr(n,'/'))continue;
        size_t length=strlen(n);if(length>=sizeof(d->entry.d_name)){errno=ENAMETOOLONG;return NULL;}
        memcpy(d->entry.d_name,n,length+1);return &d->entry;
    }return NULL;
}
// ------------------------=
// FUNC: closedir
// DESC: Releases a bounded resource cursor.
// ------------------=
int closedir(DIR*d){if(!d||!d->prefix){errno=EBADF;return -1;}memset(d,0,sizeof(*d));return 0;}
// ------------------------=
// FUNC: infinity_compiler_get_host
// DESC: Reuses serial synchronization contracts without exposing any compiler or host I/O capability.
// ------------------=
const InfinityCompilerHost *infinity_compiler_get_host(void){static const InfinityCompilerHost contract={.serial_execution=1};return &contract;}
// ------------------------=
// FUNC: clock_gettime
// DESC: Reads the native ARM64 architectural counter for monotonic profiling.
// ------------------=
int clock_gettime(clockid_t id,struct timespec*t){
    if(!t||(id!=CLOCK_MONOTONIC&&id!=CLOCK_REALTIME)){errno=EINVAL;return -1;}
#if defined(__aarch64__)
    uint64_t ticks,frequency;__asm__ volatile("mrs %0,cntvct_el0":"=r"(ticks));__asm__ volatile("mrs %0,cntfrq_el0":"=r"(frequency));
#else
    extern uint64_t infinity_speech_clock_ns(void);
    uint64_t ticks=infinity_speech_clock_ns(),frequency=1000000000ull;
#endif
    if(!frequency){errno=ENOSYS;return -1;}t->tv_sec=ticks/frequency;t->tv_nsec=(ticks%frequency)*1000000000ull/frequency;return 0;
}
// ------------------------=
// FUNC: _gettimeofday
// DESC: Provides elapsed native time only; no host wall-clock service is contacted.
// ------------------=
int _gettimeofday(struct timeval*t,void*z){(void)z;struct timespec now;if(clock_gettime(CLOCK_MONOTONIC,&now))return -1;t->tv_sec=now.tv_sec;t->tv_usec=now.tv_nsec/1000;return 0;}
// ------------------------=
// FUNC: native_cancelled
// DESC: Consults only the caller's native cancellation/deadline callback.
// ------------------=
int native_cancelled(void){return cancel_callback&&cancel_callback(cancel_context);}
// ------------------------=
// FUNC: native_abort_callback
// DESC: Adapts native cancellation to GGML's CPU graph abort contract.
// ------------------=
_Bool native_abort_callback(void*unused){(void)unused;return native_cancelled()!=0;}

struct emulated_tls {size_t size,alignment;void *address;void *initial;};
static struct emulated_tls *tls_slots[128];
static size_t tls_count;
// ------------------------=
// FUNC: __emutls_get_address
// DESC: Owns bounded serial-worker TLS entirely inside the private native image.
// ------------------=
void *__emutls_get_address(struct emulated_tls*t){
    if(t->address)return t->address;
    if(tls_count==128||!t->alignment||(t->alignment&(t->alignment-1)))_exit(1);
    void*p=memalign(t->alignment,t->size);if(!p)_exit(1);
    if(t->initial)memcpy(p,t->initial,t->size);else memset(p,0,t->size);
    t->address=p;tls_slots[tls_count++]=t;return p;
}
extern int native_run(const char*,size_t,int16_t*,size_t,size_t*);
extern int whisper_private_native_transcribe(const int16_t*,size_t,char*,size_t,size_t*);
extern void native_release_synthesis_model(void);
// ------------------------=
// FUNC: native_synthesize
// DESC: Contains one bounded synthesis job and quarantines fatal engine state without crossing the Rust ABI.
// ------------------=
int native_synthesize(const char*text,size_t length,int16_t*pcm,size_t capacity,size_t*frames,int(*cancel)(void*),void*context){
    if(!frames)return 1;*frames=0;
    if(!text||!pcm||!length||length>160||!capacity||capacity>720000||active||poisoned)return 1;
    memset(pcm,0,capacity*sizeof(*pcm));cancel_callback=cancel;cancel_context=context;active=1;
    int result;
    if(setjmp(failure)){poisoned=1;result=7;}
    else{
        result=native_cancelled()?2:native_run(text,length,pcm,capacity,frames);
        infinity_thread_cleanup();
        while(tls_count){struct emulated_tls*t=tls_slots[--tls_count];free(t->address);t->address=NULL;tls_slots[tls_count]=NULL;}
    }
    active=0;cancel_callback=NULL;cancel_context=NULL;
    if(result){memset(pcm,0,capacity*sizeof(*pcm));*frames=0;}
    return result;
}

// ------------------------=
// FUNC: native_recognize
// DESC: Contains one bounded native Whisper request and prevents fatal engine state from crossing the Rust ABI.
// ------------------=
int native_recognize(const int16_t*pcm,size_t samples,char*text,size_t capacity,size_t*length,size_t*memory,int(*cancel)(void*),void*context){
    if(!length||!memory)return 1;*length=0;*memory=heap_used;
    if(!pcm||!samples||samples>160000||!text||capacity<2||active||poisoned)return 1;
    size_t nonzero=0;for(size_t i=0;i<samples;i++)nonzero|=(uint16_t)pcm[i];
    if(!nonzero){text[0]=0;return 5;}
    memset(text,0,capacity);cancel_callback=cancel;cancel_context=context;active=1;
    int result;
    if(setjmp(failure)){poisoned=1;result=7;}
    else{
        native_release_synthesis_model();
        result=native_cancelled()?2:whisper_private_native_transcribe(pcm,samples,text,capacity,length);
        infinity_thread_cleanup();
        while(tls_count){struct emulated_tls*t=tls_slots[--tls_count];free(t->address);t->address=NULL;tls_slots[tls_count]=NULL;}
    }
    if(result){memset(text,0,capacity);*length=0;}
    cancel_callback=NULL;cancel_context=NULL;active=0;*memory=heap_used;return result;
}
