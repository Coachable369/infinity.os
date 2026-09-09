# Infinity Pool Settings — implementation reference

Reference: [generated IDesign Kit](../assets/design/infinity-pool-idesign-kit-v1.png).
This is design documentation, not a runtime screenshot or installer payload.
Illustrated node names, files and health labels are not measured system state.

Use the existing Settings window, semantic theme colors, opacity, fonts and
controls. Preserve File / Performance header geometry. No mockup bitmap is used
as a panel, control, chart or live-state substitute.

- Frosted navy surfaces, soft blue outlines, smooth existing sans-serif roles.
- 16 px gutters, 12 px gaps, 48 px primary controls; scale using existing UI scale.
- Overview shows measured raw/eligible/reserved capacity and node health.
- Objects are owner-scoped, selectable rows with desired/verified copies.
- Selection exposes identity, current version, policy, placements and healing.
- Temporary / Protected / Critical use the same typed policy operation as Console.
- Loading, unavailable, empty and failure states remain explicit; no fabricated counts.
- Painting reads an immutable projection. Refresh and mutation run outside painting,
  bounded per tick. Clip all content to the scroll viewport and invalidate only damage.
- Compare rendered hierarchy, spacing, label fit, focus, hover, pressed and disabled
  behavior against the reference. Screenshot approval alone is not functional proof.

Generation mode: built-in image generation, new bitmap, design reference only.

## Generation prompt

Create a polished desktop OS design-system board, 1536x1024 landscape, for
InfinityOS distributed storage Settings. Restrained frosted navy glass #0D2238,
soft blue #4DA3FF outlines, white/gray smooth Roboto-style sans-serif, faint night
space behind glass. Compact Settings chrome, left navigation with Storage selected.
Heading Infinity Pool; Overview, Objects, Activity tabs. Health summary, measured
capacity roles Raw capacity / Eligible capacity / Reserved capacity, node rows,
protected objects and selected-object details. Temporary / Protected / Critical
policy controls. Include default / hover / pressed / disabled buttons, empty /
loading / error states, server / storage / object / copy / repair icons, scrollbar,
typography hierarchy, 16 px gutters, 12 px gaps and 48 px controls. Label DESIGN
REFERENCE. No invented capacities, fake charts, tiny labels, overlap or neon overload.
