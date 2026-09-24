#include <stdint.h>
#include <stddef.h>
#if defined(__aarch64__) && !defined(QWEN_SCALAR)
#include <arm_neon.h>
#endif
#define QWEN_MAX_WIDTH 12288
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
#if defined(__aarch64__) && !defined(QWEN_SCALAR)
// ------------------------=
// FUNC: qwen_dotprod_available
// DESC: Guards freestanding integer-dot execution on ARM CPUs that advertise the extension.
// ------------------=
static int qwen_dotprod_available(void) {
#if defined(__STDC_HOSTED__) && __STDC_HOSTED__
#if defined(__ARM_FEATURE_DOTPROD)
    return 1;
#else
    return 0;
#endif
#else
    uint64_t features;
    __asm__ volatile("mrs %0, id_aa64isar0_el1":"=r"(features));
    return ((features>>44)&15)>=1;
#endif
}

// ------------------------=
// FUNC: qwen_quantize_q8
// DESC: Quantizes one activation vector into per-block Q8 values for fused Q4_K dot products.
// ------------------=
static void qwen_quantize_q8(const float *input,size_t width,int8_t *values,float *scales,float *totals) {
    for(size_t block=0;block<width/256;block++) {
        const float *x=input+block*256;
        float32x4_t maximum=vdupq_n_f32(0);
        for(unsigned lane=0;lane<256;lane+=16) {
            maximum=vmaxq_f32(maximum,vabsq_f32(vld1q_f32(x+lane)));
            maximum=vmaxq_f32(maximum,vabsq_f32(vld1q_f32(x+lane+4)));
            maximum=vmaxq_f32(maximum,vabsq_f32(vld1q_f32(x+lane+8)));
            maximum=vmaxq_f32(maximum,vabsq_f32(vld1q_f32(x+lane+12)));
        }
        float scale=vmaxvq_f32(maximum)*(1.0f/127.0f);
        scales[block]=scale;
        float inverse=scale==0.0f?0.0f:1.0f/scale;
        for(unsigned lane=0;lane<256;lane+=16) {
            int32x4_t a=vcvtnq_s32_f32(vmulq_n_f32(vld1q_f32(x+lane),inverse));
            int32x4_t b=vcvtnq_s32_f32(vmulq_n_f32(vld1q_f32(x+lane+4),inverse));
            int32x4_t c=vcvtnq_s32_f32(vmulq_n_f32(vld1q_f32(x+lane+8),inverse));
            int32x4_t d=vcvtnq_s32_f32(vmulq_n_f32(vld1q_f32(x+lane+12),inverse));
            int16x8_t low=vcombine_s16(vqmovn_s32(a),vqmovn_s32(b));
            int16x8_t high=vcombine_s16(vqmovn_s32(c),vqmovn_s32(d));
            vst1q_s8(values+block*256+lane,vcombine_s8(vqmovn_s16(low),vqmovn_s16(high)));
        }
        for(unsigned g=0;g<8;g++) {
            const int8_t *xq=values+block*256+g*32;
            totals[block*8+g]=(float)(vaddlvq_s8(vld1q_s8(xq))+vaddlvq_s8(vld1q_s8(xq+16)));
        }
    }
}

