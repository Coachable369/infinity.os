# APIKit — build a native application

## Python quick start

Copy `/InfinitySDK/Examples/Python/counter.py` to your Documents folder, open the
copy in Text Editor, and press Run. Stop terminates the running program; Restart
runs the saved script again. Keep your program's editable source in your own
workspace, not in the SDK. `import infinity` binds to native OS services.

The counter opens a native window, publishes two buttons and its current value,
and handles mouse and AI actions through the same event handler. Use
`infinity.run(render, handle)` so close requests, cooperative scheduling and AI
acknowledgements are not forgotten. Return true only when an action was applied.

## C quick start

Copy `/InfinitySDK/Examples/C/counter.c` into Documents and open it in Text Editor.
Run uses the installed native compiler/linker and application lifecycle service.
Use `<infinity/app.h>` for the grid application helper; its implementation and
dependencies are included in APIKit. The installed compiler supplies its native
headers and runtime automatically. No host compiler flags or Linux libraries
are needed for the editor Run workflow.

For a custom layout use `<infinity/uikit.h>` and `<infinity/interaction.h>`.
Always check negative service results. Dispatch only known control/action IDs,
validate arguments and keep long work out of the event loop.

## Supported interfaces

| Interface | Public contract |
| --- | --- |
| Python `infinity` | window, label, display, button; state/choice/text_action/publish; event/done/yield_native; run |
| C `app.h` | Grid UI, status, bounded action registration and unified event loop |
| C `uikit.h` | Native window, label/display/button upsert, event polling and yielding |
| C `interaction.h` | Revisioned state, choices, typed actions, request polling and completion |
| C console/camera/font/surface/gui headers | Low-level native service contracts; inspect the shipped header for limits and required grants |

The OS has additional shared checkboxes, radio buttons, selectors and other
controls. Their internal Rust renderer is not yet a general public C/Python
control factory. Do not assume a `checkbox()` or `dropdown()` function exists in
`infinity`. Python Tk support is a separate native runtime interface, not a
replacement or duplicate of the authoritative Infinity UIKit renderer.

## Dependencies and execution

Prefer the bundled standard library and installed packages. Inspect available
modules before generating imports; not every CPython extension or POSIX package
is compatible with InfinityOS. Network installs require the native pip/TLS
services and a compatible package. Do not embed secrets or download executable
dependencies merely to display the example UI.

Application lifecycle, ownership, launch registration and permissions remain OS
responsibilities. Publishing AI actions does not register an executable for
installation or grant unrestricted storage, microphone, camera or network access.
Keep a saved source file for edits; never treat an AI's textual claim as proof
that an application installed or a service action succeeded.

## Development checklist

- Save, run, stop and restart the application in its owner session.
- Exercise click, close, resize and AI requests; reject invalid/stale requests.
- Test on each architecture you intend to distribute.
- Use current OS theme resources; do not paint competing outer window frames.
- Keep event handlers bounded and yield while waiting.
- Publish visible state again after successful actions.
