#ifndef INFINITY_SERIAL_TLS_H
#define INFINITY_SERIAL_TLS_H
#include <stdint.h>
int infinity_key_create(uint32_t *, void (*)(void *));
int infinity_key_set(uint32_t, const void *);
void *infinity_key_get(uint32_t);
int infinity_thread_destructor(void (*)(void *), void *);
void infinity_thread_cleanup(void);
#endif
