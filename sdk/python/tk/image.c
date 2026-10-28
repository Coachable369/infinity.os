#include "native.h"

#define IMAGE_LIMIT (64u*1024u*1024u)

// ------------------------=
// FUNC: image_valid
// DESC: Rejects unsupported layouts, overflow, undersized scanlines and unbounded client-side image storage.
// ------------------=
static int image_valid(const XImage *im) {
    if(!im||im->width<=0||im->height<=0||im->width>8192||im->height>8192||im->xoffset<0||im->xoffset>8192)return 0;
    if((im->byte_order!=LSBFirst&&im->byte_order!=MSBFirst)||(im->bitmap_bit_order!=LSBFirst&&im->bitmap_bit_order!=MSBFirst))return 0;
    if(im->bitmap_unit!=8&&im->bitmap_unit!=16&&im->bitmap_unit!=32)return 0;
    if(im->bitmap_pad!=8&&im->bitmap_pad!=16&&im->bitmap_pad!=32)return 0;
    int bits=im->bits_per_pixel;
    if(im->depth==1){if(bits!=1||(im->format!=XYBitmap&&im->format!=XYPixmap&&im->format!=ZPixmap))return 0;}
    else if((im->depth!=24&&im->depth!=32)||im->format!=ZPixmap||(bits!=24&&bits!=32)||bits<im->depth)return 0;
    size_t minimum=((size_t)(im->width+im->xoffset)*bits+7)/8;
    if(bits==1)minimum=(minimum+(size_t)im->bitmap_unit/8-1)/((size_t)im->bitmap_unit/8)*((size_t)im->bitmap_unit/8);
    return im->bytes_per_line>0&&(size_t)im->bytes_per_line>=minimum&&(size_t)im->bytes_per_line<=IMAGE_LIMIT/(size_t)im->height;
}

// ------------------------=
// FUNC: image_byte
// DESC: Locates a monochrome bit within its byte-order-aware bitmap unit and reports its exact mask.
// ------------------=
static size_t image_byte(const XImage *im,int x,int y,unsigned *mask) {
    size_t bit=(size_t)x+(size_t)im->xoffset,unit=(size_t)im->bitmap_unit,within=bit%unit,byte=within/8;
    if(im->byte_order!=im->bitmap_bit_order)byte=unit/8-1-byte;
    *mask=1u<<(im->bitmap_bit_order==LSBFirst?within%8:7-within%8);
    return (size_t)y*im->bytes_per_line+(bit/unit)*(unit/8)+byte;
}

// ------------------------=
// FUNC: image_get
// DESC: Reads one checked 1-bit or packed 24/32-bit pixel without alignment-dependent machine loads.
// ------------------=
static unsigned long image_get(XImage *im,int x,int y) {
    if(!image_valid(im)||!im->data||x<0||y<0||x>=im->width||y>=im->height)return 0;
    const unsigned char *data=(const unsigned char *)im->data;
    if(im->bits_per_pixel==1){unsigned mask;size_t at=image_byte(im,x,y,&mask);return (data[at]&mask)!=0;}
    size_t at=(size_t)y*im->bytes_per_line+(size_t)(x+im->xoffset)*(im->bits_per_pixel/8);
    unsigned bytes=im->bits_per_pixel/8;uint32_t value=0;
    for(unsigned i=0;i<bytes;++i)value|=(uint32_t)data[at+i]<<(8*(im->byte_order==LSBFirst?i:bytes-1-i));
    return im->depth==24?value&0xffffffu:value;
}

