# AudioKit

AudioKit documents supported audio data preparation and the boundary with the
native OS audio/voice services. It does **not** claim a public real-time microphone,
speaker or text-to-speech binding that the application SDK does not yet expose.

## Quick start: generate a real WAV

Copy `/InfinitySDK/Examples/Python/audio_tone.py` to Documents, open it in Text
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

## Permanent OS AI audio pipeline journal

Every live boot and installed boot enables the native journal automatically.
The OS creates `/Logs/ai.audiopipeline/` when its first batch is committed.
This is independent of the desktop widget's focus and does not require an SDK
application or Java runtime. `voice status` reports the directory, durable batch
count, failed writes, dropped records, and last write cost.

Records use Java `java.util.logging` style levels (`INFO`, `FINE`, `WARNING`),
logger names under `infinity.ai.audio`, and an ISO-8601 UTC timestamp. This is a
native single-line text formatter, not a Java binary serialization format.
If UTC is unavailable, the timestamp is explicitly `UTC_UNAVAILABLE`. Every
record also includes a monotonic nanosecond timestamp and boot-local sequence.
Use monotonic values for elapsed time; changing the timezone or wall clock must
not change measured performance.

The journal captures:

- Capture calibration, VAD transitions and one-second level/energy heartbeats.
- Recognition queue time, model warmup, compute time, failure and cancellation.
- Intent routing, memory inquiry, context retrieval and prompt preparation.
- Inference load, prefill/decode work, cached/new/output tokens and first token.
- First response text and **actual first display** as distinct milestones.
- Synthesis queue/compute, publication, playback queue, first audio and drain.
- Microphone rearming, automatic recovery, service-loop work and render work.

`turn`, `recognition_job`, `chat_turn` and `speech_job` correlation records connect
these stages. Timing `present` is a bitmask: a missing milestone is not a zero-time
success. `phases` bits 0–10 are endpoint, transcript, submitted, first text,
speech queued, first audio, drained, capture started, recognition ready, first
display and generation done. `mic_rearmed_present` separately qualifies the
rearm timestamp. Span status is the native provider error code; zero means no
reported provider error. Playback completion records report the driver's state,
not proof that sound physically reached the listener.

Files `pipeline-00.log` through `pipeline-15.log` are a rotating journal, at most
16 KiB each (256 KiB total). Records are buffered in a bounded producer queue;
workers never write to disk or wait for logging locks. The owner thread flushes
at most once every five seconds, retaining a failed batch for retry. Writes use
atomic native checkpoints with bounded history. A power loss can lose the
unflushed interval. Overflow/contention is counted explicitly. Rotation replaces
old log pages; timestamps and sequence numbers determine order, not filenames.
No transcripts, prompts, generated text, credentials or raw audio are recorded.

To investigate a silent reply, compare `first_text_ns`, `first_display_ns`,
`speech_queued_ns`, `first_audio_ns` and `drained_ns`, then inspect the correlated
synthesis/playback spans. To investigate missed wake input, inspect capture
state, energy and threshold around that time before blaming inference.
