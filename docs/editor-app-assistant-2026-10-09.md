# Editor app assistant

## Scope

- Route app prompts to the loaded native language model, not an uninitialized
  throwaway `ChatRuntime`.
- Generate text/code using the current document or selected passage as context.
- Apply generated insert/replace proposals through atomic, undoable editor edits.
- Execute explicit insert, clear, undo, redo, find, select-all and save commands.
- Honor the authenticated AI permission without requiring microphone permission.
- Keep desktop conversation history and speech separate from app inference.

## Behavior

The shared implementation is used by live and installed kernels on both targets.
No new runtime assets or host services are required. Existing design-kit panel
geometry, artwork and Apply/Dismiss controls are reused.

Native generation runs cooperatively. One window borrows the model at a time;
desktop token history is suspended and restored without retaining app context.
Changing owner/window, closing the panel, or revoking permission cancels the job.
Generated edits require Apply and a matching document/path/selection fingerprint.
An engine end token, not model-generated prose, establishes completion. Failed,
cancelled and capacity-truncated generations cannot become edits. Generated code
is never executed by the OS assistant.

`save file` invokes the normal storage workflow. Untitled documents open Save As;
success is reported only after a successful write. `clear text` is undoable.
Explicit editor commands execute on submission; window-management commands and
generated edits retain review. Context and output are bounded; oversized content
is rejected rather than silently applied in part.

## Verification

- `./build-kit run cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin ai-test -- --app-native`
  passed with the pinned Hermes weights. The model answered a greeting, generated
  a Python addition function and newsletter prose, and the production panel/document
  API applied and undid both edits. The test checked the code AST and evaluated two input/output cases in a
  restricted namespace. It also verified unchanged desktop chat state, competing
  request rejection, retained desktop token history on resume, owner-change
  cancellation, and disabled-AI rejection.
  Final manifest: `20261009T162502071894Z-10436.json`.
- `./build-kit run make editor-assistant-test ai-test` passed. Focused editor
  assertions cover document bytes/history, command routing, stale edits, malformed
  actions, cross-app rejection, incomplete generation, and size limits.
  Manifest: `20261009T154202986932Z-99181.json`.
- Native test artifacts: `build/app-assistant-generated.py` and
  `build/app-assistant-generated.txt`.
- The installed acceptance script now sends insert/clear/undo/save through the
  actual panel, including Save As, and inspects typed guest state.

## Images and remaining acceptance

- The x86 full-bundle ISO was rebuilt with the focused build profile. The bundle
  check and byte-for-byte installed-kernel payload parity passed.
  Manifest: `20261009T154243039495Z-99509.json`.
- The ARM full-bundle ISO was rebuilt with the same shared implementation. Its
  bundle and installed-kernel payload parity checks passed.
  Manifest: `20261009T160743019422Z-4955.json`. ARM installed interaction acceptance
  has not been rerun for this change.
- Fresh installed interaction acceptance **did not run to the desktop**. The
  original 4 GB QEMU fixture could not allocate kernel pages. An 8 GB retry reached
  kernel loading but did not publish initial guest state within 120 seconds; a
  bounded 300-second retry also failed at that same pre-desktop stage. No editor
  interaction assertions or installed screenshots passed in these attempts.
  Final manifest: `20261009T160142977719Z-4724.json`.
  Evidence: `build/editor-app-installed-20261009-8g-long/node-1/installer.log`,
  `failure-registers.json`, and `failure.ppm` in that same directory.
- The installed test now uses an 8 GB fixture and a five-minute startup bound.
  It still requires a successful run before installed interaction acceptance can
  be claimed. No user-owned VM or disk was modified.

These are incremental/focused builds, not a clean release build. Host-model
success and packaged-kernel parity are not installed-VM acceptance.
Images were built from the shared working tree, including pre-existing edits;
those unrelated edits are preserved and excluded from this change's commit.
