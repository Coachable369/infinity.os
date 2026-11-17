#include <assert.h>
#include <stdlib.h>
#include "../../sdk/python/tk/pixel_reflow.h"
// ------------------------=
// FUNC: check
// DESC: Exercise the production stride conversion and verify preserved overlap and cleared new area.
// ------------------=
static void check(unsigned ow,unsigned oh,unsigned nw,unsigned nh){
    size_t old=(size_t)ow*oh,size=(size_t)nw*nh;
    uint32_t *p=malloc((old>size?old:size)*sizeof(*p));assert(p);
    for(size_t i=0;i<old;++i)p[i]=(uint32_t)i+1;
    InfinityTkReflowPixels(p,ow,oh,nw,nh);
    for(unsigned y=0;y<nh;++y)for(unsigned x=0;x<nw;++x)
        assert(p[(size_t)y*nw+x]==(x<ow&&y<oh?(uint32_t)((size_t)y*ow+x+1):0));
    free(p);
}
// ------------------------=
// FUNC: main
// DESC: Verify mixed growth/shrink axes and repeated full-resolution layout buffer changes.
// ------------------=
int main(void){
    for(unsigned w=1;w<32;++w)for(unsigned h=1;h<32;++h){check(17,13,w,h);check(w,h,17,13);}
    check(1360,840,2400,1300);check(2400,1300,1360,840);check(2400,840,1360,1300);check(1360,1300,2400,840);
    return 0;
}
