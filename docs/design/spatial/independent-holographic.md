# Independent Holographic Desktop

The existing `idesign-kit.png` remains the visual authority: layered blue glass,
soft reflections, luminous restrained rims, readable type, and real app content.
The production card painter is shared between settled cards and cached animation
sprites. Each card has a distinct app-name pill. Existing installed icon atlases
are reused; no raster image stands in for a running application.

## Interaction

- Command/Super+Tab opens the independent app switcher and advances selection.
- Shift+Tab and Command/Super+Shift+Tab open it and rotate backwards.
- Click a side card to animate it to the front; click the settled front card or
  press Enter to activate its app and retained item. Escape returns unchanged.
- The switcher does not display Spatial Desktop tabs or workspace actions.
- Spatial Desktop's Holographic tab contains instructions and reduced-motion
  configuration, not a second embedded app switcher.
- Both interfaces use a wallpaper-only stage. Windows, desktop shortcuts,
  widgets, dock and top bar remain hidden until dismissal. The original desktop
  snapshot is retained separately for restoration. Background refreshes also
  regenerate only this isolated stage; reveal frames never fade to desktop UI.
- Animation retains the existing 320ms curved, interruption-safe card geometry
  and bounded sprite cache; no app rendering or service queries occur per frame.

## Verification boundaries

Native pixel tests cover isolated reveal pixels, exact desktop restoration,
glass-card painting and untouched exterior regions. Shortcut tests cover both
directions, ordinary Tab, and authentication/surface boundaries. Existing
spatial-state tests cover animated geometry, retargeting, depth hits and activation.
`build/holographic-glass-proof.png` is a production-painter host render inspected
against the kit, not installed-guest proof. Host interception of Command+Tab is
outside guest control; Shift+Tab offers an alternative. Installed VM interaction
and frame-rate measurements remain separate acceptance steps.
