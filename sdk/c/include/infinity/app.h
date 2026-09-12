#ifndef INFINITY_APP_H
#define INFINITY_APP_H
#include <stddef.h>
#include <stdint.h>
#define INFINITY_APP_ABI 1u
typedef struct InfinityAppApi {
    uint32_t version;
    uint32_t size;
    void *context;
    int (*write_stdout)(void *context, const unsigned char *bytes, size_t length);
} InfinityAppApi;
#endif
