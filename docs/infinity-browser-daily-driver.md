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

- `sdk/infinity-browser-core/tabs.rs`: bounded 24-tab state model, stable IDs,
  window/profile/private scope, pin partitions, reorder, duplicate, selection,
  cycling, bounded closed-tab history, generation-checked metadata, mute state.
  This is **not yet connected to multiple Servo WebViews or native tab chrome**.
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

- There is currently one engine session / WebView, not an existing full tab
  system. `Session::new` replaces Servo's global resource delegate. Multiple
  sessions must not overwrite each other's global networking authority.
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

No new daily-driver ISO has been released from this increment.
