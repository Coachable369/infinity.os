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

Successful completion requires a newer published revision. This is app-provided
execution evidence, **not an independent proof of domain correctness**. Tests
must separately inspect outcomes such as board cells or document contents.

Apps must use the same validated operation for UI and AI, recheck expected
revision and legal actions, then update the UI and publish the resulting state.
Never expose hidden cards, private opponent state, or unrelated documents.

## Bounds and safety

- Eight registry slots; currently one live native process is connected.
- 128 legal choices per observation, four fixed integer arguments per choice.
- No dynamic string/object argument schema in this initial ABI.
- 256 actions per continuous session; 90-second model decision deadline;
  10-second contract-startup and action-acknowledgment deadlines.
- Sensitive choices are not sent to the model and cannot be dispatched without
  explicit approval. An approval UI has not yet been implemented.
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

The first exercises registry/host transitions and compiles SDK layouts for both
CPU targets. The second uses pinned Hermes weights and observes an actual
fixture board change. Neither substitutes for installed-VM gameplay.

## Remaining acceptance work

1. Replace the native launcher/UI singleton with per-process window/session
   routing; prove two apps cannot receive one another's actions.
2. Add full typed parameter schemas, variable text/object references, scoped
   capability grants, inspectable action discovery through AIKit, and an explicit
   sensitive-action review path. Fixed legal choices alone do not cover general
   document editing or data applications.
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
