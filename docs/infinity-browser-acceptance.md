# Infinity Browser v0.1 acceptance ledger

This is an implementation ledger, not an installed-browser completion claim.
Servo is pinned to `d05154e2b4def11a9fefe412898a0a6c8925a9cd` (0.6).

## Installer handoff — 2026-09-27

The user requested updated ISOs followed by manual feature verification, with
work stopping at that handoff. Clean run `20260927T193335871261Z-71378.json`
failed because a later kernel rebuild left the earlier x86 installer FAT payload
stale. `build.sh` now refreshes both architectures' actual test media before the
binary parity gate and packages both model-inclusive release ISOs at the end.
The resumed run passed that gate for ARM (864,610,952 installed kernel bytes)
and x86 (862,397,304 bytes), including exact bootloader parity. All 28 browser-core
tests passed, including explicit URL association. The continuation
`20260927T203007149973Z-60677.json` produced both final ISOs, then stopped at an
overbroad font assertion: 42/56px atlases are unreachable at the desktop's 1x/2x
scale and are dead-stripped. The check now requires the used 14/28px faces.
Final verification `20260927T212119387180Z-83067.json` passed exact font/artwork
and manifest packaging checks, extracted nonempty boot payloads from all five
ISOs, generated `builds/SHA256SUMS`, and passed the VirtualBox input guard.

Final manual-test installers (2026-09-27 CDT):

- `builds/InfinityOS-aarch64.iso`: 8,590,338,048 bytes, 16:19:16;
  SHA-256 `15ffb2d3f46bdd78f07bd6ca47f0dac35230679a33f02e37b0d837bb0b9bbab0`.
- `builds/InfinityOS-x86_64.iso`: 8,590,346,240 bytes, 16:13:19;
  SHA-256 `0ce6bd08212c38181f8da5814373587388935d9b8ddf33b57e1ccc606a201c98`.

This is a verified artifact handoff after a failed clean run and incremental
continuation, not a successful clean-build or new installed-system acceptance
claim. Per the user's request, further feature/VM testing stops here for manual
verification. The optional HVF and Console-open installed harness paths remain
unverified.

## Current six-item checklist

| Item | Current evidence / remaining gate |
| --- | --- |
| Native Servo shell | ARM installed rendering, native address input, page input, JavaScript, navigation and lifecycle pass. |
| Governed live networking | Real HTTPS, explicit session consent and expired-certificate rejection pass. |
| Responsive execution | Dedicated engine worker; visible cursor samples during loading 32–337 ms in ARM TCG. Hardware-VM timing is not measured. |
| Native downloads/storage | Real small attachment persisted as an Object/Namespace file with metadata; no host download path. |
| Installed default registration | Catalog and minimized shelf work on an unmodified fresh ARM QEMU install. Focused x86 model-inclusive ISO build passed; current normal ARM media and clean-release validation remain open. |
| Cold-installed acceptance | ARM QEMU ISO bytes, detached boot, onboarding, reboot, launcher and address navigation pass. x86 installed proof and final build regression pass remain open. |

Clean-build attempts exposed two packaging-check assumptions, now corrected:
Make include resolution from a fixture directory, and a speech test requiring
contiguous rather than streamed installed kernels. Focused checks pass in
`20260927T130251926000Z-25336.json` and
`20260927T132322786706Z-68809.json`. The latter verifies 366 speech resources,
194,770,124 resource bytes and three constructor entries in each x86 kernel,
plus exact kernel shards from installer FAT media. The clean attempts remain
failed records; the current continuation is incremental, not a clean-release
success claim.

## Latest installed interaction evidence — 2026-09-27

The current tab-enabled ARM installation also passes HTTPS link/back/forward/
reload verification (`20260927T165600056847Z-59816.json`). Each action checks
the resulting URL hash, completed-load revision and visible page pixels; no
engine or network errors occurred. This reused the previously cold-installed
disk without replacing its kernel. Evidence is retained as
`builds/evidence/browser-daily-driver/installed-navigation-aarch64.json` and
`installed-navigation-aarch64.png`.

