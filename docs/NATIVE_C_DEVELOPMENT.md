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
- `sdk/c/runtime.c` implements ABI negotiation, `puts`, `putchar`, and returning
  `main`'s status. It is not a complete C standard library.
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
