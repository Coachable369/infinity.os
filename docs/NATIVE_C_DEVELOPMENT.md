# Native C development: Clang/LLVM

## Acceptance contract

The selected compiler is LLVM Clang with LLD, not TinyCC or a homemade C subset.
The delivered compiler must execute on InfinityOS. A host compiler invoked by a
console wrapper is not acceptable.

`/home/default/documents/hello.c` must be a normal editable UTF-8 Text Object,
visible through File Navigator's Documents favorite and openable in Text Editor.
The native compiler must read its current saved version. Editing, saving and
recompiling must change the executable's observable behavior. Headers, compiler
resources and generated executables must use existing ObjectStore objects and
namespace references. Do not add a separate filesystem, disk format or mount.

The end-to-end workflow is `cc hello.c -o hello`, followed by `./hello`, with
console output and an exit code. Verify the same workflow in an installed system
after cold boot with the ISO detached. Never overwrite a user's existing
`hello.c` when installing or updating the example.

## Current implementation status

Native Clang compiles C, native LLD links it, and the resulting executable runs
at EL0 on ARM64 or CPL3 on x86_64 in disposable QEMU guests. Private page tables,
W^X, guarded stacks, timer preemption, TLS, checked user copies, fault return,
cancellation and finite execution budgets have focused acceptance evidence.
These tests use bounded compiler I/O fixtures; they are not the installed
desktop compiler workflow.

The ObjectStore file service supports live capability checks, namespace grants,
versioned writable snapshots and sealed resource streaming. The native window
API supports ownership-checked geometry, title, focus, capture and lifecycle
operations. The C event ABI uses bounded IEF subscriptions with checked copies
and read-time revocation. Retained pixel surfaces now provide bounded, checked
uploads into a staging plane, explicit commit, resize and failure cleanup.
Normalized keyboard, text, composition and pointer input routes into scoped IEF
events, respecting focus, capture and secure-input exclusion. A shared GUI trap
dispatcher routes the C window, surface and event calls. These adapters compile
with both kernel targets, but the production desktop scheduler does not yet
connect the process path;
unreferenced handlers may be removed by release link-time optimization.

`sdk/c/examples/hello.c` is provisioned as a normal editable Documents Text
Object on fresh format. Existing user documents are never overwritten. The
older trusted C library probe remains a fixture, not a launcher for arbitrary
applications. The dated sections below record earlier port work; the latest
status in this section takes precedence over those historical checkpoints.

## Fast verification

Run `./build-kit native-compiler-storage-tests` for ObjectStore and compiler file
ABI behavior, `./build-kit run make runtime-test` for window/event authority,
and `./build-kit native-process-supervision-tests` for lifecycle behavior.
Use `./build-kit run python3 tools/native-process-probe/toolchain.py --arch
aarch64 --gic 3` or `--arch x86_64` for native compile/link/execute guest proof.
Exit status and structured artifacts decide success; printed prose is diagnostic.

## Remaining implementation sequence

1. Discover native interrupt controllers from firmware and bind bounded process
   visits to the production desktop scheduler, including cleanup after failure.
2. Package Clang, LLD and SDK resources in the installed System Generation;
   support larger writable compiler outputs beyond the 16 KiB snapshot limit.
3. Bind the implemented retained-surface store, GUI trap dispatcher and native
   input adapter to the production process lifecycle, desktop compositor and
   device event loop. The service-level paths are tested; installed GUI apps
   are not yet demonstrated.
4. Connect console compilation, application metadata, icons, double-click launch
   and Hermes voice-generated applications to that isolated process path.
5. Verify editable source, native compilation, GUI interaction, cancellation and
   crash isolation after cold installation with media detached on both targets.
   Generate fresh installer ISOs through `./build-kit full`.

Sources: https://clang.llvm.org/docs/Toolchain.html,
https://clang.llvm.org/c_status.html, https://lld.llvm.org/.

## Retained GUI services checkpoint: 2026-10-12

`native_surface.rs` owns separate displayed and staging planes in the native
frame pool. Limits are 16 MiB per plane, 64 MiB across both planes of all
surfaces, and 64 KiB per checked upload. Apps receive IDs, never framebuffer or
kernel pointers. Commit copies the changed span and damages the corresponding
window region. Moving a window reuses its persistent pixels. Resize preserves
committed overlap and zeroes new pixels; failed allocation preserves old state.
Context failure invalidates exposed bounds and releases only that owner's data.

`native_input.rs` translates trusted normalized events to the 96-byte versioned
C input packet in `sdk/c/include/infinity/input.h`. It follows existing input
routing rather than inventing a second focus/capture model. Surface wrappers
are in `sdk/c/include/infinity/surface.h`. `NativeSurfaces::dispatch` is the GUI
service entrypoint; it does not replace the native process scheduler.

Behavioral runtime tests exercise both ARM64 and x86 page-table implementations,
cross-page uploads, inaccessible pages, integer bounds, capability revocation,
unpublished-pixel isolation, compositor pixel output, damage, resize, budget
rejection and exact frame reclamation. Input checks cover focused Unicode text,
pointer capture, window-local coordinates, scroll, composition and secure-input
exclusion. Both SDK header targets and installed-kernel compilations passed.
These are service/harness results, not installed interactive application proof.
Kernel compilation evidence: `builds/manifests/20261012T112608330428Z-66219.json`.

## Native toolchain port attempt: 2026-09-12

The cross-toolchain experiment is isolated under ignored `build/native-c`.
No host package paths or dependency sources were installed into the OS, and no
user VM was modified. Sources were downloaded from their upstream projects:

| Source | Pin | Archive SHA-256 |
| --- | --- | --- |
| LLVM / Clang / libc++ / LLD | `llvmorg-23.1.0`, commit `ea7d852a70e8bdfaf601d6626a760f9771b2c4b4` | `5878830436d5fc0f460fa756073880e0fc124df74fd5556c798e2ed85dc7bd55` |
| Newlib | `4.6.0.20260123` | `6ff27e3bf022666f43f7802255be680eeff722ac181b1725d21e2e8318604ee3` |

Newlib configured for `x86_64-unknown-elf` and built/installed to the local
sysroot, without supplied syscalls, libgloss, multilib or multithreading.
libc++ and libc++abi also cross-built with threads/exceptions/RTTI/filesystem/
localization disabled. These are **build results**, not runtime verification.

Building the Clang/LLD libraries against that sysroot failed: LLVM's
BalancedPartitioning, RWMutex and ThreadPool still require standard mutexes,
condition variables, shared locks and futures with `LLVM_ENABLE_THREADS=OFF`.
The subsequent pthread-enabled libc++ attempt required enabling its monotonic
clock, then failed in `libcxx/src/chrono.cpp:245` because `CLOCK_MONOTONIC` is
not defined by the target. No synchronization or clock syscall implementation
has been supplied. The two-correction limit was reached; the compiler build
was not continued or represented as successful.

