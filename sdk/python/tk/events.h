#ifndef INFINITY_TK_EVENTS_H
#define INFINITY_TK_EVENTS_H
#include <X11/Xlib.h>
#include <string.h>

// ------------------------=
// FUNC: InfinityTkWindowEvent
// DESC: Initializes both target and reporting window for real structure events so upstream Tk dispatches StructureNotify, not SubstructureNotify.
// ------------------=
static inline void InfinityTkWindowEvent(XEvent *event, Display *display, Window window,
        int type, unsigned long serial, int x, int y, unsigned width, unsigned height,
        unsigned border, Bool override_redirect) {
    memset(event, 0, sizeof(*event));
    event->xany.type = type; event->xany.display = display;
    event->xany.serial = serial; event->xany.window = window;
    if (type == ConfigureNotify) {
        event->xconfigure.event = window; event->xconfigure.window = window;
        event->xconfigure.x = x; event->xconfigure.y = y;
        event->xconfigure.width = width; event->xconfigure.height = height;
        event->xconfigure.border_width = border;
        event->xconfigure.override_redirect = override_redirect;
    } else if (type == MapNotify) {
        event->xmap.event = window; event->xmap.window = window;
        event->xmap.override_redirect = override_redirect;
    } else if (type == UnmapNotify) {
        event->xunmap.event = window; event->xunmap.window = window;
    } else if (type == DestroyNotify) {
        event->xdestroywindow.event = window; event->xdestroywindow.window = window;
    } else if (type == Expose) {
        event->xexpose.width = width; event->xexpose.height = height;
    }
}
#endif
