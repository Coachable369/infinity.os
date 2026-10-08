# First-login freeze investigation

## Reproduction

A disposable ARM VirtualBox 7.2.16 VM was installed from the full public
`InfinityOS-aarch64.iso`, with 8 CPUs, HDA input/output, USB input, and TPM 2.0.
The ISO was detached before initial configuration. The installed desktop reached
voice capture and responded to keyboard input, then froze after several minutes.
The user's VM and disk were not modified.

Diagnostic evidence is retained under `build/desktop-fresh-vbox/`:
`serial.log`, screenshots, CPU register samples, `host-hang.sample`, and the
disposable VM's `Logs/VBox.log`. Debugger sampling and a separate GUI were used,
so this is diagnostic reproduction, not an uninstrumented acceptance pass.

## Two distinct stalls

1. InfinityOS synchronously calculated 15,360 filter coefficients using software
   floating point on the desktop input thread. A CPU sample located execution
   in `Resampler::configure` through `__adddf3`. Initial capture's five-second
   lease expired before initialization completed. Fixed by embedding canonical
   little-endian Q30 filter banks and selecting an immutable bank at startup.
   The same bytes are compiled into live and installed kernels on both targets.
2. The persistent freeze occurred in the host's virtual audio device. CPU0
   waited in `PGMPhysRead -> PDMCritSectEnter`; EMT7 waited in the audio timer's
   mixer lock; `MixAIO-2` continuously called the CoreAudio capture and readable
   callbacks. Even register queries and power-off stalled. Only the disposable
   backend was forcibly stopped after bounded graceful shutdown failed.

The installed `VBoxDD.dylib` loop at offsets `0x1abd38` through `0x1abd88`
matches `drvAudioStreamCaptureLocked`: successful zero-byte reads do not break
the loop. The CoreAudio callback returns zero bytes for a request equal to one
frame because its transfer loop uses `cbBuf > cbFrame`. This is a matching
no-progress mechanism, not yet a captured proof of the exact buffer arguments.
Host debugger attachment was denied, so those arguments were not inspected.

