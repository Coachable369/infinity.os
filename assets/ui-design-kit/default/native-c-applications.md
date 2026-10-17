# Native C Application UI Kit

This component closes the design-kit gap for generated native C applications.
Applications describe semantic elements through `infinity/uikit.h`; InfinityOS
owns their pixels, typography, input routing, window chrome, and active theme.

## Window recipe

- Use standard native window chrome and the existing resizable edge affordances.
- Default logical canvas: 480 × 420 units; supported range: 320–960 × 240–720.
- Surface: smoked navy glass from the active theme, with the standard soft cyan
  outline and 10-unit corner radius.
- Toolbar caption: `NATIVE C / INFINITY UI KIT` in the compact UI role.
- Content follows the 8-unit spacing rhythm. A calculator uses 20-unit outer
  margins, a 72-unit display, and 56-unit buttons separated by 12 units.

## Reusable semantic elements

- `INFINITY_UI_LABEL`: transparent strong UI text for headings and guidance.
- `INFINITY_UI_DISPLAY`: inset elevated surface with right-aligned primary text.
- `INFINITY_UI_BUTTON`: elevated smoked-navy action with cyan outline and
  centered strong text.

Stable nonzero element IDs are the interaction contract. Applications update an
existing element by submitting the same ID. The OS produces activation and close
events; native applications do not receive framebuffer pointers or style tokens.

## States and ownership

Button visuals inherit default, focused, hover, pressed, and disabled recipes
from the shared kit. The first implementation exposes activation and close to C;
the OS retains hover/focus ownership so future state expansion does not change
application layout code or the ABI.
