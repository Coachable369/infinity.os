# Yahoo resource loading and modern page compatibility

The 2026-11-06 native baseline received Yahoo's document and ten stylesheets
successfully. During its 180-second emulated observation window, the serialized
transport continued dispatching JavaScript without reaching fonts or images.
This establishes resource starvation; it does not establish packet loss.

Changes:

- Replace parking_lot's bare-metal spin-only parker with the browser executor's
  existing atomic compare/park/wake service. A stalled native Yahoo run was
  sampled repeatedly in `parking_lot::RawMutex::lock_slow`; its waiter could
  monopolize the cooperative executor and prevent the lock owner from running.
  This adapter is selected only in the browser dependency overlay on native
  targets. The kernel and direct audio pipeline are unchanged.
- Keep document/CSS precedence and active-transfer ownership. After three
  script/API dispatches, service a queued visual resource. Alternate fonts and
  images when both are waiting; retain FIFO within each category.
- Use one shared browser preference constructor in production and native
  fixtures. Enable implemented IntersectionObserver, FontFace and variable
  fonts. Device APIs and transport authorization remain
  unchanged.
- Optimize the browser's Rust engine and adapter at level 2. The previous
  native component linked unoptimized development archives. Preserve development
  safety checks and established archive paths; omit debug information.
- Capture actual document readiness, font readiness, image decode counts and
  viewport/content dimensions in native public-site diagnostics.

Verification entry points:

```
./build-kit browser-core-tests
./build-kit browser-modern-web-tests
```

The modern-web fixture runs actual native Servo: an intersection callback starts
an image load, decoded image dimensions are checked, a FontFace loads from the
packaged catalog, and variable font axes change measured glyph widths.
It also forces real parking_lot mutex contention, owner handoff, and timed-lock
expiry on native executor threads.
It requires prepared native engine archives.

CSS container queries remain a known engine gap. The pinned Servo implementation
of `ServoDangerousStyleElement::query_container_size` is `todo!()`. Enabling the
experimental preference is not an implementation and must not be shipped as a
fix. Yahoo uses container queries; full responsive-layout parity remains an
acceptance item independent from font and image loading.

Public-site evidence is under `builds/evidence/yahoo-20261106-*`. A received HTTP
200, successful compilation, or a styled header alone is not full-page acceptance.
The direct AI audio pipeline is outside this change.

## Verification outcome

- Browser core: 64 behavioral tests pass; manifest
  `20261106T191745231995Z-73599.json`.
- Native ARM fixture: mutex contention/handoff/timeout, FontFace, variable font
  metrics and IntersectionObserver-triggered decoding pass; manifest
  `20261106T193637158011Z-74571.json`.
- Live Yahoo at 1536x864 in disposable ARM TCG: acceptance fails after 180 seconds.
  Five of 38 image elements decoded, zero reported broken images, document
  remained interactive; screenshot still contains overlapping content.
  Manifest `20261106T193743942200Z-74631.json`, evidence
  `builds/evidence/yahoo-20261106-native-parking-tcg/`.
  The previous lock stall did not recur in this run.
- Maximum observed engine/network/paint slices were 49.311/174.701/681.691 ms.
  Earlier unoptimized 800x600 baseline paint maximum was 3095.733 ms.
  These different-viewport emulation samples are diagnostic, not an installed
  VM performance guarantee or a controlled whole-page speedup measurement.
- A native virtualization fixture attempt could not obtain hardware entropy;
  its unsupported acceleration option was removed. No entropy fallback was added.

Remaining acceptance: finish Yahoo loading, implement native CSS container sizing
and its layout/restyle lifecycle, and verify responsive card placement and
images in the installed OS. Do not enable the unfinished container-query flag.
After the user requested a fast incremental ARM test ISO, `./build-kit arm-iso`
completed successfully. Local artifact: `builds/InfinityOS-aarch64.iso`.
SHA-256: `fe87ae7b722739333a991c346b3c80267ddb8ebfc2e1cdf6a3d16f70cbd3d14f`.
Build manifest: `builds/manifests/20261106T210721222036Z-75926.json`.
Kernel freshness, live/installed UI and SDK/model payload parity, and checksum
checks passed. Broad regression tests were deferred. This packages the verified
improvements; the Yahoo acceptance failures above remain unresolved. No x86 ISO
was rebuilt and no installed VM was restarted or validated in this packaging run.

## Follow-up: full-page acceptance remains failed (2026-11-07 UTC)

