#include "native.h"
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct NativeProtocol {
    Atom atom;
    Tcl_Obj *command;
    struct NativeProtocol *next;
} NativeProtocol;

typedef struct TkWmInfo {
    TkWindow *window;
    struct TkWmInfo *nextPtr;
    Tcl_Obj *title;
    NativeProtocol *protocols;
    int state, width, height, min_width, min_height, max_width, max_height;
    int resize_width, resize_height, grid_width, grid_height, width_step, height_step;
    Tk_Window grid_window, menu;
} WmInfo;

// ------------------------=
// FUNC: wm_error
// DESC: Reports unsupported or denied native operations without manufacturing a window-manager success.
// ------------------=
static int wm_error(Tcl_Interp *interp, const char *message) {
    Tcl_SetObjResult(interp, Tcl_NewStringObj(message, -1));
    Tcl_SetErrorCode(interp, "TK", "WM", "INFINITY", NULL);
    return TCL_ERROR;
}

// ------------------------=
// FUNC: wm_limit
// DESC: Bounds a real geometry request to the declared minimum and maximum.
// ------------------=
static int wm_limit(int value, int minimum, int maximum) {
    if (value < minimum) value = minimum;
    return value > maximum ? maximum : value;
}

// ------------------------=
// FUNC: geometry_number
// DESC: Parses one bounded decimal geometry component without overflow or accepting missing digits.
// ------------------=
static int geometry_number(const char **cursor, unsigned *value) {
    const char *text = *cursor;
    unsigned number = 0;
    if (*text < '0' || *text > '9') return 0;
    while (*text >= '0' && *text <= '9') {
        unsigned digit = (unsigned)(*text++ - '0');
        if (number > ((unsigned)INT_MAX - digit) / 10) return 0;
        number = number * 10 + digit;
    }
    *cursor = text; *value = number; return 1;
}

// ------------------------=
// FUNC: XParseGeometry
// DESC: Parses the standard optional size and signed offsets atomically, retaining negative-zero edge anchoring flags.
// ------------------=
int XParseGeometry(const char *text, int *out_x, int *out_y, unsigned *out_width, unsigned *out_height) {
    if (!text || !*text) return 0;
    if (*text == '=') ++text;
    int mask = 0, x = 0, y = 0;
    unsigned width = 0, height = 0, value;
    if (*text >= '0' && *text <= '9') {
        if (!geometry_number(&text, &width)) return 0;
        mask |= WidthValue;
    }
    if (*text == 'x' || *text == 'X') {
        ++text;
        if (!geometry_number(&text, &height)) return 0;
        mask |= HeightValue;
    }
    if (*text == '+' || *text == '-') {
        int negative = *text++ == '-';
        if (!geometry_number(&text, &value)) return 0;
        x = negative ? -(int)value : (int)value;
        mask |= XValue | (negative ? XNegative : 0);
        if (*text == '+' || *text == '-') {
            negative = *text++ == '-';
            if (!geometry_number(&text, &value)) return 0;
            y = negative ? -(int)value : (int)value;
            mask |= YValue | (negative ? YNegative : 0);
        }
    }
    if (*text || !mask) return 0;
    if (mask & WidthValue) *out_width = width;
    if (mask & HeightValue) *out_height = height;
    if (mask & XValue) *out_x = x;
    if (mask & YValue) *out_y = y;
    return mask;
}

