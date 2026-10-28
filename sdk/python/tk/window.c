#include "native.h"
#include <infinity/gui.h>
#include <infinity/surface.h>
#include <infinity/event.h>
#include <infinity/input.h>
#include <time.h>
#include "tkBusy.h"
#include "events.h"

#define WINDOW_LIMIT 128
typedef struct NativeWindow {
    Window id,parent;
    TkWindow *tk;
    int x,y,mapped,input_only;
    unsigned width,height;
    unsigned long background;
    Cursor cursor;
    unsigned border_width;
    unsigned long border_color;
    Pixmap background_pixmap,border_pixmap;
    char title[97];
    uint64_t order,paint_revision,generation;
    InfinityWindowId native;
    InfinitySurfaceId surface;
    InfinityCapability publish,resize,destroy,manage,focus,capture;
    uint64_t input,closed,close_requested,input_grant,closed_grant,request_grant,lease_deadline;
} NativeWindow;
static NativeWindow windows[WINDOW_LIMIT];
static Display display;
static Screen screen;
static Visual visual;
static Depth depth;
static uint64_t stacking;
static int initialized;
static Window keyboard_window,hover_window,captured_window,keyboard_grab;
static int pointer_x,pointer_y;
static unsigned pointer_state;
static XAfterFunction synchronous_hook;
extern void InfinityTclSetEventPump(int (*pump)(void));
static int pump(void);
static int root_position(Window id,int *x,int *y);

