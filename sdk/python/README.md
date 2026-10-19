# Native Python and bundled pip

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
- Additional OS threads, subprocesses, and arbitrary ambient file writes are
  unavailable. Bundled pip uses a separate, bounded per-user package transaction;
  it does not acquire host filesystem or network authority.
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
interactive stdin and quoted arguments are not implemented. Owned installed
packages are additionally mounted for imports. Hash randomization and Python's
entropy requests derive from the validated native launch entropy service.

Ownership compatibility: authenticated users may read unowned Personal objects
under the existing shared `/home/default/documents/` namespace, including the
installer examples. Explicitly foreign-owned objects and unowned System objects
are denied. This compatibility contract remains active until shared Documents
are migrated to per-user ownership; Python does not silently claim or mutate them.

Newlib binary-mode opens retain Object Store byte semantics. Unsupported
signal waits, CPU-accounting queries, and clock-resolution queries report errors
rather than simulating facilities. Directory mutation is confined to granted
mutable mounts, including private guest scratch; read-only mounts reject it.
`time.sleep` yields the process until its deadline without blocking the OS input loop.

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

## Offline wheel packages

The native executable includes upstream pip 26.2.1 with narrow native platform
adaptations, its vendored dependencies, and the required frozen standard library.
The adaptations are documented below; this is not an alternate installer that
only imitates pip's command names.

Public commands:

```text
pip --version
pip install /home/default/downloads/package-1.0-py3-none-any.whl
pip install --upgrade /home/default/downloads/package-2.0-py3-none-any.whl
pip list --format=json
pip show package
pip check
pip freeze --all
pip uninstall -y package
python -m package
```

`pip3` and `python -m pip` use the same authenticated package service. Local
dependency wheels alongside the requested wheel are made available to pip's
real resolver. The service enforces offline, binary-only installation, disables
pip's self-update check and threaded progress display, and does not emit `.pyc`
files. Supported install switches are `--upgrade`/`-U`, `--force-reinstall`, and
`--no-deps`. Listing supports `json`, `columns`, and `freeze` formats. Uninstall
requires the explicit `-y` flag. Paths and arguments currently cannot contain
shell quoting or whitespace.

Only interpreter-compatible pure-Python `*-none-any` wheels are admitted in this
release. Index/PyPI access, source distributions/PEP 517 builds, virtual environments, arbitrary
third-party native wheels, pip self-modification, and automatic registration of
wheel console scripts as OS commands are not supported. Use `python -m module`
for a package that supplies a module entry point. Supporting the CPython native
extension ABI does not imply third-party platform-wheel compatibility.

Package contents persist in the initiating user's native Object Store bundle.
The process sees `/packages` for installed packages, read-only `/work` for the
explicit wheel inputs, and `/tmp` for **guest virtual scratch only**. This `/tmp`
is neither host storage nor persistent OS storage. Writes remain staged until
successful native completion and authorized version-fenced publication;
failure, cancellation, quota exhaustion, or loss of session authority must not
publish a partial environment. Other users' bundles are not mounted.

Bounds are 4 MiB per package file, 8 MiB installed bundle, 512 total VFS entries
(including four mount roots, so at most 508 package entries), 255-byte paths,
a 64 MiB process heap, and a two-minute operation deadline. Quota failures are
errors, not permission to write outside these mounts.

### Platform adapters and provenance

- The standard `importlib.resources` reader exposes exact pinned pip resource
  bytes, including certifi data; genuine pip distribution metadata is discoverable
  through `importlib.metadata` without making the system pip writable.
- Native `sysconfig` reports `infinityos_aarch64` or `infinityos_x86_64` and routes
  installation schemes to the owned `/packages` mount. It never advertises Linux
  or host-macOS compatibility.
- pip's supported candidate tags are restricted to `none` ABI and `any` platform,
  including resolver support-index checks; the interpreter's own ABI is unchanged.
- No POSIX root UID is fabricated. Vendored platformdirs treats an absent Unix
  UID API as a per-user configuration, with private guest runtime scratch.
- Socket creation, network operations, descriptor polling, file-backed memory
  mapping, and subprocess execution reject unsupported use. They do not return
  simulated success. pip's offline path does not require HTTP cache mappings.
- CPython's native zlib, hashing, and bundled Expat modules implement real wheel
  decompression, CRC checks, package hashing, and XML parsing. zlib 1.3.2 is sourced from the existing pinned
  `libz-sys 1.1.29` crate, SHA-256
  `85bc9657773828b90eeb625adff10eeac83cc21bbfd8e23a03eaa8a33c9e28d9`.
- pip, vendored dependency, zlib, Expat, CPython, and compiler-runtime notices accompany
  the installed Python license object.

The acceptance gate is native dependency resolution, install, import in a
separate process, metadata inspection, uninstall, rollback on failure, and
persistence after an installed-system reboot. A printed pip version or successful
cross-compile alone does not satisfy this gate.

Source: https://www.python.org/ftp/python/3.13.16/Python-3.13.16.tar.xz

Archive SHA-256:
`f4b1bfb3c79b5bb11b8d228a12504163b4c0dab4d679828d8f5f26b6cb6ab35d`

CPython is distributed under the Python Software Foundation license. Its
upstream `LICENSE` and bundled third-party notices must accompany release
artifacts. Downloaded sources stay outside Git; the builder verifies the pinned
archive before extraction.
