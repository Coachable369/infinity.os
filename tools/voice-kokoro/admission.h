#ifndef INFINITY_NATIVE_SPEECH_ADMISSION_H
#define INFINITY_NATIVE_SPEECH_ADMISSION_H

enum { NATIVE_ENGINE_BUSY = 8 };

// ------------------------=
// FUNC: native_engine_try_acquire
// DESC: Admits exactly one caller to the shared native heap, TLS and fault boundary without blocking an AP.
// ------------------=
static inline int native_engine_try_acquire(unsigned *active) {
    unsigned expected = 0;
    return __atomic_compare_exchange_n(active, &expected, 1, 0, __ATOMIC_ACQUIRE, __ATOMIC_RELAXED);
}

// ------------------------=
// FUNC: native_engine_release
// DESC: Publishes all native cleanup before another speech operation can acquire its shared storage.
// ------------------=
static inline void native_engine_release(unsigned *active) {
    __atomic_store_n(active, 0, __ATOMIC_RELEASE);
}

// ------------------------=
// FUNC: native_engine_is_active
// DESC: Reads native fault-boundary ownership without a non-atomic cross-core access.
// ------------------=
static inline int native_engine_is_active(const unsigned *active) {
    return __atomic_load_n(active, __ATOMIC_ACQUIRE) != 0;
}

#endif
