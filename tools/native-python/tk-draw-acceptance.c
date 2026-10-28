#include "native.h"

// ------------------------=
// FUNC: rgb
// DESC: Reads the actual private drawable color independently of alpha packing.
// ------------------=
static uint32_t rgb(InfinityTkPixels *image,unsigned x,unsigned y){return image->pixels[(size_t)y*image->width+x]&0xffffffu;}
// ------------------------=
// FUNC: clear
// DESC: Resets the owned test drawable while preserving its geometry and allocation identity.
// ------------------=
static void clear(InfinityTkPixels *image){memset(image->pixels,0,(size_t)image->width*image->height*4);}
// ------------------------=
// FUNC: InfinityTkDrawAcceptance
// DESC: Verifies ellipse coverage, antialiasing, positive/negative slices, chords, region clipping and point raster operations using actual native pixels.
// ------------------=
int InfinityTkDrawAcceptance(void){
    Drawable id=InfinityTkAllocate(32,32,24);if(!id)return 1;
    InfinityTkPixels *pixels=InfinityTkDrawable(id);XGCValues gc;memset(&gc,0,sizeof(gc));
    gc.function=GXcopy;gc.plane_mask=0xffffff;gc.foreground=0xffffff;gc.line_width=2;gc.arc_mode=ArcPieSlice;
    uint64_t revision=InfinityTkRevision(id);
    if(XFillArc(NULL,id,&gc,4,8,24,16,0,360*64)!=Success||InfinityTkRevision(id)<=revision)return 2;
    if(rgb(pixels,16,16)!=0xffffff||rgb(pixels,4,8)||rgb(pixels,0,0))return 3;
    unsigned filled=0,soft=0;for(unsigned y=0;y<32;++y)for(unsigned x=0;x<32;++x){uint32_t p=rgb(pixels,x,y);if(p)++filled;if(p&&p!=0xffffff)++soft;}
    if(filled<280||filled>350||soft<12)return 4;
    clear(pixels);if(XDrawArc(NULL,id,&gc,4,8,24,16,0,360*64)!=Success)return 5;
    if(rgb(pixels,16,16)||!rgb(pixels,4,16)||rgb(pixels,1,16))return 6;
    clear(pixels);if(XFillArc(NULL,id,&gc,4,8,24,16,0,90*64)!=Success)return 7;
    if(rgb(pixels,22,12)!=0xffffff||rgb(pixels,10,12)||rgb(pixels,22,20))return 8;
    clear(pixels);if(XFillArc(NULL,id,&gc,4,8,24,16,0,-90*64)!=Success)return 9;
    if(rgb(pixels,22,20)!=0xffffff||rgb(pixels,22,12)||rgb(pixels,10,20))return 10;
    clear(pixels);gc.arc_mode=ArcChord;
    if(XFillArc(NULL,id,&gc,4,8,24,16,0,90*64)!=Success)return 11;
    if(rgb(pixels,18,14)||rgb(pixels,22,11)!=0xffffff)return 12;
    Region region=XCreateRegion();XRectangle rectangle={0,0,16,32};XUnionRectWithRegion(&rectangle,region,region);
    TkpClipMask mask;memset(&mask,0,sizeof(mask));mask.type=TKP_CLIP_REGION;mask.value.region=(TkRegion)region;gc.clip_mask=(Pixmap)&mask;
    clear(pixels);if(XFillArc(NULL,id,&gc,4,8,24,16,0,360*64)!=Success)return 13;
    if(rgb(pixels,10,16)!=0xffffff||rgb(pixels,22,16))return 14;
    for(unsigned y=0;y<32;++y)for(unsigned x=16;x<32;++x)if(rgb(pixels,x,y))return 15;
    gc.clip_mask=None;XDestroyRegion(region);clear(pixels);gc.function=GXxor;
    if(XDrawPoint(NULL,id,&gc,3,4)!=Success||rgb(pixels,3,4)!=0xffffff)return 16;
    XDrawPoint(NULL,id,&gc,3,4);if(rgb(pixels,3,4))return 17;
    XDrawPoint(NULL,id,&gc,INT_MIN,INT_MAX);if(rgb(pixels,0,0))return 18;
    gc.function=GXcopy;XPoint points[3]={{2,2},{1,0},{0,1}};
    if(XDrawPoints(NULL,id,&gc,points,3,CoordModePrevious)!=Success||rgb(pixels,2,2)!=0xffffff||rgb(pixels,3,2)!=0xffffff||rgb(pixels,3,3)!=0xffffff)return 19;
    if(XDrawArc(NULL,id,&gc,0,0,8193,1,0,64)!=BadValue||XDrawPoint(NULL,None,&gc,0,0)!=BadDrawable)return 20;
    clear(pixels);gc.arc_mode=ArcPieSlice;
    if(XFillArc(NULL,id,&gc,-8,-8,24,24,0,360*64)!=Success||rgb(pixels,4,4)!=0xffffff||rgb(pixels,20,20))return 21;
    clear(pixels);
    if(XFillArc(NULL,id,&gc,INT_MIN,INT_MIN,24,24,0,360*64)!=Success||XFillArc(NULL,id,&gc,INT_MAX,INT_MAX,24,24,0,360*64)!=Success)return 22;
    for(unsigned y=0;y<32;++y)for(unsigned x=0;x<32;++x)if(rgb(pixels,x,y))return 23;
    InfinityTkRelease(id);return 0;
}
