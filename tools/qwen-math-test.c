#include <assert.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>
void infinity_qwen_dot(uint32_t,const uint8_t *,const float *,size_t,float *);
// ------------------------=
// FUNC: main
// DESC: Checks CPU kernel outputs against independently calculated quantization vectors.
// ------------------=
int main(void) {
    uint8_t q4[144]={0},q6[210]={0};float x[256],out=0;
    for(unsigned i=0;i<256;i++)x[i]=1;
    q4[1]=60;memset(q4+4,1,4);memset(q4+12,1,4);memset(q4+16,0x21,128);
    infinity_qwen_dot(12,q4,x,256,&out);assert(out==384);
    memset(q6,0x10,128);memset(q6+192,1,16);q6[209]=60;
    infinity_qwen_dot(14,q6,x,256,&out);assert(out==-8064);
    for(unsigned i=0;i<256;i++)x[i]=(i&1)?-1:1;
    infinity_qwen_dot(12,q4,x,256,&out);assert(out==0);
    infinity_qwen_dot(14,q6,x,256,&out);assert(out==0);
    return 0;
}
