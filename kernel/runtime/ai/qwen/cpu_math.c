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
#if defined(__aarch64__) && !defined(QWEN_SCALAR)
    // ARMv8's base floating-point conversion supports half storage even when
    // the optional half-precision arithmetic extension is unavailable.
    union { uint16_t bits; __fp16 value; } half={.bits=h};
    return (float)half.value;
#else
    uint32_t e=(h>>10)&31,f=h&1023,sign=(uint32_t)(h&32768)<<16;
    union {uint32_t u;float f;} v;
    if(!e)return(sign?-1.0f:1.0f)*(float)f*(1.0f/16777216.0f);
    v.u=sign|(e==31?0x7f800000u|f<<13:(e+112)<<23|f<<13);return v.f;
#endif
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
// DESC: Quantizes one activation vector for fused K-quant dots; optional Q4 offset totals are skipped for Q6.
// ------------------=
static void qwen_quantize_q8(const float *input,size_t width,int8_t *values,float *scales,float *totals) {
    for(size_t block=0;block<width/256;block++) {
        const float *x=input+block*256;
        float32x4_t maximum=vdupq_n_f32(0),maximum1=maximum,maximum2=maximum,maximum3=maximum;
        for(unsigned lane=0;lane<256;lane+=16) {
            maximum=vmaxq_f32(maximum,vabsq_f32(vld1q_f32(x+lane)));
            maximum1=vmaxq_f32(maximum1,vabsq_f32(vld1q_f32(x+lane+4)));
            maximum2=vmaxq_f32(maximum2,vabsq_f32(vld1q_f32(x+lane+8)));
            maximum3=vmaxq_f32(maximum3,vabsq_f32(vld1q_f32(x+lane+12)));
        }
        maximum=vmaxq_f32(vmaxq_f32(maximum,maximum1),vmaxq_f32(maximum2,maximum3));
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
        for(unsigned g=0;totals && g<8;g++) {
            const int8_t *xq=values+block*256+g*32;
            totals[block*8+g]=(float)(vaddlvq_s8(vld1q_s8(xq))+vaddlvq_s8(vld1q_s8(xq+16)));
        }
    }
}

