#ifndef INFINITY_FLITE_STDIO_H
#define INFINITY_FLITE_STDIO_H
#include <stddef.h>
#include <stdarg.h>
typedef struct InfinityFliteFile FILE;
#define EOF (-1)
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2
extern FILE *stderr;
extern FILE *stdin;
extern FILE *stdout;
int sprintf(char *, const char *, ...);
int snprintf(char *, size_t, const char *, ...);
int vsprintf(char *, const char *, va_list);
int fprintf(FILE *, const char *, ...);
int printf(const char *, ...);
int sscanf(const char *, const char *, ...);
#endif
