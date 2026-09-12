# Load with cmake -C when configuring the pinned LLVM runtimes source.
# Dependencies already downloaded/built remain in build/native-c; this file
# records semantic options rather than host-specific tool and directory paths.
set(RUNTIMES_USE_LIBC newlib CACHE STRING "InfinityOS C library" FORCE)
set(LIBCXX_ENABLE_LOCALIZATION ON CACHE BOOL "Needed by streams and regex" FORCE)
set(LIBCXX_ENABLE_RANDOM_DEVICE ON CACHE BOOL "Native getentropy adapter" FORCE)
set(LIBCXX_ENABLE_THREADS ON CACHE BOOL "LLVM's serial futures still need synchronization types" FORCE)
set(LIBCXX_ENABLE_MONOTONIC_CLOCK ON CACHE BOOL "Native clock adapter" FORCE)
set(LIBCXX_HAS_PTHREAD_API ON CACHE BOOL "Explicit native synchronization bridge" FORCE)
set(LIBCXX_ENABLE_FILESYSTEM OFF CACHE BOOL "ObjectStore adapters, not std::filesystem" FORCE)
set(LIBCXX_ENABLE_EXCEPTIONS OFF CACHE BOOL "Freestanding compiler runtime" FORCE)
set(LIBCXX_ENABLE_RTTI OFF CACHE BOOL "Freestanding compiler runtime" FORCE)
set(LIBCXX_ENABLE_SHARED OFF CACHE BOOL "Static native payload" FORCE)
set(LIBCXX_ENABLE_STATIC ON CACHE BOOL "Static native payload" FORCE)
set(LIBCXX_ENABLE_TIME_ZONE_DATABASE OFF CACHE BOOL "No implicit filesystem database" FORCE)
set(LIBCXXABI_ENABLE_THREADS ON CACHE BOOL "Match libc++" FORCE)
set(LIBCXXABI_ENABLE_EXCEPTIONS OFF CACHE BOOL "Match libc++" FORCE)
set(LIBCXXABI_ENABLE_SHARED OFF CACHE BOOL "Static native payload" FORCE)
set(LIBCXXABI_ENABLE_STATIC ON CACHE BOOL "Static native payload" FORCE)
# API declarations do not imply separately installed Unix runtime libraries.
set(LIBCXX_HAS_PTHREAD_LIB OFF CACHE INTERNAL "Native service bridge supplies calls" FORCE)
set(LIBCXXABI_HAS_PTHREAD_LIB OFF CACHE INTERNAL "Native service bridge supplies calls" FORCE)
set(LIBCXX_HAS_RT_LIB OFF CACHE INTERNAL "Native service bridge supplies clocks" FORCE)
