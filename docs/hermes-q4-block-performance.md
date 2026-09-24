# Hermes Q4 block reduction — September 24, 2026

Promoted the measured ARM64 dot-product Q4 optimization in
`kernel/runtime/ai/qwen/cpu_math.c`. Q4 accounted for 77.3% of profiled matrix
kernel time (not of the whole request). Packed weights and Q8 activations now
remain integer through the eight-group weighted reduction; floating-point
scales are applied once per 256-weight block. The maximum absolute integer
sums are 30,723,840 and 2,048,256, both safely within int32.

No model, context, response limit, worker policy, or Q6 algorithm changes.
Floating-point association changes slightly; this is not bit-exact arithmetic
relative to the previous kernel. Scalar/non-dotprod fallbacks are unchanged.

## Controlled measurements

Native ARM64 host, same loaded Hermes Q4_K_M model, fresh conversation per
process, alternating baseline/candidate. Timing starts before submit and ends
at completed generation, includes tokenization, excludes model loading and
guest desktop scheduling/presentation. No token logging in the timed loop.

| Median, five paired `hello` trials | Before | After |
| --- | ---: | ---: |
| First token | 1.614058 s | 1.443063 s |
| Decode (8 intervals) | 1.638881 s | 1.494591 s |
| Decode tokens/sec | 4.881 | 5.353 |
| Complete response | 3.459298 s | 3.121934 s |

Complete-response waiting time fell 9.75%. All nine-token response bytes were
identical. Three paired arithmetic trials (`What is 7 plus 5? Answer only the
number.`) fell from 3.953847 to 3.531020 seconds (10.69%); output remained `12`.
One-token responses have no steady-state decode interval.

Evidence: `builds/hermes-analysis-20260924/complete/result.json` and
`arithmetic/result.json`. These are host results, **not installed-VM evidence**.
No new peak-RAM or CPU-utilization improvement is claimed.

## Verification and delivery

`make ai-test` and AddressSanitizer/UndefinedBehaviorSanitizer Q4 reference
checks passed. Independent scalar reference validates the new block arithmetic
exactly across zero widths, output tails, finite signed half scales, extreme
activations, and ties-to-even rounding. Existing approximation/fallback and
worker tests remain enabled. The native timing harness now rejects incomplete
responses instead of reporting a token-limited result as complete.
Real Hermes complete-response and Ministral forward runs passed against the
production change. The ARM64 installed kernel also built successfully.

Installer rebuild was stopped before payload staging: only 5.5 GiB free on the
host. Canonical `builds/InfinityOS-aarch64.iso` remains the previous artifact.
Allow roughly 25 GiB free for safe staging/publication. Installed-VM validation
and fresh-install binary/model parity remain pending that rebuild; do not claim
the installed system has this change yet.
