#include <assert.h>
#include "../boot/common/video_modes.h"

// ------------------------=
// FUNC: main
// DESC: Exercises native-format normalization, high-resolution mode policy and framebuffer bounds.
// ------------------=
int main(void) {
    uint32_t rgb[] = {0xff, 0xff00, 0xff0000, 0xff000000};
    uint32_t bgr[] = {0xff0000, 0xff00, 0xff, 0xff000000};
    uint32_t rgb565[] = {0xf800, 0x7e0, 0x1f, 0};
    assert(infinity_video_format(2, rgb) == 0);
    assert(infinity_video_format(2, bgr) == 1);
    assert(infinity_video_format(2, rgb565) == INFINITY_VIDEO_INVALID);
    assert(infinity_video_format(3, rgb) == INFINITY_VIDEO_INVALID);
    assert(!infinity_video_score(1920, 1080, 640, 0, rgb));
    assert(infinity_video_score(800, 600, 800, 0, rgb) > 0);
    assert(infinity_video_score(2560, 1440, 2560, 2, bgr) > infinity_video_score(1920, 1080, 1920, 0, rgb));
    assert(infinity_video_score(3840, 2160, 3840, 0, rgb) > infinity_video_score(7680, 4320, 7680, 0, rgb));
    assert(infinity_video_score(5120, 1440, 5120, 1, bgr) > infinity_video_score(800, 600, 800, 0, rgb));
    assert(infinity_video_memory_valid(0x80000000, 1920 * 1080 * 4, 1920, 1080));
    assert(!infinity_video_memory_valid(0x80000000, 1024, 1920, 1080));
    assert(!infinity_video_memory_valid(UINT64_MAX - 3, 4096, 16, 16));
    return 0;
}
