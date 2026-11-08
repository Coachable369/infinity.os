# Task Manager — daemon and scheduled-task workspace

## Reuse inventory and IDesign Kit

This workspace reuses `eclipse-precision-v1-reference.png` and its generated RGBA
atlas, `controls.md`, the packaged Inter typography, and the existing native
Task Manager window. No artwork gap exists: all controls already have approved
normal/focus/pressed/disabled recipes. Do not generate alternate window chrome,
icons, wallpaper, or borders. The compositor continues to own the outer frame.

| Element | Reusable recipe | Layout and states |
| --- | --- | --- |
| Tabs | Secondary / Primary button atlas | Processes, Services, Scheduled Tasks; selected tab uses Primary |
| Lists | Panel surface and Secondary selection | 40 px rows, selected row focus; real catalog entries only |
| Fields | Field atlas | Name, Python script, timing; focused border, proportional-font caret hit testing, select-all highlight and atomic single-line paste |
| Choices | Secondary surface | Cron/manual/boot/interval/once policy, enabled, restart policy |
| Actions | Primary / Secondary button atlas | Save/New primary; Start/Stop/Restart/Kill/Edit/Delete/View output secondary |
| Inline editor | Panel surface | Existing list alongside a persistent draft; Save and Cancel are explicit |
| Typography | Packaged Inter | 20/28 heading, 14/20 labels, 12/16 metadata |
| Spacing | Existing 8 px rhythm | 16 px outer gutter, 8 px gaps; 32 px actions |
| Feedback | Inline status region | Typed validation, stale edits, read-only authority, storage/clock recovery and lifecycle requests |

The content is a split workspace: a 200 px list and a flexible detail area at
normal Task Manager sizes. Rows show a readable name and typed lifecycle state.
The detail card exposes script path, enablement, trigger/timing, next run, last
exit, and run count. Editing uses labeled fields, native choices, and a separate
save row. Destructive deletion requires an inline confirmation; no modal is used.
No empty row or unbacked status is rendered as an actual task.

New Scheduled Tasks use the existing timing field for a standard five-field
Cron expression in UTC, initially `*/15 * * * *`. Its label names the field order:
minute, hour, day, month, weekday. The native choice menu adds Cron without new
artwork or geometry; Manual, Boot, Interval and Once remain available. Validation
focuses the expression and preserves the draft. The canonical command is
`taskscheduler create NAME /path.py cron="*/15 * * * *"`; the shared runtime
computes the next UTC execution. Canonical expression formatting is lossless and
bounded at 256 bytes. Long expressions use the existing measured field scrolling.

Once-only schedules use readable UTC `YYYY-MM-DD HH:MM[:SS]` dates; interval
and boot delay fields use seconds. Next-run dates are displayed in UTC. Disabled, manually stopped, manual-only,
completed Once, next-boot, and clock-unavailable states replace ineligible dates.
Manual timing is disabled, omitted from keyboard traversal and never parsed. Policy choices use the existing native
menu contract: opening preserves the value, arrows highlight, Enter commits,
and Escape cancels.

The Services and Scheduled Tasks tabs share a native catalog with the Command
Window. All mutations use the same parser and authorized persistence path. Read-only
sessions see disabled mutations with a local-administrator explanation. Selection
follows stable identity; an edit/delete stores its original durable definition.
A changed or replaced record preserves the draft and disables commit until the
user cancels and reviews it. Lifecycle telemetry alone does not create a conflict.
Created records are selected and revealed. View output opens the authorized
Command Window log route, preserving any active terminal editor, pending command
or remote interactive session by refusing to displace it.
Keyboard Tab / Shift+Tab traverses visible controls, arrows select list items, Enter activates
a focused control, Escape cancels a draft/confirmation, and normal text editing
is confined to the focused field. Ctrl+A selects all; Ctrl+C/X/V use the
session clipboard with atomic length validation. Invalid fields receive focus
and a specific recovery message, with the draft retained. Dirty drafts block tab changes until Save or
Cancel. Drafts and focus reset when the authenticated user or session changes. The Processes page preserves its existing monitoring and process actions.

## Review gates

The complete Task Manager toolbar has one painter: Processes retains its Task
menu; Services and Scheduled Tasks paint New directly on the toolbar substrate.
Never layer a disabled translucent action over an earlier action or glyph.
At content heights below 416 logical pixels, retain 32 px controls and 24 px
field labels, reduce the field rhythm to 56 px, and reserve a separate 32 px
feedback row. Compact browse metadata uses the existing 14/20 menu face. The
installed 1024×768 desktop with the normal 160/140/760/650 window placement
provides a 746×381 content rectangle and is a mandatory rendering fixture.
Save, Cancel and all browse actions must end before feedback begins, and stay
inside their panel; changing feedback must not alter any action's pixels.

Compare production-rendered pixels to the Precision reference: hierarchy, 8 px
spacing rhythm, glass edge softness, font quality, button state consistency,
proportions, and unclipped controls at the 620 px minimum width. Host pixel review
is separate from installed-runtime acceptance.
