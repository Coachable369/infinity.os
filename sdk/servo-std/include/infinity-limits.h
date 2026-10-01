#ifndef INFINITY_NATIVE_LIMITS_H
#define INFINITY_NATIVE_LIMITS_H
/* Matches native::MAX_THREADS; root is a separate lock reader, not a worker. */
#define INFINITY_NATIVE_THREADS 64
#define INFINITY_NATIVE_READERS (INFINITY_NATIVE_THREADS + 1)
/* SpiderMonkey allocates locks for script compilation and runtime objects. */
#define INFINITY_NATIVE_MUTEXES 4096
#endif
