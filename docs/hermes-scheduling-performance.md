# Hermes scheduling performance — September 24, 2026

## Change and scope

`AiRuntime::poll_qwen_inner` previously stopped after 256 service polls, even
when those polls merely checked an unfinished worker mailbox and used little
of the existing 2 ms budget. Control then returned through the event loop;
synchronous USB mouse and keyboard transfers each specify a 1 ms timeout.
Actual firmware latency varies and has not been measured in this pass.

The new allocation-free `qwen::pump::PumpBudget` uses the **same 2 ms deadline**
instead of treating 256 fast polls as a completed time slice. A 16,384-poll
guard bounds a stalled clock, a missing clock retains the original 256 limit,
and a disappearing/regressing clock yields immediately. Input remains ahead
of inference on every event-loop pass. Each individual engine slice remains
bounded, and completion/error/model selection/cancellation handling is unchanged.

This is a scheduling optimization, not faster matrix arithmetic. Three kernel
variants (paired rows, block unrolling, lane-wise float accumulation) did not
produce reliable gains and were discarded. The shipped Q4/Q6 kernels, model
artifacts, sampling, 4K context, response limits, permissions, and allocation
sizes are unchanged. Both Hermes and Ministral use the corrected pump.

## Measurement boundaries

Real native Hermes Q4_K_M inference, ARM64 host, deterministic `hello`, fresh
conversation per process, three alternating paired trials. Both schedules
have the same 2 ms deadline; baseline additionally has the old 256-poll cap.
The harness explicitly injects a **1,000 us event-loop sleep between pumps**
to study the effect of external I/O. This is **not installed-VM timing**, nor
does it measure the actual firmware input delay. Model loading is excluded;
tokenization through EOS completion is included. All output bytes match.

| Median with injected event delay | Old pump | New pump |
| --- | ---: | ---: |
| First token | 7.411253 s | 2.404816 s |
| Decode interval (8 tokens) | 7.099019 s | 2.477166 s |
| Complete response | 15.775465 s | 5.190439 s |

That is **67.1% less complete-response waiting in this controlled condition**,
not an assertion that the user's VM will improve by 67.1% or meet 40%.
The zero-injected-delay control is effectively unchanged (about 0.8% slower),
confirming the benefit depends on removing event-loop waits, not compute speed.
No new RAM reduction, CPU utilization improvement, or guest latency is claimed.

## Reproduce

Build `hermes-native-test` in release mode, then run:

```sh
python3 tools/hermes-response-perf.py \
  --baseline tools/behavior-harness/target/release/hermes-native-test \
  --candidate tools/behavior-harness/target/release/hermes-native-test \
  --baseline-pump legacy --candidate-pump deadline --io-delay-us 1000 \
  --trials 3 --output builds/hermes-pump-comparison
```

Use `--io-delay-us 0` and a different output directory for the control. Without
pump options, the existing unpaced native benchmark remains unchanged.
Binary receipts validate token counts, timing boundaries and exact output bytes;
the report records the injected delay explicitly.

Evidence for this run is under `builds/hermes-40pass-20260924/final-scheduled/`
and `final-no-delay/`. Deadline/clock-failure behavioral tests are part of
`make ai-test`. Real Ministral forward inference also passed.

## Delivery status

`sh build.sh` completed successfully, including the AI/worker/pump tests,
Hermes/Ministral fresh-install payload parity, boot-file extraction, and SHA256
manifest generation. All four ISOs under `builds/` were rebuilt. The full log
is `builds/hermes-40pass-20260924/full-build.log`.

The existing VM was saved/stopped at the user's request; no installed-guest
after measurement is available. Resume of that saved VM continues its old
in-memory kernel, not the newly built installer. Use a fresh install for guest
comparison. Only reproducible build intermediates were reclaimed for disk space;
VM disks, saved state, model-cache files, and backups were preserved.

## Generated-code performance follow-up — October 19, 2026 UTC

The installed ARM kernel uses Rust's soft-float target. Attention's score and
value kernels already used native floating point, but its exponential/serial-sum
loop still ran through software floating point for every causal head. The new
pointer-only native adapter uses the existing `native_expf`, preserving the
polynomial, evaluation order, and serial accumulation. x86_64 keeps its existing
implementation. Model files, sampling, context, response limits, authorization,
and the cooperative inference budget are unchanged.

