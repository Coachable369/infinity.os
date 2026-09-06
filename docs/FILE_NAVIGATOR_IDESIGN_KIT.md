# InfinityOS File Navigator — IDesign Kit

## Product identity

- Application identity: `app.infinity.file-navigator`.
- Display name: File Navigator.
- The generated Crystal Blue Glass icon combines a translucent folder, the Infinity path mark, and connected object nodes. The 256 px runtime asset is `assets/apps/file-navigator-icon-v1.png`; its lossless source is retained beside it.
- The application is an Object and Namespace navigator. It never presents a POSIX filesystem, inode, drive-letter, or process-working-directory model.

## Window recipe

- Normal, independently movable and resizable Window Server window with persistent surface state, minimize, maximize, restore, focus, and workspace participation.
- Default geometry is 780 x 560 logical pixels; minimum geometry is 640 x 420. The inspector collapses before the object view becomes unusably narrow.
- Chrome uses frosted translucent navy, a white-blue one-pixel key line, a soft cyan focus edge, and restrained luminous highlights. Background opacity and blur follow the active theme while controls and outlines remain fully legible.

## Layout and density

- A 44 px toolbar contains Back, Forward, Up, Home, a segmented breadcrumb/direct-location field, Search, List/Grid, and Inspector controls.
- A 176 px sidebar shows only locations that resolve through Namespace or Space services. It groups Favorites, System, Spaces, and Activity without inventing unavailable roots.
- The central viewport uses 36 px list rows or 112 px grid cells. Enumeration is incremental and virtualized; the scrollbar represents the full result count and is draggable.
- A collapsible 240 px inspector shows real Identity, Metadata, Storage, Security, References, Versions, and Relationships values. Missing values are omitted rather than synthesized.

## Component states

- Breadcrumb segments use quiet glass wells, white labels, and a blue hover/focus edge. Direct entry accepts NamespaceRef or ObjectId and reports typed resolution errors inline.
- Object rows carry a semantic type icon, display name, type, size when meaningful, modified value, owner, reference count, version, and protection state. Grid cards preserve the same selection and capability states.
- Selected objects use a translucent blue fill plus a white-blue outline. Protected or unavailable operations remain visible when discoverability is useful, but disabled with a policy reason.
- Long operations use a compact progress strip with operation type, completed/total values, current item, error count, and Cancel. Conflicts require Keep Both, Rename Incoming, Replace Reference where valid, or Cancel.
- Empty, loading, stale, permission-revoked, degraded-service, and error states are first-class polished components rather than blank panels.

## Interaction contract

- Back, Forward, Up, Home, breadcrumb, direct entry, Search, sidebar, and Open in Console Here all carry explicit NamespaceRef values. No ambient CWD is consulted.
- Move preserves ObjectId. Copy creates a new ObjectId. Create Reference adds a ReferenceId to the same ObjectId. Remove Reference detaches only that reference. Delete moves to Trash. Destroy is separated, capability checked, and confirmed.
- Pointer drag declares Move, Copy, or Create Reference before commit. Clipboard payloads retain typed ObjectRefs, source NamespaceRefs, and intended action.
- Every persistent create action exposes its inverse. Undo is advertised only for operations actually represented by reversible state.
- Keyboard focus is visible, tab order follows toolbar → sidebar → object viewport → inspector, and activation does not depend on rendered text.

## Measurable screenshot review targets

- Hierarchy: toolbar and active path read before content; inspector reads as supporting detail.
- Balance: sidebar 20–24%, object viewport 48–60%, inspector 24–30% at default size.
- Density: at least eight list rows at minimum height and four grid columns at default width.
- Border softness: 10–14 px panels, 8–10 px fields, one-pixel white-blue edge; no heavy teal slabs.
- Typography: title 18 px strong, toolbar 13 px, row title 14 px strong, metadata 12 px, section label 11 px tracked.
- Icon consistency: generated application icon for launcher/manifest; installed theme roles for semantic objects and actions.

## Implemented Finder-style extension

- The navigation strip uses themed left, right, and parent arrows with disabled-state treatment and a directly editable NamespaceRef field.
- List and grid views enumerate live direct children, virtualize scrolling, expose Name, Kind, and Size, and use the selected installed icon family.
- File selection supports pointer and keyboard navigation. Namespace nodes open in place; UTF-8 objects open in the native Text Editor.
- Native secondary-click menus provide Open, Rename, Duplicate, Move to Trash, Get Info, New Folder, view selection, name sorting, and inspector control.
- Secondary, back, and forward mouse buttons are preserved end-to-end by the pointer input path.
- Navigator state changes repaint only the navigator window damage region; applications retain no direct global framebuffer access.
