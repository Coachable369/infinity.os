# InfinityUI

## Contract

InfinityUI is the native architecture-neutral UI runtime. Its retained semantic
tree is independent of framebuffer, input transport, skin, and service policy.
Elements carry stable IDs, parentage, type, logical bounds, state, actions,
accessibility roles, and z-order. Rendering is a projection; rendered pixels and
terminal text are never authoritative state.

Logical geometry uses integer units and explicit 1.00, 1.25, 1.50, and 2.00
scales. The default authentication composition is authored against the supplied
1536×1024 gold standard and proportionally centered on other modes. Wallpaper
uses cover scaling while controls remain inside safe work-area bounds.

## Status

| Capability | Status |
|---|---|
| Geometry, scale, rows/columns, retained elements | TESTED |
| Keyboard focus, pointer hit testing, adaptive acceleration | TESTED |
| Typed async task, stale-response rejection, and cancellation foundation | TESTED at unit boundary |
| Frame clock, damage tracking, bounded cache budget | TESTED at unit boundary |
| Trusted secure-input lease and capability-gated typed clipboard | TESTED at unit boundary |
| Installed-system authentication composition at firmware-selected resolution | TESTED by framebuffer capture |
| Complete GPU compositor and complex text shaping | PLANNED |

The current framebuffer renderer is the migration adapter for boot, installer,
authentication, and desktop surfaces. It consumes the same semantics and uses
localized damage for the cursor and infinity animation. Full per-window backing
surfaces are not claimed yet.

## Default Dark visual system

The production skin uses a restrained blue-black material system, a 4/8-based
spacing rhythm, soft navy shadows, quiet borders, and rounded controls. Arimo
Regular and Arimo Bold atlases provide anti-aliased body, label, control,
heading, title, and display roles; JetBrains Mono remains the Console face. The
source skin also ships a single coherent semantic vector family plus richer
`app-*` vectors for Home, Console, Settings, AI, Voice, Security, and Trash.

First boot, authentication, desktop, system menu, Settings, and Home reuse the
same top bar, glass panel, typography, icon, control, and window recipes. The
first-boot card uses progressive disclosure, a six-stage indicator, inline
validation, and preserved input when navigating backward. The desktop Home
window supports title-bar-only pointer dragging and retains its position for the
session.

Visible pointer targets are derived from `SystemLayout` using the active
framebuffer dimensions. Onboarding fields and buttons, sign-in controls, system
menu rows, Settings sections and values, the Home title bar, and dock items no
longer depend on reference-resolution constants. Hover changes focus without
committing a setting; a click on a Settings navigation row selects that section,
while activation is reserved for its content control.

## Repaint and window movement

Pointer-only updates restore and redraw the cursor sprite. Home-window dragging
restores the old window bounds from the cached desktop artwork and redraws the
window at its new position; it does not invalidate the full framebuffer. The
Window Server independently exposes owner-checked move and resize primitives,
work-area constraints, minimum dimensions, focus, and pointer capture. Complete
surface compositing and resize cursors remain planned.