For resumption, generated build directories retain exact CMake caches and Ninja
commands: `cxx-build-x86_64`, `clang-build-x86_64`, and `newlib-build-x86_64`.
At that checkpoint the threaded build was failed. The subsequent continuation
enabled Newlib's monotonic-clock declarations and successfully rebuilt and
installed matching thread-enabled libc++/libc++abi archives. This supplies types
and library code, not working native synchronization or clock services.

The new `sdk/compiler` platform bridge defines explicit clock, mapping,
protection, synchronization and executable-object-path callbacks. Missing
services return errors. Its host behavioral contract test and x86-64 freestanding
compilation pass (`tools/native-c-probe/compiler-platform-test.sh`); neither
proves native page protection or native clocks. The LLVM patch uses the supplied
executable object path instead of assuming Linux procfs. `InfinityOS.cmake`
detects available interfaces rather than declaring the target Linux.

The current compiler build directory is `clang-build-infinity-x86_64`. After two
corrections, LLVM support compilation stops in `ExponentialBackoff.h` because
`std::random_device` is unavailable in the current libc++ configuration. A real
native entropy provider and the corresponding library configuration are still
needed. No complete compiler binary, production runtime integration, installer
payload or installed compilation acceptance exists yet.

## Standard C library compatibility scope

Target broad hosted C library compatibility using the ported Newlib libc/libm,
not just the small bootstrap SDK. Prioritize strings and memory, character
classification, numeric conversion, formatted and buffered I/O, allocation,
sorting/searching, integer types, errors, floating-point math, time, wide
characters and multibyte conversion. Track supported C language/library versions
and exceptions explicitly; this is a target, not a conformance claim.

Architecture requirements remain authoritative:

- File names and descriptors resolve to capability-authorized ObjectStore
  objects and namespaces. Reads, writes, seeking, rename and removal must obey
  native versioning, persistence and access rules; no second filesystem.
- Allocators, clocks, entropy, atomics and synchronization use actual native
  services. No host OS fallback or successful no-op service implementations.
- POSIX extensions are compatibility adapters only where native semantics can
  support them. Process creation, shell execution, signals, sockets and memory
  mapping are not permission bypasses; unavailable operations fail explicitly.
- Locale, timezone data, headers, archives and compiler resources must be
  packaged in both live and freshly installed System Generations through the
  existing object-storage architecture.
- Acceptance exercises numeric results, formatted byte buffers, allocation
  behavior, object versions, access denial and persistence. Final proof compiles
  saved editable C source and runs the result after booting without the ISO.

Third-party C libraries are separate ports with their own dependency, licensing
and behavioral checks; broad libc compatibility does not imply arbitrary Unix
applications already work.

## Random-device correction checkpoint

The `std::random_device` build failure is resolved: libc++ was rebuilt and
installed with `LIBCXX_ENABLE_RANDOM_DEVICE=ON`, and LLVM subsequently compiled
its ExponentialBackoff implementation. `libcxx-infinity.patch` selects
`getentropy` for InfinityOS instead of opening `/dev/urandom`. Both C and C++
target flags include `sdk/compiler/target.h` and `sdk/compiler/include`.

The compiler service table now accepts an entropy callback. `getentropy` bounds
requests to 256 bytes, rejects invalid pointers, returns ENOSYS without a
provider, preserves provider errors, and publishes no partial bytes on failure.
It clears its temporary buffer. Host behavioral tests verify byte forwarding,
boundary handling and failure atomicity; target C compilation also passes.
There is no production entropy callback wired to this compiler yet, and these
tests do not claim that fixture bytes are real randomness.

LLVM next failed in Mustache.cpp because `std::ostringstream` requires libc++
localization. Enabling `LIBCXX_ENABLE_LOCALIZATION=ON` revealed incorrect libc
selection: the generic/IBM locale fallback conflicts with Newlib's `strtod_l`,
`strtof_l` and `strtold_l` declarations and reports an unknown character table.
The two-correction limit was reached. Next resolve libc++'s Newlib selection
in its CMake configuration and rebuild consistently; do not add a dummy locale
table or suppress the errors. The build directory now requests localization,
but its installed sysroot is the last successful non-localized, random-enabled
build. Full compiler linking, native service integration and ISO-detached
acceptance remain outstanding.

## Newlib localization correction checkpoint

Setting `RUNTIMES_USE_LIBC=newlib` selects the proper libc++ locale backend.
The target's signed `char` then exposed two narrowing errors in libc++'s regex
character-class table. The pinned patch now converts the blank/print masks
through their unsigned mask type before widening. This preserves their low-byte
bits without setting the separate regex word bit or changing Newlib's ABI.

The localized, random-enabled libc++/libc++abi archives now build and install
successfully. `sdk/compiler/cmake/runtime-options.cmake` records these runtime
options; load it with `cmake -C` alongside the target/compiler/sysroot flags.
Build `cxx cxxabi` successfully before invoking `install-cxx install-cxxabi`, so
a failed build does not install mismatched headers ahead of its archives.

`sh tools/native-c-probe/locale-test.sh` links the actual target libc++ regex
table and Newlib character operations into a freestanding x86-64 probe. The
disposable QEMU guest passed with exit status 33, verifying blank/print mask
values, separation of the word-class bit, unknown classes, case-insensitive
class expansion and native character classification. Results are recorded in
`build/native-c/locale-proof.json`. The host cross-compiles this probe; this is
native library execution, not native compilation or installed-system acceptance.

LLVM subsequently compiled Mustache.cpp (the previous string-stream blocker).
Its next failure is `Unix/Unix.h` unconditionally including missing `dlfcn.h`
while compiling RandomNumberGenerator.cpp, despite `HAVE_DLOPEN` being absent.
Dynamic loading is not implemented by the current static payload. The next
change must remove that unsupported platform assumption or add a truthful
native adapter, not claim a host dynamic loader exists. The two-correction limit
was reached here; the compiler remains unlinked and uninstalled.

## Dynamic-loading and socket correction checkpoint

The common LLVM Unix header now includes `dlfcn.h` only with `HAVE_DLOPEN`,
matching LLVM's existing unsupported dynamic-loader branch. The target
`RandomNumberGenerator.cpp` object compiles successfully.

InfinityOS's raw Unix socket stream branch explicitly rejects listener and
connection creation with `operation_not_supported`. Reads fail without changing
the caller's buffer, even for an indefinite timeout. It does not adopt supplied
descriptors or create socket files. This is an unsupported compatibility API,
not a native networking implementation. The tracked LLVM patch selects
`sdk/compiler/include/infinity/llvm_socket_unavailable.inc` only for InfinityOS.

TESTED: `sh tools/native-c-probe/socket-unavailable-test.sh` executes those
typed failure paths against host LLVM 23.1.0; compiler-platform contract tests
also pass. Both corrected translation units compile for the freestanding
x86-64 target. Host tests do not establish installed runtime acceptance.

