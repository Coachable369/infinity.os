# Access panel motion

Extension to Eclipse Access for first-boot configuration and login.

- Entrance: 360 ms center-out scale, cubic ease-out, from zero extent to
  the authored panel bounds. Bilinear filtering during motion; exact native
  pixels at rest. The rounded outline expands with the content. No bounce.
- One entrance per screen transition, never per keystroke or focus change.
- Active panel: inward six-pixel blue-white edge light, opacity 24–44/255,
  smooth 3.2-second breathing cycle. No moving highlights or wallpaper flash.
- Existing field focus, button feedback, input geometry and security behavior
  are unchanged. Utility trays are not animated as input panels.
- Reduced motion: immediate full panel, static low-intensity outline.
- Reuse existing Eclipse artwork, typography and surface recipes. No new
  bitmap assets are needed for this presentation-only motion.
- A bounded 8 MiB UI-thread scratch surface is cleared after every paint.
  Panels larger than its capacity appear immediately rather than allocate
  memory or drop content.
