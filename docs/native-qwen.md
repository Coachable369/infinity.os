# Native Qwen3-8B bring-up

The current installer also packages [Ministral 3 3B](native-ministral.md) for
comparison. That document describes the additional P3 payload, larger ESP and
combined RAM reservations; Qwen-only sizes below describe the earlier layout.

## Performance follow-up

The later [editor/CPU follow-up](editor-pointer-performance.md) removes the
four-worker cap and extra spare-core reservation. The timings below remain
historical four-worker measurements, not measurements of that new policy.

### Balanced batches and four-row Q4 kernels

The dispatcher balances small projections across online workers and limits each
worker to 4,096 rows per job. Larger output mailboxes add 60 KiB across four
workers compared with 256-row jobs; model format, weights, context and KV layout
are unchanged. Cancellable jobs still publish into private buffers only.

Q4_K rows are interleaved four at a time, sharing activation loads and exposing
independent arithmetic to the CPU. Each row retains the original accumulation
order; Q6_K and 1–3-row tails use the existing kernel. Behavioral tests compare
float bits across widths 256 through 12,288, zero rows, tails, multiple worker
batches and cancellation. Host timings are diagnostics, never test pass criteria.

`ai timing` shows per-request first-token latency, token count, decode interval,
BSP prefill/decode work and maximum pump duration in six visible console rows.
`ai workers` shows cumulative worker compute and completed-result idle time,
summed across cores. Those sums are not elapsed request times and exclude pending
uncollected jobs. Timing uses the hardware counter without per-job logging.

The longer busy-poll experiment was rejected: decode improved only from 15.4 to
14.3 seconds while BSP work rose from 3.25 to 6.18 seconds. The original bounded
256-iteration / 2 ms pump remains. A vector-byte-unpacking experiment was also
rejected because it showed no speed gain. The four-row Q6 experiment was slower
and is not included.

### Installed guest measurements (2026-09-12)

The 12 GiB, six-vCPU ARM VirtualBox QA clone booted from its installed disk with
the ISO detached and no NIC. The final kernel answered `hello` with
`Hello! How can I assist you today?` (11 tokens). First-token latency was
15,352 ms; the following ten decode intervals totaled 14,386 ms (0.695 tokens/s,
displayed as 0.6). This is roughly twice the earlier 0.3 tokens/s guest baseline,
but **still below satisfactory interactive performance**.

BSP prefill/decode work was 2,582/3,178 ms and maximum service pump was 4,619 us.
Four workers completed 24,672 jobs, totaling 55,056 ms compute and 34,099 ms
completed-result idle across cores. These aggregate counters are not wall time.
Compared with the same balanced dispatcher using the original row kernel,
decode fell from 15,424 to 14,386 ms; host Q4 microbenchmarks improved about 30%,
which must not be confused with end-to-end guest speedup. Runs were single samples
on the same host, not statistically controlled benchmarks.

A second request showed the animated thinking indicator; keyboard Escape
cancelled it and the desktop remained visible. Arithmetic tests cover exact-bit
parity, cancellation, multi-batch outputs and tails. The final installed clone was
updated in place with rollback disks retained; the full fresh-install interaction
was not repeated for this optimization. Rebuilt media and extracted installed-ESP
parity cover packaging, separately from that installed runtime check.

Remaining performance work is batched prompt prefill and reducing worker result
collection idle without busy-waiting on the desktop CPU. Neither is claimed here.

### Earlier worker bring-up baseline

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

ARM inference now dispatches bounded Q4_K/Q6_K row batches to up to four native
secondary-core workers. Boot ABI v8 appends a versioned worker-start bridge
(264-byte BootInfo). UEFI MP Services is preferred; validated ACPI FADT/MADT
data permits a PSCI CPU_ON fallback on identity-mapped EL1 systems. AP stacks
are reserved before the boot memory map is captured. Unsupported firmware or
unavailable CPUs retain the single-core path; x86 initializes the bridge to zero.

Workers own private activation/result mailboxes and read immutable model weights.
They never access the desktop, services, allocator, or engine state. Release/acquire
ownership publishes bounded jobs; cancellation discards results without reclaiming
worker-owned memory, and engine destruction drains jobs before releasing weights.
This is a dedicated inference worker pool, not a general SMP scheduler. One CPU
is reserved for the desktop and an additional spare is retained when available.
Batched prefill and worker-fault recovery remain unimplemented.

The disk-only 12 GiB, six-CPU VirtualBox QA guest (ISO detached, NIC absent)
completed `hello` with `Hello! How can I assist you today?`. The final 256-row
batches produced visible text within 56 seconds and completed within 88 seconds
of submission; these are screenshot observation bounds, not exact latency metrics.
The guest displayed 0.3 tokens/sec. The initial 32-row batches had no visible text
at 126 seconds and first observed text at 185 seconds, so that granularity was
rejected. A rigorous repeated single-core/multicore benchmark remains outstanding;
0.3 tokens/sec is still not satisfactory interactive LLM performance.

After a second request and keyboard cancellation, the final guest reported four
online workers, 208,520 completed jobs, 24 cached prompt tokens and a maximum
AI pump of 4,385 microseconds. Spinner/clock continued updating; Escape cancelled
the request and a second Escape released chat focus for launcher keyboard use.
Concurrent host tests compare Q4_K/Q6_K results bit-for-bit with direct execution,
including multi-batch tails, cancellation and changed destination geometry.
ARM kernel/loader, x86 loader/check, AI tests and generated installed-ESP parity
passed. This revision was tested by updating a preserved installed QA clone;
the new ISO's full fresh-install interaction was not repeated.

The desktop chat header shows a grayscale shimmer on `Thinking...` during
active generation. The rotating spinner has been removed. Its 30 Hz monotonic-clock
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
and reached the desktop with no network adapter. Subsequent installed-clone
output and performance observations are recorded in Performance follow-up above.

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
Broader prompt-quality and sustained desktop-interaction acceptance remain pending.

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
