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

### September 12 Q6 optimization and guest baseline

The running `infinityos-4` desktop selected Ministral 3 3B with Ctrl+J,
Ctrl+M and produced a coherent greeting, reporting 1.2 tokens/sec when finished.
This was the older installed kernel (streamed response and old widget row still
visible), with the ISO still attached. It is a functional guest baseline, not
ISO-detached acceptance of the current build or a responsiveness pass.

The ARM Q6 decoder now loads eight packed values at once, widens them with NEON,
and performs the same two ordered four-lane accumulations. Q4, sampling and
worker scheduling are unchanged. `make ai-test` verifies bit-identical results
against scalar math, including Ministral widths 3072 and 9216 and row tails.
The real-weight `ministral-native-test --forward` completed after this change.
Sequential warm-cache host microbenchmarks measured eight-row Q6 work at width
3072 as 0.1988s before / 0.1361s after, and width 9216 as 0.5539s / 0.4119s
(20,000 repetitions). These are isolated host arithmetic measurements, not
end-to-end guest speedups. Updating the installed VM and measuring it again
remain required before proceeding to HTTPS work.

### Updated installed-system check

The installed kernel was updated without reinstalling on September 12. The
offline updater validated GPT, container, generation and component checksums,
updated eleven kernel references, and verified all unrelated logical disk bytes
unchanged. The final VDI compared identical to the validated patched RAW image.
An APFS clone of the original disk and copies of its firmware/settings remain
at `/Users/jonathan.mcallister/VirtualBox VMs/ministral-update.MVzD0m/`.

With the optical drive empty, the VM reached login, retained the existing user
and Ministral selection, and answered `Hello` coherently. No partial assistant
bubble appeared while generating. The completed response was visible by the
31-second observation, with a displayed decode rate of 1.5 tokens/sec versus
the earlier 1.2 baseline (rounded UI measurements, not an isolated Q6 speedup).
Cancellation was also observed. Command/launcher focus and repaint behavior
was inconsistent during diagnostics, so overall desktop responsiveness is not
accepted yet. HTTPS work remains deferred until that requirement passes.

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
