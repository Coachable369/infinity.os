#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <math.h>

typedef void (*Kernel)(uint32_t,const uint8_t *,const float *,size_t,size_t,float *);
void infinity_qwen_dot_rows(uint32_t,const uint8_t *,const float *,size_t,size_t,float *);
void baseline_dot_rows(uint32_t,const uint8_t *,const float *,size_t,size_t,float *);

// ------------------------=
// FUNC: outputs_are_valid
// DESC: Requires bounded normalized activation-quantization error for Q4 and Q6 integer-dot paths.
// ------------------=
static int outputs_are_valid(unsigned kind,const float *expected,const float *actual,unsigned rows) {
    (void)kind;
    double error=0.0,reference=0.0;
    for(unsigned row=0;row<rows;row++) {
        if(!isfinite(actual[row]))return 0;
        double delta=(double)expected[row]-actual[row];
        error+=delta*delta;
        reference+=(double)expected[row]*expected[row];
    }
    return sqrt(error/(reference+rows))<=0.02;
}

// ------------------------=
// FUNC: measure
// DESC: Times equal native CPU workloads without formatting or allocations inside the timed region.
// ------------------=
static double measure(Kernel kernel,unsigned kind,const uint8_t *data,const float *input,
                      unsigned width,unsigned rows,unsigned repeats,float *output) {
    clock_t start=clock();
    for(unsigned i=0;i<repeats;i++) kernel(kind,data,input,width,rows,output);
    return (double)(clock()-start)/CLOCKS_PER_SEC;
}

// ------------------------=
// FUNC: median
// DESC: Returns the median of seven repeated measurements without depending on printed diagnostics.
// ------------------=
static double median(double *values) {
    for(unsigned i=1;i<7;i++) for(unsigned j=i;j && values[j]<values[j-1];j--) {
        double t=values[j];values[j]=values[j-1];values[j-1]=t;
    }
    return values[3];
}

// ------------------------=
// FUNC: main
// DESC: Compares real kernels with bounded approximation error, alternating order, and cache-sized and worker-sized matrices.
// ------------------=
int main(void) {
    const unsigned widths[]={3072,4096,8192,12288}, row_counts[]={8,4096};
    uint8_t *data=malloc(4096*48*210);
    float *input=malloc(12288*sizeof(float)), *expected=malloc(4096*sizeof(float)), *actual=malloc(4096*sizeof(float));
    assert(data && input && expected && actual);
    uint32_t seed=42; unsigned q6_count=0;
    for(unsigned kind=12;kind<=14;kind+=2) for(unsigned wi=0;wi<sizeof(widths)/sizeof(widths[0]);wi++) for(unsigned ri=0;ri<2;ri++) {
        unsigned width=widths[wi],rows=row_counts[ri],block=kind==12?144:210;
        size_t stride=width/256*block;
        for(size_t i=0;i<rows*stride;i++){seed=seed*1664525+1013904223;data[i]=seed>>24;}
        for(unsigned i=0;i<width;i++){seed=seed*1664525+1013904223;input[i]=((int)(seed>>16)-32768)/32768.0f;}
        for(unsigned r=0;r<rows;r++) for(unsigned b=0;b<width/256;b++) {
            size_t at=r*stride+b*block+(kind==12?0:208);
            data[at]=0; data[at+1]=0x24;
            if(kind==12){data[at+2]=0;data[at+3]=0x20;}
        }
        baseline_dot_rows(kind,data,input,width,rows,expected);
        infinity_qwen_dot_rows(kind,data,input,width,rows,actual);
        assert(outputs_are_valid(kind,expected,actual,rows));
        double before[7],after[7]; unsigned repeats=rows==8?20000:40;
        for(unsigned trial=0;trial<7;trial++) {
            if(trial&1) {
                after[trial]=measure(infinity_qwen_dot_rows,kind,data,input,width,rows,repeats,actual);
                before[trial]=measure(baseline_dot_rows,kind,data,input,width,rows,repeats,expected);
            } else {
                before[trial]=measure(baseline_dot_rows,kind,data,input,width,rows,repeats,expected);
                after[trial]=measure(infinity_qwen_dot_rows,kind,data,input,width,rows,repeats,actual);
            }
            assert(outputs_are_valid(kind,expected,actual,rows));
        }
        double b=median(before),a=median(after);
        printf("{\"kind\":%u,\"width\":%u,\"rows\":%u,\"repeats\":%u,\"trials\":7,\"baseline_seconds\":%.6f,\"optimized_seconds\":%.6f,\"speedup\":%.6f,\"valid\":true}\n",kind,width,rows,repeats,b,a,b/a);
        fflush(stdout);
        if(kind==14) q6_count++;
    }
    // The measurement report is authoritative; no noisy speed threshold is
    // enforced in CI. Deployment acceptance evaluates the recorded medians.
    assert(q6_count==2*sizeof(widths)/sizeof(widths[0]));
    free(data);free(input);free(expected);free(actual);
    return 0;
}
