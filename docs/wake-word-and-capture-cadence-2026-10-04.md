# Wake-word tolerance and microphone polling

The Infinity wake selection now also accepts the complete leading tokens
`Infin` and `finity`, case-insensitively, with the existing optional `hey` prefix
and punctuation. These are explicit aliases, not fuzzy substring matching.
The Computer selection is unchanged. An alias alone arms one command; an alias
with a command strips only the wake prefix and preserves the command bytes.

## Pointer investigation

Both native input loops call the audio services unconditionally, including when
no keyboard or pointer reports are available. ARM's controller runs after input
and animation processing; x86's idle branch also calls it. The desktop voice
animation is clock-driven, and the bootstrap pointer cooldown does not gate
desktop microphone capture. VAD advances from captured samples, not pointer
events.

One indirect coupling existed: the hardware adapter polled on every loop pass,
so idle/rendering workload changed its MMIO polling frequency. The adapter now
bounds this work to a 1 ms monotonic cadence. This introduces no sleeps and no
mouse dependency. Capture and playback use the same shared adapter on ARM and
x86. Security failures still fail closed and new streams can poll immediately.

## Verification scope

`tools/voice-toggle-test.rs` exercises the real controller and VAD with reviewed
recognizer results, including clipped wake-only/combined commands, exact prompt
preservation, configuration changes, and one-turn authorization. It uses no
pointer events; it does not measure real microphone recognition accuracy.

`tools/audio-poll-test.rs` exercises the production hardware adapter with a
deterministic clock and fake HDA transport. Idle-loop versus input-work schedules
must deliver the same ordered audio with bounded hardware polling. This test is
part of `tools/voice-pipeline-test.py`, which every ISO build runs.

The full voice gate passed under build-kit manifest
`20261004T081700473880Z-64098.json`: 24 controller/parser tests, the production
audio cadence case plus four PCM checks, and the existing timing, worker,
boundary, output, resampling, VAD and InfinityAudio checks. The cadence case
delivered identical 4,800-sample output with 200 calls/ms, one call/ms, and one
call/10 ms (100, 100 and 11 hardware reads respectively).

Read-only observations from the existing installed VM are retained in
`build/voice-capture-idle-20261004/evidence.json`. The service loop continued
renewing capture authority during multi-second zero-sample periods. At inspection
the session had auto-locked, and VirtualBox showed roughly 564 seconds of virtual
timer debt, including overdue non-audio timers. These snapshots confirm a timer
service problem, but do not establish its cause or a measured mouse correlation.
The earlier isolated native capture/timer probe did not reproduce it. The bounded
polling change must not be described as a verified fix for this hypervisor issue.

ISO packaging remains through `./build.sh --target aarch64`, with full model
bundle and live/installed payload checks. No VM restart or installed playback
acceptance is performed in this pass.
