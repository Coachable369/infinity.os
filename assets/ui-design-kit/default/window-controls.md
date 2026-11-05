# Circular glass window controls

The approved Eclipse Precision panel and compositor geometry remain authoritative.
`window-controls-circular-v2.png` supplies normal, illuminated-hover and recessed-
pressed plates, left to right. `tools/build-window-controls.py` packs three 96x96
RGBA cells into the stable `window-controls-v1.rgba` runtime/Studio asset contract.
The filename identifies the atlas format, not the artwork revision.

Use the shared top-right minimize, maximize/restore and close bar: 26 logical pixel
squares, 8 pixel gutters and 12 pixel right inset, vertically centered in the title
bar. The circular visible plate retains the generous square hit target. Browser
controls keep their native hit boxes and center this same circular material.

Shared native glyphs remain crisp at every scale. Neutral glass is tinted by the
selected theme while preserving reflections and alpha. Hover lights the rim;
press recesses the center. Close uses the same material as its neighbors. No idle
animation. Studio asset overrides stay authoritative across system and user apps.

Only the topmost visible owner receives hover. Repaint previous/current control
bounds. Existing maximize/restore geometry, minimize state and unsaved-document
close guards remain attached to these controls.

See `window-controls-circular-v2.md` for artwork provenance and acceptance criteria.
Production pixel and state checks passed in build manifest
`20261105T021229665598Z-45697`; normal, hover and pressed states were visually
reviewed. Installed interaction verification remains pending.
