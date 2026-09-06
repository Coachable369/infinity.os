# InfinityOS Control IDesign Kit

This is the shared sizing and spacing contract for interactive controls in the
installed OS and installer. It supplements the screen-specific design kits and
is authoritative when a screen has no stricter measured geometry.

## Control sizes

| Role | Logical height | Horizontal gutter | Radius | Use |
| --- | ---: | ---: | ---: | --- |
| Toolbar action | 34 | 12 | 8 | Compact icon-and-label actions |
| Compact action | 40 | 16 | 10 | Sheets and inline Settings actions |
| Standard action | 48 | 16 | 12 | Onboarding and ordinary primary actions |
| Hero action | 56 | 20 | 13 | Authentication and installer navigation |

All measurements scale through the active InfinityUI density. Peers in one
action row have equal heights and, when paired, equal widths with a 12-unit gap.
Labels are vertically centered. Icon-and-label controls reserve the same left
and right gutters, with the icon centered in a 20-unit leading slot.

## Content gutters

- Window content uses at least 16 units on both sides.
- Cards and summary rows use 16-unit text gutters.
- Values that precede a disclosure indicator reserve 40 units on the trailing
  edge so text and icon never collide.
- Dialogs use 24-unit outer gutters; paired actions use the same outer gutters.
- A control's visible bounds and hit bounds come from the same shared geometry.

## States

Primary, secondary, focused, hovered, and pressed states preserve the control's
dimensions. State changes may alter fill, outline, and emphasis only; they must
not move adjacent content or change label alignment.

## Review targets

- Text Editor toolbar actions are equal-size 34-unit buttons and remain fully
  visible beside the saved-state label at every supported window size.
- Settings summary rows use matched 16-unit leading gutters and a protected
  trailing disclosure gutter.
- Settings inline actions and editor sheet actions use the compact 40-unit
  recipe.
- Onboarding uses the standard 48-unit recipe.
- Authentication and installer navigation remain on the 55-56-unit hero recipe.
