#include <stdint.h>
#include <stdarg.h>
#include <stdio.h>
extern void infinity_probe_diagnostic(const char *);
extern void infinity_probe_bytes(const char *,size_t);
extern void infinity_probe_fatal(uint64_t, const char *, const char *, uint64_t);
// ------------------------=
// FUNC: __wrap_abort
// DESC: Reports a real C abort callsite before terminating the disposable native guest.
// ------------------=
__attribute__((noreturn)) void __wrap_abort(void) {
    infinity_probe_fatal(0, 0, 0, (uintptr_t)__builtin_return_address(0));
    __builtin_trap();
}
// ------------------------=
// FUNC: __wrap___assert_func
// DESC: Preserves native assertion diagnostics without depending on nonexistent Unix stderr.
// ------------------=
__attribute__((noreturn)) void __wrap___assert_func(const char *file, int line,
                                                  const char *function, const char *condition) {
    (void)function;
    infinity_probe_fatal((uint64_t)line, file, condition, (uintptr_t)__builtin_return_address(0));
    __builtin_trap();
}
// ------------------------=
// FUNC: __wrap_fprintf
// DESC: Sends bounded C engine diagnostics to the fixture UART instead of a nonexistent Unix descriptor.
// ------------------=
int __wrap_fprintf(FILE *stream, const char *format, ...) {
    (void)stream;
    char buffer[512];
    va_list args;
    va_start(args,format);
    int length=vsnprintf(buffer,sizeof(buffer),format,args);
    va_end(args);
    if (length>=0) infinity_probe_diagnostic(buffer);
    return length<512 ? length : 511;
}
// ------------------------=
// FUNC: __wrap_fputs
// DESC: Preserves bounded engine diagnostics without making any filesystem service succeed.
// ------------------=
int __wrap_fputs(const char *message,FILE *stream) {
    (void)stream;
    infinity_probe_diagnostic(message);
    return 0;
}
// ------------------------=
// FUNC: __wrap_fwrite
// DESC: Reports at most one bounded native diagnostic block with an honest partial-write count.
// ------------------=
size_t __wrap_fwrite(const void *bytes,size_t size,size_t count,FILE *stream) {
    (void)stream;
    if (!size || size>512) return 0;
    size_t accepted=count<512/size ? count : 512/size;
    infinity_probe_bytes(bytes,accepted*size);
    return accepted;
}
