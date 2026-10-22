# Native camera prerequisite — 2026-10-22

## Verified evidence

The dedicated VirtualBox ARM64 firmware probe captured a real MJPEG frame from
the exposed FaceTime camera (USB 80ee:0030, streaming endpoint 0x82). The primary
user VM was not modified. The probe VM was stopped and its camera detached.

- Evidence: `builds/validation/camera/20261022T200759Z-c625d5e3/`
- JPEG: 640×480, 15,601 bytes; full JPEG decoding passed.
- Capture elapsed: 1,386 ms; longest synchronous bulk call: 14 ms.
- Bulk calls: 140; successful reads: 6; timeouts: 134.
- Negotiated payload: 3,060 bytes; requested per-call timeout: 10 ms.
- Capture manifest: `builds/manifests/20261022T200759209072Z-59673.json`
- Decode manifest: `builds/manifests/20261022T200937337147Z-59742.json`

Host JPEG decoding validates the artifact only. Capture executes in the guest;
no host camera API or host inference produced the frame. These timings are not
desktop latency measurements or a continuous-stream throughput claim.

## Service boundary implemented, not yet wired into installed dispatch

`kernel/runtime/camera.rs` adds an explicit CameraCapture capability, a bounded
lease, one 1,228,800-byte frame slot, sequence-tagged reads of at most 4,096 bytes,
authenticated-owner checks, and native page-table validated copies. Revocation,
expiry, cancellation and destruction invalidate buffered image data. No firmware
or DMA address is returned to a client. JPEG marker checks are framing checks,
not an untrusted-image decoder or an image validity/security proof.

`tools/camera-service-test.rs` exercises production service and memory-checking
code using the real captured JPEG. It verifies byte equality, forged-caller
rejection, invalid ranges, supervisor-page rejection, stale sequences,
cancellation, expiration, revocation and buffer clearing. It runs on the host:
it is not evidence of an isolated guest process receiving frames.

Passing service manifest: `builds/manifests/20261022T201528136624Z-60267.json`.
Passing synthetic transport state-machine manifest:
`builds/manifests/20261022T201614414387Z-60329.json`.
Synthetic transport tests are not real disconnect/cancellation evidence.

## Remaining acceptance gate

1. Native USB controller transport and bounded scheduling. ARM firmware capture
   is diagnostic only; x86 exits boot services and cannot use that backend.
2. Wire camera IOP requests and frame metadata into authenticated isolated-task
   dispatch; connect the driver producer to the service slot.
3. Implement hardware stream shutdown, handle invalidation on disconnect, and
   bounded rediscovery on reconnect. Software cancellation alone is insufficient.
4. Verify real frames in an isolated process, cancellation, unplug/reconnect and
   desktop responsiveness in installed systems, independently on both targets.
5. Include all executable/service/policy components in fresh-install payload and
   behavioral parity assertions. An ISO alone cannot prove this acceptance.

Do not start facial inference or claim facial login readiness until this gate
passes. Password authentication is unchanged. No recognition dependencies or
weights have been adopted. The inference/license spike and pre-login key
protection design remain subsequent, separately reviewed work.
