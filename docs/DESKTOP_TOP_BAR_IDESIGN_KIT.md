# InfinityOS Desktop Top Bar IDesign Kit

Reference target: the supplied 2258 x 80 desktop navigation screenshot.

## Geometry

- Standard-density rail: 38 px high, reduced from 46 px.
- Rail inset: 2 px left/right/top with a 7 px corner radius.
- Brand mark: 36 x 17 px at standard density with transparent breathing room.
- Wordmark: `I N F I N I T Y O S`, regular UI face, cool white, vertically centered.
- Menu and status hit regions use the same shared height as their rendered controls.

## Surface recipe

- Base: nearly black navy glass at high opacity.
- Highlight: restrained blue upper wash and a soft center-right blue bloom.
- Edge: one-pixel steel-blue outline with a quiet cyan lower separator.
- Active state: compact translucent blue capsule contained within the rail.

## Live layers

The generated raster is limited to the illuminated infinity mark. The rail,
wordmark, menus, status icons, clock, hover states, and pull-down panels remain
native framebuffer content with shared renderer/input geometry.

## Acceptance

- The desktop rail is visibly slimmer than the prior 46 px bar.
- The left brand treatment matches the reference hierarchy and blue-white glow.
- Menus, system indicators, clock updates, and pull-down hit targets remain live.
- The icon BMP is embedded in both ISO and fresh-installed kernels.
