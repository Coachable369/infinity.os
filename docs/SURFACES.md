# Retained Surfaces

`SurfaceRegistry` owns bounded metadata for persistent pixel surfaces. Each
surface has a stable `SurfaceId`, one `ContextId` owner, dimensions, stride,
pixel format, security class, byte reservation, visibility, opacity, optional
window association, pending local damage, and monotonically increasing content
generation. Pixel storage remains caller-owned and is never exposed by registry
inspection.

Surface creation validates geometry and checked byte arithmetic before charging
the global reservation budget. Damage, visibility, opacity, association,
publication, resize, and destruction require the owner. Damage is clipped and
unioned in surface-local coordinates, then cleared by a newer committed
generation. Replayed generations, cross-context access, oversized geometry, and
budget overflow are rejected. Context failure reclaims only that owner's
reservations.

Security classes are Application, System, Trusted, and Cursor. Ordinary creation
cannot request Trusted or Cursor authority. Their corresponding compositor
z-classes are also independently validated during composition.

## Status

- Ownership, damage, visibility, opacity, association, generation, resize,
  budget, cleanup, and privileged-class denial:
  **TESTED**.
- MMU-isolated shared surface pages: **PLANNED**. Current isolation is metadata
  and capability enforced; pixel backing supplied by a service is not yet mapped
  through per-context page capabilities.
