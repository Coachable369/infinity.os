#ifndef INFINITY_SERVO_INET_H
#define INFINITY_SERVO_INET_H
/* Byte-order operations only. This header grants no socket or host access. */
#include <stdint.h>
// ------------------------=
// FUNC: htons
// DESC: Converts a 16-bit host value to network byte order exactly once.
// ------------------=
static inline uint16_t htons(uint16_t value) {
#if __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
    return __builtin_bswap16(value);
#else
    return value;
#endif
}
// ------------------------=
// FUNC: ntohs
// DESC: Converts a 16-bit network value to host byte order.
// ------------------=
static inline uint16_t ntohs(uint16_t value) { return htons(value); }
// ------------------------=
// FUNC: htonl
// DESC: Converts a 32-bit host value to network byte order exactly once.
// ------------------=
static inline uint32_t htonl(uint32_t value) {
#if __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
    return __builtin_bswap32(value);
#else
    return value;
#endif
}
// ------------------------=
// FUNC: ntohl
// DESC: Converts a 32-bit network value to host byte order.
// ------------------=
static inline uint32_t ntohl(uint32_t value) { return htonl(value); }
#endif
