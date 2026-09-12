#ifndef INFINITY_APP_H
#define INFINITY_APP_H
#include <stddef.h>
#include <stdint.h>
#define INFINITY_APP_ABI 2u
typedef struct InfinityObjectApi {
    int (*open)(void *, const unsigned char *, size_t, uint32_t);
    ptrdiff_t (*read)(void *, int, unsigned char *, size_t);
    ptrdiff_t (*write)(void *, int, const unsigned char *, size_t);
    int64_t (*seek)(void *, int, int64_t, uint32_t);
    int (*flush)(void *, int);
    int (*close)(void *, int);
} InfinityObjectApi;
typedef struct InfinityAppApi {
    uint32_t version;
    uint32_t size;
    void *context;
    int (*write_stdout)(void *context, const unsigned char *bytes, size_t length);
    const InfinityObjectApi *objects;
} InfinityAppApi;
#endif
