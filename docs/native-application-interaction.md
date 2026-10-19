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

## Generated application helper API

Include `<infinity/app.h>` in the generated application's single C translation
unit. It implements the declarations in `app_api.h` over the same UIKit and
interaction syscalls above; it is not another application runtime or an AI
permission grant. Use `interaction.h` directly for sensitive actions, references,
or richer schemas beyond these convenience helpers. Do not mix direct observation
publication with helper publication, because the helpers track their own committed
revision.

`app.h` also preserves the existing ABI-2 `InfinityAppApi`/`InfinityObjectApi`
entry and object callback declarations unchanged. The helpers supplement that
runtime contract; they do not replace it. Production CRT builds for both CPU
targets are required in addition to the helper boundary probe.

| Helper | Contract |
|---|---|
| `app_window(title, width, height)` | Open the themed native window; width 320–960, height 240–720, title at most 47 bytes |
| `app_button`, `app_label`, `app_display` | Upsert a stable positive UI element ID; positive size, nonnegative origin, label at most 31 bytes |
| `app_grid(title, rows, columns)` | Open a themed grid window once at startup: 1–9 rows, 1–12 columns, at most 95 cells |
| `app_cell(id, row, column, label)` | Create/update a 56-unit themed button at zero-based grid coordinates; keep the positive ID and its position stable |
| `app_status(text)` | Update the grid's reserved status label, at most 31 bytes |
| `app_offer(id, label)` | Advertise `app.activate` with four zero arguments for a successfully created cell; the AI receives the same ID as a user click |
| `app_state(ai_turn, terminal, text)` | Begin a replacement observation, nonempty UTF-8 state at most 2047 bytes; stage the next revision without publishing it |
| `app_choice(id, action, label, a, b, c, d)` | Add one currently legal reversible choice with four fixed integer arguments |
| `app_text_action(id, action, label, parameter, minimum, maximum, text_name)` | Add a reversible action with one bounded integer and up to 512 UTF-8 text bytes; other arguments are fixed zero |
| `app_publish()` | Atomically publish the complete staged state and choices, then advance the helper's committed revision |
| `app_event(&event)` | Nonblocking: `1` input, `0` idle, negative error; `APP_CLICK`, `APP_CLOSE`, or `APP_AI` distinguish the source |
| `app_done(&event, applied)` | Acknowledge the exact delivered AI token; applied success requires a newer committed observation |
| `app_yield()` | Yield through the native scheduler when idle |
| `app_run(render, handle)` | Run the shared event loop, rendering/publishing before applied receipts and yielding on every iteration |

Action and parameter identifiers start with an ASCII letter and contain only
letters, digits, dots or underscores. Action names allow at most 47 bytes,
parameter/text names 23 bytes, and nonempty choice labels 95 bytes. Parameter and
text names must differ. Choice IDs need only identify the published action; they
do not have to equal UI IDs. Recheck the app's current rules and input bounds in
the shared operation used by both UI and AI handlers. `AppEvent.a`–`.d` are signed
64-bit received arguments; `.text` contains at most 512 bytes plus a terminator.

The grid helpers only calculate geometry for existing UIKit controls. They use
20-unit margins, 56-unit buttons, 12-unit gaps, and a 32-unit status row; the
window grows within native canvas limits. An 8×8 board is 572×616 units; a 9×8
grid with an additional control row is 572×684, below the 720-unit height cap. Open
the grid once before `app_run`; redraw cell labels and status from `render`.
`INT32_MAX` is reserved for status and cannot be a cell ID. Moving a registered
ID to another cell or replacing another cell's ID is rejected. A successful new
`app_grid` or `app_window` clears the grid map; failed opens preserve it.
Do not overwrite grid-managed IDs with the lower-level geometry helpers.
Only offer cells that are currently legal, after drawing them successfully.
An invalid/missing cell offer or failed grid update poisons an active staged
contract. These helpers supply no game rules, turn rules, AI strategy, permissions
or additional render types; those remain the application's and OS's responsibility.

Check every return value. Publish once after startup and after each accepted
operation: **state → all legal choices/actions → publish**. A failed state or
action declaration poisons an active helper transaction; subsequent publication
is refused rather than exposing a partial contract. Recover by starting a fresh
successful `app_state` and rebuilding the full action set. A failed native commit
does not advance the revision and can be retried. Terminal observations publish
no legal choices; do not set `ai_turn` unless the AI should act now.

For an AI event, validate and apply the same operation as user input, update the
UI, publish the resulting observation, then call `app_done(&event, 1)` only if
all those steps succeeded. Reject invalid operations with `app_done(&event, 0)`.
An ACK failure is not success: retain/report the failure and never replay the
operation automatically. The kernel still checks session, grant, token,
deliver-once status, deadline, and committed revision independently of these
helpers. Stage-only changes cannot satisfy an applied ACK; replayed, revoked,
expired, and stale actions cannot obtain a successful receipt.