// ------------------------=
// FUNC: wm_geometry
// DESC: Sends the geometry manager's current request to the native toplevel rather than a host display server.
// ------------------=
static void wm_geometry(ClientData client, Tk_Window tkwin) {
    (void)client;
    TkWindow *window = (TkWindow *)tkwin;
    WmInfo *wm = window->wmInfoPtr;
    if (!wm || window->flags & TK_ALREADY_DEAD) return;
    int width = wm_limit(wm->width > 0 ? wm->width : window->reqWidth, wm->min_width, wm->max_width);
    int height = wm_limit(wm->height > 0 ? wm->height : window->reqHeight, wm->min_height, wm->max_height);
    if (window->window == None) {
        window->changes.width = width; window->changes.height = height;
        window->dirtyChanges |= CWWidth | CWHeight;
    } else {
        XMoveResizeWindow(window->display, window->window,
            window->changes.x, window->changes.y, width, height);
    }
    if (wm->menu) {
        Tk_MoveResizeWindow(wm->menu, 0, 0, width, Tk_ReqHeight(wm->menu));
        if (window->flags & TK_MAPPED) Tk_MapWindow(wm->menu);
    }
}

static const Tk_GeomMgr wm_manager = {"infinity", wm_geometry, NULL};

// ------------------------=
// FUNC: wm_event
// DESC: Applies declared resize bounds to actual configure events, including resizes initiated by the desktop.
// ------------------=
static void wm_event(ClientData client, XEvent *event) {
    TkWindow *window = client;
    WmInfo *wm = window->wmInfoPtr;
    if (!wm || event->type != ConfigureNotify || window->flags & TK_ALREADY_DEAD) return;
    int width = wm_limit(event->xconfigure.width, wm->min_width, wm->max_width);
    int height = wm_limit(event->xconfigure.height, wm->min_height, wm->max_height);
    if (wm->grid_window) {
        int base_width = window->reqWidth - wm->grid_width * wm->width_step;
        int base_height = window->reqHeight - wm->grid_height * wm->height_step;
        width = wm_limit(base_width + ((width - base_width) / wm->width_step) * wm->width_step,
            wm->min_width, wm->max_width);
        height = wm_limit(base_height + ((height - base_height) / wm->height_step) * wm->height_step,
            wm->min_height, wm->max_height);
    }
    if (!wm->resize_width && wm->width > 0) width = wm->width;
    if (!wm->resize_height && wm->height > 0) height = wm->height;
    if (width != event->xconfigure.width || height != event->xconfigure.height)
        XResizeWindow(window->display, window->window, width, height);
}

// ------------------------=
// FUNC: TkWmNewWindow
// DESC: Creates one genuine window-manager record and attaches Tk's layout requests to the native window.
// ------------------=
void TkWmNewWindow(TkWindow *window) {
    WmInfo *wm = (WmInfo *)ckalloc(sizeof(*wm));
    memset(wm, 0, sizeof(*wm));
    wm->window = window; wm->state = NormalState;
    wm->width = wm->height = -1;
    wm->min_width = wm->min_height = 1;
    wm->max_width = DisplayWidth(window->display, window->screenNum);
    wm->max_height = DisplayHeight(window->display, window->screenNum);
    wm->resize_width = wm->resize_height = 1;
    wm->width_step = wm->height_step = 1;
    wm->title = Tcl_NewStringObj(window->nameUid ? window->nameUid : "Tk", -1);
    Tcl_IncrRefCount(wm->title);
    wm->nextPtr = window->dispPtr->firstWmPtr;
    window->dispPtr->firstWmPtr = wm; window->wmInfoPtr = wm;
    Tk_ManageGeometry((Tk_Window)window, &wm_manager, NULL);
    Tk_CreateEventHandler((Tk_Window)window, StructureNotifyMask, wm_event, window);
}

// ------------------------=
// FUNC: wm_free
// DESC: Frees only the owned protocol scripts and metadata after the window has detached.
// ------------------=
static void wm_free(WmInfo *wm) {
    while (wm->protocols) {
        NativeProtocol *entry = wm->protocols;
        wm->protocols = entry->next;
        Tcl_DecrRefCount(entry->command); ckfree(entry);
    }
    Tcl_DecrRefCount(wm->title); ckfree(wm);
}