// ------------------------=
// FUNC: image_put
// DESC: Writes a bounded pixel while preserving neighboring bitmap bits, padding and host-independent byte order.
// ------------------=
static int image_put(XImage *im,int x,int y,unsigned long value) {
    if(!image_valid(im)||!im->data||x<0||y<0||x>=im->width||y>=im->height)return 0;
    unsigned char *data=(unsigned char *)im->data;
    if(im->bits_per_pixel==1){unsigned mask;size_t at=image_byte(im,x,y,&mask);data[at]=(unsigned char)((data[at]&~mask)|((value&1)?mask:0));return 1;}
    size_t at=(size_t)y*im->bytes_per_line+(size_t)(x+im->xoffset)*(im->bits_per_pixel/8);
    unsigned bytes=im->bits_per_pixel/8;if(im->depth==24)value&=0xffffffu;
    for(unsigned i=0;i<bytes;++i)data[at+i]=(unsigned char)(value>>(8*(im->byte_order==LSBFirst?i:bytes-1-i)));
    return 1;
}

// ------------------------=
// FUNC: image_destroy
// DESC: Releases Tk-allocator-owned image storage; callers retaining external data must detach it before destruction.
// ------------------=
static int image_destroy(XImage *im) {
    if(im){if(im->data)ckfree(im->data);ckfree(im);}return 1;
}

// ------------------------=
// FUNC: image_sub
// DESC: Copies a bounded rectangular subimage into separately owned storage with the same pixel format and ordering.
// ------------------=
static XImage *image_sub(XImage *im,int x,int y,unsigned width,unsigned height) {
    if(!image_valid(im)||!im->data||x<0||y<0||!width||!height||width>(unsigned)im->width||height>(unsigned)im->height||
        (unsigned)x>(unsigned)im->width-width||(unsigned)y>(unsigned)im->height-height)return NULL;
    XImage *copy=XCreateImage(NULL,NULL,(unsigned)im->depth,im->format,0,NULL,width,height,im->bitmap_pad,0);
    if(!copy)return NULL;copy->bits_per_pixel=im->bits_per_pixel;copy->byte_order=im->byte_order;
    copy->bitmap_bit_order=im->bitmap_bit_order;copy->bitmap_unit=im->bitmap_unit;
    copy->red_mask=im->red_mask;copy->green_mask=im->green_mask;copy->blue_mask=im->blue_mask;
    copy->bytes_per_line=(int)(((size_t)width*copy->bits_per_pixel+copy->bitmap_pad-1)/copy->bitmap_pad*(copy->bitmap_pad/8));
    if(!image_valid(copy)){image_destroy(copy);return NULL;}
    size_t size=(size_t)copy->bytes_per_line*height;copy->data=Tcl_AttemptAlloc((unsigned)size);
    if(!copy->data){image_destroy(copy);return NULL;}memset(copy->data,0,size);
    for(unsigned row=0;row<height;++row)for(unsigned col=0;col<width;++col)image_put(copy,col,row,image_get(im,x+col,y+row));
    return copy;
}

// ------------------------=
// FUNC: image_add
// DESC: Applies XAddPixel arithmetic through checked format-aware accessors, including one-bit wrapping.
// ------------------=
static int image_add(XImage *im,long amount) {
    if(!image_valid(im)||!im->data)return 0;
    for(int y=0;y<im->height;++y)for(int x=0;x<im->width;++x)image_put(im,x,y,image_get(im,x,y)+(unsigned long)amount);
    return 1;
}

// ------------------------=
// FUNC: _XInitImageFuncPtrs
// DESC: Refreshes Tk's image accessors after it changes the image's byte order or packing metadata.
// ------------------=
int _XInitImageFuncPtrs(XImage *im) {
    if(!image_valid(im))return 0;
    im->f.create_image=XCreateImage;im->f.destroy_image=image_destroy;im->f.get_pixel=image_get;
    im->f.put_pixel=image_put;im->f.sub_image=image_sub;im->f.add_pixel=image_add;return 1;
}

