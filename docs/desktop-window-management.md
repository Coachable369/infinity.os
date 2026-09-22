# Desktop window management and top-bar polish

## Five added workflows

| Feature | Interaction |
| --- | --- |
| Side-by-side tiling | Ctrl+Shift+H / B: place the focused window on the left / right |
| Center / recover window | Ctrl+Shift+G: bring the focused window fully inside the work area |
| Cycle open apps | Ctrl+Shift+W: cycle File Navigator, Command Window, Text Editor, Task Manager and Settings, skipping closed or minimized apps |
| Title-bar maximize / restore | Double-click a window title within 450 ms without dragging |
| Precise resizing | Ctrl+Shift+U / I: grow / shrink the focused window by 2.5% of the display, respecting its minimum size |

The top-bar gear menu also exposes tiling, centering, cycling and resizing for
mouse users. File Navigator's windows share one app slot in the app cycle; this
is not a thumbnail switcher for every individual navigator window. Settings
retains its 60%-display minimum width, so it edge-aligns rather than shrinking
into an unreadable half-width layout. The other apps support half-width tiling.

Existing glass chrome, semantic icons, typography, rounded menu rows and shared
geometry are reused. No bitmap controls or new UI asset family are introduced.
Window actions retain document buffers and command sessions, and use the existing
layout checkpoint path. Geometry/gestures are allocation-free; no additional
per-frame animation, background polling, or framebuffer ownership is introduced.

## Top bar

- Escape, outside click and re-clicking the open menu return to the prior app or
  Settings focus instead of resetting the desktop.
- Left/Right traverse the visible menus; Up/Down and Tab traverse rows; Home/End
  select the first/last row; Enter activates. Moving across top-bar icons while
  a menu is open switches menus.
- Status popups are anchored near their icons and constrained to the screen.
- Restart and shutdown use a confirmation menu with Cancel selected initially.
- About routes to About rather than Storage. Help/status output uses the native
  Command Window rather than replacing the desktop.
- The calendar retains Previous/Today/Next navigation and adds System Settings.
  Its 2x-scale backing region, including shadow, fits the existing menu cache.

This is not complete macOS/Windows control-center parity. Audio volume/mute,
Bluetooth pairing and battery telemetry still depend on real device services;
this change does not fabricate those controls. Their existing menus lead to
device, input and power management. Dedicated date/time editing was not added.

## Verification and packaging

`window-workflows-test` exercises geometry, application minimums, tiling gaps,
off-screen recovery, size round-trips, app cycling and double-click boundaries.
The input tests cover each new chord and authentication/surface guards. The
calendar/input harness checks actual hit-test targets and saved backing bounds at
1024x768, 1920x1080 and 2560x1600, menu traversal and power-confirmation actions.
These are behavioral model tests, not installed GUI evidence.

The common installed kernel includes the new module and both input transports;
there is no live-only feature or new packaged asset. ARM64 and x86_64 installed
kernel builds passed. The ARM64 streamed installer rebuild and binary payload
parity checks are run before publication. Installed gesture/screenshot validation
remains pending: the existing VM is at login and automated guest input remains
unreliable. Do not report full desktop interaction acceptance from compilation.
