# Sapphire visual acceptance — open

The required minimum is 95% for **each element and state**, across apps and OS
base features. No aggregate score or successful build substitutes for this gate.
No all-application 95% result has been established.

## Verified foundation

- Shared native surface sampling: 1,122,240 sampled pixels, 29,859,840 adapter
  comparisons, and 39,626 glyph samples across scales 1–4. Includes signed
  clipping, independent fill/edge alpha, active/inactive frame distinction,
  focus interpolation, disabled controls and user accent overrides.
- Studio/native material parity: all 15 focused tests passed, including existing
  explicit button presets and persistent cloudy glass texture.
- Native UIKit integration checks passed. These are behavioral/component
  checks, not installed application screenshot acceptance.
- Python app palette and button/field artwork now derive from the shared kit.

## Remaining visual acceptance

- Live-rendered Themes & Skins still has legacy authored navigation color,
  dense single-column rows, clipped hero caption and proportions different from
  the approved layout. This page has not passed.
- App-specific menus, tabs, scrollbars, text baselines and full control states
  require reference-size screenshots for every affected application.
- Desktop launcher, tray, top bar and widget composition need separate review;
  shared sampler parity cannot establish their visual fidelity.
- Installed ARM and x86 runtime captures, fresh-install asset parity and release
  packaging remain open. Existing published ISOs do not contain this work.

Reference: `sapphire-controls-reference.png` and
`teal-sapphire-desktop-reference.png` (layout only; Sapphire owns the palette).
Current native captures are in `builds/validation/sapphire-native-controls.png`
and `builds/validation/sapphire-themes.png`. These are production painter captures
with host fixture data, not screenshots of an installed VM.
