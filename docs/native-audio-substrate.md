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

Reference behavior: QEMU `hw/audio/intel-hda.c`, `hda-codec-common.h`; VirtualBox
`src/VBox/Devices/Audio/DevHdaCodec.cpp`. Implementation is native Rust, not copied
controller-emulator source.
