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
| Fields | Field atlas | Name, Python script, timing; focused border and visible caret |
| Choices | Secondary surface | Boot/manual/interval/once policy, enabled, restart policy |
| Actions | Primary / Secondary button atlas | Save/New primary; Start/Stop/Restart/Kill/Edit/Delete secondary |
| Inline editor | Panel surface | Existing list alongside a persistent draft; Save and Cancel are explicit |
| Typography | Packaged Inter | 20/28 heading, 14/20 labels, 12/16 metadata |
| Spacing | Existing 8 px rhythm | 16 px outer gutter, 8 px gaps; 32 px actions |
| Feedback | Inline status region | Success, invalid input, failed persistence and unavailable state |

The content is a split workspace: a 200 px list and a flexible detail area at
normal Task Manager sizes. Rows show a readable name and typed lifecycle state.
The detail card exposes script path, enablement, trigger/timing, next run, last
exit, and run count. Editing uses labeled fields, native choices, and a separate
save row. Destructive deletion requires an inline confirmation; no modal is used.
No empty row or unbacked status is rendered as an actual task.

Once-only schedules use readable UTC `YYYY-MM-DD HH:MM[:SS]` dates; interval
and boot delay fields use seconds. Next-run dates are displayed in UTC rather
than exposing internal epoch counters. Policy choices use the existing native
menu contract: opening preserves the value, arrows highlight, Enter commits,
and Escape cancels.

The Services and Scheduled Tasks tabs share a native catalog with the Command
Window. All mutations use the same parser and authorized persistence path.
Keyboard Tab traverses visible controls, arrows select list items, Enter activates
a focused control, Escape cancels a draft/confirmation, and normal text editing
is confined to the focused field. Dirty drafts block tab changes until Save or
Cancel. Drafts and focus reset when the authenticated user or session changes. The Processes page preserves its existing monitoring and process actions.

## Review gates

Compare production-rendered pixels to the Precision reference: hierarchy, 8 px
spacing rhythm, glass edge softness, font quality, button state consistency,
proportions, and unclipped controls at the 620 px minimum width. Host pixel review
is separate from installed-runtime acceptance.
