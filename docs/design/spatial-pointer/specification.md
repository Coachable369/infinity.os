# Spatial and Pointer refinement

Reference: `idesign-kit-v1.png`, generated before implementation with the built-in
image generation tool. Preserve the existing spatial wallpaper, header and dock.

- Appearance > Wallpaper: saved spatial veil RGB and strength, five named color
  presets and a live color preview. Default navy RGB 8/23/42 at 65% strength;
  framebuffer RGB/BGR channel order must not alter that color.
- Collection circles: soft outside aura, glass specular lighting, a vertically
  mirrored fading reflection. Click toggles an inline collection detail well;
  live items open for editing and empty collections offer Add idea. No fake items.
- Devices > Pointer: continuous speed and size sliders, ten selectable generated
  cursors in a responsive gallery, clear selected border and cursor preview.
  Keep Input's other existing settings. Preserve cursor hit point and restore all
  painted pixels when moving, resizing, switching style, or entering text/resize.
- Native shared geometry for painting and hit testing; 16px gutters, 8px gaps,
  readable elided labels, cyan focus state, scrollable inline details.
- Persist through existing user-owned object storage, include assets in fresh
  installer payloads, and verify typed state, pixels and installed asset bytes.

Cursor prompt set: Classic White, Classic Black, Outline, Crystal, Silver,
Comet, Rocket, Leaf, Wand, Pixel. Each is generated separately as one transparent
northwest-pointing cursor with a legible silhouette and restrained highlights.
Runtime derivatives are format/size conversions of these generated PNGs.

Generation mode: built-in ImageGen, one independent image per cursor (no stock
icons or composite-sheet crops). Common prompt: production native mouse sprite,
one cursor, square transparent RGBA, no background/checkerboard/text/labels;
sharp northwest click tip, body extending southeast, full silhouette within
generous margins, crisp readable 32px silhouette, restrained attached glow.
Variants: pearl white/dark outline; charcoal/white outline; hollow ice outline;
faceted cyan crystal; brushed silver bevel; cyan comet/violet tail; silver rocket
with orange exhaust; emerald leaf; golden star wand/navy handle; retro white/cyan
pixel arrow with purple shadow.

`tools/build-cursor-assets.swift` trims transparent margins and converts generated
art into 128px straight-alpha runtime sprites. Click hotspots are opaque pixels
at each upper-left tip; hotspot-anchored sampling keeps the tip visible at small
sizes and display edges. `cursor-contact-sheet.png` shows originals at 128/32px.
