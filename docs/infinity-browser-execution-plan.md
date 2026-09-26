# Infinity Browser execution plan

Acceptance: an installed native Servo browser opens a real HTTPS page, paints
text/CSS/images into InfinityUI, scrolls, accepts keyboard/pointer input, executes
JavaScript, follows links with back/forward/reload, and saves a native object.
No host browser or Linux userspace runtime. Both architectures share application
logic. Compile-only and freestanding-probe evidence never substitute for installed
acceptance. Preserve existing OS behavior and unrelated working changes.

## Gates, in dependency order

1. **Executable runtime** — real independent stacks, scheduling, join/detach,
   atomic compare-and-park/wake, per-thread TLS/destructors, governed allocator,
   entropy and clocks. Start with context switching; test register/stack isolation
   and repeated resumption in disposable guests. Then execute std thread/wait
   roundtrips inside InfinityOS. Cooperative stack switching alone is insufficient
   for desktop responsiveness: engine work must execute off the desktop path.
2. **Engine build** — pin Servo and dependencies, bind native socket/event I/O,
   complete the C/C++ and SpiderMonkey platform interfaces, compile and link a real
   engine executable. No fake successful threads or Linux target impersonation.
3. **First rendered document** — Servo software-rendered persistent surface,
   native fonts, resize, bounded damage, input mapping, cancellation/close.
   Inspect actual pixels and JS DOM state, not diagnostic prose.
4. **Native browsing services** — fail-closed request interception including
   subresources/redirects; DNS/TCP/certificate-validated TLS/HTTP; separate profile,
   site-data and download object namespaces; explicit denial of unsupported APIs.
5. **Usable shell** — implement existing browser design kit with URL entry,
   navigation controls, title/loading/error state; screenshot comparison and
   interaction tests. One stable browsing context; no optional tabs or extras.
6. **Installed acceptance** — package all resources in System Generation and
   parity assertions; build-kit full; cold install, detach ISO, reboot, run all
   browsing cases. Record launch/load/first-paint times, peak RAM, CPU use and
   desktop latency. Report missing evidence rather than infer completion.

## Iteration discipline

For each gate: implement the smallest real missing path, run behavioral tests
through build-kit, inspect only failures, correct and rerun. After six
unsuccessful correction loops, report exact remaining failures per repository
rules. Keep bounded resources/cancellation and preserve authority checks.
Do not rebuild ISOs for compile-only prerequisite changes or advertise the
browser as installed before gate 6 passes.

## Current checkpoint

- Native HTTP metadata/readiness improvements tested; bounded selector exists.
- Runtime std bindings now execute through an opt-in single-owner provider in
  both native guests; complete Servo build still fails.
- Browser shell, engine execution and installed browsing remain unaccepted.
- Executable context-switch prerequisite passes on both architectures: 2,002
  resumptions, independent stack canaries and FP controls, 2,000 wait/wake
  roundtrips. Six host primitive tests pass. See the port readout for manifests.
- Real thread lifecycle, join/detach, TLS teardown, std mutex/condvar and sleeps
  pass guest tests. Production worker/service integration and preemption are not
  implemented. The runtime remains opt-in, not installed.
- Native std/Mio/Tokio TCP and fail-closed entropy now pass in both architecture
  guests. Nineteen HTTP/reactor regression tests pass. These use two packet
  stacks in a disposable guest, not a production NIC or installed browser.
- The engine correction loop stopped after six failed attempts. Current failures:
  bundled FreeType/libpng cannot locate target zlib headers; servo-allocator lacks
  native malloc/free/realloc and allocation-size bindings. The memory-font backend
  staging is not yet compile-verified. Resume at these failures, not UI work.
- Production governed worker, network grants/event pump, C/C++ runtime linkage,
  actual engine execution, rendering and installed acceptance remain required.
