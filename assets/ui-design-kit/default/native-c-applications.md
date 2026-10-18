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

The semantic canvas supports up to 96 elements. This accommodates dense native
tools and complete 8 x 8 game boards while keeping all paint, hit testing, theme,
focus, and typography ownership inside the OS.

## Generated application grid layout

`infinity/app.h` reuses these same label and button elements through `app_grid`,
`app_cell`, and `app_status`; it introduces no separate visual recipe. The compact
layout reserves a 32-unit status label at (20,20), begins buttons at (20,64), and
uses 56-unit square buttons with 12-unit gaps and 20-unit outer margins. The
window is at least 320×240; an 8×8 layout is 572×616. A 9×8 layout is 572×684,
allowing an additional control row within the native canvas. Rows are limited to 9,
columns to 12, and cells to 95 so the status remains within the 96-element cap.
Apps keep stable positive cell IDs and supply their own labels and behavior.

## States and ownership

Button visuals inherit default, focused, hover, pressed, and disabled recipes
from the shared kit. The first implementation exposes activation and close to C;
the OS retains hover/focus ownership so future state expansion does not change
application layout code or the ABI.
