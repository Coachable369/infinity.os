# Semantic Compositor

## Architecture

InfinityOS composes retained `SurfaceFrame` layers into a caller-owned software
back buffer. A frame is not visible while layers are being cleared or blended.
Only after every damaged rectangle is complete does `present` copy those
rectangles to the front buffer. Layers are traversed by policy z-class rather
than caller order.

The compositor accepts bounded semantic damage from `DamageTracker`. Damage is
clipped to the display, intersected with each surface, and presented region by
region. Trusted and cursor z-classes reject ordinary application surfaces.
Counters expose composed/presented frames and pixels, rejected layers, and
damage collapses as structured state.

## Failure and resource behavior

- Caller-owned buffers make allocation and budget policy explicit.
- Short front, back, or surface buffers fail before presentation.
- A rejected layer leaves the pending frame unavailable to `present`.
- Surface memory reservations and counts are bounded.
- Damage storms collapse to one safe union rather than allocate.
- Adaptive quality disables optional blur/shadow/animation work under sustained
  deadline or memory pressure; interaction and content remain enabled.

## Status

- Software back-buffer composition and atomic damage presentation: **TESTED**.
- Semantic damage, clipping, collapse, diagnostics, and privileged z-order:
  **TESTED**.
- Architecture-neutral compositor ABI and IOP operation IDs: **IMPLEMENTED**.
- Live desktop window movement uses bounded back-buffer relocation and exposure
  repair: **IMPLEMENTED, COMPILE-TESTED**.
- Bootstrap, installer, onboarding, authentication, and desktop primitives draw
  into one bounded software back buffer and expose only completed dirty regions:
  **IMPLEMENTED, COMPILE-TESTED**. Displays above the current 2560x1600 storage
  ceiling use the documented direct-framebuffer fallback and do not claim atomic
  presentation.
- GPU composition: **PLANNED**.

The live presentation adapter retains at most eight disjoint dirty rectangles.
Touching regions merge; excess fragmentation collapses to one conservative
union. Full-frame presentation is reserved for initial composition and explicit
recovery. GOP has no page flip or vsync contract, so scanout tearing cannot yet
be ruled out even though clear/draw intermediate states are hidden.
