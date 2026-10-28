#include "../../sdk/python/tk/events.h"
#include <assert.h>

// ------------------------=
// FUNC: main
// DESC: Exercises the production event constructor's target identity and geometry; self-structure delivery must never become a substructure event.
// ------------------=
int main(void) {
    const int types[] = {ConfigureNotify, MapNotify, UnmapNotify, DestroyNotify, Expose};
    XEvent event;
    for (unsigned index = 0; index < sizeof(types) / sizeof(types[0]); ++index) {
        memset(&event, 0xff, sizeof(event));
        InfinityTkWindowEvent(&event, NULL, 42, types[index], 19, -120, 80, 520, 400, 2, True);
        assert(event.type == types[index] && event.xany.window == 42);
        assert(event.xany.serial == 19 && event.xany.send_event == False && event.xany.display == NULL);
        if (types[index] != Expose) {
            /* These common fields determine Tk's StructureNotify/SubstructureNotify routing. */
            assert(event.xmap.event == event.xmap.window && event.xmap.window == 42);
        }
        if (types[index] == ConfigureNotify) {
            assert(event.xconfigure.x == -120 && event.xconfigure.y == 80);
            assert(event.xconfigure.width == 520 && event.xconfigure.height == 400);
            assert(event.xconfigure.border_width == 2 && event.xconfigure.override_redirect == True);
        }
        if (types[index] == Expose) {
            assert(event.xexpose.x == 0 && event.xexpose.y == 0 && event.xexpose.count == 0);
            assert(event.xexpose.width == 520 && event.xexpose.height == 400);
        }
    }
    return 0;
}
