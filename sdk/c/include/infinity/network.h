#ifndef INFINITY_NETWORK_H
#define INFINITY_NETWORK_H
#include <infinity/syscall.h>
#include <stddef.h>
#define INFINITY_NETWORK_LIMIT 16384u
enum { INFINITY_NET_PENDING=1, INFINITY_NET_CONNECTED=2, INFINITY_NET_COMPLETE=4,
       INFINITY_NET_CLOSED=5, INFINITY_NET_FAILED=6, INFINITY_NET_RESOLVED=7 };
typedef struct { uint64_t state, count; unsigned char address[4], reserved[4]; } InfinityNetworkStatus;
// ------------------------=
// FUNC: infinity_network_open
// DESC: Begins an authorized IPv4 TCP connection or DNS-only query and returns its generation-fenced handle.
// ------------------=
static inline int64_t infinity_network_open(const char *host, size_t length, unsigned port, int resolve_only) {
    if (!host || !length || length>253 || !port || port>65535 || (resolve_only!=0 && resolve_only!=1)) return -22;
    return infinity_system_call(0x1270,resolve_only,(uintptr_t)host,length,port,0,0);
}
// ------------------------=
// FUNC: infinity_network_command
// DESC: Queues a bounded read, write or verified TLS upgrade without blocking the application loop.
// ------------------=
static inline int64_t infinity_network_command(uint64_t handle, unsigned operation, const void *bytes, size_t count) {
    if (operation>2 || count>INFINITY_NETWORK_LIMIT || (operation && !bytes)
        || (operation==2 && (!count || count>253))) return -22;
    return infinity_system_call(0x1271,handle,operation,(uintptr_t)bytes,count,0,0);
}
// ------------------------=
// FUNC: infinity_network_poll
// DESC: Retrieves native state and completed byte count; callers yield while progress is pending.
// ------------------=
static inline int64_t infinity_network_poll(uint64_t handle, InfinityNetworkStatus *status) {
    if (!status) return -22;
    return infinity_system_call(0x1272,handle,(uintptr_t)status,0,0,0,0);
}
// ------------------------=
// FUNC: infinity_network_receive
// DESC: Copies completed incoming bytes into caller memory without exceeding the native page limit.
// ------------------=
static inline int64_t infinity_network_receive(uint64_t handle, void *bytes, size_t capacity) {
    if (!bytes || capacity>INFINITY_NETWORK_LIMIT) return -22;
    return infinity_system_call(0x1273,handle,(uintptr_t)bytes,capacity,0,0,0);
}
// ------------------------=
// FUNC: infinity_network_close
// DESC: Cancels and releases exactly this application's network channel.
// ------------------=
static inline int64_t infinity_network_close(uint64_t handle) {
    return infinity_system_call(0x1274,handle,0,0,0,0,0);
}
#endif
