# Sapphire QA defects

All findings below are production-painter host captures, not installed VM proof.
No measured 95% acceptance is asserted. Preserve explicit Studio overrides when
correcting default presentation.

| ID | Surface | Observed failure | Required verification | Status |
| --- | --- | --- | --- | --- |
| SAPPHIRE-001 | Themes & Skins | Dense single-column setting rows do not match the approved two-column reference composition and spacing. | Normal/maximized/restored layouts, row hit targets, expanded details and keyboard order. | Open |
| SAPPHIRE-002 | Themes & Skins | Hero subtitle and action label are truncated at the reference capture size. | Complete readable copy with adequate button width and responsive wrapping; no overlap at supported sizes. | Open |
| SAPPHIRE-003 | Themes & Skins | Wallpaper row is cut off at the bottom of the initial viewport. | Reference layout density, complete visible controls and usable scrolling at smaller sizes. | Open |

Evidence: `builds/validation/sapphire-coverage/settings-2.png`, 1536×1024,
compiled from the current pending Studio document. Manifest:
`builds/manifests/20261119T222503831504Z-40661.json`.

## Coverage checkpoints

- Initial catalog: 76 surface entries / 650 candidate state checks. These are
  inventory counts, not unique component counts or completed checks.
- Default captures: 11 Settings pages and five desktop template surfaces.
- Settings contact sheet: `builds/validation/sapphire-coverage/settings-contact-sheet.png`.
- Desktop contact sheet: `builds/validation/sapphire-coverage/desktop-contact-sheet.png`.
- Individual controls, interactive states, runtime-only surfaces, applications
  and installed ARM/x86 checks remain open in `sapphire-coverage.json`.
- Desktop preview scenes use different authored backgrounds. Do not score their
  relative colors against one another until backdrop and transparency fixtures
  are normalized to each approved reference.
