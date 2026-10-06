# Browser transport correction - 2026-10-06

This is a partial implementation receipt, not a claim that the reported sites
or fresh installations pass acceptance.

## Implemented

- Authenticated HTTP headers and payload chunks reach Servo before EOF. A
  bounded mailbox acknowledges each loan before its bytes can be overwritten.
- Servo uses its streaming fetch channel; native parser reflow can publish
  partial documents. The existing session frame pacer remains active.
- Standard GET, HEAD, POST, PUT, PATCH, DELETE and OPTIONS requests cross the
  native boundary. Bodies are limited to 64 KiB; the transport owns framing.
  CONNECT, TRACE, header injection and oversized requests remain rejected.
- Native request targets allow 8192 bytes and response headers allow 128
  fields within the existing 8192-byte wire-header budget.
- Queued documents and stylesheets precede scripts, fonts and media. Active
  transfers are not replaced; equal-priority requests retain FIFO order.
- Worker ABI version 5 carries methods/bodies and chunk acknowledgements.
  Both architectures use the same implementation and installer-linked component.

## Behavioral Evidence

- `20261006T165030756569Z-1675.json`: HTTP transport and HTTPS actor tests pass,
  including independent TLS delivery before server EOF, cooperative
  backpressure, exact binary POST payloads and native-owned Content-Length.
- `20261006T163946728415Z-730.json`: kernel mailbox ownership, chunk ordering,
  cancellation and completion tests pass.
- `20261006T165110479933Z-1993.json`: ARM native component builds and links.
- `20261006T165645230772Z-6867.json`: native pixels appear while the provider
  withholds the remaining HTML; later pixels and completion follow EOF.
  POST/307/303/form transitions also pass. The combined run subsequently
  exceeds its overall timeout during a later screenshot export, so the
  combined suite is not reported as passing.
- `20261006T165808178012Z-6928.json`: focused native binary POST, preserved
  307 body, 303 GET conversion and real HTML form submission pass.
- `20261006T170114304209Z-7148.json`: native cookie-dependent redirects,
  credential omission and cross-origin rejection pass.
- `20261006T171543732687Z-7735.json`: all 47 browser-core tests pass, including
  actual queue insertions preserving active work and equal-priority order.
- `20261006T171558550561Z-7767.json`: final combined native suite passes with
  a 90-second overall screenshot-export budget. Early pixels before EOF,
  later pixels/completion, POST/form redirects, cookies and all eight browser
  interaction stages pass. The retained selection screenshot was reviewed.
  This supersedes the earlier combined-suite timeout, not the live-site failures.
- `20261006T171711344379Z-7858.json`: final x86 component compiles and links.
  This is not x86 VM execution evidence.
- `20261006T172341409581Z-12698.json`: final ARM component, including the
  shared resource-order policy, links successfully.
- `20261006T172554361043Z-12804.json`: ARM and x86 installed-kernel libraries
  compile with the new ABI and transport. Existing warnings remain. This is
  not an installed boot, refreshed payload parity, or ISO-build result.

## Navigation Failure Recovery (October 6 Follow-Up)

The component previously dropped the entire tab group after a renderer/pixel
readback failure. The desktop retained its open-window state, so subsequent
navigation commands reached an empty session and were ignored. Renderer errors
now retain the tab group, release failed-tab network work, and continue servicing
healthy tabs. Explicit navigation or reload replaces the failed WebView while
preserving its tab identity and the other tabs. The failed tab's document state
and history cannot survive replacement; healthy tabs retain theirs. Error events
are coalesced rather than filling the worker mailbox every tick.

The native recovery fixture tests failure before headers, failure after a partial
body, timeout, renderer failure, navigation retry, reload retry, and preservation
of a healthy neighboring tab. Recovery requires matching engine location,
completion, and real rendered pixels. Transport cases enforce exact lease release.
The network cases also pass before the fix, so they are regression coverage, not
evidence that the original network path caused the reported lockout.

- `20261006T215951337369Z-19154.json`: baseline native transport recovery passes.
- `20261006T220451733150Z-19388.json`: final combined native suite passes,
  including transport and renderer recovery, reload, incremental pixels,
  request-body redirects, cookies, and browser interactions. Evidence is retained
  under `builds/evidence/browser-recovery/final/`; the selection image was reviewed.
- `20261006T220617437887Z-19450.json`: updated ARM browser component links.
- `20261006T220749063304Z-19914.json`: updated x86 browser component links.
- `20261006T220952282303Z-20012.json`: `./build-kit incremental` completes,
  rebuilding both full-bundle ISOs with live/installed kernel payload comparisons,
  model-bundle checks and browser artwork/font parity. This is an incremental
  build, not a clean-build or installed-runtime claim. ISO SHA-256 values:
  - ARM: `f98f837947303355a9e3d31340ece0c31857abc757e5683b535c64c993c36aa3`
  - x86: `5bf3268f9bb5653f7e83538997e67e7e85d1aa4f9ddd8820e573c50c013f4892`
- `20261006T230454991060Z-40711.json`: cold x86 full-ISO install attempt
  times out at live startup (no runtime snapshot) before installation or browser
  execution. This is not an x86 browser pass.
- `20261006T231033831235Z-41110.json`: native ARM/HVF QEMU-media cold install
  passes exact installed-kernel comparison and media-detached boot. Real browser
  pixels, favorite add/remove/navigation, detached reboot persistence, failed
  HTTPS destination state and subsequent successful navigation all pass. Receipt:
  `build/browser-installed-1791328233934142000/result.json`. This tests the QEMU
  ARM media variant, not the VirtualBox hardware layout of the distributable ISO.

Yahoo compatibility and the user's installed-system reproduction are separate
acceptance gates; fault injection and the controlled installed HTTPS failure do
not establish the cause of Yahoo's failure.

## Remaining Site And Installation Gates

DuckDuckGo search still fails the result-DOM assertion. The 120-second TCG
diagnostic (`20261006T170227314247Z-7262.json`) receives 6,294,168 bytes but
remains in the loading state with no result links. A 60-second trace records
a failed font resolution despite valid DNS replies in the packet capture;
that diagnostic alone does not establish the cause. The separate search
endpoint root returns HTTP 200 with an empty body, which is not search proof.

The probe now continues network pumping during DOM callbacks, retains per-run
screenshots/reports, and supports an explicit bounded emulation load budget.
These are diagnostic improvements, not acceptance relaxations: real result
elements and nonuniform rendered pixels are still required.

Google search reaches its HTTP 429 challenge page rather than results
(`20261006T170906321192Z-7565.json`). Its redirect and challenge resources are
transferred, but neither CAPTCHA completion nor post-challenge results are
verified. No challenge or rate-limit bypass is implemented.

Yahoo still fails its DOM/pixel gate after receiving 1,325,095 bytes
(`20261006T171339414626Z-7670.json`). The priority trace confirms stylesheets
are fetched before the script backlog, but the document remains loading.
Receiving HTTP 200 is not counted as a rendering pass.

Live-site acceptance, x86 runtime verification and the ARM VirtualBox
distributable's detached installed path remain required. The ARM QEMU installed
path and refreshed ISO packaging are verified above. Production networking still
uses a serialized bounded HTTPS transport; broad modern-site compatibility
and hardware-VM performance are not established by these fixtures.
