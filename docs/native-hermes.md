# Hermes 3 Llama 3.2 3B — native optional model

Built with Llama.

This adds Nous Research's Hermes alongside Qwen and the existing Ministral model,
not in place of either. Artifact: `Hermes-3-Llama-3.2-3B.Q4_K_M.gguf`, official
GGUF revision `3cd927095d8cbab12c743f932aa63b6f7bbfa141`, 2,019,373,888 bytes,
SHA-256 `91776fe0f6cd7483d9d5e06162fdd1f8f0262c15ced269791b4d96a655e8a5a2`.
Upstream: https://huggingface.co/NousResearch/Hermes-3-Llama-3.2-3B-GGUF

## Scope

Model ID `0x41491005`, existing local CPU provider, System Optional install class.
The model is described as the lightweight intent/tool-oriented choice; Qwen
remains the heavier-reasoning option. Generated text does not grant authority
or execute tools. Existing capability, confirmation and typed-IOP checks remain
the execution boundary. No distributed inference or new framework is introduced.

Hermes uses a separate pinned loader and private KV/tokenizer/output state.
The shared engine retains Qwen's geometry and arithmetic on its existing path;
Hermes supplies 28 layers, 24 query heads, eight KV heads, width 3072, MLP width
8192, Llama-3.2 scaled RoPE, tied embeddings and its own ChatML/EOS handling.
Native Q4/Q6 math and bounded background workers are reused. Switching cancels
the old job and parks its service without moving weight or KV allocations.
Owner changes clear all three conversation states.

## Packaging

`sh tools/build-hermes.sh` opts into the Hermes installer. Standard Qwen media
is not replaced. The Hermes image is `build/hermes/InfinityOS-Hermes-Qwen-aarch64.iso`.
P4 shards carry Hermes in the installed EFI payload; its hash is checked before
native initialization. License and attribution files are packaged and verified.
The upstream card labels its license `llama3` while naming Llama 3.2 as its base;
the base-model agreement is included. Review upstream terms before redistribution.

All three resident models reserve 9,194,180,384 model bytes plus three 1,280 MiB
arenas, before the rest of the OS. Use a 20 GiB VM for this all-model image.
The installed VirtualBox ARM64 boot failed its contiguous model/arena allocation
at 16,288 MiB and reached onboarding at 20,480 MiB. The allocation sum alone
does not establish the minimum bootable VM size. Smaller configurations remain
unverified; the standard Qwen image is unchanged.
Hermes's model plus arena reservation is 3,361,551,168 bytes; this is a fixed
reservation, **not a measured peak-RAM result**. The initial context remains 4K.

## Console and selectors

Hermes is independently listed in the existing Settings/chat model catalog.
Console commands:

```text
model list
model inspect qwen
model inspect hermes
model select qwen
model select hermes
model test qwen
model test hermes
ai timing
```

Tests submit the identical `hello` prompt through the normal background chat
service. An occupied composer or active request is not overwritten. Timing
reports include first-token, decode interval and total response including EOS.
For steady-state throughput use `(output_tokens - 1) / decode_seconds`.
Run each model from a fresh conversation for a fair comparison.

## Verification status

Native host loading, cancellation and a real Hermes forward pass succeeded;
the observed output was “Hello! How can I assist you today?”. These are host
smoke tests, not installed-system acceptance or a controlled speed comparison.
The Qwen artifact/parser/tokenizer regression and AI behavioral suite passed.
`make ai-test`, the pinned Qwen parser/tokenizer check, Hermes pinned loader and
cancellation test, two geometry/RoPE unit tests, ARM64 kernel/loader builds and
the x86_64 kernel compile check passed. The optional ISO build completed, and
extracted EFI/model/license parity assertions passed for all three models.
## Installed guest verification — 2026-09-21

Fresh installation to the separate `infinityos-hermes-qa` 64 GiB VDI completed.
The DVD was detached before installed boot and first-run setup. Both native
models loaded; `model inspect hermes` reported the expected artifact length,
local CPU state and 4K context. `model test qwen`, then `model test hermes`,
completed on the same boot, exercising selection and real native inference.
Both chat results visibly ended in "can I assist you today?"; the command
window obscured the beginning of the bubbles, so no full-response transcript
is claimed here.

Environment: Apple M2 Max host, VirtualBox ARM64 native execution, 7 vCPUs,
20,480 MiB guest RAM, Q4_K_M artifacts, 4K context, first `hello` request for
each model. One sample each, not a statistical benchmark. Counters are from
the guest's `ai timing` command; throughput is the rounded desktop display.

| Metric | Qwen3-8B | Hermes 3 Llama 3.2 3B |
|---|---:|---:|
| First token | 11.420 s | 4.622 s |
| Decode interval | 9.479 s | 3.791 s |
| Displayed decode throughput | 1.0 tokens/s | 2.1 tokens/s |
| Total response including EOS | 21.918 s | 8.898 s |
| BSP prefill work | 2.528 s | 0.861 s |
| BSP decode work | 3.155 s | 1.168 s |
| Maximum AI pump | 3.782 ms | 3.335 ms |

Hermes total latency was 59.4% lower in this short-prompt sample. Do not
generalize it to long prompts, sustained throughput or tool-call accuracy.
The console clips the first line of the seven-line timing report, so exact
output token counts were not captured. Both model metadata and live timing
were inspected in the installed guest, not inferred from host smoke tests.

Outstanding acceptance: measured per-model peak RAM and a controlled pointer/
drag responsiveness test during inference. Fixed arena reservations are not
peak measurements. Native desktop keyboard commands worked; host-driven mouse
automation did not reliably move the guest pointer, so no pointer-latency claim
is made. The original `infinityos-4` disk was not attached to the QA VM or
modified; its session was saved while the isolated test ran.
