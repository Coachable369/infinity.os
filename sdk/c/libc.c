#include <ctype.h>
#include <errno.h>
#include <limits.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define HEAP_BYTES (64u * 1024u)
#define ALIGNMENT 16u

typedef struct Block {
    size_t size;
    struct Block *next;
    int free;
} Block;

typedef union Heap {
    max_align_t alignment;
    unsigned char bytes[HEAP_BYTES];
} Heap;

static Heap heap;
static Block *first;
_Thread_local int errno;

// ------------------------=
// FUNC: aligned_size
// DESC: Rounds one allocation to the native maximum-alignment boundary without overflow.
// ------------------=
static size_t aligned_size(size_t size) {
    if (size > SIZE_MAX - (ALIGNMENT - 1)) return 0;
    return (size + ALIGNMENT - 1) & ~(ALIGNMENT - 1);
}

// ------------------------=
// FUNC: initialize_heap
// DESC: Creates one coalescible application-local heap extent on first use.
// ------------------=
static void initialize_heap(void) {
    if (first) return;
    first = (Block *)heap.bytes;
    first->size = HEAP_BYTES - sizeof(Block);
    first->next = 0;
    first->free = 1;
}

// ------------------------=
// FUNC: infinity_libc_reset
// DESC: Resets process-local errno and heap metadata before entering application main.
// ------------------=
void infinity_libc_reset(void) {
    first = 0;
    errno = 0;
    memset(heap.bytes, 0, sizeof heap.bytes);
}

// ------------------------=
// FUNC: memcpy
// DESC: Copies an exact non-overlapping byte range.
// ------------------=
void *memcpy(void *destination, const void *source, size_t length) {
    unsigned char *out = destination;
    const unsigned char *in = source;
    for (size_t index = 0; index < length; ++index) out[index] = in[index];
    return destination;
}

// ------------------------=
// FUNC: memmove
// DESC: Copies a byte range correctly in either overlap direction.
// ------------------=
void *memmove(void *destination, const void *source, size_t length) {
    unsigned char *out = destination;
    const unsigned char *in = source;
    if (out <= in) for (size_t index = 0; index < length; ++index) out[index] = in[index];
    else while (length) { --length; out[length] = in[length]; }
    return destination;
}

// ------------------------=
// FUNC: memset
// DESC: Fills an exact byte range with one unsigned value.
// ------------------=
void *memset(void *destination, int value, size_t length) {
    unsigned char *out = destination;
    for (size_t index = 0; index < length; ++index) out[index] = (unsigned char)value;
    return destination;
}

// ------------------------=
// FUNC: memcmp
// DESC: Lexically compares two bounded byte ranges.
// ------------------=
int memcmp(const void *left, const void *right, size_t length) {
    const unsigned char *a = left, *b = right;
    for (size_t index = 0; index < length; ++index) if (a[index] != b[index]) return a[index] - b[index];
    return 0;
}

// ------------------------=
// FUNC: strlen
// DESC: Counts bytes before the first string terminator.
// ------------------=
size_t strlen(const char *text) { size_t length = 0; while (text[length]) ++length; return length; }

// ------------------------=
// FUNC: strnlen
// DESC: Counts bytes before a terminator without exceeding the caller's bound.
// ------------------=
size_t strnlen(const char *text, size_t limit) { size_t length = 0; while (length < limit && text[length]) ++length; return length; }

// ------------------------=
// FUNC: strcmp
// DESC: Lexically compares two terminated byte strings.
// ------------------=
int strcmp(const char *left, const char *right) {
    while (*left && *left == *right) { ++left; ++right; }
    return (unsigned char)*left - (unsigned char)*right;
}

// ------------------------=
// FUNC: strncmp
// DESC: Lexically compares no more than the requested number of string bytes.
// ------------------=
int strncmp(const char *left, const char *right, size_t length) {
    while (length && *left && *left == *right) { ++left; ++right; --length; }
    return length ? (unsigned char)*left - (unsigned char)*right : 0;
}

// ------------------------=
// FUNC: strcpy
// DESC: Copies one terminated string including its terminator.
// ------------------=
char *strcpy(char *destination, const char *source) {
    char *out = destination; do { *out++ = *source; } while (*source++); return destination;
}

// ------------------------=
// FUNC: strncpy
// DESC: Copies and zero-pads one bounded string region.
// ------------------=
char *strncpy(char *destination, const char *source, size_t length) {
    size_t index = 0;
    for (; index < length && source[index]; ++index) destination[index] = source[index];
    for (; index < length; ++index) destination[index] = 0;
    return destination;
}

// ------------------------=
// FUNC: strcat
// DESC: Appends one terminated string to another.
// ------------------=
char *strcat(char *destination, const char *source) { strcpy(destination + strlen(destination), source); return destination; }

