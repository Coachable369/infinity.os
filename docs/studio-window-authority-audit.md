# Installed window skin authority

Infinity Studio is the authoring authority. The saved UIKit document supplies
`uikit-styles.bin`, `uikit-tokens.bin`, the Precision atlas, window control atlas
and AI tab atlas. The kernel and Studio preview sample the same native painters.
Saving prepares the next build; publishing an ISO does not change an already
installed system. Never claim an installed screenshot reflects an unpublished build.

## Window inventory and ownership

| Installed surface | Frame owner | Controls / assistant |
| --- | --- | --- |
| File Navigator, all six instances | retained application compositor | shared controls and attached tab |
| Command Window, including native C app content | retained application compositor | shared controls and attached tab |
| Text Editor, including terminal panel | retained application compositor | shared controls and attached tab |
| Task Manager | retained application compositor | shared controls and attached tab |
| Settings, every section | retained application compositor | shared controls and attached tab; explicit Studio content overrides preserved |
| Infinity Browser | retained application compositor | shared control atlas inside existing hit targets; same attached tab, not a separate circular button |
| InfinityCode and Python/Tk application windows | native GUI compositor | shared controls; authenticated, window-bound assistant with native lifecycle dispatch |
| File Preview | shared Precision frame and close-control atlas | read-only transient preview; no separate application assistant |
| Editor open/save and unsaved-work dialogs | shared Precision frame | modal actions, no independent assistant or lifecycle controls |
| System permission / confirmation sheets | shared Precision frame or panel | trusted modal actions; no assistant allowed to intercept approval |
| Launcher, desktop widgets, menus, dock | Studio desktop templates and shared panel recipes | shell surfaces, not application windows; authored desktop controls |
| Login and first-run configuration | Studio login/configuration templates and shared panel recipes | secure pre-session surfaces, no app assistant |
| Fullscreen spatial presentation | shared spatial renderer | full-screen surface, not framed app chrome; shared assistant artwork |

Applications own content, not the exterior frame. Retained composition reapplies
the frame after cached, partial and uncached paints. Native GUI draws the same
frame after presenting client pixels. Fullscreen, withdrawn, minimized, trusted
and modal native surfaces do not receive an interactive application tab.

## Corrections in this cycle

- Removed browser-only assistant geometry and circular artwork bypass.
- Replaced browser-only window control glyphs with Studio's shared control atlas,
  preserving the browser's larger input targets and restore behavior.
- Connected draft assistant/control RGBA overrides to the native preview transport
  and production samplers, with exact-size validation and safe fallback.
- Added the shared AI adorner to ordinary native toolkit windows. Its target is
  the explicitly selected window, not whichever window later receives focus.
  Focus changes dismiss the panel; close uses the application's ordinary close
  request rather than destroying its process. Teardown clears assistant state.
- Native move/hide/destroy damage includes the external tab footprint.

## Verification contract

`./build-kit window-skin-tests` verifies replacement sprite pixels, malformed
payload fallback, mirrored alpha, identical assistant pixels across application
owners, shared frame pixels across retained cache paths, and window control states.
`./build-kit run sh tools/runtime-test.sh` exercises native tab interaction and
window damage with both native address-space adapters.

These are host behavioral checks, not an installed-window visual audit. Before
final acceptance, compare normal/focused/maximized windows and open/closed AI
panels in a fresh installed VM without the ISO attached, including a saved Studio
theme change. Verify Studio preview, packaged bytes and those installed pixels
agree. Runtime visual acceptance and new release packaging remain pending until
their evidence is recorded.

## Evidence recorded 2026-11-01

- Shared atlas/control/assistant pixel checks passed:
  `builds/manifests/20261101T162445308088Z-8702.json`.
- Native lifecycle and damage checks passed with both address-space adapters:
  `builds/manifests/20261101T162045077111Z-7473.json`.
- Studio's 185 tests passed; signed package rebuilt:
  `builds/manifests/20261101T163022245194Z-9851.json` and
  `builds/manifests/20261101T164752203385Z-74905.json`.
- Clean dual-architecture release did **not** complete:
  `builds/manifests/20261101T163529049711Z-10349.json`.
  x86_64 installed-kernel linking exceeded the ordinary-data address ceiling.
  The diagnostic link map places the end of BSS at `0x81cdee70`, beyond
  `0x80000000`; a concurrently added login-wave cache occupies `0x1fa4038`
  bytes. Do not treat existing published ISOs as containing this cycle.
- Fresh-installed visual acceptance, including a Studio theme round-trip,
  remains pending. Host pixel tests and compilation do not replace that check.
