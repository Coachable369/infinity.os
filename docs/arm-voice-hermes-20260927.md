# ARM voice and Hermes pass — 2026-09-27

## Scope and status

Fix reply speech defaults, investigate vocal-input freezes, optimize Hermes
input-to-complete-response latency, and rebuild the regular installer ISOs.
The user's `infinityos-4` was not stopped or modified. A separate disposable
VirtualBox machine, `infinityos-voice-test-20260927`, was created and is powered
off. Native decoder verification used an independent QEMU/HVF machine.

**The requested 75% end-to-end improvement is not established.** Neither host
inference nor isolated decoder tests constitute installed-desktop acceptance.

## Changes

- `identity.rs`: per-user spoken-reply preference defaults on, independent of
  microphone permission. Bit 6 of the existing AI-preference byte stores the
  disabled state, preserving the model selector and other packed preferences.
  Existing records without that bit also enable output. Explicit changes persist.
- Settings exposes **Spoken Replies** with enable/mute actions. Completed typed
  native-model responses enter the existing capability-governed background speech
  path. Output-only replies never open capture. Lock/logout, cancellation, and a
  newer chat turn stop obsolete work. Microphone listening still requires action.
- ARM MP Services callbacks now enter reserved 1 MiB native stacks rather than
  inheriting an unspecified firmware stack. PSCI workers use the same reservation;
  x86 already reserves 1 MiB. Allocations happen before the boot memory-map capture.
  This corrects the stack contract; it is not yet proof of the reported freeze's
  exact cause.
- Microphone resampling uses Q30 integer MACs instead of repeated soft-float
  operations on the desktop thread. Filter preparation remains outside capture
  processing. Duration, phase, clipping, alias rejection and chunking tests pass.
- ARM SwiGLU activation uses a pointer-only hardware-FP adapter with bounded
  interrupt protection and the existing exponential polynomial/order. x86 retains
  its existing path. No quantization, context, model, response cap, or prompt change.
- Host benchmark workers park/wake instead of burning CPU while idle, matching
  the native ARM event-wait behavior more closely. This is a harness correction,
  not a guest-performance gain.

## Evidence so far

- `./build-kit run sh tools/ai-test.sh`: AI acceptance passed, including speech
  preference default, persistence and microphone independence.
- Behavior-harness `ai-test` unit suite: 111 passed with `RUST_MIN_STACK=33554432`.
  The first debug run exhausted the default host test-thread stack in an unrelated
  node-operator fixture; that is not evidence of a native speech fault.
- Native ARM QEMU/HVF recognizer: six cases passed through the production MP
  stack-switch assembly: recorded speech, repeated speech, cancellation, bounded
  output, silence and expired work. Measured stack high-water mark: 39,784 bytes.
  The private stack's bottom 4 KiB remained untouched, and callback return worked.
- `voice-pcm-test`: sample-rate conversion acceptance passed at 16/44.1/48 kHz.
- `native_swiglu_matches_reference`: 48,001 activation values passed against the
  previous scalar/libm path with bounded relative error. Added to `ai-test.sh`.
- Real Hermes Q4_K_M, fixed `hello`, seven worker threads, three alternating trials:

| Host metric (median) | Baseline | Native activation |
| --- | ---: | ---: |
| Input to complete output including stop token | 1.888448 s | 1.714498 s |
| First token | 0.835857 s | 0.932367 s |
| First-to-last token interval | 0.836651 s | 0.857432 s |

Complete-response median improved **9.21%** in this host sample; TTFT and decode
did not improve. All six runs produced the identical nine-token response bytes,
SHA-256 `cd153d3c18e782c4f4b3ceec574adccc8e68bc557110b0bc263b01e09bfcc8ef`.
Host contention and run variance limit inference from these results. ARM guest
soft-float costs are not represented by host timing.

Evidence: `builds/evidence/hermes-voice-20260927/activation-comparison/result.json`.

The initial seven-versus-three-worker experiment suggested 66% improvement, but
corrected parking benchmarks did not reproduce it. The production worker cap
was rejected, not shipped. Do not cite that exploratory number as an OS gain.

## Remaining acceptance

- Fresh installed ARM desktop: typed reply speaks by default; disabling speech
  survives reboot; microphone stays off until explicitly enabled.
- Real vocal-input capture through recognition and Hermes without a UI freeze.
- Installed input-to-final-text timings and responsiveness, with the same prompt
  and environment, to establish the actual improvement toward 75%.

## Rebuilt installer artifacts

`./build-kit incremental` completed successfully in 4002.646 seconds, with
manifest `builds/manifests/20260927T231049038598Z-87138.json`. This was an
incremental rebuild, not a clean release. It includes the existing unrelated
working-tree changes; the task commit only contains the voice/performance work.
Live/installed kernel and boot-loader comparisons passed on both architectures;
model-enabled media speech-resource parity, boot-payload extraction, and final
checksum generation passed. These artifact checks are not installed desktop
voice acceptance.

| Installer | Bytes | SHA-256 |
| --- | ---: | --- |
| `builds/InfinityOS-aarch64.iso` | 8590338048 | `ac6e23128305f7b30c6df987ef418e2aa703eb5ac8bb8258e4a444d329c20993` |
| `builds/InfinityOS-x86_64.iso` | 8590346240 | `e59151de0c7d81c1801ebf90158a6ae53a140d461b78ad1d23f01f8a4b01ee46` |

Completed September 27, 2026, 19:17 CDT. The requested 75% installed Hermes
latency reduction remains unmet; the reported microphone freeze still requires
an end-to-end installed-system reproduction/retest before calling it resolved.