The application-generation poller also scans for a completed source frame only
when the inference service publishes changed output, instead of rescanning up to
16 KiB while waiting for a worker or processing prompt tokens. Terminal output
validation still runs on completion or failure. Editor glyphs now accumulate one
damage rectangle per glyph; see `editor-pointer-performance.md` for separately
measured repaint results and pixel-equivalence coverage.

### Numerical and service-contract verification

- Native attention numerators and serial sums match the reference bit-for-bit
  for finite results; NaN classification, underflow, zero-length work, causal
  lengths through 4096, and untouched tail storage are checked behaviorally.
- Full causal attention output parity and three paired real-Hermes host trials
  preserve exact generated bytes. Host median completion is effectively
  unchanged (1122.30 ms baseline; 1138.33 ms candidate); this is not a host
  throughput improvement.
- A real service run checks every unchanged poll against the previous output,
  append-only published output, and retained final bytes after completion.
  That run observed 62,456,595 polls and nine published output events.
- Both installed kernels and the focused editor/AI regressions pass under
  manifest `20261019T032851598903Z-62824.json`.

Numerical and host receipts are in
`builds/validation/performance/hermes-softmax/`. The full `ai-test` build-kit
target includes the native softmax, installed-metric reducer, and read-only
profile-projection tests. The installed benchmark uses actual desktop input,
sampled native progress counters, and captured framebuffer pixels; it does not
infer success from assistant prose.

### Installed ARM before/after measurement

One paired run used a fresh clone of the same stopped installation for each
kernel, with the ISO detached: QEMU/HVF, four vCPUs, 12 GiB RAM, 1024×768, and no
emulated audio device. The native persisted profile selected Hermes, and the
installed ESP model shards passed their exact length/SHA256 checks. Both runs
submitted the same graphical-calendar objective (871 prompt tokens; request
SHA256 `b56141f84eaad7cdedaba5db9c667f31a0ea484b2f0c5da3b00b0b9acdb7d3e2`).

| Installed measured boundary | Before | After |
| --- | ---: | ---: |
| Submit to first published token | 257.434–257.644 s | 171.176–171.386 s |
| Submit to first document delivery | 257.641–257.853 s | 171.176–171.386 s |
| Published tokens 16→128 (112 tokens) | 55.693–56.117 s | 31.965–32.385 s |
| Decode throughput for that interval | 1.996–2.011 tokens/s | 3.458–3.504 tokens/s |
| Maximum sampled source-to-editor backlog | 8 bytes | 5 bytes |

This is approximately **74% higher decode throughput** and **34% less waiting
for first code** in this paired installed-ARM workload. It is not a claim about
every prompt, x86 performance, voice-to-audio latency, or completed-application
quality. The initial wait remains substantial. The intervals bound sampled
counters (roughly every 200 ms), not exact internal inference/present events;
they are not statistical confidence intervals.

Both runs sustained at least 128 decode tokens and 60 seconds, captured actual
editor pixels during generation, and verified cancellation by a user edit.
The compiler sequence remained zero in both runs: compilation did not contaminate
the timing. Source arrived incrementally, not only on completion. The candidate
delivered 599 bytes in 191 flushes over its sustained window. Different total
window token counts are not used as the before/after throughput boundary.

Evidence, screenshots and raw samples are preserved under
`builds/validation/performance/20261019-before/` and `20261019-after/`.
The installed manifests are `20261019T032024934459Z-61947.json` (before) and
`20261019T033246009219Z-63875.json` (after). These are private installed-clone
measurements with a QEMU-linked version of each kernel, not a modification of the
user's VM or proof of a fresh installation from the canonical VirtualBox ISO.

Reproduction uses `./build-kit run python3 tools/native-editor-installed-test.py`
with `--arch aarch64 --reuse-installed --update-kernel <qemu-linked-kernel>` and
`--benchmark-generation --benchmark-model hermes --generation-timeout 600`,
plus `--generate-request` containing the exact saved request. Each baseline and
candidate must begin from a separate clone of the same stopped fixture. Keep
the model, prompt, hardware and harness identical; build-kit serialization
prevents competing project builds during measurement.