The next full compiler build fails first in `Unix/Path.inc`: Newlib's generic
target has no directory backend (`DIR` is unavailable), and `sys/statvfs.h`
is missing. Concurrent compilation also exposes missing resource-limit and
process-wait interfaces in `Unix/Process.inc` and `Unix/Program.inc` (`rlimit`,
`RLIMIT_CORE`, `RLIMIT_DATA`, `wait4`, `rusage.ru_maxrss`, `_POSIX_ARG_MAX`).
The next continuation must map directory operations to authorized ObjectStore
enumeration and explicitly handle unsupported process/resource operations;
do not fabricate headers with successful no-op semantics.

The two-correction limit was reached at this checkpoint. No linked native Clang
binary or new installer payload was produced. Production service wiring and
ISO-detached compile/edit/recompile/run acceptance remain outstanding.

## Directory and resource-boundary continuation

The original Path, Process, Program and ProgramStack translation units now
compile for the freestanding target. Apply `llvm-resource-errors.patch` after
`llvm-infinity.patch` when preparing the pinned LLVM sources. Both patches were
checked against the original upstream archive.

The compiler platform table now has directory-open/next/close callbacks. The
compatibility adapter owns bounded cursors, binds each to its opening provider,
rejects closed/unknown handles, distinguishes end-of-directory from errors, and
publishes only complete direct-child names. The provider must enforce namespace
authorization, revocation and snapshot semantics. There is no host-directory
fallback. This checkpoint permits 64 lifetime opens per compiler process without
handle reuse; it needs scaling before hosting a real Clang compilation. No
production ObjectStore directory provider is wired yet.

Capacity, link metadata, memory-advice, process-wait and resource-limit queries
return explicit errors where no native provider exists. LLVM resource-limit and
usage callers now check failed queries rather than reading uninitialized output.
Its Unix process-launch path rejects execution on InfinityOS; an Execution
Context launch service must replace that path. These are unsupported boundaries,
not claims of operational POSIX services or a complete hosted C library.

TESTED: `sh tools/native-c-probe/directory-test.sh` verifies denied paths, distinct
cursors, provider binding, EOF, malformed entries, failure atomicity, stale
handles, the open bound, and unchanged output on unavailable resource queries.
Those callbacks are fixtures, not installed storage evidence. Existing compiler
platform tests and freestanding compilation also pass.

The full compiler build next fails in `Unix/Signals.inc:55`, which unconditionally
includes `dlfcn.h`. Native signal/crash symbolization support needs a truthful
platform boundary. Two correction loops reached this new failure; no compiler
binary, new ISO, production provider wiring or detached-media compilation proof
is claimed by this checkpoint.

## dlfcn compatibility boundary

`sdk/compiler/include/dlfcn.h` now declares `dlopen`, `dlsym`, `dlclose`,
`dlerror`, `dladdr`, `Dl_info` and the initial supported flag vocabulary.
`sdk/compiler/dlfcn.c` supplies deterministic failure behavior for the static
toolchain port. This is **not a dynamic linker**: loads (including `dlopen(NULL)`),
symbol lookups, closes and address symbolization report unavailable services.
Invalid binding flags and null required arguments report invalid arguments.
No path, image handle or address is dereferenced to discover host resources.
Failed symbolization leaves the caller's result unchanged.

Errors are thread-local, consumed by `dlerror`, and replaced by the next failed
operation. Production execution therefore requires working native TLS; no
installed TLS acceptance is claimed here. The source is not yet linked into a
production compiler runtime, and `HAVE_DLOPEN` remains disabled in the existing
LLVM configuration. Exporting compatibility declarations does not establish
dynamic-loader availability or authorize loading executable objects.

TESTED: `sh tools/native-c-probe/dlfcn-test.sh` checks return values, error codes,
diagnostic consumption, thread isolation and unchanged result fields, then
cross-compiles the implementation for freestanding x86-64. The host tests rename
the entry points so they cannot interpose on the host's actual dynamic loader.

The full LLVM retry passes the missing-header point but fails in
`Unix/Signals.inc`: Newlib lacks `sigaction.sa_sigaction`, `SA_NODEFER`,
`SA_RESETHAND`, `SA_ONSTACK`, `SA_SIGINFO`, and `siginfo_t.si_pid`. Resolving that
requires a separate native crash/signal boundary, not made-up Unix signal
support. No new ISO or working on-device compiler is produced by this change.

## Native crash registration and target identity checkpoint

Apply `llvm-native-host.patch` after the previous two LLVM patches. InfinityOS
now bypasses Unix signal registration in favor of explicit compiler-host crash
registration/unregistration callbacks. Registration failure is fatal to compiler
startup rather than falsely claiming fault protection. Requests for POSIX
utility signal semantics remain unsupported. The service provider must bind the
callback to its owning Execution Context and must not resume a faulted context.
It is not wired to the production kernel yet.

The adapter preserves the registering provider across service-table changes,
rejects duplicate registration, retains ownership on failed release, and permits
release retry. Calls are serialized by the compiler integration; this initial
platform table is scoped to one compiler process. The callback contract is not
proof of working hardware fault isolation, async-signal safety, or crash recovery.

The native host-version branch preserves the configured target triple without
querying Unix `uname` or inventing a release number. Both host and default target
remain configured as `x86_64-unknown-elf` in this cross-build.

TESTED: crash-hook lifecycle behavior (`crash-hook-test.sh`), actual patched
host-version behavior (`host-triple-test.sh`), existing platform/directory/dlfcn
tests, freestanding platform compilation and upstream patch applicability.
The target Signals.cpp and Host.cpp objects compile successfully.

The full retry advanced through 1,906 build steps, including Clang semantic
analysis and parsing, before failing in `clang/lib/Frontend/CompilerInvocation.cpp`
at the `std::ifstream` used by `-frandomize-layout-seed-file`. The configured
libc++ disables filesystem support, leaving that stream template unavailable.
The next correction must preserve authorized object-backed reads (or explicitly
reject that option), not enable a host filesystem fallback. Two correction
loops completed this checkpoint. Clang is still unlinked, providers remain
unbound, and no ISO-detached native compilation acceptance is claimed.

## Layout seed reader checkpoint

`llvm-seed-reader.patch` replaces the InfinityOS layout-seed option's C++
`ifstream` with an LLVM VFS buffer read. The helper accepts a filesystem instance,
preserves first-line semantics (including empty input and retained carriage
return), and propagates read errors to Clang's existing diagnostic. Other
platforms keep their previous implementation. Its namespace is `infinityos`,
avoiding Newlib's global `infinity()` math function.