// ------------------------=
// FUNC: TkWmDeadWindow
// DESC: Detaches callbacks and registry state before the drawable is destroyed by Tk.
// ------------------=
void TkWmDeadWindow(TkWindow *window) {
    WmInfo *wm = window->wmInfoPtr;
    if (!wm) return;
    WmInfo **entry = &window->dispPtr->firstWmPtr;
    while (*entry && *entry != wm) entry = &(*entry)->nextPtr;
    if (*entry) *entry = wm->nextPtr;
    Tk_DeleteEventHandler((Tk_Window)window, StructureNotifyMask, wm_event, window);
    window->wmInfoPtr = NULL; wm_free(wm);
}

// ------------------------=
// FUNC: TkWmCleanup
// DESC: Releases display-owned metadata after Tk has stopped dispatching its window events.
// ------------------=
void TkWmCleanup(TkDisplay *display) {
    while (display->firstWmPtr) {
        WmInfo *wm = display->firstWmPtr;
        display->firstWmPtr = wm->nextPtr; wm_free(wm);
    }
}

// ------------------------=
// FUNC: TkWmMapWindow
// DESC: Publishes an actual native top-level window with the requested geometry, title and state.
// ------------------=
void TkWmMapWindow(TkWindow *window) {
    WmInfo *wm = window->wmInfoPtr;
    if (!wm || wm->state == WithdrawnState || window->flags & TK_ALREADY_DEAD) return;
    wm_geometry(NULL, (Tk_Window)window);
    Tk_MakeWindowExist((Tk_Window)window);
    InfinityTkSetTitle(window->window, Tcl_GetString(wm->title));
    window->flags |= TK_MAPPED;
    XMapWindow(window->display, window->window);
    if (wm->state == IconicState) InfinityTkSetState(window->window, 1);
}

// ------------------------=
// FUNC: TkWmUnmapWindow
// DESC: Withdraws actual native presentation while preserving the Tk window for remapping.
// ------------------=
void TkWmUnmapWindow(TkWindow *window) {
    window->flags &= ~TK_MAPPED;
    if (window->window != None) XUnmapWindow(window->display, window->window);
}

// ------------------------=
// FUNC: TkpWmSetState
// DESC: Changes the native window state only when the platform accepts the operation.
// ------------------=
int TkpWmSetState(TkWindow *window, int state) {
    WmInfo *wm = window->wmInfoPtr;
    if (!wm || (state != NormalState && state != IconicState && state != WithdrawnState)) return 0;
    if (state == WithdrawnState) { TkWmUnmapWindow(window); wm->state = state; return 1; }
    Tk_MakeWindowExist((Tk_Window)window);
    if (!InfinityTkSetState(window->window, state == IconicState ? 1 : 0)) return 0;
    wm->state = state;
    if (state == NormalState) TkWmMapWindow(window);
    return 1;
}

// ------------------------=
// FUNC: TkWmSetClass
// DESC: Maintains the Tk class identity locally; InfinityOS has no X11 resource-class server.
// ------------------=
void TkWmSetClass(TkWindow *window) { (void)window; }

// ------------------------=
// FUNC: TkWmAddToColormapWindows
// DESC: Records participation in the native true-color display, which has no mutable shared colormap.
// ------------------=
void TkWmAddToColormapWindows(TkWindow *window) { window->flags |= TK_WM_COLORMAP_WINDOW; }

// ------------------------=
// FUNC: TkWmRemoveFromColormapWindows
// DESC: Removes the corresponding true-color window bookkeeping.
// ------------------=
void TkWmRemoveFromColormapWindows(TkWindow *window) { window->flags &= ~TK_WM_COLORMAP_WINDOW; }

// ------------------------=
// FUNC: TkpGetWrapperWindow
// DESC: Returns the real top-level because native chrome needs no separate X11 wrapper window.
// ------------------=
TkWindow *TkpGetWrapperWindow(TkWindow *window) { return window; }

