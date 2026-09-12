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

## Current checkpoint (not feature completion)

- `sdk/c/examples/hello.c` is embedded as a normal Documents Text Object when
  formatting a fresh ObjectStore, shared by live and installed initialization.
  Mounting an existing store does not overwrite or re-create this user document.
- `sdk/c/runtime.c` implements ABI v2 negotiation, `puts`, `putchar`, stream
  cleanup and returning `main`'s status. `sdk/c/stdio.c` supplies bounded
  `fopen`/`fclose`, element reads/writes, seek/tell, EOF/error state and line I/O.
  This is not a complete C standard library: for example `printf`, `errno`,
  allocation, threads and clocks still require the larger runtime port.
- `kernel/runtime/native_c_io.rs` is a safe, typed object-stream adapter with
  explicit path grants, normalized namespace references, independent opened
  snapshots, bounded seek/append, and optimistic write-conflict detection.
  Flush publishes one ObjectStore version; close reports conflicts without
  replacing a newer editor save. Open handles are never recycled within a session.
  It currently shares ObjectStore's 16 KiB per-object limit. It is used by the
  development probe, not yet exposed as a production syscall or console service.
- `kernel/runtime/native_c_image.rs` validates the initial static PIE ELF64
  envelope and copies it into caller-owned memory. It rejects wrong architectures,
  overlapping segments, writable executable segments, dynamic dependencies,
  relocation sections and invalid entry points. Metadata checks do not implement
  hardware page protection.
- `tools/native-c-probe/test.sh` cross-compiles the real source using the build
  host's Clang/LLD, tests x86_64 and AArch64 ELF loading, and boots a small x86_64
  QEMU probe. The probe resolves the provisioned Text Object and creates the executable as
  an ApplicationData Object using production ObjectStore, remounts the store,
  verifies stable identities and source editing, and executes the stored ELF.
- A second Clang-built C executable opens the **edited** source with `fopen`,
  verifies its current bytes, writes and appends a Documents Text Object using
  standard stream calls, and exits. The VM remounts the store and verifies the
  resulting bytes and object type. Host I/O tests also cover access denial,
  aliases to System objects, stale handles, overflow and external-edit conflicts.
- This is a trusted fixture with shared-address-space execution. Do not expose
  it as a launcher for arbitrary user programs. There is no hardware isolation,
  on-device compiler or full fresh-install/desktop acceptance
  yet. The image loader is not wired into the production console.
- The prototype compiler work was set aside under the ignored
  `build/native-c-prototype-archive` directory when Clang was selected.

## Fast verification

Run `sh tools/native-c-probe/test.sh` from the repository. It rebuilds only the
small C artifacts and probe, reuses the boot loader, and does not rebuild the
desktop or ISO. Exit status and typed binary state assertions decide success;
guest log messages do not. Results are in `build/native-c/proof.json`.
The FAT boot volume belongs to the UEFI test harness only; application source
and executable I/O in the guest use ObjectStore, not that volume.

`tools/object-store-test.sh` additionally verifies Documents enumeration, Text
metadata, exact sample bytes and durable user edits. The full
`tools/documents-installed-test.py` scenario now covers opening the seeded sample
through File Navigator's Open With menu on a fresh install and again without the
ISO. That expanded desktop scenario has not yet been run for this checkpoint.
The current store has only 32 namespace slots; the sample consumes one. Expanding
that pre-existing limit needs a separately validated storage-format change.

## Remaining implementation sequence

1. Isolated application execution, fault recovery, cancellation, arguments and
   capability-checked ABI calls.
2. Object-backed C library handles and stream operations, memory allocation,
   clocks, and the C++ support required to host LLVM/Clang.
3. Native Clang/LLD runtime and object-backed compiler resource access; preserve
   diagnostics and disable unsupported platform operations explicitly.
4. Console integration, existing-install sample migration and header provisioning in both
   live and installed System Generations.
5. Verify saved-source edits affect native compilation, then perform the full
   installed-system/ISO-detached acceptance test and review the visible workflow.

Sources: https://clang.llvm.org/docs/Toolchain.html,
https://clang.llvm.org/c_status.html, https://lld.llvm.org/.

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
