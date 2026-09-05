# InfinityOS Native App Launcher IDesign Kit

Reference target: the supplied full-screen InfinityOS launcher composition.

## Composition

- The launcher rises above the centered bottom dock and remains inside the desktop work area.
- The dock and right-side widgets are persistent foreground glass. Window movement recomposes an intersected foreground layer after repairing its backing, so dragging can never punch holes through system chrome.
- At standard density the panel occupies roughly 78% of the framebuffer width and 72% of the usable height, with a 24 px corner radius.
- The wallpaper remains visible through a deep navy glass surface; a soft blue edge and restrained top highlight establish depth without a black outline.
- The search well is centered at the top, followed by a two-row application grid, pagination, a divider, and one row of category actions.

## Components and states

- Search: recessed rounded glass, search glyph, live query text or muted prompt, and `Ctrl + K` affordance.
- App tiles: 84 px glass-backed icon wells using the active installed icon family, centered labels, hover/focus glow, and press activation.
- Categories: Home, Work, System, Utilities, and Create Folder use wide glass cards with themed icons and a persistent active indicator.
- Dock launcher: an illuminated Infinity mark in the reserved ninth dock position; its active indicator is visible while the launcher is open.
- Empty search: the grid is replaced by a centered no-results state while the query remains editable.

## Interaction contract

- Dock Infinity click opens the launcher; a second click, Escape, or a click outside the panel closes it.
- Typing filters applications by name. Backspace edits. `Ctrl + K` and `/` focus search.
- Arrow keys move selection through visible results. Tab traverses apps and categories. Enter launches the selected action.
- App and category actions route into native shell modes and typed Settings/Home operations; unavailable decorative placeholders are forbidden.
- Geometry and hit testing come from `SystemLayout`, shared by renderer and input routing at every supported scale.

## Asset contract

- Wallpaper: existing packaged InfinityOS desktop wallpaper.
- Typography: packaged InfinityUI Arimo atlases.
- Icons: active packaged semantic icon theme, including Crystal Blue Glass, Professional Clear Glass, and Luminous Glass.
- No launcher-only image is loaded during bootstrap or installation. All dependencies are already part of the installed System Generation.
