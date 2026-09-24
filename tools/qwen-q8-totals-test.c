#include <assert.h>
#include <math.h>
#include <stdint.h>
#include <string.h>
#include <stddef.h>
#include <stdio.h>

void infinity_qwen_dot_rows(uint32_t,const uint8_t *,const float *,size_t,size_t,float *);

// ------------------------=
// FUNC: half_value
// DESC: Independently expands finite half precision scales, including signed zeros and subnormals.
// ------------------=
static float half_value(const uint8_t *p) {
    unsigned h=p[0]|(unsigned)p[1]<<8,exponent=(h>>10)&31,mantissa=h&1023;
    float value=exponent?ldexpf((float)(1024+mantissa),(int)exponent-25):ldexpf((float)mantissa,-24);
    return (h&32768)?-value:value;
}

// ------------------------=
// FUNC: reference
// DESC: Independently computes Q8 activation rounding and integer-weighted Q4 block sums, recalculating totals for each row.
// ------------------=
static float reference(const uint8_t *data,const float *input,unsigned width) {
    float result=0;
    for(unsigned block=0;block<width/256;block++) {
        const uint8_t *p=data+block*144,*s=p+4;
        const float *x=input+block*256;
        float maximum=0;
        for(unsigned i=0;i<256;i++)maximum=fmaxf(maximum,fabsf(x[i]));
        float xd=maximum*(1.0f/127.0f),inverse=xd==0?0:1.0f/xd;
        int32_t weighted=0,offset=0;
        for(unsigned g=0;g<8;g++) {
            int total=0,dot=0;
            for(unsigned lane=0;lane<32;lane++) {
                int activation=(int)nearbyintf(x[g*32+lane]*inverse);
                unsigned packed=p[16+(g/2)*32+lane];
                unsigned q=(g&1)?packed>>4:packed&15;
                total+=activation; dot+=(int)q*activation;
            }
            unsigned scale=g<4?s[g]&63:(s[g+4]&15)|((s[g-4]>>6)<<4);
            unsigned minimum=g<4?s[g+4]&63:(s[g+4]>>4)|((s[g]>>6)<<4);
            weighted+=(int32_t)scale*dot;
            offset+=(int32_t)minimum*total;
        }
        result+=((float)weighted*half_value(p)-(float)offset*half_value(p+2))*xd;
    }
    return result;
}

// ------------------------=
// FUNC: main
// DESC: Checks exact Q4 results and untouched output tails across supported widths, signed extremes, zero vectors, and ties-to-even activation rounding.
// ------------------=
int main(void) {
#if defined(__aarch64__) && defined(__ARM_FEATURE_DOTPROD) && !defined(QWEN_SCALAR)
    static uint8_t data[9*48*144];
    static float input[12288];
    const unsigned widths[]={0,256,512,3072,4096,8192,12288};
    uint32_t seed=42;
    for(unsigned wi=0;wi<sizeof(widths)/sizeof(widths[0]);wi++) {
        unsigned width=widths[wi],stride=width/256*144;
        for(unsigned i=0;i<9*stride;i++){seed=seed*1664525+1013904223;data[i]=seed>>24;}
        for(unsigned r=0;r<9;r++)for(unsigned b=0;b<width/256;b++) {
            uint8_t *p=data+r*stride+b*144;
            // Keep randomized signs, mantissas and finite exponents, including
            // subnormal/zero/minimum/maximum scales at explicit boundaries.
            p[1]&=0xfb;p[3]&=0xfb;
            if(b==0) { p[0]=r==0?0:r==1?1:255;p[1]=r<2?0:0x7b; }
        }
        for(unsigned pattern=0;pattern<5;pattern++) {
            for(unsigned i=0;i<width;i++) {
                seed=seed*1664525+1013904223;
                input[i]=pattern==0?0:pattern==1?127:pattern==2?-127:
                         pattern==3?((i%256)==0?127:((int)(i%255)-127)+0.5f):
                         ((int)(seed>>16)-32768)/32768.0f;
            }
            float expected[9];
            for(unsigned row=0;row<9;row++)expected[row]=reference(data+row*stride,input,width);
            for(unsigned rows=0;rows<=9;rows++) {
                float actual[10];for(unsigned i=0;i<10;i++)actual[i]=123;
                infinity_qwen_dot_rows(12,data,input,width,rows,actual);
                assert(memcmp(expected,actual,rows*sizeof(float))==0);
                for(unsigned i=rows;i<10;i++)assert(actual[i]==123);
            }
        }
    }
    return 0;
#else
    fprintf(stderr,"Q8 totals check requires ARM integer dot product; skipped on this target.\n");
    return 77;
#endif
}
