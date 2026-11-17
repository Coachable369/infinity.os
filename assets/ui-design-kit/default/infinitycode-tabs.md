# InfinityCode workspace chrome

Reference: `infinitycode-tabs-reference.png`, derived from the approved
`browser-tabs-soft-v4` silhouette. Native implementation:
`sdk/python/infinity_uikit.py::workspace_tabs`.

- Menu row: 36 logical pixels; toolbar: 48. These heights depend only on the
  explicitly selected workspace zoom and initial display density, never on the window height.
  Displays at least 2560 × 1440 use a 2× default density for readable controls.
- Tabs: 32-pixel base artwork with fixed shoulders, flat stretchable centers,
  Inter 18-pixel labels, active #243651, idle #101B3A, hover #2E4263.
- Permanent workspace tabs are not closable documents. Keyboard navigation
  and focus remain native ttk; focused selected edges use #C9D2E0.
- Zoom: 75–175%, Cmd/Ctrl plus, minus, zero and View menu. Stored form
  coordinates remain logical pixels. Design handles remain 10 physical pixels.
- Properties remain anchored right, toolbox left, and the center viewport fills
  the remaining space. The form remains centered or scrollable at every zoom.
- OS compositor owns title bar, frame and window controls; the app does not draw
  additional window chrome.

The compact tab source assets are `workspace-tabs/{normal,selected,hover,focus}.svg`.
`tools/native-python/uikit-tabs.py` compiles these into the packaged native UIKit
module. The native widget must consume these pixels, retain 32 logical pixel
height, and stretch only the center. Do not add duplicate vertical label padding.
