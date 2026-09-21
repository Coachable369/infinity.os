# Qwen CPU multiplication optimization

## Q6_K plan and implementation (2026-09-20)

Task-specific acceptance plan:

1. Compare the existing and candidate kernels on identical quantized weights and
   activations, compiled with identical optimization flags.
2. Extend the ARM64 four-row kernel to Q6_K. Share activation loads and decode
   sixteen packed values together, retaining exact accumulation order. Keep
   model format, worker limits, cancellation and service routing unchanged.
3. Require exact outputs, at least 1.15x median Q6 throughput at representative
   widths, and no more than 5% Q4 regression. Measure seven alternating-order
   trials for small batches and 4,096-row worker jobs.
4. Run AI behavioral tests and rebuild both live and installed payloads with
   model packaging parity checks. Separately measure installed inference before
   making an end-to-end throughput claim.

Implemented in `kernel/runtime/ai/qwen/cpu_math.c`. Q4_K_M models contain Q6_K
tensors, so this removes the previous single-row fallback for those worker
jobs. It adds no heap allocation, persistent buffers or model conversion.
Scalar/non-ARM fallbacks and the single-row path are unchanged. The optimization
benefits ARM64 batched workers, not the host Rust scalar transformer harness.

### Controlled host measurements

Apple M2 Max, Apple clang 21.0.0, `-O3 -ffp-contract=off`, baseline commit
`8ead345164081a67b7ea84319ac7e698b9a46a3b`. Times below are median CPU seconds
for 40 calls of 4,096 rows; each result matched the baseline bit-for-bit.

| Q6 width | Baseline seconds | Optimized seconds | Throughput speedup |
| --- | ---: | ---: | ---: |
| 4096 | 0.183155 | 0.128176 | 1.429x |
| 8192 | 0.368614 | 0.269227 | 1.369x |
| 12288 | 0.554650 | 0.399921 | 1.387x |

Eight-row batches improved 1.441–1.453x. Q4 measurements were 1.5–2.2% slower,
within the regression budget; its arithmetic was not changed. The largest
matrix is approximately 39.4 MiB. These measurements use deterministic synthetic
quantized data and CPU time, not whole-model inference or guest wall-clock time.
The first eight-value candidate improved only 1.12–1.16x; widening decode to
sixteen values produced the accepted measurements above.

Reproduce on an otherwise idle ARM64 host:

```sh
sh tools/qwen-kernel-perf.sh > qwen-kernel-results.jsonl
make ai-test
sh tools/build-qwen.sh
```

The benchmark compares actual output bytes before and after each trial and
emits structured measurements. Timing thresholds are reviewed separately, not
flaky CI assertions. Existing AI tests also cover scalar/SIMD exactness, row
tails and worker cancellation. Both installed and live ARM64 kernels already
depend on the same math object, requiring no new packaged component.

Final verification: `make ai-test` passed; the exact-width/tail suite also
passed AddressSanitizer/UndefinedBehaviorSanitizer. ARM64 live and installed
kernels built, and the x86 freestanding C fallback compiled. The Qwen ISO
rebuild completed with extracted EFI/model/license parity assertions passing
for Qwen and Ministral. Artifact:
`build/qwen/InfinityOS-Qwen3-8B-aarch64.iso`. No user VM was modified.

### Remaining end-to-end acceptance

Do not interpret 1.37–1.45x kernel throughput as the same tokens/sec gain:
Q4 work, attention, bandwidth, scheduling and other inference costs remain.
For a controlled installed test, use an ISO-detached 12 GiB VM, fixed CPU count,
the same Qwen3-8B Q4_K_M model, 4K context and exact prompt. Reset conversation
and KV state between trials, separate cold loading from warm decode, alternate
baseline/candidate kernels, and collect at least five trials of `ai status`
token counts/decode duration plus first-token and maximum work-slice metrics.
Report median decode tokens/sec and input responsiveness together. Installed
VM throughput and interaction validation remain outstanding for this revision.

## Previous Q4 optimization (2026-09-12)

The ARM64 four-row Q4_K kernel now loads eight quantized bytes per row at
once, masks/shifts nibbles in byte vectors, and widens them for two consecutive
four-lane float operations. Activations remain shared across four rows. The
accumulation order and disabled floating-point contraction are unchanged.
Single-row, Q6_K, scalar fallback, model loading and service scheduling are unchanged.
No activation requantization or additional buffers are introduced.

## Verification (2026-09-12)

`make ai-test` passes including exact scalar/SIMD comparisons for Q4_K and
Q6_K, widths 256–12288, row counts 0–9 and output-tail preservation. The ARM64
freestanding math object builds. Both live and installed kernels link this same
object through their existing Makefile dependencies.

Native Apple Silicon host benchmark, 40,000 calls of eight rows, compared with
the prior committed kernel compiled using identical `-O3 -ffp-contract=off`:

| Width | Previous batch CPU seconds | New batch CPU seconds | Speedup |
| --- | ---: | ---: | ---: |
| 4096 | 0.3397 | 0.2869 | 1.18x |
| 8192 | 0.6276 | 0.5319 | 1.18x |
| 12288 | 1.0034 | 0.8939 | 1.12x |

These are warm-cache microbenchmarks, not installed-VM inference throughput.
End-to-end tokens/sec and memory-bandwidth effects still require guest testing.
The current installed VM and published ISO have not been updated by these tests.
