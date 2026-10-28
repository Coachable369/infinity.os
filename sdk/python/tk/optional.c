#include "native.h"
#include "tkUnixInt.h"

typedef struct LocalApp { TkMainInfo *main; Tk_Uid name; } LocalApp;
static LocalApp apps[16];

// ------------------------=
// FUNC: unsupported
// DESC: Reports unsupported ambient desktop operations explicitly rather than granting authority or fabricating success.
// ------------------=
static int unsupported(Tcl_Interp *interp,const char *operation){Tcl_SetObjResult(interp,Tcl_ObjPrintf("%s is unavailable through this native application's capabilities",operation));Tcl_SetErrorCode(interp,"INFINITY","UNSUPPORTED",operation,NULL);return TCL_ERROR;}
// ------------------------=
// FUNC: TkGetCursorByName
// DESC: Maps named cursors only to actual installed native pointer shapes, rejecting unimplemented custom bitmap cursors.
// ------------------=
TkCursor *TkGetCursorByName(Tcl_Interp *interp,Tk_Window window,Tk_Uid name){
    unsigned shape;if(!strcmp(name,"arrow")||!strcmp(name,"left_ptr")||!strcmp(name,"top_left_arrow"))shape=0;
    else if(!strcmp(name,"xterm"))shape=1;
    else if(!strcmp(name,"bottom_right_corner")||!strcmp(name,"top_left_corner"))shape=2;
    else if(!strcmp(name,"bottom_left_corner")||!strcmp(name,"top_right_corner"))shape=3;
    else if(!strcmp(name,"sb_v_double_arrow")||!strcmp(name,"top_side")||!strcmp(name,"bottom_side"))shape=4;
    else if(!strcmp(name,"sb_h_double_arrow")||!strcmp(name,"left_side")||!strcmp(name,"right_side"))shape=5;
    else{unsupported(interp,"cursor shape");return NULL;}
    TkCursor *cursor=ckalloc(sizeof(*cursor));memset(cursor,0,sizeof(*cursor));cursor->cursor=(Tk_Cursor)(uintptr_t)shape;cursor->display=Tk_Display(window);return cursor;
}
// ------------------------=
// FUNC: TkpFreeCursor
// DESC: Releases no native resources for immutable installed cursor shapes; upstream Tk owns the cursor record.
// ------------------=
void TkpFreeCursor(TkCursor *cursor){(void)cursor;}
// ------------------------=
// FUNC: TkCreateCursorFromData
// DESC: Rejects custom cursor upload until native cursor authority supports bitmap resources.
// ------------------=
TkCursor *TkCreateCursorFromData(Tk_Window window,const char *source,const char *mask,int width,int height,int x,int y,XColor fg,XColor bg){(void)window;(void)source;(void)mask;(void)width;(void)height;(void)x;(void)y;(void)fg;(void)bg;return NULL;}
// ------------------------=
// FUNC: Tk_SetAppName
// DESC: Registers a stable application-local interpreter identity without installing an unauthenticated cross-process send endpoint.
// ------------------=
const char *Tk_SetAppName(Tk_Window window,const char *name){TkMainInfo *main=((TkWindow *)window)->mainPtr;
    LocalApp *slot=NULL;for(unsigned i=0;i<16;++i){if(apps[i].main==main){slot=&apps[i];break;}if(!apps[i].main&&!slot)slot=&apps[i];}
    if(!slot)return NULL;slot->main=main;slot->name=Tk_GetUid(name);return slot->name;}
