#include <stddef.h>
// ------------------------=
// FUNC: memcpy
// DESC: Supplies freestanding private-memory copies without a host C runtime.
// ------------------=
void *memcpy(void *to,const void *from,size_t n) {unsigned char *d=to;const unsigned char *s=from;for(size_t i=0;i<n;i++)d[i]=s[i];return to;}
// ------------------------=
// FUNC: memmove
// DESC: Handles overlapping private memory spans for compiler-generated runtime operations.
// ------------------=
void *memmove(void *to,const void *from,size_t n) {unsigned char *d=to;const unsigned char *s=from;if(d<s){for(size_t i=0;i<n;i++)d[i]=s[i];}else{while(n){n--;d[n]=s[n];}}return to;}
// ------------------------=
// FUNC: memset
// DESC: Initializes worker-owned memory without importing allocator or operating-system services.
// ------------------=
void *memset(void *to,int value,size_t n) {unsigned char *d=to;for(size_t i=0;i<n;i++)d[i]=(unsigned char)value;return to;}
// ------------------------=
// FUNC: memcmp
// DESC: Compares non-secret decoder metadata and compiler buffers; not used for biometric authentication decisions.
// ------------------=
int memcmp(const void *a,const void *b,size_t n) {const unsigned char *x=a,*y=b;for(size_t i=0;i<n;i++)if(x[i]!=y[i])return x[i]<y[i]?-1:1;return 0;}
// ------------------------=
// FUNC: strlen
// DESC: Provides freestanding length for decoder-owned constant error strings.
// ------------------=
size_t strlen(const char *s) {size_t n=0;while(s[n])n++;return n;}
// ------------------------=
// FUNC: abs
// DESC: Computes the bounded signed decoder coefficient magnitude.
// ------------------=
int abs(int value) {return value<0?-value:value;}
