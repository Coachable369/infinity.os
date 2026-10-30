# InfinityCode — initial native Python workspace

Reuse Eclipse Precision's OS-owned outer frame and existing Inter typography.
No alternate window borders or new icon family. Content uses navy #090F26,
surface #101B3A, border #86C7FF, text #FFFFFF and accent #3E6FFF.

Layout: compact action bar; left component/layer list; central Designer,
Python and Event Methods tabs; right numeric/text properties; persistent bottom status. Use
installed native Tk controls, not screenshot stand-ins. Designer controls are
the actual widgets exported to Python, positioned in a fixed 640×420 canvas.
Selection outline and resize grip are editor-only and never exported.

Initial contract: add/select/move/resize/delete controls, ten-pixel snapping,
edit text and geometry, undo/redo, save/load versioned JSON, export editable
Python. Explicitly regenerating Python asks before replacing manual source
changes. Export never executes edited code. Projects and exports use the
application's capability-scoped /data directory on InfinityOS.

This first version is not a debugger, package manager or arbitrary-code runner.
Run exported Python through the existing Text Editor/DevKit workflow.

Event Methods source is stored independently from generated layout code.
Set an interactive control's Handler property to a method defined in that tab.
Buttons, checkboxes and radios invoke it on activation; entries on Return;
comboboxes on selection. Methods take no arguments and access named widgets
through `controls`. Regeneration retains method bodies and rejects undefined
bindings. Existing version-1 projects without handlers remain supported.

Application lifecycle uses the shared native window service: voice/AI open,
focus, close, minimize, maximize and restore resolve the authenticated user's
owned window. Close requests pass through the IDE's unsaved-project confirmation;
they do not terminate the Python process behind that confirmation.
