#!/bin/sh
# Rebuilds the pinned x86-64 InfinityOS Clang/LLD development toolchain.
set -eu

cd "$(dirname "$0")/.."
repo=$PWD
target=x86_64-unknown-elf
work=$repo/build/native-c
downloads=$work/downloads
sources=$work/src
sysroot=$work/sysroot/$target
llvm_source=$sources/llvm-project
newlib_source=$sources/newlib
llvm_archive=$downloads/llvm-project-ea7d852.tar.gz
newlib_archive=$downloads/newlib-4.6.0.20260123.tar.gz
llvm_url=https://github.com/llvm/llvm-project/archive/ea7d852a70e8bdfaf601d6626a760f9771b2c4b4.tar.gz
newlib_url=https://sourceware.org/pub/newlib/newlib-4.6.0.20260123.tar.gz
llvm_sha=5878830436d5fc0f460fa756073880e0fc124df74fd5556c798e2ed85dc7bd55
newlib_sha=6ff27e3bf022666f43f7802255be680eeff722ac181b1725d21e2e8318604ee3
llvm_bin=${LLVM_BIN:-/opt/homebrew/opt/llvm/bin}
ld_lld=${LD_LLD:-/opt/homebrew/bin/ld.lld}
jobs=${NATIVE_C_JOBS:-6}

for required in curl shasum tar patch cmake ninja make rustc; do
  command -v "$required" >/dev/null 2>&1 || {
    echo "missing required host tool: $required" >&2
    exit 2
  }
done
for required in clang clang++ llvm-ar llvm-as llvm-nm llvm-objcopy llvm-ranlib llvm-readelf llvm-strip llvm-tblgen clang-tblgen; do
  test -x "$llvm_bin/$required" || {
    echo "missing required LLVM 23 host tool: $llvm_bin/$required" >&2
    exit 2
  }
done
test -x "$ld_lld" || {
  echo "missing host ELF linker: $ld_lld" >&2
  exit 2
}

mkdir -p "$downloads" "$sources" "$sysroot"
test -f "$llvm_archive" || curl -fL --retry 3 -o "$llvm_archive" "$llvm_url"
test -f "$newlib_archive" || curl -fL --retry 3 -o "$newlib_archive" "$newlib_url"
printf '%s  %s\n' "$llvm_sha" "$llvm_archive" | shasum -a 256 -c
printf '%s  %s\n' "$newlib_sha" "$newlib_archive" | shasum -a 256 -c

if test ! -d "$llvm_source"; then
  tar -xzf "$llvm_archive" -C "$sources"
  mv "$sources/llvm-project-ea7d852a70e8bdfaf601d6626a760f9771b2c4b4" "$llvm_source"
fi
if test ! -d "$newlib_source"; then
  tar -xzf "$newlib_archive" -C "$sources"
  mv "$sources/newlib-4.6.0.20260123" "$newlib_source"
fi

if test ! -f "$llvm_source/.infinity-patched"; then
  patches="llvm-infinity llvm-resource-errors llvm-native-host llvm-seed-reader llvm-layout-reader llvm-elf-linker llvm-native-invocation libcxx-infinity"
  already_patched=1
  for patch_name in $patches; do
    patch -f --dry-run -R -p1 -d "$llvm_source" < "$repo/sdk/compiler/$patch_name.patch" >/dev/null 2>&1 || already_patched=0
  done
  if test "$already_patched" -eq 0; then
    for patch_name in $patches; do
      patch -f -s -p1 -d "$llvm_source" < "$repo/sdk/compiler/$patch_name.patch"
    done
  fi
  touch "$llvm_source/.infinity-patched"
fi

