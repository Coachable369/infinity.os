# InfinityOS Installer Wizard IDesign Kit

## Gold-standard composition

The Infinity Pool explanation screen is the geometry reference for the complete
installer. At every step the photographic background, centered InfinityOS
masthead, outer console frame, header divider, navigation rail, button row, and
keyboard-help footer remain fixed. Only the title, content region, step number,
and action labels change.

At the normalized 1000 x 1000 layout scale:

- Masthead: centered at `y=0`, `h=300`; width is the smaller of `3 x height`
  and `820`, preserving the artwork ratio and gold-standard scale.
- Console: `x=20`, `y=320`, `w=960`, `h=660`.
- Header baseline: `y=347`; divider: `y=377`.
- Content safe area: `x=38..962`, `y=396..800`.
- Back action: `x=135`, `y=820`, `w=350`, `h=55`.
- Primary action: `x=510`, `y=820`, `w=350`, `h=55`.
- Footer rail: `y=900`; keyboard hint baseline: `y=935`.

The masthead is a fixed symmetrical lockup. No page may substitute a taller,
narrower, shifted, or page-specific logo treatment.

## Panel recipes

- Outer console: near-black translucent glass, nested blue hairlines, cyan
  corner brackets, restrained top sheen.
- Header: semibold cool-white page title at left and cyan `GUIDED SETUP` at
  right, aligned to one baseline.
- Content: use a balanced two-column composition for explanatory screens and a
  centered hero card for status screens. Content must stay inside the safe area
  and preserve at least 14 normalized units between peer cards.
- Navigation: equal-width paired controls with one visual hierarchy expressed
  through focus, hover, and the primary label—not through different geometry.
- Typography: 32 px semibold for content headlines, 24 px regular/semibold for
  primary installer copy, and 19 px for labels and supporting details.
- States: cyan communicates selection and forward motion; warm coral is
  reserved for destructive warnings; muted blue-gray communicates secondary
  information.

## Screen composition

- Welcome: concise promise and mesh hero above two evenly weighted feature
  rows; no content may overlap the shared navigation row.
- Infinity Pool: explanatory cards and topology diagram remain the reference
  balance.
- Disk discovery and disk review: device facts form a readable summary card;
  supporting imagery and benefits occupy the opposite column.
- Date and time: controls and world map share equal visual weight with aligned
  top and bottom edges.
- Review and confirmation: the immutable plan uses grouped summary cards; the
  destructive confirmation remains a centered overlay without moving the
  underlying console.
- Progress, completion, failure, and help: use centered bounded content within
  the same console and masthead geometry.

## Acceptance

All eleven installer states must resolve to identical outer-console and
masthead rectangles at a given display size. Both rectangles must be centered,
contained by the display, non-overlapping, and ordered masthead then console.
The content safe area, navigation controls, and footer must remain inside the
console. Geometry is verified through the typed layout API; VM framebuffer
captures provide the visual review evidence.
