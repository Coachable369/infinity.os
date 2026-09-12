#include <stdint.h>
#include <stddef.h>
#if defined(__aarch64__) && !defined(QWEN_SCALAR)
#include <arm_neon.h>
#endif
// ------------------------=
// FUNC: qwen_half
// DESC: Decodes unaligned little-endian half precision values.
// ------------------=
static float qwen_half(const uint8_t *p) {
    uint16_t h=(uint16_t)p[0]|(uint16_t)p[1]<<8;
    uint32_t e=(h>>10)&31,f=h&1023,sign=(uint32_t)(h&32768)<<16;
    union {uint32_t u;float f;} v;
    if(!e)return(sign?-1.0f:1.0f)*(float)f*(1.0f/16777216.0f);
    v.u=sign|(e==31?0x7f800000u|f<<13:(e+112)<<23|f<<13);return v.f;
}
// ------------------------=
// FUNC: infinity_qwen_dot
// DESC: Fuses K-quant decoding with CPU floating-point dot products through a pointer-only ABI.
// ------------------=
void infinity_qwen_dot(uint32_t kind,const uint8_t *data,const float *input,size_t width,float *output) {
    float sums[4]={0,0,0,0};
#if defined(__aarch64__) && !defined(QWEN_SCALAR)
    float32x4_t accum=vdupq_n_f32(0);
#endif
    for(size_t block=0;block<width/256;block++) {
        const uint8_t *p=data+block*(kind==12?144:210);const float *x=input+block*256;
        if(kind==12) {
            float d=qwen_half(p),m=qwen_half(p+2);const uint8_t *s=p+4;
            for(unsigned g=0;g<8;g++) {
                unsigned scale=g<4?s[g]&63:(s[g+4]&15)|((s[g-4]>>6)<<4);
                unsigned minimum=g<4?s[g+4]&63:(s[g+4]>>4)|((s[g]>>6)<<4);
                float ds=d*(float)scale,dm=m*(float)minimum;
#if defined(__aarch64__) && !defined(QWEN_SCALAR)
                for(unsigned lane=0;lane<32;lane+=4) {
                    const uint8_t *v=p+16+(g/2)*32+lane;
                    uint32x4_t packed={v[0],v[1],v[2],v[3]};
                    uint32x4_t q=(g&1)?vshrq_n_u32(packed,4):vandq_u32(packed,vdupq_n_u32(15));
                    float32x4_t weight=vsubq_f32(vmulq_n_f32(vcvtq_f32_u32(q),ds),vdupq_n_f32(dm));
                    accum=vaddq_f32(accum,vmulq_f32(weight,vld1q_f32(x+g*32+lane)));
                }
#else
                for(unsigned lane=0;lane<32;lane++) {
                    unsigned packed=p[16+(g/2)*32+lane],q=(g&1)?packed>>4:packed&15;
                    sums[lane&3]+=(ds*(float)q-dm)*x[g*32+lane];
                }
#endif
            }
        } else {
            float d=qwen_half(p+208);
#if defined(__aarch64__) && !defined(QWEN_SCALAR)
            for(unsigned part=0;part<2;part++)for(unsigned g=0;g<4;g++)for(unsigned lane=0;lane<32;lane+=4) {
                const uint8_t *lo=p+part*64+(g%2)*32+lane,*hi=p+128+part*32+lane;
                uint32x4_t low={lo[0],lo[1],lo[2],lo[3]},high={hi[0],hi[1],hi[2],hi[3]};
                low=g<2?vandq_u32(low,vdupq_n_u32(15)):vshrq_n_u32(low,4);
                high=vandq_u32(vshlq_u32(high,vdupq_n_s32(-(int)(g*2))),vdupq_n_u32(3));
                int32x4_t q=vsubq_s32(vreinterpretq_s32_u32(vorrq_u32(low,vshlq_n_u32(high,4))),vdupq_n_s32(32));
                float ds=d*(float)(int8_t)p[192+part*8+g*2+lane/16];
                float32x4_t weight=vmulq_n_f32(vcvtq_f32_s32(q),ds);
                accum=vaddq_f32(accum,vmulq_f32(weight,vld1q_f32(x+part*128+g*32+lane)));
            }
#else
            for(unsigned part=0;part<2;part++)for(unsigned g=0;g<4;g++)for(unsigned lane=0;lane<32;lane++) {
                unsigned low=p[part*64+(g%2)*32+lane];low=g<2?low&15:low>>4;
                unsigned high=(p[128+part*32+lane]>>(g*2))&3;
                int q=(int)(low|(high<<4))-32,scale=(int8_t)p[192+part*8+g*2+lane/16];
                sums[lane&3]+=d*(float)scale*(float)q*x[part*128+g*32+lane];
            }
#endif
        }
    }
#if defined(__aarch64__) && !defined(QWEN_SCALAR)
    vst1q_f32(sums,accum);
#endif
    *output=(sums[0]+sums[1])+(sums[2]+sums[3]);
}
