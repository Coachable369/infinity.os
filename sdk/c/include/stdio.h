#ifndef INFINITY_STDIO_H
#define INFINITY_STDIO_H
#include <stddef.h>
#include <stdarg.h>
#define EOF (-1)
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2
#define FOPEN_MAX 8
typedef struct InfinityFile FILE;
FILE *fopen(const char *path, const char *mode);
int fclose(FILE *stream);
int fflush(FILE *stream);
size_t fread(void *data, size_t size, size_t count, FILE *stream);
size_t fwrite(const void *data, size_t size, size_t count, FILE *stream);
int fseek(FILE *stream, long offset, int origin);
long ftell(FILE *stream);
void rewind(FILE *stream);
int feof(FILE *stream);
int ferror(FILE *stream);
void clearerr(FILE *stream);
int fgetc(FILE *stream);
int fputc(int character, FILE *stream);
char *fgets(char *data, int size, FILE *stream);
int fputs(const char *text, FILE *stream);
int puts(const char *text);
int putchar(int character);
int printf(const char *format, ...);
int snprintf(char *output, size_t size, const char *format, ...);
int vsnprintf(char *output, size_t size, const char *format, va_list arguments);
#endif
