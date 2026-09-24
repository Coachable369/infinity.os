# Hermes performance pass

## Scope and acceptance checklist

- Capture a fresh-conversation `hello` baseline on installed ARM64 InfinityOS,
  7 vCPUs, 20,480 MiB RAM, Hermes Q4_K_M, 4K context, deterministic sampling.
- Record TTFT, output count, decode interval, total elapsed time, inference-owned
  allocation high-water, worker utilization, and the five largest measured operations.
- Remove Qwen's registry, selector, boot payload, packaging and model-specific
  implementation; retain shared native tensor infrastructure and Ministral behavior.
- Make Hermes the default without bypassing capability/IOP checks.
- Optimize only the measured dominant bottleneck; compare identical requests
  and verify native output, cancellation and responsiveness after each change.
- Verify both models in a fresh, ISO-detached installed System Generation.

## Instrumentation (baseline preparation only)

`model bench hermes` clears idle native conversation caches, then submits `hello`
through the existing chat boundary. It refuses an occupied composer or running
generation. `ai bench timing` reports output tokens, decode, TTFT, total and BSP
compute microseconds. `ai compute` reports request-local summed AP Q4/Q6 compute,
completed-job collection wait, online workers and fixed inference allocation.
`ai profile` reports the five largest BSP phases, including polling overhead.
AP kernel times must be considered separately: BSP phase timing is not AP
matrix compute time, and summed AP time is not wall time.

Inference-owned allocation includes weights, the exclusively reserved arena and
service state. It is a fixed allocation high-water, not process RSS or system-wide
peak RAM. Worker utilization is summed Q4/Q6 compute divided by request wall time
and online worker count. This excludes firmware/OS CPU usage and initial loading.
Steady-state rate is `(output_tokens - 1) / decode_seconds`.

## Controlled ARM64 host comparison, 2026-09-23

The pinned Hermes artifact and deterministic `hello` request were run through
the same release `hermes-native-test --forward` harness before and after this
change. The baseline was rebuilt from the pre-change Git revision in an isolated
temporary tree; both runs used the same cached model file and produced the same
nine-token response, `Hello! How can I assist you today?`.

| Measurement | Baseline | Optimized | Reduction |
| --- | ---: | ---: | ---: |
| First visible token | 15.288925 s | 4.276984 s | 72.0% |
| Eighth visible token | 27.489967 s | 7.711431 s | 72.0% |

Hermes' Q4_K matrices now quantize each shared activation to Q8 once per row
batch and use ARMv8.2 integer dot products. The freestanding path checks the CPU
feature before executing those instructions and retains the existing floating
point fallback. At the large worker batch size, alternating seven-trial kernel
medians improved by 3.13x at width 4096, 3.18x at width 8192, and 3.17x at width
12288. Q4 normalized RMS error is bounded to 2%; Q6 remains bit exact. The real
Hermes harness also returned the correct concise result for an independent
arithmetic prompt.

`make ai-test` passes the concurrent six-worker, cancellation, numerical-bound,
service, privacy, memory, and chat behavior checks. Both the live and installed
ARM64 kernels link the same optimized object, and installed-kernel/loader binary
parity passes for ARM64 and x86_64. The measurements above are controlled host
evidence; installed-guest `model bench hermes` remains a separate proof level and
must not be inferred from host timing.

## Widget-visible response correction, 2026-09-23

The acceptance clock now starts when the OS AI Chat widget submits a turn and
stops only after the first assistant text frame has been presented. `ai timing`
and `ai bench timing` expose this independently as `Widget visible`; token
compute and whole-response timing remain available for diagnosis.

The previous chat boundary withheld every partial result until EOS, so the
historical installed sample in `native-hermes.md` did not display anything for
8.898 seconds even though its first token existed at 4.622 seconds. The native
chat path now publishes each cumulative partial immediately and updates one
assistant bubble until completion. Holding inference performance constant, that
sample moves first visible response from 8.898 to approximately 4.622 seconds,
a 48.1% reduction. This comparison isolates the removed UI/service buffering;
it is not presented as a new guest run.

A private clone of the preserved installed ARM64 System Generation was patched
with the candidate kernel and booted successfully to its existing login screen.
Fresh authenticated widget timing remains pending because the account requires
the user's password; no credential was guessed, reset, or bypassed. The original
VM disk was reattached unchanged after the boot check.

