/* InfinityOS compact Flite port. No files, host audio, sockets, or heap calls.
 * One caller owns this provider; abort recovery never crosses an FFI frame.
 * Upstream synthesis and CMU voice data retain their original licenses.
 */
#include <stdint.h>
#include <stdarg.h>
#include "flite.h"
#include "cst_cg.h"

#define ARENA_BYTES (8u * 1024u * 1024u)
static _Alignas(16) unsigned char arena[ARENA_BYTES];
static size_t used, peak, budget = ARENA_BYTES;
static int failure, active;
static uint32_t noise_state;
static int (*cancelled)(void);
static jmp_buf recovery;
jmp_buf *cst_errjmp;
FILE *stdin, *stdout, *stderr;
extern cst_voice *cmu_us_kal16_diphone;
extern cst_voice *register_cmu_us_kal16(const char *);

// ------------------------=
// FUNC: fail
// DESC: Aborts only the active C synthesis frame and returns a typed status to its caller.
// ------------------=
static void fail(int code) {
    failure = code;
    if (active) longjmp(recovery, 1);
    __builtin_trap(); /* Programming error: no public API calls upstream outside this boundary. */
}
// ------------------------=
// FUNC: checkpoint
// DESC: Honors the caller's cancellation/deadline predicate without unwinding through that callback.
// ------------------=
static void checkpoint(void) { if (cancelled && cancelled()) fail(2); }
// ------------------------=
// FUNC: cst_safe_alloc
// DESC: Allocates zeroed aligned objects in a fixed per-provider arena, failing closed on exhaustion.
// ------------------=
void *cst_safe_alloc(int size) {
    checkpoint();
    if (size < 0) fail(3);
    size_t bytes = ((size_t)(size ? size : 1) + 15u) & ~(size_t)15u;
    if (bytes > budget || used > budget - bytes || budget - used - bytes < 16) fail(3);
    size_t *header = (size_t *)(arena + used);
    header[0] = bytes;
    unsigned char *result = arena + used + 16;
    for (size_t i = 0; i < bytes; i++) result[i] = 0;
    used += bytes + 16;
    if (used > peak) peak = used;
    return result;
}
// ------------------------=
// FUNC: cst_safe_calloc
// DESC: Uses the same bounded zeroed arena contract as upstream Flite allocation.
// ------------------=
void *cst_safe_calloc(int size) { return cst_safe_alloc(size); }
// ------------------------=
// FUNC: cst_safe_realloc
// DESC: Grows an arena object without copying beyond either old or new allocation.
// ------------------=
void *cst_safe_realloc(void *old, int size) {
    void *result = cst_safe_alloc(size);
    if (old) {
        uintptr_t address = (uintptr_t)old;
        if (address < (uintptr_t)arena + 16 || address >= (uintptr_t)arena + used) fail(3);
        size_t bytes = *(size_t *)((unsigned char *)old - 16);
        if (bytes > (size_t)size) bytes = (size_t)size;
        memcpy(result, old, bytes);
    }
    return result;
}
// ------------------------=
// FUNC: cst_free
// DESC: Defers reclamation to the utterance boundary; there is never an unbounded process heap.
// ------------------=
void cst_free(void *value) { (void)value; checkpoint(); }
// ------------------------=
// FUNC: cst_errmsg
// DESC: Keeps upstream diagnostics off serial and out of the desktop-critical path.
// ------------------=
int cst_errmsg(const char *format, ...) { (void)format; return 0; }
// ------------------------=
// FUNC: infinity_flite_exit
// DESC: Converts upstream fatal errors to request failure, never machine termination.
// ------------------=
void exit(int value) { (void)value; fail(4); __builtin_unreachable(); }
// ------------------------=
// FUNC: infinity_flite_abort
// DESC: Converts unexpected library aborts to a bounded failed synthesis request.
// ------------------=
void abort(void) { fail(4); __builtin_unreachable(); }

