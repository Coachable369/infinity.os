# Hermes Q6 integer-dot pass

## Change and tradeoff

ARM64 CPUs advertising integer dot-product support now use fused Q6_K/Q8 activation dot products. Q6 weights remain unchanged. Activations are rounded per 256-value block to signed eight-bit values, as already done for Q4 computation. Integer accumulation is bounded; no full floating-point tensor is materialized. Q6 skips the Q4-only activation totals pass. Single-row and worker-batch paths use identical arithmetic.

This is an approximation, not bit-exact floating-point evaluation. It does not shorten responses, reduce context, remove tools, change model selection, or bypass the existing background service/cancellation boundary. CPUs without dot-product support retain the previous exact kernel. `QWEN_EXACT_Q6` retains explicit reference-path coverage.

## Controlled measurements

Baseline: working tree at `253b527`, including the preceding exact SIMD optimization. Same pinned Hermes Q4_K_M weights, context configuration, deterministic prompt, fresh process and conversation per trial. Timings exclude loading and integrity verification. These are **native ARM64 host measurements, not installed VM input-to-visible-response measurements**.

Five alternating paired runs of `hello` returned identical nine-token response bytes:

| Metric | Baseline median | Candidate median |
| --- | ---: | ---: |
| First emitted token | 2.881 s | 2.181 s |
| Eight-token decode interval | 3.388 s | 2.195 s |
| Decode tokens/sec | 2.36 | 3.65 |
| Complete response, including EOS | 6.699 s | 4.724 s |

Complete-response time decreased **29.5%** (1.418x speed); decode throughput increased **54.4%**. The requested 50% end-to-end latency reduction is **not achieved**.

Three additional paired trials per prompt retained identical output bytes:

- Arithmetic: 6.895 -> 5.237 seconds; response was 12.
- Capital question: 6.827 -> 5.625 seconds; response was Paris.

This small sample is not a broad model-quality evaluation. RAM and guest CPU utilization were not remeasured; no additional resident model allocation was introduced.

Receipts and baseline/candidate executables: `builds/hermes-next50/`. The original benchmark summarizer divided by zero for one-token replies; all six binary receipts per additional prompt were retained and independently inspected. The utility now reports no decode speed ratio when the interval is zero, with regression coverage.

## Checks

- `make ai-test`: service, capability, cancellation and concurrent worker coverage; exact fallback fixtures retained.
- New independent Q8 reference test: zero inputs, widths 256 through 12288 (including Hermes width 3072), row tails and output sentinels; corpus-normalized rounding error below 2%, arithmetic error against independently quantized input below 0.002%.
- ASan/UBSan on the new fused-kernel fixture.
- Timing-receipt tests, and real Ministral generation.

Fresh-install packaging uses the same installed kernel via the canonical ARM64 ISO build. Installed-guest timing and desktop responsiveness under inference remain separate acceptance gates; host numbers must not be presented as guest results.
