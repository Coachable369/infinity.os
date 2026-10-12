#ifndef INFINITY_EVENT_H
#define INFINITY_EVENT_H
#include <stdint.h>
#include <infinity/syscall.h>

typedef struct InfinityEvent {
    uint32_t version, type;
    uint64_t sequence, timestamp;
    uint32_t payload_length, sequence_gap;
    uint8_t payload[96];
} InfinityEvent;
_Static_assert(sizeof(InfinityEvent) == 128, "Infinity event ABI");

// ------------------------=
// FUNC: infinity_event_subscribe
// DESC: Subscribes to an exact authorized topic and scope with one to eight queued events and a finite lease.
// ------------------=
static inline int64_t infinity_event_subscribe(uint64_t capability, uint32_t type,
    uint64_t scope, uint64_t expires_at, uint32_t queue_limit) {
    return infinity_system_call(INFINITY_SYS_EVENT_SUBSCRIBE, capability, type, scope, expires_at, queue_limit, 0);
}

// ------------------------=
// FUNC: infinity_event_poll
// DESC: Returns one event, zero when idle, or a negative error; a sequence gap requires state reconciliation.
// ------------------=
static inline int64_t infinity_event_poll(uint64_t lease, InfinityEvent *event) {
    return infinity_system_call(INFINITY_SYS_EVENT_POLL, lease, (uintptr_t)event, sizeof(*event), 0, 0, 0);
}

// ------------------------=
// FUNC: infinity_event_unsubscribe
// DESC: Releases this process's subscription and queued data, including after its read authority expires.
// ------------------=
static inline int64_t infinity_event_unsubscribe(uint64_t lease) {
    return infinity_system_call(INFINITY_SYS_EVENT_UNSUBSCRIBE, lease, 0, 0, 0, 0, 0);
}
#endif