// ------------------------=
// FUNC: TkWmFocusToplevel
// DESC: Resolves the actual containing top-level for native focus events.
// ------------------=
TkWindow *TkWmFocusToplevel(TkWindow *window) {
    while (window && !(window->flags & TK_TOP_HIERARCHY)) window = window->parentPtr;
    return window;
}

// ------------------------=
// FUNC: TkWmRestackToplevel
// DESC: Routes layer changes to the native drawable/window backend.
// ------------------=
void TkWmRestackToplevel(TkWindow *window, int above, TkWindow *other) {
    Tk_MakeWindowExist((Tk_Window)window);
    if (other) {
        Tk_MakeWindowExist((Tk_Window)other);
        XWindowChanges changes = {0}; changes.sibling = other->window; changes.stack_mode = above;
        XConfigureWindow(window->display, window->window, CWSibling | CWStackMode, &changes);
    } else if (above == Above) XRaiseWindow(window->display, window->window);
    else XLowerWindow(window->display, window->window);
}

// ------------------------=
// FUNC: TkWmProtocolEventProc
// DESC: Executes a registered close callback safely or destroys the actual window when no handler exists.
// ------------------=
void TkWmProtocolEventProc(TkWindow *window, XEvent *event) {
    WmInfo *wm = window->wmInfoPtr;
    if (!wm) return;
    Atom atom = (Atom)event->xclient.data.l[0];
    for (NativeProtocol *entry = wm->protocols; entry; entry = entry->next) {
        if (entry->atom != atom) continue;
        Tcl_Interp *interp = window->mainPtr->interp;
        Tcl_Obj *command = entry->command;
        Tcl_IncrRefCount(command); Tcl_Preserve(interp);
        int result = Tcl_EvalObjEx(interp, command, TCL_EVAL_GLOBAL);
        if (result != TCL_OK && !Tcl_InterpDeleted(interp)) Tcl_BackgroundException(interp, result);
        Tcl_DecrRefCount(command); Tcl_Release(interp); return;
    }
    if (atom == Tk_InternAtom((Tk_Window)window, "WM_DELETE_WINDOW")) Tk_DestroyWindow((Tk_Window)window);
}

// ------------------------=
// FUNC: wm_protocol
// DESC: Stores and retrieves actual protocol callbacks with owned Tcl references and deletion semantics.
// ------------------=
static int wm_protocol(Tcl_Interp *interp, TkWindow *window, int objc, Tcl_Obj *const objv[]) {
    WmInfo *wm = window->wmInfoPtr;
    if (objc == 3) {
        Tcl_Obj *list = Tcl_NewListObj(0, NULL);
        for (NativeProtocol *p = wm->protocols; p; p = p->next)
            Tcl_ListObjAppendElement(interp, list, Tcl_NewStringObj(Tk_GetAtomName((Tk_Window)window, p->atom), -1));
        Tcl_SetObjResult(interp, list); return TCL_OK;
    }
    if (objc != 4 && objc != 5) { Tcl_WrongNumArgs(interp, 3, objv, "?name? ?command?"); return TCL_ERROR; }
    Atom atom = Tk_InternAtom((Tk_Window)window, Tcl_GetString(objv[3]));
    NativeProtocol **entry = &wm->protocols;
    while (*entry && (*entry)->atom != atom) entry = &(*entry)->next;
    if (objc == 4) { if (*entry) Tcl_SetObjResult(interp, (*entry)->command); return TCL_OK; }
    if (!*Tcl_GetString(objv[4])) {
        if (*entry) { NativeProtocol *old = *entry; *entry = old->next; Tcl_DecrRefCount(old->command); ckfree(old); }
        return TCL_OK;
    }
    if (!*entry) { *entry = (NativeProtocol *)ckalloc(sizeof(**entry)); (*entry)->atom = atom; (*entry)->next = NULL; (*entry)->command = NULL; }
    Tcl_IncrRefCount(objv[4]);
    if ((*entry)->command) Tcl_DecrRefCount((*entry)->command);
    (*entry)->command = objv[4]; return TCL_OK;
}