// ------------------------=
// FUNC: lookup_window
// DESC: Resolves only process-owned Tk windows by their nonreused drawable identity.
// ------------------=
static NativeWindow *lookup_window(Window id){for(unsigned i=0;i<WINDOW_LIMIT;++i)if(id&&windows[i].id==id)return &windows[i];return NULL;}
// ------------------------=
// FUNC: top_window
// DESC: Resolves an owned native toplevel through a bounded private child hierarchy.
// ------------------=
static NativeWindow *top_window(NativeWindow *window){for(unsigned i=0;window&&i<WINDOW_LIMIT;++i){if(window->parent==1||!window->parent)return window;window=lookup_window(window->parent);}return NULL;}
// ------------------------=
// FUNC: monotonic_ns
// DESC: Reads the same native monotonic clock used by finite IEF leases, never host wall time.
// ------------------=
static uint64_t monotonic_ns(void){struct timespec value;if(clock_gettime(CLOCK_MONOTONIC,&value))return 0;return (uint64_t)value.tv_sec*1000000000u+value.tv_nsec;}
// ------------------------=
// FUNC: queue_event
// DESC: Queues a real mapped/configured/exposed native window transition through upstream Tk event dispatch.
// ------------------=
static void queue_event(NativeWindow *w,int type){
    XEvent e;InfinityTkWindowEvent(&e,&display,w->id,type,++display.request,w->x,w->y,
        w->width,w->height,w->border_width,w->tk?w->tk->atts.override_redirect:False);
    Tk_QueueWindowEvent(&e,TCL_QUEUE_TAIL);
}
// ------------------------=
// FUNC: native_release
// DESC: Releases event leases, window, and its retained surface in ownership-safe order.
// ------------------=
static void native_release(NativeWindow *w){
    if(w->input)infinity_event_unsubscribe(w->input);if(w->closed)infinity_event_unsubscribe(w->closed);if(w->close_requested)infinity_event_unsubscribe(w->close_requested);
    if(w->native)infinity_window_close(w->manage,w->native);
    if(w->surface&&w->destroy)infinity_surface_destroy(w->destroy,w->surface);
    w->input=w->closed=w->close_requested=0;w->native=w->surface=0;w->lease_deadline=0;
}
// ------------------------=
// FUNC: renew_leases
// DESC: Renews bounded owned input subscriptions before expiry so long-running Tk applications do not silently lose interaction.
// ------------------=
static int renew_leases(NativeWindow *w){
    uint64_t now=monotonic_ns();if(w->input&&w->closed&&w->close_requested&&now+1000000000u<w->lease_deadline)return 1;
    if(w->input)infinity_event_unsubscribe(w->input);if(w->closed)infinity_event_unsubscribe(w->closed);if(w->close_requested)infinity_event_unsubscribe(w->close_requested);w->input=w->closed=w->close_requested=0;
    uint64_t deadline=now+300000000000u;
    int64_t input=infinity_event_subscribe(w->input_grant,0x9800d,w->native,deadline,8);if(input<0)return 0;
    int64_t closed=infinity_event_subscribe(w->closed_grant,0x98002,w->native,deadline,2);
    if(closed<0){infinity_event_unsubscribe(input);return 0;}
    int64_t requested=infinity_event_subscribe(w->request_grant,0x9800e,w->native,deadline,2);
    if(requested<0){infinity_event_unsubscribe(input);infinity_event_unsubscribe(closed);return 0;}
    w->input=input;w->closed=closed;w->close_requested=requested;w->lease_deadline=deadline;return 1;
}
// ------------------------=
// FUNC: native_create
// DESC: Creates a capability-scoped retained native toplevel and input subscriptions, rolling back incomplete creation.
// ------------------=
static int native_create(NativeWindow *w){
    if(w->native)return 1;int64_t create=infinity_gui_request(INFINITY_GUI_SURFACE_CREATE,0);if(create<0)return 0;
    int64_t surface=infinity_surface_create(create,w->width,w->height,INFINITY_XRGB8888);if(surface<0)return 0;w->surface=surface;
    int64_t grant=infinity_gui_request(INFINITY_GUI_SURFACE_DESTROY,w->surface);if(grant<0)return 0;w->destroy=grant;
    grant=infinity_gui_request(INFINITY_GUI_SURFACE_PUBLISH,w->surface);if(grant<0)goto failed;w->publish=grant;
    grant=infinity_gui_request(INFINITY_GUI_SURFACE_RESIZE,w->surface);if(grant<0)goto failed;w->resize=grant;
    create=infinity_gui_request(INFINITY_GUI_WINDOW_CREATE,0);if(create<0)goto failed;
    int64_t native=infinity_window_create(create,w->surface,w->x,w->y,w->width,w->height);if(native<0)goto failed;w->native=native;
    grant=infinity_gui_request(INFINITY_GUI_WINDOW_MANAGE,w->native);if(grant<0)goto failed;w->manage=grant;
    if(w->title[0]&&infinity_window_title(w->manage,w->native,w->title,strlen(w->title))<0)goto failed;
    grant=infinity_gui_request(INFINITY_GUI_WINDOW_FOCUS,w->native);if(grant<0)goto failed;w->focus=grant;
    grant=infinity_gui_request(INFINITY_GUI_WINDOW_CAPTURE,w->native);if(grant<0)goto failed;w->capture=grant;
    grant=infinity_gui_request(INFINITY_GUI_INPUT_EVENTS,w->native);if(grant<0)goto failed;w->input_grant=grant;
    grant=infinity_gui_request(INFINITY_GUI_CLOSE_EVENTS,w->native);if(grant<0)goto failed;w->closed_grant=grant;
    grant=infinity_gui_request(INFINITY_GUI_CLOSE_REQUEST_EVENTS,w->native);if(grant<0)goto failed;w->request_grant=grant;
    if(!renew_leases(w))goto failed;
    w->x=infinity_gui_window_geometry(w->native,0);w->y=infinity_gui_window_geometry(w->native,1);
    w->paint_revision=0;w->generation=0;return 1;
failed:native_release(w);return 0;
}
// ------------------------=
// FUNC: compose
// DESC: Composes the real retained child hierarchy with ancestor clipping and deterministic sibling stacking.
// ------------------=
static uint64_t compose(NativeWindow *w,uint32_t *target,unsigned width,unsigned height,int ox,int oy,int clip_x,int clip_y,int clip_right,int clip_bottom,unsigned nesting){
    if(!w->mapped||nesting>=WINDOW_LIMIT)return 0;
    InfinityTkPixels *pixels=InfinityTkDrawable(w->id);if(!pixels)return 0;
    int left=ox>clip_x?ox:clip_x,top=oy>clip_y?oy:clip_y,right=ox+(int)w->width,bottom=oy+(int)w->height;
    if(right>clip_right)right=clip_right;if(bottom>clip_bottom)bottom=clip_bottom;
    if(target&&!w->input_only&&w->border_width){int b=w->border_width;
        InfinityTkPixels *tile=InfinityTkDrawable(w->border_pixmap);
        int bl=ox-b>clip_x?ox-b:clip_x,bt=oy-b>clip_y?oy-b:clip_y;
        int br=ox+(int)w->width+b,bb=oy+(int)w->height+b;if(br>clip_right)br=clip_right;if(bb>clip_bottom)bb=clip_bottom;
        for(int y=bt;y<bb;++y)for(int x=bl;x<br;++x)if(x>=0&&y>=0&&(unsigned)x<width&&(unsigned)y<height&&(x<ox||y<oy||x>=ox+(int)w->width||y>=oy+(int)w->height)){
            uint32_t color=tile?tile->pixels[(size_t)((y-oy+b)%tile->height)*tile->width+(x-ox+b)%tile->width]:w->border_color;
            target[(size_t)y*width+x]=color|0xff000000u;}}
    if(target&&!w->input_only)for(int y=top;y<bottom;++y)for(int x=left;x<right;++x)if(x>=0&&y>=0&&(unsigned)x<width&&(unsigned)y<height)
        target[(size_t)y*width+x]=pixels->pixels[(size_t)(y-oy)*pixels->width+(x-ox)]|0xff000000u;
    uint64_t hash=InfinityTkRevision(w->id)^((uint64_t)(uint32_t)ox<<32)^(uint32_t)oy^w->order;
    uint64_t previous=0;for(unsigned n=0;n<WINDOW_LIMIT;++n){NativeWindow *child=NULL;for(unsigned i=0;i<WINDOW_LIMIT;++i)
        if(windows[i].id&&windows[i].parent==w->id&&windows[i].mapped&&windows[i].order>previous&&(!child||windows[i].order<child->order))child=&windows[i];
        if(!child)break;previous=child->order;hash=hash*1099511628211u^compose(child,target,width,height,ox+child->x,oy+child->y,left,top,right,bottom,nesting+1);}
    return hash?hash:1;
}
// ------------------------=
// FUNC: present
// DESC: Uploads a fully composed toplevel in bounded native chunks and commits one generation only after every upload succeeds.
// ------------------=
static int present(NativeWindow *w){
    if(!w->mapped||!w->native)return 0;size_t bytes=(size_t)w->width*w->height*4;if(bytes>16u*1024u*1024u)return 0;
    uint64_t revision=compose(w,NULL,w->width,w->height,0,0,0,0,w->width,w->height,0);
    if(revision==w->paint_revision)return 0;
    uint32_t *pixels=calloc(1,bytes);if(!pixels)return 0;
    compose(w,pixels,w->width,w->height,0,0,0,0,w->width,w->height,0);
    for(size_t at=0;at<bytes;){size_t size=bytes-at;if(size>65536)size=65536;
        if(infinity_surface_upload(w->publish,w->surface,pixels+at/4,at,size)<0){free(pixels);return 0;}at+=size;}
    free(pixels);if(infinity_surface_commit(w->publish,w->surface,w->generation+1)<0)return 0;
    ++w->generation;w->paint_revision=revision;return 1;
}
// ------------------------=
// FUNC: hit_window
// DESC: Hit-tests mapped children front to back with coordinates relative to the actual native toplevel.
// ------------------=
static NativeWindow *hit_window(NativeWindow *parent,int *x,int *y,unsigned level){
    if(level>=WINDOW_LIMIT)return parent;NativeWindow *child=NULL;
    for(unsigned i=0;i<WINDOW_LIMIT;++i){NativeWindow *w=&windows[i];if(w->id&&w->mapped&&w->parent==parent->id&&*x>=w->x&&*y>=w->y&&*x-w->x<(int)w->width&&*y-w->y<(int)w->height&&(!child||w->order>child->order))child=w;}
    if(!child)return parent;*x-=child->x;*y-=child->y;return hit_window(child,x,y,level+1);
}
// ------------------------=
// FUNC: cross_event
// DESC: Reports real pointer enter/leave transitions so upstream button hover and bindings remain functional.
// ------------------=
static void cross_event(Window window,int kind,int x,int y,Time time){
    if(!window)return;XEvent e;memset(&e,0,sizeof(e));e.xcrossing.type=kind;e.xcrossing.display=&display;e.xcrossing.window=window;e.xcrossing.root=1;
    e.xcrossing.time=time;e.xcrossing.x=x;e.xcrossing.y=y;e.xcrossing.x_root=pointer_x;e.xcrossing.y_root=pointer_y;
    e.xcrossing.mode=NotifyNormal;e.xcrossing.detail=NotifyNonlinear;e.xcrossing.same_screen=True;e.xcrossing.state=pointer_state;Tk_QueueWindowEvent(&e,TCL_QUEUE_TAIL);
}
// ------------------------=
// FUNC: route_input
// DESC: Converts trusted window-local IEF input packets to real Tk child events without synthesizing device input into other applications.
// ------------------=
static void route_input(NativeWindow *top,const InfinityEvent *packet){
    if(packet->payload_length!=96)return;uint32_t data[24];memcpy(data,packet->payload,96);if(data[0]!=1||data[2]!=top->native)return;
    Time time=packet->timestamp/1000000u;XEvent e;memset(&e,0,sizeof(e));e.xany.display=&display;e.xany.serial=packet->sequence;
    if(data[1]==3){int x=(int32_t)data[4],y=(int32_t)data[5];pointer_x=top->x+x;pointer_y=top->y+y;
        NativeWindow *target=hit_window(top,&x,&y,0);if(!target)return;
        NativeWindow *captured=lookup_window(captured_window);
        if(captured&&top_window(captured)==top){int rx,ry;if(root_position(captured_window,&rx,&ry)){target=captured;x=pointer_x-rx;y=pointer_y-ry;}}
        infinity_gui_cursor(top->native,target->cursor);
        if(hover_window!=target->id){int rx=pointer_x-x,ry=pointer_y-y;root_position(hover_window,&rx,&ry);
            cross_event(hover_window,LeaveNotify,pointer_x-rx,pointer_y-ry,time);cross_event(target->id,EnterNotify,x,y,time);hover_window=target->id;}
        unsigned phase=data[3]&255,button=data[3]>>8;
        e.xbutton.window=target->id;e.xbutton.root=1;e.xbutton.time=time;e.xbutton.x=x;e.xbutton.y=y;e.xbutton.x_root=pointer_x;e.xbutton.y_root=pointer_y;e.xbutton.same_screen=True;e.xbutton.state=pointer_state;
        if(phase==0)e.type=MotionNotify;
        else if(phase==1||phase==2){e.type=phase==1?ButtonPress:ButtonRelease;e.xbutton.button=button;if(button>=1&&button<=5){unsigned mask=Button1Mask<<(button-1);if(phase==1)pointer_state|=mask;else pointer_state&=~mask;}}
        else if(phase==3){e.type=ButtonPress;e.xbutton.button=(int32_t)data[7]<0?4:5;Tk_QueueWindowEvent(&e,TCL_QUEUE_TAIL);e.type=ButtonRelease;}
        else{pointer_state=0;return;}
        Tk_QueueWindowEvent(&e,TCL_QUEUE_TAIL);
    }else if(data[1]==1){NativeWindow *target=lookup_window(keyboard_grab?keyboard_grab:keyboard_window);if(!target||top_window(target)!=top)target=top;
        static const KeySym special[]={NoSymbol,XK_Tab,XK_Return,XK_Escape,XK_space,XK_Left,XK_Right,XK_Up,XK_Down,XK_Home,XK_End,XK_BackSpace,XK_Delete};
        unsigned code=data[4];KeySym symbol=code==13?data[5]:code<sizeof(special)/sizeof(special[0])?special[code]:NoSymbol;
        if(!symbol)return;e.xkey.type=(data[3]&1)?KeyPress:KeyRelease;e.xkey.window=target->id;e.xkey.root=1;e.xkey.time=time;
        unsigned modifiers=data[3]>>8;
        e.xkey.keycode=symbol;e.xkey.state=pointer_state;
        if(modifiers&INFINITY_MODIFIER_SHIFT)e.xkey.state|=ShiftMask;
        if(modifiers&INFINITY_MODIFIER_CONTROL)e.xkey.state|=ControlMask;
        if(modifiers&INFINITY_MODIFIER_SUPER)e.xkey.state|=Mod4Mask;
        e.xkey.same_screen=True;Tk_QueueWindowEvent(&e,TCL_QUEUE_TAIL);
    }
}
// ------------------------=
// FUNC: observe_geometry
// DESC: Reconciles genuine shell move/resize/visibility changes into Tk's retained pixels and actual Configure/Map/Unmap events.
// ------------------=
static int observe_geometry(NativeWindow *w){
    int64_t x=infinity_gui_window_geometry(w->native,0),y=infinity_gui_window_geometry(w->native,1);
    int64_t width=infinity_gui_window_geometry(w->native,2),height=infinity_gui_window_geometry(w->native,3),visible=infinity_gui_window_geometry(w->native,4);
    if(width<=0||height<=0||width>8192||height>8192||visible<0)return 0;
    if(w->width!=(unsigned)width||w->height!=(unsigned)height){
        if(!InfinityTkResize(w->id,width,height))return 0;
        if(infinity_surface_resize(w->resize,w->surface,width,height)<0){InfinityTkResize(w->id,w->width,w->height);return 0;}
        ++w->generation;
    }
    int changed=w->x!=x||w->y!=y||w->width!=(unsigned)width||w->height!=(unsigned)height;
    if(changed){w->x=x;w->y=y;w->width=width;w->height=height;w->paint_revision=0;
        if(w->tk){w->tk->changes.x=x;w->tk->changes.y=y;w->tk->changes.width=width;w->tk->changes.height=height;}
        queue_event(w,ConfigureNotify);if(visible)queue_event(w,Expose);}
    if(w->mapped!=!!visible){w->mapped=!!visible;if(w->tk){if(visible)w->tk->flags|=TK_MAPPED;else w->tk->flags&=~TK_MAPPED;}
        queue_event(w,visible?MapNotify:UnmapNotify);if(visible)queue_event(w,Expose);changed=1;}
    return changed;
}
// ------------------------=
// FUNC: pump
// DESC: Performs bounded native input/close polling and retained presentation between Tcl timer/event iterations.
// ------------------=
static int pump(void){
    int work=0;for(unsigned i=0;i<WINDOW_LIMIT;++i){NativeWindow *w=&windows[i];if(!w->id||!w->native)continue;
        InfinityEvent e;for(unsigned n=0;n<8&&w->input;++n){int64_t result=infinity_event_poll(w->input,&e);if(result<=0)break;route_input(w,&e);++work;}
        if(w->close_requested&&infinity_event_poll(w->close_requested,&e)>0&&w->tk){
            XEvent close;memset(&close,0,sizeof(close));close.xclient.type=ClientMessage;close.xclient.display=&display;close.xclient.window=w->id;
            close.xclient.message_type=Tk_InternAtom((Tk_Window)w->tk,"WM_PROTOCOLS");close.xclient.format=32;
            close.xclient.data.l[0]=Tk_InternAtom((Tk_Window)w->tk,"WM_DELETE_WINDOW");Tk_QueueWindowEvent(&close,TCL_QUEUE_TAIL);++work;}
        if(w->closed&&infinity_event_poll(w->closed,&e)>0){w->mapped=0;queue_event(w,DestroyNotify);++work;}
        renew_leases(w);work+=observe_geometry(w);
        work+=present(w);}
    return work;
}
// ------------------------=
// FUNC: TkpOpenDisplay
// DESC: Opens the authenticated native Window Server using actual display dimensions and installed true-color pixel semantics.
// ------------------=
TkDisplay *TkpOpenDisplay(const char *name){
    if(name&&*name&&strcmp(name,":0")&&strcmp(name,"native:0"))return NULL;
    int64_t width=infinity_gui_geometry(2),height=infinity_gui_geometry(3);if(width<=0||height<=0)return NULL;
    if(!initialized){memset(&display,0,sizeof(display));memset(&screen,0,sizeof(screen));memset(&visual,0,sizeof(visual));
        visual.visualid=1;visual.c_class=TrueColor;visual.red_mask=0xff0000;visual.green_mask=0xff00;visual.blue_mask=0xff;visual.bits_per_rgb=8;visual.map_entries=256;
        depth.depth=24;depth.nvisuals=1;depth.visuals=&visual;screen.display=&display;screen.root=1;screen.width=width;screen.height=height;
        screen.mwidth=width*254/960;screen.mheight=height*254/960;screen.root_depth=24;screen.root_visual=&visual;screen.ndepths=1;screen.depths=&depth;screen.cmap=1;
        screen.white_pixel=0xffffff;screen.black_pixel=0;screen.max_maps=1;screen.min_maps=1;screen.backing_store=Always;
        display.fd=-1;display.vendor="InfinityOS native Window Server";display.release=1;display.display_name="native:0";display.nscreens=1;display.screens=&screen;
        display.byte_order=LSBFirst;display.bitmap_bit_order=LSBFirst;display.bitmap_unit=32;display.bitmap_pad=32;display.max_request_size=16384;display.min_keycode=1;display.max_keycode=0x10ffff;
        initialized=1;InfinityTclSetEventPump(pump);}
    TkDisplay *result=ckalloc(sizeof(*result));memset(result,0,sizeof(*result));result->display=&display;TkpInitKeymapInfo(result);return result;
}
// ------------------------=
// FUNC: TkpCloseDisplay
// DESC: Closes all owned native windows and retained pixels when the last Tk display is destroyed.
// ------------------=
void TkpCloseDisplay(TkDisplay *d){(void)d;for(unsigned i=0;i<WINDOW_LIMIT;++i)if(windows[i].id){native_release(&windows[i]);InfinityTkRelease(windows[i].id);memset(&windows[i],0,sizeof(windows[i]));}InfinityTclSetEventPump(NULL);initialized=0;}
// ------------------------=
// FUNC: TkpMakeWindow
// DESC: Creates a private retained Tk child or native toplevel drawable without granting a framebuffer mapping.
// ------------------=
Window TkpMakeWindow(TkWindow *tk,Window parent){
    NativeWindow *slot=NULL;for(unsigned i=0;i<WINDOW_LIMIT;++i)if(!windows[i].id){slot=&windows[i];break;}if(!slot)return None;
    unsigned width=tk->changes.width>0?tk->changes.width:1,height=tk->changes.height>0?tk->changes.height:1;
    Drawable id=InfinityTkAllocate(width,height,24);if(!id)return None;memset(slot,0,sizeof(*slot));slot->id=id;slot->parent=Tk_IsTopLevel((Tk_Window)tk)?1:parent;slot->tk=tk;
    slot->width=width;slot->height=height;slot->x=tk->changes.x;slot->y=tk->changes.y;slot->background=tk->atts.background_pixel;slot->order=++stacking;
    InfinityTkPixels *p=InfinityTkDrawable(id);for(size_t i=0;i<(size_t)width*height;++i)p->pixels[i]=slot->background|0xff000000u;return id;
}
// ------------------------=
// FUNC: XMapWindow
// DESC: Maps an owned Tk window only after its native toplevel resource is available and queues paintable exposure.
// ------------------=
int XMapWindow(Display *d,Window id){(void)d;NativeWindow *w=lookup_window(id);if(!w)return BadWindow;if(w->parent==1&&!native_create(w))return BadAccess;
    if(w->native&&infinity_window_state(w->manage,w->native,INFINITY_WINDOW_NORMAL)<0)return BadAccess;
    w->mapped=1;if(w->tk)w->tk->flags|=TK_MAPPED;w->order=++stacking;queue_event(w,MapNotify);queue_event(w,Expose);return Success;}
