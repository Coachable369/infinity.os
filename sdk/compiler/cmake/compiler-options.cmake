# Do not inherit a successful host librt probe into a cross-build.
# Native clock functions are supplied by the compiler service bridge.
set(HAVE_LIBRT 0 CACHE INTERNAL "" FORCE)
set(CMAKE_EXE_LINKER_FLAGS "" CACHE STRING "" FORCE)