newlib_build=$work/newlib-build-x86_64
mkdir -p "$newlib_build"
if test ! -f "$newlib_build/Makefile"; then
  cd "$newlib_build"
  "$newlib_source/configure" \
    --target="$target" \
    --prefix="$sysroot" \
    --disable-newlib-supplied-syscalls \
    --disable-libgloss \
    --disable-multilib \
    --disable-newlib-multithread \
    CC_FOR_TARGET="$llvm_bin/clang --target=$target" \
    CXX_FOR_TARGET="$llvm_bin/clang++ --target=$target" \
    AR_FOR_TARGET="$llvm_bin/llvm-ar" \
    AS_FOR_TARGET="$llvm_bin/llvm-as" \
    LD_FOR_TARGET="$ld_lld" \
    NM_FOR_TARGET="$llvm_bin/llvm-nm" \
    OBJCOPY_FOR_TARGET="$llvm_bin/llvm-objcopy" \
    RANLIB_FOR_TARGET="$llvm_bin/llvm-ranlib" \
    READELF_FOR_TARGET="$llvm_bin/llvm-readelf" \
    STRIP_FOR_TARGET="$llvm_bin/llvm-strip" \
    CFLAGS_FOR_TARGET="-O2 -mno-red-zone -ffunction-sections -fdata-sections"
  cd "$repo"
fi
make -C "$newlib_build" -j"$jobs" all-target-newlib
make -C "$newlib_build" install-target-newlib
ln -sfn "$target/include" "$sysroot/include"
ln -sfn "$target/lib" "$sysroot/lib"

builtins_build=$work/builtins-x86_64
cmake -G Ninja \
  -S "$llvm_source/compiler-rt" \
  -B "$builtins_build" \
  -C "$repo/sdk/compiler/cmake/builtins-options.cmake" \
  -DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_SYSTEM_NAME=InfinityOS \
  -DCMAKE_MODULE_PATH="$repo/sdk/compiler/cmake" \
  -DCMAKE_C_COMPILER="$llvm_bin/clang" \
  -DCMAKE_C_COMPILER_TARGET="$target" \
  -DCMAKE_ASM_COMPILER="$llvm_bin/clang" \
  -DCMAKE_ASM_COMPILER_TARGET="$target" \
  -DCMAKE_AR="$llvm_bin/llvm-ar" \
  -DCMAKE_RANLIB="$llvm_bin/llvm-ranlib" \
  -DCMAKE_LINKER="$ld_lld" \
  -DCMAKE_SYSROOT="$sysroot" \
  -DCMAKE_INSTALL_PREFIX="$sysroot" \
  -DCMAKE_C_FLAGS="-O2 -mno-red-zone -ffunction-sections -fdata-sections"
ninja -C "$builtins_build" -j"$jobs"

common_flags="-O2 -fPIE -mno-red-zone -ffunction-sections -fdata-sections -include $repo/sdk/compiler/target.h -I$repo/sdk/compiler/include"
cxx_build=$work/cxx-build-x86_64
cmake -G Ninja \
  -S "$llvm_source/runtimes" \
  -B "$cxx_build" \
  -C "$repo/sdk/compiler/cmake/runtime-options.cmake" \
  -DLLVM_ENABLE_RUNTIMES="libcxx;libcxxabi" \
  -DLIBCXXABI_USE_LLVM_UNWINDER=OFF \
  -DLLVM_DEFAULT_TARGET_TRIPLE="$target" \
  -DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_SYSTEM_NAME=InfinityOS \
  -DCMAKE_MODULE_PATH="$repo/sdk/compiler/cmake" \
  -DCMAKE_C_COMPILER="$llvm_bin/clang" \
  -DCMAKE_CXX_COMPILER="$llvm_bin/clang++" \
  -DCMAKE_ASM_COMPILER="$llvm_bin/clang" \
  -DCMAKE_C_COMPILER_TARGET="$target" \
  -DCMAKE_CXX_COMPILER_TARGET="$target" \
  -DCMAKE_ASM_COMPILER_TARGET="$target" \
  -DCMAKE_AR="$llvm_bin/llvm-ar" \
  -DCMAKE_RANLIB="$llvm_bin/llvm-ranlib" \
  -DCMAKE_LINKER="$ld_lld" \
  -DCMAKE_SYSROOT="$sysroot" \
  -DCMAKE_INSTALL_PREFIX="$sysroot" \
  -DCMAKE_C_FLAGS="$common_flags" \
  -DCMAKE_CXX_FLAGS="$common_flags"
ninja -C "$cxx_build" -j"$jobs" cxx cxxabi
ninja -C "$cxx_build" install-cxx install-cxxabi

