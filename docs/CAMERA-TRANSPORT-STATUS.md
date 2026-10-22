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

## Service boundary

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

## Original acceptance gate (integration update below)

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

## Dispatch/lifecycle follow-up

`camera_transport.rs` now routes authenticated typed IOP start/read/stop requests,
including sequence/length metadata copied into checked user memory. Each service
visit makes at most one backend call. Disconnect invalidates frame delivery;
reconnect waits for a shutdown acknowledgement and is limited to three start
attempts, spaced by at least 250 ms. Cancellation/revocation clears delivery and
requires shutdown. Eight unsuccessful shutdown visits latch Faulted, which does
not permit reuse. Faulted is not a claim that the physical camera stopped.

These lifecycle tests use an explicitly scripted backend; they do not substitute
for real capture, hardware shutdown, or installed-process proof. Those tests
preceded the installed adapter described below.

The full release build at revision 9c1bb7ab failed in the unrelated existing
configuration-wizard painter check (`tools/configuration-wizard-render.rs:561`,
PrimaryButton pixel validation). Manifest:
`builds/manifests/20261022T201830765060Z-61051.json`.
Per user direction, that unrelated UI failure is outside the current camera work.
Subsequent incremental, tests-skipped camera ISOs succeeded as recorded below.
They are not full-release or facial-authentication acceptance evidence.

## Installed integration first pass

`boot/common/uefi_camera.h` exposes a kernel-private, fixed-operation ARM adapter
for the 80ee:0030 camera. Discovery advances one candidate per visit, bounded to
64 handles. Probe/commit fixes 640×480 MJPEG at five fps. Each control/bulk call
requests a 10 ms timeout. Re-resolving the protocol detects removed handles.
Old firmware handoffs remain valid; the appended function pointer is read only
when the version magic is present.

`camera_uvc.rs` provides shared bounded negotiation/deframing. Partial/error
frames are discarded until a new FID. `drivers/camera.rs` connects it to the
typed transport and isolated native-app dispatch. Locking a session, expiry,
revocation and process exit revoke delivery and schedule shutdown. Failed stop
acknowledgement latches Faulted. Synchronous transfer quiescence does not prove
the sensor is powered down; that still requires installed observation.

No untrusted user address reaches firmware. Reads copy a maximum 4096 bytes
through checked application page tables. One frame slot, bounded scratch and
three reconnect attempts prevent unbounded queues. Buffers total approximately
3.52 MiB plus 64 KiB packet scratch, allocated statically.

Manual installed entrypoints, after installing updated media:
1. `camera build` compiles bundled `sdk/c/examples/camera_capture.c` through the
   native isolated compiler and publishes an ordinary executable object.
2. `camera allow` grants the authenticated session a 30-second launch lease.
3. Run the reported executable; it captures for at most five seconds and checks
   sequence-consistent JPEG chunks in isolated memory, reporting length/checksum.
4. `camera stop` revokes permission. Repeat with disconnect, reconnect, lock,
   cancellation and unrelated desktop interaction. Inspect actual camera LED.

The header and client source are embedded in both live/installed kernels and
included in the compiler artifact parity assertion. x86_64 compiles the shared
code but returns unavailable; a native x86 USB backend remains missing.
No automated end-to-end runs were performed during this implementation pass.

Compile-only evidence (not installed-runtime acceptance):
- ARM64 kernel: `builds/manifests/20261022T215934957002Z-50226.json`.
- x86_64 kernel: `builds/manifests/20261022T220108605456Z-50320.json`.
- ARM firmware adapter: `builds/manifests/20261022T215747044188Z-49900.json`.
- Isolated C client, both targets: `builds/manifests/20261022T215707699444Z-49843.json`.

Implementation commit: `d8501ecb`. Subsequent ISO packaging includes final source
and reports its own build manifest. Packaging is not evidence of real installed
camera capture, disconnect recovery, sensor shutdown or desktop frame times.

Manual acceptance remains outstanding:
- Install, detach ISO, reboot, then build/run the client from the installed OS.
- Run without `camera allow`: permission must be denied, not implicitly granted.
- Grant and run: obtain a nonempty sequence-consistent JPEG result; this client
  reports bytes/checksum, not a decoded preview or a face match.
- Press Ctrl+C, lock, close the owning Command Window, or let the five-second deadline
  expire: delivery must stop. Observe the physical camera indicator separately.
- Disconnect during capture, reconnect, and try a fresh grant. Record failures;
  do not interpret Faulted or a zero-byte result as successful recovery.
- Drag a window and type while capturing; record visible stalls and frame times.
- Reboot again and confirm ordinary password login still works. Facial login,
  enrollment and recognition must not be advertised as enabled.

## Updated manual-test media

Incremental ARM-first, both-target packaging completed using
`./build-kit incremental --skip-tests --target all`.
Manifest: `builds/manifests/20261022T221248697082Z-54665.json`.
This is not a full release build. Broad regression and installed runtime testing
were deferred as requested.

| ISO | SHA-256 |
| --- | --- |
| `builds/InfinityOS-aarch64.iso` | `d2ff85204bfb89b46a03322283d29176f2e97a8b6013b4077062c0c27f65c2b3` |
| `builds/InfinityOS-x86_64.iso` | `eb0034650356d63ea36e3cc1f666ef7a8243175f79cee469aaa463b99cd4d53a` |

Both packaging runs verified checksums and live/installed kernel payload parity.
Camera SDK and capture source bytes were checked against the actual ARM ISO live
kernel and its installed kernel (manifest
`builds/manifests/20261022T223338826446Z-61733.json`). The x86 packaging run checked
the exact live and installed input kernels before ISO staging. Artifact records
are in `builds/validation/native-apps/compiler-parity-{aarch64,x86_64}.json`.
These are packaging assertions, not behavioral camera proof.

Optional-feature compile guards added after packaging do not change the default
native-compiler-enabled ISO behavior. No VM was reinstalled or updated by this run.
