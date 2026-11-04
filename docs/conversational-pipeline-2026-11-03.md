# Conversational pipeline investigation — in progress

Acceptance requires recognized speech, visible response text, first audible
response, sustained playback, and return to listening. Text-only timings do not
establish this acceptance. No updated ISO has been verified for this change.

## Reproduced cost

The generic desktop inference route supplied the complete native action registry
even for informational questions. In the native host benchmark, “How do stars
produce energy?” required 1,048 prefill tokens and 126.25 seconds to its first
token. Deferring the registry reduced prefill to 72 tokens and first text to
6.61 seconds. This experiment exposed a separate semantic action-routing failure;
the routing change is not release-accepted.

Idle preparation of the authenticated context, interrupted safely on a new user
turn, reduced new prompt processing for “What is your name?” to 15 tokens. The
final preparation-boundary benchmark recorded 1.46 seconds to first text and
1.98 seconds to completion for its first prepared turn, and 1.50/2.40 seconds for
the repeated recognized-transcript turn. The changed-profile run took 6.09/8.06
seconds under contention. Preparation itself is recorded separately and must not
be counted as free when the user is waiting for readiness. The routing guidance
has subsequently changed and requires fresh measurement.

Evidence: `builds/validation/voice-regression-20261103/name-final.json`.
These timings exclude microphone capture, speech recognition, audio synthesis,
hardware playback, and installed presentation.

## Native speech work

The unchanged native synthesis probe uses ARM64 QEMU HVF and actual generated PCM.
The retained optimizations are scalar broadcast hoisting, disjoint output-channel
transpose convolution, disjoint normalization stripes, and up to five idle math
helpers. They preserve upstream per-output arithmetic and join helpers before
reusing buffers. No host audio or inference services are introduced.

| Fixture | Original synthesis | Latest synthesis |
| --- | ---: | ---: |
| Short, warm (case 1) | 1.689 s | 1.383 s |
| Short, repeated (case 5) | 1.634 s | 1.407 s |
| Greeting (case 6) | 4.303 s | 3.261 s |
| Opening phrase (case 7) | 6.313 s | 4.756 s |
| Repeated opening phrase (case 9) | 6.134 s | 5.177 s |

All ten fixture PCM hashes match the original. The opening phrase contains about
4.18 seconds of audio: its synthesis is still slower than real time. The timings
are single runs with a separate VirtualBox VM consuming nearly eight host CPU
cores, so they are not controlled throughput or installed-playback acceptance.
An activation-parallelism experiment was removed because improvement was not
consistent.

Evidence directories under `builds/validation/voice-regression-20261103/native/`:
`conversational-20261103`, `conversational-five-helpers-20261103`,
`conversational-broadcast-20261103`, `conversational-transpose-20261103`, and
`conversational-norm-20261103`. Each contains its structured timing and PCM hashes.

## Remaining gates

- The sixth control correction pass passed clock and machine-name dispatch, then
  hit its 180-second deadline on the text-editor paraphrase with 1,129 prefill
  tokens and no output. Evidence:
  `builds/validation/voice-regression-20261103/controls-six-workers.log`, manifest
  `builds/manifests/20261104T003111756234Z-20508.json`. The repository correction-loop
  limit was reached; no release was generated from this unfinished routing change.
- Semantic controls must dispatch the intended typed command, including negations
  and examples that must not execute. A fast explanatory answer is not success.
- Validate cancellation, owner changes, and interrupted idle preparation.
- Repeat timings under controlled contention; separately measure concurrent LLM
  generation and speech synthesis rather than adding isolated measurements.
- Measure microphone endpoint to recognition, first presented text, first hardware
  playback, inter-phrase gaps, and next wake-word readiness in the installed OS.
- Complete both architecture builds, packaging parity, checksums, and installed
  runtime verification before calling the cycle complete.

## Resumed correction cycle

The user resumed work after the earlier correction limit. The competing test VM
was stopped with authorization. Uncontended prepared-name inference measured
0.569 seconds to first text and 0.783 seconds to completion. These measurements
precede the expanded resident common-tool schema and are not current full-pipeline
acceptance. Evidence: `builds/validation/voice-regression-20261103/name-uncontended.json`.