TESTED: `seed-file-test.sh` executes the reader against LLVM's in-memory VFS,
including a missing path and different versions of the same path. The target
CompilerInvocation.cpp object now compiles, and the patch applies to the pinned
upstream archive. This is not installed ObjectStore acceptance: the production
LLVM filesystem backend still needs native object-service wiring. No host file
access was used by these behavioral tests, and no filesystem capability is
granted by this helper.

The next retry fails in `clang/lib/Frontend/LayoutOverrideSource.cpp:44`, another
`std::ifstream` dependency. Two corrections were required for this checkpoint;
that separate reader remains unresolved. No linked compiler or new ISO was
produced, and the on-device compile/run goal remains incomplete.

## Layout overrides and ELF linker checkpoint

`llvm-layout-reader.patch` removes the layout-override file-stream dependency.
The default constructor delegates to an overload accepting an LLVM VFS
instance; the parser consumes an in-memory string stream from that snapshot.
Existing parsing semantics remain unchanged. `layout-reader-test.sh` compiles
the actual patched parser and verifies numeric record size, alignment, field
offsets, missing input, CRLF input, and a changed snapshot through in-memory VFS.
Both that behavioral test and the target LayoutOverrideSource.cpp build pass.
This still does not establish a production ObjectStore VFS binding.

The build then reached LLD's Mach-O backend, which required unavailable Apple
compact-unwind headers. `llvm-elf-linker.patch` restricts InfinityOS builds to
the ELF backend and `ld.lld` alias; other platforms retain all existing drivers.
The native dispatch table includes only the ELF driver, consistent with the
current executable loader. Mach-O, COFF and WebAssembly linking are not claimed
for this native toolchain. CMake regeneration and compilation advanced beyond
the missing Apple header; linker executable behavior is not verified yet.

Both new patches apply to the pinned upstream archive. After these two
corrections, the next failure is `clang/tools/driver/cc1_main.cpp:94,97`: the
resource compatibility header lacks `RLIM_INFINITY`. Compiler linking, native
runtime providers, installation and detached-media compile/run acceptance are
still outstanding. No new ISO is produced at this checkpoint.

## Resource constant and cross-link selection checkpoint

The resource ABI now defines the uint64 unlimited sentinel `RLIM_INFINITY`.
This does not claim unlimited resources: unavailable `getrlimit` still returns
ENOSYS without changing the caller's limits. The platform behavioral test covers
that result and passes, together with freestanding platform compilation.

Clang and ELF LLD compilation now reach executable linking. The CMake platform
explicitly passes `CMAKE_LINKER` to the Clang driver using `--ld-path`; setting
CMAKE_LINKER alone had allowed selection of the host macOS linker. The local
cross-build's inherited host OpenSSL search path was also cleared using
`-DCMAKE_EXE_LINKER_FLAGS=`. `linker-test.sh` successfully uses this platform to
build a dependency-free executable and validates its binary ELF64/x86-64 type
and nonzero entry point. It does not execute that artifact on InfinityOS.

After these two corrections, the full link reports missing `crt0.o`, `-lrt`,
and the target compiler-rt builtins archive. LLD also rejects four BLAKE3 assembly
archive members as neither ELF relocatables nor bitcode. Next work must supply
the real native startup/runtime and correct assembly target configuration,
not hide unresolved dependencies. No linked compiler, runtime integration,
fresh-install acceptance or new ISO is claimed.

## Six-round native runtime checkpoint

The correction limit is now six (also already present in repository AGENTS.md).
This pass addressed assembly targeting, compiler builtins, native startup,
runtime autolink assumptions, object I/O, and Newlib's read/write return ABI.

- The CMake platform propagates the C target to assembly. Binary inspection of
  all four BLAKE3 assembly objects now verifies ELF64 x86-64 relocatables.
  SIMD implementations were retained, not replaced by slower portable code.
- `builtins-options.cmake` configures bare-metal compiler-rt from the pinned
  LLVM source. All 171 build steps succeeded, producing
  `build/native-c/builtins-x86_64/lib/generic/libclang_rt.builtins-x86_64.a`.
  This is a cross-build result, not runtime arithmetic acceptance.
- `compiler-options.cmake` removes the stale host librt probe; runtime options
  disable separate pthread/rt library autolinking without disabling threads.
  Matching libc++/libc++abi archives rebuilt and installed into the local sysroot.
  Actual synchronization functions remain required at link time.
- `start.c` and `entry.c` supply an explicit one-shot native launch boundary
  rather than a Unix crt0 stack contract. Tests verify invalid launch rejection,
  argument forwarding, service binding, constructor order, reverse finalizer
  order, exit-status propagation and repeat-launch rejection.
  The launcher must supply validated memory, TLS and isolation before entry.
  This does not yet handle abnormal termination or C-library atexit cleanup.
- `files.c` bridges open/read/write/seek/close to explicit object-provider
  callbacks, retaining provider ownership across table changes. It bounds active
  handles to 128 and uses nonwrapping generations to reject stale descriptors.
  Tests verify mode translation, short reads, write delivery, revocation,
  failed close consumption, read-only enforcement, capacity and repeated reuse.
  Newlib returns int for read/write in this configuration, so requests are
  bounded to INT_MAX and the target declaration is respected.

TESTED: `files-test.sh`, `start-test.sh`, `compiler-platform-test.sh`,
`directory-test.sh`, `dlfcn-test.sh`, `crash-hook-test.sh`, `linker-test.sh`.
These exercise host fixtures and target compilation/artifact state, not installed
ObjectStore access. File callbacks must still be wired to the production native
services; no ambient descriptors 0/1/2 or host filesystem fallback are supplied.

The full link now consumes the startup/platform/files/dlfcn objects and explicit
libc++, libc++abi, Newlib and compiler-rt archives using `-nostdlib` and entry
`infinity_compiler_entry`. Exact invocation remains in the current CMake cache.
Load compiler-options only during initial configuration, before supplying those
explicit runtime link flags. The link still fails: remaining dependencies include
pthread synchronization and TLS destruction, `sbrk`, `_exit`, object metadata and
namespace operations (stat/fstat/access/rename-related support), positional reads,
clocks/waits and unsupported Unix process/signal assumptions. Current diagnostics
are in `/tmp/infinity-native-c-files.log`. These must be implemented using native
services or removed from inapplicable upstream paths, not filled with successful
no-ops. Native providers, safe execution, packaging and ISO-detached acceptance
remain outstanding. No working compiler or fresh ISO is claimed.

## Object metadata, positional reads, heap and timing checkpoint

The file bridge now implements `pread`, `stat`, `fstat` and `access` using
explicit native metadata/read-at callbacks. Metadata conversion checks identity,
size and timestamp representability before changing output. Effective capability
rights map to compatibility permission bits; they do not grant authority to
later operations. Fields with no native mapping remain zero, not measured Unix
user/device statistics. The metadata identity must fit this Newlib ABI; oversized
identities are rejected rather than truncated. A production provider still needs
a collision-free compatibility identity mapping where necessary.

