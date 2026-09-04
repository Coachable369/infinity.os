# InfinityUI Rendering

The renderer uses retained state and bounded damage rectangles. Old and new
element bounds are invalidated together; overlapping damage is merged; overflow
collapses to one safe region rather than allocating. Login animation restores
only prior particle footprints and redraws new orbs along the right-hand
infinity curve, outside the authentication panel.

Wallpaper, icon, glyph, and surface caches have explicit budgets and eviction
signals. Frame pacing uses a monotonic clock and records missed deadlines.
Motion can be full, reduced, or disabled.

The animated bootstrap has an explicit input-first frame policy. Pointer
reports are drained and cursor damage is presented before particle work. A
deadline containing pointer motion skips the software glow pass; the full
60-frame-per-second effect resumes on the first idle deadline. Absolute HID
queues collapse to their newest coordinate, and an active UEFI Absolute Pointer
source suppresses duplicate raw-tablet axes. This keeps animation state
independent from cursor state and prevents stale movement replay.

The ISO-only reveal pulse is vertically calibrated 50 framebuffer pixels below
its original mathematical origin so its particles follow the visible Infinity
ribbon. This calibration affects only the installer boot reveal; installed-boot
and desktop particle geometry retain their independent layout paths.

Damage calculation and bounded cache behavior are TESTED at the native unit
boundary. The current platform presenter still writes damaged regions to the
firmware framebuffer; a hardware-accelerated compositor and full per-window
double buffering are PLANNED and are not claimed.
