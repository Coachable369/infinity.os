# Shared glass button themes

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

Studio Surface Styles presents three 250×52 production-sampled buttons above
the component grid: Sapphire Glass, Smoked Glass and Ice Glass. Their existing
10px antialiased rounded geometry uses the approved icy-silver rim. Reflections
are sampled from the current normal/focus/pressed artwork, neutralized relative
to that state's center, then applied after tint. The fill transmits the scene;
the rim remains independently opaque. Matte and authored pixels are unchanged.

A preset changes shared primary and secondary recipes atomically, with undo.
Buttons whose material is Kit default inherit the recipe including its colors.
An explicit layer material uses the layer's own colors instead. Other surface
families retain their prior independent color contract. Existing authored
documents are not rewritten merely by opening Studio.

Preview and native integer samplers must match byte-for-byte. Validate preset
distinctness, translucency, corner coverage, serialization and undo. The saved
recipe uses the existing UIKit JSON/style payload and existing ISO-update path.
Runtime and installed-system visual checks remain separate from host parity.

Remaining Studio integration work: login is still an imperative authentication
scene; top-bar placement, menus and hit testing are also imperative. Exposing
either as editable layers requires a shared authored geometry/action contract,
native preview support, installer parity and input-dispatch tests. Do not add
editable-looking screens which the runtime ignores.
