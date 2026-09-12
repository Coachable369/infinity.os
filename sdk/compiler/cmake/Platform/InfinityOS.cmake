# InfinityOS exposes selected POSIX-shaped library interfaces over ObjectStore.
# This is not a Linux target: header/function availability must be detected.
set(UNIX 1)
set(CMAKE_EXECUTABLE_SUFFIX ".elf")
set(CMAKE_DL_LIBS "")
set(CMAKE_SHARED_LIBRARY_SUPPORTED FALSE)
add_link_options("-Wl,--gc-sections")
# Assembly must use the same target as C; otherwise Clang emits host objects.
if(CMAKE_C_COMPILER_TARGET AND NOT CMAKE_ASM_COMPILER_TARGET)
  set(CMAKE_ASM_COMPILER_TARGET "${CMAKE_C_COMPILER_TARGET}")
endif()
# CMAKE_LINKER alone does not select the linker invoked by the Clang driver.
if(CMAKE_LINKER)
  add_link_options("--ld-path=${CMAKE_LINKER}")
endif()