// ------------------------=
// FUNC: qwen_q4_q8_rows
// DESC: Reuses activation group totals across Q4_K rows without changing their floating-point accumulation order.
// ------------------=
__attribute__((target("dotprod")))
static void qwen_q4_q8_rows(const uint8_t *data,const int8_t *input,const float *scales,const float *totals,
                            size_t width,size_t rows,float *output) {
    size_t stride=width/256*144;
    for(size_t row=0;row<rows;row++) {
        float sum=0.0f;
        for(size_t block=0;block<width/256;block++) {
            const uint8_t *p=data+row*stride+block*144;
            const int8_t *x=input+block*256;
            float d=qwen_half(p),m=qwen_half(p+2),xd=scales[block];
            const uint8_t *s=p+4;
            for(unsigned g=0;g<8;g++) {
                unsigned scale=g<4?s[g]&63:(s[g+4]&15)|((s[g-4]>>6)<<4);
                unsigned minimum=g<4?s[g+4]&63:(s[g+4]>>4)|((s[g]>>6)<<4);
                const uint8_t *packed=p+16+(g/2)*32;
                int32x4_t products=vdupq_n_s32(0);
                float total=totals[block*8+g];
                for(unsigned lane=0;lane<32;lane+=16) {
                    uint8x16_t nibble=vld1q_u8(packed+lane);
                    nibble=(g&1)?vshrq_n_u8(nibble,4):vandq_u8(nibble,vdupq_n_u8(15));
                    int8x16_t activation=vld1q_s8(x+g*32+lane);
                    products=vdotq_s32(products,vreinterpretq_s8_u8(nibble),activation);
                }
                int dot=vaddvq_s32(products);
                sum+=xd*(d*(float)scale*(float)dot-m*(float)minimum*total);
            }
        }
        output[row]=sum;
    }
}
#endif
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
            for(unsigned part=0;part<2;part++)for(unsigned g=0;g<4;g++)for(unsigned lane=0;lane<32;lane+=8) {
                const uint8_t *lo=p+part*64+(g%2)*32+lane,*hi=p+128+part*32+lane;
                uint8x8_t low=vld1_u8(lo),high=vld1_u8(hi);
                low=g<2?vand_u8(low,vdup_n_u8(15)):vshr_n_u8(low,4);
                high=vand_u8(vshl_u8(high,vdup_n_s8(-(int)(g*2))),vdup_n_u8(3));
                int16x8_t q=vsubq_s16(vreinterpretq_s16_u16(vmovl_u8(vorr_u8(low,vshl_n_u8(high,4)))),vdupq_n_s16(32));
                float ds=d*(float)(int8_t)p[192+part*8+g*2+lane/16];
                float32x4_t weight=vmulq_n_f32(vcvtq_f32_s32(vmovl_s16(vget_low_s16(q))),ds);
                accum=vaddq_f32(accum,vmulq_f32(weight,vld1q_f32(x+part*128+g*32+lane)));
                weight=vmulq_n_f32(vcvtq_f32_s32(vmovl_s16(vget_high_s16(q))),ds);
                accum=vaddq_f32(accum,vmulq_f32(weight,vld1q_f32(x+part*128+g*32+lane+4)));
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

// ------------------------=
// FUNC: infinity_qwen_dot_rows
// DESC: Interleaves four independent rows, reusing activations while preserving each row's floating-point accumulation order.
// ------------------=
void infinity_qwen_dot_rows(uint32_t kind,const uint8_t *data,const float *input,size_t width,size_t rows,float *output) {
    size_t stride=width/256*(kind==12?144:210),row=0;
#if defined(__aarch64__) && !defined(QWEN_SCALAR)
    if(kind==12 && width<=QWEN_MAX_WIDTH && qwen_dotprod_available()) {
        int8_t quantized[QWEN_MAX_WIDTH];
        float scales[QWEN_MAX_WIDTH/256];
        float totals[QWEN_MAX_WIDTH/32];
        qwen_quantize_q8(input,width,quantized,scales,totals);
        qwen_q4_q8_rows(data,quantized,scales,totals,width,rows,output);
        return;
    }
#endif
#if defined(__aarch64__) && !defined(QWEN_SCALAR)
    for(;(kind==12 || kind==14) && row+4<=rows;row+=4) {
        float32x4_t accum[4]={vdupq_n_f32(0),vdupq_n_f32(0),vdupq_n_f32(0),vdupq_n_f32(0)};
        for(size_t block=0;block<width/256;block++) {
            const uint8_t *p[4]; float d[4],m[4];
            for(unsigned r=0;r<4;r++) {
                p[r]=data+(row+r)*stride+block*(kind==12?144:210);
                d[r]=qwen_half(p[r]+(kind==12?0:208));
                m[r]=kind==12?qwen_half(p[r]+2):0;
            }
            const float *x=input+block*256;
            if(kind==12) {
                for(unsigned g=0;g<8;g++) {
                    float ds[4],dm[4];
                    for(unsigned r=0;r<4;r++) {
                        const uint8_t *s=p[r]+4;
                        unsigned scale=g<4?s[g]&63:(s[g+4]&15)|((s[g-4]>>6)<<4);
                        unsigned minimum=g<4?s[g+4]&63:(s[g+4]>>4)|((s[g]>>6)<<4);
                        ds[r]=d[r]*(float)scale; dm[r]=m[r]*(float)minimum;
                    }
                    for(unsigned lane=0;lane<32;lane+=8) {
                        float32x4_t activation=vld1q_f32(x+g*32+lane);
                        float32x4_t activation_hi=vld1q_f32(x+g*32+lane+4);
                        #pragma clang loop unroll(full)
                        for(unsigned r=0;r<4;r++) {
                            const uint8_t *v=p[r]+16+(g/2)*32+lane;
                            uint8x8_t packed=vld1_u8(v);
                            uint8x8_t q=(g&1)?vshr_n_u8(packed,4):vand_u8(packed,vdup_n_u8(15));
                            uint16x8_t wide=vmovl_u8(q);
                            float32x4_t weight=vsubq_f32(vmulq_n_f32(vcvtq_f32_u32(vmovl_u16(vget_low_u16(wide))),ds[r]),vdupq_n_f32(dm[r]));
                            float32x4_t weight_hi=vsubq_f32(vmulq_n_f32(vcvtq_f32_u32(vmovl_u16(vget_high_u16(wide))),ds[r]),vdupq_n_f32(dm[r]));
                            accum[r]=vaddq_f32(accum[r],vmulq_f32(weight,activation));
                            accum[r]=vaddq_f32(accum[r],vmulq_f32(weight_hi,activation_hi));
                        }
                    }
                }
            } else {
                // Each Q6 scale covers sixteen weights. Decode that group once
                // per row and share its activations across all four rows.
                // Keep four separate adds in original order (no FMA/reduction
                // reassociation), so batched and single-row outputs stay exact.
                for(unsigned part=0;part<2;part++)for(unsigned g=0;g<4;g++)for(unsigned lane=0;lane<32;lane+=16) {
                    float32x4_t activation=vld1q_f32(x+part*128+g*32+lane);
                    float32x4_t activation_hi=vld1q_f32(x+part*128+g*32+lane+4);
                    float32x4_t activation_2=vld1q_f32(x+part*128+g*32+lane+8);
                    float32x4_t activation_3=vld1q_f32(x+part*128+g*32+lane+12);
                    #pragma clang loop unroll(full)
                    for(unsigned r=0;r<4;r++) {
                        const uint8_t *lo=p[r]+part*64+(g%2)*32+lane;
                        const uint8_t *hi=p[r]+128+part*32+lane;
                        uint8x16_t low=vld1q_u8(lo),high=vld1q_u8(hi);
                        low=g<2?vandq_u8(low,vdupq_n_u8(15)):vshrq_n_u8(low,4);
                        high=vandq_u8(vshlq_u8(high,vdupq_n_s8(-(int)(g*2))),vdupq_n_u8(3));
                        uint8x16_t packed=vorrq_u8(low,vshlq_n_u8(high,4));
                        int16x8_t q=vsubq_s16(vreinterpretq_s16_u16(vmovl_u8(vget_low_u8(packed))),vdupq_n_s16(32));
                        int16x8_t q_hi=vsubq_s16(vreinterpretq_s16_u16(vmovl_u8(vget_high_u8(packed))),vdupq_n_s16(32));
                        float ds=d[r]*(float)(int8_t)p[r][192+part*8+g*2+lane/16];
                        float32x4_t weight=vmulq_n_f32(vcvtq_f32_s32(vmovl_s16(vget_low_s16(q))),ds);
                        float32x4_t weight_hi=vmulq_n_f32(vcvtq_f32_s32(vmovl_s16(vget_high_s16(q))),ds);
                        accum[r]=vaddq_f32(accum[r],vmulq_f32(weight,activation));
                        accum[r]=vaddq_f32(accum[r],vmulq_f32(weight_hi,activation_hi));
                        weight=vmulq_n_f32(vcvtq_f32_s32(vmovl_s16(vget_low_s16(q_hi))),ds);
                        weight_hi=vmulq_n_f32(vcvtq_f32_s32(vmovl_s16(vget_high_s16(q_hi))),ds);
                        accum[r]=vaddq_f32(accum[r],vmulq_f32(weight,activation_2));
                        accum[r]=vaddq_f32(accum[r],vmulq_f32(weight_hi,activation_3));
                    }
                }
            }
        }
        for(unsigned r=0;r<4;r++) {
            float sums[4]; vst1q_f32(sums,accum[r]);
            output[row+r]=(sums[0]+sums[1])+(sums[2]+sums[3]);
        }
    }
#endif
    for(;row<rows;row++) infinity_qwen_dot(kind,data+row*stride,input,width,output+row);
}
