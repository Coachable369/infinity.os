# InfinityOS exposes selected POSIX-shaped library interfaces over ObjectStore.
# This is not a Linux target: header/function availability must be detected.
set(UNIX 1)
set(CMAKE_EXECUTABLE_SUFFIX ".elf")
set(CMAKE_DL_LIBS "")
set(CMAKE_SHARED_LIBRARY_SUPPORTED FALSE)
