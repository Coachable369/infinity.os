# Spatial Desktop IDesign Kit v2

## Acceptance target

- Replace the wide strips of equally weighted controls with a compact glass header, segmented navigation, a spacious orbit canvas, four named collection cards, and one centered action dock.
- Keep `Add idea` and `New category` visible. Place destructive and structural actions in the adjacent overflow menu without removing their keyboard or pointer behavior.
- Animate the center orb with three low-alpha expanding halos. The pulse repaints only the orb bounds at roughly 29 FPS, stops under Reduced Motion, and never invalidates the full spatial scene.
- Preserve the existing native dark-glass palette, cyan primary action, cyan/violet/amber/ice collection accents, readable text contrast, and the generated kit's restrained depth.

## Generated reference

`spatial-desktop-idesign-kit-v2.png` is the visual target generated before implementation. It is design documentation; runtime controls remain native, theme-aware, and interactive.

Generation mode: built-in image generation, UI mockup. Prompt: create a world-class InfinityOS Spatial Desktop kit from the supplied current-screen reference, using a single translucent header, compact segmented tabs, a centered animated-orb canvas, four consistent orbit category cards, and a compact action dock with primary, secondary, and overflow states; include component specimens, typography, spacing, hover, selected, disabled, and menu states; polished dark navy glass with restrained cyan and violet accents.

## Interaction map

- Collection cards select the corresponding orbit ring and remain valid drag targets.
- Primary dock actions invoke action indices 0 and 1.
- The overflow disclosure opens four bounded menu rows mapped to action indices 2 through 5.
- Clicking outside an open overflow menu dismisses it while allowing the underlying spatial target to continue normally.
