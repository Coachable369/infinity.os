# InfinityOS Fatal Crash Screen IDesign Kit

Status: TESTED

## Visual contract

- The fatal surface is an emergency-only, full-frame composition with a pure black background.
- A centered black-and-white 3D pirate flag carrying a white skull-and-crossbones is the sole illustrative asset.
- Bright red communicates the stop condition, crash code, dividers, and severity. White carries the title and primary facts. Muted gray carries recovery guidance.
- The screen uses the bundled InfinityUI type atlas; it does not depend on the desktop, window server, skin service, heap allocation, or object store.
- No animation, translucency, wallpaper, pointer, modal, or ordinary application chrome appears after the fatal latch is set.

## Information hierarchy

1. `INFINITYOS HAS STOPPED` identifies the terminal machine state.
2. A short category explains whether the failure was a kernel panic, invalid bootstrap contract, processor exception, or explicit invariant violation.
3. `WHAT HAPPENED` provides a bounded human-readable diagnostic summary.
4. `TECHNICAL DETAILS` exposes a stable crash code, boot/runtime phase, source line and column, and a deterministic incident fingerprint.
5. The footer asks the user to record the code and fingerprint, restart, and report repeatable failures.

## Safety contract

- First failure wins. Recursive panics never overwrite the original record or re-enter complex rendering.
- Rendering is allocation-free and writes directly to the firmware-provided framebuffer.
- Every write is clipped to the framebuffer dimensions and byte extent captured from trusted `BootInfo`.
- Once displayed, interrupts are disabled where the architecture supports it and the processor enters the architecture idle loop.
- The crash record stores identifiers and bounded diagnostics only. It must never include credentials, capability tokens, object content, or other secrets.

## Asset

`assets/crash/infinity-fatal-pirate-flag-v1.png` is the editable RGBA source. The matching 32-bit top-down BMP is embedded into x86_64 and AArch64 live and installed kernels. Both forms are packaged into every live image and fresh-installed System Generation for asset parity. The bounded legacy x86 image draws a monochrome fallback emblem to preserve its strict bootstrap size limit.
