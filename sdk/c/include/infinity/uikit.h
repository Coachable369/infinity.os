#ifndef INFINITY_UIKIT_H
#define INFINITY_UIKIT_H
#include <stddef.h>
#include <stdint.h>
#include <infinity/syscall.h>

#define INFINITY_UI_LABEL 1u
#define INFINITY_UI_DISPLAY 2u
#define INFINITY_UI_BUTTON 3u
#define INFINITY_UI_EVENT_ACTIVATE 1u
#define INFINITY_UI_EVENT_CLOSE 2u

typedef struct InfinityUiWindow {
    uint32_t version, width, height, title_length;
    char title[48];
} InfinityUiWindow;

typedef struct InfinityUiElement {
    uint32_t version, id, kind, x, y, width, height, label_length;
    char label[32];
} InfinityUiElement;

typedef struct InfinityUiEvent {
    uint32_t version, kind, id;
    int32_t value;
} InfinityUiEvent;

// ------------------------=
// FUNC: infinity_ui_open
// DESC: Opens one theme-aware native window from a bounded declarative description.
// ------------------=
static inline int64_t infinity_ui_open(const InfinityUiWindow *window) {
    return infinity_system_call(INFINITY_SYS_UI_OPEN, (uintptr_t)window, sizeof(*window), 0, 0, 0, 0);
}

// ------------------------=
// FUNC: infinity_ui_upsert
// DESC: Adds or updates a label, display, or button by stable semantic ID.
// ------------------=
static inline int64_t infinity_ui_upsert(const InfinityUiElement *element) {
    return infinity_system_call(INFINITY_SYS_UI_UPSERT, (uintptr_t)element, sizeof(*element), 0, 0, 0, 0);
}

// ------------------------=
// FUNC: infinity_ui_poll
// DESC: Returns one activation or close event, zero while idle, or a negative ABI error.
// ------------------=
static inline int64_t infinity_ui_poll(InfinityUiEvent *event) {
    return infinity_system_call(INFINITY_SYS_UI_POLL, (uintptr_t)event, sizeof(*event), 0, 0, 0, 0);
}

// ------------------------=
// FUNC: infinity_ui_yield
// DESC: Yields one native process slice while waiting for the next UI event.
// ------------------=
static inline void infinity_ui_yield(void) {
    (void)infinity_system_call(INFINITY_SYS_YIELD, 0, 0, 0, 0, 0, 0);
}

#endif
