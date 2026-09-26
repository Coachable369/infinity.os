"""Name the native SpiderMonkey target honestly without impersonating a Unix OS."""
import importlib.util
import hashlib
import os
from pathlib import Path
import subprocess
import tomllib

# ------------------------=
# FUNC: main
# DESC: Normalizes Rust's float-ABI suffix only for configure and adds the explicit Infinity OS identity.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    spec = importlib.util.spec_from_file_location("staging", Path(__file__).with_name("prepare-async-net.py"))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    servo = root / "build/servo-port-audit"
    original = subprocess.check_output(["git", "-C", str(servo), "show", "HEAD:Cargo.lock"], text=True)
    directory, package = helper.stage(root, tomllib.loads(original)["package"], "mozjs_sys", "153.3.0-0")
    path = directory / "makefile.cargo"
    text = path.read_text().replace('CONFIGURE_FLAGS += --target=$(TARGET)', 'CONFIGURE_FLAGS += --target=$(subst -none-softfloat,-none,$(TARGET))', 1)
    # Configure probes the compiler before applying CFLAGS; carry the native
    # identity in the target compiler command, never in HOST_CC/HOST_CXX.
    revision = hashlib.sha256(Path(__file__).read_bytes() +
                              (root / "sdk/servo-std/TimeStamp_infinity.cpp").read_bytes() +
                              (root / "sdk/servo-std/c-target.h").read_bytes()).hexdigest()
    text = ('NATIVE_PORT_REVISION := ' + revision + '\n'
            'NATIVE_CC_FLAGS := $(if $(findstring -none,$(TARGET)),$(CFLAGS),)\n'
            'NATIVE_CXX_FLAGS := $(if $(findstring -none,$(TARGET)),$(CXXFLAGS),)\n') + text
    for compiler in ("CC", "CXX"):
        text = text.replace(compiler + '="$(' + compiler + ')"',
                            compiler + '="$(' + compiler + ') $(NATIVE_' + compiler + '_FLAGS)"')
    text = text.replace('CONFIGURE_INPUTS := "', 'CONFIGURE_INPUTS := "$(NATIVE_PORT_REVISION)$(NATIVE_CC_FLAGS)$(NATIVE_CXX_FLAGS)', 1)
    text = text.replace('if [[ $(JSSRC)/configure -nt config.status ]] ; then',
                        'if [[ $(JSSRC)/configure -nt config.status || ! -f Makefile ]] ; then', 1)
    path.write_text(text)
    path = directory / "mozjs/build/moz.configure/init.configure"
    text = path.read_text()
    marker = '    elif os.startswith("wasi") and allow_wasi:'
    if text.count(marker) != 1:
        raise SystemExit("Unexpected SpiderMonkey platform selector")
    path.write_text(text.replace(marker, '    elif os == "none":\n        canonical_os = canonical_kernel = "Infinity"\n' + marker, 1))
    path = directory / "mozjs/python/mozbuild/mozbuild/configure/constants.py"
    text = path.read_text()
    for kind in ("OS", "Kernel"):
        marker = 'class ' + kind + '(EnumString):\n    POSSIBLE_VALUES = ('
        if text.count(marker) != 1:
            raise SystemExit("Unexpected platform enum")
        text = text.replace(marker, marker + '\n        "Infinity",', 1)
    marker = 'kernel_preprocessor_checks = {'
    if text.count(marker) != 1:
        raise SystemExit("Unexpected kernel compiler checks")
    text = text.replace(marker, marker + '\n    "Infinity": "__INFINITYOS__",', 1)
    path.write_text(text)
    path = directory / "mozjs/build/moz.configure/flags.configure"
    text = path.read_text()
    text = text.replace('@depends(c_compiler, link, linker_ldflags, extra_toolchain_flags)',
                        '@depends(c_compiler, link, linker_ldflags, extra_toolchain_flags, target)', 1)
    text = text.replace('def expand_libs_list_style(c_compiler, link, linker_flags, extra_flags):',
                        'def expand_libs_list_style(c_compiler, link, linker_flags, extra_flags, target):', 1)
    marker = '        ldflags = linker_flags + extra_flags'
    if text.count(marker) != 1:
        raise SystemExit("Unexpected linker response-file probe")
    text = text.replace(marker, marker + '''
        if target.kernel == "Infinity":
            # This tests response-file consumption, not C runtime startup.
            # Link a real freestanding ELF with the probe's own entry point.
            ldflags += ["-nostdlib", "-Wl,-e,main"]
''', 1)
    path.write_text(text)
    path = directory / "mozjs/mozglue/misc/timestamp.mozbuild"
    text = path.read_text()
    marker = 'elif CONFIG["HAVE_CLOCK_MONOTONIC"]:'
    if text.count(marker) != 1:
        raise SystemExit("Unexpected native timestamp selection")
    path.write_text(text.replace(marker, '''elif CONFIG["OS_TARGET"] == "Infinity":
    SOURCES += ["/mozglue/misc/TimeStamp_infinity.cpp"]
''' + marker, 1))
    (path.parent / "TimeStamp_infinity.cpp").write_bytes((root / "sdk/servo-std/TimeStamp_infinity.cpp").read_bytes())
    path = directory / "mozjs/js/src/util/NativeStack.cpp"
    text = path.read_text().replace('#ifdef XP_WIN', '''#if defined(__INFINITYOS__)
#  include <stdint.h>
// ------------------------=
// FUNC: infinity_std_stack_bounds
// DESC: Queries stack ownership from the native executor, not pthreads or guessed frame addresses.
// ------------------=
extern "C" int infinity_std_stack_bounds(uintptr_t*, uintptr_t*);
#elif defined(XP_WIN)''', 1)
    text = text.replace('#if defined(XP_WIN)', '''#if defined(__INFINITYOS__)
// ------------------------=
// FUNC: GetNativeStackBaseImpl
// DESC: Uses the live native worker's actual upper stack boundary for downward-growing targets.
// ------------------=
void* js::GetNativeStackBaseImpl() {
  static_assert(JS_STACK_GROWTH_DIRECTION < 0);
  uintptr_t low = 0, high = 0;
  MOZ_RELEASE_ASSERT(infinity_std_stack_bounds(&low, &high) == 0 && high > low);
  return reinterpret_cast<void*>(high);
}
#elif defined(XP_WIN)''', 1)
    path.write_text(text)
    path = directory / "mozjs/build/moz.configure/toolchain.configure"
    text = path.read_text().replace('("Darwin", "FreeBSD", "OpenBSD")',
                                    '("Darwin", "FreeBSD", "OpenBSD", "Infinity")', 1)
    text = text.replace('visibility_flags and target.kernel != "Darwin"',
                        'visibility_flags and target.kernel not in ("Darwin", "Infinity")', 1)
    path.write_text(text)
    path = directory / "mozjs/build/moz.configure/memory.configure"
    text = path.read_text()
    marker = 'malloc_h = check_malloc_header()'
    if text.count(marker) != 1:
        raise SystemExit("Unexpected allocator header selector")
    text = text.replace(marker, '''detected_malloc_h = check_malloc_header()
# ------------------------=
# FUNC: malloc_h
# DESC: Selects the standard native allocator declarations without Unix allocator extensions.
# ------------------=
@depends(target, detected_malloc_h)
def malloc_h(target, detected):
    # Newlib malloc.h disagrees with its own stdlib.h exception specs.
    # Native allocation uses the standard declarations and fallback paths;
    # do not import optional host allocator extensions.
    return "stdlib.h" if target.kernel == "Infinity" else detected
''', 1)
    path.write_text(text)
    lock = servo / "Cargo.lock"
    header = 'name = "mozjs_sys"\nversion = "153.3.0-0"\n'
    entry = header + 'source = "' + package["source"] + '"\nchecksum = "' + package["checksum"] + '"\n'
    lock.write_text(lock.read_text().replace(entry, header))

if __name__ == "__main__":
    main()
