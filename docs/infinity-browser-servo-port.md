# Infinity Browser v0.1: port status

Status: **not implemented or packaged; acceptance remains open**.

## Binding traversal correction, September 26

The previously blocking bindgen panic is resolved without disabling JavaScript.
The failing compound was libc++'s recursive variadic `std::__union`, which has
no bitfields. Bindgen unnecessarily resolved its layout while temporarily
removing the item from its registry. The checksum-pinned source overlay now asks
for layout only when real bitfields require allocation units. Real bitfield
normalization remains upstream code; no ABI size or padding is guessed.

`tools/servo-bindgen-test/run.py` passes through build-kit: generated compile-time
size/alignment/offset assertions plus one runtime Rust-to-C++ roundtrip for
normal and packed bitfields. Manifest
`builds/manifests/20260926T190626642567Z-7129.json`. This is host ABI evidence only.

The native SpiderMonkey allocation-size hook now queries the existing governed
allocator instead of selecting a host malloc API. Opaque C stream and
pointer-sized integer declarations are supplied by the native libc overlay.
The native navigator platform identifies InfinityOS in pages and web workers.
The complete minimal-feature AArch64 Servo metadata check now passes, manifest
`builds/manifests/20260926T191902704731Z-18582.json`. The locked ABI regression
also passes again, manifest `20260926T192336471107Z-22251.json`.
These fixes do not establish engine linkage, rendering or installed browsing.
The probe now has an explicit `--codegen` mode to build real native archives;
even successful archive production is not executable or installed acceptance.

Actual AArch64 archive generation **passes**, manifest
`builds/manifests/20260926T192356360339Z-22363.json` (7m 14s, debug profile).
`link-engine.py` now retains real `ServoBuilder::build` initialization and attempts
an executable link using the matching custom Rust core/panic archives and host
proc-macro search path. It is diagnostic-only and must never be booted: native
providers are not initialized by its entry point.

