# Desktop interaction controls

## Implemented

- Launcher wheel and scrollbar changes take effect immediately, preserve wheel magnitude, and clamp to the viewport. Settings scrolling no longer waits for an easing tail.
- Launcher transitions advance from elapsed time over 160 ms. A frozen desktop backdrop and retained launcher surface avoid rebuilding applications and repeating glass blur for transition frames. Damage remains bounded to the reveal or changed panel.
- Right-side status icons open native menus: audio devices/permissions, network/mesh configuration, device/input management, power/session actions, Settings, app/file search, and user/system management.
- The clock opens a Gregorian calendar with previous month, next month, Today, and current-day highlighting from the live clock.
- Settings → Input changes pointer gain, acceleration, primary button, scroll speed/direction, and keyboard repeat delay/rate. Reset restores defaults. Preferences use reserved bytes in the existing per-user desktop record, without increasing its size; old records load defaults.

## Boundaries

Audio and Bluetooth menus route to existing management screens; they do not claim mixer or pairing capabilities that the hardware drivers do not provide. Keyboard repeat timing applies to transports reporting both key-down and key-up; firmware-only text input retains firmware repeat behavior. Absolute host-integrated pointers retain host coordinates; guest gain applies to relative devices.

The backdrop cache supports buffers up to 3840×2160 pixels including stride. The retained panel cache supports 2560×1600 pixels including effect margins. Larger surfaces use the existing uncached renderer. First opening still composes the desktop once; subsequent transition frames reuse it.

## Behavioral verification

- `make app-launcher-interaction-test`: immediate scroll state, elapsed-time transitions, drag ordering, repeat timing, preference encoding, calendar boundaries, and menu hit regions/backing capacity.
- `make active-painter-test`: clipped backdrop restoration, rejection of partial captures, persistent window surface reuse, and real pixel comparisons.
- `make input-regression-test`: input, navigation, session, and desktop regression suite.
- `tools/installed-kernel-parity-test.py`: architecture-correct installed ELF embedded byte-for-byte in each installer, plus installed/live boot-loader parity.
- `tools/ui-install-parity-test.sh`: packaged UI payload parity for live and installed images.

VM visual review and actual guest frame-time measurements are separate acceptance steps; host fixture timings are not VirtualBox performance measurements.
