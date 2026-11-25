# Task tray — glass hover and sliding selection

Reference: supplied 9.36-second Glassy Sidebar clip, contact sheet `reference.png`.
Reuse the selected UIKit Panel recipe unchanged: no substituted canvas, tint,
opacity, texture or blur. Existing theme icons and fonts remain authoritative.

- Collapsed rail retains its saved desktop anchor and drag handle.
- Pointer entry expands inward over 240 ms; exit collapses over 240 ms.
- Compact app icons retain their size and vertical position during expansion; the expanded action menu replaces the compact list.
- App icons identify minimized apps in the collapsed rail; expansion reveals the selected app name and its action menu. Expanded labeled rows are actions for the selected app: Show, Maximize, Close, Force Kill, Show in Task Browser.
- A pale selection capsule travels between rows with 220 ms leading and 320 ms trailing motion, joined to the trailing
  pale action spine with concave shoulders as in the clip.
- Close uses the existing unsaved-work guard. Force Kill uses the exact registered task and bypasses that guard deliberately. Show in Task Browser selects that task. Right-click and keyboard menus expose the same actions.
- Rapid reversals retarget from the displayed position, never restart from zero.
- Reduced motion commits endpoints immediately. Repaint old and new coverage,
  never leave expanded pixels behind. No allocations or service calls in paint.

Acceptance: production-rendered collapsed, expansion midpoint, expanded,
row transition, menu, collapse and both docking sides; hit-test stable identities,
scroll overflow, reduced motion and incremental/full-frame pixel equivalence.

## Verification evidence

Production renderer: `builds/validation/task-tray-reference/actions.png`.
The panel uses the unchanged selected Panel recipe; action selection adds the
pale spine and sliding capsule. Geometry/motion checks: 48 passed. The production
partial-frame harness matches complete redraw pixels across its resolution and
docking cases. These checks do not constitute installed-VM acceptance or an ISO
publication claim.

## Reference correction: connected bubble (2026-11-23)

Failed comparison: the first implementation used a detached pill, a 12px spine,
late abrupt action-list reveal, and no expanding per-row hover bubble. Those are
not the supplied motion. The reusable correction uses a 0.62-base-width white
spine, 0.46-base-height rows, concave 0.18-base shoulders joining the selection
to the spine, and a pale ice-blue hover lens growing across the row over 220ms.
The selection stretches during travel with independently eased leading/trailing
edges. White is #F8FAFF; hover lens is #D6E9FF with translucent coverage; selected
labels are #152B41. Panel canvas sampling remains exactly the selected UIKit
material. Normal rows have a soft glass highlight and no separate outline.
The same component is rendered at intermediate times, reversal, and rest; not
just endpoint screenshots. Reference motion crops are extracted from the supplied
video rather than replaced with newly imagined artwork.

Correction evidence: `builds/validation/task-tray-bubble/reference-hover.png`
and `rendered-motion.png` compare source-video frames with the production painter.
The 11-frame transition test and partial/full damage equivalence checks pass.
Labels use size-controlled native font rendering and are recolored only where
covered by the selection. These are native-renderer checks, not installed-VM
or literal pixel identity claims: action copy/icons and the user-selected panel
material intentionally remain InfinityOS components.

## Hover/selection separation correction

The reference keeps its white selection stationary during pointer hover. Hover
expands a separate translucent glass lens; a click changes the selected row.
The controller and painter now maintain these as independent states. Normal
rows use vertical glass shading instead of a flat fill. Pointer movement into
row gaps clears the hover lens without clearing selection.

Verification: 48 behavioral checks passed, including stationary selection during
hover and reduced-motion endpoints. Both native animation-frame and partial/full
repaint comparisons passed. Updated comparison artifacts are in
`builds/validation/task-tray-bubble/`. Visual acceptance remains open: the current
icon treatment and glass shading are not an exact reference match. No updated
ISO is claimed for this correction.

## Accepted motion refinement

Keep the accepted hover/selection timing. Center the existing app icon below the
app name, at two-thirds of the base header height, sized two-fifths of that height.
Do not repaint an invisible context menu during hover. The expansion damage
envelope follows the actual three-base rail width, including edge padding.

Selection and spine use shared pale glass at 112/255 alpha, preserving the
underlying canvas. Native preview: `builds/validation/task-tray-bubble/app-icon-glass.gif`.
49 behavioral checks and both targeted production-painter checks pass.
Repaint reduction is verified structurally; installed-VM frame rate is not measured.

Open-menu responsiveness: start hover timing on pointer delivery, settle the hover
lens in 140 ms, and repaint only the action-row band once rail expansion is
complete. Selection travel and rail expansion retain their accepted durations.

## Expansion endpoint and hover repaint correction

Full-height expansion damage persists until composition consumes its last frame;
only subsequent stationary-rail hover frames use the action band. This prevents
spine seams at the band's upper and lower boundaries. State and animation damage
are merged before composition, avoiding two paints of the same tray per frame.
Header artwork outside the damage clip is skipped. The selected panel material
is unchanged; selection glass uses 112/255 pale coverage.

The timed production-painter regression compares each 16 ms expansion/rapid-hover
frame against complete composition. Endpoint, partial-frame and motion checks pass.
Reference render: `builds/validation/task-tray-bubble/endpoint-fixed.png`.
Installed-VM latency has not been measured by these host renderer checks.

## Direct hover feedback

Tray motion remains eligible during busy interactive frames instead of sharing the
decoration deferral gate. Expansion, row exit, and row entry clocks start on
pointer delivery. Hover settles in 90 ms (previously 140 ms), reaches half strength
within 45 ms, and reverses continuously. Rail geometry, glass material, selection
travel, and reduced-motion endpoints remain unchanged.

## Independent Studio material selection

Studio's **Task Tray** toolbar chooser stores a seventh UIKit surface recipe.
Existing six-recipe documents inherit their global Panel material without changes.
Choices reuse Sapphire Glass, Smoked Glass, Frosted Ice, Satin Blue, Graphite Matte,
and Minimal Blue from the existing kit, with rendered panel previews. This changes
only the rail substrate; task actions, glyphs, geometry, translucent hover bubble,
and the 90 ms response contract stay unchanged. Reset restores global Panel inheritance.
The recipe is packaged in the existing UIKit style payload for both live and installed boots.