Focused x86 build `20260927T163105799603Z-44120.json` produced both bootstrap
test media and the model-inclusive `builds/InfinityOS-x86_64.iso`. It is not a
clean full-release result. The x86 installation finished and booted into
media-detached onboarding in `20260927T170120540479Z-60316.json`, but its artifact
comparison failed: the later model build had overwritten the loose kernel
reference. The harness now extracts its live kernel and ordered installed-kernel
shards from the tested ISO itself. The same untouched disk matches all
868,555,928 installed kernel bytes, SHA-256
`1988a777eb8f765389b64e5d3b8e77c9bb504e4dae193b93d16833890607aa3c`, with 11
component references checked. Resume run `20260927T172849910136Z-61049.json`
completed onboarding and another media-detached authenticated reboot, then
launched the real engine and rendered HTTPS CSS. Two native HTTP transactions
completed with status 200, with no recorded network or engine failure. However,
the image region contained only one color rather than the required image.
JavaScript, scrolling and tabs were not reached in that run. This remains a
failed browser acceptance result, not proof of full x86 interaction support.
Screenshot: `builds/evidence/browser-daily-driver/installed-x86-image-failure.png`.
The 30-minute TCG timeout changes no integrity checks.

## Current implementation batch and failure queue

- Clean release build started from `e780373` using `./build-kit full` on
  27 September. Completion and refreshed media are pending, not inferred from
  successful experimental kernel links. The artwork parity gate now also
  checks exact Inter atlas/metric/kerning bytes in both installed kernels.
- Hardware timing: the host ARM QEMU binary exposes HVF. The installed runner
  now accepts `--accel hvf` for ARM and records its accelerator in the receipt;
  actual hardware-accelerated execution remains unverified. TCG stays default.
- Default-browser scope: installed catalog launch is proven. Console `open`
  now routes explicit HTTP/HTTPS URLs to Infinity Browser through its existing
  consent boundary; local objects and privileged schemes are not turned into
  searches. Typed association tests and installed `--open-url` acceptance were
  added. Execution of these new checks remains pending, not covered by catalog
  availability alone.

- Revised browser kit: shared 48px tab/window row, native Inter atlases at four
  scales, 36px beveled tabs, bounded exterior glow, 8px tab gaps, 24px close
  targets, trailing add-tab control and aligned compact address toolbar.
  ARM installed interaction passes in `20260927T191738314582Z-68105.json`.
  Final paint-only review, both native links and 27 behavioral core tests pass
  in `20260927T192510242805Z-69676.json`. Screenshot:
  `docs/design/infinity-browser/installed-kit-chrome-v2.png`.
  Fonts are embedded in both installed kernels. Updated release media and
  final x86 installed visual verification remain open, not implied by links.

- 27 September x86 follow-up: the reused-install image/JavaScript checks passed
  in `20260927T174428672056Z-62250.json`; scroll and second-tab checks failed.
  Inspection found the scroll pixel probe directly under the software cursor,
  fixed 900px content unsuitable for the 2048px firmware viewport, and a second
  navigation attempted after the ten-minute lease during slow emulated typing.
  The fixture now uses viewport-height sections, samples away from the cursor,
  and uses a short real second-page URL. No network policy or pixel assertion
  is relaxed. These corrections are not a passing scrolling result by themselves.
- Updated x86 kernel link succeeded, but batch
  `20260927T183420477372Z-65179.json` failed before Servo launch: rapid Console
  input lost a portion of its command at 260 bytes. x86 verification now uses
  individually acknowledged keys. The disposable updated disk is reused;
  original cold-install disk and provenance remain untouched. Individually
  acknowledged input succeeded in `20260927T184109832219Z-66103.json`.
  HTTPS image and JS input passed; scrolling still failed and the dependent
  tab scroll-preservation assertion consequently failed.
