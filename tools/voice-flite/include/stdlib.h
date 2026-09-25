#ifndef INFINITY_FLITE_STDLIB_H
#define INFINITY_FLITE_STDLIB_H
#include <stddef.h>
void *malloc(size_t);
void *calloc(size_t, size_t);
void *realloc(void *, size_t);
void free(void *);
int atoi(const char *);
double atof(const char *);
int abs(int);
int rand(void);
#define RAND_MAX 2147483647
void abort(void) __attribute__((noreturn));
void exit(int) __attribute__((noreturn));
void qsort(void *, size_t, size_t, int (*)(const void *, const void *));
#endif
