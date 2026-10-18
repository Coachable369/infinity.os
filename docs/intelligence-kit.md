# InfinityOS IntelligenceKit v1

## Architecture and integration map

IntelligenceKit is a bounded, headless sidecar inside `kernel/runtime/ai`. It
does not own a model, dispatcher, compiler, storage engine, UI toolkit, or
distributed scheduler.

| Concern | Existing authority reused by IntelligenceKit |
|---|---|
| Voice and text ingress | AIKit `RequestApi` and the shared `AiRuntime::submit_chat_from` path |
| Fast commands | AIKit mapping plus `IntentRouter`; direct commands bypass planning |
| Ambiguous interpretation | The installed Hermes/local inference service; proposals remain data |
| Action discovery | `InstalledCapabilities`, projected from AIKit's authoritative mapping directory |
| Native software creation | DevKit/native C compiler and application lifecycle adapter |
| UI and application launch | UIKit/native application dispatcher adapter |
| Persistent state | Versioned ObjectStore object under `/system/intelligence/v1/<owner>` |
| Authorization | Native capability manager and final executor authorization check |
| Distributed resources | Existing Infinity Pool compute/resource services only |
| Progress | Structured IntelligenceKit events projected by voice, console, or graphical clients |

The public API version and persisted schema version are independently fixed at
`1`. CPU-specific execution remains behind existing native adapters, so the same
planner is built for x86_64 and aarch64.

## Native service interface

`Service::classify` distinguishes conversation, direct action, and complex goal
requests. `Service::propose` creates an inspectable plan. `begin`, `approve`,
`poll`, `interrupt`, and `cancel` implement the lifecycle. `inspect` and `event`
are read-only projections. `encode` and `restore` provide portable checkpoints.

Execution is dependency-injected through two narrow contracts:

- `CapabilityRegistry` describes actual installed facilities and resolves
  existing AIKit actions.
- `Executor` performs final authorization, starts an operation, observes native
  evidence, and cancels an exact execution lease.

An LLM cannot implement either interface. It may propose goal fields or tasks,
but the native validator rejects missing capabilities, invalid dependencies,
excessive limits, and forward/cyclic dependencies before execution.

## Goal and plan schemas

A `Goal` carries the original objective, desired outcome, success criteria,
constraints, context, missing information, required permissions, origin,
owner, and creation time. All text and collections are fixed-capacity.

A `Plan` contains at most 16 tasks. Each `Task` has a stable ID, description,
capability, up to four dependencies, typed inputs and expected output, a
verification contract, state, bounded failure policy, attempt count, execution
lease, and native evidence digest. Limits bound task count, concurrency,
replanning, and deadline.

## Planning, execution, and verification

Planning prefers mapped OS actions. DevKit is selected only when the objective
explicitly requires software and installed actions cannot fulfill it. Storage
and Pool work are represented by their existing service adapters rather than
new schedulers.

`poll` starts only dependency-ready tasks and never exceeds the parallel limit.
Sensitive, destructive, and externally consequential tasks wait for explicit
owner approval. Every task is reauthorized immediately before start. Completion
requires `Executor::observe` to return structured native evidence; generated
text is never success proof. A recoverable failure retries or replans only the
affected task and never exceeds the plan's limits.

Interrupting cancels active leases but preserves verified tasks. Restoring a
checkpoint keeps verified evidence, converts incomplete tasks back to pending,
and retains the original owner. The restored plan still must pass current
authorization, so persistence cannot elevate permissions.

## Frontend and DevKit integration

Voice, desktop text, application assistants, consoles, and authorized services
enter through the same AI runtime. Complex goals receive an immediate factual
plan summary while work remains asynchronous. Frontends should project
structured progress events, present task-level approval requests, and call
interrupt or cancel without blocking AudioKit or Hermes.

A DevKit executor should pass the objective as source-generation context, wait
for the native compiler's real status and diagnostics, retain the produced
artifact reference, and verify the application lifecycle launch receipt. It
must not infer compiler or launch success from assistant output.

### Native software workflow receipts

Application-creation requests recognized by the existing DevKit intent parser
enter the shared IntelligenceKit route for voice, typed input, and application
assistants. Simple commands such as opening an existing application retain the
direct action path. No game-specific or application-specific executor is added.

The native plan has two dependent tasks: generate/save/compile, then launch. An
explicit request for AI interaction adds a third task that observes the existing
application agent and its published action contract. The original objective stays
on the goal; task inputs refer to it without silently truncating source requirements.

`AiRuntime::software_goal_for_request()` binds an editor request only to the
matching goal command. `Panel.workflow_goal` is carried unchanged through the
queued request, inference lease, source drain and generated build lease. Trusted native services call
`record_software_event(goal, owner, intelligence::software::Event)` with:

| Event | Required native evidence |
|---|---|
| `SourceSaved` | Nonzero committed source ObjectStore ID and monotonic save revision |
| `Compiled` | Nonzero published executable ID, after source save and successful compile/link |
| `Launched` | Actual isolated application instance, after executable publication |
| `InteractionReady` | Granted owner/session agent and nonzero published revision with offered actions |
| `InteractionApplied` | Native Applied receipt, new revision and unique action token |
| `InteractionStopped` | Revoked or ended application interaction session |
| `Failed` | Final native failure after the existing bounded repair policy is exhausted |

The receipt observer rejects wrong goals, owners, empty identifiers, reordered
events, and duplicate action receipts. Opening the editor or accepting a dispatch
does not complete the generation task. The observer does not compile, launch,
invoke a model, grant permissions, or retry operations itself. Existing DevKit
diagnostic repair remains the sole owner of bounded compiler retries.

New voice, text or application requests interrupt the old plan and cancel only
its matching panels. Stale queued work, completed model output and compiler
callbacks cannot attach themselves to a newer active goal. Editing the document
after model completion but before final source drain cancels the pending apply
and compile while retaining the user's edits. A completed build cannot restart
generation or publish an executable after its originating goal is replaced.

Software goals have a 20-minute orchestration budget; other goals retain the
five-minute default. Generation itself has a ten-minute absolute limit and a
90-second no-progress watchdog. These limits preserve continuously progressing
local generation while still releasing stuck work.

Plan completion with interaction means the requested application is saved,
compiled, launched, and ready for authorized AI interaction. It does **not** mean
a game was won or a later user task was completed. `software_execution(owner)`
separately exposes the active interaction flag, applied-action count, last action
token, and latest verified revision. A user-first game may await the user's move.
Persisted plan evidence never restores an application grant: a resumed process
must establish its current instance and authenticated interaction authority again.

## Acceptance coverage

`./build-kit intelligence-kit-tests` behaviorally verifies direct-action
bypass, dependency graph construction, installed-capability reuse, DevKit
fallback, bounded recovery, safe checkpoint resume, shared voice/text ingress,
approval, and native authorization denial. Release builds additionally compile
the same module into both architecture kernels and package it through the normal
fresh-install path.

The suite also checks native software admission versus completion, differing
game/non-game goals through voice and text, ordered storage/compiler/launch
receipts, interaction readiness distinct from executed actions, stale owner/goal
rejection, and new voice-command admission after a failed plan. These deterministic
service tests are not evidence that real model generation, ARM execution, or
microphone/speaker playback succeeded; those require separate installed-VM runs.