// ------------------------=
// FUNC: XUnmapWindow
// DESC: Hides the actual owned native toplevel while retaining private Tk widget state for restoration.
// ------------------=
int XUnmapWindow(Display *d,Window id){(void)d;NativeWindow *w=lookup_window(id);if(!w)return BadWindow;if(w->native&&infinity_window_state(w->manage,w->native,INFINITY_WINDOW_WITHDRAWN)<0)return BadAccess;
    w->mapped=0;if(w->tk)w->tk->flags&=~TK_MAPPED;queue_event(w,UnmapNotify);return Success;}
// ------------------------=
// FUNC: XDestroyWindow
// DESC: Destroys one owned window and its native resources after upstream Tk has dismantled child widgets.
// ------------------=
int XDestroyWindow(Display *d,Window id){(void)d;NativeWindow *w=lookup_window(id);if(!w)return BadWindow;native_release(w);InfinityTkReleaseProperties(id);InfinityTkRelease(id);if(keyboard_window==id)keyboard_window=0;if(hover_window==id)hover_window=0;memset(w,0,sizeof(*w));return Success;}
// ------------------------=
// FUNC: XMoveResizeWindow
// DESC: Applies native and private geometry changes with retained pixel overlap, then reports the actual committed geometry to Tk.
// ------------------=
int XMoveResizeWindow(Display *d,Window id,int x,int y,unsigned width,unsigned height){
    (void)d;NativeWindow *w=lookup_window(id);if(!w||!width||!height)return BadWindow;
    if(w->x==x&&w->y==y&&w->width==width&&w->height==height)return Success;
    if(w->native){if((size_t)width*height*4>16u*1024u*1024u)return BadAlloc;
        if(infinity_surface_resize(w->resize,w->surface,width,height)<0)return BadAlloc;++w->generation;
        if(infinity_window_resize(w->manage,w->native,width,height)<0)return BadAccess;
        if(infinity_window_move(w->manage,w->native,x,y)<0)return BadAccess;
        x=infinity_gui_window_geometry(w->native,0);y=infinity_gui_window_geometry(w->native,1);}
    if(!InfinityTkResize(id,width,height))return BadAlloc;w->x=x;w->y=y;w->width=width;w->height=height;w->paint_revision=0;
    if(w->tk){w->tk->changes.x=x;w->tk->changes.y=y;w->tk->changes.width=width;w->tk->changes.height=height;}
    queue_event(w,ConfigureNotify);if(w->mapped)queue_event(w,Expose);return Success;
}
// ------------------------=
// FUNC: XMoveWindow
// DESC: Preserves current dimensions while moving one real native or child window.
// ------------------=
int XMoveWindow(Display *d,Window id,int x,int y){NativeWindow *w=lookup_window(id);return w?XMoveResizeWindow(d,id,x,y,w->width,w->height):BadWindow;}
// ------------------------=
// FUNC: XResizeWindow
// DESC: Preserves current position while resizing one real retained window.
// ------------------=
int XResizeWindow(Display *d,Window id,unsigned width,unsigned height){NativeWindow *w=lookup_window(id);return w?XMoveResizeWindow(d,id,w->x,w->y,width,height):BadWindow;}
// ------------------------=
// FUNC: XRaiseWindow
// DESC: Raises a real native toplevel through focus authority or a private Tk child within its parent.
// ------------------=
int XRaiseWindow(Display *d,Window id){(void)d;NativeWindow *w=lookup_window(id);if(!w)return BadWindow;if(w->native&&infinity_window_focus(w->focus,w->native)<0)return BadAccess;w->order=++stacking;return Success;}
// ------------------------=
// FUNC: InfinityTkSetTitle
// DESC: Updates the real native toplevel title through its exact management grant.
// ------------------=
int InfinityTkSetTitle(Window id,const char *title){NativeWindow *w=top_window(lookup_window(id));if(!w||!title||strlen(title)>96)return 0;
    if(w->native&&infinity_window_title(w->manage,w->native,title,strlen(title))<0)return 0;memcpy(w->title,title,strlen(title)+1);return 1;}
