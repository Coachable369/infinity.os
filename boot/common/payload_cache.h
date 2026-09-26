/* Firmware-independent read capability. Memory remains loader-owned after exit. */
#define PAYLOAD_PART_BYTES UINT64_C(536870912)
#define PAYLOAD_PARTS 16
typedef struct { const uint8_t *bytes; uint64_t length; } PayloadPart;
static PayloadPart payload_cache[2][PAYLOAD_PARTS];

// ------------------------=
// FUNC: payload_cached_read
// DESC: Copies a validated bounded range across immutable preloaded shards without firmware calls.
// ------------------=
static uint64_t EFIAPI payload_cached_read(uint32_t kind, uint64_t offset, size_t length, void *destination) {
    if (kind >= 2 || length > 1024 * 1024 || (!destination && length)) return 1;
    /* Validate the entire request first so a failed read never partially writes. */
    uint64_t cursor = offset;
    size_t remaining = length;
    while (remaining) {
        uint64_t part = cursor / PAYLOAD_PART_BYTES, within = cursor % PAYLOAD_PART_BYTES;
        if (part >= PAYLOAD_PARTS || !payload_cache[kind][part].bytes ||
            within >= payload_cache[kind][part].length) return 1;
        size_t count = remaining;
        uint64_t available = payload_cache[kind][part].length - within;
        if (count > available) count = (size_t)available;
        cursor += count; remaining -= count;
        if (remaining && cursor % PAYLOAD_PART_BYTES) return 1;
    }
    uint8_t *out = destination;
    while (length) {
        uint64_t part = offset / PAYLOAD_PART_BYTES, within = offset % PAYLOAD_PART_BYTES;
        size_t count = length;
        if (count > PAYLOAD_PART_BYTES - within) count = (size_t)(PAYLOAD_PART_BYTES - within);
        memcpy(out, payload_cache[kind][part].bytes + within, count);
        offset += count; out += count; length -= count;
    }
    return 0;
}