`ObjectIo::read_at` now reads a real opened ObjectStore snapshot without changing
its sequential cursor. The existing Rust object-store harness checks exact
snapshot bytes, extreme offsets and unchanged cursor state. The C callback is
not yet wired to that Rust service in the installed runtime.

`heap.c` supplies Newlib's break allocator only within a launcher-granted,
committed private region. It checks capacity, negative increments including
PTRDIFF_MIN, alignment and provider replacement. This is a serial boundary, not
a page allocator or proof of thread-safe malloc. The launcher still must provide
the region and enforce memory protection.

`time.c` implements gettimeofday, nanosleep, usleep and getpagesize through
explicit observed-time/wait/page-size services. Tests cover conversions, absent
services, interruption remainders, invalid values and preserved failure output.
No guessed page size, clock or successful no-op sleep is used.

TESTED: files-test, heap-test, time-test, start-test, compiler-platform-test,
directory-test, crash-hook-test, dlfcn-test and the existing native C probe.
The QEMU probe returned guest exit code 33 and passed object round-trip, trusted
native execution and C stdio object-I/O assertions. That probe cross-compiles on
the host: on-device compiler, hardware isolation and installed acceptance remain
false. Its existing boot-loader build used the current working tree; unrelated
boot edits were not changed or committed as part of this pass.

The full Clang/LLD link was retried with the new files/heap/time objects. It remains
blocked by synchronization/TLS, termination, additional namespace operations and
Unix process assumptions. Diagnostics: `/tmp/infinity-native-c-metadata.log`.
All new adapters remain development runtime components, not a delivered native
compiler. No ISO was rebuilt and no user's VirtualBox VM was modified.

## Serial compiler synchronization and native invocation checkpoint

The existing LLVM build has LLVM_ENABLE_THREADS=OFF. Its C++ synchronization
requirements now have an explicitly single-thread implementation rather than
successful no-op locks. Startup rejects launchers that do not guarantee
serial_execution=1. This is not an OS-wide pthread implementation.

`serial_sync.c` implements normal and recursive mutex ownership, busy/deadlock
errors, unlock validation and destroy-while-held rejection. `serial_tls.c`
implements 128 nonrecycled invocation-local keys and 256 LIFO thread-destructor
registrations with bounded cleanup. These are serial runtime data structures,
not hardware TLS setup; ELF TLS register/segment initialization remains part of
the missing execution integration. Dynamic library unloading is unsupported.

`pthread.c` translates Newlib's actual types to that implementation. Additional
thread creation and condition waits return ENOTSUP. Condition notification with
no possible concurrent waiter is supported; it never manufactures a successful
wait or timeout. Parallel compiler options are not supported by this first
serial runtime. Ordinary lock/key operations require the explicit launch flag.

TESTED: serial-sync-test exercises lock/key/destructor state transitions on the
host and compiles the Newlib wrappers. The expanded QEMU native C probe loads
and executes the actual x86-64 wrapper ELF, testing recursive ownership, busy
locks, native key cleanup, invalid conditions and explicit unsupported thread
operations. It returned debug exit 33 with native_serial_sync_abi=true.
Existing object I/O and trusted execution assertions also passed. This is not
an on-device compiler or installed-system acceptance result.

`llvm-native-invocation.patch`, applied after the preceding LLVM patches,
removes Unix signal masking from native descriptor close, obtains home from
the explicit native launch namespace, leaves other-user tilde expressions
unexpanded instead of querying a password database, and disables requests for
POSIX utility signal handlers in generated native drivers. Native crash-handler
registration is not removed. Reverse dry-run validates the stored patch against
the changed source; target compilation reaches the linker. Home-provider tests
check absence and exact namespace forwarding. Linker garbage collection is now
recorded in the CMake platform and its ELF artifact test passes.

After this pass the full link still has 19 unresolved functions, including
CrashRecoveryContext's sigaction/sigprocmask, termination (_exit/kill), namespace
mutation/canonicalization, descriptor controls, working-directory and identity
queries. The current full-link diagnostics are
`/tmp/infinity-compiler-serial-final.log`. Native crash containment, provider
binding, linker execution, resource packaging and detached-install acceptance
remain incomplete. No compiler executable or ISO release is claimed.

## Namespace and descriptor checkpoint

TESTED: native ObjectIo working-directory changes, canonical resolution,
namespace creation, content-reference removal, and truncation. Behavioral
tests exercise real ObjectStore state, authority rejection, open-writer
conflicts, zero-filled extension and cursor preservation. The C namespace
bridge also rejects malformed provider output without publishing partial paths.
Descriptor flags and writable-provider truncation are implemented in files.c.
These C callbacks are not yet bound to an installed compiler execution context.

The ten focused SDK checks and QEMU native C probe pass. The guest returned
exit 33, including object I/O and serial synchronization. Its proof still
reports on_device_compiler=false, hardware_isolation=false and
installed_acceptance=false. This does not demonstrate native compilation.

The full Clang/LLD link after these changes still fails on 12 functions:
sigaction, sigprocmask, getpid, isatty, readlink, _exit, kill, link, dup2,
symlink, gethostname and getsid. Diagnostics are in
`/tmp/infinity-compiler-namespace-link.log`. No successful link is claimed.

Completion also requires actual execution integration: native_c_image currently
limits images to 256 KiB and rejects TLS/dynamic records, while execution.rs
does not switch page tables. Compiler-sized loading, hardware TLS, fault
containment, standard-stream/service binding, compiler invocation and installed
payload acceptance remain unimplemented. Resolving linker names alone cannot
close those gates. No compiler ISO was produced in this checkpoint.

## Process, console and reference boundary checkpoint

The SDK now defines every symbol from the preceding 12-function link failure.
Native process and session identities come only from the launch service. Node
identity is bounded and launch supplied. `kill` supports a read-only liveness
query but rejects signal delivery, while `_exit` requests native context
termination and traps if a provider incorrectly returns. POSIX `sigaction` and
signal masks remain explicit unsupported adapters rather than fabricated fault
containment.

Standard streams 0, 1 and 2 are now launch-bound console services. Reads are
limited to stdin, writes to stdout/stderr, terminal status is queried through
the provider, and metadata identifies an actual bound character stream. LLVM's
CrashRecoveryContext patch no longer compiles or installs Unix signal handlers
for InfinityOS; explicit HandleExit recovery remains, while hardware faults are
owned by the containing native Execution Context.

Hard links map to additional ObjectStore namespace references to the same stable
ObjectId. The production ObjectIo operation enforces source-read and destination-
write grants, preserves identity, rejects conflicting names and does not copy
content. Symbolic references remain unsupported until the object model gains a
truthful target-reference representation. The C bridge preserves readlink output
on provider failure.

