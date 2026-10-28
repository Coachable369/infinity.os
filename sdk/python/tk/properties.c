#include "native.h"

#define PROPERTY_LIMIT 256
#define PROPERTY_BYTES (4u*1024u*1024u)
typedef struct NativeProperty { Window window; Atom atom,type; int format; size_t count; unsigned char *data; } NativeProperty;
static NativeProperty properties[PROPERTY_LIMIT];
static size_t property_bytes;
typedef struct NativeSelection { Atom atom; Window owner; Time time; } NativeSelection;
static NativeSelection selections[32];

// ------------------------=
// FUNC: valid_window
// DESC: Accepts only this process's actual Tk windows and its private display metadata root.
// ------------------=
static int valid_window(Display *display,Window window){return window==DefaultRootWindow(display)||Tk_IdToWindow(display,window)!=NULL;}
// ------------------------=
// FUNC: property_find
// DESC: Resolves one exact property owner/name pair without querying another process.
// ------------------=
static NativeProperty *property_find(Window window,Atom atom){for(unsigned i=0;i<PROPERTY_LIMIT;++i)if(properties[i].window==window&&properties[i].atom==atom)return &properties[i];return NULL;}
// ------------------------=
// FUNC: property_free
// DESC: Releases real property storage and accounts its bounded byte budget.
// ------------------=
static void property_free(NativeProperty *property){property_bytes-=property->count*(property->format/8);if(property->data)ckfree(property->data);memset(property,0,sizeof(*property));}
// ------------------------=
// FUNC: property_event
// DESC: Reports a completed application-local property change to upstream Tk event bindings.
// ------------------=
static void property_event(Display *display,Window window,Atom atom,int state){XEvent event;memset(&event,0,sizeof(event));event.xproperty.type=PropertyNotify;
    event.xproperty.display=display;event.xproperty.window=window;event.xproperty.atom=atom;event.xproperty.state=state;event.xproperty.serial=++display->request;
    Tk_QueueWindowEvent(&event,TCL_QUEUE_TAIL);}
// ------------------------=
// FUNC: XChangeProperty
// DESC: Implements checked replacement/append/prepend of actual typed protocol data, including X's 32-bit protocol versus native-long convention.
// ------------------=
int XChangeProperty(Display *display,Window window,Atom atom,Atom type,int format,int mode,const unsigned char *data,int count){
    if(!valid_window(display,window))return BadWindow;
    if(!atom||!type||(format!=8&&format!=16&&format!=32)||count<0||(!data&&count)||mode<PropModeReplace||mode>PropModeAppend)return BadValue;
    NativeProperty *existing=property_find(window,atom),*slot=existing;
    if(existing&&mode!=PropModeReplace&&(existing->type!=type||existing->format!=format))return BadMatch;
    if(!slot){for(unsigned i=0;i<PROPERTY_LIMIT;++i)if(!properties[i].window){slot=&properties[i];break;}if(!slot)return BadAlloc;}
    size_t unit=format/8,old=existing?existing->count*(existing->format/8):0,retain=existing&&mode!=PropModeReplace?old:0;
    if((size_t)count>PROPERTY_BYTES/unit||retain>PROPERTY_BYTES-(size_t)count*unit)return BadAlloc;
    size_t size=retain+(size_t)count*unit;if(property_bytes-old>PROPERTY_BYTES-size)return BadAlloc;
    unsigned char *copy=(unsigned char *)Tcl_AttemptAlloc(size?size:1);if(!copy)return BadAlloc;
    size_t insert=mode==PropModeAppend?retain:0;if(retain)memcpy(copy+(mode==PropModePrepend?(size_t)count*unit:0),existing->data,retain);
    if(format==32){for(int i=0;i<count;++i){uint32_t value=((const unsigned long *)data)[i];memcpy(copy+insert+(size_t)i*4,&value,4);}}
    else if(count)memcpy(copy+insert,data,(size_t)count*unit);
    if(existing)property_free(existing);*slot=(NativeProperty){window,atom,type,format,size/unit,copy};property_bytes+=size;
    property_event(display,window,atom,PropertyNewValue);return Success;
}
// ------------------------=
// FUNC: XDeleteProperty
// DESC: Removes only the specified owned property and emits a real deletion notification.
// ------------------=
int XDeleteProperty(Display *display,Window window,Atom atom){if(!valid_window(display,window))return BadWindow;NativeProperty *property=property_find(window,atom);
    if(property){property_free(property);property_event(display,window,atom,PropertyDelete);}return Success;}
