#include <assert.h>
#include <stdint.h>
#include <string.h>
#include <math.h>
#include <stddef.h>

void infinity_qwen_scalar_dot(uint32_t,const uint8_t *,const float *,size_t,float *);
void infinity_qwen_dot_rows(uint32_t,const uint8_t *,const float *,size_t,size_t,float *);

// ------------------------=
// FUNC: main
// DESC: Checks every finite half encoding through Q6 batch execution against independent scalar decoding, including signed scales and row tails.
// ------------------=
int main(void) {
    uint8_t data[8*210]; float input[256],expected[8],actual[9];
    uint32_t seed=42;
    for(unsigned i=0;i<256;i++) {
        seed=seed*1664525+1013904223;
        input[i]=((int)(seed>>16)-32768)/32768.0f;
    }
    input[0]=0;input[1]=-0.0f;input[2]=0x1p-140f;input[3]=-0x1p-140f;
    for(unsigned base=0;base<65536;base+=8) {
        if((base&0x7c00)==0x7c00)continue; // model weights must be finite
        for(unsigned r=0;r<8;r++) {
            uint8_t *p=data+r*210;
            for(unsigned i=0;i<208;i++){seed=seed*1664525+1013904223;p[i]=seed>>24;}
            // Include both extrema, zero, and both signs in every block.
            p[192]=128;p[193]=127;p[194]=0;p[195]=255;p[196]=1;
            unsigned half=base+r;p[208]=half&255;p[209]=half>>8;
            infinity_qwen_scalar_dot(14,p,input,256,expected+r);
            assert(isfinite(expected[r]));
        }
        for(unsigned rows=0;rows<=8;rows++) {
            for(unsigned r=0;r<9;r++)actual[r]=123;
            infinity_qwen_dot_rows(14,data,input,256,rows,actual);
            assert(memcmp(expected,actual,rows*sizeof(float))==0);
            for(unsigned r=rows;r<9;r++)assert(actual[r]==123);
        }
    }
    return 0;
}
