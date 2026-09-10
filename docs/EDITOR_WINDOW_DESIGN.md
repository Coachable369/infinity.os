# Editor and window correction design kit

Reuse the installed theme's glass panels, font metrics, focus outlines, themed
folder/document icons and polished buttons. Network uses the existing generated
network artwork (role 17) in each selectable icon family, never the diamond fallback.

The object picker has a location field, parent navigation, a bounded paged list
of actual Pool namespace children, and a separate filename field for Save As.
Folder selection navigates; file selection opens persisted content. Save As
creates a distinct named object at the selected location. Duplicate names and
invalid paths remain visible errors rather than silent failures.

Keep panel gutters at 24 logical pixels, readable inherited typography, and
buttons aligned to the bottom action row. Use the same geometry for painting
and hit testing. Do not introduce another UI framework or background artwork.

Desktop windows share one back-to-front order for rendering and hit testing.
Only the topmost visible window under a click receives activation. Raising a
window preserves other windows' bounds and document state. Pointer capture
continues to own drag/resize until release. Focus changes repaint once; pointer
motion does not cause full-screen repaint.
