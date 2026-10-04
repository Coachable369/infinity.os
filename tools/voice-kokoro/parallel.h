#ifndef INFINITY_NATIVE_PARALLEL_MATH_H
#define INFINITY_NATIVE_PARALLEL_MATH_H
#include <stddef.h>

// Optional native kernel adapter. Standalone single-core probes and machines
// without helper capacity execute the same math on the owning speech worker.
// The hook joins every accepted helper before returning. Callbacks may only
// read immutable inputs and write exclusively claimed output ranges: no heap,
// thread-local state, native error/longjmp path, or recursive dispatch.
#ifdef __cplusplus
extern "C" {
#endif
// ------------------------=
// FUNC: infinity_speech_parallel
// DESC: Borrows idle native workers and joins every accepted pure-math callback before returning.
// ------------------=
extern size_t infinity_speech_parallel(void (*task)(void *), void *context)
    __attribute__((weak));
#ifdef __cplusplus
}
#endif

// ------------------------=
// FUNC: native_parallel_math
// DESC: Runs allocation-free math through an optional bounded native helper cohort, always completing and joining before return.
// ------------------=
static inline size_t native_parallel_math(void (*task)(void *), void *context) {
    if (infinity_speech_parallel) return infinity_speech_parallel(task, context);
    task(context);
    return 0;
}
#endif
