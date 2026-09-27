# Infinity Browser daily-driver milestone

Status: **in progress; not accepted, not a daily-driver release.**

## Compact implementation / acceptance checklist

1. Extend request interception to carry method, headers and bounded body/stream
   handles. Verify POST, redirects, cookies, authentication, upload, cancellation
   and origin checks through native networking. Never bypass Servo security.
2. Connect stable tab/window/profile identities throughout commands, events,
   resources and retained surfaces; implement tab controls and shortcuts. Verify
   independent history/input, reorder, close/reopen, background navigation and
   isolation. Do not create a second WebView just to preview a tab.
3. Commit profile-scoped bookmarks/history/settings/recovery through native
   objects. Add private lifetime disposal, encrypted credentials, autofill,
   permissions and native managers. Validate crash recovery and data deletion.
4. Wire engine find/context menus/devtools, downloads and PDF/media adapters.
   Verify actual content, progress, controls and permission decisions; no inert
   controls or fabricated engine support.
5. Connect Time Travel's fail-closed capture policy to trusted whole-page engine
   assessment and bounded native storage. Connect Tab Glance to retained tab
   surfaces, with the 500 ms delay and immediate dismissal.
6. Package both installed targets and validate the user's complete site/workflow
   matrix, including authentication, uploads, media, permission prompts and PDFs.
   Record latency/memory and screenshot review against the generated kit.

## Implemented in this increment

- Browser address/caret/title-only changes now invalidate native chrome instead
  of discarding the retained page surface. A shared `damage::PageKey` tracks
  page frames, active tab, error/permission/download state, loading and input
  pressure; changes to those still require a full browser update. Both retained
  surface invalidation and desktop damage use this distinction. The clipped
  chrome paint returns before copying page pixels. This source optimization
  awaits compilation, installed regression checks and before/after timings;
  no latency reduction is claimed yet.

- Omnibox Ctrl/Cmd+L now selects the entire draft; typing replaces it, deletion
  clears it, and Home/Left or End/Right collapse the selection to the expected
  edge. Ctrl/Cmd+A selects the focused omnibox. The native painter shows the
  selected span and hides the caret while selected. This source change awaits
  the next kernel build and the updated installed `--address` acceptance case;
  it is not in the current ISO.

- `sdk/infinity-browser-core/tabs.rs`: bounded 24-tab state model, stable IDs,
  window/profile/private scope, pin partitions, reorder, duplicate, selection,
  cycling, bounded closed-tab history, generation-checked metadata, mute state.
  The richer pin/reorder/reopen/recovery model is **not yet connected** to the
  native shell; the engine integration below currently supports basic tabs.
- `sdk/infinity-browser-servo/session.rs` now owns a bounded group of real
  WebViews with a stable global delegate, per-tab resource queues and independent
  history. Hidden tabs service requests without painting; selection focuses and
  paints the selected WebView. Closing one tab cancels only its requests.
- Native tab strip: create, select, close, page titles, Ctrl/Cmd+T and W.
  Frame generations include the active tab identity to reject another tab's
  retained pixels. More shortcuts, overflow, pin/reorder/reopen, favicons and
  background metadata remain pending. The eight-slot limit is a capacity bound,
  **not an eight-page memory/stability acceptance result**.
- Increased matching native thread/C lock-reader budgets from 32 to 64 after the
  second real page exhausted the previous thread table. Both architecture guest
  probes exercise capacity, exhaustion, joining and slot reuse. Corrected the C
  denial fixture's syscall return type and architecture-specific symbol names;
  ambient file access remains denied.
- Versioned recovery encoding preserves order, URLs, titles, pin/mute state and
  selection. Decode rejects wrong profile, invalid records, truncation and
  corruption. Private windows cannot encode. Native atomic persistence and
  crash/reboot recovery are **not yet wired**. The checksum is corruption
  detection, not encryption or authentication.
