#ifndef INFINITY_TK_PORT_H
#define INFINITY_TK_PORT_H
#define __UNIX__ 1
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <ctype.h>
#include <limits.h>
#include <math.h>
#include <assert.h>
#include <errno.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <unistd.h>
#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/keysym.h>
#include <X11/Xatom.h>
#include <X11/cursorfont.h>
#define NBBY CHAR_BIT
#define TkpGetPixel(color) ((((unsigned long)(color)->red >> 8) << 16) | (((unsigned long)(color)->green >> 8) << 8) | ((unsigned long)(color)->blue >> 8))
Status XAllocNamedColor(Display *,Colormap,const char *,XColor *,XColor *);
int XSetTransientForHint(Display *,Window,Window);
int XClearArea(Display *,Window,int,int,unsigned,unsigned,Bool);
#define TkPutImage(colors,ncolors,display,pixels,gc,image,sx,sy,dx,dy,w,h) \
    XPutImage(display,pixels,gc,image,sx,sy,dx,dy,w,h)
#define TkpButtonSetDefaults() {}
#define TkpDestroyButton(button) {}
#define TkSelUpdateClipboard(a,b) {}
#define TkSetPixmapColormap(p,c) {}
#define TkpDefineNativeBitmaps()
#define TkpCreateNativeBitmap(display,source) None
#define TkpGetNativeAppBitmap(display,name,w,h) None
#define TkpPrintWindowId(buffer,window) snprintf(buffer,TCL_INTEGER_SPACE,"0x%08lx",(unsigned long)(window))
#endif
