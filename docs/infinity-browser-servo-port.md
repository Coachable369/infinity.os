# Infinity Browser v0.1: port status

Status: **not implemented or packaged; acceptance remains open**.

## Native event-layer continuation

`Transport::readiness` now reflects whether reads/writes can complete, including
EOF and terminal failures. `async_stream::Session` wakes only ready operations,
and releases its internal borrow guards before invoking executor callbacks.
Packet-based tests use two native TCP stacks (no host sockets) to verify idle
polls do not wake blocked reads/writes, received data wakes once, acknowledged
send capacity wakes once, and clean FIN/cancellation complete waiting reads.

`http::selector` adds bounded edge-triggered bookkeeping: generation-tagged
registrations, interest replacement/rearming, event coalescing, control wakes,
stale-event rejection and round-robin delivery under output backpressure.
Registration identities are selector-local, not capabilities. The owner must
serialize access, check network authority, and publish readiness after both NIC
progress and operations that drain readiness. The selector does not itself wait,
own sockets, or grant access.

This is **not yet the Mio target backend**. Mio socket types, native service
handles, and real thread park/wake still require integration. No engine compile
success, browser execution, fresh-install acceptance, or new ISO is claimed.

Evidence under `builds/manifests/`: behavioral suite 27/27 passed in
`20260926T090024298874Z-47977.json`; shared native HTTP library compilation
passed for AArch64 in `20260926T085927150902Z-47831.json` and x86_64 in
`20260926T090004048925Z-47921.json`. These compile gates are not guest execution.

## Latest continuation: native response metadata and shared compiler gates

The native HTTP service now exposes `client::get_with_headers` and
`https::get_with_headers`. These use the existing capability/deadline-governed
transaction, retain the final authenticated response head in caller-owned bounded
storage, reject insufficient header capacity, and leave legacy body-only GET
behavior intact. `response::Headers` validates framing and exposes repeated
fields separately without allocation. This supplies metadata needed by a future
browser adapter; it does not itself implement redirect policy, cookies, or Servo
request interception.

Behavioral checks exercise TLS 1.3 against an independent test server, interim
103 followed by final 200, decoded chunked bytes, repeated Set-Cookie fields,
header capacity failure, malformed framing, and capability/deadline cancellation.
Host test sockets belong only to the fixture, not the native service.

The shared dependency probe now accepts `--arch aarch64|x86_64`, selects native
target headers/compiler flags, and keeps per-target evidence separate.

Evidence (all under `builds/manifests/`):

- HTTP behavioral suite: `20260926T080926800727Z-41682.json`, 22 tests passed.
- Native HTTP AArch64 compile: `20260926T080646693289Z-39858.json`.
- Native HTTP x86_64 compile: `20260926T080725779455Z-40116.json`.
- x86_64 fontsan compile: `20260926T080758563455Z-40291.json`.
- x86_64 aws-lc-sys compile: `20260926T080829577060Z-40732.json`.
- Full AArch64 Servo check: `20260926T080857567635Z-41610.json`, exit 101;
  mio 1.2.3 lacks the native socket/event backend (47 compiler errors).

Remaining blockers are real native socket/event integration, executable std
thread/wait/allocator bindings, complete engine compilation/linking, shell and
surface integration, and installed-system acceptance. No browser ISO or installed
runtime proof is produced by this continuation. Shared HTTP source changes will
enter both kernel builds, but freshly installed behavior remains unverified.

## Pinned source inspection

Servo upstream commit `d05154e2b4def11a9fefe412898a0a6c8925a9cd`, workspace
version 0.6.0, was inspected in `build/servo-port-audit`. This is an audit checkout,
not an installed dependency or a supported engine build.
Upstream: https://github.com/servo/servo/tree/d05154e2b4def11a9fefe412898a0a6c8925a9cd

## First blocker

InfinityOS builds freestanding `no_std` kernels. Servo uses Rust `std`, real
threads, synchronization, timers and platform libraries. The native compiler C
compatibility layer is explicitly serial: `sdk/compiler/pthread.c` rejects
`pthread_create` with ENOTSUP. It must not be passed off as a Servo threading
implementation. Existing memory mapping/protection functions delegate to host
callbacks; they are not evidence of SpiderMonkey compatibility.