- Tab Glance policy: 500 ms intentional hover; immediate dismissal; no selection
  mutation; exact window/profile/private/tab/document/surface matching. Actual
  surface retention and native preview rendering remain to be connected.
- `time_travel.rs`: bounded event-driven RGBA snapshot store in caller-owned
  memory, count/byte/age caps, duplicate suppression, expiry and clearing.
  Private, cross-profile, unknown and sensitive/editable captures are rejected.
  No heap allocation, DOM replay or continuous screenshot loop. Safe capture
  assessment, disk storage and history UI are **not yet implemented**. The
  adapter must remain fail-closed until it can assess subframes as well as the
  main page; a webpage-supplied boolean is not a trusted safety verdict.
- `omnibox.rs` and `kernel/core/browser_controller.rs`: address submission now
  normalizes bare domains to HTTPS and percent-encodes search queries. Explicit
  HTTP(S) addresses retain their original bytes. Search currently uses Google;
  selectable engines and suggestions remain pending. Real Google compatibility
  is not established by URL construction tests.

## Evidence and limits

The new two-tab AArch64 component guest passed actual pixel, switch, independent
page restoration and background-close checks, alongside existing lifecycle,
JavaScript/input, history, request-failure recovery and download byte checks:
`20260927T155042429029Z-34591.json`. Peak engine allocation was 263,158,336 bytes
(about 251 MiB). Requests are injected by this test fixture: this is **not live
HTTPS or installed-system proof**, and does not establish eight-tab capacity.

`./build-kit browser-core-tests` passed 24 tests, including the new independent
tab/close/toolbar hit targets: `20260927T155356256690Z-35008.json`.
Native std/thread guest probes passed on AArch64 and x86_64 respectively:
`20260927T155617058227Z-35332.json`,
`20260927T155736612258Z-35623.json`.
The updated x86_64 native-browser kernel compile check also passed:
`20260927T162822482342Z-43358.json` (existing warnings remain).

The AArch64 live/install test ISO was rebuilt successfully through
`./build-kit browser-aarch64` (`20260927T155825375026Z-35725.json`). The cold
installation verified the installed kernel SHA-256
`89395ae1ca683c6205e6637ead658fec57ca702dfe94005fe2de8cbc363d826a`
and booted with media detached. That first run stopped at a harness-only symbol
lookup (`20260927T160846711210Z-42571.json`); no kernel replacement was made.
The resumed test on the same disk passed real HTTPS CSS/image rendering,
JavaScript keyboard input, scrolling, native tab create/select/close, and
background scroll restoration (`20260927T162027164551Z-42885.json`). Its receipt
says `browser_iso_parity: false` because it is a reuse run; the original cold
install receipt supplies the separate byte-for-byte provenance. This is not an
x86 installed result or broad site-compatibility acceptance.

Evidence: `builds/evidence/browser-daily-driver/installed-tabs-aarch64.json`,
`installed-two-tabs.png`, and `installed-tabs-js.png`. Visual review confirms
separate native tab/close targets and the existing sapphire chrome; the simple
fixtures intentionally have no HTML title, so their labels show "New tab".
The test now types browser fields through keyboard/event-loop acknowledgement,
not Console editor-length counters. Optimized kernels without the private PEAK
symbol can run interaction tests; memory-measurement tests still require it and
fail explicitly rather than reporting an invented number.

`./build-kit run cargo test --manifest-path sdk/infinity-browser-core/Cargo.toml`
passed 23 behavioral tests, including the new transitions, actual snapshot pixel
bytes, limits, recovery corruption and URL encoding. Receipt:
`builds/manifests/20260927T153535932185Z-32837.json`.
These are host core tests, **not installed browser feature acceptance**.
The normal build now includes these behavioral tests before expensive image
builds; the focused entrypoint is `./build-kit browser-core-tests`.

Native-browser kernel compile checks also passed for AArch64 and x86_64:
`20260927T153704259271Z-32948.json` and
`20260927T153819545393Z-33655.json`, respectively. These checks do not link,
boot, or prove live browsing and still emit existing repository warnings.

