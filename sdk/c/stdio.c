#include <stdio.h>
#include <stdint.h>
#include <limits.h>
#include "internal.h"

struct InfinityFile { int handle; int error; int eof; };
static FILE streams[FOPEN_MAX];
FILE infinity_stdin = {0, 0, 0};
FILE infinity_stdout = {1, 0, 0};
FILE infinity_stderr = {2, 0, 0};

// ------------------------=
// FUNC: standard
// DESC: Recognizes only the three SDK-owned standard stream objects by identity.
// ------------------=
static int standard(FILE *stream) {
    return stream == stdin || stream == stdout || stream == stderr;
}

// ------------------------=
// FUNC: objects
// DESC: Requires a complete optional object-stream callback table.
// ------------------=
static const InfinityObjectApi *objects(void) {
    if (!infinity_host) return 0;
    const InfinityObjectApi *o = infinity_host->objects;
    return o && o->open && o->read && o->write && o->seek && o->flush && o->close ? o : 0;
}

// ------------------------=
// FUNC: valid
// DESC: Validates a FILE pointer by identity before dereferencing it.
// ------------------=
static int valid(FILE *stream) {
    if (standard(stream)) return infinity_host && stream->handle >= 0;
    for (size_t i = 0; i < FOPEN_MAX; ++i)
        if (stream == &streams[i]) return stream->handle > 0 && objects();
    return 0;
}

// ------------------------=
// FUNC: infinity_stdio_reset
// DESC: Resets per-invocation stream bookkeeping before main runs.
// ------------------=
void infinity_stdio_reset(void) {
    for (size_t i = 0; i < FOPEN_MAX; ++i) streams[i] = (FILE){0, 0, 0};
    infinity_stdin = (FILE){0, 0, 0};
    infinity_stdout = (FILE){1, 0, 0};
    infinity_stderr = (FILE){2, 0, 0};
}

// ------------------------=
// FUNC: fopen
// DESC: Parses standard r/w/a binary, update and exclusive modes and opens a granted ObjectStore reference.
// ------------------=
FILE *fopen(const char *path, const char *mode) {
    const InfinityObjectApi *o = objects();
    if (!o || !path || !mode || !mode[0]) return 0;
    unsigned flags;
    if (mode[0] == 'r') flags = 1;
    else if (mode[0] == 'w') flags = 2 | 4 | 8;
    else if (mode[0] == 'a') flags = 2 | 4 | 16;
    else return 0;
    int binary = 0, update = 0, exclusive = 0;
    for (size_t i = 1; mode[i]; ++i) {
        if (mode[i] == 'b' && !binary) binary = 1;
        else if (mode[i] == '+' && !update) update = 1;
        else if (mode[i] == 'x' && !exclusive && mode[0] == 'w') exclusive = 1;
        else return 0;
    }
    if (update) flags |= 1 | 2;
    if (exclusive) flags |= 32;
    if (!binary) flags |= 64;
    FILE *stream = 0;
    for (size_t i = 0; i < FOPEN_MAX; ++i) if (!streams[i].handle) { stream = &streams[i]; break; }
    if (!stream) return 0;
    size_t length = 0;
    while (path[length]) { if (++length > 95) return 0; }
    int handle = o->open(infinity_host->context, (const unsigned char *)path, length, flags);
    if (handle < 3) return 0;
    *stream = (FILE){handle, 0, 0};
    return stream;
}

// ------------------------=
// FUNC: fflush
// DESC: Publishes buffered versions and preserves callback failure as a stream error.
// ------------------=
int fflush(FILE *stream) {
    if (!stream) {
        int result = 0;
        for (size_t i = 0; i < FOPEN_MAX; ++i) if (streams[i].handle && fflush(&streams[i])) result = EOF;
        return result;
    }
    if (!valid(stream)) return EOF;
    if (standard(stream)) return 0; /* Native terminal streams are unbuffered. */
    if (objects()->flush(infinity_host->context, stream->handle)) { stream->error = 1; return EOF; }
    return 0;
}

// ------------------------=
// FUNC: fclose
// DESC: Releases the object handle even when publication fails.
// ------------------=
int fclose(FILE *stream) {
    if (!valid(stream)) return EOF;
    if (standard(stream)) { stream->handle = -1; return 0; }
    int result = objects()->close(infinity_host->context, stream->handle);
    *stream = (FILE){0, 0, 0};
    return result ? EOF : 0;
}

// ------------------------=
// FUNC: infinity_stdio_finish
// DESC: Closes all remaining streams and makes failed implicit flushes observable to the launcher.
// ------------------=
int infinity_stdio_finish(void) {
    int result = 0;
    for (size_t i = 0; i < FOPEN_MAX; ++i) if (streams[i].handle && fclose(&streams[i])) result = EOF;
    return result;
}

