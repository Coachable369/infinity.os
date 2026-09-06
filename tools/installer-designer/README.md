# InfinityOS Installer Studio

A native macOS visual editor for all eleven InfinityOS installation screens and all eight post-install OS configuration screens. Its two expandable screen collections share the zoomable snapping artboard, layer ordering, eight-handle resizing, image import, editable copy, undo/redo, validation, and atomic JSON/runtime-template saving. Navigation buttons are visible but immutable.

Run `./run.sh`. Save writes:

- `assets/boot/installer-screens.infinityui` — human-readable source template.
- `assets/boot/installer-screens.iuit` — validated binary consumed by the installer.
- `assets/boot/configuration-screens.infinityui` — editable post-install configuration templates.
- `assets/boot/configuration-screens.iuit` — validated binary consumed by first-boot configuration.

The editor uses normalized 1000 × 1000 geometry, so a design scales consistently to every supported display mode. If a runtime template is malformed, the installer rejects it and uses its compiled safe layout.