// ------------------------=
// FUNC: TkGetInterpNames
// DESC: Lists only this process's actual registered Tk interpreters, never pretending to discover unauthorized external processes.
// ------------------=
int TkGetInterpNames(Tcl_Interp *interp,Tk_Window window){(void)window;Tcl_Obj *result=Tcl_NewListObj(0,NULL);for(unsigned i=0;i<16;++i)if(apps[i].main)Tcl_ListObjAppendElement(interp,result,Tcl_NewStringObj(apps[i].name,-1));Tcl_SetObjResult(interp,result);return TCL_OK;}
// ------------------------=
// FUNC: TkSendCleanup
// DESC: Clears local interpreter registrations when their display is destroyed; no ambient IPC endpoint exists.
// ------------------=
void TkSendCleanup(TkDisplay *display){for(unsigned i=0;i<16;++i)if(apps[i].main&&apps[i].main->winPtr->dispPtr==display)memset(&apps[i],0,sizeof(apps[i]));}
// ------------------------=
// FUNC: TkpTestsendCmd
// DESC: Rejects upstream development-only cross-application send operations outside native capability authority.
// ------------------=
int TkpTestsendCmd(ClientData client,Tcl_Interp *interp,int argc,Tcl_Obj *const argv[]){(void)client;(void)argc;(void)argv;return unsupported(interp,"cross-application send");}
// ------------------------=
// FUNC: TkpUseWindow
// DESC: Rejects foreign window embedding because native windows cannot be adopted without explicit delegated authority.
// ------------------=
int TkpUseWindow(Tcl_Interp *interp,Tk_Window window,const char *id){(void)window;(void)id;return unsupported(interp,"foreign window embedding");}
// ------------------------=
// FUNC: TkpGetOtherWindow
// DESC: Reports that no foreign embedded window has been authorized for this native widget.
// ------------------=
TkWindow *TkpGetOtherWindow(TkWindow *window){(void)window;return NULL;}
// ------------------------=
// FUNC: TkpMakeContainer
// DESC: Marks local embedding intent without granting another process access to the private native drawable.
// ------------------=
void TkpMakeContainer(Tk_Window window){((TkWindow *)window)->flags|=TK_CONTAINER;}
// ------------------------=
// FUNC: TkUnixContainerId
// DESC: Returns no externally usable window identifier for unsupported foreign embedding.
// ------------------=
Window TkUnixContainerId(TkWindow *window){(void)window;return None;}
// ------------------------=
// FUNC: TkSelGetSelection
// DESC: Reports unsupported cross-process selection retrieval; actual same-interpreter selection handlers remain upstream Tk's responsibility.
// ------------------=
int TkSelGetSelection(Tcl_Interp *interp,Tk_Window window,Atom selection,Atom target,Tk_GetSelProc *callback,ClientData data){(void)window;(void)selection;(void)target;(void)callback;(void)data;return unsupported(interp,"external clipboard selection");}
// ------------------------=
// FUNC: TkSelEventProc
// DESC: Ignores X-server selection transport events because this native backend does not expose such a transport.
// ------------------=
void TkSelEventProc(Tk_Window window,XEvent *event){(void)window;(void)event;}
// ------------------------=
// FUNC: TkSelPropProc
// DESC: Accepts no external X property-transfer stream; native selection data is handled synchronously within upstream Tk.
// ------------------=
void TkSelPropProc(XEvent *event){(void)event;}
// ------------------------=
// FUNC: Tk_FreeXId
// DESC: Retires no reusable server ID because native drawable IDs are monotonic and freed by their owning resource path.
// ------------------=
void Tk_FreeXId(Display *display,XID id){(void)display;(void)id;}
// ------------------------=
// FUNC: Tk_GetUserInactiveTime
// DESC: Explicitly reports unavailable global user-activity metadata rather than exposing another process's input history.
// ------------------=
long Tk_GetUserInactiveTime(Display *display){(void)display;return -1;}
// ------------------------=
// FUNC: Tk_ResetUserInactiveTime
// DESC: Does not fabricate user activity or bypass native lock policy for application-owned widgets.
// ------------------=
void Tk_ResetUserInactiveTime(Display *display){(void)display;}
// ------------------------=
// FUNC: XBell
// DESC: Rejects audible alert requests until the application owns a real AudioKit playback capability.
// ------------------=
int XBell(Display *display,int percent){(void)display;(void)percent;errno=ENOTSUP;return BadAccess;}
// ------------------------=
// FUNC: XForceScreenSaver
// DESC: Denies application attempts to reset or activate OS lock policy through a compatibility display API.
// ------------------=
int XForceScreenSaver(Display *display,int mode){(void)display;(void)mode;errno=EPERM;return BadAccess;}
// ------------------------=
// FUNC: TkpWarpPointer
// DESC: Reports an unauthorized OS-pointer warp as a Tcl background error instead of fabricating input-device movement.
// ------------------=
void TkpWarpPointer(TkDisplay *display){TkWindow *window=(TkWindow *)display->warpMainwin;if(!window||!window->mainPtr)return;
    Tcl_Interp *interp=window->mainPtr->interp;unsupported(interp,"OS pointer warp");Tcl_BackgroundException(interp,TCL_ERROR);}
// ------------------------=
// FUNC: XRefreshKeyboardMapping
// DESC: Validates an identity-mapped native keyboard refresh; normalized IEF key scalars need no host keymap cache.
// ------------------=
int XRefreshKeyboardMapping(XMappingEvent *event){return event&&event->type==MappingNotify?Success:BadValue;}
// ------------------------=
// FUNC: TkpRedirectKeyEvent
// DESC: Discards only foreign-embedding redirects, which this capability-isolated backend never authorizes.
// ------------------=
void TkpRedirectKeyEvent(TkWindow *window,XEvent *event){(void)event;if(window&&(window->flags&TK_EMBEDDED)&&window->mainPtr){
    Tcl_Interp *interp=window->mainPtr->interp;unsupported(interp,"foreign key-event redirect");Tcl_BackgroundException(interp,TCL_ERROR);}}
// ------------------------=
// FUNC: TkpTestembedCmd
// DESC: Returns an explicit unsupported result for upstream development-only foreign embedding tests.
// ------------------=
int TkpTestembedCmd(ClientData client,Tcl_Interp *interp,int count,Tcl_Obj *const values[]){(void)client;(void)count;(void)values;return unsupported(interp,"foreign embedding test");}
// ------------------------=
// FUNC: TkClipCleanup
// DESC: Releases genuine upstream clipboard handlers and its private selection window on display teardown.
// ------------------=
void TkClipCleanup(TkDisplay *display){if(display->clipWindow){Tk_Window window=display->clipWindow;display->clipWindow=NULL;
    Tk_DeleteSelHandler(window,display->clipboardAtom,display->applicationAtom);Tk_DeleteSelHandler(window,display->clipboardAtom,display->windowAtom);
    Tk_DestroyWindow(window);Tcl_Release(window);}}
