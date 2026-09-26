#ifndef INFINITY_SERVO_C_TARGET_H
#define INFINITY_SERVO_C_TARGET_H
#define __INFINITYOS__ 1
/* ICU must consume packaged memory data, never host/filesystem ICU paths. */
#define UCONFIG_NO_FILE_IO 1
/* Declaration selection for the isolated native port, not a libc implementation.
 * Native pthread/time/entropy symbols remain required at executable link time. */
#define _GNU_SOURCE 1
#define _POSIX_THREADS 1
#define _POSIX_READER_WRITER_LOCKS 200809L
#define _UNIX98_THREAD_MUTEX_ATTRIBUTES 1
#define _POSIX_TIMERS 1
#define _POSIX_MONOTONIC_CLOCK 200809L
/* Crypto consumers must use native transport/storage/UI adapters, not BIO
 * sockets, host files or /dev/tty. Cryptography and entropy remain enabled. */
#define OPENSSL_NO_SOCK 1
#define OPENSSL_NO_FILESYSTEM 1
#define OPENSSL_NO_TTY 1
#ifndef __ASSEMBLER__
#include <stdint.h>
#endif
#endif