// ------------------------=
// FUNC: qwen_q4_q8_rows
// DESC: Computes Q4_K blocks with exact integer weighted reductions before applying floating point scales.
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
            uint8x8_t s0=vld1_u8(s),s4=vld1_u8(s+4),s8=vld1_u8(s+8);
            // Bounds: weighted <= 256*15*127*63, offset <= 256*127*63.
            // Both fit int32; only final block scaling rounds to float.
            int32x4_t weighted=vdupq_n_s32(0),offset=vdupq_n_s32(0);
            #pragma clang loop unroll(full)
            for(unsigned g=0;g<8;g+=4) {
                uint8x8_t scale=g==0?vand_u8(s0,vdup_n_u8(63)):
                    vorr_u8(vand_u8(s8,vdup_n_u8(15)),vshl_n_u8(vshr_n_u8(s0,6),4));
                uint8x8_t minimum=g==0?vand_u8(s4,vdup_n_u8(63)):
                    vorr_u8(vshr_n_u8(s8,4),vshl_n_u8(vshr_n_u8(s4,6),4));
                const uint8_t *packed=p+16+(g/2)*32;
                int32x4_t dots[4]={vdupq_n_s32(0),vdupq_n_s32(0),vdupq_n_s32(0),vdupq_n_s32(0)};
                #pragma clang loop unroll(full)
                for(unsigned lane=0;lane<32;lane+=16) {
                    uint8x16_t a=vld1q_u8(packed+lane),b=vld1q_u8(packed+32+lane);
                    dots[0]=vdotq_s32(dots[0],vreinterpretq_s8_u8(vandq_u8(a,vdupq_n_u8(15))),vld1q_s8(x+g*32+lane));
                    dots[1]=vdotq_s32(dots[1],vreinterpretq_s8_u8(vshrq_n_u8(a,4)),vld1q_s8(x+(g+1)*32+lane));
                    dots[2]=vdotq_s32(dots[2],vreinterpretq_s8_u8(vandq_u8(b,vdupq_n_u8(15))),vld1q_s8(x+(g+2)*32+lane));
                    dots[3]=vdotq_s32(dots[3],vreinterpretq_s8_u8(vshrq_n_u8(b,4)),vld1q_s8(x+(g+3)*32+lane));
                }
                int32x4_t dot=vpaddq_s32(vpaddq_s32(dots[0],dots[1]),vpaddq_s32(dots[2],dots[3]));
                int32x4_t si=vreinterpretq_s32_u32(vmovl_u16(vget_low_u16(vmovl_u8(scale))));
                int32x4_t mi=vreinterpretq_s32_u32(vmovl_u16(vget_low_u16(vmovl_u8(minimum))));
                weighted=vaddq_s32(weighted,vmulq_s32(si,dot));
                offset=vaddq_s32(offset,vmulq_s32(mi,vcvtq_s32_f32(vld1q_f32(totals+block*8+g))));
            }
            sum+=((float)vaddvq_s32(weighted)*d-(float)vaddvq_s32(offset)*m)*xd;
        }
        output[row]=sum;
    }
}
// ------------------------=
// FUNC: qwen_q6_q8_rows
// DESC: Fuses Q6 decoding with bounded Q8 activation integer dots without materializing float tensors.
// ------------------=
__attribute__((target("dotprod")))
static void qwen_q6_q8_rows(const uint8_t *data,const int8_t *input,const float *scales,
                           size_t width,size_t rows,float *output) {
    size_t stride=width/256*210;
    for(size_t row=0;row<rows;row++) {
        float sum=0;
        for(size_t block=0;block<width/256;block++) {
            const uint8_t *p=data+row*stride+block*210;
            const int8_t *x=input+block*256;
            int32x4_t total=vdupq_n_s32(0);
            for(unsigned part=0;part<2;part++) for(unsigned g=0;g<4;g++) {
                for(unsigned lane=0;lane<32;lane+=16) {
                    uint8x16_t lo=vld1q_u8(p+part*64+(g%2)*32+lane);
                    uint8x16_t hi=vld1q_u8(p+128+part*32+lane);
                    lo=g<2?vandq_u8(lo,vdupq_n_u8(15)):vshrq_n_u8(lo,4);
                    hi=vandq_u8(vshlq_u8(hi,vdupq_n_s8(-(int)(g*2))),vdupq_n_u8(3));
                    int8x16_t q=vsubq_s8(vreinterpretq_s8_u8(vorrq_u8(lo,vshlq_n_u8(hi,4))),vdupq_n_s8(32));
                    int32x4_t dot=vdotq_s32(vdupq_n_s32(0),q,vld1q_s8(x+part*128+g*32+lane));
                    total=vaddq_s32(total,vmulq_n_s32(dot,(int8_t)p[192+part*8+g*2+lane/16]));
                }
            }
            sum+=(float)vaddvq_s32(total)*qwen_half(p+208)*scales[block];
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
#if defined(__aarch64__) && !defined(QWEN_SCALAR) && !defined(QWEN_EXACT_Q6)
    if(kind==14 && width<=QWEN_MAX_WIDTH && qwen_dotprod_available()) {
        int8_t quantized[QWEN_MAX_WIDTH];
        float scales[QWEN_MAX_WIDTH/256];
        qwen_quantize_q8(input,width,quantized,scales,0);
        qwen_q6_q8_rows(data,quantized,scales,width,1,output);
        return;
    }
#endif
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
// DESC: Dispatches shared-activation integer dots on capable ARM CPUs, retaining exact floating-point fallback kernels.
// ------------------=
void infinity_qwen_dot_rows(uint32_t kind,const uint8_t *data,const float *input,size_t width,size_t rows,float *output) {
    size_t stride=width/256*(kind==12?144:210),row=0;
#if defined(__aarch64__) && !defined(QWEN_SCALAR)
    if((kind==12
#ifndef QWEN_EXACT_Q6
        || kind==14
#endif
        ) && width<=QWEN_MAX_WIDTH && qwen_dotprod_available()) {
        int8_t quantized[QWEN_MAX_WIDTH];
        float scales[QWEN_MAX_WIDTH/256];
        float totals[QWEN_MAX_WIDTH/32];
        qwen_quantize_q8(input,width,quantized,scales,kind==12?totals:0);
        if(kind==14) qwen_q6_q8_rows(data,quantized,scales,width,rows,output);
        else qwen_q4_q8_rows(data,quantized,scales,totals,width,rows,output);
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
                        // q * scale fits int16. A finite half's 11-bit
                        // significand times that product fits float32 exactly,
                        // so scaling before conversion preserves each weight.
                        int8x16_t centered=vsubq_s8(vreinterpretq_s8_u8(packed),vdupq_n_s8(32));
                        int8x8_t scale=vdup_n_s8((int8_t)p[r][192+part*8+g*2+lane/16]);
                        int16x8_t q=vmull_s8(vget_low_s8(centered),scale);
                        int16x8_t q_hi=vmull_s8(vget_high_s8(centered),scale);
                        float ds=d[r];
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

// ------------------------=
// FUNC: infinity_qwen_dot_rows_cached
// DESC: Reuses engine-owned activation quantization across cooperative row slices; refresh starts each new matrix.
// ------------------=
void infinity_qwen_dot_rows_cached(uint32_t kind,const uint8_t *data,const float *input,
                                  size_t width,size_t rows,float *output,float *scratch,int refresh) {
#if defined(__aarch64__) && !defined(QWEN_SCALAR)
    if((kind==12
#ifndef QWEN_EXACT_Q6
        || kind==14
#endif
        ) && width && width%256==0 && width<=QWEN_MAX_WIDTH && qwen_dotprod_available()) {
        // The engine provides width floats. Values plus scales and totals use
        // less than two bytes per input element, with naturally aligned floats.
        int8_t *values=(int8_t *)scratch;
        float *scales=(float *)(values+width);
        float *totals=scales+width/256;
        if(refresh) qwen_quantize_q8(input,width,values,scales,kind==12?totals:0);
        if(kind==14) qwen_q6_q8_rows(data,values,scales,width,rows,output);
        else qwen_q4_q8_rows(data,values,scales,totals,width,rows,output);
        return;
    }
#endif
    infinity_qwen_dot_rows(kind,data,input,width,rows,output);
}
