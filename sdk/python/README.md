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
- Additional OS threads, general subprocesses, and arbitrary ambient file writes
  are unavailable. Bundled pip has a narrowly scoped native child-Python adapter
  for PEP 517 hooks inside its bounded per-user package transaction.
  Explicit online installs acquire the existing profile-governed native web
  capability leases; ordinary Python scripts do not inherit that authority.

## Development commands

All commands require the repository build kit and its exclusive build slot:

```sh
./build-kit run python3 tools/native-python/build.py aarch64
./build-kit run python3 tools/native-python/build.py x86_64
./build-kit run python3 tools/native-python/verify-loader.py
./build-kit run python3 tools/native-python/socket-test.py
./build-kit run python3 tools/native-python/network-test.py
./build-kit run python3 tools/native-python/process-test.py
./build-kit run python3 tools/native-process-probe/run.py --arch aarch64 --compiler builds/toolchains/aarch64/python.elf --python-script
./build-kit run python3 tools/native-process-probe/run.py --arch aarch64 --compiler builds/toolchains/aarch64/python.elf --python-script --python-extension builds/toolchains/aarch64/infinity_native_test.so
```

The loader check exercises mapping and cleanup without guest execution. The
last command is a hardware-isolated guest test, not installed acceptance.
It checks a calculated exit status and memory reclamation, not diagnostic prose.
Until that gate and command-line integration pass, this port is not released.

The pip lifecycle probe executes the real native interpreter and bundled pip,
including dependency resolution, compressed wheels, metadata, rejected wheels,
upgrade, archive remount, native extension install/import, malformed extension
rollback, and uninstall:

```sh
./build-kit run python3 tools/native-process-probe/run.py --arch aarch64 --compiler builds/toolchains/aarch64/python.elf --python-pip
./build-kit run python3 tools/native-process-probe/run.py --arch x86_64 --compiler builds/toolchains/x86_64/python.elf --python-pip
```

That isolated-process probe does not prove the installed console or an OS reboot.
After `./build-kit full`, run the following separately in the exclusive build
slot for fresh installation, authenticated console commands, cold-boot package
imports, upgrade, failure rollback, and uninstall:

```sh
./build-kit run python3 tools/native-python/pip-installed.py --arch aarch64 --iso build/test-media/InfinityOS-aarch64-qemu.bootmedia
./build-kit run python3 tools/native-python/pip-installed.py --arch x86_64 --iso builds/InfinityOS-x86_64.iso
./build-kit run python3 tools/native-python/pip-installed.py --arch aarch64 --iso build/test-media/InfinityOS-aarch64-qemu.bootmedia --online
./build-kit run python3 tools/native-python/pip-installed.py --arch aarch64 --iso build/test-media/InfinityOS-aarch64-qemu.bootmedia --online --source
```

The ARM command intentionally uses QEMU-linked test media: its live and installed
kernels start at `0x50000000`, inside QEMU virt RAM. The published
`builds/InfinityOS-aarch64.iso` starts at `0x10000000` for VirtualBox and must not
be passed to this QEMU runner. The harness validates actual ELF load ranges,
not filenames, before starting a VM. For ARM full-model-bundle testing, create
`./build-kit package-aarch64-qemu` and substitute
`build/test-media/InfinityOS-aarch64-qemu-models.bootmedia`. The focused Python/pip
test does not need the model bundle. Both commands install unchanged media on a
new disposable disk; they never replace an installed kernel offline. A separate
authenticated second-user session test is not claimed by this harness.

## Native Tcl/Tk architecture and GUI acceptance

`_tkinter` is CPython's upstream module, linked to pinned upstream Tcl 8.6.18
and Tk 8.6.18. The native port keeps their interpreters, widgets, bindings,
geometry managers, ttk themes, image handling and text engine. It is not a
Python widget imitation and it does not contact X11, macOS or a host GUI service.
Exact Tcl/Tk library resources are an immutable native filesystem inside the
interpreter payload; no extraction to host temporary storage is involved.

