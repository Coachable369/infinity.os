# Sapphire QA defects

Findings identify host and installed evidence separately below.
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

## Installed ARM follow-up

The model-bearing private QA image passed fresh installation, initial setup and
disk-only login. The installed lifecycle run passed InfinityCode and all six
native shell applications: File Navigator, Command Window, Text Editor, Task
Manager, Settings and Browser. Evidence:
`builds/validation/native-python/aarch64/tk-installed-1795140356516157000/installed.json`
(build-kit manifest `20261120T020541885544Z-81167.json`).

Reviewed normal/maximized/restored production captures establish that the empty
tray no longer covers the workspace and the updated Python scrollbar artwork is
present. Lifecycle checks do not establish full visual acceptance. Additional
findings were truncated native application titles, oversized Navigator columns
and clipped status text, and a cramped Task toolbar and graph caption. These
were corrected with shared app typography, bounded labels and field-metric
carets. The final installed rerun passed in manifest
`20261120T023232911114Z-83586.json`; its structured results are in
`builds/validation/native-python/aarch64/tk-installed-1795142012884160000/installed.json`.
Reviewed production captures in `builds/validation/sapphire-coverage/` include
`command-final.png`, `navigator-final.png` and `tasks-final.png`. Command Window
now shows the complete title with separate menu hit regions; Navigator's location,
columns and status use bounded shared typography; Task Manager's title, selector,
table and graph caption fit their regions. These are installed private-kernel
checks, not proof that the published release contains the corrections.

All eleven Settings pages also rendered with their first detail row expanded in
manifest `20261120T023204490543Z-83555.json`. The forced General preview exposes an
old name-edit detail state, but normal field activation explicitly clears that
state and edits inline. No speculative runtime change was made for that preview.
Reference-size per-element fidelity and unexercised states remain open.

## Memory installed follow-up

Manifest `20261120T023640172080Z-83721.json` passed native add/edit/search,
LLM recall, model switching, deletion and two disk-only cold boots. Results:
`builds/validation/sapphire-coverage/memory-installed-result.json`. Launch to
visible was 2126 ms on this isolated ARM fixture. Its screenshots exposed excess
button padding and unsupported separator glyphs; these were corrected without
changing memory retrieval. The padding/background and menu/clipboard/history
interaction checks passed in `20261120T024541876571Z-84200.json`; its capture is
`memory-host-uikit.png`. The final padding correction still needs an updated
installed screenshot. This evidence does not establish all-element 95% parity.
