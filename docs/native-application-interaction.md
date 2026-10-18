# Native application interaction — implementation checkpoint

This is an **incomplete implementation checkpoint**, not a declaration that the
general generated-app interaction plan or chess acceptance has passed.

## Implemented path

`explicit user request → trusted app session → existing local model → validated
choice → isolated app syscall → new observation + receipt`

- `kernel/runtime/ai/application.rs`: version-one instance registry. Bindings are
  owner/session/launch-specific; no authority is inherited by a relaunch.
- `application_host.rs`: BSP-owned adapter for the current native process.
- `application_agent.rs`: cooperative decision loop using the existing model
  service. Desktop conversation is suspended/restored, not copied to the app.
- `application_decision.rs`: bounded prompt and strict choice parser. Model
  output selects a published choice; it cannot supply a syscall number.
- `sdk/c/include/infinity/interaction.h`: shared ARM64/x86-64 C ABI.
- `console_compile.rs`: checked process-memory transport and process teardown.
- `console_assistant.rs`: app-directed text/voice dispatch and cancellation.

Generated GUI applications are instructed to implement this contract. Explicit
“create ... and play me” requests carry their original objective through the
compile lease and authorize an interaction session after launch. Merely creating
or opening an application does not grant autonomous control.

## Version-one wire contract

All packets use little-endian fixed-width fields, version `1`, and no pointers
inside packets. The syscall's pointer and exact packet length are checked.

| Operation | Number | Packet | Behavior |
|---|---|---|---|
| Begin observation | `0x1150` | 2072 bytes | Stage revision, turn/terminal flags, up to 2048 UTF-8 bytes |
| Offer choice | `0x1151` | 192 bytes | Stage ID, risk, dotted action name, label, four signed arguments |
| Commit observation | `0x1152` | 8 bytes | Publish state and choices atomically |
| Poll request | `0x1153` | 72 bytes | Deliver token, instance, expected revision, deadline, choice and arguments once |
| Finish request | `0x1154` | 16 bytes | Acknowledge applied/rejected using the exact token |
| Offer typed action | `0x1155` | 416 bytes | Choice plus four named parameter constraints and bounded UTF-8 text |
| Poll typed request | `0x1156` | 592 bytes | Original request plus text length, typed flag and 512-byte payload |

Typed schemas support bounded integers, booleans, fixed values, and pinned
app-local object references. References cannot be substituted by the model and
are not OS handles. Each action supports one arbitrary UTF-8 text payload; larger
documents need app-defined bounded edits. Up to 16 typed schemas can be published
per observation. AIKit's `CapabilityDirectory::application_descriptor` exposes
live contracts only to the granted owner/session. This is not a second static
action inventory.

Generated non-game UI apps are instructed to use `InfinityAiAction` and
`InfinityAiTypedRequest` where finite choices cannot express their input space.
The original choice ABI remains an active contract; its poll operation rejects
typed requests without consuming them. Typed receivers accept both formats.

Successful completion requires a newer published revision. This is app-provided
execution evidence, **not an independent proof of domain correctness**. Tests
must separately inspect outcomes such as board cells or document contents.

Apps must use the same validated operation for UI and AI, recheck expected
revision and legal actions, then update the UI and publish the resulting state.
Never expose hidden cards, private opponent state, or unrelated documents.

## Bounds and safety

- Eight registry slots; currently one live native process is connected.
- 128 legal choices per observation, four fixed integer arguments per choice.
- Typed input constraints are revalidated before dispatch; UTF-8 length, integer
  ranges, boolean domains and pinned references are enforced.
- 256 actions per continuous session; 90-second model decision deadline;
  10-second contract-startup and action-acknowledgment deadlines.
- Sensitive proposals are quarantined in an inspectable review and displayed
  through the existing assistant Apply/Dismiss UI. A trusted approval must match
  the exact user, session, instance, revision and one-shot token. Reviews expire
  after two minutes or when app state changes. They do not hold the model lease.
- Undelivered cancellation is distinct from a delivered-but-unacknowledged
  unknown outcome. Neither is automatically replayed.
- Closing the app, changing user/session, disabling AI, locking the desktop,
  or cancelling the interaction revokes control. Grants are not persisted.
- Unknown or invalid model output pauses control; it does not execute a fallback.

## Verification entrypoints

Run only through the build kit:

```
./build-kit application-interaction-tests
./build-kit application-interaction-model-test
./build-kit native-uikit-tests
./build-kit full
```

The first exercises registry/host transitions, typed document edits, approval
quarantine, and compiles SDK layouts for both CPU targets. The second uses pinned
Hermes weights and observes actual fixture board/document changes through the
runtime agent. Neither substitutes for generated-C or installed-VM gameplay.

## Remaining acceptance work

1. Replace the native launcher/UI singleton with per-process window/session
   routing; prove two apps cannot receive one another's actions.
2. Extend the bounded scalar/text schema when real application contracts require
   collections or multiple text fields; add finer action-family grants. Current
   grants are scoped to a live app instance, with per-operation sensitive review.
3. Integrate ongoing interaction tasks and evidence into IntelligenceKit plans,
   including inspect/resume behavior without persisting ambient authority.
