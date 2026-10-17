#include <stdio.h>
#include <string.h>
#include <infinity/uikit.h>

// ------------------------=
// FUNC: put_element
// DESC: Publishes one bounded semantic UI Kit element.
// ------------------=
static int put_element(uint32_t id, uint32_t kind, uint32_t x, uint32_t y,
    uint32_t width, uint32_t height, const char *label) {
    InfinityUiElement element = {0};
    size_t length = strlen(label);
    if (length > sizeof(element.label)) length = sizeof(element.label);
    element.version = 1; element.id = id; element.kind = kind;
    element.x = x; element.y = y; element.width = width; element.height = height;
    element.label_length = (uint32_t)length;
    memcpy(element.label, label, length);
    return infinity_ui_upsert(&element) == 0;
}

// ------------------------=
// FUNC: show_value
// DESC: Updates the calculator display from its current integer value.
// ------------------=
static int show_value(long value) {
    char text[32];
    snprintf(text, sizeof(text), "%ld", value);
    return put_element(1, INFINITY_UI_DISPLAY, 20, 24, 440, 72, text);
}

// ------------------------=
// FUNC: calculate
// DESC: Applies one validated integer operation and reports division by zero.
// ------------------=
static long calculate(long left, long right, char operation, int *valid) {
    *valid = 1;
    if (operation == '+') return left + right;
    if (operation == '-') return left - right;
    if (operation == '*') return left * right;
    if (operation == '/' && right != 0) return left / right;
    *valid = 0;
    return 0;
}

// ------------------------=
// FUNC: main
// DESC: Runs a native theme-aware four-function calculator until its window closes.
// ------------------=
int main(void) {
    InfinityUiWindow window = {0};
    const char *title = "Calculator";
    const char *labels[16] = {"7","8","9","/", "4","5","6","*",
        "1","2","3","-", "C","0","=","+"};
    const uint32_t ids[16] = {107,108,109,203,104,105,106,202,
        101,102,103,201,205,100,204,200};
    long value = 0, stored = 0;
    char operation = 0;
    int entering = 0;
    window.version = 1; window.width = 480; window.height = 420;
    window.title_length = (uint32_t)strlen(title);
    memcpy(window.title, title, window.title_length);
    if (infinity_ui_open(&window) != 0 || !show_value(0)) return 1;
    for (uint32_t index = 0; index < 16; ++index) {
        uint32_t column = index % 4, row = index / 4;
        if (!put_element(ids[index], INFINITY_UI_BUTTON, 20 + column * 112,
            116 + row * 70, 100, 56, labels[index])) return 2;
    }
    for (;;) {
        InfinityUiEvent event = {0};
        int64_t ready = infinity_ui_poll(&event);
        if (ready < 0) return 3;
        if (ready == 0) { infinity_ui_yield(); continue; }
        if (event.kind == INFINITY_UI_EVENT_CLOSE) break;
        if (event.kind != INFINITY_UI_EVENT_ACTIVATE) continue;
        if (event.id >= 100 && event.id <= 109) {
            int digit = (int)(event.id - 100);
            value = entering ? value * 10 + digit : digit;
            entering = 1;
        } else if (event.id == 205) {
            value = 0; stored = 0; operation = 0; entering = 0;
        } else if (event.id == 204 && operation) {
            int valid = 0;
            value = calculate(stored, value, operation, &valid);
            if (!valid) {
                put_element(1, INFINITY_UI_DISPLAY, 20, 24, 440, 72, "Cannot divide by zero");
                value = 0;
            } else show_value(value);
            operation = 0; entering = 0;
            continue;
        } else if (event.id >= 200 && event.id <= 203) {
            stored = value;
            operation = event.id == 200 ? '+' : event.id == 201 ? '-' : event.id == 202 ? '*' : '/';
            entering = 0;
        }
        show_value(value);
    }
    return 0;
}