// ------------------------=
// FUNC: XGetWindowProperty
// DESC: Reads an exact bounded typed property slice and deletes only after a complete matching read when requested.
// ------------------=
int XGetWindowProperty(Display *display,Window window,Atom atom,long offset,long length,Bool remove,Atom requested,Atom *type,int *format,unsigned long *count,unsigned long *after,unsigned char **data){
    if(!type||!format||!count||!after||!data)return BadValue;*type=None;*format=0;*count=*after=0;*data=NULL;
    if(!valid_window(display,window))return BadWindow;if(offset<0||length<0||(unsigned long)offset>SIZE_MAX/4||(unsigned long)length>SIZE_MAX/4)return BadValue;
    NativeProperty *property=property_find(window,atom);if(!property)return Success;
    *type=property->type;*format=property->format;size_t unit=property->format/8,total=property->count*unit;
    if(requested!=AnyPropertyType&&requested!=property->type){*after=total;return Success;}
    size_t start=(size_t)offset*4;if(start>total)return BadValue;size_t bytes=total-start;if(bytes>(size_t)length*4)bytes=(size_t)length*4;
    *count=bytes/unit;*after=total-start-bytes;size_t output=property->format==32?*count*sizeof(unsigned long):bytes;
    unsigned char *copy=(unsigned char *)Tcl_AttemptAlloc(output+1);if(!copy)return BadAlloc;
    if(property->format==32){for(size_t i=0;i<*count;++i){uint32_t value;memcpy(&value,property->data+start+i*4,4);((unsigned long *)copy)[i]=value;}}
    else if(bytes)memcpy(copy,property->data+start,bytes);copy[output]=0;*data=copy;
    if(remove&&!*after)XDeleteProperty(display,window,atom);return Success;
}
// ------------------------=
// FUNC: XListProperties
// DESC: Lists only actual property identities belonging to one caller-owned window.
// ------------------=
Atom *XListProperties(Display *display,Window window,int *count){if(!count)return NULL;*count=0;if(!valid_window(display,window))return NULL;
    for(unsigned i=0;i<PROPERTY_LIMIT;++i)if(properties[i].window==window)++*count;
    Atom *result=(Atom *)Tcl_AttemptAlloc((*count?*count:1)*sizeof(Atom));if(!result){*count=0;return NULL;}unsigned next=0;
    for(unsigned i=0;i<PROPERTY_LIMIT;++i)if(properties[i].window==window)result[next++]=properties[i].atom;return result;
}
// ------------------------=
// FUNC: XSetTransientForHint
// DESC: Stores a genuine validated same-process transient hint without incorrectly turning it into native modal input capture.
// ------------------=
int XSetTransientForHint(Display *display,Window window,Window parent){if(window==parent||!Tk_IdToWindow(display,parent)||!Tk_IdToWindow(display,window))return BadWindow;
    unsigned long value=parent;return XChangeProperty(display,window,XA_WM_TRANSIENT_FOR,XA_WINDOW,32,PropModeReplace,(const unsigned char *)&value,1);}
// ------------------------=
// FUNC: InfinityTkReleaseProperties
// DESC: Reclaims all property buffers when the actual owning window is destroyed.
// ------------------=
void InfinityTkReleaseProperties(Window window){for(unsigned i=0;i<PROPERTY_LIMIT;++i)if(properties[i].window==window)property_free(&properties[i]);
    for(unsigned i=0;i<32;++i)if(selections[i].owner==window)memset(&selections[i],0,sizeof(selections[i]));}
// ------------------------=
// FUNC: XSetSelectionOwner
// DESC: Maintains actual same-process selection ownership and loss notifications while leaving external clipboard access explicitly unsupported.
// ------------------=
int XSetSelectionOwner(Display *display,Atom atom,Window owner,Time time){
    if(!atom||(owner&&!valid_window(display,owner)))return BadWindow;NativeSelection *slot=NULL;
    for(unsigned i=0;i<32;++i){if(selections[i].atom==atom){slot=&selections[i];break;}if(!selections[i].atom&&!slot)slot=&selections[i];}
    if(!slot)return BadAlloc;if(time&&slot->time&&time<slot->time)return Success;
    if(slot->owner&&slot->owner!=owner){XEvent event;memset(&event,0,sizeof(event));event.xselectionclear.type=SelectionClear;
        event.xselectionclear.display=display;event.xselectionclear.window=slot->owner;event.xselectionclear.selection=atom;event.xselectionclear.time=time;
        Tk_QueueWindowEvent(&event,TCL_QUEUE_TAIL);}
    *slot=(NativeSelection){atom,owner,time};return Success;
}
