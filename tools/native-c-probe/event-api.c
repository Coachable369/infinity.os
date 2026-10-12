#include <stddef.h>
#include <infinity/event.h>

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
