# Sapphire QA defects

All findings below are production-painter host captures, not installed VM proof.
No measured 95% acceptance is asserted. Preserve explicit Studio overrides when
correcting default presentation.

| ID | Surface | Observed failure | Required verification | Status |
| --- | --- | --- | --- | --- |
| SAPPHIRE-001 | Themes & Skins | Dense single-column setting rows do not match the approved two-column reference composition and spacing. | Normal/maximized/restored layouts, row hit targets, expanded details and keyboard order. | Corrected in host production render; installed verification pending |
| SAPPHIRE-002 | Themes & Skins | Hero subtitle and action label are truncated at the reference capture size. | Complete readable copy with adequate button width and responsive wrapping; no overlap at supported sizes. | Corrected in host production render; installed verification pending |
| SAPPHIRE-003 | Themes & Skins | Wallpaper row is cut off at the bottom of the initial viewport. | Reference layout density, complete visible controls and usable scrolling at smaller sizes. | Corrected in host production render; installed verification pending |

Evidence: `builds/validation/sapphire-coverage/settings-2.png`, 1536×1024,
compiled from the current pending Studio document. Manifest:
`builds/manifests/20261119T222503831504Z-40661.json`.

## Coverage checkpoints

- Initial catalog: 76 surface entries / 650 candidate state checks. These are
  inventory counts, not unique component counts or completed checks.
- Default captures: 11 Settings pages and five desktop template surfaces.
- Settings contact sheet: `builds/validation/sapphire-coverage/settings-contact-sheet.png`.
- Desktop contact sheet: `builds/validation/sapphire-coverage/desktop-contact-sheet.png`.
- Individual controls, interactive states, runtime-only surfaces, applications
  and installed ARM/x86 checks remain open in `sapphire-coverage.json`.
- Desktop preview scenes use different authored backgrounds. Do not score their
  relative colors against one another until backdrop and transparency fixtures
  are normalized to each approved reference.

## Correction evidence — 2026-11-19

- Themes cards now use the reference two-column composition, with a one-column
  fallback below 800 logical content pixels. Expansion moves subsequent rows
  without moving the adjacent card in the same row. All eight cards fit the
  1536×1024 default capture. Hero action and subtitle fit that reference size.
- Reviewed default and expanded first/second cards at 1024, 1536 and 1920 widths:
  `builds/validation/sapphire-coverage/themes-responsive.png`. Native capture
  manifest: `20261119T232132731795Z-42708.json`.
- Responsive geometry, hit targets, expansion and scrolling passed the behavioral
  harness: `20261119T231931661446Z-42458.json`.
- Shared panel 100% opacity now yields an opaque center on different backdrops.
  Python UIKit includes genuine disabled artwork for buttons and fields.
- Active production-painter suite passed:
  `20261119T232341417327Z-43080.json`. This includes Settings, Task Manager,
  shared surfaces, clipping and retained-window rendering. It is host evidence.
- Task Manager and account-management state captures were reviewed in
  `builds/validation/sapphire-coverage/tasks-accounts.png`.

These corrections do not establish a 95% score for complete screens. Legacy
Studio-authored sidebar color, app-specific composition and installed runtime
states still require comparison. Narrow hero copy can elide at small widths.

## Access-control palette correction

The clean release exposed pre-session default colors still independent of UIKit.
Access controls now use the same packaged semantic palette as desktop controls;
explicit authored colors remain independent. All 17 access pixel/geometry/state
checks pass in manifest `20261120T010552422342Z-73002.json`. Normal and focused
production surfaces were inspected in
`builds/validation/sapphire-coverage/access-focus-corrected.png`.
The first ARM test image predates this correction; final images must be rebuilt.

## Installed workspace corrections

- InfinityCode's installed ARM screenshots exposed an empty task tray covering
  its toolbox. Empty trays now paint no pixels and intercept no pointer, wheel
  or keyboard input. Nine native shelf/overlay fixtures pass, including exact
  partial/full-frame equivalence: `20261120T012706414648Z-74539.json`.
- Python workspace scrollbars now share rounded UIKit thumb artwork, semantic
  states and compact rails. The inherited host trough padding is removed.
  Eleven real Tk interaction/geometry tests pass, including drag scrolling:
  `20261120T013013146402Z-74863.json`. Native installed verification of this
  latest adapter remains pending; artwork inspection is not that proof.
- Infinity Memory uses semantic palette values and the shared release-to-open
  menu dispatcher. Its menu height remains fixed during resize. Host interaction
  and captures use isolated memory records and production UIKit button fixtures;
  `memory-host-uikit.png` is not an installed screenshot or an exact native
  service-button geometry comparison.
- Media Player now consumes the packaged semantic palette rather than legacy
  hardcoded colors. Playback/open/replace/close behavior remains unchanged and
  passes `20261120T013049558539Z-74907.json`.
- Installed ARM InfinityCode launch/maximize/keyboard/close proof:
  `20261120T011933810951Z-73829.json`; screenshots:
  `infinitycode-installed-normal.png`, `infinitycode-installed-maximized.png`.
  These show the pre-correction rails and empty shelf; they are failure evidence
  for those elements, not final visual acceptance.
