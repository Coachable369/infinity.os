# InfinityOS desktop polish — IDesign Kit v1

Authority: `assets/ui-design-kit/default/desktop-refinement-reference-v1.png`
and `assets/ui-design-kit/default/desktop-refinement.md`.

This pass reuses the default kit's authored wallpaper, icon atlases, Inter UI
atlases, smoked navy glass, cyan focus rim, violet secondary light and 8px
spacing rhythm. The kit already supplies every required visual asset, so no new
bitmap or icon family is introduced.

## Ten-polish implementation checklist

1. Add a restrained edge vignette that improves foreground legibility without
   obscuring the authored wallpaper.
2. Give the top rail a crisp inner highlight and lower cyan edge.
3. Place the InfinityOS identity in a quiet home capsule with a divider.
4. Present `LOCAL | PRIVATE` as a compact privacy badge with an honest online
   status light.
5. Give status icons consistent circular wells and a bounded hover halo.
6. Refine the clock well with inset depth and a live-status dot.
7. Add a layered dock shadow to separate it from bright wallpaper areas.
8. Add a restrained cyan-to-violet dock rim and top specular line.
9. Lift and illuminate only the hovered dock item, with a readable label above
   it; pointer movement across empty gaps must not select an item.
10. Derive dock running indicators from actual visible native windows instead
    of hard-coded icons, with a divider before spatial/system destinations.

## Interaction and motion

Hover changes geometry by at most 3 logical pixels and never changes hit
regions. It triggers a repaint only when the semantic hover target changes.
There are no idle loops. Keyboard and pointer activation behavior is unchanged,
and reduced-motion therefore needs no special fallback for this pass.

## Review targets

- Maintain the reference board's hierarchy: wallpaper first, active work second,
  shell chrome third.
- Avoid large bloom, icy materials, oversized controls or low-contrast text.
- Keep all geometry shared across x86_64 and aarch64.
- Validate target boundaries, active-state derivation and hover transitions via
  typed behavioral tests, then capture the installed desktop for visual review.
