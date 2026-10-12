#ifndef INFINITY_COMPILER_STDDEF_H
#define INFINITY_COMPILER_STDDEF_H
/* Freestanding C types supplied by the compiler's target data model. */
typedef __SIZE_TYPE__ size_t;
typedef __PTRDIFF_TYPE__ ptrdiff_t;
typedef __WCHAR_TYPE__ wchar_t;
typedef struct { long long integer; long double real; } max_align_t;
#define NULL ((void *)0)
#define offsetof(type, member) __builtin_offsetof(type, member)
#endif
