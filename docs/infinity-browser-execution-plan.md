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
unsuccessful correction loops, continue with evidence-led corrections per the
user's explicit override of the limit. Keep bounded resources/cancellation and
preserve authority checks.
Do not rebuild ISOs for compile-only prerequisite changes or advertise the
browser as installed before gate 6 passes.

## Current checkpoint

- Expanded native page proof now passes pointer/keyboard, navigation/reload,
  history and scrolling: `20260926T224911285190Z-76406.json`. Shared bounded
  TLS exhaustion/reuse passes on both CPU guests. External network and installed
  integration are still open; no beta ISO has been produced.

- Native AArch64 inline HTML/CSS rendering and JavaScript DOM mutation now pass
  actual pixel/value assertions: `20260926T222800003418Z-67530.json`. This
  supersedes older compile/startup/rendering blockers below. Next verify native
  input/navigation, network service wiring, shell and installed packaging.

- Latest link pass resolves C++/zlib lookup and includes real native primitives.
  C and reentrant allocation ABI tests pass in both architecture guests. The
  engine link now fails on duplicate newlib/native allocator and abort providers
  (`20260926T203455445992Z-31857.json`). Correct archive selection/order next;
  do not permit duplicate providers. Native threading/mapping/storage and
  installed-browser acceptance remain open. See the port readout for evidence.
- Native HTTP metadata/readiness improvements tested; bounded selector exists.
- Runtime std bindings now execute through an opt-in single-owner provider in
  both native guests; AArch64 Servo metadata compilation passes, but executable
  linking and engine execution remain outstanding.
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
- FreeType/libpng target headers and native allocator bindings are now implemented;
  the rasterizer and allocator compile. Allocation behavior passes both guest tests.
  The engine build has advanced into SpiderMonkey's native platform configuration.
  GPU context creation explicitly rejects unsupported requests; a software page
  rendering context is still required, not implied by the Surfman type adapter.
- SpiderMonkey's response-file probe now links a freestanding test ELF without
  requiring Unix startup objects. This proves that probe only, not engine linkage.
- Native worker stack-bound queries pass on both architectures, including local
  addresses across sleep, disjoint stacks and refusal to guess the root stack.
  A native SpiderMonkey timestamp backend uses the same clock provider as std;
  its checked conversion passes C/C++ host tests, not engine execution.
- The current engine correction pass is still compile-only. Native C/C++ runtime,
  actual software rendering and installed-system acceptance remain outstanding.
- `MADV_NORMAL` is now declared; unsupported advice still fails explicitly and
  its boundary tests pass. Packaged font catalog and memory-backed FreeType paths
  type-check. Native file-access scope rejection passes a host behavior test.
- The bindgen bitfield-layout panic is corrected: non-bitfield compounds no
  longer resolve recursive libc++ template layouts while their item is loaned.
  Actual bitfield allocation is unchanged. Host generated-layout assertions and
  C++ observation of Rust-written normal/packed bitfields pass, manifest
  `20260926T190626642567Z-7129.json`. Engine compilation passed binding generation
  and reached SpiderMonkey's allocation-size reporting hook.
  Subsequent native allocator, opaque stream/integer ABI and navigator platform
  corrections pass the full minimal-feature AArch64 metadata check, manifest
  `20260926T191902704731Z-18582.json`. Actual native archive generation also passes,
  manifest `20260926T192356360339Z-22363.json`. Executable link diagnostics now
  expose missing stdc++/zlib linkage (`20260926T193439702091Z-26580.json`). Select
  the native libc++ ABI consistently and propagate native dependency search paths
  before resolving service symbols. Never boot the link-only diagnostic entry.
  Native mapping/protection and production C++ synchronization remain unproved.
  Platform enums, image-reader stdin compilation, native clock selection and
  filesystem URL rejection now pass the Rust compiler stage. Configure uses
  explicit target headers/tools rather than accidental host header detection.
- Native-root WebPKI adapter: four host behavioral tests pass (valid public
  chain, wrong hostname, expiry and malformed data). No guest TLS handshake or
  installed browser is implied. See `tools/servo-certificate-test/run.py`.
- Production governed worker, network grants/event pump, C/C++ runtime linkage,
  actual engine execution, rendering and installed acceptance remain required.
