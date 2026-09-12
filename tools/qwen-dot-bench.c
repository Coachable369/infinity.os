#include <assert.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <time.h>
void infinity_qwen_dot(uint32_t,const uint8_t *,const float *,size_t,float *);
void qwen_scalar_dot(uint32_t,const uint8_t *,const float *,size_t,float *);
// ------------------------=
// FUNC: main
// DESC: Compares scalar and vector outputs on varied K-quant rows and times equal CPU workloads.
// ------------------=
int main(void) {
    uint8_t data[48*210]; float input[12288],a,b; uint32_t seed=42;
    for(unsigned kind=12;kind<=14;kind+=2) {
        unsigned block=kind==12?144:210;
        for(unsigned trial=0;trial<100;trial++) {
            for(unsigned i=0;i<sizeof(data);i++) {seed=seed*1664525+1013904223;data[i]=seed>>24;}
            for(unsigned i=0;i<12288;i++) {seed=seed*1664525+1013904223;input[i]=((int)(seed>>16)-32768)/32768.0f;}
            for(unsigned i=0;i<48;i++) {
                unsigned offset=i*block+(kind==12?0:208);
                data[offset]=0;data[offset+1]=0x24;
                if(kind==12){data[offset+2]=0;data[offset+3]=0x20;}
            }
            infinity_qwen_dot(kind,data,input,12288,&a);
            qwen_scalar_dot(kind,data,input,12288,&b);
            assert(isfinite(a) && a==b);
        }
        clock_t start=clock();
        for(unsigned i=0;i<20000;i++)qwen_scalar_dot(kind,data,input,12288,&b);
        double scalar=(double)(clock()-start)/CLOCKS_PER_SEC;
        start=clock();
        for(unsigned i=0;i<20000;i++)infinity_qwen_dot(kind,data,input,12288,&a);
        double vector=(double)(clock()-start)/CLOCKS_PER_SEC;
        printf("kind=%u scalar_seconds=%.3f vector_seconds=%.3f speedup=%.2f\n",kind,scalar,vector,scalar/vector);
        assert(a==b);
    }
}
