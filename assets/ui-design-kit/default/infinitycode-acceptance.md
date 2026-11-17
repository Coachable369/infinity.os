# InfinityCode workspace acceptance — 2026-11-17

Native ARM installed-system verification passed for startup, menu interaction,
fixed chrome, maximize/restore, keyboard zoom and scrollable toolbox/inspector.
Release packaging is a separate gate; see the release build manifest.

## Observable evidence

Disposable installed ARM guest, 2560×1440, same VM and account:
- Before: blank startup lasting approximately 75 seconds.
- After: complete workspace in the first 5-second capture (`click-start-5.png`).
- View opens on a completed click and remains visible (`click-menu-open.png`).
  Selecting Zoom in changes workspace density to 110%; Cmd-minus restores 100%.
- Menu/toolbar heights stay fixed during maximize/restore; inspector stays right
  anchored; the form remains centered when it fits and scrollable otherwise.
- Dragging the toolbox scrollbar exposes Layers and Delete selected.

Screenshots under `builds/validation/infinitycode-fixed-chrome/`:
`click-menu-open.png`, `final-cmd-minus.png`, `final-maximized.png`,
`final-restored-toolbox-scroll.png`.

## Design comparison

Compared installed pixels with the application-workspace and tab specifications:
Inter typography, navy pane colors, fixed chrome, soft-shoulder tabs, production
UIKit button/input artwork and compositor-owned outer frame are integrated.
Host behavioral checks also compare decoded control pixels with the production
surface samples at 1× and 2×. This verifies those components; it is not a claim
that the whole screen is pixel-identical to the older illustrative workspace
mockup. Native toolkit scrollbars remain the existing shared implementation.

## Verification manifests

- 10 host behavioral/pixel checks: `20261117T181332634024Z-8101.json`.
- ARM installed kernel: `20261117T181338677619Z-8125.json`.
- Private installed disk byte verification: `20261117T181656388493Z-9041.json`.
- Native menu action: `20261117T181943689261Z-9116.json`.
- Native maximize/restore: `20261117T182001026823Z-9132.json`,
  `20261117T182008944269Z-9143.json`.
- Toolbox drag: `20261117T182016914522Z-9152.json`.

Native rendered acceptance above is ARM-only. Shared code and x86 packaging
checks do not substitute for an installed x86 visual acceptance run.
