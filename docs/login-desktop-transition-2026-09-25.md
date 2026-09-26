# Login-to-desktop transition

The authentication scene retains the orb drop, splash, and expanding water
ripples at full opacity. At 1600 ms it hands directly to the fully rendered
desktop without a fade-out, black frame, or desktop fade-in. Advancement follows
real elapsed time so a slow software renderer cannot stretch the transition
until its audio ends. Mandatory impact, expanded-ripple, and final-ripple
checkpoints remain visible even when a delayed frame crosses more than one
authored keyframe.

The authentication and desktop scenes remain fully opaque throughout the
handoff. Ordinary desktop damage handling and displays without a software back
buffer retain their existing presentation behavior.

Login audio now preconverts the complete existing cue to an aligned resident
stereo buffer and starts hardware DMA through the existing capability-checked
Audio Service. It no longer depends on 100 ms buffer refills while the login
scene or desktop is rendering. Desktop entry does not stop the cue. The
resident storage is reused only after stopping the same kernel owner's prior
playback, and other owners' playback is not interrupted.

## Verification

- `make authentication-motion-test`: two behavioral tests for drop, ripple,
  direct full-opacity handoff, gated completion, delayed frames, and packed
  pixel opacity endpoints.
- `make system-sound-test audio-hardware-test` passed. The login PCM asset
  contains 87,772 frames (5.485 seconds). QEMU resident DMA emitted a 439.85 Hz
  test tone for 1.998 seconds without scheduler refills. This checks the resident
  playback mechanism, not listening to the login cue in installed VirtualBox.
- Installed visual smoothness and uninterrupted login-cue listening still need
  VM verification. Do not treat compilation or mathematical timing as that proof.

Existing unrelated boot-audio changes in the working tree are preserved.

## Fresh installer artifact

`sh tools/build-hermes.sh` completed successfully. The refreshed
`builds/InfinityOS-aarch64.iso` is 5,503,328,256 bytes, with SHA256
`e3f5aa698727f979bc1067e715e750f620e8190b6a086efeab6a90de328112dc`.
Artifact extraction verified installed-kernel, loader, and speech-model parity
(255,623,096, 24,576, and 33,722,129 bytes respectively). This is packaging
evidence, not a cold-installed visual or listening test.
