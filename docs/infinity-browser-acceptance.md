# Infinity Browser v0.1 acceptance ledger

This is an implementation ledger, not an installed-browser completion claim.
Servo is pinned to `d05154e2b4def11a9fefe412898a0a6c8925a9cd` (0.6).

## Open acceptance gates

Installed integration exposed a payload-capacity blocker: the experimental ARM
kernel is 802 MiB after debug stripping, above the old 512 MiB reservation.
The offline updater refused the write before changing the disk. Fresh layouts
now support up to 1 GiB with a separate object-store offset; legacy, 256 MiB and
512 MiB layouts remain readable and are not migrated in place. UEFI uses the
same 1 GiB payload limit. Layout boundary, object format/write/remount and loader
size-bound tests passed (`20260927T061422344899Z-79781.json`). New installer media
and a matching loader still need to be built and cold-tested; the old installer
cannot be used to prove this larger payload.

The opt-in `./build-kit browser-aarch64` profile now builds the shared browser
component into live and installed kernels. Its incremental build passed
(`20260927T061739267337Z-79995.json`) and atomically replaced
`builds/InfinityOS-aarch64-qemu-test.iso`. This is experimental QEMU test media,
not an updated default/reprovision ISO or a clean-release claim. The installed
kernel is approximately 830 MiB. The configuration stamp invalidates kernel
builds when browser inclusion changes. Installed GUI testing is in progress;
the normal build still leaves the incomplete browser opt-in.

The first browser-enabled ISO boot exposed a UEFI contiguous-allocation failure
across memory-map entries, before kernel entry. The loader now falls back to
descriptor-bounded conventional-RAM reservations and rolls back partial success;
reserved-memory holes remain fatal. Host behavioral tests cover adjacent ranges,
allocation failure and reserved holes (`20260927T063005500199Z-81154.json`). The
firmware scratch space is bounded loader BSS, avoiding an unresolved stack-probe
runtime dependency. The first retry still failed safely because the requested
span was unavailable. The experimental browser build now reuses the existing
streamed installer payload protocol, with target-specific manifests and binary
reassembly checks for both ESP and installed kernel. This reduced the ARM live
ELF from 2.1 GiB to 823 MiB and brought its load span below 4 GiB. Build manifest
`20260927T064220906014Z-82461.json` passed. The actual QEMU installer then booted
and completed disk provisioning. The disposable installation cold-booted without
ISO media, completed onboarding, preserved its identity across another reboot,
and launched the native browser worker with zero reported failure
(`20260927T064832537858Z-82458.json`). A settled rerun
(`20260927T065606170203Z-83428.json`) produced six engine frames; manual review of
`build/browser-installed-1790491712614395000/node-1/browser-launch.png` confirms
the real HTTPS Example Domain page rendered with text, CSS and its link inside
the native shell. The initial two frames were only `about:blank` and were not
accepted as page proof. The harness uses a QEMU-addressed installed-kernel
replacement, so this is not unmodified release-ISO parity. JS, interactive
navigation, downloads and performance acceptance remain open. The zero PEAK
counter is not a RAM measurement: that event is emitted at engine teardown.

Installed pointer QA found that QEMU's USB tablet did not deliver movement in
this guest. The harness now uses the existing ARM USB-mouse configuration and
requires observed coordinates and button transitions. Manual captures showed
the real link loading IANA's HTTPS 301 body and Back restoring Example Domain.
That exposed missing redirect metadata in the interception path; redirect
acceptance is not passed. The later pointer wait timed out, so Forward/Reload
are also not accepted (`20260927T070333756165Z-83590.json`). These captures are
diagnostic evidence, not a passing complete interaction test.

1. Connect the native shell and retained compositor to the engine worker.
2. Connect governed desktop networking to the resource callback boundary.
3. Verify responsiveness, input, resize, cancellation and window lifecycle in
   the actual desktop; measure launch/load/first-paint, CPU and peak RAM.
4. Commit downloads into native Object/Namespace storage with metadata.
5. Register and package the default browser for live and installed generations.
6. Cold-install, detach the ISO, and prove HTTPS, CSS/image, JS, link navigation,
   input, scrolling and downloads. Verify both architecture targets separately.

## Evidence already obtained

Native address-field editing, caret scrolling/blinking, Go/history/reload and
window-control hit routing are now implemented behind `native-browser`. Both
kernel targets compile (`20260927T055139547238Z-78802.json`), but desktop interaction
and visual matching remain unverified. Downloads and menu actions remain open.
The rebuilt engine guest passed (`20260927T055217855046Z-78858.json`), including
explicit loading transitions for initial navigation and history traversal,
paired keyboard delivery, JS pixels, resize and shutdown. This is not an
installed-OS test and does not update the installer ISO.

