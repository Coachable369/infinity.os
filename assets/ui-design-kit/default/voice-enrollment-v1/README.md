# Voice enrollment — implementation target

Reference: `reference.png`, 1672 × 941. Companion to facial enrollment; reuse
Glossy-Blue-Infinity-Wave-Wallpaper-2.png, Precision panel surfaces, Access Inter
fonts and packaged primary/secondary button states. No custom outer border.

The panel occupies 4.5%–95.5% horizontally and 6.5%–93.5% vertically.
Header: small secure-sign-in eyebrow, large title, one-line subtitle.
Left: read-aloud card and live waveform, adjacent instruction/status block,
three equal sample cards (first, second, independent verification).
Right: generated silver microphone artwork with cyan scan rings. No camera image.
Footer: privacy/fallback explanation, Back, Skip, Record/Continue actions.

States: idle, recording, comparing, retry, saved. Recording shows actual local
microphone amplitude with a restrained activity pulse before speech begins. Comparison has a distinct moving pulse and never simulates a successful capture. A rejected
sample retains prior accepted recordings. Navigation revokes capture immediately.
Reduced motion freezes decorative rings; live capture feedback remains legible.

Native geometry and Studio authoring must use screen 10 of the same configuration
template. The matching reference comparison and native screen acceptance remain
open until rendered snapshots have been reviewed.

Security: this is speaker verification, not speech-to-text. A matching transcript
is insufficient. The reference phrase guides recording; it is not a secret.
RGB face matching and this speaker model do not establish liveness or prevent
photo/video/recording replay. Password sign-in remains available. Enrollment is
optional; face-and-voice login requires an enrolled voice. No desktop assistant
wake, synthesis, echo, interruption or recognition behavior is changed.