Prefer `app_run(int (*render)(void), int (*handle)(const AppEvent *))` for generated
apps. `render` draws the current state, stages its legal actions and returns
`app_publish()`; `handle` validates the common UI/AI operation and returns positive
for an accepted change, zero for rejection, or negative for a fatal error.
The helper renders once at startup, exits successfully on `APP_CLOSE`, polls
without blocking, and yields each iteration. It rerenders accepted changes,
requiring an actual newer committed revision before acknowledging AI success.
It rejects the AI receipt and exits on rendering/publication failure; polling,
handler and acknowledgment errors also exit without replay. Null callbacks fail
before any syscall. These callbacks contain app-specific rules; the helper does
not create game logic, choose AI actions, grant authority or add a scheduler.

Both helper headers are embedded in the compiler's `/sdk/infinity/` VFS for live
and installed kernels. `native-compiler-parity-test.py` compares their actual
bytes in both architecture artifacts. The behavioral host-C boundary probe is
`tools/native-app-sdk-test.c`; it checks packet fields, revision transitions,
failure propagation, fail-closed staging, typed input and receipt dispatch. Run
it through the build kit; this does not replace installed-app behavior checks.

Two independent generated-app cases live under
`tools/test-fixtures/native-applications/`. Supply them to the installed harness
with `--application-case`; they provide no application source or guest patches:

- `tic-tac-toe.json`: request a graphical 3×3 native grid with empty cell labels,
  human X starting, and the OS AI playing O. It checks nine empty cells, a real
  upper-left click, one X and one O after a delivered/applied AI move, return to
  the human turn, seven legal actions, and unchanged state after clicking the
  occupied square. The AI may choose any legal reply. This is one-exchange and
  illegal-move acceptance, not a complete-game/win/draw/strategy claim.
- `calculator.json`: request a four-function graphical calculator using a native
  4×4 grid with rows `7 8 9 /`, `4 5 6 *`, `1 2 3 -`, `C 0 = +`, and its numeric
  result in the grid status label. It checks digit entry, clear, addition,
  subtraction, multiplication, division, multidigit entry and idle stability.
  It does not claim decimal, divide-by-zero or arithmetic-overflow coverage.

Selectors resolve generated IDs from real numeric controls and kit geometry.
The specified layout and empty-cell encoding are test input requirements, not
values extracted from generated C. Cases deliberately fail on an incompatible
layout rather than silently selecting a different control. Their JSON presence
alone proves nothing; only a successful run against the generated installed app
with stored before/after UI and contract evidence constitutes case acceptance.

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

To diagnose a real generated application independently of the fixture games,
run the same host model harness with an arbitrary objective:

```sh
./build-kit run sh -c 'CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin application-interaction-model-test -- --generate "Create a graphical chess game and play against me" --target aarch64 --output-dir build/application-generation/chess-01 --deadline-seconds 600'
```

Choose a new evidence directory for every run. The harness records the exact
request/prompt, structured prompt-processing and generation progress, first-token
and first-editor-source times, incremental generated C snapshots, compiler/linker
diagnostics, and native ELF machine/type checks. `--target both` checks both
native targets using their existing application CRTs. Only actual model output
is compiled; the harness does not substitute sample application code.
`--model ministral` compares the already installed secondary model with the
default `--model hermes` using the same production prompt and workflow. This
host-only choice neither downloads models nor changes the OS's Hermes default.

This is host-Hermes generation and native cross-compilation evidence, not an
installed compiler or gameplay acceptance test. `result.json` explicitly leaves
`installed_acceptance` and `domain_behavior_verified` false. Domain acceptance
requires the generated application's actual state and validated interactions in
the isolated installed VM; a valid ELF or a visible window is insufficient.

`tools/native-editor-installed-test.py --generate-request "..."` drives the
same request through desktop keyboard input in its disposable installed VM.
Use its existing `--arch`, explicit ARM QEMU `--iso`, and repository-local work
directory arguments; invoke it only through `./build-kit run`. It records
prefill/source/editor-delivery progress and reads exported application contracts
and native UI element values. It never writes guest memory, substitutes source,
or touches the user's running VirtualBox VM. Presentation alone remains an
incomplete acceptance result.

