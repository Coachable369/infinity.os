# AIKit quick start and reference

## Run a complete integration

Copy `/InfinitySDK/Examples/Python/counter.py` to Documents and run it in Text
Editor. With the window open, use the desktop AI widget to ask Infinity to
increase the counter. The application offers `counter.increase` and
`counter.decrease`; equivalent wording is interpreted by the OS, not matched
against a list of phrases in the sample. A click and an authorized AI request
reach the same handler. The display should change once, not twice.

Use the installed local model. No API key, model download or cloud connection is
needed for this integration. A working OS voice pipeline is needed for spoken
requests; typed AI input exercises the same application action boundary.

## Contract reference

| Stage | Python | C helper |
| --- | --- | --- |
| Begin state | `state(False, False, description)` | `app_state` |
| Offer action | `choice(id, semantic_name, label)` | `app_choice` |
| Commit revision | `publish()` | `app_publish` |
| Receive request | `event()` | `app_event` |
| Apply and acknowledge | `run(render, handle)` | `app_run` |

For low-level C use `interaction.h`: state includes a revision; requests include
instance, token, revision, deadline and arguments. Fixed, integer, boolean and
reference parameters have explicit bounds; text is bounded. Risk classes are
read-only, reversible and sensitive. The header is the exact ABI reference.
Do not downgrade a sensitive action to avoid approval or trust AI-generated text
as executable code. Reject stale, unknown, out-of-range and unauthorized actions.

## Extend the example

To add Reset, create a button with a new ID, offer `counter.reset`, and handle
that ID by setting the value to zero. Return true only after the change. Publish
state from the value actually displayed, not from the requested value. Avoid
registering actions that cannot currently succeed.

## Diagnostics and scope

If nothing happens, first verify the mouse action. Then verify the app is open,
its current choices were published, and the AI completion is acknowledged once.
Keep expensive work outside the dispatch loop. Report permission or validation
failure without modifying state. `Integration.md` covers security and lifecycle;
`Architecture.md` describes the internal Intelligence Kit, not additional public
functions. Direct arbitrary inference, conversation-memory access and ownership
of the OS wake-word microphone are not public application APIs in this SDK.