The first prerequisite is a native runtime target supporting actual thread
creation/join, waiting/wakeup, independent TLS, allocation and monotonic clocks.
Do not substitute Linux userspace or silently report successful fake threads.

`./build-kit run python3 tools/servo-platform-probe/run.py` records compiler exit
status for a small std prerequisite on both current freestanding targets. This
is only a compile gate, not a full Servo build or runtime test. The Rust function
contains behavioral assertions for eventual guest execution; the script does
not execute them. Build-kit serialization initially prevented running this gate
because another incremental build was active. No build lock was bypassed.

### September 26 compiler evidence

The lock cleared and the probe ran through the build kit. Both current targets
returned exit status 1 for the prebuilt-std compile gate (E0463: std unavailable).
Added an isolated Cargo manifest and `--build-std` probe mode to test whether
rebuilding the standard library resolves this without a platform port.

`./build-kit run python3 tools/servo-platform-probe/run.py --build-std`
returned status 101 for both targets. Rust 1.98.0 standard-library compilation
fails on missing allocator and I/O-error platform selections, TLS key operations,
and random-byte support. This confirms that installing a precompiled library or
merely adding `build-std=std` is insufficient. No unsupported-service stubs were
added to make compilation appear successful.

Structured evidence: `build/servo-platform-probe/build-std-result.json`.
Per-target diagnostics: `build/servo-platform-probe/*-build-std.log`.
Build manifest: `builds/manifests/20260926T062036189443Z-74686.json`.
These are failed prerequisite builds, not passing behavioral tests or Servo
execution. The existing AP matrix workers are bounded compute jobs, not proof
of general Rust threads with blocking/wakeup and independent TLS.

Next implementation gate remains a native Rust standard-library platform port:
allocator, native error translation, entropy, TLS and real thread/wait support,
followed by guest execution of `runtime_roundtrip`. Do not modify the host
toolchain in place or reuse the serial compiler shim as a threaded runtime.

### Initial port code

`sdk/servo-std` now contains isolated std allocator, error translation, entropy,
and OS-key TLS adapters. A pinned project-local Rust source staging script and
`--native-overlay` compile mode were added. Native service symbols deliberately
remain required at link time; their implementations are not yet supplied.
This is incomplete port code, not a fix for thread execution or a working Servo
runtime. Verification is pending because build-kit session
`20260926T062125727600Z-74866` holds an interactive `/bin/zsh -f` shell. It had no
child process at inspection; the session was not terminated or bypassed.

### Browser core and resumed compilation

Added `sdk/infinity-browser-core`, a standalone no_std crate with no engine or
OS-service dependencies. It implements navigation generations and deadlines,
terminal close state, a 32-entry input queue with explicit backpressure,
viewport pointer clipping, bounded download staging and filename/media-type
validation. Download staging is not an object write; native authorization,
durable storage and metadata integration are still required.

Five behavioral unit tests passed via
`./build-kit run cargo test --manifest-path sdk/infinity-browser-core/Cargo.toml`.
Evidence: `builds/manifests/20260926T064004393125Z-76997.json`.

After the lock cleared, `--native-overlay` compilation passed for both ARM64 and
x86_64. The probe explicitly opts into Rust's experimental `restricted_std` only
with the isolated overlay flag. This does not remove or implement unsupported
services. Native ABI symbols are still unresolved until executable linking;
threads, waits and clocks still need platform implementations and guest proof.
The library compile is not Servo compilation or runtime acceptance.
Evidence: `builds/manifests/20260926T064112083671Z-77475.json` and
`build/servo-platform-probe/native-overlay-result.json`.

No production runtime consumers, installer packages or ISOs were changed by
this step. Fresh-install browser acceptance remains entirely outstanding.

## Narrow integration sequence after runtime support

### Additional prerequisite work (September 26)

- Added native thread, clock and compare/wait/wake ABI adapters. The overlay now
  selects upstream futex-based mutex, condition-variable, once, rwlock and parking
  code, rather than unsupported no-thread fallbacks. Both architecture library
  compile gates passed in manifest
  `builds/manifests/20260926T065703273765Z-89026.json`; **no native ABI provider has
  been linked or executed**.
