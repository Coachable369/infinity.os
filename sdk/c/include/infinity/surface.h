#ifndef INFINITY_SURFACE_H
#define INFINITY_SURFACE_H
#include <infinity/window.h>

enum InfinityPixelFormat { INFINITY_XRGB8888 = 0, INFINITY_ARGB8888 = 1 };

// ------------------------=
// FUNC: infinity_surface_create
// DESC: Allocates zeroed retained pixels; returns a positive surface ID or a negative error. No global framebuffer mapping is granted.
// ------------------=
static inline int64_t infinity_surface_create(InfinityCapability authority, uint32_t width,
    uint32_t height, enum InfinityPixelFormat format) {
    return infinity_system_call(INFINITY_SYS_SURFACE_CREATE, authority, width, height, format, 0, 0);
}

// ------------------------=
// FUNC: infinity_surface_upload
// DESC: Copies up to 64 KiB of pixel-aligned data into private staging storage; displayed pixels remain unchanged until commit.
// ------------------=
static inline int64_t infinity_surface_upload(InfinityCapability publish, InfinitySurfaceId surface,
    const uint32_t *pixels, size_t byte_offset, size_t byte_length) {
    return infinity_system_call(INFINITY_SYS_SURFACE_UPLOAD, publish, surface, (uintptr_t)pixels, byte_offset, byte_length, 0);
}

// ------------------------=
// FUNC: infinity_surface_commit
// DESC: Publishes staged pixels with a strictly increasing generation (initially zero); unchanged pixels persist across commits.
// ------------------=
static inline int64_t infinity_surface_commit(InfinityCapability publish, InfinitySurfaceId surface, uint64_t generation) {
    return infinity_system_call(INFINITY_SYS_SURFACE_COMMIT, publish, surface, generation, 0, 0, 0);
}

// ------------------------=
// FUNC: infinity_surface_resize
// DESC: Preserves committed overlap, clears new pixels and discards uncommitted uploads; success advances the current generation by one.
// ------------------=
static inline int64_t infinity_surface_resize(InfinityCapability resize, InfinitySurfaceId surface, uint32_t width, uint32_t height) {
    return infinity_system_call(INFINITY_SYS_SURFACE_RESIZE, resize, surface, width, height, 0, 0);
}

// ------------------------=
// FUNC: infinity_surface_destroy
// DESC: Releases owned pixel memory after its window has closed; another application's surface cannot be destroyed.
// ------------------=
static inline int64_t infinity_surface_destroy(InfinityCapability destroy, InfinitySurfaceId surface) {
    return infinity_system_call(INFINITY_SYS_SURFACE_DESTROY, destroy, surface, 0, 0, 0, 0);
}
#endif
