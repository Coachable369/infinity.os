#include <stdio.h>
#include <stdint.h>

typedef struct Writer {
    char *output;
    size_t size;
    size_t length;
} Writer;

// ------------------------=
// FUNC: emit
// DESC: Records one formatted byte while retaining the total required length.
// ------------------=
static void emit(Writer *writer, char value) {
    if (writer->size && writer->length + 1 < writer->size) writer->output[writer->length] = value;
    ++writer->length;
}

// ------------------------=
// FUNC: emit_text
// DESC: Records one terminated string through the bounded writer.
// ------------------=
static void emit_text(Writer *writer, const char *text) {
    if (!text) text = "(null)";
    while (*text) emit(writer, *text++);
}

// ------------------------=
// FUNC: emit_unsigned
// DESC: Formats one unsigned value in a caller-selected radix.
// ------------------=
static void emit_unsigned(Writer *writer, uint64_t value, unsigned base) {
    char digits[32]; size_t length = 0;
    do { unsigned digit = (unsigned)(value % base); digits[length++] = "0123456789abcdef"[digit]; value /= base; } while (value);
    while (length) emit(writer, digits[--length]);
}

// ------------------------=
// FUNC: vsnprintf
// DESC: Formats the production SDK's bounded integer, pointer, character and string conversion set.
// ------------------=
int vsnprintf(char *output, size_t size, const char *format, va_list arguments) {
    if (!format || (size && !output)) return -1;
    Writer writer = {output, size, 0};
    while (*format) {
        if (*format++ != '%') { emit(&writer, format[-1]); continue; }
        if (*format == '%') { emit(&writer, *format++); continue; }
        int long_value = 0, long_long_value = 0, size_value = 0;
        if (*format == 'z') { size_value = 1; ++format; }
        else if (*format == 'l') {
            long_value = 1; ++format;
            if (*format == 'l') { long_long_value = 1; ++format; }
        }
        char conversion = *format ? *format++ : 0;
        if (conversion == 's') emit_text(&writer, va_arg(arguments, const char *));
        else if (conversion == 'c') emit(&writer, (char)va_arg(arguments, int));
        else if (conversion == 'd' || conversion == 'i') {
            int64_t value = size_value ? (int64_t)va_arg(arguments, ptrdiff_t) :
                (long_long_value ? (int64_t)va_arg(arguments, long long) :
                (long_value ? (int64_t)va_arg(arguments, long) : (int64_t)va_arg(arguments, int)));
            uint64_t magnitude = (uint64_t)value;
            if (value < 0) { emit(&writer, '-'); magnitude = 0u - magnitude; }
            emit_unsigned(&writer, magnitude, 10);
        } else if (conversion == 'u' || conversion == 'x' || conversion == 'X') {
            uint64_t value = size_value ? (uint64_t)va_arg(arguments, size_t) :
                (long_long_value ? (uint64_t)va_arg(arguments, unsigned long long) :
                (long_value ? (uint64_t)va_arg(arguments, unsigned long) : (uint64_t)va_arg(arguments, unsigned)));
            emit_unsigned(&writer, value, conversion == 'u' ? 10 : 16);
        } else if (conversion == 'p') {
            emit_text(&writer, "0x"); emit_unsigned(&writer, (uintptr_t)va_arg(arguments, void *), 16);
        } else return -1;
    }
    if (size) writer.output[writer.length < size ? writer.length : size - 1] = 0;
    return writer.length > 0x7fffffffU ? -1 : (int)writer.length;
}

// ------------------------=
// FUNC: snprintf
// DESC: Formats into caller-owned bounded storage.
// ------------------=
int snprintf(char *output, size_t size, const char *format, ...) {
    va_list arguments; va_start(arguments, format); int result = vsnprintf(output, size, format, arguments); va_end(arguments); return result;
}

// ------------------------=
// FUNC: printf
// DESC: Formats one bounded console record and writes it through the native output capability.
// ------------------=
int printf(const char *format, ...) {
    char output[1024]; va_list arguments; va_start(arguments, format);
    int result = vsnprintf(output, sizeof output, format, arguments); va_end(arguments);
    if (result < 0 || (size_t)result >= sizeof output) return EOF;
    for (int index = 0; index < result; ++index) if (putchar((unsigned char)output[index]) == EOF) return EOF;
    return result;
}
