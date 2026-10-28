# Voice interaction v2

Reference: `voice-interaction-v2-reference.png`. This extends Eclipse Precision;
the existing shared glass surfaces, typography and frame geometry remain authoritative.

- Header: live microphone envelope, readable status, independent stop-audio button.
  The status is not a dropdown. Keyboard focus 6 activates Stop.
- Microphone visualization consumes amplitude only, with immediate attack and
  gradual release. It does not consume or retain PCM samples.
- A low-opacity blue light moves beneath the conversation while generating or
  listening. Reduced-motion mode suppresses this decorative movement.
- Settings editable values use shared glass button surfaces; readonly values
  remain plain text. Existing typed actions and permission checks remain authoritative.
- Clicking an exposed desktop or widget minimizes applications without closing
  their tasks or discarding documents. Application-covered pixels and the dock
  retain their existing input handling.

## Generated artwork

Created with the built-in image generator. No external runtime image service is used.

`stop-audio-v1.png` prompt: A single isolated transparent front-facing square glossy
navy button, icy silver antialiased edge, white speaker and stop square, no text
or perspective; readable at 40 pixels. Native bitmap packaging preserves alpha
and downsamples with Lanczos through `tools/build-voice-artwork.py`.

Reference prompt: InfinityOS Eclipse Precision voice widget and Settings design
board; midnight #090F26/#101B3A, silver #86C7FF, blue #3E6FFF; Inter typography;
waveform and status without dropdown, glass stop-audio button, subtle aurora;
Listening, Thinking, Speaking and Idle variants, reusable Settings controls.
Generated example labels do not override installed voice names or OS copy.
