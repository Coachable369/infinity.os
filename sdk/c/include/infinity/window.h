#ifndef INFINITY_WINDOW_H
#define INFINITY_WINDOW_H
#include <stdint.h>
#include <stddef.h>
#include <infinity/syscall.h>

typedef uint32_t InfinityWindowId;
typedef uint32_t InfinitySurfaceId;
typedef uint64_t InfinityCapability;
enum InfinityWindowState {
    INFINITY_WINDOW_NORMAL = 0,
    INFINITY_WINDOW_MINIMIZED = 1,
    INFINITY_WINDOW_MAXIMIZED = 2,
    INFINITY_WINDOW_FULLSCREEN = 3
};
enum InfinityWindowError {
    INFINITY_WINDOW_DENIED = -1,
    INFINITY_WINDOW_UNKNOWN = -2,
    INFINITY_WINDOW_INVALID = -3,
    INFINITY_WINDOW_CAPACITY = -4,
    INFINITY_WINDOW_BAD_ADDRESS = -5,
    INFINITY_WINDOW_UNSUPPORTED = -6
};

// ------------------------=
// FUNC: infinity_window_create
// DESC: Creates a native normal-layer window over an owned surface, returning its positive ID or a negative error.
// ------------------=
static inline int64_t infinity_window_create(InfinityCapability create, InfinitySurfaceId surface,
    int32_t x, int32_t y, uint32_t width, uint32_t height) {
    return infinity_system_call(INFINITY_SYS_WINDOW_CREATE, create, surface,
        (uint64_t)(int64_t)x, (uint64_t)(int64_t)y, width, height);
}
// ------------------------=
// FUNC: infinity_window_close
// DESC: Closes only the window covered by the caller's management capability.
// ------------------=
static inline int64_t infinity_window_close(InfinityCapability manage, InfinityWindowId window) {
    return infinity_system_call(INFINITY_SYS_WINDOW_CLOSE, manage, window, 0, 0, 0, 0);
}
// ------------------------=
// FUNC: infinity_window_title
// DESC: Sets a validated UTF-8 title of at most 96 bytes without granting access to kernel text storage.
// ------------------=
static inline int64_t infinity_window_title(InfinityCapability manage, InfinityWindowId window,
    const char *title, size_t length) {
    return infinity_system_call(INFINITY_SYS_WINDOW_TITLE, manage, window, (uintptr_t)title, length, 0, 0);
}
// ------------------------=
// FUNC: infinity_window_move
// DESC: Requests a position constrained by the native desktop work area.
// ------------------=
static inline int64_t infinity_window_move(InfinityCapability manage, InfinityWindowId window, int32_t x, int32_t y) {
    return infinity_system_call(INFINITY_SYS_WINDOW_MOVE, manage, window,
        (uint64_t)(int64_t)x, (uint64_t)(int64_t)y, 0, 0);
}
// ------------------------=
// FUNC: infinity_window_resize
// DESC: Requests bounded geometry while preserving compositor-owned window policy.
// ------------------=
static inline int64_t infinity_window_resize(InfinityCapability manage, InfinityWindowId window, uint32_t width, uint32_t height) {
    return infinity_system_call(INFINITY_SYS_WINDOW_RESIZE, manage, window, width, height, 0, 0);
}
// ------------------------=
// FUNC: infinity_window_state
// DESC: Minimizes, maximizes, restores or enters fullscreen without direct framebuffer access.
// ------------------=
static inline int64_t infinity_window_state(InfinityCapability manage, InfinityWindowId window, enum InfinityWindowState state) {
    return infinity_system_call(INFINITY_SYS_WINDOW_STATE, manage, window, state, 0, 0, 0);
}
// ------------------------=
// FUNC: infinity_window_focus
// DESC: Requests focus using separately revocable focus authority, respecting modal and secure-input policy.
// ------------------=
static inline int64_t infinity_window_focus(InfinityCapability focus, InfinityWindowId window) {
    return infinity_system_call(INFINITY_SYS_WINDOW_FOCUS, focus, window, 0, 0, 0, 0);
}
// ------------------------=
// FUNC: infinity_window_capture
// DESC: Requests pointer capture for the exact owned window covered by the input capability.
// ------------------=
static inline int64_t infinity_window_capture(InfinityCapability input, InfinityWindowId window) {
    return infinity_system_call(INFINITY_SYS_WINDOW_CAPTURE, input, window, 0, 0, 0, 0);
}
// ------------------------=
// FUNC: infinity_window_release
// DESC: Releases only this window's pointer capture, never a sibling app or window's capture.
// ------------------=
static inline int64_t infinity_window_release(InfinityCapability input, InfinityWindowId window) {
    return infinity_system_call(INFINITY_SYS_WINDOW_RELEASE, input, window, 0, 0, 0, 0);
}
#endif