The link fails, manifest `20260926T193439702091Z-26580.json`, on missing
`-lstdc++` and `-lz`. Next: select libc++ consistently in all native C++ dependency
builds (including mozjs's global CXXSTDLIB handling), propagate actual Cargo native
library search paths, then resolve the native allocator/thread/mapping/service
symbols. Do not substitute host libraries or fabricate successful implementations.
No engine execution, page render, installed browser or new ISO was produced.

## Memory advice follow-through, September 26

The `MADV_NORMAL` compile blocker is resolved in the shared native mapping header.
The existing native C boundary still rejects unimplemented advice with ENOSYS;
it does not falsely validate a mapped range. The directory/platform behavioral
test passes, including normal advice and zero-length rejection, manifest
`builds/manifests/20260926T181826201747Z-73261.json`.

Further corrections in this pass:

- SpiderMonkey native condition variables select monotonic timed waits instead
  of falling into the macOS-only relative-wait API. Native C++ synchronization
  linkage/execution remains required; this is not a working pthread claim.
- Native mapped-file access scopes reject before executing their bodies. Owned
  buffers execute without pretending to recover Unix SIGBUS faults. A test of
  the actual staged macros passes: manifest
  `builds/manifests/20260926T182653848720Z-85372.json`. This is a host behavior test,
  not a guest fault-recovery test.
- Optional page-fault statistics use the upstream unavailable sentinel. Zero
  here must not be reported as a measured native fault count.
- Added a packaged three-family font catalog (Fira Sans, EB Garamond, IBM Plex
  Mono), memory-backed FreeType construction and raw-font paint registration.
  No fontconfig, host path or native filesystem font lookup is enabled. Font
  code type-checks; rendered glyphs and installed font parity are not yet proved.
- Added the native ENOSPC value and pinned bindgen to the same LLVM installation
  as target compilation. Preserved existing target-independent application code.

Six engine passes were attempted. The final manifest is
`builds/manifests/20260926T183456290484Z-92976.json`, exit 101. The remaining observed
failure is bindgen 0.72.1 while generating SpiderMonkey Rust bindings:
`Not an item: ItemId(57186)` at `ir/context.rs:1492`. The backtrace identifies
`compute_bitfield_units -> Type::layout -> CompInfo::layout -> Type::layout ->
resolve_type`; matching LLVM/libclang does not resolve it. Next correction must
investigate the unresolved type during bitfield layout and validate resulting
ABI layouts, not replace them with guessed padding or suppress the panic.

Stopped at the six-correction limit. No engine executable, installed browser,
new installer or successful HTTPS/JS/download proof was produced. This checkpoint
supersedes the earlier immediate compile blockers below.

## Native clock and stack checkpoint, September 26

The linker response-file probe now links an actual freestanding test ELF with
the probe's main as entry; it no longer requires Unix startup libraries to test
response-file consumption. This is not a production engine link.

Added a SpiderMonkey timestamp backend using the existing native monotonic ABI.
The provider must be installed before C++ engine static initialization. Checked
nanosecond conversion rejects malformed fields, provider failures and overflow.
Native worker stack bounds come from the executor's leased stack allocation,
never pthread introspection or guessed frame addresses. Unknown root stacks are
rejected. The provider remains single-owner and opt-in.

Behavioral evidence through build-kit:

- C and C++ clock conversion vectors pass: manifest
  `builds/manifests/20260926T175721534245Z-56358.json`.
- ARM64 guest std/runtime tests pass: manifest
  `builds/manifests/20260926T180348658521Z-64389.json`.
- x86_64 guest std/runtime tests pass: manifest
  `builds/manifests/20260926T180418979952Z-64549.json`.

Guest tests cover two disjoint live worker stacks, local-address containment,
stable bounds across sleep and root-stack rejection, alongside existing std
thread/allocation/TLS/wait coverage. They do not execute Servo or an installed OS.

Build corrections also invalidate partial configure outputs, select native
libc++ visibility flags without GCC system-header pragmas, and expose the native
memory-map declarations. Declarations do not implement mapping/protection.

The sixth engine attempt ended with exit 101, manifest
`builds/manifests/20260926T180939707188Z-68952.json`. The remaining observed compile
failure is `mfbt/Poison.cpp:125`: `MADV_NORMAL` is undeclared. The native
memory-advice/protection contract needs implementation and behavioral tests;
adding a constant alone would not establish working VM services. The previous
allocator exception-specification conflict and Rust clock extern safety lint
did not recur. Native allocation header selection now uses standard declarations
instead of optional Unix allocator extensions.

Stopped after six unsuccessful engine correction loops as required. No new ISO,
linked engine or installed browser was produced. Software rendering, production
worker integration and cold-installed HTTPS/JS/download acceptance remain open.

## Previous correction checkpoint, September 26

Six further engine attempts ended at manifest
`builds/manifests/20260926T174500552737Z-44932.json` (exit 101).
The immediate failure is SpiderMonkey's linker response-file probe: the native
compiler cannot find `crt0.o`, target `libclang_rt.builtins.a`, or `-lc`.
Do not force the probe to succeed or link macOS libraries. Supply the actual
native executable/toolchain contract, then continue the C/C++ port.

Changes in this pass:

- Added Infinity OS/kernel compiler detection, explicit target AR/CPP, removed
  inherited include flags, and applied target flags to early configure probes.
  Configure input invalidation now accounts for those command changes.
- Disabled ICU filesystem data access; packaged memory data remains required.
- `imsz` native stdin returns Unsupported; its byte/reader decoding is preserved.
- Servo's monotonic timer uses the existing native clock ABI. Filesystem URL
  conversion returns an error rather than inventing host paths.
- Rustls platform verification uses WebPKI with the same pinned 1.0.9 Mozilla
  trust roots as native HTTPS. Signature, chain, hostname and time checks remain.

Behavioral evidence: `./build-kit run python3 tools/servo-certificate-test/run.py`
passed all four tests (valid recorded public chain, wrong hostname, expired chain,
malformed certificate), manifest
`builds/manifests/20260926T174335463695Z-44266.json` (exit 0).
This compiles the staged verifier's native-root branch on the host; it is not
guest networking or installed-system evidence. Earlier guest primitive results
below were not rerun in this pass.

The six-correction limit has been reached. No new ISO was produced and no browser
was installed. Remaining gates include native C/C++ runtime and memory services,
software page rendering, production service/worker integration, shell, packaging
and cold-installed HTTPS/JS/navigation/download proof. This supersedes the
immediate compile failures recorded in the historical checkpoints below.

## Binding follow-through, September 26

Native C malloc/free/realloc and Rust System allocation share the granted heap.
Checked metadata preserves original layouts and provides real payload-size
reporting. Null/zero-size behavior, alignment through 4096 bytes, byte preservation
across grow/shrink, failed resize preserving the old allocation, and complete
reclamation are asserted inside both architecture guests:

- ARM64: `builds/manifests/20260926T171041355901Z-7978.json`, exit 0.
- x86_64: `builds/manifests/20260926T171149836879Z-11131.json`, exit 0.

FreeType now uses authenticated target zlib headers; its memory-font rasterizer
passes the compiler stage. WebDriver server startup is deliberately unsupported
on native targets. Surfman's native GPU types are uninhabited and connection
creation returns an error: this prevents pretending a working GPU context exists.
The browser must use a real software rendering context, which is not implemented
yet. Downloaded SWGL source is exploratory only, not an integrated renderer.

SQLite now selects its custom-platform interface instead of Unix files and
pthread mutexes. Native VFS/mutex linkage and execution remain unimplemented.
SpiderMonkey is staged with an explicit Infinity OS/kernel identity and canonical
C target spelling; this is configuration work, not proof that JavaScript runs.
The full browser remains unlinked and uninstalled. Prior binding failures in
the historical sections below have been superseded by this checkpoint.

Sixth resumed compiler attempt:
`builds/manifests/20260926T172018929566Z-22141.json`, exit 101.
Remaining immediate failures are SpiderMonkey's configure `Kernel` enum rejecting
`Infinity`, and `imsz` 0.4.1 moving `Stdin` from a shared reference because its
platform-specific stdin implementation is absent. Also audit inherited build
environment before claiming hermetic cross compilation: SpiderMonkey printed a
host OpenSSL include path and an unavailable target-prefixed AR/CPP command.
The six-correction limit stops this iteration here. No beta browser or new ISO
is claimed; the requested installed-browser outcome has not been achieved.

## Latest checkpoint: native TCP and async runtime, September 26

Added a fixed-capacity smoltcp reactor with generation handles, endpoint grants,
deadlines, backpressure, TCP EOF/reset, UDP datagram boundaries and revocation.
Native owner-local callbacks bind real std TCP streams; Mio observes those
streams through the bounded selector. Tokio and Hyper use that path without
socket2/raw descriptors. Unsupported bind/tuning/signal APIs reject explicitly.
All three pinned getrandom versions delegate to the existing entropy ABI and
fail closed when the provider refuses a fill.

Behavioral evidence:

- ARM64 std/Mio/Tokio TCP plus entropy success/denial:
  `builds/manifests/20260926T164507979288Z-3403.json` (exit 0).
- x86_64 equivalent:
  `builds/manifests/20260926T164551905660Z-3507.json` (exit 0).
- Full HTTP/reactor library suite: 19 passed, 0 failed:
  `builds/manifests/20260926T165046464193Z-4156.json`.

Guest assertions cover repeated TCP readiness, partial I/O, shared stream
lifetime, timeout cancellation, async echo, and entropy refusal. Packets travel
between two independent stacks inside the guest. There is no host TCP helper,
but this is also **not real NIC, internet, or installed-system evidence**.

Engine compile attempt `20260926T164930203585Z-3870.json` still exits 101:
FreeType/libpng lacks the target zlib include path; servo-allocator cannot find
native C allocation and usable-size bindings. Source staging now selects
memory-backed FreeType, in-process IPC, and Stylo opaque native thread IDs;
those changes do not establish a linked or executing engine. The six-attempt
correction limit was reached. Resume from these exact compiler failures.

Production worker isolation/preemption, C/C++ runtime binding, NIC/service grants,
queued-frame egress revalidation, DNS integration, engine linkage/rendering,
shell and installed acceptance remain open. No fresh browser ISO was generated.
Older sections below are historical checkpoints, not current completion claims.

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

### Native std execution and Mio readiness, September 26 follow-up

The opt-in `native-abi` provider now executes real Rust standard-library threads
on independent native stacks. Shared `executor.rs` implements bounded creation,
fair cooperative dispatch, sleep, compare/wait/wake, join/detach, stale handles,
and per-thread TLS teardown. `native.rs` supplies the staged std ABI using an
explicitly granted heap and native callbacks; it binds to one owner CPU and
reports one CPU of parallelism. No Linux/POSIX runtime or serial pthread shim
is used in the guest execution.

Both guest targets pass actual `std::thread::Builder`, Mutex/Condvar, sleep,
join and thread-local destructor tests. A 16-thread exhaustion test verifies
rejected callbacks do not run and all joined stacks return to the initial heap
allocation count. The fixture uses real ARM architectural counters / x86 HPET;
it explicitly denies UTC and entropy, which these tests do not require.

Pinned Mio 1.2.3 is staged from checksum-verified bytes without changing the
registry cache. Its native Poll/Waker/Source bridge shares
`kernel/runtime/http/selector.rs`; guest tests cover control wake coalescing,
timeouts, read/write readiness edges, rearming, foreign-registry rejection,
deregistration with pending events, exhaustion and capacity recovery.

- Combined AArch64 std/runtime/Mio guest:
  `builds/manifests/20260926T160435306035Z-94773.json`.
- Combined x86_64 std/runtime/Mio guest:
  `builds/manifests/20260926T160500012164Z-94818.json`.
- Six host primitive regression tests:
  `builds/manifests/20260926T160314929560Z-94712.json`.
- Full pinned engine attempt:
  `builds/manifests/20260926T155913083171Z-91144.json`, exit 101.
  Selector definitions now compile; Mio TCP/UDP wrappers and socket2's missing
  native backend still prevent the engine build (29 Mio errors remain).

Run the combined fixture with `./build-kit run python3
tools/servo-platform-probe/run-memory-guest.py --mio-probe --arch aarch64`
(or `x86_64`). Evidence lives under `build/servo-mio-guest/<arch>/`.

**Not production or installed-browser acceptance:** the executor is cooperative
and needs a dedicated governed worker, native event/service wiring, C/C++ TLS
integration and stack protection. A non-yielding script can monopolize that
worker. The std fixture's idle callback is only a processor hint, not the
production service event pump. TCP socket binding, full engine linkage,
rendering, native shell and installed browsing tests remain unfinished. The
existing typed connection service supports loopback/datagrams; it must not be
misrepresented as the missing outbound TCP socket backend.

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
