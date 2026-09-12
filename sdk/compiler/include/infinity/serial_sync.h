#ifndef INFINITY_SERIAL_SYNC_H
#define INFINITY_SERIAL_SYNC_H
#include <stdint.h>
#define INFINITY_MUTEX_INITIALIZER UINT32_MAX
int infinity_mutex_init(uint32_t *, int);
int infinity_mutex_lock(uint32_t *, int);
int infinity_mutex_unlock(uint32_t *);
int infinity_mutex_destroy(uint32_t *);
#endif
