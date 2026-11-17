#ifndef INFINITY_TK_PIXEL_REFLOW_H
#define INFINITY_TK_PIXEL_REFLOW_H
#include <stdint.h>
#include <string.h>
// ------------------------=
// FUNC: InfinityTkReflowPixels
// DESC: Preserve overlap in an already resized buffer without a second allocation; clear exposed pixels.
// ------------------=
static void InfinityTkReflowPixels(uint32_t *pixels,unsigned old_width,unsigned old_height,unsigned width,unsigned height){
    unsigned w=width<old_width?width:old_width,h=height<old_height?height:old_height;
    // Retain row overlap in-place: widen backwards and narrow forwards so source
    // rows are never overwritten. This avoids a second full-window allocation.
    if(width>old_width){
        for(unsigned y=h;y-->0;)memmove(pixels+(size_t)y*width,pixels+(size_t)y*old_width,(size_t)w*4);
    }else if(width<old_width){
        for(unsigned y=0;y<h;++y)memmove(pixels+(size_t)y*width,pixels+(size_t)y*old_width,(size_t)w*4);
    }
    for(unsigned y=0;y<h;++y)if(width>w)memset(pixels+(size_t)y*width+w,0,(size_t)(width-w)*4);
    if(height>h)memset(pixels+(size_t)h*width,0,(size_t)(height-h)*width*4);
}
#endif