## Installed observation, 2026-09-21

The logged-in ARM64 desktop displayed a completed Hermes response to
`how are you?` at 1.3 tokens/sec. This is not the fixed `hello` benchmark,
so it cannot serve as the controlled before/after comparison.

A five-second macOS process sample of the running VirtualBox VM reported a
33.1 GiB physical footprint (33.2 GiB peak). The host has 32 GiB physical RAM;
host-wide swap usage was 6,277.75 MiB, and a process CPU snapshot was 701.1%.
All six AP threads appeared in Hypervisor execution while chat showed its
completed response. These are host/process observations, not inference-owned
RAM or guest kernel hotspots. They motivate checking memory pressure and idle
worker behavior, but do not establish either as the dominant inference cost.

Automated pointer/keyboard attempts failed to reliably open Command Window or
enter the benchmark command, including the VirtualBox soft keyboard. No
controlled TTFT, request-local worker profile, or improvement claim is available
from this session yet. Preserve this installed account and disk; routine
profiling updates must not recreate the VM or repeat credential setup.

## Rebuild and installed update, 2026-09-22

`sh build.sh` completed successfully with the desktop productivity shortcuts and
two-model packaging. `builds/InfinityOS-aarch64.iso` is the published 8 GiB ISO;
the historical provisioning filename under `build/hermes/` remains compatible.
Binary extraction verified the exact bootloader, pinned Hermes and Ministral
hashes/licenses, and absence of all ten Qwen payload shards.

The existing `infinityos-4` was updated without recreating its VM, TPM, or account.
The kernel updater validated GPT, boot manifests and component records, then
verified all bytes outside its kernel/record changes were unchanged. Its ESP
received the matching bootloader; only Qwen shards and their license were removed.
The patched disk passed both model parity executables. It booted to the existing
login screen with the optical drive empty, at 7 vCPUs and 20,480 MiB RAM.

Authenticated shortcut checks and guest Hermes timings remain pending: automated
typing, individual key events and VirtualBox soft-keyboard input did not populate
the guest password field. No password was changed and authentication was not
bypassed. Neither successful compilation nor removal of the Qwen reservation
constitutes measured Hermes latency improvement.

### Host-only fallback check (not guest performance acceptance)

The release `hermes-native-test --forward` executable passed loading,
cancellation and real forward generation for `hello`, producing nine tokens:
"Hello! How can I assist you today?". First emitted token was 13.397361 s;
the ninth token was emitted at 27.093604 s, giving 0.584 tokens/s over the
eight-token decode interval. Whole-process time including initialization and EOS
was 32.57 s; user/system CPU was 29.17/0.87 s; peak physical footprint was
2,972,125,584 bytes. This host harness does not exercise the guest AP workers,
desktop scheduling, or installed boot path. The running VM also competed for
host resources. These numbers are diagnostic only, not a before/after comparison
or evidence of improved guest performance.

`ministral-native-test --forward` also passed loading, cancellation and generation,
returning "Hello" for its existing one-word test prompt. Because that prompt is
different, its 21.077 s generation time is not a Hermes comparison.

## Follow-up: shared activation totals, 2026-09-23

The next pass leaves model weights, context, sampling, quantization and each
row's accumulation order unchanged. Q4_K now computes its eight Q8 activation
group sums once per 256-value block, including their exact conversion to float,
and reuses them for every output row. Maximum additional private stack scratch
is 1,536 bytes; no new resident model allocation or framework is introduced.
Q6 and CPUs without integer-dot support retain their existing paths.

Five alternating native-host trials per variant, using the same deterministic
`hello` prompt and fresh conversation state, produced identical nine-token
response bytes in every run:

| Median | Before | After | Time reduction |
| --- | ---: | ---: | ---: |
| First emitted token | 3.574 s | 3.400 s | 4.9% |
| Decode interval | 4.032 s | 3.636 s | 9.8% |
| Total, including EOS | 8.292 s | 7.523 s | 9.3% |

These are ARM64 **host** timings, not widget-visible or installed-VM timings.
An existing VirtualBox VM remained running during the alternating trials, so
the medians are evidence for this local comparison, not a universal latency
guarantee. A separate arithmetic request also produced identical generated bytes.
At Hermes widths 3072 and 8192, seven-trial kernel medians improved 1.24x and
1.27x for large batches, and about 1.17x for eight-row batches. An earlier
four-row interleaving experiment was slower and was discarded.

