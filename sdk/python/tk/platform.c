#include "native.h"
#include "tkUnixInt.h"
#include <time.h>

typedef struct NamedKey { const char *name; KeySym value; } NamedKey;
static const NamedKey key_names[]={
#include "ks_names.h"
    {NULL,0}
};
static XErrorHandler error_handler;

// ------------------------=
// FUNC: TkpInit
// DESC: Selects the exact bundled upstream Tk resource library; native display authentication is checked when its real display is opened.
// ------------------=
int TkpInit(Tcl_Interp *interp){Tcl_SetVar2(interp,"tk_library",NULL,"/system/python/tk8.6",TCL_GLOBAL_ONLY);return TCL_OK;}
// ------------------------=
// FUNC: TkGetDefaultScreenName
// DESC: Defaults to the native Window Server instead of requiring a host X11 DISPLAY environment.
// ------------------=
const char *TkGetDefaultScreenName(Tcl_Interp *interp,const char *name){(void)interp;return name&&*name?name:"native:0";}
// ------------------------=
// FUNC: TkpGetAppName
// DESC: Derives the Tk application name from the current native script identity.
// ------------------=
void TkpGetAppName(Tcl_Interp *interp,Tcl_DString *name){const char *value=Tcl_GetVar2(interp,"argv0",NULL,TCL_GLOBAL_ONLY);if(!value||!*value)value="Python";const char *tail=strrchr(value,'/');Tcl_DStringAppend(name,tail?tail+1:value,-1);}
// ------------------------=
// FUNC: TkpDisplayWarning
// DESC: Emits native Tk startup diagnostics through the invoking application's real stderr stream.
// ------------------=
void TkpDisplayWarning(const char *message,const char *title){fprintf(stderr,"%s: %s\n",title,message);}
// ------------------------=
// FUNC: TkGetServerInfo
// DESC: Reports the actual native graphics implementation rather than claiming an external X server exists.
// ------------------=
void TkGetServerInfo(Tcl_Interp *interp,Tk_Window window){(void)window;Tcl_SetObjResult(interp,Tcl_NewStringObj("InfinityOS native Window Server / FontKit / IEF",-1));}
// ------------------------=
// FUNC: XStringToKeysym
// DESC: Resolves upstream Tk's real complete keysym-name table for event bindings.
// ------------------=
KeySym XStringToKeysym(const char *name){if(!name)return NoSymbol;if(name[0]&&!name[1])return (unsigned char)name[0];for(unsigned i=0;key_names[i].name;++i)if(!strcmp(name,key_names[i].name))return key_names[i].value;return NoSymbol;}
// ------------------------=
// FUNC: XKeysymToString
// DESC: Returns the upstream name of a known keysym without inventing unsupported key identities.
// ------------------=
char *XKeysymToString(KeySym key){for(unsigned i=0;key_names[i].name;++i)if(key_names[i].value==key)return (char *)key_names[i].name;return NULL;}
// ------------------------=
// FUNC: TkpInitKeymapInfo
// DESC: Uses normalized native modifier flags instead of querying an absent X keyboard server.
// ------------------=
void TkpInitKeymapInfo(TkDisplay *d){d->bindInfoStale=0;d->lockUsage=LU_IGNORE;d->modeModMask=0;d->metaModMask=Mod1Mask;d->altModMask=Mod1Mask;d->numModKeyCodes=0;d->modKeyCodes=NULL;}
// ------------------------=
// FUNC: TkpGetKeySym
// DESC: Returns the trusted normalized keysym carried by native key events, including full Unicode scalar values.
// ------------------=
KeySym TkpGetKeySym(TkDisplay *display,XEvent *event){(void)display;return event->xkey.keycode;}
// ------------------------=
// FUNC: TkpSetKeycodeAndState
// DESC: Preserves a requested keysym in Tk-generated application-local events without injecting native device input.
// ------------------=
void TkpSetKeycodeAndState(Tk_Window window,KeySym key,XEvent *event){(void)window;event->xkey.keycode=key;}
// ------------------------=
// FUNC: TkpGetString
// DESC: Converts genuine native key scalars to UTF-8 and preserves standard Return/Tab editing behavior.
// ------------------=
const char *TkpGetString(TkWindow *window,XEvent *event,Tcl_DString *result){
    (void)window;KeySym key=event->xkey.keycode;char bytes[TCL_UTF_MAX];int length=0;
    if(key==XK_Return)key='\r';else if(key==XK_Tab)key='\t';else if(key==XK_BackSpace)key='\b';
    else if(key>=0xff00&&key<=0xffff)key=0;
    if((event->xkey.state&ControlMask)&&key>='@'&&key<127)key&=31;
    if(key&&key<=0xffff)length=Tcl_UniCharToUtf((Tcl_UniChar)key,bytes);
    Tcl_DStringAppend(result,bytes,length);return Tcl_DStringValue(result);
}
// ------------------------=
// FUNC: TkKeycodeToKeysym
// DESC: Provides identity mapping for normalized native keycodes; no host hardware key map is consulted.
// ------------------=
KeySym TkKeycodeToKeysym(TkDisplay *display,unsigned int code,int group,int index){(void)display;(void)group;(void)index;return code;}
// ------------------------=
// FUNC: XLookupString
// DESC: Supplies ordinary key text for upstream Tk consumers from the same normalized native event.
// ------------------=
int XLookupString(XKeyEvent *event,char *buffer,int capacity,KeySym *key,XComposeStatus *compose){
    (void)compose;if(key)*key=event->keycode;Tcl_DString result;Tcl_DStringInit(&result);TkpGetString(NULL,(XEvent *)event,&result);
    int count=Tcl_DStringLength(&result);if(count>capacity)count=capacity;if(count>0)memcpy(buffer,Tcl_DStringValue(&result),count);Tcl_DStringFree(&result);return count;
}
// ------------------------=
// FUNC: Tk_SetCaretPos
// DESC: Records the real caret geometry for native text-input clients; this platform does not advertise XIM composition support.
// ------------------=
void Tk_SetCaretPos(Tk_Window window,int x,int y,int height){TkDisplay *display=((TkWindow *)window)->dispPtr;display->caret.winPtr=(TkWindow *)window;display->caret.x=x;display->caret.y=y;display->caret.height=height;}
// ------------------------=
// FUNC: XAllocColor
// DESC: Converts a requested color to the native true-color pixel encoding without palette allocation.
// ------------------=
Status XAllocColor(Display *display,Colormap map,XColor *color){(void)display;if(map!=1||!color)return 0;color->pixel=TkpGetPixel(color);return 1;}
// ------------------------=
// FUNC: XAllocNamedColor
// DESC: Uses upstream's actual named-color parser and returns the native representable color.
// ------------------=
Status XAllocNamedColor(Display *d,Colormap map,const char *name,XColor *screen,XColor *exact){if(!XParseColor(d,map,name,exact))return 0;*screen=*exact;return XAllocColor(d,map,screen);}
// ------------------------=
// FUNC: XLookupColor
// DESC: Resolves color metadata without allocating nonexistent palette entries.
// ------------------=
Status XLookupColor(Display *d,Colormap map,const char *name,XColor *exact,XColor *screen){return XAllocNamedColor(d,map,name,screen,exact);}
// ------------------------=
// FUNC: XFreeColors
// DESC: Releases no palette resources because native true-color values are immutable scalar pixel values.
// ------------------=
int XFreeColors(Display *d,Colormap map,unsigned long *pixels,int count,unsigned long planes){(void)d;(void)pixels;(void)count;(void)planes;return map==1?Success:BadColor;}
// ------------------------=
// FUNC: XQueryColor
// DESC: Reads exact channel values from a native true-color pixel.
// ------------------=
int XQueryColor(Display *d,Colormap map,XColor *color){(void)d;if(map!=1||!color)return BadColor;color->red=((color->pixel>>16)&255)*257;color->green=((color->pixel>>8)&255)*257;color->blue=(color->pixel&255)*257;color->flags=DoRed|DoGreen|DoBlue;return Success;}
// ------------------------=
// FUNC: XQueryColors
// DESC: Resolves every supplied native true-color pixel without querying an external display.
// ------------------=
int XQueryColors(Display *d,Colormap map,XColor *colors,int count){if(count<0)return BadValue;for(int i=0;i<count;++i)if(XQueryColor(d,map,&colors[i])!=Success)return BadColor;return Success;}
// ------------------------=
// FUNC: XCreateColormap
// DESC: Exposes only the installed native TrueColor visual; unsupported palette allocation is rejected.
// ------------------=
Colormap XCreateColormap(Display *d,Window window,Visual *visual,int allocation){(void)window;return d&&visual==DefaultVisual(d,0)&&allocation==AllocNone?1:None;}
// ------------------------=
// FUNC: XFreeColormap
// DESC: Keeps the immutable shared native TrueColor visual alive while rejecting unknown palette identities.
// ------------------=
int XFreeColormap(Display *d,Colormap map){(void)d;return map==1?Success:BadColor;}
// ------------------------=
// FUNC: XGetVisualInfo
// DESC: Returns the actual native display visual only when every requested property matches it.
// ------------------=
XVisualInfo *XGetVisualInfo(Display *d,long mask,XVisualInfo *wanted,int *count){
    *count=0;Visual *v=DefaultVisual(d,0);XVisualInfo value={v,v->visualid,0,DefaultDepth(d,0),v->c_class,v->red_mask,v->green_mask,v->blue_mask,v->map_entries,v->bits_per_rgb};
    if(((mask&VisualIDMask)&&wanted->visualid!=value.visualid)||((mask&VisualScreenMask)&&wanted->screen!=0)||((mask&VisualDepthMask)&&wanted->depth!=value.depth)||((mask&VisualClassMask)&&wanted->c_class!=value.c_class))return NULL;
    XVisualInfo *result=ckalloc(sizeof(*result));*result=value;*count=1;return result;
}
// ------------------------=
// FUNC: XVisualIDFromVisual
// DESC: Returns the actual installed native visual identity.
// ------------------=
VisualID XVisualIDFromVisual(Visual *visual){return visual?visual->visualid:0;}
// ------------------------=
// FUNC: XFree
// DESC: Releases X-interface allocations with the same Tcl allocator that created them.
// ------------------=
int XFree(void *memory){if(memory)ckfree(memory);return Success;}
// ------------------------=
// FUNC: XSetErrorHandler
// DESC: Maintains the caller's actual local X-interface error handler for upstream Tk scoped error handling.
// ------------------=
XErrorHandler XSetErrorHandler(XErrorHandler next){XErrorHandler previous=error_handler;error_handler=next;return previous;}
// ------------------------=
// FUNC: XGetErrorText
// DESC: Formats native graphics API error codes without claiming success for unavailable operations.
// ------------------=
int XGetErrorText(Display *d,int code,char *buffer,int length){(void)d;if(length>0)snprintf(buffer,length,"Native graphics error %d",code);return Success;}
