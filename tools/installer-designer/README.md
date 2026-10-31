# Infinity Studio

From any working directory, run `/opt/codebase/infinity.os/run-infinitystudio.sh`
to clean Studio's Swift build products, rebuild and verify the app through the
Studio-isolated build kit, then open it. Existing ISOs, models and OS build caches
are preserved. Use `--no-launch` to build without opening the app. Build failures
stop the launcher, so it never opens an older app after a failed build. Close any
already-running Studio instance first to ensure macOS opens the newly built app.

Studio profiles use `builds/.studio-work` for Swift products, native preview
helpers, caches, fixtures and temporary files, with a separate
`builds/.build-kit-studio.lock`. They can run while an ISO build owns the normal
build lock; concurrent Studio builds still serialize. ISO cleanup does not
remove Studio's workspace. Studio packaging does not rewrite runtime templates:
Save exports them, and ISO builds compile their own templates independently.

The collapsible left navigation contains **UIKit**, **Configuration Wizard**,
**Installation Wizard**, and **System Settings**. Switching sections retains
pending edits. **Save Section** (Command-S) saves the active section; **Save All
Sections** (Shift-Command-S) validates and saves UIKit plus every screen collection.
The status bar tracks unsaved changes across sections. Closing or quitting offers
Save All, Discard Changes or Cancel; a failed save keeps the draft open. Studio
also stays open while an ISO update is running so its publication cannot be
silently abandoned.
Unreadable or invalid source files are protected from being overwritten by a
factory fallback. Import a validated replacement for the affected screen collection,
or repair the source and reopen Studio; errors identify the protected file.

UIKit exposes the shared native component states, glyphs, icons, wallpapers and
other repository raster artwork in a searchable visual library. Select an item
to inspect its actual pixels and runtime identity, then upload a replacement.
Images retain alpha and are normalized to the native element's dimensions.
Each native atlas cell is separately replaceable. Colors & Theme edits all
thirteen semantic default-skin colors with a live Precision surface preview,
color pickers and exact RGB/ARGB entry. Reset and undo/redo are scoped to UIKit.

**Surface Styles** configures window borders/backgrounds, panels and dialogs,
primary buttons, secondary buttons and input fields independently. Choose the
approved artwork, glass or matte material; keep the approved corners or choose
square/rounded geometry and a radius. Optional fill and border/rim colors belong
to that component family, including alpha. Reset restores the approved artwork.
Outer application windows retain the approved rectangular OS-owned frame;
rounded geometry is available for dialogs, panels, buttons and fields.
The live preview includes normal, focused and pressed states. In a wizard or
Settings screen, select a layer and use **Appearance → Surface / Corners** for
an individual override. Explicit layer colors take precedence over global tints.
Unlock protected layers before editing; required actions cannot be deleted.

Decorative images are optional: installer/configuration mastheads, installer
backgrounds, progress illustrations, timezone maps, and Settings navigation icons
can be removed after unlocking them. Functional navigation remains protected.
Installer scenes use IUIT v10 and expose the previously implicit wallpaper as
**Installer Background (includes logo)**. This image contains baked-in artwork;
replace the image to change that artwork, or delete the layer to remove it.
Deleted backgrounds and illustrations stay deleted after save/reopen. The native
renderer no longer paints a second hidden masthead behind authored scenes.
Older kernels require an updated kernel before accepting v10 templates; use the
Studio ISO injection workflow rather than copying the template alone.

**Themes** saves named combinations of colors, materials, geometry and artwork.
Eclipse Precision, Frosted Glass and Matte are protected presets. Save the current
draft as a new theme, duplicate a saved theme, update or rename a custom theme,
and import/export a self-contained `.infinitytheme` JSON bundle. Uploaded PNGs
are embedded with SHA-256 identities and validated native dimensions. Applying a
theme is one undoable draft operation; it never implicitly publishes an ISO or
overwrites screen layouts. Removed custom themes move to
`assets/ui-design-kit/themes/.trash` for recovery. All project exports remain
inside the repository.

Screen layers can select **Fill source** and **Rim source** from the semantic
palette. A theme binding resolves to exact native RGBA bytes at compilation and
in the native preview. Selecting **Custom color** restores the layer's retained
custom value; no color-matching heuristic changes existing designs. Save All
Sections when changing both the theme and bound screen layers.