4. Verify generated contract behavior, not just the presence of declarations or
   a successful C compile. Add bounded compile/contract repair using diagnostics.
5. Exercise chess rules and turns with real Hermes, then other game/app domains,
   including hidden information, edits, drawing, cancellation, crashes and restart.
6. Complete deterministic regression tests for the live model-session scheduler
   and audio interruption, plus measured latency/resource limits.
7. Build fresh dual-architecture ISOs, verify embedded SDK parity, and test both
   live and newly installed boot paths without the ISO attached.

Do not describe this checkpoint as a complete general app-agent platform.

## Installed verification checkpoint (2026-10-18 UTC)

The `7f1309cf` clean release passed dual-architecture packaging and checksum
verification (`20261018T000824196272Z-42401`). ARM64 QEMU-compatible media
installed and cold-booted without ISO media, but UI app generation hit its
no-progress watchdog before any source appeared. The x86_64 software-emulated
installation was interrupted while still writing; it is not an acceptance pass.

The watchdog now counts actual prompt-processing token progress as well as
accepted output, retaining the five-minute overall limit. Real-Hermes regression,
protocol/ABI checks, and both kernel builds passed in
`20261018T020450513183Z-51622`. Installed retesting and new release packaging of
this correction remain required; the earlier ISOs do not contain it.

The updated installed clone still failed to reach compilation
(`20261018T021357284760Z-53772`). An exact-prompt Hermes diagnostic reproduced
unframed source, invented UIKit calls, and repeated unfinished code. Source
preview now accepts raw C only for an explicit app-creation request, while
completion and compilation remain gated. The prompt now includes the actual
UIKit header rather than expecting the model to know private APIs.

With that reference, generation completed, but real ARM64-targeted compilation
failed on an invented `infinity/ui.h` include; the generated program also lacked
the requested calculator semantics and interaction hooks
(`20261018T023947671033Z-55162`). These are **failed acceptance results**. Do not
replace them with a hand-written calculator or claim the generated-app plan is
finished. Compiler-driven bounded repair, generated-contract execution checks,
and domain-behavior acceptance are still required before general app generation
can be claimed. The `--generate` diagnostic on the model test preserves its
fixture output under `build/` and deliberately fails when generation is incomplete.

## Bounded compiler recovery checkpoint (2026-10-18 UTC)

Generated applications now queue at most two compiler-diagnostic repair attempts
through the existing model service. Admission requires the original request,
unchanged editor revision, active session and compiler publication authority.
Manual builds, cancellation and empty diagnostics do not trigger repair. Source
that cannot fit the bounded repair context is retained rather than truncated.
Editing during streaming cancels the model lease; generated previews cannot
overwrite that edit. Each attempted source remains saved in Documents.

Raw C beginning with complete function comments is now recognized for streaming
and final staging, exclusively for an explicit creation request. Comment-only,
incomplete comments and conversational prefixes remain rejected.

The host repair probe passed: real Clang rejected a missing semicolon, Hermes
corrected the source, and the resulting executable returned status 42. Typed
tests exercised revision/request fences, two-attempt exhaustion and completion
gating. This is **host evidence, not installed native acceptance**.

The initial stricter dual-target rerun stopped at the correction-loop limit. Its
`-nostdinc` compiler invocation cannot find the compiler-provided `stddef.h`
required by the SDK's `stdio.h` (manifest
`20261018T042944167639Z-57691`). Fix the probe's builtin-header search path to
match the real toolchain before rerunning `./build-kit
application-generation-repair-test`. Do not add a fake SDK header or remove the
cross-target check. The combined run short-circuited before interaction regressions
and kernel builds, so those checks remain pending for this change.

No fresh ISOs were produced for this checkpoint. General GUI generation, real
calculator behavior, generated AI hooks, chess play and installed-runtime
acceptance remain unproven. Successful compilation alone is not their acceptance
criterion.

### Compiler-header correction

The repair harness now obtains the selected Clang's resource directory and adds
its builtin include directory explicitly, retaining `-nostdinc` and the native
SDK headers. A valid header-dependent C preflight must compile for both targets
before a compiler failure can be supplied as repair evidence. No host libc
headers or substitute SDK headers were added.

`./build-kit application-generation-repair-test` passed in
`20261018T044405195381Z-58448`: real model repair, ARM64 and x86-64 compilation,
host execution returning 42, and subsequent model-lease reuse. This resolves the
`stddef.h` harness failure above, not the separate GUI/chess acceptance gaps.

The real-Hermes interaction regression also passed
(`20261018T044444495263Z-58543`), followed by the registry and both target C ABI
checks (`20261018T044531596644Z-58642`). Clean release build
`20261018T044546039521Z-58672` passed at code revision `b46d47ee`, including both
kernel builds and release packaging/parity checks. It produced current
`builds/InfinityOS-aarch64.iso` and `builds/InfinityOS-x86_64.iso` containing the
compiler-recovery and source-preview changes. Independent verification of both
entries in `builds/SHA256SUMS` passed in `20261018T062223023626Z-64593`.

These replace the stale-ISO status for this checkpoint. No installed-VM
calculator/chess or generated-interaction acceptance is claimed by this release.