`make ai-test`, exact Q4 arithmetic/row-tail fixtures, ASan/UBSan, freestanding
ARM64/x86_64 compilation, and real Ministral generation passed. Binary response
receipt validation is covered by `test_hermes_response_perf.py`.
`tools/hermes-response-perf.py` repeats the real-model alternating comparison;
`HERMES_TEST_REPORT` on `hermes-native-test --forward` emits its binary evidence.
Local measurement artifacts are under `builds/hermes-refine-20260923/`.

The installer build links the same tested math object into live and installed
kernels; the installed ELF and its P1 payload were independently byte-compared.
The published `builds/InfinityOS-aarch64.iso` was checked directly through its
EFI image: its installed-kernel shard and bootloader match the verified build.
ISO SHA-256: `ac28b5624fa6b34300dde437963e590b1a1e25085a6bfe2515facdf0eb78bfde`.
Installed-kernel SHA-256: `d8532a3dc8624308223a4c015eaad026bafc5d90b921940c6b7181fe2c868d14`.
Authenticated, ISO-detached VM response timing remains pending. The user's
running VM and credentials were not modified or bypassed.

## Additional 25% target pass, 2026-09-23

Baseline: the already optimized math kernel in `b36ad4f` / `6ce06e7`, not the
older pre-optimization kernel. Profiling the actual Hermes forward path attributed
about 63% of matrix time to Q4 and 36% to Q6. The bounded changes are:

- Four independent activation-maximum accumulators remove a serial dependency.
- Four Q4 groups share packed loads, integer reductions and vector scale math;
  their final floating-point additions retain the original order.
- ARM half-storage conversion replaces software bit decoding. Scalar/x86 paths
  retain the software decoder.
- Q6 signed weight/scale products are formed exactly in int16 before conversion
  to float. Finite half scales make the resulting float weights exact; the
  activation multiplication and ordered accumulation are unchanged.

No weight requantization, shortened responses, context reduction, extra model
allocation, changed service scheduling, or larger work slices were introduced.
The ARM freestanding main kernel stack frame decreased from 14,656 to 14,512
bytes; its Q4 helper frame decreased from 32 to 16 bytes.

Five alternating fresh-process native ARM64 host trials, with identical `hello`
prompts and conversation state, returned identical nine-token response bytes:

| Median | Previous baseline | New | Time reduction | Throughput gain |
| --- | ---: | ---: | ---: | ---: |
| First emitted token | 2.775 s | 2.112 s | 23.9% | 31.4% |
| Eight-token decode interval | 3.045 s | 2.422 s | 20.5% | 25.7% |
| Complete response, including EOS | 6.201 s | 4.839 s | 22.0% | 28.1% |

This meets a 25% **throughput** target on the controlled host, not a 25% reduction
in complete-response latency. These are not installed guest or widget-visible
timings. Unlike the preceding pass, no VirtualBox VM was running during these
trials; only the alternating measurements against this pass's baseline are a
valid comparison. Timings exclude model loading and integrity verification.

The seven-trial kernel comparison measured Q4 throughput gains of 48–50% for
eight-row slices and 34–36% for 4,096-row worker batches. Q6 improved about 4–6%
in most cases (the 8,192-wide large batch gained 1.7%). Guest scheduling and
multicore memory contention can produce different end-to-end results.

Verification passed: `make ai-test` (including worker concurrency/cancellation),
exact Q4 reference comparisons with varied signed finite half scales, all 63,488
finite half encodings through Q6 against an independent scalar path, row tails,
ASan/UBSan, ARM64/x86_64 freestanding compilation, binary timing-receipt tests,
and actual Hermes/Ministral generation. An additional arithmetic request returned
identical output bytes in both variants. No console prose was used as an oracle.

Reproduction: `sh tools/qwen-kernel-perf.sh b36ad4f` and
`tools/hermes-response-perf.py` with the saved baseline/candidate executables.
Evidence is in `builds/hermes-plus25-20260923/`; the five-trial binary receipts and
numeric summary are in its `response-pass4/` directory. ISO publication and
authenticated, detached-media guest timing are separate verification gates.
