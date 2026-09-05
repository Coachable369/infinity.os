# InfinityOS Fatal Crash Handling

## Runtime contract

The fatal path is independent of the desktop, object store, service manager, and heap. Kernel entry records the trusted boot-information address immediately and advances a small phase marker before drivers, runtime, storage, services, UI, and the input loop start. A Rust panic or explicit fatal condition is converted into a fixed-size `CrashReport`.

The first report wins. A recursive fault cannot overwrite the original report or re-enter framebuffer construction. The emergency renderer clears the display to black, composes the monochrome 3D pirate flag, presents red and white diagnostics as one completed frame, masks interrupts, and enters the architecture stop loop.

The report contains:

- a stable numeric stop code;
- the active startup or runtime phase;
- source line and column when Rust supplies them;
- a bounded panic summary;
- a deterministic non-secret fingerprint for comparing repeat incidents.

No credentials, capability tokens, object content, or arbitrary memory are captured.

## Architecture status

- Rust panic capture and emergency display on x86_64: **TESTED** by target compilation and host behavior harness.
- Rust panic capture and emergency display on AArch64: **TESTED** by target compilation and host behavior harness.
- Rust panic capture and compact emergency display on legacy x86: **TESTED** by target compilation and host behavior harness.
- Explicit fatal conditions, including invalid boot contracts when a usable framebuffer descriptor remains available: **IMPLEMENTED BUT UNTESTED IN VM**.
- Architecture exception-vector routing for page faults, general protection faults, synchronous aborts, and double faults: **PLANNED**. Those traps do not yet have a complete IDT/vector substrate in this kernel, so this milestone does not claim that all hardware exceptions are caught.
- Durable crash-record persistence across restart: **PLANNED**. Writing storage from the fatal path is intentionally avoided until a crash-safe append primitive exists.

## Verification

`make crash-screen-test` exercises bounded report capture, first-failure retention, stable structured fields, fingerprint differentiation, and the alpha/monochrome behavior of the packaged pirate-flag bitmap.

`make ui-install-parity-test` extracts the crash assets from every live and installed image and compares their bytes to the source assets. This guarantees fresh-install parity without using rendered text as an acceptance oracle.
