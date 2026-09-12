# Qwen Q4 CPU multiplication optimization

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
