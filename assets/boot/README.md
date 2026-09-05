# InfinityOS boot artwork

This folder contains the directly editable visual assets used by the boot and
installation experience.

## Asset pairs

- `infinity-eclipse-header-v1.png` — current boot/loading background with InfinityOS masthead
- `infinity-eclipse.png` — original masthead-free boot/loading background
- `infinity-console-background-v1.png` — distinct installed-system Console background with a dark lower interaction zone
- `infinity-emblem-v2.png` — animated infinity mark source
- `infinity-installer-background-v2.png` — installer background source
- `infinity-installer-masthead-v2.png` — generated emblem and InfinityOS masthead source
- `infinity-installer-welcome-v1.png` — legacy first-step welcome illustration source (not packaged)
- `infinity-installer-mesh-hero-v1.png` — connected-device hero for the mesh welcome screen
- `infinity-installer-mesh-overview-v1.png` — planetary network artwork for the reusable overview card
- `infinity-installer-mesh-diagram-v1.png` — panoramic device-to-InfinityOS mesh diagram used beneath the first-step overview heading
- `infinity-cursor-v1.png` — mouse cursor source
- `infinity-storage-comparison-v1.png` — fixed partitions versus Infinity Pool explainer
- `infinity-storage-hierarchy-v2.png` — Infinity Pool to four Spaces explainer
- `infinity-storage-hierarchy-v3.png` — step-two topology with labeled-space safe zones
- `infinity-disk-discovery-vision-v1.png` — step-three Infinity Pool growth illustration with code-rendered copy safe zone
- `infinity-storage-device-v1.png` — step-three discovered-device product thumbnail
- `infinity-date-time-world-v1.png` — date/time step world-clock and synchronized-orbit artwork
- `infinity-time-zone-map-v1.png` — text-free 2:1 world map used by the live time-zone picker; the selected longitude window, latitude guide, and marker are rendered from typed configuration
- `infinity-installer-activation-v1.png` — generated system-generation activation artwork for progress and completion
- `infinity-installer-progress-hero-v1.png` — exact progress-console hero crop with the captured cursor removed

The same-named `.bmp` files are the firmware-ready copies embedded in the
kernel. Edit or replace the PNG, then regenerate its BMP before building:

```sh
sips -z 540 960 -s format bmp assets/boot/NAME.png --out assets/boot/NAME.bmp
./build.sh
```

The `960x540` conversion is intended for embedded explanatory graphics. Keep
full-screen backgrounds at their original 16:9 dimensions when converting.

Keep backgrounds at a 16:9 aspect ratio. The framebuffer renderer scales and
crops them to the screen's maximum negotiated resolution. The installer keeps
the upper 28% clear for the image-based infinity mark and draws its UI across
the darker center of the scene.

The bootstrap and installed Console use separate full-screen backgrounds. The
Console background deliberately contains no baked-in logo or interface text;
the renderer composites the complete emblem and its live particle trail into a
fixed safe region before drawing the Console panel. This prevents transition
damage from clipping the emblem and keeps future branding changes modular.

The welcome illustration's editable PNG and firmware-ready BMP are both kept
here. Older unlinked assets remain as design history.
