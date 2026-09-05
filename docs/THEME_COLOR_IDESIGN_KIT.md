# InfinityOS Theme Color IDesign Kit

Status: IMPLEMENTED BY THE INSTALLED APPEARANCE SERVICE

## User control

The installed Settings application exposes adjacent Primary and Secondary color
editors under `Themes & Skins`. Each editor is a direct-manipulation HSV surface
with a hue rail, a visible selection marker, and a live preview. Changes preview
immediately and commit once when pointer interaction ends. Primary controls
machine-wide frosted surfaces; Secondary controls the authenticated user's
interactive emphasis color.

## Semantic color contract

Two independently persisted RGB values feed semantic surface roles rather than
isolated paint constants:

| Surface role | Treatment |
| --- | --- |
| Window outline | brightened accent |
| Window/header emphasis | accent at restrained opacity |
| Top navigation bar | dark accent tint beneath translucent glass |
| Dock | dark accent tint beneath translucent glass |
| Widgets/cards | dark accent tint with accent edge |
| Focus/selection | full accent with a bright inner edge |
| Interactive icons | brightened accent |

Text remains neutral white/gray for contrast. Destructive, warning, and
success colors remain semantically fixed and are not recolored by the accent.

## Picker geometry and interaction

- Each picker opens from its own Primary or Secondary row without replacing
  other Settings sections.
- Saturation increases left-to-right; brightness increases bottom-to-top.
- The vertical hue rail spans the full hue wheel.
- Pointer press and drag updates the preview continuously.
- Pointer release persists only the selected color.
- Activating either color row advances through its own accessible preset palette.
- Both selected colors are restored at the next installed-system boot.

## Quality and accessibility

- Markers use a white outer ring and dark inner ring so they remain visible on
  every hue.
- The picker maintains generous gutters and uses the same radius, glass,
  outline, and focus recipes as the installed skin.
- All affected chrome is rendered from semantic roles, allowing future skins
  and accessibility modes to reinterpret the accent without changing apps.
