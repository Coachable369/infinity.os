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
arenas, before the rest of the OS. Use a 16 GiB VM for this all-model image.
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
Installed-OS testing is a separate, still-pending acceptance level.

Outstanding acceptance: ISO-detached Hermes/Qwen inference, no-reboot switching,
desktop responsiveness, and comparative TTFT, steady-state tokens/sec, total
response time and measured peak RAM. Do not report host timings as guest results.
The running user VM has not been shut down or modified for this work.
