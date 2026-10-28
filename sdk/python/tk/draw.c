#include "native.h"

#define MAX_DRAWABLES 256
#define PIXEL_BUDGET (64u*1024u*1024u)
typedef struct NativeDrawable {
    Drawable id;
    InfinityTkPixels image;
    uint64_t revision;
} NativeDrawable;
static NativeDrawable drawables[MAX_DRAWABLES];
static Drawable next_drawable=2;
static size_t pixel_bytes;

// ------------------------=
// FUNC: lookup
// DESC: Resolves monotonic process-private drawable identifiers without permitting stale slot reuse.
// ------------------=
static NativeDrawable *lookup(Drawable id) {
    if(!id)return NULL;
    for(unsigned i=0;i<MAX_DRAWABLES;++i)if(drawables[i].id==id)return &drawables[i];
    return NULL;
}
// ------------------------=
// FUNC: InfinityTkDrawable
// DESC: Exposes only this native application's checked drawable storage to Tk's software rasterizer.
// ------------------=
InfinityTkPixels *InfinityTkDrawable(Drawable id){NativeDrawable *d=lookup(id);return d?&d->image:NULL;}
// ------------------------=
// FUNC: InfinityTkAllocate
// DESC: Allocates zeroed private pixels subject to per-drawable dimensions and total memory bounds.
// ------------------=
Drawable InfinityTkAllocate(unsigned width,unsigned height,int depth){
    if(!width||!height||width>8192||height>8192||(depth!=1&&depth!=24&&depth!=32))return None;
    size_t size=(size_t)width*height*4;if(size>PIXEL_BUDGET-pixel_bytes||next_drawable==~(Drawable)0)return None;
    NativeDrawable *slot=NULL;for(unsigned i=0;i<MAX_DRAWABLES;++i)if(!drawables[i].id){slot=&drawables[i];break;}
    if(!slot)return None;uint32_t *pixels=calloc(1,size);if(!pixels)return None;
    slot->id=next_drawable++;slot->image=(InfinityTkPixels){pixels,width,height,depth};slot->revision=1;pixel_bytes+=size;
    return slot->id;
}
// ------------------------=
// FUNC: InfinityTkResize
// DESC: Preserves drawable overlap on resize and atomically replaces its storage only after allocation succeeds.
// ------------------=
int InfinityTkResize(Drawable id,unsigned width,unsigned height){
    NativeDrawable *d=lookup(id);if(!d||!width||!height||width>8192||height>8192)return 0;
    size_t old=(size_t)d->image.width*d->image.height*4,size=(size_t)width*height*4;
    if(size>PIXEL_BUDGET-(pixel_bytes-old))return 0;
    uint32_t *pixels=calloc(1,size);if(!pixels)return 0;
    unsigned w=width<d->image.width?width:d->image.width,h=height<d->image.height?height:d->image.height;
    for(unsigned y=0;y<h;++y)memcpy(pixels+(size_t)y*width,d->image.pixels+(size_t)y*d->image.width,(size_t)w*4);
    free(d->image.pixels);d->image.pixels=pixels;d->image.width=width;d->image.height=height;
    pixel_bytes=pixel_bytes-old+size;++d->revision;return 1;
}
// ------------------------=
// FUNC: InfinityTkRelease
// DESC: Releases one owned drawable; its identifier is never assigned to a later allocation.
// ------------------=
void InfinityTkRelease(Drawable id){NativeDrawable *d=lookup(id);if(!d)return;pixel_bytes-=(size_t)d->image.width*d->image.height*4;free(d->image.pixels);memset(d,0,sizeof(*d));}
// ------------------------=
// FUNC: InfinityTkDamage
// DESC: Marks completed raster mutations for retained compositor publication without publishing half-drawn widgets.
// ------------------=
void InfinityTkDamage(Drawable id){NativeDrawable *d=lookup(id);if(d)++d->revision;}
// ------------------------=
// FUNC: InfinityTkRevision
// DESC: Reports the actual private drawable mutation revision to native toplevel composition.
// ------------------=
uint64_t InfinityTkRevision(Drawable id){NativeDrawable *d=lookup(id);return d?d->revision:0;}
// ------------------------=
// FUNC: raster_operation
// DESC: Implements the sixteen X logical raster operations over real source and destination pixels.
// ------------------=
static uint32_t raster_operation(int operation,uint32_t s,uint32_t d){
    switch(operation){case GXclear:return 0;case GXand:return s&d;case GXandReverse:return s&~d;case GXcopy:return s;
    case GXandInverted:return ~s&d;case GXnoop:return d;case GXxor:return s^d;case GXor:return s|d;case GXnor:return ~(s|d);
    case GXequiv:return ~(s^d);case GXinvert:return ~d;case GXorReverse:return s|~d;case GXcopyInverted:return ~s;
    case GXorInverted:return ~s|d;case GXnand:return ~(s&d);case GXset:return ~0u;default:return d;}
}
// ------------------------=
// FUNC: InfinityTkBlend
// DESC: Clips signed coordinates against exact regions or bitmasks, applies native raster operations, and blends antialiased glyph coverage.
// ------------------=
void InfinityTkBlend(Drawable id,GC gc,int x,int y,uint32_t color,unsigned coverage){
    InfinityTkPixels *d=InfinityTkDrawable(id);if(!d||!gc||!coverage||x<0||y<0||(unsigned)x>=d->width||(unsigned)y>=d->height)return;
    if(gc->clip_mask){TkpClipMask *mask=(TkpClipMask *)gc->clip_mask;int cx=x-gc->clip_x_origin,cy=y-gc->clip_y_origin;
        if(mask->type==TKP_CLIP_REGION){if(!XPointInRegion((Region)mask->value.region,cx,cy))return;}
        else{InfinityTkPixels *bits=InfinityTkDrawable(mask->value.pixmap);if(!bits||cx<0||cy<0||(unsigned)cx>=bits->width||(unsigned)cy>=bits->height||!bits->pixels[(size_t)cy*bits->width+cx])return;}}
    uint32_t *pixel=&d->pixels[(size_t)y*d->width+x],old=*pixel;
    uint32_t value=raster_operation(gc->function,color,old),mask=(uint32_t)gc->plane_mask;
    value=(value&mask)|(old&~mask);if(coverage>255)coverage=255;
    if(coverage<255){uint32_t out=0;for(unsigned shift=0;shift<24;shift+=8)out|=((((value>>shift)&255)*coverage+((old>>shift)&255)*(255-coverage)+127)/255)<<shift;value=out;}
    *pixel=d->depth==1?(value&1):(value|0xff000000u);
}
// ------------------------=
// FUNC: fill_pixel
// DESC: Resolves solid, tiled and stippled fills using actual private pixmaps and checked tile coordinates.
// ------------------=
static void fill_pixel(Drawable id,GC gc,int x,int y){
    uint32_t color=(uint32_t)gc->foreground;
    if(gc->fill_style!=FillSolid){InfinityTkPixels *tile=InfinityTkDrawable(gc->fill_style==FillTiled?gc->tile:gc->stipple);if(!tile)return;
        int tx=(x-gc->ts_x_origin)%(int)tile->width,ty=(y-gc->ts_y_origin)%(int)tile->height;if(tx<0)tx+=tile->width;if(ty<0)ty+=tile->height;
        uint32_t bit=tile->pixels[(size_t)ty*tile->width+tx];if(gc->fill_style==FillTiled)color=bit;
        else if(!bit){if(gc->fill_style==FillStippled)return;color=(uint32_t)gc->background;}}
    InfinityTkBlend(id,gc,x,y,color,255);
}
// ------------------------=
// FUNC: XFillRectangles
// DESC: Fills only the clipped rectangle intersections, avoiding unbounded work for offscreen geometry.
// ------------------=
int XFillRectangles(Display *display,Drawable id,GC gc,XRectangle *rectangles,int count){
    (void)display;InfinityTkPixels *d=InfinityTkDrawable(id);if(!d||!gc||count<0)return BadDrawable;
    for(int i=0;i<count;++i){XRectangle r=rectangles[i];int left=r.x<0?0:r.x,top=r.y<0?0:r.y,right=r.x+r.width,bottom=r.y+r.height;
        if(right>(int)d->width)right=d->width;if(bottom>(int)d->height)bottom=d->height;
        for(int y=top;y<bottom;++y)for(int x=left;x<right;++x)fill_pixel(id,gc,x,y);}
    InfinityTkDamage(id);return Success;
}
// ------------------------=
// FUNC: line
// DESC: Draws a bounded-width antialiased segment with the selected logical raster operation and dash pattern.
// ------------------=
static void line(Drawable id,GC gc,int x1,int y1,int x2,int y2){
    InfinityTkPixels *d=InfinityTkDrawable(id);if(!d)return;
    double dx=(double)x2-x1,dy=(double)y2-y1,length=sqrt(dx*dx+dy*dy),radius=(gc->line_width>0?gc->line_width:1)*0.5;
    if(radius>4096)return;int left=(int)floor(fmin(x1,x2)-radius-1),top=(int)floor(fmin(y1,y2)-radius-1);
    int right=(int)ceil(fmax(x1,x2)+radius+1),bottom=(int)ceil(fmax(y1,y2)+radius+1);
    if(left<0)left=0;if(top<0)top=0;if(right>(int)d->width)right=d->width;if(bottom>(int)d->height)bottom=d->height;
    for(int y=top;y<bottom;++y)for(int x=left;x<right;++x){double t=length?(((x-x1)*dx+(y-y1)*dy)/(length*length)):0;
        if(t<0)t=0;if(t>1)t=1;double distance=hypot(x-x1-t*dx,y-y1-t*dy),alpha=radius+0.5-distance;
        if(alpha<=0)continue;uint32_t color=gc->foreground;
        if(gc->line_style!=LineSolid){unsigned dash=(unsigned char)gc->dashes;if(!dash)dash=4;
            if(((unsigned)(t*length+gc->dash_offset)/dash)&1){if(gc->line_style==LineOnOffDash)continue;color=gc->background;}}
        InfinityTkBlend(id,gc,x,y,color,(unsigned)(fmin(alpha,1.0)*255+0.5));}
}
// ------------------------=
// FUNC: XDrawLines
// DESC: Draws a connected polyline respecting absolute or relative coordinates and shared native clipping.
// ------------------=
int XDrawLines(Display *display,Drawable id,GC gc,XPoint *points,int count,int mode){
    (void)display;if(!InfinityTkDrawable(id)||!gc||count<0)return BadDrawable;if(!count)return Success;
    int x=points[0].x,y=points[0].y;if(count==1)line(id,gc,x,y,x,y);
    for(int i=1;i<count;++i){int nx=points[i].x+(mode==CoordModePrevious?x:0),ny=points[i].y+(mode==CoordModePrevious?y:0);line(id,gc,x,y,nx,ny);x=nx;y=ny;}
    InfinityTkDamage(id);return Success;
}
// ------------------------=
// FUNC: XDrawRectangle
// DESC: Draws the four requested rectangle edges using the native antialiased line rasterizer.
// ------------------=
int XDrawRectangle(Display *display,Drawable id,GC gc,int x,int y,unsigned w,unsigned h){
    (void)display;if(w>8192||h>8192)return BadValue;line(id,gc,x,y,x+w,y);line(id,gc,x+w,y,x+w,y+h);line(id,gc,x+w,y+h,x,y+h);line(id,gc,x,y+h,x,y);InfinityTkDamage(id);return Success;
}
// ------------------------=
// FUNC: XFillPolygon
// DESC: Rasterizes the actual polygon using nonzero winding or even-odd coverage instead of its bounding rectangle.
// ------------------=
int XFillPolygon(Display *display,Drawable id,GC gc,XPoint *points,int count,int shape,int mode){
    (void)display;(void)shape;InfinityTkPixels *d=InfinityTkDrawable(id);if(!d||!gc||count<3||count>65536)return BadValue;
    XPoint *p=ckalloc((size_t)count*sizeof(*p));memcpy(p,points,(size_t)count*sizeof(*p));
    if(mode==CoordModePrevious)for(int i=1;i<count;++i){p[i].x+=p[i-1].x;p[i].y+=p[i-1].y;}
    int left=d->width,right=0,top=d->height,bottom=0;for(int i=0;i<count;++i){if(p[i].x<left)left=p[i].x;if(p[i].x>right)right=p[i].x;if(p[i].y<top)top=p[i].y;if(p[i].y>bottom)bottom=p[i].y;}
    if(left<0)left=0;if(top<0)top=0;if(right>(int)d->width)right=d->width;if(bottom>(int)d->height)bottom=d->height;
    for(int y=top;y<bottom;++y)for(int x=left;x<right;++x){int winding=0;double px=x+0.5,py=y+0.5;
        for(int i=0,j=count-1;i<count;j=i++){double cross=(p[i].x-p[j].x)*(py-p[j].y)-(px-p[j].x)*(p[i].y-p[j].y);
            if(p[j].y<=py){if(p[i].y>py&&cross>0)++winding;}else if(p[i].y<=py&&cross<0)--winding;}
        if(gc->fill_rule==WindingRule?winding!=0:(winding&1)!=0)fill_pixel(id,gc,x,y);}
    ckfree(p);InfinityTkDamage(id);return Success;
}
// ------------------------=
// FUNC: XCopyArea
// DESC: Copies private pixels with overlap-safe staging and publishes a real NoExpose completion for Tk scrolling.
// ------------------=
int XCopyArea(Display *display,Drawable source,Drawable destination,GC gc,int sx,int sy,unsigned width,unsigned height,int dx,int dy){
    InfinityTkPixels *s=InfinityTkDrawable(source),*d=InfinityTkDrawable(destination);if(!s||!d||!gc||width>8192||height>8192)return BadDrawable;
    size_t bytes=(size_t)width*height*4;if(bytes>PIXEL_BUDGET)return BadAlloc;uint32_t *copy=malloc(bytes?bytes:4);if(!copy)return BadAlloc;
    for(unsigned y=0;y<height;++y)for(unsigned x=0;x<width;++x){int xx=sx+x,yy=sy+y;copy[(size_t)y*width+x]=(xx>=0&&yy>=0&&(unsigned)xx<s->width&&(unsigned)yy<s->height)?s->pixels[(size_t)yy*s->width+xx]:0;}
    for(unsigned y=0;y<height;++y)for(unsigned x=0;x<width;++x)InfinityTkBlend(destination,gc,dx+x,dy+y,copy[(size_t)y*width+x],255);
    free(copy);InfinityTkDamage(destination);
    if(gc->graphics_exposures){XEvent event;memset(&event,0,sizeof(event));event.xnoexpose.type=NoExpose;event.xnoexpose.display=display;event.xnoexpose.drawable=destination;Tk_QueueWindowEvent(&event,TCL_QUEUE_TAIL);}
    return Success;
}
// ------------------------=
// FUNC: XCopyPlane
// DESC: Expands one real source bitplane into foreground/background colors with overlap-safe source sampling.
// ------------------=
int XCopyPlane(Display *display,Drawable source,Drawable destination,GC gc,int sx,int sy,unsigned width,unsigned height,int dx,int dy,unsigned long plane){
    (void)display;InfinityTkPixels *s=InfinityTkDrawable(source);if(!s||!InfinityTkDrawable(destination)||!plane||(plane&(plane-1))||width>8192||height>8192)return BadValue;
    size_t size=(size_t)width*height;if(size>PIXEL_BUDGET)return BadAlloc;unsigned char *copy=malloc(size?size:1);if(!copy)return BadAlloc;
    for(unsigned y=0;y<height;++y)for(unsigned x=0;x<width;++x){int xx=sx+x,yy=sy+y;copy[(size_t)y*width+x]=xx>=0&&yy>=0&&(unsigned)xx<s->width&&(unsigned)yy<s->height&&(s->pixels[(size_t)yy*s->width+xx]&plane);}
    for(unsigned y=0;y<height;++y)for(unsigned x=0;x<width;++x)InfinityTkBlend(destination,gc,dx+x,dy+y,copy[(size_t)y*width+x]?gc->foreground:gc->background,255);
    free(copy);InfinityTkDamage(destination);return Success;
}
// ------------------------=
// FUNC: native_arc
// DESC: Rasterizes actual elliptical arcs, pie slices and chords with subpixel coverage, bounded drawable clipping and native GC semantics.
// ------------------=
static int native_arc(Drawable id,GC gc,int x,int y,unsigned width,unsigned height,int start,int extent,int fill){
    InfinityTkPixels *pixels=InfinityTkDrawable(id);if(!pixels||!gc)return BadDrawable;if(!width||!height||!extent)return Success;
    if(width>8192||height>8192||gc->line_width>8192)return BadValue;
    const double turn=6.2831853071795864769;double span=extent*(turn/(360*64)),angle=start*(turn/(360*64));
    if(span<0){angle+=span;span=-span;}if(span>turn)span=turn;angle=fmod(angle,turn);if(angle<0)angle+=turn;
    double rx=width*0.5,ry=height*0.5,cx=x+rx,cy=y+ry,stroke=(gc->line_width?gc->line_width:1)*0.5;
    double dl=floor((double)x-(fill?0:stroke)-1),dt=floor((double)y-(fill?0:stroke)-1);
    double dr=ceil((double)x+width+(fill?0:stroke)+1),db=ceil((double)y+height+(fill?0:stroke)+1);
    if(dr<=0||db<=0||dl>=pixels->width||dt>=pixels->height)return Success;
    int left=(int)fmax(dl,0),top=(int)fmax(dt,0);
    int right=(int)fmin(dr,pixels->width),bottom=(int)fmin(db,pixels->height);
    double ax=cos(angle),ay=sin(angle),bx=cos(angle+span),by=sin(angle+span);
    double side=(bx-ax)*(sin(angle+span/2)-ay)-(by-ay)*(cos(angle+span/2)-ax);
    for(int py=top;py<bottom;++py)for(int px=left;px<right;++px){unsigned covered=0;
        for(unsigned sy=0;sy<4;++sy)for(unsigned sx=0;sx<4;++sx){double nx=(px+(sx+0.5)/4-cx)/rx,ny=-(py+(sy+0.5)/4-cy)/ry,r=nx*nx+ny*ny;
            if(fill){if(r>1)continue;}else{double gradient=2*sqrt(nx*nx/(rx*rx)+ny*ny/(ry*ry));if(!gradient||fabs(r-1)/gradient>stroke)continue;}
            if(span<turn-0.000001){
                if(fill&&gc->arc_mode==ArcChord){double point=(bx-ax)*(ny-ay)-(by-ay)*(nx-ax);if(point*side<0)continue;}
                else{double delta=atan2(ny,nx)-angle;while(delta<0)delta+=turn;if(delta>span)continue;}
            }
            ++covered;
        }
        if(covered)InfinityTkBlend(id,gc,px,py,gc->foreground,(covered*255+8)/16);
    }
    InfinityTkDamage(id);return Success;
}
// ------------------------=
// FUNC: XDrawArc
// DESC: Draws one antialiased native ellipse segment with the real Tk graphics context.
// ------------------=
int XDrawArc(Display *display,Drawable id,GC gc,int x,int y,unsigned width,unsigned height,int start,int extent){(void)display;return native_arc(id,gc,x,y,width,height,start,extent,0);}
// ------------------------=
// FUNC: XFillArc
// DESC: Fills one actual antialiased pie or chord rather than replacing it with a bounding box.
// ------------------=
int XFillArc(Display *display,Drawable id,GC gc,int x,int y,unsigned width,unsigned height,int start,int extent){(void)display;return native_arc(id,gc,x,y,width,height,start,extent,1);}
// ------------------------=
// FUNC: XDrawArcs
// DESC: Draws each supplied real ellipse segment and propagates invalid drawing requests.
// ------------------=
int XDrawArcs(Display *display,Drawable id,GC gc,XArc *arcs,int count){if(count<0||(!arcs&&count))return BadValue;for(int i=0;i<count;++i){XArc a=arcs[i];int r=XDrawArc(display,id,gc,a.x,a.y,a.width,a.height,a.angle1,a.angle2);if(r!=Success)return r;}return Success;}
// ------------------------=
// FUNC: XFillArcs
// DESC: Fills each supplied real ellipse segment with the selected chord or pie mode.
// ------------------=
int XFillArcs(Display *display,Drawable id,GC gc,XArc *arcs,int count){if(count<0||(!arcs&&count))return BadValue;for(int i=0;i<count;++i){XArc a=arcs[i];int r=XFillArc(display,id,gc,a.x,a.y,a.width,a.height,a.angle1,a.angle2);if(r!=Success)return r;}return Success;}
// ------------------------=
// FUNC: XDrawSegments
// DESC: Draws disconnected segments using the same native clipped antialiased stroke implementation.
// ------------------=
int XDrawSegments(Display *display,Drawable id,GC gc,XSegment *segments,int count){(void)display;if(!InfinityTkDrawable(id)||!gc||count<0||(!segments&&count))return BadValue;
    for(int i=0;i<count;++i)line(id,gc,segments[i].x1,segments[i].y1,segments[i].x2,segments[i].y2);InfinityTkDamage(id);return Success;}
// ------------------------=
// FUNC: XDrawPoints
// DESC: Writes actual absolute or relative point pixels using checked native raster operations.
// ------------------=
int XDrawPoints(Display *display,Drawable id,GC gc,XPoint *points,int count,int mode){(void)display;if(!InfinityTkDrawable(id)||!gc||count<0||(!points&&count))return BadValue;
    int x=0,y=0;for(int i=0;i<count;++i){x=points[i].x+(mode==CoordModePrevious?x:0);y=points[i].y+(mode==CoordModePrevious?y:0);InfinityTkBlend(id,gc,x,y,gc->foreground,255);}InfinityTkDamage(id);return Success;}
// ------------------------=
// FUNC: XDrawPoint
// DESC: Draws one real clipped pixel with full signed coordinates and the selected graphics-context raster operation.
// ------------------=
int XDrawPoint(Display *display,Drawable id,GC gc,int x,int y){
    (void)display;if(!InfinityTkDrawable(id)||!gc)return BadDrawable;
    InfinityTkBlend(id,gc,x,y,gc->foreground,255);InfinityTkDamage(id);return Success;
}
