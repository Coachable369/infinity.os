# Progressive Conversational Speech

Conversation replies use `submit_progressive_span`: the first bounded phrase
starts playback as soon as its PCM is ready. One subsequent phrase can synthesize
on a background worker while the current phrase plays. The existing native
matrix/convolution helpers remain enabled; this change does not add concurrent
model owners or change microphone, VAD, wake-word, or recognizer admission.

Streaming text waits for a natural sentence boundary rather than dispatching at
the first space after 24 characters. Commas, semicolons, and colons can release a
first clause after 40 bytes, or a following clause after 80 bytes. Common title
abbreviations, initials, and decimal points do not end a sentence. Completed
answers of up to 160 bytes remain one synthesis request, even when they contain
several short sentences. Short unfinished greetings are coalesced with the next
phrase instead of forcing another native synthesis startup.

For unpunctuated input, 160 available bytes is the hard buffering bound: prefer
an earlier punctuation boundary, otherwise the last whole-word boundary within
that limit. An oversized indivisible token uses the provider's existing bounded
path. This bounds buffered text, not wall-clock latency when generation itself
is slow. The conversation normalizer produces printable ASCII before splitting,
and speech submission independently rejects non-ASCII input; no UTF-8 sequence
is cut on this path.

The worker's PCM and converted samples are private. The driver receives a copy
in its sealed resident generation, which remains immutable until hardware drains.
A ready lookahead waits for that drain before opening the next generation. If
synthesis is slower than playback, a gap can still occur; this is bounded
phrase streaming, not a claim of uninterrupted sample-level streaming. It
removes the requirement to synthesize an entire response before hearing it.

Only one lookahead can be accepted. Foreign owners, overlapping jobs, and
rewound active content ranges are rejected. Cancellation, permission revocation,
and synthesis failures stop playback and retire private results without handing
cancelled audio to the device. The first-playback timestamp is preserved across
phrase generations. Text synchronization does not reveal unspoken later text
just because an intermediate phrase has drained.

The existing `submit_span` API retains whole-response buffering for callers
that need that contract. No new payload component or architecture-specific
implementation is introduced; live and installed kernels use the same code.

## Verification Boundaries

`voice-output-test` runs the production output and input state machines against
deterministic synthesis, device, clock, and authority seams. It checks early
playback, fast/slow lookahead, immutable playing PCM, ordering, timing, failure,
and subsequent recognition admission. `voice-toggle-test` separately runs the
production conversation controller with mocked speech services, including
repeated fresh wake-word admission after progressive replies. Neither fixture
proves microphone recognition or speaker quality in an installed VM.
Phrase tests separately verify incremental token arrival, punctuation boundaries,
abbreviations, bounded unpunctuated delivery, and lossless final flushing. Output
tests distinguish actual slow synthesis from segmentation, checking that an
underrun neither replays earlier PCM nor publishes a false completion and that
ready lookahead proceeds on the first eligible drain poll. No numeric audible
latency or naturalness improvement is claimed without a measured PCM capture.
