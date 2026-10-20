# Browser loading investigation — 2026-10-20

Status: **not accepted as a general browsing or CAPTCHA-completion fix**.

## Reproduction

Native AArch64 Servo guest, real DNS/TCP/TLS, QEMU TCG, 800×600 viewport.
This is not installed-system verification.

| Probe | Observable outcome |
| --- | --- |
| Google `search?q=hello+world` | Initial HTTP 200 navigated to Google's HTTP 429 unusual-traffic challenge. Challenge JavaScript fetched; search-results DOM assertion failed. |
| Yahoo, original scheduling | HTML and scripts fetched; after the bounded observation window the DOM had no body and no stylesheets. Visible-page acceptance failed. |
| Yahoo, stylesheet priority | Nine stylesheets became available, but the body was still absent. Visible-page acceptance still failed. |
| Wikipedia | Link/style DOM condition succeeded, but the rendered-pixel gate failed while stylesheet loading continued. Not a successful page-render claim. |

Raw structured reports and pixel captures:

- `builds/evidence/google-results-current/`
- `builds/evidence/yahoo-current/`
- `builds/evidence/yahoo-priority/`
- `builds/evidence/wikipedia-current/`

## Targeted changes under verification

- Prioritize render-blocking stylesheets and embedded documents over queued
  scripts, retaining active-transfer ownership and equal-priority FIFO ordering.
- Drain immediately ready resource events in batches of at most eight, checking
  elapsed time after each event against a 2 ms budget. Pending I/O yields
  immediately. This is a cooperative budget, not preemption of a single event.
- Observe the requested DOM condition across page-load transitions, rather than
  ending on the first completed document before a script-triggered navigation.
- Add behavioral coverage for priority ordering and bounded ready-event
  delivery, including single release on completion.

## Remaining acceptance

Successful visible rendering of independent modern sites; user-completed
CAPTCHA followed by real search results; installed ARM64 and x86_64 acceptance;
updated installer packaging. No CAPTCHA bypass, identity spoofing, relaxed TLS,
or same-origin exceptions were introduced.

## Verification of the targeted changes

- Native progressive-streaming and gzip fixture: passed, including the new
  eight-event ready-budget assertion and exact single release.
  Manifest: `builds/manifests/20261020T042633581768Z-79556.json`.
- Queue ordering behavioral tests: 2 passed.
- Google with the corrected observation window and both changes: the screenshot
  visibly contains the challenge checkbox. Its frame and stylesheet returned
  HTTP 200. The actual search-results assertion still failed; no challenge was
  solved or bypassed.
  Evidence: `builds/evidence/google-results-final/page.png` and `result.json`;
  manifest `builds/manifests/20261020T042949097653Z-80580.json`.
- Yahoo after both changes: still failed visible-page acceptance.
  Evidence: `builds/evidence/yahoo-batched/result.json`;
  manifest `builds/manifests/20261020T042718657445Z-79649.json`.
- Yahoo with a 180-second observation window reached an interactive DOM and
  rendered styled navigation and loading placeholders, but required script
  transfers disconnected/timed out, images/content were incomplete, and the
  overall guest timed out during termination. This is **partial rendering,
  not successful browsing**.
  Evidence: `builds/evidence/yahoo-extended/`;
  manifest `builds/manifests/20261020T043201471915Z-81027.json`.

The external-site probe currently links its network dependency with Cargo's
development profile and runs under QEMU TCG. Its elapsed times must not be
reported as installed-system performance measurements.

## Continued investigation: transport pressure and allocator stalls

- Fixed a confirmed receive-queue bug: the client previously consumed upstream
  packets even when `EthernetQueue` had no room, silently discarding them.
  It now checks capacity before consuming. A behavioral burst test verifies
  retention and exact FIFO payloads across repeated blocked pumps.
- HTTP suite: **44 tests passed**, including the new regression.
  Manifest: `builds/manifests/20261020T071234341135Z-23572.json`.
- Yahoo still failed the stricter complete-document/decoded-image condition
  after this fix. Evidence: `builds/evidence/yahoo-backpressure/`;
  manifest `builds/manifests/20261020T071340426937Z-24918.json`.
  Required JavaScript still disconnected/timed out; this is not a complete fix.
- Added structured maximum work timings to the external-page probe.
  `builds/evidence/yahoo-work-profile/` recorded engine 33.146 ms,
  network 1192.817 ms, paint 41.219 ms. These measurements exclude other
  cooperative tasks run while sleeping and are **not installed performance**.
  Manifest: `builds/manifests/20261020T072013059302Z-29642.json`.
- Repeated QMP CPU samples during the stall resolved to native arena
  `remove`/`next` free-list traversal. The large-heap allocator now uses indexed
  free-order membership plus doubly linked free lists instead of linear buddy
  searches. Metadata stays inside the granted heap and consumes about 6.25%
  of large grants. Small grants retain their original capacity.
  This is a **candidate stall fix, not yet remeasured**.
- Added a >100,000-block fragmentation/reclamation/guard-byte regression and
  made runtime-primitives tests a prerequisite of both browser component builds.
  The allocator regression and native-page retest remain pending: another
  task's `./build-kit full` acquired the shared build lock as
  `20261020T072349579720Z-29748`.
- A hardware-accelerated disposable probe failed before browser initialization;
  no compatibility claim is based on that attempt.

Do not label the current ISOs as containing a verified complete-page fix.
Next gate: runtime-primitives tests, same Yahoo retest, additional independent
sites, then installed-system verification and release parity/checksums.
