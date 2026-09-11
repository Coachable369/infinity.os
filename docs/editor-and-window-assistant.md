# Native editor and app assistant

## Implemented

The Text Editor uses the existing InfinityOS renderer, file picker and persistence
path. It now supports selection (drag, Shift + navigation, or the Select tool),
copy/cut/paste within the editor, eight-step undo/redo, literal find and replace
all, go to line, duplicate line, automatic indentation, and an inline command
palette. Line numbers, caret location, selection and syntax colors share the
editor's actual viewport geometry. Opening and saving choose syntax mode from
the filename; the Syntax tool cycles modes manually.

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
currently overlays the right side of app content rather than reflowing it.

`help` explains the available operations. All apps expose reviewed maximize,
restore, minimize and refresh requests. The editor also supports `describe
document`, `find TEXT`, `insert TEXT`, `undo`, `redo`, `select all`, and `save`.
Requests propose actions; **Apply** executes them and **Dismiss** cancels them.
Editor proposals expire if the document, caret, selection or file identity has
changed. Inserted text participates in normal undo and save handling.

Other prompts use the shipped local dialogue runtime in an isolated request.
There is no external-model connection, automatic code generation, arbitrary
shell execution, or ambient access to app files. The current panel retains the
latest response, not a multi-turn conversation. It must not be presented as an
unrestricted AI coding agent.

## Verification and packaging

- `make editor-assistant-test`: typed editor operations, syntax categories,
  per-window assistant isolation, geometry and reviewed/stale action behavior.
- `make editor-window-test`: existing native editor/window behavior.
- `make x86_64 aarch64`: both live and installed kernels use these same modules.
- `python3 tools/editor-assistant-installed-test.py --artifacts-only`: checks the
  actual JetBrains Mono atlas bytes and feature contract in all four kernel
  artifacts. This is packaging verification, not installed execution evidence.
- `python3 tools/editor-assistant-installed-test.py /tmp/UNIQUE-RUN-DIRECTORY`:
  installs onto a newly created disposable disk, boots without ISO, then uses
  actual keyboard/pointer input and typed state to check editing, saving and
  independent app panels. QEMU and its x86_64 EDK2 firmware are required.
  Append `--live-only` for the same UI checks through the recovery Console's
  normal app-launch command; that mode deliberately excludes installation,
  authentication and saving and reports those checks as unverified.

The September 11 verification built both architectures, passed the host and
binary-packaging tests, and passed the live-only QEMU interaction suite: syntax,
selection, clipboard, undo/redo, find, reviewed AI insertion, window minimization,
and an independent Settings assistant. Editor and Settings screenshots were
reviewed after correcting glyph overlap and reply wrapping. Saving and installed
execution remain unverified: the fresh-install test stopped safely before
formatting because the current
installed kernel is about 173 MiB while the native layout reserves 128 MiB
(127 MiB usable for the kernel). `make installer-capacity-test` independently
rejects those artifacts. The storage boundary has not been moved, and no
installed-system parity claim is made until packaging is corrected and this test
passes. Do not promote these ISOs as a verified fresh-install release.