// ------------------------=
// FUNC: InfinityTkSetState
// DESC: Applies actual native normal/minimized/maximized/fullscreen/withdrawn state rather than only changing Tk metadata.
// ------------------=
int InfinityTkSetState(Window id,int state){NativeWindow *w=top_window(lookup_window(id));if(!w||state<0||state>INFINITY_WINDOW_WITHDRAWN||!native_create(w))return 0;return infinity_window_state(w->manage,w->native,state)>=0;}
// ------------------------=
// FUNC: InfinityTkGetState
// DESC: Reads the real shell's committed native window state, preserving an explicit unavailable result for unmapped resources.
// ------------------=
int InfinityTkGetState(Window id){NativeWindow *w=top_window(lookup_window(id));return w&&w->native?(int)infinity_gui_window_geometry(w->native,5):-1;}
// ------------------------=
// FUNC: TkpSync
// DESC: Flushes completed widget pixels and currently queued native events without blocking audio or native scheduling.
// ------------------=
void TkpSync(Display *d){(void)d;pump();}
// ------------------------=
// FUNC: XFlush
// DESC: Publishes real retained Tk frames at the native event boundary.
// ------------------=
int XFlush(Display *d){TkpSync(d);return Success;}
// ------------------------=
// FUNC: XSync
// DESC: Synchronizes local retained drawing; there is no remote X transport or speculative remote acknowledgement.
// ------------------=
int XSync(Display *d,Bool discard){(void)discard;TkpSync(d);return Success;}
// ------------------------=
// FUNC: Tk_GetPixmap
// DESC: Allocates a genuine private native pixmap used by upstream Tk double buffering and images.
// ------------------=
Pixmap Tk_GetPixmap(Display *d,Drawable reference,int width,int height,int bits){(void)d;(void)reference;return width>0&&height>0?InfinityTkAllocate(width,height,bits):None;}
// ------------------------=
// FUNC: Tk_FreePixmap
// DESC: Releases a native pixmap without touching any unrelated window or surface.
// ------------------=
void Tk_FreePixmap(Display *d,Pixmap pixmap){(void)d;if(!lookup_window(pixmap))InfinityTkRelease(pixmap);}
// ------------------------=
// FUNC: XConfigureWindow
// DESC: Applies only the requested checked geometry fields and actual sibling stacking operation.
// ------------------=
int XConfigureWindow(Display *d,Window id,unsigned mask,XWindowChanges *changes){NativeWindow *w=lookup_window(id);if(!w||!changes)return BadWindow;
    int result=XMoveResizeWindow(d,id,mask&CWX?changes->x:w->x,mask&CWY?changes->y:w->y,mask&CWWidth?changes->width:w->width,mask&CWHeight?changes->height:w->height);
    if(result!=Success)return result;if(mask&CWStackMode)return changes->stack_mode==Above?XRaiseWindow(d,id):XLowerWindow(d,id);return Success;}
