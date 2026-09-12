#include <stdio.h>
#include <stdint.h>

// ------------------------=
// FUNC: main
// DESC: Exercises real C stream functions against the source's current saved ObjectStore version.
// ------------------=
int main(void) {
    FILE *source = fopen("hello.c", "rb");
    if (!source) return 10;
    unsigned char buffer[128];
    size_t length = fread(buffer, 1, sizeof(buffer), source);
    // The VM edited this document after it built this executable on the host.
    const char expected[] = "int main(void) { return 23; }\n";
    if (length != sizeof(expected) - 1 || !feof(source)) return 11;
    for (size_t i = 0; i < length; ++i) if (buffer[i] != (unsigned char)expected[i]) return 12;
    rewind(source);
    if (feof(source) || ftell(source) != 0 || fgetc(source) != 'i') return 13;
    if (fclose(source)) return 14;
    if (fopen("../pictures/denied.txt", "wb")) return 15;
    if (fopen("hello.c", "invalid")) return 16;
    FILE *output = fopen("native-output.txt", "w+");
    if (!output) return 17;
    if (fputs("Object-backed C streams\n", output) == EOF || fflush(output)) return 18;
    if (fseek(output, -8, SEEK_END) || fputc('S', output) != 'S') return 19;
    rewind(output);
    if (!fgets((char *)buffer, sizeof(buffer), output) || buffer[16] != 'S') return 20;
    if (fwrite(buffer, SIZE_MAX, 2, output) != 0 || !ferror(output)) return 21;
    clearerr(output);
    if (ferror(output)) return 22;
    if (fclose(output)) return 23;
    output = fopen("native-output.txt", "ab");
    if (!output || fseek(output, 0, SEEK_SET) || fputc('!', output) != '!') return 24;
    // Implicit close at normal app exit must publish the final append.
    return 0;
}
