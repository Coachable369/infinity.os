# Facial capture recovery

## Source changes

Completed camera snapshots are consumed after their final checked page read.
Previously, a polling client could re-pin the same completed snapshot while the
driver dropped new frames during the pin. The change preserves stable paged
delivery, owner checks, revocation and monotonic frame sequences.

An enrollment camera timeout is retryable within the existing session deadline.
Idle-preview timeouts pause until an explicit capture instead of terminating the
worker and clearing accepted views. Capture timeouts likewise leave the current
view pending. Disconnects, cancellation, permission loss and terminal failures
still erase temporary enrollment. Initial camera checking, recognition and login
remain bounded; recognition thresholds and camera permissions are unchanged.

A fully transferred explicit capture now fills its Front/Left/Right slot while
analysis runs, marked Checking rather than Accepted. It cannot be replaced by
another preview during analysis. Only a successful worker report accepts that
photo and advances. Quality or pose rejection removes the candidate and requests
the same view again with specific guidance. Earlier accepted views are retained.
Missing capture pixels cannot advance enrollment. The existing model requires
three accepted poses and three separate verification comparisons; no unsupported
confidence score, extra enrollment samples or liveness claim has been added.

The camera is closed before initial model loading as well as before recognition.
Model preparation, analysis, acquisition and session timeout messages are now
distinct, rather than all appearing to be a camera-check timeout.

## Verification status

Source whitespace lint (`git diff --check`) passed. This is not behavioral proof.
Behavioral cases were added for consumed-frame delivery, visible checking slots,
acceptance requiring pixels, rejection/retry, retained prior views, stale reports,
timeout phases, and production thumbnail pixels at 1280x800 and 1920x1080.
Synthetic RGB in the painter test checks transfer/rendering only, not recognition.

Tests and clean release generation have not run for this patch. Build-kit refused
`face-capture-tests` while build `20261028T075117366097Z-63409` held the lock, and
refused `full` while build `20261028T084521732901Z-57595` held it. Neither lock was
removed and neither external build was stopped. No new ISO, manifest, checksum,
live-camera success or installed-runtime acceptance is claimed by this change.

When the build kit is available, run the capture, setup/login, camera-service and
production face-painter harnesses, then `./build-kit full`. Both generated ISOs
must pass packaging/checksum and fresh-install parity gates. The modified worker
and shared camera/setup/rendering code use existing dual-target build dependencies
and installed-kernel payloads; no live-only component was added.

Installed acceptance still requires a real camera: capture a front view, observe
its Checking image then acceptance/advancement, reject and retry a poor side
view without losing the front image, exercise a stalled camera and disconnect,
cancel, and complete verification plus encrypted save/readback. Repeat on every
supported camera/target configuration before claiming hardware acceptance.