Pointer movement and three-button capture now route to viewport-local Servo input.
Normal FIFO producers reserve three entries for releases; a captured release is
routed before desktop overlays even outside the viewport. The bounded-pressure
behavior passes core tests (`20260927T055854548204Z-79088.json`, 15 tests), and
both kernel targets compile (`20260927T055946758454Z-79148.json`). A fixture-only
run against the previously rebuilt engine (`20260927T060024309498Z-79214.json`)
requires a real DOM click at (40,40) plus paired key events before accepting the
expected page pixels. It passed, with a component allocator peak of 150,826,624
bytes. This is not total system RAM, a desktop performance measurement, or
installed pointer/drag verification.

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
| `20260927T042213768189Z-48699.json` | Governed browser-network queue rejects missing authority, forbidden URLs, queue overflow and stale cancellation; existing HTTPS actor lifecycle remains passing | Host service harness |
| `20260927T042251408923Z-48745.json` | Exact binary-body preservation and HTTP header handoff without duplicate transfer decoding | Focused host behavioral test |
| `20260927T042330025765Z-48799.json` | Shared kernel including BSP browser-network pump compiles | AArch64 installed configuration, compile only |
| `20260927T042402000298Z-48919.json` | Same shared adapter compiles | x86_64 installer configuration, compile only |
| `20260927T043602153977Z-59009.json` | Native browser supervisor and bounded ABI callbacks compile | AArch64 installed configuration with native-browser, compile only |
| `20260927T043850445456Z-63400.json` | Same supervisor compiles | x86_64 installer configuration with native-browser, compile only |
| `20260927T044127660539Z-67839.json` | Real Servo, SpiderMonkey and software renderer code generation succeeds with hardware-float ABI and matching LLVM intrinsic headers | x86_64 native component compilation, not execution |
| `20260927T044841231205Z-72829.json` | Fatal cleanup invalidates both active and queued network handles and allows bridge reconfiguration after release | Host HTTPS actor behavioral harness |
| `20260927T045144799929Z-72990.json` | Isolated x86 Servo component links with no unresolved required symbols | Native linkage only, not execution |
| `20260927T045309082355Z-73038.json` | AArch64 component regression with updated C syscall adapter: four real pixel gates, keyboard JS, resize, close/reopen, two released requests and clean shutdown | Freestanding guest, not installed OS |
| `20260927T045735758478Z-73265.json` | Servo and existing speech providers link into the production installed-kernel configuration | AArch64 linkage, not boot proof |
| `20260927T045922151909Z-74042.json` | Same production installed-kernel linkage | x86_64 linkage, not boot proof |
| `20260927T050142723257Z-74529.json` | Binary ELF inspection verifies target architecture, executable entry, nonoverlapping segments and no writable-executable load segments | Artifact structure, not runtime security proof |
| `20260927T050741867645Z-75613.json` | Browser stack slot raises, hides and hit-tests with existing windows; five-window callers keep the extra slot hidden | Shared native window-stack behavioral test, not browser UI proof |
| `20260927T052146575941Z-76540.json` | Real Servo second navigation, exact Back/Forward destinations and history flags, fragment-free provider request, released request handles, four prior pixel/input gates and clean shutdown | AArch64 freestanding component guest, not installed desktop |
| `20260927T053040513878Z-77044.json` | Approved PNG artwork converted to native BMP with every BGRA row and alpha byte preserved | Binary artwork verification, not screenshot review |
| `20260927T053403252290Z-77187.json` | Native retained browser renderer, independent window state and metadata drain compile | AArch64 compile only |
| `20260927T053513507015Z-77289.json` | Same shared renderer and window integration compile | x86_64 installer compile only |
| `20260927T053436488285Z-77233.json` | Existing editor/window stack behavioral regressions pass | Host harness |
| `20260927T053555084820Z-77341.json` | Six-window cycling includes browser, wraps correctly and handles an empty window set | Host window-workflow behavior |
| `20260927T054039406345Z-77993.json` | Authenticated console launch, queued lifecycle/resize and browser-specific repaint scheduling compile | AArch64 compile only |
| `20260927T054109765351Z-78034.json` | Same launch and repaint path compiles | x86_64 installer compile only |
| `20260927T054618595998Z-78436.json` | 14 core tests pass, including atomic gesture admission, backpressure retry, FIFO wrap and lifetime clearing | Host behavioral harness |
| `20260927T054633897630Z-78487.json` | Native keyboard/wheel controller compiles for both kernel targets | Compilation, not installed input proof |
| `20260927T054725789432Z-78533.json` | Production input FIFO delivers a key press/release pair into real Servo; JavaScript requires both before changing every pixel; history/lifecycle regressions remain passing | AArch64 component guest using unchanged cached engine; not installed OS |

The history test exposed fragment-bearing URLs crossing the native HTTP boundary.
The adapter now removes fragments only from provider requests, preserving the
original document URL for Servo history and anchors. Native controls receive
engine-derived Back/Forward availability; disabled traversal leaves requests
alone. The expanded fixture peaked at 150,009,600 allocator-reserved bytes.
`--reuse-engine` is an explicit fixture-only debugging mode; the passing manifest
above rebuilt the engine and did not use it.

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

