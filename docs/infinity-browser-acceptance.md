# Infinity Browser v0.1 acceptance ledger

This is an implementation ledger, not an installed-browser completion claim.
Servo is pinned to `d05154e2b4def11a9fefe412898a0a6c8925a9cd` (0.6).

## Open acceptance gates

1. Connect the native shell and retained compositor to the engine worker.
2. Connect governed desktop networking to the resource callback boundary.
3. Verify responsiveness, input, resize, cancellation and window lifecycle in
   the actual desktop; measure launch/load/first-paint, CPU and peak RAM.
4. Commit downloads into native Object/Namespace storage with metadata.
5. Register and package the default browser for live and installed generations.
6. Cold-install, detach the ISO, and prove HTTPS, CSS/image, JS, link navigation,
   input, scrolling and downloads. Verify both architecture targets separately.

## Evidence already obtained

All commands run through `./build-kit run`. Manifests are under
`builds/manifests/`; they identify actual commands and exit statuses.

| Manifest | Behavioral evidence | Boundary |
| --- | --- | --- |
| `20260927T022632587679Z-16112.json` | `https://example.com/`, HTTP 200, real Servo DOM and rendered pixels over native DNS/TCP/TLS | AArch64 freestanding guest, not installed desktop |
| `20260927T035835527192Z-42968.json` | Disconnected-channel retirement, live-channel delivery across selector reconstruction, JS pixels, resize, close/reopen, clean shutdown | Tracing component guest |
| `20260927T040459264127Z-47143.json` | Same component lifecycle with tracing disabled and an aligned 256 MiB heap | Three raster gates, two released requests, native return 0 |
| `20260927T040808632926Z-47636.json` | 13 browser-core tests, including concurrent frame integrity, buffer bounds, coalescing and stale-generation rejection | Host behavioral tests, not guest/UI proof |
| `20260927T040859563049Z-47691.json` | Actual Servo pixels copied into owned frame transport and inspected by the consumer across resize/reopen | Non-tracing component guest |
| `20260927T041146653085Z-47811.json` | Ctrl+Shift+K autorepeat reaches real JS and changes pixels; four raster gates through frame transport plus clean shutdown | Non-tracing component guest, 256 MiB grant |

The small lifecycle fixture with the keyboard listener peaked at **149,722,880 bytes of allocator
reservation**, including buddy rounding. This is not total RAM and is not a
representative production-page memory measurement.

## Shutdown defect fixed

The memory-profiler receiver disconnected during shutdown. Servo's in-process
selector kept returning that closed receiver. ResourceManager continued the loop
without yielding, starving the native script-thread joiner. Closed receivers now
retire once, and receiver IDs remain stable when rebuilding selectors. No engine
teardown is bypassed, leaked or reported complete prematurely.

## Current limitations

- No cold-installed browser acceptance yet; the existing ISO must not be
  described as containing a working browser on the strength of these probes.
- Native engine execution evidence is AArch64 only.
- The native HTTPS provider currently accepts GET over HTTPS and bounds response
  size. Broader method/plaintext handling is not proven.
- Worker ABI and owned pixel transport are implemented, but production desktop
  callbacks, default-app registration and download persistence remain open above.
