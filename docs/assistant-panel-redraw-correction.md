# Attached assistant clipped repaint correction

## Cause

The attached assistant revision participates in the desktop content hash. On
File Navigator's desktop screen, that change previously selected the independent
desktop chat widget's damage rectangle. Only the overlapping right-hand slice
of the attached panel was repainted, clipping text and leaving stale app pixels.

## Change

- Track assistant revisions separately and route changes through bounded owner
  window composition, including the outside tab and glow margin.
- Exclude assistant changes from desktop-widget-only and command-input-only
  repaint paths.
- Preserve separate widget damage when desktop chat changes simultaneously.
- Respect maximized File Navigator geometry.
- Retain the existing smooth, antialiased rounded-shoulder tab renderer.

The change is shared by ARM64 and x86_64 and lives in the kernel used by both
live and installed System Generations; no new installer-only assets are needed.

## Evidence

Build-kit manifest `20260928T010723833176Z-14411.json` completed successfully:
editor assistant behavioral tests, InfinityUI harness, and both architecture
native-browser kernel compile checks. The new regression exercises expansion,
text editing, collapse, revision dispatch, concurrent chat damage, and complete
panel/halo coverage at scales 1, 2, and 3 without full-screen damage.

Manifest `20260928T010756060374Z-14539.json` renders the production tab recipe;
`build/behavior-tests/assistant-tab-proof.png` was visually reviewed for rounded
shoulders and antialiased glow. This is an isolated tab render, not VM proof.

An updated ISO and installed-system screenshot verification are still pending.
The currently installed VM has not been changed by this correction.
