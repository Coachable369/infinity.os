# Movable desktop widgets

Drag the grip/title area of System Overview or AI Chat to reposition it. Release
near a work-area edge or another visible widget's aligned edge to snap. App
windows retain input priority above widgets. The chat close/minimize controls
remain separate from the drag target.

Right-click exposed desktop space for the glass Widgets chooser. Checked rows
toggle System Overview and AI Chat; Reset positions restores their original
right-column arrangement without changing visibility. Escape or an outside
click dismisses the chooser. These are the two currently implemented widgets;
this change does not add a weather provider or new widget types.

Placement and visibility are saved per user and in Worldshift layouts using
reserved bytes in existing desktop/spatial records. Legacy records retain the
original positions. Drag capture and context menus are never persisted.

Rendering and hit testing share geometry. Movement damages old/new widget
coverage plus the glass shadow margin, not the full desktop; unchanged pointer
movement adds no widget damage. Application surfaces remain owned by the
existing compositor. No new runtime bitmap dependency is introduced.

## Design and evidence

`widget-layout-reference.png` is a generated design reference, not a running OS
screenshot. Built-in image-generation prompt: InfinityOS desktop widget
interaction design kit; midnight navy glass, cyan outlines, crisp white
typography, 16px gutters; System Overview and AI Chat header grips, edge-snapped
and floating panels, checked visibility menu and reset positions, matching
buttons/fields/sidebar/dock/window chrome. Its illustrative charts and values
are not implemented by this interaction change; existing live content remains.

`build/widget-menu-proof.ppm` is the production glass/font/menu renderer on a
host-owned framebuffer (reviewed PNG: `widget-menu-native-proof.png`). The pixel test compares incremental open, move and
dismiss frames to full composition. Initial testing caught a missing shadow
damage margin; the corrected comparison passes. Model tests exercise snapping,
capture/release, visibility, persistence, display bounds and idle damage.

Local validation: 2 widget behavior tests, 16 spatial tests, 12 painter tests,
desktop-session reconstruction and `make input-regression-test` pass. Installed
VM drag/chat interaction and complete desktop screenshot review remain pending;
host pixel evidence does not establish those acceptance items.

The i686 text-mode fallback cargo check also passes. The graphical menu renderer
is gated to the existing x86_64/AArch64 desktop paths.

Release validation: ARM64 and x86_64 native builds, the Hermes/Ministral ARM64
installer package, installed-kernel/boot-loader binary parity, and UI installed
asset parity all passed. Updated ARM64 and x86_64 ISOs are in `builds/` with
SHA-256 checksums. This packaging proof does not replace the pending live VM test.
