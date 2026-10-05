# Native ARM audio diagnostics

These bounded probes use InfinityOS's production loader and HDA driver inside
the explicitly approved disposable `infinity-audio-timer-test` VM. They do not
build an ISO or alter an installed/user VM. Every invocation requires build-kit.

## Production output gate

```sh
./build-kit run python3 tools/arm-capture-timer-probe/playback.py --build-only --production-long
./build-kit run python3 tools/arm-capture-timer-probe/playback.py --run --production-long --vm infinity-audio-timer-test
```

The gate plays eight different short utterances followed by a 32-second source,
with ADC service active and the host microphone explicitly disabled. Nonperiodic
sample watermarks expose replay after a DMA ring wrap. Acceptance requires each
whole source exactly once and in order in both raw DMA and post-ring recordings,
plus zero HDA output-buffer discards. Inter-turn gaps are test cases, not runtime
playback delays. Lossy UART telemetry is retained separately and cannot substitute
for PCM evidence.

Each run retains a new artifact directory under
`build/arm-capture-timer-probe/playback-production-long/`. Prior disks, waveforms,
statistics and manifests are not overwritten. Temporary VirtualBox audio debug
settings are restored after the bounded run. Recordings contain synthetic tones
only; this is a device-path gate, not installed conversational or speaker proof.

Reanalyze an existing artifact without running a VM:

```sh
./build-kit run python3 tools/arm-capture-timer-probe/playback.py --analyze build/arm-capture-timer-probe/playback-production-long/<attempt> --production-long
```

## Retained short-period experiment

`--short-periods` selects an active diagnostic-only immutable descriptor layout
in `short_periods.rs`, not a second production driver. Its retained A/B recordings
isolate descriptor cadence from refill behavior. The default build always uses
the current production HDA implementation; an old baseline must be analyzed from
its preserved artifacts, not assumed reproducible from today's source.

The raw DMA and post-ring WAVs are the exact PCM oracles. The optional connector
`DrvAudioPlay` recording can retain a stale WAV data-length header across stream
restarts and is therefore diagnostic only.