// ------------------------=
// FUNC: Tk_WmObjCmd
// DESC: Implements real Tk window controls and rejects unsupported desktop protocols with a useful Tcl error.
// ------------------=
int Tk_WmObjCmd(ClientData client, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
    if (objc < 3) { Tcl_WrongNumArgs(interp, 1, objv, "option window ?arg ...?"); return TCL_ERROR; }
    TkWindow *window = (TkWindow *)Tk_NameToWindow(interp, Tcl_GetString(objv[2]), (Tk_Window)client);
    if (!window) return TCL_ERROR;
    WmInfo *wm = window->wmInfoPtr;
    if (!wm) return wm_error(interp, "window is not a native top-level");
    const char *option = Tcl_GetString(objv[1]);
    if (!strcmp(option, "protocol")) return wm_protocol(interp, window, objc, objv);
    if (!strcmp(option, "title")) {
        if (objc == 3) { Tcl_SetObjResult(interp, wm->title); return TCL_OK; }
        if (objc != 4) goto arguments;
        if (window->window != None && !InfinityTkSetTitle(window->window, Tcl_GetString(objv[3])))
            return wm_error(interp, "native window title update was denied");
        Tcl_IncrRefCount(objv[3]); Tcl_DecrRefCount(wm->title); wm->title = objv[3]; return TCL_OK;
    }
    if (!strcmp(option, "geometry")) {
        if (objc == 3) {
            int width = window->changes.width, height = window->changes.height;
            if (wm->grid_window) { width = wm->grid_width + (width - window->reqWidth) / wm->width_step;
                height = wm->grid_height + (height - window->reqHeight) / wm->height_step; }
            Tcl_SetObjResult(interp, Tcl_ObjPrintf("%dx%d%+d%+d", width,
                height, window->changes.x, window->changes.y)); return TCL_OK;
        }
        if (objc != 4) goto arguments;
        const char *geometry = Tcl_GetString(objv[3]);
        if (!*geometry) { wm->width = wm->height = -1; wm_geometry(NULL, (Tk_Window)window); return TCL_OK; }
        int x = window->changes.x, y = window->changes.y; unsigned width = 0, height = 0;
        int mask = XParseGeometry(geometry, &x, &y, &width, &height);
        if (!mask || (mask & WidthValue && (!width || width > INT_MAX))
            || (mask & HeightValue && (!height || height > INT_MAX))) return wm_error(interp, "invalid native window geometry");
        if (wm->grid_window) {
            int64_t w = (int64_t)window->reqWidth + ((int64_t)width - wm->grid_width) * wm->width_step;
            int64_t h = (int64_t)window->reqHeight + ((int64_t)height - wm->grid_height) * wm->height_step;
            if ((mask & WidthValue && (w < 1 || w > INT_MAX)) || (mask & HeightValue && (h < 1 || h > INT_MAX)))
                return wm_error(interp, "grid geometry exceeds native window limits");
            if (mask & WidthValue) width = (unsigned)w;
            if (mask & HeightValue) height = (unsigned)h;
        }
        if (mask & WidthValue) wm->width = width;
        if (mask & HeightValue) wm->height = height;
        if (mask & XNegative) x += DisplayWidth(window->display, window->screenNum) - (wm->width > 0 ? wm->width : window->reqWidth);
        if (mask & YNegative) y += DisplayHeight(window->display, window->screenNum) - (wm->height > 0 ? wm->height : window->reqHeight);
        window->changes.x = x; window->changes.y = y;
        wm_geometry(NULL, (Tk_Window)window); return TCL_OK;
    }
    if (!strcmp(option, "minsize") || !strcmp(option, "maxsize") || !strcmp(option, "resizable")) {
        int resize = !strcmp(option, "resizable"), minimum = !strcmp(option, "minsize");
        int *width = resize ? &wm->resize_width : minimum ? &wm->min_width : &wm->max_width;
        int *height = resize ? &wm->resize_height : minimum ? &wm->min_height : &wm->max_height;
        if (objc == 3) { Tcl_Obj *values[2] = {Tcl_NewIntObj(*width), Tcl_NewIntObj(*height)};
            Tcl_SetObjResult(interp, Tcl_NewListObj(2, values)); return TCL_OK; }
        if (objc != 5) goto arguments;
        int w, h;
        if (resize ? (Tcl_GetBooleanFromObj(interp, objv[3], &w) || Tcl_GetBooleanFromObj(interp, objv[4], &h))
            : (Tcl_GetIntFromObj(interp, objv[3], &w) || Tcl_GetIntFromObj(interp, objv[4], &h))) return TCL_ERROR;
        if (!resize && (w < 1 || h < 1 || w > 8192 || h > 8192)) return wm_error(interp, "window size must be between 1 and 8192");
        if (!resize && (minimum ? w > wm->max_width || h > wm->max_height : w < wm->min_width || h < wm->min_height))
            return wm_error(interp, "minimum size must not exceed maximum size");
        *width = w; *height = h;
        if (resize) { if (!w) wm->width = window->changes.width; if (!h) wm->height = window->changes.height; }
        wm_geometry(NULL, (Tk_Window)window); return TCL_OK;
    }
    if (!strcmp(option, "state") || !strcmp(option, "withdraw") || !strcmp(option, "deiconify") || !strcmp(option, "iconify")) {
        if (!strcmp(option, "state") && objc == 3) {
            int native = InfinityTkGetState(window->window);
            const char *value = native == 4 || (native < 0 && wm->state == WithdrawnState) ? "withdrawn"
                : native == 1 || (native < 0 && wm->state == IconicState) ? "iconic" : native == 2 ? "zoomed" : "normal";
            Tcl_SetObjResult(interp, Tcl_NewStringObj(value, -1)); return TCL_OK;
        }
        int state;
        if (!strcmp(option, "state")) {
            if (objc != 4) goto arguments;
            const char *value = Tcl_GetString(objv[3]);
            state = !strcmp(value, "normal") ? NormalState : !strcmp(value, "iconic") ? IconicState : !strcmp(value, "withdrawn") ? WithdrawnState : -1;
        } else { if (objc != 3) goto arguments; state = !strcmp(option, "withdraw") ? WithdrawnState : !strcmp(option, "iconify") ? IconicState : NormalState; }
        return TkpWmSetState(window, state) ? TCL_OK : wm_error(interp, "native window state is unsupported or denied");
    }
    if (!strcmp(option, "frame") && objc == 3) {
        Tk_MakeWindowExist((Tk_Window)window);
        Tcl_SetObjResult(interp, Tcl_ObjPrintf("0x%lx", (unsigned long)window->window)); return TCL_OK;
    }
    if (!strcmp(option, "attributes") && objc == 5 && !strcmp(Tcl_GetString(objv[3]), "-fullscreen")) {
        int value;
        if (Tcl_GetBooleanFromObj(interp, objv[4], &value)) return TCL_ERROR;
        Tk_MakeWindowExist((Tk_Window)window);
        return InfinityTkSetState(window->window, value ? 3 : 0) ? TCL_OK : wm_error(interp, "native fullscreen update was denied");
    }
    return wm_error(interp, "this window-manager option is not implemented by the native Tk backend");
arguments:
    Tcl_WrongNumArgs(interp, 1, objv, "option window ?arg ...?"); return TCL_ERROR;
}