- Added `sdk/servo-runtime-primitives`: bounded, generation-tagged TLS keys and
  independent per-thread values. Two host behavioral tests verify revocation,
  slot reuse, separation and clear-before-destructor behavior. Evidence:
  `builds/manifests/20260926T065832159015Z-89483.json`. This is bookkeeping, not a
  scheduler or working guest TLS.
- Added native-shell layout geometry and hit testing to the browser core. Six
  total host tests passed in
  `builds/manifests/20260926T065732998094Z-89376.json`. The generated visual target
  and its prompt are in `design/infinity-browser/README.md` and
  `design/infinity-browser/idesign-kit-v1.png`. No browser screen is rendered yet.
- `tools/servo-platform-probe/check-servo.py` records a real pinned engine check
  with project-local dependency caches. The engine has not compiled. Diagnostic
  output is in `build/servo-platform-probe/servo-check.log` and structured result
  in `servo-check.json`; `executed` remains false.

The actual engine check exposed AWS-LC's C platform dependency (including missing
freestanding `stdlib.h`), beyond the std opt-in issue. AWS-LC is used not only by
Servo's transport but by subresource integrity and script WebCrypto operations.
Removing its networking consumer alone therefore does not solve this dependency.
Do not use host system headers, silently omit integrity validation, or weaken TLS
to get a successful compiler result. A native C/platform port or explicitly
isolated native crypto integration remains necessary. SpiderMonkey, fonts and
software renderer dependencies have not yet reached their verification gates.

Latest combined check:
`builds/manifests/20260926T070850871847Z-91921.json`.
Both std library probes passed after isolating the stability opt-in in staged
std itself. The Servo check exited 101. It now uses actual freestanding ARM
Newlib headers, not host headers, and reaches these remaining failures:

- `fontsan` cannot find C++ standard headers (`cstdarg`, `vector`, `new`).
- AWS-LC/jitterentropy expects `pthread_rwlock_t` and C atomic integer types not
  provided by the current cross-compilation environment. The existing serial
  voice/compiler libc must not be linked as if it supplied real threading.
- Native `infinity_std_*` symbols still need real allocation, scheduler, wait,
  TLS, entropy and clock implementations plus guest execution. An rlib compile
  cannot prove those symbols exist or behave correctly.

The TLS primitive now also provides bounded four-pass teardown. A third host test
executes callbacks, verifies cleared values before re-entry, repeatedly repopulates
TLS, and proves cleanup terminates. All three tests passed in
`builds/manifests/20260926T071010885646Z-96659.json`.
No new browser ISO was produced; concurrent voice-build ISOs are not browser proof.

### Resumed native dependency and memory work

The previous font/C-header blockers are now resolved for the ARM64 compiler gate:

- `fontsan`: compile exit 0, manifest
  `builds/manifests/20260926T073554340711Z-33744.json`.
- `aws-lc-sys`: compile exit 0, manifest
  `builds/manifests/20260926T073453907584Z-32775.json`.
- Native C offset/lock-handle sizes and atomic support are checked by C static
  assertions before each dependency check. These are ABI compile checks, not
  behavioral thread or crypto tests.
- Byte-order code executes as both C and C++ on the host, checking wire bytes,
  single evaluation and all 65,536 16-bit roundtrips. Manifest
  `builds/manifests/20260926T073004516591Z-30441.json` contains those passing
  commands followed by an earlier, subsequently repaired staging failure.

`prepare-deps.py` verifies the upstream libc archive against the pinned Servo
lockfile, extracts it into a separate native overlay and supplies basic C scalar,
size and offset aliases only under `infinity_native`. It leaves Cargo's registry
cache untouched and preserves all other upstream lock resolutions. The temporary
per-zlib overlay was removed after consumers migrated to this common ABI.

`c-target.h` selects native Newlib declarations and disables AWS-LC's own socket,
filesystem and terminal access with its supported `OPENSSL_NO_SOCK`,
`OPENSSL_NO_FILESYSTEM` and `OPENSSL_NO_TTY` options. Native services must own those
operations. Cryptography, certificate checks, entropy and threading are not
disabled. Passing compilation does not prove entropy/thread symbols link or work.

