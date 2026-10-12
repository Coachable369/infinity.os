# Native Text Editor compilation

## Contract

The attached editor assistant can propose `REPLACE_COMPILE` with complete C
source, or compile the current document on an explicit compile request. Generated
source remains a reviewed, revision-fenced, undoable editor operation. Compilation
does not automatically execute the result.

The desktop captures an immutable source snapshot and runs native Clang, then
native LLD, as preemptible user-mode processes. Successful output is stored as a
new `application/x-elf` object under `/home/default/program-<id>.elf`. Opening that
object in File Navigator starts a separate hardware-isolated application process.
Compiler errors are reported in the existing attached panel. Existing objects are
never overwritten by this publication path.

## Shared implementation

- `native_compiler_task`: compiler address space, launch record, checked file and
  stream adapters, one bounded process visit per desktop poll.
- `native_compile_job`: immutable source, compile/link transitions, finite memory
  and time limits, cancellation, actual binary/diagnostic results.
- `native_job_files`: exact read-only SDK/input grants; no ambient filesystem.
- `native_app_task`: isolated executable mapping and preemptible execution.
- `console_compile`: authenticated owner/publication lease, native object
  publication and File Navigator execution integration.
- `sdk/c/isolated.c`: user-local startup/API table and syscall-based output.

ARM64 and x86-64 share this logic. CPU adapters map a small supervisor-only trap
trampoline, not the desktop kernel's multi-gigabyte data/BSS into each process.
Compiler, linker and startup object are embedded in both live and installed
kernels. Release packaging removes non-runtime compiler symbols after verifying
the complete link; the production image loader verifies the packaged binaries.

## Evidence and boundaries

`builds/validation/native-apps/{aarch64-gic3-compiler,x86_64-compiler}/pipeline.json`
records actual guest Clang compilation, LLD linking, isolated execution returning
54, rejection of malformed C, and frame reclamation. The source uses the native
`stdio.h` and `puts`. The host does not compile that test source. These are hardware
guest probes, **not installed desktop acceptance**.

`tools/native-editor-installed-test.py` exercises installation, cold boot without
media, real attached-panel input, valid/invalid C, executable publication and
execution through File Navigator. It also requests actual local-model C generation
and uses the normal Apply control before compiling that proposal. A passing run is required before
claiming installed acceptance; merely adding the test is not evidence of passing.

Run each installed check through the build authority after building the release:

```sh
./build-kit run python3 tools/native-editor-installed-test.py --arch x86_64 build/native-editor-installed-x86
```

These create disposable disks, extract kernel metadata from the tested ISO itself,
install without patching the disk, then boot with installer media detached. They
do not use or modify the personal VirtualBox machine. Evidence is retained under
`builds/validation/native-apps/<architecture>-installed/`.

ARM QEMU verification requires explicit `--iso` test media linked for QEMU's RAM
layout. The canonical ARM installer targets VirtualBox and cannot be silently
substituted into this harness. Its evidence records the source media and whether
it is a canonical release; a QEMU derivative does not prove the VirtualBox ISO.

`tools/native-compiler-parity-test.py` compares actual toolchain bytes in both live
and installed artifacts. It does not substitute for installed execution.

## Current limits

- One compiler job and one console application at a time.
- Source 16 KiB; object/executable 256 KiB; diagnostics 8 KiB; application output
  4 KiB. Compiler deadline 120 seconds; application deadline 60 seconds.
- Small freestanding C SDK, not a complete hosted C standard library.
- The new isolated launcher currently grants console output only. Native GUI
  window/event/surface syscalls and generated-app catalog icons are not connected
  to this launcher; full graphical app generation is not accepted by this work.
- No networking, arbitrary object access, process spawning or kernel callbacks
  are granted to generated programs.
- Installed model-generated source, installed responsiveness, and both-target
  cold-install execution must be reported independently of probe/build success.
