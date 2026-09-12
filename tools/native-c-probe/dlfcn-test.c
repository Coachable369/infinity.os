#include <dlfcn.h>
#include <errno.h>
#include <assert.h>
#include <pthread.h>

// ------------------------=
// FUNC: other_thread
// DESC: Verifies error state neither leaks from nor consumes another thread's error.
// ------------------=
static void *other_thread(void *unused) {
    (void)unused;
    assert(!dlerror());
    assert(dlclose(RTLD_NEXT) == -1 && errno == ENOSYS);
    assert(dlerror());
    assert(!dlerror());
    return 0;
}
// ------------------------=
// FUNC: main
// DESC: Tests failure results, diagnostic lifecycle, unchanged output and TLS isolation.
// ------------------=
int main(void) {
    assert(!dlerror());
    int modes[] = {RTLD_NOW, RTLD_LAZY, RTLD_NOW | RTLD_GLOBAL};
    for (unsigned i = 0; i < sizeof modes / sizeof *modes; ++i) {
        assert(!dlopen("/system/compiler/plugin", modes[i]) && errno == ENOSYS);
        assert(dlerror()); assert(!dlerror());
    }
    assert(!dlopen(0, RTLD_NOW) && errno == ENOSYS);
    assert(!dlopen("x", 0) && errno == EINVAL);
    assert(!dlopen("x", RTLD_NOW | RTLD_LAZY) && errno == EINVAL);
    assert(!dlopen("x", 0x4000) && errno == EINVAL);
    assert(!dlsym(RTLD_DEFAULT, "main") && errno == ENOSYS);
    assert(!dlsym(RTLD_NEXT, 0) && errno == EINVAL);
    Dl_info info = {"image", &info, "symbol", &info};
    Dl_info previous = info;
    assert(!dladdr(&info, &info) && errno == ENOSYS);
    assert(info.dli_fname == previous.dli_fname && info.dli_fbase == previous.dli_fbase);
    assert(info.dli_sname == previous.dli_sname && info.dli_saddr == previous.dli_saddr);
    assert(!dladdr(0, 0) && errno == EINVAL);
    pthread_t thread;
    assert(!pthread_create(&thread, 0, other_thread, 0));
    assert(!pthread_join(thread, 0));
    assert(dlerror()); assert(!dlerror());
    return 0;
}