// ------------------------=
// FUNC: infinity_flite_strlen
// DESC: Measures an internal null-terminated provider string.
// ------------------=
size_t strlen(const char *s) { size_t n = 0; while (s[n]) n++; return n; }
// ------------------------=
// FUNC: infinity_flite_memcpy
// DESC: Copies provider-owned byte spans without invoking a host library.
// ------------------=
void *memcpy(void *d, const void *s, size_t n) { unsigned char *a=d; const unsigned char *b=s; for(size_t i=0;i<n;i++) a[i]=b[i]; return d; }
// ------------------------=
// FUNC: infinity_flite_memmove
// DESC: Copies potentially overlapping provider buffers.
// ------------------=
void *memmove(void *d, const void *s, size_t n) { unsigned char *a=d; const unsigned char *b=s; if ((uintptr_t)a<(uintptr_t)b) return memcpy(d,s,n); while(n) {n--;a[n]=b[n];} return d; }
// ------------------------=
// FUNC: infinity_flite_memset
// DESC: Fills bounded provider-owned memory.
// ------------------=
void *memset(void *d, int c, size_t n) { unsigned char *a=d; for(size_t i=0;i<n;i++) a[i]=(unsigned char)c; return d; }
// ------------------------=
// FUNC: infinity_flite_memcmp
// DESC: Compares unsigned bytes with the C ordering contract.
// ------------------=
int memcmp(const void *a, const void *b, size_t n) { const unsigned char *x=a,*y=b; for(size_t i=0;i<n;i++) if(x[i]!=y[i]) return x[i]-y[i]; return 0; }
// ------------------------=
// FUNC: infinity_flite_strncmp
// DESC: Compares bounded strings for lexicon lookup.
// ------------------=
int strncmp(const char *a,const char *b,size_t n) { for(size_t i=0;i<n;i++) {int d=(unsigned char)a[i]-(unsigned char)b[i]; if(d||!a[i]) return d;} return 0; }
// ------------------------=
// FUNC: infinity_flite_strcmp
// DESC: Orders lexicon strings without locale or external state.
// ------------------=
int strcmp(const char *a,const char *b) { while(*a&&*a==*b){a++;b++;} return (unsigned char)*a-(unsigned char)*b; }
// ------------------------=
// FUNC: infinity_flite_strcpy
// DESC: Copies an internally sized string including its terminator.
// ------------------=
char *strcpy(char *d,const char *s) { memcpy(d,s,strlen(s)+1); return d; }
// ------------------------=
// FUNC: infinity_flite_strncpy
// DESC: Copies and zero-pads a fixed-width lexicon string field.
// ------------------=
char *strncpy(char *d,const char *s,size_t n) {size_t i=0;for(;i<n&&s[i];i++)d[i]=s[i];for(;i<n;i++)d[i]=0;return d;}
// ------------------------=
// FUNC: infinity_flite_strcat
// DESC: Appends to an upstream pre-sized internal string.
// ------------------=
char *strcat(char *d,const char *s) {strcpy(d+strlen(d),s);return d;}
// ------------------------=
// FUNC: infinity_flite_strchr
// DESC: Finds a byte including the terminating null.
// ------------------=
char *strchr(const char *s,int c) { do {if(*s==(char)c)return (char *)s;}while(*s++);return NULL;}
// ------------------------=
// FUNC: infinity_flite_strrchr
// DESC: Finds the last byte occurrence in an internal string.
// ------------------=
char *strrchr(const char *s,int c) {const char *found=NULL;do {if(*s==(char)c)found=s;}while(*s++);return (char *)found;}
// ------------------------=
// FUNC: infinity_flite_strstr
// DESC: Finds a bounded internal substring for text normalization.
// ------------------=
char *strstr(const char *s,const char *needle) {size_t n=strlen(needle);for(;;s++){if(!strncmp(s,needle,n))return (char *)s;if(!*s)return NULL;}}

