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