TESTED: process identity/session/node behavior, terminal detection, liveness
queries, explicit signal rejection, console read/write/metadata, C namespace
reference forwarding, and real ObjectStore hard-reference identity and removal.
The updated LLVM patch dry-runs against the pinned upstream sources. All host
behavioral tests pass. `build-compiler-runtime.sh` now assembles the complete
native bridge object set, including the new process boundary, against a restored
target sysroot. The ignored Newlib/LLVM build directory was absent at this
checkpoint, so target compilation and the full Clang link were not rerun. A
linked compiler, production provider binding, compiler-sized execution,
packaging and detached-install acceptance remain outstanding.

## Static hardware Thread-Local Storage

The native ELF loader now accepts one validated `PT_TLS` template. It rejects
duplicate templates, truncation, oversized allocations, invalid alignment and
templates whose initialized bytes are not backed by a load segment. Empty TLS
records emitted for non-TLS programs are harmless. The application linker script
now emits `.tdata` and `.tbss` with an explicit TLS program header.

`kernel/runtime/native_tls.rs` initializes unique caller-owned blocks: `.tdata`
is copied and `.tbss` is zeroed for each context. Static TLS is capped at 64 KiB,
with power-of-two alignment up to 4096 bytes; templates must start aligned.
x86-64 uses variant II data before the thread pointer and a self pointer at
FS:0. AArch64 uses a 16-byte TCB followed by aligned data, addressed through
TPIDR_EL0. These follow the [ELF TLS layout](https://www.uclibc.org/docs/tls.pdf)
and [AArch64 ABI](https://github.com/ARM-software/abi-aa/blob/main/sysvabi64/sysvabi64.rst).

The privileged `replace_thread_pointer` seam saves/installs IA32_FS_BASE or
TPIDR_EL0 and returns the previous pointer. Its caller must keep storage alive,
pin execution to the CPU, prevent conflicting context switches and restore the
previous pointer. It does not provide address-space isolation, pthread creation
or scheduling. Initial-exec/local-exec static images are the scope; dynamic TLS
modules, TLS relocations and dynamic loading remain rejected.

TESTED: real Clang-generated TLS ELF templates for x86-64 and AArch64;
initialization, zero fill, alignment, separate allocation and malformed bounds.
Existing non-TLS image-loader tests pass. `make native-tls-test` boots an isolated
x86-64 QEMU probe, executes actual `_Thread_local` accesses over 128 alternating
activations of two blocks, verifies independent values and restores the previous
FS base. Guest exit is 33. This target is included in `build.sh` and requires no
Newlib sysroot. The broader native C probe currently cannot rebuild because its
Newlib `errno.h` is missing; that is separate from the passing TLS-only probe.

The loader module is included in both live and installed kernel configurations.
Production native-app launch/scheduler integration, AArch64 hardware execution,
full Clang execution and ISO-detached installed acceptance are still pending.
The trusted probe is not evidence that the installed compiler is complete.

## Native Clang and LLD link checkpoint

`tools/build-native-c-toolchain.sh` now provides a reproducible development
build for the x86-64 compiler stack. It downloads SHA-256-pinned LLVM and
Newlib sources, applies the InfinityOS patch series, builds the Newlib sysroot,
compiler-rt builtins, libc++, libc++abi and the runtime bridge, then links
Clang and ELF LLD as native static executables. The basic compiler build omits
Clang's optional static analyzer. InfinityOS also leaves LLVM's Unix alarm
watchdog disabled because native crash and termination ownership belongs to the
containing Execution Context. ELF LLD uses initial-exec TLS to match the native
static-image TLS contract.

TESTED: the complete build script succeeds from the pinned inputs. The produced
Clang and LLD files are x86-64 `ET_EXEC` ELF images with nonzero entry points,
executable load segments, nonempty static TLS segments and no unresolved strong
symbols, as verified from their binary ELF structures by
`tools/native-c-toolchain-test.rs`.

This checkpoint does not package or execute either tool. The current artifacts
are approximately 99 MiB for Clang and 56 MiB for LLD, beyond the existing
256 KiB native image boundary. Compiler-sized ObjectStore loading, production
provider wiring, shell integration, installed payload parity and fresh
ISO-detached `cc hello.c -o hello` execution remain separate work. No installed
compiler acceptance is claimed here.

## Compiler image loading checkpoint

The production native ELF parser now has a separate compiler policy. Ordinary
applications retain the existing 256 KiB `ET_DYN` boundary. Compiler images may
be x86-64 `ET_EXEC` files up to 128 MiB, with fixed virtual addresses normalized
to isolated destination offsets. The parser retains executable/data segment
permissions, static TLS and GNU RELRO ranges, rejects executable GNU stacks,
and continues to reject dynamic loaders, dynamic relocations, overlapping
segments and insufficient destination memory.

TESTED: the ordinary x86-64 and AArch64 image suite still passes. The production
parser and copier load the exact linked 99 MiB Clang and 56 MiB LLD artifacts,
compare every initialized segment with its source, verify every BSS tail is
zeroed, and reject both undersized destinations and attempts to admit those
executables through the ordinary-app policy. The installed x86-64 kernel build
also succeeds.

Execution is now blocked at a larger architecture boundary rather than ELF
parsing. The kernel has no runtime page allocator or per-context page-table
mapper; `ExecutionManager` records ownership ranges but cannot map or protect
them. The current ObjectStore allocation bitmap addresses only 1,968 4-KiB
blocks (about 61.5 MiB total), and its largest streamed extent is 1 MiB, so it
cannot truthfully store even the 99 MiB Clang image. Expanding that metadata
format requires a versioned on-disk layout and migration, not a relaxed size
constant. No unisolated compiler execution or out-of-band host filesystem
fallback is introduced.

## Scalable system-object storage checkpoint

ObjectStore format v6 expands allocation coverage from 1,968 to 65,536 bytes
of bitmap state, addressing up to 2 GiB while retaining every existing content
extent coordinate. The additional alternating-bank bitmap sectors live beyond
the complete v5 allocation range and are permanently reserved from content.
Compact devices retain the inline map. Versions 4 and 5 remain readable; the
next successful mutation on a sufficiently large v5 store publishes v6 metadata
and exposes the additional capacity without moving existing content.

Trusted installation code can reserve, stream, integrity-check, atomically seal
and range-read system components up to 128 MiB. Large staging and sealed roles
are distinct from Pool transfers and ordinary 16-KiB objects; incomplete bytes
remain unresolvable, and ordinary reads/writes cannot bypass the streaming API.

TESTED: the full ObjectStore behavioral suite passes. A sparse-disk acceptance
test verifies at least 128 MiB of addressable storage, a 100 MiB compiler-image
reservation, a fully streamed and CRC-sealed multi-megabyte component, bounded
range reads, ordinary-API rejection, remount persistence, and a compact v5 to
scalable v6 mutation preserving namespace identity.

## Native process memory checkpoint

BootInfo v9 now transfers a loader-reserved 256 MiB physical frame arena, exact
kernel RX, R/NX and RW/NX segment bounds, the kernel stack, and the boot CR3.
The loader reserves the arena before the final UEFI memory map and expands the
temporary identity map to 8 GiB so firmware may place the UEFI handoff above
4 GiB on a 4 GiB guest.

The kernel frame allocator uses fixed bounded metadata, exact owner-bearing
allocation tokens, zero-on-allocation, and rejects forged or partial releases.
The x86-64 mapper constructs independent four-level roots from those frames. A
compiler address space maps ELF text RX, read-only data R/NX, mutable data and
heap RW/NX, transitions GNU RELRO to R/NX, leaves both stack guard pages absent,
keeps kernel code/data/stack supervisor-only, and maps only an explicit user RX
gateway for future kernel requests. CR3 activation enables CR0.WP and EFER.NXE.
Execution Contexts now bind the resulting 64-bit page-table root only after the
mapping transaction succeeds.

TESTED: `make native-memory-test` walks the produced page tables in memory and
verifies translations and effective permissions, independent roots, exact frame
ownership, forged-release rejection, guard faults by absence, and complete
reclamation. Both UEFI loaders compile, both installed and live x86-64 kernels
link, and a 4 GiB QEMU boot reaches the Rust kernel after reserving the arena.

This does not yet execute Clang. Secure execution still requires x86-64 ring-3
entry, a TSS/privileged kernel stack, a trap or syscall dispatcher behind the
gateway ABI, saved-register context switching, page-fault termination, and then
fresh installed ISO-detached `cc hello.c -o hello` execution. Until those are
complete, the production scheduler must not activate an untrusted compiler CR3.

## Hardware-isolated execution checkpoint — 2026-10-11

The shared address-space builder now supports x86-64 and AArch64. Disposable
QEMU guests execute compiler-produced C at CPL3/EL0, preempt an infinite loop,
reject supervisor writes and executable heap access, and survive privileged
instruction faults. Repeated resumptions preserve TLS and SIMD registers.
All process frames are reclaimed after each case. The native C syscall header
passes six arguments and receives results through the real privilege boundary.

The window service adds capability-checked title, movement, resize, state, focus,
capture, and close operations with checked user-memory copies. Behavioral tests
cover ownership, grant revocation, modal focus, z-order, invalid pointers, and
capture cleanup. This is service-level evidence, not rendered desktop evidence.

The AArch64 Clang/LLD artifacts now build against native Newlib bridges. Their
4-KiB-aligned ELF segments load through the compiler image policy. This does not
yet prove that Clang compiles a program inside InfinityOS.

Evidence, produced exclusively through the build kit:

- `builds/manifests/20261012T044620932094Z-37909.json`: successful ARM compiler build.
- `builds/manifests/20261012T044644485911Z-38160.json`: memory/window/runtime tests
  and x86, ARM GICv2, and ARM GICv3 user-execution probes.
- `build/native-process-probe/{x86_64,aarch64-gic2,aarch64-gic3}/evidence.json`:
  structured fault, privilege, preemption, and reclamation results.

Remaining acceptance gates: production process scheduling and trap dispatch;
compiler service syscalls and resource packaging; complete C window/event/render
integration with the desktop; in-guest compilation and executable launch; and
fresh-installed, ISO-detached verification on both targets. The existing ISO
release does not contain this checkpoint. None of these remaining gates is
claimed complete by the disposable execution probes.

## Native compiler execution checkpoint 2026 10 12

Clang and LLD now compile and link real C inside disposable InfinityOS guests
on AArch64 and x86_64. The resulting executable enters a separate EL0 or CPL3
address space and returns the expected result, 54. Compilation, linking and
application execution reclaim all owned frames. The host builds the toolchain
and probe kernel, but does not compile or link the test C program.

`sdk/compiler/isolated.c` replaces privileged callback entry with checked syscall
marshalling. The launch record contains private addresses and process metadata,
not kernel function pointers. LLVM's optional crash cleanup notification can be
unavailable without preventing startup; Unix signal recovery remains disabled
because the supervisor owns fatal process termination. Newlib and compiler
builtins use position-independent code for the compiler's high virtual address.

The ELF loader supports bounded, symbol-free relative relocations for private
callback tables. Dynamic anonymous mappings use shared ownership and page-budget
accounting, reject executable allocation, and clean up on exit. Window creation
now records an exclusive surface association; close releases that association.

The reproducible native pipeline is:

```
./build-kit run python3 tools/native-process-probe/toolchain.py --arch aarch64
./build-kit run python3 tools/native-process-probe/toolchain.py --arch x86_64
```

Structured results, generated objects, executables and checksums are retained in
`builds/validation/native-apps/aarch64-gic3-compiler/` and
`builds/validation/native-apps/x86_64-compiler/`. The verification batch manifest
is `builds/manifests/20261012T055043516630Z-56355.json`; it also covers the two
hardware page-table encodings, dynamic mapping budgets and runtime window tests.

These probes use bounded input/output streams and read-only linker fixtures.
They do not yet provide the installed ObjectStore compiler resource adapter,
production process scheduling, desktop rendering/events for C apps, or the
voice-to-application workflow. The native compiler is not yet installed by the
ISO. Fresh-installed, ISO-detached acceptance and updated release ISOs remain
required; neither target is complete on the strength of these probes alone.

### Object service authority checkpoint

`kernel/runtime/native_c_service.rs` now binds an ObjectStore stream session to
the launching security identity, namespace grants, and live read/write
capabilities. Every operation rechecks holder, scope, rights, expiry and
revocation, including operations on previously opened handles. Revoked close
discards pending output rather than publishing it. Process cancellation erases
all stream snapshots and permanently rejects further requests.

The production ObjectStore behavioral harness verifies persisted writes,
cross-process rejection, revoked writes/flush/close, expired reads without
output-buffer mutation, wrong-scope denial, and cancellation without version
changes. The storage and runtime verification batch passed under manifest
`builds/manifests/20261012T055822644071Z-57066.json`.

This is the typed service boundary, not yet a production trap-dispatch binding.
Writable snapshots remain limited to 16 KiB. Sealed SystemComponent resources
now stream through the same grant-checked file adapter without whole-file
buffering. Larger linked outputs still require a writable extent adapter.
Installed acceptance remains open as described above.

### Compiler file syscall adapter

`kernel/runtime/native_c_syscall.rs` decodes the existing compiler file ABI into
the capability-checked ObjectStore session. It supports open, read, positional
read, write, seek, close, truncate, metadata inspection and namespace operations.
Transfers are bounded to 4 KiB per call and return the completed byte count.
Every output range is validated before advancing a cursor or mutating storage;
user addresses are translated through the owning hardware page tables.

Metadata describes the opened snapshot, including buffered length. Path metadata
reports current effective read/write rights. Execute permission is not implied.
ObjectStore logical modification counters are not Unix time, so timestamps
remain zero until a real wall-clock mapping exists. Unsupported calls return
Newlib ENOSYS rather than simulated success.

Run `./build-kit native-compiler-storage-tests` for the behavioral suite. It
exercises both ARM64 and x86_64 page-table encodings against real ObjectStore
operations, including cross-page paths, invalid output pages, supervisor input,
forged descriptor high bits, positional-read cursor preservation, metadata and
capability revocation. These host-executed tests validate the adapter, not a
production scheduler binding or installed compiler workflow.

The combined storage ABI, runtime, and ARM64/x86_64 installed-kernel compilation
batch passed under `builds/manifests/20261012T074030521377Z-60216.json`.

### Shared native execution supervision

`kernel/runtime/native_cpu.rs` now owns the hardware context adapter previously
located in the probe tools. Both probes and installed-kernel builds consume this
same implementation. Interrupt-controller addresses come from the caller;
platform entry failures return errors instead of panicking the kernel.

`native_process.rs` supervises one hardware slice per scheduler visit. It handles
exit and yield, enforces a finite slice budget and optional monotonic deadline,
binds service dispatch to the launch identity, and prevents re-entry after
cancellation, faults or exit. The scheduler caller remains responsible for
retaining mappings while runnable and releasing windows, streams and memory
after a terminal outcome. Service handlers must also remain bounded.

The guest-linked C application now runs through this supervisor on both CPU
targets and still returns 54. Compile/link/execute evidence is recorded under
`builds/manifests/20261012T074733038876Z-61118.json`. Host lifecycle tests run with
`./build-kit native-process-supervision-tests`; they test cancellation, deadlines,
budgets, dispatch identity and terminal-state behavior independently of hardware.

The ARM adapter also deactivates the timer interrupt when GICv3 uses split EOI
mode. The hardware probe checks two consecutive busy-loop preemptions, preventing
an active timer interrupt from silently disabling subsequent time slices.

Both installed-kernel compilations and the architecture regression batch passed
under `builds/manifests/20261012T074913699005Z-61279.json`. The final split-EOI
readback, lifecycle and runtime checks passed under
`builds/manifests/20261012T075146247584Z-61604.json`.

Desktop-loop scheduling, firmware controller discovery, calibrated x86 slice
timing, installed compiler resources, and C window/event/render integration
remain acceptance gates. No new installer release is implied by these checks.

### Resource streaming and native C events

Sealed compiler resources use positional ObjectStore reads capped at 64 KiB;
the compiler syscall boundary retains its smaller 4 KiB transfer limit. Open
handles preserve independent cursors and immutable resource identity. Seeks
are bounded by resource length. Writes and truncation remain denied even when
a namespace grant is writable. Remount, cross-block reads, short reads, EOF,
cursor independence and denied mutation pass the storage behavior suite under
`builds/manifests/20261012T080046115400Z-62079.json`.

`sdk/c/include/infinity/event.h` defines the 128-byte native event packet and
subscribe, poll and unsubscribe calls. `native_event.rs` adapts these operations
to IEF rather than introducing another message queue. Every read checks holder,
lease, scope, expiry and live capability authority. A bad user pointer leaves
the event queued; overflow sets a reconciliation flag. Unsubscription permits
owner cleanup after revocation but rejects other processes.

The runtime behavioral suite exercises this event ABI using both ARM64 and
x86_64 page-table implementations. The suite and both installed-kernel
compilations passed under
`builds/manifests/20261012T080828435900Z-62602.json`. This is service-adapter acceptance, not
installed native application acceptance: the production process dispatcher
still needs to route these calls, and input/render wiring remains incomplete.

### Clean release checkpoint — 2026-10-12

`./build-kit full` completed successfully with manifest
`builds/manifests/20261012T081205430107Z-62990.json` (revision `66ad31b`,
including the pre-existing workspace changes). Binary installed-kernel/loader
parity and final model-bundle/artwork checks passed for both architectures.

| Installer | SHA-256 |
| --- | --- |
| `builds/InfinityOS-aarch64.iso` | `b3334371afb6d72e55daa22f42203b352e600d6cd598297e103ac2f24855257e` |
| `builds/InfinityOS-x86_64.iso` | `15393905cd89661c4ebb0d46957c78948abce14352f08f0dd424a985f1d8ebf0` |

This is a release-build checkpoint, **not completion of native app creation**.
Clang/LLD and SDK resources are not yet installed by this release. Production
process launch/dispatch, retained app pixel surfaces, desktop input routing,
package/double-click launch and Hermes app creation remain incomplete. Neither
ISO has been cold-installed to prove that complete workflow; no existing VM was
upgraded by building these files.

The post-build runtime behavioral suite, C event-header code generation for
both CPU targets, and independent checksum verification passed under
`builds/manifests/20261012T093531684229Z-63761.json`. The header fixture is
`tools/native-c-probe/event-api.c`; its compile check is not guest execution.

### Firmware controller handoff

BootInfo v10 appends a typed interrupt-controller descriptor while retaining
the existing segment and memory-pool fields. Live and installed UEFI loaders
share discovery. Unsupported hardware yields an unavailable descriptor rather
than a guessed address. The legacy 32-bit bootstrap now initializes the full
392-byte contract instead of emitting its obsolete v8 prefix.

ARM discovery validates MADT and GTDT checksums, record lengths, the enabled
boot CPU affinity, physical timer PPI30, and bounded GICv3 redistributor ranges.
GICv2 supplies distributor and CPU-interface addresses; GICv3 supplies the
matching redistributor. x86 discovery accepts enabled memory-mapped xAPIC only;
x2APIC is not implemented. `native_cpu::controller_from_handoff` rejects
cross-architecture kinds, nonzero reserved fields and invalid address alignment.

The ARM loader now reserves the same 256 MiB native process arena as x86.
Previously its production handoff supplied zero, even though bare-metal process
tests used their own arena. ARM slices preserve and mask the firmware virtual
timer, then restore it on return. GICv2 retains the firmware-selected security
group for PPI30 instead of moving it into a potentially disabled group.

`./build-kit native-controller-tests` exercises actual parser results, including
corrupt checksums, malformed records, missing CPUs, unsupported timers, address
bounds and MMIO read limits. This suite also runs before ISO generation.
`tools/native-process-probe/run.py --arch aarch64 --gic 2|3 --firmware`
boots the production loader in a disposable guest and runs the same twelve
hardware isolation cases as the bare-metal probe. The x86 probe now consumes
the production handoff instead of hard-coding its APIC address. Binary case
records, not console prose, determine acceptance. These are firmware-backed
guest tests, not a cold-installed desktop application test.

Production desktop process scheduling, safe handling of unrelated interrupts,
x86 descriptor-state restoration and calibrated slices, installed Clang/LLD
resources, package/double-click launch and Hermes app creation still need
integration and installed-system verification. Controller discovery does not
by itself enable arbitrary native application execution in the desktop.
