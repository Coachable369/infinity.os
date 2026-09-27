# Native Servo adapter

`resources.rs` owns the bounded, fail-closed Servo resource interception queue.
Install it as the global `ServoDelegate` and forward each WebView's resource
callback to it. Pump it on the engine owner worker, never from a paint callback.
Call `cancel_all` before superseding navigation and `close` during teardown.
Providers must return unique nonzero operation IDs, enforce revocable native
authority, bound allocations and release resources through `cancel`.

`native_https.rs` connects the queue to the shared InfinityOS native DNS/TCP/TLS
client. Its factory supplies authorized links, network configuration, trusted
UTC and genuine entropy. It uses public certificate roots, serializes requests
over the owned link and caps a response at 128 KiB. No host HTTP implementation
is involved. HTTP plaintext and non-GET operations currently fail closed.

Current proof: `tools/servo-platform-probe/run-engine-guest.py --page-probe`
executes real Servo with injected response bytes; `--network-probe` is the
separate external HTTPS test. Neither is an installed browser. Production
worker, shell, object storage and installer wiring remain acceptance gates.

`session.rs` owns one WebView and its bounded software surface. It accepts
navigation, input, history and resize operations on the engine owner thread and
publishes RGBA bytes only after a ready-frame notification. The host must copy
these into an owned window surface; no global framebuffer is exposed. Initial
navigation is retained until the initial document is ready. Guest coverage proves
resource loading, pixel delivery, resize, idle suppression and invalid-input
rejection. Production worker/desktop wiring is still pending.

## Native worker component

`component.rs` provides the integer/pointer-only ABI in
`../infinity-browser-core/worker.rs`. Link with
`./build-kit run python3 tools/servo-platform-probe/link-engine.py --component`.
The relocatable output prefixes every runtime symbol and retains Rust inventory
constructors separately from lazily linked C libraries. It must not collide with
the kernel's Rust runtime or other native components.

The OS must grant a stationary callback table and a permanent 128–512 MiB heap,
then call the entry on one dedicated owner CPU. Callbacks are nonblocking mailbox
operations, not permission to use BSP-only services or UI from that CPU. The
component processes at most 16 commands per pump. There is one reopenable WebView;
window close cancels network requests. Frames are borrowed RGBA bytes valid only
during the frame callback. Native display composition and persistent frame
ownership remain the host's responsibility.

Each response handle retains its buffers until `cancel`, including after success.
Headers are bounded to 16 KiB/32 fields and body to 1 MiB. The host enforces network
capabilities and revocation; the component cannot open a socket itself. Diagnostic
events belong to the supervisor and must never become raw user-facing error text.

The component guest proves JS-mutated pixels, resize, close/reopen and full
engine shutdown. Production desktop callbacks and installed packaging are not
yet connected. Do not treat component linkage as installed-browser acceptance.

The lifecycle fixture measured 149,722,624 bytes of peak allocator reservation
(including buddy rounding and transient allocations) on its small test pages.
This is not total process RAM or a production-page budget. A 192 MiB grant at
the fixture address yields only a 128 MiB buddy arena and failed allocation;
the current fixture passes with a 256 MiB aligned grant and tracing disabled.
Heap high-water accounting is covered by allocator exhaustion/reclamation tests.

The shutdown defect was a disconnected memory-profiler receiver remaining in
Servo's in-process selector. ResourceManager repeatedly received ChannelClosed
without blocking, starving a runnable script-thread joiner. The native port now
retires disconnected receivers while preserving stable receiver IDs, including
when the selector is rebuilt. `run-component-guest.py --trace` additionally tests
closed-channel retirement and blocking delivery from real native worker threads.

Evidence manifests (repository-relative):

- `builds/manifests/20260927T035835527192Z-42968.json`: selector regression plus
  complete native component lifecycle, tracing enabled.
- `builds/manifests/20260927T040459264127Z-47143.json`: production non-tracing
  component, 256 MiB heap, three pixel gates, two released requests and clean exit.

These are freestanding AArch64 guest results, not cold-installed OS verification
or x86_64 engine execution. Peak reservation excludes code, surfaces outside the
component and the rest of the OS.

The updated fixture also routes actual frames through `infinity-browser-core/frames.rs`
and verifies Ctrl+Shift+K autorepeat by a real JS listener changing rendered pixels.
Its four-gate lifecycle passed in `20260927T041146653085Z-47811.json`, with
149,722,880 bytes peak reservation. `Command.b` carries the native modifier mask;
unsupported flag bits are rejected rather than silently changing input meaning.