- PS/2 wheel decoding inverted down into up, unlike the native positive-down
  convention. The decoder now preserves the PS/2 sign and handles IntelliMouse's
  full signed byte separately from Explorer's signed nibble. Behavioral packet
  and USB/PS2 direction-parity tests pass, together with native x86 linkage,
  in `20260927T185514949275Z-66522.json`. Installed run
  `20260927T185647729669Z-66589.json` passes real HTTPS image/CSS, keyboard JS
  mutation, scrolling, native tab create/select/close, background scroll
  preservation and Ctrl+L replacement. Receipt:
  `builds/evidence/browser-daily-driver/x86-wheel-corrected-interactions.json`.
  This is an updated disposable installation, not unchanged-release ISO proof.

- Browser random seeds previously repeated after engine restart within one boot.
  `browser_entropy.rs` now derives owner/launch-separated HKDF seeds from validated
  boot entropy, with a nonwrapping boot-lifetime counter. Two behavioral tests
  passed (`20260927T172648405631Z-60967.json`). Kernel rebuild and installed
  verification of this change are pending; it is not in the current ISOs.
- Resolved: x86 HTTPS image check now waits for decoded image pixels. Both
  subsequent batches passed the unchanged distinct-color assertion.
- The interaction runner now records independent image, JavaScript and scrolling
  failures before returning failure. Infrastructure failures still abort safely.
  Preserve already-passing install phases and reuse their disk/provenance.
- Timing coordinates now follow the observed framebuffer dimensions instead of
  assuming 1024x768. This correction still needs the consolidated timing pass.

Workflow: finish the implementation batch, run consolidated acceptance, record
all surfaced failures, fix those failures together, then rerun affected checks
and necessary regressions. Passing installation steps are not repeated unless
packaging or boot behavior changed.

Earlier x86 cold-install verification exposed low-memory pressure before kernel
entry: firmware's maximum-address allocation still staged the large ELF below
3 GiB, overlapping its fixed load reservation. The loader now explicitly selects
conventional high RAM from the firmware map. The separate native runtime pool
also uses that allocator, bounded by the existing 64-GiB identity map. A
diagnostic firmware boot reaches kernel entry; this is not yet installed-browser
proof. Allocation tests cover high-RAM selection, reserved-region exclusion,
size overflow, failure and matching page release. The full build also found a
stale 1-GiB production-payload assumption; its corrected capacity test checks
both real payloads, exact minimum disk boundaries and a historical small payload
(`20260927T140822977505Z-19149.json`).

Manifest `20260927T115142932176Z-15762.json` passes real external HTTPS
HTML/CSS/image loading, keyboard-triggered JavaScript DOM mutation, and wheel
scrolling in the installed ARM desktop. The document is served by HTTPbun and
its PNG by HTTPbingo; neither response is injected into Servo. Acceptance checks
the framebuffer's CSS color, image color diversity, green background after
typing into the input, and the red lower section after scrolling. Captures:
`build/browser-js-input-current.png` and `build/browser-scroll-current.png`.
The engine reported two completed HTTPS requests, HTTP 200, and zero page,
engine, network or allocation failure. The earlier HTTPbin variant failed native
transport before receiving HTTP (`20260927T114111366053Z-15517.json`); it is not
counted as passing compatibility evidence.

Expired-certificate rejection also passes inside the installed browser:
`20260927T115837021724Z-16195.json`,
`https://expired-isrgrootx1.letsencrypt.org/`. Native TLS returned the distinct
certificate-rejection value (9), zero completed HTTP responses and a recoverable
page error; the engine remained running without failure. The transport now
preserves certificate rejection separately from connection failure, so an
unreachable host cannot falsely satisfy this check. Focused HTTP/TLS tests pass
in `20260927T115822171664Z-15893.json` (31 behavioral tests).

This uses the same disposable cold-booted installed disk with an offline kernel
update, not unmodified release ISO parity. Its 15-second observation interval is
not launch/load timing. Lifecycle/performance measurement,
default release packaging and target parity remain open. Older entries below
describe historical checkpoints and do not supersede this evidence.