**Export Section Templates** writes a new folder containing the current editable
JSON and native IUIT output without saving or changing the active draft. Custom
images are embedded in the native export. Importing IUIT restores those pixels
under content-addressed project asset names before changing the canvas. Editable
JSON retains semantic theme bindings; IUIT contains their resolved runtime colors.
Save All prepares and verifies every source/runtime output before publication;
an I/O failure rolls back already-published files. It does not claim power-loss
atomicity across multiple filesystem entries. Recovery paths are reported if the
filesystem itself prevents rollback.

## Native preview contract

Installer and configuration canvases are framebuffers produced by the OS's
production painters, fonts, image decoders and surface code. Unsaved templates,
palettes, artwork and styles are passed to the same native renderer used for
verification. Canvas controls offer multiple viewport sizes and normal, focused,
pressed and transition samples. Pending, failed and timed-out frames are labeled;
an older request cannot overwrite a newer edit. Dynamic service values in Studio
are explicitly simulated; Studio never erases a disk, changes a clock or contacts
OS services to render a preview.

This is not yet a universal application-layout editor. Several specialized
Settings panes and imperative application layouts still have native-owned
geometry. Shared surface recipes and artwork apply to those consumers, but that
does not make every internal control canvas-editable. Unsupported scenes must be
labeled as template-layer previews, not represented as full runtime parity.

Current editing coverage:

| Surface | Native pixel preview | Authoring boundary |
| --- | --- | --- |
| Installer and configuration templates | Production renderer, including live-value simulations | Authored layer geometry, copy, images, palette bindings and surface styles; the erase confirmation's internal layout remains OS-owned |
| General, account, AI, privacy, devices, information and Input Settings | Production renderer with simulated values and expanded controls | Forty row descriptions and detail surfaces, eight General/AI/Privacy actions, and the inactivity-timeout slider support native-bound authoring; specialized pointer controls remain OS-owned |
| Network and Nodes Settings | Production dashboard renderer and available subpages | Copy and appearance; responsive dashboard control geometry remains OS-owned |
| Themes and Pool Settings | Template-layer preview only | No claim of complete native scene parity |
| Other applications and desktop widgets | Shared UIKit component preview, not a complete application canvas | Shared component recipes/artwork, not application-internal layout authoring |

The remaining universal-editor work is to move the OS-owned internal layouts into
typed authorable scene contracts and expose those same contracts to Studio. A
theme's shared styling coverage must not be mistaken for complete layout-editing
coverage.

Expanded Settings children use IUIT v9 parent UUID bindings. Selecting a
child reveals its real parent row; its 0–1000 frame is relative to that row's
summary or expanded detail area, not the whole screen. Studio obtains the physical parent bounds
from the OS renderer for selection and drag/resize mapping. The OS uses that
same resolver for drawing and pointer hit testing, including scroll clipping.
Native captions follow service state; switch to a literal caption for custom
copy. Button caption RGBA is independently editable. Summary and detail descriptions
support authored text, font size, color and visibility; detail backgrounds reuse
the same image-backed material and corner recipes as other panels. Legacy v6–v8 imports add
the required child layers deterministically without replacing existing layers.
Required bindings cannot be deleted, duplicated, or attached to unrelated rows.
Hiding a button suppresses both its pixels and click target, but keeps its layer
available in Studio. The Devices pointer-detail description starts hidden to avoid
covering its existing specialized controls, but remains selectable and editable.

The Privacy inactivity-timeout slider has separate track and thumb layers. Both
support appearance, visibility and sizing. The OS still owns its range, current
value, permission checks and persistence. Thumb horizontal position is derived
from that value: Studio explicitly disables X editing rather than saving a value
the runtime would ignore. Track movement and thumb dimensions feed the same
native paint and pointer resolver, including viewport clipping. Other specialized
sliders, pointer controls and application-internal layouts are not yet authorable.

Customizations are saved in `assets/ui-design-kit/uikit.infinityui`, with imported
assets under `assets/ui-design-kit/overrides/`. The build validates and applies
them before packing the live and installed kernels. Use **Save & Update ISO**
(disc button or File menu) to save all sections and update a selected existing
ARM64 or x86-64 ISO. This reuses model, browser and toolchain payloads and rebuilds
only the UI-dependent kernels; it does not run the full OS build. A staged image
must pass payload and checksum verification before replacing the selected ISO.
This does not change a running or already installed VM. A changed original asset is reported as a
conflict instead of silently overwriting new work. The shared surface previews
sample the same Precision artwork and nine-slice contract as the OS.

