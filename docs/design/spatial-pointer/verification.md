# Spatial / Pointer verification

## Automated behavioral checks

- `tools/personalization-test.rs`: speed/size/style binary round trips; legacy
  preference migration; owner isolation and corrupt tint-state rejection; legacy
  workspace migration; nonoverlapping controls; clipped hit rejection; slider
  endpoints; bottom-of-panel scrolling; opaque cursor hotspots and bounded sizes.
- `tools/spatial-backdrop-test.rs`: exact RGB/BGR channel results, zero/full tint
  strength, clipped damage, cached composition and restoration.
- `tools/active-painter-test.rs`: native cursor pixels stay inside restoration
  bounds across ten styles, three sizes and display edges; actual collection
  aura/reflection pixels; existing native damage-rendering regressions.
- `tools/cursor-install-parity.rs`: exact generated RGBA bytes must appear in
  both live and installed ELF kernels. The streamed ISO builder additionally
  compares its reassembled installed-kernel payload byte-for-byte with the ELF.

Native pixel outputs reviewed: `build/personalization-panels.png` and
`build/gravity-controls.png`. These are native framebuffer renderings, not
ImageGen mockups. The design reference and original cursor sprites were generated
using the imagegen skill in built-in mode; prompts are in `specification.md`.

## In-OS handoff checks (not yet performed)

1. Settings → Appearance → Wallpaper: select a preset or adjust RGB/strength;
   open Spatial Desktop and inspect the veil.
2. Click Ideas/Projects/Inspiration/Archive to expand details; open a live item,
   add an idea, page the list and collapse the detail well.
3. Settings → Devices → Pointer: drag speed/size and choose each cursor style.
   Check movement and restoration over text, window edges and the display edges.
   Speed applies to relative mouse motion; an absolute tablet retains direct
   position mapping.
4. Sign out/in, then cold boot the installed disk with ISO detached. Verify saved
   tint, cursor style/size/speed, and existing workspace content remain intact.

Host rendering and binary parity do not substitute for these installed GUI and
cold-boot checks. No existing VM accounts or disks were modified.
