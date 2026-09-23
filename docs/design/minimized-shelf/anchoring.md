# Drawer edge anchoring

Drag the top handle labeled DRAG to move the running-app drawer horizontally.
Release to anchor it to the nearest left or right screen edge. Context menus
and hover labels open toward the desktop. App buttons retain their restore and
right-click management behavior; wheel scrolling remains available.

The selected edge is stored per user in the previously reserved last byte of
the desktop-layout record. Existing records default to the right. Drag capture
is transient and is never restored. Spatial saved layouts also retain the edge.
This uses the existing glass recipe and icons, without new packaged assets.

Validation: drawer geometry tests cover both edges and dragged positions at
800x600, 1920x1080 and 3840x2160. Session round-trip coverage includes left-edge
placement. Live installed-VM interaction and ISO publication remain pending.