INFINITY_COMPILER_SYSROOT="$sysroot" LLVM_BIN="$llvm_bin" sh "$repo/tools/native-c-probe/build-compiler-runtime.sh"
runtime_objects="$work/compiler-start.o $work/compiler-entry.o $work/compiler-platform.o $work/compiler-dlfcn.o $work/compiler-files.o $work/compiler-heap.o $work/compiler-time.o $work/compiler-serial_sync.o $work/compiler-serial_tls.o $work/compiler-pthread.o $work/compiler-namespace.o $work/compiler-process.o"
runtime_builtins="$builtins_build/lib/infinityos/libclang_rt.builtins-x86_64.a"
compiler_build=$work/clang-build-infinity-x86_64
compiler_cxx_flags="$common_flags -nostdinc++ -isystem $sysroot/include/c++/v1"
compiler_link_flags="-nostdlib -Wl,--gc-sections,--error-limit=0,-e,infinity_compiler_entry $runtime_objects -L$sysroot/lib -Wl,--start-group -lc++ -lc++abi -lc -lm $runtime_builtins -Wl,--end-group"
env LDFLAGS= CPPFLAGS= cmake -G Ninja \
  -S "$llvm_source/llvm" \
  -B "$compiler_build" \
  -C "$repo/sdk/compiler/cmake/compiler-options.cmake" \
  -DLLVM_ENABLE_PROJECTS="clang;lld" \
  -DLLVM_TARGETS_TO_BUILD=X86 \
  -DLLVM_ENABLE_THREADS=OFF \
  -DLLVM_ENABLE_ZLIB=OFF \
  -DLLVM_ENABLE_ZSTD=OFF \
  -DLLVM_ENABLE_LIBXML2=OFF \
  -DLLVM_ENABLE_TERMINFO=OFF \
  -DLLVM_ENABLE_CURL=OFF \
  -DLLVM_ENABLE_RTTI=OFF \
  -DLLVM_ENABLE_EH=OFF \
  -DLLVM_INCLUDE_TESTS=OFF \
  -DLLVM_INCLUDE_EXAMPLES=OFF \
  -DLLVM_INCLUDE_BENCHMARKS=OFF \
  -DLLVM_INCLUDE_DOCS=OFF \
  -DCLANG_INCLUDE_TESTS=OFF \
  -DCLANG_ENABLE_STATIC_ANALYZER=OFF \
  -DLLVM_BUILD_UTILS=OFF \
  -DLLVM_BUILD_EXAMPLES=OFF \
  -DLLVM_PARALLEL_LINK_JOBS=1 \
  -DLLVM_HOST_TRIPLE="$target" \
  -DLLVM_DEFAULT_TARGET_TRIPLE="$target" \
  -DLLVM_NATIVE_TOOL_DIR="$llvm_bin" \
  -DLLVM_TABLEGEN="$llvm_bin/llvm-tblgen" \
  -DCLANG_TABLEGEN="$llvm_bin/clang-tblgen" \
  -DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_SYSTEM_NAME=InfinityOS \
  -DCMAKE_MODULE_PATH="$repo/sdk/compiler/cmake" \
  -DCMAKE_C_COMPILER="$llvm_bin/clang" \
  -DCMAKE_CXX_COMPILER="$llvm_bin/clang++" \
  -DCMAKE_ASM_COMPILER="$llvm_bin/clang" \
  -DCMAKE_C_COMPILER_TARGET="$target" \
  -DCMAKE_CXX_COMPILER_TARGET="$target" \
  -DCMAKE_ASM_COMPILER_TARGET="$target" \
  -DCMAKE_AR="$llvm_bin/llvm-ar" \
  -DCMAKE_RANLIB="$llvm_bin/llvm-ranlib" \
  -DCMAKE_LINKER="$ld_lld" \
  -DCMAKE_SYSROOT="$sysroot" \
  -DCMAKE_C_FLAGS="$common_flags" \
  -DCMAKE_CXX_FLAGS="$compiler_cxx_flags" \
  -DCMAKE_EXE_LINKER_FLAGS="$compiler_link_flags"
ninja -C "$compiler_build" -j"$jobs" clang lld

rustc --edition 2021 "$repo/tools/native-c-toolchain-test.rs" -o "$work/native-c-toolchain-test"
"$work/native-c-toolchain-test" "$compiler_build/bin/clang.elf" "$compiler_build/bin/lld.elf"