// ------------------------=
// FUNC: XLowerWindow
// DESC: Lowers an owned child within its real parent; native toplevel lowering is explicitly unavailable until the Window Server exposes that operation.
// ------------------=
int XLowerWindow(Display *d,Window id){(void)d;NativeWindow *w=lookup_window(id);if(!w)return BadWindow;if(w->native)return BadAccess;
    for(unsigned i=0;i<WINDOW_LIMIT;++i)if(windows[i].id&&windows[i].parent==w->parent)++windows[i].order;
    ++stacking;w->order=1;return Success;}
// ------------------------=
// FUNC: root_position
// DESC: Computes checked application-local root coordinates through the actual retained child hierarchy.
// ------------------=
static int root_position(Window id,int *x,int *y){*x=*y=0;if(id==1)return 1;NativeWindow *w=lookup_window(id);for(unsigned n=0;w&&n<WINDOW_LIMIT;++n){*x+=w->x;*y+=w->y;if(w->parent==1)return 1;w=lookup_window(w->parent);}return 0;}
// ------------------------=
// FUNC: XTranslateCoordinates
// DESC: Translates between actual owned windows without inspecting other processes or exposing desktop pixel memory.
// ------------------=
Bool XTranslateCoordinates(Display *d,Window source,Window destination,int sx,int sy,int *dx,int *dy,Window *child){(void)d;int ax,ay,bx,by;if(!root_position(source,&ax,&ay)||!root_position(destination,&bx,&by))return False;
    *dx=sx+ax-bx;*dy=sy+ay-by;if(child)*child=None;return True;}
