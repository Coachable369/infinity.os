# Varied voice replies: output-delivery investigation

## Installed-session evidence

The current ARM VM uses the October 4 full-bundle ISO, SHA-256
`5355cad57cf20f2043111d319d33f20d0cbb3b4a8625833fc292c3cac76a9910`.
Four distinct response generations (2, 4, 6, 8) contain 79,906, 98,615,
119,473, and 116,285 stereo frames. Every guest playback reports completion;
none reports cancellation, timeout, underrun, or device loss. These are each
single synthesis spans, not phrase-queue rollover.

A single read-only VirtualBox statistics snapshot found **8 output DMA buffer
overflows** and **360 output DMA buffer problems**. These counters are
session-cumulative and do not attribute a failure to an individual reply.
The snapshot records a 12,780-byte DMA period at 44.1 kHz (72.45 ms), an
81,324-byte DMA ring, and 67,392 queued bytes. The complete structured snapshot
and original XML are in
`build/voice-output-20261004/current-vm-audio-stats.json`.

Oracle's HDA implementation attempts recovery when the output ring has
insufficient room. If recovery fails, the overflow path discards the queued
ring contents even while guest DMA progress continues. Thus guest completion
does not establish complete audible delivery.
[VirtualBox HDA stream implementation](https://github.com/VirtualBox/virtualbox/blob/main/src/VBox/Devices/Audio/DevHdaStream.cpp).

Capture-authority renewal was checked: it updates leases without restarting
capture or output DMA, so its overlap with the third reply is not proof of a
cause. No speech content or microphone PCM was extracted.

## Synthesis isolation

The current native engine passed 29 mixed recognition/synthesis/lifetime
transitions, including 12 varied synthesized replies and final intentional
fault quarantine. All 12 complete waveform hashes match the retained native
parallel-worker results for the same native object. This does not prove
speaker delivery, but provides no evidence of cumulative synthesis corruption.

- Native lifetime manifest: `builds/manifests/20261005T004052539624Z-53759.json`
- Serial/parallel PCM comparison: `builds/manifests/20261005T004638624907Z-54234.json`
- Native object SHA-256: `ed1c71fa19aaf6f32745d20115a86068724b46a13d210c8ada69b26cc07dc392`

## Native delivery A/B

Eight synthetic utterances use the four observed lengths with distinct signals,
including separated turns and immediate consecutive starts. The isolated
`infinity-audio-timer-test` VM uses the production loader, six application APs,
active guest capture, and **host microphone input disabled**. No private audio
was recorded.

| Output path | Intact before output ring | Intact after output ring | DMA buffer overflows |
| --- | --- | --- | --- |
| Original whole-phrase descriptor | 8 / 8 | 3 / 8 | 15 |
| Diagnostic <=20 ms IOC descriptors | 8 / 8 | 8 / 8 | 0 |

All eight ending markers survived the original path: the failure includes
interior audio loss, not just stopping before the last word. The second path
keeps the same speech and silence-drain threshold; it changes descriptor pacing,
not synthesis, capture, or artificial inter-phrase delays. Periodic descriptor
completion is acknowledged but is not treated as end-of-speech.

Native serial telemetry lost some bytes in both runs. Those incomplete timing
records are not acceptance evidence. The waveform oracle checks actual complete
PCM bytes and sequence at both the DMA and post-ring HDA-to-mixer boundaries,
plus structured device counters. It does not establish physical speaker output
or a complete installed conversation.

- Original artifacts: `build/arm-capture-timer-probe/playback/1791161690395172000/`
- Short-period artifacts: `build/arm-capture-timer-probe/playback-short/1791161975244151000/`
- Short-period run manifest: `builds/manifests/20261005T005934922852Z-55247.json`

## Production fix and verification

The shared HDA driver now uses 256 fixed, aligned descriptors with periods no
larger than 20 ms. A roughly five-second PCM ring is filled from the existing
immutable response. Only consumed halves are refilled, with a full FIFO reserve
checked before and after each bounded copy. The final guard is zero-filled.
Capture has a separate descriptor array. A native monotonic clock detects missed
rotations; failures stop playback instead of silently replaying stale samples.
No wake-word, recognition, LLM, or application-level speech ordering was changed.

Behavioral tests consumed every sample across 22 varied playback generations at
44.1 and 48 kHz, including maximum-length replies, ring/drain boundaries,
nonterminal IOC events, invalid/backwards cursors and missed service deadlines.
`./build-kit run make audio-test` passed:
`builds/manifests/20261005T011046925610Z-56322.json`.

The actual production driver then delivered **9 / 9** distinct watermarked
signals intact, once and in order, at both the DMA and post-ring boundaries.
The ninth signal lasted 32 seconds and exercised multiple refills. Device
read/write positions both reached **9,569,664 bytes**, with **zero output buffer
overflows**. Host microphone input remained disabled.

- Production evidence: `build/arm-capture-timer-probe/playback-production-long/1791162732774945000/evidence.json`
- Native run manifest: `builds/manifests/20261005T011212532938Z-56589.json`
- Reproduction instructions: `tools/arm-capture-timer-probe/README.md`

These gates prove the reproduced delivery defect is repaired at the measured
boundary. The final rebuilt installed conversational path and physical speaker
output still require the user's retest. No x86 hardware acceptance is claimed.
The ARM ISO is produced only through `./build.sh --target aarch64`, including its
full-bundle and live/installed boot-payload gates.
