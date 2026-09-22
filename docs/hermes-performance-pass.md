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

No controlled performance improvement has been established yet. The former
baseline ISO under `build/` was removed by the full clean build on September 22;
do not rely on that path as a retained baseline artifact. Historical observations
below are not a replacement for a matched before/after benchmark.

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
