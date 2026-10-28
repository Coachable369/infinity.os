#include "native.h"
#include "tkFont.h"

typedef struct NativeFont {
    TkFont font;
    unsigned height, style;
} NativeFont;
typedef struct GlyphCache {
    unsigned scalar,height,style;
    uint64_t used;
    InfinityFontMetrics metrics;
    unsigned char coverage[96*96];
} GlyphCache;
static GlyphCache glyph_cache[64];
static uint64_t glyph_clock;

// ------------------------=
// FUNC: glyph
// DESC: Obtains glyph coverage and metrics from the installed native FontKit atlas, using an explicit missing-glyph fallback for unavailable scalar coverage.
// ------------------=
static int glyph(const NativeFont *font, unsigned scalar, InfinityFontMetrics *metrics, unsigned char *coverage) {
    if(scalar<32||scalar>126)scalar='?';
    GlyphCache *slot=&glyph_cache[0];
    for(unsigned i=0;i<64;++i){GlyphCache *entry=&glyph_cache[i];
        if(entry->used&&entry->scalar==scalar&&entry->height==font->height&&entry->style==font->style){
            *metrics=entry->metrics;memcpy(coverage,entry->coverage,metrics->bytes_written);entry->used=++glyph_clock;return 1;}
        if(entry->used<slot->used)slot=entry;
    }
    InfinityFontRequest request={1,scalar,font->height,font->style};
    if(infinity_font_glyph(&request,metrics,coverage,96*96)<0||metrics->bytes_written<0||metrics->bytes_written>96*96)return 0;
    slot->scalar=scalar;slot->height=font->height;slot->style=font->style;slot->metrics=*metrics;
    memcpy(slot->coverage,coverage,metrics->bytes_written);slot->used=++glyph_clock;return 1;
}
// ------------------------=
// FUNC: TkpFontPkgInit
// DESC: Uses installed process-independent FontKit resources; no host font cache or X server connection is created.
// ------------------=
void TkpFontPkgInit(TkMainInfo *main) { (void)main; }
// ------------------------=
// FUNC: TkpGetFontFromAttributes
// DESC: Maps Tk attributes to actual installed Inter/JetBrainsMono faces and reports the resulting metrics and unsupported-style substitution honestly.
// ------------------=
TkFont *TkpGetFontFromAttributes(TkFont *old, Tk_Window window, const TkFontAttributes *wanted) {
    NativeFont *font=(NativeFont *)old;
    unsigned style=0;
    const char *family=wanted->family ? wanted->family : "Inter";
    if (!strcmp(family,"JetBrains Mono") || !strcmp(family,"JetBrainsMono") ||
        !strcmp(family,"monospace") || !strcmp(family,"Courier") || !strcmp(family,"TkFixedFont")) style|=INFINITY_FONT_MONO;
    if (wanted->weight==TK_FW_BOLD) style|=INFINITY_FONT_BOLD;
    double pixels=wanted->size<0 ? -wanted->size : wanted->size==0 ? 18 : wanted->size*96.0/72.0;
    unsigned height=(unsigned)(pixels+0.5); if(height<8)height=8;if(height>96)height=96;
    NativeFont candidate={0};candidate.height=height;candidate.style=style;
    InfinityFontMetrics metrics;unsigned char coverage[96*96];
    if(!glyph(&candidate,'M',&metrics,coverage))return NULL;
    if(!font){font=ckalloc(sizeof(*font));memset(font,0,sizeof(*font));}
    font->height=height;font->style=style;
    font->font.screen=Tk_Screen(window);
    font->font.fid=(Font)(uintptr_t)font;
    font->font.fa=*wanted;
    font->font.fa.family=Tk_GetUid(style&INFINITY_FONT_MONO?"JetBrains Mono":"Inter");
    font->font.fa.size=height*72.0/96.0;
    font->font.fa.slant=TK_FS_ROMAN;
    font->font.fm.ascent=metrics.ascent;font->font.fm.descent=metrics.descent;
    font->font.fm.maxWidth=metrics.width;font->font.fm.fixed=(style&INFINITY_FONT_MONO)!=0;
    font->font.underlinePos=1;font->font.underlineHeight=height>=32?2:1;
    glyph(font,'0',&metrics,coverage);font->font.tabWidth=metrics.advance_x*8;
    return &font->font;
}
// ------------------------=
// FUNC: TkpGetNativeFont
// DESC: Recognizes only actual installed native face names; ordinary Tk font syntax remains handled by the upstream parser.
// ------------------=
TkFont *TkpGetNativeFont(Tk_Window window,const char *name) {
    if(strcmp(name,"Inter")&&strcmp(name,"JetBrains Mono"))return NULL;
    TkFontAttributes wanted={Tk_GetUid(name),13.5,TK_FW_NORMAL,TK_FS_ROMAN,0,0};
    return TkpGetFontFromAttributes(NULL,window,&wanted);
}
// ------------------------=
// FUNC: TkpDeleteFont
// DESC: Releases no kernel resources because glyphs are immutable copied data; upstream Tk owns the font allocation.
// ------------------=
void TkpDeleteFont(TkFont *font) {(void)font;}
// ------------------------=
// FUNC: TkpGetFontFamilies
// DESC: Lists the actual default-installed native font families rather than host fonts or unsupported substitutes.
// ------------------=
void TkpGetFontFamilies(Tcl_Interp *interp,Tk_Window window) {
    (void)window;Tcl_Obj *names[2]={Tcl_NewStringObj("Inter",-1),Tcl_NewStringObj("JetBrains Mono",-1)};
    Tcl_SetObjResult(interp,Tcl_NewListObj(2,names));
}
// ------------------------=
// FUNC: TkpGetSubFonts
// DESC: Reports the actual selected native face and its currently installed ASCII coverage encoding.
// ------------------=
void TkpGetSubFonts(Tcl_Interp *interp,Tk_Font font) {
    Tcl_Obj *values[3]={Tcl_NewStringObj(((NativeFont *)font)->font.fa.family,-1),Tcl_NewStringObj("Infinity FontKit",-1),Tcl_NewStringObj("ascii",-1)};
    Tcl_Obj *record=Tcl_NewListObj(3,values);Tcl_SetObjResult(interp,Tcl_NewListObj(1,&record));
}
// ------------------------=
// FUNC: TkpGetFontAttrsForChar
// DESC: Returns selected native face attributes; unsupported Unicode renders the same face's explicit replacement glyph.
// ------------------=
void TkpGetFontAttrsForChar(Tk_Window window,Tk_Font font,int scalar,TkFontAttributes *attributes) {
    (void)window;(void)scalar;*attributes=((NativeFont *)font)->font.fa;
}
// ------------------------=
// FUNC: Tk_MeasureChars
// DESC: Measures complete UTF-8 characters with actual FontKit advances and honors Tk partial/whole-word bounds.
// ------------------=
int Tk_MeasureChars(Tk_Font token,const char *source,int length,int maximum,int flags,int *width) {
    NativeFont *font=(NativeFont *)token;int offset=0,total=0,word_end=0,word_width=0;
    unsigned char coverage[96*96];InfinityFontMetrics metrics;
    while(offset<length) {
        Tcl_UniChar scalar;int count=Tcl_UtfToUniChar(source+offset,&scalar);
        if(count<=0||count>length-offset)break;
        if(!glyph(font,scalar,&metrics,coverage))break;
        int next=total+metrics.advance_x;
        if(maximum>=0&&next>maximum) {
            if((flags&TK_PARTIAL_OK)||(!offset&&(flags&TK_AT_LEAST_ONE))){offset+=count;total=next;}
            break;
        }
        if(Tcl_UniCharIsSpace(scalar)){word_end=offset;word_width=total;}
        offset+=count;total=next;
    }
    if(offset<length&&(flags&TK_WHOLE_WORDS)&&word_end){offset=word_end;total=word_width;}
    *width=total;return offset;
}
// ------------------------=
// FUNC: TkpMeasureCharsInContext
// DESC: Measures a checked UTF-8 range using the same native metrics as drawing.
// ------------------=
int TkpMeasureCharsInContext(Tk_Font font,const char *text,int total,int start,int length,int maximum,int flags,int *width) {
    if(start<0||length<0||start>total||length>total-start){*width=0;return 0;}
    return Tk_MeasureChars(font,text+start,length,maximum,flags,width);
}
// ------------------------=
// FUNC: draw_text
// DESC: Blends FontKit antialiased glyph coverage into an owned Tk drawable, including rotated canvas text without host rendering.
// ------------------=
static void draw_text(Drawable drawable,GC gc,NativeFont *font,const char *text,int length,double x,double y,double angle) {
    const double radians=angle*3.14159265358979323846/180.0;
    double cosine=cos(radians),sine=sin(radians);int offset=0,advance=0;
    unsigned char coverage[96*96];InfinityFontMetrics metrics;
    while(offset<length){
        Tcl_UniChar scalar;int count=Tcl_UtfToUniChar(text+offset,&scalar);if(count<=0||count>length-offset)break;
        if(!glyph(font,scalar,&metrics,coverage))break;
        for(int row=0;row<metrics.height;++row)for(int col=0;col<metrics.width;++col){
            unsigned alpha=coverage[row*metrics.width+col];if(!alpha)continue;
            double dx=advance+metrics.bearing_x+col,dy=row-metrics.bearing_y;
            InfinityTkBlend(drawable,gc,(int)floor(x+cosine*dx+sine*dy+0.5),(int)floor(y-sine*dx+cosine*dy+0.5),(uint32_t)gc->foreground,alpha);
        }
        advance+=metrics.advance_x;offset+=count;
    }
    InfinityTkDamage(drawable);
}
// ------------------------=
// FUNC: Tk_DrawChars
// DESC: Renders unrotated upstream Tk text through installed native FontKit glyph coverage.
// ------------------=
void Tk_DrawChars(Display *display,Drawable drawable,GC gc,Tk_Font font,const char *text,int length,int x,int y) {
    (void)display;draw_text(drawable,gc,(NativeFont *)font,text,length,x,y,0);
}
// ------------------------=
// FUNC: TkDrawAngledChars
// DESC: Renders rotated native glyphs for the actual upstream canvas text item.
// ------------------=
void TkDrawAngledChars(Display *display,Drawable drawable,GC gc,Tk_Font font,const char *text,int length,double x,double y,double angle) {
    (void)display;draw_text(drawable,gc,(NativeFont *)font,text,length,x,y,angle);
}
// ------------------------=
// FUNC: TkpDrawCharsInContext
// DESC: Positions a checked substring using its preceding real native glyph advances.
// ------------------=
void TkpDrawCharsInContext(Display *display,Drawable drawable,GC gc,Tk_Font font,const char *text,int total,int start,int length,int x,int y) {
    if(start<0||length<0||start>total||length>total-start)return;
    int preceding;Tk_MeasureChars(font,text,start,-1,0,&preceding);
    Tk_DrawChars(display,drawable,gc,font,text+start,length,x+preceding,y);
}
// ------------------------=
// FUNC: TkpDrawAngledCharsInContext
// DESC: Applies the native substring advance consistently to rotated canvas text.
// ------------------=
void TkpDrawAngledCharsInContext(Display *display,Drawable drawable,GC gc,Tk_Font font,const char *text,int total,int start,int length,double x,double y,double angle) {
    if(start<0||length<0||start>total||length>total-start)return;
    int preceding;Tk_MeasureChars(font,text,start,-1,0,&preceding);
    double radians=angle*3.14159265358979323846/180.0;
    TkDrawAngledChars(display,drawable,gc,font,text+start,length,x+cos(radians)*preceding,y-sin(radians)*preceding,angle);
}
