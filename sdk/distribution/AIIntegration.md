# AIKit — local AI application integration

AIKit connects application intent to the installed local LLM and OS action
broker. The application does not host another model. It publishes meaningful
state and authorized actions; the OS interprets the user's request and dispatches
a structured request to the application. Mouse and AI input must share the same
validated action implementation.

## Lifecycle

1. Open the native application window.
2. Publish a concise, truthful state description. Exclude secrets, invisible
   private documents, passwords and unrelated user data.
3. Offer only currently legal actions, with stable IDs and descriptive labels.
4. Commit the state/action revision with `publish()` / `app_publish()`.
5. Poll events. Validate ID, arguments, current state and permissions before
   executing a request. Never interpret action parameters as shell commands.
6. After success, publish updated state. Acknowledge the request once, with the
   actual applied/rejected result; do not report success in advance.
7. Continue yielding, and handle window close normally.

`Examples/Python/counter.py` and `Examples/C/counter.c` demonstrate this complete
loop. The Python run helper and C app_run helper perform the polling and
completion protocol. Do not acknowledge the same request a second time when
using these helpers.

## Python

`state(ai_turn, terminal, text)` describes current state. `choice(id, action,
label, a, b, c, d)` publishes a bounded action (see the example for default
arguments). `text_action` adds a typed parameter contract. `publish()` makes the
revision visible. `event()` returns click, close or AI events; AI events carry
a completion token. `done(token, applied)` reports the real result.

Use semantic action names such as `counter.increase`, not a table of every
possible spoken phrase. The local model can infer equivalent requests from the
published descriptions. The OS still validates and authorizes execution.

## C

`interaction.h` defines state revision, choice ID, risk, arguments, instance,
token and deadline fields. Actions support fixed, integer, boolean and reference
parameters with bounds, plus bounded text. Respect the limits in the header;
do not truncate user input silently into a different operation. The high-level
`app.h` helper combines UI interaction with this protocol.

Read-only, reversible and sensitive risk classes describe the action; declaring
one is not a permission grant. Sensitive operations remain subject to the OS's
authorization and approval policy. Validate even a request originating from AI.

## Scope of the public API

This SDK supports local-LLM **control of your application**. It does not currently
expose an arbitrary `llm.generate(prompt)` API, model loading, unrestricted
conversation-memory access, or raw microphone ownership through `infinity`.
Do not invent those APIs. Wake-word recognition, transcription, inference,
speech output and recovery are owned by the OS voice pipeline.

System service and intent dictionaries under `/system` describe OS endpoints.
They are distinct from an application's revisioned action list. Editing a
dictionary cannot create a missing service or bypass authorization. The included
`Architecture.md` explains the internal Intelligence Kit, clearly separate from
the public application ABI.

## Acceptance checklist

- A click and a semantically equivalent spoken request cause the same state change.
- Disabled, unknown, stale and out-of-range actions do not mutate state.
- State is republished after a change and the OS receives one completion result.
- Closing the window ends the event loop without leaving a pending action.
- Long operations stay responsive and report failure honestly.
- Offline AI control works with the OS's installed model; internet is needed only
  for actions that intrinsically access online resources.

## Persistent owner memory

See [PersistentMemory.md](PersistentMemory.md) for native APIs, CLI examples, format, privacy policy and limits. Lexical recall is available; semantic embedding generation requires a validated native model.
