# Native Python port (in progress)

This directory targets CPython 3.13.16 as an isolated InfinityOS user process.
The host Python executable is a build-time bytecode freezer only; it is not a
runtime service. The same port is compiled for aarch64 and x86_64.

## Acceptance gate

Execute an actual `.py` object through the InfinityOS command line, capture its
standard output and exit status, import an InfinityOS-targeted native C extension,
and reclaim the process resources. Verify the
installed path without installer media. A successful cross-compile alone does
not satisfy this gate.

## Initial boundary

- Script input is an explicitly granted immutable Object Store snapshot.
- CPython runs in private, timer-preempted user memory using the existing native
  runtime ABI and Newlib adapters.
- Core Python and selected frozen standard-library modules are included.
- No host filesystem, host Python, shell transport, or ambient file authority.
- Native extension loading is required. It must resolve CPython symbols and
  enforce checked relocations and non-WX mappings; the existing compiler
  `dlopen` rejection adapter is not an implementation of this requirement.
- The initial loader accepts native ELF64 C extensions with standard
  `PyInit_<module>` entry points and checked relative, data, and function-pointer
  relocations. It retains handles until process exit. TLS-dependent extensions,
  constructors, and dependent shared libraries are explicitly rejected for now;
  this is not compatibility with arbitrary third-party wheels.
- Additional OS threads, subprocesses, package installation, and arbitrary file
  writes are outside this first milestone. Package installation (`pip`) is the
  medium-term goal; preserve CPython's standard extension ABI for that work.
- Editor terminal UI is a separate follow-up, not a prerequisite for script
  execution.

## Development commands

All commands require the repository build kit and its exclusive build slot:

```sh
./build-kit run python3 tools/native-python/build.py aarch64
./build-kit run python3 tools/native-python/build.py x86_64
./build-kit run python3 tools/native-python/verify-loader.py
./build-kit run python3 tools/native-process-probe/run.py --arch aarch64 --compiler builds/toolchains/aarch64/python.elf --python-script
./build-kit run python3 tools/native-process-probe/run.py --arch aarch64 --compiler builds/toolchains/aarch64/python.elf --python-script --python-extension builds/toolchains/aarch64/infinity_native_test.so
```

The loader check exercises mapping and cleanup without guest execution. The
last command is a hardware-isolated guest test, not installed acceptance.
It checks a calculated exit status and memory reclamation, not diagnostic prose.
Until that gate and command-line integration pass, this port is not released.

## Console integration

The native console accepts `python /home/default/documents/hello.py` and the
`python3` alias. Fresh installations seed this editable example and
`native_demo.py`, which imports the installed `infinity_native_test.so` module.
`python-cancel` cancels the authenticated session's running script.

The launcher snapshots up to eight readable script-directory `.py`/`.so` objects,
at most 1 MiB each, and uses the isolated runtime's 64 MiB heap and 1 MiB stack.
Execution is timer-preemptible, has a two-minute deadline, and is cancelled on
session loss. Output is captured up to 32 KiB stdout and 8 KiB stderr. This first
command interface splits arguments on whitespace and requires absolute paths;
interactive stdin, quoted arguments, recursive packages, and pip are not yet
implemented. Hash randomization derives from validated boot entropy.

Ownership compatibility: authenticated users may read unowned Personal objects
under the existing shared `/home/default/documents/` namespace, including the
installer examples. Explicitly foreign-owned objects and unowned System objects
are denied. This compatibility contract remains active until shared Documents
are migrated to per-user ownership; Python does not silently claim or mutate them.

Newlib binary-mode opens retain Object Store byte semantics. Unsupported
signal waits, CPU-accounting queries, clock-resolution queries, and directory
mutation report errors rather than simulating facilities. `time.sleep` yields
the process until its deadline without blocking the OS input loop.

## AI source workflows

`Create Hello world in Python` uses the existing AIKit/IntelligenceKit path to
stream Python into the main editor, save an owned `.py` object, validate it with
native CPython, and run the authorized script. It does not translate Python into
C or invoke a host interpreter. Syntax failures can enter bounded repair;
interpreter initialization failures are reported as infrastructure failures.

Existing-app updates resolve one owned C or Python source in Documents, or an
explicit source path. The current editor must be saved and match the selected
immutable storage version. Ambiguous names, foreign ownership, changed versions,
or unsaved work stop the update without replacing the user's document.
Generation and automatic compiler repairs retain the same object identity.
An update saves and validates; only an explicit launch request starts the updated
program. Generated source still requires native compilation/execution evidence;
successful compilation alone does not verify application behavior.

## Path to package installation

Keep CPython's extension ABI and import protocol, rather than introducing a
Python-like interpreter or a bespoke extension API. A subsequent package phase
must provide persistent per-user package objects, import/search-path authority,
wheel platform tags for InfinityOS, TLS/HTTPS and certificate validation,
archive extraction, dependency metadata, and failure-atomic installation and
removal. Build backends need native compiler access and declared library
dependencies. None of these are implied by a successful extension import.

Source: https://www.python.org/ftp/python/3.13.16/Python-3.13.16.tar.xz

Archive SHA-256:
`f4b1bfb3c79b5bb11b8d228a12504163b4c0dab4d679828d8f5f26b6cb6ab35d`

CPython is distributed under the Python Software Foundation license. Its
upstream `LICENSE` and bundled third-party notices must accompany release
artifacts. Downloaded sources stay outside Git; the builder verifies the pinned
archive before extraction.
