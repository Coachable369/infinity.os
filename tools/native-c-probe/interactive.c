#include <stdio.h>
#include <stdlib.h>
#include <infinity/console.h>
// ------------------------=
// FUNC: main
// DESC: Exercises repeated native terminal input, arithmetic and output with an exit value derived only from delivered input.
// ------------------=
int main(void) {
    char line[64];
    long result = 0;
    for (;;) {
        puts("Enter an expression, or quit:");
        if (infinity_read_line(line, sizeof line) < 0) return 92;
        if (line[0] == 'q') return (int)result;
        char *end;
        long first = strtol(line, &end, 10);
        while (*end == ' ') ++end;
        char operation = *end++;
        long second = strtol(end, &end, 10);
        if (operation == '+') result = first + second;
        else if (operation == '*') result = first * second;
        else return 93;
        printf("%ld\n", result);
    }
}
