# Task tray — sliding selection

Reference: supplied 9.36-second Glassy Sidebar clip, contact sheet `reference.png`.
Reuse the selected UIKit Panel recipe unchanged: no substituted canvas, tint,
opacity, texture or blur. Existing theme icons and fonts remain authoritative.

- Collapsed rail retains its saved desktop anchor and drag handle.
- Pointer entry expands inward over 240 ms; exit collapses over 240 ms.
- Compact app icons retain their size and vertical position during expansion; the expanded action menu replaces the compact list.
- App icons identify minimized apps in the collapsed rail; expansion reveals the selected app name and its action menu. Expanded labeled rows are actions for the selected app: Show, Maximize, Close, Force Kill, Show in Task Browser.
- A pale selection capsule travels between rows in 180 ms, joined to the trailing
  pale action spine with rounded shoulders as in the clip.
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
pale spine and sliding capsule. Geometry/motion checks: 46 passed. The production
partial-frame harness matches complete redraw pixels across its resolution and
docking cases. These checks do not constitute installed-VM acceptance or an ISO
publication claim.
