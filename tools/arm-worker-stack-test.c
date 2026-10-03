#include <stdint.h>
#include <stddef.h>
#define INFINITY_PSCI_TEST
typedef void EFI_SYSTEM_TABLE;
#include "../boot/common/psci_workers.h"

static uint8_t native_stack[32768] __attribute__((aligned(4096)));
static uint8_t emergency_stack[16384] __attribute__((aligned(4096)));
static uint8_t caller_stack[32768] __attribute__((aligned(4096)));
static uint64_t reports[3][24];
uint64_t fault_record[4];
static uint64_t failures;
// ------------------------=
// FUNC: run_mp_case
// DESC: Executes the assembly ABI-state observer around the production MP trampoline.
// ------------------=
extern void run_mp_case(InfinityApContext *, uint64_t, uint64_t *, uint64_t);
// ------------------------=
// FUNC: test_callback
// DESC: Runs the exact-register test callback on the native worker stack.
// ------------------=
extern void test_callback(void *);
// ------------------------=
// FUNC: test_fault_instruction
// DESC: Provides the known fault PC for comparison with the captured architectural ELR.
// ------------------=
extern void test_fault_instruction(void);

// ------------------------=
// FUNC: require
// DESC: Accumulates structured guest assertion failures without depending on diagnostic prose.
// ------------------=
static void require(int condition) { failures += !condition; }
// ------------------------=
// FUNC: write_word
// DESC: Emits fixed-width binary results through the disposable guest UART.
// ------------------=
static void write_word(uint64_t value) {
    for (size_t i=0;i<8;++i) {
        while (*(volatile uint32_t *)(uintptr_t)0x09000018 & 32) {}
        *(volatile uint32_t *)(uintptr_t)0x09000000 = (uint8_t)(value>>(i*8));
    }
}
// ------------------------=
// FUNC: assert_worker
// DESC: Verifies native and emergency stacks are disjoint, valid, aligned, and usable for the original synchronous fault.
// ------------------=
static void assert_worker(uint64_t *r, uint64_t count) {
    require(r[0] == (uintptr_t)emergency_stack + sizeof(emergency_stack));
    require(r[1] > (uintptr_t)native_stack && r[1] <= (uintptr_t)native_stack + sizeof(native_stack));
    require((r[0] & 15) == 0 && (r[1] & 15) == 0);
    require(r[2] == 1 && (r[3] & 0x3c0) == 0x3c0 && r[4] == (uintptr_t)r);
    require(r[9] == 0xc0fe);
    require(fault_record[0] == (uintptr_t)test_fault_instruction);
    require(fault_record[1] == UINT64_C(0xf2000123));
    require(fault_record[2] == r[0] - 0x330 && fault_record[3] == count);
}
// ------------------------=
// FUNC: psci_finish
// DESC: Asserts the real cold-AP entry stack invariant, then powers off the isolated guest with a binary result.
// ------------------=
static void psci_finish(void *arg) {
    test_callback(arg);
    assert_worker(arg, 3);
    write_word(UINT64_C(0x494e465350303031));
    write_word(failures);
    write_word(fault_record[3]);
    for (size_t i=0;i<3;++i) for (size_t j=0;j<5;++j) write_word(reports[i][j]);
    __asm__ volatile("mov x0, #0x8; movk x0, #0x8400, lsl #16; hvc #0" ::: "x0", "memory");
    for (;;) __asm__ volatile("wfe");
}
// ------------------------=
// FUNC: test_main
// DESC: Exercises both MP incoming stack modes and the production PSCI entry without replacing either implementation.
// ------------------=
void test_main(void) {
    InfinityApContext *c = &psci_contexts[0];
    c->stack = (uintptr_t)native_stack + sizeof(native_stack);
    c->exception_stack = (uintptr_t)emergency_stack + sizeof(emergency_stack);
    c->procedure = (uintptr_t)test_callback;
    for (size_t mode=0;mode<2;++mode) {
        uint64_t *r=reports[mode];
        c->argument=(uintptr_t)r;
        run_mp_case(c,mode,r,(uintptr_t)caller_stack+sizeof(caller_stack));
        assert_worker(r,mode+1);
        require(r[5]==r[16] && r[6]==r[15] && r[7]==mode && r[8]==0x340);
        for (size_t i=0;i<5;++i) require(r[10+i]==r[17+i]);
    }
    c->procedure=(uintptr_t)psci_finish;
    c->argument=(uintptr_t)reports[2];
    c->level=4;
    __asm__ volatile("mrs %0,sctlr_el1" : "=r"(c->sctlr));
    __asm__ volatile("mrs %0,tcr_el1" : "=r"(c->tcr));
    __asm__ volatile("mrs %0,ttbr0_el1" : "=r"(c->ttbr0));
    __asm__ volatile("mrs %0,ttbr1_el1" : "=r"(c->ttbr1));
    __asm__ volatile("mrs %0,mair_el1" : "=r"(c->mair));
    __asm__ volatile("mrs %0,vbar_el1" : "=r"(c->vbar));
    __asm__ volatile("msr sp_el0,xzr; mov x0,%0; b infinity_ap_entry" :: "r"(c) : "x0", "memory");
    __builtin_unreachable();
}