Primary sources:
- [Oracle audio connector](https://github.com/VirtualBox/virtualbox/blob/main/src/VBox/Devices/Audio/DrvAudio.cpp)
- [Oracle CoreAudio backend](https://github.com/VirtualBox/virtualbox/blob/main/src/VBox/Devices/Audio/DrvHostAudioCoreAudio.cpp)

No VirtualBox binary, host permissions, or user's VM settings were changed.
The embedded-filter change alone must not be described as fixing this persistent
host audio lockup. Disabling VM audio input is a possible temporary isolation
step, not a completed voice solution.

## Verification commands

Run separately because the build kit serializes work:

```sh
./build-kit run env CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin voice-pcm-coefficients -- --check
./build-kit run make voice-pcm-test
./build-kit run env CARGO_TARGET_DIR=build/behavior-harness cargo test --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin voice-toggle-test -- --test-threads=1
./build-kit run python3 tools/voice-pcm-install-parity.py
```

The generator checks exact binary contents. PCM tests exercise sample counts,
chunk invariance, passband error, alias rejection, clipping, VAD, reset, and
immutable-bank selection. The parity assertion checks actual ELF payload bytes.
The existing `audio-install-parity.py` ISO gate now also checks both canonical
filter banks in the actual live kernel and byte-verified installed kernel shards.

## Updated-image retest

The ARM build completed with manifest `20261007T175556491425Z-60875.json`.
The DSP executable and seven PCM/VAD unit tests passed. All 25 conversation
state tests passed (`20261007T181236758339Z-66335.json`). The published ARM ISO
passed the extended audio/filter parity gate
(`20261007T181251517958Z-66370.json`).

The ARM ISO SHA-256 is
`9e0200b8c9f3e1e1d2df156633392afbc937c1d6d79dfdfc442e25e1831f45d2`.
The x86_64 live and installed kernels compiled successfully in
`20261007T183759575110Z-67075.json`; both targets passed the direct ELF filter
parity check in `20261007T184227807527Z-67545.json`. This turn rebuilt the ARM
ISO only, not the public x86_64 ISO. No x86_64 installed-runtime acceptance is
claimed. These are incremental/focused builds, not a clean release build.

A second clean disk was installed from this updated ISO. The ISO was ejected,
the guest booted from its installed disk, and initial configuration completed.
No GDB connection, register query, or separate GUI was used during this retest.
The first capture started without the former initialization timeout, but the
desktop froze immediately afterward: its displayed clock stayed at 18:26:16
and the launcher shortcut did not open the launcher. A host sample taken only
after the freeze showed the same CPU0 device-lock wait, EMT7 mixer-lock wait,
and continuously spinning CoreAudio capture loop.

Evidence: `build/desktop-filter-vbox/desktop-first.png`,
`desktop-input-first.png`, `frozen-serial.log`, `frozen-VBox.log`, and
`host-hang.sample`. The persistent microphone-enabled freeze remains a failed
acceptance item. The fixed first-use filter stall is not its complete solution.

## Temporary isolation result

On the same disposable installed disk, disabling only VirtualBox audio input
allowed configuration and desktop interaction to complete. The launcher opened,
keyboard input opened Settings, and screenshots show the clock advancing from
18:33:25 to 18:35:59. The five-minute VM observation ended with normal power-off,
and the fixture's input setting was restored afterward. Evidence is under
`build/desktop-no-input-vbox/`; run manifest:
`20261007T183139763832Z-66938.json`.

This validates a bounded temporary workaround, not microphone functionality or
long-duration stability. With the VM powered off, its VirtualBox Audio settings
can disable audio input while preserving output. The user's VM was not changed.

## Source repair and deterministic regression (2026-10-08)

A fresh read-only sample of the user's hung VirtualBox 7.2.16 process again
shows CPU0 blocked in `PGMPhysRead -> PDMCritSectEnter`, CPU7 waiting on the
audio mixer, and `MixAIO-2` spinning at the same capture-loop offsets.
`build/ai-post-prompt-host.sample` retains that evidence. The serial trace
records completed speech playback and renewed capture before the stall.

The host-side source fix is
`third_party/patches/virtualbox/audio-capture-progress.patch`:

- CoreAudio capture accepts `cbBuf == cbFrame`, not only larger requests.
- The generic capture connector breaks on a successful zero-byte transfer,
  returning accumulated data rather than looping forever with locks held.

No listening delay, microphone disablement, Hermes change, security bypass,
or guest watchdog is substituted for this repair. Once the host is stuck in
the device lock, a guest watchdog cannot release it.

Run `./build-kit run python3 tools/virtualbox-audio-fix/test.py` to fetch pinned
GPL-3.0 upstream source at `32f5f1de3fe17a3df177d6b3259112bcbf79856e`, apply
the patch without fuzzy matching, and compile the actual two upstream
functions against deterministic AudioQueue/backend fixtures. Source hashes
and structured outcomes are saved in `build/virtualbox-audio-fix/result.json`.

Manifest `20261008T181142246566Z-10229.json` records:

- Original one-frame capture fails its byte-count assertion.
- Original zero-progress connector and combined one-frame CoreAudio/connector
  calls both exceed a two-second bound and are terminated by the harness.
- Patched zero, partial, error, single-frame, buffer-tail, empty, disabled,
  and combined capture cases all pass with AddressSanitizer and UBSan.
- Checks include exact returned bytes/counts, preserved partial progress,
  bounded callback count, and another thread successfully acquiring the
  released backend mutex.

These are executable source-regression tests, not an installed VirtualBox or
microphone acceptance pass. The installed host binary is unchanged. Deployment
requires building and installing VirtualBox with this patch using its normal
platform signing/install process, followed by microphone-enabled conversation
tests on a disposable VM. An InfinityOS ISO rebuild alone cannot deliver this
host-library fix. Do not mark the installed freeze resolved until that gate passes.
