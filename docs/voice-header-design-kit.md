# Voice header — existing Infinity glass kit extension

Reuse the shipped chat panel, typeface, cyan focus color, rounded window controls,
and 46-unit header. Do not add a bitmap microphone icon or a second panel.

The status control ends 66 units before the panel's right edge, preserving the
minimize/close gutter. Its width is at most 146 units and shrinks on narrow widgets
without overlapping the `AI CHAT` title. Hit height: 30 units, centered in header.

States: `VOICE OFF` (activate), `LISTEN` (mute), `HEARING` (recognition),
`THINKING`, `SPEAKING`, `STOPPING`, `UNAVAILABLE`. Clicking an active state mutes.
Closing the widget also stops capture; minimizing retains a visible voice status.

Listening displays a cyan sine wave immediately before its label. Height follows
captured PCM amplitude, capped at seven units; silence stays near flat. Motion is
limited to 30 header-only updates/second. No spinner, fake amplitude, or full-screen
animation. Thinking retains the existing grayscale text animation.

Privacy: capture starts only by explicit activation in an active login, stops on
lock/logout/mute, and is paused during spoken replies to avoid self-transcription.
The current implementation is turn-taking, not acoustic-echo-cancelled full duplex.

Verification: inspect both normal and minimized headers, mute/close, silent input,
speech amplitude, lock, and repaint damage. A compile pass is not visual acceptance.
