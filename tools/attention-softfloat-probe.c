// Standalone QEMU diagnostic, not an installed-desktop latency benchmark.
// Links the actual kernel's soft-float compiler builtins as the old arithmetic.
#include <stdint.h>
#include <stddef.h>
extern uint32_t __mulsf3(uint32_t, uint32_t);
extern uint32_t __addsf3(uint32_t, uint32_t);
extern uint32_t __divsf3(uint32_t, uint32_t);
void infinity_attention_scores(const float *, const float *, size_t, size_t, float *, float *);
void infinity_attention_values(const float *, const float *, const float *, size_t, size_t, float *);
static float kv[4096*2048], query[128], scores[4096], expected[4096];
static float output[128], expected_output[128];
// ------------------------=
// FUNC: bits
// DESC: Reinterprets IEEE storage for the kernel soft-float integer-register ABI.
// ------------------=
static uint32_t bits(float value) { union {float f;uint32_t u;} v={.f=value}; return v.u; }
// ------------------------=
// FUNC: number
// DESC: Reinterprets the soft-float result without a numerical conversion.
// ------------------=
static float number(uint32_t value) { union {float f;uint32_t u;} v={.u=value}; return v.f; }
// ------------------------=
// FUNC: ticks
// DESC: Reads QEMU's architectural virtual counter with instruction ordering.
// ------------------=
static uint64_t ticks(void) { uint64_t t; __asm__ volatile("isb\nmrs %0,cntvct_el0":"=r"(t)::"memory"); return t; }
// ------------------------=
// FUNC: put
// DESC: Writes one diagnostic byte to the test machine's PL011 UART.
// ------------------=
static void put(char c) { *(volatile uint32_t *)0x09000000=c; }
// ------------------------=
// FUNC: integer
// DESC: Prints an unsigned numeric measurement without libc.
// ------------------=
static void integer(uint64_t n) { if(n>=10) integer(n/10); put('0'+n%10); }
// ------------------------=
// FUNC: stop
// DESC: Returns a behavioral pass or failure code through QEMU semihosting.
// ------------------=
static void stop(uint64_t code) {
    if (code == 0) {
        // PSCI shutdown is supported by HVF as well as TCG; semihosting exit
        // is not consistently handled by HVF. Only passing tests reach this.
        register uint64_t shutdown __asm__("x0")=0x84000008;
        __asm__ volatile("hvc #0"::"r"(shutdown):"memory");
    }
    uint64_t args[2]={0x20026,code};
    register uint64_t operation __asm__("x0")=0x20;
    register uint64_t argument __asm__("x1")=(uint64_t)args;
    __asm__ volatile("hlt #0xf000"::"r"(operation),"r"(argument):"memory");
    for(;;) __asm__ volatile("wfe");
}
// ------------------------=
// FUNC: main
// DESC: Compares real software-FP builtins with native attention at growing context lengths; asserts bit equality.
// ------------------=
void main(void) {
    for(size_t i=0;i<4096*2048;i++) kv[i]=((int)(i*17%251)-125)/127.0f;
    for(size_t i=0;i<128;i++) query[i]=((int)(i*31%127)-63)/63.0f;
    size_t counts[]={16,128,512,2048,4096};
    uint64_t frequency; __asm__ volatile("mrs %0,cntfrq_el0":"=r"(frequency));
    for(size_t c=0;c<5;c++) {
        size_t count=counts[c]; float maximum;
        uint64_t start=ticks();
        for(size_t t=0;t<count;t++) {
            uint32_t dot=bits(-0.0f);
            for(size_t i=0;i<128;i++) dot=__addsf3(dot,__mulsf3(bits(query[i]),bits(kv[t*2048+i])));
            expected[t]=number(__mulsf3(dot,bits(0.08838834764831845f)));
        }
        uint64_t old_scores=ticks()-start;
        start=ticks(); infinity_attention_scores(query,kv,count,2048,scores,&maximum);
        uint64_t new_scores=ticks()-start;
        for(size_t t=0;t<count;t++) if(bits(scores[t])!=bits(expected[t])) stop(1);
        // Fixed positive softmax weights isolate value accumulation from expf.
        float sum=0;
        for(size_t t=0;t<count;t++) { scores[t]=1.0f+(float)(t%17)/17.0f; sum+=scores[t]; }
        for(size_t i=0;i<128;i++) expected_output[i]=0;
        start=ticks();
        for(size_t t=0;t<count;t++) {
            uint32_t weight=__divsf3(bits(scores[t]),bits(sum));
            for(size_t i=0;i<128;i++) expected_output[i]=number(__addsf3(bits(expected_output[i]),__mulsf3(weight,bits(kv[t*2048+1024+i]))));
        }
        uint64_t old_values=ticks()-start;
        start=ticks(); infinity_attention_values(kv+1024,scores,&sum,count,2048,output);
        uint64_t new_values=ticks()-start;
        for(size_t i=0;i<128;i++) if(bits(output[i])!=bits(expected_output[i])) stop(2);
        integer(count); put(','); integer(old_scores*1000000000/frequency); put(',');
        integer(new_scores*1000000000/frequency); put(','); integer(old_values*1000000000/frequency);
        put(','); integer(new_values*1000000000/frequency); put('\n');
    }
    stop(0);
}
// ------------------------=
// FUNC: _start
// DESC: Establishes a test stack and FP access before entering the standalone probe.
// ------------------=
__asm__(".section .text.entry,\"ax\"\n.global _start\n_start:\n"
        "ldr x0,=0x47000000\nmov sp,x0\nmsr daifset,#15\n"
        "mov x0,#0x300000\nmsr cpacr_el1,x0\nisb\nbl main\nb .\n");
