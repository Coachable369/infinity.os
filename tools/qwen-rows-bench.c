#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <math.h>
#include <time.h>
#ifndef QWEN_BENCH_REPEATS
#define QWEN_BENCH_REPEATS 2000
#endif
#ifdef QWEN_VERIFY_SCALAR
void infinity_qwen_scalar_dot(uint32_t,const uint8_t *,const float *,size_t,float *);
#endif
void infinity_qwen_dot(uint32_t,const uint8_t *,const float *,size_t,float *);
void infinity_qwen_dot_rows(uint32_t,const uint8_t *,const float *,size_t,size_t,float *);
// ------------------------=
// FUNC: q4_error_is_bounded
// DESC: Accepts Q8 activation-rounding drift only within a strict mixed absolute and relative bound.
// ------------------=
static int q4_error_is_bounded(float expected,float actual) {
    if(isfinite(actual) && fabsf(expected-actual)<=1.0f+0.05f*fabsf(expected))return 1;
    fprintf(stderr,"q4 expected=%f actual=%f error=%f\n",expected,actual,fabsf(expected-actual));
    return 0;
}
// ------------------------=
// FUNC: main
// DESC: Verifies bounded Q4 and exact Q6 arithmetic, widths and row tails, then compares equal workloads.
// ------------------=
int main(void) {
    static uint8_t data[9*48*210]; static float input[12288];
    float reference[9],actual[9]; uint32_t seed=42;
    for(unsigned kind=12;kind<=14;kind+=2) {
        unsigned widths[]={256,512,1024,2048,3072,4096,8192,9216,12288};
        for(unsigned wi=0;wi<sizeof(widths)/sizeof(widths[0]);wi++) {
            unsigned width=widths[wi];
            size_t stride=width/256*(kind==12?144:210);
            for(size_t i=0;i<sizeof(data);i++){seed=seed*1664525+1013904223;data[i]=seed>>24;}
            for(unsigned i=0;i<width;i++){seed=seed*1664525+1013904223;input[i]=((int)(seed>>16)-32768)/32768.0f;}
            for(unsigned r=0;r<9;r++)for(unsigned b=0;b<width/256;b++) {
                size_t at=r*stride+b*(kind==12?144:210)+(kind==12?0:208);
                data[at]=0;data[at+1]=0x24;
                if(kind==12){data[at+2]=0;data[at+3]=0x20;}
            }
            for(unsigned r=0;r<9;r++) infinity_qwen_dot(kind,data+r*stride,input,width,&reference[r]);
#ifdef QWEN_VERIFY_SCALAR
            for(unsigned r=0;r<9;r++) {
                float scalar;
                infinity_qwen_scalar_dot(kind,data+r*stride,input,width,&scalar);
                assert(memcmp(&scalar,&reference[r],sizeof(float))==0);
            }
#endif
            for(unsigned rows=0;rows<=9;rows++) {
                actual[rows<9?rows:8]=123;
                infinity_qwen_dot_rows(kind,data,input,width,rows,actual);
                if(kind==12) {
                    for(unsigned r=0;r<rows;r++)assert(q4_error_is_bounded(reference[r],actual[r]));
                } else {
                    assert(memcmp(reference,actual,rows*sizeof(float))==0);
                }
                if(rows<9)assert(actual[rows]==123);
            }
            clock_t start=clock();
            for(unsigned repeat=0;repeat<QWEN_BENCH_REPEATS;repeat++)for(unsigned r=0;r<8;r++)infinity_qwen_dot(kind,data+r*stride,input,width,&actual[r]);
            double single=(double)(clock()-start)/CLOCKS_PER_SEC;
            start=clock();
            for(unsigned repeat=0;repeat<QWEN_BENCH_REPEATS;repeat++)infinity_qwen_dot_rows(kind,data,input,width,8,actual);
            double batch=(double)(clock()-start)/CLOCKS_PER_SEC;
            printf("kind=%u width=%u single=%.4f batch=%.4f speedup=%.2f\n",kind,width,single,batch,single/batch);
        }
    }
}
