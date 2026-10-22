# InfinityOS Installer Studio — IDesign Kit

## Product character

Installer Studio is a native macOS production tool: dark, precise, spatial,
and unmistakably InfinityOS. The chrome stays quiet so the installation screen
remains the brightest object. Cyan is reserved for selection, snapping, and
save state; warm amber marks validation warnings. The generated infinity-grid
icon is the application identity.

## Unified Studio workspace

The left rail exposes independently collapsible **UIKit**, **Configuration
Wizard**, **Installation Wizard**, and **System Settings** sections. UIKit is a
library workspace, not a pretend installer screen. Its categories reveal actual
OS surface states, glyphs, image assets and semantic color roles. Keep the same
three-column hierarchy: navigation, visual canvas, and inspector. Use the
existing generated Studio identity and approved Eclipse Precision reference;
no replacement icon family or alternate panel treatment is introduced.

UIKit previews preserve transparent edges over a dark neutral canvas. Surface
previews use the same nine-slice sampling as the OS, with independently editable
fill and edge layers and explicit normal/focused/pressed state selection. Color
wells show their semantic roles and exact hexadecimal values. Uploaded artwork
must be previewed with its true alpha and native aspect ratio; never flatten it
onto an opaque background. Large libraries scroll and filter without enlarging
the toolbar or hiding the inspector.

**Save Section** persists the active workspace; **Save All Sections** persists
every edited collection and UIKit. Preserve pending edits when switching
sections. Required workflow roles remain protected from removal even when their
appearance is unlocked. Source assets, imported replacements and runtime
exports stay inside the repository and are included in both installed and live
builds. Saving is not a live update of an already installed VM.

**Save & Update ISO** expands a compact publishing panel below the toolbar.
Keep the editor visible while showing the target architecture, selected ISO,
progress, and verified output. Save all pending sections before injection.
Update a staged copy, rebuild only UI-dependent kernels, validate both live and
installed UI payloads, then replace the selected ISO only on success. Preserve
existing model, browser and toolchain payloads; do not run a full build. Explain
incompatible cached components instead of silently replacing them. Do not imply that updating an ISO updates
an already installed VM. Use existing Studio controls and status colors.

The approved `assets/ui-design-kit/default/eclipse-precision-v1-reference.png`
owns all default surface artwork. Existing authored geometry, wording and assets
remain editable; switching sections must not reset them to a factory template.

## Editable surface recipes

### Quiet editing chrome and discoverable artwork

Keep scene backgrounds and mastheads first in a **Scene & Header** layer group;
list other authored elements under **Content**, including hidden and locked layers.
Search by layer name, identifier or image asset. The layer list and property
inspector share a draggable vertical split so neither hides the other's controls.
Selecting a locked layer reveals its properties and explicit unlock action.

Reuse existing Studio controls: keep Marquee, Add and Save Section prominent,
use compact Undo/Redo icons, and collect grid, snapping and console layouts in
Canvas. Put validation and publishing in the Save menu. Preview options and
simulation diagnostics are collapsed by default; rendering failures remain
visible and offer details. No new artwork or alternate surface recipe is needed.

The kit lacked saved material/geometry choices. Surface Styles adds six shared
recipes: window frame, window background, panel/dialog, primary action, secondary
action and field. Reuse the approved atlas for reflections, edge lighting and
interaction states. Glass keeps those highlights; matte removes reflective
shading. Authored is the unchanged default. Square and rounded geometry are
explicit choices with antialiased coverage, never per-app hand-drawn frames.
Fill and rim colors are independent, including transparent overrides. Show
normal, focused and pressed samples before saving. Per-screen layers can override
material/corners/radius and retain explicit colors. Global window chrome remains
OS-owned. Every choice participates in undo, save, reopen and ISO publication.

## Theme library

Use the existing Studio panel, control, type and spacing recipes; do not invent
new artwork. Themes is a UIKit workspace with a scrollable card library, an
inline name field and explicit Apply, Save New, Update, Rename, Import and Export
actions. Cards show the native surface artwork with that theme's actual palette
and material choices. Presets are read-only; custom themes can be duplicated or
moved to the theme trash. Applying is one undoable draft change, never an
implicit save or ISO publication. Exported bundles contain the versioned UIKit
document and content-addressed image overrides, so imports cannot reference
arbitrary filesystem paths. Keep errors visible and preserve the active draft
on a rejected import. Screen-specific overrides remain independent and explicit.
Each screen color can explicitly bind to a semantic theme role or retain a
custom RGBA value. Switching to a binding never destroys the custom value.
Compilation resolves bindings once to the exact native RGBA bytes; preview
uses that same compiler with the unsaved palette. No heuristic color matching.

## Canonical UIKit tokens

InfinityStudio and every installer or post-install configuration template use
the same semantic Default Dark tokens unless the user deliberately overrides a
saved style or screen element.

