# InfinityOS Default Type Library

InfinityOS installs 46 Open Font License families (the original 45 plus Arimo)
and 47 font resources into the EFI-backed
System bootstrap space on both the live image and every fresh installed System
Generation. `kernel/runtime/font.rs` exposes stable typed descriptors; UI text
uses anti-aliased atlases derived from Roboto Regular and Roboto Medium during early
framebuffer bring-up.

The installer uses dedicated `InfinityInstaller-Regular-24.atlas` and
`InfinityInstaller-Semibold-24.atlas` resources generated from Roboto at 24px. Their
`.metrics` and `.kern` companions preserve proportional advances and pair kerning.
The 24x28 cells preserve complete descenders and wide glyphs, while explicit
font-derived advance metrics guarantee consistent spacing throughout setup.
Dense four-column information cards use the matching 19px installer companion
atlas so their copy retains generous side gutters without changing type family.

The `.ttf` files and their per-family `OFL-*.txt` licenses are sourced from the
official Google Fonts repository. They are resources, not filesystem identity:
the native catalog resolves each stable font ID to its System-space resource.

Families include: Inter, Roboto, Open Sans, Lato, Montserrat, Fira Sans, Fira Code,
JetBrains Mono, IBM Plex Sans, IBM Plex Mono, Source Sans 3, Source Serif 4,
Noto Sans, Noto Serif, Atkinson Hyperlegible, and Arimo.
