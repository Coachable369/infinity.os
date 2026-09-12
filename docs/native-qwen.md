# Native Qwen3-8B bring-up

## Performance follow-up

New turns preserve fully computed KV positions. Only uncached tokens are
prefilled; cancellation discards the in-progress position, not the valid prefix.
Changing authenticated owner still clears conversation/cache state. Real-weight
parity checks compare resumed execution after partial-token cancellation with a
cold replay, asserting the same selected token.

`ai status` reports model-load time, elapsed first-token latency (including
tokenization), prefill/decode compute time, maximum individual engine slice,
maximum complete service-pump time, and reused/new prompt-token counts.
Existing decode tokens/time remain available.
Timing is gathered without per-slice logging; zero timings mean the monotonic
clock was unavailable. Maximum slice is not an end-to-end desktop latency metric.

Inference remains single-core: ARM secondary-core startup and a worker scheduler
are not implemented. Batched prefill is also not implemented. Guest comparative
benchmarks and responsiveness acceptance remain pending; host parity is not
evidence of guest speedup.

The desktop chat header shows a rotating eight-spoke spinner and a grayscale
shimmer on `Thinking...` during active generation. Its 30 Hz monotonic-clock
animation only damages the chat header; it reuses retained application surfaces
and the existing font atlas. Hidden/minimized chat and terminal generation states
stop animation. The installed kernel includes this code without extra assets.
Lifecycle tests and actual shaded-font pixel/clip tests cover this path; a guest
visual check is still required before claiming installed-animation acceptance.

The ARM64 streamed ISO is built by `sh tools/build-qwen.sh`, and is selected
by `build.sh` for the published ARM64 image. This path needs a 12 GiB VM,
at least a 16 GiB disk, EFI, xHCI USB input, and a firmware entropy source
(TPM 2.0 on the tested VirtualBox ARM configuration).

## Payload and inference

- Official Qwen/Qwen3-8B-GGUF revision
  `7c41481f57cb95916b40956ab2f0b139b296d974`, Q4_K_M: 5,027,783,488 bytes.
- SHA-256: `d98cdcbd03e17ce47681435b5150e34c1417f50b5c0019dd560e4882c5745785`.
- 512 MiB file shards avoid FAT's per-file limit. The installer transfers
  bounded 64 KiB ranges, verifies the complete SHA and disk readback before
  activation, and installs the model into the 6 GiB EFI System Partition.
- Installed boot loads the quantized weights plus a dedicated 1,280 MiB
  work arena. These reservations total 6,369,960,768 bytes, excluding OS
  memory. The FP32 4K KV cache itself occupies 1,207,959,552 bytes.
- Runtime code lives in `kernel/runtime/ai/qwen`, behind `AiRuntime`.
  It uses native Rust and a freestanding ARM64 CPU matrix kernel, not a
  host inference process. Jobs advance cooperatively outside paint paths.

## Verification

### Boot/install architecture change

Boot ABI version 7 adds the payload-read capability and the reserved model
and work-arena address/length pairs. The model is no longer linked into the
kernel executable. `boot/common/payload_loader.h` reads bounded ranges from
three logical shard streams: P0 (installed ESP), P1 (installed kernel), and
P2 (model within the installed ESP). Each stream supports up to 1,000 shards.
This removes the FAT single-file limit from payload transport; it does not
remove disk capacity or RAM limits, or the native kernel partition limit.

The ARM64 installer enables `streamed-payload` and consumes the generated
length/CRC/SHA manifest through `kernel/storage/payload.rs`. The destination
still uses the existing partition plan and complete-generation activation
protocol. Partial copies, source hash mismatches, and readback differences
fail before activation. The Qwen shard license is installed with the model.

On disk-only boot, firmware loads P2 into dedicated pages and reserves the
work arena before passing control to the kernel. `core/main.rs` transfers
exclusive ownership to the AI runtime, which verifies the pinned model hash,
initializes tokenizer indices and the persistent KV cache, and only then
marks Qwen selectable/ready. Chat submission and incremental output go
through `AiRuntime`; the input loop polls cancellable work outside rendering.
Selecting a different model or disabling chat cancels the active job.

This capability currently relies on retained ARM64 firmware file/block
services. The x86_64 path remains the embedded-payload path; it does not yet
gain native Qwen loading from this change. Future arbitrary model payloads
need a versioned model manifest rather than the current pinned model size
and hash. Avoid storing QA VM configurations or disks in `build/`, because
the main build script cleans that directory.

### Observed VM results

The separate 12 GiB VirtualBox ARM QA VM completed installation onto a fresh
16 GiB disk, booted with its ISO detached, completed first-boot account setup,
and reached the desktop with no network adapter. Real guest model output,
guest throughput, and responsiveness during inference are not yet verified.

