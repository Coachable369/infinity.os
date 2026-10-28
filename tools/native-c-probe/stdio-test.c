#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include "../../sdk/c/internal.h"

const InfinityAppApi *infinity_host;
static unsigned char output[8192], errors[8192], object[64];
static size_t output_length, error_length, object_length, object_cursor, input_cursor;
static int reject_write, flushes, closes;

// ------------------------=
// FUNC: open_object
// DESC: Supplies one bounded granted object to exercise normal FILE behavior alongside standard streams.
// ------------------=
static int open_object(void *context, const unsigned char *name, size_t length, uint32_t flags) {
    (void)context;
    return length == 1 && name[0] == 'x' && (flags & 3) == 3 ? 3 : -13;
}

// ------------------------=
// FUNC: read_stream
// DESC: Returns deliberately short reads from native-style stdin or the single granted object.
// ------------------=
static ptrdiff_t read_stream(void *context, int handle, unsigned char *bytes, size_t length) {
    (void)context;
    const unsigned char input[] = {'3', '7', '\n'};
    size_t *cursor = handle == 0 ? &input_cursor : &object_cursor;
    const unsigned char *source = handle == 0 ? input : object;
    size_t available = handle == 0 ? sizeof input : object_length;
    if (handle != 0 && handle != 3) return -9;
    if (length > available - *cursor) length = available - *cursor;
    if (length > 2) length = 2;
    memcpy(bytes, source + *cursor, length); *cursor += length;
    return (ptrdiff_t)length;
}

// ------------------------=
// FUNC: write_stream
// DESC: Records exact bytes and descriptor identity while forcing callers to handle short standard-stream writes.
// ------------------=
static ptrdiff_t write_stream(void *context, int handle, const unsigned char *bytes, size_t length) {
    (void)context;
    if (reject_write) return -5;
    unsigned char *destination;
    size_t *used, capacity;
    if (handle == 1) { destination = output; used = &output_length; capacity = sizeof output; }
    else if (handle == 2) { destination = errors; used = &error_length; capacity = sizeof errors; }
    else if (handle == 3) { destination = object; used = &object_length; capacity = sizeof object; }
    else return -9;
    if (handle != 3 && length > 3) length = 3;
    if (length > capacity - *used) return -28;
    memcpy(destination + *used, bytes, length); *used += length;
    return (ptrdiff_t)length;
}

// ------------------------=
// FUNC: write_stdout
// DESC: Preserves the existing ABI2 stdout callback while recording the same explicit standard-stream descriptor.
// ------------------=
static int write_stdout(void *context, const unsigned char *bytes, size_t length) {
    return (int)write_stream(context, 1, bytes, length);
}

// ------------------------=
// FUNC: seek_object
// DESC: Moves only the fixture's granted object cursor and rejects standard-stream seeking.
// ------------------=
static int64_t seek_object(void *context, int handle, int64_t offset, uint32_t origin) {
    (void)context;
    if (handle != 3) return -29;
    int64_t next = offset + (origin == SEEK_CUR ? (int64_t)object_cursor : origin == SEEK_END ? (int64_t)object_length : 0);
    if (next < 0 || (uint64_t)next > object_length) return -22;
    object_cursor = (size_t)next; return next;
}

// ------------------------=
// FUNC: flush_object
// DESC: Records real object flush requests without treating standard streams as stored objects.
// ------------------=
static int flush_object(void *context, int handle) {
    (void)context; if (handle != 3) return -9; ++flushes; return 0;
}

// ------------------------=
// FUNC: close_object
// DESC: Records granted-object closure separately from standard FILE lifecycle.
// ------------------=
static int close_object(void *context, int handle) {
    (void)context; if (handle != 3) return -9; ++closes; return 0;
}

// ------------------------=
// FUNC: main
// DESC: Verifies standard channels, partial I/O, EOF, invalid access, reset, formatting, and unchanged object-stream semantics.
// ------------------=
int main(void) {
    const InfinityObjectApi objects = {open_object, read_stream, write_stream, seek_object, flush_object, close_object};
    const InfinityAppApi host = {INFINITY_APP_ABI, sizeof host, 0, write_stdout, &objects};
    infinity_host = &host; infinity_stdio_reset();
    if (printf("%d:%s", 42, "out") != 6 || output_length != 6 || memcmp(output, "42:out", 6)) return 1;
    if (fprintf(stderr, "%d:%s", 17, "error") != 8 || error_length != 8 || memcmp(errors, "17:error", 8)) return 2;
    unsigned char bulk[5000]; memset(bulk, 0xa5, sizeof bulk);
    if (fwrite(bulk, 2, 2500, stdout) != 2500 || output_length != 5006) return 3;
    if (memcmp(output + 6, bulk, sizeof bulk) || error_length != 8) return 4;
    char line[16];
    if (fgets(line, sizeof line, stdin) != line || memcmp(line, "37\n\0", 4) || feof(stdin)) return 5;
    if (fgetc(stdin) != EOF || !feof(stdin) || ferror(stdin)) return 6;
    clearerr(stdin);
    if (feof(stdin) || ferror(stdin) || fseek(stdin, 0, SEEK_SET) != -1 || ftell(stdout) != -1) return 7;
    if (fwrite("x", 1, 1, stdin) || !ferror(stdin)) return 8;
    if (fread(line, 1, 1, stdout) || !ferror(stdout)) return 9;
    clearerr(stdout);
    reject_write = 1;
    if (fprintf(stderr, "%d", 7) != EOF || !ferror(stderr) || error_length != 8) return 10;
    reject_write = 0; clearerr(stderr);
    if (fwrite("x", SIZE_MAX, 2, stderr) || !ferror(stderr)) return 11;
    if (fclose(stderr) || fputs("x", stderr) != EOF || fclose(stderr) != EOF) return 12;
    infinity_stdio_reset();
    if (ferror(stderr) || fprintf(stderr, "%u", 9u) != 1 || errors[8] != '9') return 13;
    if (fflush(stdout) || fflush(stderr) || flushes) return 14;
    FILE *file = fopen("x", "w+");
    if (!file || fprintf(file, "%d", 123) != 3 || fflush(file) || flushes != 1) return 15;
    rewind(file);
    if (fgetc(file) != '1' || ftell(file) != 1 || fclose(file) || closes != 1) return 16;
    if (ferror((FILE *)(uintptr_t)1) != 1 || fread(line, 1, 1, (FILE *)(uintptr_t)1)) return 17;
    memset(bulk, 'x', 1100); bulk[1100] = 0;
    if (fprintf(stderr, "%s", bulk) != EOF || error_length != 9) return 18;
    if (infinity_stdio_finish() || closes != 1) return 19;
    return 0;
}
