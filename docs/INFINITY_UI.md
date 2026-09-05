# InfinityUI and Milestone 7C

InfinityUI is the semantic client runtime above the Window Server. Applications
and system clients own retained content surfaces; they never own the display
framebuffer or select arbitrary global z values. Window Server owns focus,
pointer capture, modal containment, placement, policy z-classes, cleanup, and
machine-readable window state. The compositor projects that authoritative state
into a bounded software back buffer and presents completed damage regions.

## Lifecycle and authority

Surface identity is stable across screen movement. Creation, commit, resize,
destruction, and metadata inspection use independently revocable capabilities.
Window creation also verifies that the caller owns the referenced surface.
Semantic inspection exposes owner, bounds, state, focus, and permitted type
metadata without exposing pixels. Pixel capture has separate authority.

Window state supports normal, minimized, maximized, and fullscreen placement.
Moves retain the existing surface, damage old and new coverage, and repair only
the newly exposed desktop regions. Title-bar drag holds pointer capture until
release even when the pointer leaves the initial bounds. Owner failure removes
its windows and surface reservations and releases capture; unrelated contexts
remain alive.

## Composition policy

Composition order is Desktop, Normal, Floating, Menu, Modal, Trusted, Cursor.
Ordinary callers cannot create Trusted or Cursor layers. An active secure-input
lease routes keyboard and pointer input exclusively to its trusted context.
Damage is typed, clipped, merged, and bounded; excess fragmentation collapses
conservatively. Trusted and interactive updates have protected presentation
priority, while cosmetic work may defer and must later converge.

Optional blur, shadow, and animation quality can step from Full through
Balanced and Reduced to Safe after measured deadline or memory pressure. Text,
geometry, hit testing, focus, trusted identity, and security decisions never
degrade.

## Recovery and limits

Surface and window registries are fixed-capacity. Surface dimensions and packed
byte calculations are overflow checked against an explicit reservation budget.
The live coherent back buffer supports framebuffer strides through 2560x1600;
larger modes use an explicitly reported direct-present fallback. GOP provides
neither page flipping nor vsync.

Window metadata and surface ownership are separate from pixels, allowing the
Window Server to be reconstructed while surviving clients retain their surface
identities and recommit content. A failed client must redraw after its surfaces
are reclaimed. Seamless preservation of every client pixel across a Window
Server crash is not claimed.

Architecture-neutral surface, compositor, window, damage, secure-input,
capability, IOP, and IEF behavior is **TESTED**. Live desktop relocation is
**IMPLEMENTED, BEHAVIOR-TESTED IN THE HOST HARNESS**, but direct VirtualBox
title-bar relocation remains **IMPLEMENTED BUT NOT ACCEPTED** because two
desktop-automation drag gestures did not move the installed Home window. The
AArch64 ISO install, reboot countdown, installed-disk boot, onboarding, and
desktop presentation are **TESTED IN VIRTUALBOX**. GPU composition, hardware
cursors, page flipping, vsync, and production multi-display behavior are
**PLANNED**.

The earlier general InfinityUI contract remains in `INFINITYUI.md`.
