# Login ripple fade and audio tail

The previous success transition held opacity at 255 and cut directly to the
desktop after 1.6 seconds. It did not fade the login scene or the desktop.

The shared authentication timeline now runs a 2.4-second orb/water scene.
At 1.2 seconds, while both rings are expanding, it begins a smoothstep fade
to black. Desktop setup and its first render occur at zero opacity, followed
by a 600-ms smoothstep fade-in of the rendered desktop. Input remains gated
through the transition. Visual catch-up is capped at 50 ms per frame so a
rendering stall cannot skip the visible waves. Slow machines may therefore
take longer than the nominal three seconds.

Audio remains independent of the visual timeline. The complete resident login
PCM is unchanged; 250 ms of existing zero-filled resident storage is included
in the playback stop threshold to allow DMA/codec output to drain. The visual
clock starts after PCM preparation rather than including preparation latency.
This protects the final audio tail but is not proof of the reported audible
cutoff's sole cause.

## Verification

Build-kit manifest `20260928T011118806356Z-15150.json` passed:

- Authentication timeline tests: exact commit and completion, input gating,
  monotonic fade-out/in, and visible ripples under stalled-frame updates.
- ARM64 and x86_64 native-browser kernel compile checks.

These are shared kernel paths for live and installed systems, with no new
assets or installer-only logic. No ISO was rebuilt in this pass. Installed VM
visual and audible verification is pending; no hardware result is claimed.