The focused `--streaming-only` mode isolates model delivery from application
correctness. It requires two increasing editor deliveries while inference is
still running, captures actual token/byte/flush and narration checkpoint counters,
cancels through a real document edit, and admits an ordinary follow-up request to
the shared model before cancelling it again. The follow-up must preserve the
unsaved source; creating a second application would correctly require saving or
discarding that work first. Screenshots and typed receipts are
retained. This mode does not claim generated application, compiler, microphone,
or audible speech acceptance; it never substitutes generated source or writes
guest memory. It accepts an optional `--generate-request` and bounded
`--generation-timeout`, and otherwise requests a graphical calculator.
Add `--audio-checkpoints` to attach the existing emulated HDA duplex/WAV path,
retain content-free conversation/output/device diagnostics, and require a
minimum 1024 delivered source bytes, a completed mid-generation SourceStreaming
checkpoint, and non-silent recorded PCM during generation. After the real edit
cancels generation, it also requires the completed GenerationCancelled checkpoint
and non-silent PCM in the separate cancellation interval before admitting the
follow-up request. Use a genuinely longer objective for this mode; a naturally
completed short source is not a sustained-streaming pass.
The probe first cancels any restored goal through the supported desktop controls,
clears only the chat composer, and records quiescent/new-generation sequence
receipts and request-entry screenshots. This prevents a restored earlier goal
from being counted as evidence for the newly requested application.
The repository-local WAV and its measured interval are preserved; this verifies
guest audio output, not physical speakers or microphone recognition.
For an isolated terminal-speech regression, `--terminal-audio-only` replaces
`--audio-checkpoints` and cancels after two real source deliveries. It still
requires completed GenerationCancelled speech, independently non-silent terminal
PCM, and subsequent model admission. Its receipt explicitly does **not** claim
the 1024-byte sustained-generation checkpoint; retain the separate sustained
probe when using this narrower correction test.

After stopping its private VM, the generated-application harness mounts that
disk read-only through the native ObjectStore implementation. It exports the
saved generated C and ELF with object identities, versions, CRCs and SHA-256s.
The source must match the live editor's byte count/fingerprint, and the ELF must
match the actual publication object and CPU architecture. This checks persisted
artifacts rather than substituting host-generated code or copying chat prose.

The read-only diagnostic contract is version 1:

- `INFINITY_APP_GENERATION_SNAPSHOT`: 14 little-endian `u64` fields for generation
  status, prompt/processed/generated tokens, output/source/editor bytes, flushes,
  repairs and clock; matching even sequence fields bracket publication.
- `INFINITY_APP_INTERACTION_SNAPSHOT`: 18 `u64` fields: magic, version, sequence,
  instance, revision, agent turn, terminal, observation length, choices, schemas,
  commits, deliveries, applied receipts, rejected receipts, last choice, last
  token, last result (0 none / 1 applied / 2 rejected), ending sequence.
  `INFINITY_APP_INTERACTION_STATE` contains the published observation;
  `INFINITY_APP_INTERACTION_CHOICES` contains packed 416-byte choice/schema ABI
  records, with the typed flag in the otherwise reserved field at byte 12.
- `INFINITY_NATIVE_UI_ELEMENTS`: packed 64-byte UI element records under the
  existing native UI snapshot seqlock. These are actual generated IDs, kinds,
  rectangles and current display/cell values, not a separate test application.
  `INFINITY_NATIVE_UI_EVENT_RECEIPT` holds queued count, last event kind/ID and
  consumed count under that same seqlock, so fast handlers cannot hide delivery
  merely by immediately updating their UI.

Contract buffers clear on detach; diagnostic reads neither grant agent authority
nor dispatch actions. Applied receipts prove dispatch and a newer publication,
not independent domain correctness. Behavioral cases must corroborate app state
using real button input and expected board/document/display transitions.

### Bounded installed application cases

Pass `--application-case <repository-local.json>` with `--generate-request` to
execute a case against the generated application's actual controls. Case IDs
come from its runtime `ui-state.json`, not from scanning generated C. The runner
supports only `capture`, real pointer `click`, `expect`, and `expect_unchanged`;
there is no script evaluation, source replacement, guest-memory mutation or
direct action injection. It accepts at most 64 steps/1200 seconds, bounded waits,
and requires actual input plus at least two assertions before passing.

An `id` may also be a live selector, avoiding hardcoded IDs chosen by generated
source: `{"kind":2}` selects the unique numeric display, and
`{"kind":3,"number":7}` selects the unique numeric button. A measured board cell
can use `{"kind":3,"grid":{"rows":3,"columns":3,"row":0,"column":2}}`.
Grid coordinates are zero-based; optional logical `bounds:[x,y,width,height]`
exclude unrelated controls. The runner requires a complete unique rectangular
grid and refuses ambiguous selectors. The same selectors work in `board.ids`
and `ui_ids`; click evidence records the resolved actual runtime ID. Selection
is not an outcome oracle: expected numeric/board values remain independently
specified by the case.