The shared architecture adapter translates Tk's platform drawing interface into
private bounded raster surfaces, with installed FontKit glyph coverage and real
native Window Server windows. Each window obtains context-scoped management,
surface and IEF-input grants. Event subscriptions have finite renewed leases;
application exit releases windows, surfaces, subscriptions and capabilities.
The desktop supplies its existing window chrome outside Tk's client pixels.
Close requests enter `WM_DELETE_WINDOW` before destruction, allowing normal
application confirmation or veto. Withdrawn windows are distinct from minimized
windows. Tcl timers yield the native process instead of blocking OS input.

Current platform limits are explicit: installed native font coverage is ASCII
with a visible replacement glyph for unsupported characters; available font
families are Inter and JetBrains Mono. Foreign-window embedding, cross-process
Tk `send`, external clipboard transfer, pointer warping, host screensaver control,
audio bell and arbitrary custom cursors are not implemented. A successful
Tcl/Tk import does not imply that those facilities exist. The port does not
introduce Python worker threads or a host display-server dependency.

Native import/interpreter assertions belong to `stdlib-acceptance.py`; actual
window, input and pixel acceptance is a separate installed-OS gate:

```sh
./build-kit run python3 tools/native-python/tk-installed.py --arch aarch64 --iso build/test-media/InfinityOS-aarch64-qemu.bootmedia
./build-kit run python3 tools/native-python/tk-installed.py --arch x86_64 --iso builds/InfinityOS-x86_64.iso
```

The installed harness uses real pointer and keyboard events, rendered Canvas and
PhotoImage pixels, numeric window geometry and process-exit/resource receipts.
It exercises ttk controls, Entry editing, Text undo, grid/pack layout, timers,
move/resize, close veto then close, repeated multi-window allocation, withdrawal,
and the standard `python -m tkinter` demonstration. It seeds only four test
source files on a stopped disposable disk; it never writes guest memory,
replaces installed kernels or fabricates widget responses. `--reuse-work` may
reuse a stopped test installation only with matching media and an unchanged
installed kernel. Its report distinguishes reuse from a fresh installation.
Passing cross-compilation, import tests and installed GUI acceptance are separate
claims; consult the corresponding build manifests and `installed.json` receipts.

## Console integration

The native console accepts `python /home/default/documents/hello.py` and the
`python3` alias. Fresh installations seed this editable example and
`native_demo.py`, which imports the installed `infinity_native_test.so` module.
`python-cancel` cancels the authenticated session's running script.
`python --allow-network /absolute/script.py` explicitly requests the current
session's profile-governed network capabilities for that invocation. The editor's
docked console accepts the same command. Ordinary Run and generated apps do not
silently inherit it. Session loss, revoked leases and profile changes stop I/O.

The launcher snapshots up to eight readable script-directory `.py`/`.so` objects,
at most 1 MiB each, and uses the isolated runtime's 64 MiB heap and 1 MiB stack.
Execution is timer-preemptible and cancelled on session loss. Ordinary interactive
scripts have no artificial wall-clock expiry; `-c` validation is bounded to two
minutes and pip operations to ten minutes. Output is captured up to 32 KiB stdout
and 8 KiB stderr. The command interface splits arguments on whitespace and
requires absolute paths; quoted arguments are not implemented. Owned installed
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

### Application data and standard-library limits

Ordinary scripts receive read-only document and installed-package snapshots.
Their writable guest scratch namespace is process-private and is discarded at
exit. Direct execution of a saved source additionally grants `/data` and makes
it the process working directory. This mount belongs to the authenticated
profile and stable source ObjectId, so another source cannot read it, and source
renames or editor updates do not discard it. It does not grant write access to
Documents, source code or installed packages. `-c` validation, `-m` commands,
pip and its build children do not receive this application-data grant.

Application data uses the existing bounded native VFS and ObjectStore
copy-on-write transaction, with its own `IDAT0001` archive rather than the
package archive. Successful `fsync`, file close and namespace operations are
acknowledged only after the durable checkpoint commits. Clean script exit also
flushes remaining data. Disk-full, stale-revision, session-loss or storage errors
preserve the previous checkpoint and return an error; further data mutations
are disabled after a failed checkpoint. Scratch is never archived. Package
installation retains its separate successful-pip transaction rule. The shared
64 MiB staging, 32 MiB archive, 16 MiB per-file and entry quotas still apply.

