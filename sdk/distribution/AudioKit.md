# AudioKit

AudioKit documents supported audio data preparation and the boundary with the
native OS audio/voice services. It does **not** claim a public real-time microphone,
speaker or text-to-speech binding that the application SDK does not yet expose.

## Quick start: generate a real WAV

Copy `/InfintySDK/Examples/Python/audio_tone.py` to Documents, open it in Text
Editor and press Run. The script writes `infinity-tone.wav` in its working
directory and prints that path. It generates a 0.5-second, 440 Hz, mono, 16-bit
PCM WAV at 16 kHz using only Python's standard library. No pip packages, network
access, native extension or permission to capture the microphone is required.
The file has 8,000 samples and 16,000 PCM bytes, plus its WAV header.

The script does not play sound automatically. Inspect or export the file using
available file tools. Speaker playback and microphone integration need a future
public, capability-checked application binding; internal service symbols are not
safe substitutes. This limitation is distinct from the OS's functioning AI voice
pipeline and system sound service.

## Reference

`write_tone(path, frequency=440.0, duration=0.5, rate=16000)` returns the number of
frames written. Frequency must be positive and below Nyquist (`rate/2`); duration
must be positive and no more than ten seconds; rates are 8000, 16000, 22050, 44100
or 48000. Invalid parameters raise `ValueError` before creating an output file.
The example uses signed little-endian 16-bit samples, a 20% peak amplitude, and
a short attack/release envelope to avoid abrupt waveform edges.

Python dependencies: `math`, `struct`, `wave`. WAV files record channel count,
sample width, sample rate and frame count; never interpret encoded JPEG/MP3 bytes
as PCM or change rate metadata to resample audio. Keep peak samples in the signed
16-bit range. Long streams should use bounded chunks rather than building an
entire recording in memory; the example caps duration and streams one frame at
a time for simplicity, not as a real-time performance recommendation.

## OS integration architecture

The native service distinguishes AudioInput and AudioOutput capabilities,
validates caller identity and bounded deadlines, and revalidates revocation.
Private captured samples are cleared after consumption; cancellation must not
leave stale audio queued for a later user. The OS AI pipeline owns wake-word,
transcription, inference and speech output. Integrate application actions through
AIKit so the OS can speak results without your app competing for microphone
ownership. Do not call kernel driver functions from application code.

## Troubleshooting and acceptance

- Permission denied writing: select a personal writable path; never write in SDK.
- Missing module: use the shipped native Python runtime, and report the exact
  missing standard-library module rather than silently installing a substitute.
- Wrong pitch/duration: inspect sample rate and frame count together.
- Click at start/end: preserve the envelope and return to zero at boundaries.
- No audible output: expected; the example creates audio data, not playback.

Package tests decode the generated WAV and assert metadata, duration, nonzero
bounded samples and rejection of invalid input. Those tests do not establish
physical microphone/speaker behavior. Native capture/playback SDK acceptance
remains pending until a public binding and device tests are supplied.
