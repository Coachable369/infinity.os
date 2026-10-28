# Desktop scenes in Infinity Studio

Expand **Desktop & Widgets** in the left sidebar to edit the logged-in navigation
bar, System Overview, and AI Conversation widget. These are fixed native scenes;
their individual layers can be moved, resized, styled, hidden, or removed.

The documents are `assets/boot/desktop-screens.infinityui` and its compiled
`desktop-screens.iuit`. Save All and Save & Update ISO include both. The ISO updater
rebuilds UI-linked kernels and checks the payload in the live and installed kernels.
It does not update an already installed VM.

The native compositor renders the Studio canvas. Native roles retain clock,
machine status, microphone, model, conversation, prompt and button behavior
independently of their labels. Hidden controls have no pointer target. Navbar
items use their authored bounds for both input and menu anchors. User-dragged
widget positions translate child controls together.

Decorative layers cannot acquire execution authority. Each native binding occurs
at most once and only in its owning scene. The required scene frame remains;
optional images and controls are removable. Menu contents still use native menus;
this section does not introduce arbitrary executable menu commands.

Focused verification lives in `DesktopSceneTests`, `desktop-widgets-test.rs`, and
`desktop-polish-test.rs`. Host pixels and geometry checks do not establish
installed-VM acceptance.
