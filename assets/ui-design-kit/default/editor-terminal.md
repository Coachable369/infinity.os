# Editor terminal dock

Reuse the approved editor IDesign Kit, current document palette, native Inter
labels, measured monospace font atlas, shared rounded control plates and cyan
focus treatment. No new icon family or raster decoration is needed.

The collapsible bottom dock reserves a 32px header with Terminal and real Run,
Stop and Restart controls. Expanded height defaults to 224px and is draggable
from a 6px top splitter; at least 72px of code remains visible. Layout preferences
persist per authenticated user; transcripts and input never cross users.

Output follows an 18px row rhythm with bounded native scrollback. Long output
soft-wraps to the available width; resizing preserves a scrolled reader's logical
row and byte position. Wrapped diagnostics still navigate using the original
complete output row. The input has
a visible caret, horizontal reveal and independent focus. Compiler and Python
locations matching the current file are cyan and navigate to source on click.
Collapsed docks retain running processes; Stop cancels, and Restart waits for
native cleanup before starting a saved source snapshot. C compilation and
Python execution use existing DevKit/native jobs, not a simulated console.

Verification: `tools/editor-terminal-test.rs` exercises geometry, preferences,
stream buffers, diagnostic coordinates and stdin. Production pixels and fresh
installed interaction must be reviewed before final acceptance.

Generated application entries reuse the existing launcher cell, native editor
document glyph (role 49), search, selection, scrolling and transition states.
Their labels and stable source identities are a disposable projection of the
authenticated user's native catalog. Launching always revalidates the actual
source and executable. Built-in reorder/desktop-shortcut persistence is unchanged.

## Native Python/Tk windows

Reuse the same glass header, generated three-state window control plates,
antialiased Inter title and themed outline. The 44px title is outside the client
viewport; side/corner resize hit areas likewise do not consume widget pixels.
Close sends an application request, allowing a real save/cancel callback.
Minimize collapses to the recoverable title; clicking that title restores it.
No replacement widget icons, font family or decorative artwork are introduced.