The genuine SQLite backend supports SQL, transactions, constraints, callbacks,
FTS5, serialization and private namespace files. Its native tests currently
verify in-memory databases and serialization; they do not establish durable
application databases across process exit or reboot. SQLite's VFS does not
provide cross-process shared-memory WAL. The source-scoped data adapter now
connects SQLite sync and journal-deletion barriers to actual native checkpoints;
its separate installed acceptance gate must pass before durable database support
is claimed:

```sh
./build-kit run python3 tools/native-python/tk-installed.py --arch aarch64 --iso build/test-media/InfinityOS-aarch64-qemu-models.bootmedia --durable-data --control-selection
```

Use refreshed media containing both the data adapter and normalized native
shortcut routing. This gate verifies SQL integrity and values after detached
cold boots, committed abnormal exit and hot-journal rollback, plus source
isolation and read-only input/package enforcement. Earlier media or a passing
import probe cannot satisfy it.

Module inclusion is not functionality. `_thread` is present for CPython's serial
runtime, but creating worker threads remains unsupported; general
`threading`, `multiprocessing` and executor workloads are not supported.
`ctypes` now has genuine outbound libffi calls, but executable callbacks remain
unsupported. `mmap` supports private anonymous buffers; file-backed mappings
remain unsupported. Terminal/POSIX-specific modules such as `curses`, `readline`, `termios`,
`fcntl`, `pwd`, `grp` and `resource` have no general native implementation.
IDLE and virtual environments are not included. `decimal` uses CPython's genuine
pure-Python implementation, not an unavailable-module shim.