// ------------------------=
// FUNC: Tk_GetRootCoords
// DESC: Computes root coordinates from actual native top-level and child geometry.
// ------------------=
void Tk_GetRootCoords(Tk_Window tkwin, int *x, int *y) {
    TkWindow *window = (TkWindow *)tkwin;
    *x = *y = 0;
    while (window) {
        *x += window->changes.x + window->changes.border_width;
        *y += window->changes.y + window->changes.border_width;
        if (window->flags & TK_TOP_HIERARCHY) break;
        window = window->parentPtr;
    }
}

// ------------------------=
// FUNC: Tk_GetVRootGeometry
// DESC: Returns the real native screen extent; there is no synthetic X11 virtual root.
// ------------------=
void Tk_GetVRootGeometry(Tk_Window tkwin, int *x, int *y, int *width, int *height) {
    *x = *y = 0; *width = WidthOfScreen(Tk_Screen(tkwin)); *height = HeightOfScreen(Tk_Screen(tkwin));
}

// ------------------------=
// FUNC: Tk_MoveToplevelWindow
// DESC: Moves the native top-level and retains its requested coordinates before realization.
// ------------------=
void Tk_MoveToplevelWindow(Tk_Window tkwin, int x, int y) {
    TkWindow *window = (TkWindow *)tkwin;
    window->changes.x = x; window->changes.y = y;
    if (window->window != None) XMoveWindow(window->display, window->window, x, y);
    else window->dirtyChanges |= CWX | CWY;
}

