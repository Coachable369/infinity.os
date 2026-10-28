#ifndef INFINITY_GUI_H
#define INFINITY_GUI_H
#include <infinity/syscall.h>

enum InfinityGuiGrant {
    INFINITY_GUI_SURFACE_CREATE = 0,
    INFINITY_GUI_WINDOW_CREATE = 1,
    INFINITY_GUI_SURFACE_PUBLISH = 2,
    INFINITY_GUI_SURFACE_RESIZE = 3,
    INFINITY_GUI_SURFACE_DESTROY = 4,
    INFINITY_GUI_WINDOW_MANAGE = 5,
    INFINITY_GUI_WINDOW_FOCUS = 6,
    INFINITY_GUI_WINDOW_CAPTURE = 7,
    INFINITY_GUI_INPUT_EVENTS = 8,
    INFINITY_GUI_RESIZE_EVENTS = 9,
    INFINITY_GUI_CLOSE_EVENTS = 10,
    INFINITY_GUI_CLOSE_REQUEST_EVENTS = 11
};
// ------------------------=
// FUNC: infinity_gui_request
// DESC: Requests a nondelegable grant for the current application's exact native resource; only an authenticated GUI host may issue it.
// ------------------=
static inline int64_t infinity_gui_request(enum InfinityGuiGrant category, uint32_t resource) {
    return infinity_system_call(0x1251, category, resource, 0, 0, 0, 0);
}
// ------------------------=
// FUNC: infinity_gui_geometry
// DESC: Reads actual authenticated display/work-area geometry (display x/y/width/height then work x/y/width/height); no framebuffer address is exposed.
// ------------------=
static inline int64_t infinity_gui_geometry(unsigned component) {
    return infinity_system_call(0x1252, component, 0, 0, 0, 0, 0);
}
// ------------------------=
// FUNC: infinity_gui_window_geometry
// DESC: Reads committed geometry/state only for the caller's exact native window; no other application metadata is exposed.
// ------------------=
static inline int64_t infinity_gui_window_geometry(uint32_t window, unsigned component) {
    return infinity_system_call(0x1253, window, component, 0, 0, 0, 0);
}
// ------------------------=
// FUNC: infinity_gui_cursor
// DESC: Selects an installed antialiased OS pointer only while trusted input is over this exact application's window; unsupported shapes fail explicitly.
// ------------------=
static inline int64_t infinity_gui_cursor(uint32_t window, unsigned shape) {
    return infinity_system_call(0x1254, window, shape, 0, 0, 0, 0);
}
#endif