// ------------------------=
// FUNC: strchr
// DESC: Finds the first matching byte including a requested terminator.
// ------------------=
char *strchr(const char *text, int character) {
    char wanted = (char)character; do { if (*text == wanted) return (char *)text; } while (*text++); return 0;
}

// ------------------------=
// FUNC: strrchr
// DESC: Finds the last matching byte including a requested terminator.
// ------------------=
char *strrchr(const char *text, int character) {
    const char *found = 0; char wanted = (char)character;
    do { if (*text == wanted) found = text; } while (*text++); return (char *)found;
}

// ------------------------=
// FUNC: strstr
// DESC: Finds the first complete occurrence of one string inside another.
// ------------------=
char *strstr(const char *text, const char *needle) {
    size_t length = strlen(needle); if (!length) return (char *)text;
    for (; *text; ++text) if (!strncmp(text, needle, length)) return (char *)text;
    return 0;
}

// ------------------------=
// FUNC: malloc
// DESC: Allocates one aligned process-local block with split reuse and a finite heap bound.
// ------------------=
void *malloc(size_t size) {
    size = aligned_size(size);
    if (!size) { errno = ENOMEM; return 0; }
    initialize_heap();
    for (Block *block = first; block; block = block->next) if (block->free && block->size >= size) {
        if (block->size >= size + sizeof(Block) + ALIGNMENT) {
            Block *next = (Block *)((unsigned char *)(block + 1) + size);
            *next = (Block){block->size - size - sizeof(Block), block->next, 1};
            block->next = next; block->size = size;
        }
        block->free = 0;
        return block + 1;
    }
    errno = ENOMEM;
    return 0;
}

// ------------------------=
// FUNC: free
// DESC: Releases an exact heap block and coalesces adjacent free extents.
// ------------------=
void free(void *pointer) {
    if (!pointer) return;
    initialize_heap();
    for (Block *block = first; block; block = block->next) if (block + 1 == pointer) {
        if (block->free) { errno = EINVAL; return; }
        block->free = 1;
        for (Block *scan = first; scan; scan = scan->next) {
            while (scan->free && scan->next && scan->next->free) {
                scan->size += sizeof(Block) + scan->next->size;
                scan->next = scan->next->next;
            }
        }
        return;
    }
    errno = EINVAL;
}

// ------------------------=
// FUNC: calloc
// DESC: Allocates overflow-checked zeroed storage.
// ------------------=
void *calloc(size_t count, size_t size) {
    if (size && count > SIZE_MAX / size) { errno = ENOMEM; return 0; }
    size_t bytes = count * size;
    void *result = malloc(bytes);
    if (result) memset(result, 0, bytes);
    return result;
}

// ------------------------=
// FUNC: realloc
// DESC: Resizes an owned block while preserving the prior allocation on failure.
// ------------------=
void *realloc(void *pointer, size_t size) {
    if (!pointer) return malloc(size);
    if (!size) { free(pointer); return 0; }
    initialize_heap();
    for (Block *block = first; block; block = block->next) if (block + 1 == pointer && !block->free) {
        if (block->size >= size) return pointer;
        void *replacement = malloc(size);
        if (!replacement) return 0;
        memcpy(replacement, pointer, block->size);
        free(pointer);
        return replacement;
    }
    errno = EINVAL;
    return 0;
}

// ------------------------=
// FUNC: digit_value
// DESC: Converts one ASCII digit for integer parsing.
// ------------------=
static int digit_value(int character) {
    if (character >= '0' && character <= '9') return character - '0';
    if (character >= 'a' && character <= 'z') return character - 'a' + 10;
    if (character >= 'A' && character <= 'Z') return character - 'A' + 10;
    return -1;
}

// ------------------------=
// FUNC: parse_unsigned
// DESC: Parses one bounded-base integer with prefix handling and saturation.
// ------------------=
static unsigned long parse_unsigned(const char *text, char **end, int base, int *negative) {
    const char *start = text;
    while (isspace((unsigned char)*text)) ++text;
    *negative = *text == '-'; if (*text == '-' || *text == '+') ++text;
    if ((base == 0 || base == 16) && text[0] == '0' && (text[1] == 'x' || text[1] == 'X')) { base = 16; text += 2; }
    else if (base == 0) base = text[0] == '0' ? 8 : 10;
    if (base < 2 || base > 36) { errno = EINVAL; if (end) *end = (char *)start; return 0; }
    unsigned long value = 0; int any = 0;
    for (;;) {
        int digit = digit_value((unsigned char)*text);
        if (digit < 0 || digit >= base) break;
        any = 1;
        if (value > (ULONG_MAX - (unsigned long)digit) / (unsigned long)base) { value = ULONG_MAX; errno = ERANGE; }
        else value = value * (unsigned long)base + (unsigned long)digit;
        ++text;
    }
    if (end) *end = (char *)(any ? text : start);
    return any ? value : 0;
}

