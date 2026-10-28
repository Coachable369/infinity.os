#include "native.h"

typedef struct NativeRect { int x1,y1,x2,y2; } NativeRect;
struct _XRegion { NativeRect *rectangles; size_t count,capacity; };

// ------------------------=
// FUNC: append
// DESC: Adds one nonempty rectangle to a bounded exact region without silently expanding its coverage.
// ------------------=
static void append(Region region,NativeRect rectangle) {
    if(rectangle.x1>=rectangle.x2||rectangle.y1>=rectangle.y2)return;
    if(region->count==65536)Tcl_Panic("native Tk region complexity exceeded");
    if(region->count==region->capacity){
        size_t next=region->capacity?region->capacity*2:8;
        region->rectangles=ckrealloc(region->rectangles,next*sizeof(NativeRect));region->capacity=next;
    }
    region->rectangles[region->count++]=rectangle;
}
// ------------------------=
// FUNC: intersection
// DESC: Computes exact half-open rectangle intersection with signed coordinates.
// ------------------=
static NativeRect intersection(NativeRect a,NativeRect b) {
    return (NativeRect){a.x1>b.x1?a.x1:b.x1,a.y1>b.y1?a.y1:b.y1,a.x2<b.x2?a.x2:b.x2,a.y2<b.y2?a.y2:b.y2};
}
// ------------------------=
// FUNC: subtract_rectangle
// DESC: Produces the exact disjoint remainder after removing an intersecting rectangle.
// ------------------=
static void subtract_rectangle(Region result,NativeRect rectangle,NativeRect remove) {
    NativeRect overlap=intersection(rectangle,remove);
    if(overlap.x1>=overlap.x2||overlap.y1>=overlap.y2){append(result,rectangle);return;}
    append(result,(NativeRect){rectangle.x1,rectangle.y1,rectangle.x2,overlap.y1});
    append(result,(NativeRect){rectangle.x1,overlap.y2,rectangle.x2,rectangle.y2});
    append(result,(NativeRect){rectangle.x1,overlap.y1,overlap.x1,overlap.y2});
    append(result,(NativeRect){overlap.x2,overlap.y1,rectangle.x2,overlap.y2});
}
// ------------------------=
// FUNC: replace
// DESC: Publishes a completed region atomically, allowing destination/source aliasing.
// ------------------=
static void replace(Region destination,Region temporary) {
    if(destination->rectangles)ckfree(destination->rectangles);
    *destination=*temporary;ckfree(temporary);
}
// ------------------------=
// FUNC: XCreateRegion
// DESC: Creates an empty exact region in private native task memory.
// ------------------=
Region XCreateRegion(void) {Region region=ckalloc(sizeof(*region));memset(region,0,sizeof(*region));return region;}
// ------------------------=
// FUNC: XDestroyRegion
// DESC: Releases the exact region geometry without affecting other copied regions.
// ------------------=
int XDestroyRegion(Region region){if(region){if(region->rectangles)ckfree(region->rectangles);ckfree(region);}return 1;}
// ------------------------=
// FUNC: XPointInRegion
// DESC: Tests union membership without approximating holes by a bounding box.
// ------------------=
Bool XPointInRegion(Region region,int x,int y){
    if(!region)return False;
    for(size_t i=0;i<region->count;++i){NativeRect r=region->rectangles[i];if(x>=r.x1&&x<r.x2&&y>=r.y1&&y<r.y2)return True;}return False;
}
// ------------------------=
// FUNC: XEmptyRegion
// DESC: Reports whether the exact union contains any pixel.
// ------------------=
Bool XEmptyRegion(Region region){return !region||region->count==0;}
// ------------------------=
// FUNC: XUnionRegion
// DESC: Unions exact rectangle sets without mutating aliased sources during enumeration.
// ------------------=
int XUnionRegion(Region a,Region b,Region destination){
    Region result=XCreateRegion();for(size_t i=0;i<a->count;++i)append(result,a->rectangles[i]);
    for(size_t i=0;i<b->count;++i)append(result,b->rectangles[i]);replace(destination,result);return 1;
}
// ------------------------=
// FUNC: XUnionRectWithRegion
// DESC: Unions a bounded X rectangle and an existing exact region while preserving signed placement.
// ------------------=
int XUnionRectWithRegion(XRectangle *rectangle,Region source,Region destination){
    Region result=XCreateRegion();for(size_t i=0;i<source->count;++i)append(result,source->rectangles[i]);
    append(result,(NativeRect){rectangle->x,rectangle->y,rectangle->x+rectangle->width,rectangle->y+rectangle->height});replace(destination,result);return 1;
}
// ------------------------=
// FUNC: XIntersectRegion
// DESC: Computes exact region intersection and safely permits either input as destination.
// ------------------=
int XIntersectRegion(Region a,Region b,Region destination){
    Region result=XCreateRegion();for(size_t i=0;i<a->count;++i)for(size_t j=0;j<b->count;++j)append(result,intersection(a->rectangles[i],b->rectangles[j]));
    replace(destination,result);return 1;
}
// ------------------------=
// FUNC: XSubtractRegion
// DESC: Preserves region holes and disconnected pieces when subtracting arbitrary rectangle unions.
// ------------------=
int XSubtractRegion(Region a,Region b,Region destination){
    Region result=XCreateRegion();for(size_t i=0;i<a->count;++i)append(result,a->rectangles[i]);
    for(size_t i=0;i<b->count;++i){Region next=XCreateRegion();for(size_t j=0;j<result->count;++j)subtract_rectangle(next,result->rectangles[j],b->rectangles[i]);XDestroyRegion(result);result=next;}
    replace(destination,result);return 1;
}
// ------------------------=
// FUNC: XRectInRegion
// DESC: Distinguishes fully contained, partly overlapping and fully excluded rectangles using exact subtraction.
// ------------------=
int XRectInRegion(Region region,int x,int y,unsigned width,unsigned height){
    if(width>INT_MAX||height>INT_MAX||x>INT_MAX-(int)width||y>INT_MAX-(int)height)return RectangleOut;
    NativeRect rectangle={x,y,x+(int)width,y+(int)height};int overlaps=0;
    for(size_t i=0;i<region->count;++i){NativeRect part=intersection(rectangle,region->rectangles[i]);if(part.x1<part.x2&&part.y1<part.y2){overlaps=1;break;}}
    if(!overlaps)return RectangleOut;
    Region source=XCreateRegion(),remaining=XCreateRegion();append(source,rectangle);XSubtractRegion(source,region,remaining);
    int result=remaining->count?RectanglePart:RectangleIn;XDestroyRegion(source);XDestroyRegion(remaining);return result;
}
// ------------------------=
// FUNC: XOffsetRegion
// DESC: Translates exact region geometry after checking every coordinate for signed overflow.
// ------------------=
int XOffsetRegion(Region region,int dx,int dy){
    for(size_t i=0;i<region->count;++i){NativeRect r=region->rectangles[i];
        if((int64_t)r.x1+dx<INT_MIN||(int64_t)r.x2+dx>INT_MAX||(int64_t)r.y1+dy<INT_MIN||(int64_t)r.y2+dy>INT_MAX)return 0;}
    for(size_t i=0;i<region->count;++i){region->rectangles[i].x1+=dx;region->rectangles[i].x2+=dx;region->rectangles[i].y1+=dy;region->rectangles[i].y2+=dy;}return 1;
}
// ------------------------=
// FUNC: XClipBox
// DESC: Returns the documented region bounding rectangle while retaining exact membership separately.
// ------------------=
int XClipBox(Region region,XRectangle *rectangle){
    if(!region->count){*rectangle=(XRectangle){0,0,0,0};return 1;}
    NativeRect box=region->rectangles[0];for(size_t i=1;i<region->count;++i){NativeRect r=region->rectangles[i];
        if(r.x1<box.x1)box.x1=r.x1;if(r.y1<box.y1)box.y1=r.y1;if(r.x2>box.x2)box.x2=r.x2;if(r.y2>box.y2)box.y2=r.y2;}
    *rectangle=(XRectangle){box.x1,box.y1,box.x2-box.x1,box.y2-box.y1};return 1;
}
// ------------------------=
// FUNC: XEqualRegion
// DESC: Compares geometric coverage rather than rectangle decomposition or allocation identity.
// ------------------=
Bool XEqualRegion(Region a,Region b){
    Region delta=XCreateRegion();XSubtractRegion(a,b,delta);int same=!delta->count;
    if(same){XSubtractRegion(b,a,delta);same=!delta->count;}XDestroyRegion(delta);return same;
}
// ------------------------=
// FUNC: TkpCopyRegion
// DESC: Copies actual native region coverage for upstream Tk photo and widget drawing.
// ------------------=
void TkpCopyRegion(TkRegion destination,TkRegion source){Region empty=XCreateRegion();XUnionRegion((Region)source,empty,(Region)destination);XDestroyRegion(empty);}
// ------------------------=
// FUNC: TkpBuildRegionFromAlphaData
// DESC: Adds exact nonzero-alpha runs, preserving transparency holes in native Tk photo images.
// ------------------=
void TkpBuildRegionFromAlphaData(TkRegion region,unsigned x,unsigned y,unsigned width,unsigned height,unsigned char *pixels,unsigned pixel_stride,unsigned line_stride){
    for(unsigned row=0;row<height;++row){unsigned col=0;while(col<width){while(col<width&&!pixels[row*line_stride+col*pixel_stride])++col;unsigned start=col;
        while(col<width&&pixels[row*line_stride+col*pixel_stride])++col;
        if(col>start)append((Region)region,(NativeRect){(int)(x+start),(int)(y+row),(int)(x+col),(int)(y+row+1)});}}
}