The user's installed screenshot still shows missing images and overlapping carousel
content. The previously published ARM image is not accepted as a Yahoo fix.

A disposable native guest observed Yahoo for 300 seconds: readyState remained
interactive and only 5 of 38 image elements had decoded images. Evidence:
`builds/evidence/yahoo-console-20261107/result.json` and `page.png`, manifest
`builds/manifests/20261107T005802762015Z-85444.json`. This is not installed-VM proof.
A separate 90-second font trace also failed full-page acceptance, manifest
`builds/manifests/20261107T011325945663Z-87332.json`.

Added opt-in engine warning/font/cache diagnostics and page console capture to
the disposable guest. These do not change the production audio pipeline.
Expanded the native modern-web fixture to check WOFF2 decoding, streamed cache
body delivery/request counts, concurrent cache consumers, and a preloaded network
font consumed by FontFace. The final fixture passed through build-kit:
`builds/manifests/20261107T011801318885Z-88204.json`.

The cache already exists in Servo beneath native transport interception. Both
completed and streamed cache tests pass; adding another cache is not justified.
Yahoo's live run fetched 129 resources (88 unique URLs) in the 300-second trace.
Native transport serializes requests. Remote font downloads succeed but the main
font remains unapplied during the observed page load. These observations do not
prove packet loss or a single root cause for the layout failure.

The pinned engine also lacks layout container-query evaluation. Enabling its
preference alone would execute an unimplemented method and is not a fix. Isolate
layout failures with reduced behavioral fixtures before implementing that support.

The old acceptance expression used document.fonts.status, which is not exposed
by this pinned engine. Follow-up runs instead check document readiness and decoded
image counts; they still fail. No replacement ISO was published for these
diagnostic-only changes, and Yahoo/browser completion must not be claimed.

## Concurrent fetch correction and native browser capacity (2026-11-07 UTC)

The earlier cache test started its second consumer after response headers arrived.
A new regression starts both consumers before headers. It reproduced two native
requests for one cacheable URL while delivering correct bodies to both consumers:
`20261107T020102008539Z-90901.json` (expected failure).

The native engine overlay now retains a new cache entry's producer guard until
headers arrive. Waiting consumers then use the existing streamed cache entry;
this does not introduce another cache or bypass HTTP policy. The same test passes
with one native request: `20261107T020431369519Z-91246.json`.
The subsequent 300-second Yahoo trace attempted 150 requests for 143 distinct
URLs; repeated analytics or denied ad endpoints are counted in that total. It
rendered the page font, hero photos, navigation and
thumbnails. Full-page acceptance still failed in that run; see
`builds/evidence/yahoo-cache-producer-20261107/`.

Additional bounded browser changes:

- Reserve network/input service time after a paint exceeds its frame budget;
  preserve the cadence of fast paints. Browser core: 65 tests pass,
  `20261107T021745445845Z-91863.json`.
- Increase the private browser arena from 512 MiB to 1 GiB after the extended
  native run exhausted the former limit while creating another script thread.
  Keep the same shared grant limit in host and component. Move only the x86
  browser arena past the existing display allocation to avoid overlap.
- Enable the engine's Web Crypto feature. Yahoo's analytics script previously
  raised `crypto is not defined`. Native tests verify random buffers, UUID
  structure, the SHA-256 known-answer digest and an AES-GCM round trip.
  Manifest: `20261107T021549668975Z-91749.json`.
- Exercise duplicate font preloads, stylesheet glyph widths, carousel item
  geometry and wrapping within a constrained flex row. The final combined
  fixture passes: `20261107T021703568340Z-91819.json`.

CSS FontFace status is not a reliable acceptance oracle in this engine version:
stylesheet glyphs can render while that status remains `loading`. The regression
uses measured glyph widths instead. The direct AI audio pipeline is unchanged.

The subsequent full-viewport native run exited in SpiderMonkey's condition
variable constructor before it could deliver a final frame. The fatal caller in
manifest `20261107T021905304711Z-91934.json` resolves to the assertion on
pthread condition initialization. The Servo C runtime limited conditions to 128
irrespective of the larger mutex pool. Its separate condition budget now
permits 4096 without enlarging the reader/writer lock pool. The ARM runtime
test creates 512 live conditions, releases them, rejects a stale handle after
slot reuse and still exercises real condition waits and broadcasts:
`20261107T023010095400Z-92207.json`.