// ------------------------=
// FUNC: XGetGeometry
// DESC: Reports the actual private drawable dimensions and owned window position.
// ------------------=
Status XGetGeometry(Display *d,Drawable id,Window *root,int *x,int *y,unsigned *width,unsigned *height,unsigned *border,unsigned *bits){
    (void)d;NativeWindow *w=lookup_window(id);InfinityTkPixels *p=InfinityTkDrawable(id);if(id!=1&&!p)return 0;
    *root=1;*x=w?w->x:0;*y=w?w->y:0;*width=id==1?screen.width:p->width;*height=id==1?screen.height:p->height;*border=0;*bits=id==1?24:p->depth;return 1;}
// ------------------------=
// FUNC: XGetWindowAttributes
// DESC: Exposes checked metadata for the actual root or caller-owned Tk window.
// ------------------=
Status XGetWindowAttributes(Display *d,Window id,XWindowAttributes *attributes){NativeWindow *w=lookup_window(id);if(id!=1&&!w)return 0;memset(attributes,0,sizeof(*attributes));
    attributes->x=w?w->x:0;attributes->y=w?w->y:0;attributes->width=w?(int)w->width:screen.width;attributes->height=w?(int)w->height:screen.height;
    attributes->depth=24;attributes->visual=&visual;attributes->root=1;attributes->c_class=InputOutput;attributes->colormap=1;attributes->map_installed=True;attributes->map_state=!w||w->mapped?IsViewable:IsUnmapped;attributes->screen=&screen;(void)d;return 1;}
// ------------------------=
// FUNC: XSetWindowBackground
// DESC: Updates the actual clear color of one owned Tk drawable.
// ------------------=
int XSetWindowBackground(Display *d,Window id,unsigned long pixel){(void)d;NativeWindow *w=lookup_window(id);if(!w)return BadWindow;w->background=pixel;w->background_pixmap=None;return Success;}
// ------------------------=
// FUNC: XClearWindow
// DESC: Clears real retained pixels using the selected background color and invalidates presentation.
// ------------------=
int XClearWindow(Display *d,Window id){(void)d;NativeWindow *w=lookup_window(id);InfinityTkPixels *pixels=InfinityTkDrawable(id);if(!w||!pixels)return BadWindow;
    return XClearArea(d,id,0,0,pixels->width,pixels->height,False);}
// ------------------------=
// FUNC: XClearArea
// DESC: Clears a clipped retained region and optionally queues the actual repaint exposure requested by Tk.
// ------------------=
int XClearArea(Display *d,Window id,int x,int y,unsigned width,unsigned height,Bool exposures){NativeWindow *w=lookup_window(id);InfinityTkPixels *p=InfinityTkDrawable(id);if(!w||!p)return BadWindow;
    if(width>8192||height>8192)return BadValue;
    int right=width?x+(int)width:(int)p->width,bottom=height?y+(int)height:(int)p->height;if(right>(int)p->width)right=p->width;if(bottom>(int)p->height)bottom=p->height;
    InfinityTkPixels *tile=InfinityTkDrawable(w->background_pixmap);
    if(tile||w->background!=~0UL)for(int yy=y<0?0:y;yy<bottom;++yy)for(int xx=x<0?0:x;xx<right;++xx){
        uint32_t color=tile?tile->pixels[(size_t)(yy%tile->height)*tile->width+xx%tile->width]:w->background;
        p->pixels[(size_t)yy*p->width+xx]=color|0xff000000u;}
    InfinityTkDamage(id);if(exposures)queue_event(w,Expose);(void)d;return Success;}
// ------------------------=
// FUNC: XChangeWindowAttributes
// DESC: Applies supported native child-window metadata and rejects unsupported backing resources instead of silently accepting them.
// ------------------=
int XChangeWindowAttributes(Display *d,Window id,unsigned long mask,XSetWindowAttributes *attributes){
    NativeWindow *w=lookup_window(id);if(!w)return BadWindow;if((mask&CWBackPixmap)&&attributes->background_pixmap!=None)return BadPixmap;
    if(mask&CWBackPixel)w->background=attributes->background_pixel;if(mask&CWCursor)return XDefineCursor(d,id,attributes->cursor);return Success;}
