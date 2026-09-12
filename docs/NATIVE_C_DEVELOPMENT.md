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
The threaded C++ build directory is currently failed; the installed sysroot
contains the earlier non-threaded libraries. Do not treat them as matching the
new configuration. The next implementation must supply real target runtime
services and rebuild consistently, not substitute successful no-op thread calls
or a dummy clock merely to satisfy linkage.
