#include "Python.h"
#include <pthread.h>
#include <stdint.h>
#include <infinity/syscall.h>

_Static_assert(sizeof(pthread_t) == SIZEOF_PTHREAD_T, "CPython pthread ABI mismatch");
_Static_assert(sizeof(void *) == SIZEOF_VOID_P, "CPython pointer ABI mismatch");
_Static_assert(sizeof(wchar_t) == SIZEOF_WCHAR_T, "CPython wchar ABI mismatch");

// ------------------------=
// FUNC: main
// DESC: Runs native script arguments with isolated initialization and no host environment or site injection.
// ------------------=
int main(int argc, char **argv) {
    unsigned char seed[32];
    if (infinity_system_call(0x1232, (uintptr_t)seed, sizeof seed, 0, 0, 0, 0) < 0) {
        fprintf(stderr, "Python launch entropy unavailable\n"); return 125;
    }
    PyConfig config;
    PyConfig_InitIsolatedConfig(&config);
    config.parse_argv = 1;
    config.safe_path = 1; /* Only the explicit script directory is searched. */
    config.site_import = 0;
    config.write_bytecode = 0;
    config.install_signal_handlers = 0;
    config.use_hash_seed = 1;
    config.hash_seed = (uint32_t)seed[0] | ((uint32_t)seed[1] << 8) |
        ((uint32_t)seed[2] << 16) | ((uint32_t)seed[3] << 24);
    /* CPython treats zero as disabling randomization. */
    if (!config.hash_seed) config.hash_seed = 1;
    memset(seed, 0, sizeof seed);
    config.module_search_paths_set = 1;
    PyStatus status = PyConfig_SetBytesArgv(&config, argc, argv);
    if (!PyStatus_Exception(status) && argc > 1 && argv[1][0] == '/') {
        wchar_t *directory = Py_DecodeLocale(argv[1], NULL);
        if (!directory) { PyConfig_Clear(&config); return 125; }
        wchar_t *slash = wcsrchr(directory, L'/');
        if (slash == directory) slash[1] = L'\0';
        else *slash = L'\0';
        status = PyWideStringList_Append(&config.module_search_paths, directory);
        PyMem_RawFree(directory);
    }
    if (!PyStatus_Exception(status))
        status = Py_InitializeFromConfig(&config);
    PyConfig_Clear(&config);
    if (PyStatus_Exception(status)) {
        if (PyStatus_IsExit(status)) return status.exitcode;
        fprintf(stderr, "Python initialization failed: %s\n", status.err_msg);
        return 125;
    }
    return Py_RunMain();
}
