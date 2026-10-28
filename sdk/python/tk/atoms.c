#include "native.h"

#define MAX_NATIVE_ATOMS 512
static const char *predefined[] = {NULL,"PRIMARY","SECONDARY","ARC","ATOM","BITMAP","CARDINAL","COLORMAP","CURSOR",
    "CUT_BUFFER0","CUT_BUFFER1","CUT_BUFFER2","CUT_BUFFER3","CUT_BUFFER4","CUT_BUFFER5","CUT_BUFFER6","CUT_BUFFER7",
    "DRAWABLE","FONT","INTEGER","PIXMAP","POINT","RECTANGLE","RESOURCE_MANAGER","RGB_COLOR_MAP","RGB_BEST_MAP",
    "RGB_BLUE_MAP","RGB_DEFAULT_MAP","RGB_GRAY_MAP","RGB_GREEN_MAP","RGB_RED_MAP","STRING","VISUALID","WINDOW",
    "WM_COMMAND","WM_HINTS","WM_CLIENT_MACHINE","WM_ICON_NAME","WM_ICON_SIZE","WM_NAME","WM_NORMAL_HINTS",
    "WM_SIZE_HINTS","WM_ZOOM_HINTS","MIN_SPACE","NORM_SPACE","MAX_SPACE","END_SPACE","SUPERSCRIPT_X","SUPERSCRIPT_Y",
    "SUBSCRIPT_X","SUBSCRIPT_Y","UNDERLINE_POSITION","UNDERLINE_THICKNESS","STRIKEOUT_ASCENT","STRIKEOUT_DESCENT",
    "ITALIC_ANGLE","X_HEIGHT","QUAD_WIDTH","WEIGHT","POINT_SIZE","RESOLUTION","COPYRIGHT","NOTICE","FONT_NAME",
    "FAMILY_NAME","FULL_NAME","CAP_HEIGHT","WM_CLASS","WM_TRANSIENT_FOR"};
static char *application_atoms[MAX_NATIVE_ATOMS];
static unsigned atom_count;

// ------------------------=
// FUNC: atom_name
// DESC: Resolves a stable process-local atom including the standard predefined identities used by genuine Tk protocols.
// ------------------=
static const char *atom_name(Atom atom){
    if(atom>0&&atom<sizeof(predefined)/sizeof(predefined[0]))return predefined[atom];
    if(atom>XA_LAST_PREDEFINED&&atom-XA_LAST_PREDEFINED-1<atom_count)return application_atoms[atom-XA_LAST_PREDEFINED-1];
    return NULL;
}
// ------------------------=
// FUNC: XInternAtom
// DESC: Interns bounded application-local protocol names without exposing or depending on an external X server.
// ------------------=
Atom XInternAtom(Display *display,const char *name,Bool existing){
    (void)display;if(!name||strlen(name)>1024)return None;
    for(unsigned i=1;i<sizeof(predefined)/sizeof(predefined[0]);++i)if(!strcmp(predefined[i],name))return i;
    for(unsigned i=0;i<atom_count;++i)if(!strcmp(application_atoms[i],name))return XA_LAST_PREDEFINED+1+i;
    if(existing||atom_count==MAX_NATIVE_ATOMS)return None;
    size_t size=strlen(name)+1;char *copy=Tcl_AttemptAlloc(size);if(!copy)return None;memcpy(copy,name,size);
    application_atoms[atom_count++]=copy;return XA_LAST_PREDEFINED+atom_count;
}
// ------------------------=
// FUNC: XGetAtomName
// DESC: Returns an owned exact protocol-name copy using the same allocator as the native XFree implementation.
// ------------------=
char *XGetAtomName(Display *display,Atom atom){
    (void)display;const char *name=atom_name(atom);if(!name)return NULL;size_t size=strlen(name)+1;
    char *copy=Tcl_AttemptAlloc(size);if(copy)memcpy(copy,name,size);return copy;
}
// ------------------------=
// FUNC: XInternAtoms
// DESC: Resolves a bounded array of actual atom names while preserving failures instead of inventing identifiers.
// ------------------=
Status XInternAtoms(Display *display,char **names,int count,Bool existing,Atom *atoms){
    if(count<0||(!names&&count)||(!atoms&&count))return 0;
    for(int i=0;i<count;++i)atoms[i]=XInternAtom(display,names[i],existing);return 1;
}
