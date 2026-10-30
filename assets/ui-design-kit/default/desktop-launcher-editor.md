# Native dock and launcher editing

Desktop & Widgets includes **Desktop App Dock** and **App Launcher Panel**.
Opening an older project appends these scenes without moving or replacing its
navigation, widgets, custom menu scenes, or menu destinations. Saving the desktop
section writes the ordinary desktop IUIT payload consumed by the live image and
fresh installation; Save & Update ISO remains a separate explicit operation.

Dock roles 72–80 bind the surface and its eight native actions. Layer labels,
images, positions, dimensions, visibility and ordering remain editable. Runtime
hit testing uses those same bounds. Open/hover and running indicators retain
native state; artwork does not grant action authority.

Launcher roles 81–86 bind the surface, search field, close action, application
grid, categories and scroll track. The grid contains the actual installed app
catalog, not editable placeholder launch targets. Search/filtering, scrolling,
drag ordering and category actions remain OS-owned. Resizing the grid changes
its six-column/two-visible-row layout. Hidden regions are not interactive.

Studio invokes the production launcher painter and icon atlas. The preview and
OS share layout, clipping, rendering and persisted data; there is no SwiftUI
facsimile of the launcher. Preview applications reflect the renderer's compiled
capabilities; installed catalogs can additionally contain registered user apps.

Panels and controls reuse Eclipse Precision surfaces and existing icon families.
The outer launcher clips content to its authored surface, including unusually
small or moved regions. Keep search and control bounds within that surface.
