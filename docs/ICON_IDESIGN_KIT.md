# InfinityOS Icon IDesign Kit

The installed desktop uses one stable 60-role semantic catalog across three interchangeable visual families. Every family ships in 24, 32, 48, 64, 96, 128, and 256 pixel PNG sizes with transparent backgrounds, plus compact BGRA runtime atlases for framebuffer rendering. The live ISO and bootstrap do not package or embed these desktop-only resources.

## Families

- **Crystal Blue Glass** — the default. Genuinely translucent pale-blue architectural glass, thin satin-silver edges, restrained cyan rim light, neutral highlights, and soft refraction inspired by the supplied glass-folder reference.
- **Luminous Obsidian** — smoked black glass with restrained electric-blue and warm-gold edge lighting.
- **Frosted Quartz** — neutral clear and softly frosted optical glass with smoke-gray etched cores, delicate ice-white highlights, and subtle champagne edges.

The complete visual references are the generated base and action masters within each family directory. Runtime atlases are in `assets/icons/runtime/`; individual production resources are grouped by family, size, and semantic group.

## Size and state rules

- 24 and 32 px: menus, top-bar status, compact controls.
- 48 and 64 px: sidebars, dock, Settings, and standard desktop density.
- 96 and 128 px: large desktop, file-manager, accessibility, and preview surfaces.
- Selected icons retain their original artwork and receive a separate skin-aware selection plate or running indicator. Disabled, hover, and pressed states are composed by the surface, never baked into the icon bitmap.
- Icons always render by semantic role. Switching the family changes every consumer without changing its role or behavior.

## Catalog

`assets/icons/manifest.csv` is the authoritative ordered catalog. It includes 45 standard OS objects and devices plus 15 file/action controls: home, users, folders, documents, media, projects, trash states, internal/external/optical/USB/cloud drives, network, servers, computer/display/printer/camera/audio hardware, terminal, settings, search, information/help/security/power/radio/status/PIM roles, and back/forward/up/refresh/new/save/cut/copy/paste/undo/redo/add/remove/close actions.

## Settings contract

The default is Crystal Blue Glass. Users choose any family directly in **System Settings → Themes & Skins → Icon Set**. The selection is user-scoped, persisted in the identity profile, restored at session entry, and invalidates all live icon consumers transactionally.
