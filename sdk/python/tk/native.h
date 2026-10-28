#ifndef INFINITY_TK_NATIVE_H
#define INFINITY_TK_NATIVE_H
#include "tkInt.h"
#include <infinity/font.h>

/* This is a native platform backend for upstream Tk, not a replacement widget
 * implementation. Drawables are private native application memory. */
typedef struct InfinityTkPixels {
    uint32_t *pixels;
    unsigned width, height;
    int depth;
} InfinityTkPixels;

InfinityTkPixels *InfinityTkDrawable(Drawable drawable);
Drawable InfinityTkAllocate(unsigned width, unsigned height, int depth);
int InfinityTkResize(Drawable drawable, unsigned width, unsigned height);
void InfinityTkRelease(Drawable drawable);
uint64_t InfinityTkRevision(Drawable drawable);
int InfinityTkSetTitle(Window window, const char *title);
int InfinityTkSetState(Window window, int state);
int InfinityTkGetState(Window window);
void InfinityTkReleaseProperties(Window window);
int XParseGeometry(const char *text, int *x, int *y, unsigned *width, unsigned *height);
int XSetTransientForHint(Display *display, Window window, Window parent);
void InfinityTkBlend(Drawable drawable, GC gc, int x, int y,
    uint32_t rgb, unsigned coverage);
void InfinityTkDamage(Drawable drawable);
#endif
