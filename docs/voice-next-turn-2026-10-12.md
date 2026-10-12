# Voice reply-to-input handoff

## Acceptance scope

- Preserve first-turn recognition, synthesis and playback behavior.
- Drop microphone audio captured during the previous reply before accepting the next wake phrase.
- Keep the existing capture stream, calibration and authority; add no cooldown.
- Preserve wake-word admission and fail closed on revoked authority.
- Exercise several different conversational turns, a busy adapter and a fresh DMA onset.

## Evidence and limits

The retained October 12 VM trace shows seven `reply drained` to first capture
sample intervals of 24, 45, 31, 61, 52, 51 and 48 milliseconds. It does **not**
establish a twenty-second microphone restart or recognizer reload. Several later
utterances complete recognition in approximately 0.6 seconds but fail the wake
gate. Transcripts and conversational output PCM were not recorded, so those
rejections cannot be attributed to a particular spoken word or audible endpoint.
The pre-change trace is retained at
`builds/validation/voice-rearm-20261012/serial-before.log`.

Code inspection and a failing behavioral fixture established a separate concrete
defect: the muted capture path removes at most 4,800 queued mono samples per
controller pass, while the queue can hold 192,000. The reply-to-listening reset
cleared the controller's scratch buffers but neither that queue nor unpumped DMA
frames. Speaker-era audio could therefore precede a new wake phrase in the next
recognition request.

## Change

`discard_capture_pending(owner)` establishes an authorized boundary under the
existing audio adapter lock. It advances only the completed-frame input cursor
and erases queued microphone samples. It does not stop/reopen DMA or extend the
lease. Frames captured after the cursor snapshot remain available normally.
The controller uses this boundary immediately before publishing Listening after
a reply. A busy adapter retries on the next service pass; permission and device
failures remain observable. There is no new fixed delay and wake authorization
is still required for each new request.

## Behavioral verification

The new controller regression failed against the old implementation because
speaker-era PCM remained pending at the next Listening transition. With the
boundary in place, all 28 controller tests pass, including three distinct next
commands with 4,801, 32,000 and 192,000 old samples queued. They verify no clock
advance, no capture restart, no old sample markers delivered to recognition,
and no premature Listening publication while the adapter is busy.

The adapter fixture separately covers queued plus unpumped DMA samples, exact
fresh sample continuity, caller ownership, lease revocation and lock contention.
The actual HDA register fixture also covers cursor wrap, partial stereo frames,
unchanged RUN registers and fault recovery. The complete voice pipeline gate
passed through `./build-kit run python3 tools/voice-pipeline-test.py`, manifest
`builds/manifests/20261012T164951499447Z-62700.json`.
Host behavioral verification and release packaging are distinct from an
installed microphone/speaker acceptance test. This change has not yet been
verified by speaking into the rebuilt installed system and does not prove that
every perceived delay has the same cause.