// ------------------------=
// FUNC: XSetInputFocus
// DESC: Requests native focus for the exact owning toplevel and delivers a real Tk focus transition to its child.
// ------------------=
int XSetInputFocus(Display *d,Window id,int revert,Time time){(void)d;(void)revert;(void)time;NativeWindow *w=lookup_window(id),*top=top_window(w);if(!w||!top||!native_create(top))return BadWindow;
    if(infinity_window_focus(top->focus,top->native)<0)return BadAccess;
    Window old=keyboard_window;keyboard_window=id;XEvent event;memset(&event,0,sizeof(event));event.xfocus.display=&display;event.xfocus.mode=NotifyNormal;event.xfocus.detail=NotifyNonlinear;
    if(old&&old!=id){event.xfocus.type=FocusOut;event.xfocus.window=old;Tk_QueueWindowEvent(&event,TCL_QUEUE_TAIL);}event.xfocus.type=FocusIn;event.xfocus.window=id;Tk_QueueWindowEvent(&event,TCL_QUEUE_TAIL);return Success;}
// ------------------------=
// FUNC: XGetInputFocus
// DESC: Reports the actual focus selected within this native Tk context.
// ------------------=
int XGetInputFocus(Display *d,Window *window,int *revert){(void)d;*window=keyboard_window;*revert=RevertToParent;return Success;}
// ------------------------=
// FUNC: TkpChangeFocus
// DESC: Delegates Tk focus changes to the capability-checked native focus path and returns the event serial only on success.
// ------------------=
int TkpChangeFocus(TkWindow *w,int force){(void)force;Tk_MakeWindowExist((Tk_Window)w);return XSetInputFocus(w->display,w->window,RevertToParent,CurrentTime)==Success?(int)++display.request:0;}
// ------------------------=
// FUNC: TkpClaimFocus
// DESC: Claims only the requesting application's real native toplevel focus.
// ------------------=
void TkpClaimFocus(TkWindow *w,int force){(void)TkpChangeFocus(w,force);}
// ------------------------=
// FUNC: XQueryPointer
// DESC: Returns the latest trusted native pointer position relative to the requested owned window.
// ------------------=
Bool XQueryPointer(Display *d,Window id,Window *root,Window *child,int *rx,int *ry,int *x,int *y,unsigned *state){(void)d;int ox,oy;if(!root_position(id,&ox,&oy))return False;
    *root=1;*child=hover_window;*rx=pointer_x;*ry=pointer_y;*x=pointer_x-ox;*y=pointer_y-oy;*state=pointer_state;return True;}
// ------------------------=
// FUNC: XGrabPointer
// DESC: Captures pointer input only using the owning native toplevel's exact capability.
// ------------------=
int XGrabPointer(Display *d,Window id,Bool events,unsigned mask,int pmode,int kmode,Window confine,Cursor cursor,Time time){
    (void)d;(void)events;(void)mask;(void)pmode;(void)kmode;(void)cursor;(void)time;if(confine!=None)return GrabNotViewable;
    NativeWindow *w=lookup_window(id),*top=top_window(w);if(!w||!top||!native_create(top))return GrabNotViewable;
    if(infinity_window_capture(top->capture,top->native)<0)return AlreadyGrabbed;captured_window=id;return GrabSuccess;}
// ------------------------=
// FUNC: XUngrabPointer
// DESC: Releases only this application's native pointer capture.
// ------------------=
int XUngrabPointer(Display *d,Time time){(void)d;(void)time;NativeWindow *top=top_window(lookup_window(captured_window));if(top&&top->native)infinity_window_release(top->capture,top->native);captured_window=0;return Success;}
// ------------------------=
// FUNC: XDefineCursor
// DESC: Stores a supported native pointer shape for a real Tk child; cursor application remains checked by the Window Server.
// ------------------=
int XDefineCursor(Display *d,Window id,Cursor cursor){(void)d;NativeWindow *w=lookup_window(id);if(!w||cursor>5)return BadCursor;w->cursor=cursor;
    NativeWindow *top=top_window(w);if(hover_window==id&&top&&top->native&&infinity_system_call(0x1254,top->native,cursor,0,0,0,0)<0)return BadAccess;return Success;}
// ------------------------=
// FUNC: XRootWindow
// DESC: Returns the native display metadata root, not a global framebuffer drawable.
// ------------------=
Window XRootWindow(Display *d,int index){return d==&display&&index==0?1:None;}
// ------------------------=
// FUNC: XQueryTree
// DESC: Returns the actual retained child hierarchy in bottom-to-top order, never another process's windows.
// ------------------=
Status XQueryTree(Display *d,Window id,Window *root,Window *parent,Window **children,unsigned *count){
    (void)d;NativeWindow *w=lookup_window(id);if(id!=1&&!w)return 0;*root=1;*parent=w?w->parent:None;*count=0;
    for(unsigned i=0;i<WINDOW_LIMIT;++i)if(windows[i].id&&windows[i].parent==id)++*count;
    *children=(Window *)Tcl_AttemptAlloc((*count?*count:1)*sizeof(Window));if(!*children){*count=0;return 0;}
    uint64_t previous=0;for(unsigned n=0;n<*count;++n){NativeWindow *next=NULL;for(unsigned i=0;i<WINDOW_LIMIT;++i)
        if(windows[i].id&&windows[i].parent==id&&windows[i].order>previous&&(!next||windows[i].order<next->order))next=&windows[i];
        if(!next){ckfree(*children);*children=NULL;*count=0;return 0;}(*children)[n]=next->id;previous=next->order;}return 1;
}
// ------------------------=
// FUNC: XGrabKeyboard
// DESC: Restricts key delivery to one actual widget inside the current native application's authorized focused toplevel.
// ------------------=
int XGrabKeyboard(Display *d,Window id,Bool owner,int pointer_mode,int keyboard_mode,Time time){
    (void)owner;if(pointer_mode!=GrabModeAsync||keyboard_mode!=GrabModeAsync)return GrabFrozen;
    if(keyboard_grab&&keyboard_grab!=id)return AlreadyGrabbed;
    if(XSetInputFocus(d,id,RevertToParent,time)!=Success)return GrabNotViewable;keyboard_grab=id;return GrabSuccess;
}
// ------------------------=
// FUNC: XUngrabKeyboard
// DESC: Releases only the private Tk keyboard routing override, without taking focus from another application.
// ------------------=
int XUngrabKeyboard(Display *d,Time time){(void)d;(void)time;keyboard_grab=None;return Success;}
// ------------------------=
// FUNC: TkpMakeTransparentWindowExist
// DESC: Creates a real input-only busy shield that participates in hit testing but never overwrites underlying pixels.
// ------------------=
void TkpMakeTransparentWindowExist(Tk_Window window,Window parent){TkWindow *tk=(TkWindow *)window;
    tk->changes.border_width=0;tk->window=TkpMakeWindow(tk,parent);NativeWindow *w=lookup_window(tk->window);if(w)w->input_only=1;}