// ------------------------=
// FUNC: infinity_flite_isdigit
// DESC: Recognizes ASCII decimal digits only.
// ------------------=
int isdigit(int c){return c>='0'&&c<='9';}
// ------------------------=
// FUNC: infinity_flite_isupper
// DESC: Recognizes ASCII uppercase letters only.
// ------------------=
int isupper(int c){return c>='A'&&c<='Z';}
// ------------------------=
// FUNC: infinity_flite_islower
// DESC: Recognizes ASCII lowercase letters only.
// ------------------=
int islower(int c){return c>='a'&&c<='z';}
// ------------------------=
// FUNC: infinity_flite_isalpha
// DESC: Recognizes the native English provider alphabet.
// ------------------=
int isalpha(int c){return isupper(c)||islower(c);}
// ------------------------=
// FUNC: infinity_flite_isalnum
// DESC: Classifies ASCII words and numbers for normalization.
// ------------------=
int isalnum(int c){return isalpha(c)||isdigit(c);}
// ------------------------=
// FUNC: infinity_flite_isspace
// DESC: Recognizes ASCII whitespace without locale tables.
// ------------------=
int isspace(int c){return c==' '||(c>=9&&c<=13);}
// ------------------------=
// FUNC: infinity_flite_ispunct
// DESC: Classifies printable punctuation for sentence segmentation.
// ------------------=
int ispunct(int c){return c>=33&&c<=126&&!isalnum(c);}
// ------------------------=
// FUNC: infinity_flite_tolower
// DESC: Folds ASCII uppercase letters for lexicon lookup.
// ------------------=
int tolower(int c){return isupper(c)?c+32:c;}
// ------------------------=
// FUNC: infinity_flite_toupper
// DESC: Folds ASCII lowercase letters for acronym expansion.
// ------------------=
int toupper(int c){return islower(c)?c-32:c;}
// ------------------------=
// FUNC: infinity_flite_abs
// DESC: Returns magnitude for bounded internal integer values.
// ------------------=
int abs(int c){return c<0?-c:c;}
// ------------------------=
// FUNC: infinity_flite_rand
// DESC: Supplies deterministic acoustic excitation only, never security entropy.
// ------------------=
int rand(void){noise_state=noise_state*1664525u+1013904223u;return (int)(noise_state>>1);}
// ------------------------=
// FUNC: infinity_flite_fabs
// DESC: Clears the IEEE-754 sign bit without depending on a target math library.
// ------------------=
double infinity_flite_fabs(double value){union {double d;uint64_t u;}v={.d=value};v.u&=UINT64_C(0x7fffffffffffffff);return v.d;}
// ------------------------=
// FUNC: infinity_flite_atof
// DESC: Parses bounded decimal pronunciation features without host locale dependencies.
// ------------------=
double atof(const char *s){while(isspace(*s))s++;int sign=1;if(*s=='-'||*s=='+'){if(*s=='-')sign=-1;s++;}double v=0;while(isdigit(*s))v=v*10+(*s++-'0');if(*s=='.'){s++;double scale=.1;while(isdigit(*s)){v+=(*s++-'0')*scale;scale*=.1;}}if(*s=='e'||*s=='E'){s++;int direction=1;if(*s=='-'||*s=='+'){if(*s=='-')direction=-1;s++;}int e=0;while(isdigit(*s)){e=e*10+(*s++-'0');if(e>308)fail(1);}while(e--)v*=direction>0?10:.1;}return sign*v;}
// ------------------------=
// FUNC: infinity_flite_atoi
// DESC: Parses bounded integer features and saturates rather than overflowing a signed integer.
// ------------------=
int atoi(const char *s){double v=atof(s);return v>2147483647.0?2147483647:v<-2147483648.0?(-2147483647-1):(int)v;}

// ------------------------=
// FUNC: format
// DESC: Implements the exact string/character/integer formatting used by the pinned compact voice.
// ------------------=
static int format(char *out,size_t capacity,const char *fmt,va_list args){
    size_t n=0;
#define EMIT(c) do { if(n+1<capacity) out[n]=(c); n++; } while(0)
    while(*fmt){if(*fmt!='%'){EMIT(*fmt++);continue;}fmt++;int precision=-1;
        if(*fmt=='.'){fmt++;precision=0;if(*fmt=='*'){precision=va_arg(args,int);fmt++;}else while(isdigit(*fmt))precision=precision*10+(*fmt++-'0');}
        char kind=*fmt++;
        if(kind=='s'){const char *s=va_arg(args,const char *);for(int i=0;s[i]&&(precision<0||i<precision);i++)EMIT(s[i]);}
        else if(kind=='c'){EMIT((char)va_arg(args,int));}
        else if(kind=='d'){int64_t v=va_arg(args,int);if(v<0){EMIT('-');v=-v;}char digits[20];int k=0;do{digits[k++]=(char)('0'+v%10);v/=10;}while(v);while(k)EMIT(digits[--k]);}
        else if(kind=='%'){EMIT('%');}else fail(4);
    }
    if(capacity)out[n<capacity?n:capacity-1]=0;
    return (int)n;
#undef EMIT
}
// ------------------------=
// FUNC: cst_sprintf
// DESC: Formats into upstream buffers whose sizes are derived from bounded input and fixed phoneme names.
// ------------------=
int cst_sprintf(char *out,const char *fmt,...){va_list a;va_start(a,fmt);int n=format(out,(size_t)-1,fmt,a);va_end(a);return n;}
// ------------------------=
// FUNC: infinity_flite_snprintf
// DESC: Formats UTF-8 extraction buffers with an explicit capacity.
// ------------------=
int snprintf(char *out,size_t capacity,const char *fmt,...){va_list a;va_start(a,fmt);int n=format(out,capacity,fmt,a);va_end(a);return n;}

