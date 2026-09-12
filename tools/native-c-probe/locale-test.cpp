#include <regex>
#include <ctype.h>

// ------------------------=
// FUNC: finish
// DESC: Reports the native library probe result through QEMU's structured exit status.
// ------------------=
[[noreturn]] static void finish(unsigned value) {
    asm volatile("outl %0, %1" : : "a"(value), "Nd"((unsigned short)0xf4));
    for (;;) asm volatile("pause");
}

// ------------------------=
// FUNC: infinity_kernel_entry
// DESC: Exercises the linked libc++ regex table and Newlib character classification without a host OS.
// ------------------=
extern "C" [[noreturn]] void infinity_kernel_entry(const void *) {
    const auto blank = std::__get_classname("blank", false);
    const auto printable = std::__get_classname("print", false);
    const auto word = std::__get_classname("w", false);
    const auto unknown = std::__get_classname("not-a-class", false);
    if (blank != 0x80 || printable != 0x97 || unknown != 0) finish(0x12);
    if ((blank & std::regex_traits<char>::__regex_word) != 0 ||
        (printable & std::regex_traits<char>::__regex_word) != 0 ||
        (word & std::regex_traits<char>::__regex_word) == 0) finish(0x13);
    volatile int space = ' ', letter = 'A', control = '\n';
    if (!isblank(space) || !isprint(letter) || isprint(control)) finish(0x14);
    if ((std::__get_classname("lower", true) & (_U | _L)) != (_U | _L)) finish(0x15);
    finish(0x10);
}
