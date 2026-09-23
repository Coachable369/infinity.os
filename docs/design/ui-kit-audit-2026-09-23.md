# OS-wide non-icon UI audit

## Authoritative references

Reuse the existing generated `spatial/idesign-kit.png`,
`spatial/idesign-motion-depth-v2.png` and the screen-specific IDesign Kit documents.
Do not generate replacement icons, inspect icon quality, or alter icon assets.
Screen-specific dimensions override the generic `UI_CONTROLS_IDESIGN_KIT.md`.
Preserve user-authored templates and intentional accessibility themes.

## Correction specification

- Carousel labels: measured semibold text; 16 px horizontal gutters; centered
  vertically and horizontally; compact 44 px glass capsule with subtle inner
  highlight. Do not multiply capsule height by display density while retaining
  unscaled text. Selected and ordinary labels have identical geometry.
- Action labels: center the actual 28 px font cell, not a hard-coded 10 px
  baseline offset. Keep existing interactive bounds and action routing.
- Glass: restrained navy fill, steel-blue hairline and continuous top reflection;
  avoid thick cyan wireframes and abrupt rectangular reflection caps.
- Independent carousel footer: match the carousel glass palette, not a different
  world-specific widget tint. Other theme-aware application surfaces retain the
  selected skin and accessibility settings.

## Inventory and evidence boundary

| Surface | Reference | Review status |
| --- | --- | --- |
| Carousel, name pills, action rail | Spatial kit + motion/depth kit | Pill correction rendered and inspected; footer palette corrected in source; installed interaction pending |
| Shared buttons, fields, focus/hover states | UI Controls | Action, login and onboarding label offsets corrected; full state gallery pending |
| File Navigator: chrome, toolbar, rail, rows, menus | File Navigator | Contract read; complete rendered state review pending |
| Settings: rail, accordions, sliders, dashboards | Settings Window, Pool | Contract read; complete rendered state review pending |
| Launcher: search, tiles, categories, scrolling | App Launcher | Contract read; complete rendered state review pending |
| Top bar: menus, clock, calendar, status dropdowns | Desktop Top Bar | Contract read; complete rendered state review pending |
| Editor and Command Window | Desktop Apps | Contract read; complete rendered state review pending |
| Task Manager | Task Manager | Contract read; complete rendered state review pending |
| Chat, status and weather widgets | AI Chat, widget references | Chat contract read; complete rendered state review pending |
| Running-app drawer and desktop context menu | Minimized Shelf kit | Native renderer tests available; new visual review pending |
| Worldshift, Gravity Wall, Matter Shelf, Constellations | Spatial kits | Complete rendered state review pending |
| Login and onboarding | Desktop/control references | Complete rendered state review pending |
| Bootstrap and installer states | Installer Wizard, Progress | Contracts read; complete rendered state review pending |
| Fatal/emergency surface | Crash Screen | Contract read; intentionally exempt from ordinary glass styling |

Passing compilation or reading a specification is not a visual approval. No
claim that every OS element matches the kit is justified until each screen and
its relevant interaction states has rendered evidence, including fresh-install
verification. Keep animation caching and bounded damage behavior intact.

## Verified correction pass

- Production pill renderer: 16 native painter tests pass, including rendered
  glyph-ink centering within 3 pixels in both axes and byte-identical capsule
  output across 1280x720, 2560x1440 and 3840x2160 density boundaries.
- Existing interrupted-carousel cache, partial-versus-complete composition,
  dragging, and motion tests remain passing. No icon code or assets changed.
- Native card render inspected against the generated reference: compact readable
  label capsules, stable lower gutters, quiet edge and restrained reflection.
  This is a host framebuffer proof, not installed-guest verification.
- Installed AArch64 and x86_64 kernels compile. One initial visibility error was
  corrected by restricting the shared pill method to the bootstrap facade's
  scope, matching the existing shared glass helpers.
- Live VM desktop observed; clicks, drag movement and the File Navigator keyboard
  shortcut did not open requested windows through automation. User was asked to
  leave File Navigator and Settings open. Until those surfaces are visible,
  their complete visual review is pending, not passed.
- The final VM observation was the locked Welcome Back screen; further installed
  application review requires unlocking and opening the requested windows.
- Rebuilt release installers directly in `builds/` using `make x86_64` and
  `sh tools/build-hermes.sh`. ARM installed-kernel byte parity and both model
  payload checks passed. These updated ISOs do not alter the currently running
  installed VM; fresh-install visual verification remains pending.