Use the pinned CPython 3.13 source and
[official library reference](https://docs.python.org/3.13/library/) for the
platform module contract. Some third-party module lists still include
[removed modules](https://docs.python.org/3.13/library/removed.html), such as
`aifc`, `cgi` and `cgitb`; those are not part of Python 3.13. Windows-only APIs
are not presented as portable InfinityOS features.

## AI source workflows

Generation retains the model's typed completion reason. Context/output exhaustion
does not approve partial code: the same owner-bound editor transaction can request
up to three continuation-only suffixes, retaining the complete source prefix,
document revision and original total deadline. Continuations never append a
replayed program prefix and STOP clears their delivery state. Explicit source
frames require a closing marker; recognizable raw source retains normal model-EOS
admission to native syntax/dependency validation, not automatic proof of success.
Source is currently limited to 12 KiB of ASCII (Unicode string escapes are allowed).
Capacity, unsupported characters and exhausted recovery produce visible errors
and release inference instead of silently stopping the stream. Large programs
still need smaller modules or suitable installed libraries.

Generated applications additionally use a conservative incomplete-source lint:
an entire application consisting only of placeholder bodies and entrypoint
wiring is rejected with typed validation exit 67 and can enter the existing
two-attempt, revision-checked repair flow. This includes standard `abc`/local
inheritance scaffolds when the guarded local entrypoint is itself empty.
Individual no-op callbacks, external/decorator-provided implementations,
standalone abstract declarations, and working top-level scripts are not rejected.
Ordinary `validate_source` and manual script execution are unchanged.
The lint neither executes code nor proves gameplay or application functionality.

`tools/application-generation-probe.rs` exercises real Hermes, paced editor
delivery and native-inventory Python syntax/import validation without executing
generated source on the host. Its receipts explicitly distinguish generation,
validation, model-lease reuse, installed execution and application/game behavior.

`Create Hello world in Python` uses the existing AIKit/IntelligenceKit path to
stream Python into the main editor, save an owned `.py` object, validate it with
native CPython, and run the authorized script. It does not translate Python into
C or invoke a host interpreter. Syntax failures can enter bounded repair;
interpreter initialization failures are reported as infrastructure failures.

Existing-app updates resolve one owned C or Python source in Documents, or an
explicit source path. When the requested source is already open, its exact saved
checkpoint must match the latest immutable version. The model then receives the
live buffer, including unsaved edits. An update is applied atomically with one
undo record only if that buffer's revision is still unchanged. Ambiguous names,
foreign ownership, stale persisted versions, concurrent edits, and a different
dirty document stop the update without replacing the user's work.
Generation and automatic compiler repairs retain the same object identity.
An update saves and validates; only an explicit launch request starts the updated
program. Generated source still requires native compilation/execution evidence;
successful compilation alone does not verify application behavior.

## Native wheel packages and authenticated HTTPS

The native executable includes upstream pip 26.2.1 with narrow native platform
adaptations, its vendored dependencies, and the required frozen standard library.
The adaptations are documented below; this is not an alternate installer that
only imitates pip's command names.

Public commands:

```text
pip --version
pip install six==1.17.0
pip install python-dateutil==2.9.0.post0
pip install https://example.org/package-1.0-py3-none-any.whl
pip install /home/default/downloads/package-1.0-py3-none-any.whl
pip install /home/default/downloads/package-1.0.tar.gz
pip install --upgrade /home/default/downloads/package-2.0-py3-none-any.whl
pip list --format=json
pip show package
pip check
pip freeze --all
pip uninstall -y package
python -m package
```

`pip3` and `python -m pip` use the same authenticated package service. Local
dependency wheels and source archives alongside the requested input are made available to pip's
real resolver. Local-only requests stay offline; named requirements and HTTPS
archive URLs use authenticated native transport and the PyPI index. The service
prefers compatible wheels, disables
pip's self-update check and threaded progress display, and does not emit `.pyc`
files. Supported install switches are `--upgrade`/`-U`, `--force-reinstall`, and
`--no-deps`. Listing supports `json`, `columns`, and `freeze` formats. Uninstall
requires the explicit `-y` flag. Paths and arguments currently cannot contain
shell quoting or whitespace.

Upstream pip selects interpreter-compatible pure-Python wheels and matching
`cp313` InfinityOS native wheels. A native wheel is not accepted merely because
its filename matches: every staged `.so` is validated through the real ELF
loader against the actual interpreter exports before the transaction commits.
Foreign ABI/platform tags, unsupported relocations, shared-library dependencies,
TLS-dependent extensions and constructors fail without replacing installed state.
Admitted extension bytes are copied into stable read-only per-process snapshots;
later package scratch mutations cannot alter a live library grant.

The source-build adapter uses upstream pip's `VirtualBuildEnvironment` and
subprocess build-dependency installer, with both dependency installation and
each actual PEP 517 backend hook running in a separate native CPython `Task`.
Only one child is admitted at a
time; it cannot recursively spawn another child, launch a shell, or execute
external build tools. Build dependencies go in private guest `/tmp` prefixes,
not the committed user environment. Parent descriptors and cwd are parked;
children receive distinct descriptor epochs and no inherited open handles.
Cancellation, revocation and deadlines reclaim the child before the parent
can resume or publish. Native hooks requiring external tools or native build
dependency extensions outside the existing immutable package snapshots remain
unavailable. That is a current build-platform gap, not an assertion that every
such package is inherently incompatible with InfinityOS.

General virtual environments, OS-managed pip/tzdata self-modification, and
automatic registration of wheel console scripts as OS commands are not
supported. Use `python -m module` for a package that supplies a module entry point.
No Linux/macOS wheel compatibility is advertised.

Package contents persist in the initiating user's native Object Store bundle.
The process sees `/packages` for installed packages, read-only `/work` for the
explicit wheel inputs, and `/tmp` for **guest virtual scratch only**. This `/tmp`
is neither host storage nor persistent OS storage. Writes remain staged until
successful native completion and authorized version-fenced publication;
failure, cancellation, quota exhaustion, or loss of session authority must not
publish a partial environment. Other users' bundles are not mounted.

Bounds are 16 MiB per package file, 32 MiB installed bundle, 64 MiB guest staging,
2048 total VFS entries (including four mount roots), 255-byte paths, a 64 MiB
process heap, and a ten-minute pip operation deadline. The live native-extension
snapshot budget is eight modules totaling 16 MiB. Native package HTTPS limits
each decoded response to 16 MiB; browser requests retain their 4 MiB budget.
These are capacity limits, not package
incompatibility claims, and exceeding them cannot grant writes outside the mounts.
Storage and guest archive limits share `native_python_package_limits.rs` and
retain the existing versioned, atomic native extent format.

### Platform adapters and provenance

- The standard `importlib.resources` reader exposes exact pinned pip resource
  bytes, including certifi data; genuine pip distribution metadata is discoverable
  through `importlib.metadata` without making the system pip writable.
- Native `sysconfig` reports `infinityos_aarch64` or `infinityos_x86_64` and routes
  installation schemes to the owned `/packages` mount. It never advertises Linux
  or host-macOS compatibility.
- pip retains upstream interpreter/ABI/platform tag selection; native packages
  additionally pass trusted ELF validation before publication.
- No POSIX root UID is fabricated. Vendored platformdirs treats an absent Unix
  UID API as a per-user configuration, with private guest runtime scratch.
- CPython's unchanged `socket.py` and `ssl.py` wrap native `_socket`/`_ssl`
  platform adapters. `_infinity_socket` delegates real DNS, persistent TCP reads
  and writes, and TLS upgrades to the existing BSP network actor. Partial writes,
  short reads, socket file buffering, EOF, cancellation and timeouts retain actual
  transport semantics. This is a separate byte-stream interface, not HTTP
  responses disguised as SSL sockets. The backend identifies itself truthfully
  as embedded-tls, not OpenSSL.
- The current socket boundary is one active IPv4 TCP client connection on the
  shared actor, TLS 1.3 with AES-128-GCM, and mandatory system-root chain,
  hostname and validity checks. UDP, IPv6, listening sockets, native descriptor
  polling, TLS server mode, client certificates, arbitrary custom trust stores, negotiated ALPN,
  memory BIOs and TLS versions below 1.3 are not implemented. Unsupported
  operations raise errors. Certificate inspection and SSL pending-byte APIs are
  also not yet exposed. Do not treat this as arbitrary CPython SSL compatibility.
- Upstream `http.client` only skips its ALPN setter when the backend explicitly
  advertises `HAS_ALPN = False`; HTTP/1.1 still uses real verified TLS.
- The exact certifi 2026.06.17 PEM selects a per-context **intersection** with
  native roots, never a union or a pathname-only alias. The native root set is
  fingerprinted over complete subjects, public keys and name constraints;
  changed root versions fail closed until the intersection is reverified.
  Unknown/mutated bundles remain unsupported. Default contexts retain system
  trust. This path still requires installed `requests` acceptance on the next
  payload; it is not included in the earlier baseline proof.
- File-backed memory mapping and general subprocess execution remain unsupported.
  A request to delegate hostname checking to the caller does not disable the
  stronger native SAN policy; `check_hostname` remains true. HTTP/1.1-only ALPN
  configuration is admitted as protocol fallback, with no ALPN extension sent
  and `selected_alpn_protocol()` returning `None`. HTTP/2/mixed protocol lists
  remain unsupported. Arbitrary CA files remain rejected; bundled pip uses its
  explicit native HTTPS adapter.
- `_infinity_process` exposes only pip-owned start/poll/read/cancel/release.
  It starts the real installed CPython image through the existing isolated
  runtime, never host Python or in-process backend evaluation. Child output is
  captured up to 64 KiB per stream and published after completion; combined
  streams preserve each stream's bytes but not cross-stream timing. Each hook
  has at most two minutes within the parent's ten-minute total deadline.
  Because frozen pip has no adjacent filesystem `__pip-runner__.py`, its existing
  build-dependency installer receives a small real entry file in private guest
  scratch. That file configures native transport and invokes frozen upstream pip
  in the same isolated child path. It is reused during the transaction, removed
  on normal exit, and reclaimed with private scratch after cancellation.
- `_infinity_https` exposes only bounded GET/poll/read/close through the existing
  BSP HTTPS actor. TLS 1.3 chain, hostname, trusted-time and handshake verification
  use the installed system roots. Native code rejects HTTP, non-443 ports,
  revoked capabilities and unauthenticated sessions. The real upstream resolver,
  redirects, wheel hashes and installer remain in pip; certificate verification
  cannot be disabled with `verify=False`, trusted-host flags, proxies or custom
  certificates. Ordinary scripts cannot reuse pip's web authorization.
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
The additional online gate downloads pinned public packages in a freshly
installed guest, verifies imports and dependency metadata, cold-boots without
installation media, checks a failed transaction preserved the archive and
uninstalls the packages. Host-side adapter fixtures are not evidence of native
TLS, installed PyPI access or complete Python SSL support.
It also runs the actual standard-library `socket`, `ssl`, `HTTPResponse` and
`urllib.request` APIs against multi-record HTTPS responses, rejects the wrong
TLS hostname and verifies denial without `--allow-network`. The ARM installed
baseline passed these checks plus actual `six`, `python-dateutil`, and
`packaging` downloads, installation, function calls, metadata, detached cold-boot
imports, failed-install exact archive rollback, and uninstall in manifest
`20261020T003453766324Z-26516`. Its receipt is
`builds/validation/native-python/aarch64/pip-installed-1792456506977077000/installed.json`.
This is not x86 or general SSL compatibility evidence. Overall acceptance remains
false: the independent SQLite private-file gate and first PEP 517 source build
failed on that baseline. Subsequent adapter changes require new native proof.
The source failure was traced to a 4096-byte regular-file write truncating the
materialized upstream build hook. The shared blocking file adapter now completes
bounded writes; the installed gate also checks a single 25,600-byte `os.write`
roundtrip and exact resource materialization before attempting a build.

`--source` adds a source-only backend fixture with an observed child process ID,
failed-backend archive rollback, cold-boot import and uninstall. It then downloads
the SHA-256-pinned public `packaging 25.0` source archive, installs its declared
`flit_core>=3.3` build dependency privately, builds it with the native child,
resolves `six` in the same pip command, and checks that build dependencies did
not leak into the committed archive. These remain acceptance gates, not claims
of passing native source-build support until the resulting receipts are verified.

### Native foreign functions

The native build includes upstream `_ctypes` with checksum-pinned libffi 3.4.8
(`bc9842a18898bfacb0ed1252c4febcc7e78fa139fd27fdc7a3e30d9d9356119b`).
Its shared ARM64/x86-64 implementation uses the existing process-private,
capability-scoped native library loader, not a host library bridge. The intended
scope is ordinary ctypes types, layouts, private memory buffers and outbound
calls to explicitly available InfinityOS-native ELF libraries, including
structures and variadic calls. The actual isolated ARM guest passed typed,
mixed-register, structure-by-value, variadic, memory, pointer and rejection
behavior with exit 54 and all pages reclaimed in manifest
`20261020T012058479315Z-34009`. This is not installed-application or x86 evidence.

Executable Python callbacks raise `NotImplementedError` before allocation.
No writable-executable mapping, closure allocator, or executable-memory bypass
is enabled. `CDLL(None)`, foreign-platform libraries, absent/ungranted paths,
unsupported dependency/TLS/constructor ELF features and early unloading retain
the existing native loader's rejection behavior. `ctypes.pythonapi` is therefore
`None`. Raw pointer operations are process-private native operations and can
still fault their own process; they do not provide extra OS authority.

The behavioral fixture `tools/native-python/ctypes-acceptance.py` loads the
actual target `native_ffi_test.so` through the same loader and checks numeric,
structure, variadic, pointer and rejection results. It must run in each native
guest; host Python execution is not acceptance evidence.

Upstream `mmap` also passed its ARM anonymous-buffer behavioral gate in manifest
`20261020T012039970217Z-33976`, including zero initialization, slicing, read/write,
seeking, movement, flush, memoryviews, exported-buffer close protection,
read-only access, repeated release/reallocation and the 1 MiB committed limit.
Mappings remain process-private and non-executable. File-backed persistence,
cross-process sharing and resize are not provided; neither installed-runtime nor
x86 acceptance is implied by this ARM probe.

### Library discovery for AI generation

`python -m _infinity_modules --generation-catalog` returns versioned, structured
task-oriented default libraries, installed import roots and explicit capability
limits. Discovery reads metadata/files only; it does not import packages or run
their initializers. `package_apis_verified` is false: installation is not proof
that every dependency or API is compatible with native InfinityOS.

The editor's create, update and repair prompts use the same conservative
`generation-libraries.txt` guidance plus a maximum 512-byte directory from the
authenticated profile's committed package archive. Only sanitized import names
enter the prompt, never package descriptions, README instructions or source.
Prompt guidance is limited to two task-relevant library families; it is not an
exhaustive import allowlist. Native UI instructions are included only for an
explicit UI request or an existing native UI import. The complete inventory
remains discoverable through the catalog API. Repair source is preserved byte
for byte and separated from instructions even when its final line has no newline.
The archive is streamed and CRC-checked with bounded memory; a cache is reused
only for the same owner and immutable archive revision, after session authority
is checked again. Installation/uninstallation changes the revision and refreshes
the directory. No package installation, native process or extra inference turn
is started by discovery. Missing directory access falls back to verified defaults.
Behavioral fixtures are `module-tests.py`, `library-catalog-test.rs` and the
catalog checks in `package-storage-test.rs`; installed generation remains a
separate end-to-end acceptance gate.

Generated applications also use `validate_generated_application`: syntax and
dependency validation plus a conservative whole-placeholder lint. Exit status
67 enters the existing bounded repair path; ordinary user-script validation is
unchanged. Empty callbacks, delegated installed applications and executable
annotations must not be rejected as whole-app stubs. This lint does not prove
that game rules or other application behavior are correct.

Interrupted source generation preserves the exact accepted prefix for at most
three bounded suffix continuations, under the original owner, document revision
and deadline. Exhausted capacity or malformed/repeated output fails explicitly
and releases the inference lease. New source streams into the editor; repairs
and updates retain the existing document until a complete atomic replacement is
ready. Native validation probes snapshot custom source into a repository-local
read-only build input and bind their receipts to those exact bytes.

### Generated application runtime diagnostics

Generated execution uses `python -m _infinity_generated_run /granted/source.py`
after validation, not for ordinary CLI scripts or pip. The trusted entry retains
the application's `__main__` module through finalization, preserves script argv,
uses the same scoped import directory, and re-raises uncaught exceptions so the
normal traceback and exit behavior remain visible. Standard streams are not
redirected or consumed by the entry.

The native `_infinity_runtime` adapter reads the actual exception object and
traceback frame, then submits a 352-byte versioned record through `0x1290`.
The receiver must authorize the generated run and bind the record to its exact
source path, source line range and execution context. Only one report is
accepted for that job. Source exceptions, OS errors, resource failures and
other exceptions have distinct typed categories. `SystemExit`, cancellation,
`KeyboardInterrupt`, native faults and deadlines are not source-error evidence.
Neither a nonzero exit nor words in a traceback qualify by themselves.

This channel supplies diagnostics, not proof that arbitrary guest code is
trustworthy and not additional authority to modify files. Existing user/session,
source-revision, bounded-repair and execution-lifecycle checks remain required.
Uncaught native UIKit callback errors reach this entry. Tkinter's default
callback handler catches exceptions without terminating the app; those caught
errors are not currently reported by this outer entry, and custom handlers are
not replaced.

Native fixtures `runtime-value-error.py`, `runtime-callback-type-error.py` and
`runtime-exit.py` check typed class/line evidence and the distinction from
explicit exit status. `runtime-ui-callback.py` additionally requires a real
installed UIKit button click; the standalone callback fixture is not UI proof.
Implementation and fixture availability alone do not establish passing native
or installed-runtime acceptance.

Source: https://www.python.org/ftp/python/3.13.16/Python-3.13.16.tar.xz

Archive SHA-256:
`f4b1bfb3c79b5bb11b8d228a12504163b4c0dab4d679828d8f5f26b6cb6ab35d`

CPython is distributed under the Python Software Foundation license. Its
upstream `LICENSE` and bundled third-party notices must accompany release
artifacts. Downloaded sources stay outside Git; the builder verifies the pinned
archive before extraction.
