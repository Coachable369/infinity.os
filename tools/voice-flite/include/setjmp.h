#ifndef INFINITY_FLITE_SETJMP_H
#define INFINITY_FLITE_SETJMP_H
#include <stdint.h>
typedef uint64_t jmp_buf[24];
int infinity_flite_setjmp(jmp_buf) __attribute__((returns_twice));
void infinity_flite_longjmp(jmp_buf, int) __attribute__((noreturn));
#define setjmp infinity_flite_setjmp
#define longjmp infinity_flite_longjmp
#endif
