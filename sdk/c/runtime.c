#include <infinity/app.h>
#include <stdio.h>

static const InfinityAppApi *api;
extern int main(void);

// ------------------------=
// FUNC: putchar
// DESC: Writes one unsigned byte through the granted native console endpoint.
// ------------------=
int putchar(int character) {
    unsigned char byte = (unsigned char)character;
    if (!api || api->write_stdout(api->context, &byte, 1) != 1) return EOF;
    return byte;
}

// ------------------------=
// FUNC: puts
// DESC: Writes a null-terminated string and newline through the native console endpoint.
// ------------------=
int puts(const char *text) {
    if (!text || !api) return EOF;
    size_t length = 0;
    while (text[length]) ++length;
    if (api->write_stdout(api->context, (const unsigned char *)text, length) != (int)length)
        return EOF;
    return putchar('\n') == EOF ? EOF : 0;
}

// ------------------------=
// FUNC: infinity_app_entry
// DESC: Validates the native application ABI and returns main's exit status to its caller.
// ------------------=
int infinity_app_entry(const InfinityAppApi *host) {
    if (!host || host->version != INFINITY_APP_ABI || host->size < sizeof(*host) || !host->write_stdout)
        return 126;
    api = host;
    int result = main();
    api = 0;
    return result;
}
