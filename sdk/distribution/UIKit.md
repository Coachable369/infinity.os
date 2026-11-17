# UIKit — one design system, one renderer

This kit is the same Eclipse Precision UIKit used by InfinityOS, not a visual
approximation. The packaged reference, RGBA atlas and style payload come directly
from `assets/ui-design-kit/default`; the catalog and editable Studio document
come from `assets/ui-design-kit`.

Read `EclipsePrecision.md` for geometry, colors, transparency, typography,
interaction states and reduced-motion requirements. `Controls.md` documents the
OS control family. `catalog.json` identifies the authoritative reusable assets.
Catalog entries describing other OS artwork are references, not promises that
every wallpaper or browser asset is duplicated in this compact SDK.

## Runtime integration

Native applications request windows and controls through the public UIKit API.
The OS compositor renders outer frames and AI adorners; applications own their
content, not competing window borders. Buttons inherit the installed theme.
Do not hard-code a screenshot as a functional control or ship a second theme
renderer. Shared controls preserve focus, hover, pressed, disabled and error
states and their actual behavior.

The exported atlas and style payload are useful for design inspection and
tooling. Normal apps should let the runtime consume the active theme rather
than reading these build-time assets to override a user's current preferences.
The bundled SDK snapshot reflects the release from which it was built.

## Authoring

Use Infinity Studio's shared UIKit editor for OS theme work. A saved section
feeds the normal build; Save & Update ISO changes an explicitly selected test
ISO, not a running installed system. Preserve behavior and semantic roles when
changing artwork. Validate all states, contrast, clipping and scaling, including
the active/inactive window and AI adorner.

See APIKit/GettingStarted.md for the supported public control API. The internal
OS control catalog is broader than the currently exposed C/Python factories.

## Application workspace standard

New apps must follow `UIKit/application-workspace.md`: fixed-height
menu and toolbar, compositor-owned title bar, anchored sidebars, reflowing content
and zoom independent of saved geometry. Reuse native Python `infinity_uikit`
constants `MENU_HEIGHT` and `TOOLBAR_HEIGHT`, `workspace_tabs(root, style)`, and `workspace_controls(root, style)`.
Retain the returned images and use `Workspace.TNotebook`. The tab design reference
and normal/hover/selected/focused states are supplied in the same UIKit directory.

Native transient menus use `infinity_menu_create` with the standard window-create
grant. They occupy the menu layer without title bars, resize handles, or AI
adorners; they retain owned input, surface, focus, and teardown capabilities.
