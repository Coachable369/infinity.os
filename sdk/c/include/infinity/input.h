#ifndef INFINITY_INPUT_H
#define INFINITY_INPUT_H
#include <stdint.h>
#include <stddef.h>

#define INFINITY_EVENT_NATIVE_INPUT 0x9800du
enum InfinityInputKind {
    INFINITY_INPUT_KEY=1, INFINITY_INPUT_TEXT=2, INFINITY_INPUT_POINTER=3,
    INFINITY_COMPOSITION_START=4, INFINITY_COMPOSITION_UPDATE=5,
    INFINITY_COMPOSITION_COMMIT=6, INFINITY_COMPOSITION_CANCEL=7
};
enum InfinityKey {
    INFINITY_KEY_TAB=1, INFINITY_KEY_ENTER=2, INFINITY_KEY_ESCAPE=3,
    INFINITY_KEY_SPACE=4, INFINITY_KEY_LEFT=5, INFINITY_KEY_RIGHT=6,
    INFINITY_KEY_UP=7, INFINITY_KEY_DOWN=8, INFINITY_KEY_HOME=9,
    INFINITY_KEY_END=10, INFINITY_KEY_BACKSPACE=11, INFINITY_KEY_DELETE=12,
    INFINITY_KEY_CHARACTER=13
};
enum InfinityPointerPhase {
    INFINITY_POINTER_MOVE=0, INFINITY_POINTER_DOWN=1, INFINITY_POINTER_UP=2,
    INFINITY_POINTER_SCROLL=3, INFINITY_POINTER_CANCEL=4
};
/* Copy the native input event payload into this structure (do not cast a
 * potentially unaligned byte buffer). All values use little-endian native ABI.
 * Key: flags bit0=pressed, bits8..15=normalized modifiers; a=key, b=Unicode.
 * Pointer: flags low8=phase, bits8..15=button; a/b=window-local x/y,
 * c/d=scroll deltas in the normalized device router's units.
 * Text/composition: c=UTF-8 byte count, max64; no terminating NUL required.
 * A sequence gap in the enclosing InfinityEvent requires input reconciliation.
 */
typedef struct InfinityInput {
    uint32_t version, kind, window, flags;
    int32_t a, b, c, d;
    uint8_t text[64];
} InfinityInput;
_Static_assert(sizeof(InfinityInput)==96, "native input packet size");
_Static_assert(offsetof(InfinityInput,text)==32, "native input text offset");
#endif
