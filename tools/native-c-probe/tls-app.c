static _Thread_local volatile unsigned initialized = 37;
static _Thread_local volatile unsigned zero;
static _Thread_local __attribute__((aligned(64))) volatile unsigned aligned_value = 9;

// ------------------------=
// FUNC: infinity_app_entry
// DESC: Reads and mutates compiler-emitted native TLS while checking initialization and independent state on repeated entry.
// ------------------=
int infinity_app_entry(unsigned turn) {
    if (((__UINTPTR_TYPE__)&aligned_value & 63) != 0) return 1;
    if (initialized != 37 + turn || zero != turn || aligned_value != 9 + turn) return 2;
    initialized++; zero++; aligned_value++;
    return 0;
}