Installed lifecycle checkpoint `20260927T122056355733Z-17052.json` passes
maximize/restore pixels, minimizing, and restoring the same live page from the
new browser drawer entry. Close acknowledges teardown with a measured allocator
peak of 156,332,864 bytes (not total system/browser RAM). Reopen renders Example
Domain, but its automated pixel check expected the former background color;
the served CSS is now `#eee`, matching the actual `(238,238,238)` pixel. The bounded
reopen-only retest passes (`20260927T122538019598Z-17217.json`), including a real
HTTPS page after close/reopen and a 209,058,816-byte allocator peak for that run.
Earlier failures sampled the drawer/shadow or
overshot pointer coordinates; the controller now reduces relative movement after
overshoot and uses individually acknowledged keys for short commands. These are
test corrections, not claimed OS performance fixes.

The browser now has a stable minimized-shelf identity and uses the existing
restore/maximize/close actions. Same-session network lease renewal replaces the
authority for future requests without changing ownership or weakening checks on
already-active transfers. Focused shelf and network-bridge behavioral tests pass;
an accidentally unfiltered host service suite exceeded its default thread stack,
so that broad invocation is not cited as passing evidence.

## Open acceptance gates

### Timing measurement checkpoint

The reproducible component wrapper and matching QEMU installed-kernel packaging
build pass in `20260927T123409619320Z-18143.json`. The generated media is
`builds/InfinityOS-aarch64-qemu-test.iso`; this was incremental, not a clean
release. The new `--iso-parity` run uses a new disk, disables offline kernel
replacement, verifies the installed bytes and boots with media detached.
Cold-install run `20260927T124526053815Z-24464.json` passes. The new disk's
870,728,112-byte kernel exactly matches SHA-256
`ea0e214870e1c68c6784edb531afe50e700677cbd29f18e6bc2d8effb6b16027`
and all 11 references verified. It booted without media, completed account
setup, rebooted and authenticated, launched Browser from the app catalog with
explicit network consent, rendered real HTTPS Example Domain, then accepted a
new URL through its native address field and rendered the external red CSS page.
No offline replacement was used. Receipt and screenshot are preserved under
`builds/evidence/browser-v0.1/`. This establishes the ARM QEMU path only.

Subsequent builds include the browser by default (`NATIVE_BROWSER=1`). The
model-inclusive installer also selects that mode for its live kernel, while
retaining its own model/ESP payload manifest. A clean release with that default,
model-inclusive media verification and x86 installed execution remain open.

`20260927T122822317730Z-17672.json` measured cold native engine startup and
Example Domain loading on the installed 4-vCPU, 12-GiB ARM QEMU TCG guest:
first engine frame 5.7003 s, first real page pixels 10.6092 s, and page complete
10.6092 s. Peak engine allocator use was 209,058,816 bytes. This excludes the
512-MiB reserved arena, desktop surfaces and other OS memory. The QEMU host
process consumed 21.26 CPU seconds (200.39% over that interval); that is emulator
CPU accounting, not guest CPU utilization. These are polling upper bounds,
not hardware-accelerated VM performance claims.

The original pointer snapshot samples (0.52–1.30 s) are **not cursor latency**:
relative-pointer handling does not publish the broad diagnostics snapshot, so
they include waiting for its coarse clock publication. The harness now measures
visible cursor pixels in a quiet screen corner instead. No scheduler optimization is claimed
from the misleading snapshot numbers. The pixel-based retest passes in
`20260927T125502320022Z-24796.json`: 37.65–78.31 ms submission-to-visible-cursor
roundtrips after page loading. Its first page paint/complete was 10.5450 s,
first engine frame 5.6474 s, peak allocation 209,058,816 bytes, and host emulator
CPU 200.66%. It reused the unmodified fresh-installed disk, not an offline
kernel update. Active-load run `20260927T125713345953Z-24869.json` also completes:
10 pointer-to-pixel samples while the page was loading range from 32.32 to
336.70 ms; post-load samples range from 30.16 to 171.45 ms. First page paint
was 10.4796 s, allocator peak unchanged, emulator CPU 200.77%. This demonstrates
that input continues during native rendering, but the emulated worst-case
latency is not a claim of 60-FPS hardware performance. No engine/UI failure or
allocation failure was observed. Both timing receipts use actual framebuffer
changes and are retained alongside the logs.
Logs are preserved under
`builds/evidence/browser-v0.1/` before clean builds.

## Historical packaging checkpoints (superseded by the dated results above)

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

