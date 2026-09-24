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
