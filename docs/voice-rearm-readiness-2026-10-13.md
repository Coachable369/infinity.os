# Voice rearm and model readiness

The installed trace preserved in `builds/validation/voice-rearm-20261013/serial.log`
shows successful model turns and drained replies followed by new VAD onsets and
ten-second sample limits. Initial calibration admitted a threshold of 300 while
the final calibration chunk had RMS approximately 402; a later run admitted 434
and post-reply noise repeatedly crossed that threshold. These energy summaries
do not establish what words were spoken or recognized.

Changes:

- Calibration uses the greater of the whole-window median and the settled final
  nine-frame median, preserving the existing 500 ms budget and impulse rejection.
- Extending speech and rejecting a pause require the same three-frame sustained
  voice evidence as onset. An isolated noise crossing no longer latches the
  ten-second retry state. Pending onset frames defer endpoint publication until
  they resolve; genuine overlong speech still cannot become a truncated command.
- The existing veil follows selected language-model availability, not microphone
  readiness. Both visibility edges retain full-body damage until rendered.
  Microphone preparation stays in the header and cannot block typed conversation.
- Wake-word authorization, recognition models, inference, synthesis, and playback
  ownership are unchanged.

Verification: the shared voice pipeline gate passed, including 31 controller,
calibration, VAD, and animation tests. Fixtures cover startup ramping, noise spikes
after speech/overlong input, sustained speech near the endpoint, and retained
body damage on veil removal. Hardware wake-word accuracy and rendered installed
behavior require retesting; host fixtures are not installed-runtime proof.