The intercepted-navigation redirect fix now passes the real Servo component
guest (`20260927T090340265723Z-89690.json`): an initial 301 reaches the destination
through another provider request and must produce the expected JS pixels.
Paired keyboard/mouse input, resize, Back/Forward and shutdown still pass. The
patch supplies location metadata to Servo's existing navigation redirect
controller and intercepts the current redirect URL, not the original URL.
Installed rerun is pending; subresource Fetch redirect modes are not claimed.

The completion-gated installed rerun failed
(`20260927T091952982886Z-94058.json`): the link received paired pointer events,
but load revision stayed at 2 and history stayed at 1. Frame revision advanced
from 6 to 7 only. This supersedes any interpretation of screenshot capture or
click delivery as successful navigation. The harness now waits for an actual
load transition and completion rather than a fixed eight-second delay. Further
network-result diagnostics and recoverable main-document error reporting are
under verification; no updated default-browser release is claimed.

Native component recovery now passes (`build/servo-platform-probe/component-boot.json`,
the component portion of `build/browser-queued-retry-test.log`). Provider failure
is distinct from intentional cancellation: Servo receives `ConnectionFailure`,
the native shell receives a bounded error event, and a retry arriving before the
failed document completes is retained rather than lost. The fixture denies a
document, immediately requests a different allowed document, then requires its
actual location and JS-painted pixels before shutdown. Existing redirect,
paired input, resize and history checks remain passing. Engine allocator peak
for this expanded fixture is 169,657,472 bytes; it is not whole-system peak RAM.
The installed navigation rerun (`20260927T094933377595Z-3096.json`) then completed
three native transactions (initial 200, followed by two 301 responses), but the
next request failed. Inspection of the real redirect chain found an HTTP target
paired with HSTS; interception had omitted Servo's HSTS update. HSTS handling is
being restored and tested before claiming this navigation passes. A recoverable
document error also no longer latches the supervisor's permanent engine-failure
counter. Plain HTTP transport remains unsupported and is a separate open gate.

The HSTS fixture subsequently passed and the installed rerun progressed through
13 real native requests, reaching 200 responses
(`20260927T095910127797Z-7263.json`). The engine then aborted through the C abort
handler: supervisor state 3, failure 4, allocator peak 244,012,544 bytes. This is
not a successful completed-page/navigation result. Memory pressure near the
256 MiB arena is a lead, not yet a proven cause. No release ISO was updated.

The allocation diagnostic rerun (`20260927T101420564854Z-11668.json`)
confirmed a failed 16,777,264-byte arena request immediately before that abort.
The allocator previously discarded half of a misaligned 512 MiB grant to select
one power-of-two root. It now partitions the same grant into aligned buddy roots
and bounds coalescing to that grant. Kernel load span is unchanged. Seven host
behavioral tests pass, including exhaustion and scrambled reclamation across
non-power-of-two roots (`20260927T102718005581Z-11991.json`). The native Servo
component also passes. Installed rerun `20260927T102047099692Z-11812.json`
no longer aborts or reports failed allocation; link load revision reaches 4
and loading becomes false after 13 completed native requests. This is not yet
navigation acceptance: Back stalls with loading set. The PNG initially reviewed
was an older conversion, not the newly captured PPM; do not use it as evidence
of a presentation defect.

`20260927T103006872681Z-12181.json` verifies the redirected IANA destination,
Back to Example Domain, and Forward to IANA through the actual engine URL and
completed-load state. Fresh PPM conversion confirms IANA text/CSS in the native
viewport. Cached traversal now consumes Servo's traversal-complete callback;
it does not wait for a nonexistent new document-load event. Reload failed closed
after the existing sixty-second network lease expired. An explicit browser
authorization command with a bounded ten-minute lease is being verified; it
retains authenticated operator consent and endpoint policy. This is not yet
the native permission UI or a completed browser acceptance result.

