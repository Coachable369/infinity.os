#ifndef INFINITY_COMPILER_DLFCN_H
#define INFINITY_COMPILER_DLFCN_H
/* Compatibility declarations, not authority to load native code. The initial
   static compiler payload does not provide a dynamic linker or symbol registry. */
#define RTLD_LAZY 1
#define RTLD_NOW 2
#define RTLD_LOCAL 0
#define RTLD_GLOBAL 4
#define RTLD_DEFAULT ((void *)0)
#define RTLD_NEXT ((void *)-1)
typedef struct {
    const char *dli_fname;
    void *dli_fbase;
    const char *dli_sname;
    void *dli_saddr;
} Dl_info;
#ifdef __cplusplus
extern "C" {
#endif
void *dlopen(const char *, int);
void *dlsym(void *, const char *);
int dlclose(void *);
char *dlerror(void);
int dladdr(const void *, Dl_info *);
#ifdef __cplusplus
}
#endif
#endif
