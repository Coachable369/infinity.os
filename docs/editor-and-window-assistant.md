# Native editor and app assistant

## Implemented

The Text Editor uses the existing InfinityOS renderer, file picker and persistence
path. It supports selection (drag or Shift + navigation),
copy/cut/paste within the editor, eight-step undo/redo, literal find and replace
all, go to line, duplicate line, automatic indentation, and an inline command
palette. Line numbers, caret location, selection and syntax colors share the
editor's actual viewport geometry. Opening and saving choose syntax mode from
the filename. The status-bar language selector and View → Syntax highlighting
open a named dropdown; arrows and Enter also select a mode.

The supplied editor IDesign kit is the visual reference: a landscape window,
16 px antialiased UI text, navy surfaces, soft borders, compact document identity,
and a slim status bar. File contains New, Open, Save, Save As, Delete and Close;
Edit, Selection and View expose editing commands without button rows. Menu hits,
code painting, scrolling and pointer selection use the same viewport geometry.

Syntax modes: plain text, Rust, Python, JavaScript, C/C++, JSON, shell, HTML and
CSS. These are lexical highlighters, not parsers or language servers.

| Shortcut | Action |
| --- | --- |
| Ctrl + N / O / S | New / open / save |
| Ctrl + Z / Y, Ctrl + Shift + Z | Undo / redo |
| Ctrl + A / C / X / V | Select all / copy / cut / paste |
| Ctrl + F / H | Find / replace all |
| Ctrl + G | Go to line |
| Ctrl + D | Duplicate current line |
| Ctrl + P | Inline command palette |
| Ctrl + I | Expand/collapse the current app assistant |
| Shift + arrows / Home / End | Extend selection |
| Escape | Close the active inline tool or clear selection |

Use Tab to switch between the find and replacement fields; Enter executes the
operation. Command palette entries include `undo`, `redo`, `select all`, `copy`,
`cut`, `paste`, `duplicate line`, `save`, `find`, `replace`, `go to line`, and
`syntax`. USB and PS/2 support modifier shortcuts; basic firmware text input can
conflate some control chords with Tab or Backspace, so mouse tools remain
available.

Unsaved edits prevent replacing the current document through New/Open. A failed
or oversized edit is rejected atomically. Current limits are **16 KiB of ASCII
text, one document, eight undo snapshots**, and an editor-local clipboard. This
is not complete Sublime Text parity: Unicode, multi-document tabs, projects,
multi-cursor editing, regular expressions, folding, completion and LSP are not
implemented.

Known polish issue: the most recent find-result notice can remain in the
document strip after leaving Find or editing. It does not change the selection,
document contents, or saved-state indicator.

## Core window rule

Normal application windows expose the same right-edge AI tab and expandable
in-window panel. This is integrated into the current desktop, Console, Text
Editor, Task Manager, Settings, and each File Navigator window, including
inactive-window rendering. Window geometry and hit regions come from
`kernel/ui/app_assistant.rs`; future apps must use this shared chrome and register
their own bounded actions instead of adding a separate chat surface. Secure
login/lock surfaces, installer screens, and modal file sheets are excluded.

Panel state is isolated by window and cleared with the editor clipboard on a
session change. A tab expands/collapses the panel; clicking the
composer focuses it, and clicking outside releases keyboard focus. The panel
reflows the editor viewport instead of covering its code. Other existing app
surfaces still use their shared overlay rail.

`help` explains the available operations. All apps expose reviewed maximize,
restore, minimize and refresh requests. The editor also supports `describe
document`, `find TEXT`, `insert TEXT`, `undo`, `redo`, `select all`, and `save`.
Requests propose actions; **Apply** executes them and **Dismiss** cancels them.
Editor proposals expire if the document, caret, selection or file identity has
changed. Inserted text participates in normal undo and save handling.

Other prompts use the shipped local dialogue runtime in an isolated request.
There is no external-model connection, automatic code generation, arbitrary
shell execution, or ambient access to app files. The current panel retains the
latest user prompt and response, not a multi-turn conversation. It must not be presented as an
unrestricted AI coding agent.

## Verification and packaging

- `make editor-assistant-test`: typed editor operations, syntax categories,
  menu-command mappings, named syntax state and reflowed popup/hit bounds,
  per-window assistant isolation, geometry and reviewed/stale action behavior.
- `make editor-window-test`: existing native editor/window behavior.
- `make x86_64 aarch64`: both live and installed kernels use these same modules.
- `python3 tools/editor-assistant-installed-test.py --artifacts-only`: checks the
  actual JetBrains Mono and compact UI atlas bytes and feature contract in all four kernel
  artifacts. This is packaging verification, not installed execution evidence.
- `python3 tools/editor-assistant-installed-test.py /tmp/UNIQUE-RUN-DIRECTORY`:
  installs onto a newly created disposable disk, boots without ISO, then uses
  actual keyboard/pointer input and typed state to check editing, saving and
  independent app panels. QEMU and its x86_64 EDK2 firmware are required.
  Append `--live-only` for the same UI checks through the recovery Console's
  normal app-launch command; that mode deliberately excludes installation,
The current kernel reservation accepts both real installed payloads; the older
128 MiB reservation failure was corrected separately before this UI change.
Build and artifact checks are distinct from disk-only runtime checks. The VM
suite records its exact verification scope in `result.json`; a live-only result
must never be described as proof of installed execution or persisted saving.

The September 11 menu/layout revision passed both host suites, the real-payload
capacity check, both architecture builds and four-kernel font/contract packaging
checks. QEMU x86_64 passed the live-only suite and a fresh install, detached-ISO
boot, configuration, authenticated cold boot, File → Open, named syntax selection,
history/clipboard/find, Save As, reviewed insertion, viewport reflow and independent
Settings assistant checks. Actual live and installed screenshots were reviewed;
the lingering find-result notice above remains a known visual issue.