The pre-existing x86 cold-install run timed out after 900 seconds during
installation verification under x86 TCG on an ARM host:
`builds/manifests/20260927T150252504957Z-31200.json`.
This is not a successful installed x86 result. The test disk remains available
under `build/browser-installed-1790521372574912000` for diagnosis, until normal
build cleanup removes it. Earlier v0.1 evidence remains separately documented in
`infinity-browser-acceptance.md`; it does not establish this new milestone.

## First integration constraints found

### Verification follow-up (27 September)

The focused `./build-kit x86_64` build passed in receipt
`20260927T163105799603Z-44120.json`, producing both the bootstrap test ISO and
the model-inclusive `builds/InfinityOS-x86_64.iso`. This is not a clean full
release or an installed x86 acceptance result. Browser peak allocation now has
an exported diagnostic counter so release optimization cannot remove the
measurement point.

The navigation harness now checks real HTTPS link/back/forward/reload using
per-page URL hashes, completed load revisions and actual framebuffer colors.
Its first run (`20260927T165251295443Z-59745.json`) failed while typing the
Console launch command, before browser navigation. Navigation uses acknowledged
individual key events for the retry rather than rapid batches. The retry passed
in `20260927T165600056847Z-59816.json`: link, back, forward and reload each
matched the intended URL hash, advanced completed-load revision, and rendered
the expected CSS pixels, with zero engine/network errors. This used the same
previously cold-installed ARM disk without replacing its kernel; it is not a
new cold install or x86 result. The x86 installation verification timeout is now
30 minutes under cross-architecture TCG; byte and checksum verification are
unchanged.

- The normal-profile multi-WebView group now retains one global delegate.
  Private/profile partitioning and stable persisted identity remain pending.
  Metadata delivery under mailbox pressure needs further hardening before rapid
  multi-tab operation can be accepted.
- `resources.rs` rejects non-GET requests. The pinned Servo
  `WebResourceRequest` has method/headers but no request body. The adapter must
  be extended before real login forms or uploads can be accepted.
- The native download mailbox is currently 16 KiB and one attachment. It is not
  a streaming background download manager.
- Private/profile isolation, password encryption, PDFs, media and developer
  tooling still require integration and runtime verification. No acceptance
  claim is made for these from this increment.

## Visual specification

Reference: `design/infinity-browser/idesign-kit-everyday-v1.png`.
Generated using the built-in image tool, extending the existing v0.1 reference;
no icon redesign. The kit guides implementation, not a bitmap painted in place
of functioning UI. Use native Fira Sans, the existing sapphire/titanium skin,
8/16/24 gutters, neutral page pixels and restrained focus rings.

- Add a compact tab strip between the title and navigation bars; keep existing
  navigation ordering and window controls. Pinned tabs use compact slots.
- Glance: large readable preview beneath the hovered tab, capped below half the
  viewport; include title and domain. No keyboard focus theft.
- Time Travel: compact Back/history disclosure with thumbnails and timestamps;
  display “Saved visual — not a live page” when inspecting a snapshot.
- Private state: subtle violet accent plus an explicit label. Do not use the
  generated board's incidental privacy sentence: private browsing does not
  make network activity private from sites or network operators.
- Menus progressively expose bookmarks, history, downloads, permissions,
  profiles and settings rather than adding permanent toolbar buttons.

Generation brief: preserve the existing midnight sapphire native chrome, cyan
edge, grey titanium navigation, Fira Sans and spacing. Show three tabs, large
Tab Glance, Time Travel thumbnails, pinned/audio/muted/private states, omnibox
suggestions, bookmark folder, download, permission and focus recipes. Keep the
existing icon family; clear production UI rather than conceptual decoration.

An updated AArch64 **test** ISO exists at
`builds/InfinityOS-aarch64-qemu-test.iso`. No daily-driver release or fresh full
release build is claimed. The remaining checklist above is still required.
