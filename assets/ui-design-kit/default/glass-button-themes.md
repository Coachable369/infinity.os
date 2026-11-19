# Shared button styles

Reuse the shared Precision atlas, Inter type and 140ms focus transition.
The six button cells are regenerated from `glass-buttons-v2-reference.png`.
Frames, panels, fields and progress markers retain their previous artwork.

## Regenerated glass artwork, November 19, 2026

The built-in image generator produced the reference using the approved
Eclipse Precision board as a style example. Prompt: six blank optical-glass
buttons in two columns (sapphire primary, smoked navy secondary) and three
rows (normal, hover, pressed), matching geometry, 250x52 logical footprint,
modest rounded corners, icy-silver rim, soft upper reflection, brighter hover,
darker inset pressed state, no text or glyphs, frontal orthographic view.

`tools/build-precision-artwork.py` extracts the six states into the existing
256px nine-slice cells with independently translucent fill and rim. The
reference sheet's background and gutters are not runtime artwork. Labels
remain live Inter text; hit rectangles and focus timing are unchanged.

Studio Surface Styles presents five 250×52 production-sampled buttons above
the component grid. `button-styles-v2-reference.png` is the generated design board:

| Style | Material | Corner radius | Fill ARGB |
| --- | --- | --- | --- |
| Classic Sapphire Glass | Glass | 10 | E8204DAD |
| Smoked Glass | Glass | 10 | D8101B3A |
| Frosted Ice | Glass | 10 | C8447394 |
| Satin Blue | Matte | 8 | FF3159A6 |
| Minimal Blue | Flat | 6 | FF245CD8 |

Generation prompt: an aligned InfinityOS reusable button board on midnight navy,
five labeled rows and Normal/Hover/Pressed columns; traditional sapphire and
smoked glass with fine icy rims and upper reflections, soft frosted ice glass,
opaque satin cobalt, and restrained flat blue. White Inter-like Continue labels.
Hover increases edge light without changing geometry; pressed darkens. Reference
the approved Eclipse board and regenerated glass family. These are shared native
material, tint and corner variants, not separately baked controls.

All presets use the approved icy-silver rim. Glass reflections
are sampled from the current normal/focus/pressed artwork, neutralized relative
to that state's center, then applied after tint. The fill transmits the scene;
the rim remains independently opaque. Satin and Minimal use opaque fills through
the existing matte and flat samplers. The authored material remains available.

A preset changes shared primary and secondary recipes atomically, with undo.
The selected Button inspector also exposes a dropdown immediately beneath its
header. Each choice includes a 112x32 native-rendered thumbnail and selection
indicator. Selecting a preset stores its material, corners, radius, fill and rim
on that element, clears color-token bindings, and preserves its action and frame.
Use shared UIKit style restores inheritance. Locked layers must be unlocked;
custom recipes remain labeled Custom appearance. The dropdown is a reusable
popover using the kit panel and typography, with click-outside dismissal.
Buttons whose material is Kit default inherit the recipe including its colors.
An explicit layer material uses the layer's own colors instead. Other surface
families retain their prior independent color contract. Existing authored
documents are not rewritten merely by opening Studio.

## Persistent cloudy glass texture

The user-approved cloudy appearance is part of the glass surface itself.
`glass-button-clouds-v1.rgb` is a 256×72 RGB texture extracted from the existing
UIKit preview wallpaper by `tools/build-button-cloud-texture.py`. Its identical
Studio resource copy is an active packaging input, verified byte-for-byte.
Native and Studio samplers bilinearly map it over the button interior, composite
at 90/255 opacity (70/255 when pressed), and apply the existing rounded coverage
and independent rim last. Layer opacity still fades the whole control. Matte,
flat, fields and panels do not sample this texture. No runtime allocation is
needed. The texture is embedded in both live and installed kernels.

Button dropdown previews use a plain navy backdrop, so cloud detail visible in
the preview is saved artwork, not an unrelated wallpaper. Real backdrops remain
visible through the translucent result. Keyboard focus retains the brighter rim.

Preview and native integer samplers must match byte-for-byte. Validate preset
distinctness, translucency, corner coverage, serialization and undo. The saved
recipe uses the existing UIKit JSON/style payload and existing ISO-update path.
Runtime and installed-system visual checks remain separate from host parity.

Remaining Studio integration work: login is still an imperative authentication
scene; top-bar placement, menus and hit testing are also imperative. Exposing
either as editable layers requires a shared authored geometry/action contract,
native preview support, installer parity and input-dispatch tests. Do not add
editable-looking screens which the runtime ignores.
