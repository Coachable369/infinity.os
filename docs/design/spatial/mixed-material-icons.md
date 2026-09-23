# Infinity Blue — mixed-material default icons

The Spatial IDesign Kit is the visual authority. The default pack uses material
appropriate to each object, not a uniform glass or ice treatment:

- Documents, envelopes, calendars and clipboards: opaque paper.
- Drives, tools, gears and microphone: brushed metal and dark hardware.
- Folders and controls: blue enamel or textured blue surfaces.
- Projects: navy leather with metal fasteners.
- Pictures and displays: photographic imagery.
- Search lens, network nodes and wastebasket: selective glass.

The stable persisted ID remains 0 (`CrystalBlueGlass`) for compatibility. Its
visible label is now **Infinity Blue**. Other theme IDs and their assets are
unchanged. The existing shared semantic renderer and installed icon payload
consume the replacement pack; there is no per-app duplicate theme.

## Generated sources and prompt set

Built-in image generation was used, with `idesign-kit.png` as style reference.
Accepted source sheets live in
`assets/icons/crystal-blue-glass/source-mixed-v3/`. The rejected all-glass sheets
are not used. Each source is 1536×1024 RGBA, five columns by three rows.

Shared generation specification: match the authoritative kit with near-frontal,
shallow 3D icons; coordinated blue accents, white paper, brushed silver and dark
hardware. Preserve material-specific textures. No ice, frost, uniform glass,
wireframe, labels, floor, cross-cell shadows or background haze. Require true
transparent alpha, generous gutters and clean edges suitable for 256px export.

Ordered sheets:

1. `base-00-14.png`: home, user, folder, open folder, documents, downloads,
   photograph, music, film, leather briefcase, empty/full glass trash,
   internal/external drives, optical disc.
2. `base-15-29.png`: USB, cloud drive, network nodes, server, computer, display,
   printer with paper, camera, microphone, headphones, terminal, metal gear,
   magnifying lens, information, help.
3. `base-30-44.png`: lock, unlock, shield, key, power, restart, moon, Wi-Fi,
   Bluetooth, battery, speaker, clipboard, envelope, calendar, clock.
4. `actions-45-59.png`: back, forward, up, refresh, paper new file, new folder,
   save, scissors, paper copy, paste, undo, redo, add, remove, close.

The first three sheets are vertically assembled, preserving alpha, into
`master-base-v3.png`; the fourth is `master-actions-v3.png`. Existing slicing and
Lanczos sizing produce all 60 roles at 24, 32, 48, 64, 96, 128 and 256 pixels.
Runtime compact and launcher atlases are rebuilt from those same files.

Rebuild only this family with `sh tools/build-icon-themes.sh crystal-blue-glass`.
Run `make icon-theme-test` for registry transitions and artifact dimensions,
coverage and alpha. Installed-payload byte parity is separately checked by
`ui-install-parity-test`; asset checks alone are not installed-VM proof.