Each successful injection writes a receipt identifying the compiled UI payloads,
their native format versions, both replacement kernels and preserved installed
payload members by byte count and SHA-256. The updater verifies the staged ISO
before atomically replacing the selected image. This is a UI update, not a
replacement for a clean release build or installed-VM testing.

The equivalent command after saving is:

```sh
./build-kit update-iso --target aarch64 --iso builds/InfinityOS-aarch64.iso
```

The update runs asynchronously in Studio with progress and expandable output.
If the required cached native components are unavailable or do not match the
chosen image, it stops with an explanation rather than claiming a successful
update. All temporary files and update manifests remain in the project.

A native macOS visual editor for both boot-loading screens, all eleven InfinityOS installation screens, all eight post-install configuration screens, and System Settings. The screen collections share the zoomable snapping artboard, top-navigation Marquee Select with enclosure and uniform group dragging, layer ordering, eight-handle resizing, image import, editable copy, undo/redo, validation, and JSON/runtime-template saving. Disabling Marquee clears selection and canvas handles. Unlock a layer to edit its appearance and geometry; required runtime roles remain protected from removal.

### Boot Loading Screens

Expand **Boot Loading Screens** in the sidebar and select **Pre-installation Boot**
or **Installed OS Boot**. These are independent scenes. All visible artwork is an
editable layer, including the background and revealing emblem; decorative images
can be replaced, hidden or deleted. Use the inspector for geometry, opacity,
layering, image fit/crop, text, colors and the shared surface styles. One progress
control is retained per scene because the boot service updates it.

The native preview invokes the same compositor as the kernel, not a SwiftUI
approximation. Enable simulated service values to scrub progress and reveal time.
The loading service supplies live status text; the saved progress caption appears
on the completed frame. The emblem's center-out reveal lasts 2.4 seconds; its
timing is currently OS-owned, while its image and geometry are authored.

Save writes `assets/boot/boot-screens.infinityui` and its embedded-asset
`boot-screens.iuit`. **Save & Update ISO…** includes this payload in both
the live and installed kernels through the existing cached kernel rebuild and ISO
update path. It does not alter an already-installed VM. Boot firmware screens and
the subsequent installer menu remain separate from these loading scenes.

Build with `./build-kit studio`, then open `builds/InfinityOS Installer Studio.app`. Saving a wizard section writes:

- `assets/boot/installer-screens.infinityui` — human-readable source template.
- `assets/boot/installer-screens.iuit` — validated binary consumed by the installer.
- `assets/boot/configuration-screens.infinityui` — editable post-install configuration templates.
- `assets/boot/configuration-screens.iuit` — validated binary consumed by first-boot configuration.

The editor uses normalized 1000 × 1000 geometry, so a design scales consistently to every supported display mode. If a runtime template is malformed, the installer rejects it and uses its compiled safe layout.

Installing and Complete use the same saved layer composition as the other template-driven steps. During installation, only the progress control's status and percentage are live; its frame, colors, opacity, visibility, and stacking order remain authored. Progress updates reconstruct that region before painting, so translucent controls do not accumulate ghosting or overwrite neighboring artwork. Studio's progress surface fills its actual resize bounds. These two screens are installer-only UI; the shared compositor is also compiled into the installed kernel.

Disk Discovery, Disk Review, Plan Review, and Attention have a **Live Installer Details** text layer. Select it in Layers or via **Add → Live Installer Details…**, then move, resize, lock, hide, or style it using the canvas and sidebar. The preview contains sample values; the ISO substitutes the actual disk, plan, date/time, or error data into the saved bounds. Text wraps and clips within that layer and respects its stacking order. Existing projects gain this layer automatically. Keep space below introductory copy for it; a deliberately hidden layer remains hidden after saving and rebuilding.

OS Configuration input layers include a Data Binding picker. It maps the field to one typed runtime variable: `machine.node_name`, `user.profile_name`, `user.display_name`, `credential.password`, or `credential.facial_profile_name`. The binding is saved in the editable project, encoded into the `.iuit` artifact, validated during the build, and used by first boot for value storage, restoration, and secure password masking.

The Facial Recognition page is appended when opening an eight-page configuration project, preserving existing authored screens. Its `facePreview`, `facePhoto0` through `facePhoto2`, and `faceMesh` layers retain native camera, accepted-photo, and animation behavior. The production renderer also supplies enrollment guidance, errors, and action captions. Preserve these code identifiers when styling the live elements; ordinary added images and text remain decorative. Saving the configuration section includes this page in the next ISO template compilation.