// ------------------------=
// FUNC: XCreateImage
// DESC: Creates an owned image descriptor over caller-provided pixels without silently allocating or replacing its data.
// ------------------=
XImage *XCreateImage(Display *display,Visual *visual,unsigned depth,int format,int offset,char *data,
    unsigned width,unsigned height,int pad,int stride) {
    (void)display;if(!width||!height||width>8192||height>8192||offset<0||offset>8192||stride<0)return NULL;
    if(!pad)pad=32;
    if(pad!=8&&pad!=16&&pad!=32)return NULL;
    if(depth!=1&&depth!=24&&depth!=32)return NULL;
    XImage *im=(XImage *)Tcl_AttemptAlloc(sizeof(XImage));if(!im)return NULL;memset(im,0,sizeof(*im));
    im->width=width;im->height=height;im->depth=depth;im->format=format;im->xoffset=offset;im->data=data;
    im->byte_order=LSBFirst;im->bitmap_bit_order=LSBFirst;im->bitmap_pad=pad;im->bitmap_unit=pad;
    im->bits_per_pixel=(int)depth;
    im->bytes_per_line=stride?stride:(int)(((size_t)(width+offset)*im->bits_per_pixel+pad-1)/pad*(pad/8));
    im->red_mask=visual?visual->red_mask:0xff0000;im->green_mask=visual?visual->green_mask:0xff00;im->blue_mask=visual?visual->blue_mask:0xff;
    if(!_XInitImageFuncPtrs(im)){im->data=NULL;image_destroy(im);return NULL;}return im;
}

// ------------------------=
// FUNC: XPutImage
// DESC: Copies real image pixels into a native drawable using GC clipping, plane masks, raster operations and bitmap colors.
// ------------------=
int XPutImage(Display *display,Drawable drawable,GC gc,XImage *im,int sx,int sy,int dx,int dy,unsigned width,unsigned height) {
    (void)display;InfinityTkPixels *target=InfinityTkDrawable(drawable);
    if(!target||!gc||!image_valid(im)||!im->data||width>8192||height>8192)return BadValue;
    int copy=target->depth!=1&&im->format==ZPixmap&&im->depth!=1&&!gc->clip_mask&&gc->function==GXcopy&&
        (gc->plane_mask&0xffffffu)==0xffffffu;
    for(unsigned row=0;row<height;++row)for(unsigned col=0;col<width;++col){
        int64_t x=(int64_t)sx+col,y=(int64_t)sy+row,tx=(int64_t)dx+col,ty=(int64_t)dy+row;
        if(x<0||y<0||x>=im->width||y>=im->height||tx<0||ty<0||tx>=target->width||ty>=target->height)continue;
        uint32_t value=(uint32_t)image_get(im,(int)x,(int)y);
        if(im->format==XYBitmap)value=value?(uint32_t)gc->foreground:(uint32_t)gc->background;
        if(copy)target->pixels[(size_t)ty*target->width+(size_t)tx]=value|0xff000000u;
        else InfinityTkBlend(drawable,gc,(int)tx,(int)ty,value,255);
    }InfinityTkDamage(drawable);return Success;
}

// ------------------------=
// FUNC: XGetImage
// DESC: Reads an in-bounds native drawable rectangle into independently owned pixels with the requested plane mask.
// ------------------=
XImage *XGetImage(Display *display,Drawable drawable,int x,int y,unsigned width,unsigned height,unsigned long planes,int format) {
    InfinityTkPixels *source=InfinityTkDrawable(drawable);
    if(!source||x<0||y<0||!width||!height||width>source->width||height>source->height||
       (unsigned)x>source->width-width||(unsigned)y>source->height-height)return NULL;
    if(format!=ZPixmap&&(format!=XYPixmap||source->depth!=1))return NULL;
    XImage *im=XCreateImage(display,NULL,(unsigned)source->depth,format,0,NULL,width,height,32,0);if(!im)return NULL;
    size_t size=(size_t)im->bytes_per_line*height;im->data=Tcl_AttemptAlloc((unsigned)size);
    if(!im->data){image_destroy(im);return NULL;}memset(im->data,0,size);
    for(unsigned row=0;row<height;++row)for(unsigned col=0;col<width;++col)
        image_put(im,col,row,source->pixels[((size_t)y+row)*source->width+x+col]&planes);
    return im;
}
