#include <infinity/compiler_host.h>
#include <sys/mman.h>
#include <time.h>
#include <errno.h>
#include <assert.h>
#include <sys/random.h>
#include <sys/resource.h>

struct State { uint64_t ns; int error; unsigned calls; int protection; unsigned char bytes[16]; };

// ------------------------=
// FUNC: measured_clock
// DESC: Supplies controlled boundary values for conversion and error propagation tests.
// ------------------=
static int measured_clock(void *context, int clock, uint64_t *out) {
    struct State *state = context;
    assert(clock == CLOCK_MONOTONIC || clock == CLOCK_REALTIME);
    state->calls++;
    *out = state->ns;
    return state->error;
}

// ------------------------=
// FUNC: mapped_memory
// DESC: Records mapping parameters and returns a test-owned buffer, never claiming hardware page protection.
// ------------------=
static int mapped_memory(void *context, void **out, size_t size, int protection, int flags, int fd, int64_t offset) {
    struct State *state = context;
    assert(size == 16 && flags == MAP_PRIVATE && fd == 3 && offset == 0);
    state->calls++;
    state->protection = protection;
    *out = state->bytes;
    return state->error;
}

// ------------------------=
// FUNC: release_memory
// DESC: Records a release request and preserves provider failure codes.
// ------------------=
static int release_memory(void *context, void *address, size_t size) {
    struct State *state = context;
    assert(address == state->bytes && size == 16);
    state->calls++;
    return state->error;
}

// ------------------------=
// FUNC: protect_memory
// DESC: Checks exact protection request forwarding to the provider.
// ------------------=
static int protect_memory(void *context, void *address, size_t size, int protection) {
    struct State *state = context;
    assert(address == state->bytes && size == 16);
    state->calls++;
    state->protection = protection;
    return state->error;
}

// ------------------------=
// FUNC: sync_memory
// DESC: Checks sync flags and request parameters without publishing to any host filesystem.
// ------------------=
static int sync_memory(void *context, void *address, size_t size, int flags) {
    assert(flags == MS_SYNC);
    return release_memory(context, address, size);
}

// ------------------------=
// FUNC: test_entropy
// DESC: Supplies deterministic fixture bytes to verify forwarding, not as production randomness.
// ------------------=
static int test_entropy(void *context, void *out, size_t size) {
    struct State *state = context;
    state->calls++;
    unsigned char *bytes = out;
    for (size_t i = 0; i < size; ++i) bytes[i] = (unsigned char)(i ^ 0xa5);
    return state->error;
}

// ------------------------=
// FUNC: main
// DESC: Exercises service absence, exact time conversion, successful forwarding and failure preservation through the compiler's C ABI.
// ------------------=
int main(void) {
    struct timespec stamp = {17, 19};
    infinity_compiler_set_host(0);
    struct rlimit limit = {17, 19};
    assert(RLIM_INFINITY == UINT64_MAX && RLIM_INFINITY > limit.rlim_max);
    assert(getrlimit(RLIMIT_STACK, &limit) == -1 && errno == ENOSYS);
    assert(limit.rlim_cur == 17 && limit.rlim_max == 19);
    assert(clock_gettime(CLOCK_MONOTONIC, &stamp) == -1 && errno == ENOSYS);
    assert(stamp.tv_sec == 17 && stamp.tv_nsec == 19);
    assert(mmap(0, 16, PROT_READ, MAP_PRIVATE, 3, 0) == MAP_FAILED && errno == ENOSYS);
    assert(mprotect(0, 16, PROT_READ) == -1 && errno == ENOSYS);
    struct State state = {.ns = 2000000003ull};
    const char executable[] = "/system/compiler/clang";
    InfinityCompilerHost host = {.context=&state, .executable_path=executable,
        .clock_ns=measured_clock, .map=mapped_memory, .unmap=release_memory,
        .protect=protect_memory, .sync=sync_memory, .entropy=test_entropy};
    infinity_compiler_set_host(&host);
    assert(infinity_compiler_executable_path() == executable);
    assert(infinity_compiler_home_path() == 0);
    host.home_path = executable;
    assert(infinity_compiler_home_path() == executable);
    assert(clock_gettime(CLOCK_MONOTONIC, &stamp) == 0);
    assert(stamp.tv_sec == 2 && stamp.tv_nsec == 3);
    unsigned calls = state.calls;
    assert(clock_gettime((clockid_t)-1, &stamp) == -1 && errno == EINVAL);
    assert(state.calls == calls);
    assert(mmap(0, 0, PROT_READ, MAP_PRIVATE, 3, 0) == MAP_FAILED && errno == EINVAL);
    assert(state.calls == calls);
    assert(mmap(0, 16, PROT_READ, MAP_PRIVATE, 3, 0) == state.bytes);
    assert(state.protection == PROT_READ);
    assert(mprotect(state.bytes, 16, PROT_READ | PROT_WRITE) == 0);
    assert(state.protection == (PROT_READ | PROT_WRITE));
    assert(msync(state.bytes, 16, MS_SYNC) == 0);
    assert(munmap(state.bytes, 16) == 0);
    state.error = EACCES;
    assert(mmap(0, 16, PROT_EXEC, MAP_PRIVATE, 3, 0) == MAP_FAILED && errno == EACCES);
    assert(mprotect(state.bytes, 16, PROT_EXEC) == -1 && errno == EACCES);
    assert(munmap(state.bytes, 16) == -1 && errno == EACCES);
    assert(msync(state.bytes, 16, MS_SYNC) == -1 && errno == EACCES);
    assert(clock_gettime(CLOCK_REALTIME, &stamp) == -1 && errno == EACCES);
    unsigned char entropy[256] = {17};
    calls = state.calls;
    assert(getentropy(entropy, 257) == -1 && errno == EIO);
    assert(getentropy(0, 1) == -1 && errno == EFAULT);
    assert(getentropy(0, 0) == 0);
    assert(state.calls == calls);
    assert(getentropy(entropy, sizeof entropy) == -1 && errno == EACCES);
    assert(entropy[0] == 17 && entropy[255] == 0);
    state.error = 0;
    assert(getentropy(entropy, sizeof entropy) == 0);
    for (size_t i = 0; i < sizeof entropy; ++i) assert(entropy[i] == (unsigned char)(i ^ 0xa5));
    infinity_compiler_set_host(0);
    assert(getentropy(entropy, 1) == -1 && errno == ENOSYS);
    assert(entropy[0] == 0xa5);
    return 0;
}