Use the `qwen-contract-test`, `qwen-native-test`, and `qwen-install-parity`
bins in `tools/behavior-harness/Cargo.toml`. The native test's `--forward`
option exercises actual weights on the host; it is **not installed-OS proof**.
The parity test accepts an ESP image or an mtools disk-offset specification.
For installed verification, detach the ISO before booting the disk, select
Qwen3-8B in chat, and submit a prompt with networking disabled. `ai status`
reports output-token count and decode milliseconds for guest measurement.

## Current limits

ARM64 firmware-backed boot only; CPU only; 4K context; greedy decoding;
UTF-8 tokenizer input; conversation tokens retained within 4K; a bounded
16 KiB response buffer and 4 KiB composer. The physical keyboard input path
still needs Unicode text-entry support beyond its existing byte-key events.
The model is loaded at boot, and initialization hashes all weights.
Cancellation is cooperative. No GPU, vision, 27B model, or DeltaNet support.
Guest throughput and full installed-desktop acceptance remain unverified
until the VM test cycle completes.

## Direct keyboard access

The updated source handles Ctrl+J before desktop application shortcuts. It
opens/restores chat and focuses the composer from the desktop, Settings,
system menu, or launcher. Ctrl+M cycles the model while retaining composer
focus. Tab / Shift+Tab traverse model, composer, Send, minimize, and close;
Enter activates the focused control. Ctrl+C cancels generation. Escape first
cancels active generation, or releases chat focus when idle. Generation,
cancellation, failure, and context-full states are displayed in the header.
Conversation tokens and displayed messages are discarded when the authenticated
user changes; switching focus within the same user's desktop retains context.

The initial keyboard and context changes were installed in the QA VM, but
manual Ctrl+J testing failed. The ARM loader now negotiates Simple Text Input
Ex on the console handle (boot flag 64), preserving valid Ctrl/Shift state;
the kernel retains legacy text input as a fallback. Both ISO and installed
boot use this shared loader. Packet-level modifier regression tests cover
Ctrl+J, Ctrl+M, cancellation, reverse Tab, and selection movement. This latest
firmware change was verified on a backed-up QA disk: both EFI loader paths and
the installed kernel were updated, all ten model shards passed the pinned SHA
check, the 12 GiB VM booted with the ISO detached, password login succeeded,
and Ctrl+J followed by typing entered text in the chat composer. Enter submitted
the prompt to the selected Qwen3-8B model and displayed GENERATING. This proves
keyboard focus and submission, not completed model output or throughput.
Ctrl+C visibly changed GENERATING to CANCELLED, and Ctrl+M cycled models and
returned to Qwen3-8B. The `hello` prompt produced no visible output during the
approximately two-minute interval before cancellation; guest inference
performance and completed native output remain unresolved acceptance items.

## CPU inference optimization

Intermediate prompt tokens now complete their KV updates without evaluating
the 151,936-row vocabulary projection. Only the final prompt token and decoded
tokens request logits. Rotary sine/cosine pairs are computed once per position
and reused by all 36 layers and 40 Q/K heads. ARM64 K-quant dot products use
four CPU SIMD lanes while retaining the scalar accumulation order and the
pointer-only kernel ABI; this is CPU-only, not GPU acceleration.

The isolated Apple Silicon CPU benchmark (`tools/qwen-dot-bench.c`) measured
Q4_K 0.313 s scalar versus 0.068 s vector, and Q6_K 0.301 s versus 0.091 s,
over 20,000 equal 12,288-element rows. One hundred varied rows per format
matched bit-for-bit. These are host microbenchmarks, not guest tokens/sec.
The real-weight prefill parity test also selected the identical next token
with intermediate projections disabled.

Installed ARM64 proof after optimization: with NIC disabled and ISO detached,
the 12 GiB QA guest accepted `hello` through Ctrl+J and Enter, emitted real
model tokens, completed `Hello! How can I assist you today?`, and returned to
ready. The first inspected output was within approximately 66 seconds of
submission; completion was observed within 148 seconds (polling bounds, not
precise timings). That run also exposed a literal closing-think marker.
The template now inserts vocabulary IDs 151667/151668 for its trusted thinking
delimiters, matching the upstream tokenizer configuration, instead of ordinary
BPE-encoding their text. Real-weight host tests pass with the corrected template.
The final QA disk contains this correction and a decode-rate readout, but its
guest recheck is pending login. A launcher rendering failure was also observed
after the first completed run; it is not included in Qwen numerical acceptance.
The native tokenizer also passes UTF-8 round-trip checks for Latin accented,
Chinese, Arabic, and emoji text; these are not reference-tokenization parity
or guest-generation tests.