// ------------------------=
// FUNC: Tk_SetGrid
// DESC: Retains a widget's genuine grid increments and translates later requests through the native layout manager.
// ------------------=
void Tk_SetGrid(Tk_Window tkwin, int width, int height, int width_step, int height_step) {
    TkWindow *window = TkWmFocusToplevel((TkWindow *)tkwin);
    if (!window || !window->wmInfoPtr || width_step < 1 || height_step < 1) return;
    WmInfo *wm = window->wmInfoPtr;
    wm->grid_window = tkwin; wm->grid_width = width; wm->grid_height = height;
    wm->width_step = width_step; wm->height_step = height_step;
    wm_geometry(NULL, (Tk_Window)window);
}

// ------------------------=
// FUNC: Tk_UnsetGrid
// DESC: Releases the requesting widget's grid policy without changing other top-levels.
// ------------------=
void Tk_UnsetGrid(Tk_Window tkwin) {
    TkWindow *window = TkWmFocusToplevel((TkWindow *)tkwin);
    if (!window || !window->wmInfoPtr || window->wmInfoPtr->grid_window != tkwin) return;
    window->wmInfoPtr->grid_window = NULL;
    window->wmInfoPtr->width_step = window->wmInfoPtr->height_step = 1;
}

// ------------------------=
// FUNC: TkUnixSetMenubar
// DESC: Positions and maps the real Tk menu widget inside its native top-level.
// ------------------=
void TkUnixSetMenubar(Tk_Window tkwin, Tk_Window menu) {
    TkWindow *window = (TkWindow *)tkwin;
    if (!window->wmInfoPtr) return;
    if (window->wmInfoPtr->menu && window->wmInfoPtr->menu != menu) Tk_UnmapWindow(window->wmInfoPtr->menu);
    window->wmInfoPtr->menu = menu; wm_geometry(NULL, tkwin);
}

// ------------------------=
// FUNC: TkpMakeMenuWindow
// DESC: Marks popup menus as unmanaged native transient surfaces before creating their drawable.
// ------------------=
void TkpMakeMenuWindow(Tk_Window tkwin, int transient) {
    (void)transient;
    XSetWindowAttributes attributes = {0}; attributes.override_redirect = True;
    Tk_ChangeWindowAttributes(tkwin, CWOverrideRedirect, &attributes);
}

