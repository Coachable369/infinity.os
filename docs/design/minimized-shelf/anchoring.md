# Drawer edge anchoring

Drag the top handle labeled DRAG to move the running-app drawer in two dimensions.
Release in the desktop interior to keep it floating. Release within 3.5% of the
left or right edge to anchor it there. Context menus
and hover labels open toward the desktop. App buttons retain their restore and
right-click management behavior; wheel scrolling remains available.

The selected edge and floating coordinates are stored per user in reserved
desktop-layout bytes. Existing records default to the right. Drag capture is
transient and is never restored. Spatial saved layouts also retain placement.
This uses the existing glass recipe and icons, without new packaged assets.

Validation: drawer geometry tests cover both edges and dragged positions at
800x600, 1920x1080 and 3840x2160. Session round-trip coverage includes left-edge
placement. Live installed-VM interaction and ISO publication remain pending.
