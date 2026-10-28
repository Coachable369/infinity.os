# Authored login screen

The Login Screen section in Infinity Studio edits `assets/boot/login-screens.infinityui`.
Saving compiles `login-screens.iuit`; Studio previews that payload with the production
native compositor. The live and installed kernels consume the same payload.

Reuse the approved Precision panels, button surfaces, field artwork, fonts, emblem,
and header. No alternate window frame or icon family is introduced. Shared button
recipes apply unless an element explicitly overrides its surface settings.

Decorative images, text, and panels can be moved, restyled, hidden, or removed.
The password field and primary Sign In button must remain visible. Authentication
continues through the existing identity service; authored text cannot execute code.
Optional action buttons have explicit `Login:` roles in the inspector, independent
of their editable labels and IDs. Native action roles include account selection,
security key, account creation, recovery, options, shutdown, restart, network, and
accessibility. Removing an optional button removes its pointer target.

The password control always masks entered credentials. Studio renders a blank
credential field and never obtains a real user's password or identity data.

Verification: `./build-kit studio-login-tests` exercises round-trip persistence,
undo, credential constraints, and production pixels. `./build-kit login-layout-tests`
exercises the native parser, typed actions, visibility, and pointer targeting.
The screenshot is written to `builds/studio-login-preview.png`.

ISO delivery still requires rebuilding the UI-linked kernels. The incremental
Save & Update ISO path verifies login payload bytes in both live and installed
kernels. An already installed VM is not changed by updating the ISO.

Desktop top-bar item and dropdown editing is a separate, unfinished integration.

## Per-element typography and backgrounds

The inspector reuses existing pickers and sliders: Font family selects one of the
46 immutable FontKit families (0 keeps the screen default); Sharpness adjusts
glyph coverage contrast from 0–100; Text glow applies a bounded same-color halo
from 0–100. Zero effect preserves legacy pixels. Effects are scoped to one layer,
never to the desktop font preference. Native canvas and installed screens use the
same painter and IUIT v12 records; v6–v11 retain their original defaults.

Border radius is always visible for surfaces. Editing it explicitly selects
Rounded corners, or Square at zero; Kit default remains available in Corners.
Edit Background selects the screen's existing image layer. Locked images expose
an explicit Unlock Image action; replacement retains geometry, fit and stacking,
resets the crop, and copies the selected raster into packaged boot assets.
