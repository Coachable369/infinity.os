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

```sh
./build-kit run sh tools/native-editor-arm-media.sh
./build-kit run python3 tools/native-editor-installed-test.py --arch aarch64 --iso build/test-media/native-editor-aarch64.bootmedia build/native-editor-installed-arm
```

This derivative keeps the release's packaged model/asset ESP and relinks the
shared kernel at QEMU's load address. It never replaces the published ARM ISO.

`tools/native-compiler-parity-test.py` compares actual toolchain bytes in both live
and installed artifacts. It does not substitute for installed execution.

## Current limits

### Installed verification status — 2026-10-12

Not accepted end-to-end. Six unsuccessful installed verification/correction
attempts have reached this pass's repository limit; no speculative kernel fix
was applied to bypass the remaining failures.

- Full release build passed: manifest
  `builds/manifests/20261012T215524743829Z-16081.json`. Both canonical ISOs and
  `builds/SHA256SUMS` were regenerated. Build success is not installed acceptance.
- ARM QEMU derivative installed and retained its authenticated identity after a
  detached cold boot. It is not proof of the canonical VirtualBox ARM installer.
- Installed TCG run compiled valid C through the attached panel, linked a real
  8,648-byte ELF, and published its ObjectId. Malformed C produced 148 bytes of
  diagnostics and no replacement executable. Structured evidence:
  `builds/validation/native-apps/aarch64-installed/tcg-editor-evidence.json`.
- Pointer latency during these compilations was 17.595 seconds and 3.145 seconds
  respectively under software emulation. The first exceeds the five-second
  acceptance bound. Compiler image loading/preparation remains synchronous;
  this is a candidate to profile, not a proven attribution of the whole delay.
- Actual Hermes generation did not produce a reviewed compile proposal within
  600 seconds. The session subsequently auto-locked; no generated-source or
  File Navigator execution acceptance is claimed.
- An earlier accelerated ARM run aborted inside QEMU HVF at its `isv` assertion.
  A final instrumented run failed to open the launcher and its diagnostic
  connection timed out, before reaching compiler execution. Broad MMIO tracing
  was removed because it perturbs timing and generates excessive diagnostics.
- x86 native compiler/linker/isolation probes pass, but the complete installed
  desktop workflow on x86 has not been verified.

Next required proof: bound compiler initialization work, diagnose the ARM HVF
transition with minimally intrusive diagnostics (or the canonical VirtualBox
test VM), obtain actual reviewed model-generated source, compile it, and open
the saved ELF through File Navigator with exit status checked. Repeat on x86.
Any production fix requires a new full release build and renewed installed proof.

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