## Desktop network bridge

`kernel/drivers/browser_network.rs` now bridges a single native engine owner to
the BSP HTTPS actor using sixteen bounded slots. It is polled from the real OS
network pump. The caller supplies existing capabilities; the bridge grants none.
Requests retain generation tickets, so stale browser cancellation cannot cancel
a newer transaction belonging to the same user. Queues, response bodies (128 KiB)
and headers (8 KiB) are bounded. Existing `geturl` retains its 8 KiB body limit.
Replies remain borrowed until cancellation and are never read while the BSP is
writing them. Unsupported schemes and non-default TLS ports fail closed.

The bridge still needs the production browser supervisor to configure and call
it. Its service tests do not establish installed page loading or download proof.

## Supervisor integration in progress

`kernel/runtime/browser.rs` is feature-gated by `native-browser` until production
linkage and desktop activation are verified. It leases one existing background
worker, supplies an isolated heap, derives a worker-owned cryptographic RNG from
firmware entropy, and advances verified UTC using the monotonic clock. Servo
input, metadata and pixels cross bounded queues. Network callbacks use the BSP
bridge rather than entering runtime services from the engine CPU. Fatal engine
failure cancels outstanding requests and quarantines that worker; restart after
fatal failure is not yet supported. The idle hook currently spins, so idle CPU
cost remains an explicit measurement and correction item.

The supervisor is not enabled in release images yet. No shell, installed launch,
responsiveness or total-RAM acceptance follows from these compilation checks.
The in-progress native renderer now has a separate retained slot (12), approved
losslessly converted artwork, bounded viewport pixel copying and engine-driven
metadata. Browser geometry has its own console state and stack identity (5).
The painter retains a borrowed completed frame while the worker can publish into
the other slot; it never invokes Servo. This is not yet an interactive shell:
launch/permission flow, input routing, resize commands, repaint scheduling and
rendered screenshot review remain unverified. The new window starts hidden and
has no launcher entry until that integration is ready. Browser session-layout
persistence and minimized-shelf representation also remain open.

The experimental `native-browser` build now accepts `browser https://example.com/`
from an authenticated Console session with existing connect/send/receive/resolve
capabilities. It grants none; the current diagnostic permission path remains
`https authorize`. The controller queues OPEN then NAVIGATE, retries mailbox
backpressure, synchronizes the physical viewport and retains CLOSE until accepted.
The desktop watches browser revisions separately and includes the browser bounds
in bounded scene damage. Neither this command path nor its repaint behavior has
been exercised in an installed guest yet. Toolbar/page input and native user-facing
permission UX remain required before release. A subsequent small correction retains
the last submitted viewport across repeated navigation so resize is not suppressed.

Native text/editing keys and wheel deltas now enter a bounded BSP input FIFO.
Key press/release pairs are admitted atomically. A full queue rejects the whole
gesture with a visible busy state; it does not report delivery. Admitted releases
drain before a subsequent navigation. Wheel coordinates are clipped to the content
viewport. The real component fixture confirms paired delivery and resulting JS
pixels, but the desktop controller still requires installed interaction testing.
Address editing, pointer buttons/selection, toolbar action wiring and permission
UX remain incomplete. The browser remains excluded from release images.
The kernel boot path retains its loader-owned BootInfo for later authenticated
launch; it does not launch Servo automatically. Start and command submission are
bound to one nonzero session owner. Cross-session restart/teardown remains an
integration gate. Boot wiring compiles on ARM (`20260927T050259018212Z-74615.json`)
and x86 (`20260927T050356924058Z-75003.json`); this is not a runtime ownership test.

The x86 engine needs a separate hardware-float native target: Rust's built-in
bare-metal target uses softfloat, unlike the C/C++ SysV ABI used by Servo. The
custom target keeps `target_os=none` and enables SSE2 without introducing Linux
semantics. Upstream Unix sandbox dependencies are excluded on native targets;
unsupported multiprocess requests fail closed. Engine code generation now passes
on x86, as does component linkage. Execution remains a separate gate. Both
architectures use the same C policy adapters; their Newlib syscall symbol
spellings differ, and read/write return types follow each target's headers.

`tools/servo-platform-probe/link-kernel.py` keeps experimental kernels separate
from release images. Production-kernel linkage succeeds for both targets. Actual
load segments contain 590,608,512 file bytes / 2,336,586,224 memory bytes on ARM,
and 635,897,464 file bytes / 2,381,880,744 memory bytes on x86. Those totals include
the whole kernel and existing providers, not just browser RAM or a runtime peak.
ELFs retain debug data and are roughly 2 GB on disk; release packaging and cold
boot remain unverified, and no installer ISO was updated by these link probes.
