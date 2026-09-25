# Native audio substrate — first playback increment

## Scope and architecture

One controller family: Intel HDA. QEMU `intel-hda` + `hda-duplex` is the
reference device; VirtualBox HDA/STAC9221 is also required, not yet certified.
No ALSA, PulseAudio, CoreAudio or host runtime is linked into InfinityOS.
The host audio backend belongs to the emulator, not the guest.

Firmware PCI discovery → native HDA controller → capability-checked audio
adapter → bounded PCM. Voice/STT/TTS will consume this boundary later.

The optional, formerly reserved 32-bit BootInfo word contains a firmware-assigned
HDA MMIO BAR below 4 GiB. Structure size/version remain unchanged; old loaders
provide zero and audio stays unavailable. Both installed and installer kernels
include the same driver. No audio is automatically played or recorded at boot.

## Implemented

- Bounded reset/codec-command waits, firmware-owned PCI BAR discovery and bus mastering.
- Codec/function-group/output-pin/DAC discovery, D0, output gain/unmute,
  stream routing, 16-bit stereo rate negotiation (48 kHz, fallback 44.1 kHz).
- Resident DMA buffers, two-descriptor cyclic playback, DMA-position/error reads.
- Integer-generated 440 Hz diagnostic signal; no embedded recording or host synthesis.
- `audio status` and authenticated `audio tone` Console commands. Tone lasts two
  seconds, uses an expiring AudioOutput capability and revalidates revocation.
- Typed IOP AudioTone / AudioCaptureStart IDs and request authorization contract;
  currently a local adapter, not a registered general-purpose streaming endpoint.
- Initial direct-route ADC discovery/DMA read/stop primitives. These are **not yet
  exposed as a usable capture service** and are not microphone acceptance proof.

## Behavioral evidence

- `make audio-test`: actual capability manager; caller binding, playback/capture
  authority separation, expiry, revocation, malformed payload rejection and
  deterministic whole-cycle PCM at both rates.
- `make audio-hardware-test`: small freestanding ARM64 test image runs the
  **same HDA Rust driver**, asserts hardware DMA position advances, stops output,
  and shuts down. Host verifier analyzes bytes emitted by QEMU's WAV audio sink.
- Initial passing result: **439.76 Hz**, **1.990 seconds**, **48,000 Hz stereo**,
  RMS **5787.9** (16-bit scale). Tone peak is 8191, approximately quarter scale.
- The one-descriptor version exposed cyclic-wrap discontinuities in QEMU. Two
  descriptors removed them; the ±2 Hz acceptance threshold was not loosened.
- `sh tools/audio-probe/run.sh coreaudio` exercises speaker forwarding on macOS;
  successful exit alone does not prove a human heard it.
- Latest repeated waveform measurement: 439.56 Hz, 1.995 seconds, RMS 5788.1.
  The current macOS QEMU invocation reports no host ADC driver; do not interpret
  its playback success as microphone support.
- `python3 tools/audio-install-parity.py` checks installed/live kernel payload
  identity and exact bootloader bytes in the installed and live ESP images.

QEMU's WAV backend leaves RIFF lengths unfinalized on shutdown in this environment.
The verifier validates the PCM header and analyzes actual emitted samples rather
than changing samples or accepting log messages as evidence.

## Remaining acceptance

1. Boot/install/cold-installed Console tone verification, without ISO attached.
2. VirtualBox speaker proof. Its codec advertises a different rate/topology;
   source compatibility is not runtime proof.
3. Complete selector/mixer input-route discovery for VirtualBox, bounded capture
   stream buffering, overrun reporting, cancel/device-loss handling and live mic proof.
4. Full AudioDevice/AudioBuffer/AudioCapture/AudioPlayback service interfaces,
   registered IOP stream operations and packaging parity assertion.
5. Voice providers and chat-header capture animation remain separate later work.

This increment does **not** claim the full audio substrate or voice interface is complete.

## VirtualBox capture follow-up (September 24–25, 2026)

The current source now uses resident CORB/RIRB command DMA instead of the
immediate-response register. VirtualBox's immediate-response register masks
guest reads; the DMA command path completes on both emulators. RIRB response
events are acknowledged while CPU interrupt delivery remains disabled.

Input discovery walks bounded ADC/mixer/selector paths, including VirtualBox's
6 → 23 → 18 → 14 route. Authenticated `audio capture`, `audio stop`, and
`audio status` expose a three-second diagnostic recording. A fixed mono FIFO
rejects overflow; owner/capability checks gate PCM reads. Stop, revocation and
failure erase captured samples. This is not yet the general voice stream service.

Behavioral results for this follow-up:

- `make audio-test` passes route traversal/cycle/error cases, fixed FIFO ordering,
  stereo folding, wrap, overflow rejection and clearing, plus authority tests.
- The same command-ring driver in QEMU emits 439.69 Hz for 1.992 seconds,
  RMS 5787.6 at 48 kHz stereo.
- A private copy of the installed VirtualBox disk cold-boots with no ISO attached,
  accepts login, and detects both HDA routes. The original disk is preserved.
- VirtualBox's host-side PCM debug capture contains actual generated tone samples
  at 44.1 kHz stereo. First use produced only 0.300 seconds; the second invocation
  added 1.975 seconds. The source now starts the duration timer after hardware
  initialization, within the existing bounded capability lease. That latest
  timing change was subsequently tested on a cold boot; the user confirmed hearing
  the approximately two-second tone through the VM speakers.
- After macOS microphone access became available, the first capture overrun
  check incorrectly included host device initialization. Capture now starts its
  duration and polling clocks after DMA initialization, without extending its
  capability lease. The rebuilt, disk-only installed system completed capture:
  **130,800 stereo frames at 44,100 Hz, peak 1,631**, with nonzero real microphone
  samples and no reported overrun. Captured PCM is erased on completion.
- Temporary host PCM debug recording is disabled. Subsequent verification uses
  the guest's counters and observed stream state, not recorded private speech.
- Installed cancellation test: a second capture was cancelled at **26,400
  frames**, peak **711**. The HDA input stream control register was **0x200000**
  afterward (stream tag retained, DMA RUN cleared).
- Another disk-only cold reboot returns capture to Idle, sample rate/frame/peak
  counters to zero, and both HDA stream control registers to zero. No active
  recording or playback survives reboot.

These follow-up changes are now in `builds/InfinityOS-aarch64.iso`, rebuilt
September 24 at 23:58 CDT, **8,590,336,000 bytes**. Published-artifact parity
passes for the **208,758,480-byte** installed kernel and **24,576-byte** loader.
Hermes and Ministral packaging assertions also pass. The runtime tests used an
updated private installed disk, not a new destructive installation from this
latest ISO. Full native STT, TTS, voice conversation, and the chat-header
waveform remain unimplemented.

## Build handoff

`make aarch64` completed successfully on September 24, 2026. The published
`builds/InfinityOS-aarch64.iso` is 8,590,336,000 bytes (21:55:53 CDT).
Published-ISO parity passed against the 208,747,744-byte installed kernel and
24,576-byte loader. ARM64 and x86_64 release checks pass; existing compiler
warnings remain. x86/x86_64 ISOs and the running VirtualBox installation were
not upgraded by this increment.

Reference behavior: QEMU `hw/audio/intel-hda.c`, `hda-codec-common.h`; VirtualBox
`src/VBox/Devices/Audio/DevHdaCodec.cpp`. Implementation is native Rust, not copied
controller-emulator source.
