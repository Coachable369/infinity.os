# Spatial desktop implementation checklist

Status: design, motion and retained-transform foundations only; not a delivered
desktop feature set.

## Authoritative interaction scope

1. Holographic Workspace: live retained-window overview, activity islands, direct
   selection and continuous zoom. No screenshots posing as interactive windows.
2. Worldshift: named environments with app/document/layout identity, safe switching
   without losing dirty buffers, durable per-user state, and a reversible transition.
3. Gravity Well: explicit gathering of references to existing files, animated
   preview and drop confirmation. Never relocate or delete originals implicitly.
4. Matter Shelf: explicitly collected text/file references, accessible ribbon,
   drag/drop and keyboard alternatives. No ambient capture of passwords or clipboard.
5. Constellations: user-authored durable relationships between real objects,
   navigable nodes and edges; stale/deleted references handled explicitly.

## Visual recipe

`idesign-kit.png` is an AI-generated design reference, not runtime or proof.
Retain InfinityOS's installed typefaces and selected icon pack. Do not copy the
generated board's incidental Apple/Finder marks, invented font name, extra slogans,
or traffic-light window controls. Retain the native accessible window controls.
Use midnight glass, restrained cyan edges, 8-unit spacing, 16-unit internal padding,
24-unit group gutters, strong text contrast and a distinct keyboard-focus outline.

## Motion acceptance

- Overview zoom: 240 ms; environment transition: 320 ms; collection settle: 180 ms;
  shelf expansion: 160 ms; connection emphasis: 120 ms.
- Motion samples elapsed monotonic time, not frame count. Delayed frames skip
  forward rather than extending duration. Interruption starts at the displayed pose.
- Reduced motion settles immediately. Drag tracking itself has no artificial lag.
- Reuse persistent surfaces; no app re-render solely for transforms. Damage covers
  old/new effects bounds. No idle animation loop or full-frame pointer repaint.
- Never delay input until effects finish; Escape cancels transient interactions.

## Remaining delivery gates

Native controls, permission-checked object operations, persistence, actual compositor
integration, keyboard/drag behavior, pixel review, frame-time measurements, builds,
fresh-install parity and ISO-detached installed testing remain required for all five.
The motion module alone does not satisfy any complete-feature acceptance claim.

## Implemented foundation evidence

- `ui::app_launcher::motion` provides elapsed-time fixed-point interpolation,
  continuous retargeting, reduced-motion settling and exact endpoints. Two tests
  cover skipped frames, interruption, full-range coordinates and completion.
- Existing launcher presentation now eases its transition; a zero elapsed-time
  tick cannot advance it. Existing scrolling/drag behavior is preserved.
- `SoftwareCompositor::compose_transformed` samples retained surfaces into
  destination bounds with premultiplied-alpha bilinear filtering. Ordinary
  composition retains its previous unscaled path. No temporary pixel buffers
  or application calls are introduced.
- Pixel tests verify transparent-edge color, untouched pixels outside damage,
  unchanged source content, bounded composition counts and trusted-layer rejection.
- The five experiences are not wired into the desktop yet. No installed frame-time
  result or polished-screen acceptance is claimed by these foundation tests.
- The default ARM64 reprovision ISO was rebuilt successfully on September 22.
  Both architecture installed kernels link; byte-identical ARM64 installer kernel
  payload parity passes. The running VM was inspected at login and was not updated.
