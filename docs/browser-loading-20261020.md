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
  The allocator regression subsequently passed (8 runtime-primitives tests,
  manifest `20261020T142414295578Z-47935.json`), but the same Yahoo acceptance
  still failed. It is not a complete-page fix.
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

## Critical-resource follow-up

- Indexed-allocator Yahoo retest:
  `builds/evidence/yahoo-indexed-allocator/`,
  manifest `20261020T142514631788Z-48276.json`. Required JavaScript still
  disconnected/timed out; the document remained interactive rather than complete.
- The one-shot HTTP client incorrectly advertised keep-alive whenever browser
  headers were supplied. It now requests connection closure consistently.
  The separate persistent TLS channel is unchanged.
- HTTP suite before the additional EOF-body regression: 44 passed, manifest
  `20261020T143116909748Z-49719.json`. The independent TLS server checks the
  parsed connection field; this is protocol behavior, not a source-text oracle.
- Closure retest: `builds/evidence/yahoo-connection-close/`, manifest
  `20261020T143318420502Z-52351.json`. **Still failed** complete-page acceptance.
  New interval telemetry measured **21,513.78 ms between resource pumps**,
  versus maximum engine-call 114.566 ms, network-call 1,302.649 ms, and
  paint-call 2,633.386 ms. Individual-call timings had missed cooperative
  scheduling delays. These are disposable TCG probe measurements, not installed
  desktop timing.
- Added an allocation-boundary scheduling safepoint, checking elapsed time every
  64 allocate/free operations and yielding a worker after observing 1 ms.
  No allocator borrow survives its yield; root service work does not reenter.
  This is cooperative fairness, not preemption for allocation-free CPU loops.
- HTTP suite: **45 passed**, including authenticated EOF framing. Native std
  probes passed on **AArch64 and x86_64**, including the allocation-heavy worker
  fairness regression, TLS destruction, synchronization and allocation checks.
  Combined manifest: `20261020T144522673002Z-79325.json`. Its final exit is failure
  because the subsequent Yahoo acceptance remained incomplete.
- Same 180-second Yahoo observation:
  `builds/evidence/yahoo-allocation-safepoint/`. Maximum network-service gap fell
  from **21,513.78 ms to 3,002.827 ms** (86.0% lower). The previously failing CSS
  and JavaScript resource completed; additional script transfers proceeded.
  The document still had not completed loading, and the decoded-image gate
  remained false. This is measured starvation improvement, **not a general
  browsing acceptance pass**.
- The probe can now observe up to 600 seconds without changing production
  transport deadlines. Extended Yahoo observation and independent Wikipedia
  verification could not start: another full build owns the shared lock as
  `20261020T145014529831Z-80550`. An attempted release build was also blocked.

No installed-system or updated-ISO acceptance is claimed for this follow-up.
