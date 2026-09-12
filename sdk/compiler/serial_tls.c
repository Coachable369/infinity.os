#include <infinity/serial_tls.h>
#include <errno.h>
#include <stddef.h>
struct Key { const void *value; void (*destroy)(void *); };
struct Destructor { void (*call)(void *); void *argument; };
static struct Key keys[128];
static uint32_t key_count;
static struct Destructor destructors[256];
static size_t destructor_count;
static int cleaning;
// ------------------------=
// FUNC: infinity_key_create
// DESC: Allocates a bounded nonrecycled key for the single invocation thread.
// ------------------=
int infinity_key_create(uint32_t *out, void (*destroy)(void *)) {
    if (!out) return EINVAL;
    if (key_count == 128) return EAGAIN;
    keys[key_count].destroy = destroy;
    *out = ++key_count;
    return 0;
}
// ------------------------=
// FUNC: infinity_key_set
// DESC: Sets a valid invocation-local value without aliasing invalid keys.
// ------------------=
int infinity_key_set(uint32_t key, const void *value) {
    if (!key || key > key_count) return EINVAL;
    keys[key - 1].value = value; return 0;
}
// ------------------------=
// FUNC: infinity_key_get
// DESC: Retrieves the sole thread's value, returning null for invalid keys.
// ------------------=
void *infinity_key_get(uint32_t key) {
    return key && key <= key_count ? (void *)keys[key - 1].value : 0;
}
// ------------------------=
// FUNC: infinity_thread_destructor
// DESC: Registers bounded static-image thread-local cleanup in LIFO order.
// ------------------=
int infinity_thread_destructor(void (*call)(void *), void *argument) {
    if (!call) return EINVAL;
    if (cleaning || destructor_count == 256) return EAGAIN;
    destructors[destructor_count++] = (struct Destructor){call, argument}; return 0;
}
// ------------------------=
// FUNC: infinity_thread_cleanup
// DESC: Runs bounded thread-local cleanup and four key-destructor iterations.
// ------------------=
void infinity_thread_cleanup(void) {
    if (cleaning) return;
    cleaning = 1;
    while (destructor_count) {
        struct Destructor value = destructors[--destructor_count];
        value.call(value.argument);
    }
    for (unsigned pass = 0; pass < 4; ++pass) {
        uint32_t limit = key_count;
        for (uint32_t i = 0; i < limit; ++i) {
            void *value = (void *)keys[i].value;
            keys[i].value = 0;
            if (value && keys[i].destroy) keys[i].destroy(value);
        }
    }
    for (uint32_t i = 0; i < key_count; ++i) keys[i].value = 0;
    cleaning = 0;
}
