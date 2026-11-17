# InfinityCode workspace acceptance — 2026-11-17

Status: incomplete; do not claim exact visual parity or a completed ISO release.

Implemented: fixed menu/toolbar geometry, anchored/scrollable inspector,
logical-coordinate-preserving zoom, shared soft-shoulder tabs, and button/field
states exported from the production UIKit renderer. The application workspace
specification is the required contract for future apps.

Passing evidence:
- 10 host behavioral tests: `builds/manifests/20261117T170954246881Z-1909.json`.
- Runtime menu-layer lifecycle checks: `builds/manifests/20261117T165648188616Z-99350.json`.
- ARM installed-kernel build: `builds/manifests/20261117T170411230657Z-741.json`.
- Disposable disk byte verification: `builds/manifests/20261117T170804203069Z-1775.json`.
- SDK package: `builds/manifests/20261117T170143462913Z-639.json`.

Remaining failures after the bounded native correction passes:
1. The native application takes tens of seconds with a blank client area before
   its first complete paint. Host rendering does not exhibit this delay.
2. Clicking View highlights the menubutton and transfers focus, but the dropdown
   is not visible in the captured native client. The low-level menu lifecycle
   test passes; actual toolkit popup presentation does not yet pass.
3. Final native maximize/restore, Cmd zoom and complete design-reference parity
   must be rechecked after the above presentation defects are resolved.

Evidence directory: `builds/validation/infinitycode-fixed-chrome/`.
See `native-presentation-open.png`, `native-presentation-open-late.png`, and
`native-presentation-menu-final.png`. The updated release ISOs have not been
built or published. Earlier ISOs do not contain this implementation.
