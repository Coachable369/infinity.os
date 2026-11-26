# Fabric workspace — Sapphire dashboard

`reference.png` is the AI-generated visual target (2026-11-25). `components.png`
shows the exact SVG-derived production component pixels. The original design's
fictional resource values are illustrative; production values use authenticated
Fabric snapshots. The compositor owns the title bar; File/Help occupy a separate
fixed 36px application menu, never a second title bar.

## Geometry and responsive behavior

Menu 36px; toolbar 48px; outer inset 16px; gutter 12px. Right inspector 280px,
scrollable independently. Center resource cards precede Nodes and Job activity.
Nodes receive three fifths of flexible vertical space; jobs two fifths. Card
interior #112D48, inspector #0E1C36, steel border #365774, selected row uses the
active Studio accent. Inter 16px body, 20–22px section headings, 24px totals.
Icons are 44px with 12px spacing. Below 690px center width the decorative icons
hide, preserving all values and fixed text sizes. Tables scroll rather than
compressing headers. The inspector retains its width at normal/maximized sizes.

## Reusable controls

Gap: Python UIKit had no image-backed dashboard resource card or filled toggle
adapter. `infinity_uikit_dashboard.dashboard_controls(root, style)` now supplies
DashboardCard.TFrame, DashboardPanel.TFrame, corresponding label styles, and
Dashboard.TCheckbutton. Native ttk keyboard, focus, state and command semantics
remain authoritative. Toggle states are off/on/hover-off/hover-on/disabled.
No animation timers or application-specific event loop is added.

SVG source is limited to rect, circle and line primitives. The build compiler
`tools/native-python/uikit-dashboard.py` renders at 4x and downsamples, then emits
PPM-compatible compressed RGB for native Tk. Panel nine-slices reserve 16px on
all edges. Do not scale corner radii with window geometry. Standard buttons,
entries, menus and scrollbars retain Studio's existing UIKit theme overrides.

## Visual acceptance

Compare native normal/maximized/restored captures with the reference, including
small-screen behavior, card values, selected nodes, empty jobs, disabled policy,
conflict/retry and keyboard toggle states. Generated reference artwork is a
visual target, not proof of native rendering. Pixel parity remains open until
native captures are reviewed; do not report 100% based on a successful build.

### Installed review — 2026-11-25

ARM disk-only diagnostic run: `builds/manifests/20261126T012029915299Z-34148.json`.
Launch: 1.29 seconds. Normal, maximized, and restored lifecycle checks passed.
Captures: `build/validation/fabric-manager-installed/node-1/fabric-manager-{normal,maximized,restored}.png`.
Host control behavior: `builds/manifests/20261126T011753138731Z-33880.json`.
SVG pixel and package parity: `builds/manifests/20261126T012322186115Z-34185.json`.

Corrected during six bounded correction passes: corner slicing artifacts,
empty-state clipping, inspector reachability, unequal card widths, initial node
label clipping, and small-window table height. These checks do not establish
100% reference parity. Remaining failures: reference gradient materials versus
flat native interiors; reference table header/row treatments and status icons;
reference inspector separators and structured node details. Menu placement uses
the mandatory compositor/application separation instead of the generated board's
combined title/menu. Visual acceptance remains open; stop speculative correction
under the repository's six-loop limit. This diagnostic kernel is not a published
installer, and this UI does not establish arbitrary CPU/RAM/GPU pooling.
