#include <ctype.h>
#include <errno.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../../sdk/c/internal.h"
const InfinityAppApi *infinity_host;

// ------------------------=
// FUNC: compare_int
// DESC: Orders integer records for qsort and bsearch behavior checks.
// ------------------=
static int compare_int(const void *left, const void *right) {
    int a = *(const int *)left;
    int b = *(const int *)right;
    return (a > b) - (a < b);
}

// ------------------------=
// FUNC: main
// DESC: Exercises the production SDK allocator, strings, parsing, sorting, character classes, and bounded formatting.
// ------------------=
int main(void) {
    infinity_libc_reset();
    unsigned char *first = malloc(128);
    unsigned char *second = calloc(64, 2);
    if (!first || !second) return 1;
    for (size_t index = 0; index < 128; ++index) {
        first[index] = (unsigned char)index;
        if (second[index] != 0) return 2;
    }
    free(first);
    free(second);
    unsigned char *joined = malloc(240);
    if (!joined) return 3;
    memset(joined, 0x5a, 240);
    joined = realloc(joined, 512);
    if (!joined) return 4;
    for (size_t index = 0; index < 240; ++index) if (joined[index] != 0x5a) return 5;

    char text[48] = "Infinity";
    strcat(text, "OS");
    if (strlen(text) != 10 || strcmp(text, "InfinityOS") || strstr(text, "ity") != text + 5) return 6;
    memmove(text + 1, text, 10);
    if (strncmp(text, "IInfinityOS", 11)) return 7;

    char *end = 0;
    errno = 0;
    if (strtol(" -0x2a!", &end, 0) != -42 || !end || *end != '!') return 8;
    if (strtoul("184467440737095516160", &end, 10) != ULONG_MAX || errno != ERANGE) return 9;

    int values[] = {9, 2, 7, 1, 4};
    qsort(values, 5, sizeof values[0], compare_int);
    int key = 7;
    int *found = bsearch(&key, values, 5, sizeof values[0], compare_int);
    if (!found || *found != 7 || values[0] != 1 || values[4] != 9) return 10;

    char formatted[64];
    int required = snprintf(formatted, sizeof formatted, "native:%d:%x:%zu:%lld", -42, 42u, strlen(text), -9223372036854775807LL);
    if (required != 37 || strcmp(formatted, "native:-42:2a:11:-9223372036854775807")) return 11;
    char short_buffer[5];
    if (snprintf(short_buffer, sizeof short_buffer, "abcdef") != 6 || strcmp(short_buffer, "abcd")) return 12;
    if (!isalpha('Q') || !isdigit('8') || !isspace('\n') || toupper('z') != 'Z' || tolower('B') != 'b') return 13;

    free(joined);
    errno = 0;
    free(joined);
    if (errno != EINVAL) return 14;
    errno = 0;
    if (malloc(128u * 1024u) || errno != ENOMEM) return 15;
    return 0;
}
