/* Diagnostic entry for an unchanged, fully linked ARM installed kernel. */
#include <stdint.h>
#include "../../../boot/common/boot_info.h"
#include "linked-symbols.h"
extern void linked_vectors(void);
typedef void (*Task)(void);
typedef void (*Ap)(void *);

// ------------------------=
// FUNC: emit
// DESC: Emits a fixed-width binary diagnostic through the disposable VirtualBox UART.
// ------------------=
static void emit(uint64_t value) {
    for (unsigned i=0; i<8; ++i) {
        while (*(volatile uint32_t *)0xffddf018 & 32) {}
        *(volatile uint32_t *)0xffddf000 = (value >> (8*i)) & 255;
    }
}
// ------------------------=
// FUNC: capture_stack
// DESC: Exports the fault stack as bounded hex words, avoiding binary serial framing ambiguity.
// ------------------=
void capture_stack(uintptr_t *stack) {
    for (unsigned i=0;i<512;i++) {
        uint64_t value=stack[i];
        for (unsigned j=0;j<16;j++) {
            while (*(volatile uint32_t *)0xffddf018 & 32) {}
            *(volatile uint32_t *)0xffddf000 = "0123456789abcdef"[(value >> (60-j*4)) & 15];
        }
        while (*(volatile uint32_t *)0xffddf018 & 32) {}
        *(volatile uint32_t *)0xffddf000 = '\n';
    }
    for (;;) __asm__ volatile("wfe");
}
// ------------------------=
// FUNC: finish
// DESC: Powers off only the disposable diagnostic guest after exporting its result.
// ------------------=
static void finish(void) {
    register uint64_t call __asm__("x0") = 0x84000008;
    __asm__ volatile("hvc #0" : "+r"(call));
    for (;;) {}
}
// ------------------------=
// FUNC: linked_fault
// DESC: Captures the first CPU exception before the firmware handler can obscure its origin.
// ------------------=
void linked_fault(void) {
    uint64_t esr,elr,far;
    __asm__ volatile("mrs %0,esr_el1\nmrs %1,elr_el1\nmrs %2,far_el1" : "=r"(esr),"=r"(elr),"=r"(far));
    emit(0x4953504545434841); emit(0); emit(esr); emit(elr); emit(far);
    emit((uintptr_t)__builtin_return_address(0));
    uintptr_t *frame=__builtin_frame_address(0);
    for (unsigned i=0;i<128;i++) emit(frame[i]);
    __asm__ volatile("dsb sy");
    for (;;) __asm__ volatile("wfe");
}
// ------------------------=
// FUNC: speech
// DESC: Runs the actual linked production speech worker including its rate conversion and resident output buffer.
// ------------------=
static void speech(void) {
    __asm__ volatile("msr vbar_el1,%0\nisb" :: "r"(linked_vectors));
    ((Task)SPEECH_WORKER)();
    emit(*(volatile uint64_t *)STATE);
    emit(*(volatile uint64_t *)FRAMES);
    emit(*(volatile uint32_t *)ERROR);
    emit(*(volatile uint64_t *)PEAK);
    for (uint64_t i=0;i<*(volatile uint64_t *)FRAMES;i++) emit(((uint16_t *)PCM)[i]);
    finish();
}
// ------------------------=
// FUNC: linked_entry
// DESC: Starts real firmware workers and exercises the installed image without altering accounts or booting its desktop.
// ------------------=
void linked_entry(InfinityBootInfo *info) {
#ifdef CAPTURE_NATIVE
    ((void (*)(InfinityBootInfo *))KERNEL_ENTRY)(info);
#else
    const char text[]="Hello! How can I assist you today?";
    emit(0x4953504545434841);
    for (unsigned i=0;i<sizeof(text)-1;i++) ((char *)TEXT)[i]=text[i];
    *(uint64_t *)LENGTH=sizeof(text)-1;
    *(uint32_t *)RATE=48000;
    *(uint64_t *)DEADLINE=UINT64_MAX;
    struct Bridge {uint64_t version; uint64_t (*start)(Ap,uint64_t);};
    struct Bridge *bridge=(void *)(uintptr_t)info->worker_bridge;
    bridge->start((Ap)WORKER_ENTRY,7);
    while (!((int (*)(Task))BACKGROUND)(speech)) {}
    for (;;) {}
#endif
}
/* Preserve the faulting call's LR and general registers without touching UART,
 * stack, or any potentially damaged memory mapping. */
__asm__(".balign 2048\n.globl linked_vectors\nlinked_vectors:\n.rept 16\n.balign 128\nb capture_fault\n.endr\n"
        "capture_fault:\nmrs x8,esr_el1\nmrs x9,elr_el1\nmrs x10,far_el1\nmov x11,sp\n1: wfe\nb 1b");
