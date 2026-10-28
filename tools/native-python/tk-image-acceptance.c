#include "native.h"

// ------------------------=
// FUNC: InfinityTkImageAcceptance
// DESC: Exercises real pixel layouts, subimages, native drawables, raster masks and readback without depending on rendered copy.
// ------------------=
int InfinityTkImageAcceptance(void) {
    for(int unit=8;unit<=32;unit*=2)for(int bytes=0;bytes<2;++bytes)for(int bits=0;bits<2;++bits){
        unsigned char storage[16];memset(storage,0xa5,sizeof(storage));
        XImage *im=XCreateImage(NULL,NULL,1,XYBitmap,3,(char *)storage,17,2,unit,8);if(!im)return 1;
        im->byte_order=bytes;im->bitmap_bit_order=bits;if(!_XInitImageFuncPtrs(im))return 2;
        for(int y=0;y<2;++y)for(int x=0;x<17;++x){unsigned value=(unsigned)(x+y)&1;
            if(!XPutPixel(im,x,y,value)||XGetPixel(im,x,y)!=value)return 3;}
        unsigned mask=1u<<(bits==LSBFirst?3:4),byte=bytes==bits?0:unit/8-1;
        if(storage[byte]&mask)return 4;
        unsigned char baseline[16];memcpy(baseline,storage,sizeof(storage));
        if(XPutPixel(im,-1,0,1)||XPutPixel(im,17,0,1)||memcmp(baseline,storage,16))return 5;
        XImage *copy=XSubImage(im,2,0,7,2);if(!copy)return 6;
        for(int y=0;y<2;++y)for(int x=0;x<7;++x)if(XGetPixel(copy,x,y)!=XGetPixel(im,x+2,y))return 7;
        XDestroyImage(copy);im->data=NULL;XDestroyImage(im);
    }
    for(int depth=24;depth<=32;depth+=8)for(int order=0;order<2;++order){
        unsigned char data[32];memset(data,0xcc,sizeof(data));
        XImage *im=XCreateImage(NULL,NULL,depth,ZPixmap,0,(char *)data,2,2,8,16);if(!im)return 8;
        im->byte_order=order;if(!_XInitImageFuncPtrs(im))return 9;
        if(!XPutPixel(im,1,1,0x123456)||XGetPixel(im,1,1)!=0x123456)return 10;
        unsigned at=16+depth/8;
        if(data[at]!=(order==LSBFirst?0x56:depth==24?0x12:0))return 11;
        if(data[at+depth/8-1]!=(order==LSBFirst?(depth==24?0x12:0):0x56))return 12;
        if(data[15]!=0xcc||data[31]!=0xcc)return 13;
        XAddPixel(im,1);if(XGetPixel(im,1,1)!=0x123457)return 14;
        im->data=NULL;XDestroyImage(im);
    }
    if(XCreateImage(NULL,NULL,24,ZPixmap,0,NULL,8193,1,32,0)||
       XCreateImage(NULL,NULL,24,ZPixmap,0,NULL,4,2,32,3)||
       XCreateImage(NULL,NULL,16,ZPixmap,0,NULL,4,2,32,0))return 15;
    Drawable target=InfinityTkAllocate(4,3,24);if(!target)return 16;
    XGCValues gc;memset(&gc,0,sizeof(gc));gc.function=GXcopy;gc.plane_mask=0xffffff;gc.foreground=0x123456;gc.background=0xabcdef;
    unsigned char bits[1]={5};XImage *bitmap=XCreateImage(NULL,NULL,1,XYBitmap,0,(char *)bits,3,1,8,1);if(!bitmap)return 17;
    uint64_t before=InfinityTkRevision(target);
    if(XPutImage(NULL,target,&gc,bitmap,0,0,0,1,3,1)!=Success||InfinityTkRevision(target)<=before)return 18;
    XImage *read=XGetImage(NULL,target,0,1,3,1,AllPlanes,ZPixmap);if(!read)return 19;
    if(XGetPixel(read,0,0)!=0x123456||XGetPixel(read,1,0)!=0xabcdef||XGetPixel(read,2,0)!=0x123456)return 20;
    gc.function=GXxor;gc.plane_mask=0xff;
    if(XPutImage(NULL,target,&gc,read,0,0,0,1,3,1)!=Success)return 21;
    XImage *masked=XGetImage(NULL,target,0,1,3,1,0xffff00,ZPixmap);if(!masked)return 22;
    if(XGetPixel(masked,0,0)!=0x123400||XGetPixel(masked,1,0)!=0xabcd00)return 23;
    if(XGetImage(NULL,target,-1,0,1,1,AllPlanes,ZPixmap))return 24;
    XDestroyImage(read);XDestroyImage(masked);bitmap->data=NULL;XDestroyImage(bitmap);InfinityTkRelease(target);
    return 0;
}
