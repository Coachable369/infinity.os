# Crystal Blue Glass Icon Design Kit

## Visual recipe

- Material: transparent cobalt crystal glass with internal refraction.
- Edges: thick rounded electric-cyan bevels with narrow icy-white specular highlights.
- Depth: premium three-dimensional objects in a consistent, slightly elevated three-quarter view.
- Contrast: deep navy cores against bright cyan rims; no gray, purple, or opaque matte backing.
- Background: true RGBA transparency. Clear gutters and no detached glow particles.
- Palette anchors: `#001B6F`, `#01277B`, `#012AA1`, `#0236AB`, `#0348DE`, `#0F5FE1`, `#2281F2`, `#3BB3FB`, `#9DE1FC`.

## Production masters

- `assets/icons/crystal-blue-glass/master-base.png`: 45 semantic system objects in manifest order, 5 columns by 9 rows.
- `assets/icons/crystal-blue-glass/master-actions.png`: 15 semantic actions in manifest order, 5 columns by 3 rows.
- `tools/slice-icon-atlas.py` removes low-alpha fringe, discovers transparent cell boundaries, preserves translucent glass, and derives the 24, 32, 48, 64, 96, 128, and 256 pixel tiers.

## Generation prompts

Base atlas: Create a strict 5 by 9 production OS icon atlas containing the 45 base roles in `assets/icons/manifest.csv` order. Match the supplied InfinityOS folder reference exactly: saturated deep-cobalt transparent crystal, electric-cyan thick luminous bevels, icy-white highlights, internal refraction, clean antialiased edges, consistent scale, and a unified three-quarter perspective. Use genuine transparent RGBA gutters with no labels, matte, checkerboard, floor, duplicate, or cropped object.

Action atlas: Create a strict 5 by 3 production OS action atlas containing the 15 action roles in `assets/icons/manifest.csv` order. Use the same crystal material, palette, bevel, lighting, scale, and perspective. Directional and arithmetic actions must be freestanding sculpted symbols without tiles or backplates; only new-folder may use a folder object. Use genuine transparent RGBA gutters with no labels, matte, checkerboard, duplicate, detached glow particle, or cropped object.
