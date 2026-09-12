#include <infinity/compiler_host.h>
#include <assert.h>
#include <errno.h>
#include <time.h>
#include <sys/time.h>
#include <unistd.h>
static const InfinityCompilerHost *host;
static int failure;
// ------------------------=
// FUNC: infinity_compiler_get_host
// DESC: Returns explicit timing fixture services.
// ------------------=
const InfinityCompilerHost *infinity_compiler_get_host(void) { return host; }
// ------------------------=
// FUNC: clock_value
// DESC: Supplies measured-format fixture nanoseconds.
// ------------------=
static int clock_value(void *context, int clock, uint64_t *out) {
    (void)context; assert(clock == CLOCK_REALTIME); *out = 2000003456; return failure;
}
// ------------------------=
// FUNC: wait_value
// DESC: Records a wait without sleeping and provides a controlled interruption.
// ------------------=
static int wait_value(void *context, uint64_t ns, uint64_t *left) {
    (void)context; assert(ns == 1000000); *left = 500000; return failure;
}
// ------------------------=
// FUNC: main
// DESC: Tests conversion, absent services, measured remainders and failure atomicity.
// ------------------=
int main(void) {
    struct timeval value = {17, 19};
    assert(gettimeofday(&value, 0) == -1 && errno == ENOSYS && value.tv_sec == 17);
    InfinityCompilerHost service = {.clock_ns = clock_value, .page_size = 4096, .sleep_ns = wait_value};
    host = &service;
    assert(gettimeofday(&value, 0) == 0 && value.tv_sec == 2 && value.tv_usec == 3);
    assert(getpagesize() == 4096);
    service.page_size = 4097;
    assert(getpagesize() == -1 && errno == EINVAL);
    assert(usleep(1000) == 0);
    failure = EINTR;
    struct timespec request = {0, 1000000}, remaining = {17, 19};
    assert(nanosleep(&request, &remaining) == -1 && errno == EINTR);
    assert(remaining.tv_sec == 0 && remaining.tv_nsec == 500000);
    request.tv_nsec = 1000000000;
    assert(nanosleep(&request, &remaining) == -1 && errno == EINVAL);
    assert(remaining.tv_nsec == 500000);
    failure = EACCES;
    assert(gettimeofday(&value, 0) == -1 && value.tv_sec == 2 && value.tv_usec == 3);
    return 0;
}
