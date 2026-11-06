#include <stdio.h>

// ------------------------=
// FUNC: main
// DESC: Exercises the real native C library's console output and process return value.
// ------------------=
int main(void) {
    return puts("Hello, world!") == EOF ? 1 : 0;
}
