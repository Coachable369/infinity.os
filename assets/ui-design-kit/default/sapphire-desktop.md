# Sapphire desktop — authoritative shared controls

The user-supplied `teal-sapphire-desktop-reference.png` defines density, spacing,
surface depth and optical edge softness. The subsequent correction makes
**sapphire dominant**, with teal confined to small status accents. The generated
`sapphire-controls-reference.png` defines the revised control states.

## Reusable contract

- Canvas #091522; surface #0E1C36; elevated surface #172B48.
- Primary sapphire #245AD4, hover #3474EC, pressed #17449E.
- Quiet steel-blue edge #365774; focused sapphire #5995F5.
- Text #F2F6FC, secondary #AEC3DA. Teal #00C8FF is a status accent only.
- Control height 40 logical pixels; compact control 32; radius 8; row radius
  12; padding 12–16; inter-control gap 8; row gap 12. No hover geometry changes.
- Buttons have substantial filled surfaces and restrained reflected light.
  Fields are inset. Dropdowns share button geometry, with a separate trailing
  chevron well. Toggles use a filled pill and a solid pale thumb; sliders use
  a six-pixel rail and 20-pixel thumb with a filled progress segment.
- Keep existing mouse/keyboard state transitions, service authorization and
  saved values. Disabled controls cannot mutate values. Focus is visible.
- Window title/menu/toolbar heights remain fixed during resize. Applications
  use compositor chrome and shared UIKit controls, never independent borders.

## Overrides and packaging

The Sapphire atlas is the default. Explicit Studio Glass/Matte/Flat recipes
continue to use the existing Precision atlas, including persistent Smoked Glass
texture. These remain active supported alternatives, not deprecated assets.
Per-element overrides and their saved geometry must survive this migration.
The same atlas and palette ship in Studio, both kernels and installed UI kit.

## Acceptance checklist

The minimum visual fidelity is **95% for every applied element and state**,
across all applications and OS base surfaces. A whole-screen average is not
an acceptance substitute: blank space or already-correct controls must not
conceal a failing element. Explicit user-authored Studio overrides are compared
against their selected recipe rather than the default Sapphire recipe.

Record reference and native captures at identical size, scale, backdrop, data,
and animation time. Assess each element's silhouette, spacing, typography,
palette/gradient, edge coverage and state treatment separately. Include overlays
and numerical pixel differences where reference crops are comparable. Do not
report an unmeasured subjective impression as a 95% result. Untested controls,
apps or states remain open; functional or build success cannot close them.

1. Native and Studio render identical default and explicit-override pixels.
2. Buttons, fields, panels, toggles and sliders share materials and geometry.
3. Render reference-size controls and Settings; compare fills, softened edges,
   spacing, type baselines, focus, pressed and disabled states.
4. Existing explicit Smoked Glass texture remains visible.
5. Verify packaging/parity; ARM before x86. Installed-runtime acceptance is
   separate from host-renderer evidence and build success.
