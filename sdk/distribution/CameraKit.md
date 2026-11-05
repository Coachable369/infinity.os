# CameraKit — quick start and reference

Copy `/InfinitySDK/Examples/C/camera.c` to Documents and run from Text Editor in
a session authorized to use the camera. The sample starts capture, waits while
yielding, reads one JPEG in bounded pages, verifies its boundaries and reports
byte count and checksum. It stops capture and clears its temporary buffer.
This is a capture example, not facial recognition or biometric enrollment.

## Public API

The exact ABI is `../APIKit/C/include/infinity/camera.h`.

| Call | Result and handling |
| --- | --- |
| `infinity_camera_start()` | Start authorized capture; negative means denied/unavailable |
| `infinity_camera_frame(&frame)` | Receive sequence and byte length; -11 means wait/yield |
| `infinity_camera_read(sequence, offset, buffer, count)` | Copy at most 4096 bytes; -116 means reacquire a fresh sequence |
| `infinity_camera_stop()` | Invalidate delivery and request transport shutdown |

Never mix pages from different sequences. Bound total frame size by
`INFINITY_CAMERA_FRAME_LIMIT`, validate each read length and discard partial
frames. No caller supplies DMA addresses or USB handles. A public header does
not grant camera permission; the OS's capability policy remains authoritative.

## Errors and lifecycle

Yield while no frame exists and bound waiting. Stop on all terminal paths after
successful start. Do not keep private images in logs or publish them as AI state.
On missing hardware or denied permission, explain the failure and let the user
change policy; never substitute a fake camera image. In a VM, host camera access
and device passthrough must also be configured by the user.

## Validation

Expected success is a complete sequence-consistent JPEG and clean shutdown.
Exercise permission denial, no-device, timeout, stale sequence and close paths.
Successful compilation or package parity is not proof of camera hardware access,
face quality, biometric enrollment or facial-login security.
