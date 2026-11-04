# Desktop, microphone and browser responsiveness

## Changes

- Background desktop damage uses a 16.67 ms start-to-start frame budget. Paint time no longer adds another 33.33 ms delay. Expensive inference-time decoration retains a proportional CPU budget; input event draining is unchanged.
- Speech preparation, recovery and recognition use the speech-priority worker admission path.
- An authorized microphone reporting Recording but delivering no samples for three seconds is reopened, with at most two consecutive retries. Recovery is restricted to listening, preserves recognition handoff, and does not restart after a manual stop. Playback-lock contention no longer discards ambient calibration accumulated so far. Invalid-rate recovery retires its capability.
- Browser gzip bodies decode incrementally, allowing HTML and dependency discovery before the complete body arrives. Cumulative decompressed limits and final CRC/trailer validation remain enforced. Attachments remain unpublished until complete.
- Browser navigation stops holding the busy pointer after its first new frame or two seconds. Actual document loading/error status remains independent and accurate.

## Focused evidence

- Frame scheduler: seven behavioral checks pass, including render cost inside the frame period and preservation of final pending damage.
- Browser compression: nine checks pass, including output before the trailer, exact bytes, cumulative expansion limits, truncated input and CRC failure.
- Voice controller: 49 checks pass, including zero-sample recovery, calibration contention, wake/command handoff, cancellation and authorization. The capture fixture supplies no samples for the recovery scenario explicitly.
- Browser core: 62 checks pass, including pointer feedback independent from document completion.
- Production window skin checks pass for Studio overrides, shared frames, retained translation/clipping/focus changes, window controls and AI adorners. Existing shared artwork was preserved; the production control image was visually inspected.

Focused manifests:
- `builds/manifests/20261104T041047070668Z-40670.json` (voice controller and browser core)
- `builds/manifests/20261104T041110348427Z-40736.json` (window skin authority and production pixels)

## Acceptance limits

The user's VM was closed and was left stopped. These checks do not establish real microphone input, installed-VM frame rate, public-site completion speed, or conversational end-to-end latency. The full clean ARM64/x86_64 release passed. Building these images does not update an installed VM.

## Release

`./build-kit full` passed in 6030.434 seconds. Manifest: `builds/manifests/20261104T041638465080Z-45240.json`.

- ARM64: `builds/InfinityOS-aarch64.iso`, SHA-256 `d54582406b902d2abd038c0dae8afcd0437324c15f07059eb21690b558bac3e6`.
- x86_64: `builds/InfinityOS-x86_64.iso`, SHA-256 `a22ae2f17c6c257c7aa97ec7584fcb8bd968c38737ee372e192d9c51a4929f73`.

Current-kernel, live/installed UI, shared artwork, SDK-resource and native-model packaging checks passed. The build includes the existing saved Studio designs, which remain separate from this code commit. The manifest records the pre-commit HEAD; the working-tree changes described here were compiled in that run.

The host framebuffer-copy harness measured 1.608 ms average and 1.943 ms p95 for 16,384,000 bytes over 60 samples. This is not an installed-VM frame-rate or before/after end-to-end performance claim.
