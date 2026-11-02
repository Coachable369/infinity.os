# APIKit reference

Use this guide with `C/include/infinity/` and `Python/infinity.py`, the shipped
source-level references. Header structures, constants and return types are the
authority for the installed SDK version. Start with [Getting started](GettingStarted.md).

## Python application API

Import `infinity as ui`; `_infinity_app` is supplied by InfinityOS, not PyPI.

| Call | Purpose |
| --- | --- |
| `window(title, width, height)` | Open the native application window; dimensions are client pixels |
| `label(id, x, y, width, height, text)` | Create/update a text label |
| `display(id, x, y, width, height, text)` | Create/update an inset themed display |
| `button(id, x, y, width, height, text)` | Create/update a clickable native button |
| `state(ai_turn, terminal, text)` | Begin a new published application state |
| `choice(id, action, label, a=0, b=0, c=0, d=0)` | Offer a semantic action with up to four integer arguments |
| `text_action(id, action, label, parameter, minimum, maximum, text_name)` | Offer a bounded integer parameter plus named text parameter |
| `publish()` | Commit the current state and offered actions |
| `event()` | Return an event dictionary or `None` when idle |
| `done(token, applied)` | Acknowledge an AI request with its actual result |
| `yield_native()` | Cooperatively yield to the OS |
| `run(render, handle)` | Render, dispatch events, republish changes and acknowledge AI |

IDs must remain stable and positive. Keep geometry inside the window and use
separate IDs for separate controls. Updating an existing ID changes that element;
it does not append another copy. Events use `kind` and `id`; AI events also carry
`token`. The exported kinds are `CLICK=1`, `CLOSE=2`, `AI=3`. Handle unknown kinds
and IDs by returning false without changing application state.

`run` invokes `render` initially and after `handle` returns true. It returns on
close. It acknowledges AI events after rendering; do not call `done` yourself
inside a handler used with `run`. Exceptions are not successful action results:
catch expected validation failures, return false, and keep diagnostics visible.
`text_action` uses the same typed contract as `app_text_action` in `interaction.h`.
Events also carry integer fields `a`, `b`, `c`, `d` and `text`; validate their
bounds before changing data. Native service rejection raises `OSError` in Python;
argument/type errors are exceptions, not successful completion.

## C header map

| Header | Responsibility |
| --- | --- |
| `app.h` | Inline high-level grid UI and application event loop |
| `app_api.h` | Application helper declarations and event data |
| `uikit.h` | Window/element/event structures and native UI calls |
| `interaction.h` | AI state, typed actions, revisioned requests and completion |
| `console.h` | Bounded interactive line input |
| `camera.h` | Authorized camera start, metadata, paged JPEG read and stop |
| `window.h`, `surface.h` | Lower-level native window and surface contracts |
| `font.h` | Native font identifiers and text rendering contract |
| `input.h`, `event.h` | Native input/event structures |
| `gui.h` | Scoped GUI capability and geometry interfaces |
| `syscall.h` | Architecture-selected syscall adapter and service numbers |

Prefer `app.h` unless you need custom geometry. `app_grid`, `app_cell` and
`app_status` render native controls; `app_state`, `app_choice`, `app_publish`
publish AI state; `app_run(render, handle)` provides the event loop. Follow
`../Examples/C/counter.c` for exact compilable signatures and ordering.

## Errors, permissions and portability

Check negative C service results before using output buffers. Do not assume a
successful window grants camera, microphone, network or storage access. A denied
service is not an invitation to bypass its capability checks. Polling loops must
yield, have bounded retries and stop when the application closes.

Use fixed-width types from headers at ABI boundaries. Do not serialize pointers,
copy internal Rust layouts, or hard-code architecture-specific instructions.
The installed compiler and Python runtime provide native adapters on ARM64 and
x86_64. Source portability does not prove hardware availability on either target.

## Troubleshooting

- Import failure on a development Mac/Linux host: run inside InfinityOS; the
  native extension is not a host Python package.
- Example cannot save: copy the individual file from the read-only SDK to your
  personal Documents folder first.
- UI freezes: remove blocking computation from the handler and yield while idle.
- AI action fails: ensure the window is running, publish current choices, validate
  the request and acknowledge once. See AIKit's quick start and reference.
- Unsupported third-party library: check native runtime/package compatibility;
  do not assume Linux wheels or host GUI toolkits work on InfinityOS.