// ------------------------=
// FUNC: strtoul
// DESC: Converts an ASCII integer to unsigned long with standard prefix and end-pointer behavior.
// ------------------=
unsigned long strtoul(const char *text, char **end, int base) {
    int negative = 0; unsigned long value = parse_unsigned(text, end, base, &negative); return negative ? 0ul - value : value;
}

// ------------------------=
// FUNC: strtol
// DESC: Converts an ASCII integer to signed long with range saturation.
// ------------------=
long strtol(const char *text, char **end, int base) {
    int negative = 0; unsigned long value = parse_unsigned(text, end, base, &negative);
    unsigned long limit = negative ? (unsigned long)LONG_MAX + 1ul : (unsigned long)LONG_MAX;
    if (value > limit) { errno = ERANGE; return negative ? LONG_MIN : LONG_MAX; }
    return negative ? (value == (unsigned long)LONG_MAX + 1ul ? LONG_MIN : -(long)value) : (long)value;
}

// ------------------------=
// FUNC: atoi
// DESC: Converts a decimal string using the bounded signed parser.
// ------------------=
int atoi(const char *text) { return (int)strtol(text, 0, 10); }

// ------------------------=
// FUNC: abs
// DESC: Returns the magnitude of one int.
// ------------------=
int abs(int value) { return value < 0 ? -value : value; }

// ------------------------=
// FUNC: labs
// DESC: Returns the magnitude of one long.
// ------------------=
long labs(long value) { return value < 0 ? -value : value; }

// ------------------------=
// FUNC: qsort
// DESC: Sorts fixed-size records in place with bounded temporary storage and no allocation.
// ------------------=
void qsort(void *base, size_t count, size_t size, int (*compare)(const void *, const void *)) {
    unsigned char *bytes = base;
    if (!bytes || !compare || !size) return;
    for (size_t index = 1; index < count; ++index) for (size_t current = index; current && compare(bytes + (current - 1) * size, bytes + current * size) > 0; --current) {
        for (size_t byte = 0; byte < size; ++byte) {
            unsigned char saved = bytes[(current - 1) * size + byte];
            bytes[(current - 1) * size + byte] = bytes[current * size + byte];
            bytes[current * size + byte] = saved;
        }
    }
}

// ------------------------=
// FUNC: bsearch
// DESC: Searches a sorted fixed-record array without pointer overflow.
// ------------------=
void *bsearch(const void *key, const void *base, size_t count, size_t size, int (*compare)(const void *, const void *)) {
    const unsigned char *bytes = base;
    if (!key || !bytes || !compare || !size) return 0;
    size_t first_index = 0;
    while (first_index < count) {
        size_t middle = first_index + (count - first_index) / 2;
        int order = compare(key, bytes + middle * size);
        if (!order) return (void *)(bytes + middle * size);
        if (order < 0) count = middle; else first_index = middle + 1;
    }
    return 0;
}

// ------------------------=
// FUNC: isdigit
// DESC: Tests ASCII decimal digits.
// ------------------=
int isdigit(int character) { return character >= '0' && character <= '9'; }

// ------------------------=
// FUNC: islower
// DESC: Tests ASCII lowercase letters.
// ------------------=
int islower(int character) { return character >= 'a' && character <= 'z'; }

// ------------------------=
// FUNC: isupper
// DESC: Tests ASCII uppercase letters.
// ------------------=
int isupper(int character) { return character >= 'A' && character <= 'Z'; }

// ------------------------=
// FUNC: isalpha
// DESC: Tests ASCII alphabetic characters.
// ------------------=
int isalpha(int character) { return islower(character) || isupper(character); }

// ------------------------=
// FUNC: isalnum
// DESC: Tests ASCII alphanumeric characters.
// ------------------=
int isalnum(int character) { return isalpha(character) || isdigit(character); }

// ------------------------=
// FUNC: isspace
// DESC: Tests standard ASCII whitespace characters.
// ------------------=
int isspace(int character) { return character == ' ' || (character >= '\t' && character <= '\r'); }

// ------------------------=
// FUNC: isxdigit
// DESC: Tests ASCII hexadecimal digits.
// ------------------=
int isxdigit(int character) { return isdigit(character) || (tolower(character) >= 'a' && tolower(character) <= 'f'); }

// ------------------------=
// FUNC: tolower
// DESC: Converts one ASCII uppercase character while preserving other values.
// ------------------=
int tolower(int character) { return isupper(character) ? character + ('a' - 'A') : character; }

// ------------------------=
// FUNC: toupper
// DESC: Converts one ASCII lowercase character while preserving other values.
// ------------------=
int toupper(int character) { return islower(character) ? character - ('a' - 'A') : character; }
