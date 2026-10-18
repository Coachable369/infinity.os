#ifndef INFINITY_APP_API_H
#define INFINITY_APP_API_H
#include <stdint.h>
/* Include <infinity/app.h> to use these implemented SDK helpers. */
#define APP_CLICK 1
#define APP_CLOSE 2
#define APP_AI 3
typedef struct AppEvent {
    int kind, id;
    int64_t a, b, c, d;
    uint64_t token;
    char text[513];
} AppEvent;
static inline int app_window(const char *title, int width, int height);
static inline int app_button(int id, int x, int y, int w, int h, const char *label);
static inline int app_label(int id, int x, int y, int w, int h, const char *label);
static inline int app_display(int id, int x, int y, int w, int h, const char *label);
/* Grid rows/columns are bounded (1..9 / 1..12, at most 95 cells).
 * Coordinates are zero-based; positive cell IDs must remain stable. */
// ------------------------=
// FUNC: app_grid
// DESC: Opens a bounded native grid layout using existing themed controls.
// ------------------=
static inline int app_grid(const char *title, int rows, int columns);
// ------------------------=
// FUNC: app_cell
// DESC: Creates or updates one stable native button at zero-based grid coordinates.
// ------------------=
static inline int app_cell(int id, int row, int column, const char *label);
// ------------------------=
// FUNC: app_status
// DESC: Updates the reserved grid status label.
// ------------------=
static inline int app_status(const char *text);
// ------------------------=
// FUNC: app_offer
// DESC: Offers app.activate for an existing grid cell; the shared handler validates its operation.
// ------------------=
static inline int app_offer(int id, const char *label);
/* Publish state and legal choices after startup and every operation. */
static inline int app_state(int ai_turn, int terminal, const char *text);
static inline int app_choice(int id, const char *action, const char *label, int a, int b, int c, int d);
static inline int app_text_action(int id, const char *action, const char *label, const char *parameter, int minimum, int maximum, const char *text_name);
static inline int app_publish(void);
/* app_event returns 1 for input, 0 idle; kind distinguishes user and AI. */
static inline int app_event(AppEvent *event);
/* After an AI action, publish updated state before acknowledging applied=1. */
static inline int app_done(const AppEvent *event, int applied);
static inline void app_yield(void);
// ------------------------=
// FUNC: app_run
// DESC: Runs the native event loop; render publishes and returns its status, handle returns positive for accepted input or zero for rejection.
// ------------------=
static inline int app_run(int (*render)(void), int (*handle)(const AppEvent *));
#endif
