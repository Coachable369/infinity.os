# InfinityOS Installer Studio

A native macOS visual editor for all eleven InfinityOS installation screens and all eight post-install OS configuration screens. Its two expandable screen collections share the zoomable snapping artboard, top-navigation Marquee Select with PowerPoint-style enclosure and uniform group dragging, layer ordering, eight-handle resizing while the selection tool is active, image import, editable copy, undo/redo, validation, and atomic JSON/runtime-template saving. Disabling Marquee clears the selection and all canvas handles. Navigation buttons are visible but immutable.

Run `./run.sh`. Save writes:

- `assets/boot/installer-screens.infinityui` — human-readable source template.
- `assets/boot/installer-screens.iuit` — validated binary consumed by the installer.
- `assets/boot/configuration-screens.infinityui` — editable post-install configuration templates.
- `assets/boot/configuration-screens.iuit` — validated binary consumed by first-boot configuration.

The editor uses normalized 1000 × 1000 geometry, so a design scales consistently to every supported display mode. If a runtime template is malformed, the installer rejects it and uses its compiled safe layout.

Disk Discovery, Disk Review, Plan Review, and Attention have a **Live Installer Details** text layer. Select it in Layers or via **Add → Live Installer Details…**, then move, resize, lock, hide, or style it using the canvas and sidebar. The preview contains sample values; the ISO substitutes the actual disk, plan, date/time, or error data into the saved bounds. Text wraps and clips within that layer and respects its stacking order. Existing projects gain this layer automatically. Keep space below introductory copy for it; a deliberately hidden layer remains hidden after saving and rebuilding.

OS Configuration input layers include a Data Binding picker. It maps the field to one typed runtime variable: `machine.node_name`, `user.profile_name`, `user.display_name`, or `credential.password`. The binding is saved in the editable project, encoded into the `.iuit` artifact, validated during the build, and used by first boot for value storage, restoration, and secure password masking.
