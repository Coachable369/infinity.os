#include <infinity/compiler_host.h>
#include <assert.h>
#include <errno.h>
static void (*registered)(void *);
static void *cookie;
static int provider_error, notifications, releases;
// ------------------------=
// FUNC: notification
// DESC: Counts explicit fixture notifications; does not cause or recover a real hardware fault.
// ------------------=
static void notification(void *context) { assert(context == &notifications); ++notifications; }
// ------------------------=
// FUNC: bind_hook
// DESC: Models provider registration failure and saves successful callback ownership.
// ------------------=
static int bind_hook(void *context, void (*callback)(void *), void *value) {
    assert(context == &provider_error);
    if (provider_error) return provider_error;
    registered = callback; cookie = value; return 0;
}
// ------------------------=
// FUNC: release_hook
// DESC: Models retryable unregistration and successful callback removal.
// ------------------=
static int release_hook(void *context) {
    assert(context == &provider_error);
    if (provider_error) return provider_error;
    registered = 0; cookie = 0; ++releases; return 0;
}
// ------------------------=
// FUNC: main
// DESC: Verifies registration failures, delivery, ownership, duplicate rejection and release retry.
// ------------------=
int main(void) {
    assert(infinity_compiler_register_crash_handler(notification, 0) == -1 && errno == ENOSYS);
    assert(infinity_compiler_register_crash_handler(0, 0) == -1 && errno == EINVAL);
    InfinityCompilerHost services = {.context=&provider_error,
        .register_crash_handler=bind_hook, .unregister_crash_handler=release_hook};
    infinity_compiler_set_host(&services);
    provider_error = EACCES;
    assert(infinity_compiler_register_crash_handler(notification, &notifications) == -1 && errno == EACCES);
    assert(!registered);
    provider_error = 0;
    assert(!infinity_compiler_register_crash_handler(notification, &notifications));
    assert(infinity_compiler_register_crash_handler(notification, 0) == -1 && errno == EBUSY);
    registered(cookie); assert(notifications == 1);
    infinity_compiler_set_host(0);
    provider_error = EIO;
    assert(infinity_compiler_unregister_crash_handler() == -1 && errno == EIO);
    assert(registered);
    provider_error = 0;
    assert(!infinity_compiler_unregister_crash_handler());
    assert(!registered && releases == 1);
    assert(infinity_compiler_unregister_crash_handler() == -1 && errno == EINVAL);
    return 0;
}
