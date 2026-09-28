# Setup persistence and default listening

## Changes

- New users default to continuous local listening after authentication. The
  desktop retries startup every five seconds while the model/capture route is
  unavailable, without interrupting an active voice transaction. Existing
  enabled profiles also resume; explicitly disabled profiles stay disabled.
- AI-widget microphone control, Settings, and `voice stop` persist the opt-out.
  `voice listen` enables it again. Lock/session authority checks remain active.
- Identity, network, and shell settings now use bounded copy-on-write checkpoints
  instead of adding permanent object versions on every save.
- Setup commits its completed state before entering the desktop. A failed save
  restores the previous retryable identity state and keeps setup open.

## Defect and evidence

The native object store has 64 global version records. The prior identity write
path exhausted these records with repeated configuration writes, and setup
ignored persistence failure. The regression reproduces that exhaustion using
the actual append-only write API, then verifies checkpoint replacement recovers.

`settings-reboot-test` uses real file-backed sectors and the production object
store, not a mock persistence function. It verifies 200 successive saves,
completed onboarding, a saved appearance value, successful password
authentication, spoken-reply settings, and a durable microphone opt-out across
independent file close/reopen/remount cycles. A read-only media write fails and
the last committed state remains intact.

Passed through build-kit:

- `sh tools/ai-test.sh`, including the new disk regression and activation test:
  `builds/manifests/20260928T002722357253Z-9519.json`.
- Production voice controller lifecycle tests: five passed;
  `builds/manifests/20260928T002738486528Z-9568.json`.
- Native `cargo check` for ARM64 and x86_64 with browser enabled:
  `builds/manifests/20260928T002804493611Z-9612.json`.

These are behavioral persistence/controller tests and native compilation checks,
not a cold-installed desktop VM reboot test. No existing VM was changed. ISO
files from the preceding rebuild do not yet contain this change. Changes live
in the shared installed/live kernel source and the disk regression is included
in the normal AI/build test entrypoint. Settings never successfully saved by the
old build cannot be reconstructed; they must be configured again on the fixed
build.