- Canvas `#090F26`; panel `#101B3A` at 90%; raised panel `#1A2E63` at 94%.
- Border `#86C7FF`; primary text `#FFFFFF`; secondary text `#BEBEBE`.
- Accent `#3E6FFF`; bright accent `#86C7FF`; focus `#FFFFFF`.
- Primary action base `#3E6FFF`; secondary action and field base `#101B3A`.
- Borders use the approved layered silver-blue artwork, tinted only by explicit user overrides.
- Spacing scale: 4, 8, 12, 16, 20, 24, 32, 48. Panel content uses a 32 px
  outer inset and a 16 px control gutter at the 1536 x 1024 reference size.
- Corner radii: 4 px detail, 10 px control, 16 px panel, 24 px hero.
- Control heights: 34 px toolbar, 40 px compact, 48 px standard, 56 px hero.
- Typography: Inter for body and controls, Inter Semibold for headings and
  actions, JetBrains Mono only for technical values. Body text never drops
  below 16 px at the reference size.

Buttons are role-driven components rather than freely styled rectangles.
Primary and secondary actions have equal height, centered labels, a 12 px gap,
and at least a 40 px interactive target. Text fields share the standard action
height, control radius, and 16 px internal leading gutter. Focus changes the
edge and highlight, never the geometry.

## Window topology

- Toolbar (64 pt): identity, undo/redo, insertion, grid controls, validation,
  and save. Every labeled action is 32 pt high; icon-only actions are square.
- Left rail (248 pt): independently expandable Installation Screens and OS
  Configuration Screens collections. Every row is selectable and shows its
  flow-local number, title, and layer count. The selected collection owns all
  add, duplicate, reorder, rename, remove, reset, and canvas operations.
- Center stage: scalable 1000 × 1000 normalized artboard on a graphite pasteboard.
- Right inspector (300 pt): element identity, geometry, content, appearance,
  image source, lock state, and layer ordering.
- Bottom status bar (34 pt): zoom, selected element geometry, snap state, validation,
  and output path.

## Canvas language

- Grid: 10-unit minor lines, 50-unit major lines, emphasized center axes.
- Selection: 2 pt cyan outline, eight white/cyan handles, translucent bounds.
- Snapping: live cyan guides and a compact coordinate badge.
- Panels: semantic Default Dark glass with the canonical 16 px radius and
  border; nested surfaces use the raised-panel token.
- Console: darker monospaced surface with inset rim.
- Images: aspect-fit preview over a checker-free obsidian backing.
- Text: direct double-click editing plus full inspector editing.
- Buttons: reusable primary and secondary controls; unlock their appearance,
  labels and geometry for editing. Required runtime actions remain protected
  from removal. Lock state is visible through the selection overlay.

## Post-install composition

The 1536 x 1024 reference composition uses a 521 x 754 px configuration card,
32 px card insets, 16 px nested content gutters, 48 px fields and actions, and
12 px between adjacent actions. All eight screens retain the same card, header,
progress, title, body, content, and navigation anchors. Dynamic input and
network state overlays exactly replace their design-time preview surfaces.

## Date and Time component parity

The Date & Time kit is represented by semantic, independently selectable
layers: section label, date field, time field, time-zone selector, UTC offset
badge, time-zone map, metadata, Back, and Primary. Fields use the field token
with a top-edge glass lift; the badge uses bright cyan metadata; the map adds a
bounded longitude band and luminous marker. Hover, focus, and pressed states
change fill lift, edge luminance, and glow without changing element geometry.

The macOS canvas and kernel framebuffer compositor resolve the same roles and
authored base colors. The map remains a built-in runtime asset, so saving the
project records its editable geometry and crop without duplicating the bitmap
inside the runtime template. Installer and installed-system images package the
same generated `.iuit` artifact.

## Interaction contract

- Click selects; Shift-click toggles selection.
- Drag moves; eight handles resize; arrow keys nudge; Shift-arrow moves by grid.
- Grid snapping is on by default and can be disabled from the toolbar.
- Command-Z/Shift-Command-Z undo and redo all document mutations.
- Command-S validates and atomically saves editable and runtime templates for
  both the installer and post-install OS configuration flow.
- Delete removes editable elements only. Command-D duplicates editable elements.
- New Panel, Text, Image, and Console commands create real elements at the
  current viewport center.

## Template contract

The installer editable document is versioned JSON
(`installer-screens.infinityui`) and its compact runtime export is
`installer-screens.iuit`. The OS configuration collection uses the parallel
`configuration-screens.infinityui` and `configuration-screens.iuit` artifacts.
Both formats retain normalized integer geometry, bounded UTF-8 strings, stable
element IDs, kind/role enums, z-order, appearance tokens, image asset
identifiers, crop state, and lock flags. The installer contains eleven factory
screens; OS configuration contains eight factory screens matching the actual
first-boot state machine. Every screen retains one locked Back action and one
locked Primary action; all non-button layers remain unlockable and editable.

The InfinityOS kernel parses the embedded runtime template through a typed API.
Invalid data, an unknown version, missing screens, malformed rectangles, or a
changed button record rejects the saved template as a unit and selects the
compiled gold-standard fallback.

## Accessibility

All toolbar actions have labels and keyboard equivalents. Inspector controls use
native labels, fields, steppers, toggles, and color wells. Selection is conveyed
by outline and label, not color alone. Minimum hit targets are 28 pt and canvas
handles are at least 10 pt onscreen.
