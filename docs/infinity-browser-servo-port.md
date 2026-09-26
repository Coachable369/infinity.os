# Infinity Browser v0.1: port status

Status: **not implemented or packaged; acceptance remains open**.

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

## Narrow integration sequence after runtime support

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

No HTTPS URL, JavaScript result, download, timings, RAM or installed-browser
proof exists yet. No browser launcher item, simulated renderer, host-browser
fallback, new ISO or passing browser acceptance is claimed. Existing MS9–MS12
runtime code has not been changed by this prerequisite investigation.
