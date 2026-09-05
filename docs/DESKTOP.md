# Desktop and Window Lifecycle

The Window Server owns window IDs, surface associations, owner contexts, current
and previous geometry, restore bounds, minimum size, state, focus, modal parent,
pointer capture, and z-order. Supported states are Normal, Minimized, Maximized,
and Fullscreen.

Moving and resizing preserve old and new bounds for semantic damage. The live
Home window drag path now selects framebuffer relocation plus newly exposed
wallpaper repair; it no longer forces a complete desktop redraw for each motion
report. Maximize, resize, visibility, content, and structural screen changes
still request a structural repaint.

The Window Server publishes a bounded typed lifecycle queue for create, destroy,
focus, move, resize, state, capture, and context-failure transitions. The
runtime reserves stable IOP IDs for surface/window operations and IEF type IDs
for lifecycle bridging. Queue pressure drops the oldest notification and records
the count; authoritative window state remains queryable.

## Restart behavior

Window and surface state live in the runtime, not in the rendering backend.
Recreating the compositor can therefore recompose retained state. An application
context crash removes only its windows and surfaces. Unrelated windows survive.

## Status

- Ownership, move/resize constraints, states, focus, modal policy, capture,
  typed lifecycle queue, and crash cleanup: **TESTED**.
- Capability-validated surface create/publish and window create/move runtime
  entry points: **TESTED**.
- Live Home window bounded movement path: **IMPLEMENTED, HOST-BEHAVIOR-TESTED,
  BUT NOT ACCEPTED IN DIRECT VIRTUALBOX AUTOMATION**.
- General application-facing window SDK: **PLANNED**.