An `expect` state can check `numbers` (`id`, `equals`), a `board` (`ids`, `cells`),
and typed `contract` fields. Contract comparisons are `eq`, `gte`, `gt`, `lte`,
`after` a named capture, or `unchanged_since` one. `initial` is captured
automatically. `save_as` preserves a successful expectation for subsequent
comparisons. A human-to-AI turn case should check independent expected cell
values and require revision/applied-receipt advancement, rather than treating
the model's claim or application receipt alone as proof.
For nondeterministic but legal opponent locations, a board may use `counts`
instead of or alongside exact `cells`, for example `{"X":1,"O":1,"":7}`.
Counts must account for every selected cell and reject unknown state codes.
Assert the user's selected square separately and require real AI revision and
receipt advancement; piece counts alone do not establish legal game behavior.

`expect_unchanged` names a `from` capture, `ui_ids`, `contract_fields`, and
`for_seconds` (1–30) to verify rejected input leaves the relevant state intact.
The evidence directory retains the exact case, each actual before/after UI and
contract snapshot, screenshots, and the final pass/failure receipt. A successful
result is scoped to the declared case, not a claim of complete chess rules or
general application correctness. No domain pass is reported without a case.

The runner itself has separate fixture-based behavioral checks:

```sh
./build-kit run python3 tools/native-application-case-test.py
```

Those validate assertion/dispatch/failure semantics only; they are explicitly
not model-generated application or installed-VM evidence.

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

### Buffered editor delivery and speech ownership

`generation_stream::Stream` advances a contiguous delivery cursor at most once
per 75 ms while inference is running. Each editor update appends at most 256
bytes; completing the model output does not bypass that per-update bound. The
final source must drain completely before saving or compiling. Partial protocol
terminators and unstable trailing whitespace remain buffered, never entering the
document. A C Markdown fence inside a framed compile proposal is also withheld
and removed consistently; an incomplete closing fence cannot authorize compilation.
Hidden model-token changes do not redraw the desktop: first/status transitions,
buffered editor delivery, and completion/failure remain visible. Ordinary chat
keeps its existing incremental presentation. The whole generated edit has one undo checkpoint. Auto-follow applies
only when the reader was already near the bottom.

Application inference scans for a complete source frame only when the model
service reports changed output. Prefill and worker-wait polls cannot change the
append-only output and no longer repeatedly parse the same source. Completion,
EOS, capacity and failure paths retain final validation. The real-model service
test checks byte-for-byte immutability on unchanged polls, append-only growth on
output events, and final-output retention after EOS. This optimization does not
change the 2 ms cooperative budget, source limits, paced delivery or admission
rules.

Every generation is fenced by its originating goal and the editor revision.
Replacing the goal or editing during either live streaming or the final drain
cancels the pending automatic apply/save/compile and preserves current document
contents. Progress narration distinguishes planning, generation, sustained source
delivery, source completion, save, compile, repair and launch. The sustained
delivery checkpoint requires both elapsed time and at least 1024 bytes actually
appended to the editor. Long prefill can announce one preparation checkpoint
after 20 seconds, but only while observed prompt-token processing advances and
before source generation begins. Unspoken preparation checkpoints are removed
when source arrives or the attempt ends; bounded repair resets this attempt's
checkpoint gates. A transient speech-output failure recovers the audio
session without cancelling a completed conversational reply's background work;
an explicit user stop retains cancellation behavior.

Narration is bound to the original goal and exact request, and prefill samples
must belong to the editor's model lease. Replacing or cancelling that request
revokes pending generation narration. Compiler and launch receipts can still
be announced after the editor's generation phase has completed. A cancelled
compiler job remains observable to its owner until the caller retires its lease,
so a later compile is not left permanently busy.

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

### Installed buffered-streaming verification (2026-10-18)

The isolated ARM calendar probe delivered 1074 source bytes through 355 editor
updates while inference remained active and completed the sustained-source speech
checkpoint. That run exposed a terminal-speech ownership race rather than a
streaming failure. Both conversation recovery and workflow narration now bind
cancellation and completion to their exact accepted speech request; explicit
user stop remains session-wide.

Final-kernel verification `20261018T204058998361Z-2884` passed: incremental source,
real edit cancellation, completed cancellation speech with 137398 non-silent PCM
samples, preserved source, and subsequent shared-model admission. Its installed
kernel SHA-256 is
`18b117ca34daf067508788ad446a6b98046d4ff959f81709f0fb307299b3c4ef`.
The sustained and terminal-only receipts remain separate; the latter does not
claim a sustained-generation checkpoint. These tests refresh a private installed
clone and verify guest audio, not a fresh release installation, microphone
recognition, or physical speakers. Release packaging is verified separately by
the clean-build manifest and parity checks.

Two generated applications with verified domain behavior remain unaccepted:
observed model output still has compiler/domain errors or exceeds bounded repair
time. General app requests with trailing feature descriptions can also miss the
current conservative intent matcher. Neither valid ELF output nor these
streaming/audio receipts establishes that broader acceptance.
