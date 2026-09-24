#include <assert.h>
#include <stdint.h>
#include <stdlib.h>
#include <math.h>
void infinity_qwen_scalar_dot(uint32_t,const uint8_t*,const float*,size_t,float*);
void infinity_qwen_dot_rows(uint32_t,const uint8_t*,const float*,size_t,size_t,float*);
// ------------------------=
// FUNC: main
// DESC: Independently quantizes activations and checks fused Q6 output, tails, zero vectors and bounded approximation error.
// ------------------=
int main(void) {
    uint32_t seed=19;
    const unsigned widths[]={256,512,1024,2048,3072,4096,8192,9216,12288};
    for(unsigned wi=0;wi<sizeof(widths)/sizeof(widths[0]);wi++) {
        unsigned width=widths[wi];
        size_t stride=width/256*210;
        uint8_t *data=malloc(stride*7);
        float *x=malloc(width*sizeof(float)),*quantized=malloc(width*sizeof(float));
        assert(data && x && quantized);
        double aggregate_error=0,aggregate_energy=0,aggregate_roundoff=0;
        for(unsigned trial=0;trial<12;trial++) {
            for(size_t i=0;i<stride*7;i++) {seed=seed*1664525+1013904223;data[i]=seed>>24;}
            for(unsigned r=0;r<7;r++) for(unsigned b=0;b<width/256;b++) {
                data[r*stride+b*210+208]=0;data[r*stride+b*210+209]=0x24;
            }
            for(unsigned i=0;i<width;i++) {seed=seed*1664525+1013904223;x[i]=trial==0?0:((int)(seed>>16)-32768)/32768.f;}
            for(unsigned b=0;b<width;b+=256) {
                float maximum=0;
                for(unsigned i=0;i<256;i++) maximum=fmaxf(maximum,fabsf(x[b+i]));
                float scale=maximum*(1.f/127.f),inverse=scale==0?0:1.f/scale;
                for(unsigned i=0;i<256;i++) quantized[b+i]=nearbyintf(x[b+i]*inverse)*scale;
            }
            for(unsigned rows=0;rows<=7;rows++) {
                float actual[8];for(unsigned r=0;r<8;r++)actual[r]=123;
                infinity_qwen_dot_rows(14,data,x,width,rows,actual);
                double error=0,approximation=0,energy=0;
                for(unsigned r=0;r<rows;r++) {
                    float expected,original;
                    infinity_qwen_scalar_dot(14,data+r*stride,quantized,width,&expected);
                    infinity_qwen_scalar_dot(14,data+r*stride,x,width,&original);
                    assert(isfinite(actual[r]));
                    error+=(actual[r]-expected)*(double)(actual[r]-expected);
                    approximation+=(actual[r]-original)*(double)(actual[r]-original);
                    energy+=expected*(double)expected;
                }
                aggregate_roundoff+=error;
                aggregate_error+=approximation;
                aggregate_energy+=energy;
                for(unsigned r=rows;r<8;r++)assert(actual[r]==123);
            }
        }
        // Assess activation rounding across the corpus, not a single nearly
        // cancelling dot whose relative error is undefined near zero.
        assert(sqrt(aggregate_error/(aggregate_energy+1))<0.02);
#if defined(__ARM_FEATURE_DOTPROD)
        assert(sqrt(aggregate_roundoff/(aggregate_energy+1))<0.00002);
#endif
        free(data);free(x);free(quantized);
    }
}
