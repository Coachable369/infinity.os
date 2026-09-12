# Native Ministral 3 3B comparison addition

Ministral 3 3B Instruct 2512 is an additional local model, not a replacement for
Qwen3-8B. Settings and desktop chat share the five-entry model catalog and the
same persisted selection. A catalog entry is only ready after native loading;
native selections never fall back to canned conversational responses.

## Pinned artifact

- Publisher: `mistralai/Ministral-3-3B-Instruct-2512-GGUF`
- Revision: `eb599d408350ea2bb60452cb86be7c7b2fc28227`
- File: `Ministral-3-3B-Instruct-2512-Q4_K_M.gguf`
- Bytes: 2,147,023,008
- SHA-256: `9ed150d4367e68df0ac8e1540f6ddc65b42d0ee26378329d1ecbca60f93fc5f8`
- Apache 2.0 license is included in the installed payload.

## Native execution

The existing allocation-free GGUF/K-quant engine now supports two validated
geometries. Ministral uses 26 blocks, a 3072-wide residual stream, 9216-wide MLP,
32 query heads, eight KV heads, and 128-dimensional heads. Input and output
embeddings are tied. The GGUF's adjacent-pair Q/K layout, YaRN frequencies and
1e-5 RMS epsilon are handled separately from Qwen's learned Q/K normalization
and split-half rotary layout. The initial 4096-token limit remains below the
16384-position attention-temperature threshold.

Tekken byte BPE uses the artifact's merge table and case-sensitive ASCII
pretokenization. This initial implementation rejects non-ASCII prompt text
explicitly; full Unicode Tekken category segmentation is not yet implemented.
Model-produced UTF-8 output uses the existing byte decoder. Prompts use native
`<s>`, `[INST]`, `[/INST]`, and `</s>` controls, not Qwen's chat format.

Only one engine dispatches work at a time. Switching cancels the active job
before changing engines; each model retains separate mutable KV and token state.
Both models use the existing bounded service pump and CPU worker pool.

### Complete-response presentation

Native inference retains decoded intermediate tokens inside the AI service.
The desktop adds the assistant message only when inference completes; partial
tokens neither create an empty bubble nor invalidate the chat transcript.
Cancellation and failure keep their explicit status without publishing partial
answers. This applies to both Ministral and Qwen and does not change sampling,
model selection, or inference throughput. The host AI acceptance harness checks
that incomplete publication leaves the transcript unchanged and completion
publishes the full bounded response.

## Boot and installation

P2 remains the unchanged Qwen shard stream. P3 adds four FAT-safe Ministral
shards. The loader reads the optional P3 stream into the tail of the reserved
model allocation; the kernel splits that allocation before passing exclusive
arenas to the AI service. Old Qwen-only installed media remain loadable.

The combined model allocation is 7,174,806,496 bytes. Two 1280 MiB work arenas
reserve another 2,684,354,560 bytes, for 9,859,161,056 bytes total before other OS
allocations. Ministral currently reserves the same arena capacity as Qwen;
right-sizing its smaller cache is future memory optimization, not required for
this 12 GiB target.

`sh tools/build-qwen.sh` now includes both models and builds a 9 GiB installed
ESP inside a 10 GiB ISO EFI image. Existing 6 GiB ESPs cannot hold both models;
do not overwrite or repartition an existing user disk without a migration plan.
Fresh installation derives the partition size from the complete ESP payload.
Installer/recovery media now boot their own `EFI/INFINITY/KERNEL.ELF` before
probing installed generations. This prevents a new media loader from handing
its boot ABI to an older installed kernel during reinstallation. Disk-only boot
continues using the validated installed-generation path.
`qwen-install-parity -- <ESP> --ministral` extracts both models and validates
their exact shard sizes and SHA-256, plus loader and license bytes.

## Verification status

The native host harness completed a real instruction with a one-token `Hello`
answer and EOS in approximately 16.8 seconds, including prompt processing.
This is neither guest throughput nor installed-system acceptance. Qwen's
artifact/tokenizer regression and the ARM kernel check passed. Guest boot,
installation, model switching, and tokens/sec comparison still require VM QA.

No Linux, Python, PyTorch, llama.cpp, GPU, or vision runtime is included in
InfinityOS. Host harnesses are development verification only.

## Distribution media

`re-provision.sh` now defaults directly to the combined model-enabled ISO from
`tools/build-qwen.sh`, not a potentially older `builds/` export. A missing image
stops provisioning before VM deletion. An explicit second argument still allows
selecting a different installer intentionally. Existing installations created
from older media are not upgraded by this selection fix.

### VirtualBox reinstall check (2026-09-12)

The combined ISO completed a fresh installation on the backed-up 16 GiB
`infinityos-4` disk. With the ISO detached, the installed disk passed the previous
boot-contract failure and reached the boot artwork's ready stage. It had not yet
reached onboarding after several minutes; installed chat output and throughput
remain unverified. This is not full installed-system acceptance.

The current 10 GiB ISO exceeds both single-layer and dual-layer DVD capacity.
Use sufficiently large USB media for this combined model image. A future smaller
boot image with a separately discoverable model payload/companion medium should
retain the same verified installation and offline-installed-boot guarantees.
The present installer does not yet support installing these models from a
second DVD or downloading them after boot.