// ------------------------=
// FUNC: fread
// DESC: Reads complete elements with overflow checks and distinct EOF versus error state.
// ------------------=
size_t fread(void *data, size_t size, size_t count, FILE *stream) {
    if (!size || !count) return 0;
    if (!valid(stream)) return 0;
    if (!data || count > PTRDIFF_MAX / size) { stream->error = 1; return 0; }
    size_t bytes = size * count;
    if (standard(stream)) {
        const InfinityObjectApi *o = objects();
        if (stream != stdin || !o) { stream->error = 1; return 0; }
        size_t consumed = 0;
        while (consumed < bytes) {
            ptrdiff_t got = o->read(infinity_host->context, 0, (unsigned char *)data + consumed, bytes - consumed);
            if (got < 0 || (size_t)got > bytes - consumed) { stream->error = 1; break; }
            if (!got) { stream->eof = 1; break; }
            consumed += (size_t)got;
        }
        return consumed / size;
    }
    ptrdiff_t result = objects()->read(infinity_host->context, stream->handle, data, bytes);
    if (result < 0 || (size_t)result > bytes) { stream->error = 1; return 0; }
    if ((size_t)result < bytes) stream->eof = 1;
    return (size_t)result / size;
}

// ------------------------=
// FUNC: fwrite
// DESC: Writes complete elements through the object endpoint, preserving short-write and overflow failures.
// ------------------=
size_t fwrite(const void *data, size_t size, size_t count, FILE *stream) {
    if (!size || !count) return 0;
    if (!valid(stream)) return 0;
    if (!data || count > PTRDIFF_MAX / size) { stream->error = 1; return 0; }
    size_t bytes = size * count;
    if (standard(stream)) {
        const InfinityObjectApi *o = objects();
        if (stream == stdin || (stream == stdout && !infinity_host->write_stdout)
            || (stream == stderr && !o)) { stream->error = 1; return 0; }
        size_t written = 0;
        while (written < bytes) {
            size_t chunk = bytes - written;
            if (chunk > 4096) chunk = 4096;
            ptrdiff_t sent = stream == stdout ? infinity_host->write_stdout(infinity_host->context,
                (const unsigned char *)data + written, chunk) :
                o->write(infinity_host->context, 2, (const unsigned char *)data + written, chunk);
            if (sent <= 0 || (size_t)sent > chunk) { stream->error = 1; break; }
            written += (size_t)sent;
        }
        return written / size;
    }
    ptrdiff_t result = objects()->write(infinity_host->context, stream->handle, data, bytes);
    if (result < 0 || (size_t)result > bytes) { stream->error = 1; return 0; }
    if ((size_t)result < bytes) stream->error = 1;
    return (size_t)result / size;
}

// ------------------------=
// FUNC: fseek
// DESC: Moves the object cursor and clears EOF only after a successful seek.
// ------------------=
int fseek(FILE *stream, long offset, int origin) {
    if (!valid(stream) || origin < SEEK_SET || origin > SEEK_END) return -1;
    if (standard(stream)) return -1; /* Console streams are not seekable. */
    if (objects()->seek(infinity_host->context, stream->handle, offset, (uint32_t)origin) < 0) return -1;
    stream->eof = 0;
    return 0;
}

// ------------------------=
// FUNC: ftell
// DESC: Returns the current object cursor or a representability failure.
// ------------------=
long ftell(FILE *stream) {
    if (!valid(stream)) return -1;
    if (standard(stream)) return -1;
    int64_t position = objects()->seek(infinity_host->context, stream->handle, 0, SEEK_CUR);
    return position < 0 || position > LONG_MAX ? -1 : (long)position;
}

// ------------------------=
// FUNC: clearerr
// DESC: Clears recorded EOF and error without changing the object cursor.
// ------------------=
void clearerr(FILE *stream) { if (valid(stream)) stream->error = stream->eof = 0; }

// ------------------------=
// FUNC: rewind
// DESC: Seeks to the start and clears the stream error indicator.
// ------------------=
void rewind(FILE *stream) { (void)fseek(stream, 0, SEEK_SET); clearerr(stream); }

// ------------------------=
// FUNC: feof
// DESC: Reports whether a previous read reached the end of the opened version.
// ------------------=
int feof(FILE *stream) { return valid(stream) ? stream->eof : 0; }

// ------------------------=
// FUNC: ferror
// DESC: Reports a recorded stream error, including an invalid stream.
// ------------------=
int ferror(FILE *stream) { return valid(stream) ? stream->error : 1; }

// ------------------------=
// FUNC: fgetc
// DESC: Reads one unsigned character or reports EOF.
// ------------------=
int fgetc(FILE *stream) { unsigned char byte; return fread(&byte, 1, 1, stream) == 1 ? byte : EOF; }

// ------------------------=
// FUNC: fputc
// DESC: Writes one unsigned character or reports EOF.
// ------------------=
int fputc(int character, FILE *stream) { unsigned char byte = (unsigned char)character; return fwrite(&byte, 1, 1, stream) == 1 ? byte : EOF; }

// ------------------------=
// FUNC: fgets
// DESC: Reads a bounded terminated line, retaining its newline when present.
// ------------------=
char *fgets(char *data, int size, FILE *stream) {
    if (!data || size <= 0 || !valid(stream)) return 0;
    int length = 0;
    while (length < size - 1) {
        int byte = fgetc(stream);
        if (byte == EOF) { if (ferror(stream) || !length) return 0; break; }
        data[length++] = (char)byte;
        if (byte == '\n') break;
    }
    data[length] = 0;
    return data;
}

// ------------------------=
// FUNC: fputs
// DESC: Writes a terminated string without adding a newline.
// ------------------=
int fputs(const char *text, FILE *stream) {
    if (!text) return EOF;
    size_t length = 0;
    while (text[length]) ++length;
    return fwrite(text, 1, length, stream) == length ? 0 : EOF;
}