Everyday registered tools now remain in the prepared desktop prefix. Other tools
use deferred discovery. Tool generation is constrained to registered protocol
syntax after the model starts a tool record; it does not constrain conversational
text or bypass argument validation and native execution authorization. Idle
preparation loads the installed mapping before its first request and is cancelled
safely on owner changes or foreground work.

Speech now receives a temporary worker reservation while synthesis is active.
Ordinary inference gets its workers back when speech finishes. The native
concurrent-load comparison uses identical synthesis binaries, eight CPUs, and a
continuous 3072-by-3072 Q4 matrix workload. All ten PCM hashes match.

| Fixture | Without reservation | With reservation | Improvement |
| --- | ---: | ---: | ---: |
| Short reply | 0.911 s | 0.719 s | 21.0% |
| Repeated short reply | 0.891 s | 0.724 s | 18.7% |
| Greeting | 2.408 s | 1.787 s | 25.8% |
| Opening phrase | 3.837 s | 2.776 s | 27.6% |
| Repeated opening phrase | 3.851 s | 2.790 s | 27.6% |

Both workloads completed thousands of concurrent matrices. Reserving workers
trades some matrix throughput for speech latency, intentionally. These are
single-run native firmware measurements, not microphone or speaker measurements.
Evidence: `builds/validation/voice-regression-20261103/native/conversational-matrix-{unreserved,priority}-20261103/evidence.json`.

Worker ownership/cancellation tests passed (3), protocol grammar tests passed (4),
and speech output state/queue tests passed (14). Semantic native controls and
updated prepared-context timings are still being verified. No new release ISO
or installed-runtime acceptance is claimed.

## Final focused evidence (before release build)

- All 16 native semantic-control cases passed, including settings, task manager,
  launcher, named-window actions, negations, and quoted examples. No case reloaded
  the full registry. Manifest: `20261104T012725829206Z-24275.json`.
- Updated prepared-name benchmark: first text 0.77–0.81 s; complete 1.05–1.10 s.
  `name-resident-final.json` retains the actual output and token times.
- Prepared explanation benchmark: first text 1.13–1.28 s; first naturally
  speakable phrase 1.87–2.10 s; entire response 3.50–5.31 s.
  `explanation-resident.json` uses the production phrase-boundary algorithm.
- Initial idle preparation costs about 13 seconds. A request arriving before
  preparation finishes must still pay the remaining work. These are prepared
  response measurements, not cold-start claims.
- Focused AI behavior, owner isolation, interrupted preparation, inference math,
  and durable settings/remount checks passed. Manifest:
  `20261104T013520965440Z-24541.json`.
- Native shared-engine lifetime, cancellation, and buffer/heap checks passed.
  Recognition of the prerecorded 10-second fixture took 0.709–0.711 s.
  Manifest: `20261104T013611110895Z-24643.json`.
- Native JSON evidence is retained under
  `builds/validation/voice-regression-20261103/native/` so clean builds do not
  remove the measurements.

These isolated and concurrent component measurements must not be added together
and presented as measured end-to-end latency. Installed microphone endpoint,
rendering, first hardware playback, phrase gaps, and next wake-word acceptance
remain a separate runtime gate.

## Clean release delivery

The full clean ARM64 and x86_64 release passed on 2026-11-04 at 03:03 UTC.
Manifest: `builds/manifests/20261104T014324307718Z-29141.json`.
It records `cleaned: true`, `result: passed`, and 4812.085 seconds elapsed.
Both final ISOs passed complete model-bundle, SDK, and live/installed UI payload
verification. The build used the working tree based on `3dfe6bd8`; the performance
changes were not yet committed when the build started.

| Image | SHA-256 |
| --- | --- |
| `builds/InfinityOS-aarch64.iso` | `38667bebff076297b00a5d0339fe0a2badfbdfa91bd959ff77d62c4d2d582660` |
| `builds/InfinityOS-x86_64.iso` | `aa42f5b67bb92f7e237a95473b5ce17e24f76712a02167e0b310794f31c3997e` |

These are updated local installer images. The user's VM remains stopped and its
installed system has not been updated. End-to-end microphone-to-speaker acceptance
is still unverified; the component measurements above do not establish it.