// ------------------------=
// FUNC: TkWmStackorderToplevel
// DESC: Returns this interpreter's mapped top-levels in the actual native root's stacking order.
// ------------------=
TkWindow **TkWmStackorderToplevel(TkWindow *parent) {
    Window root, ancestor, *children = NULL; unsigned count = 0;
    if (!XQueryTree(parent->display, RootWindow(parent->display, parent->screenNum), &root, &ancestor, &children, &count)) return NULL;
    TkWindow **result = (TkWindow **)ckalloc(((size_t)count + 1) * sizeof(*result));
    unsigned used = 0;
    for (unsigned i = 0; i < count; ++i) {
        TkWindow *window = (TkWindow *)Tk_IdToWindow(parent->display, children[i]);
        if (window && window->mainPtr == parent->mainPtr && (window->flags & TK_TOP_HIERARCHY)
            && (window->flags & TK_MAPPED)) result[used++] = window;
    }
    result[used] = NULL;
    if (children) XFree(children);
    return result;
}

// ------------------------=
// FUNC: wm_hit
// DESC: Finds the frontmost mapped child containing a real desktop point, bounded by Tk's own window hierarchy.
// ------------------=
static TkWindow *wm_hit(TkWindow *window, int x, int y) {
    if (!(window->flags & TK_MAPPED)) return NULL;
    int left, top; Tk_GetRootCoords((Tk_Window)window, &left, &top);
    if (x < left || y < top || x - left >= window->changes.width || y - top >= window->changes.height) return NULL;
    TkWindow *hit = window;
    for (TkWindow *child = window->childList; child; child = child->nextPtr) {
        if (child->flags & TK_TOP_HIERARCHY) continue;
        TkWindow *candidate = wm_hit(child, x, y);
        if (candidate) hit = candidate;
    }
    return hit;
}

// ------------------------=
// FUNC: Tk_CoordsToWindow
// DESC: Resolves a desktop point through real stacking and mapped child geometry.
// ------------------=
Tk_Window Tk_CoordsToWindow(int x, int y, Tk_Window tkwin) {
    TkWindow **ordered = TkWmStackorderToplevel((TkWindow *)tkwin), *hit = NULL;
    if (!ordered) return NULL;
    for (size_t i = 0; ordered[i]; ++i) { TkWindow *candidate = wm_hit(ordered[i], x, y); if (candidate) hit = candidate; }
    ckfree(ordered); return (Tk_Window)hit;
}

// ------------------------=
// FUNC: TkGetPointerCoords
// DESC: Queries the native input backend and reports unavailable coordinates rather than a fictitious pointer origin.
// ------------------=
void TkGetPointerCoords(Tk_Window tkwin, int *x, int *y) {
    Window root, child; int local_x, local_y; unsigned mask;
    Tk_MakeWindowExist(tkwin);
    if (!XQueryPointer(Tk_Display(tkwin), Tk_WindowId(tkwin), &root, &child, x, y, &local_x, &local_y, &mask)) *x = *y = -1;
}

// ------------------------=
// FUNC: TkSetTransientFor
// DESC: Records a native drawable's real parent relationship through the backend's window property contract.
// ------------------=
void TkSetTransientFor(Tk_Window tkwin, Tk_Window parent) {
    Tk_MakeWindowExist(tkwin);
    if (parent) { Tk_MakeWindowExist(parent); XSetTransientForHint(Tk_Display(tkwin), Tk_WindowId(tkwin), Tk_WindowId(parent)); }
    else XDeleteProperty(Tk_Display(tkwin), Tk_WindowId(tkwin), Tk_InternAtom(tkwin, "WM_TRANSIENT_FOR"));
}

// ------------------------=
// FUNC: TkpScanWindowId
// DESC: Parses a representable native window token without resolving host window identifiers.
// ------------------=
int TkpScanWindowId(Tcl_Interp *interp, const char *text, Window *out) {
    Tcl_Obj *value = Tcl_NewStringObj(text, -1); Tcl_IncrRefCount(value);
    Tcl_WideInt number;
    int result = Tcl_GetWideIntFromObj(interp, value, &number);
    Tcl_DecrRefCount(value);
    if (result != TCL_OK) return result;
    if (number < 0 || (uint64_t)(Window)number != (uint64_t)number) return wm_error(interp, "invalid native window identifier");
    *out = (Window)number; return TCL_OK;
}
