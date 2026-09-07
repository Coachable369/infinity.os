#ifndef INFINITY_VIDEO_MODES_H
#define INFINITY_VIDEO_MODES_H
#include <stdint.h>
#include <stddef.h>

#define INFINITY_VIDEO_INVALID UINT32_MAX
#define INFINITY_VIDEO_BUFFER_PIXELS (UINT64_C(3840) * 2160)

// ------------------------=
// FUNC: infinity_video_format
// DESC: Normalizes directly writable 32-bit GOP modes to the unchanged kernel RGB/BGR ABI.
// ------------------=
static uint32_t infinity_video_format(uint32_t format, const uint32_t masks[4]) {
    if (format <= 1) return format;
    if (format != 2 || masks[1] != 0x0000ff00 || masks[3] != 0xff000000)
        return INFINITY_VIDEO_INVALID;
    if (masks[0] == 0x000000ff && masks[2] == 0x00ff0000) return 0;
    if (masks[0] == 0x00ff0000 && masks[2] == 0x000000ff) return 1;
    return INFINITY_VIDEO_INVALID;
}

// ------------------------=
// FUNC: infinity_video_score
// DESC: Ranks valid modes, preferring persistent-buffer capacity over an oversized direct-paint mode.
// ------------------=
static uint64_t infinity_video_score(uint32_t width, uint32_t height, uint32_t stride,
                                     uint32_t format, const uint32_t masks[4]) {
    if (!width || !height || stride < width || width > 16384 || height > 16384 ||
        stride > 32768 || infinity_video_format(format, masks) == INFINITY_VIDEO_INVALID) return 0;
    uint64_t pixels = (uint64_t)width * height;
    uint64_t storage = (uint64_t)stride * height;
    return pixels + (storage <= INFINITY_VIDEO_BUFFER_PIXELS ? UINT64_C(1) << 40 : 0);
}

// ------------------------=
// FUNC: infinity_video_memory_valid
// DESC: Checks the complete scanout allocation before publishing an address to the kernel.
// ------------------=
static int infinity_video_memory_valid(uint64_t base, uint64_t size, uint32_t stride, uint32_t height) {
    uint64_t required = (uint64_t)stride * height * 4;
    return base && !(base & 3) && required && required <= size && size <= UINT64_MAX - base;
}
#endif
