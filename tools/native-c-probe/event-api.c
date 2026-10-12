#include <stddef.h>
#include <infinity/event.h>
#include <infinity/surface.h>
#include <infinity/input.h>

_Static_assert(offsetof(InfinityEvent, sequence) == 8, "sequence offset");
_Static_assert(offsetof(InfinityEvent, timestamp) == 16, "timestamp offset");
_Static_assert(offsetof(InfinityEvent, payload_length) == 24, "length offset");
_Static_assert(offsetof(InfinityEvent, payload) == 32, "payload offset");

// ------------------------=
// FUNC: native_event_api_codegen_probe
// DESC: Forces target code generation for each C event wrapper; this is a compile check, not installed runtime proof.
// ------------------=
int64_t native_event_api_codegen_probe(uint64_t authority, InfinityEvent *event) {
    int64_t lease = infinity_event_subscribe(authority, 77, 9, 100, 2);
    if (lease < 0) return lease;
    int64_t result = infinity_event_poll((uint64_t)lease, event);
    int64_t closed = infinity_event_unsubscribe((uint64_t)lease);
    return closed < 0 ? closed : result;
}

// ------------------------=
// FUNC: native_surface_api_codegen_probe
// DESC: Compiles every retained-pixel wrapper for the selected native CPU ABI; not a runtime authority fixture.
// ------------------=
int64_t native_surface_api_codegen_probe(uint64_t authority, const uint32_t *pixels) {
    int64_t surface = infinity_surface_create(authority, 128, 64, INFINITY_XRGB8888);
    if (surface < 0) return surface;
    infinity_surface_upload(authority, (uint32_t)surface, pixels, 0, 512);
    infinity_surface_commit(authority, (uint32_t)surface, 1);
    infinity_surface_resize(authority, (uint32_t)surface, 64, 64);
    return infinity_surface_destroy(authority, (uint32_t)surface);
}
