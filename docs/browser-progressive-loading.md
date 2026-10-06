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

## Remaining Gates

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

Live-site acceptance, x86 runtime verification, refreshed ISO packaging and
detached fresh-installed runtime verification remain required. Existing ISOs
must not be presented as containing this change. Production networking still
uses a serialized bounded HTTPS transport; broad modern-site compatibility
and hardware-VM performance are not established by these fixtures.
