#include "tclInt.h"
#include <infinity/syscall.h>

typedef struct NativeFileHandler {
    int descriptor, mask, queued;
    unsigned serial;
    Tcl_FileProc *proc;
    ClientData data;
    struct NativeFileHandler *next;
} NativeFileHandler;
typedef struct { Tcl_Event event; int descriptor, mask; unsigned serial; } NativeFileEvent;
static NativeFileHandler *handlers;
static unsigned handler_serial;
static int alerted;
static int (*native_event_pump)(void);

// ------------------------=
// FUNC: InfinityTclSetEventPump
// DESC: Attaches the native Tk input pump to Tcl's existing event service without a second scheduler.
// ------------------=
void InfinityTclSetEventPump(int (*pump)(void)) { native_event_pump = pump; }
// ------------------------=
// FUNC: file_event
// DESC: Dispatches only the current registered descriptor generation, discarding stale queued callbacks.
// ------------------=
static int file_event(Tcl_Event *event, int flags) {
    NativeFileEvent *ready = (NativeFileEvent *)event;
    if (!(flags & TCL_FILE_EVENTS)) return 0;
    for (NativeFileHandler *h = handlers; h; h = h->next) {
        if (h->descriptor == ready->descriptor && h->serial == ready->serial) {
            h->queued = 0;
            h->proc(h->data, ready->mask & h->mask);
            break;
        }
    }
    return 1;
}
// ------------------------=
// FUNC: Tcl_InitNotifier
// DESC: Returns this single-threaded interpreter image's notifier identity.
// ------------------=
ClientData Tcl_InitNotifier(void) { return &handlers; }
// ------------------------=
// FUNC: Tcl_FinalizeNotifier
// DESC: Releases owned registrations without taking ownership of application descriptors.
// ------------------=
void Tcl_FinalizeNotifier(ClientData data) {
    (void)data;
    while (handlers) { NativeFileHandler *next = handlers->next; ckfree(handlers); handlers = next; }
    native_event_pump = NULL;
    alerted = 0;
}
// ------------------------=
// FUNC: Tcl_AlertNotifier
// DESC: Records an alert for the next native scheduler turn.
// ------------------=
void Tcl_AlertNotifier(ClientData data) { (void)data; alerted = 1; }
// ------------------------=
// FUNC: Tcl_SetTimer
// DESC: Leaves timer deadlines with Tcl's timer queue, which supplies the bounded wait argument.
// ------------------=
void Tcl_SetTimer(const Tcl_Time *time) { (void)time; }
// ------------------------=
// FUNC: Tcl_ServiceModeHook
// DESC: Uses Tcl's own service mode; no independent platform event thread exists.
// ------------------=
void Tcl_ServiceModeHook(int mode) { (void)mode; }
// ------------------------=
// FUNC: Tcl_DeleteFileHandler
// DESC: Invalidates a descriptor callback so already queued events cannot call its former owner.
// ------------------=
void Tcl_DeleteFileHandler(int descriptor) {
    NativeFileHandler **slot = &handlers;
    while (*slot) {
        if ((*slot)->descriptor == descriptor) {
            NativeFileHandler *old = *slot; *slot = old->next; ckfree(old); return;
        }
        slot = &(*slot)->next;
    }
}
// ------------------------=
// FUNC: Tcl_CreateFileHandler
// DESC: Registers capability-mounted regular-file readiness; unsupported device readiness is never fabricated.
// ------------------=
void Tcl_CreateFileHandler(int descriptor, int mask, Tcl_FileProc *proc, ClientData data) {
    Tcl_DeleteFileHandler(descriptor);
    if (!proc || !mask) return;
    NativeFileHandler *h = ckalloc(sizeof(*h));
    h->descriptor = descriptor; h->mask = mask; h->queued = 0;
    h->serial = ++handler_serial; h->proc = proc; h->data = data;
    h->next = handlers; handlers = h;
}
// ------------------------=
// FUNC: TclUnixWaitForFile
// DESC: Reports immediate regular-file readiness only; device and socket polling require an explicit native provider.
// ------------------=
int TclUnixWaitForFile(int descriptor, int mask, int timeout) {
    (void)timeout;
    struct stat info;
    if (fstat(descriptor, &info) < 0) return 0;
    if (S_ISREG(info.st_mode)) return mask & (TCL_READABLE | TCL_WRITABLE);
    errno = ENOTSUP; return 0;
}
// ------------------------=
// FUNC: Tcl_Sleep
// DESC: Sleeps through the native execution context's interruptible timer rather than a host select loop.
// ------------------=
void Tcl_Sleep(int milliseconds) {
    if (milliseconds <= 0) return;
    struct timespec request = { milliseconds / 1000, (milliseconds % 1000) * 1000000L };
    while (nanosleep(&request, &request) < 0 && errno == EINTR) { }
}
// ------------------------=
// FUNC: Tcl_WaitForEvent
// DESC: Pumps authorized native input, queues observed regular-file events and bounds scheduler waits to ten milliseconds.
// ------------------=
int Tcl_WaitForEvent(const Tcl_Time *timeout) {
    int ready = native_event_pump ? native_event_pump() : 0;
    for (NativeFileHandler *h = handlers; h; h = h->next) {
        if (h->queued) continue;
        int mask = TclUnixWaitForFile(h->descriptor, h->mask, 0);
        if (!mask) continue;
        NativeFileEvent *e = ckalloc(sizeof(*e));
        e->event.proc = file_event; e->descriptor = h->descriptor;
        e->mask = mask; e->serial = h->serial; h->queued = 1;
        Tcl_QueueEvent(&e->event, TCL_QUEUE_TAIL); ready = 1;
    }
    if (ready || alerted) { alerted = 0; return 0; }
    if (!timeout && !native_event_pump && !handlers) return -1;
    long usec = 10000;
    if (timeout && timeout->sec == 0 && timeout->usec < usec) usec = timeout->usec;
    if (timeout && timeout->sec < 0) usec = 0;
    if (usec > 0) {
        struct timespec request = { 0, usec * 1000 };
        if (nanosleep(&request, NULL) < 0 && errno != EINTR) return -1;
    } else {
        infinity_system_call(INFINITY_SYS_YIELD, 0, 0, 0, 0, 0, 0);
    }
    return 0;
}