// ------------------------=
// FUNC: TkpCreateBusy
// DESC: Associates an input-only busy shield with its actual native parent hierarchy.
// ------------------=
void TkpCreateBusy(Tk_FakeWin *window,Tk_Window reference,Window *parent,Tk_Window tk_parent,TkBusy busy){
    (void)window;(void)reference;(void)busy;Tk_MakeWindowExist(tk_parent);*parent=Tk_WindowId(tk_parent);}
// ------------------------=
// FUNC: TkpShowBusyWindow
// DESC: Maps and raises the real transparent input shield above its sibling widgets.
// ------------------=
void TkpShowBusyWindow(TkBusy token){Busy *busy=(Busy *)token;if(busy->tkBusy){Tk_MapWindow(busy->tkBusy);XRaiseWindow(Tk_Display(busy->tkBusy),Tk_WindowId(busy->tkBusy));}}
// ------------------------=
// FUNC: TkpHideBusyWindow
// DESC: Removes the busy shield from hit testing while preserving its reusable widget identity.
// ------------------=
void TkpHideBusyWindow(TkBusy token){Busy *busy=(Busy *)token;if(busy->tkBusy)Tk_UnmapWindow(busy->tkBusy);}
// ------------------------=
// FUNC: XSetWindowBorderWidth
// DESC: Records checked client border geometry for retained composition, keeping it separate from native shell chrome.
// ------------------=
int XSetWindowBorderWidth(Display *d,Window id,unsigned width){(void)d;NativeWindow *w=lookup_window(id);if(!w)return BadWindow;if(width>256)return BadValue;w->border_width=width;InfinityTkDamage(id);return Success;}
// ------------------------=
// FUNC: XSetWindowBorder
// DESC: Updates the actual retained client border color and invalidates its visible composition.
// ------------------=
int XSetWindowBorder(Display *d,Window id,unsigned long pixel){(void)d;NativeWindow *w=lookup_window(id);if(!w)return BadWindow;w->border_color=pixel;w->border_pixmap=None;InfinityTkDamage(id);return Success;}
// ------------------------=
// FUNC: XSetWindowBorderPixmap
// DESC: Selects a real private tiled border pixmap; unimplemented inherited pixmaps are rejected explicitly.
// ------------------=
int XSetWindowBorderPixmap(Display *d,Window id,Pixmap pixmap){(void)d;NativeWindow *w=lookup_window(id);if(!w)return BadWindow;if(pixmap&&!InfinityTkDrawable(pixmap))return BadPixmap;w->border_pixmap=pixmap;InfinityTkDamage(id);return Success;}
// ------------------------=
// FUNC: XSetWindowBackgroundPixmap
// DESC: Selects actual private background tile pixels or disables clear painting for None, without inventing foreign drawable access.
// ------------------=
int XSetWindowBackgroundPixmap(Display *d,Window id,Pixmap pixmap){(void)d;NativeWindow *w=lookup_window(id);if(!w)return BadWindow;if(pixmap&&!InfinityTkDrawable(pixmap))return BadPixmap;w->background_pixmap=pixmap;w->background=~0UL;return Success;}
// ------------------------=
// FUNC: XSetWindowColormap
// DESC: Accepts only the immutable true-color visual already installed on this real native display.
// ------------------=
int XSetWindowColormap(Display *d,Window id,Colormap map){(void)d;return lookup_window(id)?(map==1?Success:BadColor):BadWindow;}
// ------------------------=
// FUNC: synchronous_after
// DESC: Flushes genuine retained frame mutations when upstream explicitly enables synchronous display operations.
// ------------------=
static int synchronous_after(Display *d){TkpSync(d);return Success;}
// ------------------------=
// FUNC: XSynchronize
// DESC: Stores a real optional post-operation flush hook for the local synchronous backend, returning the prior hook.
// ------------------=
XAfterFunction XSynchronize(Display *d,Bool enabled){XAfterFunction previous=synchronous_hook;synchronous_hook=enabled?synchronous_after:NULL;
    if(enabled)synchronous_after(d);return previous;}
// ------------------------=
// FUNC: TkCreateXEventSource
// DESC: Connects Tk's existing event-source entrypoint to the genuine native IEF pump rather than creating a second scheduler.
// ------------------=
void TkCreateXEventSource(void){InfinityTclSetEventPump(pump);}
// ------------------------=
// FUNC: TkUnixDoOneXEvent
// DESC: Services one real pending native/Tk window event, yielding through Tcl's native notifier when no event is yet ready.
// ------------------=
int TkUnixDoOneXEvent(Tcl_Time *deadline){
    pump();if(Tcl_ServiceEvent(TCL_WINDOW_EVENTS))return 1;
    Tcl_Time wait={0,1000};if(deadline){Tcl_Time now;Tcl_GetTime(&now);long long us=(deadline->sec-now.sec)*1000000LL+deadline->usec-now.usec;if(us<=0)return 0;if(us<wait.usec)wait.usec=us;}
    Tcl_WaitForEvent(&wait);pump();return Tcl_ServiceEvent(TCL_WINDOW_EVENTS);
}
// ------------------------=
// FUNC: Tk_UpdatePointer
// DESC: Updates application-local pointer metadata from a toolkit event without injecting or warping the OS device pointer.
// ------------------=
void Tk_UpdatePointer(Tk_Window window,int x,int y,int state){if(window&&!lookup_window(Tk_WindowId(window)))return;pointer_x=x;pointer_y=y;pointer_state=state;}