// ------------------------=
// FUNC: cst_fopen
// DESC: Rejects file access; this provider accepts only compiled voice data and caller-owned text.
// ------------------=
cst_file cst_fopen(const char *path,int mode){(void)path;(void)mode;fail(4);return NULL;}
// ------------------------=
// FUNC: cst_fgetc
// DESC: Fails closed if an unexpected file-backed token stream is requested.
// ------------------=
int cst_fgetc(cst_file file){(void)file;fail(4);return -1;}
// ------------------------=
// FUNC: cst_fclose
// DESC: Fails closed for unsupported file-backed streams.
// ------------------=
int cst_fclose(cst_file file){(void)file;fail(4);return -1;}
// ------------------------=
// FUNC: cst_fprintf
// DESC: Discards optional library diagnostics rather than sending captured text to serial output.
// ------------------=
int cst_fprintf(cst_file file,const char *fmt,...){(void)file;(void)fmt;return 0;}
// ------------------------=
// FUNC: delete_cg_db
// DESC: Rejects unsupported dynamic statistical voices instead of accepting partial implementations.
// ------------------=
void delete_cg_db(cst_cg_db *db){if(db)fail(4);}
// ------------------------=
// FUNC: delete_audio_streaming_info
// DESC: Reclaims callback metadata at the bounded arena reset boundary.
// ------------------=
void delete_audio_streaming_info(cst_audio_streaming_info *info){cst_free(info);}
CST_VAL_REGISTER_TYPE(cg_db,cst_cg_db)
CST_VAL_REGISTER_TYPE(audio_streaming_info,cst_audio_streaming_info)
// ------------------------=
// FUNC: cst_munmap_file
// DESC: Rejects unexpected mapped files; this provider has no file mapping capability.
// ------------------=
int cst_munmap_file(cst_filemap *map){if(map)fail(4);return 0;}

// ------------------------=
// FUNC: infinity_flite_synthesize
// DESC: Produces genuine CMU diphone speech in bounded caller PCM, catching errors and erasing request state.
// ------------------=
int infinity_flite_synthesize(const unsigned char *text,size_t length,int16_t *pcm,size_t capacity,
                             size_t *frames,size_t *high_water,int (*cancel)(void),size_t memory_limit){
    if(active)return 5;
    if(!text||!pcm||!frames||!high_water||!length||length>160)return 1;
    *frames=0;*high_water=0;
    for(size_t i=0;i<length;i++)if(text[i]<32||text[i]>126)return 1;
    char input[161];memcpy(input,text,length);input[length]=0;
    used=0;peak=0;budget=memory_limit<ARENA_BYTES?memory_limit:ARENA_BYTES;
    cmu_us_kal16_diphone=NULL;
    noise_state=1;
    failure=0;cancelled=cancel;active=1;cst_errjmp=&recovery;
    if(setjmp(recovery)==0){
        checkpoint();
        flite_init();register_cmu_us_kal16(NULL);
        cst_utterance *utterance=flite_synth_text(input,cmu_us_kal16_diphone);
        if(!utterance)fail(4);
        cst_wave *wave=utt_wave(utterance);
        if(!wave||wave->sample_rate!=16000||wave->num_channels!=1||wave->num_samples<0)fail(4);
        if((size_t)wave->num_samples>capacity)fail(3);
        checkpoint();
        memcpy(pcm,wave->samples,(size_t)wave->num_samples*sizeof(int16_t));
        *frames=(size_t)wave->num_samples;
    } else if(!failure) failure=4;
    *high_water=peak;
    cmu_us_kal16_diphone=NULL;
    for(size_t i=0;i<used;i++)((volatile unsigned char *)arena)[i]=0;
    for(size_t i=0;i<sizeof(input);i++)((volatile char *)input)[i]=0;
    used=0;active=0;cancelled=NULL;cst_errjmp=NULL;
    return failure;
}
// ------------------------=
// FUNC: infinity_flite_clean
// DESC: Diagnoses actual PCM/text arena erasure without exposing its private contents.
// ------------------=
int infinity_flite_clean(void){if(active||used)return 0;for(size_t i=0;i<ARENA_BYTES;i++)if(arena[i])return 0;return 1;}
