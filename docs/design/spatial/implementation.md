# Spatial desktop implementation checklist

Status: native workflows are wired into the installed shell. This remains an
incomplete milestone until the interaction/visual gates below are satisfied.

## September 22 native integration

- Open with **Ctrl+Shift+K**, or top-bar Search → Spatial desktop.
- Holographic view samples the real retained surfaces for the five built-in app
  classes, with aspect-preserving bilinear zoom (wheel or +/-). No generated
  desktop screenshots are used. These are captured previews, not continuously
  refreshed live applications, and separate File Navigator instances are not yet
  independently represented.
- Four named Worldshift environments retain window layouts, navigator location,
  and saved editor paths. In-session editor buffers, cursor, selection and undo
  history are kept separately. Unsaved buffers are **not** saved across reboot.
- Gravity Well collects references to the current File Navigator selection.
  Dragging changes reference placement; dropping over one of four collection
  buttons changes membership. Source files are not moved or deleted.
- Matter Shelf accepts explicit editor selections or manually entered clippings,
  and inserts them into the existing editor without clearing its document.
- Constellations supports reference positioning, symmetric link/unlink, opening
  existing references, and safe unlinking when a reference is removed. A moved
  source is reported as stale rather than resolving to another file by accident.
- Metadata is bounded to 16 references, four environments, 96-byte paths,
  32-byte item labels and 192-byte printable-ASCII clippings. An 8 KiB versioned
  checkpoint is bound to the active authenticated owner. Generic object reads,
  copies and writes cannot expose or alter its contents through an alias.
- Opening/closing fades and lifts the panel using elapsed time; overview zoom is
  eased. Reduced motion is persisted per user. Ordinary pointer movement paints
  only the cursor; dragging clips to old/new bounds (or the connected graph area).
  Idle ticks do not repaint the overlay; caret ticks repaint the text-field area.

## Current verification and remaining work

Passed: four state/geometry tests; exact native backdrop pixel test; native object
store persistence/remount/alias tests; InfinityUI and launcher regression suites;
12 HTTP/TCP library tests; ARM64 and x86_64 installed kernel builds. The rebuilt
Hermes/Ministral ISO passes model and byte-identical installed-kernel payload
parity. The updated `infinityos-4` disk boots to sign-in with the ISO detached.

Installed visual/input acceptance is blocked at sign-in: synthetic input did not
work in the previous verification attempt; an unlocked desktop was requested.
No measured frame-time or polished-screen acceptance is claimed.

Still required for the original complete feature scope: live invalidation and
independent multi-window overview; the reversible world-switch transition;
collection settling/preview confirmation; a compact Matter Shelf ribbon and
cross-application drag/drop; final screenshot comparison, keyboard/pointer QA,
and measured installed frame times. The current panel is not a substitute for
those remaining workflows.

Current ISO: `build/hermes/InfinityOS-Hermes-Qwen-aarch64.iso` (legacy filename,
contains Hermes + Ministral, not Qwen). Staging: `build/hermes/payload.7ZdQm4`.
VM disk: `infinityos-4-spatial-20260922.vdi`; the previous `infinityos-4-updated.vdi`
is retained for rollback. Only the temporary RAW conversion was deleted.

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

## Original delivery gates

Native controls, permission-checked object operations, persistence, actual compositor
integration, keyboard/drag behavior, pixel review, frame-time measurements, builds,
fresh-install parity and ISO-detached installed testing are required for all five.
The motion module alone does not satisfy any complete-feature acceptance claim.

## Prior foundation evidence (before native integration)

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
