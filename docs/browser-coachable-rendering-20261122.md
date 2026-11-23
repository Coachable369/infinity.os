# Coachable native browser rendering correction

## Scope and cause

The native software renderer aborted when Coachable used CSS blend modes.
WebRender selected a GPU blend-factor shortcut for `screen`/`exclusion` that
SWGL's blend-key table does not implement. Select SWGL's supported coherent
advanced blend equations instead. This is shared by both native architectures.

The component previously classified every C abort as memory exhaustion. It now
reports an engine failure when the allocator has not recorded a failed request,
and retains the memory failure classification when a rejected allocation exists.
The allocator records its latest failure; this is diagnostic evidence, not proof
that a prior allocation failure caused a subsequent unrelated abort.

No audio pipeline inputs were changed. All 24 files in the saved audio freeze
baseline matched during verification.

## Behavioral evidence

- `builds/evidence/coachable-oom-before/result.json`: original software-renderer
  assertion; peak 298,085,824 bytes, no rejected allocation.
- `builds/evidence/coachable-blending/result.json`: actual native pixel checks
  for screen and exclusion blending passed.
- `builds/evidence/coachable-hvf-large/result.json`: live HTTPS Coachable at
  1920 x 1080 reached document completion and decoded every initial-viewport
  image; screenshot `page.png`. Peak 733,634,432 bytes, no rejected allocation.
- `builds/evidence/coachable-lazy/result.json`: after document completion,
  scrolling to the practitioner section caused its visible lazy photos to
  decode; screenshot `page.png`. Peak 966,999,232 bytes, no rejected allocation.
- Browser core tests passed, including the typed abort classification check.

The native guest uses the production software renderer and real networking.
It is a disposable engine fixture, not an installed-desktop acceptance test.
Its HVF-only entropy input consumes fresh host CSPRNG bytes once and fails closed
at exhaustion; this does not alter installed OS entropy.

## Remaining limits

Loading remains slow: approximately 299 seconds for initial completion in the
1920 x 1080 run and 540 seconds including lazy-image scrolling in the separate
1280 x 800 run. These are different workloads, not a before/after speed claim.
An experiment giving downloads more time between paints increased concurrent
memory pressure and was removed. The browser heap limit was not increased.
The lazy-image run approached the existing arena limit; this fix does not claim
unlimited page capacity or validation of every page interaction and viewport.

Release manifests and package parity/checksum results are recorded separately
by build-kit. Building an ISO does not update an existing installed system.
