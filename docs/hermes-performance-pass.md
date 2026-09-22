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

The baseline ISO is preserved as `build/hermes/baseline-three-models-aarch64.iso`.
No performance improvement has been established yet. The instrumented baseline
must be run before removing Qwen or optimizing the native compute path.
