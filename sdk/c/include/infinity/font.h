#ifndef INFINITY_FONT_H
#define INFINITY_FONT_H
#include <stddef.h>
#include <infinity/syscall.h>

enum InfinityFontStyle { INFINITY_FONT_MONO = 1, INFINITY_FONT_BOLD = 2 };
typedef struct { uint32_t version, scalar, pixel_height, style_flags; } InfinityFontRequest;
typedef struct {
    int32_t version, width, height, bearing_x, bearing_y, advance_x;
    int32_t ascent, descent, line_spacing, bytes_written;
} InfinityFontMetrics;

// ------------------------=
// FUNC: infinity_font_glyph
// DESC: Reads an antialiased installed Inter or JetBrains Mono ASCII glyph into private memory; unsupported glyphs/styles return a negative error.
// ------------------=
static inline int64_t infinity_font_glyph(const InfinityFontRequest *request,
    InfinityFontMetrics *metrics, unsigned char *coverage, size_t capacity) {
    return infinity_system_call(INFINITY_SYS_FONT_GLYPH, (uintptr_t)request, sizeof(*request),
        (uintptr_t)metrics, sizeof(*metrics), (uintptr_t)coverage, capacity);
}
#endif