The new reclaiming buddy arena in `sdk/servo-runtime-primitives/arena.rs` uses only
an explicitly granted mutable region. Five total host primitive tests pass
(`builds/manifests/20260926T072455531003Z-869.json`). A disposable freestanding
ARM64 QEMU guest also executed 65 allocations, payload checks, exhaustion, and
full reclamation of a 4 KiB arena. Structured result:
`build/servo-memory-guest/evidence.json`; manifest
`builds/manifests/20260926T073909758111Z-34504.json`.
This is real freestanding execution, **not an installed InfinityOS runtime or
Servo execution**. The allocator is not yet connected to the native std ABI.

The full engine still fails because `mio` and `socket2` lack this target's I/O
backend (`builds/manifests/20260926T072127728217Z-408.json`). Do not define Linux
target flags to bypass that failure. Actual native scheduler/waits/TLS providers,
network transport integration and further engine/platform dependencies remain.

1. Compile and execute the runtime roundtrip inside InfinityOS, then build
   pinned Servo with default features disabled. Initially omit multiprocess,
   native clipboard, JIT, WebGL/WebGPU and GStreamer. Disabling JIT does not
   remove the need to port SpiderMonkey itself.
2. Connect Servo `SoftwareRenderingContext` / `RenderingContext` to a persistent
   InfinityUI surface. Audit software-renderer platform dependencies before
   claiming it works. Publish bounded damage; never let the engine write the
   global framebuffer.
3. Intercept both `ServoDelegate::load_web_resource` and
   `WebViewDelegate::load_web_resource` through capability-governed native HTTP.
   The default delegate falls through to Servo networking, so a patched,
   fail-closed transport boundary is mandatory, including redirects/subresources.
4. Supply native font/resource loading and separate object-backed profile,
   cookies, site storage, cache, history and download namespaces. Deny unsupported
   privileged APIs. Keep browser UI and permissions outside Servo.
5. Implement the native shell only against the working engine, with bounded
   input/work queues, navigation generations, cancellation and close teardown.
6. Package it into System Generation and prove HTTP/CSS/image, navigation,
   input/scroll, JS DOM mutation, valid/invalid TLS and object downloads after
   cold reboot with media detached. Measure launch, page load, first paint,
   peak RAM, CPU and desktop latency in that installed run.

## Evidence and acceptance

### Native execution prerequisite, September 26

`sdk/servo-runtime-primitives/context.rs` now provides ABI-preserved stack
switching for AArch64 and x86_64, including floating-point control state.
`wait.rs` adds bounded single-owner registration, wake-one/all, monotonic deadline
evaluation and cancellation. It must be serialized with scheduler transitions;
it is not a general cross-CPU futex or a native std thread provider.

The shared freestanding guest performs 2,002 resumptions on two independent
32 KiB stacks, checks private stack canaries and distinct FP rounding modes,
and performs 2,000 prepare/suspend/wake/resume roundtrips. Both guests pass:

- AArch64: `builds/manifests/20260926T153031275297Z-88877.json`.
- x86_64 UEFI handoff: `builds/manifests/20260926T153056832778Z-88958.json`.
- Six host primitive tests: `builds/manifests/20260926T153007238065Z-88826.json`.

Run via `./build-kit run python3 tools/servo-platform-probe/run-memory-guest.py
--context-probe --arch aarch64` (or `x86_64`). Evidence is stored under
`build/servo-context-guest/<arch>/evidence.json`. The x86 probe uses the existing
native UEFI loader and an assertion-controlled QEMU exit status; ARM emits a
binary result record. Neither uses console wording as its test oracle.

These are disposable native guests, **not installed-system browser proof**.
General thread lifecycle, preemption/off-desktop execution, machine TLS and the
native std ABI remain unwired. The execution plan is in
`docs/infinity-browser-execution-plan.md`.

No HTTPS URL, JavaScript result, download, timings, RAM or installed-browser
proof exists yet. No browser launcher item, simulated renderer, host-browser
fallback, new ISO or passing browser acceptance is claimed. Existing MS9–MS12
runtime code has not been changed by this prerequisite investigation.