Installed navigation passes in `20260927T103806468778Z-12383.json`: real link
navigation through IANA's redirect chain, Back, Forward, and Reload all reach
the expected engine URL with loading false and no page/worker failure. Reload
adds ten completed native requests (23 total). Fresh `browser-reload.ppm`
shows IANA content in the native window. The browser command's explicit
ten-minute lease preserves capability checks; the normal HTTPS command still
uses sixty seconds. The component fixture also requires history completion,
not just location change. Some optional IANA subresources failed on the first
load; comprehensive site compatibility is not claimed. Installed JS/input,
image/scroll/download, permission UI, performance and release-ISO parity remain
open. The active-session PEAK counter still is not a RAM measurement.

Download implementation now includes validated Content-Disposition attachment
interception and a version-two native ABI offer into a single bounded consent
mailbox (16 KiB file limit). No website-selected path is accepted. The real Servo
component fixture verifies exact attachment bytes, basename and MIME type
(`20260927T105118118796Z-12917.json`); the integrated ARM kernel links with that
ABI (`20260927T105518500459Z-13066.json`). Core tests pass (16 tests,
`20260927T104956042077Z-12859.json`). Native object storage has an atomic download
transaction that attaches content and a related MIME metadata object with owner
identity, refuses overwrite, and rolls back failed writes. Remount and injected
write-failure coverage passed (`20260927T105727871041Z-13163.json`). The typed
ObjectService entry additionally requires Create, NamespaceAttach and
RelationshipAttach authority. The browser-to-service save-consent UI/controller
is now wired with explicit Save/Discard, active-session validation, a short-lived
object-write capability and saved/error states. Installed persistence verification
now passes as detailed below.
RFC 5987 extended filenames currently use the safe fallback basename rather
than claiming full filename-encoding support. No release ISO was updated.

The first installed download retry reached the real HTTPS endpoint but received
HTTP 402 because the native request omitted User-Agent. Adding the honest
`InfinityOS/0.1` identifier passed the native HTTP tests and all 17 browser-core
tests. The installed retry received HTTP 200 and an attachment with zero engine
or network failures (`20260927T111621094082Z-14276.json`), but Save did not activate:
the underlying desktop AI widget consumed its click. Browser occlusion is being
added to widget dispatch; this failed interaction is not download acceptance.

The corrected installed download test passed
(`20260927T112310848229Z-14737.json`): real HTTPS from
`https://httpbingo.org/response-headers` delivered an attachment; the native Save
button moved the consent state from 1 to 2. After stopping the guest, a read-only
mount of its installed disk verified the exact 128 received bytes at
`/home/default/downloads/native-browser-test.txt`, nonzero owner identity and a
related `text/plain` metadata object with the same owner. The saved-state capture
`build/browser-download-saved-current.png` was visually reviewed. No network,
page, engine or allocation failure was reported. Browser bounds now prevent
covered widgets from consuming its controls; pending consent is cleared when
the session changes. The measured 6.15 seconds is attachment observation time,
not browser first-paint or general performance acceptance. This remains the
kernel-updated disposable ARM installation, not unmodified release-ISO parity.

The installed launcher/consent path also passes
(`20260927T113301747791Z-15110.json`). Browser-enabled builds append a typed
Infinity Browser catalog entry with its dedicated generated artwork; older
launcher indices are unchanged. Selecting it displays native network consent
without starting the engine or completing any network requests. Explicit Allow
uses the existing authenticated-operator, policy-governed ten-minute lease.
The browser then loads Example Domain over HTTPS and passes real link to IANA,
Back, Forward and Reload with observed location and load transitions. The native
permission capture was visually reviewed. A subresource transport failure was
observed around history traversal; it did not fail the main document, and reload
ended with status 200 and zero current network/page/engine failure. The 15-second
observation window is a test wait, not a measured launch-time claim.
The normal release build remains browser-disabled pending full installed
acceptance; this catalog integration is not default release-ISO registration.
Catalog-enabled host behavior testing exposed the old fixed 94-byte shortcut
record overflowing with an eighteenth app. Its size now follows catalog length,
while decoding retains the 15-app and 17-app formats and appends default positions
for new entries. Launcher drag/order/persistence tests pass with browser disabled
and enabled (`20260927T113810063199Z-15250.json`), including both legacy migrations.
This persistence correction still needs inclusion in the next installed kernel.

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
