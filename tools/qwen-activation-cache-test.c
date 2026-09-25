#include <assert.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

void infinity_qwen_dot_rows(uint32_t,const uint8_t *,const float *,size_t,size_t,float *);
void infinity_qwen_dot_rows_cached(uint32_t,const uint8_t *,const float *,size_t,size_t,float *,float *,int);

// ------------------------=
// FUNC: main
// DESC: Verifies cached cooperative slices exactly match direct kernels, refresh changed inputs, and preserve scratch guard bytes.
// ------------------=
int main(void) {
    const size_t widths[]={256,3072,8192,12288}, rows=33;
    for(size_t w=0;w<4;w++) for(unsigned kind=12;kind<=14;kind+=2) {
        size_t width=widths[w], block=kind==12?144:210, stride=width/256*block;
        uint8_t *data=malloc(rows*stride);
        float *input=malloc(width*sizeof(float)),*scratch=malloc((width+16)*sizeof(float));
        float expected[33],actual[33];
        assert(data && input && scratch);
        for(size_t i=0;i<rows*stride;i++)data[i]=(uint8_t)(i*37+19);
        for(size_t r=0;r<rows;r++)for(size_t b=0;b<width/256;b++) {
            size_t at=r*stride+b*block+(kind==12?0:208);
            data[at]=0;data[at+1]=0x24;
            if(kind==12){data[at+2]=0;data[at+3]=0x20;}
        }
        for(size_t pass=0;pass<3;pass++) {
            for(size_t i=0;i<width;i++)input[i]=pass==2?0.0f:((int)(i%257)-128)*(int)(pass+1)/128.0f;
            for(size_t i=width;i<width+16;i++)scratch[i]=12345.0f;
            infinity_qwen_dot_rows(kind,data,input,width,rows,expected);
            for(size_t r=0;r<rows;r+=8) {
                size_t count=rows-r<8?rows-r:8;
                infinity_qwen_dot_rows_cached(kind,data+r*stride,input,width,count,actual+r,scratch,r==0);
            }
            assert(memcmp(actual,expected,sizeof(actual))==0);
            for(size_t i=width;i<width+16;i++)assert(scratch[i]==12345.0f);
        }
        free(data);free(input);free(scratch);
    }
    return 0;
}
